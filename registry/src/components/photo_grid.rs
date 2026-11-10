//! Browse a photo library with density controls and a shared-surface viewer.
use crate::{
    foundation::surface_radius,
    primitives::{
        family::{Paint, Slide, disabled_motion},
        flyout::TriggerBounds,
        glyph::{Glyph, icon},
        nav::{Fade, column, label, row, tr},
    },
};
use gpui::{prelude::*, *};
use std::collections::BTreeSet;
use web_time::Instant;

const GAP: Pixels = px(3.);
const HEADER: Pixels = px(60.);
const MIN_HEIGHT: f32 = 320.;
const COMPACT_WIDTH: Pixels = px(480.);

/// Stable photo identity, accessible name, and known intrinsic size.
#[derive(Clone)]
pub struct GridPhoto {
    pub id: SharedString,
    pub source: ImageSource,
    pub thumbnail: Option<ImageSource>,
    pub alt: SharedString,
    pub title: SharedString,
    pub detail: Option<SharedString>,
    pub size: Size<Pixels>,
}

impl GridPhoto {
    pub fn new(
        id: impl Into<SharedString>,
        source: impl Into<ImageSource>,
        alt: impl Into<SharedString>,
        title: impl Into<SharedString>,
        size: Size<Pixels>,
    ) -> Self {
        Self {
            id: id.into(),
            source: source.into(),
            thumbnail: None,
            alt: alt.into(),
            title: title.into(),
            detail: None,
            size,
        }
    }

    pub fn with_thumbnail(mut self, source: impl Into<ImageSource>) -> Self {
        self.thumbnail = Some(source.into());
        self
    }

    pub fn with_detail(mut self, detail: impl Into<SharedString>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    fn thumbnail(&self) -> ImageSource {
        self.thumbnail.as_ref().unwrap_or(&self.source).clone()
    }

    fn ratio(&self) -> f32 {
        let ratio = self.size.width / self.size.height;
        if ratio.is_finite() && ratio > 0. {
            ratio.clamp(0.1, 10.)
        } else {
            1.
        }
    }
}

/// Changes committed by the grid or viewer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PhotoGridEvent {
    ColumnsChanged(usize),
    FavoriteChanged { id: SharedString, favorite: bool },
    ViewerChanged(Option<SharedString>),
}

/// Registers the inner viewport with an embedding page's gesture router.
pub struct PhotoGridScrollRegion(pub fn(Bounds<Pixels>, bool, bool, bool));
impl Global for PhotoGridScrollRegion {}

#[derive(Clone, Copy)]
struct Tiles {
    columns: usize,
    side: Pixels,
}

impl Tiles {
    fn new(width: Pixels, columns: usize) -> Self {
        let side = ((width - GAP * (columns + 1) as f32) / columns as f32).max(px(1.));
        Self { columns, side }
    }

    fn bounds(self, i: usize) -> Bounds<Pixels> {
        let pitch = self.side + GAP;
        Bounds::new(
            point(
                GAP + pitch * (i % self.columns) as f32,
                pitch * (i / self.columns) as f32,
            ),
            size(self.side, self.side),
        )
    }

    fn height(self, count: usize) -> Pixels {
        (self.side + GAP) * count.div_ceil(self.columns) as f32
    }

    fn hit(self, at: Point<Pixels>, count: usize) -> usize {
        let pitch = self.side + GAP;
        let row = (at.y / pitch).max(0.) as usize;
        let col = ((at.x - GAP) / pitch).max(0.) as usize;
        (row * self.columns + col.min(self.columns - 1)).min(count.saturating_sub(1))
    }
}

fn mix(a: Bounds<Pixels>, b: Bounds<Pixels>, t: f32) -> Bounds<Pixels> {
    Bounds::new(
        a.origin + (b.origin - a.origin) * t,
        size(
            a.size.width + (b.size.width - a.size.width) * t,
            a.size.height + (b.size.height - a.size.height) * t,
        ),
    )
}

fn translated(mut bounds: Bounds<Pixels>, offset: Point<Pixels>) -> Bounds<Pixels> {
    bounds.origin += offset;
    bounds
}

fn fit_image(room: Size<Pixels>, ratio: f32) -> Bounds<Pixels> {
    let width = (room.width - px(24.)).min(room.height * ratio).max(px(1.));
    let height = width / ratio;
    Bounds::new(
        point((room.width - width) / 2., (room.height - height) / 2.),
        size(width, height),
    )
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DragTarget {
    Viewer,
    Strip,
}

struct Drag {
    origin: Point<Pixels>,
    from: usize,
    target: DragTarget,
}

/// Scrollable, square-cell album with an in-place photo viewer.
pub struct PhotoGrid {
    title: SharedString,
    photos: Vec<GridPhoto>,
    levels: Vec<usize>,
    columns: usize,
    height: Pixels,
    favorites: BTreeSet<SharedString>,
    panel: TriggerBounds,
    scroll: ScrollHandle,
    focus: Vec<FocusHandle>,
    focused: usize,
    reflow: Vec<Bounds<Pixels>>,
    density: Slide,
    zoom_amount: f32,
    zoom_anchor: Option<(usize, Point<Pixels>)>,
    wheel_at: Option<Instant>,
    wheel_delta: f32,
    active: Option<usize>,
    closing: bool,
    viewer: Slide,
    fade: Fade,
    page: Slide,
    page_fade: Fade,
    previous: Option<(usize, f32)>,
    dialog: FocusHandle,
    back: FocusHandle,
    strip: FocusHandle,
    drag: Option<Drag>,
    touch: Point<Pixels>,
    strip_touch: Option<(usize, Pixels)>,
    pull: [Slide; 2],
    heart: Slide,
}

impl EventEmitter<PhotoGridEvent> for PhotoGrid {}

impl PhotoGrid {
    pub fn new(title: impl Into<SharedString>, photos: Vec<GridPhoto>, cx: &App) -> Self {
        let focus = photos
            .iter()
            .map(|_| cx.focus_handle().tab_stop(true))
            .collect();
        Self {
            title: title.into(),
            photos,
            levels: vec![5, 3, 2],
            columns: 3,
            height: px(380.),
            favorites: BTreeSet::new(),
            panel: TriggerBounds::default(),
            scroll: ScrollHandle::new(),
            focus,
            focused: 0,
            reflow: Vec::new(),
            density: Slide::new(1.),
            zoom_amount: 0.,
            zoom_anchor: None,
            wheel_at: None,
            wheel_delta: 0.,
            active: None,
            closing: false,
            viewer: Slide::new(0.),
            fade: Fade::default(),
            page: Slide::new(1.),
            page_fade: Fade::visible(),
            previous: None,
            dialog: cx.focus_handle(),
            back: cx.focus_handle().tab_stop(true),
            strip: cx.focus_handle().tab_stop(true),
            drag: None,
            touch: Point::default(),
            strip_touch: None,
            pull: [Slide::new(0.), Slide::new(0.)],
            heart: Slide::new(1.),
        }
    }

    /// Descending list of distinct column counts. Empty input keeps the defaults.
    pub fn with_zoom_levels(mut self, levels: impl IntoIterator<Item = usize>) -> Self {
        let mut levels: Vec<_> = levels
            .into_iter()
            .filter(|n| (1..=12).contains(n))
            .collect();
        levels.sort_unstable_by(|a, b| b.cmp(a));
        levels.dedup();
        if !levels.is_empty() {
            self.levels = levels;
        }
        self
    }

    pub fn with_default_columns(mut self, columns: usize) -> Self {
        if (1..=12).contains(&columns) {
            self.columns = columns;
        }
        self
    }

    pub fn with_default_favorites(mut self, ids: impl IntoIterator<Item = SharedString>) -> Self {
        self.favorites = ids
            .into_iter()
            .filter(|id| self.photos.iter().any(|p| p.id == *id))
            .collect();
        self
    }

    pub fn with_height(mut self, height: Pixels) -> Self {
        self.height = height.max(px(MIN_HEIGHT));
        self
    }

    pub fn columns(&self) -> usize {
        self.columns
    }

    pub fn is_favorite(&self, id: &str) -> bool {
        self.favorites.iter().any(|fav| fav.as_ref() == id)
    }

    pub fn viewed_photo(&self) -> Option<&str> {
        self.active
            .filter(|_| !self.closing)
            .map(|i| self.photos[i].id.as_ref())
    }

    fn dimensions(&self) -> Size<Pixels> {
        self.panel.get().size
    }

    fn limit(&self, tiles: Tiles) -> Pixels {
        (tiles.height(self.photos.len()) - (self.height - HEADER)).max(px(0.))
    }

    fn target_cell(&self, tiles: Tiles, i: usize) -> Bounds<Pixels> {
        translated(tiles.bounds(i), point(px(0.), self.scroll.offset().y))
    }

    fn cell(&self, tiles: Tiles, i: usize, t: f32) -> Bounds<Pixels> {
        let dest = self.target_cell(tiles, i);
        self.reflow.get(i).map_or(dest, |from| mix(*from, dest, t))
    }

    fn pinched_cell(&self, tiles: Tiles, i: usize, t: f32) -> Bounds<Pixels> {
        let mut bounds = self.cell(tiles, i, t);
        if let Some((_, anchor)) = self.zoom_anchor {
            let scale = 1. + self.zoom_amount * 0.8;
            bounds.origin = anchor + (bounds.origin - anchor) * scale;
            bounds.size.width *= scale;
            bounds.size.height *= scale;
        }
        bounds
    }

    fn visible_photo(&self, tiles: Tiles, at: Point<Pixels>) -> usize {
        tiles.hit(
            at - self.panel.get().origin - point(px(0.), HEADER + self.scroll.offset().y),
            self.photos.len(),
        )
    }

    fn change_columns(&mut self, columns: usize, anchor: usize, cx: &mut Context<Self>) {
        if columns == self.columns || !self.levels.contains(&columns) {
            return;
        }
        let width = self.dimensions().width;
        let old = Tiles::new(width, self.columns);
        let target = Tiles::new(width, columns);
        let t = self.density.value(cx).clamp(0., 1.);
        let screen_y = self.pinched_cell(old, anchor, t).center().y;
        self.reflow = (0..self.photos.len())
            .map(|i| self.pinched_cell(old, i, t))
            .collect();
        self.columns = columns;
        self.scroll.set_offset(point(
            px(0.),
            (screen_y - target.bounds(anchor).center().y).clamp(-self.limit(target), px(0.)),
        ));
        self.density.reset(0.);
        self.density.duration(1., 0.33, cx);
        cx.emit(PhotoGridEvent::ColumnsChanged(columns));
        cx.notify();
    }

    fn adjacent_level(&self, inwards: bool) -> usize {
        let next = if inwards {
            self.levels.iter().find(|&&count| count < self.columns)
        } else {
            self.levels
                .iter()
                .rev()
                .find(|&&count| count > self.columns)
        };
        next.copied().unwrap_or(self.columns)
    }

    fn focus_cell(&mut self, i: usize, w: &mut Window, cx: &mut Context<Self>) {
        let Some(focus) = self.focus.get(i) else {
            return;
        };
        self.focused = i;
        let tiles = Tiles::new(self.dimensions().width, self.columns);
        let b = self.target_cell(tiles, i);
        let offset = self.scroll.offset().y;
        let next = if b.top() < px(0.) {
            offset - b.top()
        } else if b.bottom() > self.height - HEADER {
            offset + self.height - HEADER - b.bottom()
        } else {
            offset
        };
        self.scroll
            .set_offset(point(px(0.), next.clamp(-self.limit(tiles), px(0.))));
        focus.focus(w, cx);
        cx.notify();
    }

    fn grid_key(&mut self, i: usize, e: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        let delta = self.columns as isize;
        let next = match e.keystroke.key.as_str() {
            "left" => i.saturating_sub(1),
            "right" => i + 1,
            "up" => i.saturating_add_signed(-delta),
            "down" => i + self.columns,
            "pageup" => i.saturating_add_signed(-3 * delta),
            "pagedown" => i + 3 * self.columns,
            "home" => 0,
            "end" => self.photos.len() - 1,
            "enter" | "space" if !e.is_held => {
                self.open(i, w, cx);
                i
            }
            "+" | "=" | "-" => {
                self.change_columns(self.adjacent_level(e.keystroke.key != "-"), i, cx);
                i
            }
            _ => return,
        };
        if self.active.is_none() && next != i {
            self.focus_cell(next.min(self.photos.len() - 1), w, cx);
        }
        w.prevent_default();
        cx.stop_propagation();
    }

    fn open(&mut self, i: usize, w: &mut Window, cx: &mut Context<Self>) {
        if i >= self.photos.len() || self.active.is_some() {
            return;
        }
        self.active = Some(i);
        self.closing = false;
        self.focused = i;
        self.viewer.duration(1., 0.36, cx);
        self.fade.set(true, 0.15, cx);
        self.back.focus(w, cx);
        cx.emit(PhotoGridEvent::ViewerChanged(Some(
            self.photos[i].id.clone(),
        )));
        cx.notify();
    }

    fn close(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if let Some(i) = self.active.filter(|_| !self.closing) {
            self.closing = true;
            self.drag = None;
            self.previous = None;
            self.focus_cell(i, w, cx);
            self.viewer.duration(0., 0.30, cx);
            self.fade.set(false, 0.10, cx);
            self.return_pull(cx);
            cx.emit(PhotoGridEvent::ViewerChanged(None));
            cx.notify();
        }
    }

    fn show(&mut self, i: usize, cx: &mut Context<Self>) {
        let Some(old) = self.active.filter(|old| !self.closing && *old != i) else {
            return;
        };
        if i >= self.photos.len() {
            return;
        }
        let from = if self.previous.is_some_and(|(prev, _)| prev == i) {
            1. - self.page.value(cx)
        } else {
            0.
        };
        self.previous = Some((old, if i > old { 1. } else { -1. }));
        self.active = Some(i);
        self.page.reset(from);
        self.page.duration(1., 0.30, cx);
        self.page_fade = Fade::default();
        self.page_fade.set(true, 0.15, cx);
        cx.emit(PhotoGridEvent::ViewerChanged(Some(
            self.photos[i].id.clone(),
        )));
        cx.notify();
    }

    fn step(&mut self, delta: isize, cx: &mut Context<Self>) {
        if let Some(i) = self.active {
            self.show(i.saturating_add_signed(delta), cx);
        }
    }

    fn favorite(&mut self, cx: &mut Context<Self>) {
        if let Some(i) = self.active.filter(|_| !self.closing) {
            let id = self.photos[i].id.clone();
            let favorite = if self.favorites.remove(&id) {
                false
            } else {
                self.favorites.insert(id.clone())
            };
            self.heart.reset(0.);
            self.heart.duration(1., 0.28, cx);
            cx.emit(PhotoGridEvent::FavoriteChanged { id, favorite });
            cx.notify();
        }
    }

    fn viewer_key(&mut self, e: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        match e.keystroke.key.as_str() {
            "left" => self.step(-1, cx),
            "right" => self.step(1, cx),
            "home" => self.show(0, cx),
            "end" => self.show(self.photos.len() - 1, cx),
            "escape" => self.close(w, cx),
            "tab" => {
                if e.keystroke.modifiers.shift {
                    w.focus_prev(cx)
                } else {
                    w.focus_next(cx)
                }
                if !self.dialog.contains_focused(w, cx) {
                    if e.keystroke.modifiers.shift {
                        self.strip.focus(w, cx)
                    } else {
                        self.back.focus(w, cx)
                    }
                }
            }
            _ => return,
        }
        w.prevent_default();
        cx.stop_propagation();
    }

    fn zoom(&mut self, delta: f32, at: Point<Pixels>, phase: TouchPhase, cx: &mut Context<Self>) {
        if self.active.is_some() {
            return;
        }
        if phase == TouchPhase::Cancelled {
            self.zoom_anchor = None;
            self.zoom_amount = 0.;
            cx.notify();
            return;
        }
        let width = self.dimensions().width;
        let anchor = self.visible_photo(Tiles::new(width, self.columns), at);
        self.zoom_anchor
            .get_or_insert((anchor, at - self.panel.get().origin - point(px(0.), HEADER)));
        self.zoom_amount = (self.zoom_amount + delta).clamp(-0.40, 0.40);
        if phase == TouchPhase::Ended {
            if self.zoom_amount.abs() >= 0.12
                && self.adjacent_level(self.zoom_amount > 0.) != self.columns
            {
                let next = self.adjacent_level(self.zoom_amount > 0.);
                self.change_columns(next, self.zoom_anchor.map_or(anchor, |(i, _)| i), cx);
            } else {
                let tiles = Tiles::new(width, self.columns);
                let t = self.density.value(cx).clamp(0., 1.);
                self.reflow = (0..self.photos.len())
                    .map(|i| self.pinched_cell(tiles, i, t))
                    .collect();
                self.density.reset(0.);
                self.density.duration(1., 0.26, cx);
            }
            self.zoom_amount = 0.;
            self.zoom_anchor = None;
        }
        cx.notify();
    }

    fn grid_wheel(&mut self, e: &ScrollEvent, w: &mut Window, cx: &mut Context<Self>) {
        if !e.modifiers.control {
            cx.notify();
            return;
        }
        let now = cx.background_executor().now();
        let elapsed = self
            .wheel_at
            .replace(now)
            .map(|last| now.saturating_duration_since(last));
        if elapsed.is_none_or(|dt| dt.as_millis() > 180) {
            self.wheel_delta = 0.;
        }
        self.wheel_delta += f32::from(e.delta.pixel_delta(px(16.)).y);
        if self.wheel_delta.abs() >= 34. {
            let anchor = self.visible_photo(
                Tiles::new(self.dimensions().width, self.columns),
                e.position,
            );
            self.change_columns(self.adjacent_level(self.wheel_delta > 0.), anchor, cx);
            self.wheel_delta = 0.;
        }
        w.prevent_default();
        cx.stop_propagation();
    }

    fn return_pull(&mut self, cx: &App) {
        self.pull[0].duration(0., 0.28, cx);
        self.pull[1].duration(0., 0.28, cx);
        self.touch = Point::default();
    }

    fn track_pull(&mut self, delta: Point<Pixels>, cx: &mut Context<Self>) {
        let horizontal = delta.x.abs() > delta.y.abs();
        self.pull[0].reset(if horizontal { delta.x.into() } else { 0. });
        self.pull[1].reset(if horizontal {
            0.
        } else {
            delta.y.max(px(0.)).into()
        });
        cx.notify();
    }

    fn finish_pull(&mut self, delta: Point<Pixels>, w: &mut Window, cx: &mut Context<Self>) {
        if delta.y > px(85.) && delta.y > delta.x.abs() {
            self.close(w, cx);
        } else if delta.x.abs() > px(44.) && delta.x.abs() > delta.y.abs() {
            let from = self.active;
            self.step(if delta.x > px(0.) { -1 } else { 1 }, cx);
            if self.active != from {
                self.pull[0].reset(0.);
                self.page
                    .reset((delta.x.abs() / self.dimensions().width.max(px(1.))).clamp(0., 1.));
                self.page.duration(1., 0.30, cx);
            }
        }
        self.return_pull(cx);
        cx.notify();
    }

    fn start_drag(&mut self, origin: Point<Pixels>, target: DragTarget) {
        if let Some(from) = self.active.filter(|_| !self.closing) {
            self.drag = Some(Drag {
                origin,
                from,
                target,
            });
        }
    }

    fn strip_pitch(&self) -> Pixels {
        px(if self.dimensions().width < COMPACT_WIDTH {
            20.
        } else {
            25.
        })
    }

    fn scrub(&mut self, from: usize, delta: Pixels, cx: &mut Context<Self>) {
        let step = (-delta / self.strip_pitch()).round() as isize;
        self.show(
            from.saturating_add_signed(step).min(self.photos.len() - 1),
            cx,
        );
    }

    fn strip_scroll(&mut self, e: &ScrollEvent, w: &mut Window, cx: &mut Context<Self>) {
        if e.source != ScrollSource::Touch {
            return;
        }
        match e.touch_phase {
            TouchPhase::Started => self.strip_touch = self.active.map(|i| (i, px(0.))),
            TouchPhase::Moved => {
                if let Some((from, mut dx)) = self.strip_touch {
                    dx += e.delta.pixel_delta(px(16.)).x;
                    self.strip_touch = Some((from, dx));
                    self.scrub(from, dx, cx);
                }
            }
            TouchPhase::Ended | TouchPhase::Cancelled => self.strip_touch = None,
        }
        w.prevent_default();
        cx.stop_propagation();
    }

    fn drag_capture(&self, cx: &Context<Self>) -> impl IntoElement {
        let moved = cx.listener(|s, e: &MouseMoveEvent, _, cx| {
            if let Some(drag) = &s.drag
                && e.pressed_button == Some(MouseButton::Left)
            {
                let delta = e.position - drag.origin;
                match drag.target {
                    DragTarget::Viewer => s.track_pull(delta, cx),
                    DragTarget::Strip => s.scrub(drag.from, delta.x, cx),
                }
            }
        });
        let released = cx.listener(|s, e: &MouseUpEvent, w, cx| {
            if e.button == MouseButton::Left
                && let Some(drag) = s.drag.take()
            {
                let delta = e.position - drag.origin;
                if drag.target == DragTarget::Viewer {
                    s.finish_pull(delta, w, cx);
                } else if delta.x.abs() > px(6.) {
                    s.scrub(drag.from, delta.x, cx);
                }
                if delta.x.abs() > px(6.) || delta.y.abs() > px(6.) {
                    cx.stop_propagation();
                }
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
}

fn action(
    id: &'static str,
    caption: impl Into<SharedString>,
    side: Pixels,
    p: Paint,
) -> Stateful<Div> {
    div()
        .id(id)
        .role(Role::Button)
        .aria_label(caption)
        .size(side)
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .border_1()
        .border_color(p.line)
        .bg(p.panel)
        .cursor_pointer()
        .hover(move |d| d.bg(p.hover))
        .focus_visible(move |d| d.border_color(p.bright))
}

fn mosaic(cols: usize, ink: Hsla) -> Div {
    let n = cols.clamp(2, 5);
    let side = (16. - (n - 1) as f32 * 1.5) / n as f32;
    column().gap(px(1.5)).children((0..n).map(|_| {
        row()
            .gap(px(1.5))
            .children((0..n).map(|_| div().size(px(side)).rounded(px(0.5)).bg(ink)))
    }))
}

fn heart(ink: Hsla, fill: bool) -> AnyElement {
    if !fill {
        return icon(Glyph::Heart, 18., ink).into_any_element();
    }
    canvas(
        |_, _, _| (),
        move |b, _, w, _| {
            let pt = |x: f32, y: f32| b.origin + point(b.size.width * x, b.size.height * y);
            let mut shape = PathBuilder::fill();
            shape.move_to(pt(0.50, 0.88));
            shape.cubic_bezier_to(pt(0.06, 0.38), pt(0.29, 0.71), pt(0.06, 0.56));
            shape.cubic_bezier_to(pt(0.50, 0.25), pt(0.06, 0.13), pt(0.36, 0.08));
            shape.cubic_bezier_to(pt(0.94, 0.38), pt(0.64, 0.08), pt(0.94, 0.13));
            shape.cubic_bezier_to(pt(0.50, 0.88), pt(0.94, 0.56), pt(0.71, 0.71));
            shape.close();
            if let Ok(path) = shape.build() {
                w.paint_path(path, ink);
            }
        },
    )
    .size(px(16.))
    .into_any_element()
}

fn direction(back: bool, arrow: bool, ink: Hsla) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |b, _, w, _| {
            let pt = |x: f32, y: f32| {
                let x = if back { 16. - x } else { x };
                b.origin + point(px(x), px(y))
            };
            let mut line = PathBuilder::stroke(px(1.5));
            line.move_to(pt(6., 3.));
            line.line_to(pt(11., 8.));
            line.line_to(pt(6., 13.));
            if arrow {
                line.move_to(pt(2., 8.));
                line.line_to(pt(11., 8.));
            }
            if let Ok(path) = line.build() {
                w.paint_path(path, ink);
            }
        },
    )
    .size(px(16.))
}

impl PhotoGrid {
    fn header(&self, width: Pixels, p: Paint, cx: &mut Context<Self>) -> Div {
        let compact = width < COMPACT_WIDTH;
        let count = tr(cx, "photos", "张照片");
        row()
            .h(HEADER)
            .px(px(if compact { 10. } else { 12. }))
            .justify_between()
            .gap_2()
            .child(
                column()
                    .flex_1()
                    .min_w_0()
                    .child(
                        div()
                            .text_size(px(16.))
                            .line_height(px(21.))
                            .text_color(p.ink)
                            .truncate()
                            .child(self.title.clone()),
                    )
                    .child(
                        row()
                            .gap_1()
                            .child(label(
                                format!("{} {count}", self.photos.len()),
                                12.,
                                p.secondary,
                            ))
                            .when(!self.favorites.is_empty(), |d| {
                                d.child(row().gap_1().pl_1().child(heart(p.secondary, true)).child(
                                    label(self.favorites.len().to_string(), 12., p.secondary),
                                ))
                            }),
                    ),
            )
            .child(
                row()
                    .flex_none()
                    .p(px(3.))
                    .gap(px(2.))
                    .rounded_full()
                    .bg(p.hover)
                    .children(self.levels.iter().enumerate().map(|(i, &n)| {
                        let active = self.columns == n;
                        div()
                            .id(("photo-density", i))
                            .role(Role::Button)
                            .aria_toggled(if active {
                                Toggled::True
                            } else {
                                Toggled::False
                            })
                            .tab_index(0)
                            .aria_label(if tr(cx, "en", "zh") == "zh" {
                                format!("每行 {n} 张")
                            } else {
                                format!("{n} photos per row")
                            })
                            .w(px(if compact { 30. } else { 38. }))
                            .h(px(30.))
                            .rounded_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .when(active, |d| d.bg(p.panel))
                            .hover(move |d| d.bg(p.panel))
                            .child(mosaic(n, if active { p.ink } else { p.secondary }))
                            .on_click(cx.listener(move |s, _, _, cx| {
                                let top = s.panel.get().origin + point(GAP, HEADER + GAP);
                                let anchor = s.visible_photo(
                                    Tiles::new(s.dimensions().width, s.columns),
                                    top,
                                );
                                s.change_columns(n, anchor, cx);
                            }))
                            .on_key_down(cx.listener(move |s, e: &KeyDownEvent, w, cx| {
                                if !e.is_held
                                    && matches!(e.keystroke.key.as_str(), "enter" | "space")
                                {
                                    s.change_columns(n, s.focused, cx);
                                    w.prevent_default();
                                    cx.stop_propagation();
                                }
                            }))
                    })),
            )
    }

    fn photo_tiles(
        &self,
        layout: Tiles,
        progress: f32,
        radius: Pixels,
        p: Paint,
        cx: &mut Context<Self>,
    ) -> Div {
        let scroll_y = self.scroll.offset().y;
        let still = disabled_motion(cx);
        div()
            .relative()
            .w_full()
            .h(layout.height(self.photos.len()))
            .children(self.photos.iter().enumerate().map(|(i, photo)| {
                let b = if still {
                    self.cell(layout, i, progress)
                } else {
                    self.pinched_cell(layout, i, progress)
                };
                let favorite = self.is_favorite(&photo.id);
                let name = if favorite {
                    format!("{}, {}", photo.alt, tr(cx, "favorite", "已收藏")).into()
                } else {
                    photo.alt.clone()
                };
                div()
                    .id(("photo-cell", i))
                    .debug_selector(move || format!("photo-cell-{i}"))
                    .role(Role::Button)
                    .aria_label(name)
                    .track_focus(&self.focus[i])
                    .tab_index(
                        if i == self.focused && (self.active.is_none() || self.closing) {
                            0
                        } else {
                            -1
                        },
                    )
                    .absolute()
                    .left(b.left())
                    .top(b.top() - scroll_y)
                    .w(b.size.width)
                    .h(b.size.height)
                    .rounded(radius)
                    .overflow_hidden()
                    .bg(p.hover)
                    .cursor_pointer()
                    .child(
                        img(photo.thumbnail())
                            .size_full()
                            .rounded(radius)
                            .object_fit(ObjectFit::Cover),
                    )
                    .when(favorite, |d| {
                        d.child(
                            div()
                                .absolute()
                                .left(px(6.))
                                .bottom(px(5.))
                                .child(heart(white(), true)),
                        )
                    })
                    .focus_visible(move |d| {
                        d.shadow(vec![
                            BoxShadow::new(px(0.), px(0.), p.bright).spread_radius(px(1.)),
                        ])
                    })
                    .on_key_down(
                        cx.listener(move |s, e: &KeyDownEvent, w, cx| s.grid_key(i, e, w, cx)),
                    )
                    .on_click(cx.listener(move |s, _, w, cx| s.open(i, w, cx)))
            }))
    }

    fn thumbnail_width(&self, i: usize, height: Pixels) -> Pixels {
        (height * self.photos[i].ratio()).clamp(px(22.), px(60.))
    }

    fn filmstrip(
        &self,
        i: usize,
        width: Pixels,
        compact: bool,
        page: f32,
        p: Paint,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let thumb_h = px(if compact { 36. } else { 40. });
        let first = i.saturating_sub(12);
        let last = (i + 13).min(self.photos.len());
        let pitch = self.strip_pitch();
        let collapsed = pitch - GAP;
        let old = self.previous.map(|(old, _)| old);
        let thumb_width = |j| {
            let emphasis = if j == i {
                page
            } else if Some(j) == old {
                1. - page
            } else {
                0.
            };
            collapsed + (self.thumbnail_width(j, thumb_h) - collapsed) * emphasis
        };
        let from_center: Pixels = (first..i).map(|j| thumb_width(j) + GAP).sum();
        let travel = old.map_or(px(0.), |old| {
            (pitch * (i as f32 - old as f32)).clamp(-width, width)
        }) * (1. - page);
        let value = if tr(cx, "en", "zh") == "zh" {
            format!(
                "{}，第 {} 张，共 {} 张",
                self.photos[i].title,
                i + 1,
                self.photos.len()
            )
        } else {
            format!(
                "{}, {} of {}",
                self.photos[i].title,
                i + 1,
                self.photos.len()
            )
        };
        div()
            .id("photo-strip")
            .debug_selector(|| "photo-strip".into())
            .role(Role::Slider)
            .aria_label(tr(cx, "Photos", "照片胶片条"))
            .aria_value(value)
            .aria_numeric_value((i + 1) as f64)
            .aria_min_numeric_value(1.)
            .aria_max_numeric_value(self.photos.len() as f64)
            .aria_numeric_value_step(1.)
            .track_focus(&self.strip)
            .tab_index(0)
            .block_mouse_except_scroll()
            .absolute()
            .bottom_0()
            .left_0()
            .w_full()
            .h(px(if compact { 60. } else { 64. }))
            .overflow_hidden()
            .on_scroll(cx.listener(Self::strip_scroll))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|s, e: &MouseDownEvent, _, _| {
                    s.start_drag(e.position, DragTarget::Strip)
                }),
            )
            .child(
                row()
                    .absolute()
                    .top(px(8.))
                    .left(width / 2. - from_center - thumb_width(i) / 2. + travel)
                    .gap(GAP)
                    .children((first..last).map(|j| {
                        div()
                            .id(("photo-strip-thumb", j))
                            .w(thumb_width(j))
                            .h(thumb_h)
                            .flex_none()
                            .relative()
                            .rounded(px(4.))
                            .overflow_hidden()
                            .cursor_pointer()
                            .opacity(if i == j { 1. } else { 0.53 })
                            .hover(|d| d.opacity(1.))
                            .child(
                                img(self.photos[j].thumbnail())
                                    .size_full()
                                    .rounded(px(4.))
                                    .object_fit(ObjectFit::Cover),
                            )
                            .when(i == j, |d| {
                                d.child(
                                    div()
                                        .absolute()
                                        .inset_0()
                                        .rounded(px(4.))
                                        .border_1()
                                        .border_color(p.bright),
                                )
                            })
                            .on_click(cx.listener(move |s, _, _, cx| s.show(j, cx)))
                    })),
            )
    }

    fn viewer_layer(
        &mut self,
        layout: Tiles,
        density: f32,
        i: usize,
        w: &mut Window,
        p: Paint,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let width = self.dimensions().width;
        let compact = width < COMPACT_WIDTH;
        let radius = surface_radius(cx);
        let photo = &self.photos[i];
        let still = disabled_motion(cx);
        let t = if still {
            self.fade.get(w, cx)
        } else {
            self.viewer.get(w, cx).clamp(0., 1.)
        };
        let page = if still {
            self.page_fade.get(w, cx)
        } else {
            self.page.get(w, cx).clamp(0., 1.)
        };
        if page >= 0.999 {
            self.previous = None;
        }
        let top = px(if compact { 64. } else { 72. });
        let room = size(width, (self.height - top - px(76.)).max(px(1.)));
        let mut target = translated(fit_image(room, photo.ratio()), point(px(0.), top));
        let drag_x = if still { 0. } else { self.pull[0].get(w, cx) };
        let drag_y = if still {
            0.
        } else {
            self.pull[1].get(w, cx).max(0.)
        };
        let scale = 1. - (drag_y / 420.).min(0.24);
        target.origin += point(
            target.size.width * (1. - scale) / 2.,
            target.size.height * (1. - scale) / 2.,
        );
        target.size.width *= scale;
        target.size.height *= scale;
        target.origin += point(px(drag_x), px(drag_y * 0.75));
        let travel_direction = self.previous.map_or(0., |(_, dir)| dir);
        if !still {
            target.origin.x += width * travel_direction * (1. - page);
        }
        let origin = translated(self.cell(layout, i, density), point(px(0.), HEADER));
        let b = if still {
            target
        } else {
            mix(origin, target, t)
        };
        let side = px(if compact { 36. } else { 40. });
        let favorite = self.is_favorite(&photo.id);
        let pop = self.heart.get(w, cx).clamp(0., 1.);
        let pos = if let Some(detail) = &photo.detail {
            format!("{detail} · {} / {}", i + 1, self.photos.len())
        } else {
            format!("{} / {}", i + 1, self.photos.len())
        };
        let dim = if still {
            t
        } else {
            t * (1. - (drag_y / 300.).min(0.65))
        };
        let place = |image: ImageSource, b: Bounds<Pixels>, opacity: f32, fit| {
            img(image)
                .absolute()
                .left(b.left())
                .top(b.top())
                .w(b.size.width)
                .h(b.size.height)
                .rounded(radius)
                .opacity(opacity)
                .object_fit(fit)
        };
        div()
            .id("photo-viewer")
            .debug_selector(|| "photo-viewer".into())
            .role(Role::Dialog)
            .aria_label(tr(cx, "Photo viewer", "照片查看器"))
            .track_focus(&self.dialog)
            .absolute()
            .inset_0()
            .rounded(radius)
            .occlude()
            .bg(p.panel.opacity(dim))
            .on_key_down(cx.listener(Self::viewer_key))
            .on_scroll(cx.listener(|s, e: &ScrollEvent, w, cx| {
                if e.source != ScrollSource::Touch {
                    return;
                }
                match e.touch_phase {
                    TouchPhase::Started => s.touch = Point::default(),
                    TouchPhase::Moved => {
                        s.touch += e.delta.pixel_delta(px(16.));
                        s.track_pull(s.touch, cx);
                    }
                    TouchPhase::Ended => s.finish_pull(s.touch, w, cx),
                    TouchPhase::Cancelled => s.return_pull(cx),
                }
                w.prevent_default();
                cx.stop_propagation();
            }))
            .child(self.drag_capture(cx))
            .when_some(self.previous, |d, (old, dir)| {
                let mut last = translated(
                    fit_image(room, self.photos[old].ratio()),
                    point(px(0.), top),
                );
                if !still {
                    last.origin.x -= width * dir * page;
                }
                d.child(place(
                    self.photos[old].source.clone(),
                    last,
                    if still { 1. - page } else { 1. },
                    ObjectFit::Contain,
                ))
            })
            .child(place(
                photo.source.clone(),
                b,
                if still { t * page } else { 1. },
                if t > 0.998 {
                    ObjectFit::Contain
                } else {
                    ObjectFit::Cover
                },
            ))
            .child(
                div()
                    .id("photo-swipe-zone")
                    .absolute()
                    .left_0()
                    .right_0()
                    .top(top)
                    .h(room.height)
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|s, e: &MouseDownEvent, _, _| {
                            s.start_drag(e.position, DragTarget::Viewer)
                        }),
                    ),
            )
            .child(
                row()
                    .absolute()
                    .top(px(10.))
                    .left(px(10.))
                    .right(px(10.))
                    .gap_2()
                    .justify_between()
                    .opacity(t)
                    .child(
                        action(
                            "photo-back",
                            tr(cx, "Back to all photos", "返回所有照片"),
                            side,
                            p,
                        )
                        .debug_selector(|| "photo-back".into())
                        .track_focus(&self.back)
                        .tab_index(0)
                        .block_mouse_except_scroll()
                        .child(direction(true, true, p.ink))
                        .on_click(cx.listener(|s, _, w, cx| s.close(w, cx)))
                        .on_key_down(cx.listener(
                            |s, e: &KeyDownEvent, w, cx| {
                                if !e.is_held
                                    && matches!(e.keystroke.key.as_str(), "enter" | "space")
                                {
                                    s.close(w, cx);
                                    cx.stop_propagation();
                                }
                            },
                        )),
                    )
                    .child(
                        column()
                            .flex_1()
                            .min_w_0()
                            .items_center()
                            .child(
                                div()
                                    .max_w_full()
                                    .truncate()
                                    .text_size(px(14.))
                                    .text_color(p.ink)
                                    .child(photo.title.clone()),
                            )
                            .child(
                                div()
                                    .id("photo-position")
                                    .role(Role::Status)
                                    .max_w_full()
                                    .truncate()
                                    .text_size(px(12.))
                                    .text_color(p.secondary)
                                    .child(pos),
                            ),
                    )
                    .child(
                        action("photo-favorite", tr(cx, "Favorite", "收藏"), side, p)
                            .debug_selector(|| "photo-favorite".into())
                            .aria_toggled(if favorite {
                                Toggled::True
                            } else {
                                Toggled::False
                            })
                            .tab_index(0)
                            .block_mouse_except_scroll()
                            .child(
                                div().opacity(0.7 + 0.3 * pop).child(heart(
                                    if favorite { p.danger } else { p.ink },
                                    favorite,
                                )),
                            )
                            .on_click(cx.listener(|s, _, _, cx| s.favorite(cx)))
                            .on_key_down(cx.listener(|s, e: &KeyDownEvent, _, cx| {
                                if !e.is_held
                                    && matches!(e.keystroke.key.as_str(), "enter" | "space")
                                {
                                    s.favorite(cx);
                                    cx.stop_propagation();
                                }
                            })),
                    ),
            )
            .when(!compact, |d| {
                d.children([true, false].map(|back| {
                    let enabled = if back {
                        i > 0
                    } else {
                        i + 1 < self.photos.len()
                    };
                    action(
                        if back { "photo-prev" } else { "photo-next" },
                        if back {
                            tr(cx, "Previous photo", "上一张照片")
                        } else {
                            tr(cx, "Next photo", "下一张照片")
                        },
                        side,
                        p,
                    )
                    .absolute()
                    .top(top + room.height / 2. - side / 2.)
                    .when(back, |b| b.left(px(12.)))
                    .when(!back, |b| b.right(px(12.)))
                    .opacity(if enabled { t } else { t * 0.28 })
                    .tab_index(if enabled { 0 } else { -1 })
                    .block_mouse_except_scroll()
                    .child(direction(back, false, p.ink))
                    .on_click(cx.listener(move |s, _, _, cx| s.step(if back { -1 } else { 1 }, cx)))
                    .on_key_down(cx.listener(
                        move |s, e: &KeyDownEvent, _, cx| {
                            if !e.is_held && matches!(e.keystroke.key.as_str(), "enter" | "space") {
                                s.step(if back { -1 } else { 1 }, cx);
                                cx.stop_propagation();
                            }
                        },
                    ))
                }))
            })
            .child(
                self.filmstrip(i, width, compact, if still { 1. } else { page }, p, cx)
                    .opacity(t),
            )
    }
}

impl Render for PhotoGrid {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let radius = surface_radius(cx);
        let measured = self.dimensions().width;
        let width = if measured > px(0.) {
            measured
        } else {
            w.viewport_size().width
        };
        let tiles = Tiles::new(width, self.columns);
        let t = self.density.get(w, cx).clamp(0., 1.);
        if t >= 0.999 {
            self.reflow.clear();
        }
        let open_progress = if disabled_motion(cx) {
            self.fade.get(w, cx)
        } else {
            self.viewer.get(w, cx)
        };
        if self.closing && open_progress <= 0.001 {
            self.active = None;
            self.closing = false;
        }
        let scroll = self.scroll.clone();
        let viewer = self.active.is_some();
        let panel = self.panel.clone();
        let header = self.header(width, p, cx);
        let cells = self.photo_tiles(tiles, t, radius.min(px(6.)), p, cx);
        let grid = div()
            .id("photo-scroll")
            .debug_selector(|| "photo-scroll".into())
            .role(Role::List)
            .aria_label(format!(
                "{}, {} {}",
                self.title,
                self.photos.len(),
                tr(cx, "photos", "张照片")
            ))
            .absolute()
            .left_0()
            .right_0()
            .top(HEADER)
            .bottom_0()
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .on_scroll(cx.listener(Self::grid_wheel))
            .on_pinch(cx.listener(|s, e: &PinchEvent, w, cx| {
                s.zoom(e.delta, e.position, e.phase, cx);
                w.prevent_default();
                cx.stop_propagation();
            }))
            .child(cells);
        let viewer_layer = self
            .active
            .map(|i| self.viewer_layer(tiles, t, i, w, p, cx));
        div()
            .id("photo-grid")
            .relative()
            .w_full()
            .h(self.height)
            .rounded(radius)
            .bg(p.panel)
            .overflow_hidden()
            .child(panel.measure())
            .child(
                canvas(
                    |_, _, _| (),
                    move |b, _, w, cx| {
                        if b.size.width != width {
                            w.request_animation_frame();
                        }
                        if let Some(host) = cx.try_global::<PhotoGridScrollRegion>() {
                            let region = if viewer {
                                b
                            } else {
                                Bounds::new(
                                    b.origin + point(px(0.), HEADER),
                                    size(b.size.width, (b.size.height - HEADER).max(px(0.))),
                                )
                            };
                            (host.0)(
                                region,
                                scroll.offset().y < px(-0.5),
                                scroll.offset().y > -scroll.max_offset().y + px(0.5),
                                viewer,
                            );
                        }
                    },
                )
                .absolute()
                .inset_0(),
            )
            .child(grid)
            .child(header)
            .children(viewer_layer)
    }
}

#[cfg(test)]
mod tests {
    use super::{GridPhoto, PhotoGrid, PhotoGridEvent};
    use gpui::{
        App, Modifiers, MouseButton, PinchEvent, TestAppContext, TouchPhase, VisualTestContext,
        point, px, size,
    };
    use std::time::Duration;

    fn album(cx: &App) -> PhotoGrid {
        let photos = (0..16)
            .map(|i| {
                GridPhoto::new(
                    i.to_string(),
                    "photo.jpg".to_owned(),
                    format!("Photo {i}"),
                    format!("Title {i}"),
                    size(px(400.), px(300.)),
                )
            })
            .collect();
        PhotoGrid::new("Album", photos, cx).with_default_favorites(["2".into(), "14".into()])
    }

    fn tick(window: &mut VisualTestContext, ms: u64) {
        window.executor().advance_clock(Duration::from_millis(ms));
        window.update(|w, cx| {
            w.simulate_next_frame(cx);
            w.refresh();
        });
        window.run_until_parked();
    }

    fn click(window: &mut VisualTestContext, name: &'static str) {
        let p = window.debug_bounds(name).unwrap().center();
        window.simulate_mouse_move(p, None, Modifiers::default());
        window.simulate_mouse_down(p, MouseButton::Left, Modifiers::default());
        window.simulate_mouse_up(p, MouseButton::Left, Modifiers::default());
    }

    #[test]
    fn gallery_keyboard_and_favorite_preserve_identity_on_dismiss() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (grid, window) = app.add_window_view(|_, cx| album(cx));
        window.simulate_resize(size(px(520.), px(400.)));
        tick(window, 1);
        let events = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let events_copy = events.clone();
        let _sub = window.update(|w, cx| {
            grid.read(cx).focus[0].clone().focus(w, cx);
            cx.subscribe(&grid, move |_, e: &PhotoGridEvent, _| {
                events_copy.borrow_mut().push(e.clone())
            })
        });
        window.simulate_keystrokes("right enter");
        tick(window, 700);
        assert!(window.update(|w, cx| grid.read(cx).back.is_focused(w)));
        click(window, "photo-favorite");
        window.simulate_keystrokes("right tab");
        assert!(window.update(|w, cx| grid.read(cx).dialog.contains_focused(w, cx)));
        window.simulate_keystrokes("escape");
        tick(window, 700);
        assert!(window.update(|w, cx| {
            let grid = grid.read(cx);
            grid.is_favorite("1") && grid.focus[2].is_focused(w) && grid.viewed_photo().is_none()
        }));
        assert_eq!(
            *events.borrow(),
            vec![
                PhotoGridEvent::ViewerChanged(Some("1".into())),
                PhotoGridEvent::FavoriteChanged {
                    id: "1".into(),
                    favorite: true
                },
                PhotoGridEvent::ViewerChanged(Some("2".into())),
                PhotoGridEvent::ViewerChanged(None),
            ]
        );
    }

    #[test]
    fn reflow_starts_at_the_current_cells_and_keeps_the_viewed_anchor() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (grid, window) = app.add_window_view(|_, cx| album(cx));
        window.simulate_resize(size(px(522.), px(400.)));
        tick(window, 1);
        window.update(|w, cx| grid.update(cx, |s, cx| s.focus_cell(7, w, cx)));
        tick(window, 1);
        let before = window.debug_bounds("photo-cell-7").unwrap();
        window.update(|_, cx| grid.update(cx, |s, cx| s.change_columns(2, 7, cx)));
        tick(window, 0);
        let initial = window.debug_bounds("photo-cell-7").unwrap();
        assert!((initial.top() - before.top()).abs() < px(1.));
        assert_eq!(initial.size, before.size);
        tick(window, 700);
        let after = window.debug_bounds("photo-cell-7").unwrap();
        assert!(after.size.width > before.size.width + px(70.));
        assert!((after.center().y - before.center().y).abs() < px(1.));
    }

    #[test]
    fn pinch_scales_live_and_commits_from_an_independent_initial_count() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (grid, window) = app.add_window_view(|_, cx| album(cx).with_default_columns(4));
        window.simulate_resize(size(px(522.), px(400.)));
        tick(window, 1);
        let before = window.debug_bounds("photo-cell-0").unwrap();
        let position = before.center();
        let event = |phase, delta| PinchEvent {
            position,
            delta,
            phase,
            modifiers: Default::default(),
        };
        window.simulate_event(event(TouchPhase::Started, 0.));
        window.simulate_event(event(TouchPhase::Moved, 0.20));
        tick(window, 16);
        assert_eq!(window.update(|_, cx| grid.read(cx).columns()), 4);
        let live = window.debug_bounds("photo-cell-0").unwrap();
        assert!(live.size.width > before.size.width + px(10.));
        window.simulate_event(event(TouchPhase::Ended, 0.));
        tick(window, 0);
        assert_eq!(window.update(|_, cx| grid.read(cx).columns()), 3);
        assert!(
            (window.debug_bounds("photo-cell-0").unwrap().size.width - live.size.width).abs()
                < px(1.)
        );
    }

    #[test]
    fn compact_viewer_swipe_and_downward_release() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (grid, window) = app.add_window_view(|_, cx| album(cx));
        window.simulate_resize(size(px(320.), px(400.)));
        tick(window, 1);
        click(window, "photo-cell-0");
        tick(window, 700);
        let start = point(px(160.), px(180.));
        for delta in [point(px(-90.), px(0.)), point(px(0.), px(115.))] {
            let end = start + delta;
            window.simulate_mouse_move(start, None, Modifiers::default());
            window.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
            window.simulate_mouse_move(end, MouseButton::Left, Modifiers::default());
            tick(window, 16);
            window.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
            tick(window, 700);
            let actual = window.update(|_, cx| grid.read(cx).viewed_photo().map(str::to_owned));
            assert_eq!(actual, (delta.x < px(0.)).then_some("1".into()));
        }
    }
}
