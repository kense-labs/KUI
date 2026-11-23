//! Blurred two-layer morphing color gradient anchored to the bottom edge.
//!
//! Renders a warm three-stop color crest and a `plus-lighter` highlight crest
//! whose peaks drift and oscillate smoothly while visible, pausing automatically
//! when [`Theme::reduced_motion`](crate::foundation::Theme::reduced_motion) is active
//! or `speed` is zero.

use crate::foundation::{Theme, ThemeMode};
use gpui::{prelude::*, *};
use image::{Frame, RgbaImage};
use std::{cell::RefCell, f32::consts::TAU, rc::Rc, sync::Arc};
use web_time::Instant;

const TEX_W: usize = 160;
const TEX_H: usize = 96;
const PAD_X: usize = 20;
const PAD_Y: usize = 16;
const GW: usize = TEX_W + 2 * PAD_X;
const GH: usize = TEX_H + 2 * PAD_Y;

#[derive(Clone, Copy, PartialEq, Eq)]
struct CacheKey {
    stops: [[u8; 3]; 3],
    bg: [u8; 3],
    aspect_q: u16,
    static_pose: bool,
}

struct LiveSurface {
    key: Option<CacheKey>,
    last_aspect: f32,
    last_phase: f32,
    image: Option<Arc<RenderImage>>,
    color_buf: Vec<[f32; 4]>,
    color_tmp: Vec<[f32; 4]>,
    light_buf: Vec<[f32; 1]>,
    light_tmp: Vec<[f32; 1]>,
}

impl LiveSurface {
    fn new() -> Self {
        Self {
            key: None,
            last_aspect: 0.45,
            last_phase: -2.0,
            image: None,
            color_buf: vec![[0.0; 4]; GW * GH],
            color_tmp: vec![[0.0; 4]; GW * GH],
            light_buf: vec![[0.0; 1]; GW * GH],
            light_tmp: vec![[0.0; 1]; GW * GH],
        }
    }

    fn quantize_aspect(&mut self, raw_aspect: f32) -> u16 {
        let clamped = raw_aspect.clamp(0.18, 2.50);
        if self.key.is_some() && (clamped - self.last_aspect).abs() < 0.015 {
            ((self.last_aspect * 100.).round() as u16).clamp(18, 250)
        } else {
            self.last_aspect = clamped;
            ((clamped * 100.).round() as u16).clamp(18, 250)
        }
    }

    fn frame(
        &mut self,
        stops: [[u8; 3]; 3],
        bg: [u8; 3],
        raw_aspect: f32,
        phase: f32,
        static_pose: bool,
        window: &mut Window,
    ) -> Arc<RenderImage> {
        let aspect_q = self.quantize_aspect(raw_aspect);
        let key = CacheKey {
            stops,
            bg,
            aspect_q,
            static_pose,
        };
        let phase_diff = if static_pose {
            0.0
        } else {
            let d = (phase - self.last_phase).abs();
            d.min(1.0 - d)
        };
        if self.key == Some(key)
            && (static_pose || phase_diff < 0.0022)
            && let Some(existing) = &self.image
        {
            return existing.clone();
        }

        let rendered = Arc::new(rasterize_floor_frame(
            key,
            if static_pose { -1.0 } else { phase },
            &mut self.color_buf,
            &mut self.color_tmp,
            &mut self.light_buf,
            &mut self.light_tmp,
        ));
        if let Some(prev) = self.image.replace(rendered.clone()) {
            let _ = window.drop_image(prev);
        }
        self.key = Some(key);
        self.last_phase = phase;
        rendered
    }
}

fn hsla_to_rgb8(color: Hsla) -> [u8; 3] {
    let rgba: Rgba = color.into();
    [
        (rgba.r.clamp(0., 1.) * 255.).round() as u8,
        (rgba.g.clamp(0., 1.) * 255.).round() as u8,
        (rgba.b.clamp(0., 1.) * 255.).round() as u8,
    ]
}

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

/// Smooth C-infinity trigonometric interpolation passing through `p0` at `t=0`,
/// `p33` at `t=1/3`, and `p66` at `t=2/3` without velocity dead-stops.
fn smooth_three_phase(
    t: f32,
    p33: (f32, f32, f32, f32),
    p66: (f32, f32, f32, f32),
) -> (f32, f32, f32, f32) {
    const INV_SQRT3: f32 = 0.577_350_26;
    let (s, c) = (t * TAU).sin_cos();
    let interp = |y0: f32, y1: f32, y2: f32| -> f32 {
        let a0 = (y0 + y1 + y2) * (1. / 3.);
        let a1 = y0 - a0;
        let b1 = (y1 - y2) * INV_SQRT3;
        a0 + a1 * c + b1 * s
    };
    (
        interp(0., p33.0, p66.0),
        interp(0., p33.1, p66.1),
        interp(1., p33.2, p66.2),
        interp(1., p33.3, p66.3),
    )
}

/// Evaluate crest Y in Layer-0 SVG coordinates (`0..3447.1` x `0..1514.1`) for a given `svg_x`.
fn crest_y_layer0(svg_x: f32, crests: [f32; 5]) -> Option<f32> {
    let [y_a, y_b, y_c, y_d, y_e] = crests;
    if !(291.3..=3155.8).contains(&svg_x) {
        return None;
    }
    let y = if svg_x <= 926.8 {
        let u = ((svg_x - 291.3) / (926.8 - 291.3)).clamp(0., 1.);
        let inv = 1. - u;
        inv * inv * inv * y_b
            + 3. * inv * inv * u * (y_b + 6.)
            + 3. * inv * u * u * y_a
            + u * u * u * y_a
    } else if svg_x <= 1230.8 {
        y_a
    } else if svg_x <= 1510.8 {
        let u = (svg_x - 1230.8) / (1510.8 - 1230.8);
        y_a + (y_e - y_a) * u
    } else if svg_x <= 1898.8 {
        y_e
    } else if svg_x <= 2189.3 {
        let u = (svg_x - 1898.8) / (2189.3 - 1898.8);
        y_e + (y_d - y_e) * u
    } else if svg_x <= 2493.8 {
        y_d
    } else {
        let u = (svg_x - 2493.8) / (3155.8 - 2493.8);
        y_d + (y_c - y_d) * u
    };
    Some(y)
}

/// Fast 3-pass separable box blur approximating a Gaussian blur with standard deviation `sigma`.
fn blur_planar<const CH: usize>(
    buf: &mut [[f32; CH]],
    scratch: &mut [[f32; CH]],
    w: usize,
    h: usize,
    sigma_x: f32,
    sigma_y: f32,
) {
    let rx = ((sigma_x * 0.90).round() as usize).clamp(1, w / 2);
    let ry = ((sigma_y * 0.90).round() as usize).clamp(1, h / 2);
    for _ in 0..3 {
        let inv_x = 1. / (2 * rx + 1) as f32;
        for y in 0..h {
            let row = y * w;
            let mut acc = [0.0f32; CH];
            for _ in 0..=rx {
                for c in 0..CH {
                    acc[c] += buf[row][c];
                }
            }
            for x in 1..=rx {
                let sx = x.min(w - 1);
                for c in 0..CH {
                    acc[c] += buf[row + sx][c];
                }
            }
            for x in 0..w {
                for c in 0..CH {
                    scratch[row + x][c] = acc[c] * inv_x;
                }
                let rem = x.saturating_sub(rx);
                let add = (x + rx + 1).min(w - 1);
                for c in 0..CH {
                    acc[c] += buf[row + add][c] - buf[row + rem][c];
                }
            }
        }
        let inv_y = 1. / (2 * ry + 1) as f32;
        for x in 0..w {
            let mut acc = [0.0f32; CH];
            for _ in 0..=ry {
                for c in 0..CH {
                    acc[c] += scratch[x][c];
                }
            }
            for y in 1..=ry {
                let sy = y.min(h - 1);
                for c in 0..CH {
                    acc[c] += scratch[sy * w + x][c];
                }
            }
            for y in 0..h {
                for c in 0..CH {
                    buf[y * w + x][c] = acc[c] * inv_y;
                }
                let rem = y.saturating_sub(ry);
                let add = (y + ry + 1).min(h - 1);
                for c in 0..CH {
                    acc[c] += scratch[add * w + x][c] - scratch[rem * w + x][c];
                }
            }
        }
    }
}

fn rasterize_floor_frame(
    key: CacheKey,
    phase: f32,
    color_buf: &mut [[f32; 4]],
    color_tmp: &mut [[f32; 4]],
    light_buf: &mut [[f32; 1]],
    light_tmp: &mut [[f32; 1]],
) -> RenderImage {
    let aspect = (key.aspect_q as f32 / 100.).clamp(0.18, 2.50);
    let cont_w = 2200.0f32;
    let cont_h = cont_w * aspect;

    let (crests0, drift_c, drift_l) = if phase < 0.0 {
        (
            [583.4, 389.3, 456.3, 587.8, 318.8],
            (0., 0., 1., 1.),
            (0., 0., 1., 1.),
        )
    } else {
        let shifted = (phase + 0.55).fract();
        let a = shifted * TAU;
        let crests = [
            565.65 + 79.55 * (a - 0.52).sin(),
            291.70 + 109.60 * (a + 2.25).sin(),
            361.45 + 109.85 * (a - 1.38).sin(),
            565.40 + 79.60 * (a + 3.10).sin(),
            429.30 + 109.50 * (a + 1.18).sin(),
        ];
        let dc = smooth_three_phase(shifted, (-120., -20., 1.05, 1.08), (100., 12., 0.98, 0.96));
        let dl = smooth_three_phase(shifted, (-100., -10., 1.03, 1.00), (85., 10., 0.98, 0.94));
        (crests, dc, dl)
    };

    let c0 = key.stops[0].map(|v| v as f32 / 255.);
    let c1 = key.stops[1].map(|v| v as f32 / 255.);
    let c2 = key.stops[2].map(|v| v as f32 / 255.);
    let sample_grad = |svg_y: f32| -> [f32; 3] {
        let g = ((svg_y - 423.3) / (894.3 - 423.3)).clamp(0., 1.);
        if g <= 0.302_885 {
            let u = g / 0.302_885;
            [
                c0[0] + (c1[0] - c0[0]) * u,
                c0[1] + (c1[1] - c0[1]) * u,
                c0[2] + (c1[2] - c0[2]) * u,
            ]
        } else if g <= 0.673_077 {
            let u = (g - 0.302_885) / (0.673_077 - 0.302_885);
            [
                c1[0] + (c2[0] - c1[0]) * u,
                c1[1] + (c2[1] - c1[1]) * u,
                c1[2] + (c2[2] - c1[2]) * u,
            ]
        } else {
            c2
        }
    };

    color_buf.fill([0.0; 4]);
    light_buf.fill([0.0; 1]);

    let c_left = -581.3f32;
    let c_top = cont_h - 738.3f32;
    let c_ox = c_left + 3447.1 * 0.5;
    let c_oy = c_top + 1514.1 * 0.7;

    let l_left = -310.5f32;
    let l_top = cont_h - 523.5f32;
    let l_ox = l_left + 2904.96 * 0.5;
    let l_oy = l_top + 1275.97 * 0.7;
    let l_scale_to_l0 = 3447.1 / 2904.96;

    // Precompute 1D crest heights across columns (200 evaluations instead of 26,000)
    let mut top_c = [None; GW];
    let mut top_l = [None; GW];
    for gx in 0..GW {
        let u = (gx as f32 - PAD_X as f32 + 0.5) / TEX_W as f32;
        let cx = u * cont_w;
        let lx0 = c_ox + (cx - drift_c.0 - c_ox) / drift_c.2 - c_left;
        top_c[gx] = crest_y_layer0(lx0, crests0);
        let lx1 = (l_ox + (cx - drift_l.0 - l_ox) / drift_l.2 - l_left) * l_scale_to_l0;
        top_l[gx] = crest_y_layer0(lx1, crests0);
    }

    let dy0 = (cont_h / TEX_H as f32 / drift_c.3).abs().max(1.0);
    let dy1 = ((cont_h / TEX_H as f32 / drift_l.3) * l_scale_to_l0)
        .abs()
        .max(1.0);

    for gy in 0..GH {
        let v = (gy as f32 - PAD_Y as f32 + 0.5) / TEX_H as f32;
        let cy = v * cont_h;
        let ly0 = c_oy + (cy - drift_c.1 - c_oy) / drift_c.3 - c_top;
        let ly1 = (l_oy + (cy - drift_l.1 - l_oy) / drift_l.3 - l_top) * l_scale_to_l0;
        let bot_cov0 = ((1222.8 - ly0) / dy0 + 0.5).clamp(0., 1.);
        let bot_cov1 = ((1222.8 - ly1) / dy1 + 0.5).clamp(0., 1.);
        let rgb = sample_grad(ly0);
        let row = gy * GW;

        for gx in 0..GW {
            if bot_cov0 > 0.0
                && let Some(ty) = top_c[gx]
            {
                let cov = (((ly0 - ty) / dy0 + 0.5).clamp(0., 1.)) * bot_cov0;
                if cov > 0.0 {
                    color_buf[row + gx] = [rgb[0] * cov, rgb[1] * cov, rgb[2] * cov, cov];
                }
            }
            if bot_cov1 > 0.0
                && let Some(ty) = top_l[gx]
            {
                let cov = (((ly1 - ty) / dy1 + 0.5).clamp(0., 1.)) * bot_cov1;
                if cov > 0.0 {
                    light_buf[row + gx] = [0.80 * cov];
                }
            }
        }
    }

    let px_per_unit_x = TEX_W as f32 / cont_w;
    let px_per_unit_y = TEX_H as f32 / cont_h;
    blur_planar(
        color_buf,
        color_tmp,
        GW,
        GH,
        145.65 * px_per_unit_x,
        145.65 * px_per_unit_y,
    );
    blur_planar(
        light_buf,
        light_tmp,
        GW,
        GH,
        122.74 * px_per_unit_x,
        122.74 * px_per_unit_y,
    );

    let bg = key.bg.map(|v| v as f32 / 255.);
    // GPUI RenderImage expects BGRA byte layout
    let mut raw = vec![0u8; TEX_W * TEX_H * 4];
    for y in 0..TEX_H {
        for x in 0..TEX_W {
            let src = (y + PAD_Y) * GW + (x + PAD_X);
            let [pr, pg, pb, ca] = color_buf[src];
            let ca = ca.clamp(0., 1.);
            let la = light_buf[src][0].clamp(0., 1.);
            let r = (bg[0] * (1. - ca) + pr + la).clamp(0., 1.);
            let g = (bg[1] * (1. - ca) + pg + la).clamp(0., 1.);
            let b = (bg[2] * (1. - ca) + pb + la).clamp(0., 1.);
            let dst = (y * TEX_W + x) * 4;
            raw[dst] = (b * 255.).round() as u8;
            raw[dst + 1] = (g * 255.).round() as u8;
            raw[dst + 2] = (r * 255.).round() as u8;
            raw[dst + 3] = 255;
        }
    }

    let img_buf = RgbaImage::from_raw(TEX_W as u32, TEX_H as u32, raw)
        .expect("TEX_W * TEX_H * 4 matches raw buffer length");
    RenderImage::new(vec![Frame::new(img_buf)])
}

type OverlayBuilder = Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>;

/// Interactive GPUI Gradient Floor backdrop with optional foreground content.
///
/// # Examples
///
/// ```
/// use gpui::{div, prelude::*};
/// use kui::components::gradient_floor::GradientFloor;
///
/// let hero = GradientFloor::new("hero-floor")
///     .with_speed(3.0)
///     .with_child(div().child("Ideas start to glow"));
/// ```
#[derive(IntoElement)]
pub struct GradientFloor {
    id: ElementId,
    colors: [Hsla; 3],
    speed: f32,
    paused: bool,
    background: Option<Hsla>,
    height: Pixels,
    radius: Option<Pixels>,
    child_builder: Option<OverlayBuilder>,
    started_at: Instant,
    surface: Rc<RefCell<LiveSurface>>,
}

impl GradientFloor {
    /// Construct a `GradientFloor` with default `["#FF7700", "#FF0055", "#0073FF"]` crest stops.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            colors: [
                rgb(0xff7700).into(),
                rgb(0xff0055).into(),
                rgb(0x0073ff).into(),
            ],
            speed: 3.0,
            paused: false,
            background: None,
            height: px(360.),
            radius: None,
            child_builder: None,
            started_at: Instant::now(),
            surface: Rc::new(RefCell::new(LiveSurface::new())),
        }
    }

    /// Override the three gradient stops from the crest downward.
    pub fn with_colors(mut self, colors: [Hsla; 3]) -> Self {
        self.colors = colors;
        self
    }

    /// Override the three gradient stops using `#rgb` or `#rrggbb` hex strings.
    pub fn with_hex_colors(mut self, hex: [&str; 3]) -> Self {
        for (slot, raw) in self.colors.iter_mut().zip(hex) {
            if let Some(parsed) = parse_hex_rgb(raw) {
                *slot = parsed;
            }
        }
        self
    }

    /// Set seconds per morph cycle (`3.0` default; `0.0` freezes the floor).
    pub fn with_speed(mut self, seconds: f32) -> Self {
        self.speed = seconds.max(0.);
        self
    }

    /// Pause or resume the morph loop while keeping the rendered frame visible.
    pub fn paused(mut self, paused: bool) -> Self {
        self.paused = paused;
        self
    }

    /// Builder alias to pause or resume the morph loop.
    pub fn with_paused(self, paused: bool) -> Self {
        self.paused(paused)
    }

    /// Override the resting backdrop color (`#fafafa` in light mode, `#111113` in dark mode).
    pub fn with_background(mut self, background: Hsla) -> Self {
        self.background = Some(background);
        self
    }

    /// Set the explicit height of the floor container (`360px` default).
    pub fn with_height(mut self, height: Pixels) -> Self {
        self.height = height;
        self
    }

    /// Override the container corner radius.
    pub fn with_radius(mut self, radius: Pixels) -> Self {
        self.radius = Some(radius);
        self
    }

    /// Mount a static element above the floor layers.
    pub fn with_child(mut self, child: impl IntoElement + 'static) -> Self {
        let cell = RefCell::new(Some(child.into_any_element()));
        self.child_builder = Some(Rc::new(move |_, _| {
            cell.borrow_mut()
                .take()
                .unwrap_or_else(|| div().into_any_element())
        }));
        self
    }

    /// Mount dynamic foreground content rendered above the floor on each frame.
    pub fn with_content(
        mut self,
        render: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        self.child_builder = Some(Rc::new(render));
        self
    }

    /// Alias for [`Self::with_content`] mounting foreground overlay elements above the floor.
    pub fn with_overlay(
        self,
        render: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        self.with_content(render)
    }

    /// Mutate colors on a mounted entity.
    pub fn set_colors(&mut self, colors: [Hsla; 3], cx: &mut Context<Self>) {
        self.colors = colors;
        cx.notify();
    }

    /// Mutate loop speed on a mounted entity.
    pub fn set_speed(&mut self, seconds: f32, cx: &mut Context<Self>) {
        self.speed = seconds.max(0.);
        cx.notify();
    }

    /// Mutate paused state on a mounted entity.
    pub fn set_paused(&mut self, paused: bool, cx: &mut Context<Self>) {
        self.paused = paused;
        cx.notify();
    }

    fn render_surface(
        &self,
        started_at: Instant,
        surface: Rc<RefCell<LiveSurface>>,
        w: &mut Window,
        cx: &mut App,
    ) -> Stateful<Div> {
        let theme = Theme::global(cx);
        let dark = theme.mode == ThemeMode::Dark;
        let bg = self.background.unwrap_or_else(|| {
            if dark {
                rgb(0x111113).into()
            } else {
                rgb(0xfafafa).into()
            }
        });
        let radius = self.radius.unwrap_or(px(20.));
        let speed = self.speed;
        let animate = !theme.reduced_motion && !self.paused && speed > 0.001;
        let stops = self.colors.map(hsla_to_rgb8);
        let bg_rgb = hsla_to_rgb8(bg);

        let mut root = div()
            .id(self.id.clone())
            .relative()
            .w_full()
            .h(self.height)
            .rounded(radius)
            .overflow_hidden()
            .bg(bg)
            .child(
                canvas(
                    |_, _, _| (),
                    move |bounds, _, window, _| {
                        let bw = f32::from(bounds.size.width);
                        let bh = f32::from(bounds.size.height);
                        if bw <= 2. || bh <= 2. {
                            return;
                        }
                        let phase = if animate {
                            (started_at.elapsed().as_secs_f32() / speed).fract()
                        } else {
                            0.0
                        };
                        let image = surface.borrow_mut().frame(
                            stops,
                            bg_rgb,
                            bh / bw,
                            phase,
                            !animate,
                            window,
                        );
                        let _ = window.paint_image(
                            bounds,
                            bounds,
                            Corners::all(radius),
                            image,
                            0,
                            false,
                        );
                        if animate {
                            window.request_animation_frame();
                        }
                    },
                )
                .absolute()
                .inset_0(),
            );

        if let Some(builder) = &self.child_builder {
            root = root.child(
                div()
                    .relative()
                    .size_full()
                    .flex()
                    .flex_col()
                    .child(builder(w, cx)),
            );
        }
        root
    }
}

impl Render for GradientFloor {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_surface(self.started_at, self.surface.clone(), w, cx)
    }
}

impl RenderOnce for GradientFloor {
    fn render(self, w: &mut Window, cx: &mut App) -> impl IntoElement {
        let state_id: ElementId = (self.id.clone(), "gf-live-state").into();
        let state = w.use_keyed_state(state_id, cx, |_, _| {
            (Instant::now(), Rc::new(RefCell::new(LiveSurface::new())))
        });
        let (started_at, surface) = state.read(cx).clone();
        self.render_surface(started_at, surface, w, cx)
    }
}

#[cfg(test)]
mod tests {
    use super::{CacheKey, GH, GW, TEX_H, TEX_W, rasterize_floor_frame, smooth_three_phase};

    #[test]
    fn floor_frame_blends_background_at_top_and_glows_at_bottom() {
        let key = CacheKey {
            stops: [[0xff, 0x77, 0x00], [0xff, 0x00, 0x55], [0x00, 0x73, 0xff]],
            bg: [0x11, 0x11, 0x13],
            aspect_q: 45,
            static_pose: false,
        };
        let mut cb = vec![[0.0; 4]; GW * GH];
        let mut ct = vec![[0.0; 4]; GW * GH];
        let mut lb = vec![[0.0; 1]; GW * GH];
        let mut lt = vec![[0.0; 1]; GW * GH];
        let img = rasterize_floor_frame(key, 0.25, &mut cb, &mut ct, &mut lb, &mut lt);
        let bytes = img.as_bytes(0).expect("rendered BGRA slice");
        let top_mid = (TEX_W / 2) * 4;
        assert!(bytes[top_mid] < 32 && bytes[top_mid + 1] < 32 && bytes[top_mid + 2] < 32);
        let bot_mid = ((TEX_H - 2) * TEX_W + TEX_W / 2) * 4;
        let lum = u16::from(bytes[bot_mid])
            + u16::from(bytes[bot_mid + 1])
            + u16::from(bytes[bot_mid + 2]);
        assert!(
            lum > 450,
            "expected bright glowing crest at bottom, got lum={lum}"
        );
    }

    #[test]
    fn harmonic_drift_hits_all_three_keyframes_smoothly() {
        let p33 = (-120., -20., 1.05, 1.08);
        let p66 = (100., 12., 0.98, 0.96);
        let a = smooth_three_phase(0.0, p33, p66);
        let b = smooth_three_phase(1.0 / 3.0, p33, p66);
        let c = smooth_three_phase(2.0 / 3.0, p33, p66);
        assert!((a.0 - 0.0).abs() < 1e-3 && (a.2 - 1.0).abs() < 1e-3);
        assert!((b.0 - p33.0).abs() < 1e-3 && (b.3 - p33.3).abs() < 1e-3);
        assert!((c.0 - p66.0).abs() < 1e-3 && (c.3 - p66.3).abs() < 1e-3);
    }
}
