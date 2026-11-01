//! Chronological entries grouped by day with an inline detail disclosure.
use crate::primitives::{
    family::{Paint, Slide},
    nav::Fold,
    presentation::*,
};
use gpui::{prelude::*, *};
#[derive(Clone)]
pub struct TimelineEntry {
    pub group: SharedString,
    pub title: SharedString,
    pub detail: SharedString,
    pub time: SharedString,
}
impl TimelineEntry {
    pub fn new(
        group: impl Into<SharedString>,
        title: impl Into<SharedString>,
        detail: impl Into<SharedString>,
        time: impl Into<SharedString>,
    ) -> Self {
        Self {
            group: group.into(),
            title: title.into(),
            detail: detail.into(),
            time: time.into(),
        }
    }
}
struct Row {
    key: usize,
    entry: TimelineEntry,
    presence: Fold,
    detail: Fold,
    marker: Slide,
    stem: Slide,
}
impl Row {
    fn new(key: usize, entry: TimelineEntry, visible: bool) -> Self {
        Self {
            key,
            entry,
            presence: Fold::new(visible, 44.),
            detail: Fold::new(false, 28.),
            marker: Slide::new(visible as u8 as f32),
            stem: Slide::new(visible as u8 as f32),
        }
    }
}
pub struct Timeline {
    rows: Vec<Row>,
}
impl Timeline {
    pub fn new(entries: Vec<TimelineEntry>) -> Self {
        Self {
            rows: entries
                .into_iter()
                .enumerate()
                .map(|(i, e)| Row::new(i, e, true))
                .collect(),
        }
    }
    fn disclose(&mut self, key: usize, cx: &mut Context<Self>) {
        for row in &mut self.rows {
            let open = row.key == key && !row.detail.open;
            row.detail
                .set_duration(open, if open { 0.4 } else { 0.28 }, cx);
        }
        cx.notify()
    }
    pub fn prepend(&mut self, entry: TimelineEntry, cx: &mut Context<Self>) {
        let key = self.rows.len();
        let mut row = Row::new(key, entry, false);
        row.presence.set_duration(true, 0.4, cx);
        row.marker.duration(1., 0.26, cx);
        row.stem.delayed(1., 0.4, 0.10, cx);
        self.rows.insert(0, row);
        cx.notify()
    }
}
impl Render for Timeline {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let mut list = column()
            .id("timeline-scroll")
            .w(px(480.))
            .max_w_full()
            .max_h(px(320.))
            .overflow_y_scroll()
            .role(Role::Feed);
        let mut group = None;
        for row_state in &mut self.rows {
            let entry = &row_state.entry;
            if group.as_ref() != Some(&entry.group) {
                list = list.child(title(entry.group.clone(), p).py_2());
                group = Some(entry.group.clone());
            }
            let key = row_state.key;
            let marker = row_state.marker.get(w, cx).clamp(0., 1.);
            let stem = row_state.stem.get(w, cx).clamp(0., 1.);
            let mut copy = column().flex_1().min_w_0().child(
                command(
                    ("timeline-item", key),
                    entry.title.clone(),
                    cx,
                    move |v, _, cx| v.disclose(key, cx),
                )
                .aria_expanded(row_state.detail.open)
                .w_full()
                .justify_start()
                .border_0()
                .text_size(px(13.))
                .overflow_hidden(),
            );
            let detail = row_state.detail.fraction(w, cx);
            if row_state.detail.open || detail > 0.001 {
                copy = copy.child(row_state.detail.render(
                    hint(entry.detail.clone(), p).pl_3().pb_1(),
                    w,
                    cx,
                ));
            }
            let body =
                row()
                    .items_start()
                    .gap_2()
                    .py(px(2.))
                    .child(
                        div()
                            .relative()
                            .w(px(12.))
                            .self_stretch()
                            .flex_none()
                            .child(
                                div()
                                    .absolute()
                                    .top(px(20.))
                                    .bottom_0()
                                    .left(px(5.))
                                    .w(px(1.))
                                    .child(div().w_full().h(relative(stem)).bg(p.line)),
                            )
                            .child(div().absolute().top(px(12.)).left(px(2.)).child(dot(
                                7. * (0.4 + 0.6 * marker),
                                p.secondary.opacity(marker),
                            ))),
                    )
                    .child(copy)
                    .child(hint(entry.time.clone(), p).pt_2());
            list = list.child(row_state.presence.render(body, w, cx));
        }
        list
    }
}
