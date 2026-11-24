//! Multi-branch coding-agent conflict matrix with line-overlap connectors and resolution radio cards.
//!
//! Displays one row per modified file and one column per parallel coding agent ([`pro_agent_avatar`]),
//! draws horizontal [`PathBuilder`] connector bars with a live pulse between agents whose edits touch
//! overlapping line ranges, presents one-sentence resolution radio cards per conflicting branch, and
//! enables `"Merge branches"` once every overlap is settled.

use crate::{
    components::pro_chrome::{
        ProBtnStyle, ProPalette, pro_agent_avatar, pro_btn, pro_check_draw, pro_eid,
        pro_empty_state, pro_mark_selected, pro_pill_indicator, pro_popover_presence,
        pro_rolling_number, pro_selection_sensor, pro_skeleton_bar, pro_spring_f32, pro_tactile,
        pro_view_transition, tr,
    },
    foundation::Theme,
    primitives::{
        family::disabled_motion,
        pro_icons::{ProIcon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::collections::HashMap;
use web_time::Instant;

const RADAR_ICON_SVG: &str = r#"<circle cx="8" cy="8" r="6.4" stroke="currentColor" stroke-width="1.3" opacity="0.45"></circle><circle cx="8" cy="8" r="3.3" stroke="currentColor" stroke-width="1.3" opacity="0.7"></circle><path d="M8 8 12.6 3.4" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"></path><circle cx="11.2" cy="10.6" r="1.3" fill="currentColor"></circle>"#;

/// Events emitted by [`ConflictRadar`].
#[derive(Clone, Debug, PartialEq)]
pub enum ConflictRadarEvent {
    /// User picked `agent_id`'s approach to resolve an overlapping file edit at `path`.
    Resolved {
        path: SharedString,
        agent_id: SharedString,
    },
    /// User focused a different file row in the radar matrix.
    SelectedFile(SharedString),
    /// User clicked `Merge branches` after settling every conflict.
    MergeBranches(HashMap<SharedString, SharedString>),
    /// User reset resolutions to review conflicts again.
    Reset,
}

/// Execution state of a branch agent column in [`ConflictRadar`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RadarAgentStatus {
    /// Still writing commits on its branch.
    Running,
    /// Branch changes complete and ready for conflict resolution / merge.
    #[default]
    Done,
}

/// One parallel coding agent working on its own branch.
#[derive(Clone, Debug, PartialEq)]
pub struct RadarAgent {
    /// Stable agent identifier referenced by [`RadarEdit::agent_id`].
    pub id: SharedString,
    /// Short branch or agent name (`"Checkout"`, `"Stripe v2"`, `"Retries"`).
    pub name: SharedString,
    /// Optional Chinese branch/agent label.
    pub name_zh: Option<SharedString>,
    /// Model or Git branch subtitle shown in the column hover KPI popover.
    pub model: Option<SharedString>,
    /// Pixel avatar accent color for [`pro_agent_avatar`].
    pub color: Option<Hsla>,
    /// Whether the agent is still running or finished.
    pub status: RadarAgentStatus,
}

impl RadarAgent {
    /// Create a [`RadarAgent`] (`Done` by default).
    pub fn new(id: impl Into<SharedString>, name: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            name_zh: None,
            model: None,
            color: None,
            status: RadarAgentStatus::Done,
        }
    }

    /// Set localized Chinese name.
    pub fn with_name_zh(mut self, name_zh: impl Into<SharedString>) -> Self {
        self.name_zh = Some(name_zh.into());
        self
    }

    /// Set model and branch metadata (`"Claude Sonnet · feat/checkout"`).
    pub fn with_model(mut self, model: impl Into<SharedString>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set avatar accent color.
    pub fn with_color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }

    /// Set execution status (`Running` or `Done`).
    pub fn with_status(mut self, status: RadarAgentStatus) -> Self {
        self.status = status;
        self
    }

    fn display_name<'a>(&'a self, cx: &App) -> &'a str {
        match &self.name_zh {
            Some(zh) => tr(cx, self.name.as_ref(), zh.as_ref()),
            None => self.name.as_ref(),
        }
    }
}

/// One agent's line-range modification inside a [`RadarFile`].
#[derive(Clone, Debug, PartialEq)]
pub struct RadarEdit {
    /// Authoring agent id.
    pub agent_id: SharedString,
    /// Inclusive modified line range `(start_line, end_line)`.
    pub lines: (u32, u32),
    /// One-sentence explanation of this agent's implementation approach.
    pub summary: Option<SharedString>,
    /// Optional Chinese translation of `summary`.
    pub summary_zh: Option<SharedString>,
    /// Whether the agent is actively writing to these lines right now.
    pub active: bool,
}

impl RadarEdit {
    /// Create a [`RadarEdit`] touching inclusive `start_line..=end_line`.
    pub fn new(agent_id: impl Into<SharedString>, start_line: u32, end_line: u32) -> Self {
        let lo = start_line.min(end_line);
        let hi = start_line.max(end_line);
        Self {
            agent_id: agent_id.into(),
            lines: (lo, hi),
            summary: None,
            summary_zh: None,
            active: false,
        }
    }

    /// Provide a one-sentence resolution summary for conflict comparison cards.
    pub fn with_summary(
        mut self,
        summary: impl Into<SharedString>,
        summary_zh: impl Into<SharedString>,
    ) -> Self {
        self.summary = Some(summary.into());
        self.summary_zh = Some(summary_zh.into());
        self
    }

    /// Mark this edit as actively streaming.
    pub fn with_active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    fn display_summary<'a>(&'a self, cx: &App) -> Option<&'a str> {
        match (&self.summary, &self.summary_zh) {
            (Some(en), Some(zh)) => Some(tr(cx, en.as_ref(), zh.as_ref())),
            (Some(en), None) => Some(en.as_ref()),
            _ => None,
        }
    }
}

/// One repository file row in [`ConflictRadar`].
#[derive(Clone, Debug, PartialEq)]
pub struct RadarFile {
    /// Repository-relative file path (`"src/checkout/payment.ts"`).
    pub path: SharedString,
    /// Per-agent edits on this file.
    pub edits: Vec<RadarEdit>,
}

impl RadarFile {
    /// Create a [`RadarFile`] with `path` and `edits`.
    pub fn new(path: impl Into<SharedString>, edits: Vec<RadarEdit>) -> Self {
        Self {
            path: path.into(),
            edits,
        }
    }
}

/// Return sorted column indices of `agents` whose edits in `edits` overlap in line range with at least one other agent.
pub fn overlapping_agent_columns(agents: &[RadarAgent], edits: &[RadarEdit]) -> Vec<usize> {
    let mut cols = Vec::new();
    for (i, a) in edits.iter().enumerate() {
        let overlaps_any = edits.iter().enumerate().any(|(j, b)| {
            i != j && a.agent_id != b.agent_id && a.lines.0 <= b.lines.1 && b.lines.0 <= a.lines.1
        });
        if overlaps_any
            && let Some(col_idx) = agents.iter().position(|ag| ag.id == a.agent_id)
            && !cols.contains(&col_idx)
        {
            cols.push(col_idx);
        }
    }
    cols.sort_unstable();
    cols
}

/// Multi-branch coding-agent conflict matrix with [`PathBuilder`] overlap connectors and resolution radio cards.
pub struct ConflictRadar {
    id: ElementId,
    title: SharedString,
    title_zh: Option<SharedString>,
    agents: Vec<RadarAgent>,
    files: Vec<RadarFile>,
    resolved: HashMap<SharedString, SharedString>,
    selected_path: Option<SharedString>,
    hovered_agent_col: Option<usize>,
    hovered_cell: Option<(usize, usize)>,
    merged: bool,
    loading: bool,
    empty: bool,
    started_at: Instant,
}

impl EventEmitter<ConflictRadarEvent> for ConflictRadar {}

impl ConflictRadar {
    /// Create a [`ConflictRadar`] matrix preloaded with 3 parallel checkout branches and 2 overlapping file conflicts.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let agents = default_agents();
        let files = default_files();
        let first_conflict = files
            .iter()
            .find(|f| overlapping_agent_columns(&agents, &f.edits).len() >= 2)
            .map(|f| f.path.clone());
        Self {
            id: id.into(),
            title: SharedString::from("Parallel work"),
            title_zh: Some(SharedString::from("多分支并行冲突雷达")),
            agents,
            files,
            resolved: HashMap::new(),
            selected_path: first_conflict,
            hovered_agent_col: None,
            hovered_cell: None,
            merged: false,
            loading: false,
            empty: false,
            started_at: Instant::now(),
        }
    }

    /// Set header title (`"Parallel work"` default).
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = title.into();
        self.title_zh = None;
        self
    }

    /// Set localized Chinese header title.
    pub fn with_title_zh(mut self, title_zh: impl Into<SharedString>) -> Self {
        self.title_zh = Some(title_zh.into());
        self
    }

    /// Configure the branch agents (columns).
    pub fn with_agents(mut self, agents: Vec<RadarAgent>) -> Self {
        self.agents = agents;
        self.sync_selected_conflict();
        self
    }

    /// Configure the touched repository files (rows).
    pub fn with_files(mut self, files: Vec<RadarFile>) -> Self {
        self.files = files;
        self.sync_selected_conflict();
        self
    }

    /// Pre-seed chosen agent resolutions (`path -> agent_id`).
    pub fn with_resolved(mut self, resolved: HashMap<SharedString, SharedString>) -> Self {
        self.resolved = resolved;
        self
    }

    /// Show skeleton placeholder state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Show empty state when no parallel branches are active.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Return current resolution picks (`path -> agent_id`).
    pub fn resolved(&self) -> &HashMap<SharedString, SharedString> {
        &self.resolved
    }

    /// Return count of conflicting files that have not yet been resolved.
    pub fn unresolved_conflict_count(&self) -> usize {
        self.files
            .iter()
            .filter(|f| {
                overlapping_agent_columns(&self.agents, &f.edits).len() >= 2
                    && !self.resolved.contains_key(&f.path)
            })
            .count()
    }

    /// Return true when every overlapping file has a chosen agent and all agents have finished.
    pub fn can_merge(&self) -> bool {
        !self.merged
            && !self.files.is_empty()
            && self
                .agents
                .iter()
                .all(|a| a.status == RadarAgentStatus::Done)
            && self.unresolved_conflict_count() == 0
    }

    /// Pick `agent_id`'s implementation for `path`, advancing selection to any remaining unresolved conflict.
    pub fn resolve_conflict(&mut self, path: &str, agent_id: &str, cx: &mut Context<Self>) -> bool {
        let Some(file) = self.files.iter().find(|f| f.path.as_ref() == path) else {
            return false;
        };
        let cols = overlapping_agent_columns(&self.agents, &file.edits);
        if cols.len() < 2 {
            return false;
        }
        let key = file.path.clone();
        let val = SharedString::from(agent_id.to_string());
        self.resolved.insert(key.clone(), val.clone());
        self.merged = false;

        // Auto-focus next unresolved conflict if one exists
        if let Some(next_open) = self.files.iter().find(|f| {
            overlapping_agent_columns(&self.agents, &f.edits).len() >= 2
                && !self.resolved.contains_key(&f.path)
        }) {
            self.selected_path = Some(next_open.path.clone());
        }

        cx.emit(ConflictRadarEvent::Resolved {
            path: key,
            agent_id: val,
        });
        cx.notify();
        true
    }

    /// Execute branch merge once `can_merge()` is true.
    pub fn merge_branches(&mut self, cx: &mut Context<Self>) -> bool {
        if !self.can_merge() {
            return false;
        }
        self.merged = true;
        cx.emit(ConflictRadarEvent::MergeBranches(self.resolved.clone()));
        cx.notify();
        true
    }

    /// Clear resolutions and merged flag to inspect conflicts again.
    pub fn reset_resolutions(&mut self, cx: &mut Context<Self>) {
        self.resolved.clear();
        self.merged = false;
        self.sync_selected_conflict();
        cx.emit(ConflictRadarEvent::Reset);
        cx.notify();
    }

    fn sync_selected_conflict(&mut self) {
        self.selected_path = self
            .files
            .iter()
            .find(|f| overlapping_agent_columns(&self.agents, &f.edits).len() >= 2)
            .map(|f| f.path.clone());
    }
}

impl Render for ConflictRadar {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let reduced = disabled_motion(cx) || Theme::global(cx).reduced_motion;
        let elapsed_secs = if reduced {
            0.0
        } else {
            self.started_at.elapsed().as_secs_f32()
        };
        let elapsed_ms = elapsed_secs * 1000.0;

        let unresolved_n = self.unresolved_conflict_count();
        let any_running = self
            .agents
            .iter()
            .any(|a| a.status == RadarAgentStatus::Running);

        if !reduced && (self.loading || unresolved_n > 0 || any_running) {
            w.request_animation_frame();
        }

        let card_bg: Hsla = if p.dark {
            rgb(0x1c1c1f).into()
        } else {
            rgb(0xffffff).into()
        };

        let title_str = match &self.title_zh {
            Some(zh) => tr(cx, self.title.as_ref(), zh.as_ref()).to_string(),
            None => self.title.to_string(),
        };

        let running_n = self
            .agents
            .iter()
            .filter(|a| a.status == RadarAgentStatus::Running)
            .count();
        let ready = self.can_merge();

        let action_el: AnyElement = if self.merged {
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(
                    div()
                        .h(px(26.0))
                        .px(px(9.0))
                        .rounded(px(6.0))
                        .bg(p.green_soft)
                        .flex()
                        .items_center()
                        .gap(px(5.0))
                        .text_size(px(12.0))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.green_ink)
                        .child(pro_check_draw(
                            pro_eid(&self.id, "merged-chk"),
                            true,
                            13.0,
                            false,
                            p.green,
                        ))
                        .child(tr(cx, "Merged", "已合并分支")),
                )
                .child(pro_btn(
                    pro_eid(&self.id, "reset-radar-btn"),
                    tr(cx, "Reset", "重置"),
                    Some(ProIcon::Refresh),
                    ProBtnStyle::Ghost,
                    true,
                    cx,
                    |this, _, cx| {
                        this.reset_resolutions(cx);
                    },
                ))
                .into_any_element()
        } else if ready {
            pro_btn(
                pro_eid(&self.id, "merge-branches-btn"),
                tr(cx, "Merge branches", "合并分支"),
                Some(ProIcon::GitBranch),
                ProBtnStyle::Primary,
                true,
                cx,
                |this, _, cx| {
                    this.merge_branches(cx);
                },
            )
            .into_any_element()
        } else if running_n > 0 {
            let wait_txt = if tr(cx, "en", "zh") == "zh" {
                format!("等待 {running_n} 个分支完成")
            } else {
                format!("Waiting for {running_n} agents")
            };
            div()
                .h(px(26.0))
                .px(px(9.0))
                .rounded(px(6.0))
                .bg(p.chip)
                .border_1()
                .border_color(p.line)
                .flex()
                .items_center()
                .text_size(px(12.0))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.muted)
                .child(wait_txt)
                .into_any_element()
        } else {
            let is_zh = tr(cx, "en", "zh") == "zh";
            let suffix = if is_zh {
                " 处冲突待解"
            } else if unresolved_n == 1 {
                " conflict left"
            } else {
                " conflicts left"
            };
            div()
                .h(px(26.0))
                .px(px(9.0))
                .rounded(px(6.0))
                .bg(p.chip)
                .border_1()
                .border_color(p.line)
                .flex()
                .items_center()
                .child(pro_rolling_number(
                    pro_eid(&self.id, "unresolved-cnt"),
                    unresolved_n as f64,
                    0,
                    if is_zh { "剩余 " } else { "" },
                    suffix,
                    12.0,
                    FontWeight::MEDIUM,
                    p.muted,
                ))
                .into_any_element()
        };

        let header = div()
            .h(px(52.0))
            .pl(px(16.0))
            .pr(px(12.0))
            .border_b_1()
            .border_color(p.line)
            .flex()
            .items_center()
            .gap(px(10.0))
            .child(pro_svg("0 0 16 16", RADAR_ICON_SVG, 16.0, p.accent_info))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(px(13.5))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .child(title_str),
            )
            .child(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(10.0))
                    .text_size(px(11.0))
                    .text_color(p.muted)
                    .child(legend_pill(
                        p.blue.opacity(0.24),
                        p.blue.opacity(0.55),
                        tr(cx, "Changed", "已改"),
                    ))
                    .child(legend_pill(
                        p.orange.opacity(0.26),
                        p.orange,
                        tr(cx, "Conflict", "冲突"),
                    ))
                    .child(legend_pill(
                        p.green.opacity(0.24),
                        p.green,
                        tr(cx, "Kept", "采纳"),
                    )),
            )
            .child(action_el);

        if self.loading {
            let phase = (elapsed_secs * 0.9).fract();
            return div()
                .id(self.id.clone())
                .w_full()
                .flex()
                .justify_center()
                .child(
                    div()
                        .w_full()
                        .max_w(px(580.0))
                        .rounded(px(10.0))
                        .bg(card_bg)
                        .border_1()
                        .border_color(p.line)
                        .overflow_hidden()
                        .flex()
                        .flex_col()
                        .child(header)
                        .child(
                            div()
                                .p(px(16.0))
                                .flex()
                                .flex_col()
                                .gap(px(10.0))
                                .child(pro_skeleton_bar(520.0, 28.0, 6.0, phase, cx))
                                .child(pro_skeleton_bar(520.0, 28.0, 6.0, phase + 0.15, cx))
                                .child(pro_skeleton_bar(520.0, 28.0, 6.0, phase + 0.30, cx))
                                .child(pro_skeleton_bar(520.0, 96.0, 8.0, phase + 0.45, cx)),
                        ),
                )
                .into_any_element();
        }

        if self.empty || self.agents.is_empty() || self.files.is_empty() {
            return div()
                .id(self.id.clone())
                .w_full()
                .flex()
                .justify_center()
                .child(
                    div()
                        .w_full()
                        .max_w(px(580.0))
                        .rounded(px(10.0))
                        .bg(card_bg)
                        .border_1()
                        .border_color(p.line)
                        .overflow_hidden()
                        .flex()
                        .flex_col()
                        .child(header)
                        .child(pro_empty_state(
                            ProIcon::GitBranch,
                            tr(cx, "No parallel branches", "暂无并行分支"),
                            tr(
                                cx,
                                "Run coding agents on sibling branches to inspect overlapping line edits before merge.",
                                "启动多分支编码智能体即可在合并前检测重叠行号冲突。",
                            ),
                            Some((
                                pro_eid(&self.id, "seed-radar"),
                                SharedString::from(tr(cx, "Load parallel branches", "载入并行分支")),
                                Box::new(|this: &mut Self, _, cx| {
                                    this.empty = false;
                                    if this.agents.is_empty() {
                                        this.agents = default_agents();
                                    }
                                    if this.files.is_empty() {
                                        this.files = default_files();
                                    }
                                    this.sync_selected_conflict();
                                    cx.notify();
                                }),
                            )),
                            cx,
                        )),
                )
                .into_any_element();
        }

        let matrix = self.render_matrix(elapsed_secs, elapsed_ms, reduced, &p, w, cx);
        let picker = self.render_resolution_picker(elapsed_ms, &p, w, cx);

        div()
            .id(self.id.clone())
            .w_full()
            .flex()
            .justify_center()
            .child(
                div()
                    .w_full()
                    .max_w(px(580.0))
                    .rounded(px(10.0))
                    .bg(card_bg)
                    .border_1()
                    .border_color(p.line)
                    .when(!p.dark, |d| d.shadow_sm())
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .child(header)
                    .child(matrix)
                    .child(picker),
            )
            .into_any_element()
    }
}

impl ConflictRadar {
    fn render_matrix(
        &self,
        elapsed_secs: f32,
        elapsed_ms: f32,
        reduced: bool,
        p: &ProPalette,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let col_w = 104.0f32;
        let n_agents = self.agents.len();
        let right_w = col_w * n_agents as f32;
        let active_col_focus = self
            .hovered_agent_col
            .or_else(|| self.hovered_cell.map(|(_, c)| c));

        // Column header row
        let mut head_cells = div().w(px(right_w)).flex_none().h_full().flex();
        for (c_idx, ag) in self.agents.iter().enumerate() {
            let hovered = self.hovered_agent_col == Some(c_idx);
            let col_lit = active_col_focus.is_none_or(|fc| fc == c_idx);
            let col_op = pro_spring_f32(
                pro_eid(&self.id, format!("cr-hcol-op-{c_idx}")),
                if col_lit { 1.0 } else { 0.48 },
                0.20,
                w,
                cx,
            );
            let avatar_col = ag.color.unwrap_or(p.accent_info);
            let is_running = ag.status == RadarAgentStatus::Running;
            let avatar =
                pro_agent_avatar(ag.id.as_ref(), avatar_col, 18.0, is_running, elapsed_ms, cx);
            let name_txt = ag.display_name(cx).to_string();

            // Per-agent KPI counts for hover popover
            let files_touched = self
                .files
                .iter()
                .filter(|f| f.edits.iter().any(|e| e.agent_id == ag.id))
                .count();
            let open_conflicts = self
                .files
                .iter()
                .filter(|f| {
                    let cols = overlapping_agent_columns(&self.agents, &f.edits);
                    cols.len() >= 2 && cols.contains(&c_idx) && !self.resolved.contains_key(&f.path)
                })
                .count();
            let kept_count = self.resolved.values().filter(|&aid| aid == &ag.id).count();
            let model_str = ag.model.clone();

            let status_lbl = if is_running {
                tr(cx, "Running", "编写中").to_string()
            } else {
                tr(cx, "Ready", "已就绪").to_string()
            };
            let files_lbl = tr(cx, "files", "文件").to_string();
            let open_lbl = tr(cx, "open", "冲突").to_string();
            let kept_lbl = tr(cx, "kept", "采纳").to_string();
            let kpi_files_id = pro_eid(&self.id, format!("kpi-f-{c_idx}"));
            let kpi_open_id = pro_eid(&self.id, format!("kpi-o-{c_idx}"));
            let kpi_kept_id = pro_eid(&self.id, format!("kpi-k-{c_idx}"));
            let p_copy = *p;
            let name_for_pop = name_txt.clone();

            let pop_presence = pro_popover_presence(
                pro_eid(&self.id, format!("hcol-pop-{c_idx}")),
                hovered,
                move || {
                    div()
                        .w(px(214.0))
                        .p(px(10.0))
                        .rounded(px(9.0))
                        .bg(p_copy.card)
                        .border_1()
                        .border_color(p_copy.line_strong)
                        .shadow_lg()
                        .flex()
                        .flex_col()
                        .gap(px(4.0))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .child(
                                    div()
                                        .text_size(px(12.5))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(p_copy.strong)
                                        .child(name_for_pop),
                                )
                                .child(
                                    div()
                                        .px(px(6.0))
                                        .h(px(18.0))
                                        .rounded(px(4.5))
                                        .bg(if is_running {
                                            p_copy.accent_soft
                                        } else {
                                            p_copy.chip
                                        })
                                        .flex()
                                        .items_center()
                                        .text_size(px(10.5))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(if is_running {
                                            p_copy.accent_info
                                        } else {
                                            p_copy.muted
                                        })
                                        .child(status_lbl),
                                ),
                        )
                        .when_some(model_str, |d, m| {
                            d.child(
                                div()
                                    .truncate()
                                    .text_size(px(11.0))
                                    .text_color(p_copy.muted)
                                    .child(m),
                            )
                        })
                        .child(
                            div()
                                .pt(px(4.0))
                                .flex()
                                .gap(px(5.0))
                                .child(kpi_mini_badge(
                                    kpi_files_id,
                                    files_touched,
                                    &files_lbl,
                                    p_copy.ink,
                                    p_copy.muted,
                                    p_copy.subtle,
                                    p_copy.line,
                                ))
                                .child(kpi_mini_badge(
                                    kpi_open_id,
                                    open_conflicts,
                                    &open_lbl,
                                    if open_conflicts > 0 {
                                        p_copy.amber_ink
                                    } else {
                                        p_copy.ink
                                    },
                                    p_copy.muted,
                                    p_copy.subtle,
                                    p_copy.line,
                                ))
                                .child(kpi_mini_badge(
                                    kpi_kept_id,
                                    kept_count,
                                    &kept_lbl,
                                    if kept_count > 0 {
                                        p_copy.green_ink
                                    } else {
                                        p_copy.ink
                                    },
                                    p_copy.muted,
                                    p_copy.subtle,
                                    p_copy.line,
                                )),
                        )
                        .into_any_element()
                },
            );
            let positioned_pop = if c_idx + 1 == n_agents {
                div()
                    .absolute()
                    .top(px(34.0))
                    .right(px(6.0))
                    .child(pop_presence)
            } else {
                div()
                    .absolute()
                    .top(px(34.0))
                    .left(px(-36.0))
                    .child(pop_presence)
            };

            head_cells = head_cells.child(
                div()
                    .id(pro_eid(&self.id, format!("head-col-{c_idx}")))
                    .w(px(col_w))
                    .h_full()
                    .relative()
                    .border_l_1()
                    .border_color(p.line)
                    .px(px(8.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(px(6.0))
                    .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                        if *inside {
                            this.hovered_agent_col = Some(c_idx);
                        } else if this.hovered_agent_col == Some(c_idx) {
                            this.hovered_agent_col = None;
                        }
                        cx.notify();
                    }))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .min_w_0()
                            .opacity(col_op)
                            .child(avatar)
                            .child(
                                div()
                                    .truncate()
                                    .text_size(px(12.0))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.muted)
                                    .child(name_txt),
                            ),
                    )
                    .child(positioned_pop),
            );
        }

        let head_row = div()
            .id(pro_eid(&self.id, "matrix-head-row"))
            .role(Role::Row)
            .h(px(34.0))
            .w_full()
            .flex()
            .items_center()
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .px(px(16.0))
                    .text_size(px(11.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(p.muted)
                    .child(tr(cx, "FILE", "变更文件")),
            )
            .child(head_cells);

        let mut table = div()
            .id(pro_eid(&self.id, "matrix-grid"))
            .role(Role::Table)
            .aria_label(tr(cx, "Files by agent", "按智能体展示的文件变更矩阵"))
            .w_full()
            .flex()
            .flex_col()
            .child(head_row);

        let pulse_phase = if reduced {
            0.5
        } else {
            ((elapsed_secs / 1.5).sin() * 0.5 + 0.5).clamp(0.0, 1.0)
        };

        for (r_idx, file) in self.files.iter().enumerate() {
            let overlap_cols = overlapping_agent_columns(&self.agents, &file.edits);
            let has_overlap = overlap_cols.len() >= 2;
            let chosen_agent = self.resolved.get(&file.path).cloned();
            let is_settled = has_overlap && chosen_agent.is_some();
            let is_open_conflict = has_overlap && chosen_agent.is_none();
            let is_selected = self.selected_path.as_ref() == Some(&file.path);

            let row_lit = if let Some((hr, _)) = self.hovered_cell {
                hr == r_idx
            } else if let Some(hc) = self.hovered_agent_col {
                let ag_id = &self.agents[hc].id;
                file.edits.iter().any(|e| &e.agent_id == ag_id)
            } else {
                true
            };
            let row_op = pro_spring_f32(
                pro_eid(&self.id, format!("cr-row-op-{r_idx}")),
                if row_lit { 1.0 } else { 0.38 },
                0.20,
                w,
                cx,
            );

            let dot_col = if is_open_conflict {
                p.amber_ink
            } else if is_settled {
                p.green_ink
            } else {
                p.chip_hover
            };

            let (dir_part, name_part) = split_dir_file(file.path.as_ref());

            // Horizontal connector bar (`PathBuilder`) between conflicting agent columns
            let connector_canvas = if is_open_conflict {
                let first_col = overlap_cols[0];
                let last_col = overlap_cols[overlap_cols.len() - 1];
                let x0 = (first_col as f32 + 0.5) * col_w;
                let x1 = (last_col as f32 + 0.5) * col_w;
                let line_col = p.orange.opacity(0.62);
                let glow_col = p.amber_ink;
                Some(
                    canvas(
                        |_, _, _| (),
                        move |bounds, _, window, _| {
                            let cy = bounds.center().y;
                            let p0 = point(bounds.left() + px(x0), cy);
                            let p1 = point(bounds.left() + px(x1), cy);
                            let mut base = PathBuilder::stroke(px(1.75));
                            base.move_to(p0);
                            base.line_to(p1);
                            if let Ok(path) = base.build() {
                                window.paint_path(path, line_col);
                            }
                            if !reduced && (x1 - x0) > 8.0 {
                                let head = x0 + (x1 - x0) * pulse_phase;
                                let span = 22.0f32;
                                let gx0 = (head - span).max(x0);
                                let gx1 = (head + span).min(x1);
                                if gx1 > gx0 {
                                    let mut pulse = PathBuilder::stroke(px(2.5));
                                    pulse.move_to(point(bounds.left() + px(gx0), cy));
                                    pulse.line_to(point(bounds.left() + px(gx1), cy));
                                    if let Ok(path) = pulse.build() {
                                        window.paint_path(path, glow_col);
                                    }
                                }
                            }
                        },
                    )
                    .absolute()
                    .inset_0(),
                )
            } else {
                None
            };

            let mut cells_row = div().w(px(right_w)).flex_none().h_full().relative().flex();
            if let Some(conn) = connector_canvas {
                cells_row = cells_row.child(conn);
            }

            for (c_idx, ag) in self.agents.iter().enumerate() {
                let edit_opt = file.edits.iter().find(|e| e.agent_id == ag.id);
                let cell_hovered = self.hovered_cell == Some((r_idx, c_idx));
                let is_conflict_participant = overlap_cols.contains(&c_idx);

                let (target_bar_w, bar_h, bar_rad, bar_bg, bar_edge, tip_state) = match edit_opt {
                    None => (4.0, 4.0, 99.0, p.chip_hover, transparent_black(), None),
                    Some(ed) => {
                        if is_conflict_participant {
                            match &chosen_agent {
                                None => (
                                    32.0,
                                    8.0,
                                    4.0,
                                    p.orange.opacity(0.24),
                                    p.orange.opacity(0.68),
                                    Some((
                                        tr(cx, "Conflict", "行号冲突").to_string(),
                                        p.amber_soft,
                                        p.amber_ink,
                                        ed.lines,
                                    )),
                                ),
                                Some(kept_id) if kept_id == &ag.id => (
                                    32.0,
                                    8.0,
                                    4.0,
                                    p.green_soft,
                                    p.green.opacity(0.55),
                                    Some((
                                        tr(cx, "Kept", "已采纳").to_string(),
                                        p.green_soft,
                                        p.green_ink,
                                        ed.lines,
                                    )),
                                ),
                                Some(_) => (
                                    12.0,
                                    8.0,
                                    4.0,
                                    p.chip_hover,
                                    transparent_black(),
                                    Some((
                                        tr(cx, "Discarded", "未采用").to_string(),
                                        p.chip,
                                        p.muted,
                                        ed.lines,
                                    )),
                                ),
                            }
                        } else {
                            (
                                32.0,
                                8.0,
                                4.0,
                                p.accent_soft,
                                p.blue.opacity(0.45),
                                Some((
                                    tr(cx, "Changed", "独立变更").to_string(),
                                    p.accent_soft,
                                    p.accent_info,
                                    ed.lines,
                                )),
                            )
                        }
                    }
                };

                let bar_w = pro_spring_f32(
                    pro_eid(&self.id, format!("cr-bw-{r_idx}-{c_idx}")),
                    target_bar_w,
                    0.24,
                    w,
                    cx,
                );

                let ag_lbl = ag.display_name(cx).to_string();
                let cell_pop_id = pro_eid(&self.id, format!("cell-pop-{r_idx}-{c_idx}"));
                let cell_num_id = pro_eid(&self.id, format!("cell-lns-{r_idx}-{c_idx}"));
                let p_cell = *p;

                cells_row = cells_row.child(
                    div()
                        .id(pro_eid(&self.id, format!("cell-{r_idx}-{c_idx}")))
                        .w(px(col_w))
                        .h_full()
                        .relative()
                        .border_l_1()
                        .border_color(p.line)
                        .flex()
                        .items_center()
                        .justify_center()
                        .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                            if *inside {
                                this.hovered_cell = Some((r_idx, c_idx));
                            } else if this.hovered_cell == Some((r_idx, c_idx)) {
                                this.hovered_cell = None;
                            }
                            cx.notify();
                        }))
                        .child(
                            div()
                                .w(px(bar_w))
                                .h(px(bar_h))
                                .rounded(px(bar_rad))
                                .bg(bar_bg)
                                .opacity(row_op)
                                .when(bar_edge.a > 0.01, |d| d.border_1().border_color(bar_edge)),
                        )
                        .when_some(tip_state, |cell, (st_lbl, s_bg, s_fg, (l0, l1))| {
                            let span_lines = l1.saturating_sub(l0) + 1;
                            let tip_presence =
                                pro_popover_presence(cell_pop_id, cell_hovered, move || {
                                    div()
                                        .px(px(8.5))
                                        .py(px(5.5))
                                        .rounded(px(7.0))
                                        .bg(p_cell.card)
                                        .border_1()
                                        .border_color(p_cell.line_strong)
                                        .shadow_md()
                                        .whitespace_nowrap()
                                        .flex()
                                        .flex_col()
                                        .gap(px(2.0))
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap(px(6.0))
                                                .child(
                                                    div()
                                                        .text_size(px(11.5))
                                                        .font_weight(FontWeight::SEMIBOLD)
                                                        .text_color(p_cell.ink)
                                                        .child(ag_lbl),
                                                )
                                                .child(
                                                    div()
                                                        .px(px(5.0))
                                                        .h(px(16.0))
                                                        .rounded(px(4.0))
                                                        .bg(s_bg)
                                                        .flex()
                                                        .items_center()
                                                        .text_size(px(10.0))
                                                        .font_weight(FontWeight::MEDIUM)
                                                        .text_color(s_fg)
                                                        .child(st_lbl),
                                                ),
                                        )
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap(px(5.0))
                                                .text_size(px(11.0))
                                                .text_color(p_cell.muted)
                                                .child(format!("L{l0}–{l1} ·"))
                                                .child(pro_rolling_number(
                                                    cell_num_id,
                                                    span_lines as f64,
                                                    0,
                                                    "",
                                                    " ln",
                                                    11.0,
                                                    FontWeight::MEDIUM,
                                                    p_cell.ink,
                                                )),
                                        )
                                        .into_any_element()
                                });
                            let pos = if c_idx + 1 == n_agents {
                                div()
                                    .absolute()
                                    .bottom(px(30.0))
                                    .right(px(6.0))
                                    .child(tip_presence)
                            } else {
                                div()
                                    .absolute()
                                    .bottom(px(30.0))
                                    .left(px(-16.0))
                                    .child(tip_presence)
                            };
                            cell.child(pos)
                        }),
                );
            }

            let row_bg = if is_selected && is_open_conflict {
                p.orange.opacity(if p.dark { 0.09 } else { 0.06 })
            } else if is_selected {
                p.select
            } else if !r_idx.is_multiple_of(2) {
                p.ink.opacity(0.015)
            } else {
                transparent_black()
            };

            let file_path = file.path.clone();
            let file_aria = file.path.clone();
            let lead_indicator: AnyElement = if is_settled {
                pro_check_draw(
                    pro_eid(&self.id, format!("frow-chk-{r_idx}")),
                    true,
                    12.0,
                    false,
                    p.green,
                )
            } else {
                div()
                    .size(px(6.0))
                    .rounded_full()
                    .flex_none()
                    .bg(dot_col)
                    .into_any_element()
            };

            let file_cell = div()
                .flex_1()
                .min_w_0()
                .px(px(16.0))
                .opacity(row_op)
                .flex()
                .items_center()
                .gap(px(9.0))
                .child(lead_indicator)
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .items_center()
                        .text_size(px(12.5))
                        .when(!dir_part.is_empty(), |d| {
                            d.child(div().text_color(p.faint).child(dir_part))
                        })
                        .child(
                            div()
                                .truncate()
                                .font_weight(if has_overlap {
                                    FontWeight::MEDIUM
                                } else {
                                    FontWeight::NORMAL
                                })
                                .text_color(if has_overlap { p.ink } else { p.text })
                                .child(name_part),
                        ),
                );

            if has_overlap {
                let row_inner = div()
                    .h(px(36.0))
                    .w_full()
                    .flex()
                    .items_center()
                    .child(file_cell)
                    .child(cells_row);

                let tact_row = pro_tactile(
                    pro_eid(&self.id, format!("frow-{r_idx}")),
                    file_aria,
                    0.0,
                    row_bg,
                    p.hover,
                    transparent_black(),
                    transparent_black(),
                    is_selected,
                    row_inner,
                    cx,
                    move |this, _, cx| {
                        this.selected_path = Some(file_path.clone());
                        cx.emit(ConflictRadarEvent::SelectedFile(file_path.clone()));
                        cx.notify();
                    },
                )
                .role(Role::Row)
                .h(px(36.0))
                .w_full()
                .border_t_1()
                .border_color(p.line);

                table = table.child(tact_row);
            } else {
                let row_el = div()
                    .id(pro_eid(&self.id, format!("frow-{r_idx}")))
                    .role(Role::Row)
                    .h(px(36.0))
                    .w_full()
                    .border_t_1()
                    .border_color(p.line)
                    .bg(row_bg)
                    .flex()
                    .items_center()
                    .child(file_cell)
                    .child(cells_row);
                table = table.child(row_el);
            }
        }

        table
    }

    fn render_resolution_picker(
        &self,
        elapsed_ms: f32,
        p: &ProPalette,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let conflict_files: Vec<RadarFile> = self
            .files
            .iter()
            .filter(|f| overlapping_agent_columns(&self.agents, &f.edits).len() >= 2)
            .cloned()
            .collect();

        if conflict_files.is_empty() {
            return div()
                .h(px(72.0))
                .border_t_1()
                .border_color(p.line)
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(12.0))
                .text_color(p.muted)
                .child(tr(
                    cx,
                    "Watching parallel branches — zero overlapping edits.",
                    "正监控各分支改动，未检测到行号重叠冲突。",
                ));
        }

        let target_file = self
            .selected_path
            .as_ref()
            .and_then(|sp| conflict_files.iter().find(|f| &f.path == sp))
            .or_else(|| conflict_files.first())
            .cloned();

        let Some(target) = target_file else {
            return div();
        };

        let overlap_cols = overlapping_agent_columns(&self.agents, &target.edits);
        let chosen_id = self.resolved.get(&target.path).cloned();
        let min_line = target.edits.iter().map(|e| e.lines.0).min().unwrap_or(1);
        let max_line = target.edits.iter().map(|e| e.lines.1).max().unwrap_or(1);

        // Quick file conflict switcher tabs with liquid `pro_pill_indicator`
        let tab_sensor = pro_selection_sensor();
        let pill_fill: Hsla = if p.dark {
            rgb(0x26262a).into()
        } else {
            rgb(0xffffff).into()
        };
        let mut conflict_tabs = div()
            .relative()
            .p(px(2.0))
            .rounded(px(7.0))
            .bg(p.chip)
            .flex()
            .items_center()
            .gap(px(3.0))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "ctab-pill"),
                target.path.clone(),
                tab_sensor.clone(),
                5.5,
                pill_fill,
                Some(p.line_strong),
                true,
            ));

        for cf in &conflict_files {
            let active = cf.path == target.path;
            let settled = self.resolved.contains_key(&cf.path);
            let (_, short_name) = split_dir_file(cf.path.as_ref());
            let cpath = cf.path.clone();
            conflict_tabs = conflict_tabs.child(
                div()
                    .id(pro_eid(&self.id, format!("ctab-{cpath}")))
                    .role(Role::Button)
                    .aria_label(short_name.clone())
                    .aria_toggled(active.into())
                    .tab_index(0)
                    .relative()
                    .h(px(22.0))
                    .px(px(7.5))
                    .rounded(px(5.5))
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .gap(px(5.0))
                    .text_size(px(11.5))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(if active { p.ink } else { p.muted })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.selected_path = Some(cpath.clone());
                        cx.emit(ConflictRadarEvent::SelectedFile(cpath.clone()));
                        cx.notify();
                    }))
                    .when(active, |d| d.child(pro_mark_selected(&tab_sensor)))
                    .child(if settled {
                        pro_check_draw(
                            pro_eid(&self.id, format!("ctab-chk-{}", cf.path)),
                            true,
                            12.0,
                            false,
                            p.green,
                        )
                    } else {
                        div()
                            .size(px(5.5))
                            .rounded_full()
                            .bg(p.amber_ink)
                            .into_any_element()
                    })
                    .child(short_name),
            );
        }

        let mut ways_row = div()
            .id(pro_eid(&self.id, "ways-radiogroup"))
            .role(Role::RadioGroup)
            .aria_label(format!(
                "{} {}",
                tr(cx, "Pick implementation for", "选择采纳的分支方案："),
                target.path
            ))
            .px(px(12.0))
            .pb(px(12.0))
            .flex()
            .gap(px(8.0));

        for &c_idx in &overlap_cols {
            let ag = &self.agents[c_idx];
            let Some(ed) = target.edits.iter().find(|e| e.agent_id == ag.id) else {
                continue;
            };
            let is_kept = chosen_id.as_ref() == Some(&ag.id);
            let is_lost = chosen_id.is_some() && !is_kept;
            let card_op = pro_spring_f32(
                pro_eid(&self.id, format!("way-op-{}-{}", target.path, ag.id)),
                if is_lost { 0.56 } else { 1.0 },
                0.22,
                w,
                cx,
            );
            let avatar_col = ag.color.unwrap_or(p.accent_info);
            let avatar = pro_agent_avatar(ag.id.as_ref(), avatar_col, 18.0, false, elapsed_ms, cx);
            let ag_name = ag.display_name(cx).to_string();
            let summary_txt = ed
                .display_summary(cx)
                .unwrap_or_else(|| tr(cx, "Modified overlapping lines.", "修改了同一段代码区间。"))
                .to_string();
            let path_for_pick = target.path.clone();
            let aid_for_pick = ag.id.clone();
            let line_span = ed.lines.1.saturating_sub(ed.lines.0) + 1;

            let way_inner = div()
                .w_full()
                .p(px(11.0))
                .flex()
                .flex_col()
                .gap(px(6.5))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(7.0))
                        .child(avatar)
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .text_size(px(12.5))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.ink)
                                .child(ag_name.clone()),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(4.0))
                                .text_size(px(11.0))
                                .text_color(p.muted)
                                .child(format!("L{}–{}", ed.lines.0, ed.lines.1))
                                .child(pro_rolling_number(
                                    pro_eid(&self.id, format!("way-ln-{}-{}", target.path, ag.id)),
                                    line_span as f64,
                                    0,
                                    "(",
                                    ")",
                                    10.5,
                                    FontWeight::NORMAL,
                                    p.faint,
                                )),
                        )
                        .child(if is_kept {
                            div()
                                .h(px(18.0))
                                .px(px(6.0))
                                .rounded(px(4.5))
                                .bg(p.green_soft)
                                .flex()
                                .items_center()
                                .gap(px(4.0))
                                .text_size(px(10.5))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.green_ink)
                                .child(pro_check_draw(
                                    pro_eid(&self.id, format!("way-chk-{}-{}", target.path, ag.id)),
                                    true,
                                    12.0,
                                    false,
                                    p.green,
                                ))
                                .child(tr(cx, "Kept", "已选"))
                                .into_any_element()
                        } else {
                            div()
                                .size(px(14.0))
                                .rounded_full()
                                .border_1()
                                .border_color(p.line_strong)
                                .into_any_element()
                        }),
                )
                .child(
                    div()
                        .text_size(px(12.0))
                        .line_height(px(17.5))
                        .text_color(p.text)
                        .child(summary_txt.clone()),
                );

            let way_card = pro_tactile(
                pro_eid(&self.id, format!("way-{}-{}", target.path, ag.id)),
                format!("{ag_name}: {summary_txt}"),
                8.0,
                if is_kept { p.card } else { p.subtle },
                p.select,
                if is_kept {
                    p.green.opacity(0.55)
                } else {
                    p.line
                },
                if is_kept {
                    p.green.opacity(0.78)
                } else {
                    p.line_strong
                },
                is_kept,
                way_inner,
                cx,
                move |this, _, cx| {
                    this.resolve_conflict(path_for_pick.as_ref(), aid_for_pick.as_ref(), cx);
                },
            )
            .role(Role::RadioButton)
            .aria_toggled(is_kept.into())
            .flex_1()
            .min_w_0()
            .opacity(card_op);

            ways_row = ways_row.child(way_card);
        }

        let inner = div()
            .w_full()
            .flex()
            .flex_col()
            .child(
                div()
                    .px(px(16.0))
                    .pt(px(12.0))
                    .pb(px(8.0))
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .justify_between()
                    .gap(px(8.0))
                    .child(
                        div()
                            .flex()
                            .items_baseline()
                            .gap(px(7.0))
                            .child(
                                div()
                                    .text_size(px(12.5))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .child(tr(cx, "Pick one implementation", "选择保留的修改方案")),
                            )
                            .child(
                                div()
                                    .text_size(px(11.5))
                                    .text_color(p.muted)
                                    .child(format!("{} · L{min_line}–{max_line}", target.path)),
                            ),
                    )
                    .child(conflict_tabs),
            )
            .child(ways_row);

        div()
            .border_t_1()
            .border_color(p.line)
            .child(pro_view_transition(
                pro_eid(&self.id, "picker-tx"),
                target.path,
                inner,
            ))
    }
}

fn legend_pill(bg: Hsla, edge: Hsla, label: &str) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(5.0))
        .child(
            div()
                .w(px(12.0))
                .h(px(6.0))
                .rounded(px(3.0))
                .bg(bg)
                .border_1()
                .border_color(edge),
        )
        .child(label.to_string())
}

fn kpi_mini_badge(
    num_id: ElementId,
    num: usize,
    lbl: &str,
    num_c: Hsla,
    lbl_c: Hsla,
    bg: Hsla,
    border_c: Hsla,
) -> Div {
    div()
        .flex_1()
        .h(px(24.0))
        .px(px(5.0))
        .rounded(px(5.0))
        .bg(bg)
        .border_1()
        .border_color(border_c)
        .flex()
        .items_center()
        .justify_center()
        .gap(px(3.5))
        .child(pro_rolling_number(
            num_id,
            num as f64,
            0,
            "",
            "",
            11.5,
            FontWeight::SEMIBOLD,
            num_c,
        ))
        .child(
            div()
                .text_size(px(10.5))
                .text_color(lbl_c)
                .child(lbl.to_string()),
        )
}

fn split_dir_file(path: &str) -> (String, String) {
    if let Some((d, f)) = path.rsplit_once('/') {
        (format!("{d}/"), f.to_string())
    } else {
        (String::new(), path.to_string())
    }
}

fn default_agents() -> Vec<RadarAgent> {
    vec![
        RadarAgent::new("checkout", "Checkout")
            .with_name_zh("结账流")
            .with_model("Claude Sonnet · feat/checkout")
            .with_color(rgb(0xd97757).into()),
        RadarAgent::new("stripe", "Stripe v2")
            .with_name_zh("Stripe v2")
            .with_model("GPT-6 Sol · feat/stripe-v2")
            .with_color(rgb(0x0190fe).into()),
        RadarAgent::new("retries", "Retries")
            .with_name_zh("幂等重试")
            .with_model("Gemini 3.8 · feat/idempotent-retry")
            .with_color(rgb(0xa651f0).into()),
    ]
}

fn default_files() -> Vec<RadarFile> {
    vec![
        RadarFile::new(
            "src/checkout/payment.ts",
            vec![
                RadarEdit::new("checkout", 42, 78).with_summary(
                    "Validates card intent client-side before confirming payment.",
                    "在确认扣款前先于客户端校验支付意图状态与金额一致性。",
                ),
                RadarEdit::new("stripe", 54, 92).with_summary(
                    "Delegates confirmation to Stripe PaymentElement and webhook callback.",
                    "统一交由 Stripe PaymentElement 与服务端异步回调确认扣款。",
                ),
            ],
        ),
        RadarFile::new(
            "src/checkout/receipt.tsx",
            vec![RadarEdit::new("checkout", 12, 46)],
        ),
        RadarFile::new(
            "src/lib/stripe.ts",
            vec![
                RadarEdit::new("stripe", 18, 55).with_summary(
                    "Pins Stripe SDK API 2026-08 with typed idempotency headers.",
                    "锁定 Stripe 2026-08 版协议并生成强类型幂等请求头。",
                ),
                RadarEdit::new("retries", 36, 70).with_summary(
                    "Wraps stripe.paymentIntents with jittered exponential backoff.",
                    "为 paymentIntents 接口封装带随机抖动的指数退避重试层。",
                ),
            ],
        ),
        RadarFile::new(
            "tests/checkout.spec.ts",
            vec![
                // Non-overlapping line edits in the same file -> both clean!
                RadarEdit::new("checkout", 10, 38),
                RadarEdit::new("retries", 112, 154),
            ],
        ),
        RadarFile::new("src/lib/retry.ts", vec![RadarEdit::new("retries", 1, 64)]),
        RadarFile::new("package.json", vec![RadarEdit::new("stripe", 14, 19)]),
    ]
}

#[cfg(test)]
mod tests {
    use super::{ConflictRadar, RadarAgent, RadarEdit, overlapping_agent_columns};
    use gpui::{AppContext as _, TestAppContext};

    #[test]
    fn line_range_overlap_distinguishes_conflicts_from_disjoint_edits() {
        let agents = vec![
            RadarAgent::new("a", "A"),
            RadarAgent::new("b", "B"),
            RadarAgent::new("c", "C"),
        ];
        let disjoint = vec![RadarEdit::new("a", 10, 30), RadarEdit::new("c", 80, 110)];
        assert!(
            overlapping_agent_columns(&agents, &disjoint).is_empty(),
            "disjoint line ranges in the same file should not conflict"
        );

        let overlapping = vec![RadarEdit::new("a", 42, 78), RadarEdit::new("b", 54, 92)];
        assert_eq!(overlapping_agent_columns(&agents, &overlapping), vec![0, 1]);
    }

    #[gpui::test]
    fn resolving_all_conflicts_enables_merge_branches(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let view = cx.new(|cx| ConflictRadar::new("test-conflict-radar", cx));

        view.update(cx, |v, cx| {
            assert_eq!(v.unresolved_conflict_count(), 2);
            assert!(!v.can_merge());

            assert!(v.resolve_conflict("src/checkout/payment.ts", "stripe", cx));
            assert_eq!(v.unresolved_conflict_count(), 1);
            assert!(!v.can_merge());

            assert!(v.resolve_conflict("src/lib/stripe.ts", "retries", cx));
            assert_eq!(v.unresolved_conflict_count(), 0);
            assert!(v.can_merge());
            assert!(v.merge_branches(cx));
            assert!(!v.can_merge(), "merged state disables duplicate merge");
        });
    }
}
