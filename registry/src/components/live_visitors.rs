//! Realtime visitor counts, arrivals, page rankings and conversion activity.
use crate::{
    foundation::surface_radius,
    primitives::{
        control::UiLanguage,
        family::{Paint, disabled_motion, measure},
        glyph::{Glyph, icon},
        phone_flags,
    },
};
use gpui::{prelude::*, *};
use std::{
    cell::{Cell, RefCell},
    collections::HashSet,
    rc::Rc,
    sync::Arc,
    time::Duration,
};
use web_time::{Instant, SystemTime};

const WIDTH: f32 = 640.;
const ROW: f32 = 32.;
const STEP: f32 = ROW + 4.;
const DIGIT_MS: u64 = 280;
const ROW_MS: u64 = 280;
const IN_MS: u64 = 160;
const OUT_MS: u64 = 100;
const PING_MS: u64 = 900;
const EXPIRE_MS: u64 = 400;
const CITY_MS: u64 = 2000;
const PULSE_MS: u64 = 1600;
const DEMO_MS: u64 = 600;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum LiveConnection {
    #[default]
    Live,
    Connecting,
    Reconnecting,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LivePage {
    pub path: SharedString,
    pub visitors: u64,
}

impl LivePage {
    pub fn new(path: impl Into<SharedString>, visitors: u64) -> Self {
        Self {
            path: path.into(),
            visitors,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum LivePingKind {
    #[default]
    Arrival,
    Event,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LiveLocation {
    pub lat: f32,
    pub lon: f32,
}

impl LiveLocation {
    pub const fn new(lat: f32, lon: f32) -> Self {
        Self { lat, lon }
    }

    fn point(self) -> Option<Point<f32>> {
        (self.lat.is_finite()
            && self.lon.is_finite()
            && self.lat.abs() <= 90.
            && self.lon.abs() <= 180.)
            .then(|| {
                point(
                    (self.lon + 180.) / 360.,
                    ((84. - self.lat) / 144.).clamp(0., 1.),
                )
            })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LiveVisitorPing {
    pub id: SharedString,
    pub location: LiveLocation,
    pub kind: LivePingKind,
}

impl LiveVisitorPing {
    pub fn new(id: impl Into<SharedString>, location: LiveLocation) -> Self {
        Self {
            id: id.into(),
            location,
            kind: LivePingKind::Arrival,
        }
    }

    pub fn with_kind(mut self, kind: LivePingKind) -> Self {
        self.kind = kind;
        self
    }
}

#[derive(Clone)]
pub struct LiveVisitorEvent {
    pub id: SharedString,
    pub label: SharedString,
    pub city: SharedString,
    /// ISO 3166-1 alpha-2 code.
    pub country: SharedString,
    pub at: SystemTime,
    pub location: Option<LiveLocation>,
    pub flag: Option<Arc<Image>>,
}

impl LiveVisitorEvent {
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        city: impl Into<SharedString>,
        country: impl Into<SharedString>,
        at: SystemTime,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            city: city.into(),
            country: country.into(),
            at,
            location: None,
            flag: None,
        }
    }

    pub fn with_location(mut self, location: LiveLocation) -> Self {
        self.location = Some(location);
        self
    }

    pub fn with_flag(mut self, flag: Arc<Image>) -> Self {
        self.flag = Some(flag);
        self
    }
}

#[derive(Clone, Default)]
pub struct LiveVisitorsSnapshot {
    pub count: u64,
    pub pages: Vec<LivePage>,
    pub visitors: Vec<LiveVisitorPing>,
    pub events: Vec<LiveVisitorEvent>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LiveVisitorsPaused(pub bool);

type CountLabel = Box<dyn Fn(u64) -> SharedString>;
type Visibility = Box<dyn Fn(&App) -> bool>;

#[derive(Clone, Copy)]
struct Tween {
    from: f32,
    to: f32,
    start: Instant,
    duration: u64,
}

impl Tween {
    fn at(value: f32, now: Instant) -> Self {
        Self {
            from: value,
            to: value,
            start: now,
            duration: 0,
        }
    }

    fn active(self, now: Instant) -> bool {
        self.from != self.to && now.saturating_duration_since(self.start) < ms(self.duration)
    }

    fn sample(self, now: Instant, animate: bool) -> f32 {
        if !animate || !self.active(now) {
            return self.to;
        }
        let t = now.saturating_duration_since(self.start).as_secs_f32()
            / ms(self.duration).as_secs_f32();
        self.to + (self.from - self.to) * (1. - t).powi(3)
    }

    fn set(&mut self, value: f32, now: Instant, duration: u64) {
        if self.to != value {
            *self = Self {
                from: self.sample(now, true),
                to: value,
                start: now,
                duration,
            };
        }
    }
}

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

/// A column belongs to a decimal place, so a carry leaves unchanged digits in place.
#[derive(Clone)]
struct Roll {
    count: u64,
    places: Vec<(Tween, Tween)>,
}

impl Roll {
    fn new(count: u64, now: Instant) -> Self {
        Self {
            count,
            places: count
                .to_string()
                .bytes()
                .rev()
                .map(|digit| (Tween::at(f32::from(digit - b'0'), now), Tween::at(1., now)))
                .collect(),
        }
    }

    fn set(&mut self, count: u64, now: Instant) {
        if count == self.count {
            return;
        }
        let down = count < self.count;
        self.count = count;
        let digits = count.to_string();
        self.places
            .resize_with(self.places.len().max(digits.len()), || {
                (Tween::at(0., now), Tween::at(0., now))
            });
        for (place, (strip, width)) in self.places.iter_mut().enumerate() {
            width.set(f32::from(place < digits.len()), now, DIGIT_MS);
            if let Some(digit) = digits.as_bytes().iter().rev().nth(place) {
                // Keep the strip coordinates bounded during long-running streams.
                let cycle = (strip.to / 10.).floor() * 10.;
                strip.from -= cycle;
                strip.to -= cycle;
                let delta = i32::from(digit - b'0') - strip.to as i32;
                let travel = if down {
                    -(-delta).rem_euclid(10)
                } else {
                    delta.rem_euclid(10)
                };
                strip.set(strip.to + travel as f32, now, DIGIT_MS);
            }
        }
    }

    fn active(&self, now: Instant) -> bool {
        self.places
            .iter()
            .any(|(roll, width)| roll.active(now) || width.active(now))
    }

    fn render(&self, em: f32, height: f32, ink: Hsla, frame: Frame) -> impl IntoElement {
        let column = em * 0.628;
        let mut width = 0.;
        let mut strips = Vec::with_capacity(self.places.len());
        for (i, (strip, visibility)) in self.places.iter().enumerate().rev() {
            let visible = visibility.sample(frame.now, frame.animate);
            if visible <= 0. {
                continue;
            }
            let value = strip.sample(frame.now, frame.animate);
            strips.push((width, column * visible, Some(value)));
            width += column * visible;
            if i > 0 && i.is_multiple_of(3) {
                strips.push((width, em * 0.24 * visible, None));
                width += em * 0.24 * visible;
            }
        }
        canvas(
            |_, _, _| (),
            move |bounds, _, w, cx| {
                for (left, clip_width, value) in strips {
                    let origin = bounds.origin + point(px(left), px(0.));
                    let mask = Bounds::new(origin, size(px(clip_width), bounds.size.height));
                    w.with_content_mask(Some(ContentMask { bounds: mask }), |w| {
                        let mut paint = |text: SharedString, y: f32| {
                            let mut run = w.text_style().to_run(text.len());
                            run.color = ink;
                            if em == 32. {
                                run.font.weight = FontWeight::MEDIUM;
                            }
                            let line = w.text_system().shape_line(text, px(em), &[run], None);
                            let _ = line.paint(
                                origin + point(px(0.), px(y)),
                                px(height),
                                TextAlign::Center,
                                Some(px(if value.is_some() { column } else { clip_width })),
                                w,
                                cx,
                            );
                        };
                        if let Some(value) = value {
                            let digit = value.floor() as i32;
                            let offset = (value - value.floor()) * height;
                            paint(digit.rem_euclid(10).to_string().into(), -offset);
                            if offset > 0. {
                                paint(
                                    (digit + 1).rem_euclid(10).to_string().into(),
                                    height - offset,
                                );
                            }
                        } else {
                            paint(",".into(), 0.);
                        }
                    });
                }
            },
        )
        .w(px(width))
        .h(px(height))
        .flex_none()
    }
}

#[derive(Clone)]
struct PageVisual {
    visitors: Roll,
    bar: Tween,
}

struct Row<T> {
    id: SharedString,
    content: T,
    offset: Tween,
    opacity: Tween,
    leaving: bool,
}

fn sync_rows<T, I>(
    rows: &mut Vec<Row<T>>,
    values: impl IntoIterator<Item = (SharedString, I)>,
    now: Instant,
    snap: bool,
    mut content: impl FnMut(Option<T>, I) -> T,
) {
    let mut previous = std::mem::take(rows);
    for (index, (id, item)) in values.into_iter().enumerate() {
        let target = index as f32 * STEP;
        let old = previous
            .iter()
            .position(|row| row.id == id)
            .map(|i| previous.remove(i));
        let (old, mut offset, mut opacity) = match old {
            Some(row) => (Some(row.content), row.offset, row.opacity),
            None => (
                None,
                Tween::at(target + STEP * 0.55, now),
                Tween::at(0., now),
            ),
        };
        if snap {
            offset = Tween::at(target, now);
            opacity = Tween::at(1., now);
        } else {
            offset.set(target, now, ROW_MS);
            opacity.set(1., now, IN_MS);
        }
        rows.push(Row {
            id,
            content: content(old, item),
            offset,
            opacity,
            leaving: false,
        });
    }
    if !snap {
        let exit = rows.len() as f32 * STEP;
        for mut row in previous {
            row.offset.set(exit, now, ROW_MS);
            row.opacity.set(0., now, IN_MS);
            row.leaving = true;
            if row.opacity.active(now) {
                rows.push(row);
            }
        }
    }
}

struct Ping {
    id: SharedString,
    center: Point<f32>,
    kind: LivePingKind,
    since: Instant,
    removed: Option<Instant>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Status {
    Live,
    Paused,
    Connecting,
    Reconnecting,
}

impl Status {
    fn text(self, lang: UiLanguage) -> &'static str {
        match self {
            Self::Live => lang.text("Live", "实时"),
            Self::Paused => lang.text("Paused", "已暂停"),
            Self::Connecting => lang.text("Connecting", "连接中"),
            Self::Reconnecting => lang.text("Reconnecting", "正在重连"),
        }
    }

    fn color(self, paint: Paint) -> Hsla {
        match self {
            Self::Live => rgb(0x5ac7a1).into(),
            Self::Reconnecting => rgb(0xd5992d).into(),
            Self::Connecting | Self::Paused => paint.faint,
        }
    }
}

type MapCache = Rc<RefCell<Option<(Bounds<Pixels>, Path<Pixels>)>>>;

#[derive(Clone, Copy, PartialEq)]
struct LayoutSample {
    width: Pixels,
    visible: bool,
}

/// Omitting a host snapshot starts the local, deterministic sample stream.
pub struct LiveVisitors {
    id: ElementId,
    title: Option<SharedString>,
    label: Option<CountLabel>,
    style: Option<Box<dyn Fn(Div) -> Div>>,
    visibility: Option<Visibility>,
    max_pages: usize,
    max_events: usize,
    hide_pause: bool,
    controlled: bool,
    paused: bool,
    connection: LiveConnection,
    last_status: Status,
    status_since: Option<Instant>,
    displayed: LiveVisitorsSnapshot,
    pending: Option<LiveVisitorsSnapshot>,
    sample: Option<LiveVisitorsSimulation>,
    displaying_sample: bool,
    count: Roll,
    pages: Vec<Row<PageVisual>>,
    events: Vec<Row<LiveVisitorEvent>>,
    pings: Vec<Ping>,
    city: Option<(LiveVisitorEvent, Instant)>,
    focus: FocusHandle,
    layout: Rc<Cell<LayoutSample>>,
    map: MapCache,
    clock: Option<Task<()>>,
    last_wall: SystemTime,
    started: Instant,
}

impl EventEmitter<LiveVisitorsPaused> for LiveVisitors {}

impl LiveVisitors {
    pub fn new(cx: &App) -> Self {
        let now = cx.background_executor().now();
        let wall = SystemTime::now();
        let sample = LiveVisitorsSimulation::new(wall);
        let initial = sample.snapshot(UiLanguage::current(cx));
        let mut this = Self {
            id: "live-visitors".into(),
            title: None,
            label: None,
            style: None,
            visibility: None,
            max_pages: 4,
            max_events: 4,
            hide_pause: false,
            controlled: false,
            paused: false,
            connection: LiveConnection::Live,
            last_status: Status::Live,
            status_since: None,
            displayed: Default::default(),
            pending: None,
            sample: Some(sample),
            displaying_sample: true,
            count: Roll::new(0, now),
            pages: Vec::new(),
            events: Vec::new(),
            pings: Vec::new(),
            city: None,
            focus: cx.focus_handle().tab_stop(true),
            layout: Rc::new(Cell::new(LayoutSample {
                width: px(0.),
                visible: true,
            })),
            map: Default::default(),
            clock: None,
            last_wall: wall,
            started: now,
        };
        this.apply(initial, now, true);
        this
    }

    pub fn with_id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = id.into();
        self
    }

    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn with_label(mut self, label: impl Fn(u64) -> SharedString + 'static) -> Self {
        self.label = Some(Box::new(label));
        self
    }

    pub fn with_style(mut self, style: impl Fn(Div) -> Div + 'static) -> Self {
        self.style = Some(Box::new(style));
        self
    }

    /// Hosts with an outer viewport can suspend animations and sampling offscreen.
    pub fn with_visibility(mut self, visible: impl Fn(&App) -> bool + 'static) -> Self {
        self.visibility = Some(Box::new(visible));
        self
    }

    pub fn with_limits(mut self, max_pages: usize, max_events: usize) -> Self {
        self.max_pages = max_pages;
        self.max_events = max_events;
        self.apply(self.displayed.clone(), self.started, true);
        self
    }

    pub fn with_hide_pause(mut self, hide: bool) -> Self {
        self.hide_pause = hide;
        self
    }

    pub fn with_snapshot(mut self, snapshot: LiveVisitorsSnapshot) -> Self {
        self.sample = None;
        self.displaying_sample = false;
        self.city = None;
        self.displayed.events.clear();
        self.apply(snapshot, self.started, true);
        self
    }

    pub fn with_connection(mut self, connection: LiveConnection) -> Self {
        self.connection = connection;
        self
    }

    pub fn with_default_paused(mut self, paused: bool) -> Self {
        if !self.controlled {
            self.paused = paused;
        }
        self
    }

    /// The pause button emits a request. Confirm it with `set_paused`.
    pub fn with_paused(mut self, paused: bool) -> Self {
        self.controlled = true;
        self.paused = paused;
        self
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    pub fn snapshot(&self) -> &LiveVisitorsSnapshot {
        &self.displayed
    }

    pub fn set_snapshot(&mut self, snapshot: LiveVisitorsSnapshot, cx: &mut Context<Self>) {
        self.sample = None;
        self.pending = Some(snapshot);
        self.flush(cx);
        cx.notify();
    }

    pub fn set_connection(&mut self, connection: LiveConnection, cx: &mut Context<Self>) {
        if connection != self.connection {
            let old = self.status();
            self.connection = connection;
            self.transition(old, cx);
            if connection != LiveConnection::Live {
                self.clock.take();
            } else if self.sample.is_none() && !self.paused {
                self.last_wall = SystemTime::now();
            }
            self.flush(cx);
            cx.notify();
        }
    }

    pub fn set_paused(&mut self, paused: bool, cx: &mut Context<Self>) {
        if paused == self.paused {
            return;
        }
        let old = self.status();
        self.paused = paused;
        self.transition(old, cx);
        if paused {
            self.clock.take();
        } else {
            if self.sample.is_none() {
                self.last_wall = SystemTime::now();
            }
            self.flush(cx);
        }
        cx.notify();
    }

    fn toggle(&mut self, cx: &mut Context<Self>) {
        let paused = !self.paused;
        if !self.controlled {
            self.set_paused(paused, cx);
        }
        cx.emit(LiveVisitorsPaused(paused));
    }

    fn status(&self) -> Status {
        if self.paused {
            Status::Paused
        } else {
            match self.connection {
                LiveConnection::Live => Status::Live,
                LiveConnection::Connecting => Status::Connecting,
                LiveConnection::Reconnecting => Status::Reconnecting,
            }
        }
    }

    fn transition(&mut self, old: Status, cx: &App) {
        if old != self.status() {
            self.last_status = old;
            self.status_since = Some(cx.background_executor().now());
        }
    }

    fn visible(&self, cx: &App) -> bool {
        self.layout.get().visible && self.visibility.as_ref().is_none_or(|visible| visible(cx))
    }

    fn flush(&mut self, cx: &App) {
        if !self.paused
            && self.connection == LiveConnection::Live
            && let Some(snapshot) = self.pending.take()
        {
            let switching = std::mem::take(&mut self.displaying_sample);
            if switching {
                self.city = None;
                self.displayed.events.clear();
                self.last_wall = SystemTime::now();
            }
            self.apply(
                snapshot,
                cx.background_executor().now(),
                switching || disabled_motion(cx),
            );
        }
    }

    fn apply(&mut self, mut snapshot: LiveVisitorsSnapshot, now: Instant, snap: bool) {
        snapshot
            .pages
            .sort_by_key(|page| std::cmp::Reverse(page.visitors));
        snapshot
            .events
            .sort_by_key(|event| std::cmp::Reverse(event.at));
        let mut seen = HashSet::new();
        snapshot.pages.retain(|page| seen.insert(page.path.clone()));
        seen.clear();
        snapshot
            .events
            .retain(|event| seen.insert(event.id.clone()));
        seen.clear();
        snapshot
            .visitors
            .retain(|ping| ping.location.point().is_some() && seen.insert(ping.id.clone()));
        if snap {
            self.count = Roll::new(snapshot.count, now);
        } else {
            self.count.set(snapshot.count, now);
        }
        let top = snapshot
            .pages
            .first()
            .map_or(1, |page| page.visitors.max(1)) as f32;
        sync_rows(
            &mut self.pages,
            snapshot
                .pages
                .iter()
                .take(self.max_pages)
                .map(|p| (p.path.clone(), p.visitors)),
            now,
            snap,
            |previous, count| {
                let ratio = count as f32 / top;
                if let Some(mut page) = previous.filter(|_| !snap) {
                    page.visitors.set(count, now);
                    page.bar.set(ratio, now, ROW_MS);
                    page
                } else {
                    PageVisual {
                        visitors: Roll::new(count, now),
                        bar: Tween::at(ratio, now),
                    }
                }
            },
        );
        sync_rows(
            &mut self.events,
            snapshot
                .events
                .iter()
                .take(self.max_events)
                .cloned()
                .map(|e| (e.id.clone(), e)),
            now,
            snap,
            |_, event| event,
        );
        if snap {
            self.pings.clear();
        } else {
            self.pings.retain(|ping| {
                ping.removed
                    .is_none_or(|then| now.saturating_duration_since(then) < ms(EXPIRE_MS))
            });
        }
        for ping in &mut self.pings {
            if !seen.contains(&ping.id) {
                ping.removed.get_or_insert(now);
            }
        }
        for ping in &snapshot.visitors {
            let Some(center) = ping.location.point() else {
                continue;
            };
            if let Some(old) = self.pings.iter_mut().find(|old| old.id == ping.id) {
                old.center = center;
                old.kind = ping.kind;
                old.removed = None;
            } else {
                self.pings.push(Ping {
                    id: ping.id.clone(),
                    center,
                    kind: ping.kind,
                    since: now,
                    removed: None,
                });
            }
        }
        if let Some(event) = snapshot.events.first()
            && self
                .displayed
                .events
                .first()
                .is_none_or(|old| old.id != event.id)
        {
            self.city = event
                .location
                .and_then(LiveLocation::point)
                .map(|_| (event.clone(), now));
        }
        if snapshot.events.is_empty() {
            self.city = None;
        }
        self.displayed = snapshot;
    }

    fn start_clock(&mut self, cx: &mut Context<Self>) {
        if !self.visible(cx) {
            self.clock.take();
            return;
        }
        if self.paused || self.connection != LiveConnection::Live || self.clock.is_some() {
            return;
        }
        self.clock = Some(cx.spawn(async |this, cx| {
            loop {
                let interval =
                    this.read_with(
                        cx,
                        |card, _| if card.sample.is_some() { DEMO_MS } else { 1000 },
                    );
                let Ok(interval) = interval else { break };
                cx.background_executor().timer(ms(interval)).await;
                if this
                    .update(cx, |card, cx| {
                        if card.paused || !card.visible(cx) {
                            return;
                        }
                        if card.connection == LiveConnection::Live {
                            if let Some(sample) = card.sample.as_mut() {
                                sample.advance();
                                let snapshot = sample.snapshot(UiLanguage::current(cx));
                                card.last_wall = sample.now();
                                card.apply(
                                    snapshot,
                                    cx.background_executor().now(),
                                    disabled_motion(cx),
                                );
                            } else {
                                card.last_wall = SystemTime::now();
                            }
                        }
                        cx.notify();
                    })
                    .is_err()
                {
                    break;
                }
            }
        }));
    }
}

#[derive(Clone, Copy)]
struct Frame {
    now: Instant,
    wall: SystemTime,
    animate: bool,
    lang: UiLanguage,
    paint: Paint,
    radius: Pixels,
}

fn row() -> Div {
    div().flex().items_center()
}

struct PauseHint(&'static str);

impl Render for PauseHint {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        div()
            .rounded(surface_radius(cx))
            .border_1()
            .border_color(p.line)
            .bg(p.panel)
            .text_color(p.ink)
            .text_size(px(12.))
            .px(px(8.))
            .py(px(5.))
            .child(self.0)
    }
}

fn flag(event: &LiveVisitorEvent, paint: Paint) -> impl IntoElement {
    let iso = event.country.to_ascii_uppercase();
    let image = event.flag.clone().or_else(|| match iso.as_str() {
        "US" | "GB" | "DE" | "FR" | "NL" | "SE" | "CH" | "JP" | "IN" | "BR" => {
            phone_flags::image(&iso)
        }
        _ => None,
    });
    div()
        .w(px(16.))
        .h(px(12.))
        .flex_none()
        .rounded(px(2.))
        .overflow_hidden()
        .bg(paint.line)
        .children(image.map(|image| img(image).size_full()))
}

fn country_name(country: &str, lang: UiLanguage) -> SharedString {
    let names = match country.to_ascii_uppercase().as_str() {
        "US" => ("United States", "美国"),
        "GB" => ("United Kingdom", "英国"),
        "DE" => ("Germany", "德国"),
        "FR" => ("France", "法国"),
        "NL" => ("Netherlands", "荷兰"),
        "SE" => ("Sweden", "瑞典"),
        "CH" => ("Switzerland", "瑞士"),
        "JP" => ("Japan", "日本"),
        "IN" => ("India", "印度"),
        "BR" => ("Brazil", "巴西"),
        _ => return country.to_owned().into(),
    };
    lang.text(names.0, names.1).into()
}

fn age(wall: SystemTime, at: SystemTime, lang: UiLanguage) -> (SharedString, SharedString) {
    let seconds = wall.duration_since(at).unwrap_or_default().as_secs();
    if seconds < 3 {
        return (
            lang.text("now", "刚刚").into(),
            lang.text("just now", "刚刚").into(),
        );
    }
    let (value, short, singular, plural, zh) = if seconds < 60 {
        (seconds, "s", "second", "seconds", "秒")
    } else if seconds < 3600 {
        (seconds / 60, "m", "minute", "minutes", "分钟")
    } else if seconds < 86400 {
        (seconds / 3600, "h", "hour", "hours", "小时")
    } else {
        (seconds / 86400, "d", "day", "days", "天")
    };
    if lang == UiLanguage::Chinese {
        let unit = match short {
            "m" => "分",
            "h" => "时",
            _ => zh,
        };
        (
            format!("{value}{unit}").into(),
            format!("{value}{zh}前").into(),
        )
    } else {
        (
            format!("{value}{short}").into(),
            format!("{value} {} ago", if value == 1 { singular } else { plural }).into(),
        )
    }
}

fn disc(center: Point<Pixels>, radius: Pixels) -> Bounds<Pixels> {
    Bounds::new(
        center - point(radius, radius),
        size(radius * 2., radius * 2.),
    )
}

fn map_position(center: Point<f32>, bounds: Bounds<Pixels>) -> Point<Pixels> {
    bounds.origin + point(bounds.size.width * center.x, bounds.size.height * center.y)
}

fn land(bounds: Bounds<Pixels>) -> Option<Path<Pixels>> {
    let mut path = PathBuilder::fill();
    let r = (bounds.size.width * 0.0016).max(px(0.5));
    for (y, row) in LAND_ROWS.into_iter().enumerate() {
        let mut row = row;
        while row != 0 {
            let x = row.trailing_zeros();
            row &= row - 1;
            let at = map_position(
                point((x as f32 + 0.5) / 100., (y as f32 + 0.5) / 36.),
                bounds,
            );
            for i in 0..8 {
                let angle = i as f32 * std::f32::consts::TAU / 8.;
                let vertex = at + point(r * angle.cos(), r * angle.sin());
                if i == 0 {
                    path.move_to(vertex);
                } else {
                    path.line_to(vertex);
                }
            }
            path.close();
        }
    }
    path.build().ok()
}

impl LiveVisitors {
    fn head(&self, compact: bool, stack_count: bool, frame: Frame, cx: &mut Context<Self>) -> Div {
        let p = frame.paint;
        let lang = frame.lang;
        let status = self.status();
        let elapsed = self.status_since.map_or(u64::MAX, |then| {
            frame.now.saturating_duration_since(then).as_millis() as u64
        });
        let old = if !disabled_motion(cx) && elapsed < OUT_MS {
            1. - elapsed as f32 / OUT_MS as f32
        } else {
            0.
        };
        let opacity = if !disabled_motion(cx) && elapsed < IN_MS {
            elapsed as f32 / IN_MS as f32
        } else {
            1.
        };
        let pulse = if status == Status::Live && frame.animate {
            frame
                .now
                .saturating_duration_since(self.started)
                .as_secs_f32()
                .rem_euclid(PULSE_MS as f32 / 1000.)
                / (PULSE_MS as f32 / 1000.)
        } else {
            1.
        };
        let dot = status.color(p);
        let pause_label = if self.paused {
            lang.text("Resume live updates", "继续实时更新")
        } else {
            lang.text("Pause live updates", "暂停实时更新")
        };
        let count = self.displayed.count;
        let label = self.label.as_ref().map_or_else(
            || lang.text("people on the site now", "人正在访问").into(),
            |label| label(count),
        );
        let horizontal = if compact { 16. } else { 20. };
        let mut header = div()
            .flex()
            .flex_col()
            .w_full()
            .relative()
            .px(px(horizontal))
            .pt(px(if compact { 16. } else { 20. }))
            .child(
                row()
                    .h(px(20.))
                    .gap(px(8.))
                    .child(
                        div()
                            .size(px(8.))
                            .flex_none()
                            .relative()
                            .when(pulse < 1., |point| {
                                let spread = pulse * 5.;
                                point.child(
                                    div()
                                        .absolute()
                                        .top(px(-spread))
                                        .left(px(-spread))
                                        .size(px(8. + spread * 2.))
                                        .rounded_full()
                                        .bg(dot.opacity((1. - pulse) * 0.22)),
                                )
                            })
                            .child(div().size(px(8.)).rounded_full().bg(dot)),
                    )
                    .child(
                        div()
                            .relative()
                            .h(px(20.))
                            .w(px(114.))
                            .id("live-visitors-status")
                            .role(Role::Status)
                            .aria_label(status.text(lang))
                            .line_height(px(20.))
                            .text_color(p.secondary)
                            .when(old > 0., |line| {
                                line.child(
                                    div()
                                        .absolute()
                                        .inset_0()
                                        .opacity(old)
                                        .child(self.last_status.text(lang)),
                                )
                            })
                            .child(
                                div()
                                    .absolute()
                                    .inset_0()
                                    .opacity(opacity)
                                    .child(status.text(lang)),
                            ),
                    ),
            );
        if self.displaying_sample {
            header = header.child(
                div()
                    .absolute()
                    .top(px(if compact { 20. } else { 24. }))
                    .right(px(if self.hide_pause {
                        horizontal
                    } else {
                        horizontal + 40.
                    }))
                    .text_size(px(12.))
                    .line_height(px(16.))
                    .text_color(p.faint)
                    .child(lang.text("Sample data", "样例数据")),
            );
        }
        if !self.hide_pause {
            header = header.child(
                row()
                    .absolute()
                    .top(px(if compact { 10. } else { 14. }))
                    .right(px(horizontal))
                    .size(px(36.))
                    .justify_center()
                    .rounded(frame.radius)
                    .border_1()
                    .border_color(transparent_black())
                    .id("live-visitors-pause")
                    .track_focus(&self.focus)
                    .role(Role::Button)
                    .aria_label(pause_label)
                    .aria_toggled(self.paused.into())
                    .cursor_pointer()
                    .hover(move |s| s.bg(p.hover))
                    .focus_visible(move |s| s.border_color(p.focus))
                    .tooltip(move |_, cx| cx.new(|_| PauseHint(pause_label)).into())
                    .on_click(cx.listener(|s, _, _, cx| s.toggle(cx)))
                    .when(self.paused, |button| {
                        button.child(icon(Glyph::Play, 16., p.secondary))
                    })
                    .when(!self.paused, |button| {
                        button.child(row().gap(px(2.)).children((0..2).map(|_| {
                            div()
                                .w(px(3.))
                                .h(px(12.))
                                .rounded(px(0.75))
                                .border_1()
                                .border_color(p.secondary)
                        })))
                    }),
            );
        }
        header.child(
            row()
                .id("live-visitor-count")
                .role(Role::Label)
                .items_end()
                .h(px(if stack_count { 64. } else { 44. }))
                .gap(px(8.))
                .when(stack_count, |r| {
                    r.flex_col().items_start().justify_end().gap(px(0.))
                })
                .when(status == Status::Connecting, |r| {
                    r.child(
                        div()
                            .w(px(60.))
                            .h(px(26.))
                            .mb(px(6.))
                            .rounded(frame.radius)
                            .bg(p.line),
                    )
                    .child(
                        div()
                            .w(px(130.))
                            .h(px(13.))
                            .mb(px(8.))
                            .rounded(px(4.))
                            .bg(p.line),
                    )
                })
                .when(status != Status::Connecting, |r| {
                    r.aria_label(format!("{count} {label}"))
                        .child(self.count.render(32., 40., p.ink, frame))
                        .child(
                            div()
                                .min_w_0()
                                .text_ellipsis()
                                .line_height(px(20.))
                                .pb(px(3.))
                                .text_color(p.secondary)
                                .child(label),
                        )
                }),
        )
    }

    fn map(&self, width: Pixels, frame: Frame, w: &Window) -> Div {
        let p = frame.paint;
        let height = width * 0.36;
        let cached = self.map.clone();
        let pings = self
            .pings
            .iter()
            .filter_map(|ping| {
                let removed = ping.removed.map_or(0., |then| {
                    if frame.animate {
                        frame.now.saturating_duration_since(then).as_secs_f32()
                            / ms(EXPIRE_MS).as_secs_f32()
                    } else {
                        1.
                    }
                });
                let age = frame
                    .now
                    .saturating_duration_since(ping.since)
                    .as_secs_f32();
                (removed < 1.).then_some((ping.center, ping.kind, removed, age))
            })
            .collect::<Vec<_>>();
        let map = canvas(
            |_, _, _| (),
            move |b, _, w, _| {
                if cached
                    .borrow()
                    .as_ref()
                    .is_none_or(|(bounds, _)| *bounds != b)
                {
                    *cached.borrow_mut() = land(b).map(|path| (b, path));
                }
                if let Some((_, base)) = cached.borrow().as_ref() {
                    w.paint_path(base.clone(), p.ink.opacity(0.23));
                }
                let scale = (b.size.width / px(488.)).clamp(0.4, 1.3);
                for (center, kind, removed, age) in pings {
                    let event = kind == LivePingKind::Event;
                    let center = map_position(center, b);
                    let radius = px(if event { 3. } else { 2.2 }) * scale;
                    let enter = if frame.animate {
                        (age / 0.16).clamp(0., 1.)
                    } else {
                        1.
                    };
                    let alpha = enter * (1. - removed);
                    let core = p.ink.opacity(alpha * if event { 0.9 } else { 0.62 });
                    w.paint_quad(fill(disc(center, radius), core).corner_radii(radius));
                    let phase = (age / ms(PING_MS).as_secs_f32()).clamp(0., 1.);
                    if phase < 1. && frame.animate && removed == 0. {
                        let spread = radius
                            + px((1. - (1. - phase).powi(2)) * if event { 13. } else { 8. })
                                * scale;
                        w.paint_quad(
                            outline(
                                disc(center, spread),
                                p.ink
                                    .opacity((1. - phase) * if event { 0.63 } else { 0.42 }),
                                BorderStyle::Solid,
                            )
                            .corner_radii(spread),
                        );
                    }
                }
            },
        )
        .absolute()
        .inset_0();
        let show_city = self
            .city
            .as_ref()
            .filter(|(_, since)| frame.now.saturating_duration_since(*since) < ms(CITY_MS));
        div()
            .relative()
            .w_full()
            .h(height)
            .child(map)
            .children(show_city.and_then(|(event, _)| {
                let center = event.location?.point()?;
                let text_width =
                    px(measure(&event.city, 12., w).ceil()).min((width - px(46.)).max(px(0.)));
                let badge_width = text_width + px(46.);
                let x =
                    (width * center.x - px(12.)).clamp(px(0.), (width - badge_width).max(px(0.)));
                let y = (height * center.y - px(34.)).max(px(0.));
                Some(
                    row()
                        .absolute()
                        .left(x)
                        .top(y)
                        .h(px(24.))
                        .w(badge_width)
                        .rounded(frame.radius)
                        .bg(p.panel)
                        .border_1()
                        .border_color(p.line)
                        .px(px(8.))
                        .gap(px(6.))
                        .child(flag(event, p))
                        .child(
                            div()
                                .min_w_0()
                                .text_ellipsis()
                                .text_size(px(12.))
                                .child(event.city.clone()),
                        ),
                )
            }))
    }

    fn page_rows(&self, frame: Frame) -> Stateful<Div> {
        let p = frame.paint;
        let lang = frame.lang;
        let mut list = list_surface(
            "live-pages",
            self.max_pages,
            lang.text("Top pages", "热门页面"),
        );
        if self.status() == Status::Connecting {
            return list.children(placeholders(self.max_pages, p));
        }
        for page in &self.pages {
            let opacity = page.opacity.sample(frame.now, frame.animate);
            if opacity <= 0. {
                continue;
            }
            let width = page.content.bar.sample(frame.now, frame.animate);
            let count = page.content.visitors.count;
            list = list.child(
                row_slot(page, frame)
                    .id(page.id.clone())
                    .role(Role::ListItem)
                    .aria_label(if lang == UiLanguage::Chinese {
                        format!("{}，{count} 位访客", page.id)
                    } else {
                        format!("{}: {count} visitors", page.id)
                    })
                    .child(
                        div()
                            .absolute()
                            .top_0()
                            .bottom_0()
                            .left_0()
                            .w(relative(width))
                            .rounded(frame.radius)
                            .bg(p.ink.opacity(0.10)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_ellipsis()
                            .child(page.id.clone()),
                    )
                    .child(page.content.visitors.render(14., 20., p.secondary, frame)),
            );
        }
        if self.pages.is_empty() {
            list = list.child(empty_row(lang.text("No page activity", "暂无页面访问"), p));
        }
        list
    }

    fn event_rows(&self, frame: Frame) -> Stateful<Div> {
        let p = frame.paint;
        let lang = frame.lang;
        let mut list = list_surface(
            "live-events",
            self.max_events,
            lang.text("Recent signups and upgrades", "近期注册和升级"),
        );
        if self.status() == Status::Connecting {
            return list.children(placeholders(self.max_events, p));
        }
        for entry in &self.events {
            if entry.opacity.sample(frame.now, frame.animate) <= 0. {
                continue;
            }
            let event = &entry.content;
            let (short, readable) = age(frame.wall, event.at, lang);
            let description = format!(
                "{}, {}, {}, {readable}",
                event.label,
                event.city,
                country_name(&event.country, lang)
            );
            list = list.child(
                row_slot(entry, frame)
                    .id(entry.id.clone())
                    .role(Role::ListItem)
                    .aria_label(description)
                    .child(flag(event, p))
                    .child(
                        row()
                            .min_w_0()
                            .flex_1()
                            .gap(px(6.))
                            .overflow_hidden()
                            .child(
                                div()
                                    .flex_shrink_0()
                                    .max_w(relative(0.72))
                                    .text_ellipsis()
                                    .child(event.label.clone()),
                            )
                            .child(
                                div()
                                    .min_w_0()
                                    .text_ellipsis()
                                    .text_color(p.faint)
                                    .child(event.city.clone()),
                            ),
                    )
                    .child(
                        div()
                            .w(px(if lang == UiLanguage::Chinese {
                                38.
                            } else {
                                29.
                            }))
                            .text_right()
                            .text_ellipsis()
                            .text_size(px(12.))
                            .text_color(p.faint)
                            .flex_none()
                            .child(short),
                    ),
            );
        }
        if self.events.is_empty() {
            list = list.child(empty_row(lang.text("No recent conversions", "暂无转化"), p));
        }
        list
    }
}

fn list_surface(id: &'static str, limit: usize, label: &'static str) -> Stateful<Div> {
    div()
        .debug_selector(|| id.to_owned())
        .id(id)
        .role(Role::List)
        .aria_label(label)
        .relative()
        .w_full()
        .overflow_hidden()
        .h(px(if limit == 0 {
            0.
        } else {
            limit as f32 * STEP - 4.
        }))
}

fn row_slot<T>(entry: &Row<T>, frame: Frame) -> Div {
    row()
        .absolute()
        .left_0()
        .right_0()
        .top(px(entry.offset.sample(frame.now, frame.animate)))
        .h(px(ROW))
        .px(px(10.))
        .gap(px(10.))
        .opacity(entry.opacity.sample(frame.now, frame.animate))
        .overflow_hidden()
}

fn empty_row(label: &'static str, p: Paint) -> Div {
    row()
        .h(px(ROW))
        .px(px(10.))
        .text_color(p.faint)
        .child(label)
}

fn placeholders(count: usize, p: Paint) -> impl Iterator<Item = Div> {
    (0..count).map(move |i| {
        div()
            .absolute()
            .left_0()
            .top(px(i as f32 * STEP))
            .w(relative(0.76 - (i % 4) as f32 * 0.1))
            .h(px(ROW))
            .rounded(px(8.))
            .bg(p.line)
    })
}

fn panel(title: &'static str, caption: &'static str, body: Stateful<Div>, p: Paint) -> Div {
    div()
        .flex()
        .flex_col()
        .flex_1()
        .min_w_0()
        .gap(px(8.))
        .child(
            row()
                .h(px(20.))
                .gap(px(8.))
                .px(px(10.))
                .justify_between()
                .child(
                    div()
                        .text_ellipsis()
                        .font_weight(FontWeight::MEDIUM)
                        .child(title),
                )
                .child(
                    div()
                        .min_w_0()
                        .text_ellipsis()
                        .text_size(px(12.))
                        .text_color(p.faint)
                        .child(caption),
                ),
        )
        .child(body)
}

impl Render for LiveVisitors {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.start_clock(cx);
        let now = cx.background_executor().now();
        let animate = self.visible(cx) && !disabled_motion(cx) && !self.paused;
        let frame = Frame {
            now,
            wall: self.last_wall,
            animate,
            lang: UiLanguage::current(cx),
            paint: Paint::get(cx),
            radius: surface_radius(cx),
        };
        let layout = self.layout.get();
        let width = if layout.width > px(0.) {
            layout.width
        } else if w.viewport_size().width > px(0.) {
            w.viewport_size().width.min(px(WIDTH))
        } else {
            px(WIDTH)
        };
        let compact = width - px(2.) <= px(520.);
        let edge = if compact { 16. } else { 20. };
        let p = frame.paint;
        let lang = frame.lang;
        let connecting = self.status() == Status::Connecting;
        self.pages
            .retain(|row| !row.leaving || (animate && row.opacity.active(now)));
        self.events
            .retain(|row| !row.leaving || (animate && row.opacity.active(now)));
        if self.visible(cx)
            && !disabled_motion(cx)
            && (self
                .status_since
                .is_some_and(|since| now.saturating_duration_since(since) < ms(IN_MS))
                || (animate
                    && (self.status() == Status::Live
                        || self.count.active(now)
                        || self.pages.iter().any(|r| {
                            r.opacity.active(now)
                                || r.offset.active(now)
                                || r.content.bar.active(now)
                                || r.content.visitors.active(now)
                        })
                        || self
                            .events
                            .iter()
                            .any(|r| r.opacity.active(now) || r.offset.active(now)))))
        {
            w.request_animation_frame();
        }
        let map_width = (width - px(2. + edge * 2.)).max(px(1.));
        let map_inset = if compact { 12. } else { 16. };
        let map = div()
            .w_full()
            .px(px(edge))
            .py(px(map_inset))
            .when(connecting, |e| {
                e.child(
                    div()
                        .w_full()
                        .h(map_width * 0.36)
                        .rounded(frame.radius)
                        .bg(p.line),
                )
            })
            .when(!connecting, |e| e.child(self.map(map_width, frame, w)));
        let page = (self.max_pages > 0).then(|| {
            panel(
                lang.text("Top pages", "热门页面"),
                lang.text("Visitors", "访客"),
                self.page_rows(frame),
                p,
            )
        });
        let events = (self.max_events > 0).then(|| {
            panel(
                lang.text("Just now", "刚刚"),
                lang.text("Signups and upgrades", "注册和升级"),
                self.event_rows(frame),
                p,
            )
        });
        let body = div()
            .w_full()
            .flex()
            .gap(px(if compact { 16. } else { 24. }))
            .border_t_1()
            .border_color(p.line)
            .px(px(if compact { 6. } else { 10. }))
            .pt(px(if compact { 12. } else { 16. }))
            .pb(px(if compact { 12. } else { 15. }))
            .when(compact, |e| e.flex_col())
            .children(page)
            .children(events);
        let control = div()
            .w_full()
            .max_w(px(WIDTH))
            .relative()
            .flex()
            .flex_col()
            .rounded(frame.radius)
            .border_1()
            .border_color(p.line)
            .bg(p.panel)
            .text_color(p.ink)
            .text_size(px(14.))
            .id(self.id.clone())
            .role(Role::Region)
            .aria_label(
                self.title
                    .clone()
                    .unwrap_or_else(|| lang.text("Live visitors", "实时访客").into()),
            )
            .child(self.head(compact, width < px(360.), frame, cx))
            .child(map)
            .when(self.max_pages > 0 || self.max_events > 0, |e| e.child(body))
            .into_any_element();
        let layout = self.layout.clone();
        let shell = div()
            .w_full()
            .max_w(px(WIDTH))
            .child(control)
            .on_children_prepainted(move |bounds, w, _| {
                if let Some(bounds) = bounds.first() {
                    let viewport = Bounds::new(Point::default(), w.viewport_size());
                    let next = LayoutSample {
                        width: bounds.size.width,
                        visible: w.is_visible() && bounds.intersects(&viewport),
                    };
                    if layout.replace(next) != next {
                        w.request_animation_frame();
                    }
                }
            });
        match &self.style {
            Some(style) => style(shell),
            None => shell,
        }
    }
}

const CITIES: &[(&str, &str, &str, LiveLocation)] = &[
    ("Berlin", "柏林", "DE", LiveLocation::new(52.52, 13.405)),
    (
        "San Francisco",
        "旧金山",
        "US",
        LiveLocation::new(37.77, -122.42),
    ),
    ("Tokyo", "东京", "JP", LiveLocation::new(35.68, 139.69)),
    ("London", "伦敦", "GB", LiveLocation::new(51.51, -0.13)),
    (
        "Amsterdam",
        "阿姆斯特丹",
        "NL",
        LiveLocation::new(52.37, 4.9),
    ),
    (
        "São Paulo",
        "圣保罗",
        "BR",
        LiveLocation::new(-23.55, -46.63),
    ),
    (
        "Stockholm",
        "斯德哥尔摩",
        "SE",
        LiveLocation::new(59.33, 18.07),
    ),
    ("New York", "纽约", "US", LiveLocation::new(40.71, -74.01)),
    ("Paris", "巴黎", "FR", LiveLocation::new(48.86, 2.35)),
    ("Austin", "奥斯汀", "US", LiveLocation::new(30.27, -97.74)),
    ("Zürich", "苏黎世", "CH", LiveLocation::new(47.38, 8.54)),
    (
        "Bengaluru",
        "班加罗尔",
        "IN",
        LiveLocation::new(12.97, 77.59),
    ),
];

/// In-memory example feed. Each call to `advance` advances the sample by 600ms.
pub struct LiveVisitorsSimulation {
    epoch: SystemTime,
    step: u64,
    active: bool,
}

impl LiveVisitorsSimulation {
    pub fn new(epoch: SystemTime) -> Self {
        Self {
            epoch,
            step: 0,
            active: true,
        }
    }

    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    pub fn advance(&mut self) {
        if self.active {
            self.step = self.step.saturating_add(1);
        }
    }

    pub fn now(&self) -> SystemTime {
        self.epoch
            .checked_add(ms(self.step.saturating_mul(DEMO_MS)))
            .unwrap_or(self.epoch)
    }

    pub fn snapshot(&self, lang: UiLanguage) -> LiveVisitorsSnapshot {
        const PATHS: [&str; 5] = [
            "/pricing",
            "/docs/quickstart",
            "/",
            "/changelog",
            "/blog/realtime-analytics",
        ];
        let step = self.step;
        let drift = (step as f64 * 0.28).sin() * 13. + (step as f64 * 0.09).sin() * 9.;
        let count = (128. + drift).round() as u64;
        let pages = PATHS
            .into_iter()
            .zip([31., 24., 19., 12., 8.])
            .enumerate()
            .map(|(i, (path, base))| {
                let delta = ((step as f64 * (0.12 + i as f64 * 0.025)).sin() * (3. + i as f64))
                    + (step as f64 * (0.22 + i as f64 * 0.08)).sin() * (1. + i as f64);
                LivePage::new(path, (base + delta).round().max(1.) as u64)
            })
            .collect();
        let visitors = (step.saturating_sub(11)..=step)
            .map(|n| {
                let location = CITIES
                    [((n.wrapping_mul(7).wrapping_add(n / 7)) % CITIES.len() as u64) as usize]
                    .3;
                LiveVisitorPing::new(format!("arrival-{n}"), location).with_kind(
                    if n.is_multiple_of(4) {
                        LivePingKind::Event
                    } else {
                        LivePingKind::Arrival
                    },
                )
            })
            .chain(
                (step..12)
                    .map(|i| LiveVisitorPing::new(format!("initial-{i}"), CITIES[i as usize].3)),
            )
            .collect();
        let round = step / 4;
        let mut events = (round.saturating_sub(3)..=round)
            .rev()
            .map(|n| {
                let i = (n.wrapping_mul(7).wrapping_add(n / 9) % CITIES.len() as u64) as usize;
                let (en, zh, country, location) = CITIES[i];
                let label = if n % 3 == 1 {
                    lang.text("Upgraded to Pro", "已升级到专业版")
                } else {
                    lang.text("Signed up", "已注册")
                };
                LiveVisitorEvent::new(
                    format!("conversion-{n}"),
                    label,
                    lang.text(en, zh),
                    country,
                    self.epoch + ms(n.saturating_mul(DEMO_MS * 4)),
                )
                .with_location(location)
            })
            .collect::<Vec<_>>();
        for (i, delay) in [(1, 16), (2, 38), (3, 60)]
            .into_iter()
            .take(4 - events.len())
        {
            let (en, zh, country, location) = CITIES[i];
            events.push(
                LiveVisitorEvent::new(
                    format!("initial-event-{i}"),
                    if i == 1 {
                        lang.text("Upgraded to Pro", "已升级到专业版")
                    } else {
                        lang.text("Signed up", "已注册")
                    },
                    lang.text(en, zh),
                    country,
                    self.epoch
                        .checked_sub(Duration::from_secs(delay))
                        .unwrap_or(self.epoch),
                )
                .with_location(location),
            );
        }
        LiveVisitorsSnapshot {
            count,
            pages,
            visitors,
            events,
        }
    }
}

// Point-in-polygon samples of Natural Earth public-domain 1:110m land, v5.1.2.
// Cell centers cover longitude -180..180 and latitude 84..-60.
const LAND_ROWS: [u128; 36] = [
    0x0_0000_0000_0000_0ffe_fe00_0000,
    0x0_0000_0000_0140_1fff_dc00_0000,
    0x0_0001_fc02_0000_0ffc_0103_8000,
    0x2_3fff_ffa0_0300_0ff8_7cbe_43e0,
    0xf_ffff_ffff_efe0_38f9_63ff_fff7,
    0x7_7fff_ffff_ff78_0030_b0ff_fff0,
    0x0_41ff_ffff_fe61_0001_e0ff_e040,
    0x0_61ff_ffff_fff2_8003_f7ff_c000,
    0x0_03ff_ffff_fffc_0004_7fff_c000,
    0x0_00ff_fffe_ebfc_0000_ffff_0000,
    0x0_003f_fffe_4bb7_8000_3fff_8000,
    0x0_0027_fffe_ff23_0000_1fff_0000,
    0x0_0047_ffff_f01f_0000_0ffe_0000,
    0x0_000f_ffff_ffff_8000_05f8_0000,
    0x0_0007_fffc_ffff_c000_0870_0000,
    0x0_0001_f7e7_efff_e000_0460_0000,
    0x0_0008_f1c3_dfff_c000_93c0_0000,
    0x0_0000_e080_ffff_e000_0600_0000,
    0x0_0000_a080_ffff_c001_e400_0000,
    0x0_0004_0000_fffb_8003_e000_0000,
    0x0_0026_6000_7fe0_000f_f000_0000,
    0x0_010a_4000_1fe0_003f_f000_0000,
    0x0_0f00_8000_1fe0_00ff_f800_0000,
    0x0_0818_0000_1fc0_00ff_f000_0000,
    0x0_02c0_0000_9fe0_007f_e000_0000,
    0x8_07f0_0000_cfe0_007f_c000_0000,
    0x0_0ffc_0000_4fc0_007f_8000_0000,
    0x0_1ffc_0000_07c0_001f_c000_0000,
    0x0_1ffc_0000_0780_000f_c000_0000,
    0x0_0f04_0000_0080_0007_c000_0000,
    0x8_0600_0000_0000_0003_c000_0000,
    0x0_0400_0000_0000_0000_e000_0000,
    0x1_0000_0000_0000_0000_6000_0000,
    0x0_0000_0000_0000_0000_6000_0000,
    0x0_0000_0000_0000_0000_4000_0000,
    0x0_0000_0000_0000_0000_0000_0000,
];

#[cfg(test)]
mod tests {
    use super::{
        DIGIT_MS, Instant, LiveConnection, LiveVisitors, LiveVisitorsSnapshot, ROW_MS, Roll, STEP,
        Status, ms, sync_rows,
    };
    use gpui::{SharedString, TestAppContext, prelude::*, px, size};

    fn snapshot(count: u64) -> LiveVisitorsSnapshot {
        LiveVisitorsSnapshot {
            count,
            ..Default::default()
        }
    }

    #[gpui::test]
    fn pause_and_reconnect_commit_only_the_latest_snapshot(cx: &mut TestAppContext) {
        let control =
            cx.update(|cx| cx.new(|cx| LiveVisitors::new(cx).with_snapshot(snapshot(128))));
        control.update(cx, |card, cx| {
            card.set_paused(true, cx);
            card.set_snapshot(snapshot(129), cx);
            card.set_snapshot(snapshot(131), cx);
            assert_eq!(card.snapshot().count, 128);
            card.set_connection(LiveConnection::Reconnecting, cx);
            card.set_paused(false, cx);
            assert_eq!(card.status(), Status::Reconnecting);
            assert_eq!(card.snapshot().count, 128);
            card.set_snapshot(snapshot(146), cx);
            card.set_connection(LiveConnection::Live, cx);
            assert_eq!(card.snapshot().count, 146);
            assert!(card.pending.is_none());
        });
    }

    #[gpui::test]
    fn switching_to_host_data_keeps_the_paused_frame_and_drops_demo_pings_on_resume(
        cx: &mut TestAppContext,
    ) {
        let control = cx.update(|cx| cx.new(|cx| LiveVisitors::new(cx).with_default_paused(true)));
        control.update(cx, |card, cx| {
            let old = card.displayed.events[0].id.clone();
            let n = card.pings.len();
            assert!(n > 0);
            card.set_snapshot(snapshot(20), cx);
            assert_eq!(card.displayed.events[0].id, old);
            assert_eq!(card.pings.len(), n);
            assert!(card.displaying_sample);
            card.set_paused(false, cx);
            assert_eq!(card.snapshot().count, 20);
            assert!(card.pings.is_empty() && card.events.is_empty() && !card.displaying_sample);
        });
    }

    #[test]
    fn interrupted_reordering_starts_at_the_row_current_position() {
        let start = Instant::now();
        let mut rows = Vec::new();
        let list = |a: &str, b: &str| {
            [
                (SharedString::from(a.to_owned()), 1),
                (SharedString::from(b.to_owned()), 2),
            ]
        };
        sync_rows(&mut rows, list("a", "b"), start, true, |_, value| value);
        sync_rows(&mut rows, list("b", "a"), start, false, |_, value| value);
        let middle = start + ms(65);
        let at = rows[0].offset.sample(middle, true);
        assert!(at > 0. && at < STEP);
        sync_rows(&mut rows, list("a", "b"), middle, false, |_, value| value);
        assert_eq!(rows[1].offset.sample(middle, true), at);
        assert_eq!(rows[1].offset.sample(middle + ms(ROW_MS), true), STEP);
    }

    #[test]
    fn carry_keeps_unchanged_decimal_places_and_grows_the_leading_place() {
        let now = Instant::now();
        let mut roll = Roll::new(109, now);
        roll.set(110, now);
        assert!(!roll.places[2].0.active(now));
        assert!(roll.places[0].0.active(now));
        let mut roll = Roll::new(99, now);
        roll.set(100, now);
        let width = roll.places[2].1;
        assert_eq!(width.sample(now, true), 0.);
        assert!(width.sample(now + ms(65), true) > 0.);
        assert_eq!(width.sample(now + ms(DIGIT_MS), true), 1.);
    }

    #[test]
    fn column_breakpoint_uses_the_laid_out_card_width() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (control, window) =
            app.add_window_view(|_, cx| LiveVisitors::new(cx).with_default_paused(true));
        for (width, stacked) in [(640., false), (522., true), (521.6, true), (320., true)] {
            window.simulate_resize(size(px(width), px(750.)));
            window.update(|w, _| w.refresh());
            window.run_until_parked();
            window.update(|w, _| w.refresh());
            window.run_until_parked();
            let pages = window.debug_bounds("live-pages").expect("pages list");
            let events = window
                .debug_bounds("live-events")
                .expect("conversions list");
            assert_eq!(events.top() > pages.top() + px(160.), stacked);
            window.update(|_, cx| {
                let measured = control.read(cx).layout.get().width;
                assert!((measured - px(width)).abs() <= px(0.5));
            });
        }
    }
}
