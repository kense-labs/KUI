//! Hardware-logic-style timing diagram for a multi-agent run with rounded step waveforms and handoffs.
//!
//! Plots a left-to-right time axis with one continuous rounded step-waveform per participant
//! (`You`, `Lead`, `Coder`, `Tester`, `Reviewer`, `Docs`) via [`PathBuilder`] and vertical
//! gradient fills: baseline = `Idle`, low step = `Thinking`, high step = `Tool call` (with tool
//! name underneath), dashed amber step = `Waiting on you`, pulsing dots on `"now"`, and vertical
//! dashed hand-off connectors with interactive hover/keyboard inspection.

use crate::{
    components::pro_chrome::{
        ProPalette, pro_agent_avatar, pro_card_shadow, pro_check_draw, pro_ease, pro_eid,
        pro_empty_state, pro_icon_btn, pro_person_avatar, pro_popover_presence, pro_rolling_number,
        pro_skeleton_bar, pro_spring_f32, tr,
    },
    foundation::Theme,
    primitives::{
        family::{Slide, disabled_motion},
        pro_icons::ProIcon,
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, collections::HashMap, f32::consts::TAU, rc::Rc};
use web_time::Instant;

const ROW_H: f32 = 64.0;
const BASELINE_Y: f32 = 52.0;
const LEVEL_THINKING: f32 = 16.0;
const LEVEL_WAITING: f32 = 25.0;
const LEVEL_TOOL: f32 = 36.0;
const GUTTER_W: f32 = 128.0;
const AXIS_H: f32 = 26.0;

/// Kind of participant row in [`AgentTiming`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TimingParticipantKind {
    /// Human operator rendered with a full 3x3 ink mark.
    Person,
    /// Autonomous agent rendered with its deterministic 3x3 pixel mark.
    #[default]
    Agent,
}

/// One row participant in [`AgentTiming`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimingParticipant {
    /// Unique participant id referenced by [`TimingSegment::who`] and [`TimingHandoff`].
    pub id: SharedString,
    /// Display name (`"You"`, `"Lead"`, `"Coder"`).
    pub name: SharedString,
    /// Optional Chinese translation.
    pub name_zh: Option<SharedString>,
    /// Person or Agent kind.
    pub kind: TimingParticipantKind,
    /// Optional custom RGB hex override (`0xRRGGBB`).
    pub color_hex: Option<u32>,
}

impl TimingParticipant {
    /// Create an agent [`TimingParticipant`].
    pub fn agent(id: impl Into<SharedString>, name: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            name_zh: None,
            kind: TimingParticipantKind::Agent,
            color_hex: None,
        }
    }

    /// Create a human [`TimingParticipant`].
    pub fn person(id: impl Into<SharedString>, name: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            name_zh: None,
            kind: TimingParticipantKind::Person,
            color_hex: None,
        }
    }

    /// Set a localized Chinese name.
    pub fn with_name_zh(mut self, zh: impl Into<SharedString>) -> Self {
        self.name_zh = Some(zh.into());
        self
    }

    /// Override the signal line color with a custom hex (`0xRRGGBB`).
    pub fn with_color_hex(mut self, hex: u32) -> Self {
        self.color_hex = Some(hex);
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

/// Waveform signal level of a [`TimingSegment`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TimingState {
    /// Low step (`Thinking`).
    #[default]
    Thinking,
    /// High step (`Tool call`, displays tool name underneath the top step edge).
    Tool,
    /// Dashed amber step (`Waiting on you`).
    Waiting,
}

impl TimingState {
    fn level_px(self) -> f32 {
        match self {
            Self::Thinking => LEVEL_THINKING,
            Self::Waiting => LEVEL_WAITING,
            Self::Tool => LEVEL_TOOL,
        }
    }

    fn badge_label(self, cx: &App) -> &'static str {
        match self {
            Self::Thinking => tr(cx, "Thinking", "思考中"),
            Self::Tool => tr(cx, "Tool call", "工具调用"),
            Self::Waiting => tr(cx, "Waiting on you", "等待您确认"),
        }
    }
}

/// One non-idle waveform step interval along a participant's signal line.
#[derive(Clone, Debug, PartialEq)]
pub struct TimingSegment {
    /// Optional stable id.
    pub id: Option<SharedString>,
    /// Owning [`TimingParticipant::id`].
    pub who: SharedString,
    /// Waveform state (`Thinking`, `Tool`, or `Waiting`).
    pub state: TimingState,
    /// Start time in seconds from run start.
    pub start: f32,
    /// End time in seconds (`None` means still running up to `now`).
    pub end: Option<f32>,
    /// Tool name or thought summary (`"search_code"`, `"Planning the fix"`).
    pub label: Option<SharedString>,
    /// Optional Chinese translation of `label` (keep tool identifiers untranslated when code).
    pub label_zh: Option<SharedString>,
}

impl TimingSegment {
    /// Create a [`TimingSegment`] for `who` with `state` starting at `start` seconds.
    pub fn new(who: impl Into<SharedString>, state: TimingState, start: f32) -> Self {
        Self {
            id: None,
            who: who.into(),
            state,
            start,
            end: None,
            label: None,
            label_zh: None,
        }
    }

    /// Set the segment end time in seconds (`None` stretches live to `now`).
    pub fn with_end(mut self, end: f32) -> Self {
        self.end = Some(end);
        self
    }

    /// Set the step label (tool name or thought description).
    pub fn with_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set a localized Chinese step label.
    pub fn with_label_zh(mut self, zh: impl Into<SharedString>) -> Self {
        self.label_zh = Some(zh.into());
        self
    }

    fn display_label<'a>(&'a self, cx: &App) -> Option<&'a str> {
        match (&self.label, &self.label_zh) {
            (Some(en), Some(zh)) => Some(tr(cx, en.as_ref(), zh.as_ref())),
            (Some(en), None) => Some(en.as_ref()),
            _ => None,
        }
    }
}

/// Vertical dashed hand-off signal between two participant rows at `at` seconds.
#[derive(Clone, Debug, PartialEq)]
pub struct TimingHandoff {
    /// Sender participant id.
    pub from: SharedString,
    /// Receiver participant id.
    pub to: SharedString,
    /// Timestamp in seconds from run start.
    pub at: f32,
    /// Hand-off message summary shown on hover.
    pub label: Option<SharedString>,
    /// Optional Chinese translation.
    pub label_zh: Option<SharedString>,
}

impl TimingHandoff {
    /// Create a [`TimingHandoff`] from `from` to `to` at `at` seconds.
    pub fn new(from: impl Into<SharedString>, to: impl Into<SharedString>, at: f32) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            at,
            label: None,
            label_zh: None,
        }
    }

    /// Attach a hand-off summary label.
    pub fn with_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Attach a localized Chinese summary label.
    pub fn with_label_zh(mut self, zh: impl Into<SharedString>) -> Self {
        self.label_zh = Some(zh.into());
        self
    }

    fn display_label<'a>(&'a self, cx: &App) -> Option<&'a str> {
        match (&self.label, &self.label_zh) {
            (Some(en), Some(zh)) => Some(tr(cx, en.as_ref(), zh.as_ref())),
            (Some(en), None) => Some(en.as_ref()),
            _ => None,
        }
    }
}

/// Inspected element inside [`AgentTiming`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimingInspection {
    /// Segment index in `segments`.
    Segment(usize),
    /// Handoff index in `handoffs`.
    Handoff(usize),
}

/// Typed events emitted by [`AgentTiming`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgentTimingEvent {
    /// Hovered or keyboard-focused inspection target changed.
    Inspect(Option<TimingInspection>),
    /// Replay started from `0:00`.
    ReplayStarted,
}

impl EventEmitter<AgentTimingEvent> for AgentTiming {}

/// Sample a continuous rounded step-waveform polyline `(x_px, y_px)` across `[0.0 ..= max_x]` for one row!
pub(crate) fn build_row_waveform_points(
    segments: &[&TimingSegment],
    now_secs: f32,
    duration_secs: f32,
    plot_w: f32,
    row_top: f32,
) -> Vec<(f32, f32)> {
    let dur = duration_secs.max(0.1);
    let end_x = ((now_secs / dur).clamp(0., 1.)) * plot_w;
    let base_y = row_top + BASELINE_Y;
    if end_x <= 0.5 {
        return vec![(0.0, base_y), (end_x.max(0.5), base_y)];
    }

    // Collect sorted active intervals clipped to `[0.0 .. now_secs]`
    let mut intervals: Vec<(f32, f32, f32, bool)> = segments
        .iter()
        .filter(|s| s.start < now_secs - 0.01)
        .map(|s| {
            let s0 = s.start.max(0.0);
            let s1 = s.end.unwrap_or(now_secs).min(now_secs);
            let still_open = s.end.is_none_or(|e| e > now_secs - 0.005);
            let x0 = (s0 / dur).clamp(0., 1.) * plot_w;
            let x1 = (s1 / dur).clamp(0., 1.) * plot_w;
            (
                x0,
                x1.max(x0 + 1.0),
                base_y - s.state.level_px(),
                still_open,
            )
        })
        .collect();
    intervals.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    let ramp = 3.2f32;
    let mut pts: Vec<(f32, f32)> = Vec::with_capacity(intervals.len() * 18 + 4);
    let mut cur_x = 0.0f32;
    let mut cur_y = base_y;
    pts.push((0.0, base_y));

    let push_smooth_step =
        |pts: &mut Vec<(f32, f32)>, x_edge: f32, y0: f32, y1: f32, r_half: f32| {
            if (y1 - y0).abs() < 0.4 {
                pts.push((x_edge, y1));
                return;
            }
            let x_a = (x_edge - r_half).max(pts.last().map(|p| p.0).unwrap_or(0.0));
            let x_b = (x_edge + r_half).min(end_x);
            if x_a > pts.last().map(|p| p.0).unwrap_or(0.0) + 0.2 {
                pts.push((x_a, y0));
            }
            for k in 1..=8 {
                let u = k as f32 / 8.0;
                let inv = 1.0 - u;
                let bx = inv * inv * inv * x_a
                    + 3.0 * inv * inv * u * x_edge
                    + 3.0 * inv * u * u * x_edge
                    + u * u * u * x_b;
                let by = inv * inv * inv * y0
                    + 3.0 * inv * inv * u * y0
                    + 3.0 * inv * u * u * y1
                    + u * u * u * y1;
                pts.push((bx, by));
            }
        };

    for (idx, &(x0, x1, target_y, still_open)) in intervals.iter().enumerate() {
        // Return to baseline first if there is an idle gap since previous interval
        if x0 > cur_x + 1.5 && (cur_y - base_y).abs() > 0.4 {
            push_smooth_step(&mut pts, cur_x, cur_y, base_y, ramp);
            cur_y = base_y;
        }
        // Step from `cur_y` to `target_y` at `x0`
        push_smooth_step(&mut pts, x0, cur_y, target_y, ramp);
        cur_y = target_y;
        cur_x = x1.min(end_x);

        let next_touches = intervals
            .get(idx + 1)
            .is_some_and(|&(nx0, _, _, _)| (nx0 - x1).abs() <= 2.0);
        if next_touches {
            // Stay at `cur_y` until next interval smoothly steps at `nx0`
            let hold_to = (cur_x - ramp).max(pts.last().map(|p| p.0).unwrap_or(cur_x));
            if hold_to > pts.last().map(|p| p.0).unwrap_or(0.0) + 0.2 {
                pts.push((hold_to, cur_y));
            }
        } else if still_open && (end_x - cur_x).abs() <= 1.5 {
            // Still running right on the `now` line: hold high level all the way to `end_x`!
            pts.push((end_x, cur_y));
            cur_x = end_x;
        } else {
            // Step back down to baseline at `cur_x`
            push_smooth_step(&mut pts, cur_x, cur_y, base_y, ramp);
            cur_y = base_y;
        }
    }

    if end_x > pts.last().map(|p| p.0).unwrap_or(0.0) + 0.2 {
        pts.push((end_x, cur_y));
    }
    pts
}

fn format_secs_axis(secs: f32) -> String {
    let s = secs.max(0.0).round() as u32;
    format!("{}:{:02}", s / 60, s % 60)
}

fn participant_palette_color(idx: usize, part: &TimingParticipant, p: &ProPalette) -> Hsla {
    if let Some(hex) = part.color_hex {
        return rgb(hex).into();
    }
    if part.kind == TimingParticipantKind::Person {
        return p.ink;
    }
    let agent_colors = [p.blue, p.purple, p.teal, p.green, p.pink, p.orange];
    agent_colors[idx % agent_colors.len()]
}

fn timing_avatar_seed(part: &TimingParticipant) -> &'static str {
    let combined = format!("{} {}", part.id, part.name).to_ascii_lowercase();
    if combined.contains("claude")
        || combined.contains("lead")
        || combined.contains("doc")
        || combined.contains("write")
        || combined.contains("plan")
    {
        "claude"
    } else if combined.contains("gemini")
        || combined.contains("review")
        || combined.contains("audit")
        || combined.contains("ops")
    {
        "gemini"
    } else if combined.contains("deepseek") {
        "deepseek"
    } else if combined.contains("mistral") {
        "mistral"
    } else if combined.contains("qwen") {
        "qwen"
    } else {
        "gpt"
    }
}

fn render_timing_participant_mark(
    part: &TimingParticipant,
    color: Hsla,
    busy: bool,
    elapsed_ms: f32,
    cx: &App,
) -> AnyElement {
    if part.kind == TimingParticipantKind::Person {
        pro_person_avatar(part.name.as_ref(), 20.0, true, None, cx)
    } else {
        pro_agent_avatar(timing_avatar_seed(part), color, 20.0, busy, elapsed_ms, cx)
    }
}

fn default_timing_participants() -> Vec<TimingParticipant> {
    vec![
        TimingParticipant::person("you", "You").with_name_zh("您"),
        TimingParticipant::agent("lead", "Lead").with_name_zh("主控"),
        TimingParticipant::agent("coder", "Coder").with_name_zh("编码"),
        TimingParticipant::agent("tester", "Tester").with_name_zh("测试"),
        TimingParticipant::agent("reviewer", "Reviewer").with_name_zh("审查"),
        TimingParticipant::agent("docs", "Docs").with_name_zh("文档"),
    ]
}

fn default_timing_segments() -> Vec<TimingSegment> {
    vec![
        TimingSegment::new("you", TimingState::Thinking, 0.2)
            .with_end(2.0)
            .with_label("Prompt: negative cart total")
            .with_label_zh("提问：负数购物车总额缺陷"),
        TimingSegment::new("lead", TimingState::Thinking, 2.0)
            .with_end(3.8)
            .with_label("Plan reproduction & patch")
            .with_label_zh("规划复现与修补方案"),
        TimingSegment::new("lead", TimingState::Tool, 3.8)
            .with_end(10.0)
            .with_label("search_code"),
        TimingSegment::new("coder", TimingState::Thinking, 10.0)
            .with_end(12.2)
            .with_label("Inspect coupon rounding")
            .with_label_zh("排查优惠券舍入逻辑"),
        TimingSegment::new("coder", TimingState::Tool, 12.2)
            .with_end(18.6)
            .with_label("edit_file"),
        TimingSegment::new("tester", TimingState::Tool, 18.6)
            .with_end(22.6)
            .with_label("vitest"),
        TimingSegment::new("coder", TimingState::Tool, 22.6)
            .with_end(28.6)
            .with_label("apply_patch"),
        TimingSegment::new("lead", TimingState::Thinking, 28.6)
            .with_end(29.4)
            .with_label("Summarize diff")
            .with_label_zh("汇总变更差异"),
        // Dashed amber step waiting on user!
        TimingSegment::new("lead", TimingState::Waiting, 29.4)
            .with_label("Approve hotfix PR #419")
            .with_label_zh("请批准热修复 PR #419"),
        // Parallel running steps at `now`!
        TimingSegment::new("tester", TimingState::Tool, 27.6).with_label("playwright"),
        TimingSegment::new("reviewer", TimingState::Thinking, 28.6)
            .with_end(30.4)
            .with_label("Audit negative floor")
            .with_label_zh("审计下界保护"),
        TimingSegment::new("reviewer", TimingState::Tool, 30.4).with_label("gh_diff"),
        TimingSegment::new("docs", TimingState::Thinking, 31.0)
            .with_label("Draft changelog")
            .with_label_zh("起草变更日志"),
    ]
}

fn default_timing_handoffs() -> Vec<TimingHandoff> {
    vec![
        TimingHandoff::new("you", "lead", 2.0)
            .with_label("Fix the cart total bug")
            .with_label_zh("修复购物车金额为负的缺陷"),
        TimingHandoff::new("lead", "coder", 10.0)
            .with_label("Clamp subtotal in cart.ts")
            .with_label_zh("在 cart.ts 中限制小计下界"),
        TimingHandoff::new("coder", "tester", 18.6)
            .with_label("Run unit suite on cart.ts")
            .with_label_zh("对 cart.ts 运行单元测试"),
        TimingHandoff::new("tester", "coder", 22.6)
            .with_label("1 edge case failed on stacked coupons")
            .with_label_zh("叠加优惠券场景 1 项用例失败"),
        TimingHandoff::new("coder", "reviewer", 28.6)
            .with_label("Review updated guard")
            .with_label_zh("审查更新后的守卫逻辑"),
        TimingHandoff::new("lead", "you", 29.4)
            .with_label("Ready to merge PR #419?")
            .with_label_zh("确认合并 PR #419？"),
    ]
}

/// Hardware-logic-style multi-agent timing diagram component.
///
/// # Examples
///
/// ```no_run
/// # #[allow(unused_variables, unused_imports)]
/// # fn example(window: &mut gpui::Window, cx: &mut gpui::App) {
/// # use gpui::prelude::*;
/// use gpui::prelude::*;
/// use kui::components::agent_timing::{
///     AgentTiming, AgentTimingEvent, TimingHandoff, TimingParticipant, TimingSegment, TimingState,
/// };
///
/// let timing = cx.new(|cx| {
///     AgentTiming::new("cart-timing", cx)
///         .with_title("Fix the cart total")
///         .with_participants(vec![
///             TimingParticipant::person("you", "You"),
///             TimingParticipant::agent("lead", "Lead"),
///             TimingParticipant::agent("coder", "Coder"),
///         ])
///         .with_segments(vec![
///             TimingSegment::new("lead", TimingState::Thinking, 0.5).with_end(4.0).with_label("Planning the fix"),
///             TimingSegment::new("lead", TimingState::Tool, 4.0).with_end(8.5).with_label("search_code"),
///             TimingSegment::new("coder", TimingState::Tool, 9.0).with_label("edit_file"),
///         ])
///         .with_handoffs(vec![
///             TimingHandoff::new("you", "lead", 0.4).with_label("Fix the cart total bug"),
///             TimingHandoff::new("lead", "coder", 8.8).with_label("Fix rounding in cart.ts"),
///         ])
/// });
/// cx.subscribe(&timing, |_, ev: &AgentTimingEvent, _| {
///     let _ = ev;
/// }).detach();
/// # }
/// ```
pub struct AgentTiming {
    id: ElementId,
    title: Option<SharedString>,
    participants: Vec<TimingParticipant>,
    segments: Vec<TimingSegment>,
    handoffs: Vec<TimingHandoff>,
    duration_secs: f32,
    base_now_secs: f32,
    live: bool,
    replay_mode: bool,
    inspected: Option<TimingInspection>,
    last_inspected: Option<TimingInspection>,
    hovered_who: Option<SharedString>,
    plot_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    scrubber_frac: Option<f32>,
    scrubber_slide: Slide,
    started_at: Instant,
    loading: bool,
    empty: bool,
}

impl AgentTiming {
    /// Create an [`AgentTiming`] logic diagram on `"Fix the cart total"` across 6 participants.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        Self {
            id: id.into(),
            title: Some("Fix the cart total".into()),
            participants: default_timing_participants(),
            segments: default_timing_segments(),
            handoffs: default_timing_handoffs(),
            duration_secs: 40.0,
            base_now_secs: 35.6,
            live: true,
            replay_mode: false,
            inspected: None,
            last_inspected: None,
            hovered_who: None,
            plot_bounds: Rc::new(Cell::new(None)),
            scrubber_frac: None,
            scrubber_slide: Slide::snappy(0.5),
            started_at: Instant::now(),
            loading: false,
            empty: false,
        }
    }

    /// Override the diagram title in the top bar.
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the participant rows (`You`, agents, etc.).
    pub fn with_participants(mut self, participants: Vec<TimingParticipant>) -> Self {
        self.participants = participants;
        self
    }

    /// Set all waveform step segments (in seconds from run start).
    pub fn with_segments(mut self, segments: Vec<TimingSegment>) -> Self {
        self.segments = segments;
        self
    }

    /// Set vertical handoff arrows between participants.
    pub fn with_handoffs(mut self, handoffs: Vec<TimingHandoff>) -> Self {
        self.handoffs = handoffs;
        self
    }

    /// Set the `"now"` position in seconds from run start.
    pub fn with_now(mut self, now_secs: f32) -> Self {
        self.base_now_secs = now_secs.max(0.0);
        self.replay_mode = false;
        self
    }

    /// Set the horizontal time axis span in seconds (`40.0` by default).
    pub fn with_duration(mut self, duration_secs: f32) -> Self {
        self.duration_secs = duration_secs.max(5.0);
        self
    }

    /// Enable or pause live progression toward `duration_secs`.
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

    /// Current simulated `now` in seconds.
    pub fn current_now_secs(&self, reduced: bool) -> f32 {
        let dur = self.duration_secs.max(1.0);
        if self.replay_mode && !reduced {
            (self.started_at.elapsed().as_secs_f32() * 5.0).min(dur)
        } else if self.live && !reduced {
            (self.base_now_secs + self.started_at.elapsed().as_secs_f32() * 0.65).min(dur)
        } else {
            self.base_now_secs.min(dur)
        }
    }

    /// Set or clear the inspected step/handoff readout card.
    pub fn set_inspected(&mut self, target: Option<TimingInspection>, cx: &mut Context<Self>) {
        if self.inspected != target {
            if let Some(t) = target {
                self.last_inspected = Some(t);
            }
            self.inspected = target;
            cx.emit(AgentTimingEvent::Inspect(target));
            cx.notify();
        }
    }

    /// Replay the timing diagram from `0:00`.
    pub fn replay(&mut self, cx: &mut Context<Self>) {
        self.replay_mode = true;
        self.live = true;
        self.inspected = None;
        self.started_at = Instant::now();
        cx.emit(AgentTimingEvent::ReplayStarted);
        cx.notify();
    }
}

impl Render for AgentTiming {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let reduced = disabled_motion(cx) || Theme::global(cx).reduced_motion;
        let dur = self.duration_secs.max(1.0);
        let now_s = self.current_now_secs(reduced);
        let wall_ms = self.started_at.elapsed().as_secs_f32() * 1000.0;

        let any_open_at_now = self
            .segments
            .iter()
            .any(|s| s.start <= now_s && s.end.is_none_or(|e| e > now_s - 0.01));
        if ((self.live || self.replay_mode) && (any_open_at_now || now_s < dur - 0.02) && !reduced)
            || (self.loading && !reduced)
        {
            window.request_animation_frame();
        }
        if self.replay_mode && now_s >= dur - 0.02 {
            self.replay_mode = false;
            self.base_now_secs = dur;
        }

        let card_bg: Hsla = if p.dark {
            rgb(0x1c1c1e).into()
        } else {
            rgb(0xffffff).into()
        };
        let card_border: Hsla = if p.dark {
            hsla(0., 0., 1., 0.09)
        } else {
            hsla(0., 0., 0., 0.065)
        };
        let amber_c: Hsla = if p.dark {
            rgb(0xffb35c).into()
        } else {
            rgb(0xe08a00).into()
        };
        let blue_c: Hsla = if p.dark {
            rgb(0x6cbcff).into()
        } else {
            rgba(0x0060e2c7).into()
        };

        let waiting_now = self.segments.iter().any(|s| {
            s.state == TimingState::Waiting
                && s.start <= now_s
                && s.end.is_none_or(|e| e > now_s - 0.01)
        });
        let visible_handoffs_count = self.handoffs.iter().filter(|h| h.at <= now_s).count();

        let title_txt: SharedString = self
            .title
            .as_deref()
            .map(|t| {
                if t == "Fix the cart total" {
                    tr(cx, "Fix the cart total", "修复购物车总价计算")
                } else {
                    t
                }
            })
            .unwrap_or_else(|| tr(cx, "Agent timing", "智能体时序波形"))
            .into();

        let handoff_unit = if tr(cx, "en", "zh") == "zh" {
            "次交接"
        } else if visible_handoffs_count == 1 {
            "hand-off"
        } else {
            "hand-offs"
        };

        let header = div()
            .mb(px(8.))
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
                    .text_color(p.strong)
                    .child(title_txt),
            )
            .child(
                div()
                    .flex_none()
                    .ml_auto()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .text_size(px(12.))
                    .text_color(p.muted)
                    .when(waiting_now, |d| {
                        d.child(
                            div()
                                .h(px(20.))
                                .px(px(7.))
                                .rounded(px(5.))
                                .bg(amber_c.opacity(0.14))
                                .text_color(amber_c)
                                .text_size(px(11.5))
                                .font_weight(FontWeight::MEDIUM)
                                .flex()
                                .items_center()
                                .child(tr(cx, "Waiting on you", "等待您确认")),
                        )
                    })
                    .child(
                        div()
                            .flex()
                            .items_baseline()
                            .gap(px(4.))
                            .child(pro_rolling_number(
                                pro_eid(&self.id, "atg-ho-cnt"),
                                visible_handoffs_count as f64,
                                0,
                                "",
                                "",
                                12.0,
                                FontWeight::MEDIUM,
                                p.ink,
                            ))
                            .child(handoff_unit),
                    )
                    .child(
                        div()
                            .min_w(px(32.))
                            .text_right()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(format_secs_axis(now_s)),
                    )
                    .child(pro_icon_btn(
                        pro_eid(&self.id, "atg-replay"),
                        tr(cx, "Replay timing diagram", "重放时序图"),
                        ProIcon::Refresh,
                        22.,
                        false,
                        cx,
                        |this, _, cx| {
                            this.replay(cx);
                        },
                    )),
            );

        if self.loading {
            let phase = if reduced {
                0.5
            } else {
                (wall_ms / 1100.0).fract()
            };
            let mut skel = div().py(px(8.)).flex().flex_col().gap(px(12.));
            for i in 0..4usize {
                skel = skel.child(
                    div()
                        .h(px(40.))
                        .flex()
                        .items_center()
                        .gap(px(16.))
                        .child(pro_skeleton_bar(96., 14., 4., phase + i as f32 * 0.1, cx))
                        .child(pro_skeleton_bar(480., 28., 5., phase + i as f32 * 0.1, cx)),
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
                        .max_w(px(780.))
                        .pt(px(14.))
                        .pr(px(18.))
                        .pb(px(10.))
                        .pl(px(16.))
                        .rounded(px(10.))
                        .bg(card_bg)
                        .border_1()
                        .border_color(card_border)
                        .shadow(pro_card_shadow(p.dark))
                        .flex()
                        .flex_col()
                        .child(header)
                        .child(skel),
                )
                .into_any_element();
        }

        if self.empty || self.participants.is_empty() {
            let empty_el = pro_empty_state(
                ProIcon::Activity,
                tr(cx, "Nothing happened yet", "尚无时序信号"),
                tr(
                    cx,
                    "Participant signal lines, tool calls, and hand-offs render along the time axis.",
                    "参与者信号波形、工具调用高电平与跨行交接虚线将沿时间轴展开。",
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
                        .max_w(px(780.))
                        .pt(px(14.))
                        .pr(px(18.))
                        .pb(px(10.))
                        .pl(px(16.))
                        .rounded(px(10.))
                        .bg(card_bg)
                        .border_1()
                        .border_color(card_border)
                        .shadow(pro_card_shadow(p.dark))
                        .flex()
                        .flex_col()
                        .child(header)
                        .child(empty_el),
                )
                .into_any_element();
        }

        let mut row_of: HashMap<&str, usize> = HashMap::with_capacity(self.participants.len());
        let mut row_colors: Vec<Hsla> = Vec::with_capacity(self.participants.len());
        let mut agent_palette_idx = 0usize;
        for (idx, part) in self.participants.iter().enumerate() {
            row_of.insert(part.id.as_ref(), idx);
            let col = participant_palette_color(agent_palette_idx, part, &p);
            if part.kind == TimingParticipantKind::Agent {
                agent_palette_idx += 1;
            }
            row_colors.push(col);
        }

        let mut row_alphas: Vec<f32> = Vec::with_capacity(self.participants.len());
        for part in &self.participants {
            let row_dim = if let Some(hw) = &self.hovered_who {
                hw != &part.id
            } else if let Some(ins) = self.inspected {
                match ins {
                    TimingInspection::Segment(si) => {
                        self.segments.get(si).is_some_and(|s| s.who != part.id)
                    }
                    TimingInspection::Handoff(hi) => self
                        .handoffs
                        .get(hi)
                        .is_some_and(|h| h.from != part.id && h.to != part.id),
                }
            } else {
                false
            };
            let ra = pro_spring_f32(
                pro_eid(&self.id, format!("atg-row-a-{}", part.id)),
                if row_dim { 0.34 } else { 1.0 },
                0.20,
                window,
                cx,
            );
            row_alphas.push(ra);
        }

        let mut seg_alphas: Vec<f32> = Vec::with_capacity(self.segments.len());
        for (s_idx, seg) in self.segments.iter().enumerate() {
            let dim = if let Some(hw) = &self.hovered_who {
                hw != &seg.who
            } else {
                self.inspected
                    .is_some_and(|ins| ins != TimingInspection::Segment(s_idx))
            };
            let sa = pro_spring_f32(
                pro_eid(&self.id, format!("atg-seg-a-{s_idx}")),
                if dim { 0.30 } else { 1.0 },
                0.20,
                window,
                cx,
            );
            seg_alphas.push(sa);
        }

        let mut ho_alphas: Vec<f32> = Vec::with_capacity(self.handoffs.len());
        for (h_idx, ho) in self.handoffs.iter().enumerate() {
            let dim = if let Some(hw) = &self.hovered_who {
                hw != &ho.from && hw != &ho.to
            } else {
                self.inspected
                    .is_some_and(|ins| ins != TimingInspection::Handoff(h_idx))
            };
            let ha = pro_spring_f32(
                pro_eid(&self.id, format!("atg-ho-a-{h_idx}")),
                if dim { 0.28 } else { 0.85 },
                0.20,
                window,
                cx,
            );
            ho_alphas.push(ha);
        }

        // Left participant column (`64px` per row: vector mark + name + current live status!)
        let mut names_col = div()
            .w(px(GUTTER_W))
            .pr(px(10.))
            .flex_none()
            .flex()
            .flex_col();
        for (r_idx, part) in self.participants.iter().enumerate() {
            let pid = &part.id;
            let pid_hov = pid.clone();
            let active_seg = self.segments.iter().rfind(|s| {
                &s.who == pid && s.start <= now_s && s.end.is_none_or(|e| e > now_s - 0.01)
            });
            let is_busy = active_seg.is_some();
            let row_c = row_colors[r_idx];
            let (status_txt, status_col) = match active_seg {
                Some(seg) => match seg.state {
                    TimingState::Tool => (
                        seg.display_label(cx)
                            .unwrap_or_else(|| tr(cx, "Tool call", "工具调用"))
                            .to_string(),
                        row_c,
                    ),
                    TimingState::Waiting => {
                        (tr(cx, "Waiting on you", "等待您确认").to_string(), amber_c)
                    }
                    TimingState::Thinking => (tr(cx, "Thinking", "思考中").to_string(), p.text),
                },
                None => (tr(cx, "Idle", "空闲").to_string(), p.faint),
            };

            names_col = names_col.child(
                div()
                    .id(pro_eid(&self.id, format!("atg-who-{pid}")))
                    .h(px(ROW_H))
                    .pt(px(15.))
                    .opacity(row_alphas[r_idx])
                    .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                        let next = if *inside { Some(pid_hov.clone()) } else { None };
                        if this.hovered_who != next {
                            this.hovered_who = next;
                            cx.notify();
                        }
                    }))
                    .flex()
                    .items_start()
                    .gap(px(8.))
                    .child(div().mt(px(1.)).child(render_timing_participant_mark(
                        part, row_c, is_busy, wall_ms, cx,
                    )))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .truncate()
                                    .text_size(px(12.5))
                                    .line_height(px(18.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .child(part.display_name(cx).to_string()),
                            )
                            .child(
                                div()
                                    .truncate()
                                    .text_size(px(11.5))
                                    .line_height(px(16.))
                                    .text_color(status_col)
                                    .child(status_txt),
                            ),
                    ),
            );
        }

        // Prepare plot canvas data + Slide::snappy time-scrubber crosshair
        let scrub_x = self.scrubber_slide.get(window, cx).clamp(0.0, 1.0);
        let scrub_alpha = pro_spring_f32(
            pro_eid(&self.id, "atg-scrub-a"),
            if self.scrubber_frac.is_some() {
                1.0
            } else {
                0.0
            },
            0.18,
            window,
            cx,
        );
        let scrub_line_c = p.ink.opacity(0.32);

        let num_rows = self.participants.len();
        let plot_h = (num_rows as f32) * ROW_H;
        let parts_snapshot = self.participants.clone();
        let segs_snapshot = self.segments.clone();
        let handoffs_snapshot = self.handoffs.clone();
        let row_colors_snap = row_colors.clone();
        let row_alphas_snap = row_alphas.clone();
        let seg_alphas_snap = seg_alphas.clone();
        let ho_alphas_snap = ho_alphas.clone();
        let pulse_phase = if reduced {
            0.0
        } else {
            (wall_ms / 1800.0).fract()
        };
        let grid_c: Hsla = if p.dark {
            hsla(0., 0., 1., 0.05)
        } else {
            hsla(0., 0., 0., 0.038)
        };
        let base_c: Hsla = if p.dark {
            hsla(0., 0., 1., 0.12)
        } else {
            hsla(0., 0., 0., 0.10)
        };
        let bounds_for_prepaint = self.plot_bounds.clone();

        let plot_canvas = canvas(
            move |bounds, _, _| {
                bounds_for_prepaint.set(Some(bounds));
            },
            move |bounds, _, win, _| {
                let pw = f32::from(bounds.size.width);
                let ph = f32::from(bounds.size.height);
                if pw <= 8.0 || ph <= 8.0 {
                    return;
                }
                let ox = f32::from(bounds.left());
                let oy = f32::from(bounds.top());

                // 1) Vertical 10s grid hairlines + horizontal row baselines
                for tick in 1..4usize {
                    let gx = ox + (tick as f32 / 4.0) * pw;
                    let mut pb = PathBuilder::stroke(px(1.0));
                    pb.move_to(point(px(gx), px(oy)));
                    pb.line_to(point(px(gx), px(oy + ph)));
                    if let Ok(path) = pb.build() {
                        win.paint_path(path, grid_c);
                    }
                }
                for r in 0..num_rows {
                    let by = oy + (r as f32) * ROW_H + BASELINE_Y;
                    let mut pb = PathBuilder::stroke(px(1.0));
                    pb.move_to(point(px(ox), px(by)));
                    pb.line_to(point(px(ox + pw), px(by)));
                    if let Ok(path) = pb.build() {
                        win.paint_path(path, base_c);
                    }
                }

                let mut r_lookup: HashMap<&str, usize> = HashMap::new();
                for (i, pt) in parts_snapshot.iter().enumerate() {
                    r_lookup.insert(pt.id.as_ref(), i);
                }

                // 2) Per-segment soft vertical gradient fill + dashed amber top on `Waiting`
                for (s_idx, seg) in segs_snapshot.iter().enumerate() {
                    if seg.start >= now_s - 0.01 {
                        continue;
                    }
                    let Some(&r_idx) = r_lookup.get(seg.who.as_ref()) else {
                        continue;
                    };
                    let alpha_mul = seg_alphas_snap.get(s_idx).copied().unwrap_or(1.0);
                    let s0 = seg.start.max(0.0);
                    let s1 = seg.end.unwrap_or(now_s).min(now_s);
                    let x0 = ox + (s0 / dur).clamp(0., 1.) * pw;
                    let x1 = (ox + (s1 / dur).clamp(0., 1.) * pw).max(x0 + 1.5);
                    let by = oy + (r_idx as f32) * ROW_H + BASELINE_Y;
                    let top_y = by - seg.state.level_px();
                    let fill_col = if seg.state == TimingState::Waiting {
                        amber_c
                    } else {
                        row_colors_snap[r_idx]
                    };

                    let mut area = PathBuilder::fill();
                    area.move_to(point(px(x0), px(by)));
                    area.line_to(point(px(x0), px(top_y + 2.0)));
                    area.line_to(point(px(x1), px(top_y + 2.0)));
                    area.line_to(point(px(x1), px(by)));
                    area.close();
                    if let Ok(path) = area.build() {
                        win.paint_path(
                            path,
                            linear_gradient(
                                180.,
                                linear_color_stop(fill_col.opacity(0.20 * alpha_mul), 0.),
                                linear_color_stop(fill_col.opacity(0.02 * alpha_mul), 1.),
                            ),
                        );
                    }

                    // Dashed amber signal top + vertical risers for `Waiting on you`!
                    if seg.state == TimingState::Waiting {
                        let w_span = x1 - x0;
                        let nd = ((w_span / 6.0).ceil() as usize).max(1);
                        for d in 0..nd {
                            let sx = x0 + (d as f32) * 6.0;
                            let ex = (sx + 3.2).min(x1);
                            if ex > sx {
                                let mut pb = PathBuilder::stroke(px(1.6));
                                pb.move_to(point(px(sx), px(top_y)));
                                pb.line_to(point(px(ex), px(top_y)));
                                if let Ok(path) = pb.build() {
                                    win.paint_path(path, amber_c.opacity(alpha_mul));
                                }
                            }
                        }
                        let mut rise = PathBuilder::stroke(px(1.5));
                        rise.move_to(point(px(x0), px(by)));
                        rise.line_to(point(px(x0), px(top_y)));
                        if let Ok(path) = rise.build() {
                            win.paint_path(path, amber_c.opacity(0.75 * alpha_mul));
                        }
                    }
                }

                // 3) Continuous rounded step-waveform per participant row (`Thinking` & `Tool` + baseline)
                for (r_idx, part) in parts_snapshot.iter().enumerate() {
                    let row_segs: Vec<&TimingSegment> = segs_snapshot
                        .iter()
                        .filter(|s| s.who == part.id && s.state != TimingState::Waiting)
                        .collect();
                    let row_top = (r_idx as f32) * ROW_H;
                    let wf_pts = build_row_waveform_points(&row_segs, now_s, dur, pw, row_top);
                    let row_a = row_alphas_snap.get(r_idx).copied().unwrap_or(1.0);
                    let stroke_c = row_colors_snap[r_idx].opacity(0.92 * row_a);
                    if wf_pts.len() >= 2 {
                        let mut pb = PathBuilder::stroke(px(1.6));
                        for (i, &(x, y)) in wf_pts.iter().enumerate() {
                            let pt = point(px(ox + x), px(oy + y));
                            if i == 0 {
                                pb.move_to(pt);
                            } else {
                                pb.line_to(pt);
                            }
                        }
                        if let Ok(path) = pb.build() {
                            win.paint_path(path, stroke_c);
                        }
                    }

                    // Pulsing dot on `"now"` for any active open segment on this row!
                    if let Some(open_seg) = segs_snapshot.iter().rfind(|s| {
                        s.who == part.id
                            && s.start <= now_s
                            && s.end.is_none_or(|e| e > now_s - 0.01)
                    }) {
                        let dot_x = ox + (now_s / dur).clamp(0., 1.) * pw;
                        let dot_y = oy + row_top + BASELINE_Y - open_seg.state.level_px();
                        let dot_c = if open_seg.state == TimingState::Waiting {
                            amber_c
                        } else {
                            row_colors_snap[r_idx]
                        };
                        if !reduced {
                            let pr = 3.2 + 7.5 * pro_ease(pulse_phase);
                            let pa = (1.0 - pulse_phase) * 0.42;
                            paint_timing_disc(
                                point(px(dot_x), px(dot_y)),
                                pr,
                                dot_c.opacity(pa),
                                transparent_black(),
                                0.,
                                win,
                            );
                        }
                        paint_timing_disc(
                            point(px(dot_x), px(dot_y)),
                            3.3,
                            dot_c,
                            card_bg,
                            1.5,
                            win,
                        );
                    }
                }

                // 4) Vertical dashed hand-off arrows between rows + landing dots!
                for (h_idx, ho) in handoffs_snapshot.iter().enumerate() {
                    if ho.at > now_s {
                        continue;
                    }
                    let Some(&r_from) = r_lookup.get(ho.from.as_ref()) else {
                        continue;
                    };
                    let Some(&r_to) = r_lookup.get(ho.to.as_ref()) else {
                        continue;
                    };
                    let ho_a = ho_alphas_snap.get(h_idx).copied().unwrap_or(0.72);
                    let draw_t = if reduced {
                        1.0
                    } else {
                        pro_ease(((now_s - ho.at) / 0.35).clamp(0., 1.))
                    };
                    let hx = ox + (ho.at / dur).clamp(0., 1.) * pw;
                    let y0 = oy + (r_from as f32) * ROW_H + BASELINE_Y;
                    let y1_full = oy + (r_to as f32) * ROW_H + BASELINE_Y;
                    let y1 = y0 + (y1_full - y0) * draw_t;
                    let hc = row_colors_snap[r_from].opacity(ho_a);

                    let dist = (y1 - y0).abs();
                    let dir = (y1 - y0).signum();
                    let nd = ((dist / 6.0).ceil() as usize).max(1);
                    for d in 0..nd {
                        let s = (d as f32) * 6.0;
                        let e = (s + 3.0).min(dist);
                        if e > s {
                            let mut pb = PathBuilder::stroke(px(1.15));
                            pb.move_to(point(px(hx), px(y0 + dir * s)));
                            pb.line_to(point(px(hx), px(y0 + dir * e)));
                            if let Ok(path) = pb.build() {
                                win.paint_path(path, hc);
                            }
                        }
                    }
                    if draw_t >= 0.85 {
                        paint_timing_disc(
                            point(px(hx), px(y1_full)),
                            2.6,
                            card_bg,
                            row_colors_snap[r_from].opacity((ho_a * 1.15).min(0.95)),
                            1.35,
                            win,
                        );
                    }
                }

                // 5) Smooth `Slide::snappy` time-scrubber crosshair on hover
                if scrub_alpha > 0.01 {
                    let sx = ox + scrub_x * pw;
                    let mut spb = PathBuilder::stroke(px(1.0));
                    spb.move_to(point(px(sx), px(oy)));
                    spb.line_to(point(px(sx), px(oy + ph + 4.0)));
                    if let Ok(path) = spb.build() {
                        win.paint_path(path, scrub_line_c.opacity(scrub_line_c.a * scrub_alpha));
                    }
                }

                // 6) Vertical `"now"` line
                let nx = ox + (now_s / dur).clamp(0., 1.) * pw;
                let mut now_pb = PathBuilder::stroke(px(1.0));
                now_pb.move_to(point(px(nx), px(oy)));
                now_pb.line_to(point(px(nx), px(oy + ph + 4.0)));
                if let Ok(path) = now_pb.build() {
                    win.paint_path(path, blue_c.opacity(0.55));
                }
            },
        )
        .absolute()
        .inset_0();

        let bounds_for_move = self.plot_bounds.clone();
        let mut plot_box = div()
            .id(pro_eid(&self.id, "atg-plot"))
            .role(Role::Group)
            .aria_label(tr(
                cx,
                "Timing diagram steps and hand-offs; use arrow keys to inspect",
                "智能体时序波形与交接信号；可使用方向键逐步检视",
            ))
            .tab_index(0)
            .w_full()
            .h(px(plot_h))
            .relative()
            .focus_visible(move |s| s.border_1().border_color(p.focus))
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
            .on_key_down(cx.listener(move |this, ev: &KeyDownEvent, _, cx| {
                let vis_segs: Vec<usize> = this
                    .segments
                    .iter()
                    .enumerate()
                    .filter(|(_, s)| s.start <= now_s)
                    .map(|(i, _)| i)
                    .collect();
                if vis_segs.is_empty() {
                    return;
                }
                let cur_pos = match this.inspected {
                    Some(TimingInspection::Segment(si)) => vis_segs.iter().position(|&x| x == si),
                    _ => None,
                };
                match ev.keystroke.key.as_str() {
                    "right" | "arrowright" | "down" | "arrowdown" => {
                        let next_pos = match cur_pos {
                            Some(pos) => (pos + 1).min(vis_segs.len() - 1),
                            None => 0,
                        };
                        this.set_inspected(Some(TimingInspection::Segment(vis_segs[next_pos])), cx);
                        cx.stop_propagation();
                    }
                    "left" | "arrowleft" | "up" | "arrowup" => {
                        let next_pos = match cur_pos {
                            Some(pos) => pos.saturating_sub(1),
                            None => vis_segs.len() - 1,
                        };
                        this.set_inspected(Some(TimingInspection::Segment(vis_segs[next_pos])), cx);
                        cx.stop_propagation();
                    }
                    "home" => {
                        this.set_inspected(Some(TimingInspection::Segment(vis_segs[0])), cx);
                        cx.stop_propagation();
                    }
                    "end" => {
                        if let Some(&last) = vis_segs.last() {
                            this.set_inspected(Some(TimingInspection::Segment(last)), cx);
                        }
                        cx.stop_propagation();
                    }
                    "escape" => {
                        this.set_inspected(None, cx);
                        cx.stop_propagation();
                    }
                    _ => {}
                }
            }))
            .child(plot_canvas);

        // Interactive hit-boxes + inline tool/state labels tucked underneath high steps!
        for (s_idx, seg) in self.segments.iter().enumerate() {
            if seg.start >= now_s - 0.01 {
                continue;
            }
            let Some(&r_idx) = row_of.get(seg.who.as_ref()) else {
                continue;
            };
            let s0 = seg.start.max(0.0);
            let s1 = seg.end.unwrap_or(now_s).min(now_s);
            let seg_finished = seg.end.is_some_and(|e| e <= now_s);
            let left_f = (s0 / dur).clamp(0., 0.99);
            let width_f = ((s1 - s0).max(0.4) / dur).clamp(0.015, 1.0 - left_f);
            let step_h = seg.state.level_px();
            let top_px = (r_idx as f32) * ROW_H + BASELINE_Y - step_h;
            let row_c = row_colors[r_idx];
            let seg_a = seg_alphas.get(s_idx).copied().unwrap_or(1.0);

            let inline_label: Option<String> = if width_f >= 0.085 {
                match seg.state {
                    TimingState::Tool => seg.display_label(cx).map(str::to_string),
                    TimingState::Waiting => {
                        Some(tr(cx, "Waiting on you", "等待您确认").to_string())
                    }
                    TimingState::Thinking if width_f >= 0.14 => {
                        seg.display_label(cx).map(str::to_string)
                    }
                    TimingState::Thinking => None,
                }
            } else {
                None
            };

            let lbl_color = match seg.state {
                TimingState::Tool => row_c,
                TimingState::Waiting => amber_c,
                TimingState::Thinking => p.text,
            };

            plot_box = plot_box.child(
                div()
                    .id(pro_eid(&self.id, format!("atg-seg-{s_idx}")))
                    .absolute()
                    .left(relative(left_f))
                    .w(relative(width_f))
                    .top(px(top_px))
                    .h(px(step_h))
                    .px(px(4.))
                    .pt(px(4.))
                    .opacity(seg_a)
                    .cursor_pointer()
                    .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                        if *inside {
                            this.set_inspected(Some(TimingInspection::Segment(s_idx)), cx);
                        } else if this.inspected == Some(TimingInspection::Segment(s_idx)) {
                            this.set_inspected(None, cx);
                        }
                    }))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        let target = Some(TimingInspection::Segment(s_idx));
                        let next = if this.inspected == target {
                            None
                        } else {
                            target
                        };
                        this.set_inspected(next, cx);
                    }))
                    .when_some(inline_label, |d, lbl| {
                        d.child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(3.))
                                .min_w_0()
                                .when(seg_finished && width_f >= 0.17, |row| {
                                    row.child(pro_check_draw(
                                        pro_eid(&self.id, format!("atg-seg-chk-{s_idx}")),
                                        true,
                                        11.5,
                                        false,
                                        p.green_ink,
                                    ))
                                })
                                .child(
                                    div()
                                        .min_w_0()
                                        .truncate()
                                        .text_size(px(10.5))
                                        .line_height(px(15.))
                                        .font_weight(if seg.state == TimingState::Thinking {
                                            FontWeight::NORMAL
                                        } else {
                                            FontWeight::MEDIUM
                                        })
                                        .text_color(lbl_color)
                                        .child(lbl),
                                ),
                        )
                    }),
            );
        }

        // Interactive hover hit-strips on vertical handoff wires!
        for (h_idx, ho) in self.handoffs.iter().enumerate() {
            if ho.at > now_s {
                continue;
            }
            let Some(&r_from) = row_of.get(ho.from.as_ref()) else {
                continue;
            };
            let Some(&r_to) = row_of.get(ho.to.as_ref()) else {
                continue;
            };
            let y_min = (r_from.min(r_to) as f32) * ROW_H + BASELINE_Y - 4.0;
            let y_max = (r_from.max(r_to) as f32) * ROW_H + BASELINE_Y + 4.0;
            let x_frac = (ho.at / dur).clamp(0.01, 0.99);

            plot_box = plot_box.child(
                div()
                    .id(pro_eid(&self.id, format!("atg-ho-{h_idx}")))
                    .absolute()
                    .left(relative(x_frac))
                    .ml(px(-5.))
                    .w(px(10.))
                    .top(px(y_min))
                    .h(px((y_max - y_min).max(12.)))
                    .cursor_pointer()
                    .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                        if *inside {
                            this.set_inspected(Some(TimingInspection::Handoff(h_idx)), cx);
                        } else if this.inspected == Some(TimingInspection::Handoff(h_idx)) {
                            this.set_inspected(None, cx);
                        }
                    })),
            );
        }

        // Hover / Keyboard Step & Handoff Readout Card (`pro_popover_presence`)
        let tip_open = self.inspected.is_some();
        let tip_target = self.inspected.or(self.last_inspected);
        type TipTuple = (String, String, Hsla, String, String, f32, usize, bool);
        let tip_data: Option<TipTuple> = match tip_target {
            Some(TimingInspection::Segment(si)) => self.segments.get(si).and_then(|seg| {
                let r_i = *row_of.get(seg.who.as_ref())?;
                let who_name = self.participants[r_i].display_name(cx).to_string();
                let badge = seg.state.badge_label(cx).to_string();
                let col = if seg.state == TimingState::Waiting {
                    amber_c
                } else {
                    row_colors[r_i]
                };
                let lbl = seg
                    .display_label(cx)
                    .unwrap_or_else(|| seg.state.badge_label(cx))
                    .to_string();
                let end_v = seg.end.unwrap_or(now_s).min(now_s);
                let took = (end_v - seg.start).max(0.0);
                let is_done_seg = seg.end.is_some_and(|e| e <= now_s);
                let meta = if is_done_seg {
                    format!(
                        "{} – {} · {:.1}s",
                        format_secs_axis(seg.start),
                        format_secs_axis(end_v),
                        took
                    )
                } else {
                    format!(
                        "{} – {} · {:.1}s ({})",
                        format_secs_axis(seg.start),
                        tr(cx, "now", "此刻"),
                        took,
                        tr(cx, "running", "进行中")
                    )
                };
                Some((
                    who_name,
                    badge,
                    col,
                    lbl,
                    meta,
                    (seg.start / dur).clamp(0., 1.),
                    r_i,
                    is_done_seg,
                ))
            }),
            Some(TimingInspection::Handoff(hi)) => self.handoffs.get(hi).and_then(|ho| {
                let r_from = *row_of.get(ho.from.as_ref())?;
                let r_to = *row_of.get(ho.to.as_ref())?;
                let head = format!(
                    "{} → {}",
                    self.participants[r_from].display_name(cx),
                    self.participants[r_to].display_name(cx)
                );
                let badge = tr(cx, "Hand-off", "交接").to_string();
                let lbl = ho
                    .display_label(cx)
                    .unwrap_or_else(|| tr(cx, "Context hand-off", "上下文交接"))
                    .to_string();
                let meta = format!(
                    "{} {}",
                    tr(cx, "Sent at", "发送于"),
                    format_secs_axis(ho.at)
                );
                Some((
                    head,
                    badge,
                    row_colors[r_from],
                    lbl,
                    meta,
                    (ho.at / dur).clamp(0., 1.),
                    r_from.min(r_to),
                    true,
                ))
            }),
            None => None,
        };

        if let Some((head_s, badge_s, col, body_s, meta_s, ux, r_i, show_done_chk)) = tip_data {
            let align_right = ux > 0.58;
            let top_y = ((r_i as f32) * ROW_H + 4.0).min(plot_h - 76.0).max(2.0);
            let smooth_ux = pro_spring_f32(
                pro_eid(&self.id, "atg-tip-ux"),
                ux.clamp(0.02, 0.52),
                0.20,
                window,
                cx,
            );
            let smooth_top =
                pro_spring_f32(pro_eid(&self.id, "atg-tip-top"), top_y, 0.20, window, cx);
            let chk_id = pro_eid(&self.id, "atg-tip-chk");
            let card = div()
                .min_w(px(184.))
                .max_w(px(264.))
                .px(px(11.))
                .py(px(9.))
                .rounded(px(10.))
                .bg(card_bg)
                .border_1()
                .border_color(p.line_strong)
                .shadow_md()
                .flex()
                .flex_col()
                .gap(px(3.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(
                            div()
                                .truncate()
                                .text_size(px(12.5))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.ink)
                                .child(head_s),
                        )
                        .child(
                            div()
                                .ml_auto()
                                .h(px(18.))
                                .px(px(6.))
                                .flex_none()
                                .rounded(px(4.))
                                .bg(col.opacity(0.14))
                                .text_color(col)
                                .text_size(px(11.))
                                .font_weight(FontWeight::MEDIUM)
                                .flex()
                                .items_center()
                                .gap(px(4.))
                                .when(show_done_chk, |d| {
                                    d.child(pro_check_draw(chk_id, true, 12.0, false, col))
                                })
                                .child(badge_s),
                        ),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .line_height(px(16.))
                        .text_color(p.text)
                        .child(body_s),
                )
                .child(div().text_size(px(11.5)).text_color(p.muted).child(meta_s))
                .into_any_element();
            let mut cell = Some(card);
            let pop_inner =
                pro_popover_presence(pro_eid(&self.id, "atg-tip"), tip_open, move || {
                    cell.take().unwrap_or_else(|| div().into_any_element())
                });
            let pos_wrap = div().absolute().top(px(smooth_top));
            plot_box = plot_box.child(if align_right {
                pos_wrap.right(px(8.)).child(pop_inner)
            } else {
                pos_wrap
                    .left(relative(smooth_ux.clamp(0.02, 0.52)))
                    .child(pop_inner)
            });
        }

        // Bottom X-axis (`0:00`, `0:10`, `0:20`, `0:30`, `0:40` + live blue `"now"` pill + scrubber pill!)
        let now_frac = (now_s / dur).clamp(0., 1.);
        let mut x_axis = div().w_full().h(px(AXIS_H)).relative();
        for step in 0..=4usize {
            let frac = step as f32 / 4.0;
            // Hide static tick when the moving `now` pill sits right on top of it
            let collides_with_now = (frac - now_frac).abs() < 0.065;
            if !collides_with_now {
                let tick_s = dur * frac;
                let mut t_el = div()
                    .absolute()
                    .top(px(6.))
                    .text_size(px(11.))
                    .line_height(px(16.))
                    .text_color(p.faint)
                    .child(format_secs_axis(tick_s));
                t_el = if step == 0 {
                    t_el.left_0()
                } else if step == 4 {
                    t_el.right_0()
                } else {
                    t_el.left(relative(frac)).ml(px(-12.))
                };
                x_axis = x_axis.child(t_el);
            }
        }
        x_axis = x_axis.child(
            div()
                .absolute()
                .top(px(5.))
                .left(relative(now_frac.clamp(0.03, 0.94)))
                .ml(px(-16.))
                .h(px(18.))
                .px(px(6.))
                .rounded(px(5.))
                .bg(blue_c.opacity(0.14))
                .text_color(blue_c)
                .text_size(px(11.))
                .font_weight(FontWeight::MEDIUM)
                .flex()
                .items_center()
                .child(format_secs_axis(now_s)),
        );
        if scrub_alpha > 0.01 {
            x_axis = x_axis.child(
                div()
                    .absolute()
                    .top(px(4.))
                    .left(relative(scrub_x.clamp(0.03, 0.94)))
                    .ml(px(-16.))
                    .h(px(18.))
                    .px(px(6.))
                    .rounded(px(5.))
                    .bg(card_bg)
                    .border_1()
                    .border_color(p.line_strong)
                    .opacity(scrub_alpha)
                    .text_color(p.ink)
                    .text_size(px(10.5))
                    .font_weight(FontWeight::MEDIUM)
                    .flex()
                    .items_center()
                    .child(format_secs_axis(scrub_x * dur)),
            );
        }

        let right_col = div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .child(plot_box)
            .child(x_axis);

        div()
            .id(self.id.clone())
            .w_full()
            .flex()
            .justify_center()
            .child(
                div()
                    .w_full()
                    .max_w(px(780.))
                    .pt(px(14.))
                    .pr(px(18.))
                    .pb(px(10.))
                    .pl(px(16.))
                    .rounded(px(10.))
                    .bg(card_bg)
                    .border_1()
                    .border_color(card_border)
                    .shadow(pro_card_shadow(p.dark))
                    .flex()
                    .flex_col()
                    .child(header)
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .items_start()
                            .child(names_col)
                            .child(right_col),
                    ),
            )
            .into_any_element()
    }
}

fn paint_timing_disc(
    center: Point<Pixels>,
    radius: f32,
    fill_c: Hsla,
    stroke_c: Hsla,
    stroke_w: f32,
    win: &mut Window,
) {
    if radius <= 0.3 {
        return;
    }
    let steps = 20usize;
    if fill_c.a > 0.001 {
        let mut disc = PathBuilder::fill();
        for i in 0..=steps {
            let a = (i as f32 / steps as f32) * TAU;
            let pt = point(
                center.x + px(radius * a.cos()),
                center.y + px(radius * a.sin()),
            );
            if i == 0 {
                disc.move_to(pt);
            } else {
                disc.line_to(pt);
            }
        }
        disc.close();
        if let Ok(path) = disc.build() {
            win.paint_path(path, fill_c);
        }
    }
    if stroke_w > 0.1 {
        let mut ring = PathBuilder::stroke(px(stroke_w));
        for i in 0..=steps {
            let a = (i as f32 / steps as f32) * TAU;
            let pt = point(
                center.x + px(radius * a.cos()),
                center.y + px(radius * a.sin()),
            );
            if i == 0 {
                ring.move_to(pt);
            } else {
                ring.line_to(pt);
            }
        }
        if let Ok(path) = ring.build() {
            win.paint_path(path, stroke_c);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AgentTiming, BASELINE_Y, LEVEL_THINKING, LEVEL_TOOL, TimingInspection, TimingSegment,
        TimingState, build_row_waveform_points,
    };
    use gpui::{AppContext as _, TestAppContext};

    #[test]
    fn waveform_steps_from_baseline_to_thinking_and_tool_levels() {
        let s1 = TimingSegment::new("lead", TimingState::Thinking, 2.0).with_end(6.0);
        let s2 = TimingSegment::new("lead", TimingState::Tool, 6.0).with_end(10.0);
        let pts = build_row_waveform_points(&[&s1, &s2], 14.0, 20.0, 400.0, 0.0);
        assert!(pts.len() > 10);
        // Starts and ends on baseline (`52.0`)
        assert!((pts.first().expect("first").1 - BASELINE_Y).abs() < 0.1);
        assert!((pts.last().expect("last").1 - BASELINE_Y).abs() < 0.1);
        // Reaches `Tool` high level (`52.0 - 36.0 = 16.0`)
        let min_y = pts.iter().map(|p| p.1).fold(f32::INFINITY, f32::min);
        assert!((min_y - (BASELINE_Y - LEVEL_TOOL)).abs() < 0.5);
        assert!(LEVEL_TOOL > LEVEL_THINKING);
    }

    #[gpui::test]
    fn agent_timing_inspects_segments_and_replays(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let view = cx.new(|cx| AgentTiming::new("test-timing", cx));
        view.update(cx, |v, cx| {
            v.set_inspected(Some(TimingInspection::Segment(2)), cx);
            assert_eq!(v.inspected, Some(TimingInspection::Segment(2)));
            v.replay(cx);
            assert!(v.replay_mode);
            assert_eq!(v.inspected, None);
        });
    }
}
