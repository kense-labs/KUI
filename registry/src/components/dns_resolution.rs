//! A self-contained, simulated DNS lookup drawn in GPUI.
use crate::{
    foundation::Theme,
    primitives::{
        family::{Paint, PhaseClock, Slide, Tone, disabled_motion, face},
        glyph::{Glyph, icon},
        presentation::tr,
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, f32::consts::TAU, rc::Rc, sync::OnceLock};

// Cue times measured from the public 16-second playback, rounded to tenths.
const MARKS: [f32; 7] = [0., 1.9, 4.1, 6.3, 8.4, 11., 16.];
const ANSWER: f32 = MARKS[4];
const CONNECTED: f32 = 10.3;
const HIT: f32 = 12.1;
const CACHED: f32 = 12.8;
const EXPIRE: f32 = 14.9;
const STEPS: [(&str, &str, &str, &str); 6] = [
    ("Ask", "查询", "Ask the resolver", "向解析器查询"),
    ("Root", "根", "Root server", "根服务器"),
    (".dev", "顶级域", "TLD server", "顶级域服务器"),
    (
        "Nameserver",
        "权威",
        "Authoritative nameserver",
        "权威域名服务器",
    ),
    ("Answer", "应答", "Answer and connect", "应答并建立连接"),
    ("Cache", "缓存", "Answered from cache", "从缓存应答"),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DnsNodeKind {
    Browser,
    Resolver,
    Root,
    Tld,
    Authoritative,
}

#[derive(Clone, Default)]
pub struct DnsNode {
    pub label: Option<SharedString>,
    pub meta: Option<SharedString>,
    pub detail: Option<SharedString>,
    pub icon: Option<Glyph>,
    pub color: Option<Hsla>,
    /// A host-supplied GPUI element factory for an image, custom path or other mark.
    pub mark: Option<Rc<dyn Fn() -> AnyElement>>,
}

#[derive(Clone)]
pub struct DnsResolutionOptions {
    pub domain: SharedString,
    pub address: SharedString,
    pub ttl: u32,
    pub lookup_ms: u32,
    pub cached_ms: u32,
    pub nodes: [DnsNode; 5],
    pub speed: f32,
    pub paused: bool,
    pub accent: Option<Hsla>,
    pub controls: bool,
    pub caption: bool,
    pub label: Option<SharedString>,
}

impl Default for DnsResolutionOptions {
    fn default() -> Self {
        Self {
            domain: "app.acme.dev".into(),
            address: "203.0.113.24".into(),
            ttl: 300,
            lookup_ms: 84,
            cached_ms: 1,
            nodes: Default::default(),
            speed: 1.,
            paused: false,
            accent: None,
            controls: true,
            caption: true,
            label: None,
        }
    }
}

impl DnsResolutionOptions {
    pub fn with_node(mut self, kind: DnsNodeKind, node: DnsNode) -> Self {
        self.nodes[kind as usize] = node;
        self
    }

    fn domains(&self) -> (&str, &str) {
        let name = self.domain.trim_end_matches('.');
        let Some((prefix, suffix)) = name.rsplit_once('.') else {
            return (name, name);
        };
        let base = prefix.rsplit_once('.').map_or(prefix, |(_, tail)| tail);
        (
            &name[name.len() - suffix.len() - 1..],
            &name[name.len() - base.len() - suffix.len() - 1..],
        )
    }
}

#[derive(Clone, Copy)]
struct Snapshot {
    time: f32,
    stage: usize,
    expired: bool,
}

impl Snapshot {
    fn at(time: f32) -> Self {
        let time = time.rem_euclid(MARKS[6]);
        Self {
            time,
            stage: MARKS[..6].partition_point(|mark| *mark <= time) - 1,
            expired: time >= EXPIRE,
        }
    }

    fn fraction(self, start: f32, end: f32) -> f32 {
        ((self.time - start) / (end - start)).clamp(0., 1.)
    }

    fn ttl(self) -> f32 {
        1. - self.fraction(ANSWER, EXPIRE).powf(1.17)
    }

    fn alpha(self) -> f32 {
        1. - self.fraction(15.62, 16.)
    }

    fn packet(self) -> Option<(usize, f32, bool, bool)> {
        // Each of the server hops carries a question out and a referral back.
        let (route, fraction, reverse) = match self.stage {
            0 => (0, self.fraction(0., MARKS[1] - 0.16), false),
            1..=3 => {
                let t = self.fraction(MARKS[self.stage], MARKS[self.stage + 1] - 0.15);
                (self.stage, 1. - (2. * t - 1.).abs(), t >= 0.5)
            }
            4 => (0, 1. - self.fraction(ANSWER + 0.14, CONNECTED), true),
            _ if self.time >= MARKS[5] + 0.14 && self.time < CACHED => {
                let t = self.fraction(MARKS[5] + 0.14, CACHED);
                (0, 1. - (2. * t - 1.).abs(), t >= 0.5)
            }
            _ => return None,
        };
        let eased = (1. - (std::f32::consts::PI * fraction).cos()) * 0.5;
        Some((route, eased, reverse, self.stage >= 4))
    }
}

type Coordinate = (f32, f32);

#[derive(Clone, Copy)]
struct Platform {
    x: f32,
    y: f32,
    half: f32,
    depth: f32,
    label: f32,
}

struct Geometry {
    size: Coordinate,
    nodes: [Platform; 5],
    routes: [Arc; 4],
}

struct Arc {
    start: Coordinate,
    control: Coordinate,
    end: Coordinate,
    lengths: [f32; 33],
}

impl Arc {
    fn new(start: Coordinate, control: Coordinate, end: Coordinate) -> Self {
        let mut route = Self {
            start,
            control,
            end,
            lengths: [0.; 33],
        };
        let mut last = start;
        for i in 1..route.lengths.len() {
            let next = route.point(i as f32 / 32.);
            route.lengths[i] = route.lengths[i - 1] + (next.0 - last.0).hypot(next.1 - last.1);
            last = next;
        }
        route
    }

    fn point(&self, t: f32) -> Coordinate {
        let s = 1. - t;
        let a = s * s;
        let b = 2. * s * t;
        let c = t * t;
        (
            a * self.start.0 + b * self.control.0 + c * self.end.0,
            a * self.start.1 + b * self.control.1 + c * self.end.1,
        )
    }

    fn at(&self, progress: f32) -> Coordinate {
        let distance = self.lengths[32] * progress.clamp(0., 1.);
        let i = self.lengths.partition_point(|x| *x < distance).clamp(1, 32);
        let offset = (distance - self.lengths[i - 1]) / (self.lengths[i] - self.lengths[i - 1]);
        self.point((i as f32 - 1. + offset) / 32.)
    }
}

fn geometry(narrow: bool) -> &'static Geometry {
    static WIDE: OnceLock<Geometry> = OnceLock::new();
    static NARROW: OnceLock<Geometry> = OnceLock::new();
    let cache = if narrow { &NARROW } else { &WIDE };
    cache.get_or_init(|| {
        let (size, positions, bends) = if narrow {
            (
                (390., 676.),
                [
                    (104., 122., 62., 12., 38.),
                    (112., 452., 70., 18., 517.),
                    (294., 104., 50., 22., 163.),
                    (294., 318., 50., 22., 377.),
                    (294., 532., 50., 22., 591.),
                ],
                [(44., 251.), (163., 202.), (193., 303.), (223., 446.)],
            )
        } else {
            (
                (1000., 612.),
                [
                    (160., 290., 74., 14., 353.),
                    (480., 290., 84., 20., 364.),
                    (850., 80., 58., 26., 147.),
                    (850., 272., 58., 26., 339.),
                    (850., 464., 58., 26., 531.),
                ],
                [(320., 166.), (625., 87.), (665., 171.), (689., 311.)],
            )
        };
        let nodes = positions.map(|(x, y, half, depth, label)| Platform {
            x,
            y,
            half,
            depth,
            label,
        });
        let resolver = (nodes[1].x, nodes[1].y - 12.);
        let routes = std::array::from_fn(|i| {
            let n = nodes[if i == 0 { 0 } else { i + 1 }];
            if i == 0 {
                Arc::new((n.x, n.y), bends[i], resolver)
            } else {
                Arc::new(resolver, bends[i], (n.x, n.y))
            }
        });
        Geometry {
            size,
            nodes,
            routes,
        }
    })
}

type Refine = Box<dyn Fn(Div) -> Div>;
type Visibility = Box<dyn Fn(&App) -> bool>;

pub struct DnsResolution {
    options: DnsResolutionOptions,
    elapsed: f32,
    age: f32,
    clock: PhaseClock,
    pace: Slide,
    paused: bool,
    visible: bool,
    visibility: Option<Visibility>,
    hovered: Option<usize>,
    step: Option<usize>,
    width: Rc<Cell<Pixels>>,
    focus: [FocusHandle; 6],
    id: ElementId,
    style: Option<Refine>,
}

impl DnsResolution {
    pub fn new(options: DnsResolutionOptions, cx: &mut Context<Self>) -> Self {
        let paused = options.paused;
        Self {
            elapsed: if paused { CACHED } else { 0. },
            pace: Slide::new(if paused { 0. } else { speed(options.speed) }),
            options,
            age: 0.,
            clock: PhaseClock::default(),
            paused,
            visible: true,
            visibility: None,
            hovered: None,
            step: None,
            width: Rc::default(),
            focus: std::array::from_fn(|_| cx.focus_handle()),
            id: "dns-resolution".into(),
            style: None,
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

    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        if self.visible != visible {
            self.visible = visible;
            self.clock = PhaseClock::default();
            cx.notify();
        }
    }

    pub fn set_speed(&mut self, value: f32, cx: &mut Context<Self>) {
        self.options.speed = speed(value);
        self.retarget(cx);
    }

    pub fn set_paused(&mut self, paused: bool, cx: &mut Context<Self>) {
        self.paused = paused;
        if !paused {
            self.step = None;
        }
        self.retarget(cx);
    }

    pub fn seek(&mut self, stage: usize, cx: &mut Context<Self>) {
        let stage = stage.min(5);
        self.step = Some(stage);
        self.elapsed = MARKS[stage];
        self.clock = PhaseClock::default();
        if self.paused {
            self.pace.reset(0.);
        }
        cx.notify();
    }

    fn retarget(&mut self, cx: &mut Context<Self>) {
        let pace = if self.paused {
            0.
        } else {
            speed(self.options.speed)
        };
        self.pace.set(
            if self.hovered.is_some() {
                pace * 0.55
            } else {
                pace
            },
            cx,
        );
        cx.notify();
    }

    fn snapshot(&mut self, window: &Window, cx: &App) -> Snapshot {
        let reduced = disabled_motion(cx);
        if !self.visible
            || !window.is_visible()
            || reduced
            || self.visibility.as_ref().is_some_and(|visible| !visible(cx))
        {
            self.clock = PhaseClock::default();
            if reduced {
                return Snapshot::at(
                    self.step
                        .map_or(CACHED, |step| (MARKS[step] + MARKS[step + 1]) / 2.),
                );
            }
        } else {
            if !self.clock.age(cx).is_finite() {
                self.clock.restart(cx);
                self.age = 0.;
            }
            let age = self.clock.age(cx);
            let (pace, moving) = self
                .pace
                .sample_batch(cx.background_executor().now(), false);
            self.elapsed =
                (self.elapsed + (age - self.age).max(0.) * pace.max(0.)).rem_euclid(MARKS[6]);
            self.age = age;
            if moving || pace > 0. {
                window.request_animation_frame();
            }
        }
        Snapshot::at(self.elapsed)
    }
}

fn speed(value: f32) -> f32 {
    if value.is_finite() { value.max(0.) } else { 1. }
}

fn draw_shape(w: &mut Window, points: &[Point<Pixels>], fill_color: Option<Hsla>, ink: Hsla) {
    if let Some(color) = fill_color {
        let mut path = PathBuilder::fill();
        path.move_to(points[0]);
        for point in &points[1..] {
            path.line_to(*point);
        }
        path.close();
        if let Ok(path) = path.build() {
            w.paint_path(path, color);
        }
    }
    let mut outline = PathBuilder::stroke(px(1.));
    outline.move_to(points[0]);
    for point in &points[1..] {
        outline.line_to(*point);
    }
    if fill_color.is_some() {
        outline.close();
    }
    if let Ok(path) = outline.build() {
        w.paint_path(path, ink);
    }
}

fn draw_platform(
    w: &mut Window,
    at: impl Fn(Coordinate) -> Point<Pixels>,
    n: Platform,
    paint: Paint,
) {
    let Platform {
        x, y, half, depth, ..
    } = n;
    let fore = y + half * 0.5;
    let top = [(x, y - half * 0.5), (x + half, y), (x, fore), (x - half, y)];
    let left = [top[3], top[2], (x, fore + depth), (x - half, y + depth)];
    let right = [top[2], top[1], (x + half, y + depth), (x, fore + depth)];
    let line = paint.ink.opacity(0.19);
    draw_shape(w, &left.map(&at), Some(paint.tray), line);
    draw_shape(w, &right.map(&at), Some(paint.hover), line);
    draw_shape(w, &top.map(at), Some(paint.panel), line);
}

fn dot(w: &mut Window, point: Point<Pixels>, radius: f32, color: Hsla) {
    let r = px(radius);
    w.paint_quad(
        fill(
            Bounds::new(point - gpui::point(r, r), size(r * 2., r * 2.)),
            color,
        )
        .corner_radii(r),
    );
}

fn draw_scene(
    w: &mut Window,
    b: Bounds<Pixels>,
    geo: &Geometry,
    frame: Snapshot,
    p: Paint,
    accent: Hsla,
    reduced: bool,
) {
    let (width, height) = geo.size;
    let at = |(x, y): Coordinate| {
        point(
            b.origin.x + b.size.width * (x / width),
            b.origin.y + b.size.height * (y / height),
        )
    };
    let mut ground = PathBuilder::stroke(px(1.));
    for i in 0..geo.routes.len() {
        let a = geo.nodes[if i == 0 { 0 } else { 1 }];
        let z = geo.nodes[if i == 0 { 1 } else { i + 1 }];
        for j in (0..24).step_by(2) {
            let floor = |t: f32| {
                at((
                    a.x + (z.x - a.x) * t,
                    a.y + a.depth + (z.y + z.depth - a.y - a.depth) * t,
                ))
            };
            ground.move_to(floor(j as f32 / 24.));
            ground.line_to(floor((j + 1) as f32 / 24.));
        }
    }
    if let Ok(path) = ground.build() {
        w.paint_path(path, p.ink.opacity(0.06));
    }
    for i in [2, 3, 0, 1, 4] {
        let n = geo.nodes[i];
        draw_platform(w, at, n, p);
        if i == 1 {
            draw_platform(
                w,
                at,
                Platform {
                    half: n.half * 0.58,
                    y: n.y - 12.,
                    depth: 12.,
                    ..n
                },
                p,
            );
            let center = at((n.x, n.y - 12.));
            let scale = b.size.width / px(width);
            let r = 30. * scale;
            for (fraction, color) in [
                (1., p.ink.opacity(0.09)),
                (
                    if frame.time >= ANSWER {
                        frame.ttl()
                    } else {
                        0.
                    },
                    accent.opacity(frame.alpha()),
                ),
            ] {
                if fraction > 0. {
                    let mut line = PathBuilder::stroke(px(1.));
                    let count = (fraction * 32.).ceil() as usize;
                    for j in 0..=count {
                        let a = TAU * fraction * j as f32 / count as f32 - TAU / 4.;
                        let point = center + point(px(a.cos() * r), px(a.sin() * r * 0.5));
                        if j == 0 {
                            line.move_to(point);
                        } else {
                            line.line_to(point);
                        }
                    }
                    if let Ok(path) = line.build() {
                        w.paint_path(path, color);
                    }
                }
            }
        } else if i == 0 {
            let (x, y, h) = (n.x, n.y, n.half);
            for j in 0..if frame.time >= CONNECTED { 5 } else { 3 } {
                let shift = j as f32 * h * 0.14;
                draw_shape(
                    w,
                    &[
                        at((x - h * 0.13 - shift, y - h * 0.33 + shift * 0.5)),
                        at((x + h * 0.64 - shift, y + h * 0.055 + shift * 0.5)),
                    ],
                    None,
                    p.ink
                        .opacity(0.08 + if frame.time >= CONNECTED { 0.05 } else { 0. }),
                );
            }
        } else {
            let x = n.x + n.half * 0.8;
            dot(w, at((x, n.y + n.depth * 0.58)), 1.5, p.ink.opacity(0.22));
        }
    }
    for (i, route) in geo.routes.iter().enumerate() {
        let mut path = PathBuilder::stroke(px(1.));
        path.move_to(at(route.start));
        path.curve_to(at(route.end), at(route.control));
        if let Ok(path) = path.build() {
            let ink = if frame.stage > i {
                accent.opacity(0.32 * frame.alpha())
            } else {
                p.ink.opacity(0.19)
            };
            w.paint_path(path, ink);
        }
        let port = if i == 0 { route.start } else { route.end };
        dot(w, at(port), 2., p.ink.opacity(0.27));
        dot(w, at(port), 1., p.panel);
    }
    if !reduced && let Some((index, progress, reverse, _)) = frame.packet() {
        let route = &geo.routes[index];
        let (x, y) = route.at(progress);
        let floor = route.start.1 + (route.end.1 - route.start.1) * progress + 25.;
        dot(
            w,
            at((x, floor)),
            (3.8 - (floor - y).abs() * 0.02).clamp(1.4, 3.8),
            p.ink.opacity(0.13),
        );
        for i in (1..=5).rev() {
            let delta = i as f32 * 0.009 * if reverse { 1. } else { -1. };
            dot(
                w,
                at(route.at(progress + delta)),
                2.3,
                accent.opacity((6 - i) as f32 * 0.1),
            );
        }
        dot(w, at((x, y)), 3.5, accent);
    }
}

impl DnsResolution {
    fn node_meta(&self, index: usize, tld: &str, base: &str) -> SharedString {
        self.options.nodes[index]
            .meta
            .clone()
            .unwrap_or_else(|| match index {
                0 => self.options.domain.clone(),
                1 => "1.1.1.1".into(),
                2 => "a.root-servers.net".into(),
                3 => format!("{tld} registry").into(),
                _ => format!("ns1.{base}").into(),
            })
    }

    fn statuses(
        &self,
        frame: Snapshot,
        tld: &str,
        nameserver: &str,
        cx: &App,
    ) -> [SharedString; 5] {
        let o = &self.options;
        let lookup = if frame.time >= CACHED {
            format!("{} {} ms", tr(cx, "Connected in", "连接耗时"), o.cached_ms)
        } else if frame.time >= CONNECTED {
            format!("{} {} ms", tr(cx, "Connected in", "连接耗时"), o.lookup_ms)
        } else {
            let ms = (o.lookup_ms as f32 * frame.fraction(0.36, 9.6)).round() as u32;
            format!("{} {ms} ms", tr(cx, "Resolving", "解析中"))
        };
        let resolver = match frame.time {
            t if t >= EXPIRE => tr(cx, "Expired", "已过期").to_string(),
            t if t >= HIT => tr(cx, "Cache hit", "命中缓存").to_string(),
            t if t >= ANSWER => tr(cx, "Answer cached", "应答已缓存").to_string(),
            t if t >= MARKS[3] => tr(cx, "Asking nameserver", "查询权威服务器").to_string(),
            t if t >= MARKS[2] => format!("{} {tld}", tr(cx, "Asking", "查询")),
            t if t >= MARKS[1] + 0.06 => tr(cx, "Asking root", "查询根服务器").to_string(),
            t if t >= 1.7 => tr(cx, "Not cached", "尚未缓存").to_string(),
            _ => tr(cx, "Waiting", "等待").to_string(),
        };
        let waiting = tr(cx, "Looking up", "查询中");
        [
            lookup.into(),
            resolver.into(),
            if frame.stage >= 2 {
                format!("{} {tld}", tr(cx, "Try", "转向")).into()
            } else {
                waiting.into()
            },
            if frame.stage >= 3 {
                format!("{} {nameserver}", tr(cx, "Try", "转向")).into()
            } else {
                waiting.into()
            },
            if frame.stage >= 4 {
                o.address.clone()
            } else {
                waiting.into()
            },
        ]
    }

    fn caption(&self, stage: usize, tld: &str, base: &str, ns: &str, cx: &App) -> String {
        let o = &self.options;
        if crate::primitives::control::UiLanguage::current(cx)
            == crate::primitives::control::UiLanguage::Chinese
        {
            return match stage {
                0 => format!("浏览器向解析器查询 {}。缓存中还没有这条记录。", o.domain),
                1 => format!("根服务器不知道地址，它会指向 {tld} 的服务器。"),
                2 => format!("{tld} 服务器指向 {base} 的权威域名服务器 {ns}。"),
                3 => format!("{ns} 返回地址 {}。", o.address),
                4 => format!(
                    "解析器将应答缓存 {} 秒，浏览器在 {} ms 内建立连接。",
                    o.ttl, o.lookup_ms
                ),
                _ => format!(
                    "TTL 到期之前，下次查询直接从缓存应答，耗时 {} ms。",
                    o.cached_ms
                ),
            };
        }
        match stage {
            0 => format!(
                "Your browser asks the resolver for {}. It is not in the cache yet.",
                o.domain
            ),
            1 => format!("The root server points the resolver to the {tld} servers."),
            2 => format!("The {tld} server points to {ns}, the nameserver for {base}."),
            3 => format!("{ns} answers with the address {}.", o.address),
            4 => format!(
                "The resolver caches the answer for {} s and the browser connects in {} ms.",
                o.ttl, o.lookup_ms
            ),
            _ => format!(
                "The next lookup comes straight from the cache in {} ms, until the TTL runs out.",
                o.cached_ms
            ),
        }
    }
}

impl Render for DnsResolution {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let radius = Theme::global(cx).tokens.radius.md;
        let accent = self.options.accent.unwrap_or(p.bright);
        let width = match self.width.get() {
            width if width > px(0.) => f32::from(width),
            _ => f32::from(w.viewport_size().width).clamp(240., 1000.),
        };
        let narrow = width < 640.;
        let geo = geometry(narrow);
        let scale = width / geo.size.0;
        let height = geo.size.1 * scale;
        let reduced = disabled_motion(cx);
        let frame = self.snapshot(w, cx);
        let stage = frame.stage;
        let (tld, base) = self.options.domains();
        let nameserver = self.node_meta(4, tld, base);
        let statuses = self.statuses(frame, tld, &nameserver, cx);
        let label_width = width * if narrow { 0.47 } else { 0.29 };
        let mut label_tops = geo.nodes.map(|node| node.label * scale);
        if narrow {
            let browser = geo.nodes[0];
            let platform_top = (browser.y - browser.half * 0.5) * scale;
            label_tops[0] = label_tops[0].min((platform_top - 58.).max(0.));
        }
        let lang = crate::primitives::control::UiLanguage::current(cx);
        let labels = [
            lang.text("Browser", "浏览器"),
            lang.text("Resolver", "解析器"),
            lang.text("Root server", "根服务器"),
            lang.text("TLD server", "顶级域服务器"),
            lang.text("Nameserver", "权威域名服务器"),
        ];
        let description = self.options.label.clone().unwrap_or_else(|| {
            format!(
                "{} {} · {} → {} → {} → {} → {} · {} · {} ms · TTL {} s · {} {} ms",
                lang.text("DNS resolution for", "DNS 解析"),
                self.options.domain,
                labels[0],
                labels[1],
                labels[2],
                tld,
                nameserver,
                self.options.address,
                self.options.lookup_ms,
                self.options.ttl,
                lang.text("Cache hit", "命中缓存"),
                self.options.cached_ms
            )
            .into()
        });
        let scene = div()
            .id("dns-scene")
            .debug_selector(|| "dns-scene".into())
            .role(Role::Image)
            .aria_label(description)
            .relative()
            .w_full()
            .h(px(height))
            .flex_none()
            .child(
                canvas(
                    |_, _, _| {},
                    move |b, _, w, _| draw_scene(w, b, geo, frame, p, accent, reduced),
                )
                .absolute()
                .inset_0()
                .size_full(),
            )
            .children(geo.nodes.iter().enumerate().map(|(i, n)| {
                let node = &self.options.nodes[i];
                let meta = self.node_meta(i, tld, base);
                let label = node.label.clone().unwrap_or_else(|| labels[i].into());
                let show_status = i < 2 || stage >= i - 1;
                let color = node.color.unwrap_or(p.ink);
                let hover = self.hovered == Some(i);
                let ready = match i {
                    0 => frame.time >= CONNECTED,
                    1 => frame.time >= ANSWER && !frame.expired,
                    4 => stage >= 4,
                    _ => false,
                };
                let detail = node.detail.clone().unwrap_or_else(|| {
                    let defaults = [
                        lang.text(
                            "Needs an address before it can connect",
                            "需要地址才能建立连接",
                        ),
                        lang.text("Finds and caches the answer", "查询并缓存应答"),
                        lang.text("Points to top-level domain servers", "指向顶级域服务器"),
                        lang.text(
                            "Points to the authoritative nameserver",
                            "指向权威域名服务器",
                        ),
                        lang.text("Holds the address record", "保存地址记录"),
                    ];
                    defaults[i].into()
                });
                let hit_top = (n.y - n.half * 0.5) * scale;
                let top = hit_top.min(label_tops[i]);
                let label_y = label_tops[i] - top;
                div()
                    .id(("dns-node", i))
                    .absolute()
                    .top(px(top))
                    .left(px(n.x * scale - label_width * 0.5))
                    .w(px(label_width))
                    .pt(px(label_y))
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(2.))
                    .text_color(color)
                    .line_height(px(18.))
                    .on_hover(cx.listener(move |s, over: &bool, _, cx| {
                        if *over {
                            s.hovered = Some(i);
                        } else if s.hovered == Some(i) {
                            s.hovered = None;
                        }
                        s.retarget(cx);
                    }))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(4.))
                            .max_w_full()
                            .when_some(node.mark.clone(), |e, mark| e.child(mark()))
                            .when_some(node.icon, |e, glyph| e.child(icon(glyph, 12., color)))
                            .child(
                                div()
                                    .min_w(px(0.))
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .text_ellipsis()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(label),
                            ),
                    )
                    .when(narrow || width >= 720., |e| {
                        e.child(
                            div()
                                .max_w_full()
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .text_ellipsis()
                                .text_size(px(12.))
                                .line_height(px(16.))
                                .text_color(p.secondary)
                                .child(meta),
                        )
                    })
                    .when(show_status, |e| {
                        e.child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(4.))
                                .max_w_full()
                                .px(px(5.))
                                .rounded(radius)
                                .border_1()
                                .border_color(p.line)
                                .bg(p.panel)
                                .text_size(px(11.))
                                .line_height(px(16.))
                                .text_color(if ready { p.ink } else { p.secondary })
                                .child(div().size(px(4.)).flex_none().rounded_full().bg(if ready {
                                    p.success
                                } else {
                                    p.faint.opacity(0.65)
                                }))
                                .child(
                                    div()
                                        .min_w(px(0.))
                                        .overflow_hidden()
                                        .whitespace_nowrap()
                                        .text_ellipsis()
                                        .child(statuses[i].clone()),
                                ),
                        )
                    })
                    .when(i == 1 && frame.time >= ANSWER, |e| {
                        let ttl = if frame.expired {
                            lang.text("Expired", "已过期").to_string()
                        } else {
                            format!(
                                "TTL {} s",
                                (self.options.ttl as f32 * frame.ttl()).ceil() as u32
                            )
                        };
                        e.child(
                            div()
                                .mt(px(6.))
                                .w(px(if narrow { 152. } else { 184. }))
                                .max_w_full()
                                .px(px(8.))
                                .py(px(5.))
                                .rounded(radius)
                                .border_1()
                                .border_color(p.line)
                                .bg(p.panel)
                                .opacity(frame.alpha())
                                .flex()
                                .flex_col()
                                .gap(px(2.))
                                .text_size(px(11.))
                                .line_height(px(14.))
                                .child(
                                    div()
                                        .max_w_full()
                                        .overflow_hidden()
                                        .whitespace_nowrap()
                                        .text_ellipsis()
                                        .text_color(p.secondary)
                                        .child(self.options.domain.clone()),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .flex_wrap()
                                        .justify_between()
                                        .gap(px(3.))
                                        .child(
                                            div()
                                                .max_w_full()
                                                .overflow_hidden()
                                                .whitespace_nowrap()
                                                .text_ellipsis()
                                                .child(format!("A {}", self.options.address)),
                                        )
                                        .child(div().text_color(p.secondary).child(ttl)),
                                ),
                        )
                    })
                    .when(hover, |e| {
                        e.child(
                            div()
                                .absolute()
                                .left_0()
                                .right_0()
                                .top(px(-13.))
                                .px(px(4.))
                                .text_center()
                                .text_size(px(10.))
                                .line_height(px(13.))
                                .text_color(p.secondary)
                                .child(detail),
                        )
                    })
            }))
            .when_some(
                if reduced { None } else { frame.packet() },
                |e, (index, progress, reverse, answer)| {
                    let (x, y) = geo.routes[index].at(progress);
                    let record = if answer && reverse {
                        format!("A {}", self.options.address)
                    } else if reverse && index == 1 {
                        format!("NS {tld}")
                    } else if reverse && index == 2 {
                        format!("NS {nameserver}")
                    } else if reverse && index == 3 {
                        format!("A {}", self.options.address)
                    } else {
                        format!("A? {}", self.options.domain)
                    };
                    let chip = (width * 0.37).clamp(92., 142.);
                    let left = (x * scale - chip * 0.5).clamp(0., (width - chip).max(0.));
                    let offset = if narrow && index == 0 { 11. } else { -31. };
                    let top = (y * scale + offset).clamp(0., (height - 22.).max(0.));
                    // Fade the tag near node readouts without changing the packet's path.
                    let opacity = geo
                        .nodes
                        .iter()
                        .enumerate()
                        .fold(1.0_f32, |opacity, (i, n)| {
                            let dx = (left + chip * 0.5 - n.x * scale).abs()
                                - (chip + label_width) * 0.5;
                            let label_top = label_tops[i];
                            let label_height = if i == 1 && frame.time >= ANSWER {
                                122.
                            } else {
                                56.
                            };
                            let dy = (top - label_top - label_height).max(label_top - top - 22.);
                            opacity.min((dx.max(dy) / 7.).clamp(0., 1.))
                        });
                    e.child(
                        div()
                            .absolute()
                            .left(px(left))
                            .top(px(top))
                            .w(px(chip))
                            .px(px(5.))
                            .h(px(22.))
                            .opacity(opacity)
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(radius)
                            .bg(p.panel)
                            .border_1()
                            .border_color(p.line)
                            .text_color(p.ink)
                            .text_size(px(11.))
                            .child(
                                div()
                                    .min_w(px(0.))
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .text_ellipsis()
                                    .child(record),
                            ),
                    )
                },
            );
        let width_cell = self.width.clone();
        let controls = self.options.controls;
        let caption = self.options.caption;
        let label = STEPS[stage];
        let paused = self.paused || reduced;
        let root = div()
            .w_full()
            .max_w(px(1000.))
            .min_w(px(0.))
            .flex()
            .flex_col()
            .items_center()
            .gap(px(12.));
        let root = if let Some(style) = &self.style {
            style(root)
        } else {
            root
        };
        root.on_children_prepainted(move |children, w, _| {
            if let Some(stage) = children.first()
                && stage.size.width != width_cell.replace(stage.size.width)
            {
                w.request_animation_frame();
            }
        })
        .id(self.id.clone())
        .child(scene)
        .when(controls, |e| {
            e.child(
                div()
                    .id("dns-scrubber")
                    .role(Role::Group)
                    .aria_label(lang.text("Lookup steps", "解析步骤"))
                    .w_full()
                    .max_w(px(640.))
                    .px(px(4.))
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(
                        face(
                            "dns-playback",
                            if paused {
                                lang.text("Play", "播放")
                            } else {
                                lang.text("Pause", "暂停")
                            },
                            36.,
                            f32::from(radius),
                            Tone::Clear,
                            cx,
                        )
                        .w(px(36.))
                        .border_1()
                        .border_color(p.line)
                        .aria_toggled(paused.into())
                        .when(reduced, |e| e.tab_stop(false).opacity(0.5))
                        .child(icon(
                            if paused { Glyph::Play } else { Glyph::Pause },
                            14.,
                            p.secondary,
                        ))
                        .on_click(cx.listener(|s, _, _, cx| {
                            if !disabled_motion(cx) {
                                s.set_paused(!s.paused, cx);
                            }
                        })),
                    )
                    .children(
                        STEPS
                            .iter()
                            .enumerate()
                            .map(|(i, (en, zh, title, title_zh))| {
                                let label = if i == 2
                                    && lang != crate::primitives::control::UiLanguage::Chinese
                                {
                                    tld
                                } else {
                                    lang.text(en, zh)
                                };
                                let progress = frame.fraction(MARKS[i], MARKS[i + 1]);
                                face(
                                    ("dns-step", i),
                                    format!(
                                        "{} {}/6 · {}",
                                        lang.text("Step", "步骤"),
                                        i + 1,
                                        lang.text(title, title_zh)
                                    ),
                                    44.,
                                    f32::from(radius),
                                    Tone::Clear,
                                    cx,
                                )
                                .track_focus(&self.focus[i])
                                .flex_1()
                                .min_w(px(0.))
                                .flex_col()
                                .gap(px(3.))
                                .text_size(px(11.))
                                .text_color(if i == stage { p.ink } else { p.secondary })
                                .aria_toggled((i == stage).into())
                                .child(
                                    div().h(px(3.)).w_full().rounded_full().bg(p.line).child(
                                        div()
                                            .h_full()
                                            .w(relative(progress))
                                            .rounded_full()
                                            .bg(accent),
                                    ),
                                )
                                .when(!narrow, |e| {
                                    e.child(
                                        div()
                                            .max_w_full()
                                            .overflow_hidden()
                                            .text_ellipsis()
                                            .child(label.to_string()),
                                    )
                                })
                                .on_click(cx.listener(move |s, _, _, cx| s.seek(i, cx)))
                                .on_key_down(cx.listener(
                                    move |s, event: &KeyDownEvent, w, cx| {
                                        let next = match event.keystroke.key.as_str() {
                                            "left" => i.saturating_sub(1),
                                            "right" => (i + 1).min(5),
                                            "home" => 0,
                                            "end" => 5,
                                            _ => return,
                                        };
                                        s.seek(next, cx);
                                        s.focus[next].focus(w, cx);
                                        w.prevent_default();
                                        cx.stop_propagation();
                                    },
                                ))
                            }),
                    ),
            )
        })
        .when(caption, |e| {
            e.child(
                div()
                    .w_full()
                    .min_h(px(66.))
                    .px(px(8.))
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .items_center()
                    .text_center()
                    .text_size(px(13.))
                    .line_height(px(18.))
                    .child(
                        div()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(lang.text(label.2, label.3)),
                    )
                    .child(div().text_color(p.secondary).child(self.caption(
                        stage,
                        tld,
                        base,
                        &nameserver,
                        cx,
                    ))),
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CACHED, DnsNode, DnsNodeKind, DnsResolution, DnsResolutionOptions, EXPIRE, MARKS, Snapshot,
        geometry, speed,
    };
    use gpui::{TestAppContext, px, size};

    #[test]
    fn clock_maps_each_chapter_and_cache_expiry_without_drift() {
        for (i, time) in MARKS[..6].iter().enumerate() {
            assert_eq!(Snapshot::at(*time).stage, i);
        }
        assert_eq!(Snapshot::at(16.).stage, 0);
        assert_eq!(Snapshot::at(CACHED).stage, 5);
        assert!(Snapshot::at(CACHED).packet().is_none());
        assert!(Snapshot::at(MARKS[5]).packet().is_none());
        assert!(Snapshot::at(CACHED).ttl() > 0.);
        assert!(Snapshot::at(EXPIRE).expired);
        assert_eq!(Snapshot::at(EXPIRE).ttl(), 0.);
        assert!(Snapshot::at(15.9).alpha() < 0.3);
    }

    #[test]
    fn layouts_switch_routes_and_keep_arc_lengths_monotonic() {
        for narrow in [false, true] {
            let geometry = geometry(narrow);
            for (i, route) in geometry.routes.iter().enumerate() {
                assert_eq!(route.at(0.), route.start);
                assert_eq!(route.at(1.), route.end);
                assert!(route.lengths.windows(2).all(|w| w[0] < w[1]));
                let node = geometry.nodes[i + 1];
                let endpoint = if i == 0 {
                    (node.x, node.y - 12.)
                } else {
                    (node.x, node.y)
                };
                assert_eq!(route.end, endpoint);
            }
        }
    }

    #[test]
    fn keyboard_seeks_paused_chapters_and_responsive_stage_fits() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (control, window) = app.add_window_view(|_, cx| {
            let options = DnsResolutionOptions {
                paused: true,
                ..Default::default()
            };
            DnsResolution::new(options, cx)
        });
        for (width, expected) in [(320., 676. * 320. / 390.), (720., 612. * 720. / 1000.)] {
            window.simulate_resize(size(px(width), px(950.)));
            for _ in 0..2 {
                window.update(|w, cx| {
                    w.simulate_next_frame(cx);
                    w.refresh();
                });
                window.run_until_parked();
            }
            let bounds = window.debug_bounds("dns-scene").unwrap();
            assert_eq!(bounds.size.width, px(width));
            assert!((f32::from(bounds.size.height) - expected).abs() < 1.);
        }
        window.update(|w, cx| {
            let focus = control.read(cx).focus[1].clone();
            focus.focus(w, cx);
        });
        window.simulate_keystrokes("end");
        assert_eq!(window.update(|_, cx| control.read(cx).step), Some(5));
        window.simulate_keystrokes("left");
        assert_eq!(window.update(|_, cx| control.read(cx).step), Some(4));
        window.simulate_keystrokes("home");
        assert_eq!(window.update(|_, cx| control.read(cx).step), Some(0));
        assert!(window.update(|_, cx| control.read(cx).paused));
        window.update(|_, cx| control.update(cx, |s, cx| s.set_paused(false, cx)));
        window.simulate_keystrokes("end");
        assert!(!window.update(|_, cx| control.read(cx).paused));
        window.update(|_, cx| {
            control.update(cx, |s, cx| {
                s.pace.reset(1.);
                s.set_paused(true, cx);
                s.seek(5, cx);
                assert_eq!(s.pace.value(cx), 0.);
                assert_eq!(s.elapsed, MARKS[5]);
            });
        });
    }

    #[test]
    fn public_defaults_and_overrides_use_the_same_record() {
        let o = DnsResolutionOptions::default();
        assert_eq!(o.domains(), (".dev", "acme.dev"));
        assert_eq!((o.ttl, o.lookup_ms, o.cached_ms), (300, 84, 1));
        let o = DnsResolutionOptions {
            domain: "example.test.".into(),
            ..o
        }
        .with_node(
            DnsNodeKind::Authoritative,
            DnsNode {
                meta: Some("ns.example.test".into()),
                ..Default::default()
            },
        );
        assert_eq!(o.domains(), (".test", "example.test"));
        assert_eq!(o.nodes[4].meta.as_deref(), Some("ns.example.test"));
        assert_eq!(speed(0.), 0.);
        assert_eq!(speed(f32::NAN), 1.);
    }
}
