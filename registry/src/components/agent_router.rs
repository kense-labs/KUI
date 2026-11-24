//! Incoming request router sorting work across specialist agents and a human fallback.
//!
//! Renders an incoming request card above a central router agent and side-by-side specialist
//! queues on soft blue connectors, animating routing pulses, confidence scores, and manual reroutes.

use crate::{
    components::pro_chrome::{
        ProBtnStyle, ProPalette, pro_agent_avatar, pro_btn, pro_check_draw, pro_connector_pulse,
        pro_ease, pro_eid, pro_empty_state, pro_icon_btn, pro_mix_color, pro_presence,
        pro_rolling_number, pro_shimmer_text, pro_skeleton_bar, pro_spring_bar, pro_spring_f32,
        pro_swap_label, pro_tactile, pro_toast_banner, pro_view_transition, tr,
    },
    foundation::Theme,
    primitives::pro_icons::{ProIcon, pro_app_artwork, pro_icon, pro_svg},
};
use gpui::{prelude::*, *};
use std::{cell::Cell, collections::HashMap, rc::Rc};
use web_time::Instant;

const CORNER_ARROW_SVG: &str = r#"<path d="M5 5V10.5C5 12.433 6.567 14 8.5 14H19M19 14L15 10M19 14L15 18" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8"/>"#;
const SPIN_ARC_SVG: &str = r#"<circle cx="12" cy="12" r="8.5" stroke="currentColor" stroke-opacity="0.25" stroke-width="2.5"/><path d="M12 3.5A8.5 8.5 0 0 1 20.5 12" stroke="currentColor" stroke-linecap="round" stroke-width="2.5"/>"#;

/// Source channel of an incoming [`RouterRequest`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RouterChannel {
    /// Email message.
    #[default]
    Email,
    /// Live chat conversation.
    Chat,
    /// Support ticket submission.
    Ticket,
    /// Slack channel mention.
    Slack,
}

impl RouterChannel {
    fn icon(self) -> ProIcon {
        match self {
            Self::Email => ProIcon::Mail,
            Self::Chat => ProIcon::Thread,
            Self::Ticket => ProIcon::ClipboardDoc,
            Self::Slack => ProIcon::Inbox,
        }
    }
}

/// Lifecycle state of a [`RouterRequest`] inside [`AgentRouter`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RouterRequestStatus {
    /// Newly arrived and awaiting router inspection.
    New,
    /// Currently being classified by the router agent.
    #[default]
    Routing,
    /// Dispatched to a specialist agent's queue.
    Routed,
    /// Escalated to the human fallback due to low confidence or policy.
    Escalated,
    /// Resolved by its assigned specialist.
    Done,
}

impl RouterRequestStatus {
    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::New => tr(cx, "New", "新请求"),
            Self::Routing => tr(cx, "Routing", "路由判定中"),
            Self::Routed => tr(cx, "Routed", "已分发"),
            Self::Escalated => tr(cx, "Escalated", "已转交人工"),
            Self::Done => tr(cx, "Done", "已完成"),
        }
    }
}

/// Router hub, specialist target, or human fallback endpoint in [`AgentRouter`].
#[derive(Clone, Debug, PartialEq)]
pub struct RouterAgent {
    /// Stable endpoint identifier (for example `"billing"`, `"tech"`, `"human"`).
    pub id: SharedString,
    /// Display name shown on the card or fallback tag.
    pub name: SharedString,
    /// Model label (for example `"Claude Sonnet"`, `"GPT-5"`, or `"Human"`).
    pub model: SharedString,
    /// One-line current activity shown on the specialist card.
    pub activity: SharedString,
    /// Current load ratio (`0.0..=1.0`) rendered as a thin meter on specialist cards.
    pub load: f32,
    /// Whether this target represents the off-to-the-right `Human` fallback tag.
    pub human: bool,
    /// Optional custom accent color.
    pub color: Option<Hsla>,
}

impl RouterAgent {
    /// Construct a specialist or router [`RouterAgent`].
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        model: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            model: model.into(),
            activity: "Waiting for work".into(),
            load: 0.25,
            human: false,
            color: None,
        }
    }

    /// Construct a `Human` fallback endpoint rendered off the right side of the router.
    pub fn human_fallback(id: impl Into<SharedString>, name: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            model: "Human".into(),
            activity: "Escalation desk".into(),
            load: 0.0,
            human: true,
            color: None,
        }
    }

    /// Set the present-tense activity line.
    pub fn with_activity(mut self, activity: impl Into<SharedString>) -> Self {
        self.activity = activity.into();
        self
    }

    /// Set current queue load (`0.0..=1.0`).
    pub fn with_load(mut self, load: f32) -> Self {
        self.load = if load.is_finite() {
            load.clamp(0.0, 1.0)
        } else {
            0.0
        };
        self
    }

    /// Override the specialist's accent color.
    pub fn with_color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }
}

/// Incoming or queued customer request sorted by [`AgentRouter`].
#[derive(Clone, Debug, PartialEq)]
pub struct RouterRequest {
    /// Stable request identifier.
    pub id: SharedString,
    /// Short subject line (for example `"Refund for order #4821"`).
    pub title: SharedString,
    /// Sender handle or email address.
    pub from: SharedString,
    /// Origin channel icon.
    pub channel: RouterChannel,
    /// First-line message preview.
    pub preview: SharedString,
    /// Current routing lifecycle status.
    pub status: RouterRequestStatus,
    /// Assigned specialist or human fallback id.
    pub to: Option<SharedString>,
    /// Router confidence score in `0.0..=1.0`.
    pub confidence: f32,
    /// Per-target classification scores `(agent_id, score)`.
    pub scores: Vec<(SharedString, f32)>,
    /// Explanation of why the router chose `to`.
    pub reason: SharedString,
    /// Up to three compact topic tags shown in task queues.
    pub tags: Vec<SharedString>,
}

impl RouterRequest {
    /// Construct a [`RouterRequest`] with `id`, `title`, `from`, and `preview`.
    pub fn new(
        id: impl Into<SharedString>,
        title: impl Into<SharedString>,
        from: impl Into<SharedString>,
        preview: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            from: from.into(),
            channel: RouterChannel::Email,
            preview: preview.into(),
            status: RouterRequestStatus::Routing,
            to: None,
            confidence: 0.90,
            scores: Vec::new(),
            reason: SharedString::default(),
            tags: Vec::new(),
        }
    }

    /// Set the ingress [`RouterChannel`].
    pub fn with_channel(mut self, channel: RouterChannel) -> Self {
        self.channel = channel;
        self
    }

    /// Set routing status, target `to`, and `confidence`.
    pub fn with_routing(
        mut self,
        status: RouterRequestStatus,
        to: Option<&str>,
        confidence: f32,
    ) -> Self {
        self.status = status;
        self.to = to.map(Into::into);
        self.confidence = if confidence.is_finite() {
            confidence.clamp(0.0, 1.0)
        } else {
            0.0
        };
        self
    }

    /// Set per-agent candidate scores `(agent_id, score)`.
    pub fn with_scores(mut self, scores: Vec<(&str, f32)>) -> Self {
        self.scores = scores
            .into_iter()
            .map(|(k, v)| (k.into(), v.clamp(0.0, 1.0)))
            .collect();
        self
    }

    /// Set the router rationale displayed in the score breakdown dialog.
    pub fn with_reason(mut self, reason: impl Into<SharedString>) -> Self {
        self.reason = reason.into();
        self
    }

    /// Attach up to three topic tags rendered in the queue card.
    pub fn with_tags(mut self, tags: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.tags = tags.into_iter().take(3).map(Into::into).collect();
        self
    }
}

/// Typed events emitted by [`AgentRouter`].
#[derive(Clone, Debug, PartialEq)]
pub enum AgentRouterEvent {
    /// User clicked a request card to inspect its per-agent score breakdown.
    RequestSelected {
        /// Request identifier.
        request_id: SharedString,
    },
    /// Router finished dispatching an incoming request to `target_id`.
    RequestDispatched {
        /// Request identifier.
        request_id: SharedString,
        /// Target specialist or human fallback identifier.
        target_id: SharedString,
    },
    /// Operator manually rerouted a request to a different target.
    Rerouted {
        /// Request identifier.
        request_id: SharedString,
        /// Newly assigned specialist or human fallback identifier.
        target_id: SharedString,
    },
}

impl EventEmitter<AgentRouterEvent> for AgentRouter {}

#[derive(Clone, Debug)]
struct RoutingFlight {
    req_id: SharedString,
    target_id: SharedString,
    two_stage: bool,
    started: Instant,
}

/// Live request router sorting incoming items across specialist queues and a human fallback.
///
/// # Examples
///
/// ```ignore
/// use kui::components::agent_router::AgentRouter;
///
/// let router = cx.new(|cx| AgentRouter::new("support-router", cx));
/// ```
pub struct AgentRouter {
    id: ElementId,
    focus: FocusHandle,
    title: Option<SharedString>,
    router: RouterAgent,
    agents: Vec<RouterAgent>,
    requests: Vec<RouterRequest>,
    custom_data: bool,
    active_incoming_id: Option<SharedString>,
    flight: Option<RoutingFlight>,
    arrival_glows: HashMap<SharedString, Instant>,
    filter_agent_id: Option<SharedString>,
    hovered_agent_id: Option<SharedString>,
    hovered_req_target: Option<SharedString>,
    inspect_req_id: Option<SharedString>,
    last_inspect_req_id: Option<SharedString>,
    sample_cursor: usize,
    sr_status: SharedString,
    toast: Option<(SharedString, Instant)>,
    loading: bool,
    empty: bool,
    boot: Instant,
    measured_width: Rc<Cell<f32>>,
}

impl AgentRouter {
    /// Create a live [`AgentRouter`] initialized with `"Support inbox"`, 3 specialist queues, and a `Human` fallback.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let now = Instant::now();
        let (router, agents, requests) = default_router_setup();
        let incoming = requests
            .iter()
            .find(|r| r.status == RouterRequestStatus::Routing)
            .map(|r| r.id.clone());
        let flight = incoming.as_ref().map(|rid| RoutingFlight {
            req_id: rid.clone(),
            target_id: "billing".into(),
            two_stage: true,
            started: now,
        });
        Self {
            id: id.into(),
            focus: cx.focus_handle(),
            title: None,
            router,
            agents,
            requests,
            custom_data: false,
            active_incoming_id: incoming,
            flight,
            arrival_glows: HashMap::new(),
            filter_agent_id: None,
            hovered_agent_id: None,
            hovered_req_target: None,
            inspect_req_id: None,
            last_inspect_req_id: None,
            sample_cursor: 0,
            sr_status: "Routing Refund for order #4821 to Billing (94% sure)".into(),
            toast: None,
            loading: false,
            empty: false,
            boot: now,
            measured_width: Rc::new(Cell::new(780.0)),
        }
    }

    /// Override the header title (defaults to `"Support inbox"` / `"智能客服分流路由"`).
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Replace the central router agent metadata.
    pub fn with_router(mut self, router: RouterAgent) -> Self {
        self.router = router;
        self.custom_data = true;
        self
    }

    /// Replace the specialist agents and optional `Human` fallback.
    pub fn with_agents(mut self, agents: Vec<RouterAgent>) -> Self {
        self.custom_data = true;
        if agents.is_empty() {
            self.agents.clear();
            self.empty = true;
        } else {
            self.agents = agents;
        }
        self.flight = None;
        self
    }

    /// Replace the incoming and queued requests.
    pub fn with_requests(mut self, requests: Vec<RouterRequest>) -> Self {
        self.custom_data = true;
        self.active_incoming_id = requests
            .iter()
            .find(|r| {
                matches!(
                    r.status,
                    RouterRequestStatus::Routing | RouterRequestStatus::New
                )
            })
            .or_else(|| requests.first())
            .map(|r| r.id.clone());
        self.requests = requests;
        self.flight = None;
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

    /// Return how many requests are currently routed to `agent_id`.
    pub fn queue_len(&self, agent_id: &str) -> usize {
        self.requests
            .iter()
            .filter(|r| {
                matches!(
                    r.status,
                    RouterRequestStatus::Routed | RouterRequestStatus::Escalated
                ) && r.to.as_deref() == Some(agent_id)
            })
            .count()
    }

    /// Manually reroute `request_id` to `target_id`, running a fresh connector pulse into that target.
    pub fn reroute_request(
        &mut self,
        request_id: impl Into<SharedString>,
        target_id: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) {
        let rid = request_id.into();
        let tid = target_id.into();
        let is_human = self
            .agents
            .iter()
            .find(|a| a.id == tid)
            .map(|a| a.human)
            .unwrap_or(false);
        let target_name = self
            .agents
            .iter()
            .find(|a| a.id == tid)
            .map(|a| a.name.clone())
            .unwrap_or_else(|| tid.clone());

        if let Some(req) = self.requests.iter_mut().find(|r| r.id == rid) {
            req.to = Some(tid.clone());
            req.status = if is_human {
                RouterRequestStatus::Escalated
            } else {
                RouterRequestStatus::Routed
            };
        }
        self.inspect_req_id = None;
        if !Theme::global(cx).reduced_motion {
            self.flight = Some(RoutingFlight {
                req_id: rid.clone(),
                target_id: tid.clone(),
                two_stage: false,
                started: Instant::now(),
            });
            self.arrival_glows.insert(tid.clone(), Instant::now());
        }
        let msg = format!("{} {target_name}", tr(cx, "Rerouted to", "已改派至"));
        self.sr_status = msg.clone().into();
        self.toast = Some((msg.into(), Instant::now()));
        cx.emit(AgentRouterEvent::Rerouted {
            request_id: rid,
            target_id: tid,
        });
        cx.notify();
    }

    /// Inject the next realistic incoming request (cycling specialist & low-confidence human escalations).
    pub fn simulate_incoming(&mut self, cx: &mut Context<Self>) {
        let idx = self.sample_cursor;
        self.sample_cursor = self.sample_cursor.wrapping_add(1);
        let samples = [
            RouterRequest::new(
                format!("sim-{idx}"),
                "Custom MSA & SOC2 redlines",
                "legal@acme-corp.io",
                "Can our counsel review clause 9.2 before we sign 120 annual seats?",
            )
            .with_channel(RouterChannel::Email)
            .with_routing(RouterRequestStatus::Routing, Some("human"), 0.41)
            .with_scores(vec![("sales", 0.41), ("billing", 0.19), ("tech", 0.08)])
            .with_reason("Low confidence (41%): custom legal redlines require a human lead.")
            .with_tags(["legal", "enterprise"]),
            RouterRequest::new(
                format!("sim-{idx}"),
                "Webhook 502 on eu-central-1",
                "dev@LinearFlow.dev",
                "Events from invoice.paid started timing out after rotating TLS certs.",
            )
            .with_channel(RouterChannel::Slack)
            .with_routing(RouterRequestStatus::Routing, Some("tech"), 0.96)
            .with_scores(vec![("tech", 0.96), ("billing", 0.14), ("sales", 0.03)])
            .with_reason("TLS webhook 502 error signature matches Tech support runbooks.")
            .with_tags(["webhooks", "tls"]),
            RouterRequest::new(
                format!("sim-{idx}"),
                "Upgrade 35 seats to Enterprise",
                "vp-eng@meridian.co",
                "We need SAML SSO and annual invoicing for 35 engineers starting Nov 1.",
            )
            .with_channel(RouterChannel::Chat)
            .with_routing(RouterRequestStatus::Routing, Some("sales"), 0.92)
            .with_scores(vec![("sales", 0.92), ("billing", 0.31), ("tech", 0.11)])
            .with_reason("Seat expansion + SAML enterprise inquiry — routed to Sales.")
            .with_tags(["expansion", "sso"]),
        ];
        let mut next_req = samples[idx % samples.len()].clone();
        let fallback_target = self
            .agents
            .first()
            .map(|a| a.id.clone())
            .unwrap_or_else(|| "billing".into());
        let target_id = next_req
            .to
            .clone()
            .filter(|tid| self.agents.iter().any(|a| &a.id == tid))
            .unwrap_or(fallback_target);
        next_req.to = Some(target_id.clone());

        let rid = next_req.id.clone();
        self.router.activity = format!("Reading “{}”", next_req.title).into();
        if Theme::global(cx).reduced_motion {
            let is_human = self
                .agents
                .iter()
                .find(|a| a.id == target_id)
                .map(|a| a.human)
                .unwrap_or(false);
            next_req.status = if is_human {
                RouterRequestStatus::Escalated
            } else {
                RouterRequestStatus::Routed
            };
            self.active_incoming_id = Some(rid.clone());
            self.requests.insert(0, next_req);
            cx.emit(AgentRouterEvent::RequestDispatched {
                request_id: rid,
                target_id,
            });
        } else {
            self.active_incoming_id = Some(rid.clone());
            self.requests.insert(0, next_req);
            self.flight = Some(RoutingFlight {
                req_id: rid,
                target_id,
                two_stage: true,
                started: Instant::now(),
            });
        }
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

    /// Return whether the router is empty.
    pub fn is_empty(&self) -> bool {
        self.empty || self.agents.is_empty()
    }

    fn advance_flight(&mut self, reduced: bool, cx: &mut Context<Self>) -> bool {
        let now = Instant::now();
        self.arrival_glows
            .retain(|_, t| now.duration_since(*t).as_secs_f32() < 0.95);

        let mut finished: Option<(SharedString, SharedString)> = None;
        if let Some(fl) = &self.flight {
            let dur = if fl.two_stage { 1.45 } else { 0.72 };
            if reduced || now.duration_since(fl.started).as_secs_f32() >= dur {
                finished = Some((fl.req_id.clone(), fl.target_id.clone()));
            }
        }

        if let Some((rid, tid)) = finished {
            self.flight = None;
            let is_human = self
                .agents
                .iter()
                .find(|a| a.id == tid)
                .map(|a| a.human)
                .unwrap_or(false);
            if let Some(req) = self.requests.iter_mut().find(|r| r.id == rid)
                && req.status == RouterRequestStatus::Routing
            {
                req.status = if is_human {
                    RouterRequestStatus::Escalated
                } else {
                    RouterRequestStatus::Routed
                };
                if let Some(ag) = self.agents.iter_mut().find(|a| a.id == tid) {
                    ag.activity = req.title.clone();
                }
                self.router.activity =
                    format!("Routed “{}” ({:.0}%)", req.title, req.confidence * 100.0).into();
                if !reduced {
                    self.arrival_glows.insert(tid.clone(), now);
                }
                cx.emit(AgentRouterEvent::RequestDispatched {
                    request_id: rid,
                    target_id: tid,
                });
            }
        }

        !reduced && (self.flight.is_some() || !self.arrival_glows.is_empty())
    }

    fn specialist_color(&self, idx: usize, agent: &RouterAgent, p: &ProPalette) -> Hsla {
        if let Some(c) = agent.color {
            return c;
        }
        if agent.human {
            return p.orange;
        }
        let cols = [p.purple, p.cyan, p.green, p.pink, p.teal];
        cols[idx % cols.len()]
    }

    fn render_score_modal(
        &self,
        req: &RouterRequest,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let rid = req.id.clone();
        let conf_pct = (req.confidence * 100.0).round() as u32;

        let close_btn = pro_icon_btn(
            pro_eid(&self.id, "r-close-dlg"),
            tr(cx, "Close routing breakdown", "关闭路由评分详情"),
            ProIcon::Close,
            24.,
            false,
            cx,
            |this, w, cx| {
                this.inspect_req_id = None;
                this.focus.focus(w, cx);
                cx.notify();
            },
        );

        let mut candidates = div()
            .w_full()
            .pt(px(8.))
            .border_t_1()
            .border_b_1()
            .border_color(p.line)
            .pb(px(8.))
            .flex()
            .flex_col()
            .gap(px(6.));

        for (ai, ag) in self.agents.iter().enumerate() {
            let acol = self.specialist_color(ai, ag, p);
            let score = req
                .scores
                .iter()
                .find(|(id, _)| id == &ag.id)
                .map(|(_, s)| *s)
                .unwrap_or_else(|| {
                    if req.to.as_ref() == Some(&ag.id) {
                        req.confidence
                    } else if ag.human {
                        (1.0 - req.confidence).clamp(0.05, 0.85)
                    } else {
                        0.08
                    }
                });
            let is_current = req.to.as_ref() == Some(&ag.id);
            let req_c = rid.clone();
            let target_c = ag.id.clone();

            candidates = candidates.child(pro_tactile(
                pro_eid(&self.id, format!("r-cand-{ai}")),
                format!("{} {:.0}%", ag.name, score * 100.0),
                7.,
                if is_current { p.select } else { p.subtle },
                p.hover,
                if is_current {
                    p.line_strong
                } else {
                    transparent_black()
                },
                p.line_strong,
                is_current,
                div()
                    .w_full()
                    .p(px(7.))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(if ag.human {
                        div()
                            .size(px(22.))
                            .rounded(px(6.))
                            .bg(p.primary)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(pro_icon(ProIcon::User, 12., p.on_primary))
                            .into_any_element()
                    } else {
                        pro_agent_avatar(&ag.id, acol, 22., false, 0., cx)
                    })
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap(px(3.))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .gap(px(6.))
                                    .child(
                                        div()
                                            .text_size(px(12.5))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.strong)
                                            .child(ag.name.clone()),
                                    )
                                    .child(pro_rolling_number(
                                        pro_eid(&self.id, format!("sc-pct-{}-{ai}", req.id)),
                                        f64::from(score * 100.0),
                                        0,
                                        "",
                                        "%",
                                        11.5,
                                        FontWeight::MEDIUM,
                                        if is_current { p.ink } else { p.muted },
                                    )),
                            )
                            .child(pro_spring_bar(
                                pro_eid(&self.id, format!("sc-bar-{}-{ai}", req.id)),
                                score,
                                3.0,
                                2.0,
                                p.chip,
                                acol,
                            )),
                    )
                    .child(
                        div()
                            .h(px(22.))
                            .px(px(7.))
                            .rounded(px(5.))
                            .bg(if is_current { p.chip } else { p.card })
                            .border_1()
                            .border_color(p.line)
                            .flex()
                            .items_center()
                            .gap(px(4.))
                            .when(is_current, |d| {
                                d.child(pro_check_draw(
                                    pro_eid(&self.id, format!("sc-chk-{}-{ai}", req.id)),
                                    true,
                                    12.,
                                    false,
                                    p.green,
                                ))
                            })
                            .child(pro_swap_label(
                                pro_eid(&self.id, format!("sc-lbl-{}-{ai}", req.id)),
                                if is_current {
                                    tr(cx, "Assigned", "当前分配")
                                } else {
                                    tr(cx, "Reroute", "改派")
                                },
                                11.,
                                FontWeight::MEDIUM,
                                if is_current { p.muted } else { p.ink },
                            )),
                    ),
                cx,
                move |this, _, cx| {
                    this.reroute_request(req_c.clone(), target_c.clone(), cx);
                },
            ));
        }

        let dialog = div()
            .id(pro_eid(&self.id, "r-score-dlg"))
            .role(Role::Dialog)
            .aria_label(req.title.clone())
            .on_click(|_, _, cx| cx.stop_propagation())
            .w(px(356.))
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
                    .child(
                        div()
                            .size(px(24.))
                            .rounded(px(6.))
                            .bg(p.chip)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(pro_icon(req.channel.icon(), 13., p.ink)),
                    )
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
                                    .child(req.title.clone()),
                            )
                            .child(
                                div()
                                    .text_size(px(11.5))
                                    .text_color(p.muted)
                                    .truncate()
                                    .child(format!(
                                        "{} · {conf_pct}% {}",
                                        req.from,
                                        tr(cx, "sure", "置信度")
                                    )),
                            ),
                    )
                    .child(close_btn),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .line_height(px(17.))
                    .text_color(p.text)
                    .child(req.preview.clone()),
            )
            .when(!req.reason.is_empty(), |d| {
                d.child(
                    div()
                        .p(px(7.))
                        .rounded(px(6.))
                        .bg(p.accent_soft)
                        .text_size(px(11.5))
                        .line_height(px(16.))
                        .text_color(p.blue)
                        .child(req.reason.clone()),
                )
            })
            .child(candidates);

        let dialog_body = pro_view_transition(pro_eid(&self.id, "r-modal-vtx"), rid, dialog);
        let dark = p.dark;
        let scrim_id = pro_eid(&self.id, "r-scrim");
        let on_scrim = cx.listener(|this, _: &ClickEvent, w, cx| {
            this.inspect_req_id = None;
            this.focus.focus(w, cx);
            cx.notify();
        });
        pro_presence(
            pro_eid(&self.id, "r-modal-pres"),
            self.inspect_req_id.is_some(),
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
                            .child(dialog_body),
                    )
                    .into_any_element()
            },
        )
    }
}

impl Render for AgentRouter {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let reduced = Theme::global(cx).reduced_motion;
        if self.advance_flight(reduced, cx) {
            w.request_animation_frame();
        }
        let elapsed_ms = self.boot.elapsed().as_secs_f32() * 1000.0;

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
                .p(px(14.))
                .flex()
                .flex_col()
                .gap(px(18.))
                .child(width_sensor)
                .child(
                    div()
                        .w_full()
                        .flex()
                        .justify_between()
                        .child(pro_skeleton_bar(140., 18., 5., phase, cx))
                        .child(pro_skeleton_bar(160., 16., 5., phase, cx)),
                )
                .child(
                    div()
                        .w_full()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap(px(28.))
                        .child(pro_skeleton_bar(236., 92., 10., phase, cx))
                        .child(pro_skeleton_bar(236., 110., 10., phase, cx)),
                )
                .child(
                    div()
                        .w_full()
                        .flex()
                        .justify_center()
                        .gap(px(14.))
                        .child(pro_skeleton_bar(236., 240., 10., phase, cx))
                        .child(pro_skeleton_bar(236., 240., 10., phase, cx))
                        .child(pro_skeleton_bar(236., 240., 10., phase, cx)),
                )
                .into_any_element();
        }

        if self.empty || self.agents.is_empty() {
            return div()
                .id(self.id.clone())
                .w_full()
                .child(width_sensor)
                .child(pro_empty_state(
                    ProIcon::Inbox,
                    tr(cx, "No specialist routes configured", "未配置分流目标智能体"),
                    tr(
                        cx,
                        "Connect specialist agents and an optional Human fallback to sort incoming requests live.",
                        "配置专项处理智能体与人工兜底通道后，即可实时展示置信度评分与分流队列。",
                    ),
                    None,
                    cx,
                ))
                .into_any_element();
        }

        let container_w = self.measured_width.get();
        let wide = container_w >= 700.0;

        let routing_cnt = self
            .requests
            .iter()
            .filter(|r| r.status == RouterRequestStatus::Routing)
            .count();
        let escalated_cnt = self
            .requests
            .iter()
            .filter(|r| r.status == RouterRequestStatus::Escalated)
            .count();
        let routed_cnt = self
            .requests
            .iter()
            .filter(|r| r.status == RouterRequestStatus::Routed)
            .count();

        let title_str: SharedString = self
            .title
            .clone()
            .unwrap_or_else(|| tr(cx, "Support inbox", "智能客服分流台").into());

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
                    .gap(px(10.))
                    .child(
                        div()
                            .text_size(px(13.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .child(title_str.clone()),
                    )
                    .child(pro_btn(
                        pro_eid(&self.id, "r-sim-btn"),
                        tr(cx, "+ New request", "+ 模拟新请求"),
                        Some(ProIcon::Plus),
                        ProBtnStyle::Secondary,
                        true,
                        cx,
                        |this, _, cx| {
                            this.simulate_incoming(cx);
                        },
                    )),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(10.))
                    .text_size(px(12.))
                    .text_color(p.muted)
                    .when(routing_cnt > 0, |d| {
                        d.child(pro_rolling_number(
                            pro_eid(&self.id, "r-hdr-rtg"),
                            routing_cnt as f64,
                            0,
                            "",
                            format!(" {}", tr(cx, "routing", "判定中")),
                            12.,
                            FontWeight::MEDIUM,
                            p.blue,
                        ))
                    })
                    .when(escalated_cnt > 0, |d| {
                        d.child(pro_rolling_number(
                            pro_eid(&self.id, "r-hdr-esc"),
                            escalated_cnt as f64,
                            0,
                            "",
                            format!(" {}", tr(cx, "needs human", "人工待办")),
                            12.,
                            FontWeight::MEDIUM,
                            p.amber_ink,
                        ))
                    })
                    .child(pro_rolling_number(
                        pro_eid(&self.id, "r-hdr-rtd"),
                        routed_cnt as f64,
                        0,
                        "",
                        format!(" {}", tr(cx, "routed", "已分发")),
                        12.,
                        FontWeight::NORMAL,
                        p.muted,
                    )),
            );

        let focus_target = self
            .hovered_agent_id
            .clone()
            .or_else(|| self.hovered_req_target.clone())
            .or_else(|| self.filter_agent_id.clone());

        let incoming_req = self
            .active_incoming_id
            .as_ref()
            .and_then(|id| self.requests.iter().find(|r| &r.id == id))
            .or_else(|| self.requests.first())
            .cloned();

        let incoming_sensor: Rc<Cell<Option<Bounds<Pixels>>>> = Rc::new(Cell::new(None));
        let router_sensor: Rc<Cell<Option<Bounds<Pixels>>>> = Rc::new(Cell::new(None));
        let human_sensor: Rc<Cell<Option<Bounds<Pixels>>>> = Rc::new(Cell::new(None));
        let mut spec_sensors: HashMap<SharedString, Rc<Cell<Option<Bounds<Pixels>>>>> =
            HashMap::new();

        // Stage 1: Top Incoming Request Card
        let incoming_el = if let Some(ref req) = incoming_req {
            let rid = req.id.clone();
            let req_to_hov = req.to.clone();
            let sens = incoming_sensor.clone();
            let conf_pct = (req.confidence * 100.0).round() as u32;
            let (sbg, sfg) = match req.status {
                RouterRequestStatus::Routing | RouterRequestStatus::New => (p.accent_soft, p.blue),
                RouterRequestStatus::Escalated => (p.amber_soft, p.amber_ink),
                RouterRequestStatus::Routed | RouterRequestStatus::Done => {
                    (p.green_soft, p.green_ink)
                }
            };
            let is_settled_ok = matches!(
                req.status,
                RouterRequestStatus::Routed | RouterRequestStatus::Done
            );

            pro_tactile(
                pro_eid(&self.id, "r-incoming-card"),
                format!("{}: {}", req.title, req.status.label(cx)),
                10.,
                p.tray,
                pro_mix_color(p.tray, p.chip_hover, 0.45),
                p.line_soft,
                p.line_strong,
                self.inspect_req_id.as_ref() == Some(&rid),
                div().w_full().p(px(2.5)).child(
                    div()
                        .w_full()
                        .relative()
                        .rounded(px(8.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .p(px(10.))
                        .flex()
                        .flex_col()
                        .gap(px(6.))
                        .child(
                            canvas(
                                move |bounds, _, _| {
                                    sens.set(Some(bounds));
                                },
                                |_, _, _, _| (),
                            )
                            .absolute()
                            .inset_0(),
                        )
                        .child(
                            div()
                                .w_full()
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .min_w_0()
                                .child(
                                    div()
                                        .size(px(22.))
                                        .flex_none()
                                        .rounded(px(6.))
                                        .bg(p.chip)
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .child(pro_icon(req.channel.icon(), 12.5, p.ink)),
                                )
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
                                                .text_color(p.strong)
                                                .truncate()
                                                .child(req.title.clone()),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(11.))
                                                .text_color(p.muted)
                                                .truncate()
                                                .child(req.from.clone()),
                                        ),
                                )
                                .child(
                                    div()
                                        .h(px(18.))
                                        .px(px(6.))
                                        .flex_none()
                                        .rounded(px(4.))
                                        .bg(sbg)
                                        .text_color(sfg)
                                        .text_size(px(11.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .flex()
                                        .items_center()
                                        .gap(px(3.5))
                                        .when(req.status == RouterRequestStatus::Routing, |d| {
                                            d.child(pro_svg("0 0 24 24", SPIN_ARC_SVG, 11., sfg))
                                        })
                                        .when(is_settled_ok, |d| {
                                            d.child(pro_check_draw(
                                                pro_eid(&self.id, "r-inc-chk"),
                                                true,
                                                11.5,
                                                false,
                                                p.green,
                                            ))
                                        })
                                        .child(pro_swap_label(
                                            pro_eid(&self.id, "r-inc-st"),
                                            req.status.label(cx),
                                            11.,
                                            FontWeight::MEDIUM,
                                            sfg,
                                        )),
                                ),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .line_height(px(16.))
                                .text_color(p.text)
                                .truncate()
                                .child(req.preview.clone()),
                        )
                        .child(pro_rolling_number(
                            pro_eid(&self.id, "r-inc-conf"),
                            f64::from(conf_pct),
                            0,
                            "",
                            format!("% {}", tr(cx, "confidence", "置信度")),
                            11.,
                            FontWeight::NORMAL,
                            p.muted,
                        )),
                ),
                cx,
                move |this, _, cx| {
                    this.inspect_req_id = Some(rid.clone());
                    this.last_inspect_req_id = Some(rid.clone());
                    cx.emit(AgentRouterEvent::RequestSelected {
                        request_id: rid.clone(),
                    });
                    cx.notify();
                },
            )
            .when(wide, |d| d.w(px(244.)))
            .when(!wide, |d| d.w_full())
            .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                let next = if *inside { req_to_hov.clone() } else { None };
                if this.hovered_req_target != next {
                    this.hovered_req_target = next;
                    cx.notify();
                }
            }))
            .into_any_element()
        } else {
            div().into_any_element()
        };

        // Stage 2: Router Card + Off-to-the-right Human Fallback Tag
        let router_busy = self.flight.is_some();
        let conf_val = incoming_req.as_ref().map(|r| r.confidence).unwrap_or(0.92);
        let conf_pct = (conf_val * 100.0).round() as u32;
        let conf_col = if conf_val < 0.65 { p.amber } else { p.blue };
        let r_sens = router_sensor.clone();
        let r_pkey = infer_provider_key(&self.router.model);

        let router_card = div()
            .when(wide, |d| d.w(px(244.)))
            .when(!wide, |d| d.w_full())
            .rounded(px(10.))
            .bg(p.tray)
            .border_1()
            .border_color(p.line_soft)
            .p(px(2.5))
            .flex()
            .flex_col()
            .child(
                div()
                    .w_full()
                    .relative()
                    .rounded(px(8.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .p(px(10.))
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .child(
                        canvas(
                            move |bounds, _, _| {
                                r_sens.set(Some(bounds));
                            },
                            |_, _, _, _| (),
                        )
                        .absolute()
                        .inset_0(),
                    )
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .min_w_0()
                            .child(pro_agent_avatar(
                                &self.router.id,
                                p.blue,
                                22.,
                                router_busy,
                                elapsed_ms,
                                cx,
                            ))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .text_size(px(13.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.strong)
                                            .child(self.router.name.clone()),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(4.))
                                            .text_size(px(11.))
                                            .text_color(p.muted)
                                            .children(pro_app_artwork(r_pkey, 11., cx))
                                            .child(self.router.model.clone()),
                                    ),
                            )
                            .child(
                                div()
                                    .h(px(18.))
                                    .px(px(6.))
                                    .rounded(px(4.))
                                    .bg(p.accent_soft)
                                    .flex()
                                    .items_center()
                                    .child(pro_swap_label(
                                        pro_eid(&self.id, "r-hub-st"),
                                        if router_busy {
                                            tr(cx, "Routing", "分析中")
                                        } else {
                                            tr(cx, "Ready", "监听中")
                                        },
                                        11.,
                                        FontWeight::MEDIUM,
                                        p.blue,
                                    )),
                            ),
                    )
                    .child(
                        div()
                            .w_full()
                            .h(px(16.))
                            .overflow_hidden()
                            .child(pro_shimmer_text(
                                pro_eid(&self.id, "r-act-shim"),
                                self.router.activity.clone(),
                                p.text,
                                p.strong,
                                12.,
                                FontWeight::NORMAL,
                                router_busy,
                            )),
                    )
                    .child(
                        div()
                            .w_full()
                            .h(px(14.))
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(div().flex_1().child(pro_spring_bar(
                                pro_eid(&self.id, "r-conf-bar"),
                                conf_val,
                                3.0,
                                2.0,
                                p.chip,
                                conf_col,
                            )))
                            .child(pro_rolling_number(
                                pro_eid(&self.id, "r-hub-conf"),
                                f64::from(conf_pct),
                                0,
                                "",
                                format!("% {}", tr(cx, "sure", "确信")),
                                11.,
                                FontWeight::NORMAL,
                                p.muted,
                            )),
                    ),
            )
            .when_some(incoming_req.as_ref(), |d, req| {
                d.child(
                    div()
                        .w_full()
                        .h(px(21.))
                        .px(px(7.))
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .text_size(px(11.))
                        .text_color(p.muted)
                        .child(pro_svg("0 0 24 24", CORNER_ARROW_SVG, 11., p.muted))
                        .child(div().flex_1().min_w_0().truncate().child(req.title.clone())),
                )
            });

        let human_agent = self.agents.iter().find(|a| a.human).cloned();
        let human_dimmed = focus_target
            .as_ref()
            .is_some_and(|ft| human_agent.as_ref().is_some_and(|h| &h.id != ft));
        let human_op = pro_spring_f32(
            pro_eid(&self.id, "r-human-op"),
            if human_dimmed { 0.34 } else { 1.0 },
            0.22,
            w,
            cx,
        )
        .clamp(0.0, 1.0);
        let human_tag = human_agent.map(|hag| {
            let hid = hag.id.clone();
            let hid_hov = hid.clone();
            let h_cnt = self.queue_len(&hid);
            let h_sens = human_sensor.clone();
            let glowing = self.arrival_glows.contains_key(&hid);
            let escalated_req_id = self
                .requests
                .iter()
                .find(|r| r.to.as_ref() == Some(&hid))
                .map(|r| r.id.clone());
            let rest_bg = if glowing || h_cnt > 0 {
                p.amber_soft
            } else {
                p.card
            };
            let rest_bdr = if h_cnt > 0 { p.amber } else { p.line_strong };

            pro_tactile(
                pro_eid(&self.id, "r-human-tag"),
                format!("Human fallback: {h_cnt}"),
                999.,
                rest_bg,
                pro_mix_color(rest_bg, p.chip_hover, 0.50),
                rest_bdr,
                if h_cnt > 0 { p.amber } else { p.blue },
                focus_target.as_ref() == Some(&hid),
                div()
                    .h(px(28.))
                    .px(px(9.))
                    .relative()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(
                        canvas(
                            move |bounds, _, _| {
                                h_sens.set(Some(bounds));
                            },
                            |_, _, _, _| (),
                        )
                        .absolute()
                        .inset_0(),
                    )
                    .child(
                        div()
                            .size(px(16.))
                            .rounded(px(4.))
                            .bg(p.primary)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(pro_icon(ProIcon::User, 10., p.on_primary)),
                    )
                    .child(
                        div()
                            .text_size(px(11.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(if h_cnt > 0 { p.amber_ink } else { p.ink })
                            .child(tr(cx, "Human", "人工坐席")),
                    )
                    .child(pro_rolling_number(
                        pro_eid(&self.id, "r-hcnt"),
                        h_cnt as f64,
                        0,
                        "",
                        "",
                        11.5,
                        FontWeight::SEMIBOLD,
                        if h_cnt > 0 { p.amber_ink } else { p.muted },
                    )),
                cx,
                move |this, _, cx| {
                    if let Some(ref erid) = escalated_req_id {
                        this.inspect_req_id = Some(erid.clone());
                        this.last_inspect_req_id = Some(erid.clone());
                        cx.notify();
                    }
                },
            )
            .opacity(human_op)
            .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                let next = if *inside {
                    Some(hid_hov.clone())
                } else if this.hovered_agent_id.as_ref() == Some(&hid_hov) {
                    None
                } else {
                    this.hovered_agent_id.clone()
                };
                if this.hovered_agent_id != next {
                    this.hovered_agent_id = next;
                    cx.notify();
                }
            }))
        });

        let router_stage_row = if wide {
            div()
                .w_full()
                .mt(px(34.))
                .relative()
                .flex()
                .items_center()
                .justify_center()
                .child(router_card)
                .when_some(human_tag, |d, htag| {
                    d.child(div().absolute().right(px(12.)).child(htag))
                })
        } else {
            div()
                .w_full()
                .mt(px(16.))
                .flex()
                .flex_col()
                .gap(px(10.))
                .child(router_card)
                .when_some(human_tag, |d, htag| {
                    d.child(div().flex().justify_end().child(htag))
                })
        };

        // Stage 3: Specialist Columns + Task Queues Below
        let specialists: Vec<(usize, RouterAgent)> = self
            .agents
            .iter()
            .enumerate()
            .filter(|(_, a)| !a.human)
            .map(|(i, a)| (i, a.clone()))
            .collect();

        let mut spec_cols: Vec<AnyElement> = Vec::with_capacity(specialists.len());
        let mut spec_link_weights: HashMap<SharedString, (f32, f32)> =
            HashMap::with_capacity(specialists.len());

        for (ai, spec) in &specialists {
            let sid = spec.id.clone();
            let sid_hov = sid.clone();
            let acol = self.specialist_color(*ai, spec, &p);
            let s_sens = Rc::new(Cell::new(None));
            spec_sensors.insert(sid.clone(), s_sens.clone());

            let queued: Vec<RouterRequest> = self
                .requests
                .iter()
                .filter(|r| r.status == RouterRequestStatus::Routed && r.to.as_ref() == Some(&sid))
                .cloned()
                .collect();
            let q_cnt = queued.len();
            let dimmed = focus_target
                .as_ref()
                .map(|fid| fid != &sid)
                .unwrap_or(false);
            let spotlighted = focus_target.as_ref() == Some(&sid);
            let col_op = pro_spring_f32(
                pro_eid(&self.id, format!("rspec-op-{sid}")),
                if dimmed { 0.32 } else { 1.0 },
                0.22,
                w,
                cx,
            )
            .clamp(0.0, 1.0);
            let link_spot = pro_spring_f32(
                pro_eid(&self.id, format!("rspec-spt-{sid}")),
                if spotlighted { 1.0 } else { 0.0 },
                0.22,
                w,
                cx,
            )
            .clamp(0.0, 1.0);
            spec_link_weights.insert(sid.clone(), (col_op, link_spot));

            let glow_overlay = self.arrival_glows.get(&sid).map(|started| {
                let age = (started.elapsed().as_secs_f32() / 0.95).clamp(0.0, 1.0);
                let alpha = (1.0 - pro_ease(age)) * (if p.dark { 0.30 } else { 0.22 });
                div()
                    .absolute()
                    .inset_0()
                    .rounded(px(8.))
                    .bg(linear_gradient(
                        180.,
                        linear_color_stop(acol.opacity(alpha), 0.),
                        linear_color_stop(transparent_black(), 0.88),
                    ))
            });

            let pkey = infer_provider_key(&spec.model);
            let filter_click = sid.clone();
            let is_filtered = self.filter_agent_id.as_ref() == Some(&sid);

            let spec_card = pro_tactile(
                pro_eid(&self.id, format!("rspec-{sid}")),
                format!("{}, {} in queue", spec.name, q_cnt),
                10.,
                p.tray,
                pro_mix_color(p.tray, p.chip_hover, 0.45),
                if is_filtered { acol } else { p.line_soft },
                acol,
                is_filtered,
                div().w_full().p(px(2.5)).child(
                    div()
                        .w_full()
                        .relative()
                        .overflow_hidden()
                        .rounded(px(8.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .p(px(10.))
                        .flex()
                        .flex_col()
                        .gap(px(6.))
                        .children(glow_overlay)
                        .child(
                            canvas(
                                move |bounds, _, _| {
                                    s_sens.set(Some(bounds));
                                },
                                |_, _, _, _| (),
                            )
                            .absolute()
                            .inset_0(),
                        )
                        .child(
                            div()
                                .w_full()
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .min_w_0()
                                .child(pro_agent_avatar(
                                    &spec.id,
                                    acol,
                                    22.,
                                    q_cnt > 0,
                                    elapsed_ms,
                                    cx,
                                ))
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .flex()
                                        .flex_col()
                                        .child(
                                            div()
                                                .text_size(px(13.))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(p.strong)
                                                .truncate()
                                                .child(spec.name.clone()),
                                        )
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap(px(4.))
                                                .text_size(px(11.))
                                                .text_color(p.muted)
                                                .children(pro_app_artwork(pkey, 11., cx))
                                                .child(spec.model.clone()),
                                        ),
                                )
                                .child(
                                    div()
                                        .h(px(18.))
                                        .px(px(6.))
                                        .rounded(px(4.))
                                        .bg(if q_cnt > 0 { p.accent_soft } else { p.chip })
                                        .flex()
                                        .items_center()
                                        .child(pro_swap_label(
                                            pro_eid(&self.id, format!("rspec-st-{sid}")),
                                            if q_cnt > 0 {
                                                tr(cx, "Active", "处理中")
                                            } else {
                                                tr(cx, "Idle", "空闲")
                                            },
                                            11.,
                                            FontWeight::MEDIUM,
                                            if q_cnt > 0 { p.blue } else { p.muted },
                                        )),
                                ),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .line_height(px(16.))
                                .text_color(p.text)
                                .truncate()
                                .child(spec.activity.clone()),
                        )
                        .child(
                            div()
                                .w_full()
                                .h(px(14.))
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .child(div().flex_1().child(pro_spring_bar(
                                    pro_eid(&self.id, format!("sload-{sid}")),
                                    (spec.load + (q_cnt as f32) * 0.14).clamp(0.08, 0.96),
                                    3.0,
                                    2.0,
                                    p.chip,
                                    acol,
                                )))
                                .child(pro_rolling_number(
                                    pro_eid(&self.id, format!("sqcnt-{sid}")),
                                    q_cnt as f64,
                                    0,
                                    "",
                                    format!(" {}", tr(cx, "in queue", "条排队")),
                                    11.,
                                    FontWeight::NORMAL,
                                    p.muted,
                                )),
                        ),
                ),
                cx,
                move |this, _, cx| {
                    if this.filter_agent_id.as_ref() == Some(&filter_click) {
                        this.filter_agent_id = None;
                    } else {
                        this.filter_agent_id = Some(filter_click.clone());
                    }
                    cx.notify();
                },
            )
            .w_full()
            .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                let next = if *inside {
                    Some(sid_hov.clone())
                } else if this.hovered_agent_id.as_ref() == Some(&sid_hov) {
                    None
                } else {
                    this.hovered_agent_id.clone()
                };
                if this.hovered_agent_id != next {
                    this.hovered_agent_id = next;
                    cx.notify();
                }
            }));

            // Task queue container directly under specialist card
            let mut queue_box = div()
                .w_full()
                .p(px(8.))
                .rounded(px(10.))
                .bg(p.subtle)
                .border_1()
                .border_color(p.line_soft)
                .flex()
                .flex_col()
                .gap(px(6.))
                .child(
                    div()
                        .w_full()
                        .px(px(2.))
                        .flex()
                        .items_center()
                        .justify_between()
                        .text_size(px(11.5))
                        .child(
                            div()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.muted)
                                .child(tr(cx, "Task queue", "任务队列")),
                        )
                        .child(
                            div()
                                .h(px(18.))
                                .px(px(6.))
                                .rounded(px(4.))
                                .bg(p.chip)
                                .flex()
                                .items_center()
                                .child(pro_rolling_number(
                                    pro_eid(&self.id, format!("sqpill-{sid}")),
                                    q_cnt as f64,
                                    0,
                                    "",
                                    "",
                                    11.,
                                    FontWeight::MEDIUM,
                                    p.muted,
                                )),
                        ),
                );

            if queued.is_empty() {
                queue_box = queue_box.child(
                    div()
                        .w_full()
                        .py(px(16.))
                        .rounded(px(7.))
                        .border_1()
                        .border_color(p.line_soft)
                        .text_size(px(11.5))
                        .text_color(p.faint)
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(tr(cx, "No tasks yet", "暂无排队任务")),
                );
            } else {
                for qreq in queued.iter().take(3) {
                    let qrid = qreq.id.clone();
                    let q_target = qreq.to.clone();
                    let qconf = (qreq.confidence * 100.0).round() as u32;
                    let mut tags_el = div().flex().flex_wrap().gap(px(4.));
                    for tg in &qreq.tags {
                        tags_el = tags_el.child(
                            div()
                                .h(px(17.))
                                .px(px(5.))
                                .rounded(px(4.))
                                .bg(p.chip)
                                .text_size(px(10.5))
                                .text_color(p.muted)
                                .flex()
                                .items_center()
                                .child(tg.clone()),
                        );
                    }
                    queue_box = queue_box.child(
                        pro_tactile(
                            pro_eid(&self.id, format!("qitem-{qrid}")),
                            format!("{} · {qconf}%", qreq.title),
                            7.,
                            p.card,
                            pro_mix_color(p.card, p.hover, 0.75),
                            p.line,
                            p.line_strong,
                            self.inspect_req_id.as_ref() == Some(&qrid),
                            div()
                                .w_full()
                                .p(px(8.))
                                .flex()
                                .flex_col()
                                .gap(px(5.))
                                .child(
                                    div()
                                        .w_full()
                                        .flex()
                                        .items_center()
                                        .gap(px(6.))
                                        .min_w_0()
                                        .child(pro_icon(qreq.channel.icon(), 12., p.muted))
                                        .child(
                                            div()
                                                .flex_1()
                                                .min_w_0()
                                                .text_size(px(12.))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(p.strong)
                                                .truncate()
                                                .child(qreq.title.clone()),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(10.5))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(acol)
                                                .child(format!("{qconf}%")),
                                        ),
                                )
                                .when(!qreq.tags.is_empty(), |d| d.child(tags_el)),
                            cx,
                            move |this, _, cx| {
                                this.inspect_req_id = Some(qrid.clone());
                                this.last_inspect_req_id = Some(qrid.clone());
                                cx.emit(AgentRouterEvent::RequestSelected {
                                    request_id: qrid.clone(),
                                });
                                cx.notify();
                            },
                        )
                        .w_full()
                        .on_hover(cx.listener(
                            move |this, inside: &bool, _, cx| {
                                let next = if *inside { q_target.clone() } else { None };
                                if this.hovered_req_target != next {
                                    this.hovered_req_target = next;
                                    cx.notify();
                                }
                            },
                        )),
                    );
                }
            }

            spec_cols.push(
                div()
                    .when(wide, |d| d.flex_1().min_w_0())
                    .when(!wide, |d| d.w_full())
                    .opacity(col_op)
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(spec_card)
                    .child(queue_box)
                    .into_any_element(),
            );
        }

        // Compute flight pulses for Stage 1 (Incoming -> Router) and Stage 2 (Router -> Target)
        let (in_prog, out_prog): (Option<f32>, Option<(SharedString, f32)>) = if let Some(ref fl) =
            self.flight
            && !reduced
        {
            let el = fl.started.elapsed().as_secs_f32();
            if fl.two_stage {
                if el < 0.60 {
                    (Some(pro_ease(el / 0.60)), None)
                } else {
                    let t2 = ((el - 0.60) / 0.85).clamp(0.0, 1.0);
                    (None, Some((fl.target_id.clone(), pro_ease(t2))))
                }
            } else {
                let t = (el / 0.72).clamp(0.0, 1.0);
                (None, Some((fl.target_id.clone(), pro_ease(t))))
            }
        } else {
            (None, None)
        };

        let link_blue: Hsla = if p.dark {
            p.blue.opacity(0.68)
        } else {
            rgb(0x83c7fe).into()
        };
        let pulse_blue = p.blue;
        let pulse_orange = p.orange;
        let card_bg = p.card;
        let human_id = self.agents.iter().find(|a| a.human).map(|a| a.id.clone());
        let human_spot = pro_spring_f32(
            pro_eid(&self.id, "r-human-spt"),
            if human_id
                .as_ref()
                .is_some_and(|hid| focus_target.as_ref() == Some(hid))
            {
                1.0
            } else {
                0.0
            },
            0.22,
            w,
            cx,
        )
        .clamp(0.0, 1.0);
        let in_spot = pro_spring_f32(
            pro_eid(&self.id, "r-in-spt"),
            if focus_target.is_some() { 1.0 } else { 0.0 },
            0.22,
            w,
            cx,
        )
        .clamp(0.0, 1.0);

        let connectors_canvas = canvas(
            |_, _, _| (),
            move |_, _, window, _| {
                // 1) Incoming -> Router vertical connector
                if let (Some(bi), Some(br)) = (incoming_sensor.get(), router_sensor.get()) {
                    let p0 = point((bi.left() + bi.right()) * 0.5, bi.bottom() + px(2.5));
                    let p1 = point((br.left() + br.right()) * 0.5, br.top() - px(2.5));
                    let in_c = pro_mix_color(link_blue, pulse_blue, in_spot * 0.60);
                    pro_connector_pulse(
                        p0,
                        p1,
                        6.0,
                        true,
                        in_c,
                        pulse_blue,
                        in_prog,
                        1.5 + 0.35 * in_spot,
                        window,
                    );
                    paint_ring_dot(p0, 3.0, card_bg, in_c, 1.5, window);
                }

                // 2) Router -> Specialist bottom S-bends + Router -> Human right fade line
                if let Some(br) = router_sensor.get() {
                    let p0 = point((br.left() + br.right()) * 0.5, br.bottom() + px(2.5));
                    for (sid, s_cell) in &spec_sensors {
                        let Some(bs) = s_cell.get() else {
                            continue;
                        };
                        let p1 = point((bs.left() + bs.right()) * 0.5, bs.top() - px(2.5));
                        let prog = out_prog
                            .as_ref()
                            .and_then(|(tid, pr)| if tid == sid { Some(*pr) } else { None });
                        let (vis, spot) = spec_link_weights.get(sid).copied().unwrap_or((1.0, 0.0));
                        let base_alpha = if prog.is_some() {
                            0.85
                        } else {
                            (0.45 * vis + 0.45 * spot).clamp(0.14, 0.92)
                        };
                        pro_connector_pulse(
                            p0,
                            p1,
                            8.0,
                            true,
                            link_blue.opacity(base_alpha),
                            pulse_blue,
                            prog,
                            1.5 + 0.45 * spot,
                            window,
                        );
                    }
                    paint_ring_dot(p0, 3.0, card_bg, link_blue, 1.5, window);

                    if let (Some(hid), Some(bh)) = (&human_id, human_sensor.get())
                        && bh.left() > br.right() + px(8.)
                    {
                        let hp0 = point(br.right() + px(2.5), (br.top() + br.bottom()) * 0.5);
                        let hp1 = point(bh.left() - px(2.0), (bh.top() + bh.bottom()) * 0.5);
                        let hprog = out_prog
                            .as_ref()
                            .and_then(|(tid, pr)| if tid == hid { Some(*pr) } else { None });
                        let h_alpha = (0.38 * human_op + 0.50 * human_spot).clamp(0.14, 0.90);
                        pro_connector_pulse(
                            hp0,
                            hp1,
                            6.0,
                            false,
                            pro_mix_color(
                                link_blue.opacity(h_alpha),
                                pulse_orange.opacity(0.88),
                                human_spot * 0.70,
                            ),
                            pulse_orange,
                            hprog,
                            1.5 + 0.45 * human_spot,
                            window,
                        );
                        paint_ring_dot(hp0, 3.0, card_bg, link_blue, 1.5, window);
                    }
                }
            },
        )
        .absolute()
        .inset_0();

        let open_modal = self
            .inspect_req_id
            .clone()
            .or_else(|| self.last_inspect_req_id.clone())
            .and_then(|rid| {
                let req = self.requests.iter().find(|r| r.id == rid)?;
                Some(self.render_score_modal(req, &p, cx))
            });

        let sr_announce = self.sr_status.clone();

        div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .role(Role::Region)
            .aria_label(title_str)
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, w, cx| {
                if ev.keystroke.key == "escape" {
                    if this.inspect_req_id.is_some() {
                        this.inspect_req_id = None;
                        this.focus.focus(w, cx);
                        cx.notify();
                    } else if this.filter_agent_id.is_some() {
                        this.filter_agent_id = None;
                        cx.notify();
                    }
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
                    .relative()
                    .flex()
                    .flex_col()
                    .when(wide, |d| d.items_center())
                    .child(connectors_canvas)
                    .child(incoming_el)
                    .child(router_stage_row)
                    .child(
                        div()
                            .w_full()
                            .mt(px(if wide { 36. } else { 18. }))
                            .when(wide, |d| d.flex().items_start().gap(px(12.)))
                            .when(!wide, |d| d.flex().flex_col().gap(px(14.)))
                            .children(spec_cols),
                    ),
            )
            .children(open_modal)
            .child(pro_toast_banner(self.toast.clone()))
            .child(
                div()
                    .id(pro_eid(&self.id, "sr-router"))
                    .role(Role::Status)
                    .aria_label(sr_announce)
                    .absolute()
                    .size_0(),
            )
            .into_any_element()
    }
}

fn paint_ring_dot(
    center: Point<Pixels>,
    r: f32,
    fill_color: Hsla,
    stroke_color: Hsla,
    stroke_w: f32,
    window: &mut Window,
) {
    let outer = r + stroke_w * 0.5;
    window.paint_quad(
        fill(
            Bounds::new(
                point(center.x - px(outer), center.y - px(outer)),
                size(px(outer * 2.0), px(outer * 2.0)),
            ),
            stroke_color,
        )
        .corner_radii(px(outer)),
    );
    let inner = (outer - stroke_w).max(1.0);
    window.paint_quad(
        fill(
            Bounds::new(
                point(center.x - px(inner), center.y - px(inner)),
                size(px(inner * 2.0), px(inner * 2.0)),
            ),
            fill_color,
        )
        .corner_radii(px(inner)),
    );
}

fn infer_provider_key(model: &str) -> &'static str {
    let lower = model.to_ascii_lowercase();
    if lower.contains("claude")
        || lower.contains("haiku")
        || lower.contains("sonnet")
        || lower.contains("opus")
    {
        "claude"
    } else if lower.contains("gemini") {
        "gemini"
    } else {
        "openai"
    }
}

fn default_router_setup() -> (RouterAgent, Vec<RouterAgent>, Vec<RouterRequest>) {
    let router = RouterAgent::new("router", "Router", "Claude Haiku")
        .with_activity("Reading “Refund for order #4821”");

    let agents = vec![
        RouterAgent::new("billing", "Billing", "Claude Sonnet")
            .with_activity("Issuing duplicate-charge credit")
            .with_load(0.38),
        RouterAgent::new("tech", "Tech support", "GPT-5")
            .with_activity("Triaging SAML callback logs")
            .with_load(0.46),
        RouterAgent::new("sales", "Sales", "Gemini 2.5 Flash")
            .with_activity("Preparing enterprise seat quote")
            .with_load(0.22),
        RouterAgent::human_fallback("human", "Support lead"),
    ];

    let requests = vec![
        RouterRequest::new(
            "r1",
            "Refund for order #4821",
            "maria@northwind.io",
            "I was charged twice for the same order – can you refund one?",
        )
        .with_channel(RouterChannel::Email)
        .with_routing(RouterRequestStatus::Routing, Some("billing"), 0.94)
        .with_scores(vec![("billing", 0.94), ("tech", 0.12), ("sales", 0.04)])
        .with_reason("Double-charge mention on order #4821 — 94% match for Billing.")
        .with_tags(["refund", "invoice"]),
        RouterRequest::new(
            "r2",
            "Update VAT number on October invoice",
            "finance@atelier.de",
            "Please add DE298410229 to our October receipt PDF.",
        )
        .with_channel(RouterChannel::Email)
        .with_routing(RouterRequestStatus::Routed, Some("billing"), 0.91)
        .with_scores(vec![("billing", 0.91), ("sales", 0.18), ("tech", 0.05)])
        .with_reason("Tax ID & invoice amendment — handled by Billing.")
        .with_tags(["vat", "receipt"]),
        RouterRequest::new(
            "r3",
            "SAML Okta redirect loop on Safari",
            "it@vanguard-bio.com",
            "Users bounce back to /login after Okta POST assertion.",
        )
        .with_channel(RouterChannel::Ticket)
        .with_routing(RouterRequestStatus::Routed, Some("tech"), 0.96)
        .with_scores(vec![("tech", 0.96), ("billing", 0.06), ("sales", 0.04)])
        .with_reason("Okta SAML assertion loop — Tech support SSO playbook.")
        .with_tags(["sso", "okta"]),
        RouterRequest::new(
            "r4",
            "Rate limit headers missing on /v2/runs",
            "sdk@pulsemetrics.io",
            "X-RateLimit-Remaining stopped appearing in staging responses.",
        )
        .with_channel(RouterChannel::Slack)
        .with_routing(RouterRequestStatus::Routed, Some("tech"), 0.89)
        .with_scores(vec![("tech", 0.89), ("sales", 0.09), ("billing", 0.05)])
        .with_reason("REST API header regression — routed to Tech support.")
        .with_tags(["api", "v2"]),
        RouterRequest::new(
            "r5",
            "Annual quote for 80 seats + HIPAA BAA",
            "procurement@medcore.org",
            "Looking to move 80 seats onto an annual contract next quarter.",
        )
        .with_channel(RouterChannel::Chat)
        .with_routing(RouterRequestStatus::Routed, Some("sales"), 0.93)
        .with_scores(vec![("sales", 0.93), ("billing", 0.24), ("tech", 0.08)])
        .with_reason("80-seat annual contract + BAA inquiry — routed to Sales.")
        .with_tags(["enterprise", "baa"]),
    ];

    (router, agents, requests)
}

#[cfg(test)]
mod tests {
    use gpui::AppContext;

    use super::{AgentRouter, RouterRequestStatus};
    use gpui::TestAppContext;

    #[test]
    fn default_router_initializes_specialists_human_fallback_and_queues() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let router = app.new(|cx| AgentRouter::new("router-test", cx));
        router.read_with(&app, |r, _| {
            assert_eq!(r.agents.len(), 4);
            assert!(r.agents.iter().any(|a| a.human));
            assert_eq!(r.queue_len("billing"), 1);
            assert_eq!(r.queue_len("tech"), 2);
            assert_eq!(r.queue_len("sales"), 1);
        });
    }

    #[test]
    fn manual_reroute_moves_request_into_target_or_human_fallback() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let router = app.new(|cx| AgentRouter::new("router-reroute", cx));
        router.update(&mut app, |r, cx| {
            r.reroute_request("r2", "human", cx);
            assert_eq!(r.queue_len("billing"), 0);
            assert_eq!(r.queue_len("human"), 1);
            let r2 = r
                .requests
                .iter()
                .find(|x| x.id.as_ref() == "r2")
                .expect("r2 exists");
            assert_eq!(r2.status, RouterRequestStatus::Escalated);
        });
    }
}
