//! A host-driven voice visualization; it never opens an audio device.
use crate::primitives::{
    family::{Paint, PhaseClock, disabled_motion},
    nav::{button, tr},
};
use gpui::{prelude::*, *};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VoiceState {
    Idle,
    Listening,
    Thinking,
    Speaking,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VoiceOrbEvent {
    Start,
    End,
    ToggleMute,
}
pub struct VoiceOrb {
    state: VoiceState,
    level: f32,
    muted: bool,
    transcript: SharedString,
    visible: bool,
    in_view: bool,
    elapsed: u64,
    clock: PhaseClock,
}
impl EventEmitter<VoiceOrbEvent> for VoiceOrb {}
impl VoiceOrb {
    pub fn new(cx: &App) -> Self {
        let mut clock = PhaseClock::default();
        clock.restart(cx);
        Self {
            state: VoiceState::Idle,
            level: 0.,
            muted: false,
            transcript: "".into(),
            visible: true,
            in_view: true,
            elapsed: 0,
            clock,
        }
    }
    pub fn set_state(&mut self, state: VoiceState, cx: &mut Context<Self>) {
        self.state = state;
        self.clock.restart(cx);
        cx.notify();
    }
    pub fn set_level(&mut self, level: f32, cx: &mut Context<Self>) {
        self.level = if level.is_finite() {
            level.clamp(0., 1.)
        } else {
            0.
        };
        cx.notify();
    }
    pub fn set_muted(&mut self, muted: bool, cx: &mut Context<Self>) {
        self.muted = muted;
        cx.notify();
    }
    pub fn set_transcript(&mut self, text: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.transcript = text.into();
        cx.notify();
    }
    pub fn set_elapsed(&mut self, seconds: u64, cx: &mut Context<Self>) {
        self.elapsed = seconds;
        cx.notify();
    }
    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        if self.visible != visible {
            self.visible = visible;
            cx.notify();
        }
    }
}
#[derive(Clone, Copy)]
struct OrbMotion {
    breath: f32,
    wobble: f32,
    flow: f32,
    gain: f32,
    halo: f32,
    grow: f32,
}

fn orb_motion(state: VoiceState, muted: bool) -> OrbMotion {
    if muted {
        return OrbMotion {
            breath: 0.01,
            wobble: 0.01,
            flow: 0.25,
            gain: 0.,
            halo: 0.3,
            grow: -0.02,
        };
    }
    match state {
        VoiceState::Idle => OrbMotion {
            breath: 0.028,
            wobble: 0.014,
            flow: 0.32,
            gain: 0.,
            halo: 0.55,
            grow: 0.,
        },
        VoiceState::Listening => OrbMotion {
            breath: 0.01,
            wobble: 0.022,
            flow: 0.7,
            gain: 0.15,
            halo: 0.75,
            grow: 0.03,
        },
        VoiceState::Thinking => OrbMotion {
            breath: 0.012,
            wobble: 0.03,
            flow: 0.55,
            gain: 0.,
            halo: 0.6,
            grow: -0.08,
        },
        VoiceState::Speaking => OrbMotion {
            breath: 0.006,
            wobble: 0.026,
            flow: 0.95,
            gain: 0.19,
            halo: 0.9,
            grow: 0.02,
        },
    }
}

fn orb_scale(grow: f32, breath: f32, clock: f32, voice: f32) -> f32 {
    1. + grow + breath * (1.25 * clock).sin() + 0.9 * voice
}

impl Render for VoiceOrb {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let active = self.state != VoiceState::Idle;
        let animate = self.visible && self.in_view && w.is_visible() && !disabled_motion(cx);
        let age = self.clock.age(cx);
        if animate {
            w.request_animation_frame();
        }
        let motion = orb_motion(self.state, self.muted);
        let voice = self.level * motion.gain;
        let scale = if animate {
            orb_scale(motion.grow, motion.breath, age, voice)
        } else {
            1. + motion.grow
        };
        let size = 208. * scale;
        let mut primary = crate::foundation::Theme::global(cx)
            .accent
            .for_mode(crate::foundation::Theme::global(cx).mode);
        let default_tint = primary.a < 0.01;
        if default_tint {
            primary = hsla(0., 0., 0.82, 1.);
        }
        let secondary = if default_tint {
            hsla(0., 0., 0.54, 1.)
        } else {
            hsla(
                (primary.h + 0.05) % 1.,
                primary.s * 0.8,
                (primary.l * 0.68).clamp(0., 1.),
                1.,
            )
        };
        let status = match self.state {
            VoiceState::Idle => tr(cx, "Ready", "准备就绪"),
            VoiceState::Listening if self.muted => tr(cx, "Muted", "已静音"),
            VoiceState::Listening => tr(cx, "Listening", "正在聆听"),
            VoiceState::Thinking => tr(cx, "Thinking", "正在思考"),
            VoiceState::Speaking => tr(cx, "Speaking", "正在回应"),
        };
        let action = if active {
            VoiceOrbEvent::End
        } else {
            VoiceOrbEvent::Start
        };
        let viewport = w.viewport_size();
        let weak = cx.weak_entity();
        let mut root = div()
            .on_children_prepainted(move |bounds, _, cx| {
                if let Some(bounds) = bounds.get(1) {
                    let on_screen = bounds.origin.x < viewport.width
                        && bounds.origin.y < viewport.height
                        && bounds.origin.x + bounds.size.width > px(0.)
                        && bounds.origin.y + bounds.size.height > px(0.);
                    let _ = weak.update(cx, |s, cx| {
                        if s.in_view != on_screen {
                            s.in_view = on_screen;
                            cx.notify();
                        }
                    });
                }
            })
            .id("voice-orb-root")
            .role(Role::Group)
            .aria_label(status)
            .flex()
            .flex_col()
            .items_center()
            .gap(px(14.))
            .child(
                div()
                    .h(px(20.))
                    .text_size(px(14.))
                    .line_height(px(20.))
                    .text_color(p.secondary)
                    .child(status),
            )
            .child(
                div()
                    .id("voice-orb")
                    .role(Role::Button)
                    .aria_label(if active {
                        tr(cx, "End voice chat", "结束语音对话")
                    } else {
                        tr(cx, "Start voice chat", "开始语音对话")
                    })
                    .tab_index(0)
                    .size(px(208.))
                    .overflow_hidden()
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .on_click(cx.listener(move |_, _, _, cx| cx.emit(action)))
                    .child(
                        div()
                            .size(px(208.))
                            .rounded_full()
                            .bg(primary.opacity(0.12 * motion.halo))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                canvas(
                                    |_, _, _| (),
                                    move |bounds, _, w, _| {
                                        let center = bounds.center();
                                        let radius = size * 0.46;
                                        let mut body = PathBuilder::fill();
                                        for i in 0..96 {
                                            let angle = std::f32::consts::TAU * i as f32 / 96.;
                                            let wobble = if animate {
                                                (angle * 3. + age * motion.flow).sin()
                                                    * motion.wobble
                                            } else {
                                                0.
                                            };
                                            let radius = radius * (1. + wobble);
                                            let point = center
                                                + point(
                                                    px(angle.cos() * radius),
                                                    px(angle.sin() * radius),
                                                );
                                            if i == 0 {
                                                body.move_to(point);
                                            } else {
                                                body.line_to(point);
                                            }
                                        }
                                        body.close();
                                        if let Ok(path) = body.build() {
                                            w.paint_path(
                                                path,
                                                linear_gradient(
                                                    135.,
                                                    linear_color_stop(primary, 0.),
                                                    linear_color_stop(secondary, 1.),
                                                ),
                                            );
                                        }
                                        // Layered translucent paths create a soft, moving specular light
                                        // without requiring a platform-specific shader.
                                        for layer in 0..4 {
                                            let offset = if animate {
                                                (age * 0.55).sin() * radius * 0.03
                                            } else {
                                                0.
                                            };
                                            let light_center = center
                                                + point(
                                                    px(-radius * 0.28 + offset),
                                                    px(-radius * 0.31),
                                                );
                                            let light_radius =
                                                radius * (0.21 + layer as f32 * 0.065);
                                            let mut light = PathBuilder::fill();
                                            for i in 0..48 {
                                                let angle = std::f32::consts::TAU * i as f32 / 48.;
                                                let point = light_center
                                                    + point(
                                                        px(angle.cos() * light_radius),
                                                        px(angle.sin() * light_radius),
                                                    );
                                                if i == 0 {
                                                    light.move_to(point);
                                                } else {
                                                    light.line_to(point);
                                                }
                                            }
                                            light.close();
                                            if let Ok(path) = light.build() {
                                                w.paint_path(
                                                    path,
                                                    linear_gradient(
                                                        135.,
                                                        linear_color_stop(
                                                            white().opacity(
                                                                0.16 - layer as f32 * 0.025,
                                                            ),
                                                            0.,
                                                        ),
                                                        linear_color_stop(white().opacity(0.), 1.),
                                                    ),
                                                );
                                            }
                                        }
                                    },
                                )
                                .size(px(size)),
                            ),
                    ),
            )
            .child(
                div()
                    .max_w(px(480.))
                    .h(px(45.))
                    .text_center()
                    .text_size(px(16.))
                    .line_height(px(22.))
                    .text_color(p.ink)
                    .child(self.transcript.clone()),
            );
        let controls = if active {
            let mute_label = if self.muted {
                tr(cx, "Unmute", "取消静音")
            } else {
                tr(cx, "Mute", "静音")
            };
            div()
                .h(px(48.))
                .px(px(4.))
                .gap(px(4.))
                .flex()
                .items_center()
                .border_1()
                .border_color(p.line)
                .rounded(px(8.))
                .bg(p.panel)
                .shadow_sm()
                .child(
                    button("voice-mute", mute_label, cx)
                        .size(px(40.))
                        .rounded_full()
                        .p_0()
                        .when(self.muted, |b| b.bg(p.ink).text_color(p.inverse))
                        .when(!self.muted, |b| b.hover(|s| s.bg(p.tray)))
                        .child(mute_label)
                        .on_click(cx.listener(|_, _, _, cx| cx.emit(VoiceOrbEvent::ToggleMute))),
                )
                .child(
                    div()
                        .min_w(px(36.))
                        .px(px(8.))
                        .text_center()
                        .text_size(px(14.))
                        .line_height(px(20.))
                        .text_color(p.secondary)
                        .child(format!("{}:{:02}", self.elapsed / 60, self.elapsed % 60)),
                )
                .child(
                    button("voice-session", tr(cx, "End", "结束"), cx)
                        .size(px(40.))
                        .rounded_full()
                        .p_0()
                        .bg(p.danger.opacity(0.12))
                        .text_color(p.danger)
                        .hover(|s| s.bg(p.danger.opacity(0.18)))
                        .child(tr(cx, "End", "结束"))
                        .on_click(cx.listener(move |_, _, _, cx| cx.emit(action))),
                )
        } else {
            div()
                .h(px(48.))
                .border_1()
                .border_color(p.line)
                .rounded(px(8.))
                .bg(p.panel)
                .shadow_sm()
                .child(
                    button(
                        "voice-session",
                        tr(cx, "Start voice chat", "开始语音对话"),
                        cx,
                    )
                    .h(px(48.))
                    .pl(px(16.))
                    .pr(px(20.))
                    .gap(px(8.))
                    .text_size(px(14.))
                    .line_height(px(20.))
                    .font_weight(FontWeight::MEDIUM)
                    .hover(|s| s.bg(p.tray))
                    .child(tr(cx, "Start voice chat", "开始语音对话"))
                    .on_click(cx.listener(move |_, _, _, cx| cx.emit(action))),
                )
        };
        root = root.child(controls);
        root
    }
}

#[cfg(test)]
mod tests {
    use super::orb_scale;

    #[test]
    fn scale_follows_the_source_breath_and_voice() {
        assert!((orb_scale(0., 0.028, 0., 0.) - 1.).abs() < 0.0001);
        assert!((orb_scale(0.02, 0., 0., 0.19) - (1.02 + 0.171)).abs() < 0.0001);
        assert!((orb_scale(-0.08, 0., 0., 0.) - 0.92).abs() < 0.0001);
    }
}
