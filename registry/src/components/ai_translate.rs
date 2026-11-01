//! Request a translation and compare it with the original message.
use crate::primitives::family::Paint;
use crate::primitives::ops_surface::*;
use gpui::{prelude::*, *};
pub struct TranslationRequest {
    pub ticket: u64,
    pub language: String,
}
pub struct MessageTranslation {
    id: SharedString,
    pub author: String,
    pub original: String,
    pub detected: String,
    pub languages: Vec<String>,
    pub credit: String,
    ticket: u64,
    target: String,
    picker: bool,
    cache: std::collections::HashMap<String, (String, String)>,
    translation: String,
    source_visible: bool,
    loading: bool,
    error: bool,
}
impl EventEmitter<TranslationRequest> for MessageTranslation {}
impl MessageTranslation {
    pub fn new(
        id: impl Into<SharedString>,
        author: impl Into<String>,
        original: impl Into<String>,
        languages: Vec<String>,
    ) -> Self {
        Self {
            id: id.into(),
            author: author.into(),
            original: original.into(),
            detected: String::new(),
            languages,
            credit: String::new(),
            ticket: 0,
            target: String::new(),
            picker: false,
            cache: Default::default(),
            translation: String::new(),
            source_visible: true,
            loading: false,
            error: false,
        }
    }
    fn start(&mut self, l: String, cx: &mut Context<Self>) {
        self.ticket += 1;
        self.picker = false;
        if let Some((source, text)) = self.cache.get(&l)
            && source == &self.original
        {
            self.target = l;
            self.translation = text.clone();
            self.source_visible = false;
            self.loading = false;
            self.error = false;
            cx.notify();
            return;
        }
        self.target = l.clone();
        self.source_visible = false;
        self.translation.clear();
        self.loading = true;
        self.error = false;
        cx.emit(TranslationRequest {
            ticket: self.ticket,
            language: l,
        });
        cx.notify()
    }
    pub fn update_translation(
        &mut self,
        ticket: u64,
        text: Result<String, ()>,
        finished: bool,
        cx: &mut Context<Self>,
    ) {
        if ticket != self.ticket || !self.loading {
            return;
        }
        match text {
            Ok(t) => {
                self.translation = t;
                if finished {
                    self.cache.insert(
                        self.target.clone(),
                        (self.original.clone(), self.translation.clone()),
                    );
                }
            }
            Err(()) => self.error = true,
        }
        self.loading = !finished && !self.error;
        cx.notify()
    }
}
impl Render for MessageTranslation {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let original = self.source_visible || self.translation.is_empty();
        let p = Paint::get(cx);
        let mut shown = if original {
            self.original.clone()
        } else {
            self.translation.clone()
        };
        let end = shown.len();
        let mut styles = stream_highlights(
            "translation-words",
            if original { "" } else { &self.translation },
            w,
            cx,
        );
        if self.loading && !original {
            let n = self
                .translation
                .chars()
                .filter(|c| matches!(c, '.' | '!' | '?' | '。' | '！' | '？'))
                .count()
                .max(1);
            let rest = self
                .original
                .split_inclusive(['.', '!', '?', '。', '！', '？'])
                .skip(n)
                .collect::<String>();
            if !rest.trim().is_empty() {
                shown.push(' ');
                shown.push_str(rest.trim());
                styles.push((
                    end..shown.len(),
                    HighlightStyle {
                        color: Some(p.ink.opacity(0.45)),
                        ..Default::default()
                    },
                ))
            }
        }
        panel(&self.id, 522., cx)
            .child(
                row()
                    .child(avatar(&self.author, 0, cx))
                    .child(title(self.author.clone()))
                    .child(badge(self.detected.clone(), cx)),
            )
            .child(
                div()
                    .min_h(px(85.))
                    .text_size(px(15.))
                    .line_height(px(23.))
                    .text_color(if self.loading && original {
                        p.faint
                    } else {
                        p.ink
                    })
                    .child(StyledText::new(shown).with_highlights(styles)),
            )
            .child(divider(cx))
            .child(
                row().child(
                    btn(
                        "languages",
                        if self.target.is_empty() {
                            tr(cx, "Translate", "翻译").to_string()
                        } else {
                            self.target.clone()
                        },
                        false,
                        cx,
                        |s, _, cx| {
                            s.picker = !s.picker;
                            cx.notify()
                        },
                    )
                    .child(crate::primitives::glyph::icon(
                        if self.picker {
                            crate::primitives::glyph::Glyph::Up
                        } else {
                            crate::primitives::glyph::Glyph::Down
                        },
                        12.,
                        p.secondary,
                    ))
                    .aria_expanded(self.picker),
                ),
            )
            .when(self.picker, |r| {
                r.child(
                    row()
                        .flex_wrap()
                        .children(self.languages.iter().enumerate().map(|(i, l)| {
                            let l = l.clone();
                            btn(
                                ("language", i),
                                l.clone(),
                                self.target == l,
                                cx,
                                move |s, _, cx| s.start(l.clone(), cx),
                            )
                        })),
                )
            })
            .child(
                row()
                    .flex_wrap()
                    .when(!self.translation.is_empty(), |r| {
                        r.child(btn(
                            "original",
                            if self.source_visible {
                                tr(cx, "Show translation", "查看译文")
                            } else {
                                tr(cx, "Show original", "查看原文")
                            },
                            false,
                            cx,
                            |s, _, cx| {
                                s.source_visible = !s.source_visible;
                                cx.notify()
                            },
                        ))
                    })
                    .when(self.error, |r| {
                        r.child(btn(
                            "retry",
                            tr(cx, "Translation failed · Retry", "翻译失败 · 重试"),
                            false,
                            cx,
                            |s, _, cx| s.start(s.target.clone(), cx),
                        ))
                    })
                    .child(note(
                        if self.loading {
                            tr(cx, "Translating…", "翻译中…").to_string()
                        } else {
                            self.credit.clone()
                        },
                        cx,
                    )),
            )
    }
}
