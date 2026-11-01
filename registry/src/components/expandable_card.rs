//! A summary with an in-place, measured detail region.
use crate::primitives::{
    family::Paint,
    glyph::{Glyph, transformed},
    nav::*,
};
use gpui::{prelude::*, *};
pub struct DetailCard {
    title: SharedString,
    summary: SharedString,
    body: Content,
    fold: Fold,
}
pub struct DetailsChanged(pub bool);
impl EventEmitter<DetailsChanged> for DetailCard {}
impl DetailCard {
    pub fn new(
        title: impl Into<SharedString>,
        summary: impl Into<SharedString>,
        body: Content,
    ) -> Self {
        Self {
            title: title.into(),
            summary: summary.into(),
            body,
            fold: Fold::new(false, 160.),
        }
    }
    pub fn set_expanded(&mut self, v: bool, cx: &mut Context<Self>) {
        if self.fold.open != v {
            self.fold.set_duration(v, if v { 0.36 } else { 0.24 }, cx);
            cx.emit(DetailsChanged(v));
            cx.notify()
        }
    }
}
impl Render for DetailCard {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let content = (self.body)(w, cx);
        let a = self.fold.fraction(w, cx);
        panel(cx)
            .w(fit(w, 390.) + (fit(w, 510.) - fit(w, 390.)) * a)
            .child(
                button("summary", self.title.clone(), cx)
                    .h_auto()
                    .px_4()
                    .py_3()
                    .justify_between()
                    .aria_expanded(self.fold.open)
                    .child(
                        column()
                            .gap_1()
                            .child(label(self.title.clone(), 15., p.ink))
                            .child(label(self.summary.clone(), 12., p.secondary)),
                    )
                    .child(transformed(
                        Glyph::Down,
                        16.,
                        p.secondary,
                        std::f32::consts::PI * a,
                        1.,
                        (0., 0.),
                        1.,
                    ))
                    .on_click(cx.listener(|s, _, _, cx| s.set_expanded(!s.fold.open, cx))),
            )
            .child(
                self.fold.render(
                    column()
                        .w(fit(w, 510.) - px(2.))
                        .opacity(((a - 0.30) / 0.70).clamp(0., 1.))
                        .border_t_1()
                        .border_color(p.line)
                        .p_4()
                        .child(content),
                    w,
                    cx,
                ),
            )
    }
}
