//! Expand a story card into a scrollable, keyboard-contained article.
use crate::{
    foundation::surface_radius,
    primitives::{
        family::{Paint, Slide, disabled_motion, ease},
        flyout::TriggerBounds,
        glyph::{Glyph, icon},
        nav::tr,
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

const MAX_SHEET: f32 = 680.;
const TWO_COLUMNS: f32 = 520.;
const DRAG_DISMISS: f32 = 100.;
const FLICK_SPEED: f32 = 780.;

type Body = Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>;

/// Image, text, and a reusable GPUI article body.
#[derive(Clone)]
pub struct ArticleItem {
    pub id: SharedString,
    pub title: SharedString,
    pub image: ImageSource,
    pub image_alt: SharedString,
    pub tag: Option<SharedString>,
    pub meta: Option<SharedString>,
    body: Body,
}

impl ArticleItem {
    pub fn new(
        id: impl Into<SharedString>,
        title: impl Into<SharedString>,
        image: impl Into<ImageSource>,
        image_alt: impl Into<SharedString>,
        body: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            image: image.into(),
            image_alt: image_alt.into(),
            tag: None,
            meta: None,
            body: Rc::new(body),
        }
    }

    pub fn with_tag(mut self, tag: impl Into<SharedString>) -> Self {
        self.tag = Some(tag.into());
        self
    }

    pub fn with_meta(mut self, meta: impl Into<SharedString>) -> Self {
        self.meta = Some(meta.into());
        self
    }
}

/// Open or close intent. A controlled host applies the value with `set_value`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArticleValueChanged(pub Option<SharedString>);

/// Optional bridge for a canvas embedded in a larger document.
pub struct ArticlePreviewHost {
    /// Visible vertical interval in canvas coordinates.
    pub visible_y: fn() -> Option<(Pixels, Pixels)>,
    /// Lock the host document's scroll for the lifetime of the modal.
    pub modal: fn(bool),
    /// Register the reading viewport as the wheel and touch target.
    pub scroll_region: fn(Bounds<Pixels>),
}
impl Global for ArticlePreviewHost {}

#[derive(Clone, Copy)]
struct Pull {
    start: Point<Pixels>,
    last_y: Pixels,
    last_at: Instant,
    speed: f32,
    raw: f32,
}

impl Pull {
    fn new(start: Point<Pixels>, now: Instant) -> Self {
        Self {
            start,
            last_y: start.y,
            last_at: now,
            speed: 0.,
            raw: 0.,
        }
    }

    fn sample(&mut self, at: Point<Pixels>, now: Instant) {
        let delta = at - self.start;
        let dt = now.saturating_duration_since(self.last_at).as_secs_f32();
        if dt > 0.001 {
            self.speed = (f32::from(at.y - self.last_y) / dt).clamp(-2400., 2400.);
        }
        self.last_y = at.y;
        self.last_at = now;
        self.raw = if delta.y.abs() >= delta.x.abs() * 0.75 {
            f32::from(delta.y).max(0.)
        } else {
            0.
        };
    }

    fn dismissed(self, now: Instant) -> bool {
        self.raw >= DRAG_DISMISS
            || (self.raw >= 18.
                && self.speed >= FLICK_SPEED
                && now.saturating_duration_since(self.last_at).as_millis() < 90)
    }
}

fn columns(width: Pixels) -> usize {
    if width >= px(TWO_COLUMNS) { 2 } else { 1 }
}

fn sheet_bounds(viewport: Size<Pixels>, visible: Option<(Pixels, Pixels)>) -> Bounds<Pixels> {
    let (top, bottom) = visible.unwrap_or((px(0.), viewport.height));
    let top = top.clamp(px(0.), viewport.height);
    let bottom = bottom.clamp(top, viewport.height);
    let narrow = viewport.width < px(TWO_COLUMNS);
    let side = px(if narrow { 8. } else { 12. });
    let inset = px(if narrow { 12. } else { 24. });
    let width = (viewport.width - 2. * side).min(px(MAX_SHEET)).max(px(1.));
    let height = (bottom - top - 2. * inset).max(px(1.));
    Bounds::new(
        point((viewport.width - width) / 2., top + inset),
        size(width, height),
    )
}

fn mix(from: Bounds<Pixels>, to: Bounds<Pixels>, t: f32) -> Bounds<Pixels> {
    Bounds::new(
        from.origin + (to.origin - from.origin) * t,
        size(
            from.size.width + (to.size.width - from.size.width) * t,
            from.size.height + (to.size.height - from.size.height) * t,
        ),
    )
}

/// Grid and article reader driven by the same image and title.
pub struct CardToArticle {
    items: Vec<ArticleItem>,
    slots: Vec<(FocusHandle, TriggerBounds)>,
    grid_width: Rc<Cell<Pixels>>,
    aspect: f32,
    close_label: SharedString,
    selected: Option<usize>,
    shown: Option<usize>,
    controlled: bool,
    target_y: Option<(Pixels, Pixels)>,
    host_modal: Option<fn(bool)>,
    modal_focus: FocusHandle,
    close_focus: FocusHandle,
    scroll_focus: FocusHandle,
    want_focus: bool,
    movement: Slide,
    drag: Option<Pull>,
    scroll_pull: Option<(f32, Instant, f32)>,
    pull: Slide,
    scroll: ScrollHandle,
    close_scroll: Option<(Pixels, f32)>,
}

impl EventEmitter<ArticleValueChanged> for CardToArticle {}

impl Drop for CardToArticle {
    fn drop(&mut self) {
        if let Some(unlock) = self.host_modal.take() {
            unlock(false);
        }
    }
}

impl CardToArticle {
    pub fn new(items: Vec<ArticleItem>, cx: &App) -> Self {
        let slots = items
            .iter()
            .map(|_| (cx.focus_handle().tab_stop(true), TriggerBounds::default()))
            .collect();
        Self {
            items,
            slots,
            grid_width: Default::default(),
            aspect: 4. / 3.,
            close_label: tr(cx, "Close article", "关闭文章").into(),
            selected: None,
            shown: None,
            controlled: false,
            target_y: None,
            host_modal: None,
            modal_focus: cx.focus_handle(),
            close_focus: cx.focus_handle().tab_stop(true),
            scroll_focus: cx.focus_handle().tab_stop(true),
            want_focus: false,
            movement: Slide::new(0.),
            drag: None,
            scroll_pull: None,
            pull: Slide::new(0.),
            scroll: ScrollHandle::new(),
            close_scroll: None,
        }
    }

    /// Ratio shared by the card and full-size hero.
    pub fn with_aspect_ratio(mut self, ratio: f32) -> Self {
        if ratio.is_finite() && ratio > 0. {
            self.aspect = ratio.clamp(0.4, 4.);
        }
        self
    }

    pub fn with_close_label(mut self, label: impl Into<SharedString>) -> Self {
        self.close_label = label.into();
        self
    }

    pub fn with_default_value(mut self, value: Option<&str>) -> Self {
        self.selected = self.index(value);
        self.shown = self.selected;
        self.want_focus = self.selected.is_some();
        self.movement.reset(f32::from(self.selected.is_some()));
        self
    }

    pub fn with_value(mut self, value: Option<&str>) -> Self {
        self.controlled = true;
        self.with_default_value(value)
    }

    pub fn value(&self) -> Option<&str> {
        self.selected.map(|i| self.items[i].id.as_ref())
    }

    pub fn set_value(&mut self, value: Option<&str>, w: &mut Window, cx: &mut Context<Self>) {
        self.change(self.index(value), w, cx);
    }

    fn index(&self, value: Option<&str>) -> Option<usize> {
        value.and_then(|id| self.items.iter().position(|item| item.id.as_ref() == id))
    }

    fn request(&mut self, value: Option<usize>, w: &mut Window, cx: &mut Context<Self>) {
        if self.selected != value {
            if !self.controlled {
                self.change(value, w, cx);
            }
            cx.emit(ArticleValueChanged(value.map(|i| self.items[i].id.clone())));
        }
    }

    fn change(&mut self, value: Option<usize>, _: &mut Window, cx: &mut Context<Self>) {
        if value == self.selected {
            return;
        }
        if let Some(i) = value {
            let reopen = self.shown == Some(i) && self.selected.is_none();
            if !reopen {
                self.scroll.set_offset(Point::default());
                self.pull.reset(0.);
                self.movement.reset(0.);
            }
            self.shown = Some(i);
            self.close_scroll = None;
            self.want_focus = true;
            self.movement.duration(1., 0.46, cx);
        } else {
            self.close_scroll = Some((self.scroll.offset().y, self.movement.value(cx).max(0.001)));
            self.movement.duration(0., 0.34, cx);
            self.pull.duration(0., 0.30, cx);
        }
        self.selected = value;
        self.drag = None;
        self.scroll_pull = None;
        cx.notify();
    }

    fn lock_host(&mut self, cx: &App) {
        if let Some(host) = cx.try_global::<ArticlePreviewHost>() {
            self.target_y = (host.visible_y)();
            (host.modal)(true);
            self.host_modal = Some(host.modal);
        }
    }

    fn unlock_host(&mut self) {
        if let Some(unlock) = self.host_modal.take() {
            unlock(false);
        }
        self.target_y = None;
    }

    fn set_scroll(&self, value: Pixels) {
        let top = value.clamp(px(0.), self.scroll.max_offset().y);
        self.scroll.set_offset(point(px(0.), -top));
    }

    fn begin_pull(&mut self, at: Point<Pixels>, cx: &mut Context<Self>) {
        self.drag = Some(Pull::new(at, cx.background_executor().now()));
    }

    fn move_pull(&mut self, at: Point<Pixels>, cx: &mut Context<Self>) {
        if let Some(mut drag) = self.drag {
            drag.sample(at, cx.background_executor().now());
            self.pull.reset(drag.raw);
            self.drag = Some(drag);
            cx.notify();
        }
    }

    fn end_pull(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if let Some(drag) = self.drag.take() {
            if drag.dismissed(cx.background_executor().now()) {
                self.request(None, w, cx);
            }
            self.pull.duration(0., 0.30, cx);
            cx.notify();
        }
    }

    fn capture_pull(&self, cx: &Context<Self>) -> impl IntoElement {
        let moved = cx.listener(|s, e: &MouseMoveEvent, _, cx| {
            if e.pressed_button == Some(MouseButton::Left) {
                s.move_pull(e.position, cx);
            }
        });
        let released = cx.listener(|s, e: &MouseUpEvent, w, cx| {
            if e.button == MouseButton::Left
                && let Some(drag) = s.drag
            {
                if e.position.y != drag.last_y {
                    s.move_pull(e.position, cx);
                }
                s.end_pull(w, cx);
                cx.stop_propagation();
            }
        });
        canvas(
            |_, _, _| (),
            move |_, _, w, _| {
                w.on_mouse_event(move |e: &MouseMoveEvent, phase, w, cx| {
                    if phase.capture() {
                        moved(e, w, cx);
                    }
                });
                w.on_mouse_event(move |e: &MouseUpEvent, phase, w, cx| {
                    if phase.capture() {
                        released(e, w, cx);
                    }
                });
            },
        )
        .absolute()
        .size(px(0.))
    }

    fn wheel(&mut self, e: &ScrollEvent, w: &mut Window, cx: &mut Context<Self>) {
        if self.selected.is_none() || e.modifiers.platform || e.modifiers.control {
            return;
        }
        let now = cx.background_executor().now();
        if matches!(e.touch_phase, TouchPhase::Ended | TouchPhase::Cancelled) {
            if let Some((raw, at, speed)) = self.scroll_pull.take() {
                if e.touch_phase == TouchPhase::Ended
                    && (raw >= DRAG_DISMISS
                        || (raw >= 18.
                            && speed >= FLICK_SPEED
                            && now.saturating_duration_since(at).as_millis() < 90))
                {
                    self.request(None, w, cx);
                }
                self.pull.duration(0., 0.30, cx);
            }
        } else if let ScrollDelta::Pixels(d) = e.delta
            && d.y.abs() >= d.x.abs()
            && self.scroll.offset().y >= -px(0.5)
            && (d.y > px(0.) || self.scroll_pull.is_some())
        {
            let delta = f32::from(d.y);
            let (previous, speed) = self
                .scroll_pull
                .map(|(raw, at, _)| (raw, now.saturating_duration_since(at).as_secs_f32()))
                .filter(|(_, dt)| *dt < 0.16)
                .map_or((0., 0.), |(raw, dt)| (raw, delta / dt.max(0.005)));
            let raw = (previous + delta).clamp(0., 240.);
            self.scroll_pull = (raw > 0.).then_some((raw, now, speed));
            self.pull.reset(raw);
            // Continuous browser wheel events need not have a terminating phase.
            if cfg!(target_family = "wasm") && e.source == ScrollSource::Pointer {
                if raw >= DRAG_DISMISS {
                    self.request(None, w, cx);
                }
                self.pull.duration(0., 0.30, cx);
            }
            if raw > 0. {
                cx.stop_propagation();
                w.prevent_default();
            }
        }
        cx.notify();
    }

    fn key(&mut self, e: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        let amount = -self.scroll.offset().y;
        let page = (self.scroll.bounds().size.height - px(40.)).max(px(40.));
        match e.keystroke.key.as_str() {
            "escape" => self.request(None, w, cx),
            "tab" => {
                if e.keystroke.modifiers.shift {
                    w.focus_prev(cx);
                } else {
                    w.focus_next(cx);
                }
                if !self.modal_focus.contains_focused(w, cx) {
                    if e.keystroke.modifiers.shift {
                        self.scroll_focus.focus(w, cx);
                    } else {
                        self.close_focus.focus(w, cx);
                    }
                }
            }
            "up" => self.set_scroll(amount - px(40.)),
            "down" => self.set_scroll(amount + px(40.)),
            "pageup" => self.set_scroll(amount - page),
            "pagedown" => self.set_scroll(amount + page),
            "home" => self.set_scroll(px(0.)),
            "end" => self.set_scroll(self.scroll.max_offset().y),
            _ => return,
        }
        w.prevent_default();
        cx.stop_propagation();
        cx.notify();
    }

    fn hero(&self, item: &ArticleItem, width: Pixels, radius: Pixels, heading: bool) -> Div {
        let width = f32::from(width);
        let title = (width * 0.074).clamp(14., 52.);
        let meta = (width * 0.034).clamp(10., 20.);
        let tag = (width * 0.032).clamp(10., 22.);
        div()
            .relative()
            .w_full()
            .h(px(width / self.aspect))
            .flex_none()
            .overflow_hidden()
            .rounded_t(radius)
            .when(!heading, |d| d.rounded_b(radius))
            .bg(rgb(0x343434))
            .child(
                img(item.image.clone())
                    .size_full()
                    .rounded_t(radius)
                    .when(!heading, |image| image.rounded_b(radius))
                    .object_fit(ObjectFit::Cover),
            )
            .when(!item.image_alt.is_empty(), |d| {
                d.child(
                    div()
                        .id("article-image")
                        .role(Role::Image)
                        .aria_label(item.image_alt.clone())
                        .absolute()
                        .size(px(1.)),
                )
            })
            .child(
                div()
                    .absolute()
                    .bottom_0()
                    .w_full()
                    .h(relative(0.60))
                    .when(!heading, |d| d.rounded_b(radius))
                    .bg(linear_gradient(
                        180.,
                        linear_color_stop(black().opacity(0.), 0.),
                        linear_color_stop(black().opacity(0.90), 1.),
                    )),
            )
            .children(item.tag.as_ref().map(|label| {
                div()
                    .absolute()
                    .left(px(width * 0.05))
                    .top(px(width * 0.05))
                    .rounded_full()
                    .bg(white().opacity(0.82))
                    .text_color(rgb(0x181818))
                    .px(px((width * 0.036).max(8.)))
                    .py(px((width * 0.011).max(3.)))
                    .text_size(px(tag))
                    .line_height(px(tag * 1.4))
                    .font_weight(FontWeight::MEDIUM)
                    .child(label.clone())
            }))
            .child(
                div()
                    .absolute()
                    .left(px(width * 0.055))
                    .right(px(width * 0.055))
                    .bottom(px(width * 0.05))
                    .flex()
                    .flex_col()
                    .gap(px(width * 0.014))
                    .text_color(white())
                    .child(
                        div()
                            .id("article-title")
                            .when(heading, |d| d.role(Role::Heading))
                            .text_size(px(title))
                            .line_height(px(title * 1.1))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(item.title.clone()),
                    )
                    .children(item.meta.as_ref().map(|label| {
                        div()
                            .text_size(px(meta))
                            .line_height(px(meta * 1.4))
                            .text_color(white().opacity(0.77))
                            .child(label.clone())
                    })),
            )
    }

    fn grid(&self, width: Pixels, radius: Pixels, p: Paint, cx: &Context<Self>) -> Div {
        let cols = columns(width);
        let gap = px(if cols == 2 { 16. } else { 12. });
        let card_width = ((width - gap * (cols - 1) as f32) / cols as f32).max(px(1.));
        let modal = self.shown.is_some();
        let measured = self.grid_width.clone();
        div()
            .relative()
            .w_full()
            .min_w_0()
            .flex()
            .flex_col()
            .gap(gap)
            .child(
                canvas(
                    move |b, w, _| {
                        if measured.replace(b.size.width) != b.size.width {
                            w.request_animation_frame();
                        }
                    },
                    |_, _, _, _| (),
                )
                .absolute()
                .inset_0(),
            )
            .children(self.items.chunks(cols).enumerate().map(|(row, items)| {
                div()
                    .w_full()
                    .flex()
                    .gap(gap)
                    .children(items.iter().enumerate().map(|(col, item)| {
                        let i = row * cols + col;
                        let (focus, target) = &self.slots[i];
                        div()
                            .id(("article-card", i))
                            .debug_selector(move || format!("article-card-{i}"))
                            .w(card_width)
                            .h(card_width / self.aspect)
                            .flex_none()
                            .relative()
                            .overflow_hidden()
                            .rounded(radius)
                            .track_focus(focus)
                            .tab_index(if modal { -1 } else { 0 })
                            .role(Role::Button)
                            .aria_label(item.title.clone())
                            .aria_expanded(self.selected == Some(i))
                            .focus_visible(move |style| {
                                style.shadow(vec![
                                    BoxShadow::new(px(0.), px(0.), p.bright).spread_radius(px(1.)),
                                ])
                            })
                            .cursor_pointer()
                            .child(target.measure())
                            .child(
                                self.hero(item, card_width, radius, false)
                                    .absolute()
                                    .inset_0()
                                    .opacity(f32::from(self.shown != Some(i))),
                            )
                            .when(!modal, |d| {
                                d.on_click(
                                    cx.listener(move |s, _, w, cx| s.request(Some(i), w, cx)),
                                )
                                .on_key_down(cx.listener(
                                    move |s, e: &KeyDownEvent, w, cx| {
                                        if !e.is_held
                                            && matches!(e.keystroke.key.as_str(), "enter" | "space")
                                        {
                                            s.request(Some(i), w, cx);
                                            w.prevent_default();
                                            cx.stop_propagation();
                                        }
                                    },
                                ))
                            })
                    }))
            }))
    }
}

impl Render for CardToArticle {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let viewport = w.viewport_size();
        let p = Paint::get(cx);
        let radius = surface_radius(cx);
        let t = self.movement.get(w, cx).clamp(0., 1.);
        let reduced = disabled_motion(cx);
        let opacity = if reduced { 1. } else { t };
        if self.selected.is_none()
            && t <= 0.001
            && let Some(i) = self.shown.take()
        {
            self.unlock_host();
            let focus = self.slots[i].0.clone();
            w.on_next_frame(move |w, cx| focus.focus(w, cx));
        }
        let measured = self.grid_width.get();
        let width = if measured > px(0.) {
            measured.min(viewport.width)
        } else {
            viewport.width
        };
        let root = self.grid(width, radius, p, cx);
        let Some(i) = self.shown else {
            return root;
        };
        if self.selected.is_some() && self.host_modal.is_none() {
            self.lock_host(cx);
        }
        if std::mem::take(&mut self.want_focus) {
            let focus = self.close_focus.clone();
            w.on_next_frame(move |w, cx| focus.focus(w, cx));
        }
        let item = &self.items[i];
        let sheet = sheet_bounds(viewport, self.target_y);
        let slot = self.slots[i].1.get();
        let slot = if slot.size.width > px(0.) {
            slot
        } else {
            sheet
        };
        let mut surface = mix(slot, sheet, t);
        let raw_pull = self.pull.get(w, cx).max(0.);
        let pull = if reduced { 0. } else { raw_pull * 0.62 };
        let scale = 1. - (pull / 850.).clamp(0., 0.18);
        let shrink = surface.size.width * (1. - scale);
        surface.origin.x += shrink / 2.;
        surface.origin.y += px(pull);
        surface.size.width -= shrink;
        surface.size.height *= scale;
        if let Some((offset, from)) = self.close_scroll {
            self.scroll
                .set_offset(point(px(0.), offset * (t / from).clamp(0., 1.)));
        }
        let show_body = if reduced { 1. } else { ease((t - 0.74) / 0.26) };
        let show_close = if reduced { 1. } else { ease((t - 0.85) / 0.15) };
        let hero = self
            .hero(item, surface.size.width, radius, true)
            .id("article-hero")
            .debug_selector(|| "article-hero".into())
            .cursor_grab()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|s, e: &MouseDownEvent, _, cx| {
                    if s.selected.is_some() && s.scroll.offset().y >= -px(0.5) {
                        s.begin_pull(e.position, cx);
                    }
                }),
            );
        let scroller = div()
            .id("article-reading")
            .debug_selector(|| "article-reading".into())
            .track_focus(&self.scroll_focus)
            .tab_index(0)
            .role(Role::Group)
            .aria_label(tr(cx, "Article content", "文章内容"))
            .relative()
            .size_full()
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .on_scroll(cx.listener(Self::wheel))
            .child(hero)
            .child(
                div()
                    .w_full()
                    .px((surface.size.width / 16.).min(px(48.)).max(px(16.)))
                    .pt(px(28.))
                    .pb(px(56.))
                    .text_size(px(16.))
                    .line_height(px(26.))
                    .text_color(p.ink)
                    .opacity(show_body)
                    .child((item.body)(w, cx)),
            );
        let progress = if self.scroll.max_offset().y > px(0.) {
            (-self.scroll.offset().y / self.scroll.max_offset().y).clamp(0., 1.)
        } else {
            0.
        };
        let close = div()
            .id("article-close")
            .debug_selector(|| "article-close".into())
            .role(Role::Button)
            .aria_label(self.close_label.clone())
            .block_mouse_except_scroll()
            .track_focus(&self.close_focus)
            .tab_index(0)
            .absolute()
            .right(px(14.))
            .top(px(14.))
            .size(px(36.))
            .rounded_full()
            .bg(white().opacity(0.82))
            .border_1()
            .border_color(transparent_black())
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .opacity(show_close)
            .focus_visible(|s| s.border_color(rgb(0x171717)))
            .hover(|s| s.bg(white()))
            .child(icon(Glyph::Close, 16., black()))
            .on_click(cx.listener(|s, _, w, cx| s.request(None, w, cx)))
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                if !e.is_held && matches!(e.keystroke.key.as_str(), "enter" | "space") {
                    s.request(None, w, cx);
                    w.prevent_default();
                    cx.stop_propagation();
                }
            }));
        let sheet = div()
            .id("article-sheet")
            .debug_selector(|| "article-sheet".into())
            .relative()
            .left(surface.left())
            .top(surface.top())
            .absolute()
            .w(surface.size.width)
            .h(surface.size.height)
            .rounded(radius)
            .overflow_hidden()
            .bg(p.panel)
            .occlude()
            .child(scroller)
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .h(px(3.))
                    .w(relative(progress))
                    .bg(p.bright)
                    .opacity(show_body),
            )
            .child(close);
        let layer = div()
            .id("article-modal")
            .role(Role::Dialog)
            .aria_label(item.title.clone())
            .track_focus(&self.modal_focus)
            .relative()
            .w(viewport.width)
            .h(viewport.height)
            .occlude()
            .bg(black().opacity(0.52 * opacity * (1. - (pull / 400.).clamp(0., 0.55))))
            .on_key_down(cx.listener(Self::key))
            .child(self.capture_pull(cx))
            .child(
                div()
                    .id("article-backdrop")
                    .absolute()
                    .inset_0()
                    .on_click(cx.listener(|s, _, w, cx| s.request(None, w, cx))),
            )
            .child(
                canvas(
                    |_, _, _| (),
                    move |_, _, _, cx| {
                        if let Some(host) = cx.try_global::<ArticlePreviewHost>() {
                            (host.scroll_region)(surface);
                        }
                    },
                )
                .absolute()
                .size(px(0.)),
            )
            .child(sheet);
        root.child(deferred(anchored().position(Point::default()).child(layer)).with_priority(5))
    }
}

#[cfg(test)]
mod tests {
    use super::{ArticleItem, CardToArticle, Pull, sheet_bounds};
    use gpui::prelude::{ParentElement, Styled};
    use gpui::{
        AppContext, Context, Entity, IntoElement, Modifiers, MouseButton, Render, TestAppContext,
        Window, div, point, px, size,
    };
    use std::time::Duration;
    use web_time::Instant;

    struct Gallery(Entity<CardToArticle>);
    impl Render for Gallery {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(self.0.clone())
        }
    }

    fn items() -> Vec<ArticleItem> {
        ["one", "two", "three", "four"]
            .into_iter()
            .map(|id| {
                ArticleItem::new(id, id, "story.jpg".to_owned(), "", |_, _| {
                    div()
                        .flex()
                        .flex_col()
                        .children((0..40).map(|n| div().h(px(24.)).child(format!("Paragraph {n}"))))
                        .into_any_element()
                })
            })
            .collect()
    }

    #[test]
    fn release_distinguishes_a_flick_from_a_short_or_stale_pull() {
        let now = Instant::now();
        let mut drag = Pull::new(point(px(10.), px(10.)), now);
        drag.sample(point(px(12.), px(30.)), now + Duration::from_millis(60));
        assert!(!drag.dismissed(now + Duration::from_millis(61)));
        drag.sample(point(px(13.), px(58.)), now + Duration::from_millis(80));
        assert!(drag.dismissed(now + Duration::from_millis(81)));
        assert!(!drag.dismissed(now + Duration::from_millis(200)));
        drag.sample(point(px(13.), px(125.)), now + Duration::from_millis(350));
        assert!(drag.dismissed(now + Duration::from_millis(500)));
    }

    #[test]
    fn keyboard_reader_contains_focus_scrolls_and_returns_to_the_relaid_slot() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (gallery, window) =
            app.add_window_view(|_, cx| Gallery(cx.new(|cx| CardToArticle::new(items(), cx))));
        window.simulate_resize(size(px(620.), px(720.)));
        window.run_until_parked();
        let control = window.update(|_, cx| gallery.read(cx).0.clone());
        let first = window.debug_bounds("article-card-0").unwrap();
        let second = window.debug_bounds("article-card-1").unwrap();
        assert_eq!(second.top(), first.top());
        window.update(|w, cx| {
            control.read(cx).slots[0].0.clone().focus(w, cx);
            w.refresh();
        });
        window.simulate_keystrokes("enter");
        window.executor().advance_clock(Duration::from_secs(2));
        window.update(|w, cx| {
            w.simulate_next_frame(cx);
            w.refresh();
        });
        window.run_until_parked();
        assert_eq!(
            window.update(|_, cx| control.read(cx).value().map(str::to_owned)),
            Some("one".into())
        );
        assert!(window.update(|w, cx| control.read(cx).close_focus.is_focused(w)));
        assert_eq!(
            window.debug_bounds("article-sheet").unwrap(),
            sheet_bounds(size(px(620.), px(720.)), None)
        );
        window.simulate_keystrokes("tab");
        assert!(window.update(|w, cx| control.read(cx).modal_focus.contains_focused(w, cx)));
        window.simulate_keystrokes("pagedown");
        assert!(window.update(|_, cx| control.read(cx).scroll.offset().y < px(0.)));
        window.simulate_resize(size(px(400.), px(720.)));
        window.run_until_parked();
        let second = window.debug_bounds("article-card-1").unwrap();
        let first = window.debug_bounds("article-card-0").unwrap();
        assert!(second.top() > first.top());
        window.simulate_keystrokes("escape");
        window.executor().advance_clock(Duration::from_secs(2));
        window.update(|w, cx| {
            w.simulate_next_frame(cx);
            w.refresh();
        });
        window.run_until_parked();
        window.update(|w, cx| {
            w.simulate_next_frame(cx);
            w.refresh();
        });
        assert!(window.update(|w, cx| {
            let reader = control.read(cx);
            reader.value().is_none() && reader.shown.is_none() && reader.slots[0].0.is_focused(w)
        }));
    }

    #[test]
    fn mouse_pull_uses_sheet_geometry_and_release_position() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (gallery, window) = app.add_window_view(|_, cx| {
            Gallery(cx.new(|cx| CardToArticle::new(items(), cx).with_default_value(Some("one"))))
        });
        window.simulate_resize(size(px(620.), px(720.)));
        window.run_until_parked();
        let start = window.debug_bounds("article-hero").unwrap().center();
        let sheet_top = window.debug_bounds("article-sheet").unwrap().top();
        let mid = start + point(px(0.), px(62.));
        let end = start + point(px(0.), px(145.));
        window.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
        window.simulate_mouse_move(mid, MouseButton::Left, Modifiers::default());
        window.run_until_parked();
        assert!(window.debug_bounds("article-sheet").unwrap().top() > sheet_top + px(20.));
        window.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
        assert!(window.update(|_, cx| gallery.read(cx).0.read(cx).value().is_none()));
    }

    #[test]
    fn clicking_close_does_not_begin_a_hero_drag() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (gallery, window) = app.add_window_view(|_, cx| {
            Gallery(cx.new(|cx| CardToArticle::new(items(), cx).with_default_value(Some("one"))))
        });
        window.simulate_resize(size(px(620.), px(720.)));
        window.run_until_parked();
        let close = window.debug_bounds("article-close").unwrap().center();
        window.simulate_mouse_move(close, None, Modifiers::default());
        window.simulate_mouse_down(close, MouseButton::Left, Modifiers::default());
        assert!(window.update(|_, cx| gallery.read(cx).0.read(cx).drag.is_none()));
        window.simulate_mouse_up(close, MouseButton::Left, Modifiers::default());
        assert!(window.update(|_, cx| gallery.read(cx).0.read(cx).value().is_none()));
    }

    #[test]
    fn controlled_selection_waits_for_the_host() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (gallery, window) = app.add_window_view(|_, cx| {
            Gallery(cx.new(|cx| CardToArticle::new(items(), cx).with_value(None)))
        });
        let control = window.update(|_, cx| gallery.read(cx).0.clone());
        window.update(|w, cx| {
            control.update(cx, |reader, cx| {
                reader.request(Some(1), w, cx);
                assert_eq!(reader.value(), None);
                reader.set_value(Some("two"), w, cx);
                assert_eq!(reader.value(), Some("two"));
                reader.request(None, w, cx);
                assert_eq!(reader.value(), Some("two"));
                reader.set_value(None, w, cx);
                assert_eq!(reader.value(), None);
            })
        });
    }
}
