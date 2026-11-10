//! Kense assistant mark with host-controlled activity poses.
use crate::primitives::{
    family::{Paint, disabled_motion},
    flyout::TriggerBounds,
    nav::tr,
};
use gpui::{prelude::*, *};
use web_time::Instant;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MascotStatus {
    #[default]
    Idle,
    Listening,
    Thinking,
    Writing,
    Done,
    Error,
}
/// Selects the role of the mascot's body and eye colors.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MascotTone {
    #[default]
    Accent,
    Ink,
    Soft,
}
/// Defines which mouse movements steer the idle or listening gaze.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MascotPointerFollow {
    None,
    #[default]
    Hover,
    Page,
}
#[derive(Clone, Copy, Debug, PartialEq)]
struct Pose {
    scale: f32,
    x: f32,
    y: f32,
    dot_x: f32,
    dot_y: f32,
    dot_size: f32,
    line: f32,
    sparkle: f32,
    eye_x: f32,
    eye_y: f32,
    eye_open: f32,
    smile: f32,
    brows: f32,
}
impl Pose {
    fn mix(self, to: Self, t: f32) -> Self {
        let f = |a: f32, b: f32| a + (b - a) * t;
        Self {
            scale: f(self.scale, to.scale),
            x: f(self.x, to.x),
            y: f(self.y, to.y),
            dot_x: f(self.dot_x, to.dot_x),
            dot_y: f(self.dot_y, to.dot_y),
            dot_size: f(self.dot_size, to.dot_size),
            line: f(self.line, to.line),
            sparkle: f(self.sparkle, to.sparkle),
            eye_x: f(self.eye_x, to.eye_x),
            eye_y: f(self.eye_y, to.eye_y),
            eye_open: f(self.eye_open, to.eye_open),
            smile: f(self.smile, to.smile),
            brows: f(self.brows, to.brows),
        }
    }
}
fn target(status: MascotStatus, time: f32, age: f32, reduced: bool) -> Pose {
    let wave = if reduced { 0. } else { (time * 1.6).sin() };
    let mut pose = Pose {
        scale: 1.,
        x: 0.,
        y: 0.,
        dot_x: 0.80,
        dot_y: 0.24,
        dot_size: 0.07,
        line: 0.,
        sparkle: 0.,
        eye_x: 0.,
        eye_y: 0.,
        eye_open: 1.,
        smile: 0.,
        brows: 0.,
    };
    match status {
        MascotStatus::Idle => {
            pose.scale = 1. + wave * 0.025;
            pose.y = wave * 0.015;
            pose.dot_y += wave * 0.02;
            pose.eye_x = wave * 0.037;
            if !reduced {
                let cycle = time.rem_euclid(12.4);
                for at in [2.8, 6.7, 11.4, 11.62] {
                    let blink = (1. - ((cycle - at) / 0.08).abs()).clamp(0., 1.);
                    pose.eye_open *= 1. - blink * 0.94;
                }
            }
        }
        MascotStatus::Listening => {
            pose.scale = 1.04 + wave * 0.012;
            pose.y = 0.025;
            pose.dot_size = 0.07 + wave.max(0.) * 0.025;
            pose.eye_open = 1.15;
        }
        MascotStatus::Thinking => {
            let angle = if reduced {
                -std::f32::consts::FRAC_PI_2
            } else {
                time * 1.5
            };
            pose.x = wave * 0.015;
            pose.dot_x = 0.5 + angle.cos() * 0.38;
            pose.dot_y = 0.42 + angle.sin() * 0.34;
            pose.eye_x = angle.cos() * 0.048;
            pose.eye_y = -0.065;
        }
        MascotStatus::Writing => {
            let cycle = if reduced {
                0.5
            } else {
                (time * 0.7).rem_euclid(1.)
            };
            pose.y = 0.025 + wave * 0.012;
            pose.dot_x = 0.22 + cycle * 0.56;
            pose.dot_y = 0.84;
            let envelope = (cycle * std::f32::consts::PI).sin().max(0.);
            pose.dot_size *= envelope;
            pose.line = if reduced { 0.5 } else { envelope };
            pose.eye_x = (cycle - 0.5) * envelope * 0.13;
            pose.eye_y = 0.058;
            pose.eye_open = 0.76;
        }
        MascotStatus::Done => {
            pose.scale = 1.03;
            pose.dot_x = 0.82;
            pose.dot_y = 0.20;
            pose.dot_size = 0.;
            pose.sparkle = 1.;
            pose.eye_open = 0.;
            pose.smile = 1.;
        }
        MascotStatus::Error => {
            let decay = if reduced {
                0.
            } else {
                (1. - age / 0.7).clamp(0., 1.).powi(2)
            };
            pose.x = (age * 27.).sin() * 0.06 * decay;
            pose.dot_x = 0.82;
            pose.dot_y = 0.22;
            pose.eye_open = 0.8;
            pose.brows = 1.;
        }
    }
    pose
}
pub struct AiMascot {
    name: SharedString,
    status: MascotStatus,
    size: f32,
    color: Option<Hsla>,
    eye_color: Option<Hsla>,
    tone: MascotTone,
    companion: bool,
    visible: bool,
    in_view: bool,
    paused: bool,
    elapsed: f32,
    phase_start: f32,
    blend: f32,
    from: Pose,
    current: Pose,
    last_frame: Option<Instant>,
    follow_pointer: MascotPointerFollow,
    pointer: Option<Point<Pixels>>,
    bounds: TriggerBounds,
}
impl AiMascot {
    pub fn new(name: impl Into<SharedString>) -> Self {
        let pose = target(MascotStatus::Idle, 0., 0., true);
        Self {
            name: name.into(),
            status: MascotStatus::Idle,
            size: 96.,
            color: None,
            eye_color: None,
            tone: MascotTone::default(),
            companion: true,
            visible: true,
            in_view: true,
            paused: false,
            elapsed: 0.,
            phase_start: 0.,
            blend: 0.,
            from: pose,
            current: pose,
            last_frame: None,
            follow_pointer: MascotPointerFollow::default(),
            pointer: None,
            bounds: TriggerBounds::default(),
        }
    }
    pub fn with_size(mut self, size: f32) -> Self {
        if size.is_finite() {
            self.size = size.clamp(24., 160.);
        }
        self
    }
    pub fn with_color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }
    pub fn with_eye_color(mut self, color: Hsla) -> Self {
        self.eye_color = Some(color);
        self
    }
    pub fn with_tone(mut self, tone: MascotTone) -> Self {
        self.tone = tone;
        self
    }
    pub fn with_companion(mut self, visible: bool) -> Self {
        self.companion = visible;
        self
    }
    pub fn with_status(mut self, status: MascotStatus) -> Self {
        self.status = status;
        self.from = target(status, 0., 0., true);
        self.current = self.from;
        self
    }
    pub fn with_pointer_follow(mut self, follow: MascotPointerFollow) -> Self {
        self.follow_pointer = follow;
        self
    }
    pub fn follow_pointer(self, enabled: bool) -> Self {
        self.with_pointer_follow(if enabled {
            MascotPointerFollow::Hover
        } else {
            MascotPointerFollow::None
        })
    }
    pub fn status(&self) -> MascotStatus {
        self.status
    }
    pub fn set_status(&mut self, status: MascotStatus, cx: &mut Context<Self>) {
        if self.status != status {
            self.from = self.current;
            self.status = status;
            self.phase_start = self.elapsed;
            self.blend = 0.;
            cx.notify();
        }
    }
    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        if self.visible != visible {
            self.visible = visible;
            self.last_frame = None;
            cx.notify();
        }
    }
    pub fn set_paused(&mut self, paused: bool, cx: &mut Context<Self>) {
        if self.paused != paused {
            self.paused = paused;
            self.last_frame = None;
            cx.notify();
        }
    }
}
impl Render for AiMascot {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let reduced = disabled_motion(cx);
        let can_move = self.visible && self.in_view && w.is_visible() && !self.paused && !reduced;
        if can_move {
            let now = Instant::now();
            if let Some(last) = self.last_frame {
                let dt = now.saturating_duration_since(last).as_secs_f32().min(0.1);
                self.elapsed += dt;
                self.blend = (self.blend + dt / 0.42).min(1.);
            }
            self.last_frame = Some(now);
        } else {
            self.last_frame = None;
            if reduced {
                self.blend = 1.;
            }
        }
        let age = (self.elapsed - self.phase_start).max(0.);
        let target = target(self.status, self.elapsed, age, reduced);
        let t = self.blend.clamp(0., 1.);
        let ease = t * t * (3. - 2. * t);
        self.current = self.from.mix(target, ease);
        let loop_active = matches!(
            self.status,
            MascotStatus::Idle
                | MascotStatus::Listening
                | MascotStatus::Thinking
                | MascotStatus::Writing
        );
        if can_move
            && (loop_active || self.blend < 1. || (self.status == MascotStatus::Error && age < 0.7))
        {
            w.request_animation_frame();
        }
        let mut pose = self.current;
        let side = self.size;
        let compact = side < 56.;
        let pointer = match self.follow_pointer {
            MascotPointerFollow::None => None,
            MascotPointerFollow::Hover => self.pointer,
            MascotPointerFollow::Page => Some(w.mouse_position() - self.bounds.get().origin),
        };
        if can_move
            && !w.last_input_was_touch()
            && matches!(self.status, MascotStatus::Idle | MascotStatus::Listening)
            && let Some(pointer) = pointer
        {
            pose.eye_x += (f32::from(pointer.x) / side - 0.5).clamp(-0.5, 0.5) * 0.13;
            pose.eye_y += (f32::from(pointer.y) / side - 0.5).clamp(-0.5, 0.5) * 0.12;
        }
        let label = match self.status {
            MascotStatus::Idle => tr(cx, "idle", "空闲"),
            MascotStatus::Listening => tr(cx, "listening", "聆听中"),
            MascotStatus::Thinking => tr(cx, "thinking", "思考中"),
            MascotStatus::Writing => tr(cx, "writing", "生成中"),
            MascotStatus::Done => tr(cx, "done", "已完成"),
            MascotStatus::Error => tr(cx, "error", "出错"),
        };
        let (base, eyes) = match self.tone {
            MascotTone::Accent => {
                let tint = if p.panel.l > 0.5 { 0.13 } else { 0.23 };
                (p.bright.blend(p.panel.opacity(tint)), p.inverse)
            }
            MascotTone::Ink => (p.ink, p.panel),
            MascotTone::Soft => (p.tray, p.ink),
        };
        let color = self.color.unwrap_or(base);
        let eyes = self.eye_color.unwrap_or_else(|| {
            self.color
                .map_or(eyes, |color| color.blend(gpui::black().opacity(0.78)))
        });
        let body_w = side * if compact { 0.78 } else { 0.64 } * pose.scale;
        let body_h = body_w * 0.945;
        let body_x = side * (0.5 + pose.x) - body_w / 2.;
        let body_y = side * (0.49 + pose.y) - body_h / 2.;
        let dot_size = (side * pose.dot_size).max(if compact && pose.dot_size > 0. {
            3.
        } else {
            0.
        });
        let dot_x = side * pose.dot_x - dot_size / 2.;
        let dot_y = side * pose.dot_y - dot_size / 2.;
        let dot = || {
            div()
                .absolute()
                .left(px(dot_x))
                .top(px(dot_y))
                .size(px(dot_size))
                .rounded_full()
                .bg(
                    if self.status == MascotStatus::Thinking
                        && !reduced
                        && dot_y < body_y + body_h * 0.48
                        && dot_x > body_x
                        && dot_x < body_x + body_w
                    {
                        eyes
                    } else {
                        color
                    },
                )
        };
        let back =
            self.status == MascotStatus::Thinking && !reduced && (self.elapsed * 1.5).cos() < 0.;
        let viewport = w.viewport_size();
        let weak = cx.weak_entity();
        let mut root = div()
            .on_children_prepainted(move |bounds, _, cx| {
                let visible = bounds.iter().any(|b| {
                    b.left() < viewport.width
                        && b.top() < viewport.height
                        && b.right() > px(0.)
                        && b.bottom() > px(0.)
                });
                let _ = weak.update(cx, |s, cx| {
                    if s.in_view != visible {
                        s.in_view = visible;
                        s.last_frame = None;
                        cx.notify();
                    }
                });
            })
            .id("ai-mascot")
            .role(Role::Image)
            .aria_label(format!("{}, {}", self.name, label))
            .relative()
            .size(px(side))
            .flex_none()
            .child(self.bounds.measure())
            .on_mouse_move(cx.listener(move |s, e: &MouseMoveEvent, w, cx| {
                if s.follow_pointer != MascotPointerFollow::Hover
                    || w.last_input_was_touch()
                    || disabled_motion(cx)
                    || !matches!(s.status, MascotStatus::Idle | MascotStatus::Listening)
                {
                    return;
                }
                let origin = s.bounds.get().origin;
                s.pointer = Some(e.position - origin);
                cx.notify();
            }))
            .on_hover(cx.listener(|s, over: &bool, _, cx| {
                if !*over && s.pointer.take().is_some() {
                    cx.notify();
                }
            }));
        if !compact {
            root = root.child(
                div()
                    .absolute()
                    .left(px(side * 0.24))
                    .top(px(side * 0.84))
                    .w(px(side * 0.52))
                    .h(px(side * 0.035))
                    .rounded_full()
                    .bg(p.ink.opacity(0.08)),
            );
        }
        if self.companion && back && dot_size > 0. {
            root = root.child(dot());
        }
        let outline = (self.tone == MascotTone::Soft && self.color.is_none()).then_some(p.line);
        root = root.child(
            canvas(
                |_, _, _| (),
                move |b, _, w, _| {
                    let center = point(b.origin.x + px(body_x), b.origin.y + px(body_y));
                    let f = |x: f32, y: f32| {
                        point(center.x + px(x * body_w), center.y + px(y * body_h))
                    };
                    let shade = color.blend(gpui::black().opacity(0.16));
                    paint_body(
                        w,
                        point(center.x + px(body_w * 0.03), center.y + px(body_h * 0.035)),
                        body_w,
                        body_h,
                        shade,
                        None,
                    );
                    paint_body(w, center, body_w, body_h, color, outline);
                    let eye_w = px(body_w * if compact { 0.13 } else { 0.105 });
                    let eye_h = px(body_h * 0.24 * pose.eye_open.max(0.055));
                    for (i, x) in [0.35, 0.70].into_iter().enumerate() {
                        let x = x + pose.eye_x;
                        let y = 0.53 + pose.eye_y;
                        let eye = f(x, y);
                        let pill =
                            Bounds::new(eye - point(eye_w / 2., eye_h / 2.), size(eye_w, eye_h));
                        w.paint_quad(
                            fill(pill, eyes.opacity(1. - pose.smile)).corner_radii(eye_w / 2.),
                        );
                        if pose.smile > 0.001 {
                            let mut squint = PathBuilder::stroke(eye_w * 0.62);
                            squint.move_to(f(x - 0.082, y + 0.015));
                            squint.cubic_bezier_to(
                                f(x + 0.082, y + 0.015),
                                f(x - 0.042, y - 0.085),
                                f(x + 0.042, y - 0.085),
                            );
                            if let Ok(path) = squint.build() {
                                w.paint_path(path, eyes.opacity(pose.smile));
                            }
                        }
                        if pose.brows > 0.001 {
                            let slope = if i == 0 { -0.055 } else { 0.055 };
                            let mut brow = PathBuilder::stroke(eye_w * 0.38);
                            brow.move_to(f(x - 0.075, y - 0.17 - slope));
                            brow.line_to(f(x + 0.075, y - 0.17 + slope));
                            if let Ok(path) = brow.build() {
                                w.paint_path(path, eyes.opacity(pose.brows));
                            }
                        }
                    }
                },
            )
            .absolute()
            .inset_0(),
        );
        if self.companion && !back && dot_size > 0. {
            root = root.child(dot());
        }
        if pose.line > 0. && !compact {
            root = root.child(
                div()
                    .absolute()
                    .left(px(side * 0.22))
                    .top(px(side * 0.86))
                    .w(px(side * 0.56 * pose.line))
                    .h(px(2.))
                    .rounded(px(1.))
                    .bg(color.opacity(0.65)),
            );
        }
        if self.companion && pose.sparkle > 0.001 {
            root = root.child(
                canvas(
                    |_, _, _| (),
                    move |b, _, w, _| {
                        let at = |x, y| point(b.origin.x + px(x), b.origin.y + px(y));
                        let radius = side * 0.077 * pose.sparkle;
                        let x = side * pose.dot_x;
                        let y = side * pose.dot_y;
                        let mut sparkle = PathBuilder::fill();
                        sparkle.move_to(at(x, y - radius));
                        for (dx, dy) in [
                            (0.26, -0.26),
                            (1., 0.),
                            (0.26, 0.26),
                            (0., 1.),
                            (-0.26, 0.26),
                            (-1., 0.),
                            (-0.26, -0.26),
                        ] {
                            sparkle.line_to(at(x + dx * radius, y + dy * radius));
                        }
                        sparkle.close();
                        if let Ok(path) = sparkle.build() {
                            w.paint_path(path, color.opacity(pose.sparkle));
                        }
                    },
                )
                .absolute()
                .inset_0(),
            );
        }
        root
    }
}
fn paint_body(
    w: &mut Window,
    origin: Point<Pixels>,
    width: f32,
    height: f32,
    color: Hsla,
    outline: Option<Hsla>,
) {
    let at = |x: f32, y: f32| point(origin.x + px(x * width), origin.y + px(y * height));
    let curve = |path: &mut PathBuilder| {
        path.move_to(at(0.51, 0.));
        path.cubic_bezier_to(at(1., 0.57), at(0.82, 0.), at(1., 0.23));
        path.cubic_bezier_to(at(0.51, 1.), at(1., 0.86), at(0.8, 1.));
        path.cubic_bezier_to(at(0., 0.57), at(0.19, 1.), at(0., 0.83));
        path.cubic_bezier_to(at(0.51, 0.), at(0., 0.24), at(0.19, 0.));
        path.close();
    };
    let mut body = PathBuilder::fill();
    curve(&mut body);
    if let Ok(path) = body.build() {
        w.paint_path(path, color);
    }
    if let Some(ink) = outline {
        let mut border = PathBuilder::stroke(px(1.));
        curve(&mut border);
        if let Ok(path) = border.build() {
            w.paint_path(path, ink);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::{MascotStatus, target};
    #[test]
    fn reduced_motion_has_stable_distinct_poses() {
        let a = target(MascotStatus::Thinking, 1., 1., true);
        let b = target(MascotStatus::Thinking, 19., 5., true);
        assert_eq!(a, b);
        assert_ne!(a, target(MascotStatus::Done, 1., 1., true));
    }
}
