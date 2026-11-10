//! Overlapping profile portraits with an expandable roster.
use crate::{
    components::avatar::{Avatar, Person},
    primitives::{
        family::{Paint, Slide},
        nav::Fold,
        presentation::*,
    },
};
use gpui::{prelude::*, *};
struct Slot {
    person: Person,
    space: Slide,
    x: Slide,
    y: Slide,
    tip: Slide,
    leaving: bool,
}
impl Slot {
    fn new(person: Person, visible: bool) -> Self {
        Self {
            person,
            space: Slide::new(visible as u8 as f32),
            x: Slide::new(0.),
            y: Slide::new(0.),
            tip: Slide::new(0.),
            leaving: false,
        }
    }
}
pub struct AvatarGroup {
    people: Vec<Person>,
    slots: Vec<Slot>,
    limit: usize,
    over: bool,
    active: Option<SharedString>,
    count: ChangingText,
    overflow: Slide,
    overflow_x: Slide,
    roster: Fold,
}
impl AvatarGroup {
    pub fn new(people: Vec<Person>, limit: usize) -> Self {
        let extra = people.len().saturating_sub(limit);
        let slots = people
            .iter()
            .take(limit)
            .cloned()
            .map(|p| Slot::new(p, true))
            .collect();
        Self {
            people,
            slots,
            limit,
            over: false,
            active: None,
            count: ChangingText::new(format!("+{extra}")),
            overflow: Slide::new((extra > 0) as u8 as f32),
            overflow_x: Slide::new(0.),
            roster: Fold::new(false, 120.),
        }
    }
    fn fan(&mut self, cx: &App) {
        let n = self.people.len().min(self.limit) + (self.people.len() > self.limit) as usize;
        let middle = n.saturating_sub(1) as f32 / 2.;
        let focused = self
            .slots
            .iter()
            .position(|s| Some(&s.person.name) == self.active.as_ref());
        for (i, s) in self.slots.iter_mut().enumerate() {
            let near = match focused {
                Some(k) if k == i + 1 => -3.,
                Some(k) if k + 1 == i => 3.,
                _ => 0.,
            };
            s.x.duration(
                if self.over {
                    (i as f32 - middle) * 4. + near
                } else {
                    0.
                },
                0.26,
                cx,
            );
            let selected = self.over && focused == Some(i);
            s.y.duration(if selected { -2. } else { 0. }, 0.18, cx);
            s.tip.delayed(
                selected as u8 as f32,
                0.16,
                if selected { 0.06 } else { 0. },
                cx,
            );
        }
        self.overflow_x
            .duration(if self.over { middle * 4. } else { 0. }, 0.26, cx);
    }
    pub fn set_people(&mut self, people: Vec<Person>, cx: &mut Context<Self>) {
        let old = self.people.len().saturating_sub(self.limit);
        let more = people.len().saturating_sub(self.limit);
        // Retain departing keyed slots and their velocities. A re-added person reclaims the slot.
        for s in &mut self.slots {
            s.leaving = !people
                .iter()
                .take(self.limit)
                .any(|p| p.name == s.person.name);
            if s.leaving {
                s.space.duration(0., 0.42, cx);
            }
        }
        for (i, person) in people.iter().take(self.limit).cloned().enumerate() {
            if let Some(index) = self.slots.iter().position(|s| s.person.name == person.name) {
                let mut slot = self.slots.remove(index);
                slot.person = person;
                slot.space.duration(1., 0.42, cx);
                self.slots.insert(i.min(self.slots.len()), slot);
            } else {
                let mut slot = Slot::new(person, false);
                slot.space.duration(1., 0.42, cx);
                self.slots.insert(i.min(self.slots.len()), slot);
            }
        }
        if more > 0 {
            self.count
                .set(format!("+{more}"), (more as f32 - old as f32).signum(), cx);
        }
        self.overflow.duration((more > 0) as u8 as f32, 0.42, cx);
        self.people = people;
        self.roster.set(false, cx);
        self.fan(cx);
        cx.notify();
    }
}
impl Render for AvatarGroup {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        self.slots
            .retain(|s| !s.leaving || s.space.value(cx) > 0.001);
        let mut faces = row()
            .id("member-stack")
            .on_hover(cx.listener(|v, on: &bool, _, cx| {
                v.over = *on;
                if !*on {
                    v.active = None;
                }
                v.fan(cx);
                cx.notify();
            }));
        for (i, s) in self.slots.iter().enumerate() {
            let t = s.space.get(w, cx).clamp(0., 1.);
            let name = s.person.name.clone();
            let tip = s.tip.get(w, cx).clamp(0., 1.);
            let face = div()
                .id(s.person.name.clone())
                .relative()
                .left(px(s.x.get(w, cx)))
                .top(px(s.y.get(w, cx)))
                .on_hover(cx.listener(move |v, on: &bool, _, cx| {
                    if *on {
                        v.active = Some(name.clone());
                    } else if v.active.as_ref() == Some(&name) {
                        v.active = None;
                    }
                    v.fan(cx);
                    cx.notify();
                }))
                .size(px(40.))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .p(px(2.))
                .bg(p.stage)
                .rounded_full()
                .child(Avatar::portrait(&s.person, 36. * (0.9 + 0.1 * t), p))
                .when(tip > 0.001, |el| {
                    el.child(
                        div()
                            .absolute()
                            .bottom(px(43.))
                            .opacity(tip)
                            .whitespace_nowrap()
                            .child(
                                hint(s.person.name.clone(), p)
                                    .px_2()
                                    .py_1()
                                    .bg(p.panel)
                                    .rounded(crate::foundation::surface_radius(cx)),
                            ),
                    )
                });
            faces = faces.child(
                div()
                    .w(px(
                        if i + 1 == self.slots.len() && self.people.len() <= self.limit {
                            40. * t
                        } else {
                            33. * t
                        },
                    ))
                    .flex_none()
                    .relative()
                    .opacity(t)
                    .child(face),
            );
        }
        let extra = self.overflow.get(w, cx).clamp(0., 1.);
        if extra > 0.001 {
            faces = faces.child(
                div()
                    .relative()
                    .left(px(self.overflow_x.get(w, cx)))
                    .w(px(36. * extra))
                    .opacity(extra)
                    .child(
                        command("roster", "", cx, |s, _, cx| {
                            s.roster.set(!s.roster.open, cx);
                            cx.notify();
                        })
                        .aria_label(tr(cx, "Show members", "显示成员"))
                        .aria_expanded(self.roster.open)
                        .size(px(36.))
                        .p_0()
                        .rounded_full()
                        .child(self.count.render(12., w, cx)),
                    ),
            );
        }
        let mut root = column().items_center().child(faces);
        if self.roster.open || self.roster.fraction(w, cx) > 0.001 {
            let body = frame(220., p)
                .mt_2()
                .children(self.people.iter().map(|person| {
                    row()
                        .gap_2()
                        .child(Avatar::portrait(person, 24., p))
                        .child(hint(person.name.clone(), p))
                }));
            root = root.child(self.roster.render(body, w, cx));
        }
        root
    }
}
