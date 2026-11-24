//! Process swimlane diagram across people, agents, and tools with live handoff pulses and inline gate approvals.
//!
//! Renders one horizontal lane per participant (`You`, each `Agent`, and each `Tool`),
//! places tasks, start/end circles, and `PathBuilder` approval diamonds into topological columns,
//! animates traveling light pulses along active handoffs via [`pro_connector_pulse`], and opens
//! an interactive detail card on any lane or node where pending gates can be approved or rejected.

use crate::{
    components::pro_chrome::{
        ProBtnStyle, ProPalette, pro_app_tile, pro_btn, pro_check_draw, pro_connector_pulse,
        pro_ease, pro_eid, pro_empty_state, pro_icon_btn, pro_mix_color, pro_presence,
        pro_rolling_number, pro_skeleton_bar, pro_spring_f32, pro_swap_label, pro_tactile,
        pro_toast_banner, pro_view_transition, tr,
    },
    foundation::Theme,
    primitives::{
        family::disabled_motion,
        pro_icons::{
            AGENT_WAVE_MS, ProIcon, agent_pixel_cells, pro_app_artwork, pro_icon, pro_svg,
        },
    },
};
use gpui::{prelude::*, *};
use std::{
    cell::Cell,
    collections::{HashMap, HashSet},
    rc::Rc,
};
use web_time::Instant;

const VERCEL_MARK_SVG: &str = r#"<path d="M12 3.5 22 20.5H2z" fill="currentColor"></path>"#;
const CROSS_TINY_SVG: &str = r#"<path d="M7 7l10 10M17 7 7 17" stroke="currentColor" stroke-width="2.2" stroke-linecap="round"></path>"#;
const GATE_ASK_SVG: &str = r#"<circle cx="12" cy="12" r="2.4" fill="currentColor"></circle>"#;

const LANE_H: f32 = 60.0;
const TASK_H: f32 = 40.0;
const GATE_HIT: f32 = 26.0;
const EVENT_SIDE: f32 = 18.0;

/// Participant category determining how a [`SwimLane`] mark is rendered.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SwimLaneKind {
    /// Human owner or reviewer rendered with a solid 3x3 pixel block in ink.
    Person,
    /// Autonomous agent rendered with its deterministic 3x3 mirrored pixel mark.
    #[default]
    Agent,
    /// External tool or deployment target rendered with its provider mark.
    Tool,
}

/// Accent color preset for a [`SwimLane`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SwimLaneTone {
    /// Primary blue accent (default for planning/orchestration agents).
    #[default]
    Blue,
    /// Purple accent (coding/synthesis agents).
    Purple,
    /// Cyan accent (writing/documentation agents).
    Cyan,
    /// Green accent (verification/review agents).
    Green,
    /// Amber accent (human gates or caution lanes).
    Amber,
    /// Neutral ink/muted tone (people or external tools).
    Neutral,
}

impl SwimLaneTone {
    fn resolve(self, kind: SwimLaneKind, p: &ProPalette) -> Hsla {
        match kind {
            SwimLaneKind::Person => p.ink,
            SwimLaneKind::Tool => p.muted,
            SwimLaneKind::Agent => match self {
                Self::Blue => {
                    if p.dark {
                        hsla(218. / 360., 1.0, 0.65, 0.88)
                    } else {
                        hsla(214. / 360., 1.0, 0.45, 0.82)
                    }
                }
                Self::Purple => p.purple.opacity(0.88),
                Self::Cyan => p.cyan.opacity(0.88),
                Self::Green => p.green.opacity(0.92),
                Self::Amber => p.amber.opacity(0.90),
                Self::Neutral => p.ink,
            },
        }
    }
}

/// Participant row in [`HandoffSwimlanes`].
#[derive(Clone, Debug, PartialEq)]
pub struct SwimLane {
    /// Stable lane identifier referenced by [`SwimTask::lane`].
    pub id: SharedString,
    /// Participant display name (`"You"`, `"Planner"`, `"Vercel"`).
    pub name: SharedString,
    /// Participant category (`Person`, `Agent`, or `Tool`).
    pub kind: SwimLaneKind,
    /// Short responsibility caption shown beneath `name`.
    pub role: Option<SharedString>,
    /// Optional tool/provider identifier for [`SwimLaneKind::Tool`] logos.
    pub logo: Option<SharedString>,
    /// Semantic color preset for agent marks, washes, and handoff pulses.
    pub tone: SwimLaneTone,
    /// Optional custom `Hsla` override for this lane.
    pub custom_color: Option<Hsla>,
    /// Extended description displayed inside the lane's detail card.
    pub description: Option<SharedString>,
}

impl SwimLane {
    /// Construct a new [`SwimLane`] (defaults to [`SwimLaneKind::Agent`]).
    pub fn new(id: impl Into<SharedString>, name: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            kind: SwimLaneKind::Agent,
            role: None,
            logo: None,
            tone: SwimLaneTone::Blue,
            custom_color: None,
            description: None,
        }
    }

    /// Set the participant category (`Person`, `Agent`, or `Tool`).
    pub fn with_kind(mut self, kind: SwimLaneKind) -> Self {
        self.kind = kind;
        self
    }

    /// Set the secondary role label under the participant name.
    pub fn with_role(mut self, role: impl Into<SharedString>) -> Self {
        self.role = Some(role.into());
        self
    }

    /// Set the provider name used to look up [`SwimLaneKind::Tool`] artwork.
    pub fn with_logo(mut self, logo: impl Into<SharedString>) -> Self {
        self.logo = Some(logo.into());
        self
    }

    /// Set the semantic accent preset for this lane.
    pub fn with_tone(mut self, tone: SwimLaneTone) -> Self {
        self.tone = tone;
        self
    }

    /// Override the lane color with a specific [`Hsla`].
    pub fn with_color(mut self, color: Hsla) -> Self {
        self.custom_color = Some(color);
        self
    }

    /// Provide a longer summary shown when clicking the lane header.
    pub fn with_description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    fn color(&self, p: &ProPalette) -> Hsla {
        self.custom_color
            .unwrap_or_else(|| self.tone.resolve(self.kind, p))
    }
}

/// Visual node shape in [`HandoffSwimlanes`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SwimTaskKind {
    /// Rectangular work step box (`40px` tall).
    #[default]
    Task,
    /// Rotated diamond human approval gate (`PathBuilder`).
    Gate,
    /// Thin-ring BPMN start event circle (`18px`).
    Start,
    /// Thick-ring BPMN end event circle (`18px`).
    End,
}

/// Execution state of a [`SwimTask`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SwimTaskStatus {
    /// Waiting on upstream steps to finish.
    #[default]
    Queued,
    /// Currently executing (shimmers in lane color with twinkling agent mark).
    Active,
    /// Blocked waiting on human approval or input (amber highlight + breathing gate).
    Waiting,
    /// Completed cleanly.
    Done,
    /// Halted with an error or rejected at a gate (red highlight).
    Failed,
    /// Bypassed after an upstream rejection or branch skip.
    Skipped,
}

impl SwimTaskStatus {
    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::Queued => tr(cx, "Queued", "排队中"),
            Self::Active => tr(cx, "Running", "运行中"),
            Self::Waiting => tr(cx, "Waiting for you", "待你确认"),
            Self::Done => tr(cx, "Done", "已完成"),
            Self::Failed => tr(cx, "Failed", "失败"),
            Self::Skipped => tr(cx, "Skipped", "已跳过"),
        }
    }
}

/// Single step, approval gate, or start/end marker in [`HandoffSwimlanes`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SwimTask {
    /// Unique step identifier.
    pub id: SharedString,
    /// Owning [`SwimLane::id`].
    pub lane: SharedString,
    /// Concise step title.
    pub title: SharedString,
    /// Execution status.
    pub status: SwimTaskStatus,
    /// Node shape (`Task`, `Gate`, `Start`, or `End`).
    pub kind: SwimTaskKind,
    /// Upstream task ids that hand off into this node (`None` = previous task in list).
    pub after: Option<Vec<SharedString>>,
    /// Optional elapsed or completed duration (`"14s"`).
    pub duration: Option<SharedString>,
    /// Extended explanation shown inside the node's detail popover card.
    pub detail: Option<SharedString>,
}

impl SwimTask {
    /// Construct a new [`SwimTask`] in `lane` with `Queued` status and `Task` kind.
    pub fn new(
        id: impl Into<SharedString>,
        lane: impl Into<SharedString>,
        title: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            lane: lane.into(),
            title: title.into(),
            status: SwimTaskStatus::Queued,
            kind: SwimTaskKind::Task,
            after: None,
            duration: None,
            detail: None,
        }
    }

    /// Set the execution status.
    pub fn with_status(mut self, status: SwimTaskStatus) -> Self {
        self.status = status;
        self
    }

    /// Set the node shape (`Task`, `Gate`, `Start`, `End`).
    pub fn with_kind(mut self, kind: SwimTaskKind) -> Self {
        self.kind = kind;
        self
    }

    /// Specify explicit upstream task ids (`[]` starts an independent root branch).
    pub fn with_after<I, S>(mut self, after: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        self.after = Some(after.into_iter().map(Into::into).collect());
        self
    }

    /// Attach a tabular duration label such as `"14s"`.
    pub fn with_duration(mut self, duration: impl Into<SharedString>) -> Self {
        self.duration = Some(duration.into());
        self
    }

    /// Attach extended context displayed when the node's card opens.
    pub fn with_detail(mut self, detail: impl Into<SharedString>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}

/// Active detail card target inside [`HandoffSwimlanes`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SwimSelection {
    /// A task box, gate diamond, or start/end event circle is open.
    Task(SharedString),
    /// A participant lane header card is open.
    Lane(SharedString),
}

/// Strongly typed events emitted by [`HandoffSwimlanes`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HandoffSwimlanesEvent {
    /// Emitted when a task, gate, or lane detail card opens or closes.
    Select(Option<SwimSelection>),
    /// Emitted when the user approves a waiting gate or task card.
    Approve(SharedString),
    /// Emitted when the user rejects a waiting gate or task card.
    Reject(SharedString),
    /// Emitted when an active step completes or a failed step is retried.
    Advance(SharedString),
}

impl EventEmitter<HandoffSwimlanesEvent> for HandoffSwimlanes {}

#[derive(Clone, Debug)]
struct PlacedNode {
    task_idx: usize,
    lane_idx: usize,
    col: usize,
    preds: Vec<usize>,
    succs: Vec<usize>,
    x_left: f32,
    x_right: f32,
    x_center: f32,
    y_center: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EdgeVisualState {
    Done,
    Live,
    Next,
}

#[derive(Clone, Debug)]
struct SwimGraph {
    nodes: Vec<PlacedNode>,
    by_id: HashMap<SharedString, usize>,
    col_count: usize,
}

fn build_swim_graph(lanes: &[SwimLane], tasks: &[SwimTask]) -> SwimGraph {
    let mut lane_lookup: HashMap<SharedString, usize> = HashMap::new();
    for (idx, lane) in lanes.iter().enumerate() {
        lane_lookup.entry(lane.id.clone()).or_insert(idx);
    }

    let mut valid_indices: Vec<usize> = Vec::new();
    let mut id_to_node: HashMap<SharedString, usize> = HashMap::new();
    for (t_idx, task) in tasks.iter().enumerate() {
        if !lane_lookup.contains_key(&task.lane) || id_to_node.contains_key(&task.id) {
            continue;
        }
        let n_idx = valid_indices.len();
        valid_indices.push(t_idx);
        id_to_node.insert(task.id.clone(), n_idx);
    }

    let n = valid_indices.len();
    let mut raw_preds: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (n_idx, &t_idx) in valid_indices.iter().enumerate() {
        let task = &tasks[t_idx];
        let candidates: Vec<usize> = match &task.after {
            Some(explicit) => explicit
                .iter()
                .filter_map(|pid| id_to_node.get(pid).copied())
                .filter(|&p| p != n_idx)
                .collect(),
            None => {
                if n_idx > 0 {
                    vec![n_idx - 1]
                } else {
                    Vec::new()
                }
            }
        };
        let mut seen = HashSet::new();
        for p in candidates {
            if seen.insert(p) {
                raw_preds[n_idx].push(p);
            }
        }
    }

    // Cut cycles cleanly while computing longest-path column indices.
    let mut cols = vec![0usize; n];
    let mut clean_preds: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut state = vec![0u8; n]; // 0 = unvisited, 1 = visiting, 2 = done

    fn visit(
        u: usize,
        raw_preds: &[Vec<usize>],
        clean_preds: &mut [Vec<usize>],
        cols: &mut [usize],
        state: &mut [u8],
    ) -> usize {
        if state[u] == 2 {
            return cols[u];
        }
        state[u] = 1;
        let mut max_col = 0usize;
        let mut kept = Vec::new();
        for &p in &raw_preds[u] {
            if state[p] == 1 {
                // Cut cycle back-edge.
                continue;
            }
            let pc = visit(p, raw_preds, clean_preds, cols, state);
            max_col = max_col.max(pc + 1);
            kept.push(p);
        }
        clean_preds[u] = kept;
        cols[u] = max_col;
        state[u] = 2;
        max_col
    }

    let mut max_col_overall = 0usize;
    for u in 0..n {
        let c = visit(u, &raw_preds, &mut clean_preds, &mut cols, &mut state);
        max_col_overall = max_col_overall.max(c);
    }

    let mut succs: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (u, preds) in clean_preds.iter().enumerate() {
        for &p in preds {
            succs[p].push(u);
        }
    }

    let nodes = valid_indices
        .iter()
        .enumerate()
        .map(|(n_idx, &t_idx)| {
            let lane_idx = lane_lookup.get(&tasks[t_idx].lane).copied().unwrap_or(0);
            PlacedNode {
                task_idx: t_idx,
                lane_idx,
                col: cols[n_idx],
                preds: clean_preds[n_idx].clone(),
                succs: succs[n_idx].clone(),
                x_left: 0.0,
                x_right: 0.0,
                x_center: 0.0,
                y_center: lane_idx as f32 * LANE_H + LANE_H * 0.5,
            }
        })
        .collect();

    SwimGraph {
        nodes,
        by_id: id_to_node,
        col_count: if n == 0 { 0 } else { max_col_overall + 1 },
    }
}

fn collect_swim_chain(graph: &SwimGraph, root_idx: usize) -> HashSet<usize> {
    let mut out = HashSet::new();
    out.insert(root_idx);
    let mut up_stack = vec![root_idx];
    while let Some(u) = up_stack.pop() {
        for &p in &graph.nodes[u].preds {
            if out.insert(p) {
                up_stack.push(p);
            }
        }
    }
    let mut dn_stack = vec![root_idx];
    while let Some(u) = dn_stack.pop() {
        for &s in &graph.nodes[u].succs {
            if out.insert(s) {
                dn_stack.push(s);
            }
        }
    }
    out
}

fn layout_swim_columns(graph: &mut SwimGraph, tasks: &[SwimTask], canvas_w: f32, label_w: f32) {
    if graph.nodes.is_empty() || graph.col_count == 0 {
        return;
    }
    let avail = (canvas_w - label_w - 10.0).max(160.0);
    let mut col_kinds = vec![SwimTaskKind::Start; graph.col_count];
    for node in &graph.nodes {
        let k = tasks[node.task_idx].kind;
        let cur = col_kinds[node.col];
        let rank = |kind: SwimTaskKind| match kind {
            SwimTaskKind::Start | SwimTaskKind::End => 0,
            SwimTaskKind::Gate => 1,
            SwimTaskKind::Task => 2,
        };
        if rank(k) > rank(cur) {
            col_kinds[node.col] = k;
        }
    }

    let task_cols = col_kinds
        .iter()
        .filter(|&&k| k == SwimTaskKind::Task)
        .count()
        .max(1) as f32;
    let fixed_non_task: f32 = col_kinds
        .iter()
        .map(|&k| match k {
            SwimTaskKind::Start | SwimTaskKind::End => EVENT_SIDE,
            SwimTaskKind::Gate => GATE_HIT,
            SwimTaskKind::Task => 0.0,
        })
        .sum();
    let gaps = graph.col_count.saturating_sub(1) as f32;

    // Adapt gap first, then task box width so the swimlane diagram never scrolls sideways.
    let mut gap_x = 24.0_f32;
    let mut task_w = 98.0_f32;
    if fixed_non_task + task_cols * task_w + gaps * gap_x > avail {
        gap_x = ((avail - fixed_non_task - task_cols * task_w) / gaps.max(1.0)).clamp(12.0, 24.0);
    }
    if fixed_non_task + task_cols * task_w + gaps * gap_x > avail {
        task_w = ((avail - fixed_non_task - gaps * gap_x) / task_cols).clamp(54.0, 98.0);
    } else if gaps > 0.0 {
        let spare = avail - (fixed_non_task + task_cols * task_w + gaps * gap_x);
        if spare > 0.0 {
            let bonus_w = (spare * 0.35 / task_cols).clamp(0.0, 18.0);
            task_w += bonus_w;
            let rem = (avail - (fixed_non_task + task_cols * task_w)) / gaps;
            gap_x = rem.clamp(12.0, 46.0);
        }
    }

    let mut col_x = vec![0.0_f32; graph.col_count];
    let mut col_w = vec![0.0_f32; graph.col_count];
    let mut cursor = label_w + 8.0;
    for c in 0..graph.col_count {
        let w = match col_kinds[c] {
            SwimTaskKind::Start | SwimTaskKind::End => EVENT_SIDE,
            SwimTaskKind::Gate => GATE_HIT,
            SwimTaskKind::Task => task_w,
        };
        col_x[c] = cursor;
        col_w[c] = w;
        cursor += w + gap_x;
    }

    for node in &mut graph.nodes {
        let k = tasks[node.task_idx].kind;
        let cw = col_w[node.col];
        let nw = match k {
            SwimTaskKind::Start | SwimTaskKind::End => EVENT_SIDE,
            SwimTaskKind::Gate => GATE_HIT,
            SwimTaskKind::Task => task_w,
        };
        let cx_pos = col_x[node.col] + cw * 0.5;
        node.x_center = cx_pos;
        node.x_left = cx_pos - nw * 0.5;
        node.x_right = cx_pos + nw * 0.5;
    }
}

fn default_swim_lanes(cx: &App) -> Vec<SwimLane> {
    vec![
        SwimLane::new("you", tr(cx, "You", "你"))
            .with_kind(SwimLaneKind::Person)
            .with_role(tr(cx, "Owner", "负责人"))
            .with_description(tr(
                cx,
                "Starts the run, reviews release risk at human gates, and signs off production deploys.",
                "发起任务运行，在人工审批门把控发布风险并确认生产环境上线。",
            )),
        SwimLane::new("planner", tr(cx, "Planner", "规划智能体"))
            .with_tone(SwimLaneTone::Blue)
            .with_role(tr(cx, "Splits the work", "拆解任务"))
            .with_description(tr(
                cx,
                "Breaks incoming requests into parallel code, test, and changelog workstreams.",
                "将需求拆解为并行的代码修复、测试验证与更新日志分支。",
            )),
        SwimLane::new("coder", tr(cx, "Coder", "编码智能体"))
            .with_tone(SwimLaneTone::Purple)
            .with_role(tr(cx, "Writes code", "编写代码"))
            .with_description(tr(
                cx,
                "Patches billing rounding logic, updates unit tests, and opens pull requests.",
                "修复账单舍入逻辑、补充回归测试并提交拉取请求。",
            )),
        SwimLane::new("writer", tr(cx, "Writer", "文档智能体"))
            .with_tone(SwimLaneTone::Cyan)
            .with_role(tr(cx, "Docs & changelog", "文档与日志"))
            .with_description(tr(
                cx,
                "Drafts customer-facing changelog entries and migration notes alongside code changes.",
                "同步撰写面向用户的版本更新说明与接口变更记录。",
            )),
        SwimLane::new("reviewer", tr(cx, "Reviewer", "审查智能体"))
            .with_tone(SwimLaneTone::Green)
            .with_role(tr(cx, "Reviews PRs", "审查 PR"))
            .with_description(tr(
                cx,
                "Verifies invoice math edge cases, tax totals, and test coverage before human sign-off.",
                "在人工审批前核验发票精度边界、税额汇总与测试覆盖率。",
            )),
        SwimLane::new("vercel", "Vercel")
            .with_kind(SwimLaneKind::Tool)
            .with_logo("vercel")
            .with_role(tr(cx, "Deploys", "生产部署"))
            .with_description(tr(
                cx,
                "Builds and promotes the approved release artifact to production edge regions.",
                "在人工放行后构建并将制品发布至生产边缘节点。",
            )),
    ]
}

fn default_swim_tasks(cx: &App) -> Vec<SwimTask> {
    vec![
        SwimTask::new(
            "start",
            "you",
            tr(cx, "Ship the billing fix", "发布账单精度修复"),
        )
        .with_kind(SwimTaskKind::Start)
        .with_status(SwimTaskStatus::Done)
        .with_after(std::iter::empty::<&str>())
        .with_detail(tr(
            cx,
            "Triggered from #billing-ops after multi-currency cent rounding drifted by $0.01.",
            "源于多币种分位舍入偏差 $0.01 的线上修复请求。",
        )),
        SwimTask::new("plan", "planner", tr(cx, "Split into tasks", "拆解并行子任务"))
            .with_status(SwimTaskStatus::Done)
            .with_duration("14s")
            .with_after(["start"])
            .with_detail(tr(
                cx,
                "Split into parallel invoice patch and changelog tracks, converging at deploy approval.",
                "拆分为账单补丁与更新日志两条并行轨道，并在部署审批门汇合。",
            )),
        SwimTask::new(
            "fix",
            "coder",
            tr(cx, "Fix invoice rounding", "修复发票分位舍入"),
        )
        .with_status(SwimTaskStatus::Done)
        .with_duration("19s")
        .with_after(["plan"])
        .with_detail(tr(
            cx,
            "Switched line-item tax aggregation to banker's rounding and added 12 currency fixtures.",
            "将税额聚合切换为银行家舍入法并新增 12 组多币种回归用例。",
        )),
        SwimTask::new(
            "docs",
            "writer",
            tr(cx, "Draft the changelog", "撰写更新日志"),
        )
        .with_status(SwimTaskStatus::Active)
        .with_duration("9s")
        .with_after(["plan"])
        .with_detail(tr(
            cx,
            "Summarizing the invoice rounding fix for the October billing release notes.",
            "正在整理十月账单版本说明中的舍入修复摘要。",
        )),
        SwimTask::new("review", "reviewer", tr(cx, "Review PR #482", "审查 PR #482"))
            .with_status(SwimTaskStatus::Done)
            .with_duration("11s")
            .with_after(["fix"])
            .with_detail(tr(
                cx,
                "All 148 billing checks green; zero regressions across USD, EUR, and JPY invoices.",
                "全部 148 项账单测试通过，USD、EUR 与 JPY 发票零回归。",
            )),
        SwimTask::new("approve", "you", tr(cx, "Approve deploy", "批准上线"))
            .with_kind(SwimTaskKind::Gate)
            .with_status(SwimTaskStatus::Waiting)
            .with_after(["review"])
            .with_detail(tr(
                cx,
                "PR #482 passed automated review. Approve to promote build #914 to Vercel production.",
                "PR #482 已通过自动审查。批准后将构建 #914 发布至 Vercel 生产环境。",
            )),
        SwimTask::new(
            "deploy",
            "vercel",
            tr(cx, "Deploy to production", "部署至生产环境"),
        )
        .with_status(SwimTaskStatus::Queued)
        .with_after(["approve", "docs"])
        .with_detail(tr(
            cx,
            "Promotes build #914 across global regions and verifies health checks.",
            "在审批通过与日志就绪后将构建 #914 推送至生产集群。",
        )),
        SwimTask::new("end", "you", tr(cx, "Shipped", "已上线"))
            .with_kind(SwimTaskKind::End)
            .with_status(SwimTaskStatus::Queued)
            .with_after(["deploy"])
            .with_detail(tr(
                cx,
                "Billing hotfix is live in production and release notes are published.",
                "账单热修复已在生产环境生效并同步发布版本说明。",
            )),
    ]
}

#[allow(clippy::too_many_arguments)]
fn render_pixel_mark(
    seed: &str,
    person: bool,
    color: Hsla,
    side: f32,
    sq: f32,
    gap: f32,
    busy: bool,
    elapsed_ms: f32,
    reduced: bool,
) -> AnyElement {
    let cells = if person {
        [true; 9]
    } else {
        agent_pixel_cells(seed)
    };
    let mut col_el = div()
        .size(px(side))
        .flex_none()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(gap));
    for r in 0..3usize {
        let mut row_el = div().flex().items_center().gap(px(gap));
        for c in 0..3usize {
            let i = r * 3 + c;
            let on = cells[i];
            let alpha = if busy && !reduced {
                let phase = ((elapsed_ms + AGENT_WAVE_MS[i]) / 1200.0) * std::f32::consts::TAU;
                let wave = 0.60 + 0.40 * phase.cos();
                if on { wave } else { 0.16 * wave }
            } else if on {
                1.0
            } else {
                0.0
            };
            row_el = row_el.child(
                div()
                    .size(px(sq))
                    .rounded(px((sq * 0.28).max(0.75)))
                    .bg(color.opacity(alpha)),
            );
        }
        col_el = col_el.child(row_el);
    }
    col_el.into_any_element()
}

fn render_lane_mark(
    lane: &SwimLane,
    side: f32,
    busy: bool,
    elapsed_ms: f32,
    p: &ProPalette,
    cx: &App,
) -> AnyElement {
    let reduced = Theme::global(cx).reduced_motion;
    let color = lane.color(p);
    let (sq, gap) = if side <= 13.0 {
        (2.5, 1.0)
    } else if side <= 15.0 {
        (2.5, 1.25)
    } else {
        (3.0, 1.5)
    };
    match lane.kind {
        SwimLaneKind::Person => render_pixel_mark(
            &lane.id, true, color, side, sq, gap, false, elapsed_ms, reduced,
        ),
        SwimLaneKind::Agent => render_pixel_mark(
            &lane.id, false, color, side, sq, gap, busy, elapsed_ms, reduced,
        ),
        SwimLaneKind::Tool => {
            let key = lane.logo.as_deref().unwrap_or(lane.name.as_ref());
            let icon_sz = (side - 2.0).max(11.0);
            if key.eq_ignore_ascii_case("vercel") {
                div()
                    .size(px(side))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(pro_svg("0 0 24 24", VERCEL_MARK_SVG, icon_sz, p.ink))
                    .into_any_element()
            } else if let Some(art) = pro_app_artwork(key, icon_sz, cx) {
                div()
                    .size(px(side))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(art)
                    .into_any_element()
            } else {
                pro_app_tile(key, side, cx)
            }
        }
    }
}

fn paint_dashed_connector(
    p0: Point<Pixels>,
    p1: Point<Pixels>,
    radius: f32,
    color: Hsla,
    stroke_w: f32,
    window: &mut Window,
) {
    let x0 = f32::from(p0.x);
    let y0 = f32::from(p0.y);
    let x1 = f32::from(p1.x);
    let y1 = f32::from(p1.y);
    let xm = (x0 + x1) * 0.5;
    let dy = y1 - y0;
    let dx1 = xm - x0;
    let dx2 = x1 - xm;
    let r = radius
        .min(dy.abs() * 0.5)
        .min(dx1.abs())
        .min(dx2.abs())
        .max(0.0);

    let mut pts: Vec<(f32, f32)> = Vec::with_capacity(32);
    if r < 1.0 || dy.abs() < 1.0 {
        for i in 0..=24 {
            let t = i as f32 / 24.0;
            pts.push((x0 + (x1 - x0) * t, y0 + dy * t));
        }
    } else {
        let sy = dy.signum();
        let sx1 = dx1.signum();
        let sx2 = dx2.signum();
        let b = (xm - sx1 * r, y0);
        let c1 = (xm, y0);
        let d = (xm, y0 + sy * r);
        let e = (xm, y1 - sy * r);
        let c2 = (xm, y1);
        let f = (xm + sx2 * r, y1);
        pts.push((x0, y0));
        pts.push(b);
        for k in 1..=6 {
            let u = k as f32 / 6.0;
            let inv = 1.0 - u;
            pts.push((
                inv * inv * b.0 + 2.0 * inv * u * c1.0 + u * u * d.0,
                inv * inv * b.1 + 2.0 * inv * u * c1.1 + u * u * d.1,
            ));
        }
        pts.push(e);
        for k in 1..=6 {
            let u = k as f32 / 6.0;
            let inv = 1.0 - u;
            pts.push((
                inv * inv * e.0 + 2.0 * inv * u * c2.0 + u * u * f.0,
                inv * inv * e.1 + 2.0 * inv * u * c2.1 + u * u * f.1,
            ));
        }
        pts.push((x1, y1));
    }

    // Walk polyline length and emit crisp 4px dashes / 4.5px gaps.
    let dash_len = 4.0_f32;
    let gap_len = 4.5_f32;
    let cycle = dash_len + gap_len;
    let mut dist_acc = 0.0_f32;
    for pair in pts.windows(2) {
        let (ax, ay) = pair[0];
        let (bx, by) = pair[1];
        let seg_len = ((bx - ax).powi(2) + (by - ay).powi(2)).sqrt();
        if seg_len < 0.25 {
            continue;
        }
        let mut s = 0.0_f32;
        while s < seg_len {
            let phase = (dist_acc + s) % cycle;
            if phase < dash_len {
                let take = (dash_len - phase).min(seg_len - s);
                let t0 = s / seg_len;
                let t1 = (s + take) / seg_len;
                let mut seg = PathBuilder::stroke(px(stroke_w));
                seg.move_to(point(px(ax + (bx - ax) * t0), px(ay + (by - ay) * t0)));
                seg.line_to(point(px(ax + (bx - ax) * t1), px(ay + (by - ay) * t1)));
                if let Ok(path) = seg.build() {
                    window.paint_path(path, color);
                }
                s += take;
            } else {
                s += (cycle - phase).min(seg_len - s).max(0.5);
            }
        }
        dist_acc += seg_len;
    }
}

fn paint_arrowhead(tip: Point<Pixels>, color: Hsla, window: &mut Window) {
    let mut head = PathBuilder::fill();
    head.move_to(point(tip.x - px(6.5), tip.y - px(3.8)));
    head.line_to(tip);
    head.line_to(point(tip.x - px(6.5), tip.y + px(3.8)));
    head.close();
    if let Ok(path) = head.build() {
        window.paint_path(path, color);
    }
}

fn paint_diamond(
    center: Point<Pixels>,
    half_diag: f32,
    fill_c: Hsla,
    stroke_c: Option<(f32, Hsla)>,
    window: &mut Window,
) {
    let r = 2.6_f32.min(half_diag * 0.28);
    let top = (f32::from(center.x), f32::from(center.y) - half_diag);
    let right = (f32::from(center.x) + half_diag, f32::from(center.y));
    let bottom = (f32::from(center.x), f32::from(center.y) + half_diag);
    let left = (f32::from(center.x) - half_diag, f32::from(center.y));
    let corners = [top, right, bottom, left];

    let build_points = || -> Vec<Point<Pixels>> {
        let mut pts = Vec::with_capacity(28);
        for i in 0..4 {
            let prev = corners[(i + 3) % 4];
            let cur = corners[i];
            let next = corners[(i + 1) % 4];
            let d_in = ((cur.0 - prev.0).hypot(cur.1 - prev.1)).max(1.0);
            let d_out = ((next.0 - cur.0).hypot(next.1 - cur.1)).max(1.0);
            let p_in = (
                cur.0 - (cur.0 - prev.0) / d_in * r,
                cur.1 - (cur.1 - prev.1) / d_in * r,
            );
            let p_out = (
                cur.0 + (next.0 - cur.0) / d_out * r,
                cur.1 + (next.1 - cur.1) / d_out * r,
            );
            pts.push(point(px(p_in.0), px(p_in.1)));
            for k in 1..=5 {
                let u = k as f32 / 5.0;
                let inv = 1.0 - u;
                pts.push(point(
                    px(inv * inv * p_in.0 + 2.0 * inv * u * cur.0 + u * u * p_out.0),
                    px(inv * inv * p_in.1 + 2.0 * inv * u * cur.1 + u * u * p_out.1),
                ));
            }
        }
        pts
    };

    let pts = build_points();
    if pts.len() < 3 {
        return;
    }
    let mut fill_b = PathBuilder::fill();
    for (i, &pt) in pts.iter().enumerate() {
        if i == 0 {
            fill_b.move_to(pt);
        } else {
            fill_b.line_to(pt);
        }
    }
    fill_b.close();
    if let Ok(path) = fill_b.build() {
        window.paint_path(path, fill_c);
    }

    if let Some((sw, sc)) = stroke_c {
        let mut stroke_b = PathBuilder::stroke(px(sw));
        for (i, &pt) in pts.iter().enumerate() {
            if i == 0 {
                stroke_b.move_to(pt);
            } else {
                stroke_b.line_to(pt);
            }
        }
        stroke_b.close();
        if let Ok(path) = stroke_b.build() {
            window.paint_path(path, sc);
        }
    }
}

/// Process swimlane diagram across people, agents, and tools with inline gate approvals.
pub struct HandoffSwimlanes {
    id: ElementId,
    title: Option<SharedString>,
    lanes: Vec<SwimLane>,
    tasks: Vec<SwimTask>,
    initial_lanes: Vec<SwimLane>,
    initial_tasks: Vec<SwimTask>,
    selected: Option<SwimSelection>,
    last_selected: Option<SwimSelection>,
    hovered_node: Option<SharedString>,
    hovered_lane: Option<SharedString>,
    allow_actions: bool,
    loading: bool,
    empty: bool,
    started_at: Instant,
    measured_w: Rc<Cell<f32>>,
    toast: Option<(SharedString, Instant)>,
    announcement: SharedString,
    focus: FocusHandle,
}

impl HandoffSwimlanes {
    /// Create a [`HandoffSwimlanes`] preloaded with the 6-lane `"Ship the billing fix"` workflow.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let lanes = default_swim_lanes(cx);
        let tasks = default_swim_tasks(cx);
        Self {
            id: id.into(),
            title: None,
            initial_lanes: lanes.clone(),
            initial_tasks: tasks.clone(),
            lanes,
            tasks,
            selected: None,
            last_selected: None,
            hovered_node: None,
            hovered_lane: None,
            allow_actions: true,
            loading: false,
            empty: false,
            started_at: Instant::now(),
            measured_w: Rc::new(Cell::new(766.0)),
            toast: None,
            announcement: SharedString::default(),
            focus: cx.focus_handle(),
        }
    }

    /// Override the diagram heading and accessible region label.
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Supply custom participant lanes from top to bottom.
    pub fn with_lanes(mut self, lanes: Vec<SwimLane>) -> Self {
        self.initial_lanes = lanes.clone();
        self.lanes = lanes;
        self
    }

    /// Supply custom tasks, gates, and start/end events.
    pub fn with_tasks(mut self, tasks: Vec<SwimTask>) -> Self {
        self.initial_tasks = tasks.clone();
        self.tasks = tasks;
        self
    }

    /// Pre-open a specific task or lane detail card.
    pub fn with_selected(mut self, selected: Option<SwimSelection>) -> Self {
        self.selected = selected;
        self
    }

    /// Enable or hide inline `Approve` / `Reject` buttons on waiting gate cards.
    pub fn with_approvals_enabled(mut self, enabled: bool) -> Self {
        self.allow_actions = enabled;
        self
    }

    /// Render synchronized skeleton placeholders while run topology loads.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Force the empty swimlane state (`"Nothing to hand over yet"`).
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    fn resolved_title(&self, cx: &App) -> SharedString {
        self.title
            .clone()
            .unwrap_or_else(|| tr(cx, "Ship the billing fix", "发布账单精度修复").into())
    }

    fn toggle_selection(&mut self, target: SwimSelection, cx: &mut Context<Self>) {
        if self.selected.as_ref() == Some(&target) {
            self.selected = None;
        } else {
            self.selected = Some(target);
        }
        cx.emit(HandoffSwimlanesEvent::Select(self.selected.clone()));
        cx.notify();
    }

    fn close_card(&mut self, cx: &mut Context<Self>) {
        if self.selected.take().is_some() {
            cx.emit(HandoffSwimlanesEvent::Select(None));
            cx.notify();
        }
    }

    fn promote_downstream(&mut self) {
        let graph = build_swim_graph(&self.lanes, &self.tasks);
        let mut changed = true;
        while changed {
            changed = false;
            for node in &graph.nodes {
                let status = self.tasks[node.task_idx].status;
                if status != SwimTaskStatus::Queued {
                    continue;
                }
                if node.preds.is_empty() {
                    continue;
                }
                let all_done = node
                    .preds
                    .iter()
                    .all(|&p| self.tasks[graph.nodes[p].task_idx].status == SwimTaskStatus::Done);
                if all_done {
                    let kind = self.tasks[node.task_idx].kind;
                    self.tasks[node.task_idx].status = match kind {
                        SwimTaskKind::Gate => SwimTaskStatus::Waiting,
                        SwimTaskKind::End | SwimTaskKind::Start => SwimTaskStatus::Done,
                        SwimTaskKind::Task => SwimTaskStatus::Active,
                    };
                    if matches!(kind, SwimTaskKind::End | SwimTaskKind::Start) {
                        changed = true;
                    }
                }
            }
        }
    }

    fn approve_task(&mut self, id: SharedString, cx: &mut Context<Self>) {
        let mut found = false;
        for t in &mut self.tasks {
            if t.id == id {
                t.status = SwimTaskStatus::Done;
                if t.duration.is_none() {
                    t.duration = Some("2s".into());
                }
                found = true;
            } else if t.status == SwimTaskStatus::Active && t.id.as_ref() == "docs" {
                // Allow parallel changelog draft to finish cleanly when deploy gate is approved.
                t.status = SwimTaskStatus::Done;
                t.duration = Some("12s".into());
            }
        }
        if !found {
            return;
        }
        self.promote_downstream();
        self.selected = None;
        let msg: SharedString = tr(cx, "Approved handoff gate", "已批准交接关卡").into();
        self.announcement = msg.clone();
        self.toast = Some((msg, Instant::now()));
        cx.emit(HandoffSwimlanesEvent::Approve(id));
        cx.emit(HandoffSwimlanesEvent::Select(None));
        cx.notify();
    }

    fn reject_task(&mut self, id: SharedString, cx: &mut Context<Self>) {
        let graph = build_swim_graph(&self.lanes, &self.tasks);
        let Some(&start_node) = graph.by_id.get(&id) else {
            return;
        };
        self.tasks[graph.nodes[start_node].task_idx].status = SwimTaskStatus::Failed;
        let mut stack = graph.nodes[start_node].succs.clone();
        let mut visited = HashSet::new();
        while let Some(u) = stack.pop() {
            if !visited.insert(u) {
                continue;
            }
            let st = &mut self.tasks[graph.nodes[u].task_idx].status;
            if *st == SwimTaskStatus::Queued {
                *st = SwimTaskStatus::Skipped;
            }
            for &nx in &graph.nodes[u].succs {
                stack.push(nx);
            }
        }
        self.selected = None;
        let msg: SharedString = tr(
            cx,
            "Gate rejected — downstream skipped",
            "已拒绝关卡并跳过后续步骤",
        )
        .into();
        self.announcement = msg.clone();
        self.toast = Some((msg, Instant::now()));
        cx.emit(HandoffSwimlanesEvent::Reject(id));
        cx.emit(HandoffSwimlanesEvent::Select(None));
        cx.notify();
    }

    fn complete_or_retry_task(&mut self, id: SharedString, cx: &mut Context<Self>) {
        let mut was_failed = false;
        for t in &mut self.tasks {
            if t.id == id {
                was_failed = t.status == SwimTaskStatus::Failed;
                if was_failed {
                    t.status = if t.kind == SwimTaskKind::Gate {
                        SwimTaskStatus::Waiting
                    } else {
                        SwimTaskStatus::Active
                    };
                } else {
                    t.status = SwimTaskStatus::Done;
                    if t.duration.is_none() {
                        t.duration = Some("15s".into());
                    }
                }
            } else if was_failed && t.status == SwimTaskStatus::Skipped {
                t.status = SwimTaskStatus::Queued;
            }
        }
        if !was_failed {
            self.promote_downstream();
        }
        self.selected = None;
        let msg: SharedString = if was_failed {
            tr(cx, "Step restarted", "已重新启动步骤").into()
        } else {
            tr(cx, "Step completed — handed off", "步骤已完成并移交下一步").into()
        };
        self.announcement = msg.clone();
        self.toast = Some((msg, Instant::now()));
        cx.emit(HandoffSwimlanesEvent::Advance(id));
        cx.emit(HandoffSwimlanesEvent::Select(None));
        cx.notify();
    }

    fn reset_workflow(&mut self, cx: &mut Context<Self>) {
        self.lanes = self.initial_lanes.clone();
        self.tasks = self.initial_tasks.clone();
        self.selected = None;
        let msg: SharedString = tr(cx, "Reset swimlane run", "已重置泳道流程").into();
        self.toast = Some((msg, Instant::now()));
        cx.notify();
    }

    fn overall_run_status(&self, graph: &SwimGraph) -> SwimTaskStatus {
        let mut has_waiting = false;
        let mut has_active = false;
        let mut has_queued = false;
        let mut count = 0usize;
        for node in &graph.nodes {
            let t = &self.tasks[node.task_idx];
            if matches!(t.kind, SwimTaskKind::Start | SwimTaskKind::End) {
                continue;
            }
            count += 1;
            match t.status {
                SwimTaskStatus::Failed => return SwimTaskStatus::Failed,
                SwimTaskStatus::Waiting => has_waiting = true,
                SwimTaskStatus::Active => has_active = true,
                SwimTaskStatus::Queued => has_queued = true,
                SwimTaskStatus::Done | SwimTaskStatus::Skipped => {}
            }
        }
        if count == 0 {
            SwimTaskStatus::Queued
        } else if has_waiting {
            SwimTaskStatus::Waiting
        } else if has_active {
            SwimTaskStatus::Active
        } else if has_queued {
            SwimTaskStatus::Queued
        } else {
            SwimTaskStatus::Done
        }
    }

    fn step_counts(&self, graph: &SwimGraph) -> (usize, usize) {
        let mut done = 0usize;
        let mut total = 0usize;
        for node in &graph.nodes {
            let t = &self.tasks[node.task_idx];
            if matches!(t.kind, SwimTaskKind::Start | SwimTaskKind::End) {
                continue;
            }
            total += 1;
            if t.status == SwimTaskStatus::Done {
                done += 1;
            }
        }
        (done, total)
    }

    fn handle_key_down(&mut self, ev: &KeyDownEvent, _w: &mut Window, cx: &mut Context<Self>) {
        if ev.keystroke.key == "escape" && self.selected.is_some() {
            self.close_card(cx);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_detail_card(
        &self,
        sel: &SwimSelection,
        graph: &SwimGraph,
        canvas_w: f32,
        total_h: f32,
        elapsed_ms: f32,
        amount: f32,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let p = ProPalette::get(cx);
        let card_w = (canvas_w - 16.0).clamp(200.0, 260.0);
        let card_bg: Hsla = if p.dark {
            rgb(0x232327).into()
        } else {
            rgb(0xffffff).into()
        };

        let status_chip = |key: &str, st: SwimTaskStatus| -> Div {
            let (bg, fg) = match st {
                SwimTaskStatus::Active => (p.accent_soft, p.blue),
                SwimTaskStatus::Waiting => (p.amber_soft, p.amber_ink),
                SwimTaskStatus::Done => (p.green_soft, p.green_ink),
                SwimTaskStatus::Failed => (p.red_soft, p.red_ink),
                SwimTaskStatus::Queued | SwimTaskStatus::Skipped => (p.chip, p.muted),
            };
            div()
                .h(px(18.0))
                .px(px(6.0))
                .rounded(px(5.0))
                .bg(bg)
                .text_size(px(11.0))
                .font_weight(FontWeight::MEDIUM)
                .text_color(fg)
                .flex()
                .items_center()
                .gap(px(3.5))
                .when(st == SwimTaskStatus::Done, |d| {
                    d.child(pro_check_draw(
                        pro_eid(&self.id, format!("sw-dchk-{key}")),
                        true,
                        12.0,
                        false,
                        fg,
                    ))
                })
                .child(st.label(cx))
        };

        match sel {
            SwimSelection::Task(tid) => {
                let Some(&n_idx) = graph.by_id.get(tid) else {
                    return div().into_any_element();
                };
                let node = &graph.nodes[n_idx];
                let task = &self.tasks[node.task_idx];
                let lane = &self.lanes[node.lane_idx];
                let lane_busy = self
                    .tasks
                    .iter()
                    .any(|t| t.lane == lane.id && t.status == SwimTaskStatus::Active);

                let left_x =
                    (node.x_center - card_w * 0.5).clamp(4.0, (canvas_w - card_w - 4.0).max(4.0));
                let open_above = node.y_center > total_h * 0.58 && total_h > 210.0;
                let slide_dy = if open_above {
                    4.0 * (1.0 - amount)
                } else {
                    -4.0 * (1.0 - amount)
                };

                let mut body = div()
                    .id(pro_eid(&self.id, format!("card-task-{tid}")))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .w(px(card_w))
                    .p(px(10.0))
                    .rounded(px(8.0))
                    .bg(card_bg)
                    .border_1()
                    .border_color(p.line_strong)
                    .shadow_lg()
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .opacity(amount)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .child(status_chip(tid.as_ref(), task.status))
                            .when_some(task.duration.clone(), |d, dur| {
                                d.child(div().text_size(px(11.5)).text_color(p.muted).child(dur))
                            })
                            .child(div().ml_auto().child(pro_icon_btn(
                                pro_eid(&self.id, format!("close-{tid}")),
                                tr(cx, "Close card", "关闭卡片"),
                                ProIcon::Close,
                                22.0,
                                false,
                                cx,
                                |this, _, cx| this.close_card(cx),
                            ))),
                    )
                    .child(
                        div()
                            .text_size(px(13.0))
                            .line_height(px(18.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .child(task.title.clone()),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .child(render_lane_mark(lane, 14.0, lane_busy, elapsed_ms, &p, cx))
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .child(lane.name.clone()),
                            )
                            .when_some(lane.role.clone(), |d, role| {
                                d.child(
                                    div()
                                        .text_size(px(11.5))
                                        .text_color(p.muted)
                                        .truncate()
                                        .child(role),
                                )
                            }),
                    );

                if let Some(detail) = &task.detail {
                    body = body.child(
                        div()
                            .text_size(px(12.0))
                            .line_height(px(16.5))
                            .text_color(p.text)
                            .child(detail.clone()),
                    );
                }

                if !node.preds.is_empty() || !node.succs.is_empty() {
                    let mut flow = div()
                        .pt(px(7.0))
                        .border_t_1()
                        .border_color(p.line)
                        .flex()
                        .flex_col()
                        .gap(px(5.0));

                    if !node.preds.is_empty() {
                        let mut from_items =
                            div().flex().flex_wrap().gap(px(6.0)).flex_1().min_w_0();
                        for &pred_idx in &node.preds {
                            let pn = &graph.nodes[pred_idx];
                            let pt = &self.tasks[pn.task_idx];
                            let pl = &self.lanes[pn.lane_idx];
                            from_items = from_items.child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(4.0))
                                    .child(render_lane_mark(pl, 12.0, false, elapsed_ms, &p, cx))
                                    .child(
                                        div()
                                            .text_size(px(11.5))
                                            .text_color(p.ink)
                                            .child(pt.title.clone()),
                                    ),
                            );
                        }
                        flow = flow.child(
                            div()
                                .flex()
                                .items_start()
                                .gap(px(6.0))
                                .child(
                                    div()
                                        .w(px(38.0))
                                        .flex_none()
                                        .text_size(px(11.0))
                                        .text_color(p.muted)
                                        .child(tr(cx, "From", "来自")),
                                )
                                .child(from_items),
                        );
                    }

                    if !node.succs.is_empty() {
                        let mut to_items = div().flex().flex_wrap().gap(px(6.0)).flex_1().min_w_0();
                        for &succ_idx in &node.succs {
                            let sn = &graph.nodes[succ_idx];
                            let st = &self.tasks[sn.task_idx];
                            let sl = &self.lanes[sn.lane_idx];
                            to_items = to_items.child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(4.0))
                                    .child(render_lane_mark(sl, 12.0, false, elapsed_ms, &p, cx))
                                    .child(
                                        div()
                                            .text_size(px(11.5))
                                            .text_color(p.ink)
                                            .child(st.title.clone()),
                                    ),
                            );
                        }
                        flow = flow.child(
                            div()
                                .flex()
                                .items_start()
                                .gap(px(6.0))
                                .child(
                                    div()
                                        .w(px(38.0))
                                        .flex_none()
                                        .text_size(px(11.0))
                                        .text_color(p.muted)
                                        .child(tr(cx, "To", "移交")),
                                )
                                .child(to_items),
                        );
                    }
                    body = body.child(flow);
                }

                if self.allow_actions {
                    let task_id = task.id.clone();
                    if task.status == SwimTaskStatus::Waiting {
                        let rej_id = task_id.clone();
                        let app_id = task_id;
                        body = body.child(
                            div()
                                .pt(px(8.0))
                                .border_t_1()
                                .border_color(p.line)
                                .flex()
                                .justify_end()
                                .gap(px(6.0))
                                .child(pro_btn(
                                    pro_eid(&self.id, format!("rej-{rej_id}")),
                                    tr(cx, "Reject", "拒绝"),
                                    None,
                                    ProBtnStyle::Secondary,
                                    true,
                                    cx,
                                    move |this, _, cx| this.reject_task(rej_id.clone(), cx),
                                ))
                                .child(pro_btn(
                                    pro_eid(&self.id, format!("app-{app_id}")),
                                    tr(cx, "Approve", "批准"),
                                    Some(ProIcon::Check),
                                    ProBtnStyle::Primary,
                                    true,
                                    cx,
                                    move |this, _, cx| this.approve_task(app_id.clone(), cx),
                                )),
                        );
                    } else if matches!(task.status, SwimTaskStatus::Active | SwimTaskStatus::Failed)
                    {
                        let act_id = task_id;
                        let is_fail = task.status == SwimTaskStatus::Failed;
                        body = body.child(
                            div()
                                .pt(px(8.0))
                                .border_t_1()
                                .border_color(p.line)
                                .flex()
                                .justify_end()
                                .child(pro_btn(
                                    pro_eid(&self.id, format!("step-{act_id}")),
                                    if is_fail {
                                        tr(cx, "Retry step", "重试步骤")
                                    } else {
                                        tr(cx, "Complete step", "完成此步")
                                    },
                                    Some(if is_fail {
                                        ProIcon::Refresh
                                    } else {
                                        ProIcon::Check
                                    }),
                                    ProBtnStyle::Secondary,
                                    true,
                                    cx,
                                    move |this, _, cx| {
                                        this.complete_or_retry_task(act_id.clone(), cx)
                                    },
                                )),
                        );
                    }
                }

                let mut pos_wrap = div().absolute().left(px(left_x));
                pos_wrap = if open_above {
                    let bottom_offset = (total_h - node.y_center + 24.0 + slide_dy).max(8.0);
                    pos_wrap.bottom(px(bottom_offset))
                } else {
                    let top_offset = (node.y_center + 24.0 + slide_dy).max(8.0);
                    pos_wrap.top(px(top_offset))
                };
                pos_wrap.child(body).into_any_element()
            }
            SwimSelection::Lane(lid) => {
                let Some((l_idx, lane)) = self.lanes.iter().enumerate().find(|(_, l)| &l.id == lid)
                else {
                    return div().into_any_element();
                };
                let lane_tasks: Vec<&SwimTask> =
                    self.tasks.iter().filter(|t| &t.lane == lid).collect();
                let done_count = lane_tasks
                    .iter()
                    .filter(|t| t.status == SwimTaskStatus::Done)
                    .count();
                let busy = lane_tasks
                    .iter()
                    .any(|t| t.status == SwimTaskStatus::Active);
                let y_mid = l_idx as f32 * LANE_H + LANE_H * 0.5;
                let open_above = y_mid > total_h * 0.62 && total_h > 200.0;
                let kind_lbl = match lane.kind {
                    SwimLaneKind::Person => tr(cx, "Person", "成员"),
                    SwimLaneKind::Agent => tr(cx, "Agent", "智能体"),
                    SwimLaneKind::Tool => tr(cx, "Tool", "工具"),
                };

                let mut chips = div().flex().flex_wrap().gap(px(4.0));
                for lt in &lane_tasks {
                    let (bg, fg) = match lt.status {
                        SwimTaskStatus::Active => (p.accent_soft, p.blue),
                        SwimTaskStatus::Waiting => (p.amber_soft, p.amber_ink),
                        SwimTaskStatus::Done => (p.green_soft, p.green_ink),
                        SwimTaskStatus::Failed => (p.red_soft, p.red_ink),
                        SwimTaskStatus::Queued | SwimTaskStatus::Skipped => (p.chip, p.text),
                    };
                    chips = chips.child(
                        div()
                            .px(px(6.0))
                            .py(px(1.5))
                            .rounded(px(5.0))
                            .bg(bg)
                            .text_size(px(11.0))
                            .text_color(fg)
                            .child(lt.title.clone()),
                    );
                }

                let card = div()
                    .id(pro_eid(&self.id, format!("card-lane-{lid}")))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .w(px(card_w))
                    .p(px(10.0))
                    .rounded(px(8.0))
                    .bg(card_bg)
                    .border_1()
                    .border_color(p.line_strong)
                    .shadow_lg()
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .opacity(amount)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .child(render_lane_mark(lane, 15.0, busy, elapsed_ms, &p, cx))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .text_size(px(13.0))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.strong)
                                    .truncate()
                                    .child(lane.name.clone()),
                            )
                            .child(pro_icon_btn(
                                pro_eid(&self.id, format!("close-lane-{lid}")),
                                tr(cx, "Close card", "关闭卡片"),
                                ProIcon::Close,
                                22.0,
                                false,
                                cx,
                                |this, _, cx| this.close_card(cx),
                            )),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .child(
                                div()
                                    .px(px(6.0))
                                    .py(px(1.0))
                                    .rounded(px(5.0))
                                    .bg(p.chip)
                                    .text_size(px(11.0))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.text)
                                    .child(kind_lbl),
                            )
                            .when_some(lane.role.clone(), |d, role| {
                                d.child(
                                    div()
                                        .text_size(px(12.0))
                                        .text_color(p.muted)
                                        .truncate()
                                        .child(role),
                                )
                            }),
                    )
                    .when_some(lane.description.clone(), |d, desc| {
                        d.child(
                            div()
                                .text_size(px(12.0))
                                .line_height(px(16.5))
                                .text_color(p.text)
                                .child(desc),
                        )
                    })
                    .child(
                        div()
                            .pt(px(7.0))
                            .border_t_1()
                            .border_color(p.line)
                            .flex()
                            .items_center()
                            .gap(px(16.0))
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .child(div().text_size(px(11.0)).text_color(p.muted).child(tr(
                                        cx,
                                        "Steps",
                                        "步骤数",
                                    )))
                                    .child(
                                        div()
                                            .text_size(px(12.5))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.ink)
                                            .child(format!("{}", lane_tasks.len())),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .child(div().text_size(px(11.0)).text_color(p.muted).child(tr(
                                        cx,
                                        "Completed",
                                        "已完成",
                                    )))
                                    .child(
                                        div()
                                            .text_size(px(12.5))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.ink)
                                            .child(format!("{done_count}/{}", lane_tasks.len())),
                                    ),
                            ),
                    )
                    .when(!lane_tasks.is_empty(), |d| d.child(chips));

                let mut pos_wrap = div().absolute().left(px(6.0));
                pos_wrap = if open_above {
                    pos_wrap.bottom(px((total_h - y_mid + 22.0).max(8.0)))
                } else {
                    pos_wrap.top(px((y_mid + 22.0).max(8.0)))
                };
                pos_wrap.child(card).into_any_element()
            }
        }
    }
}

impl Render for HandoffSwimlanes {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let reduced = disabled_motion(cx);
        let elapsed_secs = self.started_at.elapsed().as_secs_f32();
        let elapsed_ms = elapsed_secs * 1000.0;

        let card_surface: Hsla = if p.dark {
            rgb(0x1c1c1e).into()
        } else {
            rgb(0xffffff).into()
        };
        let box_surface: Hsla = if p.dark {
            rgb(0x2c2c2f).into()
        } else {
            rgb(0xffffff).into()
        };
        let edge_done: Hsla = if p.dark {
            hsla(0., 0., 1., 0.32)
        } else {
            hsla(0., 0., 0., 0.30)
        };
        let edge_next: Hsla = if p.dark {
            hsla(0., 0., 1., 0.16)
        } else {
            hsla(0., 0., 0., 0.16)
        };

        let title = self.resolved_title(cx);

        if self.loading {
            if !reduced {
                window.request_animation_frame();
            }
            let phase = (elapsed_secs / 1.4).fract();
            let mut skel_rows = div().w_full().flex().flex_col();
            for i in 0..5usize {
                skel_rows = skel_rows.child(
                    div()
                        .h(px(56.0))
                        .border_t_1()
                        .border_color(p.line)
                        .flex()
                        .items_center()
                        .gap(px(16.0))
                        .child(pro_skeleton_bar(
                            92.0,
                            14.0,
                            5.0,
                            phase + i as f32 * 0.08,
                            cx,
                        ))
                        .child(div().ml(px((i * 44) as f32)).child(pro_skeleton_bar(
                            96.0,
                            34.0,
                            8.0,
                            phase + i as f32 * 0.12,
                            cx,
                        ))),
                );
            }
            return div()
                .id(self.id.clone())
                .role(Role::Group)
                .aria_label(title.clone())
                .w_full()
                .p(px(16.0))
                .rounded(px(14.0))
                .bg(card_surface)
                .border_1()
                .border_color(p.line)
                .flex()
                .flex_col()
                .gap(px(12.0))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(pro_skeleton_bar(160.0, 16.0, 5.0, phase, cx))
                        .child(pro_skeleton_bar(90.0, 20.0, 5.0, phase + 0.2, cx)),
                )
                .child(skel_rows);
        }

        let mut graph = build_swim_graph(&self.lanes, &self.tasks);
        if self.empty || self.lanes.is_empty() || graph.nodes.is_empty() {
            return div()
                .id(self.id.clone())
                .role(Role::Group)
                .aria_label(title.clone())
                .w_full()
                .p(px(16.0))
                .rounded(px(14.0))
                .bg(card_surface)
                .border_1()
                .border_color(p.line)
                .flex()
                .flex_col()
                .gap(px(12.0))
                .child(
                    div()
                        .text_size(px(13.0))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.strong)
                        .child(title),
                )
                .child(pro_empty_state(
                    ProIcon::GitBranch,
                    tr(cx, "Nothing to hand over yet", "暂无泳道交接步骤"),
                    tr(
                        cx,
                        "Add participants and tasks to map how work moves across people, agents, and tools.",
                        "添加参与方与任务节点以展示人机与工具间的交接链路。",
                    ),
                    None,
                    cx,
                ));
        }

        let canvas_w = self.measured_w.get().max(260.0);
        let mark_only = canvas_w < 484.0;
        let label_w = if mark_only { 30.0 } else { 112.0 };
        layout_swim_columns(&mut graph, &self.tasks, canvas_w, label_w);

        let run_status = self.overall_run_status(&graph);
        let (done_count, total_count) = self.step_counts(&graph);
        let has_motion = matches!(run_status, SwimTaskStatus::Active | SwimTaskStatus::Waiting)
            || self.toast.is_some();
        if has_motion && !reduced {
            window.request_animation_frame();
        }

        let pulse_phase = ((elapsed_secs / 1.55).fract()).clamp(0.0, 1.0);
        let sheen_phase = ((elapsed_secs / 1.80).fract()).clamp(0.0, 1.0);
        let ask_breath = if reduced {
            0.0
        } else {
            0.5 + 0.5 * ((elapsed_secs / 2.0) * std::f32::consts::TAU).sin()
        };
        let status_dot_alpha = if reduced {
            1.0
        } else {
            0.45 + 0.55 * (0.5 + 0.5 * ((elapsed_secs / 1.6) * std::f32::consts::TAU).cos())
        };

        let (state_bg, state_fg, show_pulse_dot) = match run_status {
            SwimTaskStatus::Active => (p.accent_soft, p.blue, true),
            SwimTaskStatus::Waiting => (p.amber_soft, p.amber_ink, true),
            SwimTaskStatus::Done => (p.green_soft, p.green_ink, false),
            SwimTaskStatus::Failed => (p.red_soft, p.red_ink, false),
            SwimTaskStatus::Queued | SwimTaskStatus::Skipped => (p.chip, p.text, false),
        };

        if let Some(s) = &self.selected {
            self.last_selected = Some(s.clone());
        }

        let show_reset = matches!(run_status, SwimTaskStatus::Done | SwimTaskStatus::Failed);
        let steps_sfx = format!(" {} {total_count}", tr(cx, "of", "/"));
        let header = div()
            .w_full()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(10.0))
            .child(
                div()
                    .flex_1()
                    .min_w(px(120.0))
                    .text_size(px(13.0))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .truncate()
                    .child(title.clone()),
            )
            .child(
                div()
                    .ml_auto()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .when(show_reset, |d| {
                        d.child(
                            pro_btn(
                                pro_eid(&self.id, "reset-run"),
                                tr(cx, "Replay", "重放"),
                                Some(ProIcon::Refresh),
                                ProBtnStyle::Secondary,
                                true,
                                cx,
                                |this, _, cx| this.reset_workflow(cx),
                            )
                            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation()),
                        )
                    })
                    .child(pro_rolling_number(
                        pro_eid(&self.id, "swim-done-cnt"),
                        done_count as f64,
                        0,
                        "",
                        &steps_sfx,
                        12.0,
                        FontWeight::NORMAL,
                        p.muted,
                    ))
                    .child(
                        div()
                            .id(pro_eid(&self.id, "run-status"))
                            .role(Role::Status)
                            .h(px(20.0))
                            .px(px(7.0))
                            .rounded(px(5.0))
                            .bg(state_bg)
                            .flex()
                            .items_center()
                            .gap(px(5.0))
                            .when(show_pulse_dot, |d| {
                                d.child(
                                    div()
                                        .size(px(6.0))
                                        .rounded_full()
                                        .bg(state_fg.opacity(status_dot_alpha)),
                                )
                            })
                            .when(run_status == SwimTaskStatus::Done, |d| {
                                d.child(pro_check_draw(
                                    pro_eid(&self.id, "swim-hdr-chk"),
                                    true,
                                    12.0,
                                    false,
                                    state_fg,
                                ))
                            })
                            .child(pro_swap_label(
                                pro_eid(&self.id, "swim-run-lbl"),
                                run_status.label(cx),
                                12.0,
                                FontWeight::MEDIUM,
                                state_fg,
                            )),
                    ),
            );

        let total_h = self.lanes.len() as f32 * LANE_H;

        // Connected-chain spotlight across nodes and handoff connectors via pro_spring_f32!
        let focus_task_id = self.hovered_node.clone().or_else(|| match &self.selected {
            Some(SwimSelection::Task(tid)) => Some(tid.clone()),
            _ => None,
        });
        let focus_lane_id = self.hovered_lane.clone().or_else(|| match &self.selected {
            Some(SwimSelection::Lane(lid)) => Some(lid.clone()),
            _ => None,
        });
        let mut lit_node_indices: HashSet<usize> = HashSet::new();
        let has_spotlight = if let Some(ftid) = &focus_task_id
            && let Some(&n_idx) = graph.by_id.get(ftid)
        {
            lit_node_indices = collect_swim_chain(&graph, n_idx);
            true
        } else if let Some(flid) = &focus_lane_id {
            for (n_idx, node) in graph.nodes.iter().enumerate() {
                if &self.lanes[node.lane_idx].id == flid {
                    lit_node_indices.insert(n_idx);
                    for &p_idx in &node.preds {
                        lit_node_indices.insert(p_idx);
                    }
                    for &s_idx in &node.succs {
                        lit_node_indices.insert(s_idx);
                    }
                }
            }
            !lit_node_indices.is_empty()
        } else {
            false
        };

        let node_opacities: Vec<f32> = graph
            .nodes
            .iter()
            .enumerate()
            .map(|(n_idx, node)| {
                let tid = &self.tasks[node.task_idx].id;
                let target_op = if has_spotlight {
                    if lit_node_indices.contains(&n_idx) {
                        1.0_f32
                    } else {
                        0.30_f32
                    }
                } else {
                    1.0_f32
                };
                pro_spring_f32(
                    pro_eid(&self.id, format!("swim-nop-{tid}")),
                    target_op,
                    0.22,
                    window,
                    cx,
                )
            })
            .collect();

        // Precompute edges for canvas painting with spring spotlight!
        #[derive(Clone)]
        struct EdgeDraw {
            x0: f32,
            y0: f32,
            x1: f32,
            y1: f32,
            state: EdgeVisualState,
            color: Hsla,
            draw_dot: bool,
            vis: f32,
            spt: f32,
        }
        let mut edges_to_paint: Vec<EdgeDraw> = Vec::new();
        for (dst_idx, dst) in graph.nodes.iter().enumerate() {
            let dst_task = &self.tasks[dst.task_idx];
            let dst_lane = &self.lanes[dst.lane_idx];
            let dst_color = if dst_task.status == SwimTaskStatus::Waiting {
                p.amber
            } else {
                dst_lane.color(&p)
            };
            for &src_idx in &dst.preds {
                let src = &graph.nodes[src_idx];
                let src_task = &self.tasks[src.task_idx];
                let state = if src_task.status == SwimTaskStatus::Done
                    && matches!(
                        dst_task.status,
                        SwimTaskStatus::Active | SwimTaskStatus::Waiting
                    ) {
                    EdgeVisualState::Live
                } else if src_task.status == SwimTaskStatus::Done
                    && dst_task.status == SwimTaskStatus::Done
                {
                    EdgeVisualState::Done
                } else {
                    EdgeVisualState::Next
                };
                let on_chain = has_spotlight
                    && lit_node_indices.contains(&src_idx)
                    && lit_node_indices.contains(&dst_idx);
                let target_vis = if has_spotlight {
                    if on_chain { 1.0_f32 } else { 0.22_f32 }
                } else {
                    1.0_f32
                };
                let target_spt = if on_chain { 1.0_f32 } else { 0.0_f32 };
                let vis = pro_spring_f32(
                    pro_eid(&self.id, format!("swim-ev-{}-{}", src_task.id, dst_task.id)),
                    target_vis,
                    0.22,
                    window,
                    cx,
                );
                let spt = pro_spring_f32(
                    pro_eid(&self.id, format!("swim-es-{}-{}", src_task.id, dst_task.id)),
                    target_spt,
                    0.20,
                    window,
                    cx,
                );
                edges_to_paint.push(EdgeDraw {
                    x0: src.x_right,
                    y0: src.y_center,
                    x1: dst.x_left,
                    y1: dst.y_center,
                    state,
                    color: dst_color,
                    draw_dot: src_task.kind == SwimTaskKind::Task,
                    vis,
                    spt,
                });
            }
        }

        let measure_cell = self.measured_w.clone();
        let edge_layer = canvas(
            move |bounds, window, _| {
                let bw = f32::from(bounds.size.width);
                if bw > 40.0 && (measure_cell.get() - bw).abs() > 1.0 {
                    measure_cell.set(bw);
                    window.request_animation_frame();
                }
            },
            move |bounds, _, window, _| {
                let ox = bounds.left();
                let oy = bounds.top();
                for e in &edges_to_paint {
                    let p0 = point(ox + px(e.x0), oy + px(e.y0));
                    let p1_line = point(ox + px((e.x1 - 4.0).max(e.x0 + 2.0)), oy + px(e.y1));
                    let p1_tip = point(ox + px(e.x1), oy + px(e.y1));
                    let sw = 1.5 + 0.35 * e.spt;
                    match e.state {
                        EdgeVisualState::Live => {
                            let line_c = e.color.opacity(0.65 * e.vis);
                            pro_connector_pulse(
                                p0,
                                p1_line,
                                6.0,
                                false,
                                line_c,
                                e.color.opacity(e.vis),
                                if reduced { None } else { Some(pulse_phase) },
                                sw,
                                window,
                            );
                            paint_arrowhead(p1_tip, line_c, window);
                        }
                        EdgeVisualState::Done => {
                            let done_c = pro_mix_color(edge_done, e.color, 0.55 * e.spt)
                                .opacity((edge_done.a + 0.35 * e.spt).min(0.92) * e.vis);
                            pro_connector_pulse(
                                p0,
                                p1_line,
                                6.0,
                                false,
                                done_c,
                                e.color.opacity(e.vis),
                                (e.spt > 0.5 && !reduced).then_some(pulse_phase),
                                sw,
                                window,
                            );
                            paint_arrowhead(p1_tip, done_c, window);
                        }
                        EdgeVisualState::Next => {
                            let next_c = edge_next.opacity(edge_next.a * e.vis + 0.18 * e.spt);
                            paint_dashed_connector(p0, p1_line, 6.0, next_c, sw, window);
                            paint_arrowhead(p1_tip, next_c, window);
                        }
                    }

                    if e.draw_dot {
                        let stroke_c = match e.state {
                            EdgeVisualState::Live => e.color.opacity(0.75 * e.vis),
                            EdgeVisualState::Done => edge_done.opacity(edge_done.a * e.vis),
                            EdgeVisualState::Next => edge_next.opacity(edge_next.a * e.vis),
                        };
                        let dot_rect = Bounds::new(
                            point(p0.x - px(3.25), p0.y - px(3.25)),
                            size(px(6.5), px(6.5)),
                        );
                        window.paint_quad(quad(
                            dot_rect,
                            Corners::all(px(3.25)),
                            box_surface,
                            Edges::all(px(1.5)),
                            stroke_c,
                            BorderStyle::Solid,
                        ));
                    }
                }
            },
        )
        .absolute()
        .inset_0();

        // Build lane rows.
        let mut lanes_stack = div().w_full().flex().flex_col();
        for lane in &self.lanes {
            let lane_id = lane.id.clone();
            let lane_open = self.selected.as_ref() == Some(&SwimSelection::Lane(lane_id.clone()));
            let task_cnt = self.tasks.iter().filter(|t| t.lane == lane.id).count();
            let lane_busy = self
                .tasks
                .iter()
                .any(|t| t.lane == lane.id && t.status == SwimTaskStatus::Active);
            let aria = match &lane.role {
                Some(r) => format!("{}, {} – {} steps", lane.name, r, task_cnt),
                None => format!("{} – {} steps", lane.name, task_cnt),
            };
            let hover_bg = p.hover;
            let lane_id_hov = lane_id.clone();

            let lane_btn = pro_tactile(
                pro_eid(&self.id, format!("lane-btn-{lane_id}")),
                aria,
                6.0,
                if lane_open {
                    p.select
                } else {
                    transparent_black()
                },
                hover_bg,
                transparent_black(),
                transparent_black(),
                lane_open,
                div()
                    .w(px(label_w))
                    .h(px(LANE_H - 4.0))
                    .px(px(4.0))
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(render_lane_mark(lane, 16.0, lane_busy, elapsed_ms, &p, cx))
                    .when(!mark_only, |d| {
                        d.child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .child(
                                    div()
                                        .text_size(px(13.0))
                                        .line_height(px(18.0))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(if lane_open { p.strong } else { p.ink })
                                        .truncate()
                                        .child(lane.name.clone()),
                                )
                                .when_some(lane.role.clone(), |col, role| {
                                    col.child(
                                        div()
                                            .text_size(px(12.0))
                                            .line_height(px(16.0))
                                            .text_color(p.muted)
                                            .truncate()
                                            .child(role),
                                    )
                                }),
                        )
                    }),
                cx,
                move |this, _, cx| {
                    this.toggle_selection(SwimSelection::Lane(lane_id.clone()), cx);
                },
            )
            .w(px(label_w))
            .h(px(LANE_H - 4.0))
            .aria_expanded(lane_open)
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                let next = if *inside {
                    Some(lane_id_hov.clone())
                } else if this.hovered_lane.as_ref() == Some(&lane_id_hov) {
                    None
                } else {
                    this.hovered_lane.clone()
                };
                if this.hovered_lane != next {
                    this.hovered_lane = next;
                    cx.notify();
                }
            }));

            lanes_stack = lanes_stack.child(
                div()
                    .w_full()
                    .h(px(LANE_H))
                    .border_t_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .child(lane_btn),
            );
        }

        // Build positioned task/gate/event nodes.
        let mut nodes_layer = div().absolute().inset_0();
        for (n_idx, node) in graph.nodes.iter().enumerate() {
            let task = &self.tasks[node.task_idx];
            let lane = &self.lanes[node.lane_idx];
            let task_id = task.id.clone();
            let is_open = self.selected.as_ref() == Some(&SwimSelection::Task(task_id.clone()));
            let is_hovered = self.hovered_node.as_ref() == Some(&task_id);
            let lane_c = lane.color(&p);
            let aria = format!("{} – {}, {}", task.title, lane.name, task.status.label(cx));
            let nop = node_opacities.get(n_idx).copied().unwrap_or(1.0);

            let node_el: AnyElement = match task.kind {
                SwimTaskKind::Start | SwimTaskKind::End => {
                    let is_end = task.kind == SwimTaskKind::End;
                    let ring_w = if is_end { 3.0 } else { 1.5 };
                    let ring_c = if is_open || is_hovered {
                        p.ink
                    } else if is_end {
                        if task.status == SwimTaskStatus::Done {
                            p.ink
                        } else {
                            edge_next
                        }
                    } else {
                        edge_done
                    };
                    let hov_id = task_id.clone();
                    let click_id = task_id.clone();
                    pro_tactile(
                        pro_eid(&self.id, format!("node-{task_id}")),
                        aria,
                        EVENT_SIDE * 0.5,
                        transparent_black(),
                        transparent_black(),
                        transparent_black(),
                        transparent_black(),
                        is_open,
                        div()
                            .size(px(EVENT_SIDE))
                            .rounded_full()
                            .bg(box_surface)
                            .border(px(ring_w))
                            .border_color(ring_c),
                        cx,
                        move |this, _, cx| {
                            this.toggle_selection(SwimSelection::Task(click_id.clone()), cx);
                        },
                    )
                    .aria_expanded(is_open)
                    .absolute()
                    .left(px(node.x_left))
                    .top(px(node.y_center - EVENT_SIDE * 0.5))
                    .size(px(EVENT_SIDE))
                    .opacity(nop)
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                        this.hovered_node = if *inside {
                            Some(hov_id.clone())
                        } else if this.hovered_node.as_ref() == Some(&hov_id) {
                            None
                        } else {
                            this.hovered_node.clone()
                        };
                        cx.notify();
                    }))
                    .into_any_element()
                }
                SwimTaskKind::Gate => {
                    let (fill_c, stroke_c, glyph_c, pill_bg, pill_fg) = match task.status {
                        SwimTaskStatus::Waiting => (
                            pro_mix_color(box_surface, p.amber, if p.dark { 0.22 } else { 0.16 }),
                            p.amber.opacity(0.68),
                            p.amber_ink,
                            pro_mix_color(box_surface, p.amber, if p.dark { 0.20 } else { 0.14 }),
                            p.amber_ink,
                        ),
                        SwimTaskStatus::Failed => (
                            pro_mix_color(box_surface, p.red, if p.dark { 0.22 } else { 0.14 }),
                            p.red.opacity(0.65),
                            p.red_ink,
                            pro_mix_color(box_surface, p.red, if p.dark { 0.20 } else { 0.12 }),
                            p.red_ink,
                        ),
                        SwimTaskStatus::Done => (
                            box_surface,
                            p.green.opacity(0.60),
                            p.green_ink,
                            box_surface,
                            p.text,
                        ),
                        SwimTaskStatus::Active => (
                            box_surface,
                            p.blue.opacity(0.60),
                            p.blue,
                            box_surface,
                            p.ink,
                        ),
                        SwimTaskStatus::Queued | SwimTaskStatus::Skipped => (
                            box_surface,
                            if is_open || is_hovered {
                                p.line_strong
                            } else {
                                edge_done
                            },
                            p.muted,
                            box_surface,
                            p.muted,
                        ),
                    };
                    let is_waiting = task.status == SwimTaskStatus::Waiting;
                    let halo_alpha = if is_waiting && !reduced {
                        0.28 * (1.0 - ask_breath)
                    } else {
                        0.0
                    };
                    let halo_expand = 3.5 * ask_breath;
                    let warn_c = p.amber;

                    let diamond_canvas = canvas(
                        |_, _, _| (),
                        move |bounds, _, window, _| {
                            let c = bounds.center();
                            if halo_alpha > 0.01 {
                                paint_diamond(
                                    c,
                                    11.8 + halo_expand,
                                    transparent_black(),
                                    Some((1.5, warn_c.opacity(halo_alpha))),
                                    window,
                                );
                            }
                            paint_diamond(c, 11.5, fill_c, Some((1.15, stroke_c)), window);
                        },
                    )
                    .absolute()
                    .inset_0();

                    let glyph_el = match task.status {
                        SwimTaskStatus::Done => pro_check_draw(
                            pro_eid(&self.id, format!("swim-gchk-{task_id}")),
                            true,
                            12.0,
                            false,
                            glyph_c,
                        ),
                        SwimTaskStatus::Failed => {
                            pro_svg("0 0 24 24", CROSS_TINY_SVG, 10.5, glyph_c)
                        }
                        _ => pro_svg("0 0 24 24", GATE_ASK_SVG, 11.0, glyph_c),
                    };

                    let hov_id = task_id.clone();
                    let click_id = task_id.clone();
                    let pill_w = 92.0_f32;
                    pro_tactile(
                        pro_eid(&self.id, format!("node-{task_id}")),
                        aria,
                        6.0,
                        transparent_black(),
                        transparent_black(),
                        transparent_black(),
                        transparent_black(),
                        is_open,
                        div()
                            .size(px(GATE_HIT))
                            .relative()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(diamond_canvas)
                            .child(div().relative().child(glyph_el))
                            .child(
                                div()
                                    .absolute()
                                    .top(px(GATE_HIT + 3.0))
                                    .left(px((GATE_HIT - pill_w) * 0.5))
                                    .w(px(pill_w))
                                    .flex()
                                    .justify_center()
                                    .child(
                                        div()
                                            .max_w(px(pill_w))
                                            .h(px(18.0))
                                            .px(px(6.0))
                                            .rounded(px(5.0))
                                            .bg(pill_bg)
                                            .border_1()
                                            .border_color(p.line)
                                            .text_size(px(11.0))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(pill_fg)
                                            .flex()
                                            .items_center()
                                            .truncate()
                                            .child(task.title.clone()),
                                    ),
                            ),
                        cx,
                        move |this, _, cx| {
                            this.toggle_selection(SwimSelection::Task(click_id.clone()), cx);
                        },
                    )
                    .aria_expanded(is_open)
                    .absolute()
                    .left(px(node.x_center - GATE_HIT * 0.5))
                    .top(px(node.y_center - GATE_HIT * 0.5))
                    .size(px(GATE_HIT))
                    .opacity(nop)
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                        this.hovered_node = if *inside {
                            Some(hov_id.clone())
                        } else if this.hovered_node.as_ref() == Some(&hov_id) {
                            None
                        } else {
                            this.hovered_node.clone()
                        };
                        cx.notify();
                    }))
                    .into_any_element()
                }
                SwimTaskKind::Task => {
                    let bw = (node.x_right - node.x_left).max(50.0);
                    let (bg_c, border_c, text_c) = match task.status {
                        SwimTaskStatus::Active => (
                            pro_mix_color(box_surface, lane_c, if p.dark { 0.16 } else { 0.11 }),
                            lane_c.opacity(0.58),
                            p.strong,
                        ),
                        SwimTaskStatus::Done => (
                            pro_mix_color(box_surface, lane_c, if p.dark { 0.09 } else { 0.06 }),
                            if is_open || is_hovered {
                                p.line_strong
                            } else {
                                p.line
                            },
                            p.text,
                        ),
                        SwimTaskStatus::Waiting => (
                            pro_mix_color(box_surface, p.amber, if p.dark { 0.18 } else { 0.13 }),
                            p.amber.opacity(0.55),
                            p.amber_ink,
                        ),
                        SwimTaskStatus::Failed => (
                            pro_mix_color(box_surface, p.red, if p.dark { 0.18 } else { 0.12 }),
                            p.red.opacity(0.55),
                            p.red_ink,
                        ),
                        SwimTaskStatus::Queued | SwimTaskStatus::Skipped => (
                            box_surface,
                            if is_open || is_hovered {
                                p.line_strong
                            } else {
                                p.line
                            },
                            p.muted,
                        ),
                    };

                    let lead_glyph: Option<AnyElement> = match task.status {
                        SwimTaskStatus::Done => Some(pro_check_draw(
                            pro_eid(&self.id, format!("swim-chk-{task_id}")),
                            true,
                            12.0,
                            false,
                            lane_c,
                        )),
                        SwimTaskStatus::Active => {
                            Some(render_lane_mark(lane, 12.0, true, elapsed_ms, &p, cx))
                        }
                        SwimTaskStatus::Waiting => {
                            Some(pro_icon(ProIcon::Clock, 11.5, p.amber_ink))
                        }
                        SwimTaskStatus::Failed => {
                            Some(pro_svg("0 0 24 24", CROSS_TINY_SVG, 11.5, p.red_ink))
                        }
                        SwimTaskStatus::Queued | SwimTaskStatus::Skipped => None,
                    };

                    let is_active = task.status == SwimTaskStatus::Active;
                    let is_skipped = task.status == SwimTaskStatus::Skipped;
                    let hov_id = task_id.clone();
                    let click_id = task_id.clone();
                    let hover_bg = pro_mix_color(bg_c, lane_c, if p.dark { 0.08 } else { 0.05 });

                    pro_tactile(
                        pro_eid(&self.id, format!("node-{task_id}")),
                        aria,
                        8.0,
                        bg_c,
                        hover_bg,
                        border_c,
                        p.line_strong,
                        is_open,
                        div()
                            .w(px(bw))
                            .h(px(TASK_H))
                            .px(px(9.0))
                            .relative()
                            .overflow_hidden()
                            .flex()
                            .items_center()
                            .gap(px(5.0))
                            .when(is_active && !reduced, |d| {
                                let band_w = (bw * 0.52).max(28.0);
                                let sx = -band_w + sheen_phase * (bw + band_w * 2.0);
                                d.child(
                                    canvas(
                                        |_, _, _| (),
                                        move |bounds, _, window, _| {
                                            let sheen_rect = Bounds::new(
                                                point(bounds.left() + px(sx), bounds.top()),
                                                size(px(band_w), bounds.size.height),
                                            );
                                            window.paint_quad(fill(
                                                sheen_rect,
                                                linear_gradient(
                                                    90.0,
                                                    linear_color_stop(lane_c.opacity(0.0), 0.0),
                                                    linear_color_stop(lane_c.opacity(0.18), 0.5),
                                                ),
                                            ));
                                        },
                                    )
                                    .absolute()
                                    .inset_0(),
                                )
                            })
                            .when_some(lead_glyph, |d, g| d.child(g))
                            .child(
                                div()
                                    .relative()
                                    .flex_1()
                                    .min_w_0()
                                    .text_size(px(12.0))
                                    .line_height(px(14.5))
                                    .text_color(text_c)
                                    .when(is_skipped, |t| t.line_through())
                                    .line_clamp(2)
                                    .child(task.title.clone()),
                            ),
                        cx,
                        move |this, _, cx| {
                            this.toggle_selection(SwimSelection::Task(click_id.clone()), cx);
                        },
                    )
                    .aria_expanded(is_open)
                    .absolute()
                    .left(px(node.x_left))
                    .top(px(node.y_center - TASK_H * 0.5))
                    .w(px(bw))
                    .h(px(TASK_H))
                    .opacity(nop)
                    .when(!p.dark && (is_open || is_hovered || is_active), |d| {
                        d.shadow_xs()
                    })
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                        this.hovered_node = if *inside {
                            Some(hov_id.clone())
                        } else if this.hovered_node.as_ref() == Some(&hov_id) {
                            None
                        } else {
                            this.hovered_node.clone()
                        };
                        cx.notify();
                    }))
                    .into_any_element()
                }
            };
            nodes_layer = nodes_layer.child(node_el);
        }

        let card_open = self.selected.is_some();
        let shown_sel = self.selected.clone().or_else(|| self.last_selected.clone());
        let card_overlay = if let Some(sel) = shown_sel {
            let sel_key: SharedString = match &sel {
                SwimSelection::Task(tid) => format!("task:{tid}").into(),
                SwimSelection::Lane(lid) => format!("lane:{lid}").into(),
            };
            let graph_clone = graph.clone();
            let card_el =
                self.render_detail_card(&sel, &graph_clone, canvas_w, total_h, elapsed_ms, 1.0, cx);
            let vtx_el = pro_view_transition(pro_eid(&self.id, "swim-card-vtx"), sel_key, card_el);
            pro_presence(
                pro_eid(&self.id, "swim-card-pres"),
                card_open,
                0.18,
                0.14,
                move |amt| {
                    div()
                        .absolute()
                        .inset_0()
                        .opacity(pro_ease(amt))
                        .child(vtx_el)
                        .into_any_element()
                },
            )
        } else {
            div().into_any_element()
        };

        div()
            .id(self.id.clone())
            .role(Role::Group)
            .aria_label(title)
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::handle_key_down))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    if this.selected.is_some() {
                        this.close_card(cx);
                    }
                }),
            )
            .relative()
            .w_full()
            .min_w_0()
            .pt(px(16.0))
            .px(px(18.0))
            .pb(px(14.0))
            .rounded(px(14.0))
            .bg(card_surface)
            .border_1()
            .border_color(p.line)
            .when(!p.dark, |d| d.shadow_xs())
            .flex()
            .flex_col()
            .gap(px(12.0))
            .child(header)
            .child(
                div()
                    .relative()
                    .w_full()
                    .h(px(total_h))
                    .child(lanes_stack)
                    .child(edge_layer)
                    .child(nodes_layer)
                    .child(card_overlay),
            )
            .when(!self.announcement.is_empty(), |d| {
                d.child(
                    div()
                        .id(pro_eid(&self.id, "swim-announce"))
                        .role(Role::Status)
                        .aria_label(self.announcement.clone())
                        .size(px(0.0))
                        .overflow_hidden(),
                )
            })
            .child(pro_toast_banner(self.toast.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        SwimLane, SwimLaneKind, SwimTask, SwimTaskKind, SwimTaskStatus, build_swim_graph,
        layout_swim_columns,
    };

    #[test]
    fn swim_graph_places_parallel_branches_and_cuts_cycles_cleanly() {
        let lanes = vec![
            SwimLane::new("you", "You").with_kind(SwimLaneKind::Person),
            SwimLane::new("coder", "Coder"),
            SwimLane::new("writer", "Writer"),
        ];
        let tasks = vec![
            SwimTask::new("start", "you", "Start")
                .with_kind(SwimTaskKind::Start)
                .with_status(SwimTaskStatus::Done)
                .with_after(["end"]), // deliberate cycle back-edge
            SwimTask::new("code", "coder", "Write fix")
                .with_after(["start"])
                .with_status(SwimTaskStatus::Active),
            SwimTask::new("docs", "writer", "Write docs")
                .with_after(["start"])
                .with_status(SwimTaskStatus::Active),
            SwimTask::new("ghost", "missing-lane", "Ignored"),
            SwimTask::new("end", "you", "Gate")
                .with_kind(SwimTaskKind::Gate)
                .with_after(["code", "docs"])
                .with_status(SwimTaskStatus::Waiting),
        ];

        let mut graph = build_swim_graph(&lanes, &tasks);
        assert_eq!(graph.nodes.len(), 4);
        assert_eq!(graph.col_count, 3);
        // `code` and `docs` share column 1 in parallel, converging into `end` at column 2.
        let code_col = graph.nodes[graph.by_id["code"]].col;
        let docs_col = graph.nodes[graph.by_id["docs"]].col;
        let end_col = graph.nodes[graph.by_id["end"]].col;
        assert_eq!(code_col, docs_col);
        assert_eq!(end_col, code_col + 1);

        layout_swim_columns(&mut graph, &tasks, 360.0, 30.0);
        for node in &graph.nodes {
            assert!(node.x_right <= 360.0);
        }
    }
}
