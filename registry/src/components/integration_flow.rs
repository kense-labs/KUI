//! Rotating integration pills along a dashed orbit beaming into a central Kense hub.
//!
//! Renders a `2400 : 1490` responsive product scene over breathing coral radial blobs
//! where integration capsules travel slot by slot along a dashed arc and fire glowing
//! data pulses along curved beams into the central Kense `Data Analysis` hub.

use crate::{
    components::pro_chrome::{
        ProPalette, pro_ease, pro_eid, pro_empty_state, pro_mix_color, pro_skeleton_bar, tr,
    },
    foundation::{Theme, ThemeMode},
    primitives::{
        control::UiLanguage,
        family::{Slide, disabled_motion},
        pro_icons::{ProIcon, pro_app_artwork, pro_icon, pro_raw_svg, pro_svg},
    },
};
use gpui::{prelude::*, *};
use image::{Frame, RgbaImage};
use std::{
    cell::{Cell, RefCell},
    f32::consts::TAU,
    rc::Rc,
    sync::Arc,
};
use web_time::Instant;

const BG_TEX_W: usize = 160;
const BG_TEX_H: usize = 100;
const ART_W: f32 = 1800.0;
const ART_H: f32 = 1117.5;

const SNOWFLAKE_MONO_SVG: &str = r#"<path transform="translate(-2 -1)" fill="currentColor" fill-rule="evenodd" clip-rule="evenodd" d="M58.82 26.22L51.64 30.37L58.82 34.47C60.55 35.47 61.15 37.68 60.20 39.64C59.25 41.40 56.95 42.05 55.04 41.02L42.16 33.60C40.85 32.84 40.15 31.35 40.41 29.34C40.67 28.39 41.30 27.58 42.15 27.08L55.03 19.69C56.93 18.65 59.24 19.31 60.21 21.08C61.18 22.85 60.59 25.15 58.82 26.22ZM52.02 46.30L39.15 38.89C37.80 38.12 36.15 38.12 34.80 38.89C33.90 39.45 33.45 40.75 33.45 42.16V56.95C33.48 59.00 35.18 60.61 37.24 60.61C39.30 60.61 40.99 59.00 41.03 56.95V48.66L48.23 52.80C50.12 53.85 52.42 53.20 53.40 51.42C54.38 49.64 53.78 47.35 52.02 46.30ZM37.18 31.83L31.82 37.13H28.90L23.56 31.81V28.91L28.90 23.59H31.82L37.17 28.91V31.83ZM8.70 14.44L21.57 21.82C22.90 22.55 24.55 22.55 25.90 21.82C26.80 21.25 27.28 19.95 27.28 18.55V3.77C27.24 1.72 25.55 0.11 23.49 0.11C21.43 0.11 19.73 1.72 19.70 3.77V12.06L12.48 7.92C10.59 6.85 8.29 7.50 7.32 9.30C6.35 11.10 6.94 13.38 8.70 14.44ZM36.95 22.32C37.71 22.38 38.47 22.21 39.13 21.83L52.01 14.41C53.77 13.36 54.38 11.08 53.39 9.25C52.40 7.42 50.09 6.80 48.22 7.87L41.02 12.05V3.76C40.98 1.70 39.29 0.10 37.23 0.10C35.17 0.10 33.47 1.70 33.44 3.76V18.55C33.44 20.55 34.95 22.18 36.95 22.32ZM23.77 38.40C23.01 38.34 22.25 38.51 21.59 38.89L8.70 46.28C6.94 47.33 6.34 49.62 7.32 51.44C8.30 53.26 10.60 53.90 12.48 52.82L19.70 48.68V56.97C19.73 59.02 21.43 60.64 23.49 60.64C25.55 60.64 27.24 59.02 27.28 56.97V42.16C27.27 40.16 25.77 38.54 23.77 38.40ZM20.28 31.47C20.62 30.00 19.92 27.90 18.54 27.11L5.67 19.70C3.80 18.65 1.48 19.30 0.51 21.08C-0.46 22.86 0.13 25.15 1.90 26.22L9.08 30.37L1.90 34.47C0.13 35.52 -0.46 37.82 0.51 39.63C1.48 41.44 3.80 42.08 5.67 41.02L18.54 33.60C19.36 33.14 19.99 32.38 20.28 31.47Z"></path>"#;

const OKTA_AURA_SVG: &str = r#"<circle cx="12" cy="12" r="9.2" stroke="currentColor" stroke-width="3.6" stroke-dasharray="2.4 3.2"></circle><circle cx="12" cy="12" r="4.2" fill="currentColor"></circle>"#;

const CLICKUP_MONO_SVG: &str = r#"<path fill="currentColor" fill-rule="evenodd" clip-rule="evenodd" d="M0 45.9L9.16 38.82C14.06 45.17 19.23 48.17 24.95 48.17C30.66 48.17 35.74 45.27 40.37 38.92L49.71 45.81C43.0 54.88 34.65 59.69 24.95 59.69C15.33 59.69 6.89 54.88 0 45.9ZM24.95 15.33L8.63 29.39L1.1 20.59L25.04 0L48.81 20.59L41.19 29.3L24.95 15.33Z"></path>"#;

const CLICKUP_COLOR_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 54 64" fill="none"><path fill-rule="evenodd" clip-rule="evenodd" d="M0 49.22L9.95 41.63C15.27 48.44 20.89 51.65 27.10 51.65C33.31 51.65 38.82 48.54 43.85 41.73L54.0 49.12C46.71 58.85 37.64 64.00 27.10 64.00C16.65 64.00 7.49 58.85 0 49.22Z" fill="#6E56CF"/><path fill-rule="evenodd" clip-rule="evenodd" d="M27.10 16.44L9.36 31.51L1.18 22.08L27.20 0L53.02 22.08L44.74 31.42L27.10 16.44Z" fill="#FF3B9A"/></svg>"##;

const SLACK_MONO_SVG: &str = r#"<path fill="currentColor" d="M5.04 15.17a2.53 2.53 0 0 1-2.52 2.52A2.53 2.53 0 0 1 0 15.17a2.53 2.53 0 0 1 2.52-2.53h2.52zm1.27 0a2.53 2.53 0 0 1 2.53-2.53 2.53 2.53 0 0 1 2.52 2.53v6.31A2.53 2.53 0 0 1 8.84 24a2.53 2.53 0 0 1-2.53-2.52zM8.84 5.04a2.53 2.53 0 0 1-2.53-2.52A2.53 2.53 0 0 1 8.84 0a2.53 2.53 0 0 1 2.52 2.52v2.52zm0 1.27a2.53 2.53 0 0 1 2.52 2.53 2.53 2.53 0 0 1-2.52 2.52H2.52A2.53 2.53 0 0 1 0 8.84a2.53 2.53 0 0 1 2.52-2.53zM18.96 8.84a2.53 2.53 0 0 1 2.52-2.53A2.53 2.53 0 0 1 24 8.84a2.53 2.53 0 0 1-2.52 2.52h-2.52zm-1.27 0a2.53 2.53 0 0 1-2.53 2.52 2.53 2.53 0 0 1-2.52-2.52V2.52A2.53 2.53 0 0 1 15.16 0a2.53 2.53 0 0 1 2.53 2.52zM15.16 18.96a2.53 2.53 0 0 1 2.53 2.52A2.53 2.53 0 0 1 15.16 24a2.53 2.53 0 0 1-2.52-2.52v-2.52zm0-1.27a2.53 2.53 0 0 1-2.52-2.53 2.53 2.53 0 0 1 2.52-2.52h6.32A2.53 2.53 0 0 1 24 15.16a2.53 2.53 0 0 1-2.52 2.53z"></path>"#;

const SLACK_COLOR_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none"><path fill="#E01E5A" d="M5.04 15.17a2.53 2.53 0 0 1-2.52 2.52A2.53 2.53 0 0 1 0 15.17a2.53 2.53 0 0 1 2.52-2.53h2.52v2.53zm1.27 0a2.53 2.53 0 0 1 2.53-2.53 2.53 2.53 0 0 1 2.52 2.53v6.31A2.53 2.53 0 0 1 8.84 24a2.53 2.53 0 0 1-2.53-2.52v-6.31z"/><path fill="#36C5F0" d="M8.84 5.04a2.53 2.53 0 0 1-2.53-2.52A2.53 2.53 0 0 1 8.84 0a2.53 2.53 0 0 1 2.52 2.52v2.52H8.84zm0 1.27a2.53 2.53 0 0 1 2.52 2.53 2.53 2.53 0 0 1-2.52 2.52H2.52A2.53 2.53 0 0 1 0 8.84a2.53 2.53 0 0 1 2.52-2.53h6.32z"/><path fill="#2EB67D" d="M18.96 8.84a2.53 2.53 0 0 1 2.52-2.53A2.53 2.53 0 0 1 24 8.84a2.53 2.53 0 0 1-2.52 2.52h-2.52V8.84zm-1.27 0a2.53 2.53 0 0 1-2.53 2.52 2.53 2.53 0 0 1-2.52-2.52V2.52A2.53 2.53 0 0 1 15.16 0a2.53 2.53 0 0 1 2.53 2.52v6.32z"/><path fill="#ECB22E" d="M15.16 18.96a2.53 2.53 0 0 1 2.53 2.52A2.53 2.53 0 0 1 15.16 24a2.53 2.53 0 0 1-2.52-2.52v-2.52h2.52zm0-1.27a2.53 2.53 0 0 1-2.52-2.53 2.53 2.53 0 0 1 2.52-2.52h6.32A2.53 2.53 0 0 1 24 15.16a2.53 2.53 0 0 1-2.52 2.53h-6.32z"/></svg>"##;

const ZENDESK_MONO_SVG: &str = r#"<path fill="currentColor" d="M12.914 2.904V16.29L24 2.905H12.914zM0 2.906C0 5.966 2.483 8.45 5.543 8.45s5.542-2.484 5.543-5.544H0zm11.086 4.807L0 21.096h11.086V7.713zm7.37 7.84c-3.063 0-5.542 2.48-5.542 5.543H24c0-3.06-2.48-5.543-5.543-5.543z"></path>"#;

type CustomIconFn = Rc<dyn Fn(bool, f32, &mut Window, &mut App) -> AnyElement>;

/// Single integration capsule rotating along the orbit in [`IntegrationFlow`].
#[derive(Clone)]
pub struct IntegrationItem {
    /// Stable identifier (also used to match built-in artwork).
    pub id: SharedString,
    /// Integration subtitle (for example `"Slack"`, `"Snowflake"`, `"ClickUp"`).
    pub name: SharedString,
    /// Optional top caption inside the pill (`"API Integration"` when `None`).
    pub title: Option<SharedString>,
    /// Whether the brand mark renders monochrome in the active center slot.
    pub mono: bool,
    icon: Option<CustomIconFn>,
}

impl std::fmt::Debug for IntegrationItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IntegrationItem")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("title", &self.title)
            .field("mono", &self.mono)
            .finish()
    }
}

impl IntegrationItem {
    /// Create an [`IntegrationItem`] with `id` and display `name`.
    pub fn new(id: impl Into<SharedString>, name: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            title: None,
            mono: false,
            icon: None,
        }
    }

    /// Override the top caption line inside the pill (`"API Integration"` by default).
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Mark the logo as monochrome (`true` tints dark in light mode and white in dark mode at center).
    pub fn with_mono(mut self, mono: bool) -> Self {
        self.mono = mono;
        self
    }

    /// Supply a custom icon renderer `(is_center, size_px, window, cx) -> AnyElement`.
    pub fn with_icon(
        mut self,
        render: impl Fn(bool, f32, &mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        self.icon = Some(Rc::new(render));
        self
    }
}

/// Default 5-integration rotation matching the reference scene (`Okta`, `Snowflake`, `Slack` at center, `ClickUp`, `Zendesk`).
pub fn default_integration_items() -> Vec<IntegrationItem> {
    vec![
        IntegrationItem::new("okta", "Okta").with_mono(true),
        IntegrationItem::new("snowflake", "Snowflake"),
        IntegrationItem::new("slack", "Slack"),
        IntegrationItem::new("clickup", "ClickUp"),
        IntegrationItem::new("zendesk", "Zendesk").with_mono(true),
    ]
}

/// Events emitted by [`IntegrationFlow`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IntegrationFlowEvent {
    /// Emitted when clicking or pressing Space pauses or resumes the orbit loop.
    PausedChanged(bool),
    /// Emitted when clicking an integration pill focuses/selects it.
    Select(SharedString),
}

impl EventEmitter<IntegrationFlowEvent> for IntegrationFlow {}

#[derive(Clone, Copy, PartialEq, Eq)]
struct CoralBgKey {
    dark: bool,
    bg_rgb: [u8; 3],
    fade_rgb: [u8; 3],
    phase_q: u16,
}

struct CoralBgCache {
    key: Option<CoralBgKey>,
    image: Option<Arc<RenderImage>>,
}

impl CoralBgCache {
    fn new() -> Self {
        Self {
            key: None,
            image: None,
        }
    }

    fn render(&mut self, key: CoralBgKey, window: &mut Window) -> Arc<RenderImage> {
        if self.key == Some(key)
            && let Some(existing) = &self.image
        {
            return existing.clone();
        }
        let img = Arc::new(rasterize_coral_backdrop(key));
        if let Some(prev) = self.image.replace(img.clone()) {
            let _ = window.drop_image(prev);
        }
        self.key = Some(key);
        img
    }
}

fn hsla_to_rgb8(c: Hsla) -> [u8; 3] {
    let rgba: Rgba = c.into();
    [
        (rgba.r.clamp(0., 1.) * 255.).round() as u8,
        (rgba.g.clamp(0., 1.) * 255.).round() as u8,
        (rgba.b.clamp(0., 1.) * 255.).round() as u8,
    ]
}

fn blend_rgb(dst: [f32; 3], src: [f32; 3], alpha: f32) -> [f32; 3] {
    let a = alpha.clamp(0., 1.);
    let inv = 1. - a;
    [
        dst[0] * inv + src[0] * a,
        dst[1] * inv + src[1] * a,
        dst[2] * inv + src[2] * a,
    ]
}

fn radial_falloff(t: f32) -> f32 {
    if t >= 1.0 {
        return 0.0;
    }
    let inv = 1.0 - t * t;
    inv * inv
}

/// Rasterize the warm coral/peach radial-blob stage backdrop with 16s harmonic drift and bottom fade.
fn rasterize_coral_backdrop(key: CoralBgKey) -> RenderImage {
    let phase = (f32::from(key.phase_q) / 1000.0) * TAU;
    let base = key.bg_rgb.map(|v| f32::from(v) / 255.);
    let fade = key.fade_rgb.map(|v| f32::from(v) / 255.);
    let blob_mul = if key.dark { 0.72 } else { 1.0 };

    // Normalized `(cx, cy, rx, ry, rgb, peak_alpha, phase_shift)` derived from `.ui-if-blob--*`
    type BlobSpec = (f32, f32, f32, f32, [f32; 3], f32, f32);
    let blobs: [BlobSpec; 6] = [
        (
            0.08 + 0.05 * (phase + 0.4).cos(),
            0.34 + 0.05 * (phase + 0.4).sin(),
            0.62,
            0.72,
            [1.0, 112. / 255., 56. / 255.],
            0.78,
            0.0,
        ),
        (
            0.32 + 0.06 * (phase - 1.1).sin(),
            0.12 + 0.05 * (phase - 1.1).cos(),
            0.68,
            0.76,
            [245. / 255., 89. / 255., 86. / 255.],
            0.74,
            1.2,
        ),
        (
            0.18 + 0.05 * (phase + 2.0).cos(),
            0.64 + 0.04 * (phase + 2.0).sin(),
            0.56,
            0.66,
            [1.0, 130. / 255., 68. / 255.],
            0.76,
            2.1,
        ),
        (
            0.64 + 0.05 * (phase - 0.7).cos(),
            0.22 + 0.05 * (phase - 0.7).sin(),
            0.64,
            0.78,
            [1.0, 146. / 255., 138. / 255.],
            0.54,
            3.0,
        ),
        (
            0.86 + 0.04 * (phase + 1.5).sin(),
            0.52 + 0.05 * (phase + 1.5).cos(),
            0.58,
            0.70,
            [1.0, 168. / 255., 124. / 255.],
            0.44,
            4.2,
        ),
        (
            0.54 + 0.04 * (phase + 2.7).cos(),
            0.56 + 0.04 * (phase + 2.7).sin(),
            0.54,
            0.62,
            [1.0, 1.0, 1.0],
            if key.dark { 0.0 } else { 0.42 },
            5.0,
        ),
    ];

    let mut raw = vec![0u8; BG_TEX_W * BG_TEX_H * 4];
    for py in 0..BG_TEX_H {
        let ny = (py as f32 + 0.5) / BG_TEX_H as f32;
        // `.ui-if-fade`: bottom 41% (`612 / 1490`) fades smoothly into `--ui-if-fade`
        let fade_t = ((ny - 0.59) / (0.975 - 0.59)).clamp(0., 1.);
        let fade_smooth = fade_t * fade_t * (3. - 2. * fade_t);

        for px_i in 0..BG_TEX_W {
            let nx = (px_i as f32 + 0.5) / BG_TEX_W as f32;
            let mut c = base;
            for &(bx, by, rx, ry, rgb, a0, p_off) in &blobs {
                if a0 <= 0.001 {
                    continue;
                }
                let breathe = 0.90 + 0.10 * (phase + p_off).sin();
                let dist = ((nx - bx) / rx).hypot((ny - by) / ry);
                let w = radial_falloff(dist) * a0 * blob_mul * breathe;
                if w > 0.001 {
                    c = blend_rgb(c, rgb, w);
                }
            }
            c = blend_rgb(c, fade, fade_smooth);

            let dst = (py * BG_TEX_W + px_i) * 4;
            raw[dst] = (c[2].clamp(0., 1.) * 255.).round() as u8;
            raw[dst + 1] = (c[1].clamp(0., 1.) * 255.).round() as u8;
            raw[dst + 2] = (c[0].clamp(0., 1.) * 255.).round() as u8;
            raw[dst + 3] = 255;
        }
    }

    let buf = RgbaImage::from_raw(BG_TEX_W as u32, BG_TEX_H as u32, raw)
        .expect("BG_TEX_W * BG_TEX_H * 4 matches raw buffer length");
    RenderImage::new(vec![Frame::new(buf)])
}

fn smooth_taper(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0., 1.);
    t * t * (3. - 2. * t)
}

/// Evaluate orbit slot `(dx, dy, opacity)` in `1800 x 1117.5` artboard coordinates for continuous slot offset `s` in `-3.0..=3.0`.
/// At `s = 0.0`, the pill sits at the active center slot `(1020, 479.4)`.
fn orbit_pill_offset(s: f32) -> (f32, f32, f32) {
    let dy = s * 279.5;
    let ratio = (dy / 776.625).clamp(-0.98, 0.98);
    let x_on_ellipse = 757.5 * (1.0 - ratio * ratio).sqrt();
    let dx = (757.5 - x_on_ellipse) * 1.48;
    let a = s.abs();
    let opacity = if a <= 1.15 {
        1.0
    } else {
        1.0 - smooth_taper(1.15, 2.35, a)
    };
    (dx, dy, opacity)
}

/// Evaluate loop state `(step_index, shift_frac, beam_progress, charge_glow)` at `elapsed_s`.
/// Starts settled at center (`shift_frac = 0`) while sweeping the beam (`0.0..1.1s`), then rotates one slot (`1.3..2.2s`).
fn evaluate_orbit_cycle(elapsed_s: f32, item_count: usize) -> (usize, f32, Option<f32>, f32) {
    if item_count == 0 {
        return (0, 0., None, 0.);
    }
    let period = 2.35f32;
    let step = ((elapsed_s / period).floor() as usize) % item_count;
    let within = elapsed_s.rem_euclid(period);
    if within < 1.15 {
        let bt = (within / 1.10).clamp(0., 1.);
        let charge = (1.0 - ((bt - 0.5) * 2.0).abs()).clamp(0., 1.);
        (step, 0.0, Some(bt), charge)
    } else if within < 1.45 {
        (step, 0.0, None, 0.0)
    } else {
        let u = ((within - 1.45) / 0.90).clamp(0., 1.);
        let eased = pro_ease(u);
        (step, eased, None, 0.0)
    }
}

fn render_integration_mark(
    item: &IntegrationItem,
    is_center: bool,
    dark: bool,
    side: f32,
    w: &mut Window,
    cx: &mut App,
) -> AnyElement {
    if let Some(custom) = &item.icon {
        return custom(is_center, side, w, cx);
    }
    let key = item.id.trim().to_ascii_lowercase();
    let ghost_white: Hsla = hsla(0., 0., 1., 0.92);
    let mono_center: Hsla = if dark {
        hsla(0., 0., 1., 0.92)
    } else {
        hsla(0., 0., 0., 0.86)
    };

    if !is_center {
        return match key.as_str() {
            "snowflake" => pro_svg("0 0 60 61", SNOWFLAKE_MONO_SVG, side, ghost_white),
            "okta" => pro_svg("0 0 24 24", OKTA_AURA_SVG, side, ghost_white),
            "clickup" => pro_svg("0 0 50 60", CLICKUP_MONO_SVG, side, ghost_white),
            "slack" => pro_svg("0 0 24 24", SLACK_MONO_SVG, side * 0.88, ghost_white),
            "zendesk" => pro_svg("0 0 24 24", ZENDESK_MONO_SVG, side * 0.88, ghost_white),
            _ => pro_app_artwork(&key, side * 0.88, cx)
                .map(|el| div().opacity(0.90).child(el).into_any_element())
                .unwrap_or_else(|| pro_icon(ProIcon::Link, side * 0.8, ghost_white)),
        };
    }

    // Active center pill shows full-color provider artwork (or dark/light ink for `mono` logos)
    match key.as_str() {
        "snowflake" => pro_svg("0 0 60 61", SNOWFLAKE_MONO_SVG, side, rgb(0x29b5e8).into()),
        "slack" => pro_raw_svg(SLACK_COLOR_SVG, side * 0.88, side * 0.88),
        "okta" => pro_svg("0 0 24 24", OKTA_AURA_SVG, side, mono_center),
        "clickup" => pro_raw_svg(CLICKUP_COLOR_SVG, side * 0.86, side),
        "zendesk" => pro_svg("0 0 24 24", ZENDESK_MONO_SVG, side * 0.88, mono_center),
        _ if item.mono => pro_app_artwork(&key, side * 0.88, cx)
            .unwrap_or_else(|| pro_icon(ProIcon::Link, side * 0.8, mono_center)),
        _ => pro_app_artwork(&key, side * 0.88, cx)
            .unwrap_or_else(|| pro_icon(ProIcon::Link, side * 0.8, mono_center)),
    }
}

/// Self-playing integration orbit scene beaming data pulses into a central Kense hub.
///
/// # Examples
///
/// ```no_run
/// # #[allow(unused_variables, unused_imports)]
/// # fn example(window: &mut gpui::Window, cx: &mut gpui::App) {
/// # use gpui::prelude::*;
/// use kui::components::integration_flow::IntegrationFlow;
///
/// let flow = cx.new(|cx| IntegrationFlow::new("hero-integrations", cx));
/// # }
/// ```
pub struct IntegrationFlow {
    id: ElementId,
    focus: FocusHandle,
    items: Vec<IntegrationItem>,
    hub_title: Option<SharedString>,
    hub_subtitle: Option<SharedString>,
    label: Option<SharedString>,
    background: Option<Hsla>,
    fade_color: Option<Hsla>,
    paused: bool,
    hovered_slot: Option<isize>,
    selected_id: Option<SharedString>,
    slot_illum: [Slide; 5],
    slot_press: [Slide; 5],
    hub_press: Slide,
    slot_packets: Vec<(isize, Instant)>,
    height: Option<Pixels>,
    loading: bool,
    empty: bool,
    clock: Instant,
    measured_w: Rc<Cell<f32>>,
    measured_h: Rc<Cell<f32>>,
    bg_cache: Rc<RefCell<CoralBgCache>>,
}

impl IntegrationFlow {
    /// Construct an [`IntegrationFlow`] with `Okta`, `Snowflake`, `Slack`, `ClickUp`, and `Zendesk` orbiting into the Kense hub.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<UiLanguage>(|_, cx| cx.notify())
            .detach();
        Self {
            id: id.into(),
            focus: cx.focus_handle().tab_stop(true),
            items: default_integration_items(),
            hub_title: None,
            hub_subtitle: None,
            label: None,
            background: None,
            fade_color: None,
            paused: false,
            hovered_slot: None,
            selected_id: None,
            slot_illum: [
                Slide::snappy(0.),
                Slide::snappy(0.),
                Slide::snappy(0.),
                Slide::snappy(0.),
                Slide::snappy(0.),
            ],
            slot_press: [
                Slide::snappy(0.),
                Slide::snappy(0.),
                Slide::snappy(0.),
                Slide::snappy(0.),
                Slide::snappy(0.),
            ],
            hub_press: Slide::snappy(0.),
            slot_packets: Vec::new(),
            height: None,
            loading: false,
            empty: false,
            clock: cx.background_executor().now(),
            measured_w: Rc::new(Cell::new(760.)),
            measured_h: Rc::new(Cell::new(0.)),
            bg_cache: Rc::new(RefCell::new(CoralBgCache::new())),
        }
    }

    /// Replace the rotating integration items (`5` or more recommended for full-arc occupancy).
    pub fn with_items(mut self, items: Vec<IntegrationItem>) -> Self {
        self.items = items;
        self
    }

    /// Override the central hub heading (`"Data Analysis"` by default).
    pub fn with_hub_title(mut self, title: impl Into<SharedString>) -> Self {
        self.hub_title = Some(title.into());
        self
    }

    /// Override the central hub subtitle (`"Centralized Data Processing"` by default).
    pub fn with_hub_subtitle(mut self, subtitle: impl Into<SharedString>) -> Self {
        self.hub_subtitle = Some(subtitle.into());
        self
    }

    /// Override the accessible description on the root scene container.
    pub fn with_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Override the base stage color behind the coral radial blobs.
    pub fn with_background(mut self, bg: Hsla) -> Self {
        self.background = Some(bg);
        self
    }

    /// Override the bottom fade destination color (`#ffffff` in light, `#1f1f21` in dark by default).
    pub fn with_fade_color(mut self, fade: Hsla) -> Self {
        self.fade_color = Some(fade);
        self
    }

    /// Start with orbit and beam animations paused.
    pub fn with_paused(mut self, paused: bool) -> Self {
        self.paused = paused;
        self
    }

    /// Override the container height explicitly (defaults to `2400 : 1490` aspect ratio).
    pub fn with_height(mut self, height: Pixels) -> Self {
        self.height = Some(height);
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

    /// Return whether playback is paused.
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// Pause or resume the orbit loop at runtime.
    pub fn set_paused(&mut self, paused: bool, cx: &mut Context<Self>) {
        if self.paused != paused {
            self.paused = paused;
            cx.emit(IntegrationFlowEvent::PausedChanged(paused));
            cx.notify();
        }
    }

    /// Return the configured integration items.
    pub fn items(&self) -> &[IntegrationItem] {
        &self.items
    }

    /// Return whether skeleton loading is active.
    pub fn is_loading(&self) -> bool {
        self.loading
    }

    /// Toggle skeleton loading at runtime.
    pub fn set_loading(&mut self, loading: bool, cx: &mut Context<Self>) {
        if self.loading != loading {
            self.loading = loading;
            cx.notify();
        }
    }

    /// Return whether empty state is active.
    pub fn is_empty(&self) -> bool {
        self.empty
    }

    /// Toggle empty state at runtime.
    pub fn set_empty(&mut self, empty: bool, cx: &mut Context<Self>) {
        if self.empty != empty {
            self.empty = empty;
            cx.notify();
        }
    }
    fn launch_slot_packet(&mut self, slot: isize, cx: &mut Context<Self>) {
        let now = cx.background_executor().now();
        self.slot_packets
            .retain(|&(_, t0)| now.saturating_duration_since(t0).as_secs_f32() < 0.95);
        self.slot_packets.push((slot, now));
    }
}

impl Render for IntegrationFlow {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let dark = Theme::global(cx).mode == ThemeMode::Dark;
        let reduced = disabled_motion(cx);
        let now = cx.background_executor().now();
        let total_elapsed = now.saturating_duration_since(self.clock).as_secs_f32();

        if self.loading {
            let phase = if reduced {
                0.0
            } else {
                w.request_animation_frame();
                (total_elapsed / 1.4).fract()
            };
            return div()
                .id(pro_eid(&self.id, "loading"))
                .w_full()
                .h(self.height.unwrap_or(px(380.)))
                .rounded(px(18.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .flex()
                .items_center()
                .justify_around()
                .px(px(32.))
                .child(pro_skeleton_bar(210., 62., 31., phase, cx))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(18.))
                        .child(pro_skeleton_bar(176., 54., 27., phase, cx))
                        .child(pro_skeleton_bar(188., 62., 31., phase, cx))
                        .child(pro_skeleton_bar(176., 54., 27., phase, cx)),
                )
                .into_any_element();
        }

        if self.empty || self.items.is_empty() {
            return div()
                .id(pro_eid(&self.id, "empty"))
                .w_full()
                .child(pro_empty_state::<Self>(
                    ProIcon::Link,
                    tr(cx, "No integrations configured", "暂无集成连接项"),
                    tr(
                        cx,
                        "Provide integration items to animate data flowing into the Kense hub.",
                        "添加集成应用条目后即可展示数据流向 Kense 中枢的动效。",
                    ),
                    None,
                    cx,
                ))
                .into_any_element();
        }

        let animate = !reduced && !self.paused;
        let elapsed_s = if animate { total_elapsed } else { 0.45 };

        let measured_cell = self.measured_w.clone();
        let measured_h_cell = self.measured_h.clone();
        let stage_w = measured_cell.get().max(320.);
        let auto_h = (stage_w * (1490.0 / 2400.0)).round().max(200.);
        let stage_h = self.height.map(f32::from).unwrap_or_else(|| {
            let mh = measured_h_cell.get();
            if mh > 40.0 { mh } else { auto_h }
        });
        let scale = stage_w / ART_W;
        let y_art_shift = (stage_h - ART_H * scale) * 0.5;

        let default_bg: Hsla = if dark {
            rgb(0x1f1f22).into()
        } else {
            rgb(0xffffff).into()
        };
        let bg_hsla = self.background.unwrap_or(default_bg);
        let fade_hsla = self.fade_color.unwrap_or(default_bg);

        let bg_phase_q = if animate {
            ((elapsed_s / 16.0).fract() * 1000.0).round() as u16
        } else {
            180
        };
        let bg_key = CoralBgKey {
            dark,
            bg_rgb: hsla_to_rgb8(bg_hsla),
            fade_rgb: hsla_to_rgb8(fade_hsla),
            phase_q: bg_phase_q.min(999),
        };

        let n_items = self.items.len();
        let (step_idx, shift_frac, beam_prog, charge_glow) =
            evaluate_orbit_cycle(elapsed_s, n_items);
        let hovered_slot = self.hovered_slot;

        // Prune completed manual packets and compute active packet progress `(slot, prog)` in `0.0..=1.0`
        let packet_dur = 0.92f32;
        self.slot_packets
            .retain(|&(_, t0)| now.saturating_duration_since(t0).as_secs_f32() < packet_dur);
        let active_packets: Vec<(isize, f32)> = if reduced {
            Vec::new()
        } else {
            self.slot_packets
                .iter()
                .map(|&(sl, t0)| {
                    let u = (now.saturating_duration_since(t0).as_secs_f32() / packet_dur)
                        .clamp(0., 1.);
                    (sl, pro_ease(u))
                })
                .collect()
        };
        let manual_hub_glow = active_packets
            .iter()
            .map(|&(_, pt)| ((pt - 0.55) / 0.45).clamp(0., 1.) * (1.0 - pt) * 3.2)
            .fold(0.0f32, f32::max)
            .clamp(0., 1.);

        let mut slot_illums = [0.0f32; 5];
        let mut slot_downs = [0.0f32; 5];
        for (si, slot) in (-2isize..=2isize).enumerate() {
            let item_idx = (step_idx as isize + 2 + slot).rem_euclid(n_items as isize) as usize;
            let is_sel = self
                .selected_id
                .as_ref()
                .is_some_and(|sid| *sid == self.items[item_idx].id);
            let has_pkt = active_packets.iter().any(|&(ps, _)| ps == slot);
            let target_illum = if hovered_slot == Some(slot) {
                1.0
            } else if is_sel || has_pkt {
                0.78
            } else {
                0.0
            };
            self.slot_illum[si].duration(target_illum, 0.20, cx);
            slot_illums[si] = self.slot_illum[si].get(w, cx).clamp(0., 1.);
            slot_downs[si] = self.slot_press[si].get(w, cx).clamp(0., 1.);
        }
        let hub_down = self.hub_press.get(w, cx).clamp(0., 1.);
        let any_packets = !active_packets.is_empty();
        let bg_cache = self.bg_cache.clone();

        // Layer 0 + Layer 1: Full-bleed coral breathing radial backdrop + dashed orbit arc + curved inward beams
        let stage_canvas = canvas(
            move |bounds, window, _| {
                let bw = f32::from(bounds.size.width);
                let bh = f32::from(bounds.size.height);
                if bw > 40. && bh > 40. {
                    let changed = (bw - measured_cell.get()).abs() > 1.0
                        || (bh - measured_h_cell.get()).abs() > 1.0;
                    if changed {
                        measured_cell.set(bw);
                        measured_h_cell.set(bh);
                        window.request_animation_frame();
                    }
                }
            },
            move |bounds, _, window, _| {
                let bw = f32::from(bounds.size.width);
                let bh = f32::from(bounds.size.height);
                if bw <= 4. || bh <= 4. {
                    return;
                }
                let sc = bw / ART_W;
                let y_off = (bh - ART_H * sc) * 0.5;
                let at = |ax: f32, ay: f32| -> Point<Pixels> {
                    point(
                        bounds.origin.x + px(ax * sc),
                        bounds.origin.y + px(y_off + ay * sc),
                    )
                };

                // 1) Paint full-bleed breathing coral/peach backdrop + bottom fade (zero corner clipping)
                let bg_img = bg_cache.borrow_mut().render(bg_key, window);
                let _ = window.paint_image(bounds, bounds, Corners::default(), bg_img, 0, false);

                // 2) Paint dashed elliptical orbit arc (`cx = 1860, cy = 559, rx = 757.5, ry = 776.6`)
                let orbit_c: Hsla = if dark {
                    rgb(0xffd9cc).into()
                } else {
                    rgb(0xffffff).into()
                };
                let dash_count = 64usize;
                for i in 0..dash_count {
                    let a0 = std::f32::consts::PI * (0.64 + 0.72 * (i as f32 / dash_count as f32));
                    let a1 = std::f32::consts::PI
                        * (0.64 + 0.72 * ((i as f32 + 0.52) / dash_count as f32));
                    let y_screen = y_off + (559.0 + 776.625 * a0.sin()) * sc;
                    let ny = (y_screen / bh).clamp(-0.2, 1.2);
                    let top_f = smooth_taper(0.0, 0.10, ny);
                    let bot_f = 1.0 - smooth_taper(0.60, 0.94, ny);
                    let edge_fade = (top_f * bot_f).clamp(0., 1.);
                    if edge_fade <= 0.01 {
                        continue;
                    }
                    let p0 = at(1860.0 + 757.5 * a0.cos(), 559.0 + 776.625 * a0.sin());
                    let p1 = at(1860.0 + 757.5 * a1.cos(), 559.0 + 776.625 * a1.sin());
                    let mut seg = PathBuilder::stroke(px((4.2 * sc).max(1.2)));
                    seg.move_to(p0);
                    seg.line_to(p1);
                    if let Ok(path) = seg.build() {
                        window.paint_path(path, orbit_c.opacity(0.58 * edge_fade));
                    }
                }

                // 3) Curved inward guide beams from all 5 visible orbit slots (`-2..=2`) into the Kense hub (`x = 780, y = 559`)
                let hub_in = (780.0f32, 559.0f32);
                for (si, slot) in (-2isize..=2isize).enumerate() {
                    let s_f = slot as f32 - shift_frac;
                    let (dx, dy, _) = orbit_pill_offset(s_f);
                    let src = (1020.0 + dx, 559.0 + dy);
                    let mid_x = (src.0 + hub_in.0) * 0.5;
                    let illum = slot_illums[si];

                    let rest_a = match slot {
                        0 => 0.28,
                        -1 | 1 => 0.12,
                        _ => 0.04,
                    };
                    let base_a = rest_a + (0.76 - rest_a) * illum;
                    if base_a <= 0.01 {
                        continue;
                    }
                    let stroke_w = ((2.6 + 2.2 * illum) * sc).max(1.0);
                    let mut curve = PathBuilder::stroke(px(stroke_w));
                    curve.move_to(at(src.0, src.1));
                    curve.cubic_bezier_to(
                        at(hub_in.0, hub_in.1),
                        at(mid_x, src.1),
                        at(mid_x, hub_in.1),
                    );
                    if let Ok(path) = curve.build() {
                        window.paint_path(path, white().opacity(base_a));
                    }

                    let eval_bz = |t: f32| -> Point<Pixels> {
                        let inv = 1. - t;
                        let bx = inv * inv * inv * src.0
                            + 3. * inv * inv * t * mid_x
                            + 3. * inv * t * t * mid_x
                            + t * t * t * hub_in.0;
                        let by = inv * inv * inv * src.1
                            + 3. * inv * inv * t * src.1
                            + 3. * inv * t * t * hub_in.1
                            + t * t * t * hub_in.1;
                        at(bx, by)
                    };

                    // Collect both ambient center/hover stream and immediate click/hover packet bursts on this slot
                    let mut pulses: Vec<f32> = Vec::with_capacity(3);
                    if slot == 0
                        && let Some(bp) = beam_prog
                    {
                        pulses.push(bp);
                    } else if hovered_slot == Some(slot) && animate {
                        pulses.push((total_elapsed * 0.95).fract());
                    }
                    for &(ps, prog) in &active_packets {
                        if ps == slot {
                            pulses.push(prog);
                        }
                    }

                    for pt in pulses {
                        let seg_u0 = (pt * 1.25 - 0.25).clamp(0., 1.);
                        let seg_u1 = (pt * 1.25).clamp(0., 1.);
                        if seg_u1 > seg_u0 + 0.01 {
                            let pulse_alpha =
                                (1.0 - ((pt - 0.5) * 1.8).abs().max(0.)).clamp(0.22, 1.0);
                            for (w_mul, a_mul) in [(10.5, 0.30), (4.4, 0.96)] {
                                let mut beam = PathBuilder::stroke(px((w_mul * sc).max(1.5)));
                                for k in 0..=8 {
                                    let tk = seg_u0 + (seg_u1 - seg_u0) * (k as f32 / 8.);
                                    let q = eval_bz(tk);
                                    if k == 0 {
                                        beam.move_to(q);
                                    } else {
                                        beam.line_to(q);
                                    }
                                }
                                if let Ok(path) = beam.build() {
                                    window.paint_path(path, white().opacity(pulse_alpha * a_mul));
                                }
                            }
                        }
                    }
                }

                if animate || any_packets {
                    window.request_animation_frame();
                }
            },
        )
        .absolute()
        .inset_0();

        // Card surface palettes matching `light.png` and `dark.png`
        let light_card_top: Hsla = if dark {
            hsla(240. / 360., 0.05, 0.145, 0.92)
        } else {
            hsla(0., 0., 1., 0.86)
        };
        let light_card_bot: Hsla = if dark {
            hsla(240. / 360., 0.05, 0.12, 0.84)
        } else {
            hsla(0., 0., 1., 0.56)
        };
        let light_card_bdr: Hsla = if dark {
            hsla(0., 0., 1., 0.13)
        } else {
            hsla(0., 0., 1., 0.50)
        };
        let ghost_top: Hsla = if dark {
            hsla(0., 0., 1., 0.11)
        } else {
            hsla(0., 0., 1., 0.20)
        };
        let ghost_bot: Hsla = if dark {
            hsla(0., 0., 1., 0.05)
        } else {
            hsla(0., 0., 1., 0.10)
        };
        let ghost_bdr: Hsla = if dark {
            hsla(0., 0., 1., 0.09)
        } else {
            hsla(0., 0., 1., 0.20)
        };

        let active_title_c: Hsla = if dark {
            hsla(0., 0., 1., 0.93)
        } else {
            hsla(0., 0., 0., 0.84)
        };
        let active_sub_c: Hsla = if dark {
            hsla(0., 0., 1., 0.64)
        } else {
            hsla(0., 0., 0., 0.45)
        };
        let ghost_title_c: Hsla = hsla(0., 0., 1., 0.86);
        let ghost_sub_c: Hsla = hsla(0., 0., 1., 0.54);

        // Central Kense `Data Analysis` hub (`left = 137, top = 479.4, w = 642.6, h = 158.6` in artboard)
        let hub_x = 137.0 * scale;
        let hub_y = y_art_shift + 479.4 * scale;
        let hub_w = 642.6 * scale;
        let hub_h = 158.6 * scale;
        let badge_d = (118.6 * scale).max(28.);
        let pad_in = (20.0 * scale).max(6.);
        let title_fs = (37.0 * scale).clamp(11., 24.);
        let sub_fs = (33.5 * scale).clamp(10., 21.);

        let hub_title_str: SharedString = self
            .hub_title
            .clone()
            .unwrap_or_else(|| tr(cx, "Data Analysis", "数据分析中枢").into());
        let hub_sub_str: SharedString = self.hub_subtitle.clone().unwrap_or_else(|| {
            tr(
                cx,
                "Centralized Data Processing",
                "集中式实时数据汇聚与处理",
            )
            .into()
        });

        let kense_logo_px = (badge_d * 0.56).round().max(14.);
        let kense_hub_mark = pro_svg(
            "0 0 60 61",
            SNOWFLAKE_MONO_SVG,
            kense_logo_px,
            rgb(0x29b5e8).into(),
        );

        let badge_bg: Hsla = if dark {
            rgb(0x27272b).into()
        } else {
            rgb(0xffffff).into()
        };
        let badge_bdr: Hsla = if dark {
            hsla(0., 0., 1., 0.11)
        } else {
            hsla(0., 0., 0., 0.05)
        };
        let ghost_badge_bg: Hsla = if dark {
            hsla(240. / 360., 0.05, 0.17, 0.52)
        } else {
            hsla(0., 0., 1., 0.16)
        };
        let ghost_badge_bdr: Hsla = if dark {
            hsla(0., 0., 1., 0.12)
        } else {
            hsla(0., 0., 1., 0.18)
        };

        let total_hub_glow = (charge_glow + manual_hub_glow).clamp(0., 1.);
        let hub_inset = 2.0 * hub_down;
        let hub_el = div()
            .id(pro_eid(&self.id, "kense-hub"))
            .role(Role::Button)
            .aria_label(hub_title_str.clone())
            .absolute()
            .left(px(hub_x + hub_inset))
            .top(px(hub_y + hub_inset * 0.8))
            .w(px((hub_w - hub_inset * 2.0).max(40.)))
            .h(px((hub_h - hub_inset * 1.6).max(28.)))
            .rounded(px(9999.))
            .pl(px((52.0 * scale).max(12.)))
            .pr(px(pad_in))
            .bg(linear_gradient(
                180.,
                linear_color_stop(light_card_top, 0.),
                linear_color_stop(light_card_bot, 1.),
            ))
            .border_1()
            .border_color(light_card_bdr)
            .cursor_pointer()
            .shadow(vec![
                BoxShadow::new(
                    px(0.),
                    px(10. * scale),
                    black().opacity(if dark { 0.30 } else { 0.07 }),
                )
                .blur_radius(px(24. * scale)),
                BoxShadow::new(
                    px(0.),
                    px(0.),
                    white().opacity(total_hub_glow * if dark { 0.28 } else { 0.62 }),
                )
                .blur_radius(px(22. * scale)),
            ])
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.hub_press.duration(1., 0.08, cx);
                    cx.stop_propagation();
                    cx.notify();
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.hub_press.duration(0., 0.14, cx);
                    cx.notify();
                }),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.hub_press.duration(0., 0.14, cx);
                    cx.notify();
                }),
            )
            .on_click(cx.listener(|this, _, _, cx| {
                for sl in -1isize..=1isize {
                    this.launch_slot_packet(sl, cx);
                }
                cx.stop_propagation();
                cx.notify();
            }))
            .flex()
            .items_center()
            .justify_end()
            .gap(px((20.0 * scale).max(8.)))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .items_end()
                    .gap(px((4.0 * scale).max(1.5)))
                    .child(
                        div()
                            .max_w_full()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .text_size(px(title_fs))
                            .line_height(px(title_fs * 1.15))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(active_title_c)
                            .child(hub_title_str),
                    )
                    .child(
                        div()
                            .max_w_full()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .text_size(px(sub_fs))
                            .line_height(px(sub_fs * 1.15))
                            .text_color(active_sub_c)
                            .child(hub_sub_str),
                    ),
            )
            .child(
                div()
                    .size(px(badge_d))
                    .flex_none()
                    .rounded(px(9999.))
                    .bg(badge_bg)
                    .border_1()
                    .border_color(badge_bdr)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(kense_hub_mark),
            );

        let mut root = div()
            .id(self.id.clone())
            .role(Role::Image)
            .aria_label(self.label.clone().unwrap_or_else(|| {
                tr(
                    cx,
                    "Integrations like Slack, Okta, and Snowflake beaming data into the central Kense analysis hub",
                    "Slack、Okta、Snowflake 等集成应用沿轨道旋转并向中央 Kense 数据分析中枢传输光束",
                )
                .into()
            }))
            .track_focus(&self.focus)
            .tab_index(0)
            .relative()
            .w_full();
        root = match self.height {
            Some(explicit_h) => root.h(explicit_h),
            None => root.flex_1().h_full().min_h(px(auto_h)),
        };
        root = root
            .overflow_hidden()
            .cursor_pointer()
            .focus_visible(move |s| s.border_1().border_color(p.focus))
            .on_click(cx.listener(|this, _, _, cx| {
                this.paused = !this.paused;
                cx.emit(IntegrationFlowEvent::PausedChanged(this.paused));
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, e: &KeyDownEvent, _, cx| {
                if matches!(e.keystroke.key.as_str(), "enter" | "space" | " ") {
                    this.paused = !this.paused;
                    cx.emit(IntegrationFlowEvent::PausedChanged(this.paused));
                    cx.stop_propagation();
                    cx.notify();
                }
            }))
            .child(stage_canvas)
            .child(hub_el);

        // Render visible integration pills along the orbit (`slot` in `-2..=2`)
        let pill_w = 492.6 * scale;
        let pill_h = 158.6 * scale;
        let default_pill_title: SharedString = tr(cx, "API Integration", "API 集成").into();

        let items_snapshot: Vec<IntegrationItem> = self.items.clone();
        for (si, slot) in (-2isize..=2isize).enumerate() {
            let item_idx =
                (step_idx as isize + 2 + slot).rem_euclid(items_snapshot.len() as isize) as usize;
            let item = &items_snapshot[item_idx];

            let s_f = slot as f32 - shift_frac;
            let (dx, dy, slot_op) = orbit_pill_offset(s_f);
            let center_weight = (1.0 - s_f.abs()).clamp(0., 1.);
            let illum = slot_illums[si];
            let down = slot_downs[si];
            let active_mix = center_weight.max(illum);
            let is_center = active_mix > 0.52;

            let press_ix = 2.6 * down * scale * 2.2;
            let press_iy = 1.8 * down * scale * 2.2;
            let hover_lift = -2.2 * illum * (1.0 - down);
            let pill_x = (1020.0 + dx) * scale + press_ix;
            let pill_y = y_art_shift + (479.4 + dy) * scale + hover_lift + press_iy;

            // Smoothly taper satellite pill opacity to 0.0 near both top and bottom vertical stage edges
            let pill_mid_ny = ((pill_y + pill_h * 0.5) / stage_h.max(1.0)).clamp(-0.3, 1.3);
            let top_fade = smooth_taper(-0.02, 0.18, pill_mid_ny);
            let bottom_fade = 1.0 - smooth_taper(0.65, 0.90, pill_mid_ny);
            let final_op = (slot_op * top_fade * bottom_fade).clamp(0., 1.);
            if final_op <= 0.015 {
                continue;
            }

            let mark_px = (badge_d * 0.54 * (1.0 - 0.06 * down)).round().max(13.);
            let icon_el = render_integration_mark(item, is_center, dark, mark_px, w, cx);
            let cap_title = item
                .title
                .clone()
                .unwrap_or_else(|| default_pill_title.clone());
            let cap_sub = item.name.clone();
            let select_id = item.id.clone();

            let top_c = pro_mix_color(ghost_top, light_card_top, active_mix);
            let bot_c = pro_mix_color(ghost_bot, light_card_bot, active_mix);
            let bdr_c = pro_mix_color(
                ghost_bdr,
                pro_mix_color(light_card_bdr, white().opacity(0.85), down * 0.55),
                active_mix,
            );
            let t_col = pro_mix_color(ghost_title_c, active_title_c, active_mix);
            let s_col = pro_mix_color(ghost_sub_c, active_sub_c, active_mix);
            let b_bg = pro_mix_color(ghost_badge_bg, badge_bg, active_mix);
            let b_bdr = pro_mix_color(ghost_badge_bdr, badge_bdr, active_mix);

            let mut pill_el = div()
                .id(pro_eid(&self.id, format!("pill-slot-{}", slot + 3)))
                .role(Role::Button)
                .aria_label(cap_sub.clone())
                .absolute()
                .left(px(pill_x))
                .top(px(pill_y))
                .w(px((pill_w - press_ix * 2.0).max(32.)))
                .h(px((pill_h - press_iy * 2.0).max(24.)))
                .rounded(px(9999.))
                .pl(px(pad_in))
                .pr(px((52.0 * scale).max(12.)))
                .bg(linear_gradient(
                    180.,
                    linear_color_stop(top_c, 0.),
                    linear_color_stop(bot_c, 1.),
                ))
                .border_1()
                .border_color(bdr_c)
                .opacity(final_op)
                .cursor_pointer()
                .flex()
                .items_center()
                .gap(px((20.0 * scale).max(8.)))
                .on_hover(cx.listener(move |this, hov: &bool, _, cx| {
                    if *hov {
                        if this.hovered_slot != Some(slot) {
                            this.hovered_slot = Some(slot);
                            this.slot_illum[si].duration(1.0, 0.20, cx);
                            this.launch_slot_packet(slot, cx);
                            cx.notify();
                        }
                    } else if this.hovered_slot == Some(slot) {
                        this.hovered_slot = None;
                        this.slot_illum[si].duration(0.0, 0.20, cx);
                        this.slot_press[si].duration(0., 0.14, cx);
                        cx.notify();
                    }
                }))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _, _, cx| {
                        this.slot_press[si].duration(1., 0.08, cx);
                        cx.stop_propagation();
                        cx.notify();
                    }),
                )
                .on_mouse_up(
                    MouseButton::Left,
                    cx.listener(move |this, _, _, cx| {
                        this.slot_press[si].duration(0., 0.14, cx);
                        cx.notify();
                    }),
                )
                .on_mouse_up_out(
                    MouseButton::Left,
                    cx.listener(move |this, _, _, cx| {
                        this.slot_press[si].duration(0., 0.14, cx);
                        cx.notify();
                    }),
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.selected_id = Some(select_id.clone());
                    this.launch_slot_packet(slot, cx);
                    cx.emit(IntegrationFlowEvent::Select(select_id.clone()));
                    cx.stop_propagation();
                    cx.notify();
                }))
                .child(
                    div()
                        .size(px(badge_d))
                        .flex_none()
                        .rounded(px(9999.))
                        .bg(b_bg)
                        .border_1()
                        .border_color(b_bdr)
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(icon_el),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap(px((4.0 * scale).max(1.5)))
                        .child(
                            div()
                                .max_w_full()
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .text_ellipsis()
                                .text_size(px(title_fs))
                                .line_height(px(title_fs * 1.15))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(t_col)
                                .child(cap_title),
                        )
                        .child(
                            div()
                                .max_w_full()
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .text_ellipsis()
                                .text_size(px(sub_fs))
                                .line_height(px(sub_fs * 1.15))
                                .text_color(s_col)
                                .child(cap_sub),
                        ),
                );

            if active_mix > 0.08 {
                let glow_amt = if slot == 0 {
                    charge_glow.max(illum * 0.65)
                } else {
                    illum * 0.72
                };
                pill_el = pill_el.shadow(vec![
                    BoxShadow::new(
                        px(0.),
                        px(10. * scale * active_mix),
                        black().opacity((if dark { 0.30 } else { 0.07 }) * active_mix),
                    )
                    .blur_radius(px(24. * scale)),
                    BoxShadow::new(
                        px(0.),
                        px(0.),
                        white().opacity(glow_amt * if dark { 0.22 } else { 0.55 }),
                    )
                    .blur_radius(px(22. * scale)),
                ]);
            }

            root = root.child(pill_el);
        }

        root.into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BG_TEX_H, BG_TEX_W, CoralBgKey, IntegrationFlow, IntegrationFlowEvent,
        default_integration_items, evaluate_orbit_cycle, orbit_pill_offset,
        rasterize_coral_backdrop,
    };
    use gpui::{
        MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, TestAppContext, point, px, size,
    };
    use std::{cell::RefCell, rc::Rc, time::Duration};

    #[test]
    fn default_orbit_places_slack_at_center_and_cycles_beam_then_step() {
        let items = default_integration_items();
        assert_eq!(items.len(), 5);
        // At step_idx=0, slot=0 maps to `(0 + 2 + 0) % 5 = 2` -> "Slack" in center!
        assert_eq!(items[2].name.as_ref(), "Slack");

        let (step0, shift0, beam0, charge0) = evaluate_orbit_cycle(0.55, 5);
        assert_eq!(step0, 0);
        assert_eq!(shift0, 0.0);
        assert!(beam0.is_some());
        assert!(charge0 > 0.7);

        let (step1, shift1, beam1, _) = evaluate_orbit_cycle(1.72, 5);
        assert_eq!(step1, 0);
        assert!(beam1.is_none());
        assert!(shift1 > 0.2 && shift1 < 0.9);

        let (dx0, dy0, op0) = orbit_pill_offset(0.0);
        assert_eq!(dx0, 0.0);
        assert_eq!(dy0, 0.0);
        assert_eq!(op0, 1.0);

        let (dx1, dy1, _) = orbit_pill_offset(1.0);
        assert!(dx1 > 40.0 && dy1 > 250.0);
    }

    #[test]
    fn coral_backdrop_warms_upper_stage_and_fades_cleanly_at_bottom() {
        let light = rasterize_coral_backdrop(CoralBgKey {
            dark: false,
            bg_rgb: [0xff, 0xff, 0xff],
            fade_rgb: [0xff, 0xff, 0xff],
            phase_q: 250,
        });
        let bytes = light.as_bytes(0).expect("BGRA coral bytes");
        // Upper-left coral blob (`R > B + 60`)
        let upper = (25 * BG_TEX_W + 30) * 4;
        assert!(
            bytes[upper + 2] > bytes[upper] + 60,
            "expected warm coral R > B, got R={} B={}",
            bytes[upper + 2],
            bytes[upper]
        );
        // Bottom edge fades cleanly into `#ffffff`
        let bot = ((BG_TEX_H - 1) * BG_TEX_W + BG_TEX_W / 2) * 4;
        assert!(bytes[bot] > 248 && bytes[bot + 1] > 248 && bytes[bot + 2] > 248);
    }

    #[gpui::test]
    fn hovering_and_clicking_satellite_illuminates_connector_and_fires_packet(
        cx: &mut TestAppContext,
    ) {
        cx.update(crate::init);
        let exec = cx.executor();
        let events: Rc<RefCell<Vec<IntegrationFlowEvent>>> = Rc::new(RefCell::new(Vec::new()));
        let ev_sink = events.clone();

        let (view, vcx) = cx.add_window_view(|_, cx| {
            let flow = IntegrationFlow::new("if-sat-test", cx).with_paused(true);
            cx.subscribe_self(move |_, ev: &IntegrationFlowEvent, _| {
                ev_sink.borrow_mut().push(ev.clone());
            })
            .detach();
            flow
        });
        vcx.simulate_resize(size(px(800.), px(520.)));
        vcx.update(|w, app| w.simulate_next_frame(app));
        vcx.update(|w, app| w.simulate_next_frame(app));
        vcx.run_until_parked();

        // Compute upper satellite (`slot = -1`, Snowflake) center in stage coordinates
        let sat_pt = vcx.update(|_, app| {
            let sw = view.read(app).measured_w.get();
            let mh = view.read(app).measured_h.get();
            let sh = if mh > 40.0 {
                mh
            } else {
                (sw * (1490.0 / 2400.0)).round().max(200.)
            };
            let sc = sw / 1800.0;
            let y_off = (sh - 1117.5 * sc) * 0.5;
            let (dx, dy, _) = orbit_pill_offset(-1.0);
            point(
                px((1020.0 + dx + 220.0) * sc),
                px(y_off + (479.4 + dy + 78.0) * sc),
            )
        });

        vcx.simulate_event(MouseMoveEvent {
            position: sat_pt,
            pressed_button: None,
            modifiers: Default::default(),
        });
        exec.advance_clock(Duration::from_millis(160));
        vcx.update(|w, app| w.simulate_next_frame(app));
        vcx.run_until_parked();

        vcx.update(|_, app| {
            let flow = view.read(app);
            assert_eq!(flow.hovered_slot, Some(-1));
            assert!(!flow.slot_packets.is_empty());
            assert!(
                flow.slot_illum[1].value(app) > 0.6,
                "expected upper satellite connector illumination > 0.6"
            );
        });

        // Click upper satellite -> selects Snowflake without toggling paused state!
        vcx.simulate_event(MouseDownEvent {
            position: sat_pt,
            button: MouseButton::Left,
            modifiers: Default::default(),
            click_count: 1,
            first_mouse: false,
        });
        vcx.simulate_event(MouseUpEvent {
            position: sat_pt,
            button: MouseButton::Left,
            modifiers: Default::default(),
            click_count: 1,
        });
        vcx.run_until_parked();

        vcx.update(|_, app| {
            let flow = view.read(app);
            assert!(flow.is_paused());
            assert_eq!(flow.selected_id.as_deref(), Some("snowflake"));
        });
        assert!(
            events
                .borrow()
                .iter()
                .any(|e| *e == IntegrationFlowEvent::Select("snowflake".into()))
        );
    }
}
