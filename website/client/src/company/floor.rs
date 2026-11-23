//! Subtle KUI `GradientFloor` (`流光底幕`) backdrop for the Kense homepage hero (`[data-hero-floor]`).
//!
//! Uses the same two-layer 3-stop crest (`#FF7700` -> `#FF0055` -> `#0073FF` + additive white light
//! crest) and 3-pass separable Gaussian blur as `kui::components::gradient_floor::GradientFloor`,
//! rendered into a lightweight `160x96` `CanvasRenderingContext2d` surface and paused automatically
//! when scrolled out of view or when `prefers-reduced-motion` is active.

use crate::cover::{Frame, Result, listen};
use std::{cell::RefCell, f32::consts::TAU, rc::Rc};
use wasm_bindgen::{Clamped, JsCast, closure::Closure};
use web_sys::{
    CanvasRenderingContext2d, Document, Element, HtmlCanvasElement, ImageData,
    IntersectionObserver, IntersectionObserverEntry, MediaQueryList, MutationObserver,
    MutationObserverInit, Window,
};

const TEX_W: usize = 160;
const TEX_H: usize = 96;
const PAD_X: usize = 20;
const PAD_Y: usize = 16;
const GW: usize = TEX_W + 2 * PAD_X;
const GH: usize = TEX_H + 2 * PAD_Y;
const CYCLE_MS: f64 = 7800.0;

const STOPS: [[f32; 3]; 3] = [
    [1.0, 0x77 as f32 / 255.0, 0.0],
    [1.0, 0.0, 0x55 as f32 / 255.0],
    [0.0, 0x73 as f32 / 255.0, 1.0],
];

fn smooth_three_phase(
    t: f32,
    p33: (f32, f32, f32, f32),
    p66: (f32, f32, f32, f32),
) -> (f32, f32, f32, f32) {
    const INV_SQRT3: f32 = 0.577_350_26;
    let (s, c) = (t * TAU).sin_cos();
    let interp = |y0: f32, y1: f32, y2: f32| -> f32 {
        let a0 = (y0 + y1 + y2) * (1.0 / 3.0);
        let a1 = y0 - a0;
        let b1 = (y1 - y2) * INV_SQRT3;
        a0 + a1 * c + b1 * s
    };
    (
        interp(0.0, p33.0, p66.0),
        interp(0.0, p33.1, p66.1),
        interp(1.0, p33.2, p66.2),
        interp(1.0, p33.3, p66.3),
    )
}

fn crest_y_layer0(svg_x: f32, crests: [f32; 5]) -> Option<f32> {
    let [y_a, y_b, y_c, y_d, y_e] = crests;
    if !(291.3..=3155.8).contains(&svg_x) {
        return None;
    }
    let y = if svg_x <= 926.8 {
        let u = ((svg_x - 291.3) / (926.8 - 291.3)).clamp(0.0, 1.0);
        let inv = 1.0 - u;
        inv * inv * inv * y_b
            + 3.0 * inv * inv * u * (y_b + 6.0)
            + 3.0 * inv * u * u * y_a
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
        let inv_x = 1.0 / (2 * rx + 1) as f32;
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
        let inv_y = 1.0 / (2 * ry + 1) as f32;
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

fn sample_grad(svg_y: f32) -> [f32; 3] {
    let g = ((svg_y - 423.3) / (894.3 - 423.3)).clamp(0.0, 1.0);
    let [c0, c1, c2] = STOPS;
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
}

struct HeroFloor {
    page: Document,
    hero: Element,
    canvas: HtmlCanvasElement,
    ctx: CanvasRenderingContext2d,
    media: MediaQueryList,
    visible: bool,
    start_ms: Option<f64>,
    last_paint_ms: f64,
    color_buf: Vec<[f32; 4]>,
    color_tmp: Vec<[f32; 4]>,
    light_buf: Vec<[f32; 1]>,
    light_tmp: Vec<[f32; 1]>,
    rgba: Vec<u8>,
}

impl HeroFloor {
    fn is_dark(&self) -> bool {
        self.page
            .document_element()
            .and_then(|e| e.get_attribute("data-theme"))
            .as_deref()
            == Some("dark")
    }

    fn aspect(&self) -> f32 {
        let r = self.canvas.get_bounding_client_rect();
        if r.width() > 1.0 && r.height() > 1.0 {
            ((r.height() / r.width()) as f32).clamp(0.24, 0.85)
        } else {
            0.42
        }
    }

    fn render(&mut self, phase: f32) {
        let cont_w = 2200.0f32;
        let cont_h = cont_w * self.aspect();

        let (crests0, drift_c, drift_l) = if phase < 0.0 {
            (
                [583.4, 389.3, 456.3, 587.8, 318.8],
                (0.0, 0.0, 1.0, 1.0),
                (0.0, 0.0, 1.0, 1.0),
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
            let dc = smooth_three_phase(
                shifted,
                (-120.0, -20.0, 1.05, 1.08),
                (100.0, 12.0, 0.98, 0.96),
            );
            let dl = smooth_three_phase(
                shifted,
                (-100.0, -10.0, 1.03, 1.00),
                (85.0, 10.0, 0.98, 0.94),
            );
            (crests, dc, dl)
        };

        self.color_buf.fill([0.0; 4]);
        self.light_buf.fill([0.0; 1]);

        let c_left = -581.3f32;
        let c_top = cont_h - 738.3f32;
        let c_ox = c_left + 3447.1 * 0.5;
        let c_oy = c_top + 1514.1 * 0.7;

        let l_left = -310.5f32;
        let l_top = cont_h - 523.5f32;
        let l_ox = l_left + 2904.96 * 0.5;
        let l_oy = l_top + 1275.97 * 0.7;
        let l_scale_to_l0 = 3447.1 / 2904.96;

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
            let bot_cov0 = ((1222.8 - ly0) / dy0 + 0.5).clamp(0.0, 1.0);
            let bot_cov1 = ((1222.8 - ly1) / dy1 + 0.5).clamp(0.0, 1.0);
            let rgb = sample_grad(ly0);
            let row = gy * GW;

            for gx in 0..GW {
                if bot_cov0 > 0.0
                    && let Some(ty) = top_c[gx]
                {
                    let cov = (((ly0 - ty) / dy0 + 0.5).clamp(0.0, 1.0)) * bot_cov0;
                    if cov > 0.0 {
                        self.color_buf[row + gx] = [rgb[0] * cov, rgb[1] * cov, rgb[2] * cov, cov];
                    }
                }
                if bot_cov1 > 0.0
                    && let Some(ty) = top_l[gx]
                {
                    let cov = (((ly1 - ty) / dy1 + 0.5).clamp(0.0, 1.0)) * bot_cov1;
                    if cov > 0.0 {
                        self.light_buf[row + gx] = [0.80 * cov];
                    }
                }
            }
        }

        let px_per_unit_x = TEX_W as f32 / cont_w;
        let px_per_unit_y = TEX_H as f32 / cont_h;
        blur_planar(
            &mut self.color_buf,
            &mut self.color_tmp,
            GW,
            GH,
            145.65 * px_per_unit_x,
            145.65 * px_per_unit_y,
        );
        blur_planar(
            &mut self.light_buf,
            &mut self.light_tmp,
            GW,
            GH,
            122.74 * px_per_unit_x,
            122.74 * px_per_unit_y,
        );

        let bg = if self.is_dark() { 0.0f32 } else { 1.0f32 };
        for y in 0..TEX_H {
            for x in 0..TEX_W {
                let src = (y + PAD_Y) * GW + (x + PAD_X);
                let [pr, pg, pb, ca] = self.color_buf[src];
                let ca = ca.clamp(0.0, 1.0);
                let la = self.light_buf[src][0].clamp(0.0, 1.0);
                let r = (bg * (1.0 - ca) + pr + la).clamp(0.0, 1.0);
                let g = (bg * (1.0 - ca) + pg + la).clamp(0.0, 1.0);
                let b = (bg * (1.0 - ca) + pb + la).clamp(0.0, 1.0);
                let dst = (y * TEX_W + x) * 4;
                self.rgba[dst] = (r * 255.0).round() as u8;
                self.rgba[dst + 1] = (g * 255.0).round() as u8;
                self.rgba[dst + 2] = (b * 255.0).round() as u8;
                self.rgba[dst + 3] = 255;
            }
        }

        if let Ok(img) = ImageData::new_with_u8_clamped_array_and_sh(
            Clamped(&self.rgba),
            TEX_W as u32,
            TEX_H as u32,
        ) {
            let _ = self.ctx.put_image_data(&img, 0.0, 0.0);
            self.canvas.set_hidden(false);
        }
    }

    fn tick(&mut self, now: f64) -> bool {
        if !self.hero.is_connected() {
            return false;
        }
        let reduced = self.media.matches();
        if reduced {
            self.render(-1.0);
            return false;
        }
        if !self.visible || self.page.hidden() {
            return false;
        }
        let t0 = *self.start_ms.get_or_insert(now);
        if self.last_paint_ms == 0.0 || now - self.last_paint_ms >= 28.0 {
            self.last_paint_ms = now;
            let phase = (((now - t0) % CYCLE_MS) / CYCLE_MS) as f32;
            self.render(phase);
        }
        true
    }
}

pub(super) fn init(
    window: &Window,
    page: &Document,
    root: &Element,
    media: &MediaQueryList,
) -> Result {
    let Some(hero) = root.query_selector("[data-kense-hero]")? else {
        return Ok(());
    };
    let Some(canvas) = hero
        .query_selector("[data-hero-floor]")?
        .and_then(|n| n.dyn_into::<HtmlCanvasElement>().ok())
    else {
        return Ok(());
    };
    let Some(ctx) = canvas
        .get_context("2d")
        .ok()
        .flatten()
        .and_then(|c| c.dyn_into::<CanvasRenderingContext2d>().ok())
    else {
        return Ok(());
    };

    let mut floor = HeroFloor {
        page: page.clone(),
        hero: hero.clone(),
        canvas,
        ctx,
        media: media.clone(),
        visible: true,
        start_ms: None,
        last_paint_ms: 0.0,
        color_buf: vec![[0.0; 4]; GW * GH],
        color_tmp: vec![[0.0; 4]; GW * GH],
        light_buf: vec![[0.0; 1]; GW * GH],
        light_tmp: vec![[0.0; 1]; GW * GH],
        rgba: vec![0u8; TEX_W * TEX_H * 4],
    };
    // Paint immediate first frame so there is zero pop-in
    floor.render(if media.matches() { -1.0 } else { 0.0 });

    let state = Rc::new(RefCell::new(floor));
    let frame = Frame::new(window, {
        let state = state.clone();
        move |now| state.borrow_mut().tick(now)
    });

    let intersect = Closure::<dyn FnMut(js_sys::Array)>::new({
        let (state, frame) = (state.clone(), frame.clone());
        move |entries: js_sys::Array| {
            let mut s = state.borrow_mut();
            for entry in entries.iter() {
                if let Ok(e) = entry.dyn_into::<IntersectionObserverEntry>() {
                    s.visible = e.is_intersecting();
                }
            }
            if s.visible {
                frame.request();
            }
        }
    });
    let observer = IntersectionObserver::new(intersect.as_ref().unchecked_ref())?;
    observer.observe(&hero);
    intersect.forget();

    // Repaint immediately when light/dark theme toggles even if paused/reduced-motion
    if let Some(doc_el) = page.document_element() {
        let (state, frame) = (state.clone(), frame.clone());
        let mut_cb = Closure::<dyn FnMut()>::new(move || {
            state.borrow_mut().last_paint_ms = 0.0;
            frame.request();
        });
        let mo = MutationObserver::new(mut_cb.as_ref().unchecked_ref())?;
        let init = MutationObserverInit::new();
        init.set_attributes(true);
        let filter = js_sys::Array::of1(&"data-theme".into());
        init.set_attribute_filter(&filter);
        let _ = mo.observe_with_options(&doc_el, &init);
        mut_cb.forget();
    }

    for (target, ev) in [
        (page.as_ref(), "visibilitychange"),
        (media.as_ref(), "change"),
        (window.as_ref(), "resize"),
    ] {
        let (state, frame) = (state.clone(), frame.clone());
        listen(target, ev, move |_| {
            state.borrow_mut().last_paint_ms = 0.0;
            frame.request();
        })?;
    }

    frame.request();
    Ok(())
}
