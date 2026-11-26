//! Git-like checkpoint rail for an autonomous agent run with non-destructive branch rewind.
//!
//! Displays chronological state checkpoints along a `#82bcff` active rail with tool marks,
//! file counts, elapsed times, and cumulative dollar cost. Rewinding to any earlier checkpoint
//! preserves prior attempts as dimmed parallel branches, marks any running head as `Stopped`,
//! and bends a new lane out of the chosen parent via [`PathBuilder`].

use crate::{
    components::pro_chrome::{
        ProPalette, pro_app_tile, pro_card_shadow, pro_ease, pro_eid, pro_empty_state, pro_fold,
        pro_icon_btn, pro_rolling_number, pro_shimmer_text, pro_skeleton_bar, pro_spring_f32,
        pro_tactile, tr,
    },
    primitives::{
        family::disabled_motion,
        pro_icons::{ProIcon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{
    collections::{HashMap, HashSet},
    f32::consts::TAU,
};
use web_time::Instant;

const ROW_H: f32 = 30.0;
const RAIL_X0: f32 = 22.0;
const LANE_STEP: f32 = 14.0;
const BASE_GUTTER: f32 = 38.0;
const MAX_VISIBLE_ROWS: usize = 14;

const HEADER_BRANCH_SVG: &str = r#"<path d="M5 2.5v11" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" opacity="0.45"></path><path d="M5 8c0 2 1.6 2.6 3.2 2.6H9" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" opacity="0.45"></path><circle cx="5" cy="5" r="2.25" fill="currentColor"></circle><circle cx="11.25" cy="10.6" r="2.25" fill="currentColor"></circle>"#;

const REWIND_ICON_SVG: &str = r#"<path d="M2.6 3.6A4 4 0 1 1 2 6.4" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"></path><path d="M2.2 1.6v2.4h2.4" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"></path>"#;

const FOLDER_META_SVG: &str = r#"<path d="M1.4 3.1c0-.72.58-1.3 1.3-1.3h1.72c.38 0 .73.19.94.5l.6.9h3.34c.72 0 1.3.58 1.3 1.3V6H1.4Z" fill="currentColor"></path><path d="M1 6.95C.86 6.3 1.36 5.7 2.02 5.7h7.96c.66 0 1.16.6 1.02 1.25l-.5 2.4a1.3 1.3 0 0 1-1.27 1.05H2.77A1.3 1.3 0 0 1 1.5 9.35Z" fill="currentColor" opacity="0.45"></path>"#;

/// Execution status of a single [`Checkpoint`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CheckpointStatus {
    /// Completed state snapshot.
    #[default]
    Done,
    /// Active checkpoint currently being worked toward (shimmers in place).
    Running,
    /// Checkpoint where verification or tool calls failed.
    Failed,
    /// Interrupted when the user rewound to an earlier checkpoint while running.
    Stopped,
}

/// One saved state snapshot in [`AgentCheckpoints`].
#[derive(Clone, Debug, PartialEq)]
pub struct Checkpoint {
    /// Stable identifier referenced by `head` and child `parent` links.
    pub id: SharedString,
    /// Human-readable summary of what was accomplished at this checkpoint.
    pub title: SharedString,
    /// Optional Chinese translation for bilingual demo surfaces.
    pub title_zh: Option<SharedString>,
    /// Optional app/tool key rendered via [`pro_app_tile`] before the title (`"slack"`, `"github"`, `"notion"`, etc.).
    pub icon: Option<SharedString>,
    /// Parent checkpoint identifier; a second child of the same parent opens a parallel lane.
    pub parent: Option<SharedString>,
    /// Execution status (`Done`, `Running`, `Failed`, or `Stopped`).
    pub status: CheckpointStatus,
    /// Elapsed run timestamp in milliseconds (`9_000` -> `"0:09"`).
    pub at_ms: Option<u64>,
    /// Count of touched files at this checkpoint.
    pub files: Option<u32>,
    /// Incremental run cost in USD contributed by this step.
    pub cost: Option<f64>,
}

impl Checkpoint {
    /// Create a [`Checkpoint`] with `id` and `title` (`Done` by default).
    pub fn new(id: impl Into<SharedString>, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            title_zh: None,
            icon: None,
            parent: None,
            status: CheckpointStatus::Done,
            at_ms: None,
            files: None,
            cost: None,
        }
    }

    /// Provide a localized Chinese title for bilingual UI switching.
    pub fn with_title_zh(mut self, title_zh: impl Into<SharedString>) -> Self {
        self.title_zh = Some(title_zh.into());
        self
    }

    /// Set the leading app or provider mark key (for example `"slack"`, `"github"`, `"notion"`).
    pub fn with_icon(mut self, icon: impl Into<SharedString>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Link this checkpoint after `parent`.
    pub fn with_parent(mut self, parent: impl Into<SharedString>) -> Self {
        self.parent = Some(parent.into());
        self
    }

    /// Set the execution status (`Done`, `Running`, `Failed`, or `Stopped`).
    pub fn with_status(mut self, status: CheckpointStatus) -> Self {
        self.status = status;
        self
    }

    /// Set the elapsed timestamp in milliseconds (`0:09`, `1:21`, etc.).
    pub fn with_at_ms(mut self, at_ms: u64) -> Self {
        self.at_ms = Some(at_ms);
        self
    }

    /// Set the modified file count shown in the right metadata column.
    pub fn with_files(mut self, files: u32) -> Self {
        self.files = Some(files);
        self
    }

    /// Set the incremental dollar cost contributed by this checkpoint.
    pub fn with_cost(mut self, cost: f64) -> Self {
        self.cost = Some(cost);
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

/// Typed events emitted by [`AgentCheckpoints`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentCheckpointsEvent {
    /// User rewound to `parent_id`, preserving history and branching `new_head_id` on `attempt`.
    Rewind {
        parent_id: SharedString,
        new_head_id: Option<SharedString>,
        attempt: usize,
    },
}

impl EventEmitter<AgentCheckpointsEvent> for AgentCheckpoints {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ComputedCheckpointNode {
    pub lane: usize,
    pub parent_idx: Option<usize>,
    pub attempt_badge: Option<usize>,
    pub is_live: bool,
    pub is_head: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ComputedCheckpointGraph {
    pub nodes: Vec<ComputedCheckpointNode>,
    pub num_lanes: usize,
    pub head_idx: Option<usize>,
}

pub(crate) fn compute_checkpoint_graph(
    checkpoints: &[Checkpoint],
    head: Option<&str>,
) -> ComputedCheckpointGraph {
    if checkpoints.is_empty() {
        return ComputedCheckpointGraph {
            nodes: Vec::new(),
            num_lanes: 1,
            head_idx: None,
        };
    }

    let mut id_to_idx: HashMap<&str, usize> = HashMap::with_capacity(checkpoints.len());
    for (idx, cp) in checkpoints.iter().enumerate() {
        id_to_idx.insert(cp.id.as_ref(), idx);
    }

    let resolved_head_idx = head
        .and_then(|h| id_to_idx.get(h).copied())
        .unwrap_or(checkpoints.len() - 1);

    let mut live_indices = HashSet::new();
    let mut cursor = Some(resolved_head_idx);
    let mut guard = 0usize;
    while let Some(idx) = cursor {
        if !live_indices.insert(idx) || guard > checkpoints.len() {
            break;
        }
        guard += 1;
        cursor = checkpoints[idx]
            .parent
            .as_deref()
            .and_then(|pid| id_to_idx.get(pid).copied())
            .filter(|&p| p < idx);
    }

    let mut child_counts: HashMap<usize, usize> = HashMap::new();
    let mut lanes: Vec<usize> = Vec::with_capacity(checkpoints.len());
    let mut parent_indices: Vec<Option<usize>> = Vec::with_capacity(checkpoints.len());
    let mut attempt_badges: Vec<Option<usize>> = Vec::with_capacity(checkpoints.len());
    let mut next_lane = 0usize;

    for (idx, cp) in checkpoints.iter().enumerate() {
        let p_idx = cp
            .parent
            .as_deref()
            .and_then(|pid| id_to_idx.get(pid).copied())
            .filter(|&p| p < idx);
        parent_indices.push(p_idx);

        match p_idx {
            None => {
                let lane = next_lane;
                next_lane += 1;
                lanes.push(lane);
                attempt_badges.push(if idx > 0 { Some(lane + 1) } else { None });
            }
            Some(p) => {
                let count = child_counts.entry(p).or_insert(0);
                *count += 1;
                if *count == 1 {
                    lanes.push(lanes[p]);
                    attempt_badges.push(None);
                } else {
                    let lane = next_lane;
                    next_lane += 1;
                    lanes.push(lane);
                    attempt_badges.push(Some(*count));
                }
            }
        }
    }

    let num_lanes = next_lane.max(1);
    let nodes = (0..checkpoints.len())
        .map(|idx| ComputedCheckpointNode {
            lane: lanes[idx],
            parent_idx: parent_indices[idx],
            attempt_badge: attempt_badges[idx],
            is_live: live_indices.contains(&idx),
            is_head: idx == resolved_head_idx,
        })
        .collect();

    ComputedCheckpointGraph {
        nodes,
        num_lanes,
        head_idx: Some(resolved_head_idx),
    }
}

/// Format elapsed milliseconds into compact `m:ss` or `h:mm:ss` checkpoint notation.
pub fn format_checkpoint_ms(ms: u64) -> String {
    let total_secs = ms / 1_000;
    let hours = total_secs / 3_600;
    let mins = (total_secs % 3_600) / 60;
    let secs = total_secs % 60;
    if hours > 0 {
        format!("{hours}:{mins:02}:{secs:02}")
    } else {
        format!("{mins}:{secs:02}")
    }
}

fn default_checkpoints() -> Vec<Checkpoint> {
    vec![
        Checkpoint::new("c1", "Read the brief")
            .with_title_zh("阅读需求简报")
            .with_icon("slack")
            .with_at_ms(9_000)
            .with_cost(0.03),
        Checkpoint::new("c2", "Scaffolded the pricing page")
            .with_title_zh("搭建定价页骨架")
            .with_parent("c1")
            .with_icon("github")
            .with_files(4)
            .with_at_ms(30_000)
            .with_cost(0.06),
        Checkpoint::new("c3", "Wrote the plan copy")
            .with_title_zh("撰写套餐方案文案")
            .with_parent("c2")
            .with_icon("notion")
            .with_files(1)
            .with_at_ms(47_000)
            .with_cost(0.04),
        Checkpoint::new("c4", "Built the pricing table")
            .with_title_zh("构建定价对比表格")
            .with_parent("c3")
            .with_icon("figma")
            .with_files(3)
            .with_at_ms(81_000)
            .with_cost(0.11)
            .with_status(CheckpointStatus::Running),
    ]
}

fn demo_streaming_fourth_checkpoint() -> Checkpoint {
    Checkpoint::new("c4", "Built the pricing table")
        .with_title_zh("构建定价对比表格")
        .with_parent("c3")
        .with_icon("figma")
        .with_files(3)
        .with_at_ms(81_000)
        .with_cost(0.11)
        .with_status(CheckpointStatus::Running)
}

fn rewind_branch_template(seq: usize, parent_id: &str, next_at_ms: u64) -> Checkpoint {
    let templates: [(&str, &str, &str, u32, f64); 4] = [
        (
            "Switched to Stripe Elements",
            "切换至 Stripe Elements 组件",
            "stripe",
            3,
            0.08,
        ),
        (
            "Rebuilt plan toggle & annual math",
            "重构年付切换与折扣计费",
            "github",
            2,
            0.06,
        ),
        (
            "Aligned enterprise FAQ copy",
            "对齐企业版常见问题文案",
            "notion",
            2,
            0.05,
        ),
        (
            "Verified responsive tier cards",
            "校验响应式套餐卡片布局",
            "figma",
            4,
            0.07,
        ),
    ];
    let (en, zh, icon, files, cost) = templates[(seq.saturating_sub(1)) % templates.len()];
    Checkpoint::new(format!("c-rw-{seq}"), en)
        .with_title_zh(zh)
        .with_parent(parent_id.to_string())
        .with_icon(icon)
        .with_files(files)
        .with_at_ms(next_at_ms)
        .with_cost(cost)
        .with_status(CheckpointStatus::Running)
}

/// Git-like checkpoint history card for an autonomous agent run.
///
/// # Examples
///
/// ```no_run
/// # #[allow(unused_variables, unused_imports)]
/// # fn example(window: &mut gpui::Window, cx: &mut gpui::App) {
/// # use gpui::prelude::*;
/// use gpui::prelude::*;
/// use kui::components::agent_checkpoints::{
///     AgentCheckpoints, AgentCheckpointsEvent, Checkpoint, CheckpointStatus,
/// };
///
/// let checkpoints = cx.new(|cx| {
///     AgentCheckpoints::new("pricing-run", cx)
///         .with_task("Ship the pricing page")
///         .with_checkpoints(vec![
///             Checkpoint::new("c1", "Read the brief").with_icon("slack").with_at_ms(9_000).with_cost(0.03),
///             Checkpoint::new("c2", "Scaffolded the pricing page").with_parent("c1").with_icon("github").with_files(4).with_at_ms(30_000).with_cost(0.06),
///             Checkpoint::new("c3", "Built the pricing table").with_parent("c2").with_icon("figma").with_files(3).with_at_ms(81_000).with_cost(0.11).with_status(CheckpointStatus::Running),
///         ])
/// });
/// cx.subscribe(&checkpoints, |_, ev: &AgentCheckpointsEvent, _| {
///     let _ = ev;
/// }).detach();
/// # }
/// ```
pub struct AgentCheckpoints {
    id: ElementId,
    title: Option<SharedString>,
    task: Option<SharedString>,
    checkpoints: Vec<Checkpoint>,
    head: Option<SharedString>,
    auto_branch_on_rewind: bool,
    demo_stream_pending: bool,
    mounted_at: Instant,
    hovered_idx: Option<usize>,
    focused_idx: Option<usize>,
    expanded_id: Option<SharedString>,
    rewind_seq: usize,
    last_added: Option<(SharedString, Instant)>,
    loading: bool,
    empty: bool,
}

impl AgentCheckpoints {
    /// Create an [`AgentCheckpoints`] card preloaded with the `"Ship the pricing page"` run.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        Self {
            id: id.into(),
            title: None,
            task: None,
            checkpoints: default_checkpoints(),
            head: Some("c4".into()),
            auto_branch_on_rewind: true,
            demo_stream_pending: false,
            mounted_at: Instant::now(),
            hovered_idx: None,
            focused_idx: None,
            expanded_id: None,
            rewind_seq: 0,
            last_added: None,
            loading: false,
            empty: false,
        }
    }

    /// Override the card heading (defaults to `"Checkpoints"` / `"检查点"`).
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Override the run task description shown next to the heading.
    pub fn with_task(mut self, task: impl Into<SharedString>) -> Self {
        self.task = Some(task.into());
        self
    }

    /// Supply custom ordered checkpoints (`parent` links establish lanes automatically).
    pub fn with_checkpoints(mut self, checkpoints: Vec<Checkpoint>) -> Self {
        self.head = checkpoints.last().map(|c| c.id.clone());
        self.checkpoints = checkpoints;
        self.demo_stream_pending = false;
        self
    }

    /// Set the active `head` checkpoint id.
    pub fn with_head(mut self, head: impl Into<SharedString>) -> Self {
        self.head = Some(head.into());
        self.demo_stream_pending = false;
        self
    }

    /// Control whether pressing `"Rewind"` automatically appends a new branched attempt checkpoint.
    pub fn with_auto_branch_on_rewind(mut self, enabled: bool) -> Self {
        self.auto_branch_on_rewind = enabled;
        self
    }

    /// Render loading skeleton rows while checkpoint state is being fetched.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Force the empty state view (`"No checkpoints yet"`).
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Return the current checkpoint slice (never shrinks on rewind).
    pub fn checkpoints(&self) -> &[Checkpoint] {
        &self.checkpoints
    }

    /// Return the active `head` checkpoint id.
    pub fn head(&self) -> Option<&str> {
        self.head
            .as_deref()
            .or_else(|| self.checkpoints.last().map(|c| c.id.as_ref()))
    }

    /// Rewind to `checkpoint_id` without deleting history: marks any `Running` head as `Stopped`,
    /// fades the abandoned branch, and (when `auto_branch_on_rewind` is enabled) appends a new
    /// checkpoint with `parent = checkpoint_id` and moves `head` to it.
    pub fn rewind_to(
        &mut self,
        checkpoint_id: &str,
        cx: &mut Context<Self>,
    ) -> Option<SharedString> {
        let target_idx = self
            .checkpoints
            .iter()
            .position(|c| c.id.as_ref() == checkpoint_id)?;
        self.demo_stream_pending = false;

        // Mark any currently running checkpoint (especially the active head) as Stopped.
        for cp in &mut self.checkpoints {
            if cp.status == CheckpointStatus::Running {
                cp.status = CheckpointStatus::Stopped;
            }
        }

        let parent_id = self.checkpoints[target_idx].id.clone();
        let sibling_count = self
            .checkpoints
            .iter()
            .filter(|c| c.parent.as_deref() == Some(parent_id.as_ref()))
            .count();
        let attempt = sibling_count + 1;

        let new_head_id = if self.auto_branch_on_rewind {
            self.rewind_seq += 1;
            let max_at = self
                .checkpoints
                .iter()
                .filter_map(|c| c.at_ms)
                .max()
                .unwrap_or(45_000);
            let next_cp =
                rewind_branch_template(self.rewind_seq, parent_id.as_ref(), max_at + 19_000);
            let new_id = next_cp.id.clone();
            self.checkpoints.push(next_cp);
            self.head = Some(new_id.clone());
            self.last_added = Some((new_id.clone(), Instant::now()));
            Some(new_id)
        } else {
            self.head = Some(parent_id.clone());
            None
        };

        cx.emit(AgentCheckpointsEvent::Rewind {
            parent_id,
            new_head_id: new_head_id.clone(),
            attempt,
        });
        cx.notify();
        new_head_id
    }
}

fn sample_edge_points(x0: f32, y0: f32, x1: f32, y1: f32, progress: f32) -> Vec<Point<Pixels>> {
    let progress = progress.clamp(0., 1.);
    if progress <= 0.002 {
        return Vec::new();
    }
    let total_steps = 36usize;
    let active_steps = ((total_steps as f32 * progress).ceil() as usize).clamp(1, total_steps);
    let mut pts = Vec::with_capacity(active_steps + 1);
    if (x1 - x0).abs() < 0.5 {
        for i in 0..=active_steps {
            let t = (i as f32 / active_steps as f32) * progress;
            pts.push(point(px(x0), px(y0 + (y1 - y0) * t)));
        }
    } else {
        let bend_h = (y1 - y0).clamp(12.0, 24.0);
        let y_turn = y0 + bend_h;
        let straight_h = (y1 - y_turn).max(0.0);
        let curve_len = (bend_h.hypot(x1 - x0)).max(1.0);
        let total_len = curve_len + straight_h;
        let curve_frac = (curve_len / total_len).clamp(0.15, 1.0);
        for i in 0..=active_steps {
            let s = (i as f32 / active_steps as f32) * progress;
            if s <= curve_frac {
                let u = (s / curve_frac).clamp(0., 1.);
                let inv = 1.0 - u;
                // Smooth S-bend out of parent checkpoint into parallel attempt lane
                let cx1 = x0;
                let cy1 = y0 + bend_h * 0.55;
                let cx2 = x1;
                let cy2 = y_turn - bend_h * 0.55;
                let bx = inv * inv * inv * x0
                    + 3.0 * inv * inv * u * cx1
                    + 3.0 * inv * u * u * cx2
                    + u * u * u * x1;
                let by = inv * inv * inv * y0
                    + 3.0 * inv * inv * u * cy1
                    + 3.0 * inv * u * u * cy2
                    + u * u * u * y_turn;
                pts.push(point(px(bx), px(by)));
            } else {
                let v = ((s - curve_frac) / (1.0 - curve_frac).max(1e-4)).clamp(0., 1.);
                pts.push(point(px(x1), px(y_turn + straight_h * v)));
            }
        }
    }
    pts
}

fn paint_circle_disc(
    center: Point<Pixels>,
    radius: f32,
    fill_color: Hsla,
    stroke_color: Hsla,
    stroke_w: f32,
    window: &mut Window,
) {
    if radius <= 0.4 {
        return;
    }
    let steps = 24usize;
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
        window.paint_path(path, fill_color);
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
            window.paint_path(path, stroke_color);
        }
    }
}

impl Render for AgentCheckpoints {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let reduced = disabled_motion(cx);

        // Stream the 4th live running checkpoint (`Built the pricing table`) smoothly in non-reduced mode.
        if self.demo_stream_pending && !reduced && !self.loading && !self.empty {
            let elapsed = self.mounted_at.elapsed().as_secs_f32();
            if elapsed >= 1.15 {
                let fourth = demo_streaming_fourth_checkpoint();
                let fourth_id = fourth.id.clone();
                self.checkpoints.push(fourth);
                self.head = Some(fourth_id.clone());
                self.last_added = Some((fourth_id, Instant::now()));
                self.demo_stream_pending = false;
            } else {
                window.request_animation_frame();
            }
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
        // `#82bcff` active checkpoint rail + deep/bright head dot
        let rail_live: Hsla = rgb(0x82bcff).into();
        let rail_dim: Hsla = if p.dark {
            hsla(0., 0., 1., 0.16)
        } else {
            hsla(0., 0., 0., 0.13)
        };
        let head_dot: Hsla = if p.dark {
            rgb(0x4fb0fe).into()
        } else {
            rgb(0x0070d6).into()
        };

        let heading_text: SharedString = self
            .title
            .clone()
            .unwrap_or_else(|| tr(cx, "Checkpoints", "检查点").into());
        let task_text: SharedString = self
            .task
            .clone()
            .unwrap_or_else(|| tr(cx, "Ship the pricing page", "上线定价页面").into());

        let count = if self.empty {
            0
        } else {
            self.checkpoints.len()
        };
        let total_cost: f64 = if self.empty {
            0.0
        } else {
            self.checkpoints.iter().filter_map(|c| c.cost).sum()
        };

        let count_suffix = if count == 1 {
            tr(cx, "checkpoint", "个检查点")
        } else {
            tr(cx, "checkpoints", "个检查点")
        };

        let header = div()
            .w_full()
            .pt(px(12.))
            .pr(px(16.))
            .pb(px(6.))
            .pl(px(14.))
            .flex()
            .items_center()
            .gap(px(8.))
            .child(
                div()
                    .size(px(16.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(pro_svg("0 0 16 16", HEADER_BRANCH_SVG, 16., p.muted)),
            )
            .child(
                div()
                    .flex_none()
                    .text_size(px(13.))
                    .line_height(px(20.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .child(heading_text),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(px(13.))
                    .line_height(px(20.))
                    .text_color(p.muted)
                    .child(task_text),
            )
            .child(
                div()
                    .flex_none()
                    .ml_auto()
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .text_size(px(12.))
                    .line_height(px(16.))
                    .text_color(p.muted)
                    .child(pro_rolling_number(
                        pro_eid(&self.id, "cp-cnt"),
                        count as f64,
                        0,
                        "",
                        "",
                        12.,
                        FontWeight::MEDIUM,
                        p.ink,
                    ))
                    .child(count_suffix)
                    .child("·")
                    .child(pro_rolling_number(
                        pro_eid(&self.id, "total-cost"),
                        total_cost,
                        2,
                        "$",
                        "",
                        12.,
                        FontWeight::NORMAL,
                        p.muted,
                    )),
            );

        if self.loading {
            let phase = if reduced {
                0.5
            } else {
                window.request_animation_frame();
                (self.mounted_at.elapsed().as_secs_f32() * 0.9).fract()
            };
            let mut skel = div()
                .w_full()
                .pb(px(12.))
                .pt(px(4.))
                .flex()
                .flex_col()
                .gap(px(6.));
            for r in 0..3usize {
                skel = skel.child(
                    div()
                        .h(px(ROW_H))
                        .pl(px(BASE_GUTTER))
                        .pr(px(16.))
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(pro_skeleton_bar(18., 18., 5., phase + r as f32 * 0.12, cx))
                        .child(pro_skeleton_bar(
                            150. + (r as f32 * 24.),
                            12.,
                            4.,
                            phase + r as f32 * 0.12,
                            cx,
                        ))
                        .child(div().ml_auto().child(pro_skeleton_bar(
                            42.,
                            11.,
                            4.,
                            phase + r as f32 * 0.12,
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
                        .max_w(px(500.))
                        .rounded(px(12.))
                        .bg(card_bg)
                        .border_1()
                        .border_color(card_border)
                        .shadow_sm()
                        .flex()
                        .flex_col()
                        .child(header)
                        .child(skel),
                )
                .into_any_element();
        }

        if self.empty || self.checkpoints.is_empty() {
            let body = pro_empty_state(
                ProIcon::GitBranch,
                tr(cx, "No checkpoints yet", "暂无检查点"),
                tr(
                    cx,
                    "State snapshots appear here as the agent completes each step.",
                    "智能体完成每一步后，状态快照会依次记录于此。",
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
                        .max_w(px(500.))
                        .rounded(px(12.))
                        .bg(card_bg)
                        .border_1()
                        .border_color(card_border)
                        .shadow_sm()
                        .flex()
                        .flex_col()
                        .child(header)
                        .child(body),
                )
                .into_any_element();
        }

        let graph = compute_checkpoint_graph(&self.checkpoints, self.head.as_deref());
        let extra_lanes = graph.num_lanes.saturating_sub(1) as f32;
        let target_gutter = BASE_GUTTER + extra_lanes * LANE_STEP;
        let gutter = pro_spring_f32(
            pro_eid(&self.id, "lane-gutter"),
            target_gutter,
            0.24,
            window,
            cx,
        );

        let target_list_h = (self.checkpoints.len().min(MAX_VISIBLE_ROWS) as f32) * ROW_H + 10.0;
        let animated_list_h =
            pro_spring_f32(pro_eid(&self.id, "list-h"), target_list_h, 0.22, window, cx);

        let new_anim: Option<(SharedString, f32)> =
            self.last_added.as_ref().and_then(|(aid, t0)| {
                if reduced {
                    return None;
                }
                let age = t0.elapsed().as_secs_f32();
                if age < 0.72 {
                    window.request_animation_frame();
                    Some((aid.clone(), age))
                } else {
                    None
                }
            });

        // Rail canvas painting all parent->child edges and checkpoint dots.
        let nodes_for_canvas = graph.nodes.clone();
        let ids_for_canvas: Vec<SharedString> =
            self.checkpoints.iter().map(|c| c.id.clone()).collect();
        let new_anim_canvas = new_anim.clone();
        let rail_w = gutter + 8.0;
        let rail_h = (self.checkpoints.len() as f32) * ROW_H + 6.0;

        let rail_canvas = canvas(
            |_, _, _| (),
            move |bounds, _, win, _| {
                let ox = f32::from(bounds.left());
                let oy = f32::from(bounds.top());

                // Pass 0: non-live (abandoned/faded) edges, Pass 1: live blue edges on top.
                for pass_live in [false, true] {
                    for (idx, node) in nodes_for_canvas.iter().enumerate() {
                        if node.is_live != pass_live {
                            continue;
                        }
                        let Some(p_idx) = node.parent_idx else {
                            continue;
                        };
                        let p_lane = nodes_for_canvas[p_idx].lane;
                        let x0 = ox + RAIL_X0 + p_lane as f32 * LANE_STEP;
                        let y0 = oy + p_idx as f32 * ROW_H + ROW_H * 0.5;
                        let x1 = ox + RAIL_X0 + node.lane as f32 * LANE_STEP;
                        let y1 = oy + idx as f32 * ROW_H + ROW_H * 0.5;

                        let is_newly_added = new_anim_canvas
                            .as_ref()
                            .is_some_and(|(aid, _)| aid == &ids_for_canvas[idx]);
                        let draw_prog = if let Some((_, age)) = new_anim_canvas.as_ref()
                            && is_newly_added
                        {
                            let dur = if p_lane != node.lane { 0.36 } else { 0.22 };
                            pro_ease((age / dur).clamp(0., 1.))
                        } else {
                            1.0
                        };

                        let pts = sample_edge_points(x0, y0, x1, y1, draw_prog);
                        if pts.len() >= 2 {
                            let mut builder = PathBuilder::stroke(px(1.55));
                            for (k, &pt) in pts.iter().enumerate() {
                                if k == 0 {
                                    builder.move_to(pt);
                                } else {
                                    builder.line_to(pt);
                                }
                            }
                            if let Ok(path) = builder.build() {
                                win.paint_path(path, if pass_live { rail_live } else { rail_dim });
                            }
                        }
                    }
                }

                // Dots on every checkpoint node.
                for (idx, node) in nodes_for_canvas.iter().enumerate() {
                    let cx_pt = ox + RAIL_X0 + node.lane as f32 * LANE_STEP;
                    let cy_pt = oy + idx as f32 * ROW_H + ROW_H * 0.5;
                    let is_newly_added = new_anim_canvas
                        .as_ref()
                        .is_some_and(|(aid, _)| aid == &ids_for_canvas[idx]);
                    let pop_scale = if let Some((_, age)) = new_anim_canvas.as_ref()
                        && is_newly_added
                    {
                        0.4 + 0.6 * pro_ease((age / 0.18).clamp(0., 1.))
                    } else {
                        1.0
                    };

                    if node.is_head {
                        // Subtle arrival glint ring on newly added head checkpoint
                        if let Some((_, age)) = new_anim_canvas.as_ref()
                            && is_newly_added
                            && *age > 0.10
                        {
                            let gt = ((*age - 0.10) / 0.55).clamp(0., 1.);
                            let halo_r = 4.5 + 6.0 * pro_ease(gt);
                            let alpha = (1.0 - gt) * 0.42;
                            paint_circle_disc(
                                point(px(cx_pt), px(cy_pt)),
                                halo_r,
                                transparent_black(),
                                head_dot.opacity(alpha),
                                1.3,
                                win,
                            );
                        }
                        paint_circle_disc(
                            point(px(cx_pt), px(cy_pt)),
                            4.3 * pop_scale,
                            head_dot,
                            card_bg,
                            1.6,
                            win,
                        );
                    } else if node.is_live {
                        paint_circle_disc(
                            point(px(cx_pt), px(cy_pt)),
                            3.3 * pop_scale,
                            rail_live,
                            card_bg,
                            1.5,
                            win,
                        );
                    } else {
                        paint_circle_disc(
                            point(px(cx_pt), px(cy_pt)),
                            3.1 * pop_scale,
                            card_bg,
                            rail_dim,
                            1.5,
                            win,
                        );
                    }
                }
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .w(px(rail_w))
        .h(px(rail_h));

        let mut rows_wrap = div().relative().w_full().pb(px(10.)).child(rail_canvas);

        for (idx, cp) in self.checkpoints.iter().enumerate() {
            let node = &graph.nodes[idx];
            let cp_id = cp.id.clone();
            let is_hovered = self.hovered_idx == Some(idx);
            let is_focused = self.focused_idx == Some(idx);
            let show_rewind = !node.is_head && (is_hovered || is_focused);
            let content_opacity = if node.is_live {
                1.0
            } else if is_hovered || is_focused {
                0.80
            } else {
                0.48
            };

            let is_newly_added = new_anim.as_ref().is_some_and(|(aid, _)| aid == &cp_id);
            let rise_t = if let Some((_, age)) = new_anim.as_ref()
                && is_newly_added
            {
                pro_ease((age / 0.20).clamp(0., 1.))
            } else {
                1.0
            };

            let title_str = cp.display_title(cx).to_string();
            let title_el = match cp.status {
                CheckpointStatus::Running => pro_shimmer_text(
                    pro_eid(&self.id, format!("shim-{cp_id}")),
                    title_str.clone(),
                    p.ink,
                    p.faint,
                    13.,
                    FontWeight::NORMAL,
                    true,
                ),
                CheckpointStatus::Stopped => div()
                    .truncate()
                    .text_size(px(13.))
                    .line_height(px(20.))
                    .text_color(p.muted)
                    .child(title_str.clone())
                    .into_any_element(),
                CheckpointStatus::Done | CheckpointStatus::Failed => div()
                    .truncate()
                    .text_size(px(13.))
                    .line_height(px(20.))
                    .text_color(p.ink)
                    .child(title_str.clone())
                    .into_any_element(),
            };

            let status_pill: Option<AnyElement> = match cp.status {
                CheckpointStatus::Failed => Some(
                    div()
                        .h(px(18.))
                        .px(px(6.))
                        .flex_none()
                        .rounded(px(5.))
                        .bg(p.red_soft)
                        .text_color(p.red_ink)
                        .text_size(px(11.))
                        .font_weight(FontWeight::MEDIUM)
                        .flex()
                        .items_center()
                        .child(tr(cx, "Failed", "失败"))
                        .into_any_element(),
                ),
                CheckpointStatus::Stopped => Some(
                    div()
                        .h(px(18.))
                        .px(px(6.))
                        .flex_none()
                        .rounded(px(5.))
                        .bg(p.chip)
                        .text_color(p.muted)
                        .text_size(px(11.))
                        .font_weight(FontWeight::MEDIUM)
                        .flex()
                        .items_center()
                        .child(tr(cx, "Stopped", "已停止"))
                        .into_any_element(),
                ),
                CheckpointStatus::Done | CheckpointStatus::Running => None,
            };

            let attempt_pill: Option<AnyElement> = node.attempt_badge.map(|att| {
                let text = if tr(cx, "en", "zh") == "zh" {
                    format!("第 {att} 次尝试")
                } else {
                    format!("Attempt {att}")
                };
                div()
                    .h(px(18.))
                    .px(px(6.))
                    .flex_none()
                    .rounded(px(5.))
                    .bg(p.chip)
                    .text_color(p.text)
                    .text_size(px(11.))
                    .font_weight(FontWeight::MEDIUM)
                    .flex()
                    .items_center()
                    .child(text)
                    .into_any_element()
            });

            let files_el: Option<AnyElement> = cp.files.map(|n| {
                let suffix = if tr(cx, "en", "zh") == "zh" {
                    " 个文件"
                } else if n == 1 {
                    " file"
                } else {
                    " files"
                };
                let exp_id = cp_id.clone();
                let is_exp = self.expanded_id.as_ref() == Some(&cp_id);
                let inner = div()
                    .h(px(20.))
                    .px(px(5.))
                    .flex()
                    .items_center()
                    .gap(px(3.))
                    .child(pro_svg("0 0 12 12", FOLDER_META_SVG, 11.5, p.muted))
                    .child(pro_rolling_number(
                        pro_eid(&self.id, format!("files-n-{cp_id}")),
                        n as f64,
                        0,
                        "",
                        suffix,
                        11.5,
                        FontWeight::NORMAL,
                        if is_exp { p.ink } else { p.faint },
                    ));
                pro_tactile(
                    pro_eid(&self.id, format!("files-btn-{cp_id}")),
                    SharedString::from(format!("{n}{suffix}")),
                    5.,
                    if is_exp { p.chip } else { transparent_black() },
                    p.hover,
                    transparent_black(),
                    transparent_black(),
                    is_exp,
                    inner,
                    cx,
                    move |this, _, cx| {
                        this.expanded_id = if this.expanded_id.as_ref() == Some(&exp_id) {
                            None
                        } else {
                            Some(exp_id.clone())
                        };
                        cx.notify();
                    },
                )
                .flex_none()
                .into_any_element()
            });

            let time_el: Option<AnyElement> = cp.at_ms.map(|ms| {
                if ms < 60_000 {
                    div()
                        .min_w(px(32.))
                        .flex_none()
                        .flex()
                        .justify_end()
                        .child(pro_rolling_number(
                            pro_eid(&self.id, format!("time-s-{cp_id}")),
                            (ms / 1_000) as f64,
                            0,
                            "0:",
                            "",
                            12.,
                            FontWeight::NORMAL,
                            p.muted,
                        ))
                        .into_any_element()
                } else {
                    div()
                        .min_w(px(32.))
                        .flex_none()
                        .text_right()
                        .text_size(px(12.))
                        .line_height(px(16.))
                        .text_color(p.muted)
                        .child(format_checkpoint_ms(ms))
                        .into_any_element()
                }
            });

            let row_bg = if is_hovered || is_focused || self.expanded_id.as_ref() == Some(&cp_id) {
                p.hover
            } else {
                transparent_black()
            };

            let aria = if node.is_head {
                format!("{title_str}, current")
            } else {
                format!("{title_str} – press Enter to rewind here")
            };

            let mut left_content = div()
                .flex_1()
                .min_w_0()
                .flex()
                .items_center()
                .gap(px(7.))
                .opacity(content_opacity * rise_t);

            if let Some(icon_key) = &cp.icon {
                left_content = left_content.child(pro_app_tile(icon_key.as_ref(), 18., cx));
            }
            left_content = left_content.child(div().min_w_0().flex_shrink(1.0).child(title_el));
            if let Some(att_el) = attempt_pill {
                left_content = left_content.child(att_el);
            }
            if let Some(st_el) = status_pill {
                left_content = left_content.child(st_el);
            }

            let meta_group = div()
                .flex_none()
                .ml_auto()
                .flex()
                .items_center()
                .gap(px(8.))
                .opacity(if show_rewind {
                    0.0
                } else {
                    content_opacity * rise_t
                })
                .when_some(files_el, |d, el| d.child(el))
                .when_some(time_el, |d, el| d.child(el));

            let rewind_target = cp_id.clone();
            let rw_aria =
                SharedString::from(format!("{} {}", tr(cx, "Rewind to", "回退至"), title_str));
            let rw_inner = div()
                .h(px(22.))
                .pl(px(7.))
                .pr(px(9.))
                .flex()
                .items_center()
                .gap(px(5.))
                .text_size(px(12.))
                .line_height(px(16.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.ink)
                .child(pro_svg("0 0 12 12", REWIND_ICON_SVG, 11.5, p.muted))
                .child(tr(cx, "Rewind", "回退"));
            let rewind_btn = pro_tactile(
                pro_eid(&self.id, format!("rw-btn-{cp_id}")),
                rw_aria,
                6.,
                card_bg,
                p.chip_hover,
                p.line_strong,
                p.line_strong,
                false,
                rw_inner,
                cx,
                move |this, _, cx| {
                    this.rewind_to(rewind_target.as_ref(), cx);
                },
            )
            .flex_none()
            .when(!p.dark, |d| d.shadow_xs());

            let row_click_id = cp_id.clone();
            let is_head = node.is_head;
            let row = div()
                .id(pro_eid(&self.id, format!("row-{cp_id}")))
                .role(Role::ListItem)
                .aria_label(aria)
                .w_full()
                .h(px(ROW_H))
                .relative()
                .top(px(4.0 * (1.0 - rise_t)))
                .pl(px(gutter))
                .pr(px(14.))
                .flex()
                .items_center()
                .gap(px(8.))
                .cursor_pointer()
                .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                    let next = if *inside { Some(idx) } else { None };
                    if this.hovered_idx != next {
                        this.hovered_idx = next;
                        cx.notify();
                    }
                }))
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.focused_idx = Some(idx);
                    if !is_head {
                        this.rewind_to(row_click_id.as_ref(), cx);
                    } else {
                        this.expanded_id = if this.expanded_id.as_ref() == Some(&row_click_id) {
                            None
                        } else {
                            Some(row_click_id.clone())
                        };
                        cx.notify();
                    }
                }))
                .child(
                    // Inset hover / keyboard focus highlight pill matching `.ui-ackc__row::before`
                    div()
                        .absolute()
                        .top(px(2.))
                        .bottom(px(2.))
                        .left(px(6.))
                        .right(px(6.))
                        .rounded(px(7.))
                        .bg(row_bg)
                        .when(is_focused, |d| d.border_1().border_color(p.focus)),
                )
                .child(left_content)
                .child(meta_group)
                .when(show_rewind, |d| {
                    d.child(
                        div()
                            .absolute()
                            .right(px(10.))
                            .top_0()
                            .bottom_0()
                            .flex()
                            .items_center()
                            .child(rewind_btn),
                    )
                });

            rows_wrap = rows_wrap.child(row);
        }

        let scrolls = self.checkpoints.len() > MAX_VISIBLE_ROWS;
        let list_container = div()
            .id(pro_eid(&self.id, "list"))
            .role(Role::List)
            .aria_label(tr(cx, "Agent checkpoints", "智能体检查点列表"))
            .tab_index(0)
            .w_full()
            .h(px(animated_list_h))
            .max_h(px(420.))
            .when(scrolls, |d| d.overflow_y_scroll())
            .when(!scrolls, |d| d.overflow_hidden())
            .focus_visible(move |s| s.border_1().border_color(p.focus))
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _, cx| {
                let len = this.checkpoints.len();
                if len == 0 {
                    return;
                }
                let cur = this
                    .focused_idx
                    .or_else(|| {
                        this.head()
                            .and_then(|h| this.checkpoints.iter().position(|c| c.id.as_ref() == h))
                    })
                    .unwrap_or(0);
                match ev.keystroke.key.as_str() {
                    "up" | "arrowup" => {
                        this.focused_idx = Some(cur.saturating_sub(1));
                        cx.stop_propagation();
                        cx.notify();
                    }
                    "down" | "arrowdown" => {
                        this.focused_idx = Some((cur + 1).min(len - 1));
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
                        let target_id = this.checkpoints[cur].id.clone();
                        if this.head() != Some(target_id.as_ref()) {
                            this.rewind_to(target_id.as_ref(), cx);
                        }
                        cx.stop_propagation();
                    }
                    _ => {}
                }
            }))
            .child(rows_wrap);

        // Smooth pro_fold file/cost snapshot disclosure when inspecting a checkpoint
        let inspected_cp = self
            .expanded_id
            .as_ref()
            .and_then(|eid| self.checkpoints.iter().find(|c| &c.id == eid))
            .cloned();
        let inspector_open = inspected_cp.is_some();
        let close_btn = inspector_open.then(|| {
            pro_icon_btn(
                pro_eid(&self.id, "close-cp-insp"),
                SharedString::from(tr(cx, "Close checkpoint details", "关闭检查点详情")),
                ProIcon::Close,
                22.,
                false,
                cx,
                |this, _, cx| {
                    this.expanded_id = None;
                    cx.notify();
                },
            )
        });
        let insp_base_id = self.id.clone();
        let inspector_fold = pro_fold(
            pro_eid(&self.id, "cp-inspector-fold"),
            inspector_open,
            62.0,
            0.0,
            move || {
                let Some(cp) = inspected_cp else {
                    return div().into_any_element();
                };
                let files_n = cp.files.unwrap_or(1);
                let cost_v = cp.cost.unwrap_or(0.0);
                let sample_files: [&str; 4] = [
                    "src/pricing/page.tsx",
                    "src/pricing/tiers.ts",
                    "src/pricing/faq.md",
                    "tests/pricing.spec.ts",
                ];
                let mut chips = div().flex().flex_wrap().items_center().gap(px(5.));
                for f_path in sample_files.into_iter().take(files_n.clamp(1, 4) as usize) {
                    chips = chips.child(
                        div()
                            .h(px(20.))
                            .px(px(6.))
                            .rounded(px(5.))
                            .bg(p.chip)
                            .font_family(crate::components::pro_chrome::pro_mono_name())
                            .text_size(px(11.))
                            .text_color(p.text)
                            .flex()
                            .items_center()
                            .child(f_path),
                    );
                }
                div()
                    .w_full()
                    .px(px(14.))
                    .py(px(9.))
                    .border_t_1()
                    .border_color(card_border)
                    .bg(p.subtle)
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .text_size(px(11.5))
                            .text_color(p.muted)
                            .child(
                                div()
                                    .font_family(crate::components::pro_chrome::pro_mono_name())
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .child(format!("#{}", cp.id)),
                            )
                            .child("·")
                            .child(pro_rolling_number(
                                pro_eid(&insp_base_id, "insp-cost"),
                                cost_v,
                                2,
                                "+$",
                                "",
                                11.5,
                                FontWeight::MEDIUM,
                                p.ink,
                            ))
                            .when_some(cp.at_ms, |d, ms| {
                                d.child("·").child(format_checkpoint_ms(ms))
                            })
                            .when_some(close_btn, |d, cb| d.child(div().ml_auto().child(cb))),
                    )
                    .child(chips)
                    .into_any_element()
            },
        );

        let outer_bdr: Hsla = if p.dark {
            hsla(0., 0., 1., 0.11)
        } else {
            hsla(0., 0., 0., 0.08)
        };

        div()
            .id(self.id.clone())
            .w_full()
            .flex()
            .justify_center()
            .child(
                div()
                    .w_full()
                    .max_w(px(480.))
                    .rounded(px(12.))
                    .bg(card_bg)
                    .border_1()
                    .border_color(outer_bdr)
                    .overflow_hidden()
                    .shadow(pro_card_shadow(p.dark))
                    .flex()
                    .flex_col()
                    .child(header)
                    .child(list_container)
                    .child(inspector_fold),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AgentCheckpoints, Checkpoint, CheckpointStatus, compute_checkpoint_graph,
        format_checkpoint_ms,
    };
    use gpui::{AppContext as _, TestAppContext};

    #[test]
    fn parent_links_open_parallel_lanes_and_dim_abandoned_branches() {
        let items = vec![
            Checkpoint::new("c1", "Read the brief").with_at_ms(9_000),
            Checkpoint::new("c2", "Built pricing")
                .with_parent("c1")
                .with_at_ms(64_000),
            Checkpoint::new("c3", "Tests failing")
                .with_parent("c2")
                .with_status(CheckpointStatus::Failed),
            Checkpoint::new("c4", "Switched to Stripe Elements")
                .with_parent("c2")
                .with_status(CheckpointStatus::Running),
        ];
        let graph = compute_checkpoint_graph(&items, Some("c4"));
        assert_eq!(graph.num_lanes, 2);
        assert_eq!(graph.nodes[0].lane, 0);
        assert_eq!(graph.nodes[1].lane, 0);
        assert_eq!(graph.nodes[2].lane, 0);
        assert_eq!(graph.nodes[3].lane, 1);
        assert_eq!(graph.nodes[3].attempt_badge, Some(2));

        // Ancestors of head ("c4") are c1 -> c2 -> c4; failed sibling c3 is off-branch.
        assert!(graph.nodes[0].is_live);
        assert!(graph.nodes[1].is_live);
        assert!(!graph.nodes[2].is_live);
        assert!(graph.nodes[3].is_live);
        assert!(graph.nodes[3].is_head);
        assert_eq!(format_checkpoint_ms(9_000), "0:09");
        assert_eq!(format_checkpoint_ms(81_000), "1:21");
    }

    #[gpui::test]
    fn rewind_preserves_checkpoints_marks_running_stopped_and_branches(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let view = cx.new(|cx| {
            AgentCheckpoints::new("test-checkpoints", cx).with_checkpoints(vec![
                Checkpoint::new("c1", "Read the brief"),
                Checkpoint::new("c2", "Scaffolded page").with_parent("c1"),
                Checkpoint::new("c3", "Drafting table")
                    .with_parent("c2")
                    .with_status(CheckpointStatus::Running),
            ])
        });

        view.update(cx, |v, cx| {
            assert_eq!(v.checkpoints().len(), 3);
            assert_eq!(v.head(), Some("c3"));
            let branched = v.rewind_to("c1", cx).expect("new branched checkpoint");
            assert_eq!(v.checkpoints().len(), 4, "never delete prior checkpoints");
            assert_eq!(v.checkpoints()[2].status, CheckpointStatus::Stopped);
            assert_eq!(v.head(), Some(branched.as_ref()));
            assert_eq!(v.checkpoints()[3].parent.as_deref(), Some("c1"));
            assert_eq!(v.checkpoints()[3].status, CheckpointStatus::Running);
        });
    }
}
