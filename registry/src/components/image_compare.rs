//! Compare two host-supplied images using a keyboard and pointer divider.
use crate::foundation::{Theme, ThemeMode};
use crate::primitives::family::{Paint, Slide, measure};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CompareOrientation {
    #[default]
    Horizontal,
    Vertical,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CompareChanged(pub f32);

/// Touch gate: a sideways drag claims the divider, otherwise the page scrolls.
#[derive(Clone, Copy, Default)]
struct TouchGate {
    active: bool,
    dragging: bool,
    /// `Some(true)` locked to the horizontal axis, `Some(false)` vertical.
    axis: Option<bool>,
    origin: (f32, f32),
}

pub struct ImageCompare {
    before: ImageSource,
    after: ImageSource,
    label: SharedString,
    captions: Option<[SharedString; 2]>,
    orientation: CompareOrientation,
    ratio: f32,
    value: f32,
    position: Slide,
    handle_w: Slide,
    handle_h: Slide,
    /// 0 horizontal, 1 vertical; animates the divider swing between them.
    swing: Slide,
    /// 1 -> 0 poke shown when a key presses against an edge.
    bump: Slide,
    bump_dir: f32,
    bounds: Rc<Cell<Bounds<Pixels>>>,
    focus: FocusHandle,
    container_width: Rc<Cell<Pixels>>,
    dragging: bool,
    hovered: bool,
    /// Recent pointer samples used to settle the divider with release speed.
    prev_move: Option<(f32, Instant)>,
    last_move: Option<(f32, Instant)>,
    touch: Rc<Cell<TouchGate>>,
}
impl EventEmitter<CompareChanged> for ImageCompare {}
impl ImageCompare {
    pub fn new(
        before: impl Into<ImageSource>,
        after: impl Into<ImageSource>,
        label: impl Into<SharedString>,
        cx: &App,
    ) -> Self {
        Self {
            before: before.into(),
            after: after.into(),
            label: label.into(),
            captions: None,
            orientation: CompareOrientation::Horizontal,
            ratio: 1.5,
            value: 0.5,
            position: Slide::new(0.5),
            handle_w: Slide::new(40.),
            handle_h: Slide::new(40.),
            swing: Slide::new(0.),
            bump: Slide::new(0.),
            bump_dir: 1.,
            bounds: Default::default(),
            focus: cx.focus_handle().tab_stop(true),
            container_width: Default::default(),
            dragging: false,
            hovered: false,
            prev_move: None,
            last_move: None,
            touch: Default::default(),
        }
    }
    pub fn with_position(mut self, value: f32) -> Self {
        self.value = if value.is_finite() {
            value.clamp(0., 1.)
        } else {
            0.5
        };
        self.position.reset(self.value);
        self
    }
    pub fn with_ratio(mut self, width_over_height: f32) -> Self {
        if width_over_height.is_finite() {
            self.ratio = width_over_height.clamp(0.5, 3.);
        }
        self
    }
    pub fn with_orientation(mut self, orientation: CompareOrientation) -> Self {
        self.orientation = orientation;
        self.swing
            .reset(if orientation == CompareOrientation::Vertical {
                1.
            } else {
                0.
            });
        self
    }
    pub fn orientation(&self) -> CompareOrientation {
        self.orientation
    }
    /// Swap axes; the divider swings around the frame centre while travelling.
    pub fn set_orientation(&mut self, orientation: CompareOrientation, cx: &mut Context<Self>) {
        if self.orientation == orientation {
            return;
        }
        self.orientation = orientation;
        self.swing.duration(
            if orientation == CompareOrientation::Vertical {
                1.
            } else {
                0.
            },
            0.45,
            cx,
        );
        cx.notify();
    }
    pub fn with_captions(
        mut self,
        before: impl Into<SharedString>,
        after: impl Into<SharedString>,
    ) -> Self {
        self.captions = Some([before.into(), after.into()]);
        self
    }
    pub fn position(&self) -> f32 {
        self.value
    }
    pub fn set_position(&mut self, value: f32, cx: &mut Context<Self>) {
        if !value.is_finite() {
            return;
        }
        let value = value.clamp(0., 1.);
        if (self.value - value).abs() < 0.0001 {
            return;
        }
        self.value = value;
        if self.dragging {
            self.position.reset(value);
        } else {
            self.position.duration(value, 0.26, cx);
        }
        cx.emit(CompareChanged(value));
        cx.notify();
    }
    fn pointer_value(&self, p: Point<Pixels>) -> f32 {
        let b = self.bounds.get();
        match self.orientation {
            CompareOrientation::Horizontal => (p.x - b.left()) / b.size.width.max(px(1.)),
            CompareOrientation::Vertical => (p.y - b.top()) / b.size.height.max(px(1.)),
        }
        .clamp(0., 1.)
    }
    /// Record pointer motion while dragging so release keeps the hand's speed.
    fn track_move(&mut self, value: f32, cx: &App) {
        let now = cx.background_executor().now();
        if let Some((_, at)) = self.last_move
            && now.saturating_duration_since(at).as_secs_f32() > 0.25
        {
            self.prev_move = None;
            self.last_move = None;
        }
        self.prev_move = self.last_move.take();
        self.last_move = Some((value, now));
    }
    /// Continue the divider with the measured release velocity, in value units per second.
    fn settle(&mut self, cx: &mut Context<Self>) {
        let mut velocity = 0.;
        if let (Some((v0, t0)), Some((v1, t1))) = (self.prev_move, self.last_move) {
            let dt = t1.saturating_duration_since(t0).as_secs_f32();
            if dt > 0.004
                && cx
                    .background_executor()
                    .now()
                    .saturating_duration_since(t1)
                    .as_secs_f32()
                    < 0.15
            {
                velocity = ((v1 - v0) / dt).clamp(-4., 4.);
            }
        }
        self.prev_move = None;
        self.last_move = None;
        self.position.release(self.value, velocity, 0.3, cx);
    }
    /// Nudge the handle when a key keeps pressing past an edge.
    fn edge_bump(&mut self, dir: f32, cx: &mut Context<Self>) {
        self.bump.reset(1.);
        self.bump.duration(0., 0.24, cx);
        self.bump_dir = dir;
        cx.notify();
    }
}

/// Half length of the frame chord through the centre along a unit axis.
fn chord_half(width: f32, height: f32, dx: f32, dy: f32) -> f32 {
    let tx = if dx.abs() > 1e-4 {
        (width * 0.5) / dx.abs()
    } else {
        f32::INFINITY
    };
    let ty = if dy.abs() > 1e-4 {
        (height * 0.5) / dy.abs()
    } else {
        f32::INFINITY
    };
    tx.min(ty)
}

/// Chevron polylines in handle-local coordinates around the centre: `‹ ›`.
const CHEVRONS: [[(f32, f32); 3]; 2] = [
    [(-1., 4.), (-5., 0.), (-1., -4.)],
    [(1., 4.), (5., 0.), (1., -4.)],
];

// Measure the available parent width independently of the component's inner layout.
fn constrained(
    content: impl IntoElement,
    width: Rc<Cell<Pixels>>,
    maximum: Pixels,
) -> impl IntoElement {
    div()
        .relative()
        .w_full()
        .max_w(maximum)
        .child(
            canvas(
                move |bounds, w, _| {
                    if (width.get() - bounds.size.width).abs() > px(0.5) {
                        width.set(bounds.size.width);
                        w.request_animation_frame();
                    }
                },
                |_, _, _, _| (),
            )
            .absolute()
            .inset_0(),
        )
        .child(content)
}
impl Render for ImageCompare {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let dark = Theme::global(cx).mode == ThemeMode::Dark;
        let measured = self.container_width.get();
        let width = if measured > px(0.) {
            measured
        } else {
            w.viewport_size().width - px(32.)
        }
        .max(px(1.));
        let height = width / self.ratio;
        let fraction = self.position.get(w, cx).clamp(0., 1.);
        let vertical = self.orientation == CompareOrientation::Vertical;
        let img_radius = crate::foundation::surface_radius(cx);
        let old = img(self.before.clone())
            .w(width)
            .h(height)
            .object_fit(ObjectFit::Cover);
        let new = img(self.after.clone())
            .w(width)
            .h(height)
            .object_fit(ObjectFit::Cover);
        let clip = div()
            .absolute()
            .top_0()
            .left_0()
            .overflow_hidden()
            .when(vertical, |d| d.w_full().h(relative(fraction)))
            .when(!vertical, |d| d.h_full().w(relative(fraction)))
            .child(old);

        // Swing geometry: the divider axis rotates from horizontal to vertical
        // around the frame centre while the handle travels along the arc.
        let theta = self.swing.get(w, cx) * std::f32::consts::FRAC_PI_2;
        let (dir_x, dir_y) = (theta.cos(), theta.sin());
        let (perp_x, perp_y) = (-theta.sin(), theta.cos());
        let width_px = f32::from(width);
        let height_px = f32::from(height);
        let half = chord_half(width_px, height_px, dir_x, dir_y);
        let radius = (fraction - 0.5) * 2. * half;
        let pivot_x = width_px * 0.5 + dir_x * radius;
        let pivot_y = height_px * 0.5 + dir_y * radius;
        let bump = self.bump.get(w, cx);
        let bump_off = 6. * bump * self.bump_dir;
        let handle_x = pivot_x + dir_x * bump_off;
        let handle_y = pivot_y + dir_y * bump_off;

        // Reference line: raised surface in light, foreground in dark, with a
        // 0.5px dark halo drawn as a wider stroke behind it.
        let line_color = if dark { p.ink } else { p.tray };
        let halo = black().opacity(0.14);
        let line_pivot = (pivot_x, pivot_y);
        let overscan = width_px + height_px;
        let line = canvas(
            |_, _, _| (),
            move |b, _, w, _| {
                let o = b.origin;
                let (px0, py0) = line_pivot;
                let ax = o.x + px(px0 - perp_x * overscan);
                let ay = o.y + px(py0 - perp_y * overscan);
                let bx = o.x + px(px0 + perp_x * overscan);
                let by = o.y + px(py0 + perp_y * overscan);
                for (stroke, color) in [(3., halo), (2., line_color)] {
                    let mut path = PathBuilder::stroke(px(stroke));
                    path.move_to(point(ax, ay));
                    path.line_to(point(bx, by));
                    if let Ok(path) = path.build() {
                        w.paint_path(path, color);
                    }
                }
            },
        )
        .absolute()
        .inset_0();

        let (target_w, target_h) = if self.dragging {
            if vertical { (36., 56.) } else { (56., 36.) }
        } else {
            (40., 40.)
        };
        self.handle_w.duration(target_w, 0.42, cx);
        self.handle_h.duration(target_h, 0.42, cx);
        let handle_w = self.handle_w.get(w, cx);
        let handle_h = self.handle_h.get(w, cx);
        let chevron_color = p.ink;
        let thumb = div()
            .absolute()
            .w(px(handle_w))
            .h(px(handle_h))
            .left(px(handle_x - handle_w * 0.5))
            .top(px(handle_y - handle_h * 0.5))
            .rounded_full()
            .border_1()
            .border_color(if self.hovered { p.faint } else { p.line })
            .bg(p.tray)
            .shadow(vec![gpui::BoxShadow {
                color: black().opacity(0.22),
                offset: point(px(0.), px(8.)),
                blur_radius: px(22.),
                spread_radius: px(0.),
                inset: false,
            }])
            .child(
                canvas(
                    |_, _, _| (),
                    move |b, _, w, _| {
                        let c = point(
                            b.origin.x + px(handle_w * 0.5),
                            b.origin.y + px(handle_h * 0.5),
                        );
                        let (s, co) = theta.sin_cos();
                        for tri in CHEVRONS {
                            let mut path = PathBuilder::stroke(px(1.75));
                            let mut first = true;
                            for (x, y) in tri {
                                let pt = point(c.x + px(x * co - y * s), c.y + px(x * s + y * co));
                                if first {
                                    path.move_to(pt);
                                    first = false;
                                } else {
                                    path.line_to(pt);
                                }
                            }
                            if let Ok(path) = path.build() {
                                w.paint_path(path, chevron_color);
                            }
                        }
                    },
                )
                .absolute()
                .inset_0(),
            );

        let bounds = self.bounds.clone();
        let moved = cx.listener(|s, e: &MouseMoveEvent, _, cx| {
            if s.dragging {
                let value = s.pointer_value(e.position);
                s.track_move(value, cx);
                s.set_position(value, cx);
            }
        });
        let released = cx.listener(|s, _: &MouseUpEvent, _, cx| {
            if s.dragging {
                s.dragging = false;
                s.settle(cx);
                cx.notify();
            }
        });
        let touch_gate = self.touch.clone();
        let touch_state = self.touch.clone();
        let touch = cx.listener(move |s, e: &TouchDragEvent, w, cx| {
            let mut gate = touch_state.get();
            match e.phase {
                TouchPhase::Started => {
                    s.dragging = false;
                    gate.axis = None;
                    gate.origin = (f32::from(e.position.x), f32::from(e.position.y));
                    s.focus.focus(w, cx);
                }
                TouchPhase::Moved => {
                    if gate.axis.is_none() {
                        let (dx, dy) = (
                            f32::from(e.position.x) - gate.origin.0,
                            f32::from(e.position.y) - gate.origin.1,
                        );
                        if dx.abs() < 6. && dy.abs() < 6. {
                            touch_state.set(gate);
                            return;
                        }
                        gate.axis = Some(dx.abs() > dy.abs());
                    }
                    // A swipe along the divider's cross axis scrolls the page.
                    let travels_horizontally = s.orientation == CompareOrientation::Horizontal;
                    if gate.axis != Some(travels_horizontally) {
                        gate.dragging = false;
                        touch_state.set(gate);
                        return;
                    }
                    gate.dragging = true;
                    s.dragging = true;
                    let value = s.pointer_value(e.position);
                    s.track_move(value, cx);
                    s.set_position(value, cx);
                }
                TouchPhase::Ended | TouchPhase::Cancelled => {
                    if gate.dragging {
                        s.dragging = false;
                        s.settle(cx);
                    } else if gate.axis.is_none() && e.phase == TouchPhase::Ended {
                        // A tap without a sideways drag jumps the divider there.
                        s.focus.focus(w, cx);
                        s.set_position(s.pointer_value(e.position), cx);
                    }
                    gate = TouchGate::default();
                }
            }
            touch_state.set(gate);
        });
        let mask_color = p.stage;
        let contact = canvas(
            |b, w, _| w.insert_hitbox(b, HitboxBehavior::Normal),
            move |_, hit, w, _| {
                let touch_gate = touch_gate.clone();
                w.on_mouse_event(move |e: &TouchDragEvent, phase, w, cx| {
                    if phase.bubble() {
                        let mut gate = touch_gate.get();
                        if e.phase == TouchPhase::Started {
                            gate.active = hit.is_hovered(w);
                            if gate.active {
                                w.prevent_default();
                            }
                        }
                        if !gate.active {
                            return;
                        }
                        if matches!(e.phase, TouchPhase::Ended | TouchPhase::Cancelled) {
                            gate.active = false;
                            touch_gate.set(gate);
                        }
                        touch(e, w, cx);
                        cx.stop_propagation();
                    }
                });
            },
        )
        .absolute()
        .inset_0();
        let mut frame = div()
            .relative()
            .w(width)
            .h(height)
            .overflow_hidden()
            .bg(p.stage)
            .rounded(crate::foundation::surface_radius(cx))
            .on_children_prepainted(move |bs, _, _| {
                if let Some(b) = bs.first() {
                    bounds.set(*b);
                }
            })
            .id("image-compare")
            .role(Role::Slider)
            .aria_label(self.label.clone())
            .aria_value(format!("{}% after", (100. - self.value * 100.).round()))
            .track_focus(&self.focus)
            .tab_index(0)
            .when(vertical, |d| d.cursor_ns_resize())
            .when(!vertical, |d| d.cursor_ew_resize())
            .on_hover(cx.listener(|s, over: &bool, _, cx| {
                if s.hovered != *over {
                    s.hovered = *over;
                    cx.notify();
                }
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|s, e: &MouseDownEvent, w, cx| {
                    if e.click_count == 2 && (s.pointer_value(e.position) - s.value).abs() < 0.08 {
                        s.dragging = false;
                        s.set_position(0.5, cx);
                        return;
                    }
                    s.dragging = true;
                    s.prev_move = None;
                    s.last_move = None;
                    s.focus.focus(w, cx);
                    let value = s.pointer_value(e.position);
                    s.track_move(value, cx);
                    s.set_position(value, cx);
                }),
            )
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, _, cx| {
                let step = if e.keystroke.modifiers.shift {
                    0.1
                } else {
                    0.01
                };
                let vertical = s.orientation == CompareOrientation::Vertical;
                // Each orientation answers only its own arrows.
                let value = match e.keystroke.key.as_str() {
                    "left" if !vertical => s.value - step,
                    "right" if !vertical => s.value + step,
                    "up" if vertical => s.value - step,
                    "down" if vertical => s.value + step,
                    "pageup" => s.value - 0.1,
                    "pagedown" => s.value + 0.1,
                    "home" => 1.,
                    "end" => 0.,
                    _ => return,
                };
                let clamped = value.clamp(0., 1.);
                if (clamped - s.value).abs() < 0.0001 {
                    // Already at the edge: show the little bump instead.
                    let dir = if value > s.value { 1. } else { -1. };
                    s.edge_bump(dir, cx);
                } else {
                    s.set_position(clamped, cx);
                }
                cx.stop_propagation();
            }))
            .child(new)
            .child(clip)
            .child(line)
            .child(thumb)
            .child(
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
            .child(contact)
            .child(
                div()
                    .absolute()
                    .inset_0()
                    .rounded(crate::foundation::surface_radius(cx))
                    .border_1()
                    .border_color(if self.focus.is_focused(w) {
                        p.bright.opacity(0.55)
                    } else {
                        p.ink.opacity(0.08)
                    }),
            )
            .child(
                // Reference overflow clip: the frame background shows through
                // outside each corner arc, above every layer including chips.
                canvas(
                    |_, _, _| (),
                    move |b, _, w, _| {
                        let o = b.origin;
                        let r = f32::from(img_radius);
                        for (sx, sy) in [(-1., -1.), (1., -1.), (-1., 1.), (1., 1.)] {
                            let vx = if sx < 0. { 0. } else { width_px };
                            let vy = if sy < 0. { 0. } else { height_px };
                            let (cx, cy) = (vx - sx * r, vy - sy * r);
                            let mut pts = vec![point(o.x + px(vx), o.y + px(vy))];
                            for i in 0..=8 {
                                let t = i as f32 / 8.;
                                let dy = (1. - t) * r * sy;
                                let dx = (r * r - dy * dy).sqrt() * sx;
                                pts.push(point(o.x + px(cx + dx), o.y + px(cy + dy)));
                            }
                            let mut path = PathBuilder::fill();
                            path.add_polygon(&pts, true);
                            if let Ok(path) = path.build() {
                                w.paint_path(path, mask_color);
                            }
                        }
                    },
                )
                .absolute()
                .inset_0(),
            );
        if let Some(labels) = &self.captions {
            let before_w = measure(&labels[0], 12., w) + 20.;
            let after_w = measure(&labels[1], 12., w) + 20.;
            let span = if vertical { height_px } else { width_px };
            let before_room = fraction * span - (12. + before_w + 8.);
            let after_room = span - 12. - after_w - 8. - fraction * span;
            let chip = |label: SharedString, opacity: f32| {
                div()
                    .absolute()
                    .px(px(10.))
                    .py(px(3.))
                    .rounded_full()
                    .bg(p.panel.opacity(0.74))
                    .text_size(px(12.))
                    .line_height(px(17.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.ink)
                    .opacity(opacity)
                    .child(label)
            };
            frame = frame
                .child(
                    chip(labels[0].clone(), (before_room / 28.).clamp(0., 1.))
                        .top(px(12.))
                        .left(px(12.)),
                )
                .child(
                    chip(labels[1].clone(), (after_room / 28.).clamp(0., 1.))
                        .when(vertical, |d| d.bottom(px(12.)).left(px(12.)))
                        .when(!vertical, |d| d.top(px(12.)).right(px(12.))),
                );
        }
        constrained(frame, self.container_width.clone(), px(4096.))
    }
}

#[cfg(test)]
mod tests {
    use super::{CHEVRONS, chord_half};

    #[test]
    fn chord_reaches_the_frame_edge_on_both_axes() {
        assert!((chord_half(600., 400., 1., 0.) - 300.).abs() < 0.001);
        assert!((chord_half(600., 400., 0., 1.) - 200.).abs() < 0.001);
        // A 45-degree chord meets the nearer edge.
        let d = 2f32.sqrt() / 2.;
        assert!((chord_half(600., 400., d, d) - 200. / d).abs() < 0.001);
    }

    #[test]
    fn chevrons_stay_symmetric_under_a_quarter_turn() {
        let (s, c) = (
            std::f32::consts::FRAC_PI_2.sin(),
            std::f32::consts::FRAC_PI_2.cos(),
        );
        for tri in CHEVRONS {
            for (x, y) in tri {
                // After the swing the pair still spans the same width, upright.
                let (rx, ry) = (x * c - y * s, x * s + y * c);
                assert!((rx.abs() - y.abs()).abs() < 0.001);
                assert!((ry.abs() - x.abs()).abs() < 0.001);
            }
        }
    }
}
