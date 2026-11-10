//! Choose one moment across time zones with a rotary dial.
use crate::foundation::{Theme, ThemeMode};
use crate::primitives::{
    control::UiLanguage,
    direct_touch::Contact,
    family::{Paint, Slide, disabled_motion, measure},
    glyph::{Glyph, transformed},
    presentation::ChangingText,
};
use gpui::{prelude::*, *};
use std::{
    cell::Cell,
    f32::consts::{FRAC_PI_2, TAU},
    rc::Rc,
};
use web_time::Instant;

const DAY: i64 = 1440;
const QUARTER: i64 = 15;
const DIAMETER: f32 = 256.;
const DEFAULT_HOURS: (u16, u16) = (540, 1080);
const FLICK_LOOKAHEAD: f64 = 0.15;

/// One office with a host-supplied, date-correct offset from UTC.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimeDialCity {
    pub id: SharedString,
    pub name: SharedString,
    pub code: SharedString,
    pub offset_minutes: i32,
}

impl TimeDialCity {
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        code: impl Into<SharedString>,
        offset_minutes: i32,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            code: code.into(),
            offset_minutes,
        }
    }
}

/// Display a twelve- or twenty-four-hour clock.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TimeDialCycle {
    #[default]
    Twelve,
    TwentyFour,
}

/// Destination in UTC minutes, emitted when the dial is released or stepped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimeDialValueChanged(pub i32);

/// Stable ID of the city selected by the user.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimeDialHomeChanged(pub SharedString);

#[derive(Clone, Copy)]
struct Drag {
    last_angle: f64,
    raw: f64,
    velocity: f64,
    last_sample: Instant,
}

struct CityMotion {
    radius: Slide,
    selected: Slide,
    light: Slide,
    clock: ClockText,
}

struct ClockText {
    digits: [ChangingText; 4],
    meridiem: ChangingText,
    minute: Option<i64>,
}

impl ClockText {
    fn new() -> Self {
        Self {
            digits: std::array::from_fn(|_| ChangingText::new("")),
            meridiem: ChangingText::new(""),
            minute: None,
        }
    }

    fn render(
        &mut self,
        minute: i64,
        cycle: TimeDialCycle,
        em: f32,
        animate: bool,
        w: &Window,
        cx: &App,
    ) -> Div {
        let (digits, meridiem) = clock_parts(minute, cycle);
        let direction = self.minute.map_or(1., |old| (minute - old).signum() as f32);
        let first = self.minute.is_none();
        self.minute = Some(minute);
        let width = measure(&"0".into(), em, w);
        let mut clock = div()
            .flex()
            .items_end()
            .flex_none()
            .font_weight(FontWeight::MEDIUM)
            .whitespace_nowrap()
            .text_size(px(em))
            .line_height(px(em * 1.22));
        for (index, (digit, wheel)) in digits.into_iter().zip(&mut self.digits).enumerate() {
            if index == 2 {
                clock = clock.child(":");
            }
            let value: SharedString = if digit == b' ' {
                "".into()
            } else {
                char::from(digit).to_string().into()
            };
            let place = div().w(px(width)).h(px(em * 1.22)).overflow_hidden();
            if first || !animate {
                *wheel = ChangingText::new(value.clone());
            }
            clock = clock.child(place.child(if animate {
                wheel.set(value, direction, cx);
                wheel.render(em, w, cx)
            } else {
                div().child(value)
            }));
        }
        if cycle == TimeDialCycle::Twelve {
            if first || !animate {
                self.meridiem = ChangingText::new(meridiem);
            }
            let suffix = div()
                .w(px(measure(&"PM".into(), em * 0.54, w) + 2.))
                .ml(px(3.))
                .pb(px(em * 0.05))
                .text_size(px(em * 0.54))
                .line_height(px(em * 0.72))
                .overflow_hidden();
            clock = clock.child(suffix.child(if animate {
                self.meridiem.set(meridiem, direction, cx);
                self.meridiem.render(em * 0.54, w, cx)
            } else {
                div().child(meridiem)
            }));
        }
        clock
    }
}

fn clock_parts(minute: i64, cycle: TimeDialCycle) -> ([u8; 4], &'static str) {
    let local = minute.rem_euclid(DAY);
    let hour = local / 60;
    let (hour, meridiem) = match cycle {
        TimeDialCycle::Twelve => ((hour + 11) % 12 + 1, if hour < 12 { "AM" } else { "PM" }),
        TimeDialCycle::TwentyFour => (hour, ""),
    };
    let min = local % 60;
    let digits = [
        if hour < 10 && cycle == TimeDialCycle::Twelve {
            b' '
        } else {
            b'0' + (hour / 10) as u8
        },
        b'0' + (hour % 10) as u8,
        b'0' + (min / 10) as u8,
        b'0' + (min % 10) as u8,
    ];
    (digits, meridiem)
}

fn clock_label(minute: i64, cycle: TimeDialCycle) -> String {
    let (digits, suffix) = clock_parts(minute, cycle);
    let mut text = String::with_capacity(8);
    if digits[0] != b' ' {
        text.push(char::from(digits[0]));
    }
    text.push(char::from(digits[1]));
    text.push(':');
    text.push(char::from(digits[2]));
    text.push(char::from(digits[3]));
    if !suffix.is_empty() {
        text.push(' ');
        text.push_str(suffix);
    }
    text
}

fn working(minute: i64, hours: (u16, u16)) -> bool {
    let (start, end) = (i64::from(hours.0), i64::from(hours.1));
    let minute = minute.rem_euclid(DAY);
    if start <= end {
        (start..end).contains(&minute)
    } else {
        minute >= start || minute < end
    }
}

fn daylight(minute: i64) -> bool {
    (360..1080).contains(&minute.rem_euclid(DAY))
}

fn marker_radius(city: usize, home: usize) -> f32 {
    if city == home {
        106.
    } else {
        88. - 11. * (city - usize::from(city > home)) as f32
    }
}

fn bounded(value: i64) -> i32 {
    value.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

fn snapped(value: f64) -> i64 {
    (value / QUARTER as f64).round() as i64 * QUARTER
}

fn angle_delta(next: f64, previous: f64) -> f64 {
    (next - previous + DAY as f64 / 2.).rem_euclid(DAY as f64) - DAY as f64 / 2.
}

fn rubber(value: f64, min: i32, max: i32) -> f64 {
    let edge = value.clamp(f64::from(min), f64::from(max));
    let extra = value - edge;
    edge + extra / (1. + extra.abs() / 72.)
}

fn cursor_minutes(position: Point<Pixels>, bounds: Bounds<Pixels>) -> Option<f64> {
    let dx = f64::from(f32::from(position.x - bounds.center().x));
    let dy = f64::from(f32::from(position.y - bounds.center().y));
    (dx.hypot(dy) > f64::from(f32::from(bounds.size.width)) * 0.17).then(|| {
        (dx.atan2(-dy) / std::f64::consts::TAU * DAY as f64 + DAY as f64 / 2.)
            .rem_euclid(DAY as f64)
    })
}

type HostVisibility = Box<dyn Fn(&App) -> bool>;
type RefineStyle = Box<dyn Fn(Div) -> Div>;

/// A 24-hour dial with local clocks, an office-hours band and quarter-hour detents.
pub struct TimeDial {
    id: ElementId,
    cities: Vec<TimeDialCity>,
    home: usize,
    home_controlled: bool,
    value: i32,
    value_controlled: bool,
    min: i32,
    max: i32,
    weekday: u8,
    cycle: TimeDialCycle,
    hours: (u16, u16),
    label: SharedString,
    focus: FocusHandle,
    stage: Rc<Cell<Bounds<Pixels>>>,
    width: Rc<Cell<Pixels>>,
    contact: Contact,
    drag: Option<Drag>,
    origin: i32,
    position: Slide,
    city_motion: Vec<CityMotion>,
    center: ClockText,
    visibility: Option<HostVisibility>,
    style: Option<RefineStyle>,
}

impl EventEmitter<TimeDialValueChanged> for TimeDial {}
impl EventEmitter<TimeDialHomeChanged> for TimeDial {}

impl TimeDial {
    /// Accept up to five cities, retaining the first occurrence of each stable ID.
    pub fn new(cities: Vec<TimeDialCity>, cx: &mut Context<Self>) -> Self {
        let mut unique = Vec::<TimeDialCity>::with_capacity(5);
        for city in cities {
            if unique.iter().all(|existing| existing.id != city.id) {
                unique.push(city);
                if unique.len() == 5 {
                    break;
                }
            }
        }
        let city_motion = unique
            .iter()
            .enumerate()
            .map(|(i, city)| CityMotion {
                radius: Slide::new(marker_radius(i, 0)),
                selected: Slide::new(f32::from(i == 0)),
                light: Slide::new(f32::from(daylight(i64::from(city.offset_minutes)))),
                clock: ClockText::new(),
            })
            .collect();
        Self {
            id: "time-dial".into(),
            cities: unique,
            home: 0,
            home_controlled: false,
            value: 0,
            value_controlled: false,
            min: i32::MIN,
            max: i32::MAX,
            weekday: 1,
            cycle: TimeDialCycle::Twelve,
            hours: DEFAULT_HOURS,
            label: UiLanguage::current(cx).text("Time", "时间").into(),
            focus: cx.focus_handle().tab_stop(true),
            stage: Default::default(),
            width: Default::default(),
            contact: Contact::default(),
            drag: None,
            origin: 0,
            position: Slide::new(0.),
            city_motion,
            center: ClockText::new(),
            visibility: None,
            style: None,
        }
    }

    pub fn with_id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = id.into();
        self
    }

    pub fn with_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = label.into();
        self
    }

    pub fn with_default_value(mut self, value: i32) -> Self {
        self.value = value.clamp(self.min, self.max);
        self.origin = self.value;
        self.position.reset(0.);
        for (city, marker) in self.cities.iter().zip(&mut self.city_motion) {
            marker.light.reset(f32::from(daylight(
                i64::from(self.value) + i64::from(city.offset_minutes),
            )));
        }
        self
    }

    /// Use a controlled value and accept or replace proposals with `set_value`.
    pub fn with_value(mut self, value: i32) -> Self {
        self.value_controlled = true;
        self.with_default_value(value)
    }

    pub fn with_default_home(mut self, id: &str) -> Self {
        if let Some(home) = self.city_index(id) {
            self.home = home;
            for (i, marker) in self.city_motion.iter_mut().enumerate() {
                marker.radius.reset(marker_radius(i, home));
                marker.selected.reset(f32::from(i == home));
            }
        }
        self
    }

    /// Use a controlled home city and accept proposals with `set_home`.
    pub fn with_home(mut self, id: &str) -> Self {
        self.home_controlled = true;
        self.with_default_home(id)
    }

    /// Limit the UTC minute range. Either endpoint may be unbounded.
    pub fn with_bounds(mut self, min: Option<i32>, max: Option<i32>) -> Self {
        let (min, max) = (min.unwrap_or(i32::MIN), max.unwrap_or(i32::MAX));
        (self.min, self.max) = (min.min(max), max.max(min));
        let value = self.value;
        self.with_default_value(value)
    }

    /// Set the UTC reference day, where 0 is Sunday.
    pub fn with_weekday(mut self, weekday: u8) -> Self {
        self.weekday = weekday % 7;
        self
    }

    pub fn with_hour_cycle(mut self, cycle: TimeDialCycle) -> Self {
        self.cycle = cycle;
        self
    }

    /// Set a half-open local interval. An end before the start wraps through midnight.
    pub fn with_working_hours(mut self, start: u16, end: u16) -> Self {
        self.hours = (start.min(1440), end.min(1440));
        self
    }

    pub fn with_style(mut self, refine: impl Fn(Div) -> Div + 'static) -> Self {
        self.style = Some(Box::new(refine));
        self
    }

    /// Supply visibility for hosts that hide a live preview without closing its window.
    pub fn with_visibility(mut self, visible: impl Fn(&App) -> bool + 'static) -> Self {
        self.visibility = Some(Box::new(visible));
        self
    }

    pub fn value(&self) -> i32 {
        self.value
    }

    pub fn home(&self) -> Option<&TimeDialCity> {
        self.cities.get(self.home)
    }

    /// Apply a UTC value from the host without emitting an input event.
    pub fn set_value(&mut self, value: i32, cx: &mut Context<Self>) {
        let value = value.clamp(self.min, self.max);
        if value != self.value {
            self.drag = None;
            self.value = value;
            self.land(value, 0., cx);
            cx.notify();
        }
    }

    /// Apply a known home city without altering the selected UTC instant.
    pub fn set_home(&mut self, id: &str, cx: &mut Context<Self>) {
        if let Some(home) = self.city_index(id)
            && self.home != home
        {
            self.home = home;
            cx.notify();
        }
    }

    fn city_index(&self, id: &str) -> Option<usize> {
        self.cities.iter().position(|city| city.id.as_ref() == id)
    }

    fn local(&self, value: i32) -> i64 {
        i64::from(value) + i64::from(self.home().map_or(0, |c| c.offset_minutes))
    }

    fn land(&mut self, value: i32, velocity: f32, cx: &App) {
        // Rebase the spring to the destination without losing its current phase.
        let delta = f64::from(self.origin) - f64::from(value);
        if delta.abs() > (DAY * 2) as f64 {
            let visual = f64::from(self.position.value(cx)) + delta;
            self.position.reset(angle_delta(visual, 0.) as f32);
        } else {
            self.position.translate(delta as f32);
        }
        self.origin = value;
        if velocity.abs() > 1. {
            self.position.release(0., velocity, 0.50, cx);
        } else {
            self.position.duration(0., 0.36, cx);
        }
    }

    fn commit(&mut self, value: i64, velocity: f32, cx: &mut Context<Self>) {
        let value = bounded(value).clamp(self.min, self.max);
        if self.value != value {
            if !self.value_controlled {
                self.value = value;
            }
            cx.emit(TimeDialValueChanged(value));
        }
        self.land(self.value, velocity, cx);
        cx.notify();
    }

    fn choose_home(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.home != index {
            let id = self.cities[index].id.clone();
            if !self.home_controlled {
                self.home = index;
            }
            cx.emit(TimeDialHomeChanged(id));
            cx.notify();
        }
    }

    fn begin(&mut self, point: Point<Pixels>, w: &mut Window, cx: &mut Context<Self>) {
        self.focus.focus(w, cx);
        let bounds = self.stage.get();
        let Some(angle) = cursor_minutes(point, bounds) else {
            return;
        };
        if self.drag.is_some() {
            return;
        }
        let start = f64::from(self.origin) + f64::from(self.position.value(cx));
        let local = start + f64::from(self.home().map_or(0, |c| c.offset_minutes));
        let raw = start + angle_delta(angle, local);
        self.drag = Some(Drag {
            last_angle: angle,
            raw,
            velocity: 0.,
            last_sample: Instant::now(),
        });
        self.show_drag(raw, cx);
    }

    fn show_drag(&mut self, raw: f64, cx: &mut Context<Self>) {
        let detent = raw + (snapped(raw) as f64 - raw) * 0.32;
        self.position
            .reset((rubber(detent, self.min, self.max) - f64::from(self.origin)) as f32);
        cx.notify();
    }

    fn drag_to(&mut self, point: Point<Pixels>, cx: &mut Context<Self>) {
        let Some(angle) = cursor_minutes(point, self.stage.get()) else {
            return;
        };
        let Some(mut drag) = self.drag else {
            return;
        };
        let delta = angle_delta(angle, drag.last_angle);
        if delta.abs() < 0.002 {
            return;
        }
        let now = Instant::now();
        let seconds = now
            .saturating_duration_since(drag.last_sample)
            .as_secs_f64()
            .max(0.008);
        let sample = delta / seconds;
        drag.velocity = if seconds > 0.10 {
            sample
        } else {
            drag.velocity * 0.36 + sample * 0.64
        };
        drag.raw += delta;
        drag.last_angle = angle;
        drag.last_sample = now;
        self.drag = Some(drag);
        self.show_drag(drag.raw, cx);
    }

    fn end(&mut self, point: Point<Pixels>, cancelled: bool, cx: &mut Context<Self>) {
        if !cancelled {
            self.drag_to(point, cx);
        }
        let Some(drag) = self.drag.take() else {
            return;
        };
        if cancelled {
            self.land(self.value, 0., cx);
            cx.notify();
            return;
        }
        let velocity = if disabled_motion(cx) || drag.last_sample.elapsed().as_secs_f64() > 0.09 {
            0.
        } else {
            let max_velocity = DAY as f64 / FLICK_LOOKAHEAD;
            drag.velocity.clamp(-max_velocity, max_velocity)
        };
        let destination = snapped(drag.raw + velocity * FLICK_LOOKAHEAD);
        self.commit(destination, velocity as f32, cx);
    }

    fn on_key(&mut self, key: &str, w: &mut Window, cx: &mut Context<Self>) {
        let value = i64::from(self.value);
        let local = self.local(self.value);
        let minute = local.rem_euclid(DAY);
        let next = match key {
            "right" | "up" => value + QUARTER,
            "left" | "down" => value - QUARTER,
            "pageup" => value + 60,
            "pagedown" => value - 60,
            "home" => value - minute,
            "end" => value - minute + DAY - QUARTER,
            _ => return,
        };
        self.drag = None;
        self.commit(next, 0., cx);
        w.prevent_default();
        cx.stop_propagation();
    }
}

fn week_label(weekday: u8, local: i64, lang: UiLanguage) -> &'static str {
    let index = (i64::from(weekday) + local.div_euclid(DAY)).rem_euclid(7) as usize;
    match lang {
        UiLanguage::English => [
            "Sunday",
            "Monday",
            "Tuesday",
            "Wednesday",
            "Thursday",
            "Friday",
            "Saturday",
        ][index],
        UiLanguage::Chinese => ["周日", "周一", "周二", "周三", "周四", "周五", "周六"][index],
    }
}

fn utc_offset(minutes: i32) -> String {
    let sign = if minutes < 0 { "−" } else { "+" };
    let minutes = i64::from(minutes).abs();
    if minutes % 60 == 0 {
        format!("UTC{sign}{}", minutes / 60)
    } else {
        format!("UTC{sign}{}:{:02}", minutes / 60, minutes % 60)
    }
}

fn difference(minutes: i64, lang: UiLanguage) -> String {
    if minutes == 0 {
        return lang.text("same time", "同一时区").to_owned();
    }
    let sign = if minutes < 0 {
        lang.text("behind", "晚")
    } else {
        lang.text("ahead", "早")
    };
    let (hours, remainder) = (minutes.abs() / 60, minutes.abs() % 60);
    match (lang, remainder) {
        (UiLanguage::English, 0) => format!("{hours} h {sign}"),
        (UiLanguage::English, _) => format!("{hours} h {remainder} min {sign}"),
        (UiLanguage::Chinese, 0) => format!("{sign} {hours} 小时"),
        (UiLanguage::Chinese, _) => format!("{sign} {hours} 小时 {remainder} 分"),
    }
}

#[derive(Clone, Copy)]
struct DialGeometry {
    center: Point<Pixels>,
    scale: f32,
}

impl DialGeometry {
    fn at(self, minute: f32, radius: f32) -> Point<Pixels> {
        let angle = FRAC_PI_2 + minute / DAY as f32 * TAU;
        let (sine, cosine) = angle.sin_cos();
        self.center
            + point(
                px(cosine * radius * self.scale),
                px(sine * radius * self.scale),
            )
    }

    fn circle(self, radius: f32, color: Hsla, width: f32, border: Hsla, w: &mut Window) {
        let radius = px(radius * self.scale);
        w.paint_quad(quad(
            Bounds::new(
                self.center - point(radius, radius),
                size(radius * 2., radius * 2.),
            ),
            radius,
            color,
            px(width * self.scale),
            border,
            BorderStyle::Solid,
        ));
    }

    fn sector(self, hours: (u16, u16), color: Hsla, w: &mut Window) {
        let start = f32::from(hours.0);
        let end = f32::from(hours.1);
        let span = if start <= end {
            end - start
        } else {
            DAY as f32 - start + end
        };
        if span <= 0. {
            return;
        }
        let segments = (span / 15.).ceil() as usize;
        let sector = |mut path: PathBuilder| {
            path.move_to(self.at(start, 130.));
            for i in 1..=segments {
                path.line_to(self.at(start + span * i as f32 / segments as f32, 130.));
            }
            for i in (0..=segments).rev() {
                path.line_to(self.at(start + span * i as f32 / segments as f32, 60.));
            }
            path.close();
            path.build()
        };
        if let Ok(path) = sector(PathBuilder::fill()) {
            w.paint_path(path, color.opacity(0.09));
        }
        if let Ok(path) = sector(PathBuilder::stroke(px(0.75 * self.scale))) {
            w.paint_path(path, color.opacity(0.22));
        }
    }

    fn line(self, from: Point<Pixels>, to: Point<Pixels>, width: f32, color: Hsla, w: &mut Window) {
        let mut path = PathBuilder::stroke(px(width * self.scale));
        path.move_to(from);
        path.line_to(to);
        if let Ok(path) = path.build() {
            w.paint_path(path, color);
        }
    }

    fn face(
        self,
        p: Paint,
        hours: (u16, u16),
        count: usize,
        local: f32,
        hand: f32,
        w: &mut Window,
    ) {
        let clear = transparent_black();
        self.circle(146., p.panel, 1., p.line, w);
        self.sector(hours, p.success, w);
        self.circle(106., clear, 1., p.line, w);
        for ring in 0..count.saturating_sub(1) {
            let mut path = PathBuilder::stroke(px(0.8 * self.scale));
            let radius = 88. - 11. * ring as f32;
            let dots = (radius * TAU / 6.).round() as usize;
            let dash = DAY as f32 / (radius * TAU);
            for dot in 0..dots {
                let minute = dot as f32 / dots as f32 * DAY as f32;
                path.move_to(self.at(minute, radius));
                path.line_to(self.at(minute + dash, radius));
            }
            if let Ok(path) = path.build() {
                w.paint_path(path, p.faint);
            }
        }
        self.circle(56., p.panel, 0.65, p.line, w);
        let mut ticks = PathBuilder::stroke(px(self.scale));
        for hour in 0..24 {
            let minute = hour as f32 * 60.;
            if hour % 6 == 0 {
                self.line(
                    self.at(minute, 141.),
                    self.at(minute, 133.),
                    1.1,
                    p.secondary,
                    w,
                );
            } else {
                ticks.move_to(self.at(minute, 141.));
                ticks.line_to(self.at(minute, 137.));
            }
        }
        if let Ok(path) = ticks.build() {
            w.paint_path(path, p.faint.opacity(0.48));
        }
        self.line(self.at(local, 56.), self.at(local, hand), 1.35, p.ink, w);
    }
}

fn positioned_label(
    text: impl Into<SharedString>,
    at: Point<Pixels>,
    anchor: f32,
    em: f32,
    color: Hsla,
    w: &Window,
) -> Div {
    let text = text.into();
    div()
        .absolute()
        .left(at.x - px(measure(&text, em, w) * anchor))
        .top(at.y - px(em * 0.6))
        .text_size(px(em))
        .line_height(px(em * 1.2))
        .whitespace_nowrap()
        .text_color(color)
        .child(text)
}

fn city_icon(side: f32, day: f32, color: Hsla, animate: bool) -> Div {
    let tilt = if animate { 0.58 } else { 0. };
    div()
        .relative()
        .flex_none()
        .size(px(side))
        .child(div().absolute().inset_0().child(transformed(
            Glyph::Sun,
            side,
            color.opacity(day),
            tilt * (1. - day),
            0.8 + 0.2 * day,
            (0., 0.),
            1.,
        )))
        .child(div().absolute().inset_0().child(transformed(
            Glyph::Moon,
            side,
            color.opacity(1. - day),
            -tilt * day,
            1. - 0.2 * day,
            (0., 0.),
            1.,
        )))
}

impl Render for TimeDial {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let width = self.width.clone();
        let mut p = Paint::get(cx);
        let dark = Theme::global(cx).mode == ThemeMode::Dark;
        p.panel = rgb(if dark { 0x1f1f1f } else { 0xffffff }).into();
        p.line = rgb(if dark { 0x2b2b2b } else { 0xe9e9e9 }).into();
        p.success = rgb(if dark { 0x55d69f } else { 0x00774d }).into();
        let track: Hsla = rgb(if dark { 0x484848 } else { 0xc4c4c4 }).into();
        let selected_row: Hsla = rgb(if dark { 0x242424 } else { 0xf8f8f8 }).into();
        let lang = UiLanguage::current(cx);
        let measured = width.get();
        let root_width = if measured > px(0.) {
            measured
        } else {
            w.viewport_size().width.min(px(720.))
        };
        let wide = root_width >= px(600.);
        let side = f32::from(root_width).clamp(0., DIAMETER);
        let shell = div()
            .w_full()
            .max_w(px(720.))
            .flex()
            .items_center()
            .gap(px(20.))
            .when(wide, |d| d.flex_row().gap(px(28.)))
            .when(!wide, |d| d.flex_col());
        let shell = match &self.style {
            Some(refine) => refine(shell),
            None => shell,
        };
        let root = shell
            .on_children_prepainted(move |children, w, _| {
                if let Some(bounds) = children.last()
                    && width.replace(bounds.size.width) != bounds.size.width
                {
                    w.request_animation_frame();
                }
            })
            .id(self.id.clone())
            .debug_selector(|| "time-dial-root".into());
        let Some(home) = self.home().cloned() else {
            return root;
        };
        let animate = w.is_visible()
            && !disabled_motion(cx)
            && self.visibility.as_ref().is_none_or(|visible| visible(cx));
        let mut motion_active = false;
        let now = cx.background_executor().now();
        let (offset, moving) = self.position.sample_batch(now, !animate);
        motion_active |= moving;
        let visual = f64::from(self.origin) + f64::from(offset);
        let minute = if self.drag.is_some() || moving {
            snapped(visual)
        } else {
            i64::from(self.value)
        };
        let local = minute + i64::from(home.offset_minutes);
        let local_precise =
            ((visual + f64::from(home.offset_minutes)).rem_euclid(DAY as f64)) as f32;
        let committed_local = self.local(self.value);
        let day = week_label(self.weekday, local, lang);
        let time = clock_label(committed_local, self.cycle);
        let word = week_label(self.weekday, committed_local, lang);
        let day_base = committed_local - committed_local.rem_euclid(DAY);
        let min =
            (i64::from(self.min) + i64::from(home.offset_minutes) - day_base).clamp(0, DAY - 1);
        let max =
            (i64::from(self.max) + i64::from(home.offset_minutes) - day_base).clamp(0, DAY - 1);
        let face_geometry = DialGeometry {
            center: point(px(side / 2.), px(side / 2.)),
            scale: side / 300.,
        };
        let mut stages = Vec::with_capacity(self.cities.len());
        for (index, (city, state)) in self.cities.iter().zip(&mut self.city_motion).enumerate() {
            let is_home = index == self.home;
            let city_minute = minute + i64::from(city.offset_minutes);
            let light = f32::from(daylight(city_minute));
            state
                .radius
                .duration(marker_radius(index, self.home), 0.38, cx);
            state.selected.duration(f32::from(is_home), 0.31, cx);
            state.light.duration(light, 0.24, cx);
            let (radius, a) = state.radius.sample_batch(now, !animate);
            let (selected, b) = state.selected.sample_batch(now, !animate);
            let (light, c) = state.light.sample_batch(now, !animate);
            motion_active |= a || b || c;
            let phase = (visual + f64::from(city.offset_minutes)).rem_euclid(DAY as f64) as f32;
            stages.push((
                radius,
                selected.clamp(0., 1.),
                light.clamp(0., 1.),
                phase,
                city_minute,
            ));
        }
        if motion_active && animate {
            w.request_animation_frame();
        }

        let stage = self.stage.clone();
        let hours = self.hours;
        let count = self.cities.len();
        let hand = stages[self.home].0;
        let paint = canvas(
            move |b, _, _| stage.set(b),
            move |bounds, _, w, _| {
                let mut p = p;
                p.faint = track;
                DialGeometry {
                    center: bounds.center(),
                    scale: f32::from(bounds.size.width) / 300.,
                }
                .face(p, hours, count, local_precise, hand, w);
            },
        )
        .absolute()
        .inset_0();
        let focus = p.focus;
        let mut face = div()
            .id("time-dial-stage")
            .debug_selector(|| "time-dial-stage".into())
            .relative()
            .size(px(side))
            .flex_none()
            .rounded_full()
            .track_focus(&self.focus)
            .tab_index(0)
            .role(Role::Slider)
            .aria_label(self.label.clone())
            .aria_value(format!("{time}, {word}, {}", home.name))
            .aria_numeric_value(committed_local.rem_euclid(DAY) as f64)
            .aria_min_numeric_value(min as f64)
            .aria_max_numeric_value(
                max.min(DAY - QUARTER).max(committed_local.rem_euclid(DAY)) as f64
            )
            .aria_numeric_value_step(QUARTER as f64)
            .cursor(if self.drag.is_some() {
                CursorStyle::ClosedHand
            } else {
                CursorStyle::OpenHand
            })
            .focus_visible(move |d| {
                d.shadow(vec![
                    BoxShadow::new(px(0.), px(0.), focus).spread_radius(px(2.)),
                ])
            })
            .on_key_down(cx.listener(|s, event: &KeyDownEvent, w, cx| {
                s.on_key(&event.keystroke.key, w, cx);
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|s, e: &MouseDownEvent, w, cx| {
                    s.begin(e.position, w, cx);
                }),
            )
            .on_mouse_move(cx.listener(|s, e: &MouseMoveEvent, _, cx| s.drag_to(e.position, cx)))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|s, e: &MouseUpEvent, _, cx| {
                    s.end(e.position, false, cx);
                }),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|s, e: &MouseUpEvent, _, cx| {
                    s.end(e.position, false, cx);
                }),
            )
            .child(paint);

        let numeral = 9.8 * face_geometry.scale;
        let numerals = [
            (0., 127., 0.5, lang.text("12 AM", "午夜")),
            (360., 130., 0., lang.text("6 AM", "早 6 点")),
            (720., 127., 0.5, lang.text("Noon", "正午")),
            (1080., 130., 1., lang.text("6 PM", "晚 6 点")),
        ];
        for (time, radius, anchor, text) in numerals {
            let text = if self.cycle == TimeDialCycle::TwentyFour {
                format!("{:02}:00", time as i32 / 60).into()
            } else {
                SharedString::from(text)
            };
            face = face.child(positioned_label(
                text,
                face_geometry.at(time, radius),
                anchor,
                numeral,
                p.faint,
                w,
            ));
        }
        // Keep dots behind the active knob as cities trade tracks.
        for i in (0..count)
            .filter(|&i| i != self.home)
            .chain(std::iter::once(self.home))
        {
            let city = &self.cities[i];
            let (radius, selected, light, phase, city_minute) = stages[i];
            let at = face_geometry.at(phase, radius);
            let dot = px((10. + selected * 22.) * face_geometry.scale);
            let on = working(city_minute, self.hours);
            let marker = if on { p.success } else { p.secondary };
            let fill = (if on { p.success } else { p.panel }).blend(p.ink.opacity(selected));
            face = face.child(
                div()
                    .absolute()
                    .left(at.x - dot / 2.)
                    .top(at.y - dot / 2.)
                    .size(dot)
                    .rounded_full()
                    .bg(fill)
                    .border_1()
                    .border_color(marker.opacity(1. - selected))
                    .flex()
                    .items_center()
                    .justify_center()
                    .when(selected > 0.01, |d| {
                        d.child(city_icon(
                            14. * face_geometry.scale,
                            light,
                            p.panel.opacity(selected),
                            animate,
                        ))
                    }),
            );
            if selected < 0.999 {
                let label_at = face_geometry.at(phase, radius - 14.);
                face = face.child(positioned_label(
                    city.code.clone(),
                    label_at,
                    0.5,
                    numeral,
                    p.secondary.opacity(1. - selected),
                    w,
                ));
            }
        }
        let center_width = side * 0.42;
        let center = div()
            .absolute()
            .left(px((side - center_width) * 0.5))
            .top(px(side * 0.365))
            .w(px(center_width))
            .flex()
            .flex_col()
            .items_center()
            .gap(px(side * 0.007))
            .child(
                div()
                    .text_size(px(side * 0.044))
                    .line_height(px(side * 0.052))
                    .text_color(p.secondary)
                    .truncate()
                    .max_w_full()
                    .child(home.name.clone()),
            )
            .child(
                self.center
                    .render(local, self.cycle, side * 0.10, animate, w, cx)
                    .text_color(p.ink),
            )
            .child(
                div()
                    .text_size(px(side * 0.039))
                    .line_height(px(side * 0.06))
                    .text_color(p.faint)
                    .child(day),
            );
        face = face.child(center).child(self.contact.listen(
            cx,
            |s, event, w, cx| match event.phase {
                TouchPhase::Started => s.begin(event.position, w, cx),
                TouchPhase::Moved => s.drag_to(event.position, cx),
                TouchPhase::Ended => s.end(event.position, false, cx),
                TouchPhase::Cancelled => s.end(event.position, true, cx),
            },
        ));

        let list_width = if wide {
            root_width - px(side + 28.)
        } else {
            root_width.min(px(420.))
        };
        let tight = list_width < px(360.);
        let mut list = div()
            .id("time-dial-cities")
            .flex()
            .flex_col()
            .gap(px(2.))
            .w(list_width)
            .min_w_0()
            .role(Role::List)
            .aria_label(lang.text("Cities", "城市"));
        for (index, (city, state)) in self.cities.iter().zip(&mut self.city_motion).enumerate() {
            let (_, selected, light, _, city_minute) = stages[index];
            let active = index == self.home;
            let on = working(city_minute, self.hours);
            let delta = i64::from(city.offset_minutes) - i64::from(home.offset_minutes);
            let delta_day = city_minute.div_euclid(DAY) - local.div_euclid(DAY);
            let offset = utc_offset(city.offset_minutes);
            let relative = if active {
                lang.text("Home", "主城市").to_owned()
            } else {
                difference(delta, lang)
            };
            let note = if active {
                format!("{relative} · {offset}")
            } else {
                format!("{offset} · {relative}")
            };
            let suffix = if delta_day == 0 {
                String::new()
            } else if lang == UiLanguage::Chinese {
                format!("{delta_day:+} 天")
            } else if delta_day.abs() == 1 {
                format!("{delta_day:+} day")
            } else {
                format!("{delta_day:+} days")
            };
            let chip = || {
                div()
                    .flex_none()
                    .px(px(6.))
                    .text_size(px(11.))
                    .line_height(px(18.))
                    .rounded(px(4.))
                    .bg(p.hover)
                    .text_color(p.secondary)
                    .child(suffix.clone())
            };
            let row_focus = p.focus;
            let label = format!(
                "{}, {}, {}{}{}",
                city.name,
                clock_label(city_minute, self.cycle),
                relative,
                if on {
                    lang.text(", working hours", "，工作时段")
                } else {
                    ""
                },
                if delta_day > 0 {
                    lang.text(", next day", "，次日")
                } else if delta_day < 0 {
                    lang.text(", previous day", "，前一日")
                } else {
                    ""
                },
            );
            list = list.child(
                div()
                    .id(("time-dial-city", index))
                    .debug_selector({
                        let name = city.name.clone();
                        move || name.to_string()
                    })
                    .role(Role::Button)
                    .aria_label(label)
                    .aria_toggled(active.into())
                    .tab_index(0)
                    .w_full()
                    .relative()
                    .min_h(px(52.))
                    .px(px(if tight { 12. } else { 16. }))
                    .py(px(7.))
                    .gap(px(if tight { 8. } else { 12. }))
                    .rounded(crate::foundation::surface_radius(cx))
                    .flex()
                    .items_center()
                    .bg(selected_row.opacity(selected))
                    .hover(|d| d.bg(p.hover))
                    .focus_visible(move |d| {
                        d.shadow(vec![
                            BoxShadow::new(px(0.), px(0.), row_focus).spread_radius(px(1.)),
                        ])
                    })
                    .cursor_pointer()
                    .on_click(cx.listener(move |s, _, _, cx| s.choose_home(index, cx)))
                    .when(selected > 0.01, |d| {
                        d.child(
                            div()
                                .absolute()
                                .inset_0()
                                .rounded(crate::foundation::surface_radius(cx))
                                .border_1()
                                .border_color(p.line.opacity(selected)),
                        )
                    })
                    .child(city_icon(
                        16.,
                        light,
                        if on { p.success } else { p.faint },
                        animate,
                    ))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .items_start()
                            .child(
                                div()
                                    .max_w_full()
                                    .truncate()
                                    .text_size(px(14.))
                                    .line_height(px(18.4))
                                    .text_color(p.ink)
                                    .child(city.name.clone()),
                            )
                            .child(
                                div()
                                    .max_w_full()
                                    .truncate()
                                    .text_size(px(12.))
                                    .line_height(px(16.))
                                    .text_color(p.faint)
                                    .child(note),
                            )
                            .when(tight && !suffix.is_empty(), |d| d.child(chip())),
                    )
                    .when(!tight && !suffix.is_empty(), |d| d.child(chip()))
                    .child(
                        state
                            .clock
                            .render(city_minute, self.cycle, 18., animate, w, cx)
                            .text_color(p.ink),
                    ),
            );
        }
        root.child(face)
            .child(list)
            .child(canvas(|_, _, _| (), |_, _, _, _| {}).absolute().inset_0())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DEFAULT_HOURS, DialGeometry, TimeDial, TimeDialCity, TimeDialCycle, TimeDialHomeChanged,
        TimeDialValueChanged, angle_delta, clock_label, cursor_minutes, marker_radius, rubber,
        snapped, working,
    };
    use gpui::{Bounds, Modifiers, MouseButton, TestAppContext, point, px, size};
    use std::{cell::RefCell, rc::Rc};

    fn cities() -> Vec<TimeDialCity> {
        vec![
            TimeDialCity::new("nyc", "New York", "NYC", -240),
            TimeDialCity::new("sf", "San Francisco", "SF", -420),
            TimeDialCity::new("lon", "London", "LON", 60),
            TimeDialCity::new("tyo", "Tokyo", "TYO", 540),
        ]
    }

    #[test]
    fn the_pointer_and_the_drawn_clock_agree_at_each_cardinal_and_across_midnight() {
        let bounds = Bounds::new(point(px(21.), px(40.)), size(px(256.), px(256.)));
        let geometry = DialGeometry {
            center: bounds.center(),
            scale: 256. / 300.,
        };
        for minute in [0., 15., 360., 720., 1080., 1425.] {
            let hit = cursor_minutes(geometry.at(minute, 106.), bounds).unwrap();
            assert!(angle_delta(hit, f64::from(minute)).abs() < 0.001);
        }
        assert_eq!(angle_delta(8., 1430.), 18.);
        assert_eq!(angle_delta(1430., 8.), -18.);
        assert_eq!(snapped(1438.), 1440);
        assert!(cursor_minutes(bounds.center(), bounds).is_none());
        assert_eq!(marker_radius(2, 2), 106.);
        assert_eq!(marker_radius(3, 2), 66.);
    }

    #[test]
    fn work_hours_are_half_open_and_may_wrap_across_days() {
        assert!(working(540, DEFAULT_HOURS));
        assert!(working(1440 + 1079, DEFAULT_HOURS));
        assert!(!working(1080, DEFAULT_HOURS));
        assert!(working(-60, (1320, 360)));
        assert!(working(345, (1320, 360)));
        assert!(!working(360, (1320, 360)));
        assert_eq!(clock_label(-15, TimeDialCycle::Twelve), "11:45 PM");
        assert_eq!(clock_label(1440 + 15, TimeDialCycle::TwentyFour), "00:15");
        assert!((1005. ..1077.).contains(&rubber(1170., 0, 1005)));
    }

    #[test]
    fn keyboard_uses_the_home_day_and_changing_home_preserves_utc() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) =
            app.add_window_view(|_, cx| TimeDial::new(cities(), cx).with_default_value(1260));
        w.update(|w, cx| view.read(cx).focus.clone().focus(w, cx));
        w.simulate_keystrokes("right pageup");
        assert_eq!(w.update(|_, cx| view.read(cx).value()), 1335);
        w.update(|_, cx| view.update(cx, |v, cx| v.choose_home(2, cx)));
        assert_eq!(w.update(|_, cx| view.read(cx).local(1335)), 1395);
        w.simulate_keystrokes("end");
        assert_eq!(w.update(|_, cx| view.read(cx).value()), 1365);
        w.simulate_keystrokes("right");
        assert_eq!(w.update(|_, cx| view.read(cx).value()), 1380);
        w.simulate_keystrokes("home");
        assert_eq!(w.update(|_, cx| view.read(cx).value()), 1380);
    }

    #[test]
    fn drag_submits_once_on_release_and_controlled_authority_stays_with_the_host() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) = app.add_window_view(|_, cx| {
            TimeDial::new(cities(), cx)
                .with_value(1260)
                .with_home("nyc")
        });
        let values = Rc::new(RefCell::new(Vec::new()));
        let homes = Rc::new(RefCell::new(Vec::new()));
        let (_values, _homes) = w.update(|_, cx| {
            let values = values.clone();
            let homes = homes.clone();
            (
                cx.subscribe(&view, move |_, e: &TimeDialValueChanged, _| {
                    values.borrow_mut().push(e.0)
                }),
                cx.subscribe(&view, move |_, e: &TimeDialHomeChanged, _| {
                    homes.borrow_mut().push(e.0.clone())
                }),
            )
        });
        let stage = w.debug_bounds("time-dial-stage").unwrap();
        let noon = point(stage.center().x, stage.top() + px(20.));
        w.simulate_mouse_move(noon, None, Modifiers::default());
        w.simulate_mouse_down(noon, MouseButton::Left, Modifiers::default());
        assert!(values.borrow().is_empty());
        w.simulate_mouse_up(noon, MouseButton::Left, Modifiers::default());
        assert_eq!(*values.borrow(), [960]);
        assert_eq!(w.update(|_, cx| view.read(cx).value()), 1260);
        w.update(|_, cx| {
            view.update(cx, |v, cx| {
                v.set_value(960, cx);
                v.choose_home(2, cx);
            })
        });
        assert_eq!(homes.borrow()[0].as_ref(), "lon");
        assert_eq!(w.update(|_, cx| view.read(cx).local(960)), 720);
        w.update(|_, cx| view.update(cx, |v, cx| v.set_home("lon", cx)));
        assert_eq!(w.update(|_, cx| view.read(cx).local(960)), 1020);
        assert_eq!(*values.borrow(), [960]);
    }

    #[test]
    fn responsive_layout_keeps_city_rows_inside_the_container() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (_, w) = app.add_window_view(|_, cx| TimeDial::new(cities(), cx));
        for (width, wide) in [(720., true), (320., false), (610., true)] {
            w.simulate_resize(size(px(width), px(680.)));
            for _ in 0..2 {
                w.update(|w, _| w.refresh());
                w.run_until_parked();
            }
            let stage = w.debug_bounds("time-dial-stage").unwrap();
            let city = w.debug_bounds("New York").unwrap();
            assert_eq!(stage.size.width, px(256.));
            assert!(city.right() <= px(width + 0.5));
            assert_eq!(city.left() > stage.right(), wide);
            assert_eq!(city.top() >= stage.bottom(), !wide);
        }
    }
}
