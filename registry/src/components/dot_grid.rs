//! A pointer-reactive dot field behind interactive GPUI content.
use crate::{foundation::Theme, primitives::direct_touch::Contact};
use gpui::{prelude::*, *};
use std::{cell::RefCell, collections::VecDeque, rc::Rc};
use web_time::Instant;

const RIPPLE_SPEED: f32 = 620.;
const RIPPLE_LIFETIME: f32 = 1.5;
const RIPPLE_LIMIT: usize = 6;
const MAX_DOTS: f32 = 14_400.;
const DOT_EPSILON: f32 = 0.015;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DotGridOptions {
    /// Center-to-center spacing in logical pixels.
    pub gap: f32,
    /// Radius of a resting dot in logical pixels.
    pub dot_size: f32,
    /// Radius of the pointer lens in logical pixels.
    pub radius: f32,
    /// Maximum lens displacement in logical pixels.
    pub strength: f32,
    pub ripple: bool,
    pub fade: bool,
    pub color: Option<Hsla>,
    pub active_color: Option<Hsla>,
}

impl Default for DotGridOptions {
    fn default() -> Self {
        Self {
            gap: 22.,
            dot_size: 1.4,
            radius: 150.,
            strength: 16.,
            ripple: true,
            fade: true,
            color: None,
            active_color: None,
        }
    }
}

impl DotGridOptions {
    fn normalized(self) -> Self {
        let value = |v: f32, min: f32, max: f32, default: f32| {
            if v.is_finite() {
                v.clamp(min, max)
            } else {
                default
            }
        };
        Self {
            gap: value(self.gap, 4., 256., 22.),
            dot_size: value(self.dot_size, 0., 32., 1.4),
            radius: value(self.radius, 0., 1024., 150.),
            strength: value(self.strength, 0., 128., 16.),
            ..self
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Position {
    x: f32,
    y: f32,
}

impl Position {
    fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    fn from_window(p: Point<Pixels>, bounds: Bounds<Pixels>) -> Self {
        Self::new(
            f32::from(p.x - bounds.origin.x),
            f32::from(p.y - bounds.origin.y),
        )
    }

    fn difference(self, origin: Self) -> Self {
        Self::new(self.x - origin.x, self.y - origin.y)
    }

    fn length(self) -> f32 {
        self.x.hypot(self.y)
    }

    fn approach(&mut self, target: Self, t: f32) {
        self.x += (target.x - self.x) * t;
        self.y += (target.y - self.y) * t;
    }
}

#[derive(Default)]
struct Spring {
    value: f32,
    velocity: f32,
}

impl Spring {
    fn step(&mut self, target: f32, coefficients: Coefficients) -> bool {
        let error = self.value - target;
        if error.abs() < DOT_EPSILON && self.velocity.abs() < 0.12 {
            self.value = target;
            self.velocity = 0.;
            return false;
        }
        self.value = target + coefficients.xx * error + coefficients.xv * self.velocity;
        self.velocity = coefficients.vx * error + coefficients.vv * self.velocity;
        true
    }
}

/// A closed-form damped oscillator, sampled at the actual frame interval.
#[derive(Clone, Copy)]
struct Coefficients {
    xx: f32,
    xv: f32,
    vx: f32,
    vv: f32,
}

impl Coefficients {
    fn for_dt(dt: f32) -> Self {
        const OMEGA: f32 = 24.;
        const DAMPING: f32 = 0.58;
        let falloff = OMEGA * DAMPING;
        let oscillation = OMEGA * (1. - DAMPING * DAMPING).sqrt();
        let decay = (-falloff * dt).exp();
        let (sin, cos) = (oscillation * dt).sin_cos();
        let skew = sin * falloff / oscillation;
        Self {
            xx: decay * (cos + skew),
            xv: decay * sin / oscillation,
            vx: -decay * OMEGA * OMEGA * sin / oscillation,
            vv: decay * (cos - skew),
        }
    }
}

struct Dot {
    center: Position,
    edge: f32,
    x: Spring,
    y: Spring,
    heat: Spring,
}

#[derive(Clone, Copy)]
struct Wave {
    center: Position,
    start: f32,
}

#[derive(Default)]
struct Field {
    bounds: Bounds<Pixels>,
    dots: Vec<Dot>,
    gap: f32,
    pointer: Option<Position>,
    lens: Position,
    lens_weight: f32,
    press: Option<Position>,
    touch: bool,
    waves: VecDeque<Wave>,
    time: f32,
    last: Option<Instant>,
    moving: bool,
    reduced: bool,
}

fn smooth(t: f32) -> f32 {
    let t = t.clamp(0., 1.);
    t * t * (3. - 2. * t)
}

fn lens_strength(ratio: f32) -> f32 {
    if !(0. ..1.).contains(&ratio) {
        return 0.;
    }
    if ratio < 1. / 3. {
        smooth(ratio * 3.)
    } else {
        1. - smooth((ratio - 1. / 3.) * 1.5)
    }
}

impl Field {
    fn resize(&mut self, bounds: Bounds<Pixels>, gap: f32) {
        let rebuild = self.bounds.size != bounds.size || self.gap != gap;
        if self.bounds.origin != bounds.origin
            && let Some(pointer) = self.pointer
            && !self.touch
        {
            let window = self.bounds.origin + point(px(pointer.x), px(pointer.y));
            self.pointer(
                bounds
                    .contains(&window)
                    .then(|| Position::from_window(window, bounds)),
            );
        }
        self.bounds = bounds;
        if !rebuild {
            return;
        }
        self.gap = gap;
        self.moving |= self.pointer.is_some() || !self.waves.is_empty();
        let w = f32::from(bounds.size.width);
        let h = f32::from(bounds.size.height);
        self.dots.clear();
        if w <= 0. || h <= 0. {
            return;
        }
        let spacing = gap
            .max((w * h / MAX_DOTS).sqrt())
            .max(w / 240.)
            .max(h / 240.);
        let columns = (w / spacing).floor() as usize + 1;
        let rows = (h / spacing).floor() as usize + 1;
        let left = (w - spacing * (columns - 1) as f32) * 0.5;
        let top = (h - spacing * (rows - 1) as f32) * 0.5;
        self.dots.reserve(rows * columns);
        for row in 0..rows {
            let y = top + row as f32 * spacing;
            let fy = smooth(y.min(h - y) / (h * 0.25).max(1.));
            for column in 0..columns {
                let x = left + column as f32 * spacing;
                let fx = smooth(x.min(w - x) / (w * 0.23).max(1.));
                self.dots.push(Dot {
                    center: Position::new(x, y),
                    edge: 0.38 + fx * fy * 0.62,
                    x: Spring::default(),
                    y: Spring::default(),
                    heat: Spring::default(),
                });
            }
        }
    }

    fn pointer(&mut self, pointer: Option<Position>) -> bool {
        if self.pointer == pointer {
            return false;
        }
        if self.pointer.is_none()
            && self.lens_weight == 0.
            && let Some(position) = pointer
        {
            self.lens = position;
        }
        self.pointer = pointer;
        self.moving = true;
        true
    }

    fn ripple(&mut self, center: Position, enabled: bool) -> bool {
        if !enabled || self.reduced {
            return false;
        }
        if self.waves.len() == RIPPLE_LIMIT {
            self.waves.pop_front();
        }
        self.waves.push_back(Wave {
            center,
            start: self.time,
        });
        self.moving = true;
        true
    }

    fn step(
        &mut self,
        now: Instant,
        options: DotGridOptions,
        reduced: bool,
        visible: bool,
    ) -> bool {
        if self.reduced != reduced {
            self.reduced = reduced;
            self.moving = true;
        }
        if !visible {
            self.last = None;
            return false;
        }
        if !self.moving {
            return false;
        }
        let elapsed = self.last.replace(now).map_or(1. / 60., |prev| {
            now.saturating_duration_since(prev).as_secs_f32()
        });
        let dt = elapsed.min(0.05);
        if reduced {
            self.waves.clear();
            for dot in &mut self.dots {
                dot.x = Spring::default();
                dot.y = Spring::default();
                dot.heat.value = self.pointer.map_or(0., |pointer| {
                    lens_strength(
                        dot.center.difference(pointer).length() / options.radius.max(0.01),
                    )
                });
                dot.heat.velocity = 0.;
            }
            self.last = None;
            self.moving = false;
            return false;
        }
        if !options.ripple {
            self.waves.clear();
        }
        self.time += elapsed;
        while self
            .waves
            .front()
            .is_some_and(|w| self.time - w.start >= RIPPLE_LIFETIME)
        {
            self.waves.pop_front();
        }
        let weight = f32::from(self.pointer.is_some());
        self.lens_weight += (weight - self.lens_weight) * (1. - (-15. * dt).exp());
        if (self.lens_weight - weight).abs() < 0.003 {
            self.lens_weight = weight;
        }
        let trailing = if let Some(pointer) = self.pointer {
            self.lens.approach(pointer, 1. - (-20. * dt).exp());
            if self.lens.difference(pointer).length() < 0.08 {
                self.lens = pointer;
            }
            self.lens != pointer
        } else {
            false
        };
        let spring = Coefficients::for_dt(dt);
        let mut moving = trailing || self.lens_weight != weight || !self.waves.is_empty();
        let wave_at = |wave: &Wave| {
            let age = self.time - wave.start;
            (
                wave.center,
                age * RIPPLE_SPEED,
                (1. - age / RIPPLE_LIFETIME).max(0.).powi(2),
            )
        };
        let mut waves = [(Position::default(), 0., 0.); RIPPLE_LIMIT];
        for (slot, wave) in waves.iter_mut().zip(&self.waves) {
            *slot = wave_at(wave);
        }
        let waves = &waves[..self.waves.len()];
        for dot in &mut self.dots {
            let mut x = 0.;
            let mut y = 0.;
            let mut heat = 0.;
            if self.lens_weight > 0. && options.radius > 0. {
                let v = dot.center.difference(self.lens);
                if v.x.abs() < options.radius && v.y.abs() < options.radius {
                    let d = v.length();
                    if d > 0.01 {
                        heat = lens_strength(d / options.radius) * self.lens_weight;
                        let push = options.strength * heat / d;
                        x += v.x * push;
                        y += v.y * push;
                    }
                }
            }
            for (center, ring, life) in waves {
                let v = dot.center.difference(*center);
                if v.x.abs() > ring + 54. || v.y.abs() > ring + 54. {
                    continue;
                }
                let d = v.length();
                let delta = (d - ring) / 54.;
                if delta.abs() < 1. && d > 0.01 {
                    let envelope = (1. - smooth(delta.abs())) * life;
                    let offset =
                        (delta * std::f32::consts::PI).sin() * envelope * options.strength * 0.95
                            / d;
                    x += v.x * offset;
                    y += v.y * offset;
                    heat += envelope * 0.84;
                }
            }
            moving |= dot.x.step(x, spring);
            moving |= dot.y.step(y, spring);
            moving |= dot.heat.step(heat.min(1.), spring);
        }
        self.moving = moving;
        if self.waves.is_empty() {
            self.time = 0.;
        }
        if !moving {
            self.last = None;
        }
        moving
    }
}

type Refine = Box<dyn Fn(Div) -> Div>;
type Overlay = Box<dyn Fn(&mut Window, &mut App) -> AnyElement>;
type Visibility = Rc<dyn Fn(&App) -> bool>;

/// A decoration with an independent animation layer. Overlay content keeps its GPUI semantics.
pub struct DotGrid {
    layer: Entity<DotLayer>,
    id: ElementId,
    style: Option<Refine>,
    content: Option<Overlay>,
}

impl DotGrid {
    pub fn new(options: DotGridOptions, cx: &mut Context<Self>) -> Self {
        Self {
            layer: cx.new(|_| DotLayer {
                field: Rc::default(),
                options: options.normalized(),
                contact: Contact::default(),
                visibility: None,
            }),
            id: "dot-grid".into(),
            style: None,
            content: None,
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

    /// Create ordinary GPUI content above the decorative layer.
    pub fn with_content<E: IntoElement + 'static>(
        mut self,
        content: impl Fn(&mut Window, &mut App) -> E + 'static,
    ) -> Self {
        self.content = Some(Box::new(move |w, cx| content(w, cx).into_any_element()));
        self
    }

    pub fn with_visibility(
        self,
        visible: impl Fn(&App) -> bool + 'static,
        cx: &mut Context<Self>,
    ) -> Self {
        self.layer
            .update(cx, |layer, _| layer.visibility = Some(Rc::new(visible)));
        self
    }

    pub fn set_options(&mut self, options: DotGridOptions, cx: &mut Context<Self>) {
        self.layer.update(cx, |layer, cx| {
            let options = options.normalized();
            if layer.options != options {
                layer.options = options;
                layer.field.borrow_mut().moving = true;
                cx.notify();
            }
        });
    }
}

impl Render for DotGrid {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let root = div().relative().w_full().overflow_hidden();
        let root = if let Some(style) = &self.style {
            style(root)
        } else {
            root
        };
        root.id(self.id.clone())
            .child(div().absolute().inset_0().child(self.layer.clone()))
            .when_some(self.content.as_ref(), |e, make| {
                e.child(
                    div()
                        .occlude()
                        .min_w(px(0.))
                        .max_w_full()
                        .child(make(w, cx)),
                )
            })
    }
}

struct DotLayer {
    field: Rc<RefCell<Field>>,
    options: DotGridOptions,
    contact: Contact,
    visibility: Option<Visibility>,
}

impl DotLayer {
    fn touch(&mut self, event: &TouchDragEvent, _: &mut Window, cx: &mut Context<Self>) {
        let mut field = self.field.borrow_mut();
        let p = Position::from_window(event.position, field.bounds);
        let inside = field.bounds.contains(&event.position);
        match event.phase {
            TouchPhase::Started => {
                field.touch = true;
                field.press = Some(p);
                field.pointer(Some(p));
            }
            TouchPhase::Moved => {
                field.pointer(inside.then_some(p));
            }
            TouchPhase::Ended | TouchPhase::Cancelled => {
                if event.phase == TouchPhase::Ended
                    && inside
                    && field
                        .press
                        .is_some_and(|start| p.difference(start).length() <= 8.)
                {
                    field.ripple(p, self.options.ripple);
                }
                field.press = None;
                field.touch = false;
                field.pointer(None);
            }
        }
        cx.notify();
    }
}

fn listen_mouse(
    bounds: Bounds<Pixels>,
    options: DotGridOptions,
    view: WeakEntity<DotLayer>,
    w: &mut Window,
) {
    let mouse = view.clone();
    w.on_mouse_event(move |event: &MouseMoveEvent, phase, w, cx| {
        if !phase.capture() || w.last_input_was_touch() {
            return;
        }
        let _ = mouse.update(cx, |view, cx| {
            let mut field = view.field.borrow_mut();
            if !field.touch
                && field.pointer(
                    bounds
                        .contains(&event.position)
                        .then(|| Position::from_window(event.position, bounds)),
                )
            {
                cx.notify();
            }
        });
    });
    let exit = view.clone();
    w.on_mouse_event(move |_: &MouseExitEvent, phase, _, cx| {
        if !phase.capture() {
            return;
        }
        let _ = exit.update(cx, |view, cx| {
            let mut field = view.field.borrow_mut();
            if !field.touch && field.pointer(None) {
                cx.notify();
            }
        });
    });
    let down = view.clone();
    w.on_mouse_event(move |event: &MouseDownEvent, phase, _, cx| {
        if !phase.capture() || event.button != MouseButton::Left {
            return;
        }
        let _ = down.update(cx, |view, _| {
            let mut field = view.field.borrow_mut();
            if !field.touch {
                field.press = bounds
                    .contains(&event.position)
                    .then(|| Position::from_window(event.position, bounds));
            }
        });
    });
    w.on_mouse_event(move |event: &MouseUpEvent, phase, w, cx| {
        if !phase.capture() || event.button != MouseButton::Left {
            return;
        }
        let _ = view.update(cx, |view, cx| {
            let mut field = view.field.borrow_mut();
            if field.touch {
                return;
            }
            let p = Position::from_window(event.position, bounds);
            let tapped = field
                .press
                .take()
                .is_some_and(|start| start.difference(p).length() <= 8.)
                && bounds.contains(&event.position);
            let removed_mouse_lens = w.last_input_was_touch() && field.pointer(None);
            if (tapped && field.ripple(p, options.ripple)) || removed_mouse_lens {
                cx.notify();
            }
        });
    });
}

impl Render for DotLayer {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.field.clone();
        let options = self.options;
        let theme = Theme::global(cx);
        let reduced = theme.reduced_motion;
        let rest = options.color.unwrap_or(theme.tokens.colors.border_strong);
        let active = options.active_color.unwrap_or(theme.tokens.colors.accent);
        let view = cx.weak_entity();
        let visibility = self.visibility.clone();
        div()
            .size_full()
            .relative()
            .child(
                canvas(
                    |_, _, _| {},
                    move |bounds, _, w, cx| {
                        let visible = w.is_visible()
                            && Bounds::new(Point::default(), w.viewport_size()).intersects(&bounds)
                            && visibility.as_ref().is_none_or(|visible| visible(cx));
                        let mut state = state.borrow_mut();
                        state.resize(bounds, options.gap);
                        let moving =
                            state.step(cx.background_executor().now(), options, reduced, visible);
                        for dot in &state.dots {
                            let heat = dot.heat.value.clamp(0., 1.);
                            let growth = if reduced { 0. } else { heat * 0.48 };
                            let radius = px(options.dot_size * (1. + growth));
                            if radius <= px(0.) {
                                continue;
                            }
                            let center = bounds.origin
                                + point(
                                    px(dot.center.x + dot.x.value),
                                    px(dot.center.y + dot.y.value),
                                );
                            let rect = Bounds::new(
                                center - point(radius, radius),
                                size(radius * 2., radius * 2.),
                            );
                            let edge = if options.fade { dot.edge } else { 1. };
                            let opacity = edge * (0.75 + 0.23 * heat);
                            let color = if heat > 0. {
                                rest.blend(active.opacity(heat * 0.76))
                            } else {
                                rest
                            };
                            w.paint_quad(fill(rect, color.opacity(opacity)).corner_radii(radius));
                        }
                        if moving {
                            let next = view.clone();
                            w.on_next_frame(move |_, cx| {
                                let _ = next.update(cx, |_, cx| cx.notify());
                            });
                        }
                        listen_mouse(bounds, options, view, w);
                    },
                )
                .absolute()
                .inset_0()
                .size_full(),
            )
            .child(self.contact.listen(cx, Self::touch))
    }
}

#[cfg(test)]
mod tests {
    use super::{DotGrid, DotGridOptions, Field, Position, RIPPLE_LIMIT, lens_strength};
    use gpui::{
        Bounds, ClickEvent, TestAppContext, TouchEvent, TouchId, TouchPhase, div, point,
        prelude::*, px, size,
    };
    use std::{cell::Cell, rc::Rc, time::Duration};
    use web_time::Instant;

    fn bounds(w: f32, h: f32) -> Bounds<gpui::Pixels> {
        Bounds::new(point(px(0.), px(0.)), size(px(w), px(h)))
    }

    #[test]
    fn densities_center_even_when_the_remaining_space_is_odd() {
        let mut field = Field::default();
        for (gap, columns, rows) in [(14., 48, 31), (22., 31, 20), (32., 21, 14)] {
            field.resize(bounds(669., 420.), gap);
            assert_eq!(field.dots.len(), columns * rows);
            let start = field.dots.first().unwrap().center;
            let end = field.dots.last().unwrap().center;
            assert_eq!(start.x + end.x, 669.);
            assert_eq!(start.y + end.y, 420.);
            assert_eq!(field.dots[1].center.x - start.x, gap);
        }
        field.resize(bounds(272., 380.), 22.);
        assert_eq!(field.dots.len(), 13 * 18);
    }

    #[test]
    fn lens_peaks_at_one_third_and_releases_without_remaining_frame_demand() {
        assert_eq!(lens_strength(1. / 3.), 1.);
        assert_eq!(lens_strength(1.), 0.);
        let mut field = Field::default();
        let options = DotGridOptions::default();
        let now = Instant::now();
        field.resize(bounds(304., 212.), options.gap);
        field.pointer(Some(Position::new(140., 100.)));
        for tick in 0..45 {
            field.step(now + Duration::from_millis(tick * 16), options, false, true);
        }
        assert!(
            field
                .dots
                .iter()
                .any(|dot| dot.x.value > 10. && dot.heat.value > 0.6)
        );
        field.pointer(None);
        for tick in 45..140 {
            field.step(now + Duration::from_millis(tick * 16), options, false, true);
        }
        assert!(
            field
                .dots
                .iter()
                .all(|dot| dot.x.value == 0. && dot.y.value == 0.)
        );
        assert!(!field.moving);
        assert!(field.last.is_none());
    }

    #[test]
    fn ripple_capacity_hidden_pause_and_reduced_motion_are_independent() {
        let mut field = Field::default();
        let options = DotGridOptions::default();
        field.resize(bounds(320., 380.), options.gap);
        let now = Instant::now();
        for i in 0..10 {
            field.ripple(Position::new(10. * i as f32, 80.), true);
        }
        assert_eq!(field.waves.len(), RIPPLE_LIMIT);
        assert_eq!(field.waves.front().unwrap().center.x, 40.);
        assert!(field.step(now, options, false, true));
        let age = field.time;
        assert!(!field.step(now + Duration::from_secs(9), options, false, false));
        assert_eq!(field.time, age);
        assert!(field.step(now + Duration::from_secs(9), options, false, true));
        assert!(field.time < age + 0.018);
        field.pointer(Some(Position::new(140., 110.)));
        assert!(!field.step(now + Duration::from_secs(10), options, true, true));
        assert!(field.waves.is_empty());
        assert!(
            field
                .dots
                .iter()
                .all(|dot| dot.x.value == 0. && dot.y.value == 0.)
        );
        assert!(field.dots.iter().any(|dot| dot.heat.value > 0.4));
        assert!(!field.ripple(Position::new(120., 110.), true));
    }

    #[test]
    fn blank_touch_tracks_contact_and_overlay_still_receives_its_tap() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let taps = Rc::new(Cell::new(0));
        let count = taps.clone();
        let (grid, w) = app.add_window_view(|_, cx| {
            DotGrid::new(DotGridOptions::default(), cx)
                .with_style(|e| {
                    e.h(px(380.))
                        .flex()
                        .justify_center()
                        .items_end()
                        .pb(px(16.))
                })
                .with_content(move |_, _| {
                    let count = count.clone();
                    div()
                        .id("dot-grid-test-button")
                        .debug_selector(|| "dot-grid-test-button".into())
                        .w(px(72.))
                        .h(px(36.))
                        .role(gpui::Role::Button)
                        .on_click(move |_: &ClickEvent, _, _| count.set(count.get() + 1))
                        .child("Action")
                })
        });
        w.simulate_resize(size(px(320.), px(420.)));
        w.update(|w, _| w.refresh());
        w.run_until_parked();
        let target = w.debug_bounds("dot-grid-test-button").unwrap().center();
        let blank = point(px(100.), px(100.));
        for phase in [TouchPhase::Started, TouchPhase::Ended] {
            w.simulate_event(TouchEvent {
                id: TouchId(1),
                phase,
                position: target,
                ..Default::default()
            });
        }
        assert_eq!(taps.get(), 1);
        w.simulate_event(TouchEvent {
            id: TouchId(2),
            phase: TouchPhase::Started,
            position: blank,
            ..Default::default()
        });
        assert!(w.update(|_, cx| grid.read(cx).layer.read(cx).field.borrow().touch));
        let moved = point(px(103.), px(102.));
        w.simulate_event(TouchEvent {
            id: TouchId(2),
            phase: TouchPhase::Moved,
            position: moved,
            ..Default::default()
        });
        assert_eq!(
            w.update(|_, cx| grid.read(cx).layer.read(cx).field.borrow().pointer),
            Some(Position::new(103., 102.))
        );
        w.simulate_event(TouchEvent {
            id: TouchId(2),
            phase: TouchPhase::Ended,
            position: moved,
            ..Default::default()
        });
        w.update(|_, cx| {
            let field = grid.read(cx).layer.read(cx).field.borrow();
            assert!(!field.touch);
            assert!(field.pointer.is_none());
            assert_eq!(field.waves.len(), 2);
        });
    }
}
