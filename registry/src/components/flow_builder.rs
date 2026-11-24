//! Top-down in-place workflow automation builder with conditional forks, undo, and live test runs.
//!
//! Renders sequential trigger, agent, tool, wait, and `If / else` branching steps connected by
//! vertical rails with `+` insertion menus on every connector, step reordering/removal with undo,
//! and a live test runner that sends glowing pulses down active connectors into executing steps.

use crate::{
    components::pro_chrome::{
        ProBtnStyle, ProPalette, pro_agent_avatar, pro_app_tile, pro_btn, pro_check_draw,
        pro_connector_pulse, pro_eid, pro_empty_state, pro_icon_btn, pro_mix_color,
        pro_popover_presence, pro_presence, pro_rolling_number, pro_shimmer_text, pro_skeleton_bar,
        pro_spring_bar, pro_spring_f32, pro_swap_label, pro_tactile, pro_toast_banner, tr,
    },
    primitives::{
        family::disabled_motion,
        pro_icons::{ProIcon, pro_icon},
    },
};
use gpui::{prelude::*, *};
use std::collections::{HashMap, HashSet};
use web_time::Instant;

const STEP_W: f32 = 256.0;
const STEP_H: f32 = 92.0;
const LANE_PAD_X: f32 = 14.0;
const LANE_W: f32 = STEP_W + 2.0 * LANE_PAD_X;
const LINK_H: f32 = 46.0;
const FORK_TOP_H: f32 = 34.0;
const MENU_W: f32 = 244.0;

/// Functional category of a [`FlowStep`] in [`FlowBuilder`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FlowStepKind {
    /// Entry event that starts the workflow (solid primary icon tile).
    Trigger,
    /// External integration or tool call (`Gmail`, `Slack`, `HubSpot`, HTTP, etc.).
    #[default]
    Action,
    /// Autonomous LLM step rendered with a twinkling 3x3 agent pixel mark.
    Agent,
    /// Conditional `If / else` fork splitting into named parallel branch lanes.
    Branch,
    /// Scheduled delay or business-hours wait gate.
    Wait,
}

/// Live or controlled test-run execution state of a [`FlowStep`] or [`FlowBranchPath`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FlowRunStatus {
    /// Not yet executed in the current run.
    #[default]
    Idle,
    /// Actively executing with ambient blue glow, sweeping progress bar, and shimmering note.
    Running,
    /// Passed cleanly.
    Done,
    /// Terminated with an error.
    Failed,
    /// Bypassed because another `If / else` branch was taken.
    Skipped,
}

/// One named conditional branch lane under a `FlowStepKind::Branch` step.
#[derive(Clone, Debug, PartialEq)]
pub struct FlowBranchPath {
    /// Stable path identifier (can be marked `Skipped` in status maps).
    pub id: SharedString,
    /// Capsule label shown at the top of the lane (for example `"Enterprise"`).
    pub label: SharedString,
    /// Optional Chinese path label.
    pub label_zh: Option<SharedString>,
    /// Ordered steps executed along this conditional path.
    pub steps: Vec<FlowStep>,
}

impl FlowBranchPath {
    /// Create a named [`FlowBranchPath`] with `id` and `label`.
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            label_zh: None,
            steps: Vec::new(),
        }
    }

    /// Provide a localized Chinese branch label.
    pub fn with_label_zh(mut self, label_zh: impl Into<SharedString>) -> Self {
        self.label_zh = Some(label_zh.into());
        self
    }

    /// Populate the ordered steps inside this branch lane.
    pub fn with_steps(mut self, steps: Vec<FlowStep>) -> Self {
        self.steps = steps;
        self
    }

    fn display_label<'a>(&'a self, cx: &App) -> &'a str {
        match &self.label_zh {
            Some(zh) => tr(cx, self.label.as_ref(), zh.as_ref()),
            None => self.label.as_ref(),
        }
    }
}

/// One workflow step node inside [`FlowBuilder`].
#[derive(Clone, Debug, PartialEq)]
pub struct FlowStep {
    /// Stable step identifier.
    pub id: SharedString,
    /// Step archetype (`Trigger`, `Action`, `Agent`, `Branch`, or `Wait`).
    pub kind: FlowStepKind,
    /// Icon or provider key (`"form"`, `"mail"`, `"slack"`, `"hubspot"`, `"database"`, `"wait"`, `"branch"`).
    pub icon: Option<SharedString>,
    /// Primary step title.
    pub title: SharedString,
    /// Optional Chinese step title.
    pub title_zh: Option<SharedString>,
    /// Secondary subtitle (provider, trigger source, or branch predicate).
    pub detail: Option<SharedString>,
    /// Optional Chinese subtitle.
    pub detail_zh: Option<SharedString>,
    /// One-line description that shimmers while the step is `Running`.
    pub note: Option<SharedString>,
    /// Optional Chinese note description.
    pub note_zh: Option<SharedString>,
    /// Nominal step execution duration in seconds during test runs.
    pub duration_s: f32,
    /// Named conditional lanes when `kind == FlowStepKind::Branch`.
    pub paths: Vec<FlowBranchPath>,
}

impl FlowStep {
    /// Create a [`FlowStep`] with `id`, `kind`, and `title`.
    pub fn new(
        id: impl Into<SharedString>,
        kind: FlowStepKind,
        title: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            kind,
            icon: None,
            title: title.into(),
            title_zh: None,
            detail: None,
            detail_zh: None,
            note: None,
            note_zh: None,
            duration_s: 0.9,
            paths: Vec::new(),
        }
    }

    /// Provide a localized Chinese title.
    pub fn with_title_zh(mut self, title_zh: impl Into<SharedString>) -> Self {
        self.title_zh = Some(title_zh.into());
        self
    }

    /// Set the glyph or app provider key shown in the leading `24x24` tile.
    pub fn with_icon(mut self, icon: impl Into<SharedString>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Set the secondary subtitle below the title.
    pub fn with_detail(mut self, detail: impl Into<SharedString>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Provide a localized Chinese secondary subtitle.
    pub fn with_detail_zh(mut self, detail_zh: impl Into<SharedString>) -> Self {
        self.detail_zh = Some(detail_zh.into());
        self
    }

    /// Set the descriptive note line shown above the step progress bar.
    pub fn with_note(mut self, note: impl Into<SharedString>) -> Self {
        self.note = Some(note.into());
        self
    }

    /// Provide a localized Chinese descriptive note.
    pub fn with_note_zh(mut self, note_zh: impl Into<SharedString>) -> Self {
        self.note_zh = Some(note_zh.into());
        self
    }

    /// Set the nominal duration in seconds used when running a test flow.
    pub fn with_duration_s(mut self, duration_s: f32) -> Self {
        self.duration_s = duration_s.max(0.2);
        self
    }

    /// Attach conditional branch paths (`FlowStepKind::Branch`).
    pub fn with_paths(mut self, paths: Vec<FlowBranchPath>) -> Self {
        self.paths = paths;
        self
    }

    fn display_title<'a>(&'a self, cx: &App) -> &'a str {
        match &self.title_zh {
            Some(zh) => tr(cx, self.title.as_ref(), zh.as_ref()),
            None => self.title.as_ref(),
        }
    }

    fn display_detail<'a>(&'a self, cx: &App) -> Option<&'a str> {
        let en = self.detail.as_deref()?;
        Some(match &self.detail_zh {
            Some(zh) => tr(cx, en, zh.as_ref()),
            None => en,
        })
    }

    fn display_note<'a>(&'a self, cx: &App) -> Option<&'a str> {
        let en = self.note.as_deref()?;
        Some(match &self.note_zh {
            Some(zh) => tr(cx, en, zh.as_ref()),
            None => en,
        })
    }
}

/// Insertable step blueprint offered in the `+` connector menu.
#[derive(Clone, Debug, PartialEq)]
pub struct FlowTemplate {
    /// Step archetype to instantiate.
    pub kind: FlowStepKind,
    /// Icon or app key.
    pub icon: SharedString,
    /// English template title.
    pub title: SharedString,
    /// Chinese template title.
    pub title_zh: SharedString,
    /// English subtitle.
    pub detail: SharedString,
    /// Chinese subtitle.
    pub detail_zh: SharedString,
    /// English note line.
    pub note: SharedString,
    /// Chinese note line.
    pub note_zh: SharedString,
}

/// Location where a `+` connector button inserts a new step.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FlowInsertTarget {
    /// Insert at `index` in the top-level workflow sequence.
    Root(usize),
    /// Insert at `index` inside `path_idx` of `branch_id`.
    Branch {
        /// Identifier of the parent `Branch` step.
        branch_id: SharedString,
        /// Zero-based branch lane index.
        path_idx: usize,
        /// Insertion index inside that lane's `steps`.
        index: usize,
    },
}

/// Typed events emitted by [`FlowBuilder`].
#[derive(Clone, Debug, PartialEq)]
pub enum FlowBuilderEvent {
    /// Workflow structure changed (step added, moved, removed, or undone).
    StepsChanged(Vec<FlowStep>),
    /// Selected step id changed (`None` when cleared).
    Select(Option<SharedString>),
    /// User started a live test run.
    TestRunStarted,
    /// User stopped an in-flight test run.
    TestRunStopped,
    /// Live test run finished all active steps.
    TestRunCompleted,
    /// User reverted the last workflow edit via Undo.
    Undone,
}

impl EventEmitter<FlowBuilderEvent> for FlowBuilder {}

#[derive(Clone, Debug)]
struct RunEntry {
    step_id: SharedString,
    title: SharedString,
    title_zh: Option<SharedString>,
    incoming_key: SharedString,
    branch_choice: Option<(SharedString, usize)>,
    start_s: f32,
    end_s: f32,
}

#[derive(Clone, Debug)]
struct LiveRun {
    started_at: Instant,
    schedule: Vec<RunEntry>,
    skipped_paths: Vec<SharedString>,
    skipped_steps: Vec<SharedString>,
    total_s: f32,
}

/// Count every step recursively across root steps and nested branch paths.
pub fn count_flow_steps(steps: &[FlowStep]) -> usize {
    steps
        .iter()
        .map(|s| {
            1 + s
                .paths
                .iter()
                .map(|p| count_flow_steps(&p.steps))
                .sum::<usize>()
        })
        .sum()
}

fn flatten_step_ids(steps: &[FlowStep], out: &mut Vec<SharedString>) {
    for s in steps {
        out.push(s.id.clone());
        for p in &s.paths {
            flatten_step_ids(&p.steps, out);
        }
    }
}

fn collect_skipped_ids(steps: &[FlowStep], out: &mut Vec<SharedString>) {
    for s in steps {
        out.push(s.id.clone());
        for p in &s.paths {
            out.push(p.id.clone());
            collect_skipped_ids(&p.steps, out);
        }
    }
}

fn collect_all_path_ids(steps: &[FlowStep], out: &mut Vec<SharedString>) {
    for s in steps {
        for p in &s.paths {
            out.push(p.id.clone());
            collect_all_path_ids(&p.steps, out);
        }
    }
}

fn collect_flow_spotlight(
    steps: &[FlowStep],
    focus_id: &str,
) -> (HashSet<SharedString>, HashSet<SharedString>) {
    let mut lit_steps = HashSet::new();
    let mut lit_paths = HashSet::new();

    if let Some(pos) = steps.iter().position(|s| s.id.as_ref() == focus_id) {
        lit_steps.insert(steps[pos].id.clone());
        if pos > 0 {
            lit_steps.insert(steps[pos - 1].id.clone());
        }
        if pos + 1 < steps.len() {
            lit_steps.insert(steps[pos + 1].id.clone());
        }
        if steps[pos].kind == FlowStepKind::Branch {
            for p in &steps[pos].paths {
                lit_paths.insert(p.id.clone());
                if let Some(first) = p.steps.first() {
                    lit_steps.insert(first.id.clone());
                }
            }
        }
        return (lit_steps, lit_paths);
    }

    fn search_branch(
        steps: &[FlowStep],
        focus_id: &str,
        lit_steps: &mut HashSet<SharedString>,
        lit_paths: &mut HashSet<SharedString>,
    ) -> bool {
        for (idx, s) in steps.iter().enumerate() {
            if s.id.as_ref() == focus_id {
                for step in steps {
                    lit_steps.insert(step.id.clone());
                }
                if s.kind == FlowStepKind::Branch {
                    for p in &s.paths {
                        lit_paths.insert(p.id.clone());
                        if let Some(first) = p.steps.first() {
                            lit_steps.insert(first.id.clone());
                        }
                    }
                }
                let _ = idx;
                return true;
            }
            for p in &s.paths {
                if search_branch(&p.steps, focus_id, lit_steps, lit_paths) {
                    lit_paths.insert(p.id.clone());
                    for trunk in steps.iter().take(idx + 1) {
                        lit_steps.insert(trunk.id.clone());
                    }
                    return true;
                }
            }
        }
        false
    }

    search_branch(steps, focus_id, &mut lit_steps, &mut lit_paths);
    (lit_steps, lit_paths)
}

fn build_run_schedule(
    steps: &[FlowStep],
    t_cursor: &mut f32,
    prev_link: &str,
    schedule: &mut Vec<RunEntry>,
    skipped_paths: &mut Vec<SharedString>,
    skipped_steps: &mut Vec<SharedString>,
) {
    for (i, s) in steps.iter().enumerate() {
        let transit = if i == 0 && prev_link == "root" {
            0.0
        } else {
            0.34
        };
        let start_s = *t_cursor + transit;
        let dur = s.duration_s.max(0.35);
        let end_s = start_s + dur;
        let link_key = SharedString::from(format!("in:{}", s.id));
        let chosen = if s.kind == FlowStepKind::Branch && !s.paths.is_empty() {
            Some((s.id.clone(), 0usize))
        } else {
            None
        };
        schedule.push(RunEntry {
            step_id: s.id.clone(),
            title: s.title.clone(),
            title_zh: s.title_zh.clone(),
            incoming_key: link_key,
            branch_choice: chosen,
            start_s,
            end_s,
        });
        *t_cursor = end_s;

        if s.kind == FlowStepKind::Branch && !s.paths.is_empty() {
            for (p_idx, path) in s.paths.iter().enumerate() {
                if p_idx == 0 {
                    build_run_schedule(
                        &path.steps,
                        t_cursor,
                        path.id.as_ref(),
                        schedule,
                        skipped_paths,
                        skipped_steps,
                    );
                } else {
                    skipped_paths.push(path.id.clone());
                    collect_skipped_ids(&path.steps, skipped_steps);
                }
            }
        }
    }
}

/// Top-down interactive workflow builder with `+` insertion menus, conditional branches, undo, and live test run.
pub struct FlowBuilder {
    id: ElementId,
    name: SharedString,
    name_zh: SharedString,
    steps: Vec<FlowStep>,
    templates: Vec<FlowTemplate>,
    controlled_status: HashMap<SharedString, FlowRunStatus>,
    last_finished_status: HashMap<SharedString, (FlowRunStatus, f32)>,
    live_run: Option<LiveRun>,
    selected_id: Option<SharedString>,
    hovered_id: Option<SharedString>,
    open_menu: Option<FlowInsertTarget>,
    last_open_menu: Option<FlowInsertTarget>,
    undo_stack: Vec<Vec<FlowStep>>,
    toast: Option<(SharedString, Instant)>,
    next_seq: usize,
    canvas_height: f32,
    started_at: Instant,
    loading: bool,
    empty: bool,
}

impl FlowBuilder {
    /// Create a [`FlowBuilder`] preloaded with the 8-step `"Onboard new signups"` workflow.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        Self {
            id: id.into(),
            name: SharedString::from("Onboard new signups"),
            name_zh: SharedString::from("新注册用户自动引导"),
            steps: default_onboarding_steps(),
            templates: default_templates(),
            controlled_status: HashMap::new(),
            last_finished_status: HashMap::new(),
            live_run: None,
            selected_id: None,
            hovered_id: None,
            open_menu: None,
            last_open_menu: None,
            undo_stack: Vec::new(),
            toast: None,
            next_seq: 100,
            canvas_height: 452.0,
            started_at: Instant::now(),
            loading: false,
            empty: false,
        }
    }

    /// Set the workflow heading name.
    pub fn with_name(mut self, name: impl Into<SharedString>) -> Self {
        self.name = name.into();
        self
    }

    /// Provide a localized Chinese workflow heading name.
    pub fn with_name_zh(mut self, name_zh: impl Into<SharedString>) -> Self {
        self.name_zh = name_zh.into();
        self
    }

    /// Supply the workflow step tree (first step is typically a `Trigger`).
    pub fn with_steps(mut self, steps: Vec<FlowStep>) -> Self {
        self.steps = steps;
        self
    }

    /// Override the templates offered inside `+` connector menus.
    pub fn with_templates(mut self, templates: Vec<FlowTemplate>) -> Self {
        self.templates = templates;
        self
    }

    /// Supply external step/path statuses keyed by id.
    pub fn with_status(mut self, status: HashMap<SharedString, FlowRunStatus>) -> Self {
        self.controlled_status = status;
        self
    }

    /// Set the initially selected step id.
    pub fn with_selected(mut self, selected: Option<SharedString>) -> Self {
        self.selected_id = selected;
        self
    }

    /// Set the scrollable canvas viewport height in pixels (`452.0` by default).
    pub fn with_height(mut self, height: f32) -> Self {
        self.canvas_height = height.max(240.0);
        self
    }

    /// Toggle skeleton loading placeholders.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Force the trigger-less empty workflow state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        if empty {
            self.steps.clear();
        }
        self
    }

    /// Return the current top-level workflow steps.
    pub fn steps(&self) -> &[FlowStep] {
        &self.steps
    }

    /// Count all steps recursively including nested branch paths.
    pub fn total_step_count(&self) -> usize {
        count_flow_steps(&self.steps)
    }

    /// Return the currently selected step id, if any.
    pub fn selected_id(&self) -> Option<&str> {
        self.selected_id.as_deref()
    }

    /// Return whether an undo snapshot is available.
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Return whether a live test run is currently executing.
    pub fn is_running(&self) -> bool {
        self.live_run.is_some()
    }

    /// Select or clear a step id and emit [`FlowBuilderEvent::Select`].
    pub fn select_step(&mut self, id: Option<SharedString>, cx: &mut Context<Self>) {
        self.selected_id = id.clone();
        self.open_menu = None;
        cx.emit(FlowBuilderEvent::Select(id));
        cx.notify();
    }

    /// Insert `step` at `target`, record an undo snapshot, and emit [`FlowBuilderEvent::StepsChanged`].
    pub fn insert_step_at(
        &mut self,
        target: &FlowInsertTarget,
        step: FlowStep,
        cx: &mut Context<Self>,
    ) {
        self.undo_stack.push(self.steps.clone());
        let new_id = step.id.clone();
        let new_title = step.display_title(cx).to_string();
        match target {
            FlowInsertTarget::Root(idx) => {
                let pos = (*idx).min(self.steps.len());
                self.steps.insert(pos, step);
            }
            FlowInsertTarget::Branch {
                branch_id,
                path_idx,
                index,
            } => {
                insert_in_branch(&mut self.steps, branch_id.as_ref(), *path_idx, *index, step);
            }
        }
        self.empty = false;
        self.open_menu = None;
        self.last_finished_status.clear();
        self.selected_id = Some(new_id.clone());
        self.toast = Some((
            SharedString::from(format!(
                "{}: {new_title}",
                tr(cx, "Added step", "已添加步骤")
            )),
            Instant::now(),
        ));
        cx.emit(FlowBuilderEvent::StepsChanged(self.steps.clone()));
        cx.emit(FlowBuilderEvent::Select(Some(new_id)));
        cx.notify();
    }

    /// Remove `step_id` (lifting a `Branch` step's first path steps into place), record Undo, and emit [`FlowBuilderEvent::StepsChanged`].
    pub fn remove_step(&mut self, step_id: &str, cx: &mut Context<Self>) -> bool {
        let mut flat = Vec::new();
        flatten_step_ids(&self.steps, &mut flat);
        let prev_focus = flat
            .iter()
            .position(|id| id.as_ref() == step_id)
            .and_then(|pos| {
                if pos > 0 {
                    flat.get(pos - 1).cloned()
                } else {
                    flat.get(1).cloned()
                }
            });

        let snapshot = self.steps.clone();
        if let Some(removed_title) = remove_from_list(&mut self.steps, step_id, cx) {
            self.undo_stack.push(snapshot);
            self.last_finished_status.clear();
            self.open_menu = None;
            self.selected_id = prev_focus.clone();
            self.toast = Some((
                SharedString::from(format!(
                    "{} \"{removed_title}\"",
                    tr(cx, "Removed", "已移除")
                )),
                Instant::now(),
            ));
            cx.emit(FlowBuilderEvent::StepsChanged(self.steps.clone()));
            cx.emit(FlowBuilderEvent::Select(prev_focus));
            cx.notify();
            true
        } else {
            false
        }
    }

    /// Move `step_id` up (`delta = -1`) or down (`delta = 1`) within its containing sequence.
    pub fn move_step(&mut self, step_id: &str, delta: isize, cx: &mut Context<Self>) -> bool {
        let snapshot = self.steps.clone();
        if move_in_list(&mut self.steps, step_id, delta, true) {
            self.undo_stack.push(snapshot);
            self.last_finished_status.clear();
            cx.emit(FlowBuilderEvent::StepsChanged(self.steps.clone()));
            cx.notify();
            true
        } else {
            false
        }
    }

    /// Restore the last workflow snapshot from the undo stack.
    pub fn undo(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(prev) = self.undo_stack.pop() else {
            return false;
        };
        self.steps = prev;
        self.empty = self.steps.is_empty();
        self.open_menu = None;
        self.last_finished_status.clear();
        self.toast = Some((
            SharedString::from(tr(cx, "Undid last change", "已撤销上一步修改")),
            Instant::now(),
        ));
        cx.emit(FlowBuilderEvent::Undone);
        cx.emit(FlowBuilderEvent::StepsChanged(self.steps.clone()));
        cx.notify();
        true
    }

    /// Start a live sequential test run with connector light pulses and step progress meters.
    pub fn start_test_run(&mut self, cx: &mut Context<Self>) {
        if self.steps.is_empty() {
            return;
        }
        let mut t_cursor = 0.0f32;
        let mut schedule = Vec::new();
        let mut skipped_paths = Vec::new();
        let mut skipped_steps = Vec::new();
        build_run_schedule(
            &self.steps,
            &mut t_cursor,
            "root",
            &mut schedule,
            &mut skipped_paths,
            &mut skipped_steps,
        );
        self.open_menu = None;
        self.last_finished_status.clear();
        self.live_run = Some(LiveRun {
            started_at: Instant::now(),
            schedule,
            skipped_paths,
            skipped_steps,
            total_s: t_cursor.max(0.4),
        });
        cx.emit(FlowBuilderEvent::TestRunStarted);
        cx.notify();
    }

    /// Stop the active test run immediately.
    pub fn stop_test_run(&mut self, cx: &mut Context<Self>) {
        if self.live_run.take().is_some() {
            cx.emit(FlowBuilderEvent::TestRunStopped);
            cx.notify();
        }
    }

    fn instantiate_template(&mut self, tpl: &FlowTemplate) -> FlowStep {
        self.next_seq += 1;
        let id = format!("step-{}", self.next_seq);
        let mut step = FlowStep::new(id.clone(), tpl.kind, tpl.title.clone())
            .with_title_zh(tpl.title_zh.clone())
            .with_icon(tpl.icon.clone())
            .with_detail(tpl.detail.clone())
            .with_detail_zh(tpl.detail_zh.clone())
            .with_note(tpl.note.clone())
            .with_note_zh(tpl.note_zh.clone())
            .with_duration_s(if tpl.kind == FlowStepKind::Agent {
                1.3
            } else {
                0.7
            });
        if tpl.kind == FlowStepKind::Branch {
            step.paths = vec![
                FlowBranchPath::new(format!("{id}-yes"), "True")
                    .with_label_zh("满足条件")
                    .with_steps(vec![
                        FlowStep::new(
                            format!("{id}-yes-1"),
                            FlowStepKind::Action,
                            "Notify channel",
                        )
                        .with_title_zh("发送频道通知")
                        .with_icon("slack")
                        .with_detail("Slack · #ops-alerts")
                        .with_detail_zh("Slack · #ops-alerts")
                        .with_note("Posts context and owner mention")
                        .with_note_zh("推送上下文摘要并提醒负责人")
                        .with_duration_s(0.6),
                    ]),
                FlowBranchPath::new(format!("{id}-no"), "Else")
                    .with_label_zh("其他情况")
                    .with_steps(vec![
                        FlowStep::new(format!("{id}-no-1"), FlowStepKind::Action, "Log event")
                            .with_title_zh("记录审计日志")
                            .with_icon("database")
                            .with_detail("Postgres · workflow_events")
                            .with_detail_zh("Postgres · workflow_events")
                            .with_note("Appends structured trace row")
                            .with_note_zh("写入结构化执行追踪记录")
                            .with_duration_s(0.5),
                    ]),
            ];
        }
        step
    }
}

fn insert_in_branch(
    steps: &mut [FlowStep],
    branch_id: &str,
    path_idx: usize,
    index: usize,
    new_step: FlowStep,
) -> bool {
    for s in steps {
        if s.id.as_ref() == branch_id {
            if let Some(path) = s.paths.get_mut(path_idx) {
                let pos = index.min(path.steps.len());
                path.steps.insert(pos, new_step);
                return true;
            }
            return false;
        }
        for p in &mut s.paths {
            if insert_in_branch(&mut p.steps, branch_id, path_idx, index, new_step.clone()) {
                return true;
            }
        }
    }
    false
}

fn remove_from_list(steps: &mut Vec<FlowStep>, target_id: &str, cx: &App) -> Option<String> {
    if let Some(pos) = steps.iter().position(|s| s.id.as_ref() == target_id) {
        let removed = steps.remove(pos);
        let title = removed.display_title(cx).to_string();
        if removed.kind == FlowStepKind::Branch
            && let Some(first_path) = removed.paths.into_iter().next()
        {
            for (offset, child) in first_path.steps.into_iter().enumerate() {
                steps.insert(pos + offset, child);
            }
        }
        return Some(title);
    }
    for s in steps {
        for p in &mut s.paths {
            if let Some(t) = remove_from_list(&mut p.steps, target_id, cx) {
                return Some(t);
            }
        }
    }
    None
}

fn move_in_list(steps: &mut [FlowStep], target_id: &str, delta: isize, is_root: bool) -> bool {
    if let Some(pos) = steps.iter().position(|s| s.id.as_ref() == target_id) {
        if is_root && pos == 0 {
            return false;
        }
        let min_idx = usize::from(
            is_root
                && steps
                    .first()
                    .is_some_and(|s| s.kind == FlowStepKind::Trigger),
        );
        let next = pos as isize + delta;
        if next < min_idx as isize || next >= steps.len() as isize {
            return false;
        }
        steps.swap(pos, next as usize);
        return true;
    }
    for s in steps {
        for p in &mut s.paths {
            if move_in_list(&mut p.steps, target_id, delta, false) {
                return true;
            }
        }
    }
    false
}

#[derive(Clone)]
struct RuntimeSnapshot {
    step_status: HashMap<SharedString, (FlowRunStatus, f32, Option<f32>)>,
    skipped_paths: Vec<SharedString>,
    incoming_pulse: HashMap<SharedString, f32>,
    branch_pulse: HashMap<(SharedString, usize), f32>,
    active_banner: Option<(FlowRunStatus, String)>,
    step_opacities: HashMap<SharedString, f32>,
    path_opacities: HashMap<SharedString, f32>,
}

impl FlowBuilder {
    fn sample_runtime(&mut self, window: &mut Window, cx: &mut Context<Self>) -> RuntimeSnapshot {
        let reduced = disabled_motion(cx);
        let mut snap = RuntimeSnapshot {
            step_status: HashMap::new(),
            skipped_paths: Vec::new(),
            incoming_pulse: HashMap::new(),
            branch_pulse: HashMap::new(),
            active_banner: None,
            step_opacities: HashMap::new(),
            path_opacities: HashMap::new(),
        };

        for (k, &st) in &self.controlled_status {
            snap.step_status.insert(k.clone(), (st, 1.0, None));
        }
        for (k, &(st, dur)) in &self.last_finished_status {
            snap.step_status.insert(k.clone(), (st, 1.0, Some(dur)));
        }

        if let Some(run) = &self.live_run {
            let elapsed = if reduced {
                run.total_s + 0.1
            } else {
                run.started_at.elapsed().as_secs_f32()
            };

            if elapsed >= run.total_s {
                let mut finished = HashMap::new();
                for entry in &run.schedule {
                    finished.insert(
                        entry.step_id.clone(),
                        (FlowRunStatus::Done, entry.end_s - entry.start_s),
                    );
                }
                for sk in &run.skipped_steps {
                    finished.insert(sk.clone(), (FlowRunStatus::Skipped, 0.0));
                }
                for skp in &run.skipped_paths {
                    finished.insert(skp.clone(), (FlowRunStatus::Skipped, 0.0));
                }
                let done_steps = run.schedule.len();
                let total_s = run.total_s;
                self.last_finished_status = finished;
                self.live_run = None;
                cx.emit(FlowBuilderEvent::TestRunCompleted);
                for (k, &(st, dur)) in &self.last_finished_status {
                    snap.step_status
                        .insert(k.clone(), (st, 1.0, (dur > 0.0).then_some(dur)));
                    if st == FlowRunStatus::Skipped {
                        snap.skipped_paths.push(k.clone());
                    }
                }
                snap.active_banner = Some((
                    FlowRunStatus::Done,
                    tr(
                        cx,
                        &format!("Completed {done_steps} steps in {total_s:.1}s"),
                        &format!("已完成 {done_steps} 个步骤，总耗时 {total_s:.1}s"),
                    )
                    .to_string(),
                ));
            } else {
                window.request_animation_frame();
                let mut prev_end = 0.0f32;
                let mut prev_branch: Option<(SharedString, usize)> = None;
                for entry in &run.schedule {
                    if elapsed >= entry.end_s {
                        snap.step_status.insert(
                            entry.step_id.clone(),
                            (FlowRunStatus::Done, 1.0, Some(entry.end_s - entry.start_s)),
                        );
                    } else if elapsed >= entry.start_s {
                        let dur = (entry.end_s - entry.start_s).max(0.1);
                        let prog = ((elapsed - entry.start_s) / dur).clamp(0.0, 1.0);
                        snap.step_status.insert(
                            entry.step_id.clone(),
                            (FlowRunStatus::Running, prog, Some(elapsed - entry.start_s)),
                        );
                        let title = match &entry.title_zh {
                            Some(zh) => tr(cx, entry.title.as_ref(), zh.as_ref()),
                            None => entry.title.as_ref(),
                        };
                        snap.active_banner = Some((FlowRunStatus::Running, title.to_string()));
                    } else if elapsed >= prev_end {
                        let span = (entry.start_s - prev_end).max(0.05);
                        let p_prog = ((elapsed - prev_end) / span).clamp(0.0, 1.0);
                        snap.incoming_pulse
                            .insert(entry.incoming_key.clone(), p_prog);
                        if let Some(bc) = &prev_branch {
                            snap.branch_pulse.insert(bc.clone(), p_prog);
                        }
                        let title = match &entry.title_zh {
                            Some(zh) => tr(cx, entry.title.as_ref(), zh.as_ref()),
                            None => entry.title.as_ref(),
                        };
                        snap.active_banner = Some((FlowRunStatus::Running, title.to_string()));
                    }

                    if elapsed >= entry.end_s && entry.branch_choice.is_some() {
                        for sk in &run.skipped_paths {
                            snap.skipped_paths.push(sk.clone());
                        }
                        for sks in &run.skipped_steps {
                            snap.step_status
                                .insert(sks.clone(), (FlowRunStatus::Skipped, 0.0, None));
                        }
                    }

                    prev_end = entry.end_s;
                    prev_branch = entry.branch_choice.clone();
                }
            }
        } else if !self.last_finished_status.is_empty() {
            let mut done_n = 0usize;
            let mut sum_s = 0.0f32;
            for &(st, d) in self.last_finished_status.values() {
                if st == FlowRunStatus::Done {
                    done_n += 1;
                    sum_s += d;
                }
            }
            for (k, &(st, _)) in &self.last_finished_status {
                if st == FlowRunStatus::Skipped {
                    snap.skipped_paths.push(k.clone());
                }
            }
            if done_n > 0 {
                snap.active_banner = Some((
                    FlowRunStatus::Done,
                    tr(
                        cx,
                        &format!("Passed {done_n} steps · {sum_s:.1}s"),
                        &format!("已通过 {done_n} 个步骤 · {sum_s:.1}s"),
                    )
                    .to_string(),
                ));
            }
        }

        if let Some(m) = &self.open_menu {
            self.last_open_menu = Some(m.clone());
        }

        let focus_id = if self.live_run.is_none() {
            self.hovered_id.clone().or_else(|| self.selected_id.clone())
        } else {
            None
        };
        let (lit_steps, lit_paths) = match &focus_id {
            Some(fid) => collect_flow_spotlight(&self.steps, fid.as_ref()),
            None => (HashSet::new(), HashSet::new()),
        };

        let mut all_step_ids = Vec::new();
        flatten_step_ids(&self.steps, &mut all_step_ids);
        for sid in all_step_ids {
            let skipped = snap
                .step_status
                .get(&sid)
                .is_some_and(|&(st, _, _)| st == FlowRunStatus::Skipped);
            let target_op = if focus_id.is_some() {
                if lit_steps.contains(&sid) {
                    1.0_f32
                } else {
                    0.34_f32
                }
            } else if skipped {
                0.48_f32
            } else {
                1.0_f32
            };
            let op = pro_spring_f32(
                pro_eid(&self.id, format!("fstep-op-{sid}")),
                target_op,
                0.22,
                window,
                cx,
            );
            snap.step_opacities.insert(sid, op);
        }

        let mut all_path_ids = Vec::new();
        collect_all_path_ids(&self.steps, &mut all_path_ids);
        for pid in all_path_ids {
            let skipped = snap.skipped_paths.contains(&pid);
            let target_op = if focus_id.is_some() {
                if lit_paths.contains(&pid) {
                    1.0_f32
                } else {
                    0.34_f32
                }
            } else if skipped {
                0.48_f32
            } else {
                1.0_f32
            };
            let op = pro_spring_f32(
                pro_eid(&self.id, format!("fpath-op-{pid}")),
                target_op,
                0.22,
                window,
                cx,
            );
            snap.path_opacities.insert(pid, op);
        }

        snap
    }

    #[allow(clippy::too_many_arguments)]
    fn render_connector(
        &self,
        key: &str,
        aria_en: &str,
        aria_zh: &str,
        target: FlowInsertTarget,
        pulse: Option<f32>,
        skipped: bool,
        compact: bool,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let link_h = if compact { 36.0 } else { LINK_H };
        let card_bg: Hsla = if p.dark {
            rgb(0x1c1c1e).into()
        } else {
            p.panel
        };
        let link_c = pro_mix_color(p.blue, card_bg, if p.dark { 0.35 } else { 0.42 });
        let pulse_c = p.blue;
        let menu_open = self.open_menu.as_ref() == Some(&target);
        let is_running = self.live_run.is_some();

        let line_canvas = canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let mid_x = bounds.center().x;
                let y0 = f32::from(bounds.top());
                let y1 = f32::from(bounds.bottom());
                if skipped {
                    let mut y = y0;
                    while y < y1 {
                        let seg_end = (y + 3.5).min(y1);
                        let mut seg = PathBuilder::stroke(px(1.5));
                        seg.move_to(point(mid_x, px(y)));
                        seg.line_to(point(mid_x, px(seg_end)));
                        if let Ok(path) = seg.build() {
                            window.paint_path(path, link_c.opacity(0.45));
                        }
                        y += 7.0;
                    }
                } else {
                    pro_connector_pulse(
                        point(mid_x, px(y0)),
                        point(mid_x, px(y1)),
                        0.0,
                        true,
                        link_c,
                        pulse_c,
                        pulse,
                        1.6,
                        window,
                    );
                }
            },
        )
        .absolute()
        .inset_0();

        let port_dot = div()
            .absolute()
            .top(px(-3.5))
            .left(px(8.5))
            .size(px(7.))
            .rounded_full()
            .bg(card_bg)
            .border(px(1.5))
            .border_color(if skipped {
                link_c.opacity(0.45)
            } else {
                link_c
            });

        let target_for_btn = target.clone();
        let plus_fg = if menu_open { p.blue } else { p.muted };
        let plus_border = if menu_open { link_c } else { p.line_strong };
        let hover_border = link_c;
        let aria = tr(cx, aria_en, aria_zh).to_string();
        let menu_shown =
            (menu_open || self.last_open_menu.as_ref() == Some(&target)) && !is_running;

        let plus_btn: AnyElement = if is_running {
            div()
                .id(pro_eid(&self.id, format!("plus-{key}")))
                .size(px(20.))
                .opacity(0.0)
                .into_any_element()
        } else {
            pro_tactile(
                pro_eid(&self.id, format!("plus-{key}")),
                aria,
                10.0,
                card_bg,
                p.hover,
                plus_border,
                hover_border,
                menu_open,
                div()
                    .size(px(20.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(pro_icon(ProIcon::Plus, 11., plus_fg)),
                cx,
                move |this, _, cx| {
                    if this.open_menu.as_ref() == Some(&target_for_btn) {
                        this.open_menu = None;
                    } else {
                        this.last_open_menu = Some(target_for_btn.clone());
                        this.open_menu = Some(target_for_btn.clone());
                    }
                    cx.notify();
                },
            )
            .size(px(20.))
            .into_any_element()
        };

        let menu_el = if menu_shown {
            let pop_bg: Hsla = if p.dark { rgb(0x1f1f22).into() } else { p.card };
            let mut menu_col = div()
                .id(pro_eid(&self.id, format!("menu-{key}")))
                .role(Role::Group)
                .aria_label(tr(cx, "Insert workflow step", "插入工作流步骤"))
                .w(px(MENU_W))
                .p(px(4.))
                .rounded(px(10.))
                .bg(pop_bg)
                .border_1()
                .border_color(p.line_strong)
                .shadow_lg()
                .flex()
                .flex_col()
                .gap(px(2.));

            for (t_idx, tpl) in self.templates.iter().enumerate() {
                let tpl_clone = tpl.clone();
                let target_clone = target.clone();
                let item_hover = p.hover;
                let t_title = tr(cx, tpl.title.as_ref(), tpl.title_zh.as_ref()).to_string();
                let t_detail = tr(cx, tpl.detail.as_ref(), tpl.detail_zh.as_ref()).to_string();
                let tile_icon =
                    step_icon_element(tpl.kind, Some(tpl.icon.as_ref()), false, 0.0, p, cx);
                menu_col = menu_col.child(pro_tactile(
                    pro_eid(&self.id, format!("tpl-{key}-{t_idx}")),
                    t_title.clone(),
                    6.0,
                    transparent_black(),
                    item_hover,
                    transparent_black(),
                    transparent_black(),
                    false,
                    div()
                        .w_full()
                        .min_h(px(38.))
                        .px(px(8.))
                        .py(px(4.))
                        .flex()
                        .items_center()
                        .gap(px(9.))
                        .child(tile_icon)
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
                                        .text_color(p.ink)
                                        .truncate()
                                        .child(t_title),
                                )
                                .child(
                                    div()
                                        .text_size(px(11.))
                                        .text_color(p.muted)
                                        .truncate()
                                        .child(t_detail),
                                ),
                        ),
                    cx,
                    move |this, _, cx| {
                        let step = this.instantiate_template(&tpl_clone);
                        this.insert_step_at(&target_clone, step, cx);
                    },
                ));
            }
            Some(
                div()
                    .absolute()
                    .top(px(link_h * 0.5 + 12.0))
                    .left(px(-(MENU_W * 0.5 - 12.0)))
                    .child(pro_popover_presence(
                        pro_eid(&self.id, format!("menu-pop-{key}")),
                        menu_open && !is_running,
                        move || menu_col.into_any_element(),
                    )),
            )
        } else {
            None
        };

        div()
            .relative()
            .w(px(24.))
            .h(px(link_h))
            .flex()
            .items_center()
            .justify_center()
            .child(line_canvas)
            .child(port_dot)
            .child(plus_btn)
            .children(menu_el)
            .into_any_element()
    }

    #[allow(clippy::too_many_arguments)]
    fn render_step_card(
        &self,
        step: &FlowStep,
        can_move_up: bool,
        can_move_down: bool,
        can_remove: bool,
        snap: &RuntimeSnapshot,
        elapsed_ms: f32,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let (status, prog, shown_secs) =
            snap.step_status
                .get(&step.id)
                .copied()
                .unwrap_or((FlowRunStatus::Idle, 0.0, None));
        let is_running = status == FlowRunStatus::Running;
        let is_skipped = status == FlowRunStatus::Skipped;
        let is_selected = self.selected_id.as_ref() == Some(&step.id);
        let is_hovered = self.hovered_id.as_ref() == Some(&step.id);
        let step_op = snap.step_opacities.get(&step.id).copied().unwrap_or(1.0);

        let card_bg: Hsla = if is_skipped {
            transparent_black()
        } else if p.dark {
            rgb(0x1c1c1e).into()
        } else {
            p.card
        };
        let border_c: Hsla = if is_selected {
            p.blue.opacity(0.65)
        } else if is_running {
            p.blue.opacity(0.45)
        } else if is_hovered {
            p.line_strong
        } else if p.dark {
            hsla(0., 0., 1., 0.08)
        } else {
            hsla(0., 0., 0., 0.07)
        };
        let hover_bg = if is_skipped {
            p.hover
        } else {
            pro_mix_color(card_bg, p.blue, if p.dark { 0.05 } else { 0.03 })
        };

        let tile = step_icon_element(
            step.kind,
            step.icon.as_deref(),
            is_running,
            elapsed_ms,
            p,
            cx,
        );

        let badge_el = status_pill(&self.id, step.id.as_ref(), step.kind, status, p, cx);

        let note_str = step
            .display_note(cx)
            .unwrap_or_else(|| tr(cx, "Configured step", "已配置步骤"));
        let note_el = if is_running {
            pro_shimmer_text(
                pro_eid(&self.id, format!("shim-{}", step.id)),
                note_str.to_string(),
                p.text,
                p.strong,
                12.0,
                FontWeight::NORMAL,
                true,
            )
        } else {
            div()
                .w_full()
                .truncate()
                .text_size(px(12.))
                .line_height(px(16.))
                .text_color(p.text)
                .child(note_str.to_string())
                .into_any_element()
        };

        let link_c = pro_mix_color(p.blue, card_bg, if p.dark { 0.28 } else { 0.35 });
        let meter_fill = match status {
            FlowRunStatus::Running => p.blue,
            FlowRunStatus::Done => link_c,
            FlowRunStatus::Failed => p.red,
            FlowRunStatus::Idle | FlowRunStatus::Skipped => transparent_black(),
        };
        let meter_ratio = match status {
            FlowRunStatus::Running => prog.clamp(0.08, 1.0),
            FlowRunStatus::Done | FlowRunStatus::Failed => 1.0,
            FlowRunStatus::Idle | FlowRunStatus::Skipped => 0.0,
        };
        let dur_text = match shown_secs {
            Some(s) => format!("{s:.1}s"),
            None => "–".to_string(),
        };

        let ambient_wash = is_running.then(|| {
            let wash_top = p.blue.opacity(if p.dark { 0.20 } else { 0.14 });
            let wash_bot = p.blue.opacity(0.0);
            canvas(
                |_, _, _| (),
                move |bounds, _, window, _| {
                    let mut rect_path = PathBuilder::fill();
                    rect_path.move_to(bounds.origin);
                    rect_path.line_to(point(bounds.right(), bounds.top()));
                    rect_path.line_to(point(bounds.right(), bounds.bottom()));
                    rect_path.line_to(point(bounds.left(), bounds.bottom()));
                    rect_path.close();
                    if let Ok(path) = rect_path.build() {
                        window.paint_path(
                            path,
                            linear_gradient(
                                180.,
                                linear_color_stop(wash_top, 0.0),
                                linear_color_stop(wash_bot, 0.78),
                            ),
                        );
                    }
                },
            )
            .absolute()
            .inset_0()
        });

        let step_id_click = step.id.clone();
        let step_id_hover = step.id.clone();
        let show_tools = (is_hovered || is_selected) && self.live_run.is_none();

        let tools_overlay = {
            let pop_bg: Hsla = if p.dark { rgb(0x232327).into() } else { p.card };
            let mut bar = div()
                .h(px(22.))
                .px(px(3.))
                .rounded_full()
                .bg(pop_bg)
                .border_1()
                .border_color(p.line_strong)
                .shadow_sm()
                .flex()
                .items_center()
                .gap(px(2.));

            if can_move_up {
                let id_up = step.id.clone();
                bar = bar.child(
                    pro_icon_btn(
                        pro_eid(&self.id, format!("up-{}", step.id)),
                        tr(cx, "Move step up", "上移步骤"),
                        ProIcon::ChevronUp,
                        18.0,
                        false,
                        cx,
                        move |this, _, cx| {
                            this.move_step(id_up.as_ref(), -1, cx);
                        },
                    )
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_click(|_, _, cx| cx.stop_propagation()),
                );
            }
            if can_move_down {
                let id_dn = step.id.clone();
                bar = bar.child(
                    pro_icon_btn(
                        pro_eid(&self.id, format!("dn-{}", step.id)),
                        tr(cx, "Move step down", "下移步骤"),
                        ProIcon::ChevronDown,
                        18.0,
                        false,
                        cx,
                        move |this, _, cx| {
                            this.move_step(id_dn.as_ref(), 1, cx);
                        },
                    )
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_click(|_, _, cx| cx.stop_propagation()),
                );
            }
            if can_remove {
                let id_rm = step.id.clone();
                bar = bar.child(
                    pro_icon_btn(
                        pro_eid(&self.id, format!("rm-{}", step.id)),
                        tr(cx, "Remove step", "删除步骤"),
                        ProIcon::Close,
                        18.0,
                        false,
                        cx,
                        move |this, _, cx| {
                            this.remove_step(id_rm.as_ref(), cx);
                        },
                    )
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_click(|_, _, cx| cx.stop_propagation()),
                );
            }
            div()
                .absolute()
                .top(px(-10.))
                .right(px(6.))
                .child(pro_presence(
                    pro_eid(&self.id, format!("tools-pres-{}", step.id)),
                    show_tools,
                    0.15,
                    0.11,
                    move |amt| {
                        div()
                            .relative()
                            .top(px(2.0 * (1.0 - amt)))
                            .opacity(amt)
                            .child(bar)
                            .into_any_element()
                    },
                ))
        };

        let title_str = step.display_title(cx).to_string();
        let detail_str = step.display_detail(cx).unwrap_or("").to_string();
        let step_bar = pro_spring_bar(
            pro_eid(&self.id, format!("sbar-{}", step.id)),
            meter_ratio,
            3.0,
            2.0,
            p.line,
            meter_fill,
        );

        div()
            .relative()
            .w(px(STEP_W))
            .opacity(step_op)
            .child(
                pro_tactile(
                    pro_eid(&self.id, format!("card-{}", step.id)),
                    title_str.clone(),
                    8.0,
                    card_bg,
                    hover_bg,
                    border_c,
                    p.line_strong,
                    is_selected,
                    div()
                        .relative()
                        .w(px(STEP_W))
                        .h(px(STEP_H))
                        .px(px(10.))
                        .pt(px(10.))
                        .pb(px(9.))
                        .overflow_hidden()
                        .children(ambient_wash)
                        .child(
                            div()
                                .relative()
                                .size_full()
                                .flex()
                                .flex_col()
                                .justify_between()
                                .child(
                                    div()
                                        .h(px(30.))
                                        .flex()
                                        .items_center()
                                        .gap(px(8.))
                                        .child(tile)
                                        .child(
                                            div()
                                                .flex_1()
                                                .min_w_0()
                                                .flex()
                                                .flex_col()
                                                .child(
                                                    div()
                                                        .text_size(px(13.))
                                                        .line_height(px(16.))
                                                        .font_weight(FontWeight::MEDIUM)
                                                        .text_color(p.strong)
                                                        .truncate()
                                                        .child(title_str),
                                                )
                                                .child(
                                                    div()
                                                        .text_size(px(11.5))
                                                        .line_height(px(14.))
                                                        .text_color(p.muted)
                                                        .truncate()
                                                        .child(detail_str),
                                                ),
                                        )
                                        .children(badge_el),
                                )
                                .child(note_el)
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(8.))
                                        .child(div().flex_1().child(step_bar))
                                        .child(
                                            div()
                                                .min_w(px(26.))
                                                .text_size(px(11.))
                                                .text_color(p.muted)
                                                .text_right()
                                                .child(dur_text),
                                        ),
                                ),
                        ),
                    cx,
                    move |this, _, cx| {
                        let next = if this.selected_id.as_ref() == Some(&step_id_click) {
                            None
                        } else {
                            Some(step_id_click.clone())
                        };
                        this.select_step(next, cx);
                    },
                )
                .w(px(STEP_W))
                .h(px(STEP_H))
                .when(!p.dark && !is_skipped, |d| d.shadow_sm())
                .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                    if *inside {
                        this.hovered_id = Some(step_id_hover.clone());
                    } else if this.hovered_id.as_ref() == Some(&step_id_hover) {
                        this.hovered_id = None;
                    }
                    cx.notify();
                })),
            )
            .child(tools_overlay)
            .into_any_element()
    }

    #[allow(clippy::too_many_arguments)]
    fn render_sequence(
        &self,
        steps: &[FlowStep],
        branch_ctx: Option<(&SharedString, usize, bool)>,
        snap: &RuntimeSnapshot,
        elapsed_ms: f32,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> Div {
        let mut col = div().flex().flex_col().items_center();
        let is_root = branch_ctx.is_none();

        if let Some((b_id, p_idx, lane_skipped)) = branch_ctx {
            let lead_key = format!("bstart-{b_id}-{p_idx}");
            let first_pulse = steps.first().and_then(|s| {
                snap.incoming_pulse
                    .get(&SharedString::from(format!("in:{}", s.id)))
                    .copied()
            });
            col = col.child(self.render_connector(
                &lead_key,
                "Add step at start of branch",
                "在分支起点添加步骤",
                FlowInsertTarget::Branch {
                    branch_id: b_id.clone(),
                    path_idx: p_idx,
                    index: 0,
                },
                first_pulse,
                lane_skipped,
                true,
                p,
                cx,
            ));
        }

        for (idx, step) in steps.iter().enumerate() {
            if is_root && idx > 0 {
                let prev_title = steps[idx - 1].title.as_ref();
                let pulse = snap
                    .incoming_pulse
                    .get(&SharedString::from(format!("in:{}", step.id)))
                    .copied();
                let skipped = snap
                    .step_status
                    .get(&step.id)
                    .is_some_and(|&(st, _, _)| st == FlowRunStatus::Skipped);
                col = col.child(self.render_connector(
                    &format!("root-{idx}"),
                    &format!("Add a step after {prev_title}"),
                    "在此步骤后添加步骤",
                    FlowInsertTarget::Root(idx),
                    pulse,
                    skipped,
                    false,
                    p,
                    cx,
                ));
            } else if let Some((b_id, p_idx, lane_skipped)) = branch_ctx
                && idx > 0
            {
                let pulse = snap
                    .incoming_pulse
                    .get(&SharedString::from(format!("in:{}", step.id)))
                    .copied();
                col = col.child(self.render_connector(
                    &format!("br-{b_id}-{p_idx}-{idx}"),
                    "Add step in branch",
                    "在分支内添加步骤",
                    FlowInsertTarget::Branch {
                        branch_id: b_id.clone(),
                        path_idx: p_idx,
                        index: idx,
                    },
                    pulse,
                    lane_skipped,
                    false,
                    p,
                    cx,
                ));
            }

            let min_move_idx = usize::from(
                is_root
                    && steps
                        .first()
                        .is_some_and(|s| s.kind == FlowStepKind::Trigger),
            );
            let can_up = idx > min_move_idx;
            let can_down = (!is_root || idx >= min_move_idx) && idx + 1 < steps.len();
            let can_rm =
                !(is_root && idx == 0 && step.kind == FlowStepKind::Trigger && steps.len() > 1);

            col = col.child(
                self.render_step_card(step, can_up, can_down, can_rm, snap, elapsed_ms, p, cx),
            );

            if step.kind == FlowStepKind::Branch && !step.paths.is_empty() {
                col = col.child(self.render_branch_fork(step, snap, elapsed_ms, p, cx));
            }
        }

        // Tail '+' connector to append at the end of a lane or non-branching root
        let ends_with_branch = steps.last().is_some_and(|s| s.kind == FlowStepKind::Branch);
        if !ends_with_branch {
            let (tail_key, target, skipped) = match branch_ctx {
                Some((b_id, p_idx, lane_skipped)) => (
                    format!("btail-{b_id}-{p_idx}"),
                    FlowInsertTarget::Branch {
                        branch_id: b_id.clone(),
                        path_idx: p_idx,
                        index: steps.len(),
                    },
                    lane_skipped,
                ),
                None => (
                    "root-tail".to_string(),
                    FlowInsertTarget::Root(steps.len()),
                    false,
                ),
            };
            col = col.child(self.render_connector(
                &tail_key,
                "Add step at end",
                "在末尾追加步骤",
                target,
                None,
                skipped,
                true,
                p,
                cx,
            ));
        }

        col
    }

    fn render_branch_fork(
        &self,
        branch: &FlowStep,
        snap: &RuntimeSnapshot,
        elapsed_ms: f32,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let lane_count = branch.paths.len();
        let total_w = lane_count as f32 * LANE_W;
        let card_bg: Hsla = if p.dark {
            rgb(0x1c1c1e).into()
        } else {
            p.panel
        };
        let link_c = pro_mix_color(p.blue, card_bg, if p.dark { 0.35 } else { 0.42 });
        let pulse_c = p.blue;

        type LaneArcSpec = (usize, f32, Option<f32>);
        let arc_specs: Vec<LaneArcSpec> = (0..lane_count)
            .map(|idx| {
                let p_id = &branch.paths[idx].id;
                let path_op = snap.path_opacities.get(p_id).copied().unwrap_or(1.0);
                let pulse = snap.branch_pulse.get(&(branch.id.clone(), idx)).copied();
                (idx, path_op, pulse)
            })
            .collect();

        let fork_canvas = canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let mid_x = bounds.center().x;
                let top_y = bounds.top();
                let bot_y = bounds.bottom();
                for &(idx, path_op, pulse) in &arc_specs {
                    let lane_cx = bounds.left() + px(idx as f32 * LANE_W + LANE_W * 0.5);
                    let base = link_c.opacity(path_op.clamp(0.28, 1.0));
                    pro_connector_pulse(
                        point(mid_x, top_y),
                        point(lane_cx, bot_y),
                        10.0,
                        true,
                        base,
                        pulse_c,
                        pulse,
                        1.6,
                        window,
                    );
                }
            },
        )
        .w(px(total_w))
        .h(px(FORK_TOP_H));

        let port_dot = div()
            .absolute()
            .top(px(-3.5))
            .left(px(total_w * 0.5 - 3.5))
            .size(px(7.))
            .rounded_full()
            .bg(card_bg)
            .border(px(1.5))
            .border_color(link_c);

        let brand_bg: Hsla = if p.dark {
            p.blue.opacity(0.14)
        } else {
            hsla(205. / 360., 1.0, 0.48, 0.10)
        };
        let brand_fg: Hsla = if p.dark {
            rgb(0x6cbcff).into()
        } else {
            rgb(0x006dcb).into()
        };

        let mut lanes_row = div().flex().items_start();
        for (p_idx, path) in branch.paths.iter().enumerate() {
            let skipped = snap.skipped_paths.contains(&path.id);
            let path_op = snap.path_opacities.get(&path.id).copied().unwrap_or(1.0);
            let pill_label = path.display_label(cx).to_string();
            let seq = self.render_sequence(
                &path.steps,
                Some((&branch.id, p_idx, skipped)),
                snap,
                elapsed_ms,
                p,
                cx,
            );
            let lane = div()
                .w(px(LANE_W))
                .px(px(LANE_PAD_X))
                .flex()
                .flex_col()
                .items_center()
                .child(
                    div()
                        .h(px(18.))
                        .px(px(8.))
                        .rounded_full()
                        .bg(brand_bg)
                        .opacity(path_op)
                        .text_size(px(11.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(brand_fg)
                        .when(skipped, |d| d.line_through())
                        .flex()
                        .items_center()
                        .child(pill_label),
                )
                .child(seq);
            lanes_row = lanes_row.child(lane);
        }

        div()
            .relative()
            .flex()
            .flex_col()
            .items_center()
            .child(fork_canvas)
            .child(port_dot)
            .child(lanes_row)
            .into_any_element()
    }
}

fn step_icon_element(
    kind: FlowStepKind,
    icon_key: Option<&str>,
    is_running: bool,
    elapsed_ms: f32,
    p: &ProPalette,
    cx: &App,
) -> AnyElement {
    if kind == FlowStepKind::Agent {
        return pro_agent_avatar(
            icon_key.unwrap_or("flow-agent"),
            p.blue,
            24.0,
            is_running,
            elapsed_ms,
            cx,
        );
    }
    if let Some(key) = icon_key
        && matches!(
            key,
            "slack" | "gmail" | "hubspot" | "stripe" | "github" | "notion" | "linear"
        )
    {
        return pro_app_tile(key, 24.0, cx);
    }

    let is_trigger = kind == FlowStepKind::Trigger;
    let tile_bg: Hsla = if is_trigger {
        p.primary
    } else if p.dark {
        rgb(0x222226).into()
    } else {
        p.card
    };
    let tile_fg = if is_trigger { p.on_primary } else { p.ink };
    let ic = match (kind, icon_key) {
        (FlowStepKind::Branch, _) | (_, Some("branch")) => ProIcon::GitBranch,
        (FlowStepKind::Wait, _) | (_, Some("wait" | "schedule")) => ProIcon::Clock,
        (_, Some("mail")) => ProIcon::Mail,
        (_, Some("database")) => ProIcon::Database,
        (_, Some("http" | "globe")) => ProIcon::Globe,
        (_, Some("webhook" | "flash")) => ProIcon::Bolt,
        (_, Some("user")) => ProIcon::User,
        (_, Some("task")) => ProIcon::Tasks,
        (_, Some("form")) => ProIcon::Note,
        (FlowStepKind::Trigger, _) => ProIcon::Bolt,
        _ => ProIcon::Sparkles,
    };

    div()
        .size(px(24.))
        .flex_none()
        .rounded(px(7.))
        .bg(tile_bg)
        .when(!is_trigger, |d| d.border_1().border_color(p.line_strong))
        .flex()
        .items_center()
        .justify_center()
        .child(pro_icon(ic, 13.5, tile_fg))
        .into_any_element()
}

fn status_pill(
    base_id: &ElementId,
    key: &str,
    kind: FlowStepKind,
    status: FlowRunStatus,
    p: &ProPalette,
    cx: &App,
) -> Option<AnyElement> {
    let (bg, fg, label, check) = match status {
        FlowRunStatus::Running => (
            if p.dark {
                p.blue.opacity(0.16)
            } else {
                hsla(205. / 360., 1.0, 0.48, 0.10)
            },
            if p.dark {
                rgb(0x6cbcff).into()
            } else {
                rgb(0x006dcb).into()
            },
            tr(cx, "Running", "运行中"),
            false,
        ),
        FlowRunStatus::Done => (p.green_soft, p.green_ink, tr(cx, "Passed", "已通过"), true),
        FlowRunStatus::Failed => (p.red_soft, p.red_ink, tr(cx, "Failed", "失败"), false),
        FlowRunStatus::Skipped => (p.chip, p.muted, tr(cx, "Skipped", "已跳过"), false),
        FlowRunStatus::Idle => {
            if kind == FlowStepKind::Branch {
                (p.chip, p.muted, tr(cx, "If / else", "条件分支"), false)
            } else {
                return None;
            }
        }
    };

    Some(
        div()
            .h(px(18.))
            .px(px(6.))
            .flex_none()
            .rounded(px(4.))
            .bg(bg)
            .text_size(px(11.))
            .font_weight(FontWeight::MEDIUM)
            .text_color(fg)
            .flex()
            .items_center()
            .gap(px(3.5))
            .when(check, |d| {
                d.child(pro_check_draw(
                    pro_eid(base_id, format!("chk-{key}")),
                    true,
                    12.0,
                    false,
                    fg,
                ))
            })
            .when(status == FlowRunStatus::Running, |d| {
                d.child(
                    div()
                        .size(px(6.))
                        .rounded_full()
                        .border(px(1.5))
                        .border_color(fg),
                )
            })
            .child(label)
            .into_any_element(),
    )
}

impl Render for FlowBuilder {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let reduced = disabled_motion(cx);
        let elapsed_ms = self.started_at.elapsed().as_secs_f32() * 1_000.0;

        if self.loading {
            if !reduced {
                window.request_animation_frame();
            }
            let phase = (elapsed_ms / 1_400.0).fract();
            return div()
                .id(self.id.clone())
                .w_full()
                .max_w(px(640.))
                .flex()
                .flex_col()
                .items_center()
                .gap(px(16.))
                .child(
                    div()
                        .w_full()
                        .flex()
                        .justify_between()
                        .child(pro_skeleton_bar(190., 16., 5., phase, cx))
                        .child(pro_skeleton_bar(84., 28., 6., phase + 0.1, cx)),
                )
                .child(pro_skeleton_bar(256., 92., 8., phase + 0.15, cx))
                .child(pro_skeleton_bar(256., 92., 8., phase + 0.25, cx))
                .child(pro_skeleton_bar(256., 92., 8., phase + 0.35, cx))
                .into_any_element();
        }

        let snap = self.sample_runtime(window, cx);
        let has_running_agent = snap
            .step_status
            .values()
            .any(|&(st, _, _)| st == FlowRunStatus::Running);
        if has_running_agent && !reduced {
            window.request_animation_frame();
        }

        let total_steps = self.total_step_count();
        let is_running = self.live_run.is_some();
        let card_bg: Hsla = if p.dark { rgb(0x1c1c1e).into() } else { p.card };

        let run_btn: AnyElement = {
            let can_run = !self.steps.is_empty();
            let hover_bg = if is_running {
                p.chip_hover
            } else {
                pro_mix_color(p.primary, p.on_primary, 0.12)
            };
            let btn_label = if is_running {
                tr(cx, "Stop", "停止")
            } else {
                tr(cx, "Test run", "测试运行")
            };
            let rest_bg = if is_running { card_bg } else { p.primary };
            let rest_bdr = if is_running { p.line_strong } else { p.primary };
            let fg_c = if is_running { p.ink } else { p.on_primary };
            if can_run {
                pro_tactile(
                    pro_eid(&self.id, "run-btn"),
                    btn_label,
                    6.0,
                    rest_bg,
                    hover_bg,
                    rest_bdr,
                    rest_bdr,
                    false,
                    div()
                        .h(px(28.))
                        .px(px(10.))
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .child(if is_running {
                            div()
                                .size(px(9.))
                                .rounded(px(2.))
                                .bg(p.ink)
                                .into_any_element()
                        } else {
                            pro_icon(ProIcon::Play, 11.5, p.on_primary)
                        })
                        .child(pro_swap_label(
                            pro_eid(&self.id, "run-btn-lbl"),
                            btn_label,
                            12.5,
                            FontWeight::MEDIUM,
                            fg_c,
                        )),
                    cx,
                    |this, _, cx| {
                        if this.is_running() {
                            this.stop_test_run(cx);
                        } else {
                            this.start_test_run(cx);
                        }
                    },
                )
                .into_any_element()
            } else {
                div()
                    .id(pro_eid(&self.id, "run-btn"))
                    .role(Role::Button)
                    .aria_label(btn_label)
                    .tab_index(-1)
                    .h(px(28.))
                    .px(px(10.))
                    .rounded(px(6.))
                    .bg(p.primary)
                    .text_color(p.on_primary)
                    .text_size(px(12.5))
                    .font_weight(FontWeight::MEDIUM)
                    .opacity(0.42)
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(pro_icon(ProIcon::Play, 11.5, p.on_primary))
                    .child(btn_label)
                    .into_any_element()
            }
        };

        let steps_sfx = tr(cx, " steps", " 个步骤");
        let header = div()
            .w_full()
            .min_h(px(32.))
            .mb(px(6.))
            .flex()
            .items_center()
            .justify_between()
            .gap(px(12.))
            .child(
                div()
                    .flex()
                    .items_baseline()
                    .gap(px(8.))
                    .min_w_0()
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .truncate()
                            .child(tr(cx, self.name.as_ref(), self.name_zh.as_ref()).to_string()),
                    )
                    .child(pro_rolling_number(
                        pro_eid(&self.id, "step-count"),
                        total_steps as f64,
                        0,
                        "",
                        steps_sfx,
                        12.0,
                        FontWeight::NORMAL,
                        p.muted,
                    )),
            )
            .child(run_btn);

        let canvas_body = if self.empty || self.steps.is_empty() {
            div()
                .w_full()
                .h(px(self.canvas_height))
                .flex()
                .items_center()
                .justify_center()
                .child(pro_empty_state(
                    ProIcon::Bolt,
                    tr(cx, "Start with a trigger", "从触发器开始构建工作流"),
                    tr(
                        cx,
                        "Add a signup, webhook, or schedule trigger to assemble your automation.",
                        "添加表单、Webhook 或定时触发器，原地编排后续智能体与条件分支。",
                    ),
                    Some((
                        pro_eid(&self.id, "add-trigger"),
                        SharedString::from(tr(cx, "Load onboarding flow", "加载示例工作流")),
                        Box::new(|this: &mut Self, _, cx| {
                            this.empty = false;
                            this.steps = default_onboarding_steps();
                            cx.emit(FlowBuilderEvent::StepsChanged(this.steps.clone()));
                            cx.notify();
                        }),
                    )),
                    cx,
                ))
                .into_any_element()
        } else {
            let flow_tree = self.render_sequence(&self.steps, None, &snap, elapsed_ms, &p, cx);
            div()
                .id(pro_eid(&self.id, "canvas"))
                .role(Role::Group)
                .aria_label(tr(cx, "Workflow steps", "工作流步骤画布"))
                .tab_index(0)
                .w_full()
                .h(px(self.canvas_height))
                .overflow_y_scroll()
                .focus_visible(move |s| s.border_1().border_color(p.focus))
                .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _, cx| {
                    let mut flat = Vec::new();
                    flatten_step_ids(&this.steps, &mut flat);
                    if ev.keystroke.modifiers.platform && ev.keystroke.key.as_str() == "z" {
                        this.undo(cx);
                        cx.stop_propagation();
                        return;
                    }
                    if ev.keystroke.modifiers.alt
                        && let Some(sel) = this.selected_id.clone()
                    {
                        match ev.keystroke.key.as_str() {
                            "up" | "arrowup" => {
                                this.move_step(sel.as_ref(), -1, cx);
                                cx.stop_propagation();
                                return;
                            }
                            "down" | "arrowdown" => {
                                this.move_step(sel.as_ref(), 1, cx);
                                cx.stop_propagation();
                                return;
                            }
                            _ => {}
                        }
                    }
                    let cur_pos = this
                        .selected_id
                        .as_ref()
                        .and_then(|s| flat.iter().position(|id| id == s));
                    match ev.keystroke.key.as_str() {
                        "up" | "arrowup" if !flat.is_empty() => {
                            let next = cur_pos.map_or(0, |i| i.saturating_sub(1));
                            this.select_step(Some(flat[next].clone()), cx);
                            cx.stop_propagation();
                        }
                        "down" | "arrowdown" if !flat.is_empty() => {
                            let next = cur_pos.map_or(0, |i| (i + 1).min(flat.len() - 1));
                            this.select_step(Some(flat[next].clone()), cx);
                            cx.stop_propagation();
                        }
                        "home" if !flat.is_empty() => {
                            this.select_step(Some(flat[0].clone()), cx);
                            cx.stop_propagation();
                        }
                        "end" if !flat.is_empty() => {
                            this.select_step(flat.last().cloned(), cx);
                            cx.stop_propagation();
                        }
                        "backspace" | "delete" => {
                            if let Some(sel) = this.selected_id.clone() {
                                this.remove_step(sel.as_ref(), cx);
                                cx.stop_propagation();
                            }
                        }
                        "escape" => {
                            if this.open_menu.is_some() {
                                this.open_menu = None;
                                cx.notify();
                            } else if this.selected_id.is_some() {
                                this.select_step(None, cx);
                            }
                            cx.stop_propagation();
                        }
                        _ => {}
                    }
                }))
                .child(
                    div()
                        .w_full()
                        .py(px(18.))
                        .px(px(8.))
                        .flex()
                        .flex_col()
                        .items_center()
                        .child(flow_tree),
                )
                .into_any_element()
        };

        let left_foot = if let Some((st, msg)) = &snap.active_banner {
            let pill = status_pill(&self.id, "foot-banner", FlowStepKind::Action, *st, &p, cx);
            div()
                .flex()
                .items_center()
                .gap(px(8.))
                .children(pill)
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(p.muted)
                        .truncate()
                        .child(msg.clone()),
                )
                .into_any_element()
        } else if let Some(sel_id) = &self.selected_id {
            div()
                .flex()
                .items_center()
                .gap(px(8.))
                .text_size(px(12.))
                .text_color(p.muted)
                .child(
                    tr(
                        cx,
                        &format!("Selected {sel_id} · ⌥↑↓ reorder · ⌫ remove"),
                        &format!("已选 {sel_id} · ⌥↑↓ 调整顺序 · ⌫ 删除"),
                    )
                    .to_string(),
                )
                .into_any_element()
        } else {
            div()
                .text_size(px(12.))
                .text_color(p.muted)
                .child(tr(
                    cx,
                    "Click + on any connector to insert a step, or Test run to trace live",
                    "点击任意连线上的 + 插入步骤，或点击“测试运行”观察脉冲执行",
                ))
                .into_any_element()
        };

        let undo_btn = self.can_undo().then(|| {
            pro_btn(
                pro_eid(&self.id, "undo-btn"),
                tr(cx, "Undo", "撤销"),
                Some(ProIcon::Undo),
                ProBtnStyle::Secondary,
                true,
                cx,
                |this, _, cx| {
                    this.undo(cx);
                },
            )
        });

        let footer = div()
            .w_full()
            .h(px(32.))
            .mt(px(4.))
            .flex()
            .items_center()
            .justify_between()
            .gap(px(8.))
            .child(left_foot)
            .children(undo_btn);

        div()
            .id(self.id.clone())
            .relative()
            .w_full()
            .flex()
            .justify_center()
            .child(
                div()
                    .relative()
                    .w_full()
                    .max_w(px(620.))
                    .flex()
                    .flex_col()
                    .child(header)
                    .child(canvas_body)
                    .child(footer)
                    .child(pro_toast_banner(self.toast.clone())),
            )
            .into_any_element()
    }
}

fn default_onboarding_steps() -> Vec<FlowStep> {
    vec![
        FlowStep::new("signup", FlowStepKind::Trigger, "New signup")
            .with_title_zh("新用户注册")
            .with_icon("form")
            .with_detail("Signup form")
            .with_detail_zh("官网注册表单")
            .with_note("Someone signs up on the website")
            .with_note_zh("访客在产品官网提交注册信息")
            .with_duration_s(0.6),
        FlowStep::new("research", FlowStepKind::Agent, "Research the company")
            .with_title_zh("调研企业背景")
            .with_icon("research-agent")
            .with_detail("Claude · web search")
            .with_detail_zh("Claude · 联网检索")
            .with_note("Reads their site, finds size and industry")
            .with_note_zh("读取官网，识别团队规模与所属行业")
            .with_duration_s(1.2),
        FlowStep::new("company-size", FlowStepKind::Branch, "Company size")
            .with_title_zh("判断企业规模")
            .with_icon("branch")
            .with_detail("employees > 200")
            .with_detail_zh("员工人数 > 200")
            .with_note("Goes on as Enterprise or Self-serve")
            .with_note_zh("按企业大客户或自助团队分流")
            .with_duration_s(0.5)
            .with_paths(vec![
                FlowBranchPath::new("enterprise", "Enterprise")
                    .with_label_zh("企业客户")
                    .with_steps(vec![
                        FlowStep::new("crm-deal", FlowStepKind::Action, "Create CRM deal")
                            .with_title_zh("创建 CRM 商机")
                            .with_icon("hubspot")
                            .with_detail("HubSpot · Enterprise pipeline")
                            .with_detail_zh("HubSpot · 企业销售漏斗")
                            .with_note("Opens deal with enriched headcount")
                            .with_note_zh("附带团队规模与行业数据创建商机")
                            .with_duration_s(0.7),
                        FlowStep::new("slack-ae", FlowStepKind::Action, "Notify account exec")
                            .with_title_zh("通知客户经理")
                            .with_icon("slack")
                            .with_detail("Slack · #enterprise-signups")
                            .with_detail_zh("Slack · #enterprise-signups")
                            .with_note("Posts company brief and booking link")
                            .with_note_zh("发送客户简报与会议预约链接")
                            .with_duration_s(0.6),
                    ]),
                FlowBranchPath::new("self-serve", "Self-serve")
                    .with_label_zh("自助团队")
                    .with_steps(vec![
                        FlowStep::new("welcome-mail", FlowStepKind::Action, "Send welcome email")
                            .with_title_zh("发送欢迎邮件")
                            .with_icon("gmail")
                            .with_detail("Gmail · personalized note")
                            .with_detail_zh("Gmail · 个性化欢迎信")
                            .with_note("A personal welcome written from research")
                            .with_note_zh("结合调研摘要生成定制欢迎邮件")
                            .with_duration_s(0.7),
                        FlowStep::new("wait-2d", FlowStepKind::Wait, "Wait 2 days")
                            .with_title_zh("等待 2 天")
                            .with_icon("wait")
                            .with_detail("Delay · check workspace activity")
                            .with_detail_zh("延时 · 检查工作区活跃度")
                            .with_note("Pauses until initial trial session")
                            .with_note_zh("等待用户完成首次试用操作")
                            .with_duration_s(0.5),
                        FlowStep::new("quickstart", FlowStepKind::Action, "Send quickstart tips")
                            .with_title_zh("发送快速上手指南")
                            .with_icon("gmail")
                            .with_detail("Gmail · activation guide")
                            .with_detail_zh("Gmail · 激活引导")
                            .with_note("Shares tailored templates for their stack")
                            .with_note_zh("推送匹配其技术栈的模板")
                            .with_duration_s(0.6),
                    ]),
            ]),
    ]
}

fn default_templates() -> Vec<FlowTemplate> {
    vec![
        FlowTemplate {
            kind: FlowStepKind::Agent,
            icon: SharedString::from("enrich-agent"),
            title: SharedString::from("AI research step"),
            title_zh: SharedString::from("AI 联网调研步骤"),
            detail: SharedString::from("Claude · web & docs"),
            detail_zh: SharedString::from("Claude · 网页与文档检索"),
            note: SharedString::from("Summarizes account signals and intent"),
            note_zh: SharedString::from("归纳客户意图与关键业务信号"),
        },
        FlowTemplate {
            kind: FlowStepKind::Action,
            icon: SharedString::from("slack"),
            title: SharedString::from("Send Slack message"),
            title_zh: SharedString::from("发送 Slack 通知"),
            detail: SharedString::from("Slack · #growth-feed"),
            detail_zh: SharedString::from("Slack · #growth-feed"),
            note: SharedString::from("Posts formatted summary to channel"),
            note_zh: SharedString::from("向指定频道发送结构化摘要"),
        },
        FlowTemplate {
            kind: FlowStepKind::Action,
            icon: SharedString::from("gmail"),
            title: SharedString::from("Send email"),
            title_zh: SharedString::from("发送电子邮件"),
            detail: SharedString::from("Gmail · automated follow-up"),
            detail_zh: SharedString::from("Gmail · 自动跟进邮件"),
            note: SharedString::from("Delivers personalized follow-up note"),
            note_zh: SharedString::from("发送个性化跟进邮件"),
        },
        FlowTemplate {
            kind: FlowStepKind::Wait,
            icon: SharedString::from("wait"),
            title: SharedString::from("Wait 24 hours"),
            title_zh: SharedString::from("等待 24 小时"),
            detail: SharedString::from("Delay · business hours"),
            detail_zh: SharedString::from("延时 · 工作日窗口"),
            note: SharedString::from("Holds until the next workday window"),
            note_zh: SharedString::from("暂停至下一个工作日时间窗"),
        },
        FlowTemplate {
            kind: FlowStepKind::Branch,
            icon: SharedString::from("branch"),
            title: SharedString::from("If / else condition"),
            title_zh: SharedString::from("If / else 条件分支"),
            detail: SharedString::from("Split into True / Else paths"),
            detail_zh: SharedString::from("拆分为双命名路径"),
            note: SharedString::from("Routes execution based on rule match"),
            note_zh: SharedString::from("根据条件表达式分流执行路径"),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::{FlowBuilder, FlowInsertTarget, FlowStep, FlowStepKind, count_flow_steps};
    use gpui::{AppContext as _, TestAppContext};

    #[gpui::test]
    fn insert_remove_branch_lift_and_undo_preserve_step_hierarchy(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let view = cx.new(|cx| FlowBuilder::new("test-flb", cx));

        view.update(cx, |v, cx| {
            assert_eq!(v.total_step_count(), 8);
            assert!(!v.can_undo());

            // Insert a step after New signup (Root(1))
            v.insert_step_at(
                &FlowInsertTarget::Root(1),
                FlowStep::new("extra", FlowStepKind::Action, "Verify domain"),
                cx,
            );
            assert_eq!(v.total_step_count(), 9);
            assert_eq!(v.steps()[1].id.as_ref(), "extra");
            assert!(v.can_undo());

            // Removing the Branch ("company-size") lifts its first path ("enterprise": 2 steps) into root!
            assert!(v.remove_step("company-size", cx));
            // 9 total - 1 branch - 3 self-serve steps = 5 steps remaining
            assert_eq!(count_flow_steps(v.steps()), 5);
            assert!(v.steps().iter().any(|s| s.id.as_ref() == "crm-deal"));

            // Undo restores the full 9-step tree including the branch
            assert!(v.undo(cx));
            assert_eq!(v.total_step_count(), 9);
        });
    }
}
