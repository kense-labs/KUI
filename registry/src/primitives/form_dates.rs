//! Civil calendar arithmetic without timezone or DST conversions in the selection path.
use super::{
    family::{Paint, Slide, Tone, face},
    form_chrome::*,
    glyph::{Glyph, transformed},
};
use gpui::{prelude::*, *};
use std::{cell::Cell, marker::PhantomData, rc::Rc};
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CivilDate {
    pub year: i32,
    pub month: u8,
    pub day: u8,
}
impl CivilDate {
    pub fn new(year: i32, month: u8, day: u8) -> Option<Self> {
        let d = Self { year, month, day };
        ((1..=9999).contains(&year) && (1..=12).contains(&month) && day >= 1 && day <= d.days())
            .then_some(d)
    }
    fn days(self) -> u8 {
        match self.month {
            2 if self.year % 4 == 0 && (self.year % 100 != 0 || self.year % 400 == 0) => 29,
            2 => 28,
            4 | 6 | 9 | 11 => 30,
            _ => 31,
        }
    }
    fn serial(self) -> i64 {
        let m = self.month as i64;
        let y = self.year as i64 - i64::from(m <= 2);
        let era = y.div_euclid(400);
        let yo = y - era * 400;
        era * 146097 + yo * 365 + yo / 4 - yo / 100
            + (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5
            + i64::from(self.day)
            - 1
            - 719468
    }
    fn from_serial(s: i64) -> Self {
        let z = s + 719468;
        let era = z.div_euclid(146097);
        let doe = z - era * 146097;
        let y = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
        let doy = doe - (365 * y + y / 4 - y / 100);
        let mp = (5 * doy + 2) / 153;
        let day = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        Self {
            year: (y + era * 400 + i64::from(m <= 2)) as i32,
            month: m as u8,
            day: day as u8,
        }
    }
    pub fn today_utc() -> Self {
        let t = web_time::SystemTime::now()
            .duration_since(web_time::UNIX_EPOCH)
            .unwrap_or_default();
        Self::from_serial((t.as_secs() / 86400) as i64)
    }
    pub fn add_days(self, n: i32) -> Self {
        Self::from_serial(self.serial() + i64::from(n))
    }
    pub fn add_months(self, n: i32) -> Self {
        let i = self.year * 12 + i32::from(self.month) - 1 + n;
        let mut d = Self {
            year: i.div_euclid(12).clamp(1, 9999),
            month: (i.rem_euclid(12) + 1) as u8,
            day: self.day,
        };
        d.day = d.day.min(d.days());
        d
    }
    pub fn weekday(self) -> usize {
        (self.serial() + 4).rem_euclid(7) as usize
    }
    pub fn format(self, cx: &App) -> String {
        if tr(cx, "en", "zh") == "zh" {
            format!("{}年{}月{}日", self.year, self.month, self.day)
        } else {
            format!(
                "{} {}, {}",
                &MONTHS[(self.month - 1) as usize][..3],
                self.day,
                self.year
            )
        }
    }
}
const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DateSpan {
    pub start: CivilDate,
    pub end: CivilDate,
}
impl DateSpan {
    pub fn new(a: CivilDate, b: CivilDate) -> Self {
        Self {
            start: a.min(b),
            end: a.max(b),
        }
    }
    fn label(self, zh: bool) -> String {
        let (a, b) = (self.start, self.end);
        if zh {
            let start = format!("{}年{}月{}日", a.year, a.month, a.day);
            if a == b {
                return start;
            }
            let end = if a.year != b.year {
                format!("{}年{}月{}日", b.year, b.month, b.day)
            } else if a.month != b.month {
                format!("{}月{}日", b.month, b.day)
            } else {
                format!("{}日", b.day)
            };
            format!("{start} – {end}")
        } else {
            let (ma, mb) = (
                &MONTHS[(a.month - 1) as usize][..3],
                &MONTHS[(b.month - 1) as usize][..3],
            );
            if a == b {
                format!("{ma} {}, {}", a.day, a.year)
            } else if a.year != b.year {
                format!("{ma} {}, {} – {mb} {}, {}", a.day, a.year, b.day, b.year)
            } else if a.month != b.month {
                format!("{ma} {} – {mb} {}, {}", a.day, b.day, b.year)
            } else {
                format!("{ma} {} – {}, {}", a.day, b.day, b.year)
            }
        }
    }
    pub fn days(self) -> usize {
        (self.end.serial() - self.start.serial() + 1) as usize
    }
}
/// Emitted when a selected date is removed using the field's Clear action.
pub struct DateCleared;
pub trait DateLayout: 'static {
    const FIELD: bool;
    const RANGE: bool;
}
#[derive(Default)]
struct RangeLabelSlots {
    trigger: Rc<Cell<Bounds<Pixels>>>,
    summary: Rc<Cell<Bounds<Pixels>>>,
}
impl RangeLabelSlots {
    fn probe(slot: &Rc<Cell<Bounds<Pixels>>>) -> impl IntoElement + 'static {
        let slot = slot.clone();
        canvas(move |bounds, _, _| slot.set(bounds), |_, _, _, _| ())
            .absolute()
            .inset_0()
    }

    fn flight(&self, progress: f32, label: Div) -> Div {
        let from = self.trigger.get();
        let to = self.summary.get();
        let to = if to.size.width > px(0.) { to } else { from };
        let origin = from.origin + (to.origin - from.origin) * progress;
        let width = from.size.width + (to.size.width - from.size.width) * progress;
        div().absolute().size(px(0.)).child(
            anchored().position(origin).snap_to_window().child(
                div()
                    .w(width.max(px(1.)))
                    .h(px(20.))
                    .overflow_hidden()
                    .child(label),
            ),
        )
    }
}
pub struct DateControl<L: DateLayout> {
    label: SharedString,
    help: SharedString,
    today: CivilDate,
    min: Option<CivilDate>,
    max: Option<CivilDate>,
    view: CivilDate,
    active: CivilDate,
    value: Option<DateSpan>,
    anchor: Option<CivilDate>,
    draft: Option<DateSpan>,
    open: bool,
    reveal: FormReveal,
    month_offset: Slide,
    edges: [Slide; 2],
    cursor: [Slide; 2],
    previous_date: Option<CivilDate>,
    value_label: RollingValue,
    range_label: RangeLabelSlots,
    close_after: Option<super::family::PhaseClock>,
    trigger: FocusHandle,
    grid: FocusHandle,
    _layout: PhantomData<L>,
}
impl<L: DateLayout> EventEmitter<DateCleared> for DateControl<L> {}
impl<L: DateLayout> EventEmitter<DateSpan> for DateControl<L> {}
impl<L: DateLayout> DateControl<L> {
    pub fn new(label: impl Into<SharedString>, today: CivilDate, cx: &mut Context<Self>) -> Self {
        Self {
            label: label.into(),
            help: "".into(),
            today,
            min: None,
            max: None,
            view: today,
            active: today,
            value: None,
            anchor: None,
            draft: None,
            open: !L::FIELD,
            reveal: FormReveal::new(!L::FIELD),
            month_offset: Slide::snappy((today.year * 12 + i32::from(today.month)) as f32),
            edges: [Slide::snappy(0.); 2],
            cursor: [Slide::snappy(0.); 2],
            previous_date: None,
            value_label: Default::default(),
            range_label: Default::default(),
            close_after: None,
            trigger: cx.focus_handle().tab_stop(true),
            grid: cx.focus_handle().tab_stop(true),
            _layout: PhantomData,
        }
    }
    pub fn with_value(mut self, value: DateSpan) -> Self {
        self.view = value.end;
        self.active = value.end;
        self.value = Some(value);
        self.month_offset
            .reset((value.end.year * 12 + i32::from(value.end.month)) as f32);
        self
    }
    /// Inclusive civil dates that may receive focus or be committed.
    pub fn with_bounds(mut self, min: Option<CivilDate>, max: Option<CivilDate>) -> Self {
        (self.min, self.max) = match (min, max) {
            (Some(lo), Some(hi)) if lo > hi => (Some(hi), Some(lo)),
            bounds => bounds,
        };
        self.active = self.bounded(self.active);
        self.view = self.active;
        self
    }
    fn bounded(&self, date: CivilDate) -> CivilDate {
        self.min
            .map_or(date, |lo| lo.max(date))
            .min(self.max.unwrap_or(CivilDate {
                year: 9999,
                month: 12,
                day: 31,
            }))
    }
    fn visible_month(&self, d: CivilDate, wide: bool) -> CivilDate {
        let key = |d: CivilDate| (d.year, d.month);
        if wide && key(d) > key(self.view) {
            if key(d) == key(self.view.add_months(1)) {
                self.view
            } else {
                d.add_months(-1)
            }
        } else {
            d
        }
    }
    pub fn with_description(mut self, s: impl Into<SharedString>) -> Self {
        self.help = s.into();
        self
    }
    pub fn value(&self) -> Option<DateSpan> {
        self.value
    }
    fn show(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        self.open = true;
        self.anchor = None;
        self.draft = self.value;
        self.active = self.bounded(self.value.map_or(self.today, |r| r.end));
        self.view = if L::RANGE && w.viewport_size().width >= px(728.) {
            self.active.add_months(-1)
        } else {
            self.active
        };
        self.grid.focus(w, cx);
        self.close_after = None;
        self.month_offset
            .reset((self.view.year * 12 + i32::from(self.view.month)) as f32);
        cx.notify()
    }
    fn close(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        self.close_after = None;
        if L::FIELD {
            self.open = false;
            self.trigger.focus(w, cx)
        }
        self.anchor = None;
        // Keep the last drawn range through the exit. Opening resets it from the committed value.
        if !L::RANGE {
            self.draft = None;
        }
        cx.notify()
    }
    fn commit(&mut self, value: DateSpan, w: &mut Window, cx: &mut Context<Self>) {
        if self.value != Some(value) {
            self.previous_date = self.value.map(|d| d.start);
            self.value = Some(value);
            cx.emit(value)
        }
        if L::FIELD && !L::RANGE && !super::family::disabled_motion(cx) {
            let mut c = super::family::PhaseClock::default();
            c.restart(cx);
            self.close_after = Some(c);
            cx.notify();
        } else {
            self.close(w, cx)
        }
    }
    fn day(&mut self, d: CivilDate, w: &mut Window, cx: &mut Context<Self>) {
        if self.bounded(d) != d {
            return;
        }
        self.active = d;
        self.view = self.visible_month(d, L::RANGE && w.viewport_size().width >= px(728.));
        if L::RANGE {
            if let Some(first) = self.anchor.take() {
                self.draft = Some(DateSpan::new(first, d))
            } else {
                self.anchor = Some(d);
                self.draft = Some(DateSpan::new(d, d))
            }
            cx.notify()
        } else {
            self.commit(DateSpan::new(d, d), w, cx)
        }
    }
    fn move_to(&mut self, d: CivilDate, cx: &mut Context<Self>) {
        self.active = d;
        self.view = d;
        if let Some(a) = self.anchor {
            self.draft = Some(DateSpan::new(a, d))
        }
        cx.notify()
    }
    fn key(&mut self, e: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        let d = self.active;
        let next = match e.keystroke.key.as_str() {
            "left" => d.add_days(-1),
            "right" => d.add_days(1),
            "up" => d.add_days(-7),
            "down" => d.add_days(7),
            "home" => d.add_days(-(d.weekday() as i32)),
            "end" => d.add_days(6 - d.weekday() as i32),
            "pageup" => d.add_months(-1),
            "pagedown" => d.add_months(1),
            "enter" | "space" => {
                self.day(d, w, cx);
                cx.stop_propagation();
                return;
            }
            "escape" => {
                self.close(w, cx);
                cx.stop_propagation();
                return;
            }
            _ => return,
        };
        let next = self.bounded(next);
        let wanted = self.visible_month(next, L::RANGE && w.viewport_size().width >= px(728.));
        self.move_to(next, cx);
        self.view = wanted;
        w.prevent_default();
        cx.stop_propagation()
    }
    fn month_strip(
        &self,
        month: CivilDate,
        second: bool,
        offset: f32,
        width: f32,
        w: &Window,
        cx: &Context<Self>,
    ) -> Div {
        let p = Paint::get(cx);
        let zh = tr(cx, "en", "zh") == "zh";
        let title = |m: CivilDate| {
            if zh {
                format!("{}年{}月", m.year, m.month)
            } else {
                format!("{} {}", MONTHS[(m.month - 1) as usize], m.year)
            }
        };
        let old = month.add_months(if offset > 0. { -1 } else { 1 });
        let heading = div()
            .relative()
            .flex_1()
            .min_w_0()
            .text_size(px(if L::RANGE { 14. } else { 16. }))
            .font_weight(FontWeight::MEDIUM)
            .when(offset.abs() > 0.001, |d| {
                d.child(
                    div()
                        .absolute()
                        .left(px((offset - offset.signum()) * 12.))
                        .opacity(offset.abs())
                        .child(title(old)),
                )
            })
            .child(
                div()
                    .relative()
                    .left(px(offset * 12.))
                    .opacity(1. - offset.abs())
                    .child(title(month)),
            );
        let nav = |next: bool| {
            let index = |d: CivilDate| (d.year, d.month);
            let outer = if L::RANGE && w.viewport_size().width >= px(728.) {
                self.view.add_months(1)
            } else {
                self.view
            };
            let disabled = if next {
                self.max.is_some_and(|hi| index(outer) >= index(hi))
            } else {
                self.min.is_some_and(|lo| index(self.view) <= index(lo))
            };
            let name = if next {
                tr(cx, "Next month", "下个月")
            } else {
                tr(cx, "Previous month", "上个月")
            };
            face(
                if next { "next" } else { "prev" },
                name,
                32.,
                crate::foundation::surface_radius(cx).into(),
                Tone::Clear,
                cx,
            )
            .size(px(32.))
            .when(disabled, |d| {
                d.aria_description(tr(cx, "Unavailable", "不可用"))
            })
            .tab_stop(!disabled)
            .when(disabled, |d| d.opacity(0.36).cursor_default())
            .hover(move |v| v.bg(p.hover))
            .child(transformed(
                Glyph::Down,
                14.,
                p.secondary,
                if next {
                    -std::f32::consts::FRAC_PI_2
                } else {
                    std::f32::consts::FRAC_PI_2
                },
                1.,
                (0., 0.),
                1.,
            ))
            .on_click(cx.listener(move |s, _, _, cx| {
                if !disabled {
                    s.move_to(s.bounded(s.view.add_months(if next { 1 } else { -1 })), cx)
                }
            }))
        };
        let mut head = if L::RANGE {
            row()
                .relative()
                .w_full()
                .h(px(32.))
                .justify_center()
                .child(heading.flex_none())
                .when(!second, |d| {
                    d.child(div().absolute().left_0().child(nav(false)))
                })
                .when(second || w.viewport_size().width < px(728.), |d| {
                    d.child(div().absolute().right_0().child(nav(true)))
                })
        } else {
            let controls = row()
                .gap(px(2.))
                .child(
                    button("today", tr(cx, "Today", "今天"), cx)
                        .on_click(cx.listener(|s, _, w, cx| s.day(s.today, w, cx))),
                )
                .child(nav(false))
                .child(nav(true));
            if width < 260. {
                col()
                    .gap(px(4.))
                    .child(heading.flex_none())
                    .child(row().justify_end().child(controls))
            } else {
                row()
                    .h(px(34.))
                    .w_full()
                    .justify_between()
                    .child(heading)
                    .child(controls)
            }
        };
        head = head.debug_selector(move || format!("calendar-head-{}", usize::from(second)));
        let gap = if L::RANGE { 0. } else { 4. };
        let mut labels = row()
            .gap(px(gap))
            .h(px(if L::RANGE { 28. } else { 24. }))
            .w_full();
        for (i, text) in (if zh {
            ["日", "一", "二", "三", "四", "五", "六"]
        } else {
            ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"]
        })
        .into_iter()
        .enumerate()
        {
            labels = labels.child(
                div()
                    .debug_selector(move || format!("weekday-{}-{i}", usize::from(second)))
                    .w(px((width - 6. * gap) / 7.))
                    .flex_none()
                    .text_center()
                    .text_color(p.faint)
                    .text_size(px(12.))
                    .child(text),
            );
        }
        let travel = if L::RANGE { 48. } else { width };
        let mut panes = div().relative().w_full().overflow_hidden();
        if offset.abs() > 0.0001 {
            panes = panes.child(
                div()
                    .absolute()
                    .top_0()
                    .left(px(
                        (offset - offset.signum()) * if L::RANGE { 36. } else { travel }
                    ))
                    .w_full()
                    .opacity(if L::RANGE { offset.abs() } else { 1. })
                    .child(self.month_grid(old, second, false, width, w, cx)),
            );
        }
        panes = panes.child(
            div()
                .relative()
                .left(px(offset * travel))
                .w_full()
                .opacity(if L::RANGE { 1. - offset.abs() } else { 1. })
                .child(self.month_grid(month, second, true, width, w, cx)),
        );
        col()
            .w(px(width))
            .max_w_full()
            .gap_0()
            .child(head)
            .child(labels.when(!L::RANGE, |d| d.mt(px(14.)).mb(px(6.))))
            .child(panes)
    }
    fn month_grid(
        &self,
        month: CivilDate,
        second: bool,
        live: bool,
        width: f32,
        w: &Window,
        cx: &Context<Self>,
    ) -> Div {
        let p = Paint::get(cx);
        let gap = if L::RANGE { 0. } else { 4. };
        let cell = (width - gap * 6.) / 7.;
        let mut grid = col().gap(px(gap)).w_full();
        let first = CivilDate { day: 1, ..month };
        let start = first.add_days(-(first.weekday() as i32));
        let span = if self.open && L::RANGE {
            self.draft.or(self.value)
        } else {
            self.value
        };
        let edge = [self.edges[0].get(w, cx), self.edges[1].get(w, cx)];
        let week_base = start.serial() as f32;
        let cursor = [self.cursor[0].get(w, cx), self.cursor[1].get(w, cx)];
        for week in 0..6 {
            let mut days = row().relative().gap(px(gap)).w_full().h(px(cell));
            let week_start = week_base + week as f32 * 7.;
            if L::RANGE && span.is_some() && edge[1] >= week_start && edge[0] < week_start + 7. {
                let x = (edge[0].max(first.serial() as f32) - week_start).clamp(0., 7.) / 7.;
                let month_end = CivilDate {
                    day: month.days(),
                    ..month
                };
                let end =
                    (edge[1].min(month_end.serial() as f32) - week_start + 1.).clamp(0., 7.) / 7.;
                days = days.child(
                    div()
                        .absolute()
                        .left(relative(x))
                        .w(relative((end - x).max(0.)))
                        .top(px(4.))
                        .bottom(px(4.))
                        .rounded(crate::foundation::surface_radius(cx))
                        .bg(p.hover),
                );
            }
            if !L::RANGE && self.value.is_some() {
                let rel_y = cursor[1] - ((week_base + 4.) / 7.).floor() - week as f32;
                if rel_y.abs() < 1. {
                    days = days.child(
                        div()
                            .absolute()
                            .top(px(rel_y * (cell + gap)))
                            .left(px(cursor[0] * (cell + gap)))
                            .size(px(cell))
                            .rounded(crate::foundation::surface_radius(cx))
                            .bg(p.bright),
                    )
                }
            }
            for day in 0..7 {
                let d = start.add_days(week * 7 + day);
                if L::RANGE && month.month != d.month {
                    days = days.child(div().w(px(cell)).flex_none());
                    continue;
                }
                let enabled = self.bounded(d) == d;
                let on = span.is_some_and(|s| s.start == d || s.end == d);
                let dname = d.format(cx);
                let b = face(
                    (
                        if live { "day" } else { "leaving-day" },
                        d.serial() as usize,
                    ),
                    dname,
                    cell,
                    crate::foundation::surface_radius(cx).into(),
                    Tone::Clear,
                    cx,
                )
                .tab_stop(false)
                .when(!enabled, |b| {
                    b.aria_description(tr(cx, "Unavailable", "不可用"))
                })
                .when(!enabled, |b| b.opacity(0.38).cursor_default())
                .when(live, |el| {
                    el.debug_selector(move || {
                        format!("calendar-cell-{}-{week}-{day}", usize::from(second))
                    })
                })
                .relative()
                .role(Role::GridCell)
                .aria_selected(on)
                .rounded(crate::foundation::surface_radius(cx))
                .w(px(cell))
                .flex_none()
                .bg(if on && L::RANGE {
                    p.bright
                } else {
                    transparent_black()
                })
                .text_color(
                    if on
                        && (L::RANGE
                            || ((cursor[0] - d.weekday() as f32).abs() < 0.55
                                && (cursor[1] - ((d.serial() as f32 + 4.) / 7.).floor()).abs()
                                    < 0.55))
                    {
                        p.inverse
                    } else if month.month != d.month {
                        p.faint
                    } else {
                        p.ink
                    },
                )
                .when(self.active == d && !on && self.grid.is_focused(w), |b| {
                    b.border_1().border_color(p.faint)
                })
                .when(!on && enabled, |b| b.hover(move |s| s.bg(p.hover)))
                .font_weight(if on || d == self.today {
                    FontWeight::MEDIUM
                } else {
                    FontWeight::NORMAL
                })
                .child(d.day.to_string())
                .when(d == self.today, |b| {
                    b.child(
                        div()
                            .absolute()
                            .bottom(px(4.))
                            .size(px(4.))
                            .rounded_full()
                            .bg(if on { p.inverse } else { p.ink }),
                    )
                })
                .when(live, |b| {
                    b.on_hover(cx.listener(move |s, over: &bool, _, cx| {
                        if *over
                            && enabled
                            && let Some(a) = s.anchor
                        {
                            s.draft = Some(DateSpan::new(a, d));
                            cx.notify()
                        }
                    }))
                    .on_click(cx.listener(move |s, _, w, cx| s.day(d, w, cx)))
                });
                days = days.child(b)
            }
            grid = grid.child(days)
        }
        grid
    }
}
impl<L: DateLayout> Render for DateControl<L> {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        if let Some(c) = self.close_after
            && c.time(0.30, w, cx) >= 0.30
        {
            self.close(w, cx);
        }
        let progress = if L::RANGE {
            self.reveal.range_progress(self.open, w, cx)
        } else {
            self.reveal.sample(self.open, 0.26, 0.14, w, cx)
        };
        let target = (self.view.year * 12 + i32::from(self.view.month)) as f32;
        let from = self.month_offset.value(cx);
        if (target - from).abs() > 2. {
            self.month_offset.reset(target - (target - from).signum());
        }
        self.month_offset.duration(target, 0.42, cx);
        let offset = (target - self.month_offset.get(w, cx)).clamp(-1., 1.);
        let span = if L::RANGE && self.open {
            self.draft.or(self.value)
        } else {
            self.value
        };
        if let Some(v) = span {
            for (spring, d) in self.edges.iter_mut().zip([v.start, v.end]) {
                let val = d.serial() as f32;
                if (spring.value(cx) - val).abs() > 65. {
                    spring.reset(val);
                }
                spring.duration(val, 0.40, cx);
            }
            let end = v.end;
            let week = ((end.serial() as f32 + 4.) / 7.).floor();
            if (self.cursor[1].value(cx) - week).abs() > 6. {
                self.cursor[0].reset(end.weekday() as f32);
                self.cursor[1].reset(week);
            }
            self.cursor[0].duration(end.weekday() as f32, 0.42, cx);
            self.cursor[1].duration(week, 0.42, cx);
        }
        let compact = w.viewport_size().width < px(728.);
        let zh = tr(cx, "en", "zh") == "zh";
        let available = (f32::from(w.viewport_size().width) - 16.).max(1.);
        let preferred =
            |title: &str| super::family::measure(&title.to_owned().into(), 14., w).ceil() + 82.;
        let format_range = |r: DateSpan| {
            let label = r.label(zh);
            if preferred(&label) <= available {
                label
            } else if r.start.year == r.end.year {
                format!(
                    "{}/{}/{} – {}/{}",
                    r.start.year, r.start.month, r.start.day, r.end.month, r.end.day
                )
            } else {
                format!(
                    "{}/{}/{} – {}/{}/{}",
                    r.start.year, r.start.month, r.start.day, r.end.year, r.end.month, r.end.day
                )
            }
        };
        let title = match self.value {
            Some(r) if L::RANGE => format_range(r),
            Some(r) => r.start.format(cx),
            None => tr(cx, "Select date", "选择日期").into(),
        };
        let flying = L::RANGE
            && if self.open {
                progress < 1.
            } else {
                progress > 0.
            };
        let displayed = if L::RANGE && self.open {
            self.draft
                .map_or_else(|| tr(cx, "No dates", "未选日期").into(), format_range)
        } else {
            title.clone()
        };
        let direction = if self
            .previous_date
            .is_some_and(|d| self.value.is_some_and(|r| r.start < d))
        {
            -1.
        } else {
            1.
        };
        // The same shaped label occupies exactly one of trigger, moving layer, or footer.
        let mut value_label = L::FIELD.then(|| {
            self.value_label
                .label(displayed.clone(), direction, w, cx)
                .debug_selector(|| "calendar-visible-value".into())
                .when(L::RANGE, |el| {
                    el.font_weight(FontWeight::MEDIUM)
                        .text_color(p.ink)
                        .text_size(px(14.))
                        .line_height(px(20.))
                        .whitespace_nowrap()
                })
        });
        let closed = preferred(&title).max(220.).min(available);
        let open_width = if compact {
            352_f32.min(available)
        } else {
            712.
        };
        let width = if L::RANGE {
            closed
        } else if L::FIELD {
            340.
        } else {
            328.
        };
        let mut root = section(width, cx)
            .relative()
            .id("date")
            .on_mouse_down_out(cx.listener(|s, e: &MouseDownEvent, w, cx| {
                if L::FIELD && s.open && !s.reveal.contains(e.position) {
                    s.close(w, cx)
                }
            }));
        if L::FIELD {
            root = root
                .when(!self.label.is_empty(), |r| r.child(self.label.clone()))
                .child(
                    field("open", title.clone(), cx)
                        .relative()
                        .debug_selector(|| "date-trigger".into())
                        .gap(px(9.))
                        .child(self.reveal.anchor())
                        .track_focus(&self.trigger)
                        .when(L::RANGE, |b| {
                            b.h(px(44.))
                                .opacity((1. - progress * 1.6).clamp(0., 1.))
                                .overflow_hidden()
                                .tab_stop(!self.open)
                        })
                        .aria_expanded(self.open)
                        .child(
                            div()
                                .debug_selector(|| "date-leading".into())
                                .flex_none()
                                .size(px(16.))
                                .child(calendar_mark(p.faint)),
                        )
                        .child(
                            div()
                                .debug_selector(|| "date-value".into())
                                .flex_1()
                                .min_w_0()
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .h(px(20.))
                                .relative()
                                .when(L::RANGE, |el| {
                                    el.child(RangeLabelSlots::probe(&self.range_label.trigger))
                                })
                                .when(!L::RANGE || (!self.open && !flying), |el| {
                                    el.children(value_label.take())
                                }),
                        )
                        .child(
                            div()
                                .debug_selector(|| "date-trailing".into())
                                .flex_none()
                                .size(px(16.))
                                .child(super::glyph::transformed(
                                    Glyph::Down,
                                    16.,
                                    p.faint,
                                    std::f32::consts::PI * progress,
                                    1.,
                                    (0., 0.),
                                    1.,
                                )),
                        )
                        .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                            if !s.open
                                && matches!(e.keystroke.key.as_str(), "down" | "enter" | "space")
                            {
                                s.show(w, cx);
                                w.prevent_default();
                                cx.stop_propagation()
                            }
                        }))
                        .on_click(cx.listener(|s, _, w, cx| {
                            if s.open {
                                s.close(w, cx)
                            } else {
                                s.show(w, cx)
                            }
                        })),
                )
        }
        if self.open || progress > 0. {
            let pop_width = if w.viewport_size().width <= px(360.) {
                f32::from(self.reveal.anchor_bounds().size.width)
                    .max(1.)
                    .min(available)
            } else {
                320.
            };
            let pad = if L::RANGE {
                if compact { 8. } else { 10. }
            } else if L::FIELD && w.viewport_size().width > px(360.) {
                16.
            } else {
                12.
            };
            let grid_width = if L::RANGE {
                if compact {
                    (((open_width - 24.) / 7.).floor().clamp(1., 42.)) * 7.
                } else {
                    252.
                }
            } else if L::FIELD {
                (pop_width - pad * 2. - 2.).max(1.)
            } else {
                width.min(available) - pad * 2. - 2.
            };
            let mut body = panel(cx)
                .when(L::RANGE, |p| {
                    p.border_0().bg(transparent_black()).relative().occlude()
                })
                .when(!L::RANGE, |p| p.shadow_lg())
                .id("grid")
                .track_focus(&self.grid)
                .on_key_down(cx.listener(Self::key))
                .p(px(pad));
            let mut layout = if compact { col() } else { row().items_start() };
            if L::RANGE {
                let mut presets = if compact {
                    row().gap(px(4.)).id("presets").overflow_x_scroll()
                } else {
                    col().gap(px(2.)).id("presets")
                };
                let today = self.today;
                let month = CivilDate { day: 1, ..today };
                let previous = month.add_months(-1);
                let q = (today.month - 1) / 3 * 3 + 1;
                for (i, (en, zh, start, end)) in [
                    ("Today", "今天", today, today),
                    ("Yesterday", "昨天", today.add_days(-1), today.add_days(-1)),
                    ("Last 7 days", "近 7 天", today.add_days(-6), today),
                    ("Last 30 days", "近 30 天", today.add_days(-29), today),
                    ("This month", "本月", month, today),
                    ("Last month", "上月", previous, month.add_days(-1)),
                    (
                        "This quarter",
                        "本季度",
                        CivilDate { month: q, ..month },
                        today,
                    ),
                    (
                        "Year to date",
                        "本年",
                        CivilDate { month: 1, ..month },
                        today,
                    ),
                ]
                .into_iter()
                .enumerate()
                {
                    let chosen = DateSpan::new(self.bounded(start), self.bounded(end));
                    presets = presets.child(
                        button(("preset", i), tr(cx, en, zh), cx)
                            .when(self.draft == Some(chosen), |e| e.bg(p.hover))
                            .on_click(cx.listener(move |s, _, _, cx| {
                                let end = chosen.end;
                                s.anchor = None;
                                s.active = end;
                                s.view = if compact { end } else { end.add_months(-1) };
                                s.draft = Some(chosen);
                                cx.notify()
                            })),
                    );
                }
                layout = layout.child(presets);
            }
            let mut months = row()
                .justify_center()
                .gap(px(24.))
                .items_start()
                .min_w_0()
                .child(self.month_strip(self.view, false, offset, grid_width, w, cx));
            if L::RANGE && !compact {
                months = months.child(self.month_strip(
                    self.view.add_months(1),
                    true,
                    offset,
                    grid_width,
                    w,
                    cx,
                ))
            }
            body = body.child(layout.child(months));
            if L::RANGE {
                let cancel = tr(cx, "Cancel", "取消");
                let apply = tr(cx, "Apply", "应用");
                let text_width =
                    |text: &str| super::family::measure(&text.to_owned().into(), 14., w).ceil();
                let actions_width = text_width(cancel) + text_width(apply) + 2. * 18. + 8.;
                let summary_width = self.draft.map_or(0., |v| text_width(&format_range(v)));
                let stack = summary_width + 8. + actions_width > open_width - 2. * pad - 2.;
                let summary = col()
                    .gap_0()
                    .when(!stack, |el| el.flex_1())
                    .child(
                        div()
                            .relative()
                            .h(px(20.))
                            .w_full()
                            .overflow_hidden()
                            .debug_selector(|| "range-summary-slot".into())
                            .child(RangeLabelSlots::probe(&self.range_label.summary))
                            .when(!flying && self.open, |el| el.children(value_label.take())),
                    )
                    .when_some(self.draft, |s, v| {
                        s.child(hint(format!("{} {}", v.days(), tr(cx, "days", "天")), cx))
                    });
                let actions = row()
                    .justify_end()
                    .flex_none()
                    .child(
                        button("cancel", cancel, cx)
                            .on_click(cx.listener(|s, _, w, cx| s.close(w, cx))),
                    )
                    .child(
                        button("apply", apply, cx)
                            .bg(p.bright)
                            .text_color(p.inverse)
                            .on_click(cx.listener(|s, _, w, cx| {
                                if let Some(r) = s.draft {
                                    s.commit(r, w, cx)
                                }
                            })),
                    );
                let footer = if stack {
                    col().gap(px(6.))
                } else {
                    row().justify_between()
                };
                body = body
                    .child(footer.child(summary).child(actions))
                    .when(!self.open, |panel| {
                        panel.child(
                            div()
                                .id("range-exit")
                                .absolute()
                                .inset_0()
                                .occlude()
                                .on_click(cx.listener(|s, e: &ClickEvent, w, cx| {
                                    if s.reveal.anchor_bounds().contains(&e.position()) {
                                        s.show(w, cx);
                                        cx.stop_propagation();
                                    }
                                })),
                        )
                    });
            }
            if L::FIELD && !L::RANGE {
                let status = self.value.map_or_else(
                    || tr(cx, "Choose a day", "选择一天").to_owned(),
                    |v| format!("{} {}", tr(cx, "Selected", "已选择"), v.start.format(cx)),
                );
                body = body.child(
                    row()
                        .justify_between()
                        .mt(px(6.))
                        .pt(px(10.))
                        .border_t_1()
                        .border_color(p.line)
                        .child(
                            button("clear-date", tr(cx, "Clear", "清除"), cx)
                                .h(px(25.))
                                .text_size(px(12.))
                                .on_click(cx.listener(|s, _, w, cx| {
                                    if s.value.take().is_some() {
                                        cx.emit(DateCleared);
                                    }
                                    s.close(w, cx)
                                })),
                        )
                        .child(hint(status, cx)),
                );
            }
            root = if L::RANGE {
                root.child(
                    self.reveal.over_trigger(
                        (
                            body,
                            value_label
                                .take()
                                .map(|label| self.range_label.flight(progress, label)),
                        ),
                        progress,
                        self.open,
                        open_width,
                        w,
                        cx,
                    ),
                )
            } else if L::FIELD {
                root.child(
                    self.reveal
                        .popup(body, progress, self.open, (pop_width, 407., 8.), w),
                )
            } else {
                root.child(self.reveal.wrap(
                    body,
                    progress,
                    self.open,
                    if L::RANGE { 375. } else { 300. },
                    if L::RANGE { 0. } else { 8. },
                ))
            }
        }
        root.when(!self.help.is_empty(), |r| {
            r.child(hint(self.help.clone(), cx).opacity(1. - progress))
        })
        .when(L::RANGE, |r| {
            r.when_some(self.value, |r, v| {
                r.child(
                    hint(
                        format!("{} {}", v.days(), tr(cx, "days selected", "天已选")),
                        cx,
                    )
                    .opacity(1. - progress),
                )
            })
        })
    }
}
#[cfg(test)]
mod tests {
    use super::{CivilDate, DateSpan};
    #[test]
    fn civil_boundaries() {
        let leap = CivilDate::new(2024, 2, 29).unwrap();
        assert_eq!(leap.add_months(12), CivilDate::new(2025, 2, 28).unwrap());
        assert!(CivilDate::new(2100, 2, 29).is_none());
        assert_eq!(CivilDate::new(1970, 1, 1).unwrap().weekday(), 4);
        assert_eq!(DateSpan::new(leap, leap.add_days(-6)).days(), 7)
    }
}
