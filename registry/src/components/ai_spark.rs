//! An optically sized activity mark with interruptible state transitions.
use crate::{
    components::text_shimmer::TextShimmer,
    foundation::Theme,
    primitives::{
        color_motion::{ColorFade, mix},
        family::{Slide, disabled_motion, standard_ease},
    },
};
use gpui::{prelude::*, *};
use std::{f64::consts::TAU, time::Duration};
use web_time::Instant;

/// The host-reported activity. `Writing` is the generation phase.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SparkStatus {
    #[default]
    Idle,
    Thinking,
    Writing,
    Done,
    Error,
}
impl SparkStatus {
    fn index(self) -> usize {
        self as usize
    }
    fn busy(self) -> bool {
        matches!(self, Self::Thinking | Self::Writing)
    }
}
/// An isolated mark, a label with a mark, or an avatar surface.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SparkVariant {
    #[default]
    Mark,
    Pill,
    Avatar,
}
/// Semantic ink; an explicit color takes precedence, except in the error state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SparkTone {
    #[default]
    Accent,
    Foreground,
    Muted,
    Inherit,
}
#[derive(Clone, Copy)]
enum MarkSize {
    Px(f32),
    Em(f32),
    Rem(f32),
}

const PHASE: [f64; 8] = [0., 0.021, -0.014, 0.017, -0.019, 0.012, 0.008, -0.016];
type Pose = [f32; 35];
fn fraction(x: f64) -> f64 {
    x - x.floor()
}
fn breath(x: f64) -> f64 {
    0.5 - 0.5 * (TAU * x).cos()
}
fn pulse(x: f64, start: f64, width: f64) -> f64 {
    if x <= start || x >= start + width {
        0.
    } else {
        (std::f64::consts::PI * (x - start) / width).sin().powi(2)
    }
}
fn heartbeat(x: f64) -> f64 {
    pulse(x, 0., 0.2) + 0.62 * pulse(x, 0.2, 0.22)
}
fn cubic(x: f64) -> f64 {
    if x < 0.5 {
        4. * x * x * x
    } else {
        1. - (-2. * x + 2.).powi(3) / 2.
    }
}
fn label_exit_ease(x: f32) -> f32 {
    let (mut lo, mut hi) = (0_f32, 1_f32);
    for _ in 0..20 {
        let t = (lo + hi) * 0.5;
        let at = 3. * 0.58 * (1. - t) * t * t + t * t * t;
        if at < x {
            lo = t;
        } else {
            hi = t;
        }
    }
    let t = (lo + hi) * 0.5;
    3. * (1. - t) * t * t + t * t * t
}
fn pose(source: usize, t: f64) -> Pose {
    let mut out = [0.; 35];
    for (i, phase) in PHASE.iter().enumerate() {
        let even = i % 2 == 0;
        let row = match source {
            0 | 1 => {
                let a = if source == 0 && !even {
                    pulse(fraction(t / 6.4 - (i as f64 - 1.) / 8. + phase), 0., 0.32)
                } else {
                    0.
                };
                [1. + 0.3 * a, 1. + 0.08 * a, a, 1.]
            }
            2 => {
                let a = breath(t / 1.9 - i as f64 / 8. + phase).powf(1.35);
                if even {
                    [0.7 + 0.3 * a, 0.94 + 0.08 * a, -1. + 2. * a, 1.]
                } else {
                    [
                        0.42 + 0.78 * a,
                        0.88 + 0.16 * a,
                        -1. + 3. * a,
                        0.5 + 0.5 * a,
                    ]
                }
            }
            3 => {
                let a = heartbeat(fraction(
                    t / 1.04 - if even { 0. } else { 0.17 } - 0.045 * (i >> 1) as f64 + 0.3 * phase,
                ));
                if even {
                    [0.7 + 0.32 * a, 0.96 + 0.08 * a, 0.8 * a, 1.]
                } else {
                    [0.55 + 0.65 * a, 0.92 + 0.12 * a, 2.5 * a, 0.6 + 0.4 * a]
                }
            }
            4 => [1., 1., 0., 1.],
            _ => {
                if even {
                    [0.56, 0.96, -1., 1.]
                } else {
                    [0.34, 0.9, -2., 0.45]
                }
            }
        };
        for j in 0..4 {
            out[4 * i + j] = row[j] as f32;
        }
    }
    let tail = match source {
        0 => [1. + 0.018 * breath(t / 6.4), 1. + 0.1 * breath(t / 3.2), 0.],
        1 | 4 => [1., 1., 0.],
        2 => [0.965 + 0.03 * breath(t / 4.), 0.7 + 0.22 * breath(t), 0.],
        3 => [
            0.975 + 0.035 * heartbeat(fraction(t / 1.04)),
            0.9 + 0.5 * heartbeat(fraction(t / 1.04 - 0.03)),
            1.,
        ],
        _ => [0.96, 0.8, 0.],
    };
    for i in 0..3 {
        out[32 + i] = tail[i] as f32;
    }
    out
}
fn source(status: SparkStatus, twinkle: bool) -> usize {
    match status {
        SparkStatus::Idle => usize::from(!twinkle),
        _ => status.index() + 1,
    }
}
#[derive(Clone, Copy, Default)]
struct Glide {
    start: f64,
    from: f64,
    v0: f64,
    distance: f64,
    duration: f64,
}
struct Motion {
    status: SparkStatus,
    twinkle: bool,
    from: [f64; 6],
    to: usize,
    mix_start: f64,
    mix_duration: f64,
    time: f64,
    clock: f64,
    angle: f64,
    velocity: f64,
    target: Option<f64>,
    glide: Glide,
    impulses: Vec<(SparkStatus, f64)>,
}
impl Motion {
    fn new(status: SparkStatus, twinkle: bool) -> Self {
        let to = source(status, twinkle);
        let mut from = [0.; 6];
        from[to] = 1.;
        Self {
            status,
            twinkle,
            from,
            to,
            mix_start: 0.,
            mix_duration: 0.,
            time: 0.,
            clock: 0.,
            angle: 0.,
            velocity: 0.,
            target: None,
            glide: Glide::default(),
            impulses: Vec::new(),
        }
    }
    fn progress(&self) -> f64 {
        if self.mix_duration <= 0. {
            1.
        } else {
            ((self.clock - self.mix_start) / self.mix_duration).clamp(0., 1.)
        }
    }
    fn weights(&self) -> [f64; 6] {
        let u = cubic(self.progress());
        let mut weights = self.from.map(|w| w * (1. - u));
        weights[self.to] += u;
        weights
    }
    fn retarget(&mut self, status: SparkStatus, twinkle: bool, reduced: bool) {
        if self.status == status && self.twinkle == twinkle {
            return;
        }
        self.from = self.weights();
        self.to = source(status, twinkle);
        self.mix_start = self.clock;
        self.mix_duration = if reduced {
            0.2
        } else {
            match status {
                SparkStatus::Writing => 0.42,
                SparkStatus::Done => 0.5,
                _ => 0.56,
            }
        };
        if self.status != status {
            self.target = None;
            if !reduced && matches!(status, SparkStatus::Done | SparkStatus::Error) {
                self.impulses.push((status, self.clock));
            }
        }
        self.status = status;
        self.twinkle = twinkle;
    }
    fn advance(&mut self, dt: f64, speed: f64, reduced: bool) {
        let dt = dt.clamp(0., 1. / 30.);
        self.clock += dt;
        if !reduced {
            self.time += dt * speed;
        }
        self.impulses.retain(|(_, start)| self.clock - start < 0.7);
        if reduced {
            self.velocity = 0.;
            self.angle = 90. * (self.angle / 90. + 0.5).floor();
            return;
        }
        if self.status.busy() {
            self.target = None;
            let spin = if self.status == SparkStatus::Thinking {
                26.
            } else {
                46.
            };
            self.velocity += (spin * speed - self.velocity) * (1. - (-3. * dt).exp());
            self.angle += self.velocity * dt;
            if self.angle >= 360. {
                self.angle -= 360.;
            }
        } else {
            if self.target.is_none() {
                let v0 = self.velocity.max(0.);
                let target = 90. * ((self.angle + 0.2 * v0) / 90. - 1e-6).ceil();
                let distance = target - self.angle;
                self.target = Some(target);
                self.glide = Glide {
                    start: self.clock,
                    from: self.angle,
                    v0,
                    distance,
                    duration: (if v0 > 0.5 { 3. * distance / v0 } else { 1.2 }).clamp(0.6, 1.8),
                };
            }
            let g = self.glide;
            let t = self.clock - g.start;
            if g.distance < 0.01 || t >= g.duration {
                self.angle = self.target.unwrap_or(self.angle);
                self.velocity = 0.;
            } else {
                let a = (3. * g.distance - 2. * g.v0 * g.duration) / g.duration.powi(2);
                let b = (g.v0 * g.duration - 2. * g.distance) / g.duration.powi(3);
                self.angle = g.from + g.v0 * t + a * t * t + b * t * t * t;
                self.velocity = g.v0 + 2. * a * t + 3. * b * t * t;
            }
        }
        if !self.active(false) {
            self.angle = self.angle.rem_euclid(360.);
            self.target = Some(self.angle);
        }
    }
    fn active(&self, reduced: bool) -> bool {
        self.progress() < 1.
            || !self.impulses.is_empty()
            || (!reduced
                && (self.status.busy()
                    || self.status == SparkStatus::Idle && self.twinkle
                    || self.target.is_none_or(|v| (v - self.angle).abs() > 1e-8)))
    }
    fn drawing(&self, reduced: bool) -> (Pose, f64) {
        let mut out = [0.; 35];
        let t = if reduced { 0. } else { self.time };
        for (i, weight) in self.weights().into_iter().enumerate() {
            if weight == 0. {
                continue;
            }
            let p = pose(i, t);
            for j in 0..35 {
                out[j] += (p[j] as f64 * weight) as f32;
            }
        }
        let mut shake = 0.;
        if !reduced {
            for (kind, start) in &self.impulses {
                let t = self.clock - start;
                if *kind == SparkStatus::Error {
                    if (0. ..0.46).contains(&t) {
                        shake += 3.2 * (TAU * t / 0.15).sin() * (1. - t / 0.46).powi(2);
                    }
                    continue;
                }
                let g = if !(0. ..0.68).contains(&t) {
                    0.
                } else if t < 0.17 {
                    1. - (1. - t / 0.17).powi(3)
                } else {
                    1. - cubic((t - 0.17) / 0.51)
                };
                for i in 0..8 {
                    out[4 * i] *= (1. - if i % 2 == 0 { 0.12 * g } else { 0.2 * g }) as f32;
                    out[4 * i + 2] -= if i % 2 == 0 { 2.4 * g } else { 3. * g } as f32;
                }
                out[32] *= (1. - 0.045 * g) as f32;
                out[33] *= (1. + 0.35 * g) as f32;
            }
        }
        (out, shake)
    }
}
#[derive(Clone, Copy)]
struct Ray {
    inner: f64,
    outer: f64,
    base: f64,
    tip: f64,
    dot: bool,
}
#[derive(Clone, Copy)]
struct Geometry {
    primary: Ray,
    diagonal: Ray,
    core: f64,
    tone: f64,
    particles: bool,
}
impl Geometry {
    fn new(side: f64) -> Self {
        let h = ((side.max(1.) / 12.).ln() / 8_f64.ln()).clamp(0., 1.);
        let lerp = |a, b| a + (b - a) * h;
        Self {
            primary: Ray {
                inner: lerp(11., 13.5),
                outer: lerp(41.5, 44.),
                base: lerp(8.4, 5.6),
                tip: lerp(3.9, 1.8),
                dot: false,
            },
            diagonal: if side < 18. {
                Ray {
                    inner: 27.,
                    outer: 27.,
                    base: 5.6 + (5. - 5.6) * ((side - 12.) / 6.).clamp(0., 1.),
                    tip: 0.,
                    dot: true,
                }
            } else {
                Ray {
                    inner: lerp(23.5, 22.),
                    outer: lerp(32., 36.),
                    base: lerp(4.9, 3.7),
                    tip: lerp(2.3, 1.4),
                    dot: false,
                }
            },
            core: if side >= 28. { lerp(2.6, 3.4) } else { 0. },
            tone: lerp(0.66, 0.5),
            particles: side >= 40.,
        }
    }
    fn ray(self, index: usize, pose: &Pose) -> Ray {
        let base = if index.is_multiple_of(2) {
            self.primary
        } else {
            self.diagonal
        };
        let length = pose[4 * index].max(0.) as f64;
        let width = pose[4 * index + 1] as f64;
        let inner = base.inner + pose[4 * index + 2] as f64;
        Ray {
            inner,
            outer: inner + (base.outer - base.inner) * length,
            base: base.base
                * width
                * if base.dot {
                    0.72 + 0.28 * length.min(1.3)
                } else {
                    1.
                },
            tip: base.tip * width,
            dot: base.dot,
        }
    }
}
fn rounded(x: f64, places: i32) -> f64 {
    let d = 10_f64.powi(places);
    (x * d + 0.5).floor() / d
}
fn paint_circle(window: &mut Window, center: Point<Pixels>, radius: f32, color: Hsla) {
    if radius > 0. {
        let r = px(radius);
        window.paint_quad(
            fill(Bounds::centered_at(center, size(r * 2., r * 2.)), color).corner_radii(r),
        );
    }
}
fn paint_ray(
    window: &mut Window,
    ray: Ray,
    angle: f64,
    center: Point<Pixels>,
    unit: f64,
    color: Hsla,
) {
    let (sin, cos) = angle.to_radians().sin_cos();
    let point_at = |x: f64, y: f64| {
        center
            + point(
                px(((x * cos - y * sin) * unit) as f32),
                px(((x * sin + y * cos) * unit) as f32),
            )
    };
    let distance = ray.outer - ray.inner;
    let a = rounded(ray.base.max(0.), 2);
    let b = rounded(ray.tip.max(0.), 2);
    if ray.dot || distance <= ray.base - ray.tip + 0.001 {
        paint_circle(
            window,
            point_at(0., rounded(-ray.inner, 2)),
            a as f32 * unit as f32,
            color,
        );
        return;
    }
    let s = (ray.base - ray.tip) / distance;
    let c = (1. - s * s).max(0.).sqrt();
    let x = rounded(ray.base * c, 2);
    let y = rounded(-ray.inner - ray.base * s, 2);
    let tx = rounded(ray.tip * c, 2);
    let ty = rounded(-ray.outer - ray.tip * s, 2);
    let mut path = PathBuilder::fill();
    path.move_to(point_at(x, y));
    path.line_to(point_at(tx, ty));
    let r = px((b * unit) as f32);
    path.arc_to(
        point(r, r),
        px(angle as f32),
        false,
        false,
        point_at(-tx, ty),
    );
    path.line_to(point_at(-x, y));
    let r = px((a * unit) as f32);
    path.arc_to(point(r, r), px(angle as f32), true, false, point_at(x, y));
    path.close();
    if let Ok(path) = path.build() {
        window.paint_path(path, color);
    }
}

struct LabelLayer {
    text: SharedString,
    view: Entity<TextShimmer>,
    opacity: f32,
    from: f32,
    at: Option<Instant>,
    exiting: bool,
}
impl LabelLayer {
    fn opacity(&self, now: Instant, reduced: bool) -> f32 {
        let Some(at) = self.at else {
            return self.opacity;
        };
        let t = (now.saturating_duration_since(at).as_secs_f32()
            / if self.exiting || reduced { 0.1 } else { 0.16 })
        .clamp(0., 1.);
        let u = if self.exiting {
            label_exit_ease(t)
        } else {
            standard_ease(t)
        };
        self.from + (if self.exiting { 0. } else { 1. } - self.from) * u
    }
}

/// An eight-ray activity mark. Without a host status it cycles through four phases.
pub struct AiSpark {
    id: ElementId,
    motion: Motion,
    size: MarkSize,
    custom_size: bool,
    variant: SparkVariant,
    tone: SparkTone,
    color: Option<Hsla>,
    speed: f64,
    paused: bool,
    visible: bool,
    in_view: bool,
    window_visible: bool,
    announce: bool,
    decorative: bool,
    aria_label: Option<SharedString>,
    labels: [SharedString; 5],
    aria_labels: [SharedString; 5],
    label: Option<SharedString>,
    layers: Vec<LabelLayer>,
    label_width: Slide,
    sized: bool,
    ink: Option<ColorFade>,
    background: Option<ColorFade>,
    frame_pending: bool,
    last_frame: Option<Instant>,
    auto_play: bool,
    auto_index: usize,
    auto_task: Option<Task<()>>,
}
impl AiSpark {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            motion: Motion::new(SparkStatus::Idle, true),
            size: MarkSize::Px(24.),
            custom_size: false,
            variant: SparkVariant::Mark,
            tone: SparkTone::Accent,
            color: None,
            speed: 1.,
            paused: false,
            visible: true,
            in_view: true,
            window_visible: true,
            announce: true,
            decorative: false,
            aria_label: None,
            labels: ["Ready", "Thinking", "Writing", "Done", "Couldn't finish"].map(Into::into),
            aria_labels: [
                "AI ready",
                "AI is thinking",
                "AI is writing",
                "AI finished",
                "AI couldn't finish",
            ]
            .map(Into::into),
            label: None,
            layers: Vec::new(),
            label_width: Slide::new(0.),
            sized: false,
            ink: None,
            background: None,
            frame_pending: false,
            last_frame: None,
            auto_play: true,
            auto_index: 0,
            auto_task: None,
        }
    }
    pub fn with_size(mut self, size: f32) -> Self {
        if size.is_finite() && size > 0. {
            self.size = MarkSize::Px(size);
            self.custom_size = true;
        }
        self
    }
    /// Relative to the inherited font size; numeric avatar sizing is rounded separately.
    pub fn with_em_size(mut self, size: f32) -> Self {
        if size.is_finite() && size > 0. {
            self.size = MarkSize::Em(size);
            self.custom_size = true;
        }
        self
    }
    /// Relative to the window's rem size.
    pub fn with_rem_size(mut self, size: f32) -> Self {
        if size.is_finite() && size > 0. {
            self.size = MarkSize::Rem(size);
            self.custom_size = true;
        }
        self
    }
    pub fn with_variant(mut self, variant: SparkVariant) -> Self {
        self.variant = variant;
        if !self.custom_size {
            self.size = MarkSize::Px(match variant {
                SparkVariant::Mark => 24.,
                SparkVariant::Pill => 16.,
                SparkVariant::Avatar => 32.,
            });
        }
        self
    }
    pub fn with_status(mut self, status: SparkStatus) -> Self {
        self.motion = Motion::new(status, self.motion.twinkle);
        self.auto_play = false;
        self
    }
    pub fn with_tone(mut self, tone: SparkTone) -> Self {
        self.tone = tone;
        self
    }
    pub fn with_color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }
    pub fn with_speed(mut self, speed: f64) -> Self {
        if speed.is_finite() {
            self.speed = speed;
        }
        self
    }
    pub fn with_twinkle(mut self, twinkle: bool) -> Self {
        self.motion = Motion::new(self.status(), twinkle);
        self
    }
    pub fn with_paused(mut self, paused: bool) -> Self {
        self.paused = paused;
        self
    }
    pub fn with_announce(mut self, announce: bool) -> Self {
        self.announce = announce;
        self
    }
    pub fn with_aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }
    pub fn with_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }
    pub fn with_labels(mut self, labels: [impl Into<SharedString>; 5]) -> Self {
        self.labels = labels.map(Into::into);
        self
    }
    /// Localize nonvisual activity descriptions independently from visible pill labels.
    pub fn with_aria_labels(mut self, labels: [impl Into<SharedString>; 5]) -> Self {
        self.aria_labels = labels.map(Into::into);
        self
    }
    pub fn decorative(mut self, value: bool) -> Self {
        self.decorative = value;
        self
    }
    pub fn status(&self) -> SparkStatus {
        self.motion.status
    }
    pub fn is_auto_playing(&self) -> bool {
        self.auto_play
    }
    pub fn set_status(&mut self, status: SparkStatus, cx: &mut Context<Self>) {
        self.auto_play = false;
        self.auto_task = None;
        self.change_status(status, cx);
    }
    fn change_status(&mut self, status: SparkStatus, cx: &mut Context<Self>) {
        self.motion
            .retarget(status, self.motion.twinkle, disabled_motion(cx));
        cx.notify();
    }
    /// Resume the built-in cycle; host-controlled state is disabled while it runs.
    pub fn set_auto_play(&mut self, enabled: bool, cx: &mut Context<Self>) {
        if self.auto_play != enabled {
            self.auto_play = enabled;
            self.auto_task = None;
            if enabled {
                self.change_status(
                    [
                        SparkStatus::Idle,
                        SparkStatus::Thinking,
                        SparkStatus::Writing,
                        SparkStatus::Done,
                    ][self.auto_index],
                    cx,
                );
            }
            cx.notify();
        }
    }
    pub fn set_paused(&mut self, paused: bool, cx: &mut Context<Self>) {
        if self.paused != paused {
            self.paused = paused;
            self.last_frame = None;
            self.auto_task = None;
            cx.notify();
        }
    }
    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        if self.visible != visible {
            self.visible = visible;
            self.last_frame = None;
            for layer in &self.layers {
                layer.view.update(cx, |v, cx| v.set_visible(visible, cx));
            }
            cx.notify();
        }
    }
    pub fn set_twinkle(&mut self, twinkle: bool, cx: &mut Context<Self>) {
        self.motion
            .retarget(self.status(), twinkle, disabled_motion(cx));
        cx.notify();
    }
    pub fn set_speed(&mut self, speed: f64, cx: &mut Context<Self>) {
        if speed.is_finite() && self.speed != speed {
            self.speed = speed;
            cx.notify();
        }
    }
    fn visible(&self) -> bool {
        self.visible && self.in_view && self.window_visible
    }
    fn queue(&mut self, w: &Window, cx: &Context<Self>) {
        if self.frame_pending || !self.visible() {
            return;
        }
        self.frame_pending = true;
        let weak = cx.weak_entity();
        w.on_next_frame(move |_, cx| {
            let _ = weak.update(cx, |v, cx| {
                v.frame_pending = false;
                if v.visible() {
                    if !v.paused && v.motion.active(disabled_motion(cx)) {
                        let now = cx.background_executor().now();
                        let dt = v.last_frame.replace(now).map_or(1. / 60., |last| {
                            now.saturating_duration_since(last).as_secs_f64()
                        });
                        v.motion.advance(dt, v.speed, disabled_motion(cx));
                    } else {
                        v.last_frame = None;
                    }
                    cx.notify();
                } else {
                    v.last_frame = None;
                }
            });
        });
    }
    fn cycle(&mut self, cx: &mut Context<Self>) {
        if self.auto_play && !self.paused && self.auto_task.is_none() {
            let delay = [1600, 2800, 3600, 2000][self.auto_index];
            self.auto_task = Some(cx.spawn(async move |weak, cx| {
                cx.background_executor()
                    .timer(Duration::from_millis(delay))
                    .await;
                let _ = weak.update(cx, |v, cx| {
                    v.auto_task = None;
                    v.auto_index = (v.auto_index + 1) % 4;
                    v.change_status(
                        [
                            SparkStatus::Idle,
                            SparkStatus::Thinking,
                            SparkStatus::Writing,
                            SparkStatus::Done,
                        ][v.auto_index],
                        cx,
                    );
                });
            }));
        }
    }
    fn sync_label(&mut self, text: SharedString, cx: &mut Context<Self>) {
        let now = cx.background_executor().now();
        let reduced = disabled_motion(cx);
        self.layers
            .retain(|l| !l.exiting || l.opacity(now, reduced) > 0.0001);
        if self.layers.iter().any(|l| !l.exiting && l.text == text) {
            for l in &self.layers {
                if !l.exiting {
                    l.view
                        .update(cx, |v, cx| v.set_active(self.status().busy(), cx));
                }
            }
            return;
        }
        let initial = self.layers.is_empty();
        for l in &mut self.layers {
            l.from = l.opacity(now, reduced);
            l.at = Some(now);
            l.exiting = l.text != text;
        }
        if self.layers.iter().any(|l| l.text == text) {
            for layer in &self.layers {
                if !layer.exiting {
                    layer
                        .view
                        .update(cx, |v, cx| v.set_active(self.motion.status.busy(), cx));
                }
            }
            return;
        }
        let busy = self.status().busy();
        let view = cx.new(|cx| {
            TextShimmer::new(text.clone(), cx)
                .with_font_size(13.)
                .with_active(busy)
        });
        self.layers.push(LabelLayer {
            text,
            view,
            opacity: if initial { 1. } else { 0. },
            from: 0.,
            at: if initial { None } else { Some(now) },
            exiting: false,
        });
    }
}
impl Render for AiSpark {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.cycle(cx);
        let now = cx.background_executor().now();
        let reduced = disabled_motion(cx);
        if reduced {
            self.motion.velocity = 0.;
            self.motion.angle = 90. * (self.motion.angle / 90. + 0.5).floor();
        }
        if self.window_visible != w.is_visible() {
            self.window_visible = w.is_visible();
            self.last_frame = None;
        }
        let scale = match self.size {
            MarkSize::Px(v) => v,
            MarkSize::Em(v) => v * f32::from(w.text_style().font_size.to_pixels(w.rem_size())),
            MarkSize::Rem(v) => v * f32::from(w.rem_size()),
        };
        let side = if self.variant == SparkVariant::Avatar {
            match self.size {
                MarkSize::Px(_) => (scale * 0.56).round(),
                _ => scale * 0.56,
            }
        } else {
            scale
        };
        let hint = match self.size {
            MarkSize::Px(_) => side,
            _ => side.round(),
        };
        let geo = Geometry::new(hint as f64);
        let colors = &Theme::global(cx).tokens.colors;
        let base = self.color.unwrap_or(match self.tone {
            SparkTone::Accent => colors.accent,
            SparkTone::Foreground => colors.foreground,
            SparkTone::Muted => colors.text_muted,
            SparkTone::Inherit => w.text_style().color,
        });
        let target = if self.status() == SparkStatus::Error {
            mix(colors.text_muted, colors.danger, 0.72)
        } else {
            base
        };
        let bg_target = mix(
            colors.surface,
            if self.status() == SparkStatus::Error {
                colors.danger
            } else {
                base
            },
            if self.status() == SparkStatus::Error {
                0.07
            } else {
                0.08
            },
        );
        let duration = if reduced { 0.16 } else { 0.32 };
        let (ink, ink_moving) = self
            .ink
            .get_or_insert(ColorFade {
                from: target,
                to: target,
                at: now,
            })
            .retarget(target, now, duration);
        let (bg, bg_moving) = self
            .background
            .get_or_insert(ColorFade {
                from: bg_target,
                to: bg_target,
                at: now,
            })
            .retarget(bg_target, now, duration);
        let border = colors.border;
        let surface = colors.surface;
        let secondary = colors.text_secondary;
        let (pose, shake) = self.motion.drawing(reduced);
        let angle = rounded(self.motion.angle, 3);
        let time = self.motion.time;
        let overall = rounded(pose[32] as f64, 4);
        let mark = canvas(
            |_, _, _| (),
            move |bounds, _, w, _| {
                let unit = side as f64 / 100. * overall;
                let center = bounds.center()
                    + point(px((rounded(shake, 2) * side as f64 / 100.) as f32), px(0.));
                for i in 0..8 {
                    paint_ray(
                        w,
                        geo.ray(i, &pose),
                        angle + 45. * i as f64,
                        center,
                        unit,
                        ink.opacity(
                            (rounded(pose[4 * i + 3] as f64, 3)
                                * if i % 2 == 0 { 1. } else { geo.tone })
                                as f32,
                        ),
                    );
                }
                paint_circle(
                    w,
                    center,
                    (rounded(geo.core * (pose[33] as f64).max(0.), 2) * unit) as f32,
                    ink.opacity(geo.tone as f32),
                );
                let particles = if geo.particles && !reduced {
                    pose[34] as f64
                } else {
                    0.
                };
                if particles >= 0.01 {
                    for i in 0..2 {
                        let degrees = if i == 0 {
                            30. + 360. * time / 2.6
                        } else {
                            210. + 360. * time / 3.9
                        };
                        let (sin, cos) = rounded(degrees % 360., 2).to_radians().sin_cos();
                        let at = center
                            + point(
                                px((47.5 * sin * unit) as f32),
                                px((-47.5 * cos * unit) as f32),
                            );
                        let opacity = rounded(
                            particles * (0.55 + 0.45 * breath(time / 1.3 + 0.5 * i as f64)),
                            3,
                        ) * if i == 0 { 0.8 } else { geo.tone };
                        paint_circle(
                            w,
                            at,
                            ((if i == 0 { 1.9 } else { 1.3 })
                                * rounded(0.5 + 0.5 * particles, 3)
                                * unit) as f32,
                            ink.opacity(opacity as f32),
                        );
                    }
                }
            },
        )
        .size(px(side))
        .flex_none();
        let text = self
            .label
            .clone()
            .unwrap_or_else(|| self.labels[self.status().index()].clone());
        let mut label = div().relative().h(px(18.)).flex_none();
        let mut label_moving = false;
        if self.variant == SparkVariant::Pill {
            self.sync_label(text.clone(), cx);
            let mut style = w.text_style();
            style.font_weight = FontWeight::NORMAL;
            let width = f32::from(
                w.text_system()
                    .shape_line(text.clone(), px(13.), &[style.to_run(text.len())], None)
                    .width,
            );
            if !self.sized || reduced {
                self.label_width.reset(width);
                self.sized = true;
            } else if (self.label_width.value(cx) - width).abs() >= 0.5 {
                self.label_width.duration(width, 0.24, cx);
            }
            label = label.w(px(self.label_width.get(w, cx)));
            for l in &self.layers {
                let opacity = l.opacity(now, reduced);
                let age =
                    l.at.map_or(1., |at| now.saturating_duration_since(at).as_secs_f32());
                let enter = standard_ease((age / if reduced { 0.1 } else { 0.16 }).clamp(0., 1.));
                label_moving |=
                    l.at.is_some() && age < if l.exiting || reduced { 0.1 } else { 0.16 };
                label = label.child(
                    div()
                        .absolute()
                        .top(px(if reduced || l.exiting {
                            0.
                        } else {
                            2. * (1. - enter)
                        }))
                        .opacity(opacity)
                        .child(l.view.clone()),
                );
            }
        }
        if self.motion.active(reduced) && !self.paused
            || ink_moving
            || bg_moving && self.variant == SparkVariant::Avatar
            || label_moving
        {
            self.queue(w, cx);
        } else {
            self.last_frame = None;
        }
        let name = self.aria_label.clone().unwrap_or_else(|| {
            if self.variant == SparkVariant::Pill {
                text
            } else {
                self.aria_labels[self.status().index()].clone()
            }
        });
        let viewport = w.viewport_size();
        let weak = cx.weak_entity();
        let visible = self.visible;
        div()
            .flex()
            .items_center()
            .flex_none()
            .line_height(relative(1.))
            .on_children_prepainted(move |bounds, _, cx| {
                if let Some(b) = bounds.first() {
                    let margin = px(48.);
                    let seen = b.left() < viewport.width + margin
                        && b.top() < viewport.height + margin
                        && b.right() > -margin
                        && b.bottom() > -margin;
                    let _ = weak.update(cx, |v, cx| {
                        if v.in_view != seen {
                            v.in_view = seen;
                            v.last_frame = None;
                            cx.notify();
                        }
                        for l in &v.layers {
                            l.view
                                .update(cx, |s, cx| s.set_visible(visible && seen, cx));
                        }
                    });
                }
            })
            .id(self.id.clone())
            .when(!self.decorative, |e| {
                e.role(if self.announce {
                    Role::Status
                } else {
                    Role::Image
                })
                .aria_label(name)
            })
            .when(self.variant == SparkVariant::Avatar, |e| {
                e.size(px(scale))
                    .justify_center()
                    .rounded(crate::foundation::surface_radius(cx))
                    .border_1()
                    .border_color(border)
                    .bg(bg)
            })
            .when(self.variant == SparkVariant::Pill, |e| {
                e.h(px(28.))
                    .gap(px(8.))
                    .pl(px(7.))
                    .pr(px(12.))
                    .border_1()
                    .border_color(border)
                    .rounded(crate::foundation::surface_radius(cx))
                    .bg(surface)
                    .font_weight(FontWeight::NORMAL)
                    .text_size(px(13.))
                    .line_height(px(18.))
                    .text_color(secondary)
            })
            .child(mark)
            .when(self.variant == SparkVariant::Pill, |e| e.child(label))
    }
}

#[cfg(test)]
mod tests {
    use super::{AiSpark, Geometry, Motion, SparkStatus, SparkVariant, heartbeat, mix, pose};
    use gpui::{TestAppContext, px, rgb, size};
    use std::time::Duration;

    #[test]
    fn interrupted_mixtures_remain_continuous_and_keep_their_phase() {
        let mut motion = Motion::new(SparkStatus::Thinking, true);
        for _ in 0..80 {
            motion.advance(1. / 60., 1., false);
        }
        motion.retarget(SparkStatus::Writing, true, false);
        for _ in 0..9 {
            motion.advance(1. / 60., 1., false);
        }
        let before = motion.drawing(false).0;
        let angle = motion.angle;
        let velocity = motion.velocity;
        motion.retarget(SparkStatus::Idle, true, false);
        let after = motion.drawing(false).0;
        for i in 0..35 {
            assert!(
                (before[i] - after[i]).abs() < 0.00001,
                "pose discontinuity at {i}"
            );
        }
        assert_eq!(motion.angle, angle);
        assert_eq!(motion.velocity, velocity);
        let time = motion.time;
        motion.advance(1. / 60., 1., false);
        assert!(motion.time > time);
        assert_ne!(motion.drawing(false).0, after);
    }
    #[test]
    fn a_result_glides_forward_to_a_quarter_turn_then_stops() {
        let mut motion = Motion::new(SparkStatus::Writing, true);
        for _ in 0..150 {
            motion.advance(1. / 60., 1., false);
        }
        let before = motion.angle;
        motion.retarget(SparkStatus::Done, true, false);
        motion.advance(1. / 60., 1., false);
        assert!(motion.angle >= before);
        for _ in 0..140 {
            motion.advance(1. / 60., 1., false);
        }
        assert!(!motion.active(false));
        assert_eq!(motion.velocity, 0.);
        assert!((motion.angle / 90. - (motion.angle / 90.).round()).abs() < 1e-9);
        assert!(motion.impulses.is_empty());
        assert_eq!(motion.drawing(false).0, pose(4, motion.time));
    }
    #[test]
    fn done_collects_rays_and_error_shakes_without_restarting_repeated_states() {
        let mut motion = Motion::new(SparkStatus::Done, true);
        motion.retarget(SparkStatus::Thinking, true, false);
        motion.advance(1. / 60., 1., false);
        motion.retarget(SparkStatus::Done, true, false);
        for _ in 0..10 {
            motion.advance(0.017, 1., false);
        }
        let (gathered, shake) = motion.drawing(false);
        assert_eq!(shake, 0.);
        assert!(gathered[32] < 1.);
        assert!(gathered[33] > 1.);
        let count = motion.impulses.len();
        motion.retarget(SparkStatus::Done, true, false);
        assert_eq!(motion.impulses.len(), count);
        motion.retarget(SparkStatus::Error, true, false);
        motion.advance(0.02, 1., false);
        assert!(motion.drawing(false).1.abs() > 1.);
    }
    #[test]
    fn optical_hinting_changes_at_the_actual_rendered_mark_size() {
        let small = Geometry::new(16.);
        let avatar = Geometry::new((32_f64 * 0.56).round());
        assert!(small.diagonal.dot);
        assert!(!avatar.diagonal.dot);
        assert_eq!(Geometry::new(27.).core, 0.);
        assert!(Geometry::new(28.).core > 0.);
        assert!(!Geometry::new(39.).particles);
        assert!(Geometry::new(40.).particles);
        let p = pose(3, 0.31);
        for side in [12., 16., 18., 24., 32., 48., 120.] {
            for i in 0..8 {
                let r = Geometry::new(side).ray(i, &p);
                assert!(r.base.is_finite() && r.base > 0.);
                assert!(r.outer >= r.inner);
            }
        }
    }
    #[test]
    fn heartbeat_has_two_distinct_beats_and_reduced_motion_only_blends_the_pose() {
        assert!((heartbeat(0.1) - 1.).abs() < 1e-9);
        assert!((heartbeat(0.31) - 0.62).abs() < 1e-9);
        assert_eq!(heartbeat(0.5), 0.);
        let mut motion = Motion::new(SparkStatus::Thinking, true);
        for _ in 0..10 {
            motion.advance(1. / 60., 1., false);
        }
        let time = motion.time;
        motion.retarget(SparkStatus::Error, true, true);
        assert!(motion.impulses.is_empty());
        let a = motion.drawing(true).0;
        for _ in 0..15 {
            motion.advance(1. / 60., 1., true);
        }
        assert_eq!(motion.time, time);
        assert_ne!(a, motion.drawing(true).0);
        assert!(!motion.active(true));
        assert_eq!(motion.drawing(true).0, pose(5, 0.));
    }
    #[test]
    fn initial_color_is_settled_without_backdating_the_browser_clock() {
        let now = web_time::Instant::now();
        let color = gpui::Hsla::white();
        let fade = super::ColorFade {
            from: color,
            to: color,
            at: now,
        };
        assert_eq!(fade.value(now, 0.32), (color, false));
    }
    #[test]
    fn color_tints_use_premultiplied_oklab() {
        let black = rgb(0).into();
        let white = rgb(0xffffff).into();
        let mid = gpui::Rgba::from(mix(black, white, 0.5));
        assert!((mid.r - 0.38857).abs() < 0.0001);
        assert!((mid.r - mid.g).abs() < 0.0001);
        let clear = gpui::hsla(0., 0., 0., 0.);
        let tinted = gpui::Rgba::from(mix(clear, white, 0.5));
        assert!((tinted.r - 1.).abs() < 0.0001);
        assert!((tinted.a - 0.5).abs() < 0.0001);
    }
    #[test]
    fn host_control_cancels_autoplay_and_pausing_preserves_phase() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (spark, window) =
            app.add_window_view(|_, _| AiSpark::new("test-spark").with_variant(SparkVariant::Pill));
        window.simulate_resize(size(px(400.), px(150.)));
        window.run_until_parked();
        window.executor().advance_clock(Duration::from_millis(1600));
        window.run_until_parked();
        window.update(|_, cx| {
            assert_eq!(spark.read(cx).status(), SparkStatus::Thinking);
            spark.update(cx, |v, cx| v.set_status(SparkStatus::Writing, cx));
        });
        for _ in 0..5 {
            window.executor().advance_clock(Duration::from_millis(16));
            window.update(|w, cx| {
                w.simulate_next_frame(cx);
            });
            window.run_until_parked();
        }
        let before = window.update(|_, cx| {
            spark.update(cx, |v, cx| {
                v.set_paused(true, cx);
                v.motion.time
            })
        });
        window.executor().advance_clock(Duration::from_secs(10));
        window.update(|w, cx| {
            w.simulate_next_frame(cx);
        });
        window.run_until_parked();
        window.update(|_, cx| {
            assert_eq!(spark.read(cx).motion.time, before);
            assert_eq!(spark.read(cx).status(), SparkStatus::Writing);
            assert!(!spark.read(cx).is_auto_playing());
            spark.update(cx, |v, cx| v.set_paused(false, cx));
        });
        window.run_until_parked();
        window.update(|w, cx| {
            w.simulate_next_frame(cx);
        });
        window.run_until_parked();
        window.update(|_, cx| {
            let after = spark.read(cx).motion.time;
            assert!(after > before);
            assert!(after - before <= 1. / 60. + 1e-9);
        });
    }
}
