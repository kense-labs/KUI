//! Host-controlled image generation. No network, download or model is invoked by this view.
use crate::primitives::{
    editor::{EditEvent, Editor, EditorShortcutEvent, InputMode},
    family::{Paint, Slide, Tone, face},
    nav::tr,
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};

#[derive(Clone)]
pub struct GeneratedImage {
    pub id: SharedString,
    pub source: ImageSource,
    pub alt: SharedString,
    pub width: u32,
    pub height: u32,
}
impl GeneratedImage {
    pub fn new(
        id: impl Into<SharedString>,
        source: impl Into<ImageSource>,
        alt: impl Into<SharedString>,
        width: u32,
        height: u32,
    ) -> Self {
        Self {
            id: id.into(),
            source: source.into(),
            alt: alt.into(),
            width,
            height,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImageGenerationEvent {
    Generate {
        ticket: u64,
        index: usize,
        prompt: SharedString,
        model: SharedString,
        aspect: SharedString,
        variation_of: Option<SharedString>,
    },
    Cancel(u64),
    Select(Option<usize>),
    Upscale {
        ticket: u64,
        image_id: SharedString,
    },
    Download(SharedString),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TileStatus {
    Empty,
    Running,
    Ready,
    Failed,
    Stopped,
    Upscaling,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TileOperation {
    Generate,
    Variation,
    Upscale,
}
struct Tile {
    status: TileStatus,
    operation: TileOperation,
    variation_of: Option<SharedString>,
    aspect: f32,
    requested_aspect: f32,
    ticket: u64,
    progress: f32,
    visual: Slide,
    image: Option<GeneratedImage>,
    error: SharedString,
}
impl Default for Tile {
    fn default() -> Self {
        Self {
            status: TileStatus::Empty,
            operation: TileOperation::Generate,
            variation_of: None,
            aspect: 1.,
            requested_aspect: 1.,
            ticket: 0,
            progress: 0.,
            visual: Slide::new(0.),
            image: None,
            error: "".into(),
        }
    }
}
fn accepts_update(tile: &Tile, ticket: u64) -> bool {
    tile.ticket == ticket && matches!(tile.status, TileStatus::Running | TileStatus::Upscaling)
}
fn requested_ratio(index: usize) -> f32 {
    match index {
        1 => 4. / 5.,
        2 => 16. / 9.,
        _ => 1.,
    }
}
fn image_ratio(image: &GeneratedImage, fallback: f32) -> f32 {
    if image.width == 0 || image.height == 0 {
        fallback
    } else {
        (image.width as f32 / image.height as f32).clamp(0.2, 5.)
    }
}
fn prepare_tile(tile: &mut Tile, ticket: u64, variation: Option<SharedString>, ratio: f32) {
    tile.ticket = ticket;
    tile.status = TileStatus::Running;
    tile.progress = 0.;
    tile.visual.reset(0.);
    tile.operation = if variation.is_some() {
        TileOperation::Variation
    } else {
        TileOperation::Generate
    };
    tile.variation_of = variation;
    tile.requested_aspect = ratio;
    if tile.operation == TileOperation::Generate {
        tile.image = None;
        tile.aspect = ratio;
    }
    tile.error = "".into();
}
fn stop_tile(tile: &mut Tile) {
    tile.status = if tile.image.is_some() {
        TileStatus::Ready
    } else {
        TileStatus::Stopped
    };
}
pub struct ImageGeneration {
    editor: Entity<Editor>,
    prompt: String,
    models: Vec<SharedString>,
    model: usize,
    aspects: Vec<SharedString>,
    aspect: usize,
    tiles: Vec<Tile>,
    selected: Option<usize>,
    serial: u64,
    cursor: usize,
    width: Rc<Cell<Pixels>>,
    tile_focus: FocusHandle,
    _subscriptions: Vec<Subscription>,
}
impl EventEmitter<ImageGenerationEvent> for ImageGeneration {}
impl ImageGeneration {
    pub fn new(w: &mut Window, cx: &mut Context<Self>) -> Self {
        let editor = cx.new(|cx| {
            let mut e = Editor::new(tr(cx, "Image prompt", "图片描述"), InputMode::Multi, w, cx);
            e.set_placeholder(tr(cx, "Describe the image you want", "描述你想要的图片"));
            e.set_command_submit(true);
            e
        });
        let sub = cx.subscribe_in(
            &editor,
            w,
            |s: &mut Self, editor, event: &EditEvent, _, cx| {
                if *event == EditEvent::Changed {
                    s.prompt = editor.read(cx).value().to_string();
                    cx.notify();
                }
            },
        );
        let shortcut = cx.subscribe_in(
            &editor,
            w,
            |s: &mut Self, _, event: &EditorShortcutEvent, _, cx| {
                if *event == EditorShortcutEvent::CommandSubmit {
                    s.generate(cx);
                }
            },
        );
        Self {
            editor,
            prompt: String::new(),
            models: vec!["Kense".into()],
            model: 0,
            aspects: vec!["1:1".into(), "4:5".into(), "16:9".into()],
            aspect: 0,
            tiles: (0..4).map(|_| Tile::default()).collect(),
            selected: None,
            serial: 0,
            cursor: 0,
            width: Rc::new(Cell::new(px(0.))),
            tile_focus: cx.focus_handle().tab_stop(true),
            _subscriptions: vec![sub, shortcut],
        }
    }
    pub fn models(mut self, models: Vec<SharedString>) -> Self {
        if !models.is_empty() {
            self.models = models;
            self.model = 0;
        }
        self
    }
    pub fn set_prompt(&mut self, prompt: impl Into<String>, w: &Window, cx: &mut Context<Self>) {
        self.editor.update(cx, |e, cx| e.set_value(prompt, w, cx));
    }
    fn next_ticket(&mut self) -> u64 {
        self.serial = self.serial.wrapping_add(1);
        self.serial
    }
    fn start_one(
        &mut self,
        index: usize,
        variation_of: Option<SharedString>,
        cx: &mut Context<Self>,
    ) {
        if index >= self.tiles.len() || self.prompt.trim().is_empty() {
            return;
        }
        if matches!(
            self.tiles[index].status,
            TileStatus::Running | TileStatus::Upscaling
        ) {
            cx.emit(ImageGenerationEvent::Cancel(self.tiles[index].ticket));
        }
        let ticket = self.next_ticket();
        let ratio = requested_ratio(self.aspect);
        prepare_tile(&mut self.tiles[index], ticket, variation_of.clone(), ratio);
        cx.emit(ImageGenerationEvent::Generate {
            ticket,
            index,
            prompt: self.prompt.clone().into(),
            model: self.models[self.model].clone(),
            aspect: self.aspects[self.aspect].clone(),
            variation_of,
        });
        cx.notify();
    }
    pub fn generate(&mut self, cx: &mut Context<Self>) {
        if self.prompt.trim().is_empty() {
            return;
        }
        if self.selected.take().is_some() {
            cx.emit(ImageGenerationEvent::Select(None));
        }
        for i in 0..self.tiles.len() {
            self.start_one(i, None, cx);
        }
    }
    pub fn stop(&mut self, cx: &mut Context<Self>) {
        for tile in &mut self.tiles {
            if matches!(tile.status, TileStatus::Running | TileStatus::Upscaling) {
                cx.emit(ImageGenerationEvent::Cancel(tile.ticket));
                stop_tile(tile);
            }
        }
        cx.notify();
    }
    pub fn set_progress(&mut self, ticket: u64, progress: f32, cx: &mut Context<Self>) {
        if let Some(tile) = self.tiles.iter_mut().find(|t| accepts_update(t, ticket)) {
            tile.progress = if progress.is_finite() {
                progress.clamp(0., 1.)
            } else {
                0.
            };
            tile.visual.duration(tile.progress, 0.24, cx);
            cx.notify();
        }
    }
    pub fn finish(&mut self, ticket: u64, image: GeneratedImage, cx: &mut Context<Self>) {
        if let Some(tile) = self.tiles.iter_mut().find(|t| accepts_update(t, ticket)) {
            tile.aspect = image_ratio(&image, tile.requested_aspect);
            tile.image = Some(image);
            tile.variation_of = None;
            tile.status = TileStatus::Ready;
            tile.progress = 1.;
            tile.visual.duration(1., 0.24, cx);
            cx.notify();
        }
    }
    pub fn fail(&mut self, ticket: u64, message: impl Into<SharedString>, cx: &mut Context<Self>) {
        if let Some(tile) = self.tiles.iter_mut().find(|t| accepts_update(t, ticket)) {
            tile.status = TileStatus::Failed;
            tile.error = message.into();
            cx.notify();
        }
    }
    fn select(&mut self, index: usize, cx: &mut Context<Self>) {
        if self
            .tiles
            .get(index)
            .is_some_and(|t| t.status == TileStatus::Ready)
        {
            self.cursor = index;
            self.selected = if self.selected == Some(index) {
                None
            } else {
                Some(index)
            };
            cx.emit(ImageGenerationEvent::Select(self.selected));
            cx.notify();
        }
    }
    fn upscale(&mut self, cx: &mut Context<Self>) {
        let Some(index) = self.selected else { return };
        let Some(image) = self.tiles[index].image.as_ref() else {
            return;
        };
        let id = image.id.clone();
        let ticket = self.next_ticket();
        let tile = &mut self.tiles[index];
        tile.ticket = ticket;
        tile.status = TileStatus::Upscaling;
        tile.operation = TileOperation::Upscale;
        tile.variation_of = None;
        tile.requested_aspect = tile.aspect;
        tile.progress = 0.;
        tile.visual.reset(0.);
        cx.emit(ImageGenerationEvent::Upscale {
            ticket,
            image_id: id,
        });
        cx.notify();
    }
}
impl Render for ImageGeneration {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let r = crate::foundation::surface_radius(cx);
        let measured = self.width.get();
        let side = if measured > px(0.) {
            measured - px(24.)
        } else {
            w.viewport_size().width - px(80.)
        }
        .min(px(616.))
        .max(px(1.));
        let measured_width = self.width.clone();
        let tile_size = (side - px(6.)) / 2.;
        let busy = self
            .tiles
            .iter()
            .any(|t| matches!(t.status, TileStatus::Running | TileStatus::Upscaling));
        let mut root = div()
            .id("image-generation")
            .role(Role::Group)
            .aria_label(tr(cx, "Image generation", "图片生成"))
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                if e.keystroke.key == "escape" && s.selected.is_some() {
                    s.selected = None;
                    cx.emit(ImageGenerationEvent::Select(None));
                    s.tile_focus.focus(w, cx);
                    cx.notify();
                    cx.stop_propagation();
                }
            }))
            .w_full()
            .relative()
            .child(
                canvas(
                    move |bounds, w, _| {
                        if (measured_width.get() - bounds.size.width).abs() > px(0.5) {
                            measured_width.set(bounds.size.width);
                            w.request_animation_frame();
                        }
                    },
                    |_, _, _, _| (),
                )
                .absolute()
                .inset_0(),
            )
            .max_w(px(640.))
            .p_4()
            .rounded(r)
            .border_1()
            .border_color(if self.editor.read(cx).focus.is_focused(w) {
                p.line.blend(p.ink.opacity(0.22))
            } else {
                p.line
            })
            .bg(p.panel)
            .text_color(p.ink)
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .min_h(px(70.))
                    .max_h(px(142.))
                    .w_full()
                    .pt(px(16.))
                    .px(px(18.))
                    .pb(px(6.))
                    .text_size(px(16.))
                    .line_height(px(22.))
                    .child(self.editor.clone()),
            );
        let mut options = div().flex().flex_wrap().items_center().gap_2();
        for (i, name) in self.models.iter().enumerate() {
            options = options.child(
                face(("image-model", i), name.clone(), 32., 8., Tone::Clear, cx)
                .h(px(32.))
                .px(px(10.))
                .rounded(px(8.))
                .text_size(px(14.))
                .line_height(px(20.))
                .text_color(if self.model == i { p.ink } else { p.secondary })
                .bg(if self.model == i { p.panel } else { gpui::transparent_black() })
                .when(self.model == i, |b| b.border_1().border_color(p.line))
                .when(self.model != i, |b| b.hover(|s| s.bg(p.panel.opacity(0.55)).text_color(p.ink)))
                .child(name.clone())
                .on_click(cx.listener(move |s, _, _, cx| {
                    s.model = i;
                    cx.notify();
                })),
            );
        }
        for (i, name) in self.aspects.iter().enumerate() {
            options = options.child(
                face(("image-aspect", i), name.clone(), 32., 8., Tone::Clear, cx)
                .h(px(32.))
                .pl(px(9.))
                .pr(px(10.))
                .rounded(px(8.))
                .text_size(px(14.))
                .line_height(px(20.))
                .text_color(if self.aspect == i { p.ink } else { p.secondary })
                .bg(if self.aspect == i { p.panel } else { gpui::transparent_black() })
                .when(self.aspect == i, |b| b.border_1().border_color(p.line))
                .when(self.aspect != i, |b| {
                    b.hover(|s| s.bg(p.panel.opacity(0.55)).text_color(p.ink))
                })
                .child(name.clone())
                .on_click(cx.listener(move |s, _, _, cx| {
                    s.aspect = i;
                    cx.notify();
                })),
            );
        }
        let label = if busy {
            tr(cx, "Stop", "停止")
        } else {
            tr(cx, "Generate", "生成")
        };
        options = options.child(
            face("image-generate", label, 32., 8., Tone::Primary, cx)
                .px_2()
                .child(label)
                .on_click(
                    cx.listener(move |s, _, _, cx| if busy { s.stop(cx) } else { s.generate(cx) }),
                ),
        );
        root = root.child(options);
        let mut grid = div()
            .id("image-grid")
            .track_focus(&self.tile_focus)
            .tab_stop(true)
            .role(Role::Group)
            .aria_label(tr(cx, "Generated images", "生成的图片"))
            .flex()
            .flex_wrap()
            .gap_2()
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                if e.keystroke.key == "escape" && s.selected.is_some() {
                    s.selected = None;
                    cx.emit(ImageGenerationEvent::Select(None));
                    cx.notify();
                    cx.stop_propagation();
                    return;
                }
                if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                    s.select(s.cursor, cx);
                    cx.stop_propagation();
                    return;
                }
                let current = s.cursor;
                let next = match e.keystroke.key.as_str() {
                    "left" => current.saturating_sub(1),
                    "right" => (current + 1).min(3),
                    "up" => current.saturating_sub(2),
                    "down" => (current + 2).min(3),
                    _ => return,
                };
                s.cursor = next;
                s.tile_focus.focus(w, cx);
                cx.notify();
                cx.stop_propagation();
            }));
        let tile_indices: Vec<usize> = self
            .selected
            .filter(|i| self.tiles[*i].status == TileStatus::Upscaling)
            .map_or_else(|| (0..4).collect(), |i| vec![i]);
        for i in tile_indices {
            let tile = &self.tiles[i];
            let progress = tile.visual.get(w, cx).clamp(0., 1.);
            let selected = self.selected == Some(i);
            let cursor = self.cursor == i && self.tile_focus.is_focused(w);
            let status = match tile.status {
                TileStatus::Empty => tr(cx, "Waiting", "等待生成"),
                TileStatus::Running => tr(cx, "Generating", "生成中"),
                TileStatus::Ready => tr(cx, "Image ready", "图片已生成"),
                TileStatus::Failed => tr(cx, "Failed", "生成失败"),
                TileStatus::Stopped => tr(cx, "Stopped", "已停止"),
                TileStatus::Upscaling => tr(cx, "Upscaling", "放大中"),
            };
            let width = if tile.status == TileStatus::Upscaling {
                side
            } else {
                tile_size
            };
            let ratio = tile.aspect;
            let height = (width / ratio).min(px(210.));
            let width = height * ratio;
            let mut tile_view = div()
                .id(("image-tile", i))
                .role(Role::Button)
                .aria_label(match &tile.image {
                    Some(image) => format!("{} {} · {}", status, i + 1, image.alt),
                    None => format!("{} {}", status, i + 1),
                })
                .aria_toggled(selected.into())
                .tab_index(0)
                .w(width)
                .h(height)
                .relative()
                .overflow_hidden()
                .rounded(px(0.))
                .border(px(if selected || cursor { 2. } else { 1. }))
                .border_color(if selected || cursor {
                    p.bright
                } else {
                    p.ink.opacity(0.07)
                })
                .when(
                    tile.status == TileStatus::Ready && !selected && !cursor,
                    |tile| tile.hover(|s| s.border_color(p.ink.opacity(0.24))),
                )
                .bg(p.tray);
            if let Some(image) = &tile.image {
                tile_view = tile_view.child(
                    img(image.source.clone())
                        .absolute()
                        .size_full()
                        .object_fit(ObjectFit::Cover),
                );
            }
            if matches!(tile.status, TileStatus::Running | TileStatus::Upscaling) {
                if tile.status == TileStatus::Running {
                    let cells = if progress < 0.33 {
                        4
                    } else if progress < 0.66 {
                        8
                    } else {
                        16
                    };
                    tile_view = tile_view.child(
                        canvas(
                            |_, _, _| (),
                            move |bounds, _, w, _| {
                                let cw = bounds.size.width / cells as f32;
                                let ch = bounds.size.height / cells as f32;
                                for y in 0..cells {
                                    for x in 0..cells {
                                        let hash = (x * 37 + y * 53 + i * 17) % 13;
                                        let alpha = 0.025 + hash as f32 * 0.004;
                                        let color = p.tray.blend(p.bright.opacity(alpha));
                                        let origin =
                                            bounds.origin + point(cw * x as f32, ch * y as f32);
                                        w.paint_quad(fill(
                                            Bounds::new(origin, size(cw + px(0.5), ch + px(0.5))),
                                            color,
                                        ));
                                    }
                                }
                            },
                        )
                        .absolute()
                        .inset_0(),
                    );
                }
                tile_view = tile_view
                    .child(div().absolute().inset_0().bg(p.panel.opacity(
                        if tile.status == TileStatus::Upscaling {
                            0.35 * (1. - progress)
                        } else {
                            0.
                        },
                    )))
                    .child(
                        div()
                            .absolute()
                            .bottom(px(8.))
                            .left(px(8.))
                            .flex()
                            .h(px(24.))
                            .items_center()
                            .pl(px(5.))
                            .pr(px(8.))
                            .gap(px(6.))
                            .rounded_full()
                            .bg(black().opacity(0.5))
                            .text_size(px(12.))
                            .text_color(white())
                            .child(format!("{} {:.0}%", status, progress * 100.)),
                    );
            } else if tile.status == TileStatus::Failed {
                let error = tile.error.clone();
                let retry_upscale = tile.operation == TileOperation::Upscale;
                let retry_variation = tile.variation_of.clone();
                let has_original = tile.image.is_some();
                tile_view = tile_view.child(
                    div()
                        .absolute()
                        .inset_0()
                        .flex()
                        .flex_col()
                        .items_center()
                        .justify_center()
                        .gap_2()
                        .child(format!("{} · {}", status, error))
                        .child(
                            face(
                                ("image-retry", i),
                                tr(cx, "Retry", "重试"),
                                32.,
                                8.,
                                Tone::Surface,
                                cx,
                            )
                            .px_2()
                            .child(tr(cx, "Retry", "重试"))
                            .on_click(cx.listener(
                                move |s, _, _, cx| {
                                    if retry_upscale {
                                        s.selected = Some(i);
                                        s.upscale(cx);
                                    } else {
                                        s.start_one(i, retry_variation.clone(), cx);
                                    }
                                    cx.stop_propagation();
                                },
                            )),
                        )
                        .when(has_original, |entry| {
                            entry.child(
                                face(
                                    ("image-restore", i),
                                    tr(cx, "Restore original", "恢复原图"),
                                    32.,
                                    8.,
                                    Tone::Surface,
                                    cx,
                                )
                                .px_2()
                                .child(tr(cx, "Restore original", "恢复原图"))
                                .on_click(cx.listener(
                                    move |s, _, _, cx| {
                                        stop_tile(&mut s.tiles[i]);
                                        s.tiles[i].error = "".into();
                                        cx.notify();
                                        cx.stop_propagation();
                                    },
                                )),
                            )
                        }),
                );
            } else if tile.status != TileStatus::Ready {
                tile_view = tile_view.child(
                    div()
                        .absolute()
                        .inset_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(12.))
                        .text_color(p.secondary)
                        .child(status),
                );
            }
            grid = grid.child(tile_view.on_click(cx.listener(move |s, _, _, cx| s.select(i, cx))));
        }
        root = root.child(grid);
        if let Some(i) = self.selected
            && let Some(image) = self.tiles[i].image.as_ref()
        {
            let id = image.id.clone();
            let vary_id = id.clone();
            let mut actions = div().flex().flex_wrap().gap_2();
            if self.tiles[i].status == TileStatus::Ready {
                actions = actions
                    .child(
                        face(
                            "image-upscale",
                            tr(cx, "Upscale", "放大"),
                            32.,
                            8.,
                            Tone::Surface,
                            cx,
                        )
                        .px_2()
                        .child(tr(cx, "Upscale", "放大"))
                        .on_click(cx.listener(|s, _, _, cx| s.upscale(cx))),
                    )
                    .child(
                        face(
                            "image-vary",
                            tr(cx, "Vary", "生成变体"),
                            32.,
                            8.,
                            Tone::Surface,
                            cx,
                        )
                        .px_2()
                        .child(tr(cx, "Vary", "生成变体"))
                        .on_click(cx.listener(move |s, _, _, cx| {
                            if s.selected.take().is_some() {
                                cx.emit(ImageGenerationEvent::Select(None));
                            }
                            s.start_one(i, Some(vary_id.clone()), cx);
                        })),
                    )
                    .child(
                        face(
                            "image-download",
                            tr(cx, "Download", "下载"),
                            32.,
                            8.,
                            Tone::Surface,
                            cx,
                        )
                        .px_2()
                        .child(tr(cx, "Download", "下载"))
                        .on_click(cx.listener(move |_, _, _, cx| {
                            cx.emit(ImageGenerationEvent::Download(id.clone()))
                        })),
                    );
            }
            root = root.child(actions);
        }
        root
    }
}

#[cfg(test)]
mod tests {
    use super::{
        GeneratedImage, Tile, TileOperation, TileStatus, accepts_update, image_ratio, prepare_tile,
        stop_tile,
    };
    #[test]
    fn stopped_or_replaced_jobs_ignore_late_results() {
        let mut tile = Tile {
            ticket: 4,
            status: TileStatus::Running,
            ..Default::default()
        };
        assert!(accepts_update(&tile, 4));
        tile.status = TileStatus::Stopped;
        assert!(!accepts_update(&tile, 4));
        tile.ticket = 5;
        tile.status = TileStatus::Running;
        assert!(!accepts_update(&tile, 4));
        assert!(accepts_update(&tile, 5));
    }
    #[test]
    fn variation_failure_or_stop_keeps_original_and_image_ratio() {
        let original = GeneratedImage::new("original", "sample.png", "sample", 1600, 900);
        let mut tile = Tile {
            image: Some(original.clone()),
            aspect: image_ratio(&original, 1.),
            status: TileStatus::Ready,
            ..Default::default()
        };
        prepare_tile(&mut tile, 2, Some("original".into()), 0.8);
        assert_eq!(tile.operation, TileOperation::Variation);
        assert_eq!(
            tile.image.as_ref().map(|image| image.id.as_ref()),
            Some("original")
        );
        assert!((tile.aspect - 16. / 9.).abs() < 0.001);
        tile.status = TileStatus::Failed;
        assert_eq!(
            tile.image.as_ref().map(|image| image.id.as_ref()),
            Some("original")
        );
        prepare_tile(&mut tile, 3, Some("original".into()), 0.8);
        stop_tile(&mut tile);
        assert_eq!(tile.status, TileStatus::Ready);
        assert!(!accepts_update(&tile, 3));
        assert_eq!(
            tile.image.as_ref().map(|image| image.id.as_ref()),
            Some("original")
        );
        assert!(
            (image_ratio(
                &GeneratedImage::new("portrait", "sample.png", "sample", 800, 1000),
                1.
            ) - 0.8)
                .abs()
                < 0.001
        );
    }
}
