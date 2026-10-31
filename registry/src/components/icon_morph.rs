//! An icon that interpolates its strokes when toggled.
use crate::foundation::Theme;
use crate::primitives::{
    control::activate,
    family::{Paint, PhaseClock, Slide, Tone, disabled_motion, face, standard_ease},
    glyph::{Glyph, morph_mark},
};
use gpui::{prelude::*, *};
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct MorphChanged(pub bool);
const TILE_FADE_SECONDS: f32 = 0.16;
pub struct IconMorph {
    id: ElementId,
    pair: [Glyph; 2],
    labels: [SharedString; 2],
    captions: Option<[SharedString; 2]>,
    on: bool,
    hovered: bool,
    tint_from: f32,
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
            hovered: false,
            tint_from: 0.,
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
    fn tile_tint(&self, elapsed: f32) -> f32 {
        let target = if self.on { 1. } else { 0. };
        self.tint_from + (target - self.tint_from) * standard_ease(elapsed / TILE_FADE_SECONDS)
    }
    pub fn set(&mut self, on: bool, cx: &mut Context<Self>) {
        if on == self.on {
            return;
        }
        if disabled_motion(cx) {
            self.moment = Default::default()
        } else if self.pair == [Glyph::Sun, Glyph::Moon] {
            self.tint_from = self.tile_tint(self.moment.age(cx));
            self.moment.restart(cx)
        } else if (self.pair[0] == Glyph::Heart && on) || (self.pair[0] == Glyph::Bell && !on) {
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
        let theme_tile = self.pair == [Glyph::Sun, Glyph::Moon];
        let duration = if theme_tile {
            TILE_FADE_SECONDS
        } else if self.pair[0] == Glyph::Bell {
            0.95
        } else {
            0.6
        };
        let age = self.moment.time(duration, w, cx);
        let highlighted = self.hovered || (self.on && !theme_tile && self.pair[0] != Glyph::Bell);
        let mut surface = if highlighted { p.stage } else { p.panel };
        let mut border = if highlighted {
            p.ink.opacity(0.20)
        } else {
            p.line
        };
        let mut ink = p.ink;
        let mut caption = p.secondary;
        if theme_tile {
            let tint = self.tile_tint(age);
            border = surface.blend(border).blend(p.bright.opacity(tint));
            surface = surface.blend(p.bright.opacity(tint));
            caption =
                (if self.hovered { p.ink } else { p.secondary }).blend(p.inverse.opacity(tint));
            ink = caption;
        }
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
        .bg(surface)
        .border_color(border)
        .on_hover(cx.listener(|v, hovered: &bool, _, cx| {
            v.hovered = *hovered;
            cx.notify();
        }))
        .aria_selected(self.on)
        .child(morph_mark(
            self.pair,
            t,
            age,
            if self.captions.is_some() { 32. } else { 24. },
            ink,
            p.danger,
            surface,
        ));
        if let Some(c) = &self.captions {
            f = f.child(
                div()
                    .text_color(caption)
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
