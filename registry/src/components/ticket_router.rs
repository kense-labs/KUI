//! Host-driven support classification and assignment, with review and retry.
use crate::primitives::{
    brand_mark::brand_mark,
    control::UiLanguage,
    family::{Paint, PhaseClock, Tone, disabled_motion, ease, face},
    presentation::ChangingText,
};
use gpui::{prelude::*, *};
use std::{cell::Cell, collections::HashSet, rc::Rc};
const TICKETS_PER_PAGE: usize = 4;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ticket {
    pub id: SharedString,
    pub subject: SharedString,
    pub body: SharedString,
    pub sender: SharedString,
    pub channel: SharedString,
}
impl Ticket {
    pub fn new(
        id: impl Into<SharedString>,
        subject: impl Into<SharedString>,
        body: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            subject: subject.into(),
            body: body.into(),
            sender: "".into(),
            channel: "".into(),
        }
    }
    pub fn with_sender(mut self, sender: impl Into<SharedString>) -> Self {
        self.sender = sender.into();
        self
    }
    pub fn with_channel(mut self, channel: impl Into<SharedString>) -> Self {
        self.channel = channel.into();
        self
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TicketLane {
    pub id: SharedString,
    pub label: SharedString,
}
impl TicketLane {
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct TicketDecision {
    pub intent: SharedString,
    pub sentiment: SharedString,
    pub confidence: f32,
    pub lane: SharedString,
    pub assignee: SharedString,
}
impl TicketDecision {
    pub fn new(lane: impl Into<SharedString>, confidence: f32) -> Self {
        Self {
            intent: "".into(),
            sentiment: "".into(),
            confidence: if confidence.is_finite() {
                confidence.clamp(0., 1.)
            } else {
                0.
            },
            lane: lane.into(),
            assignee: "".into(),
        }
    }
    pub fn with_intent(mut self, value: impl Into<SharedString>) -> Self {
        self.intent = value.into();
        self
    }
    pub fn with_sentiment(mut self, value: impl Into<SharedString>) -> Self {
        self.sentiment = value.into();
        self
    }
    pub fn with_assignee(mut self, value: impl Into<SharedString>) -> Self {
        self.assignee = value.into();
        self
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TicketStatus {
    #[default]
    Idle,
    Queued,
    Classifying,
    Review,
    RouteQueued,
    Routing,
    Routed,
    Failed,
}
#[derive(Clone, Debug)]
pub enum TicketRouterEvent {
    ClassifyRequested {
        request: u64,
        ticket: Ticket,
    },
    RouteRequested {
        request: u64,
        ticket: Ticket,
        decision: TicketDecision,
    },
    Cancelled(u64),
}
struct Entry {
    ticket: Ticket,
    status: TicketStatus,
    request: Option<u64>,
    decision: Option<TicketDecision>,
    error: SharedString,
    route_failed: bool,
    clock: PhaseClock,
}
/// Requests are emitted to the host; a ticket is marked routed only after host confirmation.
pub struct TicketRouter {
    title: SharedString,
    model: Option<SharedString>,
    lanes: Vec<TicketLane>,
    entries: Vec<Entry>,
    threshold: f32,
    concurrency: usize,
    request_seq: u64,
    paused: bool,
    visible: bool,
    lane_counts: Vec<ChangingText>,
    lane_pages: Vec<usize>,
    review_count: ChangingText,
    content_width: Rc<Cell<Pixels>>,
}
impl EventEmitter<TicketRouterEvent> for TicketRouter {}
impl TicketRouter {
    pub fn new(
        title: impl Into<SharedString>,
        lanes: Vec<TicketLane>,
        tickets: Vec<Ticket>,
    ) -> Self {
        let mut lane_ids = HashSet::new();
        let lanes: Vec<_> = lanes
            .into_iter()
            .filter(|l| lane_ids.insert(l.id.clone()))
            .collect();
        let mut ids = HashSet::new();
        let entries = tickets
            .into_iter()
            .filter(|t| ids.insert(t.id.clone()))
            .map(|ticket| Entry {
                ticket,
                status: TicketStatus::Idle,
                request: None,
                decision: None,
                error: "".into(),
                route_failed: false,
                clock: Default::default(),
            })
            .collect();
        let lane_counts = lanes.iter().map(|_| ChangingText::new("0")).collect();
        let lane_pages = vec![0; lanes.len()];
        Self {
            title: title.into(),
            model: None,
            lanes,
            entries,
            threshold: 0.75,
            concurrency: 1,
            request_seq: 0,
            paused: false,
            visible: true,
            lane_counts,
            lane_pages,
            review_count: ChangingText::new("0"),
            content_width: Default::default(),
        }
    }
    /// Name the host-selected classification model in the header.
    pub fn with_model(mut self, model: impl Into<SharedString>) -> Self {
        self.model = Some(model.into());
        self
    }
    pub fn with_review_threshold(mut self, threshold: f32) -> Self {
        if threshold.is_finite() {
            self.threshold = threshold.clamp(0., 1.);
        }
        self
    }
    pub fn with_concurrency(mut self, concurrency: usize) -> Self {
        self.concurrency = concurrency.clamp(1, 8);
        self
    }
    pub fn status(&self, ticket: &str) -> Option<TicketStatus> {
        self.entries
            .iter()
            .find(|e| e.ticket.id.as_ref() == ticket)
            .map(|e| e.status)
    }
    pub fn active_request(&self, ticket: &str) -> Option<u64> {
        self.entries
            .iter()
            .find(|e| e.ticket.id.as_ref() == ticket)
            .and_then(|e| e.request)
    }
    fn pump(&mut self, cx: &mut Context<Self>) {
        if self.paused {
            return;
        }
        let mut slots = self
            .concurrency
            .saturating_sub(self.entries.iter().filter(|e| e.request.is_some()).count());
        for entry in &mut self.entries {
            if slots == 0 {
                break;
            }
            if !matches!(
                entry.status,
                TicketStatus::Queued | TicketStatus::RouteQueued
            ) {
                continue;
            }
            self.request_seq = self.request_seq.wrapping_add(1).max(1);
            let request = self.request_seq;
            entry.request = Some(request);
            entry.clock.restart(cx);
            slots -= 1;
            if entry.status == TicketStatus::RouteQueued {
                if let Some(decision) = entry.decision.clone() {
                    entry.status = TicketStatus::Routing;
                    cx.emit(TicketRouterEvent::RouteRequested {
                        request,
                        ticket: entry.ticket.clone(),
                        decision,
                    });
                } else {
                    entry.request = None;
                    entry.status = TicketStatus::Failed;
                    slots += 1;
                }
            } else {
                entry.status = TicketStatus::Classifying;
                cx.emit(TicketRouterEvent::ClassifyRequested {
                    request,
                    ticket: entry.ticket.clone(),
                });
            }
        }
    }
    /// Start all currently idle tickets, respecting the configured concurrency.
    pub fn run(&mut self, cx: &mut Context<Self>) {
        self.paused = false;
        for entry in &mut self.entries {
            if entry.status == TicketStatus::Idle {
                entry.status = TicketStatus::Queued;
            }
        }
        self.pump(cx);
        cx.notify();
    }
    /// Pause both types of in-flight work and retain tickets for a later resume.
    pub fn pause(&mut self, cx: &mut Context<Self>) {
        self.paused = true;
        for entry in &mut self.entries {
            if let Some(request) = entry.request.take() {
                cx.emit(TicketRouterEvent::Cancelled(request));
            }
            entry.status = match entry.status {
                TicketStatus::Classifying => TicketStatus::Queued,
                TicketStatus::Routing => TicketStatus::RouteQueued,
                state => state,
            };
        }
        cx.notify();
    }
    pub fn is_paused(&self) -> bool {
        self.paused
    }
    /// Hosts embedding an off-screen preview can stop scheduling animation frames.
    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        if self.visible != visible {
            self.visible = visible;
            if visible {
                cx.notify();
            }
        }
    }
    fn change_page(&mut self, lane: usize, direction: isize, cx: &mut Context<Self>) {
        let Some(spec) = self.lanes.get(lane) else {
            return;
        };
        let total = self
            .entries
            .iter()
            .filter(|entry| {
                entry.status == TicketStatus::Routed
                    && entry.decision.as_ref().is_some_and(|d| d.lane == spec.id)
            })
            .count();
        let last = total.saturating_sub(1) / TICKETS_PER_PAGE;
        self.lane_pages[lane] = self.lane_pages[lane]
            .saturating_add_signed(direction)
            .min(last);
        cx.notify();
    }
    pub fn resume(&mut self, cx: &mut Context<Self>) {
        self.paused = false;
        self.pump(cx);
        cx.notify();
    }
    /// Upserts a host ticket; changing a ticket cancels work based on the old data.
    pub fn set_ticket(&mut self, ticket: Ticket, cx: &mut Context<Self>) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.ticket.id == ticket.id) {
            if entry.ticket == ticket {
                return;
            }
            if let Some(request) = entry.request.take() {
                cx.emit(TicketRouterEvent::Cancelled(request));
            }
            let working = matches!(
                entry.status,
                TicketStatus::Classifying
                    | TicketStatus::Queued
                    | TicketStatus::Routing
                    | TicketStatus::RouteQueued
            );
            entry.ticket = ticket;
            entry.decision = None;
            entry.error = "".into();
            entry.route_failed = false;
            entry.status = if working {
                TicketStatus::Queued
            } else {
                TicketStatus::Idle
            };
        } else {
            self.entries.push(Entry {
                ticket,
                status: TicketStatus::Queued,
                request: None,
                decision: None,
                error: "".into(),
                route_failed: false,
                clock: Default::default(),
            });
        }
        self.pump(cx);
        cx.notify();
    }
    /// Complete a classifier request; uncertain decisions remain in human review.
    pub fn classified(
        &mut self,
        request: u64,
        mut decision: TicketDecision,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(i) = self
            .entries
            .iter()
            .position(|e| e.request == Some(request) && e.status == TicketStatus::Classifying)
        else {
            return false;
        };
        decision.confidence = if decision.confidence.is_finite() {
            decision.confidence.clamp(0., 1.)
        } else {
            0.
        };
        if !self.lanes.iter().any(|l| l.id == decision.lane) {
            let Some(first) = self.lanes.first() else {
                self.entries[i].request = None;
                self.entries[i].status = TicketStatus::Failed;
                self.entries[i].error = "No destination available".into();
                self.pump(cx);
                cx.notify();
                return true;
            };
            decision.lane = first.id.clone();
        }
        let entry = &mut self.entries[i];
        entry.request = None;
        entry.route_failed = false;
        entry.status = if decision.confidence < self.threshold {
            TicketStatus::Review
        } else {
            TicketStatus::RouteQueued
        };
        entry.decision = Some(decision);
        entry.clock.restart(cx);
        self.pump(cx);
        cx.notify();
        true
    }
    /// Confirm a human-selected lane; pending work for this ticket is cancelled first.
    pub fn route_to(&mut self, ticket: &str, lane: &str, cx: &mut Context<Self>) -> bool {
        if !self.lanes.iter().any(|l| l.id.as_ref() == lane) {
            return false;
        }
        let Some(entry) = self
            .entries
            .iter_mut()
            .find(|e| e.ticket.id.as_ref() == ticket)
        else {
            return false;
        };
        let Some(mut decision) = entry.decision.clone() else {
            return false;
        };
        if let Some(request) = entry.request.take() {
            cx.emit(TicketRouterEvent::Cancelled(request));
        }
        decision.lane = lane.to_owned().into();
        entry.decision = Some(decision);
        entry.status = TicketStatus::RouteQueued;
        entry.error = "".into();
        entry.clock.restart(cx);
        self.pump(cx);
        cx.notify();
        true
    }
    /// Report the host's assignment result. No success is shown before this acknowledgement.
    pub fn routed(
        &mut self,
        request: u64,
        result: Result<(), SharedString>,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(entry) = self
            .entries
            .iter_mut()
            .find(|e| e.request == Some(request) && e.status == TicketStatus::Routing)
        else {
            return false;
        };
        entry.request = None;
        entry.clock.restart(cx);
        match result {
            Ok(()) => {
                entry.status = TicketStatus::Routed;
                entry.error = "".into();
                entry.route_failed = false;
            }
            Err(error) => {
                entry.status = TicketStatus::Failed;
                entry.error = error;
                entry.route_failed = true;
            }
        }
        self.pump(cx);
        cx.notify();
        true
    }
    pub fn failed(
        &mut self,
        request: u64,
        error: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(entry) = self.entries.iter_mut().find(|e| e.request == Some(request)) else {
            return false;
        };
        entry.route_failed = entry.status == TicketStatus::Routing;
        entry.request = None;
        entry.status = TicketStatus::Failed;
        entry.error = error.into();
        entry.clock.restart(cx);
        self.pump(cx);
        cx.notify();
        true
    }
    pub fn retry(&mut self, ticket: &str, cx: &mut Context<Self>) -> bool {
        let Some(entry) = self
            .entries
            .iter_mut()
            .find(|e| e.ticket.id.as_ref() == ticket && e.status == TicketStatus::Failed)
        else {
            return false;
        };
        entry.status = if entry.route_failed && entry.decision.is_some() {
            TicketStatus::RouteQueued
        } else {
            TicketStatus::Queued
        };
        entry.error = "".into();
        self.pump(cx);
        cx.notify();
        true
    }
}
impl Render for TicketRouter {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let l = UiLanguage::current(cx);
        let radius = crate::foundation::surface_radius(cx);
        let animate = self.visible && w.is_visible() && !disabled_motion(cx);
        let t = |en: &'static str, zh: &'static str| l.text(en, zh);
        let working = self
            .entries
            .iter()
            .filter(|e| {
                matches!(
                    e.status,
                    TicketStatus::Queued
                        | TicketStatus::Classifying
                        | TicketStatus::RouteQueued
                        | TicketStatus::Routing
                )
            })
            .count();
        let button_label = if self.paused {
            t("Resume", "继续")
        } else if working > 0 {
            t("Pause", "暂停")
        } else {
            t("Start", "开始")
        };
        let header = div()
            .flex()
            .items_center()
            .justify_between()
            .min_h(px(48.))
            .pl(px(12.))
            .gap(px(8.))
            .child(
                div()
                    .text_size(px(16.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.ink)
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(7.))
                    .children(
                        self.model
                            .as_ref()
                            .and_then(|model| brand_mark(model, 14., cx)),
                    )
                    .child(self.title.clone())
                    .when_some(self.model.clone(), |header, model| {
                        header.child(
                            div()
                                .text_size(px(12.))
                                .text_color(p.secondary)
                                .child(model),
                        )
                    }),
            )
            .child(
                face(
                    "ticket-run",
                    button_label,
                    32.,
                    radius.into(),
                    Tone::Surface,
                    cx,
                )
                .px(px(8.))
                .border_1()
                .border_color(p.line)
                .child(button_label)
                .on_click(cx.listener(|s, _, _, cx| {
                    if s.paused {
                        s.resume(cx);
                    } else if s.entries.iter().any(|e| {
                        matches!(
                            e.status,
                            TicketStatus::Queued
                                | TicketStatus::Classifying
                                | TicketStatus::RouteQueued
                                | TicketStatus::Routing
                        )
                    }) {
                        s.pause(cx);
                    } else {
                        s.run(cx);
                    }
                })),
            );
        let mut queue = div().flex().flex_col().gap(px(6.)).w_full();
        let active = self.entries.iter().position(|entry| {
            matches!(
                entry.status,
                TicketStatus::Classifying | TicketStatus::Routing
            )
        });
        let settled = self
            .entries
            .iter()
            .all(|entry| matches!(entry.status, TicketStatus::Routed | TicketStatus::Review));
        for (i, entry) in self.entries.iter().enumerate().filter(|(i, e)| {
            (settled && *i + 1 == self.entries.len())
                || !matches!(e.status, TicketStatus::Routed | TicketStatus::Review)
        }) {
            let status = match entry.status {
                TicketStatus::Idle => t("Waiting", "等待中"),
                TicketStatus::Queued => t("Queued", "排队中"),
                TicketStatus::Classifying => t("Classifying", "分类中"),
                TicketStatus::RouteQueued => t("Awaiting assignment", "等待分配"),
                TicketStatus::Routing => t("Assigning", "分配中"),
                TicketStatus::Failed => t("Failed", "失败"),
                TicketStatus::Review => t("Needs review", "待审核"),
                TicketStatus::Routed => t("Assigned", "已分配"),
            };
            let key = entry.ticket.id.clone();
            let fade = if animate {
                ease(entry.clock.time(0.16, w, cx) / 0.16)
            } else {
                1.
            };
            let busy = matches!(
                entry.status,
                TicketStatus::Classifying | TicketStatus::Routing
            );
            if animate && busy {
                w.request_animation_frame();
            }
            let shimmer = if animate && busy {
                0.76 + 0.24 * (entry.clock.age(cx) * 3.).sin()
            } else {
                1.
            };
            let mut row = div()
                .id(("ticket-entry", i))
                .w_full()
                .rounded(radius)
                .border_1()
                .border_color(if active == Some(i) {
                    p.ink.opacity(0.16)
                } else {
                    p.line
                })
                .bg(p.panel)
                .p(px(12.))
                .flex()
                .flex_wrap()
                .items_center()
                .justify_between()
                .gap(px(6.))
                .opacity(fade)
                .child(
                    div()
                        .min_w(px(120.))
                        .flex_1()
                        .text_size(px(13.))
                        .text_color(p.ink)
                        .flex()
                        .flex_col()
                        .gap(px(4.))
                        .when(!entry.ticket.sender.is_empty(), |ticket| {
                            ticket.child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(5.))
                                    .text_size(px(12.))
                                    .text_color(p.secondary)
                                    .children(brand_mark(&entry.ticket.channel, 13., cx))
                                    .child(entry.ticket.sender.clone()),
                            )
                        })
                        .child(
                            div()
                                .font_weight(FontWeight::MEDIUM)
                                .child(entry.ticket.subject.clone()),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .line_height(px(18.))
                                .text_color(p.secondary)
                                .child(entry.ticket.body.clone()),
                        ),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(if entry.status == TicketStatus::Failed {
                            p.danger
                        } else {
                            p.secondary
                        })
                        .opacity(shimmer)
                        .child(status),
                );
            if let Some(decision) = &entry.decision {
                row = row.child(
                    div()
                        .w_full()
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .gap(px(8.))
                        .pt(px(8.))
                        .border_t_1()
                        .border_color(p.line)
                        .children(
                            [
                                decision.intent.clone(),
                                decision.sentiment.clone(),
                                format!("{:.0}%", decision.confidence * 100.).into(),
                            ]
                            .into_iter()
                            .filter(|value| !value.is_empty())
                            .map(|value| {
                                div()
                                    .px(px(6.))
                                    .py(px(2.))
                                    .rounded(px(4.))
                                    .bg(p.tray)
                                    .text_size(px(12.))
                                    .text_color(p.secondary)
                                    .child(value)
                            }),
                        ),
                );
            }
            if entry.status == TicketStatus::Failed {
                let label = format!("{} {}", t("Retry", "重试"), entry.ticket.subject);
                row = row
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(p.danger)
                            .child(entry.error.clone()),
                    )
                    .child(
                        face(
                            ("ticket-retry", i),
                            label,
                            32.,
                            radius.into(),
                            Tone::Surface,
                            cx,
                        )
                        .px(px(8.))
                        .child(t("Retry", "重试"))
                        .on_click(cx.listener(move |s, _, _, cx| {
                            s.retry(key.as_ref(), cx);
                        })),
                    );
            }
            queue = queue.child(row);
        }
        let width = if self.content_width.get() > px(0.) {
            self.content_width.get()
        } else {
            w.viewport_size().width
        };
        let pulse = if animate && width >= px(600.) {
            self.entries.iter().rev().find_map(|entry| {
                if entry.status != TicketStatus::Routed || entry.clock.age(cx) >= 0.24 {
                    return None;
                }
                let lane = entry
                    .decision
                    .as_ref()
                    .and_then(|d| self.lanes.iter().position(|l| l.id == d.lane))?;
                let progress = ease(entry.clock.time(0.24, w, cx) / 0.24);
                let destination = (lane as f32 + 0.5) / (self.lanes.len() + 1) as f32;
                Some((progress, destination))
            })
        } else {
            None
        };
        let columns = if width >= px(350.) { 2 } else { 1 };
        let mut lanes = div().w_full().grid().grid_cols(columns).gap(px(8.));
        for (i, lane) in self.lanes.iter().enumerate() {
            let total = self
                .entries
                .iter()
                .filter(|e| {
                    e.status == TicketStatus::Routed
                        && e.decision.as_ref().is_some_and(|d| d.lane == lane.id)
                })
                .count();
            let last_page = total.saturating_sub(1) / TICKETS_PER_PAGE;
            let page = self.lane_pages[i].min(last_page);
            self.lane_pages[i] = page;
            let items: Vec<_> = self
                .entries
                .iter()
                .filter(|e| {
                    e.status == TicketStatus::Routed
                        && e.decision.as_ref().is_some_and(|d| d.lane == lane.id)
                })
                .rev()
                .skip(page * TICKETS_PER_PAGE)
                .take(TICKETS_PER_PAGE)
                .collect();
            let label = format!("{} · {}", lane.label, total);
            let old = self.lane_counts[i].current.parse::<usize>().unwrap_or(0);
            self.lane_counts[i].set(total.to_string(), (total as f32 - old as f32).signum(), cx);
            let count_view = if animate {
                self.lane_counts[i].render(12., w, cx)
            } else {
                div().child(total.to_string())
            };
            let mut panel = div()
                .id(("ticket-lane", i))
                .role(Role::List)
                .aria_label(label.clone())
                .min_w_0()
                .min_h(px(84.))
                .rounded(radius)
                .bg(p.stage)
                .p(px(3.))
                .flex()
                .flex_col()
                .child(
                    div()
                        .h(px(32.))
                        .px(px(8.))
                        .text_size(px(14.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.ink)
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(8.))
                        .child(lane.label.clone())
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(p.secondary)
                                .child(count_view),
                        ),
                );
            for (j, entry) in items.iter().enumerate() {
                let decision = entry.decision.as_ref().unwrap();
                let subject = entry.ticket.subject.clone();
                let landing = if animate {
                    ease(entry.clock.time(0.24, w, cx) / 0.24)
                } else {
                    1.
                };
                panel = panel.child(
                    div()
                        .id(("ticket-lane-item", i * 1000 + j))
                        .debug_selector(move || subject.to_string())
                        .role(Role::ListItem)
                        .relative()
                        .top(px(3. * (1. - landing)))
                        .opacity(landing)
                        .h(px(56.))
                        .px(px(8.))
                        .rounded(radius)
                        .flex()
                        .flex_col()
                        .justify_center()
                        .gap(px(2.))
                        .hover(|s| s.bg(p.ink.opacity(0.05)))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .children(brand_mark(&entry.ticket.channel, 12., cx))
                                .child(
                                    div()
                                        .min_w_0()
                                        .truncate()
                                        .text_size(px(14.))
                                        .line_height(px(20.))
                                        .text_color(p.ink)
                                        .child(entry.ticket.subject.clone()),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w(px(0.))
                                        .text_size(px(12.))
                                        .line_height(px(18.))
                                        .text_color(p.secondary)
                                        .child(decision.assignee.clone()),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .line_height(px(18.))
                                        .text_color(p.faint)
                                        .child(format!("{:.0}%", decision.confidence * 100.)),
                                ),
                        ),
                );
            }
            if last_page > 0 {
                let mut pages = div().flex().flex_wrap().items_center().gap(px(4.));
                if page > 0 {
                    pages = pages.child(
                        face(
                            ("ticket-newer", i),
                            format!("{} {}", t("Newer", "较新"), lane.label),
                            32.,
                            radius.into(),
                            Tone::Clear,
                            cx,
                        )
                        .px(px(6.))
                        .child(t("Newer", "较新"))
                        .on_click(cx.listener(move |s, _, _, cx| s.change_page(i, -1, cx))),
                    );
                }
                pages = pages.child(
                    div()
                        .text_size(px(11.))
                        .text_color(p.secondary)
                        .child(format!("{} / {}", page + 1, last_page + 1)),
                );
                if page < last_page {
                    pages = pages.child(
                        face(
                            ("ticket-older", i),
                            format!("{} {}", t("Older", "较早"), lane.label),
                            32.,
                            radius.into(),
                            Tone::Clear,
                            cx,
                        )
                        .px(px(6.))
                        .child(t("Older", "较早"))
                        .on_click(cx.listener(move |s, _, _, cx| s.change_page(i, 1, cx))),
                    );
                }
                panel = panel.child(pages);
            }
            lanes = lanes.child(panel);
        }
        let review: Vec<_> = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.status == TicketStatus::Review)
            .collect();
        let old_review = self.review_count.current.parse::<usize>().unwrap_or(0);
        self.review_count.set(
            review.len().to_string(),
            (review.len() as f32 - old_review as f32).signum(),
            cx,
        );
        let review_count_view = if animate {
            self.review_count.render(12., w, cx)
        } else {
            div().child(review.len().to_string())
        };
        let mut review_panel = div()
            .id("ticket-review")
            .role(Role::List)
            .aria_label(format!(
                "{} · {}",
                t("Needs review", "待审核"),
                review.len()
            ))
            .min_w_0()
            .min_h(px(84.))
            .rounded(radius)
            .bg(p.stage)
            .p(px(3.))
            .flex()
            .flex_col()
            .child(
                div()
                    .h(px(32.))
                    .px(px(8.))
                    .text_size(px(14.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.ink)
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .child(t("Needs review", "待审核"))
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(p.secondary)
                            .child(review_count_view),
                    ),
            );
        for (i, entry) in review {
            let decision = entry.decision.as_ref().unwrap();
            let ticket = entry.ticket.id.clone();
            let lane = decision.lane.clone();
            let suggested = self
                .lanes
                .iter()
                .find(|l| l.id == lane)
                .map(|l| l.label.clone())
                .unwrap_or_default();
            let mut card = div()
                .id(("ticket-review-item", i))
                .role(Role::ListItem)
                .min_h(px(56.))
                .px(px(8.))
                .py(px(6.))
                .rounded(radius)
                .bg(p.panel)
                .flex()
                .flex_col()
                .justify_center()
                .gap(px(2.))
                .hover(|s| s.bg(p.ink.opacity(0.05)))
                .child(
                    div()
                        .text_size(px(14.))
                        .line_height(px(20.))
                        .text_color(p.ink)
                        .child(entry.ticket.subject.clone()),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .line_height(px(18.))
                        .text_color(p.secondary)
                        .child(format!(
                            "{} · {} · {:.0}%",
                            decision.intent,
                            decision.sentiment,
                            decision.confidence * 100.
                        )),
                );
            let label = format!(
                "{} {}: {}",
                t("Route", "分配"),
                entry.ticket.subject,
                suggested
            );
            card = card.child(
                face(
                    ("ticket-confirm", i),
                    label,
                    32.,
                    radius.into(),
                    Tone::Surface,
                    cx,
                )
                .px(px(8.))
                .child(format!("{} {}", t("Route to", "分配到"), suggested))
                .on_click(cx.listener(move |s, _, _, cx| {
                    s.route_to(ticket.as_ref(), lane.as_ref(), cx);
                })),
            );
            for (j, alternative) in self
                .lanes
                .iter()
                .enumerate()
                .filter(|(_, v)| v.id != decision.lane)
            {
                let ticket = entry.ticket.id.clone();
                let lane = alternative.id.clone();
                let label = format!(
                    "{} {}: {}",
                    t("Route", "分配"),
                    entry.ticket.subject,
                    alternative.label
                );
                card = card.child(
                    face(
                        ("ticket-alternative", i * 1000 + j),
                        label,
                        32.,
                        radius.into(),
                        Tone::Clear,
                        cx,
                    )
                    .px(px(8.))
                    .child(alternative.label.clone())
                    .on_click(cx.listener(move |s, _, _, cx| {
                        s.route_to(ticket.as_ref(), lane.as_ref(), cx);
                    })),
                );
            }
            review_panel = review_panel.child(card);
        }
        lanes = lanes.child(review_panel);
        let content_width = self.content_width.clone();
        let weak = cx.weak_entity();
        let card = div()
            .id("ticket-router")
            .role(Role::Group)
            .aria_label(self.title.clone())
            .w_full()
            .max_w(px(840.))
            .rounded(radius)
            .border_1()
            .border_color(p.line)
            .bg(p.panel)
            .p(px(16.))
            .flex()
            .flex_col()
            .gap(px(12.))
            .child(header)
            .child(queue)
            .when_some(pulse, |v, (progress, destination)| {
                v.child(
                    div()
                        .relative()
                        .w_full()
                        .h(px(3.))
                        .rounded_full()
                        .bg(p.line)
                        .child(
                            div()
                                .absolute()
                                .left_0()
                                .top_0()
                                .h_full()
                                .w(relative((progress * destination).clamp(0., 1.)))
                                .rounded_full()
                                .bg(p.focus),
                        ),
                )
            })
            .child(lanes);
        div()
            .w_full()
            .max_w(px(840.))
            .child(card)
            .on_children_prepainted(move |bounds, _, cx| {
                if let Some(bounds) = bounds.first()
                    && (content_width.get() - (bounds.size.width - px(32.)).max(px(0.))).abs()
                        > px(0.5)
                {
                    content_width.set((bounds.size.width - px(32.)).max(px(0.)));
                    let _ = weak.update(cx, |_, cx| cx.notify());
                }
            })
    }
}

#[cfg(test)]
mod tests {
    use super::{Ticket, TicketDecision, TicketLane, TicketRouter, TicketStatus};
    use gpui::{Modifiers, TestAppContext, VisualTestContext};
    use std::time::Duration;

    fn tick(w: &mut VisualTestContext, ms: u64) {
        w.executor().advance_clock(Duration::from_millis(ms));
        w.run_until_parked();
        w.update(|w, cx| {
            w.simulate_next_frame(cx);
            w.refresh();
        });
        w.run_until_parked();
    }

    #[test]
    fn lane_navigation_keeps_older_routed_tickets_reachable() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let tickets = (0..10)
            .map(|i| Ticket::new(format!("id-{i}"), format!("Ticket {i}"), "Body"))
            .collect();
        let (control, mut w) = app.add_window_view(|_, _| {
            TicketRouter::new(
                "Inbox",
                vec![TicketLane::new("support", "Support")],
                tickets,
            )
        });
        w.update(|_, cx| {
            control.update(cx, |router, cx| {
                for entry in &mut router.entries {
                    entry.status = TicketStatus::Routed;
                    entry.decision = Some(TicketDecision::new("support", 1.));
                }
                cx.notify();
            })
        });
        tick(&mut w, 400);
        assert!(w.debug_bounds("Ticket 9").is_some());
        assert!(w.debug_bounds("Ticket 0").is_none());
        for _ in 0..2 {
            let older = w
                .debug_bounds("Older Support")
                .expect("older tickets")
                .center();
            w.simulate_click(older, Modifiers::default());
            tick(&mut w, 100);
        }
        assert!(w.debug_bounds("Ticket 0").is_some());
        assert!(w.debug_bounds("Ticket 9").is_none());
        let newer = w
            .debug_bounds("Newer Support")
            .expect("newer tickets")
            .center();
        w.simulate_click(newer, Modifiers::default());
        tick(&mut w, 100);
        assert!(w.debug_bounds("Ticket 5").is_some());
    }
    #[test]
    fn classification_review_route_ack_retry_and_stale_results() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let lanes = vec![
            TicketLane::new("support", "Support"),
            TicketLane::new("billing", "Billing"),
        ];
        let tickets = vec![
            Ticket::new("a", "A", "Question"),
            Ticket::new("b", "B", "Invoice"),
            Ticket::new("c", "C", "Request"),
        ];
        let (control, w) = app
            .add_window_view(|_, _| TicketRouter::new("Inbox", lanes, tickets).with_concurrency(2));
        w.update(|_, cx| {
            control.update(cx, |router, cx| {
                router.run(cx);
                let a = router.active_request("a").unwrap();
                let b = router.active_request("b").unwrap();
                assert_eq!(router.status("c"), Some(TicketStatus::Queued));
                assert!(router.classified(a, TicketDecision::new("support", 0.5), cx));
                assert_eq!(router.status("a"), Some(TicketStatus::Review));
                assert!(router.active_request("c").is_some());
                assert!(router.classified(b, TicketDecision::new("billing", 0.95), cx));
                let route = router.active_request("b").unwrap();
                assert_ne!(route, b);
                assert_eq!(router.status("b"), Some(TicketStatus::Routing));
                assert!(router.routed(route, Err("Temporary failure".into()), cx));
                assert_eq!(router.status("b"), Some(TicketStatus::Failed));
                assert!(router.retry("b", cx));
                let retry = router.active_request("b").unwrap();
                assert!(router.routed(retry, Ok(()), cx));
                assert_eq!(router.status("b"), Some(TicketStatus::Routed));
                assert!(router.route_to("a", "billing", cx));
                let assigned = router.active_request("a").unwrap();
                assert!(router.routed(assigned, Ok(()), cx));
                assert_eq!(router.status("a"), Some(TicketStatus::Routed));
                let stale = router.active_request("c").unwrap();
                router.set_ticket(Ticket::new("c", "Updated", "User edit"), cx);
                assert!(!router.classified(stale, TicketDecision::new("support", 1.), cx));
            })
        });
    }
    #[test]
    fn pause_cancels_and_resume_restarts_with_a_new_request() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (control, w) = app.add_window_view(|_, _| {
            TicketRouter::new(
                "Inbox",
                vec![TicketLane::new("support", "Support")],
                vec![Ticket::new("a", "A", "Body")],
            )
        });
        w.update(|_, cx| {
            control.update(cx, |router, cx| {
                router.run(cx);
                let first = router.active_request("a").unwrap();
                router.pause(cx);
                assert_eq!(router.status("a"), Some(TicketStatus::Queued));
                assert!(!router.classified(first, TicketDecision::new("support", 1.), cx));
                router.resume(cx);
                assert_ne!(router.active_request("a"), Some(first));
            })
        });
    }
}
