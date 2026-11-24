//! Dictation panel with a quiet waveform well, ticking timer, shortcut caps, and under-glow.
//!
//! Renders a `468px` floating voice-capture card fading softly on the right, housing
//! 68 pixel-snapped vertical loudness bars in a recessed quiet well, a breathing
//! recording indicator with live `m:ss` clock, interactive pause/confirm/cancel
//! controls, and a drifting blue-purple-pink glow underneath.

use crate::{
    components::pro_chrome::{ProPalette, pro_eid, pro_empty_state, pro_skeleton_bar, tr},
    foundation::Theme,
    primitives::{
        control::UiLanguage,
        pro_icons::{ProIcon, pro_icon},
    },
};
use gpui::{prelude::*, *};
use std::sync::{Arc, OnceLock};
use web_time::Instant;

/// Default 68-bar speech rhythm profile (`<= 3` represents quiet breath pauses).
pub const DEFAULT_VOICE_PROFILE: [u8; 68] = [
    3, 6, 2, 10, 5, 13, 2, 2, 2, 5, 8, 11, 6, 16, 10, 5, 2, 2, 3, 2, 8, 14, 19, 13, 8, 18, 11, 5,
    2, 2, 2, 6, 10, 6, 13, 8, 3, 2, 2, 2, 2, 5, 11, 16, 21, 14, 10, 6, 11, 8, 3, 2, 2, 5, 8, 5, 2,
    2, 6, 13, 10, 16, 11, 6, 3, 2, 2, 2,
];

/// Events emitted by [`VoiceCapture`] when the user interacts with its dictation controls.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VoiceCaptureEvent {
    /// User toggled pause/resume on the recording indicator (`true` when paused).
    Paused(bool),
    /// User confirmed dictation via `Done` (or `⌥ Space` / `Enter`).
    Confirm {
        /// Elapsed recording duration in whole seconds at confirmation time.
        elapsed_seconds: u32,
    },
    /// User cancelled/stopped dictation via `Cancel` (or `Esc`).
    Cancel,
}

impl EventEmitter<VoiceCaptureEvent> for VoiceCapture {}

/// Format elapsed seconds as tabular `"m:ss"` (for example `14 -> "0:14"`, `75 -> "1:15"`).
pub fn format_voice_timer(seconds: u32) -> String {
    let mins = seconds / 60;
    let secs = seconds % 60;
    format!("{mins}:{secs:02}")
}

/// Evaluate neighbor-staggered harmonic modulation (`0.30..=1.10`) for waveform bar `idx` at `elapsed_ms`.
fn bar_harmonic(elapsed_ms: f32, idx: usize) -> f32 {
    let period = (640 + ((idx * 53) % 5) * 110) as f32;
    let n = ((elapsed_ms + (idx as f32) * 70.0).rem_euclid(period)) / period;
    let tau = std::f32::consts::TAU;
    (0.66 + 0.36 * (tau * n).sin() + 0.12 * (2.0 * tau * n + 1.0).sin()).clamp(0.30, 1.10)
}

/// Compute rendered height (in logical px) for bar `idx` with base weight `weight`.
fn bar_height_px(weight: u8, idx: usize, moving: bool, elapsed_ms: f32, loudness: f32) -> f32 {
    let min_dot = 2.0f32;
    if weight <= 3 {
        return min_dot;
    }
    let wave = if moving {
        bar_harmonic(elapsed_ms, idx)
    } else {
        bar_harmonic(0.0, idx)
    };
    let gain = loudness.clamp(0.0, 2.0);
    if gain <= 0.02 {
        return min_dot;
    }
    let raw = (weight as f32) * 1.6 * wave * gain;
    // Snap to whole pixels with even symmetry around center for crisp rendering
    let rounded = raw.round().max(min_dot);
    let parity_adj = if ((rounded as i32) - 2) % 2 != 0 {
        rounded + 1.0
    } else {
        rounded
    };
    parity_adj.clamp(min_dot, 62.0)
}

fn voice_under_glow_svg(dark: bool) -> Arc<Image> {
    static LIGHT_GLOW: OnceLock<Arc<Image>> = OnceLock::new();
    static DARK_GLOW: OnceLock<Arc<Image>> = OnceLock::new();
    let cache = if dark { &DARK_GLOW } else { &LIGHT_GLOW };
    cache
        .get_or_init(|| {
            let (blue, purple, pink) = if dark {
                ("#4fb0fe", "#b98af0", "#f06db5")
            } else {
                ("#0190fe", "#a651f0", "#ee3e9e")
            };
            let doc = format!(
                r##"<svg xmlns="http://www.w3.org/2000/svg" width="580" height="160" viewBox="0 0 290 80" fill="none"><defs><filter id="vcpg" x="-30%" y="-60%" width="160%" height="220%"><feGaussianBlur stdDeviation="15"/></filter></defs><g filter="url(#vcpg)"><ellipse cx="58" cy="34" rx="48" ry="15" fill="{blue}" fill-opacity="0.72"/><ellipse cx="148" cy="32" rx="50" ry="15" fill="{purple}" fill-opacity="0.68"/><ellipse cx="232" cy="30" rx="46" ry="14" fill="{pink}" fill-opacity="0.64"/></g></svg>"##
            );
            Arc::new(Image::from_bytes(ImageFormat::Svg, doc.into_bytes()))
        })
        .clone()
}

/// Paint the `20px`-rounded voice capture panel with a smooth right-edge alpha fade.
fn paint_fading_panel(panel_bg: Hsla, border_col: Hsla, shadow_col: Hsla) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            let bw = f32::from(bounds.size.width);
            let bh = f32::from(bounds.size.height);
            if bw <= 40.0 || bh <= 20.0 {
                return;
            }
            let r = 20.0f32.min(bh * 0.5);
            let fade_w = 132.0f32.min(bw * 0.34);
            let solid_w = (bw - fade_w).max(r + 12.0);
            let x0 = f32::from(bounds.left());
            let y0 = f32::from(bounds.top());

            // Soft elevation shadow under solid portion
            window.paint_quad(quad(
                Bounds::new(
                    point(px(x0 + 2.0), px(y0 + 6.0)),
                    size(px(solid_w), px((bh - 2.0).max(1.0))),
                ),
                Corners {
                    top_left: px(r),
                    bottom_left: px(r),
                    top_right: px(0.0),
                    bottom_right: px(0.0),
                },
                shadow_col,
                Edges::all(px(0.0)),
                transparent_black(),
                BorderStyle::Solid,
            ));

            // Solid left panel body
            window.paint_quad(quad(
                Bounds::new(point(px(x0), px(y0)), size(px(solid_w), px(bh))),
                Corners {
                    top_left: px(r),
                    bottom_left: px(r),
                    top_right: px(0.0),
                    bottom_right: px(0.0),
                },
                panel_bg,
                Edges {
                    top: px(1.0),
                    bottom: px(1.0),
                    left: px(1.0),
                    right: px(0.0),
                },
                border_col,
                BorderStyle::Solid,
            ));

            // Right horizontal fade-out tail
            let slices = 28usize;
            let slice_w = fade_w / slices as f32;
            for i in 0..slices {
                let u = (i as f32 + 0.5) / slices as f32;
                let fade = (1.0 - u) * (1.0 - u);
                if fade <= 0.004 {
                    continue;
                }
                let sx = x0 + solid_w + (i as f32) * slice_w;
                let sw = slice_w + 0.4;
                window.paint_quad(fill(
                    Bounds::new(
                        point(px(sx), px(y0 + 6.0)),
                        size(px(sw), px((bh - 2.0).max(1.0))),
                    ),
                    shadow_col.opacity(shadow_col.a * fade),
                ));
                window.paint_quad(quad(
                    Bounds::new(point(px(sx), px(y0)), size(px(sw), px(bh))),
                    Corners::all(px(0.0)),
                    panel_bg.opacity(panel_bg.a * fade),
                    Edges {
                        top: px(1.0),
                        bottom: px(1.0),
                        left: px(0.0),
                        right: px(0.0),
                    },
                    border_col.opacity(border_col.a * fade),
                    BorderStyle::Solid,
                ));
            }
        },
    )
    .absolute()
    .inset_0()
}

fn render_kbd_cap(label: impl Into<SharedString>, bg: Hsla, fg: Hsla) -> Div {
    div()
        .min_w(px(16.0))
        .h(px(16.0))
        .px(px(4.5))
        .rounded(px(4.0))
        .bg(bg)
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(10.5))
        .line_height(px(12.0))
        .font_weight(FontWeight::MEDIUM)
        .text_color(fg)
        .child(label.into())
}

/// Product-scene dictation panel with animated vertical voice bars, live clock, and under-glow.
///
/// # Examples
///
/// ```no_run
/// use gpui::prelude::*;
/// use kui::components::voice_capture::VoiceCapture;
///
/// # fn example(cx: &mut gpui::App) {
/// let _capture = cx.new(|cx| VoiceCapture::new("hero-dictation", cx).with_start_seconds(14));
/// # }
/// ```
pub struct VoiceCapture {
    id: ElementId,
    focus: FocusHandle,
    playing: bool,
    paused: bool,
    start_seconds: u32,
    loudness: f32,
    bars: Vec<u8>,
    rec_hovered: bool,
    done_hovered: bool,
    cancel_hovered: bool,
    confirmed_flash: Option<Instant>,
    loading: bool,
    empty: bool,
    started_at: Instant,
    accumulated_ms: u64,
    segment_started_at: Instant,
}

impl VoiceCapture {
    /// Create a [`VoiceCapture`] panel starting at `0:14` with live voice rhythm and under-glow.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<UiLanguage>(|_, cx| cx.notify())
            .detach();
        let now = Instant::now();
        Self {
            id: id.into(),
            focus: cx.focus_handle().tab_stop(true),
            playing: true,
            paused: false,
            start_seconds: 14,
            loudness: 1.0,
            bars: DEFAULT_VOICE_PROFILE.to_vec(),
            rec_hovered: false,
            done_hovered: false,
            cancel_hovered: false,
            confirmed_flash: None,
            loading: false,
            empty: false,
            started_at: now,
            accumulated_ms: 0,
            segment_started_at: now,
        }
    }

    /// Control whether the waveform, glow, and clock animate (`true` by default; `false` holds a still frame).
    pub fn with_playing(mut self, playing: bool) -> Self {
        self.playing = playing;
        self
    }

    /// Set the starting timer offset in seconds (`14` -> `"0:14"` by default).
    pub fn with_start_seconds(mut self, seconds: u32) -> Self {
        self.start_seconds = seconds;
        self
    }

    /// Alias for [`Self::with_start_seconds`] matching the `start` prop.
    pub fn with_start(self, seconds: u32) -> Self {
        self.with_start_seconds(seconds)
    }

    /// Scale the simulated or host-supplied voice loudness (`1.0` default, `0.0..=2.0`).
    pub fn with_loudness(mut self, loudness: f32) -> Self {
        if loudness.is_finite() {
            self.loudness = loudness.clamp(0.0, 2.0);
        }
        self
    }

    /// Replace the bar rhythm profile (`<= 3` renders quiet `2×2px` pause dots).
    pub fn with_bars(mut self, bars: Vec<u8>) -> Self {
        if !bars.is_empty() {
            self.bars = bars;
        }
        self
    }

    /// Configure the skeleton loading state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Configure the empty state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Return the current displayed timer value in whole seconds.
    pub fn current_seconds(&self, reduced_motion: bool) -> u32 {
        if !self.playing || reduced_motion {
            return self.start_seconds + (self.accumulated_ms / 1000) as u32;
        }
        let extra_ms = if self.paused {
            self.accumulated_ms
        } else {
            self.accumulated_ms + self.segment_started_at.elapsed().as_millis() as u64
        };
        self.start_seconds + (extra_ms / 1000) as u32
    }

    /// Return whether recording is currently paused by the user.
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// Toggle pause/resume on the dictation session.
    pub fn toggle_pause(&mut self, cx: &mut Context<Self>) {
        if !self.paused {
            self.accumulated_ms += self.segment_started_at.elapsed().as_millis() as u64;
            self.paused = true;
        } else {
            self.segment_started_at = Instant::now();
            self.paused = false;
        }
        cx.emit(VoiceCaptureEvent::Paused(self.paused));
        cx.notify();
    }

    /// Confirm the dictation capture (`Done`), emitting [`VoiceCaptureEvent::Confirm`].
    pub fn confirm(&mut self, cx: &mut Context<Self>) {
        let reduced = Theme::global(cx).reduced_motion;
        let secs = self.current_seconds(reduced);
        self.confirmed_flash = Some(Instant::now());
        cx.emit(VoiceCaptureEvent::Confirm {
            elapsed_seconds: secs,
        });
        cx.notify();
    }

    /// Cancel/reset the dictation capture (`Cancel`), resetting the timer to `start_seconds`.
    pub fn cancel(&mut self, cx: &mut Context<Self>) {
        self.accumulated_ms = 0;
        self.segment_started_at = Instant::now();
        self.paused = false;
        self.confirmed_flash = None;
        cx.emit(VoiceCaptureEvent::Cancel);
        cx.notify();
    }

    /// Update the live loudness multiplier (`0.0..=2.0`) from host audio metering at runtime.
    pub fn set_loudness(&mut self, loudness: f32, cx: &mut Context<Self>) {
        if loudness.is_finite() {
            self.loudness = loudness.clamp(0.0, 2.0);
            cx.notify();
        }
    }

    /// Return the current loudness multiplier.
    pub fn loudness(&self) -> f32 {
        self.loudness
    }

    /// Toggle skeleton loading mode at runtime.
    pub fn set_loading(&mut self, loading: bool, cx: &mut Context<Self>) {
        if self.loading != loading {
            self.loading = loading;
            cx.notify();
        }
    }

    /// Return whether skeleton loading mode is active.
    pub fn is_loading(&self) -> bool {
        self.loading
    }

    /// Toggle the empty state at runtime.
    pub fn set_empty(&mut self, empty: bool, cx: &mut Context<Self>) {
        if self.empty != empty {
            self.empty = empty;
            cx.notify();
        }
    }

    /// Return whether the empty state is active.
    pub fn is_empty(&self) -> bool {
        self.empty
    }
}

impl Render for VoiceCapture {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let reduced = Theme::global(cx).reduced_motion;
        let total_elapsed_s = self.started_at.elapsed().as_secs_f32();

        if self.loading {
            let phase = (total_elapsed_s / 1.4).fract();
            if !reduced {
                w.request_animation_frame();
            }
            return div()
                .id(self.id.clone())
                .w_full()
                .py(px(32.0))
                .flex()
                .items_center()
                .justify_center()
                .child(pro_skeleton_bar(380.0, 124.0, 20.0, phase, cx))
                .into_any_element();
        }

        if self.empty {
            return div()
                .id(self.id.clone())
                .w_full()
                .child(pro_empty_state(
                    ProIcon::Mic,
                    tr(cx, "Microphone idle", "麦克风空闲"),
                    tr(
                        cx,
                        "Start voice dictation to visualize live speech loudness.",
                        "开始语音听写以展示实时音量波形。",
                    ),
                    None,
                    cx,
                ))
                .into_any_element();
        }

        let moving = self.playing && !self.paused && !reduced;
        let flash_active = self
            .confirmed_flash
            .is_some_and(|t| t.elapsed().as_secs_f32() < 1.1);
        if !flash_active {
            self.confirmed_flash = None;
        }
        if moving || flash_active {
            w.request_animation_frame();
        }

        let active_ms = if !self.playing || reduced {
            0.0
        } else if self.paused {
            self.accumulated_ms as f32
        } else {
            (self.accumulated_ms as f32) + self.segment_started_at.elapsed().as_secs_f32() * 1000.0
        };
        let shown_secs = self.current_seconds(reduced);
        let timer_text = format_voice_timer(shown_secs);

        let dark = p.dark;
        let panel_bg: Hsla = if dark {
            rgb(0x161618).into()
        } else {
            rgb(0xffffff).into()
        };
        let border_col: Hsla = if dark {
            hsla(0.0, 0.0, 1.0, 0.10)
        } else {
            hsla(0.0, 0.0, 0.0, 0.08)
        };
        let shadow_col: Hsla = if dark {
            hsla(0.0, 0.0, 0.0, 0.44)
        } else {
            hsla(0.0, 0.0, 0.0, 0.055)
        };
        let well_bg: Hsla = if dark {
            hsla(0.0, 0.0, 1.0, 0.035)
        } else {
            hsla(0.0, 0.0, 0.0, 0.03)
        };
        let bar_col: Hsla = if dark {
            hsla(0.0, 0.0, 1.0, 0.80)
        } else {
            hsla(0.0, 0.0, 0.0, 0.72)
        };
        let ink_col: Hsla = if dark {
            hsla(0.0, 0.0, 1.0, 0.86)
        } else {
            hsla(0.0, 0.0, 0.0, 0.82)
        };
        let muted_col: Hsla = if dark {
            hsla(0.0, 0.0, 1.0, 0.40)
        } else {
            hsla(0.0, 0.0, 0.0, 0.40)
        };
        let key_bg: Hsla = if dark {
            hsla(0.0, 0.0, 1.0, 0.06)
        } else {
            hsla(0.0, 0.0, 0.0, 0.045)
        };
        let sep_col: Hsla = if dark {
            hsla(0.0, 0.0, 1.0, 0.10)
        } else {
            hsla(0.0, 0.0, 0.0, 0.08)
        };
        let rec_red: Hsla = rgb(0xe5484d).into();

        // Recessed quiet well + 68 pixel-snapped vertical waveform bars (fading cleanly on the right)
        let bars_snapshot = self.bars.clone();
        let loudness = if self.paused { 0.22 } else { self.loudness };
        let well_canvas = canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let bw = f32::from(bounds.size.width);
                let bh = f32::from(bounds.size.height);
                if bw <= 20.0 || bh <= 10.0 {
                    return;
                }
                let r = 14.0f32.min(bh * 0.5);
                let fade_w = 126.0f32.min(bw * 0.34);
                let solid_w = (bw - fade_w).max(r + 8.0);
                let x0 = f32::from(bounds.left());
                let y0 = f32::from(bounds.top());

                // Quiet well solid left region + faded right tail
                window.paint_quad(quad(
                    Bounds::new(point(px(x0), px(y0)), size(px(solid_w), px(bh))),
                    Corners {
                        top_left: px(r),
                        bottom_left: px(r),
                        top_right: px(0.0),
                        bottom_right: px(0.0),
                    },
                    well_bg,
                    Edges::all(px(0.0)),
                    transparent_black(),
                    BorderStyle::Solid,
                ));
                let slices = 20usize;
                let slice_w = fade_w / slices as f32;
                for i in 0..slices {
                    let u = (i as f32 + 0.5) / slices as f32;
                    let fade = (1.0 - u) * (1.0 - u);
                    if fade <= 0.005 {
                        continue;
                    }
                    window.paint_quad(fill(
                        Bounds::new(
                            point(px(x0 + solid_w + (i as f32) * slice_w), px(y0)),
                            size(px(slice_w + 0.4), px(bh)),
                        ),
                        well_bg.opacity(well_bg.a * fade),
                    ));
                }

                // 68 vertical waveform bars (`2px` wide, `5px` pitch starting at `20px`)
                let bar_w = 2.0f32;
                let pitch = 5.0f32;
                let start_x = 20.0f32;
                for (idx, &weight) in bars_snapshot.iter().enumerate() {
                    let bx = start_x + (idx as f32) * pitch;
                    if bx + bar_w > bw - 6.0 {
                        break;
                    }
                    let right_fade = if bx <= solid_w {
                        1.0
                    } else {
                        let u = ((bx - solid_w) / fade_w).clamp(0.0, 1.0);
                        (1.0 - u) * (1.0 - u)
                    };
                    if right_fade <= 0.02 {
                        continue;
                    }
                    let h = bar_height_px(weight, idx, moving, active_ms, loudness).min(bh - 12.0);
                    let by = ((bh - h) * 0.5).round();
                    window.paint_quad(
                        fill(
                            Bounds::new(point(px(x0 + bx), px(y0 + by)), size(px(bar_w), px(h))),
                            bar_col.opacity(bar_col.a * right_fade),
                        )
                        .corner_radii(px(1.0)),
                    );
                }
            },
        )
        .w_full()
        .h(px(80.0));

        // Breathing red recording dot (`1.6s` cycle: opacity 1.0 <-> 0.35)
        let dot_alpha = if self.paused {
            0.85
        } else if moving {
            let phase = (total_elapsed_s / 1.6) * std::f32::consts::TAU;
            0.675 + 0.325 * phase.cos()
        } else {
            1.0
        };
        let dot_color = if self.paused { p.amber } else { rec_red };

        let rec_indicator = if self.paused {
            pro_icon(ProIcon::Play, 10.0, dot_color)
        } else if self.rec_hovered {
            pro_icon(ProIcon::Pause, 10.0, rec_red)
        } else {
            div()
                .size(px(6.0))
                .rounded_full()
                .bg(dot_color.opacity(dot_alpha))
                .into_any_element()
        };

        let rec_btn = div()
            .id(pro_eid(&self.id, "rec-toggle"))
            .role(Role::Button)
            .aria_label(if self.paused {
                tr(cx, "Resume dictation", "继续听写")
            } else {
                tr(cx, "Pause dictation", "暂停听写")
            })
            .min_w(px(44.0))
            .h(px(22.0))
            .px(px(4.0))
            .ml(px(-4.0))
            .rounded(px(6.0))
            .when(self.rec_hovered, |d| d.bg(p.hover))
            .flex()
            .items_center()
            .gap(px(6.0))
            .cursor_pointer()
            .on_hover(cx.listener(|this, inside: &bool, _, cx| {
                if this.rec_hovered != *inside {
                    this.rec_hovered = *inside;
                    cx.notify();
                }
            }))
            .on_click(cx.listener(|this, _, _, cx| {
                this.toggle_pause(cx);
            }))
            .child(
                div()
                    .w(px(8.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(rec_indicator),
            )
            .child(
                div()
                    .text_size(px(12.0))
                    .line_height(px(16.0))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(ink_col)
                    .child(timer_text),
            );

        let done_label = if flash_active {
            tr(cx, "Saved", "已保存")
        } else {
            tr(cx, "Done", "完成")
        };

        let done_btn = div()
            .id(pro_eid(&self.id, "done-btn"))
            .role(Role::Button)
            .aria_label(tr(cx, "Done", "完成"))
            .h(px(24.0))
            .px(px(5.0))
            .mx(px(-5.0))
            .rounded(px(6.0))
            .when(self.done_hovered, |d| d.bg(p.hover))
            .flex()
            .items_center()
            .gap(px(4.0))
            .cursor_pointer()
            .on_hover(cx.listener(|this, inside: &bool, _, cx| {
                if this.done_hovered != *inside {
                    this.done_hovered = *inside;
                    cx.notify();
                }
            }))
            .on_click(cx.listener(|this, _, _, cx| {
                this.confirm(cx);
            }))
            .child(
                div()
                    .text_size(px(12.0))
                    .line_height(px(16.0))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(if flash_active { p.green } else { ink_col })
                    .child(done_label),
            )
            .child(
                div()
                    .ml(px(2.0))
                    .child(render_kbd_cap("⌥", key_bg, muted_col)),
            )
            .child(render_kbd_cap("Space", key_bg, muted_col));

        let cancel_btn = div()
            .id(pro_eid(&self.id, "cancel-btn"))
            .role(Role::Button)
            .aria_label(tr(cx, "Cancel", "取消"))
            .h(px(24.0))
            .px(px(5.0))
            .mx(px(-5.0))
            .rounded(px(6.0))
            .when(self.cancel_hovered, |d| d.bg(p.hover))
            .flex()
            .items_center()
            .gap(px(4.0))
            .cursor_pointer()
            .on_hover(cx.listener(|this, inside: &bool, _, cx| {
                if this.cancel_hovered != *inside {
                    this.cancel_hovered = *inside;
                    cx.notify();
                }
            }))
            .on_click(cx.listener(|this, _, _, cx| {
                this.cancel(cx);
            }))
            .child(
                div()
                    .text_size(px(12.0))
                    .line_height(px(16.0))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(if self.cancel_hovered {
                        ink_col
                    } else {
                        muted_col
                    })
                    .child(tr(cx, "Cancel", "取消")),
            )
            .child(
                div()
                    .ml(px(2.0))
                    .child(render_kbd_cap("Esc", key_bg, muted_col)),
            );

        let footer = div()
            .relative()
            .h(px(36.0))
            .px(px(10.0))
            .flex()
            .items_center()
            .gap(px(12.0))
            .whitespace_nowrap()
            .child(rec_btn)
            .child(div().w(px(1.0)).h(px(12.0)).bg(sep_col))
            .child(done_btn)
            .child(cancel_btn);

        let panel = div()
            .relative()
            .w(px(468.0))
            .pt(px(6.0))
            .pl(px(6.0))
            .pr(px(6.0))
            .pb(px(2.0))
            .flex()
            .flex_col()
            .gap(px(2.0))
            .child(paint_fading_panel(panel_bg, border_col, shadow_col))
            .child(div().relative().w_full().child(well_canvas))
            .child(footer);

        let drift_x = if moving {
            6.0 - 6.0 * ((total_elapsed_s / 9.0) * std::f32::consts::PI).cos()
        } else {
            0.0
        };
        let glow_img = voice_under_glow_svg(dark);
        let glow_opacity: f32 = (if dark { 0.50_f32 } else { 0.68_f32 }
            * if self.paused { 0.55_f32 } else { 1.0_f32 })
        .clamp(0.15, 0.85);

        let stack = div()
            .relative()
            .w(px(468.0))
            .pt(px(20.0))
            .pb(px(52.0))
            .child(
                div()
                    .absolute()
                    .left(px(8.0 + drift_x))
                    .top(px(114.0))
                    .w(px(376.0))
                    .h(px(68.0))
                    .opacity(glow_opacity)
                    .child(img(glow_img).w(px(376.0)).h(px(68.0))),
            )
            .child(panel);

        div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _, cx| {
                match ev.keystroke.key.as_str() {
                    "escape" => this.cancel(cx),
                    "space" if ev.keystroke.modifiers.alt => this.confirm(cx),
                    "enter" => this.confirm(cx),
                    "space" => this.toggle_pause(cx),
                    _ => {}
                }
            }))
            .w_full()
            .min_h(px(210.0))
            .flex()
            .items_center()
            .justify_center()
            .overflow_hidden()
            .child(stack)
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{DEFAULT_VOICE_PROFILE, bar_harmonic, bar_height_px, format_voice_timer};

    #[test]
    fn voice_timer_and_pause_dots_match_dictation_spec() {
        assert_eq!(format_voice_timer(7), "0:07");
        assert_eq!(format_voice_timer(14), "0:14");
        assert_eq!(format_voice_timer(65), "1:05");
        assert_eq!(DEFAULT_VOICE_PROFILE.len(), 68);

        // Quiet breath pauses (`<= 3`) stay crisp 2px dots regardless of phase or loudness
        for (idx, &w) in DEFAULT_VOICE_PROFILE.iter().enumerate() {
            if w <= 3 {
                assert_eq!(bar_height_px(w, idx, true, 420.0, 1.5), 2.0);
            } else {
                let h_norm = bar_height_px(w, idx, true, 420.0, 1.0);
                let h_loud = bar_height_px(w, idx, true, 420.0, 1.8);
                assert!(h_norm >= 2.0);
                assert!(h_loud >= h_norm);
                assert_eq!(bar_height_px(w, idx, true, 420.0, 0.0), 2.0);
            }
        }
    }

    #[test]
    fn neighbor_bars_oscillate_within_bounded_envelope_out_of_step() {
        let a = bar_harmonic(250.0, 4);
        let b = bar_harmonic(250.0, 5);
        assert!((0.30..=1.10).contains(&a));
        assert!((0.30..=1.10).contains(&b));
        assert!((a - b).abs() > 1e-3);
    }
}
