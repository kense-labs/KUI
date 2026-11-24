//! Living network topology of agents, tools, bidirectional call pulses, and link heat.
//!
//! Renders orchestrators, specialist agents, tools, and human operators on an orbital circuit canvas
//! where call and answer pulses travel along rounded links, busy links thicken, and amber nodes request approval.

use crate::{
    components::pro_chrome::{
        ProBtnStyle, ProPalette, pro_btn, pro_check_draw, pro_ease, pro_eid, pro_empty_state,
        pro_icon_btn, pro_mix_color, pro_presence, pro_rolling_number, pro_skeleton_bar,
        pro_spring_f32, pro_swap_label, pro_tactile, pro_toast_banner, pro_view_transition, tr,
    },
    foundation::Theme,
    primitives::pro_icons::{ProIcon, pro_app_artwork, pro_icon, pro_svg},
};
use gpui::{prelude::*, *};
use std::{cell::Cell, collections::HashMap, rc::Rc};
use web_time::Instant;

const NETWORK_MARK_SVG: &str = r#"<path d="M8 8.4 3.2 4M8 8.4l5-4.2M8 8.4l4.4 4" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" opacity="0.55"/><circle cx="8" cy="8.4" r="2.4" fill="currentColor"/><circle cx="3.2" cy="4" r="1.6" fill="currentColor"/><circle cx="13" cy="4.2" r="1.6" fill="currentColor"/><circle cx="12.4" cy="12.4" r="1.6" fill="currentColor"/>"#;

/// Category of a node inside [`AgentGraph`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GraphNodeKind {
    /// Autonomous orchestrator or specialist agent on the center/inner orbit.
    #[default]
    Agent,
    /// External API, search engine, or datastore on the outer orbit.
    Tool,
    /// Human operator or requester on the outer orbit.
    User,
}

/// Runtime status of a [`GraphNode`] inside [`AgentGraph`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GraphNodeStatus {
    /// Resting without active work.
    Idle,
    /// Actively executing; emits a soft breathing ring.
    #[default]
    Working,
    /// Waiting on human attention (`"Needs you"`); turns amber with a clock badge.
    Waiting,
    /// Encountered a failure; turns red with an alert badge.
    Error,
    /// Finished its assigned objective.
    Done,
}

impl GraphNodeStatus {
    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::Idle => tr(cx, "Idle", "空闲"),
            Self::Working => tr(cx, "Working", "运行中"),
            Self::Waiting => tr(cx, "Needs you", "待确认"),
            Self::Error => tr(cx, "Failed", "失败"),
            Self::Done => tr(cx, "Done", "已完成"),
        }
    }
}

/// One agent, tool, or user vertex in [`AgentGraph`].
#[derive(Clone, Debug, PartialEq)]
pub struct GraphNode {
    /// Stable node identifier referenced by edges and calls.
    pub id: SharedString,
    /// Full display label.
    pub name: SharedString,
    /// Compact label used on narrow mobile viewports.
    pub short: Option<SharedString>,
    /// Node category (`Agent`, `Tool`, or `User`).
    pub kind: GraphNodeKind,
    /// Model or service subtitle (for example `"Opus 5.5"`).
    pub detail: SharedString,
    /// Current runtime status.
    pub status: GraphNodeStatus,
    /// Present-tense summary inspected in the node dialog.
    pub doing: SharedString,
    /// Approval prompt shown when `status == GraphNodeStatus::Waiting`.
    pub ask: Option<SharedString>,
    /// Accumulated USD spend for this node.
    pub cost: f64,
    /// Whether this agent sits at the center hub of the network.
    pub lead: bool,
    /// Optional normalized `(x, y)` coordinates in `0.0..=1.0` (`None` uses automatic orbital placement).
    pub pos: Option<(f32, f32)>,
}

impl GraphNode {
    /// Construct a [`GraphNode`] with `id`, `name`, `kind`, and `status`.
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        kind: GraphNodeKind,
        status: GraphNodeStatus,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            short: None,
            kind,
            detail: SharedString::default(),
            status,
            doing: SharedString::default(),
            ask: None,
            cost: 0.0,
            lead: false,
            pos: None,
        }
    }

    /// Set a compact short label for narrow viewports.
    pub fn with_short(mut self, short: impl Into<SharedString>) -> Self {
        self.short = Some(short.into());
        self
    }

    /// Set the model or tool detail line.
    pub fn with_detail(mut self, detail: impl Into<SharedString>) -> Self {
        self.detail = detail.into();
        self
    }

    /// Set what this node is currently doing.
    pub fn with_doing(mut self, doing: impl Into<SharedString>) -> Self {
        self.doing = doing.into();
        self
    }

    /// Set an approval prompt when `status == GraphNodeStatus::Waiting`.
    pub fn with_ask(mut self, ask: impl Into<SharedString>) -> Self {
        self.ask = Some(ask.into());
        self
    }

    /// Set accumulated USD spend.
    pub fn with_cost(mut self, cost: f64) -> Self {
        self.cost = if cost.is_finite() && cost >= 0.0 {
            cost
        } else {
            0.0
        };
        self
    }

    /// Mark this agent as the central lead hub.
    pub fn as_lead(mut self) -> Self {
        self.lead = true;
        self
    }

    /// Pin normalized `(x, y)` canvas coordinates in `0.0..=1.0`.
    pub fn with_pos(mut self, x: f32, y: f32) -> Self {
        self.pos = Some((x.clamp(0.06, 0.94), y.clamp(0.08, 0.92)));
        self
    }
}

/// Outcome or flight phase of a [`GraphCall`] in [`AgentGraph`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GraphCallStatus {
    /// Outbound call currently in flight.
    #[default]
    Pending,
    /// Call completed; answer returned cleanly.
    Ok,
    /// Call requires human approval (`"Needs you"`).
    NeedsYou,
    /// Call failed with an error.
    Error,
}

/// Recorded tool or agent invocation across an [`AgentGraph`] link.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphCall {
    /// Unique call identifier.
    pub id: SharedString,
    /// Caller node id.
    pub from: SharedString,
    /// Callee node id.
    pub to: SharedString,
    /// Outcome or pending state.
    pub status: GraphCallStatus,
    /// Query or payload summary (for example `"pricing linear.app"`).
    pub label: SharedString,
    /// Optional round-trip duration in milliseconds.
    pub ms: Option<u32>,
}

impl GraphCall {
    /// Construct a [`GraphCall`] from `from` to `to`.
    pub fn new(
        id: impl Into<SharedString>,
        from: impl Into<SharedString>,
        to: impl Into<SharedString>,
        status: GraphCallStatus,
        label: impl Into<SharedString>,
        ms: Option<u32>,
    ) -> Self {
        Self {
            id: id.into(),
            from: from.into(),
            to: to.into(),
            status,
            label: label.into(),
            ms,
        }
    }
}

/// Typed events emitted by [`AgentGraph`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentGraphEvent {
    /// User clicked a node to open its inspector (or closed the inspector with `None`).
    NodeSelected {
        /// Selected node identifier, or `None` when closed.
        node_id: Option<SharedString>,
    },
    /// A new call pulse was dispatched across the network.
    CallDispatched {
        /// Dispatched call record.
        call: GraphCall,
    },
    /// Operator approved or declined an amber `Waiting` agent.
    Approved {
        /// Target agent node identifier.
        node_id: SharedString,
        /// `true` when approved, `false` when declined.
        approved: bool,
    },
}

impl EventEmitter<AgentGraphEvent> for AgentGraph {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PulseKind {
    Call,
    Answer,
    NeedsYou,
    Failed,
}

#[derive(Clone, Debug)]
struct LivePulse {
    from: SharedString,
    to: SharedString,
    kind: PulseKind,
    started: Instant,
    duration: f32,
}

#[derive(Clone, Debug)]
struct NodeRipple {
    node_id: SharedString,
    kind: PulseKind,
    started: Instant,
}

/// Living orbital network topology of agents, tools, call/answer pulses, and dynamic link heat.
///
/// # Examples
///
/// ```ignore
/// use kui::components::agent_graph::AgentGraph;
///
/// let graph = cx.new(|cx| AgentGraph::new("agent-graph", cx));
/// ```
pub struct AgentGraph {
    id: ElementId,
    focus: FocusHandle,
    title: Option<SharedString>,
    nodes: Vec<GraphNode>,
    edges: Vec<(SharedString, SharedString)>,
    calls: Vec<GraphCall>,
    custom_data: bool,
    selected_id: Option<SharedString>,
    last_selected_id: Option<SharedString>,
    hovered_id: Option<SharedString>,
    pulses: Vec<LivePulse>,
    ripples: Vec<NodeRipple>,
    edge_heat: HashMap<(SharedString, SharedString), (f32, Instant)>,
    auto_traffic: bool,
    demo_tick: usize,
    last_auto: Instant,
    fixed_height: Option<f32>,
    toast: Option<(SharedString, Instant)>,
    loading: bool,
    empty: bool,
    boot: Instant,
    measured_width: Rc<Cell<f32>>,
}

impl AgentGraph {
    /// Create a live [`AgentGraph`] initialized with 5 agents, 6 tools, 1 user node, and active call traffic.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let now = Instant::now();
        let (nodes, edges, calls) = default_graph_topology();
        let mut edge_heat = HashMap::new();
        edge_heat.insert(("research".into(), "web".into()), (3.2, now));
        edge_heat.insert(("lead".into(), "research".into()), (2.4, now));
        edge_heat.insert(("coder".into(), "github".into()), (2.0, now));
        edge_heat.insert(("analyst".into(), "db".into()), (1.8, now));
        let pulses = vec![
            LivePulse {
                from: "research".into(),
                to: "web".into(),
                kind: PulseKind::Call,
                started: now,
                duration: 0.95,
            },
            LivePulse {
                from: "db".into(),
                to: "analyst".into(),
                kind: PulseKind::Answer,
                started: now,
                duration: 1.05,
            },
            LivePulse {
                from: "writer".into(),
                to: "you".into(),
                kind: PulseKind::NeedsYou,
                started: now,
                duration: 1.15,
            },
        ];
        Self {
            id: id.into(),
            focus: cx.focus_handle(),
            title: None,
            nodes,
            edges,
            calls,
            custom_data: false,
            selected_id: None,
            last_selected_id: None,
            hovered_id: None,
            pulses,
            ripples: Vec::new(),
            edge_heat,
            auto_traffic: true,
            demo_tick: 0,
            last_auto: now,
            fixed_height: None,
            toast: None,
            loading: false,
            empty: false,
            boot: now,
            measured_width: Rc::new(Cell::new(760.0)),
        }
    }

    /// Override the header title (defaults to `"Agent graph"` / `"智能体调用拓扑"`).
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Supply custom [`GraphNode`] vertices.
    pub fn with_nodes(mut self, nodes: Vec<GraphNode>) -> Self {
        self.custom_data = true;
        if nodes.is_empty() {
            self.nodes.clear();
            self.empty = true;
        } else {
            self.nodes = nodes;
        }
        self.pulses.clear();
        self
    }

    /// Supply persistent undirected `(from, to)` links.
    pub fn with_edges(
        mut self,
        edges: impl IntoIterator<Item = (impl Into<SharedString>, impl Into<SharedString>)>,
    ) -> Self {
        self.edges = edges
            .into_iter()
            .map(|(a, b)| (a.into(), b.into()))
            .collect();
        self
    }

    /// Supply initial [`GraphCall`] history.
    pub fn with_calls(mut self, calls: Vec<GraphCall>) -> Self {
        self.calls = calls;
        self
    }

    /// Enable or disable continuous background call/answer simulation.
    pub fn with_auto_traffic(mut self, enabled: bool) -> Self {
        self.auto_traffic = enabled;
        if !enabled {
            self.pulses.clear();
        }
        self
    }

    /// Pin an explicit canvas stage height in pixels.
    pub fn with_height(mut self, height: f32) -> Self {
        self.fixed_height = Some(height.max(240.0));
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

    /// Return the currently inspected node id, if any.
    pub fn selected_node_id(&self) -> Option<&SharedString> {
        self.selected_id.as_ref()
    }

    /// Return total accumulated cost across all nodes.
    pub fn total_cost(&self) -> f64 {
        self.nodes.iter().map(|n| n.cost).sum()
    }

    /// Return recent call count displayed in the header bar.
    pub fn calls_per_minute(&self) -> usize {
        self.calls.len()
    }

    /// Select a node (or pass `None` to close the node inspector).
    pub fn select_node(&mut self, node_id: Option<SharedString>, cx: &mut Context<Self>) {
        let valid = node_id.filter(|id| self.nodes.iter().any(|n| &n.id == id));
        if let Some(ref vid) = valid {
            self.last_selected_id = Some(vid.clone());
        }
        if self.selected_id != valid {
            self.selected_id = valid.clone();
            cx.emit(AgentGraphEvent::NodeSelected { node_id: valid });
            cx.notify();
        }
    }

    /// Record a new [`GraphCall`], thicken its link, and launch outbound/return pulses.
    pub fn record_call(&mut self, call: GraphCall, cx: &mut Context<Self>) {
        let key = canonical_edge_key(&call.from, &call.to);
        if !self
            .edges
            .iter()
            .any(|(a, b)| canonical_edge_key(a, b) == key)
        {
            self.edges.push((call.from.clone(), call.to.clone()));
        }
        let now = Instant::now();
        let entry = self.edge_heat.entry(key).or_insert((0.0, now));
        entry.0 = (entry.0 + 1.15).min(4.0);
        entry.1 = now;

        if !Theme::global(cx).reduced_motion {
            let pkind = match call.status {
                GraphCallStatus::Pending => PulseKind::Call,
                GraphCallStatus::Ok => PulseKind::Answer,
                GraphCallStatus::NeedsYou => PulseKind::NeedsYou,
                GraphCallStatus::Error => PulseKind::Failed,
            };
            let (src, dst) = match call.status {
                GraphCallStatus::Ok => (call.to.clone(), call.from.clone()),
                _ => (call.from.clone(), call.to.clone()),
            };
            self.pulses.push(LivePulse {
                from: src,
                to: dst,
                kind: pkind,
                started: now,
                duration: 0.90,
            });
        }
        if self.calls.len() >= 24 {
            self.calls.remove(0);
        }
        self.calls.push(call.clone());
        cx.emit(AgentGraphEvent::CallDispatched { call });
        cx.notify();
    }

    /// Fire a burst of live tool calls across the graph to demonstrate link thickening and return pulses.
    pub fn trigger_call_burst(&mut self, cx: &mut Context<Self>) {
        self.demo_tick = self.demo_tick.wrapping_add(1);
        let idx = self.demo_tick;
        let samples: &[(&str, &str, GraphCallStatus, &str, Option<u32>)] = &[
            (
                "research",
                "web",
                GraphCallStatus::Pending,
                "pricing figma.com/enterprise",
                None,
            ),
            (
                "research",
                "web",
                GraphCallStatus::Ok,
                "200 OK · 5 tiers parsed",
                Some(640),
            ),
            (
                "coder",
                "github",
                GraphCallStatus::Ok,
                "push feat/pricing-grid",
                Some(410),
            ),
            (
                "analyst",
                "db",
                GraphCallStatus::Ok,
                "SELECT plan, seats FROM orgs",
                Some(190),
            ),
            (
                "lead",
                "analyst",
                GraphCallStatus::Pending,
                "Validate annual ARPU lift",
                None,
            ),
        ];
        let s = samples[idx % samples.len()];
        if self.nodes.iter().any(|n| n.id.as_ref() == s.0)
            && self.nodes.iter().any(|n| n.id.as_ref() == s.1)
        {
            if let Some(caller) = self.nodes.iter_mut().find(|n| n.id.as_ref() == s.0)
                && caller.kind == GraphNodeKind::Agent
            {
                caller.cost += 0.01;
            }
            self.record_call(
                GraphCall::new(format!("burst-{idx}"), s.0, s.1, s.2, s.3, s.4),
                cx,
            );
        }
    }

    /// Approve or decline an amber `Waiting` agent node.
    pub fn approve_node(
        &mut self,
        node_id: impl Into<SharedString>,
        approved: bool,
        cx: &mut Context<Self>,
    ) {
        let target = node_id.into();
        let mut node_name = target.clone();
        if let Some(node) = self.nodes.iter_mut().find(|n| n.id == target) {
            node_name = node.name.clone();
            node.status = if approved {
                GraphNodeStatus::Working
            } else {
                GraphNodeStatus::Idle
            };
            node.ask = None;
            node.doing = if approved {
                tr(
                    cx,
                    "Publishing approved enterprise copy to Notion",
                    "正在将已批准的企业版文案同步至 Notion",
                )
                .into()
            } else {
                tr(
                    cx,
                    "Paused awaiting revised wording",
                    "已暂停，等待修订文案",
                )
                .into()
            };
            node.cost += 0.02;
        }
        if !Theme::global(cx).reduced_motion && self.nodes.iter().any(|n| n.id.as_ref() == "you") {
            self.pulses.push(LivePulse {
                from: "you".into(),
                to: target.clone(),
                kind: if approved {
                    PulseKind::Answer
                } else {
                    PulseKind::NeedsYou
                },
                started: Instant::now(),
                duration: 0.88,
            });
        }
        self.selected_id = None;
        let msg = if approved {
            format!("{} {node_name}", tr(cx, "Approved", "已批准"))
        } else {
            format!("{} {node_name}", tr(cx, "Declined", "已拒绝"))
        };
        self.toast = Some((msg.into(), Instant::now()));
        cx.emit(AgentGraphEvent::Approved {
            node_id: target,
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

    /// Return whether the graph is empty.
    pub fn is_empty(&self) -> bool {
        self.empty || self.nodes.is_empty()
    }

    fn step_simulation(&mut self, reduced: bool) -> bool {
        if reduced {
            self.pulses.clear();
            self.ripples.clear();
            return false;
        }
        let now = Instant::now();

        // Spawn ripples for completed pulses
        let mut landed: Vec<(SharedString, PulseKind)> = Vec::new();
        self.pulses.retain(|pl| {
            let age = now.duration_since(pl.started).as_secs_f32();
            if age >= pl.duration {
                landed.push((pl.to.clone(), pl.kind));
                false
            } else {
                true
            }
        });
        for (nid, kind) in landed {
            self.ripples.push(NodeRipple {
                node_id: nid,
                kind,
                started: now,
            });
        }
        self.ripples
            .retain(|rp| now.duration_since(rp.started).as_secs_f32() < 0.60);

        // Decay link heat smoothly after 4s of quiet
        self.edge_heat.retain(|_, (heat, touched)| {
            let quiet = now.duration_since(*touched).as_secs_f32();
            if quiet > 3.5 {
                *heat = (*heat - 0.03).max(0.0);
            }
            *heat > 0.04
        });

        // Periodic background call & answer flights
        if self.auto_traffic && now.duration_since(self.last_auto).as_secs_f32() >= 1.55 {
            self.last_auto = now;
            self.demo_tick = self.demo_tick.wrapping_add(1);
            let script: &[(&str, &str, PulseKind)] = &[
                ("lead", "research", PulseKind::Call),
                ("research", "web", PulseKind::Call),
                ("web", "research", PulseKind::Answer),
                ("coder", "github", PulseKind::Call),
                ("github", "coder", PulseKind::Answer),
                ("analyst", "db", PulseKind::Call),
                ("db", "analyst", PulseKind::Answer),
                ("research", "lead", PulseKind::Answer),
            ];
            let (f, t, k) = script[self.demo_tick % script.len()];
            if self.nodes.iter().any(|n| n.id.as_ref() == f)
                && self.nodes.iter().any(|n| n.id.as_ref() == t)
            {
                let key = canonical_edge_key(&f.into(), &t.into());
                let entry = self.edge_heat.entry(key).or_insert((0.0, now));
                entry.0 = (entry.0 + 0.85).min(3.8);
                entry.1 = now;
                self.pulses.push(LivePulse {
                    from: f.into(),
                    to: t.into(),
                    kind: k,
                    started: now,
                    duration: 0.92,
                });
            }
        }

        let any_working = self.nodes.iter().any(|n| {
            matches!(
                n.status,
                GraphNodeStatus::Working | GraphNodeStatus::Waiting
            )
        });
        any_working || !self.pulses.is_empty() || !self.ripples.is_empty()
    }

    fn render_node_dialog(
        &self,
        node: &GraphNode,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let nid = node.id.clone();
        let disp_name = if self.custom_data {
            node.name.clone()
        } else {
            tr_graph_name(&node.id, &node.name, cx).into()
        };
        let related_calls: Vec<&GraphCall> = self
            .calls
            .iter()
            .rev()
            .filter(|c| c.from == nid || c.to == nid)
            .take(4)
            .collect();

        let close_btn = pro_icon_btn(
            pro_eid(&self.id, "gclose-dlg"),
            tr(cx, "Close inspector", "关闭节点详情"),
            ProIcon::Close,
            24.,
            false,
            cx,
            |this, w, cx| {
                this.select_node(None, cx);
                this.focus.focus(w, cx);
            },
        );

        let (pill_bg, pill_fg) = match node.status {
            GraphNodeStatus::Working => (p.accent_soft, p.blue),
            GraphNodeStatus::Waiting => (p.amber_soft, p.amber_ink),
            GraphNodeStatus::Error => (p.red_soft, p.red_ink),
            GraphNodeStatus::Done => (p.green_soft, p.green_ink),
            GraphNodeStatus::Idle => (p.chip, p.muted),
        };

        let mut card = div()
            .id(pro_eid(&self.id, "gnode-dialog"))
            .role(Role::Dialog)
            .aria_label(disp_name.clone())
            .on_click(|_, _, cx| cx.stop_propagation())
            .w(px(316.))
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
                    .child(render_node_glyph(node, 26., p, cx))
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
                                    .truncate()
                                    .child(disp_name),
                            )
                            .when(!node.detail.is_empty(), |d| {
                                d.child(
                                    div()
                                        .text_size(px(11.5))
                                        .text_color(p.muted)
                                        .truncate()
                                        .child(node.detail.clone()),
                                )
                            }),
                    )
                    .when(node.kind == GraphNodeKind::Agent, |d| {
                        let status_chk = node.status == GraphNodeStatus::Done;
                        d.child(
                            div()
                                .h(px(19.))
                                .px(px(6.))
                                .rounded(px(4.))
                                .bg(pill_bg)
                                .text_color(pill_fg)
                                .text_size(px(11.))
                                .font_weight(FontWeight::MEDIUM)
                                .flex()
                                .items_center()
                                .gap(px(4.))
                                .when(status_chk, |el| {
                                    el.child(pro_check_draw(
                                        pro_eid(&self.id, format!("gdlg-chk-{nid}")),
                                        true,
                                        12.,
                                        false,
                                        p.green,
                                    ))
                                })
                                .child(pro_swap_label(
                                    pro_eid(&self.id, format!("gdlg-st-{nid}")),
                                    node.status.label(cx),
                                    11.,
                                    FontWeight::MEDIUM,
                                    pill_fg,
                                )),
                        )
                    })
                    .child(close_btn),
            );

        if !node.doing.is_empty() {
            card = card.child(
                div()
                    .text_size(px(12.))
                    .line_height(px(17.))
                    .text_color(p.text)
                    .child(node.doing.clone()),
            );
        }

        if node.status == GraphNodeStatus::Waiting
            && let Some(ask) = &node.ask
        {
            let a_dec = nid.clone();
            let a_app = nid.clone();
            card = card
                .child(
                    div()
                        .p(px(8.))
                        .rounded(px(7.))
                        .bg(p.amber_soft)
                        .text_size(px(12.))
                        .line_height(px(17.))
                        .text_color(p.amber_ink)
                        .child(ask.clone()),
                )
                .child(
                    div()
                        .w_full()
                        .flex()
                        .justify_end()
                        .gap(px(6.))
                        .child(pro_btn(
                            pro_eid(&self.id, "g-decline"),
                            tr(cx, "Decline", "拒绝"),
                            None,
                            ProBtnStyle::Secondary,
                            true,
                            cx,
                            move |this, _, cx| {
                                this.approve_node(a_dec.clone(), false, cx);
                            },
                        ))
                        .child(pro_btn(
                            pro_eid(&self.id, "g-approve"),
                            tr(cx, "Approve", "批准"),
                            Some(ProIcon::Check),
                            ProBtnStyle::Primary,
                            true,
                            cx,
                            move |this, _, cx| {
                                this.approve_node(a_app.clone(), true, cx);
                            },
                        )),
                );
        }

        if !related_calls.is_empty() {
            let mut list = div()
                .w_full()
                .border_t_1()
                .border_b_1()
                .border_color(p.line)
                .flex()
                .flex_col();
            for (ci, c) in related_calls.into_iter().enumerate() {
                let outgoing = c.from == nid;
                let peer_id = if outgoing { &c.to } else { &c.from };
                let peer_name = self
                    .nodes
                    .iter()
                    .find(|n| &n.id == peer_id)
                    .map(|n| n.name.clone())
                    .unwrap_or_else(|| peer_id.clone());
                let right_txt = match (c.status, c.ms) {
                    (GraphCallStatus::Pending, _) => tr(cx, "Calling…", "调用中…").to_string(),
                    (GraphCallStatus::Error, _) => tr(cx, "Failed", "失败").to_string(),
                    (GraphCallStatus::NeedsYou, _) => tr(cx, "Needs you", "待确认").to_string(),
                    (GraphCallStatus::Ok, Some(ms)) => format!("{ms}ms"),
                    (GraphCallStatus::Ok, None) => "OK".to_string(),
                };
                let right_col = match c.status {
                    GraphCallStatus::Pending => p.blue,
                    GraphCallStatus::Error => p.red_ink,
                    GraphCallStatus::NeedsYou => p.amber_ink,
                    GraphCallStatus::Ok => p.muted,
                };
                list = list.child(
                    div()
                        .w_full()
                        .h(px(28.))
                        .when(ci > 0, |d| d.border_t_1().border_color(p.line_soft))
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .text_size(px(11.5))
                        .child(pro_icon(
                            if outgoing {
                                ProIcon::ArrowRight
                            } else {
                                ProIcon::ArrowLeft
                            },
                            11.,
                            p.faint,
                        ))
                        .child(
                            div()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.ink)
                                .child(peer_name),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .text_color(p.muted)
                                .truncate()
                                .child(c.label.clone()),
                        )
                        .child(div().flex_none().text_color(right_col).child(right_txt)),
                );
            }
            card = card.child(list);
        }

        let call_cnt = self
            .calls
            .iter()
            .filter(|c| c.from == nid || c.to == nid)
            .count();
        card = card.child(
            div()
                .w_full()
                .flex()
                .items_center()
                .justify_between()
                .text_size(px(11.5))
                .text_color(p.muted)
                .child(pro_rolling_number(
                    pro_eid(&self.id, format!("gdlg-calls-{nid}")),
                    call_cnt as f64,
                    0,
                    "",
                    format!(" {}", tr(cx, "recent calls", "次近期调用")),
                    11.5,
                    FontWeight::NORMAL,
                    p.muted,
                ))
                .when(node.cost > 0.0, |d| {
                    d.child(pro_rolling_number(
                        pro_eid(&self.id, format!("gdlg-cost-{nid}")),
                        node.cost,
                        2,
                        "$",
                        "",
                        11.5,
                        FontWeight::MEDIUM,
                        p.ink,
                    ))
                }),
        );

        let dialog_body = pro_view_transition(pro_eid(&self.id, "gmodal-vtx"), nid, card);
        let dark = p.dark;
        let scrim_id = pro_eid(&self.id, "gmodal-scrim");
        let on_scrim = cx.listener(|this, _: &ClickEvent, w, cx| {
            this.select_node(None, cx);
            this.focus.focus(w, cx);
        });
        pro_presence(
            pro_eid(&self.id, "gmodal-pres"),
            self.selected_id.is_some(),
            0.20,
            0.16,
            move |amt| {
                let scrim_a = (if dark { 0.45 } else { 0.20 }) * amt;
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
                            .child(dialog_body),
                    )
                    .into_any_element()
            },
        )
    }
}

impl Render for AgentGraph {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let reduced = Theme::global(cx).reduced_motion;
        if self.step_simulation(reduced) {
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
            let phase = (self.boot.elapsed().as_secs_f32() / 1.2).fract();
            if !reduced {
                w.request_animation_frame();
            }
            return div()
                .id(self.id.clone())
                .w_full()
                .p(px(12.))
                .flex()
                .flex_col()
                .gap(px(14.))
                .child(width_sensor)
                .child(
                    div()
                        .w_full()
                        .flex()
                        .justify_between()
                        .child(pro_skeleton_bar(140., 18., 5., phase, cx))
                        .child(pro_skeleton_bar(220., 16., 5., phase, cx)),
                )
                .child(pro_skeleton_bar(740., 440., 10., phase, cx).w_full())
                .into_any_element();
        }

        if self.empty || self.nodes.is_empty() {
            return div()
                .id(self.id.clone())
                .w_full()
                .child(width_sensor)
                .child(pro_empty_state(
                    ProIcon::Activity,
                    tr(cx, "No network nodes active", "网络中暂无活跃节点"),
                    tr(
                        cx,
                        "Connect agents and tools to visualize live call pulses, link traffic, and pending approvals.",
                        "连接智能体与外部工具后，即可实时查看调用光脉冲、链路热度与待确认状态。",
                    ),
                    None,
                    cx,
                ))
                .into_any_element();
        }

        let container_w = self.measured_width.get();
        let narrow = container_w < 540.0;
        let stage_h = self.fixed_height.unwrap_or_else(|| {
            if narrow {
                (container_w * 1.05).clamp(380.0, 500.0)
            } else {
                (container_w * 0.60).clamp(390.0, 486.0)
            }
        });

        let agent_cnt = self
            .nodes
            .iter()
            .filter(|n| n.kind == GraphNodeKind::Agent)
            .count();
        let tool_cnt = self
            .nodes
            .iter()
            .filter(|n| n.kind == GraphNodeKind::Tool)
            .count();
        let waiting_nodes: Vec<SharedString> = self
            .nodes
            .iter()
            .filter(|n| n.status == GraphNodeStatus::Waiting)
            .map(|n| n.id.clone())
            .collect();
        let error_nodes: Vec<SharedString> = self
            .nodes
            .iter()
            .filter(|n| n.status == GraphNodeStatus::Error)
            .map(|n| n.id.clone())
            .collect();
        let total_cost = self.total_cost();
        let cpm = self.calls_per_minute();

        let title_str: SharedString = self
            .title
            .clone()
            .unwrap_or_else(|| tr(cx, "Agent graph", "智能体调用拓扑").into());

        let wait_chip = waiting_nodes.first().cloned().map(|wid| {
            let cnt = waiting_nodes.len();
            let lbl = format!("{cnt} {}", tr(cx, "needs you", "待确认"));
            pro_tactile(
                pro_eid(&self.id, "g-wait-chip"),
                lbl.clone(),
                5.,
                p.amber_soft,
                p.amber.opacity(0.24),
                transparent_black(),
                p.amber.opacity(0.45),
                false,
                div()
                    .h(px(22.))
                    .px(px(7.))
                    .text_color(p.amber_ink)
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .child(pro_icon(ProIcon::Clock, 12., p.amber_ink))
                    .child(lbl),
                cx,
                move |this, _, cx| {
                    this.select_node(Some(wid.clone()), cx);
                },
            )
        });

        let err_chip = error_nodes.first().cloned().map(|eid| {
            let cnt = error_nodes.len();
            let lbl = format!("{cnt} {}", tr(cx, "failed", "失败"));
            pro_tactile(
                pro_eid(&self.id, "g-err-chip"),
                lbl.clone(),
                5.,
                p.red_soft,
                p.red.opacity(0.24),
                transparent_black(),
                p.red.opacity(0.45),
                false,
                div()
                    .h(px(22.))
                    .px(px(7.))
                    .text_color(p.red_ink)
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .child(pro_icon(ProIcon::AlertTriangle, 12., p.red_ink))
                    .child(lbl),
                cx,
                move |this, _, cx| {
                    this.select_node(Some(eid.clone()), cx);
                },
            )
        });

        let header = div()
            .w_full()
            .mb(px(14.))
            .flex()
            .flex_wrap()
            .items_center()
            .justify_between()
            .gap(px(8.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(7.))
                    .child(pro_svg("0 0 16 16", NETWORK_MARK_SVG, 16., p.blue))
                    .child(
                        div()
                            .text_size(px(13.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .child(title_str.clone()),
                    )
                    .children(wait_chip)
                    .children(err_chip),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(10.))
                    .text_size(px(12.))
                    .text_color(p.muted)
                    .child(pro_btn(
                        pro_eid(&self.id, "g-burst-btn"),
                        tr(cx, "Send burst", "发送调用簇"),
                        Some(ProIcon::Bolt),
                        ProBtnStyle::Secondary,
                        true,
                        cx,
                        |this, _, cx| {
                            this.trigger_call_burst(cx);
                        },
                    ))
                    .child(format!("{agent_cnt} {}", tr(cx, "agents", "个智能体")))
                    .child(format!("{tool_cnt} {}", tr(cx, "tools", "个工具")))
                    .child(pro_rolling_number(
                        pro_eid(&self.id, "g-cpm"),
                        cpm as f64,
                        0,
                        "",
                        format!(" {}", tr(cx, "calls/min", "次调用/分")),
                        12.,
                        FontWeight::NORMAL,
                        p.muted,
                    ))
                    .child(pro_rolling_number(
                        pro_eid(&self.id, "g-total-cost"),
                        total_cost,
                        2,
                        "$",
                        "",
                        12.,
                        FontWeight::MEDIUM,
                        p.ink,
                    )),
            );

        // Compute normalized positions for all nodes
        let coords = resolve_node_positions(&self.nodes);
        let focus_node = self.hovered_id.clone().or_else(|| self.selected_id.clone());

        // Determine neighbor set for focus dimming
        let mut neighbors: Vec<SharedString> = Vec::new();
        if let Some(ref fid) = focus_node {
            neighbors.push(fid.clone());
            for (a, b) in &self.edges {
                if a == fid {
                    neighbors.push(b.clone());
                } else if b == fid {
                    neighbors.push(a.clone());
                }
            }
        }

        // Pre-sample smooth spring focus weights per edge for connected-path spotlight
        let mut edge_focus_weights: HashMap<(SharedString, SharedString), (f32, f32)> =
            HashMap::with_capacity(self.edges.len());
        for (a, b) in &self.edges {
            let key = canonical_edge_key(a, b);
            let near = match &focus_node {
                Some(fid) => a == fid || b == fid,
                None => true,
            };
            let target_vis = if near { 1.0 } else { 0.22 };
            let target_spot = if focus_node.is_some() && near {
                1.0
            } else {
                0.0
            };
            let vis = pro_spring_f32(
                pro_eid(&self.id, format!("ge-vis-{}-{}", key.0, key.1)),
                target_vis,
                0.22,
                w,
                cx,
            )
            .clamp(0.0, 1.0);
            let spot = pro_spring_f32(
                pro_eid(&self.id, format!("ge-spt-{}-{}", key.0, key.1)),
                target_spot,
                0.22,
                w,
                cx,
            )
            .clamp(0.0, 1.0);
            edge_focus_weights.insert(key, (vis, spot));
        }

        let edges_paint = self.edges.clone();
        let coords_paint = coords.clone();
        let heat_paint = self.edge_heat.clone();
        let pulses_paint = self.pulses.clone();
        let ripples_paint = self.ripples.clone();
        let nodes_status: HashMap<SharedString, GraphNodeStatus> = self
            .nodes
            .iter()
            .map(|n| (n.id.clone(), n.status))
            .collect();
        let p_canvas = p;
        let elapsed_s = self.boot.elapsed().as_secs_f32();

        let network_canvas = canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let ox = f32::from(bounds.origin.x);
                let oy = f32::from(bounds.origin.y);
                let bw = f32::from(bounds.size.width).max(100.0);
                let bh = f32::from(bounds.size.height).max(100.0);
                let to_px = |(nx, ny): (f32, f32)| -> (f32, f32) { (ox + nx * bw, oy + ny * bh) };

                // Soft center radial glow behind lead hub
                let (hx, hy) = to_px((0.50, 0.50));
                for (rx, ry, alpha) in [
                    (135.0, 78.0, if p_canvas.dark { 0.055 } else { 0.045 }),
                    (82.0, 48.0, if p_canvas.dark { 0.08 } else { 0.06 }),
                ] {
                    window.paint_quad(
                        fill(
                            Bounds::new(
                                point(px(hx - rx), px(hy - ry)),
                                size(px(rx * 2.0), px(ry * 2.0)),
                            ),
                            p_canvas.blue.opacity(alpha),
                        )
                        .corner_radii(px(ry)),
                    );
                }

                // Paint rounded circuit edges with dynamic heat stroke-width & smooth spotlight
                for (a, b) in &edges_paint {
                    let (Some(&pa), Some(&pb_norm)) = (coords_paint.get(a), coords_paint.get(b))
                    else {
                        continue;
                    };
                    let p0 = to_px(pa);
                    let p1 = to_px(pb_norm);
                    let pts = build_circuit_polyline(p0, p1, 8.0);
                    let key = canonical_edge_key(a, b);
                    let heat = heat_paint.get(&key).map(|(h, _)| *h).unwrap_or(0.0);
                    let (vis, spot) = edge_focus_weights.get(&key).copied().unwrap_or((1.0, 0.0));
                    let stroke_w = (1.35 + heat * 0.62 + 0.50 * spot).clamp(1.35, 4.1);

                    let a_st = nodes_status.get(a).copied().unwrap_or_default();
                    let b_st = nodes_status.get(b).copied().unwrap_or_default();
                    let is_ask =
                        a_st == GraphNodeStatus::Waiting || b_st == GraphNodeStatus::Waiting;
                    let is_err = a_st == GraphNodeStatus::Error || b_st == GraphNodeStatus::Error;

                    let rest_col = if is_err {
                        p_canvas.red.opacity(0.80 * vis)
                    } else if is_ask {
                        p_canvas.amber.opacity(0.72 * vis)
                    } else if heat > 0.3 {
                        p_canvas
                            .blue
                            .opacity(((0.24 + heat * 0.12).clamp(0.24, 0.68)) * vis)
                    } else {
                        p_canvas.line_strong.opacity(0.95 * vis)
                    };
                    let spot_col = if is_err {
                        p_canvas.red.opacity(0.92)
                    } else if is_ask {
                        p_canvas.amber.opacity(0.88)
                    } else {
                        p_canvas.blue.opacity(0.82)
                    };
                    let edge_col = pro_mix_color(rest_col, spot_col, spot * 0.65);

                    let mut feather_b = PathBuilder::stroke(px(stroke_w + 1.4));
                    let mut path_b = PathBuilder::stroke(px(stroke_w));
                    for (i, &(x, y)) in pts.iter().enumerate() {
                        if i == 0 {
                            feather_b.move_to(point(px(x), px(y)));
                            path_b.move_to(point(px(x), px(y)));
                        } else {
                            feather_b.line_to(point(px(x), px(y)));
                            path_b.line_to(point(px(x), px(y)));
                        }
                    }
                    if let Ok(feather) = feather_b.build() {
                        window.paint_path(feather, edge_col.opacity(edge_col.a * 0.22));
                    }
                    if let Ok(built) = path_b.build() {
                        window.paint_path(built, edge_col);
                    }
                }

                // Paint traveling Call & Answer pulses along the circuit polylines
                for pl in &pulses_paint {
                    let (Some(&pa), Some(&pb_norm)) =
                        (coords_paint.get(&pl.from), coords_paint.get(&pl.to))
                    else {
                        continue;
                    };
                    let pts = build_circuit_polyline(to_px(pa), to_px(pb_norm), 8.0);
                    if pts.len() < 2 {
                        continue;
                    }
                    let u = pro_ease(
                        (pl.started.elapsed().as_secs_f32() / pl.duration).clamp(0.0, 1.0),
                    );
                    let col = match pl.kind {
                        PulseKind::Call => p_canvas.blue,
                        PulseKind::Answer => p_canvas.green,
                        PulseKind::NeedsYou => p_canvas.amber,
                        PulseKind::Failed => p_canvas.red,
                    };
                    paint_polyline_streak(
                        &pts,
                        u,
                        col,
                        pl.kind == PulseKind::Answer,
                        p_canvas.card,
                        window,
                    );
                }

                // Paint landing ripples on nodes
                for rp in &ripples_paint {
                    if let Some(&pn) = coords_paint.get(&rp.node_id) {
                        let (cx_px, cy_px) = to_px(pn);
                        let t = (rp.started.elapsed().as_secs_f32() / 0.60).clamp(0.0, 1.0);
                        let side = 36.0 + 26.0 * pro_ease(t);
                        let alpha = (1.0 - t) * 0.55;
                        let rcol = match rp.kind {
                            PulseKind::Call => p_canvas.blue,
                            PulseKind::Answer => p_canvas.green,
                            PulseKind::NeedsYou => p_canvas.amber,
                            PulseKind::Failed => p_canvas.red,
                        };
                        window.paint_quad(quad(
                            Bounds::new(
                                point(px(cx_px - side * 0.5), px(cy_px - side * 0.5)),
                                size(px(side), px(side)),
                            ),
                            Corners::all(px(side * 0.34)),
                            transparent_black(),
                            Edges::all(px(1.4)),
                            rcol.opacity(alpha),
                            BorderStyle::Solid,
                        ));
                    }
                }

                // Breathing rings around working & waiting agents
                let breathe = 0.5 + 0.5 * (elapsed_s * 2.4).sin();
                for (nid, st) in &nodes_status {
                    if !matches!(st, GraphNodeStatus::Working | GraphNodeStatus::Waiting) {
                        continue;
                    }
                    let Some(&pn) = coords_paint.get(nid) else {
                        continue;
                    };
                    let (cx_px, cy_px) = to_px(pn);
                    let base = if nid.as_ref() == "lead" { 48.0 } else { 38.0 };
                    let ring_side = base + 6.0 + 7.0 * breathe;
                    let rcol = if *st == GraphNodeStatus::Waiting {
                        p_canvas.amber.opacity(0.28 + 0.22 * breathe)
                    } else {
                        p_canvas.blue.opacity(0.14 + 0.14 * breathe)
                    };
                    window.paint_quad(quad(
                        Bounds::new(
                            point(px(cx_px - ring_side * 0.5), px(cy_px - ring_side * 0.5)),
                            size(px(ring_side), px(ring_side)),
                        ),
                        Corners::all(px(ring_side * 0.36)),
                        rcol.opacity(0.18),
                        Edges::all(px(1.2)),
                        rcol,
                        BorderStyle::Solid,
                    ));
                }
            },
        )
        .absolute()
        .inset_0();

        // Render interactive node discs positioned over the stage
        let nodes_snapshot = self.nodes.clone();
        let mut node_elements: Vec<AnyElement> = Vec::with_capacity(nodes_snapshot.len());

        for node in &nodes_snapshot {
            let Some(&(nx, ny)) = coords.get(&node.id) else {
                continue;
            };
            let side: f32 = if node.lead {
                if narrow { 42.0 } else { 48.0 }
            } else if node.kind == GraphNodeKind::Agent {
                if narrow { 34.0 } else { 38.0 }
            } else if narrow {
                28.0
            } else {
                32.0
            };

            let dimmed = focus_node.is_some() && !neighbors.contains(&node.id);
            let node_op = pro_spring_f32(
                pro_eid(&self.id, format!("gn-op-{}", node.id)),
                if dimmed { 0.32 } else { 1.0 },
                0.22,
                w,
                cx,
            )
            .clamp(0.0, 1.0);
            let disp_name: SharedString = if narrow && let Some(sh) = &node.short {
                sh.clone()
            } else if self.custom_data {
                node.name.clone()
            } else {
                tr_graph_name(&node.id, &node.name, cx).into()
            };

            let (disc_bg, disc_border) = match (node.kind, node.status) {
                (GraphNodeKind::User, _) => (p.primary, p.primary),
                (_, GraphNodeStatus::Waiting) => (
                    if p.dark {
                        rgb(0x332714).into()
                    } else {
                        rgb(0xfff6e5).into()
                    },
                    p.amber,
                ),
                (_, GraphNodeStatus::Error) => (
                    if p.dark {
                        rgb(0x331918).into()
                    } else {
                        rgb(0xffefed).into()
                    },
                    p.red,
                ),
                _ => (p.card, p.line_strong),
            };

            let badge = match node.status {
                GraphNodeStatus::Waiting => Some(
                    div()
                        .absolute()
                        .top(px(-4.))
                        .right(px(-4.))
                        .size(px(14.))
                        .rounded_full()
                        .bg(p.amber)
                        .border_1()
                        .border_color(p.panel)
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(pro_icon(ProIcon::Clock, 9.5, rgb(0x111113).into()))
                        .into_any_element(),
                ),
                GraphNodeStatus::Error => Some(
                    div()
                        .absolute()
                        .top(px(-4.))
                        .right(px(-4.))
                        .size(px(14.))
                        .rounded_full()
                        .bg(p.red)
                        .border_1()
                        .border_color(p.panel)
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(pro_icon(ProIcon::AlertTriangle, 9.5, rgb(0xffffff).into()))
                        .into_any_element(),
                ),
                GraphNodeStatus::Done => Some(
                    div()
                        .absolute()
                        .top(px(-4.))
                        .right(px(-4.))
                        .child(pro_check_draw(
                            pro_eid(&self.id, format!("gn-chk-{}", node.id)),
                            true,
                            14.,
                            false,
                            p.green,
                        ))
                        .into_any_element(),
                ),
                _ => None,
            };

            let sub_line: Option<(SharedString, Hsla)> = if narrow {
                None
            } else if node.status == GraphNodeStatus::Waiting {
                Some((tr(cx, "Needs you", "待确认").into(), p.amber_ink))
            } else if node.status == GraphNodeStatus::Error {
                Some((tr(cx, "Failed", "失败").into(), p.red_ink))
            } else if node.kind == GraphNodeKind::Agent && !node.detail.is_empty() {
                Some((node.detail.clone(), p.muted))
            } else {
                None
            };

            let nid_click = node.id.clone();
            let nid_hov = node.id.clone();
            let hov_border = if node.status == GraphNodeStatus::Waiting {
                p.amber
            } else {
                p.blue
            };
            let disc_hov_bg = pro_mix_color(disc_bg, p.chip_hover, 0.45);
            let is_sel = self.selected_id.as_ref() == Some(&node.id);

            let disc_btn = pro_tactile(
                pro_eid(&self.id, format!("gnode-{}", node.id)),
                format!("{disp_name}, {}", node.status.label(cx)),
                (side * 0.34).round(),
                disc_bg,
                disc_hov_bg,
                disc_border,
                hov_border,
                is_sel,
                div()
                    .size(px(side))
                    .relative()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(render_node_glyph(node, (side * 0.46).round(), &p, cx))
                    .children(badge),
                cx,
                move |this, _, cx| {
                    this.select_node(Some(nid_click.clone()), cx);
                },
            )
            .w(px(side))
            .h(px(side))
            .shadow_sm()
            .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                let next = if *hovered {
                    Some(nid_hov.clone())
                } else if this.hovered_id.as_ref() == Some(&nid_hov) {
                    None
                } else {
                    this.hovered_id.clone()
                };
                if this.hovered_id != next {
                    this.hovered_id = next;
                    cx.notify();
                }
            }));

            let label_above = ny < 0.23;
            let label_box = div()
                .px(px(5.))
                .py(px(1.))
                .rounded(px(5.))
                .bg(p.panel.opacity(0.92))
                .flex()
                .flex_col()
                .items_center()
                .child(
                    div()
                        .text_size(px(if narrow { 11. } else { 12. }))
                        .line_height(px(15.))
                        .font_weight(if node.kind == GraphNodeKind::Agent {
                            FontWeight::MEDIUM
                        } else {
                            FontWeight::NORMAL
                        })
                        .text_color(if node.kind == GraphNodeKind::Agent {
                            p.strong
                        } else {
                            p.text
                        })
                        .child(disp_name),
                )
                .when_some(sub_line, |d, (txt, col)| {
                    d.child(
                        div()
                            .text_size(px(10.5))
                            .line_height(px(13.))
                            .font_weight(if node.status == GraphNodeStatus::Waiting {
                                FontWeight::MEDIUM
                            } else {
                                FontWeight::NORMAL
                            })
                            .text_color(col)
                            .child(txt),
                    )
                });

            let px_x = nx * container_w;
            let px_y = ny * stage_h;
            let box_w = 128.0_f32;
            let left_pos = (px_x - box_w * 0.5).clamp(2.0, (container_w - box_w - 2.0).max(2.0));
            let top_pos = if label_above {
                (px_y - side * 0.5 - 22.0).max(4.0)
            } else {
                (px_y - side * 0.5).max(4.0)
            };

            node_elements.push(
                div()
                    .absolute()
                    .left(px(left_pos))
                    .top(px(top_pos))
                    .w(px(box_w))
                    .opacity(node_op)
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(3.))
                    .map(|d| {
                        if label_above {
                            d.child(label_box).child(disc_btn)
                        } else {
                            d.child(disc_btn).child(label_box)
                        }
                    })
                    .into_any_element(),
            );
        }

        let legend = div()
            .w_full()
            .pb(px(10.))
            .px(px(12.))
            .flex()
            .flex_wrap()
            .items_center()
            .justify_center()
            .gap(px(16.))
            .text_size(px(11.))
            .text_color(p.muted)
            .child(legend_streak_item(tr(cx, "Call", "发起调用"), p.blue))
            .child(legend_streak_item(tr(cx, "Answer", "返回响应"), p.green))
            .child(legend_streak_item(tr(cx, "Needs you", "待你确认"), p.amber))
            .child(legend_streak_item(tr(cx, "Failed", "调用失败"), p.red));

        let open_dialog = self
            .selected_id
            .clone()
            .or_else(|| self.last_selected_id.clone())
            .and_then(|sid| {
                let n = self.nodes.iter().find(|n| n.id == sid)?;
                Some(self.render_node_dialog(n, &p, cx))
            });

        div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .role(Role::Region)
            .aria_label(title_str)
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, w, cx| {
                if ev.keystroke.key == "escape" && this.selected_id.is_some() {
                    this.select_node(None, cx);
                    this.focus.focus(w, cx);
                }
            }))
            .w_full()
            .p(px(12.))
            .relative()
            .flex()
            .flex_col()
            .child(width_sensor)
            .child(header)
            .child(
                div()
                    .w_full()
                    .rounded(px(10.))
                    .bg(p.panel)
                    .border_1()
                    .border_color(p.line)
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .w_full()
                            .h(px(stage_h))
                            .relative()
                            .child(network_canvas)
                            .children(node_elements),
                    )
                    .child(legend),
            )
            .children(open_dialog)
            .child(pro_toast_banner(self.toast.clone()))
            .into_any_element()
    }
}

fn legend_streak_item(label: &str, color: Hsla) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(5.))
        .child(
            div()
                .w(px(18.))
                .h(px(2.5))
                .rounded_full()
                .bg(linear_gradient(
                    90.,
                    linear_color_stop(transparent_black(), 0.),
                    linear_color_stop(color, 1.),
                )),
        )
        .child(label.to_string())
}

fn render_node_glyph(node: &GraphNode, side: f32, p: &ProPalette, cx: &App) -> AnyElement {
    match node.kind {
        GraphNodeKind::User => pro_icon(ProIcon::User, side, p.on_primary),
        GraphNodeKind::Agent => {
            let key = infer_provider_key(&node.detail);
            if let Some(art) = pro_app_artwork(key, side, cx) {
                art
            } else {
                pro_icon(ProIcon::Sparkles, side, p.blue)
            }
        }
        GraphNodeKind::Tool => {
            let lower = node.name.to_ascii_lowercase();
            if lower.contains("search") || lower.contains("web") {
                pro_icon(ProIcon::Search, side, p.ink)
            } else if lower.contains("database")
                || lower.contains("postgres")
                || lower.contains("db")
            {
                pro_icon(ProIcon::Database, side, p.ink)
            } else if let Some(art) = pro_app_artwork(&node.name, side, cx) {
                art
            } else {
                pro_icon(ProIcon::Terminal, side, p.ink)
            }
        }
    }
}

fn resolve_node_positions(nodes: &[GraphNode]) -> HashMap<SharedString, (f32, f32)> {
    let mut out = HashMap::with_capacity(nodes.len());
    let mut auto_agents = Vec::new();
    let mut auto_outer = Vec::new();
    for n in nodes {
        if let Some(pos) = n.pos {
            out.insert(n.id.clone(), pos);
        } else if n.lead {
            out.insert(n.id.clone(), (0.50, 0.50));
        } else if n.kind == GraphNodeKind::Agent {
            auto_agents.push(n.id.clone());
        } else {
            auto_outer.push(n.id.clone());
        }
    }
    let na = auto_agents.len().max(1) as f32;
    for (i, id) in auto_agents.into_iter().enumerate() {
        let ang = -std::f32::consts::FRAC_PI_4 + (i as f32 / na) * std::f32::consts::TAU;
        out.insert(id, (0.50 + 0.19 * ang.cos(), 0.50 + 0.21 * ang.sin()));
    }
    let no = auto_outer.len().max(1) as f32;
    for (i, id) in auto_outer.into_iter().enumerate() {
        let ang = -std::f32::consts::FRAC_PI_2 + (i as f32 / no) * std::f32::consts::TAU;
        out.insert(id, (0.50 + 0.37 * ang.cos(), 0.50 + 0.38 * ang.sin()));
    }
    out
}

fn build_circuit_polyline(p0: (f32, f32), p1: (f32, f32), radius: f32) -> Vec<(f32, f32)> {
    let dx = p1.0 - p0.0;
    let dy = p1.1 - p0.1;
    if dx.abs() < 2.5 || dy.abs() < 2.5 {
        let mut pts = Vec::with_capacity(17);
        for i in 0..=16 {
            let t = i as f32 / 16.0;
            pts.push((p0.0 + dx * t, p0.1 + dy * t));
        }
        return pts;
    }
    // Single clean L-bend with quadratic corner (vertical first from center, horizontal out)
    let corner = (p0.0, p1.1);
    let r = radius.min(dx.abs() * 0.45).min(dy.abs() * 0.45).max(1.0);
    let sy = dy.signum();
    let sx = dx.signum();
    let b = (p0.0, corner.1 - sy * r);
    let d = (corner.0 + sx * r, p1.1);

    let mut pts = Vec::with_capacity(26);
    for k in 0..=8 {
        let u = k as f32 / 8.0;
        pts.push((p0.0, p0.1 + (b.1 - p0.1) * u));
    }
    for k in 1..=6 {
        let u = k as f32 / 6.0;
        let inv = 1.0 - u;
        pts.push((
            inv * inv * b.0 + 2.0 * inv * u * corner.0 + u * u * d.0,
            inv * inv * b.1 + 2.0 * inv * u * corner.1 + u * u * d.1,
        ));
    }
    for k in 1..=10 {
        let u = k as f32 / 10.0;
        pts.push((d.0 + (p1.0 - d.0) * u, p1.1));
    }
    pts
}

fn paint_polyline_streak(
    pts: &[(f32, f32)],
    u: f32,
    color: Hsla,
    hollow_head: bool,
    card_bg: Hsla,
    window: &mut Window,
) {
    let n = (pts.len() - 1) as f32;
    let head = u * n;
    let tail_span = (n * 0.28).max(3.0);
    for i in 0..(pts.len() - 1) {
        let pos = i as f32 + 0.5;
        let behind = head - pos;
        if behind >= -0.6 && behind <= tail_span {
            let falloff = if behind < 0.0 {
                (1.0 + behind / 0.6).clamp(0.0, 1.0)
            } else {
                (1.0 - behind / tail_span).clamp(0.0, 1.0)
            };
            let alpha = falloff * falloff;
            let mut seg = PathBuilder::stroke(px(2.2 + 1.0 * alpha));
            seg.move_to(point(px(pts[i].0), px(pts[i].1)));
            seg.line_to(point(px(pts[i + 1].0), px(pts[i + 1].1)));
            if let Ok(path) = seg.build() {
                window.paint_path(path, color.opacity(alpha));
            }
        }
    }
    let idx = (head.floor() as usize).min(pts.len() - 2);
    let frac = (head - idx as f32).clamp(0.0, 1.0);
    let hx = pts[idx].0 + (pts[idx + 1].0 - pts[idx].0) * frac;
    let hy = pts[idx].1 + (pts[idx + 1].1 - pts[idx].1) * frac;
    let r = if hollow_head { 3.6 } else { 3.1 };
    window.paint_quad(
        fill(
            Bounds::new(
                point(px(hx - r), px(hy - r)),
                size(px(r * 2.0), px(r * 2.0)),
            ),
            color,
        )
        .corner_radii(px(r)),
    );
    if hollow_head {
        let ir = 1.8;
        window.paint_quad(
            fill(
                Bounds::new(
                    point(px(hx - ir), px(hy - ir)),
                    size(px(ir * 2.0), px(ir * 2.0)),
                ),
                card_bg,
            )
            .corner_radii(px(ir)),
        );
    }
}

fn canonical_edge_key(a: &SharedString, b: &SharedString) -> (SharedString, SharedString) {
    if a.as_ref() <= b.as_ref() {
        (a.clone(), b.clone())
    } else {
        (b.clone(), a.clone())
    }
}

fn infer_provider_key(detail: &str) -> &'static str {
    let lower = detail.to_ascii_lowercase();
    if lower.contains("opus") || lower.contains("sonnet") || lower.contains("claude") {
        "claude"
    } else if lower.contains("gemini") {
        "gemini"
    } else {
        "openai"
    }
}

fn tr_graph_name<'a>(id: &str, fallback: &'a str, cx: &App) -> &'a str {
    match id {
        "lead" => tr(cx, "Orchestrator", "主控编排器"),
        "research" => tr(cx, "Researcher", "调研智能体"),
        "analyst" => tr(cx, "Analyst", "分析智能体"),
        "coder" => tr(cx, "Coder", "编码智能体"),
        "writer" => tr(cx, "Writer", "文案智能体"),
        "web" => tr(cx, "Web search", "网页检索"),
        "db" => tr(cx, "Database", "数据库"),
        "you" => tr(cx, "You", "你"),
        _ => fallback,
    }
}

fn default_graph_topology() -> (
    Vec<GraphNode>,
    Vec<(SharedString, SharedString)>,
    Vec<GraphCall>,
) {
    let nodes =
        vec![
        GraphNode::new(
            "lead",
            "Orchestrator",
            GraphNodeKind::Agent,
            GraphNodeStatus::Working,
        )
        .as_lead()
        .with_short("Lead")
        .with_detail("Opus 5.5")
        .with_doing("Coordinating four specialist agents across the pricing launch.")
        .with_cost(0.08)
        .with_pos(0.50, 0.50),
        GraphNode::new(
            "research",
            "Researcher",
            GraphNodeKind::Agent,
            GraphNodeStatus::Working,
        )
        .with_short("Research")
        .with_detail("Sonnet 5.5")
        .with_doing("Reading the pricing pages of five competitors.")
        .with_cost(0.04)
        .with_pos(0.64, 0.35),
        GraphNode::new(
            "analyst",
            "Analyst",
            GraphNodeKind::Agent,
            GraphNodeStatus::Working,
        )
        .with_short("Analyst")
        .with_detail("Gemini 3.8 Pro")
        .with_doing("Counting seats and upgrades per plan in warehouse.")
        .with_cost(0.03)
        .with_pos(0.64, 0.66),
        GraphNode::new(
            "coder",
            "Coder",
            GraphNodeKind::Agent,
            GraphNodeStatus::Working,
        )
        .with_short("Coder")
        .with_detail("GPT-6 Sol")
        .with_doing("Building the pricing page on a feature branch.")
        .with_cost(0.03)
        .with_pos(0.36, 0.66),
        GraphNode::new(
            "writer",
            "Writer",
            GraphNodeKind::Agent,
            GraphNodeStatus::Waiting,
        )
        .with_short("Writer")
        .with_detail("Opus 5.5")
        .with_doing("Drafting enterprise tier SLA & security copy.")
        .with_ask(
            "Approve publishing the 99.99% enterprise SLA commitment to the Notion launch spec?",
        )
        .with_cost(0.02)
        .with_pos(0.36, 0.35),
        GraphNode::new("web", "Web search", GraphNodeKind::Tool, GraphNodeStatus::Idle)
            .with_short("Search")
            .with_detail("Search tool")
            .with_doing("Live web indexing & competitor page extraction.")
            .with_pos(0.85, 0.35),
        GraphNode::new("notion", "Notion", GraphNodeKind::Tool, GraphNodeStatus::Idle)
            .with_detail("Workspace docs")
            .with_doing("Launch spec & tier matrix documents.")
            .with_pos(0.64, 0.14),
        GraphNode::new("hubspot", "HubSpot", GraphNodeKind::Tool, GraphNodeStatus::Idle)
            .with_detail("CRM")
            .with_doing("Enterprise pipeline & deal stage lookups.")
            .with_pos(0.85, 0.66),
        GraphNode::new("db", "Database", GraphNodeKind::Tool, GraphNodeStatus::Idle)
            .with_short("DB")
            .with_detail("Analytics SQL")
            .with_doing("Read-only billing & seat telemetry replica.")
            .with_pos(0.64, 0.86),
        GraphNode::new("github", "GitHub", GraphNodeKind::Tool, GraphNodeStatus::Idle)
            .with_detail("Repository")
            .with_doing("Pull requests, checks, and preview deploys.")
            .with_pos(0.28, 0.84),
        GraphNode::new("slack", "Slack", GraphNodeKind::Tool, GraphNodeStatus::Idle)
            .with_detail("#launch-pricing")
            .with_doing("Team release channel notifications.")
            .with_pos(0.11, 0.35),
        GraphNode::new("you", "You", GraphNodeKind::User, GraphNodeStatus::Idle)
            .with_detail("Operator")
            .with_doing("Requested the pricing page launch and approves external commits.")
            .with_pos(0.28, 0.16),
    ];

    let edges = vec![
        ("you".into(), "lead".into()),
        ("lead".into(), "research".into()),
        ("lead".into(), "analyst".into()),
        ("lead".into(), "coder".into()),
        ("lead".into(), "writer".into()),
        ("lead".into(), "slack".into()),
        ("research".into(), "web".into()),
        ("research".into(), "notion".into()),
        ("analyst".into(), "hubspot".into()),
        ("analyst".into(), "db".into()),
        ("coder".into(), "github".into()),
        ("writer".into(), "notion".into()),
        ("writer".into(), "slack".into()),
        ("writer".into(), "you".into()),
    ];

    let calls = vec![
        GraphCall::new(
            "c1",
            "you",
            "lead",
            GraphCallStatus::Ok,
            "Launch pricing page",
            Some(120),
        ),
        GraphCall::new(
            "c2",
            "lead",
            "research",
            GraphCallStatus::Ok,
            "Compare 5 competitors",
            Some(340),
        ),
        GraphCall::new(
            "c3",
            "research",
            "web",
            GraphCallStatus::Ok,
            "pricing linear.app",
            Some(820),
        ),
        GraphCall::new(
            "c4",
            "research",
            "web",
            GraphCallStatus::Ok,
            "pricing vercel.com",
            Some(760),
        ),
        GraphCall::new(
            "c5",
            "lead",
            "analyst",
            GraphCallStatus::Ok,
            "Count seats per tier",
            Some(280),
        ),
        GraphCall::new(
            "c6",
            "analyst",
            "db",
            GraphCallStatus::Ok,
            "SELECT seats, plan",
            Some(195),
        ),
        GraphCall::new(
            "c7",
            "lead",
            "coder",
            GraphCallStatus::Ok,
            "Scaffold pricing grid",
            Some(310),
        ),
        GraphCall::new(
            "c8",
            "coder",
            "github",
            GraphCallStatus::Ok,
            "open PR #418",
            Some(450),
        ),
        GraphCall::new(
            "c9",
            "writer",
            "you",
            GraphCallStatus::NeedsYou,
            "Approve enterprise SLA?",
            None,
        ),
    ];

    (nodes, edges, calls)
}

#[cfg(test)]
mod tests {
    use gpui::AppContext;

    use super::{AgentGraph, GraphCall, GraphCallStatus, GraphNodeStatus};
    use gpui::TestAppContext;

    #[test]
    fn default_graph_contains_five_agents_six_tools_and_amber_waiting_node() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let graph = app.new(|cx| AgentGraph::new("graph-test", cx));
        graph.read_with(&app, |g, _| {
            assert_eq!(g.nodes.len(), 12);
            assert_eq!(g.calls_per_minute(), 9);
            assert!((g.total_cost() - 0.20).abs() < 1e-6);
            let writer = g.nodes.iter().find(|n| n.id.as_ref() == "writer");
            assert_eq!(writer.map(|n| n.status), Some(GraphNodeStatus::Waiting));
        });
    }

    #[test]
    fn recording_calls_and_approving_amber_node_update_state_cleanly() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let graph = app.new(|cx| AgentGraph::new("graph-mut", cx));
        graph.update(&mut app, |g, cx| {
            g.approve_node("writer", true, cx);
            let writer = g
                .nodes
                .iter()
                .find(|n| n.id.as_ref() == "writer")
                .expect("writer exists");
            assert_eq!(writer.status, GraphNodeStatus::Working);
            assert!(writer.ask.is_none());

            g.record_call(
                GraphCall::new(
                    "c10",
                    "writer",
                    "notion",
                    GraphCallStatus::Ok,
                    "Publish SLA section",
                    Some(290),
                ),
                cx,
            );
            assert_eq!(g.calls_per_minute(), 10);
        });
    }
}
