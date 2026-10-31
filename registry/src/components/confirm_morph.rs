//! Confirm and undo a host operation inside one morphing surface.
use crate::foundation::Theme;
use crate::primitives::{
    control::{UiLanguage, activate},
    family::{Paint, PhaseClock, Slide, Tone, ease, face, lettering, measure},
    glyph::{Glyph, icon, transformed},
};
use gpui::{prelude::*, *};
use std::time::Duration;
use web_time::Instant;
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmMode {
    #[default]
    Click,
    Hold,
    Countdown,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Delete,
    Undo,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfirmRequest {
    pub ticket: u64,
    pub intent: Intent,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Idle,
    Armed,
    Holding,
    Counting,
    Working(Intent),
    Done,
    Failed(Intent),
}
pub struct ConfirmMorph {
    id: ElementId,
    label: SharedString,
    prompt: SharedString,
    mode: ConfirmMode,
    available: bool,
    state: State,
    epoch: u64,
    started: Option<Instant>,
    width: Slide,
    natural: Option<f32>,
    drain: Slide,
    parts: PhaseClock,
    previous: State,
    focus: FocusHandle,
    answer: FocusHandle,
    timer: Option<Task<()>>,
    _blur: Subscription,
}
impl EventEmitter<ConfirmRequest> for ConfirmMorph {}
impl ConfirmMorph {
    pub fn new(
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        prompt: impl Into<SharedString>,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let answer = cx.focus_handle().tab_stop(true);
        let blur = cx.on_blur(&answer, w, |v, _, cx| v.release(cx));
        Self {
            id: id.into(),
            label: label.into(),
            prompt: prompt.into(),
            mode: ConfirmMode::Click,
            available: true,
            state: State::Idle,
            epoch: 0,
            started: None,
            width: Slide::new(146.),
            natural: None,
            drain: Slide::snappy(0.),
            parts: Default::default(),
            previous: State::Idle,
            focus: cx.focus_handle().tab_stop(true),
            answer,
            timer: None,
            _blur: blur,
        }
    }
    pub fn set_available(&mut self, available: bool, cx: &mut Context<Self>) {
        self.available = available;
        cx.notify()
    }
    pub fn restore(&mut self, cx: &mut Context<Self>) {
        self.available = true;
        self.set(State::Idle, cx)
    }
    pub fn set_mode(&mut self, m: ConfirmMode, cx: &mut Context<Self>) {
        if !matches!(self.state, State::Working(_)) {
            self.mode = m;
            self.set(State::Idle, cx)
        }
    }
    fn set(&mut self, s: State, cx: &mut Context<Self>) {
        self.epoch += 1;
        let same_face = matches!(
            (self.state, s),
            (State::Armed, State::Holding) | (State::Holding, State::Armed)
        ) || self.state == s;
        if !same_face {
            self.parts.restart(cx);
            self.previous = self.state
        }
        self.state = s;
        self.started = None;
        self.timer = None;
        cx.notify()
    }
    fn arm(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        self.set(State::Armed, cx);
        let f = self.answer.clone();
        f.focus(w, cx);
        self.idle_after(6000, cx)
    }
    fn idle_after(&mut self, ms: u64, cx: &mut Context<Self>) {
        let id = self.epoch;
        self.timer = Some(cx.spawn(async move |v, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(ms))
                .await;
            let _ = v.update(cx, |v, cx| {
                if v.epoch == id {
                    v.set(State::Idle, cx)
                }
            });
        }));
    }
    fn request(&mut self, intent: Intent, cx: &mut Context<Self>) {
        if matches!(self.state, State::Working(_)) {
            return;
        }
        self.set(State::Working(intent), cx);
        self.started = Some(cx.background_executor().now());
        cx.emit(ConfirmRequest {
            ticket: self.epoch,
            intent,
        })
    }
    pub fn complete(&mut self, ticket: u64, ok: bool, cx: &mut Context<Self>) {
        if self.epoch != ticket {
            return;
        }
        let State::Working(intent) = self.state else {
            return;
        };
        self.set(
            if !ok {
                State::Failed(intent)
            } else if intent == Intent::Undo {
                State::Idle
            } else {
                State::Done
            },
            cx,
        );
        if ok && intent == Intent::Delete {
            self.idle_after(5000, cx)
        }
    }
    fn release(&mut self, cx: &mut Context<Self>) {
        if self.state == State::Holding {
            let value = self.started.map_or(0., |s| {
                cx.background_executor()
                    .now()
                    .saturating_duration_since(s)
                    .as_secs_f32()
                    .min(1.)
            });
            self.drain.reset(value);
            self.drain.set(0., cx);
            self.set(State::Armed, cx);
            self.idle_after(6000, cx)
        }
    }
    fn begin(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.state != State::Armed {
            return;
        }
        if self.mode == ConfirmMode::Click {
            self.request(Intent::Delete, cx);
            return;
        }
        self.answer.focus(w, cx);
        let hold = self.mode == ConfirmMode::Hold;
        self.set(
            if hold {
                State::Holding
            } else {
                State::Counting
            },
            cx,
        );
        self.started = Some(cx.background_executor().now());
        let epoch = self.epoch;
        self.timer = Some(cx.spawn(async move |v, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(if hold { 1000 } else { 3000 }))
                .await;
            let _ = v.update(cx, |v, cx| {
                if epoch == v.epoch {
                    v.request(Intent::Delete, cx)
                }
            });
        }));
    }
    fn cancel(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if matches!(self.state, State::Armed | State::Holding | State::Counting) {
            self.set(State::Idle, cx);
            let f = self.focus.clone();
            f.focus(w, cx)
        }
    }
}
impl Render for ConfirmMorph {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let radii = Theme::global(cx).tokens.radius.clone();
        let l = UiLanguage::current(cx);
        let text = |en, zh| l.text(en, zh);
        let mw = |s: &str| measure(&s.to_owned().into(), 14., w);
        let cancel = text("Cancel", "取消");
        let answer = if self.mode == ConfirmMode::Hold {
            text("Hold to delete", "按住删除")
        } else {
            text("Delete", "删除")
        };
        let target = match self.state {
            State::Idle => mw(&self.label) + 54.,
            State::Armed | State::Holding => mw(&self.prompt) + mw(cancel) + mw(answer) + 80.,
            State::Counting => mw(text("Deleting in 3", "即将删除 3")) + mw(cancel) + 78.,
            State::Done => mw(text("Deleted", "已删除")) + mw(text("Undo", "撤销")) + 67.,
            State::Failed(_) => {
                mw(text("Couldn’t finish", "操作失败")) + mw(text("Retry", "重试")) + 51.
            }
            State::Working(intent) => {
                mw(if intent == Intent::Undo {
                    text("Restoring", "正在还原")
                } else {
                    text("Deleting", "正在删除")
                }) + 52.
            }
        }
        .min(f32::from(w.viewport_size().width) - 16.);
        if let Some(prev) = self.natural {
            self.width
                .duration(target, if prev > target { 0.4 } else { 0.42 }, cx)
        } else {
            self.width.reset(target)
        }
        self.natural = Some(target);
        let width = self.width.get(w, cx);
        let age = self.parts.time(0.40, w, cx);
        let shell = div()
            .id("clip")
            .debug_selector(|| "confirm-shell".into())
            .relative()
            .flex_none()
            .w(px(width))
            .h(px(36.))
            .rounded(radii.md)
            .bg(p.panel.blend(p.stage.opacity(0.55)))
            .overflow_hidden();
        let mut root = div()
            .id(self.id.clone())
            .relative()
            .flex()
            .items_center()
            .justify_start()
            .gap(px(2.))
            .w(px(target))
            .h(px(36.))
            .flex_none()
            .text_color(p.ink)
            .text_size(px(14.))
            .font_weight(FontWeight::MEDIUM)
            .whitespace_nowrap()
            .when(self.state != State::Idle, |e| e.px(px(4.)))
            .on_key_down(cx.listener(|v, e: &KeyDownEvent, w, cx| {
                if e.keystroke.key == "escape" {
                    v.cancel(w, cx);
                    cx.stop_propagation()
                }
            }))
            .on_mouse_down_out(cx.listener(|v, _, w, cx| v.cancel(w, cx)));
        let small = |name: &'static str, label: &'static str| {
            let delay = if name == "answer" { 0.12 } else { 0.085 };
            let t = ease((age - delay) / 0.26);
            face(name, label, 28., radii.sm.into(), Tone::Clear, cx)
                .px(px(12.))
                .opacity(t)
                .child(label)
        };
        let seconds = self.started.map_or(0., |s| {
            cx.background_executor()
                .now()
                .duration_since(s)
                .as_secs_f32()
        });
        match self.state {
            State::Idle => {
                let f = face(
                    "trigger",
                    self.label.clone(),
                    36.,
                    radii.md.into(),
                    Tone::Surface,
                    cx,
                )
                .w_full()
                .track_focus(&self.focus)
                .child(icon(Glyph::Trash, 16., p.secondary))
                .child(self.label.clone());
                root = if self.available {
                    root.child(activate(f, cx, |v, w, cx| v.arm(w, cx)))
                } else {
                    root.child(f.tab_stop(false).opacity(0.45).cursor_default())
                };
            }
            State::Armed | State::Holding => {
                let cancel = activate(
                    small("cancel", l.text("Cancel", "取消")),
                    cx,
                    |v, w, cx| v.cancel(w, cx),
                );
                let hold = self.mode == ConfirmMode::Hold;
                let text = if hold {
                    l.text("Hold to delete", "按住删除")
                } else {
                    l.text("Delete", "删除")
                };
                let progress = if self.state == State::Holding {
                    seconds.min(1.)
                } else {
                    self.drain.get(w, cx).clamp(0., 1.)
                };
                let label_width = mw(text);
                let button_w = label_width + 24.;
                let mut f = face("answer", text, 28., radii.sm.into(), Tone::Clear, cx)
                    .track_focus(&self.answer)
                    .relative()
                    .w(px(button_w))
                    .overflow_hidden()
                    .text_color(p.danger)
                    .bg(p.danger.opacity(0.14))
                    .child(text)
                    .opacity(ease((age - 0.12) / 0.26));
                if progress > 0. {
                    f = f.child(
                        div()
                            .absolute()
                            .left_0()
                            .top_0()
                            .bottom_0()
                            .w(px(button_w * progress))
                            .overflow_hidden()
                            .child(
                                div()
                                    .rounded(radii.sm)
                                    .bg(p.danger)
                                    .w(px(button_w))
                                    .h_full()
                                    .flex_none()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_color(p.inverse)
                                    .child(text),
                            ),
                    )
                }
                if hold {
                    if self.state == State::Holding {
                        w.request_animation_frame()
                    }
                    let handle = cx.entity();
                    let hit = canvas(
                        |_, _, _| (),
                        move |b, _, w, _| {
                            let handle = handle.clone();
                            w.on_mouse_event(move |e: &TouchDragEvent, phase, w, cx| {
                                if phase.bubble() && b.contains(&e.start_position) {
                                    handle.update(cx, |v, cx| match e.phase {
                                        TouchPhase::Started => {
                                            w.prevent_default();
                                            v.begin(w, cx)
                                        }
                                        TouchPhase::Cancelled | TouchPhase::Ended => v.release(cx),
                                        TouchPhase::Moved if !b.contains(&e.position) => {
                                            v.release(cx)
                                        }
                                        _ => {}
                                    })
                                }
                            });
                        },
                    )
                    .absolute()
                    .size_full();
                    f = f
                        .child(hit)
                        .on_mouse_down(MouseButton::Left, cx.listener(|v, _, w, cx| v.begin(w, cx)))
                        .on_mouse_up(MouseButton::Left, cx.listener(|v, _, _, cx| v.release(cx)))
                        .on_mouse_up_out(
                            MouseButton::Left,
                            cx.listener(|v, _, _, cx| v.release(cx)),
                        )
                        .on_hover(cx.listener(|v, inside: &bool, _, cx| {
                            if !inside {
                                v.release(cx)
                            }
                        }))
                        .on_key_down(cx.listener(|v, e: &KeyDownEvent, w, cx| {
                            if matches!(e.keystroke.key.as_str(), "space" | "enter") && !e.is_held {
                                v.begin(w, cx);
                                cx.stop_propagation()
                            }
                        }))
                        .on_key_up(cx.listener(|v, e: &KeyUpEvent, _, cx| {
                            if matches!(e.keystroke.key.as_str(), "space" | "enter") {
                                v.release(cx)
                            }
                        }));
                } else {
                    f = activate(f, cx, |v, w, cx| v.begin(w, cx))
                }
                root = root
                    .child(div().pl(px(12.)).pr(px(8.)).flex_none().child(lettering(
                        "".into(),
                        self.prompt.clone(),
                        (age - 0.05).max(0.),
                        false,
                        14.,
                        p.ink,
                        mw(&self.prompt),
                    )))
                    .child(cancel)
                    .child(f)
            }
            State::Counting => {
                w.request_animation_frame();
                root = root
                    .pl(px(12.))
                    .gap(px(6.))
                    .child(crate::primitives::glyph::countdown_ring(
                        1. - seconds / 3.,
                        16.,
                        p.ink,
                    ))
                    .child(format!(
                        "{} {}",
                        l.text("Deleting in", "即将删除"),
                        (3. - seconds).ceil().max(1.) as u32
                    ))
                    .child(activate(
                        small("cancel", l.text("Cancel", "取消")).track_focus(&self.answer),
                        cx,
                        |v, w, cx| v.cancel(w, cx),
                    ))
            }
            State::Working(intent) => {
                root = root
                    .pl(px(12.))
                    .gap(px(8.))
                    .child(crate::primitives::glyph::spinner(
                        if crate::foundation::Theme::global(cx).reduced_motion {
                            0.
                        } else {
                            w.request_animation_frame();
                            seconds / 0.72
                        },
                        15.,
                        p.secondary,
                    ))
                    .child(if intent == Intent::Undo {
                        l.text("Restoring", "正在还原")
                    } else {
                        l.text("Deleting", "正在删除")
                    })
            }
            State::Done => {
                root = root
                    .pl(px(12.))
                    .gap(px(8.))
                    .child(
                        div()
                            .size(px(16.))
                            .rounded_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .bg(p.success)
                            .child(transformed(
                                Glyph::Check,
                                11.,
                                p.inverse,
                                0.,
                                1.,
                                (0., 0.),
                                ease((age - 0.035) / 0.22),
                            )),
                    )
                    .child(l.text("Deleted", "已删除"))
                    .child(activate(
                        small("undo", l.text("Undo", "撤销")).track_focus(&self.focus),
                        cx,
                        |v, _, cx| v.request(Intent::Undo, cx),
                    ))
            }
            State::Failed(intent) => {
                root = root
                    .pl(px(12.))
                    .gap(px(8.))
                    .child(l.text("Couldn’t finish", "操作失败"))
                    .child(activate(
                        small("retry", l.text("Retry", "重试")).track_focus(&self.focus),
                        cx,
                        move |v, _, cx| v.request(intent, cx),
                    ))
            }
        }
        let old = match self.previous {
            State::Idle => self.label.clone(),
            State::Armed | State::Holding => self.prompt.clone(),
            State::Working(Intent::Undo) => text("Restoring", "正在还原").into(),
            State::Working(_) => text("Deleting", "正在删除").into(),
            State::Done => text("Deleted", "已删除").into(),
            State::Failed(_) => text("Couldn’t finish", "操作失败").into(),
            State::Counting => text("Deleting in", "即将删除").into(),
        };
        shell.child(root).when(age < 0.10, |s| {
            s.child(
                div()
                    .absolute()
                    .top_0()
                    .left(px(if self.previous == State::Idle {
                        38.
                    } else {
                        16.
                    }))
                    .h_full()
                    .flex()
                    .items_center()
                    .opacity(1. - ease(age / 0.1))
                    .text_color(p.secondary)
                    .child(old),
            )
        })
    }
}
