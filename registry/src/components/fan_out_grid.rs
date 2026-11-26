//! Dense live worker matrix for fan-out batch jobs with hover inspection and retry.
//!
//! Renders hundreds of parallel worker tasks as a compact status grid (`Queued`, `Running`,
//! `Retrying`, `Done`, `Failed`) with rolling completion, cost, and ETA readouts, interactive
//! status-filter legend toggles, failed-batch retry, and a floating task inspector card.

use crate::{
    components::pro_chrome::{
        ProPalette, pro_app_tile, pro_card_shadow, pro_ease, pro_eid, pro_empty_state,
        pro_mark_selected, pro_mix_color, pro_pill_indicator, pro_popover_presence, pro_presence,
        pro_rolling_number, pro_selection_sensor, pro_skeleton_bar, pro_spring_f32, pro_tactile,
        tr,
    },
    foundation::Theme,
    primitives::{
        family::{Slide, disabled_motion},
        pro_icons::{ProIcon, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, f32::consts::TAU, rc::Rc};
use web_time::Instant;

const DEFAULT_PANEL_W: f32 = 676.0;
const PANEL_PAD_X: f32 = 14.0;
const PANEL_PAD_TOP: f32 = 12.0;
const PANEL_PAD_BOTTOM: f32 = 10.0;
const PANEL_GAP: f32 = 10.0;
const FOOT_H: f32 = 22.0;
const CELL_SIDE: f32 = 10.5;
const CELL_GAP: f32 = 3.0;
const TIP_W: f32 = 252.0;

const FAN_OUT_MARK_SVG: &str = r#"<circle cx="3" cy="8" r="1.9" fill="currentColor"></circle><rect x="10.2" y="1.6" width="3.6" height="3.6" rx="1" fill="currentColor" opacity="0.9"></rect><rect x="10.2" y="6.2" width="3.6" height="3.6" rx="1" fill="currentColor" opacity="0.6"></rect><rect x="10.2" y="10.8" width="3.6" height="3.6" rx="1" fill="currentColor" opacity="0.3"></rect><path d="M4.9 8H7c.6 0 .9-.3 1-.9l.4-2.4c.1-.6.4-.9 1-.9h.6M4.9 8h5.1M4.9 8H7c.6 0 .9.3 1 .9l.4 2.4c.1.6.4.9 1 .9h.6" stroke="currentColor" stroke-width="1.1" stroke-linecap="round"></path>"#;

/// Execution state of a single worker cell in [`FanOutGrid`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FanOutStatus {
    /// Waiting in the work queue for an available worker slot.
    #[default]
    Queued,
    /// Actively executing on a worker with a live sweep sheen.
    Running,
    /// Backing off before another attempt after a transient failure.
    Retrying,
    /// Finished cleanly with an output summary.
    Done,
    /// Exhausted retries with a terminal error message.
    Failed,
}

impl FanOutStatus {
    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::Queued => tr(cx, "Queued", "排队中"),
            Self::Running => tr(cx, "Running", "运行中"),
            Self::Retrying => tr(cx, "Retrying", "重试中"),
            Self::Done => tr(cx, "Done", "已完成"),
            Self::Failed => tr(cx, "Failed", "失败"),
        }
    }

    fn legend_word(self, cx: &App) -> &'static str {
        match self {
            Self::Queued => tr(cx, "queued", "排队"),
            Self::Running => tr(cx, "running", "运行中"),
            Self::Retrying => tr(cx, "retrying", "重试中"),
            Self::Done => tr(cx, "done", "已完成"),
            Self::Failed => tr(cx, "failed", "失败"),
        }
    }
}

#[derive(Clone, Copy)]
struct StatusColors {
    bg: Hsla,
    edge: Hsla,
    ink: Hsla,
}

fn status_colors(status: FanOutStatus, p: &ProPalette) -> StatusColors {
    if p.dark {
        match status {
            FanOutStatus::Queued => StatusColors {
                bg: hsla(0., 0., 1., 0.05),
                edge: hsla(0., 0., 1., 0.09),
                ink: p.muted,
            },
            FanOutStatus::Running => StatusColors {
                bg: p.blue.opacity(0.24),
                edge: p.blue.opacity(0.50),
                ink: rgb(0x8bcbfe).into(),
            },
            FanOutStatus::Retrying => StatusColors {
                bg: p.orange.opacity(0.24),
                edge: p.orange.opacity(0.50),
                ink: p.amber_ink,
            },
            FanOutStatus::Done => StatusColors {
                bg: p.green.opacity(0.22),
                edge: p.green.opacity(0.42),
                ink: rgb(0x5fc88a).into(),
            },
            FanOutStatus::Failed => StatusColors {
                bg: p.red.opacity(0.24),
                edge: p.red.opacity(0.50),
                ink: p.red_ink,
            },
        }
    } else {
        match status {
            FanOutStatus::Queued => StatusColors {
                bg: hsla(0., 0., 0., 0.035),
                edge: hsla(0., 0., 0., 0.07),
                ink: p.muted,
            },
            FanOutStatus::Running => StatusColors {
                bg: hsla(215. / 360., 1.0, 0.44, 0.16),
                edge: hsla(215. / 360., 1.0, 0.44, 0.30),
                ink: p.accent_info,
            },
            FanOutStatus::Retrying => StatusColors {
                bg: hsla(37. / 360., 1.0, 0.50, 0.19),
                edge: p.orange.opacity(0.45),
                ink: p.amber_ink,
            },
            FanOutStatus::Done => StatusColors {
                bg: p.green.opacity(0.20),
                edge: p.green.opacity(0.38),
                ink: hsla(142. / 360., 1.0, 0.25, 0.88),
            },
            FanOutStatus::Failed => StatusColors {
                bg: hsla(11. / 360., 1.0, 0.48, 0.18),
                edge: p.red.opacity(0.48),
                ink: hsla(10. / 360., 1.0, 0.42, 0.86),
            },
        }
    }
}

/// One individual task cell rendered inside [`FanOutGrid`].
#[derive(Clone, Debug, PartialEq)]
pub struct FanOutTask {
    /// Stable task identifier preserved across status updates and retries.
    pub id: SharedString,
    /// Primary label shown in the floating inspector (for example a domain or file path).
    pub label: SharedString,
    /// Optional Chinese label for bilingual display.
    pub label_zh: Option<SharedString>,
    /// Current cell status (`Queued`, `Running`, `Retrying`, `Done`, or `Failed`).
    pub status: FanOutStatus,
    /// Assigned worker identifier (for example `"worker-04"`).
    pub worker: Option<SharedString>,
    /// Start offset in milliseconds from the beginning of the batch job.
    pub start_ms: Option<u64>,
    /// Completion offset in milliseconds from the beginning of the batch job.
    pub end_ms: Option<u64>,
    /// Incremental task cost in USD.
    pub cost: Option<f64>,
    /// Attempt count (`2` and higher is surfaced in the cell inspector).
    pub attempts: u32,
    /// Short completion summary displayed when the task succeeds.
    pub result: Option<SharedString>,
    /// Optional Chinese completion summary.
    pub result_zh: Option<SharedString>,
    /// Error detail displayed in danger ink when retrying or failed.
    pub error: Option<SharedString>,
    /// Optional Chinese error detail.
    pub error_zh: Option<SharedString>,
}

impl FanOutTask {
    /// Create a [`FanOutTask`] with `id` and `label` (`Queued`, `1` attempt by default).
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            label_zh: None,
            status: FanOutStatus::Queued,
            worker: None,
            start_ms: None,
            end_ms: None,
            cost: None,
            attempts: 1,
            result: None,
            result_zh: None,
            error: None,
            error_zh: None,
        }
    }

    /// Provide a localized Chinese task label.
    pub fn with_label_zh(mut self, label_zh: impl Into<SharedString>) -> Self {
        self.label_zh = Some(label_zh.into());
        self
    }

    /// Set the task execution status.
    pub fn with_status(mut self, status: FanOutStatus) -> Self {
        self.status = status;
        self
    }

    /// Set the worker slot handling this task.
    pub fn with_worker(mut self, worker: impl Into<SharedString>) -> Self {
        self.worker = Some(worker.into());
        self
    }

    /// Set both `start_ms` and `end_ms` relative to the job start.
    pub fn with_timing(mut self, start_ms: u64, end_ms: u64) -> Self {
        self.start_ms = Some(start_ms);
        self.end_ms = Some(end_ms);
        self
    }

    /// Set `start_ms` relative to the job start (for in-flight tasks).
    pub fn with_start_ms(mut self, start_ms: u64) -> Self {
        self.start_ms = Some(start_ms);
        self
    }

    /// Set `end_ms` relative to the job start.
    pub fn with_end_ms(mut self, end_ms: u64) -> Self {
        self.end_ms = Some(end_ms);
        self
    }

    /// Set the USD cost of this task.
    pub fn with_cost(mut self, cost: f64) -> Self {
        self.cost = Some(cost);
        self
    }

    /// Set the number of attempts (`>= 2` renders in the inspector).
    pub fn with_attempts(mut self, attempts: u32) -> Self {
        self.attempts = attempts.max(1);
        self
    }

    /// Set the output summary shown on completion.
    pub fn with_result(mut self, result: impl Into<SharedString>) -> Self {
        self.result = Some(result.into());
        self
    }

    /// Provide a localized Chinese output summary.
    pub fn with_result_zh(mut self, result_zh: impl Into<SharedString>) -> Self {
        self.result_zh = Some(result_zh.into());
        self
    }

    /// Set the failure or retry backoff diagnostic.
    pub fn with_error(mut self, error: impl Into<SharedString>) -> Self {
        self.error = Some(error.into());
        self
    }

    /// Provide a localized Chinese error diagnostic.
    pub fn with_error_zh(mut self, error_zh: impl Into<SharedString>) -> Self {
        self.error_zh = Some(error_zh.into());
        self
    }

    fn display_label<'a>(&'a self, cx: &App) -> &'a str {
        match &self.label_zh {
            Some(zh) => tr(cx, self.label.as_ref(), zh.as_ref()),
            None => self.label.as_ref(),
        }
    }

    fn display_result<'a>(&'a self, cx: &App) -> Option<&'a str> {
        let en = self.result.as_deref()?;
        Some(match &self.result_zh {
            Some(zh) => tr(cx, en, zh.as_ref()),
            None => en,
        })
    }

    fn display_error<'a>(&'a self, cx: &App) -> Option<&'a str> {
        let en = self.error.as_deref()?;
        Some(match &self.error_zh {
            Some(zh) => tr(cx, en, zh.as_ref()),
            None => en,
        })
    }
}

/// Aggregated batch telemetry computed across all tasks in [`FanOutGrid`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FanOutCounts {
    /// Count of queued tasks.
    pub queued: usize,
    /// Count of running tasks.
    pub running: usize,
    /// Count of retrying tasks.
    pub retrying: usize,
    /// Count of completed tasks.
    pub done: usize,
    /// Count of failed tasks.
    pub failed: usize,
    /// Total task count.
    pub total: usize,
    /// Sum of USD costs across tasks.
    pub total_cost: f64,
    /// Estimated remaining seconds (`None` until at least 3 tasks finish or when complete).
    pub eta_secs: Option<u64>,
    /// Total elapsed seconds once every task has reached a terminal state.
    pub finished_secs: Option<u64>,
}

/// Compute aggregate counts, USD spend, and remaining-time estimate for a task slice.
pub fn compute_fan_out_counts(
    tasks: &[FanOutTask],
    workers: Option<usize>,
    now_ms: u64,
) -> FanOutCounts {
    let mut c = FanOutCounts {
        total: tasks.len(),
        ..FanOutCounts::default()
    };
    let mut dur_sum_ms = 0u64;
    let mut dur_count = 0usize;
    let mut max_end_ms = 0u64;

    for t in tasks {
        match t.status {
            FanOutStatus::Queued => c.queued += 1,
            FanOutStatus::Running => c.running += 1,
            FanOutStatus::Retrying => c.retrying += 1,
            FanOutStatus::Done => c.done += 1,
            FanOutStatus::Failed => c.failed += 1,
        }
        if let Some(cost) = t.cost
            && cost.is_finite()
        {
            c.total_cost += cost;
        }
        if matches!(t.status, FanOutStatus::Done | FanOutStatus::Failed)
            && let (Some(s), Some(e)) = (t.start_ms, t.end_ms)
            && e >= s
        {
            dur_sum_ms += e - s;
            dur_count += 1;
            max_end_ms = max_end_ms.max(e);
        }
    }

    let remaining = c.queued + c.running + c.retrying;
    if c.total > 0 && remaining == 0 {
        let end_ref = max_end_ms.max(now_ms);
        c.finished_secs = Some((end_ref as f64 / 1000.0).round().max(1.0) as u64);
    } else if remaining > 0 && dur_count >= 3 {
        let avg_ms = dur_sum_ms as f64 / dur_count as f64;
        let active_slots = workers
            .unwrap_or_else(|| (c.running + c.retrying).max(1))
            .max(1) as f64;
        let rem_ms = (remaining as f64 * avg_ms) / active_slots;
        c.eta_secs = Some((rem_ms / 1000.0).ceil().max(1.0) as u64);
    }
    c
}

#[derive(Clone, Copy)]
struct MatrixMetrics {
    cols: usize,
    rows: usize,
    step_x: f32,
    step_y: f32,
    height: f32,
}

fn compute_matrix_metrics(grid_w: f32, count: usize) -> MatrixMetrics {
    let w = grid_w.max(CELL_SIDE);
    let cols = (((w + CELL_GAP) / (CELL_SIDE + CELL_GAP)).floor() as usize).max(1);
    let rows = if count == 0 { 1 } else { count.div_ceil(cols) };
    let step_x = if cols > 1 && count > cols {
        ((w - cols as f32 * CELL_SIDE) / (cols - 1) as f32).max(CELL_GAP) + CELL_SIDE
    } else {
        CELL_SIDE + CELL_GAP
    };
    let step_y = CELL_SIDE + CELL_GAP;
    let height = rows as f32 * CELL_SIDE + rows.saturating_sub(1) as f32 * CELL_GAP;
    MatrixMetrics {
        cols,
        rows,
        step_x,
        step_y,
        height,
    }
}

fn format_duration_ms(ms: u64) -> String {
    if ms < 1_000 {
        format!("{ms}ms")
    } else {
        format!("{:.1}s", ms as f64 / 1_000.0)
    }
}

/// Typed events emitted by [`FanOutGrid`].
#[derive(Clone, Debug, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum FanOutGridEvent {
    /// User pinned or cleared a task cell via click, `Enter` / `Space`, or `Escape`.
    Select(Option<FanOutTask>),
    /// User pressed `Retry failed` with the list of failed task ids.
    Retry { ids: Vec<SharedString> },
    /// User toggled a status filter in the bottom legend tray.
    FilterChanged(Option<FanOutStatus>),
}

impl EventEmitter<FanOutGridEvent> for FanOutGrid {}

/// Live multi-worker batch grid with rolling telemetry, status filter tray, and floating cell inspector.
///
/// # Examples
///
/// ```no_run
/// # #[allow(unused_variables, unused_imports)]
/// # fn example(window: &mut gpui::Window, cx: &mut gpui::App) {
/// # use gpui::prelude::*;
/// use gpui::prelude::*;
/// use kui::components::fan_out_grid::{
///     FanOutGrid, FanOutGridEvent, FanOutStatus, FanOutTask,
/// };
///
/// let grid = cx.new(|cx| {
///     FanOutGrid::new("lead-enrichment", cx)
///         .with_job("Enrich 240 leads")
///         .with_workers(16)
///         .with_tasks(vec![
///             FanOutTask::new("t1", "linear.app").with_status(FanOutStatus::Done).with_timing(0, 1_180).with_cost(0.014).with_result("Series B · ICP score 94%"),
///             FanOutTask::new("t2", "stripe.com").with_status(FanOutStatus::Running).with_start_ms(1_200).with_worker("worker-02"),
///             FanOutTask::new("t3", "vercel.com").with_status(FanOutStatus::Failed).with_attempts(3).with_error("429 rate_limit_exceeded"),
///         ])
/// });
/// cx.subscribe(&grid, |_, ev: &FanOutGridEvent, _| {
///     let _ = ev;
/// }).detach();
/// # }
/// ```
pub struct FanOutGrid {
    id: ElementId,
    title: SharedString,
    title_zh: SharedString,
    job: SharedString,
    job_zh: SharedString,
    icon: Option<SharedString>,
    tasks: Vec<FanOutTask>,
    workers: Option<usize>,
    now_ms: u64,
    hovered_idx: Option<usize>,
    focused_idx: Option<usize>,
    pinned_id: Option<SharedString>,
    active_filter: Option<FanOutStatus>,
    tip_x: Slide,
    tip_y: Slide,
    tip_ready: bool,
    last_tip_idx: Option<usize>,
    measured_grid_w: Rc<Cell<f32>>,
    started_at: Instant,
    loading: bool,
    empty: bool,
}

impl FanOutGrid {
    /// Create a [`FanOutGrid`] preloaded with a 240-lead enrichment fan-out batch.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        Self {
            id: id.into(),
            title: SharedString::from("Fan-out"),
            title_zh: SharedString::from("并发扇出"),
            job: SharedString::from("Enrich 240 leads"),
            job_zh: SharedString::from("补全 240 条线索"),
            icon: None,
            tasks: default_enrichment_tasks(),
            workers: Some(16),
            now_ms: 19_400,
            hovered_idx: None,
            focused_idx: None,
            pinned_id: None,
            active_filter: None,
            tip_x: Slide::snappy(8.0),
            tip_y: Slide::snappy(48.0),
            tip_ready: false,
            last_tip_idx: None,
            measured_grid_w: Rc::new(Cell::new(DEFAULT_PANEL_W - 2.0 * PANEL_PAD_X)),
            started_at: Instant::now(),
            loading: false,
            empty: false,
        }
    }

    /// Set the heading title shown before the job description (`"Fan-out"` by default).
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = title.into();
        self
    }

    /// Provide a Chinese heading title for bilingual switching.
    pub fn with_title_zh(mut self, title_zh: impl Into<SharedString>) -> Self {
        self.title_zh = title_zh.into();
        self
    }

    /// Set the batch job summary next to the heading (`"Enrich 240 leads"` by default).
    pub fn with_job(mut self, job: impl Into<SharedString>) -> Self {
        self.job = job.into();
        self
    }

    /// Provide a Chinese batch job summary for bilingual switching.
    pub fn with_job_zh(mut self, job_zh: impl Into<SharedString>) -> Self {
        self.job_zh = job_zh.into();
        self
    }

    /// Set an optional app or model mark key rendered before the title.
    pub fn with_icon(mut self, icon: impl Into<SharedString>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Supply the ordered list of tasks rendered as grid cells.
    pub fn with_tasks(mut self, tasks: Vec<FanOutTask>) -> Self {
        self.tasks = tasks;
        self
    }

    /// Set the concurrent worker slot count used for remaining-time estimation.
    pub fn with_workers(mut self, workers: usize) -> Self {
        self.workers = Some(workers.max(1));
        self
    }

    /// Set the elapsed batch clock in milliseconds from job start.
    pub fn with_now_ms(mut self, now_ms: u64) -> Self {
        self.now_ms = now_ms;
        self
    }

    /// Pre-pin a specific task id so its inspector card starts open.
    pub fn with_selected_id(mut self, id: Option<SharedString>) -> Self {
        self.pinned_id = id;
        self
    }

    /// Toggle skeleton loading placeholders.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Force the empty state placeholder.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Return the current slice of fan-out tasks.
    pub fn tasks(&self) -> &[FanOutTask] {
        &self.tasks
    }

    /// Replace the task slice in place while preserving any pinned cell selection.
    pub fn set_tasks(&mut self, tasks: Vec<FanOutTask>, cx: &mut Context<Self>) {
        self.tasks = tasks;
        if let Some(pin) = &self.pinned_id
            && !self.tasks.iter().any(|t| &t.id == pin)
        {
            self.pinned_id = None;
        }
        cx.notify();
    }

    /// Advance the elapsed batch clock in milliseconds.
    pub fn set_now_ms(&mut self, now_ms: u64, cx: &mut Context<Self>) {
        self.now_ms = now_ms;
        cx.notify();
    }

    /// Compute current batch status counts, spend, and ETA.
    pub fn counts(&self) -> FanOutCounts {
        compute_fan_out_counts(&self.tasks, self.workers, self.now_ms)
    }

    /// Return the currently pinned task, if any.
    pub fn selected_task(&self) -> Option<&FanOutTask> {
        let pin = self.pinned_id.as_ref()?;
        self.tasks.iter().find(|t| &t.id == pin)
    }

    /// Pin or clear a task by id and emit [`FanOutGridEvent::Select`].
    pub fn select_task(&mut self, id: Option<&str>, cx: &mut Context<Self>) {
        self.pinned_id = id.and_then(|target| {
            self.tasks
                .iter()
                .find(|t| t.id.as_ref() == target)
                .map(|t| t.id.clone())
        });
        let selected = self.selected_task().cloned();
        cx.emit(FanOutGridEvent::Select(selected));
        cx.notify();
    }

    /// Toggle a status filter in the bottom legend tray and emit [`FanOutGridEvent::FilterChanged`].
    pub fn toggle_filter(&mut self, status: FanOutStatus, cx: &mut Context<Self>) {
        self.active_filter = if self.active_filter == Some(status) {
            None
        } else {
            Some(status)
        };
        cx.emit(FanOutGridEvent::FilterChanged(self.active_filter));
        cx.notify();
    }

    /// Retry all failed tasks in place (`attempts + 1`, status -> `Running`) and emit [`FanOutGridEvent::Retry`].
    pub fn retry_failed(&mut self, cx: &mut Context<Self>) -> Vec<SharedString> {
        let mut retried = Vec::new();
        let now = self.now_ms;
        for t in &mut self.tasks {
            if t.status == FanOutStatus::Failed {
                t.status = FanOutStatus::Running;
                t.attempts = t.attempts.saturating_add(1).max(2);
                t.start_ms = Some(now);
                t.end_ms = None;
                t.error = None;
                t.error_zh = None;
                retried.push(t.id.clone());
            }
        }
        if self.active_filter == Some(FanOutStatus::Failed) {
            self.active_filter = None;
        }
        if !retried.is_empty() {
            cx.emit(FanOutGridEvent::Retry {
                ids: retried.clone(),
            });
            cx.notify();
        }
        retried
    }

    fn active_inspector_idx(&self) -> Option<usize> {
        if let Some(h) = self.hovered_idx
            && h < self.tasks.len()
        {
            return Some(h);
        }
        if let Some(pin) = &self.pinned_id
            && let Some(idx) = self.tasks.iter().position(|t| &t.id == pin)
        {
            return Some(idx);
        }
        if let Some(f) = self.focused_idx
            && f < self.tasks.len()
        {
            return Some(f);
        }
        None
    }
}

impl Render for FanOutGrid {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let reduced = disabled_motion(cx) || Theme::global(cx).reduced_motion;
        let elapsed_s = self.started_at.elapsed().as_secs_f32();
        let panel_bg: Hsla = if p.dark {
            rgb(0x1c1c1f).into()
        } else {
            p.panel
        };
        let panel_border: Hsla = if p.dark {
            hsla(0., 0., 1., 0.10)
        } else {
            hsla(0., 0., 0., 0.06)
        };
        let tip_bg: Hsla = if p.dark { rgb(0x232326).into() } else { p.card };
        let mark_accent: Hsla = if p.dark {
            p.blue
        } else {
            hsla(215. / 360., 1.0, 0.44, 0.765)
        };

        if self.loading {
            if !reduced {
                window.request_animation_frame();
            }
            let phase = (elapsed_s / 1.4).fract();
            return div()
                .id(self.id.clone())
                .w_full()
                .max_w(px(DEFAULT_PANEL_W))
                .flex()
                .flex_col()
                .gap(px(12.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(pro_skeleton_bar(180., 14., 4., phase, cx))
                        .child(pro_skeleton_bar(140., 14., 4., phase + 0.15, cx)),
                )
                .child(
                    div()
                        .w_full()
                        .p(px(16.))
                        .rounded(px(10.))
                        .bg(panel_bg)
                        .border_1()
                        .border_color(panel_border)
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .child(pro_skeleton_bar(420., 12., 3., phase, cx))
                        .child(pro_skeleton_bar(460., 12., 3., phase + 0.1, cx))
                        .child(pro_skeleton_bar(390., 12., 3., phase + 0.2, cx))
                        .child(pro_skeleton_bar(240., 12., 3., phase + 0.3, cx)),
                )
                .into_any_element();
        }

        if self.empty || self.tasks.is_empty() {
            return div()
                .id(self.id.clone())
                .w_full()
                .max_w(px(DEFAULT_PANEL_W))
                .rounded(px(10.))
                .bg(panel_bg)
                .border_1()
                .border_color(panel_border)
                .child(pro_empty_state(
                    ProIcon::Grid,
                    tr(cx, "No fan-out tasks queued", "暂无并发扇出任务"),
                    tr(
                        cx,
                        "Dispatch a batch job to watch parallel worker cells complete in real time.",
                        "派发批处理任务后，即可在此实时观察各工作节点完成进度。",
                    ),
                    Some((
                        pro_eid(&self.id, "empty-load"),
                        SharedString::from(tr(cx, "Load sample batch", "加载示例批次")),
                        Box::new(|this: &mut Self, _, cx| {
                            this.empty = false;
                            if this.tasks.is_empty() {
                                this.tasks = default_enrichment_tasks();
                            }
                            cx.notify();
                        }),
                    )),
                    cx,
                ))
                .into_any_element();
        }

        let counts = self.counts();
        let has_live_anim =
            !reduced && (counts.running > 0 || counts.retrying > 0 || elapsed_s < 0.90);
        if has_live_anim {
            window.request_animation_frame();
        }

        let lead_mark = match &self.icon {
            Some(app) => pro_app_tile(app.as_ref(), 16., cx),
            None => pro_svg("0 0 16 16", FAN_OUT_MARK_SVG, 16., mark_accent),
        };

        let done_colors = status_colors(FanOutStatus::Done, &p);
        let eta_el = if let Some(fin_s) = counts.finished_secs {
            div()
                .text_size(px(12.))
                .text_color(done_colors.ink)
                .child(format!("{} · {fin_s}s", tr(cx, "Done", "已完成")))
                .into_any_element()
        } else if let Some(eta_s) = counts.eta_secs {
            div()
                .text_size(px(12.))
                .text_color(p.muted)
                .child(tr(cx, &format!("~{eta_s}s left"), &format!("剩余约 {eta_s}s")).to_string())
                .into_any_element()
        } else {
            div()
                .text_size(px(12.))
                .text_color(p.muted)
                .child(tr(cx, "Estimating…", "估算中…"))
                .into_any_element()
        };

        let header = div()
            .w_full()
            .mb(px(8.))
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(8.))
            .child(lead_mark)
            .child(
                div()
                    .text_size(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .child(tr(cx, self.title.as_ref(), self.title_zh.as_ref()).to_string()),
            )
            .child(
                div()
                    .text_size(px(13.))
                    .text_color(p.muted)
                    .child(tr(cx, self.job.as_ref(), self.job_zh.as_ref()).to_string()),
            )
            .child(
                div()
                    .ml_auto()
                    .flex()
                    .items_center()
                    .gap(px(14.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(2.))
                            .child(pro_rolling_number(
                                pro_eid(&self.id, "done-num"),
                                counts.done as f64,
                                0,
                                "",
                                "",
                                12.,
                                FontWeight::MEDIUM,
                                p.ink,
                            ))
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(p.muted)
                                    .child(format!(" / {}", counts.total)),
                            ),
                    )
                    .child(pro_rolling_number(
                        pro_eid(&self.id, "cost-num"),
                        counts.total_cost,
                        2,
                        "$",
                        "",
                        12.,
                        FontWeight::MEDIUM,
                        p.ink,
                    ))
                    .child(eta_el),
            );

        let cur_grid_w = self.measured_grid_w.get();
        let metrics = compute_matrix_metrics(cur_grid_w, self.tasks.len());
        let active_idx = self.active_inspector_idx();
        let pinned_idx = self
            .pinned_id
            .as_ref()
            .and_then(|pin| self.tasks.iter().position(|t| &t.id == pin));

        let tasks_snapshot: Vec<FanOutStatus> = self.tasks.iter().map(|t| t.status).collect();
        let filter = self.active_filter;
        let width_cell = self.measured_grid_w.clone();
        let grid_origin_cell: Rc<Cell<Option<Point<Pixels>>>> = Rc::new(Cell::new(None));
        let origin_for_prepaint = grid_origin_cell.clone();
        let dark = p.dark;
        let pin_ring: Hsla = if dark {
            p.blue.opacity(0.45)
        } else {
            hsla(215. / 360., 1.0, 0.44, 0.35)
        };

        let entity = cx.entity().downgrade();
        let matrix_canvas = canvas(
            move |bounds, window, cx| {
                origin_for_prepaint.set(Some(bounds.origin));
                let w = f32::from(bounds.size.width);
                if w > 24.0 && (w - width_cell.get()).abs() > 1.5 {
                    width_cell.set(w);
                    if let Some(ent) = entity.upgrade() {
                        ent.update(cx, |_, cx| cx.notify());
                    }
                    window.refresh();
                }
            },
            move |bounds, _, window, _| {
                let live_w = f32::from(bounds.size.width).max(CELL_SIDE);
                let m = compute_matrix_metrics(live_w, tasks_snapshot.len());
                for (idx, &status) in tasks_snapshot.iter().enumerate() {
                    let col = idx % m.cols;
                    let row = idx / m.cols;
                    let x = f32::from(bounds.left()) + col as f32 * m.step_x;
                    let y = f32::from(bounds.top()) + row as f32 * m.step_y;

                    let enter = if reduced {
                        1.0
                    } else {
                        let delay = (idx as f32 * 0.002).min(0.42);
                        pro_ease(((elapsed_s - delay) / 0.36).clamp(0., 1.))
                    };
                    if enter <= 0.01 {
                        continue;
                    }

                    let is_active = active_idx == Some(idx);
                    let is_pinned = pinned_idx == Some(idx);
                    let dimmed = filter.is_some_and(|f| f != status);

                    let scale = if is_active { 1.30 } else { 0.60 + 0.40 * enter };
                    let breathe = if !reduced && status == FanOutStatus::Retrying {
                        let ph = (elapsed_s / 1.10 + idx as f32 * 0.13) * TAU;
                        0.58 + 0.42 * (0.5 + 0.5 * ph.sin())
                    } else {
                        1.0
                    };
                    let alpha = enter * breathe * if dimmed { 0.18 } else { 1.0 };

                    let side = CELL_SIDE * scale;
                    let delta = (side - CELL_SIDE) * 0.5;
                    let rect = Bounds::new(
                        point(px(x - delta), px(y - delta)),
                        size(px(side), px(side)),
                    );
                    let sc = status_colors(status, &p);

                    if is_pinned {
                        let ring_rect = Bounds::new(
                            point(rect.origin.x - px(2.), rect.origin.y - px(2.)),
                            size(rect.size.width + px(4.), rect.size.height + px(4.)),
                        );
                        window.paint_quad(
                            fill(ring_rect, pin_ring.opacity(alpha)).corner_radii(px(5.)),
                        );
                    }

                    let stroke_c = if is_active || is_pinned {
                        sc.ink.opacity(alpha)
                    } else {
                        sc.edge.opacity(alpha * 0.45)
                    };
                    window.paint_quad(quad(
                        rect,
                        Corners::all(px(2.6 * scale.min(1.15))),
                        sc.bg.opacity(alpha),
                        Edges::all(px(1.0)),
                        stroke_c,
                        BorderStyle::Solid,
                    ));

                    if !reduced && status == FanOutStatus::Running && !dimmed {
                        let sweep = ((elapsed_s + idx as f32 * 0.097) / 1.30).fract();
                        let band_w = (side * 0.46).max(3.0);
                        let band_x = -band_w + sweep * (side + band_w * 1.4);
                        let cl_left = band_x.max(1.0);
                        let cl_right = (band_x + band_w).min(side - 1.0);
                        if cl_right > cl_left + 0.5 {
                            let sheen_rect = Bounds::new(
                                point(rect.origin.x + px(cl_left), rect.origin.y + px(1.)),
                                size(px(cl_right - cl_left), px((side - 2.).max(1.))),
                            );
                            let shine = hsla(0., 0., 1., if dark { 0.28 } else { 0.62 } * alpha);
                            window.paint_quad(fill(sheen_rect, shine).corner_radii(px(2.)));
                        }
                    }
                }
            },
        )
        .w_full()
        .h(px(metrics.height));

        let origin_for_move = grid_origin_cell.clone();
        let origin_for_click = grid_origin_cell;
        let width_for_move = self.measured_grid_w.clone();
        let width_for_click = self.measured_grid_w.clone();

        let grid_el = div()
            .id(pro_eid(&self.id, "grid"))
            .role(Role::Group)
            .aria_label(
                tr(
                    cx,
                    "Fan-out worker task matrix — arrow keys inspect, Enter pins details",
                    "并发扇出任务矩阵 — 方向键查看详情，回车固定任务卡片",
                )
                .to_string(),
            )
            .tab_index(0)
            .w_full()
            .rounded(px(6.))
            .cursor_pointer()
            .focus_visible(move |s| s.border_1().border_color(p.focus))
            .on_hover(cx.listener(|this, inside: &bool, _, cx| {
                if !*inside && this.hovered_idx.is_some() {
                    this.hovered_idx = None;
                    cx.notify();
                }
            }))
            .on_mouse_move(cx.listener(move |this, ev: &MouseMoveEvent, _, cx| {
                let Some(origin) = origin_for_move.get() else {
                    return;
                };
                let m = compute_matrix_metrics(width_for_move.get(), this.tasks.len());
                let lx = f32::from(ev.position.x - origin.x);
                let ly = f32::from(ev.position.y - origin.y);
                let hit = if lx >= -2.0 && ly >= -2.0 {
                    let col = (lx / m.step_x).floor() as isize;
                    let row = (ly / m.step_y).floor() as isize;
                    if col >= 0 && (col as usize) < m.cols && row >= 0 && (row as usize) < m.rows {
                        let idx = row as usize * m.cols + col as usize;
                        (idx < this.tasks.len()).then_some(idx)
                    } else {
                        None
                    }
                } else {
                    None
                };
                if this.hovered_idx != hit {
                    this.hovered_idx = hit;
                    cx.notify();
                }
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, ev: &MouseDownEvent, _, cx| {
                    let Some(origin) = origin_for_click.get() else {
                        return;
                    };
                    let m = compute_matrix_metrics(width_for_click.get(), this.tasks.len());
                    let lx = f32::from(ev.position.x - origin.x);
                    let ly = f32::from(ev.position.y - origin.y);
                    if lx < -2.0 || ly < -2.0 {
                        return;
                    }
                    let col = (lx / m.step_x).floor() as isize;
                    let row = (ly / m.step_y).floor() as isize;
                    if col >= 0 && (col as usize) < m.cols && row >= 0 && (row as usize) < m.rows {
                        let idx = row as usize * m.cols + col as usize;
                        if let Some(task) = this.tasks.get(idx) {
                            this.focused_idx = Some(idx);
                            let next = if this.pinned_id.as_ref() == Some(&task.id) {
                                None
                            } else {
                                Some(task.id.clone())
                            };
                            this.pinned_id = next;
                            let sel = this.selected_task().cloned();
                            cx.emit(FanOutGridEvent::Select(sel));
                            cx.notify();
                        }
                    }
                }),
            )
            .on_key_down(cx.listener(move |this, ev: &KeyDownEvent, _, cx| {
                let len = this.tasks.len();
                if len == 0 {
                    return;
                }
                let cols = compute_matrix_metrics(this.measured_grid_w.get(), len).cols;
                let cur = this
                    .focused_idx
                    .or_else(|| {
                        this.pinned_id
                            .as_ref()
                            .and_then(|pin| this.tasks.iter().position(|t| &t.id == pin))
                    })
                    .unwrap_or(0);
                match ev.keystroke.key.as_str() {
                    "left" | "arrowleft" => {
                        this.focused_idx = Some(cur.saturating_sub(1));
                        cx.stop_propagation();
                        cx.notify();
                    }
                    "right" | "arrowright" => {
                        this.focused_idx = Some((cur + 1).min(len - 1));
                        cx.stop_propagation();
                        cx.notify();
                    }
                    "up" | "arrowup" => {
                        this.focused_idx = Some(cur.saturating_sub(cols));
                        cx.stop_propagation();
                        cx.notify();
                    }
                    "down" | "arrowdown" => {
                        this.focused_idx = Some((cur + cols).min(len - 1));
                        cx.stop_propagation();
                        cx.notify();
                    }
                    "home" => {
                        this.focused_idx = Some(0);
                        cx.stop_propagation();
                        cx.notify();
                    }
                    "end" => {
                        this.focused_idx = Some(len - 1);
                        cx.stop_propagation();
                        cx.notify();
                    }
                    "enter" | "space" | " " => {
                        if let Some(task) = this.tasks.get(cur) {
                            this.pinned_id = if this.pinned_id.as_ref() == Some(&task.id) {
                                None
                            } else {
                                Some(task.id.clone())
                            };
                            let sel = this.selected_task().cloned();
                            cx.emit(FanOutGridEvent::Select(sel));
                        }
                        cx.stop_propagation();
                        cx.notify();
                    }
                    "escape" => {
                        this.focused_idx = None;
                        this.hovered_idx = None;
                        if this.pinned_id.is_some() {
                            this.pinned_id = None;
                            cx.emit(FanOutGridEvent::Select(None));
                        }
                        cx.stop_propagation();
                        cx.notify();
                    }
                    _ => {}
                }
            }))
            .child(matrix_canvas);

        // Legend filter buttons with liquid pill indicator + Retry failed button
        let legend_entries = [
            (FanOutStatus::Done, counts.done, "done"),
            (FanOutStatus::Running, counts.running, "running"),
            (FanOutStatus::Retrying, counts.retrying, "retrying"),
            (FanOutStatus::Failed, counts.failed, "failed"),
        ];
        let flt_sensor = pro_selection_sensor();
        let active_slug = self.active_filter.map(|st| match st {
            FanOutStatus::Done => "done",
            FanOutStatus::Running => "running",
            FanOutStatus::Retrying => "retrying",
            FanOutStatus::Failed => "failed",
            FanOutStatus::Queued => "queued",
        });
        let mut legend = div()
            .id(pro_eid(&self.id, "legend"))
            .role(Role::Group)
            .aria_label(tr(cx, "Show only status", "按状态筛选"))
            .relative()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(2.));

        if let Some(sel_slug) = active_slug {
            legend = legend.child(pro_pill_indicator(
                pro_eid(&self.id, "flt-ind"),
                sel_slug,
                flt_sensor.clone(),
                6.0,
                p.chip,
                Some(p.line),
                false,
            ));
        }

        for (st, cnt, slug) in legend_entries {
            if cnt == 0 && self.active_filter != Some(st) {
                continue;
            }
            let sc = status_colors(st, &p);
            let pressed = self.active_filter == Some(st);
            let other_active = self.active_filter.is_some() && !pressed;
            let dim_t = pro_spring_f32(
                pro_eid(&self.id, format!("flt-dim-{slug}")),
                if other_active { 0.45 } else { 1.0 },
                0.20,
                window,
                cx,
            );
            let flt_s = flt_sensor.clone();
            legend = legend.child(
                div()
                    .relative()
                    .opacity(dim_t)
                    .when(pressed, move |d| d.child(pro_mark_selected(&flt_s)))
                    .child(
                        pro_tactile(
                            pro_eid(&self.id, format!("flt-{slug}")),
                            format!("{} {}", cnt, st.legend_word(cx)),
                            6.0,
                            transparent_black(),
                            p.chip_hover,
                            transparent_black(),
                            p.line,
                            pressed,
                            div()
                                .h(px(FOOT_H))
                                .px(px(6.))
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .text_size(px(12.))
                                .text_color(if pressed { p.ink } else { p.muted })
                                .child(
                                    div()
                                        .size(px(8.))
                                        .rounded(px(2.5))
                                        .bg(sc.bg)
                                        .border_1()
                                        .border_color(sc.edge.opacity(0.55)),
                                )
                                .child(
                                    div()
                                        .text_color(p.ink)
                                        .font_weight(FontWeight::MEDIUM)
                                        .child(cnt.to_string()),
                                )
                                .child(st.legend_word(cx)),
                            cx,
                            move |this, _, cx| {
                                this.toggle_filter(st, cx);
                            },
                        )
                        .w_auto(),
                    ),
            );
        }

        let failed_sc = status_colors(FanOutStatus::Failed, &p);
        let retry_label = tr(
            cx,
            &format!("Retry {} failed", counts.failed),
            &format!("重试 {} 个失败项", counts.failed),
        )
        .to_string();
        let retry_tactile = pro_tactile(
            pro_eid(&self.id, "retry-btn"),
            retry_label.clone(),
            6.0,
            transparent_black(),
            failed_sc.bg,
            transparent_black(),
            failed_sc.edge,
            false,
            div()
                .h(px(FOOT_H))
                .px(px(7.))
                .flex()
                .items_center()
                .gap(px(5.))
                .text_size(px(12.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(failed_sc.ink)
                .child(pro_icon(ProIcon::Refresh, 13., failed_sc.ink))
                .child(retry_label),
            cx,
            |this, _, cx| {
                this.retry_failed(cx);
            },
        )
        .w_auto();
        let retry_btn = pro_presence(
            pro_eid(&self.id, "retry-pres"),
            counts.failed > 0,
            0.20,
            0.16,
            {
                let mut btn_opt = Some(retry_tactile);
                move |amt| {
                    div()
                        .ml_auto()
                        .opacity(amt)
                        .children(btn_opt.take())
                        .into_any_element()
                }
            },
        );

        let footer = div()
            .w_full()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(12.))
            .child(legend)
            .child(div().ml_auto().child(retry_btn));

        let panel_h = PANEL_PAD_TOP + metrics.height + PANEL_GAP + FOOT_H + PANEL_PAD_BOTTOM;
        let panel_w = cur_grid_w + 2.0 * PANEL_PAD_X;

        // Floating detail card anchored to hovered/focused/pinned cell with `Slide::snappy` X/Y glide
        if let Some(idx) = active_idx {
            self.last_tip_idx = Some(idx);
            let col = idx % metrics.cols;
            let row = idx / metrics.cols;
            let cell_cx = PANEL_PAD_X + col as f32 * metrics.step_x + CELL_SIDE * 0.5;
            let cell_top = PANEL_PAD_TOP + row as f32 * metrics.step_y;
            let target_left = (cell_cx - TIP_W * 0.5).clamp(8.0, (panel_w - TIP_W - 8.0).max(8.0));
            let target_bottom = panel_h - cell_top + 8.0;
            if reduced || !self.tip_ready {
                self.tip_x.reset(target_left);
                self.tip_y.reset(target_bottom);
                self.tip_ready = true;
            } else {
                self.tip_x.duration(target_left, 0.18, cx);
                self.tip_y.duration(target_bottom, 0.18, cx);
            }
        } else {
            self.tip_ready = false;
        }
        let tip_left_px = self.tip_x.get(window, cx);
        let tip_bottom_px = self.tip_y.get(window, cx);
        let show_tip = active_idx.is_some();
        let display_idx = active_idx.or(self.last_tip_idx);

        let tip_data = display_idx.and_then(|idx| {
            let task = self.tasks.get(idx)?;
            let sc = status_colors(task.status, &p);
            let mut meta_parts: Vec<String> = Vec::new();
            if let Some(w) = &task.worker {
                meta_parts.push(w.to_string());
            }
            if let Some(s) = task.start_ms {
                let end = task.end_ms.unwrap_or_else(|| self.now_ms.max(s));
                if end > s {
                    meta_parts.push(format_duration_ms(end - s));
                }
            }
            if task.attempts >= 2 {
                meta_parts.push(
                    tr(
                        cx,
                        &format!("attempt {}", task.attempts),
                        &format!("第 {} 次尝试", task.attempts),
                    )
                    .to_string(),
                );
            }
            if let Some(cost) = task.cost {
                meta_parts.push(format!("${cost:.3}"));
            }
            let meta_line = meta_parts.join(" · ");
            let label_str = task.display_label(cx).to_string();
            let status_str = task.status.label(cx).to_string();
            let err_str = task.display_error(cx).map(str::to_string);
            let res_str = task.display_result(cx).map(str::to_string);
            let is_pinned = pinned_idx == Some(idx);
            let pin_hint = if is_pinned {
                Some(
                    tr(
                        cx,
                        "Pinned · click cell or Esc to release",
                        "已固定 · 点击单元格或按 Esc 取消",
                    )
                    .to_string(),
                )
            } else {
                None
            };
            Some((
                sc, label_str, status_str, meta_line, err_str, res_str, pin_hint,
            ))
        });

        let tip_overlay = pro_popover_presence(
            pro_eid(&self.id, "cell-tip-pop"),
            show_tip && tip_data.is_some(),
            {
                let mut data_opt = tip_data;
                move || {
                    let Some((sc, label_str, status_str, meta_line, err_str, res_str, pin_hint)) =
                        data_opt.take()
                    else {
                        return div().into_any_element();
                    };
                    div()
                        .absolute()
                        .left(px(tip_left_px))
                        .bottom(px(tip_bottom_px))
                        .w(px(TIP_W))
                        .px(px(12.))
                        .pt(px(9.))
                        .pb(px(10.))
                        .rounded(px(10.))
                        .bg(tip_bg)
                        .border_1()
                        .border_color(if p.dark {
                            hsla(0., 0., 1., 0.14)
                        } else {
                            hsla(0., 0., 0., 0.09)
                        })
                        .shadow_lg()
                        .flex()
                        .flex_col()
                        .gap(px(4.))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .truncate()
                                        .text_size(px(13.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.ink)
                                        .child(label_str),
                                )
                                .child(
                                    div()
                                        .h(px(18.))
                                        .px(px(6.))
                                        .flex_none()
                                        .rounded(px(4.))
                                        .bg(sc.bg)
                                        .text_size(px(11.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(sc.ink)
                                        .flex()
                                        .items_center()
                                        .child(status_str),
                                ),
                        )
                        .when(!meta_line.is_empty(), |d| {
                            d.child(
                                div()
                                    .text_size(px(11.5))
                                    .text_color(p.muted)
                                    .child(meta_line),
                            )
                        })
                        .when_some(err_str, |d, err| {
                            d.child(
                                div()
                                    .text_size(px(12.))
                                    .line_height(px(17.))
                                    .text_color(failed_sc.ink)
                                    .child(err),
                            )
                        })
                        .when_some(res_str, |d, res| {
                            d.child(
                                div()
                                    .text_size(px(12.))
                                    .line_height(px(17.))
                                    .text_color(p.text)
                                    .child(res),
                            )
                        })
                        .when_some(pin_hint, |d, hint| {
                            d.child(
                                div()
                                    .pt(px(2.))
                                    .text_size(px(10.5))
                                    .text_color(pro_mix_color(p.muted, p.blue, 0.45))
                                    .child(hint),
                            )
                        })
                        .into_any_element()
                }
            },
        );

        let panel = div()
            .relative()
            .w_full()
            .px(px(PANEL_PAD_X))
            .pt(px(PANEL_PAD_TOP))
            .pb(px(PANEL_PAD_BOTTOM))
            .rounded(px(10.))
            .bg(panel_bg)
            .border_1()
            .border_color(panel_border)
            .shadow(pro_card_shadow(p.dark))
            .flex()
            .flex_col()
            .gap(px(PANEL_GAP))
            .child(grid_el)
            .child(footer)
            .child(tip_overlay);

        div()
            .id(self.id.clone())
            .w_full()
            .flex()
            .justify_center()
            .child(
                div()
                    .w_full()
                    .max_w(px(DEFAULT_PANEL_W))
                    .flex()
                    .flex_col()
                    .child(header)
                    .child(panel),
            )
            .into_any_element()
    }
}

fn default_enrichment_tasks() -> Vec<FanOutTask> {
    const DOMAINS: [&str; 24] = [
        "linear.app",
        "stripe.com",
        "vercel.com",
        "supabase.com",
        "raycast.com",
        "figma.com",
        "retool.com",
        "notion.so",
        "intercom.com",
        "posthog.com",
        "planetscale.com",
        "resend.com",
        "clerk.com",
        "turso.tech",
        "neon.tech",
        "fly.io",
        "railway.app",
        "render.com",
        "workos.com",
        "sentry.io",
        "datadoghq.com",
        "loom.com",
        "attio.com",
        "ramp.com",
    ];
    const RESULTS_EN: [&str; 6] = [
        "Series B · 180 employees · DevTools · ICP score 94%",
        "Enterprise · 840 employees · FinTech · ICP score 91%",
        "Series A · 65 employees · Infra SaaS · ICP score 89%",
        "Series C · 410 employees · Collaboration · ICP score 87%",
        "Growth · 290 employees · Cloud Platform · ICP score 92%",
        "Seed+ · 42 employees · AI Tooling · ICP score 88%",
    ];
    const RESULTS_ZH: [&str; 6] = [
        "B 轮 · 180 人 · 开发者工具 · ICP 匹配度 94%",
        "企业级 · 840 人 · 金融科技 · ICP 匹配度 91%",
        "A 轮 · 65 人 · 基础设施 SaaS · ICP 匹配度 89%",
        "C 轮 · 410 人 · 协同办公 · ICP 匹配度 87%",
        "成长期 · 290 人 · 云平台 · ICP 匹配度 92%",
        "种子轮+ · 42 人 · AI 工具链 · ICP 匹配度 88%",
    ];

    let mut out = Vec::with_capacity(240);
    for idx in 0..240usize {
        let base = DOMAINS[idx % DOMAINS.len()];
        let cohort = idx / DOMAINS.len();
        let label = if cohort == 0 {
            base.to_string()
        } else {
            format!("{base} #{cohort}")
        };
        let worker = format!("worker-{:02}", (idx % 16) + 1);
        let dur_ms = 920 + ((idx as u64 * 173) % 1_450);
        let start_ms = (idx as u64 / 16) * 1_150 + ((idx as u64 % 7) * 90);

        let task = if matches!(idx, 31 | 58 | 93 | 119 | 134) {
            FanOutTask::new(format!("lead-{idx:03}"), label)
                .with_status(FanOutStatus::Failed)
                .with_worker(worker)
                .with_timing(start_ms, start_ms + dur_ms + 1_800)
                .with_attempts(3)
                .with_cost(0.018)
                .with_error("429 rate_limit_exceeded on enrichment upstream after 3 attempts")
                .with_error_zh("上游富化接口连续 3 次返回 429 速率限制错误")
        } else if matches!(idx, 108 | 127 | 141 | 149 | 154 | 161) {
            FanOutTask::new(format!("lead-{idx:03}"), label)
                .with_status(FanOutStatus::Retrying)
                .with_worker(worker)
                .with_start_ms(17_200)
                .with_attempts(2)
                .with_cost(0.014)
                .with_error("WHOIS socket timeout — retrying with exponential backoff (1.6s)")
                .with_error_zh("WHOIS 连接超时 — 正在指数退避重试 (1.6s)")
        } else if idx < 146 {
            let r_i = idx % RESULTS_EN.len();
            FanOutTask::new(format!("lead-{idx:03}"), label)
                .with_status(FanOutStatus::Done)
                .with_worker(worker)
                .with_timing(start_ms, start_ms + dur_ms)
                .with_attempts(if idx.is_multiple_of(19) { 2 } else { 1 })
                .with_cost(0.012 + (idx % 5) as f64 * 0.002)
                .with_result(RESULTS_EN[r_i])
                .with_result_zh(RESULTS_ZH[r_i])
        } else if idx < 168 {
            FanOutTask::new(format!("lead-{idx:03}"), label)
                .with_status(FanOutStatus::Running)
                .with_worker(worker)
                .with_start_ms(17_900 + (idx as u64 % 9) * 140)
                .with_cost(0.006)
                .with_result("Fetching LinkedIn headcount & Crunchbase funding signals…")
                .with_result_zh("正在抓取团队规模与融资信号…")
        } else {
            FanOutTask::new(format!("lead-{idx:03}"), label).with_status(FanOutStatus::Queued)
        };
        out.push(task);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{FanOutGrid, FanOutStatus, FanOutTask, compute_fan_out_counts};
    use gpui::{AppContext as _, TestAppContext};

    #[test]
    fn counts_aggregate_spend_and_estimate_eta_after_three_completions() {
        let tasks = vec![
            FanOutTask::new("t1", "a.io")
                .with_status(FanOutStatus::Done)
                .with_timing(0, 2_000)
                .with_cost(0.02),
            FanOutTask::new("t2", "b.io")
                .with_status(FanOutStatus::Done)
                .with_timing(0, 2_000)
                .with_cost(0.02),
            FanOutTask::new("t3", "c.io")
                .with_status(FanOutStatus::Done)
                .with_timing(0, 2_000)
                .with_cost(0.02),
            FanOutTask::new("t4", "d.io")
                .with_status(FanOutStatus::Running)
                .with_start_ms(2_000)
                .with_cost(0.01),
            FanOutTask::new("t5", "e.io").with_status(FanOutStatus::Queued),
        ];
        let c = compute_fan_out_counts(&tasks, Some(2), 2_500);
        assert_eq!(c.done, 3);
        assert_eq!(c.running, 1);
        assert_eq!(c.queued, 1);
        assert!((c.total_cost - 0.07).abs() < 1e-6);
        assert_eq!(c.eta_secs, Some(2));
    }

    #[gpui::test]
    fn retry_failed_requeues_failures_with_incremented_attempts(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let view = cx.new(|cx| FanOutGrid::new("test-fog", cx));
        view.update(cx, |v, cx| {
            let before = v.counts();
            assert_eq!(before.total, 240);
            assert!(before.failed > 0);
            let retried = v.retry_failed(cx);
            assert_eq!(retried.len(), before.failed);
            let after = v.counts();
            assert_eq!(after.failed, 0);
            assert_eq!(after.running, before.running + before.failed);
            let first = v
                .tasks()
                .iter()
                .find(|t| t.id == retried[0])
                .expect("retried task exists");
            assert_eq!(first.status, FanOutStatus::Running);
            assert!(first.attempts >= 2);
        });
    }
}
