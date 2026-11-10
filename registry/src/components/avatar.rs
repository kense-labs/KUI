//! Square portrait cropped to a circle with a readable initials fallback.
use crate::primitives::{
    family::{Paint, PhaseClock, Slide, ease},
    presentation::*,
};
use gpui::{prelude::*, *};
#[derive(Clone)]
pub struct Person {
    pub name: SharedString,
    pub image: Option<ImageSource>,
}
impl Person {
    pub fn new(name: impl Into<SharedString>) -> Self {
        Self {
            name: name.into(),
            image: None,
        }
    }
    pub fn with_image(mut self, src: impl Into<ImageSource>) -> Self {
        self.image = Some(src.into());
        self
    }
}
pub struct Avatar {
    person: Person,
    size: f32,
    pending: bool,
    arrival: PhaseClock,
    status: Option<bool>,
    status_motion: Slide,
    status_color: Slide,
}
impl Avatar {
    pub fn new(person: Person, size: f32) -> Self {
        Self {
            person,
            size: size.max(16.),
            pending: false,
            arrival: Default::default(),
            status: None,
            status_motion: Slide::new(0.),
            status_color: Slide::new(0.),
        }
    }
    /// Use Some(true) for online, Some(false) for offline, and None to hide presence.
    pub fn set_status(&mut self, online: Option<bool>, cx: &mut Context<Self>) {
        if self.status != online {
            self.status = online;
            if let Some(on) = online {
                self.status_color.duration(on as u8 as f32, 0.26, cx);
            }
            self.status_motion
                .duration(online.is_some() as u8 as f32, 0.26, cx);
            cx.notify();
        }
    }
    pub fn portrait(person: &Person, size: f32, p: Paint) -> Stateful<Div> {
        let initials = person
            .name
            .split_whitespace()
            .filter_map(|w| w.chars().next())
            .take(2)
            .collect::<String>();
        row()
            .relative()
            .size(px(size))
            .flex_none()
            .justify_center()
            .rounded_full()
            .overflow_hidden()
            .bg(p.hover)
            .text_size(px(size * 0.35))
            .text_color(p.ink)
            .id(person.name.clone())
            .role(Role::Image)
            .aria_label(person.name.clone())
            .child(initials)
            .when_some(person.image.clone(), |el, src| {
                el.child(
                    div().absolute().inset_0().child(
                        img(src)
                            .size_full()
                            .rounded_full()
                            .object_fit(ObjectFit::Cover),
                    ),
                )
            })
    }
}
// The renderer keeps this decode in the window asset cache, including between portraits.
#[derive(Clone)]
enum PortraitDecoder {}
impl Asset for PortraitDecoder {
    type Source = std::sync::Arc<Image>;
    type Output = Result<std::sync::Arc<RenderImage>, ImageCacheError>;
    fn load(
        src: Self::Source,
        cx: &mut App,
    ) -> impl Future<Output = Self::Output> + Send + 'static {
        let svg = cx.svg_renderer();
        async move { src.to_image_data(svg).map_err(Into::into) }
    }
}
impl Render for Avatar {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let mut fallback = self.person.clone();
        fallback.image = None;
        let mut face = Self::portrait(&fallback, self.size, p);
        if let Some(source) = &self.person.image {
            let decoded = match source {
                ImageSource::Resource(resource) => w.use_asset::<ImgResourceLoader>(resource, cx),
                ImageSource::Image(data) => w.use_asset::<PortraitDecoder>(data, cx),
                ImageSource::Render(image) => Some(Ok(image.clone())),
                ImageSource::Custom(load) => load(w, cx),
            };
            match decoded {
                None => self.pending = true,
                Some(Ok(image)) => {
                    if self.pending {
                        self.pending = false;
                        self.arrival.restart(cx);
                    }
                    let shown = ease(self.arrival.time(0.24, w, cx) / 0.24);
                    face = face.child(
                        div().absolute().inset_0().opacity(shown).child(
                            img(image)
                                .size_full()
                                .rounded_full()
                                .object_fit(ObjectFit::Cover),
                        ),
                    );
                }
                Some(Err(_)) => self.pending = false,
            }
        }
        let status = self.status_motion.get(w, cx).clamp(0., 1.);
        let color = self.status_color.get(w, cx).clamp(0., 1.);
        let dot_size = self.size * 0.24 * (0.6 + status * 0.4);
        div()
            .relative()
            .size(px(self.size))
            .child(face)
            .when(status > 0.001, |el| {
                el.child(
                    div()
                        .absolute()
                        .right_0()
                        .bottom_0()
                        .size(px(dot_size))
                        .rounded_full()
                        .border_1()
                        .border_color(p.stage)
                        .opacity(status)
                        .bg(p.secondary)
                        .child(
                            div()
                                .size_full()
                                .rounded_full()
                                .bg(p.success.opacity(color)),
                        ),
                )
            })
    }
}
