//! A layered wallet with continuous card motion and accessible activity.
use crate::{
    foundation::surface_radius,
    primitives::{
        family::{Paint, PhaseClock, Slide, disabled_motion, ease},
        glyph::{Glyph, icon, transformed},
        presentation::{column, row, tr},
    },
};
use gpui::{prelude::*, *};
use image::{Frame, Rgba, RgbaImage};
use std::{cell::Cell, rc::Rc, sync::Arc};

const WIDTH: f32 = 296.;
const ASPECT: f32 = 53.98 / 85.6;
const MARGIN: f32 = 12.;
const SHADOW_GUTTER: f32 = 76.;
const CARD_FAN: f32 = 0.24;
const OPEN_SCALE: f32 = 0.88;

/// The visual finish of a card.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum WalletVariant {
    #[default]
    Metal,
    Black,
    Glass,
    Color,
}

/// Legacy finish names mapped to current materials.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WalletFinish {
    Paper,
    Graphite,
    Clay,
    Photo,
}

impl From<WalletFinish> for WalletVariant {
    fn from(value: WalletFinish) -> Self {
        match value {
            WalletFinish::Paper => Self::Metal,
            WalletFinish::Graphite => Self::Black,
            WalletFinish::Clay | WalletFinish::Photo => Self::Color,
        }
    }
}

/// The optional mark on a card. The default is a fictional payment network.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum WalletNetwork {
    #[default]
    Fictional,
    Hidden,
    Label(SharedString),
    Svg(SharedString),
}

/// An activity item. Negative amounts represent spending.
#[derive(Clone)]
pub struct WalletTransaction {
    pub id: SharedString,
    pub merchant: SharedString,
    pub detail: SharedString,
    pub amount: f64,
    pub icon: Option<Glyph>,
    pub avatar: Option<ImageSource>,
}

impl WalletTransaction {
    pub fn new(
        id: impl Into<SharedString>,
        merchant: impl Into<SharedString>,
        detail: impl Into<SharedString>,
        amount: f64,
    ) -> Self {
        Self {
            id: id.into(),
            merchant: merchant.into(),
            detail: detail.into(),
            amount,
            icon: None,
            avatar: None,
        }
    }

    pub fn with_icon(mut self, icon: Glyph) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn with_avatar(mut self, avatar: impl Into<ImageSource>) -> Self {
        self.avatar = Some(avatar.into());
        self
    }
}

/// A card with only a display suffix, never a full account number.
#[derive(Clone)]
pub struct WalletStackCard {
    pub id: SharedString,
    pub issuer: SharedString,
    pub product: SharedString,
    last_four: SharedString,
    pub holder: SharedString,
    pub variant: WalletVariant,
    pub tint: Option<Hsla>,
    pub expires: Option<SharedString>,
    pub network: WalletNetwork,
    pub image: Option<ImageSource>,
    pub balance: f64,
    pub balance_label: SharedString,
    pub transactions: Vec<WalletTransaction>,
}

impl WalletStackCard {
    pub fn new(
        id: impl Into<SharedString>,
        issuer: impl Into<SharedString>,
        product: impl Into<SharedString>,
        last_four: impl AsRef<str>,
        holder: impl Into<SharedString>,
        balance: f64,
        balance_label: impl Into<SharedString>,
    ) -> Self {
        let raw = last_four.as_ref();
        let digits: String = raw
            .chars()
            .rev()
            .filter(|c| c.is_ascii_digit())
            .take(4)
            .collect();
        let suffix: String = digits.chars().rev().collect();
        Self {
            id: id.into(),
            issuer: issuer.into(),
            product: product.into(),
            last_four: suffix.into(),
            holder: holder.into(),
            variant: WalletVariant::Metal,
            tint: None,
            expires: None,
            network: WalletNetwork::Fictional,
            image: None,
            balance,
            balance_label: balance_label.into(),
            transactions: vec![],
        }
    }

    pub fn last_four(&self) -> &str {
        self.last_four.as_ref()
    }

    pub fn with_variant(mut self, variant: WalletVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn with_finish(self, finish: WalletFinish) -> Self {
        self.with_variant(finish.into())
    }

    pub fn with_tint(mut self, tint: impl Into<Hsla>) -> Self {
        self.tint = Some(tint.into());
        self
    }

    pub fn with_expiration(mut self, expires: impl Into<SharedString>) -> Self {
        self.expires = Some(expires.into());
        self
    }

    pub fn with_network(mut self, network: WalletNetwork) -> Self {
        self.network = network;
        self
    }

    pub fn with_image(mut self, image: impl Into<ImageSource>) -> Self {
        self.image = Some(image.into());
        self
    }

    pub fn with_transactions(mut self, transactions: Vec<WalletTransaction>) -> Self {
        self.transactions = transactions;
        self
    }
}

/// Emits the opened card, or `None` on return to the stack.
#[derive(Clone)]
pub struct WalletStackSelected(pub Option<WalletStackCard>);

type AmountFormatter = Box<dyn Fn(f64, &str, &str) -> String>;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Axis {
    Pending,
    Vertical,
    Horizontal,
}

struct Drag {
    start: Point<Pixels>,
    start_value: f32,
    value: f32,
    open: bool,
    front: usize,
    step: f32,
    clock: PhaseClock,
    last_time: f32,
    last_y: f32,
    speed: f32,
    axis: Axis,
    touch: bool,
    tap: Option<usize>,
}

#[derive(Clone, Copy)]
struct Pose {
    top: f32,
    scale: f32,
}

impl Pose {
    fn mix(self, to: Self, t: f32) -> Self {
        Self {
            top: self.top + (to.top - self.top) * t,
            scale: self.scale + (to.scale - self.scale) * t,
        }
    }
}

fn card_pose(
    i: usize,
    position: f32,
    open: f32,
    fan: f32,
    h: f32,
    stage: f32,
    count: usize,
) -> Pose {
    let max = count.saturating_sub(1) as f32;
    let clamped = position.clamp(0., max);
    let d = i as f32 - clamped;
    let pocket = stage - 29.6;
    let front = Pose {
        top: h * (0.60 + CARD_FAN * fan),
        scale: 1.,
    };
    let closed = if d >= 0. {
        Pose {
            top: front.top - h * (0.074 + 0.16 * fan) * d,
            scale: (1. - 0.05 * d).max(0.60),
        }
    } else {
        let depth = -d;
        front.mix(
            Pose {
                top: pocket + 7. * (depth - 1.).max(0.),
                scale: (0.90 + 0.02 * (depth - 1.).max(0.)).min(0.96),
            },
            depth.min(1.),
        )
    };
    let weight = (1. - d.abs()).max(0.);
    let tucked = Pose {
        top: pocket + (count.saturating_sub(i + 1).min(4) as f32) * 6.,
        scale: (0.90 + count.saturating_sub(i + 1).min(3) as f32 * 0.02).min(0.96),
    };
    let lifted = Pose {
        top: MARGIN,
        scale: OPEN_SCALE,
    };
    let mut pose = closed.mix(tucked.mix(lifted, weight), open);
    pose.top += (position - clamped) * (pocket - front.top).max(40.) * (1. - open);
    pose
}

fn resist(value: f32, max: f32) -> f32 {
    let edge = value.clamp(0., max);
    let over = value - edge;
    edge + over / (2.8 + over.abs() * 2.)
}

struct WalletSlot {
    card: Rc<WalletStackCard>,
    focus: FocusHandle,
    bounds: Rc<Cell<Bounds<Pixels>>>,
}

/// A wallet whose resting, dragged, fanned and opened states share one geometry.
pub struct WalletStack {
    id: ElementId,
    label: Option<SharedString>,
    currency: SharedString,
    locale: SharedString,
    formatter: Option<AmountFormatter>,
    style: Option<Box<dyn Fn(Div) -> Div>>,
    slots: Vec<WalletSlot>,
    front: usize,
    opened: bool,
    position: Slide,
    open: Slide,
    fan: Slide,
    hover: Slide,
    highlight: Option<usize>,
    light: [Slide; 2],
    device_light: bool,
    mouse_inside: bool,
    touch_mode: bool,
    drag: Option<Drag>,
    suppress_click: bool,
    amount_clock: PhaseClock,
    detail_clock: PhaseClock,
    back: FocusHandle,
    announcement: SharedString,
    room: Rc<Cell<Pixels>>,
    stage: Rc<Cell<Bounds<Pixels>>>,
}

impl EventEmitter<WalletStackSelected> for WalletStack {}

impl WalletStack {
    pub fn new(cards: Vec<WalletStackCard>, cx: &mut Context<Self>) -> Self {
        Self {
            id: format!("wallet-{}", cx.entity_id()).into(),
            label: None,
            currency: "USD".into(),
            locale: "en-US".into(),
            formatter: None,
            style: None,
            slots: cards
                .into_iter()
                .map(|card| WalletSlot {
                    card: Rc::new(card),
                    focus: cx.focus_handle().tab_stop(true),
                    bounds: Rc::default(),
                })
                .collect(),
            front: 0,
            opened: false,
            position: Slide::new(0.),
            open: Slide::new(0.),
            fan: Slide::snappy(0.),
            hover: Slide::snappy(0.),
            highlight: None,
            light: [Slide::snappy(0.); 2],
            device_light: false,
            mouse_inside: false,
            touch_mode: false,
            drag: None,
            suppress_click: false,
            amount_clock: PhaseClock::default(),
            detail_clock: PhaseClock::default(),
            back: cx.focus_handle().tab_stop(true),
            announcement: "".into(),
            room: Rc::default(),
            stage: Rc::default(),
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

    pub fn with_currency(mut self, code: impl Into<SharedString>) -> Self {
        self.currency = code.into();
        self
    }

    pub fn with_locale(mut self, locale: impl Into<SharedString>) -> Self {
        self.locale = locale.into();
        self
    }

    /// Override amount formatting for a host with more locale data.
    pub fn with_amount_formatter(
        mut self,
        formatter: impl Fn(f64, &str, &str) -> String + 'static,
    ) -> Self {
        self.formatter = Some(Box::new(formatter));
        self
    }

    pub fn with_style(mut self, style: impl Fn(Div) -> Div + 'static) -> Self {
        self.style = Some(Box::new(style));
        self
    }

    pub fn front_card(&self) -> Option<&WalletStackCard> {
        self.slots.get(self.front).map(|s| s.card.as_ref())
    }

    pub fn selected_card(&self) -> Option<&WalletStackCard> {
        self.opened.then(|| self.front_card()).flatten()
    }

    /// Set a host-provided orientation light in the range -1 to 1.
    /// Passing `None` returns to pointer-driven light.
    pub fn set_device_light(&mut self, tilt: Option<[f32; 2]>, cx: &mut Context<Self>) {
        self.device_light = tilt.is_some();
        let value = tilt
            .unwrap_or_default()
            .map(|v| if v.is_finite() { v.clamp(-1., 1.) } else { 0. });
        for (axis, v) in self.light.iter_mut().zip(value) {
            axis.set(v, cx);
        }
        cx.notify();
    }

    fn amount(&self, value: f64) -> String {
        self.formatter.as_ref().map_or_else(
            || format_amount(value, &self.currency, &self.locale),
            |f| f(value, &self.currency, &self.locale),
        )
    }

    fn focus_front(&self, w: &mut Window, cx: &mut Context<Self>) {
        if let Some(slot) = self.slots.get(self.front) {
            slot.focus.focus(w, cx);
        }
    }

    fn select(
        &mut self,
        i: usize,
        opened: bool,
        focus: bool,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(slot) = self.slots.get(i) else {
            return;
        };
        let moved = i != self.front;
        let changed = opened != self.opened;
        self.front = i;
        self.opened = opened;
        self.position.duration(i as f32, 0.40, cx);
        self.open.duration(f32::from(opened), 0.40, cx);
        self.fan.duration(
            f32::from(!opened && self.mouse_inside && !self.touch_mode),
            0.32,
            cx,
        );
        if opened && (moved || changed) {
            self.amount_clock.restart(cx);
            self.detail_clock.restart(cx);
        }
        if focus {
            slot.focus.focus(w, cx);
        }
        if moved || changed {
            let heading = &slot.card.issuer;
            self.announcement = if tr(cx, "en", "zh") == "zh" {
                format!("{heading}，第 {} 张，共 {} 张", i + 1, self.slots.len()).into()
            } else {
                format!("{heading}, {} of {}", i + 1, self.slots.len()).into()
            };
        }
        if changed || opened && moved {
            cx.emit(WalletStackSelected(
                opened.then(|| slot.card.as_ref().clone()),
            ));
        }
        cx.notify();
    }

    fn toggle(&mut self, i: usize, w: &mut Window, cx: &mut Context<Self>) {
        self.select(i, !self.opened || i != self.front, true, w, cx);
    }

    fn key(&mut self, e: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        if self.slots.is_empty() {
            return;
        }
        let last = self.slots.len() - 1;
        match e.keystroke.key.as_str() {
            "right" | "down" => self.select((self.front + 1).min(last), self.opened, true, w, cx),
            "left" | "up" => self.select(self.front.saturating_sub(1), self.opened, true, w, cx),
            "home" => self.select(0, self.opened, true, w, cx),
            "end" => self.select(last, self.opened, true, w, cx),
            "escape" if self.opened => self.select(self.front, false, true, w, cx),
            _ => return,
        }
        w.prevent_default();
        cx.stop_propagation();
    }

    fn begin(&mut self, at: Point<Pixels>, touch: bool, w: &mut Window, cx: &mut Context<Self>) {
        if self.slots.is_empty() || self.drag.is_some() {
            return;
        }
        let tap = if touch {
            let hit = (0..self.slots.len()).find(|&i| self.slots[i].bounds.get().contains(&at));
            if hit.is_none() {
                return;
            }
            hit
        } else {
            None
        };
        self.suppress_click = false;
        if touch {
            self.touch_mode = true;
            self.fan.reset(0.);
            self.hover.reset(0.);
            if !self.device_light {
                for axis in &mut self.light {
                    axis.reset(0.);
                }
            }
        }
        let h = self.stage.get().size.height - px(220.);
        let h = f32::from(h.max(px(1.)));
        let at_rest = h * (0.60 + CARD_FAN * self.fan.value(cx));
        let step = if self.opened {
            (at_rest - MARGIN).max(48.)
        } else {
            (h + 220. - 29.6 - at_rest).max(64.)
        };
        let value = if self.opened {
            self.open.value(cx)
        } else {
            self.position.value(cx)
        };
        let mut clock = PhaseClock::default();
        clock.restart(cx);
        self.drag = Some(Drag {
            start: at,
            start_value: value,
            value,
            open: self.opened,
            front: self.front,
            step,
            clock,
            last_time: 0.,
            last_y: at.y.into(),
            speed: 0.,
            axis: Axis::Pending,
            touch,
            tap,
        });
        self.focus_front(w, cx);
        cx.notify();
    }

    fn move_drag(&mut self, at: Point<Pixels>, touch: bool, cx: &mut Context<Self>) {
        let Some(drag) = self.drag.as_mut().filter(|d| d.touch == touch) else {
            return;
        };
        let dx = f32::from(at.x - drag.start.x);
        let dy = f32::from(at.y - drag.start.y);
        if drag.axis == Axis::Pending && dx.abs().max(dy.abs()) >= 5. {
            drag.axis = if dy.abs() > dx.abs() {
                Axis::Vertical
            } else {
                Axis::Horizontal
            };
            self.suppress_click = true;
        }
        if drag.axis != Axis::Vertical {
            return;
        }
        let now = drag.clock.age(cx);
        let y = f32::from(at.y);
        let dt = now - drag.last_time;
        if dt > 0.004 && y != drag.last_y {
            drag.speed = ((y - drag.last_y) / dt).clamp(-2600., 2600.);
            drag.last_time = now;
            drag.last_y = y;
        }
        let target = dy / drag.step;
        drag.value = if drag.open {
            (drag.start_value - target).clamp(0., 1.)
        } else {
            resist(
                drag.start_value + target,
                self.slots.len().saturating_sub(1) as f32,
            )
        };
        cx.notify();
    }

    fn finish(&mut self, touch: bool, cancel: bool, w: &mut Window, cx: &mut Context<Self>) {
        if self.drag.as_ref().is_none_or(|d| d.touch != touch) {
            return;
        }
        let Some(drag) = self.drag.take() else { return };
        if drag.axis == Axis::Vertical {
            let velocity = if cancel || drag.clock.age(cx) - drag.last_time > 0.10 {
                0.
            } else {
                drag.speed / drag.step
            };
            if drag.open {
                self.open.reset(drag.value);
                let next = cancel || drag.value - velocity * 0.16 > 0.55;
                self.select(drag.front, next, true, w, cx);
                self.open.release(f32::from(next), -velocity, 0.40, cx);
            } else {
                let last = self.slots.len().saturating_sub(1);
                let target = if cancel {
                    drag.front
                } else {
                    (drag.value + velocity * 0.16).round().max(0.) as usize
                }
                .clamp(drag.front.saturating_sub(1), (drag.front + 1).min(last));
                self.position.reset(drag.value);
                self.select(target, false, true, w, cx);
                self.position.release(target as f32, velocity, 0.40, cx);
            }
        } else if !cancel
            && drag.axis == Axis::Pending
            && let Some(i) = drag.tap
        {
            self.suppress_click = true;
            self.toggle(i, w, cx);
        }
        if self.suppress_click {
            let wallet = cx.entity().downgrade();
            cx.defer(move |cx| {
                let _ = wallet.update(cx, |s, _| s.suppress_click = false);
            });
        }
        cx.notify();
    }

    fn hover_stage(&mut self, over: bool, cx: &mut Context<Self>) {
        self.mouse_inside = over;
        self.fan.duration(
            f32::from(over && !self.opened && !self.touch_mode),
            0.32,
            cx,
        );
        if !over {
            self.hover.set(0., cx);
            if !self.device_light {
                for axis in &mut self.light {
                    axis.set(0., cx);
                }
            }
        }
        cx.notify();
    }

    fn pointer(&mut self, i: usize, at: Point<Pixels>, cx: &mut Context<Self>) {
        if self.drag.is_some() {
            return;
        }
        if self.touch_mode || !self.mouse_inside {
            self.touch_mode = false;
            self.hover_stage(true, cx);
        }
        self.highlight = Some(i);
        self.hover.set(1., cx);
        if !self.device_light && !disabled_motion(cx) {
            let card = self.slots[i].bounds.get();
            if card.size.width > px(0.) && card.size.height > px(0.) {
                self.light[0].set(
                    ((at.x - card.center().x) / (card.size.width / 2.)).clamp(-1., 1.),
                    cx,
                );
                self.light[1].set(
                    ((at.y - card.center().y) / (card.size.height / 2.)).clamp(-1., 1.),
                    cx,
                );
            }
        }
        cx.notify();
    }
}

fn format_amount(value: f64, code: &str, locale: &str) -> String {
    if !value.is_finite() {
        return "—".into();
    }
    let code = code.to_ascii_uppercase();
    let precision = match code.as_str() {
        "JPY" | "KRW" | "VND" | "CLP" | "XAF" | "XOF" | "XPF" => 0,
        "BHD" | "KWD" | "OMR" | "JOD" | "TND" => 3,
        _ => 2,
    };
    let lang = locale.split(['-', '_']).next().unwrap_or("en");
    let (group, decimal, after) = match lang {
        "de" | "es" | "it" | "pt" | "nl" => ('.', ',', lang != "pt"),
        "fr" | "ru" | "sv" | "pl" => ('\u{202f}', ',', true),
        _ => (',', '.', false),
    };
    let amount = format!("{:.precision$}", value.abs());
    let (whole, fraction) = amount.split_once('.').unwrap_or((&amount, ""));
    let mut grouped = String::new();
    for (i, digit) in whole.chars().enumerate() {
        let tail = whole.len() - i;
        let size = if lang == "hi" && tail > 3 { 2 } else { 3 };
        let remainder = if size == 2 { tail - 3 } else { tail };
        if i > 0 && remainder.is_multiple_of(size) {
            grouped.push(group);
        }
        grouped.push(digit);
    }
    if precision > 0 {
        grouped.push(decimal);
        grouped.push_str(fraction);
    }
    let symbol = match code.as_str() {
        "USD" if lang == "en" => "$",
        "USD" => "US$",
        "EUR" => "€",
        "GBP" => "£",
        "JPY" => "¥",
        "CNY" => "CN¥",
        "CAD" => "CA$",
        "AUD" => "A$",
        "INR" => "₹",
        "KRW" => "₩",
        "BRL" => "R$",
        _ => code.as_str(),
    };
    let sign = if value < 0. { "−" } else { "" };
    if after {
        format!("{sign}{grouped}\u{a0}{symbol}")
    } else if symbol == code {
        format!("{sign}{symbol}\u{a0}{grouped}")
    } else {
        format!("{sign}{symbol}{grouped}")
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
enum Texture {
    Metal,
    Black,
    Glass,
    Color,
    Light,
}

impl From<WalletVariant> for Texture {
    fn from(variant: WalletVariant) -> Self {
        match variant {
            WalletVariant::Metal => Self::Metal,
            WalletVariant::Black => Self::Black,
            WalletVariant::Glass => Self::Glass,
            WalletVariant::Color => Self::Color,
        }
    }
}

enum TextureAsset {}

impl Asset for TextureAsset {
    type Source = Texture;
    type Output = Arc<RenderImage>;

    #[allow(
        clippy::manual_async_fn,
        reason = "The texture worker cannot capture the UI-thread App."
    )]
    fn load(texture: Texture, _: &mut App) -> impl Future<Output = Self::Output> + Send + 'static {
        async move {
            let (width, height) = if texture == Texture::Light {
                (128, 128)
            } else {
                (512, 323)
            };
            let image = RgbaImage::from_fn(width, height, |x, y| {
                let grain = ((x.wrapping_mul(374761393) ^ y.wrapping_mul(668265263))
                    .wrapping_mul(2246822519)
                    >> 24) as f32
                    / 255.
                    - 0.5;
                let u = x as f32 / (width - 1) as f32;
                let v = y as f32 / (height - 1) as f32;
                let (r, g, b, a) = match texture {
                    Texture::Light => {
                        let d = ((u - 0.5) * 2.).hypot((v - 0.5) * 2.);
                        (255., 255., 255., 92. * (1. - d * 2.2).max(0.).powi(2))
                    }
                    Texture::Metal => {
                        let x = u - 0.58;
                        let y = (v - 0.46) * 0.86;
                        let d = x.hypot(y);
                        let cone =
                            (3. * y.atan2(x) + 0.68).cos() * (0.42 + 0.58 * (-2.2 * d).exp());
                        let brushed =
                            0.8 * (v * 337.4).sin() + 0.4 * (v * 852.1).sin() + 2.2 * grain;
                        let l = 225. + 17. * cone + 13. * (-9. * d).exp() + 6. * (1. - v) + brushed;
                        (l, l + 1., l + 3., 255.)
                    }
                    Texture::Black => {
                        let x = u - 0.72;
                        let y = (v - 0.57) * 0.84;
                        let theta = y.atan2(x);
                        let d = x.hypot(y);
                        let ridge = (0.5
                            + 0.5 * (d * 198. + 2.8 * (theta * 13. + d * 31.).sin()).cos())
                        .powi(9);
                        let beam = 10. * (1. - 0.68 * u - 0.74 * v).max(0.);
                        let l = 17. + 12. * ridge + beam + 3. * grain;
                        (l, l + 6., l + 11., 255.)
                    }
                    Texture::Glass => {
                        let upper = 47. * (1. - 0.72 * u - 1.1 * v).max(0.);
                        let lower = 25. * (u * v).powi(2);
                        let frost = 7. * grain;
                        (
                            40. + upper + lower + frost,
                            104. + upper - 10. * v + frost,
                            98. + upper + 0.5 * lower + frost,
                            244.,
                        )
                    }
                    Texture::Color => {
                        let light = 75. * (1. - u * 0.85 - v * 0.6).max(0.) + grain * 6.;
                        (231., 255., 243., light.max(0.))
                    }
                };
                let byte = |v: f32| v.round().clamp(0., 255.) as u8;
                // GPUI's atlas receives BGRA.
                Rgba([byte(b), byte(g), byte(r), byte(a)])
            });
            Arc::new(RenderImage::new([Frame::new(image)]))
        }
    }
}

fn resolve_image(source: &ImageSource, w: &mut Window, cx: &mut App) -> Option<Arc<RenderImage>> {
    let image = match source {
        ImageSource::Image(i) => i.clone().use_render_image(w, cx)?,
        ImageSource::Render(i) => i.clone(),
        ImageSource::Resource(r) => w.use_asset::<ImgResourceLoader>(r, cx)?.ok()?,
        ImageSource::Custom(loader) => loader(w, cx)?.ok()?,
    };
    image.as_bytes(0).is_some().then_some(image)
}

fn gradient(angle: f32, a: Hsla, b: Hsla) -> Background {
    linear_gradient(angle, linear_color_stop(a, 0.), linear_color_stop(b, 1.))
}

#[derive(Clone, Copy)]
struct CardSpace(Bounds<Pixels>);

impl CardSpace {
    fn s(self) -> f32 {
        f32::from(self.0.size.width) / WIDTH
    }

    fn at(self, x: f32, y: f32) -> Point<Pixels> {
        let scale = self.s();
        self.0.origin + point(px(x * scale), px(y * scale))
    }

    fn rect(self, x: f32, y: f32, w: f32, h: f32) -> Bounds<Pixels> {
        let scale = self.s();
        Bounds::new(self.at(x, y), size(px(w * scale), px(h * scale)))
    }

    fn stroke(
        self,
        w: &mut Window,
        thickness: f32,
        ink: Hsla,
        build: impl FnOnce(&mut PathBuilder),
    ) {
        let mut path = PathBuilder::stroke(px(thickness * self.s()));
        build(&mut path);
        if let Ok(path) = path.build() {
            w.paint_path(path, ink);
        }
    }

    fn label(
        self,
        text: SharedString,
        look: (f32, FontWeight, Hsla),
        region: Bounds<Pixels>,
        align: TextAlign,
        w: &mut Window,
        cx: &mut App,
    ) {
        let (em, weight, ink) = look;
        let font = px(em * self.s());
        let mut run = w.text_style().to_run(text.len());
        run.font.weight = weight;
        run.color = ink;
        let line = w.text_system().shape_line(text, font, &[run], None);
        let origin = match align {
            TextAlign::Right => point(region.right() - line.width, region.top()),
            _ => region.origin,
        };
        let bounds = region.intersect(&w.content_mask().bounds);
        if bounds.is_empty() {
            return;
        }
        w.with_content_mask(Some(ContentMask { bounds }), |w| {
            let _ = line.paint(origin, font * 1.22, TextAlign::Left, None, w, cx);
        });
    }
}

fn paint_chip(space: CardSpace, gold: bool, w: &mut Window) {
    let chip = space.rect(25.2, 63.6, 40.2, 30.6);
    let color = |v| Hsla::from(rgb(v));
    let (start, end, edge) = if gold {
        (0xf8eac3, 0xab7940, 0x876037)
    } else {
        (0xf4f6f9, 0x8c949b, 0x69737e)
    };
    let r = px(5. * space.s());
    w.paint_quad(quad(
        chip,
        r,
        gradient(135., color(start), color(end)),
        px(0.5),
        color(edge),
        BorderStyle::Solid,
    ));
    let plate = space.rect(38., 69., 13.6, 19.8);
    w.paint_quad(
        fill(plate, gradient(125., color(end), color(start))).corner_radii(px(2.5 * space.s())),
    );
    space.stroke(w, 0.65, color(edge).opacity(0.70), |p| {
        let a = |x, y| space.at(x, y);
        for points in [
            [(25.4, 72.8), (35., 72.8), (38.2, 76.4)],
            [(25.4, 85.4), (35., 85.4), (38.2, 81.8)],
            [(65.1, 72.8), (54.7, 72.8), (51.5, 76.4)],
            [(65.1, 85.4), (54.7, 85.4), (51.5, 81.8)],
        ] {
            p.move_to(a(points[0].0, points[0].1));
            for (x, y) in points.into_iter().skip(1) {
                p.line_to(a(x, y));
            }
        }
        p.move_to(a(44.8, 64.));
        p.line_to(a(44.8, 69.));
        p.move_to(a(44.8, 89.));
        p.line_to(a(44.8, 93.8));
    });
}

fn paint_contact(space: CardSpace, ink: Hsla, w: &mut Window) {
    space.stroke(w, 1.05, ink.opacity(0.70), |path| {
        for r in [3.0_f32, 6., 9., 12.] {
            let a = 0.63_f32;
            let (sin, cos) = a.sin_cos();
            path.move_to(space.at(78. + r * cos, 79. - r * sin));
            path.curve_to(
                space.at(78. + r * cos, 79. + r * sin),
                space.at(78. + r * 1.18, 79.),
            );
        }
    });
}

#[derive(Clone)]
struct CardArt {
    card: Rc<WalletStackCard>,
    texture: Option<Arc<RenderImage>>,
    image: Option<Arc<RenderImage>>,
    lift: f32,
    radius: Pixels,
    expiry_label: &'static str,
}

impl CardArt {
    fn paint(&self, b: Bounds<Pixels>, w: &mut Window, cx: &mut App) {
        let space = CardSpace(b);
        let s = space.s();
        let r = self.radius * (s * 1.06).min(1.);
        let variant = self.card.variant;
        let gold = variant == WalletVariant::Metal;
        let ink: Hsla = rgb(if gold { 0x202224 } else { 0xe9f3f3 }).into();
        let base: Hsla = match variant {
            WalletVariant::Metal => rgb(0xd9dbdd).into(),
            WalletVariant::Black => rgb(0x141a20).into(),
            WalletVariant::Glass => rgb(0x254e45).opacity(0.44).into(),
            WalletVariant::Color => self.card.tint.unwrap_or_else(|| rgb(0x03513a).into()),
        };
        w.paint_drop_shadows(
            b,
            r.into(),
            &[
                BoxShadow::new(px(0.), px(9. * s), black().opacity(0.14 + self.lift * 0.10))
                    .blur_radius(px((20. + self.lift * 8.) * s))
                    .spread_radius(px(-4. * s)),
            ],
        );
        w.paint_quad(fill(b, base).corner_radii(r));
        if let Some(image) = &self.image {
            let size = image.size(0).map(|p| px(p.0 as f32));
            if size.width > px(0.) && size.height > px(0.) {
                let scale = (b.size.width / size.width).max(b.size.height / size.height);
                let cover = size.map(|v| v * scale);
                let image_bounds = Bounds::new(
                    b.center() - point(cover.width / 2., cover.height / 2.),
                    cover,
                );
                let _ = w.paint_image(b, image_bounds, r.into(), image.clone(), 0, false);
            }
            let glaze = if gold {
                white().opacity(0.35)
            } else {
                black().opacity(0.30)
            };
            w.paint_quad(fill(b, glaze).corner_radii(r));
        }
        if (self.image.is_none() || !matches!(variant, WalletVariant::Metal | WalletVariant::Black))
            && let Some(texture) = &self.texture
        {
            let _ = w.paint_image(b, b, r.into(), texture.clone(), 0, false);
        }
        w.paint_quad(
            outline(
                b,
                white().opacity(if gold { 0.27 } else { 0.15 }),
                BorderStyle::Solid,
            )
            .corner_radii(r),
        );
        paint_chip(space, gold, w);
        paint_contact(space, ink, w);
        let text = |text, em, weight, color, region, align, w: &mut Window, cx: &mut App| {
            space.label(text, (em, weight, color), region, align, w, cx)
        };
        text(
            self.card.issuer.clone(),
            16.,
            FontWeight::MEDIUM,
            ink,
            space.rect(20.7, 17.8, 158., 25.),
            TextAlign::Left,
            w,
            cx,
        );
        text(
            self.card.product.clone(),
            9.8,
            FontWeight::NORMAL,
            ink.opacity(0.72),
            space.rect(180., 21.7, 95.3, 15.),
            TextAlign::Right,
            w,
            cx,
        );
        for group in 0..3 {
            for dot in 0..4 {
                let x = 21.8 + group as f32 * 24. + dot as f32 * 3.55;
                w.paint_quad(
                    fill(space.rect(x, 118.7, 2.6, 2.6), ink.opacity(0.83))
                        .corner_radii(px(1.3 * s)),
                );
            }
        }
        for (i, digit) in self.card.last_four().chars().enumerate() {
            text(
                digit.to_string().into(),
                16.6,
                FontWeight::MEDIUM,
                ink,
                space.rect(93. + 11.1 * i as f32, 108.4, 11.1, 27.),
                TextAlign::Left,
                w,
                cx,
            );
        }
        text(
            self.card.holder.clone(),
            10.4,
            FontWeight::NORMAL,
            ink,
            space.rect(20.7, 154.1, 127., 19.),
            TextAlign::Left,
            w,
            cx,
        );
        if let Some(expiry) = &self.card.expires {
            text(
                self.expiry_label.into(),
                6.6,
                FontWeight::NORMAL,
                ink.opacity(0.66),
                space.rect(153.9, 147.0, 65., 11.),
                TextAlign::Left,
                w,
                cx,
            );
            text(
                expiry.clone(),
                10.4,
                FontWeight::MEDIUM,
                ink,
                space.rect(153.9, 154.3, 65., 19.),
                TextAlign::Left,
                w,
                cx,
            );
        }
        match &self.card.network {
            WalletNetwork::Hidden => {}
            WalletNetwork::Svg(asset) => {
                let _ = w.paint_svg(
                    space.rect(233., 149., 43., 22.),
                    asset.clone(),
                    None,
                    TransformationMatrix::unit(),
                    ink,
                    cx,
                );
            }
            WalletNetwork::Label(label) => text(
                label.clone(),
                12.4,
                FontWeight::MEDIUM,
                ink,
                space.rect(216., 151.8, 62., 19.),
                TextAlign::Right,
                w,
                cx,
            ),
            WalletNetwork::Fictional => {
                space.stroke(w, 2., ink, |p| {
                    p.move_to(space.at(242., 154.));
                    p.arc_to(
                        point(px(5.7 * s), px(5.7 * s)),
                        px(0.),
                        true,
                        false,
                        space.at(246., 156.),
                    );
                    p.move_to(space.at(245.5, 153.));
                    p.line_to(space.at(248., 151.));
                });
                text(
                    "orbit".into(),
                    12.2,
                    FontWeight::MEDIUM,
                    ink,
                    space.rect(251., 151.5, 30., 18.),
                    TextAlign::Left,
                    w,
                    cx,
                );
            }
        }
    }
}

fn paint_light(
    b: Bounds<Pixels>,
    radius: Pixels,
    tilt: [f32; 2],
    texture: Arc<RenderImage>,
    w: &mut Window,
) {
    let scale = CardSpace(b).s();
    let r = radius * (scale * 1.06).min(1.);
    let anchor = b.center()
        + point(
            b.size.width * (tilt[0] * 0.42),
            b.size.height * (tilt[1] * 0.42),
        );
    let extent = b.size.map(|p| p * 2.0);
    let source = Bounds::new(
        anchor - point(extent.width / 2., extent.height / 2.),
        extent,
    );
    let _ = w.paint_image(b, source, r.into(), texture, 0, false);
}

impl WalletStack {
    fn activity(&self, h: f32, reveal: f32, w: &mut Window, cx: &App) -> Option<Stateful<Div>> {
        let card = self.front_card().filter(|_| self.opened || reveal > 0.01)?;
        let p = Paint::get(cx);
        let count = self.amount_clock.time(0.53, w, cx);
        let activity = self.detail_clock.time(0.52, w, cx);
        let value = self.amount(card.balance * f64::from(ease((count - 0.16) / 0.37)));
        let mut details = column()
            .id("wallet-activity")
            .when(self.opened, |e| {
                e.role(Role::Region).aria_label(format!(
                    "{} · {}",
                    card.issuer,
                    tr(cx, "Recent activity", "近期交易"),
                ))
            })
            .absolute()
            .top(px(26. + h * OPEN_SCALE + (1. - reveal) * 9.))
            .left(px(SHADOW_GUTTER))
            .right(px(SHADOW_GUTTER))
            .opacity(reveal)
            .text_color(p.ink)
            .child(
                row()
                    .h(px(26.))
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .text_size(px(13.))
                            .line_height(px(18.))
                            .text_color(p.secondary)
                            .child(card.balance_label.clone()),
                    )
                    .child(
                        div()
                            .flex_none()
                            .text_size(px(20.))
                            .line_height(px(26.))
                            .child(value),
                    ),
            );
        if !card.transactions.is_empty() {
            details = details.child(
                column()
                    .mt(px(4.))
                    .pt(px(8.))
                    .border_t_1()
                    .border_color(p.line)
                    .children(
                        card.transactions
                            .iter()
                            .take(3)
                            .enumerate()
                            .map(|(i, txn)| {
                                let progress = ease((activity - 0.16 - i as f32 * 0.055) / 0.21);
                                let avatar = div()
                                    .size(px(32.))
                                    .rounded_full()
                                    .flex_none()
                                    .overflow_hidden()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .bg(p.tray)
                                    .map(|e| {
                                        if let Some(source) = &txn.avatar {
                                            e.child(
                                                img(source.clone())
                                                    .size_full()
                                                    .object_fit(ObjectFit::Cover),
                                            )
                                        } else {
                                            e.child(icon(
                                                txn.icon.unwrap_or(Glyph::Card),
                                                15.,
                                                p.secondary,
                                            ))
                                        }
                                    });
                                let sign = if txn.amount > 0. { "+" } else { "" };
                                row()
                                    .h(px(42.))
                                    .gap(px(9.))
                                    .relative()
                                    .top(px((1. - progress) * 7.))
                                    .opacity(progress)
                                    .child(avatar)
                                    .child(
                                        column()
                                            .flex_1()
                                            .overflow_hidden()
                                            .gap(px(1.))
                                            .child(
                                                div()
                                                    .text_size(px(13.))
                                                    .line_height(px(16.))
                                                    .truncate()
                                                    .child(txn.merchant.clone()),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(11.))
                                                    .line_height(px(14.))
                                                    .text_color(p.faint)
                                                    .truncate()
                                                    .child(txn.detail.clone()),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .flex_none()
                                            .text_size(px(13.))
                                            .line_height(px(16.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(if txn.amount > 0. {
                                                p.success
                                            } else {
                                                p.ink
                                            })
                                            .child(format!("{sign}{}", self.amount(txn.amount))),
                                    )
                            }),
                    ),
            );
        }
        Some(details)
    }
}

impl Render for WalletStack {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let radius = surface_radius(cx);
        let room = self.room.get();
        let max = w.viewport_size().width.min(px(WIDTH));
        let width = f32::from(if room > px(0.) { room.min(max) } else { max });
        let h = width * ASPECT;
        let stage_h = h + 220.;
        let len = self.slots.len();
        let active_drag = self.drag.as_ref().filter(|d| d.axis == Axis::Vertical);
        let position = active_drag
            .filter(|d| !d.open)
            .map_or_else(|| self.position.get(w, cx), |d| d.value);
        let open = active_drag
            .filter(|d| d.open)
            .map_or_else(|| self.open.get(w, cx), |d| d.value)
            .clamp(0., 1.);
        let fan = self.fan.get(w, cx).clamp(0., 1.);
        let hover = self.hover.get(w, cx).clamp(0., 1.);
        let light = self.light.map(|s| s.get(w, cx).clamp(-1., 1.));
        let motion = !disabled_motion(cx);
        let shine = (motion && (self.device_light || hover > 0.001))
            .then(|| w.use_asset::<TextureAsset>(&Texture::Light, cx))
            .flatten();
        let label = self
            .label
            .clone()
            .unwrap_or_else(|| tr(cx, "Wallet", "钱包").into());
        let mut header = row()
            .w_full()
            .h(px(32.))
            .flex_none()
            .justify_between()
            .gap_2()
            .child(
                div()
                    .min_w_0()
                    .truncate()
                    .text_size(px(15.))
                    .child(label.clone()),
            );
        header = if self.opened {
            let all = tr(cx, "All cards", "所有卡片");
            header.child(
                row()
                    .id("wallet-back")
                    .role(Role::Button)
                    .aria_label(all)
                    .track_focus(&self.back)
                    .tab_stop(true)
                    .h(px(28.))
                    .flex_none()
                    .px_2()
                    .gap(px(4.))
                    .rounded_full()
                    .bg(p.panel)
                    .text_color(p.secondary)
                    .text_size(px(12.))
                    .cursor_pointer()
                    .hover(move |s| s.bg(p.hover).text_color(p.ink))
                    .focus_visible(move |s| {
                        s.shadow(vec![
                            BoxShadow::new(px(0.), px(0.), p.focus).spread_radius(px(1.)),
                        ])
                    })
                    .on_click(cx.listener(|s, _, w, cx| s.select(s.front, false, true, w, cx)))
                    .child(transformed(
                        Glyph::Arrow,
                        13.,
                        p.secondary,
                        std::f32::consts::PI,
                        1.,
                        (0., 0.),
                        1.,
                    ))
                    .child(all),
            )
        } else {
            let i = if len == 0 { 0 } else { self.front + 1 };
            let counter = if tr(cx, "en", "zh") == "zh" {
                format!("{i} / {len}")
            } else {
                format!("{i} of {len}")
            };
            header.child(
                div()
                    .flex_none()
                    .text_color(p.faint)
                    .text_size(px(12.))
                    .child(counter),
            )
        };

        let tracked = self.stage.clone();
        let update = cx.listener(|s, e: &MouseMoveEvent, _, cx| {
            let inside = s.stage.get().contains(&e.position);
            if inside != s.mouse_inside {
                s.hover_stage(inside, cx);
            }
            s.move_drag(e.position, false, cx);
        });
        let release = cx.listener(|s, e: &MouseUpEvent, w, cx| {
            if e.button == MouseButton::Left {
                s.move_drag(e.position, false, cx);
                s.finish(false, false, w, cx);
            }
        });
        let touch = cx.listener(|s, e: &TouchDragEvent, w, cx| match e.phase {
            TouchPhase::Started if s.stage.get().contains(&e.start_position) => {
                s.begin(e.start_position, true, w, cx);
                if s.drag.as_ref().is_some_and(|d| d.touch) {
                    w.prevent_default();
                    cx.stop_propagation();
                }
            }
            _ if s.drag.as_ref().is_none_or(|d| !d.touch) => {}
            TouchPhase::Moved => {
                s.move_drag(e.position, true, cx);
                cx.stop_propagation();
            }
            TouchPhase::Ended => {
                s.move_drag(e.position, true, cx);
                s.finish(true, false, w, cx);
                cx.stop_propagation();
            }
            TouchPhase::Cancelled => {
                s.finish(true, true, w, cx);
                cx.stop_propagation();
            }
            _ => {}
        });
        let capture = canvas(
            move |bounds, _, _| tracked.set(bounds),
            move |_, _, w, _| {
                w.on_mouse_event(move |e: &MouseMoveEvent, phase, w, cx| {
                    if phase.capture() {
                        update(e, w, cx);
                    }
                });
                w.on_mouse_event(move |e: &MouseUpEvent, phase, w, cx| {
                    if phase.capture() {
                        release(e, w, cx);
                    }
                });
                w.on_mouse_event(move |e: &TouchDragEvent, phase, w, cx| {
                    if phase.bubble() {
                        touch(e, w, cx);
                    }
                });
            },
        )
        .absolute()
        .inset_0();

        let approach = (self.front as f32 - position).abs();
        let reveal = ease((open - 0.72) / 0.28) * (1. - ease((approach - 0.04) / 0.40));
        // Draw from back to front in source order even while a card changes depth.
        let mut stack = div()
            .absolute()
            .top_0()
            .bottom_0()
            .left(px(-SHADOW_GUTTER))
            .right(px(-SHADOW_GUTTER))
            .overflow_hidden()
            .children(self.activity(h, reveal, w, cx));
        for i in (0..len).rev() {
            let slot = &self.slots[i];
            let front = self.front == i;
            let lift = if motion && self.highlight == Some(i) && active_drag.is_none() {
                hover
            } else {
                0.
            };
            let mut pose = card_pose(i, position, open, fan, h, stage_h, len);
            pose.top -= 8. * lift * (1. - open);
            let card_width = width * pose.scale;
            let card_height = h * pose.scale;
            let tilt = if front && motion && active_drag.is_none() {
                let strength = if self.device_light { 1. } else { hover };
                light.map(|v| v * strength)
            } else {
                [0.; 2]
            };
            let projected_width = card_width * (tilt[0] * 5_f32.to_radians()).cos();
            let projected_height = card_height * (tilt[1] * 5_f32.to_radians()).cos();
            let texture = w.use_asset::<TextureAsset>(&slot.card.variant.into(), cx);
            let image = slot
                .card
                .image
                .as_ref()
                .and_then(|image| resolve_image(image, w, cx));
            let art = CardArt {
                card: slot.card.clone(),
                texture,
                image,
                lift,
                radius,
                expiry_label: tr(cx, "Valid thru", "有效期"),
            };
            let suffix = slot.card.last_four();
            let accessible = format!(
                "{} · {} · {} {suffix} · {} / {} · {} {}",
                slot.card.issuer,
                slot.card.product,
                tr(cx, "ending in", "尾号"),
                i + 1,
                len,
                slot.card.balance_label,
                self.amount(slot.card.balance),
            );
            let bounds = slot.bounds.clone();
            stack =
                stack.child(
                    div()
                        .id(("wallet-card", i))
                        .debug_selector(move || format!("wallet-card-{i}"))
                        .role(Role::Button)
                        .aria_label(accessible)
                        .aria_expanded(self.opened && front)
                        .track_focus(&slot.focus.clone().tab_stop(front))
                        .block_mouse_except_scroll()
                        .absolute()
                        .left(px(SHADOW_GUTTER
                            + (width - projected_width) / 2.
                            + tilt[0] * 2.))
                        .top(px(pose.top + tilt[1] * 2.))
                        .w(px(projected_width))
                        .h(px(projected_height))
                        .rounded(radius)
                        .cursor_pointer()
                        .focus_visible(move |e| {
                            e.shadow(vec![
                                BoxShadow::new(px(0.), px(0.), p.focus).spread_radius(px(2.)),
                            ])
                        })
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|s, e: &MouseDownEvent, w, cx| {
                                s.begin(e.position, false, w, cx)
                            }),
                        )
                        .on_mouse_move(cx.listener(move |s, e: &MouseMoveEvent, _, cx| {
                            s.pointer(i, e.position, cx)
                        }))
                        .on_hover(cx.listener(move |s, hovered: &bool, _, cx| {
                            if !hovered && s.highlight == Some(i) {
                                s.hover.set(0., cx);
                                cx.notify();
                            }
                        }))
                        .on_click(cx.listener(move |s, event, w, cx| {
                            let distant = matches!(event, ClickEvent::Mouse(e)
        if (e.up.position.x - e.down.position.x).abs() >= px(5.)
            || (e.up.position.y - e.down.position.y).abs() >= px(5.));
                            if !s.suppress_click && !distant {
                                s.toggle(i, w, cx);
                            }
                        }))
                        .child(
                            canvas(
                                move |b, _, _| bounds.set(b),
                                move |b, _, w, cx| art.paint(b, w, cx),
                            )
                            .size_full(),
                        )
                        .when_some(front.then(|| shine.clone()).flatten(), |card, texture| {
                            card.child(
                                canvas(
                                    |_, _, _| (),
                                    move |b, _, w, _| paint_light(b, radius, tilt, texture, w),
                                )
                                .absolute()
                                .inset_0()
                                .opacity(if self.device_light { 1. } else { hover }),
                            )
                        }),
                );
        }

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
            .child(
                column()
                    .id(self.id.clone())
                    .role(Role::Group)
                    .aria_label(label)
                    .aria_description(tr(
                        cx,
                        "Use arrow keys to browse cards.",
                        "使用方向键浏览卡片。",
                    ))
                    .w_full()
                    .text_color(p.ink)
                    .on_key_down(cx.listener(|s, e, w, cx| s.key(e, w, cx)))
                    .child(header)
                    .child(
                        div()
                            .id("wallet-stage")
                            .relative()
                            .w_full()
                            .h(px(stage_h))
                            .mt(px(-2.))
                            .child(capture)
                            .child(stack),
                    )
                    .child(
                        div()
                            .id("wallet-announcement")
                            .role(Role::Status)
                            .aria_label(self.announcement.clone())
                            .size(px(0.)),
                    ),
            );
        match &self.style {
            Some(style) => style(shell),
            None => shell,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Texture, TextureAsset, WalletStack, WalletStackCard, WalletStackSelected};
    use crate::foundation::Theme;
    use gpui::{
        Context, KeyUpEvent, Keystroke, Modifiers, MouseButton, Styled, TestAppContext, TouchEvent,
        TouchId, TouchPhase, VisualTestContext, point, px,
    };
    use std::{cell::RefCell, rc::Rc, time::Duration};

    fn wallet(cx: &mut Context<WalletStack>) -> WalletStack {
        let cards = ["one", "two", "three", "four"]
            .into_iter()
            .map(|id| {
                WalletStackCard::new(id, id, "Debit", "1234", "A. Test", 1234.56, "Available")
            })
            .collect();
        WalletStack::new(cards, cx)
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
    fn pointer_light_does_not_reuse_the_black_card_asset() {
        let app = TestAppContext::single();
        app.update(|cx| {
            cx.fetch_asset::<TextureAsset>(&Texture::Black);
            assert!(!cx.has_asset::<TextureAsset>(&Texture::Light));
        });
    }

    #[test]
    fn keyboard_opens_cycles_and_returns_focus_with_one_event_per_selection() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (wallet, w) = app.add_window_view(|_, cx| wallet(cx));
        tick(w, 0);
        let history = Rc::new(RefCell::new(Vec::new()));
        let events = history.clone();
        w.update(|w, cx| {
            cx.subscribe(&wallet, move |_, e: &WalletStackSelected, _| {
                events.borrow_mut().push(e.0.as_ref().map(|c| c.id.clone()));
            })
            .detach();
            w.focus_next(cx);
        });
        tick(w, 0);
        assert!(w.update(|w, cx| wallet.read(cx).slots[0].focus.is_focused(w)));
        let closed = w.debug_bounds("wallet-card-0").unwrap();
        w.simulate_keystrokes("enter");
        w.simulate_event(KeyUpEvent {
            keystroke: Keystroke::parse("enter").unwrap(),
        });
        tick(w, 750);
        let open = w.debug_bounds("wallet-card-0").unwrap();
        assert!(open.top() < closed.top());
        assert!(open.size.width < closed.size.width);
        w.simulate_keystrokes("right");
        tick(w, 750);
        w.simulate_keystrokes("escape");
        tick(w, 750);
        w.update(|w, cx| {
            let wallet = wallet.read(cx);
            assert_eq!(
                wallet.front_card().map(|card| card.id.as_ref()),
                Some("two")
            );
            assert!(wallet.selected_card().is_none());
            assert!(wallet.slots[1].focus.is_focused(w));
        });
        assert_eq!(
            *history.borrow(),
            vec![Some("one".into()), Some("two".into()), None]
        );
    }

    #[test]
    fn mouse_drag_tracks_the_pointer_and_moves_at_most_one_card() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (wallet, w) = app.add_window_view(|_, cx| wallet(cx));
        tick(w, 0);
        let start = w.debug_bounds("wallet-card-0").unwrap();
        let from = start.center();
        let mid = from + point(px(0.), px(85.));
        let end = from + point(px(0.), px(320.));
        w.simulate_mouse_down(from, MouseButton::Left, Modifiers::default());
        w.simulate_mouse_move(mid, MouseButton::Left, Modifiers::default());
        tick(w, 18);
        assert!(w.debug_bounds("wallet-card-0").unwrap().top() > start.top() + px(20.));
        w.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
        tick(w, 750);
        w.update(|_, cx| {
            let wallet = wallet.read(cx);
            assert_eq!(wallet.front, 1);
            assert!(wallet.selected_card().is_none());
        });
        let next = w.debug_bounds("wallet-card-1").unwrap().center();
        w.simulate_mouse_move(next, None, Modifiers::default());
        tick(w, 750);
        let next = w.debug_bounds("wallet-card-1").unwrap().center();
        w.simulate_mouse_down(next, MouseButton::Left, Modifiers::default());
        w.simulate_mouse_up(next, MouseButton::Left, Modifiers::default());
        tick(w, 750);
        assert!(
            w.update(|_, cx| wallet
                .read(cx)
                .selected_card()
                .is_some_and(|c| c.id.as_ref() == "two")),
            "click should open the second card"
        );
        let top = w.debug_bounds("wallet-card-1").unwrap().center();
        w.simulate_mouse_move(top, None, Modifiers::default());
        tick(w, 0);
        w.simulate_mouse_down(top, MouseButton::Left, Modifiers::default());
        let down = top + point(px(0.), px(140.));
        w.simulate_mouse_move(down, MouseButton::Left, Modifiers::default());
        tick(w, 16);
        w.simulate_mouse_up(down, MouseButton::Left, Modifiers::default());
        tick(w, 750);
        assert!(w.update(|_, cx| wallet.read(cx).selected_card().is_none()));
    }

    #[test]
    fn direct_touch_taps_or_cancels_without_a_second_activation_in_reduced_motion() {
        let mut app = TestAppContext::single();
        app.update(|cx| {
            crate::init(cx);
            Theme::global_mut(cx).reduced_motion = true;
        });
        let (wallet, w) = app.add_window_view(|_, cx| wallet(cx).with_style(|e| e.w(px(256.))));
        tick(w, 0);
        let first = w.debug_bounds("wallet-card-0").unwrap();
        assert!((first.size.width - px(256.)).abs() < px(1.));
        let touch = |w: &mut VisualTestContext, phase, position| {
            w.simulate_event(TouchEvent {
                id: TouchId(1),
                phase,
                position,
                ..Default::default()
            });
            tick(w, 16);
        };
        touch(w, TouchPhase::Started, first.center());
        touch(w, TouchPhase::Cancelled, first.center());
        assert!(w.update(|_, cx| wallet.read(cx).selected_card().is_none()));
        touch(w, TouchPhase::Started, first.center());
        touch(w, TouchPhase::Ended, first.center());
        w.update(|_, cx| {
            let wallet = wallet.read(cx);
            assert_eq!(
                wallet.selected_card().map(|card| card.id.as_ref()),
                Some("one")
            );
            assert_eq!(wallet.open.value(cx), 1.);
        });
    }
}
