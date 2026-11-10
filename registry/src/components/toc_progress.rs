//! Scroll-linked article and table of contents.
use crate::primitives::{
    editor::EditorScrollRegion,
    family::{Paint, Slide},
    nav::*,
};
use gpui::{prelude::*, *};
use std::{cell::RefCell, rc::Rc};
pub struct ReadingSection {
    pub heading: SharedString,
    pub body: SharedString,
    pub depth: usize,
}
impl ReadingSection {
    pub fn new(
        title: impl Into<SharedString>,
        body: impl Into<SharedString>,
        depth: usize,
    ) -> Self {
        Self {
            heading: title.into(),
            body: body.into(),
            depth,
        }
    }
}
pub struct SectionChanged(pub usize);
pub struct ReadingOutline {
    title: SharedString,
    byline: Option<SharedString>,
    sections: Vec<ReadingSection>,
    scroll: ScrollHandle,
    current: usize,
    minutes: u32,
    reading: Slide,
    jump: Option<(Slide, f32)>,
}
impl EventEmitter<SectionChanged> for ReadingOutline {}
impl ReadingOutline {
    pub fn new(
        title: impl Into<SharedString>,
        sections: Vec<ReadingSection>,
        minutes: u32,
    ) -> Self {
        Self {
            title: title.into(),
            byline: None,
            sections,
            scroll: ScrollHandle::new(),
            current: 0,
            jump: None,
            reading: Slide::new(0.),
            minutes,
        }
    }

    /// Add author and publication information below the article title.
    pub fn with_byline(mut self, byline: impl Into<SharedString>) -> Self {
        self.byline = Some(byline.into());
        self
    }
}
impl Render for ReadingOutline {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);

        let s = self.scroll.clone();
        if let Some((slide, goal)) = &self.jump {
            let y = slide.get(w, cx);
            s.set_offset(point(px(0.), px(y)));
            if (y - goal).abs() < 0.05 {
                self.jump = None
            }
        }
        let offset = -s.offset().y;
        let max = s.max_offset().y;
        let fraction = if max > px(0.) {
            (f32::from(offset) / f32::from(max)).clamp(0., 1.)
        } else {
            0.
        };
        let (index, within) = s.logical_scroll_top();
        let last = self.sections.len().saturating_sub(1);
        let active = if max > px(0.) && offset >= max - px(0.5) {
            last
        } else {
            index.min(last)
        };
        let progress = s.bounds_for_item(active).map_or(0., |b| {
            (f32::from(within.abs()) / f32::from(b.size.height.max(px(1.)))).clamp(0., 1.)
        });
        self.reading
            .duration((active as f32 + progress).min(last as f32), 0.24, cx);
        let marker = self.reading.get(w, cx);
        if active != self.current {
            self.current = active;
            cx.emit(SectionChanged(active))
        }
        let mut article = column()
            .id("article")
            .flex_1()
            .h_full()
            .overflow_y_scroll()
            .track_scroll(&s)
            .on_scroll(cx.listener(|v, _, _, cx| {
                v.jump = None;
                cx.notify()
            }));
        let remaining = format!(
            "{} {}",
            ((1. - fraction) * self.minutes as f32).ceil() as u32,
            tr(cx, "min left", "分钟读完")
        );
        let mut toc = column().relative();
        for (i, v) in self.sections.iter().enumerate() {
            article = article.child(
                column()
                    .flex_none()
                    .px_3()
                    .pb_5()
                    .gap_2()
                    .child(label(
                        v.heading.clone(),
                        if v.depth == 0 { 17. } else { 15. },
                        p.ink,
                    ))
                    .child(label(v.body.clone(), 14., p.secondary).line_height(px(22.))),
            );
            {
                toc = toc.child(
                    button(("section", i), v.heading.clone(), cx)
                        .h_auto()
                        .min_h(px(25.))
                        .py(px(3.))
                        .pl(px(22. + 8. * v.depth.min(4) as f32))
                        .justify_start()
                        .text_color(if i == active { p.ink } else { p.secondary })
                        .child(div().w_full().whitespace_normal().child(v.heading.clone()))
                        .on_click(cx.listener(move |s, _, _, cx| {
                            if let Some(bounds) = s.scroll.bounds_for_item(i) {
                                let goal = f32::from(
                                    (s.scroll.bounds().top() - bounds.top())
                                        .max(-s.scroll.max_offset().y)
                                        .min(px(0.)),
                                );
                                let mut anim = Slide::new(f32::from(s.scroll.offset().y));
                                anim.duration(goal, 0.30, cx);
                                s.jump = Some((anim, goal));
                            }
                            cx.notify()
                        })),
                )
            }
        }
        let depths = self
            .sections
            .iter()
            .map(|v| v.depth.min(4))
            .collect::<Vec<_>>();
        let count = depths.len();
        let layout = Rc::new(RefCell::new(Vec::with_capacity(count)));
        let paint_layout = layout.clone();
        let rail = canvas(
            |_, _, _| (),
            move |_, _, w, _| {
                let boxes = paint_layout.borrow();
                let centers = boxes
                    .iter()
                    .zip(&depths)
                    .map(|(b, depth): (&Bounds<Pixels>, &usize)| {
                        point(b.left() + px(10. + *depth as f32 * 8.), b.center().y)
                    })
                    .collect::<Vec<_>>();
                if let Some(first) = centers.first().copied() {
                    let mut path = PathBuilder::stroke(px(1.));
                    path.move_to(first);
                    for pair in centers.windows(2) {
                        let middle = pair[0].y + (pair[1].y - pair[0].y) * 0.5;
                        path.line_to(point(pair[0].x, middle - px(3.)));
                        path.line_to(point(pair[1].x, middle + px(3.)));
                        path.line_to(pair[1]);
                    }
                    if let Ok(path) = path.build() {
                        w.paint_path(path, p.line);
                    }
                    let index = (marker.floor() as usize).min(centers.len() - 1);
                    let from = centers[index];
                    let to = centers.get(index + 1).copied().unwrap_or(from);
                    let fraction = marker.fract();
                    let y = from.y + (to.y - from.y) * fraction;
                    let middle = from.y + (to.y - from.y) * 0.5;
                    let bend = (f32::from(y - middle + px(3.)) / 6.).clamp(0., 1.);
                    let position = point(from.x + (to.x - from.x) * bend, y);
                    w.paint_quad(
                        fill(
                            Bounds::new(position - point(px(3.), px(3.)), size(px(6.), px(6.))),
                            p.ink,
                        )
                        .corner_radii(px(3.)),
                    );
                }
            },
        )
        .absolute()
        .inset_0();
        let toc = panel(cx)
            .p_4()
            .child(
                row()
                    .justify_between()
                    .pb_3()
                    .child(
                        label(tr(cx, "On this page", "本页目录"), 14., p.ink)
                            .font_weight(FontWeight::MEDIUM),
                    )
                    .child(label(remaining, 12., p.faint)),
            )
            .child(toc.child(rail).on_children_prepainted(move |boxes, _, _| {
                *layout.borrow_mut() = boxes.iter().take(count).copied().collect();
            }));
        let monitor = canvas(
            |_, _, _| (),
            move |b, _, _, cx| {
                if let Some(host) = cx.try_global::<EditorScrollRegion>() {
                    (host.0)(
                        b,
                        -s.offset().y > px(0.1),
                        -s.offset().y + px(0.1) < s.max_offset().y,
                    )
                }
            },
        )
        .absolute()
        .inset_0();
        let height = px(416.);
        column().w(fit(w, 520.)).gap_3().child(toc).child(
            panel(cx)
                .p_4()
                .gap_4()
                .child(
                    column()
                        .px_3()
                        .gap_1()
                        .child(
                            label(self.title.clone(), 26., p.ink)
                                .font_weight(FontWeight::MEDIUM)
                                .line_height(px(31.)),
                        )
                        .when_some(self.byline.clone(), |head, byline| {
                            head.child(label(byline, 12., p.faint))
                        }),
                )
                .child(div().relative().h(height).child(article).child(monitor)),
        )
    }
}
