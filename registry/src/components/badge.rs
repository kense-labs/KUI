//! Small semantic labels, optional status mark and animated content changes.
use crate::primitives::{
    family::{Paint, PhaseClock, ease},
    glyph::{Glyph, icon},
    presentation::*,
};
use gpui::{prelude::*, *};
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BadgeTone {
    Neutral,
    Info,
    Success,
    Warning,
    Danger,
}
pub struct Badge {
    label: ChangingText,
    prior_tone: BadgeTone,
    tone: BadgeTone,
    glyph: Option<Glyph>,
    clock: PhaseClock,
}
impl Badge {
    pub fn new(label: impl Into<SharedString>, tone: BadgeTone) -> Self {
        Self {
            label: ChangingText::new(label),
            prior_tone: tone,
            tone,
            glyph: None,
            clock: Default::default(),
        }
    }
    pub fn with_icon(mut self, glyph: Glyph) -> Self {
        self.glyph = Some(glyph);
        self
    }
    pub fn set(&mut self, label: impl Into<SharedString>, tone: BadgeTone, cx: &mut Context<Self>) {
        self.label.set(label, 1., cx);
        self.prior_tone = self.tone;
        self.tone = tone;
        self.clock.restart(cx);
        cx.notify()
    }
}
impl Render for Badge {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let color = |tone| -> Hsla {
            match tone {
                BadgeTone::Neutral => p.secondary,
                BadgeTone::Info => rgb(0x658fc9).into(),
                BadgeTone::Success => p.success,
                BadgeTone::Warning => rgb(0xbe9145).into(),
                BadgeTone::Danger => p.danger,
            }
        };
        let f = ease(self.clock.time(0.24, w, cx) / 0.24);
        let c = color(self.prior_tone).blend(color(self.tone).opacity(f));
        row()
            .id("badge-status")
            .role(Role::Status)
            .gap_1()
            .px_2()
            .h(px(24.))
            .rounded(crate::foundation::surface_radius(cx))
            .bg(p.panel.blend(c.opacity(0.12)))
            .text_color(c)
            .text_size(px(12.))
            .font_weight(FontWeight::MEDIUM)
            .when_some(self.glyph, |el, g| el.child(icon(g, 14., c)))
            .child(self.label.render(12., w, cx))
    }
}
