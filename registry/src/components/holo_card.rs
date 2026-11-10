//! A two-sided card with perspective, pointer lighting and a reversible flip.
use crate::{
    foundation::surface_radius,
    primitives::{
        direct_touch::Contact,
        family::{Paint, Slide, disabled_motion},
        nav::Fade,
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, f32::consts::PI, rc::Rc};
use web_time::Instant;

const PERSPECTIVE: f32 = 900.;
const ASPECT: f32 = 5. / 7.;

type ElementFace = Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>;
type PaintedFace = Rc<dyn Fn(HoloSurface, &mut Window, &mut App)>;

#[derive(Clone)]
enum FaceContent {
    Element(ElementFace),
    Painted(PaintedFace),
}

/// Content and accessible text for one face.
#[derive(Clone)]
pub struct HoloFace {
    label: SharedString,
    content: FaceContent,
}

impl HoloFace {
    /// Mount ordinary GPUI elements inside the face's projected bounds.
    pub fn new(
        label: impl Into<SharedString>,
        content: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        Self {
            label: label.into(),
            content: FaceContent::Element(Rc::new(content)),
        }
    }

    /// Paint artwork in surface coordinates. The label describes the painted face.
    pub fn painted(
        label: impl Into<SharedString>,
        content: impl Fn(HoloSurface, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            label: label.into(),
            content: FaceContent::Painted(Rc::new(content)),
        }
    }
}

/// Projection used by a painted face. Local coordinates run from zero to one.
#[derive(Clone, Copy, Debug)]
pub struct HoloSurface {
    bounds: Bounds<Pixels>,
    radius: Pixels,
    yaw: (f32, f32),
    pitch: (f32, f32),
    lift: Pixels,
}

impl HoloSurface {
    fn new(bounds: Bounds<Pixels>, radius: Pixels, yaw: f32, pitch: f32, lift: Pixels) -> Self {
        Self {
            bounds,
            radius,
            yaw: yaw.sin_cos(),
            pitch: pitch.sin_cos(),
            lift,
        }
    }

    /// Unrotated card width in logical pixels.
    pub fn width(self) -> Pixels {
        self.bounds.size.width
    }

    /// Map a point on the face to window coordinates using a 900px perspective.
    pub fn project(self, u: f32, v: f32) -> Point<Pixels> {
        let x = f32::from(self.bounds.size.width) * (u - 0.5);
        let y = f32::from(self.bounds.size.height) * (v - 0.5);
        let (sy, cy) = self.yaw;
        let (sx, cx) = self.pitch;
        let z = -x * sy;
        let depth = y * sx + z * cx;
        let perspective = PERSPECTIVE / (PERSPECTIVE - depth).max(1.);
        self.bounds.center()
            + point(
                px(x * cy * perspective),
                px((y * cx - z * sx) * perspective) - self.lift,
            )
    }

    /// Paint a filled polygon in local face coordinates.
    pub fn polygon(
        self,
        vertices: impl IntoIterator<Item = Point<f32>>,
        color: impl Into<Background>,
        w: &mut Window,
    ) {
        let mut vertices = vertices.into_iter();
        if let Some(first) = vertices.next() {
            let mut path = PathBuilder::fill();
            path.move_to(self.project(first.x, first.y));
            for p in vertices {
                path.line_to(self.project(p.x, p.y));
            }
            path.close();
            if let Ok(path) = path.build() {
                w.paint_path(path, color);
            }
        }
    }

    /// Paint the entire rounded face.
    pub fn fill(self, color: impl Into<Background>, w: &mut Window) {
        self.polygon(self.outline(), color, w);
    }

    fn outline(self) -> impl Iterator<Item = Point<f32>> {
        let (rx, ry) = self.radii();
        let corners = [(1. - rx, ry), (1. - rx, 1. - ry), (rx, 1. - ry), (rx, ry)];
        corners
            .into_iter()
            .enumerate()
            .flat_map(move |(i, (x, y))| {
                (0..=8).map(move |step| {
                    let a = (i as f32 - 1. + step as f32 / 8.) * PI / 2.;
                    point(x + a.cos() * rx, y + a.sin() * ry)
                })
            })
    }

    fn focus_ring(self, color: Hsla, w: &mut Window) {
        let mut path = PathBuilder::stroke(px(2.));
        for (i, p) in self.outline().enumerate() {
            let p = self.project(p.x, p.y);
            if i == 0 {
                path.move_to(p);
            } else {
                path.line_to(p);
            }
        }
        path.close();
        if let Ok(path) = path.build() {
            w.paint_path(path, color);
        }
    }

    fn radii(self) -> (f32, f32) {
        (
            (self.radius / self.bounds.size.width.max(px(1.))).min(0.5),
            (self.radius / self.bounds.size.height.max(px(1.))).min(0.5),
        )
    }

    fn projected_bounds(self) -> Bounds<Pixels> {
        let a = self.project(0., 0.);
        let mut low = a;
        let mut high = a;
        for (u, v) in [(1., 0.), (0., 1.), (1., 1.)] {
            let p = self.project(u, v);
            low.x = low.x.min(p.x);
            low.y = low.y.min(p.y);
            high.x = high.x.max(p.x);
            high.y = high.y.max(p.y);
        }
        Bounds::from_corners(low, high)
    }

    fn clip(self, u: f32, v: f32) -> Point<f32> {
        let (rx, ry) = self.radii();
        let u = u.clamp(0., 1.);
        let v = v.clamp(0., 1.);
        if rx == 0. || ry == 0. {
            return point(u, v);
        }
        let cx = u.clamp(rx, 1. - rx);
        let cy = v.clamp(ry, 1. - ry);
        let length = ((u - cx) / rx).hypot((v - cy) / ry).max(1.);
        point(cx + (u - cx) / length, cy + (v - cy) / length)
    }
}

// Analytic underdamped spring. A reversal samples position and velocity from
// the existing curve, so it can change direction without a discontinuity.
#[derive(Clone, Copy, Debug)]
struct Spring {
    from: f32,
    target: f32,
    velocity: f32,
    start: Option<Instant>,
    omega: f32,
    damping: f32,
}

impl Spring {
    fn new(omega: f32, damping: f32) -> Self {
        Self {
            from: 0.,
            target: 0.,
            velocity: 0.,
            start: None,
            omega,
            damping,
        }
    }

    fn reset(&mut self, value: f32) {
        self.from = value;
        self.target = value;
        self.velocity = 0.;
        self.start = None;
    }

    fn sample(self, now: Instant) -> (f32, f32, bool) {
        let Some(start) = self.start else {
            return (self.target, 0., false);
        };
        let t = now.saturating_duration_since(start).as_secs_f32();
        let decay = self.omega * self.damping;
        let frequency = self.omega * (1. - self.damping * self.damping).sqrt();
        let a = self.from - self.target;
        let b = (self.velocity + decay * a) / frequency;
        let (sin, cos) = (t * frequency).sin_cos();
        let envelope = (-decay * t).exp();
        let wave = a * cos + b * sin;
        let delta = envelope * wave;
        let velocity = envelope * (frequency * (-a * sin + b * cos) - decay * wave);
        if (delta.abs() < 0.001 && velocity.abs() < 0.012) || t > 1.4 {
            (self.target, 0., false)
        } else {
            (self.target + delta, velocity, true)
        }
    }

    fn set(&mut self, target: f32, now: Instant) {
        if self.target != target {
            let (position, velocity, _) = self.sample(now);
            self.from = position;
            self.velocity = velocity;
            self.target = target;
            self.start = Some(now);
        }
    }
}

/// Flipped state after an uncontrolled change, or a request in controlled mode.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HoloCardFlipped(pub bool);

/// The first touch can be used by the host to start orientation input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HoloCardOrientationRequested;

/// A 5:7 collectible card with optional reverse face and orientation input.
pub struct HoloCard {
    id: ElementId,
    front: HoloFace,
    back: Option<HoloFace>,
    label: Option<SharedString>,
    width: Pixels,
    style: Option<Box<dyn Fn(Div) -> Div>>,
    max_tilt: f32,
    intensity: f32,
    gyroscope: bool,
    reduced_transparency: bool,
    requested_orientation: bool,
    mouse_over: bool,
    orientation: Option<Point<f32>>,
    controlled: bool,
    requested_direction: Option<f32>,
    flipped: bool,
    focus: FocusHandle,
    bounds: Rc<Cell<Bounds<Pixels>>>,
    contact: Contact,
    finger: Option<(Point<Pixels>, bool)>,
    turn: Spring,
    tilt: [Spring; 2],
    light: [Slide; 2],
    lift: Slide,
    fade: Fade,
}

impl EventEmitter<HoloCardFlipped> for HoloCard {}
impl EventEmitter<HoloCardOrientationRequested> for HoloCard {}

impl HoloCard {
    pub fn new(front: HoloFace, cx: &App) -> Self {
        Self {
            id: ElementId::Name(front.label.clone()),
            front,
            back: None,
            label: None,
            width: px(300.),
            style: None,
            max_tilt: 14.,
            intensity: 1.,
            gyroscope: true,
            reduced_transparency: false,
            requested_orientation: false,
            mouse_over: false,
            orientation: None,
            controlled: false,
            requested_direction: None,
            flipped: false,
            focus: cx.focus_handle().tab_stop(true),
            bounds: Default::default(),
            contact: Default::default(),
            finger: None,
            turn: Spring::new(16., 0.72),
            tilt: [Spring::new(22., 0.76); 2],
            light: [Slide::new(0.); 2],
            lift: Slide::snappy(0.),
            fade: Fade::default(),
        }
    }

    pub fn with_back(mut self, back: HoloFace) -> Self {
        self.back = Some(back);
        self
    }

    pub fn with_id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = id.into();
        self
    }

    pub fn with_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set the preferred width. A smaller parent constrains it.
    pub fn with_width(mut self, width: Pixels) -> Self {
        if f32::from(width).is_finite() && width > px(0.) {
            self.width = width;
        }
        self
    }

    pub fn with_style(mut self, style: impl Fn(Div) -> Div + 'static) -> Self {
        self.style = Some(Box::new(style));
        self
    }

    /// Set the largest tilt per axis in degrees.
    pub fn with_max_tilt(mut self, degrees: f32) -> Self {
        if degrees.is_finite() {
            self.max_tilt = degrees.clamp(0., 45.);
        }
        self
    }

    pub fn with_intensity(mut self, intensity: f32) -> Self {
        if intensity.is_finite() {
            self.intensity = intensity.clamp(0., 1.);
        }
        self
    }

    pub fn with_gyroscope(mut self, enabled: bool) -> Self {
        self.gyroscope = enabled;
        self
    }

    pub fn with_reduced_transparency(mut self, enabled: bool) -> Self {
        self.reduced_transparency = enabled;
        self
    }

    pub fn set_reduced_transparency(&mut self, enabled: bool, cx: &mut Context<Self>) {
        if self.reduced_transparency != enabled {
            self.reduced_transparency = enabled;
            cx.notify();
        }
    }

    pub fn with_default_flipped(mut self, flipped: bool) -> Self {
        if !self.controlled {
            self.initialize_side(flipped);
        }
        self
    }

    /// Enable controlled mode. Apply requested state with `set_flipped`.
    pub fn with_flipped(mut self, flipped: bool) -> Self {
        self.controlled = true;
        self.initialize_side(flipped);
        self
    }

    fn initialize_side(&mut self, flipped: bool) {
        self.flipped = flipped;
        self.turn.reset(if flipped { PI } else { 0. });
        self.fade = if flipped {
            Fade::visible()
        } else {
            Fade::default()
        };
    }

    pub fn is_flipped(&self) -> bool {
        self.back.is_some() && self.flipped
    }

    /// Apply host state without emitting a second flip request.
    pub fn set_flipped(&mut self, flipped: bool, cx: &mut Context<Self>) {
        if self.is_flipped() != flipped {
            let direction = self
                .requested_direction
                .take()
                .unwrap_or(self.tilt[0].target);
            self.change_side(flipped, direction, cx);
        }
    }

    /// Supply normalized host orientation, with each axis in `-1..=1`.
    pub fn set_device_tilt(&mut self, tilt: Option<Point<f32>>, cx: &mut Context<Self>) {
        if !self.gyroscope {
            return;
        }
        self.orientation = tilt
            .filter(|p| p.x.is_finite() && p.y.is_finite())
            .map(|p| point(p.x.clamp(-1., 1.), p.y.clamp(-1., 1.)));
        if self.finger.is_none() && !self.mouse_over {
            self.lean(
                self.orientation.unwrap_or_default(),
                self.orientation.is_some(),
                cx,
            );
        }
    }

    fn change_side(&mut self, flipped: bool, direction: f32, cx: &mut Context<Self>) {
        if self.back.is_none() || self.flipped == flipped {
            return;
        }
        self.flipped = flipped;
        let sign = if direction < 0. { -1. } else { 1. };
        let target = self.turn.target + sign * PI;
        if disabled_motion(cx) {
            self.turn.reset(target);
            self.fade.set(flipped, 0.18, cx);
        } else {
            self.turn.set(target, cx.background_executor().now());
            self.fade = if flipped {
                Fade::visible()
            } else {
                Fade::default()
            };
        }
        cx.notify();
    }

    fn activate(&mut self, position: Point<Pixels>, w: &mut Window, cx: &mut Context<Self>) {
        if self.back.is_none() {
            return;
        }
        let next = !self.flipped;
        self.focus.focus(w, cx);
        let direction = f32::from(position.x - self.bounds.get().center().x);
        if self.controlled {
            self.requested_direction = Some(direction);
        } else {
            self.change_side(next, direction, cx);
        }
        cx.emit(HoloCardFlipped(next));
        cx.stop_propagation();
    }

    fn lean(&mut self, target: Point<f32>, active: bool, cx: &mut Context<Self>) {
        if disabled_motion(cx) {
            return;
        }
        let now = cx.background_executor().now();
        for (i, axis) in [target.x, target.y].into_iter().enumerate() {
            self.tilt[i].set(axis, now);
            self.light[i].duration(axis, 0.44, cx);
        }
        self.lift.set(f32::from(active), cx);
        cx.notify();
    }

    fn track(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) {
        let b = self.bounds.get();
        if b.size.width <= px(0.) || b.size.height <= px(0.) {
            return;
        }
        let offset = position - b.center();
        self.lean(
            point(
                (2. * offset.x / b.size.width).clamp(-1., 1.),
                (2. * offset.y / b.size.height).clamp(-1., 1.),
            ),
            true,
            cx,
        );
    }

    fn release(&mut self, cx: &mut Context<Self>) {
        self.lean(
            self.orientation.unwrap_or_default(),
            self.orientation.is_some(),
            cx,
        );
    }

    fn touch(&mut self, event: &TouchDragEvent, w: &mut Window, cx: &mut Context<Self>) {
        match event.phase {
            TouchPhase::Started => {
                self.finger = Some((event.position, false));
            }
            TouchPhase::Moved => {
                if let Some((start, dragged)) = &mut self.finger {
                    let delta = event.position - *start;
                    *dragged |= f32::from(delta.x).hypot(f32::from(delta.y)) > 5.;
                    if *dragged && self.orientation.is_none() {
                        self.track(event.position, cx);
                    }
                }
            }
            TouchPhase::Ended => {
                let tapped = self.finger.take().is_some_and(|(start, dragged)| {
                    let delta = event.position - start;
                    !dragged && f32::from(delta.x).hypot(f32::from(delta.y)) <= 5.
                });
                if tapped && self.bounds.get().contains(&event.position) {
                    if self.gyroscope && !self.requested_orientation && !disabled_motion(cx) {
                        self.requested_orientation = true;
                        cx.emit(HoloCardOrientationRequested);
                    }
                    self.activate(event.position, w, cx);
                }
                self.release(cx);
            }
            TouchPhase::Cancelled => {
                self.finger = None;
                self.release(cx);
            }
        }
    }
}

fn wash(angle: f32, a: Hsla, b: Hsla) -> Background {
    linear_gradient(angle, linear_color_stop(a, 0.), linear_color_stop(b, 1.))
}

#[derive(Clone, Copy)]
struct Illumination {
    light: [f32; 2],
    tilt: [f32; 2],
    sweep: f32,
    intensity: f32,
    foil: bool,
    glitter: bool,
}

impl Illumination {
    fn paint(self, face: HoloSurface, w: &mut Window) {
        if self.intensity <= 0. {
            return;
        }
        let [x, y] = self.light;
        let shine =
            (0.35 + 0.44 * (x.abs() + y.abs()) + self.sweep * 0.45).min(1.) * self.intensity;
        let cx = (0.5 + x * 0.48 + self.sweep * self.tilt[0] * 0.14).clamp(0., 1.);
        let cy = (0.5 + y * 0.48).clamp(0., 1.);
        if self.foil {
            let hue = (0.78 + x * 0.22 + y * 0.16 + self.sweep * 0.33).rem_euclid(1.);
            face.fill(
                wash(
                    118. + x * 30.,
                    hsla(hue, 0.62, 0.77, 0.18 * shine),
                    hsla((hue + 0.43).fract(), 0.64, 0.74, 0.22 * shine),
                ),
                w,
            );
        }
        if self.glitter {
            let mut flecks = PathBuilder::fill();
            for i in 0..126 {
                let u = ((i * 69 + 17) % 131) as f32 / 131. - self.tilt[0] * 0.02;
                let v = ((i * 41 + 39) % 137) as f32 / 137. - self.tilt[1] * 0.02;
                let r = (u - cx).hypot(v - cy);
                if r < 0.33 && (0.025..0.975).contains(&u) && (0.025..0.975).contains(&v) {
                    let center = face.project(u, v);
                    let radius = px((1. - r / 0.33) * (0.55 + (i % 3) as f32 * 0.22));
                    flecks.move_to(center - point(radius, px(0.)));
                    flecks.line_to(center - point(px(0.), radius));
                    flecks.line_to(center + point(radius, px(0.)));
                    flecks.line_to(center + point(px(0.), radius));
                    flecks.close();
                }
            }
            if let Ok(path) = flecks.build() {
                w.paint_path(path, white().opacity(shine * 0.42));
            }
        }
        for i in (1..=7).rev() {
            let radius = i as f32 * 0.076;
            face.polygon(
                (0..24).map(|j| {
                    let angle = j as f32 * PI / 12.;
                    face.clip(cx + radius * angle.cos(), cy + radius * angle.sin() * 0.78)
                }),
                white().opacity(shine * 0.019),
                w,
            );
        }
    }
}

impl Render for HoloCard {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let now = cx.background_executor().now();
        let reduced = disabled_motion(cx);
        let flipping = self.back.is_some();
        let (angle, _, spinning) = if flipping {
            self.turn.sample(now)
        } else {
            (0., 0., false)
        };
        let lean = self.tilt.map(|s| s.sample(now));
        if !reduced && (spinning || lean.iter().any(|s| s.2)) {
            w.request_animation_frame();
        }
        let tilt = if reduced { [0.; 2] } else { lean.map(|s| s.0) };
        let light = if reduced {
            [0.; 2]
        } else {
            self.light.map(|s| s.get(w, cx))
        };
        let lift = if reduced {
            0.
        } else {
            4. * self.lift.get(w, cx)
        };
        let turn = if reduced { 0. } else { angle };
        let yaw = turn + (tilt[0] * self.max_tilt).to_radians();
        let pitch = (-tilt[1] * self.max_tilt).to_radians();
        let back_visible = self.back.is_some()
            && if reduced {
                self.flipped
            } else {
                yaw.cos() < 0.
            };
        let yaw = if back_visible && !reduced {
            yaw - PI
        } else {
            yaw
        };
        let mix = if !flipping {
            0.
        } else if reduced {
            self.fade.get(w, cx).clamp(0., 1.)
        } else {
            f32::from(back_visible)
        };
        let current = if back_visible {
            self.back.as_ref().unwrap_or(&self.front)
        } else {
            &self.front
        };
        let name = self.label.clone().unwrap_or_else(|| current.label.clone());
        let radius = surface_radius(cx);
        let paint = Paint::get(cx);
        let capture = self.bounds.clone();
        let lighting = Illumination {
            light,
            tilt,
            sweep: if reduced { 0. } else { angle.sin().abs() },
            intensity: self.intensity,
            foil: !self.reduced_transparency,
            glitter: !self.reduced_transparency && !reduced,
        };
        let project = move |b| HoloSurface::new(b, radius, yaw, pitch, px(lift));
        let background = canvas(
            move |b, w, _| {
                if capture.replace(b).size != b.size {
                    w.request_animation_frame();
                }
            },
            move |b, _, w, _| {
                let face = project(b);
                let shadow = face.projected_bounds();
                w.paint_drop_shadows(
                    shadow,
                    radius.into(),
                    &[BoxShadow::new(
                        px(-tilt[0] * 12.),
                        px(13. - tilt[1] * 10. + lift),
                        black().opacity(0.24),
                    )
                    .blur_radius(px(25.))],
                );
                face.fill(paint.panel, w);
            },
        )
        .absolute()
        .inset_0();
        let mut card = div()
            .id(self.id.clone())
            .relative()
            .w_full()
            .aspect_ratio(ASPECT)
            .on_mouse_move(cx.listener(|s, e: &MouseMoveEvent, w, cx| {
                if !w.last_input_was_touch() {
                    s.mouse_over = true;
                    s.track(e.position, cx);
                }
            }))
            .on_hover(cx.listener(|s, over: &bool, _, cx| {
                if !over {
                    s.mouse_over = false;
                    if s.finger.is_none() {
                        s.release(cx);
                    }
                }
            }))
            .child(background);
        if flipping {
            card = card
                .track_focus(&self.focus)
                .role(Role::Button)
                .aria_label(name)
                .aria_toggled(self.flipped.into())
                .when(self.label.is_some(), |card| {
                    card.aria_description(current.label.clone())
                })
                .cursor_pointer()
                .on_click(
                    cx.listener(|s, event: &ClickEvent, w, cx| s.activate(event.position(), w, cx)),
                );
        } else {
            card = card.role(Role::Group).aria_label(name);
        }
        for (face, opacity, present, front) in [
            (&self.front, 1. - mix, !back_visible, true),
            (
                self.back.as_ref().unwrap_or(&self.front),
                mix,
                back_visible,
                false,
            ),
        ] {
            if opacity <= 0. {
                continue;
            }
            match &face.content {
                FaceContent::Painted(draw) => {
                    let draw = draw.clone();
                    // Two opaque painted faces crossfade over the front artwork.
                    let opacity = if front && mix > 0. && mix < 1. {
                        1.
                    } else {
                        opacity
                    };
                    card = card.child(
                        canvas(|_, _, _| (), move |b, _, w, cx| draw(project(b), w, cx))
                            .absolute()
                            .inset_0()
                            .opacity(opacity),
                    );
                }
                FaceContent::Element(render) if present => {
                    let bounds = self.bounds.get();
                    let size = if bounds.size.width > px(0.) {
                        bounds.size
                    } else {
                        let width = self.width.min(w.viewport_size().width);
                        size(width, width / ASPECT)
                    };
                    let frame = project(Bounds::new(Point::default(), size)).projected_bounds();
                    let face_opacity = opacity * (yaw.cos().abs() * 3.).min(1.);
                    card = card.child(
                        div()
                            .absolute()
                            .left(frame.origin.x)
                            .top(frame.origin.y)
                            .w(frame.size.width)
                            .h(frame.size.height)
                            .overflow_hidden()
                            .rounded(radius)
                            .opacity(face_opacity)
                            .child(render(w, cx)),
                    );
                }
                FaceContent::Element(_) => {}
            }
        }
        let focus = self.focus.clone();
        let card = card
            .child(
                canvas(
                    |_, _, _| (),
                    move |b, _, w, _| {
                        let face = project(b);
                        lighting.paint(face, w);
                        if flipping && focus.is_focused(w) && w.last_input_was_keyboard() {
                            face.focus_ring(paint.focus, w);
                        }
                    },
                )
                .absolute()
                .inset_0(),
            )
            .when(flipping || !reduced, |card| {
                card.child(self.contact.listen(cx, Self::touch))
            });
        let shell = div().w(self.width).max_w_full().child(card);
        match &self.style {
            Some(style) => style(shell),
            None => shell,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{HoloCard, HoloFace, HoloSurface, Spring};
    use gpui::{Bounds, KeyUpEvent, Keystroke, TestAppContext, div, point, prelude::*, px, size};
    use std::f32::consts::PI;
    use std::time::Duration;
    use web_time::Instant;

    #[test]
    fn reversing_a_spring_keeps_instantaneous_position_and_velocity() {
        let start = Instant::now();
        let mut spring = Spring::new(16., 0.72);
        spring.set(PI, start);
        let reverse = start + Duration::from_millis(90);
        let (position, velocity, _) = spring.sample(reverse);
        assert!(position > 0. && position < PI && velocity > 0.);
        spring.set(0., reverse);
        let (next, speed, _) = spring.sample(reverse);
        assert!((next - position).abs() < 0.0001);
        assert!((speed - velocity).abs() < 0.0001);
        assert_eq!(spring.sample(reverse + Duration::from_secs(2)).0, 0.);
    }

    #[test]
    fn perspective_keeps_the_unrotated_frame_and_compresses_the_edge() {
        let b = Bounds::new(point(px(10.), px(20.)), size(px(290.), px(406.)));
        let front = HoloSurface::new(b, px(8.), 0., 0., px(0.));
        assert_eq!(front.project(0., 0.), b.origin);
        assert_eq!(front.project(1., 1.), b.bottom_right());
        let edge = HoloSurface::new(b, px(8.), PI / 2., 0., px(0.));
        assert!(edge.projected_bounds().size.width < px(0.01));
    }

    #[test]
    fn keyboard_flips_uncontrolled_and_controlled_waits_for_acknowledgment() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let face = |label: &'static str| {
            HoloFace::new(label, move |_, _| div().child(label).into_any_element())
        };
        let (card, window) =
            app.add_window_view(|_, cx| HoloCard::new(face("Front"), cx).with_back(face("Back")));
        window.update(|w, cx| card.read(cx).focus.clone().focus(w, cx));
        window.run_until_parked();
        window.simulate_keystrokes("enter");
        window.simulate_event(KeyUpEvent {
            keystroke: Keystroke::parse("enter").expect("valid key"),
        });
        assert!(window.update(|_, cx| card.read(cx).is_flipped()));
        window.update(|_, cx| {
            card.update(cx, |card, _| card.controlled = true);
        });
        window.simulate_keystrokes("space");
        window.simulate_event(KeyUpEvent {
            keystroke: Keystroke::parse("space").expect("valid key"),
        });
        assert!(window.update(|_, cx| card.read(cx).is_flipped()));
        window.update(|_, cx| {
            card.update(cx, |card, cx| card.set_flipped(false, cx));
        });
        assert!(!window.update(|_, cx| card.read(cx).is_flipped()));
    }
}
