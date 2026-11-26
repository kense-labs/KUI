//! A host-driven voice visualization; it never opens an audio device.
use crate::primitives::{
    family::{Paint, PhaseClock, Tone, disabled_motion, face},
    glyph::{Glyph, icon},
    nav::tr,
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
/// `VoiceOrb` component.
///
/// # Examples
///
/// ```no_run
/// # #[allow(unused_variables, unused_imports)]
/// # fn example(window: &mut gpui::Window, cx: &mut gpui::App) {
/// # use gpui::prelude::*;
/// use gpui::prelude::*;
/// use kui::components::voice_orb::VoiceOrb;
///
/// let control = cx.new(|cx| VoiceOrb::new(cx));
/// # }
/// ```
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
            halo: 0.42,
            grow: -0.02,
        };
    }
    match state {
        VoiceState::Idle => OrbMotion {
            breath: 0.028,
            wobble: 0.014,
            flow: 0.42,
            gain: 0.,
            halo: 0.68,
            grow: 0.,
        },
        VoiceState::Listening => OrbMotion {
            breath: 0.01,
            wobble: 0.026,
            flow: 0.95,
            gain: 0.15,
            halo: 0.92,
            grow: 0.03,
        },
        VoiceState::Thinking => OrbMotion {
            breath: 0.012,
            wobble: 0.022,
            flow: 1.35,
            gain: 0.,
            halo: 0.78,
            grow: -0.08,
        },
        VoiceState::Speaking => OrbMotion {
            breath: 0.006,
            wobble: 0.032,
            flow: 1.15,
            gain: 0.19,
            halo: 1.0,
            grow: 0.02,
        },
    }
}

fn orb_scale(grow: f32, breath: f32, clock: f32, voice: f32) -> f32 {
    1. + grow + breath * (1.25 * clock).sin() + 0.9 * voice
}

#[derive(Clone, Copy)]
struct OrbPalette {
    core_a: Hsla,
    core_b: Hsla,
    lobe_a: Hsla,
    lobe_b: Hsla,
    lobe_c: Hsla,
    rim: Hsla,
    aura: Hsla,
}

fn orb_palette(
    state: VoiceState,
    muted: bool,
    dark: bool,
    custom_accent: Option<Hsla>,
) -> OrbPalette {
    if muted {
        // Muted / error-adjacent subdued rose-plum sphere
        return OrbPalette {
            core_a: rgb(if dark { 0x26121b } else { 0x3b1827 }).into(),
            core_b: rgb(if dark { 0x160d14 } else { 0x24111b }).into(),
            lobe_a: rgb(0xf43f5e).into(),
            lobe_b: rgb(0xfb7185).into(),
            lobe_c: rgb(0x9f1239).into(),
            rim: rgb(0xfda4af).into(),
            aura: rgb(0xf43f5e).into(),
        };
    }
    if let Some(accent) = custom_accent {
        let shift_a = hsla(
            (accent.h + 0.92) % 1.,
            (accent.s * 0.95).clamp(0.45, 1.),
            0.62,
            1.,
        );
        let shift_b = hsla(
            (accent.h + 0.08) % 1.,
            (accent.s * 0.9).clamp(0.45, 1.),
            0.56,
            1.,
        );
        let core_a = hsla(accent.h, (accent.s * 0.7).clamp(0.3, 0.8), 0.18, 1.);
        let core_b = hsla((accent.h + 0.06) % 1., 0.55, 0.11, 1.);
        return OrbPalette {
            core_a,
            core_b,
            lobe_a: shift_a,
            lobe_b: accent,
            lobe_c: shift_b,
            rim: hsla(accent.h, 0.75, 0.84, 1.),
            aura: accent,
        };
    }
    match state {
        VoiceState::Idle => OrbPalette {
            core_a: rgb(if dark { 0x101830 } else { 0x172342 }).into(),
            core_b: rgb(if dark { 0x1a1233 } else { 0x241945 }).into(),
            lobe_a: rgb(0x38bdf8).into(),
            lobe_b: rgb(0x6366f1).into(),
            lobe_c: rgb(0xa855f7).into(),
            rim: rgb(0xbae6fd).into(),
            aura: rgb(0x6366f1).into(),
        },
        VoiceState::Listening => OrbPalette {
            core_a: rgb(if dark { 0x0b1f38 } else { 0x102a4a }).into(),
            core_b: rgb(if dark { 0x1e1342 } else { 0x281b56 }).into(),
            lobe_a: rgb(0x22d3ee).into(),
            lobe_b: rgb(0x6366f1).into(),
            lobe_c: rgb(0xc084fc).into(),
            rim: rgb(0xa5f3fc).into(),
            aura: rgb(0x38bdf8).into(),
        },
        VoiceState::Thinking => OrbPalette {
            core_a: rgb(if dark { 0x121536 } else { 0x1b204a }).into(),
            core_b: rgb(if dark { 0x20103a } else { 0x2c174e }).into(),
            lobe_a: rgb(0x60a5fa).into(),
            lobe_b: rgb(0x8b5cf6).into(),
            lobe_c: rgb(0x2dd4bf).into(),
            rim: rgb(0xddd6fe).into(),
            aura: rgb(0x8b5cf6).into(),
        },
        VoiceState::Speaking => OrbPalette {
            core_a: rgb(if dark { 0x241332 } else { 0x311b44 }).into(),
            core_b: rgb(if dark { 0x141d38 } else { 0x1d284b }).into(),
            lobe_a: rgb(0xfb7185).into(),
            lobe_b: rgb(0xf59e0b).into(),
            lobe_c: rgb(0x818cf8).into(),
            rim: rgb(0xfde68a).into(),
            aura: rgb(0xf472b6).into(),
        },
    }
}

impl Render for VoiceOrb {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let active = self.state != VoiceState::Idle;
        let reduced = disabled_motion(cx);
        let animate = self.visible && self.in_view && w.is_visible() && !reduced;
        let phase_t = if animate { self.clock.age(cx) } else { 0. };
        if animate {
            w.request_animation_frame();
        }
        let motion = orb_motion(self.state, self.muted);
        let level = if self.muted { 0. } else { self.level };
        let voice = level * motion.gain;
        let scale = if animate {
            orb_scale(motion.grow, motion.breath, phase_t, voice)
        } else {
            1. + motion.grow
        };
        let sphere_d = 136. * scale;
        let dark = crate::foundation::Theme::global(cx).mode == crate::foundation::ThemeMode::Dark;
        let accent_candidate = crate::foundation::Theme::global(cx)
            .accent
            .for_mode(crate::foundation::Theme::global(cx).mode);
        let custom_accent = (accent_candidate.a >= 0.01).then_some(accent_candidate);
        let pal = orb_palette(self.state, self.muted, dark, custom_accent);
        let current_state = self.state;
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
            .w(px(522.))
            .max_w_full()
            .items_center()
            .gap(px(14.))
            .p(px(24.))
            .rounded(crate::foundation::surface_radius(cx))
            .border_1()
            .border_color(p.line)
            .bg(p.panel)
            .child(
                div()
                    .h(px(20.))
                    .text_size(px(14.))
                    .line_height(px(20.))
                    .font_weight(FontWeight::MEDIUM)
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
                        canvas(
                            |_, _, _| (),
                            move |bounds, _, w, _| {
                                use std::f32::consts::TAU;
                                let center = bounds.center();
                                let r = sphere_d * 0.46;
                                let flow = phase_t * motion.flow;
                                let amp = motion.wobble + level * 0.045;

                                // 1. Smooth concentric radial aura around the acoustic sphere
                                let aura_gain = motion.halo * if dark { 1.0 } else { 0.72 };
                                for (ring_scale, ring_alpha) in [
                                    (1.50_f32, 0.022_f32),
                                    (1.36, 0.036),
                                    (1.24, 0.055),
                                    (1.13, 0.082),
                                    (1.05, 0.115),
                                ] {
                                    let rr = r * ring_scale;
                                    w.paint_quad(
                                        fill(
                                            Bounds {
                                                origin: point(center.x - px(rr), center.y - px(rr)),
                                                size: size(px(rr * 2.), px(rr * 2.)),
                                            },
                                            linear_gradient(
                                                135. + flow * 24.,
                                                linear_color_stop(
                                                    pal.aura.opacity(ring_alpha * aura_gain),
                                                    0.,
                                                ),
                                                linear_color_stop(
                                                    pal.lobe_a
                                                        .opacity(ring_alpha * aura_gain * 0.75),
                                                    1.,
                                                ),
                                            ),
                                        )
                                        .corner_radii(px(rr)),
                                    );
                                }

                                // Helper for sphere contour radius at angle theta
                                let sphere_r = |theta: f32| -> f32 {
                                    if !animate {
                                        r
                                    } else {
                                        let h1 = (theta * 3. + flow).sin() * amp;
                                        let h2 = (theta * 5. - flow * 0.72).cos() * (amp * 0.45);
                                        let h3 = (theta * 2. + flow * 0.53).sin() * (amp * 0.35);
                                        r * (1. + h1 + h2 + h3)
                                    }
                                };

                                // 2. Deep luminous jewel sphere base
                                let mut body = PathBuilder::fill();
                                const SEGMENTS: usize = 112;
                                for i in 0..SEGMENTS {
                                    let theta = TAU * i as f32 / SEGMENTS as f32;
                                    let rr = sphere_r(theta);
                                    let pt =
                                        center + point(px(theta.cos() * rr), px(theta.sin() * rr));
                                    if i == 0 {
                                        body.move_to(pt);
                                    } else {
                                        body.line_to(pt);
                                    }
                                }
                                body.close();
                                if let Ok(path) = body.build() {
                                    w.paint_path(
                                        path,
                                        linear_gradient(
                                            145.,
                                            linear_color_stop(pal.core_a, 0.),
                                            linear_color_stop(pal.core_b, 1.),
                                        ),
                                    );
                                }

                                // 3. Four organic breathing harmonic lobes / iridescent aurora sheets
                                let lobes = [
                                    (
                                        pal.lobe_a, 0.74_f32, 0.16_f32, -0.14_f32, -0.12_f32,
                                        42.0_f32, 0.62_f32, 1.0_f32,
                                    ),
                                    (pal.lobe_b, 0.70, 0.19, 0.15, 0.11, 168., 0.56, -0.82),
                                    (pal.lobe_c, 0.62, 0.21, -0.08, 0.16, 285., 0.50, 1.28),
                                    (pal.rim, 0.46, 0.14, -0.16, -0.18, 125., 0.34, 0.64),
                                ];
                                for (idx, (tint, base_k, mod_k, ox, oy, grad_deg, peak_a, spd)) in
                                    lobes.into_iter().enumerate()
                                {
                                    let k_phase = if animate {
                                        flow * spd + idx as f32 * 1.57
                                    } else {
                                        idx as f32 * 1.15
                                    };
                                    let drift_x = if animate {
                                        ox + 0.08 * (k_phase * 0.7).cos()
                                    } else {
                                        ox
                                    };
                                    let drift_y = if animate {
                                        oy + 0.08 * (k_phase * 0.6).sin()
                                    } else {
                                        oy
                                    };
                                    let audio_mod = mod_k + level * 0.16;
                                    let mut lobe = PathBuilder::fill();
                                    const LOBE_PTS: usize = 80;
                                    for i in 0..LOBE_PTS {
                                        let theta = TAU * i as f32 / LOBE_PTS as f32;
                                        let wave = (theta * 2. + k_phase).sin() * audio_mod
                                            + (theta * 3. - k_phase * 0.85).cos()
                                                * (audio_mod * 0.52);
                                        let local_r = r * base_k * (1. + wave);
                                        let lx = r * drift_x + theta.cos() * local_r;
                                        let ly = r * drift_y + theta.sin() * local_r;
                                        let dist = lx.hypot(ly);
                                        let max_r = sphere_r(ly.atan2(lx)) * 0.95;
                                        let clamp_k = if dist > max_r && dist > 0.001 {
                                            max_r / dist
                                        } else {
                                            1.
                                        };
                                        let pt = center + point(px(lx * clamp_k), px(ly * clamp_k));
                                        if i == 0 {
                                            lobe.move_to(pt);
                                        } else {
                                            lobe.line_to(pt);
                                        }
                                    }
                                    lobe.close();
                                    if let Ok(path) = lobe.build() {
                                        let rot = if animate {
                                            grad_deg + flow * 28. * spd.signum()
                                        } else {
                                            grad_deg
                                        };
                                        w.paint_path(
                                            path,
                                            linear_gradient(
                                                rot.rem_euclid(360.),
                                                linear_color_stop(tint.opacity(peak_a), 0.),
                                                linear_color_stop(tint.opacity(0.), 0.92),
                                            ),
                                        );
                                    }
                                }

                                // 4. Acoustic harmonic ribbon / orbital sweep inside sphere
                                let ribbon_count = if current_state == VoiceState::Thinking {
                                    3
                                } else {
                                    2
                                };
                                for rb in 0..ribbon_count {
                                    let rb_phase = if animate {
                                        flow * (1.3 + rb as f32 * 0.35) + rb as f32 * 2.1
                                    } else {
                                        0.6 + rb as f32 * 1.8
                                    };
                                    let tilt = 0.32 + rb as f32 * 0.24;
                                    let mut ribbon =
                                        PathBuilder::stroke(px(if rb == 0 { 1.75 } else { 1.2 }));
                                    const RIB_PTS: usize = 72;
                                    for i in 0..=RIB_PTS {
                                        let u = i as f32 / RIB_PTS as f32;
                                        let a = TAU * u;
                                        let x_raw = a.cos() * r * 0.84;
                                        let y_wave =
                                            (a * 2. + rb_phase).sin() * r * (0.18 + level * 0.22)
                                                + a.sin() * r * tilt;
                                        let rot_a = if current_state == VoiceState::Thinking {
                                            rb_phase * 0.65
                                        } else {
                                            -0.28 + rb as f32 * 0.42
                                        };
                                        let rx = x_raw * rot_a.cos() - y_wave * rot_a.sin();
                                        let ry = x_raw * rot_a.sin() + y_wave * rot_a.cos();
                                        let dist = rx.hypot(ry);
                                        let max_r = sphere_r(ry.atan2(rx)) * 0.92;
                                        let k = if dist > max_r && dist > 0.001 {
                                            max_r / dist
                                        } else {
                                            1.
                                        };
                                        let pt = center + point(px(rx * k), px(ry * k));
                                        if i == 0 {
                                            ribbon.move_to(pt);
                                        } else {
                                            ribbon.line_to(pt);
                                        }
                                    }
                                    if let Ok(path) = ribbon.build() {
                                        let rib_c = if rb == 0 { pal.rim } else { pal.lobe_a };
                                        w.paint_path(
                                            path,
                                            linear_gradient(
                                                (90. + rb_phase * 40.).rem_euclid(360.),
                                                linear_color_stop(rib_c.opacity(0.62), 0.),
                                                linear_color_stop(pal.lobe_c.opacity(0.08), 1.),
                                            ),
                                        );
                                    }
                                }

                                // 5. Top-left soft pearlescent specular sheen + crisp Fresnel rim highlight
                                let mut sheen = PathBuilder::fill();
                                for i in 0..64 {
                                    let a = TAU * i as f32 / 64.;
                                    let sx = -r * 0.24 + a.cos() * r * 0.56;
                                    let sy = -r * 0.26 + a.sin() * r * 0.42;
                                    let rot = -0.55_f32;
                                    let rx = sx * rot.cos() - sy * rot.sin();
                                    let ry = sx * rot.sin() + sy * rot.cos();
                                    let dist = rx.hypot(ry);
                                    let max_r = sphere_r(ry.atan2(rx)) * 0.94;
                                    let k = if dist > max_r && dist > 0.001 {
                                        max_r / dist
                                    } else {
                                        1.
                                    };
                                    let pt = center + point(px(rx * k), px(ry * k));
                                    if i == 0 {
                                        sheen.move_to(pt);
                                    } else {
                                        sheen.line_to(pt);
                                    }
                                }
                                sheen.close();
                                if let Ok(path) = sheen.build() {
                                    w.paint_path(
                                        path,
                                        linear_gradient(
                                            145.,
                                            linear_color_stop(white().opacity(0.30), 0.),
                                            linear_color_stop(white().opacity(0.), 0.72),
                                        ),
                                    );
                                }

                                let mut rim_outer = PathBuilder::stroke(px(1.5));
                                let mut rim_inner = PathBuilder::stroke(px(1.));
                                for i in 0..=SEGMENTS {
                                    let theta = TAU * (i % SEGMENTS) as f32 / SEGMENTS as f32;
                                    let rr = sphere_r(theta);
                                    let p_out =
                                        center + point(px(theta.cos() * rr), px(theta.sin() * rr));
                                    let p_in = center
                                        + point(
                                            px(theta.cos() * (rr - 1.5)),
                                            px(theta.sin() * (rr - 1.5)),
                                        );
                                    if i == 0 {
                                        rim_outer.move_to(p_out);
                                        rim_inner.move_to(p_in);
                                    } else {
                                        rim_outer.line_to(p_out);
                                        rim_inner.line_to(p_in);
                                    }
                                }
                                if let Ok(path) = rim_outer.build() {
                                    w.paint_path(
                                        path,
                                        linear_gradient(
                                            140.,
                                            linear_color_stop(white().opacity(0.75), 0.),
                                            linear_color_stop(pal.lobe_b.opacity(0.32), 1.),
                                        ),
                                    );
                                }
                                if let Ok(path) = rim_inner.build() {
                                    w.paint_path(
                                        path,
                                        linear_gradient(
                                            320.,
                                            linear_color_stop(pal.rim.opacity(0.38), 0.),
                                            linear_color_stop(white().opacity(0.), 0.7),
                                        ),
                                    );
                                }
                            },
                        )
                        .size(px(208.)),
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
                    .child(if !active && self.transcript.is_empty() {
                        SharedString::from(tr(
                            cx,
                            "Tap the orb to start a voice session.",
                            "轻触光球，开始语音对话。",
                        ))
                    } else {
                        self.transcript.clone()
                    }),
            );
        let radius: f32 = crate::foundation::surface_radius(cx).into();
        let controls = if active {
            let mute_label = if self.muted {
                tr(cx, "Unmute", "取消静音")
            } else {
                tr(cx, "Mute", "静音")
            };
            div()
                .h(px(48.))
                .px(px(5.))
                .gap(px(6.))
                .flex()
                .items_center()
                .border_1()
                .border_color(p.line)
                .rounded(px(radius))
                .bg(p.panel)
                .shadow_sm()
                .child(
                    face("voice-mute", mute_label, 38., radius, Tone::Clear, cx)
                        .px(px(14.))
                        .when(self.muted, |b| b.bg(p.ink).text_color(p.inverse))
                        .when(!self.muted, |b| b.hover(|s| s.bg(p.tray)))
                        .child(mute_label)
                        .on_click(cx.listener(|_, _, _, cx| cx.emit(VoiceOrbEvent::ToggleMute))),
                )
                .child(
                    div()
                        .min_w(px(42.))
                        .px(px(8.))
                        .text_center()
                        .text_size(px(14.))
                        .line_height(px(20.))
                        .font_features(FontFeatures(vec![("tnum".into(), 1)].into()))
                        .text_color(p.secondary)
                        .child(format!("{}:{:02}", self.elapsed / 60, self.elapsed % 60)),
                )
                .child(
                    face(
                        "voice-session",
                        tr(cx, "End", "结束"),
                        38.,
                        radius,
                        Tone::Clear,
                        cx,
                    )
                    .px(px(14.))
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
                .rounded(px(radius))
                .bg(p.panel)
                .shadow_sm()
                .child(
                    face(
                        "voice-session",
                        tr(cx, "Start voice chat", "开始语音对话"),
                        46.,
                        radius,
                        Tone::Clear,
                        cx,
                    )
                    .pl(px(16.))
                    .pr(px(20.))
                    .gap(px(8.))
                    .hover(|s| s.bg(p.tray))
                    .child(icon(Glyph::Mic, 16., p.ink))
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
    use super::{VoiceOrb, VoiceState, orb_motion, orb_palette, orb_scale};
    use gpui::TestAppContext;

    #[test]
    fn scale_follows_the_source_breath_and_voice() {
        assert!((orb_scale(0., 0.028, 0., 0.) - 1.).abs() < 0.0001);
        assert!((orb_scale(0.02, 0., 0., 0.19) - (1.02 + 0.171)).abs() < 0.0001);
        assert!((orb_scale(-0.08, 0., 0., 0.) - 0.92).abs() < 0.0001);
    }

    #[test]
    fn states_and_reduced_motion_render_cleanly() {
        for state in [
            VoiceState::Idle,
            VoiceState::Listening,
            VoiceState::Thinking,
            VoiceState::Speaking,
        ] {
            let m = orb_motion(state, false);
            assert!(m.halo > 0.);
            let pal = orb_palette(state, false, true, None);
            assert!(pal.lobe_a.a > 0.9 && pal.rim.a > 0.9);
        }
        let muted_pal = orb_palette(VoiceState::Listening, true, false, None);
        assert!(muted_pal.lobe_a.a > 0.9);

        let mut app = TestAppContext::single();
        app.update(|cx| {
            crate::init(cx);
            crate::foundation::Theme::global_mut(cx).reduced_motion = true;
        });
        let (view, w) = app.add_window_view(|_, cx| VoiceOrb::new(cx));
        w.update(|_, cx| {
            view.update(cx, |orb, cx| {
                orb.set_state(VoiceState::Speaking, cx);
                orb.set_level(0.75, cx);
            });
        });
        w.run_until_parked();
    }
}
