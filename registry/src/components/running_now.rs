//! Compact live header/sidebar status pill and popover for active background agent tasks.
//!
//! Summarizes background work in one quiet pill (`"3 running · 1 needs you"`) with a twinkling
//! ring and amber attention dot, expanding via [`pro_popover_presence`] into a `348px` task popover with
//! filling [`pro_progress_ring`] indicators, current step & expandable sub-steps, `Pause` /
//! `Resume`, and `Open` on tasks waiting for human input.

use crate::{
    components::pro_chrome::{
        ProBtnStyle, ProPalette, pro_btn, pro_card_shadow, pro_check_draw, pro_eid,
        pro_empty_state, pro_fold, pro_icon_btn, pro_popover_presence, pro_progress_ring,
        pro_rolling_number, pro_skeleton_bar, pro_spring_f32, pro_tactile, pro_toast_banner,
        pro_view_transition, tr,
    },
    foundation::Theme,
    primitives::{
        control::UiLanguage,
        family::{PhaseClock, disabled_motion},
        pro_icons::{ProIcon, pro_icon},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, collections::HashMap, rc::Rc};
use web_time::Instant;

/// Lifecycle state of an individual [`RunningTask`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RunningTaskState {
    /// Actively executing background steps (shows `Pause`).
    #[default]
    Running,
    /// Blocked waiting for human approval or input (shows `Needs you` badge + `Open`).
    NeedsYou,
    /// Paused by the user (shows `Resume`).
    Paused,
    /// Completed all steps (shows a full blue ring + check).
    Done,
}

/// Horizontal alignment of the [`RunningNow`] popover relative to its trigger pill.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RunningNowAlign {
    /// Align popover's leading edge with the trigger (suited for sidebars).
    Start,
    /// Align popover's trailing edge with the trigger (suited for app headers).
    #[default]
    End,
}

/// Background agent task entry displayed inside [`RunningNow`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunningTask {
    /// Stable task identifier (rows keep slice order and never jump positions).
    pub id: SharedString,
    /// Task name (for example `"Ingest pipeline"`).
    pub title: SharedString,
    /// Present-tense description of the current step (for example `"Embedding chunks"`).
    pub step: SharedString,
    /// Ordered sub-steps for this task (inspected when expanding a row and stepped in live mode).
    pub sub_steps: Vec<SharedString>,
    /// Number of completed steps (`0..=total`).
    pub done: usize,
    /// Total number of steps in the task.
    pub total: usize,
    /// Task state (`Running`, `NeedsYou`, `Paused`, `Done`).
    pub state: RunningTaskState,
}

impl RunningTask {
    /// Create a [`RunningTask`] with `id`, `title`, `step`, `done`, `total`, and `state`.
    pub fn new(
        id: impl Into<SharedString>,
        title: impl Into<SharedString>,
        step: impl Into<SharedString>,
        done: usize,
        total: usize,
        state: RunningTaskState,
    ) -> Self {
        let total = total.max(1);
        Self {
            id: id.into(),
            title: title.into(),
            step: step.into(),
            sub_steps: Vec::new(),
            done: done.min(total),
            total,
            state,
        }
    }

    /// Attach ordered sub-steps for inspection and live step progression.
    pub fn with_sub_steps(mut self, sub_steps: Vec<SharedString>) -> Self {
        self.sub_steps = sub_steps;
        self
    }

    /// Completion fraction in `0.0..=1.0`.
    pub fn progress(&self) -> f32 {
        if self.state == RunningTaskState::Done {
            return 1.0;
        }
        if self.total == 0 {
            return 0.0;
        }
        (self.done as f32 / self.total as f32).clamp(0.0, 1.0)
    }
}

/// Typed events emitted by [`RunningNow`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RunningNowEvent {
    /// Emitted when the popover opens or closes.
    OpenChanged(bool),
    /// Emitted when a running task is paused (`Pause`).
    Paused(SharedString),
    /// Emitted when a paused task is resumed (`Resume`).
    Resumed(SharedString),
    /// Emitted when a row or `Open` button is clicked.
    OpenedTask(SharedString),
    /// Emitted when the `"Open Tasks"` footer button is clicked.
    OpenedAll,
}

impl EventEmitter<RunningNowEvent> for RunningNow {}

/// Live header/sidebar pill and background-task popover.
///
/// # Examples
///
/// ```no_run
/// # #[allow(unused_variables, unused_imports)]
/// # fn example(window: &mut gpui::Window, cx: &mut gpui::App) {
/// # use gpui::prelude::*;
/// use gpui::prelude::*;
/// use kui::components::running_now::{RunningNow, RunningNowAlign, RunningNowEvent};
///
/// let indicator = cx.new(|cx| {
///     RunningNow::new("header-running-now", cx)
///         .with_align(RunningNowAlign::End)
///         .with_open(true)
/// });
/// cx.subscribe(&indicator, |_, event: &RunningNowEvent, _| {
///     let _ = event;
/// }).detach();
/// # }
/// ```
pub struct RunningNow {
    id: ElementId,
    focus: FocusHandle,
    tasks: Vec<RunningTask>,
    open: bool,
    align: RunningNowAlign,
    header_chrome: bool,
    workspace_name: SharedString,
    page_name: SharedString,
    expanded_task_id: Option<SharedString>,
    highlighted_row: Option<usize>,
    live_tick: bool,
    started_at: Instant,
    last_tick: Instant,
    done_clocks: HashMap<SharedString, PhaseClock>,
    loading: bool,
    empty: bool,
    measured_width: Rc<Cell<f32>>,
    toast: Option<(SharedString, Instant)>,
}

impl RunningNow {
    /// Create [`RunningNow`] preloaded with 5 active background tasks (`3 running · 1 needs you`).
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<UiLanguage>(|_, cx| cx.notify())
            .detach();
        let now = Instant::now();
        Self {
            id: id.into(),
            focus: cx.focus_handle(),
            tasks: default_running_tasks(),
            open: true,
            align: RunningNowAlign::End,
            header_chrome: true,
            workspace_name: "Kense".into(),
            page_name: "Agents".into(),
            expanded_task_id: Some("ingest".into()),
            highlighted_row: None,
            live_tick: true,
            started_at: now,
            last_tick: now,
            done_clocks: HashMap::new(),
            loading: false,
            empty: false,
            measured_width: Rc::new(Cell::new(720.0)),
            toast: None,
        }
    }

    /// Replace the ordered background tasks slice.
    pub fn with_tasks(mut self, tasks: Vec<RunningTask>) -> Self {
        self.tasks = tasks;
        self
    }

    /// Set whether the task popover starts open (`true` by default for immediate inspection).
    pub fn with_open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// Set popover alignment (`Start` for sidebars, `End` for right-aligned headers).
    pub fn with_align(mut self, align: RunningNowAlign) -> Self {
        self.align = align;
        self
    }

    /// Show (`true`, default) or omit (`false`) the surrounding demo header bar.
    pub fn with_header_chrome(mut self, chrome: bool) -> Self {
        self.header_chrome = chrome;
        self
    }

    /// Enable or disable gentle live background step progression (`true` default).
    pub fn with_live_tick(mut self, live_tick: bool) -> Self {
        self.live_tick = live_tick;
        self
    }

    /// Toggle skeleton loading state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Toggle empty state (`"No tasks running"`).
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Return whether the task popover is currently open.
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Return `(running_count, needs_you_count, paused_count)` across `tasks`.
    pub fn summary_counts(&self) -> (usize, usize, usize) {
        let running = self
            .tasks
            .iter()
            .filter(|t| t.state == RunningTaskState::Running)
            .count();
        let needs_you = self
            .tasks
            .iter()
            .filter(|t| t.state == RunningTaskState::NeedsYou)
            .count();
        let paused = self
            .tasks
            .iter()
            .filter(|t| t.state == RunningTaskState::Paused)
            .count();
        (running, needs_you, paused)
    }

    /// Open or close the popover and emit [`RunningNowEvent::OpenChanged`].
    pub fn set_open(&mut self, next: bool, cx: &mut Context<Self>) {
        if self.open != next {
            self.open = next;
            cx.emit(RunningNowEvent::OpenChanged(next));
            cx.notify();
        }
    }

    /// Pause a running task by `id`.
    pub fn pause_task(&mut self, id: &SharedString, cx: &mut Context<Self>) {
        if let Some(t) = self.tasks.iter_mut().find(|t| &t.id == id)
            && t.state == RunningTaskState::Running
        {
            t.state = RunningTaskState::Paused;
            cx.emit(RunningNowEvent::Paused(id.clone()));
            cx.notify();
        }
    }

    /// Resume a paused task by `id`.
    pub fn resume_task(&mut self, id: &SharedString, cx: &mut Context<Self>) {
        if let Some(t) = self.tasks.iter_mut().find(|t| &t.id == id)
            && t.state == RunningTaskState::Paused
        {
            t.state = RunningTaskState::Running;
            cx.emit(RunningNowEvent::Resumed(id.clone()));
            cx.notify();
        }
    }

    /// Open a task (resolving `NeedsYou` into `Running` when opened, or toggling sub-step inspection).
    pub fn open_task(&mut self, id: &SharedString, cx: &mut Context<Self>) {
        let mut resolved_blocker = false;
        if let Some(t) = self.tasks.iter_mut().find(|t| &t.id == id)
            && t.state == RunningTaskState::NeedsYou
        {
            t.state = RunningTaskState::Running;
            t.done = (t.done + 1).min(t.total);
            if let Some(next_sub) = t.sub_steps.get(t.done) {
                t.step = next_sub.clone();
            }
            resolved_blocker = true;
        }
        self.expanded_task_id = if self.expanded_task_id.as_ref() == Some(id) && !resolved_blocker {
            None
        } else {
            Some(id.clone())
        };
        if resolved_blocker {
            self.toast = Some((
                tr(
                    cx,
                    "Opened blocker & resumed task",
                    "已打开待办项并恢复任务执行",
                )
                .into(),
                Instant::now(),
            ));
        }
        cx.emit(RunningNowEvent::OpenedTask(id.clone()));
        cx.notify();
    }

    fn advance_one_live_step(&mut self, cx: &mut Context<Self>) {
        // Advance the first running task smoothly through its sub_steps, wrapping cleanly
        let running_indices: Vec<usize> = self
            .tasks
            .iter()
            .enumerate()
            .filter(|(_, t)| t.state == RunningTaskState::Running)
            .map(|(i, _)| i)
            .collect();
        if running_indices.is_empty() {
            return;
        }
        let slot = (self.started_at.elapsed().as_secs() as usize / 4) % running_indices.len();
        let idx = running_indices[slot];
        let task = &mut self.tasks[idx];
        if task.done + 1 < task.total {
            task.done += 1;
            if let Some(lbl) = task.sub_steps.get(task.done) {
                task.step = lbl.clone();
            }
        } else {
            task.done = task.total;
            task.state = RunningTaskState::Done;
            task.step = "Completed".into();
            let done_id = task.id.clone();
            self.done_clocks.entry(done_id).or_default().restart(cx);
        }
    }

    fn handle_key_down(&mut self, ev: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        let key = ev.keystroke.key.as_str();
        match key {
            "escape" if self.open => {
                self.set_open(false, cx);
            }
            "down" => {
                if !self.open {
                    self.set_open(true, cx);
                    self.highlighted_row = (!self.tasks.is_empty()).then_some(0);
                } else if !self.tasks.is_empty() {
                    let next = self
                        .highlighted_row
                        .map(|i| (i + 1) % self.tasks.len())
                        .unwrap_or(0);
                    self.highlighted_row = Some(next);
                    cx.notify();
                }
            }
            "up" => {
                if !self.open {
                    self.set_open(true, cx);
                    self.highlighted_row = self.tasks.len().checked_sub(1);
                } else if !self.tasks.is_empty() {
                    let prev = self
                        .highlighted_row
                        .map(|i| if i == 0 { self.tasks.len() - 1 } else { i - 1 })
                        .unwrap_or(self.tasks.len() - 1);
                    self.highlighted_row = Some(prev);
                    cx.notify();
                }
            }
            "enter" | "space" => {
                if let Some(row_i) = self.highlighted_row
                    && self.open
                    && let Some(tid) = self.tasks.get(row_i).map(|t| t.id.clone())
                {
                    self.open_task(&tid, cx);
                } else {
                    self.set_open(!self.open, cx);
                }
            }
            _ => {}
        }
    }
}

impl Render for RunningNow {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let reduced = disabled_motion(cx);
        let width = self.measured_width.get();
        let ultra_narrow = width < 380.0;
        let measure_cell = self.measured_width.clone();

        if self.loading {
            if !reduced {
                w.request_animation_frame();
            }
            return div()
                .id(self.id.clone())
                .w_full()
                .flex()
                .flex_col()
                .items_end()
                .gap(px(8.))
                .child(pro_skeleton_bar(196., 28., 6., 0.15, cx))
                .child(pro_skeleton_bar(340., 240., 10., 0.45, cx));
        }

        // Gentle live step tick every 4.2s when live_tick == true
        let (running_cnt, _needs_cnt, _paused_cnt) = self.summary_counts();
        if running_cnt > 0 && (!reduced || self.live_tick) {
            w.request_animation_frame();
            if self.live_tick && self.last_tick.elapsed().as_secs_f32() >= 4.2 {
                self.last_tick = Instant::now();
                self.advance_one_live_step(cx);
            }
        }

        let (running_cnt, needs_cnt, paused_cnt) = self.summary_counts();
        let active_tasks = if self.empty { 0 } else { self.tasks.len() };
        let phase_s = self.started_at.elapsed().as_secs_f32();
        let twinkle = if running_cnt > 0 && !reduced {
            0.55 + 0.45 * (phase_s * 3.2).sin().abs()
        } else {
            1.0
        };
        let dot_pulse = if needs_cnt > 0 && !reduced {
            0.62 + 0.38 * (phase_s * 3.8).cos().abs()
        } else {
            1.0
        };

        // Pre-sample spring progress rings and completion check clocks for every task
        let mut ring_pcts: Vec<f32> = Vec::with_capacity(self.tasks.len());
        let mut done_draws: Vec<bool> = Vec::with_capacity(self.tasks.len());
        for task in &self.tasks {
            let sp_pct = pro_spring_f32(
                pro_eid(&self.id, format!("rn-ring-{}", task.id)),
                task.progress(),
                0.34,
                w,
                cx,
            );
            ring_pcts.push(sp_pct);
            let draw_ok = if task.state == RunningTaskState::Done {
                if reduced {
                    true
                } else if let Some(clk) = self.done_clocks.get(&task.id) {
                    clk.time(0.28, w, cx) > 0.012
                } else {
                    true
                }
            } else {
                false
            };
            done_draws.push(draw_ok);
        }

        let stage_min_h = pro_spring_f32(
            pro_eid(&self.id, "rn-stage-h"),
            if self.open { 404.0 } else { 64.0 },
            0.22,
            w,
            cx,
        );

        // Trigger pill content with KUI tactile spring physics
        let trigger_pill = {
            let aria = SharedString::from(format!(
                "{}: {running_cnt} {}, {needs_cnt} {}",
                tr(cx, "Background tasks", "后台任务"),
                tr(cx, "running", "运行中"),
                tr(cx, "need you", "待确认"),
            ));
            let mut pill = div()
                .h(px(28.))
                .pl(px(10.))
                .pr(px(8.))
                .flex()
                .items_center()
                .gap(px(7.))
                .text_size(px(12.))
                .text_color(p.text);

            if active_tasks == 0 || (running_cnt == 0 && needs_cnt == 0 && paused_cnt == 0) {
                pill = pill
                    .child(
                        div()
                            .size(px(8.))
                            .rounded_full()
                            .border_2()
                            .border_color(p.faint),
                    )
                    .child(tr(cx, "No tasks running", "暂无运行任务"));
            } else if running_cnt == 0 && needs_cnt == 0 && paused_cnt > 0 {
                pill = pill
                    .child(
                        div()
                            .size(px(8.))
                            .rounded_full()
                            .border_2()
                            .border_color(p.faint),
                    )
                    .child(pro_rolling_number(
                        pro_eid(&self.id, "trig-paused"),
                        paused_cnt as f64,
                        0,
                        "",
                        "",
                        12.,
                        FontWeight::MEDIUM,
                        p.ink,
                    ))
                    .child(tr(cx, "paused", "已暂停"));
            } else {
                if running_cnt > 0 || needs_cnt == 0 {
                    pill = pill.child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(5.))
                            .child(
                                div()
                                    .size(px(8.))
                                    .rounded_full()
                                    .border_2()
                                    .border_color(p.teal.opacity(twinkle)),
                            )
                            .child(pro_rolling_number(
                                pro_eid(&self.id, "trig-run-n"),
                                running_cnt as f64,
                                0,
                                "",
                                "",
                                12.,
                                FontWeight::MEDIUM,
                                p.ink,
                            ))
                            .when(!(ultra_narrow && needs_cnt > 0), |d| {
                                d.child(tr(cx, "running", "运行中"))
                            }),
                    );
                }
                if running_cnt > 0 && needs_cnt > 0 {
                    pill = pill.child(div().text_color(p.faint).child("·"));
                }
                if needs_cnt > 0 {
                    let needs_word = if needs_cnt == 1 {
                        tr(cx, "needs you", "待你处理")
                    } else {
                        tr(cx, "need you", "待你处理")
                    };
                    pill = pill.child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(5.))
                            .child(
                                div()
                                    .size(px(6.))
                                    .rounded_full()
                                    .bg(p.amber.opacity(dot_pulse)),
                            )
                            .child(pro_rolling_number(
                                pro_eid(&self.id, "trig-need-n"),
                                needs_cnt as f64,
                                0,
                                "",
                                "",
                                12.,
                                FontWeight::MEDIUM,
                                p.ink,
                            ))
                            .when(!(ultra_narrow && running_cnt > 0), |d| d.child(needs_word)),
                    );
                }
            }

            let pill_inner = pill.child(pro_icon(
                if self.open {
                    ProIcon::ChevronUp
                } else {
                    ProIcon::ChevronDown
                },
                12.,
                p.muted,
            ));

            pro_tactile(
                pro_eid(&self.id, "rn-trigger"),
                aria,
                6.,
                p.card,
                p.chip_hover,
                p.line_strong,
                p.line_strong,
                self.open,
                pill_inner,
                cx,
                |this, _, cx| this.set_open(!this.open, cx),
            )
            .flex_none()
            .shadow(pro_card_shadow(p.dark))
        };

        // Popover panel via pro_popover_presence so opening/closing via header pill animates both ways
        let popover = {
            let tasks_snap = if self.empty {
                Vec::new()
            } else {
                self.tasks.clone()
            };
            let exp_id = self.expanded_task_id.clone();
            let hi_row = self.highlighted_row;
            let base_id = self.id.clone();
            let pop_w = (width - 24.0).clamp(260.0, 348.0);
            let head_lbl = tr(cx, "BACKGROUND TASKS", "后台智能体任务").to_owned();
            let empty_copy = tr(
                cx,
                "No background tasks right now.",
                "当前没有后台运行中的任务。",
            )
            .to_owned();
            let open_all_lbl = tr(cx, "Open Tasks", "打开全部任务").to_owned();
            let reset_lbl = tr(cx, "Reset", "重置").to_owned();
            let any_done = tasks_snap.iter().any(|t| t.state == RunningTaskState::Done);

            let rows_els: Vec<AnyElement> = tasks_snap
                .iter()
                .enumerate()
                .map(|(r_idx, task)| {
                    let pct = ring_pcts
                        .get(r_idx)
                        .copied()
                        .unwrap_or_else(|| task.progress());
                    let done_draw = done_draws.get(r_idx).copied().unwrap_or(true);
                    let ring_color = match task.state {
                        RunningTaskState::Done => p.blue,
                        RunningTaskState::NeedsYou => p.amber,
                        RunningTaskState::Paused => p.muted,
                        RunningTaskState::Running => {
                            if pct < 0.35 {
                                p.orange
                            } else {
                                p.teal
                            }
                        }
                    };
                    let is_exp = exp_id.as_ref() == Some(&task.id);
                    let is_hi = hi_row == Some(r_idx);
                    let tid_main = task.id.clone();
                    let tid_act = task.id.clone();

                    let right_action: AnyElement = match task.state {
                        RunningTaskState::Running => div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(p.muted)
                                    .child(format!("{}/{}", task.done, task.total)),
                            )
                            .child(pro_icon_btn(
                                pro_eid(&base_id, format!("rn-pause-{}", task.id)),
                                SharedString::from(format!("Pause {}", task.title)),
                                ProIcon::Pause,
                                28.,
                                false,
                                cx,
                                move |this, _, cx| this.pause_task(&tid_act, cx),
                            ))
                            .into_any_element(),
                        RunningTaskState::Paused => div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .child(div().text_size(px(12.)).text_color(p.muted).child(tr(
                                cx,
                                "Paused",
                                "已暂停",
                            )))
                            .child(pro_icon_btn(
                                pro_eid(&base_id, format!("rn-resume-{}", task.id)),
                                SharedString::from(format!("Resume {}", task.title)),
                                ProIcon::Play,
                                28.,
                                false,
                                cx,
                                move |this, _, cx| this.resume_task(&tid_act, cx),
                            ))
                            .into_any_element(),
                        RunningTaskState::NeedsYou => div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .child(
                                div()
                                    .h(px(18.))
                                    .px(px(6.))
                                    .rounded(px(5.))
                                    .bg(p.amber_soft)
                                    .flex()
                                    .items_center()
                                    .text_size(px(11.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.amber_ink)
                                    .child(tr(cx, "Needs you", "待确认")),
                            )
                            .child(pro_btn(
                                pro_eid(&base_id, format!("rn-open-{}", task.id)),
                                tr(cx, "Open", "打开"),
                                None,
                                ProBtnStyle::Secondary,
                                true,
                                cx,
                                move |this, _, cx| this.open_task(&tid_act, cx),
                            ))
                            .into_any_element(),
                        RunningTaskState::Done => div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(p.muted)
                                    .child(format!("{}/{}", task.total, task.total)),
                            )
                            .child(
                                div()
                                    .size(px(28.))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(pro_check_draw(
                                        pro_eid(&base_id, format!("rn-done-chk-{}", task.id)),
                                        done_draw,
                                        14.,
                                        false,
                                        p.blue,
                                    )),
                            )
                            .into_any_element(),
                    };

                    // Sub-steps disclosure via pro_fold
                    let subs = task.sub_steps.clone();
                    let done_n = task.done;
                    let task_state = task.state;
                    let sub_base = base_id.clone();
                    let sub_tid = task.id.clone();
                    let sub_fold = pro_fold(
                        pro_eid(&base_id, format!("rn-sub-fold-{}", task.id)),
                        is_exp && !subs.is_empty(),
                        78.,
                        0.,
                        move || {
                            let mut col = div()
                                .pl(px(34.))
                                .pr(px(10.))
                                .pb(px(8.))
                                .flex()
                                .flex_col()
                                .gap(px(4.));
                            for (si, s_txt) in subs.into_iter().enumerate() {
                                let is_done = task_state == RunningTaskState::Done || si < done_n;
                                let is_cur = !is_done && si == done_n;
                                let lead_mark = if is_done {
                                    pro_check_draw(
                                        pro_eid(&sub_base, format!("rn-sub-chk-{sub_tid}-{si}")),
                                        true,
                                        12.,
                                        false,
                                        p.teal,
                                    )
                                } else {
                                    let dot_c = if is_cur {
                                        if task_state == RunningTaskState::NeedsYou {
                                            p.amber
                                        } else {
                                            p.blue
                                        }
                                    } else {
                                        p.faint
                                    };
                                    div()
                                        .size(px(12.))
                                        .flex_none()
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .child(div().size(px(5.)).rounded_full().bg(dot_c))
                                        .into_any_element()
                                };
                                col = col.child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(6.))
                                        .text_size(px(11.5))
                                        .text_color(if is_cur {
                                            p.ink
                                        } else if is_done {
                                            p.muted
                                        } else {
                                            p.faint
                                        })
                                        .child(lead_mark)
                                        .child(div().truncate().child(s_txt)),
                                );
                            }
                            col.into_any_element()
                        },
                    );

                    let row_inner = div()
                        .w_full()
                        .h(px(52.))
                        .pl(px(9.))
                        .pr(px(4.))
                        .flex()
                        .items_center()
                        .gap(px(10.))
                        .child(pro_progress_ring(pct, 15., 2.2, ring_color, p.line_strong))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(if task.state == RunningTaskState::Done {
                                            p.text
                                        } else {
                                            p.ink
                                        })
                                        .truncate()
                                        .child(task.title.clone()),
                                )
                                .child(pro_view_transition(
                                    pro_eid(&base_id, format!("rn-step-tx-{}", task.id)),
                                    task.step.clone(),
                                    div()
                                        .text_size(px(12.))
                                        .text_color(p.muted)
                                        .truncate()
                                        .child(task.step.clone()),
                                )),
                        );

                    let row_tactile = pro_tactile(
                        pro_eid(&base_id, format!("rn-row-{}", task.id)),
                        SharedString::from(format!("{}: {}", task.title, task.step)),
                        6.,
                        transparent_black(),
                        p.hover,
                        transparent_black(),
                        transparent_black(),
                        is_exp || is_hi,
                        row_inner,
                        cx,
                        move |this, _, cx| this.open_task(&tid_main, cx),
                    )
                    .flex_1()
                    .min_w_0();

                    div()
                        .w_full()
                        .rounded(px(7.))
                        .bg(if is_hi { p.hover } else { transparent_black() })
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .w_full()
                                .pr(px(6.))
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .child(row_tactile)
                                .child(right_action),
                        )
                        .child(sub_fold)
                        .into_any_element()
                })
                .collect();

            let count_num = tasks_snap.len();
            let reset_pill = any_done.then(|| {
                pro_btn(
                    pro_eid(&base_id, "rn-reset-btn"),
                    reset_lbl,
                    Some(ProIcon::Refresh),
                    ProBtnStyle::Ghost,
                    true,
                    cx,
                    |this, _, cx| {
                        this.tasks = default_running_tasks();
                        this.done_clocks.clear();
                        this.last_tick = Instant::now();
                        cx.notify();
                    },
                )
                .ml_auto()
            });

            let open_all_inner = div()
                .w_full()
                .h(px(32.))
                .px(px(9.))
                .flex()
                .items_center()
                .gap(px(8.))
                .text_size(px(13.))
                .text_color(p.ink)
                .child(pro_icon(ProIcon::Tasks, 14., p.muted))
                .child(div().flex_1().child(open_all_lbl.clone()))
                .child(pro_icon(ProIcon::ChevronRight, 12., p.muted));

            let open_all_btn = pro_tactile(
                pro_eid(&base_id, "rn-open-all"),
                open_all_lbl,
                6.,
                transparent_black(),
                p.hover,
                transparent_black(),
                transparent_black(),
                false,
                open_all_inner,
                cx,
                |this, _, cx| {
                    this.toast = Some((
                        tr(cx, "Opened Tasks workspace", "已跳转至全部任务视图").into(),
                        Instant::now(),
                    ));
                    cx.emit(RunningNowEvent::OpenedAll);
                    cx.notify();
                },
            )
            .w_full();

            pro_popover_presence(pro_eid(&self.id, "rn-popover-pres"), self.open, move || {
                div()
                    .mt(px(6.))
                    .w(px(pop_w))
                    .p(px(4.))
                    .rounded(px(10.))
                    .bg(if p.dark { rgb(0x1c1c1f).into() } else { p.card })
                    .border_1()
                    .border_color(p.line_strong)
                    .shadow_lg()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .h(px(28.))
                            .px(px(8.))
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .child(
                                div()
                                    .text_size(px(10.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.muted)
                                    .child(head_lbl),
                            )
                            .child(
                                div()
                                    .text_size(px(10.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.faint)
                                    .child(format!("{count_num}")),
                            )
                            .when_some(reset_pill, |d, rp| d.child(rp)),
                    )
                    .child(if rows_els.is_empty() {
                        div()
                            .px(px(10.))
                            .py(px(16.))
                            .text_size(px(12.))
                            .text_color(p.muted)
                            .child(empty_copy)
                            .into_any_element()
                    } else {
                        div()
                            .id(pro_eid(&base_id, "rn-list-scroll"))
                            .max_h(px(292.))
                            .overflow_y_scroll()
                            .flex()
                            .flex_col()
                            .children(rows_els)
                            .into_any_element()
                    })
                    .child(
                        div()
                            .mt(px(4.))
                            .pt(px(4.))
                            .border_t_1()
                            .border_color(p.line)
                            .child(open_all_btn),
                    )
                    .into_any_element()
            })
        };

        let body = if self.header_chrome {
            div()
                .w_full()
                .max_w(px(720.))
                .min_h(px(stage_min_h))
                .flex()
                .flex_col()
                .child(
                    div()
                        .w_full()
                        .h(px(48.))
                        .px(px(14.))
                        .rounded(px(10.))
                        .bg(if p.dark {
                            rgb(0x18181b).into()
                        } else {
                            p.subtle
                        })
                        .border_1()
                        .border_color(p.line)
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(12.))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .min_w_0()
                                .child(
                                    div()
                                        .size(px(18.))
                                        .rounded(px(5.))
                                        .bg(p.strong)
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .text_size(px(10.5))
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(p.card)
                                        .child("K"),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(6.))
                                        .text_size(px(13.))
                                        .truncate()
                                        .child(
                                            div()
                                                .text_color(p.muted)
                                                .child(self.workspace_name.clone()),
                                        )
                                        .child(div().text_color(p.faint).child("/"))
                                        .child(
                                            div()
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(p.ink)
                                                .child(tr(cx, "Agents", "智能体")),
                                        ),
                                ),
                        )
                        .child(trigger_pill),
                )
                .child(
                    div()
                        .w_full()
                        .flex()
                        .when(self.align == RunningNowAlign::End, |d| d.justify_end())
                        .when(self.align == RunningNowAlign::Start, |d| d.justify_start())
                        .child(popover),
                )
                .into_any_element()
        } else {
            div()
                .flex()
                .flex_col()
                .when(self.align == RunningNowAlign::End, |d| d.items_end())
                .when(self.align == RunningNowAlign::Start, |d| d.items_start())
                .child(trigger_pill)
                .child(popover)
                .into_any_element()
        };

        let _ = &self.page_name;
        if self.empty && !self.header_chrome && !self.open {
            return div().id(self.id.clone()).child(pro_empty_state(
                ProIcon::Activity,
                tr(cx, "No tasks running", "暂无运行任务"),
                tr(
                    cx,
                    "Background agent jobs appear here.",
                    "后台智能体作业将汇总在此。",
                ),
                None,
                cx,
            ));
        }

        div()
            .id(self.id.clone())
            .role(Role::Region)
            .aria_label(SharedString::from(tr(
                cx,
                "Running now status",
                "正在运行的后台任务",
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
            .child(body)
            .child(pro_toast_banner(self.toast.clone()))
    }
}

fn default_running_tasks() -> Vec<RunningTask> {
    vec![
        RunningTask::new(
            "ingest",
            "Ingest pipeline",
            "Embedding chunks",
            2,
            4,
            RunningTaskState::Running,
        )
        .with_sub_steps(vec![
            "Parsed 184 source docs".into(),
            "Chunked into 1,420 passages".into(),
            "Embedding chunks".into(),
            "Write vector index".into(),
        ]),
        RunningTask::new(
            "refund",
            "Refund review",
            "Approve a $240 refund to Northwind",
            1,
            3,
            RunningTaskState::NeedsYou,
        )
        .with_sub_steps(vec![
            "Verified invoice #8912".into(),
            "Approve a $240 refund to Northwind".into(),
            "Issue Stripe credit".into(),
        ]),
        RunningTask::new(
            "report",
            "Weekly report",
            "Building 4 charts",
            2,
            5,
            RunningTaskState::Running,
        )
        .with_sub_steps(vec![
            "Queried analytics warehouse".into(),
            "Normalized weekly cohorts".into(),
            "Building 4 charts".into(),
            "Draft executive summary".into(),
            "Publish to #metrics".into(),
        ]),
        RunningTask::new(
            "deps",
            "Dependency upgrade",
            "Running 212 tests",
            1,
            4,
            RunningTaskState::Paused,
        )
        .with_sub_steps(vec![
            "Updated lockfile".into(),
            "Running 212 tests".into(),
            "Verify wasm32 check".into(),
            "Open pull request".into(),
        ]),
        RunningTask::new(
            "leads",
            "Lead enrichment",
            "Loading 420 leads",
            1,
            6,
            RunningTaskState::Running,
        )
        .with_sub_steps(vec![
            "Connected HubSpot list".into(),
            "Loading 420 leads".into(),
            "Enrich company domains".into(),
            "Score buying intent".into(),
            "Assign territory owners".into(),
            "Notify account reps".into(),
        ]),
    ]
}

#[cfg(test)]
mod tests {
    use super::{RunningNowAlign, RunningTaskState, default_running_tasks};

    #[test]
    fn default_tasks_compute_expected_summary_and_progress() {
        let tasks = default_running_tasks();
        assert_eq!(tasks.len(), 5);
        let running = tasks
            .iter()
            .filter(|t| t.state == RunningTaskState::Running)
            .count();
        let needs_you = tasks
            .iter()
            .filter(|t| t.state == RunningTaskState::NeedsYou)
            .count();
        let paused = tasks
            .iter()
            .filter(|t| t.state == RunningTaskState::Paused)
            .count();
        assert_eq!((running, needs_you, paused), (3, 1, 1));
        assert!((tasks[0].progress() - 0.50).abs() < 0.001);
        assert_eq!(RunningNowAlign::default(), RunningNowAlign::End);
    }

    #[test]
    fn completed_state_reports_full_progress_regardless_of_done_field() {
        let mut t = default_running_tasks().remove(0);
        t.state = RunningTaskState::Done;
        assert_eq!(t.progress(), 1.0);
    }
}
