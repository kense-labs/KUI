//! Quiet vertical step trail for an autonomous agent run with branching sub-steps and approval gates.
//!
//! Renders a compact progress header (`"3/5"`), completed steps with tinted checks and durations,
//! an active step with a spinning arc, shimmering status verb, live elapsed timer, S-curved
//! sub-step spine with a traveling fading light pulse, and dashed amber rings on approval gates.

use crate::{
    components::pro_chrome::{
        ProBtnStyle, ProPalette, pro_btn, pro_check_draw, pro_ease, pro_eid, pro_empty_state,
        pro_fold, pro_rolling_number, pro_shimmer_text, pro_skeleton_bar, pro_spring_bar,
        pro_tactile, tr,
    },
    primitives::{
        family::{PhaseClock, disabled_motion},
        pro_icons::{ProIcon, pro_app_artwork, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{
    collections::{HashMap, HashSet},
    f32::consts::TAU,
};
use web_time::Instant;

const STEP_ROW_H: f32 = 32.0;
const SUB_ROW_H: f32 = 24.0;
const MAIN_RAIL_X: f32 = 17.0;
const SUB_RAIL_X: f32 = 29.0;
const LOOP_SECS: f32 = 2.40;

const FAIL_MARK_SVG: &str = r#"<path d="M5.5 5.5L10.5 10.5M10.5 5.5L5.5 10.5" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"></path>"#;
const PAUSE_MARK_SVG: &str = r#"<path d="M6.4 5.4V10.6M9.6 5.4V10.6" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"></path>"#;
const DASH_MARK_SVG: &str = r#"<path d="M5.3 8H10.7" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"></path>"#;

/// Execution state of a top-level [`TrailStep`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TrailStepState {
    /// Step completed with tinted check and duration.
    Done,
    /// Step currently executing with spinning arc, shimmering status, and live timer.
    Running,
    /// Upcoming step not yet reached (`gate: true` renders a dashed amber ring).
    #[default]
    Pending,
    /// Step paused awaiting human approval (`Needs you`).
    Waiting,
    /// Step terminated with an error.
    Failed,
    /// Approval step declined by the user (struck-through title).
    Declined,
}

/// Execution state of a nested [`TrailSubstep`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TrailSubstepState {
    /// Completed sub-step.
    #[default]
    Done,
    /// Active sub-step receiving the traveling rail light pulse.
    Running,
    /// Upcoming sub-step.
    Pending,
}

/// One nested operation inside a [`TrailStep`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrailSubstep {
    /// Optional stable id.
    pub id: Option<SharedString>,
    /// Concise action summary (`"Found the empty-cart case"`).
    pub title: SharedString,
    /// Optional Chinese translation.
    pub title_zh: Option<SharedString>,
    /// Right-aligned compact fact (`"cart.ts:142"`, `"+18 −3"`, `"182/214"`).
    pub detail: Option<SharedString>,
    /// Sub-step execution state (`Done`, `Running`, or `Pending`).
    pub state: TrailSubstepState,
}

impl TrailSubstep {
    /// Create a [`TrailSubstep`] (`Done` by default).
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            id: None,
            title: title.into(),
            title_zh: None,
            detail: None,
            state: TrailSubstepState::Done,
        }
    }

    /// Set a localized Chinese sub-step title.
    pub fn with_title_zh(mut self, zh: impl Into<SharedString>) -> Self {
        self.title_zh = Some(zh.into());
        self
    }

    /// Set the right-aligned detail token (`"cart.ts:142"`, `"+18 −3"`).
    pub fn with_detail(mut self, detail: impl Into<SharedString>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Set the sub-step state (`Done`, `Running`, or `Pending`).
    pub fn with_state(mut self, state: TrailSubstepState) -> Self {
        self.state = state;
        self
    }

    #[allow(dead_code)]
    fn display_title<'a>(&'a self, cx: &App) -> &'a str {
        if self.title.is_empty() {
            return tr(cx, "Untitled step", "未命名子步骤");
        }
        if let Some(zh) = &self.title_zh {
            tr(cx, self.title.as_ref(), zh.as_ref())
        } else {
            self.title.as_ref()
        }
    }
}

/// One main step along an [`AgentTrail`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrailStep {
    /// Stable identifier used to track expanded disclosure state.
    pub id: SharedString,
    /// Step title (`"Patch the checkout form"`).
    pub title: SharedString,
    /// Optional Chinese translation.
    pub title_zh: Option<SharedString>,
    /// Execution state (`Done`, `Running`, `Pending`, `Waiting`, `Failed`, `Declined`).
    pub state: TrailStepState,
    /// Optional tool/provider key rendered inside a 20px circular medallion (`"linear"`, `"vercel"`, `"github"`, `"notion"`).
    pub tool: Option<SharedString>,
    /// Formatted or millisecond duration for completed steps (`"8s"`, `"34s"`).
    pub duration: Option<SharedString>,
    /// Present-participle verb shown shimmering while `Running` (`"Testing"`, `"Editing"`).
    pub status: Option<SharedString>,
    /// Optional Chinese translation for `status`.
    pub status_zh: Option<SharedString>,
    /// Initial elapsed seconds offset for live running timer display.
    pub elapsed_offset_secs: u64,
    /// Whether this step requires human approval before proceeding (renders dashed amber ring).
    pub gate: bool,
    /// Prompt shown above `Approve` / `Decline` when inspecting a gated or waiting step.
    pub ask: Option<SharedString>,
    /// Optional Chinese translation for `ask`.
    pub ask_zh: Option<SharedString>,
    /// Nested sub-steps executed within this step.
    pub substeps: Vec<TrailSubstep>,
}

impl TrailStep {
    /// Create a [`TrailStep`] with `id`, `title`, and `state`.
    pub fn new(
        id: impl Into<SharedString>,
        title: impl Into<SharedString>,
        state: TrailStepState,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            title_zh: None,
            state,
            tool: None,
            duration: None,
            status: None,
            status_zh: None,
            elapsed_offset_secs: 24,
            gate: false,
            ask: None,
            ask_zh: None,
            substeps: Vec::new(),
        }
    }

    /// Set a localized Chinese title.
    pub fn with_title_zh(mut self, zh: impl Into<SharedString>) -> Self {
        self.title_zh = Some(zh.into());
        self
    }

    /// Set the circular tool logo key (`"linear"`, `"vercel"`, `"github"`, `"notion"`).
    pub fn with_tool(mut self, tool: impl Into<SharedString>) -> Self {
        self.tool = Some(tool.into());
        self
    }

    /// Set the completed duration label (`"8s"`, `"34s"`).
    pub fn with_duration(mut self, duration: impl Into<SharedString>) -> Self {
        self.duration = Some(duration.into());
        self
    }

    /// Set the present-tense status label for a `Running` step (`"Testing"`, `"Editing"`).
    pub fn with_status(mut self, status: impl Into<SharedString>) -> Self {
        self.status = Some(status.into());
        self
    }

    /// Set the localized Chinese status label.
    pub fn with_status_zh(mut self, zh: impl Into<SharedString>) -> Self {
        self.status_zh = Some(zh.into());
        self
    }

    /// Set the base elapsed seconds shown on the live running clock.
    pub fn with_elapsed_offset_secs(mut self, secs: u64) -> Self {
        self.elapsed_offset_secs = secs;
        self
    }

    /// Mark this step as an approval gate (`dashed` amber ring + `"Approval"` / `"Needs you"`).
    pub fn with_gate(mut self, gate: bool) -> Self {
        self.gate = gate;
        self
    }

    /// Provide the approval prompt shown above `Decline` / `Approve`.
    pub fn with_ask(mut self, ask: impl Into<SharedString>) -> Self {
        self.ask = Some(ask.into());
        self
    }

    /// Provide a localized Chinese approval prompt.
    pub fn with_ask_zh(mut self, zh: impl Into<SharedString>) -> Self {
        self.ask_zh = Some(zh.into());
        self
    }

    /// Attach nested sub-steps to this step.
    pub fn with_substeps(mut self, substeps: Vec<TrailSubstep>) -> Self {
        self.substeps = substeps;
        self
    }

    #[allow(dead_code)]
    fn display_title<'a>(&'a self, cx: &App) -> &'a str {
        if self.title.is_empty() {
            return tr(cx, "Untitled step", "未命名步骤");
        }
        if let Some(zh) = &self.title_zh {
            tr(cx, self.title.as_ref(), zh.as_ref())
        } else {
            self.title.as_ref()
        }
    }

    fn display_status<'a>(&'a self, cx: &App) -> &'a str {
        match (&self.status, &self.status_zh) {
            (Some(en), Some(zh)) => tr(cx, en.as_ref(), zh.as_ref()),
            (Some(en), None) => en.as_ref(),
            _ => tr(cx, "Working", "执行中"),
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

/// Agent attribution chip displayed in the [`AgentTrail`] header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrailAgent {
    /// Display name (`"Claude Code"`, `"Kense Coder"`).
    pub name: SharedString,
    /// Provider artwork key (`"claude"`, `"openai"`, `"gemini"`, `"kense"`).
    pub logo: Option<SharedString>,
}

impl TrailAgent {
    /// Create a [`TrailAgent`] chip.
    pub fn new(name: impl Into<SharedString>, logo: impl Into<SharedString>) -> Self {
        Self {
            name: name.into(),
            logo: Some(logo.into()),
        }
    }
}

/// Events emitted by [`AgentTrail`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentTrailEvent {
    /// User toggled sub-step disclosure on `step_id` (`expanded` is the new open state).
    ToggleExpanded {
        step_id: SharedString,
        expanded: bool,
    },
    /// User approved a gated/waiting step.
    Approve(SharedString),
    /// User declined a gated/waiting step.
    Decline(SharedString),
}

impl EventEmitter<AgentTrailEvent> for AgentTrail {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum VisibleSubstepItem {
    FoldEarlier(usize),
    Substep(usize),
    FoldMore(usize),
    ShowFewer,
}

/// Compute visible sub-step slice: lists > 6 fold to 5 items around the running sub-step unless `unfolded` is true.
pub(crate) fn compute_visible_substeps(
    substeps: &[TrailSubstep],
    unfolded: bool,
) -> Vec<VisibleSubstepItem> {
    let n = substeps.len();
    if n <= 6 || unfolded {
        let mut out: Vec<VisibleSubstepItem> = (0..n).map(VisibleSubstepItem::Substep).collect();
        if n > 6 && unfolded {
            out.push(VisibleSubstepItem::ShowFewer);
        }
        return out;
    }

    let anchor = substeps
        .iter()
        .position(|s| s.state == TrailSubstepState::Running)
        .unwrap_or(n - 1);
    let win_len = 5usize;
    let half = win_len / 2;
    let mut start = anchor.saturating_sub(half);
    if start + win_len > n {
        start = n - win_len;
    }
    let end = start + win_len;

    let mut out = Vec::with_capacity(win_len + 2);
    if start > 0 {
        out.push(VisibleSubstepItem::FoldEarlier(start));
    }
    for idx in start..end {
        out.push(VisibleSubstepItem::Substep(idx));
    }
    if end < n {
        out.push(VisibleSubstepItem::FoldMore(n - end));
    }
    out
}

/// Format elapsed seconds cleanly (`8s`, `1m 04s`, `1h 02m`).
pub fn format_trail_secs(secs: u64) -> String {
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3_600 {
        let m = secs / 60;
        let s = secs % 60;
        if s == 0 {
            format!("{m}m")
        } else {
            format!("{m}m {s}s")
        }
    } else {
        let h = secs / 3_600;
        let m = (secs % 3_600) / 60;
        format!("{h}h {m}m")
    }
}

fn default_trail_steps() -> Vec<TrailStep> {
    vec![
        TrailStep::new("read", "Read the bug report", TrailStepState::Done)
            .with_title_zh("阅读缺陷报告")
            .with_tool("linear")
            .with_duration("8s"),
        TrailStep::new("repro", "Reproduce the crash", TrailStepState::Done)
            .with_title_zh("复现结算页崩溃")
            .with_tool("vercel")
            .with_duration("34s"),
        TrailStep::new("isolate", "Isolate null cart state", TrailStepState::Done)
            .with_title_zh("定位空购物车边界状态")
            .with_tool("github")
            .with_duration("19s"),
        TrailStep::new("patch", "Patch the checkout form", TrailStepState::Running)
            .with_title_zh("修补结算表单逻辑")
            .with_tool("github")
            .with_status("Testing")
            .with_status_zh("测试中")
            .with_elapsed_offset_secs(24)
            .with_substeps(vec![
                TrailSubstep::new("Found the empty-cart case")
                    .with_title_zh("定位空购物车触发分支")
                    .with_detail("cart.ts:142")
                    .with_state(TrailSubstepState::Done),
                TrailSubstep::new("Added a guard and 2 tests")
                    .with_title_zh("补充空值守卫与 2 项测试")
                    .with_detail("+18 −3")
                    .with_state(TrailSubstepState::Done),
                TrailSubstep::new("Running the test suite")
                    .with_title_zh("运行回归测试套件")
                    .with_detail("182/214")
                    .with_state(TrailSubstepState::Running),
            ]),
        TrailStep::new("pr", "Open a pull request", TrailStepState::Pending)
            .with_title_zh("发起合并请求")
            .with_tool("github")
            .with_gate(true)
            .with_ask("Open PR #482 against main – 3 files, 214 tests passing")
            .with_ask_zh("向 main 分支发起 PR #482 —— 涉及 3 个文件，214 项测试通过"),
    ]
}

/// Quiet vertical progress trail for an autonomous agent run.
pub struct AgentTrail {
    id: ElementId,
    title: Option<SharedString>,
    show_header: bool,
    agent: Option<TrailAgent>,
    steps: Vec<TrailStep>,
    expanded_ids: HashSet<SharedString>,
    unfolded_sub_ids: HashSet<SharedString>,
    open_ask_id: Option<SharedString>,
    hovered_step: Option<SharedString>,
    pressed_step: Option<SharedString>,
    done_clocks: HashMap<SharedString, PhaseClock>,
    started_at: Instant,
    loading: bool,
    empty: bool,
}

impl AgentTrail {
    /// Create an [`AgentTrail`] preloaded with `"Fix the checkout crash"` (`3/5` done).
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let steps = default_trail_steps();
        let mut expanded_ids = HashSet::new();
        for step in &steps {
            if step.state == TrailStepState::Running && !step.substeps.is_empty() {
                expanded_ids.insert(step.id.clone());
            }
        }
        Self {
            id: id.into(),
            title: Some("Fix the checkout crash".into()),
            show_header: true,
            agent: Some(TrailAgent::new("Claude Code", "claude")),
            steps,
            expanded_ids,
            unfolded_sub_ids: HashSet::new(),
            open_ask_id: None,
            hovered_step: None,
            pressed_step: None,
            done_clocks: HashMap::new(),
            started_at: Instant::now(),
            loading: false,
            empty: false,
        }
    }

    /// Set the task title in the top header (`None` hides the header bar).
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self.show_header = true;
        self
    }

    /// Hide the top header bar and render only the step trail.
    pub fn without_header(mut self) -> Self {
        self.show_header = false;
        self
    }

    /// Configure or omit (`None`) the agent badge in the header.
    pub fn with_agent(mut self, agent: Option<TrailAgent>) -> Self {
        self.agent = agent;
        self
    }

    /// Replace the step list (automatically expands any `Running` step with sub-steps).
    pub fn with_steps(mut self, steps: Vec<TrailStep>) -> Self {
        self.expanded_ids.clear();
        self.open_ask_id = None;
        for step in &steps {
            if step.state == TrailStepState::Running && !step.substeps.is_empty() {
                self.expanded_ids.insert(step.id.clone());
            }
            if step.state == TrailStepState::Waiting {
                self.open_ask_id = Some(step.id.clone());
            }
        }
        self.steps = steps;
        self
    }

    /// Explicitly set which step ids start with their sub-steps expanded.
    pub fn with_expanded<I, S>(mut self, ids: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        self.expanded_ids = ids.into_iter().map(Into::into).collect();
        self
    }

    /// Render skeleton placeholder rows.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Force the `"No steps yet"` empty state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Return the current steps slice.
    pub fn steps(&self) -> &[TrailStep] {
        &self.steps
    }

    /// Count completed (`Done`) steps and total steps (`(done, total)`).
    pub fn progress_counts(&self) -> (usize, usize) {
        let done = self
            .steps
            .iter()
            .filter(|s| s.state == TrailStepState::Done)
            .count();
        (done, self.steps.len())
    }

    /// Toggle sub-step expansion for `step_id`.
    pub fn toggle_step_expanded(&mut self, step_id: &str, cx: &mut Context<Self>) {
        let key: SharedString = step_id.to_string().into();
        let now_open = if self.expanded_ids.contains(&key) {
            self.expanded_ids.remove(&key);
            false
        } else {
            self.expanded_ids.insert(key.clone());
            true
        };
        cx.emit(AgentTrailEvent::ToggleExpanded {
            step_id: key,
            expanded: now_open,
        });
        cx.notify();
    }

    /// Approve a gated or waiting step, advancing it to `Done`.
    pub fn approve_step(&mut self, step_id: &str, cx: &mut Context<Self>) {
        if let Some(step) = self.steps.iter_mut().find(|s| s.id.as_ref() == step_id) {
            step.state = TrailStepState::Done;
            if step.duration.is_none() {
                step.duration = Some("2s".into());
            }
            let sid = step.id.clone();
            self.done_clocks.entry(sid.clone()).or_default().restart(cx);
            if self.open_ask_id.as_deref() == Some(step_id) {
                self.open_ask_id = None;
            }
            cx.emit(AgentTrailEvent::Approve(sid));
            cx.notify();
        }
    }

    /// Decline a gated or waiting step, marking it `Declined` with struck-through title.
    pub fn decline_step(&mut self, step_id: &str, cx: &mut Context<Self>) {
        if let Some(step) = self.steps.iter_mut().find(|s| s.id.as_ref() == step_id) {
            step.state = TrailStepState::Declined;
            if self.open_ask_id.as_deref() == Some(step_id) {
                self.open_ask_id = None;
            }
            cx.emit(AgentTrailEvent::Decline(step.id.clone()));
            cx.notify();
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_step_marker(
    check_id: ElementId,
    done_drawn: bool,
    kind_state: TrailStepState,
    gate: bool,
    spin_angle: f32,
    accent: Hsla,
    wash: Hsla,
    orange: Hsla,
    amber_ink: Hsla,
    amber_wash: Hsla,
    red: Hsla,
    red_wash: Hsla,
    line_col: Hsla,
    card_bg: Hsla,
    muted: Hsla,
) -> AnyElement {
    match kind_state {
        TrailStepState::Done => div()
            .size(px(16.))
            .rounded_full()
            .bg(wash)
            .flex()
            .items_center()
            .justify_center()
            .child(pro_check_draw(check_id, done_drawn, 13.5, false, accent))
            .into_any_element(),
        TrailStepState::Running => canvas(
            |_, _, _| (),
            move |bounds, _, win, _| {
                let c = bounds.center();
                let r = 6.25;
                let steps = 28usize;
                let mut track = PathBuilder::stroke(px(1.55));
                for i in 0..=steps {
                    let a = (i as f32 / steps as f32) * TAU;
                    let pt = point(c.x + px(r * a.cos()), c.y + px(r * a.sin()));
                    if i == 0 {
                        track.move_to(pt);
                    } else {
                        track.line_to(pt);
                    }
                }
                if let Ok(path) = track.build() {
                    win.paint_path(path, wash);
                }
                let arc_sweep = TAU * 0.30;
                let mut arc = PathBuilder::stroke(px(1.65));
                for i in 0..=12 {
                    let a = spin_angle + (i as f32 / 12.0) * arc_sweep;
                    let pt = point(c.x + px(r * a.cos()), c.y + px(r * a.sin()));
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
        .size(px(16.))
        .flex_none()
        .into_any_element(),
        TrailStepState::Pending if gate => {
            // Dashed amber/orange ring matching `.ui-atrc__mark[data-kind="gate"]`
            canvas(
                |_, _, _| (),
                move |bounds, _, win, _| {
                    let c = bounds.center();
                    let r = 6.25;
                    let dashes = 8usize;
                    for d in 0..dashes {
                        let a0 = (d as f32 / dashes as f32) * TAU - std::f32::consts::FRAC_PI_2;
                        let a1 = a0 + (TAU / dashes as f32) * 0.52;
                        let mut seg = PathBuilder::stroke(px(1.55));
                        for k in 0..=4 {
                            let a = a0 + (a1 - a0) * (k as f32 / 4.0);
                            let pt = point(c.x + px(r * a.cos()), c.y + px(r * a.sin()));
                            if k == 0 {
                                seg.move_to(pt);
                            } else {
                                seg.line_to(pt);
                            }
                        }
                        if let Ok(path) = seg.build() {
                            win.paint_path(path, orange);
                        }
                    }
                },
            )
            .size(px(16.))
            .flex_none()
            .into_any_element()
        }
        TrailStepState::Pending => div()
            .size(px(16.))
            .rounded_full()
            .bg(card_bg)
            .border(px(1.5))
            .border_color(line_col)
            .into_any_element(),
        TrailStepState::Waiting => div()
            .size(px(16.))
            .rounded_full()
            .bg(amber_wash)
            .flex()
            .items_center()
            .justify_center()
            .child(pro_svg("0 0 16 16", PAUSE_MARK_SVG, 16., amber_ink))
            .into_any_element(),
        TrailStepState::Failed => div()
            .size(px(16.))
            .rounded_full()
            .bg(red_wash)
            .flex()
            .items_center()
            .justify_center()
            .child(pro_svg("0 0 16 16", FAIL_MARK_SVG, 16., red))
            .into_any_element(),
        TrailStepState::Declined => div()
            .size(px(16.))
            .rounded_full()
            .bg(line_col.opacity(0.35))
            .flex()
            .items_center()
            .justify_center()
            .child(pro_svg("0 0 16 16", DASH_MARK_SVG, 16., muted))
            .into_any_element(),
    }
}

fn sample_substep_branch_wire(
    x_main: f32,
    x_sub: f32,
    total_sub_h: f32,
    has_next_step: bool,
) -> Vec<(f32, f32)> {
    // Starts at bottom of parent step's 16px marker (`y = -8` relative to substep container top)
    // and bends smoothly to `x_sub` over 14px, runs down `x_sub`, and optionally bends back to
    // `x_main` at `y = total_sub_h + 8` (top of next step's 16px marker).
    let y0 = -8.0;
    let y_bend_in = 6.0;
    let mut pts = Vec::with_capacity(48);
    for i in 0..=14 {
        let u = i as f32 / 14.0;
        let inv = 1.0 - u;
        let cx1 = x_main;
        let cy1 = y0 + 8.0;
        let cx2 = x_sub;
        let cy2 = y_bend_in - 8.0;
        let px_x = inv * inv * inv * x_main
            + 3.0 * inv * inv * u * cx1
            + 3.0 * inv * u * u * cx2
            + u * u * u * x_sub;
        let py_y = inv * inv * inv * y0
            + 3.0 * inv * inv * u * cy1
            + 3.0 * inv * u * u * cy2
            + u * u * u * y_bend_in;
        pts.push((px_x, py_y));
    }

    let spine_end_y = if has_next_step {
        (total_sub_h - 6.0).max(y_bend_in)
    } else {
        (total_sub_h - SUB_ROW_H * 0.5).max(y_bend_in)
    };

    for i in 1..=16 {
        let u = i as f32 / 16.0;
        pts.push((x_sub, y_bend_in + (spine_end_y - y_bend_in) * u));
    }

    if has_next_step {
        let y_out_end = total_sub_h + 8.0;
        for i in 1..=14 {
            let u = i as f32 / 14.0;
            let inv = 1.0 - u;
            let cx1 = x_sub;
            let cy1 = spine_end_y + 8.0;
            let cx2 = x_main;
            let cy2 = y_out_end - 8.0;
            let px_x = inv * inv * inv * x_sub
                + 3.0 * inv * inv * u * cx1
                + 3.0 * inv * u * u * cx2
                + u * u * u * x_main;
            let py_y = inv * inv * inv * spine_end_y
                + 3.0 * inv * inv * u * cy1
                + 3.0 * inv * u * u * cy2
                + u * u * u * y_out_end;
            pts.push((px_x, py_y));
        }
    }
    pts
}

fn sample_spark_path_to_substep(x_main: f32, x_sub: f32, target_sub_y: f32) -> Vec<(f32, f32)> {
    let y0 = -8.0;
    let y_bend_in = 6.0;
    let mut pts = Vec::with_capacity(44);
    for i in 0..=18 {
        let u = i as f32 / 18.0;
        let inv = 1.0 - u;
        let cx1 = x_main;
        let cy1 = y0 + 8.0;
        let cx2 = x_sub;
        let cy2 = y_bend_in - 8.0;
        let px_x = inv * inv * inv * x_main
            + 3.0 * inv * inv * u * cx1
            + 3.0 * inv * u * u * cx2
            + u * u * u * x_sub;
        let py_y = inv * inv * inv * y0
            + 3.0 * inv * inv * u * cy1
            + 3.0 * inv * u * u * cy2
            + u * u * u * y_bend_in;
        pts.push((px_x, py_y));
    }
    let end_y = target_sub_y.max(y_bend_in + 1.0);
    for i in 1..=22 {
        let u = i as f32 / 22.0;
        pts.push((x_sub, y_bend_in + (end_y - y_bend_in) * u));
    }
    pts
}

impl Render for AgentTrail {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let reduced = disabled_motion(cx);

        let has_running = self
            .steps
            .iter()
            .any(|s| s.state == TrailStepState::Running);
        let elapsed_secs = self.started_at.elapsed().as_secs_f32();
        if (has_running || self.loading) && !reduced {
            window.request_animation_frame();
        }

        let card_bg: Hsla = if p.dark {
            rgb(0x1c1c1e).into()
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
            rgb(0x0190fe).into()
        };
        let wash: Hsla = if p.dark {
            accent.opacity(0.18)
        } else {
            accent.opacity(0.12)
        };
        let rail_line: Hsla = if p.dark {
            hsla(0., 0., 1., 0.12)
        } else {
            hsla(0., 0., 0., 0.10)
        };
        let logo_edge: Hsla = if p.dark {
            hsla(0., 0., 1., 0.12)
        } else {
            hsla(0., 0., 0., 0.08)
        };

        let (done_count, total_count) = if self.empty {
            (0, 0)
        } else {
            self.progress_counts()
        };

        let header_el: Option<AnyElement> = if self.show_header {
            let task_label = self
                .title
                .as_deref()
                .map(|t| {
                    if t == "Fix the checkout crash" {
                        tr(cx, "Fix the checkout crash", "修复结算页崩溃")
                    } else {
                        t
                    }
                })
                .unwrap_or_else(|| tr(cx, "Agent run", "智能体运行"))
                .to_string();

            let agent_chip: Option<AnyElement> = self.agent.as_ref().map(|ag| {
                let mark = ag.logo.as_deref().and_then(|k| pro_app_artwork(k, 12., cx));
                div()
                    .h(px(22.))
                    .pl(px(7.))
                    .pr(px(8.))
                    .flex_none()
                    .rounded(px(5.))
                    .bg(p.hover)
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .when_some(mark, |d, m| d.child(m))
                    .child(
                        div()
                            .text_size(px(12.))
                            .line_height(px(16.))
                            .text_color(p.text)
                            .child(ag.name.clone()),
                    )
                    .into_any_element()
            });

            Some(
                div()
                    .h(px(36.))
                    .px(px(8.))
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_size(px(13.))
                            .line_height(px(20.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .child(task_label),
                    )
                    .when_some(agent_chip, |d, chip| d.child(chip))
                    .child(
                        div()
                            .flex_none()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .child(div().w(px(36.)).child(pro_spring_bar(
                                pro_eid(&self.id, "trail-hdr-bar"),
                                if total_count > 0 {
                                    done_count as f32 / total_count as f32
                                } else {
                                    0.0
                                },
                                4.,
                                2.,
                                p.chip,
                                accent,
                            )))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .text_size(px(12.))
                                    .line_height(px(16.))
                                    .text_color(p.muted)
                                    .child(pro_rolling_number(
                                        pro_eid(&self.id, "trail-done-n"),
                                        done_count as f64,
                                        0,
                                        "",
                                        "",
                                        12.,
                                        FontWeight::MEDIUM,
                                        p.ink,
                                    ))
                                    .child(format!("/{total_count}")),
                            ),
                    )
                    .into_any_element(),
            )
        } else {
            None
        };

        if self.loading {
            let phase = if reduced {
                0.5
            } else {
                (elapsed_secs * 0.9).fract()
            };
            let mut skel = div().pt(px(6.)).pb(px(4.)).flex().flex_col().gap(px(4.));
            for i in 0..4usize {
                skel = skel.child(
                    div()
                        .h(px(STEP_ROW_H))
                        .px(px(8.))
                        .flex()
                        .items_center()
                        .gap(px(10.))
                        .child(pro_skeleton_bar(16., 16., 8., phase + i as f32 * 0.1, cx))
                        .child(pro_skeleton_bar(20., 20., 10., phase + i as f32 * 0.1, cx))
                        .child(pro_skeleton_bar(
                            130. + (i as f32 * 18.),
                            12.,
                            4.,
                            phase + i as f32 * 0.1,
                            cx,
                        ))
                        .child(div().ml_auto().child(pro_skeleton_bar(
                            32.,
                            11.,
                            4.,
                            phase + i as f32 * 0.1,
                            cx,
                        ))),
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
                        .max_w(px(392.))
                        .pt(px(4.))
                        .px(px(8.))
                        .pb(px(10.))
                        .rounded(px(12.))
                        .bg(card_bg)
                        .border_1()
                        .border_color(card_border)
                        .when(!p.dark, |d| d.shadow_sm())
                        .flex()
                        .flex_col()
                        .when_some(header_el, |d, h| d.child(h))
                        .child(skel),
                )
                .into_any_element();
        }

        if self.empty || self.steps.is_empty() {
            let empty_view = pro_empty_state(
                ProIcon::Activity,
                tr(cx, "No steps yet", "暂无执行步骤"),
                tr(
                    cx,
                    "Steps and live sub-tasks appear here once the agent starts working.",
                    "智能体开始运行后，执行步骤与实时子任务将在此处展开。",
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
                        .max_w(px(392.))
                        .pt(px(4.))
                        .px(px(8.))
                        .pb(px(10.))
                        .rounded(px(12.))
                        .bg(card_bg)
                        .border_1()
                        .border_color(card_border)
                        .when(!p.dark, |d| d.shadow_sm())
                        .flex()
                        .flex_col()
                        .when_some(header_el, |d, h| d.child(h))
                        .child(empty_view),
                )
                .into_any_element();
        }

        let spin_angle = if reduced {
            -std::f32::consts::FRAC_PI_4
        } else {
            (elapsed_secs / 0.90).fract() * TAU - std::f32::consts::FRAC_PI_2
        };
        let loop_phase = if reduced {
            0.0
        } else {
            (elapsed_secs / LOOP_SECS).fract()
        };

        let num_steps = self.steps.len();
        let mut list_col = div()
            .id(pro_eid(&self.id, "trail-list"))
            .role(Role::List)
            .aria_label(tr(cx, "Agent step trail", "智能体步骤轨迹"))
            .pt(px(4.))
            .flex()
            .flex_col();

        for idx in 0..num_steps {
            let step = &self.steps[idx];
            let step_id = step.id.clone();
            let has_prev = idx > 0;
            let has_next = idx + 1 < num_steps;
            let prev_had_open_branch = idx > 0
                && !self.steps[idx - 1].substeps.is_empty()
                && self.expanded_ids.contains(&self.steps[idx - 1].id);
            let prev_done = idx > 0 && self.steps[idx - 1].state == TrailStepState::Done;
            let cur_done = step.state == TrailStepState::Done;

            let has_substeps = !step.substeps.is_empty();
            let is_open = has_substeps && self.expanded_ids.contains(&step_id);
            let can_open_ask = (step.gate
                && matches!(
                    step.state,
                    TrailStepState::Pending | TrailStepState::Waiting
                ))
                || step.state == TrailStepState::Waiting;
            let ask_open = self.open_ask_id.as_ref() == Some(&step_id);
            let is_interactive = has_substeps || can_open_ask;
            let is_hovered = self.hovered_step.as_ref() == Some(&step_id);

            let top_seg_color = if prev_done { accent } else { rail_line };
            let bot_seg_color = if cur_done { accent } else { rail_line };
            let done_drawn = if cur_done {
                if reduced {
                    true
                } else if let Some(clk) = self.done_clocks.get(&step_id) {
                    clk.time(0.28, window, cx) > 0.012
                } else {
                    true
                }
            } else {
                false
            };

            let marker_el = paint_step_marker(
                pro_eid(&self.id, format!("step-chk-{step_id}")),
                done_drawn,
                step.state,
                step.gate,
                spin_angle,
                accent,
                wash,
                p.orange,
                p.amber_ink,
                p.amber_soft,
                p.red,
                p.red_soft,
                rail_line,
                card_bg,
                p.muted,
            );

            let tool_medallion: Option<AnyElement> = step.tool.as_ref().map(|tool_key| {
                let mark = pro_app_artwork(tool_key.as_ref(), 11., cx);
                div()
                    .size(px(20.))
                    .mr(px(6.))
                    .flex_none()
                    .rounded_full()
                    .bg(card_bg)
                    .border_1()
                    .border_color(logo_edge)
                    .when(!p.dark, |d| d.shadow_xs())
                    .opacity(if step.state == TrailStepState::Pending {
                        0.55
                    } else {
                        1.0
                    })
                    .flex()
                    .items_center()
                    .justify_center()
                    .when_some(mark, |d, m| d.child(m))
                    .into_any_element()
            });

            let title_color = match step.state {
                TrailStepState::Running | TrailStepState::Failed | TrailStepState::Waiting => p.ink,
                TrailStepState::Done => p.text,
                TrailStepState::Pending | TrailStepState::Declined => p.muted,
            };
            let title_text = step.display_title(cx).to_string();

            let right_meta: AnyElement = match step.state {
                TrailStepState::Done => {
                    let dur = step.duration.clone().unwrap_or_default();
                    div()
                        .text_size(px(12.))
                        .line_height(px(16.))
                        .text_color(p.faint)
                        .child(dur)
                        .into_any_element()
                }
                TrailStepState::Running => {
                    let live_secs = step.elapsed_offset_secs + elapsed_secs.floor() as u64;
                    let status_word = step.display_status(cx).to_string();
                    let timer_el = if live_secs < 60 {
                        div()
                            .min_w(px(26.))
                            .flex()
                            .justify_end()
                            .child(pro_rolling_number(
                                pro_eid(&self.id, format!("run-sec-{step_id}")),
                                live_secs as f64,
                                0,
                                "",
                                "s",
                                12.,
                                FontWeight::NORMAL,
                                p.faint,
                            ))
                            .into_any_element()
                    } else {
                        div()
                            .min_w(px(26.))
                            .text_right()
                            .text_size(px(12.))
                            .line_height(px(16.))
                            .text_color(p.faint)
                            .child(format_trail_secs(live_secs))
                            .into_any_element()
                    };
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .child(pro_shimmer_text(
                            pro_eid(&self.id, format!("st-shim-{step_id}")),
                            status_word,
                            accent,
                            pro_chrome_fade(accent, card_bg),
                            12.,
                            FontWeight::MEDIUM,
                            true,
                        ))
                        .child(timer_el)
                        .into_any_element()
                }
                TrailStepState::Waiting => {
                    let ask_toggle_id = step_id.clone();
                    let needs_inner = div()
                        .h(px(20.))
                        .px(px(7.))
                        .flex()
                        .items_center()
                        .gap(px(4.))
                        .text_size(px(12.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.amber_ink)
                        .child(tr(cx, "Needs you", "待您确认"))
                        .child(pro_icon(
                            if ask_open {
                                ProIcon::ChevronUp
                            } else {
                                ProIcon::ChevronDown
                            },
                            11.,
                            p.amber_ink,
                        ));
                    pro_tactile(
                        pro_eid(&self.id, format!("needs-btn-{step_id}")),
                        SharedString::from(tr(cx, "Needs your approval", "等待您的审批")),
                        6.,
                        p.amber_soft,
                        p.amber.opacity(0.24),
                        transparent_black(),
                        transparent_black(),
                        ask_open,
                        needs_inner,
                        cx,
                        move |this, _, cx| {
                            if this.open_ask_id.as_ref() == Some(&ask_toggle_id) {
                                this.open_ask_id = None;
                            } else {
                                this.open_ask_id = Some(ask_toggle_id.clone());
                            }
                            cx.notify();
                        },
                    )
                    .flex_none()
                    .into_any_element()
                }
                TrailStepState::Pending if step.gate => div()
                    .h(px(20.))
                    .px(px(7.))
                    .rounded(px(6.))
                    .bg(p.amber_soft)
                    .text_color(p.amber_ink)
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .flex()
                    .items_center()
                    .child(tr(cx, "Approval", "需审批"))
                    .into_any_element(),
                TrailStepState::Failed => div()
                    .text_size(px(12.))
                    .line_height(px(16.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.red)
                    .child(tr(cx, "Failed", "失败"))
                    .into_any_element(),
                TrailStepState::Declined => div()
                    .text_size(px(12.))
                    .line_height(px(16.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.muted)
                    .child(tr(cx, "Declined", "已拒绝"))
                    .into_any_element(),
                TrailStepState::Pending => div().into_any_element(),
            };

            let chev_el: Option<AnyElement> = if has_substeps {
                Some(
                    div()
                        .ml(px(4.))
                        .flex_none()
                        .opacity(if is_hovered || is_open { 0.85 } else { 0.0 })
                        .child(pro_icon(
                            if is_open {
                                ProIcon::ChevronDown
                            } else {
                                ProIcon::ChevronRight
                            },
                            12.,
                            p.faint,
                        ))
                        .into_any_element(),
                )
            } else {
                None
            };

            let hover_id = step_id.clone();
            let press_id = step_id.clone();
            let click_id = step_id.clone();
            let key_id = step_id.clone();
            let declined = step.state == TrailStepState::Declined;
            let is_pressed = self.pressed_step.as_ref() == Some(&step_id);

            let mut row = div()
                .id(pro_eid(&self.id, format!("step-row-{step_id}")))
                .role(if is_interactive {
                    Role::Button
                } else {
                    Role::ListItem
                })
                .aria_label(title_text.clone())
                .when(is_interactive, |d| d.tab_index(0).cursor_pointer())
                .w_full()
                .h(px(STEP_ROW_H))
                .px(px(8.))
                .rounded(px(6.))
                .relative()
                .top(px(if is_pressed { 0.5 } else { 0.0 }))
                .flex()
                .items_center()
                .bg(if is_interactive && is_pressed {
                    p.chip
                } else if is_interactive && is_hovered {
                    p.hover
                } else {
                    transparent_black()
                })
                .focus_visible(move |s| s.border_1().border_color(p.focus))
                .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                    let next = if *inside {
                        Some(hover_id.clone())
                    } else {
                        None
                    };
                    if !*inside && this.pressed_step.as_ref() == Some(&hover_id) {
                        this.pressed_step = None;
                    }
                    if this.hovered_step != next {
                        this.hovered_step = next;
                        cx.notify();
                    }
                }));

            if is_interactive {
                row = row
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, _, cx| {
                            this.pressed_step = Some(press_id.clone());
                            cx.notify();
                        }),
                    )
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| {
                            if this.pressed_step.take().is_some() {
                                cx.notify();
                            }
                        }),
                    )
                    .on_mouse_up_out(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| {
                            if this.pressed_step.take().is_some() {
                                cx.notify();
                            }
                        }),
                    )
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        if has_substeps {
                            this.toggle_step_expanded(click_id.as_ref(), cx);
                        } else if can_open_ask {
                            if this.open_ask_id.as_ref() == Some(&click_id) {
                                this.open_ask_id = None;
                            } else {
                                this.open_ask_id = Some(click_id.clone());
                            }
                            cx.notify();
                        }
                    }))
                    .on_key_down(cx.listener(move |this, ev: &KeyDownEvent, _, cx| {
                        match ev.keystroke.key.as_str() {
                            "enter" | "space" | " " => {
                                if has_substeps {
                                    this.toggle_step_expanded(key_id.as_ref(), cx);
                                } else if can_open_ask {
                                    if this.open_ask_id.as_ref() == Some(&key_id) {
                                        this.open_ask_id = None;
                                    } else {
                                        this.open_ask_id = Some(key_id.clone());
                                    }
                                    cx.notify();
                                }
                                cx.stop_propagation();
                            }
                            "escape" if this.open_ask_id.as_ref() == Some(&key_id) => {
                                this.open_ask_id = None;
                                cx.stop_propagation();
                                cx.notify();
                            }
                            _ => {}
                        }
                    }));
            }

            // Upper 8px connector (`top: 0..8`) unless first step or previous step ended with an open branch wire.
            if has_prev && !prev_had_open_branch {
                row = row.child(
                    div()
                        .absolute()
                        .top_0()
                        .left(px(MAIN_RAIL_X - 0.75))
                        .w(px(1.5))
                        .h(px((STEP_ROW_H - 16.0) * 0.5))
                        .bg(top_seg_color),
                );
            }
            // Lower 8px connector (`bottom: 0..8`) unless last step or this step opens a branch wire.
            if has_next && !is_open {
                row = row.child(
                    div()
                        .absolute()
                        .bottom_0()
                        .left(px(MAIN_RAIL_X - 0.75))
                        .w(px(1.5))
                        .h(px((STEP_ROW_H - 16.0) * 0.5))
                        .bg(bot_seg_color),
                );
            }

            row = row
                .child(
                    div()
                        .w(px(18.))
                        .h(px(16.))
                        .mr(px(14.))
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(marker_el),
                )
                .when_some(tool_medallion, |d, m| d.child(m))
                .child(
                    div()
                        .min_w_0()
                        .flex_shrink(1.0)
                        .truncate()
                        .text_size(px(13.))
                        .line_height(px(20.))
                        .text_color(title_color)
                        .when(declined, |d| d.line_through())
                        .child(title_text),
                )
                .when_some(chev_el, |d, c| d.child(c))
                .child(
                    div()
                        .ml_auto()
                        .pl(px(12.))
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_end()
                        .child(right_meta),
                );

            let mut step_block = div().w_full().relative().flex().flex_col().child(row);

            // Sub-steps disclosure with S-curved rail wire + traveling fading light pulse!
            if has_substeps {
                let unfolded = self.unfolded_sub_ids.contains(&step_id);
                let visible_items = compute_visible_substeps(&step.substeps, unfolded);
                let sub_count = visible_items.len();
                let est_h = (sub_count as f32) * SUB_ROW_H + 4.0;
                let parent_step_done = step.state == TrailStepState::Done;
                let parent_step_running = step.state == TrailStepState::Running;
                let substeps_clone = step.substeps.clone();
                let step_id_fold = step_id.clone();
                let entity = cx.entity().clone();
                let is_zh = tr(cx, "en", "zh") == "zh";

                step_block = step_block.child(pro_fold(
                    pro_eid(&self.id, format!("subs-fold-{step_id}")),
                    is_open,
                    est_h,
                    0.0,
                    move || {
                        let total_h = (sub_count as f32) * SUB_ROW_H + 4.0;
                        let wire_color = if parent_step_done { accent } else { rail_line };

                        // Find which visible row is the `Running` sub-step for the traveling light pulse
                        let running_vis_idx = visible_items.iter().position(|item| match item {
                            VisibleSubstepItem::Substep(si) => {
                                substeps_clone[*si].state == TrailSubstepState::Running
                            }
                            _ => false,
                        });

                        let wire_canvas = canvas(
                            |_, _, _| (),
                            move |bounds, _, win, _| {
                                let ox = f32::from(bounds.left());
                                let oy = f32::from(bounds.top());
                                let wire_pts = sample_substep_branch_wire(
                                    MAIN_RAIL_X,
                                    SUB_RAIL_X,
                                    total_h,
                                    has_next,
                                );
                                if wire_pts.len() >= 2 {
                                    let mut base = PathBuilder::stroke(px(1.5));
                                    for (i, &(x, y)) in wire_pts.iter().enumerate() {
                                        let pt = point(px(ox + x), px(oy + y));
                                        if i == 0 {
                                            base.move_to(pt);
                                        } else {
                                            base.line_to(pt);
                                        }
                                    }
                                    if let Ok(path) = base.build() {
                                        win.paint_path(path, wire_color);
                                    }
                                }

                                // Fading light pulse traveling down into the active sub-step!
                                if parent_step_running
                                    && !reduced
                                    && let Some(r_vis) = running_vis_idx
                                {
                                    let target_y =
                                        2.0 + (r_vis as f32) * SUB_ROW_H + SUB_ROW_H * 0.5;
                                    let spark_pts = sample_spark_path_to_substep(
                                        MAIN_RAIL_X,
                                        SUB_RAIL_X,
                                        target_y,
                                    );
                                    let seg_n = spark_pts.len().saturating_sub(1);
                                    if seg_n >= 2 {
                                        // 0..0.62: pulse travels from parent step down to target sub-step with trailing fade
                                        let travel_u = (loop_phase / 0.62).clamp(0., 1.35);
                                        let head_pos = pro_ease(travel_u.min(1.0)) * (seg_n as f32)
                                            + (travel_u - 1.0).max(0.0) * (seg_n as f32 * 0.85);
                                        let tail_len = (seg_n as f32 * 0.42).max(6.0);
                                        for i in 0..seg_n {
                                            let pos = i as f32 + 0.5;
                                            let behind = head_pos - pos;
                                            if (-0.8..=tail_len).contains(&behind) {
                                                let alpha = if behind < 0.0 {
                                                    (1.0 + behind / 0.8).clamp(0., 1.)
                                                } else {
                                                    let u = (1.0 - behind / tail_len).clamp(0., 1.);
                                                    u * u
                                                };
                                                if alpha > 0.02 {
                                                    let mut seg =
                                                        PathBuilder::stroke(px(1.6 + 0.4 * alpha));
                                                    seg.move_to(point(
                                                        px(ox + spark_pts[i].0),
                                                        px(oy + spark_pts[i].1),
                                                    ));
                                                    seg.line_to(point(
                                                        px(ox + spark_pts[i + 1].0),
                                                        px(oy + spark_pts[i + 1].1),
                                                    ));
                                                    if let Ok(path) = seg.build() {
                                                        win.paint_path(path, accent.opacity(alpha));
                                                    }
                                                }
                                            }
                                        }

                                        // Arrival ring bloom at running sub-step dot (`0.50..0.88` of cycle)
                                        if (0.50..=0.88).contains(&loop_phase) {
                                            let rt = (loop_phase - 0.50) / 0.38;
                                            let ring_r = 2.5 + 5.0 * pro_ease(rt);
                                            let ring_alpha = (1.0 - rt) * 0.55;
                                            let mut ring = PathBuilder::stroke(px(1.35));
                                            for k in 0..=20 {
                                                let a = (k as f32 / 20.0) * TAU;
                                                let pt = point(
                                                    px(ox + SUB_RAIL_X + ring_r * a.cos()),
                                                    px(oy + target_y + ring_r * a.sin()),
                                                );
                                                if k == 0 {
                                                    ring.move_to(pt);
                                                } else {
                                                    ring.line_to(pt);
                                                }
                                            }
                                            if let Ok(path) = ring.build() {
                                                win.paint_path(path, accent.opacity(ring_alpha));
                                            }
                                        }
                                    }
                                }
                            },
                        )
                        .absolute()
                        .top_0()
                        .left_0()
                        .w(px(40.))
                        .h(px(total_h));

                        let mut sub_col = div()
                            .w_full()
                            .relative()
                            .py(px(2.))
                            .flex()
                            .flex_col()
                            .child(wire_canvas);

                        for (v_idx, item) in visible_items.iter().enumerate() {
                            match *item {
                                VisibleSubstepItem::Substep(si) => {
                                    let sub = &substeps_clone[si];
                                    let dot_bg = match sub.state {
                                        TrailSubstepState::Running => accent,
                                        TrailSubstepState::Done if parent_step_done => accent,
                                        TrailSubstepState::Done => {
                                            if p.dark {
                                                rgb(0x636369).into()
                                            } else {
                                                rgb(0xb5b5ba).into()
                                            }
                                        }
                                        TrailSubstepState::Pending => card_bg,
                                    };
                                    let sub_fg = match sub.state {
                                        TrailSubstepState::Running => p.text,
                                        TrailSubstepState::Done => p.muted,
                                        TrailSubstepState::Pending => p.faint,
                                    };
                                    let sub_title = if let Some(zh) = &sub.title_zh {
                                        if is_zh {
                                            zh.to_string()
                                        } else {
                                            sub.title.to_string()
                                        }
                                    } else {
                                        sub.title.to_string()
                                    };

                                    let sub_lead: AnyElement = if sub.state
                                        == TrailSubstepState::Done
                                    {
                                        div()
                                            .ml(px(-3.5))
                                            .flex_none()
                                            .child(pro_check_draw(
                                                SharedString::from(format!(
                                                    "sub-chk-{}-{si}",
                                                    step_id_fold
                                                )),
                                                true,
                                                12.,
                                                false,
                                                if parent_step_done { accent } else { p.muted },
                                            ))
                                            .into_any_element()
                                    } else {
                                        div()
                                            .size(px(5.))
                                            .flex_none()
                                            .rounded_full()
                                            .bg(dot_bg)
                                            .when(sub.state == TrailSubstepState::Pending, |d| {
                                                d.border_1().border_color(p.faint)
                                            })
                                            .into_any_element()
                                    };

                                    sub_col = sub_col.child(
                                        div()
                                            .w_full()
                                            .h(px(SUB_ROW_H))
                                            .pl(px(SUB_RAIL_X - 2.5))
                                            .pr(px(8.))
                                            .flex()
                                            .items_center()
                                            .gap(px(if sub.state == TrailSubstepState::Done {
                                                10.5
                                            } else {
                                                14.
                                            }))
                                            .child(sub_lead)
                                            .child(
                                                div()
                                                    .min_w_0()
                                                    .flex_shrink(1.0)
                                                    .truncate()
                                                    .text_size(px(12.))
                                                    .line_height(px(16.))
                                                    .text_color(sub_fg)
                                                    .child(sub_title),
                                            )
                                            .when_some(sub.detail.clone(), |d, det| {
                                                d.child(
                                                    div()
                                                        .ml_auto()
                                                        .pl(px(8.))
                                                        .flex_none()
                                                        .text_size(px(11.))
                                                        .line_height(px(16.))
                                                        .text_color(p.faint)
                                                        .child(det),
                                                )
                                            }),
                                    );
                                }
                                VisibleSubstepItem::FoldEarlier(cnt)
                                | VisibleSubstepItem::FoldMore(cnt) => {
                                    let is_earlier =
                                        matches!(item, VisibleSubstepItem::FoldEarlier(_));
                                    let label = if is_zh {
                                        if is_earlier {
                                            format!("+前序 {cnt} 项")
                                        } else {
                                            format!("+后续 {cnt} 项")
                                        }
                                    } else if is_earlier {
                                        format!("+{cnt} earlier")
                                    } else {
                                        format!("+{cnt} more")
                                    };
                                    let ent = entity.clone();
                                    let sid = step_id_fold.clone();
                                    sub_col = sub_col.child(
                                        div()
                                            .id(SharedString::from(format!(
                                                "fold-more-{sid}-{v_idx}"
                                            )))
                                            .role(Role::Button)
                                            .w_full()
                                            .h(px(SUB_ROW_H))
                                            .pl(px(SUB_RAIL_X - 2.5))
                                            .pr(px(8.))
                                            .rounded(px(5.))
                                            .flex()
                                            .items_center()
                                            .gap(px(14.))
                                            .cursor_pointer()
                                            .hover(move |s| s.bg(p.hover))
                                            .on_click(move |_, _, cx| {
                                                ent.update(cx, |this, cx| {
                                                    this.unfolded_sub_ids.insert(sid.clone());
                                                    cx.notify();
                                                });
                                            })
                                            .child(
                                                div()
                                                    .size(px(5.))
                                                    .flex_none()
                                                    .rounded_full()
                                                    .bg(card_bg)
                                                    .border_1()
                                                    .border_color(p.faint),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(12.))
                                                    .line_height(px(16.))
                                                    .text_color(p.muted)
                                                    .child(label),
                                            ),
                                    );
                                }
                                VisibleSubstepItem::ShowFewer => {
                                    let label = if is_zh { "收起部分" } else { "Show fewer" };
                                    let ent = entity.clone();
                                    let sid = step_id_fold.clone();
                                    sub_col = sub_col.child(
                                        div()
                                            .id(SharedString::from(format!("fold-fewer-{sid}")))
                                            .role(Role::Button)
                                            .w_full()
                                            .h(px(SUB_ROW_H))
                                            .pl(px(SUB_RAIL_X - 2.5))
                                            .pr(px(8.))
                                            .rounded(px(5.))
                                            .flex()
                                            .items_center()
                                            .gap(px(14.))
                                            .cursor_pointer()
                                            .hover(move |s| s.bg(p.hover))
                                            .on_click(move |_, _, cx| {
                                                ent.update(cx, |this, cx| {
                                                    this.unfolded_sub_ids.remove(&sid);
                                                    cx.notify();
                                                });
                                            })
                                            .child(
                                                div()
                                                    .size(px(5.))
                                                    .flex_none()
                                                    .rounded_full()
                                                    .bg(card_bg)
                                                    .border_1()
                                                    .border_color(p.faint),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(12.))
                                                    .line_height(px(16.))
                                                    .text_color(p.muted)
                                                    .child(label),
                                            ),
                                    );
                                }
                            }
                        }

                        sub_col.into_any_element()
                    },
                ));
            }

            // Inline approval disclosure (`Approve` / `Decline`) for gated or waiting steps!
            if can_open_ask {
                let ask_prompt = step
                    .display_ask(cx)
                    .unwrap_or_else(|| {
                        tr(
                            cx,
                            "Approve this gated action before the agent continues?",
                            "确认允许智能体执行此受控步骤？",
                        )
                    })
                    .to_string();
                let approve_label = tr(cx, "Approve", "批准").to_string();
                let decline_label = tr(cx, "Decline", "拒绝").to_string();
                let sid_a = step_id.clone();
                let sid_d = step_id.clone();
                let btn_decline = pro_btn(
                    pro_eid(&self.id, format!("ask-dec-{step_id}")),
                    decline_label,
                    None,
                    ProBtnStyle::Secondary,
                    true,
                    cx,
                    move |this, _, cx| {
                        this.decline_step(sid_d.as_ref(), cx);
                    },
                );
                let btn_approve = pro_btn(
                    pro_eid(&self.id, format!("ask-app-{step_id}")),
                    approve_label,
                    Some(ProIcon::Check),
                    ProBtnStyle::Primary,
                    true,
                    cx,
                    move |this, _, cx| {
                        this.approve_step(sid_a.as_ref(), cx);
                    },
                );

                step_block = step_block.child(pro_fold(
                    pro_eid(&self.id, format!("ask-fold-{step_id}")),
                    ask_open,
                    74.0,
                    0.0,
                    move || {
                        div()
                            .w_full()
                            .relative()
                            .pl(px(40.))
                            .pr(px(8.))
                            .pt(px(2.))
                            .pb(px(8.))
                            .when(has_next, |d| {
                                d.child(
                                    div()
                                        .absolute()
                                        .top_0()
                                        .bottom_0()
                                        .left(px(MAIN_RAIL_X - 0.75))
                                        .w(px(1.5))
                                        .bg(rail_line),
                                )
                            })
                            .flex()
                            .flex_col()
                            .gap(px(8.))
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .line_height(px(17.))
                                    .text_color(p.text)
                                    .child(ask_prompt),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_end()
                                    .gap(px(6.))
                                    .child(btn_decline)
                                    .child(btn_approve),
                            )
                            .into_any_element()
                    },
                ));
            }

            list_col = list_col.child(step_block);
        }

        div()
            .id(self.id.clone())
            .w_full()
            .flex()
            .justify_center()
            .child(
                div()
                    .w_full()
                    .max_w(px(392.))
                    .pt(px(4.))
                    .px(px(8.))
                    .pb(px(10.))
                    .rounded(px(12.))
                    .bg(card_bg)
                    .border_1()
                    .border_color(card_border)
                    .when(!p.dark, |d| d.shadow_sm())
                    .flex()
                    .flex_col()
                    .when_some(header_el, |d, h| d.child(h))
                    .child(list_col),
            )
            .into_any_element()
    }
}

fn pro_chrome_fade(accent: Hsla, card_bg: Hsla) -> Hsla {
    crate::components::pro_chrome::pro_mix_color(accent, card_bg, 0.65)
}

#[cfg(test)]
mod tests {
    use super::{
        AgentTrail, TrailStep, TrailStepState, TrailSubstep, TrailSubstepState, VisibleSubstepItem,
        compute_visible_substeps, format_trail_secs,
    };
    use gpui::{AppContext as _, TestAppContext};

    #[test]
    fn long_substep_lists_fold_around_running_substep_and_format_durations() {
        let subs = vec![
            TrailSubstep::new("s0"),
            TrailSubstep::new("s1"),
            TrailSubstep::new("s2"),
            TrailSubstep::new("s3"),
            TrailSubstep::new("s4").with_state(TrailSubstepState::Running),
            TrailSubstep::new("s5").with_state(TrailSubstepState::Pending),
            TrailSubstep::new("s6").with_state(TrailSubstepState::Pending),
            TrailSubstep::new("s7").with_state(TrailSubstepState::Pending),
        ];
        let folded = compute_visible_substeps(&subs, false);
        assert_eq!(folded.first(), Some(&VisibleSubstepItem::FoldEarlier(2)));
        assert_eq!(folded.last(), Some(&VisibleSubstepItem::FoldMore(1)));
        assert_eq!(folded.len(), 7); // +2 earlier, 5 substeps around s4, +1 more

        let unfolded = compute_visible_substeps(&subs, true);
        assert_eq!(unfolded.len(), 9);
        assert_eq!(unfolded.last(), Some(&VisibleSubstepItem::ShowFewer));

        assert_eq!(format_trail_secs(8), "8s");
        assert_eq!(format_trail_secs(64), "1m 4s");
    }

    #[gpui::test]
    fn default_trail_tracks_three_of_five_done_and_approves_or_declines_gates(
        cx: &mut TestAppContext,
    ) {
        cx.update(crate::init);
        let trail = cx.new(|cx| AgentTrail::new("test-trail", cx));

        trail.update(cx, |v, cx| {
            assert_eq!(v.progress_counts(), (3, 5));
            v.approve_step("pr", cx);
            assert_eq!(v.progress_counts(), (4, 5));
            assert_eq!(v.steps()[4].state, TrailStepState::Done);
        });

        let trail2 = cx.new(|cx| {
            AgentTrail::new("test-trail-dec", cx).with_steps(vec![
                TrailStep::new("s1", "Deploy preview", TrailStepState::Waiting).with_gate(true),
            ])
        });
        trail2.update(cx, |v, cx| {
            v.decline_step("s1", cx);
            assert_eq!(v.steps()[0].state, TrailStepState::Declined);
        });
    }
}
