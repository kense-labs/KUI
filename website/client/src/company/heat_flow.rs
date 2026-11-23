//! WebGL organic FBM silk-caustic Heat-Flow shader for primary Hero capsule (`[data-heat-flow]`).

use crate::cover::{Frame, Result, flag, listen};
use js_sys::{Array, Float32Array};
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::{JsCast, closure::Closure};
use web_sys::{
    Document, Element, HtmlCanvasElement, HtmlElement, IntersectionObserver,
    IntersectionObserverEntry, MediaQueryList, PointerEvent, WebGlContextAttributes, WebGlProgram,
    WebGlRenderingContext as GL, WebGlShader, WebGlUniformLocation, Window,
};

const VERT_SRC: &str = r#"
attribute vec2 a_pos;
varying vec2 v_uv;
void main() {
    v_uv = a_pos * 0.5 + 0.5;
    gl_Position = vec4(a_pos, 0.0, 1.0);
}
"#;

const FRAG_SRC: &str = r#"
precision mediump float;
varying vec2 v_uv;
uniform vec2 u_res;
uniform vec2 u_pointer;
uniform float u_time;
uniform float u_hover;

float hash(vec2 p) {
    p = fract(p * vec2(123.34, 456.21));
    p += dot(p, p + 45.32);
    return fract(p.x * p.y);
}

float noise(vec2 p) {
    vec2 i = floor(p), f = fract(p);
    f = f * f * (3.0 - 2.0 * f);
    return mix(
        mix(hash(i), hash(i + vec2(1.0, 0.0)), f.x),
        mix(hash(i + vec2(0.0, 1.0)), hash(i + vec2(1.0, 1.0)), f.x),
        f.y
    );
}

float fbm(vec2 p) {
    float v = 0.0, a = 0.52;
    mat2 r = mat2(0.82, -0.57, 0.57, 0.82);
    for (int i = 0; i < 6; i++) {
        v += a * noise(p);
        p = r * p * 1.88 + 8.17;
        a *= 0.51;
    }
    return v;
}

vec3 palette(float h) {
    vec3 c0 = vec3(0.075, 0.078, 0.095);
    vec3 c1 = vec3(0.16, 0.24, 0.44);
    vec3 c2 = vec3(0.46, 0.20, 0.34);
    vec3 c3 = vec3(0.86, 0.48, 0.22);
    vec3 c4 = vec3(0.24, 0.52, 0.92);
    vec3 col = mix(c0, c1, smoothstep(0.08, 0.36, h));
    col = mix(col, c2, smoothstep(0.32, 0.58, h));
    col = mix(col, c3, smoothstep(0.54, 0.78, h));
    return mix(col, c4, smoothstep(0.74, 0.98, h));
}

void main() {
    float aspect = u_res.x / max(u_res.y, 1.0);
    vec2 p = (v_uv - 0.5) * vec2(aspect, 1.0);
    float t = u_time * (0.42 + u_hover * 0.28);

    vec2 w = vec2(
        fbm(p * 0.95 + vec2(t * 0.45, -t * 0.32)),
        fbm(p * 1.15 + vec2(-t * 0.28, t * 0.42) + 4.7)
    );
    vec2 fp = p * 0.72 + w * 1.65 + vec2(0.1 * t, -0.55 * t);
    float caustic = abs(sin(fp.x * 3.8 + w.x * 1.8) + sin(fp.y * 4.4 - w.y * 1.5 + t * 0.4)) * 0.5;
    float field = pow(clamp(caustic, 0.0, 1.0), 1.55) * 0.62 + fbm(fp) * 0.34;

    vec2 ptr = (u_pointer - 0.5) * vec2(aspect, 1.0);
    float ptrHeat = exp(-dot(p - ptr, p - ptr) * 2.2) * (0.22 + u_hover * 0.48);

    // Subtle capsule rim glow + calm readable center
    vec2 q = abs(p) - vec2(aspect * 0.5 - 0.5, 0.0);
    float edgeDist = abs(length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - 0.48);
    float rim = exp(-edgeDist * 8.5) * (0.18 + u_hover * 0.24);
    float centerCalm = smoothstep(0.78, 0.14, length(p * vec2(0.55, 1.0)));

    float heat = clamp(field * (0.48 + u_hover * 0.32) + ptrHeat + rim - centerCalm * 0.18, 0.0, 1.0);
    vec3 col = palette(heat);

    // Subtle organic film grain
    float g = (hash(floor(gl_FragCoord.xy)) - 0.5) * 0.035;
    col = clamp(col + vec3(g), 0.0, 1.0);

    gl_FragColor = vec4(col, 1.0);
}
"#;

fn compile(gl: &GL, ty: u32, src: &str) -> Option<WebGlShader> {
    let sh = gl.create_shader(ty)?;
    gl.shader_source(&sh, src);
    gl.compile_shader(&sh);
    if gl
        .get_shader_parameter(&sh, GL::COMPILE_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Some(sh)
    } else {
        gl.delete_shader(Some(&sh));
        None
    }
}

struct HeatState {
    window: Window,
    document: Document,
    reduced: MediaQueryList,
    target: HtmlElement,
    canvas: HtmlCanvasElement,
    gl: GL,
    program: WebGlProgram,
    attr_pos: u32,
    u_res: Option<WebGlUniformLocation>,
    u_pointer: Option<WebGlUniformLocation>,
    u_time: Option<WebGlUniformLocation>,
    u_hover: Option<WebGlUniformLocation>,
    px: f64,
    py: f64,
    goal_px: f64,
    goal_py: f64,
    hover: f64,
    goal_hover: f64,
    visible: bool,
    start_ms: Option<f64>,
}

impl HeatState {
    fn tick(&mut self, now: f64) -> bool {
        if self.document.hidden()
            || !self.visible
            || self.reduced.matches()
            || self.gl.is_context_lost()
        {
            return false;
        }
        let t0 = *self.start_ms.get_or_insert(now);
        let t = ((now - t0) / 1000.0) as f32;

        self.px += (self.goal_px - self.px) * 0.16;
        self.py += (self.goal_py - self.py) * 0.16;
        self.hover += (self.goal_hover - self.hover) * 0.12;

        let dpr = self.window.device_pixel_ratio().clamp(1.0, 2.0);
        let w = (f64::from(self.target.client_width()) * dpr)
            .round()
            .max(1.0) as u32;
        let h = (f64::from(self.target.client_height()) * dpr)
            .round()
            .max(1.0) as u32;
        if self.canvas.width() != w || self.canvas.height() != h {
            self.canvas.set_width(w);
            self.canvas.set_height(h);
        }

        self.gl.viewport(0, 0, w as i32, h as i32);
        self.gl.use_program(Some(&self.program));
        self.gl.enable_vertex_attrib_array(self.attr_pos);
        self.gl
            .vertex_attrib_pointer_with_i32(self.attr_pos, 2, GL::FLOAT, false, 0, 0);
        self.gl.uniform2f(self.u_res.as_ref(), w as f32, h as f32);
        self.gl.uniform2f(
            self.u_pointer.as_ref(),
            self.px as f32,
            (1.0 - self.py) as f32,
        );
        self.gl.uniform1f(self.u_time.as_ref(), t);
        self.gl.uniform1f(self.u_hover.as_ref(), self.hover as f32);
        self.gl.draw_arrays(GL::TRIANGLES, 0, 6);

        flag(&self.target, "data-heat-ready", true);
        true
    }
}

pub(super) fn init(
    window: &Window,
    document: &Document,
    root: &Element,
    reduced: &MediaQueryList,
) -> Result {
    if reduced.matches() {
        return Ok(());
    }
    let Some(node) = root.query_selector("[data-heat-flow]")? else {
        return Ok(());
    };
    let Ok(target) = node.dyn_into::<HtmlElement>() else {
        return Ok(());
    };
    let Some(canvas_el) = target.query_selector("canvas")? else {
        return Ok(());
    };
    let Ok(canvas) = canvas_el.dyn_into::<HtmlCanvasElement>() else {
        return Ok(());
    };

    let attrs = WebGlContextAttributes::new();
    attrs.set_alpha(false);
    attrs.set_antialias(false);
    attrs.set_depth(false);
    let Ok(Some(ctx)) = canvas.get_context_with_context_options("webgl", &attrs) else {
        return Ok(());
    };
    let Ok(gl) = ctx.dyn_into::<GL>() else {
        return Ok(());
    };
    let (Some(vs), Some(fs)) = (
        compile(&gl, GL::VERTEX_SHADER, VERT_SRC),
        compile(&gl, GL::FRAGMENT_SHADER, FRAG_SRC),
    ) else {
        return Ok(());
    };
    let Some(program) = gl.create_program() else {
        return Ok(());
    };
    gl.attach_shader(&program, &vs);
    gl.attach_shader(&program, &fs);
    gl.link_program(&program);
    gl.delete_shader(Some(&vs));
    gl.delete_shader(Some(&fs));
    if !gl
        .get_program_parameter(&program, GL::LINK_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        return Ok(());
    }

    let Some(buf) = gl.create_buffer() else {
        return Ok(());
    };
    gl.bind_buffer(GL::ARRAY_BUFFER, Some(&buf));
    let quad: [f32; 12] = [
        -1.0, -1.0, 1.0, -1.0, -1.0, 1.0, -1.0, 1.0, 1.0, -1.0, 1.0, 1.0,
    ];
    let arr = Float32Array::from(quad.as_slice());
    gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &arr, GL::STATIC_DRAW);

    let attr_pos = gl.get_attrib_location(&program, "a_pos").max(0) as u32;
    let state = Rc::new(RefCell::new(HeatState {
        window: window.clone(),
        document: document.clone(),
        reduced: reduced.clone(),
        target: target.clone(),
        canvas,
        u_res: gl.get_uniform_location(&program, "u_res"),
        u_pointer: gl.get_uniform_location(&program, "u_pointer"),
        u_time: gl.get_uniform_location(&program, "u_time"),
        u_hover: gl.get_uniform_location(&program, "u_hover"),
        gl,
        program,
        attr_pos,
        px: 0.5,
        py: 0.5,
        goal_px: 0.5,
        goal_py: 0.5,
        hover: 0.0,
        goal_hover: 0.0,
        visible: true,
        start_ms: None,
    }));

    let frame = Frame::new(window, {
        let st = state.clone();
        move |now| st.borrow_mut().tick(now)
    });

    let io_cb = Closure::<dyn FnMut(Array)>::new({
        let st = state.clone();
        let fr = frame.clone();
        move |entries: Array| {
            let mut vis = false;
            if let Ok(mut s) = st.try_borrow_mut() {
                for entry in entries.iter() {
                    let Ok(e) = entry.dyn_into::<IntersectionObserverEntry>() else {
                        continue;
                    };
                    s.visible = e.is_intersecting();
                }
                vis = s.visible;
            }
            if vis {
                fr.request();
            }
        }
    });
    if let Ok(io) = IntersectionObserver::new(io_cb.as_ref().unchecked_ref()) {
        io.observe(&target);
        std::mem::forget(io);
    }
    io_cb.forget();

    listen(&target, "pointermove", {
        let st = state.clone();
        let tgt = target.clone();
        let fr = frame.clone();
        move |ev| {
            let Some(p) = ev.dyn_ref::<PointerEvent>() else {
                return;
            };
            let r = tgt.get_bounding_client_rect();
            if r.width() > 1.0 && r.height() > 1.0 {
                let mut s = st.borrow_mut();
                s.goal_px = ((f64::from(p.client_x()) - r.left()) / r.width()).clamp(0.0, 1.0);
                s.goal_py = ((f64::from(p.client_y()) - r.top()) / r.height()).clamp(0.0, 1.0);
                s.goal_hover = 1.0;
            }
            fr.request();
        }
    })?;

    for ev_name in ["pointerleave", "pointercancel"] {
        listen(&target, ev_name, {
            let st = state.clone();
            move |_| {
                let mut s = st.borrow_mut();
                s.goal_hover = 0.0;
                s.goal_px = 0.5;
                s.goal_py = 0.5;
            }
        })?;
    }

    listen(document, "visibilitychange", {
        let st = state.clone();
        let fr = frame.clone();
        move |_| {
            if !st.borrow().document.hidden() && st.borrow().visible {
                fr.request();
            }
        }
    })?;

    frame.request();
    Ok(())
}
