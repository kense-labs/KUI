//! A tri-state checkbox with a persistent input label.
use crate::primitives::{
    family::{Paint, Slide, Tone, face},
    form_chrome::*,
};
use gpui::{prelude::*, *};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckValue {
    Off,
    On,
    Mixed,
}
pub struct Checkbox {
    label: SharedString,
    description: SharedString,
    value: CheckValue,
    disabled: bool,
    tint: Slide,
    mixed: Slide,
}
impl EventEmitter<CheckValue> for Checkbox {}
impl Checkbox {
    pub fn new(label: impl Into<SharedString>, value: CheckValue) -> Self {
        Self {
            label: label.into(),
            description: "".into(),
            value,
            disabled: false,
            mixed: Slide::snappy((value == CheckValue::Mixed) as u8 as f32),
            tint: Slide::snappy(if value == CheckValue::Off { 0. } else { 1. }),
        }
    }
    pub fn with_description(mut self, t: impl Into<SharedString>) -> Self {
        self.description = t.into();
        self
    }
    pub fn with_disabled(mut self, v: bool) -> Self {
        self.disabled = v;
        self
    }
    pub fn value(&self) -> CheckValue {
        self.value
    }
    pub fn set_value(&mut self, v: CheckValue, cx: &mut Context<Self>) {
        self.value = v;
        self.tint
            .duration(if v == CheckValue::Off { 0. } else { 1. }, 0.26, cx);
        self.mixed
            .duration((v == CheckValue::Mixed) as u8 as f32, 0.26, cx);
        cx.notify()
    }
}
impl Render for Checkbox {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let t = self.tint.get(w, cx).clamp(0., 1.);
        let bx = div()
            .size(px(22.))
            .rounded(crate::foundation::surface_radius(cx))
            .flex()
            .items_center()
            .justify_center()
            .border_1()
            .border_color(p.faint.blend(p.bright.opacity(t)))
            .bg(p.panel)
            .child(
                div()
                    .relative()
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(px(20. * (0.6 + 0.4 * t)))
                    .rounded(crate::foundation::surface_radius(cx))
                    .bg(p.bright.opacity(t))
                    .child(check_stroke(
                        t,
                        self.mixed.get(w, cx),
                        16.,
                        p.inverse.opacity(t),
                    )),
            );
        let e = face(
            "checkbox",
            self.label.clone(),
            48.,
            crate::foundation::surface_radius(cx).into(),
            Tone::Clear,
            cx,
        )
        .role(Role::CheckBox)
        .aria_toggled(match self.value {
            CheckValue::On => true.into(),
            CheckValue::Off => false.into(),
            CheckValue::Mixed => Toggled::Mixed,
        })
        .gap(px(12.))
        .child(bx)
        .child(
            col()
                .gap(px(2.))
                .child(self.label.clone())
                .when(!self.description.is_empty(), |d| {
                    d.child(hint(self.description.clone(), cx))
                }),
        );
        if self.disabled {
            e.tab_stop(false).opacity(0.45)
        } else {
            e.on_click(cx.listener(|s, _, _, cx| {
                s.set_value(
                    if s.value == CheckValue::On {
                        CheckValue::Off
                    } else {
                        CheckValue::On
                    },
                    cx,
                );
                cx.emit(s.value)
            }))
        }
    }
}
