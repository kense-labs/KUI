//! Host-controlled image reveal; no generation or network request is performed.
use crate::primitives::{
    family::{Paint, Slide, Tone, face},
    nav::tr,
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RevealStatus {
    Queued,
    Generating,
    Done,
    Error,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImageRevealEvent {
    Retry,
}
fn accepts_update(current: u64, status: RevealStatus, incoming: u64) -> bool {
    current == incoming && matches!(status, RevealStatus::Queued | RevealStatus::Generating)
}
pub struct ImageReveal {
    label: SharedString,
    source: Option<ImageSource>,
    run: u64,
    status: RevealStatus,
    progress: Slide,
    steps: Option<u32>,
    aspect: f32,
    error: SharedString,
    width: Rc<Cell<Pixels>>,
    in_view: bool,
}
impl EventEmitter<ImageRevealEvent> for ImageReveal {}
impl ImageReveal {
    pub fn new(label: impl Into<SharedString>, aspect: f32) -> Self {
        Self {
            label: label.into(),
            source: None,
            run: 0,
            status: RevealStatus::Queued,
            progress: Slide::new(0.),
            steps: None,
            aspect: if aspect.is_finite() {
                aspect.clamp(0.2, 5.)
            } else {
                1.
            },
            error: "".into(),
            width: Rc::new(Cell::new(px(0.))),
            in_view: true,
        }
    }
    /// Optionally display completed sampler steps alongside the progress ring.
    pub fn with_steps(mut self, steps: u32) -> Self {
        self.steps = (steps > 0).then_some(steps);
        self
    }
    /// Begin a fresh host job; later updates are accepted only for this ID.
    pub fn begin(&mut self, run: u64, cx: &mut Context<Self>) {
        self.run = run;
        self.status = RevealStatus::Queued;
        self.progress.reset(0.);
        self.source = None;
        self.error = "".into();
        cx.notify();
    }
    /// Preview is optional. The host owns image loading and permission decisions.
    pub fn set_preview(
        &mut self,
        run: u64,
        source: impl Into<ImageSource>,
        cx: &mut Context<Self>,
    ) {
        if accepts_update(self.run, self.status, run) {
            self.source = Some(source.into());
            cx.notify();
        }
    }
    pub fn set_progress(&mut self, run: u64, progress: f32, cx: &mut Context<Self>) {
        if accepts_update(self.run, self.status, run) {
            self.status = RevealStatus::Generating;
            let progress = if progress.is_finite() {
                progress.clamp(0., 1.)
            } else {
                0.
            };
            self.progress.duration(progress, 0.16, cx);
            cx.notify();
        }
    }
    pub fn finish(&mut self, run: u64, source: impl Into<ImageSource>, cx: &mut Context<Self>) {
        if accepts_update(self.run, self.status, run) {
            self.source = Some(source.into());
            self.status = RevealStatus::Done;
            self.progress.duration(1., 0.16, cx);
            cx.notify();
        }
    }
    pub fn fail(&mut self, run: u64, message: impl Into<SharedString>, cx: &mut Context<Self>) {
        if accepts_update(self.run, self.status, run) {
            self.error = message.into();
            self.status = RevealStatus::Error;
            cx.notify();
        }
    }
    pub fn status(&self) -> RevealStatus {
        self.status
    }
}
impl Render for ImageReveal {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let radius = crate::foundation::surface_radius(cx);
        let width = if self.width.get() > px(0.) {
            self.width.get()
        } else {
            (w.viewport_size().width - px(32.))
                .min(px(560.))
                .max(px(1.))
        };
        let height = (width / self.aspect).clamp(px(80.), px(600.));
        let progress = if self.in_view && w.is_visible() {
            self.progress.get(w, cx)
        } else {
            self.progress.value(cx)
        }
        .clamp(0., 1.);
        let name = match self.status {
            RevealStatus::Queued => tr(cx, "Queued", "排队中"),
            RevealStatus::Generating => tr(cx, "Generating image", "正在生成图片"),
            RevealStatus::Done => tr(cx, "Image ready", "图片已生成"),
            RevealStatus::Error => tr(cx, "Generation failed", "生成失败"),
        };
        let capture = self.width.clone();
        let viewport = w.viewport_size();
        let weak = cx.weak_entity();
        let mut frame = div()
            .on_children_prepainted(move |bounds, _, cx| {
                if let Some(b) = bounds.first() {
                    let visible = b.left() < viewport.width
                        && b.top() < viewport.height
                        && b.right() > px(0.)
                        && b.bottom() > px(0.);
                    let _ = weak.update(cx, |s, cx| {
                        if s.in_view != visible {
                            s.in_view = visible;
                            cx.notify();
                        }
                    });
                }
            })
            .id("image-reveal")
            .role(Role::Group)
            .aria_label(self.label.clone())
            .relative()
            .w_full()
            .max_w(px(560.))
            .aspect_ratio(self.aspect)
            .min_h(px(80.))
            .max_h(px(600.))
            .rounded(radius)
            .overflow_hidden()
            .bg(p.tray)
            .child(
                canvas(
                    move |b, w, _| {
                        if (capture.get() - b.size.width).abs() > px(0.5) {
                            capture.set(b.size.width);
                            w.request_animation_frame();
                        }
                    },
                    |_, _, _, _| (),
                )
                .absolute()
                .inset_0(),
            );
        let mut visual = div()
            .id("reveal-image")
            .role(Role::Image)
            .aria_label(format!("{} · {}", name, self.label))
            .absolute()
            .inset_0()
            .overflow_hidden();
        if let Some(source) = &self.source {
            if progress >= 0.999 {
                visual = visual.child(
                    img(source.clone())
                        .absolute()
                        .size_full()
                        .rounded(radius)
                        .object_fit(ObjectFit::Cover),
                );
            } else {
                visual = visual.child(
                    img(source.clone())
                        .absolute()
                        .size_full()
                        .rounded(radius)
                        .object_fit(ObjectFit::Cover)
                        .opacity(0.35),
                );
                let front = height * progress;
                let feather = px(18.).min(front);
                let sharp = front - feather;
                for (top, band, opacity) in [
                    (px(0.), sharp, 1.),
                    (sharp, feather / 4., 0.86),
                    (sharp + feather / 4., feather / 4., 0.64),
                    (sharp + feather / 2., feather / 4., 0.40),
                    (sharp + feather * 0.75, feather / 4., 0.16),
                ] {
                    if band <= px(0.) {
                        continue;
                    }
                    visual = visual.child(
                        div()
                            .absolute()
                            .top(top)
                            .w_full()
                            .h(band)
                            .overflow_hidden()
                            .child(
                                img(source.clone())
                                    .absolute()
                                    .top(-top)
                                    .w_full()
                                    .h(height)
                                    .rounded(radius)
                                    .object_fit(ObjectFit::Cover)
                                    .opacity(opacity),
                            ),
                    );
                }
            }
        }
        if self.status == RevealStatus::Generating && progress < 0.9 {
            visual = visual.child(
                canvas(
                    |_, _, _| (),
                    move |bounds, _, window, _| {
                        let edge = bounds.top() + bounds.size.height * progress;
                        let cell = px(7.).max(bounds.size.width / 32.);
                        let (mut y, mut row) = (bounds.top(), 0u32);
                        while y < bounds.bottom() {
                            let reveal = ((y - edge) / px(20.) + 0.3).clamp(0., 1.);
                            if reveal > 0. {
                                let (mut x, mut col) = (bounds.left(), 0u32);
                                while x < bounds.right() {
                                    let hash = col
                                        .wrapping_mul(1_511_307)
                                        .wrapping_add(row.wrapping_mul(979_733));
                                    let hash =
                                        (hash ^ hash.rotate_left(11)).wrapping_mul(2_654_435_761);
                                    let sample = (hash & 255) as f32 / 255.;
                                    let shade = [0x7b8fa5, 0xc4a387, 0x7c8c78, 0x4d5968, 0xb2a9ae]
                                        [((hash >> 8) % 5) as usize];
                                    let color = rgb(shade)
                                        .opacity(reveal * (1. - progress) * (0.18 + sample * 0.54));
                                    window.paint_quad(fill(
                                        Bounds::new(point(x, y), size(cell, cell)),
                                        color,
                                    ));
                                    x += cell;
                                    col += 1;
                                }
                            }
                            y += cell;
                            row += 1;
                        }
                    },
                )
                .absolute()
                .inset_0(),
            );
        }
        frame = frame.child(visual);
        if self.status == RevealStatus::Error {
            frame = frame.child(
                div()
                    .absolute()
                    .inset_0()
                    .rounded(radius)
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap(px(12.))
                    .bg(p.panel.opacity(0.85))
                    .child(
                        div()
                            .id("reveal-error")
                            .role(Role::Alert)
                            .text_color(p.danger)
                            .child(self.error.clone()),
                    )
                    .child(
                        face(
                            "reveal-retry",
                            tr(cx, "Retry", "重试"),
                            32.,
                            8.,
                            Tone::Surface,
                            cx,
                        )
                        .px_2()
                        .child(tr(cx, "Retry", "重试"))
                        .on_click(cx.listener(|_, _, _, cx| cx.emit(ImageRevealEvent::Retry))),
                    ),
            );
        } else {
            frame = frame.child(
                div()
                    .absolute()
                    .bottom(px(12.))
                    .left(px(12.))
                    .flex()
                    .items_center()
                    .h(px(28.))
                    .pl(px(7.))
                    .pr(px(11.2))
                    .gap(px(8.))
                    .rounded_full()
                    .border_1()
                    .border_color(p.ink.opacity(0.08))
                    .bg(p.panel.opacity(0.72))
                    .text_size(px(13.))
                    .text_color(p.ink)
                    .child(
                        canvas(
                            |_, _, _| (),
                            move |b, _, w, _| {
                                let center = b.center();
                                let draw = |end: f32, color: Hsla, w: &mut Window| {
                                    let mut path = PathBuilder::stroke(px(1.5));
                                    for index in 0..=24 {
                                        let a = std::f32::consts::TAU * end * index as f32 / 24.
                                            - std::f32::consts::FRAC_PI_2;
                                        let point =
                                            center + point(px(5.5 * a.cos()), px(5.5 * a.sin()));
                                        if index == 0 {
                                            path.move_to(point);
                                        } else {
                                            path.line_to(point);
                                        }
                                    }
                                    if let Ok(path) = path.build() {
                                        w.paint_path(path, color);
                                    }
                                };
                                draw(1., p.ink.opacity(0.16), w);
                                if progress > 0. {
                                    draw(progress, p.ink, w);
                                }
                            },
                        )
                        .size(px(14.))
                        .flex_none(),
                    )
                    .child(if self.status == RevealStatus::Generating {
                        if let Some(steps) = self.steps {
                            let count = (progress * steps as f32).round() as u32;
                            if width < px(300.) {
                                format!("{count} / {steps}")
                            } else {
                                format!("{name} · {count} / {steps}")
                            }
                        } else {
                            format!("{} · {:.0}%", name, progress * 100.)
                        }
                    } else {
                        name.to_string()
                    }),
            );
        }
        frame
    }
}

#[cfg(test)]
mod tests {
    use super::{RevealStatus, accepts_update};
    #[test]
    fn finished_and_superseded_runs_ignore_late_results() {
        assert!(accepts_update(3, RevealStatus::Generating, 3));
        assert!(!accepts_update(3, RevealStatus::Done, 3));
        assert!(!accepts_update(3, RevealStatus::Error, 3));
        assert!(!accepts_update(4, RevealStatus::Generating, 3));
    }
}
