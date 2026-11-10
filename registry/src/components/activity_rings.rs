//! Host-controlled daily goals, animated as concentric tick rings.
use crate::components::animated_counter::AnimatedCounter;
use crate::primitives::{
    family::{Paint, Slide, Tone, face},
    presentation::{column, hint, row, title, tr},
};
use gpui::{prelude::*, *};
use std::{
    cell::Cell,
    collections::HashMap,
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
}
pub struct ActivityRings {
    title: SharedString,
    goals: Vec<ActivityGoal>,
    days: Vec<ActivityDay>,
    day: usize,
    day_focus: HashMap<SharedString, FocusHandle>,
    rings: Vec<Ring>,
    center: Entity<AnimatedCounter>,
    active: Option<usize>,
    hovered: Option<usize>,
    bounds: Rc<Cell<Bounds<Pixels>>>,
    width: Rc<Cell<Pixels>>,
    surface: bool,
}
impl EventEmitter<ActivityDayChanged> for ActivityRings {}
impl EventEmitter<ActivityGoalClosed> for ActivityRings {}
impl ActivityRings {
    pub fn new(
        title: impl Into<SharedString>,
        goals: Vec<ActivityGoal>,
        days: Vec<ActivityDay>,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut view = Self {
            title: title.into(),
            day: days.len().saturating_sub(1),
            day_focus: days
                .iter()
                .map(|day| (day.id.clone(), cx.focus_handle()))
                .collect(),
            center: cx.new(|_| {
                AnimatedCounter::new(0.)
                    .with_size(36.)
                    .with_wheel_style(1., 0.16, FontWeight::NORMAL)
                    .with_animate_on_view(true)
            }),
            goals,
            days,
            rings: vec![],
            active: None,
            hovered: None,
            bounds: Default::default(),
            width: Default::default(),
            surface: true,
        };
        view.retarget(cx);
        view
    }
    pub fn with_day(mut self, index: usize) -> Self {
        self.day = index.min(self.days.len().saturating_sub(1));
        self
    }
    /// Use the dial and week picker within an existing surface.
    pub fn with_surface(mut self, surface: bool) -> Self {
        self.surface = surface;
        self
    }
    /// Index of the day displayed in the dial.
    pub fn selected_day(&self) -> usize {
        self.day
    }
    pub fn set_day(&mut self, index: usize, cx: &mut Context<Self>) {
        if index >= self.days.len() || index == self.day {
            return;
        }
        let next = &self.days[index];
        self.day = index;
        cx.emit(ActivityDayChanged {
            day: next.id.clone(),
            index,
        });
        self.retarget(cx);
        cx.notify();
    }
    /// Replace values and keep the selected date by ID when it is still present.
    pub fn set_days(&mut self, days: Vec<ActivityDay>, cx: &mut Context<Self>) {
        let old = self.days.get(self.day);
        let selected = old.and_then(|old| days.iter().position(|day| day.id == old.id));
        if let (Some(old), Some(index)) = (old, selected) {
            let next = &days[index];
            for goal in &self.goals {
                if goal.goal.is_finite()
                    && goal.goal > 0.
                    && old.value(&goal.id) < goal.goal
                    && next.value(&goal.id) >= goal.goal
                {
                    cx.emit(ActivityGoalClosed {
                        goal: goal.id.clone(),
                        day: next.id.clone(),
                    });
                }
            }
        }
        let mut old_focus = std::mem::take(&mut self.day_focus);
        self.day_focus = days
            .iter()
            .map(|day| {
                let handle = old_focus
                    .remove(&day.id)
                    .unwrap_or_else(|| cx.focus_handle());
                (day.id.clone(), handle)
            })
            .collect();
        self.days = days;
        self.day = selected.unwrap_or_else(|| self.days.len().saturating_sub(1));
        self.retarget(cx);
        cx.notify();
    }
    fn value(&self, index: usize) -> f32 {
        self.goals
            .get(index)
            .and_then(|goal| self.days.get(self.day).map(|d| d.value(&goal.id)))
            .unwrap_or(0.)
    }
    fn progress(&self, index: usize) -> f32 {
        let goal = self.goals[index].goal;
        let value = self.value(index);
        if goal.is_finite() && goal > 0. {
            (value / goal).clamp(0., 2.)
        } else {
            0.
        }
    }
    /// Mean completion across goals, as shown in the dial center at rest.
    fn center_percent(&self) -> f64 {
        if self.goals.is_empty() {
            return 0.;
        }
        let sum: f32 = (0..self.goals.len())
            .map(|i| self.progress(i).clamp(0., 1.))
            .sum();
        f64::from(sum / self.goals.len() as f32 * 100.)
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
        let targets: Vec<f32> = (0..self.goals.len()).map(|i| self.progress(i)).collect();
        for (ring, target) in self.rings.iter_mut().zip(targets) {
            ring.progress.duration(target, 0.4, cx);
        }
        self.center
            .update(cx, |c, cx| c.set_value(self.center_percent(), cx));
    }
}
impl Render for ActivityRings {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let compact = self.width.get() < px(480.);
        let (dial_size, center_size, legend_height, legend_pad, swatch_height) = if compact {
            (184., 30., 52., 8., 30.)
        } else {
            (216., 36., 56., 10., 32.)
        };
        let center_percent = self.center_percent();
        self.center.update(cx, |center, cx| {
            center.set_wheel_size(center_size);
            center.set_value(center_percent, cx);
        });
        while self.rings.len() < self.goals.len() {
            let mut initial = Slide::new(0.);
            initial.duration(self.progress(self.rings.len()), 0.4, cx);
            self.rings.push(Ring { progress: initial });
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
            .size(px(dial_size))
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
                            // Ticks start at 3 o'clock, matching the reference dial phase.
                            let tick_at = |radius: f32, turn: f32| {
                                let angle = TAU * turn;
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
                                // The reference mask reveals each tick fully or not at all.
                                if *progress * ticks as f32 > tick as f32 {
                                    let mut path = PathBuilder::stroke(px(2.6 * scale));
                                    path.move_to(tick_at(inner, turn));
                                    path.line_to(tick_at(outer, turn));
                                    if let Ok(path) = path.build() {
                                        w.paint_path(path, color.opacity(alpha));
                                    }
                                }
                            }
                            let extra = (*progress - 1.).clamp(0., 1.);
                            if extra > 0. {
                                // Overflow lap hugs the inner edge of its ring.
                                let lap = (outer_u - 17.) * scale;
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
                            // Radial pointer on the ring's midline at the current progress.
                            let angle = -PI / 2. + TAU * *progress;
                            let mid = mid_u * scale;
                            let half = 5.5 * scale;
                            let head = point(
                                center.x + px(angle.cos() * (mid - half)),
                                center.y + px(angle.sin() * (mid - half)),
                            );
                            let tail = point(
                                center.x + px(angle.cos() * (mid + half)),
                                center.y + px(angle.sin() * (mid + half)),
                            );
                            let mut halo = PathBuilder::stroke(px(10. * scale));
                            halo.move_to(head);
                            halo.line_to(tail);
                            if let Ok(path) = halo.build() {
                                w.paint_path(path, p.panel);
                            }
                            // The reference pointers stay at full opacity while rings dim.
                            let mut body = PathBuilder::stroke(px(6. * scale));
                            body.move_to(head);
                            body.line_to(tail);
                            if let Ok(path) = body.build() {
                                w.paint_path(path, p.ink);
                            }
                        }
                    },
                )
                .size_full(),
            )
            .child(
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
                            .text_size(px(center_size))
                            .line_height(px(center_size))
                            .text_color(p.ink)
                            .when_some(highlighted, |line, i| {
                                let goal = &self.goals[i];
                                line.child(format!("{:.0}", self.value(i))).child(
                                    div()
                                        .ml(px(1.))
                                        .text_size(px(16.))
                                        .text_color(p.secondary)
                                        .child(goal.unit.clone()),
                                )
                            })
                            .when(highlighted.is_none(), |line| {
                                line.child(self.center.clone()).child(
                                    div()
                                        .ml(px(1.))
                                        .text_size(px(16.))
                                        .text_color(p.secondary)
                                        .child("%"),
                                )
                            }),
                    )
                    .child(
                        div()
                            .max_w(px(112.))
                            .text_size(px(12.))
                            .line_height(px(15.))
                            .text_color(p.secondary)
                            .child(highlighted.map_or_else(
                                || SharedString::from(tr(cx, "of goals", "目标")),
                                |i| self.goals[i].label.clone(),
                            )),
                    ),
            );
        let legend =
            column()
                .min_w(px(0.))
                .flex_1()
                .when(compact, |legend| legend.w_full())
                .gap(px(2.))
                .children(self.goals.iter().enumerate().map(|(i, goal)| {
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
                        legend_height,
                        crate::foundation::surface_radius(cx).into(),
                        Tone::Clear,
                        cx,
                    )
                    .w_full()
                    .px(px(12.))
                    .py(px(legend_pad))
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
                            .child(
                                div()
                                    .w(px(3.))
                                    .h(px(swatch_height))
                                    .rounded(px(2.))
                                    .bg(goal.color),
                            )
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
                                    .child(div().text_size(px(12.)).text_color(p.secondary).child(
                                        format!("{value:.0} / {:.0} {}", goal.goal, goal.unit),
                                    )),
                            )
                            .child(
                                div()
                                    .text_size(px(14.))
                                    .text_color(p.secondary)
                                    .child(format!("{pct:.0}%")),
                            ),
                    )
                }));
        let day = self.day;
        let week = div()
            .id("activity-week")
            .role(Role::RadioGroup)
            .flex()
            .gap(px(2.))
            .flex_wrap()
            .aria_label(tr(cx, "Choose a day", "选择日期"))
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
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
                s.day_focus[&s.days[next].id].focus(w, cx);
                w.prevent_default();
                cx.stop_propagation();
            }))
            .children(self.days.iter().enumerate().map(|(i, d)| {
                // Three mini rings per day button, keyed to the goal colors.
                let arcs: Vec<(Hsla, f32)> = self
                    .goals
                    .iter()
                    .map(|goal| {
                        let value = d.value(&goal.id);
                        let share = if goal.goal.is_finite() && goal.goal > 0. {
                            (value / goal.goal).clamp(0., 1.)
                        } else {
                            0.
                        };
                        (goal.color, share)
                    })
                    .collect();
                let mini = canvas(
                    move |_, _, _| {},
                    move |b, _, w, _| {
                        let size = f32::from(b.size.width);
                        let center = point(b.origin.x + px(size / 2.), b.origin.y + px(size / 2.));
                        let scale = size / 32.;
                        for (ring, (color, share)) in arcs.iter().enumerate() {
                            let radius = (14. - 4.5 * ring as f32) * scale;
                            let point_at = |turn: f32| {
                                let angle = TAU * turn;
                                point(
                                    center.x + px(angle.cos() * radius),
                                    center.y + px(angle.sin() * radius),
                                )
                            };
                            let mut path = PathBuilder::stroke(px(3. * scale));
                            for step in 0..=48 {
                                let point = point_at(step as f32 / 48.);
                                if step == 0 {
                                    path.move_to(point);
                                } else {
                                    path.line_to(point);
                                }
                            }
                            if let Ok(path) = path.build() {
                                w.paint_path(path, p.ink.opacity(0.09));
                            }
                            if *share > 0.001 {
                                let steps = (48. * share).round().max(1.) as usize;
                                let mut path = PathBuilder::stroke(px(3. * scale));
                                for step in 0..=steps {
                                    let point = point_at(share * step as f32 / steps as f32);
                                    if step == 0 {
                                        path.move_to(point);
                                    } else {
                                        path.line_to(point);
                                    }
                                }
                                if let Ok(path) = path.build() {
                                    w.paint_path(path, *color);
                                }
                            }
                        }
                    },
                )
                .size(px(30.))
                .flex_none();
                face(
                    ("activity-day", i),
                    d.label.clone(),
                    65.,
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
                .track_focus(&self.day_focus[&d.id].clone().tab_stop(day == i))
                .aria_toggled((day == i).into())
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap(px(4.))
                        .child(mini)
                        .child(
                            div()
                                .text_size(px(12.))
                                .line_height(px(16.8))
                                .child(d.label.clone()),
                        ),
                )
                .on_click(cx.listener(move |s, _, w, cx| {
                    if let Some(day) = s.days.get(i) {
                        s.day_focus[&day.id].focus(w, cx);
                        s.set_day(i, cx);
                    }
                }))
            }));
        let width = self.width.clone();
        column()
            .id("activity-rings")
            .role(Role::Group)
            .relative()
            .w_full()
            .max_w(px(608.))
            .gap(px(20.))
            .when(self.surface, |view| {
                view.p(px(16.))
                    .rounded(crate::foundation::surface_radius(cx))
                    .bg(p.panel)
                    .border_1()
                    .border_color(p.line)
            })
            .aria_label(self.title.clone())
            .text_color(p.ink)
            .child(
                canvas(
                    move |bounds, w, _| {
                        if width.replace(bounds.size.width) != bounds.size.width {
                            w.request_animation_frame();
                        }
                    },
                    |_, _, _, _| (),
                )
                .absolute()
                .inset_0(),
            )
            .when(self.surface, |view| {
                view.child(title(self.title.clone(), p))
            })
            .child(
                div()
                    .flex()
                    .when(compact, |body| body.flex_col())
                    .justify_center()
                    .items_center()
                    .gap(px(28.))
                    .child(visual)
                    .child(legend),
            )
            .child(week)
            .when(self.days.is_empty(), |view| {
                view.child(hint(tr(cx, "No activity yet", "暂无活动数据"), p))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::{ActivityDay, ActivityGoal, ActivityGoalClosed, ActivityRings};
    use gpui::{AppContext, TestAppContext, px, rgb, size};
    use std::{cell::RefCell, rc::Rc};
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

    #[test]
    fn closes_goal_only_when_the_selected_day_crosses_it() {
        let app = TestAppContext::single();
        app.update(crate::init);
        let day = |id, value| ActivityDay::new(id, id, vec![("move".into(), value)]);
        let view = app.update(|cx| {
            cx.new(|cx| {
                ActivityRings::new(
                    "Activity",
                    vec![ActivityGoal::new(
                        "move",
                        "Move",
                        100.,
                        "min",
                        rgb(0x6572e4).into(),
                    )],
                    vec![day("mon", 100.), day("tue", 20.)],
                    cx,
                )
            })
        });
        let closes = Rc::new(RefCell::new(Vec::new()));
        let events = closes.clone();
        let _subscription = app.update(|cx| {
            cx.subscribe(&view, move |_, event: &ActivityGoalClosed, _| {
                events
                    .borrow_mut()
                    .push((event.day.clone(), event.goal.clone()));
            })
        });
        app.update(|cx| {
            view.update(cx, |view, cx| {
                view.set_day(0, cx);
                view.set_day(1, cx);
            })
        });
        assert!(closes.borrow().is_empty());
        app.update(|cx| {
            view.update(cx, |view, cx| {
                view.set_days(vec![day("tue", 100.), day("mon", 110.)], cx);
                assert_eq!(view.day, 0);
                view.set_days(vec![day("tue", 150.), day("mon", 110.)], cx);
                view.set_days(vec![day("wed", 150.)], cx);
            })
        });
        assert_eq!(&*closes.borrow(), &[("tue".into(), "move".into())]);
    }

    #[test]
    fn resizes_the_dial_and_moves_focus_to_the_picked_day() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) = app.add_window_view(|_, cx| {
            ActivityRings::new(
                "Activity",
                vec![ActivityGoal::new(
                    "move",
                    "Move",
                    100.,
                    "min",
                    rgb(0x6572e4).into(),
                )],
                vec![
                    ActivityDay::new("mon", "Mon", vec![]),
                    ActivityDay::new("tue", "Tue", vec![]),
                ],
                cx,
            )
        });
        for (width, expected) in [(608., 216.), (320., 184.), (608., 216.)] {
            w.simulate_resize(size(px(width), px(650.)));
            for _ in 0..3 {
                w.update(|w, cx| {
                    w.simulate_next_frame(cx);
                    w.refresh();
                });
                w.run_until_parked();
            }
            w.update(|_, cx| assert_eq!(view.read(cx).bounds.get().size.width, px(expected)));
        }
        w.update(|w, cx| w.focus_next(cx));
        w.run_until_parked();
        w.update(|w, cx| w.focus_next(cx));
        w.run_until_parked();
        w.update(|w, cx| assert!(view.read(cx).day_focus["tue"].is_focused(w)));
        w.simulate_keystrokes("left");
        w.update(|w, cx| {
            let state = view.read(cx);
            assert_eq!(state.day, 0);
            assert!(state.day_focus["mon"].is_focused(w));
        });
    }
}
