//! System-mutation audit ledger with `before -> after` diffs and dependency-aware Undo / Redo.
//!
//! Lists every external change made by an agent newest-first with target app tiles (`pro_app_tile`),
//! struck-through `before -> after` values, irreversible locks, and live dependency previewing:
//! pointing at or focusing `Undo` highlights downstream mutations that will roll back with it
//! (`Also undone` in blue) alongside irreversible side effects that remain (`Can't undo` in amber).

use crate::{
    components::pro_chrome::{
        ProPalette, pro_app_tile, pro_card_shadow, pro_eid, pro_empty_state, pro_mix_color,
        pro_presence, pro_rolling_number, pro_skeleton_bar, pro_spring_f32, pro_tactile,
        pro_toast_banner, tr,
    },
    foundation::Theme,
    primitives::{
        family::disabled_motion,
        pro_icons::{ProIcon, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{
    cell::Cell,
    collections::{HashMap, HashSet},
    rc::Rc,
};
use web_time::Instant;

const LEDGER_DOC_SVG: &str = r#"<rect x="2" y="1.8" width="12" height="12.4" rx="2.6" stroke="currentColor" stroke-width="1.3"></rect><path d="M5 5.4h6M5 8h6M5 10.6h3.6" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"></path>"#;
const REDO_SVG: &str = r#"<path d="M20 7H9C5.68629 7 3 9.68629 3 13C3 16.3137 5.68629 19 9 19H16" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8"></path><path d="M16 3L20 7L16 11" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8"></path>"#;

/// Single system mutation recorded in [`AgentLedger`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LedgerEntry {
    /// Unique mutation identifier referenced by downstream `depends_on` links.
    pub id: SharedString,
    /// Verb phrase describing what changed (`"Changed price"`, `"Merged PR #482"`).
    pub action: SharedString,
    /// Optional object or record target (`"Pro plan"`, `"Pricing page v2"`).
    pub target: Option<SharedString>,
    /// External system name (`"Stripe"`, `"HubSpot"`, `"GitHub"`, `"Vercel"`, `"Gmail"`).
    pub system: SharedString,
    /// Optional explicit app artwork key override.
    pub icon: Option<SharedString>,
    /// Elapsed age in seconds (`0` = `"just now"`, smaller values render closer to the top).
    pub age_secs: u64,
    /// Optional `(before, after)` value pair rendered as `before → after`.
    pub change: Option<(SharedString, SharedString)>,
    /// Optional single-line summary used when `change` is `None`.
    pub detail: Option<SharedString>,
    /// Whether this mutation can be reverted (`false` for sent emails, payouts, etc.).
    pub reversible: bool,
    /// Upstream mutation ids that this change builds upon.
    pub depends_on: Vec<SharedString>,
}

impl LedgerEntry {
    /// Construct a reversible [`LedgerEntry`] in `system`.
    pub fn new(
        id: impl Into<SharedString>,
        action: impl Into<SharedString>,
        system: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            action: action.into(),
            target: None,
            system: system.into(),
            icon: None,
            age_secs: 0,
            change: None,
            detail: None,
            reversible: true,
            depends_on: Vec::new(),
        }
    }

    /// Set the target object name displayed beside `action`.
    pub fn with_target(mut self, target: impl Into<SharedString>) -> Self {
        self.target = Some(target.into());
        self
    }

    /// Override the icon lookup key passed to [`pro_app_tile`].
    pub fn with_icon(mut self, icon: impl Into<SharedString>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Set how many seconds ago this mutation happened (`0` = `"just now"`).
    pub fn with_age_secs(mut self, age_secs: u64) -> Self {
        self.age_secs = age_secs;
        self
    }

    /// Provide `(before, after)` values rendered as struck-through `before → after`.
    pub fn with_change(
        mut self,
        before: impl Into<SharedString>,
        after: impl Into<SharedString>,
    ) -> Self {
        self.change = Some((before.into(), after.into()));
        self
    }

    /// Provide a single-line detail string when `before → after` does not apply.
    pub fn with_detail(mut self, detail: impl Into<SharedString>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Mark whether this mutation is reversible (`false` shows an `Irreversible` lock).
    pub fn with_reversible(mut self, reversible: bool) -> Self {
        self.reversible = reversible;
        self
    }

    /// Declare upstream mutation ids that this entry depends upon.
    pub fn with_depends_on<I, S>(mut self, ids: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        self.depends_on = ids.into_iter().map(Into::into).collect();
        self
    }
}

/// Strongly typed events emitted by [`AgentLedger`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentLedgerEvent {
    /// Emitted with all reverted mutation ids (root + reversible downstream dependents).
    Undo(Vec<SharedString>),
    /// Emitted with all reapplied mutation ids (target + undone upstream prerequisites).
    Redo(Vec<SharedString>),
    /// Emitted when hovering or focusing an `Undo` button previews its blast radius.
    PreviewChanged(Option<SharedString>),
}

impl EventEmitter<AgentLedgerEvent> for AgentLedger {}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct UndoBlastRadius {
    /// Active reversible downstream mutations that will roll back alongside the target.
    pub also_undone: Vec<SharedString>,
    /// Active irreversible downstream mutations that built on the target and cannot be undone.
    pub kept_irreversible: Vec<SharedString>,
}

/// Compute transitive downstream impact of undoing `root_id`.
pub(crate) fn compute_undo_blast(
    entries: &[LedgerEntry],
    undone: &HashSet<SharedString>,
    root_id: &str,
) -> UndoBlastRadius {
    let mut children_map: HashMap<&str, Vec<&LedgerEntry>> = HashMap::new();
    for entry in entries {
        for dep in &entry.depends_on {
            children_map.entry(dep.as_ref()).or_default().push(entry);
        }
    }

    let mut visited: HashSet<SharedString> = HashSet::new();
    visited.insert(SharedString::from(root_id.to_string()));
    let mut stack: Vec<&str> = vec![root_id];
    let mut also_undone = Vec::new();
    let mut kept_irreversible = Vec::new();

    while let Some(cur) = stack.pop() {
        if let Some(dependents) = children_map.get(cur) {
            for &dep_entry in dependents {
                if !visited.insert(dep_entry.id.clone()) {
                    continue;
                }
                if undone.contains(&dep_entry.id) {
                    continue;
                }
                if dep_entry.reversible {
                    also_undone.push(dep_entry.id.clone());
                    stack.push(dep_entry.id.as_ref());
                } else {
                    kept_irreversible.push(dep_entry.id.clone());
                    // Even if an intermediate step is irreversible, inspect anything built on top of it.
                    stack.push(dep_entry.id.as_ref());
                }
            }
        }
    }

    UndoBlastRadius {
        also_undone,
        kept_irreversible,
    }
}

/// Compute undone upstream prerequisites that should be reapplied when redoing `target_id`.
pub(crate) fn compute_redo_chain(
    entries: &[LedgerEntry],
    undone: &HashSet<SharedString>,
    target_id: &str,
) -> Vec<SharedString> {
    let by_id: HashMap<&str, &LedgerEntry> = entries.iter().map(|e| (e.id.as_ref(), e)).collect();
    let mut visited: HashSet<SharedString> = HashSet::new();
    let mut chain: Vec<SharedString> = Vec::new();
    let mut stack = vec![target_id];

    while let Some(cur) = stack.pop() {
        let Some(&entry) = by_id.get(cur) else {
            continue;
        };
        if !visited.insert(entry.id.clone()) {
            continue;
        }
        if undone.contains(&entry.id) && entry.reversible {
            chain.push(entry.id.clone());
        }
        for dep in &entry.depends_on {
            stack.push(dep.as_ref());
        }
    }
    chain
}

fn format_relative_age(secs: u64, cx: &App) -> String {
    if secs < 5 {
        tr(cx, "just now", "刚刚").to_string()
    } else if secs < 60 {
        format!("{secs}s {}", tr(cx, "ago", "前"))
    } else if secs < 3_600 {
        format!("{}m {}", secs / 60, tr(cx, "ago", "前"))
    } else {
        format!("{}h {}", secs / 3_600, tr(cx, "ago", "前"))
    }
}

fn render_ledger_app_tile(entry: &LedgerEntry, dimmed: bool, cx: &App) -> AnyElement {
    let key = entry.icon.as_deref().unwrap_or(entry.system.as_ref());
    div()
        .flex_none()
        .opacity(if dimmed { 0.48 } else { 1.0 })
        .child(pro_app_tile(key, 28.0, cx))
        .into_any_element()
}

fn default_ledger_entries(cx: &App) -> Vec<LedgerEntry> {
    vec![
        LedgerEntry::new(
            "email",
            tr(cx, "Sent launch email", "已发送上线邮件"),
            "Gmail",
        )
        .with_target(tr(cx, "3,812 customers", "3,812 位客户"))
        .with_age_secs(0)
        .with_detail(tr(
            cx,
            "New Pro pricing & launch code KENSE20",
            "新版 Pro 定价说明与优惠码 KENSE20",
        ))
        .with_reversible(false)
        .with_depends_on(["price", "deploy"]),
        LedgerEntry::new(
            "deals",
            tr(cx, "Updated 212 deals", "更新 212 笔商机"),
            "HubSpot",
        )
        .with_target(tr(cx, "Renewals in Q4", "Q4 续约合同"))
        .with_age_secs(25)
        .with_change("$29 / seat", "$34 / seat")
        .with_depends_on(["price"]),
        LedgerEntry::new("price", tr(cx, "Changed price", "调整套餐单价"), "Stripe")
            .with_target(tr(cx, "Pro plan", "Pro 专业版"))
            .with_age_secs(60)
            .with_change("$29 / month", "$34 / month"),
        LedgerEntry::new("deploy", tr(cx, "Deployed", "部署生产站点"), "Vercel")
            .with_target("kense.dev")
            .with_age_secs(120)
            .with_change("#481", tr(cx, "#483 in production", "#483 生产生效"))
            .with_depends_on(["pr"]),
        LedgerEntry::new("pr", tr(cx, "Merged PR #482", "合并 PR #482"), "GitHub")
            .with_target(tr(cx, "Pricing page v2", "新版定价页 v2"))
            .with_age_secs(180)
            .with_change("pricing-v2", "main"),
    ]
}

/// System-mutation audit ledger with `before -> after` diffs and dependency-aware Undo.
///
/// # Examples
///
/// ```no_run
/// # #[allow(unused_variables, unused_imports)]
/// # fn example(window: &mut gpui::Window, cx: &mut gpui::App) {
/// # use gpui::prelude::*;
/// use gpui::prelude::*;
/// use kui::components::agent_ledger::{AgentLedger, AgentLedgerEvent};
///
/// let ledger = cx.new(|cx| {
///     AgentLedger::new("launch-ledger", cx)
///         .with_agent("Launch agent")
/// });
/// cx.subscribe(&ledger, |_, ev: &AgentLedgerEvent, _| {
///     let _ = ev;
/// }).detach();
/// # }
/// ```
pub struct AgentLedger {
    id: ElementId,
    title: Option<SharedString>,
    agent: Option<SharedString>,
    entries: Vec<LedgerEntry>,
    undone: HashSet<SharedString>,
    point_at: Option<SharedString>,
    hovered_undo: Option<SharedString>,
    loading: bool,
    empty: bool,
    started_at: Instant,
    measured_w: Rc<Cell<f32>>,
    toast: Option<(SharedString, Instant)>,
}

impl AgentLedger {
    /// Create an [`AgentLedger`] preloaded with the 5-mutation `"Launch agent"` audit trail.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        Self {
            id: id.into(),
            title: None,
            agent: Some(tr(cx, "Launch agent", "发布智能体").into()),
            entries: default_ledger_entries(cx),
            undone: HashSet::new(),
            point_at: None,
            hovered_undo: None,
            loading: false,
            empty: false,
            started_at: Instant::now(),
            measured_w: Rc::new(Cell::new(680.0)),
            toast: None,
        }
    }

    /// Override the ledger heading (`"Ledger"` by default).
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the agent attribution label beside the title.
    pub fn with_agent(mut self, agent: impl Into<SharedString>) -> Self {
        self.agent = Some(agent.into());
        self
    }

    /// Provide the list of [`LedgerEntry`] mutations (automatically sorted newest-first).
    pub fn with_entries(mut self, entries: Vec<LedgerEntry>) -> Self {
        self.entries = entries;
        self
    }

    /// Pre-populate mutation ids that start in the undone state.
    pub fn with_undone<I, S>(mut self, ids: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        self.undone = ids.into_iter().map(Into::into).collect();
        self
    }

    /// Programmatically spotlight an entry's `Undo` blast radius (for guided walkthroughs).
    pub fn with_point_at(mut self, point_at: Option<SharedString>) -> Self {
        self.point_at = point_at;
        self
    }

    /// Render skeleton rows while audit mutations load.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Render the empty audit ledger state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    fn perform_undo(&mut self, root_id: SharedString, cx: &mut Context<Self>) {
        let Some(entry) = self.entries.iter().find(|e| e.id == root_id) else {
            return;
        };
        if !entry.reversible || self.undone.contains(&root_id) {
            return;
        }
        let blast = compute_undo_blast(&self.entries, &self.undone, root_id.as_ref());
        let mut reverted = Vec::with_capacity(1 + blast.also_undone.len());
        reverted.push(root_id.clone());
        self.undone.insert(root_id);
        for dep_id in blast.also_undone {
            self.undone.insert(dep_id.clone());
            reverted.push(dep_id);
        }
        self.hovered_undo = None;

        let msg: SharedString = if reverted.len() > 1 {
            format!(
                "{} ({})",
                tr(cx, "Rolled back mutation chain", "已回滚关联变更链"),
                reverted.len()
            )
            .into()
        } else {
            tr(cx, "Undid mutation", "已撤销该项变更").into()
        };
        self.toast = Some((msg, Instant::now()));
        cx.emit(AgentLedgerEvent::Undo(reverted));
        cx.notify();
    }

    fn perform_redo(&mut self, target_id: SharedString, cx: &mut Context<Self>) {
        let chain = compute_redo_chain(&self.entries, &self.undone, target_id.as_ref());
        if chain.is_empty() {
            return;
        }
        for id in &chain {
            self.undone.remove(id);
        }
        let msg: SharedString = tr(cx, "Reapplied mutation", "已重做变更").into();
        self.toast = Some((msg, Instant::now()));
        cx.emit(AgentLedgerEvent::Redo(chain));
        cx.notify();
    }
}

impl Render for AgentLedger {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let reduced = disabled_motion(cx) || Theme::global(cx).reduced_motion;
        let elapsed = self.started_at.elapsed().as_secs_f32();
        let panel_bg: Hsla = if p.dark {
            rgb(0x1c1c1f).into()
        } else {
            rgb(0xffffff).into()
        };
        let title_str: SharedString = self
            .title
            .clone()
            .unwrap_or_else(|| tr(cx, "Ledger", "系统变更账本").into());

        if self.loading {
            if !reduced {
                window.request_animation_frame();
            }
            let phase = (elapsed / 1.4).fract();
            return div()
                .id(self.id.clone())
                .role(Role::Group)
                .aria_label(title_str)
                .w_full()
                .flex()
                .flex_col()
                .gap(px(14.0))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(pro_skeleton_bar(150.0, 16.0, 5.0, phase, cx))
                        .child(pro_skeleton_bar(80.0, 14.0, 4.0, phase + 0.15, cx)),
                )
                .child(
                    div()
                        .p(px(6.0))
                        .rounded(px(10.0))
                        .bg(panel_bg)
                        .border_1()
                        .border_color(p.line)
                        .shadow(pro_card_shadow(p.dark))
                        .flex()
                        .flex_col()
                        .gap(px(4.0))
                        .children((0..4usize).map(|i| {
                            div()
                                .h(px(54.0))
                                .px(px(10.0))
                                .flex()
                                .items_center()
                                .gap(px(12.0))
                                .child(pro_skeleton_bar(
                                    28.0,
                                    28.0,
                                    7.0,
                                    phase + i as f32 * 0.08,
                                    cx,
                                ))
                                .child(
                                    div()
                                        .flex_1()
                                        .flex()
                                        .flex_col()
                                        .gap(px(6.0))
                                        .child(pro_skeleton_bar(
                                            190.0,
                                            13.0,
                                            4.0,
                                            phase + i as f32 * 0.1,
                                            cx,
                                        ))
                                        .child(pro_skeleton_bar(
                                            130.0,
                                            11.0,
                                            4.0,
                                            phase + i as f32 * 0.12,
                                            cx,
                                        )),
                                )
                        })),
                );
        }

        if self.toast.is_some() && !reduced {
            window.request_animation_frame();
        }

        // Sort newest-first (`age_secs` ascending, preserving slice order on ties).
        let mut sorted_indices: Vec<usize> = (0..self.entries.len()).collect();
        sorted_indices.sort_by_key(|&idx| (self.entries[idx].age_secs, idx));

        let total_count = self.entries.len();
        let undone_count = self
            .entries
            .iter()
            .filter(|e| self.undone.contains(&e.id))
            .count();

        let width_px = self.measured_w.get().max(260.0);
        let compact = width_px < 480.0;

        let top_bar = div()
            .w_full()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_x(px(8.0))
            .gap_y(px(4.0))
            .child(pro_svg("0 0 16 16", LEDGER_DOC_SVG, 16.0, p.blue))
            .child(
                div()
                    .text_size(px(13.0))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .child(title_str.clone()),
            )
            .when_some(self.agent.clone(), |d, ag| {
                d.child(
                    div()
                        .text_size(px(13.0))
                        .text_color(p.muted)
                        .truncate()
                        .child(ag),
                )
            })
            .child(
                div()
                    .when(!compact, |d| d.ml_auto())
                    .when(compact, |d| d.w_full().justify_between())
                    .flex()
                    .items_center()
                    .gap(px(12.0))
                    .text_size(px(12.0))
                    .text_color(p.muted)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(4.0))
                            .child(pro_rolling_number(
                                pro_eid(&self.id, "led-tot-cnt"),
                                total_count as f64,
                                0,
                                "",
                                "",
                                12.0,
                                FontWeight::MEDIUM,
                                p.ink,
                            ))
                            .child(tr(cx, "changes", "项变更")),
                    )
                    .child(pro_presence(
                        pro_eid(&self.id, "led-undone-pres"),
                        undone_count > 0,
                        0.18,
                        0.14,
                        {
                            let undone_lbl = tr(cx, "undone", "已撤销").to_string();
                            let cnt_el = pro_rolling_number(
                                pro_eid(&self.id, "led-und-cnt"),
                                undone_count.max(1) as f64,
                                0,
                                "",
                                "",
                                12.0,
                                FontWeight::MEDIUM,
                                p.ink,
                            );
                            let mut cnt_opt = Some(cnt_el);
                            move |amt| {
                                div()
                                    .opacity(amt)
                                    .flex()
                                    .items_center()
                                    .gap(px(4.0))
                                    .children(cnt_opt.take())
                                    .child(undone_lbl.clone())
                                    .into_any_element()
                            }
                        },
                    )),
            );

        if self.empty || self.entries.is_empty() {
            return div()
                .id(self.id.clone())
                .role(Role::Group)
                .aria_label(title_str)
                .w_full()
                .flex()
                .flex_col()
                .gap(px(14.0))
                .child(top_bar)
                .child(
                    div()
                        .rounded(px(10.0))
                        .bg(panel_bg)
                        .border_1()
                        .border_color(p.line)
                        .shadow(pro_card_shadow(p.dark))
                        .child(pro_empty_state(
                            ProIcon::ClipboardDoc,
                            tr(cx, "No system changes yet", "尚无外部系统变更"),
                            tr(
                                cx,
                                "Every record an agent creates, edits, or deploys appears here with before/after values and dependency-safe Undo.",
                                "智能体在外部系统中执行的创建、修改或部署操作都将在此记录并支持依赖感知回滚。",
                            ),
                            None,
                            cx,
                        )),
                );
        }

        let active_preview_id = self
            .hovered_undo
            .clone()
            .or_else(|| self.point_at.clone())
            .filter(|id| !self.undone.contains(id));

        let blast = active_preview_id
            .as_ref()
            .map(|pid| compute_undo_blast(&self.entries, &self.undone, pid.as_ref()))
            .unwrap_or_default();
        let also_set: HashSet<&str> = blast.also_undone.iter().map(|s| s.as_ref()).collect();
        let kept_set: HashSet<&str> = blast.kept_irreversible.iter().map(|s| s.as_ref()).collect();

        let mut list_panel = div()
            .id(pro_eid(&self.id, "ledger-list"))
            .role(Role::ListBox)
            .aria_label(title_str.clone())
            .w_full()
            .p(px(4.0))
            .rounded(px(10.0))
            .bg(panel_bg)
            .border_1()
            .border_color(p.line)
            .shadow(pro_card_shadow(p.dark))
            .flex()
            .flex_col();

        for (pos, &entry_idx) in sorted_indices.iter().enumerate() {
            let entry = &self.entries[entry_idx];
            let eid = entry.id.clone();
            let is_undone = self.undone.contains(&eid);
            let is_preview_root = active_preview_id.as_ref() == Some(&eid);
            let is_also = also_set.contains(eid.as_ref());
            let is_kept = kept_set.contains(eid.as_ref());

            let strike_amt = pro_spring_f32(
                pro_eid(&self.id, format!("strike-{eid}")),
                if is_undone { 1.0 } else { 0.0 },
                0.32,
                window,
                cx,
            )
            .clamp(0.0, 1.0);

            let root_t = pro_spring_f32(
                pro_eid(&self.id, format!("blast-root-{eid}")),
                if is_preview_root { 1.0 } else { 0.0 },
                0.22,
                window,
                cx,
            );
            let also_t = pro_spring_f32(
                pro_eid(&self.id, format!("blast-also-{eid}")),
                if is_also { 1.0 } else { 0.0 },
                0.22,
                window,
                cx,
            );
            let kept_t = pro_spring_f32(
                pro_eid(&self.id, format!("blast-kept-{eid}")),
                if is_kept { 1.0 } else { 0.0 },
                0.22,
                window,
                cx,
            );

            let row_bg: Option<Hsla> = if root_t > 0.01 {
                Some(pro_mix_color(
                    transparent_black(),
                    p.blue.opacity(if p.dark { 0.11 } else { 0.065 }),
                    root_t,
                ))
            } else if also_t > 0.01 {
                Some(pro_mix_color(
                    transparent_black(),
                    p.blue.opacity(if p.dark { 0.075 } else { 0.045 }),
                    also_t,
                ))
            } else if kept_t > 0.01 {
                Some(pro_mix_color(
                    transparent_black(),
                    p.amber.opacity(if p.dark { 0.10 } else { 0.08 }),
                    kept_t,
                ))
            } else {
                None
            };

            let show_hairline = pos > 0 && row_bg.is_none();
            let hover_row_bg = p.hover.opacity(0.6);

            // Second line: `before -> after` OR `detail`.
            let second_line: AnyElement = if let Some((before, after)) = &entry.change {
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .gap(px(5.0))
                    .overflow_hidden()
                    .text_size(px(12.0))
                    .line_height(px(17.0))
                    .child(
                        div()
                            .flex_none()
                            .text_color(p.muted)
                            .line_through()
                            .child(before.clone()),
                    )
                    .child(pro_icon(ProIcon::ArrowRight, 11.0, p.faint))
                    .child(
                        div()
                            .min_w_0()
                            .text_color(if is_undone { p.muted } else { p.text })
                            .truncate()
                            .child(after.clone()),
                    )
                    .into_any_element()
            } else if let Some(det) = &entry.detail {
                div()
                    .w_full()
                    .text_size(px(12.0))
                    .line_height(px(17.0))
                    .text_color(p.muted)
                    .truncate()
                    .child(det.clone())
                    .into_any_element()
            } else {
                div().into_any_element()
            };

            // Compute dependent count for accessible Undo label.
            let own_blast = if entry.reversible && !is_undone {
                compute_undo_blast(&self.entries, &self.undone, eid.as_ref())
                    .also_undone
                    .len()
            } else {
                0
            };

            let time_str = if compact {
                format_relative_age(entry.age_secs, cx)
            } else {
                format!(
                    "{} · {}",
                    format_relative_age(entry.age_secs, cx),
                    entry.system
                )
            };

            let right_slot: AnyElement = if is_also {
                div()
                    .h(px(22.0))
                    .px(px(7.0))
                    .rounded(px(5.0))
                    .bg(p.accent_soft)
                    .text_size(px(11.5))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.blue)
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .child(pro_icon(ProIcon::Undo, 11.5, p.blue))
                    .child(tr(cx, "Also undone", "一并撤销"))
                    .into_any_element()
            } else if is_kept {
                div()
                    .h(px(22.0))
                    .px(px(7.0))
                    .rounded(px(5.0))
                    .bg(p.amber_soft)
                    .text_size(px(11.5))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.amber_ink)
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .child(pro_icon(ProIcon::Lock, 11.5, p.amber_ink))
                    .child(tr(cx, "Can't undo", "无法撤回"))
                    .into_any_element()
            } else if !entry.reversible {
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .when(!compact, |d| {
                        d.child(
                            div()
                                .text_size(px(12.0))
                                .text_color(p.muted)
                                .child(time_str),
                        )
                    })
                    .child(
                        div()
                            .h(px(24.0))
                            .px(px(7.0))
                            .rounded(px(6.0))
                            .bg(p.chip)
                            .text_size(px(11.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.muted)
                            .flex()
                            .items_center()
                            .gap(px(4.0))
                            .child(pro_icon(ProIcon::Lock, 12.0, p.muted))
                            .when(!compact, |d| d.child(tr(cx, "Irreversible", "不可逆"))),
                    )
                    .into_any_element()
            } else if is_undone {
                let redo_id = eid.clone();
                let aria = match &entry.target {
                    Some(t) => format!("Redo: {} {t}", entry.action),
                    None => format!("Redo: {}", entry.action),
                };
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .when(!compact, |d| {
                        d.child(
                            div()
                                .text_size(px(12.0))
                                .text_color(p.faint)
                                .child(time_str),
                        )
                    })
                    .child(
                        pro_tactile(
                            pro_eid(&self.id, format!("redo-{eid}")),
                            aria,
                            6.0,
                            p.chip,
                            p.chip_hover,
                            transparent_black(),
                            p.line,
                            false,
                            div()
                                .h(px(24.0))
                                .px(px(if compact { 6.0 } else { 8.0 }))
                                .text_size(px(12.0))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.ink)
                                .flex()
                                .items_center()
                                .gap(px(5.0))
                                .child(pro_svg("0 0 24 24", REDO_SVG, 12.5, p.ink))
                                .when(!compact, |d| d.child(tr(cx, "Redo", "重做"))),
                            cx,
                            move |this, _, cx| this.perform_redo(redo_id.clone(), cx),
                        )
                        .w_auto(),
                    )
                    .into_any_element()
            } else {
                let undo_id = eid.clone();
                let hov_id = eid.clone();
                let btn_bg = if is_preview_root {
                    p.accent_soft
                } else {
                    p.chip
                };
                let btn_fg = if is_preview_root { p.blue } else { p.ink };
                let base_desc = match &entry.target {
                    Some(t) => format!("{} {t}", entry.action),
                    None => entry.action.to_string(),
                };
                let aria = if own_blast > 0 {
                    format!("Undo: {base_desc} – and {own_blast} dependent changes")
                } else {
                    format!("Undo: {base_desc}")
                };

                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .when(!compact, |d| {
                        d.child(
                            div()
                                .text_size(px(12.0))
                                .text_color(p.muted)
                                .child(time_str),
                        )
                    })
                    .child(
                        div()
                            .id(pro_eid(&self.id, format!("undo-hov-{eid}")))
                            .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                                let next = if *inside {
                                    Some(hov_id.clone())
                                } else if this.hovered_undo.as_ref() == Some(&hov_id) {
                                    None
                                } else {
                                    this.hovered_undo.clone()
                                };
                                if this.hovered_undo != next {
                                    this.hovered_undo = next.clone();
                                    cx.emit(AgentLedgerEvent::PreviewChanged(next));
                                    cx.notify();
                                }
                            }))
                            .child(
                                pro_tactile(
                                    pro_eid(&self.id, format!("undo-{eid}")),
                                    aria,
                                    6.0,
                                    btn_bg,
                                    p.accent_soft,
                                    transparent_black(),
                                    p.blue.opacity(0.45),
                                    is_preview_root,
                                    div()
                                        .h(px(24.0))
                                        .px(px(if compact { 6.0 } else { 8.0 }))
                                        .text_size(px(12.0))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(btn_fg)
                                        .flex()
                                        .items_center()
                                        .gap(px(5.0))
                                        .child(pro_icon(ProIcon::Undo, 12.5, btn_fg))
                                        .when(!compact, |d| d.child(tr(cx, "Undo", "撤销"))),
                                    cx,
                                    move |this, _, cx| this.perform_undo(undo_id.clone(), cx),
                                )
                                .w_auto(),
                            ),
                    )
                    .into_any_element()
            };

            let strike_c: Hsla = if p.dark {
                hsla(0., 0., 1., 0.50)
            } else {
                hsla(0., 0., 0., 0.45)
            };

            let what_line = div()
                .relative()
                .flex()
                .items_baseline()
                .gap(px(6.0))
                .max_w_full()
                .min_w_0()
                .overflow_hidden()
                .child(
                    div()
                        .flex_none()
                        .text_size(px(13.0))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(if is_undone { p.muted } else { p.ink })
                        .child(entry.action.clone()),
                )
                .when_some(entry.target.clone(), |d, tgt| {
                    d.child(
                        div()
                            .min_w_0()
                            .text_size(px(13.0))
                            .text_color(if is_undone { p.muted } else { p.text })
                            .truncate()
                            .child(tgt),
                    )
                })
                .when(strike_amt > 0.01, |d| {
                    d.child(
                        div()
                            .absolute()
                            .top(relative(0.52))
                            .left_0()
                            .w(relative(strike_amt))
                            .h(px(1.0))
                            .rounded(px(1.0))
                            .bg(strike_c),
                    )
                });

            let mut row_wrap = div()
                .id(pro_eid(&self.id, format!("lrow-{eid}")))
                .role(Role::ListBoxOption)
                .relative()
                .w_full()
                .min_h(px(56.0))
                .px(px(if compact { 8.0 } else { 10.0 }))
                .py(px(8.0))
                .rounded(px(7.0))
                .when_some(row_bg, |d, bg| d.bg(bg))
                .when(row_bg.is_none(), |d| d.hover(move |s| s.bg(hover_row_bg)))
                .flex()
                .items_center()
                .gap(px(if compact { 10.0 } else { 12.0 }));

            if show_hairline {
                row_wrap = row_wrap.child(
                    div()
                        .absolute()
                        .top_0()
                        .left(px(if compact { 46.0 } else { 50.0 }))
                        .right(px(10.0))
                        .h(px(1.0))
                        .bg(p.line),
                );
            }

            row_wrap = row_wrap
                .child(render_ledger_app_tile(entry, is_undone, cx))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .items_start()
                        .child(what_line)
                        .child(second_line),
                )
                .child(div().flex_none().child(right_slot));

            list_panel = list_panel.child(row_wrap);
        }

        let measure_cell = self.measured_w.clone();
        div()
            .id(self.id.clone())
            .role(Role::Group)
            .aria_label(title_str)
            .relative()
            .w_full()
            .min_w_0()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .child(
                canvas(
                    move |b, window, _| {
                        let nw = f32::from(b.size.width);
                        if nw > 40.0 && (measure_cell.get() - nw).abs() > 1.0 {
                            measure_cell.set(nw);
                            window.request_animation_frame();
                        }
                    },
                    |_, _, _, _| (),
                )
                .w_full()
                .h(px(0.0)),
            )
            .child(top_bar)
            .child(list_panel)
            .child(pro_toast_banner(self.toast.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::{LedgerEntry, compute_redo_chain, compute_undo_blast};
    use gpui::SharedString;
    use std::collections::HashSet;

    #[test]
    fn undo_blast_separates_reversible_dependents_from_irreversible_ones() {
        let entries = vec![
            LedgerEntry::new("price", "Changed price", "Stripe"),
            LedgerEntry::new("deals", "Updated deals", "HubSpot").with_depends_on(["price"]),
            LedgerEntry::new("quotes", "Synced quotes", "HubSpot").with_depends_on(["deals"]),
            LedgerEntry::new("email", "Sent email", "Gmail")
                .with_reversible(false)
                .with_depends_on(["price"]),
        ];

        let undone = HashSet::new();
        let blast = compute_undo_blast(&entries, &undone, "price");
        assert_eq!(blast.also_undone.len(), 2);
        assert!(blast.also_undone.contains(&SharedString::from("deals")));
        assert!(blast.also_undone.contains(&SharedString::from("quotes")));
        assert_eq!(blast.kept_irreversible, vec![SharedString::from("email")]);

        let mut after_undo = HashSet::new();
        after_undo.insert(SharedString::from("price"));
        after_undo.insert(SharedString::from("deals"));
        after_undo.insert(SharedString::from("quotes"));

        // Redoing `quotes` must also bring back its undone prerequisites `deals` and `price`.
        let redo = compute_redo_chain(&entries, &after_undo, "quotes");
        assert_eq!(redo.len(), 3);
    }
}
