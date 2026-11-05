//! Destructive action with optional typed confirmation and ticketed host completion.
use crate::primitives::{
    editor::{EditEvent, Editor, InputMode},
    family::{Paint, PhaseClock, Slide, disabled_motion},
    flyout::TriggerBounds,
    glyph::{Glyph, icon},
    nav::*,
};
use gpui::{prelude::*, *};
use std::time::Duration;
#[derive(Clone, Copy, PartialEq)]
enum State {
    Closed,
    Confirm,
    Pending,
    Failed,
    Done,
}
pub struct ConfirmRequested(pub u64);
pub struct ConfirmPopover {
    label: SharedString,
    description: SharedString,
    phrase: Option<SharedString>,
    editor: Entity<Editor>,
    state: State,
    ticket: u64,
    presence: Slide,
    fill: Slide,
    busy: Slide,
    status: PhaseClock,
    bounds: TriggerBounds,
    focus: FocusHandle,
    accept: FocusHandle,
    panel: FocusHandle,
    height: std::rc::Rc<std::cell::Cell<Pixels>>,
    return_focus: bool,
    _edit: Subscription,
}
impl EventEmitter<ConfirmRequested> for ConfirmPopover {}
impl ConfirmPopover {
    pub fn new(
        label: impl Into<SharedString>,
        description: impl Into<SharedString>,
        phrase: Option<SharedString>,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let editor = cx.new(|cx| {
            Editor::new(
                tr(cx, "Confirmation text", "确认文字"),
                InputMode::Single,
                w,
                cx,
            )
        });
        let sub = cx.subscribe_in(
            &editor,
            w,
            |s: &mut Self, _, e: &EditEvent, w, cx| match e {
                EditEvent::Submitted => s.submit(w, cx),
                EditEvent::Escape => s.hide(w, cx),
                _ => cx.notify(),
            },
        );
        Self {
            label: label.into(),
            description: description.into(),
            phrase,
            editor,
            state: State::Closed,
            ticket: 0,
            presence: Slide::new(0.),
            fill: Slide::new(0.),
            busy: Slide::new(0.),
            status: Default::default(),
            bounds: Default::default(),
            focus: cx.focus_handle().tab_stop(true),
            accept: cx.focus_handle().tab_stop(true),
            panel: cx.focus_handle(),
            height: Default::default(),
            return_focus: false,
            _edit: sub,
        }
    }
    fn valid(&self, w: &mut Window, cx: &mut App) -> bool {
        self.phrase.as_deref().is_none_or(|phrase| {
            self.editor.update(cx, |e, cx| {
                e.value() == phrase && e.marked_text_range(w, cx).is_none()
            })
        })
    }
    fn submit(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if matches!(self.state, State::Confirm | State::Failed) && self.valid(w, cx) {
            self.ticket = self.ticket.wrapping_add(1);
            self.state = State::Pending;
            self.panel.focus(w, cx);
            self.editor.update(cx, |e, cx| {
                e.disabled = true;
                cx.notify()
            });
            self.status.restart(cx);
            cx.emit(ConfirmRequested(self.ticket));
            cx.notify()
        }
    }
    fn hide(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.state != State::Pending {
            self.state = State::Closed;
            self.presence.duration(0., 0.16, cx);
            self.focus.focus(w, cx);
            cx.notify()
        }
    }
    pub fn resolve(&mut self, ticket: u64, ok: bool, cx: &mut Context<Self>) {
        if self.state != State::Pending || ticket != self.ticket {
            return;
        }
        self.state = if ok { State::Done } else { State::Failed };
        if !ok {
            self.editor.update(cx, |e, cx| {
                e.disabled = false;
                cx.notify()
            });
        }
        self.status.restart(cx);
        cx.notify();
        if ok {
            cx.spawn(async move |s, cx| {
                cx.background_executor()
                    .timer(Duration::from_millis(900))
                    .await;
                let _ = s.update(cx, |v, cx| {
                    if v.ticket == ticket && v.state == State::Done {
                        v.return_focus = true;
                        v.state = State::Closed;
                        v.presence.duration(0., 0.16, cx);
                        cx.notify()
                    }
                });
            })
            .detach()
        }
    }
}
impl Render for ConfirmPopover {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.return_focus {
            self.return_focus = false;
            if self.panel.contains_focused(w, cx) {
                self.focus.focus(w, cx)
            }
        }
        let p = Paint::get(cx);
        let mut root = div().child(
            button("trigger", self.label.clone(), cx)
                .relative()
                .track_focus(&self.focus)
                .aria_expanded(self.state != State::Closed)
                .bg(p.panel)
                .border_1()
                .border_color(p.line)
                .text_color(p.danger)
                .child(self.label.clone())
                .child(self.bounds.measure())
                .on_click(cx.listener(|s, _, w, cx| {
                    if s.state == State::Closed {
                        s.state = State::Confirm;
                        s.presence.duration(1., 0.24, cx);
                        s.editor.update(cx, |e, cx| {
                            e.disabled = false;
                            e.set_value("", w, cx);
                        });
                        if s.phrase.is_some() {
                            s.editor.update(cx, |e, cx| e.focus(w, cx))
                        } else {
                            s.accept.focus(w, cx)
                        }
                        cx.notify()
                    }
                })),
        );
        let a = self.presence.get(w, cx).clamp(0., 1.);
        if self.state == State::Closed && a < 0.001 {
            return root;
        }
        let cancel = tr(cx, "Cancel", "取消");
        let go = match self.state {
            State::Pending => tr(cx, "Working…", "处理中…"),
            State::Failed => tr(cx, "Retry", "重试"),
            State::Done => tr(cx, "Done", "已完成"),
            _ => tr(cx, "Confirm", "确认"),
        };
        let mut card = column()
            .id("confirm-panel")
            .track_focus(&self.panel)
            .tab_stop(false)
            .role(Role::Dialog)
            .occlude()
            .w(fit(w, 272.))
            .p_3()
            .gap_2()
            .flex_none()
            .opacity(((a - 0.3) / 0.7).clamp(0., 1.))
            .child(label(self.label.clone(), 15., p.ink))
            .child(label(self.description.clone(), 13., p.secondary))
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                if e.keystroke.key == "escape"
                    && !s
                        .editor
                        .update(cx, |ed, cx| ed.marked_text_range(w, cx).is_some())
                {
                    s.hide(w, cx)
                }
            }))
            .on_mouse_down_out(cx.listener(|s, e: &MouseDownEvent, w, cx| {
                if !s.bounds.get().contains(&e.position) {
                    s.hide(w, cx)
                }
            }));
        if let Some(phrase) = &self.phrase {
            card = card
                .child(label(
                    format!("{} {}", tr(cx, "Type", "输入"), phrase),
                    12.,
                    p.secondary,
                ))
                .child(
                    div()
                        .id("confirm-phrase")
                        .debug_selector(|| "confirm-phrase-field".into())
                        .h(px(32.))
                        .w_full()
                        .min_w_0()
                        .px(px(12.))
                        .rounded(crate::foundation::surface_radius(cx))
                        .border_1()
                        .border_color(
                            if self.editor.read(cx).focus.is_focused(w)
                                && !self.editor.read(cx).disabled
                            {
                                p.faint
                            } else {
                                p.line
                            },
                        )
                        .when(!self.editor.read(cx).disabled, |el| {
                            el.cursor(CursorStyle::IBeam)
                                .on_click(cx.listener(|s, _, w, cx| {
                                    s.editor.update(cx, |e, cx| e.focus(w, cx));
                                    w.request_virtual_keyboard();
                                }))
                        })
                        .child(self.editor.clone()),
                )
        }
        if self.state == State::Failed {
            card = card.child(label(
                tr(cx, "The action failed. Try again.", "操作失败，请重试。"),
                12.,
                p.danger,
            ))
        }
        let enabled = matches!(self.state, State::Confirm | State::Failed) && self.valid(w, cx);
        let busy = matches!(self.state, State::Pending | State::Done);
        self.busy.duration(if busy { 1. } else { 0. }, 0.20, cx);
        let expand = self.busy.get(w, cx).clamp(0., 1.);
        let matched = if self.state == State::Pending {
            0.
        } else if self.state == State::Done || self.state == State::Failed {
            1.
        } else if let Some(phrase) = &self.phrase {
            let typed = self.editor.read(cx).value();
            typed
                .chars()
                .zip(phrase.chars())
                .take_while(|(a, b)| a == b)
                .count() as f32
                / phrase.chars().count().max(1) as f32
        } else {
            0.
        };
        self.fill.duration(matched, 0.20, cx);
        let shade = self.fill.get(w, cx).clamp(0., 1.);
        let row_width = fit(w, 272.) - px(26.);
        let normal_width = px(83.).min(row_width);
        let accept_width = normal_width + (row_width - normal_width) * expand;
        let tint = if self.state == State::Done {
            p.success
        } else {
            p.danger
        };
        let radius = crate::foundation::surface_radius(cx);
        let caption = |color| {
            row()
                .w(accept_width)
                .flex_none()
                .justify_center()
                .when(self.state == State::Done, |b| {
                    b.child(icon(Glyph::Check, 14., color))
                })
                .child(label(go, 13., color))
        };
        let mut accept = button("accept", go, cx)
            .track_focus(&self.accept)
            .w(accept_width)
            .p_0()
            .relative()
            .overflow_hidden()
            .bg(tint.opacity(0.11))
            .child(caption(tint))
            .child(
                div()
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .left_0()
                    .w(accept_width * shade)
                    .overflow_hidden()
                    // GPUI overflow clips to a rectangle. Paint a full rounded face
                    // behind that clip so the progress edge stays straight while
                    // the exposed outer corners match the button in every state.
                    .child(
                        row()
                            .id("confirm-fill-face")
                            .debug_selector(|| "confirm-fill-face".into())
                            .w(accept_width)
                            .h_full()
                            .flex_none()
                            .rounded(radius)
                            .bg(tint)
                            .child(caption(white())),
                    ),
            );
        if self.state == State::Pending && !disabled_motion(cx) {
            let t = (self.status.age(cx) / 1.15).fract();
            w.request_animation_frame();
            accept = accept.child(
                div()
                    .absolute()
                    .left(accept_width * (t * 1.4 - 0.36))
                    .top_0()
                    .bottom_0()
                    .w(accept_width * 0.34)
                    .bg(linear_gradient(
                        90.,
                        linear_color_stop(tint.opacity(0.), 0.),
                        linear_color_stop(tint.opacity(0.28), 1.),
                    )),
            );
        }
        let show_cancel = self.state != State::Pending && self.state != State::Done;
        card = card.child(
            row()
                .justify_end()
                .child(
                    div()
                        .w(px(80.) * (1. - expand))
                        .flex_none()
                        .overflow_hidden()
                        .opacity(1. - expand)
                        .child(
                            button("cancel", cancel, cx)
                                .w(px(72.))
                                .when(!show_cancel, |b| b.tab_index(-1))
                                .child(cancel)
                                .on_click(cx.listener(|s, _, w, cx| s.hide(w, cx))),
                        ),
                )
                .child(
                    accept
                        .when(!enabled, |b| b.tab_stop(false).cursor_default())
                        .when(!enabled && self.state == State::Confirm, |b| b.opacity(0.7))
                        .when(enabled, |b| {
                            b.on_click(cx.listener(|s, _, w, cx| s.submit(w, cx)))
                        }),
                ),
        );
        let bounds = self.bounds.get();
        let height = self.height.clone();
        let natural = if height.get() > px(1.) {
            height.get()
        } else {
            px(if self.phrase.is_some() { 200. } else { 140. })
        };
        let viewport = w.viewport_size();
        let upward = bounds.top() + natural > viewport.height - px(8.) && bounds.bottom() > natural;
        let right = bounds.center().x > viewport.width / 2.;
        let corner = match (right, upward) {
            (true, true) => Anchor::BottomRight,
            (true, false) => Anchor::TopRight,
            (false, true) => Anchor::BottomLeft,
            (false, false) => Anchor::TopLeft,
        };
        let shake = if self.state == State::Failed {
            let t = self.status.time(0.32, w, cx) / 0.32;
            (t * std::f32::consts::TAU * 2.).sin() * (1. - t) * 4.
        } else {
            0.
        };
        let pos = point(
            (if right { bounds.right() } else { bounds.left() }) + px(shake),
            if upward {
                bounds.bottom()
            } else {
                bounds.top()
            },
        );
        let width = fit(w, 272.);
        let shell = panel(cx)
            .shadow_lg()
            .w(bounds.size.width + (width - bounds.size.width) * a)
            .h(bounds.size.height + (natural - bounds.size.height) * a)
            .occlude()
            .child(card)
            .on_children_prepainted(move |rows, _, _| {
                if let Some(b) = rows.first() {
                    height.set(b.size.height);
                }
            });
        root = root.child(overlay(shell, pos, corner));
        root
    }
}
