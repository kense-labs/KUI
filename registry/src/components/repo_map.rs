//! Codebase treemap with live coding agents hopping across files, diff fills, and conflict alerts.
//!
//! Groups repository files into folder regions sized proportionally to line count, tints edited
//! files in each agent's color from the bottom up with a live writing sheen, outlines read-only
//! files with a dashed border, blinks orange when two agents edit the same file, and isolates any
//! agent's footprint on click.

use crate::{
    components::pro_chrome::{
        ProPalette, pro_agent_avatar, pro_ease, pro_eid, pro_empty_state, pro_mix_color,
        pro_skeleton_bar, pro_spring_f32, tr,
    },
    primitives::{
        family::disabled_motion,
        pro_icons::{ProIcon, pro_icon},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, collections::HashMap, f32::consts::TAU, rc::Rc};
use web_time::Instant;

const DEFAULT_MAP_W: f32 = 740.0;
const DEFAULT_MAP_H: f32 = 344.0;
const FOLDER_GAP: f32 = 4.0;
const TILE_GAP: f32 = 3.0;
const FOLDER_HEAD_H: f32 = 20.0;
const TIP_W: f32 = 264.0;

/// Current activity mode of a [`RepoAgent`] on its target file.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RepoAction {
    /// Writing changes (fills tile bottom-up with sweeping sheen; `>= 2` editors blink orange).
    #[default]
    Edit,
    /// Inspecting context only (renders a dashed perimeter in the agent's color).
    Read,
}

/// Historical or committed line-change attribution on a [`RepoFile`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepoEdit {
    /// Identifier of the [`RepoAgent`] that authored this change.
    pub agent_id: SharedString,
    /// Added line count.
    pub added: u32,
    /// Removed line count.
    pub removed: u32,
}

impl RepoEdit {
    /// Create a [`RepoEdit`] record for `agent_id`.
    pub fn new(agent_id: impl Into<SharedString>, added: u32, removed: u32) -> Self {
        Self {
            agent_id: agent_id.into(),
            added,
            removed,
        }
    }
}

/// One repository file tile rendered inside [`RepoMap`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepoFile {
    /// Slash-separated repository path (for example `"src/lib/session.ts"`).
    pub path: SharedString,
    /// Line count (`LOC`) determining the tile's treemap area.
    pub lines: u32,
    /// Chronological list of edits applied to this file (last editor tints the tile).
    pub edits: Vec<RepoEdit>,
}

impl RepoFile {
    /// Create a [`RepoFile`] at `path` with `lines` LOC.
    pub fn new(path: impl Into<SharedString>, lines: u32) -> Self {
        Self {
            path: path.into(),
            lines: lines.max(10),
            edits: Vec::new(),
        }
    }

    /// Append a historical agent edit (`+added` / `−removed`).
    pub fn with_edit(
        mut self,
        agent_id: impl Into<SharedString>,
        added: u32,
        removed: u32,
    ) -> Self {
        self.edits.push(RepoEdit::new(agent_id, added, removed));
        self
    }

    /// Replace all historical edits on this file.
    pub fn with_edits(mut self, edits: Vec<RepoEdit>) -> Self {
        self.edits = edits;
        self
    }

    fn basename(&self) -> &str {
        self.path.rsplit('/').next().unwrap_or(self.path.as_ref())
    }

    fn folder_key(&self, depth: usize) -> String {
        let parts: Vec<&str> = self.path.split('/').collect();
        if parts.len() <= 1 {
            "/".to_string()
        } else {
            let take_n = (parts.len() - 1).min(depth.max(1));
            parts[..take_n].join("/")
        }
    }
}

/// One coding agent working inside [`RepoMap`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepoAgent {
    /// Stable agent identifier (drives pixel avatar pattern and color matching).
    pub id: SharedString,
    /// Human-readable agent name.
    pub name: SharedString,
    /// Optional Chinese agent name.
    pub name_zh: Option<SharedString>,
    /// Palette accent slot (`0..5`).
    pub color_idx: usize,
    /// File path where this agent is currently positioned, if any.
    pub file: Option<SharedString>,
    /// Whether the agent is editing or reading `file`.
    pub action: RepoAction,
    /// Short task note shown in the file hover inspector.
    pub task: Option<SharedString>,
    /// Optional Chinese task note.
    pub task_zh: Option<SharedString>,
}

impl RepoAgent {
    /// Create a [`RepoAgent`] with `id` and `name`.
    pub fn new(id: impl Into<SharedString>, name: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            name_zh: None,
            color_idx: 0,
            file: None,
            action: RepoAction::Edit,
            task: None,
            task_zh: None,
        }
    }

    /// Provide a localized Chinese agent name.
    pub fn with_name_zh(mut self, name_zh: impl Into<SharedString>) -> Self {
        self.name_zh = Some(name_zh.into());
        self
    }

    /// Set the palette color slot (`0` purple, `1` cyan/blue, `2` green, `3` pink, `4` teal).
    pub fn with_color_idx(mut self, color_idx: usize) -> Self {
        self.color_idx = color_idx;
        self
    }

    /// Position this agent on `file` with `action` (`Edit` or `Read`).
    pub fn with_target(mut self, file: impl Into<SharedString>, action: RepoAction) -> Self {
        self.file = Some(file.into());
        self.action = action;
        self
    }

    /// Set the current task description shown in the file hover card.
    pub fn with_task(mut self, task: impl Into<SharedString>) -> Self {
        self.task = Some(task.into());
        self
    }

    /// Provide a localized Chinese task description.
    pub fn with_task_zh(mut self, task_zh: impl Into<SharedString>) -> Self {
        self.task_zh = Some(task_zh.into());
        self
    }

    fn display_name<'a>(&'a self, cx: &App) -> &'a str {
        match &self.name_zh {
            Some(zh) => tr(cx, self.name.as_ref(), zh.as_ref()),
            None => self.name.as_ref(),
        }
    }

    fn display_task<'a>(&'a self, cx: &App) -> Option<&'a str> {
        let en = self.task.as_deref()?;
        Some(match &self.task_zh {
            Some(zh) => tr(cx, en, zh.as_ref()),
            None => en,
        })
    }
}

fn agent_accent(color_idx: usize, p: &ProPalette) -> Hsla {
    match color_idx % 5 {
        0 => p.purple,
        1 => p.cyan,
        2 => p.green,
        3 => p.pink,
        _ => p.teal,
    }
}

#[derive(Clone, Copy, Debug)]
struct RectF {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

/// Balanced binary-split treemap layout preserving input neighbor order.
fn layout_treemap(weights: &[f32], rect: RectF, gap: f32) -> Vec<RectF> {
    let n = weights.len();
    if n == 0 {
        return Vec::new();
    }
    if n == 1 || rect.w <= 2.0 || rect.h <= 2.0 {
        return vec![rect; n];
    }
    let total: f32 = weights.iter().map(|w| w.max(1.0)).sum();
    let half = total * 0.5;
    let mut acc = 0.0_f32;
    let mut split = 1usize;
    let mut best_diff = f32::INFINITY;
    for (i, &w) in weights.iter().enumerate().take(n - 1) {
        acc += w.max(1.0);
        let diff = (acc - half).abs();
        if diff < best_diff {
            best_diff = diff;
            split = i + 1;
        }
    }
    let left_sum: f32 = weights[..split].iter().map(|w| w.max(1.0)).sum();
    let ratio = (left_sum / total.max(1.0)).clamp(0.14, 0.86);

    let (r_a, r_b) = if rect.w >= rect.h * 1.05 {
        let usable = (rect.w - gap).max(2.0);
        let w_a = usable * ratio;
        let w_b = (usable - w_a).max(1.0);
        (
            RectF {
                x: rect.x,
                y: rect.y,
                w: w_a,
                h: rect.h,
            },
            RectF {
                x: rect.x + w_a + gap,
                y: rect.y,
                w: w_b,
                h: rect.h,
            },
        )
    } else {
        let usable = (rect.h - gap).max(2.0);
        let h_a = usable * ratio;
        let h_b = (usable - h_a).max(1.0);
        (
            RectF {
                x: rect.x,
                y: rect.y,
                w: rect.w,
                h: h_a,
            },
            RectF {
                x: rect.x,
                y: rect.y + h_a + gap,
                w: rect.w,
                h: h_b,
            },
        )
    };

    let mut out = layout_treemap(&weights[..split], r_a, gap);
    out.extend(layout_treemap(&weights[split..], r_b, gap));
    out
}

#[derive(Clone)]
struct FolderLayout {
    name: String,
    rect: RectF,
    file_indices: Vec<usize>,
}

#[derive(Clone)]
struct ComputedMap {
    folders: Vec<FolderLayout>,
    tile_rects: Vec<RectF>, // Indexed by file index in `self.files`
}

fn compute_repo_layout(files: &[RepoFile], depth: usize, map_w: f32, map_h: f32) -> ComputedMap {
    let mut folder_order: Vec<String> = Vec::new();
    let mut grouped: HashMap<String, Vec<usize>> = HashMap::new();
    for (idx, f) in files.iter().enumerate() {
        let key = f.folder_key(depth);
        if !grouped.contains_key(&key) {
            folder_order.push(key.clone());
        }
        grouped.entry(key).or_default().push(idx);
    }

    let folder_weights: Vec<f32> = folder_order
        .iter()
        .map(|k| {
            grouped[k]
                .iter()
                .map(|&fi| files[fi].lines as f32)
                .sum::<f32>()
                .max(20.0)
        })
        .collect();

    let outer = RectF {
        x: 4.0,
        y: 4.0,
        w: (map_w - 8.0).max(40.0),
        h: (map_h - 8.0).max(40.0),
    };
    let folder_rects = layout_treemap(&folder_weights, outer, FOLDER_GAP);

    let mut tile_rects = vec![
        RectF {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0
        };
        files.len()
    ];
    let mut folders = Vec::with_capacity(folder_order.len());

    for (f_i, key) in folder_order.into_iter().enumerate() {
        let f_rect = folder_rects[f_i];
        let indices = grouped.remove(&key).unwrap_or_default();
        let f_weights: Vec<f32> = indices.iter().map(|&fi| files[fi].lines as f32).collect();
        let inner = RectF {
            x: f_rect.x + 3.5,
            y: f_rect.y + FOLDER_HEAD_H,
            w: (f_rect.w - 7.0).max(8.0),
            h: (f_rect.h - FOLDER_HEAD_H - 3.5).max(8.0),
        };
        let sub_rects = layout_treemap(&f_weights, inner, TILE_GAP);
        for (&fi, &sr) in indices.iter().zip(&sub_rects) {
            tile_rects[fi] = sr;
        }
        folders.push(FolderLayout {
            name: key,
            rect: f_rect,
            file_indices: indices,
        });
    }

    ComputedMap {
        folders,
        tile_rects,
    }
}

/// Typed events emitted by [`RepoMap`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RepoMapEvent {
    /// User toggled agent file isolation (`None` when cleared).
    AgentIsolated(Option<SharedString>),
    /// User focused or clicked a repository file path.
    FileSelected(Option<SharedString>),
    /// Agents hopped to their next working files.
    AgentsStepped,
}

impl EventEmitter<RepoMapEvent> for RepoMap {}

/// Codebase treemap visualizing live coding agents, bottom-up edit fills, dashed reads, and conflicts.
pub struct RepoMap {
    id: ElementId,
    title: SharedString,
    branch: SharedString,
    files: Vec<RepoFile>,
    agents: Vec<RepoAgent>,
    depth: usize,
    map_height: f32,
    isolated_agent: Option<SharedString>,
    hovered_file: Option<usize>,
    focused_file: Option<usize>,
    measured_w: Rc<Cell<f32>>,
    hop_timestamps: HashMap<SharedString, Instant>,
    hop_tick: usize,
    started_at: Instant,
    loading: bool,
    empty: bool,
}

impl RepoMap {
    /// Create a [`RepoMap`] preloaded with the 23-file `"acme/web"` repository and 4 active agents.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let files = default_repo_files();
        let agents = default_repo_agents();
        let now = Instant::now();
        let mut hop_timestamps = HashMap::new();
        for a in &agents {
            if let Some(fp) = &a.file {
                hop_timestamps.insert(fp.clone(), now);
            }
        }
        Self {
            id: id.into(),
            title: SharedString::from("acme/web"),
            branch: SharedString::from("feat/token-refresh"),
            files,
            agents,
            depth: 2,
            map_height: DEFAULT_MAP_H,
            isolated_agent: None,
            hovered_file: None,
            focused_file: None,
            measured_w: Rc::new(Cell::new(DEFAULT_MAP_W)),
            hop_timestamps,
            hop_tick: 0,
            started_at: now,
            loading: false,
            empty: false,
        }
    }

    /// Set the repository display name (`"acme/web"` by default).
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = title.into();
        self
    }

    /// Set the active Git branch pill (`"feat/token-refresh"` by default).
    pub fn with_branch(mut self, branch: impl Into<SharedString>) -> Self {
        self.branch = branch.into();
        self
    }

    /// Supply repository files (deduplicated by `path` in input order).
    pub fn with_files(mut self, files: Vec<RepoFile>) -> Self {
        let mut seen = Vec::new();
        let mut dedup = Vec::with_capacity(files.len());
        for f in files {
            if !seen.contains(&f.path) {
                seen.push(f.path.clone());
                dedup.push(f);
            }
        }
        self.files = dedup;
        self
    }

    /// Supply coding agents working on the repository.
    pub fn with_agents(mut self, agents: Vec<RepoAgent>) -> Self {
        let now = Instant::now();
        for a in &agents {
            if let Some(fp) = &a.file {
                self.hop_timestamps.insert(fp.clone(), now);
            }
        }
        self.agents = agents;
        self
    }

    /// Set how many folder path segments group files (`2` by default).
    pub fn with_depth(mut self, depth: usize) -> Self {
        self.depth = depth.max(1);
        self
    }

    /// Set the treemap panel height in pixels (`344.0` by default).
    pub fn with_height(mut self, height: f32) -> Self {
        self.map_height = height.max(220.0);
        self
    }

    /// Pre-isolate a specific agent id so only its touched files stay at full opacity.
    pub fn with_isolated_agent(mut self, agent_id: Option<SharedString>) -> Self {
        self.isolated_agent = agent_id;
        self
    }

    /// Toggle skeleton loading placeholders.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Force the empty repository state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        if empty {
            self.files.clear();
        }
        self
    }

    /// Return the deduplicated slice of repository files.
    pub fn files(&self) -> &[RepoFile] {
        &self.files
    }

    /// Return the slice of active agents.
    pub fn agents(&self) -> &[RepoAgent] {
        &self.agents
    }

    /// Return file paths currently edited simultaneously by `>= 2` agents (blinking orange conflicts).
    pub fn conflict_files(&self) -> Vec<SharedString> {
        let mut counts: HashMap<&str, usize> = HashMap::new();
        for a in &self.agents {
            if a.action == RepoAction::Edit
                && let Some(fp) = a.file.as_deref()
            {
                *counts.entry(fp).or_insert(0) += 1;
            }
        }
        self.files
            .iter()
            .filter(|f| counts.get(f.path.as_ref()).copied().unwrap_or(0) >= 2)
            .map(|f| f.path.clone())
            .collect()
    }

    /// Count files that have either historical edits or an active editing agent.
    pub fn changed_file_count(&self) -> usize {
        self.files
            .iter()
            .filter(|f| {
                !f.edits.is_empty()
                    || self
                        .agents
                        .iter()
                        .any(|a| a.action == RepoAction::Edit && a.file.as_ref() == Some(&f.path))
            })
            .count()
    }

    /// Toggle isolation of `agent_id`'s touched files and emit [`RepoMapEvent::AgentIsolated`].
    pub fn toggle_isolate_agent(&mut self, agent_id: &str, cx: &mut Context<Self>) {
        self.isolated_agent = if self.isolated_agent.as_deref() == Some(agent_id) {
            None
        } else {
            self.agents
                .iter()
                .find(|a| a.id.as_ref() == agent_id)
                .map(|a| a.id.clone())
        };
        cx.emit(RepoMapEvent::AgentIsolated(self.isolated_agent.clone()));
        cx.notify();
    }

    /// Move `agent_id` onto `target_path` with `action`, recording a commit edit on the file it left.
    pub fn move_agent(
        &mut self,
        agent_id: &str,
        target_path: &str,
        action: RepoAction,
        cx: &mut Context<Self>,
    ) {
        let prev_edit = self
            .agents
            .iter()
            .find(|a| a.id.as_ref() == agent_id)
            .and_then(|a| {
                (a.action == RepoAction::Edit)
                    .then(|| a.file.clone())
                    .flatten()
            });

        if let Some(prev_path) = prev_edit
            && prev_path.as_ref() != target_path
            && let Some(pf) = self.files.iter_mut().find(|f| f.path == prev_path)
            && !pf.edits.iter().any(|e| e.agent_id.as_ref() == agent_id)
        {
            pf.edits.push(RepoEdit::new(agent_id.to_string(), 24, 5));
        }

        if let Some(ag) = self.agents.iter_mut().find(|a| a.id.as_ref() == agent_id) {
            ag.file = Some(SharedString::from(target_path.to_string()));
            ag.action = action;
        }
        self.hop_timestamps
            .insert(SharedString::from(target_path.to_string()), Instant::now());
        cx.notify();
    }

    /// Advance agents one step across the repository so avatars hop and fill new tiles bottom-up.
    pub fn step_simulation(&mut self, cx: &mut Context<Self>) {
        if self.files.is_empty() || self.agents.is_empty() {
            return;
        }
        self.hop_tick += 1;
        let routes: [&[(&str, RepoAction)]; 3] = [
            &[
                ("src/lib/session.ts", RepoAction::Edit),
                ("src/lib/db.ts", RepoAction::Edit),
                ("src/app/api/auth/route.ts", RepoAction::Edit),
                ("src/lib/utils.ts", RepoAction::Read),
            ],
            &[
                ("src/lib/session.ts", RepoAction::Edit),
                ("src/components/Header.tsx", RepoAction::Edit),
                ("src/components/Dialog.tsx", RepoAction::Edit),
                ("src/app/settings/page.tsx", RepoAction::Edit),
            ],
            &[
                ("tests/session.test.ts", RepoAction::Edit),
                ("docs/auth.md", RepoAction::Read),
                ("tests/login.spec.ts", RepoAction::Edit),
                ("src/lib/db.ts", RepoAction::Edit),
            ],
        ];
        let agent_ids: Vec<SharedString> = self.agents.iter().map(|a| a.id.clone()).collect();
        for (idx, aid) in agent_ids.iter().enumerate() {
            let route = routes[idx % routes.len()];
            let (path, act) = route[self.hop_tick % route.len()];
            if self.files.iter().any(|f| f.path.as_ref() == path) {
                self.move_agent(aid.as_ref(), path, act, cx);
            }
        }
        cx.emit(RepoMapEvent::AgentsStepped);
        cx.notify();
    }

    fn agent_touched_file(&self, agent_id: &str, file: &RepoFile) -> bool {
        file.edits.iter().any(|e| e.agent_id.as_ref() == agent_id)
            || self
                .agents
                .iter()
                .any(|a| a.id.as_ref() == agent_id && a.file.as_ref() == Some(&file.path))
    }
}

impl Render for RepoMap {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let reduced = disabled_motion(cx);
        let elapsed_s = self.started_at.elapsed().as_secs_f32();
        let elapsed_ms = elapsed_s * 1_000.0;
        let panel_bg: Hsla = if p.dark {
            rgb(0x1c1c1f).into()
        } else {
            p.panel
        };
        let panel_border: Hsla = if p.dark {
            hsla(0., 0., 1., 0.09)
        } else {
            hsla(0., 0., 0., 0.06)
        };

        if self.loading {
            if !reduced {
                window.request_animation_frame();
            }
            let phase = (elapsed_s / 1.4).fract();
            return div()
                .id(self.id.clone())
                .w_full()
                .max_w(px(DEFAULT_MAP_W))
                .flex()
                .flex_col()
                .gap(px(12.))
                .child(pro_skeleton_bar(260., 20., 5., phase, cx))
                .child(pro_skeleton_bar(340., 26., 7., phase + 0.1, cx))
                .child(pro_skeleton_bar(
                    DEFAULT_MAP_W,
                    self.map_height,
                    10.,
                    phase + 0.2,
                    cx,
                ))
                .into_any_element();
        }

        if self.empty || self.files.is_empty() {
            return div()
                .id(self.id.clone())
                .w_full()
                .max_w(px(DEFAULT_MAP_W))
                .rounded(px(10.))
                .bg(panel_bg)
                .border_1()
                .border_color(panel_border)
                .child(pro_empty_state(
                    ProIcon::ProjectFolder,
                    tr(cx, "No repository files indexed", "暂无代码仓文件映射"),
                    tr(
                        cx,
                        "Provide repository file LOC and active agents to visualize codebase activity.",
                        "传入仓库文件行数与活跃智能体位置，即可可视化并发编辑版图与潜在冲突。",
                    ),
                    Some((
                        pro_eid(&self.id, "load-repo"),
                        SharedString::from(tr(cx, "Load acme/web repo", "加载 acme/web 示例")),
                        Box::new(|this: &mut Self, _, cx| {
                            this.empty = false;
                            this.files = default_repo_files();
                            this.agents = default_repo_agents();
                            cx.notify();
                        }),
                    )),
                    cx,
                ))
                .into_any_element();
        }

        let has_live = !reduced && !self.agents.is_empty();
        if has_live {
            window.request_animation_frame();
        }

        let conflicts = self.conflict_files();
        let conflict_n = conflicts.len();
        let changed_n = self.changed_file_count();
        let total_n = self.files.len();

        // Top Row 1: Repo icon + name + branch pill + conflict badge + changed file count
        let row_top = div()
            .w_full()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(8.))
            .child(pro_icon(ProIcon::Folder, 15., p.blue))
            .child(
                div()
                    .text_size(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .child(self.title.clone()),
            )
            .child(
                div()
                    .h(px(20.))
                    .px(px(7.))
                    .rounded(px(6.))
                    .bg(p.chip)
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .text_size(px(11.5))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.text)
                    .child(pro_icon(ProIcon::GitBranch, 11.5, p.muted))
                    .child(self.branch.clone()),
            )
            .child(
                div()
                    .ml_auto()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .when(conflict_n > 0, |d| {
                        let c_lbl = if conflict_n == 1 {
                            tr(cx, "1 conflict", "1 处冲突").to_string()
                        } else {
                            tr(
                                cx,
                                &format!("{conflict_n} conflicts"),
                                &format!("{conflict_n} 处冲突"),
                            )
                            .to_string()
                        };
                        d.child(
                            div()
                                .h(px(20.))
                                .px(px(7.))
                                .rounded(px(6.))
                                .bg(p.amber_soft)
                                .text_size(px(11.5))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.amber_ink)
                                .flex()
                                .items_center()
                                .child(c_lbl),
                        )
                    })
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(3.))
                            .text_size(px(12.))
                            .text_color(p.muted)
                            .child(
                                div()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .child(changed_n.to_string()),
                            )
                            .child(
                                tr(
                                    cx,
                                    &format!("of {total_n} files changed"),
                                    &format!("/ {total_n} 个文件已变更"),
                                )
                                .to_string(),
                            ),
                    ),
            );

        // Top Row 2: Agent filter pills + "Hop step" live simulation button
        let pill_card_bg: Hsla = if p.dark { rgb(0x222226).into() } else { p.card };
        let mut agent_strip = div()
            .w_full()
            .mt(px(8.))
            .mb(px(12.))
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(6.));

        for ag in &self.agents {
            let ac = agent_accent(ag.color_idx, &p);
            let touched_cnt = self
                .files
                .iter()
                .filter(|f| self.agent_touched_file(ag.id.as_ref(), f))
                .count();
            let pressed = self.isolated_agent.as_ref() == Some(&ag.id);
            let dimmed = self.isolated_agent.is_some() && !pressed;
            let aid = ag.id.clone();
            let hov_border = ac.opacity(0.55);

            agent_strip = agent_strip.child(
                div()
                    .id(pro_eid(&self.id, format!("ag-pill-{}", ag.id)))
                    .role(Role::Button)
                    .aria_label(format!("{} ({touched_cnt})", ag.display_name(cx)))
                    .tab_index(0)
                    .h(px(26.))
                    .pl(px(6.))
                    .pr(px(5.))
                    .rounded(px(8.))
                    .bg(pill_card_bg)
                    .border_1()
                    .border_color(if pressed {
                        ac.opacity(0.75)
                    } else {
                        p.line_strong
                    })
                    .opacity(if dimmed { 0.45 } else { 1.0 })
                    .cursor_pointer()
                    .hover(move |s| s.border_color(hov_border))
                    .focus_visible(move |s| s.border_2().border_color(p.focus))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.toggle_isolate_agent(aid.as_ref(), cx);
                    }))
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(pro_agent_avatar(
                        ag.id.as_ref(),
                        ac,
                        16.0,
                        ag.file.is_some() && ag.action == RepoAction::Edit,
                        elapsed_ms,
                        cx,
                    ))
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(ag.display_name(cx).to_string()),
                    )
                    .child(
                        div()
                            .min_w(px(18.))
                            .h(px(18.))
                            .px(px(5.))
                            .rounded(px(4.))
                            .bg(p.chip)
                            .text_size(px(11.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.text)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(touched_cnt.to_string()),
                    ),
            );
        }

        let step_hover = p.chip_hover;
        agent_strip = agent_strip.child(
            div()
                .id(pro_eid(&self.id, "hop-step"))
                .role(Role::Button)
                .aria_label(tr(
                    cx,
                    "Advance agents to next files",
                    "推进智能体跳转至下一批文件",
                ))
                .tab_index(0)
                .ml_auto()
                .h(px(26.))
                .px(px(8.))
                .rounded(px(7.))
                .bg(pill_card_bg)
                .border_1()
                .border_color(p.line_strong)
                .cursor_pointer()
                .hover(move |s| s.bg(step_hover))
                .focus_visible(move |s| s.border_2().border_color(p.focus))
                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                    this.step_simulation(cx);
                }))
                .flex()
                .items_center()
                .gap(px(5.))
                .text_size(px(11.5))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.text)
                .child(pro_icon(ProIcon::Play, 11., p.muted))
                .child(tr(cx, "Step agents", "推进跳转")),
        );

        // Compute Treemap Layout
        let map_w = self.measured_w.get();
        let map_h = self.map_height;
        let layout = compute_repo_layout(&self.files, self.depth, map_w, map_h);

        let folder_bg: Hsla = if p.dark {
            hsla(0., 0., 1., 0.026)
        } else {
            hsla(0., 0., 0., 0.022)
        };
        let file_base_bg: Hsla = if p.dark {
            hsla(0., 0., 1., 0.05)
        } else {
            hsla(0., 0., 0., 0.035)
        };
        let file_base_edge: Hsla = if p.dark {
            hsla(0., 0., 1., 0.075)
        } else {
            hsla(0., 0., 0., 0.065)
        };
        let add_c: Hsla = if p.dark {
            rgb(0x6fd398).into()
        } else {
            rgb(0x0b7d3e).into()
        };
        let del_c: Hsla = if p.dark {
            rgb(0xff8a7a).into()
        } else {
            rgb(0xc4320a).into()
        };

        let width_cell = self.measured_w.clone();
        let entity = cx.entity().downgrade();

        let width_sensor = canvas(
            move |bounds, window, cx| {
                let w = f32::from(bounds.size.width);
                if w > 80.0 && (w - width_cell.get()).abs() > 1.5 {
                    width_cell.set(w);
                    if let Some(ent) = entity.upgrade() {
                        ent.update(cx, |_, cx| cx.notify());
                    }
                    window.refresh();
                }
            },
            |_, _, _, _| (),
        )
        .absolute()
        .inset_0();

        let mut map_area = div()
            .id(pro_eid(&self.id, "treemap"))
            .role(Role::Group)
            .aria_label(tr(
                cx,
                "Repository treemap — arrow keys inspect files, Esc clears",
                "代码仓树图 — 方向键巡检文件，Esc 关闭详情",
            ))
            .tab_index(0)
            .relative()
            .w_full()
            .h(px(map_h))
            .rounded(px(10.))
            .bg(panel_bg)
            .border_1()
            .border_color(panel_border)
            .when(!p.dark, |d| d.shadow_sm())
            .focus_visible(move |s| s.border_2().border_color(p.focus))
            .on_hover(cx.listener(|this, inside: &bool, _, cx| {
                if !*inside && this.hovered_file.is_some() {
                    this.hovered_file = None;
                    cx.notify();
                }
            }))
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _, cx| {
                let len = this.files.len();
                if len == 0 {
                    return;
                }
                let cur = this.focused_file.unwrap_or(0);
                match ev.keystroke.key.as_str() {
                    "left" | "arrowleft" | "up" | "arrowup" => {
                        let next = cur.saturating_sub(1);
                        this.focused_file = Some(next);
                        cx.emit(RepoMapEvent::FileSelected(
                            this.files.get(next).map(|f| f.path.clone()),
                        ));
                        cx.stop_propagation();
                        cx.notify();
                    }
                    "right" | "arrowright" | "down" | "arrowdown" => {
                        let next = (cur + 1).min(len - 1);
                        this.focused_file = Some(next);
                        cx.emit(RepoMapEvent::FileSelected(
                            this.files.get(next).map(|f| f.path.clone()),
                        ));
                        cx.stop_propagation();
                        cx.notify();
                    }
                    "home" => {
                        this.focused_file = Some(0);
                        cx.stop_propagation();
                        cx.notify();
                    }
                    "end" => {
                        this.focused_file = Some(len - 1);
                        cx.stop_propagation();
                        cx.notify();
                    }
                    "escape" => {
                        this.focused_file = None;
                        this.hovered_file = None;
                        if this.isolated_agent.is_some() {
                            this.isolated_agent = None;
                            cx.emit(RepoMapEvent::AgentIsolated(None));
                        }
                        cx.stop_propagation();
                        cx.notify();
                    }
                    _ => {}
                }
            }))
            .child(width_sensor);

        // 1. Folder group boxes
        for folder in &layout.folders {
            let folder_lit = match &self.isolated_agent {
                None => true,
                Some(aid) => folder
                    .file_indices
                    .iter()
                    .any(|&fi| self.agent_touched_file(aid.as_ref(), &self.files[fi])),
            };
            map_area = map_area.child(
                div()
                    .absolute()
                    .left(px(folder.rect.x))
                    .top(px(folder.rect.y))
                    .w(px(folder.rect.w))
                    .h(px(folder.rect.h))
                    .rounded(px(8.))
                    .bg(folder_bg)
                    .opacity(if folder_lit { 1.0 } else { 0.36 })
                    .child(
                        div()
                            .px(px(7.))
                            .pt(px(2.5))
                            .text_size(px(11.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.muted)
                            .truncate()
                            .child(folder.name.clone()),
                    ),
            );
        }

        // 2. Individual file tiles with bottom-up agent fill, sweeping sheen, dashed read outline, or orange conflict blink
        let active_tip_idx = self.hovered_file.or(self.focused_file);
        for (fi, file) in self.files.iter().enumerate() {
            let r = layout.tile_rects[fi];
            if r.w < 6.0 || r.h < 6.0 {
                continue;
            }
            let editors: Vec<&RepoAgent> = self
                .agents
                .iter()
                .filter(|a| a.action == RepoAction::Edit && a.file.as_ref() == Some(&file.path))
                .collect();
            let readers: Vec<&RepoAgent> = self
                .agents
                .iter()
                .filter(|a| a.action == RepoAction::Read && a.file.as_ref() == Some(&file.path))
                .collect();

            let is_conflict = editors.len() >= 2;
            let active_editor = editors.first().copied();
            let active_reader = readers.first().copied();
            let last_edit_agent = file
                .edits
                .last()
                .and_then(|ed| self.agents.iter().find(|a| a.id == ed.agent_id));

            let lit = match &self.isolated_agent {
                None => true,
                Some(aid) => self.agent_touched_file(aid.as_ref(), file),
            };
            let is_inspected = active_tip_idx == Some(fi);

            // Compute bottom-up fill progress when an editor hops onto a file
            let fill_prog = if active_editor.is_some() && !reduced {
                self.hop_timestamps.get(&file.path).map_or(1.0, |ts| {
                    pro_ease((ts.elapsed().as_secs_f32() / 0.78).clamp(0.08, 1.0))
                })
            } else {
                1.0
            };

            let conflict_blink = if is_conflict && !reduced {
                0.5 + 0.5 * ((elapsed_s / 1.45) * TAU).sin()
            } else {
                1.0
            };

            let tint_col = active_editor
                .or(last_edit_agent)
                .map(|ag| agent_accent(ag.color_idx, &p));
            let read_col = active_reader.map(|ag| agent_accent(ag.color_idx, &p));
            let is_editing = active_editor.is_some() && !is_conflict;
            let sheen_phase = ((elapsed_s + fi as f32 * 0.19) / 2.2).fract();
            let dark = p.dark;

            let tile_canvas = canvas(
                |_, _, _| (),
                move |bounds, _, window, _| {
                    let rad = px(6.0_f32
                        .min(f32::from(bounds.size.width) * 0.25)
                        .min(f32::from(bounds.size.height) * 0.25));

                    // Base neutral tile
                    let base_bg = if is_conflict {
                        pro_mix_color(
                            file_base_bg,
                            p.orange.opacity(if dark { 0.28 } else { 0.22 }),
                            conflict_blink,
                        )
                    } else {
                        file_base_bg
                    };
                    let edge_c = if is_inspected {
                        p.focus
                    } else if is_conflict {
                        p.orange.opacity(0.75)
                    } else if let Some(tc) = tint_col {
                        tc.opacity(if is_editing { 0.52 } else { 0.28 })
                    } else {
                        file_base_edge
                    };

                    window.paint_quad(quad(
                        bounds,
                        Corners::all(rad),
                        base_bg,
                        Edges::all(px(if is_inspected {
                            1.5
                        } else if read_col.is_some() && tint_col.is_none() {
                            0.0
                        } else {
                            1.0
                        })),
                        edge_c,
                        BorderStyle::Solid,
                    ));

                    // Bottom-up agent color fill
                    if !is_conflict && let Some(tc) = tint_col {
                        let bh = f32::from(bounds.size.height);
                        let fh = (bh * fill_prog).clamp(0.0, bh);
                        if fh > 0.5 {
                            let fill_rect = Bounds::new(
                                point(bounds.left(), bounds.bottom() - px(fh)),
                                size(bounds.size.width, px(fh)),
                            );
                            let alpha = if is_editing {
                                if dark { 0.22 } else { 0.16 }
                            } else if dark {
                                0.14
                            } else {
                                0.11
                            };
                            window.paint_quad(fill(fill_rect, tc.opacity(alpha)).corner_radii(rad));
                        }
                    }

                    // Sweeping sheen while actively edited
                    if is_editing && !reduced {
                        let bw = f32::from(bounds.size.width);
                        let band = (bw * 0.45).clamp(14.0, 56.0);
                        let bx = -band + sheen_phase * (bw + band * 1.4);
                        let cl_l = bx.max(0.0);
                        let cl_r = (bx + band).min(bw);
                        if cl_r > cl_l + 1.0 {
                            let sr = Bounds::new(
                                point(bounds.left() + px(cl_l), bounds.top()),
                                size(px(cl_r - cl_l), bounds.size.height),
                            );
                            window.paint_quad(
                                fill(sr, hsla(0., 0., 1., if dark { 0.08 } else { 0.28 }))
                                    .corner_radii(rad),
                            );
                        }
                    }

                    // Dashed perimeter for Read-only agent inspection!
                    if let Some(rc) = read_col
                        && !is_conflict
                    {
                        let x0 = f32::from(bounds.left()) + 1.0;
                        let y0 = f32::from(bounds.top()) + 1.0;
                        let x1 = f32::from(bounds.right()) - 1.0;
                        let y1 = f32::from(bounds.bottom()) - 1.0;
                        let dash_c = rc.opacity(0.68);
                        let mut x = x0;
                        while x < x1 {
                            let ex = (x + 4.0).min(x1);
                            let mut top_s = PathBuilder::stroke(px(1.2));
                            top_s.move_to(point(px(x), px(y0)));
                            top_s.line_to(point(px(ex), px(y0)));
                            if let Ok(path) = top_s.build() {
                                window.paint_path(path, dash_c);
                            }
                            let mut bot_s = PathBuilder::stroke(px(1.2));
                            bot_s.move_to(point(px(x), px(y1)));
                            bot_s.line_to(point(px(ex), px(y1)));
                            if let Ok(path) = bot_s.build() {
                                window.paint_path(path, dash_c);
                            }
                            x += 7.5;
                        }
                        let mut y = y0;
                        while y < y1 {
                            let ey = (y + 4.0).min(y1);
                            let mut l_s = PathBuilder::stroke(px(1.2));
                            l_s.move_to(point(px(x0), px(y)));
                            l_s.line_to(point(px(x0), px(ey)));
                            if let Ok(path) = l_s.build() {
                                window.paint_path(path, dash_c);
                            }
                            let mut r_s = PathBuilder::stroke(px(1.2));
                            r_s.move_to(point(px(x1), px(y)));
                            r_s.line_to(point(px(x1), px(ey)));
                            if let Ok(path) = r_s.build() {
                                window.paint_path(path, dash_c);
                            }
                            y += 7.5;
                        }
                    }
                },
            )
            .absolute()
            .inset_0();

            let total_added: u32 = file.edits.iter().map(|e| e.added).sum();
            let total_removed: u32 = file.edits.iter().map(|e| e.removed).sum();

            let show_name = r.w >= 42.0 && r.h >= 22.0;
            let show_sub = r.w >= 54.0 && r.h >= 38.0;
            let name_fg = if is_conflict { p.amber_ink } else { p.ink };

            let sub_el = if is_conflict {
                div()
                    .text_size(px(11.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.amber_ink)
                    .child(tr(cx, "Conflict", "编辑冲突"))
                    .into_any_element()
            } else if total_added > 0 || total_removed > 0 {
                div()
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .text_size(px(11.))
                    .font_weight(FontWeight::MEDIUM)
                    .child(div().text_color(add_c).child(format!("+{total_added}")))
                    .child(div().text_color(del_c).child(format!("−{total_removed}")))
                    .into_any_element()
            } else {
                div()
                    .text_size(px(11.))
                    .text_color(p.muted)
                    .child(format!("{} {}", file.lines, tr(cx, "lines", "行")))
                    .into_any_element()
            };

            let fpath_click = file.path.clone();
            let tile_el = div()
                .id(pro_eid(&self.id, format!("tile-{fi}")))
                .role(Role::Button)
                .aria_label(format!("{}, {} lines", file.path, file.lines))
                .absolute()
                .left(px(r.x))
                .top(px(r.y))
                .w(px(r.w))
                .h(px(r.h))
                .rounded(px(6.))
                .overflow_hidden()
                .opacity(if lit { 1.0 } else { 0.32 })
                .cursor_pointer()
                .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                    if *inside {
                        this.hovered_file = Some(fi);
                    } else if this.hovered_file == Some(fi) {
                        this.hovered_file = None;
                    }
                    cx.notify();
                }))
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.focused_file = if this.focused_file == Some(fi) {
                        None
                    } else {
                        Some(fi)
                    };
                    cx.emit(RepoMapEvent::FileSelected(
                        this.focused_file.map(|_| fpath_click.clone()),
                    ));
                    cx.notify();
                }))
                .child(tile_canvas)
                .when(show_name, |d| {
                    d.child(
                        div()
                            .relative()
                            .px(px(7.))
                            .pt(px(5.))
                            .flex()
                            .flex_col()
                            .gap(px(1.))
                            .child(
                                div()
                                    .text_size(px(11.5))
                                    .line_height(px(15.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(name_fg)
                                    .truncate()
                                    .child(file.basename().to_string()),
                            )
                            .when(show_sub, |d2| d2.child(sub_el)),
                    )
                });

            map_area = map_area.child(tile_el);
        }

        // 3. Smoothly hopping Agent Avatar Pins (pro_spring_f32 X/Y coordinates!)
        let mut slot_per_file: HashMap<usize, usize> = HashMap::new();
        for ag in &self.agents {
            let Some(fpath) = &ag.file else {
                continue;
            };
            let Some(fi) = self.files.iter().position(|f| &f.path == fpath) else {
                continue;
            };
            let r = layout.tile_rects[fi];
            if r.w < 16.0 || r.h < 16.0 {
                continue;
            }
            let slot = *slot_per_file.entry(fi).and_modify(|n| *n += 1).or_insert(0);
            let target_x = (r.x + 6.0 + slot as f32 * 24.0)
                .min(r.x + r.w - 24.0)
                .max(r.x + 2.0);
            let target_y = (r.y + r.h - 26.0).max(r.y + 4.0);

            let pin_x = pro_spring_f32(
                pro_eid(&self.id, format!("pinx-{}", ag.id)),
                target_x,
                0.44,
                window,
                cx,
            );
            let pin_y = pro_spring_f32(
                pro_eid(&self.id, format!("piny-{}", ag.id)),
                target_y,
                0.44,
                window,
                cx,
            );
            let ac = agent_accent(ag.color_idx, &p);
            let dimmed = self
                .isolated_agent
                .as_ref()
                .is_some_and(|iso| iso != &ag.id);

            map_area = map_area.child(
                div()
                    .absolute()
                    .left(px(pin_x))
                    .top(px(pin_y))
                    .opacity(if dimmed { 0.35 } else { 1.0 })
                    .child(pro_agent_avatar(
                        ag.id.as_ref(),
                        ac,
                        22.0,
                        ag.action == RepoAction::Edit,
                        elapsed_ms,
                        cx,
                    )),
            );
        }

        // 4. Floating File Hover / Focus Inspector Popover
        let tip_overlay = active_tip_idx.and_then(|fi| {
            let file = self.files.get(fi)?;
            let r = layout.tile_rects.get(fi).copied()?;
            let cx_tile = r.x + r.w * 0.5;
            let left_px = (cx_tile - TIP_W * 0.5).clamp(8.0, (map_w - TIP_W - 8.0).max(8.0));
            let place_below = r.y < 110.0;

            let active_here: Vec<(&RepoAgent, Hsla)> = self
                .agents
                .iter()
                .filter(|a| a.file.as_ref() == Some(&file.path))
                .map(|a| (a, agent_accent(a.color_idx, &p)))
                .collect();
            let editors_n = active_here
                .iter()
                .filter(|(a, _)| a.action == RepoAction::Edit)
                .count();

            let tip_bg: Hsla = if p.dark { rgb(0x26262a).into() } else { p.card };
            let mut card = div()
                .absolute()
                .left(px(left_px))
                .when(place_below, |d| {
                    d.top(px((r.y + r.h + 8.0).min(map_h - 32.0)))
                })
                .when(!place_below, |d| d.bottom(px(map_h - r.y + 8.0)))
                .w(px(TIP_W))
                .px(px(12.))
                .py(px(10.))
                .rounded(px(10.))
                .bg(tip_bg)
                .border_1()
                .border_color(p.line_strong)
                .shadow_lg()
                .flex()
                .flex_col()
                .gap(px(5.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(8.))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .font_family("Menlo")
                                .text_size(px(11.5))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.strong)
                                .child(file.path.clone()),
                        )
                        .child(
                            div()
                                .flex_none()
                                .text_size(px(11.))
                                .text_color(p.muted)
                                .child(format!("{} LOC", file.lines)),
                        ),
                );

            for (ag, ac) in &active_here {
                let mode_lbl = match ag.action {
                    RepoAction::Edit => tr(cx, "editing", "正在编辑"),
                    RepoAction::Read => tr(cx, "reading", "正在读取"),
                };
                let task_suffix = ag
                    .display_task(cx)
                    .map(|t| format!(" · {t}"))
                    .unwrap_or_default();
                card =
                    card.child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .text_size(px(11.5))
                            .child(div().size(px(6.)).rounded_full().bg(*ac))
                            .child(div().flex_1().min_w_0().truncate().text_color(p.ink).child(
                                format!("{}: {mode_lbl}{task_suffix}", ag.display_name(cx)),
                            )),
                    );
            }

            for ed in &file.edits {
                let who = self
                    .agents
                    .iter()
                    .find(|a| a.id == ed.agent_id)
                    .map_or(ed.agent_id.as_ref(), |a| a.display_name(cx));
                card = card.child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .text_size(px(11.5))
                        .child(div().text_color(p.text).child(who.to_string()))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(5.))
                                .font_weight(FontWeight::MEDIUM)
                                .child(div().text_color(add_c).child(format!("+{}", ed.added)))
                                .child(div().text_color(del_c).child(format!("−{}", ed.removed))),
                        ),
                );
            }

            if editors_n >= 2 {
                card = card.child(
                    div()
                        .pt(px(2.))
                        .text_size(px(11.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.amber_ink)
                        .child(tr(
                            cx,
                            "Write conflict — two agents editing this file simultaneously",
                            "并发写入冲突 — 两个智能体正同时修改此文件",
                        )),
                );
            }
            Some(card)
        });

        let map_with_tip = div()
            .relative()
            .w_full()
            .child(map_area)
            .children(tip_overlay);

        div()
            .id(self.id.clone())
            .w_full()
            .flex()
            .justify_center()
            .child(
                div()
                    .w_full()
                    .max_w(px(DEFAULT_MAP_W))
                    .flex()
                    .flex_col()
                    .child(row_top)
                    .child(agent_strip)
                    .child(map_with_tip),
            )
            .into_any_element()
    }
}

fn default_repo_files() -> Vec<RepoFile> {
    vec![
        // src/app
        RepoFile::new("src/app/layout.tsx", 140),
        RepoFile::new("src/app/page.tsx", 220),
        RepoFile::new("src/app/login/page.tsx", 180).with_edit("ui", 19, 7),
        RepoFile::new("src/app/settings/page.tsx", 260),
        RepoFile::new("src/app/api/auth/route.ts", 120),
        // src/components
        RepoFile::new("src/components/LoginForm.tsx", 210).with_edit("ui", 36, 14),
        RepoFile::new("src/components/Header.tsx", 150),
        RepoFile::new("src/components/Sidebar.tsx", 190),
        RepoFile::new("src/components/Button.tsx", 90),
        RepoFile::new("src/components/Dialog.tsx", 170),
        RepoFile::new("src/components/Toast.tsx", 80).with_edit("ui", 12, 0),
        // src/lib
        RepoFile::new("src/lib/auth.ts", 320).with_edit("auth", 64, 12),
        RepoFile::new("src/lib/session.ts", 240),
        RepoFile::new("src/lib/api.ts", 280).with_edit("auth", 28, 6),
        RepoFile::new("src/lib/db.ts", 160),
        RepoFile::new("src/lib/utils.ts", 110),
        // tests
        RepoFile::new("tests/auth.test.ts", 230).with_edit("test", 58, 3),
        RepoFile::new("tests/session.test.ts", 150),
        RepoFile::new("tests/login.spec.ts", 170),
        // docs
        RepoFile::new("docs/auth.md", 130),
        RepoFile::new("docs/setup.md", 90),
        // root
        RepoFile::new("package.json", 70),
        RepoFile::new("README.md", 110),
    ]
}

fn default_repo_agents() -> Vec<RepoAgent> {
    vec![
        RepoAgent::new("auth", "Auth agent")
            .with_name_zh("鉴权智能体")
            .with_color_idx(0)
            .with_target("src/lib/session.ts", RepoAction::Edit)
            .with_task("Adding refresh token rotation")
            .with_task_zh("实现刷新令牌轮换机制"),
        RepoAgent::new("ui", "UI agent")
            .with_name_zh("界面智能体")
            .with_color_idx(1)
            .with_target("src/lib/session.ts", RepoAction::Edit)
            .with_task("Reading session expiry hook")
            .with_task_zh("接入会话过期提醒钩子"),
        RepoAgent::new("test", "Test agent")
            .with_name_zh("测试智能体")
            .with_color_idx(2)
            .with_target("tests/session.test.ts", RepoAction::Edit)
            .with_task("Writing session renewal assertions")
            .with_task_zh("编写会话续期集成断言"),
        RepoAgent::new("docs", "Docs agent")
            .with_name_zh("文档智能体")
            .with_color_idx(4)
            .with_target("docs/auth.md", RepoAction::Read)
            .with_task("Checking OAuth token spec")
            .with_task_zh("核对 OAuth 令牌规范文档"),
    ]
}

#[cfg(test)]
mod tests {
    use super::{RepoAction, RepoMap};
    use gpui::{AppContext as _, TestAppContext};

    #[gpui::test]
    fn detects_concurrent_edit_conflict_and_resolves_when_agent_moves(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let view = cx.new(|cx| RepoMap::new("test-rpm", cx));

        view.update(cx, |v, cx| {
            assert_eq!(v.files().len(), 23);
            assert_eq!(
                v.conflict_files(),
                vec!["src/lib/session.ts"],
                "Auth agent and UI agent both edit session.ts initially"
            );

            // Moving UI agent away resolves the conflict and commits an edit on session.ts
            v.move_agent("ui", "src/components/Header.tsx", RepoAction::Edit, cx);
            assert!(v.conflict_files().is_empty());

            // Isolating Auth agent toggles cleanly
            v.toggle_isolate_agent("auth", cx);
            assert_eq!(v.isolated_agent.as_deref(), Some("auth"));
            v.toggle_isolate_agent("auth", cx);
            assert_eq!(v.isolated_agent, None);
        });
    }
}
