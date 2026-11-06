//! Host-controlled daily goals, animated as concentric tick rings.
use crate::components::animated_counter::AnimatedCounter;
use crate::primitives::{
    family::{Paint, Slide, Tone, face},
    presentation::{column, hint, row, title, tr},
};
use gpui::{prelude::*, *};
use std::{
    cell::Cell,
    f32::consts::{PI, TAU},
    rc::Rc,
};

#[derive(Clone, Debug)]
pub struct ActivityGoal {
    pub id: SharedString,
    pub label: SharedString,
    pub unit: SharedString,
    pub goal: f32,
    pub color: Hsla,
}
impl ActivityGoal {
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        goal: f32,
        unit: impl Into<SharedString>,
        color: Hsla,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            goal,
            unit: unit.into(),
            color,
        }
    }
}
#[derive(Clone, Debug)]
pub struct ActivityDay {
    pub id: SharedString,
    pub label: SharedString,
    pub values: Vec<(SharedString, f32)>,
}
impl ActivityDay {
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        values: Vec<(SharedString, f32)>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            values,
        }
    }
    fn value(&self, id: &str) -> f32 {
        self.values
            .iter()
            .find(|(key, _)| key.as_ref() == id)
            .map_or(0., |(_, v)| if v.is_finite() { v.max(0.) } else { 0. })
    }
}
#[derive(Clone, Debug)]
pub struct ActivityDayChanged {
    pub day: SharedString,
    pub index: usize,
}
#[derive(Clone, Debug)]
pub struct ActivityGoalClosed {
    pub goal: SharedString,
    pub day: SharedString,
}
struct Ring {
    progress: Slide,
    counter: Entity<AnimatedCounter>,
}
pub struct ActivityRings {
    title: SharedString,
    goals: Vec<ActivityGoal>,
    days: Vec<ActivityDay>,
    day: usize,
    rings: Vec<Ring>,
    active: Option<usize>,
    hovered: Option<usize>,
    bounds: Rc<Cell<Bounds<Pixels>>>,
}
impl EventEmitter<ActivityDayChanged> for ActivityRings {}
impl EventEmitter<ActivityGoalClosed> for ActivityRings {}
impl ActivityRings {
    pub fn new(
        title: impl Into<SharedString>,
        goals: Vec<ActivityGoal>,
        days: Vec<ActivityDay>,
    ) -> Self {
        Self {
            title: title.into(),
            goals,
            days,
            day: 0,
            rings: vec![],
            active: None,
            hovered: None,
            bounds: Default::default(),
        }
    }
    pub fn with_day(mut self, index: usize) -> Self {
        self.day = index.min(self.days.len().saturating_sub(1));
        self
    }
    pub fn set_day(&mut self, index: usize, cx: &mut Context<Self>) {
        if index >= self.days.len() || index == self.day {
            return;
        }
        let old = self.days.get(self.day);
        let next = &self.days[index];
        for goal in &self.goals {
            let before = old.map_or(0., |d| d.value(&goal.id));
            let after = next.value(&goal.id);
            if goal.goal.is_finite() && goal.goal > 0. && before < goal.goal && after >= goal.goal {
                cx.emit(ActivityGoalClosed {
                    goal: goal.id.clone(),
                    day: next.id.clone(),
                });
            }
        }
        self.day = index;
        cx.emit(ActivityDayChanged {
            day: next.id.clone(),
            index,
        });
        self.retarget(cx);
        cx.notify();
    }
    pub fn set_days(&mut self, days: Vec<ActivityDay>, cx: &mut Context<Self>) {
        let selected = self.days.get(self.day).map(|d| d.id.clone());
        self.days = days;
        self.day = selected
            .and_then(|id| self.days.iter().position(|d| d.id == id))
            .unwrap_or(0)
            .min(self.days.len().saturating_sub(1));
        self.retarget(cx);
        cx.notify();
    }
    fn value(&self, index: usize) -> f32 {
        self.goals
            .get(index)
            .and_then(|goal| self.days.get(self.day).map(|d| d.value(&goal.id)))
            .unwrap_or(0.)
    }
    fn ring_at(&self, position: Point<Pixels>) -> Option<usize> {
        let bounds = self.bounds.get();
        let width = f32::from(bounds.size.width);
        let height = f32::from(bounds.size.height);
        let x = f32::from(position.x - bounds.origin.x) - width / 2.;
        let y = f32::from(position.y - bounds.origin.y) - height / 2.;
        let distance = (x * x + y * y).sqrt();
        (0..self.goals.len()).find(|i| {
            let outer = 106. - 20. * *i as f32;
            let mid = (outer - 6.5) * width.min(height) / 220.;
            mid >= 8. && (distance - mid).abs() <= 8.
        })
    }
    fn retarget(&mut self, cx: &mut Context<Self>) {
        let values: Vec<f32> = (0..self.goals.len()).map(|i| self.value(i)).collect();
        for (i, ring) in self.rings.iter_mut().enumerate() {
            let goal = self.goals[i].goal;
            let value = values[i];
            let progress = if goal.is_finite() && goal > 0. {
                value / goal
            } else {
                0.
            };
            ring.progress.duration(progress.clamp(0., 2.), 0.9, cx);
            ring.counter
                .update(cx, |c, cx| c.set_value(value as f64, cx));
        }
    }
}
impl Render for ActivityRings {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        while self.rings.len() < self.goals.len() {
            let i = self.rings.len();
            let value = self.value(i);
            let goal = self.goals[i].goal;
            let progress = if goal.is_finite() && goal > 0. {
                value / goal
            } else {
                0.
            };
            let mut initial = Slide::new(0.);
            initial.duration(progress.clamp(0., 2.), 0.9, cx);
            self.rings.push(Ring {
                progress: initial,
                counter: cx.new(|_| AnimatedCounter::new(value as f64).with_size(20.)),
            });
        }
        let items: Vec<(Hsla, f32)> = self
            .goals
            .iter()
            .zip(self.rings.iter_mut())
            .map(|(goal, ring)| (goal.color, ring.progress.get(w, cx).clamp(0., 2.)))
            .collect();
        let highlighted = self.hovered.or(self.active);
        let description = self
            .goals
            .iter()
            .enumerate()
            .map(|(i, g)| {
                let value = self.value(i);
                let pct = if g.goal.is_finite() && g.goal > 0. {
                    value / g.goal * 100.
                } else {
                    0.
                };
                format!(
                    "{} {:.0} {} / {:.0} ({pct:.0}%)",
                    g.label, value, g.unit, g.goal
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        let visual_bounds = self.bounds.clone();
        let visual = div()
            .id("activity-rings-visual")
            .role(Role::Image)
            .aria_label(description)
            .w(px(184.))
            .h(px(184.))
            .flex_none()
            .on_mouse_move(cx.listener(|s, e: &MouseMoveEvent, _, cx| {
                let next = s.ring_at(e.position);
                if next != s.hovered {
                    s.hovered = next;
                    cx.notify();
                }
            }))
            .on_hover(cx.listener(|s, over: &bool, _, cx| {
                if !over && s.hovered.take().is_some() {
                    cx.notify();
                }
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|s, e: &MouseDownEvent, _, cx| {
                    if let Some(i) = s.ring_at(e.position) {
                        s.active = if s.active == Some(i) { None } else { Some(i) };
                        cx.notify();
                    }
                }),
            )
            .child(
                canvas(
                    move |b, _, _| visual_bounds.set(b),
                    move |b, _, w, _| {
                        let width = f32::from(b.size.width);
                        let height = f32::from(b.size.height);
                        let center =
                            point(b.origin.x + px(width / 2.), b.origin.y + px(height / 2.));
                        let scale = width.min(height) / 220.;
                        for (index, (color, progress)) in items.iter().enumerate() {
                            let outer_u = 106. - 20. * index as f32;
                            if outer_u <= 20. {
                                break;
                            }
                            let mid_u = outer_u - 6.5;
                            let ticks = (4. * ((TAU * mid_u / 11.) / 4.).round()).max(4.) as usize;
                            let outer = (outer_u - 1.) * scale;
                            let inner = (outer_u - 12.) * scale;
                            let alpha = if highlighted.is_none_or(|i| i == index) {
                                1.
                            } else {
                                0.22
                            };
                            let tick_at = |radius: f32, turn: f32| {
                                let angle = -PI / 2. + TAU * turn;
                                point(
                                    center.x + px(angle.cos() * radius),
                                    center.y + px(angle.sin() * radius),
                                )
                            };
                            for tick in 0..ticks {
                                let turn = tick as f32 / ticks as f32;
                                let mut path = PathBuilder::stroke(px(2.2 * scale));
                                path.move_to(tick_at(inner, turn));
                                path.line_to(tick_at(outer, turn));
                                if let Ok(path) = path.build() {
                                    w.paint_path(path, p.ink.opacity(0.11 * alpha));
                                }
                                let fill = (*progress * ticks as f32 - tick as f32).clamp(0., 1.);
                                if fill > 0. {
                                    let mut path = PathBuilder::stroke(px(2.6 * scale));
                                    path.move_to(tick_at(inner, turn));
                                    path.line_to(tick_at(outer, turn));
                                    if let Ok(path) = path.build() {
                                        w.paint_path(path, color.opacity(fill * alpha));
                                    }
                                }
                            }
                            let extra = (*progress - 1.).clamp(0., 1.);
                            if extra > 0. {
                                let lap = (outer_u - 6.5) * scale;
                                let mut path = PathBuilder::stroke(px(1.5 * scale));
                                let steps = (ticks as f32 * extra).ceil() as usize;
                                for step in 0..=steps {
                                    let turn = extra * step as f32 / steps.max(1) as f32;
                                    let point = tick_at(lap, turn);
                                    if step == 0 {
                                        path.move_to(point);
                                    } else {
                                        path.line_to(point);
                                    }
                                }
                                if let Ok(path) = path.build() {
                                    w.paint_path(path, color.opacity(alpha));
                                }
                            }
                        }
                    },
                )
                .size_full(),
            )
            .when_some(highlighted, |view, i| {
                let goal = &self.goals[i];
                let value = self.value(i);
                view.child(
                    div()
                        .absolute()
                        .inset_0()
                        .flex()
                        .flex_col()
                        .items_center()
                        .justify_center()
                        .gap(px(2.))
                        .child(
                            div()
                                .flex()
                                .items_baseline()
                                .text_size(px(30.))
                                .line_height(px(30.))
                                .text_color(p.ink)
                                .child(format!("{value:.0}"))
                                .child(
                                    div()
                                        .ml(px(1.))
                                        .text_size(px(16.))
                                        .text_color(p.secondary)
                                        .child(goal.unit.clone()),
                                ),
                        )
                        .child(
                            div()
                                .max_w(px(112.))
                                .text_size(px(12.))
                                .line_height(px(15.))
                                .text_color(p.secondary)
                                .child(goal.label.clone()),
                        ),
                )
            });
        let legend = column().min_w(px(200.)).flex_1().gap(px(2.)).children(
            self.goals.iter().enumerate().map(|(i, goal)| {
                let value = self.value(i);
                let pct = if goal.goal.is_finite() && goal.goal > 0. {
                    value / goal.goal * 100.
                } else {
                    0.
                };
                face(
                    ("activity-goal", i),
                    format!(
                        "{} {:.0} / {:.0} {} ({pct:.0}%)",
                        goal.label, value, goal.goal, goal.unit
                    ),
                    52.,
                    crate::foundation::surface_radius(cx).into(),
                    Tone::Clear,
                    cx,
                )
                .w_full()
                .px(px(12.))
                .py(px(10.))
                .aria_toggled((self.active == Some(i)).into())
                .hover(|s| s.bg(p.tray))
                .when(highlighted == Some(i), |v| v.bg(p.tray))
                .when(highlighted.is_some() && highlighted != Some(i), |v| {
                    v.opacity(0.5)
                })
                .on_hover(cx.listener(move |s, over: &bool, _, cx| {
                    s.hovered = if *over { Some(i) } else { None };
                    cx.notify();
                }))
                .on_click(cx.listener(move |s, _, _, cx| {
                    s.active = if s.active == Some(i) { None } else { Some(i) };
                    cx.notify();
                }))
                .child(
                    row()
                        .items_center()
                        .gap(px(12.))
                        .w_full()
                        .child(div().w(px(3.)).h(px(32.)).rounded(px(2.)).bg(goal.color))
                        .child(
                            column()
                                .flex_1()
                                .min_w(px(0.))
                                .child(
                                    div()
                                        .text_size(px(14.))
                                        .line_height(px(20.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .child(goal.label.clone()),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .text_color(p.secondary)
                                        .child(format!("{value:.0} {}", goal.unit)),
                                ),
                        )
                        .child(
                            row()
                                .items_center()
                                .gap(px(6.))
                                .text_size(px(14.))
                                .text_color(p.secondary)
                                .when(pct >= 100., |row| {
                                    row.child(
                                        div()
                                            .size(px(16.))
                                            .rounded_full()
                                            .bg(p.success.opacity(0.14)),
                                    )
                                })
                                .child(format!("{pct:.0}%")),
                        ),
                )
            }),
        );
        let day = self.day;
        let week = div()
            .id("activity-week")
            .role(Role::RadioGroup)
            .tab_index(0)
            .flex()
            .gap(px(2.))
            .flex_wrap()
            .aria_label(tr(cx, "Choose a day", "选择日期"))
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, _, cx| {
                let n = s.days.len();
                if n == 0 {
                    return;
                }
                let next = match e.keystroke.key.as_str() {
                    "left" | "up" => (s.day + n - 1) % n,
                    "right" | "down" => (s.day + 1) % n,
                    "home" => 0,
                    "end" => n - 1,
                    _ => return,
                };
                s.set_day(next, cx);
                cx.stop_propagation();
            }))
            .children(self.days.iter().enumerate().map(|(i, d)| {
                face(
                    ("activity-day", i),
                    d.label.clone(),
                    36.,
                    crate::foundation::surface_radius(cx).into(),
                    Tone::Clear,
                    cx,
                )
                .flex_1()
                .pt(px(8.))
                .pb(px(6.))
                .text_color(if day == i { p.ink } else { p.secondary })
                .hover(|s| s.text_color(p.ink))
                .when(day == i, |v| v.bg(p.tray))
                .role(Role::RadioButton)
                .tab_index(-1)
                .when(day == i, |v| v.aria_active_descendant())
                .aria_toggled((day == i).into())
                .child(d.label.clone())
                .on_click(cx.listener(move |s, _, _, cx| s.set_day(i, cx)))
            }));
        column()
            .w_full()
            .max_w(px(608.))
            .gap(px(20.))
            .p(px(16.))
            .rounded(crate::foundation::surface_radius(cx))
            .bg(p.panel)
            .border_1()
            .border_color(p.line)
            .text_color(p.ink)
            .child(title(self.title.clone(), p))
            .child(week)
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .justify_center()
                    .items_center()
                    .gap(px(28.))
                    .child(visual)
                    .child(legend),
            )
            .when(self.days.is_empty(), |view| {
                view.child(hint(tr(cx, "No activity yet", "暂无活动数据"), p))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::ActivityDay;
    #[test]
    fn ignores_invalid_and_missing_day_values() {
        let day = ActivityDay::new(
            "day",
            "Day",
            vec![
                ("a".into(), f32::NAN),
                ("b".into(), -10.),
                ("c".into(), 42.),
            ],
        );
        assert_eq!(day.value("a"), 0.);
        assert_eq!(day.value("b"), 0.);
        assert_eq!(day.value("c"), 42.);
        assert_eq!(day.value("missing"), 0.);
    }
}
