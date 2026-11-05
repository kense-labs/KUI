//! An image carousel with pointer, touch and keyboard navigation.
use crate::primitives::{
    family::{Paint, Slide},
    glyph::{Glyph, icon, transformed},
    nav::button,
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

#[derive(Clone)]
pub struct CarouselSlide {
    pub image: ImageSource,
    pub label: SharedString,
}
impl CarouselSlide {
    pub fn new(image: impl Into<ImageSource>, label: impl Into<SharedString>) -> Self {
        Self {
            image: image.into(),
            label: label.into(),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CarouselChanged(pub usize);
struct Drag {
    start: Pixels,
    origin: f32,
    last: Pixels,
    last_time: Instant,
    speed: f32,
}
/// One slide is 80% of the carousel, and never wider than 340px. The step adds the 16px gap.
fn slide_width(container: f32) -> f32 {
    (container * 0.8).min(340.).max(1.)
}
fn slide_step(container: f32) -> f32 {
    slide_width(container) + 16.
}

pub struct Carousel {
    label: SharedString,
    slides: Vec<CarouselSlide>,
    previous_label: SharedString,
    next_label: SharedString,
    index: usize,
    position: Slide,
    drag: Option<Drag>,
    touch: Rc<Cell<bool>>,
    focus: FocusHandle,
    dot_hover: Option<usize>,
    container_width: Rc<Cell<Pixels>>,
}
impl EventEmitter<CarouselChanged> for Carousel {}
impl Carousel {
    pub fn new(label: impl Into<SharedString>, slides: Vec<CarouselSlide>, cx: &App) -> Self {
        Self {
            label: label.into(),
            slides,
            previous_label: "Previous".into(),
            next_label: "Next".into(),
            index: 0,
            position: Slide::new(0.),
            drag: None,
            touch: Default::default(),
            focus: cx.focus_handle().tab_stop(true),
            dot_hover: None,
            container_width: Default::default(),
        }
    }
    pub fn with_index(mut self, index: usize) -> Self {
        self.index = index.min(self.slides.len().saturating_sub(1));
        self.position.reset(self.index as f32);
        self
    }
    pub fn with_action_labels(
        mut self,
        previous: impl Into<SharedString>,
        next: impl Into<SharedString>,
    ) -> Self {
        self.previous_label = previous.into();
        self.next_label = next.into();
        self
    }
    pub fn index(&self) -> usize {
        self.index
    }
    pub fn set_index(&mut self, index: usize, cx: &mut Context<Self>) {
        let index = index.min(self.slides.len().saturating_sub(1));
        // A new command supersedes any unfinished drag.
        self.drag = None;
        self.position.duration(index as f32, 0.4, cx);
        if self.index != index {
            self.index = index;
            cx.emit(CarouselChanged(index));
        }
        cx.notify();
    }
    fn begin(&mut self, x: Pixels, w: &mut Window, cx: &mut Context<Self>) {
        self.focus.focus(w, cx);
        self.drag = Some(Drag {
            start: x,
            origin: self.position.value(cx),
            last: x,
            last_time: cx.background_executor().now(),
            speed: 0.,
        });
    }
    fn move_pointer(&mut self, x: Pixels, width: Pixels, cx: &mut Context<Self>) {
        let Some(drag) = self.drag.as_mut() else {
            return;
        };
        let now = cx.background_executor().now();
        let elapsed = now
            .saturating_duration_since(drag.last_time)
            .as_secs_f32()
            .max(0.001);
        drag.speed = f32::from(x - drag.last) / elapsed;
        drag.last = x;
        drag.last_time = now;
        let span = slide_step(f32::from(width));
        let value = (drag.origin - f32::from(x - drag.start) / span)
            .clamp(0., self.slides.len().saturating_sub(1) as f32);
        self.position.reset(value);
        cx.notify();
    }
    fn end(&mut self, cancelled: bool, cx: &mut Context<Self>) {
        let Some(drag) = self.drag.take() else {
            return;
        };
        let current = self.position.value(cx);
        let speed = if cx
            .background_executor()
            .now()
            .saturating_duration_since(drag.last_time)
            .as_secs_f32()
            > 0.12
        {
            0.
        } else {
            drag.speed
        };
        let step = slide_step(f32::from(self.container_width.get()).max(1.));
        let carry = speed * 0.199 / step;
        let mut landed = current - carry;
        if !cancelled && speed.abs() > 360. {
            let direction = if speed < 0. { 1. } else { -1. };
            if (landed - drag.origin) * direction < 1. {
                landed = drag.origin + direction;
            }
        }
        let target = if cancelled {
            self.index
        } else {
            landed.round().max(0.) as usize
        };
        self.set_index(target, cx);
    }
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
impl Render for Carousel {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let measured = self.container_width.get();
        let width = if measured > px(0.) {
            measured
        } else {
            w.viewport_size().width - px(32.)
        }
        .max(px(1.));
        let width_f = f32::from(width);
        let slide_w = slide_width(width_f);
        let step = slide_step(width_f);
        let stage_height = slide_w * 0.75;
        let center = self.position.get(w, cx);
        let mut viewport = div()
            .relative()
            .w(width)
            .h(px(stage_height))
            .overflow_hidden();
        for (i, slide) in self.slides.iter().enumerate() {
            let delta = i as f32 - center;
            if delta.abs() > 2.4 {
                continue;
            }
            let scale = (1. - 0.1 * delta.abs()).max(0.7);
            let item_w = slide_w * scale;
            let item_h = stage_height * scale;
            viewport = viewport.child(
                div()
                    .absolute()
                    .left(px(width_f * 0.5 + delta * step - item_w * 0.5))
                    .top(px((stage_height - item_h) * 0.5))
                    .w(px(item_w))
                    .h(px(item_h))
                    .overflow_hidden()
                    .rounded(crate::foundation::surface_radius(cx))
                    .opacity((1. - 0.5 * delta.abs()).clamp(0., 1.))
                    .child(
                        img(slide.image.clone())
                            .size_full()
                            .object_fit(ObjectFit::Cover),
                    )
                    .child(
                        div()
                            .absolute()
                            .bottom_0()
                            .left_0()
                            .right_0()
                            .p_3()
                            .bg(p.panel.opacity(0.82))
                            .text_color(p.ink)
                            .child(slide.label.clone()),
                    ),
            );
        }
        let moved = cx
            .listener(move |s, e: &MouseMoveEvent, _, cx| s.move_pointer(e.position.x, width, cx));
        let released = cx.listener(|s, _: &MouseUpEvent, _, cx| s.end(false, cx));
        let active = self.touch.clone();
        let contact = cx.listener(move |s, e: &TouchDragEvent, w, cx| match e.phase {
            TouchPhase::Started => s.begin(e.position.x, w, cx),
            TouchPhase::Moved => s.move_pointer(e.position.x, width, cx),
            TouchPhase::Ended => s.end(false, cx),
            TouchPhase::Cancelled => s.end(true, cx),
        });
        viewport = viewport.child(
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
        let mut indicators = div()
            .min_w_0()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(0.));
        for i in 0..self.slides.len() {
            indicators = indicators.child(
                div()
                    .id(("slide-indicator", i))
                    .role(Role::Button)
                    .tab_index(0)
                    .aria_label(format!("{} {} / {}", self.label, i + 1, self.slides.len()))
                    .h(px(36.))
                    .px(px(5.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .on_hover(cx.listener(move |s, over: &bool, _, cx| {
                        let next = if *over { Some(i) } else { None };
                        if s.dot_hover != next && (s.dot_hover == Some(i) || *over) {
                            s.dot_hover = next;
                            cx.notify();
                        }
                    }))
                    .child(
                        div()
                            .h(px(6.))
                            .w(px(if i == self.index { 22. } else { 6. }))
                            .rounded_full()
                            .bg(if i == self.index {
                                p.ink
                            } else if self.dot_hover == Some(i) {
                                p.secondary
                            } else {
                                p.faint
                            }),
                    )
                    .on_click(cx.listener(move |s, _, _, cx| s.set_index(i, cx))),
            );
        }
        let content = div()
            .id("carousel")
            .role(Role::Region)
            .aria_label(self.label.clone())
            .track_focus(&self.focus)
            .w(width)
            .flex()
            .flex_col()
            .gap(px(16.))
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, _, cx| {
                let target = match e.keystroke.key.as_str() {
                    "left" => s.index.saturating_sub(1),
                    "right" => s.index.saturating_add(1),
                    "home" => 0,
                    "end" => s.slides.len().saturating_sub(1),
                    _ => return,
                };
                s.set_index(target, cx);
                cx.stop_propagation();
            }))
            .child(
                div()
                    .relative()
                    .id("carousel-viewport")
                    .cursor_pointer()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|s, e: &MouseDownEvent, w, cx| s.begin(e.position.x, w, cx)),
                    )
                    .child(viewport)
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
                    .w_full()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .px(px((width_f - slide_w) * 0.5))
                    .child(indicators)
                    .child(
                        div()
                            .ml_auto()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(
                                button("carousel-previous", self.previous_label.clone(), cx)
                                    .size(px(36.))
                                    .rounded_full()
                                    .p_0()
                                    .border_1()
                                    .border_color(p.line)
                                    .bg(p.panel)
                                    .hover(|s| s.bg(p.tray))
                                    .when(self.index == 0, |b| b.opacity(0.5).text_color(p.faint))
                                    .child(transformed(
                                        Glyph::Arrow,
                                        16.,
                                        p.ink,
                                        std::f32::consts::PI,
                                        1.,
                                        (0., 0.),
                                        1.,
                                    ))
                                    .on_click(cx.listener(|s, _, _, cx| {
                                        s.set_index(s.index.saturating_sub(1), cx)
                                    })),
                            )
                            .child(
                                button("carousel-next", self.next_label.clone(), cx)
                                    .size(px(36.))
                                    .rounded_full()
                                    .p_0()
                                    .border_1()
                                    .border_color(p.line)
                                    .bg(p.panel)
                                    .hover(|s| s.bg(p.tray))
                                    .when(self.index + 1 >= self.slides.len(), |b| {
                                        b.opacity(0.5).text_color(p.faint)
                                    })
                                    .rounded_full()
                                    .p_0()
                                    .child(icon(Glyph::Arrow, 16., p.ink))
                                    .on_click(cx.listener(|s, _, _, cx| {
                                        s.set_index(s.index.saturating_add(1), cx)
                                    })),
                            ),
                    ),
            );
        constrained(content, self.container_width.clone(), px(4096.))
    }
}

#[cfg(test)]
mod tests {
    use super::{slide_step, slide_width};

    #[test]
    fn slides_use_the_source_width_and_gap() {
        assert!((slide_width(500.) - 340.).abs() < 0.001);
        assert!((slide_width(300.) - 240.).abs() < 0.001);
        assert!((slide_step(500.) - 356.).abs() < 0.001);
    }
}
