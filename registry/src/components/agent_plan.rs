//! Review an ordered execution plan and change the sequence of its steps.
use crate::primitives::ops_surface::*;
use crate::primitives::{
    editor::{EditEvent, Editor, InputMode},
    family::Paint,
    glyph::{Glyph, icon},
};
use gpui::{prelude::*, *};
pub enum PlanEvent {
    Changed,
    Approve,
    Revise,
}
pub struct TaskPlan {
    id: SharedString,
    pub goal: String,
    pub steps: Vec<String>,
    pub completed: usize,
    pub approved: bool,
    editing: Option<usize>,
    menu: Option<usize>,
    edit: Entity<Editor>,
    _sub: Subscription,
}
impl EventEmitter<PlanEvent> for TaskPlan {}
impl TaskPlan {
    pub fn new(
        id: impl Into<SharedString>,
        goal: impl Into<String>,
        steps: Vec<String>,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let edit = editor(
            tr(cx, "Step name", "步骤名称").into(),
            InputMode::Single,
            w,
            cx,
        );
        let sub = cx.subscribe_in(&edit, w, |s: &mut Self, _, e: &EditEvent, _, cx| match e {
            EditEvent::Submitted => s.commit(cx),
            EditEvent::Escape => {
                s.editing = None;
                cx.notify()
            }
            _ => {}
        });
        Self {
            id: id.into(),
            goal: goal.into(),
            steps,
            completed: 0,
            approved: false,
            editing: None,
            menu: None,
            edit,
            _sub: sub,
        }
    }
    fn rename(&mut self, i: usize, w: &mut Window, cx: &mut Context<Self>) {
        if self.approved {
            return;
        }
        self.editing = Some(i);
        replace(
            &self.edit,
            self.steps.get(i).cloned().unwrap_or_default(),
            w,
            cx,
        );
        self.edit.read(cx).focus.clone().focus(w, cx);
        cx.notify()
    }
    fn commit(&mut self, cx: &mut Context<Self>) {
        if let Some(i) = self.editing {
            let v = self.edit.read(cx).value().trim().to_string();
            if v.is_empty() {
                return;
            }
            if i == self.steps.len() {
                self.steps.push(v)
            } else if let Some(s) = self.steps.get_mut(i) {
                *s = v
            }
            self.editing = None;
            cx.emit(PlanEvent::Changed);
            cx.notify()
        }
    }
    fn shift(&mut self, i: usize, down: bool, cx: &mut Context<Self>) {
        let j = if down { i + 1 } else { i.saturating_sub(1) };
        if !self.approved && i < self.steps.len() && j < self.steps.len() && i != j {
            self.steps.swap(i, j);
            cx.emit(PlanEvent::Changed);
            cx.notify()
        }
    }
}
// Keep the same layout slot throughout the run, even when a numbered step turns into a check.
fn plan_marker(i: usize, fraction: f32, p: Paint) -> Div {
    let disc = div()
        .debug_selector(move || format!("plan-disc-{i}"))
        .size(px(14.))
        .rounded_full()
        .bg(p.success.opacity(fraction));
    let check = canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            if fraction <= 0. {
                return;
            }
            let center = bounds.center();
            let start = center + point(px(-3.), px(0.25));
            let elbow = center + point(px(-0.9), px(2.3));
            let end = center + point(px(3.), px(-2.));
            let mut path = PathBuilder::stroke(px(1.6));
            path.move_to(start);
            let first = (elbow - start).magnitude() as f32;
            let second = (end - elbow).magnitude() as f32;
            let length = (first + second) * fraction;
            let head = if length <= first {
                start + (elbow - start) * (length / first)
            } else {
                path.line_to(elbow);
                elbow + (end - elbow) * ((length - first) / second)
            };
            path.line_to(head);
            if let Ok(path) = path.build() {
                window.paint_path(path, p.panel);
                // Keep the cap contours in the same fractional coordinate space as the stroke.
                let mut caps = PathBuilder::fill();
                let radius = point(px(0.8), px(0.8));
                let dx = point(radius.x, px(0.));
                for tip in [start, head] {
                    caps.move_to(tip - dx);
                    caps.arc_to(radius, px(0.), false, true, tip + dx);
                    caps.arc_to(radius, px(0.), false, true, tip - dx);
                    caps.close();
                }
                if let Ok(caps) = caps.build() {
                    window.paint_path(caps, p.panel);
                }
            }
        },
    )
    .size(px(12.));
    div()
        .debug_selector(move || format!("plan-marker-{i}"))
        .relative()
        .flex_none()
        .size(px(20.))
        .flex()
        .items_center()
        .justify_center()
        .child(disc)
        .child(
            div()
                .absolute()
                .inset_0()
                .flex()
                .items_center()
                .justify_center()
                .opacity(1. - fraction)
                .text_size(px(11.))
                .child((i + 1).to_string()),
        )
        .child(
            div()
                .absolute()
                .inset_0()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .debug_selector(move || format!("plan-check-{i}"))
                        .size(px(12.))
                        .child(check),
                ),
        )
}
impl Render for TaskPlan {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let mut body = column().gap(px(3.));
        for (i, name) in self.steps.iter().enumerate() {
            if self.editing == Some(i) {
                body = body.child(input(&self.edit, cx));
                continue;
            }
            // Keep every step’s progress animation alive across approval.
            let fraction = animated_value(
                ("check-draw", i),
                f32::from(i < self.completed),
                0.22,
                w,
                cx,
            );
            if self.approved {
                let circle = plan_marker(i, fraction, p);
                body = body.child(
                    row().min_h(px(34.)).child(circle).child(
                        div()
                            .text_color(p.ink.blend(p.secondary.opacity(fraction)))
                            .child(name.clone()),
                    ),
                );
                continue;
            }
            let handle = icon_btn(
                ("handle", i),
                tr(cx, "Step actions", "步骤操作"),
                Glyph::Menu,
                cx,
                move |s, _, cx| {
                    s.menu = if s.menu == Some(i) { None } else { Some(i) };
                    cx.notify()
                },
            );
            let label = item(("step", i), name.clone(), cx, move |s, w, cx| {
                s.rename(i, w, cx)
            })
            .flex_1()
            .child(name.clone());
            body = body.child(row().id(("row", i)).child(handle).child(label).on_key_down(
                cx.listener(move |s, e: &KeyDownEvent, _, cx| {
                    if e.keystroke.modifiers.alt
                        && matches!(e.keystroke.key.as_str(), "up" | "down")
                    {
                        s.shift(i, e.keystroke.key == "down", cx);
                        cx.stop_propagation()
                    }
                }),
            ));
            if self.menu == Some(i) {
                body = body.child(
                    row()
                        .flex_wrap()
                        .child(icon_btn(
                            ("up", i),
                            tr(cx, "Move up", "上移"),
                            Glyph::Up,
                            cx,
                            move |s, _, cx| {
                                s.shift(i, false, cx);
                                s.menu = Some(i.saturating_sub(1))
                            },
                        ))
                        .child(icon_btn(
                            ("down", i),
                            tr(cx, "Move down", "下移"),
                            Glyph::Down,
                            cx,
                            move |s, _, cx| {
                                s.shift(i, true, cx);
                                s.menu = Some((i + 1).min(s.steps.len() - 1))
                            },
                        ))
                        .child(icon_btn(
                            ("delete", i),
                            tr(cx, "Remove step", "移除步骤"),
                            Glyph::Trash,
                            cx,
                            move |s, _, cx| {
                                s.steps.remove(i);
                                s.menu = None;
                                cx.emit(PlanEvent::Changed);
                                cx.notify()
                            },
                        )),
                )
            }
        }
        panel(&self.id, 522., cx)
            .child(
                row()
                    .child(icon(Glyph::Book, 17., p.secondary))
                    .child(title(format!(
                        "{} · {}",
                        tr(cx, "Plan", "计划"),
                        self.steps.len()
                    ))),
            )
            .child(note(self.goal.clone(), cx))
            .child(bar(
                animated_value(
                    "plan-completion",
                    self.completed as f32 / self.steps.len().max(1) as f32,
                    0.22,
                    w,
                    cx,
                ),
                p.success,
                cx,
            ))
            .child(body)
            .when(self.editing == Some(self.steps.len()), |r| {
                r.child(input(&self.edit, cx))
            })
            .when(self.editing.is_some(), |r| {
                r.child(btn(
                    "save",
                    tr(cx, "Save step", "保存步骤"),
                    true,
                    cx,
                    |s, _, cx| s.commit(cx),
                ))
            })
            .when(!self.approved, |r| {
                r.child(
                    row()
                        .flex_wrap()
                        .child(btn(
                            "add",
                            tr(cx, "Add step", "添加步骤"),
                            false,
                            cx,
                            |s, w, cx| s.rename(s.steps.len(), w, cx),
                        ))
                        .child(btn(
                            "revise",
                            tr(cx, "Keep planning", "继续规划"),
                            false,
                            cx,
                            |_, _, cx| cx.emit(PlanEvent::Revise),
                        ))
                        .child(btn(
                            "approve",
                            tr(cx, "Approve plan", "批准计划"),
                            true,
                            cx,
                            |s, _, cx| {
                                if !s.steps.is_empty() && s.editing.is_none() {
                                    s.approved = true;
                                    cx.emit(PlanEvent::Approve);
                                    cx.notify()
                                }
                            },
                        )),
                )
            })
    }
}
