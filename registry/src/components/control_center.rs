//! Quick settings with focus sessions, notification channels and elastic meters.
use crate::{
    foundation::{Theme, surface_radius},
    primitives::{
        family::{Paint, PhaseClock, Slide, disabled_motion, ease},
        glyph::{Glyph, icon, transformed},
        presentation::{column, row, tr},
    },
};
use chrono::{DateTime, Local, TimeDelta};
use gpui::{prelude::*, *};
use std::{
    cell::Cell,
    collections::BTreeMap,
    f32::consts::{FRAC_PI_2, PI, TAU},
    rc::Rc,
    time::Duration,
};

const WIDTH: f32 = 360.;
const HEIGHT: f32 = 396.;
const SEGMENTS: usize = 28;

/// A selectable focus session preset.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FocusMode {
    pub id: SharedString,
    pub label: SharedString,
    pub icon: Glyph,
    pub description: SharedString,
    pub default_minutes: u16,
}

impl FocusMode {
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        icon: Glyph,
        description: impl Into<SharedString>,
        default_minutes: u16,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon,
            description: description.into(),
            default_minutes,
        }
    }
}

/// An independently configurable channel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotificationChannel {
    pub id: SharedString,
    pub label: SharedString,
    pub description: Option<SharedString>,
    pub default_on: bool,
}

impl NotificationChannel {
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            description: None,
            default_on: false,
        }
    }

    pub fn with_description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_default_on(mut self, on: bool) -> Self {
        self.default_on = on;
        self
    }
}

/// A person whose alerts can reach a focused user.
#[derive(Clone)]
pub struct FocusPerson {
    pub id: SharedString,
    pub name: SharedString,
    pub avatar: ImageSource,
}

impl FocusPerson {
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        avatar: impl Into<ImageSource>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            avatar: avatar.into(),
        }
    }
}

/// A committed session with a display-ready end time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FocusSession {
    pub mode: SharedString,
    pub minutes: u16,
    pub until: SharedString,
}

/// The full settings snapshot reported to the host.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControlCenterState {
    pub focus: Option<FocusSession>,
    pub notifications: bool,
    pub channels: BTreeMap<SharedString, bool>,
    pub presenting: bool,
    pub volume: u8,
    pub text_size: u8,
}

impl Default for ControlCenterState {
    fn default() -> Self {
        Self {
            focus: None,
            notifications: true,
            channels: BTreeMap::new(),
            presenting: false,
            volume: 60,
            text_size: 100,
        }
    }
}

/// Emitted once per committed change, including distinct quantized meter steps.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControlCenterChanged(pub ControlCenterState);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Detail {
    Focus,
    Notifications,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Meter {
    Volume,
    Text,
}

impl Meter {
    const ALL: [Self; 2] = [Self::Volume, Self::Text];

    fn index(self) -> usize {
        usize::from(self == Self::Text)
    }

    fn limits(self) -> (u8, u8, u8) {
        match self {
            Self::Volume => (0, 100, 1),
            Self::Text => (85, 130, 5),
        }
    }

    fn position(self, value: u8) -> f32 {
        let (lo, hi, _) = self.limits();
        f32::from(value.saturating_sub(lo)) / f32::from(hi - lo)
    }

    fn snap(self, value: f32) -> u8 {
        let (lo, hi, step) = self.limits();
        let step = f32::from(step);
        let step_value =
            ((value.clamp(f32::from(lo), f32::from(hi)) - f32::from(lo)) / step).round() * step;
        lo + step_value as u8
    }

    fn value_at(self, position: f32) -> u8 {
        let (lo, hi, _) = self.limits();
        self.snap(f32::from(lo) + f32::from(hi - lo) * position)
    }

    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::Volume => tr(cx, "Alert volume", "提示音量"),
            Self::Text => tr(cx, "Text size", "文字大小"),
        }
    }
}

#[derive(Default)]
struct ModalFade {
    from: f32,
    to: f32,
    clock: PhaseClock,
}

impl ModalFade {
    fn value(&self, cx: &App) -> f32 {
        self.from + (self.to - self.from) * ease(self.clock.age(cx) / 0.16)
    }

    fn retarget(&mut self, to: f32, cx: &App) {
        self.from = self.value(cx);
        self.to = to;
        self.clock.restart(cx);
    }

    fn get(&self, w: &Window, cx: &App) -> f32 {
        if self.clock.age(cx) < 0.16 {
            w.request_animation_frame();
        }
        self.value(cx)
    }
}

#[derive(Clone, Copy)]
struct Draft {
    mode: usize,
    minutes: u16,
}

#[derive(Clone, Copy)]
struct MeterMotion {
    position: Slide,
    held: Slide,
}

impl MeterMotion {
    fn new(m: Meter, value: u8) -> Self {
        Self {
            position: Slide::snappy(m.position(value)),
            held: Slide::snappy(0.),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Axis {
    Pending,
    Primary,
    Cross,
}

#[derive(Clone, Copy)]
enum Drag {
    Meter {
        kind: Meter,
        origin: f32,
        axis: Axis,
    },
    Dial {
        last_angle: f32,
        raw: f32,
        initial: u16,
        moved: bool,
    },
}

#[derive(Clone, Copy)]
struct Gesture {
    touch: bool,
    start: Point<Pixels>,
    drag: Drag,
}

struct FocusTargets {
    tiles: [FocusHandle; 3],
    chevrons: [FocusHandle; 2],
    meters: [FocusHandle; 2],
    dialog: FocusHandle,
    close: FocusHandle,
    modes: Vec<FocusHandle>,
    dial: FocusHandle,
    commit: FocusHandle,
    end: FocusHandle,
    switches: Vec<FocusHandle>,
}

type TimeFormat = Box<dyn Fn(DateTime<Local>) -> SharedString>;

/// A workspace's compact settings panel and its two detail surfaces.
pub struct ControlCenter {
    id: ElementId,
    label: Option<SharedString>,
    modes: Vec<FocusMode>,
    channels: Vec<NotificationChannel>,
    people: Vec<FocusPerson>,
    state: ControlCenterState,
    time_format: Option<TimeFormat>,
    style: Option<Box<dyn Fn(Div) -> Div>>,
    focus: FocusTargets,
    detail: Option<Detail>,
    shown: Option<Detail>,
    reveal: Slide,
    modal_fade: ModalFade,
    draft: Draft,
    draft_start: DateTime<Local>,
    expiry: Option<Task<()>>,
    epoch: u64,
    meter_motion: [MeterMotion; 2],
    dial: Slide,
    mode_lens: Slide,
    tile_motion: [Slide; 3],
    tile_words: [TileWords; 3],
    switches: Vec<Slide>,
    gesture: Option<Gesture>,
    hits: [Rc<Cell<Bounds<Pixels>>>; 3],
    room: Rc<Cell<Pixels>>,
    channel_scroll: ScrollHandle,
}

impl EventEmitter<ControlCenterChanged> for ControlCenter {}

impl ControlCenter {
    pub fn new(
        modes: Vec<FocusMode>,
        channels: Vec<NotificationChannel>,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus = FocusTargets {
            tiles: std::array::from_fn(|_| cx.focus_handle().tab_stop(true)),
            chevrons: std::array::from_fn(|_| cx.focus_handle().tab_stop(true)),
            meters: std::array::from_fn(|_| cx.focus_handle().tab_stop(true)),
            dialog: cx.focus_handle(),
            close: cx.focus_handle().tab_stop(true),
            modes: modes.iter().map(|_| cx.focus_handle()).collect(),
            dial: cx.focus_handle().tab_stop(true),
            commit: cx.focus_handle().tab_stop(true),
            end: cx.focus_handle().tab_stop(true),
            switches: (0..=channels.len())
                .map(|_| cx.focus_handle().tab_stop(true))
                .collect(),
        };
        let switches = std::iter::once(true)
            .chain(channels.iter().map(|c| c.default_on))
            .map(|value| Slide::snappy(f32::from(value)))
            .collect();
        let minutes = modes
            .first()
            .map_or(50, |m| snap_minutes(f32::from(m.default_minutes)));
        let state = ControlCenterState {
            channels: channels
                .iter()
                .map(|c| (c.id.clone(), c.default_on))
                .collect(),
            ..ControlCenterState::default()
        };
        Self {
            id: format!("control-center-{}", cx.entity_id()).into(),
            label: None,
            modes,
            channels,
            people: Vec::new(),
            time_format: None,
            style: None,
            focus,
            detail: None,
            shown: None,
            reveal: Slide::new(0.),
            modal_fade: ModalFade::default(),
            draft: Draft { mode: 0, minutes },
            draft_start: Local::now(),
            expiry: None,
            epoch: 0,
            meter_motion: [
                MeterMotion::new(Meter::Volume, state.volume),
                MeterMotion::new(Meter::Text, state.text_size),
            ],
            dial: Slide::new(f32::from(minutes)),
            mode_lens: Slide::snappy(0.),
            tile_motion: [Slide::snappy(0.), Slide::snappy(1.), Slide::snappy(0.)],
            tile_words: std::array::from_fn(|_| TileWords::default()),
            switches,
            gesture: None,
            hits: std::array::from_fn(|_| Rc::default()),
            room: Rc::default(),
            channel_scroll: ScrollHandle::new(),
            state,
        }
    }

    pub fn with_id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = id.into();
        self
    }

    pub fn with_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn with_people(mut self, people: Vec<FocusPerson>) -> Self {
        self.people = people;
        self
    }

    pub fn with_default_volume(mut self, value: u8) -> Self {
        self.state.volume = value.min(100);
        self.meter_motion[0] = MeterMotion::new(Meter::Volume, self.state.volume);
        self
    }

    pub fn with_default_text_size(mut self, value: u8) -> Self {
        self.state.text_size = Meter::Text.snap(f32::from(value));
        self.meter_motion[1] = MeterMotion::new(Meter::Text, self.state.text_size);
        self
    }

    /// Formats the local end time of draft and committed sessions.
    pub fn with_format_time(
        mut self,
        format: impl Fn(DateTime<Local>) -> SharedString + 'static,
    ) -> Self {
        self.time_format = Some(Box::new(format));
        self
    }

    pub fn with_style(mut self, style: impl Fn(Div) -> Div + 'static) -> Self {
        self.style = Some(Box::new(style));
        self
    }

    pub fn state(&self) -> &ControlCenterState {
        &self.state
    }

    /// The painted dial bounds for embedding hosts that route touch gestures.
    pub fn dial_bounds(&self, cx: &App) -> Option<Bounds<Pixels>> {
        let bounds = self.hits[2].get();
        (self.detail == Some(Detail::Focus)
            && self.reveal.value(cx) >= 0.85
            && bounds.size.width > px(0.)
            && bounds.size.height > px(0.))
        .then_some(bounds)
    }

    /// The active channel list scroll handle, when its detail is open.
    pub fn channel_scroll(&self) -> Option<&ScrollHandle> {
        (self.detail == Some(Detail::Notifications)).then_some(&self.channel_scroll)
    }

    fn changed(&self, cx: &mut Context<Self>) {
        cx.emit(ControlCenterChanged(self.state.clone()));
        cx.notify();
    }

    fn end_label(&self, end: DateTime<Local>) -> SharedString {
        self.time_format
            .as_ref()
            .map_or_else(|| end.format("%-I:%M %p").to_string().into(), |f| f(end))
    }

    fn start_session(&mut self, draft: Draft, cx: &mut Context<Self>) {
        let Some(mode) = self.modes.get(draft.mode) else {
            return;
        };
        let minutes = snap_minutes(f32::from(draft.minutes));
        let end = Local::now() + TimeDelta::minutes(i64::from(minutes));
        self.state.focus = Some(FocusSession {
            mode: mode.id.clone(),
            minutes,
            until: self.end_label(end),
        });
        self.tile_motion[0].set(1., cx);
        self.epoch = self.epoch.wrapping_add(1);
        let epoch = self.epoch;
        let delay = Duration::from_secs(u64::from(minutes) * 60);
        self.expiry = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(delay).await;
            let _ = this.update(cx, |s, cx| {
                if s.epoch == epoch && s.state.focus.take().is_some() {
                    s.tile_motion[0].set(0., cx);
                    s.changed(cx);
                }
            });
        }));
        self.changed(cx);
    }

    fn end_session(&mut self, cx: &mut Context<Self>) {
        self.expiry = None;
        self.epoch = self.epoch.wrapping_add(1);
        if self.state.focus.take().is_some() {
            self.tile_motion[0].set(0., cx);
            self.changed(cx);
        }
    }

    fn toggle_focus(&mut self, cx: &mut Context<Self>) {
        if self.state.focus.is_some() {
            self.end_session(cx);
        } else if let Some(mode) = self.modes.first() {
            self.start_session(
                Draft {
                    mode: 0,
                    minutes: mode.default_minutes,
                },
                cx,
            );
        }
    }

    fn toggle_notifications(&mut self, cx: &mut Context<Self>) {
        self.state.notifications = !self.state.notifications;
        let target = f32::from(self.state.notifications);
        self.tile_motion[1].set(target, cx);
        self.switches[0].set(target, cx);
        self.changed(cx);
    }

    fn toggle_channel(&mut self, i: usize, cx: &mut Context<Self>) {
        let Some(ch) = self.channels.get(i) else {
            return;
        };
        let Some(value) = self.state.channels.get_mut(&ch.id) else {
            return;
        };
        *value = !*value;
        let target = f32::from(*value);
        // Identical IDs share the same preference in the exported map.
        for (j, c) in self
            .channels
            .iter()
            .enumerate()
            .filter(|(_, c)| c.id == ch.id)
        {
            let _ = c;
            self.switches[j + 1].set(target, cx);
        }
        self.changed(cx);
    }

    fn toggle_presenting(&mut self, cx: &mut Context<Self>) {
        self.state.presenting = !self.state.presenting;
        self.tile_motion[2].set(f32::from(self.state.presenting), cx);
        self.changed(cx);
    }

    fn open(&mut self, detail: Detail, w: &mut Window, cx: &mut Context<Self>) {
        self.finish(true, w, cx);
        if detail == Detail::Focus {
            self.draft_start = Local::now();
            let active = self.state.focus.as_ref().and_then(|a| {
                let i = self.modes.iter().position(|m| m.id == a.mode)?;
                Some(Draft {
                    mode: i,
                    minutes: a.minutes,
                })
            });
            self.draft = active.unwrap_or_else(|| Draft {
                mode: 0,
                minutes: self
                    .modes
                    .first()
                    .map_or(50, |m| snap_minutes(f32::from(m.default_minutes))),
            });
            self.dial.reset(f32::from(self.draft.minutes));
            self.mode_lens.reset(self.draft.mode as f32);
        }
        self.detail = Some(detail);
        self.shown = Some(detail);
        self.reveal.duration(1., 0.36, cx);
        self.modal_fade.retarget(1., cx);
        self.focus.close.focus(w, cx);
        cx.notify();
    }

    fn dismiss(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let Some(detail) = self.detail.take() else {
            return;
        };
        self.finish(true, w, cx);
        self.reveal.duration(0., 0.34, cx);
        self.modal_fade.retarget(0., cx);
        self.focus.chevrons[usize::from(detail == Detail::Notifications)].focus(w, cx);
        cx.notify();
    }

    fn choose_mode(&mut self, i: usize, w: &mut Window, cx: &mut Context<Self>) {
        let Some(mode) = self.modes.get(i) else {
            return;
        };
        if i != self.draft.mode {
            self.draft = Draft {
                mode: i,
                minutes: snap_minutes(f32::from(mode.default_minutes)),
            };
            self.dial.set(f32::from(self.draft.minutes), cx);
            self.mode_lens.set(i as f32, cx);
            cx.notify();
        }
        self.focus.modes[i].focus(w, cx);
    }

    fn meter_value(&self, kind: Meter) -> u8 {
        match kind {
            Meter::Volume => self.state.volume,
            Meter::Text => self.state.text_size,
        }
    }

    fn set_meter(&mut self, kind: Meter, value: u8, cx: &mut Context<Self>) {
        let field = match kind {
            Meter::Volume => &mut self.state.volume,
            Meter::Text => &mut self.state.text_size,
        };
        if *field != value {
            *field = value;
            self.changed(cx);
        }
    }

    fn step_meter(
        &mut self,
        kind: Meter,
        e: &KeyDownEvent,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (lo, hi, step) = kind.limits();
        let Some(value) = key_step(
            &e.keystroke.key,
            i32::from(self.meter_value(kind)),
            i32::from(lo),
            i32::from(hi),
            i32::from(step),
            i32::from(step) * 4,
        ) else {
            return;
        };
        self.set_meter(kind, value as u8, cx);
        self.meter_motion[kind.index()]
            .position
            .set(kind.position(value as u8), cx);
        w.prevent_default();
        cx.stop_propagation();
    }

    fn step_dial(&mut self, e: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        let Some(value) = key_step(
            &e.keystroke.key,
            i32::from(self.draft.minutes),
            5,
            120,
            5,
            15,
        ) else {
            return;
        };
        self.draft.minutes = snap_minutes(value as f32);
        self.dial.set(f32::from(self.draft.minutes), cx);
        w.prevent_default();
        cx.stop_propagation();
        cx.notify();
    }

    fn start_meter(
        &mut self,
        kind: Meter,
        at: Point<Pixels>,
        touch: bool,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.gesture.is_some() || self.detail.is_some() {
            return;
        }
        self.gesture = Some(Gesture {
            touch,
            start: at,
            drag: Drag::Meter {
                kind,
                origin: kind.position(self.meter_value(kind)),
                axis: Axis::Pending,
            },
        });
        self.meter_motion[kind.index()].held.set(1., cx);
        self.focus.meters[kind.index()].focus(w, cx);
        cx.notify();
    }

    fn start_dial(
        &mut self,
        at: Point<Pixels>,
        touch: bool,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.gesture.is_some() || self.detail != Some(Detail::Focus) {
            return;
        }
        let Some(last_angle) = angle(self.hits[2].get().center(), at) else {
            return;
        };
        self.gesture = Some(Gesture {
            touch,
            start: at,
            drag: Drag::Dial {
                last_angle,
                raw: f32::from(self.draft.minutes),
                initial: self.draft.minutes,
                moved: false,
            },
        });
        self.focus.dial.focus(w, cx);
        cx.notify();
    }

    fn move_gesture(&mut self, at: Point<Pixels>, touch: bool, cx: &mut Context<Self>) {
        let Some(mut gesture) = self.gesture.filter(|g| g.touch == touch) else {
            return;
        };
        let dx = f32::from(at.x - gesture.start.x);
        let dy = f32::from(at.y - gesture.start.y);
        match &mut gesture.drag {
            Drag::Meter { kind, origin, axis } => {
                if *axis == Axis::Pending && dx.abs().max(dy.abs()) >= 4. {
                    *axis = if touch && dy.abs() > dx.abs() {
                        Axis::Cross
                    } else {
                        Axis::Primary
                    };
                }
                if *axis == Axis::Primary {
                    let width = f32::from(self.hits[kind.index()].get().size.width).max(1.);
                    let raw = *origin + dx / width;
                    self.meter_motion[kind.index()]
                        .position
                        .reset(if disabled_motion(cx) {
                            raw.clamp(0., 1.)
                        } else {
                            rubber(raw, 0., 1., 0.30)
                        });
                    self.set_meter(*kind, kind.value_at(raw), cx);
                }
            }
            Drag::Dial {
                last_angle,
                raw,
                moved,
                ..
            } => {
                let Some(next) = angle(self.hits[2].get().center(), at) else {
                    return;
                };
                *moved |= dx.hypot(dy) >= 4.;
                if *moved {
                    let delta = (next - *last_angle + PI).rem_euclid(TAU) - PI;
                    *raw += delta / TAU * 120.;
                    *raw = raw.clamp(-45., 165.);
                    self.draft.minutes = raw.round().clamp(5., 120.) as u16;
                    self.dial.reset(if disabled_motion(cx) {
                        raw.clamp(5., 120.)
                    } else {
                        rubber(*raw, 5., 120., 22.)
                    });
                    *last_angle = next;
                }
            }
        }
        self.gesture = Some(gesture);
        cx.notify();
    }

    fn finish(&mut self, cancel: bool, _: &mut Window, cx: &mut Context<Self>) {
        let Some(gesture) = self.gesture.take() else {
            return;
        };
        match gesture.drag {
            Drag::Meter { kind, axis, .. } => {
                let hit = self.hits[kind.index()].get();
                if !cancel && axis == Axis::Pending && hit.size.width > px(0.) {
                    self.set_meter(
                        kind,
                        kind.value_at((gesture.start.x - hit.left()) / hit.size.width),
                        cx,
                    );
                }
                let target = kind.position(self.meter_value(kind));
                self.meter_motion[kind.index()]
                    .position
                    .duration(target, 0.29, cx);
                self.meter_motion[kind.index()].held.set(0., cx);
            }
            Drag::Dial {
                last_angle,
                raw,
                initial,
                moved,
            } => {
                let value = if cancel {
                    initial
                } else if moved {
                    snap_minutes(raw)
                } else {
                    snap_minutes(last_angle / TAU * 120.)
                };
                self.draft.minutes = snap_minutes(f32::from(value));
                self.dial.duration(f32::from(self.draft.minutes), 0.29, cx);
            }
        }
        cx.notify();
    }
}

fn snap_minutes(value: f32) -> u16 {
    ((value / 5.).round().clamp(1., 24.) * 5.) as u16
}

fn key_step(key: &str, value: i32, lo: i32, hi: i32, step: i32, page: i32) -> Option<i32> {
    Some(
        match key {
            "left" | "down" => value - step,
            "right" | "up" => value + step,
            "pageup" => value + page,
            "pagedown" => value - page,
            "home" => lo,
            "end" => hi,
            _ => return None,
        }
        .clamp(lo, hi),
    )
}

fn angle(center: Point<Pixels>, at: Point<Pixels>) -> Option<f32> {
    let x = f32::from(at.x - center.x);
    let y = f32::from(at.y - center.y);
    (x.hypot(y) >= 14.).then(|| (y.atan2(x) + FRAC_PI_2).rem_euclid(TAU))
}

fn rubber(value: f32, lo: f32, hi: f32, span: f32) -> f32 {
    let edge = value.clamp(lo, hi);
    let delta = value - edge;
    edge + delta * 0.42 / (1. + delta.abs() / span)
}

#[derive(Clone, Copy)]
struct BoxRect {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

impl BoxRect {
    fn at<E: Styled>(self, e: E) -> E {
        e.absolute()
            .left(px(self.x))
            .top(px(self.y))
            .w(px(self.w))
            .h(px(self.h))
    }

    fn mix(self, to: Self, t: f32) -> Self {
        let mix = |a, b| a + (b - a) * t;
        Self {
            x: mix(self.x, to.x),
            y: mix(self.y, to.y),
            w: mix(self.w, to.w),
            h: mix(self.h, to.h),
        }
    }
}

#[derive(Clone, Copy)]
struct Layout {
    width: f32,
    narrow: bool,
}

impl Layout {
    fn tile(self, i: usize) -> BoxRect {
        let inset = if self.narrow { 8. } else { 11. };
        let width = self.width - inset * 2.;
        if i == 0 {
            return BoxRect {
                x: inset,
                y: 11.,
                w: width,
                h: 84.,
            };
        }
        let side = (width - 8.) / 2.;
        BoxRect {
            x: inset + if i == 1 { 0. } else { side + 8. },
            y: 103.,
            w: side,
            h: 104.,
        }
    }

    fn detail(self) -> BoxRect {
        BoxRect {
            x: 9.,
            y: 9.,
            w: self.width - 18.,
            h: HEIGHT - 18.,
        }
    }

    fn meter(self, kind: Meter) -> BoxRect {
        let x = if self.narrow { 13. } else { 17. };
        BoxRect {
            x,
            y: 232. + kind.index() as f32 * 71.6,
            w: self.width - 2. * x,
            h: 60.,
        }
    }
}

#[derive(Clone, Copy)]
struct Palette {
    p: Paint,
    shell: Hsla,
    tile: Hsla,
    selected: Hsla,
    lens: Hsla,
}

impl Palette {
    fn current(cx: &App) -> Self {
        let p = Paint::get(cx);
        let shell = p.stage.blend(p.panel.opacity(0.46));
        Self {
            p,
            shell,
            tile: p.stage,
            selected: p.hover,
            lens: shell.blend(p.panel.opacity(0.5)),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mark {
    Headphones,
    Monitor,
    Speaker,
    Type,
    Other(Glyph),
}

fn mark(mark: Mark, side: f32, ink: Hsla) -> AnyElement {
    if let Mark::Other(glyph) = mark {
        return icon(glyph, side, ink).into_any_element();
    }
    canvas(
        |_, _, _| (),
        move |b, _, w, _| {
            let at = |x, y| b.origin + point(px(x * side / 24.), px(y * side / 24.));
            let mut path = PathBuilder::stroke(px(1.45 * side / 24.));
            match mark {
                Mark::Headphones => {
                    path.move_to(at(3., 16.));
                    path.line_to(at(3., 11.5));
                    path.cubic_bezier_to(at(21., 11.5), at(3., 0.5), at(21., 0.5));
                    path.line_to(at(21., 16.));
                    for (x, edge) in [(3., 8.), (21., 16.)] {
                        path.move_to(at(x, 13.));
                        path.line_to(at(edge, 13.));
                        path.line_to(at(edge, 20.));
                        path.line_to(at(x, 20.));
                        path.close();
                    }
                }
                Mark::Monitor => {
                    path.move_to(at(3., 4.));
                    path.line_to(at(21., 4.));
                    path.line_to(at(20., 15.));
                    path.line_to(at(4., 15.));
                    path.close();
                    path.move_to(at(12., 15.));
                    path.line_to(at(12., 21.));
                    for x in [6., 18.] {
                        path.move_to(at(12., 16.));
                        path.line_to(at(x, 21.));
                    }
                }
                Mark::Speaker => {
                    path.move_to(at(3., 9.));
                    path.line_to(at(7., 9.));
                    path.line_to(at(12., 4.));
                    path.line_to(at(12., 20.));
                    path.line_to(at(7., 15.));
                    path.line_to(at(3., 15.));
                    path.close();
                    for radius in [4., 8.] {
                        path.move_to(at(13. + radius * 0.7, 12. - radius * 0.75));
                        path.curve_to(
                            at(13. + radius * 0.7, 12. + radius * 0.75),
                            at(13. + radius * 1.35, 12.),
                        );
                    }
                }
                Mark::Type => {
                    path.move_to(at(4., 8.));
                    path.line_to(at(4., 4.));
                    path.line_to(at(20., 4.));
                    path.line_to(at(20., 8.));
                    path.move_to(at(12., 4.));
                    path.line_to(at(12., 21.));
                    path.move_to(at(8., 21.));
                    path.line_to(at(16., 21.));
                }
                Mark::Other(_) => {}
            }
            if let Ok(path) = path.build() {
                w.paint_path(path, ink);
            }
        },
    )
    .size(px(side))
    .into_any_element()
}

fn ring(color: Hsla) -> Vec<BoxShadow> {
    vec![
        BoxShadow::new(px(0.), px(0.), color)
            .spread_radius(px(1.5))
            .inset(),
    ]
}

fn polar(center: Point<Pixels>, distance: f32, radians: f32) -> Point<Pixels> {
    center + point(px(distance * radians.sin()), px(-distance * radians.cos()))
}

fn stroke(w: &mut Window, thickness: f32, ink: Hsla, draw: impl FnOnce(&mut PathBuilder)) {
    let mut path = PathBuilder::stroke(px(thickness));
    draw(&mut path);
    if let Ok(path) = path.build() {
        w.paint_path(path, ink);
    }
}

fn paint_dial(b: Bounds<Pixels>, minutes: f32, p: Palette, w: &mut Window) {
    let center = b.center();
    let side = f32::from(b.size.width);
    let radius = side * 0.472;
    let track = p.p.ink.opacity(0.10);
    w.paint_quad(
        outline(b.inset(px(side * 0.027)), track, BorderStyle::Solid)
            .border_widths(px(3.))
            .corner_radii(px(radius)),
    );
    stroke(w, 1.3, p.p.ink.opacity(0.28), |path| {
        for i in 0..24 {
            let theta = i as f32 * TAU / 24.;
            let inner = radius - if i % 6 == 0 { 15. } else { 11. };
            path.move_to(polar(center, radius - 5., theta));
            path.line_to(polar(center, inner, theta));
        }
    });
    let progress = minutes.clamp(0., 120.) / 120.;
    let theta = progress * TAU;
    let segments = ((progress * 96.).ceil() as usize).max(1);
    stroke(w, 3.3, p.p.secondary, |path| {
        path.move_to(polar(center, radius - 1.5, 0.));
        for i in 1..=segments {
            path.line_to(polar(
                center,
                radius - 1.5,
                theta * i as f32 / segments as f32,
            ));
        }
    });
    let knob = polar(center, radius - 1.5, minutes / 120. * TAU);
    let thumb = Bounds::new(knob - point(px(7.5), px(7.5)), size(px(15.), px(15.)));
    w.paint_quad(quad(
        thumb,
        px(7.5),
        p.tile,
        px(3.),
        p.p.secondary,
        BorderStyle::Solid,
    ));
}

fn paint_meter(b: Bounds<Pixels>, level: f32, held: f32, p: Palette, w: &mut Window) {
    let scale = 1. + 0.14 * held;
    let width = f32::from(b.size.width);
    let edge = level - level.clamp(0., 1.);
    let shift = (width * edge * 0.46).clamp(-13., 13.);
    let gap = 3.;
    let segment = ((width - gap * (SEGMENTS - 1) as f32) / SEGMENTS as f32).max(0.);
    let height = f32::from(b.size.height) * scale;
    let fill_edge = b.left() + px(shift + width * level.clamp(0., 1.));
    for i in 0..SEGMENTS {
        let left = b.left() + px(shift + i as f32 * (segment + gap));
        let rect = Bounds::new(
            point(left, b.center().y - px(height / 2.)),
            size(px(segment), px(height)),
        );
        w.paint_quad(fill(rect, p.p.ink.opacity(0.11)).corner_radii(px(2.)));
        if fill_edge >= rect.right() {
            w.paint_quad(fill(rect, p.p.ink).corner_radii(px(2.)));
        } else if fill_edge > left {
            let mask = Bounds::new(rect.origin, size(fill_edge - left, rect.size.height))
                .intersect(&w.content_mask().bounds);
            w.with_content_mask(Some(ContentMask { bounds: mask }), |w| {
                w.paint_quad(fill(rect, p.p.ink).corner_radii(px(2.)));
            });
        }
    }
}

type TileCopy = (SharedString, SharedString, Mark, bool);

#[derive(Default)]
struct TileWords {
    current: Option<TileCopy>,
    previous: Vec<(TileCopy, f32)>,
    clock: PhaseClock,
}

impl TileWords {
    fn retarget(&mut self, next: TileCopy, cx: &App) {
        if self.current.as_ref() == Some(&next) {
            if self.clock.age(cx) >= 0.10 {
                self.previous.clear();
            }
            return;
        }
        let prior = self.current.replace(next);
        if let Some(prior) = prior {
            let age = self.clock.age(cx);
            let fade = (1. - age / 0.10).clamp(0., 1.);
            self.previous.retain_mut(|(_, weight)| {
                *weight *= fade;
                *weight > 0.01
            });
            let weight = (age / 0.16).clamp(0., 1.);
            if weight > 0.01 {
                self.previous.push((prior, weight));
            }
            self.clock.restart(cx);
        }
    }
}

impl ControlCenter {
    fn tile_data(&self, i: usize, cx: &App) -> TileCopy {
        let off = tr(cx, "Off", "关闭");
        match i {
            0 => {
                if let Some(session) = self.state.focus.as_ref()
                    && let Some(mode) = self.modes.iter().find(|m| m.id == session.mode)
                {
                    let until = tr(cx, "Until", "持续至");
                    return (
                        mode.label.clone(),
                        format!("{until} {}", session.until).into(),
                        Mark::Other(mode.icon),
                        true,
                    );
                }
                (
                    tr(cx, "Focus", "专注").into(),
                    off.into(),
                    Mark::Headphones,
                    false,
                )
            }
            1 => {
                let count = self.state.channels.values().filter(|&&on| on).count();
                let total = self.state.channels.len();
                let text = if self.state.notifications {
                    if tr(cx, "en", "zh") == "zh" {
                        format!("{count} / {total} 个频道")
                    } else {
                        format!("{count} of {total} channels")
                    }
                } else {
                    tr(cx, "Paused", "已暂停").into()
                };
                (
                    tr(cx, "Notifications", "通知").into(),
                    text.into(),
                    Mark::Other(Glyph::Bell),
                    self.state.notifications,
                )
            }
            _ => (
                tr(cx, "Presenting", "演示").into(),
                if self.state.presenting {
                    tr(cx, "Previews hidden", "已隐藏预览").into()
                } else {
                    off.into()
                },
                Mark::Monitor,
                self.state.presenting,
            ),
        }
    }

    fn badge(&self, glyph: Mark, selected: f32, p: Palette) -> Div {
        row()
            .flex_none()
            .justify_center()
            .size(px(36.))
            .rounded_full()
            .bg(p.shell.blend(p.p.ink.opacity(selected * 0.78)))
            .child(mark(
                glyph,
                18.,
                p.p.ink.blend(p.p.inverse.opacity(selected)),
            ))
    }

    fn tile_face(&self, i: usize, copy: &TileCopy, value: f32, p: Palette) -> Div {
        let (title, subtitle, glyph, _) = copy;
        let words = column()
            .gap(px(2.))
            .min_w_0()
            .child(
                div()
                    .text_size(px(14.))
                    .line_height(px(18.))
                    .truncate()
                    .child(title.clone()),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .line_height(px(16.))
                    .text_color(p.p.secondary)
                    .truncate()
                    .child(subtitle.clone()),
            );
        let badge = self.badge(*glyph, value, p);
        let content = if i == 0 {
            row()
                .h_full()
                .gap(px(12.))
                .pr(px(38.))
                .child(badge)
                .child(words)
        } else {
            column().gap(px(7.)).pt(px(12.)).child(badge).child(words)
        };
        div().absolute().inset_0().px(px(14.)).child(content)
    }

    fn tile(
        &self,
        i: usize,
        l: Layout,
        p: Palette,
        value: f32,
        w: &Window,
        cx: &Context<Self>,
    ) -> Stateful<Div> {
        let copy = self.tile_data(i, cx);
        let (title, subtitle, _, on) = &copy;
        let label = format!("{title} {subtitle}");
        let enabled = self.detail.is_none() && (i != 0 || !self.modes.is_empty());
        let radius = surface_radius(cx);
        let words = &self.tile_words[i];
        let age = words.clock.age(cx);
        if age < 0.16 {
            w.request_animation_frame();
        }
        let fade_out = (1. - age / 0.10).clamp(0., 1.);
        let fade_in = (age / 0.16).clamp(0., 1.);
        let content = div()
            .relative()
            .size_full()
            .children(
                words
                    .previous
                    .iter()
                    .filter(|(_, alpha)| *alpha * fade_out > 0.001)
                    .map(|(prior, alpha)| {
                        self.tile_face(i, prior, value, p)
                            .opacity(*alpha * fade_out)
                    }),
            )
            .child(self.tile_face(i, &copy, value, p).opacity(fade_in));
        let tile = div()
            .id(("center-tile", i))
            .rounded(radius)
            .bg(p.tile.blend(p.selected.opacity(value)));
        let body = div()
            .id(("center-tile-action", i))
            .debug_selector(move || format!("center-tile-{i}"))
            .absolute()
            .inset_0()
            .rounded(radius)
            .when(enabled, |d| {
                d.role(Role::Button)
                    .aria_label(label)
                    .aria_toggled((*on).into())
                    .track_focus(&self.focus.tiles[i])
                    .cursor_pointer()
                    .hover(move |s| s.bg(p.p.ink.opacity(0.04)))
                    .focus_visible(move |s| s.shadow(ring(p.p.focus)))
                    .on_click(cx.listener(move |s, _, _, cx| match i {
                        0 => s.toggle_focus(cx),
                        1 => s.toggle_notifications(cx),
                        _ => s.toggle_presenting(cx),
                    }))
            })
            .child(content);
        let mut tile = l.tile(i).at(tile).child(body);
        if i < 2 && enabled {
            let name = if i == 0 {
                tr(cx, "Focus session settings", "专注会话设置")
            } else {
                tr(cx, "Notification channels", "通知频道")
            };
            let detail = if i == 0 {
                Detail::Focus
            } else {
                Detail::Notifications
            };
            tile = tile.child(
                row()
                    .id(("center-chevron", i))
                    .debug_selector(move || format!("center-chevron-{i}"))
                    .role(Role::Button)
                    .aria_label(name)
                    .aria_expanded(false)
                    .track_focus(&self.focus.chevrons[i])
                    .block_mouse_except_scroll()
                    .absolute()
                    .top(px(if i == 0 { 26. } else { 8. }))
                    .right(px(if i == 0 { 14. } else { 8. }))
                    .size(px(32.))
                    .rounded_full()
                    .justify_center()
                    .cursor_pointer()
                    .hover(move |s| s.bg(p.p.ink.opacity(0.09)))
                    .focus_visible(move |s| s.shadow(ring(p.p.focus)))
                    .child(transformed(
                        Glyph::Down,
                        14.,
                        p.p.secondary,
                        -FRAC_PI_2,
                        1.,
                        (0., 0.),
                        1.,
                    ))
                    .on_click(cx.listener(move |s, _, w, cx| s.open(detail, w, cx))),
            );
        }
        tile
    }

    fn meter(&self, kind: Meter, l: Layout, p: Palette, w: &Window, cx: &Context<Self>) -> Div {
        let i = kind.index();
        let label = kind.label(cx);
        let level = self.meter_motion[i].position.get(w, cx);
        let held = if disabled_motion(cx) {
            0.
        } else {
            self.meter_motion[i].held.get(w, cx)
        };
        let value = self.meter_value(kind);
        let bounds = self.hits[i].clone();
        let (min, max, step) = kind.limits();
        let percent = format!("{value}%");
        let sample = 14. * (85. + 45. * level.clamp(0., 1.)) / 100.;
        let chrome = row()
            .h(px(20.))
            .w_full()
            .gap(px(7.))
            .child(mark(
                if kind == Meter::Volume {
                    Mark::Speaker
                } else {
                    Mark::Type
                },
                16.,
                p.p.secondary,
            ))
            .child(div().flex_1().truncate().child(label))
            .when(kind == Meter::Text, |d| {
                d.child(
                    row()
                        .w(px(27.))
                        .h(px(20.))
                        .justify_end()
                        .text_size(px(sample))
                        .line_height(px(20.))
                        .child("Aa"),
                )
            })
            .child(
                div()
                    .w(px(39.))
                    .text_right()
                    .text_size(px(12.))
                    .text_color(p.p.secondary)
                    .child(percent.clone()),
            );
        let track = div()
            .id(("center-meter", i))
            .debug_selector(move || format!("center-meter-{i}"))
            .relative()
            .w_full()
            .h(px(30.))
            .rounded(px(3.))
            .when(self.detail.is_none(), |d| {
                d.role(Role::Slider)
                    .aria_label(label)
                    .aria_numeric_value(f64::from(value))
                    .aria_value(percent)
                    .aria_min_numeric_value(f64::from(min))
                    .aria_max_numeric_value(f64::from(max))
                    .aria_numeric_value_step(f64::from(step))
                    .track_focus(&self.focus.meters[i])
                    .cursor_pointer()
                    .focus_visible(move |s| s.shadow(ring(p.p.focus)))
                    .on_key_down(cx.listener(move |s, e, w, cx| s.step_meter(kind, e, w, cx)))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |s, e: &MouseDownEvent, w, cx| {
                            s.start_meter(kind, e.position, false, w, cx)
                        }),
                    )
            })
            .child(
                canvas(
                    move |b, _, _| bounds.set(b),
                    move |b, _, w, _| paint_meter(b, level, held, p, w),
                )
                .absolute()
                .inset_0(),
            );
        l.meter(kind)
            .at(column().gap(px(10.)))
            .child(chrome)
            .child(track)
    }

    fn mode_selector(
        &self,
        p: Palette,
        active: bool,
        w: &Window,
        cx: &Context<Self>,
    ) -> Stateful<Div> {
        let n = self.modes.len();
        let width = 1. / n.max(1) as f32;
        let left = self.mode_lens.get(w, cx) * width;
        row()
            .id("center-modes")
            .role(Role::RadioGroup)
            .aria_label(tr(cx, "Focus mode", "专注模式"))
            .relative()
            .w_full()
            .h(px(36.))
            .px(px(3.))
            .rounded_full()
            .bg(p.selected.opacity(0.74))
            .child(
                div()
                    .absolute()
                    .left(px(3.))
                    .right(px(3.))
                    .top(px(3.))
                    .bottom(px(3.))
                    .child(
                        div()
                            .absolute()
                            .left(relative(left))
                            .w(relative(width))
                            .h_full()
                            .rounded_full()
                            .bg(p.lens),
                    ),
            )
            .children(self.modes.iter().enumerate().map(|(i, mode)| {
                let chosen = i == self.draft.mode;
                row()
                    .id(("center-mode", i))
                    .debug_selector(move || format!("center-mode-{i}"))
                    .flex_1()
                    .justify_center()
                    .min_w_0()
                    .h(px(30.))
                    .rounded_full()
                    .text_size(px(12.))
                    .text_color(if chosen { p.p.ink } else { p.p.secondary })
                    .when(active, |e| {
                        e.role(Role::RadioButton)
                            .aria_label(mode.label.clone())
                            .aria_toggled(chosen.into())
                            .track_focus(&self.focus.modes[i].clone().tab_stop(chosen))
                            .cursor_pointer()
                            .focus_visible(move |s| s.shadow(ring(p.p.focus)))
                            .on_click(cx.listener(move |s, _, w, cx| s.choose_mode(i, w, cx)))
                            .on_key_down(cx.listener(move |s, e: &KeyDownEvent, w, cx| {
                                let next = match e.keystroke.key.as_str() {
                                    "left" | "up" => (i + n - 1) % n,
                                    "right" | "down" => (i + 1) % n,
                                    "home" => 0,
                                    "end" => n - 1,
                                    _ => return,
                                };
                                s.choose_mode(next, w, cx);
                                w.prevent_default();
                                cx.stop_propagation();
                            }))
                    })
                    .child(
                        div()
                            .min_w_0()
                            .px(px(3.))
                            .truncate()
                            .child(mode.label.clone()),
                    )
            }))
    }

    fn session_dial(
        &self,
        l: Layout,
        p: Palette,
        active: bool,
        w: &Window,
        cx: &Context<Self>,
    ) -> Stateful<Div> {
        let side = if l.narrow { 150. } else { 164. };
        let minutes = self.draft.minutes;
        let value = self.dial.get(w, cx);
        let bounds = self.hits[2].clone();
        let minute = tr(cx, "min", "分钟");
        let until = self
            .state
            .focus
            .as_ref()
            .filter(|session| {
                self.modes
                    .get(self.draft.mode)
                    .is_some_and(|mode| mode.id == session.mode)
                    && session.minutes == minutes
            })
            .map_or_else(
                || self.end_label(self.draft_start + TimeDelta::minutes(i64::from(minutes))),
                |s| s.until.clone(),
            );
        div()
            .id("center-dial")
            .debug_selector(|| "center-dial".into())
            .relative()
            .size(px(side))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .rounded_full()
            .when(active, |e| {
                e.role(Role::Slider)
                    .aria_label(tr(cx, "Session length", "会话时长"))
                    .aria_numeric_value(f64::from(minutes))
                    .aria_value(format!("{minutes} {minute}"))
                    .aria_min_numeric_value(5.)
                    .aria_max_numeric_value(120.)
                    .aria_numeric_value_step(5.)
                    .track_focus(&self.focus.dial)
                    .cursor_pointer()
                    .focus_visible(move |s| s.shadow(ring(p.p.focus)))
                    .on_key_down(cx.listener(|s, e, w, cx| s.step_dial(e, w, cx)))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|s, e: &MouseDownEvent, w, cx| {
                            s.start_dial(e.position, false, w, cx)
                        }),
                    )
            })
            .child(
                canvas(
                    move |b, _, _| bounds.set(b),
                    move |b, _, w, _| paint_dial(b, value, p, w),
                )
                .absolute()
                .inset_0(),
            )
            .child(
                column()
                    .items_center()
                    .child(
                        row()
                            .gap(px(4.))
                            .items_end()
                            .child(
                                div()
                                    .text_size(px(36.))
                                    .line_height(px(38.))
                                    .child(minutes.to_string()),
                            )
                            .child(
                                div()
                                    .pb(px(2.))
                                    .text_size(px(14.))
                                    .text_color(p.p.secondary)
                                    .child(minute),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .line_height(px(18.))
                            .text_color(p.p.secondary)
                            .child(format!("{} {until}", tr(cx, "Until", "持续至"))),
                    ),
            )
    }
}

impl ControlCenter {
    fn detail_button(
        &self,
        end: bool,
        label: &'static str,
        active: bool,
        p: Palette,
        cx: &Context<Self>,
    ) -> Stateful<Div> {
        let radius = Theme::global(cx).tokens.radius.full;
        row()
            .id(if end { "center-end" } else { "center-commit" })
            .debug_selector(move || {
                if end {
                    "center-end".into()
                } else {
                    "center-commit".into()
                }
            })
            .flex_1()
            .h(px(38.))
            .justify_center()
            .min_w_0()
            .rounded(radius)
            .text_size(px(14.))
            .bg(if end { p.shell } else { p.p.ink })
            .text_color(if end { p.p.ink } else { p.p.inverse })
            .when(active, |e| {
                e.role(Role::Button)
                    .aria_label(label)
                    .track_focus(if end {
                        &self.focus.end
                    } else {
                        &self.focus.commit
                    })
                    .cursor_pointer()
                    .hover(move |s| s.bg(if end { p.selected } else { p.p.secondary }))
                    .focus_visible(move |s| s.shadow(ring(p.p.focus)))
                    .on_click(cx.listener(move |s, _, w, cx| {
                        if end {
                            s.end_session(cx);
                        } else {
                            s.start_session(s.draft, cx);
                        }
                        s.dismiss(w, cx);
                    }))
            })
            .child(div().min_w_0().px(px(8.)).truncate().child(label))
    }

    fn focus_body(
        &self,
        l: Layout,
        p: Palette,
        active: bool,
        w: &Window,
        cx: &Context<Self>,
    ) -> Div {
        let description = self
            .modes
            .get(self.draft.mode)
            .map(|m| m.description.clone())
            .unwrap_or_default();
        let running = self.state.focus.is_some();
        div()
            .absolute()
            .inset_0()
            .child(
                column()
                    .absolute()
                    .left(px(14.))
                    .right(px(14.))
                    .top(px(60.))
                    .items_center()
                    .child(self.mode_selector(p, active, w, cx))
                    .child(
                        div()
                            .mt(px(9.))
                            .h(px(20.))
                            .w_full()
                            .text_center()
                            .truncate()
                            .text_size(px(12.))
                            .line_height(px(20.))
                            .text_color(p.p.secondary)
                            .child(description),
                    )
                    .child(
                        div()
                            .mt(px(9.))
                            .child(self.session_dial(l, p, active, w, cx)),
                    ),
            )
            .child(
                row()
                    .absolute()
                    .left(px(14.))
                    .right(px(14.))
                    .bottom(px(14.))
                    .gap(px(8.))
                    .when(running, |e| {
                        e.child(self.detail_button(
                            true,
                            tr(cx, "End session", "结束会话"),
                            active,
                            p,
                            cx,
                        ))
                    })
                    .child(self.detail_button(
                        false,
                        if running {
                            tr(cx, "Update session", "更新会话")
                        } else {
                            tr(cx, "Start session", "开始会话")
                        },
                        active,
                        p,
                        cx,
                    )),
            )
    }

    fn switch_row(
        &self,
        i: usize,
        p: Palette,
        active: bool,
        w: &Window,
        cx: &Context<Self>,
    ) -> Div {
        let (label, description, checked) = if i == 0 {
            let message = if self.state.notifications {
                tr(cx, "Delivered as they arrive", "收到后立即提醒")
            } else {
                tr(cx, "Paused until you resume", "暂停至手动恢复")
            };
            (
                tr(cx, "All notifications", "全部通知").into(),
                Some(SharedString::from(message)),
                self.state.notifications,
            )
        } else {
            let ch = &self.channels[i - 1];
            (
                ch.label.clone(),
                ch.description.clone(),
                self.state.channels.get(&ch.id).copied().unwrap_or(false),
            )
        };
        let position = self.switches[i].get(w, cx).clamp(0., 1.);
        let track = div()
            .id(("center-switch", i))
            .debug_selector(move || format!("center-switch-{i}"))
            .relative()
            .flex_none()
            .w(px(40.))
            .h(px(24.))
            .rounded_full()
            .bg(p.selected.blend(p.p.ink.opacity(position)))
            .when(active, |e| {
                e.role(Role::Switch)
                    .aria_label(label.clone())
                    .aria_toggled(checked.into())
                    .track_focus(&self.focus.switches[i])
                    .cursor_pointer()
                    .focus_visible(move |s| s.shadow(ring(p.p.focus)))
                    .on_click(cx.listener(move |s, _, _, cx| {
                        if i == 0 {
                            s.toggle_notifications(cx)
                        } else {
                            s.toggle_channel(i - 1, cx)
                        }
                    }))
            })
            .child(
                div()
                    .absolute()
                    .left(px(3. + 16. * position))
                    .top(px(3.))
                    .size(px(18.))
                    .rounded_full()
                    .bg(p.p.ink.blend(p.p.inverse.opacity(position))),
            );
        row()
            .h(px(if i == 0 { 46. } else { 50. }))
            .w_full()
            .flex_none()
            .gap(px(8.))
            .child(
                column()
                    .flex_1()
                    .min_w_0()
                    .child(
                        div()
                            .text_size(px(14.))
                            .line_height(px(18.))
                            .truncate()
                            .child(label),
                    )
                    .when_some(description, |e, description| {
                        e.child(
                            div()
                                .text_size(px(12.))
                                .line_height(px(16.))
                                .text_color(p.p.secondary)
                                .truncate()
                                .child(description),
                        )
                    }),
            )
            .child(track)
    }

    fn people_footer(&self, p: Palette, cx: &App) -> Div {
        let names = self
            .people
            .iter()
            .take(3)
            .map(|person| person.name.as_ref())
            .collect::<Vec<_>>()
            .join(", ");
        let more = self.people.len().saturating_sub(3);
        let names = if more > 0 {
            format!("{names} +{more}")
        } else {
            names
        };
        let label = if tr(cx, "en", "zh") == "zh" {
            format!("{names} 可在专注期间联系你")
        } else {
            format!("{names} can reach you during focus")
        };
        row()
            .absolute()
            .left(px(14.))
            .right(px(14.))
            .bottom(px(13.))
            .h(px(45.))
            .gap(px(9.))
            .pt(px(8.))
            .border_t_1()
            .border_color(p.p.ink.opacity(0.045))
            .child(
                row()
                    .flex_none()
                    .children(self.people.iter().take(3).enumerate().map(|(i, person)| {
                        div()
                            .flex_none()
                            .size(px(27.))
                            .when(i > 0, |e| e.ml(px(-6.)))
                            .rounded_full()
                            .border_1()
                            .border_color(p.tile)
                            .overflow_hidden()
                            .child(
                                img(person.avatar.clone())
                                    .size_full()
                                    .object_fit(ObjectFit::Cover),
                            )
                    })),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_size(px(12.))
                    .line_height(px(16.))
                    .text_color(p.p.secondary)
                    .child(label),
            )
    }

    fn notifications_body(&self, p: Palette, active: bool, w: &Window, cx: &Context<Self>) -> Div {
        div()
            .absolute()
            .inset_0()
            .child(
                column()
                    .absolute()
                    .left(px(14.))
                    .right(px(14.))
                    .top(px(60.))
                    .child(self.switch_row(0, p, active, w, cx))
                    .child(
                        div()
                            .h(px(1.))
                            .w_full()
                            .mt(px(14.))
                            .bg(p.p.ink.opacity(0.045)),
                    ),
            )
            .child(
                column()
                    .id("center-channel-list")
                    .absolute()
                    .left(px(14.))
                    .right(px(14.))
                    .top(px(126.))
                    .bottom(px(if self.people.is_empty() { 14. } else { 62. }))
                    .overflow_y_scroll()
                    .track_scroll(&self.channel_scroll)
                    .children(
                        (1..=self.channels.len()).map(|i| self.switch_row(i, p, active, w, cx)),
                    ),
            )
            .when(!self.people.is_empty(), |e| {
                e.child(self.people_footer(p, cx))
            })
    }

    fn detail_key(&mut self, e: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        let Some(detail) = self.detail else { return };
        match e.keystroke.key.as_str() {
            "escape" => self.dismiss(w, cx),
            "tab" => {
                if e.keystroke.modifiers.shift {
                    w.focus_prev(cx)
                } else {
                    w.focus_next(cx)
                }
                if !self.focus.dialog.contains_focused(w, cx) {
                    let last = match detail {
                        Detail::Focus => &self.focus.commit,
                        Detail::Notifications => {
                            self.focus.switches.last().unwrap_or(&self.focus.close)
                        }
                    };
                    if e.keystroke.modifiers.shift {
                        last.focus(w, cx)
                    } else {
                        self.focus.close.focus(w, cx)
                    }
                }
            }
            _ => return,
        }
        w.prevent_default();
        cx.stop_propagation();
    }

    fn render_detail(
        &self,
        detail: Detail,
        l: Layout,
        p: Palette,
        progress: f32,
        w: &Window,
        cx: &Context<Self>,
    ) -> Stateful<Div> {
        let active = self.detail == Some(detail);
        let target = l.detail();
        let source_index = usize::from(detail == Detail::Notifications);
        let source = l.tile(source_index);
        let rect = source.mix(target, progress);
        let from_value = self.tile_motion[source_index].get(w, cx).clamp(0., 1.);
        let collapsed = div()
            .absolute()
            .top_0()
            .left_0()
            .w(px(source.w))
            .h(px(source.h))
            .opacity(1. - ease((progress - 0.04) / 0.56))
            .child(self.tile_face(
                source_index,
                &self.tile_data(source_index, cx),
                from_value,
                p,
            ))
            .child(
                row()
                    .absolute()
                    .top(px(if source_index == 0 { 26. } else { 8. }))
                    .right(px(if source_index == 0 { 14. } else { 8. }))
                    .size(px(32.))
                    .justify_center()
                    .child(transformed(
                        Glyph::Down,
                        14.,
                        p.p.secondary,
                        -FRAC_PI_2,
                        1.,
                        (0., 0.),
                        1.,
                    )),
            );
        let focus = detail == Detail::Focus;
        let title = if focus {
            tr(cx, "Focus", "专注")
        } else {
            tr(cx, "Notifications", "通知")
        };
        let close = tr(cx, "Close", "关闭");
        let body = if focus {
            self.focus_body(l, p, active, w, cx)
        } else {
            self.notifications_body(p, active, w, cx)
        };
        let content = div()
            .relative()
            .flex_none()
            .w(px(target.w))
            .h(px(target.h))
            .opacity(ease((progress - 0.27) / 0.73))
            .child(
                row()
                    .absolute()
                    .left(px(14.))
                    .right(px(12.))
                    .top(px(12.))
                    .h(px(36.))
                    .gap(px(12.))
                    .child(self.badge(
                        if focus {
                            Mark::Headphones
                        } else {
                            Mark::Other(Glyph::Bell)
                        },
                        1.,
                        p,
                    ))
                    .child(div().flex_1().text_size(px(14.)).truncate().child(title))
                    .child(
                        row()
                            .id("center-close")
                            .debug_selector(|| "center-close".into())
                            .size(px(32.))
                            .rounded_full()
                            .justify_center()
                            .bg(p.shell)
                            .when(active, |e| {
                                e.role(Role::Button)
                                    .aria_label(close)
                                    .track_focus(&self.focus.close)
                                    .cursor_pointer()
                                    .hover(move |s| s.bg(p.selected))
                                    .focus_visible(move |s| s.shadow(ring(p.p.focus)))
                                    .on_click(cx.listener(|s, _, w, cx| s.dismiss(w, cx)))
                            })
                            .child(icon(Glyph::Close, 14., p.p.ink)),
                    ),
            )
            .child(body);
        rect.at(div()
            .id("center-detail")
            .debug_selector(|| "center-detail".into())
            .rounded(surface_radius(cx))
            .overflow_hidden()
            .bg(p
                .tile
                .blend(p.selected.opacity((1. - progress) * from_value)))
            .when(active, |e| {
                e.role(Role::Dialog)
                    .aria_label(title)
                    .track_focus(&self.focus.dialog)
                    .tab_group()
                    .block_mouse_except_scroll()
                    .on_key_down(cx.listener(|s, e, w, cx| s.detail_key(e, w, cx)))
            }))
        .when(progress < 0.60, |e| e.child(collapsed))
        .child(content)
    }
}

impl Render for ControlCenter {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let radius = surface_radius(cx);
        let p = Palette::current(cx);
        let room = self.room.get();
        let max = w.viewport_size().width.min(px(WIDTH));
        let width = f32::from(if room > px(0.) { room.min(max) } else { max }).max(1.);
        let l = Layout {
            width,
            narrow: width < WIDTH,
        };
        let reduced = disabled_motion(cx);
        let show = if reduced {
            self.modal_fade.get(w, cx)
        } else {
            self.reveal.get(w, cx).clamp(0., 1.)
        };
        let label = self
            .label
            .clone()
            .unwrap_or_else(|| tr(cx, "Quick settings", "快捷设置").into());
        let tile_motion = self.tile_motion.map(|m| m.get(w, cx).clamp(0., 1.));
        for i in 0..self.tile_words.len() {
            let copy = self.tile_data(i, cx);
            self.tile_words[i].retarget(copy, cx);
        }
        let root = div()
            .id(self.id.clone())
            .debug_selector(|| "center-shell".into())
            .role(Role::Group)
            .aria_label(label)
            .relative()
            .h(px(HEIGHT))
            .w_full()
            .rounded(radius)
            .bg(p.shell)
            .border_1()
            .border_color(p.p.ink.opacity(0.06))
            .text_size(px(14.))
            .text_color(p.p.ink)
            .shadow(vec![
                BoxShadow::new(px(0.), px(9.), black().opacity(0.14)).blur_radius(px(24.)),
            ])
            .child(
                div()
                    .absolute()
                    .inset_0()
                    .opacity(1. - 0.82 * show)
                    .children(
                        tile_motion
                            .into_iter()
                            .enumerate()
                            .map(|(i, v)| self.tile(i, l, p, v, w, cx)),
                    )
                    .children(Meter::ALL.into_iter().map(|m| self.meter(m, l, p, w, cx))),
            )
            .when(self.detail.is_some(), |e| {
                e.child(
                    div()
                        .id("center-scrim")
                        .absolute()
                        .inset_0()
                        .block_mouse_except_scroll()
                        .on_click(cx.listener(|s, _, w, cx| s.dismiss(w, cx))),
                )
            })
            .when_some(
                self.shown.filter(|_| self.detail.is_some() || show > 0.001),
                |e, d| {
                    let detail =
                        self.render_detail(d, l, p, if reduced { 1. } else { show }, w, cx);
                    e.child(detail.when(reduced, |d| d.opacity(show)))
                },
            )
            .child(self.capture(cx));
        let room = self.room.clone();
        let shell = div()
            .w(px(WIDTH))
            .max_w_full()
            .on_children_prepainted(move |children, w, _| {
                if let Some(b) = children.first()
                    && room.replace(b.size.width) != b.size.width
                {
                    w.request_animation_frame();
                }
            })
            .child(root);
        match &self.style {
            Some(style) => style(shell),
            None => shell,
        }
    }
}

impl ControlCenter {
    fn capture(&self, cx: &Context<Self>) -> impl IntoElement {
        let moving =
            cx.listener(|s, e: &MouseMoveEvent, _, cx| s.move_gesture(e.position, false, cx));
        let release = cx.listener(|s, e: &MouseUpEvent, w, cx| {
            if e.button == MouseButton::Left && s.gesture.is_some_and(|g| !g.touch) {
                s.move_gesture(e.position, false, cx);
                s.finish(false, w, cx);
            }
        });
        let touch = cx.listener(|s, e: &TouchDragEvent, w, cx| match e.phase {
            TouchPhase::Started if s.gesture.is_none() => {
                if s.dial_bounds(cx)
                    .is_some_and(|b| b.contains(&e.start_position))
                {
                    s.start_dial(e.start_position, true, w, cx);
                } else if s.detail.is_none()
                    && let Some(kind) = Meter::ALL
                        .into_iter()
                        .find(|m| s.hits[m.index()].get().contains(&e.start_position))
                {
                    s.start_meter(kind, e.start_position, true, w, cx);
                }
                if s.gesture.is_some_and(|g| g.touch) {
                    w.prevent_default();
                    cx.stop_propagation();
                }
            }
            TouchPhase::Moved if s.gesture.is_some_and(|g| g.touch) => {
                s.move_gesture(e.position, true, cx)
            }
            TouchPhase::Ended if s.gesture.is_some_and(|g| g.touch) => {
                s.move_gesture(e.position, true, cx);
                s.finish(false, w, cx);
            }
            TouchPhase::Cancelled if s.gesture.is_some_and(|g| g.touch) => s.finish(true, w, cx),
            _ => {}
        });
        canvas(
            |_, _, _| (),
            move |_, _, w, _| {
                w.on_mouse_event(move |e: &MouseMoveEvent, phase, w, cx| {
                    if phase.capture() {
                        moving(e, w, cx)
                    }
                });
                w.on_mouse_event(move |e: &MouseUpEvent, phase, w, cx| {
                    if phase.capture() {
                        release(e, w, cx)
                    }
                });
                w.on_mouse_event(move |e: &TouchDragEvent, phase, w, cx| {
                    if phase.bubble() {
                        touch(e, w, cx)
                    }
                });
            },
        )
        .absolute()
        .size(px(0.))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ControlCenter, ControlCenterChanged, FocusMode, Meter, NotificationChannel, polar,
    };
    use crate::{foundation::Theme, primitives::glyph::Glyph};
    use gpui::{
        Context, KeyUpEvent, Keystroke, Modifiers, MouseButton, Styled, TestAppContext, TouchEvent,
        TouchId, TouchPhase, VisualTestContext, point, px,
    };
    use std::{cell::RefCell, f32::consts::TAU, rc::Rc, time::Duration};

    fn center(cx: &mut Context<ControlCenter>) -> ControlCenter {
        ControlCenter::new(
            vec![
                FocusMode::new("deep", "Deep work", Glyph::Lock, "Mentions only", 50),
                FocusMode::new("meeting", "Meeting", Glyph::Clock, "Calendar alerts", 30),
            ],
            vec![
                NotificationChannel::new("mentions", "Mentions").with_default_on(true),
                NotificationChannel::new("deploys", "Deploys"),
            ],
            cx,
        )
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

    fn click(w: &mut VisualTestContext, id: &'static str) {
        let p = w.debug_bounds(id).unwrap().center();
        w.simulate_click(p, Modifiers::default());
        tick(w, 600);
    }

    fn press(w: &mut VisualTestContext, key: &'static str) {
        w.simulate_keystrokes(key);
        w.simulate_event(KeyUpEvent {
            keystroke: Keystroke::parse(key).unwrap(),
        });
        tick(w, 600);
    }

    #[test]
    fn focus_draft_cancels_and_expiry_clears_the_committed_session_once() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) = app.add_window_view(|_, cx| center(cx));
        tick(w, 0);
        let events = Rc::new(RefCell::new(Vec::new()));
        let log = events.clone();
        w.update(|_, cx| {
            cx.subscribe(&view, move |_, event: &ControlCenterChanged, _| {
                log.borrow_mut().push(event.0.focus.clone())
            })
            .detach()
        });
        click(w, "center-chevron-0");
        click(w, "center-mode-1");
        let dial = w.debug_bounds("center-dial").unwrap().center();
        w.simulate_click(dial, Modifiers::default());
        press(w, "pageup");
        w.update(|_, cx| assert_eq!(view.read(cx).draft.minutes, 45));
        press(w, "escape");
        assert!(w.update(|w, cx| view.read(cx).focus.chevrons[0].is_focused(w)));
        assert!(events.borrow().is_empty());

        click(w, "center-chevron-0");
        w.update(|_, cx| assert_eq!(view.read(cx).draft.minutes, 50));
        click(w, "center-commit");
        assert_eq!(events.borrow().len(), 1);
        assert_eq!(events.borrow()[0].as_ref().unwrap().mode.as_ref(), "deep");
        tick(w, 50 * 60 * 1000);
        assert_eq!(events.borrow().len(), 2);
        assert!(events.borrow()[1].is_none());
        tick(w, 50 * 60 * 1000);
        assert_eq!(events.borrow().len(), 2);
    }

    #[test]
    fn meter_drag_retains_grab_offset_and_touch_vertical_pan_does_not_change_it() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) = app.add_window_view(|_, cx| center(cx));
        tick(w, 0);
        let b = w.debug_bounds("center-meter-0").unwrap();
        let grab = point(b.left() + b.size.width * 0.2, b.center().y);
        let end = grab + point(b.size.width * 0.1, px(0.));
        w.simulate_mouse_down(grab, MouseButton::Left, Modifiers::default());
        assert_eq!(
            w.update(|_, cx| view.read(cx).meter_value(Meter::Volume)),
            60
        );
        w.simulate_mouse_move(end, MouseButton::Left, Modifiers::default());
        w.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
        tick(w, 550);
        assert_eq!(w.update(|_, cx| view.read(cx).state().volume), 70);
        press(w, "pagedown");
        assert_eq!(w.update(|_, cx| view.read(cx).state().volume), 66);

        for (phase, position) in [
            (TouchPhase::Started, grab),
            (TouchPhase::Moved, grab + point(px(2.), px(56.))),
            (TouchPhase::Ended, grab + point(px(2.), px(56.))),
        ] {
            w.simulate_event(TouchEvent {
                id: TouchId(7),
                phase,
                position,
                ..Default::default()
            });
            tick(w, 16);
        }
        assert_eq!(w.update(|_, cx| view.read(cx).state().volume), 66);
        w.simulate_click(grab, Modifiers::default());
        assert_eq!(w.update(|_, cx| view.read(cx).state().volume), 20);
    }

    #[test]
    fn dial_crosses_noon_without_wrap_and_snaps_in_a_narrow_panel() {
        let mut app = TestAppContext::single();
        app.update(|cx| {
            crate::init(cx);
            Theme::global_mut(cx).reduced_motion = true;
        });
        let (view, w) = app.add_window_view(|_, cx| center(cx).with_style(|e| e.w(px(320.))));
        tick(w, 16);
        let chevron = w.debug_bounds("center-chevron-0").unwrap().center();
        w.simulate_click(chevron, Modifiers::default());
        tick(w, 20);
        assert_eq!(
            w.debug_bounds("center-detail").unwrap().size.width,
            px(302.)
        );
        w.update(|_, cx| {
            let alpha = view.read(cx).modal_fade.value(cx);
            assert!((0.01..0.99).contains(&alpha));
        });
        tick(w, 600);
        let b = w.debug_bounds("center-dial").unwrap();
        assert_eq!(b.size.width, px(150.));
        let from = polar(b.center(), 68., TAU * 0.95);
        let to = polar(b.center(), 68., TAU * 0.10);
        w.simulate_mouse_down(from, MouseButton::Left, Modifiers::default());
        w.simulate_mouse_move(to, MouseButton::Left, Modifiers::default());
        w.update(|_, cx| assert!((65..=71).contains(&view.read(cx).draft.minutes)));
        w.simulate_mouse_up(to, MouseButton::Left, Modifiers::default());
        tick(w, 16);
        w.update(|_, cx| {
            let v = view.read(cx);
            assert_eq!(v.draft.minutes, 70);
            assert_eq!(v.dial.value(cx), 70.);
            assert!(v.state().focus.is_none());
        });
        for (phase, position) in [
            (TouchPhase::Started, from),
            (TouchPhase::Moved, to),
            (TouchPhase::Cancelled, to),
        ] {
            w.simulate_event(TouchEvent {
                id: TouchId(9),
                phase,
                position,
                ..Default::default()
            });
            tick(w, 16);
        }
        assert_eq!(w.update(|_, cx| view.read(cx).draft.minutes), 70);
    }

    #[test]
    fn notification_focus_is_trapped_and_master_keeps_channel_choices() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) = app.add_window_view(|_, cx| center(cx));
        tick(w, 0);
        click(w, "center-chevron-1");
        assert!(w.update(|w, cx| view.read(cx).focus.close.is_focused(w)));
        press(w, "shift-tab");
        assert!(w.update(|w, cx| view.read(cx).focus.switches[2].is_focused(w)));
        press(w, "tab");
        assert!(w.update(|w, cx| view.read(cx).focus.close.is_focused(w)));
        click(w, "center-switch-0");
        click(w, "center-switch-2");
        press(w, "escape");
        w.update(|w, cx| {
            let v = view.read(cx);
            assert!(v.focus.chevrons[1].is_focused(w));
            assert!(!v.state().notifications);
            assert!(v.state().channels["mentions"]);
            assert!(v.state().channels["deploys"]);
        });
    }
}
