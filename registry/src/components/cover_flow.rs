//! Browse a short image collection on a perspective rail.
use crate::{
    foundation::surface_radius,
    primitives::{
        direct_touch::Contact,
        family::{Paint, Slide, disabled_motion},
        nav::{HorizontalScrollRegion, tr},
    },
};
use gpui::{prelude::*, *};
use image::{Frame, RgbaImage, imageops};
use std::{cell::Cell, rc::Rc, sync::Arc, time::Duration};
use web_time::Instant;

const ASPECT: f32 = 4. / 5.;
const SNAP_SECONDS: f32 = 0.42;
const WHEEL_IDLE: Duration = Duration::from_millis(110);

/// A photo and the host's optional caption data.
#[derive(Clone)]
pub struct CoverFlowItem<T = ()> {
    pub id: SharedString,
    pub title: SharedString,
    pub subtitle: SharedString,
    pub meta: SharedString,
    pub image: ImageSource,
    pub alt: SharedString,
    /// Normalized horizontal and vertical crop alignment, from 0 to 1.
    pub image_position: Point<f32>,
    pub data: T,
}

impl CoverFlowItem {
    pub fn new(
        id: impl Into<SharedString>,
        title: impl Into<SharedString>,
        image: impl Into<ImageSource>,
    ) -> Self {
        let title = title.into();
        Self {
            id: id.into(),
            alt: title.clone(),
            title,
            image: image.into(),
            subtitle: "".into(),
            meta: "".into(),
            image_position: point(0.5, 0.5),
            data: (),
        }
    }
}

impl<T> CoverFlowItem<T> {
    pub fn with_subtitle(mut self, subtitle: impl Into<SharedString>) -> Self {
        self.subtitle = subtitle.into();
        self
    }

    pub fn with_meta(mut self, meta: impl Into<SharedString>) -> Self {
        self.meta = meta.into();
        self
    }

    pub fn with_alt(mut self, alt: impl Into<SharedString>) -> Self {
        self.alt = alt.into();
        self
    }

    pub fn with_image_position(mut self, x: f32, y: f32) -> Self {
        let normalized = |v: f32| if v.is_finite() { v.clamp(0., 1.) } else { 0.5 };
        self.image_position = point(normalized(x), normalized(y));
        self
    }

    pub fn with_data<U>(self, data: U) -> CoverFlowItem<U> {
        CoverFlowItem {
            id: self.id,
            title: self.title,
            subtitle: self.subtitle,
            meta: self.meta,
            image: self.image,
            alt: self.alt,
            image_position: self.image_position,
            data,
        }
    }
}

/// The requested position, including card crossings during direct manipulation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CoverFlowIndexChanged(pub usize);

type Caption<T> = dyn Fn(&CoverFlowItem<T>, usize, &mut Window, &mut App) -> AnyElement;

struct Velocity {
    at: Instant,
    speed: f32,
}

impl Velocity {
    fn update(&mut self, delta: f32, now: Instant) {
        let seconds = now.saturating_duration_since(self.at).as_secs_f32();
        if seconds > 0.004 {
            self.speed = (delta / seconds).clamp(-18., 18.);
            self.at = now;
        }
    }

    fn recent(&self, now: Instant) -> f32 {
        if now.saturating_duration_since(self.at).as_secs_f32() < 0.105 {
            self.speed
        } else {
            0.
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Axis {
    Pending,
    Horizontal,
    Vertical,
}

struct Drag {
    start: Point<Pixels>,
    prior: Point<Pixels>,
    start_index: usize,
    start_position: f32,
    distance: f32,
    axis: Axis,
    velocity: Velocity,
    touch: bool,
}

struct WheelRun {
    start_index: usize,
    raw: f32,
    velocity: Velocity,
}

/// One state model and GPU-rendered rail for native windows and embedded previews.
pub struct CoverFlow<T: 'static = ()> {
    label: SharedString,
    items: Vec<CoverFlowItem<T>>,
    textures: Vec<Option<Textures>>,
    selected: usize,
    reported: usize,
    controlled: bool,
    rail: Slide,
    drag: Option<Drag>,
    wheel: Option<WheelRun>,
    wheel_timer: Option<Task<()>>,
    focus: FocusHandle,
    blur: Option<Subscription>,
    stage: Rc<Cell<Bounds<Pixels>>>,
    touch: Contact,
    caption: Option<Box<Caption<T>>>,
    caption_from: usize,
    caption_motion: Slide,
    caption_direction: f32,
    card_width: fn(f32) -> f32,
}

impl<T: 'static> EventEmitter<CoverFlowIndexChanged> for CoverFlow<T> {}

impl<T: 'static> CoverFlow<T> {
    pub fn new(label: impl Into<SharedString>, items: Vec<CoverFlowItem<T>>, cx: &App) -> Self {
        let textures = (0..items.len()).map(|_| None).collect();
        Self {
            label: label.into(),
            items,
            textures,
            selected: 0,
            reported: 0,
            controlled: false,
            rail: Slide::new(0.),
            drag: None,
            wheel: None,
            wheel_timer: None,
            focus: cx.focus_handle().tab_stop(true),
            blur: None,
            stage: Default::default(),
            touch: Contact::default(),
            caption: None,
            caption_from: 0,
            caption_motion: Slide::new(1.),
            caption_direction: 0.,
            card_width: |width| (width * 0.38).clamp(168., 272.),
        }
    }

    pub fn with_default_index(mut self, index: usize) -> Self {
        self.selected = index.min(self.last());
        self.reported = self.selected;
        self.rail.reset(self.selected as f32);
        self
    }

    /// Start with a host-controlled index; accept changes with `set_index`.
    pub fn with_index(self, index: usize) -> Self {
        let mut control = self.with_default_index(index);
        control.controlled = true;
        control
    }

    pub fn with_caption<E: IntoElement>(
        mut self,
        caption: impl Fn(&CoverFlowItem<T>, usize, &mut Window, &mut App) -> E + 'static,
    ) -> Self {
        self.caption = Some(Box::new(move |item, index, w, cx| {
            caption(item, index, w, cx).into_any_element()
        }));
        self
    }

    /// Override the default responsive card width using the measured rail width.
    pub fn with_card_width(mut self, width: fn(f32) -> f32) -> Self {
        self.card_width = width;
        self
    }

    pub fn index(&self) -> usize {
        self.selected
    }

    /// Apply the host index without sending a second change event.
    pub fn set_index(&mut self, index: usize, cx: &mut Context<Self>) {
        let index = index.min(self.last());
        self.set_caption(index, cx);
        self.reported = index;
        if self.drag.is_none() && self.wheel.is_none() {
            self.rail.duration(index as f32, SNAP_SECONDS, cx);
        }
        cx.notify();
    }

    fn last(&self) -> usize {
        self.items.len().saturating_sub(1)
    }

    fn nearest(&self, position: f32) -> usize {
        (position.round().max(0.) as usize).min(self.last())
    }

    fn width(&self, available: f32) -> f32 {
        let width = (self.card_width)(available);
        if width.is_finite() {
            width.clamp(1., (available * 0.94).max(1.))
        } else {
            (available * 0.38).clamp(1., 272.)
        }
    }

    fn drag_step(&self) -> f32 {
        self.width(self.stage.get().size.width.into()) * 0.66
    }

    fn set_caption(&mut self, index: usize, cx: &App) {
        if index == self.selected {
            return;
        }
        self.caption_from = self.selected;
        self.caption_direction = if index > self.selected { 1. } else { -1. };
        self.selected = index;
        self.caption_motion.reset(0.);
        self.caption_motion.duration(1., 0.16, cx);
    }

    fn request(&mut self, index: usize, cx: &mut Context<Self>) {
        if !self.controlled {
            self.set_caption(index, cx);
        }
        if index != self.reported {
            self.reported = index;
            cx.emit(CoverFlowIndexChanged(index));
        }
    }

    fn settle(&mut self, target: usize, velocity: f32, cx: &mut Context<Self>) {
        self.request(target, cx);
        let end = self.selected as f32;
        let current = self.rail.value(cx);
        // At a boundary the spring always starts moving back toward the rail.
        let velocity = if (current <= 0. && velocity < 0.)
            || (current >= self.last() as f32 && velocity > 0.)
        {
            0.
        } else {
            velocity
        };
        self.rail.release(end, velocity, SNAP_SECONDS, cx);
        cx.notify();
    }

    fn select(&mut self, target: usize, cx: &mut Context<Self>) {
        self.drag = None;
        self.wheel = None;
        self.wheel_timer = None;
        self.reported = self.selected;
        self.request(target.min(self.last()), cx);
        self.rail.duration(self.selected as f32, SNAP_SECONDS, cx);
        cx.notify();
    }

    fn start_drag(
        &mut self,
        at: Point<Pixels>,
        touch: bool,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.items.is_empty() || self.drag.is_some() {
            return;
        }
        self.wheel = None;
        self.wheel_timer = None;
        self.reported = self.selected;
        let start_position = self.rail.value(cx);
        self.rail.reset(start_position);
        self.drag = Some(Drag {
            start: at,
            prior: at,
            start_index: self.nearest(start_position),
            start_position,
            distance: 0.,
            axis: Axis::Pending,
            velocity: Velocity {
                at: cx.background_executor().now(),
                speed: 0.,
            },
            touch,
        });
        self.focus.focus(w, cx);
        cx.notify();
    }

    fn move_drag(&mut self, at: Point<Pixels>, touch: bool, cx: &mut Context<Self>) {
        let step = self.drag_step();
        let last = self.last();
        let Some(drag) = self.drag.as_mut().filter(|drag| drag.touch == touch) else {
            return;
        };
        let dx = f32::from(drag.start.x - at.x);
        let dy = f32::from(drag.start.y - at.y);
        if drag.axis == Axis::Pending && dx.abs().max(dy.abs()) >= 5. {
            drag.axis = if dx.abs() > dy.abs() {
                Axis::Horizontal
            } else {
                Axis::Vertical
            };
        }
        if drag.axis != Axis::Horizontal {
            return;
        }
        drag.distance = dx;
        let delta = f32::from(drag.prior.x - at.x) / step;
        drag.prior = at;
        if delta != 0. {
            drag.velocity.update(delta, cx.background_executor().now());
        }
        if !disabled_motion(cx) {
            let raw = drag.start_position + dx / step;
            self.rail.reset(resisted(raw, last));
            self.request(self.nearest(raw), cx);
        }
        cx.notify();
    }

    fn end_drag(&mut self, at: Point<Pixels>, touch: bool, cancel: bool, cx: &mut Context<Self>) {
        if self.drag.as_ref().is_none_or(|drag| drag.touch != touch) {
            return;
        }
        if !cancel {
            self.move_drag(at, touch, cx);
        }
        let Some(drag) = self.drag.take() else { return };
        let now = cx.background_executor().now();
        let velocity = drag.velocity.recent(now);
        let reduce = disabled_motion(cx);
        let target = match drag.axis {
            Axis::Pending if !cancel => self.hit(at, reduce, cx).unwrap_or(drag.start_index),
            Axis::Horizontal if !cancel && reduce => {
                if drag.distance.abs() >= 40. {
                    offset(drag.start_index, drag.distance, self.last())
                } else {
                    drag.start_index
                }
            }
            Axis::Horizontal if !cancel => {
                landing(self.rail.value(cx), velocity, drag.start_index, self.last())
            }
            _ => drag.start_index,
        };
        let speed = if !cancel && drag.axis == Axis::Horizontal && !reduce {
            velocity
        } else {
            0.
        };
        self.settle(target, speed, cx);
    }

    fn hit(&self, position: Point<Pixels>, reduce: bool, cx: &App) -> Option<usize> {
        let stage = self.stage.get();
        let width = self.width(stage.size.width.into());
        let rail = self.rail.value(cx);
        visible_cards(self.items.len(), rail)
            .into_iter()
            .rev()
            .find(|&i| Projection::new(stage, width, i as f32 - rail, reduce).hit(position))
    }

    fn scroll(&mut self, event: &ScrollEvent, w: &mut Window, cx: &mut Context<Self>) {
        let delta = event.delta.pixel_delta(px(24.));
        if self.drag.is_some()
            || self.items.len() < 2
            || event.modifiers.control
            || event.modifiers.platform
            || delta.x.abs() <= delta.y.abs()
        {
            return;
        }
        let step = -f32::from(delta.x) / self.drag_step();
        let now = cx.background_executor().now();
        let current = self.rail.value(cx);
        let start_index = self.nearest(current);
        let last = self.last();
        let first = self.wheel.is_none();
        if first {
            self.reported = self.selected;
        }
        let wheel = self.wheel.get_or_insert(WheelRun {
            start_index,
            raw: current,
            velocity: Velocity { at: now, speed: 0. },
        });
        wheel.raw += step;
        wheel.velocity.update(step, now);
        if disabled_motion(cx) {
            if first {
                let target = offset(wheel.start_index, step, last);
                self.request(target, cx);
                self.rail.reset(self.selected as f32);
            }
        } else {
            let raw = wheel.raw;
            self.rail.reset(resisted(raw, last));
            self.request(self.nearest(raw), cx);
        }
        self.wheel_timer = Some(cx.spawn(async |entity, cx| {
            cx.background_executor().timer(WHEEL_IDLE).await;
            let _ = entity.update(cx, |s, cx| {
                if let Some(wheel) = s.wheel.take() {
                    if disabled_motion(cx) {
                        s.rail.reset(s.selected as f32);
                        cx.notify();
                    } else {
                        let velocity = wheel.velocity.speed;
                        let target =
                            landing(s.rail.value(cx), velocity, wheel.start_index, s.last());
                        s.settle(target, velocity, cx);
                    }
                }
            });
        }));
        w.prevent_default();
        cx.stop_propagation();
        cx.notify();
    }

    fn caption(&self, index: usize, w: &mut Window, cx: &mut App) -> AnyElement {
        let item = &self.items[index];
        if let Some(caption) = &self.caption {
            return caption(item, index, w, cx);
        }
        let p = Paint::get(cx);
        let secondary = [item.subtitle.as_ref(), item.meta.as_ref()]
            .into_iter()
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join(" · ");
        div()
            .flex()
            .flex_col()
            .items_center()
            .w_full()
            .min_w_0()
            .child(
                div()
                    .max_w_full()
                    .truncate()
                    .text_size(px(18.))
                    .line_height(px(24.))
                    .font_weight(FontWeight::MEDIUM)
                    .child(item.title.clone()),
            )
            .when(!secondary.is_empty(), |caption| {
                caption.child(
                    div()
                        .max_w_full()
                        .truncate()
                        .text_size(px(14.))
                        .line_height(px(20.))
                        .text_color(p.secondary)
                        .child(secondary),
                )
            })
            .into_any_element()
    }
}

fn offset(index: usize, direction: f32, last: usize) -> usize {
    if direction < 0. {
        index.saturating_sub(1)
    } else {
        (index + 1).min(last)
    }
}

fn resisted(position: f32, last: usize) -> f32 {
    let edge = position.clamp(0., last as f32);
    let beyond = position - edge;
    edge + beyond / (1. + 4. * beyond.abs())
}

fn landing(position: f32, velocity: f32, origin: usize, last: usize) -> usize {
    let predicted = (position + velocity * 0.24).round().clamp(0., last as f32) as usize;
    if predicted == origin && velocity.abs() >= 1.2 {
        offset(origin, velocity, last)
    } else {
        predicted
    }
}

const CARD_FADE_EDGE: f32 = 2.7;

fn visible_cards(count: usize, position: f32) -> Vec<usize> {
    let start = (position - CARD_FADE_EDGE).floor().max(0.) as usize;
    let end = ((position + CARD_FADE_EDGE).ceil().max(0.) as usize).min(count);
    let mut indices: Vec<_> = (start..end)
        .filter(|i| (*i as f32 - position).abs() < CARD_FADE_EDGE)
        .collect();
    indices.sort_by(|a, b| {
        (*b as f32 - position)
            .abs()
            .total_cmp(&(*a as f32 - position).abs())
    });
    indices
}

#[derive(Clone)]
struct Textures {
    face: Arc<RenderImage>,
    reflection: Arc<RenderImage>,
}

impl Textures {
    fn prepare(source: Arc<RenderImage>, alignment: Point<f32>) -> Option<Self> {
        let dims = source.size(0);
        if dims.width.0 <= 0 || dims.height.0 <= 0 {
            return None;
        }
        let (width, height) = (dims.width.0 as u32, dims.height.0 as u32);
        let rgba = RgbaImage::from_raw(width, height, source.as_bytes(0)?.to_vec())?;
        let crop_w = width.min((height as f32 * ASPECT).round().max(1.) as u32);
        let crop_h = height.min((width as f32 / ASPECT).round().max(1.) as u32);
        let x = ((width - crop_w) as f32 * alignment.x).round() as u32;
        let y = ((height - crop_h) as f32 * alignment.y).round() as u32;
        let cropped = imageops::crop_imm(&rgba, x, y, crop_w, crop_h);
        let face = if crop_w > 560 {
            imageops::resize(&*cropped, 560, 700, imageops::FilterType::Triangle)
        } else {
            cropped.to_image()
        };
        let reflection_height = (face.height() as f32 * 0.15).round().max(1.) as u32;
        let reflection = RgbaImage::from_fn(face.width(), reflection_height, |x, y| {
            let mut pixel = *face.get_pixel(x, face.height() - 1 - y);
            let t = (y as f32 / reflection_height as f32 / 0.88).min(1.);
            pixel.0[3] = (f32::from(pixel.0[3]) * (1. - t) * 0.18).round() as u8;
            pixel
        });
        Some(Self {
            face: Arc::new(RenderImage::new([Frame::new(face)])),
            reflection: Arc::new(RenderImage::new([Frame::new(reflection)])),
        })
    }
}

fn resolve(source: &ImageSource, w: &mut Window, cx: &mut App) -> Option<Arc<RenderImage>> {
    match source {
        ImageSource::Image(image) => image.clone().use_render_image(w, cx),
        ImageSource::Render(image) => Some(image.clone()),
        ImageSource::Resource(resource) => w.use_asset::<ImgResourceLoader>(resource, cx)?.ok(),
        ImageSource::Custom(loader) => loader(w, cx)?.ok(),
    }
}

/// A pinhole camera with its vertical vanishing origin at the common card baseline.
struct Projection {
    x: f32,
    bottom: f32,
    width: f32,
    height: f32,
    offset: f32,
    depth: f32,
    sine: f32,
    cosine: f32,
    drift: f32,
}

impl Projection {
    fn new(bounds: Bounds<Pixels>, width: f32, distance: f32, reduce: bool) -> Self {
        let d = distance.abs();
        let first = d.min(1.);
        let second = (d - 1.).clamp(0., 1.);
        let rest = (d - 2.).max(0.);
        let angle = if reduce {
            0.
        } else {
            distance.clamp(-1., 1.) * std::f32::consts::FRAC_PI_4
        };
        let (sine, cosine) = angle.sin_cos();
        let height = width / ASPECT;
        Self {
            x: bounds.center().x.into(),
            bottom: f32::from(bounds.top()) + 8. + height,
            width,
            height,
            offset: distance.signum() * width * (0.8015 * first + 0.3687 * second + 0.129 * rest),
            depth: width * (0.3807 * first + 0.1403 * second + 0.0675 * rest),
            sine,
            cosine,
            drift: if reduce {
                0.
            } else {
                distance.clamp(-1., 1.) * 0.5
            },
        }
    }

    fn column(&self, u: f32) -> (f32, f32) {
        let plane = (u - 0.5) * self.width;
        let camera = self.width * 3.4;
        let scale = camera / (camera + self.depth - plane * self.sine);
        (self.x + (self.offset + plane * self.cosine) * scale, scale)
    }

    fn hit(&self, at: Point<Pixels>) -> bool {
        let (left, near) = self.column(0.);
        let (right, far) = self.column(1.);
        let x = f32::from(at.x);
        let t = ((x - left) / (right - left)).clamp(0., 1.);
        let top = self.bottom - self.height * (near + (far - near) * t);
        x >= left && x <= right && at.y >= px(top) && at.y <= px(self.bottom)
    }

    fn draw(
        &self,
        w: &mut Window,
        texture: Option<&Textures>,
        radius: f32,
        paint: Paint,
        distance: f32,
    ) {
        let (left, left_scale) = self.column(0.);
        let (right, right_scale) = self.column(1.);
        let width = right - left;
        let shadow = rect(left + width * 0.05, self.bottom - 8., width * 0.90, 9.);
        w.paint_drop_shadows(
            shadow,
            px(7.).into(),
            &[BoxShadow::new(px(0.), px(4.), black().opacity(0.29)).blur_radius(px(10.))],
        );
        let strips = if self.sine.abs() < 0.003 {
            1
        } else {
            (width / 2.7).ceil().clamp(20., 90.) as usize
        };
        let veil = black().opacity((distance.abs() * 0.17).min(0.47));
        for index in 0..strips {
            let u0 = index as f32 / strips as f32;
            let u1 = (index + 1) as f32 / strips as f32;
            let (x0, _) = self.column(u0);
            let (x1, _) = self.column(u1);
            let (_, scale) = self.column((u0 + u1) * 0.5);
            let card_height = self.height
                * if strips == 1 {
                    (left_scale + right_scale) / 2.
                } else {
                    scale
                };
            let inset = if strips == 1 {
                0.
            } else {
                let edge = u0.midpoint(u1).min(1. - u0.midpoint(u1)) * self.width;
                let local = (radius - edge).max(0.);
                (radius - (radius * radius - local * local).max(0.).sqrt()) * scale
            };
            let top = self.bottom - card_height;
            let projected_width = (x1 - x0) * strips as f32;
            let origin = x0 - projected_width * u0;
            let face = rect(x0, top + inset, x1 - x0, card_height - inset * 2.);
            let corners = px(if strips == 1 { radius } else { 0. });
            w.paint_quad(fill(face, paint.tray).corner_radii(corners));
            if let Some(texture) = texture {
                let drift = projected_width * 0.055;
                let photo_left = origin - drift * (0.5 + self.drift);
                let photo_width = projected_width + drift;
                let image = rect(photo_left, top, photo_width, card_height);
                let _ = w.paint_image(face, image, corners.into(), texture.face.clone(), 0, false);
                let reflection_height = card_height * 0.15;
                let reflection_top = self.bottom + 2.;
                let reflected = rect(
                    x0,
                    reflection_top + inset,
                    x1 - x0,
                    (reflection_height - inset).max(0.),
                );
                let reflected_image =
                    rect(photo_left, reflection_top, photo_width, reflection_height);
                let _ = w.paint_image(
                    reflected,
                    reflected_image,
                    Corners::default(),
                    texture.reflection.clone(),
                    0,
                    false,
                );
            }
            if veil.a > 0. {
                w.paint_quad(fill(face, veil).corner_radii(corners));
            }
        }
    }
}

fn rect(x: f32, y: f32, w: f32, h: f32) -> Bounds<Pixels> {
    Bounds::new(point(px(x), px(y)), size(px(w.max(0.)), px(h.max(0.))))
}

impl<T: 'static> Render for CoverFlow<T> {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.blur.is_none() {
            self.blur = Some(cx.on_blur(&self.focus, w, |s, _, cx| {
                if let Some(drag) = &s.drag {
                    s.end_drag(drag.prior, drag.touch, true, cx);
                }
            }));
        }
        let p = Paint::get(cx);
        let reduce = disabled_motion(cx);
        let mut root = div()
            .id("cover-flow")
            .role(Role::Region)
            .aria_label(self.label.clone())
            .w_full()
            .min_w_0()
            .flex()
            .flex_col()
            .gap(px(16.))
            .text_color(p.ink);
        if self.items.is_empty() {
            return root.child(div().text_center().text_color(p.secondary).child(tr(
                cx,
                "No photos",
                "暂无照片",
            )));
        }
        let measured = f32::from(self.stage.get().size.width);
        let available = if measured > 0. {
            measured
        } else {
            f32::from(w.viewport_size().width)
        };
        let width = self.width(available);
        let open = ((available - 342.) / 179.61).clamp(0., 1.);
        let stage_height = width / ASPECT * (1. + 0.10 * open) + 30.1;
        let position = if w.is_visible() {
            self.rail.get(w, cx)
        } else {
            self.rail.value(cx)
        };
        let front = self.nearest(position);
        let stage_bounds = self.stage.clone();
        let radius = f32::from(surface_radius(cx));
        let multiple = self.items.len() > 1;
        let moved = cx.listener(|s, e: &MouseMoveEvent, _, cx| s.move_drag(e.position, false, cx));
        let up = cx.listener(|s, e: &MouseUpEvent, _, cx| s.end_drag(e.position, false, false, cx));
        let mut stage = div()
            .id("cover-flow-stage")
            .relative()
            .w_full()
            .h(px(stage_height))
            .overflow_hidden()
            .track_focus(&self.focus)
            .role(Role::Group)
            .aria_label(tr(cx, "Photo rail", "照片浏览带"))
            .aria_description(tr(
                cx,
                "Use left and right arrows, Home or End.",
                "使用左右方向键、Home 或 End 浏览照片。",
            ))
            .cursor(if self.drag.is_some() {
                CursorStyle::ClosedHand
            } else {
                CursorStyle::OpenHand
            })
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                let target = match e.keystroke.key.as_str() {
                    "left" => s.selected.saturating_sub(1),
                    "right" => s.selected.saturating_add(1),
                    "home" => 0,
                    "end" => s.last(),
                    _ => return,
                };
                s.select(target, cx);
                w.prevent_default();
                cx.stop_propagation();
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|s, e: &MouseDownEvent, w, cx| s.start_drag(e.position, false, w, cx)),
            )
            .on_scroll(cx.listener(Self::scroll))
            .child(
                canvas(
                    move |bounds, w, _| {
                        let prior = stage_bounds.replace(bounds);
                        if (prior.size.width - bounds.size.width).abs() > px(0.25) {
                            w.request_animation_frame();
                        }
                    },
                    move |bounds, _, w, cx| {
                        if multiple && let Some(host) = cx.try_global::<HorizontalScrollRegion>() {
                            (host.0)(bounds, true, true);
                        }
                        w.on_mouse_event(move |event: &MouseMoveEvent, phase, w, cx| {
                            if phase.capture() {
                                moved(event, w, cx);
                            }
                        });
                        w.on_mouse_event(move |event: &MouseUpEvent, phase, w, cx| {
                            if phase.capture() && event.button == MouseButton::Left {
                                up(event, w, cx);
                            }
                        });
                    },
                )
                .absolute()
                .inset_0(),
            );
        for index in visible_cards(self.items.len(), position) {
            let item = &self.items[index];
            if self.textures[index].is_none()
                && let Some(image) = resolve(&item.image, w, cx)
            {
                self.textures[index] = Textures::prepare(image, item.image_position);
            }
            let texture = self.textures[index].clone();
            let distance = index as f32 - position;
            let opacity = ((CARD_FADE_EDGE - distance.abs()) / (CARD_FADE_EDGE - 2.)).clamp(0., 1.);
            stage = stage.child(
                div()
                    .id(("cover-card", index))
                    .absolute()
                    .inset_0()
                    .when(index == front, |card| {
                        card.role(Role::Image)
                            .aria_label(format!(
                                "{} / {}: {}",
                                index + 1,
                                self.items.len(),
                                item.alt
                            ))
                            .aria_active_descendant()
                    })
                    .opacity(opacity)
                    .child(
                        canvas(
                            |_, _, _| (),
                            move |bounds, _, w, _| {
                                Projection::new(bounds, width, distance, reduce).draw(
                                    w,
                                    texture.as_ref(),
                                    radius,
                                    p,
                                    distance,
                                );
                            },
                        )
                        .size_full(),
                    ),
            );
        }
        stage = stage.child(self.touch.listen(cx, |s, event, w, cx| match event.phase {
            TouchPhase::Started => s.start_drag(event.start_position, true, w, cx),
            TouchPhase::Moved => s.move_drag(event.position, true, cx),
            TouchPhase::Ended => s.end_drag(event.position, true, false, cx),
            TouchPhase::Cancelled => s.end_drag(event.position, true, true, cx),
        }));
        if self.focus.is_focused(w) && w.last_input_was_keyboard() {
            stage = stage.child(
                canvas(
                    |_, _, _| (),
                    move |bounds, _, w, _| {
                        w.paint_quad(
                            outline(bounds, p.focus, BorderStyle::Solid).corner_radii(px(radius)),
                        );
                    },
                )
                .absolute()
                .inset_0(),
            );
        }
        let phase = if w.is_visible() {
            self.caption_motion.get(w, cx)
        } else {
            self.caption_motion.value(cx)
        };
        let direction = self.caption_direction;
        let caption = div()
            .relative()
            .w_full()
            .min_h(px(48.))
            .px_4()
            .overflow_hidden()
            .child(
                div()
                    .relative()
                    .w_full()
                    .top(px((1. - phase) * direction * 6.))
                    .opacity(phase)
                    .child(self.caption(self.selected, w, cx)),
            )
            .when(phase < 0.999, |caption| {
                caption.child(
                    div()
                        .absolute()
                        .left_4()
                        .right_4()
                        .top(px(-phase * direction * 6.))
                        .opacity(1. - phase)
                        .child(self.caption(self.caption_from, w, cx)),
                )
            });
        let ticks = div()
            .w_full()
            .flex()
            .items_center()
            .justify_center()
            .flex_wrap()
            .children(self.items.iter().enumerate().map(|(i, item)| {
                let selected = self.selected == i;
                let active = if selected {
                    phase
                } else if self.caption_from == i {
                    1. - phase
                } else {
                    0.
                };
                div()
                    .id(("cover-tick", i))
                    .role(Role::Button)
                    .tab_index(-1)
                    .aria_label(format!("{} {}", tr(cx, "Show", "查看"), item.title))
                    .aria_toggled(selected.into())
                    .w(px(16.))
                    .h(px(24.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .child(
                        div()
                            .w(px(5. + active * 7.))
                            .h(px(5.))
                            .rounded_full()
                            .bg(p.faint.opacity(0.48).blend(p.ink.opacity(active))),
                    )
                    .on_click(cx.listener(move |s, _, w, cx| {
                        s.select(i, cx);
                        s.focus.focus(w, cx);
                    }))
            }));
        root = root.child(stage).child(
            div()
                .w_full()
                .flex()
                .flex_col()
                .gap(px(12.))
                .child(caption)
                .child(ticks),
        );
        root
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CoverFlow, CoverFlowIndexChanged, CoverFlowItem, Projection, landing, rect, resisted,
    };
    use gpui::{RenderImage, TestAppContext, TouchEvent, TouchId, TouchPhase, point, px, size};
    use image::{Frame, Rgba, RgbaImage};
    use std::{cell::RefCell, rc::Rc, sync::Arc, time::Duration};

    #[test]
    fn edges_resist_and_a_quick_throw_projects_past_one_card() {
        assert!((-0.25..0.).contains(&resisted(-1.5, 6)));
        assert_eq!(landing(-0.2, -4., 0, 6), 0);
        assert_eq!(landing(2.1, 1.2, 2, 6), 3);
        assert_eq!(landing(3.9, 9., 2, 6), 6);
    }

    #[test]
    fn perspective_matches_the_measured_reference_at_rest() {
        let bounds = rect(0., 0., 521.61, 252.44);
        let width = 161.7;
        let center = Projection::new(bounds, width, 0., false);
        let right = Projection::new(bounds, width, 1., false);
        let (left, _) = right.column(0.);
        let (far, scale) = right.column(1.);
        assert!((left - 320.37).abs() < 0.15);
        assert!((far - left - 125.72).abs() < 0.15);
        assert!((right.bottom - right.height * scale - 9.60).abs() < 0.15);
        assert!((center.column(0.).0 - 179.95).abs() < 0.15);
        assert_eq!(right.bottom, center.bottom);
    }

    #[test]
    fn reduced_motion_touch_threshold_and_keyboard_use_one_index() {
        let mut app = TestAppContext::single();
        app.update(|cx| {
            crate::init(cx);
            crate::Theme::global_mut(cx).reduced_motion = true;
        });
        let (rail, window) = app.add_window_view(|_, cx| {
            let image = Arc::new(RenderImage::new([Frame::new(RgbaImage::from_pixel(
                4,
                5,
                Rgba([128, 128, 128, 255]),
            ))]));
            let items = ["One", "Two", "Three"]
                .map(|title| CoverFlowItem::new(title, title, image.clone()));
            CoverFlow::new("Test rail", items.to_vec(), cx)
        });
        window.simulate_resize(size(px(520.), px(430.)));
        window.run_until_parked();
        let events = Rc::new(RefCell::new(Vec::new()));
        let collected = events.clone();
        let _subscription = window.update(|w, cx| {
            rail.read(cx).focus.clone().focus(w, cx);
            cx.subscribe(&rail, move |_, e: &CoverFlowIndexChanged, _| {
                collected.borrow_mut().push(e.0);
            })
        });
        window.simulate_keystrokes("right home");
        window.run_until_parked();
        let start = window.update(|_, cx| rail.read(cx).stage.get().center());
        for distance in [39., 41.] {
            let end = start - point(px(distance), px(0.));
            for (phase, position) in [
                (TouchPhase::Started, start),
                (TouchPhase::Moved, end),
                (TouchPhase::Ended, end),
            ] {
                window.simulate_event(TouchEvent {
                    id: TouchId(1),
                    phase,
                    position,
                    ..Default::default()
                });
                window.executor().advance_clock(Duration::from_millis(16));
                window.run_until_parked();
            }
        }
        assert_eq!(window.update(|_, cx| rail.read(cx).index()), 1);
        assert_eq!(*events.borrow(), [1, 0, 1]);
    }
}
