//! Scroll-linked article and table of contents.
use crate::primitives::{
    editor::EditorScrollRegion,
    family::{Paint, Slide},
    nav::*,
};
use gpui::{prelude::*, *};
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
            sections,
            scroll: ScrollHandle::new(),
            current: 0,
            jump: None,
            reading: Slide::new(0.),
            minutes,
        }
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
        let mut toc = panel(cx).p_3().child(
            row()
                .justify_between()
                .pb_2()
                .child(label(tr(cx, "On this page", "本页目录"), 14., p.ink))
                .child(label(remaining, 12., p.faint)),
        );
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
                        .py_1()
                        .pl(px(8. + 8. * v.depth as f32))
                        .justify_start()
                        .h(px(25.))
                        .py_0()
                        .text_color(if i == active { p.ink } else { p.secondary })
                        .child(
                            div()
                                .relative()
                                .w(px(8.))
                                .h_full()
                                .border_l_1()
                                .border_color(p.line)
                                .when(i == marker.floor() as usize, |d| {
                                    d.child(
                                        div()
                                            .absolute()
                                            .top(px(10. + 25. * marker.fract()))
                                            .left(px(-3.))
                                            .size(px(5.))
                                            .rounded_full()
                                            .bg(p.ink),
                                    )
                                }),
                        )
                        .child(v.heading.clone())
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
        let height = px(312.);
        column().w(fit(w, 520.)).gap_3().child(toc).child(
            panel(cx)
                .p_4()
                .gap_3()
                .child(label(self.title.clone(), 20., p.ink))
                .child(div().relative().h(height).child(article).child(monitor)),
        )
    }
}
