//! Five-slot bounded page navigation. The marker and rolling page labels move independently.
use crate::primitives::{
    family::{Paint, Slide, disabled_motion},
    glyph::{Glyph, icon},
    nav::*,
};
use gpui::{prelude::*, *};
use std::collections::BTreeMap;

pub struct PageChanged(pub usize);
pub struct Pagination {
    count: usize,
    page: usize,
    first: usize,
    belt: Slide,
    marker: Slide,
    opacity: BTreeMap<usize, Slide>,
}
impl EventEmitter<PageChanged> for Pagination {}
impl Pagination {
    fn first_for(page: usize, count: usize) -> usize {
        page.saturating_sub(2)
            .clamp(1, count.saturating_sub(4).max(1))
    }
    fn numbers(first: usize, count: usize) -> std::ops::RangeInclusive<usize> {
        first..=first.saturating_add(4).min(count)
    }
    /// Difference in slots without underflow or loss of precision for large page numbers.
    fn slots(page: usize, first: usize) -> f32 {
        if page >= first {
            (page - first) as f32
        } else {
            -((first - page) as f32)
        }
    }
    pub fn new(count: usize, page: usize) -> Self {
        let count = count.max(1);
        let page = page.clamp(1, count);
        let first = Self::first_for(page, count);
        Self {
            count,
            page,
            first,
            belt: Slide::new(0.),
            marker: Slide::new((page - first) as f32),
            opacity: Self::numbers(first, count)
                .map(|p| (p, Slide::new(1.)))
                .collect(),
        }
    }
    pub fn page(&self) -> usize {
        self.page
    }
    pub fn select(&mut self, page: usize, cx: &mut Context<Self>) {
        let page = page.clamp(1, self.count);
        if page == self.page {
            return;
        }
        let first = Self::first_for(page, self.count);
        if first != self.first {
            let delta = Self::slots(first, self.first);
            if delta.abs() > 5. {
                self.opacity.clear();
            }
            // Rebase every label together. Using one coordinate for the belt means rapid
            // reversals can never change the distance between two neighboring page labels.
            self.belt.translate(delta.clamp(-5., 5.));
            self.belt.duration(0., 0.4, cx);
            let range = Self::numbers(first, self.count);
            for (p, alpha) in &mut self.opacity {
                if !range.contains(p) {
                    alpha.duration(0., 0.09, cx);
                }
            }
            for p in range {
                self.opacity
                    .entry(p)
                    .or_insert_with(|| Slide::new(0.))
                    .duration(1., 0.2, cx);
            }
        }
        self.first = first;
        self.page = page;
        self.marker.duration(Self::slots(page, first), 0.32, cx);
        cx.emit(PageChanged(page));
        cx.notify()
    }
}
impl Render for Pagination {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let theme = crate::foundation::Theme::global(cx);
        let accent = theme.accent.for_mode(theme.mode);
        let selected_bg = p.panel.blend(accent.opacity(0.16));
        let selected_ink = theme
            .accent
            .visible_ink(p.ink.blend(accent), selected_bg, 4.5);
        let visible = self.count.min(5);
        let gap = px(4.);
        let side = ((fit(w, 276.) - gap * (visible + 1) as f32) / (visible + 2) as f32)
            .clamp(px(1.), px(36.));
        let step = side + gap;
        let width = step * visible as f32 - gap;
        let mut body = row()
            .relative()
            .w(width)
            .h(px(36.))
            .overflow_hidden()
            .child(
                div()
                    .absolute()
                    .left(step * self.marker.get(w, cx))
                    .w(side)
                    .h(px(36.))
                    .rounded(crate::foundation::surface_radius(cx))
                    .border_1()
                    .border_color(p.line.blend(accent.opacity(0.65)))
                    .bg(selected_bg),
            );
        let first = self.first;
        let range = Self::numbers(first, self.count);
        let offset = self.belt.get(w, cx);
        let still = disabled_motion(cx);
        self.opacity
            .retain(|page, a| range.contains(page) || (!still && a.value(cx) > 0.001));
        for (&n, alpha) in &self.opacity {
            let at = (Self::slots(n, first) + offset) * step;
            let a = alpha.get(w, cx).clamp(0., 1.);
            let name: SharedString = n.to_string().into();
            if range.contains(&n) {
                body = body.child(
                    button(("page", n), name.clone(), cx)
                        .absolute()
                        .left(at)
                        .w(side)
                        .h(px(36.))
                        .p_0()
                        .opacity(a)
                        .aria_label(format!("{} {n}", tr(cx, "Page", "页码")))
                        .aria_selected(self.page == n)
                        .when(self.page == n, |el| el.text_color(selected_ink))
                        .child(name)
                        .on_click(cx.listener(move |s, _, _, cx| s.select(n, cx))),
                );
            } else {
                body = body.child(
                    row()
                        .absolute()
                        .left(at)
                        .w(side)
                        .h(px(36.))
                        .justify_center()
                        .text_size(px(13.))
                        .text_color(p.ink)
                        .opacity(a)
                        .child(name),
                );
            }
        }
        let arrow = |id, label: &'static str, glyph: Glyph, n: usize, disabled| {
            button(id, label, cx)
                .w(side)
                .h(px(36.))
                .p_0()
                .when(disabled, |b| b.opacity(0.35).tab_index(-1))
                .child(icon(glyph, 14., p.secondary))
                .on_click(cx.listener(move |s, _, _, cx| {
                    if !disabled {
                        s.select(n, cx)
                    }
                }))
        };
        row()
            .gap(gap)
            .child(arrow(
                "prev",
                tr(cx, "Previous page", "上一页"),
                Glyph::Undo,
                self.page.saturating_sub(1),
                self.page == 1,
            ))
            .child(body)
            .child(arrow(
                "next",
                tr(cx, "Next page", "下一页"),
                Glyph::Arrow,
                self.page.saturating_add(1),
                self.page == self.count,
            ))
    }
}
