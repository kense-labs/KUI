//! An icon that interpolates its strokes when toggled.
use crate::foundation::{Theme, ThemeMode};
use crate::primitives::{
    control::activate,
    family::{Paint, PhaseClock, Slide, Tone, disabled_motion, face, standard_ease},
    glyph::{Glyph, morph_mark},
};
use gpui::{prelude::*, *};
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct MorphChanged(pub bool);
const TILE_FADE_SECONDS: f32 = 0.16;
/// `IconMorph` component.
///
/// # Examples
///
/// ```no_run
/// # #[allow(unused_variables, unused_imports)]
/// # fn example(window: &mut gpui::Window, cx: &mut gpui::App) {
/// # use gpui::prelude::*;
/// use gpui::prelude::*;
/// use kui::components::icon_morph::IconMorph;
/// use kui::primitives::glyph::Glyph;
///
/// let control = cx.new(|_| IconMorph::new("player", [Glyph::Play, Glyph::Pause], ["Play", "Pause"]).with_captions(["Paused", "Playing"]));
/// # }
/// ```
pub struct IconMorph {
    id: ElementId,
    pair: [Glyph; 2],
    labels: [SharedString; 2],
    captions: Option<[SharedString; 2]>,
    on: bool,
    hovered: bool,
    tint_from: f32,
    slide: Slide,
    fade: PhaseClock,
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
            fade: Default::default(),
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
            self.tint_from = if on { 1. } else { 0. };
            self.fade = Default::default();
            self.moment = Default::default();
        } else {
            self.tint_from = self.tile_tint(self.fade.age(cx));
            self.fade.restart(cx);
            if (self.pair[0] == Glyph::Heart && on) || (self.pair[0] == Glyph::Bell && !on) {
                self.moment.restart(cx);
            } else {
                self.moment = Default::default();
            }
        }
        self.on = on;
        self.slide.set(if on { 1. } else { 0. }, cx);
        cx.notify()
    }
}
impl Render for IconMorph {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let dark = Theme::global(cx).mode == ThemeMode::Dark;
        let p = Paint::get(cx);
        let i = usize::from(self.on);
        let t = self.slide.get(w, cx);
        let duration = if self.pair[0] == Glyph::Bell {
            0.95
        } else {
            0.6
        };
        let age = self.moment.time(duration, w, cx);
        let fade_age = self.fade.time(TILE_FADE_SECONDS, w, cx);
        let active_tint = if self.pair[0] == Glyph::Bell {
            0.
        } else {
            self.tile_tint(fade_age)
        };
        let elev = if self.hovered {
            active_tint.max(0.65)
        } else {
            active_tint
        };
        let base_surface: Hsla = if dark { rgb(0x1e1e22).into() } else { p.panel };
        let elev_overlay = if dark {
            hsla(0., 0., 1., 0.08 * elev)
        } else {
            hsla(0., 0., 0., 0.04 * elev)
        };
        let surface = base_surface.blend(elev_overlay);
        let base_border = if dark { hsla(0., 0., 1., 0.09) } else { p.line };
        let active_border = if dark {
            hsla(0., 0., 1., 0.22)
        } else {
            p.ink.opacity(0.18)
        };
        let border = base_border.blend(active_border.opacity(elev));
        let ink = p.ink;
        let caption = if self.hovered || (self.on && self.pair[0] != Glyph::Bell) {
            p.ink
        } else {
            p.secondary
        };
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
