//! Live mid-run agent steering card with ticking plan steps, next-step queue, and instant interrupt.
//!
//! Displays an agent's ordered plan ticking forward above a steering [`Editor`] composer:
//! notes queued with `Enter` wait in the `"Queued for the next step"` tray and glide smoothly
//! into the next step when the current step finishes, while `⌘↵` or `"Now"` interrupts immediately.

use crate::{
    components::pro_chrome::{
        ProBtnStyle, ProPalette, pro_btn, pro_check_draw, pro_ease, pro_eid, pro_empty_state,
        pro_fold, pro_icon_btn, pro_mark_selected, pro_pill_indicator, pro_popover_presence,
        pro_rolling_number, pro_selection_sensor, pro_shimmer_text, pro_skeleton_bar,
        pro_swap_label, pro_tactile, pro_toast_banner, tr,
    },
    foundation::Theme,
    primitives::{
        control::UiLanguage,
        editor::{EditEvent, Editor, InputMode},
        family::{PhaseClock, Slide, disabled_motion},
        pro_icons::{ProIcon, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, collections::HashMap, rc::Rc};
use web_time::Instant;

const SPARK_FOUR_SVG: &str = r#"<path d="M7 1.2c.4 2.9 1.9 4.4 4.8 4.8-2.9.4-4.4 1.9-4.8 4.8C6.6 7.9 5.1 6.4 2.2 6 5.1 5.6 6.6 4.1 7 1.2Z" fill="currentColor"></path><circle cx="11.6" cy="11.4" r="1.1" fill="currentColor" opacity="0.55"></circle>"#;
const PENDING_DASH_SVG: &str = r#"<circle cx="8" cy="8" r="5.8" fill="none" stroke="currentColor" stroke-width="1.2" stroke-dasharray="2 2.2"></circle>"#;
const BOLT_MINI_SVG: &str =
    r#"<path d="M6.9 1 2.6 6.6h3l-.5 4.4 4.3-5.6h-3L6.9 1Z" fill="currentColor"></path>"#;
const CORNER_NOTE_SVG: &str = r#"<path d="M3.5 2.5v4.2c0 1.4 1.1 2.5 2.5 2.5h4.5m-2-2.2 2.3 2.2-2.3 2.2" fill="none" stroke="currentColor" stroke-width="1.35" stroke-linecap="round" stroke-linejoin="round"></path>"#;

/// Overall run status of [`AgentSteering`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SteerRunStatus {
    /// Actively executing the current step and ticking elapsed clocks.
    #[default]
    Running,
    /// Paused by the user; clocks hold their current value.
    Paused,
    /// All steps finished; steering composer folds closed.
    Done,
}

/// Execution state of an individual [`SteerStep`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SteerStepStatus {
    /// Completed step.
    Done,
    /// Step currently executing.
    Active,
    /// Upcoming step waiting in the plan.
    #[default]
    Pending,
}

/// How a [`SteerStepNote`] entered a plan step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SteerPickedMode {
    /// Picked up cleanly from the queue when the step started.
    NextStep,
    /// Injected immediately via `"Interrupt now"` (`⌘↵` or `"Now"`).
    InterruptNow,
}

/// Steering instruction attached to a [`SteerStep`] after pickup or interrupt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SteerStepNote {
    /// Stable note identifier.
    pub id: SharedString,
    /// Instruction text.
    pub text: SharedString,
    /// Whether this note arrived at step boundary or interrupted mid-step.
    pub picked: SteerPickedMode,
    /// Timestamp when the note landed in the step (drives the `620ms` glide + `1400ms` glow).
    pub landed_at: Option<Instant>,
}

impl SteerStepNote {
    /// Construct a landed step note.
    pub fn new(
        id: impl Into<SharedString>,
        text: impl Into<SharedString>,
        picked: SteerPickedMode,
    ) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            picked,
            landed_at: None,
        }
    }
}

/// Single step in the [`AgentSteering`] execution plan.
#[derive(Clone, Debug, PartialEq)]
pub struct SteerStep {
    /// Stable step identifier.
    pub id: SharedString,
    /// Step description.
    pub title: SharedString,
    /// Execution status (`Done`, `Active`, `Pending`).
    pub status: SteerStepStatus,
    /// Duration in seconds (final duration when `Done`, base elapsed seconds when `Active`).
    pub duration_secs: f32,
    /// Steering notes absorbed by this step.
    pub notes: Vec<SteerStepNote>,
}

impl SteerStep {
    /// Create a [`SteerStep`] with `id`, `title`, and `status`.
    pub fn new(
        id: impl Into<SharedString>,
        title: impl Into<SharedString>,
        status: SteerStepStatus,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            status,
            duration_secs: 0.0,
            notes: Vec::new(),
        }
    }

    /// Set the step duration in seconds.
    pub fn with_duration_secs(mut self, secs: f32) -> Self {
        self.duration_secs = secs.max(0.0);
        self
    }

    /// Preload steering notes already attached to this step.
    pub fn with_notes(mut self, notes: Vec<SteerStepNote>) -> Self {
        self.notes = notes;
        self
    }
}

/// Steering note waiting in the `"Queued for the next step"` tray.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SteerQueuedNote {
    /// Stable note identifier.
    pub id: SharedString,
    /// Instruction text.
    pub text: SharedString,
}

impl SteerQueuedNote {
    /// Create a queued steering note.
    pub fn new(id: impl Into<SharedString>, text: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
        }
    }
}

/// Typed events emitted by [`AgentSteering`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentSteeringEvent {
    /// Emitted when a note is queued for the next step (`Enter`).
    Queued {
        /// Generated or caller note id.
        id: SharedString,
        /// Note text.
        text: SharedString,
    },
    /// Emitted when the active step is interrupted immediately (`⌘↵` or `"Now"`).
    Interrupted {
        /// Note id.
        id: SharedString,
        /// Note text.
        text: SharedString,
        /// `true` when promoted from an already-queued chip via `"Now"`.
        from_queue: bool,
    },
    /// Emitted when a queued note is removed via `✕`.
    Removed(SharedString),
    /// Emitted when the run is paused, resumed, or completed.
    StatusChanged(SteerRunStatus),
    /// Emitted when the agent finishes a step and pulls queued notes into the next step.
    StepAdvanced {
        /// Newly active 0-based step index.
        step_index: usize,
        /// Number of queued notes absorbed into that step.
        picked_count: usize,
    },
}

impl EventEmitter<AgentSteeringEvent> for AgentSteering {}

/// Mid-run steering card pairing a live step-by-step plan with next-step queue and instant interrupt.
pub struct AgentSteering {
    id: ElementId,
    focus: FocusHandle,
    title: SharedString,
    agent: SharedString,
    steps: Vec<SteerStep>,
    queue: Vec<SteerQueuedNote>,
    status: SteerRunStatus,
    act_mode: bool,
    policy_idx: usize,
    policy_open: bool,
    step_done_clocks: HashMap<SharedString, PhaseClock>,
    queue_slides: HashMap<SharedString, Slide>,
    base_run_secs: f32,
    step_budget_secs: f32,
    auto_tick: bool,
    last_frame: Option<Instant>,
    active_step_accum: f32,
    run_accum: f32,
    next_seq: usize,
    placeholder: Option<SharedString>,
    editor: Option<Entity<Editor>>,
    editor_focused: bool,
    loading: bool,
    empty: bool,
    measured_width: Rc<Cell<f32>>,
    toast: Option<(SharedString, Instant)>,
}

impl AgentSteering {
    /// Create an [`AgentSteering`] card preloaded with the `"Migrate users table"` live run.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<UiLanguage>(|_, cx| cx.notify())
            .detach();
        Self {
            id: id.into(),
            focus: cx.focus_handle(),
            title: "Migrate users table".into(),
            agent: "Opus 5.5".into(),
            steps: default_steer_steps(),
            queue: default_steer_queue(),
            status: SteerRunStatus::Running,
            act_mode: true,
            policy_idx: 0,
            policy_open: false,
            step_done_clocks: HashMap::new(),
            queue_slides: HashMap::new(),
            base_run_secs: 128.0,
            step_budget_secs: 7.5,
            auto_tick: true,
            last_frame: None,
            active_step_accum: 0.0,
            run_accum: 0.0,
            next_seq: 2,
            placeholder: None,
            editor: None,
            editor_focused: false,
            loading: false,
            empty: false,
            measured_width: Rc::new(Cell::new(520.0)),
            toast: None,
        }
    }

    /// Set the task heading displayed in the card header.
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = title.into();
        self
    }

    /// Set the agent or model name shown beneath the title.
    pub fn with_agent(mut self, agent: impl Into<SharedString>) -> Self {
        self.agent = agent.into();
        self
    }

    /// Supply custom plan steps.
    pub fn with_steps(mut self, steps: Vec<SteerStep>) -> Self {
        self.steps = steps;
        self
    }

    /// Supply initial queued notes waiting for the next step.
    pub fn with_queue(mut self, queue: Vec<SteerQueuedNote>) -> Self {
        self.queue = queue;
        self
    }

    /// Set the initial [`SteerRunStatus`].
    pub fn with_status(mut self, status: SteerRunStatus) -> Self {
        self.status = status;
        self
    }

    /// Control whether active steps automatically tick and advance (`true` by default).
    pub fn with_auto_tick(mut self, auto_tick: bool) -> Self {
        self.auto_tick = auto_tick;
        self
    }

    /// Override the steering composer placeholder text.
    pub fn with_placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = Some(placeholder.into());
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

    /// Return the current [`SteerRunStatus`].
    pub fn status(&self) -> SteerRunStatus {
        self.status
    }

    /// Return the queued notes slice.
    pub fn queue(&self) -> &[SteerQueuedNote] {
        &self.queue
    }

    /// Return the plan steps slice.
    pub fn steps(&self) -> &[SteerStep] {
        &self.steps
    }

    fn ensure_editor(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.editor.is_none() {
            let ph: SharedString = self
                .placeholder
                .clone()
                .unwrap_or_else(|| tr(cx, "Steer the agent…", "输入引导指令干预智能体…").into());
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Single, w, cx);
                e.set_placeholder(ph);
                e.font_size = 13.0;
                e
            });
            cx.subscribe_in(&ed, w, |this, ed_ent, ev: &EditEvent, w, cx| match ev {
                EditEvent::Focused => {
                    this.editor_focused = true;
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
                    let val = ed_ent.read(cx).value().trim().to_owned();
                    if !val.is_empty() {
                        this.queue_note(val, cx);
                        ed_ent.update(cx, |e, cx| e.set_value("", w, cx));
                    }
                }
                EditEvent::Escape => {
                    let has_val = !ed_ent.read(cx).value().is_empty();
                    if has_val {
                        ed_ent.update(cx, |e, cx| e.set_value("", w, cx));
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
    }

    /// Queue a steering note for the next step (`Enter`).
    pub fn queue_note(&mut self, raw: impl Into<String>, cx: &mut Context<Self>) {
        let trimmed = raw.into().trim().to_owned();
        if trimmed.is_empty() || self.status == SteerRunStatus::Done {
            return;
        }
        let id = SharedString::from(format!("steer-{}", self.next_seq));
        self.next_seq += 1;
        let text = SharedString::from(trimmed);
        let mut slide = Slide::new(10.0);
        slide.duration(0.0, 0.24, cx);
        self.queue_slides.insert(id.clone(), slide);
        self.queue.push(SteerQueuedNote {
            id: id.clone(),
            text: text.clone(),
        });
        cx.emit(AgentSteeringEvent::Queued { id, text });
        cx.notify();
    }

    /// Interrupt the currently active step right away with `raw` (`⌘↵`).
    pub fn interrupt_with_text(&mut self, raw: impl Into<String>, cx: &mut Context<Self>) {
        let trimmed = raw.into().trim().to_owned();
        if trimmed.is_empty() || self.status == SteerRunStatus::Done {
            return;
        }
        let id = SharedString::from(format!("steer-{}", self.next_seq));
        self.next_seq += 1;
        let text = SharedString::from(trimmed);
        self.apply_interrupt_note(id, text, false, cx);
    }

    /// Promote a queued note (`queued_id`) into an immediate interrupt (`"Now"` button).
    pub fn interrupt_from_queue(&mut self, queued_id: &SharedString, cx: &mut Context<Self>) {
        let Some(pos) = self.queue.iter().position(|q| &q.id == queued_id) else {
            return;
        };
        let removed = self.queue.remove(pos);
        self.apply_interrupt_note(removed.id, removed.text, true, cx);
    }

    fn apply_interrupt_note(
        &mut self,
        id: SharedString,
        text: SharedString,
        from_queue: bool,
        cx: &mut Context<Self>,
    ) {
        let target_idx = self
            .steps
            .iter()
            .position(|s| s.status == SteerStepStatus::Active)
            .or_else(|| {
                self.steps
                    .iter()
                    .position(|s| s.status == SteerStepStatus::Pending)
            });
        if let Some(idx) = target_idx {
            self.steps[idx].status = SteerStepStatus::Active;
            self.steps[idx].notes.push(SteerStepNote {
                id: id.clone(),
                text: text.clone(),
                picked: SteerPickedMode::InterruptNow,
                landed_at: Some(Instant::now()),
            });
            // Reset step progress budget slightly as the agent restarts/adapts the current step
            self.active_step_accum = (self.active_step_accum * 0.35).min(1.5);
        }
        if self.status == SteerRunStatus::Paused {
            self.status = SteerRunStatus::Running;
            self.last_frame = Some(Instant::now());
        }
        self.toast = Some((
            tr(
                cx,
                "Interrupted current step and applied instruction",
                "已中断当前步骤并立即注入新指令",
            )
            .into(),
            Instant::now(),
        ));
        cx.emit(AgentSteeringEvent::Interrupted {
            id,
            text,
            from_queue,
        });
        cx.notify();
    }

    /// Remove a queued note by `id`.
    pub fn remove_queued(&mut self, id: &SharedString, cx: &mut Context<Self>) {
        let before = self.queue.len();
        self.queue.retain(|n| &n.id != id);
        if self.queue.len() != before {
            cx.emit(AgentSteeringEvent::Removed(id.clone()));
            cx.notify();
        }
    }

    /// Advance the active step to the next pending step, smoothly absorbing any queued notes.
    pub fn advance_step(&mut self, cx: &mut Context<Self>) {
        let Some(cur_idx) = self
            .steps
            .iter()
            .position(|s| s.status == SteerStepStatus::Active)
        else {
            return;
        };
        let total_step_secs = self.steps[cur_idx].duration_secs + self.active_step_accum;
        self.steps[cur_idx].duration_secs = total_step_secs.max(1.2);
        self.steps[cur_idx].status = SteerStepStatus::Done;
        let mut done_clk = PhaseClock::default();
        done_clk.restart(cx);
        self.step_done_clocks
            .insert(self.steps[cur_idx].id.clone(), done_clk);
        self.active_step_accum = 0.0;

        if let Some(next_idx) = self
            .steps
            .iter()
            .position(|s| s.status == SteerStepStatus::Pending)
        {
            let now = Instant::now();
            let drained: Vec<SteerStepNote> = self
                .queue
                .drain(..)
                .map(|q| SteerStepNote {
                    id: q.id,
                    text: q.text,
                    picked: SteerPickedMode::NextStep,
                    landed_at: Some(now),
                })
                .collect();
            let picked_count = drained.len();
            self.steps[next_idx].status = SteerStepStatus::Active;
            self.steps[next_idx].duration_secs = 0.1;
            self.steps[next_idx].notes.extend(drained);
            cx.emit(AgentSteeringEvent::StepAdvanced {
                step_index: next_idx,
                picked_count,
            });
        } else {
            self.status = SteerRunStatus::Done;
            self.last_frame = None;
            cx.emit(AgentSteeringEvent::StatusChanged(SteerRunStatus::Done));
        }
        cx.notify();
    }

    /// Toggle between `Running` and `Paused` (or restart when `Done`).
    pub fn toggle_pause_or_replay(&mut self, cx: &mut Context<Self>) {
        match self.status {
            SteerRunStatus::Running => {
                self.status = SteerRunStatus::Paused;
                self.last_frame = None;
                cx.emit(AgentSteeringEvent::StatusChanged(SteerRunStatus::Paused));
            }
            SteerRunStatus::Paused => {
                self.status = SteerRunStatus::Running;
                self.last_frame = Some(Instant::now());
                cx.emit(AgentSteeringEvent::StatusChanged(SteerRunStatus::Running));
            }
            SteerRunStatus::Done => {
                self.steps = default_steer_steps();
                self.queue = default_steer_queue();
                self.step_done_clocks.clear();
                self.queue_slides.clear();
                self.active_step_accum = 0.0;
                self.run_accum = 0.0;
                self.status = SteerRunStatus::Running;
                self.last_frame = Some(Instant::now());
                cx.emit(AgentSteeringEvent::StatusChanged(SteerRunStatus::Running));
            }
        }
        cx.notify();
    }

    fn handle_key_down(&mut self, ev: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        let cmd = ev.keystroke.modifiers.platform || ev.keystroke.modifiers.control;
        if cmd && ev.keystroke.key == "enter" {
            if let Some(ed) = self.editor.clone() {
                let draft = ed.read(cx).value().trim().to_owned();
                if !draft.is_empty() {
                    self.interrupt_with_text(draft, cx);
                    ed.update(cx, |e, cx| e.set_value("", w, cx));
                    return;
                }
            }
            if let Some(first_q) = self.queue.first().map(|q| q.id.clone()) {
                self.interrupt_from_queue(&first_q, cx);
            }
        }
    }

    fn render_kbd(&self, text: &str, p: &ProPalette) -> Div {
        div()
            .min_w(px(16.))
            .h(px(16.))
            .px(px(4.))
            .rounded(px(4.))
            .bg(p.card)
            .border_1()
            .border_color(p.line_strong)
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(10.))
            .text_color(p.text)
            .child(text.to_owned())
    }
}

fn format_elapsed_clock(total_secs: f32) -> String {
    let s = total_secs.max(0.0).floor() as u32;
    let mins = s / 60;
    let rem = s % 60;
    if mins > 0 {
        format!("{mins}m {rem:02}s")
    } else {
        format!("{rem}s")
    }
}

impl Render for AgentSteering {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let reduced = disabled_motion(cx);
        let width = self.measured_width.get();
        let narrow = width < 400.0;
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
                        .max_w(px(460.))
                        .rounded(px(10.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .p(px(14.))
                        .flex()
                        .flex_col()
                        .gap(px(12.))
                        .child(pro_skeleton_bar(210., 16., 5., 0.15, cx))
                        .child(pro_skeleton_bar(360., 120., 7., 0.35, cx))
                        .child(pro_skeleton_bar(380., 38., 7., 0.6, cx)),
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
                    tr(cx, "No active plan to steer", "当前无正在运行的计划"),
                    tr(
                        cx,
                        "Start an agent run to watch its steps and send mid-run steering notes.",
                        "启动智能体任务后即可实时查看步骤并发送中途引导指令。",
                    ),
                    None,
                    cx,
                ));
        }

        self.ensure_editor(w, cx);

        // Advance live clock while Running
        let now = Instant::now();
        if self.status == SteerRunStatus::Running {
            if let Some(prev) = self.last_frame {
                let dt = now.duration_since(prev).as_secs_f32().min(0.12);
                self.run_accum += dt;
                if self.auto_tick {
                    self.active_step_accum += dt;
                }
            }
            self.last_frame = Some(now);
            if !reduced || self.auto_tick {
                w.request_animation_frame();
            }
            if self.auto_tick && self.active_step_accum >= self.step_budget_secs {
                self.advance_step(cx);
            }
        } else {
            self.last_frame = None;
        }

        // Keep requesting animation frames while any note is gliding/glowing (< 1.45s)
        let any_note_animating = !reduced
            && self.steps.iter().any(|st| {
                st.notes.iter().any(|n| {
                    n.landed_at
                        .map(|t| t.elapsed().as_secs_f32() < 1.45)
                        .unwrap_or(false)
                })
            });
        if any_note_animating {
            w.request_animation_frame();
        }

        let total_steps = self.steps.len().max(1);
        let current_step_num = self
            .steps
            .iter()
            .position(|s| s.status == SteerStepStatus::Active)
            .map(|i| i + 1)
            .unwrap_or(total_steps);

        let (spark_col, state_label, pulse_col) = match self.status {
            SteerRunStatus::Running => (
                p.blue,
                format!(
                    "{} · {}",
                    tr(cx, "Running", "运行中"),
                    format_elapsed_clock(self.base_run_secs + self.run_accum)
                ),
                p.blue,
            ),
            SteerRunStatus::Paused => (
                p.amber,
                format!(
                    "{} · {}",
                    tr(cx, "Paused", "已暂停"),
                    format_elapsed_clock(self.base_run_secs + self.run_accum)
                ),
                p.amber,
            ),
            SteerRunStatus::Done => (
                p.green_ink,
                format!(
                    "{} · {}",
                    tr(cx, "Done", "已完成"),
                    format_elapsed_clock(self.base_run_secs + self.run_accum)
                ),
                p.green,
            ),
        };

        let pulse_halo = if self.status == SteerRunStatus::Running && !reduced {
            let cyc = ((self.run_accum / 1.6).fract()).clamp(0.0, 1.0);
            0.36 * (1.0 - cyc)
        } else {
            0.0
        };

        let (ctrl_icon, ctrl_aria) = match self.status {
            SteerRunStatus::Running => (ProIcon::Pause, tr(cx, "Pause the run", "暂停运行")),
            SteerRunStatus::Paused => (ProIcon::Play, tr(cx, "Resume the run", "继续运行")),
            SteerRunStatus::Done => (ProIcon::Refresh, tr(cx, "Replay run", "重新演示运行")),
        };

        // Header
        let header = div()
            .w_full()
            .px(px(14.))
            .py(px(12.))
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(10.))
            .child(pro_svg("0 0 14 14", SPARK_FOUR_SVG, 15., spark_col))
            .child(
                div()
                    .flex_1()
                    .min_w(px(130.))
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .truncate()
                            .child(self.title.clone()),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .text_size(px(12.))
                            .text_color(p.muted)
                            .child(self.agent.clone())
                            .child(div().text_color(p.faint).child("·"))
                            .child(pro_swap_label(
                                pro_eid(&self.id, "steer-step-cnt"),
                                format!(
                                    "{} {current_step_num} {} {total_steps}",
                                    tr(cx, "Step", "第"),
                                    tr(cx, "of", "/")
                                ),
                                12.0,
                                FontWeight::NORMAL,
                                p.muted,
                            )),
                    ),
            )
            .child(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(7.))
                    .text_size(px(12.))
                    .text_color(p.text)
                    .child(
                        div()
                            .relative()
                            .size(px(12.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .when(pulse_halo > 0.01, |d| {
                                d.child(
                                    div()
                                        .absolute()
                                        .size(px(12.))
                                        .rounded_full()
                                        .bg(pulse_col.opacity(pulse_halo)),
                                )
                            })
                            .child(div().size(px(6.)).rounded_full().bg(pulse_col)),
                    )
                    .child(state_label),
            )
            .child(pro_icon_btn(
                pro_eid(&self.id, "pause-resume-btn"),
                ctrl_aria,
                ctrl_icon,
                28.0,
                self.status == SteerRunStatus::Paused,
                cx,
                |this, _, cx| this.toggle_pause_or_replay(cx),
            ));

        // Ordered plan list with vertical rail, progressive `pro_check_draw`, spinning active arc, and landed notes
        let spin_angle = if self.status == SteerRunStatus::Running && !reduced {
            (self.run_accum * 4.2) % std::f32::consts::TAU
        } else {
            -std::f32::consts::FRAC_PI_2
        };

        let mut plan_col = div()
            .w_full()
            .px(px(14.))
            .py(px(10.))
            .border_t_1()
            .border_color(p.line)
            .flex()
            .flex_col();

        let step_len = self.steps.len();
        for (idx, step) in self.steps.iter().enumerate() {
            let is_last = idx + 1 == step_len;
            let step_icon: AnyElement = match step.status {
                SteerStepStatus::Done => {
                    let done_age = self
                        .step_done_clocks
                        .get(&step.id)
                        .map_or(f32::INFINITY, |clk| clk.time(0.28, w, cx));
                    let chk_on = reduced || done_age > 0.012;
                    pro_check_draw(
                        pro_eid(&self.id, format!("step-chk-{}", step.id)),
                        chk_on,
                        15.0,
                        false,
                        p.blue,
                    )
                }
                SteerStepStatus::Pending => pro_svg("0 0 16 16", PENDING_DASH_SVG, 15., p.faint),
                SteerStepStatus::Active => {
                    let ring_col = if self.status == SteerRunStatus::Paused {
                        p.amber
                    } else {
                        p.blue
                    };
                    let track_col = ring_col.opacity(0.22);
                    let ang = spin_angle;
                    canvas(
                        |_, _, _| (),
                        move |bounds, _, window, _| {
                            let c = bounds.center();
                            let r = 5.6;
                            let mut ring = PathBuilder::stroke(px(1.6));
                            for i in 0..=24 {
                                let a = (i as f32 / 24.0) * std::f32::consts::TAU;
                                let pt = c + point(px(r * a.cos()), px(r * a.sin()));
                                if i == 0 {
                                    ring.move_to(pt);
                                } else {
                                    ring.line_to(pt);
                                }
                            }
                            if let Ok(path) = ring.build() {
                                window.paint_path(path, track_col);
                            }
                            let mut arc = PathBuilder::stroke(px(1.6));
                            for i in 0..=10 {
                                let a = ang + (i as f32 / 10.0) * 1.85;
                                let pt = c + point(px(r * a.cos()), px(r * a.sin()));
                                if i == 0 {
                                    arc.move_to(pt);
                                } else {
                                    arc.line_to(pt);
                                }
                            }
                            if let Ok(path) = arc.build() {
                                window.paint_path(path, ring_col);
                            }
                        },
                    )
                    .size(px(16.))
                    .flex_none()
                    .into_any_element()
                }
            };

            let title_el: AnyElement = if step.status == SteerStepStatus::Active
                && self.status == SteerRunStatus::Running
            {
                pro_shimmer_text(
                    pro_eid(&self.id, format!("step-shim-{idx}")),
                    step.title.clone(),
                    p.ink,
                    p.faint,
                    13.,
                    FontWeight::MEDIUM,
                    true,
                )
            } else {
                let fg = match step.status {
                    SteerStepStatus::Done => p.text,
                    SteerStepStatus::Active => p.ink,
                    SteerStepStatus::Pending => p.muted,
                };
                div()
                    .text_size(px(13.))
                    .font_weight(if step.status == SteerStepStatus::Active {
                        FontWeight::MEDIUM
                    } else {
                        FontWeight::NORMAL
                    })
                    .text_color(fg)
                    .truncate()
                    .child(step.title.clone())
                    .into_any_element()
            };

            let time_label = match step.status {
                SteerStepStatus::Done if step.duration_secs > 0.0 => {
                    Some(format!("{:.1}s", step.duration_secs))
                }
                SteerStepStatus::Active => Some(format!(
                    "{:.1}s",
                    step.duration_secs + self.active_step_accum
                )),
                _ => None,
            };

            let mut notes_stack = div().pl(px(26.)).flex().flex_col().gap(px(4.));
            for note in &step.notes {
                let (glide_dy, glow_t, alpha) = match note.landed_at {
                    Some(t0) if !reduced => {
                        let el = t0.elapsed().as_secs_f32();
                        let gt = pro_ease((el / 0.62).clamp(0.0, 1.0));
                        let glow = (1.0 - (el / 1.40).clamp(0.0, 1.0)).powi(2);
                        (38.0 * (1.0 - gt), glow, 0.35 + 0.65 * gt)
                    }
                    _ => (0.0, 0.0, 1.0),
                };
                let bg_col = if glow_t > 0.02 {
                    p.accent_soft
                        .opacity((0.25 + 0.75 * glow_t).clamp(0.0, 1.0))
                } else {
                    p.subtle
                };
                let bdr_col = if glow_t > 0.02 {
                    p.blue.opacity(0.45 * glow_t + 0.12)
                } else {
                    p.line
                };
                let tag_text = match note.picked {
                    SteerPickedMode::NextStep => tr(cx, "Picked up", "已采纳"),
                    SteerPickedMode::InterruptNow => tr(cx, "Applied now", "已立即执行"),
                };
                notes_stack = notes_stack.child(
                    div()
                        .relative()
                        .top(px(glide_dy))
                        .opacity(alpha)
                        .mt(px(4.))
                        .px(px(8.))
                        .py(px(5.))
                        .rounded(px(6.))
                        .bg(bg_col)
                        .border_1()
                        .border_color(bdr_col)
                        .flex()
                        .items_start()
                        .gap(px(6.))
                        .text_size(px(12.))
                        .line_height(px(17.))
                        .text_color(p.ink)
                        .child(div().mt(px(2.)).child(
                            if note.picked == SteerPickedMode::InterruptNow {
                                pro_svg("0 0 12 12", BOLT_MINI_SVG, 12., p.blue)
                            } else {
                                pro_svg("0 0 14 14", CORNER_NOTE_SVG, 12., p.muted)
                            },
                        ))
                        .child(div().flex_1().min_w_0().child(note.text.clone()))
                        .child(
                            div()
                                .flex_none()
                                .text_size(px(11.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.blue)
                                .child(tag_text),
                        ),
                );
            }

            plan_col = plan_col.child(
                div()
                    .relative()
                    .py(px(5.))
                    .flex()
                    .flex_col()
                    .when(!is_last, |d| {
                        d.child(
                            div()
                                .absolute()
                                .left(px(7.5))
                                .top(px(24.))
                                .bottom(px(-4.))
                                .w(px(1.))
                                .bg(p.line_strong),
                        )
                    })
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(10.))
                            .min_w_0()
                            .child(
                                div()
                                    .size(px(16.))
                                    .flex_none()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(step_icon),
                            )
                            .child(div().flex_1().min_w_0().child(title_el))
                            .when_some(time_label, |d, tl| {
                                d.child(
                                    div()
                                        .flex_none()
                                        .text_size(px(12.))
                                        .text_color(p.faint)
                                        .child(tl),
                                )
                            }),
                    )
                    .when(!step.notes.is_empty(), |d| d.child(notes_stack)),
            );
        }

        // Queued for the next step section (smooth height fold + per-note `Slide` settle + tactile controls)
        let queue_snapshot = self.queue.clone();
        let queue_open = !queue_snapshot.is_empty();
        let chips: Vec<AnyElement> = queue_snapshot
            .iter()
            .map(|q| {
                let q_id_now = q.id.clone();
                let q_id_rm = q.id.clone();
                let now_lbl = tr(cx, "Now", "立即");
                let dy = if reduced {
                    0.0
                } else {
                    self.queue_slides.get(&q.id).map_or(0.0, |sl| sl.get(w, cx))
                };
                div()
                    .relative()
                    .top(px(dy))
                    .min_h(px(34.))
                    .pl(px(12.))
                    .pr(px(4.))
                    .py(px(4.))
                    .rounded(px(8.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line_strong)
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(
                        div()
                            .absolute()
                            .left(px(0.))
                            .top(px(7.))
                            .bottom(px(7.))
                            .w(px(2.5))
                            .rounded(px(2.))
                            .bg(p.blue),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_size(px(12.5))
                            .line_height(px(18.))
                            .text_color(p.ink)
                            .child(q.text.clone()),
                    )
                    .child(pro_btn(
                        pro_eid(&self.id, format!("q-now-{}", q.id)),
                        now_lbl,
                        Some(ProIcon::Bolt),
                        ProBtnStyle::Ghost,
                        true,
                        cx,
                        move |this, _, cx| {
                            this.interrupt_from_queue(&q_id_now, cx);
                        },
                    ))
                    .child(pro_icon_btn(
                        pro_eid(&self.id, format!("q-rm-{}", q.id)),
                        tr(cx, "Remove queued note", "移除排队指令"),
                        ProIcon::Close,
                        24.0,
                        false,
                        cx,
                        move |this, _, cx| {
                            this.remove_queued(&q_id_rm, cx);
                        },
                    ))
                    .into_any_element()
            })
            .collect();

        let q_count = queue_snapshot.len();
        let q_count_badge = div()
            .min_w(px(16.))
            .h(px(16.))
            .px(px(4.))
            .rounded(px(4.))
            .bg(p.chip)
            .flex()
            .items_center()
            .justify_center()
            .child(pro_rolling_number(
                pro_eid(&self.id, "steer-q-cnt"),
                q_count as f64,
                0,
                "",
                "",
                10.0,
                FontWeight::MEDIUM,
                p.text,
            ));

        let queue_fold = pro_fold(pro_eid(&self.id, "queue-fold"), queue_open, 76., 0., {
            let base_id = self.id.clone();
            let header_lbl = tr(cx, "QUEUED FOR THE NEXT STEP", "等待下一步采纳").to_owned();
            let finish_lbl = tr(cx, "Finish step ▸", "完成当前步 ▸");
            let can_advance = self.status != SteerRunStatus::Done;
            let adv_pill = can_advance.then(|| {
                div().ml_auto().child(pro_btn(
                    pro_eid(&base_id, "adv-step-btn"),
                    finish_lbl,
                    None,
                    ProBtnStyle::Ghost,
                    true,
                    cx,
                    |this, _, cx| this.advance_step(cx),
                ))
            });

            move || {
                div()
                    .w_full()
                    .px(px(14.))
                    .pt(px(10.))
                    .pb(px(4.))
                    .border_t_1()
                    .border_color(p.line)
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .child(
                                div()
                                    .text_size(px(10.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.muted)
                                    .child(header_lbl),
                            )
                            .child(q_count_badge)
                            .when_some(adv_pill, |d, btn| d.child(btn)),
                    )
                    .children(chips)
                    .into_any_element()
            }
        });

        // Steering Composer (folds closed when status == Done) with Chat/Act `pro_pill_indicator` + policy `pro_popover_presence`
        let has_draft = self
            .editor
            .as_ref()
            .map(|e| !e.read(cx).value().trim().is_empty())
            .unwrap_or(false);
        let composer_open = self.status != SteerRunStatus::Done;
        let mode_sensor = pro_selection_sensor();
        let act_mode = self.act_mode;
        let mode_key = if act_mode { "act" } else { "chat" };
        let mode_toggle = div()
            .relative()
            .p(px(2.0))
            .rounded(px(6.0))
            .bg(p.subtle)
            .border_1()
            .border_color(p.line)
            .flex()
            .items_center()
            .child(pro_pill_indicator(
                pro_eid(&self.id, "steer-mode-pill"),
                mode_key,
                mode_sensor.clone(),
                4.5,
                p.card,
                Some(p.line),
                true,
            ))
            .children(
                [
                    (false, "chat", tr(cx, "Chat", "对话")),
                    (true, "act", tr(cx, "Act", "执行")),
                ]
                .into_iter()
                .map(|(is_act, key, lbl)| {
                    let sel = act_mode == is_act;
                    div()
                        .id(pro_eid(&self.id, format!("steer-mode-{key}")))
                        .role(Role::RadioButton)
                        .aria_toggled(sel.into())
                        .tab_index(if sel { 0 } else { -1 })
                        .relative()
                        .h(px(20.0))
                        .px(px(7.0))
                        .rounded(px(4.5))
                        .flex()
                        .items_center()
                        .cursor_pointer()
                        .text_size(px(11.0))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(if sel { p.ink } else { p.muted })
                        .hover(move |s| if sel { s } else { s.text_color(p.ink) })
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.act_mode = is_act;
                            if !is_act {
                                this.policy_open = false;
                            }
                            cx.notify();
                        }))
                        .when(sel, |d| d.child(pro_mark_selected(&mode_sensor)))
                        .child(lbl)
                }),
            );

        let policies: [(&'static str, ProIcon); 3] = [
            (tr(cx, "Ask on risk", "风险询问"), ProIcon::Shield),
            (tr(cx, "Auto-edit", "自动编辑"), ProIcon::Sparkles),
            (tr(cx, "Full auto", "全自动"), ProIcon::Bolt),
        ];
        let pol_idx = self.policy_idx.min(2);
        let (pol_lbl, pol_icon) = policies[pol_idx];
        let pol_open = self.policy_open && act_mode;
        let mut pol_menu = div()
            .w(px(168.0))
            .p(px(4.0))
            .rounded(px(9.0))
            .bg(p.card)
            .border_1()
            .border_color(p.line_strong)
            .shadow_lg()
            .flex()
            .flex_col()
            .gap(px(2.0));
        for (pi, (plbl, pic)) in policies.into_iter().enumerate() {
            let sel = pi == pol_idx;
            let row_in = div()
                .w_full()
                .h(px(26.0))
                .px(px(7.0))
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(pro_icon(pic, 12.5, if sel { p.ink } else { p.muted }))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_size(px(11.5))
                        .font_weight(if sel {
                            FontWeight::MEDIUM
                        } else {
                            FontWeight::NORMAL
                        })
                        .text_color(if sel { p.strong } else { p.text })
                        .truncate()
                        .child(plbl),
                )
                .child(pro_check_draw(
                    pro_eid(&self.id, format!("steer-pol-chk-{pi}")),
                    sel,
                    13.5,
                    false,
                    p.primary,
                ));
            pol_menu = pol_menu.child(
                pro_tactile(
                    pro_eid(&self.id, format!("steer-pol-opt-{pi}")),
                    plbl,
                    5.5,
                    transparent_black(),
                    p.hover,
                    transparent_black(),
                    transparent_black(),
                    sel,
                    row_in,
                    cx,
                    move |this, _, cx| {
                        this.policy_idx = pi;
                        this.policy_open = false;
                        cx.notify();
                    },
                )
                .w_full(),
            );
        }

        let policy_ctl = div()
            .relative()
            .child(
                div()
                    .id(pro_eid(&self.id, "steer-policy-btn"))
                    .role(Role::Button)
                    .aria_expanded(pol_open)
                    .tab_index(0)
                    .h(px(24.0))
                    .px(px(6.0))
                    .rounded(px(5.5))
                    .opacity(if act_mode { 1.0 } else { 0.42 })
                    .when(act_mode, |d| {
                        d.cursor_pointer()
                            .hover(|s| s.bg(p.hover).text_color(p.ink))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.policy_open = !this.policy_open;
                                cx.notify();
                            }))
                    })
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .text_color(p.muted)
                    .child(pro_icon(pol_icon, 12.5, p.muted))
                    .when(!narrow, |d| {
                        d.child(pro_swap_label(
                            pro_eid(&self.id, "steer-pol-lbl"),
                            pol_lbl,
                            11.5,
                            FontWeight::NORMAL,
                            p.muted,
                        ))
                    })
                    .child(pro_icon(ProIcon::ChevronDown, 10.5, p.muted)),
            )
            .child(
                div()
                    .absolute()
                    .bottom(px(28.0))
                    .right_0()
                    .child(pro_popover_presence(
                        pro_eid(&self.id, "steer-policy-pop"),
                        pol_open,
                        move || pol_menu.into_any_element(),
                    )),
            );

        let composer_fold = pro_fold(pro_eid(&self.id, "compose-fold"), composer_open, 88., 0., {
            let ed_opt = self.editor.clone();
            let focused = self.editor_focused;
            let base_id = self.id.clone();
            let next_hint = tr(cx, "next step", "下一步采纳").to_owned();
            let now_hint = tr(cx, "interrupt now", "立即中断执行").to_owned();
            let now_btn_lbl = if narrow {
                "⚡"
            } else {
                tr(cx, "Interrupt now", "立即中断")
            };
            let kbd_enter = self.render_kbd("↵", &p);
            let kbd_cmd = self.render_kbd("⌘", &p);
            let kbd_enter2 = self.render_kbd("↵", &p);

            let interrupt_inline_btn = has_draft.then(|| {
                let ed_for_now = ed_opt.clone();
                pro_btn(
                    pro_eid(&base_id, "compose-now-btn"),
                    now_btn_lbl,
                    Some(ProIcon::Bolt),
                    ProBtnStyle::Secondary,
                    true,
                    cx,
                    move |this, w, cx| {
                        if let Some(ed) = &ed_for_now {
                            let v = ed.read(cx).value().trim().to_owned();
                            if !v.is_empty() {
                                this.interrupt_with_text(v, cx);
                                ed.update(cx, |e, cx| e.set_value("", w, cx));
                            }
                        }
                    },
                )
            });

            let ed_for_send = ed_opt.clone();
            let send_btn = div()
                .opacity(if has_draft { 1.0 } else { 0.35 })
                .child(pro_icon_btn(
                    pro_eid(&base_id, "compose-send-btn"),
                    tr(cx, "Queue for the next step", "加入下一步队列"),
                    ProIcon::ArrowUp,
                    28.0,
                    has_draft,
                    cx,
                    move |this, w, cx| {
                        if let Some(ed) = &ed_for_send {
                            let v = ed.read(cx).value().trim().to_owned();
                            if !v.is_empty() {
                                this.queue_note(v, cx);
                                ed.update(cx, |e, cx| e.set_value("", w, cx));
                            }
                        }
                    },
                ));

            move || {
                div()
                    .w_full()
                    .px(px(14.))
                    .py(px(10.))
                    .border_t_1()
                    .border_color(p.line)
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(
                        div()
                            .w_full()
                            .min_h(px(38.))
                            .pl(px(10.))
                            .pr(px(5.))
                            .py(px(4.))
                            .rounded(px(8.))
                            .bg(if focused { p.card } else { p.subtle })
                            .border_1()
                            .border_color(if focused { p.line_strong } else { p.line })
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .when_some(ed_opt, |d, ed| {
                                d.child(div().flex_1().min_w_0().h(px(22.)).child(ed))
                            })
                            .when_some(interrupt_inline_btn, |d, b| d.child(b))
                            .child(send_btn),
                    )
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .justify_between()
                            .gap_x(px(10.))
                            .gap_y(px(4.))
                            .text_size(px(12.))
                            .text_color(p.muted)
                            .child(
                                div()
                                    .flex()
                                    .flex_wrap()
                                    .items_center()
                                    .gap_x(px(12.))
                                    .gap_y(px(4.))
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(4.))
                                            .child(kbd_enter)
                                            .child(next_hint),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(3.))
                                            .child(kbd_cmd)
                                            .child(kbd_enter2)
                                            .child(div().ml(px(1.)).child(now_hint)),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(5.))
                                    .child(mode_toggle)
                                    .child(policy_ctl),
                            ),
                    )
                    .into_any_element()
            }
        });

        let card = div()
            .w_full()
            .max_w(px(460.))
            .rounded(px(10.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .shadow_sm()
            .flex()
            .flex_col()
            .child(header)
            .child(plan_col)
            .child(queue_fold)
            .child(composer_fold);

        div()
            .id(self.id.clone())
            .role(Role::Region)
            .aria_label(self.title.clone())
            .track_focus(&self.focus)
            .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                if this.policy_open {
                    this.policy_open = false;
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
            .child(card)
            .child(pro_toast_banner(self.toast.clone()))
    }
}

fn default_steer_steps() -> Vec<SteerStep> {
    vec![
        SteerStep::new("s1", "Read schema and 14 migrations", SteerStepStatus::Done)
            .with_duration_secs(1.4),
        SteerStep::new(
            "s2",
            "Plan the split into accounts and profiles",
            SteerStepStatus::Active,
        )
        .with_duration_secs(2.4),
        SteerStep::new(
            "s3",
            "Write migration 0042_split_users",
            SteerStepStatus::Pending,
        ),
        SteerStep::new("s4", "Dry run on staging", SteerStepStatus::Pending),
        SteerStep::new(
            "s5",
            "Open a pull request for review",
            SteerStepStatus::Pending,
        ),
    ]
}

fn default_steer_queue() -> Vec<SteerQueuedNote> {
    vec![SteerQueuedNote::new(
        "q1",
        "Use Postgres 16 syntax – prod isn't on 17 yet",
    )]
}

#[cfg(test)]
mod tests {
    use super::{
        SteerPickedMode, SteerRunStatus, SteerStepNote, SteerStepStatus, default_steer_queue,
        default_steer_steps, format_elapsed_clock,
    };

    #[test]
    fn default_plan_starts_at_step_two_with_one_queued_note() {
        let steps = default_steer_steps();
        let queue = default_steer_queue();
        assert_eq!(steps.len(), 5);
        assert_eq!(steps[0].status, SteerStepStatus::Done);
        assert_eq!(steps[1].status, SteerStepStatus::Active);
        assert_eq!(steps[2].status, SteerStepStatus::Pending);
        assert_eq!(queue.len(), 1);
        assert!(queue[0].text.contains("Postgres 16"));
    }

    #[test]
    fn elapsed_clock_and_step_notes_preserve_pickup_mode() {
        assert_eq!(format_elapsed_clock(128.4), "2m 08s");
        assert_eq!(format_elapsed_clock(9.0), "9s");
        let note = SteerStepNote::new("n1", "Skip index drop", SteerPickedMode::InterruptNow);
        assert_eq!(note.picked, SteerPickedMode::InterruptNow);
        assert_eq!(SteerRunStatus::default(), SteerRunStatus::Running);
    }
}
