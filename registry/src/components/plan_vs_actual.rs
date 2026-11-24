//! Planned-vs-actual Gantt timeline for an autonomous agent run with live overrun shift and ETA projection.
//!
//! Renders every step's planned window as a dashed ghost bar while the solid actual bar grows
//! over it; overruns turn amber, downstream dependent steps slide right via [`Slide`] springs,
//! dependency elbow connectors light up on hover/focus, and the projected finish marker (`ETA`)
//! separates from the planned finish (`Plan`).

use crate::{
    components::pro_chrome::{
        ProPalette, pro_app_tile, pro_check_draw, pro_eid, pro_empty_state, pro_fold,
        pro_mix_color, pro_popover_presence, pro_rolling_number, pro_skeleton_bar, pro_spring_f32,
        pro_swap_label, pro_tactile, tr,
    },
    foundation::Theme,
    primitives::{
        family::{Slide, disabled_motion},
        pro_icons::{ProIcon, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, collections::HashMap, f32::consts::TAU, rc::Rc};
use web_time::Instant;

const ROW_H: f32 = 32.0;
const BAR_H: f32 = 14.0;
const AXIS_H: f32 = 30.0;
const FOOT_H: f32 = 40.0;
const NAME_COL_W: f32 = 192.0;

const HEADER_GANTT_SVG: &str = r#"<rect x="1.5" y="2.5" width="7" height="3" rx="1.5" fill="currentColor"></rect><rect x="5" y="6.5" width="8" height="3" rx="1.5" fill="currentColor" opacity="0.6"></rect><rect x="8.5" y="10.5" width="6" height="3" rx="1.5" fill="currentColor" opacity="0.32"></rect>"#;

/// Execution state derived from a [`PlanStep`]'s `start_ms` and `end_ms` timestamps relative to `now_ms`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanStepStatus {
    /// Finished (`start_ms` and `end_ms` reached).
    Done,
    /// Started (`start_ms <= now_ms`) and still growing toward `now_ms`.
    Running,
    /// Waiting on upstream dependencies (`start_ms` not yet reached).
    Waiting,
}

/// One step in [`PlanVsActual`] ordered after any steps it depends on.
#[derive(Clone, Debug, PartialEq)]
pub struct PlanStep {
    /// Unique step identifier referenced by downstream `deps`.
    pub id: SharedString,
    /// Step title (`"Build the layout"`).
    pub title: SharedString,
    /// Optional Chinese translation.
    pub title_zh: Option<SharedString>,
    /// Planned duration in milliseconds (`60_000` = `1:00`).
    pub plan_ms: u64,
    /// Upstream step ids that must finish before this step can start.
    pub deps: Vec<SharedString>,
    /// Actual start timestamp in milliseconds from run start (`None` while waiting).
    pub start_ms: Option<u64>,
    /// Actual finish timestamp in milliseconds from run start (`None` while running or waiting).
    pub end_ms: Option<u64>,
    /// Optional app/tool logo key (`"slack"`, `"figma"`, `"stripe"`, `"github"`).
    pub icon: Option<SharedString>,
    /// Planned USD cost for this step.
    pub planned_cost: f64,
    /// Actual USD cost incurred once started/completed.
    pub actual_cost: Option<f64>,
}

impl PlanStep {
    /// Create a [`PlanStep`] with `id`, `title`, and `plan_ms`.
    pub fn new(id: impl Into<SharedString>, title: impl Into<SharedString>, plan_ms: u64) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            title_zh: None,
            plan_ms,
            deps: Vec::new(),
            start_ms: None,
            end_ms: None,
            icon: None,
            planned_cost: 0.12,
            actual_cost: None,
        }
    }

    /// Set a localized Chinese title.
    pub fn with_title_zh(mut self, zh: impl Into<SharedString>) -> Self {
        self.title_zh = Some(zh.into());
        self
    }

    /// Declare upstream dependency step ids.
    pub fn with_deps<I, S>(mut self, deps: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        self.deps = deps.into_iter().map(Into::into).collect();
        self
    }

    /// Set actual start and optional finish timestamps (ms from run start).
    pub fn with_actual(mut self, start_ms: Option<u64>, end_ms: Option<u64>) -> Self {
        self.start_ms = start_ms;
        self.end_ms = end_ms;
        self
    }

    /// Set the app/tool mark key rendered before the step label.
    pub fn with_icon(mut self, icon: impl Into<SharedString>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Set planned and optional actual dollar costs for the header rollup.
    pub fn with_costs(mut self, planned: f64, actual: Option<f64>) -> Self {
        self.planned_cost = planned;
        self.actual_cost = actual;
        self
    }

    fn display_title<'a>(&'a self, cx: &App) -> &'a str {
        if let Some(zh) = &self.title_zh {
            tr(cx, self.title.as_ref(), zh.as_ref())
        } else {
            self.title.as_ref()
        }
    }
}

/// Typed events emitted by [`PlanVsActual`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlanVsActualEvent {
    /// User focused or hovered a step (`None` when cleared).
    FocusStep(Option<SharedString>),
    /// Replay started from `0:00`.
    ReplayStarted,
}

impl EventEmitter<PlanVsActualEvent> for PlanVsActual {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ComputedStepSchedule {
    pub status: PlanStepStatus,
    pub plan_start_ms: u64,
    pub plan_end_ms: u64,
    pub actual_start_ms: u64,
    pub actual_end_ms: u64,
    pub base_dur_ms: u64,
    pub over_dur_ms: u64,
    pub delta_ms: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ComputedPlanSchedule {
    pub rows: Vec<ComputedStepSchedule>,
    pub planned_finish_ms: u64,
    pub projected_finish_ms: u64,
    pub axis_range_ms: u64,
}

/// Compute planned windows, actual/running bars, overrun slices, downstream forecast shifts, and finish markers.
pub(crate) fn compute_schedule(steps: &[PlanStep], now_ms: u64) -> ComputedPlanSchedule {
    if steps.is_empty() {
        return ComputedPlanSchedule {
            rows: Vec::new(),
            planned_finish_ms: 0,
            projected_finish_ms: 0,
            axis_range_ms: 60_000,
        };
    }

    let mut id_map: HashMap<&str, usize> = HashMap::with_capacity(steps.len());
    for (idx, s) in steps.iter().enumerate() {
        id_map.insert(s.id.as_ref(), idx);
    }

    let mut rows: Vec<ComputedStepSchedule> = Vec::with_capacity(steps.len());
    for (idx, step) in steps.iter().enumerate() {
        let plan_start = step
            .deps
            .iter()
            .filter_map(|d| id_map.get(d.as_ref()).copied())
            .filter(|&di| di < idx)
            .map(|di| rows[di].plan_end_ms)
            .max()
            .unwrap_or(0);
        let plan_end = plan_start + step.plan_ms;

        let dep_expected_end = step
            .deps
            .iter()
            .filter_map(|d| id_map.get(d.as_ref()).copied())
            .filter(|&di| di < idx)
            .map(|di| {
                let r = &rows[di];
                match r.status {
                    PlanStepStatus::Done => r.actual_end_ms,
                    PlanStepStatus::Running => {
                        r.actual_end_ms.max(r.actual_start_ms + steps[di].plan_ms)
                    }
                    PlanStepStatus::Waiting => r.actual_end_ms,
                }
            })
            .max()
            .unwrap_or(0);

        let (status, actual_start, actual_end, base_dur, over_dur, delta_ms) = match (
            step.start_ms.filter(|&s| s <= now_ms),
            step.end_ms.filter(|&e| e <= now_ms),
        ) {
            (Some(s), Some(e)) => {
                let dur = e.saturating_sub(s);
                let base = dur.min(step.plan_ms);
                let over = dur.saturating_sub(step.plan_ms);
                let delta = dur as i64 - step.plan_ms as i64;
                (PlanStepStatus::Done, s, e, base, over, delta)
            }
            (Some(s), None) => {
                let elapsed = now_ms.saturating_sub(s);
                let base = elapsed.min(step.plan_ms);
                let over = elapsed.saturating_sub(step.plan_ms);
                let delta = if over > 0 { over as i64 } else { 0 };
                (PlanStepStatus::Running, s, now_ms, base, over, delta)
            }
            _ => {
                let forecast_start = dep_expected_end.max(now_ms);
                let forecast_end = forecast_start + step.plan_ms;
                (
                    PlanStepStatus::Waiting,
                    forecast_start,
                    forecast_end,
                    step.plan_ms,
                    0,
                    0,
                )
            }
        };

        rows.push(ComputedStepSchedule {
            status,
            plan_start_ms: plan_start,
            plan_end_ms: plan_end,
            actual_start_ms: actual_start,
            actual_end_ms: actual_end,
            base_dur_ms: base_dur,
            over_dur_ms: over_dur,
            delta_ms,
        });
    }

    let planned_finish = rows.iter().map(|r| r.plan_end_ms).max().unwrap_or(0);
    let projected_finish = rows
        .iter()
        .enumerate()
        .map(|(i, r)| match r.status {
            PlanStepStatus::Done | PlanStepStatus::Waiting => r.actual_end_ms,
            PlanStepStatus::Running => r.actual_end_ms.max(r.actual_start_ms + steps[i].plan_ms),
        })
        .max()
        .unwrap_or(planned_finish);

    let raw_max = planned_finish.max(projected_finish).max(now_ms) + 20_000;
    let axis_range = raw_max.div_ceil(30_000).max(2) * 30_000;

    ComputedPlanSchedule {
        rows,
        planned_finish_ms: planned_finish,
        projected_finish_ms: projected_finish,
        axis_range_ms: axis_range,
    }
}

/// Format milliseconds into `m:ss` axis and bar notation (`0:42`, `2:55`).
pub fn format_plan_ms(ms: u64) -> String {
    let secs = ms / 1_000;
    format!("{}:{:02}", secs / 60, secs % 60)
}

fn format_signed_delta_secs(delta_ms: i64) -> String {
    let secs = delta_ms / 1_000;
    if secs > 0 {
        format!("+{secs}s")
    } else {
        format!("−{}s", secs.unsigned_abs())
    }
}

fn default_pricing_plan_steps() -> Vec<PlanStep> {
    // Full end-to-end event timeline (`0..202_000ms`), evaluated against `now_ms`:
    // At default `now_ms = 148_000` (`2:28`):
    // - `brief` finished 4s early (`0..16s` vs `20s`)
    // - `copy` finished 4s early (`16..52s` vs `40s`)
    // - `layout` overran by +22s (`16..98s` = `82s` vs `60s` plan -> amber overrun!)
    // - `tests` finished (`52..84s` = `32s` vs `30s`)
    // - `stripe` started late at `98s` and at `now = 148s` has run `50s` vs `45s` plan (live amber overrun!)
    // - `fix` and `pr` slide right past `Plan 2:55` to `ETA 3:18`!
    vec![
        PlanStep::new("brief", "Read the brief", 20_000)
            .with_title_zh("阅读需求简报")
            .with_icon("slack")
            .with_actual(Some(0), Some(16_000))
            .with_costs(0.06, Some(0.05)),
        PlanStep::new("copy", "Draft the copy", 40_000)
            .with_title_zh("起草页面文案")
            .with_deps(["brief"])
            .with_icon("notion")
            .with_actual(Some(16_000), Some(52_000))
            .with_costs(0.14, Some(0.12)),
        PlanStep::new("layout", "Build the layout", 60_000)
            .with_title_zh("构建响应式布局")
            .with_deps(["brief"])
            .with_icon("figma")
            .with_actual(Some(16_000), Some(98_000))
            .with_costs(0.24, Some(0.33)),
        PlanStep::new("tests", "Write the tests", 30_000)
            .with_title_zh("编写单元测试")
            .with_deps(["copy"])
            .with_icon("github")
            .with_actual(Some(52_000), Some(84_000))
            .with_costs(0.15, Some(0.16)),
        PlanStep::new("stripe", "Wire up Stripe", 45_000)
            .with_title_zh("接入 Stripe 支付")
            .with_deps(["layout"])
            .with_icon("stripe")
            .with_actual(Some(98_000), Some(160_000))
            .with_costs(0.22, Some(0.29)),
        PlanStep::new("fix", "Run tests and fix", 35_000)
            .with_title_zh("执行测试并修复")
            .with_deps(["tests", "stripe"])
            .with_icon("github")
            .with_actual(Some(160_000), Some(192_000))
            .with_costs(0.15, Some(0.14)),
        PlanStep::new("pr", "Open the pull request", 15_000)
            .with_title_zh("发起合并请求")
            .with_deps(["fix"])
            .with_icon("github")
            .with_actual(Some(192_000), Some(206_000))
            .with_costs(0.07, Some(0.07)),
    ]
}

/// Planned-vs-actual Gantt chart for an autonomous agent run.
pub struct PlanVsActual {
    id: ElementId,
    title: Option<SharedString>,
    task: Option<SharedString>,
    steps: Vec<PlanStep>,
    base_now_ms: u64,
    live: bool,
    replay_mode: bool,
    active_step_idx: Option<usize>,
    open_step_idx: Option<usize>,
    track_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    scrubber_frac: Option<f32>,
    scrubber_slide: Slide,
    started_at: Instant,
    loading: bool,
    empty: bool,
}

impl PlanVsActual {
    /// Create a [`PlanVsActual`] Gantt chart on `"Ship the pricing page"` showing live amber overruns and shifted downstream steps.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        Self {
            id: id.into(),
            title: None,
            task: Some("Ship the pricing page".into()),
            steps: default_pricing_plan_steps(),
            base_now_ms: 149_000,
            live: true,
            replay_mode: false,
            active_step_idx: None,
            open_step_idx: None,
            track_bounds: Rc::new(Cell::new(None)),
            scrubber_frac: None,
            scrubber_slide: Slide::snappy(0.5),
            started_at: Instant::now(),
            loading: false,
            empty: false,
        }
    }

    /// Override the top bar heading (`"Plan vs actual"` by default).
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the task subtitle displayed next to the heading.
    pub fn with_task(mut self, task: impl Into<SharedString>) -> Self {
        self.task = Some(task.into());
        self
    }

    /// Supply custom ordered steps (place each step after the steps it depends on).
    pub fn with_steps(mut self, steps: Vec<PlanStep>) -> Self {
        self.steps = steps;
        self
    }

    /// Set the run clock position in milliseconds from run start.
    pub fn with_now(mut self, now_ms: u64) -> Self {
        self.base_now_ms = now_ms;
        self.replay_mode = false;
        self
    }

    /// Enable or pause live clock progression.
    pub fn with_live(mut self, live: bool) -> Self {
        self.live = live;
        self
    }

    /// Render skeleton placeholder rows.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Force the empty state view.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Current simulated `now_ms` timestamp.
    pub fn current_now_ms(&self, reduced: bool) -> u64 {
        if self.replay_mode && !reduced {
            // Replay 3.5-minute run smoothly in ~10 seconds
            let ms = (self.started_at.elapsed().as_secs_f32() * 21_000.0) as u64;
            ms.min(208_000)
        } else if self.live && !reduced {
            let extra = (self.started_at.elapsed().as_secs_f32() * 1_800.0) as u64;
            (self.base_now_ms + extra).min(208_000)
        } else {
            self.base_now_ms
        }
    }

    /// Replay the entire plan-vs-actual timeline from `0:00`.
    pub fn replay(&mut self, cx: &mut Context<Self>) {
        self.replay_mode = true;
        self.live = true;
        self.started_at = Instant::now();
        cx.emit(PlanVsActualEvent::ReplayStarted);
        cx.notify();
    }
}

fn paint_dashed_ghost_rect(bounds: Bounds<Pixels>, color: Hsla, win: &mut Window) {
    let x0 = f32::from(bounds.left());
    let y0 = f32::from(bounds.top());
    let w = f32::from(bounds.size.width);
    let h = f32::from(bounds.size.height);
    if w <= 2.0 || h <= 2.0 {
        return;
    }
    let dash = 3.5f32;
    let gap = 2.8f32;
    let step = dash + gap;

    // Top and bottom horizontal dashed edges
    let nx = ((w / step).ceil() as usize).max(1);
    for i in 0..nx {
        let sx = x0 + (i as f32) * step;
        let ex = (sx + dash).min(x0 + w);
        if ex > sx {
            for &y in &[y0 + 0.5, y0 + h - 0.5] {
                let mut pb = PathBuilder::stroke(px(1.0));
                pb.move_to(point(px(sx), px(y)));
                pb.line_to(point(px(ex), px(y)));
                if let Ok(path) = pb.build() {
                    win.paint_path(path, color);
                }
            }
        }
    }
    // Left and right vertical caps
    for &x in &[x0 + 0.5, x0 + w - 0.5] {
        let mut pb = PathBuilder::stroke(px(1.0));
        pb.move_to(point(px(x), px(y0 + 1.5)));
        pb.line_to(point(px(x), px(y0 + h - 1.5)));
        if let Ok(path) = pb.build() {
            win.paint_path(path, color);
        }
    }
}

impl Render for PlanVsActual {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let reduced = disabled_motion(cx) || Theme::global(cx).reduced_motion;
        let now_ms = self.current_now_ms(reduced);
        let sched = compute_schedule(&self.steps, now_ms);

        let any_running = sched
            .rows
            .iter()
            .any(|r| r.status == PlanStepStatus::Running);
        if ((self.live || self.replay_mode) && any_running && !reduced)
            || (self.loading && !reduced)
        {
            window.request_animation_frame();
        }
        if self.replay_mode && !any_running && now_ms >= sched.projected_finish_ms {
            self.replay_mode = false;
            self.base_now_ms = now_ms;
        }

        let card_bg: Hsla = if p.dark {
            rgb(0x1c1c1f).into()
        } else {
            rgb(0xffffff).into()
        };
        let card_border: Hsla = if p.dark {
            hsla(0., 0., 1., 0.10)
        } else {
            hsla(0., 0., 0., 0.065)
        };
        let accent: Hsla = if p.dark {
            rgb(0x4fb0fe).into()
        } else {
            rgba(0x0060e2c4).into()
        };
        let ghost_c: Hsla = if p.dark {
            hsla(0., 0., 1., 0.30)
        } else {
            hsla(0., 0., 0., 0.26)
        };
        let actual_bg: Hsla = if p.dark {
            p.purple.opacity(0.20)
        } else {
            p.purple.opacity(0.13)
        };
        let actual_edge: Hsla = if p.dark {
            p.purple.opacity(0.45)
        } else {
            p.purple.opacity(0.35)
        };
        let over_bg: Hsla = if p.dark {
            p.orange.opacity(0.22)
        } else {
            p.orange.opacity(0.17)
        };
        let over_edge: Hsla = if p.dark {
            p.orange.opacity(0.50)
        } else {
            p.orange.opacity(0.44)
        };
        let over_ink = p.amber_ink;
        let forecast_bg: Hsla = if p.dark {
            hsla(0., 0., 1., 0.05)
        } else {
            hsla(0., 0., 0., 0.038)
        };
        let forecast_edge: Hsla = if p.dark {
            hsla(0., 0., 1., 0.10)
        } else {
            hsla(0., 0., 0., 0.08)
        };

        let heading_str: SharedString = self
            .title
            .clone()
            .unwrap_or_else(|| tr(cx, "Plan vs actual", "计划与实际对比").into());
        let task_str: SharedString = self
            .task
            .as_deref()
            .map(|t| {
                if t == "Ship the pricing page" {
                    tr(cx, "Ship the pricing page", "上线定价页面")
                } else {
                    t
                }
            })
            .unwrap_or("")
            .into();

        let planned_cost_sum: f64 = self.steps.iter().map(|s| s.planned_cost).sum();
        let actual_cost_sum: f64 = self
            .steps
            .iter()
            .zip(&sched.rows)
            .map(|(s, r)| match r.status {
                PlanStepStatus::Done => s.actual_cost.unwrap_or(s.planned_cost),
                PlanStepStatus::Running => {
                    let frac = ((r.actual_end_ms.saturating_sub(r.actual_start_ms)) as f64
                        / (s.plan_ms.max(1) as f64))
                        .clamp(0.0, 1.35);
                    s.planned_cost * frac
                }
                PlanStepStatus::Waiting => 0.0,
            })
            .sum();

        let finish_diff_ms = sched.projected_finish_ms as i64 - sched.planned_finish_ms as i64;
        let finish_diff_secs = finish_diff_ms / 1_000;
        let (verdict_bg, verdict_fg, verdict_text) = if finish_diff_secs > 2 {
            let lbl = if tr(cx, "en", "zh") == "zh" {
                format!("延迟 {finish_diff_secs}s")
            } else {
                format!("{finish_diff_secs}s behind")
            };
            (over_bg, over_ink, lbl)
        } else if finish_diff_secs < -2 {
            let ahead = finish_diff_secs.unsigned_abs();
            let lbl = if tr(cx, "en", "zh") == "zh" {
                format!("提前 {ahead}s")
            } else {
                format!("{ahead}s ahead")
            };
            (p.green_soft, p.green_ink, lbl)
        } else {
            (p.chip, p.text, tr(cx, "On plan", "符合计划").to_string())
        };

        let shown_elapsed = now_ms.min(sched.projected_finish_ms);
        let header =
            div()
                .px(px(16.))
                .py(px(12.))
                .border_b_1()
                .border_color(p.line)
                .flex()
                .items_center()
                .gap(px(8.))
                .child(pro_svg("0 0 16 16", HEADER_GANTT_SVG, 16., accent))
                .child(
                    div()
                        .flex_none()
                        .text_size(px(13.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.strong)
                        .child(heading_str),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_size(px(13.))
                        .text_color(p.muted)
                        .child(task_str),
                )
                .child(
                    div()
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap(px(14.))
                        .mr(px(4.))
                        .text_size(px(12.))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(5.))
                                .child(div().text_color(p.muted).child(tr(cx, "Time", "耗时")))
                                .child(
                                    div()
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(if finish_diff_secs > 2 {
                                            over_ink
                                        } else {
                                            p.ink
                                        })
                                        .child(format_plan_ms(shown_elapsed)),
                                )
                                .child(div().text_color(p.muted).child(format!(
                                    "/ {}",
                                    format_plan_ms(sched.planned_finish_ms)
                                ))),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(5.))
                                .child(div().text_color(p.muted).child(tr(cx, "Cost", "成本")))
                                .child(pro_rolling_number(
                                    pro_eid(&self.id, "pva-cost-act"),
                                    actual_cost_sum,
                                    2,
                                    "$",
                                    "",
                                    12.0,
                                    FontWeight::MEDIUM,
                                    if actual_cost_sum > planned_cost_sum + 0.02 {
                                        over_ink
                                    } else {
                                        p.ink
                                    },
                                ))
                                .child(
                                    div()
                                        .text_color(p.muted)
                                        .child(format!("/ ${planned_cost_sum:.2}")),
                                ),
                        ),
                )
                .child(
                    div()
                        .min_w(px(90.))
                        .h(px(22.))
                        .px(px(8.))
                        .flex_none()
                        .rounded(px(6.))
                        .bg(verdict_bg)
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(pro_swap_label(
                            pro_eid(&self.id, "pva-verdict"),
                            verdict_text,
                            12.0,
                            FontWeight::MEDIUM,
                            verdict_fg,
                        )),
                );

        if self.loading {
            let phase = if reduced {
                0.5
            } else {
                (self.started_at.elapsed().as_secs_f32() * 0.9).fract()
            };
            let mut skel = div().p(px(16.)).flex().flex_col().gap(px(8.));
            for i in 0..5usize {
                skel = skel.child(
                    div()
                        .h(px(26.))
                        .flex()
                        .items_center()
                        .gap(px(16.))
                        .child(pro_skeleton_bar(160., 12., 4., phase + i as f32 * 0.1, cx))
                        .child(pro_skeleton_bar(
                            180. + (i as f32 * 30.),
                            14.,
                            4.,
                            phase + i as f32 * 0.1,
                            cx,
                        )),
                );
            }
            return div()
                .id(self.id.clone())
                .w_full()
                .flex()
                .justify_center()
                .child(
                    div()
                        .w_full()
                        .max_w(px(756.))
                        .rounded(px(12.))
                        .bg(card_bg)
                        .border_1()
                        .border_color(card_border)
                        .when(!p.dark, |d| d.shadow_sm())
                        .flex()
                        .flex_col()
                        .child(header)
                        .child(skel),
                )
                .into_any_element();
        }

        if self.empty || self.steps.is_empty() {
            let empty_el = pro_empty_state(
                ProIcon::Tasks,
                tr(cx, "No planned steps", "暂无计划步骤"),
                tr(
                    cx,
                    "Supply steps with planned durations to compare execution against the baseline plan.",
                    "提供包含预计耗时的步骤后，即可在此对比计划基准与实际执行进度。",
                ),
                None,
                cx,
            );
            return div()
                .id(self.id.clone())
                .w_full()
                .flex()
                .justify_center()
                .child(
                    div()
                        .w_full()
                        .max_w(px(756.))
                        .rounded(px(12.))
                        .bg(card_bg)
                        .border_1()
                        .border_color(card_border)
                        .when(!p.dark, |d| d.shadow_sm())
                        .flex()
                        .flex_col()
                        .child(header)
                        .child(empty_el),
                )
                .into_any_element();
        }

        // Smoothly glide axis range via `Slide` when overruns push projected finish wider!
        let smooth_range_ms = pro_spring_f32(
            pro_eid(&self.id, "axis-range"),
            sched.axis_range_ms as f32,
            0.42,
            window,
            cx,
        )
        .max(30_000.0);

        let focus_target = self.active_step_idx.or(self.open_step_idx);

        // Determine connected upstream/downstream steps when a row is hovered or inspected
        let mut lit_indices: Vec<bool> = vec![true; self.steps.len()];
        let mut dep_edges: Vec<(usize, usize)> = Vec::new();
        if let Some(focus_i) = focus_target
            && focus_i < self.steps.len()
        {
            lit_indices.fill(false);
            lit_indices[focus_i] = true;
            let focus_id = &self.steps[focus_i].id;
            for (idx, st) in self.steps.iter().enumerate() {
                if idx < focus_i && self.steps[focus_i].deps.iter().any(|d| d == &st.id) {
                    lit_indices[idx] = true;
                    dep_edges.push((idx, focus_i));
                }
                if idx > focus_i && st.deps.iter().any(|d| d == focus_id) {
                    lit_indices[idx] = true;
                    dep_edges.push((focus_i, idx));
                }
            }
        }

        let scrub_x = self.scrubber_slide.get(window, cx).clamp(0.0, 1.0);
        let scrub_alpha = pro_spring_f32(
            pro_eid(&self.id, "pva-scrub-a"),
            if self.scrubber_frac.is_some() {
                1.0
            } else {
                0.0
            },
            0.18,
            window,
            cx,
        );
        let dep_alpha = pro_spring_f32(
            pro_eid(&self.id, "pva-dep-a"),
            if !dep_edges.is_empty() { 1.0 } else { 0.0 },
            0.20,
            window,
            cx,
        );

        // Axis header row (`STEP` on left + time ticks every 30s + smooth scrubber badge on right)
        let tick_count = (sched.axis_range_ms / 30_000) as usize;
        let mut axis_ticks = div().flex_1().h_full().relative();
        for t in 1..=tick_count {
            let ms = (t as u64) * 30_000;
            let frac = (ms as f32 / smooth_range_ms).clamp(0., 1.);
            if frac <= 0.96 {
                axis_ticks = axis_ticks.child(
                    div()
                        .absolute()
                        .left(relative(frac))
                        .top(px(7.))
                        .pl(px(5.))
                        .text_size(px(11.5))
                        .line_height(px(16.))
                        .text_color(p.muted)
                        .child(format_plan_ms(ms)),
                );
            }
        }
        if scrub_alpha > 0.01 {
            let scrub_ms = (scrub_x * smooth_range_ms).round() as u64;
            axis_ticks = axis_ticks.child(
                div()
                    .absolute()
                    .left(relative(scrub_x.clamp(0.02, 0.92)))
                    .top(px(5.))
                    .ml(px(-16.))
                    .h(px(19.))
                    .px(px(6.))
                    .rounded(px(5.))
                    .bg(card_bg)
                    .border_1()
                    .border_color(p.line_strong)
                    .opacity(scrub_alpha)
                    .text_size(px(11.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.ink)
                    .flex()
                    .items_center()
                    .child(format_plan_ms(scrub_ms)),
            );
        }

        let axis_bar = div()
            .w_full()
            .h(px(AXIS_H))
            .pl(px(16.))
            .pr(px(16.))
            .border_b_1()
            .border_color(p.line)
            .flex()
            .items_center()
            .gap(px(14.))
            .child(
                div()
                    .w(px(NAME_COL_W))
                    .flex_none()
                    .text_size(px(10.5))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.muted)
                    .child(tr(cx, "STEP", "执行步骤")),
            )
            .child(axis_ticks);

        // Background track overlay canvas: vertical gridlines, Plan vs ETA gap wash, Plan dashed line, ETA line, Now line, Scrubber line, and Dependency elbows!
        let n_rows = self.steps.len();
        let rows_total_h = (n_rows as f32) * ROW_H;
        let sched_for_bg = sched.clone();
        let dep_edges_bg = dep_edges.clone();
        let grid_c: Hsla = if p.dark {
            hsla(0., 0., 1., 0.05)
        } else {
            hsla(0., 0., 0., 0.04)
        };
        let late_gap_bg: Hsla = if p.dark {
            p.orange.opacity(0.07)
        } else {
            p.orange.opacity(0.065)
        };
        let early_gap_bg: Hsla = p.green.opacity(0.06);
        let link_c: Hsla = if p.dark {
            hsla(0., 0., 1., 0.55)
        } else {
            hsla(0., 0., 0., 0.48)
        };
        let scrub_line_c = p.ink.opacity(0.32);
        let now_line_c = accent.opacity(0.55);
        let show_now = any_running && (now_ms as f32) < smooth_range_ms;
        let bounds_for_prepaint = self.track_bounds.clone();

        let track_overlay = canvas(
            move |bounds, _, _| {
                bounds_for_prepaint.set(Some(bounds));
            },
            move |bounds, _, win, _| {
                let tw = f32::from(bounds.size.width);
                let th = f32::from(bounds.size.height);
                if tw <= 4.0 || th <= 4.0 {
                    return;
                }
                let ox = f32::from(bounds.left());
                let oy = f32::from(bounds.top());

                // 30s vertical hairlines
                let ticks = (sched_for_bg.axis_range_ms / 30_000) as usize;
                for t in 1..=ticks {
                    let u = ((t as f32) * 30_000.0 / smooth_range_ms).clamp(0., 1.);
                    if u < 0.995 {
                        let gx = ox + u * tw;
                        let mut pb = PathBuilder::stroke(px(1.0));
                        pb.move_to(point(px(gx), px(oy)));
                        pb.line_to(point(px(gx), px(oy + th)));
                        if let Ok(path) = pb.build() {
                            win.paint_path(path, grid_c);
                        }
                    }
                }

                // Tinted gap between Planned finish and Projected finish (`ETA`)
                let plan_x = ox
                    + (sched_for_bg.planned_finish_ms as f32 / smooth_range_ms).clamp(0., 1.) * tw;
                let eta_x = ox
                    + (sched_for_bg.projected_finish_ms as f32 / smooth_range_ms).clamp(0., 1.)
                        * tw;
                if (eta_x - plan_x).abs() > 2.0 {
                    let x_left = plan_x.min(eta_x);
                    let w_gap = (eta_x - plan_x).abs();
                    let gap_c = if eta_x > plan_x {
                        late_gap_bg
                    } else {
                        early_gap_bg
                    };
                    win.paint_quad(fill(
                        Bounds::new(point(px(x_left), px(oy)), size(px(w_gap), px(th))),
                        gap_c,
                    ));
                }

                // Planned finish vertical dashed line
                let nd = ((th / 5.5) as usize).max(1);
                for d in 0..nd {
                    let y0 = oy + (d as f32) * 5.5;
                    let y1 = (y0 + 3.0).min(oy + th);
                    let mut pb = PathBuilder::stroke(px(1.0));
                    pb.move_to(point(px(plan_x), px(y0)));
                    pb.line_to(point(px(plan_x), px(y1)));
                    if let Ok(path) = pb.build() {
                        win.paint_path(path, ghost_c);
                    }
                }

                // Projected ETA vertical line (solid amber when late, green when early)
                if (eta_x - plan_x).abs() > 2.0 {
                    let eta_c = if eta_x > plan_x {
                        over_edge
                    } else {
                        p.green.opacity(0.50)
                    };
                    let mut pb = PathBuilder::stroke(px(1.2));
                    pb.move_to(point(px(eta_x), px(oy)));
                    pb.line_to(point(px(eta_x), px(oy + th)));
                    if let Ok(path) = pb.build() {
                        win.paint_path(path, eta_c);
                    }
                }

                // Smooth `Slide::snappy` time-scrubber crosshair on hover
                if scrub_alpha > 0.01 {
                    let sx = ox + scrub_x * tw;
                    let mut spb = PathBuilder::stroke(px(1.0));
                    spb.move_to(point(px(sx), px(oy)));
                    spb.line_to(point(px(sx), px(oy + th)));
                    if let Ok(path) = spb.build() {
                        win.paint_path(path, scrub_line_c.opacity(scrub_line_c.a * scrub_alpha));
                    }
                }

                // Live `now` cursor line + top halo dot
                if show_now {
                    let nx = ox + (now_ms as f32 / smooth_range_ms).clamp(0., 1.) * tw;
                    let mut pb = PathBuilder::stroke(px(1.1));
                    pb.move_to(point(px(nx), px(oy)));
                    pb.line_to(point(px(nx), px(oy + th)));
                    if let Ok(path) = pb.build() {
                        win.paint_path(path, now_line_c);
                    }
                    win.paint_quad(
                        fill(
                            Bounds::new(point(px(nx - 2.5), px(oy - 2.5)), size(px(5.), px(5.))),
                            accent,
                        )
                        .corner_radii(px(2.5)),
                    );
                }

                // Dependency elbow links on hover/focus (`from_row -> to_row`)
                let lc = link_c.opacity(link_c.a * dep_alpha);
                for &(from_i, to_i) in &dep_edges_bg {
                    let r_from = &sched_for_bg.rows[from_i];
                    let r_to = &sched_for_bg.rows[to_i];
                    let a1_x =
                        ox + (r_from.actual_end_ms as f32 / smooth_range_ms).clamp(0., 1.) * tw;
                    let b0_x =
                        ox + (r_to.actual_start_ms as f32 / smooth_range_ms).clamp(0., 1.) * tw;
                    let start_x = a1_x.min(b0_x - 6.0).max(ox + 2.0);
                    let start_y = oy + (from_i as f32) * ROW_H + ROW_H * 0.5 + BAR_H * 0.5;
                    let end_y = oy + (to_i as f32) * ROW_H + ROW_H * 0.5;
                    let end_x = b0_x.max(start_x + 4.0);

                    let mut link = PathBuilder::stroke(px(1.15));
                    link.move_to(point(px(start_x), px(start_y)));
                    link.line_to(point(px(start_x), px(end_y)));
                    link.line_to(point(px(end_x), px(end_y)));
                    if let Ok(path) = link.build() {
                        win.paint_path(path, lc);
                    }
                    // Terminal square anchor dots
                    win.paint_quad(quad(
                        Bounds::new(
                            point(px(start_x - 2.2), px(start_y - 2.2)),
                            size(px(4.4), px(4.4)),
                        ),
                        Corners::all(px(1.)),
                        card_bg,
                        Edges::all(px(1.)),
                        lc,
                        BorderStyle::Solid,
                    ));
                    win.paint_quad(
                        fill(
                            Bounds::new(
                                point(px(end_x - 2.2), px(end_y - 2.2)),
                                size(px(4.4), px(4.4)),
                            ),
                            lc,
                        )
                        .corner_radii(px(1.)),
                    );
                }
            },
        )
        .absolute()
        .top_0()
        .bottom_0()
        .left(px(16.0 + NAME_COL_W + 14.0))
        .right(px(16.0));

        let spin_angle = if reduced {
            -std::f32::consts::FRAC_PI_4
        } else {
            (self.started_at.elapsed().as_secs_f32() / 0.90).fract() * TAU
                - std::f32::consts::FRAC_PI_2
        };

        let bounds_for_move = self.track_bounds.clone();
        let mut rows_col = div()
            .id(pro_eid(&self.id, "pva-rows-box"))
            .w_full()
            .relative()
            .flex()
            .flex_col()
            .on_mouse_move(cx.listener(move |this, ev: &MouseMoveEvent, _, cx| {
                if let Some(b) = bounds_for_move.get() {
                    let bw = f32::from(b.size.width);
                    if bw > 8.0 && ev.position.x >= b.left() && ev.position.x <= b.right() {
                        let frac = ((f32::from(ev.position.x - b.left())) / bw).clamp(0.0, 1.0);
                        this.scrubber_frac = Some(frac);
                        this.scrubber_slide.set(frac, cx);
                        cx.notify();
                    } else if this.scrubber_frac.is_some() {
                        this.scrubber_frac = None;
                        cx.notify();
                    }
                }
            }))
            .on_hover(cx.listener(|this, inside: &bool, _, cx| {
                if !*inside && this.scrubber_frac.is_some() {
                    this.scrubber_frac = None;
                    cx.notify();
                }
            }))
            .child(track_overlay);

        for (idx, step) in self.steps.iter().enumerate() {
            let r = &sched.rows[idx];
            let sid = step.id.clone();
            let is_lit = lit_indices[idx];
            let is_focused = focus_target == Some(idx);
            let is_open = self.open_step_idx == Some(idx);

            let row_lit_alpha = pro_spring_f32(
                pro_eid(&self.id, format!("pva-lit-{sid}")),
                if is_lit { 1.0 } else { 0.42 },
                0.22,
                window,
                cx,
            );
            let hov_amt = pro_spring_f32(
                pro_eid(&self.id, format!("pva-hov-{sid}")),
                if is_focused { 1.0 } else { 0.0 },
                0.18,
                window,
                cx,
            );

            // Smoothly spring `actual_start_ms` and widths so bars and downstream steps glide organically!
            let smooth_actual_start = pro_spring_f32(
                pro_eid(&self.id, format!("bar-s-{sid}")),
                r.actual_start_ms as f32,
                0.32,
                window,
                cx,
            );
            let smooth_base_dur = pro_spring_f32(
                pro_eid(&self.id, format!("bar-bw-{sid}")),
                r.base_dur_ms as f32,
                0.28,
                window,
                cx,
            );
            let smooth_over_dur = pro_spring_f32(
                pro_eid(&self.id, format!("bar-ow-{sid}")),
                r.over_dur_ms as f32,
                0.28,
                window,
                cx,
            );

            let ghost_left = (r.plan_start_ms as f32 / smooth_range_ms).clamp(0., 1.);
            let ghost_w = (step.plan_ms as f32 / smooth_range_ms).clamp(0.008, 1.0 - ghost_left);

            let act_left = (smooth_actual_start / smooth_range_ms).clamp(0., 1.);
            let base_w = (smooth_base_dur / smooth_range_ms).clamp(0.004, 1.0 - act_left);
            let over_left = (act_left + base_w).clamp(0., 1.);
            let over_w = (smooth_over_dur / smooth_range_ms).clamp(0., 1.0 - over_left);
            let tag_left = (act_left + base_w + over_w + 0.008).clamp(0., 0.84);
            let row_bar_h = BAR_H + 1.5 * hov_amt;

            let state_icon: AnyElement = match r.status {
                PlanStepStatus::Done => pro_check_draw(
                    pro_eid(&self.id, format!("pva-chk-{sid}")),
                    true,
                    14.0,
                    false,
                    p.green_ink,
                ),
                PlanStepStatus::Running => canvas(
                    |_, _, _| (),
                    move |bounds, _, win, _| {
                        let c = bounds.center();
                        let rad = 5.0;
                        let mut trk = PathBuilder::stroke(px(1.5));
                        for i in 0..=20 {
                            let a = (i as f32 / 20.0) * TAU;
                            let pt = point(c.x + px(rad * a.cos()), c.y + px(rad * a.sin()));
                            if i == 0 {
                                trk.move_to(pt);
                            } else {
                                trk.line_to(pt);
                            }
                        }
                        if let Ok(path) = trk.build() {
                            win.paint_path(path, accent.opacity(0.24));
                        }
                        let mut arc = PathBuilder::stroke(px(1.55));
                        for i in 0..=8 {
                            let a = spin_angle + (i as f32 / 8.0) * (TAU * 0.28);
                            let pt = point(c.x + px(rad * a.cos()), c.y + px(rad * a.sin()));
                            if i == 0 {
                                arc.move_to(pt);
                            } else {
                                arc.line_to(pt);
                            }
                        }
                        if let Ok(path) = arc.build() {
                            win.paint_path(path, accent);
                        }
                    },
                )
                .size(px(14.))
                .flex_none()
                .into_any_element(),
                PlanStepStatus::Waiting => canvas(
                    |_, _, _| (),
                    move |bounds, _, win, _| {
                        let c = bounds.center();
                        let rad = 4.5;
                        for d in 0..7usize {
                            let a0 = (d as f32 / 7.0) * TAU;
                            let a1 = a0 + (TAU / 7.0) * 0.52;
                            let mut seg = PathBuilder::stroke(px(1.25));
                            seg.move_to(point(c.x + px(rad * a0.cos()), c.y + px(rad * a0.sin())));
                            seg.line_to(point(c.x + px(rad * a1.cos()), c.y + px(rad * a1.sin())));
                            if let Ok(path) = seg.build() {
                                win.paint_path(path, p.faint);
                            }
                        }
                    },
                )
                .size(px(14.))
                .flex_none()
                .into_any_element(),
            };

            let ghost_el = canvas(
                |_, _, _| (),
                move |bounds, _, win, _| {
                    paint_dashed_ghost_rect(bounds, ghost_c, win);
                },
            )
            .absolute()
            .top(px((ROW_H - BAR_H) * 0.5))
            .left(relative(ghost_left))
            .w(relative(ghost_w))
            .h(px(BAR_H));

            let bar_el: AnyElement = match r.status {
                PlanStepStatus::Waiting => div()
                    .absolute()
                    .top(px((ROW_H - row_bar_h) * 0.5))
                    .left(relative(act_left))
                    .w(relative(base_w))
                    .h(px(row_bar_h))
                    .rounded(px(4.))
                    .bg(forecast_bg)
                    .border_1()
                    .border_color(pro_mix_color(forecast_edge, p.line_strong, hov_amt))
                    .into_any_element(),
                PlanStepStatus::Done | PlanStepStatus::Running => {
                    let has_over = r.over_dur_ms > 0;
                    let act_border = pro_mix_color(actual_edge, p.purple, hov_amt);
                    let ov_border = pro_mix_color(over_edge, p.orange, hov_amt);
                    div()
                        .absolute()
                        .inset_0()
                        .child(
                            div()
                                .absolute()
                                .top(px((ROW_H - row_bar_h) * 0.5))
                                .left(relative(act_left))
                                .w(relative(base_w))
                                .h(px(row_bar_h))
                                .rounded_tl(px(4.))
                                .rounded_bl(px(4.))
                                .when(!has_over, |d| d.rounded_tr(px(4.)).rounded_br(px(4.)))
                                .bg(actual_bg)
                                .border_1()
                                .border_color(act_border),
                        )
                        .when(has_over, |d| {
                            d.child(
                                div()
                                    .absolute()
                                    .top(px((ROW_H - row_bar_h) * 0.5))
                                    .left(relative(over_left))
                                    .w(relative(over_w.max(0.005)))
                                    .h(px(row_bar_h))
                                    .rounded_tr(px(4.))
                                    .rounded_br(px(4.))
                                    .bg(over_bg)
                                    .border_1()
                                    .border_color(ov_border),
                            )
                        })
                        .into_any_element()
                }
            };

            // Right-of-bar duration + delta badge
            let tag_el = {
                let dur_shown = match r.status {
                    PlanStepStatus::Waiting => step.plan_ms,
                    PlanStepStatus::Done | PlanStepStatus::Running => {
                        r.actual_end_ms.saturating_sub(r.actual_start_ms)
                    }
                };
                let diff_pill: Option<AnyElement> = if r.delta_ms.abs() >= 1_000 {
                    let is_late = r.delta_ms > 0;
                    Some(
                        div()
                            .h(px(16.))
                            .px(px(4.))
                            .rounded(px(4.))
                            .bg(if is_late { over_bg } else { p.green_soft })
                            .text_color(if is_late { over_ink } else { p.green_ink })
                            .text_size(px(11.))
                            .font_weight(FontWeight::MEDIUM)
                            .flex()
                            .items_center()
                            .child(format_signed_delta_secs(r.delta_ms))
                            .into_any_element(),
                    )
                } else {
                    None
                };

                div()
                    .absolute()
                    .top(px((ROW_H - 18.0) * 0.5))
                    .left(relative(tag_left))
                    .h(px(18.))
                    .px(px(4.))
                    .rounded(px(4.))
                    .bg(if is_focused {
                        transparent_black()
                    } else {
                        card_bg.opacity(0.88)
                    })
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .text_size(px(11.5))
                    .text_color(if r.status == PlanStepStatus::Waiting {
                        p.faint
                    } else {
                        p.ink
                    })
                    .child(format_plan_ms(dur_shown))
                    .when_some(diff_pill, |d, pill| d.child(pill))
            };

            let step_title = step.display_title(cx).to_string();
            let focus_sid = sid.clone();
            let row = div()
                .id(pro_eid(&self.id, format!("pva-row-{sid}")))
                .role(Role::ListItem)
                .aria_label(format!(
                    "{}: {} ({} {})",
                    step_title,
                    format_plan_ms(r.actual_end_ms.saturating_sub(r.actual_start_ms)),
                    tr(cx, "planned", "计划"),
                    format_plan_ms(step.plan_ms)
                ))
                .aria_expanded(is_open)
                .tab_index(0)
                .w_full()
                .h(px(ROW_H))
                .px(px(16.))
                .opacity(row_lit_alpha)
                .bg(pro_mix_color(transparent_black(), p.hover, hov_amt))
                .flex()
                .items_center()
                .gap(px(14.))
                .cursor_pointer()
                .focus_visible(move |s| s.border_1().border_color(p.focus))
                .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                    let next = if *inside { Some(idx) } else { None };
                    if this.active_step_idx != next {
                        this.active_step_idx = next;
                        cx.emit(PlanVsActualEvent::FocusStep(
                            next.map(|_| focus_sid.clone()),
                        ));
                        cx.notify();
                    }
                }))
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.open_step_idx = if this.open_step_idx == Some(idx) {
                        None
                    } else {
                        Some(idx)
                    };
                    cx.notify();
                }))
                .child(
                    div()
                        .w(px(NAME_COL_W))
                        .flex_none()
                        .min_w_0()
                        .flex()
                        .items_center()
                        .gap(px(7.))
                        .child(state_icon)
                        .when_some(step.icon.as_ref(), |d, ic| {
                            d.child(pro_app_tile(ic.as_ref(), 18., cx))
                        })
                        .child(
                            div()
                                .min_w_0()
                                .truncate()
                                .text_size(px(13.))
                                .text_color(if r.status == PlanStepStatus::Waiting {
                                    p.text
                                } else {
                                    p.ink
                                })
                                .child(step_title),
                        ),
                )
                .child(
                    div()
                        .flex_1()
                        .h_full()
                        .relative()
                        .child(ghost_el)
                        .child(bar_el)
                        .child(tag_el),
                );

            rows_col = rows_col.child(row);
        }

        // Finish callout pills (`Plan 2:55` and `ETA 3:26`) anchored right under the track!
        let plan_frac = pro_spring_f32(
            pro_eid(&self.id, "pva-plan-f"),
            (sched.planned_finish_ms as f32 / smooth_range_ms).clamp(0.05, 0.92),
            0.28,
            window,
            cx,
        );
        let eta_frac = pro_spring_f32(
            pro_eid(&self.id, "pva-eta-f"),
            (sched.projected_finish_ms as f32 / smooth_range_ms).clamp(0.05, 0.92),
            0.28,
            window,
            cx,
        );
        let show_separate_eta =
            (finish_diff_ms.abs() >= 2_000) && (eta_frac - plan_frac).abs() > 0.02;

        let finish_strip = div()
            .w_full()
            .h(px(26.))
            .pl(px(16.0 + NAME_COL_W + 14.0))
            .pr(px(16.))
            .relative()
            .child(
                div()
                    .w_full()
                    .h_full()
                    .relative()
                    .child(
                        div()
                            .absolute()
                            .top(px(3.))
                            .left(relative(
                                (plan_frac
                                    - if show_separate_eta && eta_frac > plan_frac {
                                        0.11
                                    } else {
                                        0.05
                                    })
                                .clamp(0., 0.85),
                            ))
                            .h(px(20.))
                            .px(px(6.))
                            .rounded(px(5.))
                            .bg(p.chip)
                            .text_color(p.text)
                            .text_size(px(11.5))
                            .font_weight(FontWeight::MEDIUM)
                            .flex()
                            .items_center()
                            .child(format!(
                                "{} {}",
                                tr(cx, "Plan", "计划"),
                                format_plan_ms(sched.planned_finish_ms)
                            )),
                    )
                    .when(show_separate_eta, |d| {
                        let is_late = finish_diff_ms > 0;
                        d.child(
                            div()
                                .absolute()
                                .top(px(3.))
                                .left(relative(eta_frac.clamp(0.04, 0.86)))
                                .h(px(20.))
                                .px(px(6.))
                                .rounded(px(5.))
                                .bg(if is_late { over_bg } else { p.green_soft })
                                .text_color(if is_late { over_ink } else { p.green_ink })
                                .text_size(px(11.5))
                                .font_weight(FontWeight::MEDIUM)
                                .flex()
                                .items_center()
                                .child(format!(
                                    "{} {}",
                                    tr(cx, "ETA", "预计"),
                                    format_plan_ms(sched.projected_finish_ms)
                                )),
                        )
                    }),
            );

        // Expandable Gantt step inspector (`pro_fold` + `pro_popover_presence`)
        let insp_open = self.open_step_idx.is_some_and(|i| i < self.steps.len());
        let insp_el = {
            let insp_data = self.open_step_idx.and_then(|i| {
                let st = self.steps.get(i)?;
                let rs = sched.rows.get(i)?;
                let title_s = st.display_title(cx).to_string();
                let plan_win = format!(
                    "{} – {} ({})",
                    format_plan_ms(rs.plan_start_ms),
                    format_plan_ms(rs.plan_end_ms),
                    format_plan_ms(st.plan_ms)
                );
                let act_win = format!(
                    "{} – {} ({})",
                    format_plan_ms(rs.actual_start_ms),
                    format_plan_ms(rs.actual_end_ms),
                    format_plan_ms(rs.actual_end_ms.saturating_sub(rs.actual_start_ms))
                );
                let cost_str = match st.actual_cost {
                    Some(ac) => format!("${ac:.2} / ${:.2}", st.planned_cost),
                    None => format!("${:.2} ({})", st.planned_cost, tr(cx, "planned", "计划")),
                };
                let deps_str = if st.deps.is_empty() {
                    tr(cx, "Root step", "根步骤").to_string()
                } else {
                    st.deps
                        .iter()
                        .map(|d| d.as_ref())
                        .collect::<Vec<_>>()
                        .join(", ")
                };
                Some((title_s, plan_win, act_win, cost_str, deps_str, rs.status))
            });
            let pop_id = pro_eid(&self.id, "pva-insp-pop");
            let chk_id = pro_eid(&self.id, "pva-insp-chk");
            let plan_hdr = tr(cx, "PLANNED", "计划窗口").to_string();
            let act_hdr = tr(cx, "ACTUAL / ETA", "实际 / 预计").to_string();
            let cost_hdr = tr(cx, "COST", "费用").to_string();
            let dep_hdr = tr(cx, "DEPENDS ON", "前置依赖").to_string();
            let p_copy = p;

            pro_fold(
                pro_eid(&self.id, "pva-insp-fold"),
                insp_open,
                76.0,
                0.0,
                move || {
                    let Some((title_s, plan_win, act_win, cost_str, deps_str, st_kind)) = insp_data
                    else {
                        return div().into_any_element();
                    };
                    let card = div()
                        .w_full()
                        .px(px(14.))
                        .py(px(10.))
                        .rounded(px(9.))
                        .bg(p_copy.subtle)
                        .border_1()
                        .border_color(p_copy.line)
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .justify_between()
                        .gap(px(12.))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .when(st_kind == PlanStepStatus::Done, |d| {
                                    d.child(pro_check_draw(
                                        chk_id,
                                        true,
                                        14.0,
                                        false,
                                        p_copy.green_ink,
                                    ))
                                })
                                .child(
                                    div()
                                        .text_size(px(12.5))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(p_copy.strong)
                                        .child(title_s),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_wrap()
                                .items_center()
                                .gap(px(16.))
                                .text_size(px(11.5))
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .child(
                                            div()
                                                .text_size(px(9.5))
                                                .text_color(p_copy.muted)
                                                .child(plan_hdr),
                                        )
                                        .child(div().text_color(p_copy.ink).child(plan_win)),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .child(
                                            div()
                                                .text_size(px(9.5))
                                                .text_color(p_copy.muted)
                                                .child(act_hdr),
                                        )
                                        .child(div().text_color(p_copy.ink).child(act_win)),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .child(
                                            div()
                                                .text_size(px(9.5))
                                                .text_color(p_copy.muted)
                                                .child(cost_hdr),
                                        )
                                        .child(div().text_color(p_copy.ink).child(cost_str)),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .child(
                                            div()
                                                .text_size(px(9.5))
                                                .text_color(p_copy.muted)
                                                .child(dep_hdr),
                                        )
                                        .child(div().text_color(p_copy.text).child(deps_str)),
                                ),
                        )
                        .into_any_element();
                    let mut cell = Some(card);
                    div()
                        .w_full()
                        .px(px(16.))
                        .pb(px(8.))
                        .child(pro_popover_presence(pop_id, insp_open, move || {
                            cell.take().unwrap_or_else(|| div().into_any_element())
                        }))
                        .into_any_element()
                },
            )
        };

        let _ = rows_total_h;
        let footer = div()
            .h(px(FOOT_H))
            .px(px(16.))
            .border_t_1()
            .border_color(p.line)
            .flex()
            .items_center()
            .gap(px(16.))
            .text_size(px(12.))
            .text_color(p.muted)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(
                        canvas(
                            |_, _, _| (),
                            move |b, _, win, _| paint_dashed_ghost_rect(b, ghost_c, win),
                        )
                        .w(px(14.))
                        .h(px(8.)),
                    )
                    .child(tr(cx, "Plan", "计划窗口")),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(
                        div()
                            .w(px(14.))
                            .h(px(8.))
                            .rounded(px(2.5))
                            .bg(actual_bg)
                            .border_1()
                            .border_color(actual_edge),
                    )
                    .child(tr(cx, "Actual", "实际执行")),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(
                        div()
                            .w(px(14.))
                            .h(px(8.))
                            .rounded(px(2.5))
                            .bg(over_bg)
                            .border_1()
                            .border_color(over_edge),
                    )
                    .child(tr(cx, "Over plan", "超出计划")),
            )
            .child(
                pro_tactile(
                    pro_eid(&self.id, "pva-replay"),
                    tr(cx, "Replay plan vs actual run", "重放计划与实际对比运行"),
                    6.0,
                    transparent_black(),
                    p.hover,
                    transparent_black(),
                    p.line,
                    false,
                    div()
                        .h(px(24.))
                        .px(px(8.))
                        .flex()
                        .items_center()
                        .gap(px(4.))
                        .text_size(px(11.5))
                        .text_color(p.muted)
                        .child(pro_icon(ProIcon::Refresh, 11.5, p.muted))
                        .child(tr(cx, "Replay", "重放")),
                    cx,
                    |this, _, cx| {
                        this.replay(cx);
                    },
                )
                .ml_auto()
                .w_auto(),
            );

        div()
            .id(self.id.clone())
            .w_full()
            .flex()
            .justify_center()
            .child(
                div()
                    .w_full()
                    .max_w(px(756.))
                    .rounded(px(12.))
                    .bg(card_bg)
                    .border_1()
                    .border_color(card_border)
                    .when(!p.dark, |d| d.shadow_sm())
                    .flex()
                    .flex_col()
                    .child(header)
                    .child(axis_bar)
                    .child(rows_col)
                    .child(finish_strip)
                    .child(insp_el)
                    .child(footer),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        PlanStep, PlanStepStatus, PlanVsActual, compute_schedule, default_pricing_plan_steps,
    };
    use gpui::{AppContext as _, TestAppContext};

    #[test]
    fn overruns_color_excess_and_shift_downstream_forecast_and_eta() {
        let steps = default_pricing_plan_steps();
        let sched = compute_schedule(&steps, 149_000);
        // Planned finish of the 7-step DAG is 2:55 (`175_000ms`)
        assert_eq!(sched.planned_finish_ms, 175_000);
        // `layout` overran by +22s (`82_000ms` vs `60_000ms`)
        assert_eq!(sched.rows[2].status, PlanStepStatus::Done);
        assert_eq!(sched.rows[2].over_dur_ms, 22_000);
        // `stripe` is Running at 149s (`98_000..149_000` = `51_000ms` > `45_000ms` plan -> `6_000ms` overrun!)
        assert_eq!(sched.rows[4].status, PlanStepStatus::Running);
        assert_eq!(sched.rows[4].over_dur_ms, 6_000);
        // Waiting step `fix` shifts right to `149_000` instead of its planned `125_000`!
        assert_eq!(sched.rows[5].status, PlanStepStatus::Waiting);
        assert_eq!(sched.rows[5].actual_start_ms, 149_000);
        assert!(sched.projected_finish_ms > sched.planned_finish_ms);
    }

    #[gpui::test]
    fn plan_vs_actual_builds_and_replays_from_zero(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let view = cx.new(|cx| {
            PlanVsActual::new("test-pva", cx).with_steps(vec![
                PlanStep::new("a", "Brief", 20_000).with_actual(Some(0), Some(15_000)),
                PlanStep::new("b", "Build", 40_000)
                    .with_deps(["a"])
                    .with_actual(Some(15_000), None),
            ])
        });
        view.update(cx, |v, cx| {
            v.replay(cx);
            assert!(v.replay_mode);
        });
    }
}
