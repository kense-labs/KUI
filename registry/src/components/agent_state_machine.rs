//! Finite-state machine diagram of an autonomous agent with pulses and rolling counts.
//!
//! Renders every state (`Planning`, `Searching`, `Coding`, `Testing`, `Waiting`, `Done`) as a live
//! card linked by forward arrows, top self-retry loops, and bottom loopback paths with rolling counts.

use crate::{
    components::pro_chrome::{
        ProBtnStyle, ProPalette, pro_agent_avatar, pro_app_tile, pro_btn, pro_check_draw, pro_ease,
        pro_eid, pro_empty_state, pro_icon_btn, pro_mix_color, pro_presence, pro_rolling_number,
        pro_skeleton_bar, pro_spring_f32, pro_swap_label, pro_tactile, pro_toast_banner,
        pro_view_transition, tr,
    },
    foundation::Theme,
    primitives::pro_icons::{ProIcon, pro_app_artwork, pro_icon},
};
use gpui::{prelude::*, *};
use std::{cell::Cell, collections::HashMap, rc::Rc};
use web_time::Instant;

/// Semantic kind of a [`MachineState`] inside [`AgentStateMachine`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MachineStateKind {
    /// Standard working state; glows blue when active.
    #[default]
    Normal,
    /// Human-in-the-loop pause gate; turns amber and offers `Approve` / `Reject`.
    Waiting,
    /// Error recovery state; turns red.
    Error,
    /// Terminal completion state; renders a bullseye mark and turns green when reached.
    Final,
}

impl MachineStateKind {
    fn badge_label(self, is_current: bool, cx: &App) -> &'static str {
        match (self, is_current) {
            (Self::Waiting, _) => tr(cx, "Needs approval", "等待批准"),
            (Self::Error, _) => tr(cx, "Error", "异常状态"),
            (Self::Final, true) => tr(cx, "Completed", "已完成"),
            (Self::Final, false) => tr(cx, "Final state", "终止状态"),
            (Self::Normal, true) => tr(cx, "Current state", "当前状态"),
            (Self::Normal, false) => tr(cx, "Visited", "已访问"),
        }
    }
}

/// One state vertex in [`AgentStateMachine`].
#[derive(Clone, Debug, PartialEq)]
pub struct MachineState {
    /// Stable state identifier referenced by [`MachineTransition`] and `current`.
    pub id: SharedString,
    /// Two- or three-word state title (for example `"Planning"`, `"Searching"`).
    pub label: SharedString,
    /// Semantic state role (`Normal`, `Waiting`, `Error`, `Final`).
    pub kind: MachineStateKind,
    /// Detailed explanation shown inside the dwell-time & exit-reason inspector.
    pub description: SharedString,
    /// Optional custom icon dot color.
    pub color: Option<Hsla>,
    /// Connected tool names rendered as mini tiles on the card and chips in the inspector.
    pub tools: Vec<SharedString>,
    /// Accumulated dwell time spent in this state in milliseconds.
    pub dwell_ms: u64,
    /// Number of times the agent entered this state.
    pub visits: u32,
    /// Grid column index (`0..N`).
    pub col: usize,
    /// Grid row index (`0` = main spine, `1` = lower branch).
    pub row: usize,
}

impl MachineState {
    /// Construct a [`MachineState`] at `(col, row)` on the diagram grid.
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        col: usize,
        row: usize,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            kind: MachineStateKind::Normal,
            description: SharedString::default(),
            color: None,
            tools: Vec::new(),
            dwell_ms: 0,
            visits: 0,
            col,
            row,
        }
    }

    /// Set [`MachineStateKind`] (`Waiting`, `Error`, or `Final`).
    pub fn with_kind(mut self, kind: MachineStateKind) -> Self {
        self.kind = kind;
        self
    }

    /// Set the description shown inside the state inspector dialog.
    pub fn with_description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = description.into();
        self
    }

    /// Override the state dot color.
    pub fn with_color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }

    /// Attach tool names used while in this state.
    pub fn with_tools(mut self, tools: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.tools = tools.into_iter().map(Into::into).collect();
        self
    }

    /// Set accumulated `dwell_ms` and `visits` count.
    pub fn with_metrics(mut self, dwell_ms: u64, visits: u32) -> Self {
        self.dwell_ms = dwell_ms;
        self.visits = visits;
        self
    }
}

/// Directed transition edge between two states in [`AgentStateMachine`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MachineTransition {
    /// Stable transition identifier.
    pub id: SharedString,
    /// Source state id.
    pub from: SharedString,
    /// Destination state id (`from == to` renders a self-retry loop on top).
    pub to: SharedString,
    /// Short exit trigger phrase (for example `"needs context"`, `"timeout, retry"`).
    pub label: SharedString,
    /// Number of times this transition has fired during the run.
    pub count: u32,
    /// Extended explanation shown in the transition inspector.
    pub detail: SharedString,
}

impl MachineTransition {
    /// Construct a [`MachineTransition`] from `from` to `to` with trigger `label` and `count`.
    pub fn new(
        id: impl Into<SharedString>,
        from: impl Into<SharedString>,
        to: impl Into<SharedString>,
        label: impl Into<SharedString>,
        count: u32,
    ) -> Self {
        Self {
            id: id.into(),
            from: from.into(),
            to: to.into(),
            label: label.into(),
            count,
            detail: SharedString::default(),
        }
    }

    /// Set extended detail text for the transition inspector.
    pub fn with_detail(mut self, detail: impl Into<SharedString>) -> Self {
        self.detail = detail.into();
        self
    }
}

/// Typed events emitted by [`AgentStateMachine`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentStateMachineEvent {
    /// User clicked a state card to inspect its dwell time and exit reasons.
    StateSelected {
        /// Selected state identifier.
        state_id: SharedString,
    },
    /// User clicked a transition count pill to inspect its trigger.
    TransitionSelected {
        /// Selected transition identifier.
        transition_id: SharedString,
    },
    /// Agent traversed a transition into a new `current` state.
    Transitioned {
        /// Transition identifier traversed.
        transition_id: SharedString,
        /// Source state identifier.
        from: SharedString,
        /// Destination state identifier.
        to: SharedString,
    },
    /// Operator approved or rejected a `Waiting` state gate.
    Approved {
        /// Waiting state identifier.
        state_id: SharedString,
        /// `true` when approved, `false` when rejected.
        approved: bool,
    },
}

impl EventEmitter<AgentStateMachineEvent> for AgentStateMachine {}

#[derive(Clone, Debug, PartialEq, Eq)]
enum MachineInspect {
    State(SharedString),
    Transition(SharedString),
}

#[derive(Clone, Debug)]
struct ActiveTxPulse {
    tx_id: SharedString,
    started: Instant,
    duration: f32,
}

/// Finite-state diagram of an agent with forward, loopback, and self-retry transitions.
///
/// # Examples
///
/// ```ignore
/// use kui::components::agent_state_machine::AgentStateMachine;
///
/// let sm = cx.new(|cx| AgentStateMachine::new("atlas-sm", cx));
/// ```
pub struct AgentStateMachine {
    id: ElementId,
    focus: FocusHandle,
    agent_name: SharedString,
    agent_model: SharedString,
    states: Vec<MachineState>,
    transitions: Vec<MachineTransition>,
    current: SharedString,
    custom_data: bool,
    entered_current_at: Instant,
    hovered_state_id: Option<SharedString>,
    hovered_tx_id: Option<SharedString>,
    inspect: Option<MachineInspect>,
    last_inspect: Option<MachineInspect>,
    active_pulse: Option<ActiveTxPulse>,
    demo_step: usize,
    toast: Option<(SharedString, Instant)>,
    loading: bool,
    empty: bool,
    boot: Instant,
    measured_width: Rc<Cell<f32>>,
}

impl AgentStateMachine {
    /// Create a live [`AgentStateMachine`] for `"Atlas"` (`Claude Opus 5.5`) across 6 canonical states.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let now = Instant::now();
        let (states, transitions) = default_state_machine();
        Self {
            id: id.into(),
            focus: cx.focus_handle(),
            agent_name: "Atlas".into(),
            agent_model: "Claude Opus 5.5".into(),
            states,
            transitions,
            current: "coding".into(),
            custom_data: false,
            entered_current_at: now,
            hovered_state_id: None,
            hovered_tx_id: None,
            inspect: None,
            last_inspect: None,
            active_pulse: Some(ActiveTxPulse {
                tx_id: "t-search-code".into(),
                started: now,
                duration: 0.85,
            }),
            demo_step: 0,
            toast: None,
            loading: false,
            empty: false,
            boot: now,
            measured_width: Rc::new(Cell::new(780.0)),
        }
    }

    /// Override the header agent name and model label.
    pub fn with_agent(
        mut self,
        name: impl Into<SharedString>,
        model: impl Into<SharedString>,
    ) -> Self {
        self.agent_name = name.into();
        self.agent_model = model.into();
        self
    }

    /// Supply custom [`MachineState`] nodes.
    pub fn with_states(mut self, states: Vec<MachineState>) -> Self {
        self.custom_data = true;
        if states.is_empty() {
            self.states.clear();
            self.empty = true;
        } else {
            if !states.iter().any(|s| s.id == self.current) {
                self.current = states[0].id.clone();
            }
            self.states = states;
        }
        self.active_pulse = None;
        self
    }

    /// Supply custom [`MachineTransition`] edges.
    pub fn with_transitions(mut self, transitions: Vec<MachineTransition>) -> Self {
        self.custom_data = true;
        self.transitions = transitions;
        self.active_pulse = None;
        self
    }

    /// Set the active `current` state id.
    pub fn with_current(mut self, state_id: impl Into<SharedString>) -> Self {
        let sid = state_id.into();
        if self.states.iter().any(|s| s.id == sid) {
            self.current = sid;
        }
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

    /// Return the identifier of the active state.
    pub fn current_state_id(&self) -> &SharedString {
        &self.current
    }

    /// Return total transition count summed across all edges.
    pub fn total_transitions(&self) -> u32 {
        self.transitions.iter().map(|t| t.count).sum()
    }

    /// Traverse a specific transition by `transition_id`, rolling up its count and animating a pulse.
    pub fn fire_transition(
        &mut self,
        transition_id: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) {
        let tid = transition_id.into();
        let now = Instant::now();
        let added_ms = now.duration_since(self.entered_current_at).as_millis() as u64;

        let Some(tx_idx) = self.transitions.iter().position(|t| t.id == tid) else {
            return;
        };
        let from_id = self.transitions[tx_idx].from.clone();
        let to_id = self.transitions[tx_idx].to.clone();
        self.transitions[tx_idx].count = self.transitions[tx_idx].count.saturating_add(1);

        if let Some(prev) = self.states.iter_mut().find(|s| s.id == from_id) {
            prev.dwell_ms = prev.dwell_ms.saturating_add(added_ms.min(30_000));
        }
        if let Some(next) = self.states.iter_mut().find(|s| s.id == to_id) {
            next.visits = next.visits.saturating_add(1);
            next.dwell_ms = next.dwell_ms.saturating_add(1_200);
        }

        self.current = to_id.clone();
        self.entered_current_at = now;
        if !Theme::global(cx).reduced_motion {
            self.active_pulse = Some(ActiveTxPulse {
                tx_id: tid.clone(),
                started: now,
                duration: 0.78,
            });
        }
        cx.emit(AgentStateMachineEvent::Transitioned {
            transition_id: tid,
            from: from_id,
            to: to_id,
        });
        cx.notify();
    }

    /// Step the state machine along its next demonstration transition (including self-retry, backward loop, Waiting, and Done).
    pub fn step_demo_transition(&mut self, cx: &mut Context<Self>) {
        let script = [
            "t-code-wait",
            "t-wait-test",
            "t-test-code",
            "t-code-test",
            "t-test-done",
            "t-plan-search",
            "t-search-retry",
            "t-search-code",
        ];
        if self.current.as_ref() == "done" && !self.custom_data {
            let (states, transitions) = default_state_machine();
            self.states = states;
            self.transitions = transitions;
            self.current = "planning".into();
            self.entered_current_at = Instant::now();
            self.demo_step = 5;
            cx.notify();
            return;
        }
        // Prefer an outgoing transition from `self.current` matching the script, else first outgoing
        let candidate = script
            .iter()
            .skip(self.demo_step % script.len())
            .chain(script.iter())
            .find_map(|&tid| {
                self.transitions
                    .iter()
                    .find(|t| t.id.as_ref() == tid && t.from == self.current)
                    .map(|t| t.id.clone())
            })
            .or_else(|| {
                self.transitions
                    .iter()
                    .find(|t| t.from == self.current)
                    .map(|t| t.id.clone())
            });
        self.demo_step = self.demo_step.wrapping_add(1);
        if let Some(tid) = candidate {
            self.fire_transition(tid, cx);
        }
    }

    /// Approve or reject a `Waiting` state gate and transition onward.
    pub fn resolve_waiting_state(&mut self, approved: bool, cx: &mut Context<Self>) {
        let waiting_id = self.current.clone();
        self.inspect = None;
        if approved {
            if let Some(tx) = self
                .transitions
                .iter()
                .find(|t| t.from == waiting_id)
                .map(|t| t.id.clone())
            {
                self.fire_transition(tx, cx);
            }
            self.toast = Some((
                tr(
                    cx,
                    "Approved — resuming state machine",
                    "已批准，状态机继续运行",
                )
                .into(),
                Instant::now(),
            ));
        } else {
            if let Some(plan) = self.states.first().map(|s| s.id.clone()) {
                self.current = plan;
                self.entered_current_at = Instant::now();
            }
            self.toast = Some((
                tr(
                    cx,
                    "Rejected — returned to Planning",
                    "已拒绝，已回退至规划状态",
                )
                .into(),
                Instant::now(),
            ));
        }
        cx.emit(AgentStateMachineEvent::Approved {
            state_id: waiting_id,
            approved,
        });
        cx.notify();
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

    /// Return whether the state machine is empty.
    pub fn is_empty(&self) -> bool {
        self.empty || self.states.is_empty()
    }

    fn render_state_dialog(
        &self,
        st: &MachineState,
        live_extra_ms: u64,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let sid = st.id.clone();
        let is_cur = self.current == sid;
        let disp_lbl = if self.custom_data {
            st.label.clone()
        } else {
            tr_sm_state(&st.id, &st.label, cx).into()
        };
        let total_ms = if is_cur && st.kind != MachineStateKind::Final {
            st.dwell_ms + live_extra_ms
        } else {
            st.dwell_ms
        };

        let close_btn = pro_icon_btn(
            pro_eid(&self.id, "sm-close-sdlg"),
            tr(cx, "Close state inspector", "关闭状态检查器"),
            ProIcon::Close,
            24.,
            false,
            cx,
            |this, w, cx| {
                this.inspect = None;
                this.focus.focus(w, cx);
                cx.notify();
            },
        );

        let outgoing: Vec<&MachineTransition> =
            self.transitions.iter().filter(|t| t.from == sid).collect();

        let mut exits_box = div()
            .w_full()
            .pt(px(8.))
            .border_t_1()
            .border_color(p.line)
            .flex()
            .flex_col()
            .gap(px(5.));

        exits_box = exits_box.child(
            div()
                .text_size(px(11.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.muted)
                .child(tr(cx, "Exit transitions & reasons", "退出条件与跳转次数")),
        );

        for (ei, ex) in outgoing.iter().enumerate() {
            let tid = ex.id.clone();
            let to_label = self
                .states
                .iter()
                .find(|s| s.id == ex.to)
                .map(|s| {
                    if self.custom_data {
                        s.label.clone()
                    } else {
                        tr_sm_state(&s.id, &s.label, cx).into()
                    }
                })
                .unwrap_or_else(|| ex.to.clone());
            exits_box = exits_box.child(
                pro_tactile(
                    pro_eid(&self.id, format!("sm-exit-{ei}")),
                    format!("{} -> {}", ex.label, to_label),
                    6.,
                    p.subtle,
                    p.hover,
                    transparent_black(),
                    p.line_strong,
                    false,
                    div()
                        .w_full()
                        .h(px(28.))
                        .px(px(8.))
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .text_size(px(12.))
                        .child(
                            div()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.ink)
                                .child(ex.label.clone()),
                        )
                        .child(pro_icon(ProIcon::ArrowRight, 11., p.muted))
                        .child(div().text_color(p.text).child(to_label))
                        .child(div().ml_auto().child(pro_rolling_number(
                            pro_eid(&self.id, format!("sm-ex-cnt-{tid}")),
                            f64::from(ex.count),
                            0,
                            "",
                            "×",
                            12.,
                            FontWeight::MEDIUM,
                            p.muted,
                        ))),
                    cx,
                    move |this, _, cx| {
                        this.inspect = None;
                        this.fire_transition(tid.clone(), cx);
                    },
                )
                .w_full(),
            );
        }

        let mut tools_row = div().flex().flex_wrap().gap(px(5.));
        for tool in &st.tools {
            tools_row = tools_row.child(
                div()
                    .h(px(22.))
                    .pl(px(4.))
                    .pr(px(7.))
                    .rounded(px(6.))
                    .bg(p.chip)
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .text_size(px(11.5))
                    .text_color(p.text)
                    .child(pro_app_tile(tool, 14., cx))
                    .child(tool.clone()),
            );
        }

        let mut dialog = div()
            .id(pro_eid(&self.id, "sm-sdlg"))
            .role(Role::Dialog)
            .aria_label(disp_lbl.clone())
            .on_click(|_, _, cx| cx.stop_propagation())
            .w(px(344.))
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
                    .gap(px(8.))
                    .child(render_state_mark(&self.id, st, is_cur, p))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_size(px(14.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .truncate()
                            .child(disp_lbl),
                    )
                    .child(
                        div()
                            .h(px(19.))
                            .px(px(6.))
                            .rounded(px(4.))
                            .bg(if st.kind == MachineStateKind::Waiting {
                                p.amber_soft
                            } else if is_cur {
                                p.accent_soft
                            } else {
                                p.chip
                            })
                            .text_color(if st.kind == MachineStateKind::Waiting {
                                p.amber_ink
                            } else if is_cur {
                                p.blue
                            } else {
                                p.muted
                            })
                            .text_size(px(11.))
                            .font_weight(FontWeight::MEDIUM)
                            .flex()
                            .items_center()
                            .child(st.kind.badge_label(is_cur, cx)),
                    )
                    .child(close_btn),
            )
            .when(!st.description.is_empty(), |d| {
                d.child(
                    div()
                        .text_size(px(12.))
                        .line_height(px(17.))
                        .text_color(p.text)
                        .child(st.description.clone()),
                )
            })
            .child(
                div()
                    .w_full()
                    .py(px(8.))
                    .border_t_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .child(metric_pair(
                        tr(cx, "Dwell time", "驻留耗时"),
                        format_dwell_ms(total_ms),
                        p,
                    ))
                    .child(metric_pair(
                        tr(cx, "Entries", "进入次数"),
                        format!("{}×", format_count(st.visits)),
                        p,
                    )),
            )
            .when(!st.tools.is_empty(), |d| d.child(tools_row))
            .when(!outgoing.is_empty(), |d| d.child(exits_box));

        if st.kind == MachineStateKind::Waiting {
            dialog = dialog.child(
                div()
                    .w_full()
                    .pt(px(8.))
                    .border_t_1()
                    .border_color(p.line)
                    .flex()
                    .justify_end()
                    .gap(px(6.))
                    .child(pro_btn(
                        pro_eid(&self.id, "sm-w-rej"),
                        tr(cx, "Reject", "拒绝"),
                        None,
                        ProBtnStyle::Secondary,
                        true,
                        cx,
                        |this, _, cx| {
                            this.resolve_waiting_state(false, cx);
                        },
                    ))
                    .child(pro_btn(
                        pro_eid(&self.id, "sm-w-app"),
                        tr(cx, "Approve", "批准继续"),
                        Some(ProIcon::Check),
                        ProBtnStyle::Primary,
                        true,
                        cx,
                        |this, _, cx| {
                            this.resolve_waiting_state(true, cx);
                        },
                    )),
            );
        }

        dialog.into_any_element()
    }

    fn render_transition_dialog(
        &self,
        tx: &MachineTransition,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let tid_fire = tx.id.clone();
        let from_lbl = self
            .states
            .iter()
            .find(|s| s.id == tx.from)
            .map(|s| s.label.clone())
            .unwrap_or_else(|| tx.from.clone());
        let to_lbl = self
            .states
            .iter()
            .find(|s| s.id == tx.to)
            .map(|s| s.label.clone())
            .unwrap_or_else(|| tx.to.clone());

        let close_btn = pro_icon_btn(
            pro_eid(&self.id, "sm-close-tdlg"),
            tr(cx, "Close transition card", "关闭迁移详情"),
            ProIcon::Close,
            24.,
            false,
            cx,
            |this, w, cx| {
                this.inspect = None;
                this.focus.focus(w, cx);
                cx.notify();
            },
        );

        div()
            .id(pro_eid(&self.id, "sm-tdlg"))
            .role(Role::Dialog)
            .aria_label(tx.label.clone())
            .on_click(|_, _, cx| cx.stop_propagation())
            .w(px(320.))
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
                    .justify_between()
                    .gap(px(8.))
                    .child(
                        div()
                            .text_size(px(13.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .child(tx.label.clone()),
                    )
                    .child(close_btn),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .text_size(px(12.))
                    .text_color(p.text)
                    .child(from_lbl)
                    .child(pro_icon(ProIcon::ArrowRight, 12., p.muted))
                    .child(to_lbl)
                    .child(div().ml_auto().child(pro_rolling_number(
                        pro_eid(&self.id, format!("sm-tdlg-cnt-{}", tx.id)),
                        f64::from(tx.count),
                        0,
                        "",
                        format!("× {}", tr(cx, "taken", "次触发")),
                        12.,
                        FontWeight::MEDIUM,
                        p.ink,
                    ))),
            )
            .when(!tx.detail.is_empty(), |d| {
                d.child(
                    div()
                        .text_size(px(12.))
                        .line_height(px(17.))
                        .text_color(p.muted)
                        .child(tx.detail.clone()),
                )
            })
            .child(
                div()
                    .w_full()
                    .pt(px(6.))
                    .border_t_1()
                    .border_color(p.line)
                    .flex()
                    .justify_end()
                    .child(pro_btn(
                        pro_eid(&self.id, "sm-fire-tx"),
                        tr(cx, "Traverse now", "立即触发迁移"),
                        Some(ProIcon::Play),
                        ProBtnStyle::Primary,
                        true,
                        cx,
                        move |this, _, cx| {
                            this.inspect = None;
                            this.fire_transition(tid_fire.clone(), cx);
                        },
                    )),
            )
            .into_any_element()
    }
}

impl Render for AgentStateMachine {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let reduced = Theme::global(cx).reduced_motion;
        let elapsed_ms = self.boot.elapsed().as_secs_f32() * 1000.0;
        let live_dwell_extra = self.entered_current_at.elapsed().as_millis() as u64;

        if let Some(ref pl) = self.active_pulse
            && pl.started.elapsed().as_secs_f32() >= pl.duration
        {
            self.active_pulse = None;
        }
        let is_terminal = self
            .states
            .iter()
            .find(|s| s.id == self.current)
            .map(|s| s.kind == MachineStateKind::Final)
            .unwrap_or(false);
        if (!is_terminal || self.active_pulse.is_some()) && !reduced {
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
                .p(px(20.))
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
                        .child(pro_skeleton_bar(180., 20., 6., phase, cx))
                        .child(pro_skeleton_bar(160., 20., 6., phase, cx)),
                )
                .child(pro_skeleton_bar(740., 260., 10., phase, cx).w_full())
                .into_any_element();
        }

        if self.empty || self.states.is_empty() {
            return div()
                .id(self.id.clone())
                .w_full()
                .child(width_sensor)
                .child(pro_empty_state(
                    ProIcon::GitBranch,
                    tr(cx, "No states defined", "未定义状态节点"),
                    tr(
                        cx,
                        "Provide states and transitions to inspect dwell times, retry loops, and live state changes.",
                        "定义状态节点与迁移条件后，即可实时查看状态驻留耗时、重试环路与跳转脉冲。",
                    ),
                    None,
                    cx,
                ))
                .into_any_element();
        }

        let container_w = self.measured_width.get();
        let inner_w = (container_w - 40.0).max(280.0);
        let max_col = self.states.iter().map(|s| s.col).max().unwrap_or(0).max(1);
        let max_row = self.states.iter().map(|s| s.row).max().unwrap_or(0);

        // Responsive card & gap geometry so everything fits inside inner_w without horizontal scrolling!
        let start_lead = if inner_w < 560.0 { 22.0 } else { 36.0 };
        let usable_w = (inner_w - start_lead - 8.0).max(240.0);
        let cols_f = (max_col + 1) as f32;
        let min_gap = if inner_w < 560.0 { 16.0 } else { 32.0 };
        let card_w = ((usable_w - min_gap * (cols_f - 1.0)) / cols_f).clamp(84.0, 128.0);
        let col_gap = if max_col > 0 {
            ((usable_w - card_w * cols_f) / (max_col as f32)).clamp(12.0, 48.0)
        } else {
            24.0
        };
        let card_h = 56.0_f32;
        let top_loop_room = 54.0_f32;
        let row_pitch = 108.0_f32;
        let bottom_loop_room = 52.0_f32;
        let stage_h = top_loop_room + (max_row as f32) * row_pitch + card_h + bottom_loop_room;

        let mut state_rects: HashMap<SharedString, (f32, f32, f32, f32)> =
            HashMap::with_capacity(self.states.len());
        for st in &self.states {
            let x = start_lead + (st.col as f32) * (card_w + col_gap);
            let y = top_loop_room + (st.row as f32) * row_pitch;
            state_rects.insert(st.id.clone(), (x, y, card_w, card_h));
        }

        let current_state = self.states.iter().find(|s| s.id == self.current);
        let cur_label = current_state
            .map(|s| {
                if self.custom_data {
                    s.label.clone()
                } else {
                    tr_sm_state(&s.id, &s.label, cx).into()
                }
            })
            .unwrap_or_else(|| self.current.clone());
        let cur_kind = current_state
            .map(|s| s.kind)
            .unwrap_or(MachineStateKind::Normal);

        let (hdr_bg, hdr_fg, hdr_prefix) = match cur_kind {
            MachineStateKind::Waiting => (p.amber_soft, p.amber_ink, tr(cx, "Needs you", "待确认")),
            MachineStateKind::Final => (p.green_soft, p.green_ink, tr(cx, "Finished", "已完成")),
            MachineStateKind::Error => (p.red_soft, p.red_ink, tr(cx, "Error", "异常")),
            MachineStateKind::Normal => (p.accent_soft, p.blue, tr(cx, "Running", "运行中")),
        };

        let pkey = infer_provider_key(&self.agent_model);
        let total_tx = self.total_transitions();

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
                    .child(pro_agent_avatar(
                        &self.agent_name,
                        p.blue,
                        22.,
                        cur_kind != MachineStateKind::Final,
                        elapsed_ms,
                        cx,
                    ))
                    .child(
                        div()
                            .text_size(px(14.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .child(self.agent_name.clone()),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(4.))
                            .text_size(px(12.5))
                            .text_color(p.muted)
                            .children(pro_app_artwork(pkey, 12., cx))
                            .child(self.agent_model.clone()),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(8.))
                    .child(pro_btn(
                        pro_eid(&self.id, "sm-step-btn"),
                        tr(cx, "Step state", "单步迁移"),
                        Some(ProIcon::Play),
                        ProBtnStyle::Secondary,
                        true,
                        cx,
                        |this, _, cx| {
                            this.step_demo_transition(cx);
                        },
                    ))
                    .child(pro_rolling_number(
                        pro_eid(&self.id, "sm-tot-tx"),
                        f64::from(total_tx),
                        0,
                        "",
                        format!(" {}", tr(cx, "transitions", "次迁移")),
                        12.,
                        FontWeight::NORMAL,
                        p.muted,
                    ))
                    .child(
                        div()
                            .h(px(22.))
                            .px(px(8.))
                            .rounded(px(5.))
                            .bg(hdr_bg)
                            .flex()
                            .items_center()
                            .gap(px(5.))
                            .when(cur_kind == MachineStateKind::Final, |d| {
                                d.child(pro_check_draw(
                                    pro_eid(&self.id, "sm-hdr-chk"),
                                    true,
                                    12.5,
                                    false,
                                    p.green,
                                ))
                            })
                            .child(pro_swap_label(
                                pro_eid(&self.id, "sm-hdr-st"),
                                format!("{hdr_prefix} · {cur_label}"),
                                11.5,
                                FontWeight::MEDIUM,
                                hdr_fg,
                            )),
                    ),
            );

        // Precompute transition polylines, arrowheads, and pill positions
        #[derive(Clone)]
        struct TxGeometry {
            id: SharedString,
            from: SharedString,
            to: SharedString,
            label: SharedString,
            count: u32,
            pts: Vec<(f32, f32)>,
            arrow_dir: (f32, f32),
            pill_pos: (f32, f32),
        }

        let mut tx_geoms: Vec<TxGeometry> = Vec::with_capacity(self.transitions.len());
        let spine_bottom = top_loop_room + card_h;

        for (ti, tx) in self.transitions.iter().enumerate() {
            let (Some(&r0), Some(&r1)) = (state_rects.get(&tx.from), state_rects.get(&tx.to))
            else {
                continue;
            };
            let (pts, arrow_dir, pill_pos) = compute_transition_geometry(r0, r1, spine_bottom, ti);
            tx_geoms.push(TxGeometry {
                id: tx.id.clone(),
                from: tx.from.clone(),
                to: tx.to.clone(),
                label: tx.label.clone(),
                count: tx.count,
                pts,
                arrow_dir,
                pill_pos,
            });
        }

        let focus_state_id = self.hovered_state_id.clone().or_else(|| {
            if let Some(MachineInspect::State(ref sid)) = self.inspect {
                Some(sid.clone())
            } else {
                None
            }
        });
        let focus_tx_id = self.hovered_tx_id.clone().or_else(|| {
            if let Some(MachineInspect::Transition(ref tid)) = self.inspect {
                Some(tid.clone())
            } else {
                None
            }
        });

        let mut tx_focus_weights: HashMap<SharedString, (f32, f32)> =
            HashMap::with_capacity(tx_geoms.len());
        for tg in &tx_geoms {
            let incident = if let Some(ref ftx) = focus_tx_id {
                &tg.id == ftx
            } else if let Some(ref fst) = focus_state_id {
                &tg.from == fst || &tg.to == fst
            } else {
                true
            };
            let spotlighted = (focus_tx_id.is_some() || focus_state_id.is_some()) && incident;
            let vis = pro_spring_f32(
                pro_eid(&self.id, format!("sm-tx-vis-{}", tg.id)),
                if incident { 1.0 } else { 0.22 },
                0.22,
                w,
                cx,
            )
            .clamp(0.0, 1.0);
            let spot = pro_spring_f32(
                pro_eid(&self.id, format!("sm-tx-spt-{}", tg.id)),
                if spotlighted { 1.0 } else { 0.0 },
                0.22,
                w,
                cx,
            )
            .clamp(0.0, 1.0);
            tx_focus_weights.insert(tg.id.clone(), (vis, spot));
        }

        let pulse_info = self.active_pulse.as_ref().and_then(|pl| {
            if reduced {
                None
            } else {
                let u =
                    pro_ease((pl.started.elapsed().as_secs_f32() / pl.duration).clamp(0.0, 1.0));
                Some((pl.tx_id.clone(), u))
            }
        });

        let first_rect = self
            .states
            .first()
            .and_then(|s| state_rects.get(&s.id).copied());
        let tx_paint = tx_geoms.clone();
        let tx_weights_paint = tx_focus_weights.clone();
        let p_canvas = p;

        let arrows_canvas = canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let ox = f32::from(bounds.origin.x);
                let oy = f32::from(bounds.origin.y);

                // Paint start dot -> first state arrow
                if let Some((fx, fy, _, fh)) = first_rect {
                    let sy = oy + fy + fh * 0.5;
                    let sx0 = ox + (fx - 22.0).max(6.0);
                    let sx1 = ox + fx - 2.0;
                    window.paint_quad(
                        fill(
                            Bounds::new(point(px(sx0 - 4.5), px(sy - 4.5)), size(px(9.), px(9.))),
                            p_canvas.ink.opacity(0.75),
                        )
                        .corner_radii(px(4.5)),
                    );
                    let mut pb = PathBuilder::stroke(px(1.5));
                    pb.move_to(point(px(sx0 + 4.5), px(sy)));
                    pb.line_to(point(px(sx1), px(sy)));
                    if let Ok(path) = pb.build() {
                        window.paint_path(path, p_canvas.ink.opacity(0.55));
                    }
                    paint_arrowhead(
                        (sx1, sy),
                        (1.0, 0.0),
                        5.0,
                        p_canvas.ink.opacity(0.70),
                        window,
                    );
                }

                for tg in &tx_paint {
                    if tg.pts.len() < 2 {
                        continue;
                    }
                    let (vis, spot) = tx_weights_paint.get(&tg.id).copied().unwrap_or((1.0, 0.0));
                    let active_u = pulse_info
                        .as_ref()
                        .and_then(|(pid, u)| if pid == &tg.id { Some(*u) } else { None });

                    let rest_col = if tg.count > 0 {
                        p_canvas
                            .ink
                            .opacity((if p_canvas.dark { 0.42 } else { 0.34 }) * vis)
                    } else {
                        p_canvas.line_strong.opacity(0.85 * vis)
                    };
                    let base_col = if active_u.is_some() {
                        p_canvas.blue.opacity(0.85)
                    } else {
                        pro_mix_color(rest_col, p_canvas.blue.opacity(0.82), spot * 0.72)
                    };

                    let stroke_w = if active_u.is_some() {
                        1.9
                    } else {
                        1.4 + 0.45 * spot
                    };
                    let mut pb = PathBuilder::stroke(px(stroke_w));
                    for (i, &(x, y)) in tg.pts.iter().enumerate() {
                        let pt = point(px(ox + x), px(oy + y));
                        if i == 0 {
                            pb.move_to(pt);
                        } else {
                            pb.line_to(pt);
                        }
                    }
                    if let Ok(path) = pb.build() {
                        window.paint_path(path, base_col);
                    }

                    if let Some(&(ex, ey)) = tg.pts.last() {
                        paint_arrowhead((ox + ex, oy + ey), tg.arrow_dir, 5.0, base_col, window);
                    }

                    if let Some(u) = active_u {
                        paint_tx_pulse(&tg.pts, ox, oy, u, p_canvas.blue, window);
                    }
                }
            },
        )
        .absolute()
        .inset_0();

        // Render interactive State Cards
        let breathe = 0.5 + 0.5 * ((elapsed_ms / 1000.0) * 2.8).sin();
        let mut state_elements: Vec<AnyElement> = Vec::with_capacity(self.states.len());
        let states_snapshot = self.states.clone();

        for st in &states_snapshot {
            let Some(&(sx, sy, sw, sh)) = state_rects.get(&st.id) else {
                continue;
            };
            let sid = st.id.clone();
            let sid_hov = sid.clone();
            let is_cur = self.current == sid;

            let incident_to_focus = if let Some(ref ftx) = focus_tx_id {
                self.transitions
                    .iter()
                    .any(|t| &t.id == ftx && (t.from == sid || t.to == sid))
            } else if let Some(ref hid) = focus_state_id {
                hid == &sid
                    || self
                        .transitions
                        .iter()
                        .any(|t| (&t.from == hid && t.to == sid) || (&t.to == hid && t.from == sid))
            } else {
                true
            };
            let st_op = pro_spring_f32(
                pro_eid(&self.id, format!("sm-st-op-{sid}")),
                if incident_to_focus { 1.0 } else { 0.32 },
                0.22,
                w,
                cx,
            )
            .clamp(0.0, 1.0);

            let disp_lbl: SharedString = if self.custom_data {
                st.label.clone()
            } else {
                tr_sm_state(&st.id, &st.label, cx).into()
            };

            let total_ms = if is_cur && st.kind != MachineStateKind::Final {
                st.dwell_ms + live_dwell_extra
            } else {
                st.dwell_ms
            };
            let meta_str = if st.visits == 0 && !is_cur {
                tr(cx, "Not reached", "未到达").to_string()
            } else {
                format!(
                    "{} · {}×",
                    format_dwell_ms(total_ms),
                    format_count(st.visits.max(1))
                )
            };

            let (card_bg, card_bdr) = match (st.kind, is_cur) {
                (MachineStateKind::Waiting, true) => (p.amber_soft, p.amber),
                (MachineStateKind::Waiting, false) => (p.card, p.amber.opacity(0.55)),
                (MachineStateKind::Final, true) => (p.green_soft, p.green),
                (_, true) => (
                    if p.dark {
                        rgb(0x1b2434).into()
                    } else {
                        rgb(0xf2f7ff).into()
                    },
                    p.blue,
                ),
                _ => (p.card, p.line_strong.opacity(0.85)),
            };

            let hov_bdr = if is_cur { p.blue } else { p.ink.opacity(0.35) };
            let hov_bg = pro_mix_color(card_bg, p.hover, 0.70);
            let is_inspected =
                matches!(&self.inspect, Some(MachineInspect::State(id)) if id == &sid);

            let mut tools_mini = div().flex_none().flex().items_center();
            if sw >= 108.0 && !st.tools.is_empty() {
                for (ti, t) in st.tools.iter().take(2).enumerate() {
                    tools_mini = tools_mini.child(
                        div()
                            .when(ti > 0, |d| d.ml(px(-4.)))
                            .child(pro_app_tile(t, 14., cx)),
                    );
                }
            }

            let glow_alpha = if is_cur && !reduced {
                0.14 + 0.12 * breathe
            } else {
                0.0
            };

            state_elements.push(
                pro_tactile(
                    pro_eid(&self.id, format!("sm-st-{sid}")),
                    format!("{disp_lbl}: {meta_str}"),
                    9.,
                    card_bg,
                    hov_bg,
                    card_bdr,
                    hov_bdr,
                    is_inspected,
                    div()
                        .w(px(sw))
                        .h(px(sh))
                        .px(px(9.))
                        .py(px(7.))
                        .flex()
                        .flex_col()
                        .justify_between()
                        .child(
                            div()
                                .w_full()
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .min_w_0()
                                .child(render_state_mark(&self.id, st, is_cur, &p))
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .text_size(px(12.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.strong)
                                        .truncate()
                                        .child(disp_lbl),
                                ),
                        )
                        .child(
                            div()
                                .w_full()
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap(px(4.))
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .text_size(px(10.5))
                                        .text_color(if st.visits == 0 && !is_cur {
                                            p.faint
                                        } else {
                                            p.muted
                                        })
                                        .truncate()
                                        .child(meta_str),
                                )
                                .child(tools_mini),
                        ),
                    cx,
                    move |this, _, cx| {
                        let next_ins = MachineInspect::State(sid.clone());
                        this.inspect = Some(next_ins.clone());
                        this.last_inspect = Some(next_ins);
                        cx.emit(AgentStateMachineEvent::StateSelected {
                            state_id: sid.clone(),
                        });
                        cx.notify();
                    },
                )
                .absolute()
                .left(px(sx))
                .top(px(sy))
                .w(px(sw))
                .h(px(sh))
                .opacity(st_op)
                .when(glow_alpha > 0.01, |d| d.shadow_md())
                .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                    let next = if *hovered {
                        Some(sid_hov.clone())
                    } else if this.hovered_state_id.as_ref() == Some(&sid_hov) {
                        None
                    } else {
                        this.hovered_state_id.clone()
                    };
                    if this.hovered_state_id != next {
                        this.hovered_state_id = next;
                        cx.notify();
                    }
                }))
                .into_any_element(),
            );
        }

        // Render clickable Transition Count Pills (`N×`) with rolling numbers & tactile press
        let mut count_pills: Vec<AnyElement> = Vec::with_capacity(tx_geoms.len());
        let pill_bg: Hsla = if p.dark {
            rgb(0x27272b).into()
        } else {
            rgb(0xffffff).into()
        };

        for tg in &tx_geoms {
            let tid = tg.id.clone();
            let tid_hov = tid.clone();
            let (vis, _) = tx_focus_weights.get(&tid).copied().unwrap_or((1.0, 0.0));
            let (px_c, py_c) = tg.pill_pos;
            let text_col = if tg.count > 0 { p.ink } else { p.faint };
            let is_tx_inspected =
                matches!(&self.inspect, Some(MachineInspect::Transition(id)) if id == &tid);

            count_pills.push(
                pro_tactile(
                    pro_eid(&self.id, format!("sm-cnt-{tid}")),
                    format!("{}: {}×", tg.label, tg.count),
                    999.,
                    pill_bg,
                    pro_mix_color(pill_bg, p.chip_hover, 0.65),
                    p.line_strong,
                    p.blue,
                    is_tx_inspected,
                    div()
                        .h(px(19.))
                        .px(px(5.5))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(pro_rolling_number(
                            pro_eid(&self.id, format!("sm-rnum-{tid}")),
                            f64::from(tg.count),
                            0,
                            "",
                            "×",
                            10.5,
                            FontWeight::MEDIUM,
                            text_col,
                        )),
                    cx,
                    move |this, _, cx| {
                        let next_ins = MachineInspect::Transition(tid.clone());
                        this.inspect = Some(next_ins.clone());
                        this.last_inspect = Some(next_ins);
                        cx.emit(AgentStateMachineEvent::TransitionSelected {
                            transition_id: tid.clone(),
                        });
                        cx.notify();
                    },
                )
                .absolute()
                .left(px(px_c - 14.0))
                .top(px(py_c - 9.5))
                .opacity(vis)
                .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                    let next = if *hovered {
                        Some(tid_hov.clone())
                    } else if this.hovered_tx_id.as_ref() == Some(&tid_hov) {
                        None
                    } else {
                        this.hovered_tx_id.clone()
                    };
                    if this.hovered_tx_id != next {
                        this.hovered_tx_id = next;
                        cx.notify();
                    }
                }))
                .into_any_element(),
            );
        }

        let active_ins = self.inspect.clone().or_else(|| self.last_inspect.clone());
        let open_modal = active_ins.and_then(|ins| {
            let (key, dialog_el): (SharedString, AnyElement) = match ins {
                MachineInspect::State(sid) => {
                    let st = self.states.iter().find(|s| s.id == sid)?;
                    (
                        format!("st-{sid}").into(),
                        self.render_state_dialog(st, live_dwell_extra, &p, cx),
                    )
                }
                MachineInspect::Transition(tid) => {
                    let tx = self.transitions.iter().find(|t| t.id == tid)?;
                    (
                        format!("tx-{tid}").into(),
                        self.render_transition_dialog(tx, &p, cx),
                    )
                }
            };
            let vtx = pro_view_transition(pro_eid(&self.id, "sm-modal-vtx"), key, dialog_el);
            let dark = p.dark;
            let scrim_id = pro_eid(&self.id, "sm-scrim");
            let on_scrim = cx.listener(|this, _: &ClickEvent, w, cx| {
                this.inspect = None;
                this.focus.focus(w, cx);
                cx.notify();
            });
            Some(pro_presence(
                pro_eid(&self.id, "sm-modal-pres"),
                self.inspect.is_some(),
                0.20,
                0.16,
                move |amt| {
                    let scrim_a = (if dark { 0.46 } else { 0.20 }) * amt;
                    div()
                        .id(scrim_id)
                        .absolute()
                        .inset_0()
                        .bg(hsla(0., 0., 0., scrim_a))
                        .flex()
                        .items_center()
                        .justify_center()
                        .p(px(16.))
                        .on_click(on_scrim)
                        .child(
                            div()
                                .relative()
                                .top(px(8. * (1. - amt)))
                                .opacity(amt)
                                .child(vtx),
                        )
                        .into_any_element()
                },
            ))
        });

        div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .role(Role::Region)
            .aria_label(format!("{} state machine", self.agent_name))
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, w, cx| {
                if ev.keystroke.key == "escape" && this.inspect.is_some() {
                    this.inspect = None;
                    this.focus.focus(w, cx);
                    cx.notify();
                }
            }))
            .w_full()
            .p(px(20.))
            .rounded(px(14.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .relative()
            .overflow_hidden()
            .flex()
            .flex_col()
            .gap(px(12.))
            .child(width_sensor)
            .child(header)
            .child(
                div()
                    .w_full()
                    .h(px(stage_h))
                    .relative()
                    .child(arrows_canvas)
                    .children(state_elements)
                    .children(count_pills),
            )
            .children(open_modal)
            .child(pro_toast_banner(self.toast.clone()))
            .into_any_element()
    }
}

fn render_state_mark(
    base_id: &ElementId,
    st: &MachineState,
    is_cur: bool,
    p: &ProPalette,
) -> AnyElement {
    match st.kind {
        MachineStateKind::Final => {
            if is_cur || st.visits > 0 {
                pro_check_draw(
                    pro_eid(base_id, format!("sm-fchk-{}", st.id)),
                    true,
                    14.,
                    false,
                    p.green,
                )
            } else {
                // Bullseye concentric target mark when unreached
                div()
                    .size(px(13.))
                    .flex_none()
                    .rounded_full()
                    .border_2()
                    .border_color(p.muted)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(div().size(px(5.)).rounded_full().bg(p.muted))
                    .into_any_element()
            }
        }
        MachineStateKind::Waiting => div()
            .size(px(10.))
            .flex_none()
            .rounded_full()
            .bg(p.amber)
            .into_any_element(),
        MachineStateKind::Error => div()
            .size(px(10.))
            .flex_none()
            .rounded_full()
            .bg(p.red)
            .into_any_element(),
        MachineStateKind::Normal => {
            let col = st.color.unwrap_or(p.blue);
            div()
                .size(px(9.))
                .flex_none()
                .rounded_full()
                .bg(col)
                .into_any_element()
        }
    }
}

/// Compute `(polyline, arrow_dir, count_pill_center)` for forward, self-retry top loop,
/// backward bottom loop, and vertical/diagonal row transitions.
#[allow(clippy::type_complexity)]
fn compute_transition_geometry(
    r0: (f32, f32, f32, f32),
    r1: (f32, f32, f32, f32),
    spine_bottom: f32,
    idx: usize,
) -> (Vec<(f32, f32)>, (f32, f32), (f32, f32)) {
    let (x0, y0, w0, h0) = r0;
    let (x1, y1, w1, h1) = r1;

    // 1) Self-retry loop on top of the state card!
    if (x0 - x1).abs() < 1.0 && (y0 - y1).abs() < 1.0 {
        let right_x = x0 + w0 * 0.68;
        let left_x = x0 + w0 * 0.32;
        let top_y = (y0 - 26.0).max(8.0);
        let pts = vec![
            (right_x, y0),
            (right_x, top_y + 5.0),
            (right_x - 5.0, top_y),
            (left_x + 5.0, top_y),
            (left_x, top_y + 5.0),
            (left_x, y0 - 2.0),
        ];
        return (pts, (0.0, 1.0), ((left_x + right_x) * 0.5, top_y));
    }

    // 2) Same row forward arrow right (`x1 > x0`)
    if (y0 - y1).abs() < 2.0 && x1 > x0 {
        let sy = y0 + h0 * 0.5;
        let sx = x0 + w0;
        let ex = x1 - 2.0;
        let mut pts = Vec::with_capacity(13);
        for k in 0..=12 {
            let t = k as f32 / 12.0;
            pts.push((sx + (ex - sx) * t, sy));
        }
        return (pts, (1.0, 0.0), ((sx + ex) * 0.5, sy));
    }

    // 3) Same row backward loop underneath (`x1 < x0`)
    if (y0 - y1).abs() < 2.0 && x1 < x0 {
        let start_x = x0 + w0 * 0.36;
        let end_x = x1 + w1 * 0.64;
        let lane_offset = 24.0 + ((idx % 2) as f32) * 14.0;
        let under_y = spine_bottom + lane_offset;
        let pts = vec![
            (start_x, y0 + h0),
            (start_x, under_y - 5.0),
            (start_x - 5.0, under_y),
            (end_x + 5.0, under_y),
            (end_x, under_y - 5.0),
            (end_x, y1 + h1 + 2.0),
        ];
        return (pts, (0.0, -1.0), ((start_x + end_x) * 0.5, under_y));
    }

    // 4) Downward vertical or elbow transition (`y1 > y0`)
    if y1 > y0 {
        let sx = x0 + w0 * 0.5;
        let sy = y0 + h0;
        if (x0 - x1).abs() < 2.0 {
            let ey = y1 - 2.0;
            let pts = vec![(sx, sy), (sx, (sy + ey) * 0.5), (sx, ey)];
            return (pts, (0.0, 1.0), (sx, (sy + ey) * 0.5));
        }
        let ey = y1 + h1 * 0.5;
        let ex = x1 - 2.0;
        let pts = vec![(sx, sy), (sx, ey - 5.0), (sx + 5.0, ey), (ex, ey)];
        return (pts, (1.0, 0.0), (sx, (sy + ey) * 0.5));
    }

    // 5) Upward return transition from lower row (`y1 < y0`)
    let sx = x0 + w0;
    let sy = y0 + h0 * 0.5;
    let ex = x1 + w1 * 0.5;
    let ey = y1 + h1 + 2.0;
    let pts = vec![(sx, sy), (ex - 5.0, sy), (ex, sy - 5.0), (ex, ey)];
    (pts, (0.0, -1.0), ((sx + ex) * 0.5, sy))
}

fn paint_arrowhead(
    tip: (f32, f32),
    dir: (f32, f32),
    size_px: f32,
    color: Hsla,
    window: &mut Window,
) {
    let len = dir.0.hypot(dir.1).max(1e-4);
    let ux = dir.0 / len;
    let uy = dir.1 / len;
    let px_n = -uy;
    let py_n = ux;
    let bx = tip.0 - ux * size_px * 1.25;
    let by = tip.1 - uy * size_px * 1.25;
    let w = size_px * 0.78;

    let mut pb = PathBuilder::fill();
    pb.move_to(point(px(tip.0), px(tip.1)));
    pb.line_to(point(px(bx + px_n * w), px(by + py_n * w)));
    pb.line_to(point(px(bx - px_n * w), px(by - py_n * w)));
    pb.close();
    if let Ok(path) = pb.build() {
        window.paint_path(path, color);
    }
}

fn paint_tx_pulse(pts: &[(f32, f32)], ox: f32, oy: f32, u: f32, color: Hsla, window: &mut Window) {
    // Parameterize by cumulative arc length for smooth speed around corners
    let mut dists = Vec::with_capacity(pts.len());
    dists.push(0.0_f32);
    let mut total = 0.0_f32;
    for w in pts.windows(2) {
        total += (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1);
        dists.push(total);
    }
    if total < 1.0 {
        return;
    }
    let sample_at = |s: f32| -> (f32, f32) {
        let s = s.clamp(0.0, total);
        let idx = dists
            .partition_point(|&d| d < s)
            .saturating_sub(1)
            .min(pts.len() - 2);
        let seg_len = (dists[idx + 1] - dists[idx]).max(1e-4);
        let frac = ((s - dists[idx]) / seg_len).clamp(0.0, 1.0);
        (
            ox + pts[idx].0 + (pts[idx + 1].0 - pts[idx].0) * frac,
            oy + pts[idx].1 + (pts[idx + 1].1 - pts[idx].1) * frac,
        )
    };

    let head_s = u * total;
    let tail_len = (total * 0.32).clamp(12.0, 38.0);
    let steps = 10usize;
    for k in 0..steps {
        let s0 = head_s - tail_len * ((k + 1) as f32 / steps as f32);
        let s1 = head_s - tail_len * (k as f32 / steps as f32);
        if s1 <= 0.0 {
            continue;
        }
        let a = 1.0 - (k as f32 / steps as f32);
        let p0 = sample_at(s0.max(0.0));
        let p1 = sample_at(s1);
        let mut seg = PathBuilder::stroke(px(2.4 + 0.8 * a));
        seg.move_to(point(px(p0.0), px(p0.1)));
        seg.line_to(point(px(p1.0), px(p1.1)));
        if let Ok(path) = seg.build() {
            window.paint_path(path, color.opacity(a * a));
        }
    }
    let hp = sample_at(head_s);
    window.paint_quad(
        fill(
            Bounds::new(
                point(px(hp.0 - 3.2), px(hp.1 - 3.2)),
                size(px(6.4), px(6.4)),
            ),
            color,
        )
        .corner_radii(px(3.2)),
    );
}

fn metric_pair(label: &str, value: String, p: &ProPalette) -> Div {
    div()
        .flex_1()
        .flex()
        .flex_col()
        .gap(px(2.))
        .child(
            div()
                .text_size(px(11.))
                .text_color(p.muted)
                .child(label.to_string()),
        )
        .child(
            div()
                .text_size(px(13.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.ink)
                .child(value),
        )
}

fn format_dwell_ms(ms: u64) -> String {
    let secs = ms / 1000;
    if secs >= 60 {
        format!("{}m {:02}s", secs / 60, secs % 60)
    } else if ms >= 1000 {
        let tenths = (ms % 1000) / 100;
        if secs < 10 && tenths > 0 {
            format!("{secs}.{tenths}s")
        } else {
            format!("{secs}s")
        }
    } else {
        format!("{ms}ms")
    }
}

fn format_count(n: u32) -> String {
    if n >= 10_000 {
        format!("{}K", n / 1_000)
    } else if n >= 1_000 {
        format!("{},{:03}", n / 1_000, n % 1_000)
    } else {
        format!("{n}")
    }
}

fn infer_provider_key(model: &str) -> &'static str {
    let lower = model.to_ascii_lowercase();
    if lower.contains("claude") || lower.contains("opus") || lower.contains("sonnet") {
        "claude"
    } else if lower.contains("gemini") {
        "gemini"
    } else {
        "openai"
    }
}

fn tr_sm_state<'a>(id: &str, fallback: &'a str, cx: &App) -> &'a str {
    match id {
        "planning" => tr(cx, "Planning", "任务规划"),
        "searching" => tr(cx, "Searching", "检索上下文"),
        "coding" => tr(cx, "Coding", "编写补丁"),
        "testing" => tr(cx, "Testing", "执行测试"),
        "waiting" => tr(cx, "Waiting", "等待确认"),
        "done" => tr(cx, "Done", "已完成"),
        _ => fallback,
    }
}

fn default_state_machine() -> (Vec<MachineState>, Vec<MachineTransition>) {
    let states = vec![
        MachineState::new("planning", "Planning", 0, 0)
            .with_color(rgb(0x8b5cf6).into())
            .with_tools(["Linear"])
            .with_metrics(4_200, 2)
            .with_description(
                "Decomposes the issue prompt into file targets and verification steps.",
            ),
        MachineState::new("searching", "Searching", 1, 0)
            .with_color(rgb(0x0ea5e9).into())
            .with_tools(["GitHub", "Notion"])
            .with_metrics(9_100, 3)
            .with_description(
                "Queries repository symbols, recent commits, and workspace architecture docs.",
            ),
        MachineState::new("coding", "Coding", 2, 0)
            .with_color(rgb(0x3b82f6).into())
            .with_tools(["GitHub"])
            .with_metrics(14_500, 2)
            .with_description(
                "Edits TypeScript/Rust source modules and prepares incremental patches.",
            ),
        MachineState::new("testing", "Testing", 3, 0)
            .with_color(rgb(0x14b8a6).into())
            .with_tools(["GitHub"])
            .with_metrics(7_800, 1)
            .with_description(
                "Runs unit, integration, and type-check suites against the candidate patch.",
            ),
        MachineState::new("waiting", "Waiting", 2, 1)
            .with_kind(MachineStateKind::Waiting)
            .with_tools(["Slack"])
            .with_metrics(0, 0)
            .with_description(
                "Pauses before schema migrations or external pushes so a human can approve.",
            ),
        MachineState::new("done", "Done", 4, 0)
            .with_kind(MachineStateKind::Final)
            .with_metrics(0, 0)
            .with_description("All checks passed and final summary delivered to the thread."),
    ];

    let transitions = vec![
        MachineTransition::new("t-plan-search", "planning", "searching", "needs context", 2)
            .with_detail("Agent moves from plan synthesis to repository search."),
        MachineTransition::new(
            "t-search-retry",
            "searching",
            "searching",
            "timeout, retry",
            1,
        )
        .with_detail("Retries transient rate-limited code search query with backoff."),
        MachineTransition::new("t-search-plan", "searching", "planning", "scope changed", 1)
            .with_detail("Discovered extra affected module; loops back underneath to revise plan."),
        MachineTransition::new("t-search-code", "searching", "coding", "found files", 2)
            .with_detail("Sufficient context gathered; begins authoring patch."),
        MachineTransition::new("t-code-test", "coding", "testing", "patch ready", 1)
            .with_detail("Candidate edit compiled; triggers automated test runner."),
        MachineTransition::new("t-test-code", "testing", "coding", "test failed", 1)
            .with_detail("Assertion mismatch in edge case; loops back underneath to refine patch."),
        MachineTransition::new("t-code-wait", "coding", "waiting", "needs approval", 0)
            .with_detail("Migration touches production config; requests operator sign-off."),
        MachineTransition::new("t-wait-test", "waiting", "testing", "approved", 0)
            .with_detail("Operator approved migration; proceeds directly into verification."),
        MachineTransition::new("t-test-done", "testing", "done", "all green", 0)
            .with_detail("Every suite passed with zero warnings; completes run."),
    ];

    (states, transitions)
}

#[cfg(test)]
mod tests {
    use gpui::AppContext;

    use super::{AgentStateMachine, format_count, format_dwell_ms};
    use gpui::TestAppContext;

    #[test]
    fn default_state_machine_starts_in_coding_with_seven_recorded_transitions() {
        assert_eq!(format_count(1_284), "1,284");
        assert_eq!(format_count(12_500), "12K");
        assert_eq!(format_dwell_ms(65_000), "1m 05s");

        let mut app = TestAppContext::single();
        app.update(crate::init);
        let sm = app.new(|cx| AgentStateMachine::new("sm-test", cx));
        sm.read_with(&app, |m, _| {
            assert_eq!(m.current_state_id().as_ref(), "coding");
            assert_eq!(m.total_transitions(), 8);
        });
    }

    #[test]
    fn firing_transitions_and_approving_waiting_state_advance_cleanly() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let sm = app.new(|cx| AgentStateMachine::new("sm-step", cx));
        sm.update(&mut app, |m, cx| {
            m.fire_transition("t-code-wait", cx);
            assert_eq!(m.current_state_id().as_ref(), "waiting");
            assert_eq!(m.total_transitions(), 9);

            m.resolve_waiting_state(true, cx);
            assert_eq!(m.current_state_id().as_ref(), "testing");
            assert_eq!(m.total_transitions(), 10);

            m.fire_transition("t-test-done", cx);
            assert_eq!(m.current_state_id().as_ref(), "done");
        });
    }
}
