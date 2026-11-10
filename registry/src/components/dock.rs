//! A controlled tool dock with a gliding selection and integrated group trays.
use crate::{
    components::animated_counter::AnimatedCounter,
    foundation::surface_radius,
    primitives::{
        control::UiDirection,
        family::{Paint, PhaseClock, Slide, Tone, ease, face, measure},
        flyout::TriggerBounds,
        glyph::{Glyph, icon},
        presentation::{row, tr},
    },
};
use gpui::{prelude::*, *};
use std::time::Duration;

const SLOT: f32 = 40.;
const STEP: f32 = 44.;
const PAD: f32 = 6.;
const BAR: f32 = 52.;
const TRAY: f32 = 48.;
const MEMBER: f32 = 36.;
const MEMBER_STEP: f32 = 40.;
const DRAG_SLOP: f32 = 6.;

/// A dock pictogram. Shared KUI glyphs can be used for other tools.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DockIcon {
    Pointer,
    Hand,
    Rectangle,
    Ellipse,
    Triangle,
    Star,
    Text,
    Note,
    Glyph(Glyph),
}

impl DockIcon {
    /// Draws the same tool pictogram used by the dock.
    pub fn render(self, size: f32, color: Hsla) -> impl IntoElement {
        div().size(px(size)).flex_none().child(match self {
            Self::Glyph(glyph) => icon(glyph, size, color).into_any_element(),
            _ => canvas(
                |_, _, _| (),
                move |bounds, _, w, _| draw_icon(self, size, bounds.origin, color, w),
            )
            .size_full()
            .into_any_element(),
        })
    }
}

/// A tool, or a group of directly selectable tools.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DockItem {
    pub id: SharedString,
    pub label: SharedString,
    pub icon: DockIcon,
    pub shortcut: Option<SharedString>,
    pub badge: u32,
    pub items: Vec<DockItem>,
}

impl DockItem {
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        icon: DockIcon,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon,
            shortcut: None,
            badge: 0,
            items: vec![],
        }
    }

    pub fn with_shortcut(mut self, shortcut: impl Into<SharedString>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    pub fn with_badge(mut self, count: u32) -> Self {
        self.badge = count;
        self
    }

    pub fn with_items(mut self, items: Vec<Self>) -> Self {
        self.items = items;
        self
    }
}

/// The requested selection. The host applies it with `Dock::set_value`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DockSelected(pub SharedString);

/// The new order and the moved tool's zero-based destination.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DockReordered {
    pub id: SharedString,
    pub index: usize,
    pub items: Vec<DockItem>,
}

struct Slot {
    item: DockItem,
    focus: FocusHandle,
    member_focus: Vec<FocusHandle>,
    shown: usize,
    position: Slide,
    press: Slide,
    old_icon: Option<DockIcon>,
    icon_time: PhaseClock,
    badge: Option<Entity<AnimatedCounter>>,
    badge_presence: Slide,
}

impl Slot {
    fn new(item: DockItem, index: usize, cx: &mut Context<Dock>) -> Self {
        let mut slot = Self {
            focus: cx.focus_handle().tab_stop(true),
            member_focus: item
                .items
                .iter()
                .map(|_| cx.focus_handle().tab_stop(true))
                .collect(),
            shown: 0,
            position: Slide::new(index as f32),
            press: Slide::new(1.),
            old_icon: None,
            icon_time: PhaseClock::default(),
            badge: None,
            badge_presence: Slide::new(f32::from(item.badge > 0)),
            item,
        };
        slot.sync_badge(cx);
        slot
    }

    fn visible_item(&self) -> &DockItem {
        self.item.items.get(self.shown).unwrap_or(&self.item)
    }

    fn sync_badge(&mut self, cx: &mut Context<Dock>) {
        let value = self.item.badge;
        if let Some(badge) = &self.badge {
            badge.update(cx, |badge, cx| badge.set_value(f64::from(value), cx));
        } else if value > 0 {
            self.badge = Some(cx.new(|_| {
                AnimatedCounter::new(f64::from(value))
                    .with_accessibility(false)
                    .with_size(10.)
                    .with_wheel_style(1.1, 0.10, FontWeight::SEMIBOLD)
            }));
        }
        self.badge_presence.duration(f32::from(value > 0), 0.16, cx);
    }

    fn show_member(&mut self, member: usize, cx: &App) {
        if self.shown != member {
            self.old_icon = Some(self.visible_item().icon);
            self.shown = member;
            self.icon_time.restart(cx);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Target {
    Slot(usize),
    Member(usize, usize),
}

#[derive(Clone, Copy)]
struct Drag {
    slot: usize,
    origin: Point<Pixels>,
    last_x: f32,
    to: usize,
    touch: bool,
    moving: bool,
    vertical: bool,
}

/// A horizontal toolbar with stable slots and a controlled selected value.
pub struct Dock {
    label: Option<SharedString>,
    slots: Vec<Slot>,
    value: Option<SharedString>,
    reorderable: bool,
    roving: usize,
    open: Option<usize>,
    retained: Option<usize>,
    tray: Slide,
    tray_x: Slide,
    tray_time: PhaseClock,
    selection: Option<Slide>,
    hover: Option<Target>,
    tooltip: Option<Target>,
    tip_x: Option<Slide>,
    tip_width: Option<Slide>,
    tip_opacity: Slide,
    tip_delay: Option<Task<()>>,
    tip_warmth: PhaseClock,
    recently_touched: PhaseClock,
    drag: Option<Drag>,
    consumed: Option<usize>,
    bounds: TriggerBounds,
    scroll: ScrollHandle,
}

impl EventEmitter<DockSelected> for Dock {}
impl EventEmitter<DockReordered> for Dock {}

impl Dock {
    pub fn new(items: Vec<DockItem>, cx: &mut Context<Self>) -> Self {
        Self {
            label: None,
            slots: items
                .into_iter()
                .enumerate()
                .map(|(i, item)| Slot::new(item, i, cx))
                .collect(),
            value: None,
            reorderable: false,
            roving: 0,
            open: None,
            retained: None,
            tray: Slide::new(0.),
            tray_x: Slide::new(0.),
            tray_time: PhaseClock::default(),
            selection: None,
            hover: None,
            tooltip: None,
            tip_x: None,
            tip_width: None,
            tip_opacity: Slide::new(0.),
            tip_delay: None,
            tip_warmth: PhaseClock::default(),
            recently_touched: PhaseClock::default(),
            drag: None,
            consumed: None,
            bounds: TriggerBounds::default(),
            scroll: ScrollHandle::new(),
        }
    }

    pub fn with_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn with_value(mut self, value: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        self.set_value(Some(value.into()), cx);
        self
    }

    /// Enables drag and Alt+Arrow reorder requests.
    pub fn with_reorderable(mut self, enabled: bool) -> Self {
        self.reorderable = enabled;
        self
    }

    pub fn set_value(&mut self, value: Option<SharedString>, cx: &mut Context<Self>) {
        if let Some((slot, Some(member))) = value.as_ref().and_then(|key| self.find(key)) {
            self.slots[slot].show_member(member, cx);
        }
        self.value = value;
        cx.notify();
    }

    /// Reconciles incoming order, badges and metadata using stable item IDs.
    pub fn set_items(&mut self, items: Vec<DockItem>, cx: &mut Context<Self>) {
        let active = self.slots.get(self.roving).map(|slot| slot.item.id.clone());
        let group = self
            .open
            .and_then(|i| self.slots.get(i))
            .map(|slot| slot.item.id.clone());
        let mut old = std::mem::take(&mut self.slots);
        self.slots = items
            .into_iter()
            .enumerate()
            .map(|(i, item)| {
                let Some(index) = old.iter().position(|slot| slot.item.id == item.id) else {
                    return Slot::new(item, i, cx);
                };
                let mut slot = old.remove(index);
                let shown = slot.visible_item().id.clone();
                if slot
                    .item
                    .items
                    .iter()
                    .map(|i| &i.id)
                    .ne(item.items.iter().map(|i| &i.id))
                {
                    slot.member_focus = item
                        .items
                        .iter()
                        .map(|_| cx.focus_handle().tab_stop(true))
                        .collect();
                }
                slot.shown = item.items.iter().position(|i| i.id == shown).unwrap_or(0);
                slot.item = item;
                slot.position.duration(i as f32, 0.24, cx);
                slot.sync_badge(cx);
                slot
            })
            .collect();
        self.roving = active
            .and_then(|id| self.slots.iter().position(|slot| slot.item.id == id))
            .unwrap_or(0);
        self.open = group.and_then(|id| {
            self.slots
                .iter()
                .position(|slot| slot.item.id == id && !slot.item.items.is_empty())
        });
        self.retained = self.open;
        if self.open.is_none() {
            self.tray.reset(0.);
        }
        self.drag = None;
        self.hover = None;
        self.tip_delay = None;
        self.tip_opacity.reset(0.);
        self.tooltip = None;
        self.set_value(self.value.clone(), cx);
    }

    fn find(&self, id: &str) -> Option<(usize, Option<usize>)> {
        self.slots.iter().enumerate().find_map(|(i, slot)| {
            if slot.item.id == id {
                Some((i, None))
            } else {
                slot.item
                    .items
                    .iter()
                    .position(|item| item.id == id)
                    .map(|j| (i, Some(j)))
            }
        })
    }

    fn focus_slot(&mut self, i: usize, w: &mut Window, cx: &mut Context<Self>) {
        self.roving = i;
        let left = if UiDirection::current(cx).is_rtl() {
            self.slots.len() - 1 - i
        } else {
            i
        };
        let left = px(PAD + STEP * left as f32);
        let width = self.scroll.bounds().size.width;
        let offset = self.scroll.offset().x;
        if width > px(0.) {
            let offset = if left + offset < px(PAD) {
                px(PAD) - left
            } else if left + offset + px(SLOT + PAD) > width {
                width - left - px(SLOT + PAD)
            } else {
                offset
            };
            self.scroll.set_offset(point(offset.min(px(0.)), px(0.)));
        }
        self.slots[i].focus.focus(w, cx);
        cx.notify();
    }

    fn close(&mut self, return_focus: bool, w: &mut Window, cx: &mut Context<Self>) {
        if let Some(i) = self.open.take() {
            self.tray.duration(0., 0.19, cx);
            if return_focus {
                self.focus_slot(i, w, cx);
            }
            if matches!(self.hover, Some(Target::Member(_, _))) {
                self.hover = None;
                self.tip_opacity.duration(0., 0.11, cx);
            }
            cx.notify();
        }
    }

    fn show_tray(&mut self, i: usize, move_focus: bool, w: &mut Window, cx: &mut Context<Self>) {
        if self.slots[i].item.items.is_empty() {
            return;
        }
        self.open = Some(i);
        self.retained = Some(i);
        self.tray.duration(1., 0.27, cx);
        self.tray_time.restart(cx);
        if move_focus {
            let focus = self.slots[i].member_focus[self.slots[i].shown].clone();
            cx.defer_in(w, move |_, w, cx| focus.focus(w, cx));
        }
        cx.notify();
    }

    fn choose(&mut self, target: Target, w: &mut Window, cx: &mut Context<Self>) {
        let id = match target {
            Target::Slot(i) => {
                if self.consumed.take() == Some(i)
                    || self.drag.is_some_and(|d| d.moving || d.vertical)
                {
                    return;
                }
                self.focus_slot(i, w, cx);
                if self.open == Some(i) || self.slots[i].item.items.is_empty() {
                    self.close(false, w, cx);
                } else {
                    self.show_tray(i, false, w, cx);
                }
                self.slots[i].visible_item().id.clone()
            }
            Target::Member(i, j) => {
                let id = self.slots[i].item.items[j].id.clone();
                self.close(true, w, cx);
                id
            }
        };
        cx.emit(DockSelected(id));
    }

    fn reorder(&mut self, from: usize, to: usize, cx: &mut Context<Self>) {
        if !self.reorderable || from == to || from >= self.slots.len() || to >= self.slots.len() {
            return;
        }
        let mut items: Vec<_> = self.slots.iter().map(|slot| slot.item.clone()).collect();
        let item = items.remove(from);
        let id = item.id.clone();
        items.insert(to, item);
        cx.emit(DockReordered {
            id,
            index: to,
            items,
        });
    }

    fn key(&mut self, target: Target, e: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        let step =
            UiDirection::current(cx).inline_step(if e.keystroke.key == "left" { -1 } else { 1 });
        match (target, e.keystroke.key.as_str()) {
            (Target::Slot(i), "left" | "right") if e.keystroke.modifiers.alt => {
                let to = i.saturating_add_signed(step).min(self.slots.len() - 1);
                self.reorder(i, to, cx);
            }
            (Target::Slot(i), "left" | "right") => {
                self.close(false, w, cx);
                self.focus_slot(
                    (i as isize + step).rem_euclid(self.slots.len() as isize) as usize,
                    w,
                    cx,
                );
            }
            (Target::Slot(_), "home" | "end") => {
                self.close(false, w, cx);
                self.focus_slot(
                    if e.keystroke.key == "home" {
                        0
                    } else {
                        self.slots.len() - 1
                    },
                    w,
                    cx,
                );
            }
            (Target::Slot(i), "up") if !self.slots[i].item.items.is_empty() => {
                self.show_tray(i, true, w, cx)
            }
            (Target::Member(i, j), "left" | "right" | "home" | "end") => {
                let handles = &self.slots[i].member_focus;
                let next = match e.keystroke.key.as_str() {
                    "home" => 0,
                    "end" => handles.len() - 1,
                    _ => (j as isize + step).rem_euclid(handles.len() as isize) as usize,
                };
                handles[next].focus(w, cx);
                cx.notify();
            }
            (Target::Member(_, _), "down") | (_, "escape") if self.open.is_some() => {
                self.close(true, w, cx)
            }
            (Target::Member(_, _), "tab") => {
                self.close(true, w, cx);
                return;
            }
            (_, "enter" | "space") if !e.is_held => {
                self.consumed = None;
                self.choose(target, w, cx);
            }
            _ => return,
        }
        cx.stop_propagation();
        w.prevent_default();
    }

    fn hover(&mut self, target: Target, over: bool, cx: &mut Context<Self>) {
        if !over {
            if self.hover == Some(target) {
                self.hover = None;
                self.tip_delay = None;
                self.tip_opacity.duration(0., 0.12, cx);
                cx.notify();
            }
            return;
        }
        if self.recently_touched.age(cx) < 0.55 || self.drag.is_some_and(|d| d.moving) {
            return;
        }
        self.hover = Some(target);
        if self.tip_warmth.age(cx) < 0.35 || self.tip_opacity.value(cx) > 0.01 {
            self.show_tip(target, cx);
        } else {
            self.tip_delay = Some(cx.spawn(async move |view, cx| {
                cx.background_executor()
                    .timer(Duration::from_millis(180))
                    .await;
                let _ = view.update(cx, |view, cx| {
                    if view.hover == Some(target) {
                        view.show_tip(target, cx);
                    }
                });
            }));
        }
        cx.notify();
    }

    fn show_tip(&mut self, target: Target, cx: &mut Context<Self>) {
        self.tip_delay = None;
        self.tooltip = Some(target);
        self.tip_warmth.restart(cx);
        self.tip_opacity.duration(1., 0.15, cx);
        cx.notify();
    }

    fn keyboard_target(&self, w: &Window) -> Option<Target> {
        if !w.last_input_was_keyboard() {
            return None;
        }
        if let Some(i) = self.open
            && let Some(j) = self.slots[i]
                .member_focus
                .iter()
                .position(|f| f.is_focused(w))
        {
            return Some(Target::Member(i, j));
        }
        self.slots
            .iter()
            .position(|slot| slot.focus.is_focused(w))
            .map(Target::Slot)
    }

    fn gesture_start(
        &mut self,
        slot: usize,
        at: Point<Pixels>,
        touch: bool,
        cx: &mut Context<Self>,
    ) {
        if self.drag.is_some_and(|d| d.touch) && !touch {
            return;
        }
        self.consumed = None;
        self.slots[slot].press.duration(0.92, 0.10, cx);
        self.drag = Some(Drag {
            slot,
            origin: at,
            last_x: 0.,
            to: slot,
            touch,
            moving: false,
            vertical: false,
        });
        cx.notify();
    }

    fn gesture_move(
        &mut self,
        position: Point<Pixels>,
        touch: bool,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(mut drag) = self.drag.filter(|d| d.touch == touch && !d.vertical) else {
            return;
        };
        let dx = f32::from(position.x - drag.origin.x);
        let dy = f32::from(position.y - drag.origin.y);
        if !drag.moving && dy.abs() > DRAG_SLOP && dy.abs() > dx.abs() {
            drag.vertical = true;
            self.slots[drag.slot].press.duration(1., 0.15, cx);
            self.drag = Some(drag);
            cx.notify();
            return;
        }
        if !drag.moving && (!self.reorderable || dx.abs() <= DRAG_SLOP || dx.abs() <= dy.abs()) {
            return;
        }
        if !drag.moving {
            self.close(false, w, cx);
            self.hover = None;
            self.tip_delay = None;
            self.tip_opacity.duration(0., 0.10, cx);
        }
        let direction = if UiDirection::current(cx).is_rtl() {
            -1.
        } else {
            1.
        };
        drag.moving = true;
        drag.last_x = dx;
        drag.to = (drag.slot as f32 + dx * direction / STEP)
            .round()
            .clamp(0., (self.slots.len() - 1) as f32) as usize;
        for (index, slot) in self.slots.iter_mut().enumerate() {
            let target = if drag.slot < index && index <= drag.to {
                index - 1
            } else if drag.to <= index && index < drag.slot {
                index + 1
            } else {
                index
            };
            slot.position.duration(target as f32, 0.24, cx);
        }
        self.drag = Some(drag);
        if touch {
            w.prevent_default();
        }
        cx.notify();
    }

    fn gesture_end(
        &mut self,
        at: Point<Pixels>,
        touch: bool,
        cancelled: bool,
        cx: &mut Context<Self>,
    ) {
        let Some(drag) = self.drag.filter(|d| d.touch == touch) else {
            return;
        };
        self.drag = None;
        if drag.moving || drag.vertical || cancelled {
            self.consumed = Some(drag.slot);
        }
        if drag.moving {
            for (i, slot) in self.slots.iter_mut().enumerate() {
                slot.position.duration(i as f32, 0.24, cx);
            }
            if !cancelled && self.bounds.get().contains(&at) {
                self.reorder(drag.slot, drag.to, cx);
            }
        }
        let press = &mut self.slots[drag.slot].press;
        if cancelled || drag.vertical || drag.moving {
            press.duration(1., 0.15, cx);
        } else {
            press.release(1., 4., 0.18, cx);
        }
        cx.notify();
    }

    fn visible_target(&self, t: Target) -> Option<&DockItem> {
        match t {
            Target::Slot(i) => self.slots.get(i).map(Slot::visible_item),
            Target::Member(i, j) => self.slots.get(i)?.item.items.get(j),
        }
    }

    fn display_name(&self, t: Target, selected: bool, cx: &App) -> SharedString {
        let Some(item) = self.visible_target(t) else {
            return "".into();
        };
        let mut name = item.label.to_string();
        if let Target::Slot(i) = t
            && !self.slots[i].item.items.is_empty()
        {
            name = format!("{}: {name}", self.slots[i].item.label);
        }
        if let Some(key) = &item.shortcut {
            name.push_str(&format!(", {key}"));
        }
        if item.badge > 0 {
            name.push_str(&format!(", {}", item.badge));
        }
        if selected && matches!(t, Target::Slot(i) if !self.slots[i].item.items.is_empty()) {
            name.push_str(tr(cx, ", selected", "，已选中"));
        }
        name.into()
    }

    fn slot_x(&self, i: usize, rtl: bool, w: &Window, cx: &App) -> f32 {
        let position = self.slots[i].position.get(w, cx);
        PAD + if rtl {
            (self.slots.len() as f32 - 1. - position) * STEP
        } else {
            position * STEP
        }
    }

    fn button(
        &self,
        t: Target,
        id: impl Into<ElementId>,
        selected: bool,
        w: &Window,
        cx: &Context<Self>,
    ) -> Stateful<Div> {
        let size = if matches!(t, Target::Slot(_)) {
            SLOT
        } else {
            MEMBER
        };
        let p = Paint::get(cx);
        face(
            id,
            self.display_name(t, selected, cx),
            size,
            surface_radius(cx).into(),
            Tone::Clear,
            cx,
        )
        .size(px(size))
        .relative()
        .gap_0()
        .border_1()
        .border_color(transparent_black())
        .text_color(if selected { p.ink } else { p.secondary })
        .hover(move |s| s.bg(p.hover.opacity(0.4)))
        .on_hover(cx.listener(move |s, over: &bool, _, cx| s.hover(t, *over, cx)))
        .on_click(cx.listener(move |s, _, w, cx| s.choose(t, w, cx)))
        .on_key_down(cx.listener(move |s, e: &KeyDownEvent, w, cx| s.key(t, e, w, cx)))
        .when(matches!(t, Target::Member(_, _)), |button| {
            button.child(
                self.visible_target(t)
                    .map_or(DockIcon::Rectangle, |i| i.icon)
                    .render(19., p.ink),
            )
        })
        .when(matches!(t, Target::Slot(_)), |button| {
            let Target::Slot(i) = t else {
                return button;
            };
            let slot = &self.slots[i];
            let age = slot.icon_time.time(0.16, w, cx);
            let alpha = ease(age / 0.16);
            let side = 20. * slot.press.get(w, cx);
            let color = if selected { p.ink } else { p.secondary };
            button.child(
                row()
                    .relative()
                    .size(px(side))
                    .justify_center()
                    .child(
                        div()
                            .opacity(alpha)
                            .child(slot.visible_item().icon.render(side, color)),
                    )
                    .when(alpha < 1., |el| {
                        el.children(slot.old_icon.map(|old| {
                            div()
                                .absolute()
                                .inset_0()
                                .opacity(1. - alpha)
                                .child(old.render(side, color))
                        }))
                    }),
            )
        })
    }

    fn tip(
        &mut self,
        bar_width: f32,
        tray_left: f32,
        tray_top: f32,
        rtl: bool,
        w: &Window,
        cx: &Context<Self>,
    ) -> Option<Div> {
        let keyboard = self.keyboard_target(w);
        if let Some(key) = keyboard {
            self.tooltip = Some(key);
            self.tip_opacity.duration(1., 0.12, cx);
        } else if self.hover.is_none() {
            self.tip_opacity.duration(0., 0.12, cx);
        }
        let alpha = self.tip_opacity.get(w, cx).clamp(0., 1.);
        let t = self.tooltip?;
        if alpha < 0.002 || self.drag.is_some_and(|d| d.moving) {
            return None;
        }
        let item = self.visible_target(t)?;
        let label = item.label.clone();
        let shortcut = item.shortcut.clone();
        let width = (measure(&label, 12., w).ceil()
            + 20.
            + shortcut
                .as_ref()
                .map_or(0., |key| measure(key, 10., w).ceil() + 14.))
        .min(bar_width.max(1.));
        let center = match t {
            Target::Slot(i) => {
                self.slot_x(i, rtl, w, cx) + SLOT / 2. + f32::from(self.scroll.offset().x)
            }
            Target::Member(i, j) => {
                let slot_count = self.slots[i].item.items.len();
                let j = if rtl { slot_count - 1 - j } else { j };
                tray_left + PAD + MEMBER / 2. + j as f32 * MEMBER_STEP
            }
        };
        let left = (center - width / 2.).clamp(0., (bar_width - width).max(0.));
        let x = self.tip_x.get_or_insert(Slide::new(left));
        x.duration(left, 0.22, cx);
        let width_slide = self.tip_width.get_or_insert(Slide::new(width));
        width_slide.duration(width, 0.21, cx);
        let y = if matches!(t, Target::Member(_, _)) {
            -38. - tray_top
        } else {
            -38.
        };
        let p = Paint::get(cx);
        Some(
            row()
                .absolute()
                .left(px(x.get(w, cx)))
                .top(px(y))
                .w(px(width_slide.get(w, cx)))
                .h(px(28.))
                .gap(px(6.))
                .px(px(10.))
                .rounded(surface_radius(cx))
                .bg(p.bright)
                .text_color(p.inverse)
                .text_size(px(12.))
                .opacity(alpha)
                .overflow_hidden()
                .child(div().min_w_0().truncate().child(label))
                .children(shortcut.map(|key| {
                    div()
                        .flex_none()
                        .px(px(4.))
                        .rounded(px(4.))
                        .bg(p.inverse.opacity(0.14))
                        .text_size(px(10.))
                        .child(key)
                })),
        )
    }
}

impl Render for Dock {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let rtl = UiDirection::current(cx).is_rtl();
        let natural_width = (self.slots.len().max(1) - 1) as f32 * STEP + SLOT + 2. * PAD;
        let width = f32::from(self.bounds.get().size.width).max(1.);
        let bar_width = if width > 1. {
            width
        } else {
            natural_width.min(f32::from(w.viewport_size().width))
        };
        let active = self
            .value
            .as_ref()
            .and_then(|id| self.find(id))
            .map(|(i, _)| i);
        let tray_amount = self.tray.get(w, cx).clamp(0., 1.);
        let tray_width = self.retained.map_or(0., |i| {
            let n = self.slots[i].item.items.len();
            (2. * PAD + MEMBER + MEMBER_STEP * n.saturating_sub(1) as f32).min(bar_width)
        });
        let tray_target = self.retained.map_or(0., |i| {
            (self.slot_x(i, rtl, w, cx) + f32::from(self.scroll.offset().x) + SLOT / 2.
                - tray_width / 2.)
                .clamp(0., (bar_width - tray_width).max(0.))
        });
        if tray_amount < 0.01 {
            self.tray_x.reset(tray_target);
        }
        self.tray_x.duration(tray_target, 0.22, cx);
        let tray_left = self.tray_x.get(w, cx);
        let tray_top = TRAY * tray_amount;
        let radius: f32 = surface_radius(cx).into();
        let tint = p.bright.opacity(0.13);
        let mut track = div().relative().w(px(natural_width)).h(px(BAR)).flex_none();
        if let Some(i) = active {
            let x = self.slot_x(i, rtl, w, cx);
            let selection = self.selection.get_or_insert(Slide::new(x));
            selection.duration(x, 0.25, cx);
            track = track.child(
                div()
                    .id("dock-selection")
                    .debug_selector(|| "dock-selection".into())
                    .absolute()
                    .left(px(selection.get(w, cx)))
                    .top(px(PAD))
                    .size(px(SLOT))
                    .rounded(px(radius))
                    .bg(tint),
            );
        } else {
            self.selection = None;
        }
        let dragging = self.drag.filter(|d| d.moving);
        for i in (0..self.slots.len())
            .filter(|&i| !dragging.is_some_and(|d| d.slot == i))
            .chain(dragging.map(|d| d.slot))
        {
            let slot = &self.slots[i];
            let group = !slot.item.items.is_empty();
            let dragging = dragging.filter(|d| d.slot == i);
            let x = self.slot_x(i, rtl, w, cx) + dragging.map_or(0., |d| d.last_x);
            let badge = slot.badge_presence.get(w, cx).clamp(0., 1.);
            let button = self
                .button(
                    Target::Slot(i),
                    slot.item.id.clone(),
                    active == Some(i),
                    w,
                    cx,
                )
                .absolute()
                .left(px(x))
                .top(px(PAD - if dragging.is_some() { 4. } else { 0. }))
                .track_focus(&slot.focus)
                .tab_stop(self.roving == i)
                .when(group, |b| {
                    b.aria_expanded(self.open == Some(i)).child(
                        div().absolute().right(px(5.)).bottom(px(5.)).child(icon(
                            Glyph::Down,
                            9.,
                            p.faint,
                        )),
                    )
                })
                .when(!group, |b| b.aria_toggled((active == Some(i)).into()))
                .when(dragging.is_some(), |b| b.shadow_md().bg(p.panel))
                .when(badge > 0.01, |b| {
                    b.children(slot.badge.clone().map(|counter| {
                        row()
                            .absolute()
                            .right(px(2.))
                            .top(px(3.))
                            .min_w(px(16.))
                            .h(px(16.))
                            .justify_center()
                            .px(px(5.))
                            .rounded_full()
                            .bg(rgb(0xe86587))
                            .opacity(badge)
                            .child(counter)
                    }))
                })
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |s, e: &MouseDownEvent, _, cx| {
                        s.gesture_start(i, e.position, false, cx)
                    }),
                );
            track = track.child(button);
        }
        let mut root = div()
            .id("dock-toolbar")
            .debug_selector(|| "dock-toolbar".into())
            .role(Role::Toolbar)
            .block_mouse_except_scroll()
            .aria_label(
                self.label
                    .clone()
                    .unwrap_or_else(|| tr(cx, "Tools", "工具").into()),
            )
            .relative()
            .w(px(natural_width))
            .max_w_full()
            .h(px(BAR))
            .flex_none()
            .on_mouse_down_out(cx.listener(move |s, e: &MouseDownEvent, w, cx| {
                let b = s.bounds.get();
                let x = b.left() + px(tray_left);
                let inside_tray = e.position.x >= x
                    && e.position.x <= x + px(tray_width)
                    && e.position.y >= b.top() - px(tray_top)
                    && e.position.y < b.top();
                if !inside_tray {
                    s.close(false, w, cx);
                }
            }))
            .child(self.bounds.measure())
            .child(
                canvas(
                    |_, _, _| (),
                    move |b, _, w, _| {
                        draw_surface(w, b, tray_left, tray_width, tray_top, radius, p)
                    },
                )
                .absolute()
                .left_0()
                .right_0()
                .top(px(-TRAY))
                .h(px(TRAY + BAR)),
            )
            .child(
                div()
                    .id("dock-scroll")
                    .w_full()
                    .h(px(BAR))
                    .overflow_x_scroll()
                    .track_scroll(&self.scroll)
                    .child(track),
            );
        if let Some(i) = self.retained
            && tray_amount > 0.001
        {
            let slot = &self.slots[i];
            let open = self.open == Some(i);
            let time = self.tray_time.time(0.22, w, cx);
            let selected = slot.shown;
            let mut panel = row()
                .id("dock-tray")
                .debug_selector(|| "dock-tray".into())
                .role(Role::Group)
                .block_mouse_except_scroll()
                .aria_label(slot.item.label.clone())
                .absolute()
                .left(px(tray_left))
                .top(px(-tray_top))
                .w(px(tray_width))
                .h(px(tray_top))
                .overflow_hidden()
                .items_end()
                .pb(px(PAD))
                .px(px(PAD))
                .gap(px(4.))
                .when(rtl, |r| r.flex_row_reverse());
            for (j, member) in slot.item.items.iter().enumerate() {
                let alpha = if open {
                    ease((time - j as f32 * 0.016) / 0.14)
                } else {
                    tray_amount
                };
                if open {
                    panel = panel.child(
                        self.button(
                            Target::Member(i, j),
                            member.id.clone(),
                            selected == j,
                            w,
                            cx,
                        )
                        .track_focus(&slot.member_focus[j])
                        .tab_stop(false)
                        .aria_toggled((selected == j).into())
                        .opacity(alpha)
                        .when(selected == j, |b| b.bg(p.hover.opacity(0.65))),
                    );
                } else {
                    panel = panel.child(
                        row()
                            .size(px(MEMBER))
                            .flex_none()
                            .justify_center()
                            .opacity(alpha)
                            .child(member.icon.render(19., p.secondary)),
                    );
                }
            }
            root = root.child(panel);
        }
        let move_mouse =
            cx.listener(|s, e: &MouseMoveEvent, w, cx| s.gesture_move(e.position, false, w, cx));
        let up_mouse = cx.listener(|s, e: &MouseUpEvent, _, cx| {
            if e.button == MouseButton::Left {
                s.gesture_end(e.position, false, false, cx);
            }
        });
        let touch = cx.listener(|s, e: &TouchDragEvent, w, cx| match e.phase {
            TouchPhase::Started => {
                let b = s.bounds.get();
                if !b.contains(&e.start_position) {
                    return;
                }
                s.recently_touched.restart(cx);
                s.tip_delay = None;
                s.hover = None;
                s.tip_opacity.reset(0.);
                let x = f32::from(e.start_position.x - b.left() - s.scroll.offset().x) - PAD;
                if x < 0. || s.slots.is_empty() {
                    return;
                }
                let i = (x / STEP).floor() as usize;
                if i >= s.slots.len() || x % STEP >= SLOT {
                    return;
                }
                let i = if UiDirection::current(cx).is_rtl() {
                    s.slots.len() - 1 - i
                } else {
                    i
                };
                s.gesture_start(i, e.start_position, true, cx);
            }
            TouchPhase::Moved => s.gesture_move(e.position, true, w, cx),
            TouchPhase::Ended => s.gesture_end(e.position, true, false, cx),
            TouchPhase::Cancelled => s.gesture_end(e.position, true, true, cx),
        });
        root.children(self.tip(bar_width, tray_left, tray_top, rtl, w, cx))
            .child(
                canvas(
                    |_, _, _| (),
                    move |_, _, w, _| {
                        w.on_mouse_event(move |event: &MouseMoveEvent, phase, w, cx| {
                            if phase.bubble() {
                                move_mouse(event, w, cx);
                            }
                        });
                        w.on_mouse_event(move |event: &MouseUpEvent, phase, w, cx| {
                            if phase.bubble() {
                                up_mouse(event, w, cx);
                            }
                        });
                        w.on_mouse_event(move |event: &TouchDragEvent, phase, w, cx| {
                            if phase.bubble() {
                                touch(event, w, cx);
                            }
                        });
                    },
                )
                .absolute()
                .inset_0(),
            )
    }
}

fn draw_surface(
    w: &mut Window,
    b: Bounds<Pixels>,
    tray_x: f32,
    tray_w: f32,
    rise: f32,
    radius: f32,
    p: Paint,
) {
    let width = f32::from(b.size.width);
    let r = radius.min(width / 2.);
    let origin = b.origin + point(px(0.5), px(TRAY + 0.5));
    let width = (width - 1.).max(1.);
    let at = |x, y| origin + point(px(x), px(y));
    let path = |mut path: PathBuilder| {
        path.move_to(at(r, 0.));
        if rise > 0.1 && tray_w > 2. {
            let left = tray_x.clamp(r, (width - r - 2.).max(r));
            let right = (tray_x + tray_w).clamp(left + 1., (width - r).max(left + 1.));
            let r = radius.min(rise / 2.).min((right - left) / 4.);
            let shoulder = r.min(left - radius).min(width - radius - right).max(0.);
            path.line_to(at(left - shoulder, 0.));
            path.curve_to(at(left, -shoulder), at(left, 0.));
            path.line_to(at(left, -rise + r));
            path.curve_to(at(left + r, -rise), at(left, -rise));
            path.line_to(at(right - r, -rise));
            path.curve_to(at(right, -rise + r), at(right, -rise));
            path.line_to(at(right, -shoulder));
            path.curve_to(at(right + shoulder, 0.), at(right, 0.));
        }
        path.line_to(at(width - r, 0.));
        path.curve_to(at(width, r), at(width, 0.));
        path.line_to(at(width, BAR - 1. - r));
        path.curve_to(at(width - r, BAR - 1.), at(width, BAR - 1.));
        path.line_to(at(r, BAR - 1.));
        path.curve_to(at(0., BAR - 1. - r), at(0., BAR - 1.));
        path.line_to(at(0., r));
        path.curve_to(at(r, 0.), at(0., 0.));
        path.close();
        path.build()
    };
    if let Ok(fill) = path(PathBuilder::fill()) {
        w.paint_path(fill, p.panel.blend(p.stage.opacity(0.45)));
    }
    if let Ok(stroke) = path(PathBuilder::stroke(px(1.))) {
        w.paint_path(stroke, p.ink.opacity(0.12));
    }
}

fn draw_icon(icon: DockIcon, size: f32, origin: Point<Pixels>, color: Hsla, w: &mut Window) {
    let at = |x: f32, y: f32| origin + point(px(x * size / 24.), px(y * size / 24.));
    let mut path = PathBuilder::stroke(px(1.75 * size / 24.));
    let line = |path: &mut PathBuilder, points: &[(f32, f32)], closed| {
        if let Some((&(x, y), rest)) = points.split_first() {
            path.move_to(at(x, y));
            for &(x, y) in rest {
                path.line_to(at(x, y));
            }
            if closed {
                path.close();
            }
        }
    };
    match icon {
        DockIcon::Pointer => line(
            &mut path,
            &[(3., 3.), (20., 10.), (13., 13.), (10., 20.)],
            true,
        ),
        DockIcon::Hand => {
            path.move_to(at(7., 13.));
            for (x, top) in [(7., 6.), (10., 3.), (13., 4.), (16., 7.)] {
                path.line_to(at(x, top + 1.5));
                path.curve_to(at(x + 1.5, top), at(x, top));
                path.curve_to(at(x + 3., top + 1.5), at(x + 3., top));
                path.line_to(at(x + 3., 12.));
            }
            path.line_to(at(19., 15.));
            path.cubic_bezier_to(at(8., 20.), at(19., 22.), at(12., 23.));
            path.line_to(at(4., 16.));
            path.cubic_bezier_to(at(7., 13.), at(1., 13.), at(5., 11.));
            path.close();
        }
        DockIcon::Rectangle => {
            path.move_to(at(5., 3.));
            path.line_to(at(19., 3.));
            path.curve_to(at(21., 5.), at(21., 3.));
            path.line_to(at(21., 19.));
            path.curve_to(at(19., 21.), at(21., 21.));
            path.line_to(at(5., 21.));
            path.curve_to(at(3., 19.), at(3., 21.));
            path.line_to(at(3., 5.));
            path.curve_to(at(5., 3.), at(3., 3.));
            path.close();
        }
        DockIcon::Ellipse => {
            path.move_to(at(21., 12.));
            let radius = point(px(size * 0.375), px(size * 0.375));
            path.arc_to(radius, px(0.), false, true, at(3., 12.));
            path.arc_to(radius, px(0.), false, true, at(21., 12.));
            path.close();
        }
        DockIcon::Triangle => line(&mut path, &[(12., 3.), (21.5, 20.), (2.5, 20.)], true),
        DockIcon::Star => {
            let points = std::array::from_fn::<_, 10, _>(|i| {
                let a = std::f32::consts::PI * (i as f32 / 5. - 0.5);
                let r = if i % 2 == 0 { 10. } else { 4.4 };
                (12. + r * a.cos(), 12. + r * a.sin())
            });
            line(&mut path, &points, true);
        }
        DockIcon::Text => {
            line(
                &mut path,
                &[(5., 7.), (5., 4.), (19., 4.), (19., 7.)],
                false,
            );
            line(&mut path, &[(12., 4.), (12., 20.)], false);
            line(&mut path, &[(8., 20.), (16., 20.)], false);
        }
        DockIcon::Note => {
            line(
                &mut path,
                &[(3., 4.), (14., 4.), (21., 11.), (21., 20.), (3., 20.)],
                true,
            );
            line(&mut path, &[(14., 4.), (14., 11.), (21., 11.)], false);
        }
        DockIcon::Glyph(_) => return,
    }
    if let Ok(path) = path.build() {
        w.paint_path(path, color);
    }
}

#[cfg(test)]
mod tests {
    use super::{Dock, DockIcon, DockItem, DockReordered, DockSelected, Target};
    use gpui::{
        Context, Entity, IntoElement, Modifiers, Render, TestAppContext, VisualTestContext, Window,
        div, point, prelude::*, px, size,
    };
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
        time::Duration,
    };

    fn items() -> Vec<DockItem> {
        vec![
            DockItem::new("move", "Move", DockIcon::Pointer).with_shortcut("V"),
            DockItem::new("hand", "Hand", DockIcon::Hand).with_shortcut("H"),
            DockItem::new("shapes", "Shapes", DockIcon::Rectangle).with_items(vec![
                DockItem::new("rectangle", "Rectangle", DockIcon::Rectangle).with_shortcut("R"),
                DockItem::new("ellipse", "Ellipse", DockIcon::Ellipse).with_shortcut("O"),
                DockItem::new("triangle", "Triangle", DockIcon::Triangle),
                DockItem::new("star", "Star", DockIcon::Star),
            ]),
            DockItem::new("text", "Text", DockIcon::Text),
            DockItem::new("note", "Note", DockIcon::Note),
            DockItem::new("comment", "Comment", DockIcon::Note).with_badge(2),
        ]
    }

    fn tick(w: &mut VisualTestContext, ms: u64) {
        w.executor().advance_clock(Duration::from_millis(ms));
        w.run_until_parked();
        w.update(|w, cx| {
            w.simulate_next_frame(cx);
            w.refresh();
        });
        w.run_until_parked();
    }

    #[test]
    fn keyboard_uses_the_group_tray_and_restores_focus_after_host_selection() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) = app.add_window_view(|_, cx| Dock::new(items(), cx).with_value("move", cx));
        let _sub = w.update(|_, cx| {
            cx.subscribe(&view, |view, event: &DockSelected, cx| {
                view.update(cx, |dock, cx| dock.set_value(Some(event.0.clone()), cx));
            })
        });
        w.update(|w, cx| view.update(cx, |dock, cx| dock.focus_slot(0, w, cx)));
        w.simulate_keystrokes("right right up");
        tick(w, 700);
        let toolbar = w.debug_bounds("dock-toolbar").unwrap();
        let tray = w.debug_bounds("dock-tray").unwrap();
        assert_eq!(toolbar.size, size(px(272.), px(52.)));
        assert_eq!(tray.size, size(px(168.), px(48.)));
        assert_eq!(tray.bottom(), toolbar.top());
        w.update(|w, cx| assert!(view.read(cx).slots[2].member_focus[0].is_focused(w)));
        w.simulate_keystrokes("right enter");
        w.update(|w, cx| {
            let dock = view.read(cx);
            assert_eq!(dock.value.as_deref(), Some("ellipse"));
            assert_eq!(dock.slots[2].visible_item().id.as_ref(), "ellipse");
            assert_eq!(dock.open, None);
            assert!(dock.slots[2].focus.is_focused(w));
        });
        w.simulate_keystrokes("up escape");
        w.update(|w, cx| assert!(view.read(cx).slots[2].focus.is_focused(w)));
    }

    #[test]
    fn keyboard_reorder_retains_the_moved_tools_identity_and_focus() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) = app.add_window_view(|_, cx| {
            Dock::new(items(), cx)
                .with_reorderable(true)
                .with_value("move", cx)
        });
        let _sub = w.update(|_, cx| {
            cx.subscribe(&view, |view, event: &DockReordered, cx| {
                assert_eq!(event.id.as_ref(), "move");
                assert_eq!(event.index, 1);
                view.update(cx, |dock, cx| dock.set_items(event.items.clone(), cx));
            })
        });
        w.update(|w, cx| view.update(cx, |dock, cx| dock.focus_slot(0, w, cx)));
        w.simulate_keystrokes("alt-right");
        w.update(|w, cx| {
            let dock = view.read(cx);
            assert_eq!(dock.roving, 1);
            assert_eq!(dock.slots[1].item.id.as_ref(), "move");
            assert_eq!(dock.value.as_deref(), Some("move"));
            assert!(dock.slots[1].focus.is_focused(w));
        });
    }

    #[test]
    fn horizontal_drag_requests_reorder_without_selecting_and_vertical_touch_does_not_reorder() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) = app.add_window_view(|_, cx| Dock::new(items(), cx).with_reorderable(true));
        let reordered = Rc::new(RefCell::new(Vec::<DockReordered>::new()));
        let selected = Rc::new(RefCell::new(Vec::<DockSelected>::new()));
        let _subscriptions = w.update(|_, cx| {
            let reordered = reordered.clone();
            let selected = selected.clone();
            [
                cx.subscribe(&view, move |_, event: &DockReordered, _| {
                    reordered.borrow_mut().push(event.clone())
                }),
                cx.subscribe(&view, move |_, event: &DockSelected, _| {
                    selected.borrow_mut().push(event.clone())
                }),
            ]
        });
        let origin = w.debug_bounds("Move, V").unwrap().center();
        w.update(|w, cx| {
            view.update(cx, |dock, cx| {
                dock.gesture_start(0, origin, false, cx);
                let end = origin + point(px(89.), px(2.));
                dock.gesture_move(end, false, w, cx);
                dock.gesture_end(end, false, false, cx);
                dock.choose(Target::Slot(0), w, cx);
            })
        });
        assert_eq!(reordered.borrow()[0].index, 2);
        assert!(selected.borrow().is_empty());
        w.update(|w, cx| {
            view.update(cx, |dock, cx| {
                dock.gesture_start(0, origin, true, cx);
                let end = origin + point(px(4.), px(38.));
                dock.gesture_move(end, true, w, cx);
                dock.gesture_end(end, true, false, cx);
            })
        });
        assert_eq!(reordered.borrow().len(), 1);
    }

    #[test]
    fn only_the_host_can_change_the_selection_and_rejected_order_is_retained() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) = app.add_window_view(|_, cx| {
            Dock::new(items(), cx)
                .with_reorderable(true)
                .with_value("move", cx)
        });
        w.update(|w, cx| {
            view.update(cx, |dock, cx| {
                dock.choose(Target::Slot(1), w, cx);
                dock.reorder(0, 2, cx);
                assert_eq!(dock.value.as_deref(), Some("move"));
                assert_eq!(dock.slots[0].item.id.as_ref(), "move");
            })
        });
    }

    #[test]
    fn toolbar_and_tray_do_not_activate_the_underlying_canvas() {
        struct CanvasWithDock {
            dock: Entity<Dock>,
            clicks: Rc<Cell<usize>>,
        }

        impl Render for CanvasWithDock {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                let clicks = self.clicks.clone();
                div()
                    .relative()
                    .size(px(400.))
                    .child(
                        div()
                            .id("canvas")
                            .size_full()
                            .on_click(move |_, _, _| clicks.set(clicks.get() + 1)),
                    )
                    .child(
                        div()
                            .absolute()
                            .left(px(60.))
                            .top(px(280.))
                            .child(self.dock.clone()),
                    )
            }
        }

        let mut app = TestAppContext::single();
        app.update(crate::init);
        let clicks = Rc::new(Cell::new(0));
        let (_, w) = app.add_window_view(|_, cx| CanvasWithDock {
            dock: cx.new(|cx| Dock::new(items(), cx)),
            clicks: clicks.clone(),
        });
        let shapes = w.debug_bounds("Shapes: Rectangle, R").unwrap().center();
        w.simulate_click(shapes, Modifiers::default());
        tick(w, 700);
        let ellipse = w.debug_bounds("Ellipse, O").unwrap().center();
        w.simulate_click(ellipse, Modifiers::default());
        assert_eq!(clicks.get(), 0);
        w.simulate_click(point(px(100.), px(100.)), Modifiers::default());
        assert_eq!(clicks.get(), 1);
    }
}
