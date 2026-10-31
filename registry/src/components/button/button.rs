use std::rc::Rc;

use super::style::{chrome, chrome_with_radius, disabled_chrome};
use crate::foundation::{ColorTokens, Theme};
use crate::primitives::{
    Size,
    family::{PhaseClock, Slide, disabled_motion, lettering, measure},
    glyph::{Glyph, icon, spinner, transformed},
};
use gpui::{prelude::*, *};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ButtonVariant {
    #[default]
    Primary,
    Secondary,
    Ghost,
    Danger,
}

type ClickHandler = dyn Fn(&ClickEvent, &mut Window, &mut App);

/// Finished KUI button with clear control rhythm and theme-aware chrome.
pub struct Button {
    id: ElementId,
    label: SharedString,
    variant: ButtonVariant,
    size: Size,
    glyph: Option<Glyph>,
    trailing: bool,
    only: bool,
    disabled: bool,
    loading: bool,
    on_click: Option<Rc<ClickHandler>>,
}

impl Button {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            label: SharedString::from(""),
            variant: ButtonVariant::Primary,
            size: Size::Md,
            glyph: None,
            trailing: false,
            only: false,
            disabled: false,
            loading: false,
            on_click: None,
        }
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = label.into();
        self
    }

    pub fn primary(mut self) -> Self {
        self.variant = ButtonVariant::Primary;
        self
    }

    pub fn secondary(mut self) -> Self {
        self.variant = ButtonVariant::Secondary;
        self
    }

    pub fn ghost(mut self) -> Self {
        self.variant = ButtonVariant::Ghost;
        self
    }

    pub fn danger(mut self) -> Self {
        self.variant = ButtonVariant::Danger;
        self
    }

    pub fn with_size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    pub fn on_click(
        mut self,
        listener: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(listener));
        self
    }

    /// Draw an icon before or after the label.
    pub fn with_icon(mut self, icon: Glyph, trailing: bool) -> Self {
        self.glyph = Some(icon);
        self.trailing = trailing;
        self
    }
    /// Keep the label as an accessible name while showing only the icon.
    pub fn icon_only(mut self, icon: Glyph) -> Self {
        self.glyph = Some(icon);
        self.only = true;
        self
    }
    pub fn into_themed(self, cx: &App) -> Stateful<Div> {
        let colors = Theme::global(cx).tokens.colors.clone();
        self.shell(colors)
    }
    fn shell(self, colors: ColorTokens) -> Stateful<Div> {
        div().id(self.id.clone()).flex_none().child(Visual {
            button: self,
            colors,
        })
    }
}
impl IntoElement for Button {
    type Element = Stateful<Div>;
    fn into_element(self) -> Self::Element {
        div().id(self.id.clone()).flex_none().child(Auto(self))
    }
}
#[derive(IntoElement)]
struct Auto(Button);
impl RenderOnce for Auto {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        Visual {
            button: self.0,
            colors: cx
                .try_global::<Theme>()
                .map_or_else(ColorTokens::light, |t| t.tokens.colors.clone()),
        }
    }
}
#[derive(IntoElement)]
struct Visual {
    button: Button,
    colors: ColorTokens,
}
struct Interaction {
    width: Slide,
    press: Slide,
    hover: Slide,
    label: SharedString,
    old: SharedString,
    mark: Option<Glyph>,
    old_mark: Option<Glyph>,
    clock: PhaseClock,
    loading: bool,
    spinner: PhaseClock,
}
impl RenderOnce for Visual {
    fn render(self, w: &mut Window, cx: &mut App) -> impl IntoElement {
        let b = self.button;
        let inert = b.disabled || b.loading;
        let mut colors = self.colors.clone();
        if let Some(t) = cx.try_global::<Theme>() {
            let palette = crate::primitives::family::Paint::get(cx);
            let defaults = if t.mode == crate::foundation::ThemeMode::Dark {
                ColorTokens::dark()
            } else {
                ColorTokens::light()
            };
            if colors.surface == defaults.surface {
                colors.surface = palette.panel
            }
            if colors.foreground == defaults.foreground {
                colors.foreground = palette.ink
            }
            if colors.accent == defaults.accent {
                colors.accent = palette.bright;
                colors.accent_foreground = palette.inverse
            }
            if colors.border == defaults.border {
                colors.border = palette.line
            }
        }
        let mut look = match cx.try_global::<Theme>() {
            Some(theme) => chrome_with_radius(b.variant, b.size, &colors, theme.tokens.radius.md),
            None => chrome(b.variant, b.size, &colors),
        };
        if b.disabled {
            look = disabled_chrome(b.variant, &self.colors, &look)
        }
        let name = b.label.clone();
        let label = if b.only { "".into() } else { b.label.clone() };
        let h = f32::from(look.min_h);
        let gap = if b.glyph.is_some() && !b.only { 8. } else { 0. };
        let target = if b.only {
            h
        } else {
            2. * (f32::from(look.pad_x) + 1.)
                + measure(&label, 14., w)
                + if b.glyph.is_some() { 16. + gap } else { 0. }
        };
        let st = w.use_state(cx, |_, _| Interaction {
            width: Slide::new(target),
            press: Slide::snappy(0.),
            hover: Slide::snappy(0.),
            label: label.clone(),
            old: label.clone(),
            mark: b.glyph,
            old_mark: b.glyph,
            clock: Default::default(),
            loading: false,
            spinner: Default::default(),
        });
        let (wide, down, hov, old, old_mark, time, turn) = st.update(cx, |v, cx| {
            v.width.set(target, cx);
            if v.label != label || v.mark != b.glyph {
                v.old = std::mem::replace(&mut v.label, label.clone());
                v.old_mark = std::mem::replace(&mut v.mark, b.glyph);
                v.clock.restart(cx)
            }
            if v.loading != b.loading {
                v.loading = b.loading;
                v.spinner.restart(cx)
            }
            if inert {
                v.press.reset(0.);
                v.hover.reset(0.)
            }
            (
                v.width.get(w, cx),
                v.press.get(w, cx),
                v.hover.get(w, cx),
                v.old.clone(),
                v.old_mark,
                v.clock.time(0.3, w, cx),
                if b.loading && !disabled_motion(cx) {
                    w.request_animation_frame();
                    v.spinner.age(cx) / 0.8
                } else {
                    0.
                },
            )
        });
        let scale = 1.
            - down
                * if b.only {
                    0.04
                } else if wide > 220. {
                    0.015
                } else {
                    0.03
                };
        let tw = (wide
            - 2. * (f32::from(look.pad_x) + 1.)
            - if b.glyph.is_some() { 16. + gap } else { 0. })
        .max(0.)
            * scale;
        let fg = look.fg;
        let glyph_ink = if b.glyph == Some(Glyph::Check) {
            self.colors.success
        } else {
            fg
        };
        let bg = look.bg.blend(look.fg.opacity(0.065 * hov.clamp(0., 1.)));
        let mut face = div()
            .flex_none()
            .w(px(wide * scale))
            .h(px(h * scale))
            .rounded(look.radius * scale)
            .border_1()
            .border_color(look.border)
            .bg(bg)
            .text_color(fg)
            .flex()
            .items_center()
            .justify_center()
            .gap(px(gap * scale));
        if b.loading {
            face = face.child(spinner(turn, 16., fg))
        } else {
            let glyph = |g| {
                let slot = div().relative().size(px(16. * scale));
                if old_mark == Some(g) || time >= 0.22 {
                    return slot.child(icon(g, 16. * scale, glyph_ink));
                }
                slot.when_some(old_mark, |e, m| {
                    e.child(
                        div()
                            .absolute()
                            .inset_0()
                            .opacity(1. - crate::primitives::family::ease(time / 0.1))
                            .child(icon(m, 16. * scale, fg)),
                    )
                })
                .child(transformed(
                    g,
                    16. * scale,
                    glyph_ink.opacity(crate::primitives::family::ease(time / 0.16)),
                    0.,
                    1.,
                    (0., 0.),
                    if g == Glyph::Check {
                        crate::primitives::family::ease(time / 0.22)
                    } else {
                        1.
                    },
                ))
            };
            if !b.trailing
                && let Some(g) = b.glyph
            {
                face = face.child(glyph(g))
            }
            if !b.only {
                face = face.child(lettering(old, label, time, false, 14. * scale, fg, tw))
            }
            if b.trailing
                && let Some(g) = b.glyph
            {
                face = face.child(glyph(g))
            }
        }
        let pointer_press = |st: Entity<Interaction>| {
            move |_: &MouseDownEvent, _: &mut Window, cx: &mut App| {
                st.update(cx, |s, cx| {
                    s.press.set(1., cx);
                    cx.notify()
                })
            }
        };
        let ring = self.colors.focus_outline;
        let mut hit = div()
            .id("hit")
            .debug_selector({
                let n = name.clone();
                move || n.to_string()
            })
            .role(Role::Button)
            .aria_label(name)
            .tab_index(0)
            .tab_stop(!b.disabled)
            .w(px(wide))
            .h(px(h))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .rounded(look.radius)
            .focus_visible(move |s| {
                s.shadow(vec![BoxShadow {
                    color: ring,
                    offset: point(px(0.), px(0.)),
                    blur_radius: px(0.),
                    spread_radius: px(2.),
                    inset: false,
                }])
            })
            .child(face);
        if b.disabled {
            return hit
                .opacity(if b.variant == ButtonVariant::Primary {
                    0.52
                } else {
                    1.
                })
                .cursor_default();
        }
        if b.loading {
            return hit.aria_description("Loading").cursor_default();
        }
        hit = hit
            .cursor_pointer()
            .on_hover({
                let st = st.clone();
                move |inside, w, cx| {
                    st.update(cx, |s, cx| {
                        s.hover.set(if *inside { 1. } else { 0. }, cx);
                        if !*inside {
                            s.press.set(0., cx)
                        }
                        cx.notify()
                    });
                    w.refresh()
                }
            })
            .on_mouse_down(MouseButton::Left, pointer_press(st.clone()))
            .on_mouse_up(MouseButton::Left, {
                let st = st.clone();
                move |_, _, cx| {
                    st.update(cx, |s, cx| {
                        s.press.set(0., cx);
                        cx.notify()
                    })
                }
            })
            .on_mouse_up_out(MouseButton::Left, {
                let st = st.clone();
                move |_, _, cx| {
                    st.update(cx, |s, cx| {
                        s.press.set(0., cx);
                        cx.notify()
                    })
                }
            })
            .on_key_down({
                let st = st.clone();
                move |e, _, cx| {
                    if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                        st.update(cx, |s, cx| {
                            s.press.set(1., cx);
                            cx.notify()
                        })
                    }
                }
            })
            .on_key_up(move |_, _, cx| {
                st.update(cx, |s, cx| {
                    s.press.set(0., cx);
                    cx.notify()
                })
            });
        if let Some(f) = b.on_click {
            hit = hit.on_click(move |e, w, cx| f(e, w, cx))
        }
        hit
    }
}
