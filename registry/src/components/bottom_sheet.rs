//! A bottom-anchored sheet with adjustable snap heights.
use crate::primitives::{
    family::{Paint, Slide, disabled_motion},
    flyout::TriggerBounds,
    glyph::{Glyph, icon},
    nav::*,
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BottomSheetEvent {
    Opened,
    Closed,
    Snapped(f32),
}
struct Drag {
    start: Pixels,
    origin: f32,
    last: Pixels,
    last_time: web_time::Instant,
    speed: f32,
}
pub struct BottomSheet {
    trigger: SharedString,
    title: SharedString,
    body: Content,
    open: bool,
    detents: Vec<f32>,
    snap: usize,
    height: Slide,
    fade: Fade,
    drag: Option<Drag>,
    touch: Rc<Cell<bool>>,
    body_touch: Rc<Cell<u8>>,
    body_start: Rc<Cell<Pixels>>,
    launch: FocusHandle,
    close: FocusHandle,
    scope: FocusHandle,
    container: Option<TriggerBounds>,
    handle_hover: bool,
}
impl EventEmitter<BottomSheetEvent> for BottomSheet {}
impl BottomSheet {
    pub fn new(
        trigger: impl Into<SharedString>,
        title: impl Into<SharedString>,
        body: Content,
        cx: &App,
    ) -> Self {
        Self {
            trigger: trigger.into(),
            title: title.into(),
            body,
            open: false,
            detents: vec![0.45, 0.92],
            snap: 0,
            height: Slide::new(0.),
            fade: Default::default(),
            drag: None,
            touch: Default::default(),
            body_touch: Default::default(),
            body_start: Rc::new(Cell::new(px(0.))),
            launch: cx.focus_handle().tab_stop(true),
            close: cx.focus_handle().tab_stop(true),
            scope: cx.focus_handle(),
            container: None,
            handle_hover: false,
        }
    }
    pub fn contained(mut self) -> Self {
        self.container = Some(TriggerBounds::default());
        self
    }
    pub fn with_detents(mut self, detents: Vec<f32>) -> Self {
        self.detents = detents
            .into_iter()
            .filter(|n| n.is_finite() && *n > 0. && *n <= 1.)
            .collect();
        self.detents.sort_by(f32::total_cmp);
        self.detents.dedup();
        if self.detents.is_empty() {
            self.detents.push(0.6);
        }
        self.snap = self.snap.min(self.detents.len() - 1);
        self
    }
    pub fn set_open(&mut self, open: bool, w: &mut Window, cx: &mut Context<Self>) {
        if self.open == open {
            return;
        }
        self.open = open;
        self.drag = None;
        self.body_touch.set(0);
        self.fade.set(open, 0.16, cx);
        self.height.duration(
            if open { self.detents[self.snap] } else { 0. },
            if open { 0.4 } else { 0.3 },
            cx,
        );
        if open {
            self.close.focus(w, cx);
        } else {
            self.launch.focus(w, cx);
        }
        cx.emit(if open {
            BottomSheetEvent::Opened
        } else {
            BottomSheetEvent::Closed
        });
        cx.notify();
    }
    pub fn set_snap(&mut self, index: usize, cx: &mut Context<Self>) {
        let index = index.min(self.detents.len() - 1);
        self.height.duration(self.detents[index], 0.4, cx);
        if self.snap != index {
            self.snap = index;
            cx.emit(BottomSheetEvent::Snapped(self.detents[index]));
        }
        cx.notify();
    }
    fn begin(&mut self, y: Pixels, cx: &mut Context<Self>) {
        self.drag = Some(Drag {
            start: y,
            origin: self.height.value(cx),
            last: y,
            last_time: cx.background_executor().now(),
            speed: 0.,
        });
    }
    fn move_drag(&mut self, y: Pixels, span: Pixels, cx: &mut Context<Self>) {
        if let Some(drag) = &mut self.drag {
            let now = cx.background_executor().now();
            let elapsed = now
                .saturating_duration_since(drag.last_time)
                .as_secs_f32()
                .max(0.001);
            drag.speed = f32::from(y - drag.last) / elapsed;
            drag.last = y;
            drag.last_time = now;
            let span_px = f32::from(span).max(1.);
            let value = drag.origin - f32::from(y - drag.start) / span_px;
            let ceiling = *self.detents.last().unwrap_or(&1.);
            let resisted = if value > ceiling {
                ceiling + rubber((value - ceiling) * span_px) / span_px
            } else {
                value
            };
            self.height
                .reset(resisted.clamp(0., ceiling + 120. / span_px));
            cx.notify();
        }
    }
    fn end(&mut self, cancelled: bool, w: &mut Window, cx: &mut Context<Self>) {
        let Some(drag) = self.drag.take() else {
            return;
        };
        let current = self.height.value(cx);
        let speed = if cx
            .background_executor()
            .now()
            .saturating_duration_since(drag.last_time)
            .as_millis()
            < 60
        {
            drag.speed
        } else {
            0.
        };
        let span = self
            .container
            .as_ref()
            .map_or(w.viewport_size().height, |b| b.get().size.height);
        let span_px = f32::from(span).max(1.);
        let projected = current - speed * 0.2 / span_px;
        let closed = -40. / span_px;
        let mut best = (projected - closed).abs();
        let mut closing = true;
        let mut index = 0;
        for (i, stop) in self.detents.iter().enumerate() {
            let distance = (projected - stop).abs();
            if distance < best {
                best = distance;
                closing = false;
                index = i;
            }
        }
        if !cancelled && speed.abs() > 320. && !closing && index == self.snap {
            if speed < 0. {
                index = (self.snap + 1).min(self.detents.len() - 1);
            } else if self.snap == 0 {
                closing = true;
            } else {
                index = self.snap - 1;
            }
        }
        if cancelled {
            self.set_snap(self.snap, cx);
        } else if closing {
            self.set_open(false, w, cx);
        } else {
            self.set_snap(index, cx);
        }
    }
}
impl Render for BottomSheet {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let radius = crate::foundation::surface_radius(cx);
        let root = div()
            .relative()
            .when_some(self.container.as_ref(), |d, b| {
                d.size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .overflow_hidden()
                    .rounded(radius)
                    .child(b.measure())
            })
            .child(
                button("sheet-launch", self.trigger.clone(), cx)
                    .bg(p.panel)
                    .border_1()
                    .border_color(p.line)
                    .track_focus(&self.launch)
                    .aria_expanded(self.open)
                    .child(self.trigger.clone())
                    .on_click(cx.listener(|s, _, w, cx| s.set_open(true, w, cx))),
            );
        let current = self.height.get(w, cx).clamp(0., 1.);
        let opacity = self.fade.get(w, cx);
        if !self.open && current < 0.001 && opacity < 0.001 {
            return root;
        }
        let size = self
            .container
            .as_ref()
            .map_or(w.viewport_size(), |b| b.get().size);
        let height = size.height * current;
        let width = size.width.min(px(576.));
        let left = (size.width - width) / 2.;
        let active = self.touch.clone();
        let span = size.height;
        let touch = cx.listener(move |s, e: &TouchDragEvent, w, cx| match e.phase {
            TouchPhase::Started => s.begin(e.position.y, cx),
            TouchPhase::Moved => s.move_drag(e.position.y, span, cx),
            TouchPhase::Ended => s.end(false, w, cx),
            TouchPhase::Cancelled => s.end(true, w, cx),
        });
        let handle = row()
            .relative()
            .id("sheet-drag")
            .role(Role::Slider)
            .aria_label(tr(cx, "Adjust sheet height", "调整面板高度"))
            .aria_value(format!("{}%", (self.detents[self.snap] * 100.).round()))
            .tab_index(0)
            .on_key_down(cx.listener(
                |s, e: &KeyDownEvent, _, cx| match e.keystroke.key.as_str() {
                    "up" | "right" => {
                        s.set_snap(s.snap.saturating_add(1), cx);
                        cx.stop_propagation();
                    }
                    "down" | "left" => {
                        s.set_snap(s.snap.saturating_sub(1), cx);
                        cx.stop_propagation();
                    }
                    "home" => {
                        s.set_snap(s.detents.len() - 1, cx);
                        cx.stop_propagation();
                    }
                    "end" => {
                        s.set_snap(0, cx);
                        cx.stop_propagation();
                    }
                    "enter" | "space" => {
                        let next = if s.snap == 0 { s.detents.len() - 1 } else { 0 };
                        s.set_snap(next, cx);
                        cx.stop_propagation();
                    }
                    _ => {}
                },
            ))
            .h(px(20.))
            .mb(px(4.))
            .justify_center()
            .cursor_pointer()
            .on_hover(cx.listener(|s, over: &bool, _, cx| {
                s.handle_hover = *over;
                cx.notify();
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|s, e: &MouseDownEvent, _, cx| s.begin(e.position.y, cx)),
            )
            .child(
                div()
                    .w(px(36.))
                    .h(px(5.))
                    .rounded_full()
                    .bg(if self.handle_hover { p.secondary } else { p.faint }),
            )
            .child(
                canvas(
                    |bounds, w, _| w.insert_hitbox(bounds, HitboxBehavior::Normal),
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
                                touch(e, w, cx);
                                cx.stop_propagation();
                            }
                        });
                    },
                )
                .absolute()
                .inset_0(),
            );
        // Touch panning at a peek height moves the sheet; taps still reach content.
        // At the tallest detent, input passes through to the content's scroller.
        let body_touch = self.body_touch.clone();
        let body_offset = self.body_start.clone();
        let can_expand = self.snap + 1 < self.detents.len();
        let scroll_drag = cx.listener(move |s, e: &ScrollEvent, w, cx| match e.touch_phase {
            TouchPhase::Started => {
                body_offset.set(px(0.));
                s.begin(px(0.), cx);
            }
            TouchPhase::Moved => {
                let y = body_offset.get() + e.delta.pixel_delta(px(16.)).y;
                body_offset.set(y);
                s.move_drag(y, span, cx);
            }
            TouchPhase::Ended => s.end(false, w, cx),
            TouchPhase::Cancelled => s.end(true, w, cx),
        });
        let body = div().relative().w_full().child((self.body)(w, cx)).child(
            canvas(
                |bounds, w, _| w.insert_hitbox(bounds, HitboxBehavior::Normal),
                move |_, hit, w, _| {
                    w.on_mouse_event(move |e: &ScrollEvent, phase, w, cx| {
                        if !phase.capture() || e.source != ScrollSource::Touch {
                            return;
                        }
                        if e.touch_phase == TouchPhase::Started {
                            body_touch
                                .set(u8::from(can_expand && hit.is_hovered_at(e.position, w)));
                        }
                        if body_touch.get() == 0 {
                            return;
                        }
                        scroll_drag(e, w, cx);
                        if matches!(e.touch_phase, TouchPhase::Ended | TouchPhase::Cancelled) {
                            body_touch.set(0);
                        }
                        w.prevent_default();
                        cx.stop_propagation();
                    });
                },
            )
            .absolute()
            .inset_0(),
        );
        let sheet = panel(cx)
            .id("bottom-sheet")
            .track_focus(&self.scope)
            .tab_stop(false)
            .role(Role::Dialog)
            .aria_label(self.title.clone())
            .absolute()
            .bottom_0()
            .left(left)
            .w(width)
            .h(height)
            .p_0()
            .shadow_lg()
            .rounded_none()
            .rounded_t(radius)
            .gap_0()
            .occlude()
            .overflow_y_scroll()
            .on_click(|_, _, cx| cx.stop_propagation())
            .on_key_down(cx.listener(
                |s, e: &KeyDownEvent, w, cx| match e.keystroke.key.as_str() {
                    "escape" => {
                        s.set_open(false, w, cx);
                        cx.stop_propagation();
                    }
                    "tab" => {
                        if e.keystroke.modifiers.shift {
                            w.focus_prev(cx);
                        } else {
                            w.focus_next(cx);
                        }
                        if !s.scope.contains_focused(w, cx) {
                            s.close.focus(w, cx);
                        }
                        cx.stop_propagation();
                    }
                    _ => {}
                },
            ))
            .child(
                div()
                    .pt(px(8.))
                    .px(px(24.))
                    .pb(px(16.))
                    .flex()
                    .flex_col()
                    .child(handle)
                    .child(
                        row()
                            .items_start()
                            .justify_between()
                            .gap(px(16.))
                            .child(
                                div()
                                    .text_size(px(18.))
                                    .line_height(px(25.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .child(self.title.clone()),
                            )
                            .child(
                                button("sheet-close", tr(cx, "Close", "关闭"), cx)
                                    .size(px(32.))
                                    .rounded_full()
                                    .p_0()
                                    .mt(px(-2.))
                                    .border_1()
                                    .border_color(p.line)
                                    .bg(p.tray)
                                    .text_color(p.secondary)
                                    .hover(|s| s.text_color(p.ink).border_color(p.faint))
                                    .track_focus(&self.close)
                                    .child(icon(Glyph::Close, 16., p.secondary))
                                    .on_click(cx.listener(|s, _, w, cx| s.set_open(false, w, cx))),
                            ),
                    ),
            )
            .child(body.px(px(24.)).pt(px(4.)).pb(px(24.)));
        let shade = div()
            .id("sheet-backdrop")
            .relative()
            .w(size.width)
            .h(size.height)
            .bg(black().opacity({
                let full = match crate::foundation::Theme::global(cx).mode {
                    crate::foundation::ThemeMode::Dark => 0.52,
                    crate::foundation::ThemeMode::Light => 0.34,
                };
                let dim = if disabled_motion(cx) {
                    opacity
                } else {
                    veil(current, self.detents[0], *self.detents.last().unwrap_or(&1.))
                };
                full * dim
            }))
            .overflow_hidden()
            .occlude()
            .on_click(cx.listener(|s, _, w, cx| s.set_open(false, w, cx)))
            .on_mouse_move(cx.listener(move |s, e: &MouseMoveEvent, _, cx| {
                if e.pressed_button == Some(MouseButton::Left) {
                    s.move_drag(e.position.y, span, cx);
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|s, _, w, cx| s.end(false, w, cx)),
            )
            .child(sheet);
        if self.container.is_some() {
            root.child(shade.absolute().inset_0())
        } else {
            root.child(
                deferred(anchored().position(point(px(0.), px(0.))).child(shade)).with_priority(4),
            )
        }
    }
}

/// Upward travel past the tallest detent approaches 120px and never reaches it.
fn rubber(distance: f32) -> f32 {
    let distance = distance.max(0.);
    (1. - 1. / (distance * 0.55 / 120. + 1.)) * 120.
}

/// Full dim at the tallest detent, 0.78 at the lowest, and none when closed.
fn veil(current: f32, low: f32, top: f32) -> f32 {
    if current >= low {
        let span = (top - low).max(0.0001);
        let t = ((top - current) / span).clamp(0., 1.);
        1. - 0.22 * t
    } else if low <= 0. {
        0.
    } else {
        0.78 * (current / low).clamp(0., 1.)
    }
}

#[cfg(test)]
mod tests {
    use super::{rubber, veil};

    #[test]
    fn stretch_and_dim_follow_the_source() {
        assert_eq!(rubber(0.), 0.);
        assert!(rubber(10_000.) < 120.);
        assert!(rubber(10_000.) > 110.);
        assert!((veil(0.92, 0.45, 0.92) - 1.).abs() < 0.001);
        assert!((veil(0.45, 0.45, 0.92) - 0.78).abs() < 0.001);
        assert_eq!(veil(0., 0.45, 0.92), 0.);
    }
}
