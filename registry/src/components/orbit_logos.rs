//! Browse and connect integrations arranged on two responsive orbits.
use crate::{
    foundation::{Theme, ThemeMode, surface_radius},
    primitives::{
        family::{Paint, Slide, Tone, disabled_motion, face, measure},
        glyph::{Glyph, icon},
        nav::horizontal_viewport,
        presentation::{column, hint, row, tr},
    },
};
use gpui::{prelude::*, *};
use std::{
    cell::Cell,
    f32::consts::{FRAC_PI_2, PI, TAU},
    rc::Rc,
    sync::Arc,
    time::Duration,
};
use web_time::Instant;

type Mark = Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>;
type RootStyle = Box<dyn Fn(Div) -> Div>;

/// An integration and its initial connection state.
#[derive(Clone)]
pub struct OrbitLogo {
    id: SharedString,
    name: SharedString,
    mark: Option<Mark>,
    category: Option<SharedString>,
    description: SharedString,
    connected: bool,
    docs: Option<SharedString>,
}

impl OrbitLogo {
    /// Supply a stable ID and a display name. The name also works for selection.
    pub fn new(id: impl Into<SharedString>, name: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            mark: None,
            category: None,
            description: "".into(),
            connected: false,
            docs: None,
        }
    }

    /// Render a host-provided GPUI mark in the logo slot.
    pub fn with_mark(
        mut self,
        mark: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        self.mark = Some(Rc::new(mark));
        self
    }

    /// Render a full-color asset. Prefer a bundled image for offline support.
    pub fn with_image(self, source: impl Into<ImageSource>) -> Self {
        let image = source.into();
        self.with_mark(move |_, _| {
            img(image.clone())
                .size_full()
                .object_fit(ObjectFit::Contain)
                .into_any_element()
        })
    }

    /// Render SVG path data in a 24×24 view box. None follows the light or dark ink.
    pub fn with_path(self, path: &str, color: Option<Hsla>) -> Self {
        let path = path
            .replace('&', "&amp;")
            .replace('"', "&quot;")
            .replace('<', "&lt;");
        let image = |ink: Hsla| {
            let Rgba { r, g, b, a } = ink.to_rgb();
            let (r, g, b) = (
                (r * 255.).round() as u8,
                (g * 255.).round() as u8,
                (b * 255.).round() as u8,
            );
            Arc::new(Image::from_bytes(ImageFormat::Svg, format!(
                r##"<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24"><path fill="#{r:02x}{g:02x}{b:02x}" opacity="{a}" d="{path}"/></svg>"##
            ).into_bytes()))
        };
        let light = image(color.unwrap_or_else(|| Paint::for_mode(ThemeMode::Light).ink));
        let dark = color.map_or_else(
            || image(Paint::for_mode(ThemeMode::Dark).ink),
            |_| light.clone(),
        );
        self.with_mark(move |_, cx| {
            let image = if Theme::global(cx).mode == ThemeMode::Dark {
                &dark
            } else {
                &light
            };
            img(image.clone())
                .size_full()
                .object_fit(ObjectFit::Contain)
                .into_any_element()
        })
    }

    /// Group the logo under a filter chip.
    pub fn with_category(mut self, category: impl Into<SharedString>) -> Self {
        self.category = Some(category.into());
        self
    }

    /// Set the supporting line in the detail card.
    pub fn with_description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = description.into();
        self
    }

    /// Set the initial connection state, which defaults to disconnected.
    pub fn with_connected(mut self, connected: bool) -> Self {
        self.connected = connected;
        self
    }

    /// Add a documentation action for an HTTP or HTTPS destination.
    pub fn with_docs(mut self, url: impl Into<SharedString>) -> Self {
        let url = url.into();
        self.docs = (url.starts_with("https://") || url.starts_with("http://")).then_some(url);
        self
    }
}

/// A user selection request. None indicates that the card was closed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrbitSelectionChanged {
    pub id: Option<SharedString>,
    pub index: Option<usize>,
}

/// A connection request that can be completed once by its ticket.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrbitConnectRequested {
    pub id: SharedString,
    pub index: usize,
    pub ticket: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Connection {
    Disconnected,
    Pending(u64),
    Connected,
    Failed,
}

const PERIODS: [f32; 2] = [64., -88.];
const PANEL_WIDTH: f32 = 320.;
const STAGE_MAX: f32 = 460.;
const SIDE_BY_SIDE: f32 = 760.;

#[derive(Clone, Copy)]
struct Slot {
    ring: usize,
    angle: f32,
}

fn slots(count: usize) -> Vec<Slot> {
    let inner = (count * 5).div_ceil(14).max(1);
    let outer = count.saturating_sub(inner).max(1);
    (0..count)
        .map(|index| {
            if index < inner {
                Slot {
                    ring: 0,
                    angle: -0.36 + TAU * index as f32 / inner as f32,
                }
            } else {
                Slot {
                    ring: 1,
                    angle: -0.40 + TAU * (index - inner) as f32 / outer as f32,
                }
            }
        })
        .collect()
}

fn nearest_turn(current: f32, target: f32) -> f32 {
    current + (target - current + PI).rem_euclid(TAU) - PI
}

#[derive(Clone, Copy)]
struct Geometry {
    size: f32,
    chip: f32,
    core: f32,
    radii: [f32; 2],
}

impl Geometry {
    fn new(size: f32) -> Self {
        let chip = (40. + (size - 342.) * 12. / 118.).round().clamp(32., 52.);
        let core = (60. + (size - 342.) * 18. / 118.).round().clamp(46., 78.);
        let outer = (size / 2. - chip / 2. - 2.).max(0.);
        Self {
            size,
            chip,
            core,
            radii: [outer * 0.57, outer],
        }
    }

    fn center(self) -> Point<Pixels> {
        point(px(self.size * 0.5), px(self.size * 0.5))
    }

    fn position(self, slot: Slot, phase: [f32; 2]) -> Point<Pixels> {
        let (sine, cosine) = (slot.angle + phase[slot.ring]).sin_cos();
        let radius = px(self.radii[slot.ring]);
        self.center() + point(radius * cosine, radius * sine)
    }
}

/// Two rotating rings with a filter strip and an integration detail card.
pub struct OrbitLogos {
    id: ElementId,
    logos: Vec<OrbitLogo>,
    slots: Vec<Slot>,
    categories: Vec<SharedString>,
    connections: Vec<Connection>,
    ticket: u64,
    host_connections: bool,
    controlled: bool,
    selected: Option<usize>,
    closing: Option<usize>,
    active: usize,
    hovered: Option<usize>,
    filter: Option<usize>,
    announcement: SharedString,
    label: SharedString,
    all_label: SharedString,
    center: Option<Mark>,
    style: Option<RootStyle>,
    speed: f32,
    paused: bool,
    visible: bool,
    pointer_inside: bool,
    wide: bool,
    focus: FocusHandle,
    logo_focus: Vec<FocusHandle>,
    focus_changes: Vec<Subscription>,
    filter_scroll: ScrollHandle,
    box_bounds: Rc<Cell<Bounds<Pixels>>>,
    stage_bounds: Rc<Cell<Bounds<Pixels>>>,
    panel_bounds: Rc<Cell<Bounds<Pixels>>>,
    phase: [f32; 2],
    offsets: [Slide; 2],
    rate: Slide,
    last_frame: Option<Instant>,
    shell: Slide,
    content: Slide,
    direction: f32,
    filter_x: Slide,
    filter_width: Slide,
    filter_geometry: Vec<(f32, f32)>,
}

impl EventEmitter<OrbitSelectionChanged> for OrbitLogos {}
impl EventEmitter<OrbitConnectRequested> for OrbitLogos {}

impl OrbitLogos {
    /// Build an unfiltered orbit with no card open.
    pub fn new(logos: Vec<OrbitLogo>, cx: &mut Context<Self>) -> Self {
        let categories = logos.iter().filter_map(|logo| logo.category.clone()).fold(
            Vec::new(),
            |mut categories, category| {
                if !categories.contains(&category) {
                    categories.push(category);
                }
                categories
            },
        );
        let connections = logos
            .iter()
            .map(|logo| {
                if logo.connected {
                    Connection::Connected
                } else {
                    Connection::Disconnected
                }
            })
            .collect();
        let logo_focus = logos.iter().map(|_| cx.focus_handle()).collect();
        let slots = slots(logos.len());
        Self {
            id: "orbit-logos".into(),
            logos,
            slots,
            categories,
            connections,
            ticket: 0,
            host_connections: false,
            controlled: false,
            selected: None,
            closing: None,
            active: 0,
            hovered: None,
            filter: None,
            announcement: "".into(),
            label: tr(cx, "Integrations", "集成").into(),
            all_label: tr(cx, "All", "全部").into(),
            center: None,
            style: None,
            speed: 1.,
            paused: false,
            visible: true,
            pointer_inside: false,
            wide: false,
            focus: cx.focus_handle(),
            logo_focus,
            focus_changes: Vec::new(),
            filter_scroll: ScrollHandle::new(),
            box_bounds: Rc::new(Cell::new(Bounds::default())),
            stage_bounds: Rc::new(Cell::new(Bounds::default())),
            panel_bounds: Rc::new(Cell::new(Bounds::default())),
            phase: [0.; 2],
            offsets: [Slide::new(0.); 2],
            rate: Slide::new(1.),
            last_frame: None,
            shell: Slide::new(0.),
            content: Slide::new(1.),
            direction: 0.,
            filter_x: Slide::new(0.),
            filter_width: Slide::new(0.),
            filter_geometry: Vec::new(),
        }
    }

    /// Set a stable GPUI identity for the root.
    pub fn with_id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = id.into();
        self
    }

    /// Customize the integration list's accessible and idle title.
    pub fn with_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = label.into();
        self
    }

    /// Customize the unfiltered chip, which defaults to All.
    pub fn with_all_label(mut self, label: impl Into<SharedString>) -> Self {
        self.all_label = label.into();
        self
    }

    /// Render content inside the center disc.
    pub fn with_center(
        mut self,
        center: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        self.center = Some(Rc::new(center));
        self
    }

    /// Customize root layout using GPUI styles.
    pub fn with_style(mut self, style: impl Fn(Div) -> Div + 'static) -> Self {
        self.style = Some(Box::new(style));
        self
    }

    /// Multiply the default turn rates. Zero holds the current orbit.
    pub fn with_speed(mut self, speed: f32) -> Self {
        self.speed = if speed.is_finite() { speed.max(0.) } else { 1. };
        self
    }

    /// Hold ambient rotation from the first frame.
    pub fn with_paused(mut self, paused: bool) -> Self {
        self.paused = paused;
        if paused {
            self.rate.reset(0.);
        }
        self
    }

    /// Open a logo by ID or name on first render.
    pub fn with_default_selected(mut self, id: &str) -> Self {
        self.selected = self.find(id);
        self.active = self.selected.unwrap_or(self.active);
        if self.selected.is_some() {
            self.shell.reset(1.);
        }
        self
    }

    /// Keep selection controlled by the host. Use set_selected to accept requests.
    pub fn with_selected(mut self, id: Option<&str>) -> Self {
        self.controlled = true;
        if let Some(id) = id {
            self.with_default_selected(id)
        } else {
            self.selected = None;
            self.shell.reset(0.);
            self
        }
    }

    /// Keep a connection pending until the host resolves its ticket.
    pub fn with_host_connections(mut self) -> Self {
        self.host_connections = true;
        self
    }

    /// Hold or resume ambient rotation.
    pub fn set_paused(&mut self, paused: bool, cx: &mut Context<Self>) {
        if self.paused != paused {
            self.paused = paused;
            cx.notify();
        }
    }

    /// Suspend paint scheduling when the host is hidden.
    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        if self.visible != visible {
            self.visible = visible;
            self.last_frame = None;
            cx.notify();
        }
    }

    /// Synchronize a controlled selection without emitting a request.
    pub fn set_selected(&mut self, id: Option<&str>, cx: &mut Context<Self>) {
        self.show(id.and_then(|id| self.find(id)), 0., cx);
    }

    /// Resolve an outstanding connection once. Returns false for stale tickets.
    pub fn finish_connect(&mut self, ticket: u64, success: bool, cx: &mut Context<Self>) -> bool {
        let Some(index) = self
            .connections
            .iter()
            .position(|state| *state == Connection::Pending(ticket))
        else {
            return false;
        };
        self.connections[index] = if success {
            Connection::Connected
        } else {
            Connection::Failed
        };
        self.announcement = if success {
            tr(cx, "Connected", "已连接")
        } else {
            tr(cx, "Connection failed. Try again.", "连接失败，请重试。")
        }
        .into();
        cx.notify();
        true
    }

    fn find(&self, id: &str) -> Option<usize> {
        self.logos
            .iter()
            .position(|logo| logo.id == id)
            .or_else(|| self.logos.iter().position(|logo| logo.name == id))
    }

    fn matches(&self, index: usize) -> bool {
        self.filter
            .is_none_or(|i| self.logos[index].category.as_ref() == Some(&self.categories[i]))
    }

    fn matching(&self) -> impl DoubleEndedIterator<Item = usize> + '_ {
        (0..self.logos.len()).filter(|&i| self.matches(i))
    }

    fn park(&mut self, cx: &App, immediate: bool) {
        let direction = if self.wide { 0. } else { FRAC_PI_2 };
        let choices = [0, 1].map(|ring| {
            self.selected
                .filter(|&index| self.slots[index].ring == ring)
                .or_else(|| {
                    self.filter
                        .and_then(|_| self.matching().find(|&i| self.slots[i].ring == ring))
                })
        });
        for (ring, choice) in choices.into_iter().enumerate() {
            let here = self.offsets[ring].value(cx);
            let target = if self
                .selected
                .is_some_and(|i| self.slots[i].ring == 1 && ring == 0)
            {
                let count = self
                    .slots
                    .iter()
                    .filter(|slot| slot.ring == 0)
                    .count()
                    .max(1);
                let half_step = PI / count as f32;
                let first = self.slots[0].angle + self.phase[0];
                let slot = ((direction - first - here - half_step) / (2. * half_step)).round();
                direction - first - half_step - slot * 2. * half_step
            } else if let Some(i) = choice {
                direction - self.slots[i].angle - self.phase[ring]
            } else {
                continue;
            };
            let target = nearest_turn(here, target);
            if immediate {
                self.offsets[ring].reset(target);
            } else {
                self.offsets[ring].duration(target, 0.42, cx);
            }
        }
    }

    fn show(&mut self, next: Option<usize>, direction: f32, cx: &mut Context<Self>) {
        if self.selected == next {
            return;
        }
        let before = self.selected;
        self.selected = next;
        self.closing = if next.is_none() { before } else { None };
        self.shell
            .duration(if next.is_some() { 1. } else { 0. }, 0.42, cx);
        if let Some(i) = next {
            self.active = i;
            self.direction = direction;
            self.content.reset(0.);
            self.content
                .duration(1., if before.is_some() { 0.28 } else { 0.42 }, cx);
            self.announcement =
                format!("{} · {}", self.logos[i].name, self.logos[i].description).into();
        } else {
            self.announcement = tr(cx, "Details closed", "详情已关闭").into();
        }
        self.park(cx, false);
        cx.notify();
    }

    fn select(&mut self, next: Option<usize>, direction: f32, cx: &mut Context<Self>) {
        if self.selected == next {
            return;
        }
        if !self.controlled {
            self.show(next, direction, cx);
        }
        cx.emit(OrbitSelectionChanged {
            id: next.map(|i| self.logos[i].id.clone()),
            index: next,
        });
    }

    fn filter_by(&mut self, filter: Option<usize>, cx: &mut Context<Self>) {
        if self.filter == filter {
            return;
        }
        self.filter = filter;
        if self.selected.is_some_and(|i| !self.matches(i)) {
            self.select(None, 0., cx);
        }
        if !self.logos.is_empty() && !self.matches(self.active) {
            let first = self.matching().next().unwrap_or(0);
            self.active = first;
        }
        self.park(cx, false);
        let name = filter.map_or(&self.all_label, |i| &self.categories[i]);
        self.announcement = format!("{} · {}", name, self.matching().count()).into();
        cx.notify();
    }

    fn activate(&mut self, index: usize, w: &mut Window, cx: &mut Context<Self>) {
        self.active = index;
        self.logo_focus[index].focus(w, cx);
        self.select((self.selected != Some(index)).then_some(index), 0., cx);
        cx.notify();
    }

    fn close(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if let Some(i) = self.selected {
            self.active = i;
            self.logo_focus[i].focus(w, cx);
            self.select(None, 0., cx);
        }
    }

    fn move_to(
        &mut self,
        key: &str,
        restore_logo_focus: bool,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (next, direction) = {
            let mut eligible = self.matching();
            match key {
                "home" => (eligible.next(), -1.),
                "end" => (eligible.next_back(), 1.),
                "left" | "up" => (
                    eligible
                        .rfind(|i| *i < self.active)
                        .or_else(|| self.matching().next_back()),
                    -1.,
                ),
                _ => (
                    eligible
                        .find(|i| *i > self.active)
                        .or_else(|| self.matching().next()),
                    1.,
                ),
            }
        };
        if let Some(i) = next {
            self.active = i;
            if restore_logo_focus {
                self.logo_focus[i].focus(w, cx);
            }
            if self.selected.is_some() {
                self.select(Some(i), direction, cx);
            }
            cx.notify();
        }
    }

    fn connect(&mut self, index: usize, cx: &mut Context<Self>) {
        if matches!(
            self.connections[index],
            Connection::Pending(_) | Connection::Connected
        ) {
            return;
        }
        self.ticket += 1;
        let ticket = self.ticket;
        self.connections[index] = Connection::Pending(ticket);
        self.announcement = tr(cx, "Connecting", "正在连接").into();
        cx.emit(OrbitConnectRequested {
            id: self.logos[index].id.clone(),
            index,
            ticket,
        });
        if !self.host_connections {
            cx.spawn(async move |this, cx| {
                cx.background_executor()
                    .timer(Duration::from_millis(720))
                    .await;
                let _ = this.update(cx, |orbit, cx| orbit.finish_connect(ticket, true, cx));
            })
            .detach();
        }
        cx.notify();
    }
}

fn line(from: Point<Pixels>, to: Point<Pixels>, width: f32, color: Hsla, w: &mut Window) {
    let mut path = PathBuilder::stroke(px(width));
    path.move_to(from);
    path.line_to(to);
    if let Ok(path) = path.build() {
        w.paint_path(path, color);
    }
}

fn circle(center: Point<Pixels>, radius: f32, color: Hsla, w: &mut Window) {
    let radius = px(radius);
    let extent = point(radius, radius);
    let bounds = Bounds::from_corners(center - extent, center + extent);
    w.paint_quad(quad(
        bounds,
        radius,
        transparent_black(),
        px(1.),
        color,
        BorderStyle::Solid,
    ));
}

fn mark(logo: &OrbitLogo, side: f32, w: &mut Window, cx: &mut App) -> Div {
    div()
        .size(px(side))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .child(logo.mark.as_ref().map_or_else(
            || {
                div()
                    .text_size(px(side * 0.72))
                    .font_weight(FontWeight::MEDIUM)
                    .child(
                        logo.name
                            .chars()
                            .next()
                            .map_or_else(String::new, |ch| ch.to_string()),
                    )
                    .into_any_element()
            },
            |draw| draw(w, cx),
        ))
}

impl OrbitLogos {
    fn filter_strip(
        &mut self,
        p: Paint,
        time: Instant,
        reduced: bool,
        moving: &mut bool,
        w: &Window,
        cx: &Context<Self>,
    ) -> Div {
        let labels: Vec<_> = std::iter::once(&self.all_label)
            .chain(&self.categories)
            .cloned()
            .collect();
        let mut x = 0.;
        let geometry: Vec<_> = labels
            .iter()
            .map(|label| {
                let width = measure(label, 14., w).ceil() + 28.;
                let result = (x, width);
                x += width + 4.;
                result
            })
            .collect();
        let (x, width) = geometry[self.filter.map_or(0, |i| i + 1)];
        if geometry != self.filter_geometry {
            self.filter_x.reset(x);
            self.filter_width.reset(width);
            self.filter_geometry = geometry;
        }
        self.filter_x.duration(x, 0.42, cx);
        self.filter_width.duration(width, 0.42, cx);
        let (x, a) = self.filter_x.sample_batch(time, reduced);
        let (width, b) = self.filter_width.sample_batch(time, reduced);
        *moving |= a || b;
        horizontal_viewport(
            "orbit-categories",
            &self.filter_scroll,
            row()
                .relative()
                .w(px(self.filter_geometry.last().map_or(0., |&(x, w)| x + w)))
                .h(px(36.))
                .gap(px(4.))
                .child(
                    div()
                        .absolute()
                        .left(px(x))
                        .top(px(2.))
                        .w(px(width))
                        .h(px(32.))
                        .rounded_full()
                        .bg(p.hover),
                )
                .children(labels.into_iter().enumerate().map(|(i, label)| {
                    let category = i.checked_sub(1);
                    face(
                        ("orbit-category", i),
                        label.clone(),
                        32.,
                        999.,
                        Tone::Clear,
                        cx,
                    )
                    .w(px(self.filter_geometry[i].1))
                    .rounded_full()
                    .aria_toggled((self.filter == category).into())
                    .on_click(cx.listener(move |s, _, _, cx| s.filter_by(category, cx)))
                    .child(label)
                })),
        )
    }

    fn card(
        &self,
        index: usize,
        alpha: f32,
        content: f32,
        p: Paint,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let logo = &self.logos[index];
        let state = self.connections[index];
        let (status, status_color, status_glyph) = match state {
            Connection::Disconnected => (tr(cx, "Not connected", "未连接"), p.secondary, None),
            Connection::Pending(_) => (
                tr(cx, "Connecting", "正在连接"),
                p.secondary,
                Some(Glyph::Clock),
            ),
            Connection::Connected => (tr(cx, "Connected", "已连接"), p.success, Some(Glyph::Check)),
            Connection::Failed => (
                tr(cx, "Connection failed", "连接失败"),
                p.danger,
                Some(Glyph::Alert),
            ),
        };
        let radius = f32::from(surface_radius(cx));
        let nav = [
            (
                "orbit-prev",
                tr(cx, "Previous integration", "上一个集成"),
                "left",
                Glyph::Arrow,
            ),
            (
                "orbit-next",
                tr(cx, "Next integration", "下一个集成"),
                "right",
                Glyph::Arrow,
            ),
            (
                "orbit-close",
                tr(cx, "Close", "关闭"),
                "escape",
                Glyph::Close,
            ),
        ];
        let connect = match state {
            Connection::Failed => tr(cx, "Try again", "重试"),
            Connection::Pending(_) => tr(cx, "Connecting", "正在连接"),
            _ => tr(cx, "Connect", "连接"),
        };
        column()
            .relative()
            .p(px(16.))
            .gap(px(12.))
            .min_h(px(182.))
            .opacity(alpha)
            .child(
                row()
                    .gap(px(12.))
                    .justify_between()
                    .child(
                        row()
                            .gap(px(12.))
                            .flex_1()
                            .child(mark(logo, 40., w, cx))
                            .child(
                                column()
                                    .flex_1()
                                    .child(
                                        div()
                                            .truncate()
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_size(px(16.))
                                            .child(logo.name.clone()),
                                    )
                                    .when_some(logo.category.clone(), |d, label| {
                                        d.child(hint(label, p))
                                    }),
                            ),
                    )
                    .child(row().gap(px(2.)).flex_none().children(nav.map(
                        |(id, label, key, glyph)| {
                            face(id, label, 32., radius, Tone::Clear, cx)
                                .w(px(32.))
                                .on_click(cx.listener(move |s, _, w, cx| {
                                    if key == "escape" {
                                        s.close(w, cx);
                                    } else {
                                        s.move_to(key, false, w, cx);
                                    }
                                }))
                                .child(if key == "left" {
                                    div()
                                        .child(
                                            canvas(
                                                |_, _, _| (),
                                                move |b, _, w, _| {
                                                    let left =
                                                        point(b.left() + px(2.), b.center().y);
                                                    line(
                                                        left,
                                                        point(b.right() - px(2.), left.y),
                                                        1.5,
                                                        p.secondary,
                                                        w,
                                                    );
                                                    line(
                                                        left,
                                                        left + point(px(5.), px(-5.)),
                                                        1.5,
                                                        p.secondary,
                                                        w,
                                                    );
                                                    line(
                                                        left,
                                                        left + point(px(5.), px(5.)),
                                                        1.5,
                                                        p.secondary,
                                                        w,
                                                    );
                                                },
                                            )
                                            .size(px(16.)),
                                        )
                                        .into_any_element()
                                } else {
                                    icon(glyph, 16., p.secondary).into_any_element()
                                })
                        },
                    ))),
            )
            .child(
                div()
                    .relative()
                    .left(px(self.direction * 10. * (1. - content)))
                    .text_size(px(14.))
                    .line_height(px(19.6))
                    .text_color(p.secondary)
                    .child(logo.description.clone()),
            )
            .child(
                row()
                    .mt_auto()
                    .pt(px(12.))
                    .border_t_1()
                    .border_color(p.line)
                    .gap(px(8.))
                    .flex_wrap()
                    .justify_between()
                    .child(
                        row()
                            .gap(px(6.))
                            .text_size(px(13.))
                            .text_color(status_color)
                            .when_some(status_glyph, |row, glyph| {
                                row.child(icon(glyph, 14., status_color))
                            })
                            .child(status),
                    )
                    .child(
                        row()
                            .gap(px(4.))
                            .when_some(logo.docs.clone(), |row, href| {
                                row.child(
                                    face(
                                        "orbit-docs",
                                        tr(cx, "Docs", "文档"),
                                        32.,
                                        radius,
                                        Tone::Clear,
                                        cx,
                                    )
                                    .role(Role::Link)
                                    .px(px(8.))
                                    .on_click(move |_, _, cx| cx.open_url(href.as_ref()))
                                    .child(tr(cx, "Docs", "文档"))
                                    .child(icon(
                                        Glyph::Link,
                                        13.,
                                        p.secondary,
                                    )),
                                )
                            })
                            .when(state != Connection::Connected, |row| {
                                row.child(
                                    face("orbit-connect", connect, 32., radius, Tone::Primary, cx)
                                        .px(px(10.))
                                        .when(matches!(state, Connection::Pending(_)), |d| {
                                            d.tab_stop(false).opacity(0.55)
                                        })
                                        .on_click(
                                            cx.listener(move |s, _, _, cx| s.connect(index, cx)),
                                        )
                                        .child(connect),
                                )
                            }),
                    ),
            )
    }
}

impl Render for OrbitLogos {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let reduced = disabled_motion(cx);
        let time = cx.background_executor().now();
        if self.focus_changes.is_empty() {
            self.focus_changes
                .push(cx.on_focus_in(&self.focus, w, |_, _, cx| cx.notify()));
            self.focus_changes
                .push(cx.on_focus_out(&self.focus, w, |_, _, _, cx| cx.notify()));
        }
        let root_bounds = self.box_bounds.get();
        let width = if root_bounds.size.width > px(0.) {
            f32::from(root_bounds.size.width)
        } else {
            f32::from(w.viewport_size().width).min(920.)
        };
        let wide = width >= SIDE_BY_SIDE;
        if wide != self.wide || root_bounds.size.width == px(0.) {
            self.wide = wide;
            self.park(cx, true);
        }
        let side = (width - if wide { PANEL_WIDTH + 16. } else { 0. }).clamp(100., STAGE_MAX);
        let geo = Geometry::new(side);
        let root_visible = root_bounds.size.width > px(0.)
            && root_bounds.top() < w.viewport_size().height
            && root_bounds.bottom() > px(0.);
        let on_screen = self.visible && w.is_visible() && root_visible;
        let orbit_idle = self.selected.is_none() && self.filter.is_none();
        let can_rotate = !self.paused
            && !self.pointer_inside
            && !self.focus.contains_focused(w, cx)
            && orbit_idle;
        self.rate
            .duration(if can_rotate { 1. } else { 0. }, 0.38, cx);
        let (rate, slowing) = self.rate.sample_batch(time, reduced);
        let previous = self.last_frame.replace(time);
        let dt = if on_screen && !reduced {
            previous.map_or(0., |then| (time - then).as_secs_f32().min(0.05))
        } else {
            0.
        };
        if !on_screen {
            self.last_frame = None;
        }
        let mut moving = !reduced && orbit_idle && self.speed > 0. && (rate > 0. || slowing);
        let phases = std::array::from_fn(|i| {
            if orbit_idle {
                self.phase[i] =
                    (self.phase[i] + dt * rate * self.speed * TAU / PERIODS[i]).rem_euclid(TAU);
            }
            let (offset, active) = self.offsets[i].sample_batch(time, reduced);
            moving |= active;
            self.phase[i] + offset
        });
        let (shell, opening) = self.shell.sample_batch(time, reduced);
        let (content, reveal) = self.content.sample_batch(time, reduced);
        moving |= opening || reveal;
        let shell = shell.clamp(0., 1.);
        let content = content.clamp(0., 1.);
        let selected = self.selected;
        let chip_centers: Vec<_> = self
            .slots
            .iter()
            .map(|&slot| geo.position(slot, phases))
            .collect();
        let center = geo.center();
        let connected = self
            .connections
            .iter()
            .filter(|&&state| state == Connection::Connected)
            .count();
        let shown = self.matching().count();
        let shown_connected = self
            .matching()
            .filter(|&i| self.connections[i] == Connection::Connected)
            .count();
        let accent = p.bright;
        let ring_color = p.line.blend(p.secondary.opacity(0.18));
        let selected_ring = selected.map(|i| self.slots[i].ring);
        let selected_center = selected.map(|i| chip_centers[i]);
        let canvas_rings = canvas(
            |_, _, _| (),
            move |bounds, _, w, _| {
                for (i, radius) in geo.radii.into_iter().enumerate() {
                    let color = if selected_ring == Some(i) {
                        ring_color.blend(accent.opacity(0.25))
                    } else {
                        ring_color
                    };
                    circle(bounds.origin + center, radius, color, w);
                }
                if let Some(dest) = selected_center {
                    let delta = dest - center;
                    let length = f32::from(delta.x).hypot(f32::from(delta.y));
                    if length > geo.core * 0.5 + geo.chip * 0.55 + 10. {
                        let vector = point(delta.x / length, delta.y / length);
                        let from = bounds.origin + center + vector * (geo.core * 0.5 + 6.);
                        let to = bounds.origin + dest - vector * (geo.chip * 0.55 + 6.);
                        line(
                            from,
                            from + (to - from) * shell,
                            1.5,
                            accent.opacity(shell),
                            w,
                        );
                    }
                }
            },
        )
        .absolute()
        .inset_0();

        let inner = self.center.as_ref().map_or_else(
            || {
                div()
                    .size(px(15.))
                    .rounded(px(4.))
                    .bg(accent)
                    .into_any_element()
            },
            |draw| draw(w, cx),
        );
        let core = div()
            .absolute()
            .left(center.x - px(geo.core * 0.5))
            .top(center.y - px(geo.core * 0.5))
            .size(px(geo.core))
            .rounded_full()
            .border_1()
            .border_color(ring_color)
            .bg(p.panel)
            .shadow_sm()
            .flex()
            .justify_center()
            .items_center()
            .child(inner);

        let mut order: Vec<_> = (0..self.logos.len()).collect();
        let keyboard = w.last_input_was_keyboard();
        order.sort_by_key(|&i| {
            if selected == Some(i) {
                2
            } else if self.hovered == Some(i) || keyboard && self.logo_focus[i].is_focused(w) {
                1
            } else {
                0
            }
        });
        let stage_bounds = self.stage_bounds.clone();
        let stage_layout = stage_bounds.get();
        let stage = div()
            .relative()
            .size(px(side))
            .flex_none()
            .child(canvas_rings)
            .child(
                canvas(
                    move |bounds, _, _| stage_bounds.set(bounds),
                    |_, _, _, _| (),
                )
                .absolute()
                .inset_0(),
            )
            .child(core)
            .child(
                div()
                    .id("orbit-logo-list")
                    .role(Role::List)
                    .aria_label(self.label.clone())
                    .absolute()
                    .inset_0()
                    .children(order.into_iter().map(|i| {
                        let logo = &self.logos[i];
                        let enabled = self.matches(i);
                        let opened = selected == Some(i);
                        let hovered = self.hovered == Some(i) && !w.last_input_was_touch();
                        let lifted =
                            enabled && (hovered || keyboard && self.logo_focus[i].is_focused(w));
                        let connected = self.connections[i] == Connection::Connected;
                        let size = geo.chip * if opened || lifted { 1.10 } else { 1. };
                        let pos = chip_centers[i] - point(px(size * 0.5), px(size * 0.5));
                        let accessible = [
                            Some(logo.name.as_ref()),
                            logo.category.as_deref(),
                            connected.then_some(tr(cx, "connected", "已连接")),
                        ]
                        .into_iter()
                        .flatten()
                        .collect::<Vec<_>>()
                        .join(", ");
                        let body = mark(logo, (geo.chip * 0.45).round(), w, cx);
                        let caption_width = (measure(&logo.name, 12., w) + 16.).max(size);
                        div()
                            .absolute()
                            .left(pos.x)
                            .top(pos.y)
                            .size(px(size))
                            .id(("orbit-slot", i))
                            .role(Role::ListItem)
                            .child(
                                face(("orbit-logo", i), accessible, size, 999., Tone::Surface, cx)
                                    .relative()
                                    .size_full()
                                    .rounded_full()
                                    .track_focus(&self.logo_focus[i])
                                    .tab_stop(enabled && i == self.active)
                                    .aria_toggled(opened.into())
                                    .border_1()
                                    .border_color(if opened { accent } else { ring_color })
                                    .shadow_sm()
                                    .opacity(if enabled { 1. } else { 0.27 })
                                    .on_hover(cx.listener(move |s, hovering, w, cx| {
                                        let next = if *hovering && !w.last_input_was_touch() {
                                            Some(i)
                                        } else {
                                            s.hovered.filter(|&v| v != i)
                                        };
                                        if next != s.hovered {
                                            s.hovered = next;
                                            cx.notify();
                                        }
                                    }))
                                    .on_click(cx.listener(move |s, _, w, cx| {
                                        if enabled {
                                            s.activate(i, w, cx);
                                        }
                                    }))
                                    .on_key_down(cx.listener(
                                        move |s, event: &KeyDownEvent, w, cx| {
                                            let key = event.keystroke.key.as_str();
                                            match key {
                                                "left" | "right" | "up" | "down" | "home"
                                                | "end" => s.move_to(key, true, w, cx),
                                                "enter" | "space" if enabled => {
                                                    if !event.is_held {
                                                        s.activate(i, w, cx);
                                                    }
                                                }
                                                _ => return,
                                            }
                                            w.prevent_default();
                                            cx.stop_propagation();
                                        },
                                    ))
                                    .child(body)
                                    .when(connected, |d| {
                                        d.child(
                                            div()
                                                .absolute()
                                                .right(px(1.))
                                                .bottom(px(1.))
                                                .size(px(9.))
                                                .rounded_full()
                                                .border_1()
                                                .border_color(p.panel)
                                                .bg(p.success),
                                        )
                                    }),
                            )
                            .when(lifted && !opened, |d| {
                                d.child(
                                    div()
                                        .absolute()
                                        .top(px(size + 4.))
                                        .left(px((size - caption_width) * 0.5))
                                        .w(px(caption_width))
                                        .h(px(22.))
                                        .rounded(surface_radius(cx))
                                        .flex()
                                        .justify_center()
                                        .items_center()
                                        .text_size(px(12.))
                                        .text_color(p.inverse)
                                        .bg(p.bright)
                                        .child(logo.name.clone()),
                                )
                            })
                    })),
            );

        let panel_bounds = self.panel_bounds.clone();
        let layout = panel_bounds.get();
        let mut panel = column()
            .relative()
            .w(px(if wide { PANEL_WIDTH } else { side }))
            .flex_none()
            .min_h(px(200.))
            .child(
                canvas(move |b, _, _| panel_bounds.set(b), |_, _, _, _| ())
                    .absolute()
                    .inset_0(),
            );
        if let Some(i) = selected
            .or(self.closing)
            .filter(|_| shell > 0.001 || opening)
        {
            let start_size = geo.chip * 1.10;
            let start = stage_layout.origin - layout.origin + chip_centers[i]
                - point(px(start_size / 2.), px(start_size / 2.));
            let length = if layout.size.width > px(0.) {
                layout.size.width
            } else {
                px(if wide { PANEL_WIDTH } else { side })
            };
            let card_h = (layout.size.height - px(18.)).max(px(182.));
            let radius = start_size * 0.5 * (1. - shell) + f32::from(surface_radius(cx)) * shell;
            panel = panel.child(
                div()
                    .absolute()
                    .left(start.x * (1. - shell))
                    .top(start.y * (1. - shell))
                    .w(px(start_size) * (1. - shell) + length * shell)
                    .h(px(start_size) * (1. - shell) + card_h * shell)
                    .rounded(px(radius))
                    .border_1()
                    .border_color(ring_color)
                    .bg(p.panel)
                    .shadow_sm(),
            );
            if selected.is_some() {
                let alpha = if self.direction == 0. {
                    ((shell - 0.20) / 0.80).clamp(0., 1.)
                } else {
                    content
                };
                panel = panel.child(self.card(i, alpha, content, p, w, cx));
            }
        }
        if selected.is_none() && shell < 0.85 {
            let label = self
                .filter
                .map_or(&self.label, |i| &self.categories[i])
                .clone();
            let (sub, total) = if crate::primitives::control::UiLanguage::current(cx)
                == crate::primitives::control::UiLanguage::English
            {
                (
                    format!("{shown} integrations, {shown_connected} connected"),
                    format!("{connected} of {} connected in total", self.logos.len()),
                )
            } else {
                (
                    format!("{shown} 项集成，{shown_connected} 项已连接"),
                    format!("共 {} 项集成，已连接 {connected} 项", self.logos.len()),
                )
            };
            panel = panel.child(column().p(px(16.)).gap(px(9.)).opacity(1.-shell)
                .child(column().child(div().text_size(px(16.)).font_weight(FontWeight::MEDIUM).child(label))
                    .child(hint(sub, p)))
                .child(hint(tr(cx, "Pick a logo to see what it does and connect it. Arrow keys step through them.", "选择标志查看详情或连接。使用方向键可切换。"),p))
                .child(hint(total, p)));
        }

        let strip = (!self.categories.is_empty())
            .then(|| self.filter_strip(p, time, reduced, &mut moving, w, cx));
        if moving && on_screen {
            w.request_animation_frame();
        }
        let bounds = self.box_bounds.clone();
        let root = column()
            .w_full()
            .max_w(px(920.))
            .gap(px(24.))
            .when_some(strip, |d, strip| d.child(strip))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .pt(px(20.))
                    .gap(px(if wide { 16. } else { 36. }))
                    .when(!wide, |d| d.flex_col())
                    .child(stage)
                    .child(panel),
            )
            .child(
                div()
                    .absolute()
                    .size(px(1.))
                    .overflow_hidden()
                    .id("orbit-status")
                    .role(Role::Status)
                    .aria_label(self.announcement.clone()),
            )
            .child(
                canvas(
                    move |b, w, _| {
                        let before = bounds.replace(b);
                        if b.size != before.size
                            || (before.top() < w.viewport_size().height && before.bottom() > px(0.))
                                != (b.top() < w.viewport_size().height && b.bottom() > px(0.))
                        {
                            w.request_animation_frame();
                        }
                    },
                    |_, _, _, _| (),
                )
                .absolute()
                .inset_0(),
            )
            .relative();
        let root = if let Some(style) = &self.style {
            style(root)
        } else {
            root
        };
        root.id(self.id.clone())
            .track_focus(&self.focus)
            .tab_stop(false)
            .on_hover(cx.listener(|s, hovered, w, cx| {
                s.pointer_inside = *hovered && !w.last_input_was_touch();
                cx.notify();
            }))
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                if e.keystroke.key == "escape" && s.selected.is_some() {
                    s.close(w, cx);
                    w.prevent_default();
                    cx.stop_propagation();
                }
            }))
    }
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use super::{Connection, OrbitConnectRequested, OrbitLogo, OrbitLogos, nearest_turn};
    use crate::foundation::Theme;
    use gpui::{TestAppContext, px, size};
    use std::{cell::RefCell, f32::consts::TAU, rc::Rc};

    #[test]
    fn parking_chooses_the_short_turn_after_crossing_zero() {
        let near_end = TAU - 0.03;
        let target = nearest_turn(near_end, 0.06);
        assert!((target - near_end - 0.09).abs() < 0.001);
        let reverse = nearest_turn(0.02, TAU - 0.06);
        assert!((reverse + 0.06).abs() < 0.001);
    }

    #[test]
    fn an_outer_selection_moves_inner_logos_off_the_connector_even_while_filtered() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (orbit, window) = app.add_window_view(|_, cx| {
            let logos = (0..14)
                .map(|i| {
                    OrbitLogo::new(format!("integration-{i}"), "Integration").with_category("Dev")
                })
                .collect();
            OrbitLogos::new(logos, cx).with_speed(0.)
        });
        window.update(|_, cx| {
            orbit.update(cx, |s, cx| {
                s.filter_by(Some(0), cx);
                s.set_selected(Some("integration-6"), cx);
                for (wide, direction) in [(true, 0.), (false, std::f32::consts::FRAC_PI_2)] {
                    s.wide = wide;
                    s.park(cx, true);
                    let offset = s.offsets[0].value(cx);
                    for slot in s.slots.iter().filter(|slot| slot.ring == 0) {
                        let distance = nearest_turn(direction, slot.angle + offset) - direction;
                        assert!(distance.abs() >= std::f32::consts::PI / 5. - 0.001);
                    }
                }
            });
        });
    }

    #[test]
    fn keyboard_filter_and_connection_tickets_survive_close_and_retry() {
        let mut app = TestAppContext::single();
        app.update(|cx| {
            crate::init(cx);
            Theme::global_mut(cx).reduced_motion = true;
        });
        let (orbit, window) = app.add_window_view(|_, cx| {
            OrbitLogos::new(
                vec![
                    OrbitLogo::new("a", "Alpha").with_category("Dev"),
                    OrbitLogo::new("b", "Beta").with_category("Design"),
                    OrbitLogo::new("c", "Gamma").with_category("Dev"),
                ],
                cx,
            )
            .with_host_connections()
        });
        window.simulate_resize(size(px(390.), px(790.)));
        window.run_until_parked();
        let events = Rc::new(RefCell::new(Vec::new()));
        let sink = events.clone();
        let _subscription = window.update(|w, cx| {
            orbit.update(cx, |s, cx| {
                s.filter_by(Some(0), cx);
                s.logo_focus[0].focus(w, cx);
            });
            cx.subscribe(&orbit, move |_, event: &OrbitConnectRequested, _| {
                sink.borrow_mut().push(event.clone())
            })
        });
        window.update(|w, cx| {
            w.refresh();
            w.draw(cx).clear(cx);
        });
        window.simulate_keystrokes("right");
        window.update(|w, cx| {
            assert_eq!(orbit.read(cx).active, 2);
            w.refresh();
            w.draw(cx).clear(cx);
        });
        window.simulate_keystrokes("enter");
        window.run_until_parked();
        window.update(|_, cx| {
            orbit.update(cx, |s, cx| {
                assert_eq!(s.selected, Some(2));
                s.connect(2, cx);
                s.connect(2, cx);
            });
        });
        let first = events.borrow()[0].ticket;
        assert_eq!(events.borrow().len(), 1);
        window.simulate_keystrokes("escape");
        window.run_until_parked();
        window.update(|w, cx| {
            orbit.update(cx, |s, cx| {
                assert_eq!(s.selected, None);
                assert!(s.logo_focus[2].is_focused(w));
                assert!(s.finish_connect(first, false, cx));
                s.connect(2, cx);
                assert!(!s.finish_connect(first, true, cx));
                assert!(s.finish_connect(first + 1, true, cx));
                assert_eq!(s.connections[2], Connection::Connected);
            });
        });
        assert_eq!(events.borrow().len(), 2);
    }
}
