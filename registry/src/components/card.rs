//! A content card with optional cover art and independent open/save targets.
use crate::primitives::{
    family::{Paint, Slide, Tone, face},
    flyout::TriggerBounds,
    glyph::{Glyph, icon},
    presentation::*,
};
use gpui::{prelude::*, *};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CardEvent {
    Open,
    Saved(bool),
}
pub struct Card {
    heading: SharedString,
    description: SharedString,
    byline: SharedString,
    cover: Option<ImageSource>,
    saved: bool,
    lift: Slide,
    zoom: Slide,
    presence: Slide,
    open: bool,
    anchor: TriggerBounds,
    trigger: Option<FocusHandle>,
    close: Option<FocusHandle>,
    details: SharedString,
}
impl EventEmitter<CardEvent> for Card {}
impl Card {
    pub fn new(heading: impl Into<SharedString>, description: impl Into<SharedString>) -> Self {
        Self {
            heading: heading.into(),
            description: description.into(),
            byline: "".into(),
            cover: None,
            saved: false,
            lift: Slide::snappy(0.),
            zoom: Slide::new(0.),
            presence: Slide::new(0.),
            open: false,
            anchor: Default::default(),
            trigger: None,
            close: None,
            details: "".into(),
        }
    }
    pub fn with_cover(mut self, image: impl Into<ImageSource>) -> Self {
        self.cover = Some(image.into());
        self
    }
    pub fn with_byline(mut self, text: impl Into<SharedString>) -> Self {
        self.byline = text.into();
        self
    }
    pub fn with_details(mut self, details: impl Into<SharedString>) -> Self {
        self.details = details.into();
        self
    }
    fn show(&mut self, open: bool, w: &mut Window, cx: &mut Context<Self>) {
        if self.open == open {
            return;
        }
        self.open = open;
        self.presence
            .duration(f32::from(open), if open { 0.3 } else { 0.26 }, cx);
        if open {
            cx.emit(CardEvent::Open);
        }
        if let Some(f) = if open {
            self.close.as_ref()
        } else {
            self.trigger.as_ref()
        } {
            f.focus(w, cx);
        }
        cx.notify();
    }
}
impl Render for Card {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let v = self.lift.get(w, cx);
        let trigger = self
            .trigger
            .get_or_insert_with(|| cx.focus_handle().tab_stop(true))
            .clone();
        let close = self
            .close
            .get_or_insert_with(|| cx.focus_handle().tab_stop(true))
            .clone();
        let progress = self.presence.get(w, cx).clamp(0., 1.);
        let zoom = self.zoom.get(w, cx);
        let mut root = div().relative().max_w_full().child(
            frame(340., p)
                .id("content-card")
                .relative()
                .top(px(-2. * v))
                .opacity(if self.open || progress > 0.001 {
                    0.
                } else {
                    1.
                })
                .child(self.anchor.measure())
                .role(Role::Article)
                .p_0()
                .gap_0()
                .overflow_hidden()
                .on_hover(cx.listener(|s, h: &bool, _, cx| {
                    s.lift.set(f32::from(*h), cx);
                    s.zoom
                        .duration(f32::from(*h), if *h { 0.96 } else { 0.48 }, cx);
                    cx.notify()
                }))
                .when_some(self.cover.clone(), |el, img_src| {
                    el.child(
                        div()
                            .id("card-cover")
                            .cursor_pointer()
                            .relative()
                            .w_full()
                            .h(px(170.))
                            .overflow_hidden()
                            .on_click(cx.listener(|s, _, w, cx| s.show(true, w, cx)))
                            .child(
                                img(img_src)
                                    .absolute()
                                    .left(relative(-0.025 * zoom))
                                    .top(px(-4.25 * zoom))
                                    .w(relative(1. + 0.05 * zoom))
                                    .h(px(170. * (1. + 0.05 * zoom)))
                                    .object_fit(ObjectFit::Cover),
                            ),
                    )
                })
                .child(
                    column()
                        .p_4()
                        .gap_2()
                        .child(
                            command("card-open", self.heading.clone(), cx, |s, w, cx| {
                                s.show(true, w, cx)
                            })
                            .track_focus(&trigger)
                            .bg(p.panel)
                            .border_0()
                            .px_0()
                            .justify_start(),
                        )
                        .child(hint(self.description.clone(), p))
                        .child(
                            row()
                                .justify_between()
                                .gap_2()
                                .child(hint(self.byline.clone(), p))
                                .child(command(
                                    "card-bookmark",
                                    if self.saved {
                                        tr(cx, "Saved", "已保存")
                                    } else {
                                        tr(cx, "Save", "保存")
                                    },
                                    cx,
                                    |s, _, cx| {
                                        s.saved = !s.saved;
                                        cx.emit(CardEvent::Saved(s.saved));
                                        cx.notify()
                                    },
                                )),
                        ),
                ),
        );
        if self.open || progress > 0.001 {
            let viewport = w.viewport_size();
            let b = self.anchor.get();
            let width = (viewport.width - px(32.)).min(px(480.));
            let mix = |a: Pixels, b: Pixels| a + (b - a) * progress;
            let sw = mix(b.size.width, width);
            let target_h = (viewport.height - px(24.)).min(px(492.));
            let x = mix(b.left(), (viewport.width - width) * 0.5);
            let y = mix(b.top(), (viewport.height - target_h) * 0.5);
            let mut panel = column()
                .id("card-quicklook")
                .debug_selector(|| "card-quicklook".to_owned())
                .role(Role::Dialog)
                .aria_label(self.heading.clone())
                .absolute()
                .top(y)
                .left(x)
                .w(sw)
                .h(mix(b.size.height, target_h))
                .bg(p.panel)
                .text_color(p.ink)
                .rounded(crate::foundation::surface_radius(cx))
                .overflow_hidden()
                .border_1()
                .border_color(p.line)
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_click(|_, _, cx| cx.stop_propagation())
                .on_key_down(cx.listener(
                    |s, e: &KeyDownEvent, w, cx| match e.keystroke.key.as_ref() {
                        "escape" => {
                            s.show(false, w, cx);
                            cx.stop_propagation();
                        }
                        "tab" => {
                            if let Some(close) = &s.close {
                                close.focus(w, cx);
                            }
                            cx.stop_propagation();
                            w.prevent_default();
                        }
                        _ => {}
                    },
                ));
            if let Some(src) = self.cover.clone() {
                panel = panel.child(
                    img(src)
                        .w_full()
                        .flex_none()
                        .h(mix(px(170.), target_h * 0.52))
                        .object_fit(ObjectFit::Cover),
                );
            }
            panel = panel
                .child(
                    column()
                        .p_4()
                        .gap_2()
                        .child(title(self.heading.clone(), p))
                        .child(hint(self.description.clone(), p))
                        .child(hint(self.byline.clone(), p))
                        .child(
                            div()
                                .pt_3()
                                .opacity(progress * progress)
                                .child(hint(self.details.clone(), p)),
                        ),
                )
                .child(
                    face(
                        "close-card",
                        tr(cx, "Close", "关闭"),
                        32.,
                        8.,
                        Tone::Surface,
                        cx,
                    )
                    .track_focus(&close)
                    .absolute()
                    .top_2()
                    .right_2()
                    .w(px(32.))
                    .opacity(progress)
                    .child(icon(Glyph::Close, 16., p.ink))
                    .on_click(cx.listener(|s, _, w, cx| s.show(false, w, cx))),
                );
            let shade = div()
                .id("card-backdrop")
                .relative()
                .occlude()
                .w(viewport.width)
                .h(viewport.height)
                .bg(black().opacity(0.46 * progress))
                .on_click(cx.listener(|s, _, w, cx| s.show(false, w, cx)))
                .child(panel);
            root = root.child(
                deferred(anchored().position(point(px(0.), px(0.))).child(shade)).with_priority(4),
            );
        }
        root
    }
}
