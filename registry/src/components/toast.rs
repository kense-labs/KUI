//! A transient announcement. Hover pauses the timeout; a newer message supersedes the old deadline.
use crate::primitives::{
    family::{Paint, PhaseClock, Slide, disabled_motion, ease, standard_ease},
    glyph::{Glyph, icon, transformed},
    presentation::*,
};
use gpui::{prelude::*, *};
use std::time::Duration;
use web_time::Instant;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToastEvent {
    Action,
    Dismissed,
}
pub struct Toast {
    message: ChangingText,
    description: Option<SharedString>,
    clock: PhaseClock,
    opacity_from: f32,
    exit_y: f32,
    x: Slide,
    drag: Option<(f32, web_time::Instant)>,
    action: Option<SharedString>,
    remaining: Duration,
    since: Option<Instant>,
    timer: Option<Task<()>>,
    hover_paused: bool,
    stack_paused: bool,
    in_stack: bool,
    stack_front: bool,
    shown: bool,
    slide: Slide,
}
impl EventEmitter<ToastEvent> for Toast {}
impl Default for Toast {
    fn default() -> Self {
        Self::new()
    }
}
impl Toast {
    pub fn new() -> Self {
        Self {
            message: ChangingText::new(""),
            description: None,
            clock: Default::default(),
            opacity_from: 0.,
            exit_y: 0.,
            x: Slide::new(0.),
            drag: None,
            action: None,
            remaining: Duration::ZERO,
            since: None,
            timer: None,
            hover_paused: false,
            stack_paused: false,
            in_stack: false,
            stack_front: false,
            shown: false,
            slide: Slide::snappy(0.),
        }
    }
    pub fn with_description(mut self, text: impl Into<SharedString>) -> Self {
        self.description = Some(text.into());
        self
    }
    fn opacity(&self, cx: &App) -> f32 {
        if disabled_motion(cx) {
            return f32::from(self.shown);
        }
        if self.shown {
            self.opacity_from + (1. - self.opacity_from) * ease(self.clock.age(cx) / 0.24)
        } else {
            self.opacity_from * (1. - standard_ease(self.clock.age(cx) / 0.18))
        }
    }
    pub fn show(
        &mut self,
        message: impl Into<SharedString>,
        action: Option<SharedString>,
        timeout: Duration,
        cx: &mut Context<Self>,
    ) {
        if self.shown {
            self.message.set(message, 1., cx);
        } else {
            self.message = ChangingText::new(message);
            let opacity = self.opacity(cx);
            if opacity > 0. {
                self.slide.reset(
                    self.exit_y + (8. - self.exit_y) * standard_ease(self.clock.age(cx) / 0.18),
                );
            } else {
                self.slide.reset(16.);
                self.x.reset(0.);
            }
            self.opacity_from = opacity;
            self.clock.restart(cx);
        }
        self.drag = None;
        self.x.duration(0., 0.26, cx);
        self.action = action;
        self.remaining = timeout;
        self.shown = true;
        self.slide.duration(0., 0.42, cx);
        self.arm(cx);
        cx.notify()
    }
    fn arm(&mut self, cx: &mut Context<Self>) {
        if self.hover_paused || self.stack_paused {
            return;
        }
        let wait = self.remaining;
        self.since = Some(cx.background_executor().now());
        self.timer = Some(cx.spawn(async move |entity, cx| {
            cx.background_executor().timer(wait).await;
            let _ = entity.update(cx, |v, cx| v.close(ToastEvent::Dismissed, cx));
        }));
    }
    fn close(&mut self, event: ToastEvent, cx: &mut Context<Self>) {
        if self.shown {
            self.opacity_from = self.opacity(cx);
            self.exit_y = self.slide.value(cx);
            self.clock.restart(cx);
            self.shown = false;
            self.timer = None;
            self.since = None;
            self.drag = None;
            cx.emit(event);
            cx.notify()
        }
    }
    fn release(&mut self, cx: &mut Context<Self>) {
        let Some((_, start)) = self.drag.take() else {
            return;
        };
        let x = self.x.value(cx);
        let dt = cx
            .background_executor()
            .now()
            .saturating_duration_since(start)
            .as_secs_f32()
            .max(0.016);
        if x.abs() >= 80. || (x.abs() > 1. && (x / dt).abs() >= 480.) {
            self.x.duration(x.signum() * 428., 0.3, cx);
            self.close(ToastEvent::Dismissed, cx);
        } else {
            self.x.duration(0., 0.26, cx);
            cx.notify();
        }
    }
    fn pause(&mut self, inside: bool, cx: &mut Context<Self>) {
        self.hover_paused = inside;
        self.sync_pause(cx);
    }
    pub(crate) fn set_stack_chrome(&mut self, front: bool) {
        self.in_stack = true;
        self.stack_front = front;
    }
    pub(crate) fn set_stack_paused(&mut self, paused: bool, cx: &mut Context<Self>) {
        self.stack_paused = paused;
        self.sync_pause(cx);
    }
    fn sync_pause(&mut self, cx: &mut Context<Self>) {
        if !self.shown {
            return;
        }
        if self.hover_paused || self.stack_paused {
            if let Some(s) = self.since.take() {
                self.remaining = self
                    .remaining
                    .saturating_sub(cx.background_executor().now().saturating_duration_since(s));
                self.timer = None;
            }
        } else if self.since.is_none() {
            self.arm(cx)
        }
    }
}
impl Render for Toast {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let age = self.clock.time(if self.shown { 0.36 } else { 0.18 }, w, cx);
        let f = self.opacity(cx);
        let offset = if self.shown {
            self.slide.get(w, cx)
        } else {
            self.exit_y + (8. - self.exit_y) * standard_ease(age / 0.18)
        };
        let x = self.x.get(w, cx);
        let check = ease((age - 0.12) / 0.24);
        let mut el = column().w_full().max_w(px(360.));
        if self.shown || f > 0.001 {
            el = el.child(
                row()
                    .id("notification")
                    .debug_selector(|| "toast-surface".into())
                    .role(Role::Status)
                    .gap_2()
                    .p_4()
                    .w_full()
                    .min_h(px(76.))
                    .max_w_full()
                    .bg(p.panel)
                    .rounded(crate::foundation::surface_radius(cx))
                    .border_1()
                    .border_color(p.line)
                    .when(self.in_stack, |row| {
                        row.pt(px(12.))
                            .pr(px(12.))
                            .pb(px(12.))
                            .pl(px(16.))
                    })
                    .when(self.in_stack && self.stack_front, |row| row.shadow_lg())
                    .when(!self.in_stack || !self.stack_front, |row| row.shadow_sm())
                    .relative()
                    .top(px(offset))
                    .left(px(x))
                    .opacity(f)
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|s, e: &MouseDownEvent, _, cx| {
                            if s.shown && !disabled_motion(cx) {
                                s.drag = Some((
                                    f32::from(e.position.x) - s.x.value(cx),
                                    cx.background_executor().now(),
                                ));
                                s.pause(true, cx);
                            }
                        }),
                    )
                    .on_mouse_up(MouseButton::Left, cx.listener(|s, _, _, cx| s.release(cx)))
                    .on_mouse_up_out(MouseButton::Left, cx.listener(|s, _, _, cx| s.release(cx)))
                    .on_mouse_move(cx.listener(|s, e: &MouseMoveEvent, _, cx| {
                        s.pause(true, cx);
                        if let Some((start, _)) = s.drag {
                            s.x.reset(f32::from(e.position.x) - start);
                            cx.notify();
                        }
                    }))
                    .on_hover(cx.listener(|s, h: &bool, _, cx| s.pause(*h, cx)))
                    .child(
                        row()
                            .size(px(28.))
                            .justify_center()
                            .rounded_full()
                            .bg(p.success.opacity(0.12))
                            .child(transformed(
                                Glyph::Check,
                                16.,
                                p.success,
                                0.,
                                1.,
                                (0., 0.),
                                check,
                            )),
                    )
                    .child(
                        column()
                            .flex_1()
                            .gap_1()
                            .child(
                                self.message
                                    .render(14., w, cx)
                                    .font_weight(FontWeight::MEDIUM),
                            )
                            .when_some(self.description.clone(), |el, d| el.child(hint(d, p))),
                    )
                    .when_some(self.action.clone(), |el, a| {
                        el.child(command("notification-action", a, cx, |s, _, cx| {
                            s.close(ToastEvent::Action, cx)
                        }))
                    })
                    .child(
                        crate::primitives::family::face(
                            "notification-close",
                            tr(cx, "Dismiss", "关闭"),
                            28.,
                            8.,
                            crate::primitives::family::Tone::Clear,
                            cx,
                        )
                        .w(px(28.))
                        .hover(|s| s.bg(p.tray).text_color(p.ink))
                        .child(icon(Glyph::Close, 14., p.secondary))
                        .on_click(cx.listener(|s, _, _, cx| s.close(ToastEvent::Dismissed, cx))),
                    ),
            )
        }
        el
    }
}
