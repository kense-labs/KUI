//! Twenty-four-hour API rate-limit capacity scheduler with red overflow hatching and draggable runs.
//!
//! Models aggregate tokens-per-minute across scheduled agent jobs against an API limit ceiling,
//! hatches any peak exceeding the ceiling in red via [`PathBuilder`], lets users drag schedule bars
//! across hours, and provides one-click `"Spread out"` peak elimination.

use crate::{
    components::pro_chrome::{
        ProPalette, pro_agent_avatar, pro_card_shadow, pro_check_draw, pro_eid, pro_empty_state,
        pro_icon_btn, pro_mix_color, pro_popover_presence, pro_rolling_number, pro_skeleton_bar,
        pro_spring_f32, pro_swap_label, pro_tactile, pro_toast_banner, tr,
    },
    foundation::Theme,
    primitives::{
        family::{Slide, disabled_motion},
        pro_icons::{ProIcon, pro_icon},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

const CARD_MAX_W: f32 = 760.0;
const WHO_COL_W: f32 = 206.0;
const CHART_H: f32 = 142.0;
const LANE_H: f32 = 34.0;
const SAMPLE_COUNT: usize = 145; // Every 10 minutes across 1440m

fn spread_wave_icon(side: f32, color: Hsla) -> AnyElement {
    canvas(
        |_, _, _| (),
        move |bounds, _, win, _| {
            let s = f32::from(bounds.size.width).min(f32::from(bounds.size.height)) / 16.0;
            if s <= 0.05 {
                return;
            }
            let ox = f32::from(bounds.left());
            let oy = f32::from(bounds.top());
            let pt = |x: f32, y: f32| point(px(ox + x * s), px(oy + y * s));
            let sw = (1.35 * s).clamp(1.05, 1.6);

            let mut wave = PathBuilder::stroke(px(sw));
            wave.move_to(pt(2.4, 11.2));
            wave.curve_to(pt(6.0, 5.4), pt(4.3, 11.2));
            wave.curve_to(pt(9.5, 11.2), pt(7.8, 5.4));
            wave.curve_to(pt(13.2, 5.4), pt(11.3, 11.2));
            if let Ok(path) = wave.build() {
                win.paint_path(path, color);
            }

            let mut base = PathBuilder::stroke(px(sw));
            base.move_to(pt(2.4, 13.5));
            base.line_to(pt(13.6, 13.5));
            if let Ok(path) = base.build() {
                win.paint_path(path, color);
            }
        },
    )
    .size(px(side))
    .flex_none()
    .into_any_element()
}

fn run_avatar_seed(run: &PlannedRun) -> String {
    let low_id = run.id.to_ascii_lowercase();
    let low_nm = run.name.to_ascii_lowercase();
    for kw in ["claude", "gpt", "openai", "gemini", "grok"] {
        if low_id.contains(kw) || low_nm.contains(kw) {
            return format!("{} {}", run.id, run.name);
        }
    }
    let brand = match low_id.as_str() {
        "triage" => "gemini",
        "research" => "claude",
        "report" => "gpt",
        "review" => "claude",
        _ => ["gemini", "claude", "gpt", "grok"][run.color_idx % 4],
    };
    format!("{brand} {}", run.name)
}

/// Capacity health state relative to the API ceiling and safety buffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LimitHealth {
    /// Peak stays comfortably below `limit * buffer`.
    Under,
    /// Peak exceeds `limit * buffer` without breaching `limit`.
    Close,
    /// Peak exceeds `limit` (overflow region is hatched in red).
    Over,
}

/// One scheduled agent batch job in [`RateLimitPlanner`].
#[derive(Clone, Debug, PartialEq)]
pub struct PlannedRun {
    /// Stable run identifier.
    pub id: SharedString,
    /// Display name of the agent job.
    pub name: SharedString,
    /// Optional Chinese job name.
    pub name_zh: Option<SharedString>,
    /// Start time in minutes after midnight (`0..1440`, e.g. `660` = `11:00`).
    pub start_min: i32,
    /// Peak load in tokens per minute (`TPM`).
    pub peak_tpm: f32,
    /// Minutes taken to climb smoothly from `0` to `peak_tpm`.
    pub rise_min: f32,
    /// Decay half-life in minutes after reaching `peak_tpm`.
    pub half_life_min: f32,
    /// Palette hue slot (`0..6`).
    pub color_idx: usize,
    /// Whether this run has already started and stays anchored during `Spread out`.
    pub pinned_running: bool,
}

impl PlannedRun {
    /// Create a [`PlannedRun`] starting at `start_min` (minutes after midnight) with `peak_tpm`.
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        start_min: i32,
        peak_tpm: f32,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            name_zh: None,
            start_min: start_min.rem_euclid(1_440),
            peak_tpm: peak_tpm.max(1_000.0),
            rise_min: 45.0,
            half_life_min: 75.0,
            color_idx: 0,
            pinned_running: false,
        }
    }

    /// Provide a localized Chinese job name.
    pub fn with_name_zh(mut self, name_zh: impl Into<SharedString>) -> Self {
        self.name_zh = Some(name_zh.into());
        self
    }

    /// Set the S-curve climb duration in minutes (`45.0` by default).
    pub fn with_rise_min(mut self, rise_min: f32) -> Self {
        self.rise_min = rise_min.clamp(10.0, 240.0);
        self
    }

    /// Set the exponential tail half-life in minutes (`75.0` by default).
    pub fn with_half_life_min(mut self, half_life_min: f32) -> Self {
        self.half_life_min = half_life_min.clamp(15.0, 300.0);
        self
    }

    /// Set the palette accent slot (`0..6`).
    pub fn with_color_idx(mut self, color_idx: usize) -> Self {
        self.color_idx = color_idx;
        self
    }

    /// Mark this job as already running so `Spread out` keeps its start time anchored.
    pub fn with_pinned_running(mut self, pinned_running: bool) -> Self {
        self.pinned_running = pinned_running;
        self
    }

    fn display_name<'a>(&'a self, cx: &App) -> &'a str {
        match &self.name_zh {
            Some(zh) => tr(cx, self.name.as_ref(), zh.as_ref()),
            None => self.name.as_ref(),
        }
    }

    /// Effective visual span of the run bar in minutes (`rise + 2.1 * half_life`).
    pub fn visual_span_min(&self) -> f32 {
        (self.rise_min + self.half_life_min * 2.1).clamp(60.0, 480.0)
    }

    /// Sample this run's token-per-minute contribution at `minute_of_day` (`0..1440`).
    pub fn load_at_minute(&self, minute_of_day: f32) -> f32 {
        let dt = (minute_of_day - self.start_min as f32).rem_euclid(1_440.0);
        let cutoff = self.rise_min + self.half_life_min * 6.0;
        if dt > cutoff {
            return 0.0;
        }
        if dt <= self.rise_min {
            let u = (dt / self.rise_min.max(1.0)).clamp(0.0, 1.0);
            let s = u * u * (3.0 - 2.0 * u);
            self.peak_tpm * s
        } else {
            let decay = 0.5_f32.powf((dt - self.rise_min) / self.half_life_min.max(1.0));
            self.peak_tpm * decay
        }
    }
}

/// Sampled 24-hour aggregate load curve and peak diagnostics.
#[derive(Clone, Debug, PartialEq)]
pub struct LoadSummary {
    /// Aggregate TPM sampled every 10 minutes from `start_hour` (`145` points).
    pub samples: Vec<f32>,
    /// Maximum aggregate TPM across the 24-hour window.
    pub peak_tpm: f32,
    /// Minute after midnight (`0..1440`) where `peak_tpm` occurs.
    pub peak_min: i32,
    /// Total integrated tokens across the 24-hour day.
    pub total_tokens: u64,
    /// Capacity health (`Under`, `Close`, or `Over`).
    pub health: LimitHealth,
}

/// Evaluate the 24-hour aggregate TPM curve and peak status for `runs`.
pub fn compute_load_summary(
    runs: &[PlannedRun],
    start_hour: u8,
    limit: f32,
    buffer: f32,
) -> LoadSummary {
    let base_min = (start_hour as i32 % 24) * 60;
    let mut samples = Vec::with_capacity(SAMPLE_COUNT);
    let mut peak_tpm = 0.0_f32;
    let mut peak_min = base_min;
    let mut total_tokens_f = 0.0_f64;

    for i in 0..SAMPLE_COUNT {
        let m = (base_min + i as i32 * 10).rem_euclid(1_440);
        let mut sum = 0.0_f32;
        for r in runs {
            let v = r.load_at_minute(m as f32);
            if v.is_finite() && v > 0.0 {
                sum += v;
            }
        }
        if sum > peak_tpm {
            peak_tpm = sum;
            peak_min = m;
        }
        if i + 1 < SAMPLE_COUNT {
            total_tokens_f += sum as f64 * 10.0;
        }
        samples.push(sum);
    }

    let safe_limit = limit.max(1.0);
    let health = if peak_tpm > safe_limit {
        LimitHealth::Over
    } else if peak_tpm > safe_limit * buffer.clamp(0.5, 0.99) {
        LimitHealth::Close
    } else {
        LimitHealth::Under
    };

    LoadSummary {
        samples,
        peak_tpm,
        peak_min,
        total_tokens: total_tokens_f.round().max(0.0) as u64,
        health,
    }
}

/// Format minutes after midnight (`0..1440`) as `"HH:MM"`.
pub fn format_clock_hhmm(min_of_day: i32) -> String {
    let m = min_of_day.rem_euclid(1_440);
    format!("{:02}:{:02}", m / 60, m % 60)
}

fn format_k(tpm: f32) -> String {
    if tpm >= 1_000_000.0 {
        format!("{:.1}M", tpm / 1_000_000.0)
    } else if tpm >= 1_000.0 {
        format!("{:.0}k", tpm / 1_000.0)
    } else {
        format!("{tpm:.0}")
    }
}

fn run_color(slot: usize, p: &ProPalette) -> Hsla {
    match slot % 6 {
        0 => p.blue,
        1 => p.purple,
        2 => p.teal,
        3 => p.pink,
        4 => p.orange,
        _ => p.green,
    }
}

#[derive(Clone, Copy)]
struct DragSession {
    run_idx: usize,
    origin_x: f32,
    initial_offset_min: f32,
}

/// Typed events emitted by [`RateLimitPlanner`].
#[derive(Clone, Debug, PartialEq)]
pub enum RateLimitPlannerEvent {
    /// Schedule changed via bar drag, keyboard nudge, add, or remove.
    RunsChanged(Vec<PlannedRun>),
    /// Active highlighted run id changed.
    SelectedChanged(Option<SharedString>),
    /// User clicked `Spread out` to redistribute overlapping peaks.
    SpreadOut(Vec<PlannedRun>),
}

impl EventEmitter<RateLimitPlannerEvent> for RateLimitPlanner {}

/// Interactive 24-hour API TPM capacity planner with red overflow hatching and draggable run bars.
///
/// # Examples
///
/// ```no_run
/// # #[allow(unused_variables, unused_imports)]
/// # fn example(window: &mut gpui::Window, cx: &mut gpui::App) {
/// # use gpui::prelude::*;
/// use gpui::prelude::*;
/// use kui::components::rate_limit_planner::{
///     PlannedRun, RateLimitPlanner, RateLimitPlannerEvent,
/// };
///
/// let planner = cx.new(|cx| {
///     RateLimitPlanner::new("org-tpm-planner", cx)
///         .with_limit(120_000.0)
///         .with_runs(vec![
///             PlannedRun::new("triage", "Inbox triage", 8 * 60, 46_000.0).with_pinned_running(true),
///             PlannedRun::new("research", "Lead research", 11 * 60, 82_000.0).with_color_idx(1),
///             PlannedRun::new("report", "Report writer", 11 * 60 + 30, 74_000.0).with_color_idx(2),
///         ])
/// });
/// cx.subscribe(&planner, |_, ev: &RateLimitPlannerEvent, _| {
///     let _ = ev;
/// }).detach();
/// # }
/// ```
pub struct RateLimitPlanner {
    id: ElementId,
    title: SharedString,
    title_zh: SharedString,
    runs: Vec<PlannedRun>,
    limit: f32,
    buffer: f32,
    start_hour: u8,
    selected_id: Option<SharedString>,
    hovered_id: Option<SharedString>,
    hover_chart_frac: Option<f32>,
    scrub_slide: Slide,
    drag: Option<DragSession>,
    slot_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    toast: Option<(SharedString, Instant)>,
    next_seq: usize,
    started_at: Instant,
    loading: bool,
    empty: bool,
}

impl RateLimitPlanner {
    /// Create a [`RateLimitPlanner`] preloaded with an overlapping 4-agent schedule that peaks above `120k / min`.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        Self {
            id: id.into(),
            title: SharedString::from("Rate limit"),
            title_zh: SharedString::from("速率限额规划"),
            runs: default_overlapping_runs(),
            limit: 120_000.0,
            buffer: 0.80,
            start_hour: 6,
            selected_id: None,
            hovered_id: None,
            hover_chart_frac: None,
            scrub_slide: Slide::snappy(0.5),
            drag: None,
            slot_bounds: Rc::new(Cell::new(None)),
            toast: None,
            next_seq: 10,
            started_at: Instant::now(),
            loading: false,
            empty: false,
        }
    }

    /// Set the card heading (`"Rate limit"` by default).
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = title.into();
        self
    }

    /// Provide a localized Chinese card heading.
    pub fn with_title_zh(mut self, title_zh: impl Into<SharedString>) -> Self {
        self.title_zh = title_zh.into();
        self
    }

    /// Supply the scheduled agent runs.
    pub fn with_runs(mut self, runs: Vec<PlannedRun>) -> Self {
        self.runs = runs;
        self
    }

    /// Set the API ceiling in tokens per minute (`120_000.0` by default).
    pub fn with_limit(mut self, limit: f32) -> Self {
        self.limit = limit.max(1_000.0);
        self
    }

    /// Set the warning buffer share of `limit` (`0.80` by default).
    pub fn with_buffer(mut self, buffer: f32) -> Self {
        self.buffer = buffer.clamp(0.50, 0.98);
        self
    }

    /// Set the leftmost hour of the 24-hour window (`6` = `06:00` by default).
    pub fn with_start_hour(mut self, start_hour: u8) -> Self {
        self.start_hour = start_hour % 24;
        self
    }

    /// Pre-select a run id so its individual load curve highlights in color.
    pub fn with_selected(mut self, selected: Option<SharedString>) -> Self {
        self.selected_id = selected;
        self
    }

    /// Toggle skeleton loading placeholders.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Force the empty schedule state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        if empty {
            self.runs.clear();
        }
        self
    }

    /// Return the current slice of scheduled runs.
    pub fn runs(&self) -> &[PlannedRun] {
        &self.runs
    }

    /// Return the configured API ceiling in tokens per minute.
    pub fn limit(&self) -> f32 {
        self.limit
    }

    /// Compute the current 24-hour load summary and peak health.
    pub fn summary(&self) -> LoadSummary {
        compute_load_summary(&self.runs, self.start_hour, self.limit, self.buffer)
    }

    /// Shift overlapping movable runs across the 24-hour window to minimize peak TPM below the ceiling.
    pub fn spread_out(&mut self, cx: &mut Context<Self>) {
        if self.runs.len() < 2 {
            return;
        }
        let base_min = (self.start_hour as i32 % 24) * 60;
        let mut placed: Vec<PlannedRun> = self
            .runs
            .iter()
            .filter(|r| r.pinned_running)
            .cloned()
            .collect();

        let mut movable: Vec<(usize, PlannedRun)> = self
            .runs
            .iter()
            .cloned()
            .enumerate()
            .filter(|(_, r)| !r.pinned_running)
            .collect();
        movable.sort_by(|a, b| {
            b.1.peak_tpm
                .partial_cmp(&a.1.peak_tpm)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        for (_, mut cand) in movable {
            let orig_off = (cand.start_min - base_min).rem_euclid(1_440);
            let max_off = (1_440.0 - cand.visual_span_min()).max(60.0) as i32;
            let mut best_min = cand.start_min;
            let mut best_cost = f32::INFINITY;

            let mut off = 30i32;
            while off <= max_off {
                let try_min = (base_min + off).rem_euclid(1_440);
                cand.start_min = try_min;
                placed.push(cand.clone());
                let sim = compute_load_summary(&placed, self.start_hour, self.limit, self.buffer);
                placed.pop();

                // Primary objective: minimize aggregate peak; small tie-breaker keeps runs near preferred order
                let shift_pen = ((off - orig_off).abs() as f32) * 4.5;
                let over_pen = (sim.peak_tpm - self.limit * self.buffer).max(0.0) * 8.0;
                let score = sim.peak_tpm + over_pen + shift_pen;
                if score < best_cost {
                    best_cost = score;
                    best_min = try_min;
                }
                off += 15;
            }
            cand.start_min = best_min;
            placed.push(cand);
        }

        for r in &mut self.runs {
            if let Some(updated) = placed.iter().find(|u| u.id == r.id) {
                r.start_min = updated.start_min;
            }
        }

        let after = self.summary();
        let pct = ((after.peak_tpm / self.limit) * 100.0).round() as u32;
        self.toast = Some((
            SharedString::from(
                tr(
                    cx,
                    &format!("Spread runs — peak lowered to {pct}% of limit"),
                    &format!("已错峰排布 — 峰值降至限额的 {pct}%"),
                )
                .to_string(),
            ),
            Instant::now(),
        ));
        cx.emit(RateLimitPlannerEvent::SpreadOut(self.runs.clone()));
        cx.emit(RateLimitPlannerEvent::RunsChanged(self.runs.clone()));
        cx.notify();
    }

    /// Shift `id` earlier or later by `delta_min` minutes within the 24-hour window.
    pub fn shift_run(&mut self, id: &str, delta_min: i32, cx: &mut Context<Self>) -> bool {
        let base_min = (self.start_hour as i32 % 24) * 60;
        let Some(run) = self.runs.iter_mut().find(|r| r.id.as_ref() == id) else {
            return false;
        };
        if run.pinned_running {
            return false;
        }
        let max_off = (1_440.0 - run.visual_span_min()).max(30.0) as i32;
        let cur_off = (run.start_min - base_min).rem_euclid(1_440);
        let next_off = (cur_off + delta_min).clamp(0, max_off);
        run.start_min = (base_min + next_off).rem_euclid(1_440);
        cx.emit(RateLimitPlannerEvent::RunsChanged(self.runs.clone()));
        cx.notify();
        true
    }

    /// Append a new agent job into the lowest-load time valley of the 24-hour day.
    pub fn add_run(&mut self, cx: &mut Context<Self>) {
        self.next_seq += 1;
        let base_min = (self.start_hour as i32 % 24) * 60;
        let cur = self.summary();
        let mut min_idx = 0usize;
        let mut min_val = f32::INFINITY;
        for (i, &v) in cur.samples.iter().enumerate().take(120).skip(6) {
            if v < min_val {
                min_val = v;
                min_idx = i;
            }
        }
        let best_min = (base_min + min_idx as i32 * 10).rem_euclid(1_440);
        let names = [
            ("Nightly evals", "夜间评测跑批", 48_000.0),
            ("Docs sync", "知识库增量同步", 38_000.0),
            ("Churn signals", "流失风险扫描", 52_000.0),
        ];
        let (en, zh, pk) = names[self.next_seq % names.len()];
        let slot = self.runs.len() % 6;
        let run = PlannedRun::new(format!("run-{}", self.next_seq), en, best_min, pk)
            .with_name_zh(zh)
            .with_color_idx(slot);
        self.empty = false;
        self.selected_id = Some(run.id.clone());
        self.runs.push(run);
        cx.emit(RateLimitPlannerEvent::RunsChanged(self.runs.clone()));
        cx.notify();
    }

    /// Remove `id` from the schedule.
    pub fn remove_run(&mut self, id: &str, cx: &mut Context<Self>) -> bool {
        let before = self.runs.len();
        self.runs.retain(|r| r.id.as_ref() != id);
        if self.runs.len() != before {
            if self.selected_id.as_deref() == Some(id) {
                self.selected_id = None;
            }
            cx.emit(RateLimitPlannerEvent::RunsChanged(self.runs.clone()));
            cx.notify();
            true
        } else {
            false
        }
    }
}

impl Render for RateLimitPlanner {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let reduced = disabled_motion(cx) || Theme::global(cx).reduced_motion;
        let elapsed_ms = self.started_at.elapsed().as_secs_f32() * 1_000.0;
        let card_bg: Hsla = if p.dark {
            rgb(0x1c1c1e).into()
        } else {
            p.panel
        };
        let card_border: Hsla = if p.dark {
            hsla(0., 0., 1., 0.10)
        } else {
            hsla(0., 0., 0., 0.06)
        };

        if self.loading {
            if !reduced {
                window.request_animation_frame();
            }
            let phase = (elapsed_ms / 1_400.0).fract();
            return div()
                .id(self.id.clone())
                .w_full()
                .max_w(px(CARD_MAX_W))
                .p(px(16.))
                .rounded(px(12.))
                .bg(card_bg)
                .border_1()
                .border_color(card_border)
                .shadow(pro_card_shadow(p.dark))
                .flex()
                .flex_col()
                .gap(px(14.))
                .child(pro_skeleton_bar(220., 18., 5., phase, cx))
                .child(pro_skeleton_bar(
                    CARD_MAX_W - 32.,
                    CHART_H,
                    8.,
                    phase + 0.1,
                    cx,
                ))
                .child(pro_skeleton_bar(CARD_MAX_W - 32., 30., 6., phase + 0.2, cx))
                .child(pro_skeleton_bar(CARD_MAX_W - 32., 30., 6., phase + 0.3, cx))
                .into_any_element();
        }

        if self.empty || self.runs.is_empty() {
            return div()
                .id(self.id.clone())
                .w_full()
                .max_w(px(CARD_MAX_W))
                .rounded(px(12.))
                .bg(card_bg)
                .border_1()
                .border_color(card_border)
                .shadow(pro_card_shadow(p.dark))
                .child(pro_empty_state(
                    ProIcon::UsageChart,
                    tr(cx, "No scheduled agent runs", "暂无计划运行任务"),
                    tr(
                        cx,
                        "Add recurring batch jobs to model aggregate TPM load against your API limit.",
                        "添加智能体定时任务，预测全天聚合每分钟 Token 峰值并自动规避 429 限流。",
                    ),
                    Some((
                        pro_eid(&self.id, "load-runs"),
                        SharedString::from(tr(cx, "Load 4-agent schedule", "加载 4 任务排程")),
                        Box::new(|this: &mut Self, _, cx| {
                            this.empty = false;
                            this.runs = default_overlapping_runs();
                            cx.emit(RateLimitPlannerEvent::RunsChanged(this.runs.clone()));
                            cx.notify();
                        }),
                    )),
                    cx,
                ))
                .into_any_element();
        }

        let any_pinned = self.runs.iter().any(|r| r.pinned_running);
        if any_pinned && !reduced {
            window.request_animation_frame();
        }

        let base_min = (self.start_hour as i32 % 24) * 60;
        let is_dragging = self.drag.is_some();

        // Smoothly glide bars & curve whenever Spread out / keyboard shift updates start_min
        let mut animated_runs = self.runs.clone();
        let mut smooth_offs = Vec::with_capacity(animated_runs.len());
        for (idx, r) in animated_runs.iter_mut().enumerate() {
            let target_off = (r.start_min - base_min).rem_euclid(1_440) as f32;
            let secs = if is_dragging && self.drag.is_some_and(|d| d.run_idx == idx) {
                0.01
            } else {
                0.38
            };
            let smooth_off = pro_spring_f32(
                pro_eid(&self.id, format!("roff-{}", r.id)),
                target_off,
                secs,
                window,
                cx,
            );
            smooth_offs.push(smooth_off);
            r.start_min = (base_min + smooth_off.round() as i32).rem_euclid(1_440);
        }

        let summary =
            compute_load_summary(&animated_runs, self.start_hour, self.limit, self.buffer);
        let peak_pct = ((summary.peak_tpm / self.limit) * 100.0).round() as u32;
        let total_m = summary.total_tokens as f64 / 1_000_000.0;

        let (badge_bg, badge_fg, badge_status) = match summary.health {
            LimitHealth::Under => (
                p.green_soft,
                p.green_ink,
                tr(cx, "Under limit", "低于限额").to_string(),
            ),
            LimitHealth::Close => (
                p.amber_soft,
                p.amber_ink,
                tr(cx, "Close to limit", "接近限额").to_string(),
            ),
            LimitHealth::Over => (
                p.red_soft,
                p.red_ink,
                tr(cx, "Over limit", "超出限额").to_string(),
            ),
        };

        let spread_btn = pro_tactile(
            pro_eid(&self.id, "spread-btn"),
            tr(cx, "Spread out overlapping runs", "一键错峰排布重叠任务"),
            7.0,
            card_bg,
            p.chip_hover,
            p.line_strong,
            p.line_strong,
            false,
            div()
                .h(px(28.))
                .pl(px(8.))
                .pr(px(10.))
                .flex()
                .items_center()
                .gap(px(6.))
                .text_size(px(12.5))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.ink)
                .child(spread_wave_icon(15., p.muted))
                .child(tr(cx, "Spread out", "一键错峰")),
            cx,
            |this, _, cx| {
                this.spread_out(cx);
            },
        )
        .w_auto();

        let header = div()
            .px(px(16.))
            .pt(px(16.))
            .flex()
            .flex_wrap()
            .items_start()
            .justify_between()
            .gap(px(12.))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .child(
                        div()
                            .text_size(px(15.))
                            .line_height(px(22.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .child(tr(cx, self.title.as_ref(), self.title_zh.as_ref()).to_string()),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(5.))
                            .text_size(px(12.))
                            .text_color(p.muted)
                            .child(pro_rolling_number(
                                pro_eid(&self.id, "rlp-runs-cnt"),
                                self.runs.len() as f64,
                                0,
                                "",
                                tr(cx, " runs", " 个任务"),
                                12.,
                                FontWeight::NORMAL,
                                p.muted,
                            ))
                            .child("·")
                            .child(pro_rolling_number(
                                pro_eid(&self.id, "rlp-tot-m"),
                                total_m.max(1.0),
                                0,
                                "",
                                tr(cx, "M tokens", "M Tokens"),
                                12.,
                                FontWeight::NORMAL,
                                p.muted,
                            ))
                            .child("·")
                            .child(format!(
                                "{} / {}",
                                format_k(self.limit),
                                tr(cx, "min", "分钟")
                            )),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(
                        div()
                            .h(px(22.))
                            .px(px(8.))
                            .rounded(px(7.))
                            .bg(badge_bg)
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(badge_fg)
                            .flex()
                            .items_center()
                            .gap(px(5.))
                            .when(summary.health == LimitHealth::Under, |d| {
                                d.child(pro_check_draw(
                                    pro_eid(&self.id, "rlp-ok-chk"),
                                    true,
                                    12.5,
                                    false,
                                    p.green,
                                ))
                            })
                            .child(pro_swap_label(
                                pro_eid(&self.id, "rlp-health-lbl"),
                                badge_status,
                                12.,
                                FontWeight::MEDIUM,
                                badge_fg,
                            ))
                            .child("·")
                            .child(pro_rolling_number(
                                pro_eid(&self.id, "rlp-peak-pct"),
                                f64::from(peak_pct),
                                0,
                                tr(cx, "peak ", "峰值 "),
                                "%",
                                12.,
                                FontWeight::MEDIUM,
                                badge_fg,
                            )),
                    )
                    .child(spread_btn),
            );

        // Chart scale & PathBuilder curve + red diagonal hatch above ceiling
        let y_max = (self.limit * 1.28)
            .max(summary.peak_tpm * 1.12)
            .max(1_000.0);
        let limit_y_frac = 1.0 - (self.limit / y_max).clamp(0.05, 0.95);
        let buf_y_frac = 1.0 - ((self.limit * self.buffer) / y_max).clamp(0.05, 0.95);

        let active_run_id = self.hovered_id.clone().or_else(|| self.selected_id.clone());
        let selected_curve: Option<(Hsla, Vec<f32>)> = active_run_id
            .as_ref()
            .and_then(|rid| animated_runs.iter().find(|r| &r.id == rid))
            .map(|r| {
                let col = run_color(r.color_idx, &p);
                let vals = (0..SAMPLE_COUNT)
                    .map(|i| {
                        let m = (base_min + i as i32 * 10).rem_euclid(1_440);
                        r.load_at_minute(m as f32)
                    })
                    .collect();
                (col, vals)
            });

        let samples_for_paint = summary.samples.clone();
        let limit_val = self.limit;
        let scrub_x = self.scrub_slide.get(window, cx).clamp(0.0, 1.0);
        let scrub_alpha = pro_spring_f32(
            pro_eid(&self.id, "rlp-scrub-a"),
            if self.hover_chart_frac.is_some() {
                1.0
            } else {
                0.0
            },
            0.16,
            window,
            cx,
        );
        let sel_curve_alpha = pro_spring_f32(
            pro_eid(&self.id, "rlp-sel-curve"),
            if selected_curve.is_some() { 0.88 } else { 0.0 },
            0.20,
            window,
            cx,
        );
        let dark = p.dark;
        let chart_bounds_cell: Rc<Cell<Option<Bounds<Pixels>>>> = Rc::new(Cell::new(None));
        let chart_bounds_pre = chart_bounds_cell.clone();

        let chart_canvas = canvas(
            move |bounds, _, _| {
                chart_bounds_pre.set(Some(bounds));
            },
            move |bounds, _, window, _| {
                let w = f32::from(bounds.size.width);
                let h = f32::from(bounds.size.height);
                if w <= 8.0 || h <= 8.0 || samples_for_paint.len() < 2 {
                    return;
                }
                let pad_top = 6.0;
                let plot_h = (h - pad_top).max(8.0);
                let y_of = |val: f32| -> f32 {
                    let norm = (val / y_max).clamp(0.0, 1.0);
                    f32::from(bounds.top()) + pad_top + (1.0 - norm) * plot_h
                };
                let x_of = |idx: usize| -> f32 {
                    let frac = idx as f32 / (SAMPLE_COUNT - 1) as f32;
                    f32::from(bounds.left()) + frac * w
                };
                let interp_val = |px_x: f32| -> f32 {
                    let u = ((px_x - f32::from(bounds.left())) / w).clamp(0.0, 1.0)
                        * (SAMPLE_COUNT - 1) as f32;
                    let i0 = (u.floor() as usize).min(SAMPLE_COUNT - 1);
                    let i1 = (i0 + 1).min(SAMPLE_COUNT - 1);
                    let f = u - i0 as f32;
                    samples_for_paint[i0] + (samples_for_paint[i1] - samples_for_paint[i0]) * f
                };

                // 1. Vertical 3-hour guide lines
                for step in 0..=8 {
                    let gx = f32::from(bounds.left()) + (step as f32 / 8.0) * w;
                    let mut vline = PathBuilder::stroke(px(1.0));
                    vline.move_to(point(px(gx), bounds.top()));
                    vline.line_to(point(px(gx), bounds.bottom()));
                    if let Ok(path) = vline.build() {
                        window.paint_path(path, p.line_soft);
                    }
                }

                // 2. Buffer dashed line (80% of limit)
                let y_buf = y_of(limit_val * 0.80);
                let mut bx = f32::from(bounds.left());
                while bx < f32::from(bounds.right()) {
                    let end_x = (bx + 2.5).min(f32::from(bounds.right()));
                    let mut seg = PathBuilder::stroke(px(1.0));
                    seg.move_to(point(px(bx), px(y_buf)));
                    seg.line_to(point(px(end_x), px(y_buf)));
                    if let Ok(path) = seg.build() {
                        window.paint_path(path, p.line_strong.opacity(0.6));
                    }
                    bx += 6.0;
                }

                // 3. Soft area fill under the aggregate curve
                let mut area = PathBuilder::fill();
                area.move_to(point(bounds.left(), bounds.bottom()));
                for (i, &v) in samples_for_paint.iter().enumerate() {
                    area.line_to(point(px(x_of(i)), px(y_of(v))));
                }
                area.line_to(point(bounds.right(), bounds.bottom()));
                area.close();
                if let Ok(path) = area.build() {
                    window.paint_path(
                        path,
                        linear_gradient(
                            180.,
                            linear_color_stop(p.blue.opacity(if dark { 0.16 } else { 0.11 }), 0.0),
                            linear_color_stop(p.blue.opacity(0.0), 1.0),
                        ),
                    );
                }

                // 4. Red tint + crisp -45deg PathBuilder diagonal hatch lines wherever curve > limit!
                let y_lim = y_of(limit_val);
                let has_overflow = samples_for_paint.iter().any(|&v| v > limit_val + 10.0);
                if has_overflow {
                    let mut over_fill = PathBuilder::fill();
                    over_fill.move_to(point(bounds.left(), px(y_lim)));
                    for (i, &v) in samples_for_paint.iter().enumerate() {
                        over_fill.line_to(point(px(x_of(i)), px(y_of(v.max(limit_val)))));
                    }
                    over_fill.line_to(point(bounds.right(), px(y_lim)));
                    over_fill.close();
                    if let Ok(path) = over_fill.build() {
                        window.paint_path(path, p.red.opacity(if dark { 0.13 } else { 0.09 }));
                    }

                    let hatch_stroke_c = p.red.opacity(if dark { 0.56 } else { 0.44 });
                    let left_x = f32::from(bounds.left());
                    let right_x = f32::from(bounds.right());
                    let top_y = f32::from(bounds.top());
                    let spacing = 5.5_f32;
                    let mut c_offset = -h;
                    while c_offset <= w + h {
                        // Diagonal line: y = y_lim - ((x - left_x) - c_offset) -> goes up-right or down-right:
                        // Use -45deg: y = top_y + (x - left_x - c_offset)
                        let mut in_seg: Option<(Point<Pixels>, Point<Pixels>)> = None;
                        let mut sx = left_x;
                        while sx <= right_x {
                            let sy = top_y + (sx - left_x - c_offset);
                            let curve_y = y_of(interp_val(sx));
                            if sy >= curve_y && sy <= y_lim {
                                let pt = point(px(sx), px(sy));
                                in_seg = Some(match in_seg {
                                    Some((start, _)) => (start, pt),
                                    None => (pt, pt),
                                });
                            } else if let Some((st, en)) = in_seg.take()
                                && (f32::from(en.x - st.x).hypot(f32::from(en.y - st.y))) > 1.2
                            {
                                let mut hl = PathBuilder::stroke(px(1.2));
                                hl.move_to(st);
                                hl.line_to(en);
                                if let Ok(path) = hl.build() {
                                    window.paint_path(path, hatch_stroke_c);
                                }
                            }
                            sx += 1.5;
                        }
                        if let Some((st, en)) = in_seg.take()
                            && (f32::from(en.x - st.x).hypot(f32::from(en.y - st.y))) > 1.2
                        {
                            let mut hl = PathBuilder::stroke(px(1.2));
                            hl.move_to(st);
                            hl.line_to(en);
                            if let Ok(path) = hl.build() {
                                window.paint_path(path, hatch_stroke_c);
                            }
                        }
                        c_offset += spacing;
                    }
                }

                // 5. Dashed API limit ceiling line
                let ceil_c = if has_overflow {
                    p.red.opacity(0.88)
                } else {
                    p.line_strong
                };
                let mut lx = f32::from(bounds.left());
                while lx < f32::from(bounds.right()) {
                    let end_x = (lx + 4.5).min(f32::from(bounds.right()));
                    let mut seg = PathBuilder::stroke(px(1.25));
                    seg.move_to(point(px(lx), px(y_lim)));
                    seg.line_to(point(px(end_x), px(y_lim)));
                    if let Ok(path) = seg.build() {
                        window.paint_path(path, ceil_c);
                    }
                    lx += 8.0;
                }

                // 6. Selected single-run curve overlay
                if let Some((scol, svals)) = &selected_curve
                    && sel_curve_alpha > 0.01
                {
                    let mut sl = PathBuilder::stroke(px(1.65));
                    for (i, &v) in svals.iter().enumerate() {
                        let pt = point(px(x_of(i)), px(y_of(v)));
                        if i == 0 {
                            sl.move_to(pt);
                        } else {
                            sl.line_to(pt);
                        }
                    }
                    if let Ok(path) = sl.build() {
                        window.paint_path(path, scol.opacity(sel_curve_alpha));
                    }
                }

                // 7. Aggregate curve stroke (base blue/ink below ceiling, red above ceiling)
                let base_stroke = if dark {
                    hsla(0., 0., 1., 0.78)
                } else {
                    hsla(0., 0., 0., 0.76)
                };
                for i in 0..(samples_for_paint.len() - 1) {
                    let v0 = samples_for_paint[i];
                    let v1 = samples_for_paint[i + 1];
                    let over = v0 > limit_val || v1 > limit_val;
                    let mut seg = PathBuilder::stroke(px(if over { 2.0 } else { 1.65 }));
                    seg.move_to(point(px(x_of(i)), px(y_of(v0))));
                    seg.line_to(point(px(x_of(i + 1)), px(y_of(v1))));
                    if let Ok(path) = seg.build() {
                        window.paint_path(path, if over { p.red } else { base_stroke });
                    }
                }

                // 8. Peak dot marker
                if let Some((pk_idx, &pk_val)) = samples_for_paint
                    .iter()
                    .enumerate()
                    .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                {
                    let px_c = x_of(pk_idx);
                    let py_c = y_of(pk_val);
                    let dot_c = if pk_val > limit_val { p.red } else { p.blue };
                    window.paint_quad(
                        fill(
                            Bounds::new(
                                point(px(px_c - 3.5), px(py_c - 3.5)),
                                size(px(7.), px(7.)),
                            ),
                            dot_c,
                        )
                        .corner_radii(px(99.)),
                    );
                }

                // 9. Smooth Slide::snappy hover crosshair
                if scrub_alpha > 0.01 {
                    let hx = f32::from(bounds.left()) + scrub_x * w;
                    let hv = interp_val(hx);
                    let hy = y_of(hv);
                    let mut ch = PathBuilder::stroke(px(1.1));
                    ch.move_to(point(px(hx), bounds.top()));
                    ch.line_to(point(px(hx), bounds.bottom()));
                    if let Ok(path) = ch.build() {
                        window.paint_path(path, p.line_strong.opacity(scrub_alpha));
                    }
                    window.paint_quad(
                        fill(
                            Bounds::new(point(px(hx - 3.), px(hy - 3.)), size(px(6.), px(6.))),
                            p.ink.opacity(scrub_alpha),
                        )
                        .corner_radii(px(99.)),
                    );
                }
            },
        )
        .size_full();

        let idx_f = scrub_x * (SAMPLE_COUNT - 1) as f32;
        let tip_idx = (idx_f.round() as usize).min(SAMPLE_COUNT - 1);
        let tip_m = (base_min + tip_idx as i32 * 10).rem_euclid(1_440);
        let tip_val = summary.samples[tip_idx];
        let tip_label = format!("{} · {} / min", format_clock_hhmm(tip_m), format_k(tip_val));
        let tip_bg = if p.dark { rgb(0x26262a) } else { rgb(0x18181b) };
        let hover_tooltip = div()
            .absolute()
            .top(px(4.))
            .right(px(8.))
            .child(pro_popover_presence(
                pro_eid(&self.id, "rlp-hov-tip"),
                self.hover_chart_frac.is_some(),
                move || {
                    div()
                        .h(px(20.))
                        .px(px(7.))
                        .rounded(px(5.))
                        .bg(tip_bg)
                        .text_size(px(11.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(rgb(0xffffff))
                        .flex()
                        .items_center()
                        .child(tip_label)
                        .into_any_element()
                },
            ));

        let peak_x_frac =
            ((summary.peak_min - base_min).rem_euclid(1_440) as f32 / 1_440.0).clamp(0.08, 0.78);
        let peak_y_px =
            (6.0 + (1.0 - (summary.peak_tpm / y_max).clamp(0.0, 1.0)) * (CHART_H - 6.0) - 25.0)
                .clamp(0.0, CHART_H - 26.0);
        let peak_pill_bg: Hsla = if summary.health == LimitHealth::Over {
            p.red_soft
        } else if p.dark {
            p.logo_tile
        } else {
            card_bg
        };
        let peak_pill_fg = if summary.health == LimitHealth::Over {
            p.red_ink
        } else {
            p.ink
        };
        let peak_pill_bdr = if summary.health == LimitHealth::Over {
            p.red.opacity(0.32)
        } else {
            p.line_strong
        };
        let peak_callout = (self.hover_chart_frac.is_none()).then(|| {
            div()
                .absolute()
                .left(relative(peak_x_frac))
                .top(px(peak_y_px))
                .ml(px(-42.))
                .h(px(19.))
                .px(px(6.))
                .rounded(px(5.))
                .bg(peak_pill_bg)
                .border_1()
                .border_color(peak_pill_bdr)
                .text_size(px(10.5))
                .font_weight(FontWeight::MEDIUM)
                .text_color(peak_pill_fg)
                .flex()
                .items_center()
                .child(format!(
                    "{} {} · {}",
                    tr(cx, "Peak", "峰值"),
                    format_k(summary.peak_tpm),
                    format_clock_hhmm(summary.peak_min)
                ))
        });

        let chart_bounds_move = chart_bounds_cell;
        let chart_row = div()
            .w_full()
            .h(px(CHART_H))
            .mt(px(14.))
            .pr(px(16.))
            .flex()
            .child(
                div()
                    .relative()
                    .w(px(WHO_COL_W))
                    .flex_none()
                    .h_full()
                    .child(
                        div()
                            .absolute()
                            .right(px(10.))
                            .top(px(
                                (limit_y_frac * (CHART_H - 6.0)).clamp(2.0, CHART_H - 18.0)
                            ))
                            .text_size(px(11.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(if summary.health == LimitHealth::Over {
                                p.red_ink
                            } else {
                                p.muted
                            })
                            .child(format_k(self.limit)),
                    )
                    .child(
                        div()
                            .absolute()
                            .right(px(10.))
                            .top(px(
                                (buf_y_frac * (CHART_H - 6.0)).clamp(22.0, CHART_H - 28.0)
                            ))
                            .text_size(px(10.5))
                            .text_color(p.faint)
                            .child(format_k(self.limit * self.buffer)),
                    )
                    .child(
                        div()
                            .absolute()
                            .right(px(10.))
                            .bottom(px(2.))
                            .text_size(px(10.5))
                            .text_color(p.faint)
                            .child("0"),
                    ),
            )
            .child(
                div()
                    .id(pro_eid(&self.id, "chart-hit"))
                    .relative()
                    .flex_1()
                    .h_full()
                    .border_b_1()
                    .border_color(p.line_strong)
                    .on_hover(cx.listener(|this, inside: &bool, _, cx| {
                        if !*inside && this.hover_chart_frac.is_some() {
                            this.hover_chart_frac = None;
                            cx.notify();
                        }
                    }))
                    .on_mouse_move(cx.listener(move |this, ev: &MouseMoveEvent, _, cx| {
                        if let Some(b) = chart_bounds_move.get() {
                            let w = f32::from(b.size.width).max(1.0);
                            let f = (f32::from(ev.position.x - b.left()) / w).clamp(0.0, 1.0);
                            if this.hover_chart_frac.is_none() {
                                this.scrub_slide = Slide::snappy(f);
                            } else {
                                this.scrub_slide.set(f, cx);
                            }
                            this.hover_chart_frac = Some(f);
                            cx.notify();
                        }
                    }))
                    .child(chart_canvas)
                    .when_some(peak_callout, |d, pc| d.child(pc))
                    .child(hover_tooltip),
            );

        // 24h axis ticks
        let mut axis_ticks = div()
            .flex_1()
            .h(px(26.))
            .flex()
            .items_center()
            .justify_between();
        for step in 0..8u8 {
            let hr = (self.start_hour + step * 3) % 24;
            axis_ticks = axis_ticks.child(
                div()
                    .text_size(px(11.))
                    .text_color(p.faint)
                    .child(format!("{hr:02}:00")),
            );
        }
        let axis_row = div()
            .w_full()
            .pr(px(16.))
            .flex()
            .child(div().w(px(WHO_COL_W)).flex_none())
            .child(axis_ticks);

        // Draggable run lanes
        let slot_sensor_cell = self.slot_bounds.clone();
        let slot_sensor = canvas(
            move |bounds, _, _| {
                slot_sensor_cell.set(Some(bounds));
            },
            |_, _, _, _| (),
        )
        .absolute()
        .left(px(WHO_COL_W))
        .right(px(16.))
        .top_0()
        .bottom_0();

        let mut lanes_wrap = div()
            .id(pro_eid(&self.id, "lanes"))
            .role(Role::Group)
            .aria_label(tr(cx, "Scheduled agent runs", "计划智能体运行轨道"))
            .relative()
            .w_full()
            .flex()
            .flex_col()
            .on_mouse_move(cx.listener(|this, ev: &MouseMoveEvent, _, cx| {
                let Some(drag) = this.drag else {
                    return;
                };
                let Some(sb) = this.slot_bounds.get() else {
                    return;
                };
                let slot_w = f32::from(sb.size.width).max(40.0);
                let dx = f32::from(ev.position.x) - drag.origin_x;
                let d_min = (dx / slot_w) * 1_440.0;
                let base_m = (this.start_hour as i32 % 24) * 60;
                if let Some(run) = this.runs.get_mut(drag.run_idx) {
                    let max_off = (1_440.0 - run.visual_span_min()).max(30.0);
                    let snapped = (((drag.initial_offset_min + d_min) / 10.0).round() * 10.0)
                        .clamp(0.0, max_off) as i32;
                    let next_min = (base_m + snapped).rem_euclid(1_440);
                    if run.start_min != next_min {
                        run.start_min = next_min;
                        cx.emit(RateLimitPlannerEvent::RunsChanged(this.runs.clone()));
                        cx.notify();
                    }
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _: &MouseUpEvent, _, cx| {
                    if this.drag.take().is_some() {
                        cx.notify();
                    }
                }),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, _: &MouseUpEvent, _, cx| {
                    if this.drag.take().is_some() {
                        cx.notify();
                    }
                }),
            )
            .child(slot_sensor);

        let any_lane_spot = pro_spring_f32(
            pro_eid(&self.id, "rlp-any-spot"),
            if active_run_id.is_some() { 1.0 } else { 0.0 },
            0.20,
            window,
            cx,
        );

        for (idx, run) in animated_runs.iter().enumerate() {
            let real_start = self.runs.get(idx).map_or(run.start_min, |r| r.start_min);
            let r_col = run_color(run.color_idx, &p);
            let is_on = self.selected_id.as_ref() == Some(&run.id)
                || self.hovered_id.as_ref() == Some(&run.id)
                || self.drag.is_some_and(|d| d.run_idx == idx);
            let lane_spot = pro_spring_f32(
                pro_eid(&self.id, format!("rlp-spot-{}", run.id)),
                if is_on { 1.0 } else { 0.0 },
                0.20,
                window,
                cx,
            );
            let lane_opacity = 1.0 - any_lane_spot * (1.0 - lane_spot) * 0.42;

            let off_min = smooth_offs
                .get(idx)
                .copied()
                .unwrap_or_else(|| (run.start_min - base_min).rem_euclid(1_440) as f32);
            let span_min = run.visual_span_min();
            let left_frac = (off_min / 1_440.0).clamp(0.0, 0.94);
            let target_w_frac = (span_min / 1_440.0).clamp(0.05, (1.0 - left_frac).max(0.05));
            let width_frac = pro_spring_f32(
                pro_eid(&self.id, format!("rlp-bw-{}", run.id)),
                target_w_frac,
                0.28,
                window,
                cx,
            );

            // Check whether this run contributes during an overflow window
            let sample_start = ((off_min / 10.0).floor() as usize).min(SAMPLE_COUNT - 1);
            let sample_end = (((off_min + span_min) / 10.0).ceil() as usize).min(SAMPLE_COUNT - 1);
            let contributes_to_over = summary.samples[sample_start..=sample_end]
                .iter()
                .any(|&v| v > self.limit);

            let bar_bg = if contributes_to_over {
                pro_mix_color(r_col.opacity(0.14), p.red.opacity(0.20), 0.55)
            } else {
                r_col.opacity(0.11 + 0.11 * lane_spot)
            };
            let bar_border = if contributes_to_over {
                p.red.opacity(0.60)
            } else {
                r_col.opacity(0.36 + 0.28 * lane_spot)
            };
            let bar_fg = if contributes_to_over {
                p.red_ink
            } else {
                pro_mix_color(r_col, p.ink, 0.28)
            };

            let time_str = if run.pinned_running {
                tr(cx, "Running", "运行中").to_string()
            } else {
                format_clock_hhmm(real_start)
            };

            let id_hover = run.id.clone();
            let id_who_sel = run.id.clone();
            let id_sel = run.id.clone();
            let id_key = run.id.clone();
            let id_rm = run.id.clone();
            let pinned = run.pinned_running;
            let init_off = (real_start - base_min).rem_euclid(1_440) as f32;
            let is_selected_row = self.selected_id.as_ref() == Some(&run.id);

            let av_seed = run_avatar_seed(run);
            let who_pill = pro_tactile(
                pro_eid(&self.id, format!("who-btn-{}", run.id)),
                run.display_name(cx).to_string(),
                6.0,
                transparent_black(),
                p.hover,
                transparent_black(),
                p.line,
                is_selected_row,
                div()
                    .w_full()
                    .h(px(28.))
                    .pl(px(8.))
                    .pr(px(8.))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(pro_agent_avatar(
                        &av_seed,
                        r_col,
                        18.0,
                        run.pinned_running,
                        elapsed_ms,
                        cx,
                    ))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_size(px(13.))
                            .text_color(if run.pinned_running { p.muted } else { p.ink })
                            .child(run.display_name(cx).to_string()),
                    )
                    .child(
                        div()
                            .flex_none()
                            .text_size(px(12.))
                            .text_color(if is_on { r_col } else { p.muted })
                            .child(time_str.clone()),
                    ),
                cx,
                move |this, _, cx| {
                    let next = if this.selected_id.as_ref() == Some(&id_who_sel) {
                        None
                    } else {
                        Some(id_who_sel.clone())
                    };
                    this.selected_id = next.clone();
                    cx.emit(RateLimitPlannerEvent::SelectedChanged(next));
                    cx.notify();
                },
            );

            let lane = div()
                .id(pro_eid(&self.id, format!("lane-{}", run.id)))
                .w_full()
                .h(px(LANE_H))
                .pr(px(16.))
                .opacity(lane_opacity)
                .flex()
                .items_center()
                .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                    if *inside {
                        this.hovered_id = Some(id_hover.clone());
                    } else if this.hovered_id.as_ref() == Some(&id_hover) {
                        this.hovered_id = None;
                    }
                    cx.notify();
                }))
                .child(
                    div()
                        .w(px(WHO_COL_W))
                        .flex_none()
                        .pl(px(8.))
                        .pr(px(2.))
                        .child(who_pill),
                )
                .child(
                    div()
                        .relative()
                        .flex_1()
                        .h_full()
                        .child(
                            div()
                                .id(pro_eid(&self.id, format!("bar-{}", run.id)))
                                .role(Role::Button)
                                .aria_label(format!(
                                    "{} · {} · {} / min",
                                    run.display_name(cx),
                                    time_str,
                                    format_k(run.peak_tpm)
                                ))
                                .tab_index(0)
                                .absolute()
                                .top(px(8.))
                                .left(relative(left_frac))
                                .w(relative(width_frac))
                                .h(px(18.))
                                .px(px(6.))
                                .rounded(px(5.))
                                .bg(bar_bg)
                                .border_1()
                                .border_color(bar_border)
                                .when(!pinned, |d| d.cursor_ew_resize())
                                .focus_visible(move |s| s.border_2().border_color(p.focus))
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(move |this, ev: &MouseDownEvent, _, cx| {
                                        this.selected_id = Some(id_sel.clone());
                                        cx.emit(RateLimitPlannerEvent::SelectedChanged(Some(
                                            id_sel.clone(),
                                        )));
                                        if !pinned {
                                            this.drag = Some(DragSession {
                                                run_idx: idx,
                                                origin_x: f32::from(ev.position.x),
                                                initial_offset_min: init_off,
                                            });
                                        }
                                        cx.notify();
                                    }),
                                )
                                .on_key_down(cx.listener(move |this, ev: &KeyDownEvent, _, cx| {
                                    let step = if ev.keystroke.modifiers.shift { 60 } else { 15 };
                                    match ev.keystroke.key.as_str() {
                                        "left" | "arrowleft" => {
                                            this.shift_run(id_key.as_ref(), -step, cx);
                                            cx.stop_propagation();
                                        }
                                        "right" | "arrowright" => {
                                            this.shift_run(id_key.as_ref(), step, cx);
                                            cx.stop_propagation();
                                        }
                                        "backspace" | "delete" if !pinned => {
                                            this.remove_run(id_key.as_ref(), cx);
                                            cx.stop_propagation();
                                        }
                                        _ => {}
                                    }
                                }))
                                .flex()
                                .items_center()
                                .justify_end()
                                .child(
                                    div()
                                        .text_size(px(10.5))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(bar_fg)
                                        .child(format_k(run.peak_tpm)),
                                ),
                        )
                        .when(is_on && !pinned, |d| {
                            d.child(
                                div()
                                    .absolute()
                                    .right(px(0.))
                                    .top(px(7.))
                                    .child(pro_icon_btn(
                                        pro_eid(&self.id, format!("rm-{}", run.id)),
                                        tr(cx, "Remove scheduled run", "移除计划任务"),
                                        ProIcon::Close,
                                        20.0,
                                        false,
                                        cx,
                                        move |this, _, cx| {
                                            this.remove_run(id_rm.as_ref(), cx);
                                        },
                                    )),
                            )
                        }),
                );

            lanes_wrap = lanes_wrap.child(lane);
        }

        // "+ Add run" row
        let add_row = div()
            .w_full()
            .h(px(32.))
            .pl(px(12.))
            .flex()
            .items_center()
            .child(
                pro_tactile(
                    pro_eid(&self.id, "add-run"),
                    tr(cx, "Add scheduled run", "添加定时任务"),
                    5.0,
                    transparent_black(),
                    p.chip_hover,
                    transparent_black(),
                    p.line,
                    false,
                    div()
                        .h(px(24.))
                        .px(px(6.))
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .text_size(px(12.5))
                        .text_color(p.muted)
                        .child(pro_icon(ProIcon::Plus, 12.5, p.muted))
                        .child(tr(cx, "Add run", "添加任务")),
                    cx,
                    |this, _, cx| {
                        this.add_run(cx);
                    },
                )
                .w_auto(),
            );

        // Footer verdict sentence
        let peak_clock = format_clock_hhmm(summary.peak_min);
        let verdict = match summary.health {
            LimitHealth::Over => tr(
                cx,
                &format!(
                    "Overlapping runs exceed the {} / min limit at {peak_clock} ({} peak) — drag a bar or click Spread out",
                    format_k(self.limit),
                    format_k(summary.peak_tpm)
                ),
                &format!(
                    "{peak_clock} 重叠峰值达 {} / 分钟，超出 {} 上限 — 请拖动时间条或点击“一键错峰”",
                    format_k(summary.peak_tpm),
                    format_k(self.limit)
                ),
            )
            .to_string(),
            LimitHealth::Close => tr(
                cx,
                &format!(
                    "Peak reaches {peak_pct}% of the API limit at {peak_clock} — within ceiling, above 80% safety buffer"
                ),
                &format!(
                    "{peak_clock} 峰值达限额的 {peak_pct}% — 未触发限流，但高于 80% 安全缓冲线"
                ),
            )
            .to_string(),
            LimitHealth::Under => tr(
                cx,
                &format!(
                    "All runs stay below the 80% buffer — daily peak is {} / min at {peak_clock}",
                    format_k(summary.peak_tpm)
                ),
                &format!(
                    "全天任务平稳处于 80% 缓冲线内 — 最高峰为 {peak_clock} 的 {} / 分钟",
                    format_k(summary.peak_tpm)
                ),
            )
            .to_string(),
        };

        let footer = div()
            .w_full()
            .px(px(16.))
            .py(px(11.))
            .mt(px(4.))
            .border_t_1()
            .border_color(p.line)
            .flex()
            .items_center()
            .justify_between()
            .gap(px(12.))
            .text_size(px(12.))
            .text_color(if summary.health == LimitHealth::Over {
                p.red_ink
            } else {
                p.muted
            })
            .child(div().flex_1().min_w_0().truncate().child(verdict));

        div()
            .id(self.id.clone())
            .w_full()
            .py(px(14.))
            .flex()
            .justify_center()
            .child(
                div()
                    .relative()
                    .w_full()
                    .max_w(px(CARD_MAX_W))
                    .rounded(px(12.))
                    .bg(card_bg)
                    .border_1()
                    .border_color(card_border)
                    .shadow(pro_card_shadow(p.dark))
                    .flex()
                    .flex_col()
                    .child(header)
                    .child(chart_row)
                    .child(axis_row)
                    .child(lanes_wrap)
                    .child(add_row)
                    .child(footer)
                    .child(pro_toast_banner(self.toast.clone())),
            )
            .into_any_element()
    }
}

fn default_overlapping_runs() -> Vec<PlannedRun> {
    vec![
        PlannedRun::new("triage", "Inbox triage", 8 * 60, 46_000.0)
            .with_name_zh("收件箱分流")
            .with_rise_min(45.0)
            .with_half_life_min(70.0)
            .with_color_idx(0)
            .with_pinned_running(true),
        PlannedRun::new("research", "Lead research", 11 * 60, 82_000.0)
            .with_name_zh("潜客背景调研")
            .with_rise_min(55.0)
            .with_half_life_min(85.0)
            .with_color_idx(1),
        PlannedRun::new("report", "Report writer", 11 * 60 + 30, 74_000.0)
            .with_name_zh("周报自动撰写")
            .with_rise_min(50.0)
            .with_half_life_min(90.0)
            .with_color_idx(2),
        PlannedRun::new("review", "Code review", 18 * 60 + 30, 36_000.0)
            .with_name_zh("代码巡检")
            .with_rise_min(40.0)
            .with_half_life_min(65.0)
            .with_color_idx(3),
    ]
}

#[cfg(test)]
mod tests {
    use super::{LimitHealth, RateLimitPlanner, format_clock_hhmm};
    use gpui::{AppContext as _, TestAppContext};

    #[gpui::test]
    fn default_overlap_exceeds_ceiling_and_spread_out_eliminates_overflow(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let view = cx.new(|cx| RateLimitPlanner::new("test-rlp", cx));

        view.update(cx, |v, cx| {
            let before = v.summary();
            assert_eq!(before.health, LimitHealth::Over);
            assert!(
                before.peak_tpm > v.limit(),
                "expected initial overlap peak ({}) > limit ({})",
                before.peak_tpm,
                v.limit()
            );

            // Pinned run ("Inbox triage" at 08:00) stays anchored while overlapping runs separate
            v.spread_out(cx);
            assert_eq!(v.runs()[0].start_min, 8 * 60);
            let after = v.summary();
            assert_eq!(after.health, LimitHealth::Under);
            assert!(
                after.peak_tpm < v.limit() * 0.80,
                "expected spread-out peak ({}) below 80% buffer",
                after.peak_tpm
            );
            assert_eq!(format_clock_hhmm(11 * 60 + 30), "11:30");
        });
    }
}
