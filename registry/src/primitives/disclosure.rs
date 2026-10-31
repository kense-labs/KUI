//! Focus, dismissal and animated presence for contextual surfaces.
use crate::{
    foundation::Theme,
    primitives::{
        control::{UiLanguage, activate},
        family::{Paint, Slide, Tone, face},
        flyout::TriggerBounds,
        glyph::{Glyph, icon},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, marker::PhantomData, rc::Rc};

pub trait SurfaceStyle: 'static {
    const MODAL: bool;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceEvent {
    Opened,
    Closed,
    Confirmed,
}
type Body = Box<dyn Fn(&mut Window, &mut App) -> AnyElement>;
pub struct Surface<S: SurfaceStyle> {
    id: ElementId,
    label: SharedString,
    title: SharedString,
    description: SharedString,
    body: Option<Body>,
    confirm: Option<SharedString>,
    cancel: Option<SharedString>,
    glyph: Option<Glyph>,
    open: bool,
    presence: Slide,
    anchor: TriggerBounds,
    height: Rc<Cell<Pixels>>,
    trigger: FocusHandle,
    panel: FocusHandle,
    first: FocusHandle,
    end: FocusHandle,
    marker: PhantomData<S>,
}
impl<S: SurfaceStyle> Focusable for Surface<S> {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.trigger.clone()
    }
}
impl<S: SurfaceStyle> EventEmitter<SurfaceEvent> for Surface<S> {}
impl<S: SurfaceStyle> Surface<S> {
    pub fn new(
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        title: impl Into<SharedString>,
        cx: &App,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            title: title.into(),
            description: "".into(),
            body: None,
            confirm: None,
            cancel: None,
            glyph: None,
            open: false,
            presence: Slide::new(0.),
            anchor: Default::default(),
            height: Default::default(),
            trigger: cx.focus_handle().tab_stop(true),
            panel: cx.focus_handle(),
            end: cx.focus_handle().tab_stop(true),
            first: cx.focus_handle().tab_stop(true),
            marker: PhantomData,
        }
    }
    pub fn with_icon(mut self, g: Glyph) -> Self {
        self.glyph = Some(g);
        self
    }
    pub fn with_description(mut self, d: impl Into<SharedString>) -> Self {
        self.description = d.into();
        self
    }
    pub fn with_body<E: IntoElement + 'static>(
        mut self,
        f: impl Fn(&mut Window, &mut App) -> E + 'static,
    ) -> Self {
        self.body = Some(Box::new(move |w, cx| f(w, cx).into_any_element()));
        self
    }
    pub fn with_confirm(mut self, label: impl Into<SharedString>) -> Self {
        self.confirm = Some(label.into());
        self
    }
    pub fn with_cancel(mut self, label: impl Into<SharedString>) -> Self {
        self.cancel = Some(label.into());
        self
    }
    pub fn is_open(&self) -> bool {
        self.open
    }
    pub fn set_open(&mut self, open: bool, w: &mut Window, cx: &mut Context<Self>) {
        if self.open == open {
            return;
        }
        self.open = open;
        self.presence.duration(
            if open { 1. } else { 0. },
            if open { 0.32 } else { 0.16 },
            cx,
        );
        if open {
            if S::MODAL {
                self.first.focus(w, cx)
            } else {
                self.panel.focus(w, cx)
            }
        } else {
            self.trigger.focus(w, cx)
        }
        cx.emit(if open {
            SurfaceEvent::Opened
        } else {
            SurfaceEvent::Closed
        });
        cx.notify()
    }
    fn command(&self, label: SharedString, primary: bool, cx: &Context<Self>) -> Stateful<Div> {
        let r = Theme::global(cx).tokens.radius.md.into();
        let p = Paint::get(cx);
        activate(
            face(
                if primary { "accept" } else { "cancel" },
                label.clone(),
                36.,
                r,
                if primary {
                    Tone::Primary
                } else {
                    Tone::Surface
                },
                cx,
            )
            .px_3()
            .text_size(px(13.))
            .when(!S::MODAL, |e| e.flex_1())
            .when(!primary, |e| e.border_1().border_color(p.line))
            .hover(|e| e.opacity(0.84))
            .child(label),
            cx,
            move |v, w, cx| {
                if v.open {
                    v.set_open(false, w, cx);
                    if primary {
                        cx.emit(SurfaceEvent::Confirmed)
                    }
                }
            },
        )
    }
}
impl<S: SurfaceStyle> Render for Surface<S> {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let r = Theme::global(cx).tokens.radius.md;
        let trig = activate(
            face(
                self.id.clone(),
                self.label.clone(),
                34.,
                r.into(),
                if S::MODAL {
                    Tone::Primary
                } else {
                    Tone::Surface
                },
                cx,
            )
            .text_size(px(13.))
            .track_focus(&self.trigger)
            .relative()
            .px_3()
            .border_1()
            .border_color(p.line)
            .aria_expanded(self.open)
            .hover(move |e| {
                e.bg(if S::MODAL {
                    p.bright.blend(p.inverse.opacity(0.12))
                } else {
                    p.hover
                })
            })
            .when_some(self.glyph, |el, g| el.child(icon(g, 16., p.secondary)))
            .child(self.label.clone())
            .child(self.anchor.measure()),
            cx,
            |v, w, cx| v.set_open(!v.open, w, cx),
        );
        let mut root = div().flex().child(trig);
        let a = self.presence.get(w, cx).clamp(0., 1.);
        if !self.open && a < 0.002 {
            return root;
        }
        let size = w.viewport_size();
        let width = (size.width - px(if S::MODAL { 32. } else { 20. }))
            .min(px(if S::MODAL { 440. } else { 256. }))
            .max(px(1.));
        let shift = 1. - a;
        let mut header = div().flex().items_start().gap_3();
        let mut copy = div().flex_1().min_w_0().flex().flex_col().gap_2().child(
            div()
                .font_weight(FontWeight::MEDIUM)
                .text_size(px(if S::MODAL { 18. } else { 14. }))
                .child(self.title.clone()),
        );
        if !self.description.is_empty() {
            copy = copy.child(
                div()
                    .text_color(p.secondary)
                    .text_size(px(if S::MODAL { 14. } else { 13. }))
                    .line_height(px(if S::MODAL { 21. } else { 18. }))
                    .child(self.description.clone()),
            )
        }
        header = header.child(copy);
        if S::MODAL {
            let name = UiLanguage::current(cx).text("Close", "关闭");
            header = header
                .p_6()
                .border_b_1()
                .border_color(p.line)
                .child(activate(
                    face("dismiss", name, 32., r.into(), Tone::Surface, cx)
                        .track_focus(&self.first)
                        .w(px(32.))
                        .border_1()
                        .border_color(p.line)
                        .child(icon(Glyph::Close, 16., p.secondary)),
                    cx,
                    |v, w, cx| v.set_open(false, w, cx),
                ))
        }
        let mut content = div().flex().flex_col().gap_2().when(S::MODAL, |e| e.p_6());
        if let Some(body) = &self.body {
            content = content.child(body(w, cx))
        }
        let mut row = div()
            .flex()
            .items_center()
            .flex_wrap()
            .gap_2()
            .when(S::MODAL, |e| e.justify_end());
        if let Some(label) = self.cancel.clone() {
            row = row.child(self.command(label, false, cx))
        }
        if let Some(label) = self.confirm.clone() {
            row = row.child(self.command(label, true, cx))
        }
        let open = self.open;
        let saved = self.height.clone();
        let anchor = self.anchor.clone();
        let panel = div()
            .id("surface-panel")
            .role(Role::Dialog)
            .aria_label(self.title.clone())
            .track_focus(&self.panel)
            .tab_stop(false)
            .tab_group()
            .occlude()
            .overflow_y_scroll()
            .w(width * (1. - shift * if S::MODAL { 0.04 } else { 0.03 }))
            .max_h(size.height - px(32.))
            .flex()
            .flex_col()
            .when(!S::MODAL, |e| e.p_4().gap_3())
            .rounded(Theme::global(cx).tokens.radius.lg)
            .bg(p.panel)
            .text_color(p.ink)
            .border_1()
            .border_color(p.line)
            .shadow_lg()
            .opacity(a)
            .on_click(|_, _, cx| cx.stop_propagation())
            .child(header)
            .child(content.child(row))
            .when(S::MODAL, |e| {
                e.child(
                    div()
                        .id("focus-end")
                        .track_focus(&self.end)
                        .tab_stop(true)
                        .absolute()
                        .size(px(0.)),
                )
            })
            .on_key_down(cx.listener(
                |v, e: &KeyDownEvent, w, cx| match e.keystroke.key.as_str() {
                    "escape" => {
                        v.set_open(false, w, cx);
                        cx.stop_propagation()
                    }
                    "tab" if S::MODAL => {
                        let back = e.keystroke.modifiers.shift;
                        if back {
                            w.focus_prev(cx)
                        } else {
                            w.focus_next(cx)
                        };
                        if !v.panel.contains_focused(w, cx) || v.end.is_focused(w) {
                            if back {
                                v.end.focus(w, cx);
                                w.focus_prev(cx)
                            } else {
                                v.first.focus(w, cx)
                            }
                        }
                        cx.stop_propagation()
                    }
                    _ => {}
                },
            ))
            .on_mouse_down_out(cx.listener(move |v, e: &MouseDownEvent, w, cx| {
                if open && !S::MODAL && !anchor.get().contains(&e.position) {
                    v.set_open(false, w, cx)
                }
            }));
        let panel = div()
            .child(panel)
            .on_children_prepainted(move |bounds, _, _| {
                if let Some(b) = bounds.first() {
                    saved.set(b.size.height)
                }
            });
        let layer = if S::MODAL {
            let mut shade = div()
                .id("backdrop")
                .size(size.width)
                .h(size.height)
                .flex()
                .justify_center()
                .items_center()
                .pt(px(16. * shift))
                .bg(black().opacity(0.46 * a))
                .child(panel);
            if open {
                shade = shade
                    .occlude()
                    .on_click(cx.listener(|v, _, w, cx| v.set_open(false, w, cx)))
            }
            anchored().position(point(px(0.), px(0.))).child(shade)
        } else {
            let (pos, side) = self
                .anchor
                .place(size, self.height.get().max(px(120.)), px(6.));
            let sign = if side == Anchor::TopLeft { -1. } else { 1. };
            anchored()
                .position(pos + point(px(0.), px(sign * 5. * shift)))
                .anchor(side)
                .snap_to_window_with_margin(px(10.))
                .child(panel)
        };
        root = root.child(deferred(layer).with_priority(4));
        root
    }
}
