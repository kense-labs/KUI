//! Expand tasks and update completion independently of the disclosure state.
use crate::primitives::{
    family::{Paint, Slide},
    glyph::{Glyph, countdown_ring, transformed},
    nav::Fold,
    presentation::*,
};
use gpui::{prelude::*, *};
#[derive(Clone)]
pub struct ChecklistItem {
    pub title: SharedString,
    pub description: SharedString,
    pub action: SharedString,
    pub done: bool,
}
impl ChecklistItem {
    pub fn new(
        title: impl Into<SharedString>,
        description: impl Into<SharedString>,
        action: impl Into<SharedString>,
    ) -> Self {
        Self {
            title: title.into(),
            description: description.into(),
            action: action.into(),
            done: false,
        }
    }
}
#[derive(Clone, Copy)]
pub struct TaskChanged {
    pub index: usize,
    pub done: bool,
}
struct TaskMotion {
    detail: Fold,
    listed: Fold,
    completion: Slide,
    settled: bool,
    revision: usize,
}
pub struct SetupChecklist {
    heading: SharedString,
    tasks: Vec<ChecklistItem>,
    rows: Vec<TaskMotion>,
    body: Fold,
    archive: Fold,
    ratio: Slide,
}
impl EventEmitter<TaskChanged> for SetupChecklist {}
impl SetupChecklist {
    pub fn new(heading: impl Into<SharedString>, tasks: Vec<ChecklistItem>) -> Self {
        let expanded = tasks.iter().position(|t| !t.done);
        let rows = tasks
            .iter()
            .enumerate()
            .map(|(i, t)| TaskMotion {
                detail: Fold::new(expanded == Some(i), 68.),
                listed: Fold::new(!t.done, 42.),
                completion: Slide::new(t.done as u8 as f32),
                settled: t.done,
                revision: 0,
            })
            .collect();
        let ratio = tasks.iter().filter(|t| t.done).count() as f32 / tasks.len().max(1) as f32;
        Self {
            heading: heading.into(),
            tasks,
            rows,
            body: Fold::new(true, 260.),
            archive: Fold::new(false, 42.),
            ratio: Slide::new(ratio),
        }
    }
    fn expand(&mut self, index: Option<usize>, cx: &App) {
        for (i, row) in self.rows.iter_mut().enumerate() {
            let open = index == Some(i);
            row.detail
                .set_duration(open, if open { 0.4 } else { 0.32 }, cx);
        }
    }
    fn toggle(&mut self, i: usize, cx: &mut Context<Self>) {
        let done = !self.tasks[i].done;
        self.tasks[i].done = done;
        let row = &mut self.rows[i];
        row.revision += 1;
        let revision = row.revision;
        row.completion.duration(done as u8 as f32, 0.26, cx);
        if done {
            cx.spawn(async move |v, cx| {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(720))
                    .await;
                let _ = v.update(cx, |s, cx| {
                    if s.rows[i].revision != revision || !s.tasks[i].done {
                        return;
                    }
                    s.rows[i].settled = true;
                    s.rows[i].listed.set_duration(false, 0.32, cx);
                    if s.rows[i].detail.open {
                        s.expand(s.tasks.iter().position(|t| !t.done), cx);
                    }
                    cx.notify();
                });
            })
            .detach();
        } else {
            row.settled = false;
            row.listed.set_duration(true, 0.4, cx);
            self.expand(Some(i), cx);
        }
        self.ratio.duration(
            self.tasks.iter().filter(|t| t.done).count() as f32 / self.tasks.len().max(1) as f32,
            0.42,
            cx,
        );
        cx.emit(TaskChanged { index: i, done });
        cx.notify()
    }
}
impl Render for SetupChecklist {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let done = self.tasks.iter().filter(|t| t.done).count();
        let mut el = frame(450., p).gap_0().child(
            row()
                .justify_between()
                .child(
                    row()
                        .gap_2()
                        .child(countdown_ring(self.ratio.get(w, cx), 36., p.ink))
                        .child(column().child(title(self.heading.clone(), p)).child(hint(
                            format!(
                                "{done}/{} {}",
                                self.tasks.len(),
                                tr(cx, "complete", "已完成")
                            ),
                            p,
                        ))),
                )
                .child(
                    command(
                        "checklist-fold",
                        if self.body.open {
                            tr(cx, "Hide", "收起")
                        } else {
                            tr(cx, "Show", "显示")
                        },
                        cx,
                        |v, _, cx| {
                            let open = !v.body.open;
                            v.body.set_duration(open, if open { 0.4 } else { 0.34 }, cx);
                            cx.notify()
                        },
                    )
                    .aria_expanded(self.body.open),
                ),
        );
        if !self.body.open && self.body.fraction(w, cx) < 0.001 {
            return el;
        }
        let mut items = column().pt_2();
        for (i, (task, state)) in self.tasks.iter().zip(&mut self.rows).enumerate() {
            if !state.listed.open && state.listed.fraction(w, cx) < 0.001 {
                continue;
            }
            let f = state.completion.get(w, cx).clamp(0., 1.);
            let mark = row()
                .relative()
                .size(px(20.))
                .justify_center()
                .rounded(crate::foundation::surface_radius(cx))
                .border_1()
                .border_color(p.line)
                .child(
                    div()
                        .absolute()
                        .size(px(12. + 8. * f))
                        .rounded(crate::foundation::surface_radius(cx))
                        .bg(p.ink.opacity(f)),
                )
                .child(div().relative().child(transformed(
                    Glyph::Check,
                    14.,
                    p.neutral_inverse,
                    0.,
                    1.,
                    (0., 0.),
                    f,
                )));
            let header = row()
                .gap_2()
                .child(
                    command(("complete", i), "", cx, move |v, _, cx| v.toggle(i, cx))
                        .aria_label(task.title.clone())
                        .size(px(34.))
                        .p_0()
                        .border_0()
                        .child(mark),
                )
                .child(
                    command(("explain", i), task.title.clone(), cx, move |v, _, cx| {
                        v.expand(if v.rows[i].detail.open { None } else { Some(i) }, cx);
                        cx.notify();
                    })
                    .aria_expanded(state.detail.open)
                    .border_0()
                    .when(task.done, |d| d.line_through().text_color(p.secondary)),
                );
            let mut body = column().child(header);
            if state.detail.open || state.detail.fraction(w, cx) > 0.001 {
                let details = column()
                    .ml_4()
                    .pl_4()
                    .py_1()
                    .items_start()
                    .gap_2()
                    .child(hint(task.description.clone(), p))
                    .child(command(
                        ("finish-task", i),
                        task.action.clone(),
                        cx,
                        move |v, _, cx| {
                            if !v.tasks[i].done {
                                v.toggle(i, cx)
                            }
                        },
                    ));
                body = body.child(state.detail.render(details, w, cx));
            }
            items = items.child(state.listed.render(body, w, cx));
        }
        let archived = self.rows.iter().filter(|s| s.settled).count();
        if archived > 0 {
            items = items.child(
                command(
                    "checklist-completed",
                    format!("{archived} {}", tr(cx, "completed", "项已完成")),
                    cx,
                    |v, _, cx| {
                        let open = !v.archive.open;
                        v.archive
                            .set_duration(open, if open { 0.4 } else { 0.32 }, cx);
                        cx.notify()
                    },
                )
                .aria_expanded(self.archive.open)
                .border_0()
                .justify_start(),
            );
            if self.archive.open || self.archive.fraction(w, cx) > 0.001 {
                let rows = column().children(
                    self.tasks
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| self.rows[*i].settled)
                        .map(|(i, t)| {
                            command(("undo-task", i), t.title.clone(), cx, move |v, _, cx| {
                                v.toggle(i, cx)
                            })
                            .justify_start()
                            .border_0()
                            .line_through()
                        }),
                );
                items = items.child(self.archive.render(rows, w, cx));
            }
        }
        el = el.child(self.body.render(items, w, cx));
        el
    }
}
