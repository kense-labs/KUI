//! Responsive masonry gallery and a keyboard/gesture-operated image viewer.
use crate::primitives::{
    family::{Paint, Slide},
    flyout::TriggerBounds,
    glyph::{Glyph, icon},
    nav::*,
};
use gpui::{prelude::*, *};
pub struct GalleryPhoto {
    pub image: ImageSource,
    pub title: SharedString,
    pub caption: SharedString,
    pub ratio: f32,
}
impl GalleryPhoto {
    pub fn new(
        image: impl Into<ImageSource>,
        title: impl Into<SharedString>,
        caption: impl Into<SharedString>,
        ratio: f32,
    ) -> Self {
        Self {
            image: image.into(),
            title: title.into(),
            caption: caption.into(),
            ratio: if ratio.is_finite() {
                ratio.clamp(0.25, 4.)
            } else {
                1.
            },
        }
    }
}
pub struct GallerySelection(pub Option<usize>);
pub struct PhotoExhibit {
    photos: Vec<GalleryPhoto>,
    targets: Vec<(FocusHandle, TriggerBounds)>,
    active: Option<usize>,
    closing: bool,
    show: Slide,
    fade: Fade,
    slide: Slide,
    previous: Option<(usize, f32)>,
    zoom: f32,
    offset: Point<Pixels>,
    press: Option<(Point<Pixels>, Point<Pixels>)>,
    dialog: FocusHandle,
}
impl EventEmitter<GallerySelection> for PhotoExhibit {}
impl PhotoExhibit {
    pub fn new(photos: Vec<GalleryPhoto>, cx: &App) -> Self {
        let targets = photos
            .iter()
            .map(|_| (cx.focus_handle().tab_stop(true), TriggerBounds::default()))
            .collect();
        Self {
            photos,
            targets,
            active: None,
            closing: false,
            show: Slide::new(0.),
            fade: Default::default(),
            slide: Slide::new(1.),
            previous: None,
            zoom: 1.,
            offset: Default::default(),
            press: None,
            dialog: cx.focus_handle().tab_stop(true),
        }
    }
    pub fn open(&mut self, index: usize, w: &mut Window, cx: &mut Context<Self>) {
        if index >= self.photos.len() {
            return;
        }
        self.active = Some(index);
        self.previous = None;
        self.fade.set(true, 0.15, cx);
        self.closing = false;
        self.zoom = 1.;
        self.offset = Default::default();
        self.show.duration(1., 0.32, cx);
        self.dialog.focus(w, cx);
        cx.emit(GallerySelection(Some(index)));
        cx.notify()
    }
    pub fn close(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if let Some(i) = self.active {
            self.closing = true;
            self.fade.set(false, 0.10, cx);
            self.press = None;
            self.zoom = 1.;
            self.offset = Default::default();
            self.show.duration(0., 0.22, cx);
            self.targets[i].0.focus(w, cx);
            cx.emit(GallerySelection(None));
            cx.notify()
        }
    }
    fn step(&mut self, delta: isize, w: &mut Window, cx: &mut Context<Self>) {
        if let Some(i) = self.active {
            let n = self.photos.len();
            let next = (i as isize + delta).rem_euclid(n as isize) as usize;
            if next == i {
                return;
            }
            let start = if self.previous.is_some_and(|(from, _)| from == next) {
                1. - self.slide.value(cx)
            } else {
                0.
            };
            self.open(next, w, cx);
            self.previous = Some((i, delta.signum() as f32));
            self.slide.reset(start);
            self.slide.duration(1., 0.30, cx);
        }
    }
}
impl Render for PhotoExhibit {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let radius = crate::foundation::surface_radius(cx);
        let width = fit(w, 522.);
        let cols = if width < px(340.) { 2 } else { 3 };
        let col = (width - px(8. * (cols - 1) as f32)) / cols as f32;
        let mut grid = row().items_start().gap(px(8.)).w(width);
        let mut slots = vec![Vec::new(); cols];
        let mut heights = vec![0.0_f32; cols];
        for (i, photo) in self.photos.iter().enumerate() {
            let c = heights
                .iter()
                .enumerate()
                .min_by(|a, b| a.1.total_cmp(b.1))
                .map(|(i, _)| i)
                .unwrap_or(0);
            heights[c] += f32::from(col) / photo.ratio + 8.;
            slots[c].push(i)
        }
        for slot in slots {
            let mut stack = column().w(col).gap(px(8.));
            for i in slot {
                let v = &self.photos[i];
                stack = stack.child(
                    button(("photo", i), v.title.clone(), cx)
                        .track_focus(&self.targets[i].0)
                        .relative()
                        .w(col)
                        .h(col / v.ratio)
                        .p_0()
                        .overflow_hidden()
                        .rounded(radius)
                        .bg(p.hover)
                        .child(
                            img(v.image.clone())
                                .size_full()
                                .rounded(radius)
                                .object_fit(ObjectFit::Cover),
                        )
                        .child(self.targets[i].1.measure())
                        .on_click(cx.listener(move |s, _, w, cx| s.open(i, w, cx))),
                )
            }
            grid = grid.child(stack)
        }
        let mut root = div().child(grid);
        let Some(i) = self.active else { return root };
        let geometry = self.show.get(w, cx).clamp(0., 1.);
        let fade = self.fade.get(w, cx);
        let still = crate::primitives::family::disabled_motion(cx);
        let a = if still { 1. } else { geometry };
        if self.closing && geometry < 0.002 && (!still || fade < 0.001) {
            self.active = None;
            return root;
        }
        let v = &self.photos[i];
        let size = w.viewport_size();
        let start = self.targets[i].1.get();
        let usable = size.height - px(108.);
        let ww = (size.width - px(20.)).min(usable * v.ratio).max(px(1.)) * self.zoom;
        let hh = ww / v.ratio;
        let end = point((size.width - ww) / 2., (size.height - hh) / 2.);
        let travel = self.slide.get(w, cx).clamp(0., 1.);
        if travel >= 0.999 {
            self.previous = None;
        }
        let direction = self.previous.map_or(0., |(_, d)| d);
        let origin = start.origin
            + (end - start.origin) * a
            + self.offset
            + point(
                if still {
                    px(0.)
                } else {
                    size.width * direction * (1. - travel)
                },
                px(0.),
            );
        let photo = div()
            .absolute()
            .left(origin.x)
            .top(origin.y)
            .w(start.size.width + (ww - start.size.width) * a)
            .h(start.size.height + (hh - start.size.height) * a)
            .overflow_hidden()
            .rounded(radius)
            .opacity(if still { fade } else { 1. })
            .child(
                img(v.image.clone())
                    .size_full()
                    .rounded(radius)
                    .object_fit(ObjectFit::Contain),
            );
        let mut bar = row()
            .absolute()
            .top_2()
            .right_2()
            .rounded(radius)
            .bg(p.panel)
            .opacity(a);
        for (j, (en, zh, g)) in [
            ("Previous", "上一张", Glyph::Undo),
            ("Next", "下一张", Glyph::Arrow),
            ("Zoom", "缩放", Glyph::ZoomIn),
            ("Close", "关闭", Glyph::Close),
        ]
        .into_iter()
        .enumerate()
        {
            bar = bar.child(
                button(("action", j), tr(cx, en, zh), cx)
                    .size(px(32.))
                    .p_0()
                    .child(icon(g, 16., p.ink))
                    .on_click(cx.listener(move |s, _, w, cx| {
                        match j {
                            0 => s.step(-1, w, cx),
                            1 => s.step(1, w, cx),
                            2 => {
                                s.zoom = if s.zoom > 1. { 1. } else { 2. };
                                s.offset = Default::default();
                                cx.notify()
                            }
                            _ => s.close(w, cx),
                        }
                        cx.stop_propagation()
                    })),
            )
        }
        let layer = div()
            .id("viewer")
            .role(Role::Dialog)
            .aria_label(v.title.clone())
            .track_focus(&self.dialog)
            .relative()
            .w(size.width)
            .h(size.height)
            .overflow_hidden()
            .occlude()
            .bg(black().opacity(0.93 * if still { fade } else { a }))
            .when_some(self.previous, |d, (from, direction)| {
                let old = &self.photos[from];
                let ww = (size.width - px(20.)).min(usable * old.ratio).max(px(1.));
                let hh = ww / old.ratio;
                d.child(
                    img(old.image.clone())
                        .absolute()
                        .left((size.width - ww) / 2. - size.width * travel * direction)
                        .top((size.height - hh) / 2.)
                        .w(ww)
                        .h(hh)
                        .rounded(radius)
                        .object_fit(ObjectFit::Contain),
                )
            })
            .child(photo)
            .child(
                column()
                    .absolute()
                    .left_3()
                    .bottom_2()
                    .gap_1()
                    .opacity(a)
                    .child(label(v.title.clone(), 15., white()))
                    .child(label(v.caption.clone(), 12., white().opacity(0.75))),
            )
            .child(bar)
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                match e.keystroke.key.as_str() {
                    "left" => s.step(-1, w, cx),
                    "right" => s.step(1, w, cx),
                    "escape" => s.close(w, cx),
                    "tab" => {
                        if e.keystroke.modifiers.shift {
                            w.focus_prev(cx)
                        } else {
                            w.focus_next(cx)
                        };
                        if !s.dialog.contains_focused(w, cx) {
                            s.dialog.focus(w, cx)
                        }
                    }
                    _ => return,
                }
                cx.stop_propagation()
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|s, e: &MouseDownEvent, _, _| {
                    s.press = Some((e.position, s.offset));
                }),
            )
            .on_mouse_move(cx.listener(|s, e: &MouseMoveEvent, _, cx| {
                if e.pressed_button == Some(MouseButton::Left)
                    && let Some((start, base)) = s.press
                {
                    s.offset = base + e.position - start;
                    cx.notify()
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|s, e: &MouseUpEvent, w, cx| {
                    if let Some((start, _)) = s.press.take() {
                        let d = e.position - start;
                        if s.zoom == 1. {
                            if d.y.abs() > px(70.) && d.y.abs() > d.x.abs() {
                                s.close(w, cx)
                            } else if d.x.abs() > px(45.) {
                                s.step(if d.x > px(0.) { -1 } else { 1 }, w, cx)
                            }
                            s.offset = Default::default();
                            cx.notify()
                        }
                    }
                }),
            )
            .on_pinch(cx.listener(|s, e: &PinchEvent, _, cx| {
                s.zoom = (s.zoom * (1. + e.delta)).clamp(1., 4.);
                if s.zoom == 1. {
                    s.offset = Default::default()
                }
                cx.stop_propagation();
                cx.notify()
            }));
        root = root.child(
            deferred(anchored().position(point(px(0.), px(0.))).child(layer)).with_priority(5),
        );
        root
    }
}
