//! Streamed agent answer through every lifecycle state: `Streaming`, `Stopped`, `Failed`, and `Done`.
//!
//! Renders word-by-word streaming with soft arrival fade, fenced code blocks, steer and queued
//! message chips above the answer, quiet steer-bend dividers inside the stream, exact-word
//! `Continue` after `Stop`, exact-word `Retry` after `Failed`, and `Copy` / `Regenerate` on `Done`.

use crate::{
    components::pro_chrome::{
        ProBtnStyle, ProPalette, ProSelectionSensor, pro_btn, pro_check_draw, pro_ease, pro_eid,
        pro_empty_state, pro_fold, pro_icon_btn, pro_mark_selected, pro_pill_indicator,
        pro_rolling_number, pro_selection_sensor, pro_skeleton_bar, pro_swap_label, pro_tactile,
        pro_toast_banner, pro_view_transition, tr,
    },
    foundation::Theme,
    primitives::{
        clipboard,
        control::UiLanguage,
        family::{PhaseClock, Slide, disabled_motion},
        pro_icons::{AGENT_WAVE_MS, ProIcon, agent_pixel_cells, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, collections::HashMap, rc::Rc};
use web_time::Instant;

const STOP_ROUND_RECT_SVG: &str = r#"<path d="M5 9.5c0-2.12 0-3.18.66-3.84C6.32 5 7.38 5 9.5 5h5c2.12 0 3.18 0 3.84.66.66.66.66 1.72.66 3.84v5c0 2.12 0 3.18-.66 3.84-.66.66-1.72.66-3.84.66h-5c-2.12 0-3.18 0-3.84-.66C5 17.68 5 16.62 5 14.5v-5Z" fill="currentColor"></path>"#;
const BEND_ARROW_SVG: &str = r#"<path d="M4 3v3.5A4.5 4.5 0 0 0 8.5 11H18m-2.5-2.5L18 11l-2.5 2.5" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round"></path>"#;

/// Lifecycle state of a [`ResponseStream`] answer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ResponseStreamStatus {
    /// Actively streaming words with a breathing blue caret and twinkling pixel mark.
    #[default]
    Streaming,
    /// Stopped by the user (`Esc` or `Stop`); `Continue` picks up at the exact word index.
    Stopped,
    /// Interrupted by an error mid-sentence; `Retry` continues from that exact word index.
    Failed,
    /// Completed answer showing `Copy`, `Regenerate`, and duration/token metadata.
    Done,
}

impl ResponseStreamStatus {
    fn key(self) -> &'static str {
        match self {
            Self::Streaming => "streaming",
            Self::Stopped => "stopped",
            Self::Failed => "failed",
            Self::Done => "done",
        }
    }
}

/// Kind of chip displayed above the [`ResponseStream`] answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResponseChipKind {
    /// Mid-stream steering instruction that bends the current answer.
    Steer,
    /// Message queued to become the next turn once this answer finishes.
    Queued,
}

/// Steer or queued-message chip displayed above [`ResponseStream`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResponseSteerChip {
    /// Stable chip identifier.
    pub id: SharedString,
    /// Message text.
    pub text: SharedString,
    /// `Steer` vs `Queued` badge styling.
    pub kind: ResponseChipKind,
}

impl ResponseSteerChip {
    /// Create a [`ResponseSteerChip`].
    pub fn new(
        id: impl Into<SharedString>,
        text: impl Into<SharedString>,
        kind: ResponseChipKind,
    ) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            kind,
        }
    }
}

/// Quiet inline divider marking the exact word offset where a steer bent the answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResponseBend {
    /// Word offset where the bend divider appears.
    pub word_index: usize,
    /// Short steer summary displayed along the divider.
    pub text: SharedString,
}

impl ResponseBend {
    /// Construct a [`ResponseBend`] at `word_index`.
    pub fn new(word_index: usize, text: impl Into<SharedString>) -> Self {
        Self {
            word_index,
            text: text.into(),
        }
    }
}

/// Agent identity for [`ResponseStream`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResponseAgent {
    /// Seed id for the 3x3 mirrored pixel mark.
    pub id: SharedString,
    /// Display name.
    pub name: SharedString,
}

impl ResponseAgent {
    /// Construct a [`ResponseAgent`].
    pub fn new(id: impl Into<SharedString>, name: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
        }
    }
}

/// Completion statistics displayed in the `Done` footer of [`ResponseStream`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResponseMeta {
    /// Output token count.
    pub tokens: usize,
    /// Total elapsed generation duration in milliseconds.
    pub duration_ms: u64,
}

impl Default for ResponseMeta {
    fn default() -> Self {
        Self {
            tokens: 182,
            duration_ms: 6_400,
        }
    }
}

/// Typed events emitted by [`ResponseStream`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResponseStreamEvent {
    /// Emitted when streaming is stopped at `word_index`.
    Stopped {
        /// Exact word index preserved on stop.
        word_index: usize,
    },
    /// Emitted when `Continue` resumes streaming from `word_index`.
    Continued {
        /// Word index resumed from.
        word_index: usize,
    },
    /// Emitted when streaming enters `Failed` at `word_index`.
    Failed {
        /// Word index preserved on failure.
        word_index: usize,
        /// One-sentence error explanation.
        error: SharedString,
    },
    /// Emitted when `Retry` resumes from `word_index` after a failure.
    Retried {
        /// Word index resumed from.
        word_index: usize,
    },
    /// Emitted when `Dismiss` clears a stopped or failed answer.
    Dismissed,
    /// Emitted when a steer or queued chip above the answer is removed (`✕`).
    ChipRemoved(SharedString),
    /// Emitted when `Copy` copies the answer text.
    Copied(SharedString),
    /// Emitted when `Regenerate` restarts the answer from word `0`.
    Regenerated,
}

impl EventEmitter<ResponseStreamEvent> for ResponseStream {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum StreamSegment {
    Word(SharedString),
    ParagraphBreak,
    CodeFence {
        lang: Option<SharedString>,
        code: SharedString,
    },
}

pub(crate) fn parse_stream_segments(raw: &str) -> Vec<StreamSegment> {
    let mut out = Vec::new();
    let mut in_fence = false;
    let mut fence_lang: Option<SharedString> = None;
    let mut fence_lines: Vec<&str> = Vec::new();
    let mut prose_buf: Vec<&str> = Vec::new();

    let flush_prose = |buf: &mut Vec<&str>, target: &mut Vec<StreamSegment>| {
        if buf.is_empty() {
            return;
        }
        let joined = buf.join("\n");
        buf.clear();
        let mut emitted_in_buf = false;
        for para in joined.split("\n\n") {
            let trimmed = para.trim();
            if trimmed.is_empty() {
                continue;
            }
            if emitted_in_buf {
                target.push(StreamSegment::ParagraphBreak);
            }
            emitted_in_buf = true;
            for w in trimmed.split_whitespace() {
                target.push(StreamSegment::Word(SharedString::from(w.to_owned())));
            }
        }
    };

    for line in raw.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("```") {
            if in_fence {
                out.push(StreamSegment::CodeFence {
                    lang: fence_lang.take(),
                    code: SharedString::from(fence_lines.join("\n")),
                });
                fence_lines.clear();
                in_fence = false;
            } else {
                flush_prose(&mut prose_buf, &mut out);
                let l = rest.trim();
                fence_lang = (!l.is_empty()).then(|| SharedString::from(l.to_owned()));
                in_fence = true;
            }
        } else if in_fence {
            fence_lines.push(line);
        } else {
            prose_buf.push(line);
        }
    }
    if in_fence && !fence_lines.is_empty() {
        out.push(StreamSegment::CodeFence {
            lang: fence_lang.take(),
            code: SharedString::from(fence_lines.join("\n")),
        });
    }
    flush_prose(&mut prose_buf, &mut out);
    out
}

/// Complete lifecycle component for a streamed agent response.
pub struct ResponseStream {
    id: ElementId,
    focus: FocusHandle,
    prompt: Option<SharedString>,
    agent: ResponseAgent,
    raw_text: SharedString,
    segments: Vec<StreamSegment>,
    word_cursor: usize,
    last_advance_at: Instant,
    status: ResponseStreamStatus,
    error: SharedString,
    chips: Vec<ResponseSteerChip>,
    expand_chips: bool,
    bends: Vec<ResponseBend>,
    meta: ResponseMeta,
    esc_to_stop: bool,
    interactive_controls: bool,
    words_per_sec: f32,
    stream_accum: f32,
    started_at: Instant,
    last_frame: Option<Instant>,
    copied_at: Option<Instant>,
    copy_clock: PhaseClock,
    chip_entrances: HashMap<SharedString, (Slide, PhaseClock)>,
    bend_entrances: HashMap<usize, (Slide, PhaseClock)>,
    dismissed: bool,
    next_seq: usize,
    state_sensor: ProSelectionSensor,
    loading: bool,
    empty: bool,
    measured_width: Rc<Cell<f32>>,
    toast: Option<(SharedString, Instant)>,
}

impl ResponseStream {
    /// Create a [`ResponseStream`] preloaded with `"Atlas"` streaming a 3-step rollout plan.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<UiLanguage>(|_, cx| cx.notify())
            .detach();
        let raw: SharedString = default_stream_text().into();
        let segments = parse_stream_segments(&raw);
        let now = Instant::now();
        Self {
            id: id.into(),
            focus: cx.focus_handle(),
            prompt: Some("How should we roll out the new billing page?".into()),
            agent: ResponseAgent::new("atlas", "Atlas"),
            raw_text: raw,
            word_cursor: 24.min(segments.len()),
            segments,
            last_advance_at: now,
            status: ResponseStreamStatus::Streaming,
            error: "Upstream model stream dropped (HTTP 502) — partial output preserved.".into(),
            chips: default_steer_chips(),
            expand_chips: false,
            bends: vec![ResponseBend::new(28, "Keep rollout behind flag billing_v2")],
            meta: ResponseMeta::default(),
            esc_to_stop: true,
            interactive_controls: true,
            words_per_sec: 8.5,
            stream_accum: 0.0,
            started_at: now,
            last_frame: None,
            copied_at: None,
            copy_clock: PhaseClock::default(),
            chip_entrances: HashMap::new(),
            bend_entrances: HashMap::new(),
            dismissed: false,
            next_seq: 3,
            state_sensor: pro_selection_sensor(),
            loading: false,
            empty: false,
            measured_width: Rc::new(Cell::new(560.0)),
            toast: None,
        }
    }

    /// Set the answer text (supports markdown paragraphs and ```` ``` ```` code fences).
    pub fn with_text(mut self, text: impl Into<SharedString>) -> Self {
        self.raw_text = text.into();
        self.segments = parse_stream_segments(&self.raw_text);
        self.word_cursor = if self.status == ResponseStreamStatus::Done {
            self.segments.len()
        } else {
            self.word_cursor.min(self.segments.len())
        };
        self
    }

    /// Set or clear (`None`) the user prompt bubble shown above the answer.
    pub fn with_prompt(mut self, prompt: Option<SharedString>) -> Self {
        self.prompt = prompt;
        self
    }

    /// Set the initial [`ResponseStreamStatus`].
    pub fn with_status(mut self, status: ResponseStreamStatus) -> Self {
        self.status = status;
        if status == ResponseStreamStatus::Done {
            self.word_cursor = self.segments.len();
        }
        self
    }

    /// Configure the answering agent (`id` seeds the 3x3 pixel mark).
    pub fn with_agent(
        mut self,
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
    ) -> Self {
        self.agent = ResponseAgent::new(id, name);
        self
    }

    /// Set the failure message displayed when `status == Failed`.
    pub fn with_error(mut self, error: impl Into<SharedString>) -> Self {
        self.error = error.into();
        self
    }

    /// Supply steer and queued-message chips displayed above the answer.
    pub fn with_chips(mut self, chips: Vec<ResponseSteerChip>) -> Self {
        self.chips = chips;
        self
    }

    /// Supply inline steer-bend dividers inside the streamed text.
    pub fn with_bends(mut self, bends: Vec<ResponseBend>) -> Self {
        self.bends = bends;
        self
    }

    /// Configure token and latency metadata for the `Done` footer.
    pub fn with_meta(mut self, tokens: usize, duration_ms: u64) -> Self {
        self.meta = ResponseMeta {
            tokens,
            duration_ms,
        };
        self
    }

    /// Show (`true`, default) or hide (`false`) the interactive lifecycle simulation bar.
    pub fn with_controls(mut self, controls: bool) -> Self {
        self.interactive_controls = controls;
        self
    }

    /// Toggle whether `Esc` stops an active stream (`true` by default).
    pub fn with_esc_to_stop(mut self, enabled: bool) -> Self {
        self.esc_to_stop = enabled;
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

    /// Return the current [`ResponseStreamStatus`].
    pub fn status(&self) -> ResponseStreamStatus {
        self.status
    }

    /// Return the exact segment/word cursor reached so far.
    pub fn word_cursor(&self) -> usize {
        self.word_cursor
    }

    /// Stop the stream immediately, preserving `word_cursor`.
    pub fn stop_stream(&mut self, cx: &mut Context<Self>) {
        if self.status == ResponseStreamStatus::Streaming {
            self.status = ResponseStreamStatus::Stopped;
            self.last_frame = None;
            cx.emit(ResponseStreamEvent::Stopped {
                word_index: self.word_cursor,
            });
            cx.notify();
        }
    }

    /// Continue streaming from the exact `word_cursor` after a stop.
    pub fn continue_stream(&mut self, cx: &mut Context<Self>) {
        self.dismissed = false;
        if self.word_cursor >= self.segments.len() {
            self.word_cursor = (self.segments.len() / 2).max(1);
        }
        self.status = ResponseStreamStatus::Streaming;
        self.stream_accum = 0.0;
        self.last_frame = Some(Instant::now());
        cx.emit(ResponseStreamEvent::Continued {
            word_index: self.word_cursor,
        });
        cx.notify();
    }

    /// Trigger a mid-sentence failure at the current `word_cursor`.
    pub fn fail_stream(&mut self, cx: &mut Context<Self>) {
        self.dismissed = false;
        if self.word_cursor >= self.segments.len() {
            self.word_cursor = (self.segments.len() * 2 / 3).max(1);
        }
        self.status = ResponseStreamStatus::Failed;
        self.last_frame = None;
        cx.emit(ResponseStreamEvent::Failed {
            word_index: self.word_cursor,
            error: self.error.clone(),
        });
        cx.notify();
    }

    /// Retry after `Failed`, picking back up from the exact `word_cursor`.
    pub fn retry_from_failure(&mut self, cx: &mut Context<Self>) {
        self.dismissed = false;
        if self.word_cursor >= self.segments.len() {
            self.word_cursor = 0;
        }
        self.status = ResponseStreamStatus::Streaming;
        self.stream_accum = 0.0;
        self.last_frame = Some(Instant::now());
        cx.emit(ResponseStreamEvent::Retried {
            word_index: self.word_cursor,
        });
        cx.notify();
    }

    /// Jump directly to `Done` with the complete answer revealed.
    pub fn complete_stream(&mut self, cx: &mut Context<Self>) {
        self.dismissed = false;
        self.word_cursor = self.segments.len();
        self.status = ResponseStreamStatus::Done;
        self.last_frame = None;
        cx.notify();
    }

    /// Regenerate the answer from word `0`.
    pub fn regenerate(&mut self, cx: &mut Context<Self>) {
        self.dismissed = false;
        self.word_cursor = 2.min(self.segments.len());
        self.stream_accum = 0.0;
        self.status = ResponseStreamStatus::Streaming;
        self.last_frame = Some(Instant::now());
        cx.emit(ResponseStreamEvent::Regenerated);
        cx.notify();
    }

    /// Dismiss a stopped or failed answer.
    pub fn dismiss(&mut self, cx: &mut Context<Self>) {
        self.dismissed = true;
        self.last_frame = None;
        cx.emit(ResponseStreamEvent::Dismissed);
        cx.notify();
    }

    /// Inject a live steer chip and bend marker at the current `word_cursor`.
    pub fn add_sample_steer(&mut self, cx: &mut Context<Self>) {
        let idx = self.next_seq;
        self.next_seq += 1;
        let note: SharedString = if idx % 2 == 1 {
            "Include 5% canary error-budget threshold".into()
        } else {
            "Keep rollback command to a single CLI line".into()
        };
        let id = SharedString::from(format!("chip-{idx}"));
        let mut chip_slide = Slide::new(8.0);
        chip_slide.duration(0.0, 0.24, cx);
        let mut chip_clk = PhaseClock::default();
        chip_clk.restart(cx);
        self.chip_entrances
            .insert(id.clone(), (chip_slide, chip_clk));
        self.chips.push(ResponseSteerChip::new(
            id,
            note.clone(),
            ResponseChipKind::Steer,
        ));
        let bend_pos = self.word_cursor.clamp(1, self.segments.len());
        let bend_idx = self.bends.len();
        let mut bend_slide = Slide::new(6.0);
        bend_slide.duration(0.0, 0.22, cx);
        let mut bend_clk = PhaseClock::default();
        bend_clk.restart(cx);
        self.bend_entrances.insert(bend_idx, (bend_slide, bend_clk));
        self.bends.push(ResponseBend::new(bend_pos, note));
        cx.notify();
    }

    /// Remove a steer or queued chip by `id`.
    pub fn remove_chip(&mut self, id: &SharedString, cx: &mut Context<Self>) {
        let before = self.chips.len();
        self.chips.retain(|c| &c.id != id);
        self.chip_entrances.remove(id);
        if self.chips.len() != before {
            cx.emit(ResponseStreamEvent::ChipRemoved(id.clone()));
            cx.notify();
        }
    }

    /// Copy the answer text to the clipboard.
    pub fn copy_answer(&mut self, cx: &mut Context<Self>) {
        let plain = self.raw_text.to_string();
        clipboard::write(&plain, cx).detach();
        self.copied_at = Some(Instant::now());
        self.copy_clock.restart(cx);
        self.toast = Some((
            tr(cx, "Copied answer to clipboard", "已复制回答至剪贴板").into(),
            Instant::now(),
        ));
        cx.emit(ResponseStreamEvent::Copied(self.raw_text.clone()));
        cx.notify();
    }

    fn handle_key_down(&mut self, ev: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.esc_to_stop
            && ev.keystroke.key == "escape"
            && self.status == ResponseStreamStatus::Streaming
        {
            self.stop_stream(cx);
        }
    }

    fn render_pixel_mark(&self, busy: bool, elapsed_ms: f32, p: &ProPalette, cx: &App) -> Div {
        let reduced = Theme::global(cx).reduced_motion;
        let cells = agent_pixel_cells(&self.agent.id);
        let col = if self.status == ResponseStreamStatus::Failed {
            p.red_ink
        } else {
            p.blue
        };
        let mut grid = div()
            .size(px(16.))
            .flex_none()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(1.5));
        for r in 0..3usize {
            let mut row = div().flex().items_center().gap(px(1.5));
            for c in 0..3usize {
                let i = r * 3 + c;
                let on = cells[i];
                let alpha = if busy && !reduced {
                    let ph = ((elapsed_ms + AGENT_WAVE_MS[i]) / 1200.0) * std::f32::consts::TAU;
                    let wv = 0.56 + 0.44 * ph.cos();
                    if on { wv } else { 0.16 * wv }
                } else if on {
                    0.92
                } else {
                    0.0
                };
                row = row.child(div().size(px(3.)).rounded(px(0.8)).bg(col.opacity(alpha)));
            }
            grid = grid.child(row);
        }
        grid
    }
}

impl Render for ResponseStream {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let reduced = disabled_motion(cx);
        let width = self.measured_width.get();
        let narrow = width < 380.0;
        let measure_cell = self.measured_width.clone();

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
                        .max_w(px(540.))
                        .p(px(16.))
                        .flex()
                        .flex_col()
                        .gap(px(10.))
                        .child(pro_skeleton_bar(240., 26., 7., 0.1, cx))
                        .child(pro_skeleton_bar(140., 16., 5., 0.25, cx))
                        .child(pro_skeleton_bar(460., 96., 8., 0.5, cx)),
                );
        }

        if self.empty {
            return div()
                .id(self.id.clone())
                .w_full()
                .rounded(px(10.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .child(pro_empty_state(
                    ProIcon::Sparkles,
                    tr(cx, "No response streaming", "暂无流式回答"),
                    tr(
                        cx,
                        "Streamed agent answers, steer chips, and exact-word recovery controls appear here.",
                        "流式智能体回答、中途引导胶囊与精确词位恢复控件将在此展示。",
                    ),
                    None,
                    cx,
                ));
        }

        // Advance live streaming cursor
        let now = Instant::now();
        if self.status == ResponseStreamStatus::Streaming && !self.dismissed {
            if let Some(prev) = self.last_frame {
                let dt = now.duration_since(prev).as_secs_f32().min(0.10);
                self.stream_accum += dt * self.words_per_sec;
                let step = self.stream_accum.floor() as usize;
                if step > 0 {
                    self.stream_accum -= step as f32;
                    self.word_cursor = (self.word_cursor + step).min(self.segments.len());
                    self.last_advance_at = now;
                }
            }
            self.last_frame = Some(now);
            if self.word_cursor >= self.segments.len() {
                self.status = ResponseStreamStatus::Done;
                self.last_frame = None;
            } else {
                w.request_animation_frame();
            }
        } else {
            self.last_frame = None;
        }

        let copy_fresh = self
            .copied_at
            .map(|t| t.elapsed().as_secs_f32() < 1.8)
            .unwrap_or(false);
        if copy_fresh && !reduced {
            w.request_animation_frame();
        }

        let elapsed_ms = self.started_at.elapsed().as_secs_f32() * 1000.0;
        let is_streaming = self.status == ResponseStreamStatus::Streaming;

        // Interactive lifecycle control bar
        let controls_bar = self.interactive_controls.then(|| {
            let states = [
                (
                    ResponseStreamStatus::Streaming,
                    tr(cx, "Streaming", "流式输出"),
                ),
                (ResponseStreamStatus::Stopped, tr(cx, "Stopped", "手动暂停")),
                (ResponseStreamStatus::Failed, tr(cx, "Failed", "异常中断")),
                (ResponseStreamStatus::Done, tr(cx, "Done", "已完成")),
            ];
            let seg_strip = div()
                .relative()
                .p(px(2.))
                .rounded(px(8.))
                .bg(p.subtle)
                .border_1()
                .border_color(p.line)
                .flex()
                .flex_wrap()
                .items_center()
                .gap(px(2.))
                .child(pro_pill_indicator(
                    pro_eid(&self.id, "rstr-state-pill"),
                    self.status.key(),
                    self.state_sensor.clone(),
                    6.,
                    p.card,
                    Some(p.line),
                    true,
                ))
                .children(states.into_iter().map(|(st, lbl)| {
                    let selected = self.status == st && !self.dismissed;
                    div()
                        .id(pro_eid(&self.id, format!("st-tab-{}", st.key())))
                        .role(Role::Button)
                        .tab_index(0)
                        .relative()
                        .h(px(25.))
                        .px(px(9.))
                        .rounded(px(6.))
                        .flex()
                        .items_center()
                        .cursor_pointer()
                        .text_size(px(12.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(if selected { p.ink } else { p.muted })
                        .hover(move |s| if selected { s } else { s.text_color(p.ink) })
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.dismissed = false;
                            match st {
                                ResponseStreamStatus::Streaming => {
                                    if this.word_cursor >= this.segments.len() {
                                        this.word_cursor = 8.min(this.segments.len());
                                    }
                                    this.continue_stream(cx);
                                }
                                ResponseStreamStatus::Stopped => {
                                    if this.word_cursor >= this.segments.len()
                                        || this.word_cursor == 0
                                    {
                                        this.word_cursor = (this.segments.len() / 2).max(1);
                                    }
                                    this.status = ResponseStreamStatus::Stopped;
                                    this.last_frame = None;
                                    cx.notify();
                                }
                                ResponseStreamStatus::Failed => this.fail_stream(cx),
                                ResponseStreamStatus::Done => this.complete_stream(cx),
                            }
                        }))
                        .when(selected, |d| d.child(pro_mark_selected(&self.state_sensor)))
                        .child(lbl)
                }));

            div()
                .w_full()
                .mb(px(14.))
                .flex()
                .flex_wrap()
                .items_center()
                .justify_between()
                .gap(px(8.))
                .child(seg_strip)
                .child(pro_btn(
                    pro_eid(&self.id, "add-steer-demo-btn"),
                    tr(cx, "Inject steer", "插入引导胶囊"),
                    Some(ProIcon::Plus),
                    ProBtnStyle::Secondary,
                    true,
                    cx,
                    |this, _, cx| this.add_sample_steer(cx),
                ))
        });

        // Prompt bubble
        let prompt_row = self.prompt.clone().map(|q| {
            div().w_full().mb(px(14.)).flex().justify_end().child(
                div()
                    .max_w(relative(0.84))
                    .px(px(12.))
                    .py(px(8.))
                    .rounded_tl(px(12.))
                    .rounded_tr(px(12.))
                    .rounded_bl(px(12.))
                    .rounded_br(px(4.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .shadow_xs()
                    .text_size(px(13.))
                    .line_height(px(19.))
                    .text_color(p.ink)
                    .child(q),
            )
        });

        if self.dismissed {
            return div()
                .id(self.id.clone())
                .w_full()
                .flex()
                .justify_center()
                .child(
                    div()
                        .w_full()
                        .max_w(px(540.))
                        .flex()
                        .flex_col()
                        .when_some(controls_bar, |d, cb| d.child(cb))
                        .when_some(prompt_row, |d, pr| d.child(pr))
                        .child(
                            div()
                                .w_full()
                                .p(px(16.))
                                .rounded(px(10.))
                                .bg(p.card)
                                .border_1()
                                .border_color(p.line)
                                .flex()
                                .items_center()
                                .justify_between()
                                .child(div().text_size(px(12.5)).text_color(p.muted).child(tr(
                                    cx,
                                    "Partial response dismissed.",
                                    "已移除未完成的流式回答。",
                                )))
                                .child(pro_btn(
                                    pro_eid(&self.id, "restore-dismissed"),
                                    tr(cx, "Stream again", "重新流式生成"),
                                    Some(ProIcon::Refresh),
                                    ProBtnStyle::Secondary,
                                    true,
                                    cx,
                                    |this, _, cx| this.regenerate(cx),
                                )),
                        ),
                );
        }

        // Steer & Queued message chips above the answer with smooth Slide entrance & pro_fold
        let has_chips = !self.chips.is_empty();
        let chips_rendered: Vec<AnyElement> = {
            let max_vis = if self.expand_chips || self.chips.len() <= 3 {
                self.chips.len()
            } else {
                3
            };
            let hidden_n = self.chips.len().saturating_sub(max_vis);
            let mut els: Vec<AnyElement> = Vec::with_capacity(max_vis + 1);
            let mut seen_queued = false;
            for ch in self.chips.iter().take(max_vis) {
                let cid = ch.id.clone();
                let (dy, alpha) = if reduced {
                    (0.0, 1.0)
                } else if let Some((sl, clk)) = self.chip_entrances.get(&cid) {
                    let y = sl.get(w, cx);
                    let age = clk.time(0.22, w, cx);
                    let a = 0.25 + 0.75 * pro_ease((age / 0.22).clamp(0.0, 1.0));
                    (y, a)
                } else {
                    (0.0, 1.0)
                };
                let (badge_txt, badge_bg, badge_fg) = match ch.kind {
                    ResponseChipKind::Steer => (tr(cx, "Steer", "实时引导"), p.accent_soft, p.blue),
                    ResponseChipKind::Queued => {
                        let lbl = if !seen_queued {
                            seen_queued = true;
                            tr(cx, "Next", "下一条")
                        } else {
                            tr(cx, "Queued", "排队中")
                        };
                        (lbl, p.chip, p.text)
                    }
                };
                let rm_aria = SharedString::from(format!("Remove {}", ch.text));
                els.push(
                    div()
                        .w_full()
                        .relative()
                        .top(px(dy))
                        .opacity(alpha)
                        .h(px(28.))
                        .pl(px(6.))
                        .pr(px(4.))
                        .rounded(px(7.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .shadow_xs()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(
                            div()
                                .h(px(20.))
                                .px(px(6.))
                                .rounded(px(5.))
                                .bg(badge_bg)
                                .flex()
                                .items_center()
                                .text_size(px(11.5))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(badge_fg)
                                .child(badge_txt),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .text_size(px(12.5))
                                .text_color(p.text)
                                .truncate()
                                .child(ch.text.clone()),
                        )
                        .child(pro_icon_btn(
                            pro_eid(&self.id, format!("rm-chip-{}", ch.id)),
                            rm_aria,
                            ProIcon::Close,
                            22.,
                            false,
                            cx,
                            move |this, _, cx| this.remove_chip(&cid, cx),
                        ))
                        .into_any_element(),
                );
            }
            if hidden_n > 0 {
                els.push(
                    pro_btn(
                        pro_eid(&self.id, "expand-more-chips"),
                        format!("+ {hidden_n} {}", tr(cx, "more", "条更多")),
                        None,
                        ProBtnStyle::Ghost,
                        true,
                        cx,
                        |this, _, cx| {
                            this.expand_chips = true;
                            cx.notify();
                        },
                    )
                    .self_start()
                    .into_any_element(),
                );
            }
            els
        };
        let chips_block = pro_fold(
            pro_eid(&self.id, "chips-fold"),
            has_chips,
            72.,
            0.,
            move || {
                div()
                    .w_full()
                    .mb(px(12.))
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .children(chips_rendered)
                    .into_any_element()
            },
        );

        // Header line (pixel mark + Atlas + animated state swap label)
        let (state_lbl, state_col) = match self.status {
            ResponseStreamStatus::Streaming => (tr(cx, "Writing", "正在输出"), p.muted),
            ResponseStreamStatus::Stopped => (tr(cx, "Stopped", "已暂停"), p.muted),
            ResponseStreamStatus::Failed => (tr(cx, "Failed", "输出中断"), p.red_ink),
            ResponseStreamStatus::Done => ("", p.muted),
        };
        let head = div()
            .h(px(20.))
            .flex()
            .items_center()
            .gap(px(8.))
            .child(self.render_pixel_mark(is_streaming, elapsed_ms, &p, cx))
            .child(
                div()
                    .text_size(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .child(self.agent.name.clone()),
            )
            .when(!state_lbl.is_empty(), |d| {
                d.child(pro_swap_label(
                    pro_eid(&self.id, "rstr-head-state-swap"),
                    state_lbl,
                    12.,
                    FontWeight::NORMAL,
                    state_col,
                ))
            });

        // Streamed segments body
        let indent_px = if narrow { 0.0 } else { 24.0 };
        let limit = self.word_cursor.min(self.segments.len());
        let newest_age = self.last_advance_at.elapsed().as_secs_f32();
        let newest_alpha = if !is_streaming || reduced {
            1.0
        } else {
            (0.45 + 0.55 * pro_ease((newest_age / 0.18).clamp(0.0, 1.0))).clamp(0.45, 1.0)
        };
        if is_streaming && newest_age < 0.20 && !reduced {
            w.request_animation_frame();
        }

        let mut body_col = div()
            .pl(px(indent_px))
            .mt(px(8.))
            .flex()
            .flex_col()
            .gap(px(10.));

        let mut cur_para = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_x(px(3.5))
            .gap_y(px(2.))
            .text_size(px(13.))
            .line_height(px(20.))
            .text_color(p.ink);
        let mut cur_para_has_items = false;

        for idx in 0..limit {
            // Render any steer bend divider anchored at `idx` with smooth Slide entrance
            for (b_idx, bend) in self
                .bends
                .iter()
                .enumerate()
                .filter(|(_, b)| b.word_index == idx)
            {
                if cur_para_has_items {
                    body_col = body_col.child(cur_para);
                    cur_para = div()
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .gap_x(px(3.5))
                        .gap_y(px(2.))
                        .text_size(px(13.))
                        .line_height(px(20.))
                        .text_color(p.ink);
                    cur_para_has_items = false;
                }
                let (b_dy, b_alpha) = if reduced {
                    (0.0, 1.0)
                } else if let Some((sl, clk)) = self.bend_entrances.get(&b_idx) {
                    let y = sl.get(w, cx);
                    let age = clk.time(0.20, w, cx);
                    let a = 0.25 + 0.75 * pro_ease((age / 0.20).clamp(0.0, 1.0));
                    (y, a)
                } else {
                    (0.0, 1.0)
                };
                body_col = body_col.child(
                    div()
                        .w_full()
                        .relative()
                        .top(px(b_dy))
                        .opacity(b_alpha)
                        .h(px(18.))
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .text_size(px(11.5))
                        .text_color(p.muted)
                        .child(pro_svg("0 0 24 24", BEND_ARROW_SVG, 12., p.blue))
                        .child(div().truncate().child(bend.text.clone()))
                        .child(div().flex_1().h(px(1.)).bg(p.line_strong)),
                );
            }

            let is_tail_word = is_streaming && idx + 2 >= limit;
            match &self.segments[idx] {
                StreamSegment::Word(word) => {
                    cur_para_has_items = true;
                    cur_para = cur_para.child(
                        div()
                            .opacity(if is_tail_word { newest_alpha } else { 1.0 })
                            .child(word.clone()),
                    );
                }
                StreamSegment::ParagraphBreak => {
                    if cur_para_has_items {
                        body_col = body_col.child(cur_para);
                        cur_para = div()
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .gap_x(px(3.5))
                            .gap_y(px(2.))
                            .text_size(px(13.))
                            .line_height(px(20.))
                            .text_color(p.ink);
                        cur_para_has_items = false;
                    }
                }
                StreamSegment::CodeFence { lang, code } => {
                    if cur_para_has_items {
                        body_col = body_col.child(cur_para);
                        cur_para = div()
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .gap_x(px(3.5))
                            .gap_y(px(2.))
                            .text_size(px(13.))
                            .line_height(px(20.))
                            .text_color(p.ink);
                        cur_para_has_items = false;
                    }
                    body_col = body_col.child(
                        div()
                            .w_full()
                            .px(px(12.))
                            .py(px(10.))
                            .rounded(px(8.))
                            .bg(p.subtle)
                            .border_1()
                            .border_color(p.line)
                            .flex()
                            .flex_col()
                            .gap(px(4.))
                            .when_some(lang.clone(), |d, lg| {
                                d.child(div().text_size(px(11.)).text_color(p.muted).child(lg))
                            })
                            .child(
                                div()
                                    .font_family("Menlo")
                                    .text_size(px(12.))
                                    .line_height(px(18.))
                                    .text_color(p.ink)
                                    .child(code.clone()),
                            ),
                    );
                }
            }
        }

        if is_streaming {
            let breathe = if reduced {
                0.90
            } else {
                0.38 + 0.57 * ((elapsed_ms / 1100.0) * std::f32::consts::TAU).cos().abs()
            };
            cur_para_has_items = true;
            cur_para = cur_para.child(
                div()
                    .w(px(2.))
                    .h(px(15.))
                    .ml(px(2.))
                    .rounded(px(1.))
                    .bg(p.blue.opacity(breathe)),
            );
        }

        if cur_para_has_items {
            body_col = body_col.child(cur_para);
        }

        // Cut line & failure message
        if self.status == ResponseStreamStatus::Stopped {
            body_col = body_col.child(
                div()
                    .w_full()
                    .h(px(16.))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .text_size(px(11.5))
                    .text_color(p.muted)
                    .child(format!(
                        "{} {} {}",
                        tr(cx, "Stopped at", "已暂停于第"),
                        limit,
                        tr(cx, "words", "词")
                    ))
                    .child(div().flex_1().h(px(1.)).bg(p.line)),
            );
        } else if self.status == ResponseStreamStatus::Failed {
            body_col = body_col
                .child(
                    div()
                        .w_full()
                        .h(px(16.))
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .text_size(px(11.5))
                        .text_color(p.red_ink)
                        .child(format!(
                            "{} {} {}",
                            tr(cx, "Interrupted at", "异常中断于第"),
                            limit,
                            tr(cx, "words", "词")
                        ))
                        .child(div().flex_1().h(px(1.)).bg(p.red.opacity(0.28))),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .line_height(px(18.))
                        .text_color(p.text)
                        .child(self.error.clone()),
                );
        }

        // Stable 28px action footer with tactile KUI buttons & smooth view transition
        let btn_surface = if p.dark { rgb(0x2c2c2f).into() } else { p.card };
        let copy_drawn = reduced || self.copy_clock.time(0.26, w, cx) > 0.012;
        let footer_row = match self.status {
            ResponseStreamStatus::Streaming => {
                let stop_lbl = tr(cx, "Stop", "停止").to_owned();
                let stop_inner = div()
                    .h(px(28.))
                    .px(px(8.))
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .text_size(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .child(pro_svg("0 0 24 24", STOP_ROUND_RECT_SVG, 13., p.strong))
                    .child(pro_swap_label(
                        pro_eid(&self.id, "rstr-stop-swap"),
                        stop_lbl.clone(),
                        13.,
                        FontWeight::MEDIUM,
                        p.strong,
                    ))
                    .child(
                        div()
                            .h(px(18.))
                            .px(px(5.))
                            .rounded(px(4.))
                            .bg(p.subtle)
                            .border_1()
                            .border_color(p.line)
                            .flex()
                            .items_center()
                            .text_size(px(11.))
                            .font_weight(FontWeight::NORMAL)
                            .text_color(p.muted)
                            .child("Esc"),
                    );
                div()
                    .min_h(px(28.))
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(
                        pro_tactile(
                            pro_eid(&self.id, "rstr-stop-btn"),
                            stop_lbl,
                            6.,
                            btn_surface,
                            p.chip_hover,
                            p.line_strong,
                            p.line_strong,
                            false,
                            stop_inner,
                            cx,
                            |this, _, cx| this.stop_stream(cx),
                        )
                        .flex_none()
                        .shadow_xs(),
                    )
            }
            ResponseStreamStatus::Stopped => div()
                .min_h(px(28.))
                .flex()
                .items_center()
                .gap(px(6.))
                .child(pro_btn(
                    pro_eid(&self.id, "rstr-continue-btn"),
                    tr(cx, "Continue", "从断点继续"),
                    Some(ProIcon::Play),
                    ProBtnStyle::Secondary,
                    true,
                    cx,
                    |this, _, cx| this.continue_stream(cx),
                ))
                .child(pro_btn(
                    pro_eid(&self.id, "rstr-dismiss-stop"),
                    tr(cx, "Dismiss", "移除"),
                    None,
                    ProBtnStyle::Ghost,
                    true,
                    cx,
                    |this, _, cx| this.dismiss(cx),
                )),
            ResponseStreamStatus::Failed => div()
                .min_h(px(28.))
                .flex()
                .items_center()
                .gap(px(6.))
                .child(pro_btn(
                    pro_eid(&self.id, "rstr-retry-btn"),
                    tr(cx, "Retry", "从断点重试"),
                    Some(ProIcon::Refresh),
                    ProBtnStyle::Secondary,
                    true,
                    cx,
                    |this, _, cx| this.retry_from_failure(cx),
                ))
                .child(pro_btn(
                    pro_eid(&self.id, "rstr-dismiss-fail"),
                    tr(cx, "Dismiss", "移除"),
                    None,
                    ProBtnStyle::Ghost,
                    true,
                    cx,
                    |this, _, cx| this.dismiss(cx),
                )),
            ResponseStreamStatus::Done => {
                let copy_inner = div()
                    .size(px(28.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(if copy_fresh {
                        pro_check_draw(
                            pro_eid(&self.id, "rstr-copy-chk"),
                            copy_drawn,
                            14.,
                            false,
                            p.green_ink,
                        )
                    } else {
                        pro_icon(ProIcon::Copy, 14., p.muted)
                    });
                div()
                    .min_h(px(28.))
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(4.))
                    .child(
                        pro_tactile(
                            pro_eid(&self.id, "rstr-copy-btn"),
                            SharedString::from(tr(cx, "Copy answer", "复制回答")),
                            6.,
                            transparent_black(),
                            p.hover,
                            transparent_black(),
                            transparent_black(),
                            copy_fresh,
                            copy_inner,
                            cx,
                            |this, _, cx| this.copy_answer(cx),
                        )
                        .flex_none(),
                    )
                    .child(pro_icon_btn(
                        pro_eid(&self.id, "rstr-regen-btn"),
                        SharedString::from(tr(cx, "Regenerate answer", "重新生成回答")),
                        ProIcon::Refresh,
                        28.,
                        false,
                        cx,
                        |this, _, cx| this.regenerate(cx),
                    ))
                    .child(
                        div()
                            .ml(px(6.))
                            .flex()
                            .items_center()
                            .gap(px(4.))
                            .text_size(px(12.))
                            .text_color(p.muted)
                            .child(pro_rolling_number(
                                pro_eid(&self.id, "rstr-dur-s"),
                                self.meta.duration_ms as f64 / 1000.0,
                                1,
                                "",
                                "s",
                                12.,
                                FontWeight::NORMAL,
                                p.muted,
                            ))
                            .child("·")
                            .child(pro_rolling_number(
                                pro_eid(&self.id, "rstr-tok-n"),
                                self.meta.tokens as f64,
                                0,
                                "",
                                "",
                                12.,
                                FontWeight::NORMAL,
                                p.muted,
                            ))
                            .child(tr(cx, "tokens", "Tokens")),
                    )
            }
        };
        let footer = div()
            .pl(px(indent_px))
            .mt(px(10.))
            .min_h(px(28.))
            .child(pro_view_transition(
                pro_eid(&self.id, "rstr-foot-tx"),
                self.status.key(),
                footer_row,
            ));

        let answer_card = div()
            .w_full()
            .p(px(18.))
            .rounded(px(12.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .shadow_xs()
            .flex()
            .flex_col()
            .child(chips_block)
            .child(head)
            .child(body_col)
            .child(footer);

        div()
            .id(self.id.clone())
            .role(Role::Region)
            .aria_label(SharedString::from(format!("{} answer", self.agent.name)))
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
                .w(px(0.))
                .h(px(0.)),
            )
            .child(
                div()
                    .w_full()
                    .max_w(px(540.))
                    .flex()
                    .flex_col()
                    .when_some(controls_bar, |d, cb| d.child(cb))
                    .when_some(prompt_row, |d, pr| d.child(pr))
                    .child(answer_card),
            )
            .child(pro_toast_banner(self.toast.clone()))
    }
}

fn default_steer_chips() -> Vec<ResponseSteerChip> {
    vec![
        ResponseSteerChip::new(
            "c1",
            "Keep rollout behind flag billing_v2",
            ResponseChipKind::Steer,
        ),
        ResponseSteerChip::new(
            "c2",
            "Draft the customer migration email next",
            ResponseChipKind::Queued,
        ),
    ]
}

fn default_stream_text() -> &'static str {
    "Roll the new billing page out in three steps. First, ship it behind a flag to the internal team for a week and watch the error rate and invoice webhook latency.\n\n```ts\nconst enabled = flags.check(\"billing_v2\", { orgId, cohort: \"canary_10\" });\n```\n\nSecond, ramp 10% of self-serve workspaces while keeping one-click fallback to the legacy checkout route.\n\nThird, promote to 100% once 48 hours of renewal runs finish with zero failed charges."
}

#[cfg(test)]
mod tests {
    use super::{
        ResponseChipKind, ResponseStreamStatus, StreamSegment, default_steer_chips,
        default_stream_text, parse_stream_segments,
    };

    #[test]
    fn parse_stream_segments_splits_words_paragraphs_and_code_fences() {
        let segs = parse_stream_segments(default_stream_text());
        assert!(segs.len() > 45);
        assert!(segs.iter().any(|s| matches!(
            s,
            StreamSegment::CodeFence {
                lang: Some(lg),
                ..
            } if lg.as_ref() == "ts"
        )));
        assert!(
            segs.iter()
                .any(|s| matches!(s, StreamSegment::ParagraphBreak))
        );
    }

    #[test]
    fn default_chips_pair_steer_and_next_queued_entries() {
        let chips = default_steer_chips();
        assert_eq!(chips.len(), 2);
        assert_eq!(chips[0].kind, ResponseChipKind::Steer);
        assert_eq!(chips[1].kind, ResponseChipKind::Queued);
        assert_eq!(
            ResponseStreamStatus::default(),
            ResponseStreamStatus::Streaming
        );
    }
}
