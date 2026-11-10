//! Host-resolved link previews for a text composer; this component performs no network requests.
use crate::{
    components::textarea::Textarea,
    primitives::{
        family::{Paint, Slide, Tone, face},
        field::FieldEvent,
        glyph::{Glyph, icon},
        presentation::{column, hint, row, tr},
    },
};
use gpui::{prelude::*, *};
use std::{sync::Arc, time::Duration};

/// A shortcut that replaces the composer draft with a supplied example.
pub struct LinkSample {
    pub label: SharedString,
    pub draft: SharedString,
    pub icon: ImageSource,
}

impl LinkSample {
    pub fn new(
        label: impl Into<SharedString>,
        draft: impl Into<SharedString>,
        icon: impl Into<ImageSource>,
    ) -> Self {
        Self {
            label: label.into(),
            draft: draft.into(),
            icon: icon.into(),
        }
    }
}
#[derive(Clone)]
pub struct LinkPreview {
    pub site: SharedString,
    pub title: SharedString,
    pub description: SharedString,
    pub image: Option<Arc<RenderImage>>,
}
impl LinkPreview {
    pub fn new(
        site: impl Into<SharedString>,
        title: impl Into<SharedString>,
        description: impl Into<SharedString>,
    ) -> Self {
        Self {
            site: site.into(),
            title: title.into(),
            description: description.into(),
            image: None,
        }
    }
    /// Images must already be decoded by the host; no remote image URL is loaded here.
    pub fn with_image(mut self, image: Arc<RenderImage>) -> Self {
        self.image = Some(image);
        self
    }
}
#[derive(Clone)]
pub enum LinkUnfurlEvent {
    Resolve {
        id: u64,
        url: SharedString,
    },
    Cancel {
        id: u64,
    },
    Send {
        id: u64,
        text: SharedString,
        preview: Option<LinkPreview>,
    },
    Open {
        url: SharedString,
    },
    Changed {
        text: SharedString,
    },
}
pub struct LinkUnfurl {
    editor: Entity<Textarea>,
    text: SharedString,
    url: Option<SharedString>,
    preview: Option<LinkPreview>,
    error: Option<SharedString>,
    resolve_id: Option<u64>,
    send_id: Option<u64>,
    draft_revision: u64,
    sent_revision: Option<u64>,
    serial: u64,
    revision: u64,
    open: bool,
    visible: bool,
    presence: Slide,
    simulated: bool,
    samples: Vec<LinkSample>,
    auto_open: bool,
}
impl EventEmitter<LinkUnfurlEvent> for LinkUnfurl {}
fn first_url(text: &str) -> Option<SharedString> {
    text.split_whitespace()
        .map(|s| {
            s.trim_matches(|c: char| {
                matches!(
                    c,
                    '(' | ')' | '[' | ']' | '<' | '>' | ',' | '.' | ';' | '!' | '\"' | '\''
                )
            })
        })
        .find(|s| {
            s.strip_prefix("https://")
                .or_else(|| s.strip_prefix("http://"))
                .is_some_and(|rest| {
                    !rest.is_empty() && !rest.starts_with('/') && !rest.starts_with('?')
                })
        })
        .map(Into::into)
}
impl LinkUnfurl {
    pub fn new(label: impl Into<SharedString>, w: &mut Window, cx: &mut Context<Self>) -> Self {
        let label = label.into();
        let editor = cx.new(|cx| Textarea::new("link-composer", label, w, cx));
        cx.subscribe(&editor, |s, _, event: &FieldEvent, cx| {
            if let FieldEvent::Changed(text) = event {
                s.changed(text.clone(), cx);
            }
        })
        .detach();
        Self {
            editor,
            text: "".into(),
            url: None,
            preview: None,
            error: None,
            resolve_id: None,
            send_id: None,
            draft_revision: 0,
            sent_revision: None,
            serial: 0,
            revision: 0,
            open: false,
            visible: true,
            presence: Slide::new(0.),
            simulated: false,
            samples: Vec::new(),
            auto_open: true,
        }
    }
    pub fn with_simulated(mut self, value: bool) -> Self {
        self.simulated = value;
        self
    }
    /// Show local example drafts beside the send control.
    pub fn with_samples(mut self, samples: Vec<LinkSample>) -> Self {
        self.samples = samples;
        self
    }
    /// Wait for activation of the resolved link before opening its card.
    pub fn with_collapsed_previews(mut self) -> Self {
        self.auto_open = false;
        self
    }
    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        if self.visible != visible {
            self.visible = visible;
            cx.notify();
        }
    }
    pub fn set_text(&mut self, text: impl Into<String>, w: &Window, cx: &mut Context<Self>) {
        self.editor
            .update(cx, |editor, cx| editor.set_value(text, w, cx));
    }
    fn changed(&mut self, text: SharedString, cx: &mut Context<Self>) {
        if self.text == text {
            return;
        }
        self.text = text.clone();
        self.draft_revision = self.draft_revision.wrapping_add(1);
        let url = first_url(&text);
        if self.url != url {
            if let Some(id) = self.resolve_id.take() {
                cx.emit(LinkUnfurlEvent::Cancel { id });
            }
            self.revision = self.revision.wrapping_add(1);
            self.url = url;
            self.preview = None;
            self.error = None;
            self.open = false;
            self.presence.reset(0.);
            if self.url.is_some() {
                let revision = self.revision;
                cx.spawn(async move |view, cx| {
                    cx.background_executor()
                        .timer(Duration::from_millis(350))
                        .await;
                    let _ = view.update(cx, |view, cx| {
                        if view.revision == revision {
                            view.request_preview(cx);
                        }
                    });
                })
                .detach();
            }
        }
        cx.emit(LinkUnfurlEvent::Changed { text });
        cx.notify();
    }
    pub fn request_preview(&mut self, cx: &mut Context<Self>) {
        if self.resolve_id.is_some() {
            return;
        }
        let Some(url) = self.url.clone() else {
            return;
        };
        self.serial = self.serial.wrapping_add(1);
        self.resolve_id = Some(self.serial);
        self.error = None;
        cx.emit(LinkUnfurlEvent::Resolve {
            id: self.serial,
            url,
        });
        cx.notify();
    }
    pub fn resolve_preview(
        &mut self,
        id: u64,
        result: Result<Option<LinkPreview>, SharedString>,
        cx: &mut Context<Self>,
    ) {
        if self.resolve_id != Some(id) {
            return;
        }
        self.resolve_id = None;
        match result {
            Ok(Some(preview)) => {
                self.preview = Some(preview);
                self.open = self.auto_open;
                if self.open {
                    self.presence.duration(1., 0.48, cx);
                }
                self.error = None;
            }
            Ok(None) => {
                self.preview = None;
                self.open = false;
                self.presence.reset(0.);
            }
            Err(error) => {
                self.error = Some(error);
                self.open = false;
                self.presence.reset(0.);
            }
        }
        cx.notify();
    }
    pub fn send(&mut self, cx: &mut Context<Self>) {
        if self.send_id.is_some() || self.text.trim().is_empty() {
            return;
        }
        self.serial = self.serial.wrapping_add(1);
        self.send_id = Some(self.serial);
        self.sent_revision = Some(self.draft_revision);
        cx.emit(LinkUnfurlEvent::Send {
            id: self.serial,
            text: self.text.clone(),
            preview: self.preview.clone(),
        });
        cx.notify();
    }
    pub fn resolve_send(
        &mut self,
        id: u64,
        result: Result<(), SharedString>,
        w: &Window,
        cx: &mut Context<Self>,
    ) {
        if self.send_id != Some(id) {
            return;
        }
        self.send_id = None;
        let sent_revision = self.sent_revision.take();
        match result {
            Ok(()) => {
                self.error = None;
                if sent_revision == Some(self.draft_revision) {
                    self.changed("".into(), cx);
                    self.set_text("", w, cx);
                }
            }
            Err(error) => self.error = Some(error),
        }
        cx.notify();
    }
    fn toggle(&mut self, cx: &mut Context<Self>) {
        if self.preview.is_some() {
            self.open = !self.open;
            self.presence.duration(
                f32::from(self.open),
                if self.open { 0.48 } else { 0.18 },
                cx,
            );
            cx.notify();
        }
    }
    fn remove(&mut self, w: &Window, cx: &mut Context<Self>) {
        if let Some(url) = self.url.clone() {
            let text = self.text.replacen(url.as_ref(), "", 1);
            self.set_text(text, w, cx);
        }
    }
}
impl Render for LinkUnfurl {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let progress = if self.visible && w.is_visible() {
            self.presence.get(w, cx)
        } else {
            self.presence.value(cx)
        }
        .clamp(0., 1.);
        let mut panel = column()
            .id("link-unfurl")
            .role(Role::Group)
            .aria_label(tr(cx, "Message with link preview", "带链接预览的消息"))
            .w_full()
            .max_w(px(576.))
            .gap(px(8.))
            .pt(px(16.))
            .px(px(18.))
            .pb(px(12.))
            .rounded(crate::foundation::surface_radius(cx))
            .bg(p.panel)
            .border_1()
            .border_color(p.line)
            .text_color(p.ink)
            .child(self.editor.clone());
        if let Some(url) = self.url.clone() {
            let label = self
                .preview
                .as_ref()
                .map_or_else(|| url.clone(), |preview| preview.title.clone());
            let mut controls = row().gap_2().flex_wrap().items_center();
            if self.resolve_id.is_some() {
                controls = controls.child(hint(tr(cx, "Loading preview…", "正在加载预览…"), p));
            } else if self.preview.is_some() {
                controls = controls.child(
                    face(
                        "link-toggle",
                        label.clone(),
                        32.,
                        crate::foundation::surface_radius(cx).into(),
                        Tone::Surface,
                        cx,
                    )
                    .h(px(26.))
                    .pl(px(5.))
                    .pr(px(10.))
                    .gap(px(6.))
                    .rounded_full()
                    .bg(if self.open {
                        p.bright.opacity(0.11)
                    } else {
                        p.tray
                    })
                    .border_1()
                    .border_color(if self.open {
                        p.bright.opacity(0.34)
                    } else {
                        p.line
                    })
                    .when(!self.open, |b| {
                        b.hover(|s| s.border_color(p.faint).text_color(p.ink))
                    })
                    .aria_toggled(self.open.into())
                    .child(label)
                    .on_click(cx.listener(|s, _, _, cx| s.toggle(cx)))
                    .on_key_down(cx.listener(|s, e: &KeyDownEvent, _, cx| {
                        if matches!(e.keystroke.key.as_str(), "enter" | "space") && !e.is_held {
                            s.toggle(cx);
                            cx.stop_propagation();
                        }
                    })),
                );
            } else {
                controls = controls.child(hint(url.clone(), p)).child(
                    face(
                        "link-retry",
                        tr(cx, "Try preview", "重试预览"),
                        32.,
                        crate::foundation::surface_radius(cx).into(),
                        Tone::Surface,
                        cx,
                    )
                    .px(px(8.))
                    .child(tr(cx, "Try preview", "重试预览"))
                    .on_click(cx.listener(|s, _, _, cx| s.request_preview(cx)))
                    .on_key_down(cx.listener(|s, e: &KeyDownEvent, _, cx| {
                        if matches!(e.keystroke.key.as_str(), "enter" | "space") && !e.is_held {
                            s.request_preview(cx);
                            cx.stop_propagation();
                        }
                    })),
                );
            }
            controls = controls.child(
                face(
                    "link-remove",
                    tr(cx, "Remove link", "移除链接"),
                    30.,
                    15.,
                    Tone::Clear,
                    cx,
                )
                .size(px(30.))
                .rounded_full()
                .p_0()
                .text_color(p.faint)
                .hover(|s| s.bg(p.tray).text_color(p.ink))
                .child("×")
                .on_click(cx.listener(|s, _, w, cx| s.remove(w, cx)))
                .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                    if matches!(e.keystroke.key.as_str(), "enter" | "space") && !e.is_held {
                        s.remove(w, cx);
                        cx.stop_propagation();
                    }
                })),
            );
            panel = panel.child(controls);
            if let Some(preview) = self.preview.clone().filter(|_| self.open || progress > 0.) {
                let height = if preview.image.is_some() { 372. } else { 148. };
                let mut card = column()
                    .w_full()
                    .max_w(px(440.))
                    .p(px(8.))
                    .rounded(crate::foundation::surface_radius(cx))
                    .bg(p.panel)
                    .border_1()
                    .border_color(p.line)
                    .shadow(vec![
                        gpui::BoxShadow {
                            color: p.ink.opacity(0.18),
                            offset: gpui::point(px(0.), px(18.)),
                            blur_radius: px(40.),
                            spread_radius: px(0.),
                            inset: false,
                        },
                        gpui::BoxShadow {
                            color: p.ink.opacity(0.12),
                            offset: gpui::point(px(0.), px(4.)),
                            blur_radius: px(12.),
                            spread_radius: px(0.),
                            inset: false,
                        },
                    ])
                    .child(
                        row()
                            .h(px(30.))
                            .items_center()
                            .gap(px(8.))
                            .pl(px(7.))
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .line_height(px(20.))
                                    .text_color(p.secondary)
                                    .child(preview.site),
                            ),
                    )
                    .child(
                        div()
                            .pt(px(2.))
                            .px(px(6.))
                            .text_size(px(16.))
                            .line_height(px(22.))
                            .font_weight(FontWeight::MEDIUM)
                            .hover(|s| s.border_b_1().border_color(p.bright.opacity(0.34)))
                            .child(preview.title),
                    )
                    .child(
                        div()
                            .px(px(6.))
                            .text_size(px(14.))
                            .line_height(px(20.))
                            .text_color(p.secondary)
                            .child(preview.description),
                    );
                if let Some(image) = preview.image {
                    card = card.child(
                        img(ImageSource::Render(image))
                            .w_full()
                            .h(px(228.))
                            .mt(px(8.))
                            .mx(px(-6.))
                            .object_fit(ObjectFit::Cover)
                            .rounded(px(0.)),
                    );
                }
                let target = url.clone();
                card = card.child(
                    face(
                        "link-open",
                        tr(cx, "Open link", "打开链接"),
                        32.,
                        crate::foundation::surface_radius(cx).into(),
                        Tone::Surface,
                        cx,
                    )
                    .px(px(8.))
                    .child(tr(cx, "Open link", "打开链接"))
                    .on_click(cx.listener(move |_, _, _, cx| {
                        cx.emit(LinkUnfurlEvent::Open {
                            url: target.clone(),
                        })
                    }))
                    .on_key_down(cx.listener(
                        move |s, e: &KeyDownEvent, _, cx| {
                            if matches!(e.keystroke.key.as_str(), "enter" | "space") && !e.is_held {
                                if let Some(url) = s.url.clone() {
                                    cx.emit(LinkUnfurlEvent::Open { url });
                                }
                                cx.stop_propagation();
                            }
                        },
                    )),
                );
                panel = panel.child(
                    div()
                        .w_full()
                        .h(px(height * progress))
                        .overflow_hidden()
                        .opacity(progress)
                        .child(card),
                );
            }
        }
        if let Some(error) = self.error.clone() {
            panel = panel.child(div().px(px(18.)).text_color(p.danger).child(error));
        }
        let sending = self.send_id.is_some();
        let send = face(
            "link-send",
            if sending {
                tr(cx, "Sending…", "发送中…")
            } else {
                tr(cx, "Send", "发送")
            },
            36.,
            999.,
            Tone::Primary,
            cx,
        )
        .size(px(36.))
        .rounded_full()
        .p_0()
        .flex_none()
        .when(sending, |v| v.opacity(0.6))
        .child(icon(Glyph::ArrowUp, 16., p.inverse))
        .on_click(cx.listener(|s, _, _, cx| s.send(cx)))
        .on_key_down(cx.listener(|s, e: &KeyDownEvent, _, cx| {
            if matches!(e.keystroke.key.as_str(), "enter" | "space") && !e.is_held {
                s.send(cx);
                cx.stop_propagation();
            }
        }));
        let examples = row()
            .min_w_0()
            .gap(px(4.))
            .flex_wrap()
            .when(
                !self.samples.is_empty() && w.viewport_size().width >= px(360.),
                |r| r.child(hint(tr(cx, "Try", "试试"), p)),
            )
            .children(self.samples.iter().enumerate().map(|(i, sample)| {
                let draft = sample.draft.clone();
                let keyboard_draft = draft.clone();
                face(
                    ("link-example", i),
                    sample.label.clone(),
                    28.,
                    999.,
                    Tone::Clear,
                    cx,
                )
                .px(px(5.))
                .gap(px(4.))
                .text_size(px(12.))
                .child(img(sample.icon.clone()).size(px(13.)).flex_none())
                .child(sample.label.clone())
                .on_click(cx.listener(move |s, _, w, cx| s.set_text(draft.as_ref(), w, cx)))
                .on_key_down(cx.listener(move |s, e: &KeyDownEvent, w, cx| {
                    if matches!(e.keystroke.key.as_str(), "enter" | "space") && !e.is_held {
                        s.set_text(keyboard_draft.as_ref(), w, cx);
                        cx.stop_propagation();
                    }
                }))
            }));
        panel = panel.child(
            row()
                .w_full()
                .justify_between()
                .gap(px(6.))
                .child(examples)
                .child(send),
        );
        panel.when(self.simulated, |v| {
            v.child(hint(
                tr(cx, "Simulated previews and sending", "模拟预览和发送"),
                p,
            ))
        })
    }
}
#[cfg(test)]
mod tests {
    use super::first_url;
    #[test]
    fn finds_first_http_link() {
        assert_eq!(
            first_url("see (https://example.test/path), thanks").as_deref(),
            Some("https://example.test/path")
        );
        assert!(first_url("plain text").is_none());
        assert!(first_url("javascript:alert(1)").is_none());
    }
}
