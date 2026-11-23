//! WebGL Liquid-Meniscus Command Dock (`[data-command-dock]`) at the bottom Hero edge.
//!
//! Uses a cubic smooth-min (`unionSmooth`) signed distance field in WebGL so the Agent command
//! capsule fuses seamlessly into the bottom edge of the viewport at rest, stretches and snaps free
//! with damped harmonic jelly physics on scroll, re-attaches with a soft suction bounce at top,
//! and jiggles when clicked.

use crate::cover::{Frame, Result, css, element, flag, listen, viewport};
use js_sys::Float32Array;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::{JsCast, closure::Closure};
use web_sys::{
    Document, Element, HtmlCanvasElement, HtmlElement, MediaQueryList, MutationObserver,
    MutationObserverInit, ResizeObserver, WebGlContextAttributes, WebGlProgram,
    WebGlRenderingContext as GL, WebGlShader, WebGlUniformLocation, Window,
};

const DETACH_SCROLL_PX: f64 = 21.0;
const REATTACH_DUR: f64 = 0.42;

const VERT_SRC: &str = "attribute vec2 p; void main(){gl_Position=vec4(p,0.,1.);}";

const FRAG_SRC: &str = r#"
precision highp float;
uniform vec2 resolution;
uniform float dpr, width, lift, grow, jelly, compact;
uniform vec3 ink;

float unionSmooth(float a, float b, float k) {
    float h = max(k - abs(a - b), 0.0) / k;
    return min(a, b) - h * h * h * k / 6.0;
}

float surface(vec2 p) {
    float radius = 31.0 - 7.5 * compact;
    float center = 31.0 + (1.0 - grow) * 86.0;
    vec2 q = p - vec2(0.0, center);
    q.x -= jelly * q.y * 1.45;
    q /= vec2(1.0 - jelly, 1.0 + jelly);
    float segment = max(0.0, width * 0.5 - 24.0 - 1.5 * compact - radius);
    float body = length(vec2(max(abs(q.x) - segment, 0.0), q.y)) - radius;
    float edge = 62.0 + lift + 3.0 - p.y;
    return unionSmooth(body, edge, 70.0);
}

void main() {
    vec2 p = vec2(gl_FragCoord.x, resolution.y - gl_FragCoord.y) / dpr;
    p -= vec2(80.0 + width * 0.5, 48.0);
    float field = surface(p);
    float alpha = 1.0 - smoothstep(-0.7 / dpr, 0.7 / dpr, field);

    float normalY = clamp(surface(p + vec2(0.0, 0.5)) - surface(p - vec2(0.0, 0.5)), -1.0, 1.0);
    float rim = exp(-abs(field + 1.0) * 1.3);
    vec3 face = mix(ink, vec3(1.0), rim * max(-normalY, 0.0) * 0.19);
    face *= 1.0 - rim * max(normalY, 0.0) * 0.18;

    float contact = (1.0 - smoothstep(-0.4, 0.8, surface(p - vec2(0.0, 1.0)))) * 0.24;
    float soft = (1.0 - smoothstep(-0.5, 1.8, surface(p - vec2(0.0, 2.0)))) * 0.22;
    float shadow = (contact + soft * (1.0 - contact)) * compact * (1.0 - alpha);

    gl_FragColor = vec4(face * alpha, alpha + shadow);
}
"#;

fn compile_shader(gl: &GL, ty: u32, src: &str) -> Option<WebGlShader> {
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

struct DockGl {
    gl: GL,
    program: WebGlProgram,
    attr_p: u32,
    u_resolution: Option<WebGlUniformLocation>,
    u_dpr: Option<WebGlUniformLocation>,
    u_width: Option<WebGlUniformLocation>,
    u_lift: Option<WebGlUniformLocation>,
    u_grow: Option<WebGlUniformLocation>,
    u_jelly: Option<WebGlUniformLocation>,
    u_compact: Option<WebGlUniformLocation>,
    u_ink: Option<WebGlUniformLocation>,
}

impl DockGl {
    fn new(canvas: &HtmlCanvasElement) -> Option<Self> {
        let attrs = WebGlContextAttributes::new();
        attrs.set_alpha(true);
        attrs.set_premultiplied_alpha(true);
        attrs.set_antialias(false);
        attrs.set_depth(false);
        attrs.set_preserve_drawing_buffer(true);

        let gl = canvas
            .get_context_with_context_options("webgl", &attrs)
            .ok()??
            .dyn_into::<GL>()
            .ok()?;

        let vs = compile_shader(&gl, GL::VERTEX_SHADER, VERT_SRC)?;
        let fs = compile_shader(&gl, GL::FRAGMENT_SHADER, FRAG_SRC)?;
        let program = gl.create_program()?;
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
            gl.delete_program(Some(&program));
            return None;
        }

        let buf = gl.create_buffer()?;
        gl.bind_buffer(GL::ARRAY_BUFFER, Some(&buf));
        let quad: [f32; 12] = [
            -1.0, -1.0, 1.0, -1.0, -1.0, 1.0, -1.0, 1.0, 1.0, -1.0, 1.0, 1.0,
        ];
        let arr = Float32Array::from(quad.as_slice());
        gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &arr, GL::STATIC_DRAW);

        let attr_p = gl.get_attrib_location(&program, "p").max(0) as u32;
        let loc = |name: &str| gl.get_uniform_location(&program, name);
        Some(Self {
            u_resolution: loc("resolution"),
            u_dpr: loc("dpr"),
            u_width: loc("width"),
            u_lift: loc("lift"),
            u_grow: loc("grow"),
            u_jelly: loc("jelly"),
            u_compact: loc("compact"),
            u_ink: loc("ink"),
            gl,
            program,
            attr_p,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn draw(
        &self,
        canvas: &HtmlCanvasElement,
        dpr: f64,
        width: f64,
        lift: f64,
        grow: f64,
        jelly: f64,
        compact: f64,
        ink: [f32; 3],
    ) -> bool {
        if self.gl.is_context_lost() || width <= 4.0 {
            return false;
        }
        let css_w = width + 160.0;
        let css_h = 112.0 + lift.clamp(0.0, 160.0) + 8.0;
        let pw = (css_w * dpr).round().max(1.0) as u32;
        let ph = (css_h * dpr).round().max(1.0) as u32;
        if canvas.width() != pw || canvas.height() != ph {
            canvas.set_width(pw);
            canvas.set_height(ph);
            let _ = canvas
                .style()
                .set_property("width", &format!("{css_w:.1}px"));
            let _ = canvas
                .style()
                .set_property("height", &format!("{css_h:.1}px"));
        }

        self.gl.viewport(0, 0, pw as i32, ph as i32);
        self.gl.use_program(Some(&self.program));
        self.gl.enable_vertex_attrib_array(self.attr_p);
        self.gl
            .vertex_attrib_pointer_with_i32(self.attr_p, 2, GL::FLOAT, false, 0, 0);

        self.gl
            .uniform2f(self.u_resolution.as_ref(), pw as f32, ph as f32);
        self.gl.uniform1f(self.u_dpr.as_ref(), dpr as f32);
        self.gl.uniform1f(self.u_width.as_ref(), width as f32);
        self.gl.uniform1f(self.u_lift.as_ref(), lift as f32);
        self.gl.uniform1f(self.u_grow.as_ref(), grow as f32);
        self.gl.uniform1f(self.u_jelly.as_ref(), jelly as f32);
        self.gl.uniform1f(self.u_compact.as_ref(), compact as f32);
        self.gl
            .uniform3f(self.u_ink.as_ref(), ink[0], ink[1], ink[2]);

        self.gl.draw_arrays(GL::TRIANGLES, 0, 6);
        true
    }
}

fn quintic(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

fn parse_rgb(window: &Window, el: &Element) -> [f32; 3] {
    let Some(color) = window
        .get_computed_style(el)
        .ok()
        .flatten()
        .and_then(|s| s.get_property_value("color").ok())
    else {
        return [0.09, 0.09, 0.09];
    };
    let nums: Vec<f32> = color
        .split(|c: char| !c.is_ascii_digit() && c != '.')
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse::<f32>().ok())
        .collect();
    if nums.len() >= 3 {
        [
            (nums[0] / 255.0).clamp(0.0, 1.0),
            (nums[1] / 255.0).clamp(0.0, 1.0),
            (nums[2] / 255.0).clamp(0.0, 1.0),
        ]
    } else {
        [0.09, 0.09, 0.09]
    }
}

struct DockState {
    window: Window,
    document: Document,
    reduced: MediaQueryList,
    dock: HtmlElement,
    btn: HtmlElement,
    canvas: HtmlCanvasElement,
    gl: Option<DockGl>,
    birth: f64,
    detach_t: f64,
    armed_top: bool,
    reattach_t: f64,
    armed_reattach: bool,
    prev_p: Option<f64>,
    spring_pos: f64,
    spring_vel: f64,
    hover_until: f64,
    last_ms: f64,
}

impl DockState {
    fn tick(&mut self, now: f64) -> bool {
        let (vw, vh) = viewport(&self.window);
        if self.document.hidden() || vw <= 680.0 {
            self.last_ms = 0.0;
            return false;
        }
        let dt_ms = if self.last_ms == 0.0 {
            16.0
        } else {
            (now - self.last_ms).clamp(1.0, 40.0)
        };
        self.last_ms = now;
        let reduced = self.reduced.matches();

        if reduced {
            self.spring_pos = 0.0;
            self.spring_vel = 0.0;
        } else {
            let mut rem = dt_ms / 1000.0;
            while rem > 0.0 {
                let step = rem.min(0.008);
                self.spring_vel += (-420.0 * self.spring_pos - 26.0 * self.spring_vel) * step;
                self.spring_pos += self.spring_vel * step;
                rem -= step;
            }
            if self.spring_pos.abs() < 0.02 && self.spring_vel.abs() < 0.2 {
                self.spring_pos = 0.0;
                self.spring_vel = 0.0;
            }
        }

        self.birth = (self.birth + dt_ms / 880.0).min(1.0);
        let scroll_y = self.window.scroll_y().unwrap_or_default().max(0.0);
        let grow = if reduced || scroll_y > 0.0 {
            1.0
        } else {
            quintic(self.birth)
        };

        let p = scroll_y
            + if reduced {
                0.0
            } else {
                (scroll_y * 0.18).min(100.0)
            };
        if p < 8.0 {
            self.armed_top = true;
            self.detach_t = -1.0;
        }
        if reduced {
            self.detach_t = -1.0;
            self.armed_top = false;
        } else if self.armed_top && p >= DETACH_SCROLL_PX {
            self.detach_t = 0.0;
            self.armed_top = false;
        } else if self.detach_t >= 0.0 {
            self.detach_t = (self.detach_t + dt_ms / 1000.0).min(1.2);
        }

        let lt = self.detach_t.max(0.0);
        let detached = self.detach_t >= 0.0;
        let g = quintic((p - 8.0) / (DETACH_SCROLL_PX - 8.0));
        let snap_wave = if lt < 1.2 {
            62.0 * (-7.0 * lt).exp() * (19.0 * lt).sin()
        } else {
            0.0
        };
        let detach_lift = if detached {
            g * (54.0 * quintic(lt / 0.08) + snap_wave)
        } else {
            0.0
        };
        let detach_jelly = if detached && lt < 1.2 {
            g * 0.1 * (-5.0 * lt).exp() * (23.0 * lt).sin()
        } else {
            0.0
        };
        let attached_frac = 1.0 - g;
        let base_lift = p + detach_lift - 10.0 * attached_frac;

        if reduced {
            self.reattach_t = -1.0;
            self.armed_reattach = false;
        } else {
            if p > DETACH_SCROLL_PX + 24.0 {
                self.armed_reattach = true;
                self.reattach_t = -1.0;
            }
            if self.armed_reattach
                && self.prev_p.is_some_and(|prev| p < prev)
                && base_lift <= DETACH_SCROLL_PX
            {
                self.reattach_t = 0.0;
                self.armed_reattach = false;
            } else if self.reattach_t >= 0.0 {
                self.reattach_t = (self.reattach_t + dt_ms / 1000.0).min(REATTACH_DUR);
            }
        }
        self.prev_p = Some(p);

        let rt = self.reattach_t;
        let suction = if !(0.0..REATTACH_DUR).contains(&rt) {
            0.0
        } else if rt < 0.09 {
            quintic(rt / 0.09)
        } else if rt < 0.24 {
            1.0 - 1.18 * quintic((rt - 0.09) / 0.15)
        } else {
            -0.18 * (1.0 - quintic((rt - 0.24) / (REATTACH_DUR - 0.24)))
        };

        let settled_lift = base_lift - 20.0 * suction * attached_frac;
        let lift = settled_lift - self.spring_pos;
        let jelly = detach_jelly
            - 0.15 * suction * attached_frac
            - (self.spring_pos * 0.012).clamp(-0.04, 0.12);
        let compact = quintic((settled_lift - DETACH_SCROLL_PX) / 48.0);

        // Off-screen above viewport -> sleep cleanly
        if lift > vh + 100.0 {
            css(&self.dock, "visibility", "hidden");
            flag(&self.dock, "inert", true);
            self.last_ms = 0.0;
            return false;
        }

        css(&self.dock, "visibility", "visible");
        flag(&self.dock, "inert", false);
        css(
            &self.dock,
            "--dock-content-y",
            &format!("{:.2}px", -5.0 * attached_frac),
        );
        css(
            &self.dock,
            "transform",
            &format!("translate(-50%, {:.2}px)", -lift),
        );

        let btn_w = f64::from(self.btn.client_width());
        let dpr = self.window.device_pixel_ratio().clamp(1.0, 2.0);
        let ink = parse_rgb(&self.window, &self.canvas);

        if let Some(gl) = &self.gl
            && gl.draw(&self.canvas, dpr, btn_w, lift, grow, jelly, compact, ink)
        {
            let surf_y = (1.0 - grow) * 86.0;
            css(&self.dock, "--dock-surface-y", &format!("{surf_y:.2}px"));
            css(
                &self.dock,
                "--dock-content",
                &format!("{:.3}", quintic((grow - 0.28) / 0.72)),
            );
            flag(&self.btn, "data-shader-ready", true);
            css(
                &self.canvas.clone().unchecked_into(),
                "visibility",
                "visible",
            );
        } else {
            flag(&self.btn, "data-shader-ready", false);
            css(&self.dock, "--dock-surface-y", "0px");
            css(&self.dock, "--dock-content", "1");
        }

        let keep = (self.birth < 1.0 && !reduced && scroll_y == 0.0)
            || (detached && lt < 1.2)
            || (0.0..REATTACH_DUR).contains(&rt)
            || self.spring_pos != 0.0
            || self.spring_vel != 0.0
            || now < self.hover_until;
        if !keep {
            self.last_ms = 0.0;
        }
        keep
    }
}

pub(super) fn init(
    window: &Window,
    document: &Document,
    root: &Element,
    reduced: &MediaQueryList,
) -> Result {
    let Ok(dock) = element::<HtmlElement>(root, "[data-command-dock]") else {
        return Ok(());
    };
    let btn: HtmlElement = element(&dock, "[data-dock-button]")?;
    let canvas: HtmlCanvasElement = element(&dock, "[data-dock-shape]")?;

    let gl = DockGl::new(&canvas);
    let state = Rc::new(RefCell::new(DockState {
        window: window.clone(),
        document: document.clone(),
        reduced: reduced.clone(),
        dock: dock.clone(),
        btn: btn.clone(),
        canvas,
        gl,
        birth: 0.0,
        detach_t: -1.0,
        armed_top: window.scroll_y().unwrap_or_default() < 8.0,
        reattach_t: -1.0,
        armed_reattach: false,
        prev_p: None,
        spring_pos: 0.0,
        spring_vel: 0.0,
        hover_until: 0.0,
        last_ms: 0.0,
    }));

    let frame = Frame::new(window, {
        let st = state.clone();
        move |now| st.borrow_mut().tick(now)
    });

    listen(window, "scroll", {
        let fr = frame.clone();
        move |_| fr.request()
    })?;
    listen(window, "resize", {
        let fr = frame.clone();
        move |_| fr.request()
    })?;

    for ev_name in ["pointerenter", "pointerleave"] {
        listen(&btn, ev_name, {
            let st = state.clone();
            let fr = frame.clone();
            move |_| {
                let now = st
                    .borrow()
                    .window
                    .performance()
                    .map(|p| p.now())
                    .unwrap_or_default();
                st.borrow_mut().hover_until = now + 320.0;
                fr.request();
            }
        })?;
    }

    listen(&btn, "click", {
        let st = state.clone();
        let fr = frame.clone();
        move |_| {
            {
                let mut s = st.borrow_mut();
                if !s.reduced.matches() {
                    s.spring_vel = (s.spring_vel + 250.0).min(360.0);
                }
            }
            fr.request();
        }
    })?;

    let ro_cb = Closure::<dyn FnMut()>::new({
        let fr = frame.clone();
        move || fr.request()
    });
    if let Ok(ro) = ResizeObserver::new(ro_cb.as_ref().unchecked_ref()) {
        ro.observe(&btn);
        std::mem::forget(ro);
    }
    ro_cb.forget();

    if let Some(doc_el) = document.document_element() {
        let mo_cb = Closure::<dyn FnMut()>::new({
            let st = state.clone();
            let fr = frame.clone();
            move || {
                if let Ok(mut s) = st.try_borrow_mut() {
                    let now = s.window.performance().map(|p| p.now()).unwrap_or_default();
                    s.hover_until = now + 260.0;
                }
                fr.request();
            }
        });
        if let Ok(mo) = MutationObserver::new(mo_cb.as_ref().unchecked_ref()) {
            let init = MutationObserverInit::new();
            init.set_attributes(true);
            let _ = mo.observe_with_options(&doc_el, &init);
            std::mem::forget(mo);
        }
        mo_cb.forget();
    }

    frame.request();
    Ok(())
}
