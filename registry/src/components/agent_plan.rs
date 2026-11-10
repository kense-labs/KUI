//! Review an ordered execution plan and change the sequence of its steps.
use crate::primitives::ops_surface::*;
use crate::primitives::{
    brand_mark::brand_mark,
    editor::{EditEvent, Editor, InputMode},
    family::{Paint, disabled_motion},
    glyph::{Glyph, icon, spinner},
};
use gpui::{prelude::*, *};
pub enum PlanEvent {
    Changed,
    Approve,
    Revise,
    Pause(bool),
    Restart,
}
pub struct TaskPlan {
    id: SharedString,
    pub goal: String,
    pub steps: Vec<String>,
    pub completed: usize,
    pub approved: bool,
    editing: Option<usize>,
    menu: Option<usize>,
    model: String,
    paused: bool,
    failure: Option<SharedString>,
    visibility: fn() -> bool,
    clock: web_time::Instant,
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
            model: "Kense".to_owned(),
            paused: false,
            failure: None,
            visibility: || true,
            clock: web_time::Instant::now(),
            edit,
            _sub: sub,
        }
    }
    pub fn model(mut self, name: impl Into<String>) -> Self {
        self.model = name.into();
        self
    }
    pub fn visibility(mut self, visible: fn() -> bool) -> Self {
        self.visibility = visible;
        self
    }
    pub fn fail_current(&mut self, reason: impl Into<SharedString>, cx: &mut Context<Self>) {
        if self.approved && self.completed < self.steps.len() {
            self.paused = true;
            self.failure = Some(reason.into());
            cx.emit(PlanEvent::Pause(true));
            cx.notify();
        }
    }
    fn move_step(&mut self, from: usize, to: usize, cx: &mut Context<Self>) {
        if !self.approved && from < self.steps.len() && to < self.steps.len() && from != to {
            let step = self.steps.remove(from);
            self.steps.insert(to, step);
            self.menu = None;
            cx.emit(PlanEvent::Changed);
            cx.notify();
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
        self.move_step(i, j, cx);
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
                .child(
                    div()
                        .size(px(13.))
                        .rounded_full()
                        .border_1()
                        .border_color(p.faint),
                ),
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
struct StepDrag {
    owner: EntityId,
    index: usize,
    title: SharedString,
}

struct StepGhost(SharedString);
impl Render for StepGhost {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        div()
            .rounded(crate::foundation::surface_radius(cx))
            .border_1()
            .border_color(p.line)
            .bg(p.panel)
            .text_color(p.ink)
            .shadow_md()
            .px_3()
            .py_2()
            .child(self.0.clone())
    }
}

fn grip(ink: Hsla) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |bounds, _, w, _| {
            let center = bounds.center();
            for y in [-4., 0., 4.] {
                for x in [-2., 2.] {
                    w.paint_quad(
                        fill(
                            Bounds::centered_at(
                                center + point(px(x), px(y)),
                                size(px(1.8), px(1.8)),
                            ),
                            ink,
                        )
                        .corner_radii(px(0.9)),
                    );
                }
            }
        },
    )
    .size(px(15.))
    .flex_none()
}

impl Render for TaskPlan {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let owner = cx.entity_id();
        let total = self.steps.len();
        let done = self.approved && self.completed >= total;
        let active = self.approved && !done;
        let motion = !disabled_motion(cx) && (self.visibility)() && w.is_visible();
        let turn = if active && !self.paused && motion {
            w.request_animation_frame();
            self.clock.elapsed().as_secs_f32() * 1.15
        } else {
            0.
        };
        let brand = brand_mark(&self.model, 16., cx)
            .unwrap_or_else(|| icon(Glyph::Book, 16., p.secondary).into_any_element());
        let heading = if done {
            tr(cx, "Plan complete", "计划已完成").to_owned()
        } else if self.paused {
            format!(
                "{} · {} / {total}",
                tr(cx, "Paused", "已暂停"),
                self.completed
            )
        } else if active {
            format!(
                "{} · {} / {total}",
                tr(cx, "Executing plan", "正在执行计划"),
                self.completed
            )
        } else {
            format!(
                "{} · {total} {}",
                tr(cx, "Plan", "计划"),
                tr(cx, "steps", "个步骤")
            )
        };

        let mut body = column().px_3().py_2().gap_0();
        for (i, name) in self.steps.iter().enumerate() {
            let fraction = animated_value(
                ("check-draw", i),
                f32::from(i < self.completed),
                0.22,
                w,
                cx,
            );
            if self.approved {
                let current = i == self.completed;
                let marker = if current {
                    let glyph = if self.failure.is_some() {
                        Glyph::Alert
                    } else {
                        Glyph::Pause
                    };
                    let ink = if self.failure.is_some() {
                        p.danger
                    } else {
                        p.secondary
                    };
                    div()
                        .relative()
                        .size(px(20.))
                        .flex_none()
                        .child(plan_marker(i, fraction, p).opacity(0.))
                        .child(
                            div()
                                .absolute()
                                .inset_0()
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(if self.paused {
                                    icon(glyph, 14., ink).into_any_element()
                                } else {
                                    spinner(turn, 16., p.bright).into_any_element()
                                }),
                        )
                } else {
                    plan_marker(i, fraction, p)
                };
                let color = if current && !self.paused && motion {
                    p.ink.blend(
                        p.secondary
                            .opacity(0.15 + (turn * std::f32::consts::TAU).sin() * 0.12),
                    )
                } else {
                    p.ink.blend(p.secondary.opacity(fraction))
                };
                body = body.child(
                    row().min_h(px(36.)).gap(px(12.)).child(marker).child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_color(color)
                            .child(name.clone()),
                    ),
                );
                if current && let Some(error) = &self.failure {
                    body = body.child(
                        column()
                            .pl(px(32.))
                            .pb_2()
                            .child(note(error.clone(), cx).text_color(p.danger))
                            .child(
                                row()
                                    .child(btn(
                                        ("retry-step", i),
                                        tr(cx, "Retry", "重试"),
                                        false,
                                        cx,
                                        |s, _, cx| {
                                            s.failure = None;
                                            s.paused = false;
                                            cx.emit(PlanEvent::Pause(false));
                                            cx.notify();
                                        },
                                    ))
                                    .child(btn(
                                        ("skip-step", i),
                                        tr(cx, "Skip", "跳过"),
                                        false,
                                        cx,
                                        |s, _, cx| {
                                            s.completed = (s.completed + 1).min(s.steps.len());
                                            s.failure = None;
                                            s.paused = false;
                                            cx.emit(PlanEvent::Pause(false));
                                            cx.notify();
                                        },
                                    )),
                            ),
                    );
                }
                continue;
            }
            let drag = StepDrag {
                owner,
                index: i,
                title: name.clone().into(),
            };
            let handle = item(
                ("handle", i),
                tr(cx, "Step actions", "步骤操作"),
                cx,
                move |s, _, cx| {
                    s.menu = if s.menu == Some(i) { None } else { Some(i) };
                    cx.notify();
                },
            )
            .px_0()
            .py_0()
            .size(px(24.))
            .flex_none()
            .justify_center()
            .child(grip(p.faint))
            .on_drag(drag, |d, _, _, cx| cx.new(|_| StepGhost(d.title.clone())));
            let editable = if self.editing == Some(i) {
                input(&self.edit, cx).into_any_element()
            } else {
                item(("step", i), name.clone(), cx, move |s, w, cx| {
                    s.rename(i, w, cx)
                })
                .flex_1()
                .w_full()
                .px_1()
                .py_1()
                .min_h(px(30.))
                .child(div().flex_1().min_w_0().child(name.clone()))
                .into_any_element()
            };
            body = body.child(
                row()
                    .id(("row", i))
                    .min_h(px(36.))
                    .gap(px(6.))
                    .rounded(crate::foundation::surface_radius(cx))
                    .drag_over::<StepDrag>(move |v, _, _, _| v.bg(p.hover))
                    .on_drop(cx.listener(move |s, drag: &StepDrag, _, cx| {
                        if drag.owner == owner {
                            s.move_step(drag.index, i, cx);
                        }
                    }))
                    .child(handle)
                    .child(div().flex_1().min_w_0().child(editable))
                    .on_key_down(cx.listener(move |s, e: &KeyDownEvent, _, cx| {
                        if e.keystroke.modifiers.alt
                            && matches!(e.keystroke.key.as_str(), "up" | "down")
                        {
                            s.shift(i, e.keystroke.key == "down", cx);
                            cx.stop_propagation();
                        }
                    })),
            );
            if self.menu == Some(i) {
                body = body.child(
                    row()
                        .flex_wrap()
                        .pl(px(29.))
                        .gap_1()
                        .child(icon_btn(
                            ("up", i),
                            tr(cx, "Move up", "上移"),
                            Glyph::Up,
                            cx,
                            move |s, _, cx| s.shift(i, false, cx),
                        ))
                        .child(icon_btn(
                            ("down", i),
                            tr(cx, "Move down", "下移"),
                            Glyph::Down,
                            cx,
                            move |s, _, cx| s.shift(i, true, cx),
                        ))
                        .child(icon_btn(
                            ("delete", i),
                            tr(cx, "Remove step", "移除步骤"),
                            Glyph::Trash,
                            cx,
                            move |s, _, cx| {
                                if i < s.steps.len() {
                                    s.steps.remove(i);
                                    s.menu = None;
                                    s.editing = None;
                                    cx.emit(PlanEvent::Changed);
                                    cx.notify();
                                }
                            },
                        )),
                );
            }
        }
        if !self.approved {
            if self.editing == Some(total) {
                body = body.child(div().pl(px(30.)).child(input(&self.edit, cx)));
            } else {
                body = body.child(
                    item("add", tr(cx, "Add step", "添加步骤"), cx, |s, w, cx| {
                        s.rename(s.steps.len(), w, cx)
                    })
                    .min_h(px(36.))
                    .px_1()
                    .py_0()
                    .gap(px(11.))
                    .child(icon(Glyph::Plus, 16., p.secondary))
                    .child(
                        div()
                            .text_color(p.secondary)
                            .child(tr(cx, "Add step", "添加步骤")),
                    ),
                );
            }
            if self.editing.is_some() {
                body = body.child(row().justify_end().child(btn(
                    "save",
                    tr(cx, "Save step", "保存步骤"),
                    true,
                    cx,
                    |s, _, cx| s.commit(cx),
                )));
            }
        }
        let mut footer = row()
            .justify_end()
            .flex_wrap()
            .gap_2()
            .px_3()
            .py_2()
            .border_t_1()
            .border_color(p.line);
        if done {
            footer = footer
                .child(
                    note(
                        format!("{} {total}", tr(cx, "Steps completed:", "已完成步骤：")),
                        cx,
                    )
                    .flex_1(),
                )
                .child(
                    btn(
                        "start-over",
                        tr(cx, "Start over", "重新开始"),
                        false,
                        cx,
                        |s, _, cx| {
                            s.approved = false;
                            s.completed = 0;
                            s.paused = false;
                            s.failure = None;
                            cx.emit(PlanEvent::Restart);
                            cx.notify();
                        },
                    )
                    .border_1()
                    .border_color(p.line),
                );
        } else if active {
            let pause = self.paused;
            footer = footer
                .child(note(format!("{} / {total}", self.completed), cx).flex_1())
                .when(self.failure.is_none(), |v| {
                    v.child(
                        btn(
                            "pause-plan",
                            if pause {
                                tr(cx, "Resume", "继续执行")
                            } else {
                                tr(cx, "Pause", "暂停")
                            },
                            false,
                            cx,
                            |s, _, cx| {
                                s.paused = !s.paused;
                                cx.emit(PlanEvent::Pause(s.paused));
                                cx.notify();
                            },
                        )
                        .border_1()
                        .border_color(p.line),
                    )
                });
        } else {
            footer = footer
                .child(
                    btn(
                        "revise",
                        tr(cx, "Keep planning", "继续规划"),
                        false,
                        cx,
                        |_, _, cx| cx.emit(PlanEvent::Revise),
                    )
                    .border_1()
                    .border_color(p.line),
                )
                .child(btn(
                    "approve",
                    tr(cx, "Approve plan", "批准计划"),
                    true,
                    cx,
                    |s, _, cx| {
                        if !s.steps.is_empty() && s.editing.is_none() {
                            s.approved = true;
                            s.menu = None;
                            cx.emit(PlanEvent::Approve);
                            cx.notify();
                        }
                    },
                ));
        }
        panel(&self.id, 560., cx)
            .p_0()
            .gap_0()
            .role(Role::Group)
            .aria_label(format!("{} · {}", self.model, self.goal))
            .child(
                column()
                    .px_4()
                    .py_3()
                    .gap_1()
                    .child(
                        row()
                            .gap(px(10.))
                            .child(brand)
                            .child(
                                swap_label("plan-heading", heading, w, cx)
                                    .flex_1()
                                    .text_size(px(13.)),
                            )
                            .child(note(self.model.clone(), cx).truncate().max_w(px(128.))),
                    )
                    .child(note(self.goal.clone(), cx).pl(px(26.)))
                    .when(self.approved, |v| {
                        v.child(bar(
                            animated_value(
                                "plan-completion",
                                self.completed as f32 / total.max(1) as f32,
                                0.26,
                                w,
                                cx,
                            ),
                            p.success,
                            cx,
                        ))
                    }),
            )
            .child(divider(cx))
            .child(body)
            .child(footer)
    }
}
