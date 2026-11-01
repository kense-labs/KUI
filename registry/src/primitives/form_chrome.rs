//! Shared labelled controls for the form family.
use super::{
    control::UiLanguage,
    family::{Paint, Tone, face},
    glyph::{Glyph, icon},
};
use gpui::{prelude::*, *};
pub fn tr(cx: &App, en: &'static str, zh: &'static str) -> &'static str {
    UiLanguage::current(cx).text(en, zh)
}
pub fn col() -> Div {
    div().flex().flex_col().min_w_0().gap(px(8.))
}
pub fn row() -> Div {
    div().flex().items_center().min_w_0().gap(px(8.))
}
pub fn section(width: f32, cx: &App) -> Div {
    col()
        .w(px(width))
        .max_w_full()
        .text_size(px(14.))
        .line_height(px(20.))
        .text_color(Paint::get(cx).ink)
}
pub fn panel(cx: &App) -> Div {
    col()
        .rounded(crate::foundation::surface_radius(cx))
        .border_1()
        .border_color(Paint::get(cx).line)
        .bg(Paint::get(cx).panel)
        .p(px(8.))
}
pub fn hint(text: impl Into<SharedString>, cx: &App) -> Div {
    div()
        .text_size(px(12.))
        .line_height(px(17.))
        .text_color(Paint::get(cx).faint)
        .child(text.into())
}
pub fn button(id: impl Into<ElementId>, name: impl Into<SharedString>, cx: &App) -> Stateful<Div> {
    let n = name.into();
    let p = Paint::get(cx);
    face(
        id,
        n.clone(),
        32.,
        crate::foundation::surface_radius(cx).into(),
        Tone::Clear,
        cx,
    )
    .px(px(9.))
    .hover(move |s| s.bg(p.hover))
    .when(!n.is_empty(), |button| button.child(n))
}
pub fn glyph(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    g: Glyph,
    cx: &App,
) -> Stateful<Div> {
    let p = Paint::get(cx);
    face(
        id,
        label,
        32.,
        crate::foundation::surface_radius(cx).into(),
        Tone::Clear,
        cx,
    )
    .w(px(32.))
    .hover(move |s| s.bg(p.hover))
    .child(icon(g, 16., p.secondary))
}
pub fn field(id: impl Into<ElementId>, name: impl Into<SharedString>, cx: &App) -> Stateful<Div> {
    let p = Paint::get(cx);
    face(
        id,
        name,
        44.,
        crate::foundation::surface_radius(cx).into(),
        Tone::Clear,
        cx,
    )
    .justify_between()
    .px(px(12.))
    .border_1()
    .border_color(p.line)
    .bg(p.panel)
    .hover(move |s| s.border_color(p.faint))
}

/// Calendar outline kept separate from the time-of-day glyph.
pub fn calendar_mark(ink: Hsla) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |b, _, w, _| {
            let at = |x: f32, y: f32| b.origin + point(px(x), px(y));
            let mut path = PathBuilder::stroke(px(1.4));
            path.move_to(at(2., 4.));
            path.line_to(at(14., 4.));
            path.line_to(at(14., 14.));
            path.line_to(at(2., 14.));
            path.line_to(at(2., 4.));
            path.move_to(at(2., 7.));
            path.line_to(at(14., 7.));
            path.move_to(at(5., 2.));
            path.line_to(at(5., 5.));
            path.move_to(at(11., 2.));
            path.line_to(at(11., 5.));
            if let Ok(path) = path.build() {
                w.paint_path(path, ink)
            }
        },
    )
    .size(px(16.))
}

/// The same panel stays mounted during exit. Its unclipped inner stack supplies the
/// natural height even when the outer reveal is a single pixel tall.
pub struct FormReveal {
    spring: super::family::Slide,
    anchor: std::rc::Rc<std::cell::Cell<Bounds<Pixels>>>,
    panel_bounds: std::rc::Rc<std::cell::Cell<Bounds<Pixels>>>,
    visible: bool,
    above: bool,
    alpha_start: f32,
    alpha: f32,
    fade: super::family::PhaseClock,
    height: std::rc::Rc<std::cell::Cell<Pixels>>,
}
impl FormReveal {
    pub fn new(visible: bool) -> Self {
        Self {
            spring: super::family::Slide::snappy(visible as u8 as f32),
            anchor: Default::default(),
            panel_bounds: Default::default(),
            visible,
            above: false,
            alpha_start: visible as u8 as f32,
            alpha: visible as u8 as f32,
            fade: Default::default(),
            height: Default::default(),
        }
    }
    pub fn sample(&mut self, open: bool, enter: f32, exit: f32, w: &Window, cx: &App) -> f32 {
        if self.visible != open {
            self.alpha_start = self.alpha;
            self.visible = open;
            self.fade.restart(cx);
        }
        let duration = if open { 0.16 } else { 0.12 };
        let t = self.fade.time(duration, w, cx) / duration;
        self.alpha = self.alpha_start
            + (open as u8 as f32 - self.alpha_start) * super::family::standard_ease(t);
        self.spring
            .duration(open as u8 as f32, if open { enter } else { exit }, cx);
        // Once the exit fade finishes, an invisible spring tail must not intercept
        // clicks on the field underneath it.
        if !open && self.alpha <= 0. {
            0.
        } else {
            self.spring.get(w, cx).clamp(0., 1.)
        }
    }
    pub fn above(mut self) -> Self {
        self.above = true;
        self
    }
    pub fn at(&self, b: Bounds<Pixels>) {
        self.anchor.set(b)
    }
    pub fn anchor(&self) -> impl IntoElement + 'static {
        let bounds = self.anchor.clone();
        canvas(move |b, _, _| bounds.set(b), |_, _, _, _| ())
            .absolute()
            .inset_0()
    }
    pub fn anchor_bounds(&self) -> Bounds<Pixels> {
        self.anchor.get()
    }
    pub fn contains(&self, pos: Point<Pixels>) -> bool {
        self.panel_bounds.get().contains(&pos)
    }
    /// Position after layout so the trigger's hit target never moves when the popover appears.
    pub fn popup(
        &self,
        child: impl IntoElement,
        t: f32,
        open: bool,
        geometry: (f32, f32, f32),
        w: &Window,
    ) -> Div {
        let b = self.anchor.get();
        let view = w.viewport_size();
        let (width, estimate, travel) = geometry;
        let measured = self.panel_bounds.get();
        let h = if measured.size.height > px(1.) {
            measured.size.height
        } else {
            px(estimate)
        };
        let needed = h.min(px(160.));
        let fits_above = b.top() > needed + px(14.);
        let fits_below = view.height - b.bottom() > needed + px(14.);
        let above = if self.above && fits_above {
            true
        } else if !self.above && fits_below {
            false
        } else {
            b.top() > view.height - b.bottom()
        };
        let side = if above {
            Anchor::BottomLeft
        } else {
            Anchor::TopLeft
        };
        let y = if above {
            b.top() - px(6.) + px(travel * (1. - t))
        } else {
            b.bottom() + px(6.) - px(travel * (1. - t))
        };
        let bounds = self.panel_bounds.clone();
        let width = if width > 0. { px(width) } else { b.size.width };
        let available = (if above {
            b.top()
        } else {
            view.height - b.bottom()
        } - px(14.))
        .max(px(32.));
        let face = div()
            .id("form-popover-viewport")
            .max_h(available)
            .overflow_y_scroll()
            .w(width.min((view.width - px(16.)).max(px(1.))))
            .relative()
            .opacity(self.alpha)
            .child(child)
            .child(
                canvas(move |b, _, _| bounds.set(b), |_, _, _, _| ())
                    .absolute()
                    .inset_0(),
            )
            .when(!open, |d| {
                d.child(div().id("closing-form-menu").absolute().inset_0().occlude())
            });
        div().absolute().size(px(0.)).child(
            deferred(
                anchored()
                    .anchor(side)
                    .position(point(b.left(), y))
                    .snap_to_window_with_margin(px(8.))
                    .child(face),
            )
            .with_priority(3),
        )
    }
    /// Keep the backing surface alive independently of the faster exit of its contents.
    pub fn range_progress(&mut self, open: bool, w: &Window, cx: &App) -> f32 {
        if self.visible != open {
            self.alpha_start = self.alpha;
            self.visible = open;
            self.fade.restart(cx);
        }
        let (delay, duration) = if open { (0.07, 0.26) } else { (0., 0.12) };
        let elapsed = self.fade.time(delay + duration, w, cx);
        let t = ((elapsed - delay) / duration).clamp(0., 1.);
        self.alpha = self.alpha_start
            + (open as u8 as f32 - self.alpha_start) * super::family::standard_ease(t);
        self.spring
            .duration(open as u8 as f32, if open { 0.48 } else { 0.38 }, cx);
        self.spring.get(w, cx).clamp(0., 1.)
    }

    /// Grow a single 44px field into the measured panel without moving its layout slot.
    pub fn over_trigger(
        &self,
        (child, moving_label): (impl IntoElement, Option<Div>),
        t: f32,
        open: bool,
        open_width: f32,
        w: &Window,
        cx: &App,
    ) -> Div {
        let b = self.anchor_bounds();
        let available = w.viewport_size() - size(px(16.), px(16.));
        let width = px(open_width).min(available.width);
        let target_left = b.left().clamp(
            px(8.),
            (w.viewport_size().width - width - px(8.)).max(px(8.)),
        );
        let left = b.left() + (target_left - b.left()) * t;
        let measured = self.height.clone();
        let bounds = self.panel_bounds.clone();
        let full_height = if measured.get() > px(1.) {
            measured.get()
        } else {
            px(400.)
        };
        let height = b.size.height + (full_height.min(available.height) - b.size.height) * t;
        let p = Paint::get(cx);
        let backing = if open { 1. } else { (t * 6.).min(1.) };
        let shell = div()
            .debug_selector(|| "date-range-surface".into())
            .relative()
            .overflow_hidden()
            .rounded(crate::foundation::surface_radius(cx))
            .border_1()
            .border_color(p.line.opacity(backing))
            .bg(p.panel.opacity(backing))
            .w(b.size.width + (width - b.size.width) * t)
            .h(height)
            .when(open || self.alpha > 0., |el| {
                el.child(
                    div()
                        .absolute()
                        .left_0()
                        .top_0()
                        .w((width - px(2.)).max(px(1.)))
                        .on_children_prepainted(move |bounds, _, _| {
                            if let Some(b) = bounds.first() {
                                measured.set(b.size.height + px(2.));
                            }
                        })
                        .id("range-panel-scroll")
                        .max_h((available.height - px(2.)).max(px(1.)))
                        .overflow_y_scroll()
                        .opacity(self.alpha)
                        .child(child),
                )
            })
            .children(moving_label)
            .child(
                canvas(move |b, _, _| bounds.set(b), |_, _, _, _| ())
                    .absolute()
                    .inset_0(),
            );
        div().absolute().size(px(0.)).child(
            deferred(
                anchored()
                    .anchor(Anchor::TopLeft)
                    .position(point(left, b.top()))
                    .snap_to_window_with_margin(px(8.))
                    .child(shell),
            )
            .with_priority(3),
        )
    }
    pub fn wrap(
        &self,
        child: impl IntoElement,
        t: f32,
        open: bool,
        initial_height: f32,
        lift: f32,
    ) -> Div {
        let measured = self.height.clone();
        let height = self.height.get();
        let height = if height > px(1.) {
            height
        } else {
            px(initial_height)
        };
        div()
            .relative()
            .flex()
            .flex_col()
            .min_w_0()
            .overflow_hidden()
            .when(t < 0.9999, |el| {
                el.h(height * t)
                    .opacity(self.alpha)
                    .relative()
                    .top(px(-lift * (1. - t)))
            })
            .on_children_prepainted(move |b, _, _| {
                if let Some(b) = b.first() {
                    measured.set(b.size.height);
                }
            })
            .child(div().flex_none().min_w_0().child(child))
            .when(!open, |d| {
                d.child(div().id("closing-surface").absolute().inset_0().occlude())
            })
    }
}

/// A fixed baseline and a moving copy for changing digits. Shared character
/// positions stay put while the changed places roll in the direction of change.
#[derive(Default)]
pub struct RollingValue {
    shown: Option<SharedString>,
    previous: SharedString,
    direction: f32,
    clock: super::family::PhaseClock,
}
impl RollingValue {
    pub fn label(
        &mut self,
        next: impl Into<SharedString>,
        direction: f32,
        w: &Window,
        cx: &App,
    ) -> Div {
        let next = next.into();
        if self.shown.as_ref() != Some(&next)
            && let Some(old) = self.shown.replace(next.clone())
        {
            self.previous = old;
            self.direction = direction;
            self.clock.restart(cx);
        }
        let time = self.clock.time(0.24, w, cx);
        if time >= 0.24 {
            return div().child(next);
        }
        let in_t = super::family::ease(time / 0.24);
        let out_t = super::family::standard_ease(time / 0.14);
        let a: Vec<_> = self.previous.chars().collect();
        let b: Vec<_> = next.chars().collect();
        let count = a.len().max(b.len());
        let mut row = div().flex().items_center().h(px(20.)).overflow_hidden();
        for i in 0..count {
            let prev = a.get(i).map(char::to_string).unwrap_or_default();
            let next = b.get(i).map(char::to_string).unwrap_or_default();
            if prev == next {
                row = row.child(div().child(next));
                continue;
            }
            let width = super::family::measure(&prev.clone().into(), 14., w) * (1. - in_t)
                + super::family::measure(&next.clone().into(), 14., w) * in_t;
            row = row.child(
                div()
                    .relative()
                    .w(px(width))
                    .h(px(20.))
                    .child(
                        div()
                            .absolute()
                            .top(px(-6. * self.direction * out_t))
                            .opacity(1. - out_t)
                            .child(prev),
                    )
                    .child(
                        div()
                            .absolute()
                            .top(px(6. * self.direction * (1. - in_t)))
                            .opacity(in_t)
                            .child(next),
                    ),
            );
        }
        row
    }
}

/// Trace the two check segments in order. Mixed state interpolates both vertices
/// into a horizontal dash without replacing the glyph midway.
pub fn check_stroke(amount: f32, mixed: f32, side: f32, ink: Hsla) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |b, _, w, _| {
            let vertex = |x: f32, y: f32, dx: f32, dy: f32| {
                b.origin
                    + point(
                        px((x + (dx - x) * mixed) * side),
                        px((y + (dy - y) * mixed) * side),
                    )
            };
            let pts = [
                vertex(0.14, 0.52, 0.18, 0.5),
                vertex(0.42, 0.77, 0.50, 0.5),
                vertex(0.87, 0.24, 0.82, 0.5),
            ];
            let mut stroke = PathBuilder::stroke(px(1.65));
            stroke.move_to(pts[0]);
            for i in 0..2 {
                let end = if i == 0 {
                    (amount / 0.35).clamp(0., 1.)
                } else {
                    ((amount - 0.35) / 0.65).clamp(0., 1.)
                };
                if end > 0. {
                    stroke.line_to(pts[i] + (pts[i + 1] - pts[i]) * end);
                }
            }
            if amount > 0.
                && let Ok(path) = stroke.build()
            {
                w.paint_path(path, ink);
            }
        },
    )
    .size(px(side))
}
