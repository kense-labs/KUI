//! A compact status mark with interruptible, curved stroke transitions.
use crate::primitives::family::{Paint, disabled_motion};
use gpui::{prelude::*, *};
use std::{
    cell::{Cell, RefCell},
    f64::consts::TAU,
    rc::Rc,
};
use web_time::Instant;

/// Loading animation geometry.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LoaderVariant {
    #[default]
    Dots,
    Bars,
    Ring,
    Square,
}
/// The host owns the operation and explicitly reports its result.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LoaderStatus {
    #[default]
    Loading,
    Success,
    Error,
}

// Parameters are documented, with their public preview bundle fingerprint, in
// docs/audits/component-review-2026-11-03.md. Keep the drawing on a 24-unit grid.
const MORPH_SECONDS: f64 = 0.46;
const RESULT_SECONDS: f64 = 0.52;
const UPRIGHT_SECONDS: f64 = 0.60;
const STROKE_STAGGER: f64 = 0.035;
const REST_DELTA: f64 = 0.001;
const REST_SPEED: f64 = 0.01;
const FIELD_COUNT: usize = 7;
const CX: usize = 0;
const CY: usize = 1;
const LENGTH: usize = 2;
const ANGLE: usize = 3;
const BEND: usize = 4;
const WIDTH: usize = 5;
const OPACITY: usize = 6;
type Stroke = [f64; FIELD_COUNT];

fn line(a: [f64; 2], b: [f64; 2], width: f64) -> Stroke {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    [
        (a[0] + b[0]) / 2.,
        (a[1] + b[1]) / 2.,
        dx.hypot(dy),
        dy.atan2(dx).to_degrees(),
        0.,
        width,
        1.,
    ]
}
fn strokes(variant: LoaderVariant, status: LoaderStatus, width: f64) -> [Stroke; 4] {
    match status {
        LoaderStatus::Success => [
            line([5., 12.5], [7.6, 15.1], width),
            line([7.6, 15.1], [10.2, 17.7], width),
            line([10.2, 17.7], [14.6, 12.6], width),
            line([14.6, 12.6], [19., 7.5], width),
        ],
        LoaderStatus::Error => [
            line([6.5, 6.5], [12., 12.], width),
            line([12., 12.], [17.5, 17.5], width),
            line([17.5, 6.5], [12., 12.], width),
            line([12., 12.], [6.5, 17.5], width),
        ],
        LoaderStatus::Loading => match variant {
            LoaderVariant::Dots => [
                [5., 12., 0.01, 0., 0., 4., 1.],
                [12., 12., 0.01, 0., 0., 4., 1.],
                [19., 12., 0.01, 0., 0., 4., 1.],
                [12., 12., 0.01, 0., 0., 0., 0.],
            ],
            LoaderVariant::Bars => [4.5, 9.5, 14.5, 19.5].map(|x| line([x, 6.], [x, 18.], 3.)),
            LoaderVariant::Ring => [0_f64, 90., 180., 270.].map(|angle| {
                let direction = (angle - 90.).to_radians();
                let center_radius = 8.5 * 32_f64.to_radians().cos();
                [
                    12. + center_radius * direction.cos(),
                    12. + center_radius * direction.sin(),
                    17. * 32_f64.to_radians().sin(),
                    angle,
                    1. / 8.5,
                    width,
                    1.,
                ]
            }),
            LoaderVariant::Square => [
                line([5., 5.], [19., 5.], width),
                line([19., 5.], [19., 19.], width),
                line([19., 19.], [5., 19.], width),
                line([5., 19.], [5., 5.], width),
            ],
        },
    }
}

/// Closed-form critical spring, including the current velocity on retarget.
#[derive(Clone, Copy, Debug)]
struct Spring {
    from: f64,
    target: f64,
    velocity: f64,
    started: f64,
    omega: f64,
}
impl Spring {
    fn new(value: f64) -> Self {
        Self {
            from: value,
            target: value,
            velocity: 0.,
            started: 0.,
            omega: 1.,
        }
    }
    fn sample(self, time: f64) -> (f64, f64, bool) {
        if time < self.started {
            return (self.from, self.velocity, true);
        }
        let t = time - self.started;
        let delta = self.from - self.target;
        let k = self.velocity + self.omega * delta;
        let decay = (-self.omega * t).exp();
        let offset = (delta + k * t) * decay;
        let speed = (self.velocity - self.omega * k * t) * decay;
        let active = offset.abs() >= REST_DELTA || speed.abs() >= REST_SPEED;
        if active {
            (self.target + offset, speed, true)
        } else {
            (self.target, 0., false)
        }
    }
    fn aim(&mut self, value: f64, time: f64, seconds: f64, delay: f64) {
        let (position, velocity, _) = self.sample(time);
        *self = Self {
            from: position,
            target: value,
            velocity,
            started: time + delay,
            omega: TAU / (1.2 * seconds),
        };
    }
}

#[derive(Clone, Copy)]
struct Fade {
    from: f64,
    target: f64,
    started: f64,
    seconds: f64,
}
struct InkFade {
    from: Rgba,
    target: Rgba,
    progress: Fade,
}
impl InkFade {
    fn sample(&self, time: f64) -> (Rgba, bool) {
        let (amount, moving) = self.progress.sample(time);
        let amount = amount as f32;
        let mix = |a: f32, b: f32| a + (b - a) * amount;
        (
            Rgba {
                r: mix(self.from.r, self.target.r),
                g: mix(self.from.g, self.target.g),
                b: mix(self.from.b, self.target.b),
                a: mix(self.from.a, self.target.a),
            },
            moving,
        )
    }
    fn color(&mut self, target: Rgba, time: f64, seconds: f64) -> (Rgba, bool) {
        if self.target != target {
            self.from = self.sample(time).0;
            self.target = target;
            self.progress = Fade::new(0.);
            self.progress.aim(1., time, seconds);
        }
        self.sample(time)
    }
}
impl Fade {
    fn new(value: f64) -> Self {
        Self {
            from: value,
            target: value,
            started: 0.,
            seconds: 0.,
        }
    }
    fn sample(self, time: f64) -> (f64, bool) {
        if self.seconds == 0. {
            return (self.target, false);
        }
        let progress = ((time - self.started) / self.seconds).clamp(0., 1.);
        (
            self.from + (self.target - self.from) * bezier(progress, [0.22, 1., 0.36, 1.]),
            progress < 1. && self.from != self.target,
        )
    }
    fn aim(&mut self, target: f64, time: f64, seconds: f64) {
        *self = Self {
            from: self.sample(time).0,
            target,
            started: time,
            seconds,
        };
    }
}
fn bezier(progress: f64, [x1, y1, x2, y2]: [f64; 4]) -> f64 {
    if progress <= 0. || progress >= 1. {
        return progress.clamp(0., 1.);
    }
    let cubic = |t: f64, a: f64, b: f64| {
        3. * (1. - t).powi(2) * t * a + 3. * (1. - t) * t * t * b + t * t * t
    };
    let (mut lo, mut hi) = (0., 1.);
    for _ in 0..24 {
        let t = (lo + hi) * 0.5;
        if cubic(t, x1, x2) < progress {
            lo = t;
        } else {
            hi = t;
        }
    }
    cubic((lo + hi) * 0.5, y1, y2)
}
fn smooth(value: f64) -> f64 {
    value * value * (3. - 2. * value)
}
fn square_turn(time: f64) -> f64 {
    let turns = time / 0.9;
    (turns.floor() + smooth((turns.fract() / 0.8).min(1.))) * 90.
}
fn upright(angle: f64) -> f64 {
    360. * ((angle - 1.) / 360.).ceil()
}

struct LoaderMotion {
    fields: [[Spring; FIELD_COUNT]; 4],
    rotation: Spring,
    loop_mix: Fade,
    time: f64,
    loop_time: f64,
    pop_started: Option<f64>,
    variant: LoaderVariant,
    status: LoaderStatus,
    width: f64,
}
struct Drawing {
    strokes: [Stroke; 4],
    rotation: f64,
    scale: f64,
    active: bool,
}
impl LoaderMotion {
    fn snap(&mut self) {
        self.fields = strokes(self.variant, self.status, self.width).map(|s| s.map(Spring::new));
        self.rotation = Spring::new(0.);
        self.loop_mix = Fade::new(f64::from(self.status == LoaderStatus::Loading));
        self.pop_started = None;
    }
    fn new(variant: LoaderVariant, status: LoaderStatus, width: f64) -> Self {
        Self {
            fields: strokes(variant, status, width).map(|s| s.map(Spring::new)),
            rotation: Spring::new(0.),
            loop_mix: Fade::new(f64::from(status == LoaderStatus::Loading)),
            time: 0.,
            loop_time: 0.,
            pop_started: None,
            variant,
            status,
            width,
        }
    }
    fn retarget(&mut self, variant: LoaderVariant, status: LoaderStatus) {
        self.variant = variant;
        self.status = status;
        let result = status != LoaderStatus::Loading;
        for (i, target) in strokes(variant, status, self.width).into_iter().enumerate() {
            for (field, mut value) in target.into_iter().enumerate() {
                if field == ANGLE {
                    let current = self.fields[i][field].sample(self.time).0;
                    let bend = self.fields[i][BEND].sample(self.time).0;
                    let period = if target[BEND] == 0. && bend < 0.004 {
                        180.
                    } else {
                        360.
                    };
                    value += (((current - value) / period) + 0.5).floor() * period;
                }
                self.fields[i][field].aim(
                    value,
                    self.time,
                    if result {
                        RESULT_SECONDS
                    } else {
                        MORPH_SECONDS
                    },
                    if result {
                        STROKE_STAGGER * i as f64
                    } else {
                        0.
                    },
                );
            }
        }
        self.loop_mix.aim(
            f64::from(!result),
            self.time,
            if result { 0.18 } else { 0.24 },
        );
        if result {
            let angle = self.rotation.sample(self.time).0;
            self.rotation
                .aim(upright(angle), self.time, UPRIGHT_SECONDS, 0.);
            self.pop_started = Some(self.time);
        }
    }
    fn advance(&mut self, seconds: f64) {
        self.time += seconds;
        let running = self.status == LoaderStatus::Loading;
        let mix = self.loop_mix.sample(self.time).0;
        if !running && mix < 0.001 {
            return;
        }
        let delta = seconds.min(0.064);
        let previous = self.loop_time;
        self.loop_time += delta * if running { 1. } else { mix };
        if running {
            match self.variant {
                LoaderVariant::Ring => {
                    self.rotation =
                        Spring::new(self.rotation.sample(self.time).0 + 330. * delta * mix);
                }
                LoaderVariant::Square => {
                    self.rotation = Spring::new(
                        self.rotation.sample(self.time).0
                            + (square_turn(self.loop_time) - square_turn(previous)) * mix,
                    );
                }
                _ if self.rotation.sample(self.time).0 % 360. > 0.01
                    && !self.rotation.sample(self.time).2 =>
                {
                    let angle = self.rotation.sample(self.time).0;
                    self.rotation
                        .aim(upright(angle), self.time, UPRIGHT_SECONDS, 0.);
                }
                _ => {}
            }
        }
    }
    fn drawing(&self, reduced: bool) -> Drawing {
        if reduced {
            return Drawing {
                strokes: strokes(self.variant, self.status, self.width),
                rotation: 0.,
                scale: 1.,
                active: false,
            };
        }
        let mut active = self.status == LoaderStatus::Loading;
        let mut drawing = self.fields.map(|stroke| {
            stroke.map(|spring| {
                let (value, _, settling) = spring.sample(self.time);
                active |= settling;
                value
            })
        });
        let (mix, fading) = self.loop_mix.sample(self.time);
        active |= fading;
        if self.status == LoaderStatus::Loading {
            for (i, stroke) in drawing.iter_mut().enumerate() {
                match self.variant {
                    LoaderVariant::Dots if i < 3 => {
                        let wave = (self.loop_time / 0.9 * TAU - 0.8 * i as f64).sin().max(0.);
                        stroke[CY] -= 3.2 * wave * mix;
                        stroke[WIDTH] += 0.6 * wave * mix;
                    }
                    LoaderVariant::Bars => {
                        let wave = 0.5 + 0.5 * (self.loop_time / 0.95 * TAU - 0.7 * i as f64).sin();
                        stroke[LENGTH] -= 7.5 * (1. - wave) * mix;
                    }
                    LoaderVariant::Ring => {
                        let wave = (self.loop_time * TAU / 1.4).sin();
                        stroke[LENGTH] -= 3.2 * (0.5 + 0.5 * wave) * mix;
                    }
                    LoaderVariant::Square => {
                        let wave = (smooth(((self.loop_time / 0.9).fract() / 0.8).min(1.))
                            * std::f64::consts::PI)
                            .sin();
                        stroke[LENGTH] -= 8. * wave * mix;
                    }
                    _ => {}
                }
            }
        }
        let (rotation, _, turning) = self.rotation.sample(self.time);
        active |= turning;
        let scale = self.pop_started.map_or(1., |start| {
            let phase = (self.time - start) / 0.42;
            active |= phase < 1.;
            if phase < 0.35 {
                1. + 0.14 * bezier((phase / 0.35).clamp(0., 1.), [0., 0., 0.58, 1.])
            } else {
                1.14 - 0.14 * bezier(((phase - 0.35) / 0.65).clamp(0., 1.), [0.42, 0., 0.58, 1.])
            }
        });
        Drawing {
            strokes: drawing,
            rotation,
            scale,
            active,
        }
    }
}

/// A fixed-footprint loader that morphs into a host-reported result.
pub struct MorphLoader {
    id: ElementId,
    size: f32,
    tone: bool,
    decorative: bool,
    labels: [SharedString; 3],
    motion: LoaderMotion,
    last_frame: Option<Instant>,
    in_view: bool,
    window_visible: bool,
    host_visible: bool,
    ink: Rc<RefCell<Option<InkFade>>>,
    frame_pending: Rc<Cell<bool>>,
}
fn queue_loader_frame(window: &Window, pending: &Rc<Cell<bool>>, view: &WeakEntity<MorphLoader>) {
    if pending.replace(true) {
        return;
    }
    let pending = pending.clone();
    let view = view.clone();
    window.on_next_frame(move |_, cx| {
        pending.set(false);
        let _ = view.update(cx, |loader, cx| {
            if loader.visible() {
                cx.notify();
            }
        });
    });
}
impl MorphLoader {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            size: 24.,
            tone: true,
            decorative: false,
            labels: ["Loading".into(), "Done".into(), "Failed".into()],
            motion: LoaderMotion::new(LoaderVariant::Dots, LoaderStatus::Loading, 2.5),
            last_frame: None,
            in_view: true,
            window_visible: true,
            host_visible: true,
            ink: Rc::new(RefCell::new(None)),
            frame_pending: Rc::new(Cell::new(false)),
        }
    }
    pub fn with_size(mut self, size: f32) -> Self {
        if size.is_finite() && size > 0. {
            self.size = size;
        }
        self
    }
    /// Width on the 24-unit grid; dots and bars have their own weights.
    pub fn with_stroke_width(mut self, width: f32) -> Self {
        if width.is_finite() && width > 0. {
            self.motion = LoaderMotion::new(self.variant(), self.status(), width as f64);
        }
        self
    }
    pub fn with_tone(mut self, enabled: bool) -> Self {
        self.tone = enabled;
        self
    }
    pub fn decorative(mut self, enabled: bool) -> Self {
        self.decorative = enabled;
        self
    }
    pub fn with_labels(mut self, labels: [impl Into<SharedString>; 3]) -> Self {
        self.labels = labels.map(Into::into);
        self
    }
    pub fn status(&self) -> LoaderStatus {
        self.motion.status
    }
    pub fn variant(&self) -> LoaderVariant {
        self.motion.variant
    }
    pub fn set_status(&mut self, status: LoaderStatus, cx: &mut Context<Self>) {
        if self.status() != status {
            self.tick(cx);
            self.motion.retarget(self.variant(), status);
            cx.notify();
        }
    }
    pub fn set_variant(&mut self, variant: LoaderVariant, cx: &mut Context<Self>) {
        if self.variant() != variant {
            self.tick(cx);
            if self.status() == LoaderStatus::Loading {
                self.motion.retarget(variant, self.status());
            } else {
                // Variant configures the next loading loop; a result has its own geometry.
                self.motion.variant = variant;
            }
            cx.notify();
        }
    }
    /// Pause a mounted preview without accumulating hidden animation time.
    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        if self.host_visible != visible {
            self.tick(cx);
            self.host_visible = visible;
            self.last_frame = None;
            cx.notify();
        }
    }
    fn visible(&self) -> bool {
        self.host_visible && self.in_view && self.window_visible
    }
    fn tick(&mut self, cx: &App) {
        let now = cx.background_executor().now();
        if self.visible()
            && let Some(previous) = self.last_frame
        {
            let seconds = now.saturating_duration_since(previous).as_secs_f64();
            if disabled_motion(cx) {
                self.motion.time += seconds;
            } else {
                self.motion.advance(seconds);
            }
        }
        if disabled_motion(cx) {
            self.motion.snap();
        }
        self.last_frame = if self.visible() { Some(now) } else { None };
    }
}
impl Render for MorphLoader {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.window_visible = window.is_visible();
        self.tick(cx);
        let Drawing {
            strokes,
            rotation,
            scale,
            active,
        } = self.motion.drawing(disabled_motion(cx));
        let frame_view = cx.weak_entity();
        if active && self.visible() {
            queue_loader_frame(window, &self.frame_pending, &frame_view);
        }
        let palette = Paint::get(cx);
        let tone = if !self.tone {
            None
        } else {
            match self.status() {
                LoaderStatus::Loading => None,
                LoaderStatus::Success => Some(palette.success),
                LoaderStatus::Error => Some(palette.danger),
            }
        };
        let ink_state = self.ink.clone();
        let frame_pending = self.frame_pending.clone();
        let time = self.motion.time;
        let visible = self.visible();
        let reduced = disabled_motion(cx);
        let side = self.size;
        let picture = canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let target: Rgba = tone.unwrap_or_else(|| window.text_style().color).into();
                let mut ink_state = ink_state.borrow_mut();
                let ink_state = ink_state.get_or_insert_with(|| InkFade {
                    from: target,
                    target,
                    progress: Fade::new(1.),
                });
                let (ink, changing) =
                    ink_state.color(target, time, if reduced { 0.12 } else { 0.24 });
                if changing && visible {
                    queue_loader_frame(window, &frame_pending, &frame_view);
                }
                let ink: Hsla = ink.into();
                let unit = side / 24.;
                let convert = |x: f64, y: f64| {
                    let (sin, cos) = rotation.to_radians().sin_cos();
                    let (x, y) = ((x - 12.) * scale, (y - 12.) * scale);
                    point(
                        bounds.origin.x + px((12. + x * cos - y * sin) as f32 * unit),
                        bounds.origin.y + px((12. + x * sin + y * cos) as f32 * unit),
                    )
                };
                for stroke in strokes {
                    let color = ink.opacity(stroke[OPACITY].clamp(0., 1.) as f32);
                    let width = stroke[WIDTH].max(0.) as f32 * unit * scale as f32;
                    if width <= 0. || color.a <= 0. {
                        continue;
                    }
                    let half = stroke[LENGTH].max(0.01) / 2.;
                    let (sin, cos) = stroke[ANGLE].to_radians().sin_cos();
                    let from = convert(stroke[CX] - cos * half, stroke[CY] - sin * half);
                    let to = convert(stroke[CX] + cos * half, stroke[CY] + sin * half);
                    let mut path = PathBuilder::stroke(px(width));
                    path.move_to(from);
                    if stroke[BEND].abs() >= 0.004 {
                        let radius = px((1. / stroke[BEND].abs()) as f32 * unit * scale as f32);
                        path.arc_to(point(radius, radius), px(0.), false, stroke[BEND] > 0., to);
                    } else {
                        path.line_to(to);
                    }
                    if let Ok(path) = path.build() {
                        window.paint_path(path, color);
                    }
                    let radius = px(width / 2.);
                    for center in [from, to] {
                        window.paint_quad(
                            fill(
                                Bounds::centered_at(center, size(radius * 2., radius * 2.)),
                                color,
                            )
                            .corner_radii(radius),
                        );
                    }
                }
            },
        )
        .size(px(side));
        let label = self.labels[match self.status() {
            LoaderStatus::Loading => 0,
            LoaderStatus::Success => 1,
            LoaderStatus::Error => 2,
        }]
        .clone();
        let viewport = window.viewport_size();
        let weak = cx.weak_entity();
        div()
            .on_children_prepainted(move |bounds, _, cx| {
                if let Some(bounds) = bounds.first() {
                    let visible = bounds.left() < viewport.width
                        && bounds.top() < viewport.height
                        && bounds.right() > px(0.)
                        && bounds.bottom() > px(0.);
                    let _ = weak.update(cx, |view, cx| {
                        if view.in_view != visible {
                            view.tick(cx);
                            view.in_view = visible;
                            view.last_frame = None;
                            cx.notify();
                        }
                    });
                }
            })
            .id(self.id.clone())
            .size(px(side))
            .flex_none()
            .when(!self.decorative, |view| {
                view.role(Role::Status).aria_label(label)
            })
            .child(picture)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ANGLE, BEND, CX, CY, Fade, InkFade, LENGTH, LoaderMotion, LoaderStatus, LoaderVariant,
        MORPH_SECONDS, RESULT_SECONDS, Spring, Stroke, strokes,
    };
    use gpui::prelude::*;
    #[test]
    fn changing_target_preserves_position_and_velocity() {
        let mut spring = Spring::new(0.);
        spring.aim(18., 0., MORPH_SECONDS, 0.);
        let before = spring.sample(0.17);
        spring.aim(-3., 0.17, RESULT_SECONDS, 0.);
        let after = spring.sample(0.17);
        assert!((before.0 - after.0).abs() < 1e-10);
        assert!((before.1 - after.1).abs() < 1e-10);
        assert!((spring.sample(4.).0 + 3.).abs() < 1e-10);
    }
    #[test]
    fn ring_breathes_as_four_curves_without_changing_its_centers() {
        let mut motion = LoaderMotion::new(LoaderVariant::Ring, LoaderStatus::Loading, 2.5);
        let first = motion.drawing(false);
        motion.advance(0.2);
        let next = motion.drawing(false);
        for (a, b) in first.strokes.into_iter().zip(next.strokes) {
            assert!(a[BEND] > 0.004 && b[BEND] > 0.004);
            assert_eq!((a[CX], a[CY]), (b[CX], b[CY]));
            assert_ne!(a[LENGTH], b[LENGTH]);
        }
        assert!(next.rotation > first.rotation);
    }
    #[test]
    fn result_strokes_remain_connected_and_eventually_stop_scheduling() {
        let mut motion = LoaderMotion::new(LoaderVariant::Ring, LoaderStatus::Loading, 2.5);
        motion.advance(0.13);
        motion.retarget(LoaderVariant::Ring, LoaderStatus::Success);
        assert!(motion.drawing(false).active);
        for _ in 0..240 {
            motion.advance(1. / 60.);
        }
        let final_frame = motion.drawing(false);
        assert!(!final_frame.active);
        assert!((final_frame.rotation % 360.).abs() < 1e-10);
        for pair in final_frame.strokes.windows(2) {
            let endpoint = |s: Stroke, end: f64| {
                [
                    s[CX] + end * s[LENGTH] / 2. * s[ANGLE].to_radians().cos(),
                    s[CY] + end * s[LENGTH] / 2. * s[ANGLE].to_radians().sin(),
                ]
            };
            let a = endpoint(pair[0], 1.);
            let b = endpoint(pair[1], -1.);
            assert!((a[0] - b[0]).abs() < 1e-9 && (a[1] - b[1]).abs() < 1e-9);
        }
    }
    #[test]
    fn reduced_motion_ignores_inflight_geometry_and_rotation() {
        let mut motion = LoaderMotion::new(LoaderVariant::Ring, LoaderStatus::Loading, 2.5);
        motion.advance(0.12);
        motion.retarget(LoaderVariant::Square, LoaderStatus::Error);
        let reduced = motion.drawing(true);
        assert_eq!(
            reduced.strokes,
            strokes(LoaderVariant::Square, LoaderStatus::Error, 2.5)
        );
        assert_eq!(
            (reduced.rotation, reduced.scale, reduced.active),
            (0., 1., false)
        );
        motion.snap();
        motion.advance(1. / 60.);
        assert_eq!(motion.drawing(false).strokes, reduced.strokes);
        assert!(!motion.drawing(false).active);
    }
    #[test]
    fn changing_variant_does_not_restart_the_loop_phase() {
        let mut motion = LoaderMotion::new(LoaderVariant::Ring, LoaderStatus::Loading, 2.5);
        motion.advance(0.12);
        let phase = motion.loop_time;
        motion.retarget(LoaderVariant::Bars, LoaderStatus::Loading);
        assert_eq!(motion.loop_time, phase);
        motion.retarget(LoaderVariant::Dots, LoaderStatus::Loading);
        for _ in 0..240 {
            motion.advance(1. / 60.);
        }
        assert!((motion.drawing(false).rotation % 360.).abs() < 1e-9);
    }
    #[test]
    fn interrupted_tone_transition_starts_from_the_current_visible_color() {
        let black = gpui::rgba(0x000000ff);
        let white = gpui::rgba(0xffffffff);
        let green = gpui::rgba(0x00ff00ff);
        let mut fade = InkFade {
            from: black,
            target: black,
            progress: Fade::new(1.),
        };
        fade.color(white, 0., 0.24);
        let current = fade.sample(0.1).0;
        let interrupted = fade.color(green, 0.1, 0.24).0;
        assert_eq!(current, interrupted);
        let (result, active) = fade.sample(1.);
        assert_eq!(result, green);
        assert!(!active);
    }
    #[test]
    fn painted_result_can_return_to_loading_and_keep_requesting_frames() {
        struct Host(gpui::Entity<super::MorphLoader>);
        impl gpui::Render for Host {
            fn render(
                &mut self,
                _: &mut gpui::Window,
                _: &mut gpui::Context<Self>,
            ) -> impl gpui::IntoElement {
                gpui::div()
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(self.0.clone())
            }
        }
        let mut app = gpui::TestAppContext::single();
        app.update(crate::init);
        let (host, window) =
            app.add_window_view(|_, cx| Host(cx.new(|_| super::MorphLoader::new("loader"))));
        window.simulate_resize(gpui::size(gpui::px(628.), gpui::px(520.)));
        let loader = window.update(|_, cx| host.read(cx).0.clone());
        for state in [LoaderStatus::Error, LoaderStatus::Loading] {
            window.update(|_, cx| loader.update(cx, |loader, cx| loader.set_status(state, cx)));
            for _ in 0..100 {
                window
                    .executor()
                    .advance_clock(std::time::Duration::from_millis(16));
                window.update(|window, cx| {
                    window.simulate_next_frame(cx);
                });
                window.run_until_parked();
            }
            window.update(|_, cx| {
                let loader = loader.read(cx);
                assert!(
                    loader.visible(),
                    "host={} viewport={} window={}",
                    loader.host_visible,
                    loader.in_view,
                    loader.window_visible
                );
                assert_eq!(loader.status(), state);
                let drawing = loader.motion.drawing(false);
                assert_eq!(drawing.active, state == LoaderStatus::Loading);
                if state == LoaderStatus::Loading {
                    assert!(drawing.strokes[0][LENGTH] < 0.1);
                }
            });
        }
    }
}
