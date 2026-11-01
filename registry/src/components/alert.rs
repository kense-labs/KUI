//! Persistent feedback with an optional recovery action.
use crate::primitives::{
    family::{Paint, PhaseClock, ease},
    glyph::{Glyph, transformed},
    presentation::*,
};
use gpui::{prelude::*, *};
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Info,
    Success,
    Warning,
    Error,
}
pub struct Alert {
    title: ChangingText,
    body: ChangingText,
    previous_tone: Severity,
    severity: Severity,
    action: Option<SharedString>,
    clock: PhaseClock,
}
pub struct AlertAction;
impl EventEmitter<AlertAction> for Alert {}
impl Alert {
    pub fn new(
        title: impl Into<SharedString>,
        body: impl Into<SharedString>,
        severity: Severity,
    ) -> Self {
        Self {
            title: ChangingText::new(title),
            body: ChangingText::new(body),
            previous_tone: severity,
            severity,
            action: None,
            clock: Default::default(),
        }
    }
    pub fn with_action(mut self, label: impl Into<SharedString>) -> Self {
        self.action = Some(label.into());
        self
    }
    pub fn replace(&mut self, next: Self, cx: &mut Context<Self>) {
        self.title.set(next.title.current, 1., cx);
        self.body.set(next.body.current, 1., cx);
        self.previous_tone = self.severity;
        self.severity = next.severity;
        self.action = next.action;
        self.clock.restart(cx);
        cx.notify()
    }
}
impl Render for Alert {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let color = |severity| -> Hsla {
            match severity {
                Severity::Success => p.success,
                Severity::Error => p.danger,
                Severity::Warning => rgb(0xc49343).into(),
                Severity::Info => p.secondary,
            }
        };
        column()
            .max_w_full()
            .gap_3()
            .items_start()
            .child(
                frame(520., p)
                    .id("alert-alert")
                    .role(Role::Alert)
                    .flex_row()
                    .items_start()
                    .gap_3()
                    .child({
                        let glyph = |s| {
                            if matches!(s, Severity::Success) {
                                Glyph::Check
                            } else {
                                Glyph::Alert
                            }
                        };
                        let age = self.clock.time(0.32, w, cx);
                        let a = if std::mem::discriminant(&self.severity)
                            == std::mem::discriminant(&self.previous_tone)
                        {
                            1.
                        } else {
                            ease(age / 0.26)
                        };
                        let leave = 1. - ease(age / 0.16);
                        div()
                            .relative()
                            .size(px(18.))
                            .child(transformed(
                                glyph(self.severity),
                                18.,
                                color(self.severity).opacity(a),
                                0.,
                                0.6 + 0.4 * a,
                                (0., 0.),
                                1.,
                            ))
                            .when(a < 1. && leave > 0., |e| {
                                e.child(div().absolute().inset_0().child(transformed(
                                    glyph(self.previous_tone),
                                    18.,
                                    color(self.previous_tone).opacity(leave),
                                    0.,
                                    0.6 + 0.4 * leave,
                                    (0., 0.),
                                    1.,
                                )))
                            })
                    })
                    .child(
                        column()
                            .gap_1()
                            .child(
                                self.title
                                    .render(15., w, cx)
                                    .text_size(px(15.))
                                    .font_weight(FontWeight::SEMIBOLD),
                            )
                            .child(
                                self.body
                                    .render(12., w, cx)
                                    .text_size(px(12.))
                                    .text_color(p.secondary),
                            ),
                    ),
            )
            .when_some(self.action.clone(), |el, a| {
                el.child(command("alert-action", a, cx, |_, _, cx| {
                    cx.emit(AlertAction)
                }))
            })
    }
}
