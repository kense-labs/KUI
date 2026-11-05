//! A live, keyed presence stack with a keyboard-accessible roster.
use crate::{
    components::avatar::{Avatar, Person},
    primitives::{
        control::UiLanguage,
        family::{Paint, PhaseClock, Slide, disabled_motion, measure},
        menu::{MenuAlign, MenuControl, MenuCore, MenuItem, render_menu},
        presentation::ChangingText,
    },
};
use gpui::{prelude::*, *};
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PresenceStatus {
    #[default]
    Active,
    Idle,
    Typing,
}
/// The host supplies a stable ID, name, optional image, status and activity detail.
#[derive(Clone)]
pub struct PresencePerson {
    pub id: SharedString,
    pub name: SharedString,
    pub image: Option<ImageSource>,
    pub status: PresenceStatus,
    pub detail: Option<SharedString>,
}
impl PresencePerson {
    pub fn new(id: impl Into<SharedString>, name: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            image: None,
            status: PresenceStatus::Active,
            detail: None,
        }
    }
    pub fn with_image(mut self, image: impl Into<ImageSource>) -> Self {
        self.image = Some(image.into());
        self
    }
    pub fn with_status(mut self, status: PresenceStatus) -> Self {
        self.status = status;
        self
    }
    pub fn with_detail(mut self, detail: impl Into<SharedString>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PresenceSize {
    Small,
    #[default]
    Medium,
    Large,
}
impl PresenceSize {
    fn pixels(self) -> f32 {
        match self {
            Self::Small => 28.,
            Self::Medium => 36.,
            Self::Large => 44.,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PresenceAlign {
    #[default]
    Start,
    End,
}
/// A roster item has been selected; the host decides what following a person means.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PresenceSelected(pub SharedString);
struct Slot {
    person: PresencePerson,
    position: Slide,
    visible: Slide,
    leaving: bool,
}
impl Slot {
    fn new(person: PresencePerson, index: usize, arriving: bool) -> Self {
        Self {
            person,
            position: Slide::new(index as f32),
            visible: Slide::new(if arriving { 0. } else { 1. }),
            leaving: false,
        }
    }
}
/// Stable-width presence control. Updates are keyed by `PresencePerson::id`.
pub struct PresenceStack {
    people: Vec<PresencePerson>,
    slots: Vec<Slot>,
    max: usize,
    size: PresenceSize,
    align: PresenceAlign,
    self_id: Option<SharedString>,
    over: bool,
    active: Option<SharedString>,
    fan: Slide,
    label_x: Slide,
    label_width: Slide,
    label_text: SharedString,
    label_positioned: bool,
    overflow: ChangingText,
    clock: PhaseClock,
    core: MenuCore,
}
impl EventEmitter<PresenceSelected> for PresenceStack {}
impl PresenceStack {
    pub fn new(
        id: impl Into<ElementId>,
        people: Vec<PresencePerson>,
        cx: &mut Context<Self>,
    ) -> Self {
        let people = Self::unique(people);
        let slots = people
            .iter()
            .take(4)
            .cloned()
            .enumerate()
            .map(|(i, p)| Slot::new(p, i, false))
            .collect();
        let mut clock = PhaseClock::default();
        clock.restart(cx);
        let items = Self::rows(&people, None, UiLanguage::current(cx));
        Self {
            core: MenuCore::new(id.into(), "People here now".into(), items, cx),
            overflow: ChangingText::new(format!("+{}", people.len().saturating_sub(4))),
            people,
            slots,
            max: 4,
            size: PresenceSize::Medium,
            align: PresenceAlign::Start,
            self_id: None,
            over: false,
            active: None,
            fan: Slide::snappy(0.),
            label_x: Slide::snappy(0.),
            label_width: Slide::snappy(0.),
            label_text: "".into(),
            label_positioned: false,
            clock,
        }
    }
    fn unique(people: Vec<PresencePerson>) -> Vec<PresencePerson> {
        let mut ids = HashSet::new();
        people
            .into_iter()
            .filter(|p| ids.insert(p.id.clone()))
            .collect()
    }
    fn status_text(status: PresenceStatus, lang: UiLanguage) -> &'static str {
        match status {
            PresenceStatus::Active => lang.text("Active now", "在线"),
            PresenceStatus::Idle => lang.text("Idle", "暂离"),
            PresenceStatus::Typing => lang.text("Typing", "正在输入"),
        }
    }
    fn rows(people: &[PresencePerson], self_id: Option<&str>, lang: UiLanguage) -> Vec<MenuItem> {
        people
            .iter()
            .map(|p| {
                let you = if self_id == Some(p.id.as_ref()) {
                    lang.text(" (you)", "（你）")
                } else {
                    ""
                };
                let detail = p
                    .detail
                    .as_deref()
                    .unwrap_or_else(|| Self::status_text(p.status, lang));
                MenuItem::new(p.id.clone(), format!("{}{you}", p.name)).with_detail(detail)
            })
            .collect()
    }
    pub fn with_limit(mut self, max: usize) -> Self {
        self.max = max.clamp(1, 20);
        self.slots = self
            .people
            .iter()
            .take(self.max)
            .cloned()
            .enumerate()
            .map(|(i, p)| Slot::new(p, i, false))
            .collect();
        self.overflow =
            ChangingText::new(format!("+{}", self.people.len().saturating_sub(self.max)));
        self
    }
    pub fn with_size(mut self, size: PresenceSize) -> Self {
        self.size = size;
        self
    }
    pub fn with_align(mut self, align: PresenceAlign) -> Self {
        self.align = align;
        self.core.align = if align == PresenceAlign::End {
            MenuAlign::End
        } else {
            MenuAlign::Start
        };
        self
    }
    pub fn with_self_id(mut self, id: impl Into<SharedString>, cx: &App) -> Self {
        self.self_id = Some(id.into());
        self.core.replace_items(Self::rows(
            &self.people,
            self.self_id.as_deref(),
            UiLanguage::current(cx),
        ));
        self
    }
    pub fn is_open(&self) -> bool {
        self.core.open
    }
    /// Reconcile a new host snapshot, retaining the positions of returning IDs.
    pub fn set_people(&mut self, people: Vec<PresencePerson>, cx: &mut Context<Self>) {
        let people = Self::unique(people);
        let before = self.people.len().saturating_sub(self.max);
        for slot in &mut self.slots {
            slot.leaving = !people.iter().take(self.max).any(|p| p.id == slot.person.id);
            if slot.leaving {
                slot.visible.duration(0., 0.16, cx);
            }
        }
        for (i, person) in people.iter().take(self.max).cloned().enumerate() {
            if let Some(index) = self.slots.iter().position(|s| s.person.id == person.id) {
                let slot = &mut self.slots[index];
                slot.person = person;
                slot.leaving = false;
                slot.position.duration(i as f32, 0.4, cx);
                slot.visible.duration(1., 0.42, cx);
            } else {
                let mut slot = Slot::new(person, i, true);
                slot.visible.duration(1., 0.42, cx);
                self.slots.push(slot);
            }
        }
        let after = people.len().saturating_sub(self.max);
        self.overflow.set(
            format!("+{after}"),
            (after as f32 - before as f32).signum(),
            cx,
        );
        if self
            .active
            .as_ref()
            .is_some_and(|id| !people.iter().take(self.max).any(|p| &p.id == id))
        {
            self.active = None;
        }
        self.people = people;
        self.core.replace_items(Self::rows(
            &self.people,
            self.self_id.as_deref(),
            UiLanguage::current(cx),
        ));
        cx.notify();
    }
    fn choose_active(&mut self, i: usize, cx: &mut Context<Self>) {
        if let Some(person) = self.people.get(i).filter(|_| i < self.max) {
            self.active = Some(person.id.clone());
            self.label_text = person.name.clone();
            cx.notify();
        }
    }
}
impl MenuControl for PresenceStack {
    fn core(&mut self) -> &mut MenuCore {
        &mut self.core
    }
    fn selected(&mut self, i: usize, w: &mut Window, cx: &mut Context<Self>) {
        if let Some(person) = self.people.get(i) {
            cx.emit(PresenceSelected(person.id.clone()));
        }
        self.core.close(w, cx);
        cx.notify();
    }
    fn roster(&self) -> bool {
        true
    }
    fn leading(&self, index: usize, active: bool, cx: &Context<Self>) -> Option<AnyElement> {
        let person = self.people.get(index)?;
        let p = Paint::get(cx);
        let ring = if active { p.tray } else { p.panel };
        let initials: String = person
            .name
            .split_whitespace()
            .filter_map(|part| part.chars().next())
            .take(2)
            .collect();
        Some(
            div()
                .size(px(32.))
                .flex_none()
                .rounded_full()
                .overflow_hidden()
                .bg(p.tray)
                .border_2()
                .border_color(ring)
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(12.))
                .line_height(px(12.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.secondary)
                .child(initials)
                .into_any_element(),
        )
    }
    fn header(&self, cx: &App) -> Option<AnyElement> {
        let lang = UiLanguage::current(cx);
        let p = Paint::get(cx);
        Some(
            div()
                .pt(px(8.))
                .px(px(12.))
                .pb(px(6.))
                .flex()
                .items_baseline()
                .justify_between()
                .gap(px(12.))
                .text_size(px(14.))
                .line_height(px(20.))
                .text_color(p.ink)
                .font_weight(FontWeight::MEDIUM)
                .child(lang.text("Here now", "当前在线"))
                .child(
                    div()
                        .text_color(p.faint)
                        .font_weight(FontWeight::NORMAL)
                        .child(self.people.len().to_string()),
                )
                .into_any_element(),
        )
    }
}
impl Render for PresenceStack {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let lang = UiLanguage::current(cx);
        let d = self.size.pixels();
        self.slots
            .retain(|s| !s.leaving || s.visible.value(cx) > 0.01);
        let focus = self.core.trigger.is_focused(w);
        let expanded = self.over || focus;
        if expanded
            && self.active.is_none()
            && let Some(person) = self.people.first()
        {
            self.active = Some(person.id.clone());
            self.label_text = person.name.clone();
        }
        if !expanded {
            self.label_positioned = false;
        }
        self.fan.duration(expanded as u8 as f32, 0.4, cx);
        let fan = self.fan.get(w, cx).clamp(0., 1.);
        let step = d * (0.68 + 0.38 * fan);
        let count = self.people.len().min(self.max);
        let extra = self.people.len().saturating_sub(self.max);
        let reserve = d + self.max as f32 * d * 0.68;
        let used = d + (count.saturating_sub(1) + usize::from(extra > 0)) as f32 * step;
        let origin = if self.align == PresenceAlign::End {
            reserve - used
        } else {
            0.
        };
        let mut stack = div().relative().w(px(reserve)).h(px(d + 4.)).flex_none();
        let animated_typing = !disabled_motion(cx)
            && self
                .people
                .iter()
                .take(self.max)
                .any(|person| person.status == PresenceStatus::Typing);
        if animated_typing {
            w.request_animation_frame();
        }
        let age = self.clock.age(cx);
        for slot in &mut self.slots {
            let index = slot.position.get(w, cx).max(0.);
            let progress = slot.visible.get(w, cx).clamp(0., 1.);
            let person = slot.person.clone();
            let name = person.name.clone();
            let id = person.id.clone();
            let is_active = expanded && self.active.as_ref() == Some(&person.id);
            let portrait = Person {
                name: name.clone(),
                image: person.image.clone(),
            };
            let status_color = match person.status {
                PresenceStatus::Active | PresenceStatus::Typing => p.success,
                PresenceStatus::Idle => p.faint,
            };
            let mut mark = div()
                .absolute()
                .bottom(px(-1.))
                .right(px(if person.status == PresenceStatus::Typing {
                    -4.
                } else {
                    -1.
                }))
                .h(px(if person.status == PresenceStatus::Typing {
                    12.
                } else {
                    10.
                }))
                .w(px(if person.status == PresenceStatus::Typing {
                    21.
                } else {
                    10.
                }))
                .rounded_full()
                .border_2()
                .border_color(p.stage)
                .bg(if person.status == PresenceStatus::Typing {
                    p.ink
                } else {
                    status_color
                });
            if person.status == PresenceStatus::Typing {
                mark = mark.flex().items_center().justify_center().gap(px(2.));
                for dot in 0..3 {
                    let cycle = if animated_typing {
                        ((age - dot as f32 * 0.14) / 1.1).rem_euclid(1.)
                    } else {
                        1.
                    };
                    let lift = if cycle < 0.3 {
                        cycle / 0.3
                    } else if cycle < 0.6 {
                        1. - (cycle - 0.3) / 0.3
                    } else {
                        0.
                    };
                    mark = mark.child(
                        div()
                            .relative()
                            .top(px(-2. * lift))
                            .size(px(3.))
                            .rounded_full()
                            .bg(p.stage.opacity(0.5 + 0.5 * lift)),
                    );
                }
            }
            stack = stack.child(
                div()
                    .id(SharedString::from(format!("presence-{id}")))
                    .absolute()
                    .left(px(origin + index * step))
                    .top(px(if is_active { -2. } else { 0. }))
                    .size(px(d))
                    .rounded_full()
                    .border_2()
                    .border_color(p.stage)
                    .opacity(progress)
                    .on_hover(cx.listener(move |s, over: &bool, _, cx| {
                        if *over {
                            s.active = Some(id.clone());
                            s.label_text = name.clone();
                            cx.notify();
                        }
                    }))
                    .child(
                        Avatar::portrait(&portrait, d * (0.4 + 0.6 * progress), p).opacity(
                            if person.status == PresenceStatus::Idle {
                                0.55
                            } else {
                                1.
                            },
                        ),
                    )
                    .child(mark),
            );
        }
        if extra > 0 {
            stack = stack.child(
                div()
                    .absolute()
                    .left(px(origin + count as f32 * step))
                    .top(px(0.))
                    .size(px(d))
                    .rounded_full()
                    .border_2()
                    .border_color(p.stage)
                    .bg(p.tray)
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(d * 0.34))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.secondary)
                    .child(self.overflow.render(d * 0.34, w, cx)),
            );
        }
        if expanded
            && let Some(i) = self
                .people
                .iter()
                .take(self.max)
                .position(|person| Some(&person.id) == self.active.as_ref())
        {
            let mut text = self.label_text.to_string();
            if self.self_id.as_deref() == self.active.as_deref() {
                text.push_str(lang.text(" (you)", "（你）"));
            }
            let width = measure(&SharedString::from(text.clone()), 12., w) + 20.;
            if self.label_width.value(cx) < 1. {
                self.label_width.reset(width);
            } else {
                self.label_width.duration(width, 0.42, cx);
            }
            let target = origin + i as f32 * step + d * 0.5;
            if !self.label_positioned {
                self.label_x.reset(target);
                self.label_positioned = true;
            } else {
                self.label_x.duration(target, 0.42, cx);
            }
            let label_w = self.label_width.get(w, cx);
            stack = stack.child(
                div()
                    .absolute()
                    .left(px(self.label_x.get(w, cx) - label_w * 0.5))
                    .top(px(d + 8.))
                    .w(px(label_w))
                    .h(px(26.))
                    .px(px(10.))
                    .rounded_full()
                    .bg(p.ink)
                    .shadow(vec![gpui::BoxShadow {
                        color: black().opacity(0.12),
                        offset: gpui::point(px(0.), px(4.)),
                        blur_radius: px(12.),
                        spread_radius: px(0.),
                        inset: false,
                    }])
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(px(6.))
                    .text_size(px(12.))
                    .line_height(px(26.))
                    .text_color(p.stage)
                    .child(text),
            );
        }
        let label: SharedString = format!(
            "{}: {}",
            lang.text("People here now", "当前在线"),
            self.people.len()
        )
        .into();
        self.core.name = label.clone();
        let button = div()
            .id("presence-stack-trigger")
            .debug_selector(|| "Presence stack".into())
            .role(Role::Button)
            .aria_label(label)
            .track_focus(&self.core.trigger)
            .tab_index(0)
            .relative()
            .cursor_pointer()
            .child(stack)
            .on_hover(cx.listener(|s, over: &bool, w, cx| {
                s.over = *over;
                if !*over && !s.core.trigger.is_focused(w) {
                    s.active = None;
                }
                cx.notify();
            }));
        let panel = render_menu(self, button, false, 288., w, cx);
        div()
            .relative()
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                if s.core.open {
                    return;
                }
                let count = s.people.len().min(s.max);
                if count == 0 {
                    return;
                }
                let old = s
                    .active
                    .as_ref()
                    .and_then(|id| s.people.iter().take(count).position(|p| &p.id == id));
                let next = match e.keystroke.key.as_str() {
                    "right" => old.map_or(0, |old| (old + 1) % count),
                    "left" => old.map_or(count - 1, |old| (old + count - 1) % count),
                    _ => return,
                };
                s.choose_active(next, cx);
                w.prevent_default();
                cx.stop_propagation();
            }))
            .child(panel)
    }
}
