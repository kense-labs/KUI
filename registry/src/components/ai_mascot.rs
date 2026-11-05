//! Kense assistant mark with host-controlled activity poses.
use crate::{
    foundation::{Theme, ThemeMode},
    primitives::{
        family::{Paint, disabled_motion},
        flyout::TriggerBounds,
        glyph::{Glyph, icon},
        kense_mark,
        nav::tr,
    },
};
use gpui::{prelude::*, *};
use std::sync::{Arc, OnceLock};
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
    };
    match status {
        MascotStatus::Idle => {
            pose.scale = 1. + wave * 0.025;
            pose.y = wave * 0.015;
            pose.dot_y += wave * 0.02;
        }
        MascotStatus::Listening => {
            pose.scale = 1.04 + wave * 0.012;
            pose.y = 0.025;
            pose.dot_size = 0.07 + wave.max(0.) * 0.025;
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
        }
        MascotStatus::Done => {
            pose.scale = 1.03;
            pose.dot_x = 0.82;
            pose.dot_y = 0.20;
            pose.dot_size = 0.;
            pose.sparkle = 1.;
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
        }
    }
    pose
}
pub struct AiMascot {
    name: SharedString,
    status: MascotStatus,
    size: f32,
    color: Option<Hsla>,
    visible: bool,
    in_view: bool,
    paused: bool,
    elapsed: f32,
    phase_start: f32,
    blend: f32,
    from: Pose,
    current: Pose,
    last_frame: Option<Instant>,
    follow_pointer: bool,
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
            visible: true,
            in_view: true,
            paused: false,
            elapsed: 0.,
            phase_start: 0.,
            blend: 0.,
            from: pose,
            current: pose,
            last_frame: None,
            follow_pointer: true,
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
    pub fn follow_pointer(mut self, enabled: bool) -> Self {
        self.follow_pointer = enabled;
        self
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
        if self.follow_pointer
            && !reduced
            && matches!(self.status, MascotStatus::Idle | MascotStatus::Listening)
            && let Some(pointer) = self.pointer
        {
            let gain = if compact { 0.012 } else { 0.025 };
            pose.x += (f32::from(pointer.x) / side - 0.5).clamp(-0.5, 0.5) * gain;
            pose.y += (f32::from(pointer.y) / side - 0.5).clamp(-0.5, 0.5) * gain;
        }
        let label = match self.status {
            MascotStatus::Idle => tr(cx, "idle", "空闲"),
            MascotStatus::Listening => tr(cx, "listening", "聆听中"),
            MascotStatus::Thinking => tr(cx, "thinking", "思考中"),
            MascotStatus::Writing => tr(cx, "writing", "生成中"),
            MascotStatus::Done => tr(cx, "done", "已完成"),
            MascotStatus::Error => tr(cx, "error", "出错"),
        };
        let color = self.color.unwrap_or(p.bright);
        static LIGHT: OnceLock<Arc<Image>> = OnceLock::new();
        static DARK: OnceLock<Arc<Image>> = OnceLock::new();
        let (cache, bytes) = match Theme::global(cx).mode {
            ThemeMode::Light => (&LIGHT, kense_mark::LIGHT),
            ThemeMode::Dark => (&DARK, kense_mark::DARK),
        };
        let mark = cache
            .get_or_init(|| Arc::new(Image::from_bytes(ImageFormat::Png, bytes.to_vec())))
            .clone();
        let body_side = side * if compact { 0.68 } else { 0.58 } * pose.scale;
        let body_x = side * 0.5 - body_side * 0.5 + side * pose.x;
        let body_y = side * 0.5 - body_side * 0.5 + side * pose.y;
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
                .bg(color)
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
            .aria_label(format!("{} · {}", self.name, label))
            .relative()
            .size(px(side))
            .flex_none()
            .child(self.bounds.measure())
            .on_mouse_move(cx.listener(move |s, e: &MouseMoveEvent, _, cx| {
                if !s.follow_pointer
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
        if back && dot_size > 0. {
            root = root.child(dot());
        }
        root = root.child(
            div()
                .absolute()
                .left(px(body_x))
                .top(px(body_y))
                .size(px(body_side))
                .rounded(crate::foundation::surface_radius(cx))
                .border_1()
                .border_color(p.line)
                .bg(p.panel)
                .flex()
                .items_center()
                .justify_center()
                .child(
                    img(mark)
                        .size(px(body_side * 0.75))
                        .object_fit(ObjectFit::Contain),
                ),
        );
        if !back && dot_size > 0. {
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
        if pose.sparkle > 0. {
            root = root.child(
                div()
                    .absolute()
                    .left(px(side * 0.72))
                    .top(px(side * 0.10))
                    .opacity(pose.sparkle)
                    .child(icon(Glyph::Check, side * 0.16, color)),
            );
        }
        if self.status == MascotStatus::Error {
            root = root.child(
                div()
                    .absolute()
                    .left(px(side * 0.73))
                    .top(px(side * 0.10))
                    .child(icon(Glyph::Close, side * 0.16, p.danger)),
            );
        }
        root
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
