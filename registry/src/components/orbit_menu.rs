//! A radial menu with press-drag selection and keyboard access.
use crate::{
    foundation::{Theme, ThemeMode},
    primitives::{
        direct_touch::Contact,
        family::{PhaseClock, Slide, disabled_motion, ease, measure},
        flyout::TriggerBounds,
        glyph::{Glyph, drawing, icon},
        presentation::{row, tr},
    },
};
use gpui::{prelude::*, *};
use std::time::Duration;

const TRIGGER: f32 = 56.;
const BUTTON: f32 = 46.;
const DEFAULT_RADIUS: f32 = 104.;
const DEAD_ZONE: f32 = 34.;
const REACH_MARGIN: f32 = 60.;
const TAP_LIMIT: f32 = 0.34;
const DONE_DURATION: Duration = Duration::from_millis(1400);

/// An action mark. SVG assets use the current foreground ink.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OrbitIcon {
    Glyph(Glyph),
    Album,
    Share,
    Svg(SharedString),
}

impl From<Glyph> for OrbitIcon {
    fn from(value: Glyph) -> Self {
        Self::Glyph(value)
    }
}

/// The semantic tone of an action.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OrbitTone {
    #[default]
    Neutral,
    Danger,
}

/// One action with a stable ID, visible label and optional confirmation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrbitAction {
    pub id: SharedString,
    pub label: SharedString,
    pub icon: OrbitIcon,
    pub done: Option<SharedString>,
    pub tone: OrbitTone,
}

impl OrbitAction {
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        icon: impl Into<OrbitIcon>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: icon.into(),
            done: None,
            tone: OrbitTone::Neutral,
        }
    }

    pub fn with_done(mut self, label: impl Into<SharedString>) -> Self {
        self.done = Some(label.into());
        self
    }

    pub fn with_tone(mut self, tone: OrbitTone) -> Self {
        self.tone = tone;
        self
    }
}

/// Identifies the selected action. The host performs its effect.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrbitInvoked(pub SharedString);

/// Reports the change in the arc's interactive state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OrbitOpenChanged(pub bool);

struct Slot {
    action: OrbitAction,
    focus: FocusHandle,
    reveal: Slide,
    emphasis: Slide,
    magnet_x: Slide,
    magnet_y: Slide,
    drawn: (f32, f32, f32),
}

impl Slot {
    fn new(action: OrbitAction, cx: &mut Context<OrbitMenu>) -> Self {
        Self {
            action,
            focus: cx.focus_handle().tab_stop(true),
            reveal: Slide::new(0.),
            emphasis: Slide::new(0.),
            magnet_x: Slide::new(0.),
            magnet_y: Slide::new(0.),
            drawn: (0., 0., 0.),
        }
    }
}

#[derive(Clone, Copy)]
struct Press {
    was_open: bool,
    clock: PhaseClock,
    origin: Point<Pixels>,
    moved: bool,
    touch: bool,
}

/// A circular trigger that opens a short arc of actions.
pub struct OrbitMenu {
    id: SharedString,
    slots: Vec<Slot>,
    label: Option<SharedString>,
    radius: f32,
    spread: f32,
    direction: f32,
    disabled: bool,
    announce: bool,
    open: bool,
    press: Option<Press>,
    pointer: Option<Point<Pixels>>,
    target: Option<usize>,
    roving: Option<usize>,
    bounds: TriggerBounds,
    contact: Contact,
    focus_group: FocusHandle,
    trigger: FocusHandle,
    _focus_out: Subscription,
    _window_activation: Subscription,
    done: Option<SharedString>,
    done_timer: Option<Task<()>>,
    arc: Slide,
    scale: Slide,
    lens_x: Slide,
    lens_y: Slide,
    lens_alpha: Slide,
    lens_seeded: bool,
    glyph: Glyph,
    previous_glyph: Glyph,
    glyph_clock: PhaseClock,
    pill_text: SharedString,
    pill_before: SharedString,
    pill_clock: PhaseClock,
    pill_width: Option<Slide>,
    pill_alpha: Slide,
}

impl EventEmitter<OrbitInvoked> for OrbitMenu {}
impl EventEmitter<OrbitOpenChanged> for OrbitMenu {}

impl OrbitMenu {
    pub fn new(actions: Vec<OrbitAction>, w: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_group = cx.focus_handle();
        let trigger = cx.focus_handle().tab_stop(true);
        let focus_out = cx.on_focus_out(&focus_group, w, |s: &mut Self, _, _, cx| s.close(cx));
        let activation = cx.observe_window_activation(w, |s: &mut Self, w, cx| {
            if !w.is_window_active() {
                s.close(cx);
            }
        });
        Self {
            id: format!("orbit-menu-{}", cx.entity_id()).into(),
            slots: actions.into_iter().map(|a| Slot::new(a, cx)).collect(),
            label: None,
            radius: DEFAULT_RADIUS,
            spread: 150.,
            direction: -90.,
            disabled: false,
            announce: true,
            open: false,
            press: None,
            pointer: None,
            target: None,
            roving: None,
            bounds: TriggerBounds::default(),
            contact: Contact::default(),
            focus_group,
            trigger,
            _focus_out: focus_out,
            _window_activation: activation,
            done: None,
            done_timer: None,
            arc: Slide::new(0.),
            scale: Slide::new(1.),
            lens_x: Slide::new(0.),
            lens_y: Slide::new(0.),
            lens_alpha: Slide::new(0.),
            lens_seeded: false,
            glyph: Glyph::More,
            previous_glyph: Glyph::More,
            glyph_clock: PhaseClock::default(),
            pill_text: "".into(),
            pill_before: "".into(),
            pill_clock: PhaseClock::default(),
            pill_width: None,
            pill_alpha: Slide::new(0.),
        }
    }

    pub fn with_id(mut self, id: impl Into<SharedString>) -> Self {
        self.id = id.into();
        self
    }

    pub fn with_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn with_radius(mut self, radius: f32) -> Self {
        if radius.is_finite() {
            self.radius = radius.max(1.);
        }
        self
    }

    pub fn with_spread(mut self, degrees: f32) -> Self {
        if degrees.is_finite() {
            self.spread = degrees.clamp(0., 360.);
        }
        self
    }

    pub fn with_direction(mut self, degrees: f32) -> Self {
        if degrees.is_finite() {
            self.direction = degrees.rem_euclid(360.);
        }
        self
    }

    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn with_announce(mut self, announce: bool) -> Self {
        self.announce = announce;
        self
    }

    /// Replace actions, retaining focus and animation by action ID.
    pub fn set_actions(&mut self, actions: Vec<OrbitAction>, cx: &mut Context<Self>) {
        let targeted = self.target.map(|i| self.slots[i].action.id.clone());
        let focused = self.roving.map(|i| self.slots[i].action.id.clone());
        let mut old = std::mem::take(&mut self.slots);
        self.slots = actions
            .into_iter()
            .map(|action| {
                if let Some(index) = old.iter().position(|s| s.action.id == action.id) {
                    let mut slot = old.remove(index);
                    slot.action = action;
                    slot
                } else {
                    let mut slot = Slot::new(action, cx);
                    if self.open {
                        slot.reveal.duration(1., 0.42, cx);
                    }
                    slot
                }
            })
            .collect();
        self.target = targeted.and_then(|id| self.slots.iter().position(|s| s.action.id == id));
        self.roving = focused.and_then(|id| self.slots.iter().position(|s| s.action.id == id));
        if self.slots.is_empty() {
            self.close(cx);
        }
        cx.notify();
    }

    pub fn set_disabled(&mut self, disabled: bool, cx: &mut Context<Self>) {
        if self.disabled != disabled {
            self.disabled = disabled;
            if disabled {
                self.close(cx);
            }
            cx.notify();
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Restore keyboard focus to the trigger after a host action.
    pub fn focus_trigger(&self, w: &mut Window, cx: &mut Context<Self>) {
        self.trigger.focus(w, cx);
    }

    /// Clear the confirmation and close the menu without firing an action.
    pub fn reset(&mut self, cx: &mut Context<Self>) {
        self.done_timer = None;
        self.done = None;
        self.close(cx);
        self.change_glyph(Glyph::More, cx);
        cx.notify();
    }

    fn enabled(&self) -> bool {
        !self.disabled && !self.slots.is_empty()
    }

    fn change_glyph(&mut self, glyph: Glyph, cx: &App) {
        if self.glyph != glyph {
            self.previous_glyph = self.glyph;
            self.glyph = glyph;
            self.glyph_clock.restart(cx);
        }
    }

    fn set_open(&mut self, open: bool, cx: &mut Context<Self>) {
        if self.open == open {
            return;
        }
        self.open = open;
        let count = self.slots.len();
        for (i, slot) in self.slots.iter_mut().enumerate() {
            let stagger = if open {
                i as f32 * 0.022
            } else {
                (count - 1 - i) as f32 * 0.016
            };
            slot.reveal
                .delayed(f32::from(open), if open { 0.42 } else { 0.24 }, stagger, cx);
        }
        self.arc
            .duration(f32::from(open), if open { 0.38 } else { 0.20 }, cx);
        self.scale.duration(if open { 0.94 } else { 1. }, 0.26, cx);
        self.change_glyph(if open { Glyph::Close } else { Glyph::More }, cx);
        cx.emit(OrbitOpenChanged(open));
        cx.notify();
    }

    fn close(&mut self, cx: &mut Context<Self>) {
        self.press = None;
        self.pointer = None;
        self.target = None;
        self.roving = None;
        self.set_open(false, cx);
    }

    fn open(&mut self, cx: &mut Context<Self>) {
        if !self.enabled() {
            return;
        }
        self.done_timer = None;
        self.done = None;
        self.set_open(true, cx);
    }

    fn fire(&mut self, i: usize, w: &mut Window, cx: &mut Context<Self>) {
        if !self.open || !self.enabled() {
            return;
        }
        let action = self.slots[i].action.clone();
        self.close(cx);
        self.done = Some(action.done.unwrap_or(action.label));
        self.change_glyph(Glyph::Check, cx);
        self.trigger.focus(w, cx);
        cx.emit(OrbitInvoked(action.id));
        self.done_timer = Some(cx.spawn(async move |view, cx| {
            cx.background_executor().timer(DONE_DURATION).await;
            let _ = view.update(cx, |s, cx| {
                s.done = None;
                s.change_glyph(Glyph::More, cx);
                cx.notify();
            });
        }));
        cx.notify();
    }

    fn begin(&mut self, point: Point<Pixels>, touch: bool, w: &mut Window, cx: &mut Context<Self>) {
        if !self.enabled() || self.press.is_some() || !self.hit_trigger(point) {
            return;
        }
        let was_open = self.open;
        let mut clock = PhaseClock::default();
        clock.restart(cx);
        self.press = Some(Press {
            was_open,
            origin: point,
            touch,
            moved: false,
            clock,
        });
        self.roving = None;
        self.target = None;
        self.pointer = None;
        self.trigger.focus(w, cx);
        self.open(cx);
        cx.notify();
    }

    fn moved(&mut self, point: Point<Pixels>, touch: bool, cx: &mut Context<Self>) {
        let Some(mut press) = self.press.filter(|p| p.touch == touch) else {
            return;
        };
        let delta = point - press.origin;
        press.moved |= f32::from(delta.x).hypot(f32::from(delta.y)) > 5.;
        self.press = Some(press);
        self.pointer = (!touch).then_some(point);
        self.target = press.moved.then(|| self.pick(point)).flatten();
        cx.notify();
    }

    fn release(
        &mut self,
        point: Point<Pixels>,
        touch: bool,
        cancelled: bool,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(press) = self.press.filter(|p| p.touch == touch) else {
            return;
        };
        if cancelled {
            self.close(cx);
            return;
        }
        self.moved(point, touch, cx);
        let press = self.press.take().unwrap_or(press);
        if press.moved
            && let Some(i) = self.target
        {
            self.fire(i, w, cx);
        } else if press.moved || press.was_open || press.clock.age(cx) >= TAP_LIMIT {
            self.close(cx);
        } else {
            self.target = None;
            self.pointer = None;
            cx.notify();
        }
    }

    fn pick(&self, point: Point<Pixels>) -> Option<usize> {
        let delta = point - self.bounds.get().center();
        let (x, y) = (f32::from(delta.x), f32::from(delta.y));
        let distance = x.hypot(y);
        if self.slots.is_empty() || !(DEAD_ZONE..=self.radius + REACH_MARGIN).contains(&distance) {
            return None;
        }
        let angle = y.atan2(x).to_degrees();
        let offset = angle_delta(angle, self.direction);
        let step = self.spread / self.slots.len().saturating_sub(1).max(1) as f32;
        let tolerance = (step / 2.).clamp(12., 18.);
        if offset.abs() > self.spread / 2. + tolerance {
            return None;
        }
        (0..self.slots.len()).min_by(|&a, &b| {
            angle_delta(
                angle,
                action_angle(a, self.slots.len(), self.direction, self.spread),
            )
            .abs()
            .total_cmp(
                &angle_delta(
                    angle,
                    action_angle(b, self.slots.len(), self.direction, self.spread),
                )
                .abs(),
            )
        })
    }

    fn direct_hit(&self, point: Point<Pixels>) -> Option<usize> {
        let delta = point - self.bounds.get().center();
        let (x, y) = (f32::from(delta.x), f32::from(delta.y));
        self.slots.iter().position(|s| {
            let (cx, cy, side) = s.drawn;
            side > 0. && (cx - x).hypot(cy - y) <= side / 2.
        })
    }

    fn hit_trigger(&self, point: Point<Pixels>) -> bool {
        let delta = point - self.bounds.get().center();
        f32::from(delta.x).hypot(f32::from(delta.y)) <= TRIGGER / 2.
    }

    fn focus_action(&mut self, index: usize, w: &mut Window, cx: &mut Context<Self>) {
        self.open(cx);
        if !self.open {
            return;
        }
        self.press = None;
        self.pointer = None;
        self.roving = Some(index);
        self.target = Some(index);
        self.slots[index].focus.focus(w, cx);
        cx.notify();
    }

    fn trigger_key(&mut self, event: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        if !self.enabled() {
            return;
        }
        let i = match event.keystroke.key.as_str() {
            "up" => self.slots.len() - 1,
            "down" | "right" | "left" => 0,
            "enter" | "space" if !event.is_held => {
                if self.open {
                    self.close(cx);
                } else {
                    self.focus_action(0, w, cx);
                }
                w.prevent_default();
                cx.stop_propagation();
                return;
            }
            "tab" if self.open => {
                self.advance_tab(event.keystroke.modifiers.shift, w, cx);
                w.prevent_default();
                cx.stop_propagation();
                return;
            }
            "escape" if self.open => {
                self.close(cx);
                self.trigger.focus(w, cx);
                cx.stop_propagation();
                return;
            }
            _ => return,
        };
        self.focus_action(i, w, cx);
        w.prevent_default();
        cx.stop_propagation();
    }

    fn action_key(
        &mut self,
        i: usize,
        event: &KeyDownEvent,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.open {
            return;
        }
        let n = self.slots.len();
        let target = match event.keystroke.key.as_str() {
            "up" | "left" => Some((i + n - 1) % n),
            "down" | "right" => Some((i + 1) % n),
            "home" => Some(0),
            "end" => Some(n - 1),
            "enter" | "space" if !event.is_held => {
                self.fire(i, w, cx);
                None
            }
            "escape" => {
                self.close(cx);
                self.trigger.focus(w, cx);
                None
            }
            "tab" => {
                self.advance_tab(event.keystroke.modifiers.shift, w, cx);
                None
            }
            _ => return,
        };
        if let Some(index) = target {
            self.focus_action(index, w, cx);
        }
        w.prevent_default();
        cx.stop_propagation();
    }

    fn advance_tab(&mut self, back: bool, w: &mut Window, cx: &mut Context<Self>) {
        self.close(cx);
        self.trigger.focus(w, cx);
        w.on_next_frame(move |w, cx| {
            if back {
                w.focus_prev(cx);
            } else {
                w.focus_next(cx);
            }
        });
    }
}

fn angle_delta(a: f32, b: f32) -> f32 {
    (a - b + 180.).rem_euclid(360.) - 180.
}

fn action_angle(i: usize, n: usize, direction: f32, spread: f32) -> f32 {
    if n <= 1 {
        direction
    } else {
        direction + spread * (i as f32 / (n - 1) as f32 - 0.5)
    }
}

fn offset(radius: f32, angle: f32) -> (f32, f32) {
    let (s, c) = angle.to_radians().sin_cos();
    (c * radius, s * radius)
}

struct Material {
    text: Hsla,
    inverse: Hsla,
    solid: Hsla,
    glass: Hsla,
    action: Hsla,
    line: Hsla,
    track: Hsla,
    danger: Hsla,
    shadow: Vec<BoxShadow>,
}

impl Material {
    fn get(cx: &App) -> Self {
        let dark = Theme::global(cx).mode == ThemeMode::Dark;
        let (text, inverse, surface, line, track, danger) = if dark {
            (0xf2f2f2, 0x141414, 0x1f1f1f, 0x484848, 0x484848, 0xf0738f)
        } else {
            (0x0b0b0b, 0xffffff, 0xffffff, 0xd6d6d6, 0xc4c4c4, 0xbb2c54)
        };
        let shadow = if dark {
            vec![shadow(0.22, 8., 22.)]
        } else {
            vec![shadow(0.065, 6., 18.), shadow(0.035, 1., 3.)]
        };
        Self {
            text: rgb(text).into(),
            inverse: rgb(inverse).into(),
            solid: rgb(text).into(),
            glass: rgb(surface).opacity(0.76).into(),
            action: rgb(surface).opacity(0.94).into(),
            line: rgb(line).opacity(0.55).into(),
            track: rgb(track).into(),
            danger: rgb(danger).into(),
            shadow,
        }
    }
}

fn shadow(alpha: f32, y: f32, blur: f32) -> BoxShadow {
    BoxShadow {
        color: black().opacity(alpha),
        offset: point(px(0.), px(y)),
        blur_radius: px(blur),
        spread_radius: px(0.),
        inset: false,
    }
}

fn action_mark(value: OrbitIcon, size: f32, ink: Hsla) -> AnyElement {
    match value {
        OrbitIcon::Glyph(glyph) => icon(glyph, size, ink).into_any_element(),
        OrbitIcon::Svg(path) => svg()
            .path(path)
            .size(px(size))
            .flex_none()
            .text_color(ink)
            .into_any_element(),
        OrbitIcon::Album | OrbitIcon::Share => canvas(
            |_, _, _| (),
            move |b, _, w, _| {
                let at = |x, y| b.origin + point(px(x * size / 24.), px(y * size / 24.));
                let mut p = PathBuilder::stroke(px(size * 1.75 / 24.));
                if value == OrbitIcon::Album {
                    p.move_to(at(3., 6.));
                    p.curve_to(at(5., 4.), at(3., 4.));
                    p.line_to(at(9., 4.));
                    p.line_to(at(12., 7.));
                    p.line_to(at(19., 7.));
                    p.curve_to(at(21., 9.), at(21., 7.));
                    p.line_to(at(21., 18.));
                    p.curve_to(at(19., 20.), at(21., 20.));
                    p.line_to(at(5., 20.));
                    p.curve_to(at(3., 18.), at(3., 20.));
                    p.close();
                    p.move_to(at(12., 10.));
                    p.line_to(at(12., 17.));
                    p.move_to(at(8.5, 13.5));
                    p.line_to(at(15.5, 13.5));
                } else {
                    p.move_to(at(8., 10.5));
                    p.line_to(at(15.3, 6.3));
                    p.move_to(at(8., 13.5));
                    p.line_to(at(15.3, 17.7));
                    for (x, y) in [(5., 12.), (18., 5.), (18., 19.)] {
                        let r = point(px(size / 8.), px(size / 8.));
                        p.move_to(at(x + 3., y));
                        p.arc_to(r, px(0.), false, true, at(x - 3., y));
                        p.arc_to(r, px(0.), false, true, at(x + 3., y));
                        p.close();
                    }
                }
                if let Ok(path) = p.build() {
                    w.paint_path(path, ink);
                }
            },
        )
        .size(px(size))
        .flex_none()
        .into_any_element(),
    }
}

impl Render for OrbitMenu {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let m = Material::get(cx);
        let reduced = disabled_motion(cx);
        let label = self
            .label
            .clone()
            .unwrap_or_else(|| tr(cx, "Actions", "操作").into());
        let arc = self.arc.get(w, cx).clamp(0., 1.);
        let size = TRIGGER * self.scale.get(w, cx);
        let radius = self.radius;
        let direction = self.direction;
        let spread = self.spread;
        let count = self.slots.len();
        let moving = self.open || arc > 0.001;
        let pointer = self
            .pointer
            .filter(|_| self.open && !reduced)
            .map(|point| point - self.bounds.get().center());
        let frames: Vec<_> = self
            .slots
            .iter_mut()
            .enumerate()
            .map(|(i, slot)| {
                let p = slot.reveal.get(w, cx).clamp(0., 1.);
                let angle = action_angle(i, count, direction, spread);
                let (x, y) = offset(
                    radius * p,
                    angle + if reduced { 0. } else { 44. * (1. - p) },
                );
                let (mx, my) = pointer.map_or((0., 0.), |point| {
                    let (dx, dy) = (f32::from(point.x) - x, f32::from(point.y) - y);
                    let pull = 0.16 * (1. - dx.hypot(dy) / 78.).clamp(0., 1.);
                    (dx * pull, dy * pull)
                });
                slot.magnet_x.duration(mx, 0.19, cx);
                slot.magnet_y.duration(my, 0.19, cx);
                let x = x + slot.magnet_x.get(w, cx);
                let y = y + slot.magnet_y.get(w, cx);
                slot.emphasis
                    .duration(f32::from(self.open && self.target == Some(i)), 0.26, cx);
                let high = slot.emphasis.get(w, cx).clamp(0., 1.);
                let side = BUTTON * (0.4 + p * 0.6) * (1. + high * 0.10);
                slot.drawn = (x, y, side * f32::from(self.open && p > 0.48));
                (x, y, side, p, high)
            })
            .collect();
        let highlighted = self
            .target
            .filter(|_| self.open)
            .and_then(|i| frames.get(i).map(|frame| (i, *frame)));
        if let Some((_, (x, y, _, _, _))) = highlighted {
            if !self.lens_seeded {
                self.lens_x.reset(x);
                self.lens_y.reset(y);
                self.lens_seeded = true;
            }
            self.lens_x.duration(x, 0.24, cx);
            self.lens_y.duration(y, 0.24, cx);
        }
        self.lens_alpha
            .duration(f32::from(highlighted.is_some()), 0.22, cx);
        let lens_alpha = self.lens_alpha.get(w, cx).clamp(0., 1.);
        if lens_alpha < 0.001 && highlighted.is_none() {
            self.lens_seeded = false;
        }
        let lens_at = (self.lens_x.get(w, cx), self.lens_y.get(w, cx));
        let lens_side = highlighted
            .map_or(BUTTON * 1.1, |(_, (_, _, side, _, _))| side.max(BUTTON))
            * (0.86 + 0.14 * lens_alpha);
        let lens_ink =
            if highlighted.is_some_and(|(i, _)| self.slots[i].action.tone == OrbitTone::Danger) {
                m.solid.blend(m.danger.opacity(0.14))
            } else {
                m.solid
            };

        let mut root = div()
            .id(self.id.clone())
            .relative()
            .size(px(TRIGGER))
            .flex_none()
            .track_focus(&self.focus_group)
            .tab_stop(false)
            .child(self.bounds.measure());
        if moving {
            let line = m.track.opacity(arc * 0.88);
            root = root.child(
                canvas(
                    |_, _, _| (),
                    move |b, _, w, _| {
                        let mut path = PathBuilder::stroke(px(1.5));
                        for i in 0..=32 {
                            let a = direction + spread * (i as f32 / 32. - 0.5) * arc;
                            let (x, y) = offset(radius * arc, a);
                            let p = b.center() + point(px(x), px(y));
                            if i == 0 {
                                path.move_to(p);
                            } else {
                                path.line_to(p);
                            }
                        }
                        if let Ok(path) = path.build() {
                            w.paint_path(path, line);
                        }
                    },
                )
                .absolute()
                .inset_0(),
            );
        }
        if lens_alpha > 0.001 {
            root = root.child(
                div()
                    .id("orbit-lens")
                    .debug_selector(|| "orbit-lens".into())
                    .absolute()
                    .left(px(TRIGGER / 2. + lens_at.0 - lens_side / 2.))
                    .top(px(TRIGGER / 2. + lens_at.1 - lens_side / 2.))
                    .size(px(lens_side))
                    .rounded_full()
                    .bg(lens_ink)
                    .shadow(m.shadow.clone())
                    .opacity(lens_alpha),
            );
        }
        if moving {
            let mut menu = div()
                .id("orbit-actions")
                .when(self.open, |menu| {
                    menu.role(Role::Menu).aria_label(label.clone())
                })
                .absolute()
                .inset_0();
            for (i, ((x, y, side, p, high), slot)) in
                frames.into_iter().zip(&self.slots).enumerate()
            {
                let fore = if high >= 0.52 {
                    m.inverse
                } else if slot.action.tone == OrbitTone::Danger {
                    m.danger
                } else {
                    m.text
                };
                let mut item = row()
                    .id(slot.action.id.clone())
                    .debug_selector({
                        let name = slot.action.label.clone();
                        move || name.to_string()
                    })
                    .absolute()
                    .left(px(TRIGGER / 2. + x - side / 2.))
                    .top(px(TRIGGER / 2. + y - side / 2.))
                    .size(px(side))
                    .justify_center()
                    .rounded_full()
                    .opacity(p)
                    .when(high < 0.99, |b| {
                        b.bg(m.action.opacity(1. - high))
                            .border_1()
                            .border_color(m.line.opacity(1. - high))
                            .shadow(m.shadow.clone())
                    })
                    .child(action_mark(
                        slot.action.icon.clone(),
                        18. * side / BUTTON,
                        fore,
                    ));
                if self.open {
                    item =
                        item.role(Role::MenuItem)
                            .aria_label(slot.action.label.clone())
                            .track_focus(&slot.focus)
                            .tab_stop(self.roving == Some(i) || (self.roving.is_none() && i == 0))
                            .block_mouse_except_scroll()
                            .cursor_pointer()
                            .on_click(cx.listener(move |s, e: &ClickEvent, w, cx| {
                                if e.is_keyboard() || s.direct_hit(e.position()) == Some(i) {
                                    s.fire(i, w, cx);
                                }
                            }))
                            .on_key_down(cx.listener(move |s, e: &KeyDownEvent, w, cx| {
                                s.action_key(i, e, w, cx)
                            }));
                }
                menu = menu.child(item);
            }
            root = root.child(menu);
        }
        let trigger_fore = if self.open { m.text } else { m.inverse };
        let glyph_t = ease(self.glyph_clock.time(0.18, w, cx) / 0.18);
        let trigger = row()
            .id("orbit-trigger")
            .debug_selector(|| "orbit-trigger".into())
            .relative()
            .size(px(TRIGGER))
            .justify_center()
            .rounded_full()
            .role(Role::Button)
            .aria_label(label.clone())
            .aria_expanded(self.open)
            .track_focus(&self.trigger)
            .tab_stop(self.enabled())
            .block_mouse_except_scroll()
            .focus_visible(move |s| {
                s.shadow(vec![BoxShadow {
                    color: m.track,
                    offset: point(px(0.), px(0.)),
                    blur_radius: px(0.),
                    spread_radius: px(2.),
                    inset: false,
                }])
            })
            .child(
                row()
                    .size(px(size))
                    .rounded_full()
                    .justify_center()
                    .bg(if self.open { m.glass } else { m.solid })
                    .when(self.open, |b| b.border_1().border_color(m.line))
                    .shadow(m.shadow.clone())
                    .opacity(if self.enabled() { 1. } else { 0.42 })
                    .child(drawing(
                        self.previous_glyph,
                        self.glyph,
                        glyph_t,
                        24. * size / TRIGGER,
                        trigger_fore,
                        false,
                    )),
            )
            .when(self.enabled(), |trigger| {
                trigger
                    .cursor_pointer()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|s, e: &MouseDownEvent, w, cx| {
                            s.begin(e.position, false, w, cx)
                        }),
                    )
                    .on_click(cx.listener(|s, e: &ClickEvent, w, cx| {
                        if e.is_keyboard() {
                            if s.open {
                                s.close(cx);
                                s.trigger.focus(w, cx);
                            } else {
                                s.focus_action(0, w, cx);
                            }
                        }
                    }))
                    .on_key_down(cx.listener(Self::trigger_key))
                    .child(self.contact.listen(cx, |s, e, w, cx| match e.phase {
                        TouchPhase::Started => s.begin(e.start_position, true, w, cx),
                        TouchPhase::Moved => s.moved(e.position, true, cx),
                        TouchPhase::Ended => s.release(e.position, true, false, w, cx),
                        TouchPhase::Cancelled => s.release(e.position, true, true, w, cx),
                    }))
            });
        root = root.child(trigger);
        let text = if let Some(i) = self.target.filter(|_| self.open) {
            Some(self.slots[i].action.label.clone())
        } else if self.open {
            Some(if self.press.is_some_and(|p| p.moved) {
                tr(cx, "Release to cancel", "松手取消").into()
            } else {
                label
            })
        } else {
            self.done.clone()
        };
        self.pill_alpha
            .duration(f32::from(text.is_some()), 0.22, cx);
        if let Some(text) = text.filter(|t| *t != self.pill_text) {
            self.pill_before = std::mem::replace(&mut self.pill_text, text);
            self.pill_clock.restart(cx);
        }
        let pill_alpha = self.pill_alpha.get(w, cx).clamp(0., 1.);
        if pill_alpha > 0.001 {
            let width = (measure(&self.pill_text, 14., w).ceil() + 28.).max(46.);
            let width_slide = self.pill_width.get_or_insert(Slide::new(width));
            width_slide.duration(width, 0.31, cx);
            let width = width_slide.get(w, cx);
            let swap = ease(self.pill_clock.time(0.16, w, cx) / 0.16);
            let (x, y) = offset(radius * 0.5, direction);
            root = root.child(
                row()
                    .id("orbit-pill")
                    .debug_selector(|| "orbit-pill".into())
                    .absolute()
                    .left(px(TRIGGER / 2. + x - width / 2.))
                    .top(px(TRIGGER / 2. + y - 16. + (1. - pill_alpha) * 6.))
                    .w(px(width))
                    .h(px(32.))
                    .overflow_hidden()
                    .justify_center()
                    .rounded_full()
                    .border_1()
                    .border_color(m.line)
                    .bg(m.glass)
                    .shadow(m.shadow.clone())
                    .opacity(pill_alpha)
                    .text_color(m.text)
                    .text_size(px(14.))
                    .font_weight(FontWeight::MEDIUM)
                    .when(!self.pill_before.is_empty() && swap < 0.999, |b| {
                        b.child(
                            row()
                                .absolute()
                                .inset_0()
                                .justify_center()
                                .top(px(-4. * swap))
                                .whitespace_nowrap()
                                .opacity(1. - swap)
                                .child(self.pill_before.clone()),
                        )
                    })
                    .child(
                        div()
                            .relative()
                            .top(px(4. * (1. - swap)))
                            .whitespace_nowrap()
                            .opacity(swap)
                            .child(self.pill_text.clone()),
                    ),
            );
        } else if !self.open && self.done.is_none() {
            self.pill_width = None;
            self.pill_text = "".into();
            self.pill_before = "".into();
        }
        if self.announce
            && let Some(done) = &self.done
        {
            root = root.child(
                div()
                    .id("orbit-status")
                    .role(Role::Status)
                    .aria_label(done.clone())
                    .absolute()
                    .size(px(0.)),
            );
        }
        let mouse_move = cx.listener(|s, e: &MouseMoveEvent, _, cx| {
            if s.press.is_some() && e.pressed_button == Some(MouseButton::Left) {
                s.moved(e.position, false, cx);
            } else if s.open && s.press.is_none() && e.pressed_button.is_none() {
                let relative = e.position - s.bounds.get().center();
                let near =
                    f32::from(relative.x).hypot(f32::from(relative.y)) <= s.radius + REACH_MARGIN;
                let pointer = near.then_some(e.position);
                let target = s.direct_hit(e.position).or(s.roving);
                if s.pointer != pointer || s.target != target {
                    s.pointer = pointer;
                    s.target = target;
                    cx.notify();
                }
            }
        });
        let mouse_up = cx.listener(|s, e: &MouseUpEvent, w, cx| {
            if e.button == MouseButton::Left {
                s.release(e.position, false, false, w, cx);
            }
        });
        let outside = cx.listener(|s, e: &MouseDownEvent, _, cx| {
            if s.open
                && s.press.is_none()
                && !s.hit_trigger(e.position)
                && s.direct_hit(e.position).is_none()
            {
                s.close(cx);
            }
        });
        root.child(
            canvas(
                |_, _, _| (),
                move |_, _, w, _| {
                    w.on_mouse_event(move |e: &MouseMoveEvent, phase, w, cx| {
                        if phase.capture() {
                            mouse_move(e, w, cx);
                        }
                    });
                    w.on_mouse_event(move |e: &MouseUpEvent, phase, w, cx| {
                        if phase.capture() {
                            mouse_up(e, w, cx);
                        }
                    });
                    w.on_mouse_event(move |e: &MouseDownEvent, phase, w, cx| {
                        if phase.capture() {
                            outside(e, w, cx);
                        }
                    });
                },
            )
            .absolute()
            .size(px(0.)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{OrbitAction, OrbitInvoked, OrbitMenu, OrbitOpenChanged};
    use crate::primitives::glyph::Glyph;
    use gpui::{
        Context, Entity, FocusHandle, IntoElement, Modifiers, MouseButton, Render, TestAppContext,
        TouchEvent, TouchId, TouchPhase, VisualTestContext, Window, div, point, prelude::*, px,
    };
    use std::{cell::RefCell, rc::Rc, time::Duration};

    struct Harness {
        menu: Entity<OrbitMenu>,
        next: FocusHandle,
    }

    impl Harness {
        fn new(w: &mut Window, cx: &mut Context<Self>) -> Self {
            let menu = cx.new(|cx| {
                OrbitMenu::new(
                    vec![
                        OrbitAction::new("like", "Like", Glyph::Heart).with_done("Liked"),
                        OrbitAction::new("pin", "Pin", Glyph::Pin),
                        OrbitAction::new("save", "Save", Glyph::Save),
                        OrbitAction::new("get", "Download", Glyph::Download),
                        OrbitAction::new("delete", "Delete", Glyph::Trash),
                    ],
                    w,
                    cx,
                )
            });
            Self {
                menu,
                next: cx.focus_handle().tab_stop(true),
            }
        }
    }

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
                .relative()
                .size(px(400.))
                .child(
                    div()
                        .absolute()
                        .left(px(165.))
                        .top(px(210.))
                        .child(self.menu.clone()),
                )
                .child(
                    div()
                        .id("after-orbit")
                        .track_focus(&self.next)
                        .tab_stop(true)
                        .absolute()
                        .left(px(16.))
                        .top(px(300.))
                        .size(px(32.)),
                )
        }
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
    fn a_short_tap_pins_the_menu_and_a_stationary_hold_closes_it() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) = app.add_window_view(Harness::new);
        let menu = w.update(|_, cx| view.read(cx).menu.clone());
        let center = w.debug_bounds("orbit-trigger").unwrap().center();
        let opens = Rc::new(RefCell::new(Vec::new()));
        let _sub = w.update(|_, cx| {
            let opens = opens.clone();
            cx.subscribe(&menu, move |_, event: &OrbitOpenChanged, _| {
                opens.borrow_mut().push(event.0)
            })
        });
        w.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        assert!(w.update(|_, cx| menu.read(cx).is_open()));
        tick(w, 339);
        w.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
        assert!(w.update(|_, cx| menu.read(cx).is_open()));
        w.simulate_click(center, Modifiers::default());
        assert!(!w.update(|_, cx| menu.read(cx).is_open()));
        w.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        tick(w, 340);
        w.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
        assert!(!w.update(|_, cx| menu.read(cx).is_open()));
        assert_eq!(opens.borrow().as_slice(), &[true, false, true, false]);
    }

    #[test]
    fn angular_drag_fires_once_with_a_timed_confirmation_and_off_arc_cancels() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) = app.add_window_view(Harness::new);
        let menu = w.update(|_, cx| view.read(cx).menu.clone());
        let center = w.debug_bounds("orbit-trigger").unwrap().center();
        let actions = Rc::new(RefCell::new(Vec::new()));
        let _sub = w.update(|_, cx| {
            let actions = actions.clone();
            cx.subscribe(&menu, move |_, event: &OrbitInvoked, _| {
                actions.borrow_mut().push(event.0.clone())
            })
        });
        for delta in [
            point(px(0.), px(72.)),
            point(px(-179.), px(-48.)),
            point(px(-22.), px(-6.)),
        ] {
            w.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
            w.simulate_mouse_move(
                center + delta,
                Some(MouseButton::Left),
                Modifiers::default(),
            );
            w.simulate_mouse_up(center + delta, MouseButton::Left, Modifiers::default());
            assert!(!w.update(|_, cx| menu.read(cx).is_open()));
        }
        assert!(actions.borrow().is_empty());
        let inside = center + point(px(-62.), px(-17.));
        w.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        w.simulate_mouse_move(inside, Some(MouseButton::Left), Modifiers::default());
        w.simulate_mouse_up(inside, MouseButton::Left, Modifiers::default());
        w.run_until_parked();
        assert_eq!(actions.borrow().as_slice(), &["like"]);
        tick(w, 1399);
        w.update(|w, cx| {
            let s = menu.read(cx);
            assert_eq!(s.done.as_deref(), Some("Liked"));
            assert!(s.trigger.is_focused(w));
        });
        tick(w, 1);
        assert!(w.update(|_, cx| menu.read(cx).done.is_none()));
    }

    #[test]
    fn keyboard_wraps_and_tab_advances_outside_the_arc() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) = app.add_window_view(Harness::new);
        let menu = w.update(|_, cx| view.read(cx).menu.clone());
        w.update(|w, cx| menu.update(cx, |s, cx| s.focus_trigger(w, cx)));
        w.simulate_keystrokes("up");
        w.update(|w, cx| assert!(menu.read(cx).slots[4].focus.is_focused(w)));
        w.simulate_keystrokes("right");
        w.update(|w, cx| assert!(menu.read(cx).slots[0].focus.is_focused(w)));
        w.simulate_keystrokes("escape");
        w.update(|w, cx| assert!(menu.read(cx).trigger.is_focused(w)));
        w.simulate_keystrokes("enter");
        w.update(|w, cx| assert!(menu.read(cx).slots[0].focus.is_focused(w)));
        w.simulate_keystrokes("tab");
        tick(w, 20);
        w.update(|w, cx| {
            assert!(!menu.read(cx).is_open());
            assert!(view.read(cx).next.is_focused(w));
        });
    }

    #[test]
    fn cancelled_touch_dismisses_and_a_rotated_arc_uses_the_same_dead_zone() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) = app.add_window_view(Harness::new);
        let menu = w.update(|_, cx| view.read(cx).menu.clone());
        let center = w.debug_bounds("orbit-trigger").unwrap().center();
        let toward = center + point(px(0.), px(-65.));
        for (phase, position) in [
            (TouchPhase::Started, center),
            (TouchPhase::Moved, toward),
            (TouchPhase::Cancelled, toward),
        ] {
            w.simulate_event(TouchEvent {
                id: TouchId(1),
                phase,
                position,
                ..Default::default()
            });
            tick(w, 16);
        }
        w.update(|_, cx| {
            menu.update(cx, |s, _| {
                assert!(!s.is_open());
                assert!(s.done.is_none());
                s.direction = 90.;
                assert_eq!(s.pick(center + point(px(0.), px(75.))), Some(2));
                assert_eq!(s.pick(center + point(px(0.), px(33.))), None);
                assert_eq!(s.pick(toward), None);
            })
        });
    }
}
