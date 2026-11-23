//! Pure-Rust 3D beveled-mesh, silhouette contour-outline, and studio softbox renderer
//! for `#products` hardware assemblies and `#capabilities` mechanical miniatures.
//!
//! Renders true 3D chamfered-bevel triangle meshes into a single shared supersampled WebGL
//! offscreen framebuffer and blits onto visible 2D `<canvas>` targets so theme transitions
//! (`Surface::freeze`) capture every 3D solid cleanly with zero WebGL context pressure.

use super::fusion::{self, HostProjection, Phase};
use crate::cover::{Frame, Result, css, element, flag, listen};
use js_sys::{Array, Float32Array, Reflect, Uint16Array};
use std::{cell::RefCell, f32::consts::TAU, rc::Rc};
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use web_sys::{
    CanvasRenderingContext2d, Document, Element, HtmlCanvasElement, HtmlElement,
    IntersectionObserver, IntersectionObserverEntry, MediaQueryList, PointerEvent, ResizeObserver,
    WebGlBuffer, WebGlContextAttributes, WebGlProgram, WebGlRenderingContext as GL, WebGlShader,
    WebGlUniformLocation, Window,
};

const VERT_SRC: &str = r#"
attribute vec3 a_pos;
attribute vec3 a_norm;
attribute vec3 a_color;
attribute float a_mat;

uniform mat4 u_mvp;
uniform mat4 u_model;

varying vec3 v_norm;
varying vec3 v_world;
varying vec3 v_color;
varying float v_mat;

void main() {
    vec4 wp = u_model * vec4(a_pos, 1.0);
    vec3 wn = normalize((u_model * vec4(a_norm, 0.0)).xyz);
    v_norm = wn;
    v_world = wp.xyz;
    v_color = a_color;
    v_mat = a_mat;
    gl_Position = u_mvp * wp;
}
"#;

const FRAG_SRC: &str = r#"
precision highp float;

varying vec3 v_norm;
varying vec3 v_world;
varying vec3 v_color;
varying float v_mat;

uniform float u_dark;

void main() {
    // Contact shadow disc (a_mat == 4.0): v_color.xy carries local (-1..1) quad UV
    if (v_mat > 3.5) {
        float r2 = dot(v_color.xy, v_color.xy);
        float soft = exp(-r2 * 2.8) * smoothstep(1.0, 0.15, sqrt(r2));
        float a = soft * (u_dark > 0.5 ? 0.42 : 0.085);
        vec3 sc = vec3(0.02, 0.03, 0.05);
        gl_FragColor = vec4(sc * a, a);
        return;
    }

    // Screen glass diagonal sheen (a_mat == 3.0)
    if (v_mat > 2.5) {
        vec3 n = normalize(v_norm);
        float sweep = v_world.x * 0.42 + v_world.y * 0.24 - 0.18 + n.x * 0.8;
        float band = exp(-pow(sweep / 0.26, 2.0));
        float strip = exp(-pow((sweep - 0.42) / 0.06, 2.0));
        float a = 0.012 + band * 0.10 + strip * 0.055;
        vec3 gc = vec3(0.86, 0.92, 1.0);
        gl_FragColor = vec4(gc * a, a);
        return;
    }

    // Backlit Retina UI screen decal (a_mat == 2.0)
    if (v_mat > 1.5) {
        float backlight = 0.97 + 0.03 * cos(v_world.y * 0.8);
        gl_FragColor = vec4(clamp(v_color * backlight, 0.0, 1.0), 1.0);
        return;
    }

    vec3 n = normalize(v_norm);
    vec3 v = vec3(0.0, 0.0, 1.0);
    vec3 l_key = normalize(vec3(-0.46, 0.68, 0.82));
    vec3 l_fill = normalize(vec3(0.58, -0.22, 0.56));

    float ndl = max(dot(n, l_key), 0.0);
    float fill = max(dot(n, l_fill), 0.0);
    float hemi = n.y * 0.5 + 0.5;
    float ndv = max(dot(n, v), 0.0);
    float fres = pow(1.0 - ndv, 3.2);
    float contour = smoothstep(0.02, 0.34, ndv);

    if (v_mat < 0.5) {
        // Brushed Anodized Studio Alloy (a_mat == 0.0)
        vec3 sky = u_dark > 0.5 ? vec3(0.86, 0.90, 0.96) : vec3(0.99, 1.0, 1.0);
        vec3 gnd = u_dark > 0.5 ? vec3(0.18, 0.20, 0.25) : vec3(0.75, 0.78, 0.84);
        vec3 env = mix(gnd, sky, hemi);

        vec3 r = reflect(-v, n);
        float broad = exp(-pow((r.x + 0.25) / 0.52, 2.0) - pow((r.y - 0.28) / 0.68, 2.0));
        float strip = exp(-pow((r.x - 0.42 + v_world.y * 0.08) / 0.14, 2.0));
        vec3 h = normalize(l_key + v);
        float spec = pow(max(dot(n, h), 0.0), 42.0);

        vec3 col = v_color * (0.46 + 0.44 * env + 0.30 * ndl + 0.10 * fill);
        col += vec3(0.16) * broad + vec3(0.24, 0.26, 0.30) * strip * 0.55 + vec3(0.28) * spec;
        if (u_dark > 0.5) {
            col += vec3(0.16, 0.19, 0.25) * fres;
        } else {
            col *= mix(0.78, 1.0, contour);
            col += vec3(0.06, 0.08, 0.11) * fres;
        }
        gl_FragColor = vec4(clamp(col, 0.0, 1.0), 1.0);
    } else {
        // Matte Architectural Enamel / Ceramic (a_mat == 1.0)
        float grad = mix(0.88, 1.08, smoothstep(-1.35, 1.35, v_world.y));
        float wrap = clamp((dot(n, l_key) + 0.38) / 1.38, 0.0, 1.0);
        vec3 h = normalize(l_key + v);
        float sheen = pow(max(dot(n, h), 0.0), 22.0) * 0.14;
        vec3 col = v_color * grad * (0.58 + 0.36 * wrap + 0.10 * hemi + 0.06 * fill);
        col += vec3(1.0) * sheen;
        if (u_dark > 0.5) {
            col += vec3(0.12, 0.15, 0.20) * fres;
        } else {
            col *= mix(0.77, 1.0, contour);
            col += vec3(0.05, 0.06, 0.09) * fres;
        }
        gl_FragColor = vec4(clamp(col, 0.0, 1.0), 1.0);
    }
}
"#;

const MAT_ALLOY: f32 = 0.0;
const MAT_ENAMEL: f32 = 1.0;
const MAT_DISPLAY: f32 = 2.0;
const MAT_GLASS: f32 = 3.0;
const MAT_SHADOW: f32 = 4.0;

#[derive(Clone, Copy)]
struct Rgb(f32, f32, f32);

const fn hex(c: u32) -> Rgb {
    Rgb(
        ((c >> 16) & 0xff) as f32 / 255.0,
        ((c >> 8) & 0xff) as f32 / 255.0,
        (c & 0xff) as f32 / 255.0,
    )
}

// Studio Industrial Palette
const SILVER: Rgb = hex(0xDCE1EA);
const SILVER_DEEP: Rgb = hex(0xBCC4D2);
const GRAPHITE: Rgb = hex(0x232730);
const OBSIDIAN: Rgb = hex(0x14171D);
const KEYCAP: Rgb = hex(0x1E222B);
const PORCELAIN: Rgb = hex(0xF6F7FA);
const PAPER: Rgb = hex(0xFFFFFF);
const UI_MUTED: Rgb = hex(0xE8ECF3);
const UI_LINE: Rgb = hex(0xCBD3E1);
const UI_INK: Rgb = hex(0x1E2430);
const SAPPHIRE: Rgb = hex(0x2563EB);
const SKY_COBALT: Rgb = hex(0x60A5FA);
const NAVY_PLATE: Rgb = hex(0x1E3A8A);
const EMERALD: Rgb = hex(0x10B981);
const MINT: Rgb = hex(0x6EE7B7);
const AMBER: Rgb = hex(0xF59E0B);
const CORAL: Rgb = hex(0xF87171);
const TEAL: Rgb = hex(0x14B8A6);
const VIOLET: Rgb = hex(0x7C3AED);
const SLATE: Rgb = hex(0x64748B);

#[derive(Clone, Copy)]
struct Mat4([f32; 16]);

impl Mat4 {
    fn identity() -> Self {
        Self([
            1.0, 0.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0, //
            0.0, 0.0, 0.0, 1.0,
        ])
    }

    fn ortho(l: f32, r: f32, b: f32, t: f32, n: f32, f: f32) -> Self {
        Self([
            2.0 / (r - l),
            0.0,
            0.0,
            0.0,
            0.0,
            2.0 / (t - b),
            0.0,
            0.0,
            0.0,
            0.0,
            -2.0 / (f - n),
            0.0,
            -(r + l) / (r - l),
            -(t + b) / (t - b),
            -(f + n) / (f - n),
            1.0,
        ])
    }

    fn translate(x: f32, y: f32, z: f32) -> Self {
        Self([
            1.0, 0.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0, //
            x, y, z, 1.0,
        ])
    }

    fn scale(s: f32) -> Self {
        Self([
            s, 0.0, 0.0, 0.0, //
            0.0, s, 0.0, 0.0, //
            0.0, 0.0, s, 0.0, //
            0.0, 0.0, 0.0, 1.0,
        ])
    }

    fn scale_xyz(sx: f32, sy: f32, sz: f32) -> Self {
        Self([
            sx, 0.0, 0.0, 0.0, //
            0.0, sy, 0.0, 0.0, //
            0.0, 0.0, sz, 0.0, //
            0.0, 0.0, 0.0, 1.0,
        ])
    }

    fn rot_x(a: f32) -> Self {
        let (s, c) = a.sin_cos();
        Self([
            1.0, 0.0, 0.0, 0.0, //
            0.0, c, s, 0.0, //
            0.0, -s, c, 0.0, //
            0.0, 0.0, 0.0, 1.0,
        ])
    }

    fn rot_y(a: f32) -> Self {
        let (s, c) = a.sin_cos();
        Self([
            c, 0.0, -s, 0.0, //
            0.0, 1.0, 0.0, 0.0, //
            s, 0.0, c, 0.0, //
            0.0, 0.0, 0.0, 1.0,
        ])
    }

    fn rot_z(a: f32) -> Self {
        let (s, c) = a.sin_cos();
        Self([
            c, s, 0.0, 0.0, //
            -s, c, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0, //
            0.0, 0.0, 0.0, 1.0,
        ])
    }

    fn mul(self, rhs: Self) -> Self {
        let mut out = [0.0; 16];
        for col in 0..4 {
            for row in 0..4 {
                out[col * 4 + row] = self.0[row] * rhs.0[col * 4]
                    + self.0[4 + row] * rhs.0[col * 4 + 1]
                    + self.0[8 + row] * rhs.0[col * 4 + 2]
                    + self.0[12 + row] * rhs.0[col * 4 + 3];
            }
        }
        Self(out)
    }

    fn transform_pt(&self, p: [f32; 3]) -> [f32; 3] {
        [
            self.0[0] * p[0] + self.0[4] * p[1] + self.0[8] * p[2] + self.0[12],
            self.0[1] * p[0] + self.0[5] * p[1] + self.0[9] * p[2] + self.0[13],
            self.0[2] * p[0] + self.0[6] * p[1] + self.0[10] * p[2] + self.0[14],
        ]
    }

    fn transform_dir(&self, d: [f32; 3]) -> [f32; 3] {
        let x = self.0[0] * d[0] + self.0[4] * d[1] + self.0[8] * d[2];
        let y = self.0[1] * d[0] + self.0[5] * d[1] + self.0[9] * d[2];
        let z = self.0[2] * d[0] + self.0[6] * d[1] + self.0[10] * d[2];
        let len = (x * x + y * y + z * z).sqrt().max(1e-6);
        [x / len, y / len, z / len]
    }
}

#[derive(Default)]
struct MeshBuilder {
    verts: Vec<f32>,
    idx: Vec<u16>,
    xform: Option<Mat4>,
}

impl MeshBuilder {
    fn new() -> Self {
        Self::default()
    }

    fn with_xform(&mut self, m: Mat4, f: impl FnOnce(&mut Self)) {
        let prev = self.xform;
        self.xform = Some(prev.map_or(m, |p| p.mul(m)));
        f(self);
        self.xform = prev;
    }

    fn push_v(&mut self, pos: [f32; 3], norm: [f32; 3], rgb: Rgb, mat: f32) -> u16 {
        let (p, n) = if let Some(xf) = &self.xform {
            (xf.transform_pt(pos), xf.transform_dir(norm))
        } else {
            (pos, norm)
        };
        let id = (self.verts.len() / 10) as u16;
        self.verts
            .extend_from_slice(&[p[0], p[1], p[2], n[0], n[1], n[2], rgb.0, rgb.1, rgb.2, mat]);
        id
    }

    fn push_tri(&mut self, a: u16, b: u16, c: u16) {
        self.idx.extend_from_slice(&[a, b, c]);
    }

    fn push_quad(&mut self, a: u16, b: u16, c: u16, d: u16) {
        self.idx.extend_from_slice(&[a, b, c, a, c, d]);
    }

    fn rounded_contour(w: f32, h: f32, r: f32, segs: usize) -> Vec<(f32, f32, f32, f32)> {
        let r = r.min(w * 0.499).min(h * 0.499).max(0.002);
        let hx = (w * 0.5 - r).max(0.0);
        let hy = (h * 0.5 - r).max(0.0);
        let centers = [
            (hx, hy, 0.0_f32),
            (-hx, hy, TAU * 0.25),
            (-hx, -hy, TAU * 0.5),
            (hx, -hy, TAU * 0.75),
        ];
        let mut pts = Vec::with_capacity(4 * (segs + 1));
        for &(cx, cy, a0) in &centers {
            for s in 0..=segs {
                let a = a0 + (s as f32 / segs as f32) * (TAU * 0.25);
                let (sn, cs) = a.sin_cos();
                pts.push((cx + cs * r, cy + sn * r, cs, sn));
            }
        }
        pts
    }

    /// Watertight 3D rounded box with smooth 45-degree specular bevels.
    #[allow(clippy::too_many_arguments)]
    fn bevel_box(
        &mut self,
        cx: f32,
        cy: f32,
        cz: f32,
        w: f32,
        h: f32,
        d: f32,
        r: f32,
        bevel: f32,
        rgb: Rgb,
        mat: f32,
    ) {
        let b = bevel.min(d * 0.45).min(w * 0.22).min(h * 0.22).max(0.004);
        let contour = Self::rounded_contour(w - 2.0 * b, h - 2.0 * b, (r - b).max(0.002), 5);
        let n = contour.len();
        let hz = d * 0.5;
        // 6 profile rings from front (+hz) to back (-hz): (outset, z, z_norm_weight)
        let rings: [(f32, f32, f32); 6] = [
            (0.0, hz, 1.0),
            (b * 0.55, hz - b * 0.22, 0.82),
            (b, hz - b, 0.0),
            (b, -hz + b, 0.0),
            (b * 0.55, -hz + b * 0.22, -0.82),
            (0.0, -hz, -1.0),
        ];

        let mut ring_ids = Vec::with_capacity(6);
        for &(out, rz, nz) in &rings {
            let xy_w = (1.0 - nz * nz).max(0.0).sqrt();
            let base = (self.verts.len() / 10) as u16;
            for &(px, py, nx, ny) in &contour {
                self.push_v(
                    [cx + px + nx * out, cy + py + ny * out, cz + rz],
                    [nx * xy_w, ny * xy_w, nz],
                    rgb,
                    mat,
                );
            }
            ring_ids.push(base);
        }

        for r_idx in 0..5 {
            let r0 = ring_ids[r_idx];
            let r1 = ring_ids[r_idx + 1];
            for i in 0..n {
                let ni = (i + 1) % n;
                self.push_quad(r0 + i as u16, r1 + i as u16, r1 + ni as u16, r0 + ni as u16);
            }
        }

        let front_c = self.push_v([cx, cy, cz + hz], [0.0, 0.0, 1.0], rgb, mat);
        let r_front = ring_ids[0];
        for i in 0..n {
            let ni = (i + 1) % n;
            self.push_tri(front_c, r_front + i as u16, r_front + ni as u16);
        }

        let back_c = self.push_v([cx, cy, cz - hz], [0.0, 0.0, -1.0], rgb, mat);
        let r_back = ring_ids[5];
        for i in 0..n {
            let ni = (i + 1) % n;
            self.push_tri(back_c, r_back + ni as u16, r_back + i as u16);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn bevel_disc(
        &mut self,
        cx: f32,
        cy: f32,
        cz: f32,
        radius: f32,
        d: f32,
        bevel: f32,
        rgb: Rgb,
        mat: f32,
    ) {
        let b = bevel.min(d * 0.45).min(radius * 0.35).max(0.003);
        let n = 28usize;
        let inner_r = (radius - b).max(0.004);
        let hz = d * 0.5;
        let rings: [(f32, f32, f32); 6] = [
            (0.0, hz, 1.0),
            (b * 0.55, hz - b * 0.22, 0.82),
            (b, hz - b, 0.0),
            (b, -hz + b, 0.0),
            (b * 0.55, -hz + b * 0.22, -0.82),
            (0.0, -hz, -1.0),
        ];
        let mut ring_ids = Vec::with_capacity(6);
        for &(out, rz, nz) in &rings {
            let xy_w = (1.0 - nz * nz).max(0.0).sqrt();
            let base = (self.verts.len() / 10) as u16;
            for i in 0..n {
                let a = (i as f32 / n as f32) * TAU;
                let (sn, cs) = a.sin_cos();
                let rr = inner_r + out;
                self.push_v(
                    [cx + cs * rr, cy + sn * rr, cz + rz],
                    [cs * xy_w, sn * xy_w, nz],
                    rgb,
                    mat,
                );
            }
            ring_ids.push(base);
        }
        for r_idx in 0..5 {
            let r0 = ring_ids[r_idx];
            let r1 = ring_ids[r_idx + 1];
            for i in 0..n {
                let ni = (i + 1) % n;
                self.push_quad(r0 + i as u16, r1 + i as u16, r1 + ni as u16, r0 + ni as u16);
            }
        }
        let fc = self.push_v([cx, cy, cz + hz], [0.0, 0.0, 1.0], rgb, mat);
        let rf = ring_ids[0];
        for i in 0..n {
            self.push_tri(fc, rf + i as u16, rf + ((i + 1) % n) as u16);
        }
        let bc = self.push_v([cx, cy, cz - hz], [0.0, 0.0, -1.0], rgb, mat);
        let rb = ring_ids[5];
        for i in 0..n {
            self.push_tri(bc, rb + ((i + 1) % n) as u16, rb + i as u16);
        }
    }

    /// Hollow 3D beveled rounded frame or circular ring (`outer_r == outer_w * 0.5` -> circular ring).
    #[allow(clippy::too_many_arguments)]
    fn hollow_frame(
        &mut self,
        cx: f32,
        cy: f32,
        cz: f32,
        outer_w: f32,
        outer_h: f32,
        outer_r: f32,
        thickness: f32,
        d: f32,
        rgb: Rgb,
        mat: f32,
    ) {
        let outer = Self::rounded_contour(outer_w, outer_h, outer_r, 6);
        let inner_w = (outer_w - 2.0 * thickness).max(0.04);
        let inner_h = (outer_h - 2.0 * thickness).max(0.04);
        let inner_r = (outer_r - thickness).max(0.01);
        let inner = Self::rounded_contour(inner_w, inner_h, inner_r, 6);
        let n = outer.len();
        let hz = d * 0.5;

        let f_out = (self.verts.len() / 10) as u16;
        for &(px, py, nx, ny) in &outer {
            self.push_v(
                [cx + px, cy + py, cz + hz],
                [nx * 0.35, ny * 0.35, 0.94],
                rgb,
                mat,
            );
        }
        let f_in = (self.verts.len() / 10) as u16;
        for &(px, py, nx, ny) in &inner {
            self.push_v(
                [cx + px, cy + py, cz + hz],
                [-nx * 0.35, -ny * 0.35, 0.94],
                rgb,
                mat,
            );
        }
        let b_out = (self.verts.len() / 10) as u16;
        for &(px, py, nx, ny) in &outer {
            self.push_v(
                [cx + px, cy + py, cz - hz],
                [nx * 0.35, ny * 0.35, -0.94],
                rgb,
                mat,
            );
        }
        let b_in = (self.verts.len() / 10) as u16;
        for &(px, py, nx, ny) in &inner {
            self.push_v(
                [cx + px, cy + py, cz - hz],
                [-nx * 0.35, -ny * 0.35, -0.94],
                rgb,
                mat,
            );
        }

        let s_fout = (self.verts.len() / 10) as u16;
        for &(px, py, nx, ny) in &outer {
            self.push_v([cx + px, cy + py, cz + hz], [nx, ny, 0.0], rgb, mat);
        }
        let s_bout = (self.verts.len() / 10) as u16;
        for &(px, py, nx, ny) in &outer {
            self.push_v([cx + px, cy + py, cz - hz], [nx, ny, 0.0], rgb, mat);
        }
        let s_fin = (self.verts.len() / 10) as u16;
        for &(px, py, nx, ny) in &inner {
            self.push_v([cx + px, cy + py, cz + hz], [-nx, -ny, 0.0], rgb, mat);
        }
        let s_bin = (self.verts.len() / 10) as u16;
        for &(px, py, nx, ny) in &inner {
            self.push_v([cx + px, cy + py, cz - hz], [-nx, -ny, 0.0], rgb, mat);
        }

        for i in 0..n {
            let ni = (i + 1) % n;
            let (i, ni) = (i as u16, ni as u16);
            self.push_quad(f_in + i, f_out + i, f_out + ni, f_in + ni);
            self.push_quad(b_out + i, b_in + i, b_in + ni, b_out + ni);
            self.push_quad(s_fout + i, s_bout + i, s_bout + ni, s_fout + ni);
            self.push_quad(s_bin + i, s_fin + i, s_fin + ni, s_bin + ni);
        }
    }

    /// Flat rounded-rectangle UI decal (`outline` automatically skipped via `mat >= 2.0`).
    #[allow(clippy::too_many_arguments)]
    fn decal(&mut self, cx: f32, cy: f32, cz: f32, w: f32, h: f32, r: f32, rgb: Rgb, mat: f32) {
        let contour = Self::rounded_contour(w, h, r, 4);
        let n = contour.len();
        let center = self.push_v([cx, cy, cz], [0.0, 0.0, 1.0], rgb, mat);
        let base = (self.verts.len() / 10) as u16;
        for &(px, py, _, _) in &contour {
            self.push_v([cx + px, cy + py, cz], [0.0, 0.0, 1.0], rgb, mat);
        }
        for i in 0..n {
            self.push_tri(center, base + i as u16, base + ((i + 1) % n) as u16);
        }
    }

    /// Tubular U-shaped padlock shackle in XY plane.
    #[allow(clippy::too_many_arguments)]
    fn u_shackle(
        &mut self,
        cx: f32,
        cy: f32,
        cz: f32,
        arch_r: f32,
        leg_h: f32,
        tube_r: f32,
        rgb: Rgb,
        mat: f32,
    ) {
        let mut spine: Vec<([f32; 2], [f32; 2])> = Vec::new();
        spine.push(([-arch_r, -leg_h], [-1.0, 0.0]));
        let steps = 16usize;
        for s in 0..=steps {
            let a = std::f32::consts::PI * (1.0 - s as f32 / steps as f32);
            let (sn, cs) = a.sin_cos();
            spine.push(([cs * arch_r, sn * arch_r], [cs, sn]));
        }
        spine.push(([arch_r, -leg_h], [1.0, 0.0]));

        let ring_n = 14usize;
        let mut rings = Vec::with_capacity(spine.len());
        for &(sp, n2) in &spine {
            let base = (self.verts.len() / 10) as u16;
            for j in 0..ring_n {
                let phi = (j as f32 / ring_n as f32) * TAU;
                let (sp_phi, cp_phi) = phi.sin_cos();
                let nx = n2[0] * cp_phi;
                let ny = n2[1] * cp_phi;
                let nz = sp_phi;
                self.push_v(
                    [
                        cx + sp[0] + nx * tube_r,
                        cy + sp[1] + ny * tube_r,
                        cz + nz * tube_r,
                    ],
                    [nx, ny, nz],
                    rgb,
                    mat,
                );
            }
            rings.push(base);
        }
        for s in 0..(rings.len() - 1) {
            let r0 = rings[s];
            let r1 = rings[s + 1];
            for j in 0..ring_n {
                let nj = (j + 1) % ring_n;
                self.push_quad(r0 + j as u16, r0 + nj as u16, r1 + nj as u16, r1 + j as u16);
            }
        }
    }

    /// Sculpted 3D Verification Medallion (Emerald cylinder + white face + crisp 3D checkmark).
    fn verified_medallion(&mut self, cx: f32, cy: f32, cz: f32, r: f32) {
        self.bevel_disc(cx, cy, cz, r, r * 0.42, r * 0.09, EMERALD, MAT_ENAMEL);
        self.bevel_disc(
            cx,
            cy,
            cz + r * 0.16,
            r * 0.74,
            r * 0.14,
            r * 0.04,
            PAPER,
            MAT_ENAMEL,
        );
        self.with_xform(
            Mat4::translate(cx - r * 0.14, cy - r * 0.05, cz + r * 0.25).mul(Mat4::rot_z(-0.78)),
            |m| {
                m.bevel_box(
                    0.0,
                    0.0,
                    0.0,
                    r * 0.34,
                    r * 0.13,
                    r * 0.08,
                    r * 0.05,
                    0.01,
                    EMERALD,
                    MAT_ENAMEL,
                );
            },
        );
        self.with_xform(
            Mat4::translate(cx + r * 0.11, cy + r * 0.04, cz + r * 0.25).mul(Mat4::rot_z(0.78)),
            |m| {
                m.bevel_box(
                    0.0,
                    0.0,
                    0.0,
                    r * 0.58,
                    r * 0.13,
                    r * 0.08,
                    r * 0.05,
                    0.01,
                    EMERALD,
                    MAT_ENAMEL,
                );
            },
        );
    }

    fn ground_shadow(&mut self, y: f32, rx: f32, rz: f32) {
        let a = self.push_v(
            [-rx, y, -rz],
            [0.0, 1.0, 0.0],
            Rgb(-1.0, -1.0, 0.0),
            MAT_SHADOW,
        );
        let b = self.push_v(
            [-rx, y, rz],
            [0.0, 1.0, 0.0],
            Rgb(-1.0, 1.0, 0.0),
            MAT_SHADOW,
        );
        let c = self.push_v([rx, y, rz], [0.0, 1.0, 0.0], Rgb(1.0, 1.0, 0.0), MAT_SHADOW);
        let d = self.push_v(
            [rx, y, -rz],
            [0.0, 1.0, 0.0],
            Rgb(1.0, -1.0, 0.0),
            MAT_SHADOW,
        );
        self.push_quad(a, b, c, d);
    }

    /// Extruded 3D KUI signature slanted parallelogram eyes (`19:11` box, two `5px` bars, `skewX(-7deg)`).
    fn kui_eyes(&mut self, total_w: f32, depth: f32, rgb: Rgb) {
        let bar_w = total_w * (5.0 / 19.0);
        let bar_h = total_w * (11.0 / 19.0);
        let hw = bar_w * 0.5;
        let hh = bar_h * 0.5;
        let hz = depth * 0.5;
        // CSS `skewX(-7deg)` leans top (+Y in 3D)向右 by `tan(7°) * y`
        let skew = 0.1228 * hh;

        for cx in [-total_w * (7.0 / 19.0), total_w * (7.0 / 19.0)] {
            let corners = [
                [cx - hw - skew, -hh],
                [cx + hw - skew, -hh],
                [cx + hw + skew, hh],
                [cx - hw + skew, hh],
            ];
            let f0 = self.push_v(
                [corners[0][0], corners[0][1], hz],
                [0.0, 0.0, 1.0],
                rgb,
                MAT_DISPLAY,
            );
            let f1 = self.push_v(
                [corners[1][0], corners[1][1], hz],
                [0.0, 0.0, 1.0],
                rgb,
                MAT_DISPLAY,
            );
            let f2 = self.push_v(
                [corners[2][0], corners[2][1], hz],
                [0.0, 0.0, 1.0],
                rgb,
                MAT_DISPLAY,
            );
            let f3 = self.push_v(
                [corners[3][0], corners[3][1], hz],
                [0.0, 0.0, 1.0],
                rgb,
                MAT_DISPLAY,
            );
            self.push_quad(f0, f1, f2, f3);

            for i in 0..4 {
                let p0 = corners[i];
                let p1 = corners[(i + 1) % 4];
                let dx = p1[0] - p0[0];
                let dy = p1[1] - p0[1];
                let len = (dx * dx + dy * dy).sqrt().max(1e-5);
                let n = [dy / len, -dx / len, 0.18];
                let s0 = self.push_v([p0[0], p0[1], hz], n, rgb, MAT_DISPLAY);
                let s1 = self.push_v([p0[0], p0[1], -hz], n, rgb, MAT_DISPLAY);
                let s2 = self.push_v([p1[0], p1[1], -hz], n, rgb, MAT_DISPLAY);
                let s3 = self.push_v([p1[0], p1[1], hz], n, rgb, MAT_DISPLAY);
                self.push_quad(s0, s1, s2, s3);
            }
        }
    }
}

struct GpuPart {
    vbo: WebGlBuffer,
    ibo: WebGlBuffer,
    count: i32,
}

impl GpuPart {
    fn upload(gl: &GL, mb: &MeshBuilder, _solid: bool) -> Result<Self> {
        let vbo = gl.create_buffer().ok_or("vbo")?;
        gl.bind_buffer(GL::ARRAY_BUFFER, Some(&vbo));
        let v_arr = Float32Array::from(mb.verts.as_slice());
        gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &v_arr, GL::STATIC_DRAW);

        let ibo = gl.create_buffer().ok_or("ibo")?;
        gl.bind_buffer(GL::ELEMENT_ARRAY_BUFFER, Some(&ibo));
        let i_arr = Uint16Array::from(mb.idx.as_slice());
        gl.buffer_data_with_array_buffer_view(GL::ELEMENT_ARRAY_BUFFER, &i_arr, GL::STATIC_DRAW);

        Ok(Self {
            vbo,
            ibo,
            count: mb.idx.len() as i32,
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SceneKind {
    Runtime,
    Ui,
    Agent,
    ClayVault,
    ClayShield,
    ClayRelease,
    ClayRust,
}

impl SceneKind {
    fn parse(s: &str) -> Self {
        match s {
            "runtime" => Self::Runtime,
            "ui" => Self::Ui,
            "agent" => Self::Agent,
            "clay-vault" => Self::ClayVault,
            "clay-shield" => Self::ClayShield,
            "clay-release" => Self::ClayRelease,
            _ => Self::ClayRust,
        }
    }

    fn base_angles(self) -> (f32, f32, f32) {
        match self {
            Self::Runtime => (-0.28, 0.19, 3.05),
            Self::Ui => (0.27, 0.17, 3.12),
            Self::Agent => (-0.21, 0.14, 3.05),
            Self::ClayVault => (-0.16, 0.14, 2.38),
            Self::ClayShield => (0.18, 0.15, 2.18),
            Self::ClayRelease => (-0.17, 0.21, 2.32),
            Self::ClayRust => (0.19, 0.15, 2.42),
        }
    }
}

struct FusionHost {
    part_idx: usize,
    local_anchor: [f32; 3],
    world_w: f32,
    eye_w: f32,
    symbols: GpuPart,
    eyes: GpuPart,
}

struct SceneMeshes {
    shadow: GpuPart,
    parts: Vec<GpuPart>,
    host: FusionHost,
}

fn make_eyes(gl: &GL, eye_w: f32, rgb: Rgb) -> Result<GpuPart> {
    let mut mb = MeshBuilder::new();
    mb.kui_eyes(eye_w, 0.024, rgb);
    GpuPart::upload(gl, &mb, false)
}

fn build_runtime_scene(gl: &GL) -> Result<SceneMeshes> {
    let mut sh = MeshBuilder::new();
    sh.ground_shadow(-1.32, 2.55, 1.25);

    // Part 0: Machined Anodized MicroVM Compute Blade + Telemetry Display
    let mut blade = MeshBuilder::new();
    blade.bevel_box(
        0.28, 0.04, 0.0, 3.08, 1.98, 0.34, 0.22, 0.045, SILVER, MAT_ALLOY,
    );
    blade.bevel_box(
        0.28, 0.04, 0.14, 2.80, 1.70, 0.10, 0.15, 0.02, OBSIDIAN, MAT_ENAMEL,
    );
    for &(bx, by) in &[(-1.08, 0.78), (1.64, 0.78), (-1.08, -0.70), (1.64, -0.70)] {
        blade.bevel_disc(bx, by, 0.18, 0.055, 0.05, 0.015, SILVER_DEEP, MAT_ALLOY);
    }
    for i in 0..5 {
        let ly = -0.44 + (i as f32) * 0.24;
        blade.bevel_box(
            1.83, ly, 0.0, 0.05, 0.13, 0.20, 0.02, 0.008, GRAPHITE, MAT_ENAMEL,
        );
    }
    // Backlit dark-porcelain telemetry canvas
    blade.decal(0.28, 0.04, 0.194, 2.58, 1.48, 0.10, PORCELAIN, MAT_DISPLAY);
    blade.decal(0.28, 0.56, 0.198, 2.32, 0.24, 0.06, PAPER, MAT_DISPLAY);
    blade.decal(-0.72, 0.56, 0.202, 0.10, 0.10, 0.05, EMERALD, MAT_DISPLAY);
    blade.decal(-0.24, 0.56, 0.202, 0.68, 0.065, 0.03, UI_INK, MAT_DISPLAY);
    blade.decal(1.18, 0.56, 0.202, 0.36, 0.13, 0.065, MINT, MAT_DISPLAY);
    // 8 MicroVM warm-pool load bars
    for i in 0..8 {
        let bx = -0.72 + (i as f32) * 0.285;
        blade.decal(bx, 0.02, 0.198, 0.19, 0.62, 0.045, UI_MUTED, MAT_DISPLAY);
        let fill_h = [0.46, 0.54, 0.38, 0.56, 0.48, 0.52, 0.30, 0.22][i];
        let col = if i < 4 {
            SAPPHIRE
        } else if i < 6 {
            EMERALD
        } else {
            UI_LINE
        };
        blade.decal(
            bx,
            0.02 - (0.62 - fill_h) * 0.5,
            0.202,
            0.19,
            fill_h,
            0.045,
            col,
            MAT_DISPLAY,
        );
    }
    blade.decal(0.28, -0.49, 0.198, 2.32, 0.22, 0.06, PAPER, MAT_DISPLAY);
    blade.decal(
        -0.42,
        -0.49,
        0.202,
        0.78,
        0.055,
        0.025,
        UI_LINE,
        MAT_DISPLAY,
    );
    blade.decal(1.06, -0.49, 0.202, 0.58, 0.13, 0.065, SAPPHIRE, MAT_DISPLAY);
    blade.decal(0.28, 0.04, 0.206, 2.58, 1.48, 0.10, PAPER, MAT_GLASS);

    // Part 1: Docking SVID Hardware Security Key (left side — Bot fusion host!)
    let mut key_mod = MeshBuilder::new();
    key_mod.bevel_box(
        -1.18,
        0.04,
        0.02,
        0.46,
        0.44,
        0.12,
        0.04,
        0.015,
        SILVER_DEEP,
        MAT_ALLOY,
    );
    for dy in [-0.11, 0.0, 0.11] {
        key_mod.bevel_box(
            -1.08,
            0.04 + dy,
            0.09,
            0.22,
            0.055,
            0.03,
            0.02,
            0.006,
            AMBER,
            MAT_ENAMEL,
        );
    }
    key_mod.bevel_box(
        -1.68, 0.04, 0.04, 0.92, 0.74, 0.24, 0.14, 0.035, SAPPHIRE, MAT_ENAMEL,
    );
    key_mod.hollow_frame(
        -2.22, 0.04, 0.04, 0.34, 0.42, 0.16, 0.075, 0.10, SILVER, MAT_ALLOY,
    );

    let mut sym = MeshBuilder::new();
    sym.decal(0.04, 0.09, 0.0, 0.46, 0.08, 0.04, PAPER, MAT_DISPLAY);
    sym.decal(-0.04, -0.09, 0.0, 0.30, 0.06, 0.03, SKY_COBALT, MAT_DISPLAY);

    // Part 2: Floating Emerald Verified Medallion
    let mut med = MeshBuilder::new();
    med.verified_medallion(1.68, 0.86, 0.42, 0.38);

    Ok(SceneMeshes {
        shadow: GpuPart::upload(gl, &sh, false)?,
        parts: vec![
            GpuPart::upload(gl, &blade, true)?,
            GpuPart::upload(gl, &key_mod, true)?,
            GpuPart::upload(gl, &med, true)?,
        ],
        host: FusionHost {
            part_idx: 1,
            local_anchor: [-1.66, 0.04, 0.172],
            world_w: 0.92,
            eye_w: 0.38,
            symbols: GpuPart::upload(gl, &sym, false)?,
            eyes: make_eyes(gl, 0.38, PAPER)?,
        },
    })
}

fn build_ui_scene(gl: &GL) -> Result<SceneMeshes> {
    let mut sh = MeshBuilder::new();
    sh.ground_shadow(-1.38, 2.65, 1.35);

    // Part 0: Anodized-Silver Studio MacBook (Open Lid + Hinged 30-Key Base)
    let mut laptop = MeshBuilder::new();
    laptop.with_xform(Mat4::translate(-0.48, 0.30, -0.18), |m| {
        m.bevel_box(
            0.0, 0.0, 0.0, 3.24, 2.08, 0.14, 0.16, 0.03, SILVER, MAT_ALLOY,
        );
        m.bevel_box(
            0.0, 0.0, 0.055, 3.04, 1.88, 0.05, 0.11, 0.01, OBSIDIAN, MAT_ENAMEL,
        );
        m.decal(0.0, 0.0, 0.085, 2.86, 1.70, 0.08, PORCELAIN, MAT_DISPLAY);
        // Top window bar + 3 traffic-light dots
        m.decal(0.0, 0.74, 0.089, 2.86, 0.22, 0.05, PAPER, MAT_DISPLAY);
        for (i, col) in [CORAL, AMBER, EMERALD].into_iter().enumerate() {
            m.decal(
                -1.26 + (i as f32) * 0.11,
                0.74,
                0.093,
                0.058,
                0.058,
                0.029,
                col,
                MAT_DISPLAY,
            );
        }
        m.decal(0.0, 0.74, 0.093, 0.62, 0.05, 0.025, UI_LINE, MAT_DISPLAY);
        // Sidebar
        m.decal(-1.08, -0.11, 0.089, 0.70, 1.48, 0.04, UI_MUTED, MAT_DISPLAY);
        for r in 0..5 {
            let ry = 0.42 - (r as f32) * 0.22;
            if r == 0 {
                m.decal(-1.08, ry, 0.092, 0.58, 0.15, 0.04, PAPER, MAT_DISPLAY);
                m.decal(-1.28, ry, 0.095, 0.06, 0.06, 0.02, SAPPHIRE, MAT_DISPLAY);
                m.decal(-1.00, ry, 0.095, 0.34, 0.045, 0.02, UI_INK, MAT_DISPLAY);
            } else {
                m.decal(-1.12, ry, 0.093, 0.42, 0.042, 0.02, UI_LINE, MAT_DISPLAY);
            }
        }
        // Main GPUI component sheet preview
        m.decal(0.34, -0.06, 0.090, 1.92, 1.24, 0.09, PAPER, MAT_DISPLAY);
        m.decal(0.34, 0.38, 0.094, 1.64, 0.18, 0.09, UI_MUTED, MAT_DISPLAY);
        m.decal(-0.18, 0.38, 0.097, 0.54, 0.13, 0.065, PAPER, MAT_DISPLAY);
        m.decal(-0.10, 0.10, 0.094, 0.78, 0.055, 0.025, UI_INK, MAT_DISPLAY);
        m.decal(0.02, -0.08, 0.094, 1.02, 0.045, 0.02, UI_LINE, MAT_DISPLAY);
        m.decal(-0.28, -0.40, 0.094, 0.32, 0.15, 0.075, EMERALD, MAT_DISPLAY);
        m.decal(-0.20, -0.40, 0.097, 0.11, 0.11, 0.055, PAPER, MAT_DISPLAY);
        m.decal(0.78, -0.40, 0.095, 0.68, 0.20, 0.06, SAPPHIRE, MAT_DISPLAY);
        m.decal(0.78, -0.40, 0.099, 0.38, 0.045, 0.02, PAPER, MAT_DISPLAY);
        m.decal(0.0, 0.0, 0.102, 2.86, 1.70, 0.08, PAPER, MAT_GLASS);
    });
    // Cylindrical hinge + tilted keyboard deck with 30 tactile 3D keycaps
    laptop.with_xform(
        Mat4::translate(-0.48, -0.74, -0.12).mul(Mat4::rot_y(std::f32::consts::FRAC_PI_2)),
        |m| {
            m.bevel_disc(0.0, 0.0, 0.0, 0.085, 2.60, 0.02, SILVER_DEEP, MAT_ALLOY);
        },
    );
    laptop.with_xform(
        Mat4::translate(-0.48, -0.74, -0.10).mul(Mat4::rot_x(-1.16)),
        |m| {
            m.bevel_box(
                0.0, -0.84, 0.0, 3.30, 1.68, 0.12, 0.14, 0.03, SILVER, MAT_ALLOY,
            );
            m.decal(0.0, -0.62, 0.063, 2.86, 0.84, 0.05, GRAPHITE, MAT_ENAMEL);
            for row in 0..3 {
                for col in 0..10 {
                    let kx = -1.24 + (col as f32) * 0.275;
                    let ky = -0.34 - (row as f32) * 0.26;
                    m.bevel_box(
                        kx, ky, 0.075, 0.22, 0.19, 0.04, 0.03, 0.008, KEYCAP, MAT_ENAMEL,
                    );
                }
            }
            m.decal(0.0, -1.32, 0.064, 0.96, 0.42, 0.05, SILVER_DEEP, MAT_ALLOY);
        },
    );

    // Part 1: Floating Companion Smartphone / WASM Frame (centered in its own local space — Bot fusion host!)
    let mut phone = MeshBuilder::new();
    phone.bevel_box(
        0.0, 0.0, 0.0, 1.12, 2.20, 0.18, 0.20, 0.035, OBSIDIAN, MAT_ENAMEL,
    );
    phone.bevel_box(
        0.57,
        0.36,
        0.0,
        0.04,
        0.28,
        0.08,
        0.015,
        0.005,
        SILVER_DEEP,
        MAT_ALLOY,
    );
    phone.decal(0.0, 0.0, 0.094, 0.96, 2.02, 0.14, PORCELAIN, MAT_DISPLAY);
    phone.decal(0.0, 0.86, 0.098, 0.30, 0.075, 0.037, OBSIDIAN, MAT_DISPLAY);
    phone.decal(0.0, 0.28, 0.098, 0.80, 0.68, 0.08, PAPER, MAT_DISPLAY);
    phone.decal(0.0, -0.36, 0.099, 0.78, 0.22, 0.07, SAPPHIRE, MAT_DISPLAY);
    phone.decal(0.0, -0.36, 0.103, 0.38, 0.045, 0.02, PAPER, MAT_DISPLAY);
    phone.decal(0.0, -0.88, 0.098, 0.34, 0.032, 0.016, UI_INK, MAT_DISPLAY);
    phone.decal(0.0, 0.0, 0.106, 0.96, 2.02, 0.14, PAPER, MAT_GLASS);

    let mut sym = MeshBuilder::new();
    sym.decal(-0.08, 0.16, 0.0, 0.46, 0.055, 0.025, UI_INK, MAT_DISPLAY);
    sym.decal(0.0, 0.0, 0.0, 0.62, 0.045, 0.02, UI_LINE, MAT_DISPLAY);
    sym.decal(-0.18, -0.20, 0.0, 0.24, 0.11, 0.055, EMERALD, MAT_DISPLAY);

    // Part 2: Floating Interlocking KUI Sapphire Ribbon Frames (centered at local origin)
    let mut badge = MeshBuilder::new();
    badge.hollow_frame(
        -0.13, 0.11, -0.05, 0.48, 0.48, 0.14, 0.105, 0.11, SKY_COBALT, MAT_ENAMEL,
    );
    badge.hollow_frame(
        0.13, -0.11, 0.05, 0.48, 0.48, 0.14, 0.105, 0.11, SAPPHIRE, MAT_ENAMEL,
    );

    Ok(SceneMeshes {
        shadow: GpuPart::upload(gl, &sh, false)?,
        parts: vec![
            GpuPart::upload(gl, &laptop, true)?,
            GpuPart::upload(gl, &phone, true)?,
            GpuPart::upload(gl, &badge, true)?,
        ],
        host: FusionHost {
            part_idx: 1,
            local_anchor: [0.0, 0.28, 0.112],
            world_w: 0.86,
            eye_w: 0.36,
            symbols: GpuPart::upload(gl, &sym, false)?,
            eyes: make_eyes(gl, 0.36, OBSIDIAN)?,
        },
    })
}

fn build_agent_scene(gl: &GL) -> Result<SceneMeshes> {
    let mut sh = MeshBuilder::new();
    sh.ground_shadow(-1.34, 2.50, 1.25);

    // Part 0: Brushed-Aluminum Workbench Backplate + Tabbed Warm-Porcelain Folio Pocket
    let mut dock = MeshBuilder::new();
    dock.bevel_box(
        0.0, -0.04, -0.34, 3.44, 2.08, 0.15, 0.20, 0.035, SILVER, MAT_ALLOY,
    );
    // Tabbed front pocket + folder tab
    dock.bevel_box(
        -1.02, 0.16, 0.22, 1.26, 0.42, 0.18, 0.12, 0.03, PORCELAIN, MAT_ENAMEL,
    );
    dock.bevel_box(
        0.0, -0.48, 0.24, 3.48, 1.24, 0.20, 0.18, 0.04, PORCELAIN, MAT_ENAMEL,
    );
    for x in [-1.46, 1.46] {
        dock.bevel_disc(x, -0.06, 0.35, 0.048, 0.05, 0.015, SILVER_DEEP, MAT_ALLOY);
    }
    // Recessed dark terminal window on front of folio (Bot fusion host!)
    dock.bevel_box(
        0.0, -0.48, 0.35, 1.56, 0.48, 0.08, 0.13, 0.02, OBSIDIAN, MAT_ENAMEL,
    );

    let mut sym = MeshBuilder::new();
    sym.decal(-0.54, 0.0, 0.0, 0.09, 0.09, 0.045, EMERALD, MAT_DISPLAY);
    sym.decal(0.08, 0.0, 0.0, 0.86, 0.06, 0.03, PAPER, MAT_DISPLAY);

    // Parts 1, 2, 3: 3 Cascading Protocol & Artifact Cards (`ACP`, `AI OS / MCP`, `KCL`)
    let card_spec = [
        (-1.06_f32, 0.24_f32, -0.12_f32, PAPER, TEAL, UI_INK, UI_LINE),
        (0.0, 0.44, -0.02, SAPPHIRE, PAPER, PAPER, SKY_COBALT),
        (1.06, 0.18, -0.12, PAPER, VIOLET, UI_INK, UI_LINE),
    ];
    let mut cards = Vec::with_capacity(3);
    for (cx, cy, cz, bg, pill, line1, line2) in card_spec {
        let mut c = MeshBuilder::new();
        c.bevel_box(cx, cy, cz, 1.04, 1.76, 0.12, 0.14, 0.025, bg, MAT_ENAMEL);
        c.decal(
            cx - 0.18,
            cy + 0.62,
            cz + 0.065,
            0.44,
            0.12,
            0.06,
            pill,
            MAT_DISPLAY,
        );
        c.decal(
            cx - 0.06,
            cy + 0.34,
            cz + 0.065,
            0.68,
            0.06,
            0.03,
            line1,
            MAT_DISPLAY,
        );
        c.decal(
            cx - 0.12,
            cy + 0.16,
            cz + 0.065,
            0.56,
            0.05,
            0.025,
            line2,
            MAT_DISPLAY,
        );
        c.decal(
            cx - 0.08,
            cy - 0.02,
            cz + 0.065,
            0.64,
            0.05,
            0.025,
            line2,
            MAT_DISPLAY,
        );
        cards.push(GpuPart::upload(gl, &c, true)?);
    }

    // Part 4: Floating Emerald Verified Medallion
    let mut med = MeshBuilder::new();
    med.verified_medallion(1.66, -0.78, 0.50, 0.35);

    let mut parts = vec![GpuPart::upload(gl, &dock, true)?];
    parts.extend(cards);
    parts.push(GpuPart::upload(gl, &med, true)?);
    Ok(SceneMeshes {
        shadow: GpuPart::upload(gl, &sh, false)?,
        parts,
        host: FusionHost {
            part_idx: 0,
            local_anchor: [0.0, -0.48, 0.398],
            world_w: 1.24,
            eye_w: 0.36,
            symbols: GpuPart::upload(gl, &sym, false)?,
            eyes: make_eyes(gl, 0.36, PAPER)?,
        },
    })
}

fn build_clay_vault(gl: &GL) -> Result<SceneMeshes> {
    let mut sh = MeshBuilder::new();
    sh.ground_shadow(-1.06, 2.05, 0.95);

    // Part 0: Center Beveled Porcelain Gateway Arch + Recessed Keyway Slot
    let mut gate = MeshBuilder::new();
    gate.bevel_box(
        0.0, 0.02, 0.0, 0.82, 1.72, 0.42, 0.20, 0.045, PORCELAIN, MAT_ENAMEL,
    );
    gate.bevel_box(
        0.0, -0.06, 0.18, 0.26, 0.82, 0.10, 0.08, 0.015, OBSIDIAN, MAT_ENAMEL,
    );
    gate.bevel_disc(0.0, 0.56, 0.20, 0.13, 0.08, 0.02, VIOLET, MAT_ENAMEL);

    // Part 1: Left Sculpted 3D Master Key (glides smoothly toward gateway)
    let mut key = MeshBuilder::new();
    key.hollow_frame(
        -1.42, 0.02, 0.04, 0.72, 0.72, 0.36, 0.16, 0.18, GRAPHITE, MAT_ALLOY,
    );
    key.bevel_box(
        -0.78, 0.02, 0.04, 0.72, 0.17, 0.15, 0.05, 0.02, GRAPHITE, MAT_ALLOY,
    );
    key.bevel_box(
        -0.62, -0.14, 0.04, 0.13, 0.18, 0.14, 0.03, 0.015, AMBER, MAT_ENAMEL,
    );
    key.bevel_box(
        -0.44, -0.11, 0.04, 0.12, 0.13, 0.14, 0.03, 0.015, AMBER, MAT_ENAMEL,
    );

    // Part 2: Right Emerging SVID Credential Ticket + Verified Seal (Bot fusion host!)
    let mut ticket = MeshBuilder::new();
    ticket.bevel_box(
        1.18, 0.02, 0.06, 1.12, 1.38, 0.18, 0.16, 0.035, PAPER, MAT_ENAMEL,
    );
    ticket.verified_medallion(1.42, -0.36, 0.18, 0.26);

    let mut sym = MeshBuilder::new();
    sym.decal(-0.12, 0.20, 0.0, 0.56, 0.13, 0.065, SAPPHIRE, MAT_DISPLAY);
    sym.decal(0.0, -0.06, 0.0, 0.78, 0.06, 0.03, UI_INK, MAT_DISPLAY);
    sym.decal(-0.08, -0.24, 0.0, 0.62, 0.05, 0.025, UI_LINE, MAT_DISPLAY);

    Ok(SceneMeshes {
        shadow: GpuPart::upload(gl, &sh, false)?,
        parts: vec![
            GpuPart::upload(gl, &gate, true)?,
            GpuPart::upload(gl, &key, true)?,
            GpuPart::upload(gl, &ticket, true)?,
        ],
        host: FusionHost {
            part_idx: 2,
            local_anchor: [1.14, 0.24, 0.158],
            world_w: 1.06,
            eye_w: 0.42,
            symbols: GpuPart::upload(gl, &sym, false)?,
            eyes: make_eyes(gl, 0.42, OBSIDIAN)?,
        },
    })
}

fn build_clay_shield(gl: &GL) -> Result<SceneMeshes> {
    let mut sh = MeshBuilder::new();
    sh.ground_shadow(-1.08, 1.55, 0.90);

    // Part 0: Machined Porcelain & Deep-Cobalt Padlock Body (Bot fusion host!)
    let mut body = MeshBuilder::new();
    body.bevel_box(
        0.0, -0.24, 0.0, 1.66, 1.32, 0.44, 0.24, 0.05, PORCELAIN, MAT_ENAMEL,
    );
    body.bevel_box(
        0.0, -0.24, 0.18, 1.28, 0.96, 0.12, 0.16, 0.025, NAVY_PLATE, MAT_ENAMEL,
    );
    for &(bx, by) in &[(-0.62, 0.22), (0.62, 0.22), (-0.62, -0.70), (0.62, -0.70)] {
        body.bevel_disc(bx, by, 0.22, 0.05, 0.05, 0.015, SILVER, MAT_ALLOY);
    }

    let mut sym = MeshBuilder::new();
    sym.verified_medallion(0.0, 0.0, 0.0, 0.32);

    // Part 1: Heavy Graphite U-Shackle Arch (clamps smoothly into lock body)
    let mut shackle = MeshBuilder::new();
    shackle.u_shackle(0.0, 0.46, -0.02, 0.46, 0.34, 0.115, GRAPHITE, MAT_ALLOY);

    Ok(SceneMeshes {
        shadow: GpuPart::upload(gl, &sh, false)?,
        parts: vec![
            GpuPart::upload(gl, &body, true)?,
            GpuPart::upload(gl, &shackle, true)?,
        ],
        host: FusionHost {
            part_idx: 0,
            local_anchor: [0.0, -0.24, 0.256],
            world_w: 1.24,
            eye_w: 0.44,
            symbols: GpuPart::upload(gl, &sym, false)?,
            eyes: make_eyes(gl, 0.44, PAPER)?,
        },
    })
}

fn build_clay_release(gl: &GL) -> Result<SceneMeshes> {
    let mut sh = MeshBuilder::new();
    sh.ground_shadow(-1.06, 1.95, 0.95);

    // Part 0: Precision 3-Slot Release Console Deck + Fader Track
    let mut deck = MeshBuilder::new();
    deck.bevel_box(
        0.0, -0.04, 0.0, 2.58, 1.52, 0.28, 0.22, 0.045, PORCELAIN, MAT_ENAMEL,
    );
    deck.bevel_box(
        -0.78, 0.22, 0.14, 0.58, 0.62, 0.12, 0.10, 0.02, SLATE, MAT_ENAMEL,
    );
    deck.decal(-0.78, 0.34, 0.205, 0.34, 0.06, 0.03, PAPER, MAT_DISPLAY);
    deck.bevel_box(
        0.78, 0.22, 0.14, 0.58, 0.62, 0.12, 0.10, 0.02, EMERALD, MAT_ENAMEL,
    );
    deck.decal(0.78, 0.34, 0.205, 0.34, 0.06, 0.03, PAPER, MAT_DISPLAY);
    // Recessed horizontal weight fader track + ticks
    deck.bevel_box(
        0.0, -0.44, 0.12, 1.96, 0.20, 0.08, 0.10, 0.015, OBSIDIAN, MAT_ENAMEL,
    );
    for t in 0..5 {
        let tx = -0.76 + (t as f32) * 0.38;
        deck.decal(tx, -0.64, 0.145, 0.03, 0.09, 0.015, UI_LINE, MAT_DISPLAY);
    }

    // Part 1: Center Active `v2` Sapphire Release Cartridge (elevated — Bot fusion host!)
    let mut active = MeshBuilder::new();
    active.bevel_box(
        0.0, 0.28, 0.20, 0.72, 0.76, 0.18, 0.13, 0.028, SAPPHIRE, MAT_ENAMEL,
    );

    let mut sym = MeshBuilder::new();
    sym.decal(0.0, 0.12, 0.0, 0.40, 0.07, 0.035, PAPER, MAT_DISPLAY);
    sym.decal(0.0, -0.08, 0.0, 0.28, 0.05, 0.025, SKY_COBALT, MAT_DISPLAY);

    // Part 2: Sliding 3D Cobalt/Silver Weight Fader Knob
    let mut knob = MeshBuilder::new();
    knob.bevel_box(
        0.0, -0.44, 0.20, 0.40, 0.32, 0.18, 0.10, 0.03, SILVER, MAT_ALLOY,
    );
    knob.decal(0.0, -0.44, 0.295, 0.08, 0.16, 0.04, SAPPHIRE, MAT_DISPLAY);

    Ok(SceneMeshes {
        shadow: GpuPart::upload(gl, &sh, false)?,
        parts: vec![
            GpuPart::upload(gl, &deck, true)?,
            GpuPart::upload(gl, &active, true)?,
            GpuPart::upload(gl, &knob, true)?,
        ],
        host: FusionHost {
            part_idx: 1,
            local_anchor: [0.0, 0.28, 0.298],
            world_w: 0.76,
            eye_w: 0.36,
            symbols: GpuPart::upload(gl, &sym, false)?,
            eyes: make_eyes(gl, 0.36, PAPER)?,
        },
    })
}

fn build_clay_rust(gl: &GL) -> Result<SceneMeshes> {
    let mut sh = MeshBuilder::new();
    sh.ground_shadow(-1.08, 2.05, 0.95);

    // Part 0: Left Desktop Studio Display + Stand
    let mut desk = MeshBuilder::new();
    desk.with_xform(
        Mat4::translate(-0.66, 0.12, -0.06).mul(Mat4::rot_y(0.10)),
        |m| {
            m.bevel_box(
                0.0, 0.0, 0.0, 1.72, 1.18, 0.16, 0.14, 0.03, PORCELAIN, MAT_ENAMEL,
            );
            m.bevel_box(
                0.0, -0.68, -0.02, 0.28, 0.28, 0.10, 0.04, 0.015, SILVER, MAT_ALLOY,
            );
            m.bevel_box(
                0.0, -0.82, 0.02, 0.74, 0.08, 0.28, 0.04, 0.015, SILVER, MAT_ALLOY,
            );
            m.decal(0.0, 0.0, 0.085, 1.50, 0.96, 0.07, PAPER, MAT_DISPLAY);
            m.decal(-0.54, 0.0, 0.089, 0.34, 0.84, 0.04, UI_MUTED, MAT_DISPLAY);
            m.decal(0.18, 0.16, 0.089, 0.92, 0.34, 0.05, UI_MUTED, MAT_DISPLAY);
            m.decal(0.18, -0.22, 0.091, 0.72, 0.20, 0.06, SAPPHIRE, MAT_DISPLAY);
        },
    );

    // Part 1: Right Mobile/Web Runtime Frame
    let mut web = MeshBuilder::new();
    web.with_xform(
        Mat4::translate(0.96, -0.02, 0.12).mul(Mat4::rot_y(-0.14)),
        |m| {
            m.bevel_box(
                0.0, 0.0, 0.0, 0.92, 1.48, 0.16, 0.16, 0.03, OBSIDIAN, MAT_ENAMEL,
            );
            m.decal(0.0, 0.0, 0.085, 0.76, 1.30, 0.10, PAPER, MAT_DISPLAY);
            m.decal(0.0, 0.52, 0.089, 0.26, 0.06, 0.03, OBSIDIAN, MAT_DISPLAY);
            m.decal(0.0, 0.14, 0.089, 0.60, 0.42, 0.06, UI_MUTED, MAT_DISPLAY);
            m.decal(0.0, -0.28, 0.091, 0.58, 0.18, 0.06, SAPPHIRE, MAT_DISPLAY);
        },
    );

    // Part 2: Foreground Shared-Core Clasp Module bridging Desktop & Web (Bot fusion host!)
    let mut bridge = MeshBuilder::new();
    bridge.bevel_box(
        0.0, 0.0, 0.0, 0.84, 0.62, 0.20, 0.16, 0.035, SAPPHIRE, MAT_ENAMEL,
    );

    let mut sym = MeshBuilder::new();
    sym.hollow_frame(
        0.0, 0.0, 0.0, 0.42, 0.42, 0.12, 0.075, 0.04, PAPER, MAT_ENAMEL,
    );
    sym.bevel_disc(0.0, 0.0, 0.02, 0.095, 0.05, 0.015, AMBER, MAT_ENAMEL);

    Ok(SceneMeshes {
        shadow: GpuPart::upload(gl, &sh, false)?,
        parts: vec![
            GpuPart::upload(gl, &desk, true)?,
            GpuPart::upload(gl, &web, true)?,
            GpuPart::upload(gl, &bridge, true)?,
        ],
        host: FusionHost {
            part_idx: 2,
            local_anchor: [0.0, 0.0, 0.108],
            world_w: 0.84,
            eye_w: 0.36,
            symbols: GpuPart::upload(gl, &sym, false)?,
            eyes: make_eyes(gl, 0.36, PAPER)?,
        },
    })
}

struct GlPipeline {
    gl: GL,
    gl_canvas: HtmlCanvasElement,
    prog: WebGlProgram,
    a_pos: u32,
    a_norm: u32,
    a_color: u32,
    a_mat: u32,
    u_mvp: WebGlUniformLocation,
    u_model: WebGlUniformLocation,
    u_dark: WebGlUniformLocation,
    scenes: [SceneMeshes; 7],
}

fn compile_shader(gl: &GL, ty: u32, src: &str) -> Result<WebGlShader> {
    let sh = gl.create_shader(ty).ok_or("shader")?;
    gl.shader_source(&sh, src);
    gl.compile_shader(&sh);
    if gl
        .get_shader_parameter(&sh, GL::COMPILE_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(sh)
    } else {
        Err(JsValue::from_str(
            &gl.get_shader_info_log(&sh).unwrap_or_default(),
        ))
    }
}

impl GlPipeline {
    fn new(doc: &Document) -> Result<Self> {
        let gl_canvas: HtmlCanvasElement = doc.create_element("canvas")?.dyn_into()?;
        gl_canvas.set_width(1140);
        gl_canvas.set_height(855);

        let attrs = WebGlContextAttributes::new();
        attrs.set_alpha(true);
        attrs.set_antialias(true);
        attrs.set_depth(true);
        attrs.set_premultiplied_alpha(true);

        let ctx = gl_canvas
            .get_context_with_context_options("webgl", &attrs)?
            .or_else(|| gl_canvas.get_context("experimental-webgl").ok().flatten())
            .ok_or("webgl unavailable")?;
        let gl: GL = ctx.dyn_into()?;

        let vs = compile_shader(&gl, GL::VERTEX_SHADER, VERT_SRC)?;
        let fs = compile_shader(&gl, GL::FRAGMENT_SHADER, FRAG_SRC)?;
        let prog = gl.create_program().ok_or("program")?;
        gl.attach_shader(&prog, &vs);
        gl.attach_shader(&prog, &fs);
        gl.link_program(&prog);
        if !gl
            .get_program_parameter(&prog, GL::LINK_STATUS)
            .as_bool()
            .unwrap_or(false)
        {
            return Err(JsValue::from_str(
                &gl.get_program_info_log(&prog).unwrap_or_default(),
            ));
        }

        let a_pos = gl.get_attrib_location(&prog, "a_pos") as u32;
        let a_norm = gl.get_attrib_location(&prog, "a_norm") as u32;
        let a_color = gl.get_attrib_location(&prog, "a_color") as u32;
        let a_mat = gl.get_attrib_location(&prog, "a_mat") as u32;

        let u_mvp = gl.get_uniform_location(&prog, "u_mvp").ok_or("u_mvp")?;
        let u_model = gl.get_uniform_location(&prog, "u_model").ok_or("u_model")?;
        let u_dark = gl.get_uniform_location(&prog, "u_dark").ok_or("u_dark")?;

        let scenes = [
            build_runtime_scene(&gl)?,
            build_ui_scene(&gl)?,
            build_agent_scene(&gl)?,
            build_clay_vault(&gl)?,
            build_clay_shield(&gl)?,
            build_clay_release(&gl)?,
            build_clay_rust(&gl)?,
        ];

        Ok(Self {
            gl,
            gl_canvas,
            prog,
            a_pos,
            a_norm,
            a_color,
            a_mat,
            u_mvp,
            u_model,
            u_dark,
            scenes,
        })
    }

    fn bind_part(&self, part: &GpuPart) {
        let gl = &self.gl;
        gl.bind_buffer(GL::ARRAY_BUFFER, Some(&part.vbo));
        gl.bind_buffer(GL::ELEMENT_ARRAY_BUFFER, Some(&part.ibo));
        let stride = 40;
        gl.enable_vertex_attrib_array(self.a_pos);
        gl.vertex_attrib_pointer_with_i32(self.a_pos, 3, GL::FLOAT, false, stride, 0);
        gl.enable_vertex_attrib_array(self.a_norm);
        gl.vertex_attrib_pointer_with_i32(self.a_norm, 3, GL::FLOAT, false, stride, 12);
        gl.enable_vertex_attrib_array(self.a_color);
        gl.vertex_attrib_pointer_with_i32(self.a_color, 3, GL::FLOAT, false, stride, 24);
        gl.enable_vertex_attrib_array(self.a_mat);
        gl.vertex_attrib_pointer_with_i32(self.a_mat, 1, GL::FLOAT, false, stride, 36);
    }

    fn render_slot(&mut self, slot: &SolidSlot, dark: bool) {
        // 1.5x supersampled WebGL render downsampled via smooth 2D canvas filter
        let w = ((slot.canvas.width() as i32 * 3) / 2).clamp(480, 1710);
        let h = ((slot.canvas.height() as i32 * 3) / 2).clamp(330, 1290);
        if self.gl_canvas.width() as i32 != w || self.gl_canvas.height() as i32 != h {
            self.gl_canvas.set_width(w as u32);
            self.gl_canvas.set_height(h as u32);
        }

        let gl = &self.gl;
        gl.viewport(0, 0, w, h);
        gl.clear_color(0.0, 0.0, 0.0, 0.0);
        gl.clear(GL::COLOR_BUFFER_BIT | GL::DEPTH_BUFFER_BIT);
        gl.use_program(Some(&self.prog));

        gl.uniform1f(Some(&self.u_dark), if dark { 1.0 } else { 0.0 });

        let (base_yaw, base_pitch, half_w) = slot.kind.base_angles();
        let aspect = (w as f32 / h as f32).max(0.5);
        let half_h = half_w / aspect;
        let proj = Mat4::ortho(-half_w, half_w, -half_h, half_h, -6.5, 6.5);
        gl.uniform_matrix4fv_with_f32_array(Some(&self.u_mvp), false, &proj.0);

        let ex = (1.0 - slot.appear as f32).clamp(0.0, 1.0).powi(3);
        let yaw = base_yaw + slot.yaw as f32 + ex * 0.32;
        let pitch = base_pitch + slot.pitch as f32 + ex * 0.16;
        let root_m = Mat4::rot_x(pitch)
            .mul(Mat4::rot_y(yaw))
            .mul(Mat4::scale(1.0 - ex * 0.08));

        let t = slot.time as f32;
        let hov = slot.hover_amt as f32;
        let scene_idx = slot.kind as usize;

        // Compute per-part articulated local transforms
        let part_count = self.scenes[scene_idx].parts.len();
        let mut models = Vec::with_capacity(part_count);
        for i in 0..part_count {
            let local = match (slot.kind, i) {
                (SceneKind::Runtime, 1) => {
                    let slide = (0.38 - hov * 0.34 - (t * 1.4).sin().abs() * 0.06) + ex * 0.55;
                    Mat4::translate(-slide, 0.0, 0.0)
                }
                (SceneKind::Runtime, 2) => {
                    let bob = (t * 1.8).sin() * 0.06 + hov * 0.08;
                    Mat4::translate(0.0, bob, 0.0).mul(Mat4::rot_z(hov * 0.08))
                }
                (SceneKind::Ui, 1) => {
                    let bob = (t * 1.6).sin() * 0.05 + hov * 0.09;
                    Mat4::translate(1.52 + ex * 0.35, -0.14 + bob, 0.56 + hov * 0.08)
                        .mul(Mat4::rot_z(-0.08))
                }
                (SceneKind::Ui, 2) => {
                    let bob = (t * 1.8 + 1.0).cos() * 0.05;
                    let sway = (t * 0.85).sin() * 0.24 + slot.spin as f32;
                    Mat4::translate(1.54, 1.38 + bob, 0.78).mul(Mat4::rot_y(sway))
                }
                (SceneKind::Agent, 1..=3) => {
                    let lift = [0.22_f32, 0.34, 0.25][i - 1] * hov
                        + (t * 1.5 + i as f32 * 0.8).sin() * 0.025
                        - ex * 0.35;
                    Mat4::translate(0.0, lift, 0.0)
                }
                (SceneKind::Agent, 4) => Mat4::translate(0.0, (t * 1.7).sin() * 0.05, 0.0),
                (SceneKind::ClayVault, 1) => {
                    let dx = ((t * 1.3).sin() * 0.5 + 0.5) * 0.22 + hov * 0.16 - ex * 0.35;
                    Mat4::translate(dx, 0.0, 0.0)
                }
                (SceneKind::ClayVault, 2) => {
                    let dx = ((t * 1.3).sin() * 0.5 + 0.5) * 0.14 + hov * 0.14 + ex * 0.30;
                    Mat4::translate(dx, (t * 1.6).cos() * 0.03, 0.0)
                }
                (SceneKind::ClayShield, 1) => {
                    let open = (1.0 - hov) * ((t * 1.2).sin().max(0.0) * 0.12) + ex * 0.32;
                    Mat4::translate(0.0, open, 0.0)
                }
                (SceneKind::ClayRelease, 1) => {
                    Mat4::translate(0.0, 0.09 * hov + (t * 1.5).sin() * 0.025, 0.04 * hov)
                }
                (SceneKind::ClayRelease, 2) => {
                    let slide = (t * 1.15).sin() * 0.52 + hov * 0.24;
                    Mat4::translate(slide, 0.0, 0.0)
                }
                (SceneKind::ClayRust, 2) => {
                    let bob = (t * 1.7).sin() * 0.04;
                    let tilt =
                        ((t * 0.75).sin() * 0.12 + slot.spin as f32 * 0.35) * (1.0 - hov * 0.72);
                    Mat4::translate(0.18, -0.06 + bob, 0.42 + hov * 0.06).mul(Mat4::rot_z(tilt))
                }
                _ => Mat4::identity(),
            };
            models.push(root_m.mul(local));
        }

        // Apply damped 3D elastic impact pop onto the fusion host part when the Bot dives into it!
        let host_part_idx = self.scenes[scene_idx].host.part_idx;
        let anchor = self.scenes[scene_idx].host.local_anchor;
        if slot.fusion_pulse.abs() > 0.001 && host_part_idx < models.len() {
            let p_scale = 1.0 + (slot.fusion_pulse as f32) * 0.12;
            let [ax, ay, az] = anchor;
            let pivot_pop = Mat4::translate(ax, ay, az)
                .mul(Mat4::scale(p_scale))
                .mul(Mat4::translate(-ax, -ay, -az));
            models[host_part_idx] = models[host_part_idx].mul(pivot_pop);
        }

        // Project 3D fusion anchor onto live viewport CSS coordinates for `#kense-guide`!
        if host_part_idx < models.len() {
            let world_anchor = models[host_part_idx].transform_pt(anchor);
            let ndc = proj.transform_pt(world_anchor);
            let rect = slot.canvas.get_bounding_client_rect();
            if rect.width() > 20.0 && rect.height() > 20.0 {
                let sx = rect.left() + (f64::from(ndc[0]) * 0.5 + 0.5) * rect.width();
                let sy = rect.top() + (0.5 - f64::from(ndc[1]) * 0.5) * rect.height();
                let sw =
                    f64::from(self.scenes[scene_idx].host.world_w / half_w) * 0.5 * rect.width();
                fusion::update_projection(
                    scene_idx,
                    HostProjection {
                        ready: true,
                        x: sx,
                        y: sy,
                        width: sw.clamp(28.0, 140.0),
                    },
                );
            }
        }

        // Pass 0: Soft Studio Contact Shadow (premultiplied alpha)
        gl.enable(GL::BLEND);
        gl.blend_func(GL::ONE, GL::ONE_MINUS_SRC_ALPHA);
        gl.disable(GL::DEPTH_TEST);
        gl.depth_mask(false);
        gl.disable(GL::CULL_FACE);
        let shadow_m = Mat4::identity();
        gl.uniform_matrix4fv_with_f32_array(Some(&self.u_model), false, &shadow_m.0);
        self.bind_part(&self.scenes[scene_idx].shadow);
        gl.draw_elements_with_i32(
            GL::TRIANGLES,
            self.scenes[scene_idx].shadow.count,
            GL::UNSIGNED_SHORT,
            0,
        );

        // Pass 1: Solid Studio Softbox + Alloy / Enamel / Display / Glass Shading (writes crisp Z-buffer)
        gl.enable(GL::DEPTH_TEST);
        gl.depth_func(GL::LEQUAL);
        gl.depth_mask(true);
        gl.enable(GL::CULL_FACE);
        gl.cull_face(GL::BACK);
        for (part, m) in self.scenes[scene_idx].parts.iter().zip(models.iter()) {
            gl.uniform_matrix4fv_with_f32_array(Some(&self.u_model), false, &m.0);
            self.bind_part(part);
            gl.draw_elements_with_i32(GL::TRIANGLES, part.count, GL::UNSIGNED_SHORT, 0);
        }

        // Pass 1B: Fusion Host Symbols (retract on Bot dive) & Extruded 3D KUI Eyes (appear on Bot attach!)
        if host_part_idx < models.len() {
            let base_m = models[host_part_idx];
            let [ax, ay, az] = anchor;
            let cov = (slot.fusion_cover as f32).clamp(0.0, 1.0);
            let cov_s = cov * cov * (3.0 - 2.0 * cov);
            if cov_s < 0.992 {
                let sym_scale = (1.0 - 0.92 * cov_s).max(0.01);
                let sym_m = base_m
                    .mul(Mat4::translate(ax, ay, az - 0.03 * cov_s))
                    .mul(Mat4::scale(sym_scale));
                gl.uniform_matrix4fv_with_f32_array(Some(&self.u_model), false, &sym_m.0);
                self.bind_part(&self.scenes[scene_idx].host.symbols);
                gl.draw_elements_with_i32(
                    GL::TRIANGLES,
                    self.scenes[scene_idx].host.symbols.count,
                    GL::UNSIGNED_SHORT,
                    0,
                );
            }

            let eye_a = (slot.fusion_eye as f32).clamp(0.0, 1.0);
            if eye_a > 0.01 {
                let fsnap = fusion::snapshot();
                let eye_s = eye_a * eye_a * (3.0 - 2.0 * eye_a);
                let eye_w = self.scenes[scene_idx].host.eye_w;
                let gx = fsnap.look_x * eye_w * 0.22 * eye_s;
                let gy = -fsnap.look_y * eye_w * 0.14 * eye_s;
                let sx = eye_s * (1.0 + (slot.fusion_pulse as f32) * 0.15);
                let sy = eye_s * fsnap.blink * (1.0 - (slot.fusion_pulse as f32) * 0.12);
                let eye_m = base_m
                    .mul(Mat4::translate(ax + gx, ay + gy, az + 0.014))
                    .mul(Mat4::scale_xyz(sx.max(0.02), sy.max(0.02), eye_s.max(0.02)));
                gl.uniform_matrix4fv_with_f32_array(Some(&self.u_model), false, &eye_m.0);
                self.bind_part(&self.scenes[scene_idx].host.eyes);
                gl.draw_elements_with_i32(
                    GL::TRIANGLES,
                    self.scenes[scene_idx].host.eyes.count,
                    GL::UNSIGNED_SHORT,
                    0,
                );
            }
        }

        // Blit supersampled WebGL render onto the visible 2D `<canvas>` target
        let cw = f64::from(slot.canvas.width());
        let ch = f64::from(slot.canvas.height());
        slot.ctx.set_image_smoothing_enabled(true);
        slot.ctx.clear_rect(0.0, 0.0, cw, ch);
        let _ = slot.ctx.draw_image_with_html_canvas_element_and_dw_and_dh(
            &self.gl_canvas,
            0.0,
            0.0,
            cw,
            ch,
        );
    }
}

struct SolidSlot {
    host: HtmlElement,
    card: Option<HtmlElement>,
    canvas: HtmlCanvasElement,
    ctx: CanvasRenderingContext2d,
    kind: SceneKind,
    visible: bool,
    hovered: bool,
    hover_amt: f64,
    entered: bool,
    appear: f64,
    yaw: f64,
    pitch: f64,
    target_yaw: f64,
    target_pitch: f64,
    spin: f64,
    spin_vel: f64,
    fusion_cover: f64,
    fusion_eye: f64,
    fusion_pulse: f64,
    fusion_pulse_vel: f64,
    seen_impact_seq: u32,
    time: f64,
    last_ptr: Option<(f64, f64, f64)>,
    dirty: bool,
}

struct StudioEngine {
    document: Document,
    reduced: MediaQueryList,
    pipeline: GlPipeline,
    slots: Vec<SolidSlot>,
    last_ms: f64,
    last_dark: bool,
}

impl StudioEngine {
    fn is_dark(&self) -> bool {
        self.document
            .document_element()
            .and_then(|e| e.get_attribute("data-theme"))
            .as_deref()
            == Some("dark")
    }

    fn tick(&mut self, now: f64) -> bool {
        if self.document.hidden() {
            self.last_ms = 0.0;
            return false;
        }
        let dt = if self.last_ms == 0.0 {
            0.016
        } else {
            ((now - self.last_ms) / 1000.0).clamp(0.001, 0.05)
        };
        self.last_ms = now;

        let dark = self.is_dark();
        if dark != self.last_dark {
            self.last_dark = dark;
            for s in &mut self.slots {
                s.dirty = true;
            }
        }

        let fsnap = fusion::snapshot();
        let reduced = self.reduced.matches();
        let mut keep_running = false;

        for slot in &mut self.slots {
            if !slot.visible && slot.entered {
                continue;
            }

            if slot.visible && !slot.entered {
                slot.entered = true;
                if let Some(parent) = slot.host.parent_element() {
                    let _ = parent.set_attribute("data-solid-entrance", "entering");
                }
            }

            let scene_idx = slot.kind as usize;
            let is_active_fusion = fsnap.active_scene == Some(scene_idx);
            let target_cover = if is_active_fusion {
                match fsnap.phase {
                    Phase::Idle => 0.0,
                    Phase::Diving(p) => (p * 1.15).clamp(0.25, 1.0),
                    Phase::Attached => 1.0,
                }
            } else {
                0.0
            };
            let target_eye = if is_active_fusion && fsnap.phase == Phase::Attached {
                1.0
            } else {
                0.0
            };

            if is_active_fusion
                && fsnap.phase == Phase::Attached
                && slot.seen_impact_seq != fsnap.impact_seq
            {
                slot.seen_impact_seq = fsnap.impact_seq;
                if !reduced {
                    slot.fusion_pulse_vel = 3.2;
                }
            }

            if reduced {
                slot.appear = 1.0;
                slot.yaw = 0.0;
                slot.pitch = 0.0;
                slot.hover_amt = if is_active_fusion { 1.0 } else { 0.0 };
                slot.spin_vel = 0.0;
                if (slot.fusion_cover - target_cover).abs() > 0.001
                    || (slot.fusion_eye - target_eye).abs() > 0.001
                {
                    slot.fusion_cover = target_cover;
                    slot.fusion_eye = target_eye;
                    slot.dirty = true;
                }
                slot.fusion_pulse = 0.0;
                slot.fusion_pulse_vel = 0.0;
            } else {
                if slot.entered && slot.appear < 1.0 {
                    slot.appear = (slot.appear + dt * 1.45).min(1.0);
                    slot.dirty = true;
                }
                slot.time += dt;
                let h_goal = if slot.hovered || is_active_fusion {
                    1.0
                } else {
                    0.0
                };
                let dh = (h_goal - slot.hover_amt) * (1.0 - (-9.0 * dt).exp());
                if dh.abs() > 0.0005 {
                    slot.hover_amt += dh;
                    slot.dirty = true;
                } else {
                    slot.hover_amt = h_goal;
                }

                let cov_rate = if target_cover > slot.fusion_cover {
                    14.0
                } else {
                    9.5
                };
                let dcov = (target_cover - slot.fusion_cover) * (1.0 - (-cov_rate * dt).exp());
                if dcov.abs() > 0.001 {
                    slot.fusion_cover += dcov;
                    slot.dirty = true;
                } else {
                    slot.fusion_cover = target_cover;
                }

                let eye_rate = if target_eye > slot.fusion_eye {
                    16.0
                } else {
                    14.0
                };
                let deye = (target_eye - slot.fusion_eye) * (1.0 - (-eye_rate * dt).exp());
                if deye.abs() > 0.001 {
                    slot.fusion_eye += deye;
                    slot.dirty = true;
                } else {
                    slot.fusion_eye = target_eye;
                }

                let p_acc = -460.0 * slot.fusion_pulse - 24.0 * slot.fusion_pulse_vel;
                slot.fusion_pulse_vel += p_acc * dt;
                slot.fusion_pulse =
                    (slot.fusion_pulse + slot.fusion_pulse_vel * dt).clamp(-0.18, 0.22);
                if slot.fusion_pulse.abs() < 0.001 && slot.fusion_pulse_vel.abs() < 0.015 {
                    slot.fusion_pulse = 0.0;
                    slot.fusion_pulse_vel = 0.0;
                } else {
                    slot.dirty = true;
                }

                let blend = 1.0 - (-10.0 * dt).exp();
                let dy = (slot.target_yaw - slot.yaw) * blend;
                let dp = (slot.target_pitch - slot.pitch) * blend;
                slot.yaw += dy;
                slot.pitch += dp;

                if slot.spin_vel.abs() > 0.001 {
                    slot.spin += slot.spin_vel * dt;
                    slot.spin_vel *= (-4.5 * dt).exp();
                    slot.dirty = true;
                } else {
                    slot.spin_vel = 0.0;
                }
            }

            let animating = !reduced && slot.visible;
            if slot.dirty || animating {
                self.pipeline.render_slot(slot, dark);
                flag(&slot.host, "data-solid-ready", true);
                slot.dirty = false;
            }
            if animating || slot.appear < 1.0 {
                keep_running = true;
            }
        }

        if !keep_running {
            self.last_ms = 0.0;
        }
        keep_running
    }
}

pub(super) fn init(
    window: &Window,
    document: &Document,
    root: &Element,
    reduced: &MediaQueryList,
) -> Result {
    let nodes = root.query_selector_all("[data-kense-solid]")?;
    if nodes.length() == 0 {
        return Ok(());
    }
    let Ok(pipeline) = GlPipeline::new(document) else {
        return Ok(());
    };

    let dpr = window.device_pixel_ratio().clamp(1.0, 2.0);
    let mut slots = Vec::new();
    for i in 0..nodes.length() {
        let Some(host) = nodes.item(i).and_then(|n| n.dyn_into::<HtmlElement>().ok()) else {
            continue;
        };
        let Ok(canvas) = element::<HtmlCanvasElement>(&host, "canvas") else {
            continue;
        };
        let Some(ctx) = canvas
            .get_context("2d")
            .ok()
            .flatten()
            .and_then(|c| c.dyn_into::<CanvasRenderingContext2d>().ok())
        else {
            continue;
        };
        let kind_str = host.get_attribute("data-kense-solid").unwrap_or_default();
        let is_clay = kind_str.starts_with("clay-");
        let kind = SceneKind::parse(&kind_str);
        let card = if is_clay {
            host.closest(".kense-bento-card")
                .ok()
                .flatten()
                .and_then(|e| e.dyn_into::<HtmlElement>().ok())
        } else {
            None
        };

        let rect = host.get_bounding_client_rect();
        if rect.width() > 40.0 && rect.height() > 40.0 {
            canvas.set_width((rect.width() * dpr).round() as u32);
            canvas.set_height((rect.height() * dpr).round() as u32);
        }

        slots.push(SolidSlot {
            host,
            card,
            canvas,
            ctx,
            kind,
            visible: true,
            hovered: false,
            hover_amt: 0.0,
            entered: reduced.matches(),
            appear: if reduced.matches() { 1.0 } else { 0.0 },
            yaw: 0.0,
            pitch: 0.0,
            target_yaw: 0.0,
            target_pitch: 0.0,
            spin: 0.0,
            spin_vel: 0.0,
            fusion_cover: 0.0,
            fusion_eye: 0.0,
            fusion_pulse: 0.0,
            fusion_pulse_vel: 0.0,
            seen_impact_seq: 0,
            time: (i as f64) * 0.9,
            last_ptr: None,
            dirty: true,
        });
    }

    if slots.is_empty() {
        return Ok(());
    }

    let dark_now = document
        .document_element()
        .and_then(|e| e.get_attribute("data-theme"))
        .as_deref()
        == Some("dark");

    let engine = Rc::new(RefCell::new(StudioEngine {
        document: document.clone(),
        reduced: reduced.clone(),
        pipeline,
        slots,
        last_ms: 0.0,
        last_dark: dark_now,
    }));

    // Paint initial frame synchronously so every visible sculpture is crisp on Frame 1
    if let Ok(mut eng) = engine.try_borrow_mut() {
        let dark = eng.is_dark();
        let StudioEngine {
            pipeline, slots, ..
        } = &mut *eng;
        for slot in slots.iter_mut() {
            let saved = slot.appear;
            slot.appear = 1.0;
            pipeline.render_slot(slot, dark);
            flag(&slot.host, "data-solid-ready", true);
            slot.appear = saved;
        }
    }

    let frame = Frame::new(window, {
        let engine = engine.clone();
        move |now| engine.borrow_mut().tick(now)
    });

    {
        let engine_w = engine.clone();
        let frame_w = frame.clone();
        fusion::register_waker(move || {
            if let Ok(mut eng) = engine_w.try_borrow_mut() {
                for s in &mut eng.slots {
                    if s.visible {
                        s.dirty = true;
                    }
                }
            }
            frame_w.request();
        });
    }

    {
        let engine_sc = engine.clone();
        let frame_sc = frame.clone();
        listen(window, "scroll", move |_| {
            if let Ok(mut eng) = engine_sc.try_borrow_mut() {
                for s in &mut eng.slots {
                    if s.visible {
                        s.dirty = true;
                    }
                }
            }
            frame_sc.request();
        })?;
    }

    // Pointer tilt, hover cascade, & 3D card tilt for Bento cards
    {
        let engine = engine.clone();
        let frame = frame.clone();
        listen(document, "pointermove", move |ev| {
            let Some(pe) = ev.dyn_ref::<PointerEvent>() else {
                return;
            };
            if pe.pointer_type() == "touch" {
                return;
            }
            let px = f64::from(pe.client_x());
            let py = f64::from(pe.client_y());
            let now = pe.time_stamp();
            let Ok(mut eng) = engine.try_borrow_mut() else {
                return;
            };
            if eng.reduced.matches() {
                return;
            }
            let mut woke = false;
            for slot in &mut eng.slots {
                let target_el = slot.card.as_ref().unwrap_or(&slot.host);
                let r = target_el.get_bounding_client_rect();
                let inside = px >= r.left() && px <= r.right() && py >= r.top() && py <= r.bottom();
                if inside {
                    let nx = ((px - r.left()) / r.width().max(1.0) * 2.0 - 1.0).clamp(-1.0, 1.0);
                    let ny = ((py - r.top()) / r.height().max(1.0) * 2.0 - 1.0).clamp(-1.0, 1.0);
                    slot.hovered = true;
                    slot.target_yaw = nx * 0.24;
                    slot.target_pitch = ny * 0.16;

                    if let Some((lx, ly, lt)) = slot.last_ptr {
                        let dt_s = ((now - lt) / 1000.0).clamp(0.008, 0.16);
                        let vx = (px - lx) / dt_s;
                        let vy = (py - ly) / dt_s;
                        if vx.hypot(vy) > 420.0 {
                            slot.spin_vel = (slot.spin_vel + vx * 0.0018).clamp(-3.2, 3.2);
                        }
                    }
                    slot.last_ptr = Some((px, py, now));

                    if let Some(card) = &slot.card {
                        css(card, "--tilt-rx", &format!("{:.2}deg", -ny * 3.8));
                        css(card, "--tilt-ry", &format!("{:.2}deg", nx * 4.2));
                    }
                    slot.dirty = true;
                    woke = true;
                } else if slot.hovered || slot.target_yaw != 0.0 || slot.target_pitch != 0.0 {
                    slot.hovered = false;
                    slot.target_yaw = 0.0;
                    slot.target_pitch = 0.0;
                    slot.last_ptr = None;
                    if let Some(card) = &slot.card {
                        css(card, "--tilt-rx", "0deg");
                        css(card, "--tilt-ry", "0deg");
                    }
                    slot.dirty = true;
                    woke = true;
                }
            }
            if woke {
                frame.request();
            }
        })?;
    }

    // IntersectionObserver so offscreen canvases sleep cleanly (0% idle GPU!)
    {
        let engine_io = engine.clone();
        let frame_io = frame.clone();
        let io_cb = Closure::<dyn FnMut(Array)>::new(move |entries: Array| {
            let Ok(mut eng) = engine_io.try_borrow_mut() else {
                return;
            };
            let mut any_visible = false;
            for entry in entries.iter() {
                let Ok(entry) = entry.dyn_into::<IntersectionObserverEntry>() else {
                    continue;
                };
                let target = entry.target();
                let vis = entry.is_intersecting();
                for slot in &mut eng.slots {
                    if slot.host.as_ref() as &Element == &target {
                        slot.visible = vis;
                        if vis {
                            slot.dirty = true;
                            any_visible = true;
                        }
                    }
                }
            }
            if any_visible {
                frame_io.request();
            }
        });
        if let Ok(io) = IntersectionObserver::new(io_cb.as_ref().unchecked_ref()) {
            if let Ok(eng) = engine.try_borrow() {
                for slot in &eng.slots {
                    io.observe(&slot.host);
                }
            }
            let _ = Reflect::set(window, &JsValue::from_str("__kenseSolidsIO"), &io);
            io_cb.forget();
        }
    }

    // ResizeObserver for crisp HiDPI resizing
    {
        let engine_ro = engine.clone();
        let frame_ro = frame.clone();
        let win_ro = window.clone();
        let ro_cb = Closure::<dyn FnMut(Array)>::new(move |_entries: Array| {
            let Ok(mut eng) = engine_ro.try_borrow_mut() else {
                return;
            };
            let dpr = win_ro.device_pixel_ratio().clamp(1.0, 2.0);
            for slot in &mut eng.slots {
                let r = slot.host.get_bounding_client_rect();
                if r.width() > 40.0 && r.height() > 40.0 {
                    let nw = (r.width() * dpr).round() as u32;
                    let nh = (r.height() * dpr).round() as u32;
                    if slot.canvas.width() != nw || slot.canvas.height() != nh {
                        slot.canvas.set_width(nw);
                        slot.canvas.set_height(nh);
                        slot.dirty = true;
                    }
                }
            }
            frame_ro.request();
        });
        if let Ok(ro) = ResizeObserver::new(ro_cb.as_ref().unchecked_ref()) {
            if let Ok(eng) = engine.try_borrow() {
                for slot in &eng.slots {
                    ro.observe(&slot.host);
                }
            }
            let _ = Reflect::set(window, &JsValue::from_str("__kenseSolidsRO"), &ro);
            ro_cb.forget();
        }
    }

    frame.request();
    std::mem::forget(frame);
    Ok(())
}
