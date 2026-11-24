//! Glassy translucent folder illustration whose frosted pocket tips open on hover.
//!
//! Renders a rounded folder silhouette with soft ambient shadow, layered paper
//! sheets with rounded text lines, and a blurred glass front pocket that tips
//! forward smoothly when hovered, focused, or toggled open.

use crate::{
    components::pro_chrome::{ProPalette, pro_eid, pro_empty_state, pro_skeleton_bar, tr},
    foundation::{Theme, ThemeMode},
    primitives::{
        control::UiLanguage,
        family::{Slide, disabled_motion},
        pro_icons::ProIcon,
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

const TEX_W: usize = 720;
const TEX_H: usize = 576;
const DOC_LINES: [(f32, f32); 7] = [
    (87., 78.),
    (115., 180.),
    (143., 140.),
    (171., 175.),
    (199., 126.),
    (227., 168.),
    (255., 110.),
];

/// Events emitted by [`GlassFolder`] when toggled interactively.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GlassFolderEvent {
    /// Emitted when clicking or pressing Enter/Space toggles the pinned open state.
    OpenChanged(bool),
}

impl EventEmitter<GlassFolderEvent> for GlassFolder {}

#[derive(Clone, Copy, PartialEq, Eq)]
struct FolderCacheKey {
    dark: bool,
    accent_rgb: [u8; 3],
    custom_accent: bool,
    doc_count: u8,
    open_q: u16,
    tilt_x_q: i8,
    tilt_y_q: i8,
}

struct FolderSurfaceCache {
    key: Option<FolderCacheKey>,
    image: Option<Arc<RenderImage>>,
    alpha_buf: Vec<[f32; 1]>,
    alpha_tmp: Vec<[f32; 1]>,
    shadow_a: Vec<[f32; 1]>,
    shadow_b: Vec<[f32; 1]>,
    scene_buf: Vec<[f32; 4]>,
    blur_buf: Vec<[f32; 4]>,
    blur_tmp: Vec<[f32; 4]>,
}

impl FolderSurfaceCache {
    fn new() -> Self {
        let len = TEX_W * TEX_H;
        Self {
            key: None,
            image: None,
            alpha_buf: vec![[0.0; 1]; len],
            alpha_tmp: vec![[0.0; 1]; len],
            shadow_a: vec![[0.0; 1]; len],
            shadow_b: vec![[0.0; 1]; len],
            scene_buf: vec![[0.0; 4]; len],
            blur_buf: vec![[0.0; 4]; len],
            blur_tmp: vec![[0.0; 4]; len],
        }
    }

    fn render_frame(&mut self, key: FolderCacheKey, window: &mut Window) -> Arc<RenderImage> {
        if self.key == Some(key)
            && let Some(existing) = &self.image
        {
            return existing.clone();
        }
        let rendered = Arc::new(rasterize_glass_folder(
            key,
            &mut self.alpha_buf,
            &mut self.alpha_tmp,
            &mut self.shadow_a,
            &mut self.shadow_b,
            &mut self.scene_buf,
            &mut self.blur_buf,
            &mut self.blur_tmp,
        ));
        if let Some(prev) = self.image.replace(rendered.clone()) {
            let _ = window.drop_image(prev);
        }
        self.key = Some(key);
        rendered
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

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Premultiplied Porter-Duff `src over dst`.
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

fn blur_planar<const CH: usize>(
    buf: &mut [[f32; CH]],
    scratch: &mut [[f32; CH]],
    w: usize,
    h: usize,
    sigma: f32,
) {
    let r = ((sigma * 0.90).round() as usize).clamp(1, w.min(h) / 3);
    for _ in 0..3 {
        let inv_x = 1. / (2 * r + 1) as f32;
        for y in 0..h {
            let row = y * w;
            let mut acc = [0.0f32; CH];
            for _ in 0..=r {
                for c in 0..CH {
                    acc[c] += buf[row][c];
                }
            }
            for x in 1..=r {
                let sx = x.min(w - 1);
                for c in 0..CH {
                    acc[c] += buf[row + sx][c];
                }
            }
            for x in 0..w {
                for c in 0..CH {
                    scratch[row + x][c] = acc[c] * inv_x;
                }
                let rem = x.saturating_sub(r);
                let add = (x + r + 1).min(w - 1);
                for c in 0..CH {
                    acc[c] += buf[row + add][c] - buf[row + rem][c];
                }
            }
        }
        let inv_y = 1. / (2 * r + 1) as f32;
        for x in 0..w {
            let mut acc = [0.0f32; CH];
            for _ in 0..=r {
                for c in 0..CH {
                    acc[c] += scratch[x][c];
                }
            }
            for y in 1..=r {
                let sy = y.min(h - 1);
                for c in 0..CH {
                    acc[c] += scratch[sy * w + x][c];
                }
            }
            for y in 0..h {
                for c in 0..CH {
                    buf[y * w + x][c] = acc[c] * inv_y;
                }
                let rem = y.saturating_sub(r);
                let add = (y + r + 1).min(h - 1);
                for c in 0..CH {
                    acc[c] += scratch[add * w + x][c] - scratch[rem * w + x][c];
                }
            }
        }
    }
}

/// Signed distance to an axis-aligned rounded rectangle `[x0..x1] x [y0..y1]` with corner radius `r`.
fn sd_round_rect(px_x: f32, px_y: f32, x0: f32, y0: f32, x1: f32, y1: f32, r: f32) -> f32 {
    let cx = (x0 + x1) * 0.5;
    let cy = (y0 + y1) * 0.5;
    let hx = (x1 - x0) * 0.5 - r;
    let hy = (y1 - y0) * 0.5 - r;
    let dx = (px_x - cx).abs() - hx;
    let dy = (px_y - cy).abs() - hy;
    dx.max(0.).hypot(dy.max(0.)) + dx.max(dy).min(0.) - r
}

/// Signed distance to the `600 x 500` back folder silhouette with top-left tab ramp.
fn sd_folder_back(fx: f32, fy: f32) -> f32 {
    let y_top = if fx <= 176. {
        0.
    } else if fx >= 246. {
        38.
    } else {
        let s = (fx - 176.) / 70.;
        38. * s * s * (3. - 2. * s)
    };
    let r = 38.;
    if fx < r && fy < r {
        return (r - fx).hypot(r - fy) - r;
    }
    if fx > 600. - r && fy < 38. + r {
        return (fx - (600. - r)).hypot((38. + r) - fy) - r;
    }
    if fx < r && fy > 500. - r {
        return (r - fx).hypot(fy - (500. - r)) - r;
    }
    if fx > 600. - r && fy > 500. - r {
        return (fx - (600. - r)).hypot(fy - (500. - r)) - r;
    }
    let d_left = -fx;
    let d_right = fx - 600.;
    let d_bot = fy - 500.;
    let slope_norm = if (176.0..246.0).contains(&fx) {
        let s = (fx - 176.) / 70.;
        let dy_dx = 38. * 6. * s * (1. - s) / 70.;
        1. / (1. + dy_dx * dy_dx).sqrt()
    } else {
        1.
    };
    let d_top = (y_top - fy) * slope_norm;
    let dx_out = d_left.max(d_right).max(0.);
    let dy_out = d_top.max(d_bot).max(0.);
    if dx_out > 0. || dy_out > 0. {
        dx_out.hypot(dy_out)
    } else {
        d_left.max(d_right).max(d_top).max(d_bot)
    }
}

/// Signed distance to the foreshortened front pocket (`0..600` x `y_top..500`, flaring slightly when open).
fn sd_front_pocket(fx: f32, fy: f32, open_t: f32) -> f32 {
    let y_top = lerp(108., 124., open_t);
    let r_top = lerp(38., 18., open_t);
    let r_bot = 38.;
    let v = ((fy - y_top) / (500. - y_top)).clamp(0., 1.);
    let flare = 9.5 * open_t * (1. - v);
    let x_left = -flare;
    let x_right = 600. + flare;
    let r = if v < 0.5 { r_top } else { r_bot };
    let cx = 300.;
    let cy = (y_top + 500.) * 0.5;
    let hx = (x_right - x_left) * 0.5 - r;
    let hy = (500. - y_top) * 0.5 - r;
    let dx = (fx - cx).abs() - hx;
    let dy = (fy - cy).abs() - hy;
    dx.max(0.).hypot(dy.max(0.)) + dx.max(dy).min(0.) - r
}

/// Pose `(tx, ty, deg_clockwise)` in `600 x 500` folder units for document `slot` of `count`.
fn document_pose(slot: usize, count: usize, open_t: f32) -> (f32, f32, f32) {
    match count {
        0 => (160., 66., 0.),
        1 => (
            lerp(160., 164., open_t),
            lerp(68., 52., open_t),
            lerp(-3., -6., open_t),
        ),
        2 => match slot {
            0 => (
                lerp(72., 60., open_t),
                lerp(78., 66., open_t),
                lerp(5., 8.5, open_t),
            ),
            _ => (
                lerp(244., 254., open_t),
                lerp(68., 54., open_t),
                lerp(-4., -6.5, open_t),
            ),
        },
        3 => match slot {
            0 => (
                lerp(40., 32., open_t),
                lerp(80., 70., open_t),
                lerp(6., 9., open_t),
            ),
            1 => (
                lerp(280., 286., open_t),
                lerp(80., 76., open_t),
                lerp(6., 5., open_t),
            ),
            _ => (
                lerp(160., 166., open_t),
                lerp(66., 52., open_t),
                lerp(-4., -7., open_t),
            ),
        },
        _ => {
            let last = (count - 1).max(1) as f32;
            let u = slot as f32 / last;
            let base_x = lerp(26., 294., u);
            let fan_x = base_x + (u - 0.5) * 24. * open_t;
            let arch = (u - 0.5).abs() * 2.;
            let base_y = lerp(64., 84., arch);
            let fan_y = base_y - lerp(15., 6., arch) * open_t;
            let base_deg = if slot + 1 == count {
                -4.
            } else {
                lerp(7., 4., u)
            };
            let fan_deg = base_deg + if slot + 1 == count { -3. } else { 2.5 } * open_t;
            (fan_x, fan_y, fan_deg)
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn rasterize_glass_folder(
    key: FolderCacheKey,
    alpha_buf: &mut [[f32; 1]],
    alpha_tmp: &mut [[f32; 1]],
    shadow_a: &mut [[f32; 1]],
    shadow_b: &mut [[f32; 1]],
    scene_buf: &mut [[f32; 4]],
    blur_buf: &mut [[f32; 4]],
    blur_tmp: &mut [[f32; 4]],
) -> RenderImage {
    let open_t = (f32::from(key.open_q) / 1000.).clamp(0., 1.);
    let icon_scale = lerp(1.0, 1.03, open_t);
    // Map `600 x 500` folder space into the center of `TEX_W x TEX_H` with generous shadow bleed.
    let base_u_per_px = 600.0 / (TEX_W as f32 * 0.72);
    let u_per_px = base_u_per_px / icon_scale;
    let cx_px = TEX_W as f32 * 0.5;
    let cy_px = TEX_H as f32 * 0.485;

    let to_folder = |px_x: usize, px_y: usize| -> (f32, f32) {
        let fx = 300.0 + (px_x as f32 + 0.5 - cx_px) * u_per_px;
        let fy = 250.0 + (px_y as f32 + 0.5 - cy_px) * u_per_px;
        (fx, fy)
    };

    let aa = u_per_px.max(0.8);
    for y in 0..TEX_H {
        let row = y * TEX_W;
        for x in 0..TEX_W {
            let (fx, fy) = to_folder(x, y);
            let cov = (0.5 - sd_folder_back(fx, fy) / aa).clamp(0., 1.);
            alpha_buf[row + x] = [cov];
            shadow_a[row + x] = [cov];
            shadow_b[row + x] = [cov];
        }
    }

    let px_per_u = 1. / u_per_px;
    blur_planar(
        shadow_a,
        alpha_tmp,
        TEX_W,
        TEX_H,
        (27.0 * px_per_u).max(2.0),
    );
    blur_planar(
        shadow_b,
        alpha_tmp,
        TEX_W,
        TEX_H,
        (11.0 * px_per_u).max(1.2),
    );

    let accent_f = key.accent_rgb.map(|v| f32::from(v) / 255.);
    let (back_rgb, sh_rgb, sh_w1, sh_w2, glass_top, glass_bot, depth_rgb, depth_a) =
        if !key.custom_accent {
            if key.dark {
                (
                    [0x3d as f32 / 255., 0x8f as f32 / 255., 0xc6 as f32 / 255.],
                    [0.0, 0.0, 0.0],
                    0.45,
                    0.30,
                    [100. / 255., 190. / 255., 235. / 255., 0.42],
                    [45. / 255., 130. / 255., 190. / 255., 0.50],
                    [0.0, 20. / 255., 40. / 255.],
                    0.40,
                )
            } else {
                (
                    [0x5b as f32 / 255., 0xb7 as f32 / 255., 0xea as f32 / 255.],
                    [60. / 255., 130. / 255., 180. / 255.],
                    0.28,
                    0.16,
                    [132. / 255., 214. / 255., 248. / 255., 0.45],
                    [69. / 255., 170. / 255., 220. / 255., 0.45],
                    [0.0, 40. / 255., 75. / 255.],
                    0.28,
                )
            }
        } else if key.dark {
            let back = accent_f.map(|c| (c * 0.82).clamp(0., 1.));
            let gt = accent_f.map(|c| (c * 0.75 + 0.25).clamp(0., 1.));
            let gb = accent_f.map(|c| (c * 0.78).clamp(0., 1.));
            (
                back,
                [0.0, 0.0, 0.0],
                0.45,
                0.30,
                [gt[0], gt[1], gt[2], 0.42],
                [gb[0], gb[1], gb[2], 0.50],
                accent_f.map(|c| c * 0.16),
                0.40,
            )
        } else {
            let back = accent_f.map(|c| (c * 0.88 + 0.12).clamp(0., 1.));
            let sh = accent_f.map(|c| (c * 0.68).clamp(0., 1.));
            let gt = accent_f.map(|c| (c * 0.58 + 0.42).clamp(0., 1.));
            let gb = accent_f.map(|c| (c * 0.82 + 0.10).clamp(0., 1.));
            (
                back,
                sh,
                0.28,
                0.16,
                [gt[0], gt[1], gt[2], 0.45],
                [gb[0], gb[1], gb[2], 0.45],
                accent_f.map(|c| c * 0.24),
                0.28,
            )
        };

    // Inner top highlight on back folder: blur SourceAlpha slightly and subtract dy=3u offset.
    let mut back_blur = alpha_buf.to_vec();
    blur_planar(
        &mut back_blur,
        alpha_tmp,
        TEX_W,
        TEX_H,
        (4.0 * px_per_u).max(0.9),
    );
    let dy_sh1 = (24.0 * px_per_u).round() as isize;
    let dy_sh2 = (8.0 * px_per_u).round() as isize;
    let dy_hl = (3.0 * px_per_u).round().max(1.) as isize;

    scene_buf.fill([0.0; 4]);
    for y in 0..TEX_H {
        let row = y * TEX_W;
        let sy1 = (y as isize - dy_sh1).clamp(0, TEX_H as isize - 1) as usize;
        let sy2 = (y as isize - dy_sh2).clamp(0, TEX_H as isize - 1) as usize;
        let sy_hl = (y as isize - dy_hl).clamp(0, TEX_H as isize - 1) as usize;
        for x in 0..TEX_W {
            let s1 = shadow_a[sy1 * TEX_W + x][0] * sh_w1;
            let s2 = shadow_b[sy2 * TEX_W + x][0] * sh_w2;
            let mut px_c = blend_over([0.0; 4], sh_rgb, (s1 + s2).clamp(0., 0.65));

            let cov_b = alpha_buf[row + x][0];
            if cov_b > 0.0 {
                px_c = blend_over(px_c, back_rgb, cov_b);
                let diff = (back_blur[row + x][0] - back_blur[sy_hl * TEX_W + x][0]).max(0.);
                let hl_a = diff * cov_b * if key.dark { 0.45 } else { 0.65 };
                px_c = blend_over(px_c, [1.0, 1.0, 1.0], hl_a);
            }
            scene_buf[row + x] = px_c;
        }
    }

    // Paint the angled documents in SVG layer order (left, right, middle for 3 docs).
    let count = usize::from(key.doc_count).min(5);
    let (paper_top, paper_bot, paper_stroke, paper_stroke_a, line_rgb, line_a, doc_sh_a) =
        if key.dark {
            (
                [0x34 as f32 / 255., 0x36 as f32 / 255., 0x3c as f32 / 255.],
                [0x26 as f32 / 255., 0x29 as f32 / 255., 0x2f as f32 / 255.],
                [1.0, 1.0, 1.0],
                0.10,
                [1.0, 1.0, 1.0],
                0.24,
                0.42,
            )
        } else {
            (
                [1.0, 1.0, 1.0],
                [0xea as f32 / 255., 0xf2 as f32 / 255., 0xf8 as f32 / 255.],
                [0.0, 0.0, 0.0],
                0.12,
                [0.0, 0.0, 0.0],
                0.18,
                0.18,
            )
        };

    let par_x = f32::from(key.tilt_x_q) / 10.0;
    let par_y = f32::from(key.tilt_y_q) / 10.0;

    for slot in 0..count {
        let (base_tx, base_ty, deg) = document_pose(slot, count, open_t);
        let depth_factor = 0.65 + 0.35 * (slot as f32 / count.saturating_sub(1).max(1) as f32);
        let tx = base_tx + par_x * 9.5 * depth_factor;
        let ty = base_ty + par_y * 6.0 * depth_factor;
        let rad = (deg + par_x * 1.4 * depth_factor).to_radians();
        let (sin_r, cos_r) = rad.sin_cos();
        let pivot_x = tx + 140.;
        let pivot_y = ty + 175.;

        for y in 0..TEX_H {
            let row = y * TEX_W;
            for x in 0..TEX_W {
                let (fx, fy) = to_folder(x, y);
                // Clip document bottoms/sides cleanly within the folder bottom silhouette
                // while letting their upper portions emerge above the pocket.
                if fy > 500. || (fy > 125. && (fx < -2. || fx > 602.)) {
                    continue;
                }
                let rx = fx - pivot_x;
                let ry = fy - pivot_y;
                let dx = rx * cos_r + ry * sin_r + 140.;
                let dy = -rx * sin_r + ry * cos_r + 175.;

                // Soft document drop shadow (offset 0, +4.5u)
                let s_dy = -rx * sin_r + (ry - 4.5) * cos_r + 175.;
                let d_sh = sd_round_rect(dx, s_dy, 0.6, 0.6, 279.4, 349.4, 19.4);
                if d_sh < 10.0 {
                    let sh_cov = (1. - (d_sh.max(0.) / 10.0)).powi(2) * doc_sh_a;
                    let sh_tint = if key.dark {
                        [0.0, 0.0, 0.0]
                    } else {
                        [0.0, 30. / 255., 60. / 255.]
                    };
                    scene_buf[row + x] = blend_over(scene_buf[row + x], sh_tint, sh_cov);
                }

                let d_doc = sd_round_rect(dx, dy, 0.6, 0.6, 279.4, 349.4, 19.4);
                let cov_doc = (0.5 - d_doc / aa).clamp(0., 1.);
                if cov_doc <= 0.0 {
                    continue;
                }
                let gy = (dy / 350.).clamp(0., 1.);
                let fill_rgb = [
                    lerp(paper_top[0], paper_bot[0], gy),
                    lerp(paper_top[1], paper_bot[1], gy),
                    lerp(paper_top[2], paper_bot[2], gy),
                ];
                let mut p_c = blend_over(scene_buf[row + x], fill_rgb, cov_doc);

                // Placeholder rounded text bars
                if (28.0..=215.0).contains(&dx) && (80.0..=268.0).contains(&dy) {
                    for &(ly, lw) in &DOC_LINES {
                        if (dy - (ly + 4.5)).abs() <= 6.5 {
                            let dl = sd_round_rect(dx, dy, 30., ly, 30. + lw, ly + 9., 4.5);
                            let l_cov = (0.5 - dl / aa).clamp(0., 1.);
                            if l_cov > 0.0 {
                                p_c = blend_over(p_c, line_rgb, l_cov * line_a * cov_doc);
                            }
                        }
                    }
                }

                // 1.2u crisp paper border
                let border_band = (1. - (d_doc.abs() / (1.35 * aa))).clamp(0., 1.);
                if border_band > 0.0 {
                    p_c = blend_over(p_c, paper_stroke, border_band * paper_stroke_a * cov_doc);
                }
                scene_buf[row + x] = p_c;
            }
        }
    }

    // Frosted glass front pocket: blur a copy of `scene_buf` (`backdrop-filter: blur(0.55cqw) saturate(1.1)`)
    blur_buf.copy_from_slice(scene_buf);
    blur_planar(blur_buf, blur_tmp, TEX_W, TEX_H, (3.4 * px_per_u).max(0.8));

    let y_pocket_top = lerp(108., 124., open_t);
    let depth_start = if key.dark { 0.70 } else { 0.74 };
    let rim_ring_a = if key.dark { 0.14 } else { 0.30 };
    let rim_top_a = if key.dark { 0.30 } else { 0.55 };

    for y in 0..TEX_H {
        let row = y * TEX_W;
        for x in 0..TEX_W {
            let (fx, fy) = to_folder(x, y);
            let d_p = sd_front_pocket(fx, fy, open_t);
            let cov_p = (0.5 - d_p / aa).clamp(0., 1.);
            if cov_p <= 0.0 {
                continue;
            }

            // Sample blurred backdrop inside the pocket and boost saturation by 1.10
            let b_px = blur_buf[row + x];
            let ba = b_px[3].clamp(1e-4, 1.);
            let (mut br, mut bg_c, mut bb) = (b_px[0] / ba, b_px[1] / ba, b_px[2] / ba);
            let lum = 0.2126 * br + 0.7152 * bg_c + 0.0722 * bb;
            br = (lum + (br - lum) * 1.12).clamp(0., 1.);
            bg_c = (lum + (bg_c - lum) * 1.12).clamp(0., 1.);
            bb = (lum + (bb - lum) * 1.12).clamp(0., 1.);
            let mut frosted = [br * ba, bg_c * ba, bb * ba, b_px[3].clamp(0., 1.)];

            let pv = ((fy - y_pocket_top) / (500. - y_pocket_top)).clamp(0., 1.);
            let g_rgb = [
                lerp(glass_top[0], glass_bot[0], pv),
                lerp(glass_top[1], glass_bot[1], pv),
                lerp(glass_top[2], glass_bot[2], pv),
            ];
            let g_a = lerp(glass_top[3], glass_bot[3], pv);
            frosted = blend_over(frosted, g_rgb, g_a);

            if pv > depth_start {
                let dv = ((pv - depth_start) / (1. - depth_start)).clamp(0., 1.);
                frosted = blend_over(frosted, depth_rgb, dv * depth_a);
            }

            // Inset 4.0u (`0.67cqw`) glass rim + 1.6u (`0.25cqw`) specular top highlight
            let inner_depth = -d_p;
            let ring_cov = (1. - ((inner_depth - 2.0).abs() / (2.2 + 0.5 * aa))).clamp(0., 1.);
            if ring_cov > 0.0 {
                frosted = blend_over(frosted, [1.0, 1.0, 1.0], ring_cov * rim_ring_a);
            }
            let d_top_shift = sd_front_pocket(fx, fy - 1.8, open_t);
            if d_p < 0.0 && d_top_shift >= -0.6 {
                let top_lip = ((d_top_shift + 0.6) / 2.2).clamp(0., 1.);
                frosted = blend_over(frosted, [1.0, 1.0, 1.0], top_lip * rim_top_a);
            }

            let base = scene_buf[row + x];
            let inv = 1. - cov_p;
            scene_buf[row + x] = [
                base[0] * inv + frosted[0] * cov_p,
                base[1] * inv + frosted[1] * cov_p,
                base[2] * inv + frosted[2] * cov_p,
                base[3] * inv + frosted[3] * cov_p,
            ];
        }
    }

    // Un-premultiply into straight BGRA bytes for GPUI's RenderImage atlas (+ Bayer micro-dithering + edge dilation).
    const BAYER4: [u8; 16] = [0, 8, 2, 10, 12, 4, 14, 6, 3, 11, 1, 9, 15, 7, 13, 5];
    let mut raw = vec![0u8; TEX_W * TEX_H * 4];
    for y in 0..TEX_H {
        let row = y * TEX_W;
        for x in 0..TEX_W {
            let [pr, pg, pb, pa] = scene_buf[row + x];
            let a = pa.clamp(0., 1.);
            let dst = (row + x) * 4;
            if a > 1e-4 {
                let dither = (f32::from(BAYER4[(y & 3) * 4 + (x & 3)]) / 16.0 - 0.5) / 255.0;
                let r = (pr / a + dither).clamp(0., 1.);
                let g = (pg / a + dither).clamp(0., 1.);
                let b = (pb / a + dither).clamp(0., 1.);
                raw[dst] = (b * 255.).round() as u8;
                raw[dst + 1] = (g * 255.).round() as u8;
                raw[dst + 2] = (r * 255.).round() as u8;
                raw[dst + 3] = (a * 255.).round() as u8;
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

    let img_buf = RgbaImage::from_raw(TEX_W as u32, TEX_H as u32, raw)
        .expect("TEX_W * TEX_H * 4 matches raw buffer length");
    RenderImage::new(vec![Frame::new(img_buf)])
}

/// Glassy folder card illustration with angled inner documents and a tipping frosted pocket.
///
/// # Examples
///
/// ```ignore
/// use kui::components::glass_folder::GlassFolder;
///
/// let folder = cx.new(|cx| GlassFolder::new("kb-folder", cx));
/// ```
pub struct GlassFolder {
    id: ElementId,
    focus: FocusHandle,
    label: Option<SharedString>,
    accent: Option<Hsla>,
    doc_count: usize,
    doc_labels: Vec<SharedString>,
    hovered: bool,
    pinned_open: bool,
    hover_slide: Slide,
    open_slide: Slide,
    press_slide: Slide,
    tilt_x: Slide,
    tilt_y: Slide,
    width: Option<Pixels>,
    height: Option<Pixels>,
    loading: bool,
    empty: bool,
    clock: Instant,
    measured_w: Rc<Cell<f32>>,
    stage_bounds: Rc<Cell<Bounds<Pixels>>>,
    cache: Rc<RefCell<FolderSurfaceCache>>,
}

impl GlassFolder {
    /// Create a [`GlassFolder`] with three inner documents and default sky-blue frosted glass.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<UiLanguage>(|_, cx| cx.notify())
            .detach();
        Self {
            id: id.into(),
            focus: cx.focus_handle().tab_stop(true),
            label: None,
            accent: None,
            doc_count: 3,
            doc_labels: Vec::new(),
            hovered: false,
            pinned_open: false,
            hover_slide: Slide::new(0.),
            open_slide: Slide::new(0.),
            press_slide: Slide::snappy(0.),
            tilt_x: Slide::snappy(0.),
            tilt_y: Slide::snappy(0.),
            width: None,
            height: None,
            loading: false,
            empty: false,
            clock: cx.background_executor().now(),
            measured_w: Rc::new(Cell::new(420.)),
            stage_bounds: Rc::new(Cell::new(Bounds::default())),
            cache: Rc::new(RefCell::new(FolderSurfaceCache::new())),
        }
    }

    /// Set a custom accessible description on the root illustration container.
    pub fn with_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Override the folder accent color (deriving back tint, frosted glass stops, and ambient glow).
    pub fn with_color(mut self, color: Hsla) -> Self {
        self.accent = Some(color);
        self
    }

    /// Set how many angled document sheets sit behind the frosted pocket (`0..=5`, default `3`).
    pub fn with_document_count(mut self, count: usize) -> Self {
        self.doc_count = count.min(5);
        self
    }

    /// Attach named document labels, setting the sheet count and displaying subtle caption chips when open.
    pub fn with_documents<I, S>(mut self, labels: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        self.doc_labels = labels.into_iter().take(5).map(Into::into).collect();
        self.doc_count = self.doc_labels.len();
        self
    }

    /// Configure whether the folder starts tipped open.
    pub fn with_open(mut self, open: bool) -> Self {
        self.pinned_open = open;
        self.open_slide.reset(f32::from(open));
        self
    }

    /// Override the container width (maintaining a `3 : 2` aspect ratio unless `with_height` is also set).
    pub fn with_width(mut self, width: Pixels) -> Self {
        self.width = Some(width);
        self
    }

    /// Override the container height explicitly.
    pub fn with_height(mut self, height: Pixels) -> Self {
        self.height = Some(height);
        self
    }

    /// Configure the skeleton loading placeholder state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Configure the empty state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Return whether the folder is currently open (either hovered or pinned open).
    pub fn is_open(&self) -> bool {
        self.hovered || self.pinned_open
    }

    /// Return the configured document sheet count.
    pub fn document_count(&self) -> usize {
        self.doc_count
    }

    /// Mutate the pinned open state on a mounted entity.
    pub fn set_open(&mut self, open: bool, cx: &mut Context<Self>) {
        if self.pinned_open != open {
            self.pinned_open = open;
            self.open_slide.duration(f32::from(open), 0.48, cx);
            cx.notify();
        }
    }

    /// Mutate the folder accent color at runtime.
    pub fn set_color(&mut self, color: Hsla, cx: &mut Context<Self>) {
        self.accent = Some(color);
        cx.notify();
    }

    /// Return whether the skeleton loading state is active.
    pub fn is_loading(&self) -> bool {
        self.loading
    }

    /// Toggle the skeleton loading state at runtime.
    pub fn set_loading(&mut self, loading: bool, cx: &mut Context<Self>) {
        if self.loading != loading {
            self.loading = loading;
            cx.notify();
        }
    }

    /// Return whether the empty state is active.
    pub fn is_empty(&self) -> bool {
        self.empty
    }

    /// Toggle the empty state at runtime.
    pub fn set_empty(&mut self, empty: bool, cx: &mut Context<Self>) {
        if self.empty != empty {
            self.empty = empty;
            cx.notify();
        }
    }

    fn effective_open_target(&self, focused: bool) -> (f32, f32) {
        let peek = f32::from(self.hovered || (focused && !self.pinned_open));
        let full = f32::from(self.pinned_open);
        (peek, full)
    }
}

impl Render for GlassFolder {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let dark = Theme::global(cx).mode == ThemeMode::Dark;
        let reduced = disabled_motion(cx);
        let now = cx.background_executor().now();

        if self.loading {
            let phase = if reduced {
                0.0
            } else {
                w.request_animation_frame();
                (now.saturating_duration_since(self.clock).as_secs_f32() / 1.4).fract()
            };
            return div()
                .id(pro_eid(&self.id, "loading"))
                .w_full()
                .h(self.height.unwrap_or(px(260.)))
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap(px(12.))
                .child(pro_skeleton_bar(156., 126., 16., phase, cx))
                .child(pro_skeleton_bar(112., 12., 6., phase, cx))
                .into_any_element();
        }

        if self.empty {
            return div()
                .id(pro_eid(&self.id, "empty"))
                .w_full()
                .child(pro_empty_state::<Self>(
                    ProIcon::Folder,
                    tr(cx, "Empty folder", "空文件夹"),
                    tr(
                        cx,
                        "Add documents to preview them behind the frosted pocket.",
                        "添加文档后将在磨砂前袋后方展示。",
                    ),
                    None,
                    cx,
                ))
                .into_any_element();
        }

        let focused = self.focus.is_focused(w);
        let (target_peek, target_open) = self.effective_open_target(focused);
        self.hover_slide.duration(target_peek, 0.24, cx);
        self.open_slide.duration(target_open, 0.48, cx);

        let (hover_t, full_open_t, press_t, cur_tx, cur_ty) = if reduced {
            (target_peek, target_open, 0.0, 0.0, 0.0)
        } else {
            (
                self.hover_slide.get(w, cx).clamp(0., 1.),
                self.open_slide.get(w, cx).clamp(0., 1.),
                self.press_slide.get(w, cx).clamp(0., 1.),
                self.tilt_x.get(w, cx).clamp(-1., 1.),
                self.tilt_y.get(w, cx).clamp(-1., 1.),
            )
        };

        // Hover peeks ~30% open before clicking; clicking toggles full 100% open.
        let open_t = (full_open_t + 0.30 * hover_t * (1.0 - full_open_t)).clamp(0., 1.);

        let a11y_label = self.label.clone().unwrap_or_else(|| {
            tr(
                cx,
                "Blue folder holding three documents behind a frosted front pocket",
                "磨砂玻璃前袋中收纳三份文档的蓝色文件夹插画",
            )
            .into()
        });

        let measured_cell = self.measured_w.clone();
        let bounds_cell = self.stage_bounds.clone();
        let stage_w = self
            .width
            .map(f32::from)
            .unwrap_or_else(|| measured_cell.get().max(180.));
        let stage_h = self
            .height
            .map(f32::from)
            .unwrap_or_else(|| (stage_w * (2. / 3.)).round().max(160.));

        let (accent_rgb, custom_accent) = match self.accent {
            Some(c) => (hsla_to_rgb8(c), true),
            None => ([0x5b, 0xb7, 0xea], false),
        };
        let key = FolderCacheKey {
            dark,
            accent_rgb,
            custom_accent,
            doc_count: self.doc_count.min(5) as u8,
            open_q: (open_t * 1000.).round() as u16,
            tilt_x_q: (cur_tx * 10.).round().clamp(-10., 10.) as i8,
            tilt_y_q: (cur_ty * 10.).round().clamp(-10., 10.) as i8,
        };
        let cache = self.cache.clone();
        let focus_ring = p.focus;

        let mut root = div()
            .id(self.id.clone())
            .role(Role::Image)
            .aria_label(a11y_label)
            .track_focus(&self.focus)
            .tab_index(0)
            .relative()
            .overflow_hidden()
            .cursor_pointer()
            .border_1()
            .border_color(transparent_black())
            .focus_visible(move |s| s.rounded(px(14.)).border_color(focus_ring))
            .on_mouse_move(cx.listener(|this, e: &MouseMoveEvent, w, cx| {
                if w.last_input_was_touch() {
                    return;
                }
                let b = this.stage_bounds.get();
                let bw = f32::from(b.size.width);
                let bh = f32::from(b.size.height);
                if bw > 4. && bh > 4. {
                    let nx = ((f32::from(e.position.x - b.left()) / bw - 0.5) * 2.0).clamp(-1., 1.);
                    let ny = ((f32::from(e.position.y - b.top()) / bh - 0.5) * 2.0).clamp(-1., 1.);
                    this.tilt_x.duration(nx, 0.18, cx);
                    this.tilt_y.duration(ny, 0.18, cx);
                    cx.notify();
                }
            }))
            .on_hover(cx.listener(|this, hovered: &bool, _, cx| {
                if this.hovered != *hovered {
                    this.hovered = *hovered;
                    if !*hovered {
                        this.press_slide.set(0., cx);
                        this.tilt_x.duration(0., 0.36, cx);
                        this.tilt_y.duration(0., 0.36, cx);
                    }
                    cx.notify();
                }
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, w, cx| {
                    this.focus.focus(w, cx);
                    this.press_slide.set(1., cx);
                    cx.notify();
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.press_slide.set(0., cx);
                    cx.notify();
                }),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.press_slide.set(0., cx);
                    cx.notify();
                }),
            )
            .on_click(cx.listener(|this, _, _, cx| {
                this.pinned_open = !this.pinned_open;
                this.open_slide
                    .duration(f32::from(this.pinned_open), 0.48, cx);
                cx.emit(GlassFolderEvent::OpenChanged(this.pinned_open));
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, e: &KeyDownEvent, _, cx| {
                if matches!(e.keystroke.key.as_str(), "enter" | "space" | " ") {
                    this.press_slide.set(1., cx);
                    this.pinned_open = !this.pinned_open;
                    this.open_slide
                        .duration(f32::from(this.pinned_open), 0.48, cx);
                    cx.emit(GlassFolderEvent::OpenChanged(this.pinned_open));
                    cx.stop_propagation();
                    cx.notify();
                }
            }))
            .on_key_up(cx.listener(|this, e: &KeyUpEvent, _, cx| {
                if matches!(e.keystroke.key.as_str(), "enter" | "space" | " ") {
                    this.press_slide.set(0., cx);
                    cx.notify();
                }
            }));

        root = match self.width {
            Some(explicit_w) => root.w(explicit_w),
            None => root.w_full(),
        };
        root = root.h(px(stage_h));

        let width_sensor = canvas(
            move |bounds, window, _| {
                bounds_cell.set(bounds);
                let bw = f32::from(bounds.size.width);
                if bw > 20. && (bw - measured_cell.get()).abs() > 1.0 {
                    measured_cell.set(bw);
                    window.request_animation_frame();
                }
            },
            move |bounds, _, window, _| {
                let bw = f32::from(bounds.size.width);
                let bh = f32::from(bounds.size.height);
                if bw <= 4. || bh <= 4. {
                    return;
                }
                let fit_w = bw.min(bh * 1.5);
                let press_scale = 1.0 - 0.026 * press_t;
                let draw_w = fit_w * (0.31 / 0.72) * press_scale;
                let draw_h = draw_w * (TEX_H as f32 / TEX_W as f32);
                let par_dx = cur_tx * 5.5;
                let par_dy = cur_ty * 3.8 - hover_t * 2.2 + press_t * 1.8;
                let ox = bounds.origin.x + px((bw - draw_w) * 0.5 + par_dx);
                let oy = bounds.origin.y + px((bh - draw_h) * 0.5 + par_dy);
                let dest = Bounds::new(point(ox, oy), size(px(draw_w), px(draw_h)));
                let image = cache.borrow_mut().render_frame(key, window);
                let _ = window.paint_image(dest, dest, Corners::default(), image, 0, false);
            },
        )
        .absolute()
        .inset_0();

        root = root.child(width_sensor);

        if !self.doc_labels.is_empty() && open_t > 0.05 {
            let mut badges = div()
                .absolute()
                .bottom(px(12. - 4. * (1. - open_t)))
                .left_0()
                .right_0()
                .flex()
                .items_center()
                .justify_center()
                .gap(px(6.))
                .opacity((open_t * 1.25).clamp(0., 1.));
            for name in &self.doc_labels {
                badges = badges.child(
                    div()
                        .px(px(8.))
                        .py(px(2.))
                        .rounded(px(999.))
                        .bg(p.card.opacity(0.88))
                        .border_1()
                        .border_color(p.line)
                        .text_size(px(11.))
                        .text_color(p.text)
                        .child(name.clone()),
                );
            }
            root = root.child(badges);
        }

        root.into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        FolderCacheKey, GlassFolder, RenderImage, TEX_H, TEX_W, document_pose,
        rasterize_glass_folder, sd_folder_back, sd_front_pocket,
    };
    use gpui::{TestAppContext, point, px, size};
    use std::time::Duration;

    fn render_key(dark: bool, open_q: u16, doc_count: u8) -> RenderImage {
        let len = TEX_W * TEX_H;
        let mut ab = vec![[0.0; 1]; len];
        let mut at = vec![[0.0; 1]; len];
        let mut sa = vec![[0.0; 1]; len];
        let mut sb = vec![[0.0; 1]; len];
        let mut sc = vec![[0.0; 4]; len];
        let mut bb = vec![[0.0; 4]; len];
        let mut bt = vec![[0.0; 4]; len];
        rasterize_glass_folder(
            FolderCacheKey {
                dark,
                accent_rgb: [0x5b, 0xb7, 0xea],
                custom_accent: false,
                doc_count,
                open_q,
                tilt_x_q: 0,
                tilt_y_q: 0,
            },
            &mut ab,
            &mut at,
            &mut sa,
            &mut sb,
            &mut sc,
            &mut bb,
            &mut bt,
        )
    }

    #[test]
    fn folder_back_and_pocket_geometry_match_reference_bounds() {
        // Tab top-left interior is inside (< 0), while right shoulder above y=38 is outside (> 0).
        assert!(sd_folder_back(90., 20.) < -10.);
        assert!(sd_folder_back(420., 18.) > 10.);
        assert!(sd_folder_back(300., 260.) < -150.);

        // Front pocket tips downward from y=108 to y=124 and flares wider at top when opened.
        assert!(sd_front_pocket(300., 116., 0.0) < 0.);
        assert!(sd_front_pocket(300., 116., 1.0) > 0.);
        assert!(sd_front_pocket(-4., 135., 0.0) > 0.);
        assert!(sd_front_pocket(-4., 135., 1.0) < 0.);
    }

    #[test]
    fn hover_open_fans_documents_upward_and_preserves_transparent_corners() {
        let rest_mid = document_pose(2, 3, 0.0);
        let peek_mid = document_pose(2, 3, 0.30);
        let open_mid = document_pose(2, 3, 1.0);
        assert!(peek_mid.1 < rest_mid.1 && open_mid.1 < peek_mid.1);
        assert!(open_mid.2 < rest_mid.2);

        let light = render_key(false, 0, 3);
        let bytes = light.as_bytes(0).expect("BGRA slice");
        // Outer corners stay completely transparent so the folder sits cleanly on any card.
        assert_eq!(bytes[3], 0);
        // Center of folder pocket is opaque sky-blue frosted glass (B > R).
        let mid = ((TEX_H * 3 / 5) * TEX_W + TEX_W / 2) * 4;
        assert!(bytes[mid + 3] > 240);
        assert!(bytes[mid] > bytes[mid + 2]);

        let dark = render_key(true, 1000, 3);
        let dark_bytes = dark.as_bytes(0).expect("BGRA slice");
        // Upper paper region in dark mode renders darker slate than light mode's white paper.
        let paper_sample = ((TEX_H * 33 / 100) * TEX_W + TEX_W / 2) * 4;
        assert!(dark_bytes[paper_sample + 2] < bytes[paper_sample + 2]);
    }

    #[gpui::test]
    fn hover_peeks_thirty_percent_and_click_toggles_full_open_with_parallax(
        cx: &mut TestAppContext,
    ) {
        cx.update(crate::init);
        let exec = cx.executor();
        let (folder, window) = cx.add_window_view(|_, cx| GlassFolder::new("gf-peek", cx));
        window.simulate_resize(size(px(420.), px(280.)));
        window.run_until_parked();

        // Simulate pointer hover + parallax move in upper-right quadrant before clicking
        window.simulate_mouse_move(point(px(310.), px(90.)), None, gpui::Modifiers::default());
        window.run_until_parked();
        exec.advance_clock(Duration::from_millis(320));
        window.update(|w, app| w.simulate_next_frame(app));
        window.run_until_parked();

        let (hover_val, open_val, tx_val) = window.update(|_, app| {
            let f = folder.read(app);
            (
                f.hover_slide.value(app),
                f.open_slide.value(app),
                f.tilt_x.value(app),
            )
        });
        assert!(
            hover_val > 0.90,
            "expected hover_slide near 1.0, got {hover_val}"
        );
        assert!(
            open_val < 0.05,
            "expected pinned open_slide at 0 before click, got {open_val}"
        );
        let peek_open = full_blend(open_val, hover_val);
        assert!(
            (0.27..=0.33).contains(&peek_open),
            "expected ~30% hover-peek opening before click, got {peek_open}"
        );
        assert!(
            tx_val > 0.2,
            "expected positive pointer parallax tilt_x, got {tx_val}"
        );

        // Click toggles full open (`open_slide -> 1.0`)
        window.simulate_click(point(px(210.), px(140.)), gpui::Modifiers::default());
        exec.advance_clock(Duration::from_millis(600));
        window.update(|w, app| w.simulate_next_frame(app));
        window.run_until_parked();

        let full_val = window.update(|_, app| folder.read(app).open_slide.value(app));
        assert!(
            full_val > 0.96,
            "expected full open after click, got {full_val}"
        );
    }

    fn full_blend(full_open: f32, hover: f32) -> f32 {
        (full_open + 0.30 * hover * (1.0 - full_open)).clamp(0., 1.)
    }
}
