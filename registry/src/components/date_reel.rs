//! Select a civil date or time on momentum-driven cylindrical reels.
use crate::{
    foundation::surface_radius,
    primitives::{
        direct_touch::Contact,
        editor::EditorScrollRegion,
        family::{Paint, Tone, disabled_motion, face, measure},
    },
};
use chrono::{Datelike, Days, Months, NaiveDate, NaiveDateTime, Timelike};
use gpui::{prelude::*, *};
use std::{
    cell::{Cell, OnceCell, RefCell},
    collections::VecDeque,
    rc::Rc,
};
use web_time::Instant;

const MAX_WIDTH: f32 = 520.;
const HALF_ROWS: i32 = 5;
const WHEEL_ANGLE: f32 = 0.388;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DateReelMode {
    #[default]
    DateTime,
    Date,
    Time,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MinuteStep {
    One = 1,
    #[default]
    Five = 5,
    Ten = 10,
    Fifteen = 15,
    Thirty = 30,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HourCycle {
    #[default]
    Twelve,
    TwentyFour,
}

/// Built-in display conventions. Use `with_formatter` for additional locales.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DateReelLocale {
    #[default]
    English,
    Chinese,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DateReelText {
    Day,
    Month,
    Date,
    Year,
    Hour,
    Minute,
    Period,
    Summary,
    FullDate,
}

#[derive(Clone, Debug)]
pub struct DateReelLabels {
    pub day: SharedString,
    pub month: SharedString,
    pub date: SharedString,
    pub year: SharedString,
    pub hour: SharedString,
    pub minute: SharedString,
    pub period: SharedString,
    pub at: SharedString,
    pub quick_picks: SharedString,
    pub confirm: SharedString,
    pub pending: SharedString,
    pub success: SharedString,
    pub retry: SharedString,
}

impl DateReelLabels {
    pub fn for_locale(locale: DateReelLocale) -> Self {
        let words = match locale {
            DateReelLocale::English => [
                "Day",
                "Month",
                "Date",
                "Year",
                "Hour",
                "Minute",
                "AM or PM",
                " at ",
                "Quick picks",
                "Schedule",
                "Scheduling",
                "Scheduled",
                "Try again",
            ],
            DateReelLocale::Chinese => [
                "日期",
                "月份",
                "日",
                "年份",
                "小时",
                "分钟",
                "上午或下午",
                " ",
                "快捷日期",
                "安排发送",
                "正在安排",
                "已安排",
                "重试",
            ],
        };
        let [
            day,
            month,
            date,
            year,
            hour,
            minute,
            period,
            at,
            quick_picks,
            confirm,
            pending,
            success,
            retry,
        ] = words.map(SharedString::from);
        Self {
            day,
            month,
            date,
            year,
            hour,
            minute,
            period,
            at,
            quick_picks,
            confirm,
            pending,
            success,
            retry,
        }
    }
}

impl Default for DateReelLabels {
    fn default() -> Self {
        Self::for_locale(DateReelLocale::English)
    }
}

type RelativePreset = Box<dyn Fn(NaiveDate) -> NaiveDateTime>;

pub struct DateReelPreset {
    pub label: SharedString,
    value: RelativePreset,
}

impl DateReelPreset {
    pub fn new(label: impl Into<SharedString>, value: NaiveDateTime) -> Self {
        Self::relative(label, move |_| value)
    }

    pub fn relative(
        label: impl Into<SharedString>,
        value: impl Fn(NaiveDate) -> NaiveDateTime + 'static,
    ) -> Self {
        Self {
            label: label.into(),
            value: Box::new(value),
        }
    }
}

/// Emitted once after a user-selected value settles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DateReelChanged(pub NaiveDateTime);

type Confirm = Box<dyn Fn(NaiveDateTime, &mut App) -> Task<Result<(), SharedString>>>;
type Formatter =
    Box<dyn Fn(DateReelText, NaiveDateTime, NaiveDate, HourCycle) -> Option<SharedString>>;
type Visibility = Box<dyn Fn(&App) -> bool>;
type CardStyle = Box<dyn Fn(Div) -> Div>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Lane {
    Day = 0,
    Month,
    Date,
    Year,
    Hour,
    Minute,
    Period,
}

impl Lane {
    fn text(self) -> DateReelText {
        match self {
            Self::Day => DateReelText::Day,
            Self::Month => DateReelText::Month,
            Self::Date => DateReelText::Date,
            Self::Year => DateReelText::Year,
            Self::Hour => DateReelText::Hour,
            Self::Minute => DateReelText::Minute,
            Self::Period => DateReelText::Period,
        }
    }

    fn label(self, labels: &DateReelLabels) -> &SharedString {
        match self {
            Self::Day => &labels.day,
            Self::Month => &labels.month,
            Self::Date => &labels.date,
            Self::Year => &labels.year,
            Self::Hour => &labels.hour,
            Self::Minute => &labels.minute,
            Self::Period => &labels.period,
        }
    }
}

const DATE: &[Lane] = &[Lane::Month, Lane::Date, Lane::Year];
const DATETIME: &[Lane] = &[Lane::Day, Lane::Hour, Lane::Minute, Lane::Period];
const TIME: &[Lane] = &[Lane::Hour, Lane::Minute, Lane::Period];

#[derive(Clone, Copy)]
struct Range {
    last: i32,
    wraps: bool,
}

impl Range {
    fn index(self, raw: i32) -> i32 {
        if self.wraps {
            raw.rem_euclid(self.last + 1)
        } else {
            raw.clamp(0, self.last)
        }
    }

    fn nearest(self, position: f32, index: i32) -> f32 {
        if !self.wraps {
            return self.index(index) as f32;
        }
        let count = (self.last + 1) as f32;
        let index = self.index(index) as f32;
        index + ((position - index) / count).round() * count
    }

    fn rubber(self, position: f32) -> f32 {
        if self.wraps {
            return position;
        }
        let bound = position.clamp(0., self.last as f32);
        let beyond = position - bound;
        bound + beyond * 0.48 / (1. + beyond.abs() * 0.45)
    }
}

#[derive(Clone, Copy, Debug)]
enum Travel {
    Spring {
        start: f32,
        velocity: f32,
        elapsed: f32,
    },
    Glide {
        start: f32,
        velocity: f32,
        decay: f32,
        end: f32,
        elapsed: f32,
    },
}

#[derive(Clone, Copy, Default)]
struct Spin {
    position: f32,
    target: f32,
    velocity: f32,
    delay: f32,
    travel: Option<Travel>,
}

impl Spin {
    fn reset(&mut self, target: f32) {
        *self = Self {
            position: target,
            target,
            ..Self::default()
        };
    }

    fn seek(&mut self, target: f32, delay: f32, immediate: bool) {
        if immediate || ((target - self.position).abs() < 0.0005 && self.velocity.abs() < 0.001) {
            self.reset(target);
            return;
        }
        self.target = target;
        self.delay = delay;
        self.travel = Some(Travel::Spring {
            start: self.position,
            velocity: self.velocity,
            elapsed: 0.,
        });
    }

    fn coast(&mut self, range: Range, speed: f32, immediate: bool) {
        let velocity = speed.clamp(-70., 70.);
        let target = (self.position + velocity / 7.4).round();
        let target = if range.wraps {
            target
        } else {
            target.clamp(0., range.last as f32)
        };
        let delta = target - self.position;
        if !immediate && velocity.abs() >= 2.6 && delta * velocity > 0. && delta.abs() > 0.8 {
            let decay = velocity.abs() / delta.abs();
            self.target = target;
            self.delay = 0.;
            self.travel = Some(Travel::Glide {
                start: self.position,
                velocity,
                decay,
                end: (delta.abs() / 0.46).ln() / decay,
                elapsed: 0.,
            });
        } else {
            let closest = if range.wraps {
                self.position.round()
            } else {
                self.position.round().clamp(0., range.last as f32)
            };
            self.seek(closest, 0., immediate);
        }
    }

    fn advance(&mut self, dt: f32) {
        if self.delay > 0. {
            let wait = self.delay;
            self.delay = (wait - dt).max(0.);
            if wait >= dt {
                return;
            }
            self.advance(dt - wait);
            return;
        }
        match self.travel {
            Some(Travel::Glide {
                start,
                velocity,
                decay,
                end,
                elapsed,
            }) => {
                let next = (elapsed + dt).min(end);
                let damping = (-decay * next).exp();
                self.position = start + velocity * (1. - damping) / decay;
                self.velocity = velocity * damping;
                if next >= end {
                    self.seek(self.target, 0., false);
                    self.advance((elapsed + dt - end).max(0.));
                } else {
                    self.travel = Some(Travel::Glide {
                        start,
                        velocity,
                        decay,
                        end,
                        elapsed: next,
                    });
                }
            }
            Some(Travel::Spring {
                start,
                velocity,
                elapsed,
            }) => {
                let elapsed = elapsed + dt;
                let offset = start - self.target;
                let omega = 17.;
                let carry = velocity + omega * offset;
                let decay = (-omega * elapsed).exp();
                self.position = self.target + (offset + carry * elapsed) * decay;
                self.velocity = (velocity - omega * carry * elapsed) * decay;
                if (self.position - self.target).abs() < 0.009 && self.velocity.abs() < 0.09 {
                    self.reset(self.target);
                } else {
                    self.travel = Some(Travel::Spring {
                        start,
                        velocity,
                        elapsed,
                    });
                }
            }
            None => {}
        }
    }
}

#[derive(Clone, Copy)]
struct Drag {
    lane: Lane,
    y: Pixels,
    origin: f32,
    previous_target: f32,
    previous_change: bool,
    last_y: Pixels,
    at: Instant,
    velocity: f32,
    row: Pixels,
    moved: bool,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum ConfirmState {
    #[default]
    Idle,
    Pending,
    Success,
    Error,
}

#[derive(Clone, Copy)]
struct Metrics {
    width: Pixels,
    row: Pixels,
    font: Pixels,
    inset: Pixels,
    compact: bool,
}

struct CachedEntry {
    index: i32,
    context: Option<NaiveDateTime>,
    entry: (SharedString, bool),
}

impl Metrics {
    fn new(width: Pixels) -> Self {
        let compact = width < px(480.);
        Self {
            width,
            row: px(if compact { 36. } else { 42. }),
            font: px(if compact { 18. } else { 22. }),
            inset: px(if compact { 16. } else { 24. }),
            compact,
        }
    }
}

fn date(year: i32, month: u32, day: u32) -> NaiveDate {
    // The public API takes checked date types; this helper is for known dates.
    NaiveDate::from_ymd_opt(year, month, day).unwrap_or(NaiveDate::MIN)
}

fn at(day: NaiveDate, hour: u32, minute: u32) -> NaiveDateTime {
    day.and_hms_opt(hour, minute, 0).unwrap_or_default()
}

fn shift_day(day: NaiveDate, delta: i64) -> NaiveDate {
    if delta < 0 {
        day.checked_sub_days(Days::new(delta.unsigned_abs()))
            .unwrap_or(NaiveDate::MIN)
    } else {
        day.checked_add_days(Days::new(delta as u64))
            .unwrap_or(NaiveDate::MAX)
    }
}

fn shift_year(day: NaiveDate, years: u32, forward: bool) -> NaiveDate {
    if forward {
        day.checked_add_months(Months::new(years * 12))
            .unwrap_or(NaiveDate::MAX)
    } else {
        day.checked_sub_months(Months::new(years * 12))
            .unwrap_or(NaiveDate::MIN)
    }
}

fn last_day(year: i32, month: u32) -> u32 {
    (28..=31)
        .rev()
        .find(|&day| NaiveDate::from_ymd_opt(year, month, day).is_some())
        .unwrap_or(28)
}

/// Three reel layouts share one civil date and time value.
pub struct DateReel {
    id: ElementId,
    mode: DateReelMode,
    locale: DateReelLocale,
    labels: Option<DateReelLabels>,
    label: Option<SharedString>,
    title: Option<SharedString>,
    today: Option<NaiveDate>,
    explicit_min: Option<NaiveDate>,
    explicit_max: Option<NaiveDate>,
    minute_step: MinuteStep,
    hour_cycle: HourCycle,
    start: NaiveDateTime,
    controlled: bool,
    committed: NaiveDateTime,
    external: NaiveDateTime,
    spins: [Spin; 7],
    focus: [FocusHandle; 7],
    confirm_focus: FocusHandle,
    contact: [Contact; 7],
    lane_bounds: [Rc<Cell<Bounds<Pixels>>>; 7],
    bounds: Rc<Cell<Bounds<Pixels>>>,
    drag: Option<Drag>,
    search: Option<(Lane, String, Instant)>,
    wheel_remainder: f32,
    wheel_lane: Option<(Lane, Instant)>,
    last_frame: Option<Instant>,
    speed: f32,
    intro: bool,
    entered: bool,
    paused: bool,
    user_change: bool,
    presets: Option<Vec<DateReelPreset>>,
    preset_values: OnceCell<Vec<(SharedString, NaiveDateTime)>>,
    row_labels: RefCell<[VecDeque<CachedEntry>; 7]>,
    formatter: Option<Formatter>,
    visible: Option<Visibility>,
    style: Option<CardStyle>,
    accent: Option<Hsla>,
    confirm: Option<Confirm>,
    confirm_state: ConfirmState,
    confirm_epoch: u64,
    task: Option<Task<()>>,
    error: Option<SharedString>,
    announcement: Option<SharedString>,
}

impl EventEmitter<DateReelChanged> for DateReel {}

impl DateReel {
    pub fn new(cx: &App) -> Self {
        let start = at(date(2026, 9, 24), 9, 30);
        let mut this = Self {
            id: "date-reel".into(),
            mode: Default::default(),
            locale: Default::default(),
            labels: None,
            label: None,
            title: None,
            today: None,
            explicit_min: None,
            explicit_max: None,
            minute_step: Default::default(),
            hour_cycle: Default::default(),
            start,
            controlled: false,
            committed: start,
            external: start,
            spins: Default::default(),
            focus: std::array::from_fn(|_| cx.focus_handle().tab_stop(true)),
            confirm_focus: cx.focus_handle().tab_stop(true),
            contact: Default::default(),
            lane_bounds: Default::default(),
            bounds: Default::default(),
            drag: None,
            search: None,
            wheel_remainder: 0.,
            wheel_lane: None,
            last_frame: None,
            speed: 1.,
            intro: true,
            entered: false,
            paused: false,
            user_change: false,
            presets: None,
            preset_values: OnceCell::new(),
            row_labels: Default::default(),
            formatter: None,
            visible: None,
            style: None,
            accent: None,
            confirm: None,
            confirm_state: Default::default(),
            confirm_epoch: 0,
            task: None,
            error: None,
            announcement: None,
        };
        this.reset();
        this
    }

    pub fn with_id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = id.into();
        self
    }

    pub fn with_mode(mut self, mode: DateReelMode) -> Self {
        self.mode = mode;
        self.reset();
        self
    }

    pub fn with_default_value(mut self, value: NaiveDateTime) -> Self {
        self.controlled = false;
        self.start = value;
        self.reset();
        self
    }

    pub fn with_value(mut self, value: NaiveDateTime) -> Self {
        self.controlled = true;
        self.start = value;
        self.reset();
        self
    }

    pub fn with_today(mut self, today: NaiveDate) -> Self {
        self.today = Some(today);
        self.reset();
        self
    }

    pub fn with_min_date(mut self, date: NaiveDate) -> Self {
        self.explicit_min = Some(date);
        self.reset();
        self
    }

    pub fn with_max_date(mut self, date: NaiveDate) -> Self {
        self.explicit_max = Some(date);
        self.reset();
        self
    }

    pub fn with_minute_step(mut self, step: MinuteStep) -> Self {
        self.minute_step = step;
        self.reset();
        self
    }

    pub fn with_hour_cycle(mut self, cycle: HourCycle) -> Self {
        self.hour_cycle = cycle;
        self.reset();
        self
    }

    pub fn with_locale(mut self, locale: DateReelLocale) -> Self {
        self.locale = locale;
        self.invalidate_labels();
        self
    }

    pub fn with_labels(mut self, labels: DateReelLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    pub fn with_formatter(
        mut self,
        formatter: impl Fn(DateReelText, NaiveDateTime, NaiveDate, HourCycle) -> Option<SharedString>
        + 'static,
    ) -> Self {
        self.formatter = Some(Box::new(formatter));
        self.invalidate_labels();
        self
    }

    pub fn with_presets(mut self, presets: Vec<DateReelPreset>) -> Self {
        self.presets = Some(presets);
        self.preset_values.take();
        self
    }

    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn with_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn with_intro(mut self, intro: bool) -> Self {
        self.intro = intro;
        self
    }

    pub fn with_speed(mut self, speed: f32) -> Self {
        if speed.is_finite() && speed > 0. {
            self.speed = speed;
        }
        self
    }

    pub fn with_paused(mut self, paused: bool) -> Self {
        self.paused = paused;
        self
    }

    pub fn with_accent(mut self, accent: Hsla) -> Self {
        self.accent = Some(accent);
        self
    }

    pub fn with_style(mut self, style: impl Fn(Div) -> Div + 'static) -> Self {
        self.style = Some(Box::new(style));
        self
    }

    pub fn with_visibility(mut self, visible: impl Fn(&App) -> bool + 'static) -> Self {
        self.visible = Some(Box::new(visible));
        self
    }

    pub fn with_confirm(
        mut self,
        confirm: impl Fn(NaiveDateTime, &mut App) -> Task<Result<(), SharedString>> + 'static,
    ) -> Self {
        self.confirm = Some(Box::new(confirm));
        self
    }

    pub fn value(&self) -> NaiveDateTime {
        self.committed
    }

    pub fn set_value(&mut self, value: NaiveDateTime, cx: &mut Context<Self>) {
        let value = self.normalize(value);
        self.drag = None;
        self.external = value;
        self.committed = value;
        self.user_change = false;
        self.invalidate_result();
        self.retarget(value, true, self.immediate(cx), false);
        cx.notify();
    }

    pub fn set_today(&mut self, today: NaiveDate, cx: &mut Context<Self>) {
        let old_min = self.limit().0;
        self.today = Some(today);
        self.invalidate_labels();
        let new_min = self.limit().0;
        for (lane, offset) in [
            (Lane::Day, (old_min - new_min).num_days() as f32),
            (Lane::Year, (old_min.year() - new_min.year()) as f32),
        ] {
            let spin = &mut self.spins[lane as usize];
            spin.reset(spin.position + offset);
        }
        self.set_value(self.committed, cx);
    }

    pub fn set_paused(&mut self, paused: bool, cx: &mut Context<Self>) {
        self.paused = paused;
        cx.notify();
    }

    fn today(&self) -> NaiveDate {
        self.today
            .unwrap_or(self.start.date())
            .clamp(date(1, 1, 1), date(9999, 12, 31))
    }

    fn limit(&self) -> (NaiveDate, NaiveDate) {
        let today = self.today();
        let is_date = self.mode == DateReelMode::Date;
        let min = self.explicit_min.unwrap_or_else(|| {
            if is_date {
                shift_year(today, 100, false)
            } else {
                shift_day(today, -30)
            }
        });
        let max = self
            .explicit_max
            .unwrap_or_else(|| shift_year(today, if is_date { 10 } else { 1 }, true));
        let supported = date(1, 1, 1)..=date(9999, 12, 31);
        (
            min.min(max).clamp(*supported.start(), *supported.end()),
            max.max(min).clamp(*supported.start(), *supported.end()),
        )
    }

    fn normalize(&self, value: NaiveDateTime) -> NaiveDateTime {
        let (min, max) = self.limit();
        if self.mode == DateReelMode::Date {
            return value.date().clamp(min, max).and_time(value.time());
        }
        let step = self.minute_step as u32;
        let total = value.hour() * 60 + ((value.minute() + step / 2) / step) * step;
        let day = shift_day(value.date(), i64::from(total / (24 * 60))).clamp(min, max);
        let total = if day == max && value.date() >= max && total >= 24 * 60 {
            24 * 60 - step
        } else {
            total % (24 * 60)
        };
        at(day, total / 60, total % 60)
    }

    fn lanes(&self) -> &'static [Lane] {
        let lanes = match self.mode {
            DateReelMode::DateTime => DATETIME,
            DateReelMode::Date => DATE,
            DateReelMode::Time => TIME,
        };
        if self.hour_cycle == HourCycle::TwentyFour && self.mode != DateReelMode::Date {
            &lanes[..lanes.len() - 1]
        } else {
            lanes
        }
    }

    fn range(&self, lane: Lane) -> Range {
        let (min, max) = self.limit();
        let (last, wraps) = match lane {
            Lane::Day => ((max - min).num_days() as i32, false),
            Lane::Month => (11, true),
            Lane::Date => (30, true),
            Lane::Year => (max.year() - min.year(), false),
            Lane::Hour => (
                if self.hour_cycle == HourCycle::Twelve {
                    11
                } else {
                    23
                },
                true,
            ),
            Lane::Minute => (60 / self.minute_step as i32 - 1, true),
            Lane::Period => (1, false),
        };
        Range { last, wraps }
    }

    fn indices(&self, value: NaiveDateTime) -> [i32; 7] {
        let min = self.limit().0;
        [
            (value.date() - min).num_days() as i32,
            value.month0() as i32,
            value.day0() as i32,
            value.year() - min.year(),
            (value.hour()
                % if self.hour_cycle == HourCycle::Twelve {
                    12
                } else {
                    24
                }) as i32,
            (value.minute() / self.minute_step as u32) as i32,
            (value.hour() / 12) as i32,
        ]
    }

    fn reset(&mut self) {
        self.invalidate_labels();
        let value = self.normalize(self.start);
        self.committed = value;
        self.external = value;
        let indices = self.indices(value);
        for (spin, index) in self.spins.iter_mut().zip(indices) {
            spin.reset(index as f32);
        }
    }

    fn moment(&self, goals: bool) -> NaiveDateTime {
        let index = |lane: Lane| {
            let s = &self.spins[lane as usize];
            self.range(lane)
                .index(if goals { s.target } else { s.position }.round() as i32)
        };
        let (min, max) = self.limit();
        let day = match self.mode {
            DateReelMode::DateTime => shift_day(min, i64::from(index(Lane::Day))),
            DateReelMode::Date => {
                let year = min.year() + index(Lane::Year);
                let month = index(Lane::Month) as u32 + 1;
                let day = (index(Lane::Date) as u32 + 1).min(last_day(year, month));
                date(year, month, day).clamp(min, max)
            }
            DateReelMode::Time => self.committed.date(),
        };
        if self.mode == DateReelMode::Date {
            return day.and_time(self.committed.time());
        }
        let mut hour = index(Lane::Hour) as u32;
        if self.hour_cycle == HourCycle::Twelve {
            hour += index(Lane::Period) as u32 * 12;
        }
        at(
            day,
            hour,
            index(Lane::Minute) as u32 * self.minute_step as u32,
        )
    }

    fn retarget(&mut self, value: NaiveDateTime, stagger: bool, immediate: bool, intro: bool) {
        let indices = self.indices(value);
        for (i, &lane) in self.lanes().iter().enumerate() {
            let index = lane as usize;
            let range = self.range(lane);
            let spin = &mut self.spins[index];
            let end = range.nearest(spin.position, indices[index]);
            if intro {
                spin.reset(range.rubber(end - 2. - i as f32 * 0.75));
            }
            if (spin.target - end).abs() > 0.001 || immediate {
                spin.seek(end, if stagger { i as f32 * 0.044 } else { 0. }, immediate);
            }
        }
    }

    fn immediate(&self, cx: &App) -> bool {
        self.paused || disabled_motion(cx)
    }

    fn invalidate_result(&mut self) {
        if self.confirm_state != ConfirmState::Pending {
            self.confirm_state = ConfirmState::Idle;
            self.error = None;
        }
    }

    fn aim(&mut self, lane: Lane, next: f32, cx: &mut Context<Self>) {
        let range = self.range(lane);
        let target = if range.wraps {
            next
        } else {
            next.clamp(0., range.last as f32)
        };
        let immediate = self.immediate(cx);
        self.spins[lane as usize].seek(target, 0., immediate);
        self.user_change = true;
        self.invalidate_result();
        cx.notify();
    }

    fn commit(&mut self, w: &Window, cx: &mut Context<Self>) {
        if self.drag.is_some() || self.spins.iter().any(|s| s.travel.is_some()) || !self.user_change
        {
            return;
        }
        let next = self.moment(true);
        self.retarget(next, false, self.immediate(cx), false);
        if self.spins.iter().any(|s| s.travel.is_some()) {
            return;
        }
        self.user_change = false;
        if next == self.committed {
            return;
        }
        self.committed = next;
        self.announcement = Some(self.summary(next));
        cx.emit(DateReelChanged(next));
        if self.controlled {
            let weak = cx.weak_entity();
            w.on_next_frame(move |_, cx| {
                let _ = weak.update(cx, |this, cx| {
                    if this.external != next && this.committed == next && !this.user_change {
                        this.set_value(this.external, cx);
                    }
                });
            });
        }
    }

    fn begin(
        &mut self,
        lane: Lane,
        y: Pixels,
        row: Pixels,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.drag.is_some() {
            return;
        }
        let spin = &mut self.spins[lane as usize];
        let origin = spin.position;
        let previous_target = spin.target;
        spin.reset(origin);
        self.drag = Some(Drag {
            lane,
            y,
            origin,
            previous_target,
            previous_change: self.user_change,
            last_y: y,
            at: Instant::now(),
            velocity: 0.,
            row,
            moved: false,
        });
        self.focus[lane as usize].focus(w, cx);
        self.invalidate_result();
        cx.notify();
    }

    fn move_drag(&mut self, y: Pixels, cx: &mut Context<Self>) {
        let Some(mut drag) = self.drag else { return };
        let now = Instant::now();
        let dt = now.duration_since(drag.at).as_secs_f32();
        drag.moved |= (y - drag.y).abs() > px(3.);
        if drag.moved {
            let position = drag.origin + f32::from(drag.y - y) / f32::from(drag.row);
            let range = self.range(drag.lane);
            self.spins[drag.lane as usize].reset(range.rubber(position));
            if dt > 0. {
                let sample = f32::from(drag.last_y - y) / f32::from(drag.row) / dt;
                drag.velocity = if dt > 0.09 {
                    sample
                } else {
                    drag.velocity * 0.32 + sample * 0.68
                };
            }
            drag.last_y = y;
            drag.at = now;
        }
        self.drag = Some(drag);
        cx.notify();
    }

    fn end(&mut self, lane: Lane, y: Pixels, cancelled: bool, cx: &mut Context<Self>) {
        let Some(drag) = self.drag.filter(|d| d.lane == lane) else {
            return;
        };
        if !cancelled && (y - drag.last_y).abs() > px(0.1) {
            self.move_drag(y, cx);
        }
        let drag = self.drag.take().unwrap_or(drag);
        if cancelled {
            let immediate = self.immediate(cx);
            self.spins[lane as usize].seek(drag.previous_target, 0., immediate);
            self.user_change = drag.previous_change;
            cx.notify();
        } else if !drag.moved {
            let bounds = self.lane_bounds[lane as usize].get();
            let offset = f32::from(y - bounds.center().y);
            let row = f32::from(drag.row);
            let angle = (offset * WHEEL_ANGLE / row).clamp(-1., 1.).asin();
            self.aim(lane, (drag.origin + angle / WHEEL_ANGLE).round(), cx);
        } else {
            let range = self.range(lane);
            let age = drag.at.elapsed().as_secs_f32();
            let speed = drag.velocity * (1. - age / 0.10).clamp(0., 1.);
            let immediate = self.immediate(cx);
            self.spins[lane as usize].coast(range, speed, immediate);
            self.user_change = true;
            cx.notify();
        }
    }

    fn wheel(&mut self, lane: Lane, event: &ScrollEvent, w: &mut Window, cx: &mut Context<Self>) {
        if self.drag.is_some() || event.modifiers.control || event.modifiers.platform {
            return;
        }
        let delta = match event.delta {
            ScrollDelta::Lines(p) => -p.y,
            ScrollDelta::Pixels(p) => -f32::from(p.y) / 42.,
        };
        if delta.abs() < 0.01 {
            return;
        }
        let now = Instant::now();
        if self
            .wheel_lane
            .is_none_or(|(last, when)| last != lane || now.duration_since(when).as_millis() > 150)
        {
            self.wheel_remainder = 0.;
        }
        self.wheel_lane = Some((lane, now));
        self.wheel_remainder += delta;
        let rows = self.wheel_remainder.round().clamp(-9., 9.);
        if rows.abs() >= 1. {
            self.wheel_remainder -= rows;
            let target = self.spins[lane as usize].target;
            self.aim(lane, target + rows, cx);
        }
        cx.stop_propagation();
        w.prevent_default();
    }

    fn key(&mut self, lane: Lane, event: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        let key = event.keystroke.key.as_str();
        let range = self.range(lane);
        let current = self.spins[lane as usize].target;
        let page = match lane {
            Lane::Day | Lane::Date => 7.,
            Lane::Month => 3.,
            Lane::Year => 10.,
            Lane::Hour => {
                if self.hour_cycle == HourCycle::Twelve {
                    3.
                } else {
                    6.
                }
            }
            Lane::Minute => (15. / self.minute_step as i32 as f32).round().max(1.),
            Lane::Period => 1.,
        };
        let next = match key {
            "up" => Some(current - 1.),
            "down" => Some(current + 1.),
            "pageup" => Some(current - page),
            "pagedown" => Some(current + page),
            "home" => Some(range.nearest(current, 0)),
            "end" => Some(range.nearest(current, range.last)),
            "left" | "right" => {
                let lanes = self.lanes();
                let i = lanes.iter().position(|&value| value == lane).unwrap_or(0);
                let adjacent = if key == "left" {
                    i.saturating_sub(1)
                } else {
                    (i + 1).min(lanes.len() - 1)
                };
                self.focus[lanes[adjacent] as usize].focus(w, cx);
                None
            }
            _ if !event.keystroke.modifiers.platform
                && !event.keystroke.modifiers.control
                && key.chars().count() == 1
                && key.chars().all(char::is_alphanumeric) =>
            {
                let now = Instant::now();
                let mut search = self
                    .search
                    .take()
                    .filter(|(old, _, last)| {
                        *old == lane && now.duration_since(*last).as_millis() < 700
                    })
                    .map_or_else(String::new, |(_, old, _)| old);
                search.push_str(&key.to_lowercase());
                let hit = self.find(lane, &search, range, current).or_else(|| {
                    search = key.to_lowercase();
                    self.find(lane, &search, range, current)
                });
                self.search = Some((lane, search, now));
                hit
            }
            _ => return,
        };
        if let Some(next) = next {
            self.aim(lane, next, cx);
        }
        w.prevent_default();
        cx.stop_propagation();
    }

    fn find(&self, lane: Lane, query: &str, range: Range, current: f32) -> Option<f32> {
        if lane == Lane::Year {
            let lower = self.limit().0.year();
            let upper = lower + range.last;
            let first = query.parse::<i32>().ok()?;
            if (lower..=upper).contains(&first) {
                return Some((first - lower) as f32);
            }
            if first < 0 || query.len() > 6 {
                return None;
            }
            for scale in [1, 10, 100, 1000, 10000, 100000] {
                let lo = first.saturating_mul(scale);
                let hi = lo.saturating_add(scale - 1);
                if lo <= upper && hi >= lower {
                    return Some((lo.max(lower) - lower) as f32);
                }
            }
            return None;
        }
        let active = self.moment(true);
        let limit = range
            .last
            .min(if lane == Lane::Day { 732 } else { range.last });
        let matches = |index| {
            let value = self.entry(lane, index, active).0.to_lowercase();
            value == query || value.trim_start_matches('0') == query
        };
        let starts = |index| {
            self.entry(lane, index, active)
                .0
                .to_lowercase()
                .starts_with(query)
        };
        (0..=limit)
            .find(|&i| matches(i))
            .or_else(|| (0..=limit).find(|&i| starts(i)))
            .map(|index| range.nearest(current, index))
    }

    fn activate_preset(&mut self, value: NaiveDateTime, cx: &mut Context<Self>) {
        self.drag = None;
        self.user_change = true;
        self.invalidate_result();
        self.retarget(value, true, self.immediate(cx), false);
        cx.notify();
    }

    fn schedule(&mut self, cx: &mut Context<Self>) {
        if matches!(
            self.confirm_state,
            ConfirmState::Pending | ConfirmState::Success
        ) || self.drag.is_some()
            || self.spins.iter().any(|s| s.travel.is_some())
        {
            return;
        }
        let Some(confirm) = &self.confirm else { return };
        let target = self.committed;
        let job = confirm(target, cx);
        self.confirm_state = ConfirmState::Pending;
        self.confirm_epoch = self.confirm_epoch.wrapping_add(1);
        let epoch = self.confirm_epoch;
        self.error = None;
        cx.notify();
        self.task = Some(cx.spawn(async move |this, cx| {
            let result = job.await;
            let _ = this.update(cx, |this, cx| {
                if this.confirm_epoch != epoch {
                    return;
                }
                let words = this.words();
                if target != this.committed {
                    this.confirm_state = ConfirmState::Idle;
                    this.error = None;
                } else {
                    match result {
                        Ok(()) => {
                            this.confirm_state = ConfirmState::Success;
                            this.announcement = Some(words.success);
                        }
                        Err(error) => {
                            this.confirm_state = ConfirmState::Error;
                            this.announcement = Some(error.clone());
                            this.error = Some(error);
                        }
                    }
                }
                cx.notify();
            });
        }));
    }
}

const MONTHS_ZH: [&str; 12] = [
    "1月", "2月", "3月", "4月", "5月", "6月", "7月", "8月", "9月", "10月", "11月", "12月",
];
const WEEK_ZH: [&str; 7] = [
    "星期日",
    "星期一",
    "星期二",
    "星期三",
    "星期四",
    "星期五",
    "星期六",
];
const SHORT_WEEK_ZH: [&str; 7] = ["周日", "周一", "周二", "周三", "周四", "周五", "周六"];

impl DateReel {
    fn invalidate_labels(&mut self) {
        self.preset_values.take();
        for lane in self.row_labels.get_mut() {
            lane.clear();
        }
    }

    fn words(&self) -> DateReelLabels {
        self.labels
            .clone()
            .unwrap_or_else(|| DateReelLabels::for_locale(self.locale))
    }

    fn formatted(&self, kind: DateReelText, value: NaiveDateTime) -> Option<SharedString> {
        (self.formatter.as_ref()?)(kind, value, self.today(), self.hour_cycle)
    }

    fn row_label(&self, lane: Lane, index: i32, active: NaiveDateTime) -> (SharedString, bool) {
        let context = if self.formatter.is_some() {
            Some(active)
        } else if matches!(lane, Lane::Date | Lane::Month) {
            Some(at(date(active.year(), active.month(), 1), 0, 0))
        } else {
            None
        };
        let mut cache = self.row_labels.borrow_mut();
        let rows = &mut cache[lane as usize];
        if let Some(row) = rows
            .iter()
            .find(|row| row.index == index && row.context == context)
        {
            return row.entry.clone();
        }
        let entry = self.entry(lane, index, active);
        if rows.len() >= 24 {
            rows.pop_front();
        }
        rows.push_back(CachedEntry {
            index,
            context,
            entry: entry.clone(),
        });
        entry
    }

    fn entry(&self, lane: Lane, index: i32, active: NaiveDateTime) -> (SharedString, bool) {
        let zh = self.locale == DateReelLocale::Chinese;
        let (min, max) = self.limit();
        let invalid_date =
            lane == Lane::Date && index as u32 + 1 > last_day(active.year(), active.month());
        let entry = match lane {
            Lane::Day => at(
                shift_day(min, i64::from(index)),
                active.hour(),
                active.minute(),
            ),
            Lane::Month => at(date(active.year(), index as u32 + 1, 1), 0, 0),
            Lane::Date if !invalid_date => {
                at(date(active.year(), active.month(), index as u32 + 1), 0, 0)
            }
            Lane::Year => at(date(min.year() + index, 1, 1), 0, 0),
            Lane::Hour => at(active.date(), index as u32, active.minute()),
            Lane::Minute => at(
                active.date(),
                active.hour(),
                index as u32 * self.minute_step as u32,
            ),
            Lane::Period => at(active.date(), index as u32 * 12, active.minute()),
            Lane::Date => active,
        };
        let available = match lane {
            Lane::Date => !invalid_date && (min..=max).contains(&entry.date()),
            Lane::Month => {
                let first = entry.date();
                let end = date(
                    first.year(),
                    first.month(),
                    last_day(first.year(), first.month()),
                );
                first <= max && end >= min
            }
            _ => true,
        };
        if !invalid_date && let Some(text) = self.formatted(lane.text(), entry) {
            return (text, available);
        }
        let value: SharedString = match lane {
            Lane::Day if entry.date() == self.today() => if zh { "今天" } else { "Today" }.into(),
            Lane::Day if zh => format!(
                "{} {}月{}日",
                SHORT_WEEK_ZH[entry.weekday().num_days_from_sunday() as usize],
                entry.month(),
                entry.day()
            )
            .into(),
            Lane::Day => entry.format("%a, %b %-d").to_string().into(),
            Lane::Month if zh => MONTHS_ZH[index as usize].into(),
            Lane::Month => entry.format("%B").to_string().into(),
            Lane::Date => (index + 1).to_string().into(),
            Lane::Year => entry.year().to_string().into(),
            Lane::Hour if self.hour_cycle == HourCycle::Twelve => {
                (if index == 0 { 12 } else { index }).to_string().into()
            }
            Lane::Hour => format!("{index:02}").into(),
            Lane::Minute => format!("{:02}", entry.minute()).into(),
            Lane::Period if zh => if index == 0 { "上午" } else { "下午" }.into(),
            Lane::Period => if index == 0 { "AM" } else { "PM" }.into(),
        };
        (value, available)
    }

    fn full_date(&self, value: NaiveDateTime) -> SharedString {
        self.formatted(DateReelText::FullDate, value)
            .unwrap_or_else(|| {
                if self.locale == DateReelLocale::Chinese {
                    format!(
                        "{}年{}月{}日 {}",
                        value.year(),
                        value.month(),
                        value.day(),
                        WEEK_ZH[value.weekday().num_days_from_sunday() as usize]
                    )
                    .into()
                } else {
                    value.format("%A, %B %-d, %Y").to_string().into()
                }
            })
    }

    fn summary(&self, value: NaiveDateTime) -> SharedString {
        if let Some(summary) = self.formatted(DateReelText::Summary, value) {
            return summary;
        }
        let zh = self.locale == DateReelLocale::Chinese;
        let delta = (value.date() - self.today()).num_days();
        let day = if self.mode == DateReelMode::Time {
            String::new()
        } else {
            match (zh, delta) {
                (false, -1) => "Yesterday".into(),
                (false, 0) => "Today".into(),
                (false, 1) => "Tomorrow".into(),
                (false, 2..=6) => value.format("%A").to_string(),
                (false, 7..=30) => format!("In {delta} days"),
                (true, -1) => "昨天".into(),
                (true, 0) => "今天".into(),
                (true, 1) => "明天".into(),
                (true, 2..=6) => WEEK_ZH[value.weekday().num_days_from_sunday() as usize].into(),
                (true, 7..=30) => format!("{delta} 天后"),
                (true, _) => format!("{}年{}月{}日", value.year(), value.month(), value.day()),
                (false, _) => value.format("%b %-d, %Y").to_string(),
            }
        };
        if self.mode == DateReelMode::Date {
            return day.into();
        }
        let time = if self.hour_cycle == HourCycle::TwentyFour {
            value.format("%H:%M").to_string()
        } else if zh {
            let (pm, hour) = value.hour12();
            format!(
                "{}{hour}:{:02}",
                if pm { "下午" } else { "上午" },
                value.minute()
            )
        } else {
            value.format("%-I:%M %p").to_string()
        };
        if day.is_empty() {
            time.into()
        } else {
            format!("{day}{}{time}", self.words().at).into()
        }
    }

    fn quick_picks(&self) -> &[(SharedString, NaiveDateTime)] {
        self.preset_values.get_or_init(|| self.build_quick_picks())
    }

    fn build_quick_picks(&self) -> Vec<(SharedString, NaiveDateTime)> {
        let today = self.today();
        if let Some(presets) = &self.presets {
            return presets
                .iter()
                .map(|p| (p.label.clone(), self.normalize((p.value)(today))))
                .collect();
        }
        if self.mode != DateReelMode::DateTime {
            return Vec::new();
        }
        let zh = self.locale == DateReelLocale::Chinese;
        let next_monday = 7 - i64::from(today.weekday().num_days_from_monday());
        [
            (if zh { "今晚" } else { "This evening" }, at(today, 18, 0)),
            (
                if zh { "明天" } else { "Tomorrow" },
                at(shift_day(today, 1), 9, 0),
            ),
            (
                if zh { "周一" } else { "Monday" },
                at(shift_day(today, next_monday), 9, 0),
            ),
        ]
        .into_iter()
        .map(|(text, value)| (text.into(), self.normalize(value)))
        .collect()
    }

    #[allow(clippy::too_many_arguments)]
    fn reels(
        &self,
        metrics: Metrics,
        p: Paint,
        accent: Hsla,
        words: &DateReelLabels,
        active: NaiveDateTime,
        w: &Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let radius = surface_radius(cx);
        let row = metrics.row;
        let height = row * 5.6;
        let committed = self.indices(self.committed);
        let zh = self.locale == DateReelLocale::Chinese;
        let width_factor = (f32::from(metrics.width - metrics.inset * 2.) / 263.).clamp(0.55, 1.);
        let line = p.line;
        let lens_fill = p.panel.blend(p.ink.opacity(0.045));
        let mut reels = div()
            .relative()
            .h(height)
            .w_full()
            .flex()
            .justify_center()
            .child(
                div()
                    .absolute()
                    .left_0()
                    .right_0()
                    .top((height - row) * 0.5)
                    .h(row)
                    .rounded(radius)
                    .bg(lens_fill)
                    .border_1()
                    .border_color(line),
            );
        for &lane in self.lanes() {
            let i = lane as usize;
            let range = self.range(lane);
            let position = self.spins[i].position;
            let selected = range.index(committed[i]);
            let label = lane.label(words).clone();
            let value = self.row_label(lane, selected, self.committed).0;
            let focus = self.focus[i].clone();
            let active_color = if focus.is_focused(w) { accent } else { p.ink };
            let align_right = matches!(lane, Lane::Day | Lane::Month);
            let width = match lane {
                Lane::Day => {
                    if metrics.compact {
                        if zh { 145. } else { 128. }
                    } else {
                        161.
                    }
                }
                Lane::Month => {
                    if metrics.compact {
                        107.
                    } else {
                        131.
                    }
                }
                Lane::Year => {
                    if metrics.compact {
                        70.
                    } else {
                        86.
                    }
                }
                Lane::Period => {
                    if metrics.compact {
                        if zh { 44. } else { 47. }
                    } else {
                        59.
                    }
                }
                _ => {
                    if metrics.compact {
                        if zh { 36. } else { 39. }
                    } else {
                        52.
                    }
                }
            };
            let rows: Vec<_> = (-HALF_ROWS..=HALF_ROWS)
                .filter_map(|offset| {
                    let raw = position.round() as i32 + offset;
                    if !range.wraps && !(0..=range.last).contains(&raw) {
                        return None;
                    }
                    let (text, available) = self.row_label(lane, range.index(raw), active);
                    Some((raw as f32 - position, text, available))
                })
                .collect();
            let bounds = self.lane_bounds[i].clone();
            let font_size = metrics.font;
            let padding = px(if metrics.compact { 8. } else { 12. });
            let wheel = canvas(
                move |bounds_value, _, _| bounds.set(bounds_value),
                move |b, _, w, cx| {
                    if let Some(host) = cx.try_global::<EditorScrollRegion>() {
                        (host.0)(b, true, true);
                    }
                    paint_wheel(
                        b,
                        row,
                        font_size,
                        padding,
                        align_right,
                        p,
                        active_color,
                        &rows,
                        w,
                        cx,
                    );
                },
            )
            .absolute()
            .inset_0();
            let mut column = div()
                .id(("date-reel-lane", i))
                .relative()
                .w(px(width * width_factor))
                .h_full()
                .flex_none()
                .rounded(radius)
                .border_1()
                .border_color(transparent_black())
                .focus_visible(move |s| s.border_color(p.focus))
                .role(Role::SpinButton)
                .aria_label(label)
                .aria_min_numeric_value(0.)
                .aria_max_numeric_value(f64::from(range.last))
                .aria_numeric_value(f64::from(selected))
                .aria_value(value)
                .track_focus(&focus)
                .cursor_pointer()
                .on_key_down(cx.listener(move |s, e, w, cx| s.key(lane, e, w, cx)))
                .on_scroll(cx.listener(move |s, e, w, cx| s.wheel(lane, e, w, cx)))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |s, e: &MouseDownEvent, w, cx| {
                        if !w.last_input_was_touch() {
                            s.begin(lane, e.position.y, row, w, cx)
                        }
                    }),
                )
                .on_mouse_up(
                    MouseButton::Left,
                    cx.listener(move |s, e: &MouseUpEvent, _, cx| {
                        s.end(lane, e.position.y, false, cx)
                    }),
                )
                .on_mouse_up_out(
                    MouseButton::Left,
                    cx.listener(move |s, e: &MouseUpEvent, _, cx| {
                        s.end(lane, e.position.y, false, cx)
                    }),
                )
                .child(wheel)
                .child(
                    self.contact[i].listen(cx, move |s, e, w, cx| match e.phase {
                        TouchPhase::Started => s.begin(lane, e.position.y, row, w, cx),
                        TouchPhase::Moved => s.move_drag(e.position.y, cx),
                        TouchPhase::Ended => s.end(lane, e.position.y, false, cx),
                        TouchPhase::Cancelled => s.end(lane, e.position.y, true, cx),
                    }),
                );
            if self.drag.is_some_and(|drag| drag.lane == lane) {
                let listener = cx.listener(move |s, e: &MouseMoveEvent, w, cx| {
                    if e.pressed_button == Some(MouseButton::Left) {
                        s.move_drag(e.position.y, cx)
                    } else if !w.last_input_was_touch() {
                        s.end(lane, e.position.y, false, cx)
                    }
                });
                let release = cx.listener(move |s, e: &MouseUpEvent, _, cx| {
                    if e.button == MouseButton::Left {
                        s.end(lane, e.position.y, false, cx);
                    }
                });
                column = column.child(
                    canvas(
                        |_, _, _| (),
                        move |_, _, w, _| {
                            w.on_mouse_event(move |e: &MouseMoveEvent, phase, w, cx| {
                                if phase.capture() {
                                    listener(e, w, cx);
                                }
                            });
                            w.on_mouse_event(move |e: &MouseUpEvent, phase, w, cx| {
                                if phase.capture() {
                                    release(e, w, cx);
                                }
                            });
                        },
                    )
                    .absolute()
                    .size(px(0.)),
                );
            }
            reels = reels.child(column);
        }
        reels
    }

    fn footer(
        &self,
        metrics: Metrics,
        words: &DateReelLabels,
        p: Paint,
        accent: Hsla,
        w: &Window,
        cx: &mut Context<Self>,
    ) -> Option<Div> {
        let options = self.quick_picks();
        let has_options = !options.is_empty();
        if !has_options && self.confirm.is_none() {
            return None;
        }
        let radius = surface_radius(cx);
        let moving = self.drag.is_some() || self.spins.iter().any(|s| s.travel.is_some());
        let mut content = div()
            .w_full()
            .flex()
            .gap(px(if metrics.compact { 10. } else { 8. }))
            .border_t_1()
            .border_color(p.line)
            .pt(px(16.));
        content = if metrics.compact {
            content.flex_col()
        } else {
            content.justify_between().items_center()
        };
        let mut chips = div()
            .id("date-reel-presets")
            .role(Role::Group)
            .aria_label(words.quick_picks.clone())
            .flex()
            .flex_wrap()
            .gap(px(8.));
        for (i, (label, preset)) in options.iter().cloned().enumerate() {
            let matched = self.committed == preset;
            let id = ("date-reel-preset", i);
            chips = chips.child(
                face(id, label.clone(), 36., f32::from(radius), Tone::Clear, cx)
                    .px(px(if metrics.compact { 8. } else { 12. }))
                    .border_1()
                    .border_color(if matched { accent.opacity(0.5) } else { p.line })
                    .bg(if matched {
                        accent.opacity(0.1)
                    } else {
                        transparent_black()
                    })
                    .hover(move |s| s.bg(p.hover))
                    .font_weight(FontWeight::NORMAL)
                    .text_color(if matched { accent } else { p.secondary })
                    .aria_toggled(matched.into())
                    .child(label)
                    .on_click(cx.listener(move |s, _, _, cx| s.activate_preset(preset, cx)))
                    .on_key_down(cx.listener(move |s, e: &KeyDownEvent, w, cx| {
                        if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                            s.activate_preset(preset, cx);
                            w.prevent_default();
                            cx.stop_propagation();
                        }
                    })),
            );
        }
        if has_options {
            content = content.child(chips);
        }
        if self.confirm.is_some() {
            let text = match self.confirm_state {
                ConfirmState::Idle => words.confirm.clone(),
                ConfirmState::Pending => words.pending.clone(),
                ConfirmState::Success => words.success.clone(),
                ConfirmState::Error => words.retry.clone(),
            };
            let busy = moving
                || matches!(
                    self.confirm_state,
                    ConfirmState::Pending | ConfirmState::Success
                );
            let width = [&words.confirm, &words.pending, &words.success, &words.retry]
                .into_iter()
                .map(|text| measure(text, 14., w))
                .fold(0., f32::max);
            content = content.child(
                face(
                    "date-reel-confirm",
                    text.clone(),
                    if metrics.compact { 44. } else { 36. },
                    f32::from(radius),
                    Tone::Primary,
                    cx,
                )
                .when(metrics.compact, |e| e.w_full())
                .when(!metrics.compact, |e| e.w(px((width + 36.).max(127.4))))
                .bg(if busy && self.confirm_state != ConfirmState::Success {
                    p.bright.opacity(0.64)
                } else {
                    p.bright
                })
                .when(busy, |e| e.cursor_not_allowed())
                .track_focus(&self.confirm_focus)
                .child(text)
                .on_click(cx.listener(|s, _, _, cx| s.schedule(cx)))
                .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                    if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                        s.schedule(cx);
                        w.prevent_default();
                        cx.stop_propagation();
                    }
                })),
            );
        }
        Some(content)
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_wheel(
    b: Bounds<Pixels>,
    row: Pixels,
    font_size: Pixels,
    padding: Pixels,
    right: bool,
    p: Paint,
    active: Hsla,
    rows: &[(f32, SharedString, bool)],
    w: &mut Window,
    cx: &mut App,
) {
    let center = b.center().y;
    let lens = Bounds::new(point(b.left(), center - row * 0.5), size(b.size.width, row));
    let font = w.text_style().font();
    for &(offset, ref value, enabled) in rows {
        let angle = offset * WHEEL_ANGLE;
        if angle.abs() >= std::f32::consts::FRAC_PI_2 {
            continue;
        }
        let y_scale = angle.cos();
        let y = center + row * (angle.sin() / WHEEL_ANGLE);
        let size = px((f32::from(font_size) * y_scale * 2.).round() / 2.);
        let x_scale = 0.8 + 0.2 * y_scale;
        let visibility = ((y_scale - 0.55) / 0.45).clamp(0., 1.).powf(1.45);
        let opacity = if enabled { 1. } else { 0.32 };
        let masks = [
            (
                Bounds::new(b.origin, gpui::size(b.size.width, lens.top() - b.top())),
                p.ink.opacity(0.66 * visibility),
            ),
            (lens, active),
            (
                Bounds::new(
                    point(b.left(), lens.bottom()),
                    gpui::size(b.size.width, b.bottom() - lens.bottom()),
                ),
                p.ink.opacity(0.66 * visibility),
            ),
        ];
        for (clip, color) in masks {
            if y + size * 0.6 < clip.top() || y - size * 0.6 > clip.bottom() || color.a < 0.01 {
                continue;
            }
            let mut font = font.clone();
            font.weight = if clip == lens {
                FontWeight::MEDIUM
            } else {
                FontWeight::NORMAL
            };
            let line = w.text_system().shape_line(
                value.clone(),
                size,
                &[TextRun {
                    len: value.len(),
                    font,
                    color: color.opacity(opacity),
                    background_color: None,
                    underline: None,
                    strikethrough: None,
                }],
                None,
            );
            let x = if right {
                b.right() - padding - line.width
            } else {
                b.center().x - line.width * 0.5
            };
            let center_shift = if right {
                (1. - x_scale) * padding
            } else {
                px(0.)
            };
            w.with_content_mask(Some(ContentMask { bounds: clip }), |w| {
                let _ = line.paint(
                    point(x + center_shift, y - size * 0.5),
                    size,
                    TextAlign::Left,
                    None,
                    w,
                    cx,
                );
            });
        }
    }
}

impl Render for DateReel {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let now = Instant::now();
        let radius = surface_radius(cx);
        let b = self.bounds.get();
        let view = Bounds::new(Point::default(), w.viewport_size());
        let visible = w.is_visible()
            && (b.size.width == px(0.) || b.intersects(&view))
            && self.visible.as_ref().is_none_or(|predicate| predicate(cx));
        let immediate = self.immediate(cx);
        if visible && !self.entered {
            self.entered = true;
            if self.intro && !immediate {
                self.retarget(self.committed, true, false, true);
            }
        }
        if visible {
            let dt = self
                .last_frame
                .map_or(0., |last| now.duration_since(last).as_secs_f32())
                .min(0.05)
                * self.speed;
            let dt = dt.min(2.);
            self.last_frame = Some(now);
            for spin in &mut self.spins {
                if spin.travel.is_some() && immediate {
                    spin.reset(spin.target);
                } else {
                    spin.advance(dt);
                }
            }
        } else {
            self.last_frame = None;
        }
        self.commit(w, cx);
        if visible && self.spins.iter().any(|s| s.travel.is_some()) {
            w.request_animation_frame();
        } else {
            self.last_frame = None;
        }
        let width = if b.size.width > px(0.) {
            b.size.width.min(w.viewport_size().width).min(px(MAX_WIDTH))
        } else {
            w.viewport_size().width.min(px(MAX_WIDTH))
        };
        let metrics = Metrics::new(width.max(px(200.)));
        let accent = self.accent.unwrap_or(p.bright);
        let words = self.words();
        let active = self.moment(false);
        let lang = self.locale;
        let label = self.label.clone().unwrap_or_else(|| {
            if lang == DateReelLocale::Chinese {
                "日期和时间"
            } else {
                "Date and time"
            }
            .into()
        });
        let mut face = div()
            .w_full()
            .max_w(px(MAX_WIDTH))
            .relative()
            .flex()
            .flex_col()
            .gap(px(16.))
            .p(metrics.inset)
            .rounded(radius)
            .border_1()
            .border_color(p.line)
            .bg(p.panel)
            .text_color(p.ink)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .px(px(4.))
                    .children(self.title.as_ref().map(|title| {
                        div()
                            .text_size(px(14.))
                            .line_height(px(16.))
                            .text_color(p.secondary)
                            .child(title.clone())
                    }))
                    .child(
                        div()
                            .text_size(px(22.))
                            .line_height(px(31.))
                            .font_weight(FontWeight::MEDIUM)
                            .child(self.summary(active)),
                    )
                    .when(self.mode == DateReelMode::DateTime, |e| {
                        e.child(
                            div()
                                .text_size(px(14.))
                                .line_height(px(19.))
                                .text_color(p.secondary)
                                .child(self.full_date(active)),
                        )
                    }),
            )
            .child(self.reels(metrics, p, accent, &words, active, w, cx))
            .children(self.footer(metrics, &words, p, accent, w, cx))
            .children(self.error.as_ref().map(|message| {
                div()
                    .text_size(px(12.))
                    .text_color(p.danger)
                    .child(message.clone())
            }))
            .children(self.announcement.as_ref().map(|message| {
                div()
                    .id("date-reel-status")
                    .absolute()
                    .size(px(0.))
                    .role(Role::Status)
                    .aria_label(message.clone())
            }));
        if let Some(style) = &self.style {
            face = style(face);
        }
        let bounds = self.bounds.clone();
        div()
            .on_children_prepainted(move |items, w, _| {
                if let Some(&next) = items.first() {
                    let old = bounds.replace(next);
                    if old.size.width != next.size.width
                        || old.intersects(&view) != next.intersects(&view)
                    {
                        w.request_animation_frame();
                    }
                }
            })
            .w_full()
            .max_w(px(MAX_WIDTH))
            .id(self.id.clone())
            .role(Role::Group)
            .aria_label(label)
            .child(face)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ConfirmState, DateReel, DateReelMode, HourCycle, Lane, MinuteStep, Range, Spin, Travel, at,
        date,
    };
    use gpui::{Task, TestAppContext};
    use std::{cell::Cell, rc::Rc};

    #[test]
    fn a_fling_glides_then_snaps_and_retargeting_keeps_position() {
        let range = Range {
            last: 60,
            wraps: false,
        };
        let mut spin = Spin::default();
        spin.reset(12.3);
        spin.coast(range, 32., false);
        assert!(matches!(spin.travel, Some(Travel::Glide { .. })));
        spin.advance(0.08);
        assert!(spin.position > 12.3 && spin.position < spin.target);
        let position = spin.position;
        let velocity = spin.velocity;
        spin.seek(9., 0., false);
        assert_eq!(spin.position, position);
        assert_eq!(spin.velocity, velocity);
        for _ in 0..80 {
            spin.advance(1. / 60.);
        }
        assert_eq!(spin.position, 9.);
        assert!(spin.travel.is_none());
    }

    #[test]
    fn finite_reels_resist_overscroll_and_wraps_take_the_short_path() {
        let finite = Range {
            last: 10,
            wraps: false,
        };
        assert!((-1. ..0.).contains(&finite.rubber(-8.)));
        assert!((10. ..11.).contains(&finite.rubber(18.)));
        let cycle = Range {
            last: 11,
            wraps: true,
        };
        assert_eq!(cycle.nearest(11.5, 0), 12.);
        assert_eq!(cycle.nearest(0.3, 11), -1.);
        let mut spin = Spin::default();
        spin.reset(finite.rubber(18.));
        spin.coast(finite, 22., true);
        assert_eq!(spin.position, 10.);
    }

    #[test]
    fn changing_month_in_a_leap_year_corrects_the_day_before_commit() {
        let mut app = TestAppContext::single();
        app.update(|cx| crate::init(cx));
        let start = at(date(2028, 1, 31), 9, 17);
        let (control, window) = app.add_window_view(|_, cx| {
            DateReel::new(cx)
                .with_mode(DateReelMode::Date)
                .with_default_value(start)
                .with_paused(true)
        });
        window.update(|w, cx| {
            control.update(cx, |reel, cx| {
                reel.aim(Lane::Month, 1., cx);
                assert_eq!(reel.moment(true).date(), date(2028, 2, 29));
                assert!(!reel.entry(Lane::Date, 30, reel.moment(true)).1);
                reel.commit(w, cx);
                assert_eq!(reel.value(), at(date(2028, 2, 29), 9, 17));
                assert_eq!(reel.spins[Lane::Date as usize].position, 28.);
                assert!(!reel.user_change);
            });
        });
    }

    #[test]
    fn changing_today_preserves_the_selected_civil_moment() {
        let mut app = TestAppContext::single();
        app.update(|cx| crate::init(cx));
        let start = at(date(2026, 9, 24), 23, 59);
        let (control, window) = app.add_window_view(|_, cx| {
            DateReel::new(cx)
                .with_default_value(start)
                .with_hour_cycle(HourCycle::TwentyFour)
                .with_minute_step(MinuteStep::Fifteen)
                .with_intro(false)
        });
        window.update(|_, cx| {
            control.update(cx, |reel, cx| {
                assert_eq!(reel.value(), at(date(2026, 9, 25), 0, 0));
                assert!(!reel.lanes().contains(&Lane::Period));
                let position = reel.spins[Lane::Day as usize].position;
                reel.set_today(date(2026, 9, 25), cx);
                assert_eq!(reel.value(), at(date(2026, 9, 25), 0, 0));
                assert_eq!(reel.spins[Lane::Day as usize].position, position - 1.);
                assert_eq!(reel.summary(reel.value()).as_ref(), "Today at 00:00");
            });
        });
    }

    #[test]
    fn a_failed_confirmation_keeps_the_value_and_retries_once() {
        let mut app = TestAppContext::single();
        app.update(|cx| crate::init(cx));
        let attempts = Rc::new(Cell::new(0));
        let count = attempts.clone();
        let (control, window) = app.add_window_view(|_, cx| {
            DateReel::new(cx)
                .with_intro(false)
                .with_confirm(move |_, _| {
                    count.set(count.get() + 1);
                    Task::ready(if count.get() == 1 {
                        Err("Offline".into())
                    } else {
                        Ok(())
                    })
                })
        });
        let initial = window.update(|_, cx| control.read(cx).value());
        window.update(|_, cx| {
            control.update(cx, |reel, cx| {
                reel.schedule(cx);
                reel.schedule(cx);
                assert!(reel.confirm_state == ConfirmState::Pending);
            });
        });
        window.run_until_parked();
        window.update(|_, cx| {
            control.update(cx, |reel, cx| {
                assert!(reel.confirm_state == ConfirmState::Error);
                assert_eq!(reel.error.as_deref(), Some("Offline"));
                assert_eq!(reel.value(), initial);
                reel.schedule(cx);
            });
        });
        window.run_until_parked();
        window.update(|_, cx| {
            control.update(cx, |reel, cx| {
                assert!(reel.confirm_state == ConfirmState::Success);
                assert_eq!(reel.value(), initial);
                reel.schedule(cx);
            });
        });
        assert_eq!(attempts.get(), 2);
    }
}
