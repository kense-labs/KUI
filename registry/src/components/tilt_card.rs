//! Three-dimensional perspective card with pointer-tracking glare and spring recovery.
//!
//! Tilts smoothly toward the pointer on hover, projects rounded card geometry and
//! soft radial specular glare through a configurable perspective camera, and springs
//! back with a gentle overshoot on leave.

use crate::{
    foundation::{Theme, ThemeMode},
    primitives::{
        control::UiLanguage,
        family::{Slide, disabled_motion},
    },
};
use gpui::{prelude::*, *};
use std::{
    cell::{Cell, RefCell},
    f32::consts::PI,
    rc::Rc,
};
use web_time::Instant;

const DEFAULT_WIDTH: f32 = 340.;
const DEFAULT_HEIGHT: f32 = 214.;
const DEFAULT_MAX_TILT: f32 = 10.;
const DEFAULT_SCALE: f32 = 1.02;
const DEFAULT_PERSPECTIVE: f32 = 900.;
const DEFAULT_RADIUS: f32 = 20.;

type ContentBuilder = Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>;

/// Analytic second-order spring supporting both snappy hover tracking (`zeta = 0.90`)
/// and soft-overshoot release recovery (`zeta = 0.56`, matching `cubic-bezier(0.34, 1.36, 0.64, 1)`).
#[derive(Clone, Copy, Debug)]
struct TiltSpring {
    from: f32,
    target: f32,
    velocity: f32,
    start: Option<Instant>,
    omega: f32,
    damping: f32,
}

impl TiltSpring {
    fn new(rest: f32) -> Self {
        Self {
            from: rest,
            target: rest,
            velocity: 0.,
            start: None,
            omega: 11.2,
            damping: 0.56,
        }
    }

    fn reset(&mut self, value: f32) {
        self.from = value;
        self.target = value;
        self.velocity = 0.;
        self.start = None;
    }

    fn sample(self, now: Instant) -> (f32, f32, bool) {
        let Some(start) = self.start else {
            return (self.target, 0., false);
        };
        let t = now.saturating_duration_since(start).as_secs_f32();
        let zeta = self.damping.clamp(0.15, 0.995);
        let decay = self.omega * zeta;
        let freq = self.omega * (1. - zeta * zeta).sqrt();
        let a = self.from - self.target;
        let b = (self.velocity + decay * a) / freq;
        let (sin, cos) = (t * freq).sin_cos();
        let env = (-decay * t).exp();
        let wave = a * cos + b * sin;
        let delta = env * wave;
        let vel = env * (freq * (-a * sin + b * cos) - decay * wave);
        if (delta.abs() < 0.0008 && vel.abs() < 0.01) || t > 1.35 {
            (self.target, 0., false)
        } else {
            (self.target + delta, vel, true)
        }
    }

    fn retarget(&mut self, target: f32, omega: f32, damping: f32, now: Instant) {
        if (self.target - target).abs() < 0.0004
            && (self.omega - omega).abs() < 0.05
            && (self.damping - damping).abs() < 0.02
        {
            return;
        }
        let (pos, vel, _) = self.sample(now);
        self.from = pos;
        self.velocity = vel;
        self.target = target;
        self.omega = omega;
        self.damping = damping;
        self.start = Some(now);
    }
}

/// Three-dimensional pinhole projection of the card plane in normalized `0.0..=1.0` coordinates.
#[derive(Clone, Copy, Debug)]
struct TiltSurface {
    bounds: Bounds<Pixels>,
    radius: Pixels,
    rx: (f32, f32),
    ry: (f32, f32),
    scale: f32,
    lift_px: f32,
    perspective: f32,
}

impl TiltSurface {
    fn new(
        bounds: Bounds<Pixels>,
        radius: Pixels,
        rx_rad: f32,
        ry_rad: f32,
        scale: f32,
        lift_px: f32,
        perspective: f32,
    ) -> Self {
        Self {
            bounds,
            radius,
            rx: rx_rad.sin_cos(),
            ry: ry_rad.sin_cos(),
            scale,
            lift_px,
            perspective: perspective.max(180.),
        }
    }

    fn project(self, u: f32, v: f32) -> Point<Pixels> {
        let x = f32::from(self.bounds.size.width) * self.scale * (u - 0.5);
        let y = f32::from(self.bounds.size.height) * self.scale * (v - 0.5);
        let (sy, cy) = self.ry;
        let (sx, cx) = self.rx;
        let z1 = -x * sy;
        let y2 = y * cx - z1 * sx;
        let z2 = y * sx + z1 * cx;
        let persp = self.perspective / (self.perspective - z2).max(40.);
        self.bounds.center() + point(px(x * cy * persp), px(y2 * persp - self.lift_px))
    }

    fn radii_uv(self) -> (f32, f32) {
        let w = f32::from(self.bounds.size.width).max(1.);
        let h = f32::from(self.bounds.size.height).max(1.);
        let r = f32::from(self.radius).max(0.);
        ((r / w).min(0.48), (r / h).min(0.48))
    }

    fn outline_uv(self) -> impl Iterator<Item = Point<f32>> {
        let (rx, ry) = self.radii_uv();
        Self::rounded_rect_uv((0., 0., 1., 1.), (rx, ry), 9)
    }

    fn rounded_rect_uv(
        rect: (f32, f32, f32, f32),
        radii: (f32, f32),
        steps: usize,
    ) -> impl Iterator<Item = Point<f32>> {
        let (u0, v0, u1, v1) = rect;
        let rx = radii.0.min((u1 - u0) * 0.49).max(0.);
        let ry = radii.1.min((v1 - v0) * 0.49).max(0.);
        let corners = [
            (u1 - rx, v0 + ry),
            (u1 - rx, v1 - ry),
            (u0 + rx, v1 - ry),
            (u0 + rx, v0 + ry),
        ];
        let steps = steps.max(2);
        corners
            .into_iter()
            .enumerate()
            .flat_map(move |(idx, (cu, cv))| {
                (0..=steps).map(move |step| {
                    let angle = (idx as f32 - 1. + step as f32 / steps as f32) * (PI * 0.5);
                    point(cu + angle.cos() * rx, cv + angle.sin() * ry)
                })
            })
    }

    fn clip_rounded(self, u: f32, v: f32) -> Point<f32> {
        let (rx, ry) = self.radii_uv();
        let u = u.clamp(0., 1.);
        let v = v.clamp(0., 1.);
        if rx <= 1e-4 || ry <= 1e-4 {
            return point(u, v);
        }
        let cu = u.clamp(rx, 1. - rx);
        let cv = v.clamp(ry, 1. - ry);
        let len = ((u - cu) / rx).hypot((v - cv) / ry).max(1.);
        point(cu + (u - cu) / len, cv + (v - cv) / len)
    }

    fn fill_polygon(
        self,
        pts: impl IntoIterator<Item = Point<f32>>,
        bg: impl Into<Background>,
        w: &mut Window,
    ) {
        let mut iter = pts.into_iter();
        let Some(first) = iter.next() else {
            return;
        };
        let mut pb = PathBuilder::fill();
        pb.move_to(self.project(first.x, first.y));
        for pt in iter {
            pb.line_to(self.project(pt.x, pt.y));
        }
        pb.close();
        if let Ok(path) = pb.build() {
            w.paint_path(path, bg);
        }
    }

    fn stroke_polygon(
        self,
        pts: impl IntoIterator<Item = Point<f32>>,
        stroke_px: f32,
        color: Hsla,
        w: &mut Window,
    ) {
        let mut iter = pts.into_iter();
        let Some(first) = iter.next() else {
            return;
        };
        let mut pb = PathBuilder::stroke(px(stroke_px));
        pb.move_to(self.project(first.x, first.y));
        for pt in iter {
            pb.line_to(self.project(pt.x, pt.y));
        }
        pb.close();
        if let Ok(path) = pb.build() {
            w.paint_path(path, color);
        }
    }

    fn projected_bounds(self) -> Bounds<Pixels> {
        let p0 = self.project(0., 0.);
        let mut low = p0;
        let mut high = p0;
        for (u, v) in [(1., 0.), (0., 1.), (1., 1.)] {
            let p = self.project(u, v);
            low.x = low.x.min(p.x);
            low.y = low.y.min(p.y);
            high.x = high.x.max(p.x);
            high.y = high.y.max(p.y);
        }
        Bounds::from_corners(low, high)
    }
}

struct TiltLiveState {
    bounds: Rc<Cell<Bounds<Pixels>>>,
    nx: TiltSpring,
    ny: TiltSpring,
    active: TiltSpring,
    gx: Slide,
    gy: Slide,
    glare_alpha: Slide,
    hovered: bool,
}

impl TiltLiveState {
    fn new() -> Self {
        Self {
            bounds: Rc::new(Cell::new(Bounds::default())),
            nx: TiltSpring::new(0.),
            ny: TiltSpring::new(0.),
            active: TiltSpring::new(0.),
            gx: Slide::snappy(0.5),
            gy: Slide::snappy(0.5),
            glare_alpha: Slide::new(0.),
            hovered: false,
        }
    }

    fn pointer_move(&mut self, pos: Point<Pixels>, reduced: bool, cx: &App) -> bool {
        let b = self.bounds.get();
        let w = f32::from(b.size.width);
        let h = f32::from(b.size.height);
        if w <= 1. || h <= 1. {
            return false;
        }
        let u = ((f32::from(pos.x - b.left())) / w).clamp(0., 1.);
        let v = ((f32::from(pos.y - b.top())) / h).clamp(0., 1.);
        let nx = (u - 0.5) * 2.;
        let ny = (v - 0.5) * 2.;
        self.hovered = true;

        if reduced {
            self.nx.reset(0.);
            self.ny.reset(0.);
            self.active.reset(1.);
            self.glare_alpha.reset(0.);
            return true;
        }

        let now = Instant::now();
        // Snappy 140ms hover tracking while pointer is over the card.
        self.nx.retarget(nx, 24.0, 0.90, now);
        self.ny.retarget(ny, 24.0, 0.90, now);
        self.active.retarget(1., 22.0, 0.88, now);
        self.gx.duration(u, 0.16, cx);
        self.gy.duration(v, 0.16, cx);
        self.glare_alpha.duration(1., 0.40, cx);
        true
    }

    fn pointer_leave(&mut self, reduced: bool, cx: &App) {
        self.hovered = false;
        if reduced {
            self.nx.reset(0.);
            self.ny.reset(0.);
            self.active.reset(0.);
            self.glare_alpha.reset(0.);
            return;
        }
        let now = Instant::now();
        // 700ms underdamped spring back with soft overshoot on pointer leave.
        self.nx.retarget(0., 11.2, 0.56, now);
        self.ny.retarget(0., 11.2, 0.56, now);
        self.active.retarget(0., 11.2, 0.58, now);
        self.gx.duration(0.5, 0.50, cx);
        self.gy.duration(0.5, 0.50, cx);
        self.glare_alpha.duration(0., 0.40, cx);
    }
}

fn paint_card_shadows(
    surface: TiltSurface,
    nx: f32,
    ny: f32,
    shadow_t: f32,
    dark: bool,
    w: &mut Window,
) {
    let sb = surface.projected_bounds();
    let t = shadow_t.clamp(0., 1.);
    let shift_x = -nx * 10. * t;
    let shift_y = (8. + 16. * t) - ny * 7. * t;
    let blur = 24. + 26. * t;
    let spread = -12. - 4. * t;
    let (a_near, a_far) = if dark {
        (0.40, 0.60 + 0.15 * t)
    } else {
        (0.06 - 0.01 * t, 0.18 + 0.12 * t)
    };
    w.paint_drop_shadows(
        sb,
        Corners::all(surface.radius),
        &[
            BoxShadow::new(px(0.), px(1. + t), black().opacity(a_near))
                .blur_radius(px(2. + 2. * t)),
            BoxShadow::new(px(shift_x), px(shift_y), black().opacity(a_far))
                .blur_radius(px(blur))
                .spread_radius(px(spread)),
        ],
    );
}

fn paint_glare(surface: TiltSurface, gx: f32, gy: f32, alpha: f32, dark: bool, w: &mut Window) {
    let alpha = alpha.clamp(0., 1.);
    if alpha <= 0.004 {
        return;
    }
    // Soft-light equivalent peak over saturated card surfaces (0.32 light / 0.24 dark).
    let peak = if dark { 0.165 } else { 0.215 } * alpha;
    let steps = 11usize;
    let aspect = (f32::from(surface.bounds.size.width)
        / f32::from(surface.bounds.size.height).max(1.))
    .clamp(0.6, 2.2);

    for ring in (1..=steps).rev() {
        let frac = ring as f32 / steps as f32;
        let ru = 0.56 * frac;
        let rv = ru * aspect;
        let ring_alpha = peak * (1. - frac * 0.72) / (steps as f32 * 0.52);
        if ring_alpha <= 0.0015 {
            continue;
        }
        surface.fill_polygon(
            (0..28).map(|i| {
                let ang = i as f32 * (2. * PI / 28.);
                surface.clip_rounded(gx + ru * ang.cos(), gy + rv * ang.sin())
            }),
            white().opacity(ring_alpha),
            w,
        );
    }
}

fn paint_projected_label(
    surface: TiltSurface,
    uv: (f32, f32),
    text: SharedString,
    font_px: f32,
    weight: FontWeight,
    color: Hsla,
    w: &mut Window,
) {
    let scaled_font = px(font_px * surface.scale);
    let mut run = w.text_style().to_run(text.len());
    run.font.weight = weight;
    run.color = color;
    let shaped = w.text_system().shape_line(text, scaled_font, &[run], None);
    let top_left = surface.project(uv.0, uv.1);
    let baseline = point(top_left.x, top_left.y + shaped.ascent);
    for s_run in &shaped.runs {
        for glyph in &s_run.glyphs {
            let _ = w.paint_glyph(
                baseline + glyph.position,
                s_run.font_id,
                glyph.id,
                shaped.font_size,
                color,
            );
        }
    }
}

fn paint_default_membership_card(surface: TiltSurface, lang: UiLanguage, w: &mut Window) {
    let cw = f32::from(surface.bounds.size.width).max(1.);
    let ch = f32::from(surface.bounds.size.height).max(1.);
    let sx = (cw / DEFAULT_WIDTH).clamp(0.65, 1.5);

    // Royal blue -> indigo -> violet gradient (`#2563eb` -> `#4f46e5` -> `#7c3aed`).
    surface.fill_polygon(
        surface.outline_uv(),
        linear_gradient(
            135.,
            linear_color_stop(rgb(0x2d6bff), 0.),
            linear_color_stop(rgb(0x794cff), 1.),
        ),
        w,
    );
    // Top-left luminous sheen matching the reference card highlight.
    surface.fill_polygon(
        surface.outline_uv(),
        linear_gradient(
            155.,
            linear_color_stop(hsla(216. / 360., 1., 0.66, 0.16), 0.),
            linear_color_stop(transparent_black(), 1.),
        ),
        w,
    );
    surface.stroke_polygon(surface.outline_uv(), 1.0, hsla(0., 0., 1., 0.18), w);

    // Brand lockup ("KUI Pro") on top-left.
    let pad_u = 24. / DEFAULT_WIDTH;
    let top_v = 22. / DEFAULT_HEIGHT;
    let brand_main: SharedString = "KUI".into();
    let brand_font = 15. * sx;
    paint_projected_label(
        surface,
        (pad_u, top_v),
        brand_main.clone(),
        brand_font,
        FontWeight::SEMIBOLD,
        rgb(0xffffff).into(),
        w,
    );
    let mut measure_run = w.text_style().to_run(brand_main.len());
    measure_run.font.weight = FontWeight::SEMIBOLD;
    let kui_w = f32::from(
        w.text_system()
            .shape_line(brand_main, px(brand_font), &[measure_run], None)
            .width,
    );
    paint_projected_label(
        surface,
        (pad_u + (kui_w + 5. * sx) / cw, top_v),
        "Pro".into(),
        brand_font,
        FontWeight::MEDIUM,
        hsla(0., 0., 1., 0.78),
        w,
    );

    // Contactless wave icon on top-right (`viewBox="0 0 24 24"`, 20x20px at `(296, 22)`).
    let icon_u0 = (DEFAULT_WIDTH - 44.) / DEFAULT_WIDTH;
    let icon_v0 = 22. / DEFAULT_HEIGHT;
    let icon_su = 20. / DEFAULT_WIDTH;
    let icon_sv = 20. / DEFAULT_HEIGHT;
    let arcs: [(f32, f32, f32); 3] = [
        (3.138, 7.0, 0.6982),
        (4.174, 10.5, 0.7297),
        (5.217, 14.0, 0.7458),
    ];
    let mut wave = PathBuilder::stroke(px(1.75 * sx * surface.scale));
    for (cx_svg, r_svg, half_ang) in arcs {
        let segs = 12usize;
        for step in 0..=segs {
            let frac = step as f32 / segs as f32;
            let a = -half_ang + 2. * half_ang * frac;
            let x_svg = cx_svg + r_svg * a.cos();
            let y_svg = 12.0 + r_svg * a.sin();
            let u = icon_u0 + (x_svg / 24.) * icon_su;
            let v = icon_v0 + (y_svg / 24.) * icon_sv;
            let pt = surface.project(u, v);
            if step == 0 {
                wave.move_to(pt);
            } else {
                wave.line_to(pt);
            }
        }
    }
    if let Ok(path) = wave.build() {
        w.paint_path(path, hsla(0., 0., 1., 0.86));
    }

    // Gold metallic EMV chip (`40x30px`, `rad=6px`, at `(24, 78)`).
    let chip_u0 = 24. / DEFAULT_WIDTH;
    let chip_v0 = 78. / DEFAULT_HEIGHT;
    let chip_u1 = 64. / DEFAULT_WIDTH;
    let chip_v1 = 108. / DEFAULT_HEIGHT;
    let chip_ru = 6. / DEFAULT_WIDTH;
    let chip_rv = 6. / DEFAULT_HEIGHT;
    let chip_poly: Vec<Point<f32>> =
        TiltSurface::rounded_rect_uv((chip_u0, chip_v0, chip_u1, chip_v1), (chip_ru, chip_rv), 5)
            .collect();
    surface.fill_polygon(
        chip_poly.iter().copied(),
        linear_gradient(
            135.,
            linear_color_stop(rgb(0xf4df9e), 0.),
            linear_color_stop(rgb(0xc79c49), 1.),
        ),
        w,
    );
    surface.stroke_polygon(chip_poly, 0.9 * sx, hsla(38. / 360., 0.62, 0.24, 0.42), w);

    let mut wires = PathBuilder::stroke(px(0.85 * sx * surface.scale));
    let cu1 = chip_u0 + (chip_u1 - chip_u0) * 0.335;
    let cu2 = chip_u0 + (chip_u1 - chip_u0) * 0.665;
    let cv1 = chip_v0 + (chip_v1 - chip_v0) * 0.34;
    let cv2 = chip_v0 + (chip_v1 - chip_v0) * 0.66;
    for &(u_a, v_a, u_b, v_b) in &[
        (cu1, chip_v0 + 0.004, cu1, chip_v1 - 0.004),
        (cu2, chip_v0 + 0.004, cu2, chip_v1 - 0.004),
        (chip_u0 + 0.003, cv1, cu1, cv1),
        (chip_u0 + 0.003, cv2, cu1, cv2),
        (cu2, cv1, chip_u1 - 0.003, cv1),
        (cu2, cv2, chip_u1 - 0.003, cv2),
    ] {
        wires.move_to(surface.project(u_a, v_a));
        wires.line_to(surface.project(u_b, v_b));
    }
    if let Ok(path) = wires.build() {
        w.paint_path(path, hsla(38. / 360., 0.58, 0.22, 0.36));
    }

    // Masked card number (`•••• •••• •••• 4821` at `(24, 122)`).
    paint_projected_label(
        surface,
        (pad_u, 122. / DEFAULT_HEIGHT),
        "••••  ••••  ••••  4821".into(),
        16.5 * sx,
        FontWeight::MEDIUM,
        hsla(0., 0., 1., 0.95),
        w,
    );

    // Bottom metadata columns (`MEMBER / Alex Morgan` and `SINCE / 10/26`).
    let label_v = 155. / DEFAULT_HEIGHT;
    let value_v = 173. / DEFAULT_HEIGHT;
    let since_u = 130. / DEFAULT_WIDTH;

    paint_projected_label(
        surface,
        (pad_u, label_v),
        lang.text("MEMBER", "持卡会员").into(),
        10. * sx,
        FontWeight::MEDIUM,
        hsla(0., 0., 1., 0.62),
        w,
    );
    paint_projected_label(
        surface,
        (pad_u, value_v),
        "Alex Morgan".into(),
        13. * sx,
        FontWeight::MEDIUM,
        rgb(0xffffff).into(),
        w,
    );

    paint_projected_label(
        surface,
        (since_u, label_v),
        lang.text("SINCE", "加入时间").into(),
        10. * sx,
        FontWeight::MEDIUM,
        hsla(0., 0., 1., 0.62),
        w,
    );
    paint_projected_label(
        surface,
        (since_u, value_v),
        "10/26".into(),
        13. * sx,
        FontWeight::MEDIUM,
        rgb(0xffffff).into(),
        w,
    );

    let _ = ch;
}

/// Three-dimensional pointer-tilting showcase card with radial specular glare and spring recovery.
///
/// Renders the KUI Pro membership card by default, or wraps arbitrary GPUI elements
/// supplied via [`Self::with_child`] or [`Self::with_content`].
///
/// # Examples
///
/// ```
/// use gpui::{prelude::*, px};
/// use kui::components::tilt_card::TiltCard;
///
/// fn view(cx: &gpui::App) -> TiltCard {
///     TiltCard::new("member-card", cx)
///         .with_max_tilt(10.0)
///         .with_scale(1.02)
///         .with_glare(true)
///         .with_perspective(900.0)
///         .with_radius(px(20.))
/// }
/// ```
#[derive(IntoElement)]
pub struct TiltCard {
    id: ElementId,
    max_tilt: f32,
    glare: bool,
    scale: f32,
    perspective: f32,
    radius: Pixels,
    width: Pixels,
    height: Pixels,
    focus: FocusHandle,
    once_child: RefCell<Option<AnyElement>>,
    content_builder: Option<ContentBuilder>,
}

impl TiltCard {
    /// Construct a `TiltCard` with default `10°` max tilt, `1.02` hover scale, `900px`
    /// perspective, `20px` corner radius, and the KUI Pro membership card face.
    pub fn new(id: impl Into<ElementId>, cx: &App) -> Self {
        Self {
            id: id.into(),
            max_tilt: DEFAULT_MAX_TILT,
            glare: true,
            scale: DEFAULT_SCALE,
            perspective: DEFAULT_PERSPECTIVE,
            radius: px(DEFAULT_RADIUS),
            width: px(DEFAULT_WIDTH),
            height: px(DEFAULT_HEIGHT),
            focus: cx.focus_handle().tab_stop(false),
            once_child: RefCell::new(None),
            content_builder: None,
        }
    }

    /// Construct the interactive KUI Pro membership card showcase with caption.
    pub fn demo(id: impl Into<ElementId>, cx: &mut Context<TiltCardShowcase>) -> TiltCardShowcase {
        TiltCardShowcase::new(id, cx)
    }

    /// Set maximum tilt angle in degrees along each axis (`10.0` default).
    pub fn with_max_tilt(mut self, degrees: f32) -> Self {
        if degrees.is_finite() {
            self.max_tilt = degrees.clamp(0., 35.);
        }
        self
    }

    /// Enable or disable the pointer-following soft glare overlay (`true` default).
    pub fn with_glare(mut self, glare: bool) -> Self {
        self.glare = glare;
        self
    }

    /// Set hover scale factor while the pointer is over the card (`1.02` default).
    pub fn with_scale(mut self, scale: f32) -> Self {
        if scale.is_finite() {
            self.scale = scale.clamp(0.90, 1.25);
        }
        self
    }

    /// Set 3D camera perspective distance in pixels (`900.0` default; smaller is more dramatic).
    pub fn with_perspective(mut self, perspective_px: f32) -> Self {
        if perspective_px.is_finite() {
            self.perspective = perspective_px.clamp(220., 4000.);
        }
        self
    }

    /// Set corner radius for the card surface, shadow, and glare mask (`px(20.)` default).
    pub fn with_radius(mut self, radius: Pixels) -> Self {
        if f32::from(radius).is_finite() && radius >= px(0.) {
            self.radius = radius;
        }
        self
    }

    /// Override explicit card width and height (`340x214` default).
    pub fn with_size(mut self, width: Pixels, height: Pixels) -> Self {
        if f32::from(width).is_finite() && width > px(20.) {
            self.width = width;
        }
        if f32::from(height).is_finite() && height > px(20.) {
            self.height = height;
        }
        self
    }

    /// Override card width (`px(340.)` default).
    pub fn with_width(mut self, width: Pixels) -> Self {
        if f32::from(width).is_finite() && width > px(20.) {
            self.width = width;
        }
        self
    }

    /// Override card height (`px(214.)` default).
    pub fn with_height(mut self, height: Pixels) -> Self {
        if f32::from(height).is_finite() && height > px(20.) {
            self.height = height;
        }
        self
    }

    /// Mount custom child content inside an inline `TiltCard` element.
    pub fn with_child(self, child: impl IntoElement + 'static) -> Self {
        *self.once_child.borrow_mut() = Some(child.into_any_element());
        self
    }

    /// Mount a repeatable custom content builder inside the tilting card surface.
    pub fn with_content(
        mut self,
        render: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        self.content_builder = Some(Rc::new(render));
        self
    }

    /// Configured maximum tilt angle in degrees.
    pub fn max_tilt(&self) -> f32 {
        self.max_tilt
    }

    /// Whether the pointer-tracking glare layer is enabled.
    pub fn has_glare(&self) -> bool {
        self.glare
    }

    /// Configured hover scale factor.
    pub fn scale(&self) -> f32 {
        self.scale
    }

    /// Configured perspective distance in pixels.
    pub fn perspective(&self) -> f32 {
        self.perspective
    }

    fn render_card(
        &self,
        live_entity: Entity<TiltLiveState>,
        custom_el: Option<AnyElement>,
        w: &mut Window,
        cx: &mut App,
    ) -> Stateful<Div> {
        let theme = Theme::global(cx);
        let dark = theme.mode == ThemeMode::Dark;
        let reduced = disabled_motion(cx);
        let lang = UiLanguage::current(cx);
        let now = Instant::now();

        let has_focus = self.focus.contains_focused(w, cx);
        let (nx, ny, active_t, gx, gy, glare_a, hovered, bounds_cell, animating) = live_entity
            .update(cx, |s, cx| {
                let (nx_v, _, nx_mov) = s.nx.sample(now);
                let (ny_v, _, ny_mov) = s.ny.sample(now);
                let (act_v, _, act_mov) = s.active.sample(now);
                let gx_v = s.gx.get(w, cx);
                let gy_v = s.gy.get(w, cx);
                let gl_v = s.glare_alpha.get(w, cx);
                (
                    if reduced { 0. } else { nx_v },
                    if reduced { 0. } else { ny_v },
                    act_v.clamp(0., 1.3),
                    gx_v.clamp(0., 1.),
                    gy_v.clamp(0., 1.),
                    if reduced { 0. } else { gl_v.clamp(0., 1.) },
                    s.hovered,
                    s.bounds.clone(),
                    !reduced && (nx_mov || ny_mov || act_mov),
                )
            });

        if animating {
            w.request_animation_frame();
        }

        let rx_rad = if reduced {
            0.
        } else {
            (-ny * self.max_tilt).to_radians()
        };
        let ry_rad = if reduced {
            0.
        } else {
            (nx * self.max_tilt).to_radians()
        };
        let cur_scale = if reduced {
            1.0
        } else {
            1.0 + (self.scale - 1.0) * active_t
        };
        let lift_px = if reduced {
            0.0
        } else if has_focus {
            4.0
        } else {
            2.0 * active_t
        };
        let shadow_t = if has_focus || (reduced && hovered) {
            1.0
        } else {
            active_t.clamp(0., 1.)
        };

        let radius = self.radius;
        let perspective = self.perspective;
        let show_glare = self.glare && !reduced;
        let is_default_card = custom_el.is_none();
        let panel_bg: Hsla = if dark {
            rgb(0x232326).into()
        } else {
            rgb(0xffffff).into()
        };

        let make_surface = move |b: Bounds<Pixels>| {
            TiltSurface::new(b, radius, rx_rad, ry_rad, cur_scale, lift_px, perspective)
        };

        let base_canvas = canvas(
            move |b, _, _| {
                bounds_cell.set(b);
            },
            move |b, _, window, _| {
                if b.size.width <= px(2.) || b.size.height <= px(2.) {
                    return;
                }
                let surface = make_surface(b);
                paint_card_shadows(surface, nx, ny, shadow_t, dark, window);
                if is_default_card {
                    paint_default_membership_card(surface, lang, window);
                } else {
                    surface.fill_polygon(surface.outline_uv(), panel_bg, window);
                }
            },
        )
        .absolute()
        .inset_0();

        let move_entity = live_entity.clone();
        let hover_entity = live_entity;

        let mut root = div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .relative()
            .w(self.width)
            .max_w_full()
            .h(self.height)
            .flex_none()
            .on_mouse_move(move |ev: &MouseMoveEvent, window, cx| {
                if window.last_input_was_touch() {
                    return;
                }
                let reduced = disabled_motion(cx);
                let changed =
                    move_entity.update(cx, |s, cx| s.pointer_move(ev.position, reduced, cx));
                if changed {
                    window.refresh();
                }
            })
            .on_hover(move |over: &bool, window, cx| {
                if !*over {
                    let reduced = disabled_motion(cx);
                    hover_entity.update(cx, |s, cx| s.pointer_leave(reduced, cx));
                    window.refresh();
                }
            })
            .child(base_canvas);

        if let Some(child_el) = custom_el {
            let parallax_x = if reduced { 0. } else { nx * 3.5 };
            let parallax_y = if reduced { 0. } else { ny * 2.5 - lift_px };
            root = root.child(
                div()
                    .absolute()
                    .inset_0()
                    .left(px(parallax_x))
                    .top(px(parallax_y))
                    .rounded(radius)
                    .overflow_hidden()
                    .child(child_el),
            );
        }

        if show_glare && glare_a > 0.004 {
            root = root.child(
                canvas(
                    |_, _, _| (),
                    move |b, _, window, _| {
                        if b.size.width <= px(2.) || b.size.height <= px(2.) {
                            return;
                        }
                        let surface = make_surface(b);
                        paint_glare(surface, gx, gy, glare_a, dark, window);
                    },
                )
                .absolute()
                .inset_0(),
            );
        }

        root
    }
}

impl Render for TiltCard {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state_id: ElementId = (self.id.clone(), "tc-entity-state").into();
        let live_entity = w.use_keyed_state(state_id, cx, |_, _| TiltLiveState::new());
        let custom_el = self
            .content_builder
            .as_ref()
            .map(|builder| builder(w, cx))
            .or_else(|| self.once_child.borrow_mut().take());
        self.render_card(live_entity, custom_el, w, cx)
    }
}

impl RenderOnce for TiltCard {
    fn render(self, w: &mut Window, cx: &mut App) -> impl IntoElement {
        let state_id: ElementId = (self.id.clone(), "tc-once-state").into();
        let live_entity = w.use_keyed_state(state_id, cx, |_, _| TiltLiveState::new());
        let custom_el = self
            .once_child
            .borrow_mut()
            .take()
            .or_else(|| self.content_builder.as_ref().map(|builder| builder(w, cx)));
        self.render_card(live_entity, custom_el, w, cx)
    }
}

/// Interactive showcase pairing the 3D KUI Pro membership [`TiltCard`] with its pointer hint caption.
pub struct TiltCardShowcase {
    id: ElementId,
    card: Entity<TiltCard>,
}

impl TiltCardShowcase {
    /// Construct a new `TiltCardShowcase` subscribed to [`Theme`] updates.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        let id = id.into();
        let card_id: ElementId = (id.clone(), "membership-card").into();
        let card = cx.new(|cx| {
            cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
            TiltCard::new(card_id, cx)
                .with_max_tilt(10.0)
                .with_scale(1.02)
                .with_glare(true)
                .with_perspective(900.0)
                .with_radius(px(20.))
        });
        Self { id, card }
    }

    /// Borrow the underlying `TiltCard` entity.
    pub fn card(&self) -> &Entity<TiltCard> {
        &self.card
    }
}

impl Render for TiltCardShowcase {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let lang = UiLanguage::current(cx);
        let caption_fg: Hsla = rgb(0x8f8f8f).into();

        div()
            .id(self.id.clone())
            .w_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .px(px(24.))
            .py(px(32.))
            .child(self.card.clone())
            .child(
                div()
                    .mt(px(28.))
                    .text_size(px(13.))
                    .line_height(px(18.))
                    .text_color(caption_fg)
                    .child(lang.text(
                        "Move your pointer over the card",
                        "将指针移到卡片上方体验 3D 倾斜",
                    )),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::{TiltCard, TiltCardShowcase, TiltSpring, TiltSurface};
    use gpui::{Bounds, TestAppContext, point, px, size};
    use std::time::Duration;
    use web_time::Instant;

    #[test]
    fn release_spring_overshoots_gently_and_settles_to_rest() {
        let t0 = Instant::now();
        let mut spring = TiltSpring::new(1.0);
        spring.retarget(0.0, 11.2, 0.56, t0);

        let mut min_seen = 1.0f32;
        for step_ms in (20..=700).step_by(20) {
            let (val, _, _) = spring.sample(t0 + Duration::from_millis(step_ms));
            min_seen = min_seen.min(val);
        }
        assert!(
            min_seen < -0.02 && min_seen > -0.25,
            "expected soft underdamped overshoot past 0.0, got min_seen={min_seen}"
        );

        let (settled, vel, active) = spring.sample(t0 + Duration::from_millis(1500));
        assert_eq!(settled, 0.0);
        assert_eq!(vel, 0.0);
        assert!(!active);
    }

    #[test]
    fn perspective_projection_preserves_rest_frame_and_tilts_corners() {
        let bounds = Bounds::new(point(px(20.), px(30.)), size(px(340.), px(214.)));
        let rest = TiltSurface::new(bounds, px(20.), 0., 0., 1.0, 0., 900.);
        assert_eq!(rest.project(0., 0.), bounds.origin);
        assert_eq!(rest.project(1., 1.), bounds.bottom_right());

        let tilted = TiltSurface::new(
            bounds,
            px(20.),
            (-10.0f32).to_radians(),
            10.0f32.to_radians(),
            1.02,
            2.0,
            900.,
        );
        let tl = tilted.project(0., 0.);
        let br = tilted.project(1., 1.);
        assert!(
            tl.y < bounds.origin.y,
            "lifted + tilted top-left moves upward"
        );
        assert!(br.x > tl.x);
    }

    #[test]
    fn tilt_card_and_showcase_configure_and_mount_cleanly() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        app.update(|cx| {
            let card = TiltCard::new("tc-unit", cx)
                .with_max_tilt(12.)
                .with_scale(1.04)
                .with_glare(false)
                .with_perspective(750.);
            assert_eq!(card.max_tilt(), 12.);
            assert_eq!(card.scale(), 1.04);
            assert!(!card.has_glare());
            assert_eq!(card.perspective(), 750.);
        });

        let (view, window) = app.add_window_view(|_, cx| TiltCardShowcase::new("tc-demo", cx));
        window.run_until_parked();
        assert_eq!(
            window.update(|_, cx| view.read(cx).card().read(cx).max_tilt()),
            10.0
        );
    }
}
