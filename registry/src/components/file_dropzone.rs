//! Local attachments with per-attempt tickets, bounded reads and host-controlled upload progress.
pub use crate::primitives::form_files::PickedFile;
use crate::primitives::{
    family::{Paint, Slide},
    form_chrome::*,
    form_files::{self, FileResult},
    glyph::{Glyph, icon},
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
#[derive(Clone, Debug)]
pub enum UploadState {
    Uploading(f32),
    Uploaded,
    Failed(String),
}
#[derive(Clone, Debug)]
pub enum FileEvent {
    Upload { ticket: u64, file: PickedFile },
    Removed(u64),
}
pub struct FileRow {
    pub file: PickedFile,
    pub state: UploadState,
    ticket: u64,
    progress: Slide,
    reveal: Slide,
}
pub struct FileDropzone {
    label: SharedString,
    max_bytes: u64,
    max_count: usize,
    description: SharedString,
    sequence: u64,
    accept: Vec<String>,
    rows: Vec<FileRow>,
    departing: Vec<(usize, FileRow)>,
    hover: Slide,
    error: SharedString,
    bounds: Rc<Cell<Bounds<Pixels>>>,
    _drop: Option<Subscription>,
}
impl EventEmitter<FileEvent> for FileDropzone {}
impl FileDropzone {
    pub fn new(
        label: impl Into<SharedString>,
        max_bytes: u64,
        max_count: usize,
        cx: &mut Context<Self>,
    ) -> Self {
        let bounds: Rc<Cell<Bounds<Pixels>>> = Rc::default();
        let drop = form_files::drop_subscription(
            bounds.clone(),
            max_bytes,
            cx,
            Self::received,
            Some(Self::drag_over),
        );
        Self {
            label: label.into(),
            max_bytes,
            max_count,
            description: "".into(),
            sequence: 0,
            accept: vec![],
            rows: vec![],
            departing: vec![],
            hover: Slide::snappy(0.),
            error: "".into(),
            bounds,
            _drop: drop,
        }
    }
    pub fn with_description(mut self, desc: impl Into<SharedString>) -> Self {
        self.description = desc.into();
        self
    }
    pub fn with_extensions(mut self, ext: &[&str]) -> Self {
        self.accept = ext
            .iter()
            .map(|v| v.trim_start_matches('.').to_ascii_lowercase())
            .collect();
        self
    }
    pub fn seed(&mut self, file: PickedFile, state: UploadState) {
        self.sequence += 1;
        self.rows.push(FileRow {
            file,
            state,
            ticket: self.sequence,
            progress: Slide::snappy(1.),
            reveal: Slide::snappy(1.),
        })
    }
    /// Enqueue bytes received from a host-specific drop or paste surface.
    pub fn enqueue(&mut self, files: Vec<PickedFile>, cx: &mut Context<Self>) {
        self.received(Ok(files), cx)
    }
    pub fn rows(&self) -> &[FileRow] {
        &self.rows
    }
    pub fn report(&mut self, id: u64, state: UploadState, cx: &mut Context<Self>) {
        if let Some(r) = self
            .rows
            .iter_mut()
            .find(|r| r.ticket == id && matches!(r.state, UploadState::Uploading(_)))
        {
            let v = match &state {
                UploadState::Uploading(v) => v.clamp(0., 1.),
                UploadState::Uploaded => 1.,
                UploadState::Failed(_) => 0.,
            };
            r.progress.duration(v, 0.2, cx);
            r.state = state;
            cx.notify()
        }
    }
    fn drag_over(&mut self, on: bool, cx: &mut Context<Self>) {
        self.hover.duration(on as u8 as f32, 0.26, cx);
        cx.notify()
    }
    fn received(&mut self, files: FileResult, cx: &mut Context<Self>) {
        self.error = "".into();
        match files {
            Err(e) => self.error = e.into(),
            Ok(files) => {
                for f in files {
                    if self.rows.len() >= self.max_count {
                        self.error = tr(cx, "File limit reached", "文件数量已达上限").into();
                        break;
                    }
                    if f.size > self.max_bytes {
                        self.error =
                            format!("{}: {}", f.name, tr(cx, "file too large", "文件过大")).into();
                        continue;
                    }
                    let ext = f.name.rsplit('.').next().unwrap_or("").to_lowercase();
                    if !self.accept.is_empty() && !self.accept.contains(&ext) {
                        self.error = tr(cx, "File type is not allowed", "不支持此文件类型").into();
                        continue;
                    }
                    self.sequence += 1;
                    self.rows.push(FileRow {
                        file: f.clone(),
                        state: UploadState::Uploading(0.),
                        ticket: self.sequence,
                        progress: Slide::snappy(0.),
                        reveal: Slide::snappy(0.),
                    });
                    cx.emit(FileEvent::Upload {
                        ticket: self.sequence,
                        file: f,
                    })
                }
            }
        }
        cx.notify()
    }
    fn choose(&mut self, cx: &mut Context<Self>) {
        let accept = self
            .accept
            .iter()
            .map(|e| format!(".{e}"))
            .collect::<Vec<_>>()
            .join(",");
        let task = form_files::choose(true, &accept, self.max_bytes, cx);
        cx.spawn(async move |this, cx| {
            let fs = task.await;
            let _ = this.update(cx, |s, cx| s.received(fs, cx));
        })
        .detach()
    }
}
impl Render for FileDropzone {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let hover = self.hover.get(w, cx).clamp(0., 1.);
        let rect = self.bounds.clone();
        let mut stack = div().relative().w(px(56.)).h(px(44.));
        for i in 0..3 {
            stack = stack.child(
                div()
                    .absolute()
                    .left(px(i as f32 * 7.))
                    .top(px((if i == 1 { 0. } else { 4. }) - 6. * hover))
                    .w(px(30.))
                    .h(px(35.))
                    .rounded(crate::foundation::surface_radius(cx))
                    .border_1()
                    .border_color(p.line)
                    .bg(p.panel)
                    .shadow_sm()
                    .flex()
                    .items_center()
                    .justify_center()
                    .when(i == 2, |d| d.child(icon(Glyph::Up, 16., p.secondary))),
            )
        }
        let target = div()
            .on_children_prepainted(move |bs, _, _| {
                if let Some(b) = bs.first() {
                    rect.set(*b)
                }
            })
            .id("dropzone")
            .role(Role::Button)
            .tab_index(0)
            .aria_label(self.label.clone())
            .rounded(crate::foundation::surface_radius(cx))
            .border_1()
            .border_dashed()
            .border_color(p.faint.blend(p.bright.opacity(hover)))
            .bg(p.panel.blend(p.hover.opacity(hover)))
            .on_drag_move::<ExternalPaths>(cx.listener(
                |s, e: &DragMoveEvent<ExternalPaths>, _, cx| {
                    s.drag_over(e.bounds.contains(&e.event.position), cx)
                },
            ))
            .cursor_pointer()
            .hover(move |s| s.bg(p.hover))
            .child(
                col()
                    .min_h(px(165.))
                    .items_center()
                    .justify_center()
                    .child(stack)
                    .child(self.label.clone())
                    .child(hint(
                        tr(
                            cx,
                            "Drop, paste, or choose files from your device",
                            "拖放、粘贴或点击选择设备上的文件",
                        ),
                        cx,
                    ))
                    .when(!self.description.is_empty(), |d| {
                        d.child(hint(self.description.clone(), cx))
                    }),
            )
            .on_click(cx.listener(|s, _, _, cx| s.choose(cx)));
        let mut result = section(538., cx).child(form_files::drop_target(
            target,
            cx,
            self.max_bytes,
            Self::received,
        ));
        self.departing.retain(|(_, r)| r.reveal.value(cx) > 0.001);
        for row in &mut self.rows {
            row.reveal.duration(1., 0.4, cx);
        }
        let mut visual: Vec<_> = self.rows.iter().collect();
        for (i, r) in &self.departing {
            visual.insert((*i).min(visual.len()), r);
        }
        for r in visual {
            let ticket = r.ticket;
            let t = r.reveal.get(w, cx).clamp(0., 1.);
            let status = match &r.state {
                UploadState::Uploaded => tr(cx, "Uploaded", "已上传").to_owned(),
                UploadState::Failed(e) => format!("{}: {}", tr(cx, "Failed", "失败"), e),
                UploadState::Uploading(v) => format!("{}%", (v * 100.) as usize),
            };
            let failed = matches!(r.state, UploadState::Failed(_));
            let item = row()
                .px(px(8.))
                .py(px(6.))
                .min_h(px(52.))
                .rounded(crate::foundation::surface_radius(cx))
                .border_1()
                .bg(p.panel)
                .border_color(if failed {
                    p.danger.opacity(0.44)
                } else {
                    p.line
                })
                .child(
                    div()
                        .size(px(34.))
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(icon(
                            if r.file.name.ends_with(".mov") {
                                Glyph::Play
                            } else {
                                Glyph::FileText
                            },
                            18.,
                            p.secondary,
                        )),
                )
                .child(
                    col()
                        .gap_0()
                        .flex_1()
                        .min_w_0()
                        .child(r.file.name.clone())
                        .child(
                            hint(
                                format!("{:.1} MB · {}", r.file.size as f64 / 1_000_000., status),
                                cx,
                            )
                            .when(failed, |d| d.text_color(p.danger)),
                        ),
                )
                .when(matches!(r.state, UploadState::Failed(_)), |d| {
                    d.child(
                        button(("retry", ticket as usize), tr(cx, "Retry", "重试"), cx)
                            .border_1()
                            .border_color(p.line)
                            .on_click(cx.listener(move |s, _, _, cx| {
                                if let Some(r) = s.rows.iter_mut().find(|r| r.ticket == ticket)
                                    && matches!(r.state, UploadState::Failed(_))
                                {
                                    s.sequence += 1;
                                    r.ticket = s.sequence;
                                    r.progress.reset(0.);
                                    r.state = UploadState::Uploading(0.);
                                    cx.emit(FileEvent::Upload {
                                        ticket: r.ticket,
                                        file: r.file.clone(),
                                    });
                                    cx.notify()
                                }
                            })),
                    )
                })
                .child(
                    glyph(
                        ("remove", ticket as usize),
                        tr(cx, "Remove file", "移除文件"),
                        Glyph::Close,
                        cx,
                    )
                    .on_click(cx.listener(move |s, _, _, cx| {
                        if let Some(i) = s.rows.iter().position(|r| r.ticket == ticket) {
                            let mut r = s.rows.remove(i);
                            r.reveal.duration(0., 0.40, cx);
                            cx.emit(FileEvent::Removed(r.ticket));
                            s.departing.push((i, r));
                            cx.notify()
                        }
                    })),
                );
            result = result.child(
                div()
                    .h(px(52. * t))
                    .overflow_hidden()
                    .child(item.relative().top(px(-22. * (1. - t))).opacity(t)),
            );
            if matches!(r.state, UploadState::Uploading(_)) {
                result = result.child(
                    div().h(px(3.)).w_full().rounded_full().bg(p.line).child(
                        div()
                            .h_full()
                            .w(relative(r.progress.get(w, cx).clamp(0., 1.)))
                            .bg(p.bright),
                    ),
                )
            }
        }
        result.when(!self.error.is_empty(), |d| {
            d.child(hint(self.error.clone(), cx).text_color(p.danger))
        })
    }
}
