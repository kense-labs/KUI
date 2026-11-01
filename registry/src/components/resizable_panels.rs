//! A horizontal split view with draggable and keyboard-operable dividers.
use crate::primitives::{
    direct_touch::Contact,
    family::{Paint, Slide},
    glyph::{Glyph, transformed},
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;
type Content = Box<dyn Fn(Pixels, &mut Window, &mut App) -> AnyElement>;
pub struct Pane {
    id: SharedString,
    label: SharedString,
    share: f32,
    min: f32,
    max: f32,
    collapsible: bool,
    starts_closed: bool,
    content: Content,
}
impl Pane {
    pub fn new<E: IntoElement + 'static>(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        share: f32,
        body: impl Fn(Pixels, &mut Window, &mut App) -> E + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            share: share.max(0.001),
            min: 80.,
            max: f32::INFINITY,
            collapsible: false,
            starts_closed: false,
            content: Box::new(move |n, w, cx| body(n, w, cx).into_any_element()),
        }
    }
    pub fn with_limits(mut self, min: f32, max: Option<f32>) -> Self {
        self.min = min.max(1.);
        self.max = max.unwrap_or(f32::INFINITY).max(self.min);
        self
    }
    pub fn initially_collapsed(mut self) -> Self {
        self.collapsible = true;
        self.starts_closed = true;
        self
    }
    pub fn collapsible(mut self) -> Self {
        self.collapsible = true;
        self
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutChanged(pub Vec<f32>);
struct Handle {
    focus: FocusHandle,
    contact: Contact,
    hover: Slide,
}
#[derive(Clone, Copy)]
struct Drag {
    index: usize,
    x: f32,
    last: f32,
    time: Instant,
    velocity: f32,
    original: f32,
    pair: f32,
    lo: f32,
    hi: f32,
    raw: f32,
    moved: bool,
}
pub struct ResizablePanels {
    id: ElementId,
    label: SharedString,
    panes: Vec<Pane>,
    handles: Vec<Handle>,
    shares: Vec<f32>,
    widths: Vec<Slide>,
    restore: Vec<f32>,
    extent: Rc<Cell<f32>>,
    layout_width: f32,
    drag: Option<Drag>,
    pending_layout: bool,
}
impl EventEmitter<LayoutChanged> for ResizablePanels {}
fn rubber(distance: f32) -> f32 {
    44. * distance * 0.55 / (44. + 0.55 * distance)
}
impl ResizablePanels {
    pub fn new(
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        panes: Vec<Pane>,
        cx: &App,
    ) -> Self {
        let shares = Self::initial_shares(&panes);
        let handles = (1..panes.len())
            .map(|_| Handle {
                focus: cx.focus_handle().tab_stop(true),
                contact: Default::default(),
                hover: Slide::snappy(0.),
            })
            .collect();
        Self {
            id: id.into(),
            label: label.into(),
            widths: vec![Slide::new(0.); panes.len()],
            restore: vec![0.; panes.len()],
            panes,
            handles,
            shares,
            extent: Default::default(),
            layout_width: 0.,
            drag: None,
            pending_layout: false,
        }
    }
    fn initial_shares(panes: &[Pane]) -> Vec<f32> {
        let mut values: Vec<_> = panes
            .iter()
            .map(|p| if p.starts_closed { 0. } else { p.share })
            .collect();
        if !values.is_empty() && values.iter().all(|v| *v == 0.) {
            values[0] = 1.
        }
        let sum = values.iter().sum::<f32>();
        for v in &mut values {
            *v /= sum
        }
        values
    }
    /// Restore the initial proportions within the current viewport.
    pub fn reset_layout(&mut self, cx: &mut Context<Self>) {
        if self.layout_width <= 0. {
            return;
        }
        let mut motion = self.widths.clone();
        self.shares = Self::initial_shares(&self.panes);
        self.drag = None;
        self.layout(self.layout_width);
        for (s, t) in motion.iter_mut().zip(&self.widths) {
            s.duration(t.value(cx), 0.32, cx)
        }
        self.widths = motion;
        self.pending_layout = true;
        cx.notify()
    }
    fn floor(&self, i: usize) -> f32 {
        let sum = self
            .panes
            .iter()
            .enumerate()
            .filter(|(j, _)| self.shares[*j] > 0. || *j == i)
            .map(|(_, p)| p.min)
            .sum::<f32>();
        self.panes[i].min * (self.layout_width / sum.max(1.)).min(1.)
    }
    fn layout(&mut self, width: f32) {
        self.layout_width = width;
        let mut values = self.shares.iter().map(|s| s * width).collect::<Vec<_>>();
        for (i, value) in values.iter_mut().enumerate() {
            if self.shares[i] > 0. {
                *value = value.clamp(self.floor(i), self.panes[i].max)
            }
        }
        for _ in 0..values.len() {
            let delta = width - values.iter().sum::<f32>();
            if delta.abs() < 0.01 {
                break;
            }
            let free: Vec<_> = (0..values.len())
                .filter(|i| {
                    self.shares[*i] > 0.
                        && if delta > 0. {
                            values[*i] < self.panes[*i].max
                        } else {
                            values[*i] > self.floor(*i)
                        }
                })
                .collect();
            let sum = free.iter().map(|i| self.shares[*i]).sum::<f32>();
            for i in free {
                values[i] = (values[i] + delta * self.shares[i] / sum)
                    .clamp(self.floor(i), self.panes[i].max)
            }
        }
        for (i, v) in values.into_iter().enumerate() {
            self.widths[i].reset(v);
            self.shares[i] = v / width
        }
    }
    fn limits(&self, i: usize, pair: f32) -> (f32, f32) {
        let lo = self.floor(i).max(pair - self.panes[i + 1].max).min(pair);
        let hi = (pair - self.floor(i + 1))
            .min(self.panes[i].max)
            .clamp(lo, pair);
        (lo, hi)
    }
    fn start(&mut self, i: usize, x: Pixels, w: &mut Window, cx: &mut Context<Self>) {
        if self.drag.is_some() {
            return;
        }
        let a = self.widths[i].value(cx);
        let b = self.widths[i + 1].value(cx);
        let pair = a + b;
        let (lo, hi) = self.limits(i, pair);
        self.handles[i].focus.focus(w, cx);
        self.drag = Some(Drag {
            index: i,
            x: x.into(),
            last: x.into(),
            time: cx.background_executor().now(),
            velocity: 0.,
            original: a,
            pair,
            lo,
            hi,
            raw: a,
            moved: false,
        });
        cx.notify()
    }
    fn motion(&mut self, x: Pixels, cx: &mut Context<Self>) {
        let Some(d) = self.drag.as_mut() else { return };
        let x = f32::from(x);
        let now = cx.background_executor().now();
        let dt = (now - d.time).as_secs_f32();
        if dt > 0.001 && (x - d.last).abs() > 0.1 {
            d.velocity = (x - d.last) / dt;
            d.last = x;
            d.time = now
        }
        d.raw = d.original + x - d.x;
        d.moved |= (x - d.x).abs() > 2.;
        if !d.moved {
            return;
        }
        let a = if d.raw < d.lo {
            d.lo - rubber(d.lo - d.raw)
        } else if d.raw > d.hi {
            d.hi + rubber(d.raw - d.hi)
        } else {
            d.raw
        };
        let a = a.clamp(0., d.pair);
        self.widths[d.index].reset(a);
        self.widths[d.index + 1].reset(d.pair - a);
        cx.notify()
    }
    fn commit(&mut self, i: usize, a: f32, pair: f32, cx: &mut Context<Self>) {
        if self.layout_width <= 0. {
            return;
        }
        for (j, v) in [(i, a), (i + 1, pair - a)] {
            if v == 0. && self.shares[j] > 0. {
                self.restore[j] = self.shares[j] * self.layout_width
            }
            self.shares[j] = v / self.layout_width;
            self.widths[j].duration(v, 0.32, cx)
        }
        self.pending_layout = true;
        cx.notify()
    }
    fn finish(&mut self, cancel: bool, cx: &mut Context<Self>) {
        let Some(d) = self.drag.take() else { return };
        let i = d.index;
        if cancel {
            for j in i..=i + 1 {
                self.widths[j].set(self.shares[j] * self.layout_width, cx)
            }
            cx.notify();
            return;
        }
        if !d.moved {
            if self.shares[i] == 0. || self.shares[i + 1] == 0. {
                self.keyboard(i, "enter", false, cx)
            }
            cx.notify();
            return;
        }
        let projected = d.raw
            + if (cx.background_executor().now() - d.time).as_millis() < 80 {
                (d.velocity * 0.075).clamp(-150., 150.)
            } else {
                0.
            };
        let a = if self.panes[i].collapsible && projected < d.lo - (d.lo * 0.5).max(40.) {
            0.
        } else if self.panes[i + 1].collapsible
            && projected > d.hi + ((d.pair - d.hi) * 0.5).max(40.)
        {
            d.pair
        } else {
            d.raw.clamp(d.lo, d.hi)
        };
        self.commit(i, a, d.pair, cx)
    }
    fn keyboard(&mut self, i: usize, k: &str, big: bool, cx: &mut Context<Self>) {
        if self.layout_width <= 0. {
            return;
        }
        let a = self.shares[i] * self.layout_width;
        let b = self.shares[i + 1] * self.layout_width;
        let pair = a + b;
        let (lo, hi) = self.limits(i, pair);
        let step = if big { 64. } else { 16. };
        let next = match k {
            "home" => lo,
            "end" => hi,
            "enter" if a <= 0. => self.restore[i].clamp(lo, hi),
            "enter" if b <= 0. => (pair - self.restore[i + 1]).clamp(lo, hi),
            "enter" if self.panes[i].collapsible => 0.,
            "enter" if self.panes[i + 1].collapsible => pair,
            "left" if a <= lo + 0.5 && self.panes[i].collapsible => 0.,
            "right" if a >= hi - 0.5 && self.panes[i + 1].collapsible => pair,
            "left" => (a - step).clamp(lo, hi),
            "right" => (a + step).clamp(lo, hi),
            _ => return,
        };
        if next != a {
            self.commit(i, next, pair, cx)
        }
    }
}
impl Render for ResizablePanels {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let extent = self.extent.clone();
        let width = (extent.get() - self.handles.len() as f32).max(1.);
        if extent.get() > 0. && width != self.layout_width {
            self.drag = None;
            self.layout(width)
        }
        let mut root = div()
            .id(self.id.clone())
            .role(Role::Group)
            .aria_label(self.label.clone())
            .relative()
            .flex()
            .size_full()
            .overflow_hidden();
        let mut offset = 0.;
        let mut grips = Vec::new();
        let mut settled = true;
        for (i, pane) in self.panes.iter().enumerate() {
            let n = self.widths[i].get(w, cx).max(0.);
            settled &= (n - self.shares[i] * self.layout_width).abs() < 0.01;
            root = root.child(
                div()
                    .id(pane.id.clone())
                    .flex_none()
                    .w(px(n))
                    .h_full()
                    .overflow_hidden()
                    .when(n > 0.1, |el| {
                        el.child(
                            div()
                                .w(px(n.max(self.floor(i))))
                                .h_full()
                                .opacity((n / self.floor(i).max(1.)).clamp(0., 1.))
                                .child((pane.content)(px(n), w, cx)),
                        )
                    }),
            );
            offset += n;
            if i == self.handles.len() {
                continue;
            }
            let h = &self.handles[i];
            let active = self.drag.is_some_and(|d| d.index == i) || h.focus.is_focused(w);
            let strength = if active { 1. } else { h.hover.get(w, cx) };
            let collapsed = self.shares[i] == 0. || self.shares[i + 1] == 0.;
            root = root.child(
                div()
                    .w(px(1.))
                    .h_full()
                    .flex_none()
                    .when(!collapsed, |e| e.bg(p.line)),
            );
            let outer = if collapsed { 30. } else { 6. };
            let handle = div()
                .id(("divider", i))
                .debug_selector(|| format!("divider-{}", pane.id))
                .role(Role::Splitter)
                .aria_label(pane.label.clone())
                .aria_min_numeric_value(0.)
                .aria_max_numeric_value(100.)
                .aria_numeric_value(f64::from(self.shares[i] * 100.))
                .aria_value(format!("{:.0}%", self.shares[i] * 100.))
                .track_focus(&h.focus)
                .tab_stop(true)
                .absolute()
                .top_0()
                .bottom_0()
                .left(px(offset - outer))
                .w(px(1. + 2. * outer))
                .flex()
                .items_center()
                .justify_center()
                .cursor(CursorStyle::ResizeLeftRight)
                .on_hover(cx.listener(move |v, h: &bool, _, cx| {
                    v.handles[i]
                        .hover
                        .duration(if *h { 1. } else { 0. }, 0.16, cx);
                    cx.notify()
                }))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |v, e: &MouseDownEvent, w, cx| {
                        if e.click_count == 2 {
                            v.reset_layout(cx)
                        } else {
                            v.start(i, e.position.x, w, cx)
                        }
                        w.prevent_default()
                    }),
                )
                .on_key_down(cx.listener(move |v, e: &KeyDownEvent, _, cx| {
                    let k = e.keystroke.key.as_str();
                    if matches!(k, "left" | "right" | "home" | "end" | "enter" | "escape") {
                        if k == "escape" {
                            v.finish(true, cx)
                        } else {
                            v.keyboard(i, k, e.keystroke.modifiers.shift, cx)
                        }
                        cx.stop_propagation()
                    }
                }))
                .when(!collapsed, |el| {
                    el.child(
                        div()
                            .w(px(4.))
                            .h(px(19. + 13. * strength))
                            .rounded_full()
                            .bg(p.secondary)
                            .opacity(strength),
                    )
                })
                .when(collapsed, |el| {
                    el.child(
                        div()
                            .relative()
                            .left(px(if self.shares[i] == 0. { 12. } else { -12. }))
                            .w(px(18.))
                            .h(px(40.))
                            .border_1()
                            .border_color(p.line)
                            .rounded(crate::foundation::surface_radius(cx))
                            .bg(p.panel)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(transformed(
                                Glyph::Down,
                                12.,
                                p.ink,
                                if self.shares[i] == 0. {
                                    -std::f32::consts::FRAC_PI_2
                                } else {
                                    std::f32::consts::FRAC_PI_2
                                },
                                1.,
                                (0., 0.),
                                1.,
                            )),
                    )
                })
                .child(h.contact.listen(cx, move |v, e, w, cx| match e.phase {
                    TouchPhase::Started => v.start(i, e.position.x, w, cx),
                    TouchPhase::Moved => v.motion(e.position.x, cx),
                    TouchPhase::Ended => v.finish(false, cx),
                    TouchPhase::Cancelled => v.finish(true, cx),
                }));
            grips.push(handle);
            offset += 1.;
        }
        if self.pending_layout && self.drag.is_none() && settled {
            self.pending_layout = false;
            let total = self.shares.iter().sum::<f32>().max(f32::EPSILON);
            cx.emit(LayoutChanged(
                self.shares.iter().map(|v| 100. * v / total).collect(),
            ));
        }
        root = root.children(grips).child(
            canvas(
                move |b, w, _| {
                    if extent.replace(f32::from(b.size.width)) != f32::from(b.size.width) {
                        w.request_animation_frame()
                    }
                },
                |_, _, _, _| (),
            )
            .absolute()
            .inset_0(),
        );
        if self.drag.is_some() {
            let moved = cx.listener(|v, e: &MouseMoveEvent, _, cx| {
                if e.pressed_button == Some(MouseButton::Left) {
                    v.motion(e.position.x, cx)
                } else {
                    v.finish(true, cx)
                }
            });
            let up = cx.listener(|v, e: &MouseUpEvent, _, cx| {
                v.motion(e.position.x, cx);
                v.finish(false, cx)
            });
            root = root.child(
                canvas(
                    |_, _, _| (),
                    move |_, _, w, _| {
                        w.on_mouse_event(move |e: &MouseMoveEvent, phase, w, cx| {
                            if phase.capture() {
                                moved(e, w, cx)
                            }
                        });
                        w.on_mouse_event(move |e: &MouseUpEvent, phase, w, cx| {
                            if phase.capture() && e.button == MouseButton::Left {
                                up(e, w, cx)
                            }
                        })
                    },
                )
                .absolute()
                .size(px(0.)),
            )
        }
        root
    }
}
