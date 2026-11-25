//! Isometric design-tool inspection scene of an AI chat composer.
//!
//! Renders an AI prompt composer (and optional faded predecessor above it) tilted in
//! 3D perspective with a `#0190fe` Figma-style selection frame, diagonal padding hatch,
//! four corner resize handles, dimension badges, and an animated `#ff3b30` spacing guide
//! stepping through the prompt input, toolbar, mode selector, and footer/send regions.

use crate::{
    components::pro_chrome::{
        ProPalette, pro_ease, pro_eid, pro_empty_state, pro_skeleton_bar, pro_swap_label, tr,
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
    rc::Rc,
    sync::Arc,
};
use web_time::Instant;

const STAGE_W: usize = 504;
const STAGE_H: usize = 404;
const RASTER_SCALE: usize = 2;
const TEX_W: usize = STAGE_W * RASTER_SCALE;
const TEX_H: usize = STAGE_H * RASTER_SCALE;
const COMP_W: f32 = 320.0;
const WRITE_H: f32 = 106.0;
const BODY_H: f32 = 142.0;
const PERSP_SCALE: f32 = 1.15;
const ABOVE_GAP: f32 = 20.0;

const LAYER_TAG_SVG: &str = r#"<rect x="1.5" y="2" width="9" height="3" rx="1" fill="currentColor"></rect><rect x="1.5" y="7" width="6" height="3" rx="1" fill="currentColor"></rect>"#;

const PLUS_THIN_SVG: &str = r#"<path fill="currentColor" d="M8 2.5c.28 0 .5.22.5.5v4.5H13a.5.5 0 0 1 0 1H8.5V13a.5.5 0 0 1-1 0V8.5H3a.5.5 0 0 1 0-1h4.5V3c0-.28.22-.5.5-.5Z"></path>"#;

const HAND_MODE_SVG: &str = r#"<path d="M52.5305 32.6557V24.8463C52.5305 22.1016 54.7555 19.8766 57.5001 19.8766C60.2448 19.8766 62.4698 22.1016 62.4698 24.8463V42.027C62.4698 53.3194 53.3154 62.4735 42.0233 62.4735H34.2388M22.7127 39.7552L15.94 31.8537C14.8314 30.5602 13.2129 29.8159 11.5094 29.8159H11.1235C8.11379 29.8159 5.67395 32.2558 5.67395 35.2655C5.67395 36.3412 5.99241 37.393 6.58919 38.2881L16.4118 53.0221C20.3482 58.9269 26.9753 62.4735 34.0719 62.4735" stroke="currentColor" stroke-width="4.7" stroke-linecap="round" stroke-linejoin="round"></path><path d="M52.534 24.1378V16.3284C52.534 13.5837 50.3091 11.3588 47.5644 11.3588C44.8197 11.3588 42.5948 13.5837 42.5948 16.3284V29.8174" stroke="currentColor" stroke-width="4.7" stroke-linecap="round" stroke-linejoin="round"></path><path d="M42.5958 29.8198V10.6513C42.5958 7.90661 40.3708 5.68163 37.6262 5.68163C34.8815 5.68163 32.6565 7.90661 32.6565 10.6513V28.4" stroke="currentColor" stroke-width="4.7" stroke-linecap="round" stroke-linejoin="round"></path><path d="M32.6576 28.3975V16.3284C32.6576 13.5837 30.4326 11.3588 27.6879 11.3588C24.9433 11.3588 22.7183 13.5837 22.7183 16.3284V39.7567" stroke="currentColor" stroke-width="4.7" stroke-linecap="round" stroke-linejoin="round"></path>"#;

const FOLDER_MINI_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="36" height="36" viewBox="0 0 67 67" fill="none"><path d="M11.17 15.35v15.36h44.67v-6.98c0-3.86-3.13-6.98-6.98-6.98H33.5l-3.93-5.89a5.58 5.58 0 0 0-4.64-2.49h-6.78c-3.86 0-6.98 3.13-6.98 6.98Z" fill="#00A1FF"/><path d="M33.5 16.75l-3.93-5.89a5.58 5.58 0 0 0-4.64-2.49h-6.78c-3.86 0-6.98 3.13-6.98 6.98v15.36h44.67v-6.98c0-3.86-3.13-6.98-6.98-6.98H33.5Zm0 0h-9.77" stroke="#00A1FF" stroke-width="4.19" stroke-linecap="round" stroke-linejoin="round"/><path d="M6.59 42.38c-1.05-5.26-1.57-7.89-.08-9.67l.31-.34c1.66-1.66 4.59-1.66 10.45-1.66h32.46c5.86 0 8.79 0 10.45 1.66l.31.34c1.49 1.78.97 4.41-.08 9.67-1.51 7.56-2.26 11.34-5.1 13.66l-.6.46c-3.01 2.13-7.23 2.13-15.66 2.13h-11.1c-8.43 0-12.64 0-15.66-2.13l-.6-.46c-2.84-2.32-3.6-6.1-5.1-13.66Z" fill="#3CC7FF" stroke="#3CC7FF" stroke-width="4.19"/></svg>"##;

const CLICKUP_MINI_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="32" height="32" viewBox="0 0 50 60" fill="none"><path fill-rule="evenodd" clip-rule="evenodd" d="M0 45.9L9.16 38.82C14.06 45.17 19.23 48.17 24.95 48.17C30.66 48.17 35.74 45.27 40.37 38.92L49.71 45.81C43.0 54.88 34.65 59.69 24.95 59.69C15.33 59.69 6.89 54.88 0 45.9Z" fill="#7B68EE"/><path fill-rule="evenodd" clip-rule="evenodd" d="M24.95 15.33L8.63 29.39L1.1 20.59L25.04 0L48.81 20.59L41.19 29.3L24.95 15.33Z" fill="#FF4081"/></svg>"##;

const HAND_CURSOR_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="44" height="44" viewBox="0 0 24 24"><path d="M9 11V4.5a1.5 1.5 0 0 1 3 0V10m0-1.5a1.5 1.5 0 0 1 3 0V11m0-1a1.5 1.5 0 0 1 3 0v1.5m0 0a1.5 1.5 0 0 1 3 0V15a7 7 0 0 1-7 7h-1.2a6 6 0 0 1-4.6-2.2L4.6 16.4a1.6 1.6 0 0 1 2.4-2.1L9 16V11" fill="#ffffff" stroke="#111111" stroke-width="1.35" stroke-linejoin="round"/></svg>"##;

/// Inspected layout region inside [`InspectComposer`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InspectComposerRegion {
    /// Prompt text input region (`W Fill 300 px`, `14px` vertical spacing).
    #[default]
    Message,
    /// Action toolbar row containing `+`, mode selector, microphone, and send button (`W Fill 300 px`, `14px` gap).
    Toolbar,
    /// Approval / model mode selector chip (`Auto`, `W Hug 88 px`, `8px` gap).
    Selector,
    /// Send & connected-apps footer region (`W Fill 308 px`, `10px` gap).
    SendFooter,
}

impl InspectComposerRegion {
    const ALL: [Self; 4] = [
        Self::Message,
        Self::Toolbar,
        Self::Selector,
        Self::SendFooter,
    ];

    fn from_index(idx: usize) -> Self {
        Self::ALL[idx % Self::ALL.len()]
    }

    fn index(self) -> usize {
        match self {
            Self::Message => 0,
            Self::Toolbar => 1,
            Self::Selector => 2,
            Self::SendFooter => 3,
        }
    }

    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::Message => tr(cx, "Message", "输入框"),
            Self::Toolbar => tr(cx, "Toolbar", "工具栏"),
            Self::Selector => tr(cx, "Mode", "模式器"),
            Self::SendFooter => tr(cx, "Send & Apps", "发送与连接"),
        }
    }
}

/// Events emitted by [`InspectComposer`] on region pin or playback toggle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InspectComposerEvent {
    /// Emitted when clicking pins or unpins a composer region.
    PinRegion(Option<InspectComposerRegion>),
    /// Emitted when playback is toggled.
    PlayingChanged(bool),
}

impl EventEmitter<InspectComposerEvent> for InspectComposer {}

#[derive(Clone, Copy)]
struct RegionBox {
    fx: f32,
    fy: f32,
    fw: f32,
    fh: f32,
    pad: f32,
    guide_x: f32,
    guide_y0: f32,
    guide_y1: f32,
    gap_label: &'static str,
    dim_prefix: &'static str,
    dim_px: &'static str,
}

fn region_box(region: InspectComposerRegion, mirror: bool) -> RegionBox {
    let gx_open = if mirror { 216.0 } else { 14.0 };
    match region {
        InspectComposerRegion::Message => RegionBox {
            fx: 0.0,
            fy: 0.0,
            fw: COMP_W,
            fh: 64.0,
            pad: 10.0,
            guide_x: gx_open,
            guide_y0: 54.0,
            guide_y1: 68.0,
            gap_label: "14px",
            dim_prefix: "W Fill",
            dim_px: "300 px",
        },
        InspectComposerRegion::Toolbar => RegionBox {
            fx: 0.0,
            fy: 58.0,
            fw: COMP_W,
            fh: 48.0,
            pad: 10.0,
            guide_x: gx_open,
            guide_y0: 54.0,
            guide_y1: 68.0,
            gap_label: "14px",
            dim_prefix: "W Fill",
            dim_px: "300 px",
        },
        InspectComposerRegion::Selector => RegionBox {
            fx: 40.0,
            fy: 64.0,
            fw: 96.0,
            fh: 36.0,
            pad: 6.0,
            guide_x: 132.0,
            guide_y0: 56.0,
            guide_y1: 64.0,
            gap_label: "8px",
            dim_prefix: "W Hug",
            dim_px: "84 px",
        },
        InspectComposerRegion::SendFooter => RegionBox {
            fx: 0.0,
            fy: 104.0,
            fw: COMP_W,
            fh: 38.0,
            pad: 6.0,
            guide_x: gx_open,
            guide_y0: 96.0,
            guide_y1: 106.0,
            gap_label: "10px",
            dim_prefix: "W Fill",
            dim_px: "308 px",
        },
    }
}

#[derive(Clone, Copy)]
struct ComposerTargetBox {
    rb: RegionBox,
    region: InspectComposerRegion,
    sub_tag: Option<(&'static str, &'static str)>,
}

impl ComposerTargetBox {
    fn tag_label(self, cx: &App) -> &'static str {
        if let Some((en, zh)) = self.sub_tag {
            tr(cx, en, zh)
        } else {
            self.region.label(cx)
        }
    }
}

fn composer_target_box(target_idx: usize, mirror: bool) -> ComposerTargetBox {
    match target_idx {
        0..=3 => {
            let region = InspectComposerRegion::from_index(target_idx);
            ComposerTargetBox {
                rb: region_box(region, mirror),
                region,
                sub_tag: None,
            }
        }
        4 => ComposerTargetBox {
            rb: RegionBox {
                fx: 6.0,
                fy: 64.0,
                fw: 34.0,
                fh: 36.0,
                pad: 3.0,
                guide_x: 118.0,
                guide_y0: 56.0,
                guide_y1: 64.0,
                gap_label: "8px",
                dim_prefix: "W Fixed",
                dim_px: "28 × 28 px",
            },
            region: InspectComposerRegion::Toolbar,
            sub_tag: Some(("Attach +", "添加附件")),
        },
        5 => ComposerTargetBox {
            rb: RegionBox {
                fx: 244.0,
                fy: 64.0,
                fw: 34.0,
                fh: 36.0,
                pad: 3.0,
                guide_x: 206.0,
                guide_y0: 56.0,
                guide_y1: 64.0,
                gap_label: "8px",
                dim_prefix: "W Fixed",
                dim_px: "28 × 28 px",
            },
            region: InspectComposerRegion::Toolbar,
            sub_tag: Some(("Voice Mic", "语音输入")),
        },
        6 => ComposerTargetBox {
            rb: RegionBox {
                fx: 278.0,
                fy: 64.0,
                fw: 34.0,
                fh: 36.0,
                pad: 3.0,
                guide_x: 214.0,
                guide_y0: 56.0,
                guide_y1: 64.0,
                gap_label: "8px",
                dim_prefix: "W Fixed",
                dim_px: "28 × 28 px",
            },
            region: InspectComposerRegion::Toolbar,
            sub_tag: Some(("Send Button", "发送按钮")),
        },
        7 => ComposerTargetBox {
            rb: RegionBox {
                fx: 8.0,
                fy: 107.0,
                fw: 132.0,
                fh: 30.0,
                pad: 4.0,
                guide_x: 146.0,
                guide_y0: 98.0,
                guide_y1: 107.0,
                gap_label: "9px",
                dim_prefix: "W Hug",
                dim_px: "124 px",
            },
            region: InspectComposerRegion::SendFooter,
            sub_tag: Some(("Folder Scope", "目录范围")),
        },
        _ => ComposerTargetBox {
            rb: RegionBox {
                fx: 154.0,
                fy: 107.0,
                fw: 158.0,
                fh: 30.0,
                pad: 4.0,
                guide_x: 248.0,
                guide_y0: 98.0,
                guide_y1: 107.0,
                gap_label: "9px",
                dim_prefix: "W Hug",
                dim_px: "150 px",
            },
            region: InspectComposerRegion::SendFooter,
            sub_tag: Some(("Connect Apps", "连接应用")),
        },
    }
}

#[derive(Clone, Copy)]
struct ComposerTilt {
    flat: bool,
    mirror: bool,
    ox: f32,
    oy: f32,
}

impl ComposerTilt {
    fn new(flat: bool, mirror: bool) -> Self {
        let (ox, oy) = if flat {
            (58.0, 150.0)
        } else if mirror {
            (38.0, 172.0)
        } else {
            (96.0, 172.0)
        };
        Self {
            flat,
            mirror,
            ox,
            oy,
        }
    }

    fn scale(self) -> f32 {
        if self.flat { 1.0 } else { PERSP_SCALE }
    }

    fn project(self, u: f32, v: f32) -> (f32, f32) {
        if self.flat {
            return (self.ox + u, self.oy + v);
        }
        let s = PERSP_SCALE;
        if self.mirror {
            let u_rel = u - COMP_W;
            let xp = 0.963_063 * u_rel + 0.118_249 * v;
            let yp = 0.014_517 * u_rel + 0.874_312 * v;
            let wp = (1.0 + 0.000_244 * u_rel - 0.000_428 * v).max(0.25);
            (
                self.ox + COMP_W * s + (xp / wp) * s,
                self.oy + (yp / wp) * s,
            )
        } else {
            let xp = 0.963_063 * u - 0.118_249 * v;
            let yp = -0.014_517 * u + 0.874_312 * v;
            let wp = (1.0 - 0.000_244 * u - 0.000_428 * v).max(0.25);
            (self.ox + (xp / wp) * s, self.oy + (yp / wp) * s)
        }
    }

    fn unproject(self, sx: f32, sy: f32) -> (f32, f32) {
        if self.flat {
            return (sx - self.ox, sy - self.oy);
        }
        let s = PERSP_SCALE;
        if self.mirror {
            let dx = (sx - (self.ox + COMP_W * s)) / s;
            let dy = (sy - self.oy) / s;
            let a11 = 0.963_063 - 0.000_244 * dx;
            let a12 = 0.118_249 + 0.000_428 * dx;
            let a21 = 0.014_517 - 0.000_244 * dy;
            let a22 = 0.874_312 + 0.000_428 * dy;
            let det = a11 * a22 - a12 * a21;
            if det.abs() < 1e-6 {
                return (0., 0.);
            }
            let u_rel = (dx * a22 - dy * a12) / det;
            let v = (a11 * dy - a21 * dx) / det;
            (u_rel + COMP_W, v)
        } else {
            let dx = (sx - self.ox) / s;
            let dy = (sy - self.oy) / s;
            let a11 = 0.963_063 + 0.000_244 * dx;
            let a12 = -0.118_249 + 0.000_428 * dx;
            let a21 = -0.014_517 + 0.000_244 * dy;
            let a22 = 0.874_312 + 0.000_428 * dy;
            let det = a11 * a22 - a12 * a21;
            if det.abs() < 1e-6 {
                return (0., 0.);
            }
            let u = (dx * a22 - dy * a12) / det;
            let v = (a11 * dy - a21 * dx) / det;
            (u, v)
        }
    }
}

fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0., 1.);
    t * t * (3. - 2. * t)
}

fn stage_mask_alpha(sx: f32, sy: f32, mirror: bool) -> f32 {
    let mx = if mirror {
        smoothstep(8.0, STAGE_W as f32 * 0.27, sx)
    } else {
        1. - smoothstep(STAGE_W as f32 * 0.66, STAGE_W as f32 * 0.94, sx)
    };
    let my_bot = 1. - smoothstep(STAGE_H as f32 * 0.64, STAGE_H as f32 * 0.87, sy);
    let my_top = smoothstep(14.0, STAGE_H as f32 * 0.25, sy);
    (mx * my_bot * my_top).clamp(0., 1.)
}

fn sd_round_rect(px_x: f32, px_y: f32, x0: f32, y0: f32, x1: f32, y1: f32, r: f32) -> f32 {
    let cx = (x0 + x1) * 0.5;
    let cy = (y0 + y1) * 0.5;
    let hx = (x1 - x0) * 0.5 - r;
    let hy = (y1 - y0) * 0.5 - r;
    let dx = (px_x - cx).abs() - hx;
    let dy = (px_y - cy).abs() - hy;
    dx.max(0.).hypot(dy.max(0.)) + dx.max(dy).min(0.) - r
}

fn blend_over(dst: [f32; 4], src_rgb: [f32; 3], src_a: f32) -> [f32; 4] {
    let sa = src_a.clamp(0., 1.);
    if sa <= 1e-5 {
        return dst;
    }
    let inv = 1. - sa;
    [
        src_rgb[0] * sa + dst[0] * inv,
        src_rgb[1] * sa + dst[1] * inv,
        src_rgb[2] * sa + dst[2] * inv,
        sa + dst[3] * inv,
    ]
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct ComposerPlateKey {
    dark: bool,
    has_above: bool,
    mirror: bool,
    flat: bool,
    fx_q: i32,
    fy_q: i32,
    fw_q: i32,
    fh_q: i32,
    pad_q: i32,
    gx_q: i32,
    gy0_q: i32,
    gy1_q: i32,
    reveal_q: u16,
}

struct ComposerPlateCache {
    key: Option<ComposerPlateKey>,
    image: Option<Arc<RenderImage>>,
}

impl ComposerPlateCache {
    fn new() -> Self {
        Self {
            key: None,
            image: None,
        }
    }

    fn render(&mut self, key: ComposerPlateKey, window: &mut Window) -> Arc<RenderImage> {
        if self.key == Some(key)
            && let Some(existing) = &self.image
        {
            return existing.clone();
        }
        let img = Arc::new(rasterize_composer_plate(key));
        if let Some(prev) = self.image.replace(img.clone()) {
            let _ = window.drop_image(prev);
        }
        self.key = Some(key);
        img
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_composer_shell(
    mut c: [f32; 4],
    u: f32,
    v: f32,
    px_u: f32,
    fade: f32,
    tray_rgb: [f32; 3],
    card_rgb: [f32; 3],
    border_rgb: [f32; 3],
    border_a: f32,
    send_fill_a: f32,
) -> [f32; 4] {
    let d_body = sd_round_rect(u, v, 0.0, 0.0, COMP_W, BODY_H, 16.0);
    let cov_body = (0.5 - d_body / px_u).clamp(0., 1.) * fade;
    if cov_body > 0.0 {
        c = blend_over(c, tray_rgb, cov_body);
        let b_cov = (0.5 - ((d_body + 0.5).abs() - 0.5) / px_u).clamp(0., 1.) * fade;
        if b_cov > 0.0 {
            c = blend_over(c, border_rgb, b_cov * border_a);
        }
    }

    let d_write = sd_round_rect(u, v, 0.0, 0.0, COMP_W, WRITE_H, 16.0);
    let cov_write = (0.5 - d_write / px_u).clamp(0., 1.) * fade;
    if cov_write > 0.0 {
        c = blend_over(c, card_rgb, cov_write);
        let b_cov = (0.5 - ((d_write + 0.5).abs() - 0.5) / px_u).clamp(0., 1.) * fade;
        if b_cov > 0.0 {
            c = blend_over(c, border_rgb, b_cov * border_a);
        }

        // Circular send button background (`28x28` centered at `(296, 82)`)
        let d_send = (u - 296.0).hypot(v - 82.0) - 14.0;
        let cov_send = (0.5 - d_send / px_u).clamp(0., 1.) * fade;
        if cov_send > 0.0 {
            c = blend_over(c, border_rgb, cov_send * send_fill_a);
        }
    }
    c
}

fn rasterize_composer_plate(key: ComposerPlateKey) -> RenderImage {
    let tf = ComposerTilt::new(key.flat, key.mirror);
    let fx = key.fx_q as f32 / 10.0;
    let fy = key.fy_q as f32 / 10.0;
    let fw = (key.fw_q as f32 / 10.0).max(20.0);
    let fh = (key.fh_q as f32 / 10.0).max(16.0);
    let pad = (key.pad_q as f32 / 10.0).max(4.0);
    let gx = key.gx_q as f32 / 10.0;
    let gy0 = key.gy0_q as f32 / 10.0;
    let gy1 = key.gy1_q as f32 / 10.0;
    let rev = (f32::from(key.reveal_q) / 1000.0).clamp(0., 1.);

    let box_alpha = pro_ease(((rev - 0.10) / 0.45).clamp(0., 1.));
    let handle_alpha = pro_ease(((rev - 0.24) / 0.45).clamp(0., 1.));
    let guide_alpha = pro_ease(((rev - 0.42) / 0.48).clamp(0., 1.));

    let (tray_rgb, card_rgb, border_rgb, border_a, send_a, sh_a, blue_rgb, hatch_a, handle_fill) =
        if key.dark {
            (
                [20. / 255., 20. / 255., 22. / 255.],
                [28. / 255., 28. / 255., 31. / 255.],
                [1.0, 1.0, 1.0],
                0.095,
                0.085,
                0.42,
                [0x3b as f32 / 255., 0x82 as f32 / 255., 0xf6 as f32 / 255.],
                0.30,
                [25. / 255., 25. / 255., 27. / 255.],
            )
        } else {
            (
                [245. / 255., 245. / 255., 245. / 255.],
                [1.0, 1.0, 1.0],
                [0.0, 0.0, 0.0],
                0.07,
                0.06,
                0.10,
                [0x01 as f32 / 255., 0x90 as f32 / 255., 0xfe as f32 / 255.],
                0.26,
                [1.0, 1.0, 1.0],
            )
        };
    let red_rgb = [1.0, 59. / 255., 48. / 255.];

    const BAYER4: [u8; 16] = [0, 8, 2, 10, 12, 4, 14, 6, 3, 11, 1, 9, 15, 7, 13, 5];
    let px_u = 1.0 / RASTER_SCALE as f32;
    let mut raw = vec![0u8; TEX_W * TEX_H * 4];
    let above_u_off = if key.mirror { 16.0 } else { -16.0 };
    let above_v_off = -(BODY_H + ABOVE_GAP);

    for py in 0..TEX_H {
        let sy = (py as f32 + 0.5) * px_u;
        for px_i in 0..TEX_W {
            let sx = (px_i as f32 + 0.5) * px_u;
            let mask = stage_mask_alpha(sx, sy, key.mirror);
            if mask <= 0.002 {
                continue;
            }

            let (u, v) = tf.unproject(sx, sy);
            let mut c = [0.0f32; 4];

            // Optional faded predecessor composer above the foreground composer
            if key.has_above {
                let au = u - above_u_off;
                let av = v - above_v_off;
                let above_fade = smoothstep(8.0, BODY_H - 6.0, av);
                if above_fade > 0.005 {
                    c = paint_composer_shell(
                        c, au, av, px_u, above_fade, tray_rgb, card_rgb, border_rgb, border_a,
                        send_a,
                    );
                }
            }

            // Soft elevation shadow beneath the main composer
            let d_sh = sd_round_rect(u, v - 10.0, 4.0, 4.0, COMP_W - 4.0, BODY_H - 4.0, 16.0);
            if d_sh < 24.0 {
                let s_cov = (1. - d_sh.max(0.) / 24.0).powi(2) * sh_a;
                c = blend_over(c, [0.0, 0.0, 0.0], s_cov);
            }

            // Foreground composer tray + write surface + send pill
            c = paint_composer_shell(
                c, u, v, px_u, 1.0, tray_rgb, card_rgb, border_rgb, border_a, send_a,
            );

            // `#0190fe` selection box + anti-aliased diagonal padding hatch + inner dashed contour
            if box_alpha > 0.005 {
                let fr = fx + fw;
                let fb = fy + fh;
                if (fx - 3.0..=fr + 3.0).contains(&u) && (fy - 3.0..=fb + 3.0).contains(&v) {
                    let in_outer = (fx..=fr).contains(&u) && (fy..=fb).contains(&v);
                    let in_inner =
                        (fx + pad..=fr - pad).contains(&u) && (fy + pad..=fb - pad).contains(&v);

                    if in_outer && !in_inner {
                        let stripe = ((u + v) * 0.85).rem_euclid(3.8);
                        let stripe_d = (stripe - 0.45).abs();
                        let stripe_cov = (0.5 - (stripe_d - 0.40) / (0.85 * px_u)).clamp(0., 1.);
                        if stripe_cov > 0.0 {
                            c = blend_over(c, blue_rgb, stripe_cov * hatch_a * box_alpha);
                        }
                    }

                    let d_in = sd_round_rect(u, v, fx + pad, fy + pad, fr - pad, fb - pad, 3.0);
                    let in_line = (0.5 - (d_in.abs() - 0.42) / px_u).clamp(0., 1.);
                    if in_line > 0.0 {
                        let along = (u + v).rem_euclid(5.0);
                        let dash_cov = (0.5 - ((along - 1.4).abs() - 1.35) / px_u).clamp(0., 1.);
                        if dash_cov > 0.0 {
                            c = blend_over(c, blue_rgb, in_line * dash_cov * 0.86 * box_alpha);
                        }
                    }

                    let d_out = sd_round_rect(u, v, fx, fy, fr, fb, 0.5);
                    let out_line = (0.5 - (d_out.abs() - 0.55) / px_u).clamp(0., 1.);
                    if out_line > 0.0 {
                        c = blend_over(c, blue_rgb, out_line * box_alpha);
                    }
                }
            }

            // 4 corner resize handles
            if handle_alpha > 0.005 {
                let fr = fx + fw;
                let fb = fy + fh;
                for (hx, hy) in [(fx, fy), (fr, fy), (fr, fb), (fx, fb)] {
                    let cheb = (u - hx).abs().max((v - hy).abs());
                    let outer_cov = (0.5 - (cheb - 2.55) / px_u).clamp(0., 1.);
                    if outer_cov > 0.0 {
                        let inner_cov = (0.5 - (cheb - 1.55) / px_u).clamp(0., 1.);
                        c = blend_over(c, blue_rgb, outer_cov * handle_alpha);
                        if inner_cov > 0.0 {
                            c = blend_over(c, handle_fill, inner_cov * handle_alpha);
                        }
                    }
                }
            }

            // Red vertical spacing guide (`#ff3b30` dashed line + T-caps)
            if guide_alpha > 0.005 && gy1 > gy0 {
                let g_end = gy0 + (gy1 - gy0) * guide_alpha;
                if (v >= gy0 - 1.0 && v <= g_end + 1.0) && (u - gx).abs() <= 4.2 {
                    let stem_cov = (0.5 - ((u - gx).abs() - 0.55) / px_u).clamp(0., 1.);
                    if stem_cov > 0.0 && v >= gy0 && v <= g_end {
                        let dash_a = (v - gy0).rem_euclid(5.0);
                        let d_cov = ((1.5 - (dash_a - 1.5).abs()) / (0.45 * px_u)).clamp(0., 1.);
                        if d_cov > 0.0 {
                            c = blend_over(c, red_rgb, stem_cov * d_cov * guide_alpha);
                        }
                    }
                    let cap_x = (0.5 - ((u - gx).abs() - 3.5) / px_u).clamp(0., 1.);
                    if cap_x > 0.0 {
                        let cap_t = (0.5 - ((v - gy0).abs() - 0.55) / px_u).clamp(0., 1.);
                        let cap_b = (0.5 - ((v - g_end).abs() - 0.55) / px_u).clamp(0., 1.);
                        let cap_cov = cap_t.max(cap_b) * cap_x;
                        if cap_cov > 0.0 {
                            c = blend_over(c, red_rgb, cap_cov * guide_alpha);
                        }
                    }
                }
            }

            let out_a = (c[3] * mask).clamp(0., 1.);
            if out_a > 1e-4 {
                let dither = (f32::from(BAYER4[(py & 3) * 4 + (px_i & 3)]) / 16.0 - 0.5) / 255.0;
                let r = (c[0] / c[3].max(1e-4) + dither).clamp(0., 1.);
                let g = (c[1] / c[3].max(1e-4) + dither).clamp(0., 1.);
                let b = (c[2] / c[3].max(1e-4) + dither).clamp(0., 1.);
                let dst = (py * TEX_W + px_i) * 4;
                raw[dst] = (b * 255.).round() as u8;
                raw[dst + 1] = (g * 255.).round() as u8;
                raw[dst + 2] = (r * 255.).round() as u8;
                raw[dst + 3] = (out_a * 255.).round() as u8;
            }
        }
    }

    for y in 1..TEX_H - 1 {
        let row = y * TEX_W;
        for x in 1..TEX_W - 1 {
            let idx = (row + x) * 4;
            if raw[idx + 3] != 0 {
                continue;
            }
            let mut best_a = 0u8;
            let mut best_bgr = [0u8; 3];
            for (dx, dy) in [
                (-1isize, 0isize),
                (1, 0),
                (0, -1),
                (0, 1),
                (-1, -1),
                (1, -1),
                (-1, 1),
                (1, 1),
            ] {
                let n = (((y as isize + dy) as usize) * TEX_W + (x as isize + dx) as usize) * 4;
                let na = raw[n + 3];
                if na > best_a {
                    best_a = na;
                    best_bgr = [raw[n], raw[n + 1], raw[n + 2]];
                }
            }
            if best_a > 0 {
                raw[idx] = best_bgr[0];
                raw[idx + 1] = best_bgr[1];
                raw[idx + 2] = best_bgr[2];
            }
        }
    }

    let buf = RgbaImage::from_raw(TEX_W as u32, TEX_H as u32, raw)
        .expect("TEX_W * TEX_H * 4 matches raw buffer length");
    RenderImage::new(vec![Frame::new(buf)])
}

fn step_composer_timeline(
    playing: bool,
    hold_ms: u64,
    delay_ms: u64,
    elapsed_ms: u64,
) -> (InspectComposerRegion, f32) {
    if !playing || elapsed_ms < delay_ms {
        return (InspectComposerRegion::Message, 1.0);
    }
    let active_ms = elapsed_ms - delay_ms;
    let enter_ms = 520u64;
    let exit_ms = 180u64;
    let cycle_ms = enter_ms + hold_ms.max(400) + exit_ms;
    let reg_idx = ((active_ms / cycle_ms) as usize + 1) % 4;
    let within = active_ms % cycle_ms;
    let reveal = if within < enter_ms {
        within as f32 / enter_ms as f32
    } else if within < enter_ms + hold_ms.max(400) {
        1.0
    } else {
        1.0 - ((within - enter_ms - hold_ms.max(400)) as f32 / exit_ms as f32).clamp(0., 1.)
    };
    (InspectComposerRegion::from_index(reg_idx), reveal)
}

#[allow(clippy::too_many_arguments)]
fn render_composer_contents(
    message: &SharedString,
    mode: &SharedString,
    tf: ComposerTilt,
    u_off: f32,
    v_off: f32,
    dark: bool,
    mirror: bool,
    extra_fade: f32,
    cx: &App,
) -> Vec<AnyElement> {
    let mut out = Vec::with_capacity(3);
    let s_ui = tf.scale();
    let hint_c: Hsla = if dark {
        hsla(0., 0., 1., 0.38)
    } else {
        hsla(0., 0., 0., 0.36)
    };
    let icon_c: Hsla = if dark {
        hsla(0., 0., 1., 0.55)
    } else {
        hsla(0., 0., 0., 0.45)
    };
    let muted_c: Hsla = if dark {
        hsla(0., 0., 1., 0.50)
    } else {
        hsla(0., 0., 0., 0.45)
    };
    let faint_c: Hsla = if dark {
        hsla(0., 0., 1., 0.30)
    } else {
        hsla(0., 0., 0., 0.28)
    };
    let tile_bg: Hsla = if dark {
        rgb(0x27272b).into()
    } else {
        rgb(0xffffff).into()
    };
    let tile_bdr: Hsla = if dark {
        hsla(0., 0., 1., 0.09)
    } else {
        hsla(0., 0., 0., 0.07)
    };

    // Row A: Prompt message (`u = 14, v = 14`)
    let (mx, my) = tf.project(14.0 + u_off, 14.0 + v_off);
    let m_alpha = stage_mask_alpha(mx + 120. * s_ui, my + 12. * s_ui, mirror) * extra_fade;
    if m_alpha > 0.02 {
        out.push(
            div()
                .absolute()
                .left(px(mx))
                .top(px(my))
                .w(px(286. * s_ui))
                .h(px(40. * s_ui))
                .overflow_hidden()
                .text_size(px(13. * s_ui))
                .line_height(px(20. * s_ui))
                .text_color(hint_c)
                .opacity(m_alpha)
                .child(message.clone())
                .into_any_element(),
        );
    }

    // Row B: Toolbar (`u = 10..310, v = 68`)
    let (tx, ty) = tf.project(10.0 + u_off, 68.0 + v_off);
    let t_alpha = stage_mask_alpha(tx + 140. * s_ui, ty + 14. * s_ui, mirror) * extra_fade;
    if t_alpha > 0.02 {
        out.push(
            div()
                .absolute()
                .left(px(tx))
                .top(px(ty))
                .w(px(300. * s_ui))
                .h(px(28. * s_ui))
                .flex()
                .items_center()
                .justify_between()
                .opacity(t_alpha)
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(4. * s_ui))
                        .child(
                            div()
                                .size(px(28. * s_ui))
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(pro_svg("0 0 16 16", PLUS_THIN_SVG, 16. * s_ui, icon_c)),
                        )
                        .child(
                            div()
                                .h(px(28. * s_ui))
                                .px(px(8. * s_ui))
                                .flex()
                                .items_center()
                                .gap(px(6. * s_ui))
                                .child(pro_svg(
                                    "0 0 68.155 68.155",
                                    HAND_MODE_SVG,
                                    15. * s_ui,
                                    icon_c,
                                ))
                                .child(
                                    div()
                                        .text_size(px(13. * s_ui))
                                        .line_height(px(18. * s_ui))
                                        .text_color(muted_c)
                                        .child(mode.clone()),
                                )
                                .child(pro_icon(ProIcon::ChevronDown, 13. * s_ui, faint_c)),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(4. * s_ui))
                        .child(
                            div()
                                .size(px(28. * s_ui))
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(pro_icon(ProIcon::Mic, 15. * s_ui, icon_c)),
                        )
                        .child(
                            div()
                                .size(px(28. * s_ui))
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(pro_icon(ProIcon::ArrowUp, 15. * s_ui, faint_c)),
                        ),
                )
                .into_any_element(),
        );
    }

    // Row C: Tray footer (`Work in a folder` + `Connect apps` with Gmail, Slack, ClickUp)
    let (fx_p, fy_p) = tf.project(12.0 + u_off, 111.0 + v_off);
    let f_alpha = stage_mask_alpha(fx_p + 150. * s_ui, fy_p + 12. * s_ui, mirror) * extra_fade;
    if f_alpha > 0.02 {
        let app_sz = (12.5 * s_ui).round();
        let chip_sz = 18.5 * s_ui;
        let wrap_tile = |el: AnyElement| {
            div()
                .size(px(chip_sz))
                .rounded(px(4.5 * s_ui))
                .bg(tile_bg)
                .border_1()
                .border_color(tile_bdr)
                .flex()
                .items_center()
                .justify_center()
                .child(el)
        };
        let gmail = pro_app_artwork("gmail", app_sz, cx)
            .unwrap_or_else(|| pro_icon(ProIcon::Mail, app_sz, icon_c));
        let slack = pro_app_artwork("slack", app_sz, cx)
            .unwrap_or_else(|| pro_icon(ProIcon::Sparkles, app_sz, icon_c));
        out.push(
            div()
                .absolute()
                .left(px(fx_p))
                .top(px(fy_p))
                .w(px(296. * s_ui))
                .h(px(24. * s_ui))
                .flex()
                .items_center()
                .justify_between()
                .opacity(f_alpha)
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6. * s_ui))
                        .child(pro_raw_svg(FOLDER_MINI_SVG, 15. * s_ui, 15. * s_ui))
                        .child(
                            div()
                                .text_size(px(12. * s_ui))
                                .line_height(px(16. * s_ui))
                                .text_color(muted_c)
                                .child(tr(cx, "Work in a folder", "在文件夹中协作")),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(7. * s_ui))
                        .child(
                            div()
                                .text_size(px(12. * s_ui))
                                .line_height(px(16. * s_ui))
                                .text_color(muted_c)
                                .child(tr(cx, "Connect apps", "连接应用")),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(3.5 * s_ui))
                                .child(wrap_tile(gmail))
                                .child(wrap_tile(slack))
                                .child(wrap_tile(pro_raw_svg(CLICKUP_MINI_SVG, app_sz, app_sz))),
                        ),
                )
                .into_any_element(),
        );
    }

    out
}

/// Design-tool pixel inspection scene of an AI chat composer with `#0190fe` Figma chrome.
///
/// # Examples
///
/// ```ignore
/// use kui::components::inspect_composer::InspectComposer;
///
/// let composer = cx.new(|cx| InspectComposer::new("hero-composer-inspect", cx));
/// ```
pub struct InspectComposer {
    id: ElementId,
    focus: FocusHandle,
    message: Option<SharedString>,
    above: Option<SharedString>,
    has_custom_above: bool,
    mode: Option<SharedString>,
    playing: bool,
    hold_ms: u64,
    delay_ms: u64,
    mirror: bool,
    flat: bool,
    manual_target: Option<usize>,
    pinned_target: Option<usize>,
    pinned_region: Option<InspectComposerRegion>,
    fx_slide: Slide,
    fy_slide: Slide,
    fw_slide: Slide,
    fh_slide: Slide,
    pad_slide: Slide,
    gx_slide: Slide,
    gy0_slide: Slide,
    gy1_slide: Slide,
    press_slide: Slide,
    loading: bool,
    empty: bool,
    clock: Instant,
    stage_bounds: Rc<Cell<Bounds<Pixels>>>,
    plate_cache: Rc<RefCell<ComposerPlateCache>>,
}

impl InspectComposer {
    /// Construct an [`InspectComposer`] tilted toward the left (`mirror = true`) with default prompts.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<UiLanguage>(|_, cx| cx.notify())
            .detach();
        let r0 = region_box(InspectComposerRegion::Message, true);
        Self {
            id: id.into(),
            focus: cx.focus_handle().tab_stop(true),
            message: None,
            above: None,
            has_custom_above: false,
            mode: None,
            playing: true,
            hold_ms: 1600,
            delay_ms: 600,
            mirror: true,
            flat: false,
            manual_target: None,
            pinned_target: None,
            pinned_region: None,
            fx_slide: Slide::new(r0.fx),
            fy_slide: Slide::new(r0.fy),
            fw_slide: Slide::new(r0.fw),
            fh_slide: Slide::new(r0.fh),
            pad_slide: Slide::new(r0.pad),
            gx_slide: Slide::new(r0.guide_x),
            gy0_slide: Slide::new(r0.guide_y0),
            gy1_slide: Slide::new(r0.guide_y1),
            press_slide: Slide::snappy(0.),
            loading: false,
            empty: false,
            clock: cx.background_executor().now(),
            stage_bounds: Rc::new(Cell::new(Bounds::default())),
            plate_cache: Rc::new(RefCell::new(ComposerPlateCache::new())),
        }
    }

    /// Override the message shown inside the foreground composer.
    pub fn with_message(mut self, message: impl Into<SharedString>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Override or clear (`None`) the message inside the faded composer above.
    pub fn with_above(mut self, above: Option<SharedString>) -> Self {
        self.above = above;
        self.has_custom_above = true;
        self
    }

    /// Override the label on the approval mode selector (`"Auto"` by default).
    pub fn with_mode(mut self, mode: impl Into<SharedString>) -> Self {
        self.mode = Some(mode.into());
        self
    }

    /// Control whether the selection steps through regions automatically (`true` by default).
    pub fn with_playing(mut self, playing: bool) -> Self {
        self.playing = playing;
        self
    }

    /// Set how long the selection stays on each region in milliseconds (`1600` by default).
    pub fn with_hold(mut self, hold_ms: u64) -> Self {
        self.hold_ms = hold_ms.max(400);
        self
    }

    /// Set the initial pause on `Message` before stepping onward (`600` by default).
    pub fn with_delay(mut self, delay_ms: u64) -> Self {
        self.delay_ms = delay_ms;
        self
    }

    /// Configure mirrored left-fading perspective (`true` by default; `false` flips to right-fading).
    pub fn with_mirror(mut self, mirror: bool) -> Self {
        self.mirror = mirror;
        self
    }

    /// Render the composer flat with zero 3D perspective tilt (`false` by default).
    pub fn with_flat(mut self, flat: bool) -> Self {
        self.flat = flat;
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

    /// Return whether region-stepping playback is active.
    pub fn is_playing(&self) -> bool {
        self.playing
    }

    /// Toggle region-stepping playback at runtime.
    pub fn set_playing(&mut self, playing: bool, cx: &mut Context<Self>) {
        if self.playing != playing {
            self.playing = playing;
            self.clock = cx.background_executor().now();
            cx.emit(InspectComposerEvent::PlayingChanged(playing));
            cx.notify();
        }
    }

    /// Return the manually pinned region, if any.
    pub fn pinned_region(&self) -> Option<InspectComposerRegion> {
        self.pinned_region
    }

    /// Pin or unpin inspection on a specific composer region at runtime.
    pub fn set_pinned_region(
        &mut self,
        region: Option<InspectComposerRegion>,
        cx: &mut Context<Self>,
    ) {
        if self.pinned_region != region
            || self.pinned_target != region.map(InspectComposerRegion::index)
        {
            self.pinned_region = region;
            self.pinned_target = region.map(InspectComposerRegion::index);
            cx.emit(InspectComposerEvent::PinRegion(region));
            cx.notify();
        }
    }

    /// Return whether mirrored perspective is active.
    pub fn is_mirror(&self) -> bool {
        self.mirror
    }

    /// Return whether flat mode is active.
    pub fn is_flat(&self) -> bool {
        self.flat
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

    fn hit_target_uv(u: f32, v: f32) -> Option<usize> {
        if !(0.0..=COMP_W).contains(&u) || !(0.0..=BODY_H).contains(&v) {
            return None;
        }
        if v < 60.0 {
            Some(0)
        } else if v < 104.0 {
            if (4.0..=39.0).contains(&u) {
                Some(4)
            } else if (40.0..=142.0).contains(&u) {
                Some(2)
            } else if (242.0..=277.0).contains(&u) {
                Some(5)
            } else if (278.0..=316.0).contains(&u) {
                Some(6)
            } else {
                Some(1)
            }
        } else if (6.0..=142.0).contains(&u) {
            Some(7)
        } else if (152.0..=314.0).contains(&u) {
            Some(8)
        } else {
            Some(3)
        }
    }

    fn hit_target(&self, pos: Point<Pixels>) -> Option<usize> {
        let b = self.stage_bounds.get();
        if b.size.width <= px(2.) {
            return None;
        }
        let sx = f32::from(pos.x - b.origin.x);
        let sy = f32::from(pos.y - b.origin.y);
        let tf = ComposerTilt::new(self.flat, self.mirror);
        let (u, v) = tf.unproject(sx, sy);
        Self::hit_target_uv(u, v)
    }
}

impl Render for InspectComposer {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let dark = Theme::global(cx).mode == ThemeMode::Dark;
        let reduced = disabled_motion(cx);
        let now = cx.background_executor().now();
        let elapsed = now.saturating_duration_since(self.clock);

        if self.loading {
            let phase = if reduced {
                0.0
            } else {
                w.request_animation_frame();
                (elapsed.as_secs_f32() / 1.4).fract()
            };
            return div()
                .id(pro_eid(&self.id, "loading"))
                .w_full()
                .h(px(STAGE_H as f32))
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .w(px(300.))
                        .p(px(14.))
                        .rounded(px(16.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .flex()
                        .flex_col()
                        .gap(px(14.))
                        .child(pro_skeleton_bar(228., 28., 6., phase, cx))
                        .child(pro_skeleton_bar(272., 24., 8., phase, cx))
                        .child(pro_skeleton_bar(260., 18., 6., phase, cx)),
                )
                .into_any_element();
        }

        if self.empty {
            return div()
                .id(pro_eid(&self.id, "empty"))
                .w_full()
                .child(pro_empty_state::<Self>(
                    ProIcon::NewChat,
                    tr(cx, "No composer to inspect", "暂无可检视的输入框"),
                    tr(
                        cx,
                        "Provide a prompt message to render the composer inspection scene.",
                        "提供提示词文案后即可展示输入框像素检视场景。",
                    ),
                    None,
                    cx,
                ))
                .into_any_element();
        }

        let elapsed_ms = elapsed.as_millis() as u64;
        let interactive_override = self
            .manual_target
            .or(self.pinned_target)
            .or_else(|| self.pinned_region.map(InspectComposerRegion::index));
        let auto_animate = !reduced && self.playing && interactive_override.is_none();

        let (active_target_idx, reveal) = if let Some(target_idx) = interactive_override {
            (target_idx, 1.0)
        } else if reduced || !self.playing {
            (0, 1.0)
        } else {
            let (reg, rev) =
                step_composer_timeline(self.playing, self.hold_ms, self.delay_ms, elapsed_ms);
            (reg.index(), rev)
        };

        let tb = composer_target_box(active_target_idx, self.mirror);
        let rb = tb.rb;
        self.fx_slide.duration(rb.fx, 0.24, cx);
        self.fy_slide.duration(rb.fy, 0.24, cx);
        self.fw_slide.duration(rb.fw, 0.24, cx);
        self.fh_slide.duration(rb.fh, 0.24, cx);
        self.pad_slide.duration(rb.pad, 0.22, cx);
        self.gx_slide.duration(rb.guide_x, 0.24, cx);
        self.gy0_slide.duration(rb.guide_y0, 0.24, cx);
        self.gy1_slide.duration(rb.guide_y1, 0.24, cx);

        let (cur_fx, cur_fy, cur_fw, cur_fh, cur_pad, cur_gx, cur_gy0, cur_gy1, press_t) =
            if reduced {
                (
                    rb.fx,
                    rb.fy,
                    rb.fw,
                    rb.fh,
                    rb.pad,
                    rb.guide_x,
                    rb.guide_y0,
                    rb.guide_y1,
                    0.0,
                )
            } else {
                (
                    self.fx_slide.get(w, cx),
                    self.fy_slide.get(w, cx),
                    self.fw_slide.get(w, cx),
                    self.fh_slide.get(w, cx),
                    self.pad_slide.get(w, cx),
                    self.gx_slide.get(w, cx),
                    self.gy0_slide.get(w, cx),
                    self.gy1_slide.get(w, cx),
                    self.press_slide.get(w, cx),
                )
            };

        let main_msg: SharedString = self.message.clone().unwrap_or_else(|| {
            tr(
                cx,
                "Ask the agents to ship the new pricing page",
                "让智能体团队构建并上线全新定价页面",
            )
            .into()
        });
        let above_msg: Option<SharedString> = if self.has_custom_above {
            self.above.clone()
        } else {
            Some(
                tr(
                    cx,
                    "Summarize the pricing research for the team",
                    "为团队整理五家竞品的定价调研结论",
                )
                .into(),
            )
        };
        let mode_str: SharedString = self
            .mode
            .clone()
            .unwrap_or_else(|| tr(cx, "Auto", "自动").into());

        let tf = ComposerTilt::new(self.flat, self.mirror);
        let plate_key = ComposerPlateKey {
            dark,
            has_above: above_msg.is_some(),
            mirror: self.mirror,
            flat: self.flat,
            fx_q: (cur_fx * 10.).round() as i32,
            fy_q: (cur_fy * 10.).round() as i32,
            fw_q: (cur_fw * 10.).round() as i32,
            fh_q: (cur_fh * 10.).round() as i32,
            pad_q: (cur_pad * 10.).round() as i32,
            gx_q: (cur_gx * 10.).round() as i32,
            gy0_q: (cur_gy0 * 10.).round() as i32,
            gy1_q: (cur_gy1 * 10.).round() as i32,
            reveal_q: (reveal * 1000.).round() as u16,
        };

        let bounds_cell = self.stage_bounds.clone();
        let plate_cache = self.plate_cache.clone();

        let mut scene = div()
            .id(pro_eid(&self.id, "scene"))
            .relative()
            .w(px(STAGE_W as f32))
            .h(px(STAGE_H as f32))
            .flex_none()
            .overflow_hidden()
            .cursor_pointer()
            .on_mouse_move(cx.listener(|this, e: &MouseMoveEvent, _, cx| {
                let hit = this.hit_target(e.position);
                if this.manual_target != hit {
                    let was_some = this.manual_target.is_some();
                    this.manual_target = hit;
                    if hit.is_none()
                        && was_some
                        && this.pinned_target.is_none()
                        && this.pinned_region.is_none()
                    {
                        this.clock = cx.background_executor().now();
                    }
                    cx.notify();
                }
            }))
            .on_hover(cx.listener(|this, over: &bool, _, cx| {
                if !*over && this.manual_target.take().is_some() {
                    this.press_slide.duration(0., 0.12, cx);
                    if this.pinned_target.is_none() && this.pinned_region.is_none() {
                        this.clock = cx.background_executor().now();
                    }
                    cx.notify();
                }
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, e: &MouseDownEvent, _, cx| {
                    this.press_slide.duration(1., 0.08, cx);
                    let hit = this.hit_target(e.position);
                    let next_target = if this.pinned_target == hit { None } else { hit };
                    let next_region =
                        next_target.map(|idx| composer_target_box(idx, this.mirror).region);
                    this.pinned_target = next_target;
                    this.pinned_region = next_region;
                    if next_target.is_none() && this.manual_target.is_none() {
                        this.clock = cx.background_executor().now();
                    }
                    cx.emit(InspectComposerEvent::PinRegion(next_region));
                    cx.notify();
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _: &MouseUpEvent, _, cx| {
                    this.press_slide.duration(0., 0.14, cx);
                    cx.notify();
                }),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, _: &MouseUpEvent, _, cx| {
                    this.press_slide.duration(0., 0.14, cx);
                    cx.notify();
                }),
            )
            .child(
                canvas(
                    move |b, _, _| bounds_cell.set(b),
                    move |bounds, _, window, _| {
                        let image = plate_cache.borrow_mut().render(plate_key, window);
                        let _ =
                            window.paint_image(bounds, bounds, Corners::default(), image, 0, false);
                        if auto_animate {
                            window.request_animation_frame();
                        }
                    },
                )
                .absolute()
                .inset_0(),
            );

        if let Some(above_text) = &above_msg {
            let u_off = if self.mirror { 16.0 } else { -16.0 };
            for el in render_composer_contents(
                above_text,
                &mode_str,
                tf,
                u_off,
                -(BODY_H + ABOVE_GAP),
                dark,
                self.mirror,
                0.55,
                cx,
            ) {
                scene = scene.child(el);
            }
        }

        for el in render_composer_contents(
            &main_msg,
            &mode_str,
            tf,
            0.0,
            0.0,
            dark,
            self.mirror,
            1.0,
            cx,
        ) {
            scene = scene.child(el);
        }

        let blue_pill: Hsla = if dark {
            rgb(0x3b82f6).into()
        } else {
            rgb(0x0190fe).into()
        };
        let red_pill: Hsla = rgb(0xff3b30).into();
        let white_ink: Hsla = rgb(0xffffff).into();

        let tag_a = pro_ease(((reveal - 0.30) / 0.45).clamp(0., 1.));
        let pad_a = pro_ease(((reveal - 0.38) / 0.45).clamp(0., 1.));
        let dim_a = pro_ease(((reveal - 0.48) / 0.45).clamp(0., 1.));
        let cur_a = pro_ease((reveal / 0.38).clamp(0., 1.));

        // Layer tag pill cleanly ABOVE-LEFT of the selection frame (`cur_fy - 18.0`)
        let tag_u = if self.mirror {
            (cur_fx + 44.0).min(cur_fx + cur_fw - 68.0).max(cur_fx)
        } else if cur_fy > 12.0 {
            cur_fx - 14.0
        } else {
            cur_fx
        };
        let (_, tag_top_y) = tf.project(tag_u, cur_fy);
        let (tag_x, _) = tf.project(tag_u, cur_fy - 18.0);
        let tag_y = tag_top_y - 18.0 + press_t * 1.0;
        let is_pinned = self.pinned_target.is_some() || self.pinned_region.is_some();
        let tag_text = if is_pinned {
            format!("{} •", tb.tag_label(cx))
        } else {
            tb.tag_label(cx).to_string()
        };
        scene = scene.child(
            div()
                .absolute()
                .left(px(tag_x))
                .top(px(tag_y))
                .h(px(15.))
                .px(px(5.))
                .rounded(px(3.))
                .bg(blue_pill)
                .flex()
                .items_center()
                .gap(px(3.))
                .opacity(tag_a * stage_mask_alpha(tag_x + 24., tag_y + 7., self.mirror))
                .child(pro_svg("0 0 12 12", LAYER_TAG_SVG, 9., white_ink))
                .child(pro_swap_label(
                    pro_eid(&self.id, "layer-tag"),
                    tag_text,
                    9.,
                    FontWeight::MEDIUM,
                    white_ink,
                )),
        );

        let pad_num = format!("{:.0}", cur_pad.round());
        // Top blue padding badge centered cleanly along the top edge when wide enough
        if cur_fw >= 88.0 {
            let pad_top_u = cur_fx + cur_fw * 0.48;
            let (pt_x, pt_edge_y) = tf.project(pad_top_u, cur_fy);
            let pt_y = pt_edge_y - 6.5;
            scene = scene.child(
                div()
                    .absolute()
                    .left(px(pt_x))
                    .top(px(pt_y))
                    .h(px(13.))
                    .px(px(3.5))
                    .rounded(px(3.))
                    .bg(blue_pill)
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(8.5))
                    .line_height(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(white_ink)
                    .opacity(pad_a * stage_mask_alpha(pt_x, pt_y + 6., self.mirror))
                    .child(pad_num.clone()),
            );
        }

        // Side blue padding badge in the outer margin (never covering Send button or `+`)
        let pad_side_u = if self.mirror {
            cur_fx + cur_fw + 5.0
        } else {
            cur_fx - 20.0
        };
        let (ps_x, ps_mid_y) = tf.project(pad_side_u, cur_fy + cur_fh * 0.5);
        let ps_y = ps_mid_y - 6.5;
        scene = scene.child(
            div()
                .absolute()
                .left(px(ps_x))
                .top(px(ps_y))
                .h(px(13.))
                .px(px(3.5))
                .rounded(px(3.))
                .bg(blue_pill)
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(8.5))
                .line_height(px(13.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(white_ink)
                .opacity(pad_a * stage_mask_alpha(ps_x, ps_y + 6., self.mirror))
                .child(pad_num),
        );

        // Red spacing guide badge (`14px` / `8px` / `10px` / `9px`) cleanly in open space
        let red_u = if !self.mirror && cur_gx < 30.0 {
            cur_gx - 42.0
        } else {
            cur_gx + 6.0
        };
        let (rx, r_mid_y) = tf.project(red_u, (cur_gy0 + cur_gy1) * 0.5);
        let ry = r_mid_y - 6.5;
        scene = scene.child(
            div()
                .absolute()
                .left(px(rx))
                .top(px(ry))
                .h(px(13.))
                .px(px(4.))
                .rounded(px(3.))
                .bg(red_pill)
                .flex()
                .items_center()
                .justify_center()
                .opacity(dim_a * stage_mask_alpha(rx + 14., ry + 6., self.mirror))
                .child(pro_swap_label(
                    pro_eid(&self.id, "gap-tag"),
                    rb.gap_label,
                    8.5,
                    FontWeight::MEDIUM,
                    white_ink,
                )),
        );

        // Dimension pill (`W Fill 300 px` / `W Hug 84 px` / `W Fixed 28 × 28 px`) never covering footer apps
        let size_u = if cur_fy < 10.0 && cur_fw > 200.0 {
            148.0
        } else if self.mirror {
            (cur_fx + cur_fw - 96.0).max(cur_fx)
        } else {
            cur_fx + (cur_fw - 96.0).max(0.0)
        };
        let size_base_v = if (96.0..136.0).contains(&(cur_fy + cur_fh)) {
            BODY_H
        } else {
            cur_fy + cur_fh
        };
        let (sz_x, sz_bot_y) = tf.project(size_u, size_base_v);
        let sz_y = sz_bot_y + 6.0;
        scene = scene.child(
            div()
                .absolute()
                .left(px(sz_x))
                .top(px(sz_y))
                .h(px(15.))
                .px(px(5.))
                .rounded(px(3.))
                .bg(blue_pill)
                .flex()
                .items_center()
                .gap(px(4.))
                .opacity(dim_a * stage_mask_alpha(sz_x + 25., sz_y, self.mirror))
                .child(
                    div()
                        .text_size(px(9.))
                        .line_height(px(14.))
                        .text_color(white_ink.opacity(0.76))
                        .child(rb.dim_prefix),
                )
                .child(pro_swap_label(
                    pro_eid(&self.id, "dim-px"),
                    rb.dim_px,
                    9.,
                    FontWeight::MEDIUM,
                    white_ink,
                )),
        );

        // Hand cursor positioned in open toolbar/frame space
        let cur_u = if cur_fw > 200.0 {
            196.0
        } else {
            cur_fx + cur_fw * 0.68
        };
        let (cx_p, cy_p) = tf.project(cur_u, cur_fy + cur_fh - 16.0 + press_t * 1.5);
        let cur_slide = (1. - cur_a) * 6.0;
        scene = scene.child(
            div()
                .absolute()
                .left(px(cx_p + cur_slide))
                .top(px(cy_p + cur_slide))
                .opacity(cur_a * stage_mask_alpha(cx_p, cy_p, self.mirror))
                .child(pro_raw_svg(HAND_CURSOR_SVG, 19., 19.)),
        );

        let focus_ring = p.focus;
        let a11y = tr(
            cx,
            "Design-tool pixel inspection of a tilted AI chat composer",
            "三维透视倾斜 AI 对话输入框的像素间距检视场景",
        );

        div()
            .id(self.id.clone())
            .role(Role::Image)
            .aria_label(a11y)
            .track_focus(&self.focus)
            .tab_index(0)
            .w_full()
            .min_h(px(STAGE_H as f32))
            .flex()
            .items_center()
            .justify_center()
            .border_1()
            .border_color(transparent_black())
            .focus_visible(move |s| s.rounded(px(14.)).border_color(focus_ring))
            .on_key_down(cx.listener(
                |this, e: &KeyDownEvent, _, cx| match e.keystroke.key.as_str() {
                    "down" | "right" | "j" => {
                        let cur = this
                            .pinned_target
                            .or_else(|| this.pinned_region.map(InspectComposerRegion::index))
                            .unwrap_or(0);
                        let next_idx = (cur + 1) % 9;
                        let next_reg = composer_target_box(next_idx, this.mirror).region;
                        this.pinned_target = Some(next_idx);
                        this.pinned_region = Some(next_reg);
                        cx.emit(InspectComposerEvent::PinRegion(Some(next_reg)));
                        cx.stop_propagation();
                        cx.notify();
                    }
                    "up" | "left" | "k" => {
                        let cur = this
                            .pinned_target
                            .or_else(|| this.pinned_region.map(InspectComposerRegion::index))
                            .unwrap_or(0);
                        let next_idx = (cur + 8) % 9;
                        let next_reg = composer_target_box(next_idx, this.mirror).region;
                        this.pinned_target = Some(next_idx);
                        this.pinned_region = Some(next_reg);
                        cx.emit(InspectComposerEvent::PinRegion(Some(next_reg)));
                        cx.stop_propagation();
                        cx.notify();
                    }
                    "escape" => {
                        this.pinned_target = None;
                        this.pinned_region = None;
                        this.manual_target = None;
                        this.clock = cx.background_executor().now();
                        cx.emit(InspectComposerEvent::PinRegion(None));
                        cx.stop_propagation();
                        cx.notify();
                    }
                    "enter" | "space" | " " => {
                        this.playing = !this.playing;
                        this.clock = cx.background_executor().now();
                        cx.emit(InspectComposerEvent::PlayingChanged(this.playing));
                        cx.stop_propagation();
                        cx.notify();
                    }
                    _ => {}
                },
            ))
            .child(scene)
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        COMP_W, ComposerPlateKey, ComposerTilt, InspectComposer, InspectComposerRegion,
        RASTER_SCALE, TEX_W, rasterize_composer_plate, region_box, step_composer_timeline,
    };
    use gpui::{MouseButton, MouseDownEvent, MouseMoveEvent, TestAppContext, point, px, size};
    use std::time::Duration;

    #[test]
    fn mirrored_composer_homography_roundtrips_and_cycles_all_four_regions() {
        let tf = ComposerTilt::new(false, true);
        let (sx, sy) = tf.project(210.0, 82.0);
        let (u, v) = tf.unproject(sx, sy);
        assert!((u - 210.0).abs() < 0.05 && (v - 82.0).abs() < 0.05);

        assert_eq!(
            step_composer_timeline(true, 1600, 600, 200).0,
            InspectComposerRegion::Message
        );
        assert_eq!(
            step_composer_timeline(true, 1600, 600, 600 + 800).0,
            InspectComposerRegion::Toolbar
        );
        assert_eq!(
            step_composer_timeline(true, 1600, 600, 600 + 2300 + 800).0,
            InspectComposerRegion::Selector
        );
        assert_eq!(
            step_composer_timeline(true, 1600, 600, 600 + 4600 + 800).0,
            InspectComposerRegion::SendFooter
        );
    }

    #[test]
    fn composer_plate_paints_selection_frame_and_red_spacing_guide() {
        let rb = region_box(InspectComposerRegion::Toolbar, false);
        let img = rasterize_composer_plate(ComposerPlateKey {
            dark: true,
            has_above: true,
            mirror: false,
            flat: true,
            fx_q: (rb.fx * 10.) as i32,
            fy_q: (rb.fy * 10.) as i32,
            fw_q: (rb.fw * 10.) as i32,
            fh_q: (rb.fh * 10.) as i32,
            pad_q: (rb.pad * 10.) as i32,
            gx_q: (rb.guide_x * 10.) as i32,
            gy0_q: (rb.guide_y0 * 10.) as i32,
            gy1_q: (rb.guide_y1 * 10.) as i32,
            reveal_q: 1000,
        });
        let bytes = img.as_bytes(0).expect("BGRA plate");
        // Sample across the 1px subpixel-AA top selection frame (`x = 58 + 180, y = 207..=209`)
        let max_blue = (207 * RASTER_SCALE..=209 * RASTER_SCALE + 1)
            .map(|y| {
                let p = (y * TEX_W + (58 + 180) * RASTER_SCALE) * 4;
                bytes[p].saturating_sub(bytes[p + 2])
            })
            .max()
            .unwrap_or(0);
        assert!(
            max_blue > 70,
            "expected blue selection line, max B-R={max_blue}"
        );

        // Sample across the top T-cap of `#ff3b30` spacing guide (`x = 58 + 14, y = 203..=205`)
        let max_red = (203 * RASTER_SCALE..=205 * RASTER_SCALE + 1)
            .map(|y| {
                let p = (y * TEX_W + (58 + 14) * RASTER_SCALE) * 4;
                bytes[p + 2].saturating_sub(bytes[p])
            })
            .max()
            .unwrap_or(0);
        assert!(
            max_red > 70,
            "expected red spacing guide, max R-B={max_red}"
        );
    }

    #[gpui::test]
    fn hovering_toolbar_and_footer_subregions_glides_inspector_and_resumes_tour(
        cx: &mut TestAppContext,
    ) {
        cx.update(crate::init);
        let exec = cx.executor();
        let (view, vcx) = cx.add_window_view(|_, cx| {
            InspectComposer::new("test-inspect-composer", cx)
                .with_flat(true)
                .with_mirror(false)
                .with_delay(300)
                .with_hold(500)
        });
        vcx.simulate_resize(size(px(600.), px(480.)));
        vcx.run_until_parked();

        let (send_pt, apps_pt, out_pt) = vcx.update(|_, app| {
            let b = view.read(app).stage_bounds.get();
            let tf = ComposerTilt::new(true, false);
            let (sx, sy) = tf.project(295.0, 80.0);
            let (ax, ay) = tf.project(220.0, 120.0);
            (
                point(b.origin.x + px(sx), b.origin.y + px(sy)),
                point(b.origin.x + px(ax), b.origin.y + px(ay)),
                point(b.origin.x + px(4.0), b.origin.y + px(4.0)),
            )
        });

        // Hover over circular Send button (`u = 295, v = 80` -> target 6)
        vcx.simulate_event(MouseMoveEvent {
            position: send_pt,
            pressed_button: None,
            modifiers: Default::default(),
        });
        vcx.run_until_parked();
        exec.advance_clock(Duration::from_millis(320));
        vcx.update(|w, app| w.simulate_next_frame(app));
        vcx.run_until_parked();

        vcx.update(|_, app| {
            let this = view.read(app);
            assert_eq!(this.manual_target, Some(6));
            assert!((this.fx_slide.value(app) - 278.0).abs() < 4.0);
            assert!((this.fw_slide.value(app) - 34.0).abs() < 4.0);
        });

        // Click on Connect Apps chip (`u = 220, v = 120` -> target 8) to pin
        vcx.simulate_event(MouseMoveEvent {
            position: apps_pt,
            pressed_button: None,
            modifiers: Default::default(),
        });
        vcx.simulate_event(MouseDownEvent {
            position: apps_pt,
            button: MouseButton::Left,
            modifiers: Default::default(),
            click_count: 1,
            first_mouse: false,
        });
        vcx.run_until_parked();
        vcx.update(|_, app| {
            let this = view.read(app);
            assert_eq!(this.pinned_target, Some(8));
            assert_eq!(
                this.pinned_region(),
                Some(InspectComposerRegion::SendFooter)
            );
        });

        // Click again to unpin and leave stage -> resumes automatic tour
        vcx.simulate_event(MouseDownEvent {
            position: apps_pt,
            button: MouseButton::Left,
            modifiers: Default::default(),
            click_count: 1,
            first_mouse: false,
        });
        vcx.simulate_event(MouseMoveEvent {
            position: out_pt,
            pressed_button: None,
            modifiers: Default::default(),
        });
        vcx.run_until_parked();
        exec.advance_clock(Duration::from_millis(600));
        vcx.update(|w, app| w.simulate_next_frame(app));
        vcx.run_until_parked();
        vcx.update(|_, app| {
            let this = view.read(app);
            assert_eq!(this.manual_target, None);
            assert_eq!(this.pinned_target, None);
            assert!((this.fw_slide.value(app) - COMP_W).abs() < 4.0);
        });
    }
}
