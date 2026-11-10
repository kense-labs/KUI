//! An animated illustration of card authorization, settlement, and payout.
use crate::{
    foundation::Theme,
    primitives::{
        currency::format_money,
        family::{Paint, Slide, disabled_motion},
        glyph::{Glyph, icon},
        presentation::tr,
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

const CYCLE: f64 = 15.3;
const AUTH_START: f32 = 1.56;
const AUTH_END: f32 = 7.86;
const SETTLEMENT: f32 = 9.36;
const FEE: f32 = 11.71;
const PAYOUT: f32 = 13.4;

/// A stop in the payment diagram.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaymentFlowStop {
    Terminal,
    Processor,
    Network,
    Issuer,
    Merchant,
}

/// Optional presentation overrides for a stop.
#[derive(Clone, Debug, Default)]
pub struct PaymentFlowNode {
    pub label: Option<SharedString>,
    pub meta: Option<SharedString>,
    pub detail: Option<SharedString>,
    pub icon: Option<Glyph>,
    pub color: Option<Hsla>,
}

/// Optional labels for the progress rows and approval chip.
#[derive(Clone, Debug, Default)]
pub struct PaymentFlowLabels {
    pub authorization: Option<SharedString>,
    pub settlement: Option<SharedString>,
    pub settlement_time: Option<SharedString>,
    pub approved: Option<SharedString>,
}

/// Configures the illustration. Monetary values are in major currency units.
#[derive(Clone, Debug)]
pub struct PaymentFlowOptions {
    pub nodes: [PaymentFlowNode; 5],
    pub amount: f64,
    pub fee: f64,
    pub balance: f64,
    pub currency: SharedString,
    pub locale: SharedString,
    pub authorization_ms: u32,
    pub labels: PaymentFlowLabels,
    pub hide_steps: bool,
    pub speed: f32,
    pub paused: bool,
    pub accent: Option<Hsla>,
    pub label: Option<SharedString>,
}

impl Default for PaymentFlowOptions {
    fn default() -> Self {
        Self {
            nodes: Default::default(),
            amount: 42.5,
            fee: 1.53,
            balance: 1284.1,
            currency: "USD".into(),
            locale: "en-US".into(),
            authorization_ms: 1340,
            labels: Default::default(),
            hide_steps: false,
            speed: 1.,
            paused: false,
            accent: None,
            label: None,
        }
    }
}

impl PaymentFlowOptions {
    /// Overrides one named stop.
    pub fn with_node(mut self, stop: PaymentFlowStop, node: PaymentFlowNode) -> Self {
        self.nodes[stop as usize] = node;
        self
    }
}

#[derive(Clone, Copy)]
struct Frame {
    cycle: f64,
    time: f32,
}

impl Frame {
    fn at(elapsed: f64) -> Self {
        Self {
            cycle: (elapsed / CYCLE).floor(),
            time: elapsed.rem_euclid(CYCLE) as f32,
        }
    }

    fn between(self, start: f32, end: f32) -> f32 {
        ((self.time - start) / (end - start)).clamp(0., 1.)
    }

    fn motion(self, start: f32, end: f32) -> f32 {
        let t = self.between(start, end);
        t * t * (3. - 2. * t)
    }

    fn balance(self, initial: f64, net: f64) -> f64 {
        initial + (self.cycle + f64::from(self.motion(PAYOUT, 14.82))) * net
    }

    fn chip(self) -> Option<(Chip, f32)> {
        Some(match self.time {
            t if (AUTH_START..4.72).contains(&t) => (
                Chip::Amount,
                (self.motion(AUTH_START, 2.60) + self.motion(2.63, 3.85) + self.motion(3.86, 4.72))
                    / 3.,
            ),
            t if (5.60..7.86).contains(&t) => (Chip::Approval, 1. - self.motion(5.60, 7.86)),
            t if (SETTLEMENT..PAYOUT).contains(&t) => (Chip::Settlement, 0.),
            _ => return None,
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Chip {
    Amount,
    Approval,
    Settlement,
}

#[derive(Clone, Copy)]
struct Coordinate {
    x: f32,
    y: f32,
}

impl Coordinate {
    fn at(self, to: Self, t: f32) -> Self {
        xy(self.x + (to.x - self.x) * t, self.y + (to.y - self.y) * t)
    }

    fn distance(self, to: Self) -> f32 {
        (self.x - to.x).hypot(self.y - to.y)
    }
}

fn xy(x: f32, y: f32) -> Coordinate {
    Coordinate { x, y }
}

struct Route {
    points: Vec<(Coordinate, f32)>,
    length: f32,
}

impl Route {
    fn new(points: impl IntoIterator<Item = Coordinate>) -> Self {
        let mut route = Self {
            points: Vec::new(),
            length: 0.,
        };
        for point in points {
            if let Some((last, _)) = route.points.last() {
                route.length += last.distance(point);
            }
            route.points.push((point, route.length));
        }
        route
    }

    fn at(&self, distance: f32) -> Coordinate {
        let distance = distance.clamp(0., self.length);
        let index = self
            .points
            .partition_point(|(_, d)| *d < distance)
            .clamp(1, self.points.len() - 1);
        let (a, da) = self.points[index - 1];
        let (b, db) = self.points[index];
        a.at(
            b,
            if db > da {
                (distance - da) / (db - da)
            } else {
                0.
            },
        )
    }

    fn paint(&self, from: f32, to: f32, color: Hsla, origin: gpui::Point<Pixels>, w: &mut Window) {
        if to <= from {
            return;
        }
        let translated = |p: Coordinate| origin + point(px(p.x), px(p.y));
        let mut path = PathBuilder::stroke(px(1.25));
        path.move_to(translated(self.at(from)));
        for (point, d) in &self.points {
            if *d > from && *d < to {
                path.line_to(translated(*point));
            }
        }
        path.line_to(translated(self.at(to)));
        if let Ok(path) = path.build() {
            w.paint_path(path, color);
        }
    }
}

#[derive(Clone, Copy)]
struct Card {
    origin: Coordinate,
    width: f32,
    height: f32,
}

impl Card {
    fn center(self) -> Coordinate {
        xy(
            self.origin.x + self.width / 2.,
            self.origin.y + self.height / 2.,
        )
    }
}

struct Layout {
    width: f32,
    height: f32,
    cards: [Card; 5],
    auth: Route,
    settle: Route,
    network: f32,
    processor: f32,
    stubs: [Route; 2],
}

impl Layout {
    fn new(width: f32) -> Self {
        let vertical = width < 700.;
        let height = if vertical { 391. } else { 187. };
        let cards = if vertical {
            let left = (width * 0.46).round();
            let right = (width * 0.51).round();
            std::array::from_fn(|i| Card {
                origin: if i < 4 {
                    xy(1., 10. + i as f32 * 108.)
                } else {
                    xy(width - right - 1., 10.)
                },
                width: if i < 4 { left } else { right },
                height: 56.,
            })
        } else {
            let card_width = 150.;
            std::array::from_fn(|i| Card {
                origin: xy(
                    if i < 4 {
                        1. + i as f32 * (width - card_width - 2.) / 3.
                    } else {
                        1.
                    },
                    if i < 4 { 10. } else { 126. },
                ),
                width: card_width,
                height: 60.,
            })
        };
        let [terminal, processor, network, issuer, merchant] = cards.map(Card::center);
        let (entry, corner, exit, net, fee) = if vertical {
            (
                xy(merchant.x - 18., issuer.y),
                xy(merchant.x, issuer.y),
                xy(merchant.x, issuer.y - 18.),
                xy(merchant.x, network.y),
                xy(merchant.x, processor.y),
            )
        } else {
            (
                xy(issuer.x, merchant.y - 18.),
                xy(issuer.x, merchant.y),
                xy(issuer.x - 18., merchant.y),
                xy(network.x, merchant.y),
                xy(processor.x, merchant.y),
            )
        };
        let settle = Route::new(
            [issuer, entry]
                .into_iter()
                .chain((1..=12).map(|i| {
                    let t = i as f32 / 12.;
                    entry.at(corner, t).at(corner.at(exit, t), t)
                }))
                .chain([net, fee, merchant]),
        );
        let n = settle.points.len();
        let stubs = std::array::from_fn(|i| {
            let card = cards[i + 1];
            let center = card.center();
            Route::new(if vertical {
                [
                    xy(card.origin.x + card.width, center.y),
                    xy(merchant.x - 4., center.y),
                ]
            } else {
                [
                    xy(center.x, card.origin.y + card.height),
                    xy(center.x, merchant.y - 4.),
                ]
            })
        });
        Self {
            width,
            height,
            cards,
            auth: Route::new([terminal, processor, network, issuer]),
            network: settle.points[n - 3].1,
            processor: settle.points[n - 2].1,
            settle,
            stubs,
        }
    }

    fn settlement_position(&self, frame: Frame) -> f32 {
        self.network * frame.motion(SETTLEMENT, 10.71)
            + (self.processor - self.network) * frame.motion(10.98, 11.77)
            + (self.settle.length - self.processor) * frame.motion(12.16, PAYOUT)
    }

    fn paint(
        &self,
        frame: Frame,
        origin: gpui::Point<Pixels>,
        paint: Paint,
        accent: Hsla,
        hovered: Option<usize>,
        w: &mut Window,
    ) {
        let base = paint
            .faint
            .opacity(if hovered.is_some() { 0.63 } else { 0.35 });
        for rail in [&self.auth, &self.settle] {
            rail.paint(0., rail.length, base, origin, w);
        }
        let mut dashes = PathBuilder::stroke(px(1.25));
        for stub in &self.stubs {
            for start in (0..stub.length.ceil() as usize).step_by(6) {
                let a = stub.at(start as f32);
                let b = stub.at(start as f32 + 2.);
                dashes.move_to(origin + point(px(a.x), px(a.y)));
                dashes.line_to(origin + point(px(b.x), px(b.y)));
            }
        }
        if let Ok(dashes) = dashes.build() {
            w.paint_path(dashes, base);
        }
        let auth = self.auth.length;
        if frame.time < SETTLEMENT {
            let charge = frame.chip().filter(|(chip, _)| *chip == Chip::Amount);
            let amount = charge.map_or_else(|| frame.between(4.70, 4.72), |(_, t)| t);
            self.auth
                .paint(0., auth * amount, accent.opacity(0.68), origin, w);
            let approved = frame.chip().filter(|(chip, _)| *chip == Chip::Approval);
            let back = approved.map_or_else(|| 1. - frame.between(7.84, 7.86), |(_, t)| t);
            self.auth
                .paint(auth * back, auth, paint.success.opacity(0.73), origin, w);
        }
        let settled = self.settlement_position(frame);
        self.settle
            .paint(0., settled, accent.opacity(0.70), origin, w);
    }
}

type Style = Box<dyn Fn(Div) -> Div>;
type Visibility = Box<dyn Fn(&App) -> bool>;
type Mark = Box<dyn Fn() -> AnyElement>;
type FormatMoney = Box<dyn Fn(f64, &str, &str) -> SharedString>;

/// Renders a simulated card payment as a single accessible figure.
pub struct PaymentFlow {
    options: PaymentFlowOptions,
    id: ElementId,
    visible: bool,
    visibility: Option<Visibility>,
    style: Option<Style>,
    caption: Option<SharedString>,
    marks: [Option<Mark>; 5],
    formatter: Option<FormatMoney>,
    width: Rc<Cell<Pixels>>,
    layout: Rc<Layout>,
    hovered: Option<usize>,
    pace: Slide,
    last: Option<(Instant, f32)>,
    elapsed: f64,
}

fn playback_speed(value: f32) -> f32 {
    if value.is_finite() { value.max(0.) } else { 1. }
}

impl PaymentFlow {
    pub fn new(options: PaymentFlowOptions, _: &mut Context<Self>) -> Self {
        let pace = if options.paused {
            0.
        } else {
            playback_speed(options.speed)
        };
        Self {
            options,
            id: "payment-flow".into(),
            visible: true,
            visibility: None,
            style: None,
            caption: None,
            marks: Default::default(),
            formatter: None,
            width: Rc::default(),
            layout: Rc::new(Layout::new(761.)),
            hovered: None,
            pace: Slide::new(pace),
            last: None,
            elapsed: 0.,
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

    pub fn with_caption(mut self, caption: impl Into<SharedString>) -> Self {
        self.caption = Some(caption.into());
        self
    }

    /// Provides a custom GPUI image or vector element for one stop.
    pub fn with_mark(
        mut self,
        stop: PaymentFlowStop,
        mark: impl Fn() -> AnyElement + 'static,
    ) -> Self {
        self.marks[stop as usize] = Some(Box::new(mark));
        self
    }

    /// Uses host formatting for amounts, including locales outside the built-in set.
    pub fn with_money_formatter(
        mut self,
        format: impl Fn(f64, &str, &str) -> SharedString + 'static,
    ) -> Self {
        self.formatter = Some(Box::new(format));
        self
    }

    /// Pauses the flow when the host hides the figure.
    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        if self.visible != visible {
            self.visible = visible;
            self.last = None;
            cx.notify();
        }
    }

    /// Eases to a new playback rate. Zero holds the current phase.
    pub fn set_speed(&mut self, speed: f32, cx: &mut Context<Self>) {
        self.options.speed = playback_speed(speed);
        self.retarget(cx);
    }

    /// Eases the current playback rate to or from zero.
    pub fn set_paused(&mut self, paused: bool, cx: &mut Context<Self>) {
        self.options.paused = paused;
        self.retarget(cx);
    }

    fn retarget(&mut self, cx: &mut Context<Self>) {
        let speed = playback_speed(self.options.speed);
        self.pace.set(
            if self.options.paused {
                0.
            } else if self.hovered.is_some() {
                speed * 0.65
            } else {
                speed
            },
            cx,
        );
        cx.notify();
    }

    fn frame(&mut self, w: &Window, cx: &mut Context<Self>) -> Frame {
        if disabled_motion(cx) {
            self.last = None;
            return Frame {
                cycle: (self.elapsed / CYCLE).floor(),
                time: 12.65,
            };
        }
        if !self.visible
            || !w.is_visible()
            || self.visibility.as_ref().is_some_and(|visible| !visible(cx))
        {
            self.last = None;
            return Frame::at(self.elapsed);
        }

        let now = cx.background_executor().now();
        let (rate, changing) = self.pace.sample_batch(now, false);
        let rate = rate.max(0.);
        if let Some((last, prior_rate)) = self.last.replace((now, rate)) {
            let dt = now.saturating_duration_since(last).as_secs_f64().min(0.10);
            self.elapsed += dt * f64::from((prior_rate + rate) / 2.);
        }
        if rate > 0. || changing {
            let weak = cx.weak_entity();
            w.on_next_frame(move |_, cx| {
                let _ = weak.update(cx, |_, cx| cx.notify());
            });
        } else {
            self.last = None;
        }
        Frame::at(self.elapsed)
    }

    fn money(&self, value: f64) -> SharedString {
        if let Some(format) = &self.formatter {
            return format(value, &self.options.currency, &self.options.locale);
        }
        format_money(value, &self.options.currency, &self.options.locale).into()
    }

    fn labels(&self, cx: &App) -> [SharedString; 5] {
        let defaults = [
            tr(cx, "Checkout", "结账"),
            "Stripe",
            "Mastercard",
            tr(cx, "Bank", "银行"),
            tr(cx, "Your balance", "你的余额"),
        ];
        std::array::from_fn(|i| {
            self.options.nodes[i]
                .label
                .clone()
                .unwrap_or_else(|| defaults[i].into())
        })
    }

    fn status(
        &self,
        i: usize,
        frame: Frame,
        balance: &SharedString,
        cx: &App,
    ) -> (SharedString, bool, bool) {
        let t = frame.time;
        let lang = |en, zh| SharedString::from(tr(cx, en, zh));
        let (status, success) = match i {
            0 if t >= 7.72 => (lang("Approved", "已授权"), true),
            0 if t >= 1.76 => (lang("Authorizing", "正在授权"), false),
            0 if t >= 0.92 => (lang("Card tapped", "已感应卡片"), false),
            1 if t >= FEE => (
                format!(
                    "{} {}",
                    tr(cx, "Fee", "手续费"),
                    self.money(self.options.fee)
                )
                .into(),
                false,
            ),
            1 if t >= 2.76 => (lang("Risk checked", "风控已通过"), false),
            1 if t >= 2.11 => (lang("Checking risk", "正在检查风险"), false),
            2 if t >= 11.02 => (lang("Cleared", "已清算"), true),
            2 if t >= 10.44 => (lang("Clearing", "正在清算"), false),
            2 if t >= 3.97 => (lang("Routed to bank", "已路由到银行"), false),
            2 if t >= 3.32 => (lang("Routing", "正在路由"), false),
            3 if t >= 9.83 => (lang("Settled", "已结算"), true),
            3 if t >= SETTLEMENT => (lang("Settling", "正在结算"), false),
            3 if t >= 6.48 => (lang("Funds held", "资金已冻结"), true),
            3 if t >= 5.60 => (lang("Approved", "已授权"), true),
            3 if t >= 4.56 => (lang("Checking funds", "正在检查资金"), false),
            4 => (balance.clone(), t >= PAYOUT),
            _ => {
                let meta = [
                    tr(cx, "Tap to pay", "感应付款"),
                    tr(cx, "Processor", "支付处理商"),
                    tr(cx, "Card network", "卡组织"),
                    tr(cx, "Card issuer", "发卡行"),
                ];
                return (
                    self.options.nodes[i]
                        .meta
                        .clone()
                        .unwrap_or_else(|| meta[i].into()),
                    false,
                    false,
                );
            }
        };
        (status, success, i != 4 || success)
    }
}

fn tabular() -> FontFeatures {
    FontFeatures(vec![("tnum".into(), 1)].into())
}

fn one_line() -> Div {
    div()
        .min_w(px(0.))
        .whitespace_nowrap()
        .overflow_hidden()
        .text_ellipsis()
}

fn outline(strokes: &'static [&'static [(f32, f32)]], color: Hsla) -> impl IntoElement {
    canvas(
        |_, _, _| {},
        move |bounds, _, w, _| {
            for stroke in strokes {
                let mut path = PathBuilder::stroke(px(1.1));
                for (i, &(x, y)) in stroke.iter().enumerate() {
                    let point = bounds.origin + point(px(x), px(y));
                    if i == 0 {
                        path.move_to(point);
                    } else {
                        path.line_to(point);
                    }
                }
                if let Ok(path) = path.build() {
                    w.paint_path(path, color);
                }
            }
        },
    )
    .size(px(16.))
}

fn default_mark(i: usize, color: Hsla, custom_color: bool) -> AnyElement {
    match i {
        0 => outline(
            &[
                &[(3., 6.), (3.7, 8.), (3., 10.)],
                &[(5.4, 4.7), (6.6, 8.), (5.4, 11.3)],
                &[(7.8, 3.4), (9.4, 8.), (7.8, 12.6)],
                &[(10.2, 2.), (12.2, 8.), (10.2, 14.)],
            ],
            color,
        )
        .into_any_element(),
        1 => div()
            .size(px(16.))
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(20.))
            .font_weight(FontWeight::EXTRA_BOLD)
            .text_color(if custom_color {
                color
            } else {
                rgb(0x7770ff).into()
            })
            .child("S")
            .into_any_element(),
        2 => div()
            .relative()
            .size(px(16.))
            .child(
                div()
                    .absolute()
                    .left_0()
                    .top(px(3.))
                    .size(px(10.))
                    .rounded_full()
                    .bg(if custom_color {
                        color
                    } else {
                        rgb(0xeb001b).into()
                    }),
            )
            .child(
                div()
                    .absolute()
                    .right_0()
                    .top(px(3.))
                    .size(px(10.))
                    .rounded_full()
                    .bg(if custom_color {
                        color.opacity(0.72)
                    } else {
                        rgb(0xffa000).into()
                    }),
            )
            .into_any_element(),
        3 => outline(
            &[
                &[(1.7, 5.7), (8., 2.4), (14.3, 5.7), (1.7, 5.7)],
                &[(3.5, 7.), (3.5, 12.)],
                &[(6.5, 7.), (6.5, 12.)],
                &[(9.5, 7.), (9.5, 12.)],
                &[(12.5, 7.), (12.5, 12.)],
                &[(1.7, 13.5), (14.3, 13.5)],
            ],
            color,
        )
        .into_any_element(),
        _ => outline(
            &[
                &[
                    (2., 6.),
                    (4., 2.5),
                    (12., 2.5),
                    (14., 6.),
                    (13., 8.),
                    (11., 8.),
                    (9.5, 7.),
                    (8., 8.),
                    (6.5, 7.),
                    (5., 8.),
                    (3., 8.),
                    (2., 6.),
                ],
                &[(3., 8.), (3., 13.5), (13., 13.5), (13., 8.)],
                &[(6.5, 13.5), (6.5, 10.), (9.5, 10.), (9.5, 13.5)],
            ],
            color,
        )
        .into_any_element(),
    }
}

fn chip_face(
    label: SharedString,
    center: Coordinate,
    opacity: f32,
    (ink, paint, radius): (Hsla, Paint, Pixels),
    approval: bool,
    w: &Window,
) -> Div {
    let width =
        crate::primitives::family::measure(&label, 11., w) + if approval { 28. } else { 14. };
    div()
        .absolute()
        .left(px(center.x - width / 2.))
        .top(px(center.y - 10.))
        .w(px(width))
        .h(px(20.))
        .rounded(radius)
        .border_1()
        .border_color(ink.opacity(if approval { 0.4 } else { 0.22 }))
        .bg(paint
            .panel
            .blend(ink.opacity(if approval { 0.11 } else { 0.045 })))
        .opacity(opacity)
        .flex()
        .gap(px(4.))
        .items_center()
        .justify_center()
        .text_size(px(11.))
        .line_height(px(16.))
        .font_features(tabular())
        .text_color(if approval { ink } else { paint.ink })
        .when(approval, |e| e.child(icon(Glyph::Check, 12., ink)))
        .child(one_line().child(label))
}

fn step(title: SharedString, metric: SharedString, progress: f32, paint: Paint, ink: Hsla) -> Div {
    div()
        .flex_1()
        .min_w(px(0.))
        .min_h(px(30.))
        .h(px(30.))
        .relative()
        .flex()
        .justify_between()
        .gap(px(8.))
        .text_size(px(14.))
        .line_height(px(18.))
        .child(one_line().child(title))
        .child(
            one_line()
                .font_features(tabular())
                .text_size(px(12.))
                .text_color(paint.faint)
                .child(metric),
        )
        .child(
            div()
                .absolute()
                .bottom_0()
                .left_0()
                .right_0()
                .h(px(2.))
                .rounded_full()
                .overflow_hidden()
                .bg(paint.line)
                .child(div().w(relative(progress)).h_full().rounded_full().bg(ink)),
        )
}

impl Render for PaymentFlow {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let paint = Paint::get(cx);
        let radius = Theme::global(cx).tokens.radius.md;
        let accent = self.options.accent.unwrap_or(paint.bright);
        let width = if self.width.get() > px(0.) {
            f32::from(self.width.get()).clamp(1., 761.)
        } else {
            f32::from(w.viewport_size().width).clamp(1., 761.)
        };
        if self.layout.width != width {
            self.layout = Rc::new(Layout::new(width));
        }
        let layout = self.layout.clone();
        let frame = self.frame(w, cx);
        let amount = self.money(self.options.amount);
        let net_amount = self.options.amount - self.options.fee;
        let net = self.money(net_amount);
        let balance = self.money(frame.balance(self.options.balance, net_amount));
        let labels = self.labels(cx);
        let label = self.options.label.clone().unwrap_or_else(|| {
            format!(
                "{} {} → {} → {} → {}. {}. {} {}. {} → {}.",
                tr(cx, "Simulated payment", "模拟付款"),
                amount,
                labels[0],
                labels[1],
                labels[2],
                labels[3],
                tr(cx, "Approved", "授权通过"),
                self.money(self.options.fee),
                net,
                labels[4],
            )
            .into()
        });
        let hovered = self.hovered;
        let graph = div()
            .relative()
            .w_full()
            .flex_none()
            .h(px(layout.height))
            .child({
                let layout = layout.clone();
                canvas(
                    |_, _, _| {},
                    move |bounds, _, w, _| {
                        layout.paint(frame, bounds.origin, paint, accent, hovered, w)
                    },
                )
                .absolute()
                .inset_0()
                .size_full()
            })
            .when_some(frame.chip(), |e, (chip, progress)| match chip {
                Chip::Amount => e.child(chip_face(
                    amount,
                    layout.auth.at(layout.auth.length * progress),
                    1.,
                    (accent, paint, radius),
                    false,
                    w,
                )),
                Chip::Approval => e.child(chip_face(
                    self.options
                        .labels
                        .approved
                        .clone()
                        .unwrap_or_else(|| tr(cx, "Approved", "已授权").into()),
                    layout.auth.at(layout.auth.length * progress),
                    1.,
                    (paint.success, paint, radius),
                    true,
                    w,
                )),
                Chip::Settlement => {
                    let center = layout.settle.at(layout.settlement_position(frame));
                    let crossfade = frame.motion(11.92, 12.10);
                    e.when(crossfade < 1., |e| {
                        e.child(chip_face(
                            amount,
                            center,
                            1. - crossfade,
                            (accent, paint, radius),
                            false,
                            w,
                        ))
                    })
                    .when(crossfade > 0., |e| {
                        e.child(chip_face(
                            net,
                            center,
                            crossfade,
                            (accent, paint, radius),
                            false,
                            w,
                        ))
                    })
                }
            })
            .children(layout.cards.iter().copied().enumerate().map(|(i, card)| {
                let node = &self.options.nodes[i];
                let mark_color = node.color.unwrap_or(paint.secondary);
                let (status, success, active) = self.status(i, frame, &balance, cx);
                let over = hovered == Some(i);
                let mark = self.marks[i].as_ref().map_or_else(
                    || {
                        node.icon.map_or_else(
                            || default_mark(i, mark_color, node.color.is_some()),
                            |glyph| icon(glyph, 16., mark_color).into_any_element(),
                        )
                    },
                    |mark| mark(),
                );
                let pad = if card.width >= 150. { 16. } else { 8. };
                div()
                    .id(("payment-node", i))
                    .debug_selector(move || format!("payment-node-{i}"))
                    .absolute()
                    .left(px(card.origin.x))
                    .top(px(card.origin.y))
                    .w(px(card.width))
                    .h(px(card.height))
                    .px(px(pad))
                    .flex()
                    .items_center()
                    .gap(px(if card.width >= 150. { 12. } else { 7. }))
                    .border_1()
                    .border_color(if over {
                        accent.opacity(0.48)
                    } else if success {
                        paint.success.opacity(0.58)
                    } else if active {
                        paint.ink.opacity(0.62)
                    } else {
                        paint.line
                    })
                    .rounded(radius)
                    .bg(if over {
                        paint.panel.blend(accent.opacity(0.065))
                    } else {
                        paint.panel
                    })
                    .on_hover(cx.listener(move |this, over: &bool, _, cx| {
                        if *over {
                            this.hovered = Some(i);
                        } else if this.hovered == Some(i) {
                            this.hovered = None;
                        }
                        this.retarget(cx);
                    }))
                    .child(div().flex_none().size(px(16.)).child(mark))
                    .child(
                        div()
                            .min_w(px(0.))
                            .flex()
                            .flex_col()
                            .child(
                                one_line()
                                    .text_size(px(14.))
                                    .line_height(px(18.))
                                    .child(labels[i].clone()),
                            )
                            .child(
                                div()
                                    .min_w(px(0.))
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .text_size(px(12.))
                                    .line_height(px(16.))
                                    .text_color(if success {
                                        paint.success
                                    } else if active {
                                        paint.secondary
                                    } else {
                                        paint.faint
                                    })
                                    .child(div().size(px(6.)).flex_none().rounded_full().bg(
                                        if success {
                                            paint.success
                                        } else if active {
                                            paint.secondary
                                        } else {
                                            paint.faint.opacity(0.5)
                                        },
                                    ))
                                    .child(one_line().font_features(tabular()).child(status)),
                            ),
                    )
            }))
            .when_some(hovered, |e, i| {
                let card = layout.cards[i];
                let node = &self.options.nodes[i];
                let detail = node.detail.clone().unwrap_or_else(|| {
                    [
                        tr(cx, "Customer taps their card", "顾客感应银行卡"),
                        tr(cx, "Checks the payment for risk", "检查付款风险"),
                        tr(cx, "Routes to the issuing bank", "将请求转发给发卡行"),
                        tr(cx, "Approves and settles the payment", "授权并结算付款"),
                        tr(cx, "Receives the net amount", "收到扣费后的金额"),
                    ][i]
                        .into()
                });
                let width = (crate::primitives::family::measure(&detail, 11., w) + 16.).min(width);
                e.child(
                    one_line()
                        .absolute()
                        .top(px(card.origin.y - 24.))
                        .left(px((card.center().x - width / 2.)
                            .clamp(0., (layout.width - width).max(0.))))
                        .w(px(width))
                        .h(px(20.))
                        .px(px(7.))
                        .rounded(radius)
                        .border_1()
                        .border_color(paint.line)
                        .bg(paint.panel)
                        .text_color(paint.secondary)
                        .text_size(px(11.))
                        .line_height(px(18.))
                        .text_center()
                        .child(detail),
                )
            });
        let figure =
            div()
                .id("payment-flow-figure")
                .role(Role::Image)
                .aria_label(label)
                .w_full()
                .flex_none()
                .flex()
                .flex_col()
                .gap(px(24.))
                .child(graph)
                .when(!self.options.hide_steps, |e| {
                    let auth = frame.between(AUTH_START, AUTH_END);
                    let elapsed = (f64::from(auth) * f64::from(self.options.authorization_ms))
                        .round()
                        / 1000.;
                    e.child(
                        div()
                            .flex()
                            .when(width < 420., |e| e.flex_col().gap(px(16.)))
                            .when(width >= 420., |e| e.gap(px(24.)))
                            .w_full()
                            .children([
                                step(
                                    self.options
                                        .labels
                                        .authorization
                                        .clone()
                                        .unwrap_or_else(|| tr(cx, "Authorization", "授权").into()),
                                    format!("{elapsed:.2} s").into(),
                                    auth,
                                    paint,
                                    paint.secondary,
                                ),
                                step(
                                    self.options
                                        .labels
                                        .settlement
                                        .clone()
                                        .unwrap_or_else(|| tr(cx, "Settlement", "结算").into()),
                                    self.options.labels.settlement_time.clone().unwrap_or_else(
                                        || tr(cx, "Next business day", "下一个工作日").into(),
                                    ),
                                    frame.between(SETTLEMENT, PAYOUT),
                                    paint,
                                    accent,
                                ),
                            ]),
                    )
                });
        let width_cell = self.width.clone();
        let root = div()
            .w_full()
            .max_w(px(761.))
            .min_w(px(0.))
            .flex()
            .flex_col()
            .gap(px(12.));
        let root = if let Some(style) = &self.style {
            style(root)
        } else {
            root
        };
        root.on_children_prepainted(move |children, w, _| {
            if let Some(figure) = children.first()
                && figure.size.width != width_cell.replace(figure.size.width)
            {
                w.request_animation_frame();
            }
        })
        .id(self.id.clone())
        .child(figure)
        .when_some(self.caption.clone(), |e, caption| {
            e.child(
                div()
                    .text_size(px(12.))
                    .text_color(paint.secondary)
                    .child(caption),
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{CYCLE, Chip, Frame, Layout, PaymentFlowOptions, format_money};

    #[test]
    fn fee_conversion_and_payout_are_continuous_across_cycles() {
        let options = PaymentFlowOptions::default();
        let net = options.amount - options.fee;
        let before = Frame::at(CYCLE - 0.01).balance(options.balance, net);
        let next = Frame::at(CYCLE).balance(options.balance, net);
        assert!((before - next).abs() < 0.001);
        assert_eq!(format_money(next, "USD", "en-US"), "$1,325.07");
        assert!(Frame::at(14.).balance(options.balance, net) > options.balance);
        assert!(matches!(Frame::at(6.).chip(), Some((Chip::Approval, _))));
        assert!(matches!(Frame::at(12.).chip(), Some((Chip::Settlement, _))));
        assert!(Frame::at(14.).chip().is_none());
    }

    #[test]
    fn settlement_passes_the_fee_waypoint_on_both_layouts() {
        for width in [320., 342., 699., 700., 761.] {
            let layout = Layout::new(width);
            let fee = layout.settle.at(layout.settlement_position(Frame::at(12.)));
            let processor = layout.cards[1].center();
            if width < 700. {
                assert!((fee.y - processor.y).abs() < 0.01);
                assert!(fee.x > processor.x);
            } else {
                assert!((fee.x - processor.x).abs() < 0.01);
                assert!(fee.y > processor.y);
            }
            let end = layout.settle.at(layout.settle.length);
            assert!(end.distance(layout.cards[4].center()) < 0.01);
            assert!(
                layout
                    .cards
                    .iter()
                    .all(|card| card.origin.x + card.width <= width)
            );
        }
    }
}
