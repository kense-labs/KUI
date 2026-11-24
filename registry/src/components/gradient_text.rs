//! Headline words cut from a flowing four-blob liquid gradient with an entrance sweep.
//!
//! Shapes text with GPUI's native font engine so kerning, weight, and baseline match
//! surrounding headline copy, then paints clipped liquid-gradient slices across the
//! glyphs and wipes color in from left to right on first view.

use crate::{
    foundation::{Theme, ThemeMode},
    primitives::control::UiLanguage,
};
use gpui::{prelude::*, *};
use std::{cell::Cell, f32::consts::PI, rc::Rc};
use web_time::Instant;

const SWEEP_SECONDS: f32 = 1.10;

const LIGHT_DEFAULTS: [u32; 4] = [0xdd0674, 0x5e3ee0, 0xff5fae, 0x7c5cff];
const DARK_DEFAULTS: [u32; 4] = [0xf472b6, 0xa78bfa, 0xf9a8d4, 0xc4b5fd];

/// Keyframes `(px, py)` for the three orbiting radial blobs at `0%`, `25%`, `50%`, `75%`.
const BLOB_A_KEYS: [(f32, f32); 4] = [(-0.20, 0.30), (0.40, 1.10), (1.20, 0.40), (0.60, -0.30)];
const BLOB_B_KEYS: [(f32, f32); 4] = [(1.20, 0.70), (0.60, -0.10), (-0.20, 0.40), (0.30, 1.20)];
const BLOB_C_KEYS: [(f32, f32); 4] = [(0.50, -0.40), (1.10, 0.60), (0.40, 1.40), (-0.10, 0.30)];

fn parse_hex_rgb(hex: &str) -> Option<Hsla> {
    let s = hex.trim().trim_start_matches('#');
    let full = match s.len() {
        3 => {
            let mut out = String::with_capacity(6);
            for ch in s.chars() {
                out.push(ch);
                out.push(ch);
            }
            out
        }
        6 => s.to_owned(),
        _ => return None,
    };
    let val = u32::from_str_radix(&full, 16).ok()?;
    Some(rgb(val).into())
}

fn hsla_to_rgb_f32(color: Hsla) -> [f32; 3] {
    let c = Rgba::from(color);
    [c.r.clamp(0., 1.), c.g.clamp(0., 1.), c.b.clamp(0., 1.)]
}

/// Evaluate `cubic-bezier(0.65, 0.0, 0.25, 1.0)` on `t` in `0.0..=1.0`.
fn gt_sweep_ease(t: f32) -> f32 {
    let t = t.clamp(0., 1.);
    if t <= 0. || t >= 1. {
        return t;
    }
    let mut u = t;
    for _ in 0..7 {
        let inv = 1. - u;
        let x = 3. * inv * inv * u * 0.65 + 3. * inv * u * u * 0.25 + u * u * u;
        let dx = 3. * inv * inv * 0.65 + 6. * inv * u * (0.25 - 0.65) + 3. * u * u * 0.75;
        if dx.abs() < 1e-5 {
            break;
        }
        u = (u - (x - t) / dx).clamp(0., 1.);
    }
    u * u * (3. - 2. * u)
}

fn interp_loop_keys(keys: &[(f32, f32); 4], phase: f32) -> (f32, f32) {
    let p = phase.rem_euclid(1.) * 4.;
    let seg = (p.floor() as usize) % 4;
    let local = p - p.floor();
    let s = (1. - (local * PI).cos()) * 0.5;
    let a = keys[seg];
    let b = keys[(seg + 1) % 4];
    (a.0 + (b.0 - a.0) * s, a.1 + (b.1 - a.1) * s)
}

fn blend_radial_blob(
    acc: [f32; 3],
    col: [f32; 3],
    size_wh: (f32, f32),
    pos_pct: (f32, f32),
    uv: (f32, f32),
) -> [f32; 3] {
    let (sw, sh) = size_wh;
    let cx = pos_pct.0 * (1. - sw) + 0.5 * sw;
    let cy = pos_pct.1 * (1. - sh) + 0.5 * sh;
    let rx = (0.5 * sw).max(0.05);
    let ry = (0.5 * sh).max(0.05);
    let dx = (uv.0 - cx) / rx;
    let dy = (uv.1 - cy) / ry;
    let dist = (dx * dx + dy * dy).sqrt();
    if dist >= 1. {
        return acc;
    }
    let falloff = (1. - dist).clamp(0., 1.);
    let alpha = falloff * falloff * (3. - 2. * falloff);
    [
        acc[0] * (1. - alpha) + col[0] * alpha,
        acc[1] * (1. - alpha) + col[1] * alpha,
        acc[2] * (1. - alpha) + col[2] * alpha,
    ]
}

/// Evaluate the four-layer liquid gradient (`100deg` linear base + 3 orbiting radial blobs).
fn sample_liquid_gradient(u: f32, v: f32, phase: f32, stops: [[f32; 3]; 4]) -> Hsla {
    let [c1, c2, c3, c4] = stops;
    // 100deg CSS linear-gradient(100deg, c1 10%, c2 90%)
    let proj = 0.5 + ((u - 0.5) * 0.9848 + (v - 0.5) * 0.1736) / 1.1584;
    let t_lin = ((proj - 0.10) / 0.80).clamp(0., 1.);
    let mut rgb_out = [
        c1[0] + (c2[0] - c1[0]) * t_lin,
        c1[1] + (c2[1] - c1[1]) * t_lin,
        c1[2] + (c2[2] - c1[2]) * t_lin,
    ];

    let pos_c = interp_loop_keys(&BLOB_C_KEYS, phase);
    let pos_b = interp_loop_keys(&BLOB_B_KEYS, phase);
    let pos_a = interp_loop_keys(&BLOB_A_KEYS, phase);

    // Paint back-to-front matching CSS multiple-background stacking order: c1, c4, then c3 on top.
    rgb_out = blend_radial_blob(rgb_out, c1, (0.60, 1.60), pos_c, (u, v));
    rgb_out = blend_radial_blob(rgb_out, c4, (0.80, 1.90), pos_b, (u, v));
    rgb_out = blend_radial_blob(rgb_out, c3, (0.70, 1.70), pos_a, (u, v));

    Rgba {
        r: rgb_out[0].clamp(0., 1.),
        g: rgb_out[1].clamp(0., 1.),
        b: rgb_out[2].clamp(0., 1.),
        a: 1.,
    }
    .into()
}

fn paint_glyphs_in_span(
    shaped: &ShapedLine,
    baseline_origin: Point<Pixels>,
    span_x: (f32, f32),
    pad_px: f32,
    color: Hsla,
    window: &mut Window,
) {
    let origin_x = f32::from(baseline_origin.x);
    for run in &shaped.runs {
        for glyph in &run.glyphs {
            let gx = origin_x + f32::from(glyph.position.x);
            if gx + pad_px >= span_x.0 && gx - pad_px * 0.35 <= span_x.1 {
                let at = baseline_origin + glyph.position;
                let _ = window.paint_glyph(at, run.font_id, glyph.id, shaped.font_size, color);
            }
        }
    }
}

struct LiveClock {
    flow_started: Instant,
    sweep_started: Cell<Instant>,
    replay_nonce: Cell<usize>,
}

impl LiveClock {
    fn new() -> Self {
        let now = Instant::now();
        Self {
            flow_started: now,
            sweep_started: Cell::new(now),
            replay_nonce: Cell::new(0),
        }
    }
}

/// Animated liquid-gradient text span or heading with optional left-to-right entrance sweep.
///
/// # Examples
///
/// ```
/// use gpui::{FontWeight, prelude::*, px};
/// use kui::components::gradient_text::GradientText;
///
/// let payoff = GradientText::new("hero-payoff", "feel.")
///     .with_font_size(px(48.))
///     .with_font_weight(FontWeight::BOLD)
///     .with_speed(8.0)
///     .with_sweep(true);
/// ```
#[derive(IntoElement)]
pub struct GradientText {
    id: ElementId,
    text: SharedString,
    custom_colors: Option<[Hsla; 4]>,
    speed: f32,
    sweep: bool,
    paused: bool,
    font_size: Pixels,
    font_weight: FontWeight,
    line_height: Option<Pixels>,
    base_color: Option<Hsla>,
    heading_level: Option<usize>,
    replay_nonce: usize,
    clock: Rc<LiveClock>,
}

impl GradientText {
    /// Construct a `GradientText` with tuned pink-to-violet light and dark palettes.
    pub fn new(id: impl Into<ElementId>, text: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            custom_colors: None,
            speed: 8.0,
            sweep: true,
            paused: false,
            font_size: px(48.),
            font_weight: FontWeight::BOLD,
            line_height: None,
            base_color: None,
            heading_level: None,
            replay_nonce: 0,
            clock: Rc::new(LiveClock::new()),
        }
    }

    /// Construct the interactive hero headline showcase (`"Interfaces people actually feel."` + `"Replay"`).
    pub fn demo(
        id: impl Into<ElementId>,
        cx: &mut Context<GradientTextShowcase>,
    ) -> GradientTextShowcase {
        GradientTextShowcase::new(id, cx)
    }

    /// Override the four liquid gradient stops (`[start, end, blob_a, blob_b]`) across both themes.
    pub fn with_colors(mut self, colors: [Hsla; 4]) -> Self {
        self.custom_colors = Some(colors);
        self
    }

    /// Override the four gradient stops using `#rgb` or `#rrggbb` hex strings.
    pub fn with_hex_colors(mut self, hex: [&str; 4]) -> Self {
        let mut parsed = LIGHT_DEFAULTS.map(|v| Hsla::from(rgb(v)));
        for (slot, raw) in parsed.iter_mut().zip(hex) {
            if let Some(c) = parse_hex_rgb(raw) {
                *slot = c;
            }
        }
        self.custom_colors = Some(parsed);
        self
    }

    /// Set seconds per liquid-flow cycle (`8.0` default; `0.0` freezes the flow).
    pub fn with_speed(mut self, seconds: f32) -> Self {
        if seconds.is_finite() {
            self.speed = seconds.max(0.);
        }
        self
    }

    /// Enable or disable the initial `1100ms` left-to-right color sweep (`true` default).
    pub fn with_sweep(mut self, sweep: bool) -> Self {
        self.sweep = sweep;
        self
    }

    /// Pause or resume the continuous gradient flow.
    pub fn paused(mut self, paused: bool) -> Self {
        self.paused = paused;
        self
    }

    /// Builder alias to pause or resume the continuous gradient flow.
    pub fn with_paused(self, paused: bool) -> Self {
        self.paused(paused)
    }

    /// Set the glyph font size (`px(48.)` default).
    pub fn with_font_size(mut self, size: Pixels) -> Self {
        if f32::from(size).is_finite() && size > px(1.) {
            self.font_size = size;
        }
        self
    }

    /// Set the glyph font weight (`FontWeight::BOLD` default).
    pub fn with_font_weight(mut self, weight: FontWeight) -> Self {
        self.font_weight = weight;
        self
    }

    /// Override explicit line height for inline alignment with adjacent headline runs.
    pub fn with_line_height(mut self, height: Pixels) -> Self {
        if f32::from(height).is_finite() && height > px(1.) {
            self.line_height = Some(height);
        }
        self
    }

    /// Override the base pre-sweep text color (`Theme` foreground ink by default).
    pub fn with_base_color(mut self, color: Hsla) -> Self {
        self.base_color = Some(color);
        self
    }

    /// Expose this element as a semantic heading (`1..=6`) in the accessibility tree.
    pub fn with_heading_level(mut self, level: usize) -> Self {
        self.heading_level = Some(level.clamp(1, 6));
        self
    }

    /// Increment the replay counter for inline `RenderOnce` callers so the `1100ms` sweep restarts.
    pub fn with_replay_nonce(mut self, nonce: usize) -> Self {
        self.replay_nonce = nonce;
        self
    }

    /// Restart the `1100ms` left-to-right sweep on a mounted `GradientText` entity.
    pub fn replay(&mut self, cx: &mut Context<Self>) {
        self.replay_nonce = self.replay_nonce.wrapping_add(1);
        self.clock.replay_nonce.set(self.replay_nonce);
        self.clock.sweep_started.set(Instant::now());
        cx.notify();
    }

    /// Update the displayed text on a mounted entity.
    pub fn set_text(&mut self, text: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.text = text.into();
        cx.notify();
    }

    /// Update gradient colors on a mounted entity.
    pub fn set_colors(&mut self, colors: [Hsla; 4], cx: &mut Context<Self>) {
        self.custom_colors = Some(colors);
        cx.notify();
    }

    /// Update flow cycle duration on a mounted entity.
    pub fn set_speed(&mut self, seconds: f32, cx: &mut Context<Self>) {
        if seconds.is_finite() {
            self.speed = seconds.max(0.);
            cx.notify();
        }
    }

    /// Current text string.
    pub fn text(&self) -> &str {
        self.text.as_ref()
    }

    /// Configured flow cycle duration in seconds.
    pub fn speed(&self) -> f32 {
        self.speed
    }

    /// Whether the left-to-right sweep wipe is enabled.
    pub fn is_sweep_enabled(&self) -> bool {
        self.sweep
    }

    fn resolve_stops(&self, dark: bool) -> [[f32; 3]; 4] {
        if let Some(custom) = self.custom_colors {
            custom.map(hsla_to_rgb_f32)
        } else if dark {
            DARK_DEFAULTS.map(|hex| hsla_to_rgb_f32(rgb(hex).into()))
        } else {
            LIGHT_DEFAULTS.map(|hex| hsla_to_rgb_f32(rgb(hex).into()))
        }
    }

    fn render_element(&self, clock: Rc<LiveClock>, w: &mut Window, cx: &mut App) -> Stateful<Div> {
        let theme = Theme::global(cx);
        let dark = theme.mode == ThemeMode::Dark;
        let reduced = theme.reduced_motion;

        if clock.replay_nonce.get() != self.replay_nonce {
            clock.replay_nonce.set(self.replay_nonce);
            clock.sweep_started.set(Instant::now());
        }

        let base_ink = self.base_color.unwrap_or_else(|| {
            if dark {
                rgb(0xededed).into()
            } else {
                rgb(0x171717).into()
            }
        });
        let stops = self.resolve_stops(dark);
        let font_size = self.font_size;
        let line_h = self
            .line_height
            .unwrap_or_else(|| px((f32::from(font_size) * 1.12).round()));

        let mut run = w.text_style().to_run(self.text.len());
        run.font.weight = self.font_weight;
        run.color = base_ink;
        let shaped =
            Rc::new(
                w.text_system()
                    .shape_line(self.text.clone(), font_size, &[run], None),
            );

        let speed = self.speed;
        let sweep_enabled = self.sweep;
        let paused = self.paused;

        div()
            .id(self.id.clone())
            .when_some(self.heading_level, |d, lvl| {
                d.role(Role::Heading).aria_level(lvl)
            })
            .aria_label(self.text.clone())
            .relative()
            .flex_none()
            .text_size(font_size)
            .line_height(line_h)
            .font_weight(self.font_weight)
            .whitespace_nowrap()
            .text_color(transparent_black())
            .child(self.text.clone())
            .child(
                canvas(
                    |_, _, _| (),
                    move |bounds, _, window, _| {
                        let bw = f32::from(bounds.size.width);
                        let bh = f32::from(bounds.size.height);
                        if bw <= 0.5 || bh <= 0.5 {
                            return;
                        }

                        let flow_running = !reduced && !paused && speed > 0.001;
                        let flow_phase = if flow_running {
                            (clock.flow_started.elapsed().as_secs_f32() / speed).fract()
                        } else {
                            0.0
                        };

                        let sweep_elapsed = clock.sweep_started.get().elapsed().as_secs_f32();
                        let sweep_progress = if reduced || !sweep_enabled {
                            1.0
                        } else {
                            gt_sweep_ease((sweep_elapsed / SWEEP_SECONDS).clamp(0., 1.))
                        };
                        let sweep_running =
                            !reduced && sweep_enabled && sweep_elapsed < SWEEP_SECONDS;

                        let parent_mask = window.content_mask().bounds;
                        let font_px = f32::from(font_size);
                        let over_x = font_px * 0.22;
                        let over_y = px(font_px * 0.28);
                        let mask_top = bounds.origin.y - over_y;
                        let mask_h = bounds.size.height + over_y * 2.;

                        let padding_top =
                            (bounds.size.height - shaped.ascent - shaped.descent) * 0.5;
                        let baseline_origin = point(
                            bounds.origin.x,
                            bounds.origin.y + padding_top + shaped.ascent,
                        );

                        let x_min = f32::from(bounds.origin.x) - over_x;
                        let x_max = f32::from(bounds.right()) + over_x;
                        let sweep_x = x_min + (x_max - x_min) * sweep_progress;

                        // Base headline ink to the right of the sweeping wavefront.
                        if sweep_progress < 0.999 {
                            let right_w = (x_max - sweep_x).max(0.);
                            let right_mask = Bounds::new(
                                point(px(sweep_x), mask_top),
                                size(px(right_w), mask_h),
                            )
                            .intersect(&parent_mask);
                            if !right_mask.is_empty() {
                                window.with_content_mask(
                                    Some(ContentMask { bounds: right_mask }),
                                    |window| {
                                        paint_glyphs_in_span(
                                            &shaped,
                                            baseline_origin,
                                            (sweep_x, x_max),
                                            font_px * 0.95,
                                            base_ink,
                                            window,
                                        );
                                    },
                                );
                            }
                        }

                        // Liquid-gradient vertical slices across the revealed span.
                        if sweep_progress > 0.001 {
                            let grad_end = sweep_x.min(x_max);
                            let total_span = (grad_end - x_min).max(0.);
                            let step_px = (bw / 46.).clamp(2.0, 4.5);
                            let count = ((total_span / step_px).ceil() as usize).clamp(1, 96);
                            let origin_x = f32::from(bounds.origin.x);

                            for i in 0..count {
                                let sx0 = x_min + i as f32 * step_px;
                                let sx1 = (sx0 + step_px).min(grad_end);
                                if sx1 <= sx0 {
                                    break;
                                }
                                let mid_x = (sx0 + sx1) * 0.5;
                                let u = ((mid_x - origin_x) / bw).clamp(0., 1.);
                                let v = (0.48 + 0.14 * (u - 0.5)).clamp(0.15, 0.85);
                                let col = sample_liquid_gradient(u, v, flow_phase, stops);

                                let slice_mask = Bounds::new(
                                    point(px(sx0), mask_top),
                                    size(px(sx1 - sx0), mask_h),
                                )
                                .intersect(&parent_mask);
                                if slice_mask.is_empty() {
                                    continue;
                                }
                                window.with_content_mask(
                                    Some(ContentMask { bounds: slice_mask }),
                                    |window| {
                                        paint_glyphs_in_span(
                                            &shaped,
                                            baseline_origin,
                                            (sx0, sx1),
                                            font_px * 0.95,
                                            col,
                                            window,
                                        );
                                    },
                                );
                            }
                        }

                        if flow_running || sweep_running {
                            window.request_animation_frame();
                        }
                    },
                )
                .absolute()
                .inset_0(),
            )
    }
}

impl Render for GradientText {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_element(self.clock.clone(), w, cx)
    }
}

impl RenderOnce for GradientText {
    fn render(self, w: &mut Window, cx: &mut App) -> impl IntoElement {
        let state_id: ElementId = (self.id.clone(), "gt-live-clock").into();
        let st = w.use_keyed_state(state_id, cx, |_, _| Rc::new(LiveClock::new()));
        let clock = st.read(cx).clone();
        self.render_element(clock, w, cx)
    }
}

/// Interactive hero showcase pairing `"Interfaces people actually "` with a sweeping
/// [`GradientText`] `"feel."` payoff and a `"Replay"` trigger button.
pub struct GradientTextShowcase {
    id: ElementId,
    replay_nonce: usize,
}

impl GradientTextShowcase {
    /// Construct a new `GradientTextShowcase` subscribed to [`Theme`] changes.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        Self {
            id: id.into(),
            replay_nonce: 0,
        }
    }

    /// Replay the `1100ms` left-to-right gradient sweep from the beginning.
    pub fn replay(&mut self, cx: &mut Context<Self>) {
        self.replay_nonce = self.replay_nonce.wrapping_add(1);
        cx.notify();
    }
}

impl Render for GradientTextShowcase {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::global(cx);
        let dark = theme.mode == ThemeMode::Dark;
        let lang = UiLanguage::current(cx);
        let focus_ring = theme.tokens.colors.focus_outline;

        let vw = f32::from(w.viewport_size().width);
        let (font_px, line_px): (f32, f32) = if vw > 0. && vw < 540. {
            (38., 43.)
        } else {
            (56., 61.)
        };

        let ink: Hsla = if dark {
            rgb(0xededed).into()
        } else {
            rgb(0x171717).into()
        };
        let btn_bg: Hsla = if dark {
            rgb(0x232326).into()
        } else {
            rgb(0xffffff).into()
        };
        let btn_border: Hsla = if dark {
            hsla(0., 0., 1., 0.10)
        } else {
            hsla(0., 0., 0., 0.11)
        };

        let line1 = lang.text("Interfaces people", "真正让人动容的");
        let line2_prefix = lang.text("actually ", "交互");
        let payoff = lang.text("feel.", "质感。");
        let replay_label = lang.text("Replay", "重播");

        let gt_id: ElementId = (self.id.clone(), "hero-feel").into();
        let replay_id: ElementId = (self.id.clone(), "replay-btn").into();

        div()
            .id(self.id.clone())
            .w_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .px(px(24.))
            .py(px(32.))
            .child(
                div()
                    .id((self.id.clone(), "hero-heading"))
                    .role(Role::Heading)
                    .aria_level(2)
                    .aria_label(format!("{line1} {line2_prefix}{payoff}"))
                    .flex()
                    .flex_col()
                    .items_center()
                    .text_size(px(font_px))
                    .line_height(px(line_px))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(ink)
                    .child(div().whitespace_nowrap().child(line1))
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .justify_center()
                            .child(div().whitespace_nowrap().child(line2_prefix))
                            .child(
                                GradientText::new(gt_id, payoff)
                                    .with_font_size(px(font_px))
                                    .with_line_height(px(line_px))
                                    .with_font_weight(FontWeight::SEMIBOLD)
                                    .with_base_color(ink)
                                    .with_speed(8.0)
                                    .with_sweep(true)
                                    .with_replay_nonce(self.replay_nonce),
                            ),
                    ),
            )
            .child(
                div()
                    .mt(px(28.))
                    .id(replay_id)
                    .role(Role::Button)
                    .aria_label(replay_label)
                    .tab_index(0)
                    .h(px(30.))
                    .px(px(13.))
                    .rounded(px(9.))
                    .bg(btn_bg)
                    .border_1()
                    .border_color(btn_border)
                    .shadow(vec![BoxShadow {
                        color: hsla(0., 0., 0., if dark { 0.28 } else { 0.04 }),
                        offset: point(px(0.), px(1.)),
                        blur_radius: px(2.),
                        spread_radius: px(0.),
                        inset: false,
                    }])
                    .text_color(ink)
                    .text_size(px(12.5))
                    .line_height(px(18.))
                    .font_weight(FontWeight::MEDIUM)
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .hover(|s| s.opacity(0.86))
                    .focus_visible(move |s| s.border_color(focus_ring))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.replay(cx);
                    }))
                    .child(replay_label),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DARK_DEFAULTS, GradientText, GradientTextShowcase, LIGHT_DEFAULTS, gt_sweep_ease,
        hsla_to_rgb_f32, sample_liquid_gradient,
    };
    use gpui::{Rgba, TestAppContext, rgb};

    #[test]
    fn sweep_easing_is_smooth_s_curve_bounded_zero_to_one() {
        assert!((gt_sweep_ease(0.) - 0.).abs() < 1e-5);
        assert!((gt_sweep_ease(1.) - 1.).abs() < 1e-5);
        let early = gt_sweep_ease(0.2);
        let mid = gt_sweep_ease(0.5);
        let late = gt_sweep_ease(0.8);
        assert!(early > 0. && early < mid && mid < late && late < 1.);
    }

    #[test]
    fn liquid_gradient_varies_across_word_and_cycles_smoothly() {
        let stops = LIGHT_DEFAULTS.map(|hex| hsla_to_rgb_f32(rgb(hex).into()));
        let left = Rgba::from(sample_liquid_gradient(0.05, 0.5, 0.0, stops));
        let right = Rgba::from(sample_liquid_gradient(0.95, 0.5, 0.0, stops));
        let diff = (left.r - right.r).abs() + (left.g - right.g).abs() + (left.b - right.b).abs();
        assert!(
            diff > 0.25,
            "expected distinct pink-to-violet gradient across word, got diff={diff}"
        );

        let dark_stops = DARK_DEFAULTS.map(|hex| hsla_to_rgb_f32(rgb(hex).into()));
        let p0 = Rgba::from(sample_liquid_gradient(0.5, 0.5, 0.0, dark_stops));
        let p1 = Rgba::from(sample_liquid_gradient(0.5, 0.5, 1.0, dark_stops));
        assert!((p0.r - p1.r).abs() < 1e-4 && (p0.b - p1.b).abs() < 1e-4);
    }

    #[test]
    fn showcase_replay_increments_nonce_and_mounts_cleanly() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let gt = GradientText::new("unit-gt", "feel.")
            .with_hex_colors(["#ff0066", "#6600ff", "#ff66aa", "#8866ff"]);
        assert_eq!(gt.text(), "feel.");
        assert!(gt.is_sweep_enabled());

        let (view, window) =
            app.add_window_view(|_, cx| GradientTextShowcase::new("gt-showcase", cx));
        window.run_until_parked();
        assert_eq!(window.update(|_, cx| view.read(cx).replay_nonce), 0);
        window.update(|_, cx| {
            view.update(cx, |s, cx| s.replay(cx));
        });
        assert_eq!(window.update(|_, cx| view.read(cx).replay_nonce), 1);
    }
}
