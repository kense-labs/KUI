//! Recent, host-supplied activity in a pausable notification stack.
use crate::{
    foundation::surface_radius,
    primitives::{
        family::{Paint, Slide, Tone, disabled_motion, ease, face},
        glyph::{Glyph, icon},
        presentation::{column, row, tr},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc, time::Duration};
use web_time::{Instant, SystemTime, UNIX_EPOCH};

const WIDTH: f32 = 360.;
const HEIGHT: f32 = 64.;
const PEEK: f32 = 10.;
const GAP: f32 = 8.;
const MOVE: f32 = 0.24;
const FADE: f32 = 0.16;

/// A display string or an epoch timestamp in milliseconds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SocialProofTime {
    Text(SharedString),
    TimestampMillis(i64),
}

impl From<SystemTime> for SocialProofTime {
    fn from(time: SystemTime) -> Self {
        let millis = |d: Duration| d.as_millis().min(i64::MAX as u128) as i64;
        Self::TimestampMillis(match time.duration_since(UNIX_EPOCH) {
            Ok(duration) => millis(duration),
            Err(error) => -millis(error.duration()),
        })
    }
}

/// The verification label beside an event's metadata.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum SocialProofVerification {
    #[default]
    None,
    Verified,
    Label(SharedString),
}

/// One activity record. The feed is supplied oldest first.
#[derive(Clone)]
pub struct SocialProofEvent {
    pub id: SharedString,
    pub name: SharedString,
    pub action: SharedString,
    pub meta: Option<SharedString>,
    pub time: Option<SocialProofTime>,
    pub avatar: Option<ImageSource>,
    pub logo: Option<AnyView>,
    pub verified: SocialProofVerification,
    accent: Option<[u32; 2]>,
}

impl SocialProofEvent {
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        action: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            action: action.into(),
            meta: None,
            time: None,
            avatar: None,
            logo: None,
            verified: SocialProofVerification::None,
            accent: None,
        }
    }

    pub fn with_meta(mut self, meta: impl Into<SharedString>) -> Self {
        self.meta = Some(meta.into());
        self
    }

    pub fn with_time(mut self, time: impl Into<SocialProofTime>) -> Self {
        self.time = Some(time.into());
        self
    }

    pub fn with_avatar(mut self, image: impl Into<ImageSource>) -> Self {
        self.avatar = Some(image.into());
        self
    }

    /// A decorative monochrome mark used when no avatar is provided.
    pub fn with_logo(mut self, view: impl Into<AnyView>) -> Self {
        self.logo = Some(view.into());
        self
    }

    pub fn with_verification(mut self, verified: SocialProofVerification) -> Self {
        self.verified = verified;
        self
    }

    fn sentence(&self) -> SharedString {
        format!("{} {}", self.name, self.action).into()
    }
}

/// Inline placement or a viewport corner.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SocialProofPosition {
    #[default]
    Inline,
    BottomLeft,
    BottomRight,
    TopLeft,
    TopRight,
}

impl SocialProofPosition {
    fn top(self) -> bool {
        matches!(self, Self::TopLeft | Self::TopRight)
    }

    fn right(self) -> bool {
        matches!(self, Self::TopRight | Self::BottomRight)
    }
}

/// Cycle and placement policy.
#[derive(Clone)]
pub struct SocialProofOptions {
    pub events: Option<Vec<SocialProofEvent>>,
    pub interval: Duration,
    pub initial_delay: Duration,
    pub max_visible: usize,
    pub prefill: usize,
    pub position: SocialProofPosition,
    pub expand_on_hover: bool,
    pub looping: bool,
    pub limit: Option<usize>,
    pub paused: bool,
    pub label: Option<SharedString>,
}

impl Default for SocialProofOptions {
    fn default() -> Self {
        Self {
            events: None,
            interval: Duration::from_secs(4),
            initial_delay: Duration::from_millis(800),
            max_visible: 3,
            prefill: 0,
            position: SocialProofPosition::Inline,
            expand_on_hover: true,
            looping: true,
            limit: None,
            paused: false,
            label: None,
        }
    }
}

#[derive(Clone, Copy)]
struct Fade {
    from: f32,
    to: f32,
    start: Option<Instant>,
}

impl Fade {
    fn new(value: f32) -> Self {
        Self {
            from: value,
            to: value,
            start: None,
        }
    }

    fn set(&mut self, to: f32, now: Instant) {
        if to != self.to {
            self.from = self.sample(now).0;
            self.to = to;
            self.start = Some(now);
        }
    }

    fn sample(self, now: Instant) -> (f32, bool) {
        let Some(start) = self.start else {
            return (self.to, false);
        };
        let t = now.saturating_duration_since(start).as_secs_f32() / FADE;
        (self.from + (self.to - self.from) * ease(t), t < 1.)
    }
}

struct ToastSlot {
    serial: u64,
    event: SocialProofEvent,
    close: FocusHandle,
    select: FocusHandle,
    depth: Slide,
    travel: Slide,
    fade: Fade,
    content: Fade,
}

impl ToastSlot {
    fn seeded(serial: u64, event: SocialProofEvent, depth: f32, cx: &App) -> Self {
        Self {
            serial,
            event,
            close: cx.focus_handle(),
            select: cx.focus_handle(),
            depth: Slide::new(depth),
            travel: Slide::new(0.),
            fade: Fade::new(1.),
            content: Fade::new(if depth == 0. { 1. } else { 0. }),
        }
    }
}

struct Leaving {
    event: SocialProofEvent,
    depth: f32,
    travel: f32,
    fade: Fade,
    content: f32,
    start: Instant,
}

type EventHandler = Box<dyn Fn(&SocialProofEvent, &mut App)>;
type Visibility = Box<dyn Fn(&App) -> bool>;
type Style = Box<dyn Fn(Div) -> Div>;

/// A feed that owns one cycle timer and at most one departing toast.
pub struct SocialProofToast {
    id: ElementId,
    options: SocialProofOptions,
    events: Vec<SocialProofEvent>,
    slots: Vec<ToastSlot>,
    leaving: Option<Leaving>,
    cursor: usize,
    serial: u64,
    shown: usize,
    seed_notified: bool,
    timer: Option<Task<()>>,
    deadline: Option<Instant>,
    remaining: Duration,
    manually_visible: bool,
    visible: bool,
    geometry_visible: Rc<Cell<bool>>,
    stack_bounds: Rc<Cell<Bounds<Pixels>>>,
    hit_cards: Vec<(u64, Bounds<Pixels>)>,
    visibility: Option<Visibility>,
    hovered: bool,
    hovered_slot: Option<u64>,
    focused: bool,
    expanded: Slide,
    scope: FocusHandle,
    _focus: [Subscription; 2],
    announcement: SharedString,
    touch: bool,
    style: Option<Style>,
    on_show: Option<EventHandler>,
    on_dismiss: Option<EventHandler>,
    on_select: Option<EventHandler>,
}

impl SocialProofToast {
    pub fn new(mut options: SocialProofOptions, w: &mut Window, cx: &mut Context<Self>) -> Self {
        options.max_visible = options.max_visible.max(1);
        options.interval = options.interval.max(Duration::from_millis(16));
        let events = options.events.take().unwrap_or_else(|| sample(cx));
        let count = options
            .prefill
            .min(options.max_visible)
            .min(events.len())
            .min(options.limit.unwrap_or(usize::MAX));
        let mut slots = Vec::with_capacity(count);
        for (depth, event) in events[..count].iter().rev().enumerate() {
            slots.push(ToastSlot::seeded(
                (count - depth) as u64,
                event.clone(),
                depth as f32,
                cx,
            ));
        }
        let scope = cx.focus_handle();
        let enter = cx.on_focus_in(&scope, w, |s, _, cx| {
            s.focused = true;
            s.update_activity(cx);
        });
        let leave = cx.on_focus_out(&scope, w, |s, _, _, cx| {
            s.focused = false;
            s.update_activity(cx);
        });
        let remaining = options.initial_delay;
        Self {
            id: "social-proof-toast".into(),
            options,
            events,
            slots,
            leaving: None,
            cursor: count,
            serial: count as u64,
            shown: count,
            seed_notified: false,
            timer: None,
            deadline: None,
            remaining,
            manually_visible: true,
            visible: false,
            geometry_visible: Rc::default(),
            stack_bounds: Rc::default(),
            hit_cards: Vec::new(),
            visibility: None,
            hovered: false,
            hovered_slot: None,
            focused: false,
            expanded: Slide::new(0.),
            scope,
            _focus: [enter, leave],
            announcement: "".into(),
            touch: touch_pointer(),
            style: None,
            on_show: None,
            on_dismiss: None,
            on_select: None,
        }
    }

    pub fn with_id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = id.into();
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

    /// Shows close buttons without hovering on touch-first hosts.
    pub fn with_touch_input(mut self, touch: bool) -> Self {
        self.touch = touch;
        self
    }

    pub fn on_show(mut self, handler: impl Fn(&SocialProofEvent, &mut App) + 'static) -> Self {
        self.on_show = Some(Box::new(handler));
        self
    }

    pub fn on_dismiss(mut self, handler: impl Fn(&SocialProofEvent, &mut App) + 'static) -> Self {
        self.on_dismiss = Some(Box::new(handler));
        self
    }

    pub fn on_select(mut self, handler: impl Fn(&SocialProofEvent, &mut App) + 'static) -> Self {
        self.on_select = Some(Box::new(handler));
        self
    }

    /// Updates the feed and keeps the cursor after the last consumed record when possible.
    pub fn set_events(&mut self, events: Vec<SocialProofEvent>, cx: &mut Context<Self>) {
        let last = self.cursor.checked_sub(1).and_then(|i| self.events.get(i));
        self.cursor = last
            .and_then(|event| events.iter().position(|next| next.id == event.id))
            .map_or(0, |i| i + 1);
        for slot in &mut self.slots {
            if let Some(update) = events.iter().find(|next| next.id == slot.event.id) {
                slot.event = update.clone();
            }
        }
        self.events = events;
        self.sync_timer(cx);
        cx.notify();
    }

    /// Pauses arrival without consuming the remaining interval.
    pub fn set_paused(&mut self, paused: bool, cx: &mut Context<Self>) {
        if self.options.paused != paused {
            self.options.paused = paused;
            self.sync_timer(cx);
            cx.notify();
        }
    }

    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        if self.manually_visible != visible {
            self.manually_visible = visible;
            if !visible {
                self.visible = false;
                self.sync_timer(cx);
            }
            cx.notify();
        }
    }

    pub fn visible_events(&self) -> impl Iterator<Item = &SocialProofEvent> {
        self.slots
            .iter()
            .take(self.options.max_visible)
            .map(|slot| &slot.event)
    }

    fn has_next(&self) -> bool {
        !self.events.is_empty()
            && (self.cursor < self.events.len() || self.options.looping)
            && self.options.limit.is_none_or(|limit| self.shown < limit)
    }

    fn expanded_now(&self) -> bool {
        self.options.expand_on_hover && (self.hovered || self.focused)
    }

    fn update_activity(&mut self, cx: &mut Context<Self>) {
        self.expanded
            .duration(if self.expanded_now() { 1. } else { 0. }, MOVE, cx);
        self.retarget(cx);
        self.sync_timer(cx);
        cx.notify();
    }

    fn sync_timer(&mut self, cx: &mut Context<Self>) {
        let now = cx.background_executor().now();
        let active = self.visible
            && !self.options.paused
            && !self.hovered
            && !self.focused
            && self.has_next();
        if !active {
            if let Some(deadline) = self.deadline.take() {
                self.remaining = deadline.saturating_duration_since(now);
            }
            self.timer = None;
            return;
        }
        if self.timer.is_none() {
            let wait = cx.background_executor().timer(self.remaining);
            self.deadline = Some(now + self.remaining);
            self.timer = Some(cx.spawn(async move |weak, cx| {
                wait.await;
                let _ = weak.update(cx, |s, cx| {
                    s.timer = None;
                    s.deadline = None;
                    if !s.geometry_visible.get()
                        || s.visibility.as_ref().is_some_and(|visible| !visible(cx))
                    {
                        s.visible = false;
                        s.remaining = Duration::ZERO;
                        return;
                    }
                    if s.visible && !s.options.paused && !s.hovered && !s.focused && s.has_next() {
                        s.advance(cx);
                    }
                    s.remaining = s.options.interval;
                    s.sync_timer(cx);
                });
            }));
        }
    }

    fn retarget(&mut self, cx: &App) {
        let now = cx.background_executor().now();
        let expanded = self.expanded_now();
        for (depth, slot) in self.slots.iter_mut().enumerate() {
            slot.depth.duration(depth as f32, MOVE, cx);
            slot.fade.set(
                if depth < self.options.max_visible {
                    1.
                } else {
                    0.
                },
                now,
            );
            slot.content.set(
                if depth == 0 || expanded && depth < self.options.max_visible {
                    1.
                } else {
                    0.
                },
                now,
            );
        }
    }

    fn advance(&mut self, cx: &mut Context<Self>) {
        if !self.has_next() {
            return;
        }
        if self.cursor >= self.events.len() {
            self.cursor = 0;
        }
        let event = self.events[self.cursor].clone();
        if self
            .leaving
            .as_ref()
            .is_some_and(|slot| slot.event.id == event.id)
        {
            self.leaving = None;
        }
        self.cursor += 1;
        self.shown = self.shown.saturating_add(1);
        let mut slot =
            if let Some(index) = self.slots.iter().position(|old| old.event.id == event.id) {
                let mut slot = self.slots.remove(index);
                slot.event = event.clone();
                slot
            } else {
                self.serial = self.serial.wrapping_add(1);
                let mut slot = ToastSlot::seeded(self.serial, event.clone(), 0., cx);
                slot.fade = Fade::new(0.);
                if !disabled_motion(cx) {
                    slot.travel.reset(16.);
                    slot.travel.duration(0., MOVE, cx);
                }
                slot
            };
        slot.fade.set(1., cx.background_executor().now());
        self.slots.insert(0, slot);
        self.slots
            .truncate(self.options.max_visible.saturating_add(1));
        self.retarget(cx);
        if let Some(handler) = &self.on_show {
            handler(&event, cx);
        }
        cx.notify();
    }

    fn dismiss(&mut self, serial: u64, w: &mut Window, cx: &mut Context<Self>) {
        let Some(index) = self
            .slots
            .iter()
            .take(self.options.max_visible)
            .position(|slot| slot.serial == serial)
        else {
            return;
        };
        let slot = self.slots.remove(index);
        let move_focus =
            self.scope.is_focused(w) || slot.close.is_focused(w) || slot.select.is_focused(w);
        let now = cx.background_executor().now();
        let mut fade = Fade::new(slot.fade.sample(now).0);
        fade.set(0., now);
        self.leaving = Some(Leaving {
            event: slot.event.clone(),
            depth: slot.depth.value(cx),
            travel: slot.travel.value(cx),
            fade,
            content: slot.content.sample(now).0,
            start: now,
        });
        if self.hovered_slot == Some(serial) {
            self.hovered_slot = None;
        }
        self.retarget(cx);
        self.announcement = format!(
            "{}: {}",
            tr(cx, "Dismissed", "已关闭"),
            slot.event.sentence()
        )
        .into();
        if let Some(handler) = &self.on_dismiss {
            handler(&slot.event, cx);
        }
        if move_focus {
            let visible = self.slots.len().min(self.options.max_visible);
            if visible == 0 {
                self.scope.focus(w, cx);
            } else {
                self.slots[index.min(visible - 1)].close.focus(w, cx);
            }
        }
        cx.notify();
    }

    fn select(&mut self, serial: u64, cx: &mut Context<Self>) {
        if let Some(slot) = self
            .slots
            .iter()
            .take(self.options.max_visible)
            .find(|slot| slot.serial == serial)
            && let Some(handler) = &self.on_select
        {
            handler(&slot.event, cx);
        }
    }
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(
    inline_js = "export function kuiSocialProofTouch(){return globalThis.matchMedia?.('(hover: none)').matches ?? false;}"
)]
extern "C" {
    #[wasm_bindgen::prelude::wasm_bindgen(js_name = kuiSocialProofTouch)]
    fn touch_pointer() -> bool;
}

#[cfg(not(target_family = "wasm"))]
fn touch_pointer() -> bool {
    false
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| {
            duration.as_millis().min(i64::MAX as u128) as i64
        })
}

fn timestamp(time: &SocialProofTime, cx: &App) -> SharedString {
    match time {
        SocialProofTime::Text(label) => label.clone(),
        SocialProofTime::TimestampMillis(millis) => {
            let elapsed = now_millis().saturating_sub(*millis).max(0) / 1000;
            let zh = tr(cx, "en", "zh") == "zh";
            if elapsed < 60 {
                return tr(cx, "Just now", "刚刚").into();
            }
            let (value, unit) = if elapsed < 3600 {
                (elapsed / 60, tr(cx, "min", "分钟"))
            } else if elapsed < 86400 {
                (elapsed / 3600, tr(cx, "h", "小时"))
            } else {
                (elapsed / 86400, tr(cx, "d", "天"))
            };
            if zh {
                format!("{value} {unit}前")
            } else {
                format!("{value} {unit} ago")
            }
            .into()
        }
    }
}

fn sample(cx: &App) -> Vec<SocialProofEvent> {
    let data = [
        (
            "alba",
            "Alba",
            "阿尔芭",
            "started a Pro trial",
            "开始试用专业版",
            "Lisbon",
            "里斯本",
            2,
            false,
            Some([0x67e8b2, 0xf86b9e]),
        ),
        (
            "northbank",
            "Northbank",
            "北岸",
            "upgraded to Business",
            "升级到商业版",
            "48 seats",
            "48 个席位",
            4,
            true,
            None,
        ),
        (
            "ari",
            "Ari",
            "阿里",
            "joined the waitlist",
            "加入等候名单",
            "Toronto",
            "多伦多",
            8,
            false,
            Some([0x70d6e6, 0xa987f4]),
        ),
        (
            "avery",
            "Avery",
            "艾弗里",
            "booked a product demo",
            "预约产品演示",
            "Melbourne",
            "墨尔本",
            14,
            false,
            Some([0xaaf16a, 0x8769db]),
        ),
        (
            "riley",
            "Riley",
            "莱利",
            "started a Pro trial",
            "开始试用专业版",
            "Dublin",
            "都柏林",
            21,
            false,
            Some([0xf39078, 0x79e6a4]),
        ),
        (
            "fieldwork",
            "Fieldwork",
            "原野",
            "upgraded to Business",
            "升级到商业版",
            "32 seats",
            "32 个席位",
            26,
            true,
            None,
        ),
        (
            "morgan",
            "Morgan",
            "摩根",
            "invited 6 teammates",
            "邀请了 6 位队友",
            "Buenos Aires",
            "布宜诺斯艾利斯",
            30,
            false,
            Some([0xea74a3, 0x8adc76]),
        ),
        (
            "parcel",
            "Parcel",
            "邮包",
            "renewed Enterprise",
            "续订企业版",
            "Annual plan",
            "年度计划",
            34,
            true,
            None,
        ),
    ];
    let now = now_millis();
    data.into_iter()
        .map(
            |(id, name, zh_name, action, zh_action, meta, zh_meta, minutes, verified, accent)| {
                let mut event =
                    SocialProofEvent::new(id, tr(cx, name, zh_name), tr(cx, action, zh_action))
                        .with_meta(tr(cx, meta, zh_meta))
                        .with_time(SocialProofTime::TimestampMillis(now - minutes * 60_000));
                event.accent = accent;
                if verified {
                    event.verified = SocialProofVerification::Verified;
                }
                event
            },
        )
        .collect()
}

#[derive(Clone, Copy)]
struct Geometry {
    top: f32,
    inset: f32,
    height: f32,
    scale: f32,
}

struct Layer {
    shape: Geometry,
    opacity: f32,
    ink: f32,
}

fn geometry(depth: f32, travel: f32, fan: f32, span: f32, top: bool, shrink: f32) -> Geometry {
    let closed = 1. - fan;
    let scale = (1. - 0.05 * depth.max(0.) * closed).max(0.35) * shrink;
    let offset = depth * (PEEK * closed + (HEIGHT + GAP) * fan);
    Geometry {
        top: if top {
            offset - travel
        } else {
            span - HEIGHT - offset + travel
        },
        inset: (1. - scale) * 0.5,
        height: HEIGHT * scale,
        scale,
    }
}

fn text(text: impl Into<SharedString>, size: f32, line: f32, ink: Hsla) -> Div {
    div()
        .min_w_0()
        .truncate()
        .text_size(px(size))
        .line_height(px(line))
        .text_color(ink)
        .child(text.into())
}

fn visual(event: &SocialProofEvent, scale: f32, p: Paint) -> Div {
    let side = px(36. * scale);
    let dot = div()
        .size(side)
        .flex_none()
        .rounded_full()
        .bg(p.stage)
        .flex()
        .items_center()
        .justify_center();
    if let Some(avatar) = &event.avatar {
        dot.child(
            img(avatar.clone())
                .size_full()
                .rounded_full()
                .object_fit(ObjectFit::Cover),
        )
    } else if let Some(logo) = &event.logo {
        dot.child(
            div()
                .size(px(18. * scale))
                .text_color(p.ink)
                .child(logo.clone()),
        )
    } else if let Some([a, b]) = event.accent {
        dot.bg(linear_gradient(
            136.,
            linear_color_stop(rgb(a), 0.),
            linear_color_stop(rgb(b), 1.),
        ))
    } else {
        let initials = event
            .name
            .split_whitespace()
            .filter_map(|part| part.chars().next())
            .take(2)
            .flat_map(char::to_uppercase)
            .collect::<String>();
        dot.border_1()
            .border_color(p.line)
            .text_size(px(13. * scale))
            .font_weight(FontWeight::MEDIUM)
            .text_color(p.ink)
            .child(initials)
    }
}

fn badge(verified: SharedString, scale: f32, p: Paint) -> Div {
    let size = 12. * scale;
    row()
        .gap(px(4. * scale))
        .child(
            div()
                .size(px(size))
                .flex_none()
                .rounded_full()
                .border_1()
                .border_color(p.secondary)
                .flex()
                .items_center()
                .justify_center()
                .child(icon(Glyph::Check, 8. * scale, p.secondary)),
        )
        .child(text(verified, 12. * scale, 16. * scale, p.secondary))
}

fn content(event: &SocialProofEvent, opacity: f32, scale: f32, p: Paint, cx: &App) -> Div {
    let secondary = event.meta.as_ref().filter(|meta| !meta.is_empty());
    let time = event
        .time
        .as_ref()
        .map(|time| timestamp(time, cx))
        .filter(|time| !time.is_empty());
    let verified = match &event.verified {
        SocialProofVerification::None => None,
        SocialProofVerification::Verified => Some(tr(cx, "Verified", "已验证").into()),
        SocialProofVerification::Label(label) => Some(label.clone()),
    };
    let sentence = event.sentence();
    let title = StyledText::new(sentence.clone()).with_highlights([
        (
            0..event.name.len(),
            HighlightStyle {
                font_weight: Some(FontWeight::MEDIUM),
                color: Some(p.ink),
                ..Default::default()
            },
        ),
        (
            event.name.len()..sentence.len(),
            HighlightStyle {
                color: Some(p.secondary),
                ..Default::default()
            },
        ),
    ]);
    let details = match (secondary, time) {
        (Some(meta), Some(time)) => format!("{meta} · {time}"),
        (Some(meta), None) => meta.to_string(),
        (None, Some(time)) => time.to_string(),
        (None, None) => String::new(),
    };
    let has_details = !details.is_empty();
    let has_verified = verified.is_some();
    let mut meta = row().gap(px(4. * scale)).overflow_hidden();
    if has_details {
        meta = meta.child(text(details, 12. * scale, 16. * scale, p.faint));
    }
    if let Some(verified) = verified {
        meta = meta
            .when(has_details, |e| {
                e.child(text("·", 12. * scale, 16. * scale, p.faint).flex_none())
            })
            .child(badge(verified, scale, p).flex_none().max_w(relative(0.48)));
    }
    row()
        .w_full()
        .h_full()
        .gap(px(12. * scale))
        .opacity(opacity)
        .child(visual(event, scale, p))
        .child(
            column()
                .flex_1()
                .min_w_0()
                .gap(px(2. * scale))
                .child(
                    div()
                        .min_w_0()
                        .truncate()
                        .text_size(px(14. * scale))
                        .line_height(px(20. * scale))
                        .child(title),
                )
                .when(has_details || has_verified, |e| e.child(meta)),
        )
}

fn shell(shape: Geometry, opacity: f32, p: Paint, cx: &App) -> Div {
    let dark = p.panel.l < 0.5;
    let base = p.panel.blend(p.ink.opacity(if dark { 0.025 } else { 0. }));
    div()
        .absolute()
        .top(px(shape.top))
        .left(relative(shape.inset))
        .right(relative(shape.inset))
        .h(px(shape.height))
        .pl(px(13. * shape.scale))
        .pr(px(43. * shape.scale))
        .rounded(surface_radius(cx) * shape.scale)
        .bg(base)
        .border_1()
        .border_color(p.line)
        .shadow(vec![
            BoxShadow::new(
                px(0.),
                px(8.),
                black().opacity(if dark { 0.25 } else { 0.12 }),
            )
            .blur_radius(px(24.))
            .spread_radius(px(-4.)),
        ])
        .opacity(opacity)
}

fn position_width(view: Size<Pixels>) -> (Pixels, Pixels) {
    if view.width < px(480.) {
        ((view.width - px(32.)).max(px(1.)), px(16.))
    } else {
        (px(WIDTH), px(24.))
    }
}

impl SocialProofToast {
    fn live_layer(
        &self,
        slot: &ToastSlot,
        layer: Layer,
        tab_order: Option<isize>,
        w: &Window,
        cx: &Context<Self>,
    ) -> Stateful<Div> {
        let Layer {
            shape,
            opacity,
            ink,
        } = layer;
        let p = Paint::get(cx);
        let serial = slot.serial;
        let close_visible = self.touch
            || self.hovered_slot == Some(serial)
            || slot.close.is_focused(w)
            || slot.select.is_focused(w);
        let mut card = shell(shape, opacity, p, cx)
            .id(("proof-card", serial))
            .debug_selector(move || format!("proof-card-{serial}"));
        let Some(tab_order) = tab_order else {
            return card.child(content(&slot.event, ink, shape.scale, p, cx));
        };
        let mut body = content(&slot.event, ink, shape.scale, p, cx).id(("proof-body", serial));
        if self.on_select.is_some() {
            body = body
                .role(Role::Button)
                .aria_label(slot.event.sentence())
                .track_focus(&slot.select)
                .tab_index(tab_order)
                .cursor_pointer()
                .on_click(cx.listener(move |s, _, _, cx| s.select(serial, cx)));
        }
        let close = face(
            ("proof-dismiss", serial),
            format!("{}: {}", tr(cx, "Dismiss", "关闭"), slot.event.sentence()),
            24.,
            12.,
            Tone::Clear,
            cx,
        )
        .debug_selector(move || format!("proof-dismiss-{serial}"))
        .track_focus(&slot.close)
        .tab_index(tab_order + 1)
        .absolute()
        .right(px(12.))
        .top(px((shape.height - 24.) * 0.5))
        .w(px(24.))
        .opacity(if close_visible { 1. } else { 0. })
        .hover(|e| e.bg(p.hover))
        .child(icon(Glyph::Close, 12., p.secondary))
        .on_click(cx.listener(move |s, _, w, cx| s.dismiss(serial, w, cx)));
        card = card.child(body).child(close);
        card
    }

    fn pointed_card(&self, position: Point<Pixels>) -> Option<u64> {
        if self.touch || !self.hovered {
            return None;
        }
        self.hit_cards
            .iter()
            .find(|(_, bounds)| bounds.contains(&position))
            .map(|(serial, _)| *serial)
    }

    fn dismiss_key(&mut self, event: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        if event.keystroke.key == "escape"
            && let Some(serial) = self
                .slots
                .iter()
                .take(self.options.max_visible)
                .find(|slot| slot.close.is_focused(w) || slot.select.is_focused(w))
                .or_else(|| self.slots.first())
                .map(|slot| slot.serial)
        {
            self.dismiss(serial, w, cx);
            cx.stop_propagation();
        }
    }

    fn announce_prefill(&mut self, w: &Window, cx: &mut Context<Self>) {
        if self.seed_notified {
            return;
        }
        self.seed_notified = true;
        if self.on_show.is_some() && !self.slots.is_empty() {
            let seeded: Vec<_> = self
                .slots
                .iter()
                .rev()
                .map(|slot| slot.event.clone())
                .collect();
            cx.defer_in(w, move |s, _, cx| {
                if let Some(handler) = &s.on_show {
                    for event in &seeded {
                        handler(event, cx);
                    }
                }
            });
        }
    }
}

impl Render for SocialProofToast {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focused = self.scope.is_focused(w)
            || self
                .slots
                .iter()
                .take(self.options.max_visible)
                .any(|slot| slot.close.is_focused(w) || slot.select.is_focused(w));
        if focused != self.focused {
            self.focused = focused;
            self.update_activity(cx);
        }
        let visible = self.manually_visible
            && self.geometry_visible.get()
            && w.is_visible()
            && self.visibility.as_ref().is_none_or(|visible| visible(cx));
        if visible != self.visible {
            self.visible = visible;
            self.sync_timer(cx);
        }
        if visible {
            self.announce_prefill(w, cx);
        }
        let p = Paint::get(cx);
        let reduced = disabled_motion(cx);
        let now = cx.background_executor().now();
        let top = self.options.position.top();
        let inline = self.options.position == SocialProofPosition::Inline;
        let base = HEIGHT + PEEK * self.options.max_visible.saturating_sub(1) as f32;
        let visible_slots = self.slots.len().min(self.options.max_visible);
        let span_open = HEIGHT + (HEIGHT + GAP) * visible_slots.saturating_sub(1) as f32;
        let (fan, mut moving) = self.expanded.sample_batch(now, reduced);
        let fan = fan.clamp(0., 1.);
        let span = base + (span_open.max(base) - base) * fan;
        let (width, gutter) = position_width(w.viewport_size());
        let bounds = self.stack_bounds.get();
        let root_top = if top {
            bounds.top()
        } else {
            bounds.bottom() - px(span)
        };
        self.hit_cards.clear();
        for slot in self.slots.iter().take(visible_slots) {
            let shape = geometry(
                slot.depth.value(cx),
                slot.travel.value(cx),
                fan,
                span,
                top,
                1.,
            );
            let inset = bounds.size.width * shape.inset;
            self.hit_cards.push((
                slot.serial,
                Bounds::new(
                    point(bounds.left() + inset, root_top + px(shape.top)),
                    size(bounds.size.width - inset * 2., px(shape.height)),
                ),
            ));
        }
        self.hovered_slot = self.pointed_card(w.mouse_position());
        let stack_bounds = self.stack_bounds.clone();
        let measure = self.geometry_visible.clone();
        let weak = cx.weak_entity();
        let report = canvas(
            move |bounds, w, _| {
                let viewport = Bounds::new(Point::default(), w.viewport_size());
                let visible = w.is_visible()
                    && bounds.intersects(&viewport)
                    && bounds.intersects(&w.content_mask().bounds);
                let previous = stack_bounds.replace(bounds);
                let anchor_changed = previous.left() != bounds.left()
                    || previous.size.width != bounds.size.width
                    || if top {
                        previous.top() != bounds.top()
                    } else {
                        previous.bottom() != bounds.bottom()
                    };
                if measure.replace(visible) != visible || anchor_changed {
                    let weak = weak.clone();
                    w.on_next_frame(move |_, cx| {
                        let _ = weak.update(cx, |_, cx| cx.notify());
                    });
                }
            },
            |_, _, _, _| (),
        )
        .absolute()
        .inset_0();
        let label = self
            .options
            .label
            .clone()
            .unwrap_or_else(|| tr(cx, "Recent activity", "近期动态").into());
        let mut stack = div()
            .id("proof-stack")
            .debug_selector(|| "proof-stack".into())
            .relative()
            .w_full()
            .h(px(span))
            .track_focus(&self.scope)
            .tab_group()
            .tab_stop(false)
            .role(Role::Region)
            .aria_label(label)
            .on_hover(cx.listener(|s, over: &bool, _, cx| {
                let next = *over && !s.touch && !s.slots.is_empty();
                if s.hovered != next {
                    s.hovered = next;
                    if !next {
                        s.hovered_slot = None;
                    }
                    s.update_activity(cx);
                }
            }))
            .on_mouse_move(cx.listener(|s, event: &MouseMoveEvent, _, cx| {
                let next = s.pointed_card(event.position);
                if s.hovered_slot != next {
                    s.hovered_slot = next;
                    cx.notify();
                }
            }))
            .on_key_down(cx.listener(Self::dismiss_key))
            .child(report);
        if self
            .leaving
            .as_ref()
            .is_some_and(|leaving| leaving.fade.sample(now).0 <= 0.001)
        {
            self.leaving = None;
        }
        for (index, slot) in self.slots.iter().enumerate().rev() {
            let (opacity, fading) = slot.fade.sample(now);
            let active = index < visible_slots;
            if !active && (self.leaving.is_some() || opacity <= 0.001) {
                continue;
            }
            let (ink, reveal) = slot.content.sample(now);
            let (depth, depth_moving) = slot.depth.sample_batch(now, reduced);
            let (travel, travel_moving) = slot.travel.sample_batch(now, reduced);
            moving |= fading || reveal || depth_moving || travel_moving;
            let shape = geometry(depth, travel, fan, span, top, 1.);
            let order =
                active.then_some(index.saturating_mul(2).min(isize::MAX as usize - 1) as isize);
            stack = stack.child(self.live_layer(
                slot,
                Layer {
                    shape,
                    opacity,
                    ink,
                },
                order,
                w,
                cx,
            ));
        }
        if let Some(leaving) = &self.leaving {
            let (opacity, running) = leaving.fade.sample(now);
            moving |= running;
            let age = now.saturating_duration_since(leaving.start).as_secs_f32();
            let shrink = if reduced {
                1.
            } else {
                1. - 0.035 * ease(age / FADE)
            };
            let shape = geometry(leaving.depth, leaving.travel, fan, span, top, shrink);
            stack = stack.child(shell(shape, opacity, p, cx).child(content(
                &leaving.event,
                leaving.content,
                shape.scale,
                p,
                cx,
            )));
        }
        if visible && moving {
            w.request_animation_frame();
        }
        if !self.announcement.is_empty() {
            stack = stack.child(
                div()
                    .id("proof-status")
                    .absolute()
                    .size(px(1.))
                    .opacity(0.)
                    .role(Role::Status)
                    .aria_label(self.announcement.clone()),
            );
        }
        let root = div().relative().flex_none();
        let root = if let Some(style) = &self.style {
            style(root)
        } else {
            root
        };
        let root = root.id(self.id.clone());
        if inline {
            root.w(px(WIDTH))
                .max_w_full()
                .h(px(base))
                .child(stack.absolute().bottom_0().left_0())
        } else {
            let view = w.viewport_size();
            let x = if self.options.position.right() {
                view.width - width - gutter
            } else {
                gutter
            };
            let y = if top {
                gutter
            } else {
                view.height - gutter - px(span)
            };
            root.size(px(0.)).child(
                deferred(anchored().position(point(x, y)).child(stack.w(width))).with_priority(3),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{SocialProofEvent, SocialProofOptions, SocialProofPosition, SocialProofToast};
    use crate::foundation::Theme;
    use gpui::{Modifiers, TestAppContext, VisualTestContext, point, px, size};
    use std::{cell::RefCell, rc::Rc, time::Duration};

    fn events() -> Option<Vec<SocialProofEvent>> {
        Some(
            (1..=5)
                .map(|i| {
                    SocialProofEvent::new(i.to_string(), format!("Team {i}"), "started a trial")
                })
                .collect(),
        )
    }

    fn tick(w: &mut VisualTestContext, millis: u64) {
        w.executor().advance_clock(Duration::from_millis(millis));
        w.update(|w, cx| {
            w.simulate_next_frame(cx);
            w.refresh();
        });
        w.run_until_parked();
    }

    #[test]
    fn pause_and_visibility_preserve_the_interval_and_session_limit() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let called = Rc::new(RefCell::new(vec![]));
        let shown = called.clone();
        let (view, w) = app.add_window_view(|w, cx| {
            SocialProofToast::new(
                SocialProofOptions {
                    events: events(),
                    prefill: 1,
                    limit: Some(3),
                    initial_delay: Duration::from_millis(800),
                    ..Default::default()
                },
                w,
                cx,
            )
            .on_show(move |event, _| shown.borrow_mut().push(event.id.clone()))
        });
        w.simulate_mouse_move(point(px(600.), px(300.)), None, Modifiers::default());
        tick(w, 0);
        tick(w, 300);
        assert_eq!(called.borrow().as_slice(), &["1"]);
        w.update(|_, cx| view.update(cx, |s, cx| s.set_paused(true, cx)));
        w.update(|_, cx| assert_eq!(view.read(cx).remaining, Duration::from_millis(500)));
        tick(w, 6000);
        assert_eq!(called.borrow().as_slice(), &["1"]);
        w.update(|_, cx| view.update(cx, |s, cx| s.set_paused(false, cx)));
        tick(w, 499);
        assert_eq!(called.borrow().as_slice(), &["1"]);
        tick(w, 2);
        assert_eq!(called.borrow().as_slice(), &["1", "2"]);
        tick(w, 800);
        w.update(|_, cx| view.update(cx, |s, cx| s.set_visible(false, cx)));
        w.update(|_, cx| assert_eq!(view.read(cx).remaining, Duration::from_millis(3199)));
        tick(w, 9000);
        assert_eq!(called.borrow().as_slice(), &["1", "2"]);
        w.update(|_, cx| view.update(cx, |s, cx| s.set_visible(true, cx)));
        tick(w, 0);
        tick(w, 3198);
        assert_eq!(called.borrow().as_slice(), &["1", "2"]);
        tick(w, 2);
        tick(w, 9000);
        assert_eq!(called.borrow().as_slice(), &["1", "2", "3"]);
        w.update(|_, cx| assert!(view.read(cx).timer.is_none()));
    }

    #[test]
    fn escape_moves_focus_and_brings_back_the_hidden_card() {
        let mut app = TestAppContext::single();
        app.update(|cx| {
            crate::init(cx);
            Theme::global_mut(cx).reduced_motion = true;
        });
        let dismissed = Rc::new(RefCell::new(vec![]));
        let observed = dismissed.clone();
        let (view, w) = app.add_window_view(|w, cx| {
            SocialProofToast::new(
                SocialProofOptions {
                    events: events(),
                    prefill: 3,
                    paused: true,
                    ..Default::default()
                },
                w,
                cx,
            )
            .on_dismiss(move |event, _| observed.borrow_mut().push(event.id.clone()))
        });
        tick(w, 0);
        w.update(|_, cx| {
            view.update(cx, |s, cx| {
                s.advance(cx);
                let slot = &s.slots[0];
                assert_eq!(slot.travel.value(cx), 0.);
                assert_eq!(slot.fade.sample(cx.background_executor().now()).0, 0.);
            })
        });
        tick(w, 180);
        w.update(|w, cx| view.read(cx).slots[0].close.clone().focus(w, cx));
        tick(w, 0);
        w.update(|_, cx| {
            let state = view.read(cx);
            assert!(state.focused);
            assert_eq!(
                state
                    .visible_events()
                    .map(|event| event.id.as_ref())
                    .collect::<Vec<_>>(),
                ["4", "3", "2"]
            );
        });
        w.simulate_keystrokes("escape");
        tick(w, 180);
        w.update(|w, cx| {
            let state = view.read(cx);
            assert!(state.slots[0].close.is_focused(w));
            assert_eq!(
                state
                    .visible_events()
                    .map(|event| event.id.as_ref())
                    .collect::<Vec<_>>(),
                ["3", "2", "1"]
            );
        });
        assert_eq!(dismissed.borrow().as_slice(), &["4"]);
    }

    #[test]
    fn pinned_card_keeps_its_height_and_uses_the_phone_gutter() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (_, w) = app.add_window_view(|w, cx| {
            SocialProofToast::new(
                SocialProofOptions {
                    events: events(),
                    prefill: 1,
                    paused: true,
                    position: SocialProofPosition::TopRight,
                    ..Default::default()
                },
                w,
                cx,
            )
        });
        w.simulate_resize(size(px(900.), px(480.)));
        tick(w, 0);
        let desktop = w.debug_bounds("proof-card-1").unwrap();
        assert_eq!(desktop.top(), px(24.));
        assert_eq!(desktop.right(), px(876.));
        assert_eq!(desktop.size, size(px(360.), px(64.)));
        w.simulate_resize(size(px(320.), px(480.)));
        tick(w, 0);
        let phone = w.debug_bounds("proof-card-1").unwrap();
        assert_eq!(phone.top(), px(16.));
        assert_eq!(phone.left(), px(16.));
        assert_eq!(phone.size, size(px(288.), px(64.)));
    }
}
