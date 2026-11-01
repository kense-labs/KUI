//! HSV picker. Pointer, keyboard, swatches and hex entry share one RGB result.
use crate::primitives::{
    editor::{EditEvent, Editor, InputMode},
    family::{Paint, Slide},
    form_chrome::*,
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColorValue(pub u32);
impl ColorValue {
    pub fn hex(self) -> String {
        format!("#{:06X}", self.0 & 0xffffff)
    }
}
pub struct ColorPicker {
    label: SharedString,
    h: f32,
    s: f32,
    v: f32,
    open: bool,
    swatches: Vec<ColorValue>,
    edit: Entity<Editor>,
    area: [Rc<Cell<Bounds<Pixels>>>; 2],
    drag: Option<usize>,
    reveal: FormReveal,
    knobs: [Slide; 3],
    focus: FocusHandle,
    _sub: Subscription,
}
impl EventEmitter<ColorValue> for ColorPicker {}
impl ColorPicker {
    pub fn new(
        label: impl Into<SharedString>,
        value: ColorValue,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let edit = cx.new(|cx| {
            let mut e = Editor::new("Hex", InputMode::Single, w, cx);
            e.set_value(value.hex(), w, cx);
            e
        });
        let sub = cx.subscribe_in(&edit, w, |s: &mut Self, e, ev: &EditEvent, w, cx| {
            if matches!(ev, EditEvent::Submitted | EditEvent::Blurred) {
                let t = e.read(cx).value().trim_start_matches('#');
                if t.len() == 6
                    && let Ok(n) = u32::from_str_radix(t, 16)
                {
                    s.load(ColorValue(n), w, cx)
                } else {
                    let current = s.value().hex();
                    e.update(cx, |e, cx| e.set_value(current, w, cx))
                }
            }
        });
        let (h, s, v) = hsv(value);
        Self {
            label: label.into(),
            h,
            s,
            v,
            open: false,
            swatches: vec![],
            edit,
            area: Default::default(),
            drag: None,
            reveal: FormReveal::new(false),
            knobs: [Slide::snappy(h), Slide::snappy(s), Slide::snappy(v)],
            focus: cx.focus_handle().tab_stop(true),
            _sub: sub,
        }
    }
    pub fn with_swatches(mut self, s: Vec<ColorValue>) -> Self {
        self.swatches = s;
        self
    }
    pub fn value(&self) -> ColorValue {
        rgb_value(self.h, self.s, self.v)
    }
    fn sync(&mut self, prev: ColorValue, w: &mut Window, cx: &mut Context<Self>) {
        let n = self.value();
        if n != prev {
            cx.emit(n)
        }
        self.edit.update(cx, |e, cx| e.set_value(n.hex(), w, cx));
        cx.notify()
    }
    fn load(&mut self, val: ColorValue, w: &mut Window, cx: &mut Context<Self>) {
        let prev = self.value();
        (self.h, self.s, self.v) = hsv(val);
        self.sync(prev, w, cx)
    }
    fn set(&mut self, i: usize, pos: Point<Pixels>, w: &mut Window, cx: &mut Context<Self>) {
        let b = self.area[i].get();
        if b.size.width <= px(0.) {
            return;
        }
        let old = self.value();
        let x = ((pos.x - b.left()) / b.size.width).clamp(0., 1.);
        if i == 0 {
            self.s = x;
            self.v = (1. - (pos.y - b.top()) / b.size.height).clamp(0., 1.)
        } else {
            self.h = x
        }
        self.sync(old, w, cx)
    }
    fn region(&self, i: usize, w: &Window, cx: &Context<Self>) -> Stateful<Div> {
        let rect = self.area[i].clone();
        let mut bg = div()
            .size_full()
            .relative()
            .rounded(crate::foundation::surface_radius(cx))
            .overflow_hidden();
        if i == 0 {
            bg = bg
                .bg(linear_gradient(
                    90.,
                    linear_color_stop(rgb(0xffffff), 0.),
                    linear_color_stop(rgb(rgb_value(self.h, 1., 1.).0), 1.),
                ))
                .child(div().absolute().inset_0().bg(linear_gradient(
                    180.,
                    linear_color_stop(transparent_black(), 0.),
                    linear_color_stop(rgb(0), 1.),
                )))
        } else {
            let mut band = div().flex().size_full();
            for k in 0..6 {
                band = band.child(div().flex_1().h_full().bg(linear_gradient(
                    90.,
                    linear_color_stop(rgb(rgb_value(k as f32 / 6., 1., 1.).0), 0.),
                    linear_color_stop(rgb(rgb_value((k + 1) as f32 / 6., 1., 1.).0), 1.),
                )))
            }
            bg = bg.child(band)
        }
        let thumb = div()
            .absolute()
            .left(relative(if i == 0 {
                self.knobs[1].get(w, cx)
            } else {
                self.knobs[0].get(w, cx)
            }))
            .top(relative(if i == 0 {
                1. - self.knobs[2].get(w, cx)
            } else {
                0.5
            }))
            .ml(px(-6.))
            .mt(px(-6.))
            .size(px(12.))
            .rounded_full()
            .border_2()
            .border_color(rgb(0xffffff))
            .shadow_sm();
        div()
            .relative()
            .h(px(if i == 0 { 138. } else { 18. }))
            .on_children_prepainted(move |b, _, _| {
                if let Some(x) = b.first() {
                    rect.set(*x)
                }
            })
            .id(("channel", i))
            .role(Role::Slider)
            .aria_label(if i == 0 {
                tr(cx, "Saturation and brightness", "饱和度和亮度")
            } else {
                tr(cx, "Hue", "色相")
            })
            .tab_index(0)
            .child(bg)
            .child(thumb)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |s, e: &MouseDownEvent, w, cx| {
                    s.drag = Some(i);
                    s.set(i, e.position, w, cx)
                }),
            )
            .on_mouse_move(cx.listener(move |s, e: &MouseMoveEvent, w, cx| {
                if s.drag == Some(i) {
                    s.set(i, e.position, w, cx)
                }
            }))
            .on_mouse_up(MouseButton::Left, cx.listener(|s, _, _, _| s.drag = None))
            .on_mouse_up_out(MouseButton::Left, cx.listener(|s, _, _, _| s.drag = None))
            .on_key_down(cx.listener(move |s, e: &KeyDownEvent, w, cx| {
                let old = s.value();
                let d = if e.keystroke.modifiers.shift {
                    0.1
                } else {
                    0.01
                };
                match (e.keystroke.key.as_str(), i) {
                    ("left", 0) => s.s -= d,
                    ("right", 0) => s.s += d,
                    ("up", 0) => s.v += d,
                    ("down", 0) => s.v -= d,
                    ("left", 1) => s.h -= d,
                    ("right", 1) => s.h += d,
                    _ => return,
                }
                s.h = s.h.clamp(0., 1.);
                s.v = s.v.clamp(0., 1.);
                s.s = s.s.clamp(0., 1.);
                s.sync(old, w, cx);
                w.prevent_default();
                cx.stop_propagation()
            }))
    }
}
fn hsv(v: ColorValue) -> (f32, f32, f32) {
    let n = v.0;
    let (r, g, b) = (
        ((n >> 16) & 255) as f32 / 255.,
        ((n >> 8) & 255) as f32 / 255.,
        (n & 255) as f32 / 255.,
    );
    let hi = r.max(g).max(b);
    let d = hi - r.min(g).min(b);
    let h = if d == 0. {
        0.
    } else if hi == r {
        ((g - b) / d).rem_euclid(6.)
    } else if hi == g {
        (b - r) / d + 2.
    } else {
        (r - g) / d + 4.
    };
    (h / 6., if hi == 0. { 0. } else { d / hi }, hi)
}
fn rgb_value(h: f32, s: f32, v: f32) -> ColorValue {
    let i = (h * 6.).floor();
    let f = h * 6. - i;
    let (a, b, c) = (v * (1. - s), v * (1. - s * f), v * (1. - s * (1. - f)));
    let (r, g, b) = match i as usize % 6 {
        0 => (v, c, a),
        1 => (b, v, a),
        2 => (a, v, c),
        3 => (a, b, v),
        4 => (c, a, v),
        _ => (v, a, b),
    };
    ColorValue(
        (((r * 255.).round() as u32) << 16)
            | (((g * 255.).round() as u32) << 8)
            | (b * 255.).round() as u32,
    )
}
impl Render for ColorPicker {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let n = self.value();
        let progress = self.reveal.sample(self.open, 0.42, 0.40, w, cx);
        for (knob, target) in self.knobs.iter_mut().zip([self.h, self.s, self.v]) {
            knob.duration(target, 0.13, cx);
        }
        let mut root = section(285., cx)
            .id("color")
            .items_start()
            .on_mouse_down_out(cx.listener(|s, _, _, cx| {
                if s.open {
                    s.open = false;
                    cx.notify()
                }
            }))
            .capture_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                if e.keystroke.key == "escape" && s.open {
                    s.open = false;
                    s.focus.focus(w, cx);
                    cx.stop_propagation();
                    cx.notify()
                }
            }))
            .child(
                field("swatch", self.label.clone(), cx)
                    .track_focus(&self.focus)
                    .gap(px(8.))
                    .aria_expanded(self.open)
                    .child(div().size(px(23.)).rounded_full().bg(rgb(n.0)))
                    .child(
                        col()
                            .gap_0()
                            .child(self.label.clone())
                            .child(hint(n.hex(), cx)),
                    )
                    .on_click(cx.listener(|s, _, _, cx| {
                        s.open = !s.open;
                        cx.notify()
                    })),
            );
        if self.open || progress > 0. {
            let mut sw = row().flex_wrap();
            for (i, &n) in self.swatches.iter().enumerate() {
                sw = sw.child(
                    button(("swatch", i), "", cx)
                        .aria_label(n.hex())
                        .size(px(24.))
                        .bg(rgb(n.0))
                        .on_click(cx.listener(move |s, _, w, cx| s.load(n, w, cx))),
                )
            }
            root = root.child(
                self.reveal
                    .wrap(
                        panel(cx)
                            .w_full()
                            .child(self.region(0, w, cx))
                            .child(self.region(1, w, cx))
                            .child(
                                row().child(hint("HEX", cx)).child(
                                    div()
                                        .flex_1()
                                        .h(px(28.))
                                        .border_1()
                                        .border_color(p.line)
                                        .child(self.edit.clone()),
                                ),
                            )
                            .child(sw.child(
                                button("save", tr(cx, "+ Swatch", "+ 色样"), cx).on_click(
                                    cx.listener(|s, _, _, cx| {
                                        let n = s.value();
                                        if !s.swatches.contains(&n) {
                                            s.swatches.push(n);
                                            cx.notify()
                                        }
                                    }),
                                ),
                            )),
                        progress,
                        self.open,
                        280.,
                        0.,
                    )
                    .w(px(140. + 145. * progress))
                    .max_w_full(),
            )
        }
        root
    }
}
