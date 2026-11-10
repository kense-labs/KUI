//! A decision stack that reviews cards one at a time, with a throw, an undo, and a redeal.
use crate::primitives::{
    family::{Paint, PhaseClock, Slide, disabled_motion, ease},
    glyph::{Glyph, icon},
    nav::button,
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc, time::Duration};
use web_time::Instant;

/// Card height at full scale; the stage adds the stack peek below it.
const CARD_HEIGHT: f32 = 304.;
const STAGE_PEEK: f32 = 24.;
/// Cards rendered in the stack; the last one waits invisibly and fades in as it rises.
const VISIBLE: usize = 4;
/// Each card behind peeks 12px lower and shrinks 5%, landing it 27.2px down.
const SHRINK: f32 = 0.05;
const PEEK: f32 = 12.;
/// How far the cards behind rise, in stack steps, while the top card is dragged.
const LIFT: f32 = 0.4;
const LIFT_DISTANCE: f32 = 160.;
/// Stamps fade in between 16px and 96px of travel in their direction.
const STAMP_DEAD: f32 = 16.;
const STAMP_FULL: f32 = 96.;
/// A release faster than this throws the card even when it has barely moved.
const FLICK: f32 = 500.;
/// Vertical travel is not a decision, so past a free zone it resists like a rubber band.
const FREE_Y: f32 = 48.;
const STRETCH_Y: f32 = 140.;

/// One icon-led fact row at the bottom of a card.
#[derive(Clone)]
pub struct StackFact {
    pub glyph: Glyph,
    pub text: SharedString,
}
impl StackFact {
    pub fn new(glyph: Glyph, text: impl Into<SharedString>) -> Self {
        Self {
            glyph,
            text: text.into(),
        }
    }
}
#[derive(Clone)]
pub struct StackCard {
    pub id: SharedString,
    pub code: SharedString,
    pub date: SharedString,
    pub title: SharedString,
    pub subtitle: SharedString,
    pub pitch: SharedString,
    pub facts: Vec<StackFact>,
}
impl StackCard {
    pub fn new(
        id: impl Into<SharedString>,
        title: impl Into<SharedString>,
        subtitle: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            code: "".into(),
            date: "".into(),
            title: title.into(),
            subtitle: subtitle.into(),
            pitch: "".into(),
            facts: Vec::new(),
        }
    }
    /// Top corners: a code on the left, the dates on the right.
    pub fn with_meta(
        mut self,
        code: impl Into<SharedString>,
        date: impl Into<SharedString>,
    ) -> Self {
        self.code = code.into();
        self.date = date.into();
        self
    }
    pub fn with_pitch(mut self, pitch: impl Into<SharedString>) -> Self {
        self.pitch = pitch.into();
        self
    }
    pub fn with_fact(mut self, fact: StackFact) -> Self {
        self.facts.push(fact);
        self
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    Skip,
    Choose,
}
#[derive(Clone, Debug)]
pub struct StackDecision {
    pub id: SharedString,
    pub decision: Decision,
}
#[derive(Clone, Debug)]
pub struct StackUndo {
    pub id: SharedString,
}
#[derive(Clone, Debug)]
pub struct StackReset {}
struct Drag {
    origin_x: f32,
    origin_y: f32,
    samples: Vec<(Instant, f32, f32)>,
}
pub struct CardStack {
    cards: Vec<StackCard>,
    index: usize,
    history: Vec<Decision>,
    /// Visual stack depth per card, in steps; springs on every decision.
    depths: Vec<Slide>,
    /// Per-card travel from its slot, and opacity while flying.
    dx: Vec<Slide>,
    dy: Vec<Slide>,
    op: Vec<Slide>,
    /// Where each decided card left the stage, for undo and start over.
    exits: Vec<(f32, f32)>,
    drag: Option<Drag>,
    flying: Option<usize>,
    returning: Vec<usize>,
    exit_task: Option<Task<()>>,
    home_task: Option<Task<()>>,
    empty_clock: PhaseClock,
    touch: Rc<Cell<bool>>,
    focus: FocusHandle,
    container_width: Rc<Cell<Pixels>>,
    label: SharedString,
    skip_label: SharedString,
    choose_label: SharedString,
    undo_label: SharedString,
    empty_title: SharedString,
    empty_none: SharedString,
    empty_action: SharedString,
    message: SharedString,
}
impl EventEmitter<StackDecision> for CardStack {}
impl EventEmitter<StackUndo> for CardStack {}
impl EventEmitter<StackReset> for CardStack {}
impl CardStack {
    pub fn new(label: impl Into<SharedString>, cards: Vec<StackCard>, cx: &App) -> Self {
        let count = cards.len();
        Self {
            cards,
            index: 0,
            history: Vec::new(),
            depths: (0..count).map(|j| Slide::new(j as f32)).collect(),
            dx: (0..count).map(|_| Slide::snappy(0.)).collect(),
            dy: (0..count).map(|_| Slide::snappy(0.)).collect(),
            op: (0..count).map(|_| Slide::new(1.)).collect(),
            exits: vec![(0., 0.); count],
            drag: None,
            flying: None,
            returning: Vec::new(),
            exit_task: None,
            home_task: None,
            empty_clock: PhaseClock::default(),
            touch: Default::default(),
            focus: cx.focus_handle().tab_stop(true),
            container_width: Default::default(),
            label: label.into(),
            skip_label: "Skip".into(),
            choose_label: "Shortlist".into(),
            undo_label: "Undo".into(),
            empty_title: "All cards reviewed".into(),
            empty_none: "Nothing shortlisted yet".into(),
            empty_action: "Start over".into(),
            message: "".into(),
        }
    }
    pub fn with_labels(
        mut self,
        skip: impl Into<SharedString>,
        choose: impl Into<SharedString>,
        undo: impl Into<SharedString>,
    ) -> Self {
        self.skip_label = skip.into();
        self.choose_label = choose.into();
        self.undo_label = undo.into();
        self
    }
    /// Empty-state copy: the heading, the note when nothing was kept, and the reset action.
    pub fn with_empty(
        mut self,
        title: impl Into<SharedString>,
        none_note: impl Into<SharedString>,
        action: impl Into<SharedString>,
    ) -> Self {
        self.empty_title = title.into();
        self.empty_none = none_note.into();
        self.empty_action = action.into();
        self
    }
    pub fn index(&self) -> usize {
        self.index
    }
    /// The polite status line mirroring the latest decision, undo, or redeal.
    pub fn message(&self) -> SharedString {
        self.message.clone()
    }
    pub fn decide(&mut self, decision: Decision, cx: &mut Context<Self>) {
        self.decide_with_velocity(decision, decision_flick(decision), cx);
    }
    fn decide_with_velocity(
        &mut self,
        decision: Decision,
        velocity: (f32, f32),
        cx: &mut Context<Self>,
    ) {
        if self.flying.is_some() || self.index >= self.cards.len() {
            return;
        }
        let card = self.index;
        self.drag = None;
        self.history.push(decision);
        self.index += 1;
        let left = self.cards.len() - self.index;
        let outcome = if decision == Decision::Skip {
            &self.skip_label
        } else {
            &self.choose_label
        };
        self.message = format!(
            "{}: {}. {}",
            self.cards[card].title,
            outcome,
            if left > 0 {
                format!("{left} left.")
            } else {
                "All cards reviewed.".into()
            }
        )
        .into();
        if self.index >= self.cards.len() {
            self.empty_clock.restart(cx);
        }
        cx.emit(StackDecision {
            id: self.cards[card].id.clone(),
            decision,
        });
        // The window springs up one step; the thrown card holds its depth while it leaves.
        let index = self.index as f32;
        for (j, depth) in self.depths.iter_mut().enumerate() {
            if j != card {
                depth.duration(j as f32 - index, 0.35, cx);
            }
        }
        if disabled_motion(cx) {
            self.exits[card] = (0., 0.);
            self.flying = None;
            self.dx[card].reset(0.);
            self.dy[card].reset(0.);
            self.op[card].reset(0.);
            cx.notify();
            return;
        }
        let width = stack_width(f32::from(self.container_width.get()));
        // A drag hands its finger speed to the throw; buttons and keys use a default flick.
        let (mut ux, mut uy) = velocity;
        if ux.abs() + uy.abs() < 1. {
            ux = 900. * if decision == Decision::Skip { -1. } else { 1. };
            uy = -120.;
        }
        let length = ux.hypot(uy).max(1.);
        ux /= length;
        uy /= length;
        if ux.abs() < 0.45 {
            let side = if uy >= 0. { -1. } else { 1. };
            ux = 0.45 * if decision == Decision::Skip { -1. } else { 1. };
            uy = side * (1.0_f32 - 0.45 * 0.45).sqrt();
        }
        let reach = width * 1.6;
        let now_x = self.dx[card].value(cx);
        let now_y = self.dy[card].value(cx);
        let exit = (now_x + ux * reach, now_y + uy * reach);
        self.exits[card] = exit;
        self.flying = Some(card);
        self.dx[card].duration(exit.0, 0.5, cx);
        self.dy[card].duration(exit.1, 0.5, cx);
        self.op[card].duration(0., 0.22, cx);
        self.exit_task = Some(cx.spawn(async move |entity, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(500))
                .await;
            let _ = entity.update(cx, |s, cx| {
                if s.flying == Some(card) {
                    s.flying = None;
                    s.exit_task = None;
                    cx.notify();
                }
            });
        }));
        cx.notify();
    }
    pub fn undo(&mut self, cx: &mut Context<Self>) {
        if self.flying.is_some() || self.index == 0 || self.history.pop().is_none() {
            return;
        }
        self.index -= 1;
        let card = self.index;
        self.message = format!("{} restored.", self.cards[card].title).into();
        let index = self.index as f32;
        for (j, depth) in self.depths.iter_mut().enumerate() {
            depth.duration(j as f32 - index, 0.35, cx);
        }
        // Fly home from where the card left the stage, fading in as it arrives.
        let (ex, ey) = self.exits[card];
        self.dx[card].reset(ex);
        self.dy[card].reset(ey);
        self.op[card].reset(0.);
        if disabled_motion(cx) {
            self.dx[card].reset(0.);
            self.dy[card].reset(0.);
            self.op[card].reset(1.);
        } else {
            self.returning = vec![card];
            self.dx[card].duration(0., 0.4, cx);
            self.dy[card].duration(0., 0.4, cx);
            self.op[card].duration(1., 0.15, cx);
            self.home_task = Some(cx.spawn(async move |entity, cx| {
                cx.background_executor()
                    .timer(Duration::from_millis(450))
                    .await;
                let _ = entity.update(cx, |s, cx| {
                    s.returning.clear();
                    s.home_task = None;
                    cx.notify();
                });
            }));
        }
        cx.emit(StackUndo {
            id: self.cards[card].id.clone(),
        });
        cx.notify();
    }
    pub fn reset(&mut self, cx: &mut Context<Self>) {
        if self.history.is_empty() {
            return;
        }
        self.index = 0;
        self.history.clear();
        self.message = "All cards are back in the stack.".into();
        let count = VISIBLE.min(self.cards.len());
        for (j, depth) in self.depths.iter_mut().enumerate() {
            depth.reset(j as f32);
        }
        self.returning.clear();
        if disabled_motion(cx) {
            for j in 0..self.cards.len() {
                self.dx[j].reset(0.);
                self.dy[j].reset(0.);
                self.op[j].reset(1.);
            }
        } else {
            // The deck deals itself back: the deepest card returns first, the top card last.
            for j in 0..count {
                let (ex, ey) = self.exits[j];
                self.dx[j].reset(ex);
                self.dy[j].reset(ey);
                self.op[j].reset(0.);
                let delay = (count - 1 - j) as f32 * 0.06;
                self.dx[j].delayed(0., 0.35, delay, cx);
                self.dy[j].delayed(0., 0.35, delay, cx);
                self.op[j].duration(1., 0.2, cx);
                self.returning.push(j);
            }
            for j in count..self.cards.len() {
                self.dx[j].reset(0.);
                self.dy[j].reset(0.);
                self.op[j].reset(1.);
            }
            self.home_task = Some(cx.spawn(async move |entity, cx| {
                cx.background_executor()
                    .timer(Duration::from_millis(750))
                    .await;
                let _ = entity.update(cx, |s, cx| {
                    s.returning.clear();
                    s.home_task = None;
                    cx.notify();
                });
            }));
        }
        cx.emit(StackReset {});
        cx.notify();
    }
    fn begin(&mut self, position: Point<Pixels>, w: &mut Window, cx: &mut Context<Self>) {
        if self.flying.is_some() || self.index >= self.cards.len() {
            return;
        }
        self.focus.focus(w, cx);
        let card = self.index;
        // Grabbing a card flying home hands the stack back to the drag mid-flight.
        self.returning.clear();
        self.home_task = None;
        let now_x = self.dx[card].value(cx);
        let now_y = self.dy[card].value(cx);
        self.dx[card].reset(now_x);
        self.dy[card].reset(now_y);
        self.drag = Some(Drag {
            origin_x: now_x - f32::from(position.x),
            origin_y: unresist_y(now_y) - f32::from(position.y),
            samples: vec![(timestamp(cx), f32::from(position.x), f32::from(position.y))],
        });
        cx.notify();
    }
    fn move_pointer(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) {
        let Some(drag) = &mut self.drag else { return };
        let card = self.index;
        let x = drag.origin_x + f32::from(position.x);
        let y = resist_y(drag.origin_y + f32::from(position.y));
        self.dx[card].reset(x);
        self.dy[card].reset(y);
        drag.samples
            .push((timestamp(cx), f32::from(position.x), f32::from(position.y)));
        if drag.samples.len() > 12 {
            drag.samples.remove(0);
        }
        // The cards behind already start to rise as the top card is dragged away.
        let lift = (x.hypot(y * 0.5) / LIFT_DISTANCE).clamp(0., 1.);
        let index = self.index;
        for j in (index + 1)..(index + VISIBLE).min(self.cards.len()) {
            self.depths[j].set(j as f32 - index as f32 - lift * LIFT, cx);
        }
        cx.notify();
    }
    fn end(&mut self, cancelled: bool, cx: &mut Context<Self>) {
        let Some(drag) = self.drag.take() else {
            return;
        };
        let card = self.index;
        let offset_x = self.dx[card].value(cx);
        let velocity = if cancelled {
            (0., 0.)
        } else {
            velocity_of(&drag.samples, timestamp(cx))
        };
        let width = stack_width(f32::from(self.container_width.get()));
        let direction = offset_x.signum();
        let mut decided = false;
        if !cancelled && direction != 0. {
            let flick = velocity.0.abs() > FLICK
                && velocity.0.signum() == direction
                && offset_x.abs() > 12.;
            let far = offset_x.abs() > (width * 0.35).min(140.)
                && !(velocity.0.signum() == -direction && velocity.0.abs() > 300.);
            decided = flick || far;
        }
        if decided {
            self.decide_with_velocity(
                if offset_x < 0. {
                    Decision::Skip
                } else {
                    Decision::Choose
                },
                velocity,
                cx,
            );
        } else {
            // Below the threshold the card springs home, keeping the release speed.
            let index = self.index;
            for j in (index + 1)..(index + VISIBLE).min(self.cards.len()) {
                self.depths[j].set(j as f32 - index as f32, cx);
            }
            if disabled_motion(cx) {
                self.dx[card].reset(0.);
                self.dy[card].reset(0.);
            } else {
                self.dx[card].release(0., velocity.0, 0.4, cx);
                self.dy[card].release(0., velocity.1, 0.4, cx);
            }
            cx.notify();
        }
    }
}
fn decision_flick(decision: Decision) -> (f32, f32) {
    (
        900. * if decision == Decision::Skip { -1. } else { 1. },
        -120.,
    )
}
fn timestamp(cx: &App) -> Instant {
    cx.background_executor().now()
}
/// Pointer speed over the last 80ms of samples, in pixels per second.
fn velocity_of(samples: &[(Instant, f32, f32)], now: Instant) -> (f32, f32) {
    let window = Duration::from_millis(80);
    let recent: Vec<_> = samples
        .iter()
        .filter(|&&(t, _, _)| now.duration_since(t) <= window)
        .collect();
    if let (Some(first), Some(last)) = (recent.first(), recent.last()) {
        let seconds = last.0.duration_since(first.0).as_secs_f64().max(0.001);
        (
            (last.1 - first.1) / seconds as f32,
            (last.2 - first.2) / seconds as f32,
        )
    } else {
        (0., 0.)
    }
}
fn stack_width(measured: f32) -> f32 {
    if measured > 1. {
        measured.min(320.)
    } else {
        320.
    }
}
fn resist_y(raw: f32) -> f32 {
    let distance = raw.abs();
    if distance <= FREE_Y {
        raw
    } else {
        raw.signum()
            * (FREE_Y + (1. - 1. / ((distance - FREE_Y) * 0.55 / STRETCH_Y + 1.)) * STRETCH_Y)
    }
}
fn unresist_y(shown: f32) -> f32 {
    let over = shown.abs() - FREE_Y;
    if over <= 0. {
        return shown;
    }
    let over = over.min(STRETCH_Y - 0.5);
    shown.signum() * (FREE_Y + over / (STRETCH_Y - over) * STRETCH_Y / 0.55)
}
/// Stamp ramp: dead below `dead`, fully opaque past `full`, in one direction.
fn ramp(value: f32, dead: f32, full: f32) -> f32 {
    ((value - dead) / (full - dead)).clamp(0., 1.)
}
/// Scale, inset, drop and presence for a card `depth` steps deep.
fn slot_geometry(depth: f32, width: f32) -> (f32, f32, f32, f32) {
    let depth = depth.max(0.);
    (
        1. - SHRINK * depth,
        width * SHRINK * depth * 0.5,
        PEEK * depth + CARD_HEIGHT * SHRINK * depth,
        (3. - depth).clamp(0., 1.),
    )
}
/// Darkening for cards sitting deeper in the stack.
fn shade_opacity(depth: f32) -> f32 {
    (depth.max(0.) / 2.).clamp(0., 1.)
}

// Measure the available parent width independently of the component's inner layout.
fn constrained(
    content: impl IntoElement,
    width: Rc<Cell<Pixels>>,
    maximum: Pixels,
) -> impl IntoElement {
    div()
        .relative()
        .w_full()
        .max_w(maximum)
        .child(
            canvas(
                move |bounds, w, _| {
                    if (width.get() - bounds.size.width).abs() > px(0.5) {
                        width.set(bounds.size.width);
                        w.request_animation_frame();
                    }
                },
                |_, _, _, _| (),
            )
            .absolute()
            .inset_0(),
        )
        .child(content)
}
impl Render for CardStack {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let radius = crate::foundation::surface_radius(cx);
        let measured = self.container_width.get();
        let width = if measured > px(0.) {
            measured
        } else {
            w.viewport_size().width - px(32.)
        }
        .max(px(1.))
        .min(px(320.));
        let width_f = f32::from(width);
        let empty = self.index >= self.cards.len() && self.flying.is_none();
        // Paint from the deepest card up; a flying or returning card lands last.
        let end = (self.index + VISIBLE).min(self.cards.len());
        let card_active = |j: usize| self.flying == Some(j) || self.returning.contains(&j);
        let mut order: Vec<usize> = (self.index..end).filter(|&j| !card_active(j)).collect();
        order.sort_by(|&a, &b| {
            self.depths[b]
                .get(w, cx)
                .partial_cmp(&self.depths[a].get(w, cx))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let mut stage = div()
            .relative()
            .w(width)
            .h(px(CARD_HEIGHT + STAGE_PEEK))
            .flex_shrink_0();
        for &j in &order {
            stage = stage.child(self.render_slot(j, &p, width_f, false, w, cx));
        }
        if let Some(flying) = self.flying {
            stage = stage.child(self.render_slot(flying, &p, width_f, true, w, cx));
        }
        let returning: Vec<usize> = self
            .returning
            .clone()
            .into_iter()
            .filter(|&j| self.flying != Some(j))
            .collect();
        for j in returning {
            stage = stage.child(self.render_slot(j, &p, width_f, true, w, cx));
        }
        if empty {
            let t = ease((self.empty_clock.age(cx) / 0.2).clamp(0., 1.));
            let again = button("card-again", self.empty_action.clone(), cx)
                .h(px(36.))
                .px(px(16.))
                .gap(px(8.))
                .border_1()
                .border_color(p.line)
                .bg(p.panel)
                .text_size(px(14.))
                .line_height(px(20.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.ink)
                .hover(|s| s.bg(p.tray))
                .child(icon(Glyph::Undo, 16., p.ink))
                .child(self.empty_action.clone())
                .on_click(cx.listener(|s, _, _, cx| s.reset(cx)));
            stage = stage.child(self.render_empty(&p, radius, width_f, t, again));
        }
        if self.index < self.cards.len() {
            let active = self.touch.clone();
            let contact = cx.listener(|s, e: &TouchDragEvent, w, cx| match e.phase {
                TouchPhase::Started => s.begin(e.position, w, cx),
                TouchPhase::Moved => s.move_pointer(e.position, cx),
                TouchPhase::Ended => s.end(false, cx),
                TouchPhase::Cancelled => s.end(true, cx),
            });
            stage = stage.child(
                canvas(
                    |b, w, _| w.insert_hitbox(b, HitboxBehavior::Normal),
                    move |_, hit, w, _| {
                        w.on_mouse_event(move |e: &TouchDragEvent, phase, w, cx| {
                            if phase.bubble() {
                                if e.phase == TouchPhase::Started {
                                    active.set(hit.is_hovered(w));
                                }
                                if !active.get() {
                                    return;
                                }
                                if matches!(e.phase, TouchPhase::Ended | TouchPhase::Cancelled) {
                                    active.set(false);
                                }
                                if e.phase == TouchPhase::Started {
                                    w.prevent_default();
                                }
                                contact(e, w, cx);
                                cx.stop_propagation();
                            }
                        });
                    },
                )
                .absolute()
                .inset_0(),
            );
        }
        let moved = cx.listener(|s, e: &MouseMoveEvent, _, cx| s.move_pointer(e.position, cx));
        let released = cx.listener(|s, _: &MouseUpEvent, _, cx| s.end(false, cx));
        let pressed = cx.listener(|s, e: &MouseDownEvent, w, cx| s.begin(e.position, w, cx));
        let stack_done = self.index >= self.cards.len();
        let can_undo = self.flying.is_none() && !self.history.is_empty();
        let content = div()
            .id("card-stack")
            .role(Role::Group)
            .aria_label(self.label.clone())
            .track_focus(&self.focus)
            .w(width)
            .max_w_full()
            .flex()
            .flex_col()
            .gap(px(20.))
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, _, cx| {
                let meta = e.keystroke.modifiers.platform || e.keystroke.modifiers.control;
                match e.keystroke.key.as_str() {
                    "left" => s.decide(Decision::Skip, cx),
                    "right" => s.decide(Decision::Choose, cx),
                    "backspace" => s.undo(cx),
                    "z" if meta => s.undo(cx),
                    _ => return,
                }
                cx.stop_propagation();
            }))
            .child(
                div()
                    .relative()
                    .id("card-stack-stage")
                    .on_mouse_down(MouseButton::Left, pressed)
                    .child(stage)
                    .child(
                        canvas(
                            |_, _, _| (),
                            move |_, _, w, _| {
                                w.on_mouse_event(move |e: &MouseMoveEvent, phase, w, cx| {
                                    if phase.capture() {
                                        moved(e, w, cx);
                                    }
                                });
                                w.on_mouse_event(move |e: &MouseUpEvent, phase, w, cx| {
                                    if phase.capture() && e.button == MouseButton::Left {
                                        released(e, w, cx);
                                    }
                                });
                            },
                        )
                        .absolute()
                        .size(px(0.)),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .justify_center()
                    .child(
                        button("card-skip", self.skip_label.clone(), cx)
                            .h(px(44.))
                            .px(px(16.))
                            .gap(px(8.))
                            .border_1()
                            .border_color(p.line)
                            .bg(p.panel)
                            .text_size(px(14.))
                            .line_height(px(20.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .hover(|s| s.bg(p.tray))
                            .when(stack_done, |b| b.opacity(0.42))
                            .child(icon(Glyph::Close, 16., p.ink))
                            .child(self.skip_label.clone())
                            .on_click(cx.listener(|s, _, _, cx| {
                                if s.index < s.cards.len() {
                                    s.decide(Decision::Skip, cx);
                                }
                            })),
                    )
                    .child(
                        button("card-undo", self.undo_label.clone(), cx)
                            .h(px(44.))
                            .px(px(16.))
                            .gap(px(8.))
                            .text_size(px(14.))
                            .line_height(px(20.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.secondary)
                            .hover(|s| s.text_color(p.ink))
                            .when(!can_undo, |b| b.opacity(0.42))
                            .child(icon(Glyph::Undo, 16., p.secondary))
                            .child(self.undo_label.clone())
                            .on_click(cx.listener(|s, _, _, cx| {
                                if !s.history.is_empty() && s.flying.is_none() {
                                    s.undo(cx);
                                }
                            })),
                    )
                    .child(
                        button("card-choose", self.choose_label.clone(), cx)
                            .h(px(44.))
                            .px(px(16.))
                            .gap(px(8.))
                            .border_1()
                            .border_color(p.bright)
                            .bg(p.bright)
                            .text_size(px(14.))
                            .line_height(px(20.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.inverse)
                            .hover(|s| s.bg(p.primary_hover(0.12)))
                            .when(stack_done, |b| b.opacity(0.42))
                            .child(icon(Glyph::Check, 16., p.inverse))
                            .child(self.choose_label.clone())
                            .on_click(cx.listener(|s, _, _, cx| {
                                if s.index < s.cards.len() {
                                    s.decide(Decision::Choose, cx);
                                }
                            })),
                    ),
            );
        constrained(content, self.container_width.clone(), px(320.))
    }
}
impl CardStack {
    fn render_slot(
        &self,
        j: usize,
        p: &Paint,
        width_f: f32,
        active: bool,
        w: &mut Window,
        cx: &App,
    ) -> impl IntoElement {
        let radius = crate::foundation::surface_radius(cx);
        let card = &self.cards[j];
        let raw_depth = if active {
            0.
        } else {
            self.depths[j].get(w, cx)
        };
        let (scale, inset, drop, presence) = slot_geometry(raw_depth, width_f);
        let card_w = width_f * scale;
        let card_h = CARD_HEIGHT * scale;
        let is_top = self.index == j && !active;
        let (offset_x, offset_y) = if is_top || active {
            (self.dx[j].get(w, cx), self.dy[j].get(w, cx))
        } else {
            (0., 0.)
        };
        let opacity = if active {
            self.op[j].get(w, cx)
        } else {
            presence
        };
        let shade = if is_top || active {
            0.
        } else {
            shade_opacity(raw_depth)
        };
        let keep = ramp(offset_x, STAMP_DEAD, STAMP_FULL);
        let pass = ramp(-offset_x, STAMP_DEAD, STAMP_FULL);
        let s = scale;
        let facts = card
            .facts
            .iter()
            .map(|fact| {
                div()
                    .flex()
                    .items_center()
                    .gap(px(8. * s))
                    .child(icon(fact.glyph, 16. * s, p.faint))
                    .child(
                        div()
                            .text_size(px(14. * s))
                            .line_height(px(19.6 * s))
                            .text_color(p.ink)
                            .child(fact.text.clone()),
                    )
            })
            .collect::<Vec<_>>();
        let face = div()
            .size_full()
            .overflow_hidden()
            .border_1()
            .border_color(p.line)
            .rounded(radius)
            .bg(p.panel)
            .relative()
            .flex()
            .flex_col()
            .px(px(20. * s))
            .py(px(20. * s))
            .when(!card.code.is_empty() || !card.date.is_empty(), |d| {
                d.child(
                    div()
                        .flex()
                        .justify_between()
                        .text_size(px(12. * s))
                        .line_height(px(16. * s))
                        .text_color(p.faint)
                        .child(div().child(card.code.clone()))
                        .child(div().child(card.date.clone())),
                )
            })
            .child(
                div()
                    .mt_auto()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_size(px(36. * s))
                            .line_height(px(39.6 * s))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(card.title.clone()),
                    )
                    .child(
                        div()
                            .mt(px(2. * s))
                            .text_size(px(14. * s))
                            .line_height(px(19.6 * s))
                            .text_color(p.secondary)
                            .child(card.subtitle.clone()),
                    ),
            )
            .when(!card.pitch.is_empty(), |d| {
                d.child(
                    div()
                        .mt(px(12. * s))
                        .text_size(px(14. * s))
                        .line_height(px(19.6 * s))
                        .text_color(p.secondary)
                        .child(card.pitch.clone()),
                )
            })
            .when(!facts.is_empty(), |d| {
                d.child(
                    div()
                        .mt(px(16. * s))
                        .pt(px(16. * s))
                        .border_t_1()
                        .border_color(p.line)
                        .flex()
                        .flex_col()
                        .gap(px(6. * s))
                        .children(facts),
                )
            })
            .when(shade > 0.001, |d| {
                d.child(div().absolute().inset_0().bg(black().opacity(0.22 * shade)))
            });
        div()
            .absolute()
            .top(px(drop + offset_y))
            .left(px(inset + offset_x))
            .w(px(card_w.max(1.)))
            .h(px(card_h.max(1.)))
            .opacity(opacity)
            .id(card.id.clone())
            .when(is_top, |d| {
                d.role(Role::Group).aria_label(format!(
                    "{}, {} of {}",
                    card.title,
                    j + 1,
                    self.cards.len()
                ))
            })
            .child(face)
            // Drag stamps: keep on the top-left corner, pass on the top-right.
            .child(
                div()
                    .absolute()
                    .top(px(16.))
                    .left(px(16.))
                    .h(px(32.))
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .pl(px(10.))
                    .pr(px(12.))
                    .border_1()
                    .border_color(p.success.opacity(0.45).blend(p.line.opacity(0.55)))
                    .rounded(radius)
                    .bg(p.panel.blend(p.success.opacity(0.10)))
                    .opacity(keep)
                    .child(icon(Glyph::Check, 16., p.success))
                    .child(
                        div()
                            .text_size(px(14.))
                            .line_height(px(20.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(self.choose_label.clone()),
                    ),
            )
            .child(
                div()
                    .absolute()
                    .top(px(16.))
                    .right(px(16.))
                    .h(px(32.))
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .pl(px(10.))
                    .pr(px(12.))
                    .border_1()
                    .border_color(p.line)
                    .rounded(radius)
                    .bg(p.panel)
                    .opacity(pass)
                    .child(icon(Glyph::Close, 16., p.secondary))
                    .child(
                        div()
                            .text_size(px(14.))
                            .line_height(px(20.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(self.skip_label.clone()),
                    ),
            )
    }
    fn render_empty(
        &self,
        p: &Paint,
        radius: Pixels,
        width_f: f32,
        t: f32,
        again: impl IntoElement,
    ) -> impl IntoElement {
        let kept: Vec<String> = self
            .history
            .iter()
            .enumerate()
            .filter(|&(_, decision)| decision == &Decision::Choose)
            .map(|(j, _)| self.cards[j].title.to_string())
            .collect();
        let note = if kept.is_empty() {
            self.empty_none.clone().to_string()
        } else {
            kept.join(", ")
        };
        div()
            .absolute()
            .top(px(8. * (1. - t)))
            .left_0()
            .w(px(width_f))
            .h(px(CARD_HEIGHT))
            .opacity(t)
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(12.))
            .px(px(24.))
            .border_1()
            .border_color(p.line)
            .rounded(radius)
            .bg(p.panel)
            .child(
                div()
                    .text_size(px(16.))
                    .line_height(px(22.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.ink)
                    .child(self.empty_title.clone()),
            )
            .child(
                div()
                    .max_w(px(240.))
                    .text_center()
                    .text_size(px(14.))
                    .line_height(px(19.6))
                    .text_color(p.secondary)
                    .child(note),
            )
            .child(again)
    }
}

#[cfg(test)]
mod tests {
    use super::{ramp, resist_y, shade_opacity, slot_geometry, unresist_y};

    #[test]
    fn cards_peek_shrink_and_fade_like_the_source() {
        let (s1, i1, d1, a1) = slot_geometry(1., 320.);
        assert!((s1 - 0.95).abs() < 0.001);
        assert!((i1 - 8.).abs() < 0.001);
        assert!((d1 - 27.2).abs() < 0.001);
        assert!((a1 - 1.).abs() < 0.001);
        let (s2, _, d2, _) = slot_geometry(2., 320.);
        assert!((s2 - 0.90).abs() < 0.001);
        assert!((d2 - 54.4).abs() < 0.001);
        let (_, _, _, a3) = slot_geometry(3., 320.);
        assert_eq!(a3, 0.);
        assert_eq!(shade_opacity(0.), 0.);
        assert!((shade_opacity(2.) - 1.).abs() < 0.001);
    }
    #[test]
    fn stamps_ramp_in_one_direction_each() {
        assert_eq!(ramp(0., 16., 96.), 0.);
        assert_eq!(ramp(16., 16., 96.), 0.);
        assert!((ramp(56., 16., 96.) - 0.5).abs() < 0.001);
        assert_eq!(ramp(96., 16., 96.), 1.);
        // Pass mirrors the same ramp on the other side of zero.
        assert_eq!(ramp(-56., 16., 96.), 0.);
        assert_eq!(ramp(-300., 16., 96.), 0.);
    }
    #[test]
    fn vertical_travel_resists_like_a_rubber_band() {
        assert_eq!(resist_y(30.), 30.);
        let resisted = resist_y(200.);
        assert!(resisted > 48. && resisted < 140.);
        assert!((unresist_y(resisted) - 200.).abs() < 0.5);
        assert_eq!(unresist_y(20.), 20.);
    }
}
