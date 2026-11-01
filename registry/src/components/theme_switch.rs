//! Request a theme change with the bounds of the trigger.
use crate::{
    foundation::{Theme, ThemeMode},
    primitives::{
        control::{UiLanguage, activate},
        family::{Paint, Slide, Tone, face, press},
        glyph::{Glyph, transformed},
    },
};
use gpui::{prelude::*, *};
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Transition {
    #[default]
    Reveal,
    Eclipse,
    Split,
    Rise,
}
#[derive(Clone, Copy, Debug)]
pub struct ThemeChange {
    pub next: ThemeMode,
    pub mode: Transition,
    pub trigger: Bounds<Pixels>,
}
/// Optional host intent while a reversible document transition is in progress.
pub struct ThemeIconIntent(pub ThemeMode);
impl Global for ThemeIconIntent {}

pub struct ThemeSwitch {
    id: ElementId,
    mode: Transition,
    progress: Option<Slide>,
    pressed: Slide,
}
impl EventEmitter<ThemeChange> for ThemeSwitch {}
impl ThemeSwitch {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            mode: Transition::Reveal,
            progress: None,
            pressed: Slide::snappy(0.),
        }
    }
    pub fn with_mode(mut self, m: Transition) -> Self {
        self.mode = m;
        self
    }
}
impl Render for ThemeSwitch {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut p = Paint::get(cx);
        let dark = Theme::global(cx).mode == ThemeMode::Dark;
        let next = cx
            .try_global::<ThemeIconIntent>()
            .map_or(dark, |i| i.0 == ThemeMode::Dark);
        let dest = f32::from(u8::from(next));
        let spring = self.progress.get_or_insert_with(|| Slide::snappy(dest));
        spring.set(dest, cx);
        let t = spring.get(w, cx).clamp(0., 1.);
        let light = Paint::for_mode(ThemeMode::Light);
        let night = Paint::for_mode(ThemeMode::Dark);
        p.panel = light.panel.blend(night.panel.opacity(t));
        p.ink = light.ink.blend(night.ink.opacity(t));
        p.line = light.line.blend(night.line.opacity(t));
        let depression = self.pressed.get(w, cx).clamp(0., 1.);
        let (dx, dy) = match self.mode {
            Transition::Reveal => (0., 0.),
            Transition::Split => (1., 0.),
            Transition::Eclipse => (-2., 0.),
            Transition::Rise => (0., -2.),
        };
        let lang = UiLanguage::current(cx);
        let radius = Theme::global(cx).tokens.radius.md;
        let bounds = std::rc::Rc::new(std::cell::Cell::new(Bounds::default()));
        let dest = bounds.clone();
        let f = face(
            self.id.clone(),
            if dark {
                lang.text("Switch to light mode", "切换至浅色")
            } else {
                lang.text("Switch to dark mode", "切换至深色")
            },
            36.,
            radius.into(),
            Tone::Clear,
            cx,
        )
        .relative()
        .px(px(12.))
        .gap(px(6.))
        .border_1()
        .border_color(transparent_black())
        .text_color(p.ink)
        .group("theme-key")
        .child(
            div()
                .absolute()
                .inset(px(depression))
                .rounded(radius)
                .border_1()
                .border_color(p.line)
                .bg(p.panel.blend(p.ink.opacity(depression * 0.07)))
                .group_hover("theme-key", |style| {
                    style.border_color(p.secondary.opacity(0.55))
                }),
        )
        .child(
            div()
                .relative()
                .w(px(20.))
                .h(px(20.))
                .child(div().absolute().inset(px(1.)).child(transformed(
                    Glyph::Sun,
                    18.,
                    p.ink.opacity(1. - t),
                    t * std::f32::consts::FRAC_PI_6,
                    1. - 0.3 * t,
                    (dx * t, dy * t),
                    1.,
                )))
                .child(div().absolute().inset(px(1.)).child(transformed(
                    Glyph::Moon,
                    18.,
                    p.ink.opacity(t),
                    -std::f32::consts::FRAC_PI_6 * (1. - t),
                    (0.7 + 0.3 * t)
                        * if self.mode == Transition::Reveal {
                            1. + 0.08 * t
                        } else {
                            1.
                        },
                    (dx * t, dy * t),
                    1.,
                ))),
        )
        .child(lang.text("Switch theme", "切换主题"));
        let f = press(f, cx, |v| &mut v.pressed);
        div()
            .on_children_prepainted(move |b, _, _| {
                if let Some(b) = b.first() {
                    dest.set(*b)
                }
            })
            .child(activate(f, cx, move |v, _, cx| {
                cx.emit(ThemeChange {
                    next: if dark {
                        ThemeMode::Light
                    } else {
                        ThemeMode::Dark
                    },
                    mode: v.mode,
                    trigger: bounds.get(),
                })
            }))
    }
}
