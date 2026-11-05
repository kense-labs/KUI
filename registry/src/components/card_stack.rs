//! A decision stack for reviewing cards one at a time.
use crate::primitives::{
    family::{Paint, Slide, disabled_motion},
    nav::button,
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc, time::Duration};

#[derive(Clone)]
pub struct StackCard {
    pub id: SharedString,
    pub title: SharedString,
    pub description: SharedString,
    pub eyebrow: SharedString,
    pub details: Vec<SharedString>,
}
impl StackCard {
    pub fn new(
        id: impl Into<SharedString>,
        title: impl Into<SharedString>,
        description: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            description: description.into(),
            eyebrow: "".into(),
            details: Vec::new(),
        }
    }
    pub fn with_eyebrow(mut self, eyebrow: impl Into<SharedString>) -> Self {
        self.eyebrow = eyebrow.into();
        self
    }
    pub fn with_details(mut self, details: Vec<SharedString>) -> Self {
        self.details = details;
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
struct Drag {
    start: Pixels,
    origin: f32,
    last: Pixels,
    last_time: web_time::Instant,
    speed: f32,
}
pub struct CardStack {
    cards: Vec<StackCard>,
    index: usize,
    history: Vec<Decision>,
    x: Slide,
    drag: Option<Drag>,
    exiting: bool,
    exit_task: Option<Task<()>>,
    touch: Rc<Cell<bool>>,
    focus: FocusHandle,
    container_width: Rc<Cell<Pixels>>,
    label: SharedString,
    skip_label: SharedString,
    choose_label: SharedString,
    undo_label: SharedString,
    done_label: SharedString,
}
impl EventEmitter<StackDecision> for CardStack {}
impl EventEmitter<StackUndo> for CardStack {}
impl CardStack {
    pub fn new(label: impl Into<SharedString>, cards: Vec<StackCard>, cx: &App) -> Self {
        Self {
            cards,
            index: 0,
            history: Vec::new(),
            x: Slide::snappy(0.),
            drag: None,
            exiting: false,
            exit_task: None,
            touch: Default::default(),
            focus: cx.focus_handle().tab_stop(true),
            container_width: Default::default(),
            label: label.into(),
            skip_label: "Skip".into(),
            choose_label: "Choose".into(),
            undo_label: "Undo".into(),
            done_label: "All done".into(),
        }
    }
    pub fn with_labels(
        mut self,
        skip: impl Into<SharedString>,
        choose: impl Into<SharedString>,
        undo: impl Into<SharedString>,
        done: impl Into<SharedString>,
    ) -> Self {
        self.skip_label = skip.into();
        self.choose_label = choose.into();
        self.undo_label = undo.into();
        self.done_label = done.into();
        self
    }
    pub fn index(&self) -> usize {
        self.index
    }
    pub fn decide(&mut self, decision: Decision, cx: &mut Context<Self>) {
        if self.exiting || self.index >= self.cards.len() {
            return;
        }
        self.drag = None;
        if disabled_motion(cx) {
            self.finish_decision(decision, cx);
            return;
        }
        self.exiting = true;
        let reach = stack_width(f32::from(self.container_width.get())) * 1.6;
        let current = self.x.value(cx);
        self.x.duration(
            current
                + if decision == Decision::Skip {
                    -reach
                } else {
                    reach
                },
            0.5,
            cx,
        );
        self.exit_task = Some(cx.spawn(async move |entity, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(500))
                .await;
            let _ = entity.update(cx, |s, cx| s.finish_decision(decision, cx));
        }));
        cx.notify();
    }
    fn finish_decision(&mut self, decision: Decision, cx: &mut Context<Self>) {
        if let Some(card) = self.cards.get(self.index) {
            cx.emit(StackDecision {
                id: card.id.clone(),
                decision,
            });
            self.index += 1;
            self.history.push(decision);
        }
        self.exiting = false;
        self.exit_task = None;
        self.x.reset(0.);
        cx.notify();
    }
    pub fn undo(&mut self, cx: &mut Context<Self>) {
        if !self.exiting && self.index > 0 {
            self.drag = None;
            self.index -= 1;
            let direction = self.history.pop().unwrap_or(Decision::Choose);
            let reach = stack_width(f32::from(self.container_width.get())) * 1.6;
            self.x.reset(if direction == Decision::Skip {
                -reach
            } else {
                reach
            });
            self.x.duration(0., 0.4, cx);
            cx.emit(StackUndo {
                id: self.cards[self.index].id.clone(),
            });
            cx.notify();
        }
    }
    fn begin(&mut self, x: Pixels, w: &mut Window, cx: &mut Context<Self>) {
        if self.exiting || self.index >= self.cards.len() {
            return;
        }
        self.focus.focus(w, cx);
        self.drag = Some(Drag {
            start: x,
            origin: self.x.value(cx),
            last: x,
            last_time: cx.background_executor().now(),
            speed: 0.,
        });
    }
    fn move_pointer(&mut self, x: Pixels, cx: &mut Context<Self>) {
        if let Some(drag) = &mut self.drag {
            let now = cx.background_executor().now();
            let elapsed = now
                .saturating_duration_since(drag.last_time)
                .as_secs_f32()
                .max(0.001);
            drag.speed = f32::from(x - drag.last) / elapsed;
            drag.last = x;
            drag.last_time = now;
            self.x
                .reset(drag.origin + f32::from(x - drag.start));
            cx.notify();
        }
    }
    fn end(&mut self, cancelled: bool, cx: &mut Context<Self>) {
        let Some(drag) = self.drag.take() else {
            return;
        };
        let offset = self.x.value(cx);
        let flick = offset.abs() > 12.
            && drag.speed.abs() > 500.
            && drag.speed.signum() == offset.signum();
        let width = stack_width(f32::from(self.container_width.get()));
        let threshold = (width * 0.35).min(140.);
        let opposing = drag.speed.signum() == -offset.signum() && drag.speed.abs() > 300.;
        let far = offset.abs() > threshold && !opposing;
        if !cancelled && (far || flick) {
            self.decide(
                if offset < 0. {
                    Decision::Skip
                } else {
                    Decision::Choose
                },
                cx,
            );
        } else {
            self.x.duration(0., 0.42, cx);
            cx.notify();
        }
    }
}
fn stack_width(measured: f32) -> f32 {
    if measured > 1. { measured.min(320.) } else { 320. }
}

fn shown_depth(depth: f32, lift: f32) -> f32 {
    if depth <= 0. {
        0.
    } else {
        depth - lift * 0.4
    }
}

fn card_scale(shown: f32) -> f32 {
    1. - shown * 0.05
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
        let measured = self.container_width.get();
        let width = if measured > px(0.) {
            measured
        } else {
            w.viewport_size().width - px(32.)
        }
        .max(px(1.))
        .min(px(320.));
        let x = self.x.get(w, cx);
        let lift = (x.abs() / 160.).clamp(0., 1.);
        let mut stage = div().relative().w(width).h(px(328.));
        for depth in (0..4).rev() {
            if let Some(card) = self.cards.get(self.index + depth) {
                let shown = shown_depth(depth as f32, lift);
                let scale = card_scale(shown);
                let card_h = 304. * scale;
                let card_w = f32::from(width) * scale;
                let offset = if depth == 0 { x } else { 0. };
                let presence = if shown <= 2. {
                    1.
                } else {
                    (3. - shown).clamp(0., 1.)
                };
                stage = stage.child(
                    div()
                        .absolute()
                        .top(px((304. - card_h) + shown * 12.))
                        .left(px((f32::from(width) - card_w) * 0.5 + offset))
                        .w(px(card_w.max(1.)))
                        .h(px(card_h))
                        .opacity(presence)
                        .when(depth == 0 && (x.abs() > 1. || self.exiting), |d| d.shadow_lg())
                        .p_5()
                        .bg(p.panel)
                        .border_1()
                        .border_color(p.line)
                        .rounded(crate::foundation::surface_radius(cx))
                        .flex()
                        .flex_col()
                        .gap_3()
                        .child(
                            div()
                                .text_xs()
                                .text_color(p.secondary)
                                .child(card.eyebrow.clone()),
                        )
                        .child(div().text_xl().text_color(p.ink).child(card.title.clone()))
                        .child(
                            div()
                                .text_sm()
                                .text_color(p.secondary)
                                .child(card.description.clone()),
                        )
                        .children(card.details.iter().map(|detail| {
                            div()
                                .text_sm()
                                .text_color(p.secondary)
                                .child(format!("•  {detail}"))
                        })),
                );
            }
        }
        if self.index < self.cards.len() {
            let active = self.touch.clone();
            let contact = cx.listener(|s, e: &TouchDragEvent, w, cx| match e.phase {
                TouchPhase::Started => s.begin(e.position.x, w, cx),
                TouchPhase::Moved => s.move_pointer(e.position.x, cx),
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
        } else {
            stage = stage.child(
                div()
                    .absolute()
                    .inset_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(p.secondary)
                    .child(self.done_label.clone()),
            );
        }
        let moved = cx.listener(|s, e: &MouseMoveEvent, _, cx| s.move_pointer(e.position.x, cx));
        let released = cx.listener(|s, _: &MouseUpEvent, _, cx| s.end(false, cx));
        let content = div()
            .id("card-stack")
            .role(Role::Region)
            .aria_label(self.label.clone())
            .track_focus(&self.focus)
            .w(width)
            .max_w_full()
            .flex()
            .flex_col()
            .gap(px(20.))
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, _, cx| {
                match e.keystroke.key.as_str() {
                    "left" => s.decide(Decision::Skip, cx),
                    "right" => s.decide(Decision::Choose, cx),
                    "backspace" => s.undo(cx),
                    _ => return,
                }
                cx.stop_propagation();
            }))
            .child(
                div()
                    .relative()
                    .id("card-stack-stage")
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|s, e: &MouseDownEvent, w, cx| s.begin(e.position.x, w, cx)),
                    )
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
                            .hover(|s| s.bg(p.tray))
                            .child(self.skip_label.clone())
                            .on_click(cx.listener(|s, _, _, cx| s.decide(Decision::Skip, cx))),
                    )
                    .child(
                        button("card-undo", self.undo_label.clone(), cx)
                            .h(px(44.))
                            .px(px(16.))
                            .text_size(px(14.))
                            .line_height(px(20.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.secondary)
                            .hover(|s| s.text_color(p.ink))
                            .child(self.undo_label.clone())
                            .on_click(cx.listener(|s, _, _, cx| s.undo(cx))),
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
                            .hover(|s| s.bg(p.bright.blend(black().opacity(0.12))))
                            .child(self.choose_label.clone())
                            .on_click(cx.listener(|s, _, _, cx| s.decide(Decision::Choose, cx))),
                    ),
            );
        constrained(content, self.container_width.clone(), px(320.))
    }
}

#[cfg(test)]
mod tests {
    use super::{card_scale, shown_depth};

    #[test]
    fn cards_peek_and_shrink_like_the_source() {
        assert_eq!(shown_depth(0., 1.), 0.);
        assert!((shown_depth(1., 0.) - 1.).abs() < 0.001);
        assert!((shown_depth(1., 1.) - 0.6).abs() < 0.001);
        assert!((card_scale(1.) - 0.95).abs() < 0.001);
        assert!((card_scale(0.) - 1.).abs() < 0.001);
    }
}
