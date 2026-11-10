//! Stream a message translation in place and switch back to its original text.
use crate::primitives::{
    brand_mark::brand_mark,
    control::activate,
    family::{Paint, PhaseClock, Slide, Tone, face},
    glyph::{Glyph, icon},
    ops_surface::*,
};
use gpui::{prelude::*, *};
use std::{cell::Cell, collections::HashMap, rc::Rc};

pub struct TranslationRequest {
    pub ticket: u64,
    pub language: String,
}

#[derive(Clone, Default)]
struct Reading {
    language: String,
    text: String,
    original: bool,
}

pub struct MessageTranslation {
    id: SharedString,
    pub author: String,
    pub original: String,
    pub detected: String,
    pub languages: Vec<String>,
    pub credit: String,
    byline: SharedString,
    portrait: Option<std::sync::Arc<Image>>,
    ticket: u64,
    reading: Reading,
    before_request: Reading,
    picker: bool,
    active_language: usize,
    cache: HashMap<String, (String, String)>,
    request: Option<String>,
    failed: Option<String>,
    trigger: Option<FocusHandle>,
    menu_focus: Option<FocusHandle>,
    anchor: Rc<Cell<Bounds<Pixels>>>,
    prior_paragraph: Option<String>,
    swap: PhaseClock,
    measured_message: Rc<Cell<Size<Pixels>>>,
    message_height: Slide,
    last_width: Pixels,
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
            byline: "".into(),
            portrait: None,
            ticket: 0,
            reading: Reading {
                original: true,
                ..Default::default()
            },
            before_request: Reading::default(),
            picker: false,
            active_language: 0,
            cache: HashMap::new(),
            request: None,
            failed: None,
            trigger: None,
            menu_focus: None,
            anchor: Rc::new(Cell::new(Bounds::default())),
            prior_paragraph: None,
            swap: Default::default(),
            measured_message: Rc::new(Cell::new(Size::default())),
            message_height: Slide::new(0.),
            last_width: px(0.),
        }
    }

    /// Adds author metadata under the name.
    pub fn byline(mut self, byline: impl Into<SharedString>) -> Self {
        self.byline = byline.into();
        self
    }

    /// Adds a supplied author image; otherwise the avatar uses initials.
    pub fn portrait(mut self, image: Image) -> Self {
        self.portrait = Some(std::sync::Arc::new(image));
        self
    }

    fn selected_text(&self) -> &str {
        if self.reading.original {
            &self.original
        } else {
            &self.reading.text
        }
    }

    fn source_choice(&self, choice: &str) -> bool {
        !self.detected.is_empty()
            && (choice.eq_ignore_ascii_case(&self.detected)
                || matches!(
                    (self.detected.as_str(), choice),
                    ("French", "Français") | ("法语", "Français")
                ))
    }

    fn choose(&mut self, index: usize, w: &mut Window, cx: &mut Context<Self>) {
        let Some(language) = self.languages.get(index).cloned() else {
            return;
        };
        self.close_menu(w, cx);
        if self.source_choice(&language) {
            self.show_original(cx);
        } else {
            self.start(language, cx);
        }
    }

    fn close_menu(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        self.picker = false;
        if let Some(focus) = &self.trigger {
            focus.focus(w, cx);
        }
        cx.notify();
    }

    fn show_original(&mut self, cx: &mut Context<Self>) {
        if !self.reading.original {
            self.prior_paragraph = Some(self.selected_text().to_owned());
            self.swap.restart(cx);
        }
        self.request = None;
        self.failed = None;
        self.reading.original = true;
        cx.notify();
    }

    fn start(&mut self, language: String, cx: &mut Context<Self>) {
        if self.request.as_ref() == Some(&language) {
            return;
        }
        self.ticket += 1;
        self.failed = None;
        if let Some((source, text)) = self.cache.get(&language)
            && source == &self.original
        {
            self.prior_paragraph = Some(self.selected_text().to_owned());
            self.swap.restart(cx);
            self.reading = Reading {
                language,
                text: text.clone(),
                original: false,
            };
            self.request = None;
        } else {
            if self.request.is_none() {
                self.before_request = self.reading.clone();
            }
            self.reading = self.before_request.clone();
            self.request = Some(language.clone());
            self.prior_paragraph = None;
            self.swap.restart(cx);
            self.reading.text.clear();
            cx.emit(TranslationRequest {
                ticket: self.ticket,
                language,
            });
        }
        cx.notify();
    }

    /// Supplies the cumulative text for the current request. Late tickets are ignored.
    pub fn update_translation(
        &mut self,
        ticket: u64,
        text: Result<String, ()>,
        finished: bool,
        cx: &mut Context<Self>,
    ) {
        if ticket != self.ticket || self.request.is_none() {
            return;
        }
        match text {
            Ok(text) => {
                self.reading.text = text;
                if finished && let Some(language) = self.request.take() {
                    self.cache.insert(
                        language.clone(),
                        (self.original.clone(), self.reading.text.clone()),
                    );
                    self.reading.language = language;
                    self.reading.original = false;
                }
            }
            Err(()) => {
                self.failed = self.request.take();
                self.reading = self.before_request.clone();
            }
        }
        cx.notify();
    }
}

fn sentence_starts(text: &str) -> Vec<usize> {
    let mut starts = Vec::new();
    let mut new = true;
    let mut chars = text.char_indices().peekable();
    while let Some((index, ch)) = chars.next() {
        if new && !ch.is_whitespace() {
            starts.push(index);
            new = false;
        }
        if matches!(ch, '!' | '?' | '。' | '！' | '？')
            || (ch == '.' && chars.peek().is_none_or(|(_, next)| next.is_whitespace()))
        {
            new = true;
        }
    }
    starts
}

fn language_name(name: &str, cx: &App) -> SharedString {
    match name {
        "English" => tr(cx, "English", "英语").into(),
        "Deutsch" => tr(cx, "German", "德语").into(),
        "Español" => tr(cx, "Spanish", "西班牙语").into(),
        "日本語" => tr(cx, "Japanese", "日语").into(),
        "Português" => tr(cx, "Portuguese", "葡萄牙语").into(),
        "Français" | "French" => tr(cx, "French", "法语").into(),
        _ => name.to_owned().into(),
    }
}

impl Render for MessageTranslation {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.trigger.is_none() {
            self.trigger = Some(cx.focus_handle().tab_stop(true));
            self.menu_focus = Some(cx.focus_handle());
        }
        let p = Paint::get(cx);
        let compact = w.viewport_size().width < px(500.);
        let tight = w.viewport_size().width < px(408.);
        let inset = if compact { 16. } else { 20. };
        let radius = crate::foundation::surface_radius(cx);
        let loading = self.request.is_some();
        let has_result = if loading {
            !self.before_request.language.is_empty()
        } else {
            !self.reading.language.is_empty()
        };

        let mut shown = self.selected_text().to_owned();
        let mut styles = vec![];
        let age = self.swap.time(if loading { 0.20 } else { 0.16 }, w, cx);
        if loading {
            let next = &self.reading.text;
            let previous = if self.before_request.original {
                &self.original
            } else {
                &self.before_request.text
            };
            shown = next.clone();
            styles = stream_highlights("translation-words", next, w, cx);
            let n = sentence_starts(next).len();
            if let Some(start) = sentence_starts(previous).get(n).copied() {
                if !shown.is_empty() {
                    shown.push(' ');
                }
                let start_of_old = shown.len();
                shown.push_str(&previous[start..]);
                let dim = 1. - 0.55 * (age / 0.20).clamp(0., 1.);
                styles.push((
                    start_of_old..shown.len(),
                    HighlightStyle {
                        color: Some(p.ink.opacity(dim)),
                        ..Default::default()
                    },
                ));
            }
        }
        let mut message = div()
            .relative()
            .text_size(px(15.))
            .line_height(px(24.))
            .child(
                div()
                    .opacity(if self.prior_paragraph.is_some() {
                        (age / 0.16).clamp(0., 1.)
                    } else {
                        1.
                    })
                    .child(StyledText::new(shown).with_highlights(styles)),
            );
        if let Some(old) = self.prior_paragraph.as_ref().filter(|_| age < 0.1) {
            message = message.child(
                div()
                    .absolute()
                    .inset_0()
                    .overflow_hidden()
                    .opacity((1. - age / 0.10).clamp(0., 1.))
                    .child(old.clone()),
            );
        }

        let measured = self.measured_message.get();
        let height = f32::from(measured.height);
        if height > 0. {
            if measured.width != self.last_width {
                self.message_height.reset(height);
                self.last_width = measured.width;
            } else {
                self.message_height.duration(height, 0.24, cx);
            }
        }
        let current_height = self.message_height.get(w, cx);
        let measure = self.measured_message.clone();
        let message = div()
            .w_full()
            .flex()
            .flex_col()
            .when(height > 0., |v| v.h(px(current_height)).overflow_hidden())
            .on_children_prepainted(move |children, w, _| {
                if let Some(b) = children.first()
                    && b.size != measure.get()
                {
                    measure.set(b.size);
                    w.request_animation_frame();
                }
            })
            .child(message.flex_none().w_full());

        let picked = self
            .request
            .as_ref()
            .or_else(|| (!self.reading.original).then_some(&self.reading.language));
        let trigger_label = picked
            .cloned()
            .unwrap_or_else(|| tr(cx, "Translate", "翻译").to_owned());
        let anchor = self.anchor.clone();
        let trigger = div()
            .on_children_prepainted(move |children, _, _| {
                if let Some(bounds) = children.first() {
                    anchor.set(*bounds);
                }
            })
            .child(activate(
                face(
                    "languages",
                    tr(cx, "Translate", "翻译"),
                    32.,
                    f32::from(radius),
                    Tone::Clear,
                    cx,
                )
                .track_focus(
                    self.trigger
                        .as_ref()
                        .expect("translation trigger initialized"),
                )
                .aria_expanded(self.picker)
                .border_1()
                .border_color(p.line)
                .hover(move |v| v.bg(p.hover))
                .w(px(126.))
                .px_2()
                .text_size(px(13.))
                .child(icon(Glyph::Globe, 14., p.secondary))
                .child(div().flex_1().truncate().child(trigger_label))
                .child(icon(
                    if self.picker { Glyph::Up } else { Glyph::Down },
                    12.,
                    p.secondary,
                )),
                cx,
                |s, w, cx| {
                    if s.picker {
                        s.close_menu(w, cx);
                    } else {
                        s.picker = true;
                        s.active_language = s
                            .languages
                            .iter()
                            .position(|l| l == &s.reading.language)
                            .unwrap_or(0);
                        let focus = s.menu_focus.clone();
                        cx.on_next_frame(w, move |_, w, cx| {
                            if let Some(focus) = focus {
                                focus.focus(w, cx);
                            }
                        });
                        cx.notify();
                    }
                },
            ));
        let controls = row()
            .flex_wrap()
            .child(trigger)
            .when(has_result && !loading, |v| {
                v.child(
                    btn(
                        "original",
                        if self.reading.original {
                            tr(cx, "Show translation", "查看译文")
                        } else {
                            tr(cx, "Show original", "查看原文")
                        },
                        false,
                        cx,
                        |s, _, cx| {
                            if s.reading.original {
                                s.start(s.reading.language.clone(), cx);
                            } else {
                                s.show_original(cx);
                            }
                        },
                    )
                    .min_w(px(126.))
                    .border_1()
                    .border_color(p.line),
                )
            });
        let status: SharedString = if let Some(target) = &self.request {
            format!(
                "{} {}",
                tr(cx, "Translating to", "正在翻译成"),
                language_name(target, cx)
            )
            .into()
        } else if self.failed.is_some() {
            tr(cx, "Couldn't translate", "翻译失败").into()
        } else if !self.reading.original {
            format!(
                "{} {}",
                tr(cx, "Translated from", "译自"),
                language_name(&self.detected, cx)
            )
            .into()
        } else if has_result {
            tr(cx, "Showing original", "正在查看原文").into()
        } else {
            self.credit.clone().into()
        };
        let credit = self.credit.clone();
        let mut status_row = row()
            .gap(px(6.))
            .flex_wrap()
            .when(!compact, |v| v.justify_end().min_w(px(140.)));
        if self.failed.is_some() {
            status_row = status_row.child(icon(Glyph::Alert, 14., p.danger));
        } else if let Some(mark) = brand_mark(&credit, 16., cx) {
            status_row = status_row.child(mark);
        }
        status_row = status_row.child(
            swap_label("translation-status", status, w, cx)
                .text_color(if self.failed.is_some() {
                    p.danger
                } else {
                    p.secondary
                })
                .text_size(px(12.))
                .whitespace_nowrap(),
        );
        if self.failed.is_some() {
            status_row = status_row.child(btn(
                "retry",
                tr(cx, "Retry", "重试"),
                false,
                cx,
                |s, _, cx| {
                    if let Some(target) = s.failed.take() {
                        s.start(target, cx);
                    }
                },
            ));
        }
        let mut card = panel(&self.id, 620., cx)
            .p(px(inset))
            .gap(px(14.))
            .role(Role::Group)
            .aria_label(self.author.clone())
            .child(
                row()
                    .gap(px(11.))
                    .child(if let Some(image) = &self.portrait {
                        img(image.clone())
                            .size(px(34.))
                            .rounded_full()
                            .flex_none()
                            .into_any_element()
                    } else {
                        avatar(&self.author, 0, cx)
                            .size(px(34.))
                            .flex_none()
                            .into_any_element()
                    })
                    .child(
                        column()
                            .flex_1()
                            .gap_0()
                            .child(title(self.author.clone()).truncate())
                            .when(!self.byline.is_empty(), |v| {
                                v.child(note(self.byline.clone(), cx).truncate())
                            }),
                    )
                    .when(!self.detected.is_empty(), |v| {
                        let label = language_name(&self.detected, cx);
                        v.child(
                            badge(
                                if tight {
                                    label
                                } else {
                                    format!("{} {label}", tr(cx, "Detected", "检测到")).into()
                                },
                                cx,
                            )
                            .bg(p.panel)
                            .border_1()
                            .border_color(p.line)
                            .px(px(8.)),
                        )
                    }),
            )
            .child(message)
            .child(
                row()
                    .w_full()
                    .gap_2()
                    .flex_wrap()
                    .items_center()
                    .child(controls)
                    .when(!compact, |v| v.child(div().flex_1()))
                    .child(status_row.when(compact, |v| v.w_full())),
            );
        if self.picker {
            let trigger = self.anchor.get();
            let mut menu = column()
                .id("translation-menu")
                .track_focus(
                    self.menu_focus
                        .as_ref()
                        .expect("translation menu initialized"),
                )
                .w(px(218.))
                .max_w(w.viewport_size().width - px(24.))
                .gap_0()
                .p_1()
                .bg(p.panel)
                .rounded(radius)
                .border_1()
                .border_color(p.line)
                .shadow_lg()
                .occlude()
                .child(note(tr(cx, "Translate to", "翻译成"), cx).px_2().py_1())
                .on_mouse_down_out(cx.listener(move |s, e: &MouseDownEvent, w, cx| {
                    if !trigger.contains(&e.position) {
                        s.close_menu(w, cx);
                    }
                }))
                .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                    match e.keystroke.key.as_str() {
                        "escape" | "tab" => s.close_menu(w, cx),
                        "up" => s.active_language = s.active_language.saturating_sub(1),
                        "down" => {
                            s.active_language =
                                (s.active_language + 1).min(s.languages.len().saturating_sub(1))
                        }
                        "enter" | "space" => s.choose(s.active_language, w, cx),
                        _ => return,
                    }
                    cx.notify();
                    if e.keystroke.key.as_str() != "tab" {
                        cx.stop_propagation();
                        w.prevent_default();
                    }
                }));
            for (i, name) in self.languages.iter().enumerate() {
                let original = self.source_choice(name);
                let selected = if self.reading.original {
                    original
                } else {
                    self.reading.language == *name
                };
                menu = menu.child(
                    item(("language", i), name.clone(), cx, move |s, w, cx| {
                        s.choose(i, w, cx)
                    })
                    .w_full()
                    .h(px(32.))
                    .py_0()
                    .gap_2()
                    .when(self.active_language == i, |v| v.bg(p.hover))
                    .child(
                        div()
                            .size(px(14.))
                            .flex_none()
                            .when(selected, |v| v.child(icon(Glyph::Check, 14., p.ink))),
                    )
                    .child(div().flex_1().truncate().child(name.clone()))
                    .child(note(
                        if original {
                            tr(cx, "Original", "原文").into()
                        } else {
                            language_name(name, cx)
                        },
                        cx,
                    )),
                );
            }
            card = card.child(
                deferred(
                    anchored()
                        .position(trigger.bottom_left() + point(px(0.), px(5.)))
                        .anchor(Anchor::TopLeft)
                        .snap_to_window_with_margin(px(8.))
                        .child(menu),
                )
                .with_priority(2),
            );
        }
        card
    }
}

#[cfg(test)]
mod tests {
    use super::sentence_starts;

    #[test]
    fn translation_replaces_sentences_without_splitting_a_version_or_japanese_text() {
        let source = "La version 4.2 est prête. Merci !";
        let spans = sentence_starts(source);
        assert_eq!(&source[spans[1]..], "Merci !");
        assert_eq!(spans.len(), 2);
        let japanese = "こんにちは。準備できました。";
        assert_eq!(
            &japanese[sentence_starts(japanese)[1]..],
            "準備できました。"
        );
    }
}
