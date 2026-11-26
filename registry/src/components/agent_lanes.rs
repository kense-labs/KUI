//! Live horizontal multi-agent swimlane timeline with inter-lane handoff curves and inline approvals.
//!
//! Displays one row per autonomous agent over a sliding time window (`Last 30m` by default),
//! grows active task blocks smoothly into the `"now"` cursor, connects handoffs across lanes via
//! curved dashed [`PathBuilder`] splines, and provides inline `Approve` / `Reject` controls on
//! waiting blocks plus click-to-inspect task summary popovers.

use crate::{
    components::pro_chrome::{
        ProBtnStyle, ProPalette, pro_agent_avatar, pro_app_tile, pro_btn, pro_card_shadow,
        pro_check_draw, pro_eid, pro_empty_state, pro_mix_color, pro_popover_presence,
        pro_rolling_number, pro_skeleton_bar, pro_spring_f32, pro_tactile, tr,
    },
    foundation::Theme,
    primitives::{
        family::{Slide, disabled_motion},
        pro_icons::{ProIcon, pro_app_artwork, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, collections::HashMap, rc::Rc};
use web_time::Instant;

const LANE_H: f32 = 48.0;
const BLOCK_TOP: f32 = 10.0;
const BLOCK_H: f32 = 28.0;
const AGENT_COL_W: f32 = 158.0;
const DEFAULT_SPAN_MS: u64 = 30 * 60_000;

const SPARK_BAR_SVG: &str = r#"<path d="M7 1.2c.4 2.9 1.9 4.4 4.8 4.8-2.9.4-4.4 1.9-4.8 4.8C6.6 7.9 5.1 6.4 2.2 6 5.1 5.6 6.6 4.1 7 1.2Z" fill="currentColor"></path><circle cx="11.6" cy="11.4" r="1.1" fill="currentColor" opacity="0.55"></circle>"#;

/// Execution state of a [`LaneTask`] block.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LaneTaskStatus {
    /// Completed task block.
    #[default]
    Done,
    /// Task actively executing and growing into `"now"`.
    Running,
    /// Task paused awaiting human `Approve` / `Reject` decision.
    Waiting,
    /// Task failed or was rejected.
    Failed,
}

/// One agent swimlane row in [`AgentLanes`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LaneAgent {
    /// Unique agent id referenced by [`LaneTask::agent_id`].
    pub id: SharedString,
    /// Agent role name (`"Researcher"`, `"Writer"`, `"Reviewer"`, `"Ops"`).
    pub name: SharedString,
    /// Optional Chinese translation of `name`.
    pub name_zh: Option<SharedString>,
    /// Optional model label (`"Opus 5.5"`, `"GPT-6 Sol"`).
    pub model: Option<SharedString>,
    /// Optional provider mark key (`"claude"`, `"openai"`, `"gemini"`, `"kense"`).
    pub avatar: Option<SharedString>,
}

impl LaneAgent {
    /// Create a [`LaneAgent`] with `id` and `name`.
    pub fn new(id: impl Into<SharedString>, name: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            name_zh: None,
            model: None,
            avatar: None,
        }
    }

    /// Set a localized Chinese agent role name.
    pub fn with_name_zh(mut self, zh: impl Into<SharedString>) -> Self {
        self.name_zh = Some(zh.into());
        self
    }

    /// Set the frontier model subtitle (`"Opus 5.5"`, `"GPT-6 Sol"`).
    pub fn with_model(mut self, model: impl Into<SharedString>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set the provider mark key rendered on a 24x24 rounded tile.
    pub fn with_avatar(mut self, avatar: impl Into<SharedString>) -> Self {
        self.avatar = Some(avatar.into());
        self
    }

    fn display_name<'a>(&'a self, cx: &App) -> &'a str {
        if let Some(zh) = &self.name_zh {
            tr(cx, self.name.as_ref(), zh.as_ref())
        } else {
            self.name.as_ref()
        }
    }
}

/// One scheduled or running task block inside an agent's swimlane.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LaneTask {
    /// Stable task identifier.
    pub id: SharedString,
    /// Owning [`LaneAgent::id`].
    pub agent_id: SharedString,
    /// Short task title rendered inside the block.
    pub title: SharedString,
    /// Optional Chinese translation.
    pub title_zh: Option<SharedString>,
    /// Execution status (`Done`, `Running`, `Waiting`, `Failed`).
    pub status: LaneTaskStatus,
    /// Start timestamp in milliseconds from timeline origin.
    pub start_ms: u64,
    /// Optional end timestamp in milliseconds (`None` stretches live to `"now"`).
    pub end_ms: Option<u64>,
    /// Optional downstream [`LaneTask::id`] receiving a curved handoff spline.
    pub handoff_to: Option<SharedString>,
    /// Decision question shown above `Reject` / `Approve` when `status == Waiting`.
    pub ask: Option<SharedString>,
    /// Optional Chinese translation for `ask`.
    pub ask_zh: Option<SharedString>,
    /// Bullet points shown when inspecting a task block.
    pub summary: Vec<SharedString>,
    /// Up to three app/tool keys (`"slack"`, `"figma"`, `"github"`, `"stripe"`) shown before the title.
    pub tools: Vec<SharedString>,
}

impl LaneTask {
    /// Create a [`LaneTask`] on `agent_id` starting at `start_ms`.
    pub fn new(
        id: impl Into<SharedString>,
        agent_id: impl Into<SharedString>,
        title: impl Into<SharedString>,
        status: LaneTaskStatus,
        start_ms: u64,
    ) -> Self {
        Self {
            id: id.into(),
            agent_id: agent_id.into(),
            title: title.into(),
            title_zh: None,
            status,
            start_ms,
            end_ms: None,
            handoff_to: None,
            ask: None,
            ask_zh: None,
            summary: Vec::new(),
            tools: Vec::new(),
        }
    }

    /// Set a localized Chinese title.
    pub fn with_title_zh(mut self, zh: impl Into<SharedString>) -> Self {
        self.title_zh = Some(zh.into());
        self
    }

    /// Set the completed end timestamp in milliseconds (`None` reaches `"now"`).
    pub fn with_end_ms(mut self, end_ms: u64) -> Self {
        self.end_ms = Some(end_ms);
        self
    }

    /// Draw a curved dashed handoff connector to `next_task_id`.
    pub fn with_handoff_to(mut self, next_task_id: impl Into<SharedString>) -> Self {
        self.handoff_to = Some(next_task_id.into());
        self
    }

    /// Set the decision question for a `Waiting` task.
    pub fn with_ask(mut self, ask: impl Into<SharedString>) -> Self {
        self.ask = Some(ask.into());
        self
    }

    /// Set a localized Chinese decision question.
    pub fn with_ask_zh(mut self, zh: impl Into<SharedString>) -> Self {
        self.ask_zh = Some(zh.into());
        self
    }

    /// Attach bullet summary points for the click-to-inspect popover.
    pub fn with_summary<I, S>(mut self, bullets: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        self.summary = bullets.into_iter().map(Into::into).collect();
        self
    }

    /// Attach up to three app/tool marks displayed inside the block.
    pub fn with_tools<I, S>(mut self, tools: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        self.tools = tools.into_iter().take(3).map(Into::into).collect();
        self
    }

    fn display_title<'a>(&'a self, cx: &App) -> &'a str {
        if let Some(zh) = &self.title_zh {
            tr(cx, self.title.as_ref(), zh.as_ref())
        } else {
            self.title.as_ref()
        }
    }

    fn display_ask<'a>(&'a self, cx: &App) -> Option<&'a str> {
        match (&self.ask, &self.ask_zh) {
            (Some(en), Some(zh)) => Some(tr(cx, en.as_ref(), zh.as_ref())),
            (Some(en), None) => Some(en.as_ref()),
            _ => None,
        }
    }
}

/// Typed events emitted by [`AgentLanes`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentLanesEvent {
    /// Waiting task approved (`task_id`).
    Approve(SharedString),
    /// Waiting task rejected (`task_id`).
    Reject(SharedString),
    /// User clicked `"Open"` inside a task's summary popover (`task_id`).
    OpenTask(SharedString),
    /// Selected/inspected task block changed.
    SelectTask(Option<SharedString>),
}

impl EventEmitter<AgentLanesEvent> for AgentLanes {}

fn default_lane_agents() -> Vec<LaneAgent> {
    vec![
        LaneAgent::new("research", "Researcher")
            .with_name_zh("研究智能体")
            .with_model("Opus 5.5")
            .with_avatar("claude"),
        LaneAgent::new("writer", "Writer")
            .with_name_zh("写作智能体")
            .with_model("GPT-6 Sol")
            .with_avatar("openai"),
        LaneAgent::new("reviewer", "Reviewer")
            .with_name_zh("审查智能体")
            .with_model("Gemini 3 Pro")
            .with_avatar("gemini"),
        LaneAgent::new("ops", "Ops")
            .with_name_zh("运维智能体")
            .with_model("Gemini 3.8 Flash")
            .with_avatar("gemini"),
    ]
}

fn default_lane_tasks() -> Vec<LaneTask> {
    // Timeline window `[0 .. 1_800_000]` (`30m` = `1_800_000ms`):
    vec![
        LaneTask::new(
            "t-audit",
            "research",
            "Competitor pricing audit",
            LaneTaskStatus::Done,
            60_000,
        )
        .with_title_zh("竞品定价调研")
        .with_end_ms(760_000)
        .with_tools(["drive", "notion"])
        .with_handoff_to("t-copy")
        .with_summary([
            "Compared 6 SaaS tier tables and seat models",
            "Saved structured matrix to Notion brief",
        ]),
        LaneTask::new(
            "t-sla",
            "research",
            "Enterprise SLA benchmarks",
            LaneTaskStatus::Running,
            1_080_000,
        )
        .with_title_zh("企业级 SLA 基准对比")
        .with_tools(["notion"])
        .with_summary([
            "Reviewing uptime credit clauses across 4 vendors",
            "Drafting SOC2 & SSO tier inclusions",
        ]),
        LaneTask::new(
            "t-copy",
            "writer",
            "Draft tier copy & FAQs",
            LaneTaskStatus::Done,
            660_000,
        )
        .with_title_zh("撰写套餐文案与常见问答")
        .with_end_ms(1_320_000)
        .with_tools(["slack", "figma"])
        .with_handoff_to("t-verify")
        .with_summary([
            "Wrote headline, 3 tier cards, and 8 billing FAQs",
            "Synced copy frames directly into Figma",
        ]),
        LaneTask::new(
            "t-verify",
            "reviewer",
            "Verify annual billing math",
            LaneTaskStatus::Running,
            1_120_000,
        )
        .with_title_zh("校验年付折扣计费逻辑")
        .with_tools(["github", "linear"])
        .with_summary([
            "Checked 20% annual discount & proration edge cases",
            "Running Playwright checkout assertions",
        ]),
        LaneTask::new(
            "t-crm",
            "ops",
            "Sync CRM plan tags",
            LaneTaskStatus::Done,
            160_000,
        )
        .with_title_zh("同步 CRM 套餐标签")
        .with_end_ms(740_000)
        .with_tools(["hubspot"])
        .with_handoff_to("t-stripe")
        .with_summary(["Mapped Pro & Enterprise lifecycle properties in HubSpot"]),
        LaneTask::new(
            "t-stripe",
            "ops",
            "Publish Stripe price IDs",
            LaneTaskStatus::Waiting,
            780_000,
        )
        .with_title_zh("发布 Stripe 正式价格 ID")
        .with_tools(["stripe"])
        .with_ask("Create live Stripe price IDs for Pro ($29/mo) and Team ($79/mo) in production?")
        .with_ask_zh("确认在生产环境创建 Pro（$29/月）与 Team（$79/月）的正式 Stripe 价格 ID？")
        .with_summary(["Staged 4 monthly/annual price objects in Stripe"]),
    ]
}

/// Horizontal multi-agent swimlane control room with live task blocks, handoffs, and inline approvals.
///
/// # Examples
///
/// ```no_run
/// # #[allow(unused_variables, unused_imports)]
/// # fn example(window: &mut gpui::Window, cx: &mut gpui::App) {
/// # use gpui::prelude::*;
/// use gpui::prelude::*;
/// use kui::components::agent_lanes::{AgentLanes, AgentLanesEvent};
///
/// let lanes = cx.new(|cx| {
///     AgentLanes::new("agent-control-room", cx)
///         .with_span(30 * 60_000)
///         .with_live(true)
/// });
/// cx.subscribe(&lanes, |_, ev: &AgentLanesEvent, _| {
///     let _ = ev;
/// }).detach();
/// # }
/// ```
pub struct AgentLanes {
    id: ElementId,
    title: Option<SharedString>,
    agents: Vec<LaneAgent>,
    tasks: Vec<LaneTask>,
    span_ms: u64,
    base_now_ms: u64,
    live: bool,
    selected_task_id: Option<SharedString>,
    last_selected_task: Option<LaneTask>,
    hovered_task_id: Option<SharedString>,
    hovered_agent_id: Option<SharedString>,
    track_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    scrubber_frac: Option<f32>,
    scrubber_slide: Slide,
    started_at: Instant,
    loading: bool,
    empty: bool,
}

impl AgentLanes {
    /// Create an [`AgentLanes`] swimlane view with 4 agents (`2 running`, `1 needs you`) over `Last 30m`.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        Self {
            id: id.into(),
            title: None,
            agents: default_lane_agents(),
            tasks: default_lane_tasks(),
            span_ms: DEFAULT_SPAN_MS,
            base_now_ms: DEFAULT_SPAN_MS,
            live: true,
            selected_task_id: None,
            last_selected_task: None,
            hovered_task_id: None,
            hovered_agent_id: None,
            track_bounds: Rc::new(Cell::new(None)),
            scrubber_frac: None,
            scrubber_slide: Slide::snappy(0.85),
            started_at: Instant::now(),
            loading: false,
            empty: false,
        }
    }

    /// Override the top bar heading (`"Agents"` by default).
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Configure the swimlane agents (one horizontal row per agent).
    pub fn with_agents(mut self, agents: Vec<LaneAgent>) -> Self {
        self.agents = agents;
        self
    }

    /// Supply all task blocks across agents.
    pub fn with_tasks(mut self, tasks: Vec<LaneTask>) -> Self {
        self.tasks = tasks;
        self
    }

    /// Set the visible sliding window duration in milliseconds (`1_800_000` = `30m`).
    pub fn with_span(mut self, span_ms: u64) -> Self {
        self.span_ms = span_ms.max(60_000);
        self
    }

    /// Pin or initialize the `"now"` timestamp in milliseconds.
    pub fn with_now(mut self, now_ms: u64) -> Self {
        self.base_now_ms = now_ms;
        self
    }

    /// Enable or pause live sliding clock progression.
    pub fn with_live(mut self, live: bool) -> Self {
        self.live = live;
        self
    }

    /// Render skeleton placeholder lanes.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Force the empty state view.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Current simulated `now_ms`.
    pub fn current_now_ms(&self, reduced: bool) -> u64 {
        if self.live && !reduced {
            let delta = (self.started_at.elapsed().as_secs_f32() * 1_500.0) as u64;
            self.base_now_ms + delta
        } else {
            self.base_now_ms
        }
    }

    /// Count `(running_count, waiting_count)` across all tasks.
    pub fn active_counts(&self) -> (usize, usize) {
        let running = self
            .tasks
            .iter()
            .filter(|t| t.status == LaneTaskStatus::Running)
            .count();
        let waiting = self
            .tasks
            .iter()
            .filter(|t| t.status == LaneTaskStatus::Waiting)
            .count();
        (running, waiting)
    }

    /// Return the current task slice.
    pub fn tasks(&self) -> &[LaneTask] {
        &self.tasks
    }

    /// Approve a waiting task (`Waiting -> Running`), closing its decision popover and emitting [`AgentLanesEvent::Approve`].
    pub fn approve_task(&mut self, task_id: &str, cx: &mut Context<Self>) {
        if let Some(t) = self.tasks.iter_mut().find(|t| t.id.as_ref() == task_id) {
            let snap = t.clone();
            t.status = LaneTaskStatus::Running;
            let id = t.id.clone();
            if self.selected_task_id.as_ref() == Some(&id) {
                self.last_selected_task = Some(snap);
                self.selected_task_id = None;
            }
            cx.emit(AgentLanesEvent::Approve(id));
            cx.notify();
        }
    }

    /// Reject a waiting task (`Waiting -> Failed`), closing its decision popover and emitting [`AgentLanesEvent::Reject`].
    pub fn reject_task(&mut self, task_id: &str, run_now_ms: u64, cx: &mut Context<Self>) {
        if let Some(t) = self.tasks.iter_mut().find(|t| t.id.as_ref() == task_id) {
            let snap = t.clone();
            t.status = LaneTaskStatus::Failed;
            t.end_ms = Some(run_now_ms);
            let id = t.id.clone();
            if self.selected_task_id.as_ref() == Some(&id) {
                self.last_selected_task = Some(snap);
                self.selected_task_id = None;
            }
            cx.emit(AgentLanesEvent::Reject(id));
            cx.notify();
        }
    }
}

fn paint_dashed_cubic_handoff(
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    phase_offset: f32,
    color: Hsla,
    win: &mut Window,
) {
    let steps = 36usize;
    let dx = (x1 - x0).abs().max(18.0);
    let c1x = x0 + dx * 0.52;
    let c1y = y0;
    let c2x = x1 - dx * 0.52;
    let c2y = y1;

    let mut pts: Vec<(f32, f32)> = Vec::with_capacity(steps + 1);
    let mut cum_len = Vec::with_capacity(steps + 1);
    let mut acc = 0.0f32;
    for i in 0..=steps {
        let u = i as f32 / steps as f32;
        let inv = 1.0 - u;
        let bx = inv * inv * inv * x0
            + 3.0 * inv * inv * u * c1x
            + 3.0 * inv * u * u * c2x
            + u * u * u * x1;
        let by = inv * inv * inv * y0
            + 3.0 * inv * inv * u * c1y
            + 3.0 * inv * u * u * c2y
            + u * u * u * y1;
        if let Some(&(px_prev, py_prev)) = pts.last() {
            acc += (bx - px_prev).hypot(by - py_prev);
        }
        pts.push((bx, by));
        cum_len.push(acc);
    }

    let dash_period = 6.5f32;
    for i in 0..steps {
        let mid_d = (cum_len[i] + cum_len[i + 1]) * 0.5 - phase_offset * dash_period;
        let m = mid_d.rem_euclid(dash_period);
        if m <= dash_period * 0.52 {
            let mut seg = PathBuilder::stroke(px(1.25));
            seg.move_to(point(px(pts[i].0), px(pts[i].1)));
            seg.line_to(point(px(pts[i + 1].0), px(pts[i + 1].1)));
            if let Ok(path) = seg.build() {
                win.paint_path(path, color);
            }
        }
    }
}

impl Render for AgentLanes {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let reduced = disabled_motion(cx) || Theme::global(cx).reduced_motion;
        let now_ms = self.current_now_ms(reduced);
        let (running_cnt, waiting_cnt) = self.active_counts();

        if ((self.live && (running_cnt > 0 || waiting_cnt > 0)) || self.loading) && !reduced {
            window.request_animation_frame();
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
        let run_bg: Hsla = if p.dark {
            rgba(0x0077ff3a).into()
        } else {
            rgba(0x008ff519).into()
        };
        let run_fg: Hsla = if p.dark {
            rgb(0x4fb0fe).into()
        } else {
            rgba(0x006dcbf2).into()
        };
        let run_edge: Hsla = if p.dark {
            rgb(0x4fb0fe).opacity(0.62).into()
        } else {
            rgba(0x0060e280).into()
        };
        let warn_bg: Hsla = if p.dark {
            rgba(0xfb6a0028).into()
        } else {
            rgba(0xff9c0029).into()
        };
        let warn_fg = p.amber_ink;
        let done_bg: Hsla = if p.dark {
            hsla(0., 0., 1., 0.065)
        } else {
            hsla(0., 0., 0., 0.05)
        };
        let rail_c: Hsla = if p.dark {
            hsla(0., 0., 1., 0.22)
        } else {
            hsla(0., 0., 0., 0.16)
        };

        let elapsed_s = self.started_at.elapsed().as_secs_f32();
        let pulse_alpha = if reduced {
            1.0
        } else {
            0.55 + 0.45 * ((elapsed_s / 1.6) * std::f32::consts::TAU).cos().abs()
        };
        let flow_phase = if reduced {
            0.0
        } else {
            (elapsed_s / 0.90).fract()
        };

        let title_str: SharedString = self
            .title
            .clone()
            .unwrap_or_else(|| tr(cx, "Agents", "智能体泳道").into());
        let span_mins = (self.span_ms / 60_000).max(1);
        let window_label = if tr(cx, "en", "zh") == "zh" {
            format!("最近 {span_mins} 分钟")
        } else {
            format!("Last {span_mins}m")
        };
        let run_word = tr(cx, "running", "个运行中");
        let wait_word = tr(cx, "needs you", "个待确认");

        let bar_header = div()
            .px(px(16.))
            .py(px(13.))
            .border_b_1()
            .border_color(p.line)
            .flex()
            .items_center()
            .gap(px(10.))
            .child(pro_svg("0 0 14 14", SPARK_BAR_SVG, 15., run_fg))
            .child(
                div()
                    .text_size(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .child(title_str),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .text_size(px(12.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(5.))
                            .text_color(p.muted)
                            .child(div().size(px(6.)).rounded_full().bg(run_fg))
                            .child(pro_rolling_number(
                                pro_eid(&self.id, "lanes-run-cnt"),
                                running_cnt as f64,
                                0,
                                "",
                                "",
                                12.0,
                                FontWeight::MEDIUM,
                                p.ink,
                            ))
                            .child(run_word),
                    )
                    .when(waiting_cnt > 0, |d| {
                        d.child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(5.))
                                .text_color(warn_fg)
                                .child(
                                    div()
                                        .size(px(6.))
                                        .rounded_full()
                                        .bg(warn_fg.opacity(pulse_alpha)),
                                )
                                .child(pro_rolling_number(
                                    pro_eid(&self.id, "lanes-wait-cnt"),
                                    waiting_cnt as f64,
                                    0,
                                    "",
                                    "",
                                    12.0,
                                    FontWeight::MEDIUM,
                                    warn_fg,
                                ))
                                .child(wait_word),
                        )
                    }),
            )
            .child(
                div()
                    .h(px(21.))
                    .px(px(8.))
                    .flex_none()
                    .rounded(px(6.))
                    .bg(p.chip)
                    .text_color(p.text)
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .flex()
                    .items_center()
                    .child(window_label),
            );

        if self.loading {
            let phase = if reduced {
                0.5
            } else {
                (elapsed_s * 0.9).fract()
            };
            let mut skel = div().p(px(16.)).flex().flex_col().gap(px(10.));
            for i in 0..4usize {
                skel = skel.child(
                    div()
                        .h(px(36.))
                        .flex()
                        .items_center()
                        .gap(px(14.))
                        .child(pro_skeleton_bar(24., 24., 6., phase + i as f32 * 0.1, cx))
                        .child(pro_skeleton_bar(110., 12., 4., phase + i as f32 * 0.1, cx))
                        .child(pro_skeleton_bar(
                            260. + (i as f32 * 40.),
                            26.,
                            6.,
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
                        .max_w(px(800.))
                        .rounded(px(10.))
                        .bg(card_bg)
                        .border_1()
                        .border_color(card_border)
                        .shadow(pro_card_shadow(p.dark))
                        .flex()
                        .flex_col()
                        .child(bar_header)
                        .child(skel),
                )
                .into_any_element();
        }

        if self.empty || self.agents.is_empty() {
            let empty_el = pro_empty_state(
                ProIcon::Users,
                tr(cx, "No active agent lanes", "暂无智能体泳道"),
                tr(
                    cx,
                    "Add agents and live tasks to monitor parallel work, handoffs, and approvals.",
                    "配置智能体与任务后，可在此实时检视多智能体并行执行、交接连线与待办审批。",
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
                        .max_w(px(800.))
                        .rounded(px(10.))
                        .bg(card_bg)
                        .border_1()
                        .border_color(card_border)
                        .shadow(pro_card_shadow(p.dark))
                        .flex()
                        .flex_col()
                        .child(bar_header)
                        .child(empty_el),
                )
                .into_any_element();
        }

        let span_f = self.span_ms as f32;
        let win_start_ms = now_ms.saturating_sub(self.span_ms);
        let mut agent_row_map: HashMap<&str, usize> = HashMap::with_capacity(self.agents.len());
        for (idx, ag) in self.agents.iter().enumerate() {
            agent_row_map.insert(ag.id.as_ref(), idx);
        }

        let focus_task_id = self
            .hovered_task_id
            .as_ref()
            .or(self.selected_task_id.as_ref());
        let focus_agent_from_task = focus_task_id.and_then(|ft| {
            self.tasks
                .iter()
                .find(|t| &t.id == ft)
                .map(|t| t.agent_id.clone())
        });

        // Left column: agent avatars, role names, status chips (`Idle` / `Needs you`), and models
        let mut agents_col = div().w(px(AGENT_COL_W)).flex_none().flex().flex_col();
        for ag in &self.agents {
            let aid = ag.id.clone();
            let has_running = self
                .tasks
                .iter()
                .any(|t| t.agent_id == aid && t.status == LaneTaskStatus::Running);
            let has_waiting = self
                .tasks
                .iter()
                .any(|t| t.agent_id == aid && t.status == LaneTaskStatus::Waiting);
            let is_faded = if let Some(ha) = &self.hovered_agent_id {
                ha != &aid
            } else if let Some(fa) = &focus_agent_from_task {
                fa != &aid
            } else {
                false
            };
            let ag_alpha = pro_spring_f32(
                pro_eid(&self.id, format!("lane-ag-spot-{aid}")),
                if is_faded { 0.44 } else { 1.0 },
                0.22,
                window,
                cx,
            );

            let avatar_seed = lane_agent_avatar_seed(ag);
            let avatar_el = if let Some(logo) = &ag.avatar
                && !lane_brand_is_model(logo.as_ref())
            {
                pro_app_tile(logo.as_ref(), 24., cx)
            } else {
                pro_agent_avatar(avatar_seed, run_fg, 24., has_running, elapsed_s * 1000., cx)
            };

            let state_pill: Option<AnyElement> = if has_waiting {
                Some(
                    div()
                        .h(px(16.))
                        .px(px(5.))
                        .ml(px(5.))
                        .flex_none()
                        .rounded(px(4.))
                        .bg(warn_bg)
                        .text_color(warn_fg)
                        .text_size(px(10.5))
                        .font_weight(FontWeight::MEDIUM)
                        .flex()
                        .items_center()
                        .child(tr(cx, "Needs you", "待确认"))
                        .into_any_element(),
                )
            } else if !has_running {
                Some(
                    div()
                        .h(px(16.))
                        .px(px(5.))
                        .ml(px(5.))
                        .flex_none()
                        .rounded(px(4.))
                        .bg(p.chip)
                        .text_color(p.muted)
                        .text_size(px(10.5))
                        .font_weight(FontWeight::MEDIUM)
                        .flex()
                        .items_center()
                        .child(tr(cx, "Idle", "空闲"))
                        .into_any_element(),
                )
            } else {
                None
            };

            let hov_aid = aid.clone();
            agents_col = agents_col.child(
                div()
                    .id(pro_eid(&self.id, format!("lane-ag-{aid}")))
                    .h(px(LANE_H))
                    .pl(px(14.))
                    .pr(px(8.))
                    .opacity(ag_alpha)
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                        let next = if *inside { Some(hov_aid.clone()) } else { None };
                        if this.hovered_agent_id != next {
                            this.hovered_agent_id = next;
                            cx.notify();
                        }
                    }))
                    .child(avatar_el)
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .min_w_0()
                                    .child(
                                        div()
                                            .truncate()
                                            .text_size(px(13.))
                                            .line_height(px(18.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.ink)
                                            .child(ag.display_name(cx).to_string()),
                                    )
                                    .when_some(state_pill, |d, pill| d.child(pill)),
                            )
                            .when_some(ag.model.clone(), |d, m| {
                                d.child(
                                    div()
                                        .truncate()
                                        .text_size(px(11.5))
                                        .line_height(px(15.))
                                        .text_color(p.muted)
                                        .child(m),
                                )
                            }),
                    ),
            );
        }

        // Compute handoff curves `(u0, row0, u1, row1, is_live, spot)`
        let mut handoffs: Vec<(f32, usize, f32, usize, bool, f32)> = Vec::new();
        for t in &self.tasks {
            let Some(target_id) = &t.handoff_to else {
                continue;
            };
            let Some(&r0) = agent_row_map.get(t.agent_id.as_ref()) else {
                continue;
            };
            let Some(dst) = self.tasks.iter().find(|x| &x.id == target_id) else {
                continue;
            };
            let Some(&r1) = agent_row_map.get(dst.agent_id.as_ref()) else {
                continue;
            };
            let t0_end = t.end_ms.unwrap_or(now_ms);
            if t0_end < win_start_ms || dst.start_ms > now_ms {
                continue;
            }
            let u0 = ((t0_end.saturating_sub(win_start_ms)) as f32 / span_f).clamp(0., 1.);
            let u1 = ((dst.start_ms.saturating_sub(win_start_ms)) as f32 / span_f).clamp(0., 1.);
            let is_live_handoff = matches!(
                dst.status,
                LaneTaskStatus::Running | LaneTaskStatus::Waiting
            );
            let ho_hot = focus_task_id.is_some_and(|ft| ft == &t.id || ft == &dst.id);
            let ho_spot = pro_spring_f32(
                pro_eid(&self.id, format!("lane-ho-{}-{}", t.id, dst.id)),
                if ho_hot { 1.0 } else { 0.0 },
                0.22,
                window,
                cx,
            );
            handoffs.push((u0, r0, u1, r1, is_live_handoff || ho_hot, ho_spot));
        }

        let scrub_x = self.scrubber_slide.get(window, cx).clamp(0.0, 1.0);
        let scrub_alpha = pro_spring_f32(
            pro_eid(&self.id, "lanes-scrub-a"),
            if self.scrubber_frac.is_some() {
                1.0
            } else {
                0.0
            },
            0.18,
            window,
            cx,
        );

        let num_lanes = self.agents.len();
        let track_h = (num_lanes as f32) * LANE_H;
        let grid_line_c = p.line;
        let scrub_line_c = p.ink.opacity(0.32);
        let bounds_for_prepaint = self.track_bounds.clone();

        // Track canvas: horizontal lane dividers, 5-minute vertical dashed ticks, hover crosshair, right-edge `now` line, and curved handoffs!
        let track_canvas = canvas(
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

                // Horizontal lane borders
                for r in 1..=num_lanes {
                    let ly = oy + (r as f32) * LANE_H;
                    let mut pb = PathBuilder::stroke(px(1.0));
                    pb.move_to(point(px(ox), px(ly)));
                    pb.line_to(point(px(ox + tw), px(ly)));
                    if let Ok(path) = pb.build() {
                        win.paint_path(path, grid_line_c);
                    }
                }

                // Vertical dashed time ticks (`1/6 .. 5/6`)
                for step in 1..6usize {
                    let tx = ox + (step as f32 / 6.0) * tw;
                    let nd = ((th / 5.5) as usize).max(1);
                    for d in 0..nd {
                        let y0 = oy + (d as f32) * 5.5;
                        let y1 = (y0 + 2.5).min(oy + th);
                        let mut pb = PathBuilder::stroke(px(1.0));
                        pb.move_to(point(px(tx), px(y0)));
                        pb.line_to(point(px(tx), px(y1)));
                        if let Ok(path) = pb.build() {
                            win.paint_path(path, grid_line_c);
                        }
                    }
                }

                // Smooth `Slide::snappy` hover time-scrubber crosshair
                if scrub_alpha > 0.01 {
                    let sx = ox + scrub_x * tw;
                    let mut spb = PathBuilder::stroke(px(1.0));
                    spb.move_to(point(px(sx), px(oy)));
                    spb.line_to(point(px(sx), px(oy + th)));
                    if let Ok(path) = spb.build() {
                        win.paint_path(path, scrub_line_c.opacity(scrub_line_c.a * scrub_alpha));
                    }
                }

                // Right-edge `now` cursor line
                let mut now_pb = PathBuilder::stroke(px(1.5));
                now_pb.move_to(point(px(ox + tw - 0.75), px(oy)));
                now_pb.line_to(point(px(ox + tw - 0.75), px(oy + th)));
                if let Ok(path) = now_pb.build() {
                    win.paint_path(path, run_edge);
                }

                // Inter-lane curved handoff splines (`PathBuilder`)
                for &(u0, r0, u1, r1, is_live_h, ho_spot) in &handoffs {
                    let x0 = ox + u0 * tw;
                    let y0 = oy + (r0 as f32) * LANE_H + LANE_H * 0.5;
                    let x1 = (ox + u1 * tw).max(x0 + 4.0);
                    let y1 = oy + (r1 as f32) * LANE_H + LANE_H * 0.5;
                    let base_hc = if is_live_h { run_edge } else { rail_c };
                    let hc = pro_mix_color(base_hc, run_fg, ho_spot);
                    paint_dashed_cubic_handoff(
                        x0,
                        y0,
                        x1,
                        y1,
                        if is_live_h { flow_phase } else { 0.0 },
                        hc,
                        win,
                    );
                }
            },
        )
        .absolute()
        .inset_0();

        // Top time axis labels (`−25m .. −5m` + `now` badge + smooth scrubber pill)
        let mut top_ticks = div().w_full().h(px(20.)).relative();
        for step in 1..6usize {
            let mins_ago = ((6 - step) as u64 * (self.span_ms / 60_000)) / 6;
            let frac = step as f32 / 6.0;
            top_ticks = top_ticks.child(
                div()
                    .absolute()
                    .left(relative(frac))
                    .top(px(2.))
                    .ml(px(-14.))
                    .text_size(px(11.))
                    .line_height(px(14.))
                    .text_color(p.faint)
                    .child(format!("−{mins_ago}m")),
            );
        }
        top_ticks = top_ticks.child(
            div()
                .absolute()
                .right_0()
                .top(px(2.))
                .h(px(15.))
                .px(px(5.))
                .rounded(px(4.))
                .bg(run_bg)
                .text_color(run_fg)
                .text_size(px(10.5))
                .font_weight(FontWeight::MEDIUM)
                .flex()
                .items_center()
                .child(tr(cx, "now", "此刻")),
        );
        if scrub_alpha > 0.01 {
            let scrub_mins = ((1.0 - scrub_x) * (self.span_ms as f32 / 60_000.0)).round() as u32;
            let scrub_txt = if scrub_mins == 0 {
                tr(cx, "now", "此刻").to_string()
            } else {
                format!("−{scrub_mins}m")
            };
            top_ticks = top_ticks.child(
                div()
                    .absolute()
                    .left(relative(scrub_x.clamp(0.04, 0.90)))
                    .top(px(1.))
                    .ml(px(-16.))
                    .h(px(16.))
                    .px(px(5.))
                    .rounded(px(4.))
                    .bg(card_bg)
                    .border_1()
                    .border_color(p.line_strong)
                    .opacity(scrub_alpha)
                    .text_size(px(10.5))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.ink)
                    .flex()
                    .items_center()
                    .child(scrub_txt),
            );
        }

        let bounds_for_move = self.track_bounds.clone();
        let mut track_box = div()
            .id(pro_eid(&self.id, "lanes-track-area"))
            .w_full()
            .h(px(track_h))
            .relative()
            .on_mouse_move(cx.listener(move |this, ev: &MouseMoveEvent, _, cx| {
                if let Some(b) = bounds_for_move.get() {
                    let bw = f32::from(b.size.width);
                    if bw > 8.0 {
                        let frac = ((f32::from(ev.position.x - b.left())) / bw).clamp(0.0, 1.0);
                        this.scrubber_frac = Some(frac);
                        this.scrubber_slide.set(frac, cx);
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
            .child(track_canvas);

        // Render task blocks on each lane
        for task in &self.tasks {
            let Some(&lane_idx) = agent_row_map.get(task.agent_id.as_ref()) else {
                continue;
            };
            let t_end = task.end_ms.unwrap_or(now_ms);
            if t_end <= win_start_ms || task.start_ms >= now_ms {
                continue;
            }
            let clamped_start = task.start_ms.max(win_start_ms);
            let clamped_end = t_end.min(now_ms);
            let raw_left = ((clamped_start - win_start_ms) as f32 / span_f).clamp(0., 0.98);
            let raw_width = ((clamped_end.saturating_sub(clamped_start)) as f32 / span_f)
                .clamp(0.015, 1.0 - raw_left);

            let tid = task.id.clone();
            let left_frac = pro_spring_f32(
                pro_eid(&self.id, format!("lane-l-{tid}")),
                raw_left,
                0.26,
                window,
                cx,
            )
            .clamp(0.0, 0.98);
            let width_frac = pro_spring_f32(
                pro_eid(&self.id, format!("lane-w-{tid}")),
                raw_width,
                0.26,
                window,
                cx,
            )
            .clamp(0.015, 1.0 - left_frac);

            let (blk_bg, blk_fg) = match task.status {
                LaneTaskStatus::Done => (done_bg, p.text),
                LaneTaskStatus::Running => (run_bg, run_fg),
                LaneTaskStatus::Waiting => (warn_bg, warn_fg),
                LaneTaskStatus::Failed => (p.red_soft, p.red_ink),
            };

            let is_hot = self.hovered_task_id.as_ref() == Some(&tid)
                || self.selected_task_id.as_ref() == Some(&tid);
            let is_connected = focus_task_id.is_some_and(|ft| {
                ft == &tid
                    || task.handoff_to.as_ref() == Some(ft)
                    || self
                        .tasks
                        .iter()
                        .any(|o| &o.id == ft && o.handoff_to.as_ref() == Some(&tid))
            });
            let is_faded = if let Some(ha) = &self.hovered_agent_id {
                ha != &task.agent_id
            } else if focus_task_id.is_some() {
                !is_connected
            } else {
                false
            };
            let blk_alpha = pro_spring_f32(
                pro_eid(&self.id, format!("lane-blk-a-{tid}")),
                if is_faded { 0.42 } else { 1.0 },
                0.20,
                window,
                cx,
            );
            let hot_amt = pro_spring_f32(
                pro_eid(&self.id, format!("lane-blk-hot-{tid}")),
                if is_hot { 1.0 } else { 0.0 },
                0.18,
                window,
                cx,
            );

            let mut tools_row = div().flex_none().flex().items_center().gap(px(4.));
            for tk in &task.tools {
                if let Some(mark) = pro_app_artwork(tk.as_ref(), 12., cx) {
                    tools_row = tools_row.child(mark);
                }
            }

            let tid_app = tid.clone();
            let tid_rej = tid.clone();
            let is_waiting = task.status == LaneTaskStatus::Waiting;
            let is_done = task.status == LaneTaskStatus::Done;
            let show_inline_actions = is_waiting && width_frac >= 0.30;

            let inline_controls: Option<AnyElement> = if show_inline_actions {
                Some(
                    div()
                        .ml_auto()
                        .pl(px(6.))
                        .flex_none()
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .flex()
                        .items_center()
                        .gap(px(4.))
                        .child(
                            pro_tactile(
                                pro_eid(&self.id, format!("inl-rej-{tid}")),
                                tr(cx, "Reject task", "拒绝任务"),
                                4.0,
                                card_bg,
                                p.hover,
                                p.line_strong,
                                p.line_strong,
                                false,
                                div()
                                    .h(px(20.))
                                    .px(px(6.))
                                    .text_size(px(11.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .flex()
                                    .items_center()
                                    .child(tr(cx, "Reject", "拒绝")),
                                cx,
                                move |this, _, cx| {
                                    cx.stop_propagation();
                                    let run_now = this.current_now_ms(
                                        disabled_motion(cx) || Theme::global(cx).reduced_motion,
                                    );
                                    this.reject_task(tid_rej.as_ref(), run_now, cx);
                                },
                            )
                            .w_auto(),
                        )
                        .child(
                            pro_tactile(
                                pro_eid(&self.id, format!("inl-app-{tid}")),
                                tr(cx, "Approve task", "批准任务"),
                                4.0,
                                p.primary,
                                p.primary.opacity(0.88),
                                p.primary,
                                p.primary.opacity(0.88),
                                false,
                                div()
                                    .h(px(20.))
                                    .px(px(7.))
                                    .text_color(p.on_primary)
                                    .text_size(px(11.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .flex()
                                    .items_center()
                                    .child(tr(cx, "Approve", "批准")),
                                cx,
                                move |this, _, cx| {
                                    cx.stop_propagation();
                                    this.approve_task(tid_app.as_ref(), cx);
                                },
                            )
                            .w_auto(),
                        )
                        .into_any_element(),
                )
            } else if is_waiting {
                Some(
                    div()
                        .ml_auto()
                        .h(px(18.))
                        .px(px(5.))
                        .flex_none()
                        .rounded(px(4.))
                        .bg(card_bg)
                        .text_color(warn_fg)
                        .opacity(pulse_alpha)
                        .text_size(px(10.5))
                        .font_weight(FontWeight::MEDIUM)
                        .flex()
                        .items_center()
                        .child(tr(cx, "Needs you", "待确认"))
                        .into_any_element(),
                )
            } else {
                None
            };

            let title_txt = task.display_title(cx).to_string();
            let click_tid = tid.clone();
            let key_tid = tid.clone();
            let hov_tid = tid.clone();
            let snap_task = task.clone();
            let snap_task_key = task.clone();
            let is_running = task.status == LaneTaskStatus::Running;
            let blk_border_c = pro_mix_color(transparent_black(), run_edge, hot_amt);

            let block = div()
                .id(pro_eid(&self.id, format!("lane-blk-{tid}")))
                .role(Role::Button)
                .aria_label(title_txt.clone())
                .tab_index(0)
                .absolute()
                .top(px((lane_idx as f32) * LANE_H + BLOCK_TOP - 0.8 * hot_amt))
                .left(relative(left_frac))
                .w(relative(width_frac))
                .h(px(BLOCK_H + 1.6 * hot_amt))
                .px(px(7.))
                .rounded(px(6.))
                .overflow_hidden()
                .opacity(blk_alpha)
                .bg(blk_bg)
                .border_1()
                .border_color(blk_border_c)
                .when(is_running, |d| d.border_r_2().border_color(run_edge))
                .text_color(blk_fg)
                .text_size(px(11.5))
                .font_weight(FontWeight::MEDIUM)
                .flex()
                .items_center()
                .gap(px(4.))
                .cursor_pointer()
                .focus_visible(move |s| s.border_1().border_color(p.focus))
                .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                    if *inside {
                        this.hovered_task_id = Some(hov_tid.clone());
                        cx.notify();
                    } else if this.hovered_task_id.as_ref() == Some(&hov_tid) {
                        this.hovered_task_id = None;
                        cx.notify();
                    }
                }))
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.last_selected_task = Some(snap_task.clone());
                    let next = if this.selected_task_id.as_ref() == Some(&click_tid) {
                        None
                    } else {
                        Some(click_tid.clone())
                    };
                    this.selected_task_id = next.clone();
                    cx.emit(AgentLanesEvent::SelectTask(next));
                    cx.notify();
                }))
                .on_key_down(cx.listener(move |this, ev: &KeyDownEvent, _, cx| {
                    match ev.keystroke.key.as_str() {
                        "enter" | "space" | " " => {
                            this.last_selected_task = Some(snap_task_key.clone());
                            let next = if this.selected_task_id.as_ref() == Some(&key_tid) {
                                None
                            } else {
                                Some(key_tid.clone())
                            };
                            this.selected_task_id = next.clone();
                            cx.emit(AgentLanesEvent::SelectTask(next));
                            cx.stop_propagation();
                            cx.notify();
                        }
                        "escape" if this.selected_task_id.is_some() => {
                            this.selected_task_id = None;
                            cx.emit(AgentLanesEvent::SelectTask(None));
                            cx.stop_propagation();
                            cx.notify();
                        }
                        _ => {}
                    }
                }))
                .when(is_done, |d| {
                    d.child(pro_check_draw(
                        pro_eid(&self.id, format!("lane-chk-{tid}")),
                        true,
                        12.0,
                        false,
                        p.green_ink,
                    ))
                })
                .when(!task.tools.is_empty(), |d| d.child(tools_row))
                .child(div().min_w_0().truncate().child(title_txt))
                .when_some(inline_controls, |d, ctrl| d.child(ctrl));

            track_box = track_box.child(block);
        }

        // Click-to-inspect popover (`ask` decision card for `Waiting`, or dark summary card for other states!)
        let active_selected = self
            .selected_task_id
            .as_ref()
            .and_then(|sid| self.tasks.iter().find(|t| &t.id == sid).cloned());
        let pop_is_open = active_selected.is_some();
        let selected_snapshot = active_selected.or_else(|| self.last_selected_task.clone());
        let selected_agent_name = selected_snapshot.as_ref().and_then(|st| {
            self.agents
                .iter()
                .find(|a| a.id == st.agent_id)
                .map(|a| a.display_name(cx).to_string())
        });
        let selected_lane_idx = selected_snapshot
            .as_ref()
            .and_then(|st| agent_row_map.get(st.agent_id.as_ref()).copied())
            .unwrap_or(0);

        let pop_el: Option<AnyElement> = if let Some(sel_task) = selected_snapshot {
            let st_id = sel_task.id.clone();
            let st_title = sel_task.display_title(cx).to_string();
            if sel_task.status == LaneTaskStatus::Waiting {
                let ask_txt = sel_task
                    .display_ask(cx)
                    .unwrap_or_else(|| tr(cx, "Approve this step?", "确认批准此操作？"))
                    .to_string();
                let app_id = st_id.clone();
                let rej_id = st_id.clone();
                let btn_rej = pro_btn(
                    pro_eid(&self.id, format!("pop-rej-{st_id}")),
                    tr(cx, "Reject", "拒绝"),
                    None,
                    ProBtnStyle::Secondary,
                    true,
                    cx,
                    move |this, _, cx| {
                        let run_now = this.current_now_ms(
                            disabled_motion(cx) || Theme::global(cx).reduced_motion,
                        );
                        this.reject_task(rej_id.as_ref(), run_now, cx);
                    },
                );
                let btn_app = pro_btn(
                    pro_eid(&self.id, format!("pop-app-{st_id}")),
                    tr(cx, "Approve", "批准"),
                    Some(ProIcon::Check),
                    ProBtnStyle::Primary,
                    true,
                    cx,
                    move |this, _, cx| {
                        this.approve_task(app_id.as_ref(), cx);
                    },
                );
                let place_above = selected_lane_idx >= 2;
                let top_y = if place_above {
                    ((selected_lane_idx as f32) * LANE_H - 96.0).max(4.0)
                } else {
                    (selected_lane_idx as f32 + 1.0) * LANE_H - 4.0
                };
                let card = div()
                    .id("lane-decision-dialog")
                    .role(Role::Dialog)
                    .w(px(284.))
                    .p(px(12.))
                    .rounded(px(10.))
                    .bg(card_bg)
                    .border_1()
                    .border_color(p.line_strong)
                    .shadow_md()
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .child(st_title),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .line_height(px(17.))
                            .text_color(p.text)
                            .child(ask_txt),
                    )
                    .child(
                        div()
                            .mt(px(6.))
                            .flex()
                            .items_center()
                            .justify_end()
                            .gap(px(6.))
                            .child(btn_rej)
                            .child(btn_app),
                    )
                    .into_any_element();
                let mut cell = Some(card);
                Some(
                    div()
                        .absolute()
                        .right(px(8.))
                        .top(px(top_y))
                        .child(pro_popover_presence(
                            pro_eid(&self.id, "lane-ask-pop"),
                            pop_is_open,
                            move || cell.take().unwrap_or_else(|| div().into_any_element()),
                        ))
                        .into_any_element(),
                )
            } else {
                let ag_lbl = selected_agent_name.unwrap_or_default();
                let dur_mins = ((sel_task
                    .end_ms
                    .unwrap_or(now_ms)
                    .saturating_sub(sel_task.start_ms))
                    / 60_000)
                    .max(1);
                let bullets = sel_task.summary.clone();
                let open_tid = st_id.clone();
                let open_lbl = tr(cx, "Open run", "打开运行记录").to_string();
                let ent = cx.entity().clone();
                let tip_bg: Hsla = if p.dark {
                    rgb(0x27272b).into()
                } else {
                    rgb(0x18181b).into()
                };
                let top_y = ((selected_lane_idx as f32 + 1.0) * LANE_H - 4.0)
                    .min(track_h - 68.0)
                    .max(4.0);
                let mut b_col = div().flex().flex_col().gap(px(3.));
                for b in &bullets {
                    b_col = b_col.child(
                        div()
                            .flex()
                            .items_start()
                            .gap(px(6.))
                            .text_size(px(11.5))
                            .line_height(px(16.))
                            .text_color(hsla(0., 0., 1., 0.84))
                            .child(
                                div()
                                    .mt(px(6.))
                                    .size(px(3.5))
                                    .flex_none()
                                    .rounded_full()
                                    .bg(hsla(0., 0., 1., 0.5)),
                            )
                            .child(b.clone()),
                    );
                }
                let card = div()
                    .max_w(px(248.))
                    .px(px(10.))
                    .py(px(8.))
                    .rounded(px(8.))
                    .bg(tip_bg)
                    .shadow_md()
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(hsla(0., 0., 1., 0.55))
                            .child(format!("{ag_lbl} · {dur_mins}m")),
                    )
                    .child(
                        div()
                            .text_size(px(12.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(rgb(0xffffff))
                            .child(st_title),
                    )
                    .when(!bullets.is_empty(), |d| d.child(b_col))
                    .child(
                        div()
                            .id(SharedString::from(format!("open-run-{open_tid}")))
                            .role(Role::Button)
                            .mt(px(2.))
                            .flex()
                            .items_center()
                            .gap(px(4.))
                            .text_size(px(11.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(rgb(0x82bcff))
                            .cursor_pointer()
                            .on_click(move |_, _, cx| {
                                ent.update(cx, |this, cx| {
                                    cx.emit(AgentLanesEvent::OpenTask(open_tid.clone()));
                                    this.selected_task_id = None;
                                    cx.notify();
                                });
                            })
                            .child(open_lbl)
                            .child(pro_icon(ProIcon::ArrowRight, 11.5, rgb(0x82bcff).into())),
                    )
                    .into_any_element();
                let mut cell = Some(card);
                Some(
                    div()
                        .absolute()
                        .right(px(16.))
                        .top(px(top_y))
                        .child(pro_popover_presence(
                            pro_eid(&self.id, "lane-sum-pop"),
                            pop_is_open,
                            move || cell.take().unwrap_or_else(|| div().into_any_element()),
                        ))
                        .into_any_element(),
                )
            }
        } else {
            None
        };

        let right_col = div()
            .flex_1()
            .min_w_0()
            .pr(px(16.))
            .relative()
            .flex()
            .flex_col()
            .child(top_ticks)
            .child(track_box)
            .when_some(pop_el, |d, pop| d.child(pop));

        let body = div()
            .pt(px(6.))
            .pb(px(12.))
            .flex()
            .items_end()
            .child(agents_col)
            .child(right_col);

        div()
            .id(self.id.clone())
            .w_full()
            .flex()
            .justify_center()
            .child(
                div()
                    .w_full()
                    .max_w(px(800.))
                    .rounded(px(10.))
                    .bg(card_bg)
                    .border_1()
                    .border_color(card_border)
                    .shadow(pro_card_shadow(p.dark))
                    .flex()
                    .flex_col()
                    .child(bar_header)
                    .child(body),
            )
            .into_any_element()
    }
}

fn lane_brand_is_model(brand: &str) -> bool {
    let lower = brand.trim().to_ascii_lowercase();
    matches!(
        lower.as_str(),
        "claude"
            | "anthropic"
            | "openai"
            | "gpt"
            | "codex"
            | "gemini"
            | "deepseek"
            | "mistral"
            | "qwen"
            | "meta"
            | "llama"
            | "xai"
            | "grok"
    )
}

fn lane_agent_avatar_seed(agent: &LaneAgent) -> &'static str {
    if let Some(logo) = &agent.avatar {
        let l = logo.to_ascii_lowercase();
        if l.contains("claude") || l.contains("anthropic") {
            return "claude";
        }
        if l.contains("openai") || l.contains("gpt") || l.contains("codex") {
            return "gpt";
        }
        if l.contains("gemini") {
            return "gemini";
        }
        if l.contains("deepseek") {
            return "deepseek";
        }
        if l.contains("mistral") {
            return "mistral";
        }
        if l.contains("qwen") {
            return "qwen";
        }
    }
    if let Some(m) = &agent.model {
        let ml = m.to_ascii_lowercase();
        if ml.contains("claude")
            || ml.contains("opus")
            || ml.contains("sonnet")
            || ml.contains("haiku")
        {
            return "claude";
        }
        if ml.contains("gpt") || ml.contains("openai") || ml.contains("codex") {
            return "gpt";
        }
        if ml.contains("gemini") {
            return "gemini";
        }
        if ml.contains("deepseek") {
            return "deepseek";
        }
        if ml.contains("mistral") {
            return "mistral";
        }
        if ml.contains("qwen") {
            return "qwen";
        }
    }
    let combined = format!("{} {}", agent.id, agent.name).to_ascii_lowercase();
    if combined.contains("ops") || combined.contains("review") || combined.contains("gemini") {
        "gemini"
    } else if combined.contains("code") || combined.contains("gpt") {
        "gpt"
    } else {
        "claude"
    }
}

#[cfg(test)]
mod tests {
    use super::{AgentLanes, LaneTaskStatus};
    use gpui::{AppContext as _, TestAppContext};

    #[gpui::test]
    fn agent_lanes_tracks_running_and_waiting_tasks_and_handles_inline_approvals(
        cx: &mut TestAppContext,
    ) {
        cx.update(crate::init);
        let lanes = cx.new(|cx| AgentLanes::new("test-lanes", cx));

        lanes.update(cx, |v, cx| {
            assert_eq!(v.active_counts(), (2, 1));
            v.approve_task("t-stripe", cx);
            assert_eq!(v.active_counts(), (3, 0));
            assert_eq!(v.tasks()[5].status, LaneTaskStatus::Running);
        });

        let lanes_rej = cx.new(|cx| AgentLanes::new("test-lanes-rej", cx));
        lanes_rej.update(cx, |v, cx| {
            v.reject_task("t-stripe", 1_800_000, cx);
            assert_eq!(v.active_counts(), (2, 0));
            assert_eq!(v.tasks()[5].status, LaneTaskStatus::Failed);
        });
    }
}
