//! An image carousel with pointer, touch, keyboard and optional rotation.
use crate::primitives::{
    family::{Paint, Slide},
    glyph::{Glyph, icon, transformed},
    nav::button,
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc, time::Duration};
use web_time::Instant;

#[derive(Clone)]
pub struct CarouselSlide {
    pub image: ImageSource,
    pub name: SharedString,
    pub detail: Option<SharedString>,
    pub price: Option<SharedString>,
}
impl CarouselSlide {
    pub fn new(image: impl Into<ImageSource>, name: impl Into<SharedString>) -> Self {
        Self {
            image: image.into(),
            name: name.into(),
            detail: None,
            price: None,
        }
    }
    pub fn with_detail(mut self, detail: impl Into<SharedString>) -> Self {
        self.detail = Some(detail.into());
        self
    }
    pub fn with_price(mut self, price: impl Into<SharedString>) -> Self {
        self.price = Some(price.into());
        self
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
/// One slide is 80% of the carousel by default, never wider than 340px. The step adds the 16px gap.
fn slide_width(container: f32, fixed: Option<f32>) -> f32 {
    fixed.unwrap_or_else(|| (container * 0.8).clamp(1., 340.))
}
fn slide_step(container: f32, fixed: Option<f32>) -> f32 {
    slide_width(container, fixed) + 16.
}
/// Height of a slide caption: a 20px title row, 1px gap, then a 16.8px detail row.
fn caption_height(has_detail: bool) -> f32 {
    if has_detail { 37.8 } else { 20. }
}

pub struct Carousel {
    label: SharedString,
    slides: Vec<CarouselSlide>,
    previous_label: SharedString,
    next_label: SharedString,
    start_label: SharedString,
    pause_label: SharedString,
    slide_fixed: Option<f32>,
    interval: Option<Duration>,
    rotating: bool,
    drain_from: Option<Instant>,
    frozen: f32,
    hold: bool,
    hover: bool,
    tick: u64,
    index: usize,
    prev_index: usize,
    position: Slide,
    dot_in: Slide,
    dot_out: Slide,
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
            previous_label: "Previous slide".into(),
            next_label: "Next slide".into(),
            start_label: "Start slide rotation".into(),
            pause_label: "Pause slide rotation".into(),
            slide_fixed: None,
            interval: None,
            rotating: false,
            drain_from: None,
            frozen: 0.,
            hold: false,
            hover: false,
            tick: 0,
            index: 0,
            prev_index: 0,
            position: Slide::new(0.),
            dot_in: Slide::new(22.),
            dot_out: Slide::new(6.),
            drag: None,
            touch: Default::default(),
            focus: cx.focus_handle().tab_stop(true),
            dot_hover: None,
            container_width: Default::default(),
        }
    }
    pub fn with_index(mut self, index: usize) -> Self {
        self.index = index.min(self.slides.len().saturating_sub(1));
        self.prev_index = self.index;
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
    pub fn with_rotation_labels(
        mut self,
        start: impl Into<SharedString>,
        pause: impl Into<SharedString>,
    ) -> Self {
        self.start_label = start.into();
        self.pause_label = pause.into();
        self
    }
    /// Pins every slide to a fixed width instead of the responsive default.
    pub fn with_slide_width(mut self, width: f32) -> Self {
        self.slide_fixed = Some(width.max(1.));
        self
    }
    /// Adds the rotation control; `interval` is the time a slide stays before advancing.
    pub fn with_interval(mut self, interval: Duration) -> Self {
        self.interval = Some(interval.max(Duration::from_millis(500)));
        self
    }
    /// Starts rotating on mount. Requires an interval.
    pub fn autoplay(mut self) -> Self {
        self.rotating = self.interval.is_some();
        self
    }
    pub fn index(&self) -> usize {
        self.index
    }
    pub fn is_rotating(&self) -> bool {
        self.rotating
    }
    pub fn set_index(&mut self, index: usize, cx: &mut Context<Self>) {
        let index = index.min(self.slides.len().saturating_sub(1));
        // A new command supersedes any unfinished drag.
        self.drag = None;
        if index != self.index {
            self.prev_index = self.index;
            self.dot_in.reset(6.);
            self.dot_out.reset(22.);
            self.dot_in.duration(22., 0.3, cx);
            self.dot_out.duration(6., 0.3, cx);
            self.position.duration(index as f32, 0.75, cx);
            self.index = index;
            self.restart_drain(cx);
            cx.emit(CarouselChanged(index));
        }
        cx.notify();
    }
    pub fn set_rotating(&mut self, rotating: bool, cx: &mut Context<Self>) {
        if rotating && self.interval.is_none() {
            return;
        }
        if self.rotating == rotating {
            return;
        }
        self.rotating = rotating;
        if rotating {
            self.frozen = 0.;
            self.drain_from = Some(cx.background_executor().now());
            self.schedule_tick(cx);
        }
        cx.notify();
    }
    fn restart_drain(&mut self, cx: &mut Context<Self>) {
        self.frozen = 0.;
        if self.rotating && !self.hold {
            self.drain_from = Some(cx.background_executor().now());
        } else {
            self.drain_from = None;
        }
    }
    fn drain_progress(&self, cx: &App) -> f32 {
        let Some(interval) = self.interval else {
            return 0.;
        };
        if !self.rotating {
            0.
        } else if self.hold || self.drain_from.is_none() {
            self.frozen
        } else {
            let now = cx.background_executor().now();
            let elapsed = now
                .saturating_duration_since(self.drain_from.unwrap_or(now))
                .as_secs_f32();
            (elapsed / interval.as_secs_f32()).clamp(0., 1.)
        }
    }
    fn set_hold(&mut self, hold: bool, cx: &mut Context<Self>) {
        if hold == self.hold {
            return;
        }
        if hold {
            self.frozen = self.drain_progress(cx);
            self.drain_from = None;
        } else {
            let Some(interval) = self.interval else {
                self.hold = false;
                return;
            };
            let now = cx.background_executor().now();
            self.drain_from =
                Some(now - Duration::from_secs_f32(self.frozen * interval.as_secs_f32()));
        }
        self.hold = hold;
        cx.notify();
    }
    fn schedule_tick(&mut self, cx: &mut Context<Self>) {
        let Some(interval) = self.interval else {
            return;
        };
        self.tick += 1;
        let ticket = self.tick;
        cx.spawn(async move |s, cx| {
            cx.background_executor().timer(interval).await;
            let _ = s.update(cx, |v, cx| {
                if v.tick == ticket && v.rotating {
                    v.on_tick(cx);
                }
            });
        })
        .detach();
    }
    fn on_tick(&mut self, cx: &mut Context<Self>) {
        self.schedule_tick(cx);
        if self.hold || !self.rotating {
            return;
        }
        let next = if self.index + 1 >= self.slides.len() {
            0
        } else {
            self.index + 1
        };
        let changed = next != self.index;
        let target = next;
        self.drag = None;
        self.prev_index = self.index;
        self.dot_in.reset(6.);
        self.dot_out.reset(22.);
        self.dot_in.duration(22., 0.3, cx);
        self.dot_out.duration(6., 0.3, cx);
        self.position.duration(target as f32, 0.75, cx);
        self.index = target;
        self.restart_drain(cx);
        if changed {
            cx.emit(CarouselChanged(target));
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
        let span = slide_step(f32::from(width), self.slide_fixed);
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
        let step = slide_step(
            f32::from(self.container_width.get()).max(1.),
            self.slide_fixed,
        );
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
        let clamped = target.min(self.slides.len().saturating_sub(1));
        let changed = clamped != self.index;
        self.dot_in.reset(6.);
        self.dot_out.reset(22.);
        self.dot_in.duration(22., 0.3, cx);
        self.dot_out.duration(6., 0.3, cx);
        self.position
            .release(clamped as f32, -(speed / step), 0.75, cx);
        if changed {
            self.prev_index = self.index;
            self.index = clamped;
            cx.emit(CarouselChanged(clamped));
        }
        self.restart_drain(cx);
        cx.notify();
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
        let slide_w = slide_width(width_f, self.slide_fixed);
        let step = slide_step(width_f, self.slide_fixed);
        let has_detail = self.slides.iter().any(|s| s.detail.is_some());
        let cap_h = caption_height(has_detail);
        let stage_height = slide_w + 12. + cap_h;
        let center = self.position.get(w, cx);
        let hold = self.hover || self.drag.is_some() || self.focus.contains_focused(w, cx);
        self.set_hold(hold, cx);
        let drain = self.drain_progress(cx);
        let rotating_live = self.rotating && !self.hold;
        let cursor = if self.drag.is_some() {
            CursorStyle::ClosedHand
        } else {
            CursorStyle::OpenHand
        };
        let mut viewport = div()
            .relative()
            .w(width)
            .h(px(stage_height))
            .overflow_hidden();
        for (i, slide) in self.slides.iter().enumerate() {
            let delta = i as f32 - center;
            if delta.abs() > 2.6 {
                continue;
            }
            let progress = delta.abs().min(1.);
            let scale = 1. - 0.1 * progress;
            let opacity = 1. - 0.5 * progress;
            let card_w = slide_w * scale;
            let card_h = stage_height * scale;
            let card_x = width_f * 0.5 + delta * step - card_w * 0.5;
            let card_y = (stage_height - card_h) * 0.5;
            let name_size = 14. * scale;
            let name_lh = 15.4 * scale;
            let detail_size = 12. * scale;
            let detail_lh = 16.8 * scale;
            let row_h = 20. * scale;
            let mut caption = div()
                .h(px(cap_h * scale))
                .overflow_hidden()
                .px(px(4. * scale))
                .flex()
                .flex_col()
                .gap(px(1.))
                .child(
                    div()
                        .flex()
                        .items_baseline()
                        .gap(px(12. * scale))
                        .min_h(px(row_h))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .text_size(px(name_size))
                                .line_height(px(name_lh))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.ink)
                                .child(slide.name.clone()),
                        )
                        .when(slide.price.is_some(), |row| {
                            row.child(
                                div()
                                    .flex_none()
                                    .text_size(px(name_size))
                                    .line_height(px(name_lh))
                                    .text_color(p.ink)
                                    .child(slide.price.clone().unwrap_or_default()),
                            )
                        }),
                );
            if let Some(detail) = slide.detail.clone() {
                caption = caption.child(
                    div()
                        .w_full()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_size(px(detail_size))
                        .line_height(px(detail_lh))
                        .text_color(p.faint)
                        .child(detail),
                );
            }
            let color = p.stage;
            let mut card = div()
                .absolute()
                .left(px(card_x))
                .top(px(card_y))
                .w(px(card_w))
                .h(px(card_h))
                .opacity(opacity)
                .flex()
                .flex_col()
                .gap(px(12. * scale))
                .child(
                    div()
                        .relative()
                        .w_full()
                        .h(px(card_w))
                        .overflow_hidden()
                        .bg(p.stage)
                        .child(
                            img(slide.image.clone())
                                .size_full()
                                .object_fit(ObjectFit::Cover),
                        ),
                )
                .child(caption);
            // Reference overflow clip: the corners of the photo are masked in
            // the stage colour, above every layer of the card.
            card = card.child(
                canvas(
                    move |_, _, _| (),
                    move |b, _, w, _| {
                        let o = b.origin;
                        let width_px = f32::from(b.size.width);
                        let height_px = f32::from(b.size.height);
                        let r = 8f32.min(width_px * 0.5);
                        for (sx, sy) in [(-1., -1.), (1., -1.), (-1., 1.), (1., 1.)] {
                            let vx = if sx < 0. { 0. } else { width_px };
                            let vy = if sy < 0. { 0. } else { height_px };
                            let (cx, cy) = (vx - sx * r, vy - sy * r);
                            let mut pts = vec![point(o.x + px(vx), o.y + px(vy))];
                            for i in 0..=8 {
                                let d = i as f32 / 8. * r;
                                let dx = sx * (r * r - d * d).sqrt();
                                let dy = sy * d;
                                pts.push(point(o.x + px(cx + dx), o.y + px(cy + dy)));
                            }
                            let mut path = PathBuilder::fill();
                            path.add_polygon(&pts, true);
                            if let Ok(path) = path.build() {
                                w.paint_path(path, color);
                            }
                        }
                    },
                )
                .absolute()
                .top_0()
                .left_0()
                .w_full()
                .h(px(card_w)),
            );
            viewport = viewport.child(card);
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
        let viewport_hover = cx.listener(|s, over: &bool, _, cx| {
            let next = *over;
            if s.hover != next {
                s.hover = next;
                cx.notify();
            }
        });
        viewport = viewport
            .child(
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
            )
            .when(rotating_live, |v| {
                v.child(
                    canvas(|_, _, _| (), |_, _, w, _| w.request_animation_frame())
                        .absolute()
                        .inset_0(),
                )
            });
        // Indicators: one pill per slide; the active dot stretches to 22px.
        let dot_active = self.dot_in.get(w, cx);
        let dot_old = self.dot_out.get(w, cx);
        let mut indicators = div()
            .id("carousel-tabs")
            .role(Role::TabList)
            .aria_label("Choose a slide")
            .min_w_0()
            .flex()
            .items_center();
        for i in 0..self.slides.len() {
            let is_active = i == self.index;
            let is_old = i == self.prev_index && i != self.index;
            let dot_w = if is_active {
                dot_active
            } else if is_old {
                dot_old
            } else {
                6.
            };
            indicators = indicators.child(
                div()
                    .id(("slide-indicator", i))
                    .role(Role::Tab)
                    .aria_selected(is_active)
                    .tab_index(if is_active { 0 } else { -1 })
                    .aria_label(format!(
                        "{}, {} / {}",
                        self.slides[i].name,
                        i + 1,
                        self.slides.len()
                    ))
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
                            .relative()
                            .h(px(6.))
                            .w(px(dot_w))
                            .rounded_full()
                            .overflow_hidden()
                            .bg(if is_active {
                                p.ink
                            } else if self.dot_hover == Some(i) {
                                p.secondary
                            } else {
                                p.hover
                            })
                            .when(is_active && drain > 0.001, |dot| {
                                dot.child(
                                    div()
                                        .absolute()
                                        .top_0()
                                        .right_0()
                                        .h_full()
                                        .w(px(dot_w * drain))
                                        .bg(p.hover),
                                )
                            }),
                    )
                    .on_click(cx.listener(move |s, _, _, cx| s.set_index(i, cx))),
            );
        }
        let content = div()
            .id("carousel")
            .role(Role::Region)
            .aria_label(self.label.clone())
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
                    .track_focus(&self.focus)
                    .aria_label("Slides")
                    .cursor(cursor)
                    .on_hover(viewport_hover)
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
                    .when(self.interval.is_some(), |row| {
                        row.child(
                            button(
                                "carousel-rotation",
                                if self.rotating {
                                    self.pause_label.clone()
                                } else {
                                    self.start_label.clone()
                                },
                                cx,
                            )
                            .size(px(36.))
                            .rounded_full()
                            .p_0()
                            .border_1()
                            .border_color(p.line)
                            .bg(p.panel)
                            .hover(|s| s.bg(p.tray))
                            .child(if self.rotating {
                                icon(Glyph::Pause, 16., p.ink)
                            } else {
                                icon(Glyph::Play, 16., p.ink)
                            })
                            .on_click(cx.listener(|s, _, _, cx| {
                                s.set_rotating(!s.rotating, cx);
                            })),
                        )
                    })
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
    use super::{caption_height, slide_step, slide_width};

    #[test]
    fn slides_use_the_source_width_and_gap() {
        assert!((slide_width(500., None) - 340.).abs() < 0.001);
        assert!((slide_width(300., None) - 240.).abs() < 0.001);
        assert!((slide_width(500., None) - slide_step(500., None) + 16.).abs() < 0.001);
        assert!((slide_width(500., Some(256.)) - 256.).abs() < 0.001);
    }

    #[test]
    fn captions_add_a_detail_row_when_present() {
        assert!(caption_height(true) > caption_height(false) + 16.);
        assert!((caption_height(true) - 37.8).abs() < 0.001);
    }
}
