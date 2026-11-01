//! Labelled input chrome, validation messages and trailing field actions.
use crate::{
    foundation::Theme,
    primitives::{
        control::UiLanguage,
        editor::{EditEvent, Editor, InputMode},
        family::{Paint, PhaseClock, Slide, Tone, ease, face},
        glyph::{Glyph, icon, transformed},
    },
};
use gpui::{prelude::*, *};
use std::marker::PhantomData;

fn resize_grip() -> impl IntoElement {
    canvas(
        |_, _, _| (),
        |bounds, _, w, _| {
            // Keep the grip inside the curved border, independent of font metrics.
            let corner = bounds.bottom_right() - point(px(4.5), px(4.5));
            let mut strokes = PathBuilder::stroke(px(1.));
            for reach in [7., 3.5] {
                strokes.move_to(corner - point(px(reach), px(0.)));
                strokes.line_to(corner - point(px(0.), px(reach)));
            }
            if let Ok(path) = strokes.build() {
                w.paint_path(path, w.text_style().color);
            }
        },
    )
    .size_full()
}

pub trait FieldStyle: 'static {
    const MODE: InputMode;
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FieldEvent {
    Changed(SharedString),
    Submitted,
    Visibility(bool),
    Blurred,
}
pub struct Field<S: FieldStyle> {
    id: ElementId,
    label: SharedString,
    editor: Entity<Editor>,
    description: SharedString,
    error: SharedString,
    note_height: Slide,
    error_height: Slide,
    error_copy: SharedString,
    width: std::rc::Rc<std::cell::Cell<Pixels>>,
    note_clock: PhaseClock,
    action_motion: Slide,
    height: f32,
    resize: Option<(Pixels, f32)>,
    _sub: Subscription,
    style: PhantomData<S>,
}
impl<S: FieldStyle> EventEmitter<FieldEvent> for Field<S> {}
impl<S: FieldStyle> Focusable for Field<S> {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.editor.read(cx).focus.clone()
    }
}
impl<S: FieldStyle> Field<S> {
    pub fn new(
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let label = label.into();
        let editor = cx.new(|cx| Editor::new(label.clone(), S::MODE, w, cx));
        let sub = cx.subscribe_in(&editor, w, |s, ed, ev: &EditEvent, w, cx| {
            match ev {
                EditEvent::Changed => {
                    cx.emit(FieldEvent::Changed(ed.read(cx).value().to_owned().into()))
                }
                EditEvent::Submitted => cx.emit(FieldEvent::Submitted),
                EditEvent::Blurred => cx.emit(FieldEvent::Blurred),
                EditEvent::Escape if S::MODE == InputMode::Search => s.clear(w, cx),
                _ => {}
            }
            cx.notify()
        });
        Self {
            id: id.into(),
            label,
            editor,
            description: "".into(),
            error: "".into(),
            note_height: Slide::snappy(0.),
            error_height: Slide::snappy(0.),
            error_copy: "".into(),
            width: Default::default(),
            note_clock: Default::default(),
            action_motion: Slide::snappy(0.),
            height: if S::MODE == InputMode::Multi {
                110.
            } else {
                44.
            },
            resize: None,
            _sub: sub,
            style: PhantomData,
        }
    }
    pub fn with_value(self, value: impl Into<String>, w: &Window, cx: &mut Context<Self>) -> Self {
        self.editor.update(cx, |s, cx| s.set_value(value, w, cx));
        self
    }
    pub fn with_placeholder(
        self,
        placeholder: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> Self {
        self.editor
            .update(cx, |s, _| s.set_placeholder(placeholder));
        self
    }
    pub fn with_description(mut self, copy: impl Into<SharedString>) -> Self {
        self.description = copy.into();
        self.note_height
            .reset(if self.description.is_empty() { 0. } else { 1. });
        self
    }
    pub fn value<'a>(&'a self, cx: &'a App) -> &'a str {
        self.editor.read(cx).value()
    }
    pub fn set_value(&mut self, value: impl Into<String>, w: &Window, cx: &mut Context<Self>) {
        self.editor.update(cx, |s, cx| s.set_value(value, w, cx))
    }
    pub fn set_description(&mut self, text: impl Into<SharedString>, cx: &mut Context<Self>) {
        let text = text.into();
        if self.description != text {
            self.description = text;
            self.note_clock.restart(cx);
            cx.notify()
        }
    }
    pub fn set_error(&mut self, text: impl Into<SharedString>, cx: &mut Context<Self>) {
        let text = text.into();
        if self.error != text {
            if !text.is_empty() {
                self.error_copy = text.clone();
            }
            self.error_height
                .set(if text.is_empty() { 0. } else { 1. }, cx);
            self.error = text;
            self.note_clock.restart(cx);
            cx.notify()
        }
    }
    pub fn set_disabled(&mut self, disabled: bool, cx: &mut Context<Self>) {
        self.editor.update(cx, |e, cx| {
            e.disabled = disabled;
            cx.notify()
        });
        cx.notify()
    }
    pub fn editor(&self) -> Entity<Editor> {
        self.editor.clone()
    }
    fn clear(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        self.editor.update(cx, |ed, cx| {
            ed.set_value("", w, cx);
            ed.focus(w, cx)
        })
    }
}
impl<S: FieldStyle> Render for Field<S> {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let r = Theme::global(cx).tokens.radius.clone();
        let l = UiLanguage::current(cx);
        let ed = self.editor.read(cx);
        let focus = ed.focus.clone();
        let focused = focus.is_focused(w);
        let disabled = ed.disabled;
        let revealed = ed.revealed;
        let has_value = !ed.value().is_empty();
        let multi = S::MODE == InputMode::Multi;
        let secret = S::MODE == InputMode::Password;
        let search = S::MODE == InputMode::Search;
        self.action_motion.set(
            if (secret && revealed) || (search && has_value) {
                1.
            } else {
                0.
            },
            cx,
        );
        let slide = self.action_motion.get(w, cx).clamp(0., 1.);
        let selection = self.editor.clone();
        let label = div()
            .id("label")
            .cursor(CursorStyle::IBeam)
            .text_size(px(14.))
            .font_weight(FontWeight::MEDIUM)
            .child(self.label.clone())
            .on_click(move |_, w, cx| {
                selection.update(cx, |ed, cx| ed.focus(w, cx));
                w.request_virtual_keyboard()
            });
        let content = self.editor.clone();
        let mut shell = div()
            .relative()
            .flex()
            .items_center()
            .gap(px(8.))
            .w_full()
            .min_w_0()
            .h(px(self.height))
            .flex_none()
            .rounded(r.md)
            .border_1()
            .bg(p.panel)
            .border_color(if !self.error.is_empty() {
                p.danger
            } else if focused {
                p.ink
            } else {
                p.line
            })
            .when(!disabled && !focused && self.error.is_empty(), |e| {
                e.hover(move |s| s.border_color(p.faint))
            })
            .px(px(12.))
            .when(multi, |e| e.py(px(10.)))
            .on_mouse_down(MouseButton::Left, move |_, w, cx| {
                if !content.read(cx).disabled {
                    content.update(cx, |e, cx| e.focus(w, cx));
                    w.request_virtual_keyboard()
                }
            });
        if search {
            shell = shell.child(icon(Glyph::Search, 16., p.secondary))
        }
        shell = shell.child(div().flex_1().min_w_0().h_full().child(self.editor.clone()));
        if secret || search {
            let name = if search {
                l.text("Clear search", "清除搜索")
            } else if revealed {
                l.text("Hide password", "隐藏密码")
            } else {
                l.text("Show password", "显示密码")
            };
            let mut slot = div().relative().size(px(30.)).flex_none();
            if secret || has_value || slide > 0.01 {
                let action = face("trailing", name, 30., r.md.into(), Tone::Clear, cx)
                    .tab_stop(!disabled && (secret || has_value))
                    .w_full()
                    .hover(move |s| s.bg(p.hover))
                    .when(secret, |e| e.aria_toggled(revealed.into()))
                    .opacity(if secret { 1. } else { slide })
                    .child(if secret {
                        div()
                            .relative()
                            .size(px(18.))
                            .child(icon(Glyph::Eye, 18., p.secondary))
                            .child(div().absolute().inset_0().child(transformed(
                                Glyph::Slash,
                                18.,
                                p.secondary,
                                0.,
                                1.,
                                (0., 0.),
                                slide,
                            )))
                            .into_any_element()
                    } else {
                        transformed(
                            Glyph::Close,
                            16.,
                            p.secondary,
                            0.,
                            0.8 + 0.2 * slide,
                            (0., 0.),
                            1.,
                        )
                        .into_any_element()
                    })
                    .on_mouse_down(MouseButton::Left, |_, w, _| w.prevent_default())
                    .on_click(cx.listener(move |s, _, w, cx| {
                        if disabled {
                            return;
                        }
                        if secret {
                            s.editor.update(cx, |ed, cx| {
                                ed.revealed = !ed.revealed;
                                cx.notify()
                            });
                            cx.emit(FieldEvent::Visibility(!revealed));
                            cx.notify()
                        } else {
                            s.clear(w, cx)
                        }
                    }));
                slot = slot.child(action)
            }
            shell = shell.child(slot)
        }
        if multi && !disabled {
            let handle = div()
                .id("resize")
                .aria_label(l.text("Resize height", "调整高度"))
                .absolute()
                .bottom_0()
                .right_0()
                .size(px(20.))
                .cursor(CursorStyle::ResizeUpDown)
                .text_color(if self.resize.is_some() {
                    p.secondary
                } else {
                    p.faint
                })
                .hover(move |s| s.text_color(p.secondary))
                .child(resize_grip())
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|s, e: &MouseDownEvent, w, cx| {
                        s.resize = Some((e.position.y, s.height));
                        w.prevent_default();
                        cx.notify();
                        cx.stop_propagation()
                    }),
                );
            shell = shell.child(handle)
        }
        let has_note = !self.description.is_empty() || !self.error.is_empty();
        self.note_height.set(if has_note { 1. } else { 0. }, cx);
        let show = self.note_height.get(w, cx).clamp(0., 1.);
        let age = self.note_clock.time(0.18, w, cx);
        let note_alpha = if age < 0.18 {
            0.55 + 0.45 * ease(age / 0.18)
        } else {
            1.
        };
        let error_reveal = self.error_height.get(w, cx).clamp(0., 1.);
        let error_lines = if error_reveal > 0. && !self.error_copy.is_empty() {
            let run = w.text_style().to_run(self.error_copy.len());
            w.text_system()
                .shape_text(
                    self.error_copy.clone(),
                    px(13.),
                    &[run],
                    Some(self.width.get().max(px(64.))),
                    None,
                )
                .map_or(1, |lines| {
                    lines
                        .iter()
                        .map(|row| 1 + row.wrap_boundaries().len())
                        .sum()
                })
        } else {
            0
        };
        let desc = if self.error.is_empty() {
            self.description.clone()
        } else {
            format!("{} {}", self.description, self.error)
                .trim()
                .to_owned()
                .into()
        };
        if self.editor.read(cx).description != desc {
            self.editor.update(cx, |ed, _| ed.description = desc)
        }
        div()
            .id(self.id.clone())
            .flex()
            .flex_col()
            .min_w_0()
            .w_full()
            .text_color(p.ink)
            .when(disabled, |e| e.opacity(0.55))
            .child(label)
            .child(
                div()
                    .mt(px(7.))
                    .on_children_prepainted({
                        let width = self.width.clone();
                        move |b, _, _| {
                            if let Some(b) = b.first() {
                                width.set(b.size.width)
                            }
                        }
                    })
                    .child(shell),
            )
            .when(show > 0.001, |e| {
                e.child(
                    div()
                        .mt(px(8. * show))
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .opacity(show * note_alpha)
                        .text_size(px(13.))
                        .text_color(p.secondary)
                        .when(!self.description.is_empty(), |e| {
                            e.child(self.description.clone())
                        })
                        .when(error_reveal > 0.001, |e| {
                            e.child(
                                div()
                                    .id("error")
                                    .when(!self.error.is_empty(), |e| e.role(Role::Alert))
                                    .h(px(error_lines as f32 * 19. * error_reveal))
                                    .overflow_hidden()
                                    .opacity(error_reveal)
                                    .line_height(px(19.))
                                    .text_color(p.danger)
                                    .child(
                                        div()
                                            .relative()
                                            .top(px(3. * (1. - error_reveal)))
                                            .child(self.error_copy.clone()),
                                    ),
                            )
                        }),
                )
            })
            .when(self.resize.is_some(), |e| {
                let moved = cx.listener(|s, ev: &MouseMoveEvent, w, cx| {
                    if let Some((start, height)) = s.resize {
                        if ev.pressed_button == Some(MouseButton::Left) {
                            let max =
                                (f32::from(w.viewport_size().height) - 100.).clamp(110., 640.);
                            s.height = (height + f32::from(ev.position.y - start)).clamp(110., max);
                        } else {
                            s.resize = None
                        }
                        cx.notify()
                    }
                });
                let end = cx.listener(|s, _: &MouseUpEvent, _, cx| {
                    s.resize = None;
                    cx.notify()
                });
                e.child(
                    canvas(
                        |_, _, _| (),
                        move |_, _, w, _| {
                            w.on_mouse_event(move |ev: &MouseMoveEvent, phase, w, cx| {
                                if phase.capture() {
                                    moved(ev, w, cx)
                                }
                            });
                            w.on_mouse_event(move |ev: &MouseUpEvent, phase, w, cx| {
                                if phase.capture() && ev.button == MouseButton::Left {
                                    end(ev, w, cx)
                                }
                            });
                        },
                    )
                    .size(px(0.))
                    .absolute(),
                )
            })
    }
}
