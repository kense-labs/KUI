//! A scrollable feed with a resisted pull gesture and asynchronous refresh.
use crate::{
    foundation::Theme,
    primitives::{
        control::UiLanguage,
        family::{Paint, PhaseClock, Slide, Tone, disabled_motion, face},
        glyph::{Glyph, icon},
    },
};
use gpui::{prelude::*, *};
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
    time::Duration,
};
use web_time::Instant;

const DEFAULT_THRESHOLD: f32 = 72.;
const WHEEL_RELEASE: Duration = Duration::from_millis(140);
const RESULT_TIME: Duration = Duration::from_millis(1150);

/// Error from the host's refresh operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StretchRefreshError(pub SharedString);

impl std::fmt::Display for StretchRefreshError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for StretchRefreshError {}

/// The host may supply a short visible result label.
pub type StretchRefreshResult = Result<Option<SharedString>, StretchRefreshError>;

#[derive(Clone, Debug)]
enum Phase {
    Idle,
    Pulling,
    Refreshing,
    Result {
        text: SharedString,
        failed: bool,
        closing: bool,
    },
}

#[derive(Clone, Copy)]
enum Gesture {
    Mouse,
    Touch,
    Wheel,
}

#[derive(Clone, Copy)]
enum DragIntent {
    Pending,
    Pull,
    Scroll,
    OtherAxis,
}

struct Drag {
    origin: Point<Pixels>,
    scroll_y: Pixels,
    intent: DragIntent,
}

type ItemKey<T> = Box<dyn Fn(&T) -> SharedString>;
type ItemView<T> = Box<dyn Fn(&T, &mut Window, &mut App) -> AnyElement>;
type Refresh<T> =
    Box<dyn Fn(&mut Window, &mut Context<StretchRefresh<T>>) -> Task<StretchRefreshResult>>;
type Style = Box<dyn Fn(Div) -> Div>;
type Visibility = Box<dyn Fn(&App) -> bool>;
type ScrollRegion = Rc<dyn Fn(Bounds<Pixels>, bool, bool, &App)>;

/// Hosts keyed GPUI rows in a refreshable panel.
pub struct StretchRefresh<T: 'static> {
    id: ElementId,
    title: SharedString,
    subtitle: SharedString,
    previous_subtitle: SharedString,
    labels: [Option<SharedString>; 3],
    rows: Vec<(SharedString, T)>,
    key: ItemKey<T>,
    render_item: ItemView<T>,
    on_refresh: Refresh<T>,
    style: Option<Style>,
    visibility: Option<Visibility>,
    scroll_region: Option<ScrollRegion>,
    threshold: f32,
    phase: Phase,
    gesture: Option<Gesture>,
    raw: f32,
    distance: Slide,
    subtitle_motion: Slide,
    new_rows: HashSet<SharedString>,
    row_heights: Rc<RefCell<HashMap<SharedString, Pixels>>>,
    row_motion: Slide,
    status_clock: PhaseClock,
    scroll: ScrollHandle,
    region_focus: FocusHandle,
    drag: Option<Drag>,
    wheel_native: Option<Instant>,
    wheel_release: Option<Task<()>>,
    request: Option<Task<()>>,
    result_clear: Option<Task<()>>,
}

impl<T: 'static> StretchRefresh<T> {
    /// The key must be stable across refreshes. The renderer owns row content.
    pub fn new(
        title: impl Into<SharedString>,
        items: Vec<T>,
        get_key: impl Fn(&T) -> SharedString + 'static,
        render_item: impl Fn(&T, &mut Window, &mut App) -> AnyElement + 'static,
        on_refresh: impl Fn(&mut Window, &mut Context<Self>) -> Task<StretchRefreshResult> + 'static,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            id: "stretch-refresh".into(),
            title: title.into(),
            subtitle: "".into(),
            previous_subtitle: "".into(),
            labels: Default::default(),
            rows: keyed(items, &get_key),
            key: Box::new(get_key),
            render_item: Box::new(render_item),
            on_refresh: Box::new(on_refresh),
            style: None,
            visibility: None,
            scroll_region: None,
            threshold: DEFAULT_THRESHOLD,
            phase: Phase::Idle,
            gesture: None,
            raw: 0.,
            distance: Slide::new(0.),
            subtitle_motion: Slide::new(1.),
            new_rows: HashSet::new(),
            row_heights: Rc::default(),
            row_motion: Slide::new(1.),
            status_clock: PhaseClock::default(),
            scroll: ScrollHandle::new(),
            region_focus: cx.focus_handle(),
            drag: None,
            wheel_native: None,
            wheel_release: None,
            request: None,
            result_clear: None,
        }
    }

    pub fn with_id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = id.into();
        self
    }

    pub fn with_subtitle(mut self, subtitle: impl Into<SharedString>) -> Self {
        self.subtitle = subtitle.into();
        self
    }

    /// Distance after resistance. A positive finite value is required.
    pub fn with_threshold(mut self, threshold: Pixels) -> Self {
        let threshold = f32::from(threshold);
        if threshold.is_finite() && threshold > 0. {
            self.threshold = threshold;
        }
        self
    }

    pub fn with_done_label(mut self, label: impl Into<SharedString>) -> Self {
        self.labels[0] = Some(label.into());
        self
    }

    pub fn with_error_label(mut self, label: impl Into<SharedString>) -> Self {
        self.labels[1] = Some(label.into());
        self
    }

    pub fn with_refresh_label(mut self, label: impl Into<SharedString>) -> Self {
        self.labels[2] = Some(label.into());
        self
    }

    pub fn with_style(mut self, style: impl Fn(Div) -> Div + 'static) -> Self {
        self.style = Some(Box::new(style));
        self
    }

    pub fn with_visibility(mut self, visible: impl Fn(&App) -> bool + 'static) -> Self {
        self.visibility = Some(Box::new(visible));
        self
    }

    /// Reports the scroll and pull hit area to an embedding host.
    pub fn with_scroll_region(
        mut self,
        report: impl Fn(Bounds<Pixels>, bool, bool, &App) + 'static,
    ) -> Self {
        self.scroll_region = Some(Rc::new(report));
        self
    }

    pub fn is_refreshing(&self) -> bool {
        matches!(self.phase, Phase::Refreshing)
    }

    pub fn items(&self) -> impl Iterator<Item = &T> {
        self.rows.iter().map(|(_, item)| item)
    }

    /// Replaces rows using their stable keys and animates newly inserted rows.
    pub fn set_items(&mut self, items: Vec<T>, cx: &mut Context<Self>) {
        let old: HashSet<&str> = self.rows.iter().map(|(key, _)| key.as_ref()).collect();
        let rows = keyed(items, self.key.as_ref());
        self.new_rows = rows
            .iter()
            .filter(|(key, _)| !old.contains(key.as_ref()))
            .map(|(key, _)| key.clone())
            .collect();
        self.rows = rows;
        self.row_heights.borrow_mut().clear();
        if !self.new_rows.is_empty() {
            self.row_motion.reset(0.);
            self.row_motion.duration(1., 0.39, cx);
        }
        cx.notify();
    }

    pub fn set_subtitle(&mut self, subtitle: impl Into<SharedString>, cx: &mut Context<Self>) {
        let subtitle = subtitle.into();
        if self.subtitle != subtitle {
            self.previous_subtitle = std::mem::replace(&mut self.subtitle, subtitle);
            self.subtitle_motion.reset(0.);
            self.subtitle_motion.duration(1., 0.25, cx);
            cx.notify();
        }
    }

    pub fn scroll_to_top(&self, cx: &mut Context<Self>) {
        self.scroll.set_offset(Point::default());
        cx.notify();
    }

    fn label(&self, index: usize, lang: UiLanguage) -> SharedString {
        let defaults = [
            lang.text("Up to date", "已是最新"),
            lang.text("Couldn’t refresh", "刷新失败"),
            lang.text("Refresh", "刷新"),
        ];
        self.labels[index]
            .clone()
            .unwrap_or_else(|| defaults[index].into())
    }

    /// Starts one operation. Calls while a refresh is in flight are ignored.
    pub fn refresh(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.is_refreshing() {
            return;
        }
        self.result_clear.take();
        self.wheel_release.take();
        self.gesture = None;
        self.drag = None;
        self.raw = 0.;
        self.phase = Phase::Refreshing;
        self.status_clock.restart(cx);
        self.distance
            .duration(self.threshold.min(120.) + 32., 0.29, cx);
        let task = (self.on_refresh)(w, cx);
        self.request = Some(cx.spawn(async move |weak, cx| {
            let result = task.await;
            let _ = weak.update(cx, |this, cx| this.finish_refresh(result, cx));
        }));
        cx.notify();
    }

    fn finish_refresh(&mut self, result: StretchRefreshResult, cx: &mut Context<Self>) {
        self.request = None;
        let lang = UiLanguage::current(cx);
        let failed = result.is_err();
        let text = match result {
            Ok(Some(text)) if !text.is_empty() => text,
            _ => self.label(usize::from(failed), lang),
        };
        self.phase = Phase::Result {
            text,
            failed,
            closing: false,
        };
        self.status_clock.restart(cx);
        self.distance.duration(52., 0.26, cx);
        let timer = cx.background_executor().timer(RESULT_TIME);
        self.result_clear = Some(cx.spawn(async move |weak, cx| {
            timer.await;
            let _ = weak.update(cx, |this, cx| {
                this.result_clear = None;
                if let Phase::Result { closing, .. } = &mut this.phase {
                    *closing = true;
                }
                if disabled_motion(cx) {
                    this.status_clock.restart(cx);
                }
                this.distance.duration(0., 0.34, cx);
                cx.notify();
            });
        }));
        cx.notify();
    }

    fn at_top(&self) -> bool {
        self.scroll.offset().y >= px(-0.5)
    }

    fn begin_pull(&mut self, gesture: Gesture) {
        self.result_clear.take();
        self.gesture = Some(gesture);
        self.phase = Phase::Pulling;
        self.raw = 0.;
    }

    fn move_pull(&mut self, raw: f32, cx: &mut Context<Self>) {
        self.raw = raw.max(0.);
        self.distance.reset(resisted(self.raw, self.threshold));
        cx.notify();
    }

    fn release_pull(&mut self, cancel: bool, w: &mut Window, cx: &mut Context<Self>) {
        if self.gesture.take().is_none() {
            return;
        }
        self.wheel_release.take();
        if !cancel && resisted(self.raw, self.threshold) >= self.threshold {
            self.refresh(w, cx);
        } else {
            self.phase = Phase::Idle;
            self.distance.duration(0., 0.36, cx);
            cx.notify();
        }
    }

    fn mouse_move(&mut self, event: &MouseMoveEvent, w: &mut Window, cx: &mut Context<Self>) {
        let Some(mut drag) = self.drag.take() else {
            return;
        };
        if event.pressed_button != Some(MouseButton::Left) {
            self.release_pull(true, w, cx);
            return;
        }
        let shift = event.position - drag.origin;
        if matches!(drag.intent, DragIntent::Pending) && shift.x.abs().max(shift.y.abs()) >= px(5.)
        {
            drag.intent = if shift.x.abs() > shift.y.abs() {
                DragIntent::OtherAxis
            } else if self.at_top() && shift.y > px(0.) && self.gesture.is_none() {
                self.begin_pull(Gesture::Mouse);
                DragIntent::Pull
            } else {
                DragIntent::Scroll
            };
        }
        match drag.intent {
            DragIntent::Pull => self.move_pull(f32::from(shift.y), cx),
            DragIntent::Scroll => {
                self.scroll.set_offset(point(
                    px(0.),
                    (drag.scroll_y + shift.y).clamp(-self.scroll.max_offset().y, px(0.)),
                ));
                cx.notify();
            }
            DragIntent::Pending | DragIntent::OtherAxis => {}
        }
        if matches!(drag.intent, DragIntent::Pull | DragIntent::Scroll) {
            w.prevent_default();
            cx.stop_propagation();
        }
        self.drag = Some(drag);
    }

    fn mouse_up(&mut self, event: &MouseUpEvent, w: &mut Window, cx: &mut Context<Self>) {
        if event.button != MouseButton::Left {
            return;
        }
        if let Some(drag) = self.drag.take()
            && matches!(drag.intent, DragIntent::Pull | DragIntent::Scroll)
        {
            if matches!(drag.intent, DragIntent::Pull) {
                self.move_pull(f32::from(event.position.y - drag.origin.y), cx);
                self.release_pull(false, w, cx);
            }
            w.prevent_default();
            cx.stop_propagation();
        }
    }

    fn wheel(&mut self, event: &ScrollEvent, w: &mut Window, cx: &mut Context<Self>) {
        if self.is_refreshing() || self.drag.is_some() || event.control || event.platform {
            return;
        }
        let delta = event.delta.pixel_delta(px(20.));
        if delta.x.abs() > delta.y.abs() {
            return;
        }
        let dy = f32::from(delta.y);
        if event.source == ScrollSource::Touch {
            if event.touch_phase == TouchPhase::Started
                && dy > 0.
                && self.at_top()
                && self.gesture.is_none()
            {
                self.begin_pull(Gesture::Touch);
            }
            if !matches!(self.gesture, Some(Gesture::Touch)) {
                return;
            }
            self.move_pull(self.raw + dy, cx);
            if matches!(event.touch_phase, TouchPhase::Ended | TouchPhase::Cancelled) {
                self.release_pull(event.touch_phase == TouchPhase::Cancelled, w, cx);
            }
            cx.stop_propagation();
            return;
        }

        let now = cx.background_executor().now();
        if !self.at_top() {
            self.wheel_native = Some(now);
            return;
        }
        if self.gesture.is_none()
            && dy > 0.
            && self
                .wheel_native
                .is_none_or(|last| now.saturating_duration_since(last) > WHEEL_RELEASE)
        {
            self.begin_pull(Gesture::Wheel);
        }
        if !matches!(self.gesture, Some(Gesture::Wheel)) {
            return;
        }
        self.move_pull(self.raw + dy, cx);
        if matches!(event.touch_phase, TouchPhase::Ended | TouchPhase::Cancelled) || self.raw == 0.
        {
            self.release_pull(event.touch_phase == TouchPhase::Cancelled, w, cx);
        } else {
            self.wheel_release.take();
            let timer = cx.background_executor().timer(WHEEL_RELEASE);
            self.wheel_release = Some(cx.spawn_in(w, async move |weak, cx| {
                timer.await;
                let _ = cx.update(|w, cx| {
                    let _ = weak.update(cx, |this, cx| this.release_pull(false, w, cx));
                });
            }));
        }
        cx.stop_propagation();
    }

    fn input_layer(&self, cx: &Context<Self>) -> impl IntoElement {
        let handle = self.scroll.clone();
        let can_pull = !self.is_refreshing();
        let report = self.scroll_region.clone();
        let down = cx.listener(|s, e: &MouseDownEvent, w, _| {
            if e.button == MouseButton::Left
                && !w.last_input_was_touch()
                && !s.is_refreshing()
                && s.gesture.is_none()
            {
                s.drag = Some(Drag {
                    origin: e.position,
                    scroll_y: s.scroll.offset().y,
                    intent: DragIntent::Pending,
                });
            }
        });
        let movement = cx.listener(Self::mouse_move);
        let release = cx.listener(Self::mouse_up);
        let wheel = cx.listener(Self::wheel);
        canvas(
            |_, _, _| {},
            move |bounds, _, w, cx| {
                if let Some(report) = &report {
                    let offset = -handle.offset().y;
                    report(
                        bounds,
                        offset > px(0.5) || can_pull,
                        offset + px(0.5) < handle.max_offset().y,
                        cx,
                    );
                }
                w.on_mouse_event(move |e: &MouseDownEvent, phase: DispatchPhase, w, cx| {
                    if phase.capture() && bounds.contains(&e.position) {
                        down(e, w, cx);
                    }
                });
                w.on_mouse_event(move |e: &MouseMoveEvent, phase: DispatchPhase, w, cx| {
                    if phase.capture() {
                        movement(e, w, cx);
                    }
                });
                w.on_mouse_event(move |e: &MouseUpEvent, phase: DispatchPhase, w, cx| {
                    if phase.capture() {
                        release(e, w, cx);
                    }
                });
                w.on_mouse_event(move |e: &ScrollEvent, phase: DispatchPhase, w, cx| {
                    if phase.capture() && bounds.contains(&e.position) {
                        wheel(e, w, cx);
                    }
                });
            },
        )
        .absolute()
        .inset_0()
    }
}

fn keyed<T>(items: Vec<T>, key: &dyn Fn(&T) -> SharedString) -> Vec<(SharedString, T)> {
    let mut seen = HashSet::new();
    items
        .into_iter()
        .filter_map(|item| {
            let key = key(&item);
            seen.insert(key.clone()).then_some((key, item))
        })
        .collect()
}

fn resisted(raw: f32, threshold: f32) -> f32 {
    let max = threshold * 1.8;
    max * (1. - (-raw * 0.70 / max).exp())
}

fn refresh_mark(ink: Hsla, turns: f32) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |bounds, _, w, _| {
            let (sin, cos) = (turns * std::f32::consts::TAU).sin_cos();
            let rotate = |x: f32, y: f32| {
                bounds.center() + point(px(x * cos - y * sin), px(x * sin + y * cos))
            };
            let arc = |i: usize| {
                let angle = (12. + i as f32 * 310. / 32.).to_radians();
                rotate(5.7 * angle.cos(), 5.7 * angle.sin())
            };
            let mut path = PathBuilder::stroke(px(1.45));
            path.move_to(arc(0));
            for i in 1..=32 {
                path.line_to(arc(i));
            }
            path.line_to(rotate(5.7, -3.1));
            path.move_to(rotate(1.8, -3.1));
            path.line_to(rotate(5.7, -3.1));
            path.line_to(rotate(5.7, -7.0));
            if let Ok(path) = path.build() {
                w.paint_path(path, ink);
            }
        },
    )
    .size(px(18.))
}

fn ellipsis() -> Div {
    div()
        .min_w_0()
        .overflow_hidden()
        .whitespace_nowrap()
        .text_ellipsis()
}

struct Motion {
    distance: f32,
    subtitle: f32,
    rows: f32,
    ticker: f32,
    notice_alpha: f32,
    reduced: bool,
}

impl<T: 'static> StretchRefresh<T> {
    fn motion(&mut self, w: &Window, cx: &Context<Self>) -> Motion {
        let now = cx.background_executor().now();
        let reduced = disabled_motion(cx);
        let (distance, moving) = self.distance.sample_batch(now, reduced);
        let (subtitle, swapping) = self.subtitle_motion.sample_batch(now, reduced);
        let (rows, inserting) = self.row_motion.sample_batch(now, reduced);
        let age = self.status_clock.age(cx);
        let closing = matches!(self.phase, Phase::Result { closing: true, .. });
        let fade = (age / 0.13).clamp(0., 1.);
        if closing && if reduced { fade >= 1. } else { !moving } {
            self.phase = Phase::Idle;
        }
        let active = if reduced {
            fade < 1. && matches!(self.phase, Phase::Refreshing | Phase::Result { .. })
        } else {
            moving || swapping || inserting || self.is_refreshing()
        };
        let animate =
            active && w.is_visible() && self.visibility.as_ref().is_none_or(|visible| visible(cx));
        if animate {
            let weak = cx.weak_entity();
            w.on_next_frame(move |_, cx| {
                let _ = weak.update(cx, |_, cx| cx.notify());
            });
        }
        Motion {
            distance: if reduced { 0. } else { distance.max(0.) },
            subtitle: subtitle.clamp(0., 1.),
            rows: rows.clamp(0., 1.),
            ticker: if reduced { 0. } else { age.rem_euclid(2.) },
            notice_alpha: if closing { 1. - fade } else { fade },
            reduced,
        }
    }

    fn header(&self, p: Paint, radius: Pixels, motion: &Motion, cx: &Context<Self>) -> Div {
        let busy = self.is_refreshing();
        let lang = UiLanguage::current(cx);
        let refresh = self.label(2, lang);
        let button = face(
            "stretch-refresh-action",
            refresh,
            36.,
            f32::from(radius),
            Tone::Surface,
            cx,
        )
        .debug_selector(|| "stretch-refresh-action".into())
        .w(px(36.))
        .border_1()
        .border_color(p.line)
        .hover(move |e| e.bg(p.hover))
        .when(busy, |e| {
            e.aria_description(lang.text("Refreshing", "正在刷新"))
        })
        .on_click(cx.listener(|s, _, w, cx| s.refresh(w, cx)))
        .child(refresh_mark(
            p.secondary,
            if busy { motion.ticker * 1.5 } else { 0. },
        ));
        div()
            .h(px(65.))
            .flex_none()
            .flex()
            .items_center()
            .justify_between()
            .gap(px(12.))
            .pl(px(20.))
            .pr(px(16.))
            .bg(p.panel)
            .border_b_1()
            .border_color(p.line)
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .child(
                        ellipsis()
                            .id("stretch-refresh-title")
                            .role(Role::Heading)
                            .aria_level(3)
                            .font_weight(FontWeight::MEDIUM)
                            .text_size(px(16.))
                            .line_height(px(19.))
                            .child(self.title.clone()),
                    )
                    .when(!self.subtitle.is_empty() || motion.subtitle < 0.999, |e| {
                        let t = motion.subtitle;
                        e.child(
                            div()
                                .relative()
                                .h(px(16.))
                                .overflow_hidden()
                                .text_size(px(12.))
                                .line_height(px(16.))
                                .text_color(p.faint)
                                .when(t < 0.999 && !self.previous_subtitle.is_empty(), |e| {
                                    e.child(
                                        ellipsis()
                                            .absolute()
                                            .left_0()
                                            .right_0()
                                            .top(px(-7. * t))
                                            .opacity(1. - t)
                                            .child(self.previous_subtitle.clone()),
                                    )
                                })
                                .child(
                                    ellipsis()
                                        .absolute()
                                        .left_0()
                                        .right_0()
                                        .top(px(7. * (1. - t)))
                                        .opacity(t)
                                        .child(self.subtitle.clone()),
                                ),
                        )
                    }),
            )
            .child(button)
    }

    fn status(&self, lang: UiLanguage) -> SharedString {
        match &self.phase {
            Phase::Refreshing => lang.text("Refreshing", "正在刷新").into(),
            Phase::Result { text, .. } => text.clone(),
            Phase::Pulling if resisted(self.raw, self.threshold) >= self.threshold => {
                lang.text("Release to refresh", "松手刷新").into()
            }
            Phase::Idle | Phase::Pulling => lang.text("Pull to refresh", "下拉刷新").into(),
        }
    }

    fn indicator(&self, p: Paint, motion: &Motion, lang: UiLanguage) -> Div {
        let busy = self.is_refreshing();
        let progress = (resisted(self.raw, self.threshold) / self.threshold).clamp(0., 1.);
        let failed = matches!(self.phase, Phase::Result { failed: true, .. });
        let results = matches!(self.phase, Phase::Result { .. });
        let visible = if motion.reduced {
            busy || results
        } else {
            motion.distance > 10.
        };
        div()
            .absolute()
            .left_0()
            .right_0()
            .top_0()
            .h(px(if motion.reduced { 34. } else { motion.distance }))
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(5.))
            .when(motion.reduced, |e| e.bg(p.stage))
            .opacity(if visible {
                if motion.reduced {
                    motion.notice_alpha
                } else {
                    (motion.distance / 32.).clamp(0., 1.)
                }
            } else {
                0.
            })
            .when(!results, |e| {
                let part = if busy { 0.40 } else { progress };
                let left = if busy {
                    (1. - (2. * motion.ticker.fract() - 1.).abs()) * (1. - part)
                } else {
                    (1. - part) * 0.5
                };
                e.child(
                    div()
                        .relative()
                        .w(px(72.))
                        .h(px(2.))
                        .rounded_full()
                        .bg(p.line.blend(p.faint.opacity(0.25)))
                        .child(
                            div()
                                .absolute()
                                .top_0()
                                .left(relative(left))
                                .w(relative(part))
                                .h_full()
                                .rounded_full()
                                .bg(p.secondary),
                        ),
                )
            })
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .text_size(px(12.))
                    .line_height(px(16.))
                    .text_color(if failed { p.danger } else { p.secondary })
                    .when(results, |e| {
                        e.child(icon(
                            if failed { Glyph::Alert } else { Glyph::Check },
                            12.,
                            if failed { p.danger } else { p.secondary },
                        ))
                    })
                    .child(self.status(lang)),
            )
    }

    fn list(
        &self,
        p: Paint,
        motion: &Motion,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let pulling = matches!(self.phase, Phase::Pulling);
        let row_spread = if pulling {
            (motion.distance / self.threshold).min(1.4) * 6.
        } else {
            0.
        };
        let mut list = div()
            .id("stretch-refresh-items")
            .role(Role::List)
            .w_full()
            .flex()
            .flex_col()
            .pt(px(4.))
            .pb(px(12.))
            .gap(px(row_spread))
            .text_color(p.ink);
        for (key, item) in &self.rows {
            let rendered = (self.render_item)(item, w, cx);
            let row = div()
                .id(ElementId::Name(key.clone()))
                .role(Role::ListItem)
                .w_full()
                .flex_none()
                .child(rendered);
            if motion.rows >= 0.999 || !self.new_rows.contains(key) {
                list = list.child(row);
                continue;
            }
            let sizes = self.row_heights.clone();
            let natural = sizes.borrow().get(key).copied().unwrap_or_default();
            let key = key.clone();
            let fraction = motion.rows;
            list = list.child(
                div()
                    .w_full()
                    .flex_none()
                    .h(natural * fraction)
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .on_children_prepainted(move |children, w, _| {
                        if let Some(row) = children.first() {
                            let mut sizes = sizes.borrow_mut();
                            if sizes.get(&key) != Some(&row.size.height) {
                                sizes.insert(key.clone(), row.size.height);
                                w.request_animation_frame();
                            }
                        }
                    })
                    .child(
                        row.relative()
                            .top(px(-6. * (1. - fraction)))
                            .opacity(fraction),
                    ),
            );
        }
        list
    }

    fn scroll_keys(&mut self, e: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        if e.keystroke.key == "escape" && self.gesture.is_some() {
            self.drag = None;
            self.release_pull(true, w, cx);
            cx.stop_propagation();
            return;
        }
        let current = self.scroll.offset().y;
        let max = self.scroll.max_offset().y;
        let step = self.scroll.bounds().size.height * 0.9;
        let next = match e.keystroke.key.as_str() {
            "home" => px(0.),
            "end" => -max,
            "pageup" => current + step,
            "pagedown" => current - step,
            "up" => current + px(30.),
            "down" => current - px(30.),
            _ => return,
        }
        .clamp(-max, px(0.));
        self.scroll.set_offset(point(px(0.), next));
        cx.stop_propagation();
        cx.notify();
    }
}

impl<T: 'static> Render for StretchRefresh<T> {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let radius = Theme::global(cx).tokens.radius.md;
        let lang = UiLanguage::current(cx);
        let motion = self.motion(w, cx);
        let reduced_notice =
            motion.reduced && matches!(self.phase, Phase::Refreshing | Phase::Result { .. });
        let notice = match &self.phase {
            Phase::Refreshing | Phase::Result { .. } => self.status(lang),
            _ => "".into(),
        };
        let content = self.list(p, &motion, w, cx);
        let viewport = div()
            .id("stretch-refresh-viewport")
            .debug_selector(|| "stretch-refresh-viewport".into())
            .role(Role::Region)
            .aria_label(self.title.clone())
            .track_focus(&self.region_focus)
            .tab_index(0)
            .track_scroll(&self.scroll)
            .on_scroll(cx.listener(|_, _, _, cx| cx.notify()))
            .on_key_down(cx.listener(Self::scroll_keys))
            .absolute()
            .left_0()
            .right_0()
            .top(px(motion.distance))
            .bottom_0()
            .when(motion.distance > 0.5, |e| e.rounded_t(radius))
            .bg(p.panel)
            .overflow_y_scroll()
            .focus_visible(move |s| s.border_1().border_color(p.focus))
            .child(content);
        let indicator = self.indicator(p, &motion, lang);
        let panel = div()
            .size_full()
            .min_h_0()
            .flex()
            .flex_col()
            .rounded(radius)
            .border_1()
            .border_color(p.line)
            .bg(p.stage)
            .text_color(p.ink)
            .overflow_hidden();
        let panel = if let Some(style) = &self.style {
            style(panel)
        } else {
            panel
        };
        panel
            .id(self.id.clone())
            .role(Role::Group)
            .aria_label(self.title.clone())
            .child(self.header(p, radius, &motion, cx))
            .child(
                div()
                    .relative()
                    .w_full()
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .when(!reduced_notice, |e| e.child(indicator))
                    .child(viewport)
                    .when(reduced_notice, |e| {
                        e.child(self.indicator(p, &motion, lang))
                    })
                    .child(self.input_layer(cx)),
            )
            .child(
                div()
                    .id("stretch-refresh-announcement")
                    .role(Role::Status)
                    .aria_label(notice)
                    .size(px(0.)),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::{Phase, StretchRefresh, StretchRefreshError};
    use gpui::{
        KeyUpEvent, Keystroke, Modifiers, MouseButton, MouseDownEvent, MouseMoveEvent,
        MouseUpEvent, ScrollDelta, ScrollEvent, ScrollSource, SharedString, Task, TestAppContext,
        TouchPhase, div, point, prelude::*, px,
    };
    use std::{cell::Cell, rc::Rc, time::Duration};

    #[test]
    fn short_mouse_pull_and_scroll_inside_the_feed_do_not_refresh() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let calls = Rc::new(Cell::new(0));
        let (_, w) = app.add_window_view(|_, cx| {
            let calls = calls.clone();
            StretchRefresh::new(
                "Activity",
                (0..14).collect(),
                |i: &usize| SharedString::from(i.to_string()),
                |_, _, _| div().h(px(42.)).into_any_element(),
                move |_, _| {
                    calls.set(calls.get() + 1);
                    Task::ready(Ok(None))
                },
                cx,
            )
            .with_style(|e| e.w(px(360.)).h(px(300.)))
        });
        let viewport = w.debug_bounds("stretch-refresh-viewport").unwrap();
        let origin = point(viewport.center().x, viewport.top() + px(24.));
        for (distance, expected) in [(40., 0), (248., 1)] {
            let end = origin + point(px(0.), px(distance));
            w.simulate_event(MouseDownEvent {
                button: MouseButton::Left,
                position: origin,
                modifiers: Modifiers::default(),
                click_count: 1,
                first_mouse: false,
            });
            w.simulate_event(MouseMoveEvent {
                position: end,
                pressed_button: Some(MouseButton::Left),
                modifiers: Modifiers::default(),
            });
            w.simulate_event(MouseUpEvent {
                button: MouseButton::Left,
                position: end,
                modifiers: Modifiers::default(),
                click_count: 1,
            });
            assert_eq!(calls.get(), expected);
            w.executor().advance_clock(Duration::from_millis(1600));
            w.run_until_parked();
        }
    }

    #[test]
    fn touch_cancellation_and_a_wheel_at_the_top_use_distinct_release_paths() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let calls = Rc::new(Cell::new(0));
        let (view, w) = app.add_window_view(|_, cx| {
            let calls = calls.clone();
            StretchRefresh::new(
                "Activity",
                (0..14).collect(),
                |i: &usize| i.to_string().into(),
                |_, _, _| div().h(px(42.)).into_any_element(),
                move |_, _| {
                    calls.set(calls.get() + 1);
                    Task::ready(Ok(None))
                },
                cx,
            )
            .with_style(|e| e.w(px(360.)).h(px(300.)))
        });
        let position = w.debug_bounds("stretch-refresh-viewport").unwrap().center();
        let event = |dy, touch_phase, source| ScrollEvent {
            position,
            delta: ScrollDelta::Pixels(point(px(0.), px(dy))),
            modifiers: Modifiers::default(),
            touch_phase,
            source,
        };
        w.simulate_event(event(-155., TouchPhase::Moved, ScrollSource::Pointer));
        assert!(w.update(|_, cx| view.read(cx).scroll.offset().y < px(0.)));
        w.simulate_event(event(205., TouchPhase::Moved, ScrollSource::Pointer));
        assert!(w.update(|_, cx| view.read(cx).at_top()));
        assert_eq!(calls.get(), 0);
        w.simulate_event(event(235., TouchPhase::Started, ScrollSource::Touch));
        w.simulate_event(event(0., TouchPhase::Cancelled, ScrollSource::Touch));
        assert_eq!(calls.get(), 0);
        w.executor().advance_clock(Duration::from_millis(250));
        w.simulate_event(event(235., TouchPhase::Moved, ScrollSource::Pointer));
        assert_eq!(calls.get(), 0);
        w.executor().advance_clock(Duration::from_millis(145));
        w.run_until_parked();
        assert_eq!(calls.get(), 1);
    }

    #[test]
    fn in_flight_activation_is_coalesced_and_error_allows_retry() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let calls = Rc::new(Cell::new(0));
        let (view, w) = app.add_window_view(|_, cx| {
            let calls = calls.clone();
            StretchRefresh::new(
                "Activity",
                vec!["first"],
                |item| (*item).into(),
                |_, _, _| div().h(px(30.)).into_any_element(),
                move |_, cx| {
                    let attempt = calls.get();
                    calls.set(attempt + 1);
                    let timer = cx.background_executor().timer(Duration::from_millis(350));
                    cx.spawn(async move |_, _| {
                        timer.await;
                        if attempt == 0 {
                            Err(StretchRefreshError("offline".into()))
                        } else {
                            Ok(Some("Two added".into()))
                        }
                    })
                },
                cx,
            )
            .with_error_label("Offline, try again")
            .with_style(|e| e.w(px(360.)).h(px(300.)))
        });
        let refresh = w.debug_bounds("stretch-refresh-action").unwrap().center();
        w.simulate_click(refresh, Modifiers::default());
        w.simulate_click(refresh, Modifiers::default());
        assert_eq!(calls.get(), 1);
        w.executor().advance_clock(Duration::from_millis(360));
        w.run_until_parked();
        assert!(w.update(|_, cx| matches!(&view.read(cx).phase, Phase::Result { text, failed: true, .. } if text.as_ref() == "Offline, try again")));
        w.simulate_keystrokes("space");
        w.simulate_event(KeyUpEvent {
            keystroke: Keystroke::parse("space").unwrap(),
        });
        assert_eq!(calls.get(), 2);
        w.executor().advance_clock(Duration::from_millis(360));
        w.run_until_parked();
        assert!(w.update(|_, cx| matches!(&view.read(cx).phase, Phase::Result { text, failed: false, .. } if text.as_ref() == "Two added")));
    }
}
