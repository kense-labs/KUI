//! Reserve a table through one expanding, keyboard-accessible control.
use crate::primitives::{
    family::{Paint, PhaseClock, Slide, Tone, disabled_motion, ease, face, measure},
    form_chrome::{calendar_mark, row, tr},
    glyph::{Glyph, icon, spinner, transformed},
    presentation::RollingNumber,
};
use chrono::{Datelike, Days, NaiveDate, NaiveTime, Timelike};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc, time::Duration};
use web_time::Instant;

const STEP_SECONDS: f32 = 0.42;
const CELL: f32 = 52.;

/// The visible face of a booking flow.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BookingStep {
    #[default]
    Start,
    Party,
    Date,
    Time,
    Review,
    Booked,
}

impl BookingStep {
    fn previous(self) -> Self {
        match self {
            Self::Start | Self::Party | Self::Booked => Self::Start,
            Self::Date => Self::Party,
            Self::Time => Self::Date,
            Self::Review => Self::Time,
        }
    }

    fn order(self) -> u8 {
        self as u8
    }
}

/// The date and local time that the host receives after confirmation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Booking {
    pub date: NaiveDate,
    pub time: NaiveTime,
    pub party_size: u16,
}

/// An event emitted each time the visible face changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BookingStepChanged(pub BookingStep);

/// Select a twelve- or twenty-four-hour display.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BookingHourCycle {
    #[default]
    Twelve,
    TwentyFour,
}

#[derive(Clone, Copy)]
struct Drag {
    start: Pixels,
    last: Pixels,
    at: Instant,
    origin: f32,
    velocity: f32,
    moved: bool,
}

#[derive(Clone, Copy, Default)]
struct FaceBounds {
    step: BookingStep,
    width: Pixels,
    height: Pixels,
}

type Availability = Box<dyn Fn(NaiveDate, NaiveTime, u16) -> bool>;
type Confirm = Box<dyn Fn(Booking, &mut App) -> Task<Result<(), SharedString>>>;
type Refine = Box<dyn Fn(Div) -> Div>;

/// A bottom-anchored booking flow with host-owned availability and confirmation.
pub struct BookingPill {
    id: ElementId,
    venue: SharedString,
    detail: SharedString,
    label: SharedString,
    start: NaiveDate,
    days: Vec<NaiveDate>,
    times: Vec<NaiveTime>,
    availability: Option<Availability>,
    free: Vec<Vec<bool>>,
    min_party: u16,
    max_party: u16,
    default_party: u16,
    party: u16,
    day: usize,
    time: Option<usize>,
    preferred: NaiveTime,
    cycle: BookingHourCycle,
    step: BookingStep,
    focus: [FocusHandle; 6],
    request_focus: bool,
    generation: u64,
    confirm: Option<Confirm>,
    job: Option<Task<()>>,
    pending: bool,
    error: Option<SharedString>,
    width: Slide,
    height: Slide,
    radius: Slide,
    face: Rc<Cell<FaceBounds>>,
    width_limit: Rc<Cell<Pixels>>,
    direction: f32,
    entered: PhaseClock,
    count: RollingNumber,
    strip: Slide,
    strip_bounds: Rc<Cell<Bounds<Pixels>>>,
    drag: Option<Drag>,
    touch: Rc<Cell<u8>>,
    style: Option<Refine>,
}

impl EventEmitter<BookingStepChanged> for BookingPill {}

fn clock_time(hour: u32, minute: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(hour, minute, 0).unwrap_or_default()
}

impl BookingPill {
    /// Create a booking pill starting on a host-supplied civil date.
    pub fn new(venue: impl Into<SharedString>, start: NaiveDate, cx: &mut Context<Self>) -> Self {
        let mut pill = Self {
            id: "booking-pill".into(),
            venue: venue.into(),
            detail: "".into(),
            label: tr(cx, "Book a table", "预订餐桌").into(),
            start,
            days: Vec::new(),
            times: (34..46).map(|i| clock_time(i / 2, i % 2 * 30)).collect(),
            availability: None,
            free: Vec::new(),
            min_party: 1,
            max_party: 12,
            default_party: 2,
            party: 2,
            day: 0,
            time: None,
            preferred: clock_time(19, 30),
            cycle: BookingHourCycle::Twelve,
            step: BookingStep::Start,
            focus: std::array::from_fn(|_| cx.focus_handle().tab_stop(true)),
            request_focus: false,
            generation: 0,
            confirm: None,
            job: None,
            pending: false,
            error: None,
            width: Slide::new(164.),
            height: Slide::new(52.),
            radius: Slide::new(26.),
            face: Default::default(),
            width_limit: Default::default(),
            direction: 1.,
            entered: Default::default(),
            count: RollingNumber::new(2.).with_initial_roll(false),
            strip: Slide::new(0.),
            strip_bounds: Default::default(),
            drag: None,
            touch: Default::default(),
            style: None,
        };
        pill.load_days(21);
        pill
    }

    pub fn with_id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = id.into();
        self
    }

    pub fn with_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = label.into();
        self
    }

    pub fn with_venue_detail(mut self, detail: impl Into<SharedString>) -> Self {
        self.detail = detail.into();
        self
    }

    /// Set the number of days, from one to 366, starting at the reference date.
    pub fn with_days(mut self, days: usize) -> Self {
        self.load_days(days.clamp(1, 366));
        self
    }

    /// Set ordered local seating times. Duplicate minute values are removed.
    pub fn with_times(mut self, times: Vec<NaiveTime>) -> Self {
        let mut unique = std::collections::HashSet::new();
        self.times = times
            .into_iter()
            .map(|time| clock_time(time.hour(), time.minute()))
            .filter(|time| unique.insert(*time))
            .collect();
        self.reset_selection();
        self
    }

    pub fn with_preferred_time(mut self, time: NaiveTime) -> Self {
        self.preferred = clock_time(time.hour(), time.minute());
        self.time = self.nearest_time(self.day, self.preferred);
        self
    }

    pub fn with_party_range(mut self, min: u16, max: u16) -> Self {
        self.min_party = min.min(max).max(1);
        self.max_party = min.max(max).max(1);
        let default = self.default_party;
        self.with_default_party(default)
    }

    pub fn with_default_party(mut self, party: u16) -> Self {
        self.default_party = party.clamp(self.min_party, self.max_party);
        self.party = self.default_party;
        self.count = RollingNumber::new(f32::from(self.party)).with_initial_roll(false);
        self.reset_selection();
        self
    }

    /// The callback should be inexpensive. The control builds a slot cache for each party size.
    pub fn with_availability(
        mut self,
        available: impl Fn(NaiveDate, NaiveTime, u16) -> bool + 'static,
    ) -> Self {
        self.availability = Some(Box::new(available));
        self.reset_selection();
        self
    }

    /// A failed task keeps the ticket and selection in place so the user can retry.
    pub fn with_confirm(
        mut self,
        confirm: impl Fn(Booking, &mut App) -> Task<Result<(), SharedString>> + 'static,
    ) -> Self {
        self.confirm = Some(Box::new(confirm));
        self
    }

    pub fn with_hour_cycle(mut self, cycle: BookingHourCycle) -> Self {
        self.cycle = cycle;
        self
    }

    pub fn with_style(mut self, style: impl Fn(Div) -> Div + 'static) -> Self {
        self.style = Some(Box::new(style));
        self
    }

    pub fn step(&self) -> BookingStep {
        self.step
    }

    pub fn selection(&self) -> Option<Booking> {
        let time = self.time?;
        self.is_free(self.day, time).then(|| Booking {
            date: self.days[self.day],
            time: self.times[time],
            party_size: self.party,
        })
    }

    /// Reload availability after an external change while preserving a valid selection.
    pub fn refresh_availability(&mut self, cx: &mut Context<Self>) {
        if self.pending || self.step == BookingStep::Booked {
            return;
        }
        self.rebuild_slots();
        if self.step != BookingStep::Review
            && self.time.is_none_or(|time| !self.is_free(self.day, time))
        {
            self.time = self.nearest_time(self.day, self.preferred);
        }
        cx.notify();
    }

    fn load_days(&mut self, days: usize) {
        self.days = (0..days)
            .map_while(|i| self.start.checked_add_days(Days::new(i as u64)))
            .collect();
        self.reset_selection();
    }

    fn is_free(&self, day: usize, time: usize) -> bool {
        self.free
            .get(day)
            .and_then(|row| row.get(time))
            .copied()
            .unwrap_or(false)
    }

    fn any_free(&self, day: usize) -> bool {
        self.free[day].iter().any(|&slot| slot)
    }

    fn rebuild_slots(&mut self) {
        self.free = self
            .days
            .iter()
            .map(|&date| {
                self.times
                    .iter()
                    .map(|&time| {
                        self.availability
                            .as_ref()
                            .is_none_or(|available| available(date, time, self.party))
                    })
                    .collect()
            })
            .collect();
    }

    fn reset_selection(&mut self) {
        self.rebuild_slots();
        self.day = (0..self.days.len())
            .find(|&i| self.any_free(i))
            .unwrap_or(0);
        self.time = self.nearest_time(self.day, self.preferred);
        self.strip.reset(self.day as f32);
    }

    fn nearest_time(&self, day: usize, preferred: NaiveTime) -> Option<usize> {
        let minutes = preferred.num_seconds_from_midnight();
        self.times
            .iter()
            .enumerate()
            .filter(|(i, _)| self.is_free(day, *i))
            .min_by_key(|(_, time)| time.num_seconds_from_midnight().abs_diff(minutes))
            .map(|(i, _)| i)
    }

    fn show_step(&mut self, step: BookingStep, cx: &mut Context<Self>) {
        if self.step == step {
            return;
        }
        self.direction = (i16::from(step.order()) - i16::from(self.step.order())).signum() as f32;
        self.step = step;
        self.drag = None;
        self.error = None;
        self.generation = self.generation.wrapping_add(1);
        self.request_focus = true;
        self.entered.restart(cx);
        cx.emit(BookingStepChanged(step));
        cx.notify();
    }

    fn back(&mut self, cx: &mut Context<Self>) {
        if self.pending {
            return;
        }
        if self.step == BookingStep::Booked {
            self.party = self.default_party;
            self.count = RollingNumber::new(f32::from(self.party)).with_initial_roll(false);
            self.reset_selection();
        }
        if self.step == BookingStep::Date {
            self.strip.duration(self.day as f32, 0.36, cx);
        }
        self.show_step(self.step.previous(), cx);
        if self.step == BookingStep::Time {
            self.refresh_availability(cx);
        }
    }

    fn bump_party(&mut self, delta: i16, cx: &mut Context<Self>) {
        let party = (i32::from(self.party) + i32::from(delta))
            .clamp(i32::from(self.min_party), i32::from(self.max_party)) as u16;
        if party != self.party {
            self.party = party;
            self.count.update(f32::from(party), cx);
            self.rebuild_slots();
            if !self.any_free(self.day) {
                self.day = (0..self.days.len())
                    .filter(|&i| self.any_free(i))
                    .min_by_key(|i| i.abs_diff(self.day))
                    .unwrap_or(self.day);
            }
            self.time = self.nearest_time(self.day, self.preferred);
            self.strip.reset(self.day as f32);
            cx.notify();
        }
    }

    fn set_day(&mut self, index: usize, velocity: f32, cx: &mut Context<Self>) {
        let index = index.min(self.days.len() - 1);
        if index != self.day {
            self.day = index;
            self.time = self.nearest_time(index, self.preferred);
        }
        if velocity.abs() > 0.1 && !disabled_motion(cx) {
            self.strip.release(index as f32, velocity, 0.47, cx);
        } else {
            self.strip.duration(index as f32, 0.36, cx);
        }
        cx.notify();
    }

    fn continue_to_times(&mut self, cx: &mut Context<Self>) {
        if self.any_free(self.day) {
            self.show_step(BookingStep::Time, cx);
        }
    }

    fn continue_to_review(&mut self, cx: &mut Context<Self>) {
        if self.selection().is_some() {
            self.show_step(BookingStep::Review, cx);
        }
    }

    fn date_key(&mut self, e: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        let index = self.day;
        let next = match e.keystroke.key.as_str() {
            "left" | "up" => index.saturating_sub(1),
            "right" | "down" => index + 1,
            "pageup" => index.saturating_sub(7),
            "pagedown" => index + 7,
            "home" => 0,
            "end" => self.days.len() - 1,
            "enter" => {
                self.continue_to_times(cx);
                index
            }
            _ => return,
        };
        self.set_day(next, 0., cx);
        w.prevent_default();
        cx.stop_propagation();
    }

    fn set_time(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.is_free(self.day, index) {
            self.time = Some(index);
            self.error = None;
            cx.notify();
        }
    }

    fn time_key(&mut self, e: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        let Some(index) = self.time else { return };
        let last = self.times.len() as isize;
        let (mut i, step) = match e.keystroke.key.as_str() {
            "left" => (index as isize - 1, -1),
            "right" => (index as isize + 1, 1),
            "up" => (index as isize - 4, -4),
            "down" => (index as isize + 4, 4),
            "home" => (0, 1),
            "end" => (last - 1, -1),
            _ => return,
        };
        while (0..last).contains(&i) {
            if self.is_free(self.day, i as usize) {
                self.set_time(i as usize, cx);
                break;
            }
            i += step;
        }
        w.prevent_default();
        cx.stop_propagation();
    }

    fn begin_drag(&mut self, x: Pixels, w: &mut Window, cx: &mut Context<Self>) {
        if self.drag.is_some() {
            return;
        }
        let origin = self.strip.value(cx);
        self.strip.reset(origin);
        self.drag = Some(Drag {
            start: x,
            last: x,
            at: Instant::now(),
            origin,
            velocity: 0.,
            moved: false,
        });
        self.focus[BookingStep::Date.order() as usize].focus(w, cx);
        cx.notify();
    }

    fn drag_to(&mut self, x: Pixels, cx: &mut Context<Self>) {
        let Some(mut drag) = self.drag else { return };
        if (x - drag.start).abs() > px(3.) {
            drag.moved = true;
        }
        if drag.moved {
            let now = Instant::now();
            let delta = f32::from(drag.last - x) / CELL;
            let seconds = now
                .saturating_duration_since(drag.at)
                .as_secs_f32()
                .max(0.008);
            if delta.abs() > 0.001 {
                let next = (delta / seconds).clamp(-18., 18.);
                drag.velocity = if seconds > 0.10 {
                    next
                } else {
                    drag.velocity * 0.35 + next * 0.65
                };
                drag.last = x;
                drag.at = now;
            }
            let raw = drag.origin + f32::from(drag.start - x) / CELL;
            let edge = raw.clamp(0., (self.days.len() - 1) as f32);
            let beyond = raw - edge;
            self.strip.reset(edge + beyond / (1. + 0.65 * beyond.abs()));
        }
        self.drag = Some(drag);
        cx.notify();
    }

    fn end_drag(&mut self, x: Pixels, cancelled: bool, cx: &mut Context<Self>) {
        if !cancelled {
            self.drag_to(x, cx);
        }
        let Some(drag) = self.drag.take() else { return };
        if cancelled {
            self.set_day(self.day, 0., cx);
            return;
        }
        if !drag.moved {
            let center = self.strip_bounds.get().center().x;
            let clicked = (drag.origin + f32::from(x - center) / CELL).round().max(0.) as usize;
            self.set_day(clicked, 0., cx);
            return;
        }
        let speed = if disabled_motion(cx) || drag.at.elapsed().as_secs_f32() > 0.09 {
            0.
        } else {
            drag.velocity
        };
        let projected = self.strip.value(cx) + speed * 0.20;
        self.set_day(projected.round().max(0.) as usize, speed, cx);
    }

    fn submit(&mut self, cx: &mut Context<Self>) {
        if self.pending || self.step != BookingStep::Review {
            return;
        }
        // Revalidate the ticket itself. A slot that filled must not turn
        // into a different booking without another review.
        self.rebuild_slots();
        let Some(booking) = self.selection() else {
            self.error =
                Some(tr(cx, "That time is no longer available.", "该时段已不可订。").into());
            cx.notify();
            return;
        };
        let job = match &self.confirm {
            Some(confirm) => confirm(booking, cx),
            None => {
                let timer = cx.background_executor().timer(Duration::from_millis(620));
                cx.spawn(async move |_, _| {
                    timer.await;
                    Ok(())
                })
            }
        };
        self.pending = true;
        self.error = None;
        let generation = self.generation;
        cx.notify();
        self.job = Some(cx.spawn(async move |this, cx| {
            let result = job.await;
            let _ = this.update(cx, |pill, cx| {
                if pill.generation != generation || pill.step != BookingStep::Review {
                    return;
                }
                pill.pending = false;
                match result {
                    Ok(()) => pill.show_step(BookingStep::Booked, cx),
                    Err(message) => {
                        pill.error = Some(message);
                        cx.notify();
                    }
                }
            });
        }));
    }
}

fn full_date(date: NaiveDate, cx: &App) -> String {
    if tr(cx, "en", "zh") == "zh" {
        let weekday = ["周一", "周二", "周三", "周四", "周五", "周六", "周日"];
        format!(
            "{}月{}日 {}",
            date.month(),
            date.day(),
            weekday[date.weekday().num_days_from_monday() as usize]
        )
    } else {
        date.format("%A, %B %-d").to_string()
    }
}

fn short_date(date: NaiveDate, cx: &App) -> String {
    if tr(cx, "en", "zh") == "zh" {
        full_date(date, cx)
    } else {
        date.format("%a, %b %-d").to_string()
    }
}

fn time_text(time: NaiveTime, cycle: BookingHourCycle) -> String {
    let format = if cycle == BookingHourCycle::TwentyFour {
        "%H:%M"
    } else {
        "%-I:%M %p"
    };
    time.format(format).to_string()
}

fn guests(party: u16, cx: &App) -> String {
    if tr(cx, "en", "zh") == "zh" {
        format!("{party} 位")
    } else if party == 1 {
        "1 guest".into()
    } else {
        format!("{party} guests")
    }
}

fn line(text: impl Into<SharedString>, size: f32, color: Hsla) -> Div {
    div()
        .text_size(px(size))
        .line_height(px(size * 1.32))
        .text_color(color)
        .child(text.into())
}

fn round_button(
    id: &'static str,
    label: &'static str,
    glyph: Glyph,
    side: f32,
    strong: bool,
    disabled: bool,
    cx: &App,
) -> Stateful<Div> {
    let p = Paint::get(cx);
    let tone = if strong { Tone::Primary } else { Tone::Muted };
    face(id, label, side, side * 0.5, tone, cx)
        .size(px(side))
        .when(!strong, |d| {
            d.bg(p.hover.opacity(0.58))
                .text_color(p.ink)
                .hover(|d| d.bg(p.hover))
        })
        .when(disabled, |d| {
            d.opacity(0.37).tab_index(-1).cursor(CursorStyle::Arrow)
        })
        .child(icon(glyph, 16., if strong { p.inverse } else { p.ink }))
}

fn back_button(label: &'static str, cx: &Context<BookingPill>) -> Stateful<Div> {
    let p = Paint::get(cx);
    face("booking-back", label, 44., 22., Tone::Clear, cx)
        .size(px(44.))
        .bg(p.hover.opacity(0.58))
        .hover(|d| d.bg(p.hover))
        .child(transformed(
            Glyph::Arrow,
            16.,
            p.ink,
            std::f32::consts::PI,
            1.,
            (0., 0.),
            1.,
        ))
        .on_click(cx.listener(|pill, _, _, cx| pill.back(cx)))
}

impl BookingPill {
    fn party_face(&mut self, w: &Window, cx: &Context<Self>) -> Div {
        let p = Paint::get(cx);
        let fewer = tr(cx, "Fewer guests", "减少人数");
        let more = tr(cx, "More guests", "增加人数");
        let next = tr(cx, "Next, choose a date", "下一步，选择日期");
        let word = if self.party == 1 {
            tr(cx, "guest", "位")
        } else {
            tr(cx, "guests", "位")
        };
        row()
            .h(px(60.))
            .px(px(8.))
            .child(back_button(tr(cx, "Back", "返回"), cx))
            .child(
                round_button(
                    "booking-fewer",
                    fewer,
                    Glyph::Minus,
                    36.,
                    false,
                    self.party == self.min_party,
                    cx,
                )
                .bg(transparent_black())
                .when(self.party > self.min_party, |d| {
                    d.on_click(cx.listener(|s, _, _, cx| s.bump_party(-1, cx)))
                }),
            )
            .child(
                row()
                    .flex_1()
                    .justify_center()
                    .gap(px(5.))
                    .child(
                        self.count
                            .render(22., w, cx)
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink),
                    )
                    .child(line(word, 14., p.secondary)),
            )
            .child(
                round_button(
                    "booking-more",
                    more,
                    Glyph::Plus,
                    36.,
                    false,
                    self.party == self.max_party,
                    cx,
                )
                .bg(transparent_black())
                .track_focus(&self.focus[BookingStep::Party.order() as usize])
                .when(self.party < self.max_party, |d| {
                    d.on_click(cx.listener(|s, _, _, cx| s.bump_party(1, cx)))
                }),
            )
            .child(
                round_button(
                    "booking-date-next",
                    next,
                    Glyph::Arrow,
                    44.,
                    true,
                    false,
                    cx,
                )
                .on_click(cx.listener(|s, _, _, cx| s.show_step(BookingStep::Date, cx))),
            )
    }

    fn date_face(&mut self, w: &mut Window, cx: &Context<Self>) -> Div {
        let p = Paint::get(cx);
        let available = self.any_free(self.day);
        let caption = full_date(self.days[self.day], cx);
        let strip_bounds = self.strip_bounds.clone();
        let position = self.strip.get(w, cx);
        let width = self.strip_bounds.get().size.width;
        let mut strip = div()
            .id("booking-date-strip")
            .debug_selector(|| "booking-date-strip".into())
            .role(Role::Slider)
            .aria_label(tr(cx, "Date", "日期"))
            .aria_value(if available {
                caption.clone()
            } else {
                format!("{caption}, {}", tr(cx, "no tables free", "已订满"))
            })
            .aria_numeric_value(self.day as f64)
            .aria_min_numeric_value(0.)
            .aria_max_numeric_value((self.days.len() - 1) as f64)
            .aria_numeric_value_step(1.)
            .track_focus(&self.focus[BookingStep::Date.order() as usize])
            .tab_index(0)
            .relative()
            .w_full()
            .h(px(60.))
            .overflow_hidden()
            .rounded(crate::foundation::surface_radius(cx))
            .focus_visible(move |d| d.border_1().border_color(p.focus))
            .cursor(if self.drag.is_some() {
                CursorStyle::ClosedHand
            } else {
                CursorStyle::OpenHand
            })
            .on_key_down(cx.listener(|s, e, w, cx| s.date_key(e, w, cx)))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|s, e: &MouseDownEvent, w, cx| s.begin_drag(e.position.x, w, cx)),
            )
            .on_mouse_move(cx.listener(|s, e: &MouseMoveEvent, _, cx| {
                if e.pressed_button == Some(MouseButton::Left) {
                    s.drag_to(e.position.x, cx);
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|s, e: &MouseUpEvent, _, cx| s.end_drag(e.position.x, false, cx)),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|s, e: &MouseUpEvent, _, cx| s.end_drag(e.position.x, false, cx)),
            )
            .child(
                canvas(
                    move |b, w, _| {
                        if strip_bounds.replace(b).size.width != b.size.width {
                            w.request_animation_frame();
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .inset_0(),
            )
            .child(self.touch_strip(cx));

        let center = f32::from(width) * 0.5;
        let visible = (center / CELL).ceil() as isize + 2;
        let near = position.round() as isize;
        let weekday = ["一", "二", "三", "四", "五", "六", "日"];
        for index in (near - visible).max(0) as usize..(near + visible + 1).max(0) as usize {
            let Some(&date) = self.days.get(index) else {
                break;
            };
            let x = center + (index as f32 - position) * CELL;
            let alpha =
                (x.min(f32::from(width) - x) / (f32::from(width) * 0.14).max(1.)).clamp(0., 1.);
            let closeness = (1. - (index as f32 - position).abs()).clamp(0., 1.);
            let active = closeness > 0.5;
            let enabled = self.any_free(index);
            let top = if tr(cx, "en", "zh") == "zh" {
                if date.day() == 1 {
                    format!("{}月", date.month())
                } else {
                    weekday[date.weekday().num_days_from_monday() as usize].into()
                }
            } else if date.day() == 1 {
                date.format("%b").to_string()
            } else {
                date.format("%a").to_string()
            };
            let color = if active {
                p.inverse
            } else if enabled {
                p.ink
            } else {
                p.faint
            };
            strip = strip.child(
                div()
                    .absolute()
                    .left(px(x - CELL * 0.5))
                    .top(px(4.))
                    .w(px(CELL))
                    .h(px(52.))
                    .rounded(crate::foundation::surface_radius(cx))
                    .bg(p.bright.opacity(closeness))
                    .opacity(alpha)
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .child(line(
                        top,
                        12.,
                        if active {
                            p.inverse.opacity(0.82)
                        } else {
                            p.faint
                        },
                    ))
                    .child(
                        line(date.day().to_string(), 16., color)
                            .font_weight(FontWeight::MEDIUM)
                            .when(!enabled, |d| d.line_through()),
                    ),
            );
        }

        div()
            .flex()
            .flex_col()
            .p(px(8.))
            .gap(px(4.))
            .child(
                row()
                    .h(px(44.))
                    .child(back_button(tr(cx, "Back to party size", "返回人数"), cx))
                    .child(
                        line(caption, 16., p.ink)
                            .flex_1()
                            .text_center()
                            .truncate()
                            .font_weight(FontWeight::MEDIUM),
                    )
                    .child(
                        round_button(
                            "booking-time-next",
                            tr(cx, "Next, choose a time", "下一步，选择时段"),
                            Glyph::Arrow,
                            44.,
                            true,
                            !available,
                            cx,
                        )
                        .when(available, |d| {
                            d.on_click(cx.listener(|s, _, _, cx| s.continue_to_times(cx)))
                        }),
                    ),
            )
            .child(strip)
    }

    fn touch_strip(&self, cx: &Context<Self>) -> impl IntoElement {
        let state = self.touch.clone();
        let gesture = cx.listener(|s, e: &TouchDragEvent, w, cx| match e.phase {
            TouchPhase::Started => s.begin_drag(e.start_position.x, w, cx),
            TouchPhase::Moved => s.drag_to(e.position.x, cx),
            TouchPhase::Ended => s.end_drag(e.position.x, false, cx),
            TouchPhase::Cancelled => s.end_drag(e.position.x, true, cx),
        });
        canvas(
            |bounds, w, _| w.insert_hitbox(bounds, HitboxBehavior::Normal),
            move |_, hit, w, _| {
                w.on_mouse_event(move |e: &TouchDragEvent, phase, w, cx| {
                    if !phase.bubble() {
                        return;
                    }
                    let dx = (e.position.x - e.start_position.x).abs();
                    let dy = (e.position.y - e.start_position.y).abs();
                    let ended = matches!(e.phase, TouchPhase::Ended | TouchPhase::Cancelled);
                    if e.phase == TouchPhase::Started {
                        state.set(u8::from(hit.is_hovered(w)));
                        return;
                    }
                    if state.get() == 1 && (dx.max(dy) > px(4.) || ended) {
                        let horizontal = e.phase == TouchPhase::Moved && dx > dy;
                        let tap = e.phase == TouchPhase::Ended && dx.max(dy) <= px(4.);
                        state.set(if horizontal || tap { 2 } else { 0 });
                        if state.get() == 2 {
                            let mut start = e.clone();
                            start.phase = TouchPhase::Started;
                            gesture(&start, w, cx);
                        }
                    }
                    if state.get() == 2 {
                        w.prevent_default();
                        gesture(e, w, cx);
                        cx.stop_propagation();
                    }
                    if ended {
                        state.set(0);
                    }
                });
            },
        )
        .absolute()
        .inset_0()
    }

    fn time_face(&self, cx: &Context<Self>) -> Div {
        let p = Paint::get(cx);
        let free = self.free[self.day]
            .iter()
            .filter(|&&available| available)
            .count();
        let count = guests(self.party, cx);
        let status = if tr(cx, "en", "zh") == "zh" {
            format!("{count} · {free} 个可订时段")
        } else {
            format!("{count} · {free} free")
        };
        let mut grid = div()
            .id("booking-time-grid")
            .role(Role::RadioGroup)
            .aria_label(tr(cx, "Seating times", "可预订时段"))
            .flex()
            .flex_col()
            .gap(px(6.))
            .on_key_down(cx.listener(|s, e, w, cx| s.time_key(e, w, cx)));
        for (chunk, times) in self.times.chunks(4).enumerate() {
            let mut line_row = row().gap(px(6.));
            for (offset, &time) in times.iter().enumerate() {
                let i = chunk * 4 + offset;
                let free = self.is_free(self.day, i);
                let active = free && self.time == Some(i);
                let label = time_text(time, self.cycle);
                let accessible = if free {
                    label.clone()
                } else {
                    format!("{label}, {}", tr(cx, "unavailable", "不可订"))
                };
                line_row = line_row.child(
                    div()
                        .id(("booking-time", i))
                        .debug_selector({
                            let label = label.clone();
                            move || label.clone()
                        })
                        .role(Role::RadioButton)
                        .aria_label(accessible)
                        .aria_toggled(active.into())
                        .tab_index(if active { 0 } else { -1 })
                        .when(active, |d| {
                            d.track_focus(&self.focus[BookingStep::Time.order() as usize])
                        })
                        .h(px(40.))
                        .flex_1()
                        .min_w_0()
                        .rounded(crate::foundation::surface_radius(cx))
                        .border_1()
                        .border_color(p.line)
                        .focus_visible(move |d| d.border_color(p.focus))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(13.))
                        .line_height(px(17.))
                        .whitespace_nowrap()
                        .font_weight(FontWeight::MEDIUM)
                        .when(active, |d| d.bg(p.bright).text_color(p.inverse))
                        .when(free && !active, |d| d.hover(|d| d.bg(p.hover)))
                        .when(free, |d| {
                            d.cursor_pointer()
                                .on_click(cx.listener(move |s, _, _, cx| s.set_time(i, cx)))
                        })
                        .when(!free, |d| d.text_color(p.faint).line_through())
                        .child(label),
                );
            }
            for _ in times.len()..4 {
                line_row = line_row.child(div().flex_1());
            }
            grid = grid.child(line_row);
        }
        let name = tr(cx, "Review booking", "查看预订");
        let blocked = self.selection().is_none();
        div()
            .flex()
            .flex_col()
            .p(px(10.))
            .gap(px(10.))
            .child(
                row()
                    .h(px(44.))
                    .child(back_button(tr(cx, "Back to dates", "返回日期"), cx))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .items_center()
                            .child(
                                line(full_date(self.days[self.day], cx), 16., p.ink)
                                    .max_w_full()
                                    .truncate()
                                    .font_weight(FontWeight::MEDIUM),
                            )
                            .child(line(status, 14., p.secondary)),
                    )
                    .child(div().w(px(44.)).flex_none()),
            )
            .child(grid)
            .child(
                face("booking-review", name, 44., 22., Tone::Primary, cx)
                    .w_full()
                    .child(name)
                    .when(blocked, |d| {
                        d.opacity(0.4)
                            .tab_index(-1)
                            .track_focus(&self.focus[BookingStep::Time.order() as usize])
                    })
                    .when(!blocked, |d| {
                        d.on_click(cx.listener(|s, _, _, cx| s.continue_to_review(cx)))
                    }),
            )
    }

    fn review_face(&self, w: &Window, cx: &Context<Self>) -> Div {
        let p = Paint::get(cx);
        let date = self.days[self.day];
        let time = self
            .time
            .and_then(|i| self.times.get(i))
            .map(|&time| time_text(time, self.cycle))
            .unwrap_or_default();
        let fact = |name, value| {
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(px(2.))
                .child(line(name, 12., p.faint))
                .child(line(value, 16., p.ink).font_weight(FontWeight::MEDIUM))
        };
        let label = if self.pending {
            tr(cx, "Booking…", "正在预订…")
        } else if self.error.is_some() && self.selection().is_some() {
            tr(cx, "Try again", "重试")
        } else {
            tr(cx, "Confirm booking", "确认预订")
        };
        let mut primary = face("booking-confirm", label, 44., 22., Tone::Primary, cx)
            .track_focus(&self.focus[BookingStep::Review.order() as usize])
            .min_w_0()
            .flex_1()
            .child(label);
        if self.pending {
            let phase = self.entered.time(100_000., w, cx);
            primary = primary
                .child(spinner(phase * 4., 16., p.inverse))
                .cursor(CursorStyle::Arrow);
        } else {
            primary = primary.on_click(cx.listener(|s, _, _, cx| s.submit(cx)));
        }
        div()
            .flex()
            .flex_col()
            .child(
                div()
                    .p(px(22.))
                    .flex()
                    .flex_col()
                    .gap(px(14.))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(2.))
                            .child(
                                line(self.venue.clone(), 20., p.ink)
                                    .font_weight(FontWeight::MEDIUM),
                            )
                            .when(!self.detail.is_empty(), |d| {
                                d.child(line(self.detail.clone(), 14., p.secondary))
                            }),
                    )
                    .child(
                        row()
                            .items_start()
                            .gap(px(10.))
                            .child(
                                fact(tr(cx, "Date", "日期"), short_date(date, cx)).flex_grow(1.5),
                            )
                            .child(fact(tr(cx, "Time", "时间"), time))
                            .child(fact(tr(cx, "Guests", "人数"), self.party.to_string())),
                    ),
            )
            .child(ticket_line(p.faint.opacity(0.52)))
            .when_some(self.error.clone(), |d, error| {
                d.child(
                    line(error, 13., p.danger)
                        .id("booking-error")
                        .role(Role::Alert)
                        .px(px(16.))
                        .pt(px(10.)),
                )
            })
            .child(
                row()
                    .p(px(14.))
                    .child(
                        back_button(tr(cx, "Change time", "更改时段"), cx)
                            .when(self.pending, |d| d.opacity(0.45).tab_index(-1)),
                    )
                    .child(primary),
            )
    }

    fn booked_face(&self, cx: &Context<Self>) -> Div {
        let p = Paint::get(cx);
        let summary = self
            .selection()
            .map(|booking| {
                format!(
                    "{} · {} · {}",
                    short_date(booking.date, cx),
                    time_text(booking.time, self.cycle),
                    guests(booking.party_size, cx)
                )
            })
            .unwrap_or_default();
        let title = tr(cx, "Booked", "已预订");
        let done = tr(cx, "Done", "完成");
        div()
            .flex()
            .flex_col()
            .gap(px(14.))
            .p(px(14.))
            .child(
                row()
                    .gap(px(10.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .size(px(44.))
                            .flex_none()
                            .rounded_full()
                            .bg(p.success.opacity(0.14))
                            .child(icon(Glyph::Check, 20., p.success)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(line(title, 16., p.ink).font_weight(FontWeight::MEDIUM))
                            .child(line(self.venue.clone(), 14., p.secondary))
                            .child(line(summary, 13., p.secondary)),
                    ),
            )
            .child(
                face("booking-done", done, 44., 22., Tone::Primary, cx)
                    .w_full()
                    .child(done)
                    .track_focus(&self.focus[BookingStep::Booked.order() as usize])
                    .on_click(cx.listener(|s, _, _, cx| s.back(cx))),
            )
    }
}

fn ticket_line(color: Hsla) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |bounds, _, w, _| {
            let mut x = bounds.left() + px(10.);
            let limit = bounds.right() - px(10.);
            while x < limit {
                w.paint_quad(fill(
                    Bounds::new(
                        point(x, bounds.top()),
                        size((limit - x).min(px(3.)), px(1.)),
                    ),
                    color,
                ));
                x += px(5.);
            }
        },
    )
    .h(px(1.))
    .w_full()
}

impl Render for BookingPill {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let step = self.step;
        let p = Paint::get(cx);
        let visual_radius = f32::from(crate::foundation::surface_radius(cx));
        let closed_width = measure(&self.label, 16., w) + 70.;
        let (preferred, guessed_height, radius) = match step {
            BookingStep::Start => (closed_width, 52., 26.),
            BookingStep::Party => (300., 60., 30.),
            BookingStep::Date => (420., 124., visual_radius),
            BookingStep::Time => (360., 262., visual_radius),
            BookingStep::Review => (340., 215., visual_radius),
            BookingStep::Booked => (340., 126., visual_radius),
        };
        let limit = self.width_limit.get();
        let host_width = if limit > px(0.) {
            limit
        } else {
            w.viewport_size().width.min(px(444.))
        };
        let target_width = px(preferred).min((host_width - px(24.)).max(px(1.)));
        let measured = self.face.get();
        let target_height = if measured.step == step
            && measured.width == target_width
            && measured.height > px(0.)
        {
            f32::from(measured.height)
        } else {
            guessed_height
        };
        self.width
            .duration(f32::from(target_width), STEP_SECONDS, cx);
        self.height.duration(target_height, STEP_SECONDS, cx);
        self.radius.duration(radius, STEP_SECONDS, cx);
        let width = px(self.width.get(w, cx)).min((host_width - px(24.)).max(px(1.)));
        let height = px(self.height.get(w, cx).max(0.));
        let radius = px(self.radius.get(w, cx));
        let progress = ease((self.entered.time(0.26, w, cx) - 0.045) / 0.215);
        let fit = (width / target_width).min(f32::from(height) / target_height);
        let reveal = progress * ease((fit - 0.90) / 0.10);
        let offset = if disabled_motion(cx) {
            0.
        } else {
            self.direction * (1. - progress) * 10.
        };
        let face_bounds = self.face.clone();
        let width_limit = self.width_limit.clone();
        let surface = match step {
            BookingStep::Start => face(
                "booking-start",
                self.label.clone(),
                52.,
                26.,
                Tone::Clear,
                cx,
            )
            .w_full()
            .text_size(px(16.))
            .track_focus(&self.focus[BookingStep::Start.order() as usize])
            .child(calendar_mark(p.ink))
            .child(self.label.clone())
            .on_click(cx.listener(|s, _, _, cx| s.show_step(BookingStep::Party, cx)))
            .into_any_element(),
            BookingStep::Party => self.party_face(w, cx).into_any_element(),
            BookingStep::Date => self.date_face(w, cx).into_any_element(),
            BookingStep::Time => self.time_face(cx).into_any_element(),
            BookingStep::Review => self.review_face(w, cx).into_any_element(),
            BookingStep::Booked => self.booked_face(cx).into_any_element(),
        };
        if std::mem::take(&mut self.request_focus) {
            let weak = cx.weak_entity();
            let focus = self.focus[step.order() as usize].clone();
            let generation = self.generation;
            w.on_next_frame(move |w, cx| {
                if weak
                    .upgrade()
                    .is_some_and(|view| view.read(cx).generation == generation)
                {
                    focus.focus(w, cx);
                }
            });
        }
        let root = div()
            .relative()
            .w_full()
            .max_w(px(444.))
            .flex()
            .justify_center();
        let root = match &self.style {
            Some(refine) => refine(root),
            None => root,
        };
        root.on_children_prepainted(move |children, w, _| {
            if let Some(b) = children.last()
                && width_limit.replace(b.size.width) != b.size.width
            {
                w.request_animation_frame();
            }
        })
        .id(self.id.clone())
        .role(Role::Group)
        .aria_label(format!("{} · {}", self.label, self.venue))
        .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
            if e.keystroke.key == "escape" && s.step != BookingStep::Start {
                s.back(cx);
                w.prevent_default();
                cx.stop_propagation();
            }
        }))
        .child(
            div()
                .on_children_prepainted(move |children, w, _| {
                    if let Some(b) = children.first() {
                        let previous = face_bounds.replace(FaceBounds {
                            step,
                            width: target_width,
                            height: b.size.height,
                        });
                        if previous.step != step
                            || previous.width != target_width
                            || previous.height != b.size.height
                        {
                            w.request_animation_frame();
                        }
                    }
                })
                .id("booking-surface")
                .debug_selector(|| "booking-surface".into())
                .relative()
                .w(width)
                .h(height)
                .flex_none()
                .rounded(radius)
                .overflow_hidden()
                .bg(p.panel.opacity(0.96))
                .shadow(vec![
                    BoxShadow::new(px(0.), px(8.), black().opacity(0.10)).blur_radius(px(22.)),
                ])
                .child(
                    div()
                        .absolute()
                        .bottom_0()
                        .left((width - target_width) / 2. + px(offset))
                        .w(target_width)
                        .opacity(reveal)
                        .child(surface),
                ),
        )
        .child(canvas(|_, _, _| (), |_, _, _, _| {}).absolute().inset_0())
    }
}

#[cfg(test)]
mod tests {
    use super::{BookingPill, BookingStep, BookingStepChanged, clock_time};
    use chrono::{Days, NaiveDate};
    use gpui::{Modifiers, Task, TestAppContext, VisualTestContext, px, size};
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
        time::Duration,
    };

    fn start() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 21).unwrap()
    }

    fn settle(w: &mut VisualTestContext) {
        for _ in 0..3 {
            w.executor().advance_clock(Duration::from_millis(900));
            w.update(|w, cx| {
                w.simulate_next_frame(cx);
                w.refresh();
            });
            w.run_until_parked();
        }
    }

    #[test]
    fn disabled_dates_and_times_remain_visible_and_preferred_slot_falls_back() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (v, w) = app.add_window_view(|_, cx| {
            BookingPill::new("Olea", start(), cx)
                .with_times(vec![
                    clock_time(18, 30),
                    clock_time(19, 0),
                    clock_time(19, 30),
                    clock_time(20, 0),
                ])
                .with_availability(|date, time, _| date != start() && time != clock_time(19, 30))
        });
        w.update(|_, cx| {
            assert_eq!(v.read(cx).selection().unwrap().date, start() + Days::new(1));
            assert_eq!(v.read(cx).selection().unwrap().time, clock_time(19, 0));
            v.update(cx, |s, cx| {
                s.show_step(BookingStep::Date, cx);
                s.set_day(0, 0., cx);
                s.continue_to_times(cx);
                assert_eq!(s.step(), BookingStep::Date);
                s.set_day(1, 0., cx);
                s.continue_to_times(cx);
                s.set_time(2, cx);
                assert_eq!(s.selection().unwrap().time, clock_time(19, 0));
                assert_eq!(s.step(), BookingStep::Time);
            });
        });
    }

    #[test]
    fn a_slot_lost_during_review_never_sends_a_different_time() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let available = Rc::new(Cell::new(true));
        let calls = Rc::new(Cell::new(0));
        let (v, w) = app.add_window_view(|_, cx| {
            let available = available.clone();
            let calls = calls.clone();
            BookingPill::new("Olea", start(), cx)
                .with_availability(move |_, time, _| time != clock_time(19, 30) || available.get())
                .with_confirm(move |_, _| {
                    calls.set(calls.get() + 1);
                    Task::ready(Ok(()))
                })
        });
        w.update(|_, cx| {
            v.update(cx, |s, cx| {
                s.show_step(BookingStep::Review, cx);
                available.set(false);
                s.submit(cx);
                assert_eq!(s.time.map(|i| s.times[i]), Some(clock_time(19, 30)));
                assert!(s.error.is_some());
                assert_eq!(calls.get(), 0);
                s.back(cx);
                assert_eq!(s.step(), BookingStep::Time);
                assert_eq!(s.selection().unwrap().time, clock_time(19, 0));
            })
        });
    }

    #[test]
    fn failed_submission_preserves_the_ticket_and_retry_emits_one_booked_event() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let calls = Rc::new(RefCell::new(Vec::new()));
        let (v, w) = app.add_window_view(|_, cx| {
            let calls = calls.clone();
            BookingPill::new("Olea", start(), cx).with_confirm(move |booking, _| {
                calls.borrow_mut().push(booking);
                Task::ready(if calls.borrow().len() == 1 {
                    Err("Try later".into())
                } else {
                    Ok(())
                })
            })
        });
        let events = Rc::new(RefCell::new(Vec::new()));
        let _subscription = w.update(|_, cx| {
            let events = events.clone();
            cx.subscribe(&v, move |_, e: &BookingStepChanged, _| {
                events.borrow_mut().push(e.0)
            })
        });
        w.update(|_, cx| {
            v.update(cx, |s, cx| {
                s.show_step(BookingStep::Review, cx);
                s.submit(cx);
                s.submit(cx);
                s.back(cx);
                assert!(s.pending);
                assert_eq!(s.step(), BookingStep::Review);
            })
        });
        w.run_until_parked();
        w.update(|_, cx| {
            v.update(cx, |s, cx| {
                assert!(!s.pending);
                assert_eq!(s.error.as_deref(), Some("Try later"));
                assert_eq!(s.selection(), calls.borrow().first().copied());
                s.submit(cx);
            })
        });
        w.run_until_parked();
        assert_eq!(w.update(|_, cx| v.read(cx).step()), BookingStep::Booked);
        assert_eq!(calls.borrow().len(), 2);
        assert_eq!(calls.borrow()[0], calls.borrow()[1]);
        assert_eq!(*events.borrow(), [BookingStep::Review, BookingStep::Booked]);
    }

    #[test]
    fn narrow_date_face_keeps_all_controls_inside_and_keyboard_can_turn_a_week() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (v, w) = app.add_window_view(|_, cx| BookingPill::new("Olea", start(), cx));
        w.simulate_resize(size(px(320.), px(700.)));
        settle(w);
        let trigger = w.debug_bounds("Book a table").unwrap().center();
        w.simulate_click(trigger, Modifiers::default());
        settle(w);
        let next = w.debug_bounds("Next, choose a date").unwrap().center();
        w.simulate_click(next, Modifiers::default());
        settle(w);
        let shell = w.debug_bounds("booking-surface").unwrap();
        let strip = w.debug_bounds("booking-date-strip").unwrap();
        assert_eq!(shell.size.width, px(296.));
        assert!(strip.left() >= shell.left() && strip.right() <= shell.right());
        w.simulate_keystrokes("pagedown enter");
        settle(w);
        assert_eq!(w.update(|_, cx| v.read(cx).step()), BookingStep::Time);
        assert_eq!(
            w.update(|_, cx| v.read(cx).selection().unwrap().date),
            start() + Days::new(7)
        );
    }
}
