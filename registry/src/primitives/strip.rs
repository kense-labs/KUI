//! Shared focus, sliding highlight and menu behavior for grouped actions.
use crate::foundation::Theme;
use crate::primitives::{
    control::activate,
    family::{Paint, PhaseClock, Slide, Tone, ease, face, lettering, measure},
    glyph::{Glyph, icon},
};
use gpui::{prelude::*, *};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
#[derive(Clone, Debug)]
pub struct Choice {
    pub glyph: Option<Glyph>,
    pub dangerous: bool,
    pub id: SharedString,
    pub label: SharedString,
    pub disabled: bool,
}
impl Choice {
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            glyph: None,
            dangerous: false,
            disabled: false,
        }
    }
    pub fn icon(mut self, g: Glyph) -> Self {
        self.glyph = Some(g);
        self
    }
    pub fn danger(mut self) -> Self {
        self.dangerous = true;
        self
    }
    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }
}
#[derive(Clone, Debug)]
pub struct Segment {
    pub id: SharedString,
    pub label: SharedString,
    pub glyph: Option<Glyph>,
    pub width: f32,
    pub icon_only: bool,
    pub pressed: Option<bool>,
    pub disabled: bool,
    pub divider: bool,
    pub options: Vec<Choice>,
    original: SharedString,
    original_icon: Option<Glyph>,
    generation: u64,
    previous: SharedString,
    painted: SharedString,
    clock: PhaseClock,
    measured: Option<SharedString>,
}
impl Segment {
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        glyph: Option<Glyph>,
        width: f32,
    ) -> Self {
        let label: SharedString = label.into();
        Self {
            id: id.into(),
            label: label.clone(),
            previous: label.clone(),
            painted: label.clone(),
            clock: Default::default(),
            measured: None,
            original: label,
            original_icon: glyph,
            generation: 0,
            glyph,
            width,
            icon_only: false,
            pressed: None,
            disabled: false,
            divider: false,
            options: vec![],
        }
    }
    /// Size from shaped text and reserve space for an optional acknowledgement.
    pub fn fit_label(mut self, reserve: impl Into<SharedString>) -> Self {
        self.measured = Some(reserve.into());
        self
    }
    pub fn toggle(mut self, pressed: bool) -> Self {
        self.pressed = Some(pressed);
        self
    }
    pub fn icon_only(mut self) -> Self {
        self.icon_only = true;
        self
    }
    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }
    pub fn divider(mut self) -> Self {
        self.divider = true;
        self
    }
    pub fn with_menu(mut self, items: Vec<Choice>) -> Self {
        self.options = items;
        self
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Activated(pub SharedString);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    Attached,
    Floating,
    Expanding,
    Split,
}
pub struct Strip {
    id: ElementId,
    pub items: Vec<Segment>,
    pub layout: Layout,
    pub height: f32,
    pub vertical: bool,
    pub primary: bool,
    focus: Vec<FocusHandle>,
    menu_focus: Vec<FocusHandle>,
    open: Option<usize>,
    selected: usize,
    last_focus: Option<usize>,
    lit: bool,
    visibility: Slide,
    press: Slide,
    pressed: Option<usize>,
    dividers: Vec<Slide>,
    menu_clock: PhaseClock,
    confirmed: Option<usize>,
    tip: Option<usize>,
    tip_hover: Option<usize>,
    tip_ticket: u64,
    pointer: Option<Point<Pixels>>,
    measured: bool,
    highlight_box: Rc<RefCell<HighlightBox>>,
    snap_highlight: bool,
    sizes: Vec<Slide>,
}
pub trait StripControl: Render + EventEmitter<Activated> + Sized {
    fn strip(&mut self) -> &mut Strip;
}
impl Strip {
    pub fn new(id: ElementId, items: Vec<Segment>, layout: Layout, cx: &App) -> Self {
        let height = match layout {
            Layout::Attached => 42.,
            Layout::Split => 44.,
            _ => 36.,
        };
        let sizes = items
            .iter()
            .enumerate()
            .map(|(i, s)| {
                Slide::new(if layout == Layout::Expanding && i > 0 {
                    height
                } else {
                    s.width
                })
            })
            .collect();
        let focus = items
            .iter()
            .map(|_| cx.focus_handle().tab_stop(true))
            .collect();
        let dividers = items.iter().map(|_| Slide::snappy(1.)).collect();
        Self {
            id,
            items,
            layout,
            height,
            vertical: false,
            primary: layout == Layout::Split,
            focus,
            menu_focus: vec![],
            open: None,
            selected: 0,
            last_focus: None,
            lit: layout == Layout::Expanding,
            visibility: Slide::snappy(if layout == Layout::Expanding { 1. } else { 0. }),
            press: Slide::snappy(0.),
            pressed: None,
            dividers,
            menu_clock: Default::default(),
            confirmed: None,
            tip: None,
            tip_hover: None,
            tip_ticket: 0,
            pointer: None,
            measured: false,
            highlight_box: Rc::new(RefCell::new(HighlightBox::default())),
            snap_highlight: true,
            sizes,
        }
    }
    pub fn height(&mut self, h: f32) {
        self.height = h;
        for (i, s) in self.items.iter().enumerate() {
            self.sizes[i] = Slide::new(self.width(i, s));
        }
    }
    fn width(&self, i: usize, s: &Segment) -> f32 {
        if self.layout == Layout::Expanding && i != self.selected {
            self.height
        } else {
            s.width
        }
    }
    fn gap(&self) -> f32 {
        match self.layout {
            Layout::Floating => {
                if self.height <= 30. {
                    3.
                } else {
                    4.
                }
            }
            Layout::Expanding => 2.,
            _ => 0.,
        }
    }
    fn extra(&self, s: &Segment) -> f32 {
        if self.layout == Layout::Floating && s.divider {
            9.
        } else {
            0.
        }
    }
    fn highlight(&mut self, i: usize, cx: &App) {
        let entering = !self.lit;
        self.selected = i;
        self.lit = true;
        self.visibility.set(1., cx);
        self.snap_highlight |= entering;
        for (j, item) in self.items.iter().enumerate() {
            let width = self.width(j, item);
            self.sizes[j].set(width, cx);
        }
    }
    fn close(&mut self, w: &mut Window, cx: &mut App) {
        if let Some(i) = self.open.take() {
            self.focus[i].focus(w, cx)
        }
    }
    fn open(&mut self, i: usize, w: &mut Window, cx: &mut App) {
        self.open = Some(i);
        self.menu_clock.restart(cx);
        self.menu_focus = self.items[i]
            .options
            .iter()
            .map(|_| cx.focus_handle().tab_stop(true))
            .collect();
        if let Some(n) = self.items[i].options.iter().position(|x| !x.disabled) {
            let h = self.menu_focus[n].clone();
            h.focus(w, cx)
        }
    }
}
/// Temporarily acknowledge a segment; a newer acknowledgement owns its reset timer.
pub fn acknowledge<T: StripControl>(
    entity: &Entity<T>,
    id: &str,
    label: &str,
    ok: bool,
    cx: &mut App,
) {
    let key = id.to_owned();
    let entity = entity.clone();
    entity.update(cx, |v, cx| {
        let strip = v.strip();
        let Some(index) = strip.items.iter().position(|item| *item.id == *key) else {
            return;
        };
        if strip.layout == Layout::Expanding {
            strip.confirmed = Some(index);
            strip.highlight(index, cx)
        }
        let reset_ms = if strip.layout == Layout::Expanding {
            1400
        } else {
            1800
        };
        let it = &mut strip.items[index];
        it.generation += 1;
        let generation = it.generation;
        it.label = label.to_owned().into();
        it.glyph = Some(if ok { Glyph::Check } else { Glyph::Alert });
        cx.notify();
        cx.spawn(async move |v, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(reset_ms))
                .await;
            let _ = v.update(cx, |v, cx| {
                if let Some(it) = v.strip().items.iter_mut().find(|it| *it.id == *key)
                    && it.generation == generation
                {
                    it.label = it.original.clone();
                    it.glyph = it.original_icon;
                    if v.strip().confirmed == Some(index) {
                        v.strip().confirmed = None
                    }
                    cx.notify()
                }
            });
        })
        .detach();
    });
}
fn seek(n: usize, i: usize, back: bool, allow: impl Fn(usize) -> bool) -> Option<usize> {
    (1..=n)
        .map(|step| {
            if back {
                (i + n - step) % n
            } else {
                (i + step) % n
            }
        })
        .find(|&j| allow(j))
}
// Position is local to the tray so moving the window or scrolling its host
// never becomes a highlight animation. Targets come from the current layout pass.
struct HighlightBox {
    start: Slide,
    length: Slide,
    initialized: bool,
}
impl Default for HighlightBox {
    fn default() -> Self {
        Self {
            start: Slide::new(0.),
            length: Slide::new(0.),
            initialized: false,
        }
    }
}
impl HighlightBox {
    fn follow(
        &mut self,
        target: Bounds<Pixels>,
        surface: Bounds<Pixels>,
        vertical: bool,
        duration: Option<f32>,
        w: &Window,
        cx: &App,
    ) -> Bounds<Pixels> {
        let offset = target.origin - surface.origin;
        let (start, length) = if vertical {
            (offset.y, target.size.height)
        } else {
            (offset.x, target.size.width)
        };
        // The expanding item already springs to its width. Follow that painted
        // rectangle directly instead of putting a second spring behind it.
        if self.initialized
            && let Some(duration) = duration
        {
            self.start.duration(start.into(), duration, cx);
            self.length.duration(length.into(), duration, cx);
        } else {
            self.start.reset(start.into());
            self.length.reset(length.into());
            self.initialized = true;
        }
        let mut rect = target;
        let position = px(self.start.get(w, cx));
        let length = px(self.length.get(w, cx));
        if vertical {
            rect.origin.y = surface.top() + position;
            rect.size.height = length;
        } else {
            rect.origin.x = surface.left() + position;
            rect.size.width = length;
        }
        rect.intersect(&surface)
    }
}

pub fn draw<T: StripControl>(v: &mut T, w: &mut Window, cx: &mut Context<T>) -> impl IntoElement {
    let s = v.strip();
    let p = Paint::get(cx);
    let floating = s.layout == Layout::Floating;
    let expanding = s.layout == Layout::Expanding;
    let separated = floating || expanding;
    let pad = if floating {
        if s.height <= 30. { 3. } else { 4. }
    } else if expanding {
        4.
    } else {
        0.
    };
    let h = s.height;
    let gap = s.gap();
    let bg = if s.primary {
        p.bright
    } else if floating && h > 30. {
        p.tray
    } else {
        p.panel
    };
    let fg = if s.primary { p.inverse } else { p.ink };
    let radius = Theme::global(cx).tokens.radius.md;
    let inset = if separated { 0. } else { 3. };
    let keyboard = s.focus.iter().position(|f| f.is_focused(w));
    if keyboard != s.last_focus {
        s.last_focus = keyboard;
        if let Some(i) = keyboard {
            s.highlight(i, cx)
        }
    }
    let mut resized = false;
    for (i, it) in s.items.iter_mut().enumerate() {
        if it.painted != it.label {
            it.previous = std::mem::replace(&mut it.painted, it.label.clone());
            it.clock.restart(cx)
        }
        if let Some(reserve) = &it.measured
            && !it.icon_only
        {
            let font_size = if h <= 30. { 12. } else { 14. };
            let text = measure(&it.label, font_size, w).max(measure(reserve, font_size, w));
            let edges = match s.layout {
                Layout::Expanding if h <= 30. => 18.,
                Layout::Expanding => 24.,
                Layout::Split => 32.,
                Layout::Floating if h <= 30. => 20.,
                Layout::Floating => 24.,
                Layout::Attached if h <= 34. => 24.,
                _ => 28.,
            };
            let icon_slot = match s.layout {
                Layout::Expanding if h <= 30. => 19.,
                Layout::Floating => 22.,
                Layout::Attached => 24.,
                _ => 23.,
            };
            let wanted = (edges + if it.glyph.is_some() { icon_slot } else { 0. } + text).max(h);
            if (it.width - wanted).abs() > 0.01 {
                it.width = wanted;
                resized = true
            }
            let end = if expanding && i != s.selected {
                h
            } else {
                wanted
            };
            if !s.measured {
                s.sizes[i].reset(end)
            } else {
                s.sizes[i].set(end, cx)
            }
        }
    }
    if resized && (s.lit || s.open.is_some()) {
        let selected = s.selected;
        if !s.measured {
            s.lit = false
        }
        s.highlight(selected, cx)
    }
    s.measured = true;
    let visible = s.visibility.get(w, cx).clamp(0., 1.);
    let down = s.press.get(w, cx).clamp(0., 1.);
    let widths = s
        .sizes
        .iter()
        .map(|size| size.get(w, cx))
        .collect::<Vec<_>>();
    let total: f32 = widths.iter().sum::<f32>()
        + s.items.iter().map(|it| s.extra(it)).sum::<f32>()
        + (s.items.len().saturating_sub(1) as f32) * gap;
    let available = (f32::from(w.viewport_size().width) - 18. - 2. * pad).max(100.);
    let scale = if s.vertical {
        1.
    } else {
        (available / total.max(1.)).min(1.)
    };
    let mut bar = div()
        .id(s.id.clone())
        .relative()
        .flex_none()
        .rounded(radius)
        .p(px(pad))
        .bg(bg)
        .when(!expanding, |e| {
            e.border_1()
                .border_color(if s.primary { p.bright } else { p.line })
        })
        .on_hover(cx.listener(|v, h: &bool, _, cx| {
            let s = v.strip();
            if !*h && s.layout == Layout::Expanding {
                s.pointer = None;
                let i = s.confirmed.or(s.last_focus).unwrap_or(0);
                s.highlight(i, cx);
                cx.notify()
            }
            if !*h && s.layout != Layout::Expanding {
                s.tip = None;
                s.tip_hover = None;
                s.tip_ticket = s.tip_ticket.wrapping_add(1);
                s.lit = false;
                s.visibility.set(0., cx);
                s.press.set(0., cx);
                cx.notify()
            }
        }));
    let selected_bounds = Rc::new(Cell::new(None::<Bounds<Pixels>>));
    if visible > 0.001 || s.open.is_some() {
        let bounds = selected_bounds.clone();
        let geometry = s.highlight_box.clone();
        let snap = std::mem::take(&mut s.snap_highlight);
        let vertical = s.vertical;
        let duration = match s.layout {
            Layout::Attached if !snap => Some(0.26),
            Layout::Floating if !snap => Some(0.42),
            _ => None,
        };
        let color = if s.primary {
            fg.opacity(0.09 + 0.06 * down)
        } else {
            p.hover.blend(fg.opacity(0.05 * down))
        };
        bar = bar.child(
            canvas(
                |_, _, _| (),
                move |surface, _, w, cx| {
                    let Some(target) = bounds.get() else { return };
                    let rect = geometry
                        .borrow_mut()
                        .follow(target, surface, vertical, duration, w, cx);
                    let shrink = if floating {
                        0.06 * (1. - visible) + 0.035 * down
                    } else {
                        0.
                    };
                    let dx = rect.size.width * (shrink * 0.5) + px(inset);
                    let dy = rect.size.height * (shrink * 0.5) + px(inset);
                    let rect = Bounds::from_corners(
                        rect.origin + point(dx, dy),
                        rect.bottom_right() - point(dx, dy),
                    );
                    if rect.size.width <= px(0.) || rect.size.height <= px(0.) {
                        return;
                    }
                    let corners =
                        Corners::all(radius.min(rect.size.width / 2.).min(rect.size.height / 2.));
                    w.paint_quad(fill(rect, color.opacity(visible)).corner_radii(corners));
                },
            )
            .absolute()
            .inset_0(),
        );
    }
    let trigger = std::rc::Rc::new(std::cell::Cell::new(Bounds::<Pixels>::default()));
    let mut row = div().flex().gap(px(gap)).when(s.vertical, |e| e.flex_col());
    let selected_child = s.selected
        + s.items
            .iter()
            .take(s.selected + 1)
            .filter(|it| floating && it.divider)
            .count();
    let tip_index = s.tip.or_else(|| keyboard.filter(|_| s.tip_hover.is_none()));
    for (i, it) in s.items.iter().enumerate() {
        let f = s.focus[i].clone();
        let muted = it.disabled;
        let ink = if muted {
            p.faint.opacity(0.55)
        } else if floating || expanding && i != s.selected {
            p.secondary
        } else {
            fg
        };
        if floating && it.divider {
            row = row.child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .when(s.vertical, |e| e.w(px(h)).h(px(5.)))
                    .when(!s.vertical, |e| e.h(px(h)).w(px(5.)))
                    .child(
                        div()
                            .bg(p.line)
                            .when(s.vertical, |e| e.w(px(h - 12.)).h(px(1.)))
                            .when(!s.vertical, |e| e.h(px(h - 12.)).w(px(1.))),
                    ),
            )
        }
        let wide = widths[i] * scale;
        let show = !it.icon_only;
        let active = s.selected == i;
        let reveal = if expanding {
            ((wide - h) / (it.width - h).max(1.)).clamp(0., 1.)
        } else {
            1.
        };
        let shrink = if !floating && it.options.is_empty() && s.pressed == Some(i) {
            1. - down * if it.icon_only { 0.1 } else { 0.03 }
        } else {
            1.
        };
        let target_border = if s.lit && (s.selected == i || i == s.selected + 1) {
            0.
        } else {
            1.
        };
        s.dividers[i].set(target_border, cx);
        let edge = s.dividers[i].get(w, cx);
        let hit = std::rc::Rc::new(std::cell::Cell::new(Bounds::<Pixels>::default()));
        let saved = hit.clone();

        let mut item = face(
            it.id.clone(),
            it.label.clone(),
            h,
            radius.into(),
            Tone::Clear,
            cx,
        )
        .track_focus(&f)
        .tab_stop(!muted)
        .w(px(wide))
        .relative()
        .when(expanding, |e| {
            e.justify_start().pl(px(if h <= 30. { 7. } else { 10. }))
        })
        .child(
            canvas(|_, _, _| (), move |b, _, _, _| saved.set(b))
                .absolute()
                .inset_0(),
        )
        .gap(px(if scale < 0.94 {
            3.
        } else if expanding && h <= 30. {
            5.
        } else if floating {
            6.
        } else {
            7.
        }))
        .overflow_hidden()
        .text_color(ink)
        .text_size(px(if h <= 30. || scale < 0.85 { 12. } else { 14. }))
        .when(muted, |e| e.cursor_default())
        .when(!separated && i > 0, |e| {
            e.child(
                div()
                    .absolute()
                    .left_0()
                    .top(px(h * 0.28))
                    .h(px(h * 0.44))
                    .w(px(1.))
                    .bg(fg.opacity(0.15 * edge)),
            )
        });
        if let Some(on) = it.pressed {
            item = item.aria_selected(on).when(on, |e| {
                e.bg(p.hover)
                    .text_color(p.ink)
                    .border_1()
                    .border_color(p.line)
            });
        }
        if !it.options.is_empty() {
            item = item.aria_expanded(s.open == Some(i));
        }
        if it.generation > 0 && it.glyph == Some(Glyph::Check) && expanding {
            item = item.bg(p.success.opacity(0.13));
        }
        if let Some(mut g) = it.glyph {
            if g == Glyph::Down && s.open == Some(i) {
                g = Glyph::Up
            }
            item = item.child(icon(
                g,
                (if h <= 30. { 14. } else { 16. }) * shrink,
                if expanding && g == Glyph::Check {
                    p.success
                } else {
                    ink
                },
            ))
        }
        if show {
            let fs = if h <= 30. || scale < 0.85 { 12. } else { 14. };
            let slot = measure(&it.label, fs, w).max(1.) * shrink;
            let t = it.clock.time(0.4, w, cx);
            item = item.child(
                div()
                    .relative()
                    .flex_none()
                    .w(px(slot))
                    .h(px(fs * 1.4))
                    .opacity(ease(reveal))
                    .child(
                        div()
                            .absolute()
                            .left(px(if expanding { -4. * (1. - reveal) } else { 0. }))
                            .child(lettering(
                                it.previous.clone(),
                                it.label.clone(),
                                t,
                                true,
                                fs * shrink,
                                ink,
                                slot,
                            )),
                    ),
            )
        }
        if floating && it.icon_only {
            item = item.on_hover(cx.listener(move |v, over: &bool, _, cx| {
                let s = v.strip();
                if !*over {
                    if s.tip_hover == Some(i) {
                        s.tip = None;
                        s.tip_hover = None;
                        s.tip_ticket = s.tip_ticket.wrapping_add(1);
                        cx.notify()
                    }
                    return;
                }
                s.tip_ticket = s.tip_ticket.wrapping_add(1);
                let ticket = s.tip_ticket;
                s.tip_hover = Some(i);
                s.tip = None;
                cx.spawn(async move |v, cx| {
                    cx.background_executor()
                        .timer(std::time::Duration::from_millis(250))
                        .await;
                    let _ = v.update(cx, |v, cx| {
                        let s = v.strip();
                        if s.tip_ticket == ticket && s.tip_hover == Some(i) {
                            s.tip = Some(i);
                            cx.notify()
                        }
                    });
                })
                .detach();
            }));
        }
        if !muted {
            item = activate(item, cx, move |v, w, cx| {
                let s = v.strip();
                s.highlight(i, cx);
                if let Some(on) = &mut s.items[i].pressed {
                    *on = !*on;
                }
                if !s.items[i].options.is_empty() {
                    if s.open == Some(i) {
                        s.close(w, cx)
                    } else {
                        s.open(i, w, cx)
                    }
                } else {
                    let id = s.items[i].id.clone();
                    cx.emit(Activated(id))
                }
                cx.notify()
            })
            .on_mouse_move(cx.listener(move |v, e: &MouseMoveEvent, _, cx| {
                let s = v.strip();
                if s.pointer == Some(e.position) {
                    return;
                }
                s.pointer = Some(e.position);
                let b = hit.get();
                if s.layout == Layout::Expanding
                    && !active
                    && (e.position.x < b.left() + px(4.) || e.position.x > b.right() - px(4.))
                {
                    return;
                }
                if s.confirmed.is_some() {
                    return;
                }
                if !s.lit || s.selected != i {
                    s.highlight(i, cx);
                    cx.notify()
                }
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |v, _, _, cx| {
                    let s = v.strip();
                    s.highlight(i, cx);
                    s.pressed = Some(i);
                    s.press.set(1., cx);
                    cx.notify()
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|v, _, _, cx| {
                    v.strip().press.set(0., cx);
                    cx.notify()
                }),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|v, _, _, cx| {
                    v.strip().press.set(0., cx);
                    cx.notify()
                }),
            )
            .on_key_down(cx.listener(move |v, e: &KeyDownEvent, w, cx| {
                let s = v.strip();
                let k = e.keystroke.key.as_str();
                if k == "down" && !s.items[i].options.is_empty() {
                    s.open(i, w, cx)
                } else {
                    let j = match k {
                        "home" => s.items.iter().position(|a| !a.disabled),
                        "end" => s.items.iter().rposition(|a| !a.disabled),
                        "right" | "left" | "up" | "down" => {
                            seek(s.items.len(), i, matches!(k, "left" | "up"), |j| {
                                !s.items[j].disabled
                            })
                        }
                        _ => return,
                    };
                    if let Some(j) = j {
                        s.focus[j].focus(w, cx);
                        s.highlight(j, cx)
                    }
                }
                cx.stop_propagation();
                cx.notify()
            }));
        }
        if s.open == Some(i) {
            let bounds = trigger.clone();
            item = item.child(
                canvas(|_, _, _| (), move |b, _, _, _| bounds.set(b))
                    .absolute()
                    .inset_0(),
            );
        }
        let mut slot = div().relative().flex_none().child(item);
        if floating && tip_index == Some(i) && it.icon_only {
            let label = it.original.clone();
            let width = measure(&label, 12., w) + 18.;
            let tip = div()
                .absolute()
                .flex()
                .items_center()
                .justify_center()
                .when(s.vertical, |e| {
                    e.left_full().ml(px(10.)).top_1_2().mt(px(-13.))
                })
                .when(!s.vertical, |e| {
                    e.left_1_2().ml(px(-width / 2.)).bottom_full().mb(px(8.))
                })
                .w(px(width))
                .h(px(26.))
                .rounded(radius)
                .bg(p.bright)
                .text_color(p.inverse)
                .text_size(px(12.))
                .whitespace_nowrap()
                .child(label);
            slot = slot.child(deferred(tip).with_priority(1));
        }
        row = row.child(slot);
    }
    bar = bar.child(row.on_children_prepainted(move |bounds, _, _| {
        selected_bounds.set(bounds.get(selected_child).copied());
    }));
    if let Some(i) = s.open {
        let age = s.menu_clock.time(0.35, w, cx);
        let mut menu = div()
            .opacity(ease(age / 0.12))
            .id("choices")
            .role(Role::Menu)
            .occlude()
            .flex()
            .flex_col()
            .p(px(5.))
            .w(px(190.))
            .rounded(radius)
            .border_1()
            .border_color(p.line)
            .bg(p.panel)
            .shadow_md()
            .on_mouse_down_out(cx.listener(move |v, e: &MouseDownEvent, w, cx| {
                if trigger.get().contains(&e.position) {
                    return;
                }
                v.strip().close(w, cx);
                cx.notify()
            }));
        for (j, choice) in s.items[i].options.iter().enumerate() {
            let id = choice.id.clone();
            let mut f = face(
                choice.id.clone(),
                choice.label.clone(),
                36.,
                radius.into(),
                Tone::Clear,
                cx,
            )
            .opacity(ease((age - 0.016 * j as f32) / 0.16))
            .relative()
            .top(px(3. * (1. - ease((age - 0.016 * j as f32) / 0.16))))
            .track_focus(&s.menu_focus[j])
            .tab_stop(!choice.disabled)
            .role(Role::MenuItem)
            .justify_start()
            .px(px(10.))
            .text_color(if choice.dangerous { p.danger } else { p.ink })
            .when_some(choice.glyph, |el, g| {
                el.child(icon(
                    g,
                    16.,
                    if choice.dangerous { p.danger } else { p.ink },
                ))
            })
            .child(choice.label.clone());
            if choice.disabled {
                f = f.opacity(0.4)
            } else {
                f = activate(f, cx, move |v, w, cx| {
                    v.strip().close(w, cx);
                    cx.emit(Activated(id.clone()));
                    cx.notify()
                })
                .on_key_down(cx.listener(move |v, e: &KeyDownEvent, w, cx| {
                    let s = v.strip();
                    match e.keystroke.key.as_str() {
                        "escape" => s.close(w, cx),
                        "up" | "down" => {
                            if let Some(j) =
                                seek(s.items[i].options.len(), j, e.keystroke.key == "up", |k| {
                                    !s.items[i].options[k].disabled
                                })
                            {
                                s.menu_focus[j].focus(w, cx)
                            }
                        }
                        "tab" => {
                            s.close(w, cx);
                            if e.keystroke.modifiers.shift {
                                w.focus_prev(cx)
                            } else {
                                w.focus_next(cx)
                            }
                        }
                        _ => return,
                    }
                    cx.stop_propagation();
                    cx.notify()
                }))
            }
            menu = menu.child(f);
        }
        bar = bar.child(
            deferred(div().absolute().right_0().top(px(h + 6.)).child(menu)).with_priority(2),
        );
    }
    bar
}
