//! Host-driven support classification and assignment, with review and retry.
use crate::primitives::{
    brand_mark::brand_mark,
    control::UiLanguage,
    family::{Paint, PhaseClock, Tone, disabled_motion, ease, face},
    presentation::ChangingText,
};
use gpui::{prelude::*, *};
use std::{
    cell::Cell,
    collections::{HashMap, HashSet},
    rc::Rc,
};
const CONNECTOR_MIN_WIDTH: Pixels = px(600.);
const ROUTE_TRAVEL_SECONDS: f32 = 0.24;

fn tickets_per_page(width: Pixels) -> usize {
    if width >= CONNECTOR_MIN_WIDTH { 3 } else { 2 }
}

fn activate_key(event: &KeyDownEvent) -> bool {
    !event.is_held && matches!(event.keystroke.key.as_str(), "enter" | "space")
}

fn connector(columns: usize, pulse: Option<(usize, f32)>, p: Paint) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |bounds, (), w, _| {
            let column_width = (bounds.size.width - px(8.) * (columns - 1) as f32) / columns as f32;
            let start = bounds.center().x;
            for i in 0..columns {
                let end = bounds.left() + column_width * (i as f32 + 0.5) + px(8.) * i as f32;
                let sample = |t: f32| {
                    let x = t * t * (3. - 2. * t);
                    point(
                        start + (end - start) * x,
                        bounds.top() + bounds.size.height * t,
                    )
                };
                let draw = |from: f32, to: f32, color, w: &mut Window| {
                    let mut path = PathBuilder::stroke(px(1.));
                    path.move_to(sample(from));
                    for step in 1..=16 {
                        path.line_to(sample(from + (to - from) * step as f32 / 16.));
                    }
                    if let Ok(path) = path.build() {
                        w.paint_path(path, color);
                    }
                };
                draw(0., 1., p.line, w);
                if let Some((_, t)) = pulse.filter(|(lane, _)| *lane == i && i + 1 < columns) {
                    draw((t - 0.14).max(0.), t, p.bright, w);
                }
            }
        },
    )
    .w_full()
    .h(px(24.))
    .flex_none()
}

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
            confidence: finite_confidence(confidence),
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
fn finite_confidence(confidence: f32) -> f32 {
    if confidence.is_finite() {
        confidence.clamp(0., 1.)
    } else {
        0.
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
impl Entry {
    fn new(ticket: Ticket, status: TicketStatus) -> Self {
        Self {
            ticket,
            status,
            request: None,
            decision: None,
            error: "".into(),
            route_failed: false,
            clock: PhaseClock::default(),
        }
    }
}
/// Requests are emitted to the host; a ticket is marked routed only after host confirmation.
pub struct TicketRouter {
    title: SharedString,
    model: Option<SharedString>,
    portraits: HashMap<SharedString, ImageSource>,
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
    review_options: Option<SharedString>,
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
            .map(|ticket| Entry::new(ticket, TicketStatus::Idle))
            .collect();
        let lane_counts = lanes.iter().map(|_| ChangingText::new("0")).collect();
        let lane_pages = vec![0; lanes.len() + 1];
        Self {
            title: title.into(),
            model: None,
            portraits: HashMap::new(),
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
            review_options: None,
            content_width: Default::default(),
        }
    }
    /// Name the host-selected classification model in the header.
    pub fn with_model(mut self, model: impl Into<SharedString>) -> Self {
        self.model = Some(model.into());
        self
    }
    /// Restore acknowledged assignments and pending reviews, oldest first.
    /// Incoming tickets take precedence over duplicate historical IDs.
    pub fn with_history(
        mut self,
        assigned: Vec<(Ticket, TicketDecision)>,
        review: Vec<(Ticket, TicketDecision)>,
    ) -> Self {
        let mut ids: HashSet<_> = self
            .entries
            .iter()
            .map(|entry| entry.ticket.id.clone())
            .collect();
        let restored = assigned
            .into_iter()
            .map(|pair| (pair, TicketStatus::Routed))
            .chain(review.into_iter().map(|pair| (pair, TicketStatus::Review)))
            .filter_map(|((ticket, mut decision), status)| {
                if !self.lanes.iter().any(|lane| lane.id == decision.lane)
                    || !ids.insert(ticket.id.clone())
                {
                    return None;
                }
                decision.confidence = finite_confidence(decision.confidence);
                let mut entry = Entry::new(ticket, status);
                entry.decision = Some(decision);
                Some(entry)
            })
            .collect::<Vec<_>>();
        self.entries.splice(..0, restored);
        Self {
            review_count: ChangingText::new(
                self.entries
                    .iter()
                    .filter(|entry| entry.status == TicketStatus::Review)
                    .count()
                    .to_string(),
            ),
            lane_counts: self
                .lanes
                .iter()
                .map(|lane| {
                    ChangingText::new(
                        self.entries
                            .iter()
                            .filter(|entry| {
                                entry.status == TicketStatus::Routed
                                    && entry
                                        .decision
                                        .as_ref()
                                        .is_some_and(|decision| decision.lane == lane.id)
                            })
                            .count()
                            .to_string(),
                    )
                })
                .collect(),
            ..self
        }
    }
    /// Attach host-provided portraits to assignee names.
    pub fn with_portraits(
        mut self,
        portraits: impl IntoIterator<Item = (SharedString, ImageSource)>,
    ) -> Self {
        self.portraits.extend(portraits);
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
        if self.paused || !self.visible {
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
        self.suspend_work(cx);
        cx.notify();
    }
    fn suspend_work(&mut self, cx: &mut Context<Self>) {
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
    }
    pub fn is_paused(&self) -> bool {
        self.paused
    }
    /// Suspend requests and frame scheduling while the embedded view is off screen.
    /// Showing it again resumes work unless the user paused it.
    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        if self.visible != visible {
            self.visible = visible;
            if visible {
                self.pump(cx);
            } else {
                self.suspend_work(cx);
            }
            cx.notify();
        }
    }
    fn change_page(&mut self, lane: usize, direction: isize, cx: &mut Context<Self>) {
        let Some(page) = self.lane_pages.get_mut(lane) else {
            return;
        };
        let destination = self.lanes.get(lane);
        let total = self
            .entries
            .iter()
            .filter(|entry| match destination {
                Some(spec) => {
                    entry.status == TicketStatus::Routed
                        && entry.decision.as_ref().is_some_and(|d| d.lane == spec.id)
                }
                None => entry.status == TicketStatus::Review,
            })
            .count();
        let last = total.saturating_sub(1) / tickets_per_page(self.content_width.get());
        *page = page.saturating_add_signed(direction).min(last);
        if destination.is_none() {
            self.review_options = None;
        }
        cx.notify();
    }

    fn page_controls(
        &self,
        lane: usize,
        label: &str,
        page: usize,
        last: usize,
        cx: &Context<Self>,
    ) -> Option<Div> {
        if last == 0 {
            return None;
        }
        let l = UiLanguage::current(cx);
        let p = Paint::get(cx);
        let radius = crate::foundation::surface_radius(cx).into();
        let button = |id, caption, delta| {
            face(
                (id, lane),
                format!("{caption} {label}"),
                32.,
                radius,
                Tone::Clear,
                cx,
            )
            .px(px(6.))
            .child(caption)
            .on_click(cx.listener(move |s, _, _, cx| s.change_page(lane, delta, cx)))
            .on_key_down(cx.listener(move |s, event: &KeyDownEvent, w, cx| {
                if activate_key(event) {
                    s.change_page(lane, delta, cx);
                    w.prevent_default();
                    cx.stop_propagation();
                }
            }))
        };
        Some(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap(px(4.))
                .when(page > 0, |row| {
                    row.child(button("ticket-newer", l.text("Newer", "较新"), -1))
                })
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(p.secondary)
                        .child(format!("{} / {}", page + 1, last + 1)),
                )
                .when(page < last, |row| {
                    row.child(button("ticket-older", l.text("Older", "较早"), 1))
                }),
        )
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
            self.entries.push(Entry::new(ticket, TicketStatus::Queued));
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
        decision.confidence = finite_confidence(decision.confidence);
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

    /// Move a failed ticket to manual review without acknowledging an assignment.
    pub fn send_to_review(&mut self, ticket: &str, cx: &mut Context<Self>) -> bool {
        let Some(lane) = self.lanes.first() else {
            return false;
        };
        let Some(entry) = self.entries.iter_mut().find(|entry| {
            entry.ticket.id.as_ref() == ticket && entry.status == TicketStatus::Failed
        }) else {
            return false;
        };
        entry
            .decision
            .get_or_insert_with(|| TicketDecision::new(lane.id.clone(), 0.));
        entry.status = TicketStatus::Review;
        entry.error = "".into();
        entry.route_failed = false;
        entry.clock.restart(cx);
        cx.notify();
        true
    }

    fn toggle_run(&mut self, cx: &mut Context<Self>) {
        if self.paused {
            self.resume(cx);
        } else if self.entries.iter().any(|entry| {
            matches!(
                entry.status,
                TicketStatus::Queued
                    | TicketStatus::Classifying
                    | TicketStatus::RouteQueued
                    | TicketStatus::Routing
            )
        }) {
            self.pause(cx);
        } else {
            self.run(cx);
        }
    }
}
impl Render for TicketRouter {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let l = UiLanguage::current(cx);
        let radius = crate::foundation::surface_radius(cx);
        let animate = self.visible && w.is_visible() && !disabled_motion(cx);
        let t = |en: &'static str, zh: &'static str| l.text(en, zh);
        let width = if self.content_width.get() > px(0.) {
            self.content_width.get()
        } else {
            w.viewport_size().width - px(34.)
        };
        let wide = width >= CONNECTOR_MIN_WIDTH && self.lanes.len() <= 3;
        let completed = self
            .entries
            .iter()
            .filter(|entry| entry.status == TicketStatus::Routed)
            .count();
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
            .gap(px(8.))
            .child(
                div()
                    .min_w_0()
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
                    .child(div().min_w_0().truncate().child(self.title.clone()))
                    .when(width >= px(400.), |header| {
                        header.children(self.model.clone().map(|model| {
                            div()
                                .min_w_0()
                                .truncate()
                                .text_size(px(12.))
                                .text_color(p.secondary)
                                .child(model)
                        }))
                    })
                    .when(wide, |header| {
                        header.child(
                            div()
                                .text_size(px(12.))
                                .text_color(p.faint)
                                .child(format!("{completed} {}", t("routed", "已分配"))),
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
                .child(
                    div()
                        .text_color(p.secondary)
                        .child(if !self.paused && working > 0 {
                            "Ⅱ"
                        } else {
                            "▷"
                        }),
                )
                .child(button_label)
                .on_click(cx.listener(|s, _, _, cx| s.toggle_run(cx)))
                .on_key_down(cx.listener(|s, event: &KeyDownEvent, w, cx| {
                    if activate_key(event) {
                        s.toggle_run(cx);
                        w.prevent_default();
                        cx.stop_propagation();
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
        let departing = if animate {
            self.entries
                .iter()
                .enumerate()
                .filter(|(_, entry)| {
                    matches!(entry.status, TicketStatus::Routed | TicketStatus::Review)
                        && entry.clock.age(cx) < 0.26
                })
                .min_by(|(_, first), (_, second)| {
                    first.clock.age(cx).total_cmp(&second.clock.age(cx))
                })
                .map(|(index, entry)| (index, entry.clock.time(0.26, w, cx)))
        } else {
            None
        };
        let outgoing = departing.filter(|(_, age)| *age < 0.10);
        let featured = outgoing.map(|(index, _)| index).or(active).or_else(|| {
            self.entries.iter().position(|entry| {
                matches!(
                    entry.status,
                    TicketStatus::Idle | TicketStatus::Queued | TicketStatus::RouteQueued
                )
            })
        });
        if featured.is_none()
            && !self
                .entries
                .iter()
                .any(|entry| entry.status == TicketStatus::Failed)
        {
            queue = queue.child(
                div()
                    .id("ticket-caught-up")
                    .role(Role::Status)
                    .w_full()
                    .min_h(px(160.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(radius)
                    .border_1()
                    .border_color(p.line)
                    .text_size(px(13.))
                    .text_color(p.secondary)
                    .child(t("All caught up", "已处理完毕")),
            );
        }
        for (i, entry) in self
            .entries
            .iter()
            .enumerate()
            .filter(|(i, e)| Some(*i) == featured || e.status == TicketStatus::Failed)
        {
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
            let fade = if let Some((index, age)) = departing.filter(|_| Some(i) == featured) {
                if index == i {
                    1. - ease(age / 0.10)
                } else {
                    ease(((age - 0.10) / 0.16).clamp(0., 1.))
                }
            } else if animate && entry.status == TicketStatus::Classifying {
                ease(entry.clock.time(0.22, w, cx) / 0.22)
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
                .w_full()
                .min_h(px(160.))
                .p(px(12.))
                .flex()
                .flex_wrap()
                .items_center()
                .justify_between()
                .gap(px(6.))
                .opacity(fade)
                .relative()
                .top(px(3. * (1. - fade)))
                .child(
                    div()
                        .min_w_0()
                        .w_full()
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
                                    .child(
                                        div()
                                            .min_w_0()
                                            .truncate()
                                            .child(entry.ticket.sender.clone()),
                                    ),
                            )
                        })
                        .child(
                            div()
                                .font_weight(FontWeight::MEDIUM)
                                .truncate()
                                .child(entry.ticket.subject.clone()),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .line_height(px(18.))
                                .line_clamp(2)
                                .text_color(p.secondary)
                                .child(entry.ticket.body.clone()),
                        ),
                )
                .child(
                    div()
                        .id(("ticket-active-status", i))
                        .role(Role::Status)
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
                let label_age = if animate && entry.status == TicketStatus::Routing {
                    entry.clock.time(0.15, w, cx)
                } else {
                    0.15
                };
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
                            .enumerate()
                            .filter(|(_, value)| !value.is_empty())
                            .map(|(index, value)| {
                                div()
                                    .px(px(6.))
                                    .py(px(2.))
                                    .rounded(px(4.))
                                    .bg(p.stage)
                                    .opacity(ease(
                                        ((label_age - index as f32 * 0.05) / 0.05).clamp(0., 1.),
                                    ))
                                    .text_size(px(12.))
                                    .text_color(p.secondary)
                                    .child(value)
                            }),
                        ),
                );
                if let Some(lane) = self.lanes.iter().find(|lane| lane.id == decision.lane) {
                    row = row.child(
                        div()
                            .w_full()
                            .text_size(px(12.))
                            .text_color(p.secondary)
                            .min_w_0()
                            .truncate()
                            .child(format!(
                                "→ {}{}",
                                lane.label,
                                if decision.assignee.is_empty() {
                                    String::new()
                                } else {
                                    format!(" · {}", decision.assignee)
                                }
                            )),
                    );
                }
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
                        }))
                        .on_key_down(cx.listener({
                            let key = entry.ticket.id.clone();
                            move |s, event: &KeyDownEvent, w, cx| {
                                if activate_key(event) {
                                    s.retry(key.as_ref(), cx);
                                    w.prevent_default();
                                    cx.stop_propagation();
                                }
                            }
                        })),
                    );
                let send = {
                    let id = entry.ticket.id.clone();
                    move |s: &mut Self, cx: &mut Context<Self>| {
                        s.send_to_review(id.as_ref(), cx);
                    }
                };
                row = row.when(!self.lanes.is_empty(), |row| {
                    row.child(
                        face(
                            ("ticket-manual-review", i),
                            t("Send to review", "送交审核"),
                            32.,
                            radius.into(),
                            Tone::Clear,
                            cx,
                        )
                        .px(px(8.))
                        .child(t("Send to review", "送交审核"))
                        .on_click(cx.listener({
                            let send = send.clone();
                            move |s, _, _, cx| send(s, cx)
                        }))
                        .on_key_down(cx.listener(
                            move |s, event: &KeyDownEvent, w, cx| {
                                if activate_key(event) {
                                    send(s, cx);
                                    w.prevent_default();
                                    cx.stop_propagation();
                                }
                            },
                        )),
                    )
                });
            }
            queue = queue.child(
                div()
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
                    .overflow_hidden()
                    .child(row),
            );
        }
        let pulse = active.and_then(|index| {
            let entry = &self.entries[index];
            if !animate || !wide || entry.status != TicketStatus::Routing {
                return None;
            }
            let decision = entry.decision.as_ref()?;
            let lane = self
                .lanes
                .iter()
                .position(|lane| lane.id == decision.lane)?;
            let age = entry.clock.time(0.15 + ROUTE_TRAVEL_SECONDS, w, cx);
            let phase = (age - 0.15) / ROUTE_TRAVEL_SECONDS;
            (phase > 0. && phase < 1.).then(|| (lane, ease(phase)))
        });
        let columns = if wide {
            (self.lanes.len() + 1).min(4) as u16
        } else if width >= px(256.) {
            2
        } else {
            1
        };
        let per_page = tickets_per_page(width);
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
            let last_page = total.saturating_sub(1) / per_page;
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
                .skip(page * per_page)
                .take(per_page)
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
                .min_h(px(136.))
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
                    ease(entry.clock.time(ROUTE_TRAVEL_SECONDS, w, cx) / ROUTE_TRAVEL_SECONDS)
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
                        .h(px(52.))
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
                                .gap(px(6.))
                                .children(self.portraits.get(&decision.assignee).map(|source| {
                                    img(source.clone()).size(px(14.)).flex_none().rounded_full()
                                }))
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w(px(0.))
                                        .truncate()
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
            lanes = lanes.child(panel.children(self.page_controls(
                i,
                lane.label.as_ref(),
                page,
                last_page,
                cx,
            )));
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
        let review_lane = self.lanes.len();
        let review_last = review.len().saturating_sub(1) / per_page;
        let review_page = self.lane_pages[review_lane].min(review_last);
        self.lane_pages[review_lane] = review_page;
        let review_label = t("Needs review", "待审核");
        let mut review_panel = div()
            .id("ticket-review")
            .role(Role::List)
            .aria_label(format!(
                "{} · {}",
                t("Needs review", "待审核"),
                review.len()
            ))
            .min_w_0()
            .min_h(px(136.))
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
        for (i, entry) in review
            .into_iter()
            .rev()
            .skip(review_page * per_page)
            .take(per_page)
        {
            let decision = entry.decision.as_ref().unwrap();
            let ticket = entry.ticket.id.clone();
            let lane = decision.lane.clone();
            let suggested = self
                .lanes
                .iter()
                .find(|l| l.id == lane)
                .map(|l| l.label.clone())
                .unwrap_or_default();
            let label = format!(
                "{} {}, {:.0}%: {}",
                t("Route", "分配"),
                entry.ticket.subject,
                decision.confidence * 100.,
                suggested
            );
            let assign = {
                let ticket = ticket.clone();
                move |s: &mut Self, cx: &mut Context<Self>| {
                    s.review_options = None;
                    s.route_to(ticket.as_ref(), lane.as_ref(), cx);
                }
            };
            let expanded = self.review_options.as_ref() == Some(&ticket);
            let mut card = div()
                .id(("ticket-review-item", i))
                .role(Role::ListItem)
                .min_w_0()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .min_w_0()
                        .gap(px(2.))
                        .child(
                            face(
                                ("ticket-confirm", i),
                                label,
                                52.,
                                radius.into(),
                                Tone::Clear,
                                cx,
                            )
                            .flex_1()
                            .min_w_0()
                            .px(px(8.))
                            .items_start()
                            .flex_col()
                            .gap(px(2.))
                            .hover(|style| style.bg(p.ink.opacity(0.05)))
                            .child(
                                div()
                                    .w_full()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .children(brand_mark(&entry.ticket.channel, 12., cx))
                                    .child(
                                        div()
                                            .min_w_0()
                                            .truncate()
                                            .child(entry.ticket.subject.clone()),
                                    ),
                            )
                            .child(
                                div()
                                    .w_full()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .gap(px(4.))
                                    .text_size(px(12.))
                                    .text_color(p.secondary)
                                    .child(
                                        div().min_w_0().truncate().child(format!("{}?", suggested)),
                                    )
                                    .child(format!("{:.0}%", decision.confidence * 100.)),
                            )
                            .on_click(cx.listener({
                                let assign = assign.clone();
                                move |s, _, _, cx| assign(s, cx)
                            }))
                            .on_key_down(cx.listener(
                                move |s, event: &KeyDownEvent, w, cx| {
                                    if activate_key(event) {
                                        assign(s, cx);
                                        w.prevent_default();
                                        cx.stop_propagation();
                                    }
                                },
                            )),
                        )
                        .when(self.lanes.len() > 1, |row| {
                            let toggle = {
                                let ticket = ticket.clone();
                                move |s: &mut Self, cx: &mut Context<Self>| {
                                    s.review_options = (s.review_options.as_ref() != Some(&ticket))
                                        .then(|| ticket.clone());
                                    cx.notify();
                                }
                            };
                            row.child(
                                face(
                                    ("ticket-review-options", i),
                                    t("Other destinations", "其他队列"),
                                    28.,
                                    radius.into(),
                                    Tone::Clear,
                                    cx,
                                )
                                .w(px(28.))
                                .text_color(p.secondary)
                                .aria_expanded(expanded)
                                .child("…")
                                .on_click(cx.listener({
                                    let toggle = toggle.clone();
                                    move |s, _, _, cx| toggle(s, cx)
                                }))
                                .on_key_down(cx.listener(
                                    move |s, event: &KeyDownEvent, w, cx| {
                                        if activate_key(event) {
                                            toggle(s, cx);
                                            w.prevent_default();
                                            cx.stop_propagation();
                                        }
                                    },
                                )),
                            )
                        }),
                );
            if expanded {
                let mut alternatives = div().flex().flex_wrap().gap(px(3.)).px(px(5.)).pb(px(5.));
                for (j, alternative) in self
                    .lanes
                    .iter()
                    .enumerate()
                    .filter(|(_, lane)| lane.id != decision.lane)
                {
                    let ticket = ticket.clone();
                    let lane = alternative.id.clone();
                    let route = move |s: &mut Self, cx: &mut Context<Self>| {
                        s.review_options = None;
                        s.route_to(ticket.as_ref(), lane.as_ref(), cx);
                    };
                    alternatives = alternatives.child(
                        face(
                            ("ticket-alternative", i * 1000 + j),
                            alternative.label.clone(),
                            28.,
                            radius.into(),
                            Tone::Surface,
                            cx,
                        )
                        .px(px(6.))
                        .text_size(px(12.))
                        .child(alternative.label.clone())
                        .on_click(cx.listener({
                            let route = route.clone();
                            move |s, _, _, cx| route(s, cx)
                        }))
                        .on_key_down(cx.listener(
                            move |s, event: &KeyDownEvent, w, cx| {
                                if activate_key(event) {
                                    route(s, cx);
                                    w.prevent_default();
                                    cx.stop_propagation();
                                }
                            },
                        )),
                    );
                }
                card = card.child(alternatives);
            }
            review_panel = review_panel.child(card);
        }
        lanes = lanes.child(review_panel.children(self.page_controls(
            review_lane,
            review_label,
            review_page,
            review_last,
            cx,
        )));
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
            .when(wide, |v| v.child(connector(usize::from(columns), pulse, p)))
            .child(lanes);
        div()
            .w_full()
            .max_w(px(840.))
            .child(card)
            .on_children_prepainted(move |bounds, w, _| {
                if let Some(bounds) = bounds.first()
                    && (content_width.get() - (bounds.size.width - px(34.)).max(px(0.))).abs()
                        > px(0.5)
                {
                    content_width.set((bounds.size.width - px(34.)).max(px(0.)));
                    let weak = weak.clone();
                    w.on_next_frame(move |_, cx| {
                        let _ = weak.update(cx, |_, cx| cx.notify());
                    });
                    w.request_animation_frame();
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
        for _ in 0..10 {
            let Some(older) = w.debug_bounds("Older Support") else {
                break;
            };
            w.simulate_click(older.center(), Modifiers::default());
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
        assert!(w.debug_bounds("Ticket 0").is_none());
        assert!(w.debug_bounds("Ticket 1").is_some() || w.debug_bounds("Ticket 2").is_some());
    }

    #[test]
    fn narrow_review_pages_keep_the_oldest_ticket_reachable() {
        use gpui::{px, size};
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let review = (0..3)
            .map(|i| {
                (
                    Ticket::new(format!("review-{i}"), format!("Review {i}"), ""),
                    TicketDecision::new("support", 0.49),
                )
            })
            .collect();
        let (_, mut w) = app.add_window_view(|_, _| {
            TicketRouter::new("Inbox", vec![TicketLane::new("support", "Support")], vec![])
                .with_history(vec![], review)
        });
        w.simulate_resize(size(px(312.), px(780.)));
        tick(&mut w, 400);
        assert!(w.debug_bounds("Route Review 2, 49%: Support").is_some());
        assert!(w.debug_bounds("Route Review 1, 49%: Support").is_some());
        assert!(w.debug_bounds("Route Review 0, 49%: Support").is_none());
        let older = w.debug_bounds("Older Needs review").unwrap().center();
        w.simulate_click(older, Modifiers::default());
        tick(&mut w, 100);
        assert!(w.debug_bounds("Route Review 0, 49%: Support").is_some());
        assert!(w.debug_bounds("Route Review 2, 49%: Support").is_none());
        let newer = w.debug_bounds("Newer Needs review").unwrap().center();
        w.simulate_click(newer, Modifiers::default());
        tick(&mut w, 100);
        assert!(w.debug_bounds("Route Review 2, 49%: Support").is_some());
    }

    #[test]
    fn restored_assignments_survive_visibility_and_failure_recovery() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (control, w) = app.add_window_view(|_, _| {
            TicketRouter::new(
                "Inbox",
                vec![TicketLane::new("support", "Support")],
                vec![Ticket::new("incoming", "New message", "Body")],
            )
            .with_history(
                vec![(
                    Ticket::new("history", "Old message", ""),
                    TicketDecision::new("support", 0.97),
                )],
                vec![(
                    Ticket::new("review", "Old review", ""),
                    TicketDecision::new("support", 0.48),
                )],
            )
        });
        w.update(|_, cx| {
            control.update(cx, |router, cx| {
                router.run(cx);
                assert_eq!(router.status("history"), Some(TicketStatus::Routed));
                assert_eq!(router.status("review"), Some(TicketStatus::Review));
                let original = router.active_request("incoming").unwrap();
                router.set_visible(false, cx);
                assert_eq!(router.status("incoming"), Some(TicketStatus::Queued));
                assert!(!router.failed(original, "Stale failure", cx));
                router.set_visible(true, cx);
                let resumed = router.active_request("incoming").unwrap();
                assert_ne!(resumed, original);
                assert!(router.failed(resumed, "Try again", cx));
                assert!(router.send_to_review("incoming", cx));
                assert_eq!(router.status("incoming"), Some(TicketStatus::Review));
                assert!(router.route_to("incoming", "support", cx));
                assert_eq!(router.status("incoming"), Some(TicketStatus::Routing));
            })
        });
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
