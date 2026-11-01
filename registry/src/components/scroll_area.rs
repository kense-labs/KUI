//! Constraint-based overflow with native GPUI wheel handling and an overlay thumb.
pub use crate::primitives::nav::HorizontalScrollRegion;
use crate::primitives::{
    editor::EditorScrollRegion,
    family::{Paint, Slide},
    nav::Content,
};
use gpui::{prelude::*, *};
pub struct ScrollArea {
    body: Content,
    height: f32,
    horizontal: bool,
    scroll: ScrollHandle,
    bar: Slide,
    over: bool,
    hide: Option<Task<()>>,
    track: crate::primitives::flyout::TriggerBounds,
    dragging: Option<(Pixels, Pixels)>,
    jumping: Option<(Slide, Pixels)>,
    thick: Slide,
}
impl ScrollArea {
    pub fn new(height: f32, body: Content) -> Self {
        Self {
            body,
            height: height.max(1.),
            horizontal: false,
            scroll: ScrollHandle::new(),
            bar: Slide::new(0.),
            over: false,
            hide: None,
            track: Default::default(),
            dragging: None,
            jumping: None,
            thick: Slide::new(0.),
        }
    }
    fn scroll_geometry(&self) -> (Pixels, Pixels, Pixels, Pixels) {
        let b = self.track.get();
        let m = self.scroll.max_offset();
        let o = self.scroll.offset();
        let (length, max, offset) = if self.horizontal {
            (b.size.width, m.x, -o.x)
        } else {
            (b.size.height, m.y, -o.y)
        };
        let extent = if length + max > px(0.) {
            (length * (f32::from(length) / f32::from(length + max)))
                .max(px(12.))
                .min(length)
        } else {
            px(0.)
        };
        (length, max, offset, extent)
    }
    fn set_axis_offset(&mut self, value: Pixels) {
        let mut o = self.scroll.offset();
        if self.horizontal {
            o.x = -value
        } else {
            o.y = -value
        };
        self.scroll.set_offset(o);
    }
    fn activity(&mut self, cx: &mut Context<Self>) {
        self.hide.take();
        self.bar.duration(1., 0.12, cx);
        if !self.over {
            self.hide = Some(cx.spawn(async |this, cx| {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(900))
                    .await;
                let _ = this.update(cx, |s, cx| {
                    s.bar.duration(0., 0.16, cx);
                    cx.notify()
                });
            }));
        }
        cx.notify()
    }
    pub fn horizontal(mut self) -> Self {
        self.horizontal = true;
        self
    }
    pub fn handle(&self) -> ScrollHandle {
        self.scroll.clone()
    }
}
impl Render for ScrollArea {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let handle = self.scroll.clone();
        let hor = self.horizontal;
        if let Some((jump, target)) = self.jumping {
            let value = px(jump.get(w, cx));
            if (target - value).abs() < px(0.1) {
                self.jumping = None;
                self.set_axis_offset(target);
            } else {
                self.set_axis_offset(value);
            }
        }
        let strength = self.bar.get(w, cx).clamp(0., 1.);
        let thickness = px(3. + 2. * self.thick.get(w, cx).clamp(0., 1.));
        let scroller = div()
            .id("viewport")
            .debug_selector(|| "scroll-viewport".into())
            .size_full()
            .track_scroll(&handle)
            .tab_index(0)
            .when(hor, |d| d.overflow_x_scroll())
            .when(!hor, |d| d.overflow_y_scroll())
            .on_hover(cx.listener(|s, over: &bool, _, cx| {
                s.over = *over;
                s.activity(cx)
            }))
            .on_scroll(cx.listener(|s, _, _, cx| {
                s.jumping = None;
                s.activity(cx)
            }))
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, _, cx| {
                s.jumping = None;
                let h = &s.scroll;
                let m = h.max_offset();
                let mut o = h.offset();
                let v = if s.horizontal { &mut o.x } else { &mut o.y };
                let max = if s.horizontal { m.x } else { m.y };
                *v = match e.keystroke.key.as_str() {
                    "home" => px(0.),
                    "end" => -max,
                    "pagedown" => (*v - px(s.height)).max(-max),
                    "pageup" => (*v + px(s.height)).min(px(0.)),
                    _ => return,
                };
                h.set_offset(o);
                cx.stop_propagation();
                cx.notify()
            }))
            .child((self.body)(w, cx));
        let overlay = canvas(
            |_, _, _| (),
            move |b, _, w, cx| {
                let offset = handle.offset();
                let max = handle.max_offset();
                if hor && let Some(host) = cx.try_global::<HorizontalScrollRegion>() {
                    (host.0)(b, -offset.x > px(0.1), -offset.x + px(0.1) < max.x)
                }
                if !hor && let Some(host) = cx.try_global::<EditorScrollRegion>() {
                    (host.0)(b, -offset.y > px(0.1), -offset.y + px(0.1) < max.y)
                }
                let (length, d, off) = if hor {
                    (b.size.width, max.x, -offset.x)
                } else {
                    (b.size.height, max.y, -offset.y)
                };
                if d <= px(0.) {
                    return;
                }
                let extent = (length * (f32::from(length) / f32::from(length + d)))
                    .max(px(12.))
                    .min(length);
                let pos = (length - extent) * (f32::from(off) / f32::from(d));
                let bar = if hor {
                    Bounds::new(
                        point(b.left() + pos, b.bottom() - thickness - px(1.)),
                        size(extent, thickness),
                    )
                } else {
                    Bounds::new(
                        point(b.right() - thickness - px(1.), b.top() + pos),
                        size(thickness, extent),
                    )
                };
                w.paint_quad(fill(bar, p.faint.opacity(0.58 * strength)).corner_radii(px(1.5)));
            },
        )
        .absolute()
        .inset_0();
        let rail = div()
            .id("track")
            .absolute()
            .when(hor, |b| b.bottom_0().left_0().right_0().h(px(9.)))
            .when(!hor, |b| b.right_0().top_0().bottom_0().w(px(9.)))
            .on_hover(cx.listener(|s, over: &bool, _, cx| {
                s.over = *over;
                s.thick.duration(if *over { 1. } else { 0. }, 0.12, cx);
                s.activity(cx)
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |s, e: &MouseDownEvent, _, cx| {
                    let (length, max, offset, extent) = s.scroll_geometry();
                    if max <= px(0.) {
                        return;
                    }
                    let b = s.track.get();
                    let coordinate = if hor {
                        e.position.x - b.left()
                    } else {
                        e.position.y - b.top()
                    };
                    let start = (length - extent) * (f32::from(offset) / f32::from(max));
                    if coordinate >= start && coordinate <= start + extent {
                        s.jumping = None;
                        s.dragging = Some((coordinate, offset));
                    } else {
                        let end = (offset + length * (if coordinate < start { -0.9 } else { 0.9 }))
                            .clamp(px(0.), max);
                        let mut slide = Slide::new(f32::from(offset));
                        slide.duration(f32::from(end), 0.22, cx);
                        s.jumping = Some((slide, end));
                    }
                    s.activity(cx);
                    cx.stop_propagation();
                }),
            );
        div()
            .id("scroll-area")
            .on_mouse_move(cx.listener(move |s, e: &MouseMoveEvent, _, cx| {
                let Some((origin, offset)) = s.dragging else {
                    return;
                };
                if e.pressed_button != Some(MouseButton::Left) {
                    s.dragging = None;
                    return;
                }
                let b = s.track.get();
                let pos = if hor {
                    e.position.x - b.left()
                } else {
                    e.position.y - b.top()
                };
                let (length, max, _, extent) = s.scroll_geometry();
                if length <= extent {
                    return;
                }
                s.set_axis_offset(
                    (offset + (pos - origin) * (f32::from(max) / f32::from(length - extent)))
                        .clamp(px(0.), max),
                );
                cx.notify();
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|s, _, _, _| s.dragging = None),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|s, _, _, _| s.dragging = None),
            )
            .relative()
            .w_full()
            .h(px(self.height))
            .overflow_hidden()
            .child(scroller)
            .child(overlay)
            .child(self.track.measure())
            .child(rail)
    }
}
