//! Measured line clamping with an overflow-dependent disclosure.
use crate::primitives::{
    family::{Paint, Slide},
    glyph::{Glyph, transformed},
    nav::*,
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
pub struct ExpandableText {
    text: SharedString,
    lines: usize,
    expanded: bool,
    progress: Slide,
    height: Rc<Cell<Pixels>>,
}
pub struct TextExpanded(pub bool);
impl EventEmitter<TextExpanded> for ExpandableText {}
impl ExpandableText {
    pub fn new(text: impl Into<SharedString>, lines: usize) -> Self {
        Self {
            text: text.into(),
            lines: lines.max(1),
            expanded: false,
            progress: Slide::new(0.),
            height: Default::default(),
        }
    }
}
impl Render for ExpandableText {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let saved = self.height.clone();
        let weak = cx.weak_entity();
        let max = px(21. * self.lines as f32);
        let full = saved.get();
        let a = self.progress.get(w, cx).clamp(0., 1.);
        let overflow = full > max + px(0.5);
        let less = tr(cx, "Show less", "收起");
        let more = tr(cx, "Read more", "阅读全文");
        column()
            .min_w_0()
            .child(
                column()
                    .relative()
                    .overflow_hidden()
                    .h(if full <= px(0.) {
                        max
                    } else {
                        full.min(max) + (full - max).max(px(0.)) * a
                    })
                    .child(
                        label(self.text.clone(), 14., p.secondary)
                            .line_height(px(21.))
                            .flex_none(),
                    )
                    .when(overflow && a < 0.999, |d| {
                        d.child(
                            div()
                                .absolute()
                                .bottom_0()
                                .left_0()
                                .right_0()
                                .h(px(18.))
                                .opacity(1. - a)
                                .bg(linear_gradient(
                                    180.,
                                    linear_color_stop(p.stage.opacity(0.), 0.),
                                    linear_color_stop(p.stage, 1.),
                                )),
                        )
                    })
                    .on_children_prepainted(move |bounds, _, cx| {
                        if let Some(b) = bounds.first()
                            && saved.replace(b.size.height) != b.size.height
                        {
                            let _ = weak.update(cx, |_, cx| cx.notify());
                        }
                    }),
            )
            .when(overflow, |d| {
                d.child(
                    button("more", if self.expanded { less } else { more }, cx)
                        .justify_start()
                        .px_0()
                        .aria_expanded(self.expanded)
                        .child(if self.expanded { less } else { more })
                        .child(transformed(
                            Glyph::Down,
                            16.,
                            p.secondary,
                            std::f32::consts::PI * a,
                            1.,
                            (0., 0.),
                            1.,
                        ))
                        .on_click(cx.listener(|s, _, _, cx| {
                            s.expanded = !s.expanded;
                            let distance =
                                (f32::from(s.height.get()) - 21. * s.lines as f32).max(0.);
                            let duration = if s.expanded {
                                0.5 + (distance / 1600.).min(0.1)
                            } else {
                                0.4 + (distance / 2000.).min(0.05)
                            };
                            s.progress
                                .duration(if s.expanded { 1. } else { 0. }, duration, cx);
                            cx.emit(TextExpanded(s.expanded));
                            cx.notify()
                        })),
                )
            })
    }
}
