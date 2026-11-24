//! Multi-user agent collision radar with Bezier tangle curves and conflict resolution.
//!
//! Maps where teammates' AI coding agents collide across files, prices, or duplicate work—linking
//! left/right agent chips to severity-coded conflict badges via smooth Bezier curves and listing synced peers.

use crate::{
    components::pro_chrome::{
        ProBtnStyle, ProPalette, pro_agent_avatar, pro_btn, pro_eid, pro_empty_state, pro_icon_btn,
        pro_mark_selected, pro_mix_color, pro_pill_indicator, pro_popover_presence, pro_presence,
        pro_rolling_number, pro_selection_sensor, pro_skeleton_bar, pro_spring_f32, pro_tactile,
        pro_toast_banner, pro_view_transition, tr,
    },
    foundation::Theme,
    primitives::{
        control::activate,
        pro_icons::{AGENT_WAVE_MS, ProIcon, agent_pixel_cells, pro_app_artwork},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, collections::HashMap, rc::Rc};
use web_time::Instant;

/// Severity level of an [`OverlapConflict`] inside [`AgentOverlapMap`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OverlapSeverity {
    /// Red solid curve & badge: requires an immediate team decision (`"Needs a decision"`).
    High,
    /// Amber solid curve & badge: worth coordinating (`"Heads-up"`).
    #[default]
    Medium,
    /// Grey dashed curve & badge: low-risk overlap (`"Minor"`).
    Low,
}

impl OverlapSeverity {
    fn legend_label(self, cx: &App) -> &'static str {
        match self {
            Self::High => tr(cx, "Needs a decision", "需立即决策"),
            Self::Medium => tr(cx, "Heads-up", "留意协调"),
            Self::Low => tr(cx, "Minor", "轻微重叠"),
        }
    }
}

/// Team member whose AI agent(s) participate in [`AgentOverlapMap`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OverlapPerson {
    /// Stable person identifier referenced by [`OverlapSide::person_id`].
    pub id: SharedString,
    /// Full display name (the first name is shown on compact agent chips).
    pub name: SharedString,
    /// Current area of work inspected inside the agent card.
    pub note: SharedString,
}

impl OverlapPerson {
    /// Construct an [`OverlapPerson`] with `id`, `name`, and `note`.
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        note: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            note: note.into(),
        }
    }
}

/// One participating `(person, agent)` endpoint in an [`OverlapConflict`].
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct OverlapSide {
    /// Identifier of the owning [`OverlapPerson`].
    pub person_id: SharedString,
    /// Agent product or harness display name (for example `"Claude Code"`, `"Codex"`, `"Gemini CLI"`).
    pub agent_name: SharedString,
    /// Provider logo key (`"claude"`, `"openai"`, `"gemini"`).
    pub harness: SharedString,
}

impl OverlapSide {
    /// Construct an [`OverlapSide`] linking `person_id` and `agent_name`.
    pub fn new(
        person_id: impl Into<SharedString>,
        agent_name: impl Into<SharedString>,
        harness: impl Into<SharedString>,
    ) -> Self {
        Self {
            person_id: person_id.into(),
            agent_name: agent_name.into(),
            harness: harness.into(),
        }
    }

    fn key(&self) -> SharedString {
        format!("{}::{}", self.person_id, self.agent_name).into()
    }
}

/// Detected collision between two or more teammates' AI agents.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OverlapConflict {
    /// Stable conflict identifier.
    pub id: SharedString,
    /// Short badge label (`"Same files"`, `"Contradicts"`, `"Out of sync"`, `"Same work"`).
    pub label: SharedString,
    /// Full human-readable explanation shown inside the conflict resolution card.
    pub title: SharedString,
    /// Severity tier controlling curve style and badge color.
    pub severity: OverlapSeverity,
    /// Participating `(person, agent)` sides involved in this collision.
    pub sides: Vec<OverlapSide>,
}

impl OverlapConflict {
    /// Construct an [`OverlapConflict`] with `id`, `label`, `title`, `severity`, and `sides`.
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        title: impl Into<SharedString>,
        severity: OverlapSeverity,
        sides: Vec<OverlapSide>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            title: title.into(),
            severity,
            sides,
        }
    }
}

/// Typed events emitted by [`AgentOverlapMap`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentOverlapMapEvent {
    /// User clicked a conflict badge to open its resolution card.
    ConflictSelected {
        /// Conflict identifier.
        conflict_id: SharedString,
    },
    /// User clicked an agent chip to inspect all conflicts involving that agent.
    AgentSelected {
        /// Person identifier.
        person_id: SharedString,
        /// Agent name.
        agent_name: SharedString,
    },
    /// Operator resolved an open conflict from its resolution dialog.
    ConflictResolved {
        /// Resolved conflict identifier.
        conflict_id: SharedString,
    },
}

impl EventEmitter<AgentOverlapMapEvent> for AgentOverlapMap {}

#[derive(Clone, Debug, PartialEq, Eq)]
enum OverlapHover {
    Conflict(SharedString),
    Agent(SharedString),
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum OverlapModal {
    Conflict(SharedString),
    Agent(SharedString),
}

#[derive(Clone, Debug)]
struct AgentSlot {
    key: SharedString,
    person_id: SharedString,
    first_name: SharedString,
    full_name: SharedString,
    note: SharedString,
    agent_name: SharedString,
    harness: SharedString,
    color_idx: usize,
    left_col: bool,
}

/// Multi-user AI agent collision radar with Bezier tangle curves, hover focus, and conflict resolution.
///
/// # Examples
///
/// ```ignore
/// use kui::components::agent_overlap_map::AgentOverlapMap;
///
/// let map = cx.new(|cx| AgentOverlapMap::new("agent-overlap-map", cx));
/// ```
pub struct AgentOverlapMap {
    id: ElementId,
    focus: FocusHandle,
    title: Option<SharedString>,
    people: Vec<OverlapPerson>,
    conflicts: Vec<OverlapConflict>,
    custom_data: bool,
    severity_filter: Option<OverlapSeverity>,
    hover: Option<OverlapHover>,
    modal: Option<OverlapModal>,
    last_modal: Option<OverlapModal>,
    toast: Option<(SharedString, Instant)>,
    loading: bool,
    empty: bool,
    boot: Instant,
    measured_width: Rc<Cell<f32>>,
}

impl AgentOverlapMap {
    /// Create a live [`AgentOverlapMap`] with 9 teammates, 8 open collisions, and `"Jonas is in sync"`.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let (people, conflicts) = default_overlap_dataset();
        Self {
            id: id.into(),
            focus: cx.focus_handle(),
            title: None,
            people,
            conflicts,
            custom_data: false,
            severity_filter: None,
            hover: None,
            modal: None,
            last_modal: None,
            toast: None,
            loading: false,
            empty: false,
            boot: Instant::now(),
            measured_width: Rc::new(Cell::new(780.0)),
        }
    }

    /// Override the section heading (defaults to `"Agent overlaps"` / `"智能体工作冲突雷达"`).
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Replace the workspace team members.
    pub fn with_people(mut self, people: Vec<OverlapPerson>) -> Self {
        self.custom_data = true;
        if people.is_empty() {
            self.people.clear();
            self.empty = true;
        } else {
            self.people = people;
        }
        self
    }

    /// Replace the open conflict records.
    pub fn with_conflicts(mut self, conflicts: Vec<OverlapConflict>) -> Self {
        self.custom_data = true;
        self.conflicts = conflicts;
        self
    }

    /// Render skeleton placeholders when `loading` is `true`.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Force the empty state when `empty` is `true`.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Return the count of remaining open conflicts.
    pub fn conflict_count(&self) -> usize {
        self.valid_conflicts().len()
    }

    /// Return names of team members whose agents currently have zero conflicts (`"In sync"`).
    pub fn synced_people(&self) -> Vec<SharedString> {
        let valid = self.valid_conflicts();
        self.people
            .iter()
            .filter(|p| {
                !valid
                    .iter()
                    .any(|c| c.sides.iter().any(|s| s.person_id == p.id))
            })
            .map(|p| first_word_name(&p.name).into())
            .collect()
    }

    /// Resolve a conflict by id, removing it from the open radar and updating synced teammates.
    pub fn resolve_conflict(
        &mut self,
        conflict_id: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) {
        let target = conflict_id.into();
        let mut label = target.clone();
        if let Some(pos) = self.conflicts.iter().position(|c| c.id == target) {
            label = self.conflicts[pos].label.clone();
            self.conflicts.remove(pos);
        }
        self.modal = None;
        self.hover = None;
        self.toast = Some((
            format!("{} ({label})", tr(cx, "Resolved overlap", "已解决冲突")).into(),
            Instant::now(),
        ));
        cx.emit(AgentOverlapMapEvent::ConflictResolved {
            conflict_id: target,
        });
        cx.notify();
    }

    /// Restore the default 8-conflict dataset when all overlaps have been resolved.
    pub fn reset_defaults(&mut self, cx: &mut Context<Self>) {
        if !self.custom_data {
            let (people, conflicts) = default_overlap_dataset();
            self.people = people;
            self.conflicts = conflicts;
            self.modal = None;
            cx.notify();
        }
    }

    /// Toggle skeleton loading state at runtime.
    pub fn set_loading(&mut self, loading: bool, cx: &mut Context<Self>) {
        if self.loading != loading {
            self.loading = loading;
            cx.notify();
        }
    }

    /// Return whether skeleton loading is active.
    pub fn is_loading(&self) -> bool {
        self.loading
    }

    /// Toggle empty state at runtime.
    pub fn set_empty(&mut self, empty: bool, cx: &mut Context<Self>) {
        if self.empty != empty {
            self.empty = empty;
            cx.notify();
        }
    }

    /// Return whether the component is forced empty.
    pub fn is_empty(&self) -> bool {
        self.empty || self.people.is_empty()
    }

    fn valid_conflicts(&self) -> Vec<OverlapConflict> {
        let known: Vec<&SharedString> = self.people.iter().map(|p| &p.id).collect();
        let mut out = Vec::new();
        for c in &self.conflicts {
            let mut dedup_sides: Vec<OverlapSide> = Vec::new();
            for s in &c.sides {
                if known.contains(&&s.person_id) && !dedup_sides.iter().any(|x| x.key() == s.key())
                {
                    dedup_sides.push(s.clone());
                }
            }
            if !dedup_sides.is_empty() {
                let mut copy = c.clone();
                copy.sides = dedup_sides;
                out.push(copy);
            }
        }
        out
    }

    fn build_slots(&self, valid_conflicts: &[OverlapConflict], two_sided: bool) -> Vec<AgentSlot> {
        let mut slots: Vec<AgentSlot> = Vec::new();
        let preferred_left = [
            "lena::Codex",
            "priya::Claude Code",
            "ava::Claude Code",
            "elif::Gemini CLI",
            "sam::Claude Code",
        ];
        for c in valid_conflicts {
            for s in &c.sides {
                let k = s.key();
                if slots.iter().any(|x| x.key == k) {
                    continue;
                }
                let Some(person) = self.people.iter().find(|p| p.id == s.person_id) else {
                    continue;
                };
                let idx = slots.len();
                let left_col = if !two_sided {
                    true
                } else if !self.custom_data {
                    preferred_left.contains(&k.as_ref())
                } else {
                    idx.is_multiple_of(2)
                };
                slots.push(AgentSlot {
                    key: k,
                    person_id: person.id.clone(),
                    first_name: first_word_name(&person.name).into(),
                    full_name: person.name.clone(),
                    note: person.note.clone(),
                    agent_name: s.agent_name.clone(),
                    harness: s.harness.clone(),
                    color_idx: idx,
                    left_col,
                });
            }
        }
        if !self.custom_data && two_sided {
            let order = [
                "lena::Codex",
                "priya::Claude Code",
                "ava::Claude Code",
                "elif::Gemini CLI",
                "sam::Claude Code",
                "marco::Gemini CLI",
                "sam::Codex",
                "tom::Claude Code",
                "noah::Codex",
            ];
            slots.sort_by_key(|sl| {
                order
                    .iter()
                    .position(|&o| o == sl.key.as_ref())
                    .unwrap_or(99)
            });
            for (i, sl) in slots.iter_mut().enumerate() {
                sl.color_idx = i;
            }
        }
        slots
    }

    fn render_conflict_dialog(
        &self,
        conflict: &OverlapConflict,
        slots: &[AgentSlot],
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let cid = conflict.id.clone();
        let disp_lbl = if self.custom_data {
            conflict.label.clone()
        } else {
            tr_conflict_label(&conflict.label, cx).into()
        };
        let (sev_bg, sev_fg) = severity_badge_colors(conflict.severity, p);

        let close_btn = pro_icon_btn(
            pro_eid(&self.id, "o-close-cdlg"),
            tr(cx, "Close conflict card", "关闭冲突卡片"),
            ProIcon::Close,
            24.,
            false,
            cx,
            |this, w, cx| {
                this.modal = None;
                this.focus.focus(w, cx);
                cx.notify();
            },
        );

        let mut involved = div()
            .w_full()
            .pt(px(8.))
            .border_t_1()
            .border_color(p.line)
            .flex()
            .flex_col()
            .gap(px(6.));
        for s in &conflict.sides {
            let k = s.key();
            if let Some(sl) = slots.iter().find(|x| x.key == k) {
                let col = slot_color(sl.color_idx, p);
                involved = involved.child(
                    div()
                        .w_full()
                        .p(px(6.))
                        .rounded(px(7.))
                        .bg(p.subtle)
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(pro_agent_avatar(&sl.key, col, 22., false, 0., cx))
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
                                        .gap(px(5.))
                                        .text_size(px(12.5))
                                        .child(
                                            div()
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(p.strong)
                                                .child(sl.full_name.clone()),
                                        )
                                        .children(pro_app_artwork(&sl.harness, 11.5, cx))
                                        .child(
                                            div()
                                                .text_size(px(11.5))
                                                .text_color(p.muted)
                                                .child(sl.agent_name.clone()),
                                        ),
                                )
                                .when(!sl.note.is_empty(), |d| {
                                    d.child(
                                        div()
                                            .text_size(px(11.5))
                                            .text_color(p.muted)
                                            .truncate()
                                            .child(sl.note.clone()),
                                    )
                                }),
                        ),
                );
            }
        }

        let res_id = cid.clone();
        let dialog = div()
            .id(pro_eid(&self.id, "o-cdlg"))
            .role(Role::Dialog)
            .aria_label(disp_lbl.clone())
            .on_click(|_, _, cx| cx.stop_propagation())
            .w(px(348.))
            .max_w_full()
            .rounded(px(10.))
            .bg(p.card)
            .border_1()
            .border_color(p.line_strong)
            .shadow_lg()
            .p(px(12.))
            .flex()
            .flex_col()
            .gap(px(9.))
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .gap(px(7.))
                    .child(
                        div()
                            .h(px(21.))
                            .px(px(8.))
                            .rounded(px(5.))
                            .bg(sev_bg)
                            .text_color(sev_fg)
                            .text_size(px(11.5))
                            .font_weight(FontWeight::MEDIUM)
                            .flex()
                            .items_center()
                            .child(disp_lbl),
                    )
                    .child(
                        div()
                            .text_size(px(11.5))
                            .text_color(p.muted)
                            .child(conflict.severity.legend_label(cx)),
                    )
                    .child(div().ml_auto().child(close_btn)),
            )
            .child(
                div()
                    .text_size(px(12.5))
                    .line_height(px(18.))
                    .text_color(p.ink)
                    .child(conflict.title.clone()),
            )
            .child(involved)
            .child(
                div()
                    .w_full()
                    .pt(px(6.))
                    .border_t_1()
                    .border_color(p.line)
                    .flex()
                    .justify_end()
                    .gap(px(6.))
                    .child(pro_btn(
                        pro_eid(&self.id, "o-keep-btn"),
                        tr(cx, "Keep watching", "继续观察"),
                        None,
                        ProBtnStyle::Secondary,
                        true,
                        cx,
                        |this, _, cx| {
                            this.modal = None;
                            cx.notify();
                        },
                    ))
                    .child(pro_btn(
                        pro_eid(&self.id, "o-resolve-btn"),
                        tr(cx, "Resolve conflict", "标记已解决"),
                        Some(ProIcon::Check),
                        ProBtnStyle::Primary,
                        true,
                        cx,
                        move |this, _, cx| {
                            this.resolve_conflict(res_id.clone(), cx);
                        },
                    )),
            );

        dialog.into_any_element()
    }

    fn render_agent_dialog(
        &self,
        slot: &AgentSlot,
        valid_conflicts: &[OverlapConflict],
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let skey = slot.key.clone();
        let col = slot_color(slot.color_idx, p);
        let related: Vec<&OverlapConflict> = valid_conflicts
            .iter()
            .filter(|c| c.sides.iter().any(|s| s.key() == skey))
            .collect();

        let close_btn = pro_icon_btn(
            pro_eid(&self.id, "o-close-adlg"),
            tr(cx, "Close agent card", "关闭智能体卡片"),
            ProIcon::Close,
            24.,
            false,
            cx,
            |this, w, cx| {
                this.modal = None;
                this.focus.focus(w, cx);
                cx.notify();
            },
        );

        let mut list = div()
            .w_full()
            .pt(px(8.))
            .border_t_1()
            .border_color(p.line)
            .flex()
            .flex_col()
            .gap(px(6.));
        for (ri, rc) in related.iter().enumerate() {
            let cid = rc.id.clone();
            let (sbg, sfg) = severity_badge_colors(rc.severity, p);
            let row_hov = p.hover;
            let row_lbl = if self.custom_data {
                rc.label.clone()
            } else {
                tr_conflict_label(&rc.label, cx).into()
            };
            list = list.child(pro_tactile(
                pro_eid(&self.id, format!("o-arel-{ri}")),
                format!("{row_lbl} — {}", rc.title),
                7.0,
                p.subtle,
                row_hov,
                transparent_black(),
                p.line_strong,
                false,
                div()
                    .w_full()
                    .p(px(7.))
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .child(
                                div()
                                    .h(px(19.))
                                    .px(px(6.))
                                    .rounded(px(4.))
                                    .bg(sbg)
                                    .text_color(sfg)
                                    .text_size(px(11.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .flex()
                                    .items_center()
                                    .child(row_lbl),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(p.muted)
                                    .child(rc.severity.legend_label(cx)),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .line_height(px(16.5))
                            .text_color(p.text)
                            .child(rc.title.clone()),
                    ),
                cx,
                move |this, _, cx| {
                    this.modal = Some(OverlapModal::Conflict(cid.clone()));
                    cx.emit(AgentOverlapMapEvent::ConflictSelected {
                        conflict_id: cid.clone(),
                    });
                    cx.notify();
                },
            ));
        }

        let dialog = div()
            .id(pro_eid(&self.id, "o-adlg"))
            .role(Role::Dialog)
            .aria_label(format!("{} · {}", slot.full_name, slot.agent_name))
            .on_click(|_, _, cx| cx.stop_propagation())
            .w(px(340.))
            .max_w_full()
            .rounded(px(10.))
            .bg(p.card)
            .border_1()
            .border_color(p.line_strong)
            .shadow_lg()
            .p(px(12.))
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(pro_agent_avatar(&slot.key, col, 26., false, 0., cx))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_size(px(13.5))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.strong)
                                    .child(slot.full_name.clone()),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(4.))
                                    .text_size(px(11.5))
                                    .text_color(p.muted)
                                    .children(pro_app_artwork(&slot.harness, 11.5, cx))
                                    .child(slot.agent_name.clone()),
                            ),
                    )
                    .child(close_btn),
            )
            .when(!slot.note.is_empty(), |d| {
                d.child(
                    div()
                        .text_size(px(12.))
                        .text_color(p.text)
                        .child(slot.note.clone()),
                )
            })
            .child(list);

        dialog.into_any_element()
    }
}

impl Render for AgentOverlapMap {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let reduced = Theme::global(cx).reduced_motion;
        let elapsed_ms = self.boot.elapsed().as_secs_f32() * 1000.0;

        let valid_conflicts = self.valid_conflicts();
        if !valid_conflicts.is_empty() && !reduced {
            w.request_animation_frame();
        }

        let width_cell = self.measured_width.clone();
        let width_sensor = canvas(
            move |bounds, _, _| {
                let bw = f32::from(bounds.size.width);
                if bw > 20.0 {
                    width_cell.set(bw);
                }
            },
            |_, _, _, _| (),
        )
        .absolute()
        .left_0()
        .right_0()
        .top_0()
        .h(px(1.));

        if self.loading {
            let phase = (elapsed_ms / 1200.0).fract();
            if !reduced {
                w.request_animation_frame();
            }
            return div()
                .id(self.id.clone())
                .w_full()
                .p(px(22.))
                .rounded(px(14.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .flex()
                .flex_col()
                .gap(px(20.))
                .child(width_sensor)
                .child(
                    div()
                        .w_full()
                        .flex()
                        .justify_between()
                        .child(pro_skeleton_bar(150., 20., 6., phase, cx))
                        .child(pro_skeleton_bar(240., 16., 5., phase, cx)),
                )
                .child(pro_skeleton_bar(720., 300., 10., phase, cx).w_full())
                .into_any_element();
        }

        if self.empty || self.people.is_empty() {
            return div()
                .id(self.id.clone())
                .w_full()
                .child(width_sensor)
                .child(pro_empty_state(
                    ProIcon::Users,
                    tr(cx, "No team agents connected", "暂无已连接的团队智能体"),
                    tr(
                        cx,
                        "Add teammates and active coding agents to detect overlapping edits and contradicting assumptions.",
                        "添加团队成员与编程智能体后，即可自动检测跨会话的文件冲突与重复工作。",
                    ),
                    None,
                    cx,
                ))
                .into_any_element();
        }

        if let Some(m) = &self.modal {
            self.last_modal = Some(m.clone());
        }

        let all_valid_conflicts = self.valid_conflicts();
        let valid_conflicts: Vec<OverlapConflict> = match self.severity_filter {
            Some(sev) => all_valid_conflicts
                .iter()
                .filter(|c| c.severity == sev)
                .cloned()
                .collect(),
            None => all_valid_conflicts.clone(),
        };
        let container_w = self.measured_width.get();
        let two_sided = container_w >= 580.0;
        let slots = self.build_slots(&valid_conflicts, two_sided);
        let synced = self.synced_people();

        let title_str: SharedString = self
            .title
            .clone()
            .unwrap_or_else(|| tr(cx, "Agent overlaps", "智能体工作重叠").into());

        let filter_key: SharedString = match self.severity_filter {
            None => "all".into(),
            Some(OverlapSeverity::High) => "high".into(),
            Some(OverlapSeverity::Medium) => "med".into(),
            Some(OverlapSeverity::Low) => "low".into(),
        };
        let filter_sensor = pro_selection_sensor();
        let pill_bg: Hsla = if p.dark {
            hsla(0., 0., 1., 0.10)
        } else {
            p.card
        };
        let focus_tab = p.focus;
        let filter_tabs = {
            let mut bar = div()
                .id(pro_eid(&self.id, "o-sev-tabs"))
                .role(Role::TabList)
                .relative()
                .p(px(2.))
                .rounded(px(8.))
                .bg(p.subtle)
                .border_1()
                .border_color(p.line)
                .flex()
                .flex_wrap()
                .items_center()
                .gap(px(2.))
                .child(pro_pill_indicator(
                    pro_eid(&self.id, "o-sev-ind"),
                    filter_key,
                    filter_sensor.clone(),
                    6.0,
                    pill_bg,
                    Some(p.line),
                    !p.dark,
                ));

            let items: [(Option<OverlapSeverity>, &str, Option<Hsla>, &str); 4] = [
                (None, "all", None, tr(cx, "All", "全部")),
                (
                    Some(OverlapSeverity::High),
                    "high",
                    Some(p.red),
                    OverlapSeverity::High.legend_label(cx),
                ),
                (
                    Some(OverlapSeverity::Medium),
                    "med",
                    Some(p.amber),
                    OverlapSeverity::Medium.legend_label(cx),
                ),
                (
                    Some(OverlapSeverity::Low),
                    "low",
                    Some(p.faint),
                    OverlapSeverity::Low.legend_label(cx),
                ),
            ];
            for (opt_sev, skey, dot_c, lbl) in items {
                let active = self.severity_filter == opt_sev;
                let hov_fg = p.strong;
                bar = bar.child(activate(
                    div()
                        .id(pro_eid(&self.id, format!("o-ftab-{skey}")))
                        .role(Role::Tab)
                        .aria_selected(active)
                        .tab_index(if active { 0 } else { -1 })
                        .relative()
                        .h(px(24.))
                        .px(px(8.))
                        .rounded(px(6.))
                        .cursor_pointer()
                        .when(!active, |d| d.hover(move |s| s.text_color(hov_fg)))
                        .focus_visible(move |s| s.border_2().border_color(focus_tab))
                        .flex()
                        .items_center()
                        .text_size(px(11.5))
                        .font_weight(if active {
                            FontWeight::MEDIUM
                        } else {
                            FontWeight::NORMAL
                        })
                        .text_color(if active { p.strong } else { p.muted })
                        .when(active, |d| d.child(pro_mark_selected(&filter_sensor)))
                        .child(if let Some(dc) = dot_c {
                            legend_dot(dc, lbl).into_any_element()
                        } else {
                            lbl.to_string().into_any_element()
                        }),
                    cx,
                    move |this, _, cx| {
                        this.severity_filter = opt_sev;
                        cx.notify();
                    },
                ));
            }
            bar
        };

        let header = div()
            .w_full()
            .flex()
            .flex_wrap()
            .items_center()
            .justify_between()
            .gap(px(10.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(
                        div()
                            .text_size(px(14.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .child(title_str.clone()),
                    )
                    .child(
                        div()
                            .h(px(20.))
                            .px(px(7.))
                            .rounded(px(6.))
                            .bg(p.chip)
                            .flex()
                            .items_center()
                            .child(pro_rolling_number(
                                pro_eid(&self.id, "o-cnt"),
                                valid_conflicts.len() as f64,
                                0,
                                "",
                                "",
                                12.0,
                                FontWeight::MEDIUM,
                                p.muted,
                            )),
                    ),
            )
            .when(!all_valid_conflicts.is_empty(), |d| d.child(filter_tabs));

        if valid_conflicts.is_empty() {
            return div()
                .id(self.id.clone())
                .track_focus(&self.focus)
                .role(Role::Region)
                .aria_label(title_str)
                .w_full()
                .p(px(22.))
                .rounded(px(14.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .flex()
                .flex_col()
                .gap(px(18.))
                .child(width_sensor)
                .child(header)
                .child(
                    div()
                        .w_full()
                        .py(px(28.))
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap(px(10.))
                        .text_size(px(13.))
                        .text_color(p.muted)
                        .child(tr(
                            cx,
                            "No overlaps right now — everyone’s agents are in sync.",
                            "当前没有任何冲突——所有成员的智能体均保持同步。",
                        ))
                        .when(!self.custom_data, |d| {
                            d.child(pro_btn(
                                pro_eid(&self.id, "o-reset-all"),
                                tr(cx, "Reset sample overlaps", "恢复示例冲突"),
                                Some(ProIcon::Refresh),
                                ProBtnStyle::Secondary,
                                true,
                                cx,
                                |this, _, cx| {
                                    this.reset_defaults(cx);
                                },
                            ))
                        }),
                )
                .child(pro_toast_banner(self.toast.clone()))
                .into_any_element();
        }

        // Determine active focus highlight (hover or modal) and sample smooth springs!
        let active_foc = self.hover.clone().or_else(|| match &self.modal {
            Some(OverlapModal::Conflict(id)) => Some(OverlapHover::Conflict(id.clone())),
            Some(OverlapModal::Agent(key)) => Some(OverlapHover::Agent(key.clone())),
            None => None,
        });

        let mut slot_opacities: HashMap<SharedString, f32> = HashMap::with_capacity(slots.len());
        for sl in &slots {
            let dimmed = match &active_foc {
                Some(OverlapHover::Agent(ak)) => ak != &sl.key,
                Some(OverlapHover::Conflict(cid)) => !valid_conflicts
                    .iter()
                    .find(|c| &c.id == cid)
                    .map(|c| c.sides.iter().any(|s| s.key() == sl.key))
                    .unwrap_or(false),
                None => false,
            };
            let op = pro_spring_f32(
                pro_eid(&self.id, format!("ochip-op-{}", sl.key)),
                if dimmed { 0.28 } else { 1.0 },
                0.22,
                w,
                cx,
            );
            slot_opacities.insert(sl.key.clone(), op);
        }

        let mut badge_opacities: HashMap<SharedString, f32> =
            HashMap::with_capacity(valid_conflicts.len());
        let mut edge_springs: HashMap<SharedString, (f32, f32)> = HashMap::new();
        for cf in &valid_conflicts {
            let dimmed = match &active_foc {
                Some(OverlapHover::Conflict(id)) => id != &cf.id,
                Some(OverlapHover::Agent(ak)) => !cf.sides.iter().any(|s| &s.key() == ak),
                None => false,
            };
            let bop = pro_spring_f32(
                pro_eid(&self.id, format!("obadge-op-{}", cf.id)),
                if dimmed { 0.25 } else { 1.0 },
                0.22,
                w,
                cx,
            );
            badge_opacities.insert(cf.id.clone(), bop);

            for side in &cf.sides {
                let skey = side.key();
                let ekey: SharedString = format!("{}->{}", cf.id, skey).into();
                let is_lit = match &active_foc {
                    Some(OverlapHover::Conflict(cid)) => cid == &cf.id,
                    Some(OverlapHover::Agent(ak)) => ak == &skey,
                    None => false,
                };
                let is_dim = active_foc.is_some() && !is_lit;
                let base_rest = match cf.severity {
                    OverlapSeverity::High | OverlapSeverity::Medium => 0.44_f32,
                    OverlapSeverity::Low => 0.28_f32,
                };
                let target_alpha = if is_lit {
                    0.90_f32
                } else if is_dim {
                    0.10_f32
                } else {
                    base_rest
                };
                let target_w = if is_lit { 1.85_f32 } else { 1.25_f32 };
                let sp_a = pro_spring_f32(
                    pro_eid(&self.id, format!("oedge-a-{ekey}")),
                    target_alpha,
                    0.22,
                    w,
                    cx,
                );
                let sp_w = pro_spring_f32(
                    pro_eid(&self.id, format!("oedge-w-{ekey}")),
                    target_w,
                    0.20,
                    w,
                    cx,
                );
                edge_springs.insert(ekey, (sp_a, sp_w));
            }
        }

        let mut slot_sensors: HashMap<SharedString, Rc<Cell<Option<Bounds<Pixels>>>>> =
            HashMap::new();
        let mut badge_sensors: HashMap<SharedString, Rc<Cell<Option<Bounds<Pixels>>>>> =
            HashMap::new();

        let chip_w = if two_sided { 214.0 } else { 172.0 };
        let chip_bg: Hsla = if p.dark {
            rgb(0x1f1f23).into()
        } else {
            rgb(0xffffff).into()
        };

        let render_agent_chip =
            |sl: &AgentSlot, sensor: Rc<Cell<Option<Bounds<Pixels>>>>, cx: &mut Context<Self>| {
                let skey = sl.key.clone();
                let skey_hov = sl.key.clone();
                let pid = sl.person_id.clone();
                let aname = sl.agent_name.clone();
                let col = slot_color(sl.color_idx, &p);
                let op = slot_opacities.get(&sl.key).copied().unwrap_or(1.0);
                let is_sel = self.modal == Some(OverlapModal::Agent(sl.key.clone()));

                pro_tactile(
                    pro_eid(&self.id, format!("ochip-{}", sl.key)),
                    format!("{} · {}", sl.full_name, sl.agent_name),
                    8.0,
                    chip_bg,
                    pro_mix_color(chip_bg, col, if p.dark { 0.08 } else { 0.04 }),
                    if is_sel {
                        col.opacity(0.65)
                    } else {
                        p.line_strong.opacity(0.75)
                    },
                    p.line_strong,
                    is_sel,
                    div()
                        .w(px(chip_w))
                        .h(px(36.))
                        .px(px(11.))
                        .relative()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(
                            canvas(
                                move |bounds, _, _| {
                                    sensor.set(Some(bounds));
                                },
                                |_, _, _, _| (),
                            )
                            .absolute()
                            .inset_0(),
                        )
                        .child(render_bare_pixel_mark(&sl.key, col, !reduced, elapsed_ms))
                        .child(
                            div()
                                .text_size(px(13.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.strong)
                                .truncate()
                                .child(sl.first_name.clone()),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(5.))
                                .min_w_0()
                                .children(pro_app_artwork(&sl.harness, 13., cx))
                                .when(two_sided, |d| {
                                    d.child(
                                        div()
                                            .text_size(px(12.5))
                                            .text_color(p.muted)
                                            .truncate()
                                            .child(sl.agent_name.clone()),
                                    )
                                }),
                        ),
                    cx,
                    move |this, _, cx| {
                        this.modal = Some(OverlapModal::Agent(skey.clone()));
                        cx.emit(AgentOverlapMapEvent::AgentSelected {
                            person_id: pid.clone(),
                            agent_name: aname.clone(),
                        });
                        cx.notify();
                    },
                )
                .w(px(chip_w))
                .h(px(36.))
                .opacity(op)
                .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                    let next = if *hovered {
                        Some(OverlapHover::Agent(skey_hov.clone()))
                    } else if this.hover == Some(OverlapHover::Agent(skey_hov.clone())) {
                        None
                    } else {
                        this.hover.clone()
                    };
                    if this.hover != next {
                        this.hover = next;
                        cx.notify();
                    }
                }))
            };

        let mut left_col = div()
            .flex_none()
            .flex()
            .flex_col()
            .justify_center()
            .gap(px(12.));
        let mut right_col = div()
            .flex_none()
            .flex()
            .flex_col()
            .justify_center()
            .gap(px(12.));
        for sl in &slots {
            let sens = Rc::new(Cell::new(None));
            slot_sensors.insert(sl.key.clone(), sens.clone());
            let chip = render_agent_chip(sl, sens, cx);
            if sl.left_col {
                left_col = left_col.child(chip);
            } else {
                right_col = right_col.child(chip);
            }
        }

        let mut mid_col = div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(15.));

        for cf in &valid_conflicts {
            let cid = cf.id.clone();
            let cid_hov = cf.id.clone();
            let b_sens = Rc::new(Cell::new(None));
            badge_sensors.insert(cid.clone(), b_sens.clone());

            let bop = badge_opacities.get(&cf.id).copied().unwrap_or(1.0);
            let is_cf_sel = self.modal == Some(OverlapModal::Conflict(cf.id.clone()));
            let (bg_c, fg_c) = severity_badge_colors(cf.severity, &p);
            let bdr_c = match cf.severity {
                OverlapSeverity::High => p.red.opacity(0.28),
                OverlapSeverity::Medium => p.amber.opacity(0.30),
                OverlapSeverity::Low => p.line,
            };
            let disp_lbl: SharedString = if self.custom_data {
                cf.label.clone()
            } else {
                tr_conflict_label(&cf.label, cx).into()
            };

            mid_col = mid_col.child(
                pro_tactile(
                    pro_eid(&self.id, format!("obadge-{cid}")),
                    format!("{disp_lbl} — {}", cf.title),
                    6.0,
                    bg_c,
                    pro_mix_color(bg_c, fg_c, 0.08),
                    bdr_c,
                    fg_c.opacity(0.55),
                    is_cf_sel,
                    div()
                        .h(px(24.))
                        .px(px(10.))
                        .relative()
                        .text_size(px(12.5))
                        .text_color(fg_c)
                        .flex()
                        .items_center()
                        .child(
                            canvas(
                                move |bounds, _, _| {
                                    b_sens.set(Some(bounds));
                                },
                                |_, _, _, _| (),
                            )
                            .absolute()
                            .inset_0(),
                        )
                        .child(disp_lbl),
                    cx,
                    move |this, _, cx| {
                        this.modal = Some(OverlapModal::Conflict(cid.clone()));
                        cx.emit(AgentOverlapMapEvent::ConflictSelected {
                            conflict_id: cid.clone(),
                        });
                        cx.notify();
                    },
                )
                .h(px(24.))
                .opacity(bop)
                .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                    let next = if *hovered {
                        Some(OverlapHover::Conflict(cid_hov.clone()))
                    } else if this.hover == Some(OverlapHover::Conflict(cid_hov.clone())) {
                        None
                    } else {
                        this.hover.clone()
                    };
                    if this.hover != next {
                        this.hover = next;
                        cx.notify();
                    }
                })),
            );
        }

        // Smooth Bezier curve canvas connecting each conflict's sides to its badge
        let conflicts_paint = valid_conflicts.clone();
        let p_canvas = p;

        let curves_canvas = canvas(
            |_, _, _| (),
            move |_, _, window, _| {
                for cf in &conflicts_paint {
                    let Some(b_rect) = badge_sensors.get(&cf.id).and_then(|c| c.get()) else {
                        continue;
                    };
                    let by = (f32::from(b_rect.top()) + f32::from(b_rect.bottom())) * 0.5;
                    let bl = f32::from(b_rect.left());
                    let br = f32::from(b_rect.right());

                    for side in &cf.sides {
                        let skey = side.key();
                        let Some(s_rect) = slot_sensors.get(&skey).and_then(|c| c.get()) else {
                            continue;
                        };
                        let sy = (f32::from(s_rect.top()) + f32::from(s_rect.bottom())) * 0.5;
                        let s_cx = (f32::from(s_rect.left()) + f32::from(s_rect.right())) * 0.5;
                        let b_cx = (bl + br) * 0.5;

                        let (x0, x1) = if s_cx <= b_cx {
                            (f32::from(s_rect.right()), bl)
                        } else {
                            (f32::from(s_rect.left()), br)
                        };

                        let ekey: SharedString = format!("{}->{}", cf.id, skey).into();
                        let (alpha, stroke_w) =
                            edge_springs.get(&ekey).copied().unwrap_or((0.44, 1.25));

                        let base_col = match cf.severity {
                            OverlapSeverity::High => p_canvas.red,
                            OverlapSeverity::Medium => p_canvas.amber,
                            OverlapSeverity::Low => p_canvas.muted,
                        };

                        if cf.severity == OverlapSeverity::Low {
                            paint_dashed_cubic_bezier(
                                (x0, sy),
                                (x1, by),
                                stroke_w,
                                base_col.opacity(alpha),
                                window,
                            );
                        } else {
                            let mid_x = (x0 + x1) * 0.5;
                            let mut pb = PathBuilder::stroke(px(stroke_w));
                            pb.move_to(point(px(x0), px(sy)));
                            pb.cubic_bezier_to(
                                point(px(x1), px(by)),
                                point(px(mid_x), px(sy)),
                                point(px(mid_x), px(by)),
                            );
                            if let Ok(path) = pb.build() {
                                window.paint_path(path, base_col.opacity(alpha));
                            }
                        }
                    }
                }
            },
        )
        .absolute()
        .inset_0();

        let sync_footer = if !synced.is_empty() {
            let names = synced.join(", ");
            let phrase = if synced.len() == 1 {
                format!("{names} {}", tr(cx, "is in sync", "处于同步状态"))
            } else {
                format!("{names} {}", tr(cx, "are in sync", "处于同步状态"))
            };
            Some(
                div()
                    .w_full()
                    .pt(px(12.))
                    .border_t_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .text_size(px(12.))
                    .text_color(p.muted)
                    .child(render_bare_pixel_mark("jonas-sync", p.muted, false, 0.0))
                    .child(phrase),
            )
        } else {
            None
        };

        let modal_open = self.modal.is_some();
        let shown_modal = self.modal.clone().or_else(|| self.last_modal.clone());
        let all_slots = self.build_slots(&all_valid_conflicts, two_sided);
        let open_modal = shown_modal.and_then(|m| {
            let (mkey, card): (SharedString, AnyElement) = match m {
                OverlapModal::Conflict(cid) => {
                    let cf = all_valid_conflicts.iter().find(|c| c.id == cid)?;
                    (
                        format!("cf:{cid}").into(),
                        self.render_conflict_dialog(cf, &all_slots, &p, cx),
                    )
                }
                OverlapModal::Agent(skey) => {
                    let sl = all_slots.iter().find(|x| x.key == skey)?;
                    (
                        format!("ag:{skey}").into(),
                        self.render_agent_dialog(sl, &all_valid_conflicts, &p, cx),
                    )
                }
            };
            let scrim_alpha = if p.dark { 0.46 } else { 0.20 };
            let on_scrim_click = cx.listener(|this, _, w, cx| {
                this.modal = None;
                this.focus.focus(w, cx);
                cx.notify();
            });
            let card_vtx = pro_view_transition(pro_eid(&self.id, "o-modal-vtx"), mkey, card);
            let card_pop =
                pro_popover_presence(pro_eid(&self.id, "o-modal-pop"), modal_open, move || {
                    card_vtx
                });
            Some(pro_presence(
                pro_eid(&self.id, "o-modal-pres"),
                modal_open,
                0.18,
                0.14,
                move |amt| {
                    let mut scrim = div()
                        .id(pro_eid(&ElementId::Name("o-scrim".into()), "ov"))
                        .absolute()
                        .inset_0()
                        .bg(hsla(0., 0., 0., scrim_alpha * amt))
                        .flex()
                        .items_center()
                        .justify_center()
                        .p(px(16.));
                    if modal_open {
                        scrim = scrim.on_click(on_scrim_click);
                    }
                    scrim.child(card_pop).into_any_element()
                },
            ))
        });

        div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .role(Role::Region)
            .aria_label(title_str)
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, w, cx| {
                if ev.keystroke.key == "escape" && this.modal.is_some() {
                    this.modal = None;
                    this.focus.focus(w, cx);
                    cx.notify();
                }
            }))
            .w_full()
            .p(px(22.))
            .rounded(px(14.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .relative()
            .overflow_hidden()
            .flex()
            .flex_col()
            .gap(px(20.))
            .child(width_sensor)
            .child(header)
            .child(
                div()
                    .w_full()
                    .py(px(6.))
                    .relative()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(if two_sided { 20. } else { 14. }))
                    .child(curves_canvas)
                    .child(left_col)
                    .child(mid_col)
                    .when(two_sided, |d| d.child(right_col)),
            )
            .children(sync_footer)
            .children(open_modal)
            .child(pro_toast_banner(self.toast.clone()))
            .into_any_element()
    }
}

fn render_bare_pixel_mark(seed: &str, color: Hsla, twinkle: bool, elapsed_ms: f32) -> Div {
    let cells = agent_pixel_cells(seed);
    let mut grid = div().flex_none().flex().flex_col().gap(px(1.5));
    for r in 0..3usize {
        let mut row = div().flex().items_center().gap(px(1.5));
        for c in 0..3usize {
            let i = r * 3 + c;
            let on = cells[i];
            let alpha = if twinkle {
                let phase = ((elapsed_ms + AGENT_WAVE_MS[i]) / 1350.0) * std::f32::consts::TAU;
                let wave = 0.62 + 0.38 * phase.cos();
                if on { wave } else { 0.14 * wave }
            } else if on {
                0.85
            } else {
                0.0
            };
            row = row.child(div().size(px(3.)).rounded(px(0.8)).bg(color.opacity(alpha)));
        }
        grid = grid.child(row);
    }
    grid
}

fn paint_dashed_cubic_bezier(
    p0: (f32, f32),
    p3: (f32, f32),
    stroke_w: f32,
    color: Hsla,
    window: &mut Window,
) {
    let mid_x = (p0.0 + p3.0) * 0.5;
    let p1 = (mid_x, p0.1);
    let p2 = (mid_x, p3.1);
    let eval = |t: f32| -> Point<Pixels> {
        let inv = 1.0 - t;
        let x = inv * inv * inv * p0.0
            + 3.0 * inv * inv * t * p1.0
            + 3.0 * inv * t * t * p2.0
            + t * t * t * p3.0;
        let y = inv * inv * inv * p0.1
            + 3.0 * inv * inv * t * p1.1
            + 3.0 * inv * t * t * p2.1
            + t * t * t * p3.1;
        point(px(x), px(y))
    };
    let dashes = 18usize;
    for d in 0..dashes {
        let t0 = d as f32 / dashes as f32;
        let t1 = (d as f32 + 0.52) / dashes as f32;
        let mut seg = PathBuilder::stroke(px(stroke_w));
        seg.move_to(eval(t0));
        seg.line_to(eval(t1));
        if let Ok(path) = seg.build() {
            window.paint_path(path, color);
        }
    }
}

fn legend_dot(color: Hsla, label: &str) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(5.))
        .child(div().size(px(6.)).rounded_full().bg(color))
        .child(label.to_string())
}

fn severity_badge_colors(severity: OverlapSeverity, p: &ProPalette) -> (Hsla, Hsla) {
    match severity {
        OverlapSeverity::High => (p.red_soft, p.red_ink),
        OverlapSeverity::Medium => (p.amber_soft, p.amber_ink),
        OverlapSeverity::Low => (p.chip, p.text),
    }
}

fn slot_color(idx: usize, p: &ProPalette) -> Hsla {
    let cols: [Hsla; 9] = [
        p.purple,
        p.blue,
        p.pink,
        p.purple,
        p.green,
        p.orange,
        p.cyan,
        p.blue,
        if p.dark {
            rgb(0x6e9eff).into()
        } else {
            rgb(0x3b6cf6).into()
        },
    ];
    cols[idx % cols.len()]
}

fn first_word_name(full: &str) -> String {
    for token in full.split_whitespace() {
        if token.chars().any(char::is_alphabetic) {
            return token.to_string();
        }
    }
    full.trim().to_string()
}

fn tr_conflict_label<'a>(en: &'a str, cx: &App) -> &'a str {
    match en {
        "Same files" => tr(cx, "Same files", "同一文件"),
        "Out of sync" => tr(cx, "Out of sync", "版本失步"),
        "Contradicts" => tr(cx, "Contradicts", "逻辑冲突"),
        "Same work" => tr(cx, "Same work", "重复开发"),
        _ => en,
    }
}

fn default_overlap_dataset() -> (Vec<OverlapPerson>, Vec<OverlapConflict>) {
    let people = vec![
        OverlapPerson::new("lena", "Lena Vogel", "Billing emails & dunning templates"),
        OverlapPerson::new("priya", "Priya Nair", "Pricing page and plan seat limits"),
        OverlapPerson::new("ava", "Ava Chen", "Design tokens and Tailwind theme"),
        OverlapPerson::new("elif", "Elif Demir", "Invoice tax & rounding calculations"),
        OverlapPerson::new("sam", "Sam Ortiz", "Marketing tier comparison & tokens"),
        OverlapPerson::new("marco", "Marco Rossi", "Stripe checkout & invoice totals"),
        OverlapPerson::new("tom", "Tom Lindqvist", "Responsive layout configuration"),
        OverlapPerson::new("noah", "Noah Brooks", "Developer billing documentation"),
        OverlapPerson::new("jonas", "Jonas Weber", "Workspace onboarding flow"),
    ];

    let conflicts = vec![
        OverlapConflict::new(
            "c1",
            "Same files",
            "Three agents are editing billing/invoice.ts at the same time.",
            OverlapSeverity::High,
            vec![
                OverlapSide::new("lena", "Codex", "openai"),
                OverlapSide::new("marco", "Gemini CLI", "gemini"),
                OverlapSide::new("sam", "Codex", "openai"),
            ],
        ),
        OverlapConflict::new(
            "c2",
            "Out of sync",
            "Marco’s agent still uses the old price IDs Priya’s agent replaced this morning.",
            OverlapSeverity::Medium,
            vec![
                OverlapSide::new("priya", "Claude Code", "claude"),
                OverlapSide::new("marco", "Gemini CLI", "gemini"),
            ],
        ),
        OverlapConflict::new(
            "c3",
            "Contradicts",
            "Priya’s agent raises the Pro limit to 50 seats while Lena’s keeps 25 in the emails.",
            OverlapSeverity::High,
            vec![
                OverlapSide::new("lena", "Codex", "openai"),
                OverlapSide::new("priya", "Claude Code", "claude"),
            ],
        ),
        OverlapConflict::new(
            "c4",
            "Contradicts",
            "Elif’s agent rounds invoice totals per line while Marco’s rounds once per invoice.",
            OverlapSeverity::High,
            vec![
                OverlapSide::new("elif", "Gemini CLI", "gemini"),
                OverlapSide::new("marco", "Gemini CLI", "gemini"),
            ],
        ),
        OverlapConflict::new(
            "c5",
            "Same files",
            "Sam’s and Ava’s agents both touched tokens/colors.css.",
            OverlapSeverity::Low,
            vec![
                OverlapSide::new("ava", "Claude Code", "claude"),
                OverlapSide::new("sam", "Codex", "openai"),
            ],
        ),
        OverlapConflict::new(
            "c6",
            "Same files",
            "Tom’s and Ava’s agents both changed tailwind.config.ts.",
            OverlapSeverity::Low,
            vec![
                OverlapSide::new("ava", "Claude Code", "claude"),
                OverlapSide::new("tom", "Claude Code", "claude"),
            ],
        ),
        OverlapConflict::new(
            "c7",
            "Same work",
            "Sam’s and Priya’s agents are both building the plan comparison table.",
            OverlapSeverity::Medium,
            vec![
                OverlapSide::new("priya", "Claude Code", "claude"),
                OverlapSide::new("sam", "Claude Code", "claude"),
            ],
        ),
        OverlapConflict::new(
            "c8",
            "Same work",
            "Noah’s and Elif’s agents are both writing the usage-based billing guide.",
            OverlapSeverity::Medium,
            vec![
                OverlapSide::new("elif", "Gemini CLI", "gemini"),
                OverlapSide::new("noah", "Codex", "openai"),
            ],
        ),
    ];

    (people, conflicts)
}

#[cfg(test)]
mod tests {
    use gpui::AppContext;

    use super::{AgentOverlapMap, first_word_name};
    use gpui::TestAppContext;

    #[test]
    fn default_overlap_map_lists_eight_conflicts_and_jonas_in_sync() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let map = app.new(|cx| AgentOverlapMap::new("overlap-test", cx));
        map.read_with(&app, |m, _| {
            assert_eq!(m.conflict_count(), 8);
            assert_eq!(m.synced_people(), vec![gpui::SharedString::from("Jonas")]);
        });
    }

    #[test]
    fn resolving_conflicts_moves_freed_teammates_into_synced_list() {
        assert_eq!(first_word_name("🦊 Kit Fox"), "Kit");
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let map = app.new(|cx| AgentOverlapMap::new("overlap-res", cx));
        map.update(&mut app, |m, cx| {
            // Noah only participates in c8; resolving c8 frees Noah into sync!
            m.resolve_conflict("c8", cx);
            assert_eq!(m.conflict_count(), 7);
            let synced = m.synced_people();
            assert!(synced.iter().any(|p| p.as_ref() == "Jonas"));
            assert!(synced.iter().any(|p| p.as_ref() == "Noah"));
        });
    }
}
