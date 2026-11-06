//! Host-driven tone adjustments for an editable draft.
use crate::{
    components::slider::{SlideValue, Slider},
    primitives::{
        clipboard,
        editor::{EditEvent, Editor, InputMode},
        family::{Paint, Tone, face},
        nav::tr,
    },
};
use gpui::{prelude::*, *};
use std::{rc::Rc, time::Duration};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum VoiceTone {
    #[default]
    Neutral,
    Friendly,
    Confident,
    Empathetic,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ToneSettings {
    pub formality: u8,
    pub detail: u8,
    pub voice: VoiceTone,
}
impl Default for ToneSettings {
    fn default() -> Self {
        Self {
            formality: 1,
            detail: 1,
            voice: VoiceTone::Neutral,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToneStatus {
    Original,
    Waiting,
    Rewriting,
    Rewritten,
    Stopped,
    Error,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToneTunerEvent {
    Settings(ToneSettings),
    Rewrite {
        ticket: u64,
        text: SharedString,
        settings: ToneSettings,
    },
    Cancel {
        ticket: u64,
    },
    Changed(SharedString),
    Rewritten(SharedString),
    CopyResult(bool),
}
#[derive(Default)]
struct RequestGate {
    serial: u64,
    active: Option<u64>,
}
impl RequestGate {
    fn begin(&mut self) -> u64 {
        self.serial = self.serial.wrapping_add(1);
        self.active = Some(self.serial);
        self.serial
    }
    fn cancel(&mut self) -> Option<u64> {
        self.active.take()
    }
    fn settle(&mut self, ticket: u64) -> bool {
        if self.active == Some(ticket) {
            self.active = None;
            true
        } else {
            false
        }
    }
}
/// Return byte lengths and changed flags for the new text; bound memory for unusually long inputs.
fn changed_words(original: &str, edited: &str) -> Vec<(usize, bool)> {
    let a: Vec<_> = original.split_word_bounds().collect();
    let b: Vec<_> = edited.split_word_bounds().collect();
    if a.len() > 350 || b.len() > 350 {
        return vec![(edited.len(), false)];
    }
    let stride = b.len() + 1;
    let mut table = vec![0u16; (a.len() + 1) * stride];
    for i in (0..a.len()).rev() {
        for j in (0..b.len()).rev() {
            table[i * stride + j] = if a[i] == b[j] {
                1 + table[(i + 1) * stride + j + 1]
            } else {
                table[(i + 1) * stride + j].max(table[i * stride + j + 1])
            };
        }
    }
    let (mut i, mut j) = (0, 0);
    let mut kept = vec![false; b.len()];
    while i < a.len() && j < b.len() {
        if a[i] == b[j] {
            kept[j] = true;
            i += 1;
            j += 1;
        } else if table[(i + 1) * stride + j] >= table[i * stride + j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }
    b.iter()
        .enumerate()
        .map(|(j, part)| {
            (
                part.len(),
                !kept[j] && part.chars().any(char::is_alphanumeric),
            )
        })
        .collect()
}
fn can_apply(
    active: Option<u64>,
    incoming: u64,
    requested_revision: u64,
    revision: u64,
    requested_draft: &str,
    current: &str,
) -> bool {
    active == Some(incoming) && revision == requested_revision && requested_draft == current
}
pub struct ToneTuner {
    label: SharedString,
    editor: Entity<Editor>,
    formality: Entity<Slider>,
    detail: Entity<Slider>,
    settings: ToneSettings,
    source: String,
    rendered: String,
    revision: u64,
    requested_revision: u64,
    requested_draft: String,
    gate: RequestGate,
    timer: Option<Task<()>>,
    status: ToneStatus,
    error: SharedString,
    show_changes: bool,
    copying: bool,
    copied: Option<bool>,
    _subscriptions: Vec<Subscription>,
}
impl EventEmitter<ToneTunerEvent> for ToneTuner {}
impl ToneTuner {
    pub fn new(
        label: impl Into<SharedString>,
        text: impl Into<String>,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let label = label.into();
        let text = text.into();
        let editor = cx.new(|cx| {
            let mut e = Editor::new(label.clone(), InputMode::Multi, w, cx);
            e.set_value(text.clone(), w, cx);
            e
        });
        let formality = cx.new(|cx| {
            Slider::new(
                tr(cx, "Formality", "正式程度"),
                0.0..=2.0,
                1.,
                SlideValue {
                    low: 1.,
                    high: None,
                },
                cx,
            )
        });
        let detail = cx.new(|cx| {
            Slider::new(
                tr(cx, "Detail", "详细程度"),
                0.0..=2.0,
                1.,
                SlideValue {
                    low: 1.,
                    high: None,
                },
                cx,
            )
        });
        let edits = cx.subscribe_in(
            &editor,
            w,
            |s: &mut Self, view, event: &EditEvent, _, cx| {
                if *event == EditEvent::Changed {
                    let value = view.read(cx).value().to_owned();
                    if value == s.rendered {
                        return;
                    }
                    s.rendered = value.clone();
                    s.source = value.clone();
                    s.revision = s.revision.wrapping_add(1);
                    s.cancel_pending(cx);
                    s.status = ToneStatus::Original;
                    s.copied = None;
                    cx.emit(ToneTunerEvent::Changed(value.into()));
                    cx.notify();
                }
            },
        );
        let formal_sub = cx.subscribe(&formality, |s: &mut Self, _, value: &SlideValue, cx| {
            let mut settings = s.settings;
            settings.formality = (value.low.round() as u8).min(2);
            s.set_settings(settings, cx);
        });
        let detail_sub = cx.subscribe(&detail, |s: &mut Self, _, value: &SlideValue, cx| {
            let mut settings = s.settings;
            settings.detail = (value.low.round() as u8).min(2);
            s.set_settings(settings, cx);
        });
        Self {
            label,
            editor,
            formality,
            detail,
            settings: ToneSettings::default(),
            source: text.clone(),
            rendered: text,
            revision: 0,
            requested_revision: 0,
            requested_draft: String::new(),
            gate: RequestGate::default(),
            timer: None,
            status: ToneStatus::Original,
            error: "".into(),
            show_changes: true,
            copying: false,
            copied: None,
            _subscriptions: vec![edits, formal_sub, detail_sub],
        }
    }
    /// Replace the source draft explicitly, cancelling any pending rewrite.
    pub fn set_text(&mut self, text: impl Into<String>, w: &Window, cx: &mut Context<Self>) {
        self.cancel_pending(cx);
        let text = text.into();
        self.source = text.clone();
        self.rendered = text.clone();
        self.revision = self.revision.wrapping_add(1);
        self.status = ToneStatus::Original;
        self.error = "".into();
        self.copied = None;
        self.editor
            .update(cx, |editor, cx| editor.set_value(text, w, cx));
        self.apply_highlight(cx);
        cx.notify();
    }
    pub fn settings(&self) -> ToneSettings {
        self.settings
    }
    pub fn value(&self, cx: &App) -> String {
        self.editor.read(cx).value().to_owned()
    }
    pub fn status(&self) -> ToneStatus {
        self.status
    }
    fn cancel_pending(&mut self, cx: &mut Context<Self>) {
        self.timer = None;
        if let Some(ticket) = self.gate.cancel() {
            cx.emit(ToneTunerEvent::Cancel { ticket });
        }
    }
    pub fn stop(&mut self, cx: &mut Context<Self>) {
        if self.timer.is_none() && self.gate.active.is_none() {
            return;
        }
        self.cancel_pending(cx);
        self.status = ToneStatus::Stopped;
        cx.notify();
    }
    pub fn set_settings(&mut self, mut settings: ToneSettings, cx: &mut Context<Self>) {
        settings.formality = settings.formality.min(2);
        settings.detail = settings.detail.min(2);
        if settings == self.settings {
            return;
        }
        self.settings = settings;
        self.cancel_pending(cx);
        self.status = ToneStatus::Waiting;
        self.error = "".into();
        cx.emit(ToneTunerEvent::Settings(settings));
        self.formality.update(cx, |slider, cx| {
            slider.set_low(f64::from(settings.formality), cx)
        });
        self.detail.update(cx, |slider, cx| {
            slider.set_low(f64::from(settings.detail), cx)
        });
        self.timer = Some(cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(220))
                .await;
            let _ = this.update(cx, |s, cx| s.rewrite(cx));
        }));
        cx.notify();
    }
    pub fn rewrite(&mut self, cx: &mut Context<Self>) {
        if self.source.trim().is_empty() {
            return;
        }
        self.cancel_pending(cx);
        let ticket = self.gate.begin();
        self.requested_revision = self.revision;
        self.requested_draft = self.editor.read(cx).value().to_owned();
        self.status = ToneStatus::Rewriting;
        cx.emit(ToneTunerEvent::Rewrite {
            ticket,
            text: self.source.clone().into(),
            settings: self.settings,
        });
        cx.notify();
    }
    /// Accept only the current ticket and an unchanged editable draft.
    pub fn finish(
        &mut self,
        ticket: u64,
        text: impl Into<String>,
        w: &Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if !can_apply(
            self.gate.active,
            ticket,
            self.requested_revision,
            self.revision,
            &self.requested_draft,
            self.editor.read(cx).value(),
        ) {
            return false;
        }
        self.gate.settle(ticket);
        let text = text.into();
        self.rendered = text.clone();
        self.status = ToneStatus::Rewritten;
        self.copied = None;
        self.editor
            .update(cx, |e, cx| e.set_value(text.clone(), w, cx));
        self.apply_highlight(cx);
        cx.emit(ToneTunerEvent::Rewritten(text.into()));
        cx.notify();
        true
    }
    pub fn fail(
        &mut self,
        ticket: u64,
        error: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> bool {
        if !can_apply(
            self.gate.active,
            ticket,
            self.requested_revision,
            self.revision,
            &self.requested_draft,
            self.editor.read(cx).value(),
        ) {
            return false;
        }
        self.gate.settle(ticket);
        self.error = error.into();
        self.status = ToneStatus::Error;
        cx.notify();
        true
    }
    fn apply_highlight(&mut self, cx: &mut Context<Self>) {
        let original = self.source.clone();
        let expected = self.rendered.clone();
        let ranges = changed_words(&original, &expected);
        let show = self.show_changes;
        let tint = Paint::get(cx).bright.opacity(0.18);
        self.editor.update(cx, |editor, cx| {
            editor.set_run_styler(Rc::new(move |text, base| {
                if !show || text != expected {
                    return vec![base.clone()];
                }
                ranges
                    .iter()
                    .map(|(len, changed)| {
                        let mut run = base.clone();
                        run.len = *len;
                        if *changed {
                            run.background_color = Some(tint);
                        }
                        run
                    })
                    .collect()
            }));
            cx.notify();
        });
    }
    fn copy(&mut self, cx: &mut Context<Self>) {
        if self.copying {
            return;
        }
        self.copying = true;
        let text = self.editor.read(cx).value().to_owned();
        let write = clipboard::write(&text, cx);
        cx.spawn(async move |this, cx| {
            let ok = write.await;
            let _ = this.update(cx, |s, cx| {
                s.copying = false;
                s.copied = Some(ok);
                cx.emit(ToneTunerEvent::CopyResult(ok));
                cx.notify();
            });
        })
        .detach();
    }
}
impl Render for ToneTuner {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let r = crate::foundation::surface_radius(cx);
        let status = match self.status {
            ToneStatus::Original => tr(cx, "Original draft", "原始草稿"),
            ToneStatus::Waiting | ToneStatus::Rewriting => tr(cx, "Rewriting…", "正在调整…"),
            ToneStatus::Rewritten => tr(cx, "Rewritten", "已调整"),
            ToneStatus::Stopped => tr(cx, "Stopped", "已停止"),
            ToneStatus::Error => tr(cx, "Could not rewrite", "调整失败"),
        };
        let mut root = div()
            .id("tone-tuner")
            .role(Role::Group)
            .aria_label(self.label.clone())
            .w_full()
            .max_w(px(640.))
            .rounded(r)
            .border_1()
            .border_color(p.line)
            .bg(p.panel)
            .text_color(p.ink)
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(12.))
                    .py(px(16.))
                    .px(px(24.))
                    .border_b_1()
                    .border_color(p.line)
                    .child(self.label.clone())
                    .child(
                        div()
                            .id("tone-status")
                            .role(Role::Status)
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .text_color(p.secondary)
                            .when(self.status == ToneStatus::Rewritten, |v| {
                                v.child(
                                    div()
                                        .size(px(10.))
                                        .rounded(px(3.))
                                        .bg(p.bright.opacity(0.18))
                                        .border_1()
                                        .border_color(p.bright.opacity(0.35)),
                                )
                            })
                            .child(status),
                    ),
            )
            .child(
                div()
                    .w_full()
                    .pt(px(20.))
                    .px(px(24.))
                    .pb(px(24.))
                    .text_size(px(16.))
                    .line_height(px(25.6))
                    .child(self.editor.clone()),
            );
        if !self.error.is_empty() {
            root = root.child(
                div()
                    .id("tone-error")
                    .role(Role::Alert)
                    .text_color(p.danger)
                    .text_size(px(12.))
                    .child(self.error.clone()),
            );
        }
        let mut chips = div().flex().flex_wrap().gap(px(8.));
        for (i, (voice, name)) in [
            (VoiceTone::Friendly, tr(cx, "Friendly", "友好")),
            (VoiceTone::Confident, tr(cx, "Confident", "自信")),
            (VoiceTone::Empathetic, tr(cx, "Empathetic", "共情")),
        ]
        .into_iter()
        .enumerate()
        {
            let selected = self.settings.voice == voice;
            chips = chips.child(
                face(("tone-voice", i), name, 36., 8., Tone::Clear, cx)
                    .h(px(36.))
                    .px(px(12.))
                    .gap(px(6.))
                    .rounded(px(8.))
                    .border_1()
                    .border_color(if selected {
                        p.line.blend(p.bright.opacity(0.45))
                    } else {
                        p.line
                    })
                    .bg(if selected {
                        p.bright.opacity(0.12)
                    } else {
                        gpui::transparent_black()
                    })
                    .text_color(if selected { p.ink } else { p.secondary })
                    .font_weight(FontWeight::MEDIUM)
                    .when(!selected, |b| b.hover(|s| s.bg(p.tray).text_color(p.ink)))
                    .aria_toggled(selected.into())
                    .child(name)
                    .on_click(cx.listener(move |s, _, _, cx| {
                        let mut next = s.settings;
                        if next.voice == voice {
                            next.voice = VoiceTone::Neutral;
                        } else {
                            next.voice = voice;
                            match voice {
                                VoiceTone::Friendly => {
                                    next.formality = 0;
                                    next.detail = 1;
                                }
                                VoiceTone::Confident => {
                                    next.formality = 2;
                                    next.detail = 0;
                                }
                                VoiceTone::Empathetic => {
                                    next.formality = 1;
                                    next.detail = 2;
                                }
                                VoiceTone::Neutral => {}
                            }
                        }
                        s.set_settings(next, cx);
                    })),
            );
        }
        root = root.child(
            div()
                .flex()
                .flex_col()
                .gap(px(16.))
                .pt(px(16.))
                .px(px(24.))
                .pb(px(12.))
                .border_t_1()
                .border_color(p.line)
                .child(chips)
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .gap(px(24.))
                        .child(div().flex_1().min_w(px(160.)).child(self.formality.clone()))
                        .child(div().flex_1().min_w(px(160.)).child(self.detail.clone())),
                ),
        );
        let copy = match self.copied {
            Some(true) => tr(cx, "Copied", "已复制"),
            Some(false) => tr(cx, "Copy failed — retry", "复制失败，重试"),
            None => tr(cx, "Copy", "复制"),
        };
        let mut actions = div()
            .min_h(px(56.))
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(12.))
            .py(px(12.))
            .px(px(24.))
            .border_t_1()
            .border_color(p.line)
            .child(
                face(
                    "tone-changes",
                    tr(cx, "Highlight changes", "突出显示修改"),
                    32.,
                    8.,
                    Tone::Clear,
                    cx,
                )
                .aria_toggled(self.show_changes.into())
                .px_2()
                .child(tr(cx, "Highlight changes", "突出显示修改"))
                .on_click(cx.listener(|s, _, _, cx| {
                    s.show_changes = !s.show_changes;
                    s.apply_highlight(cx);
                    cx.notify();
                })),
            )
            .child(
                face("tone-copy", copy, 32., 8., Tone::Surface, cx)
                    .px_2()
                    .child(copy)
                    .on_click(cx.listener(|s, _, _, cx| s.copy(cx))),
            );
        if matches!(self.status, ToneStatus::Waiting | ToneStatus::Rewriting) {
            actions = actions.child(
                face(
                    "tone-stop",
                    tr(cx, "Stop", "停止"),
                    32.,
                    8.,
                    Tone::Surface,
                    cx,
                )
                .px_2()
                .child(tr(cx, "Stop", "停止"))
                .on_click(cx.listener(|s, _, _, cx| s.stop(cx))),
            );
        } else if matches!(self.status, ToneStatus::Error | ToneStatus::Stopped) {
            actions = actions.child(
                face(
                    "tone-retry",
                    tr(cx, "Retry", "重试"),
                    28.,
                    999.,
                    Tone::Surface,
                    cx,
                )
                .px_2()
                .child(tr(cx, "Retry", "重试"))
                .on_click(cx.listener(|s, _, _, cx| s.rewrite(cx))),
            );
        }
        root.child(actions)
    }
}
#[cfg(test)]
mod tests {
    use super::{RequestGate, can_apply, changed_words};
    #[test]
    fn cancelled_and_replaced_results_are_rejected() {
        let mut gate = RequestGate::default();
        let first = gate.begin();
        assert_eq!(gate.cancel(), Some(first));
        let second = gate.begin();
        assert!(!gate.settle(first));
        assert!(gate.settle(second));
        assert!(!gate.settle(second));
    }
    #[test]
    fn word_diff_preserves_utf8_byte_lengths() {
        let text = "你好，new world!";
        let runs = changed_words("你好，old world!", text);
        assert_eq!(
            runs.iter().map(|(length, _)| length).sum::<usize>(),
            text.len()
        );
        assert!(runs.iter().any(|(_, changed)| *changed));
    }
    #[test]
    fn asynchronous_result_never_overwrites_edited_draft() {
        assert!(can_apply(Some(4), 4, 2, 2, "draft", "draft"));
        assert!(!can_apply(Some(4), 3, 2, 2, "draft", "draft"));
        assert!(!can_apply(Some(4), 4, 2, 3, "draft", "draft"));
        assert!(!can_apply(Some(4), 4, 2, 2, "draft", "draft edited"));
    }
}
