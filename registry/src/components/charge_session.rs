//! A simulated charge session with a direct-manipulation limit and power curve.
use crate::{
    foundation::Theme,
    primitives::{
        control::UiLanguage,
        currency::format_money,
        direct_touch::Contact,
        family::{Paint, Tone, disabled_motion, face},
        glyph::{Glyph, icon},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

const BATTERY_LEFT: f32 = 244.;
const BATTERY_WIDTH: f32 = 728.;
const BASELINE: f32 = 0.96;
const PACKETS: usize = 18;

/// Configuration for one simulated device. Energy is in kWh and power is in kW.
#[derive(Clone, Debug)]
pub struct ChargeSessionOptions {
    pub title: SharedString,
    pub location: SharedString,
    pub capacity: f64,
    pub max_power: f64,
    pub start_charge: f64,
    pub default_limit: f64,
    pub min_limit: f64,
    pub taper_at: f64,
    pub rate: f64,
    pub currency: SharedString,
    pub locale: SharedString,
    pub speed: f64,
    pub paused: bool,
    pub accent: Option<Hsla>,
    pub label: Option<SharedString>,
}

impl Default for ChargeSessionOptions {
    fn default() -> Self {
        Self {
            title: "Fast charger 4".into(),
            location: "Harbor Street hub, 150 kW".into(),
            capacity: 77.,
            max_power: 150.,
            start_charge: 34.,
            default_limit: 80.,
            min_limit: 50.,
            taper_at: 50.,
            rate: 0.42,
            currency: "USD".into(),
            locale: "en-US".into(),
            speed: 1.,
            paused: false,
            accent: None,
            label: None,
        }
    }
}

impl ChargeSessionOptions {
    fn validate(&mut self) {
        fn finite(value: f64, fallback: f64, range: std::ops::RangeInclusive<f64>) -> f64 {
            if value.is_finite() {
                value.clamp(*range.start(), *range.end())
            } else {
                fallback
            }
        }
        self.capacity = finite(self.capacity, 77., 0.001..=1e9);
        self.max_power = finite(self.max_power, 150., 0.001..=1e9);
        self.start_charge = finite(self.start_charge, 34., 0. ..=100.);
        self.min_limit = finite(self.min_limit, 50., 0. ..=100.);
        self.default_limit =
            finite(self.default_limit, 80., self.min_limit..=100.).clamp(self.min_limit, 100.);
        self.taper_at = finite(self.taper_at, 50., 0. ..=100.);
        self.rate = finite(self.rate, 0.42, 0. ..=1e9);
        self.speed = finite(self.speed, 1., 0. ..=1e6);
    }

    fn power_at(&self, charge: f64) -> f64 {
        // Normalized samples from the public curve. Interpolate between the samples
        // as a monotone cubic and map the taper onto the configured charge interval.
        const TAPER: [f64; 11] = [
            1., 0.8877, 0.7316, 0.5795, 0.4471, 0.3398, 0.2566, 0.1947, 0.1504, 0.1193, 0.0984,
        ];
        let knee = self.taper_at;
        let ramp = (charge / knee.clamp(f64::EPSILON, 10.)).clamp(0., 1.);
        let fraction = if charge < knee {
            0.62 + 0.38 * ramp * ramp * (3. - 2. * ramp)
        } else if knee >= 100. {
            1.
        } else {
            let index = ((charge - knee) * 10. / (100. - knee)).clamp(0., 10.);
            let i = (index.floor() as usize).min(9);
            let t = index - i as f64;
            let a = TAPER[i];
            let b = TAPER[i + 1];
            let d = b - a;
            let tangent = |i: usize| {
                let l = TAPER[i] - TAPER[i - 1];
                let r = TAPER[i + 1] - TAPER[i];
                2. * l * r / (l + r)
            };
            let from = if i == 0 { 0. } else { tangent(i) };
            let to = if i == 9 { d } else { tangent(i + 1) };
            a + t * (from + t * (3. * d - 2. * from - to + t * (from + to - 2. * d)))
        };
        self.max_power * fraction
    }

    fn eta(&self, charge: f64, limit: f64) -> f64 {
        let step = (limit - charge).max(0.) / 48.;
        let hours: f64 = (0..48)
            .map(|i| 1. / self.power_at(charge + (f64::from(i) + 0.5) * step))
            .sum();
        hours * step * self.capacity * 36.
    }
}

/// Operational state of the simulated charger.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChargeSessionState {
    Charging,
    Stopped,
    Complete,
}

use ChargeSessionState::{Charging, Complete, Stopped};

/// Snapshot taken when the state changes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChargeSessionDetail {
    pub charge: f64,
    pub energy: f64,
    pub cost: f64,
    pub limit: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChargeSessionStateChanged {
    pub state: ChargeSessionState,
    pub detail: ChargeSessionDetail,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChargeSessionLimitChanged(pub f64);

struct Session {
    state: ChargeSessionState,
    charge: f64,
    limit: f64,
    availability: f64,
    travel: f64,
}

impl Session {
    fn new(options: &ChargeSessionOptions) -> Self {
        let state = if options.start_charge >= options.default_limit {
            Complete
        } else {
            Charging
        };
        Self {
            state,
            charge: options.start_charge,
            limit: options.default_limit,
            availability: if state == Charging { 1. } else { 0. },
            travel: 0.,
        }
    }

    fn settle_limit(&mut self, limit: f64) {
        self.limit = limit;
        match self.state {
            Charging if self.charge >= limit => self.state = Complete,
            Complete if self.charge < limit => self.state = Charging,
            _ => {}
        }
    }

    fn activate(&mut self, start: f64) {
        self.state = match self.state {
            Charging => Stopped,
            Stopped if self.charge >= self.limit => Complete,
            Stopped => Charging,
            Complete => {
                self.charge = start;
                self.availability = 0.;
                self.travel = 0.;
                if start >= self.limit {
                    Complete
                } else {
                    Charging
                }
            }
        };
    }

    fn advance(&mut self, dt: f64, options: &ChargeSessionOptions) {
        let target = if self.state == Charging { 1. } else { 0. };
        let time_constant = if self.state == Charging { 0.52 } else { 0.43 };
        self.availability = target + (self.availability - target) * (-dt / time_constant).exp();
        if (target - self.availability).abs() < 0.005 {
            self.availability = target;
        }
        if self.availability > 0. {
            let relative_power = options.power_at(self.charge) / options.max_power;
            self.travel += dt * (0.55 + relative_power * 0.45) * self.availability;
        }
        if self.state == Charging {
            let energy = self.power(options) * dt * 40. * options.speed / 3600.;
            self.charge = (self.charge + energy / options.capacity * 100.).min(self.limit);
            if self.charge >= self.limit {
                self.state = Complete;
            }
        }
    }

    fn power(&self, options: &ChargeSessionOptions) -> f64 {
        options.power_at(self.charge) * self.availability
    }

    fn detail(&self, options: &ChargeSessionOptions) -> ChargeSessionDetail {
        let energy = (self.charge - options.start_charge).max(0.) / 100. * options.capacity;
        ChargeSessionDetail {
            charge: self.charge,
            energy,
            cost: energy * options.rate,
            limit: self.limit,
        }
    }
}

#[derive(Clone, Copy)]
enum Surface {
    Pack,
    Curve,
}

impl Surface {
    fn limit(self, bounds: Bounds<Pixels>, x: Pixels) -> f64 {
        let u = f32::from(x - bounds.left()) / f32::from(bounds.size.width.max(px(1.)));
        let percent = match self {
            Self::Pack => (u * 1000. - BATTERY_LEFT) / BATTERY_WIDTH,
            Self::Curve => u,
        };
        (f64::from(percent) * 100.).round()
    }
}

#[derive(Clone, Copy)]
struct Drag {
    bounds: Bounds<Pixels>,
    surface: Surface,
    origin: Point<Pixels>,
    offset: Pixels,
    touch: bool,
    active: bool,
    limit: f64,
}

type Style = Box<dyn Fn(Div) -> Div>;
type Visibility = Box<dyn Fn(&App) -> bool>;
type CostFormatter = Box<dyn Fn(f64, &str, &str) -> SharedString>;

/// Shared desktop and Web charging simulation. The host owns the real charger.
pub struct ChargeSession {
    options: ChargeSessionOptions,
    session: Session,
    id: ElementId,
    focus: FocusHandle,
    last: Option<Instant>,
    measured: Rc<Cell<Pixels>>,
    pack: Rc<Cell<Bounds<Pixels>>>,
    curve: Rc<Cell<Bounds<Pixels>>>,
    pack_contact: Contact,
    curve_contact: Contact,
    drag: Option<Drag>,
    announcement: SharedString,
    style: Option<Style>,
    visibility: Option<Visibility>,
    cost_formatter: Option<CostFormatter>,
}

impl EventEmitter<ChargeSessionStateChanged> for ChargeSession {}
impl EventEmitter<ChargeSessionLimitChanged> for ChargeSession {}

impl ChargeSession {
    pub fn new(mut options: ChargeSessionOptions, cx: &mut Context<Self>) -> Self {
        options.validate();
        let session = Session::new(&options);
        Self {
            options,
            session,
            id: "charge-session".into(),
            focus: cx.focus_handle(),
            last: None,
            measured: Rc::default(),
            pack: Rc::default(),
            curve: Rc::default(),
            pack_contact: Contact::default(),
            curve_contact: Contact::default(),
            drag: None,
            announcement: "".into(),
            style: None,
            visibility: None,
            cost_formatter: None,
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

    /// Allows the host to supply a locale-aware formatter.
    pub fn with_cost_formatter(
        mut self,
        format: impl Fn(f64, &str, &str) -> SharedString + 'static,
    ) -> Self {
        self.cost_formatter = Some(Box::new(format));
        self
    }

    /// Freezes or resumes the simulation and its packet animation.
    pub fn set_paused(&mut self, paused: bool, cx: &mut Context<Self>) {
        self.options.paused = paused;
        self.last = None;
        cx.notify();
    }

    pub fn detail(&self) -> ChargeSessionDetail {
        self.session.detail(&self.options)
    }

    pub fn state(&self) -> ChargeSessionState {
        self.session.state
    }

    fn cost(&self, cost: f64) -> SharedString {
        let (currency, locale) = (&self.options.currency, &self.options.locale);
        self.cost_formatter.as_ref().map_or_else(
            || format_money(cost, currency, locale).into(),
            |f| f(cost, currency, locale),
        )
    }

    fn wording(state: ChargeSessionState, lang: UiLanguage) -> (&'static str, &'static str) {
        match state {
            Charging => (
                lang.text("Charging", "正在充电"),
                lang.text("Stop charging", "停止充电"),
            ),
            Stopped => (
                lang.text("Stopped", "已停止"),
                lang.text("Resume charging", "继续充电"),
            ),
            Complete => (
                lang.text("Charged to limit", "已达充电上限"),
                lang.text("Start new session", "开始新会话"),
            ),
        }
    }

    fn announce(&mut self, cx: &App) {
        let detail = self.detail();
        let lang = UiLanguage::current(cx);
        let state = Self::wording(self.state(), lang).0;
        let limit = lang.text("Charge limit", "充电上限");
        self.announcement = format!(
            "{state}, {:.0}%, {:.1} kWh, {}, {limit} {:.0}%",
            detail.charge,
            detail.energy,
            self.cost(detail.cost),
            detail.limit
        )
        .into();
    }

    fn emit_state(&mut self, cx: &mut Context<Self>) {
        self.announce(cx);
        cx.emit(ChargeSessionStateChanged {
            state: self.state(),
            detail: self.detail(),
        });
        cx.notify();
    }

    fn change_limit(&mut self, limit: f64, cx: &mut Context<Self>) {
        let limit = limit.round().clamp(self.options.min_limit, 100.);
        if self.session.limit == limit {
            return;
        }
        let before = self.session.state;
        self.session.settle_limit(limit);
        self.announce(cx);
        cx.emit(ChargeSessionLimitChanged(limit));
        if self.session.state != before {
            self.emit_state(cx);
        } else {
            cx.notify();
        }
    }

    fn action(&mut self, cx: &mut Context<Self>) {
        self.session.activate(self.options.start_charge);
        self.emit_state(cx);
    }

    fn animate(&mut self, w: &Window, cx: &mut Context<Self>) {
        if disabled_motion(cx) {
            self.last = None;
            self.session.availability = if self.state() == Charging { 1. } else { 0. };
            return;
        }
        if self.options.paused
            || !w.is_visible()
            || self.visibility.as_ref().is_some_and(|visible| !visible(cx))
        {
            self.last = None;
            return;
        }
        let now = cx.background_executor().now();
        let before = self.state();
        if let Some(last) = self.last.replace(now) {
            let dt = now.saturating_duration_since(last).as_secs_f64().min(0.10);
            self.session.advance(dt, &self.options);
        }
        if before != self.state() {
            self.emit_state(cx);
        }
        if self.state() == Charging || self.session.availability > 0. {
            let weak = cx.weak_entity();
            w.on_next_frame(move |_, cx| {
                let _ = weak.update(cx, |_, cx| cx.notify());
            });
        } else {
            self.last = None;
        }
    }

    fn start_drag(
        &mut self,
        surface: Surface,
        origin: Point<Pixels>,
        touch: bool,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.drag.is_some() || (!touch && w.last_input_was_touch()) {
            return;
        }
        let bounds = match surface {
            Surface::Pack => self.pack.get(),
            Surface::Curve => self.curve.get(),
        };
        let offset = if matches!(surface, Surface::Pack)
            && origin.y >= bounds.top() - px(61.)
            && origin.y <= bounds.top() - px(14.)
        {
            let width = f32::from(bounds.size.width);
            let marker =
                width * (BATTERY_LEFT + BATTERY_WIDTH * self.session.limit as f32 / 100.) / 1000.;
            let center = (marker - 32.).clamp(0., (width - 64.).max(0.)) + 32.;
            if (origin.x - bounds.left() - px(center)).abs() <= px(32.) {
                origin.x - bounds.left() - px(marker)
            } else {
                px(0.)
            }
        } else {
            px(0.)
        };
        self.drag = Some(Drag {
            bounds,
            surface,
            origin,
            offset,
            touch,
            active: !touch,
            limit: self.session.limit,
        });
        if !touch {
            self.focus.focus(w, cx);
            self.drag_to(origin, cx);
        }
    }

    fn drag_to(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) {
        let Some(drag) = &mut self.drag else { return };
        if !drag.active {
            let dx = (position.x - drag.origin.x).abs();
            let dy = (position.y - drag.origin.y).abs();
            if dy > px(5.) && dy > dx {
                self.drag = None;
                return;
            }
            if dx < px(5.) {
                return;
            }
            drag.active = true;
        }
        drag.limit = drag
            .surface
            .limit(drag.bounds, position.x - drag.offset)
            .clamp(self.options.min_limit, 100.);
        cx.notify();
    }

    fn end_drag(&mut self, position: Point<Pixels>, cancel: bool, cx: &mut Context<Self>) {
        let Some(drag) = self.drag.take() else { return };
        if !cancel {
            self.change_limit(
                drag.surface.limit(drag.bounds, position.x - drag.offset),
                cx,
            );
        }
        cx.notify();
    }

    fn key(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let jump = if event.keystroke.modifiers.shift {
            10.
        } else {
            1.
        };
        let current = self.session.limit;
        let limit = match event.keystroke.key.as_str() {
            "left" | "down" => current - jump,
            "right" | "up" => current + jump,
            "pageup" => current + 10.,
            "pagedown" => current - 10.,
            "home" => self.options.min_limit,
            "end" => 100.,
            "escape" if self.drag.take().is_some() => {
                cx.stop_propagation();
                cx.notify();
                return;
            }
            _ => return,
        };
        cx.stop_propagation();
        self.change_limit(limit, cx);
    }
}

fn normalized_box(bounds: Bounds<Pixels>, [x, y, width, height]: [f32; 4]) -> Bounds<Pixels> {
    let sx = f32::from(bounds.size.width) / 1000.;
    let sy = f32::from(bounds.size.height) / 240.;
    Bounds::new(
        bounds.origin + point(px(x * sx), px(y * sy)),
        size(px(width * sx), px(height * sy)),
    )
}

fn line(
    w: &mut Window,
    points: impl IntoIterator<Item = Point<Pixels>>,
    width: Pixels,
    color: Hsla,
) {
    let mut points = points.into_iter();
    if let Some(first) = points.next() {
        let mut path = PathBuilder::stroke(width);
        path.move_to(first);
        for point in points {
            path.line_to(point);
        }
        if let Ok(path) = path.build() {
            w.paint_path(path, color);
        }
    }
}

fn box_face(
    w: &mut Window,
    bounds: Bounds<Pixels>,
    radius: Pixels,
    fill: impl Into<Background>,
    width: Pixels,
    stroke: Hsla,
) {
    w.paint_quad(quad(
        bounds,
        radius,
        fill,
        width,
        stroke,
        BorderStyle::Solid,
    ));
}

fn cable(t: f32) -> (f32, f32) {
    let u = 1. - t;
    let a = u * u * u;
    let b = 3. * t * u * u;
    let c = 3. * t * t * u;
    let d = t * t * t;
    (
        84. * a + 140. * b + 168. * c + 212. * d,
        176. * a + 238. * b + 120. * c + 120. * d,
    )
}

static CABLE: std::sync::LazyLock<[(f32, f32, f32); 33]> = std::sync::LazyLock::new(|| {
    let mut path = [(84., 176., 0.); 33];
    for i in 1..path.len() {
        let (x, y) = cable(i as f32 / 32.);
        let (last_x, last_y, length) = path[i - 1];
        path[i] = (x, y, length + (x - last_x).hypot(y - last_y));
    }
    path
});

fn cable_at(distance: f32) -> (f32, f32) {
    let i = CABLE
        .partition_point(|(_, _, length)| *length < distance)
        .clamp(1, 32);
    let (x, y, start) = CABLE[i - 1];
    let (next_x, next_y, end) = CABLE[i];
    let t = (distance - start) / (end - start);
    (x + (next_x - x) * t, y + (next_y - y) * t)
}

#[derive(Clone, Copy)]
struct Scene {
    paint: Paint,
    accent: Hsla,
    charge: f32,
    limit: f32,
    availability: f32,
    travel: f32,
}

impl Scene {
    fn paint_pack(self, bounds: Bounds<Pixels>, w: &mut Window) {
        let p = self.paint;
        let s = f32::from(bounds.size.width) / 1000.;
        let norm = |x, y| bounds.origin + point(px(x * s), px(y * s));
        let front = BATTERY_LEFT + BATTERY_WIDTH * self.charge;
        let limit = BATTERY_LEFT + BATTERY_WIDTH * self.limit;
        let shell = normalized_box(bounds, [232., 20., 752., 200.]);
        let cell = normalized_box(bounds, [BATTERY_LEFT, 32., BATTERY_WIDTH, 176.]);
        let metal = linear_gradient(
            116.,
            linear_color_stop(p.hover, 0.),
            linear_color_stop(p.panel.blend(p.faint.opacity(0.3)), 1.),
        );
        box_face(
            w,
            normalized_box(bounds, [12., 28., 72., 184.]),
            px(16. * s),
            metal,
            px(1.),
            p.line,
        );
        box_face(
            w,
            normalized_box(bounds, [28., 46., 40., 24.]),
            px(6. * s),
            p.panel,
            px(0.),
            p.line,
        );
        box_face(
            w,
            normalized_box(bounds, [44., 96., 8., 56.]),
            px(4. * s),
            self.accent.opacity(0.25 + 0.65 * self.availability),
            px(0.),
            p.line,
        );
        for (width, color) in [
            (13., p.line),
            (6., p.faint.opacity(0.24 + 0.26 * self.availability)),
        ] {
            line(
                w,
                CABLE.iter().map(|&(x, y, _)| norm(x, y)),
                px(width * s),
                color,
            );
        }
        for rect in [[72., 166., 18., 20.], [210., 96., 42., 48.]] {
            box_face(
                w,
                normalized_box(bounds, rect),
                px(8. * s),
                p.panel,
                px(1.),
                p.line,
            );
        }
        box_face(
            w,
            shell,
            px(45. * s),
            p.stage,
            px(1.),
            p.faint.opacity(0.36),
        );
        box_face(
            w,
            cell,
            px(33. * s),
            p.tray.blend(p.faint.opacity(0.15)),
            px(0.),
            p.line,
        );
        if self.charge > 0. {
            let clip = normalized_box(
                bounds,
                [BATTERY_LEFT, 32., BATTERY_WIDTH * self.charge, 176.],
            );
            let fill_color = linear_gradient(
                95.,
                linear_color_stop(self.accent.opacity(0.9), 0.),
                linear_color_stop(self.accent.opacity(0.7), 1.),
            );
            w.with_content_mask(Some(ContentMask { bounds: clip }), |w| {
                box_face(w, cell, px(33. * s), fill_color, px(0.), p.line);
            });
        }
        for tick in [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9] {
            let x = BATTERY_LEFT + BATTERY_WIDTH * tick;
            line(
                w,
                [norm(x, 36.), norm(x, 204.)],
                px(1.),
                p.line.opacity(0.34),
            );
        }
        if self.availability > 0. {
            let cable = CABLE[32].2;
            let length = cable + front - 212.;
            let alpha = self.availability.sqrt();
            for i in 0..PACKETS {
                let phase = (self.travel * 0.57 + i as f32 / PACKETS as f32).fract();
                let distance = (1. - (1. - phase).powf(1.28)) * length;
                let (x, y) = if distance < cable {
                    cable_at(distance)
                } else {
                    (212. + distance - cable, 120.)
                };
                let r = (4.6 * s).clamp(1.6, 4.5);
                let color = if x > BATTERY_LEFT {
                    p.panel
                } else {
                    self.accent
                };
                let fade = (phase * 12.).min(1.) * ((1. - phase) * 11.).min(1.);
                w.paint_quad(
                    fill(
                        Bounds::new(
                            norm(x, y) - point(px(r), px(r)),
                            size(px(r * 2.), px(r * 2.)),
                        ),
                        color.opacity(alpha * fade),
                    )
                    .corner_radii(px(r)),
                );
            }
        }
        line(
            w,
            [norm(limit, 0.), norm(limit, 20.)],
            px(1.),
            p.secondary.opacity(0.65),
        );
        let dashes = (26..220).step_by(10);
        let mut dash_path = PathBuilder::stroke(px(1.));
        for y in dashes {
            dash_path.move_to(norm(limit, y as f32));
            dash_path.line_to(norm(limit, y as f32 + 5.));
        }
        if let Ok(path) = dash_path.build() {
            w.paint_path(path, p.secondary.opacity(0.7));
        }
    }

    fn paint_curve(self, bounds: Bounds<Pixels>, model: &ChargeSessionOptions, w: &mut Window) {
        let p = self.paint;
        let width = f32::from(bounds.size.width);
        let height = f32::from(bounds.size.height);
        let plot = |x: f32, y: f32| bounds.origin + point(px(x * width), px(y * height));
        let curve_y = |x: f32| {
            BASELINE - 244. / 300. * (model.power_at(f64::from(x) * 100.) / model.max_power) as f32
        };
        for y in [curve_y(0.5).min(0.55), 0.55, BASELINE] {
            line(w, [plot(0., y), plot(1., y)], px(1.), p.line);
        }
        let future = Bounds::new(
            plot(self.limit, 0.),
            size(px(width * (1. - self.limit)), px(height * BASELINE)),
        );
        w.paint_quad(fill(future, p.faint.opacity(0.065)));
        line(
            w,
            (0..=100).map(|i| {
                let x = i as f32 / 100.;
                plot(x, curve_y(x))
            }),
            px(1.),
            p.faint.opacity(0.46),
        );
        let start = (model.start_charge / 100.) as f32;
        if self.charge > start {
            let traveled = (0..=64).map(|i| {
                let x = start + (self.charge - start) * i as f32 / 64.;
                plot(x, curve_y(x))
            });
            let mut area = PathBuilder::fill();
            area.move_to(plot(start, BASELINE));
            for point in traveled.clone() {
                area.line_to(point);
            }
            area.line_to(plot(self.charge, BASELINE));
            area.close();
            if let Ok(path) = area.build() {
                w.paint_path(
                    path,
                    linear_gradient(
                        180.,
                        linear_color_stop(self.accent.opacity(0.19), 0.),
                        linear_color_stop(self.accent.opacity(0.025), 1.),
                    ),
                );
            }
            line(w, traveled, px(1.65), self.accent.opacity(0.85));
        }
        let mut dashes = PathBuilder::stroke(px(1.));
        for y in (0..(height * BASELINE) as usize).step_by(8) {
            let at = plot(self.limit, y as f32 / height);
            dashes.move_to(at);
            dashes.line_to(at + point(px(0.), px(4.)));
        }
        if let Ok(path) = dashes.build() {
            w.paint_path(path, p.faint.opacity(0.7));
        }
        let dot_y = BASELINE + (curve_y(self.charge) - BASELINE) * self.availability;
        let dot = Bounds::new(
            plot(self.charge, dot_y) - point(px(4.), px(4.)),
            size(px(8.), px(8.)),
        );
        box_face(w, dot, px(4.), self.accent, px(1.), p.panel);
    }
}

fn state_mark(state: ChargeSessionState, ink: Hsla) -> AnyElement {
    match state {
        Charging => canvas(
            |_, _, _| {},
            move |b, _, w, _| {
                let points = [
                    (9., 1.),
                    (3., 8.5),
                    (7., 8.5),
                    (6.5, 14.5),
                    (13., 6.),
                    (9., 6.),
                    (9., 1.),
                ];
                line(
                    w,
                    points.map(|(x, y)| b.origin + point(px(x), px(y))),
                    px(1.2),
                    ink,
                );
            },
        )
        .size(px(16.))
        .into_any_element(),
        Stopped => icon(Glyph::Pause, 16., ink).into_any_element(),
        Complete => icon(Glyph::Check, 16., ink).into_any_element(),
    }
}

fn time_label(seconds: f64, lang: UiLanguage) -> String {
    let minutes = if seconds > 0. {
        (seconds / 60.).round().max(1.) as u64
    } else {
        0
    };
    match minutes {
        0 => lang.text("Done", "已完成").into(),
        1..=59 => format!("{minutes} {}", lang.text("min", "分钟")),
        _ if lang == UiLanguage::Chinese => {
            format!("{} 小时 {:02} 分钟", minutes / 60, minutes % 60)
        }
        _ => format!("{} h {:02} min", minutes / 60, minutes % 60),
    }
}

fn number_font() -> FontFeatures {
    FontFeatures(vec![("tnum".into(), 1)].into())
}

impl ChargeSession {
    fn surface_events(
        &self,
        surface: Surface,
        region: Stateful<Div>,
        cx: &Context<Self>,
    ) -> Stateful<Div> {
        let contact = match surface {
            Surface::Pack => &self.pack_contact,
            Surface::Curve => &self.curve_contact,
        };
        region
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |s, e: &MouseDownEvent, w, cx| {
                    s.start_drag(surface, e.position, false, w, cx);
                }),
            )
            .child(contact.listen(cx, move |s, e, w, cx| match e.phase {
                TouchPhase::Started => s.start_drag(surface, e.position, true, w, cx),
                TouchPhase::Moved => s.drag_to(e.position, cx),
                TouchPhase::Ended => s.end_drag(e.position, false, cx),
                TouchPhase::Cancelled => s.end_drag(e.position, true, cx),
            }))
    }

    fn mouse_capture(&self, cx: &Context<Self>) -> impl IntoElement {
        let movement = cx.listener(|s, e: &MouseMoveEvent, _, cx| {
            if s.drag.is_some_and(|drag| !drag.touch) {
                s.drag_to(e.position, cx);
            }
        });
        let release = cx.listener(|s, e: &MouseUpEvent, _, cx| {
            if e.button == MouseButton::Left && s.drag.is_some_and(|drag| !drag.touch) {
                s.end_drag(e.position, false, cx);
            }
        });
        canvas(
            |_, _, _| {},
            move |_, _, w, _| {
                w.on_mouse_event(move |e, phase: DispatchPhase, w, cx| {
                    if phase.capture() {
                        movement(e, w, cx);
                    }
                });
                w.on_mouse_event(move |e, phase: DispatchPhase, w, cx| {
                    if phase.capture() {
                        release(e, w, cx);
                    }
                });
            },
        )
        .absolute()
        .size(px(0.))
    }

    fn stage(
        &self,
        width: f32,
        scene: Scene,
        spoken: SharedString,
        cx: &Context<Self>,
    ) -> Stateful<Div> {
        let graphic = width * 0.24;
        let pack = self.pack.clone();
        let limit = scene.limit;
        let radius = Theme::global(cx).tokens.radius.md;
        let left = (width * (BATTERY_LEFT + BATTERY_WIDTH * limit) / 1000. - 32.)
            .clamp(0., (width - 64.).max(0.));
        let region = div()
            .id("charge-pack")
            .debug_selector(|| "charge-pack".into())
            .relative()
            .w_full()
            .h(px(graphic + 58.))
            .flex_none()
            .cursor_ew_resize()
            .child(
                canvas(
                    move |b, _, _| pack.set(b),
                    move |b, _, w, _| scene.paint_pack(b, w),
                )
                .absolute()
                .top(px(58.))
                .left_0()
                .right_0()
                .bottom_0(),
            )
            .child(
                div()
                    .absolute()
                    .left(px(
                        (width * (BATTERY_LEFT + BATTERY_WIDTH * limit) / 1000.).clamp(0., width)
                    ))
                    .top(px(if self.drag.is_some_and(|d| d.active) {
                        25.
                    } else {
                        28.
                    }))
                    .w(px(1.))
                    .bottom(px(graphic))
                    .bg(scene.paint.secondary.opacity(0.63)),
            )
            .child(
                div()
                    .id("charge-limit")
                    .debug_selector(|| "charge-limit".into())
                    .role(Role::Slider)
                    .aria_label(UiLanguage::current(cx).text("Charge limit", "充电上限"))
                    .aria_value(spoken)
                    .aria_numeric_value(f64::from(limit) * 100.)
                    .aria_min_numeric_value(self.options.min_limit)
                    .aria_max_numeric_value(100.)
                    .aria_numeric_value_step(1.)
                    .aria_description(UiLanguage::current(cx).text(
                        "Drag across the pack or curve to set the charge limit.",
                        "拖动电池或曲线可设置上限。",
                    ))
                    .tab_index(0)
                    .track_focus(&self.focus)
                    .on_key_down(cx.listener(|s, e, _, cx| s.key(e, cx)))
                    .absolute()
                    .left(px(left))
                    .top(px(if self.drag.is_some_and(|d| d.active) {
                        -3.
                    } else {
                        0.
                    }))
                    .w(px(64.))
                    .h(px(44.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(radius)
                    .border_2()
                    .border_color(transparent_black())
                    .focus_visible(move |s| s.border_color(scene.paint.focus))
                    .child(
                        div()
                            .rounded(radius)
                            .border_1()
                            .border_color(scene.paint.faint.opacity(0.42))
                            .bg(scene.paint.stage)
                            .px(px(9.))
                            .h(px(26.))
                            .flex()
                            .items_center()
                            .text_size(px(12.))
                            .font_features(number_font())
                            .child(format!("{:.0}%", limit * 100.)),
                    ),
            );
        self.surface_events(Surface::Pack, region, cx)
    }

    fn chart(&self, scene: Scene, plot_height: f32, power: f64, cx: &Context<Self>) -> Div {
        let lang = UiLanguage::current(cx);
        let bounds = self.curve.clone();
        let options = self.options.clone();
        let p = scene.paint;
        let region = div()
            .id("charge-curve")
            .debug_selector(|| "charge-curve".into())
            .relative()
            .w_full()
            .h(px(plot_height))
            .cursor_ew_resize()
            .child(
                canvas(
                    move |b, _, _| bounds.set(b),
                    move |b, _, w, _| scene.paint_curve(b, &options, w),
                )
                .absolute()
                .inset_0(),
            )
            .child(
                div()
                    .absolute()
                    .left_0()
                    .top_0()
                    .text_size(px(12.))
                    .text_color(p.faint)
                    .child(format!("{:.0} kW", self.options.max_power)),
            );
        div()
            .flex_1()
            .min_w(px(0.))
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(
                div()
                    .flex()
                    .justify_between()
                    .gap(px(6.))
                    .text_size(px(14.))
                    .child(lang.text("Charge curve", "充电曲线"))
                    .child(
                        div()
                            .text_color(p.faint)
                            .font_features(number_font())
                            .child(format!(
                                "{power:.0} kW {} {:.0}%",
                                lang.text("at", "·"),
                                scene.charge * 100.
                            )),
                    ),
            )
            .child(self.surface_events(Surface::Curve, region, cx))
            .child(
                div()
                    .flex()
                    .justify_between()
                    .text_size(px(12.))
                    .text_color(p.faint)
                    .children(["0%", "50%", "100%"]),
            )
    }
}

fn readouts(
    labels: [SharedString; 4],
    values: [SharedString; 4],
    paint: Paint,
    radius: Pixels,
    side_by_side: bool,
    narrow: bool,
) -> Div {
    div()
        .when(side_by_side, |e| {
            e.flex_none().w(relative(0.379)).h(px(213.))
        })
        .when(!side_by_side, |e| e.w_full().h(px(133.)))
        .rounded(radius)
        .border_1()
        .border_color(paint.line)
        .flex()
        .flex_col()
        .children((0..2).map(|row| {
            div().flex().flex_1().children((0..2).map(|col| {
                let i = row * 2 + col;
                div()
                    .flex()
                    .flex_col()
                    .justify_center()
                    .min_w(px(0.))
                    .flex_1()
                    .gap(px(2.))
                    .px(px(if narrow { 12. } else { 16. }))
                    .when(row == 0, |e| e.border_b_1())
                    .when(col == 0, |e| e.border_r_1())
                    .border_color(paint.line)
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(paint.faint)
                            .child(labels[i].clone()),
                    )
                    .child(
                        div()
                            .text_size(px(if narrow { 16. } else { 22. }))
                            .font_features(number_font())
                            .child(values[i].clone()),
                    )
            }))
        }))
}

impl Render for ChargeSession {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.animate(w, cx);
        let lang = UiLanguage::current(cx);
        let theme = Theme::global(cx);
        let p = Paint::get(cx);
        let radius = theme.tokens.radius.md;
        let state = self.state();
        let detail = self.detail();
        let power = self.session.power(&self.options);
        let limit = self.drag.map_or(detail.limit, |drag| drag.limit);
        let eta = self.options.eta(detail.charge, limit);
        let time = match state {
            Charging => time_label(eta, lang),
            Stopped => lang.text("Paused", "已暂停").into(),
            Complete => lang.text("Done", "已完成").into(),
        };
        let spoken = match (state, eta > 0.) {
            (Charging, true) => format!(
                "{} {limit:.0}%, {} {time}",
                lang.text("Charge limit", "充电上限"),
                lang.text("about", "约剩")
            ),
            _ => format!("{} {limit:.0}%", lang.text("Charge limit", "充电上限")),
        };
        let (word, action) = Self::wording(state, lang);
        let stage = Scene {
            paint: p,
            accent: self.options.accent.unwrap_or(p.bright),
            charge: (detail.charge / 100.) as f32,
            limit: (limit / 100.) as f32,
            availability: self.session.availability as f32,
            travel: self.session.travel.rem_euclid(100. / 0.57) as f32,
        };
        let width = if self.measured.get() > px(0.) {
            f32::from(self.measured.get())
        } else {
            f32::from(w.viewport_size().width).clamp(210., 761.)
        };
        let narrow = width < 420.;
        let side_by_side = width >= 640.;
        let pad = if narrow { 16. } else { 32. };
        let content = (width - pad * 2. - 2.).max(1.);
        let pack = self.stage(content, stage, spoken.into(), cx);
        let plot = self.chart(stage, if side_by_side { 163. } else { 135. }, power, cx);
        let stats = readouts(
            [
                lang.text("Power", "功率").into(),
                lang.text("Energy added", "已充电量").into(),
                format!("{} {limit:.0}%", lang.text("Time to", "充至")).into(),
                lang.text("Cost", "费用").into(),
            ],
            [
                format!("{power:.0} kW").into(),
                format!("{:.1} kWh", detail.energy).into(),
                time.into(),
                self.cost(detail.cost),
            ],
            p,
            radius,
            side_by_side,
            narrow,
        );
        let action_ink = if state == Charging { p.ink } else { p.inverse };
        let button = face(
            "charge-session-action",
            action,
            44.,
            f32::from(radius),
            if state == Charging {
                Tone::Surface
            } else {
                Tone::Primary
            },
            cx,
        )
        .debug_selector(|| "charge-session-action".into())
        .when(narrow, |e| e.w_full())
        .when(!narrow, |e| e.w(px(182.)))
        .border_1()
        .border_color(if state == Charging {
            p.faint.opacity(0.38)
        } else {
            transparent_black()
        })
        .hover(move |s| s.opacity(0.87))
        .child(match state {
            Charging => div()
                .size(px(10.))
                .rounded(px(1.))
                .border_1()
                .border_color(action_ink)
                .into_any_element(),
            Stopped => state_mark(Charging, action_ink),
            Complete => icon(Glyph::Undo, 14., action_ink).into_any_element(),
        })
        .child(action)
        .on_click(cx.listener(|s, _, _, cx| s.action(cx)));
        let body = div()
            .relative()
            .w_full()
            .p(px(pad))
            .border_1()
            .border_color(p.line)
            .rounded(radius)
            .bg(p.panel)
            .text_color(p.ink)
            .flex()
            .flex_col()
            .child(
                div()
                    .h(px(if side_by_side { 94. } else { 72. }))
                    .flex()
                    .justify_between()
                    .gap(px(12.))
                    .child(
                        div()
                            .min_w(px(0.))
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .id("charge-title")
                                    .role(Role::Heading)
                                    .text_size(px(16.))
                                    .line_height(px(19.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(self.options.title.clone()),
                            )
                            .when(!self.options.location.is_empty(), |e| {
                                e.child(
                                    div()
                                        .text_size(px(14.))
                                        .line_height(px(19.))
                                        .text_color(p.secondary)
                                        .child(self.options.location.clone()),
                                )
                            }),
                    )
                    .child(
                        div()
                            .flex_none()
                            .flex()
                            .flex_col()
                            .items_end()
                            .child(
                                div()
                                    .text_size(px(if side_by_side { 72. } else { 44. }))
                                    .line_height(px(if side_by_side { 72. } else { 46. }))
                                    .font_features(number_font())
                                    .child(format!("{:.0}%", detail.charge)),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(4.))
                                    .text_size(px(14.))
                                    .text_color(p.secondary)
                                    .child(state_mark(state, p.secondary))
                                    .child(word),
                            ),
                    ),
            )
            .mt(px(0.))
            .child(pack)
            .child(self.mouse_capture(cx))
            .child(
                div()
                    .mt(px(24.))
                    .w_full()
                    .flex()
                    .gap(px(24.))
                    .when(!side_by_side, |e| e.flex_col())
                    .child(plot)
                    .child(stats),
            )
            .child(
                div()
                    .mt(px(if narrow { 16. } else { 24. }))
                    .pt(px(16.))
                    .border_t_1()
                    .border_color(p.line)
                    .flex()
                    .gap(px(12.))
                    .when(narrow, |e| e.flex_col())
                    .when(!narrow, |e| e.items_center().justify_between())
                    .child(
                        div()
                            .min_w(px(0.))
                            .text_size(px(14.))
                            .line_height(px(19.))
                            .text_color(p.faint)
                            .child(lang.text(
                                "Drag across the pack or the curve to set the charge limit.",
                                "拖动电池或曲线可设置上限。",
                            )),
                    )
                    .child(button),
            )
            .child(
                div()
                    .id("charge-announcement")
                    .role(Role::Status)
                    .aria_label(self.announcement.clone())
                    .size(px(0.)),
            );
        let measured = self.measured.clone();
        let root = div().w_full().max_w(px(761.)).min_w(px(0.));
        let root = if let Some(style) = &self.style {
            style(root)
        } else {
            root
        };
        root.on_children_prepainted(move |children, w, _| {
            if let Some(card) = children.first()
                && card.size.width != measured.replace(card.size.width)
            {
                w.request_animation_frame();
            }
        })
        .id(self.id.clone())
        .role(Role::Group)
        .aria_label(
            self.options
                .label
                .clone()
                .unwrap_or_else(|| self.options.title.clone()),
        )
        .child(body)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ChargeSession, ChargeSessionLimitChanged, ChargeSessionOptions, ChargeSessionStateChanged,
        Charging, Complete, Session, Stopped, time_label,
    };
    use gpui::{Modifiers, TestAppContext, TouchEvent, TouchId, TouchPhase, point, px};
    use std::{cell::RefCell, rc::Rc};

    #[test]
    fn default_curve_integrates_to_the_reference_time_and_cost() {
        let options = ChargeSessionOptions::default();
        assert_eq!(options.power_at(34.), 150.);
        assert!((options.power_at(80.) - 38.49).abs() < 0.01);
        assert_eq!(
            time_label(
                options.eta(34., 80.),
                crate::primitives::control::UiLanguage::English
            ),
            "23 min"
        );
        let mut session = Session::new(&options);
        for _ in 0..5000 {
            session.advance(0.016, &options);
            if session.state == Complete {
                break;
            }
        }
        let detail = session.detail(&options);
        assert_eq!(session.state, Complete);
        assert_eq!(detail.charge, 80.);
        assert!((detail.energy - 35.42).abs() < 0.001);
        assert!((detail.cost - 14.8764).abs() < 0.001);
    }

    #[test]
    fn stopped_session_drains_and_speed_zero_preserves_visual_flow() {
        let options = ChargeSessionOptions {
            speed: 0.,
            ..Default::default()
        };
        let mut session = Session::new(&options);
        session.advance(0.1, &options);
        assert_eq!(session.charge, 34.);
        assert!(session.travel > 0.);
        session.activate(options.start_charge);
        session.advance(0.1, &options);
        assert!(session.power(&options) > 0. && session.power(&options) < options.max_power);
        for _ in 0..30 {
            session.advance(0.1, &options);
        }
        assert_eq!(session.power(&options), 0.);
        session.settle_limit(60.);
        assert_eq!(session.state, Stopped);
        session.activate(options.start_charge);
        assert_eq!(session.state, Charging);
    }

    #[test]
    fn a_settled_limit_emits_once_and_keyboard_resume_reuses_the_session() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let limits = Rc::new(RefCell::new(Vec::new()));
        let states = Rc::new(RefCell::new(Vec::new()));
        let (view, w) = app.add_window_view(|_, cx| {
            ChargeSession::new(
                ChargeSessionOptions {
                    start_charge: 60.,
                    paused: true,
                    ..Default::default()
                },
                cx,
            )
        });
        w.update(|w, cx| {
            let log = limits.clone();
            cx.subscribe(&view, move |_, event: &ChargeSessionLimitChanged, _| {
                log.borrow_mut().push(event.0)
            })
            .detach();
            let log = states.clone();
            cx.subscribe(&view, move |_, event: &ChargeSessionStateChanged, _| {
                log.borrow_mut().push(event.state)
            })
            .detach();
            let focus = view.read(cx).focus.clone();
            focus.focus(w, cx);
        });
        w.simulate_keystrokes("home home end");
        assert_eq!(*limits.borrow(), vec![50., 100.]);
        assert_eq!(*states.borrow(), vec![Complete, Charging]);
        let button = w.debug_bounds("charge-session-action").unwrap();
        w.simulate_click(button.center(), Modifiers::default());
        assert_eq!(w.update(|_, cx| view.read(cx).state()), Stopped);
        assert_eq!(w.debug_bounds("charge-session-action").unwrap(), button);
    }

    #[test]
    fn a_vertical_contact_does_not_commit_a_limit() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) = app.add_window_view(|_, cx| {
            ChargeSession::new(
                ChargeSessionOptions {
                    paused: true,
                    ..Default::default()
                },
                cx,
            )
        });
        let center = w.debug_bounds("charge-curve").unwrap().center();
        for (phase, delta) in [
            (TouchPhase::Started, point(px(0.), px(0.))),
            (TouchPhase::Moved, point(px(1.), px(22.))),
            (TouchPhase::Ended, point(px(1.), px(22.))),
        ] {
            w.simulate_event(TouchEvent {
                id: TouchId(1),
                phase,
                position: center + delta,
                predicted_position: None,
                force: None,
            });
        }
        assert_eq!(w.update(|_, cx| view.read(cx).detail().limit), 80.);
        for (phase, x) in [
            (TouchPhase::Started, 0.),
            (TouchPhase::Moved, 62.),
            (TouchPhase::Ended, 62.),
        ] {
            w.simulate_event(TouchEvent {
                id: TouchId(2),
                phase,
                position: center + point(px(x), px(0.)),
                predicted_position: None,
                force: None,
            });
        }
        let expected = w.update(|_, cx| {
            let view = view.read(cx);
            super::Surface::Curve
                .limit(view.curve.get(), center.x + px(62.))
                .clamp(50., 100.)
        });
        assert_eq!(w.update(|_, cx| view.read(cx).detail().limit), expected);
        assert_ne!(expected, 80.);
    }
}
