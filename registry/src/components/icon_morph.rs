//! An icon that interpolates its strokes when toggled.
use crate::foundation::Theme;
use crate::primitives::{
    control::activate,
    family::{Paint, PhaseClock, Slide, Tone, face},
    glyph::{Glyph, morph_mark},
};
use gpui::{prelude::*, *};
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct MorphChanged(pub bool);
pub struct IconMorph {
    id: ElementId,
    pair: [Glyph; 2],
    labels: [SharedString; 2],
    captions: Option<[SharedString; 2]>,
    on: bool,
    slide: Slide,
    moment: PhaseClock,
    tile: f32,
    controlled: bool,
}
impl EventEmitter<MorphChanged> for IconMorph {}
impl IconMorph {
    pub fn new<S: Into<SharedString>>(
        id: impl Into<ElementId>,
        pair: [Glyph; 2],
        labels: [S; 2],
    ) -> Self {
        Self {
            id: id.into(),
            pair,
            labels: labels.map(Into::into),
            captions: None,
            on: false,
            slide: Slide::new(0.),
            moment: Default::default(),
            tile: 44.,
            controlled: false,
        }
    }
    pub fn with_captions<S: Into<SharedString>>(mut self, captions: [S; 2]) -> Self {
        self.captions = Some(captions.map(Into::into));
        self.tile = 124.5;
        self
    }
    /// In controlled mode activation requests a value; the host acknowledges with set.
    pub fn controlled(mut self, enabled: bool) -> Self {
        self.controlled = enabled;
        self
    }
    pub fn set(&mut self, on: bool, cx: &mut Context<Self>) {
        if on == self.on {
            return;
        }
        if (self.pair[0] == Glyph::Heart && on) || (self.pair[0] == Glyph::Bell && !on) {
            self.moment.restart(cx)
        } else {
            self.moment = Default::default()
        }
        self.on = on;
        self.slide.set(if on { 1. } else { 0. }, cx);
        cx.notify()
    }
}
impl Render for IconMorph {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let i = usize::from(self.on);
        let t = self.slide.get(w, cx);
        let side = self
            .tile
            .min((f32::from(w.viewport_size().width) - 28.) / 2.);
        let mut f = face(
            self.id.clone(),
            self.labels[i].clone(),
            side,
            Theme::global(cx).tokens.radius.md.into(),
            Tone::Surface,
            cx,
        )
        .w(px(side))
        .flex_col()
        .gap(px(12.))
        .border_1()
        .border_color(p.line)
        .hover(|s| s.bg(p.stage).border_color(p.ink.opacity(0.20)))
        .when(self.pair[0] == Glyph::Bell && !self.on, |e| {
            e.border_1().border_color(p.line)
        })
        .aria_selected(self.on)
        .when(self.on && self.pair[0] != Glyph::Bell, |e| {
            e.bg(p.stage).border_color(p.ink.opacity(0.20))
        })
        .child(morph_mark(
            self.pair,
            t,
            self.moment.time(
                if self.pair[0] == Glyph::Bell {
                    0.95
                } else {
                    0.6
                },
                w,
                cx,
            ),
            if self.captions.is_some() { 32. } else { 24. },
            p.ink,
            p.danger,
            p.panel,
        ));
        if let Some(c) = &self.captions {
            f = f.child(
                div()
                    .text_color(p.secondary)
                    .text_size(px(12.))
                    .child(c[i].clone()),
            )
        }
        activate(f, cx, |v, _, cx| {
            let next = !v.on;
            if !v.controlled {
                v.set(next, cx)
            }
            cx.emit(MorphChanged(next))
        })
    }
}
