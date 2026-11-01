//! An ancestor trail with an operable overflow segment.
use crate::primitives::{
    family::{Paint, Slide},
    glyph::{Glyph, icon},
    nav::*,
};
use gpui::{prelude::*, *};
pub struct Breadcrumb {
    segments: Vec<SharedString>,
    current: usize,
    presence: Vec<Slide>,
    mounted: bool,
    limit: usize,
    more: bool,
}
pub struct Navigated(pub usize);
impl EventEmitter<Navigated> for Breadcrumb {}
impl Breadcrumb {
    pub fn new(items: Vec<SharedString>) -> Self {
        Self {
            current: items.len().saturating_sub(1),
            presence: vec![Slide::new(1.); items.len()],
            mounted: false,
            segments: items,
            limit: usize::MAX,
            more: false,
        }
    }
    pub fn limit(mut self, n: usize) -> Self {
        self.limit = n.max(2);
        self
    }
    pub fn navigate(&mut self, i: usize, cx: &mut Context<Self>) {
        if i < self.segments.len() {
            for (index, slide) in self.presence.iter_mut().enumerate() {
                let target = if index <= i { 1. } else { 0. };
                if self.mounted {
                    slide.duration(target, if index <= i { 0.2 } else { 0.10 }, cx)
                } else {
                    slide.reset(target)
                }
            }
            self.current = i;
            self.more = false;
            cx.emit(Navigated(i));
            cx.notify()
        }
    }
    fn crumb(&self, i: usize, cx: &Context<Self>) -> Stateful<Div> {
        let p = Paint::get(cx);
        let name = self.segments[i].clone();
        button(("crumb", i), name.clone(), cx)
            .h(px(28.))
            .px_1()
            .text_color(if i == self.current {
                p.ink
            } else {
                p.secondary
            })
            .child(name)
            .on_click(cx.listener(move |s, _, _, cx| s.navigate(i, cx)))
    }
}
impl Render for Breadcrumb {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        self.mounted = true;
        let n = self.current + 1;
        let limit = if w.viewport_size().width < px(480.) {
            self.limit.min(3)
        } else {
            self.limit
        };
        let hidden = n.saturating_sub(limit);
        let mut trail = row().flex_wrap().gap(px(3.));
        let mut menu = column().gap_1();
        for i in 0..self.segments.len() {
            let a = self.presence[i].get(w, cx).clamp(0., 1.);
            if i >= n && a < 0.001 {
                continue;
            }
            if i > 0 && i <= hidden {
                menu = menu.child(self.crumb(i, cx));
                if i != 1 {
                    continue;
                }
                trail = trail.child(
                    button("more", tr(cx, "Show path", "显示路径"), cx)
                        .child("…")
                        .aria_expanded(self.more)
                        .on_click(cx.listener(|s, _, _, cx| {
                            s.more = !s.more;
                            cx.notify()
                        })),
                )
            } else {
                trail = trail.child(
                    self.crumb(i, cx)
                        .relative()
                        .left(px(-8. * (1. - a)))
                        .opacity(a)
                        .when(i >= n, |b| b.tab_index(-1)),
                )
            }
            if i + 1 < self.segments.len() && self.presence[i + 1].get(w, cx) > 0.001 {
                trail = trail.child(icon(Glyph::Slash, 12., p.faint))
            }
        }
        column()
            .max_w_full()
            .items_start()
            .child(trail)
            .when(self.more && hidden > 0, |b| {
                b.child(panel(cx).p_1().child(menu))
            })
    }
}
