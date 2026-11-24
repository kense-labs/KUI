//! Unified composer and run-control deck for streaming, steering, stopping, and resuming an agent.
//!
//! Combines a scrollable current-turn conversation panel with a queued-message tray and
//! [`Editor`] composer featuring `Stop`, exact-word `Resume`, error `Retry` / `Dismiss`,
//! mid-answer steer pills, `Chat` vs `Act` mode (`pro_pill_indicator`), permission posture
//! menu, and a live context-window fill ring (`pro_progress_ring`).

use crate::{
    components::pro_chrome::{
        ProBtnStyle, ProPalette, ProSelectionSensor, pro_btn, pro_check_draw, pro_eid,
        pro_empty_state, pro_fold, pro_icon_btn, pro_mark_selected, pro_mix_color,
        pro_pill_indicator, pro_popover_presence, pro_progress_ring, pro_rolling_number,
        pro_selection_sensor, pro_skeleton_bar, pro_swap_label, pro_tactile, pro_toast_banner, tr,
    },
    foundation::Theme,
    primitives::{
        control::UiLanguage,
        editor::{EditEvent, Editor, InputMode},
        family::{PhaseClock, Slide, disabled_motion},
        pro_icons::{AGENT_WAVE_MS, ProIcon, agent_pixel_cells, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, collections::HashMap, rc::Rc};
use web_time::Instant;

const STEER_CORNER_SVG: &str = r#"<path d="M4 3v2.1c0 2 0 3 .15 3.84.8 4.6 4.76 8.22 9.8 8.95.92.11 2.85.11 5.05.11m-2 3c.6-.59 3-2.16 3-3s-2.4-2.41-3-3" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round"></path>"#;
const QUEUE_LINES_SVG: &str = r#"<path d="M18 18H6c-.94 0-1.41 0-1.71-.29C4 17.41 4 16.94 4 16s0-1.41.29-1.71C4.59 14 5.06 14 6 14h12c.94 0 1.41 0 1.71.29.29.3.29.77.29 1.71s0 1.41-.29 1.71c-.3.29-.77.29-1.71.29ZM4 10h16M4 6h16" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"></path>"#;
const STOP_SQUARE_SVG: &str =
    r#"<rect x="6" y="6" width="12" height="12" rx="3" fill="currentColor"></rect>"#;

/// Execution lifecycle of [`RunControls`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RunControlStatus {
    /// Quiet start awaiting a prompt.
    Idle,
    /// Actively streaming words into the answer.
    #[default]
    Running,
    /// Halted mid-answer by the user; ready to `Resume` from the exact word index.
    Stopped,
    /// Interrupted by a stream/network error; ready to `Retry` from the exact word or `Dismiss`.
    Error,
    /// Stream finished completely.
    Done,
}

/// Interaction mode in [`RunControls`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RunMode {
    /// Conversational read-only answers (permission posture disabled).
    Chat,
    /// Agentic execution mode with configurable tool/edit permissions.
    #[default]
    Act,
}

impl RunMode {
    fn key(self) -> &'static str {
        match self {
            Self::Chat => "chat",
            Self::Act => "act",
        }
    }
}

/// Permission posture applied when [`RunMode::Act`] is active.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RunPermission {
    /// Prompt before running tools or file edits.
    #[default]
    AskFirst,
    /// Apply workspace edits automatically; ask before external actions.
    AutoEdits,
    /// Unrestricted execution without confirmation prompts (rendered in alert red).
    FullAccess,
}

impl RunPermission {
    const ALL: [Self; 3] = [Self::AskFirst, Self::AutoEdits, Self::FullAccess];

    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::AskFirst => tr(cx, "Ask first", "每次询问"),
            Self::AutoEdits => tr(cx, "Auto-edit", "自动编辑"),
            Self::FullAccess => tr(cx, "Full access", "完全权限"),
        }
    }

    fn description(self, cx: &App) -> &'static str {
        match self {
            Self::AskFirst => tr(
                cx,
                "Confirm before running commands or external tools",
                "执行命令或外部工具前均需确认",
            ),
            Self::AutoEdits => tr(
                cx,
                "Apply file edits automatically; ask before network actions",
                "自动写入文件修改，网络与外部操作前询问",
            ),
            Self::FullAccess => tr(
                cx,
                "Run commands, edits, and integrations without asking",
                "无需询问直接执行命令、编辑与外部集成",
            ),
        }
    }
}

/// Agent identity displayed on the current turn of [`RunControls`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunAgentSpec {
    /// Stable seed id used to generate the 3x3 pixel mark.
    pub id: SharedString,
    /// Human-readable agent name.
    pub name: SharedString,
}

impl RunAgentSpec {
    /// Construct a [`RunAgentSpec`].
    pub fn new(id: impl Into<SharedString>, name: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
        }
    }
}

/// Inline steer note anchored at a word index in the streamed answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunSteerChip {
    /// Word offset where the steer landed.
    pub word_index: usize,
    /// Steer instruction text.
    pub text: SharedString,
}

impl RunSteerChip {
    /// Create an inline steer marker at `word_index`.
    pub fn new(word_index: usize, text: impl Into<SharedString>) -> Self {
        Self {
            word_index,
            text: text.into(),
        }
    }
}

/// Message waiting in the tray above the composer until the current run finishes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunQueuedMessage {
    /// Stable message identifier.
    pub id: SharedString,
    /// Queued user prompt text.
    pub text: SharedString,
}

impl RunQueuedMessage {
    /// Create a queued follow-up message.
    pub fn new(id: impl Into<SharedString>, text: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
        }
    }
}

/// Typed events emitted by [`RunControls`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RunControlsEvent {
    /// Emitted when a new prompt is sent from `Idle` or `Done`.
    Sent(SharedString),
    /// Emitted when a steer note is injected mid-answer (`Enter` while `Running`).
    Steered {
        /// Word index where the steer landed.
        word_index: usize,
        /// Steer instruction text.
        text: SharedString,
    },
    /// Emitted when a follow-up message is added to the queue (`⌥↵` while `Running`).
    Queued {
        /// Queue item id.
        id: SharedString,
        /// Queued text.
        text: SharedString,
    },
    /// Emitted when a queued message is removed (`✕`).
    Unqueued(SharedString),
    /// Emitted when the active stream is stopped (`Esc` or Stop button).
    Stopped {
        /// Exact word index where streaming halted.
        word_index: usize,
    },
    /// Emitted when streaming resumes from `word_index`.
    Resumed {
        /// Exact word index where streaming picked back up.
        word_index: usize,
    },
    /// Emitted when `Retry` restarts or continues after error/stop.
    Retried {
        /// Word index from which streaming continues (`0` on full restart).
        from_word: usize,
    },
    /// Emitted when `Dismiss` clears a stopped or failed turn.
    Dismissed,
    /// Emitted when `Chat` / `Act` mode changes.
    ModeChanged(RunMode),
    /// Emitted when the permission posture changes.
    PermissionChanged(RunPermission),
}

impl EventEmitter<RunControlsEvent> for RunControls {}

/// Unified agent composer and live run-control deck.
pub struct RunControls {
    id: ElementId,
    focus: FocusHandle,
    agent: RunAgentSpec,
    prompt: Option<SharedString>,
    words: Vec<SharedString>,
    word_cursor: usize,
    status: RunControlStatus,
    error_note: SharedString,
    steers: Vec<RunSteerChip>,
    queue: Vec<RunQueuedMessage>,
    steer_slides: HashMap<usize, Slide>,
    queue_slides: HashMap<SharedString, Slide>,
    done_clock: PhaseClock,
    mode: RunMode,
    permission: RunPermission,
    perm_menu_open: bool,
    context_used: f32,
    took_secs: u32,
    words_per_sec: f32,
    stream_accum: f32,
    anim_started: Instant,
    last_frame: Option<Instant>,
    next_seq: usize,
    mode_sensor: ProSelectionSensor,
    editor: Option<Entity<Editor>>,
    editor_focused: bool,
    loading: bool,
    empty: bool,
    measured_width: Rc<Cell<f32>>,
    toast: Option<(SharedString, Instant)>,
}

impl RunControls {
    /// Create [`RunControls`] preloaded with the `"Trip planner"` live streamed itinerary.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<UiLanguage>(|_, cx| cx.notify())
            .detach();
        let full_answer = default_answer_text();
        let words = split_stream_words(full_answer);
        Self {
            id: id.into(),
            focus: cx.focus_handle(),
            agent: RunAgentSpec::new("travel", "Trip planner"),
            prompt: Some(
                "Plan four days in Lisbon in March – good food, one day trip, nothing too touristy."
                    .into(),
            ),
            word_cursor: 48.min(words.len()),
            words,
            status: RunControlStatus::Running,
            error_note: "Stream connection dropped while generating Day 2".into(),
            steers: vec![RunSteerChip::new(30, "Keep it to three days")],
            queue: vec![RunQueuedMessage::new(
                "q1",
                "Add a packing list for March weather",
            )],
            steer_slides: HashMap::new(),
            queue_slides: HashMap::new(),
            done_clock: PhaseClock::default(),
            mode: RunMode::Act,
            permission: RunPermission::AskFirst,
            perm_menu_open: false,
            context_used: 0.44,
            took_secs: 14,
            words_per_sec: 7.5,
            stream_accum: 0.0,
            anim_started: Instant::now(),
            last_frame: None,
            next_seq: 2,
            mode_sensor: pro_selection_sensor(),
            editor: None,
            editor_focused: false,
            loading: false,
            empty: false,
            measured_width: Rc::new(Cell::new(540.0)),
            toast: None,
        }
    }

    /// Configure the agent identity (`id` seeds the 3x3 pixel mark).
    pub fn with_agent(
        mut self,
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
    ) -> Self {
        self.agent = RunAgentSpec::new(id, name);
        self
    }

    /// Set or clear (`None`) the current user prompt.
    pub fn with_prompt(mut self, prompt: Option<SharedString>) -> Self {
        self.prompt = prompt;
        self
    }

    /// Set the target answer text and initial visible word cursor.
    pub fn with_answer(mut self, answer: impl Into<SharedString>) -> Self {
        let s = answer.into();
        self.words = split_stream_words(s.as_ref());
        self.word_cursor = if self.status == RunControlStatus::Done {
            self.words.len()
        } else {
            self.word_cursor.min(self.words.len())
        };
        self
    }

    /// Set the initial [`RunControlStatus`].
    pub fn with_status(mut self, status: RunControlStatus) -> Self {
        self.status = status;
        if status == RunControlStatus::Done {
            self.word_cursor = self.words.len();
        } else if status == RunControlStatus::Idle {
            self.word_cursor = 0;
        }
        self
    }

    /// Set pre-existing mid-answer [`RunSteerChip`] markers.
    pub fn with_steers(mut self, steers: Vec<RunSteerChip>) -> Self {
        self.steers = steers;
        self
    }

    /// Set queued follow-up messages above the composer.
    pub fn with_queue(mut self, queue: Vec<RunQueuedMessage>) -> Self {
        self.queue = queue;
        self
    }

    /// Set the initial [`RunMode`] (`Chat` or `Act`).
    pub fn with_mode(mut self, mode: RunMode) -> Self {
        self.mode = mode;
        self
    }

    /// Set the initial [`RunPermission`] posture.
    pub fn with_permission(mut self, permission: RunPermission) -> Self {
        self.permission = permission;
        self
    }

    /// Set the context window share in `0.0..=1.0` (turns orange at `>= 0.80`).
    pub fn with_context_used(mut self, ratio: f32) -> Self {
        self.context_used = ratio.clamp(0.0, 1.0);
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

    /// Return the active [`RunControlStatus`].
    pub fn status(&self) -> RunControlStatus {
        self.status
    }

    /// Return the exact word index reached so far.
    pub fn word_cursor(&self) -> usize {
        self.word_cursor
    }

    /// Return the active [`RunMode`].
    pub fn mode(&self) -> RunMode {
        self.mode
    }

    /// Return the active [`RunPermission`].
    pub fn permission(&self) -> RunPermission {
        self.permission
    }

    fn ensure_editor(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let desired_ph: SharedString = if self.status == RunControlStatus::Running {
            tr(
                cx,
                "Steer the agent, or queue what's next…",
                "中途引导当前回答，或排队下一条提问…",
            )
            .into()
        } else {
            tr(
                cx,
                "Ask a follow-up or start a new turn…",
                "继续追问或发起新一轮任务…",
            )
            .into()
        };

        if let Some(existing) = &self.editor {
            existing.update(cx, |e, _| e.set_placeholder(desired_ph.clone()));
            return;
        }

        let ed = cx.new(|cx| {
            let mut e = Editor::new(desired_ph.clone(), InputMode::Single, w, cx);
            e.set_placeholder(desired_ph);
            e.font_size = 13.0;
            e
        });
        cx.subscribe_in(&ed, w, |this, ed_ent, ev: &EditEvent, w, cx| match ev {
            EditEvent::Focused => {
                this.editor_focused = true;
                this.perm_menu_open = false;
                cx.notify();
            }
            EditEvent::Blurred => {
                this.editor_focused = false;
                cx.notify();
            }
            EditEvent::Changed => {
                cx.notify();
            }
            EditEvent::Submitted => {
                let text = ed_ent.read(cx).value().trim().to_owned();
                if !text.is_empty() {
                    this.submit_primary_draft(text, cx);
                    ed_ent.update(cx, |e, cx| e.set_value("", w, cx));
                }
            }
            EditEvent::Escape => {
                if this.perm_menu_open {
                    this.perm_menu_open = false;
                } else if this.status == RunControlStatus::Running {
                    this.stop_run(cx);
                } else {
                    this.focus.focus(w, cx);
                }
                cx.notify();
            }
            _ => {}
        })
        .detach();
        self.editor = Some(ed);
    }

    fn submit_primary_draft(&mut self, text: String, cx: &mut Context<Self>) {
        if self.status == RunControlStatus::Running {
            self.steer_now(text, cx);
        } else {
            self.send_new_turn(text, cx);
        }
    }

    /// Inject a mid-answer steer at the exact current `word_cursor`.
    pub fn steer_now(&mut self, raw: impl Into<String>, cx: &mut Context<Self>) {
        let trimmed = raw.into().trim().to_owned();
        if trimmed.is_empty() {
            return;
        }
        let at = self.word_cursor;
        let steer_text = SharedString::from(trimmed.clone());
        let mut st_slide = Slide::new(8.0);
        st_slide.duration(0.0, 0.22, cx);
        self.steer_slides.insert(self.steers.len(), st_slide);
        self.steers.push(RunSteerChip::new(at, steer_text.clone()));
        let extra_words = split_stream_words(&format!(
            "\n\nGot it ({trimmed}) – adjusting the remaining itinerary around that preference right away."
        ));
        let tail = self.words.split_off(at.min(self.words.len()));
        self.words.extend(extra_words);
        self.words.extend(tail);
        self.context_used = (self.context_used + 0.04).clamp(0.0, 0.99);
        cx.emit(RunControlsEvent::Steered {
            word_index: at,
            text: steer_text,
        });
        cx.notify();
    }

    /// Queue a message for after the current run (`⌥↵`).
    pub fn queue_message(&mut self, raw: impl Into<String>, cx: &mut Context<Self>) {
        let trimmed = raw.into().trim().to_owned();
        if trimmed.is_empty() {
            return;
        }
        let id = SharedString::from(format!("q-{}", self.next_seq));
        self.next_seq += 1;
        let text = SharedString::from(trimmed);
        let mut q_slide = Slide::new(8.0);
        q_slide.duration(0.0, 0.22, cx);
        self.queue_slides.insert(id.clone(), q_slide);
        self.queue.push(RunQueuedMessage {
            id: id.clone(),
            text: text.clone(),
        });
        cx.emit(RunControlsEvent::Queued { id, text });
        cx.notify();
    }

    /// Remove a queued message by `id`.
    pub fn remove_queued(&mut self, id: &SharedString, cx: &mut Context<Self>) {
        let before = self.queue.len();
        self.queue.retain(|q| &q.id != id);
        if self.queue.len() != before {
            cx.emit(RunControlsEvent::Unqueued(id.clone()));
            cx.notify();
        }
    }

    /// Start a fresh turn with `raw` as the prompt.
    pub fn send_new_turn(&mut self, raw: impl Into<String>, cx: &mut Context<Self>) {
        let trimmed = raw.into().trim().to_owned();
        if trimmed.is_empty() {
            return;
        }
        let prompt_ss = SharedString::from(trimmed.clone());
        self.prompt = Some(prompt_ss.clone());
        self.steers.clear();
        let reply = format!(
            "Here is a tailored plan for \"{trimmed}\":\n\n1. Start early with local neighbourhood spots and reserve ahead for dinner.\n2. Keep afternoon transitions light with walkable stops, backup indoor options, and clear timing buffers."
        );
        self.words = split_stream_words(&reply);
        self.word_cursor = 2.min(self.words.len());
        self.stream_accum = 0.0;
        self.status = RunControlStatus::Running;
        self.last_frame = Some(Instant::now());
        self.context_used = (self.context_used + 0.06).clamp(0.0, 0.99);
        cx.emit(RunControlsEvent::Sent(prompt_ss));
        cx.notify();
    }

    /// Stop the stream immediately, preserving `word_cursor` for exact-word [`Self::resume_run`].
    pub fn stop_run(&mut self, cx: &mut Context<Self>) {
        if self.status == RunControlStatus::Running {
            self.status = RunControlStatus::Stopped;
            self.last_frame = None;
            cx.emit(RunControlsEvent::Stopped {
                word_index: self.word_cursor,
            });
            cx.notify();
        }
    }

    /// Resume a stopped stream from the exact `word_cursor` where it stopped.
    pub fn resume_run(&mut self, cx: &mut Context<Self>) {
        if matches!(
            self.status,
            RunControlStatus::Stopped | RunControlStatus::Error
        ) {
            if self.word_cursor >= self.words.len() {
                self.word_cursor = 0;
            }
            self.status = RunControlStatus::Running;
            self.stream_accum = 0.0;
            self.last_frame = Some(Instant::now());
            cx.emit(RunControlsEvent::Resumed {
                word_index: self.word_cursor,
            });
            cx.notify();
        }
    }

    /// Trigger a mid-stream connection drop to inspect error recovery (`Retry` / `Dismiss`).
    pub fn simulate_error(&mut self, cx: &mut Context<Self>) {
        self.status = RunControlStatus::Error;
        self.last_frame = None;
        cx.notify();
    }

    /// Retry from the exact word on `Error`, or restart from word `0` when `Stopped` / `Done`.
    pub fn retry_run(&mut self, cx: &mut Context<Self>) {
        let from_word = if self.status == RunControlStatus::Error {
            self.word_cursor.min(self.words.len().saturating_sub(1))
        } else {
            self.word_cursor = 0;
            0
        };
        self.status = RunControlStatus::Running;
        self.stream_accum = 0.0;
        self.last_frame = Some(Instant::now());
        cx.emit(RunControlsEvent::Retried { from_word });
        cx.notify();
    }

    /// Dismiss a stopped or failed turn back to `Idle`.
    pub fn dismiss_turn(&mut self, cx: &mut Context<Self>) {
        self.status = RunControlStatus::Idle;
        self.prompt = None;
        self.word_cursor = 0;
        self.steers.clear();
        self.last_frame = None;
        cx.emit(RunControlsEvent::Dismissed);
        cx.notify();
    }

    /// Set `Chat` or `Act` mode.
    pub fn set_mode(&mut self, mode: RunMode, cx: &mut Context<Self>) {
        if self.mode != mode {
            self.mode = mode;
            if mode == RunMode::Chat {
                self.perm_menu_open = false;
            }
            cx.emit(RunControlsEvent::ModeChanged(mode));
            cx.notify();
        }
    }

    /// Set the permission posture.
    pub fn set_permission(&mut self, perm: RunPermission, cx: &mut Context<Self>) {
        self.perm_menu_open = false;
        if self.permission != perm {
            self.permission = perm;
            cx.emit(RunControlsEvent::PermissionChanged(perm));
        }
        cx.notify();
    }

    fn run_next_queued(&mut self, cx: &mut Context<Self>) {
        if !self.queue.is_empty() {
            let next = self.queue.remove(0);
            self.send_new_turn(next.text.to_string(), cx);
        }
    }

    fn handle_key_down(&mut self, ev: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        let key = ev.keystroke.key.as_str();
        if ev.keystroke.modifiers.alt
            && key == "enter"
            && let Some(ed) = self.editor.clone()
        {
            let draft = ed.read(cx).value().trim().to_owned();
            if !draft.is_empty() {
                self.queue_message(draft, cx);
                ed.update(cx, |e, cx| e.set_value("", w, cx));
                return;
            }
        }
        if key == "escape" {
            if self.perm_menu_open {
                self.perm_menu_open = false;
                cx.notify();
            } else if self.status == RunControlStatus::Running {
                self.stop_run(cx);
            }
            return;
        }
        if !self.editor_focused {
            match key {
                "left" => self.set_mode(RunMode::Chat, cx),
                "right" => self.set_mode(RunMode::Act, cx),
                _ => {}
            }
        }
    }

    fn render_pixel_mark(&self, busy: bool, elapsed_ms: f32, p: &ProPalette, cx: &App) -> Div {
        let reduced = Theme::global(cx).reduced_motion;
        let cells = agent_pixel_cells(&self.agent.id);
        let mark_col = p.blue;
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
                let idx = r * 3 + c;
                let on = cells[idx];
                let alpha = if busy && !reduced {
                    let ph = ((elapsed_ms + AGENT_WAVE_MS[idx]) / 1200.0) * std::f32::consts::TAU;
                    let wv = 0.58 + 0.42 * ph.cos();
                    if on { wv } else { 0.16 * wv }
                } else if on {
                    0.90
                } else {
                    0.0
                };
                row = row.child(
                    div()
                        .size(px(3.))
                        .rounded(px(0.8))
                        .bg(mark_col.opacity(alpha)),
                );
            }
            grid = grid.child(row);
        }
        grid
    }
}

impl Render for RunControls {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let reduced = disabled_motion(cx);
        let width = self.measured_width.get();
        let compact = width < 420.0;
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
                        .max_w(px(520.))
                        .rounded(px(10.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .p(px(16.))
                        .flex()
                        .flex_col()
                        .gap(px(12.))
                        .child(pro_skeleton_bar(280., 36., 10., 0.1, cx))
                        .child(pro_skeleton_bar(420., 110., 8., 0.35, cx))
                        .child(pro_skeleton_bar(480., 96., 8., 0.65, cx)),
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
                    ProIcon::Thread,
                    tr(cx, "Ready when you are", "智能体已就绪"),
                    tr(
                        cx,
                        "Ask a question or switch to Act mode to run tools with granular permission controls.",
                        "输入问题发起对话，或切换至执行模式按权限策略调用工具。",
                    ),
                    None,
                    cx,
                ));
        }

        self.ensure_editor(w, cx);

        // Advance word streaming while Running
        let now = Instant::now();
        if self.status == RunControlStatus::Running {
            if let Some(prev) = self.last_frame {
                let dt = now.duration_since(prev).as_secs_f32().min(0.10);
                self.stream_accum += dt * self.words_per_sec;
                let step_words = self.stream_accum.floor() as usize;
                if step_words > 0 {
                    self.stream_accum -= step_words as f32;
                    self.word_cursor = (self.word_cursor + step_words).min(self.words.len());
                }
            }
            self.last_frame = Some(now);
            if self.word_cursor >= self.words.len() {
                self.status = RunControlStatus::Done;
                self.done_clock.restart(cx);
                self.last_frame = None;
            } else {
                w.request_animation_frame();
            }
        } else {
            self.last_frame = None;
        }

        let elapsed_ms = self.anim_started.elapsed().as_secs_f32() * 1000.0;
        let is_running = self.status == RunControlStatus::Running;
        let done_age = self.done_clock.time(0.28, w, cx);
        let done_chk_on = self.status == RunControlStatus::Done && (reduced || done_age > 0.012);

        // Top conversation log (252px scrollable height)
        let log_body: AnyElement = if self.status == RunControlStatus::Idle || self.prompt.is_none()
        {
            div()
                .size_full()
                .px(px(24.))
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap(px(8.))
                .child(self.render_pixel_mark(false, elapsed_ms, &p, cx))
                .child(
                    div()
                        .text_size(px(13.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.ink)
                        .child(self.agent.name.clone()),
                )
                .child(
                    div()
                        .text_size(px(12.5))
                        .text_color(p.muted)
                        .text_center()
                        .child(tr(
                            cx,
                            "Send a prompt below—or try one of your queued messages.",
                            "在下方输入任务提示词，或直接运行排队中的下一条消息。",
                        )),
                )
                .when(!self.queue.is_empty(), |d| {
                    d.child(div().mt(px(4.)).child(pro_btn(
                        pro_eid(&self.id, "idle-run-queued"),
                        tr(cx, "Run queued message", "运行排队消息"),
                        Some(ProIcon::Play),
                        ProBtnStyle::Primary,
                        true,
                        cx,
                        |this, _, cx| this.run_next_queued(cx),
                    )))
                })
                .into_any_element()
        } else {
            let prompt_txt = self.prompt.clone().unwrap_or_default();
            let state_text = match self.status {
                RunControlStatus::Running => {
                    if self.steers.is_empty() {
                        tr(cx, "Writing", "正在输出").to_owned()
                    } else {
                        tr(cx, "Writing · steered", "正在输出 · 已纠偏").to_owned()
                    }
                }
                RunControlStatus::Stopped => format!(
                    "{} {} {}",
                    tr(cx, "Stopped at", "已停在第"),
                    self.word_cursor,
                    tr(cx, "words", "词")
                ),
                RunControlStatus::Error => tr(cx, "Connection dropped", "连接中断").to_owned(),
                RunControlStatus::Done => {
                    format!("{} 0:{:02}", tr(cx, "Done in", "完成耗时"), self.took_secs)
                }
                RunControlStatus::Idle => String::new(),
            };

            // Render streamed paragraphs + inline steer pills at their exact word_index
            let shown_limit = self.word_cursor.min(self.words.len());
            let mut flow = div()
                .pl(px(24.))
                .mt(px(6.))
                .flex()
                .flex_wrap()
                .items_center()
                .gap_x(px(3.5))
                .gap_y(px(2.))
                .text_size(px(13.))
                .line_height(px(20.))
                .text_color(p.ink);

            for idx in 0..shown_limit {
                for (st_i, st) in self
                    .steers
                    .iter()
                    .enumerate()
                    .filter(|(_, s)| s.word_index == idx)
                {
                    let dy = if reduced {
                        0.0
                    } else {
                        self.steer_slides.get(&st_i).map_or(0.0, |sl| sl.get(w, cx))
                    };
                    flow = flow.child(
                        div()
                            .relative()
                            .top(px(dy))
                            .h(px(20.))
                            .pl(px(6.))
                            .pr(px(8.))
                            .mx(px(2.))
                            .rounded_full()
                            .bg(p.accent_soft)
                            .flex()
                            .items_center()
                            .gap(px(4.))
                            .text_size(px(12.))
                            .text_color(p.blue)
                            .child(pro_svg("0 0 24 24", STEER_CORNER_SVG, 12., p.blue))
                            .child(st.text.clone()),
                    );
                }
                let tok = &self.words[idx];
                if tok.as_ref() == "\n\n" {
                    flow = flow.child(div().w_full().h(px(6.)));
                } else {
                    flow = flow.child(tok.clone());
                }
            }
            for (st_i, st) in self
                .steers
                .iter()
                .enumerate()
                .filter(|(_, s)| s.word_index == shown_limit)
            {
                let dy = if reduced {
                    0.0
                } else {
                    self.steer_slides.get(&st_i).map_or(0.0, |sl| sl.get(w, cx))
                };
                flow = flow.child(
                    div()
                        .relative()
                        .top(px(dy))
                        .h(px(20.))
                        .pl(px(6.))
                        .pr(px(8.))
                        .mx(px(2.))
                        .rounded_full()
                        .bg(p.accent_soft)
                        .flex()
                        .items_center()
                        .gap(px(4.))
                        .text_size(px(12.))
                        .text_color(p.blue)
                        .child(pro_svg("0 0 24 24", STEER_CORNER_SVG, 12., p.blue))
                        .child(st.text.clone()),
                );
            }

            // Caret / cut marker
            match self.status {
                RunControlStatus::Running => {
                    let caret_alpha = if reduced {
                        0.9
                    } else {
                        0.45 + 0.50 * ((elapsed_ms / 1000.0) * std::f32::consts::TAU).cos().abs()
                    };
                    flow = flow.child(
                        div()
                            .w(px(7.))
                            .h(px(14.))
                            .ml(px(2.))
                            .rounded(px(2.))
                            .bg(p.ink.opacity(caret_alpha)),
                    );
                }
                RunControlStatus::Stopped => {
                    flow = flow.child(
                        div()
                            .w(px(2.))
                            .h(px(14.))
                            .ml(px(2.))
                            .rounded(px(1.))
                            .bg(p.faint),
                    );
                }
                RunControlStatus::Error => {
                    flow = flow.child(
                        div()
                            .w(px(2.))
                            .h(px(14.))
                            .ml(px(2.))
                            .rounded(px(1.))
                            .bg(p.red_ink),
                    );
                }
                _ => {}
            }

            // Footer row inside log (`Esc to stop`, or `Resume` / `Retry` / `Dismiss`)
            let after_row = match self.status {
                RunControlStatus::Running => div()
                    .pl(px(24.))
                    .mt(px(10.))
                    .min_h(px(28.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(5.))
                            .text_size(px(12.))
                            .text_color(p.muted)
                            .child(
                                div()
                                    .px(px(5.))
                                    .py(px(1.))
                                    .rounded(px(4.))
                                    .bg(p.subtle)
                                    .border_1()
                                    .border_color(p.line_strong)
                                    .text_size(px(11.))
                                    .child("Esc"),
                            )
                            .child(tr(cx, "to stop", "停止输出")),
                    )
                    .child(pro_btn(
                        pro_eid(&self.id, "sim-drop-btn"),
                        tr(cx, "Simulate drop", "模拟中断"),
                        None,
                        ProBtnStyle::Ghost,
                        true,
                        cx,
                        |this, _, cx| this.simulate_error(cx),
                    )),
                RunControlStatus::Stopped => div()
                    .pl(px(24.))
                    .mt(px(10.))
                    .min_h(px(28.))
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .child(div().text_size(px(12.)).text_color(p.muted).child(tr(
                        cx,
                        "Paused mid-sentence—resume from the exact word",
                        "已在词位暂停，可从当前词无缝续写",
                    )))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(4.))
                            .child(pro_btn(
                                pro_eid(&self.id, "act-resume"),
                                tr(cx, "Resume", "继续"),
                                Some(ProIcon::Play),
                                ProBtnStyle::Primary,
                                true,
                                cx,
                                |this, _, cx| this.resume_run(cx),
                            ))
                            .child(pro_btn(
                                pro_eid(&self.id, "act-retry-stop"),
                                tr(cx, "Retry", "重试"),
                                Some(ProIcon::Refresh),
                                ProBtnStyle::Ghost,
                                true,
                                cx,
                                |this, _, cx| this.retry_run(cx),
                            ))
                            .child(pro_btn(
                                pro_eid(&self.id, "act-dismiss-stop"),
                                tr(cx, "Dismiss", "忽略"),
                                None,
                                ProBtnStyle::Ghost,
                                true,
                                cx,
                                |this, _, cx| this.dismiss_turn(cx),
                            )),
                    ),
                RunControlStatus::Error => div()
                    .pl(px(24.))
                    .mt(px(10.))
                    .min_h(px(28.))
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(p.red_ink)
                            .child(self.error_note.clone()),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(4.))
                            .child(pro_btn(
                                pro_eid(&self.id, "act-retry-err"),
                                tr(cx, "Retry from here", "从断点重试"),
                                Some(ProIcon::Refresh),
                                ProBtnStyle::Primary,
                                true,
                                cx,
                                |this, _, cx| this.retry_run(cx),
                            ))
                            .child(pro_btn(
                                pro_eid(&self.id, "act-dismiss-err"),
                                tr(cx, "Dismiss", "关闭"),
                                None,
                                ProBtnStyle::Ghost,
                                true,
                                cx,
                                |this, _, cx| this.dismiss_turn(cx),
                            )),
                    ),
                RunControlStatus::Done => div()
                    .pl(px(24.))
                    .mt(px(10.))
                    .min_h(px(28.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .child(div().text_size(px(12.)).text_color(p.muted).child(format!(
                        "{} {}",
                        self.words.len(),
                        tr(cx, "words", "词")
                    )))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .when(!self.queue.is_empty(), |d| {
                                d.child(pro_btn(
                                    pro_eid(&self.id, "act-run-next"),
                                    tr(cx, "Send Next queued", "发送排队消息"),
                                    None,
                                    ProBtnStyle::Secondary,
                                    true,
                                    cx,
                                    |this, _, cx| this.run_next_queued(cx),
                                ))
                            })
                            .child(pro_btn(
                                pro_eid(&self.id, "act-retry-done"),
                                tr(cx, "Retry", "重新生成"),
                                Some(ProIcon::Refresh),
                                ProBtnStyle::Ghost,
                                true,
                                cx,
                                |this, _, cx| this.retry_run(cx),
                            )),
                    ),
                RunControlStatus::Idle => div(),
            };

            div()
                .w_full()
                .flex()
                .flex_col()
                .child(
                    div().w_full().flex().justify_end().child(
                        div()
                            .max_w(relative(0.84))
                            .px(px(12.))
                            .py(px(7.))
                            .rounded_tl(px(14.))
                            .rounded_tr(px(14.))
                            .rounded_bl(px(14.))
                            .rounded_br(px(4.))
                            .bg(p.subtle)
                            .border_1()
                            .border_color(p.line)
                            .text_size(px(13.))
                            .line_height(px(19.))
                            .text_color(p.ink)
                            .child(prompt_txt),
                    ),
                )
                .child(
                    div()
                        .mt(px(14.))
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .text_size(px(12.))
                        .child(if self.status == RunControlStatus::Done {
                            pro_check_draw(
                                pro_eid(&self.id, "rc-done-chk"),
                                done_chk_on,
                                16.0,
                                false,
                                p.green,
                            )
                        } else {
                            self.render_pixel_mark(is_running, elapsed_ms, &p, cx)
                                .into_any_element()
                        })
                        .child(
                            div()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.ink)
                                .truncate()
                                .child(self.agent.name.clone()),
                        )
                        .child(pro_swap_label(
                            pro_eid(&self.id, "rc-state-lbl"),
                            state_text,
                            12.0,
                            FontWeight::NORMAL,
                            if self.status == RunControlStatus::Error {
                                p.red_ink
                            } else {
                                p.muted
                            },
                        )),
                )
                .child(flow)
                .child(after_row)
                .into_any_element()
        };

        // Queued messages list inside composer tray (wrapped in smooth `pro_fold`)
        let has_queue = !self.queue.is_empty();
        let mut queue_rows = div()
            .id(pro_eid(&self.id, "tray-queue-scroll"))
            .max_h(px(124.))
            .overflow_y_scroll()
            .px(px(4.))
            .pt(px(2.))
            .pb(px(4.))
            .flex()
            .flex_col();

        for (q_idx, q_msg) in self.queue.iter().enumerate() {
            let qid = q_msg.id.clone();
            let pos_lbl = if q_idx == 0 {
                tr(cx, "Next", "下一条").to_owned()
            } else {
                format!("#{}", q_idx + 1)
            };
            let q_dy = if reduced {
                0.0
            } else {
                self.queue_slides
                    .get(&q_msg.id)
                    .map_or(0.0, |sl| sl.get(w, cx))
            };
            queue_rows = queue_rows.child(
                div()
                    .relative()
                    .top(px(q_dy))
                    .h(px(30.))
                    .pl(px(8.))
                    .pr(px(2.))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .text_size(px(12.5))
                    .child(pro_svg("0 0 24 24", QUEUE_LINES_SVG, 13., p.muted))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_color(p.ink)
                            .truncate()
                            .child(q_msg.text.clone()),
                    )
                    .child(
                        div()
                            .flex_none()
                            .text_size(px(11.5))
                            .text_color(p.faint)
                            .child(pos_lbl),
                    )
                    .child(pro_icon_btn(
                        pro_eid(&self.id, format!("unq-{}", q_msg.id)),
                        tr(cx, "Remove queued prompt", "移除排队提示词"),
                        ProIcon::Close,
                        24.0,
                        false,
                        cx,
                        move |this, _, cx| {
                            this.remove_queued(&qid, cx);
                        },
                    )),
            );
        }
        let queue_tray_fold = pro_fold(
            pro_eid(&self.id, "rc-queue-fold"),
            has_queue,
            38.0,
            0.0,
            move || queue_rows.into_any_element(),
        );

        // Mode toggle (Chat | Act) with pro_pill_indicator
        let mode_strip = div()
            .relative()
            .p(px(2.))
            .rounded(px(7.))
            .bg(p.subtle)
            .border_1()
            .border_color(p.line)
            .flex()
            .items_center()
            .child(pro_pill_indicator(
                pro_eid(&self.id, "mode-pill"),
                self.mode.key(),
                self.mode_sensor.clone(),
                5.,
                p.card,
                Some(p.line),
                true,
            ))
            .children(
                [
                    (RunMode::Chat, tr(cx, "Chat", "对话")),
                    (RunMode::Act, tr(cx, "Act", "执行")),
                ]
                .into_iter()
                .map(|(m, lbl)| {
                    let selected = self.mode == m;
                    div()
                        .id(pro_eid(&self.id, format!("mode-{}", m.key())))
                        .role(Role::RadioButton)
                        .aria_toggled(selected.into())
                        .tab_index(if selected { 0 } else { -1 })
                        .relative()
                        .h(px(24.))
                        .px(px(10.))
                        .rounded(px(5.))
                        .flex()
                        .items_center()
                        .cursor_pointer()
                        .text_size(px(12.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(if selected { p.ink } else { p.muted })
                        .hover(move |s| if selected { s } else { s.text_color(p.ink) })
                        .on_click(cx.listener(move |this, _, _, cx| this.set_mode(m, cx)))
                        .when(selected, |d| d.child(pro_mark_selected(&self.mode_sensor)))
                        .child(lbl)
                }),
            );

        // Permission posture button & `pro_popover_presence` menu
        let act_enabled = self.mode == RunMode::Act;
        let is_alert_perm = act_enabled && self.permission == RunPermission::FullAccess;
        let perm_fg = if is_alert_perm { p.red_ink } else { p.muted };
        let perm_btn = div()
            .id(pro_eid(&self.id, "perm-trigger"))
            .role(Role::Button)
            .aria_expanded(self.perm_menu_open && act_enabled)
            .aria_label(SharedString::from(format!(
                "{}: {}",
                tr(cx, "Permissions", "权限策略"),
                self.permission.label(cx)
            )))
            .tab_index(0)
            .h(px(28.))
            .px(px(8.))
            .rounded(px(7.))
            .opacity(if act_enabled { 1.0 } else { 0.45 })
            .when(act_enabled, |d| {
                d.cursor_pointer()
                    .hover(move |s| {
                        if is_alert_perm {
                            s.bg(p.red_soft)
                        } else {
                            s.bg(p.hover).text_color(p.ink)
                        }
                    })
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.perm_menu_open = !this.perm_menu_open;
                        cx.notify();
                    }))
            })
            .flex()
            .items_center()
            .gap(px(5.))
            .text_size(px(12.5))
            .text_color(perm_fg)
            .child(pro_icon(
                if self.permission == RunPermission::FullAccess {
                    ProIcon::Bolt
                } else {
                    ProIcon::Shield
                },
                14.,
                perm_fg,
            ))
            .when(!compact, |d| {
                d.child(pro_swap_label(
                    pro_eid(&self.id, "perm-trig-lbl"),
                    self.permission.label(cx),
                    12.5,
                    FontWeight::NORMAL,
                    perm_fg,
                ))
            })
            .child(pro_icon(
                if self.perm_menu_open && act_enabled {
                    ProIcon::ChevronUp
                } else {
                    ProIcon::ChevronDown
                },
                11.,
                perm_fg,
            ));

        let cur_perm = self.permission;
        let mut perm_menu_col = div()
            .w(px(272.))
            .p(px(4.))
            .rounded(px(10.))
            .bg(if p.dark { rgb(0x1f1f23).into() } else { p.card })
            .border_1()
            .border_color(p.line_strong)
            .shadow_lg()
            .flex()
            .flex_col()
            .gap(px(2.));
        for pm in RunPermission::ALL {
            let checked = cur_perm == pm;
            let danger = pm == RunPermission::FullAccess;
            let ic_col = if danger { p.red_ink } else { p.muted };
            let opt_inner = div()
                .w_full()
                .px(px(8.))
                .py(px(6.))
                .flex()
                .items_center()
                .gap(px(9.))
                .child(pro_icon(
                    if danger {
                        ProIcon::Bolt
                    } else if pm == RunPermission::AutoEdits {
                        ProIcon::Sparkles
                    } else {
                        ProIcon::Shield
                    },
                    14.,
                    ic_col,
                ))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .text_size(px(12.5))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(if danger { p.red_ink } else { p.ink })
                                .child(pm.label(cx)),
                        )
                        .child(
                            div()
                                .text_size(px(11.5))
                                .text_color(p.muted)
                                .child(pm.description(cx)),
                        ),
                )
                .child(pro_check_draw(
                    pro_eid(&self.id, format!("perm-chk-{pm:?}")),
                    checked,
                    15.0,
                    false,
                    if danger { p.red } else { p.primary },
                ));
            perm_menu_col = perm_menu_col.child(
                pro_tactile(
                    pro_eid(&self.id, format!("perm-opt-{pm:?}")),
                    pm.label(cx),
                    6.0,
                    transparent_black(),
                    p.hover,
                    transparent_black(),
                    transparent_black(),
                    checked,
                    opt_inner,
                    cx,
                    move |this, _, cx| {
                        this.set_permission(pm, cx);
                    },
                )
                .w_full(),
            );
        }
        let perm_menu = div()
            .absolute()
            .bottom(px(34.0))
            .left(px(0.))
            .child(pro_popover_presence(
                pro_eid(&self.id, "perm-menu-pres"),
                self.perm_menu_open && act_enabled,
                move || perm_menu_col.into_any_element(),
            ));

        // Context-window fill ring + right action controls
        let ctx_pct = (self.context_used * 100.0).round() as u32;
        let ctx_warn = self.context_used >= 0.80;
        let ring_col = if ctx_warn { p.orange } else { p.muted };
        let has_draft = self
            .editor
            .as_ref()
            .map(|e| !e.read(cx).value().trim().is_empty())
            .unwrap_or(false);

        let ed_for_queue = self.editor.clone();
        let queue_inline_btn = (is_running && has_draft).then(|| {
            pro_btn(
                pro_eid(&self.id, "inline-queue-btn"),
                format!("{}  ⌥↵", tr(cx, "Queue", "排队")),
                None,
                ProBtnStyle::Secondary,
                true,
                cx,
                move |this, w, cx| {
                    if let Some(ed) = &ed_for_queue {
                        let v = ed.read(cx).value().trim().to_owned();
                        if !v.is_empty() {
                            this.queue_message(v, cx);
                            ed.update(cx, |e, cx| e.set_value("", w, cx));
                        }
                    }
                },
            )
        });

        let ed_for_round = self.editor.clone();
        let round_st = w.use_keyed_state(pro_eid(&self.id, "rc-round-st"), cx, |_, _| {
            (Slide::snappy(0.0), Slide::snappy(0.0))
        });
        let (round_hov, round_down) = round_st.update(cx, |s, cx| {
            (
                s.0.get(w, cx).clamp(0.0, 1.0),
                s.1.get(w, cx).clamp(0.0, 1.0),
            )
        });
        let round_bg = pro_mix_color(
            p.primary,
            pro_mix_color(p.primary, p.on_primary, 0.14),
            round_hov,
        );
        let round_scale = 1.0 - 0.08 * round_down;
        let round_side = 28.0 * round_scale;
        let round_inset = (28.0 - round_side) * 0.5;
        let round_btn = if is_running && !has_draft {
            div()
                .id(pro_eid(&self.id, "round-stop-btn"))
                .role(Role::Button)
                .aria_label(SharedString::from(tr(
                    cx,
                    "Stop the run (Esc)",
                    "停止当前输出 (Esc)",
                )))
                .tab_index(0)
                .size(px(28.))
                .relative()
                .flex_none()
                .rounded_full()
                .cursor_pointer()
                .on_hover({
                    let round_st = round_st.clone();
                    move |inside, w, cx| {
                        round_st.update(cx, |s, cx| {
                            s.0.set(if *inside { 1.0 } else { 0.0 }, cx);
                            if !*inside {
                                s.1.set(0.0, cx);
                            }
                            cx.notify();
                        });
                        w.refresh();
                    }
                })
                .on_mouse_down(MouseButton::Left, {
                    let round_st = round_st.clone();
                    move |_, w, cx| {
                        round_st.update(cx, |s, cx| {
                            s.1.set(1.0, cx);
                            cx.notify();
                        });
                        w.refresh();
                    }
                })
                .on_mouse_up(MouseButton::Left, {
                    let round_st = round_st.clone();
                    move |_, w, cx| {
                        round_st.update(cx, |s, cx| {
                            s.1.set(0.0, cx);
                            cx.notify();
                        });
                        w.refresh();
                    }
                })
                .on_click(cx.listener(|this, _, _, cx| this.stop_run(cx)))
                .child(
                    div()
                        .absolute()
                        .left(px(round_inset))
                        .top(px(round_inset))
                        .size(px(round_side))
                        .rounded_full()
                        .bg(round_bg)
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(pro_svg(
                            "0 0 24 24",
                            STOP_SQUARE_SVG,
                            14. * round_scale,
                            p.on_primary,
                        )),
                )
        } else {
            div()
                .id(pro_eid(&self.id, "round-send-btn"))
                .role(Role::Button)
                .aria_label(SharedString::from(if is_running {
                    tr(cx, "Steer now (Enter)", "立即引导 (Enter)")
                } else {
                    tr(cx, "Send message", "发送消息")
                }))
                .tab_index(0)
                .size(px(28.))
                .relative()
                .flex_none()
                .rounded_full()
                .opacity(if has_draft { 1.0 } else { 0.24 })
                .when(has_draft, |d| {
                    d.cursor_pointer()
                        .on_hover({
                            let round_st = round_st.clone();
                            move |inside, w, cx| {
                                round_st.update(cx, |s, cx| {
                                    s.0.set(if *inside { 1.0 } else { 0.0 }, cx);
                                    if !*inside {
                                        s.1.set(0.0, cx);
                                    }
                                    cx.notify();
                                });
                                w.refresh();
                            }
                        })
                        .on_mouse_down(MouseButton::Left, {
                            let round_st = round_st.clone();
                            move |_, w, cx| {
                                round_st.update(cx, |s, cx| {
                                    s.1.set(1.0, cx);
                                    cx.notify();
                                });
                                w.refresh();
                            }
                        })
                        .on_mouse_up(MouseButton::Left, {
                            let round_st = round_st.clone();
                            move |_, w, cx| {
                                round_st.update(cx, |s, cx| {
                                    s.1.set(0.0, cx);
                                    cx.notify();
                                });
                                w.refresh();
                            }
                        })
                        .on_click(cx.listener(move |this, _, w, cx| {
                            if let Some(ed) = &ed_for_round {
                                let v = ed.read(cx).value().trim().to_owned();
                                if !v.is_empty() {
                                    this.submit_primary_draft(v, cx);
                                    ed.update(cx, |e, cx| e.set_value("", w, cx));
                                }
                            }
                        }))
                })
                .child(
                    div()
                        .absolute()
                        .left(px(if has_draft { round_inset } else { 0.0 }))
                        .top(px(if has_draft { round_inset } else { 0.0 }))
                        .size(px(if has_draft { round_side } else { 28.0 }))
                        .rounded_full()
                        .bg(if has_draft { round_bg } else { p.primary })
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(pro_icon(ProIcon::ArrowUp, 14., p.on_primary)),
                )
        };

        let write_card = div()
            .w_full()
            .p(px(10.))
            .rounded(px(8.))
            .bg(p.card)
            .border_1()
            .border_color(if self.editor_focused {
                p.line_strong
            } else {
                p.line
            })
            .flex()
            .flex_col()
            .gap(px(10.))
            .when_some(self.editor.clone(), |d, ed| {
                d.child(div().w_full().h(px(34.)).px(px(2.)).child(ed))
            })
            .child(
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
                            .gap(px(6.))
                            .child(mode_strip)
                            .child(div().relative().child(perm_btn).child(perm_menu)),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(
                                div()
                                    .h(px(28.))
                                    .flex()
                                    .items_center()
                                    .gap(px(5.))
                                    .child(pro_progress_ring(
                                        self.context_used,
                                        15.,
                                        2.0,
                                        ring_col,
                                        p.line_strong,
                                    ))
                                    .when(!compact, |d| {
                                        d.child(pro_rolling_number(
                                            pro_eid(&self.id, "rc-ctx-pct"),
                                            ctx_pct as f64,
                                            0,
                                            "",
                                            "%",
                                            12.0,
                                            FontWeight::NORMAL,
                                            if ctx_warn { p.orange } else { p.muted },
                                        ))
                                    }),
                            )
                            .when_some(queue_inline_btn, |d, qb| d.child(qb))
                            .child(round_btn),
                    ),
            );

        let composer_tray = div()
            .w_full()
            .p(px(4.))
            .rounded(px(10.))
            .bg(p.tray)
            .flex()
            .flex_col()
            .child(queue_tray_fold)
            .child(write_card);

        let outer_card = div()
            .w_full()
            .max_w(px(520.))
            .rounded(px(10.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .shadow_sm()
            .flex()
            .flex_col()
            .child(
                div()
                    .id(pro_eid(&self.id, "conv-log-scroll"))
                    .w_full()
                    .h(px(254.))
                    .p(px(16.))
                    .overflow_y_scroll()
                    .child(log_body),
            )
            .child(composer_tray);

        div()
            .id(self.id.clone())
            .role(Role::Region)
            .aria_label(SharedString::from(format!("{} run", self.agent.name)))
            .track_focus(&self.focus)
            .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                if this.perm_menu_open {
                    this.perm_menu_open = false;
                    cx.notify();
                }
            }))
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
            .child(outer_card)
            .child(pro_toast_banner(self.toast.clone()))
    }
}

fn split_stream_words(text: &str) -> Vec<SharedString> {
    let mut out = Vec::new();
    for (p_idx, para) in text.split("\n\n").enumerate() {
        if p_idx > 0 {
            out.push("\n\n".into());
        }
        for w in para.split_whitespace() {
            out.push(SharedString::from(w.to_owned()));
        }
    }
    out
}

fn default_answer_text() -> &'static str {
    "Day 1 – Alfama and Graça. Start at the Miradouro da Senhora do Monte before the crowds, then walk down through Graça to a late lunch at a tasca near the Feira da Ladra.\n\nThree days it is – the market moves to Day 1's afternoon.\n\nDay 2 – Day trip to Setúbal. Grilled cuttlefish by the harbour, a boat out to the Arrábida coves, and Moscatel tasting in Azeitão on the way back.\n\nDay 3 – Príncipe Real and Estrela. Pastries at morning opening, independent bookshops, a quiet garden stroll, and seafood rice tucked off Rua das Flores."
}

#[cfg(test)]
mod tests {
    use super::{
        RunControlStatus, RunMode, RunPermission, default_answer_text, split_stream_words,
    };

    #[test]
    fn split_stream_words_preserves_paragraph_breaks_and_word_indices() {
        let words = split_stream_words(default_answer_text());
        assert!(words.len() > 70);
        assert_eq!(words[0].as_ref(), "Day");
        assert!(words.iter().any(|w| w.as_ref() == "\n\n"));
    }

    #[test]
    fn defaults_use_act_mode_and_ask_first_posture() {
        assert_eq!(RunMode::default(), RunMode::Act);
        assert_eq!(RunPermission::default(), RunPermission::AskFirst);
        assert_eq!(RunControlStatus::default(), RunControlStatus::Running);
    }
}
