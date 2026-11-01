//! Edit a value in place with validation, optimistic save and retry.
use crate::{
    foundation::Theme,
    primitives::{
        control::UiLanguage,
        editor::{EditEvent, Editor, InputMode},
        family::{Paint, PhaseClock, Tone, face, measure as kui_measure},
        glyph::{Glyph, icon, spinner, transformed},
    },
};
use gpui::{prelude::*, *};
use std::rc::Rc;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditPhase {
    Idle,
    Editing,
    Saving,
    Saved,
    Failed,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveRequest {
    pub revision: u64,
    pub value: SharedString,
}
type Validator = Rc<dyn Fn(&str) -> Option<SharedString>>;
pub struct InlineEdit {
    id: ElementId,
    label: SharedString,
    saved: SharedString,
    draft: SharedString,
    message: SharedString,
    editor: Entity<Editor>,
    rest: FocusHandle,
    group: FocusHandle,
    _leave: Subscription,
    phase: EditPhase,
    multi: bool,
    validate: Option<Validator>,
    revision: u64,
    clock: PhaseClock,
    _sub: Subscription,
}
impl EventEmitter<SaveRequest> for InlineEdit {}
impl InlineEdit {
    pub fn new(
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        value: impl Into<SharedString>,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let label = label.into();
        let saved = value.into();
        let editor = cx.new(|cx| Editor::new(label.clone(), InputMode::Single, w, cx));
        let group = cx.focus_handle();
        let leave = cx.on_focus_out(&group, w, |s: &mut Self, _, w, cx| {
            if s.phase == EditPhase::Editing {
                s.submit(false, w, cx)
            }
        });
        let sub = cx.subscribe_in(&editor, w, |s, _, ev: &EditEvent, w, cx| {
            if s.phase != EditPhase::Editing {
                return;
            }
            match ev {
                EditEvent::Submitted => s.submit(true, w, cx),
                EditEvent::Escape => s.cancel(w, cx),

                EditEvent::Changed if !s.message.is_empty() => {
                    s.message = s.problem(cx).unwrap_or_default();
                    cx.notify()
                }
                _ => {}
            }
        });
        Self {
            id: id.into(),
            label,
            saved: saved.clone(),
            draft: saved,
            message: "".into(),
            editor,
            rest: cx.focus_handle().tab_stop(true),
            phase: EditPhase::Idle,
            multi: false,
            validate: None,
            revision: 0,
            clock: Default::default(),
            _sub: sub,
            group,
            _leave: leave,
        }
    }
    pub fn multiline(mut self) -> Self {
        self.multi = true;
        self
    }
    pub fn with_validator(mut self, f: impl Fn(&str) -> Option<SharedString> + 'static) -> Self {
        self.validate = Some(Rc::new(f));
        self
    }
    pub fn saved(&self) -> &str {
        &self.saved
    }
    pub fn phase(&self) -> EditPhase {
        self.phase
    }
    pub fn set_saved(&mut self, v: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.saved = v.into();
        if self.phase != EditPhase::Editing {
            self.revision = self.revision.wrapping_add(1);
            self.phase = EditPhase::Idle;
            self.message = "".into()
        }
        cx.notify()
    }
    fn begin(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.phase == EditPhase::Saving {
            return;
        }
        let value = if self.phase == EditPhase::Failed {
            self.draft.clone()
        } else {
            self.saved.clone()
        };
        self.phase = EditPhase::Editing;
        self.message = "".into();
        self.editor.update(cx, |s, cx| {
            s.mode = if self.multi {
                InputMode::InlineMulti
            } else {
                InputMode::Single
            };
            s.font_size = if self.multi { 14. } else { 23. };
            s.set_value(value.to_string(), w, cx);
            s.focus(w, cx);
            s.select_all(w, cx)
        });
        w.request_virtual_keyboard();
        cx.notify()
    }
    fn problem(&self, cx: &App) -> Option<SharedString> {
        self.validate
            .as_ref()
            .and_then(|f| f(self.editor.read(cx).value().trim()))
    }
    fn cancel(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        self.phase = EditPhase::Idle;
        self.message = "".into();
        self.rest.focus(w, cx);
        cx.notify()
    }
    fn submit(&mut self, restore: bool, w: &mut Window, cx: &mut Context<Self>) {
        if let Some(msg) = self.problem(cx) {
            self.message = msg;
            cx.notify();
            return;
        }
        let raw = self.editor.read(cx).value();
        self.draft = if self.multi {
            raw.trim().into()
        } else {
            raw.split_whitespace().collect::<Vec<_>>().join(" ").into()
        };
        if self.draft == self.saved {
            self.phase = EditPhase::Idle;
            cx.notify()
        } else {
            self.save(cx)
        }
        if restore {
            self.rest.focus(w, cx)
        }
    }
    fn save(&mut self, cx: &mut Context<Self>) {
        self.phase = EditPhase::Saving;
        self.message = "".into();
        self.revision = self.revision.wrapping_add(1);
        self.clock.restart(cx);
        cx.emit(SaveRequest {
            revision: self.revision,
            value: self.draft.clone(),
        });
        cx.notify()
    }
    pub fn finish(
        &mut self,
        revision: u64,
        result: Result<(), SharedString>,
        cx: &mut Context<Self>,
    ) {
        if self.phase != EditPhase::Saving || revision != self.revision {
            return;
        }
        match result {
            Ok(()) => {
                self.saved = self.draft.clone();
                self.phase = EditPhase::Saved
            }
            Err(e) => {
                self.message = e;
                self.phase = EditPhase::Failed
            }
        }
        self.clock.restart(cx);
        cx.notify()
    }
}
impl Render for InlineEdit {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let r = Theme::global(cx).tokens.radius.clone();
        let l = UiLanguage::current(cx);
        let title = !self.multi;
        let available = f32::from(w.viewport_size().width).min(440.) - 52.;
        let age = self.clock.time(
            if self.phase == EditPhase::Saving {
                60.
            } else {
                1.6
            },
            w,
            cx,
        );
        let displayed = if self.phase == EditPhase::Saving {
            self.draft.clone()
        } else {
            self.saved.clone()
        };
        let mut root = div()
            .id(self.id.clone())
            .track_focus(&self.group)
            .tab_stop(false)
            .on_mouse_down_out(cx.listener(|s, _, w, cx| {
                if s.phase == EditPhase::Editing {
                    s.submit(false, w, cx)
                }
            }))
            .w_full()
            .min_w_0()
            .flex()
            .flex_col()
            .gap(px(3.))
            .text_color(p.ink);
        if self.phase == EditPhase::Editing {
            let mut row = div().flex().items_start().gap(px(4.)).w_full();
            let ed = self.editor.clone();
            let width = available - 66.;
            let rows = if self.multi {
                let text = ed.read(cx).value();
                let runs = [w.text_style().to_run(text.len())];
                w.text_system()
                    .shape_text(
                        text.to_owned().into(),
                        px(14.),
                        &runs,
                        Some(px(width.max(40.))),
                        None,
                    )
                    .map_or(2., |lines| {
                        lines
                            .iter()
                            .map(|l| 1. + l.wrap_boundaries().len() as f32)
                            .sum::<f32>()
                    })
                    .clamp(2., 9.)
            } else {
                1.
            };
            row = row.child(
                div()
                    .when(title, |s| {
                        let value: SharedString = ed.read(cx).value().to_owned().into();
                        let measured = kui_measure(&value, 23., w);
                        s.w(px((measured + 20.).max(110.).min(width.max(60.))))
                    })
                    .when(!title, |s| s.flex_1())
                    .min_w_0()
                    .px(px(7.))
                    .h(px(if title { 36. } else { rows * 21. + 8. }))
                    .rounded(r.md)
                    .border_1()
                    .border_color(if self.message.is_empty() {
                        p.ink
                    } else {
                        p.danger
                    })
                    .child(ed),
            );
            for (id, label, glyph, save) in [
                ("save", l.text("Save", "保存"), Glyph::Check, true),
                ("cancel", l.text("Cancel", "取消"), Glyph::Close, false),
            ] {
                row = row.child(
                    face(
                        id,
                        label,
                        28.,
                        r.md.into(),
                        if save { Tone::Primary } else { Tone::Surface },
                        cx,
                    )
                    .mt(px(4.))
                    .border_1()
                    .border_color(if save { p.bright } else { p.line })
                    .w(px(28.))
                    .hover(move |s| s.bg(p.hover))
                    .child(icon(glyph, 15., if save { p.inverse } else { p.secondary }))
                    .on_mouse_down(MouseButton::Left, |_, w, _| w.prevent_default())
                    .on_click(cx.listener(move |s, _, w, cx| {
                        if s.phase == EditPhase::Editing {
                            if save {
                                s.submit(true, w, cx)
                            } else {
                                s.cancel(w, cx)
                            }
                        }
                    })),
                )
            }
            root = root.child(row)
        } else {
            let name = format!("{}: {}", self.label, displayed);
            let label = self.label.clone();
            let waiting = self.phase == EditPhase::Saving;
            let mut row = div()
                .id("edit")
                .group("editable")
                .debug_selector(move || label.to_string())
                .track_focus(&self.rest)
                .tab_stop(true)
                .role(Role::Button)
                .aria_label(name)
                .cursor_pointer()
                .max_w_full()
                .flex()
                .items_center()
                .gap(px(5.))
                .rounded(r.md)
                .px(px(2.))
                .hover(move |s| s.bg(p.hover))
                .focus_visible(move |s| s.border_1().border_color(p.ink))
                .child(
                    div()
                        .min_w_0()
                        .text_size(px(if title { 23. } else { 14. }))
                        .when(title, |s| s.font_weight(FontWeight::SEMIBOLD))
                        .child(if displayed.is_empty() {
                            self.label.clone()
                        } else {
                            displayed
                        }),
                );
            row = if waiting {
                row.child(spinner(age * 5., 14., p.secondary))
            } else if self.phase == EditPhase::Saved && age < 1.6 {
                row.child(transformed(
                    Glyph::Check,
                    15.,
                    p.success,
                    0.,
                    1.,
                    (0., 0.),
                    (age / 0.2).min(1.),
                ))
            } else {
                row.child(
                    div()
                        .opacity(0.)
                        .group_hover("editable", |s| s.opacity(1.))
                        .child(icon(Glyph::Edit, 13., p.faint)),
                )
            };
            root = root.child(
                row.on_click(cx.listener(|s, _, w, cx| s.begin(w, cx)))
                    .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                        if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                            s.begin(w, cx);
                            w.prevent_default();
                            cx.stop_propagation()
                        }
                    })),
            )
        }
        if !self.message.is_empty() {
            root = root.child(
                div()
                    .id("message")
                    .role(Role::Alert)
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .text_color(p.danger)
                    .text_size(px(13.))
                    .child(self.message.clone())
                    .when(self.phase == EditPhase::Failed, |e| {
                        e.child(
                            face(
                                "retry",
                                l.text("Retry", "重试"),
                                24.,
                                r.md.into(),
                                Tone::Clear,
                                cx,
                            )
                            .child(l.text("Retry", "重试"))
                            .on_click(cx.listener(|s, _, _, cx| {
                                if s.phase == EditPhase::Failed {
                                    s.save(cx)
                                }
                            })),
                        )
                    }),
            )
        }
        root
    }
}
