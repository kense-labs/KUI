//! Compare two host-supplied images using a keyboard and pointer divider.
use crate::primitives::family::{Paint, Slide, measure};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CompareOrientation {
    #[default]
    Horizontal,
    Vertical,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CompareChanged(pub f32);
pub struct ImageCompare {
    before: ImageSource,
    after: ImageSource,
    label: SharedString,
    captions: Option<[SharedString; 2]>,
    orientation: CompareOrientation,
    ratio: f32,
    value: f32,
    position: Slide,
    handle_w: Slide,
    handle_h: Slide,
    chevron: Slide,
    bounds: Rc<Cell<Bounds<Pixels>>>,
    focus: FocusHandle,
    container_width: Rc<Cell<Pixels>>,
    dragging: bool,
    hovered: bool,
    touch: Rc<Cell<bool>>,
}
impl EventEmitter<CompareChanged> for ImageCompare {}
impl ImageCompare {
    pub fn new(
        before: impl Into<ImageSource>,
        after: impl Into<ImageSource>,
        label: impl Into<SharedString>,
        cx: &App,
    ) -> Self {
        Self {
            before: before.into(),
            after: after.into(),
            label: label.into(),
            captions: None,
            orientation: CompareOrientation::Horizontal,
            ratio: 1.5,
            value: 0.5,
            position: Slide::new(0.5),
            handle_w: Slide::new(40.),
            handle_h: Slide::new(40.),
            chevron: Slide::new(0.),
            bounds: Default::default(),
            focus: cx.focus_handle().tab_stop(true),
            container_width: Default::default(),
            dragging: false,
            hovered: false,
            touch: Default::default(),
        }
    }
    pub fn with_position(mut self, value: f32) -> Self {
        self.value = if value.is_finite() {
            value.clamp(0., 1.)
        } else {
            0.5
        };
        self.position.reset(self.value);
        self
    }
    pub fn with_ratio(mut self, width_over_height: f32) -> Self {
        if width_over_height.is_finite() {
            self.ratio = width_over_height.clamp(0.5, 3.);
        }
        self
    }
    pub fn with_orientation(mut self, orientation: CompareOrientation) -> Self {
        self.orientation = orientation;
        self
    }
    pub fn with_captions(
        mut self,
        before: impl Into<SharedString>,
        after: impl Into<SharedString>,
    ) -> Self {
        self.captions = Some([before.into(), after.into()]);
        self
    }
    pub fn position(&self) -> f32 {
        self.value
    }
    pub fn set_position(&mut self, value: f32, cx: &mut Context<Self>) {
        if !value.is_finite() {
            return;
        }
        let value = value.clamp(0., 1.);
        if (self.value - value).abs() < 0.0001 {
            return;
        }
        self.value = value;
        if self.dragging {
            self.position.reset(value);
        } else {
            self.position.duration(value, 0.26, cx);
        }
        cx.emit(CompareChanged(value));
        cx.notify();
    }
    fn pointer_value(&self, p: Point<Pixels>) -> f32 {
        let b = self.bounds.get();
        match self.orientation {
            CompareOrientation::Horizontal => (p.x - b.left()) / b.size.width.max(px(1.)),
            CompareOrientation::Vertical => (p.y - b.top()) / b.size.height.max(px(1.)),
        }
        .clamp(0., 1.)
    }
}

fn give(distance: f32) -> f32 {
    12. * (1. - 1. / (distance / 12. + 1.))
}

/// Keeps the handle about 42px inside the frame, giving way by at most 12px.
fn soften(at: f32, size: f32) -> f32 {
    const MARGIN: f32 = 42.;
    if at < MARGIN {
        MARGIN - give(MARGIN - at)
    } else if at > size - MARGIN {
        size - MARGIN + give(at - size + MARGIN)
    } else {
        at
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
impl Render for ImageCompare {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let measured = self.container_width.get();
        let width = if measured > px(0.) {
            measured
        } else {
            w.viewport_size().width - px(32.)
        }
        .max(px(1.));
        let height = width / self.ratio;
        let fraction = self.position.get(w, cx).clamp(0., 1.);
        let vertical = self.orientation == CompareOrientation::Vertical;
        let old = img(self.before.clone())
            .w(width)
            .h(height)
            .object_fit(ObjectFit::Cover);
        let new = img(self.after.clone())
            .w(width)
            .h(height)
            .object_fit(ObjectFit::Cover);
        let clip = div()
            .absolute()
            .top_0()
            .left_0()
            .overflow_hidden()
            .when(vertical, |d| d.w_full().h(relative(fraction)))
            .when(!vertical, |d| d.h_full().w(relative(fraction)))
            .child(old);
        let line = div()
            .absolute()
            .bg(p.panel)
            .shadow(vec![gpui::BoxShadow {
                color: black().opacity(0.14),
                offset: point(px(0.), px(0.)),
                blur_radius: px(0.),
                spread_radius: px(0.5),
                inset: false,
            }])
            .when(vertical, |d| {
                d.left_0()
                    .right_0()
                    .h(px(2.))
                    .top(relative(fraction))
                    .mt(px(-1.))
            })
            .when(!vertical, |d| {
                d.top_0()
                    .bottom_0()
                    .w(px(2.))
                    .left(relative(fraction))
                    .ml(px(-1.))
            });
        let (target_w, target_h) = if self.dragging {
            if vertical { (36., 56.) } else { (56., 36.) }
        } else {
            (40., 40.)
        };
        self.handle_w.duration(target_w, 0.42, cx);
        self.handle_h.duration(target_h, 0.42, cx);
        self.chevron.duration(if self.dragging { 4. } else { 0. }, 0.42, cx);
        let handle_w = self.handle_w.get(w, cx);
        let handle_h = self.handle_h.get(w, cx);
        let spread = self.chevron.get(w, cx);
        let width_px = f32::from(width);
        let height_px = f32::from(height);
        let along = if vertical {
            soften(fraction * height_px, height_px)
        } else {
            soften(fraction * width_px, width_px)
        };
        let marks = if vertical { ("⌃", "⌄") } else { ("‹", "›") };
        let thumb = div()
            .absolute()
            .w(px(handle_w))
            .h(px(handle_h))
            .rounded_full()
            .border_1()
            .border_color(if self.hovered { p.faint } else { p.line })
            .bg(p.panel)
            .shadow_sm()
            .text_color(p.ink)
            .text_size(px(16.))
            .flex()
            .items_center()
            .justify_center()
            .when(vertical, |d| {
                d.left(px(width_px * 0.5 - handle_w * 0.5))
                    .top(px(along - handle_h * 0.5))
            })
            .when(!vertical, |d| {
                d.left(px(along - handle_w * 0.5))
                    .top(px(height_px * 0.5 - handle_h * 0.5))
            })
            .child(
                div()
                    .flex()
                    .flex_col()
                    .when(!vertical, |d| d.flex_row())
                    .items_center()
                    .child(
                        div()
                            .when(vertical, |d| d.mb(px(-2.5 + spread)))
                            .when(!vertical, |d| d.mr(px(-2.5 + spread)))
                            .child(marks.0),
                    )
                    .child(
                        div()
                            .when(vertical, |d| d.mt(px(-2.5 + spread)))
                            .when(!vertical, |d| d.ml(px(-2.5 + spread)))
                            .child(marks.1),
                    ),
            );
        let bounds = self.bounds.clone();
        let moved = cx.listener(|s, e: &MouseMoveEvent, _, cx| {
            if s.dragging {
                s.set_position(s.pointer_value(e.position), cx);
            }
        });
        let released = cx.listener(|s, _: &MouseUpEvent, _, cx| {
            if s.dragging {
                s.dragging = false;
                cx.notify();
            }
        });
        let touch_active = self.touch.clone();
        let touch = cx.listener(|s, e: &TouchDragEvent, w, cx| match e.phase {
            TouchPhase::Started => {
                s.dragging = true;
                s.focus.focus(w, cx);
                s.set_position(s.pointer_value(e.position), cx);
            }
            TouchPhase::Moved => s.set_position(s.pointer_value(e.position), cx),
            TouchPhase::Ended | TouchPhase::Cancelled => {
                s.dragging = false;
                cx.notify();
            }
        });
        let contact = canvas(
            |b, w, _| w.insert_hitbox(b, HitboxBehavior::Normal),
            move |_, hit, w, _| {
                w.on_mouse_event(move |e: &TouchDragEvent, phase, w, cx| {
                    if phase.bubble() {
                        if e.phase == TouchPhase::Started {
                            touch_active.set(hit.is_hovered(w));
                        }
                        if !touch_active.get() {
                            return;
                        }
                        if matches!(e.phase, TouchPhase::Ended | TouchPhase::Cancelled) {
                            touch_active.set(false);
                        }
                        if e.phase == TouchPhase::Started {
                            w.prevent_default();
                        }
                        touch(e, w, cx);
                        cx.stop_propagation();
                    }
                });
            },
        )
        .absolute()
        .inset_0();
        let mut frame = div()
            .relative()
            .w(width)
            .h(height)
            .overflow_hidden()
            .rounded(crate::foundation::surface_radius(cx))
            .on_children_prepainted(move |bs, _, _| {
                if let Some(b) = bs.first() {
                    bounds.set(*b);
                }
            })
            .id("image-compare")
            .role(Role::Slider)
            .aria_label(self.label.clone())
            .aria_value(format!("{}% after", (100. - self.value * 100.).round()))
            .track_focus(&self.focus)
            .tab_index(0)
            .when(vertical, |d| d.cursor_ns_resize())
            .when(!vertical, |d| d.cursor_ew_resize())
            .on_hover(cx.listener(|s, over: &bool, _, cx| {
                if s.hovered != *over {
                    s.hovered = *over;
                    cx.notify();
                }
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|s, e: &MouseDownEvent, w, cx| {
                    if e.click_count == 2 && (s.pointer_value(e.position) - s.value).abs() < 0.08 {
                        s.dragging = false;
                        s.set_position(0.5, cx);
                        return;
                    }
                    s.dragging = true;
                    s.focus.focus(w, cx);
                    s.set_position(s.pointer_value(e.position), cx);
                }),
            )
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, _, cx| {
                let step = if e.keystroke.modifiers.shift {
                    0.1
                } else {
                    0.01
                };
                let vertical = s.orientation == CompareOrientation::Vertical;
                let value = match e.keystroke.key.as_str() {
                    "left" => s.value + if vertical { step } else { -step },
                    "right" => s.value + if vertical { -step } else { step },
                    "up" => s.value - step,
                    "down" => s.value + step,
                    "home" => 1.,
                    "end" => 0.,
                    _ => return,
                };
                s.set_position(value, cx);
                cx.stop_propagation();
            }))
            .child(new)
            .child(clip)
            .child(line)
            .child(thumb)
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
            )
            .child(contact)
            .child(
                div()
                    .absolute()
                    .inset_0()
                    .rounded(crate::foundation::surface_radius(cx))
                    .border_1()
                    .border_color(p.ink.opacity(0.08)),
            );
        if let Some(labels) = &self.captions {
            let before_w = measure(&labels[0], 12., w) + 20.;
            let after_w = measure(&labels[1], 12., w) + 20.;
            let span = if vertical { height_px } else { width_px };
            let before_room = fraction * span - (12. + before_w + 8.);
            let after_room = span - 12. - after_w - 8. - fraction * span;
            let chip = |label: SharedString, opacity: f32| {
                div()
                    .absolute()
                    .px(px(10.))
                    .py(px(3.))
                    .rounded_full()
                    .bg(p.panel.opacity(0.74))
                    .text_size(px(12.))
                    .line_height(px(20.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.ink)
                    .opacity(opacity)
                    .child(label)
            };
            frame = frame
                .child(
                    chip(labels[0].clone(), (before_room / 28.).clamp(0., 1.))
                        .top(px(12.))
                        .left(px(12.)),
                )
                .child(
                    chip(labels[1].clone(), (after_room / 28.).clamp(0., 1.))
                        .when(vertical, |d| d.bottom(px(12.)).left(px(12.)))
                        .when(!vertical, |d| d.top(px(12.)).right(px(12.))),
                );
        }
        constrained(frame, self.container_width.clone(), px(4096.))
    }
}

#[cfg(test)]
mod tests {
    use super::soften;

    #[test]
    fn handle_stays_inside_the_frame_margin() {
        assert!((soften(200., 400.) - 200.).abs() < 0.001);
        let edge = soften(0., 400.);
        assert!(edge > 30. && edge < 42.);
        assert!((soften(400., 400.) - (400. - edge)).abs() < 0.001);
    }
}
