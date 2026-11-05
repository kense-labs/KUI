//! A product photo gallery with color and size selection.
use crate::primitives::{
    family::{Paint, Slide},
    flyout::TriggerBounds,
    glyph::{Glyph, icon},
    nav::{button, tr},
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
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
    lens: Option<Point<Pixels>>,
    stage_hover: bool,
    bounds: TriggerBounds,
    zoom: bool,
    zoom_slide: Slide,
    viewer_cursor: Option<Point<Pixels>>,
    main_focus: FocusHandle,
    viewer_focus: FocusHandle,
    viewer_close: FocusHandle,
    viewer: bool,
    container_width: Rc<Cell<Pixels>>,
    // 0: idle, 1: undecided, 2: horizontal, 3: vertical.
    touch: Rc<Cell<u8>>,
    touch_start: Rc<Cell<Pixels>>,
    touch_y: Rc<Cell<Pixels>>,
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
            lens: None,
            stage_hover: false,
            bounds: TriggerBounds::default(),
            zoom: false,
            zoom_slide: Slide::new(1.),
            viewer_cursor: None,
            main_focus: cx.focus_handle().tab_stop(true),
            viewer_focus: cx.focus_handle(),
            viewer_close: cx.focus_handle().tab_stop(true),
            viewer: false,
            container_width: Default::default(),
            touch: Default::default(),
            touch_start: Rc::new(Cell::new(px(0.))),
            touch_y: Rc::new(Cell::new(px(0.))),
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
    pub fn set_index(&mut self, index: usize, cx: &mut Context<Self>) {
        let len = self.colors.get(self.color).map_or(0, |v| v.photos.len());
        if len == 0 {
            return;
        }
        let index = index.min(len - 1);
        if self.index == index {
            return;
        }
        self.previous = self.current().map(|p| p.image.clone());
        self.index = index;
        self.fade.reset(0.);
        self.fade.duration(1., 0.24, cx);
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
        self.fade.duration(1., 0.24, cx);
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
        cx.emit(ProductGalleryEvent::Size(Some(
            self.sizes[index].id.clone(),
        )));
        cx.notify();
    }
    fn toggle_zoom(&mut self, cx: &mut Context<Self>) {
        self.zoom = !self.zoom;
        self.zoom_slide
            .duration(if self.zoom { 2.4 } else { 1. }, 0.42, cx);
        cx.notify();
    }
    pub fn set_viewer(&mut self, open: bool, w: &mut Window, cx: &mut Context<Self>) {
        if self.viewer == open {
            return;
        }
        self.viewer = open;
        self.lens = None;
        if !open {
            self.zoom = false;
            self.zoom_slide.reset(1.);
            self.viewer_cursor = None;
        }
        if open {
            self.viewer_close.focus(w, cx)
        } else {
            self.main_focus.focus(w, cx)
        }
        cx.emit(ProductGalleryEvent::Viewer(open));
        cx.notify();
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
impl Render for ProductGallery {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let r = crate::foundation::surface_radius(cx);
        let measured = self.container_width.get();
        let width = if measured > px(0.) {
            measured
        } else {
            w.viewport_size().width - px(32.)
        }
        .max(px(1.));
        let wide = f32::from(width) >= 680.;
        let stage_w = if wide {
            (width - px(76.)).max(px(1.))
        } else {
            width
        };
        let height = stage_w / 0.8;
        let stage_radius = if wide { 22. } else { 18. };
        let fade = self.fade.get(w, cx).clamp(0., 1.);
        let mut photo = div()
            .relative()
            .w(stage_w)
            .h(height)
            .bg(p.panel)
            .rounded(px(stage_radius))
            .overflow_hidden();
        if let Some(prev) = self.previous.clone() {
            photo = photo.child(
                img(prev)
                    .absolute()
                    .size_full()
                    .object_fit(ObjectFit::Cover),
            );
        }
        if let Some(current) = self.current() {
            photo = photo.child(
                img(current.image.clone())
                    .absolute()
                    .size_full()
                    .object_fit(ObjectFit::Cover)
                    .opacity(fade),
            );
        }
        if let (Some(current), Some(cursor)) = (self.current(), self.lens) {
            let b = self.bounds.get();
            let x = (f32::from(cursor.x - b.origin.x) / f32::from(stage_w)).clamp(0., 1.);
            let y = (f32::from(cursor.y - b.origin.y) / f32::from(height)).clamp(0., 1.);
            photo = photo.child(
                div()
                    .absolute()
                    .size(px(176.))
                    .left(
                        (cursor.x - b.origin.x - px(88.))
                            .clamp(px(0.), (stage_w - px(176.)).max(px(0.))),
                    )
                    .top(
                        (cursor.y - b.origin.y - px(88.))
                            .clamp(px(0.), (height - px(176.)).max(px(0.))),
                    )
                    .rounded_full()
                    .overflow_hidden()
                    .shadow_lg()
                    .border_2()
                    .border_color(white().opacity(0.72))
                    .child(
                        img(current.image.clone())
                            .absolute()
                            .w(stage_w * 2.4)
                            .h(height * 2.4)
                            .left(px(88.) - stage_w * 2.4 * x)
                            .top(px(88.) - height * 2.4 * y)
                            .object_fit(ObjectFit::Cover),
                    ),
            );
        }
        let touch = self.touch.clone();
        let touch_x = self.touch_start.clone();
        let touch_y = self.touch_y.clone();
        let swipe = cx.listener(|s, direction: &i8, _, cx| {
            let next = if *direction < 0 {
                s.index.saturating_add(1)
            } else {
                s.index.saturating_sub(1)
            };
            s.set_index(next, cx);
        });
        photo = photo
            .child(
                div()
                    .absolute()
                    .inset_0()
                    .rounded(px(stage_radius))
                    .border_1()
                    .border_color(p.ink.opacity(0.06)),
            )
            .when(self.stage_hover && self.lens.is_none(), |photo| {
                photo.child(
                    div()
                        .absolute()
                        .top(px(12.))
                        .right(px(12.))
                        .size(px(34.))
                        .rounded_full()
                        .bg(p.panel.opacity(0.82))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(icon(Glyph::ZoomIn, 16., p.ink)),
                )
            })
            .child(self.bounds.measure())
            .child(
            canvas(
                |b, w, _| w.insert_hitbox(b, HitboxBehavior::Normal),
                move |_, hit, w, _| {
                    w.on_mouse_event(move |e: &ScrollEvent, phase, w, cx| {
                        if !phase.capture() || e.source != ScrollSource::Touch {
                            return;
                        }
                        if e.touch_phase == TouchPhase::Started {
                            touch.set(if hit.is_hovered_at(e.position, w) {
                                1
                            } else {
                                0
                            });
                            touch_x.set(px(0.));
                            touch_y.set(px(0.));
                        }
                        if touch.get() == 0 {
                            return;
                        }
                        let delta = e.delta.pixel_delta(px(16.));
                        touch_x.set(touch_x.get() + delta.x);
                        touch_y.set(touch_y.get() + delta.y);
                        if touch.get() == 1
                            && (touch_x.get().abs() > px(4.) || touch_y.get().abs() > px(4.))
                        {
                            touch.set(if touch_x.get().abs() > touch_y.get().abs() {
                                2
                            } else {
                                3
                            });
                        }
                        if touch.get() == 2 {
                            if e.touch_phase == TouchPhase::Ended && touch_x.get().abs() > px(40.) {
                                let direction = if touch_x.get() < px(0.) { -1 } else { 1 };
                                swipe(&direction, w, cx);
                            }
                            w.prevent_default();
                            cx.stop_propagation();
                        }
                        if matches!(e.touch_phase, TouchPhase::Ended | TouchPhase::Cancelled) {
                            touch.set(0);
                        }
                    });
                },
            )
            .absolute()
            .inset_0(),
        );
        let mut thumbs = div()
            .id("product-thumbnails")
            .flex()
            .gap(px(8.))
            .when(wide, |d| d.flex_col().w(px(64.)).flex_none())
            .when(!wide, |d| d.w_full().overflow_x_scroll());
        if let Some(color) = self.colors.get(self.color) {
            for (i, item) in color.photos.iter().enumerate() {
                thumbs = thumbs.child(
                    button(("product-thumb", i), item.alt.clone(), cx)
                        .w(px(if wide { 64. } else { 56. }))
                        .h(px(if wide { 80. } else { 70. }))
                        .flex_none()
                        .p_0()
                        .rounded(crate::foundation::surface_radius(cx))
                        .overflow_hidden()
                        .opacity(0.64)
                        .hover(|s| s.opacity(1.))
                        .when(i == self.index, |b| b.border_2().border_color(p.ink))
                        .child(
                            img(item.image.clone())
                                .size_full()
                                .object_fit(ObjectFit::Cover),
                        )
                        .on_click(cx.listener(move |s, _, _, cx| s.set_index(i, cx))),
                );
            }
        }
        let mut colors = div().flex().gap(px(12.));
        for (color_index, color) in self.colors.iter().enumerate() {
            let id = color.id.clone();
            colors = colors.child(
                button(("product-color", color_index), color.label.clone(), cx)
                    .size(px(32.))
                    .p_0()
                    .rounded_full()
                    .border_2()
                    .border_color(if self.colors[self.color].id == color.id {
                        p.ink
                    } else {
                        gpui::transparent_black()
                    })
                    .child(div().size_full().rounded_full().bg(color.swatch))
                    .on_click(cx.listener(move |s, _, _, cx| s.set_color(&id, cx))),
            );
        }
        let mut sizes = div().flex().gap(px(8.));
        for (size_index, size) in self.sizes.iter().enumerate() {
            let id = size.id.clone();
            let unavailable = self
                .colors
                .get(self.color)
                .is_some_and(|c| size.sold_out_for.contains(&c.id));
            sizes = sizes.child(
                button(("product-size", size_index), size.label.clone(), cx)
                    .h(px(44.))
                    .min_w(px(76.))
                    .px(px(10.))
                    .rounded(crate::foundation::surface_radius(cx))
                    .border_1()
                    .when(unavailable, |b| {
                        b.bg(p.tray).border_color(gpui::transparent_black()).text_color(p.faint)
                    })
                    .when(
                        !unavailable && self.size.is_some_and(|i| self.sizes[i].id == size.id),
                        |b| {
                            b.bg(p.bright)
                                .border_color(gpui::transparent_black())
                                .text_color(p.inverse)
                        },
                    )
                    .when(
                        !unavailable && !self.size.is_some_and(|i| self.sizes[i].id == size.id),
                        |b| b.border_color(p.line).hover(|s| s.border_color(p.faint)),
                    )
                    .child(
                        div()
                            .when(unavailable, |d| d.line_through())
                            .child(size.label.clone()),
                    )
                    .on_click(cx.listener(move |s, _, _, cx| s.set_size(&id, cx))),
            );
        }
        let mut root = div()
            .id("product-gallery")
            .role(Role::Region)
            .aria_label(self.label.clone())
            .w(width)
            .max_w_full()
            .flex()
            .flex_col()
            .gap(px(if wide { 32. } else { 24. }))
            .child({
                let main = div()
                    .id("product-main")
                    .flex_1()
                    .track_focus(&self.main_focus)
                    .role(Role::Button)
                    .aria_label(self.label.clone())
                    .cursor_pointer()
                    .on_key_down(cx.listener(|s, e: &KeyDownEvent, _, cx| {
                        match e.keystroke.key.as_str() {
                            "left" => s.set_index(s.index.saturating_sub(1), cx),
                            "right" => s.set_index(s.index.saturating_add(1), cx),
                            _ => return,
                        }
                        cx.stop_propagation();
                    }))
                    .on_mouse_move(cx.listener(|s, e: &MouseMoveEvent, _, cx| {
                        s.stage_hover = true;
                        s.lens = Some(e.position);
                        cx.notify();
                    }))
                    .on_hover(cx.listener(|s, over: &bool, _, cx| {
                        s.stage_hover = *over;
                        if !*over {
                            s.lens = None;
                        }
                        cx.notify();
                    }))
                    .on_click(cx.listener(|s, _, w, cx| s.set_viewer(true, w, cx)))
                    .child(photo);
                if wide {
                    div()
                        .flex()
                        .flex_row()
                        .items_start()
                        .gap(px(12.))
                        .child(thumbs)
                        .child(main)
                } else {
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(12.))
                        .child(main)
                        .child(thumbs)
                }
            })
            .children(self.header.as_ref().map(|header| header(w, cx)))
            .child(colors)
            .child(sizes)
            .children(self.footer.as_ref().map(|footer| footer(w, cx)));
        if self.viewer
            && let Some(photo) = self.current()
        {
            let image = photo.image.clone();
            let window = w.viewport_size();
            let zoom = self.zoom_slide.get(w, cx).max(1.);
            let viewer_width = (window.width - px(48.))
                .max(px(1.))
                .min((window.height - px(64.)).max(px(1.)) * 0.8);
            let viewer_height = viewer_width * 1.25;
            let overflow_x = (viewer_width * zoom - viewer_width).max(px(0.));
            let overflow_y = (viewer_height * zoom - viewer_height).max(px(0.));
            let cursor = self
                .viewer_cursor
                .unwrap_or(point(window.width / 2., window.height / 2.));
            let pan_x = overflow_x
                * (0.5
                    - (f32::from(cursor.x - (window.width - viewer_width) / 2.)
                        / f32::from(viewer_width).max(1.))
                    .clamp(0., 1.));
            let pan_y = overflow_y
                * (0.5
                    - (f32::from(cursor.y - (window.height - viewer_height) / 2.)
                        / f32::from(viewer_height).max(1.))
                    .clamp(0., 1.));
            root = root.child(
                deferred(
                    anchored().position(point(px(0.), px(0.))).child(
                        div()
                            .id("product-viewer")
                            .track_focus(&self.viewer_focus)
                            .role(Role::Dialog)
                            .aria_label(self.label.clone())
                            .w(window.width)
                            .h(window.height)
                            .bg(p.stage.opacity(0.92))
                            .flex()
                            .items_center()
                            .justify_center()
                            .overflow_hidden()
                            .occlude()
                            .on_mouse_move(cx.listener(|s, e: &MouseMoveEvent, _, cx| {
                                if s.zoom {
                                    s.viewer_cursor = Some(e.position);
                                    cx.notify();
                                }
                            }))
                            .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                                match e.keystroke.key.as_str() {
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
                                }
                            }))
                            .child(
                                div()
                                    .id("product-viewer-image")
                                    .role(Role::Button)
                                    .aria_label(tr(cx, "Toggle zoom", "切换缩放"))
                                    .tab_index(0)
                                    .cursor_pointer()
                                    .w(viewer_width)
                                    .h(viewer_height)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .overflow_hidden()
                                    .rounded(r)
                                    .on_click(cx.listener(|s, _, _, cx| {
                                        s.toggle_zoom(cx);
                                    }))
                                    .child(
                                        img(image)
                                            .relative()
                                            .left(pan_x)
                                            .top(pan_y)
                                            .w(viewer_width * zoom)
                                            .h(viewer_height * zoom)
                                            .object_fit(ObjectFit::Contain),
                                    ),
                            )
                            .child(
                                div()
                                    .absolute()
                                    .top(px(22.))
                                    .left(px(24.))
                                    .text_size(px(14.))
                                    .line_height(px(20.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .child(format!(
                                        "{} / {}",
                                        self.index + 1,
                                        self.colors.get(self.color).map_or(0, |c| c.photos.len())
                                    )),
                            )
                            .child(
                                div()
                                    .absolute()
                                    .top(px(24.))
                                    .text_size(px(12.))
                                    .line_height(px(16.))
                                    .text_color(p.faint)
                                    .child(if self.zoom {
                                        tr(cx, "Move to pan, click to zoom out", "移动查看，点击缩小")
                                    } else {
                                        tr(cx, "Click the photo to zoom", "点击图片放大")
                                    }),
                            )
                            .child(
                                div()
                                    .absolute()
                                    .bottom(px(16.))
                                    .flex()
                                    .items_center()
                                    .gap(px(14.))
                                    .child(
                                        button(
                                            "product-prev",
                                            tr(cx, "Previous photo", "上一张"),
                                            cx,
                                        )
                                        .size(px(40.))
                                        .rounded_full()
                                        .p_0()
                                        .bg(p.panel)
                                        .border_1()
                                        .border_color(p.line)
                                        .hover(|s| s.bg(p.tray))
                                        .child("‹")
                                        .on_click(cx.listener(|s, _, _, cx| {
                                            s.set_index(s.index.saturating_sub(1), cx)
                                        })),
                                    )
                                    .child(
                                        button("product-next", tr(cx, "Next photo", "下一张"), cx)
                                            .size(px(40.))
                                            .rounded_full()
                                            .p_0()
                                            .bg(p.panel)
                                            .border_1()
                                            .border_color(p.line)
                                            .hover(|s| s.bg(p.tray))
                                            .child("›")
                                            .on_click(cx.listener(|s, _, _, cx| {
                                                s.set_index(s.index.saturating_add(1), cx)
                                            })),
                                    ),
                            )
                            .child(
                                button("product-viewer-close", "Close", cx)
                                    .track_focus(&self.viewer_close)
                                    .absolute()
                                    .top(px(12.))
                                    .right(px(16.))
                                    .size(px(40.))
                                    .rounded_full()
                                    .p_0()
                                    .bg(p.panel)
                                    .border_1()
                                    .border_color(p.line)
                                    .hover(|s| s.bg(p.tray))
                                    .child(icon(Glyph::Close, 16., p.ink))
                                    .on_click(
                                        cx.listener(|s, _, w, cx| s.set_viewer(false, w, cx)),
                                    ),
                            ),
                    ),
                )
                .with_priority(5),
            );
        }
        constrained(root, self.container_width.clone(), px(940.))
    }
}
