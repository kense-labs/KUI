//! Hold a pointer or key until the fill reaches the far edge.
use crate::{
    foundation::Theme,
    primitives::{
        direct_touch::Contact,
        family::{Paint, PhaseClock, Slide, Tone, ease, face, lettering, measure},
        glyph::{Glyph, transformed},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc, time::Duration};
use web_time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoldEvent {
    Holding(bool),
    Confirmed,
}
pub struct HoldToConfirm {
    id: ElementId,
    label: SharedString,
    done_label: SharedString,
    duration: Duration,
    focus: FocusHandle,
    _blur: Subscription,
    timer: Option<Task<()>>,
    done_clock: PhaseClock,
    _activation: Subscription,
    start: Option<(Instant, f32)>,
    keyboard: bool,
    bounds: Rc<Cell<Bounds<Pixels>>>,
    fill: Slide,
    scale: Slide,
    width: Option<Slide>,
    done: bool,
    danger: bool,
    glyph: Glyph,
    contact: Contact,
}
impl EventEmitter<HoldEvent> for HoldToConfirm {}
impl HoldToConfirm {
    pub fn new(
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus = cx.focus_handle().tab_stop(true);
        let blur = cx.on_blur(&focus, w, |v, _, cx| v.cancel(cx));
        Self {
            id: id.into(),
            label: label.into(),
            done_label: "Done".into(),
            duration: Duration::from_millis(1200),
            focus,
            _blur: blur,
            _activation: cx.observe_window_activation(w, |v, w, cx| {
                if !w.is_window_active() {
                    v.cancel(cx)
                }
            }),
            done_clock: Default::default(),
            timer: None,
            start: None,
            keyboard: false,
            bounds: Default::default(),
            fill: Slide::snappy(0.),
            scale: Slide::snappy(1.),
            width: None,
            done: false,
            danger: false,
            glyph: Glyph::Trash,
            contact: Default::default(),
        }
    }
    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration = duration.max(Duration::from_millis(1));
        self
    }
    pub fn with_done_label(mut self, label: impl Into<SharedString>) -> Self {
        self.done_label = label.into();
        self
    }
    pub fn with_icon(mut self, glyph: Glyph) -> Self {
        self.glyph = glyph;
        self
    }
    pub fn danger(mut self) -> Self {
        self.danger = true;
        self
    }
    pub fn reset(&mut self, cx: &mut Context<Self>) {
        self.cancel(cx);
        if self.done {
            self.done_clock.restart(cx);
        }
        self.done = false;
        self.fill.set(0., cx);
        cx.notify()
    }
    fn elapsed(&self, cx: &App) -> Option<f32> {
        self.start.map(|(start, base)| {
            (base
                + cx.background_executor()
                    .now()
                    .saturating_duration_since(start)
                    .as_secs_f32()
                    / self.duration.as_secs_f32())
            .min(1.)
        })
    }
    fn begin(&mut self, keyboard: bool, w: &mut Window, cx: &mut Context<Self>) {
        if self.done || self.start.is_some() {
            return;
        }
        self.keyboard = keyboard;
        self.focus.focus(w, cx);
        let base = self.fill.value(cx).clamp(0., 1.);
        self.start = Some((cx.background_executor().now(), base));
        self.scale.set(0.97, cx);
        let wait = self.duration.mul_f32(1. - base);
        self.timer = Some(cx.spawn(async move |v, cx| {
            cx.background_executor().timer(wait).await;
            let _ = v.update(cx, |v, cx| v.finish(cx));
        }));
        cx.emit(HoldEvent::Holding(true));
        cx.notify();
    }
    fn finish(&mut self, cx: &mut Context<Self>) {
        if self.start.take().is_none() {
            return;
        }
        self.done = true;
        self.done_clock.restart(cx);
        self.fill.reset(1.);
        self.scale.set(1., cx);
        cx.emit(HoldEvent::Holding(false));
        cx.emit(HoldEvent::Confirmed);
        cx.notify()
    }
    fn cancel(&mut self, cx: &mut Context<Self>) {
        if let Some(x) = self.elapsed(cx) {
            self.start = None;
            self.timer = None;
            self.fill.reset(x);
            self.fill.set(0., cx);
            self.scale.set(1., cx);
            cx.emit(HoldEvent::Holding(false));
            cx.notify()
        }
    }
}
impl Render for HoldToConfirm {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let r = Theme::global(cx).tokens.radius.md;
        let label = if self.done {
            &self.done_label
        } else {
            &self.label
        }
        .clone();
        let width = measure(&label, 14., w) + 67.;
        let width_anim = self.width.get_or_insert_with(|| Slide::new(width));
        width_anim.set(width, cx);
        let width = width_anim.get(w, cx);
        let scale = self.scale.get(w, cx);
        let sw = width * scale;
        let h = 44. * scale;
        let fill = self.elapsed(cx).unwrap_or_else(|| self.fill.get(w, cx));
        if self.start.is_some() {
            w.request_animation_frame()
        }
        let fg = if self.danger { p.danger } else { p.ink };
        let bg = if self.danger { p.danger } else { p.bright };
        let age = self.done_clock.time(0.48, w, cx);
        let old = if self.done {
            self.label.clone()
        } else {
            self.done_label.clone()
        };
        let label_width = (width - 67.).max(1.);
        let contents = |ink: Hsla| {
            let mark = if self.done { Glyph::Check } else { self.glyph };
            let prev = if self.done { self.glyph } else { Glyph::Check };
            div()
                .w(px(sw))
                .h(px(h))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .gap(px(8.))
                .child(
                    div()
                        .relative()
                        .size(px(18.))
                        .when(age < 0.1, |e| {
                            e.child(
                                div()
                                    .absolute()
                                    .inset_0()
                                    .opacity(1. - ease(age / 0.1))
                                    .child(transformed(prev, 18., ink, 0., 1., (0., 0.), 1.)),
                            )
                        })
                        .child(transformed(
                            mark,
                            18.,
                            ink.opacity(ease(age / 0.16)),
                            0.,
                            1.,
                            (0., 0.),
                            if self.done { ease(age / 0.22) } else { 1. },
                        )),
                )
                .child(lettering(
                    old.clone(),
                    label.clone(),
                    age,
                    false,
                    14.,
                    ink,
                    label_width,
                ))
        };
        let shell = div()
            .relative()
            .w(px(sw))
            .h(px(h))
            .rounded(r)
            .overflow_hidden()
            .bg(p.panel)
            .child(contents(fg))
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .h_full()
                    .w(px(sw * fill))
                    .overflow_hidden()
                    .child(
                        contents(if self.danger {
                            p.neutral_inverse
                        } else {
                            p.inverse
                        })
                        .rounded(r)
                        .bg(bg),
                    ),
            )
            .child(
                div()
                    .absolute()
                    .inset_0()
                    .rounded(r)
                    .border_1()
                    .border_color(if self.done { bg } else { p.line }),
            );
        let bounds = self.bounds.clone();
        let moved = cx.listener(|v, e: &MouseMoveEvent, _, cx| {
            if !v.keyboard
                && v.start.is_some()
                && !v.bounds.get().dilate(px(24.)).contains(&e.position)
            {
                v.cancel(cx)
            }
        });
        let capture = canvas(
            move |b, _, _| bounds.set(b),
            move |_, _, w, _| {
                w.on_mouse_event(move |e: &MouseMoveEvent, phase, w, cx| {
                    if phase.capture() {
                        moved(e, w, cx)
                    }
                })
            },
        )
        .absolute()
        .inset_0();
        face(
            self.id.clone(),
            label.clone(),
            44.,
            r.into(),
            Tone::Clear,
            cx,
        )
        .relative()
        .track_focus(&self.focus)
        .w(px(width))
        .child(shell)
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|v, _, w, cx| v.begin(false, w, cx)),
        )
        .on_mouse_up(
            MouseButton::Left,
            cx.listener(|v, _, _, cx| {
                if !v.keyboard {
                    v.cancel(cx)
                }
            }),
        )
        .on_mouse_up_out(
            MouseButton::Left,
            cx.listener(|v, _, _, cx| {
                if !v.keyboard {
                    v.cancel(cx)
                }
            }),
        )
        .child(capture)
        .on_key_down(cx.listener(|v, e: &KeyDownEvent, w, cx| {
            if matches!(e.keystroke.key.as_str(), "space" | "enter") {
                if !e.is_held {
                    v.begin(true, w, cx)
                };
                cx.stop_propagation()
            }
        }))
        .on_key_up(cx.listener(|v, e: &KeyUpEvent, _, cx| {
            if v.keyboard && matches!(e.keystroke.key.as_str(), "space" | "enter") {
                v.cancel(cx);
                cx.stop_propagation()
            }
        }))
        .child(self.contact.listen(cx, |v, e, w, cx| match e.phase {
            TouchPhase::Started => v.begin(false, w, cx),
            TouchPhase::Moved if !v.bounds.get().dilate(px(24.)).contains(&e.position) => {
                v.cancel(cx)
            }
            TouchPhase::Ended | TouchPhase::Cancelled => v.cancel(cx),
            _ => {}
        }))
    }
}
