//! A photo with a pointer-lit glass panel and a measured detail disclosure.
use crate::{
    foundation::{Theme, ThemeMode, surface_radius},
    primitives::{
        family::{Paint, Slide, disabled_motion},
        nav::{Content, Fade},
    },
};
use gpui::{prelude::*, *};
use image::{Frame, RgbaImage, imageops};
use std::{cell::Cell, hash::Hash, rc::Rc, sync::Arc};

const INSET: f32 = 12.;
const EXPAND_SECONDS: f32 = 0.42;

/// An expansion request. In controlled mode the host must call `set_open`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GlassCardOpenChanged(pub bool);

/// A reusable 4:5 photo card. The host supplies the header slots and interactive details.
pub struct GlassCard {
    id: ElementId,
    image: ImageSource,
    image_alt: SharedString,
    title: SharedString,
    subtitle: Option<Content>,
    aside: Option<Content>,
    body: Option<Content>,
    style: Option<Box<dyn Fn(Div) -> Div>>,
    width: f32,
    tilt: f32,
    solid: bool,
    open: bool,
    controlled: bool,
    focus: FocusHandle,
    group_focus: FocusHandle,
    progress: Slide,
    fade: Fade,
    pointer: [Slide; 2],
    highlight: Slide,
    frame_bounds: Rc<Cell<Bounds<Pixels>>>,
    body_height: Rc<Cell<Pixels>>,
    panel_bounds: Rc<Cell<Bounds<Pixels>>>,
}

impl EventEmitter<GlassCardOpenChanged> for GlassCard {}

impl GlassCard {
    pub fn new(
        image: impl Into<ImageSource>,
        image_alt: impl Into<SharedString>,
        title: impl Into<SharedString>,
        cx: &App,
    ) -> Self {
        let title = title.into();
        Self {
            id: ElementId::Name(title.clone()),
            image: image.into(),
            image_alt: image_alt.into(),
            title,
            subtitle: None,
            aside: None,
            body: None,
            style: None,
            width: 360.,
            tilt: 6.,
            solid: false,
            open: false,
            controlled: false,
            focus: cx.focus_handle().tab_stop(true),
            group_focus: cx.focus_handle(),
            progress: Slide::new(0.),
            fade: Fade::default(),
            pointer: [Slide::snappy(0.); 2],
            highlight: Slide::snappy(0.),
            frame_bounds: Rc::default(),
            body_height: Rc::default(),
            panel_bounds: Rc::default(),
        }
    }

    pub fn with_id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = id.into();
        self
    }

    pub fn with_subtitle(mut self, content: Content) -> Self {
        self.subtitle = Some(content);
        self
    }

    pub fn with_aside(mut self, content: Content) -> Self {
        self.aside = Some(content);
        self
    }

    pub fn with_body(mut self, content: Content) -> Self {
        self.body = Some(content);
        self
    }

    /// Preferred width. The containing layout can constrain it further.
    pub fn with_width(mut self, width: f32) -> Self {
        if width.is_finite() && width > 0. {
            self.width = width;
        }
        self
    }

    /// Refine the outer GPUI layout (for example, margin or maximum width).
    pub fn with_style(mut self, style: impl Fn(Div) -> Div + 'static) -> Self {
        self.style = Some(Box::new(style));
        self
    }

    /// Set the maximum pointer tilt in degrees. Zero disables tilt.
    pub fn with_tilt(mut self, tilt: f32) -> Self {
        if tilt.is_finite() {
            self.tilt = tilt.max(0.);
        }
        self
    }

    /// Use an opaque panel when the host requests reduced transparency.
    pub fn with_solid_panel(mut self, solid: bool) -> Self {
        self.solid = solid;
        self
    }

    pub fn with_default_open(mut self, open: bool) -> Self {
        if !self.controlled {
            self.open = open;
            self.progress.reset(f32::from(open));
            self.fade = if open {
                Fade::visible()
            } else {
                Fade::default()
            };
        }
        self
    }

    pub fn with_open(mut self, open: bool) -> Self {
        self.controlled = true;
        self.open = open;
        self.progress.reset(f32::from(open));
        self.fade = if open {
            Fade::visible()
        } else {
            Fade::default()
        };
        self
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Apply host state without emitting another expansion request.
    pub fn set_open(&mut self, open: bool, w: &mut Window, cx: &mut Context<Self>) {
        if open != self.open {
            if !open && self.group_focus.contains_focused(w, cx) {
                self.focus.focus(w, cx);
            }
            self.open = open;
            self.progress.duration(f32::from(open), EXPAND_SECONDS, cx);
            self.fade.set(open, if open { 0.18 } else { 0.14 }, cx);
            cx.notify();
        }
    }

    fn activate(&mut self, event: &ClickEvent, w: &mut Window, cx: &mut Context<Self>) {
        if matches!(event, ClickEvent::Touch(_)) {
            self.leave(cx);
        }
        self.focus.focus(w, cx);
        self.request_open(!self.open, w, cx);
        cx.stop_propagation();
    }

    fn request_open(&mut self, open: bool, w: &mut Window, cx: &mut Context<Self>) {
        if !self.controlled {
            self.set_open(open, w, cx);
        }
        cx.emit(GlassCardOpenChanged(open));
    }

    fn point(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        let bounds = self.frame_bounds.get();
        if bounds.size.width <= px(0.) || bounds.size.height <= px(0.) || disabled_motion(cx) {
            return;
        }
        let offset = event.position - bounds.center();
        self.pointer[0].set((2. * offset.x / bounds.size.width).clamp(-1., 1.), cx);
        self.pointer[1].set((2. * offset.y / bounds.size.height).clamp(-1., 1.), cx);
        self.highlight.set(1., cx);
        cx.notify();
    }

    fn leave(&mut self, cx: &mut Context<Self>) {
        for axis in &mut self.pointer {
            axis.set(0., cx);
        }
        self.highlight.set(0., cx);
        cx.notify();
    }

    fn decoded(&self, w: &mut Window, cx: &mut App) -> Option<Arc<RenderImage>> {
        match &self.image {
            ImageSource::Render(image) => Some(image.clone()),
            ImageSource::Image(image) => image.clone().use_render_image(w, cx),
            ImageSource::Resource(resource) => w.use_asset::<ImgResourceLoader>(resource, cx)?.ok(),
            ImageSource::Custom(load) => load(w, cx)?.ok(),
        }
    }
}

impl Render for GlassCard {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let paint = Paint::get(cx);
        let dark = Theme::global(cx).mode == ThemeMode::Dark;
        let ink: Hsla = rgb(if dark { 0xf2f2f2 } else { 0x0b0b0b }).into();
        let secondary: Hsla = rgb(if dark { 0xb5b5b5 } else { 0x585858 }).into();
        let radius = surface_radius(cx);
        let reduced = disabled_motion(cx);
        let measured = self.frame_bounds.get().size.width;
        let width = if measured > px(0.) {
            measured
        } else {
            px(self.width).min(w.viewport_size().width)
        };
        let height = width * 1.25;
        let open = self.progress.get(w, cx).clamp(0., 1.);
        let fade = self.fade.get(w, cx);
        let [x, y] = if reduced {
            [0., 0.]
        } else {
            self.pointer.map(|axis| axis.get(w, cx))
        };
        let rim = if reduced {
            0.
        } else {
            self.highlight.get(w, cx)
        };
        let header_height = px(if self.subtitle.is_some() { 73. } else { 51. });
        let inset = px(INSET).min(width / 5.);
        let max_body = (height - inset * 2. - header_height).max(px(0.));
        let panel_height = header_height + self.body_height.get().min(max_body) * open;
        let image = self
            .decoded(w, cx)
            .filter(|image| image.as_bytes(0).is_some());
        let blur = image
            .as_ref()
            .filter(|_| !self.solid)
            .and_then(|image| w.use_asset::<GlassBlur>(&PhotoFrame(image.clone()), cx))
            .flatten();
        let tint: Hsla = rgb(if dark { 0x191919 } else { 0xffffff }).into();
        let tint = tint.opacity(if blur.is_some() { 0.62 } else { 1. });
        let layers = GlassLayers {
            radius,
            inset,
            panel_height,
            open,
            tint,
            dark,
            rim,
            pointer: [x, y],
            tilt: self.tilt,
            fallback: paint.tray,
        };
        let panel_bounds = self.panel_bounds.clone();
        let artwork = canvas(
            move |bounds, _, _| {
                let panel = layers.panel(bounds);
                panel_bounds.set(panel);
                panel
            },
            move |bounds, panel, w, _| {
                layers.paint(bounds, panel, image.as_ref(), blur.as_ref(), w);
            },
        )
        .absolute()
        .inset_0();

        let header = div()
            .id("glass-card-header")
            .debug_selector(|| "glass-card-header".into())
            .track_focus(&self.focus)
            .role(Role::Button)
            .aria_label(self.title.clone())
            .aria_expanded(self.open)
            .w_full()
            .h(header_height)
            .flex_none()
            .px_4()
            .flex()
            .items_center()
            .justify_between()
            .gap_3()
            .cursor_pointer()
            .border_1()
            .border_color(transparent_black())
            .rounded(radius)
            .focus_visible(move |s| s.border_color(ink.opacity(0.85)))
            .on_click(cx.listener(Self::activate))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(px(1.))
                    .child(
                        div()
                            .text_size(px(16.))
                            .line_height(px(22.4))
                            .font_weight(FontWeight::MEDIUM)
                            .truncate()
                            .child(self.title.clone()),
                    )
                    .when_some(self.subtitle.as_ref(), |d, slot| {
                        d.child(
                            div()
                                .text_size(px(14.))
                                .line_height(px(19.6))
                                .text_color(secondary)
                                .truncate()
                                .child(slot(w, cx)),
                        )
                    }),
            )
            .when_some(self.aside.as_ref(), |d, slot| {
                d.child(div().flex_none().text_size(px(14.)).child(slot(w, cx)))
            });

        let body_height = self.body_height.clone();
        let panel = div()
            .on_children_prepainted(move |children, w, _| {
                if let Some(body) = children.get(1)
                    && body_height.replace(body.size.height) != body.size.height
                {
                    w.request_animation_frame();
                }
            })
            .id("glass-card-panel")
            .absolute()
            .bottom(inset)
            .left(inset)
            .right(inset)
            .h(panel_height)
            .overflow_hidden()
            .rounded(radius)
            .text_color(ink)
            .flex()
            .flex_col()
            .child(header)
            .when_some(self.body.as_ref().filter(|_| self.open), |d, body| {
                d.child(
                    div()
                        .id("glass-card-details")
                        .debug_selector(|| "glass-card-details".into())
                        .role(Role::Region)
                        .aria_label(self.title.clone())
                        .relative()
                        .top(px(if reduced { 0. } else { 6. * (1. - fade) }))
                        .w_full()
                        .max_h(max_body)
                        .flex_none()
                        .overflow_y_scroll()
                        .px(px(17.))
                        .pb(px(16.))
                        .opacity(fade)
                        .child(body(w, cx)),
                )
            });

        let frame_bounds = self.frame_bounds.clone();
        let shell = div()
            .w(px(self.width))
            .max_w_full()
            .on_children_prepainted(move |children, w, _| {
                if let Some(&next) = children.first()
                    && frame_bounds.replace(next).size.width != next.size.width
                {
                    w.request_animation_frame();
                }
            })
            .child(
                div()
                    .id(self.id.clone())
                    .relative()
                    .track_focus(&self.group_focus)
                    .tab_stop(false)
                    .w_full()
                    .h(height)
                    .on_mouse_move(cx.listener(Self::point))
                    .on_hover(cx.listener(|card, over: &bool, _, cx| {
                        if !over {
                            card.leave(cx);
                        }
                    }))
                    .on_key_down(cx.listener(|card, event: &KeyDownEvent, w, cx| {
                        if card.open && event.keystroke.key == "escape" {
                            card.focus.focus(w, cx);
                            card.request_open(false, w, cx);
                            cx.stop_propagation();
                        }
                    }))
                    .child(
                        div()
                            .absolute()
                            .inset_0()
                            .rounded(radius)
                            .child(artwork)
                            .child(
                                div()
                                    .id("glass-card-photo")
                                    .debug_selector(|| "glass-card-photo".into())
                                    .absolute()
                                    .inset_0()
                                    .cursor_pointer()
                                    .when(!self.image_alt.is_empty(), |d| {
                                        d.role(Role::Image).aria_label(self.image_alt.clone())
                                    })
                                    .on_click(cx.listener(|card, event: &ClickEvent, w, cx| {
                                        if !card.panel_bounds.get().contains(&event.position()) {
                                            card.activate(event, w, cx);
                                        }
                                    })),
                            )
                            .child(panel),
                    ),
            );
        match &self.style {
            Some(refine) => refine(shell),
            None => shell,
        }
    }
}

#[derive(Clone, Copy)]
struct GlassLayers {
    radius: Pixels,
    inset: Pixels,
    panel_height: Pixels,
    open: f32,
    tint: Hsla,
    dark: bool,
    rim: f32,
    pointer: [f32; 2],
    tilt: f32,
    fallback: Hsla,
}

impl GlassLayers {
    fn panel(&self, bounds: Bounds<Pixels>) -> Bounds<Pixels> {
        Bounds::new(
            point(
                bounds.left() + self.inset,
                bounds.bottom() - self.inset - self.panel_height,
            ),
            size(bounds.size.width - self.inset * 2., self.panel_height),
        )
    }

    fn paint(
        &self,
        bounds: Bounds<Pixels>,
        panel: Bounds<Pixels>,
        photo: Option<&Arc<RenderImage>>,
        blur: Option<&Arc<RenderImage>>,
        w: &mut Window,
    ) {
        // Keep text and hitboxes on a stable baseline while the artwork and light move.
        let travel = self.pointer.map(|v| (v * self.tilt).to_radians().sin());
        let pan = point(px(travel[0] * 20.), px(travel[1] * 20.));
        let mut crop = bounds;
        w.paint_drop_shadows(
            bounds,
            self.radius.into(),
            &[BoxShadow::new(px(0.), px(22.), black().opacity(0.34)).blur_radius(px(55.))],
        );
        w.paint_quad(fill(bounds, self.fallback).corner_radii(self.radius));
        if let Some(photo) = photo {
            let natural = photo.size(0).map(|d| px(d.0 as f32));
            if natural.width > px(0.) && natural.height > px(0.) {
                let cover =
                    (bounds.size.width / natural.width).max(bounds.size.height / natural.height);
                let zoom = 1.001 + 0.049 * self.open + 0.04 * self.rim;
                let size = natural.map(|p| p * (cover * zoom));
                let inset = (size - bounds.size) / 2.;
                let offset = point(
                    pan.x.clamp(-inset.width, inset.width),
                    pan.y.clamp(-inset.height, inset.height),
                );
                crop = Bounds::new(
                    bounds.center() - point(size.width / 2., size.height / 2.) + offset,
                    size,
                );
                let _ = w.paint_image(bounds, crop, self.radius.into(), photo.clone(), 0, false);
            }
        }
        let shade = linear_gradient(
            180.,
            linear_color_stop(transparent_black(), 0.55),
            linear_color_stop(black().opacity(0.12 + 0.10 * self.open), 1.),
        );
        w.paint_quad(fill(bounds, shade).corner_radii(self.radius));

        w.paint_drop_shadows(
            panel,
            self.radius.into(),
            &[
                BoxShadow::new(px(0.), px(12.), black().opacity(0.16)).blur_radius(px(32.)),
                BoxShadow::new(px(0.), px(2.), black().opacity(0.08)).blur_radius(px(6.)),
            ],
        );
        if let Some(blur) = blur {
            let _ = w.paint_image(panel, crop, self.radius.into(), blur.clone(), 0, false);
            let shade_alpha = |y: Pixels| {
                let fraction = (y - bounds.top()) / bounds.size.height;
                black()
                    .opacity(((fraction - 0.55) / 0.45).clamp(0., 1.) * (0.12 + 0.10 * self.open))
            };
            let shade = linear_gradient(
                180.,
                linear_color_stop(shade_alpha(panel.top()), 0.),
                linear_color_stop(shade_alpha(panel.bottom()), 1.),
            );
            w.paint_quad(fill(panel, shade).corner_radii(self.radius));
        }
        w.paint_quad(fill(panel, self.tint).corner_radii(self.radius));
        let light_angle = self.pointer[0].atan2(-self.pointer[1]).to_degrees() + 180.;
        let base = if self.dark { 0.14 } else { 0.42 };
        let edge = linear_gradient(
            light_angle,
            linear_color_stop(white().opacity(base), 0.15),
            linear_color_stop(white().opacity(base + self.rim * 0.42), 1.),
        );
        let r = (self.radius - px(0.5)).min(panel.size.width / 2.);
        let l = panel.left() + px(0.5);
        let t = panel.top() + px(0.5);
        let b = panel.bottom() - px(0.5);
        let right = panel.right() - px(0.5);
        let radii = point(r, r);
        let mut stroke = PathBuilder::stroke(px(1.));
        stroke.move_to(point(l + r, t));
        stroke.line_to(point(right - r, t));
        stroke.arc_to(radii, px(0.), false, true, point(right, t + r));
        stroke.line_to(point(right, b - r));
        stroke.arc_to(radii, px(0.), false, true, point(right - r, b));
        stroke.line_to(point(l + r, b));
        stroke.arc_to(radii, px(0.), false, true, point(l, b - r));
        stroke.line_to(point(l, t + r));
        stroke.arc_to(radii, px(0.), false, true, point(l + r, t));
        stroke.close();
        if let Ok(path) = stroke.build() {
            w.paint_path(path, edge);
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
struct PhotoFrame(Arc<RenderImage>);

impl Hash for PhotoFrame {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.id.hash(state);
    }
}

enum GlassBlur {}

impl Asset for GlassBlur {
    type Source = PhotoFrame;
    type Output = Option<Arc<RenderImage>>;

    #[allow(
        clippy::manual_async_fn,
        reason = "The worker future cannot capture the UI-thread App."
    )]
    fn load(src: PhotoFrame, _: &mut App) -> impl Future<Output = Self::Output> + Send + 'static {
        async move {
            let image = src.0;
            let size = image.size(0);
            let original = RgbaImage::from_raw(
                size.width.0.try_into().ok()?,
                size.height.0.try_into().ok()?,
                image.as_bytes(0)?.to_vec(),
            )?;
            let sampled = imageops::thumbnail(&original, 180, 225);
            let sigma = (22. * sampled.width() as f32 / 360.).max(1.);
            let mut blurred = imageops::fast_blur(&sampled, sigma);
            // RenderImage stores BGRA. Work in that order so the atlas needs no conversion.
            for pixel in blurred.pixels_mut() {
                let [b, g, r, _] = pixel.0.map(f32::from);
                let luminance = 0.0722 * b + 0.7152 * g + 0.2126 * r;
                for channel in &mut pixel.0[..3] {
                    *channel = (luminance + 1.7 * (f32::from(*channel) - luminance))
                        .round()
                        .clamp(0., 255.) as u8;
                }
            }
            Some(Arc::new(RenderImage::new([Frame::new(blurred)])))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{GlassCard, GlassCardOpenChanged};
    use crate::foundation::Theme;
    use gpui::{
        Context, Entity, FocusHandle, IntoElement, KeyUpEvent, Keystroke, Modifiers, Render, Role,
        TestAppContext, VisualTestContext, Window, div, point, prelude::*, px, size,
    };
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
        time::Duration,
    };

    struct Harness {
        card: Entity<GlassCard>,
        action_focus: FocusHandle,
        actions: Rc<Cell<usize>>,
    }

    fn mount(cx: &mut Context<Harness>, controlled: Option<bool>) -> Harness {
        let action_focus = cx.focus_handle().tab_stop(true);
        let actions = Rc::new(Cell::new(0));
        let button_focus = action_focus.clone();
        let count = actions.clone();
        let card = cx.new(|cx| {
            let card = GlassCard::new("", "", "Profile", cx)
                .with_subtitle(Box::new(|_, _| "Design lead".into_any_element()))
                .with_body(Box::new(move |_, _| {
                    let count = count.clone();
                    div()
                        .id("follow")
                        .debug_selector(|| "glass-card-test-action".into())
                        .track_focus(&button_focus)
                        .role(Role::Button)
                        .h(px(44.))
                        .child("Follow")
                        .on_click(move |_, _, _| count.set(count.get() + 1))
                        .into_any_element()
                }));
            match controlled {
                Some(open) => card.with_open(open),
                None => card,
            }
        });
        Harness {
            card,
            action_focus,
            actions,
        }
    }

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().w(px(280.)).child(self.card.clone())
        }
    }

    fn tick(w: &mut VisualTestContext, ms: u64) {
        w.executor().advance_clock(Duration::from_millis(ms));
        w.run_until_parked();
        w.update(|w, cx| {
            w.simulate_next_frame(cx);
            w.refresh();
        });
        w.run_until_parked();
    }

    #[test]
    fn photo_expands_header_moves_up_details_take_focus_and_escape_restores_it() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) = app.add_window_view(|_, cx| mount(cx, None));
        tick(w, 16);
        let (card, action_focus, actions) = w.update(|_, cx| {
            let view = view.read(cx);
            (
                view.card.clone(),
                view.action_focus.clone(),
                view.actions.clone(),
            )
        });
        let photo = w.debug_bounds("glass-card-photo").unwrap();
        assert_eq!(photo.size, size(px(280.), px(350.)));
        let closed = w.debug_bounds("glass-card-header").unwrap();
        assert!(w.debug_bounds("glass-card-test-action").is_none());

        w.simulate_click(
            point(photo.center().x, photo.top() + px(32.)),
            Modifiers::default(),
        );
        assert!(w.update(|_, cx| card.read(cx).is_open()));
        tick(w, 900);
        let opened = w.debug_bounds("glass-card-header").unwrap();
        assert!(opened.top() < closed.top() - px(55.));
        w.update(|w, cx| w.focus_next(cx));
        assert!(w.update(|w, _| action_focus.is_focused(w)));
        tick(w, 16);
        w.simulate_keystrokes("enter");
        w.simulate_event(KeyUpEvent {
            keystroke: Keystroke::parse("enter").unwrap(),
        });
        assert_eq!(actions.get(), 1);
        assert!(w.update(|_, cx| card.read(cx).is_open()));
        w.simulate_keystrokes("escape");
        assert!(w.update(|w, cx| card.read(cx).focus.is_focused(w)));
        tick(w, 900);
        assert!(w.debug_bounds("glass-card-test-action").is_none());
        assert_eq!(w.debug_bounds("glass-card-header").unwrap(), closed);
    }

    #[test]
    fn controlled_requests_preserve_authority_and_rapid_reversal_keeps_height_continuous() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) = app.add_window_view(|_, cx| mount(cx, Some(false)));
        let card = w.update(|_, cx| view.read(cx).card.clone());
        let changes = Rc::new(RefCell::new(Vec::new()));
        let _subscription = w.update(|_, cx| {
            let changes = changes.clone();
            cx.subscribe(&card, move |_, e: &GlassCardOpenChanged, _| {
                changes.borrow_mut().push(e.0)
            })
        });
        tick(w, 16);
        let header = w.debug_bounds("glass-card-header").unwrap().center();
        w.simulate_click(header, Modifiers::default());
        assert_eq!(&*changes.borrow(), &[true]);
        assert!(!w.update(|_, cx| card.read(cx).is_open()));

        w.update(|w, cx| card.update(cx, |card, cx| card.set_open(true, w, cx)));
        tick(w, 65);
        w.update(|w, cx| {
            card.update(cx, |card, cx| {
                let before = card.progress.value(cx);
                assert!(before > 0. && before < 1.);
                card.set_open(false, w, cx);
                assert!((card.progress.value(cx) - before).abs() < 0.0001);
            })
        });
        tick(w, 900);
        assert_eq!(w.update(|_, cx| card.read(cx).progress.value(cx)), 0.);
        assert_eq!(&*changes.borrow(), &[true]);

        w.update(|w, cx| {
            cx.update_global::<Theme, _>(|theme, _| theme.reduced_motion = true);
            card.update(cx, |card, cx| {
                card.set_open(true, w, cx);
                assert_eq!(card.progress.value(cx), 1.);
            });
        });
    }
}
