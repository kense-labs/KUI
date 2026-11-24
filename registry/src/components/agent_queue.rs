//! Live job queue, worker pool slots, and backlog depth visualizer with rate-limit backpressure.
//!
//! Visualizes jobs entering a prioritized `Queued` lane, gliding into open worker slots with
//! growing execution progress bars, and landing in `Done` or `Failed` bins, paired with a live
//! [`PathBuilder`] backlog depth area chart that climbs during simulated provider rate limits
//! and drains smoothly once workers resume.

use crate::{
    components::pro_chrome::{
        ProBtnStyle, ProPalette, pro_agent_avatar, pro_app_tile, pro_btn, pro_check_draw, pro_eid,
        pro_empty_state, pro_fold, pro_icon_btn, pro_mark_selected, pro_pill_indicator,
        pro_rolling_number, pro_selection_sensor, pro_skeleton_bar, pro_spring_bar, pro_spring_f32,
        pro_swap_label, pro_tactile, tr,
    },
    foundation::Theme,
    primitives::{
        family::disabled_motion,
        pro_icons::{ProIcon, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use web_time::Instant;

const QUEUE_MARK_SVG: &str = r#"<path d="M18 18H6C4.9 18 4 17.1 4 16C4 14.9 4.9 14 6 14H18C19.1 14 20 14.9 20 16C20 17.1 19.1 18 18 18Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8"></path><path d="M4 10H20M6 6H18" stroke="currentColor" stroke-linecap="round" stroke-width="1.8"></path>"#;

/// Events emitted by [`AgentQueue`].
#[derive(Clone, Debug, PartialEq)]
pub enum AgentQueueEvent {
    /// Queue was paused (`true`) or resumed (`false`).
    PausedChanged(bool),
    /// Simulated provider rate limit started (`true`) or cleared (`false`).
    RateLimitToggled(bool),
    /// Active worker concurrency changed (`1..=6`).
    ConcurrencyChanged(usize),
    /// User promoted a queued job to the front of the queue.
    JobPromoted(SharedString),
    /// User cancelled a queued or running job.
    JobCancelled(SharedString),
    /// New job entered the queue.
    JobEnqueued(QueueJob),
    /// User pinned or closed a job inspector card.
    JobSelected(Option<SharedString>),
}

/// Semantic workload kind determining each job's tint palette.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum QueueJobKind {
    /// Model inference / synthesis call (violet tint).
    #[default]
    Llm,
    /// External API or database tool call (cyan/teal tint).
    Tool,
    /// Delegated sub-agent run (neutral slate tint).
    Agent,
    /// Human-in-the-loop review step (blue tint).
    Human,
}

impl QueueJobKind {
    fn colors(self, p: &ProPalette) -> (Hsla, Hsla, Hsla) {
        match self {
            Self::Llm => (
                p.purple.opacity(if p.dark { 0.16 } else { 0.11 }),
                p.purple.opacity(0.36),
                p.purple,
            ),
            Self::Tool => (
                p.cyan.opacity(if p.dark { 0.14 } else { 0.10 }),
                p.cyan.opacity(0.34),
                p.cyan,
            ),
            Self::Human => (p.accent_soft, p.blue.opacity(0.34), p.accent_info),
            Self::Agent => (p.chip, p.line_strong, p.text),
        }
    }

    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::Llm => tr(cx, "LLM call", "模型调用"),
            Self::Tool => tr(cx, "Tool call", "工具调用"),
            Self::Agent => tr(cx, "Sub-agent", "子智能体"),
            Self::Human => tr(cx, "Human check", "人工复核"),
        }
    }

    fn fallback_icon(self) -> ProIcon {
        match self {
            Self::Llm => ProIcon::Sparkles,
            Self::Tool => ProIcon::Database,
            Self::Agent => ProIcon::Bolt,
            Self::Human => ProIcon::User,
        }
    }
}

/// Execution lifecycle state of a [`QueueJob`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum QueueJobStatus {
    /// Waiting in line for an open worker slot.
    #[default]
    Queued,
    /// Assigned to a worker slot and growing its execution bar.
    Running,
    /// Completed successfully.
    Done,
    /// Terminated with an error after exhausting attempts.
    Failed,
}

/// Queue scheduling priority of a [`QueueJob`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum QueuePriority {
    /// Jumps ahead of normal and low priority jobs.
    High,
    /// Standard FIFO priority.
    #[default]
    Normal,
    /// Background fill priority.
    Low,
}

/// Worker slot definition in [`AgentQueue`].
#[derive(Clone, Debug, PartialEq)]
pub struct QueueWorker {
    /// Stable worker slot id (`"w1"`).
    pub id: SharedString,
    /// Display model/worker name (`"Sonnet 5.5"`).
    pub name: SharedString,
    /// Optional provider mark key (`"claude"`, `"openai"`, `"gemini"`).
    pub provider: Option<SharedString>,
}

impl QueueWorker {
    /// Create a [`QueueWorker`] with `id` and `name`.
    pub fn new(id: impl Into<SharedString>, name: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            provider: None,
        }
    }

    /// Attach a provider logo key rendered via [`pro_app_tile`].
    pub fn with_provider(mut self, provider: impl Into<SharedString>) -> Self {
        self.provider = Some(provider.into());
        self
    }
}

/// One queued, running, or completed job in [`AgentQueue`].
#[derive(Clone, Debug, PartialEq)]
pub struct QueueJob {
    /// Stable job identifier.
    pub id: SharedString,
    /// Concise task title (`"Embed 240 docs · 4/5"`).
    pub name: SharedString,
    /// Optional Chinese task title.
    pub name_zh: Option<SharedString>,
    /// Workload tint category.
    pub kind: QueueJobKind,
    /// Current execution status.
    pub status: QueueJobStatus,
    /// Scheduling priority (`High` sorts ahead of `Normal`).
    pub priority: QueuePriority,
    /// Assigned worker id while `Running`.
    pub worker: Option<SharedString>,
    /// Execution progress (`0.0..=1.0`) while `Running`.
    pub progress: f32,
    /// Elapsed wait or run seconds shown on the right edge of the job chip.
    pub elapsed_secs: f32,
    /// Attempt counter (`> 1` displays a retry badge).
    pub attempt: u32,
    /// Optional third-party app mark (`"github"`, `"notion"`, `"slack"`).
    pub app: Option<SharedString>,
    /// Optional failure message when `Failed` or retried.
    pub error: Option<SharedString>,
}

impl QueueJob {
    /// Create a [`QueueJob`] with `id`, `name`, and `kind`.
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        kind: QueueJobKind,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            name_zh: None,
            kind,
            status: QueueJobStatus::Queued,
            priority: QueuePriority::Normal,
            worker: None,
            progress: 0.0,
            elapsed_secs: 0.4,
            attempt: 1,
            app: None,
            error: None,
        }
    }

    /// Set localized Chinese job title.
    pub fn with_name_zh(mut self, name_zh: impl Into<SharedString>) -> Self {
        self.name_zh = Some(name_zh.into());
        self
    }

    /// Set job lifecycle status.
    pub fn with_status(mut self, status: QueueJobStatus) -> Self {
        self.status = status;
        self
    }

    /// Set scheduling priority.
    pub fn with_priority(mut self, priority: QueuePriority) -> Self {
        self.priority = priority;
        self
    }

    /// Assign to `worker_id` with initial `progress` (`0.0..=1.0`).
    pub fn with_running_on(mut self, worker_id: impl Into<SharedString>, progress: f32) -> Self {
        self.status = QueueJobStatus::Running;
        self.worker = Some(worker_id.into());
        self.progress = progress.clamp(0.0, 1.0);
        self
    }

    /// Set elapsed wait/run seconds.
    pub fn with_elapsed_secs(mut self, secs: f32) -> Self {
        self.elapsed_secs = secs.max(0.0);
        self
    }

    /// Set retry attempt number (`2` -> `↻2`).
    pub fn with_attempt(mut self, attempt: u32) -> Self {
        self.attempt = attempt.max(1);
        self
    }

    /// Set leading app logo key.
    pub fn with_app(mut self, app: impl Into<SharedString>) -> Self {
        self.app = Some(app.into());
        self
    }

    /// Attach a diagnostic error reason.
    pub fn with_error(mut self, error: impl Into<SharedString>) -> Self {
        self.error = Some(error.into());
        self
    }

    fn display_name<'a>(&'a self, cx: &App) -> &'a str {
        match &self.name_zh {
            Some(zh) => tr(cx, self.name.as_ref(), zh.as_ref()),
            None => self.name.as_ref(),
        }
    }
}

/// Live job queue, worker pool slots, and [`PathBuilder`] backlog depth chart.
pub struct AgentQueue {
    id: ElementId,
    title: SharedString,
    queue_name: SharedString,
    workers_catalog: Vec<QueueWorker>,
    concurrency: usize,
    min_concurrency: usize,
    max_concurrency: usize,
    jobs: Vec<QueueJob>,
    done_total: u64,
    failed_total: u64,
    throughput_per_min: u32,
    wait_p95_secs: u32,
    backpressure: u32,
    depth_history: Vec<(f32, bool)>,
    paused: bool,
    rate_limit_ticks_left: usize,
    live_flow: bool,
    applied_ticks: usize,
    seq_counter: usize,
    lane_filter: SharedString,
    selected_job: Option<SharedString>,
    last_inspected_job: Option<QueueJob>,
    loading: bool,
    empty: bool,
    started_at: Instant,
}

impl EventEmitter<AgentQueueEvent> for AgentQueue {}

impl AgentQueue {
    /// Create an [`AgentQueue`] visualizer preloaded with 4 worker slots, active jobs, and historical depth samples.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        Self {
            id: id.into(),
            title: SharedString::from("Queue"),
            queue_name: SharedString::from("agent-jobs"),
            workers_catalog: default_workers(),
            concurrency: 4,
            min_concurrency: 1,
            max_concurrency: 6,
            jobs: default_jobs(),
            done_total: 131,
            failed_total: 3,
            throughput_per_min: 77,
            wait_p95_secs: 13,
            backpressure: 8,
            depth_history: default_depth_history(),
            paused: false,
            rate_limit_ticks_left: 0,
            live_flow: true,
            applied_ticks: 0,
            seq_counter: 502,
            lane_filter: SharedString::from("all"),
            selected_job: None,
            last_inspected_job: None,
            loading: false,
            empty: false,
            started_at: Instant::now(),
        }
    }

    /// Set the heading and muted queue identifier (`"Queue"` / `"agent-jobs"`).
    pub fn with_title(
        mut self,
        title: impl Into<SharedString>,
        queue_name: impl Into<SharedString>,
    ) -> Self {
        self.title = title.into();
        self.queue_name = queue_name.into();
        self
    }

    /// Configure the worker pool catalog; initial concurrency matches `workers.len()`.
    pub fn with_workers(mut self, workers: Vec<QueueWorker>) -> Self {
        let len = workers.len().clamp(1, 8);
        self.workers_catalog = workers;
        self.concurrency = len;
        self.max_concurrency = len.max(self.max_concurrency);
        self
    }

    /// Supply initial queued and running jobs, disabling automatic demo spawning unless re-enabled.
    pub fn with_jobs(mut self, jobs: Vec<QueueJob>) -> Self {
        self.jobs = jobs;
        self.live_flow = false;
        self
    }

    /// Set the dashed backpressure depth threshold drawn on the area chart (`8` default).
    pub fn with_backpressure(mut self, threshold: u32) -> Self {
        self.backpressure = threshold.max(1);
        self
    }

    /// Set initial cumulative `done` and `failed` counters.
    pub fn with_totals(mut self, done: u64, failed: u64) -> Self {
        self.done_total = done;
        self.failed_total = failed;
        self
    }

    /// Start paused (`false` by default).
    pub fn with_paused(mut self, paused: bool) -> Self {
        self.paused = paused;
        self
    }

    /// Enable or disable live job progression and rate-limit spikes.
    pub fn with_live_flow(mut self, live: bool) -> Self {
        self.live_flow = live;
        self
    }

    /// Show skeleton placeholder state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Show empty queue state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Return current active worker concurrency.
    pub fn concurrency(&self) -> usize {
        self.concurrency
    }

    /// Return whether the queue is paused.
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// Return whether a provider rate-limit window is currently pausing workers.
    pub fn is_rate_limited(&self) -> bool {
        self.rate_limit_ticks_left > 0
    }

    /// Return current queued and running jobs.
    pub fn jobs(&self) -> &[QueueJob] {
        &self.jobs
    }

    /// Return cumulative completed and failed job totals.
    pub fn totals(&self) -> (u64, u64) {
        (self.done_total, self.failed_total)
    }

    /// Toggle queue pause state.
    pub fn set_paused(&mut self, paused: bool, cx: &mut Context<Self>) {
        if self.paused != paused {
            self.paused = paused;
            cx.emit(AgentQueueEvent::PausedChanged(paused));
            cx.notify();
        }
    }

    /// Trigger or clear a simulated provider `429 Rate limit` window (`10` ticks ~ `5.5s`).
    pub fn toggle_rate_limit(&mut self, cx: &mut Context<Self>) {
        if self.rate_limit_ticks_left > 0 {
            self.rate_limit_ticks_left = 0;
            cx.emit(AgentQueueEvent::RateLimitToggled(false));
        } else {
            self.rate_limit_ticks_left = 10;
            cx.emit(AgentQueueEvent::RateLimitToggled(true));
        }
        cx.notify();
    }

    /// Adjust active worker concurrency within `min_concurrency..=max_concurrency`.
    pub fn set_concurrency(&mut self, next: usize, cx: &mut Context<Self>) {
        let clamped = next
            .clamp(self.min_concurrency, self.max_concurrency)
            .min(self.workers_catalog.len().max(1));
        if self.concurrency != clamped {
            // Return excess running jobs gracefully to the front of Queued when scaling down
            let active_ids: Vec<SharedString> = self
                .workers_catalog
                .iter()
                .take(clamped)
                .map(|w| w.id.clone())
                .collect();
            for job in &mut self.jobs {
                if job.status == QueueJobStatus::Running
                    && job
                        .worker
                        .as_ref()
                        .is_some_and(|wid| !active_ids.contains(wid))
                {
                    job.status = QueueJobStatus::Queued;
                    job.worker = None;
                }
            }
            self.concurrency = clamped;
            self.assign_free_workers();
            cx.emit(AgentQueueEvent::ConcurrencyChanged(clamped));
            cx.notify();
        }
    }

    /// Enqueue a new job and immediately assign it if a worker slot is free.
    pub fn enqueue_job(&mut self, job: QueueJob, cx: &mut Context<Self>) {
        self.jobs.push(job.clone());
        self.sort_queued_jobs();
        if !self.paused && self.rate_limit_ticks_left == 0 {
            self.assign_free_workers();
        }
        cx.emit(AgentQueueEvent::JobEnqueued(job));
        cx.notify();
    }

    /// Promote a queued job to `QueuePriority::High` at the very front of the queue.
    pub fn promote_job(&mut self, job_id: &str, cx: &mut Context<Self>) -> bool {
        let Some(pos) = self
            .jobs
            .iter()
            .position(|j| j.id.as_ref() == job_id && j.status == QueueJobStatus::Queued)
        else {
            return false;
        };
        let mut job = self.jobs.remove(pos);
        job.priority = QueuePriority::High;
        let id_emit = job.id.clone();
        self.jobs.insert(0, job);
        cx.emit(AgentQueueEvent::JobPromoted(id_emit));
        cx.notify();
        true
    }

    /// Cancel and remove a queued or running job by id.
    pub fn cancel_job(&mut self, job_id: &str, cx: &mut Context<Self>) -> bool {
        let Some(pos) = self.jobs.iter().position(|j| j.id.as_ref() == job_id) else {
            return false;
        };
        let removed = self.jobs.remove(pos);
        if self.selected_job.as_deref() == Some(job_id) {
            self.last_inspected_job = Some(removed.clone());
            self.selected_job = None;
        }
        if !self.paused && self.rate_limit_ticks_left == 0 {
            self.assign_free_workers();
        }
        cx.emit(AgentQueueEvent::JobCancelled(removed.id));
        cx.notify();
        true
    }

    /// Advance one simulation tick (`~0.55s`), progressing worker bars, simulating rate-limit backlog build-up, and updating depth history.
    pub fn step_simulation(&mut self, cx: &mut Context<Self>) {
        self.applied_ticks += 1;
        let tick = self.applied_ticks;

        // Periodically trigger an automatic short rate limit at tick 14 if none was triggered manually
        if tick == 14 && self.rate_limit_ticks_left == 0 && !self.paused {
            self.rate_limit_ticks_left = 7;
        }

        let limited = self.rate_limit_ticks_left > 0;
        if limited && !self.paused {
            self.rate_limit_ticks_left = self.rate_limit_ticks_left.saturating_sub(1);
        }

        // Increment wait time on queued jobs
        for job in &mut self.jobs {
            if job.status == QueueJobStatus::Queued {
                job.elapsed_secs += 0.5;
            }
        }

        if !self.paused && !limited {
            // Advance running jobs
            let mut finished_indices = Vec::new();
            for (idx, job) in self.jobs.iter_mut().enumerate() {
                if job.status == QueueJobStatus::Running {
                    job.elapsed_secs += 0.5;
                    let delta = match job.kind {
                        QueueJobKind::Tool => 0.26,
                        QueueJobKind::Llm => 0.21,
                        QueueJobKind::Agent => 0.18,
                        QueueJobKind::Human => 0.24,
                    };
                    job.progress = (job.progress + delta).min(1.0);
                    if job.progress >= 0.999 {
                        finished_indices.push(idx);
                    }
                }
            }

            // Complete finished jobs from back to front
            for &idx in finished_indices.iter().rev() {
                let completed = self.jobs.remove(idx);
                if self.selected_job.as_ref() == Some(&completed.id) {
                    self.last_inspected_job = Some(completed.clone());
                    self.selected_job = None;
                }
                if tick.is_multiple_of(11) && completed.attempt == 1 {
                    // Re-queue once as a transient retry, or count a rare failure
                    let mut retried = completed;
                    retried.status = QueueJobStatus::Queued;
                    retried.worker = None;
                    retried.progress = 0.0;
                    retried.attempt = 2;
                    retried.error =
                        Some(SharedString::from("Upstream timeout after 2.0s; retrying"));
                    self.jobs.push(retried);
                } else if tick.is_multiple_of(19) {
                    self.failed_total += 1;
                } else {
                    self.done_total += 1;
                }
            }
        }

        // Spawn fresh incoming jobs (faster arrival during rate-limit spike so backlog visibly climbs!)
        let waiting_now = self
            .jobs
            .iter()
            .filter(|j| j.status == QueueJobStatus::Queued)
            .count();
        let should_spawn = if limited {
            waiting_now < 14
        } else {
            !self.paused && (waiting_now < 2 || tick.is_multiple_of(2)) && waiting_now < 10
        };
        if should_spawn {
            let count = if limited { 2 } else { 1 };
            for _ in 0..count {
                self.seq_counter += 1;
                let next_job = sample_incoming_job(self.seq_counter);
                self.jobs.push(next_job);
            }
            self.sort_queued_jobs();
        }

        if !self.paused && self.rate_limit_ticks_left == 0 {
            self.assign_free_workers();
        }

        let q_depth = self
            .jobs
            .iter()
            .filter(|j| j.status == QueueJobStatus::Queued)
            .count() as f32;

        self.throughput_per_min = if self.paused || limited {
            0
        } else {
            (18 * self.concurrency as u32 + (tick % 7) as u32).max(12)
        };
        self.wait_p95_secs = if limited {
            19 + self.rate_limit_ticks_left as u32
        } else {
            (8 + q_depth as u32 * 2).clamp(6, 28)
        };

        if self.depth_history.len() >= 48 {
            self.depth_history.remove(0);
        }
        self.depth_history.push((q_depth, limited));
        cx.notify();
    }

    fn sort_queued_jobs(&mut self) {
        // Stable partition so High priority jobs glide ahead of Normal/Low queued jobs
        let mut running = Vec::new();
        let mut high = Vec::new();
        let mut rest = Vec::new();
        for j in self.jobs.drain(..) {
            if j.status != QueueJobStatus::Queued {
                running.push(j);
            } else if j.priority == QueuePriority::High {
                high.push(j);
            } else {
                rest.push(j);
            }
        }
        self.jobs.extend(running);
        self.jobs.extend(high);
        self.jobs.extend(rest);
    }

    fn assign_free_workers(&mut self) {
        let active_workers: Vec<SharedString> = self
            .workers_catalog
            .iter()
            .take(self.concurrency)
            .map(|w| w.id.clone())
            .collect();

        for wid in active_workers {
            let occupied = self
                .jobs
                .iter()
                .any(|j| j.status == QueueJobStatus::Running && j.worker.as_ref() == Some(&wid));
            if !occupied
                && let Some(q_job) = self
                    .jobs
                    .iter_mut()
                    .find(|j| j.status == QueueJobStatus::Queued)
            {
                q_job.status = QueueJobStatus::Running;
                q_job.worker = Some(wid);
                q_job.progress = 0.08;
                q_job.elapsed_secs = 0.1;
            }
        }
    }
}

impl Render for AgentQueue {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let reduced = disabled_motion(cx) || Theme::global(cx).reduced_motion;
        let elapsed = if reduced {
            0.0
        } else {
            self.started_at.elapsed().as_secs_f32()
        };

        if self.live_flow && !reduced && !self.paused && !self.loading && !self.empty {
            let target_ticks = (elapsed / 0.55) as usize;
            while self.applied_ticks < target_ticks {
                self.step_simulation(cx);
            }
            w.request_animation_frame();
        } else if !reduced && self.loading {
            w.request_animation_frame();
        }

        let card_bg: Hsla = if p.dark {
            rgb(0x1c1c1f).into()
        } else {
            rgb(0xffffff).into()
        };
        let wash_bg: Hsla = if p.dark {
            hsla(0., 0., 1., 0.03)
        } else {
            hsla(0., 0., 0., 0.022)
        };

        let limited = self.rate_limit_ticks_left > 0;
        let (status_bg, status_fg, status_lbl) = if self.paused {
            (p.chip, p.muted, tr(cx, "Paused", "已暂停").to_string())
        } else if limited {
            let rem_s = ((self.rate_limit_ticks_left as f32 * 0.55).ceil() as usize).max(1);
            (
                p.amber_soft,
                p.amber_ink,
                format!("{} · {rem_s}s", tr(cx, "Rate limited", "触发限流")),
            )
        } else {
            (
                p.accent_soft,
                p.accent_info,
                tr(cx, "Live", "实时").to_string(),
            )
        };

        let header = div()
            .min_h(px(52.0))
            .px(px(16.0))
            .py(px(10.0))
            .border_b_1()
            .border_color(p.line)
            .flex()
            .flex_wrap()
            .items_center()
            .justify_between()
            .gap(px(10.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(pro_svg("0 0 24 24", QUEUE_MARK_SVG, 16.0, p.accent_info))
                    .child(
                        div()
                            .text_size(px(13.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .child(self.title.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(13.0))
                            .text_color(p.muted)
                            .child(self.queue_name.clone()),
                    )
                    .child(
                        div()
                            .h(px(20.0))
                            .px(px(7.0))
                            .rounded(px(5.0))
                            .bg(status_bg)
                            .flex()
                            .items_center()
                            .gap(px(5.0))
                            .child(div().size(px(6.0)).rounded_full().bg(status_fg))
                            .child(pro_swap_label(
                                pro_eid(&self.id, "q-status-lbl"),
                                status_lbl,
                                11.5,
                                FontWeight::MEDIUM,
                                status_fg,
                            )),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(7.0))
                    // Simulate Rate Limit toggle button
                    .child(
                        pro_tactile(
                            pro_eid(&self.id, "rate-limit-btn"),
                            tr(cx, "Simulate 429 rate limit", "模拟 429 速率限制"),
                            6.0,
                            if limited { p.amber_soft } else { card_bg },
                            p.hover,
                            if limited {
                                p.amber.opacity(0.35)
                            } else {
                                p.line_strong
                            },
                            p.line_strong,
                            limited,
                            div()
                                .h(px(28.0))
                                .px(px(9.0))
                                .flex()
                                .items_center()
                                .gap(px(5.0))
                                .child(pro_icon(
                                    ProIcon::Bolt,
                                    13.0,
                                    if limited { p.amber_ink } else { p.muted },
                                ))
                                .child(pro_swap_label(
                                    pro_eid(&self.id, "rl-btn-lbl"),
                                    if limited {
                                        tr(cx, "Clear limit", "解除限流")
                                    } else {
                                        tr(cx, "Rate limit", "模拟限流")
                                    },
                                    12.0,
                                    FontWeight::MEDIUM,
                                    if limited { p.amber_ink } else { p.ink },
                                )),
                            cx,
                            |this, _, cx| {
                                this.toggle_rate_limit(cx);
                            },
                        )
                        .w_auto(),
                    )
                    // Pause / Resume toggle
                    .child(
                        pro_tactile(
                            pro_eid(&self.id, "pause-queue-btn"),
                            tr(cx, "Pause the queue", "暂停任务队列"),
                            6.0,
                            card_bg,
                            p.hover,
                            p.line_strong,
                            p.line_strong,
                            self.paused,
                            div()
                                .h(px(28.0))
                                .px(px(9.0))
                                .flex()
                                .items_center()
                                .gap(px(5.0))
                                .child(pro_icon(
                                    if self.paused {
                                        ProIcon::Play
                                    } else {
                                        ProIcon::Pause
                                    },
                                    13.0,
                                    if self.paused { p.accent_info } else { p.muted },
                                ))
                                .child(pro_swap_label(
                                    pro_eid(&self.id, "pause-btn-lbl"),
                                    if self.paused {
                                        tr(cx, "Resume", "恢复")
                                    } else {
                                        tr(cx, "Pause", "暂停")
                                    },
                                    12.0,
                                    FontWeight::MEDIUM,
                                    if self.paused { p.accent_info } else { p.ink },
                                )),
                            cx,
                            |this, _, cx| {
                                let next = !this.paused;
                                this.set_paused(next, cx);
                            },
                        )
                        .w_auto(),
                    )
                    // Workers `- N +` concurrency stepper
                    .child(
                        div()
                            .id(pro_eid(&self.id, "workers-stepper"))
                            .role(Role::Group)
                            .aria_label(tr(cx, "Workers concurrency", "并发工作槽数量"))
                            .h(px(28.0))
                            .pl(px(8.0))
                            .pr(px(3.0))
                            .rounded(px(6.0))
                            .bg(card_bg)
                            .border_1()
                            .border_color(p.line_strong)
                            .flex()
                            .items_center()
                            .gap(px(3.0))
                            .child(
                                div()
                                    .text_size(px(11.5))
                                    .text_color(p.muted)
                                    .mr(px(2.0))
                                    .child(tr(cx, "Workers", "并发")),
                            )
                            .child(
                                pro_tactile(
                                    pro_eid(&self.id, "worker-dec"),
                                    tr(cx, "One worker less", "减少一个工作槽"),
                                    4.5,
                                    transparent_black(),
                                    p.hover,
                                    transparent_black(),
                                    transparent_black(),
                                    false,
                                    div()
                                        .size(px(22.0))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .child(pro_icon(ProIcon::Minus, 12.0, p.ink)),
                                    cx,
                                    |this, _, cx| {
                                        let next = this.concurrency.saturating_sub(1);
                                        this.set_concurrency(next, cx);
                                    },
                                )
                                .size(px(22.0))
                                .opacity(
                                    if self.concurrency <= self.min_concurrency {
                                        0.35
                                    } else {
                                        1.0
                                    },
                                ),
                            )
                            .child(div().min_w(px(18.0)).flex().justify_center().child(
                                pro_rolling_number(
                                    pro_eid(&self.id, "conc-num"),
                                    self.concurrency as f64,
                                    0,
                                    "",
                                    "",
                                    12.5,
                                    FontWeight::SEMIBOLD,
                                    p.strong,
                                ),
                            ))
                            .child(
                                pro_tactile(
                                    pro_eid(&self.id, "worker-inc"),
                                    tr(cx, "One worker more", "增加一个工作槽"),
                                    4.5,
                                    transparent_black(),
                                    p.hover,
                                    transparent_black(),
                                    transparent_black(),
                                    false,
                                    div()
                                        .size(px(22.0))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .child(pro_icon(ProIcon::Plus, 12.0, p.ink)),
                                    cx,
                                    |this, _, cx| {
                                        this.set_concurrency(this.concurrency + 1, cx);
                                    },
                                )
                                .size(px(22.0))
                                .opacity(
                                    if self.concurrency
                                        >= self.max_concurrency.min(self.workers_catalog.len())
                                    {
                                        0.35
                                    } else {
                                        1.0
                                    },
                                ),
                            ),
                    ),
            );

        if self.loading {
            let phase = (elapsed * 0.9).fract();
            return div()
                .id(self.id.clone())
                .w_full()
                .flex()
                .justify_center()
                .child(
                    div()
                        .w_full()
                        .max_w(px(760.0))
                        .rounded(px(12.0))
                        .bg(card_bg)
                        .border_1()
                        .border_color(p.line)
                        .overflow_hidden()
                        .flex()
                        .flex_col()
                        .child(header)
                        .child(
                            div()
                                .p(px(18.0))
                                .flex()
                                .flex_col()
                                .gap(px(12.0))
                                .child(pro_skeleton_bar(680.0, 44.0, 8.0, phase, cx))
                                .child(pro_skeleton_bar(680.0, 160.0, 8.0, phase + 0.2, cx))
                                .child(pro_skeleton_bar(680.0, 110.0, 8.0, phase + 0.4, cx)),
                        ),
                )
                .into_any_element();
        }

        if self.empty {
            return div()
                .id(self.id.clone())
                .w_full()
                .flex()
                .justify_center()
                .child(
                    div()
                        .w_full()
                        .max_w(px(760.0))
                        .rounded(px(12.0))
                        .bg(card_bg)
                        .border_1()
                        .border_color(p.line)
                        .overflow_hidden()
                        .flex()
                        .flex_col()
                        .child(header)
                        .child(pro_empty_state(
                            ProIcon::Tasks,
                            tr(cx, "Queue is idle", "任务队列为空"),
                            tr(
                                cx,
                                "Enqueue agent jobs to watch live worker saturation and backlog depth.",
                                "推入智能体任务即可观察并发工作槽负载与实时队列深度曲线。",
                            ),
                            Some((
                                pro_eid(&self.id, "seed-queue"),
                                SharedString::from(tr(cx, "Seed live workload", "载入实时任务流")),
                                Box::new(|this: &mut Self, _, cx| {
                                    this.empty = false;
                                    if this.jobs.is_empty() {
                                        this.jobs = default_jobs();
                                    }
                                    cx.notify();
                                }),
                            )),
                            cx,
                        )),
                )
                .into_any_element();
        }

        let queued_jobs: Vec<QueueJob> = self
            .jobs
            .iter()
            .filter(|j| j.status == QueueJobStatus::Queued)
            .cloned()
            .collect();
        let running_count = self
            .jobs
            .iter()
            .filter(|j| j.status == QueueJobStatus::Running)
            .count();
        let waiting_count = queued_jobs.len() as u32;
        let over_bp = waiting_count >= self.backpressure;
        let util_ratio = (running_count as f32 / self.concurrency.max(1) as f32).clamp(0.0, 1.0);

        // 4-Column KPI Strip
        let stats_strip = div()
            .py(px(11.0))
            .border_b_1()
            .border_color(p.line)
            .flex()
            .items_center()
            .child(self.render_kpi_cell(
                "kpi-wait",
                tr(cx, "WAITING", "排队等待"),
                waiting_count as f64,
                None,
                if over_bp { p.amber_ink } else { p.strong },
                false,
                None,
                &p,
            ))
            .child(self.render_kpi_cell(
                "kpi-run",
                tr(cx, "RUNNING", "正在执行"),
                running_count as f64,
                Some(format!("/ {}", self.concurrency)),
                p.strong,
                true,
                Some((util_ratio, if limited { p.amber } else { p.accent_info })),
                &p,
            ))
            .child(self.render_kpi_cell(
                "kpi-tput",
                tr(cx, "THROUGHPUT", "吞吐速率"),
                self.throughput_per_min as f64,
                Some(tr(cx, "/min", "/分钟").to_string()),
                p.strong,
                true,
                None,
                &p,
            ))
            .child(self.render_kpi_cell(
                "kpi-p95",
                tr(cx, "WAIT P95", "P95 等待"),
                self.wait_p95_secs as f64,
                Some("s".to_string()),
                if limited { p.amber_ink } else { p.strong },
                true,
                None,
                &p,
            ));

        // 3-Stage Flow Area: status pill filter + QUEUE -> WORKERS -> DONE / FAILED
        let flow_area = self.render_flow_lanes(
            &queued_jobs,
            running_count,
            util_ratio,
            limited,
            card_bg,
            wash_bg,
            w,
            &p,
            cx,
        );

        // Smooth pro_fold Job Inspector Disclosure
        let inspector = self.render_job_inspector(&p, cx);

        // Bottom Queue Depth Area Chart (`PathBuilder`)
        let chart_area = self.render_depth_chart(waiting_count, card_bg, &p, cx);

        div()
            .id(self.id.clone())
            .w_full()
            .flex()
            .justify_center()
            .child(
                div()
                    .w_full()
                    .max_w(px(760.0))
                    .rounded(px(12.0))
                    .bg(card_bg)
                    .border_1()
                    .border_color(p.line)
                    .when(!p.dark, |d| d.shadow_sm())
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .child(header)
                    .child(stats_strip)
                    .child(flow_area)
                    .child(inspector)
                    .child(chart_area),
            )
            .into_any_element()
    }
}

impl AgentQueue {
    #[allow(clippy::too_many_arguments)]
    fn render_kpi_cell(
        &self,
        key: &str,
        label: &str,
        val: f64,
        suffix_muted: Option<String>,
        val_color: Hsla,
        left_border: bool,
        util_bar: Option<(f32, Hsla)>,
        p: &ProPalette,
    ) -> Div {
        div()
            .flex_1()
            .min_w_0()
            .px(px(16.0))
            .when(left_border, |d| d.border_l_1().border_color(p.line))
            .flex()
            .flex_col()
            .gap(px(3.0))
            .child(
                div()
                    .truncate()
                    .text_size(px(10.5))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(p.muted)
                    .child(label.to_string()),
            )
            .child(
                div()
                    .flex()
                    .items_baseline()
                    .gap(px(3.0))
                    .child(pro_rolling_number(
                        pro_eid(&self.id, key),
                        val,
                        0,
                        "",
                        "",
                        16.0,
                        FontWeight::MEDIUM,
                        val_color,
                    ))
                    .when_some(suffix_muted, |row, sfx| {
                        row.child(div().text_size(px(12.5)).text_color(p.muted).child(sfx))
                    }),
            )
            .when_some(util_bar, |d, (ratio, fill_c)| {
                d.child(pro_spring_bar(
                    pro_eid(&self.id, format!("{key}-bar")),
                    ratio,
                    3.5,
                    2.0,
                    p.chip,
                    fill_c,
                ))
            })
    }

    #[allow(clippy::too_many_arguments)]
    fn render_flow_lanes(
        &self,
        queued_jobs: &[QueueJob],
        running_count: usize,
        util_ratio: f32,
        limited: bool,
        card_bg: Hsla,
        wash_bg: Hsla,
        w: &mut Window,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> Div {
        let active_lane = self.lane_filter.as_ref();
        let lane_sensor = pro_selection_sensor();
        let pill_fill: Hsla = if p.dark {
            hsla(0., 0., 1., 0.11)
        } else {
            rgb(0xffffff).into()
        };
        let needs_you_count = queued_jobs
            .iter()
            .filter(|j| j.kind == QueueJobKind::Human || j.priority == QueuePriority::High)
            .count();

        let tabs: [(&'static str, String); 4] = [
            ("all", tr(cx, "All lanes", "全部泳道").to_string()),
            (
                "needs",
                format!("{} · {needs_you_count}", tr(cx, "Needs you", "待关注")),
            ),
            (
                "running",
                format!("{} · {running_count}", tr(cx, "Running", "运行中")),
            ),
            (
                "done",
                format!("{} · {}", tr(cx, "Done", "已完成"), self.done_total),
            ),
        ];

        let mut filter_strip = div()
            .id(pro_eid(&self.id, "lane-tabs"))
            .role(Role::TabList)
            .aria_label(tr(cx, "Queue status lanes", "队列状态筛选"))
            .relative()
            .h(px(28.0))
            .p(px(2.5))
            .rounded(px(7.5))
            .bg(wash_bg)
            .border_1()
            .border_color(p.line)
            .flex()
            .items_center()
            .gap(px(2.0))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "lane-pill"),
                self.lane_filter.clone(),
                lane_sensor.clone(),
                5.5,
                pill_fill,
                Some(p.line_strong),
                !p.dark,
            ));

        for (key, lbl) in tabs {
            let is_sel = active_lane == key;
            let key_ss = SharedString::from(key);
            filter_strip = filter_strip.child(
                div()
                    .id(pro_eid(&self.id, format!("lane-tab-{key}")))
                    .role(Role::Tab)
                    .aria_selected(is_sel)
                    .tab_index(0)
                    .relative()
                    .h(px(22.0))
                    .px(px(9.0))
                    .rounded(px(5.5))
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .text_size(px(11.0))
                    .font_weight(if is_sel {
                        FontWeight::SEMIBOLD
                    } else {
                        FontWeight::MEDIUM
                    })
                    .text_color(if is_sel { p.strong } else { p.muted })
                    .hover(|s| s.text_color(p.strong))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.lane_filter = key_ss.clone();
                        cx.notify();
                    }))
                    .when(is_sel, |d| d.child(pro_mark_selected(&lane_sensor)))
                    .child(lbl),
            );
        }

        let q_target = if active_lane == "all" || active_lane == "needs" {
            1.0
        } else {
            0.46
        };
        let w_target = if active_lane == "all" || active_lane == "running" {
            1.0
        } else {
            0.46
        };
        let b_target = if active_lane == "all" || active_lane == "done" {
            1.0
        } else {
            0.46
        };
        let q_alpha = pro_spring_f32(pro_eid(&self.id, "q-lane-a"), q_target, 0.24, w, cx);
        let w_alpha = pro_spring_f32(pro_eid(&self.id, "w-lane-a"), w_target, 0.24, w, cx);
        let b_alpha = pro_spring_f32(pro_eid(&self.id, "b-lane-a"), b_target, 0.24, w, cx);

        let visible_queued: Vec<&QueueJob> = queued_jobs
            .iter()
            .filter(|j| {
                if active_lane == "needs" {
                    j.kind == QueueJobKind::Human || j.priority == QueuePriority::High
                } else {
                    true
                }
            })
            .collect();

        // Left column: QUEUE
        let mut queue_col = div()
            .w(px(214.0))
            .flex_none()
            .opacity(q_alpha)
            .flex()
            .flex_col()
            .gap(px(7.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .child(
                                div()
                                    .text_size(px(10.5))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(p.muted)
                                    .child(tr(cx, "QUEUE", "待处理队列")),
                            )
                            .child(pro_rolling_number(
                                pro_eid(&self.id, "q-col-cnt"),
                                visible_queued.len() as f64,
                                0,
                                "",
                                "",
                                12.0,
                                FontWeight::MEDIUM,
                                p.ink,
                            )),
                    )
                    .child(
                        pro_tactile(
                            pro_eid(&self.id, "enqueue-high-btn"),
                            tr(cx, "Enqueue urgent job", "加入高优任务"),
                            4.5,
                            p.chip,
                            p.chip_hover,
                            transparent_black(),
                            p.line_strong,
                            false,
                            div()
                                .h(px(20.0))
                                .px(px(6.0))
                                .flex()
                                .items_center()
                                .gap(px(3.0))
                                .text_size(px(10.5))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.ink)
                                .child(pro_icon(ProIcon::Plus, 10.5, p.muted))
                                .child(tr(cx, "Urgent", "高优")),
                            cx,
                            |this, _, cx| {
                                this.seq_counter += 1;
                                let n = this.seq_counter;
                                let urgent = QueueJob::new(
                                    format!("urgent-{n}"),
                                    format!("VIP escalation #{n}"),
                                    QueueJobKind::Llm,
                                )
                                .with_name_zh(format!("VIP 紧急工单 #{n}"))
                                .with_priority(QueuePriority::High);
                                this.enqueue_job(urgent, cx);
                            },
                        )
                        .w_auto(),
                    ),
            );

        let mut queue_stack = div()
            .min_h(px(148.0))
            .p(px(6.0))
            .rounded(px(8.0))
            .bg(wash_bg)
            .border_1()
            .border_color(p.line)
            .flex()
            .flex_col()
            .gap(px(5.0));

        if visible_queued.is_empty() {
            queue_stack = queue_stack.justify_center().items_center().child(
                div().text_size(px(11.5)).text_color(p.faint).child(tr(
                    cx,
                    "No jobs waiting",
                    "无积压任务",
                )),
            );
        } else {
            for qj in visible_queued.iter().take(4) {
                queue_stack =
                    queue_stack.child(self.render_job_chip(qj, false, limited, card_bg, w, p, cx));
            }
            if visible_queued.len() > 4 {
                let extra = visible_queued.len() - 4;
                queue_stack = queue_stack.child(
                    div()
                        .h(px(22.0))
                        .px(px(8.0))
                        .rounded(px(5.0))
                        .bg(p.chip)
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(11.0))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.muted)
                        .child(format!("+{extra} {}", tr(cx, "more waiting", "项排队中"))),
                );
            }
        }
        queue_col = queue_col.child(queue_stack);

        // Center column: WORKERS
        let mut workers_col = div()
            .flex_1()
            .min_w_0()
            .opacity(w_alpha)
            .flex()
            .flex_col()
            .gap(px(7.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(8.0))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .child(
                                div()
                                    .text_size(px(10.5))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(p.muted)
                                    .child(tr(cx, "WORKERS", "并发工作槽")),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_baseline()
                                    .gap(px(2.0))
                                    .child(pro_rolling_number(
                                        pro_eid(&self.id, "w-col-run"),
                                        running_count as f64,
                                        0,
                                        "",
                                        "",
                                        12.0,
                                        FontWeight::MEDIUM,
                                        p.ink,
                                    ))
                                    .child(
                                        div()
                                            .text_size(px(12.0))
                                            .text_color(p.muted)
                                            .child(format!("/ {}", self.concurrency)),
                                    ),
                            ),
                    )
                    .child(div().w(px(72.0)).flex_none().child(pro_spring_bar(
                        pro_eid(&self.id, "workers-util-bar"),
                        util_ratio,
                        4.5,
                        2.5,
                        p.chip,
                        if limited { p.amber } else { p.accent_info },
                    ))),
            );

        let mut slots_stack = div().flex().flex_col().gap(px(6.0));
        let active_workers: Vec<QueueWorker> = self
            .workers_catalog
            .iter()
            .take(self.concurrency)
            .cloned()
            .collect();

        for wk in active_workers {
            let wid = wk.id.clone();
            let mark = match &wk.provider {
                Some(prov) => pro_app_tile(prov.as_ref(), 22.0, cx),
                None => pro_agent_avatar(wid.as_ref(), p.accent_info, 22.0, !self.paused, 0.0, cx),
            };
            let running_job = self
                .jobs
                .iter()
                .find(|j| j.status == QueueJobStatus::Running && j.worker.as_ref() == Some(&wid))
                .cloned();

            let slot_right = match running_job {
                Some(rj) => self.render_job_chip(&rj, true, limited, card_bg, w, p, cx),
                None => pro_tactile(
                    pro_eid(&self.id, format!("idle-slot-{wid}")),
                    tr(cx, "Idle worker slot", "空闲工作槽"),
                    6.0,
                    transparent_black(),
                    p.hover,
                    p.line_soft,
                    p.line_strong,
                    false,
                    div()
                        .w_full()
                        .h(px(28.0))
                        .px(px(10.0))
                        .flex()
                        .items_center()
                        .justify_between()
                        .text_size(px(11.5))
                        .text_color(p.faint)
                        .child(tr(cx, "Idle slot", "空闲槽位")),
                    cx,
                    |_, _, _| {},
                )
                .into_any_element(),
            };

            slots_stack = slots_stack.child(
                div()
                    .h(px(36.0))
                    .px(px(7.0))
                    .rounded(px(8.0))
                    .bg(wash_bg)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        div()
                            .w(px(114.0))
                            .flex_none()
                            .flex()
                            .items_center()
                            .gap(px(6.5))
                            .child(mark)
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .text_size(px(12.0))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.strong)
                                    .child(wk.name),
                            ),
                    )
                    .child(div().flex_1().min_w_0().child(slot_right)),
            );
        }
        workers_col = workers_col.child(slots_stack);

        // Right column: DONE & FAILED bins
        let bins_col = div()
            .w(px(108.0))
            .flex_none()
            .opacity(b_alpha)
            .pt(px(23.0))
            .flex()
            .flex_col()
            .gap(px(8.0))
            .child(self.render_terminal_bin(
                "bin-done",
                true,
                tr(cx, "DONE", "已完成"),
                self.done_total,
                p.green_soft,
                p.green_ink,
                p.strong,
                wash_bg,
                p.line,
                cx,
            ))
            .child(self.render_terminal_bin(
                "bin-failed",
                false,
                tr(cx, "FAILED", "已失败"),
                self.failed_total,
                p.red_soft,
                p.red_ink,
                p.red_ink,
                wash_bg,
                p.line,
                cx,
            ));

        div()
            .p(px(16.0))
            .border_b_1()
            .border_color(p.line)
            .flex()
            .flex_col()
            .gap(px(12.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(filter_strip),
            )
            .child(
                div()
                    .flex()
                    .items_start()
                    .gap(px(12.0))
                    .child(queue_col)
                    .child(workers_col)
                    .child(bins_col),
            )
    }

    #[allow(clippy::too_many_arguments)]
    fn render_terminal_bin(
        &self,
        key: &str,
        is_done_bin: bool,
        label: &str,
        count: u64,
        icon_bg: Hsla,
        icon_fg: Hsla,
        num_fg: Hsla,
        wash_bg: Hsla,
        border_c: Hsla,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let lead_badge: AnyElement = if is_done_bin {
            pro_check_draw(
                pro_eid(&self.id, format!("{key}-chk")),
                count > 0,
                16.0,
                true,
                icon_fg,
            )
        } else {
            div()
                .size(px(20.0))
                .rounded(px(5.0))
                .bg(icon_bg)
                .flex()
                .items_center()
                .justify_center()
                .child(pro_icon(ProIcon::Close, 11.5, icon_fg))
                .into_any_element()
        };

        pro_tactile(
            pro_eid(&self.id, format!("{key}-card")),
            label.to_string(),
            8.0,
            wash_bg,
            ProPalette::get(cx).hover,
            border_c,
            icon_fg.opacity(0.45),
            is_done_bin && self.lane_filter.as_ref() == "done",
            div()
                .h(px(70.0))
                .p(px(10.0))
                .flex()
                .flex_col()
                .justify_between()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .child(lead_badge)
                        .child(
                            div()
                                .text_size(px(10.5))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(icon_fg)
                                .child(label.to_string()),
                        ),
                )
                .child(pro_rolling_number(
                    pro_eid(&self.id, key),
                    count as f64,
                    0,
                    "",
                    "",
                    19.0,
                    FontWeight::MEDIUM,
                    num_fg,
                )),
            cx,
            move |this, _, cx| {
                if is_done_bin {
                    this.lane_filter = if this.lane_filter.as_ref() == "done" {
                        SharedString::from("all")
                    } else {
                        SharedString::from("done")
                    };
                    cx.notify();
                }
            },
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn render_job_chip(
        &self,
        job: &QueueJob,
        in_worker: bool,
        limited: bool,
        card_bg: Hsla,
        w: &mut Window,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let jid = job.id.clone();
        let job_snapshot = job.clone();
        let selected = self.selected_job.as_ref() == Some(&jid);
        let (k_bg, k_edge, k_ink) = if in_worker && limited {
            (p.amber_soft, p.amber.opacity(0.42), p.amber_ink)
        } else {
            job.kind.colors(p)
        };

        let prog = if in_worker {
            pro_spring_f32(
                pro_eid(&self.id, format!("jp-{}", job.id)),
                job.progress.clamp(0.0, 1.0),
                0.32,
                w,
                cx,
            )
            .clamp(0.0, 1.0)
        } else {
            1.0
        };

        let lead_icon = match &job.app {
            Some(app) => pro_app_tile(app.as_ref(), 18.0, cx),
            None => div()
                .size(px(18.0))
                .flex_none()
                .rounded(px(4.5))
                .bg(k_bg)
                .flex()
                .items_center()
                .justify_center()
                .child(pro_icon(job.kind.fallback_icon(), 11.0, k_ink))
                .into_any_element(),
        };

        let meta_txt = if in_worker {
            format!("{}%", (prog * 100.0).round() as u32)
        } else {
            format!("{:.1}s", job.elapsed_secs)
        };
        let title_str = job.display_name(cx).to_string();
        let aria = format!("{title_str}, {}, {meta_txt}", job.kind.label(cx));

        let inner = div()
            .w_full()
            .h(px(28.0))
            .relative()
            .overflow_hidden()
            .when(in_worker, |d| {
                d.child(
                    div()
                        .absolute()
                        .left_0()
                        .top_0()
                        .bottom_0()
                        .w(relative(prog))
                        .bg(k_bg),
                )
            })
            .child(
                div()
                    .relative()
                    .size_full()
                    .px(px(6.5))
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .child(lead_icon)
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_size(px(11.5))
                            .text_color(p.ink)
                            .child(title_str),
                    )
                    .when(job.priority == QueuePriority::High, |row| {
                        row.child(
                            div()
                                .size(px(15.0))
                                .flex_none()
                                .rounded(px(4.0))
                                .bg(p.amber_soft)
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(pro_icon(ProIcon::ArrowUp, 9.5, p.amber_ink)),
                        )
                    })
                    .when(job.attempt > 1, |row| {
                        row.child(
                            div()
                                .h(px(15.0))
                                .px(px(4.0))
                                .flex_none()
                                .rounded(px(4.0))
                                .bg(p.red_soft)
                                .flex()
                                .items_center()
                                .text_size(px(9.5))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(p.red_ink)
                                .child(format!("↻{}", job.attempt)),
                        )
                    })
                    .child(div().flex_none().child(if in_worker {
                        pro_rolling_number(
                            pro_eid(&self.id, format!("jm-{}", job.id)),
                            (prog * 100.0).round() as f64,
                            0,
                            "",
                            "%",
                            11.0,
                            FontWeight::MEDIUM,
                            k_ink,
                        )
                    } else {
                        pro_rolling_number(
                            pro_eid(&self.id, format!("jm-{}", job.id)),
                            job.elapsed_secs as f64,
                            1,
                            "",
                            "s",
                            11.0,
                            FontWeight::MEDIUM,
                            p.muted,
                        )
                    })),
            );

        pro_tactile(
            pro_eid(&self.id, format!("job-{jid}")),
            aria,
            6.0,
            card_bg,
            p.hover,
            if selected { k_ink } else { k_edge },
            k_ink,
            selected,
            inner,
            cx,
            move |this, _, cx| {
                this.last_inspected_job = Some(job_snapshot.clone());
                if this.selected_job.as_ref() == Some(&jid) {
                    this.selected_job = None;
                } else {
                    this.selected_job = Some(jid.clone());
                }
                cx.emit(AgentQueueEvent::JobSelected(this.selected_job.clone()));
                cx.notify();
            },
        )
        .aria_expanded(selected)
        .into_any_element()
    }

    fn render_job_inspector(&self, p: &ProPalette, cx: &mut Context<Self>) -> AnyElement {
        let active_job = self
            .selected_job
            .as_ref()
            .and_then(|sel_id| self.jobs.iter().find(|j| &j.id == sel_id))
            .cloned();
        let is_open = active_job.is_some();
        let display_job = active_job.or_else(|| self.last_inspected_job.clone());
        let base_id = self.id.clone();

        let Some(job) = display_job else {
            return pro_fold(
                pro_eid(&base_id, "job-inspector-fold"),
                false,
                56.0,
                0.0,
                || div().into_any_element(),
            );
        };

        let (k_bg, _, k_ink) = job.kind.colors(p);
        let jid_prom = job.id.clone();
        let jid_canc = job.id.clone();
        let is_queued = job.status == QueueJobStatus::Queued;
        let is_running = job.status == QueueJobStatus::Running;
        let can_promote = is_queued && job.priority != QueuePriority::High;
        let title_s = job.display_name(cx).to_string();
        let subtitle_s = format!(
            "{} · {} #{}",
            job.kind.label(cx),
            tr(cx, "Attempt", "尝试"),
            job.attempt
        );
        let err_opt = job.error.clone();
        let prog_val = job.progress.clamp(0.0, 1.0);
        let p_copy = *p;

        let actions = div()
            .flex()
            .items_center()
            .gap(px(6.0))
            .when(can_promote, |d| {
                d.child(pro_btn(
                    pro_eid(&self.id, "promote-job-btn"),
                    tr(cx, "Move to front", "移至队首"),
                    Some(ProIcon::ArrowUp),
                    ProBtnStyle::Secondary,
                    true,
                    cx,
                    move |this, _, cx| {
                        this.promote_job(jid_prom.as_ref(), cx);
                    },
                ))
            })
            .child(pro_btn(
                pro_eid(&self.id, "cancel-job-btn"),
                tr(cx, "Cancel job", "取消任务"),
                Some(ProIcon::Close),
                ProBtnStyle::Danger,
                true,
                cx,
                move |this, _, cx| {
                    this.cancel_job(jid_canc.as_ref(), cx);
                },
            ))
            .child(pro_icon_btn(
                pro_eid(&self.id, "close-inspector-btn"),
                tr(cx, "Close job card", "关闭任务详情"),
                ProIcon::Close,
                26.0,
                false,
                cx,
                |this, _, cx| {
                    if let Some(sel) = &this.selected_job
                        && let Some(cur) = this.jobs.iter().find(|j| &j.id == sel).cloned()
                    {
                        this.last_inspected_job = Some(cur);
                    }
                    this.selected_job = None;
                    cx.notify();
                },
            ));

        let card = div()
            .px(px(16.0))
            .py(px(10.0))
            .border_b_1()
            .border_color(p_copy.line)
            .bg(p_copy.subtle)
            .flex()
            .flex_col()
            .gap(px(8.0))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .justify_between()
                    .gap(px(10.0))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .size(px(24.0))
                                    .rounded(px(6.0))
                                    .bg(k_bg)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(pro_icon(job.kind.fallback_icon(), 13.0, k_ink)),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .text_size(px(12.5))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(p_copy.strong)
                                            .child(title_s),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.5))
                                            .text_color(p_copy.muted)
                                            .child(subtitle_s),
                                    ),
                            )
                            .when_some(err_opt, |row, err| {
                                row.child(
                                    div()
                                        .px(px(7.5))
                                        .py(px(3.0))
                                        .rounded(px(5.0))
                                        .bg(p_copy.red_soft)
                                        .text_size(px(11.0))
                                        .text_color(p_copy.red_ink)
                                        .child(err),
                                )
                            }),
                    )
                    .child(actions),
            )
            .when(is_running, |d| {
                d.child(pro_spring_bar(
                    pro_eid(&base_id, format!("insp-prog-{}", job.id)),
                    prog_val,
                    4.0,
                    2.0,
                    p_copy.chip,
                    k_ink,
                ))
            })
            .into_any_element();

        let mut cell = Some(card);
        pro_fold(
            pro_eid(&base_id, "job-inspector-fold"),
            is_open,
            64.0,
            0.0,
            move || cell.take().unwrap_or_else(|| div().into_any_element()),
        )
    }

    fn render_depth_chart(&self, waiting_now: u32, card_bg: Hsla, p: &ProPalette, cx: &App) -> Div {
        let samples = self.depth_history.clone();
        let peak = samples
            .iter()
            .map(|(v, _)| *v)
            .fold(0.0f32, f32::max)
            .max(waiting_now as f32);
        let y_max = ((peak.max(self.backpressure as f32 + 4.0) / 4.0).ceil() * 4.0).max(16.0);
        let bp_val = self.backpressure as f32;
        let bp_ratio = (bp_val / y_max).clamp(0.08, 0.92);

        let depth_stroke = p.purple;
        let warn_band = p.amber.opacity(if p.dark { 0.11 } else { 0.09 });
        let grid_line = p.line;
        let bp_line = p.faint;

        let plot_canvas = canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let bw = f32::from(bounds.size.width);
                let bh = f32::from(bounds.size.height);
                if bw <= 6.0 || bh <= 6.0 || samples.len() < 2 {
                    return;
                }

                // Top & bottom baseline grid rules
                window.paint_quad(fill(
                    Bounds::new(bounds.origin, size(bounds.size.width, px(1.0))),
                    grid_line,
                ));
                window.paint_quad(fill(
                    Bounds::new(
                        point(bounds.left(), bounds.bottom() - px(1.0)),
                        size(bounds.size.width, px(1.0)),
                    ),
                    grid_line,
                ));

                let n = samples.len();
                let step_w = bw / (n - 1) as f32;

                // Amber rate-limit bands
                let mut band_start: Option<usize> = None;
                for (i, &(_, lim)) in samples.iter().enumerate() {
                    if lim && band_start.is_none() {
                        band_start = Some(i);
                    } else if !lim && let Some(s_idx) = band_start.take() {
                        let x0 = s_idx as f32 * step_w;
                        let x1 = i as f32 * step_w;
                        window.paint_quad(fill(
                            Bounds::new(
                                point(bounds.left() + px(x0), bounds.top()),
                                size(px((x1 - x0).max(3.0)), bounds.size.height),
                            ),
                            warn_band,
                        ));
                    }
                }
                if let Some(s_idx) = band_start {
                    let x0 = s_idx as f32 * step_w;
                    window.paint_quad(fill(
                        Bounds::new(
                            point(bounds.left() + px(x0), bounds.top()),
                            size(px((bw - x0).max(3.0)), bounds.size.height),
                        ),
                        warn_band,
                    ));
                }

                // Dashed horizontal backpressure line
                let bp_y = bounds.bottom() - px(bp_ratio * bh);
                let dash = 4.0f32;
                let gap = 4.0f32;
                let mut x_cur = 0.0f32;
                let mut bp_builder = PathBuilder::stroke(px(1.0));
                let mut any_dash = false;
                while x_cur < bw {
                    let x_end = (x_cur + dash).min(bw);
                    bp_builder.move_to(point(bounds.left() + px(x_cur), bp_y));
                    bp_builder.line_to(point(bounds.left() + px(x_end), bp_y));
                    any_dash = true;
                    x_cur += dash + gap;
                }
                if any_dash && let Ok(path) = bp_builder.build() {
                    window.paint_path(path, bp_line);
                }

                let map_pt = |idx: usize, val: f32| -> Point<Pixels> {
                    let tx = idx as f32 / (n - 1) as f32;
                    let ty = 1.0 - (val / y_max).clamp(0.0, 1.0);
                    point(
                        bounds.left() + px(tx * bw),
                        bounds.top() + px(3.0 + ty * (bh - 6.0)),
                    )
                };

                // Area gradient fill
                let mut area = PathBuilder::fill();
                let first_pt = map_pt(0, samples[0].0);
                let last_pt = map_pt(n - 1, samples[n - 1].0);
                area.move_to(point(first_pt.x, bounds.bottom()));
                area.line_to(first_pt);
                for (i, &(v, _)) in samples.iter().enumerate().skip(1) {
                    area.line_to(map_pt(i, v));
                }
                area.line_to(point(last_pt.x, bounds.bottom()));
                area.close();
                if let Ok(path) = area.build() {
                    window.paint_path(
                        path,
                        linear_gradient(
                            180.0,
                            linear_color_stop(depth_stroke.opacity(0.24), 0.0),
                            linear_color_stop(depth_stroke.opacity(0.0), 1.0),
                        ),
                    );
                }

                // Crisp depth polyline
                let mut line = PathBuilder::stroke(px(1.75));
                for (i, &(v, _)) in samples.iter().enumerate() {
                    let pt = map_pt(i, v);
                    if i == 0 {
                        line.move_to(pt);
                    } else {
                        line.line_to(pt);
                    }
                }
                if let Ok(path) = line.build() {
                    window.paint_path(path, depth_stroke);
                }

                // Live tip dot
                let dot_rect = Bounds::new(
                    point(last_pt.x - px(3.5), last_pt.y - px(3.5)),
                    size(px(7.0), px(7.0)),
                );
                window.paint_quad(quad(
                    dot_rect,
                    Corners::all(px(99.0)),
                    depth_stroke,
                    Edges::all(px(1.5)),
                    card_bg,
                    BorderStyle::Solid,
                ));
            },
        )
        .w_full()
        .h(px(108.0));

        let bp_bottom_px = (bp_ratio * 108.0 - 8.0).clamp(14.0, 88.0);
        let aria_summary = format!(
            "{}: {waiting_now} waiting, peak {}, backpressure {}",
            tr(cx, "Queue depth", "队列积压深度"),
            peak.round() as u32,
            self.backpressure
        );

        div()
            .px(px(16.0))
            .py(px(12.0))
            .flex()
            .flex_col()
            .gap(px(8.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(10.5))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(p.muted)
                            .child(tr(cx, "QUEUE DEPTH", "队列积压深度")),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(12.0))
                            .text_size(px(11.0))
                            .text_color(p.muted)
                            .child(
                                div()
                                    .flex()
                                    .items_baseline()
                                    .gap(px(3.5))
                                    .child(tr(cx, "Peak", "峰值"))
                                    .child(pro_rolling_number(
                                        pro_eid(&self.id, "depth-peak-num"),
                                        peak.round() as f64,
                                        0,
                                        "",
                                        "",
                                        11.0,
                                        FontWeight::MEDIUM,
                                        p.ink,
                                    )),
                            )
                            .child(format!(
                                "{} {}",
                                tr(cx, "Backpressure", "背压阈值"),
                                self.backpressure
                            )),
                    ),
            )
            .child(
                div()
                    .id(pro_eid(&self.id, "depth-plot"))
                    .role(Role::Image)
                    .aria_label(aria_summary)
                    .w_full()
                    .flex()
                    .items_stretch()
                    .gap(px(8.0))
                    .child(
                        div()
                            .w(px(22.0))
                            .h(px(108.0))
                            .relative()
                            .flex_none()
                            .text_size(px(11.0))
                            .text_color(p.muted)
                            .child(
                                div()
                                    .absolute()
                                    .top(px(-4.0))
                                    .right_0()
                                    .child(format!("{}", y_max as u32)),
                            )
                            .child(
                                div()
                                    .absolute()
                                    .bottom(px(bp_bottom_px))
                                    .right_0()
                                    .text_color(p.amber_ink)
                                    .child(self.backpressure.to_string()),
                            )
                            .child(div().absolute().bottom(px(-4.0)).right_0().child("0")),
                    )
                    .child(div().flex_1().min_w_0().child(plot_canvas)),
            )
            .child(
                div()
                    .pl(px(30.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_size(px(11.5))
                    .text_color(p.muted)
                    .child(tr(cx, "96s ago", "96秒前"))
                    .child(tr(cx, "now", "现在")),
            )
    }
}

fn default_workers() -> Vec<QueueWorker> {
    vec![
        QueueWorker::new("w1", "Sonnet 5.5").with_provider("claude"),
        QueueWorker::new("w2", "GPT-6 Sol").with_provider("openai"),
        QueueWorker::new("w3", "Gemini 3.8 Flash").with_provider("gemini"),
        QueueWorker::new("w4", "Haiku 5").with_provider("claude"),
        QueueWorker::new("w5", "Codex Mini").with_provider("openai"),
        QueueWorker::new("w6", "Flash Lite").with_provider("gemini"),
    ]
}

fn default_jobs() -> Vec<QueueJob> {
    vec![
        QueueJob::new("j1", "Embed 240 docs · 4/5", QueueJobKind::Tool)
            .with_name_zh("向量化 240 篇文档 · 4/5")
            .with_app("notion")
            .with_running_on("w1", 0.78)
            .with_elapsed_secs(1.3),
        QueueJob::new("j2", "Draft reply – Acme VIP", QueueJobKind::Llm)
            .with_name_zh("起草回复 – Acme VIP")
            .with_priority(QueuePriority::High)
            .with_running_on("w2", 0.42)
            .with_elapsed_secs(0.6),
        QueueJob::new("j3", "Update roadmap page", QueueJobKind::Tool)
            .with_name_zh("同步产品路线图页面")
            .with_app("linear")
            .with_running_on("w3", 0.31)
            .with_elapsed_secs(0.4),
        QueueJob::new("j4", "Triage release alerts", QueueJobKind::Agent)
            .with_name_zh("分流线上发布告警")
            .with_app("slack")
            .with_running_on("w4", 0.64)
            .with_elapsed_secs(1.1),
        QueueJob::new("j5", "Open PR #500", QueueJobKind::Tool)
            .with_name_zh("创建合并请求 PR #500")
            .with_app("github")
            .with_elapsed_secs(1.6),
        QueueJob::new("j6", "Score lead – Tailspin", QueueJobKind::Llm)
            .with_name_zh("线索评分 – Tailspin")
            .with_elapsed_secs(1.1),
    ]
}

fn default_depth_history() -> Vec<(f32, bool)> {
    vec![
        (2.0, false),
        (3.0, false),
        (2.0, false),
        (4.0, false),
        (3.0, false),
        (5.0, true),
        (8.0, true),
        (11.0, true),
        (14.0, true),
        (12.0, false),
        (9.0, false),
        (6.0, false),
        (4.0, false),
        (3.0, false),
        (2.0, false),
        (3.0, false),
        (2.0, false),
        (4.0, false),
        (6.0, true),
        (10.0, true),
        (13.0, true),
        (9.0, false),
        (5.0, false),
        (3.0, false),
        (2.0, false),
        (2.0, false),
    ]
}

fn sample_incoming_job(seq: usize) -> QueueJob {
    match seq % 5 {
        0 => QueueJob::new(
            format!("job-{seq}"),
            format!("Summarize ticket #{seq}"),
            QueueJobKind::Llm,
        )
        .with_name_zh(format!("提炼工单摘要 #{seq}")),
        1 => QueueJob::new(
            format!("job-{seq}"),
            format!("Sync GitHub check #{seq}"),
            QueueJobKind::Tool,
        )
        .with_name_zh(format!("同步 GitHub 检查 #{seq}"))
        .with_app("github"),
        2 => QueueJob::new(
            format!("job-{seq}"),
            format!("Draft VIP follow-up #{seq}"),
            QueueJobKind::Llm,
        )
        .with_name_zh(format!("撰写 VIP 跟进邮件 #{seq}"))
        .with_priority(QueuePriority::High),
        3 => QueueJob::new(
            format!("job-{seq}"),
            format!("Index Notion spec #{seq}"),
            QueueJobKind::Tool,
        )
        .with_name_zh(format!("索引 Notion 规范 #{seq}"))
        .with_app("notion"),
        _ => QueueJob::new(
            format!("job-{seq}"),
            format!("Verify refund policy #{seq}"),
            QueueJobKind::Human,
        )
        .with_name_zh(format!("复核退款策略 #{seq}")),
    }
}

#[cfg(test)]
mod tests {
    use super::{AgentQueue, QueueJob, QueueJobKind, QueueJobStatus, QueuePriority};
    use gpui::{AppContext as _, TestAppContext};

    #[gpui::test]
    fn concurrency_rate_limit_promote_and_cancel_work(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let view = cx.new(|cx| AgentQueue::new("test-queue", cx).with_live_flow(false));

        view.update(cx, |v, cx| {
            assert_eq!(v.concurrency(), 4);
            assert!(!v.is_rate_limited());

            // Scaling down concurrency returns excess running jobs cleanly to Queued
            v.set_concurrency(2, cx);
            assert_eq!(v.concurrency(), 2);
            let running = v
                .jobs()
                .iter()
                .filter(|j| j.status == QueueJobStatus::Running)
                .count();
            assert_eq!(running, 2);

            // Promoting a queued job moves it to High priority at index 0
            assert!(v.promote_job("j6", cx));
            let first_q = v
                .jobs()
                .iter()
                .find(|j| j.status == QueueJobStatus::Queued)
                .expect("queued job");
            assert_eq!(first_q.id.as_ref(), "j6");
            assert_eq!(first_q.priority, QueuePriority::High);

            // Rate limit pauses workers and builds backlog on step_simulation
            v.toggle_rate_limit(cx);
            assert!(v.is_rate_limited());
            let before_q = v
                .jobs()
                .iter()
                .filter(|j| j.status == QueueJobStatus::Queued)
                .count();
            v.step_simulation(cx);
            let after_q = v
                .jobs()
                .iter()
                .filter(|j| j.status == QueueJobStatus::Queued)
                .count();
            assert!(after_q > before_q, "backlog climbs while rate limited");

            // Cancelling removes the job
            v.enqueue_job(
                QueueJob::new("custom-j", "Custom job", QueueJobKind::Tool),
                cx,
            );
            assert!(v.cancel_job("custom-j", cx));
        });
    }
}
