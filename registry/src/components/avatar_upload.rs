//! Local avatar preview and normalized crop with a ticketed upload boundary.
pub use crate::primitives::form_files::PickedFile;
use crate::primitives::{
    family::{Paint, PhaseClock, Slide, standard_ease},
    form_chrome::*,
    form_files::{self, FileResult},
    glyph::{Glyph, icon},
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc, sync::Arc};
#[derive(Clone, Copy, Debug)]
pub struct AvatarCrop {
    pub zoom: f32,
    pub x: f32,
    pub y: f32,
}
impl Default for AvatarCrop {
    fn default() -> Self {
        Self {
            zoom: 1.,
            x: 0.,
            y: 0.,
        }
    }
}
#[derive(Clone, Debug)]
pub enum AvatarEvent {
    Upload {
        ticket: u64,
        file: PickedFile,
        crop: AvatarCrop,
    },
    Removed,
}
#[derive(Clone)]
struct Photo {
    file: PickedFile,
    image: Arc<Image>,
    crop: AvatarCrop,
}
pub struct AvatarUpload {
    name: SharedString,
    current: Option<Photo>,
    draft: Option<Photo>,
    uploading: bool,
    progress: Slide,
    size: Slide,
    inset: Slide,
    saved: PhaseClock,
    ticket: u64,
    error: SharedString,
    start: Option<(Point<Pixels>, AvatarCrop)>,
    bounds: Rc<Cell<Bounds<Pixels>>>,
    _drop: Option<Subscription>,
}
impl EventEmitter<AvatarEvent> for AvatarUpload {}
const LIMIT: u64 = 5 * 1024 * 1024;
impl AvatarUpload {
    pub fn new(name: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        let bounds: Rc<Cell<Bounds<Pixels>>> = Default::default();
        let drop = form_files::drop_subscription(bounds.clone(), LIMIT, cx, Self::received, None);
        Self {
            name: name.into(),
            current: None,
            draft: None,
            uploading: false,
            progress: Slide::snappy(0.),
            size: Slide::snappy(88.),
            inset: Slide::snappy(0.),
            saved: Default::default(),
            ticket: 0,
            error: "".into(),
            start: None,
            bounds,
            _drop: drop,
        }
    }
    pub fn set_photo(&mut self, file: PickedFile) {
        if let Some(format) = file.image_format() {
            self.current = Some(Photo {
                image: Arc::new(Image::from_bytes(format, file.bytes.to_vec())),
                file,
                crop: AvatarCrop::default(),
            })
        }
    }
    pub fn select_photo(&mut self, file: PickedFile, cx: &mut Context<Self>) {
        self.received(Ok(vec![file]), cx)
    }
    fn received(&mut self, r: FileResult, cx: &mut Context<Self>) {
        if self.uploading {
            return;
        }
        self.error = "".into();
        match r {
            Err(e) => self.error = e.into(),
            Ok(fs) => {
                if let Some(f) = fs.into_iter().next() {
                    if f.size > LIMIT {
                        self.error =
                            tr(cx, "Image must be smaller than 5 MB", "图片不能超过 5 MB").into()
                    } else if let Some(t) = f.image_format() {
                        self.draft = Some(Photo {
                            image: Arc::new(Image::from_bytes(t, f.bytes.to_vec())),
                            file: f,
                            crop: AvatarCrop::default(),
                        })
                    } else {
                        self.error = tr(
                            cx,
                            "Choose a PNG, JPG or WebP image",
                            "请选择 PNG、JPG 或 WebP 图片",
                        )
                        .into()
                    }
                }
            }
        }
        cx.notify()
    }
    fn choose(&mut self, cx: &mut Context<Self>) {
        let t = form_files::choose(false, "image/png,image/jpeg,image/webp", LIMIT, cx);
        cx.spawn(async move |this, cx| {
            let r = t.await;
            let _ = this.update(cx, |s, cx| s.received(r, cx));
        })
        .detach()
    }
    fn upload(&mut self, cx: &mut Context<Self>) {
        if self.uploading {
            return;
        }
        if let Some(p) = &self.draft {
            self.ticket += 1;
            self.error = "".into();
            self.uploading = true;
            self.progress.reset(0.);
            cx.emit(AvatarEvent::Upload {
                ticket: self.ticket,
                file: p.file.clone(),
                crop: p.crop,
            });
            cx.notify()
        }
    }
    pub fn report(&mut self, ticket: u64, progress: f32, cx: &mut Context<Self>) {
        if self.uploading && self.ticket == ticket {
            self.progress.duration(progress.clamp(0., 1.), 0.16, cx);
            cx.notify()
        }
    }
    pub fn finish(&mut self, ticket: u64, result: Result<(), String>, cx: &mut Context<Self>) {
        if !self.uploading || self.ticket != ticket {
            return;
        }
        self.uploading = false;
        match result {
            Ok(()) => {
                self.current = self.draft.take();
                self.saved.restart(cx);
                self.error = "".into()
            }
            Err(e) => self.error = e.into(),
        }
        cx.notify()
    }
    fn zoom(&mut self, delta: f32, cx: &mut Context<Self>) {
        if let Some(p) = &mut self.draft {
            p.crop.zoom = (p.crop.zoom + delta).clamp(1., 3.);
            let lim = (p.crop.zoom - 1.) * 0.5;
            p.crop.x = p.crop.x.clamp(-lim, lim);
            p.crop.y = p.crop.y.clamp(-lim, lim);
            cx.notify()
        }
    }
}
impl Render for AvatarUpload {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let draft = self.draft.is_some();
        self.size
            .duration(if draft && !self.uploading { 184. } else { 88. }, 0.4, cx);
        let side = self.size.get(w, cx);
        let saved = standard_ease(self.saved.time(0.48, w, cx) / 0.48);
        self.inset.duration(self.uploading as u8 as f32, 0.26, cx);
        let inset = self.inset.get(w, cx).clamp(0., 1.);
        let rect = self.bounds.clone();
        let mut inner = div()
            .size(px(side))
            .relative()
            .rounded_full()
            .overflow_hidden()
            .flex()
            .items_center()
            .justify_center()
            .bg(p.hover);
        if let Some(ph) = self.draft.as_ref().or(self.current.as_ref()) {
            let source = ph.image.clone();
            let c = ph.crop;
            inner = inner.child(
                canvas(
                    |_, _, _| (),
                    move |bounds, _, w, cx| {
                        let Some(image) = source.clone().use_render_image(w, cx) else {
                            return;
                        };
                        let scale = 1. - 0.10 * inset;
                        let shrunk = size(bounds.size.width * scale, bounds.size.height * scale);
                        let bounds = Bounds::new(
                            bounds.center() - point(shrunk.width * 0.5, shrunk.height * 0.5),
                            shrunk,
                        );
                        let mut dst = ObjectFit::Cover.get_bounds(bounds, image.size(0));
                        dst.size = size(dst.size.width * c.zoom, dst.size.height * c.zoom);
                        dst.origin = bounds.center()
                            - point(dst.size.width * 0.5, dst.size.height * 0.5)
                            + point(bounds.size.width * c.x, bounds.size.height * c.y);
                        let _ = w.paint_image(
                            bounds,
                            dst,
                            Corners::all(bounds.size.width * 0.5),
                            image,
                            0,
                            false,
                        );
                    },
                )
                .size_full(),
            );
        } else {
            let initials = self
                .name
                .split_whitespace()
                .filter_map(|s| s.chars().next())
                .take(2)
                .collect::<String>();
            inner = inner.child(if initials.is_empty() {
                "＋".into()
            } else {
                initials
            })
        }
        if self.uploading || inset > 0. {
            let progress = self.progress.get(w, cx).clamp(0., 1.);
            inner = inner.child(
                div()
                    .absolute()
                    .inset_0()
                    .rounded_full()
                    .bg(black().opacity(0.38 * inset))
                    .child(avatar_progress(side, progress, p.bright.opacity(inset))),
            );
        }
        let target = div()
            .relative()
            .on_children_prepainted(move |b, _, _| {
                if let Some(v) = b.first() {
                    rect.set(*v)
                }
            })
            .id("photo")
            .aria_label(tr(cx, "Change profile photo", "更换头像"))
            .role(Role::Button)
            .tab_index(0)
            .rounded_full()
            .cursor_pointer()
            .child(inner)
            .when(saved < 1., |d| {
                d.child(
                    div()
                        .absolute()
                        .left(px(-side * 0.06 * saved))
                        .top(px(-side * 0.06 * saved))
                        .size(px(side * (1. + 0.12 * saved)))
                        .rounded_full()
                        .border_2()
                        .border_color(p.success.opacity(1. - saved)),
                )
            })
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|s, e: &MouseDownEvent, _, _| {
                    if !s.uploading
                        && let Some(p) = &s.draft
                    {
                        s.start = Some((e.position, p.crop))
                    }
                }),
            )
            .on_mouse_move(cx.listener(|s, e: &MouseMoveEvent, _, cx| {
                if let Some((origin, c)) = s.start
                    && let Some(d) = &mut s.draft
                {
                    let lim = (c.zoom - 1.) * 0.5;
                    d.crop.x = (c.x
                        + (e.position.x - origin.x) / s.bounds.get().size.width.max(px(1.)))
                    .clamp(-lim, lim);
                    d.crop.y = (c.y
                        + (e.position.y - origin.y) / s.bounds.get().size.width.max(px(1.)))
                    .clamp(-lim, lim);
                    cx.notify()
                }
            }))
            .on_mouse_up(MouseButton::Left, cx.listener(|s, _, _, _| s.start = None))
            .on_mouse_up_out(MouseButton::Left, cx.listener(|s, _, _, _| s.start = None))
            .on_pinch(cx.listener(|s, e: &PinchEvent, _, cx| {
                if !s.uploading {
                    s.zoom(e.delta, cx)
                }
            }))
            .on_click(cx.listener(|s, _, _, cx| {
                if s.draft.is_none() && !s.uploading {
                    s.choose(cx)
                }
            }));
        let mut details = col().child(tr(cx, "Profile photo", "个人头像")).child(hint(
            tr(
                cx,
                "PNG, JPG or WebP, up to 5 MB",
                "PNG、JPG 或 WebP，最大 5 MB",
            ),
            cx,
        ));
        if !draft {
            let mut actions = row().flex_wrap().child(
                button(
                    "replace",
                    if self.current.is_some() {
                        tr(cx, "Replace photo", "更换照片")
                    } else {
                        tr(cx, "Choose photo", "选择照片")
                    },
                    cx,
                )
                .border_1()
                .border_color(p.line)
                .on_click(cx.listener(|s, _, _, cx| s.choose(cx))),
            );
            if self.current.is_some() {
                actions = actions.child(button("remove", tr(cx, "Remove", "移除"), cx).on_click(
                    cx.listener(|s, _, _, cx| {
                        s.current = None;
                        cx.emit(AvatarEvent::Removed);
                        cx.notify()
                    }),
                ))
            }
            details = details.child(actions)
        } else if self.uploading {
            details = details.child(hint(
                format!(
                    "{} {}%",
                    tr(cx, "Uploading", "上传中"),
                    (self.progress.get(w, cx) * 100.) as i32
                ),
                cx,
            ))
        } else {
            details =
                details.child(
                    row()
                        .flex_wrap()
                        .child(
                            glyph("out", tr(cx, "Zoom out", "缩小"), Glyph::Minus, cx)
                                .on_click(cx.listener(|s, _, _, cx| s.zoom(-0.2, cx))),
                        )
                        .child(
                            glyph("in", tr(cx, "Zoom in", "放大"), Glyph::Plus, cx)
                                .on_click(cx.listener(|s, _, _, cx| s.zoom(0.2, cx))),
                        )
                        .child(
                            button(
                                "save",
                                if self.error.is_empty() {
                                    tr(cx, "Save", "保存")
                                } else {
                                    tr(cx, "Retry", "重试")
                                },
                                cx,
                            )
                            .on_click(cx.listener(|s, _, _, cx| s.upload(cx))),
                        )
                        .child(button("cancel", tr(cx, "Cancel", "取消"), cx).on_click(
                            cx.listener(|s, _, _, cx| {
                                s.draft = None;
                                s.error = "".into();
                                cx.notify()
                            }),
                        )),
                )
        }
        section(470., cx)
            .child(
                row()
                    .flex_wrap()
                    .child(form_files::drop_target(target, cx, LIMIT, Self::received))
                    .child(details),
            )
            .when(!self.error.is_empty(), |d| {
                d.child(
                    row()
                        .child(icon(Glyph::Alert, 14., p.danger))
                        .child(hint(self.error.clone(), cx).text_color(p.danger)),
                )
            })
    }
}

fn avatar_progress(side: f32, progress: f32, ink: Hsla) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |b, _, w, _| {
            let mut stroke = PathBuilder::stroke(px(2.));
            for i in 0..=64 {
                let angle = std::f32::consts::TAU * (progress * i as f32 / 64. - 0.25);
                let origin = b.center()
                    + point(
                        px((side * 0.5 - 2.) * angle.cos()),
                        px((side * 0.5 - 2.) * angle.sin()),
                    );
                if i == 0 {
                    stroke.move_to(origin);
                } else {
                    stroke.line_to(origin);
                }
            }
            if progress > 0.
                && let Ok(path) = stroke.build()
            {
                w.paint_path(path, ink);
            }
        },
    )
    .size(px(side))
}
