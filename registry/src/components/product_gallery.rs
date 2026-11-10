//! A product gallery with a hover lens, gliding highlights, and a fullscreen viewer.
use crate::primitives::{
    control::UiLanguage,
    family::{Paint, Slide, disabled_motion},
    flyout::TriggerBounds,
    glyph::{Glyph, icon},
    nav::button,
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};

const THUMB_W: f32 = 64.;
const THUMB_H: f32 = 80.;
const THUMB_STEP: f32 = 88.;
const SWATCH: f32 = 32.;
const SWATCH_STEP: f32 = 44.;
const SIZE_W: f32 = 92.;
const SIZE_H: f32 = 44.;
const SIZE_STEP: f32 = 100.;
const LENS: f32 = 176.;
const ZOOM: f32 = 2.4;
const FLICK: f32 = 360.;
const TRACK_TIME: f32 = 0.6;
const GLIDE: f32 = 0.3;
const FADE_TIME: f32 = 0.24;
const VIEWER_OPEN: f32 = 0.3;
const VIEWER_CLOSE: f32 = 0.26;
const ZOOM_TIME: f32 = 0.42;

#[derive(Clone)]
pub struct ProductPhoto {
    pub image: ImageSource,
    pub alt: SharedString,
}
impl ProductPhoto {
    pub fn new(image: impl Into<ImageSource>, alt: impl Into<SharedString>) -> Self {
        Self {
            image: image.into(),
            alt: alt.into(),
        }
    }
}
#[derive(Clone)]
pub struct ProductColor {
    pub id: SharedString,
    pub label: SharedString,
    pub swatch: Hsla,
    pub photos: Vec<ProductPhoto>,
    pub sold_out: bool,
}
impl ProductColor {
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        swatch: Hsla,
        photos: Vec<ProductPhoto>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            swatch,
            photos,
            sold_out: false,
        }
    }
}
#[derive(Clone)]
pub struct ProductSize {
    pub id: SharedString,
    pub label: SharedString,
    pub sold_out_for: Vec<SharedString>,
}
impl ProductSize {
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            sold_out_for: Vec::new(),
        }
    }
    pub fn unavailable_for(mut self, colors: Vec<SharedString>) -> Self {
        self.sold_out_for = colors;
        self
    }
}
#[derive(Clone, Debug)]
pub enum ProductGalleryEvent {
    Color(SharedString),
    Size(Option<SharedString>),
    Index(usize),
    Viewer(bool),
}
type GalleryContent = Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>;
struct Drag {
    start: Pixels,
    origin: f32,
    last: Pixels,
    last_time: web_time::Instant,
    speed: f32,
    dragged: bool,
}
/// Photo-track position with rubber-band edges: travel past either end
/// compresses to 35% and never exceeds 0.45 of a photo.
fn rubber_band(value: f32, max: f32) -> f32 {
    if value < 0. {
        (value * 0.35).max(-0.45)
    } else if value > max {
        max + ((value - max) * 0.35).min(0.45)
    } else {
        value
    }
}
/// Where a released drag lands: momentum carries the track, and a flick
/// (fast enough, same direction) always advances exactly one photo.
fn landing(current: f32, origin: f32, speed: f32, step: f32) -> f32 {
    let mut landed = current - speed * 0.199 / step.max(1.);
    if speed.abs() > FLICK {
        let direction = if speed < 0. { 1. } else { -1. };
        if (landed - origin) * direction < 1. {
            landed = origin + direction;
        }
    }
    landed.round()
}
pub struct ProductGallery {
    label: SharedString,
    header: Option<GalleryContent>,
    footer: Option<GalleryContent>,
    colors: Vec<ProductColor>,
    sizes: Vec<ProductSize>,
    color: usize,
    size: Option<usize>,
    index: usize,
    previous: Option<ImageSource>,
    fade: Slide,
    track: Slide,
    rail: Slide,
    swatch: Slide,
    size_fill: Slide,
    drag: Option<Drag>,
    /// The last press traveled as a drag; suppress the click it releases.
    dragged: bool,
    lens: Option<Point<Pixels>>,
    lens_x: Slide,
    lens_y: Slide,
    bounds: TriggerBounds,
    zoom: bool,
    zoom_slide: Slide,
    viewer_cursor: Option<Point<Pixels>>,
    main_focus: FocusHandle,
    viewer_focus: FocusHandle,
    viewer_close: FocusHandle,
    viewer: bool,
    viewer_open: Slide,
    container_width: Rc<Cell<Pixels>>,
    touch: Rc<Cell<u8>>,
}
impl EventEmitter<ProductGalleryEvent> for ProductGallery {}
impl ProductGallery {
    pub fn new(
        label: impl Into<SharedString>,
        colors: Vec<ProductColor>,
        sizes: Vec<ProductSize>,
        cx: &App,
    ) -> Self {
        let first_available = colors.iter().position(|color| !color.sold_out).unwrap_or(0);
        Self {
            label: label.into(),
            header: None,
            footer: None,
            colors,
            sizes,
            color: first_available,
            size: None,
            index: 0,
            previous: None,
            fade: Slide::new(1.),
            track: Slide::new(0.),
            rail: Slide::new(0.),
            swatch: Slide::new(first_available as f32),
            size_fill: Slide::new(-1.),
            drag: None,
            dragged: false,
            lens: None,
            lens_x: Slide::new(0.),
            lens_y: Slide::new(0.),
            bounds: TriggerBounds::default(),
            zoom: false,
            zoom_slide: Slide::new(1.),
            viewer_cursor: None,
            main_focus: cx.focus_handle().tab_stop(true),
            viewer_focus: cx.focus_handle(),
            viewer_close: cx.focus_handle().tab_stop(true),
            viewer: false,
            viewer_open: Slide::new(0.),
            container_width: Default::default(),
            touch: Default::default(),
        }
    }
    pub fn with_header(
        mut self,
        header: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        self.header = Some(Rc::new(header));
        self
    }
    pub fn with_footer(
        mut self,
        footer: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        self.footer = Some(Rc::new(footer));
        self
    }
    fn current(&self) -> Option<&ProductPhoto> {
        self.colors.get(self.color)?.photos.get(self.index)
    }
    fn glide(slide: &mut Slide, target: f32, seconds: f32, cx: &App) {
        if disabled_motion(cx) {
            slide.reset(target);
        } else {
            slide.duration(target, seconds, cx);
        }
    }
    fn photo_count(&self) -> usize {
        self.colors.get(self.color).map_or(0, |c| c.photos.len())
    }
    pub fn set_index(&mut self, index: usize, cx: &mut Context<Self>) {
        let len = self.photo_count();
        if len == 0 {
            return;
        }
        let index = index.min(len - 1);
        if self.index == index && self.track.value(cx) == index as f32 {
            return;
        }
        self.index = index;
        Self::glide(&mut self.track, index as f32, TRACK_TIME, cx);
        Self::glide(&mut self.rail, index as f32, GLIDE, cx);
        cx.emit(ProductGalleryEvent::Index(index));
        cx.notify();
    }
    pub fn set_color(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(index) = self
            .colors
            .iter()
            .position(|c| c.id.as_ref() == id && !c.sold_out)
        else {
            return;
        };
        if self.color == index {
            return;
        }
        self.previous = self.current().map(|p| p.image.clone());
        self.color = index;
        self.index = 0;
        if self.current().is_none() {
            self.previous = None;
        }
        self.fade.reset(0.);
        Self::glide(&mut self.fade, 1., FADE_TIME, cx);
        self.track.reset(0.);
        self.rail.reset(0.);
        Self::glide(&mut self.swatch, index as f32, GLIDE, cx);
        if self
            .size
            .is_some_and(|i| self.sizes[i].sold_out_for.contains(&self.colors[index].id))
        {
            self.size = None;
            cx.emit(ProductGalleryEvent::Size(None));
        }
        cx.emit(ProductGalleryEvent::Color(self.colors[index].id.clone()));
        cx.notify();
    }
    pub fn set_size(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(index) = self.sizes.iter().position(|s| s.id.as_ref() == id) else {
            return;
        };
        let Some(color) = self.colors.get(self.color) else {
            return;
        };
        if self.sizes[index].sold_out_for.contains(&color.id) || self.size == Some(index) {
            return;
        }
        self.size = Some(index);
        Self::glide(&mut self.size_fill, index as f32, GLIDE, cx);
        cx.emit(ProductGalleryEvent::Size(Some(
            self.sizes[index].id.clone(),
        )));
        cx.notify();
    }
    fn sold_out_in_color(&self, size: &ProductSize) -> bool {
        self.colors
            .get(self.color)
            .is_some_and(|c| size.sold_out_for.contains(&c.id))
    }
    fn toggle_zoom(&mut self, cx: &mut Context<Self>) {
        self.zoom = !self.zoom;
        if disabled_motion(cx) {
            self.zoom_slide.reset(if self.zoom { ZOOM } else { 1. });
        } else {
            self.zoom_slide
                .duration(if self.zoom { ZOOM } else { 1. }, ZOOM_TIME, cx);
        }
        cx.notify();
    }
    pub fn set_viewer(&mut self, open: bool, w: &mut Window, cx: &mut Context<Self>) {
        if self.viewer == open {
            return;
        }
        self.lens = None;
        if open {
            self.viewer = true;
            if disabled_motion(cx) {
                self.viewer_open.reset(1.);
            } else {
                self.viewer_open.reset(0.);
                self.viewer_open.duration(1., VIEWER_OPEN, cx);
            }
            self.viewer_close.focus(w, cx);
        } else {
            self.zoom = false;
            self.zoom_slide.reset(1.);
            self.viewer_cursor = None;
            self.main_focus.focus(w, cx);
            self.viewer = false;
            if disabled_motion(cx) {
                self.viewer_open.reset(0.);
            } else {
                // The closing overlay stays mounted until the slide settles
                // at zero (see `viewer_block`); background timers are not
                // reliable on wasm.
                self.viewer_open.duration(0., VIEWER_CLOSE, cx);
            }
        }
        cx.emit(ProductGalleryEvent::Viewer(open));
        cx.notify();
    }
    fn begin(&mut self, x: Pixels, w: &mut Window, cx: &mut Context<Self>) {
        self.main_focus.focus(w, cx);
        self.dragged = false;
        self.drag = Some(Drag {
            start: x,
            origin: self.track.value(cx),
            last: x,
            last_time: cx.background_executor().now(),
            speed: 0.,
            dragged: false,
        });
    }
    fn move_pointer(&mut self, x: Pixels, width: Pixels, cx: &mut Context<Self>) {
        let max = self.photo_count().saturating_sub(1) as f32;
        let Some(drag) = self.drag.as_mut() else {
            return;
        };
        let now = cx.background_executor().now();
        let elapsed = now
            .saturating_duration_since(drag.last_time)
            .as_secs_f32()
            .max(0.001);
        drag.speed = f32::from(x - drag.last) / elapsed;
        drag.last = x;
        drag.last_time = now;
        if (x - drag.start).abs() > px(3.) {
            drag.dragged = true;
        }
        let value = drag.origin - f32::from(x - drag.start) / f32::from(width).max(1.);
        self.track.reset(rubber_band(value, max));
        cx.notify();
    }
    fn end(&mut self, cancelled: bool, cx: &mut Context<Self>) {
        let Some(drag) = self.drag.take() else {
            return;
        };
        self.dragged = drag.dragged;
        let current = self.track.value(cx);
        let speed = if cx
            .background_executor()
            .now()
            .saturating_duration_since(drag.last_time)
            .as_secs_f32()
            > 0.12
        {
            0.
        } else {
            drag.speed
        };
        let max = self.photo_count().saturating_sub(1) as f32;
        let target = if cancelled {
            drag.origin
        } else {
            landing(current, drag.origin, speed, f32::from(self.stage_width())).clamp(0., max)
        };
        let index = target as usize;
        let changed = index != self.index;
        if disabled_motion(cx) {
            self.track.reset(target);
            self.rail.reset(target);
        } else {
            self.track.release(
                target,
                -(speed / f32::from(self.stage_width()).max(1.)),
                TRACK_TIME,
                cx,
            );
            Self::glide(&mut self.rail, target, GLIDE, cx);
        }
        self.index = index;
        if changed {
            cx.emit(ProductGalleryEvent::Index(index));
        }
        cx.notify();
    }
    fn stage_width(&self) -> Pixels {
        let measured = self.container_width.get();
        let width = if measured > px(0.) {
            measured
        } else {
            px(640.)
        };
        let wide = f32::from(width) >= 680.;
        let rail_room = if wide || f32::from(width) >= 520. {
            THUMB_W + 12.
        } else {
            0.
        };
        px((f32::from(width) - rail_room).max(1.))
    }
    fn size_or_zero(&self) -> usize {
        self.size.unwrap_or(0)
    }
}
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
fn picker_row(cx: &App, name: &str, zh_name: &str, value: &str) -> Div {
    let zh = UiLanguage::current(cx) == UiLanguage::Chinese;
    div()
        .flex()
        .items_baseline()
        .gap(px(8.))
        .text_size(px(14.))
        .line_height(px(19.6))
        .child(div().text_color(Paint::get(cx).secondary).child(if zh {
            zh_name.to_string()
        } else {
            name.to_string()
        }))
        .child(
            div()
                .font_weight(FontWeight::MEDIUM)
                .text_color(Paint::get(cx).ink)
                .child(value.to_string()),
        )
}
impl ProductGallery {
    /// Photo stage visuals: crossfade layer, spring track, hover lens.
    fn stage_inner(&mut self, w: &mut Window, cx: &mut Context<Self>) -> Div {
        let p = Paint::get(cx);
        let r = crate::foundation::surface_radius(cx);
        let stage_w = self.stage_width();
        let height = stage_w / 0.8;
        let track = -self.track.get(w, cx) * stage_w;
        let fade = self.fade.get(w, cx).clamp(0., 1.);
        let mut slides = div()
            .flex()
            .w(stage_w * self.photo_count().max(1) as f32)
            .h(height);
        if let Some(color) = self.colors.get(self.color) {
            for photo in &color.photos {
                slides = slides.child(
                    img(photo.image.clone())
                        .w(stage_w)
                        .h(height)
                        .flex_none()
                        .object_fit(ObjectFit::Cover),
                );
            }
        }
        let mut stage = div()
            .relative()
            .w(stage_w)
            .h(height)
            .bg(p.panel)
            .rounded(r)
            .overflow_hidden()
            .when_some(self.previous.clone(), |s, prev| {
                s.child(
                    img(prev)
                        .absolute()
                        .size_full()
                        .object_fit(ObjectFit::Cover),
                )
            })
            .child(
                div()
                    .absolute()
                    .inset_0()
                    .left(track)
                    .opacity(fade)
                    .child(slides),
            );
        if let (Some(current), Some(_)) = (self.current(), self.lens)
            && self.drag.is_none()
        {
            {
                let b = self.bounds.get();
                let lens_cx = self.lens_x.get(w, cx);
                let lens_cy = self.lens_y.get(w, cx);
                let left = (px(lens_cx) - b.origin.x - px(LENS / 2.))
                    .clamp(px(0.), (stage_w - px(LENS)).max(px(0.)));
                let top = (px(lens_cy) - b.origin.y - px(LENS / 2.))
                    .clamp(px(0.), (height - px(LENS)).max(px(0.)));
                let rel_x = (lens_cx - f32::from(b.origin.x)) / f32::from(stage_w).max(1.);
                let rel_y = (lens_cy - f32::from(b.origin.y)) / f32::from(height).max(1.);
                let zoomed = img(current.image.clone())
                    .absolute()
                    .w(stage_w * ZOOM)
                    .h(height * ZOOM)
                    .left(px(LENS / 2.) - stage_w * ZOOM * rel_x)
                    .top(px(LENS / 2.) - height * ZOOM * rel_y)
                    .object_fit(ObjectFit::Cover);
                stage = stage.child(
                    div()
                        .absolute()
                        .left(left)
                        .top(top)
                        .size(px(LENS))
                        .rounded_full()
                        .overflow_hidden()
                        .bg(p.panel)
                        .child(zoomed),
                );
            }
        }
        stage
    }

    /// The stage wrapped as the main photo button with drag and viewer wiring.
    fn main_block(&mut self, w: &mut Window, cx: &mut Context<Self>) -> Stateful<Div> {
        let zh = UiLanguage::current(cx) == UiLanguage::Chinese;
        let stage_w = self.stage_width();
        let alt = self.current().map_or(String::new(), |c| c.alt.to_string());
        let aria = if zh {
            format!(
                "{}，第 {} 张，共 {} 张。打开全屏",
                alt,
                self.index + 1,
                self.photo_count()
            )
        } else {
            format!(
                "{}, {} of {}. Open fullscreen",
                alt,
                self.index + 1,
                self.photo_count()
            )
        };
        let stage = self.stage_inner(w, cx);
        let stage_w_for_move = stage_w;
        let moved = cx.listener(move |s, e: &MouseMoveEvent, _, cx| {
            s.move_pointer(e.position.x, stage_w_for_move, cx)
        });
        let released = cx.listener(|s, _: &MouseUpEvent, _, cx| s.end(false, cx));
        let contact = cx.listener({
            let width = stage_w;
            move |s, e: &TouchDragEvent, w, cx| match e.phase {
                TouchPhase::Started => s.begin(e.position.x, w, cx),
                TouchPhase::Moved => s.move_pointer(e.position.x, width, cx),
                TouchPhase::Ended => s.end(false, cx),
                TouchPhase::Cancelled => s.end(true, cx),
            }
        });
        let active = self.touch.clone();
        let mouse_capture = canvas(
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
        .size(px(0.));
        let touch_capture = canvas(
            |b, w, _| w.insert_hitbox(b, HitboxBehavior::Normal),
            move |_, hit, w, _| {
                w.on_mouse_event(move |e: &TouchDragEvent, phase, w, cx| {
                    if phase.bubble() {
                        if e.phase == TouchPhase::Started {
                            active.set(if hit.is_hovered(w) { 1 } else { 0 });
                        }
                        if active.get() == 0 {
                            return;
                        }
                        if matches!(e.phase, TouchPhase::Ended | TouchPhase::Cancelled) {
                            active.set(0);
                        }
                        if e.phase == TouchPhase::Started {
                            w.prevent_default();
                        }
                        contact(e, w, cx);
                        cx.stop_propagation();
                    }
                });
            },
        )
        .absolute()
        .inset_0();
        div()
            .id("product-stage")
            .track_focus(&self.main_focus)
            .role(Role::Button)
            .aria_label(aria)
            .cursor_pointer()
            .child(stage)
            .child(self.bounds.measure())
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|s, e: &MouseDownEvent, w, cx| s.begin(e.position.x, w, cx)),
            )
            .on_mouse_move(cx.listener(|s, e: &MouseMoveEvent, _, cx| {
                if s.drag.is_none() {
                    s.lens = Some(e.position);
                    if disabled_motion(cx) {
                        s.lens_x.reset(f32::from(e.position.x));
                        s.lens_y.reset(f32::from(e.position.y));
                    } else {
                        let now_x = s.lens_x.value(cx);
                        let now_y = s.lens_y.value(cx);
                        let vx = (f32::from(e.position.x) - now_x).clamp(-900., 900.);
                        let vy = (f32::from(e.position.y) - now_y).clamp(-900., 900.);
                        s.lens_x.release(f32::from(e.position.x), vx, 0.16, cx);
                        s.lens_y.release(f32::from(e.position.y), vy, 0.16, cx);
                    }
                    cx.notify();
                }
            }))
            .on_hover(cx.listener(|s, over: &bool, _, cx| {
                if !*over {
                    s.lens = None;
                }
                cx.notify();
            }))
            .on_click(cx.listener(|s, e: &ClickEvent, w, cx| match e {
                // A click without travel opens the viewer; the click that
                // releases a drag is consumed instead.
                ClickEvent::Mouse(m) if m.up.click_count == 1 => {
                    if s.dragged {
                        s.dragged = false;
                    } else {
                        s.set_viewer(true, w, cx);
                    }
                }
                ClickEvent::Keyboard(_) => s.set_viewer(true, w, cx),
                _ => {}
            }))
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                match e.keystroke.key.as_str() {
                    "left" => s.set_index(s.index.saturating_sub(1), cx),
                    "right" => s.set_index(s.index.saturating_add(1), cx),
                    "enter" | "space" => s.set_viewer(true, w, cx),
                    _ => return,
                }
                cx.stop_propagation();
            }))
            .child(mouse_capture)
            .child(touch_capture)
    }

    /// Thumbnail rail with the gliding selection ring.
    fn rail_block(&mut self, w: &mut Window, cx: &mut Context<Self>) -> Stateful<Div> {
        let p = Paint::get(cx);
        let r = crate::foundation::surface_radius(cx);
        let zh = UiLanguage::current(cx) == UiLanguage::Chinese;
        let mid = f32::from(self.container_width.get().max(px(520.))) >= 520.;
        let rail_pos = self.rail.get(w, cx);
        let indicator = div()
            .absolute()
            .map(|d| {
                if mid {
                    d.left(px(-2.)).top(px(rail_pos * THUMB_STEP - 2.))
                } else {
                    d.left(px(rail_pos * THUMB_STEP - 2.)).top(px(-2.))
                }
            })
            .w(px(THUMB_W + 4.))
            .h(px(THUMB_H + 4.))
            .rounded(px(f32::from(r) + 2.))
            .border_2()
            .border_color(p.ink);
        let mut thumbs = div()
            .id("product-rail")
            .role(Role::TabList)
            .aria_label(if zh { "照片" } else { "Photos" })
            .relative()
            .gap(px(8.))
            .map(|d| {
                if mid {
                    d.flex().flex_col().w(px(THUMB_W)).flex_none()
                } else {
                    d.flex().flex_row().w_full().overflow_x_scroll()
                }
            })
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, _, cx| {
                let len = s.photo_count();
                if len == 0 {
                    return;
                }
                let target = match e.keystroke.key.as_str() {
                    "left" | "up" => s.index.saturating_sub(1),
                    "right" | "down" => s.index.saturating_add(1),
                    "home" => 0,
                    "end" => len - 1,
                    _ => return,
                };
                s.set_index(target.min(len - 1), cx);
                cx.stop_propagation();
            }));
        if let Some(color) = self.colors.get(self.color) {
            for (i, item) in color.photos.iter().enumerate() {
                let alt = item.alt.clone();
                let label = if zh {
                    format!("第 {} 张：{}", i + 1, alt)
                } else {
                    format!("Photo {}: {}", i + 1, alt)
                };
                thumbs = thumbs.child(
                    button(("product-thumb", i), alt.clone(), cx)
                        .w(px(THUMB_W))
                        .h(px(THUMB_H))
                        .flex_none()
                        .p_0()
                        .rounded(r)
                        .overflow_hidden()
                        .opacity(if i == self.index { 1. } else { 0.64 })
                        .hover(|s| s.opacity(1.))
                        .role(Role::Tab)
                        .aria_label(label)
                        .child(
                            img(item.image.clone())
                                .size_full()
                                .object_fit(ObjectFit::Cover),
                        )
                        .on_click(cx.listener(move |s, _, _, cx| s.set_index(i, cx))),
                );
            }
        }
        thumbs.child(indicator)
    }

    /// Media block: rail beside the stage, or under it on narrow columns.
    fn media_block(&mut self, w: &mut Window, cx: &mut Context<Self>) -> Div {
        let mid = f32::from(self.container_width.get().max(px(520.))) >= 520.;
        let main = self.main_block(w, cx);
        let rail = self.rail_block(w, cx);
        if mid {
            div().flex().gap(px(12.)).child(rail).child(main)
        } else {
            div().flex().flex_col().gap(px(12.)).child(main).child(rail)
        }
    }

    /// Color swatch radiogroup with a gliding ring.
    fn color_block(&mut self, w: &mut Window, cx: &mut Context<Self>) -> Stateful<Div> {
        let p = Paint::get(cx);
        let swatch_pos = self.swatch.get(w, cx);
        let ring = div()
            .absolute()
            .left(px(swatch_pos * SWATCH_STEP - 2.))
            .top(px(-2.))
            .size(px(SWATCH + 4.))
            .rounded_full()
            .border_2()
            .border_color(p.ink);
        let mut swatches = div()
            .id("product-colors")
            .role(Role::RadioGroup)
            .relative()
            .flex()
            .gap(px(12.))
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, _, cx| {
                let len = s.colors.len();
                if len == 0 {
                    return;
                }
                let delta: i32 = match e.keystroke.key.as_str() {
                    "left" | "up" => -1,
                    "right" | "down" => 1,
                    "home" => -(s.color as i32),
                    "end" => len as i32 - 1 - s.color as i32,
                    _ => return,
                };
                let target = (s.color as i32 + delta).clamp(0, len as i32 - 1) as usize;
                let id = s.colors[target].id.clone();
                s.set_color(&id, cx);
                cx.stop_propagation();
            }));
        for (ci, color) in self.colors.iter().enumerate() {
            let id = color.id.clone();
            let selected = ci == self.color;
            swatches = swatches.child(
                button(("product-color", ci), color.label.clone(), cx)
                    .size(px(SWATCH))
                    .p_0()
                    .rounded_full()
                    .overflow_hidden()
                    .role(Role::RadioButton)
                    .aria_toggled(selected.into())
                    .child(div().size_full().rounded_full().bg(color.swatch))
                    .on_click(cx.listener(move |s, _, _, cx| s.set_color(&id, cx))),
            );
        }
        swatches.child(ring)
    }

    /// Size radiogroup with a gliding fill; sold-out sizes strike through.
    fn size_block(&mut self, w: &mut Window, cx: &mut Context<Self>) -> Stateful<Div> {
        let p = Paint::get(cx);
        let r = crate::foundation::surface_radius(cx);
        let size_pos = self.size_fill.get(w, cx);
        let has_size = self.size.is_some();
        let fill = div()
            .absolute()
            .left(px(size_pos.max(0.) * SIZE_STEP))
            .top_0()
            .w(px(SIZE_W))
            .h(px(SIZE_H))
            .rounded(px(4.))
            .bg(p.secondary)
            .opacity(if has_size { 1. } else { 0. });
        let mut sizes = div()
            .id("product-sizes")
            .role(Role::RadioGroup)
            .relative()
            .flex()
            .gap(px(8.))
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, _, cx| {
                let len = s.sizes.len();
                if len == 0 {
                    return;
                }
                let forward = matches!(e.keystroke.key.as_str(), "right" | "down" | "end");
                let mut target = match e.keystroke.key.as_str() {
                    "left" | "up" => s.size_or_zero().saturating_sub(1),
                    "right" | "down" => (s.size_or_zero() + 1).min(len - 1),
                    "home" => 0,
                    "end" => len - 1,
                    _ => return,
                };
                // Roving: arrow/Home/End skip sizes sold out in this color.
                while target < len && s.sold_out_in_color(&s.sizes[target]) {
                    if forward {
                        if target + 1 >= len {
                            break;
                        }
                        target += 1;
                    } else {
                        if target == 0 {
                            break;
                        }
                        target -= 1;
                    }
                }
                if target < len && !s.sold_out_in_color(&s.sizes[target]) {
                    let id = s.sizes[target].id.clone();
                    s.set_size(&id, cx);
                }
                cx.stop_propagation();
            }));
        for (si, size) in self.sizes.iter().enumerate() {
            let id = size.id.clone();
            let unavailable = self.sold_out_in_color(size);
            let selected = self.size == Some(si);
            sizes = sizes.child(
                button(("product-size", si), size.label.clone(), cx)
                    .w(px(SIZE_W))
                    .h(px(SIZE_H))
                    .p_0()
                    .rounded(r)
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(14.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(if unavailable {
                        p.faint
                    } else if selected {
                        p.inverse
                    } else {
                        p.ink
                    })
                    .role(Role::RadioButton)
                    .aria_toggled(selected.into())
                    .when(unavailable, |b| b.line_through().cursor_default())
                    .child(size.label.clone())
                    .on_click(cx.listener(move |s, _, _, cx| {
                        if !s.sold_out_in_color(&s.sizes[si]) {
                            s.set_size(&id, cx);
                        }
                    })),
            );
        }
        sizes.child(fill)
    }

    /// Details column: header, both pickers, footer, sold-out meta.
    fn details_block(&mut self, w: &mut Window, cx: &mut Context<Self>) -> Div {
        let p = Paint::get(cx);
        let zh = UiLanguage::current(cx) == UiLanguage::Chinese;
        let colors = self.color_block(w, cx);
        let color_value = self
            .colors
            .get(self.color)
            .map(|c| c.label.to_string())
            .unwrap_or_default();
        let sizes = self.size_block(w, cx);
        let size_value = match self.size {
            Some(i) => self
                .sizes
                .get(i)
                .map_or(String::new(), |s| s.label.to_string()),
            None if zh => "选择尺码".to_string(),
            None => "Select a size".to_string(),
        };
        let sold_out_sizes: Vec<String> = self
            .sizes
            .iter()
            .filter(|s| self.sold_out_in_color(s))
            .map(|s| s.label.to_string())
            .collect();
        let color_label = self
            .colors
            .get(self.color)
            .map_or(String::new(), |c| c.label.to_string());
        let meta = (!sold_out_sizes.is_empty()).then(|| {
            div()
                .text_size(px(12.))
                .line_height(px(16.8))
                .text_color(p.faint)
                .child(if zh {
                    format!("{}色缺货：{}", color_label, sold_out_sizes.join("、"))
                } else {
                    format!(
                        "Sold out in {}: {}",
                        color_label.to_lowercase(),
                        sold_out_sizes.join(", ")
                    )
                })
        });
        let mut details = div().flex().flex_col().gap(px(24.)).min_w_0();
        if let Some(header) = self.header.as_ref() {
            details = details.child(header(w, cx));
        }
        details = details
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(10.))
                    .child(picker_row(cx, "Color", "颜色", &color_value))
                    .child(colors),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(10.))
                    .child(picker_row(cx, "Size", "尺码", &size_value))
                    .child(sizes),
            );
        if let Some(footer) = self.footer.as_ref() {
            details = details.child(footer(w, cx));
        }
        if let Some(meta) = meta {
            details = details.child(meta);
        }
        details
    }

    /// Fullscreen viewer overlay, growing out of the photo's own rectangle.
    fn viewer_block(&mut self, w: &mut Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let current = self.current()?;
        let p = Paint::get(cx);
        let r = crate::foundation::surface_radius(cx);
        let zh = UiLanguage::current(cx) == UiLanguage::Chinese;
        let image = current.image.clone();
        let window = w.viewport_size();
        let t = self.viewer_open.get(w, cx).clamp(0., 1.);
        let pad = if f32::from(window.height) < 640. {
            12.
        } else {
            48.
        };
        // Contain a 4/5 photo inside the padded viewport: width follows height.
        let full_w = (window.width - px(pad * 2.))
            .max(px(1.))
            .min((window.height - px(pad * 2.)).max(px(1.) / 0.8) * 0.8);
        let full_h = full_w / 0.8;
        let b = self.bounds.get();
        let left = b.origin.x + (window.width / 2. - full_w / 2. - b.origin.x) * t;
        let top = b.origin.y + (window.height / 2. - full_h / 2. - b.origin.y) * t;
        let fw = b.size.width + (full_w - b.size.width) * t;
        let fh = b.size.height + (full_h - b.size.height) * t;
        let zoom = self.zoom_slide.get(w, cx).max(1.);
        let overflow_x = (fw * zoom - fw).max(px(0.));
        let overflow_y = (fh * zoom - fh).max(px(0.));
        let cursor = self
            .viewer_cursor
            .unwrap_or(point(window.width / 2., window.height / 2.));
        let pan_x =
            overflow_x * (0.5 - (f32::from(cursor.x - left) / f32::from(fw).max(1.)).clamp(0., 1.));
        let pan_y =
            overflow_y * (0.5 - (f32::from(cursor.y - top) / f32::from(fh).max(1.)).clamp(0., 1.));
        let counter = div()
            .absolute()
            .top(top + px(20.))
            .left(left + px(24.))
            .text_size(px(14.))
            .line_height(px(20.))
            .font_weight(FontWeight::MEDIUM)
            .text_color(p.ink)
            .opacity(t)
            .child(format!("{} / {}", self.index + 1, self.photo_count()));
        let hint = div()
            .absolute()
            .bottom(window.height - (top + fh) + px(16.))
            .left(left)
            .w(fw)
            .flex()
            .justify_center()
            .text_size(px(12.))
            .line_height(px(16.))
            .text_color(p.faint)
            .opacity(t)
            .child(if self.zoom {
                if zh {
                    "移动查看，点击缩小".to_string()
                } else {
                    "Move to pan, click to zoom out".to_string()
                }
            } else if zh {
                "点击图片放大".to_string()
            } else {
                "Click the photo to zoom".to_string()
            });
        let close = button("product-viewer-close", "Close", cx)
            .track_focus(&self.viewer_close)
            .absolute()
            .top(top + px(12.))
            .right(window.width - (left + fw) + px(16.))
            .size(px(36.))
            .rounded_full()
            .p_0()
            .bg(p.panel)
            .border_1()
            .border_color(p.line)
            .hover(|s| s.bg(p.tray))
            .opacity(t)
            .child(icon(Glyph::Close, 16., p.ink))
            .on_click(cx.listener(|s, _, w, cx| s.set_viewer(false, w, cx)));
        let photo_hit = div()
            .id("product-viewer-photo")
            .role(Role::Button)
            .aria_label(if zh { "切换缩放" } else { "Toggle zoom" })
            .tab_index(0)
            .cursor_pointer()
            .absolute()
            .left(left)
            .top(top)
            .w(fw)
            .h(fh)
            .on_click(cx.listener(|s, _, _, cx| s.toggle_zoom(cx)));
        let frame = div()
            .absolute()
            .left(left)
            .top(top)
            .w(fw)
            .h(fh)
            .rounded(r)
            .overflow_hidden()
            .opacity(t.max(0.04))
            .child(
                img(image)
                    .relative()
                    .left(pan_x)
                    .top(pan_y)
                    .w(fw * zoom)
                    .h(fh * zoom)
                    .object_fit(ObjectFit::Cover),
            );
        let overlay = div()
            .id("product-viewer")
            .track_focus(&self.viewer_focus)
            .role(Role::Dialog)
            .aria_label(self.label.clone())
            .relative()
            .w(window.width)
            .h(window.height)
            .bg(p.stage.opacity(0.94 * t))
            .overflow_hidden()
            .occlude()
            .on_mouse_move(cx.listener(|s, e: &MouseMoveEvent, _, cx| {
                if s.zoom {
                    s.viewer_cursor = Some(e.position);
                    cx.notify();
                }
            }))
            .on_key_down(cx.listener(
                |s, e: &KeyDownEvent, w, cx| match e.keystroke.key.as_str() {
                    "escape" => {
                        if s.zoom {
                            s.toggle_zoom(cx);
                        } else {
                            s.set_viewer(false, w, cx);
                        }
                    }
                    "tab" => {
                        if e.keystroke.modifiers.shift {
                            w.focus_prev(cx);
                        } else {
                            w.focus_next(cx);
                        }
                        if !s.viewer_focus.contains_focused(w, cx) {
                            s.viewer_close.focus(w, cx);
                        }
                        cx.stop_propagation();
                    }
                    "left" => s.set_index(s.index.saturating_sub(1), cx),
                    "right" => s.set_index(s.index.saturating_add(1), cx),
                    _ => {}
                },
            ))
            .child(frame)
            .child(photo_hit)
            .child(counter)
            .child(hint)
            .child(close);
        let anchored_overlay = anchored().position(point(px(0.), px(0.))).child(overlay);
        Some(
            deferred(anchored_overlay)
                .with_priority(5)
                .into_any_element(),
        )
    }
}
impl Render for ProductGallery {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let measured = self.container_width.get();
        let width = if measured > px(0.) {
            measured
        } else {
            w.viewport_size().width - px(32.)
        }
        .max(px(1.));
        let wide = f32::from(width) >= 680.;
        let media = self.media_block(w, cx);
        let details = self.details_block(w, cx);
        let mut root = div()
            .id("product-gallery")
            .w(width)
            .max_w_full()
            .flex()
            .gap(px(24.))
            .map(|d| if wide { d.flex_row() } else { d.flex_col() })
            .child(media)
            .child(details);
        // While closing, keep the overlay mounted until its slide settles at
        // zero; the same value check inside `viewer_block` retires it.
        if (self.viewer || self.viewer_open.value(cx) > 0.001)
            && let Some(viewer) = self.viewer_block(w, cx)
        {
            root = root.child(viewer);
        }
        constrained(root, self.container_width.clone(), px(940.))
    }
}
#[cfg(test)]
mod tests {
    use super::{landing, rubber_band};

    #[test]
    fn rubber_band_compresses_past_edges() {
        assert_eq!(rubber_band(-1.0, 3.0), -0.35);
        assert_eq!(rubber_band(-2.0, 3.0), -0.45);
        assert_eq!(rubber_band(3.4, 3.0), 3.0 + 0.4 * 0.35);
        assert_eq!(rubber_band(4.4, 3.0), 3.45);
        assert_eq!(rubber_band(1.7, 3.0), 1.7);
    }

    #[test]
    fn flick_advances_one_photo() {
        let step = 564.;
        // Slow release near the start stays put.
        assert_eq!(landing(0.1, 0.0, 40., step), 0.);
        // A fast leftward flick from the first photo moves to the second.
        assert_eq!(landing(0.2, 0.0, -600., step), 1.);
        // A fast rightward flick falls back one photo.
        assert_eq!(landing(1.6, 2.0, 600., step), 1.);
        // Momentum without flick rounds to the nearest photo.
        assert_eq!(landing(0.6, 0.0, 100., step), 1.);
    }

    #[test]
    fn landing_clamps_inside_track() {
        assert_eq!(landing(-0.4, 0.0, -900., 564.), 1.);
    }
}
