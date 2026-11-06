//! Draw, undo, redo, replay and export a signature with selectable ink color and stroke weight.
use crate::{
    foundation::{Theme, ThemeMode},
    primitives::{
        direct_touch::Contact,
        family::{Paint, PhaseClock, Tone, disabled_motion, face},
        form_chrome::tr,
        glyph::{Glyph, icon},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, fmt::Write, rc::Rc};

/// Ink color preset for signatures.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SignatureColor {
    #[default]
    Black,
    Blue,
    Violet,
}
impl SignatureColor {
    fn hsla(self, mode: ThemeMode, ink: Hsla) -> Hsla {
        match (self, mode) {
            (Self::Black, _) => ink,
            (Self::Blue, ThemeMode::Light) => rgb(0x245bc4).into(),
            (Self::Blue, ThemeMode::Dark) => rgb(0x8bb4ff).into(),
            (Self::Violet, ThemeMode::Light) => rgb(0x7142b6).into(),
            (Self::Violet, ThemeMode::Dark) => rgb(0xbc9cff).into(),
        }
    }
}

/// Stroke thickness preset.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SignatureWeight {
    Thin,
    #[default]
    Medium,
    Bold,
}
impl SignatureWeight {
    fn scale(self) -> f32 {
        match self {
            Self::Thin => 0.72,
            Self::Medium => 1.0,
            Self::Bold => 1.4,
        }
    }
}

/// A stroke of normalized points. Coordinates are in the inclusive 0..=1 range.
#[derive(Clone, Debug, PartialEq)]
pub struct SignatureStroke {
    pub points: Vec<(f32, f32)>,
    /// Per-point pressure from 0 to 1. Missing values fall back to 0.5.
    pub pressure: Vec<f32>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SignatureChanged {
    pub is_empty: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignaturePngError {
    InvalidDimensions,
    AllocationFailed,
}
#[derive(Clone, Debug)]
pub struct SignaturePngExported(pub Result<Vec<u8>, SignaturePngError>);
/// Drawn ink is stored in normalized coordinates so it survives resizing.
pub struct SignaturePad {
    label: SharedString,
    signer: SharedString,
    color: SignatureColor,
    weight: SignatureWeight,
    strokes: Vec<SignatureStroke>,
    clearing: Vec<SignatureStroke>,
    redo: Vec<SignatureStroke>,
    bounds: Rc<Cell<Bounds<Pixels>>>,
    contact: Contact,
    dragging: bool,
    focus: FocusHandle,
    disabled: bool,
    replaying: bool,
    replay_clock: PhaseClock,
    sweep_clock: PhaseClock,
    sweeping: bool,
}
impl EventEmitter<SignatureChanged> for SignaturePad {}
impl EventEmitter<SignaturePngExported> for SignaturePad {}
impl SignaturePad {
    pub fn new(label: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        Self {
            label: label.into(),
            signer: "Emma Collins".into(),
            color: SignatureColor::Black,
            weight: SignatureWeight::Medium,
            strokes: Vec::new(),
            clearing: Vec::new(),
            redo: Vec::new(),
            bounds: Default::default(),
            contact: Default::default(),
            dragging: false,
            focus: cx.focus_handle().tab_stop(true),
            disabled: false,
            replaying: false,
            replay_clock: Default::default(),
            sweep_clock: Default::default(),
            sweeping: false,
        }
    }
    pub fn with_signer(mut self, signer: impl Into<SharedString>) -> Self {
        self.signer = signer.into();
        self
    }
    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
    pub fn strokes(&self) -> &[SignatureStroke] {
        &self.strokes
    }
    pub fn is_empty(&self) -> bool {
        self.strokes.is_empty()
    }
    fn changed(&self, cx: &mut Context<Self>) {
        cx.emit(SignatureChanged {
            is_empty: self.is_empty(),
        });
        cx.notify();
    }
    pub fn clear(&mut self, cx: &mut Context<Self>) {
        if !self.is_empty() {
            self.replaying = false;
            if !disabled_motion(cx) {
                self.clearing = std::mem::take(&mut self.strokes);
                self.sweeping = true;
                self.sweep_clock.restart(cx);
            } else {
                self.strokes.clear();
            }
            self.redo.clear();
            self.changed(cx)
        }
    }
    pub fn undo(&mut self, cx: &mut Context<Self>) {
        if let Some(stroke) = self.strokes.pop() {
            self.replaying = false;
            self.redo.push(stroke);
            self.changed(cx)
        }
    }
    pub fn redo(&mut self, cx: &mut Context<Self>) {
        if let Some(stroke) = self.redo.pop() {
            self.replaying = false;
            self.strokes.push(stroke);
            self.changed(cx)
        }
    }
    fn position(&self, p: Point<Pixels>) -> (f32, f32) {
        let b = self.bounds.get();
        let x = (p.x - b.left()) / b.size.width.max(px(1.));
        let y = (p.y - b.top()) / b.size.height.max(px(1.));
        (x.clamp(0., 1.), y.clamp(0., 1.))
    }
    fn begin(&mut self, p: Point<Pixels>, w: &mut Window, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        self.focus.focus(w, cx);
        self.begin_sample(self.position(p), 0.5, cx);
    }
    /// Host input adapters can supply normalized positions and real pen pressure.
    pub fn begin_sample(&mut self, position: (f32, f32), pressure: f32, cx: &mut Context<Self>) {
        if self.disabled || !position.0.is_finite() || !position.1.is_finite() {
            return;
        }
        self.replaying = false;
        self.sweeping = false;
        self.clearing.clear();
        self.dragging = true;
        self.redo.clear();
        self.strokes.push(SignatureStroke {
            points: vec![(position.0.clamp(0., 1.), position.1.clamp(0., 1.))],
            pressure: vec![safe_pressure(pressure)],
        });
        self.changed(cx);
    }
    fn add(&mut self, p: Point<Pixels>, cx: &mut Context<Self>) {
        if !self.dragging {
            return;
        }
        let next = self.position(p);
        let speed_pressure = self
            .strokes
            .last()
            .and_then(|s| s.points.last().copied())
            .map_or(0.55, |(x0, y0)| {
                let d = (next.0 - x0).hypot(next.1 - y0);
                (0.75 - d * 6.).clamp(0.25, 0.78)
            });
        self.append_sample(next, speed_pressure, cx);
    }
    /// Add one host-supplied pressure sample to the active stroke.
    pub fn append_sample(&mut self, position: (f32, f32), pressure: f32, cx: &mut Context<Self>) {
        if !self.dragging || !position.0.is_finite() || !position.1.is_finite() {
            return;
        }
        let position = (position.0.clamp(0., 1.), position.1.clamp(0., 1.));
        if let Some(stroke) = self.strokes.last_mut()
            && stroke.points.last() != Some(&position)
        {
            stroke.points.push(position);
            stroke.pressure.push(safe_pressure(pressure));
            cx.notify();
        }
    }
    fn finish(&mut self, p: Point<Pixels>, cx: &mut Context<Self>) {
        if self.dragging {
            self.add(p, cx);
            self.end_sample(cx);
        }
    }
    pub fn end_sample(&mut self, cx: &mut Context<Self>) {
        if self.dragging {
            self.dragging = false;
            self.changed(cx);
        }
    }
    /// Replays a copy of the strokes; the underlying signature is never changed.
    pub fn replay(&mut self, cx: &mut Context<Self>) {
        if !self.strokes.is_empty() {
            self.replaying = true;
            self.replay_clock.restart(cx);
            cx.notify();
        }
    }
    /// Export the current strokes. The host chooses where or whether to save the file.
    pub fn to_svg(&self, width: u32, height: u32) -> String {
        svg_for_strokes(&self.strokes, width, height)
    }
    /// Render transparent PNG bytes in shared Rust. Failure leaves the ink untouched.
    pub fn to_png(&self, width: u32, height: u32) -> Result<Vec<u8>, SignaturePngError> {
        png_for_strokes(&self.strokes, width, height)
    }
}
fn png_for_strokes(
    strokes: &[SignatureStroke],
    width: u32,
    height: u32,
) -> Result<Vec<u8>, SignaturePngError> {
    if width == 0
        || height == 0
        || width > 2048
        || height > 2048
        || u64::from(width) * u64::from(height) > 4_194_304
    {
        return Err(SignaturePngError::InvalidDimensions);
    }
    let stride = width as usize * 4 + 1;
    let size = stride * height as usize;
    let mut pixels = Vec::new();
    pixels
        .try_reserve_exact(size)
        .map_err(|_| SignaturePngError::AllocationFailed)?;
    pixels.resize(size, 0u8);
    let mut dab = |x: f32, y: f32, radius: f32| {
        let xmin = ((x - radius - 1.).floor() as i32).max(0);
        let xmax = ((x + radius + 1.).ceil() as i32).min(width as i32 - 1);
        let ymin = ((y - radius - 1.).floor() as i32).max(0);
        let ymax = ((y + radius + 1.).ceil() as i32).min(height as i32 - 1);
        for py in ymin..=ymax {
            for px in xmin..=xmax {
                let dx = px as f32 + 0.5 - x;
                let dy = py as f32 + 0.5 - y;
                let coverage = (radius + 0.5 - (dx * dx + dy * dy).sqrt()).clamp(0., 1.);
                let index = py as usize * stride + 1 + px as usize * 4 + 3;
                pixels[index] = pixels[index].max((coverage * 255.).round() as u8);
            }
        }
    };
    for stroke in strokes {
        if let Some(&(x, y)) = stroke.points.first() {
            dab(
                x * width as f32,
                y * height as f32,
                ink_width(stroke, 0) * 0.5,
            );
        }
        for (index, pair) in stroke.points.windows(2).enumerate() {
            let (x0, y0) = (pair[0].0 * width as f32, pair[0].1 * height as f32);
            let (x1, y1) = (pair[1].0 * width as f32, pair[1].1 * height as f32);
            let steps = (((x1 - x0).hypot(y1 - y0) * 2.).ceil() as usize).max(1);
            let a = ink_width(stroke, index);
            let b = ink_width(stroke, index + 1);
            for step in 1..=steps {
                let t = step as f32 / steps as f32;
                dab(
                    x0 + (x1 - x0) * t,
                    y0 + (y1 - y0) * t,
                    (a + (b - a) * t) * 0.5,
                );
            }
        }
    }
    let mut zlib = Vec::new();
    zlib.try_reserve_exact(size + size.div_ceil(65535) * 5 + 6)
        .map_err(|_| SignaturePngError::AllocationFailed)?;
    zlib.extend_from_slice(&[0x78, 0x01]);
    for (index, block) in pixels.chunks(65535).enumerate() {
        let final_block = (index + 1) * 65535 >= pixels.len();
        zlib.push(u8::from(final_block));
        let length = block.len() as u16;
        zlib.extend_from_slice(&length.to_le_bytes());
        zlib.extend_from_slice(&(!length).to_le_bytes());
        zlib.extend_from_slice(block);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for byte in &pixels {
        a = (a + u32::from(*byte)) % 65521;
        b = (b + a) % 65521;
    }
    zlib.extend_from_slice(&((b << 16) | a).to_be_bytes());
    fn chunk(output: &mut Vec<u8>, kind: &[u8; 4], bytes: &[u8]) {
        output.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
        output.extend_from_slice(kind);
        output.extend_from_slice(bytes);
        let mut crc = 0xffff_ffffu32;
        for byte in kind.iter().chain(bytes) {
            crc ^= u32::from(*byte);
            for _ in 0..8 {
                crc = (crc >> 1) ^ (0xedb8_8320u32 & (0u32.wrapping_sub(crc & 1)));
            }
        }
        output.extend_from_slice(&(!crc).to_be_bytes());
    }
    let mut png = Vec::new();
    png.try_reserve_exact(zlib.len() + 57)
        .map_err(|_| SignaturePngError::AllocationFailed)?;
    png.extend_from_slice(b"\x89PNG\r\n\x1a\n");
    let mut header = Vec::with_capacity(13);
    header.extend_from_slice(&width.to_be_bytes());
    header.extend_from_slice(&height.to_be_bytes());
    header.extend_from_slice(&[8, 6, 0, 0, 0]);
    chunk(&mut png, b"IHDR", &header);
    chunk(&mut png, b"IDAT", &zlib);
    chunk(&mut png, b"IEND", &[]);
    Ok(png)
}
fn safe_pressure(pressure: f32) -> f32 {
    if pressure.is_finite() {
        pressure.clamp(0., 1.)
    } else {
        0.5
    }
}
fn ink_width(stroke: &SignatureStroke, index: usize) -> f32 {
    1. + 2. * safe_pressure(stroke.pressure.get(index).copied().unwrap_or(0.5))
}
fn svg_for_strokes(strokes: &[SignatureStroke], width: u32, height: u32) -> String {
    let (width, height) = (width.max(1), height.max(1));
    let mut out = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {width} {height}\" width=\"{width}\" height=\"{height}\"><g fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\">"
    );
    for stroke in strokes {
        if let Some(&(x, y)) = stroke.points.first() {
            if stroke.points.len() == 1 {
                let _ = write!(
                    out,
                    "<path stroke-width=\"{:.2}\" d=\"M {:.2} {:.2} l .01 0\"/>",
                    ink_width(stroke, 0),
                    x * width as f32,
                    y * height as f32
                );
            } else {
                for (index, pair) in stroke.points.windows(2).enumerate() {
                    let (x0, y0) = pair[0];
                    let (x1, y1) = pair[1];
                    let _ = write!(
                        out,
                        "<path stroke-width=\"{:.2}\" d=\"M {:.2} {:.2} Q {:.2} {:.2} {:.2} {:.2} L {:.2} {:.2}\"/>",
                        (ink_width(stroke, index) + ink_width(stroke, index + 1)) * 0.5,
                        x0 * width as f32,
                        y0 * height as f32,
                        x0 * width as f32,
                        y0 * height as f32,
                        (x0 + x1) * 0.5 * width as f32,
                        (y0 + y1) * 0.5 * height as f32,
                        x1 * width as f32,
                        y1 * height as f32
                    );
                }
            }
        }
    }
    out.push_str("</g></svg>");
    out
}
/// Cubic-bezier(0.65, 0, 0.35, 1), the in-out curve used by the clearing sweep.
fn sweep_ease(x: f32) -> f32 {
    if x <= 0. || x >= 1. {
        return x.clamp(0., 1.);
    }
    let (x1, x2) = (0.65_f32, 0.35_f32);
    let mut t = x;
    for _ in 0..8 {
        let u = 1. - t;
        let xt = 3. * u * u * t * x1 + 3. * u * t * t * x2 + t * t * t;
        let dx = 3. * u * u * x1 + 6. * u * t * (x2 - x1) + 3. * t * t * (1. - x2);
        if dx.abs() < 1e-5 {
            break;
        }
        t = (t - (xt - x) / dx).clamp(0., 1.);
    }
    let u = 1. - t;
    3. * u * t * t + t * t * t
}
impl Render for SignaturePad {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let mode = Theme::global(cx).mode;
        let radius = crate::foundation::surface_radius(cx);
        let bounds = self.bounds.clone();
        let sweep_t = if self.sweeping {
            let t = self.sweep_clock.time(0.56, w, cx);
            if t >= 0.56 {
                self.sweeping = false;
                self.clearing.clear();
                1.
            } else {
                sweep_ease(t / 0.56)
            }
        } else {
            1.
        };
        let mut strokes = if self.sweeping {
            self.clearing.clone()
        } else {
            self.strokes.clone()
        };
        if self.replaying {
            let t = self.replay_clock.time(1.6, w, cx);
            if t >= 1.6 {
                self.replaying = false;
            }
            let total: usize = strokes.iter().map(|stroke| stroke.points.len()).sum();
            let mut remaining = (total as f32 * (t / 1.6).clamp(0., 1.)).ceil() as usize;
            for stroke in &mut strokes {
                let keep = remaining.min(stroke.points.len());
                remaining -= keep;
                stroke.points.truncate(keep);
                stroke.pressure.truncate(keep);
            }
            strokes.retain(|stroke| !stroke.points.is_empty());
        }
        let ink_color = self.color.hsla(mode, p.ink);
        let weight_scale = self.weight.scale();
        let sweeping = self.sweeping;
        let baseline_color = p.faint;
        let ink = canvas(
            move |b, _, _| bounds.set(b),
            move |b, _, w, _| {
                // Dashed baseline at 72%
                let y_base = b.top() + b.size.height * 0.72;
                let x_start = f32::from(b.left() + b.size.width * 0.07);
                let x_end = f32::from(b.left() + b.size.width * 0.93);
                let mut x = x_start;
                while x < x_end {
                    let next = (x + 5.).min(x_end);
                    let mut dash = PathBuilder::stroke(px(1.5));
                    dash.move_to(point(px(x), y_base));
                    dash.line_to(point(px(next), y_base));
                    if let Ok(path) = dash.build() {
                        w.paint_path(path, baseline_color.opacity(0.65));
                    }
                    x += 9.;
                }
                let clip_left = if sweeping {
                    b.left() + b.size.width * sweep_t
                } else {
                    b.left()
                };
                let mask = Bounds {
                    origin: point(clip_left, b.top()),
                    size: size((b.right() - clip_left).max(px(0.)), b.size.height),
                };
                w.with_content_mask(Some(ContentMask { bounds: mask }), |w| {
                    for stroke in &strokes {
                        if stroke.points.len() == 1 {
                            let (x, y) = stroke.points[0];
                            let at = b.origin + point(b.size.width * x, b.size.height * y);
                            let mut path =
                                PathBuilder::stroke(px(ink_width(stroke, 0) * weight_scale));
                            path.move_to(at);
                            path.line_to(at + point(px(0.01), px(0.)));
                            if let Ok(path) = path.build() {
                                w.paint_path(path, ink_color);
                            }
                        } else {
                            for (index, pair) in stroke.points.windows(2).enumerate() {
                                let (x0, y0) = pair[0];
                                let (x1, y1) = pair[1];
                                let a = b.origin + point(b.size.width * x0, b.size.height * y0);
                                let end = b.origin + point(b.size.width * x1, b.size.height * y1);
                                let mid = b.origin
                                    + point(
                                        b.size.width * ((x0 + x1) * 0.5),
                                        b.size.height * ((y0 + y1) * 0.5),
                                    );
                                let mut path = PathBuilder::stroke(px((ink_width(stroke, index)
                                    + ink_width(stroke, index + 1))
                                    * 0.5
                                    * weight_scale));
                                path.move_to(a);
                                path.curve_to(mid, a);
                                path.line_to(end);
                                if let Ok(path) = path.build() {
                                    w.paint_path(path, ink_color);
                                }
                            }
                        }
                    }
                });
                if sweeping {
                    let sx = b.left() + b.size.width * sweep_t;
                    w.paint_quad(fill(
                        Bounds::new(
                            point(sx - px(1.), b.top() + b.size.height * 0.08),
                            size(px(2.), b.size.height * 0.84),
                        ),
                        p.ink.opacity(0.5),
                    ));
                }
            },
        )
        .absolute()
        .inset_0();
        let mut surface = div()
            .id("signature-pad-area")
            .relative()
            .w_full()
            .aspect_ratio(600. / 260.)
            .rounded(radius)
            .border_1()
            .border_color(if self.dragging { p.faint } else { p.line })
            .bg(p.panel)
            .overflow_hidden()
            .track_focus(&self.focus)
            .tab_index(if self.disabled { -1 } else { 0 })
            .role(Role::Image)
            .aria_label(self.label.clone())
            .focus_visible(move |s| s.border_color(p.focus))
            .child(ink)
            .child(
                div()
                    .absolute()
                    .left(relative(0.07))
                    .top(relative(0.72))
                    .mt(px(-22.))
                    .child(icon(Glyph::Close, 12., p.faint)),
            )
            .when(!self.signer.is_empty(), |s| {
                s.child(
                    div()
                        .absolute()
                        .left(relative(0.07))
                        .top(relative(0.72))
                        .mt(px(8.))
                        .text_size(px(12.))
                        .text_color(p.faint)
                        .child(self.signer.clone()),
                )
            })
            .when(self.strokes.is_empty() && !self.sweeping, |s| {
                s.child(
                    div()
                        .absolute()
                        .left_0()
                        .right_0()
                        .top(relative(0.72))
                        .mt(px(-34.))
                        .flex()
                        .justify_center()
                        .text_size(px(16.))
                        .text_color(p.faint)
                        .child(self.label.clone()),
                )
            });
        if !self.disabled {
            surface = surface
                .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                    let key = e.keystroke.key.as_str();
                    let command = e.keystroke.modifiers.platform || e.keystroke.modifiers.control;
                    match key {
                        "z" if command && e.keystroke.modifiers.shift => s.redo(cx),
                        "z" if command => s.undo(cx),
                        "backspace" | "delete" => s.clear(cx),
                        _ => return,
                    };
                    cx.stop_propagation();
                    w.prevent_default();
                }))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|s, e: &MouseDownEvent, w, cx| s.begin(e.position, w, cx)),
                )
                .on_mouse_up(
                    MouseButton::Left,
                    cx.listener(|s, e: &MouseUpEvent, _, cx| s.finish(e.position, cx)),
                )
                .on_mouse_up_out(
                    MouseButton::Left,
                    cx.listener(|s, e: &MouseUpEvent, _, cx| s.finish(e.position, cx)),
                )
                .when(self.dragging, |surface| {
                    let moved = cx.listener(|s, e: &MouseMoveEvent, _, cx| {
                        if e.pressed_button == Some(MouseButton::Left) {
                            s.add(e.position, cx)
                        }
                    });
                    let released =
                        cx.listener(|s, e: &MouseUpEvent, _, cx| s.finish(e.position, cx));
                    surface.child(
                        canvas(
                            |_, _, _| (),
                            move |_, _, w, _| {
                                w.on_mouse_event(move |e: &MouseMoveEvent, phase, w, cx| {
                                    if phase.capture() {
                                        moved(e, w, cx);
                                    }
                                });
                                w.on_mouse_event(move |e: &MouseUpEvent, phase, w, cx| {
                                    if phase.capture() && e.button == MouseButton::Left {
                                        released(e, w, cx);
                                    }
                                });
                            },
                        )
                        .absolute()
                        .size(px(0.)),
                    )
                })
                .child(self.contact.listen(cx, |s, e, w, cx| match e.phase {
                    TouchPhase::Started => s.begin(e.position, w, cx),
                    TouchPhase::Moved => s.add(e.position, cx),
                    TouchPhase::Ended | TouchPhase::Cancelled => s.finish(e.position, cx),
                }));
        }
        let undo = tr(cx, "Undo", "撤销");
        let redo = tr(cx, "Redo", "重做");
        let replay = tr(cx, "Replay", "回放");
        let png = tr(cx, "Export PNG", "导出 PNG");
        let clear = tr(cx, "Clear", "清除");
        let mut color_group = div().flex().items_center().gap(px(2.));
        for (i, (c_opt, name)) in [
            (SignatureColor::Black, tr(cx, "Black ink", "黑色墨水")),
            (SignatureColor::Blue, tr(cx, "Blue ink", "蓝色墨水")),
            (SignatureColor::Violet, tr(cx, "Violet ink", "紫色墨水")),
        ]
        .into_iter()
        .enumerate()
        {
            let active = self.color == c_opt;
            let swatch = c_opt.hsla(mode, p.ink);
            color_group = color_group.child(
                face(("sig-color", i), name, 36., 18., Tone::Clear, cx)
                    .size(px(36.))
                    .when(active, |b| b.bg(p.hover).border_1().border_color(p.line))
                    .hover(move |s| s.bg(p.hover))
                    .child(div().size(px(16.)).rounded_full().bg(swatch))
                    .on_click(cx.listener(move |s, _, _, cx| {
                        s.color = c_opt;
                        cx.notify();
                    })),
            );
        }
        let mut weight_group = div().flex().items_center().gap(px(2.));
        for (i, (w_opt, dot_sz, name)) in [
            (SignatureWeight::Thin, 5.4, tr(cx, "Thin stroke", "细笔触")),
            (
                SignatureWeight::Medium,
                7.6,
                tr(cx, "Medium stroke", "中笔触"),
            ),
            (SignatureWeight::Bold, 10.4, tr(cx, "Bold stroke", "粗笔触")),
        ]
        .into_iter()
        .enumerate()
        {
            let active = self.weight == w_opt;
            weight_group = weight_group.child(
                face(("sig-weight", i), name, 36., 18., Tone::Clear, cx)
                    .size(px(36.))
                    .when(active, |b| b.bg(p.hover).border_1().border_color(p.line))
                    .hover(move |s| s.bg(p.hover))
                    .child(div().size(px(dot_sz)).rounded_full().bg(p.ink))
                    .on_click(cx.listener(move |s, _, _, cx| {
                        s.weight = w_opt;
                        cx.notify();
                    })),
            );
        }
        let button = |id: &'static str, label: &'static str, cx: &App| {
            face(id, label, 36., radius.into(), Tone::Clear, cx)
                .min_w(px(76.))
                .px(px(12.))
                .hover(move |s| s.bg(p.hover))
                .child(label)
        };
        div()
            .flex()
            .flex_col()
            .gap(px(10.))
            .w_full()
            .max_w_full()
            .when(self.disabled, |s| s.opacity(0.5))
            .child(surface)
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .child(color_group)
                            .child(div().w(px(1.)).h(px(20.)).bg(p.line))
                            .child(weight_group),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .gap(px(6.))
                            .child(button("signature-undo", undo, cx).on_click(cx.listener(
                                |s, _, _, cx| {
                                    if !s.disabled {
                                        s.undo(cx)
                                    }
                                },
                            )))
                            .child(button("signature-redo", redo, cx).on_click(cx.listener(
                                |s, _, _, cx| {
                                    if !s.disabled {
                                        s.redo(cx)
                                    }
                                },
                            )))
                            .child(button("signature-replay", replay, cx).on_click(cx.listener(
                                |s, _, _, cx| {
                                    if !s.disabled {
                                        s.replay(cx)
                                    }
                                },
                            )))
                            .child(button("signature-clear", clear, cx).on_click(cx.listener(
                                |s, _, _, cx| {
                                    if !s.disabled {
                                        s.clear(cx)
                                    }
                                },
                            )))
                            .child(
                                button("signature-png", png, cx)
                                    .border_1()
                                    .border_color(p.line)
                                    .on_click(cx.listener(|s, _, _, cx| {
                                        if !s.disabled {
                                            cx.emit(SignaturePngExported(s.to_png(520, 200)));
                                        }
                                    })),
                            ),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::{SignatureStroke, png_for_strokes, svg_for_strokes};
    #[test]
    fn export_scales_normalized_points_and_keeps_single_point_strokes() {
        let strokes = vec![
            SignatureStroke {
                points: vec![(0.5, 0.25), (1.0, 1.0)],
                pressure: vec![0., 1.],
            },
            SignatureStroke {
                points: vec![(0.0, 0.0)],
                pressure: vec![0.5],
            },
        ];
        let svg = svg_for_strokes(&strokes, 200, 100);
        assert!(svg.contains("M 100.00 25.00"));
        assert!(svg.contains("L 200.00 100.00"));
        assert!(svg.contains("stroke-width=\"2.00\""));
        assert!(svg.contains("M 0.00 0.00 l .01 0"));
        assert!(svg.ends_with("</g></svg>"));
    }
    #[test]
    fn png_export_and_failure_leave_original_strokes_intact() {
        let strokes = vec![SignatureStroke {
            points: vec![(0.3, 0.4)],
            pressure: vec![0.7],
        }];
        let before = strokes.clone();
        let bytes = png_for_strokes(&strokes, 20, 10).unwrap();
        assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
        assert!(png_for_strokes(&strokes, 0, 100).is_err());
        assert_eq!(strokes, before);
    }
}
