//! Shared visual tokens, spring-driven controls, and motion primitives for KUI Pro screens.

use crate::{
    foundation::{Theme, ThemeMode},
    primitives::{
        control::UiLanguage,
        family::{PhaseClock, Slide, disabled_motion},
        pro_icons::{
            AGENT_WAVE_MS, ProIcon, agent_pixel_cells, person_hue_index, person_initials,
            pro_app_artwork, pro_icon,
        },
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

/// Bilingual text selector matching the current [`UiLanguage`] global.
pub fn tr<'a>(cx: &App, en: &'a str, zh: &'a str) -> &'a str {
    UiLanguage::current(cx).text(en, zh)
}

/// Build a stable child [`ElementId`] scoped under `base`.
pub fn pro_eid(base: &ElementId, suffix: impl Into<SharedString>) -> ElementId {
    (base.clone(), suffix.into()).into()
}

/// Evaluate `cubic-bezier(0.22, 1.0, 0.36, 1.0)` on `t` in `0.0..=1.0`.
pub fn pro_ease(t: f32) -> f32 {
    let t = t.clamp(0., 1.);
    if t <= 0. || t >= 1. {
        return t;
    }
    let mut u = t;
    for _ in 0..6 {
        let inv = 1. - u;
        let x = 3. * inv * inv * u * 0.22 + 3. * inv * u * u * 0.36 + u * u * u;
        let dx = 3. * inv * inv * 0.22 + 6. * inv * u * (0.36 - 0.22) + 3. * u * u * (1. - 0.36);
        if dx.abs() < 1e-5 {
            break;
        }
        u = (u - (x - t) / dx).clamp(0., 1.);
    }
    let inv = 1. - u;
    3. * inv * inv * u + 3. * inv * u * u + u * u * u
}

/// Premultiplied RGBA color interpolation without HSL hue-wrap artifacts.
pub fn pro_mix_color(from: Hsla, to: Hsla, t: f32) -> Hsla {
    let t = t.clamp(0., 1.);
    if t <= 0.001 {
        return from;
    }
    if t >= 0.999 {
        return to;
    }
    let a = Rgba::from(from);
    let b = Rgba::from(to);
    let inv = 1. - t;
    let out_a = a.a * inv + b.a * t;
    if out_a <= 1e-5 {
        return transparent_black();
    }
    let r = (a.r * a.a * inv + b.r * b.a * t) / out_a;
    let g = (a.g * a.a * inv + b.g * b.a * t) / out_a;
    let bl = (a.b * a.a * inv + b.b * b.a * t) / out_a;
    Rgba {
        r: r.clamp(0., 1.),
        g: g.clamp(0., 1.),
        b: bl.clamp(0., 1.),
        a: out_a.clamp(0., 1.),
    }
    .into()
}

/// Semantic color and surface tokens aligned with the clean Pro screen palette.
#[derive(Clone, Copy, Debug)]
pub struct ProPalette {
    pub dark: bool,
    pub page: Hsla,
    pub side_tint: Hsla,
    pub panel: Hsla,
    pub card: Hsla,
    pub tray: Hsla,
    pub subtle: Hsla,
    pub chip: Hsla,
    pub chip_hover: Hsla,
    pub hover: Hsla,
    pub select: Hsla,
    pub line: Hsla,
    pub line_soft: Hsla,
    pub line_strong: Hsla,
    pub strong: Hsla,
    pub ink: Hsla,
    pub text: Hsla,
    pub muted: Hsla,
    pub faint: Hsla,
    pub primary: Hsla,
    pub on_primary: Hsla,
    pub focus: Hsla,
    pub accent_info: Hsla,
    pub accent_soft: Hsla,
    pub blue: Hsla,
    pub green: Hsla,
    pub green_ink: Hsla,
    pub green_soft: Hsla,
    pub amber: Hsla,
    pub amber_ink: Hsla,
    pub amber_soft: Hsla,
    pub orange: Hsla,
    pub red: Hsla,
    pub red_ink: Hsla,
    pub red_soft: Hsla,
    pub purple: Hsla,
    pub pink: Hsla,
    pub teal: Hsla,
    pub cyan: Hsla,
    pub logo_tile: Hsla,
}

impl ProPalette {
    /// Resolve palette colors for the active [`Theme`] mode and accent override.
    pub fn get(cx: &App) -> Self {
        let theme = Theme::global(cx);
        let dark = theme.mode == ThemeMode::Dark;
        let tint = theme.accent.for_mode(theme.mode);
        let mut p = if dark {
            Self {
                dark: true,
                page: rgb(0x0e0e10).into(),
                side_tint: rgb(0x111113).into(),
                panel: rgb(0x141416).into(),
                card: rgb(0x1b1b1e).into(),
                tray: rgb(0x1d1d20).into(),
                subtle: rgb(0x18181b).into(),
                chip: hsla(0., 0., 1., 0.06),
                chip_hover: hsla(0., 0., 1., 0.09),
                hover: hsla(0., 0., 1., 0.05),
                select: hsla(0., 0., 1., 0.08),
                line: hsla(0., 0., 1., 0.08),
                line_soft: hsla(0., 0., 1., 0.05),
                line_strong: hsla(0., 0., 1., 0.14),
                strong: rgb(0xffffff).into(),
                ink: hsla(0., 0., 1., 0.90),
                text: hsla(0., 0., 1., 0.62),
                muted: hsla(0., 0., 1., 0.45),
                faint: hsla(0., 0., 1., 0.28),
                primary: rgb(0xf4f4f5).into(),
                on_primary: rgb(0x111113).into(),
                focus: rgba(0x4d8dffcc).into(),
                accent_info: rgb(0x4fb0fe).into(),
                accent_soft: hsla(207. / 360., 0.99, 0.65, 0.16),
                blue: rgb(0x4fb0fe).into(),
                green: rgb(0x7dcf9f).into(),
                green_ink: rgb(0x7dcf9f).into(),
                green_soft: hsla(145. / 360., 0.46, 0.65, 0.14),
                amber: rgb(0xe6c24d).into(),
                amber_ink: rgb(0xffb35c).into(),
                amber_soft: hsla(46. / 360., 0.75, 0.60, 0.15),
                orange: rgb(0xf2a15c).into(),
                red: rgb(0xff6b5f).into(),
                red_ink: rgb(0xff8a7a).into(),
                red_soft: hsla(4. / 360., 1.0, 0.69, 0.14),
                purple: rgb(0xb98af0).into(),
                pink: rgb(0xf06db5).into(),
                teal: rgb(0x4fc3b4).into(),
                cyan: rgb(0x5cc6ee).into(),
                logo_tile: rgb(0xffffff).into(),
            }
        } else {
            Self {
                dark: false,
                page: rgb(0xf8f8f8).into(),
                side_tint: rgb(0xfcfcfc).into(),
                panel: rgb(0xffffff).into(),
                card: rgb(0xffffff).into(),
                tray: rgb(0xf4f4f4).into(),
                subtle: rgb(0xfafafa).into(),
                chip: hsla(0., 0., 0., 0.04),
                chip_hover: hsla(0., 0., 0., 0.065),
                hover: hsla(0., 0., 0., 0.032),
                select: hsla(0., 0., 0., 0.055),
                line: hsla(0., 0., 0., 0.07),
                line_soft: hsla(0., 0., 0., 0.045),
                line_strong: hsla(0., 0., 0., 0.12),
                strong: rgb(0x000000).into(),
                ink: hsla(0., 0., 0., 0.88),
                text: hsla(0., 0., 0., 0.61),
                muted: hsla(0., 0., 0., 0.45),
                faint: hsla(0., 0., 0., 0.28),
                primary: hsla(0., 0., 0., 0.88),
                on_primary: rgb(0xffffff).into(),
                focus: rgba(0x0060e2b3).into(),
                accent_info: rgb(0x0070d6).into(),
                accent_soft: hsla(209. / 360., 1.0, 0.44, 0.09),
                blue: rgb(0x0190fe).into(),
                green: rgb(0x5fc88a).into(),
                green_ink: rgb(0x1e8a4f).into(),
                green_soft: hsla(145. / 360., 0.48, 0.58, 0.14),
                amber: rgb(0xe8b000).into(),
                amber_ink: rgb(0xb25e00).into(),
                amber_soft: hsla(45. / 360., 1.0, 0.45, 0.13),
                orange: rgb(0xf28a2e).into(),
                red: rgb(0xf04438).into(),
                red_ink: rgb(0xc4320a).into(),
                red_soft: hsla(4. / 360., 0.86, 0.58, 0.10),
                purple: rgb(0xa651f0).into(),
                pink: rgb(0xee3e9e).into(),
                teal: rgb(0x13b3a0).into(),
                cyan: rgb(0x14b3f0).into(),
                logo_tile: rgb(0xffffff).into(),
            }
        };
        if !tint.is_transparent() {
            p.primary = p.primary.blend(tint);
            p.on_primary = theme.accent.on_fill(p.primary, p.on_primary);
            p.focus = theme.tokens.colors.focus_outline;
        }
        p
    }
}

/// Presence dot indicator for person or agent avatars.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProPresence {
    Online,
    Away,
    Busy,
}

/// Rounded square tile showing an app or provider mark, with neutral fallback.
pub fn pro_app_tile(app: &str, side: f32, cx: &App) -> AnyElement {
    let p = ProPalette::get(cx);
    let lower = app.trim().to_ascii_lowercase();
    let radius = (side * 0.25).clamp(5., 8.);
    if lower == "stripe"
        && let Some(mark) = pro_app_artwork("stripe", side, cx)
    {
        return div()
            .size(px(side))
            .flex_none()
            .rounded(px(radius))
            .overflow_hidden()
            .child(mark)
            .into_any_element();
    }
    let mark_side = (side * 0.62).round();
    let content = if let Some(mark) = pro_app_artwork(app, mark_side, cx) {
        mark
    } else {
        let initial = person_initials(app);
        div()
            .text_size(px((side * 0.40).max(9.)))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(p.ink)
            .child(if initial.is_empty() {
                "A".to_string()
            } else {
                initial.chars().take(1).collect()
            })
            .into_any_element()
    };
    let tile_bg: Hsla = if matches!(lower.as_str(), "github") && p.dark {
        rgb(0x242428).into()
    } else if matches!(
        lower.as_str(),
        "gmail" | "google calendar" | "calendar" | "google drive" | "drive" | "slack"
    ) {
        rgb(0xffffff).into()
    } else if p.dark {
        rgb(0x222226).into()
    } else {
        rgb(0xffffff).into()
    };
    div()
        .size(px(side))
        .flex_none()
        .rounded(px(radius))
        .bg(tile_bg)
        .border_1()
        .border_color(p.line)
        .flex()
        .items_center()
        .justify_center()
        .child(content)
        .into_any_element()
}

/// Render the 3x3 mirrored agent pixel mark with optional diagonal wave twinkle.
pub fn pro_agent_avatar(
    seed: &str,
    color: Hsla,
    side: f32,
    working: bool,
    elapsed_ms: f32,
    cx: &App,
) -> AnyElement {
    let p = ProPalette::get(cx);
    let reduced = Theme::global(cx).reduced_motion;
    let cells = agent_pixel_cells(seed);
    let sq = if side <= 20. {
        3.
    } else if side <= 26. {
        3.5
    } else if side <= 34. {
        4.
    } else {
        5.
    };
    let gap = if side <= 20. {
        1.
    } else if side <= 26. {
        1.5
    } else {
        2.
    };
    let radius = (side * 0.30).round();
    let tile_bg: Hsla = if p.dark {
        rgb(0x202024).into()
    } else {
        rgb(0xfafafb).into()
    };

    let mut grid = div().flex().flex_col().gap(px(gap));
    for r in 0..3usize {
        let mut row_el = div().flex().items_center().gap(px(gap));
        for c in 0..3usize {
            let i = r * 3 + c;
            let on = cells[i];
            let alpha = if working && !reduced {
                let phase = ((elapsed_ms + AGENT_WAVE_MS[i]) / 1200.) * std::f32::consts::TAU;
                let wave = 0.60 + 0.40 * phase.cos();
                if on { wave } else { 0.16 * wave }
            } else if on {
                1.0
            } else {
                0.0
            };
            row_el = row_el.child(
                div()
                    .size(px(sq))
                    .rounded(px((sq * 0.25).max(0.8)))
                    .bg(color.opacity(alpha)),
            );
        }
        grid = grid.child(row_el);
    }

    div()
        .size(px(side))
        .flex_none()
        .rounded(px(radius))
        .bg(tile_bg)
        .border_1()
        .border_color(p.line_strong)
        .flex()
        .items_center()
        .justify_center()
        .child(grid)
        .into_any_element()
}

/// Render a person or workspace avatar with deterministic soft hue and optional presence dot.
pub fn pro_person_avatar(
    name: &str,
    side: f32,
    square: bool,
    status: Option<ProPresence>,
    cx: &App,
) -> AnyElement {
    const HUES: [f32; 8] = [214., 152., 40., 22., 330., 268., 174., 196.];
    let p = ProPalette::get(cx);
    let h = HUES[person_hue_index(name)] / 360.;
    let (bg, fg) = if p.dark {
        (hsla(h, 0.28, 0.24, 1.0), hsla(h, 0.60, 0.80, 1.0))
    } else {
        (hsla(h, 0.70, 0.92, 1.0), hsla(h, 0.48, 0.36, 1.0))
    };
    let radius = if square {
        px((side * 0.30).round())
    } else {
        px(999.)
    };
    let fs = (side * 0.38).clamp(8.5, 14.);
    let initials = person_initials(name);
    let mut wrap = div().relative().size(px(side)).flex_none().child(
        div()
            .size_full()
            .rounded(radius)
            .bg(bg)
            .text_color(fg)
            .text_size(px(fs))
            .font_weight(FontWeight::SEMIBOLD)
            .flex()
            .items_center()
            .justify_center()
            .child(initials),
    );
    if let Some(st) = status {
        let dot_side = (side * 0.28).clamp(6., 10.);
        let dot_color = match st {
            ProPresence::Online => p.green,
            ProPresence::Away => p.amber,
            ProPresence::Busy => p.red,
        };
        wrap = wrap.child(
            div()
                .absolute()
                .right(px(if square { -1.5 } else { 0. }))
                .bottom(px(if square { -1.5 } else { 0. }))
                .size(px(dot_side))
                .rounded_full()
                .bg(dot_color)
                .border_1()
                .border_color(p.panel),
        );
    }
    wrap.into_any_element()
}

/// Button visual variant for Pro screen toolbars and inspectors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProBtnStyle {
    Primary,
    Secondary,
    Ghost,
    Danger,
}

type ProClickFn = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

struct ProBtnAnim {
    hover: Slide,
    press: Slide,
    current_label: SharedString,
    prev_label: Option<SharedString>,
    current_icon: Option<ProIcon>,
    swap_clock: PhaseClock,
}

#[derive(IntoElement)]
struct ProBtnVisual {
    state_id: ElementId,
    label: SharedString,
    lead_icon: Option<ProIcon>,
    style: ProBtnStyle,
    small: bool,
    on_click: ProClickFn,
}

impl RenderOnce for ProBtnVisual {
    fn render(self, w: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let h = if self.small { 28. } else { 32. };
        let pad_x = if self.small { 9. } else { 12. };
        let fs = if self.small { 12.5 } else { 13. };
        let icon_sz = if self.small { 13. } else { 14. };
        let (bg, hover_bg, fg, border) = match self.style {
            ProBtnStyle::Primary => (
                p.primary,
                pro_mix_color(p.primary, p.on_primary, 0.12),
                p.on_primary,
                transparent_black(),
            ),
            ProBtnStyle::Secondary => (p.card, p.chip_hover, p.ink, p.line_strong),
            ProBtnStyle::Ghost => (transparent_black(), p.hover, p.text, transparent_black()),
            ProBtnStyle::Danger => (
                p.red_soft,
                p.red.opacity(0.22),
                p.red_ink,
                p.red.opacity(0.28),
            ),
        };
        let label_init = self.label.clone();
        let icon_init = self.lead_icon;
        let st = w.use_keyed_state(self.state_id, cx, |_, _| ProBtnAnim {
            hover: Slide::snappy(0.),
            press: Slide::snappy(0.),
            current_label: label_init,
            prev_label: None,
            current_icon: icon_init,
            swap_clock: PhaseClock::default(),
        });
        let (hov, down, prev_lbl, swap_t) = st.update(cx, |s, cx| {
            if s.current_label != self.label || s.current_icon != self.lead_icon {
                let old = std::mem::replace(&mut s.current_label, self.label.clone());
                s.prev_label = Some(old);
                s.current_icon = self.lead_icon;
                s.swap_clock.restart(cx);
            }
            let age = s.swap_clock.time(0.18, w, cx);
            let t = (age / 0.18).clamp(0., 1.);
            if t >= 1. {
                s.prev_label = None;
            }
            (
                s.hover.get(w, cx).clamp(0., 1.),
                s.press.get(w, cx).clamp(0., 1.),
                s.prev_label.clone(),
                t,
            )
        });

        let face_bg = pro_mix_color(bg, hover_bg, hov);
        let fg_current = if self.style == ProBtnStyle::Ghost {
            pro_mix_color(fg, p.ink, hov * 0.85)
        } else {
            fg
        };
        let dark = p.dark;
        let style = self.style;
        let paint_face = canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let bw = f32::from(bounds.size.width);
                let bh = f32::from(bounds.size.height);
                if bw <= 1. || bh <= 1. {
                    return;
                }
                let ix = (bw * 0.022 * down).clamp(0., 2.2);
                let iy = (bh * 0.042 * down).clamp(0., 1.4);
                let rect = Bounds::new(
                    point(bounds.origin.x + px(ix), bounds.origin.y + px(iy)),
                    size(px((bw - 2. * ix).max(1.)), px((bh - 2. * iy).max(1.))),
                );
                let r = px(7.).min(rect.size.width / 2.).min(rect.size.height / 2.);
                if !dark
                    && hov > 0.02
                    && down < 0.7
                    && matches!(style, ProBtnStyle::Primary | ProBtnStyle::Secondary)
                {
                    let shadow_alpha = 0.045 * hov * (1. - down);
                    window.paint_quad(
                        fill(
                            Bounds::new(point(rect.origin.x, rect.origin.y + px(1.)), rect.size),
                            black().opacity(shadow_alpha),
                        )
                        .corner_radii(r),
                    );
                }
                if face_bg.a > 0.001 || border.a > 0.001 {
                    window.paint_quad(quad(
                        rect,
                        Corners::all(r),
                        face_bg,
                        Edges::all(px(if border.a > 0.001 { 1. } else { 0. })),
                        border,
                        BorderStyle::Solid,
                    ));
                }
            },
        )
        .absolute()
        .inset_0();

        let enter_ease = pro_ease(swap_t);
        let label_el = if let Some(old) = prev_lbl {
            let exit_ease = (1. - swap_t / 0.65).clamp(0., 1.);
            div()
                .relative()
                .flex()
                .items_center()
                .child(
                    div()
                        .relative()
                        .top(px(2.5 * (1. - enter_ease)))
                        .opacity(enter_ease)
                        .child(self.label.clone()),
                )
                .child(
                    div()
                        .absolute()
                        .left_0()
                        .top(px(-2.5 * (1. - exit_ease)))
                        .opacity(exit_ease)
                        .child(old),
                )
                .into_any_element()
        } else {
            div().child(self.label.clone()).into_any_element()
        };

        let on_click = self.on_click;
        div()
            .id("pro-btn-hit")
            .role(Role::Button)
            .aria_label(self.label)
            .tab_index(0)
            .flex_1()
            .h(px(h))
            .px(px(pad_x))
            .relative()
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(7.))
            .text_color(fg_current)
            .text_size(px(fs))
            .font_weight(FontWeight::MEDIUM)
            .whitespace_nowrap()
            .cursor_pointer()
            .focus_visible(move |s| s.border_1().border_color(p.focus))
            .on_hover({
                let st = st.clone();
                move |inside, w, cx| {
                    st.update(cx, |s, cx| {
                        s.hover.set(if *inside { 1. } else { 0. }, cx);
                        if !*inside {
                            s.press.set(0., cx);
                        }
                        cx.notify();
                    });
                    w.refresh();
                }
            })
            .on_mouse_down(MouseButton::Left, {
                let st = st.clone();
                move |_, w, cx| {
                    st.update(cx, |s, cx| {
                        s.press.set(1., cx);
                        cx.notify();
                    });
                    w.refresh();
                }
            })
            .on_mouse_up(MouseButton::Left, {
                let st = st.clone();
                move |_, w, cx| {
                    st.update(cx, |s, cx| {
                        s.press.set(0., cx);
                        cx.notify();
                    });
                    w.refresh();
                }
            })
            .on_mouse_up_out(MouseButton::Left, {
                let st = st.clone();
                move |_, w, cx| {
                    st.update(cx, |s, cx| {
                        s.press.set(0., cx);
                        cx.notify();
                    });
                    w.refresh();
                }
            })
            .on_key_down({
                let st = st.clone();
                move |ev, w, cx| {
                    if matches!(ev.keystroke.key.as_str(), "enter" | "space") {
                        st.update(cx, |s, cx| {
                            s.press.set(1., cx);
                            cx.notify();
                        });
                        w.refresh();
                    }
                }
            })
            .on_key_up({
                let st = st.clone();
                move |ev, w, cx| {
                    if matches!(ev.keystroke.key.as_str(), "enter" | "space") {
                        st.update(cx, |s, cx| {
                            s.press.set(0., cx);
                            cx.notify();
                        });
                        w.refresh();
                    }
                }
            })
            .on_click(move |ev, w, cx| (on_click)(ev, w, cx))
            .child(paint_face)
            .child(
                div()
                    .relative()
                    .top(px(0.5 * down))
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(px(5.))
                    .when_some(self.lead_icon, |d, ic| {
                        d.child(
                            div()
                                .opacity(if swap_t < 1. { enter_ease } else { 1. })
                                .child(pro_icon(ic, icon_sz * (1. - 0.05 * down), fg_current)),
                        )
                    })
                    .child(label_el),
            )
    }
}

/// Compact interactive button for Pro screens (`28px` small or `32px` standard) with KUI spring physics.
pub fn pro_btn<V: 'static>(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    lead_icon: Option<ProIcon>,
    style: ProBtnStyle,
    small: bool,
    cx: &Context<V>,
    on_press: impl Fn(&mut V, &mut Window, &mut Context<V>) + Clone + 'static,
) -> Stateful<Div> {
    let id = id.into();
    let label = label.into();
    let h = if small { 28. } else { 32. };
    let state_id = pro_eid(&id, "btn-st");
    let on_click: ProClickFn = Rc::new(cx.listener(move |v, _: &ClickEvent, w, cx| {
        on_press(v, w, cx);
    }));
    div()
        .id(id)
        .h(px(h))
        .flex_none()
        .flex()
        .items_center()
        .rounded(px(7.))
        .child(ProBtnVisual {
            state_id,
            label,
            lead_icon,
            style,
            small,
            on_click,
        })
}

struct ProIconBtnAnim {
    hover: Slide,
    press: Slide,
    active_val: bool,
    active_slide: Slide,
}

#[derive(IntoElement)]
struct ProIconBtnVisual {
    state_id: ElementId,
    aria: SharedString,
    icon: ProIcon,
    side: f32,
    active: bool,
    on_click: ProClickFn,
}

impl RenderOnce for ProIconBtnVisual {
    fn render(self, w: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let init_active = self.active;
        let st = w.use_keyed_state(self.state_id, cx, |_, _| ProIconBtnAnim {
            hover: Slide::snappy(0.),
            press: Slide::snappy(0.),
            active_val: init_active,
            active_slide: Slide::snappy(f32::from(init_active)),
        });
        let (hov, down, act) = st.update(cx, |s, cx| {
            if s.active_val != self.active {
                s.active_val = self.active;
                s.active_slide.duration(f32::from(self.active), 0.18, cx);
            }
            (
                s.hover.get(w, cx).clamp(0., 1.),
                s.press.get(w, cx).clamp(0., 1.),
                s.active_slide.get(w, cx).clamp(0., 1.),
            )
        });

        let base_bg = pro_mix_color(transparent_black(), p.chip, act);
        let face_bg = pro_mix_color(base_bg, p.hover, hov * (1. - 0.35 * act));
        let fg = pro_mix_color(pro_mix_color(p.muted, p.ink, act), p.ink, hov * 0.75);
        let icon_px = (self.side * 0.56).round().clamp(12., 16.) * (1. - 0.10 * down);

        let paint_face = canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                if face_bg.a <= 0.001 {
                    return;
                }
                let bw = f32::from(bounds.size.width);
                let bh = f32::from(bounds.size.height);
                let inset = 1.3 * down;
                let rect = Bounds::new(
                    point(bounds.origin.x + px(inset), bounds.origin.y + px(inset)),
                    size(px((bw - 2. * inset).max(1.)), px((bh - 2. * inset).max(1.))),
                );
                let r = px(7.).min(rect.size.width / 2.).min(rect.size.height / 2.);
                window.paint_quad(fill(rect, face_bg).corner_radii(r));
            },
        )
        .absolute()
        .inset_0();

        let on_click = self.on_click;
        div()
            .id("pro-ibtn-hit")
            .role(Role::Button)
            .aria_label(self.aria)
            .tab_index(0)
            .size(px(self.side))
            .relative()
            .rounded(px(7.))
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .focus_visible(move |s| s.border_1().border_color(p.focus))
            .on_hover({
                let st = st.clone();
                move |inside, w, cx| {
                    st.update(cx, |s, cx| {
                        s.hover.set(if *inside { 1. } else { 0. }, cx);
                        if !*inside {
                            s.press.set(0., cx);
                        }
                        cx.notify();
                    });
                    w.refresh();
                }
            })
            .on_mouse_down(MouseButton::Left, {
                let st = st.clone();
                move |_, w, cx| {
                    st.update(cx, |s, cx| {
                        s.press.set(1., cx);
                        cx.notify();
                    });
                    w.refresh();
                }
            })
            .on_mouse_up(MouseButton::Left, {
                let st = st.clone();
                move |_, w, cx| {
                    st.update(cx, |s, cx| {
                        s.press.set(0., cx);
                        cx.notify();
                    });
                    w.refresh();
                }
            })
            .on_mouse_up_out(MouseButton::Left, {
                let st = st.clone();
                move |_, w, cx| {
                    st.update(cx, |s, cx| {
                        s.press.set(0., cx);
                        cx.notify();
                    });
                    w.refresh();
                }
            })
            .on_key_down({
                let st = st.clone();
                move |ev, w, cx| {
                    if matches!(ev.keystroke.key.as_str(), "enter" | "space") {
                        st.update(cx, |s, cx| {
                            s.press.set(1., cx);
                            cx.notify();
                        });
                        w.refresh();
                    }
                }
            })
            .on_key_up({
                let st = st.clone();
                move |ev, w, cx| {
                    if matches!(ev.keystroke.key.as_str(), "enter" | "space") {
                        st.update(cx, |s, cx| {
                            s.press.set(0., cx);
                            cx.notify();
                        });
                        w.refresh();
                    }
                }
            })
            .on_click(move |ev, w, cx| (on_click)(ev, w, cx))
            .child(paint_face)
            .child(div().relative().child(pro_icon(self.icon, icon_px, fg)))
    }
}

/// Square icon-only button (`28x28` default) with KUI spring hover and tactile press physics.
pub fn pro_icon_btn<V: 'static>(
    id: impl Into<ElementId>,
    aria: impl Into<SharedString>,
    icon: ProIcon,
    side: f32,
    active: bool,
    cx: &Context<V>,
    on_press: impl Fn(&mut V, &mut Window, &mut Context<V>) + Clone + 'static,
) -> Stateful<Div> {
    let id = id.into();
    let aria = aria.into();
    let state_id = pro_eid(&id, "ibtn-st");
    let on_click: ProClickFn = Rc::new(cx.listener(move |v, _: &ClickEvent, w, cx| {
        on_press(v, w, cx);
    }));
    div()
        .id(id)
        .size(px(side))
        .flex_none()
        .rounded(px(7.))
        .flex()
        .items_center()
        .justify_center()
        .child(ProIconBtnVisual {
            state_id,
            aria,
            icon,
            side,
            active,
            on_click,
        })
}

struct ProSwitchAnim {
    checked: bool,
    position: Slide,
    stretch: Slide,
    hover: Slide,
    tint: Slide,
}

#[derive(IntoElement)]
struct ProSwitchVisual {
    state_id: ElementId,
    aria: SharedString,
    checked: bool,
    on_click: ProClickFn,
}

impl RenderOnce for ProSwitchVisual {
    fn render(self, w: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let init_v = u8::from(self.checked) as f32;
        let init_checked = self.checked;
        let st = w.use_keyed_state(self.state_id, cx, |_, _| ProSwitchAnim {
            checked: init_checked,
            position: Slide::new(init_v),
            stretch: Slide::snappy(0.),
            hover: Slide::snappy(0.),
            tint: Slide::new(init_v),
        });
        let (pos, extra, hov, tint) = st.update(cx, |s, cx| {
            if s.checked != self.checked {
                s.checked = self.checked;
                let target = u8::from(self.checked) as f32;
                s.position.duration(target, 0.28, cx);
                s.tint.duration(target, 0.22, cx);
            }
            let stretch = if disabled_motion(cx) {
                0.
            } else {
                3.5 * s.stretch.get(w, cx).clamp(0., 1.)
            };
            (
                s.position.get(w, cx).clamp(0., 1.),
                stretch,
                s.hover.get(w, cx).clamp(0., 1.),
                s.tint.get(w, cx).clamp(0., 1.),
            )
        });

        let off_track: Hsla = if p.dark {
            rgb(0x3a3a3f).into()
        } else {
            rgb(0xe2e2e2).into()
        };
        let off_hover = pro_mix_color(off_track, p.ink, 0.08 * hov);
        let track_bg = pro_mix_color(off_hover, p.primary, tint);
        let thumb_bg = pro_mix_color(rgb(0xffffff).into(), p.on_primary, tint);
        let travel = 14.0;
        let thumb_left = 2.0 + pos * (travel - extra);
        let thumb_w = 16.0 + extra;

        let on_click = self.on_click;
        div()
            .id("pro-sw-hit")
            .role(Role::Switch)
            .aria_label(self.aria)
            .aria_toggled(self.checked.into())
            .tab_index(0)
            .w(px(34.))
            .h(px(20.))
            .relative()
            .rounded_full()
            .bg(track_bg)
            .cursor_pointer()
            .focus_visible(move |s| s.border_1().border_color(p.focus))
            .on_hover({
                let st = st.clone();
                move |inside, w, cx| {
                    st.update(cx, |s, cx| {
                        s.hover.set(if *inside { 1. } else { 0. }, cx);
                        if !*inside {
                            s.stretch.set(0., cx);
                        }
                        cx.notify();
                    });
                    w.refresh();
                }
            })
            .on_mouse_down(MouseButton::Left, {
                let st = st.clone();
                move |_, w, cx| {
                    st.update(cx, |s, cx| {
                        s.stretch.set(1., cx);
                        cx.notify();
                    });
                    w.refresh();
                }
            })
            .on_mouse_up(MouseButton::Left, {
                let st = st.clone();
                move |_, w, cx| {
                    st.update(cx, |s, cx| {
                        s.stretch.set(0., cx);
                        cx.notify();
                    });
                    w.refresh();
                }
            })
            .on_mouse_up_out(MouseButton::Left, {
                let st = st.clone();
                move |_, w, cx| {
                    st.update(cx, |s, cx| {
                        s.stretch.set(0., cx);
                        cx.notify();
                    });
                    w.refresh();
                }
            })
            .on_key_down({
                let st = st.clone();
                move |ev, w, cx| {
                    if matches!(ev.keystroke.key.as_str(), "enter" | "space") {
                        st.update(cx, |s, cx| {
                            s.stretch.set(1., cx);
                            cx.notify();
                        });
                        w.refresh();
                    }
                }
            })
            .on_key_up({
                let st = st.clone();
                move |ev, w, cx| {
                    if matches!(ev.keystroke.key.as_str(), "enter" | "space") {
                        st.update(cx, |s, cx| {
                            s.stretch.set(0., cx);
                            cx.notify();
                        });
                        w.refresh();
                    }
                }
            })
            .on_click(move |ev, w, cx| (on_click)(ev, w, cx))
            .child(
                div()
                    .absolute()
                    .top(px(2.))
                    .left(px(thumb_left))
                    .w(px(thumb_w))
                    .h(px(16.))
                    .rounded_full()
                    .bg(thumb_bg)
                    .when(!p.dark, |d| d.shadow_sm()),
            )
    }
}

/// Capsule switch control (`34x20px`, `radius.full` per `DESIGN.md` capsule rule) with KUI spring thumb and press-stretch.
pub fn pro_switch<V: 'static>(
    id: impl Into<ElementId>,
    aria: impl Into<SharedString>,
    checked: bool,
    cx: &Context<V>,
    on_toggle: impl Fn(&mut V, &mut Window, &mut Context<V>) + Clone + 'static,
) -> Stateful<Div> {
    let id = id.into();
    let aria = aria.into();
    let state_id = pro_eid(&id, "sw-st");
    let on_click: ProClickFn = Rc::new(cx.listener(move |v, _: &ClickEvent, w, cx| {
        on_toggle(v, w, cx);
    }));
    div()
        .id(id)
        .w(px(34.))
        .h(px(20.))
        .flex_none()
        .rounded_full()
        .child(ProSwitchVisual {
            state_id,
            aria,
            checked,
            on_click,
        })
}

/// Soft tinted status pill with optional leading live dot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProTone {
    Neutral,
    Blue,
    Green,
    Amber,
    Red,
    Purple,
}

/// Compact status badge (`20px` height) with optional leading dot.
pub fn pro_badge(label: impl Into<SharedString>, tone: ProTone, dot: bool, cx: &App) -> Div {
    let p = ProPalette::get(cx);
    let (bg, fg, dot_c) = match tone {
        ProTone::Neutral => (p.chip, p.muted, p.muted),
        ProTone::Blue => (p.accent_soft, p.blue, p.blue),
        ProTone::Green => (p.green_soft, p.green_ink, p.green),
        ProTone::Amber => (p.amber_soft, p.amber_ink, p.amber),
        ProTone::Red => (p.red_soft, p.red_ink, p.red),
        ProTone::Purple => (p.purple.opacity(0.14), p.purple, p.purple),
    };
    div()
        .h(px(20.))
        .px(px(7.))
        .flex_none()
        .rounded(px(6.))
        .bg(bg)
        .text_color(fg)
        .text_size(px(11.5))
        .font_weight(FontWeight::MEDIUM)
        .flex()
        .items_center()
        .gap(px(5.))
        .when(dot, |d| {
            d.child(div().size(px(6.)).rounded_full().bg(dot_c))
        })
        .child(label.into())
}

#[derive(IntoElement)]
struct ProProgressRing {
    progress: f32,
    side: f32,
    stroke: f32,
    color: Hsla,
    track: Hsla,
}

impl RenderOnce for ProProgressRing {
    fn render(self, w: &mut Window, cx: &mut App) -> impl IntoElement {
        let target = self.progress.clamp(0., 1.);
        let st = w.use_state(cx, |_, _| (target, Slide::new(target)));
        let pct = st.update(cx, |s, cx| {
            if (s.0 - target).abs() > 0.0005 {
                s.0 = target;
                s.1.duration(target, 0.36, cx);
            }
            s.1.get(w, cx).clamp(0., 1.)
        });
        let stroke = self.stroke;
        let color = self.color;
        let track = self.track;
        canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let center = bounds.center();
                let r = (f32::from(bounds.size.width).min(f32::from(bounds.size.height)) - stroke)
                    * 0.5;
                if r <= 1. {
                    return;
                }
                let steps = 32usize;
                let mut ring = PathBuilder::stroke(px(stroke));
                for i in 0..=steps {
                    let a = (i as f32 / steps as f32) * std::f32::consts::TAU
                        - std::f32::consts::FRAC_PI_2;
                    let pt = center + point(px(r * a.cos()), px(r * a.sin()));
                    if i == 0 {
                        ring.move_to(pt);
                    } else {
                        ring.line_to(pt);
                    }
                }
                if let Ok(path) = ring.build() {
                    window.paint_path(path, track);
                }
                if pct > 0.005 {
                    let arc_steps = ((steps as f32 * pct).ceil() as usize).max(2);
                    let mut arc = PathBuilder::stroke(px(stroke));
                    for i in 0..=arc_steps {
                        let frac = (i as f32 / arc_steps as f32) * pct;
                        let a = frac * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
                        let pt = center + point(px(r * a.cos()), px(r * a.sin()));
                        if i == 0 {
                            arc.move_to(pt);
                        } else {
                            arc.line_to(pt);
                        }
                    }
                    if let Ok(path) = arc.build() {
                        window.paint_path(path, color);
                    }
                }
            },
        )
        .size(px(self.side))
        .flex_none()
    }
}

/// Circular progress ring drawn via GPUI [`PathBuilder`] with smooth spring sweep on changes.
pub fn pro_progress_ring(
    progress: f32,
    side: f32,
    stroke: f32,
    color: Hsla,
    track: Hsla,
) -> AnyElement {
    ProProgressRing {
        progress,
        side,
        stroke,
        color,
        track,
    }
    .into_any_element()
}

struct ProTrendState {
    from_vals: Vec<f32>,
    to_vals: Vec<f32>,
    from_cmp: Option<Vec<f32>>,
    to_cmp: Option<Vec<f32>>,
    clock: PhaseClock,
}

fn resample_series(src: &[f32], len: usize) -> Vec<f32> {
    if src.is_empty() || len == 0 {
        return vec![0.; len];
    }
    if src.len() == len {
        return src.to_vec();
    }
    if src.len() == 1 || len == 1 {
        return vec![src[0]; len];
    }
    (0..len)
        .map(|i| {
            let pos = (i as f32 / (len - 1) as f32) * (src.len() - 1) as f32;
            let i0 = (pos.floor() as usize).min(src.len() - 1);
            let i1 = (i0 + 1).min(src.len() - 1);
            let frac = pos - i0 as f32;
            src[i0] + (src[i1] - src[i0]) * frac
        })
        .collect()
}

#[derive(IntoElement)]
struct ProTrendCanvas {
    values: Vec<f32>,
    compare: Option<Vec<f32>>,
    stroke_color: Hsla,
    fill_alpha: f32,
    reveal: f32,
    height: f32,
}

impl RenderOnce for ProTrendCanvas {
    fn render(self, w: &mut Window, cx: &mut App) -> impl IntoElement {
        let init_v = self.values.clone();
        let init_c = self.compare.clone();
        let st = w.use_state(cx, |_, _| ProTrendState {
            from_vals: init_v.clone(),
            to_vals: init_v,
            from_cmp: init_c.clone(),
            to_cmp: init_c,
            clock: PhaseClock::default(),
        });
        let (values, compare) = st.update(cx, |s, cx| {
            let age = s.clock.time(0.28, w, cx);
            let cur_t = pro_ease((age / 0.28).clamp(0., 1.));
            if s.to_vals != self.values || s.to_cmp != self.compare {
                let target_len = self.values.len();
                let prev_a = resample_series(&s.from_vals, target_len);
                let prev_b = resample_series(&s.to_vals, target_len);
                s.from_vals = prev_a
                    .into_iter()
                    .zip(prev_b)
                    .map(|(a, b)| a + (b - a) * cur_t)
                    .collect();
                s.to_vals = self.values.clone();
                s.from_cmp = s.to_cmp.clone();
                s.to_cmp = self.compare.clone();
                s.clock.restart(cx);
            }
            let age = s.clock.time(0.28, w, cx);
            let t = pro_ease((age / 0.28).clamp(0., 1.));
            let vals: Vec<f32> = if t >= 0.999 || s.from_vals.len() != s.to_vals.len() {
                s.to_vals.clone()
            } else {
                s.from_vals
                    .iter()
                    .zip(&s.to_vals)
                    .map(|(&a, &b)| a + (b - a) * t)
                    .collect()
            };
            let cmp = match (&s.from_cmp, &s.to_cmp) {
                (Some(fc), Some(tc)) if t < 0.999 && fc.len() == tc.len() => Some(
                    fc.iter()
                        .zip(tc)
                        .map(|(&a, &b)| a + (b - a) * t)
                        .collect::<Vec<_>>(),
                ),
                (_, other) => other.clone(),
            };
            (vals, cmp)
        });

        let reveal = self.reveal.clamp(0., 1.);
        let stroke_color = self.stroke_color;
        let fill_alpha = self.fill_alpha;
        canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let w = f32::from(bounds.size.width);
                let h = f32::from(bounds.size.height);
                if w <= 4. || h <= 4. || values.len() < 2 {
                    return;
                }
                let mut min_v = f32::INFINITY;
                let mut max_v = f32::NEG_INFINITY;
                for &v in values.iter().chain(compare.iter().flatten()) {
                    min_v = min_v.min(v);
                    max_v = max_v.max(v);
                }
                let span = (max_v - min_v).max(1e-3);
                let pad_y = 4.;
                let map_pt = |idx: usize, count: usize, val: f32| -> Point<Pixels> {
                    let tx = idx as f32 / (count.saturating_sub(1)).max(1) as f32;
                    let ty = 1. - ((val - min_v) / span).clamp(0., 1.);
                    point(
                        bounds.left() + px(tx * w),
                        bounds.top() + px(pad_y + ty * (h - 2. * pad_y)),
                    )
                };

                if let Some(cmp) = &compare
                    && cmp.len() >= 2
                {
                    let mut cmp_line = PathBuilder::stroke(px(1.25));
                    for (i, &v) in cmp.iter().enumerate() {
                        let pt = map_pt(i, cmp.len(), v);
                        if i == 0 {
                            cmp_line.move_to(pt);
                        } else {
                            cmp_line.line_to(pt);
                        }
                    }
                    if let Ok(path) = cmp_line.build() {
                        window.paint_path(path, stroke_color.opacity(0.28));
                    }
                }

                let visible_n = (((values.len() - 1) as f32 * reveal).ceil() as usize + 1)
                    .clamp(2, values.len());
                if fill_alpha > 0.001 {
                    let mut area = PathBuilder::fill();
                    let first = map_pt(0, values.len(), values[0]);
                    let last = map_pt(visible_n - 1, values.len(), values[visible_n - 1]);
                    area.move_to(point(first.x, bounds.bottom()));
                    area.line_to(first);
                    for (i, &v) in values.iter().enumerate().take(visible_n).skip(1) {
                        area.line_to(map_pt(i, values.len(), v));
                    }
                    area.line_to(point(last.x, bounds.bottom()));
                    area.close();
                    if let Ok(path) = area.build() {
                        window.paint_path(
                            path,
                            linear_gradient(
                                180.,
                                linear_color_stop(stroke_color.opacity(fill_alpha), 0.),
                                linear_color_stop(stroke_color.opacity(0.), 1.),
                            ),
                        );
                    }
                }

                let mut line = PathBuilder::stroke(px(1.75));
                for (i, &v) in values.iter().enumerate().take(visible_n) {
                    let pt = map_pt(i, values.len(), v);
                    if i == 0 {
                        line.move_to(pt);
                    } else {
                        line.line_to(pt);
                    }
                }
                if let Ok(path) = line.build() {
                    window.paint_path(path, stroke_color);
                }
            },
        )
        .w_full()
        .h(px(self.height))
    }
}

/// Smooth polyline sparkline/trend chart with series-morphing transitions.
pub fn pro_trend_canvas(
    values: Vec<f32>,
    compare: Option<Vec<f32>>,
    stroke_color: Hsla,
    fill_alpha: f32,
    reveal: f32,
    height: f32,
) -> AnyElement {
    ProTrendCanvas {
        values,
        compare,
        stroke_color,
        fill_alpha,
        reveal,
        height,
    }
    .into_any_element()
}

/// Skeleton placeholder bar with synchronized sweep opacity.
pub fn pro_skeleton_bar(width: f32, height: f32, radius: f32, phase: f32, cx: &App) -> Div {
    let p = ProPalette::get(cx);
    let pulse = if Theme::global(cx).reduced_motion {
        0.55
    } else {
        0.42 + 0.26 * (phase * std::f32::consts::TAU).sin().abs()
    };
    div()
        .w(px(width))
        .h(px(height))
        .rounded(px(radius))
        .bg(p.chip_hover.opacity(pulse))
}

/// Render callback for custom Pro slots and overlays.
pub type ProSlotFn = std::rc::Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>;

/// Call-to-action specification for [`pro_empty_state`].
pub type ProEmptyCta<V> = (
    ElementId,
    SharedString,
    Box<dyn Fn(&mut V, &mut Window, &mut Context<V>) + 'static>,
);

/// Centered empty state with a soft icon tile, title, one-line caption, and optional CTA.
pub fn pro_empty_state<V: 'static>(
    icon: ProIcon,
    title: impl Into<SharedString>,
    caption: impl Into<SharedString>,
    cta: Option<ProEmptyCta<V>>,
    cx: &Context<V>,
) -> Div {
    let p = ProPalette::get(cx);
    let mut box_el = div()
        .w_full()
        .py(px(48.))
        .px(px(24.))
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(10.))
        .child(
            div()
                .size(px(40.))
                .rounded(px(10.))
                .bg(p.chip)
                .border_1()
                .border_color(p.line)
                .flex()
                .items_center()
                .justify_center()
                .child(pro_icon(icon, 18., p.muted)),
        )
        .child(
            div()
                .text_size(px(14.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.ink)
                .child(title.into()),
        )
        .child(
            div()
                .max_w(px(320.))
                .text_size(px(13.))
                .line_height(px(19.))
                .text_color(p.muted)
                .text_center()
                .child(caption.into()),
        );
    if let Some((id, label, handler)) = cta {
        let rc = std::rc::Rc::new(handler);
        box_el = box_el.child(div().pt(px(4.)).child(pro_btn(
            id,
            label,
            None,
            ProBtnStyle::Secondary,
            true,
            cx,
            move |v, w, cx| (rc)(v, w, cx),
        )));
    }
    box_el
}

// ============================================================================
// KUI Pro Motion Primitives
// ============================================================================

/// Sample a critically-damped [`Slide`] spring keyed by `id`, retargeting smoothly whenever `target` changes.
pub fn pro_spring_f32(
    id: impl Into<ElementId>,
    target: f32,
    seconds: f32,
    w: &mut Window,
    cx: &mut App,
) -> f32 {
    let state = w.use_keyed_state(id.into(), cx, |_, _| (target, Slide::new(target)));
    state.update(cx, |s, cx| {
        if (s.0 - target).abs() > 0.0005 {
            s.0 = target;
            s.1.duration(target, seconds, cx);
        }
        s.1.get(w, cx)
    })
}

#[derive(IntoElement)]
struct ProSpringBar {
    id: ElementId,
    ratio: f32,
    height: f32,
    radius: f32,
    track: Hsla,
    fill: Hsla,
}

impl RenderOnce for ProSpringBar {
    fn render(self, w: &mut Window, cx: &mut App) -> impl IntoElement {
        let r = pro_spring_f32(
            pro_eid(&self.id, "sbar"),
            self.ratio.clamp(0., 1.),
            0.34,
            w,
            cx,
        )
        .clamp(0., 1.);
        div()
            .w_full()
            .h(px(self.height))
            .rounded(px(self.radius))
            .bg(self.track)
            .overflow_hidden()
            .child(
                div()
                    .h_full()
                    .w(relative(r))
                    .rounded(px(self.radius))
                    .bg(self.fill),
            )
    }
}

/// Horizontal meter or progress bar whose fill width springs smoothly on value changes.
pub fn pro_spring_bar(
    id: impl Into<ElementId>,
    ratio: f32,
    height: f32,
    radius: f32,
    track: Hsla,
    fill: Hsla,
) -> AnyElement {
    ProSpringBar {
        id: id.into(),
        ratio,
        height,
        radius,
        track,
        fill,
    }
    .into_any_element()
}

/// Shared prepaint bounds sensor for a group of tabs, segmented pills, or navigation rows.
#[derive(Clone, Default)]
pub struct ProSelectionSensor(Rc<Cell<Option<Bounds<Pixels>>>>);

/// Create a new [`ProSelectionSensor`] for one tab strip, segmented control, or sidebar list.
pub fn pro_selection_sensor() -> ProSelectionSensor {
    ProSelectionSensor::default()
}

/// Attach inside the currently selected item (`relative()` parent) so [`pro_pill_indicator`] or
/// [`pro_underline_indicator`] can glide to its exact bounds.
pub fn pro_mark_selected(sensor: &ProSelectionSensor) -> AnyElement {
    let cell = sensor.0.clone();
    canvas(
        move |bounds, _, _| {
            cell.set(Some(bounds));
        },
        |_, _, _, _| (),
    )
    .absolute()
    .inset_0()
    .into_any_element()
}

struct ProIndicatorState {
    key: SharedString,
    x: Slide,
    y: Slide,
    w: Slide,
    h: Slide,
    ready: bool,
}

impl ProIndicatorState {
    fn new(key: SharedString) -> Self {
        Self {
            key,
            x: Slide::snappy(0.),
            y: Slide::snappy(0.),
            w: Slide::snappy(0.),
            h: Slide::snappy(0.),
            ready: false,
        }
    }

    fn update_rect(
        &mut self,
        next_key: &SharedString,
        target: Bounds<Pixels>,
        surface: Bounds<Pixels>,
        seconds: f32,
        w: &Window,
        cx: &App,
    ) -> Bounds<Pixels> {
        let tx = f32::from(target.left() - surface.left());
        let ty = f32::from(target.top() - surface.top());
        let tw = f32::from(target.size.width);
        let th = f32::from(target.size.height);
        if !self.ready || &self.key == next_key {
            self.x.reset(tx);
            self.y.reset(ty);
            self.w.reset(tw);
            self.h.reset(th);
            self.ready = true;
        } else {
            self.key = next_key.clone();
            self.x.duration(tx, seconds, cx);
            self.y.duration(ty, seconds, cx);
            self.w.duration(tw, seconds, cx);
            self.h.duration(th, seconds, cx);
        }
        let cx_x = self.x.get(w, cx);
        let cy_y = self.y.get(w, cx);
        let cw_w = self.w.get(w, cx).max(0.);
        let ch_h = self.h.get(w, cx).max(0.);
        Bounds::new(
            point(surface.left() + px(cx_x), surface.top() + px(cy_y)),
            size(px(cw_w), px(ch_h)),
        )
    }
}

#[derive(IntoElement)]
struct ProPillIndicator {
    id: ElementId,
    selected_key: SharedString,
    sensor: ProSelectionSensor,
    radius: f32,
    fill: Hsla,
    border: Option<Hsla>,
    shadow: bool,
}

impl RenderOnce for ProPillIndicator {
    fn render(self, w: &mut Window, cx: &mut App) -> impl IntoElement {
        let init_key = self.selected_key.clone();
        let st = w.use_keyed_state(pro_eid(&self.id, "pill-ind"), cx, |_, _| {
            ProIndicatorState::new(init_key)
        });
        let dark = ProPalette::get(cx).dark;
        let sensor = self.sensor.0;
        let next_key = self.selected_key;
        let radius = self.radius;
        let fill_c = self.fill;
        let border_c = self.border;
        let shadow = self.shadow;

        canvas(
            |_, _, _| (),
            move |surface, _, window, cx| {
                let Some(target) = sensor.get() else {
                    return;
                };
                let rect = st.update(cx, |s, cx| {
                    s.update_rect(&next_key, target, surface, 0.25, window, cx)
                });
                if rect.size.width <= px(0.5) || rect.size.height <= px(0.5) {
                    return;
                }
                let r = px(radius)
                    .min(rect.size.width / 2.)
                    .min(rect.size.height / 2.);
                if shadow {
                    window.paint_quad(
                        fill(
                            Bounds::new(point(rect.origin.x, rect.origin.y + px(1.)), rect.size),
                            black().opacity(if dark { 0.28 } else { 0.06 }),
                        )
                        .corner_radii(r),
                    );
                }
                let (bw, bc) = match border_c {
                    Some(c) => (px(1.), c),
                    None => (px(0.), transparent_black()),
                };
                window.paint_quad(quad(
                    rect,
                    Corners::all(r),
                    fill_c,
                    Edges::all(bw),
                    bc,
                    BorderStyle::Solid,
                ));
            },
        )
        .absolute()
        .inset_0()
    }
}

/// Sliding pill selection highlight placed as Child 0 of a `.relative()` tab/segment/nav container.
pub fn pro_pill_indicator(
    id: impl Into<ElementId>,
    selected_key: impl Into<SharedString>,
    sensor: ProSelectionSensor,
    radius: f32,
    fill: Hsla,
    border: Option<Hsla>,
    shadow: bool,
) -> AnyElement {
    ProPillIndicator {
        id: id.into(),
        selected_key: selected_key.into(),
        sensor,
        radius,
        fill,
        border,
        shadow,
    }
    .into_any_element()
}

#[derive(IntoElement)]
struct ProUnderlineIndicator {
    id: ElementId,
    selected_key: SharedString,
    sensor: ProSelectionSensor,
    thickness: f32,
    color: Hsla,
}

impl RenderOnce for ProUnderlineIndicator {
    fn render(self, w: &mut Window, cx: &mut App) -> impl IntoElement {
        let init_key = self.selected_key.clone();
        let st = w.use_keyed_state(pro_eid(&self.id, "uline-ind"), cx, |_, _| {
            ProIndicatorState::new(init_key)
        });
        let sensor = self.sensor.0;
        let next_key = self.selected_key;
        let thickness = self.thickness;
        let color = self.color;

        canvas(
            |_, _, _| (),
            move |surface, _, window, cx| {
                let Some(target) = sensor.get() else {
                    return;
                };
                let rect = st.update(cx, |s, cx| {
                    s.update_rect(&next_key, target, surface, 0.24, window, cx)
                });
                if rect.size.width <= px(0.5) {
                    return;
                }
                let bar = Bounds::new(
                    point(rect.origin.x, surface.bottom() - px(thickness)),
                    size(rect.size.width, px(thickness)),
                );
                window.paint_quad(fill(bar, color).corner_radii(px(thickness * 0.5)));
            },
        )
        .absolute()
        .inset_0()
    }
}

/// Sliding bottom underline indicator placed as Child 0 of a `.relative()` tab strip.
pub fn pro_underline_indicator(
    id: impl Into<ElementId>,
    selected_key: impl Into<SharedString>,
    sensor: ProSelectionSensor,
    thickness: f32,
    color: Hsla,
) -> AnyElement {
    ProUnderlineIndicator {
        id: id.into(),
        selected_key: selected_key.into(),
        sensor,
        thickness,
        color,
    }
    .into_any_element()
}

struct ProViewTxState {
    key: SharedString,
    step: i32,
    dir: f32,
    clock: PhaseClock,
}

#[derive(IntoElement)]
struct ProViewTransition {
    id: ElementId,
    key: SharedString,
    step: Option<i32>,
    fill_parent: bool,
    content: AnyElement,
}

impl RenderOnce for ProViewTransition {
    fn render(self, w: &mut Window, cx: &mut App) -> impl IntoElement {
        let init_key = self.key.clone();
        let init_step = self.step.unwrap_or(0);
        let st = w.use_keyed_state(pro_eid(&self.id, "vtx"), cx, |_, _| ProViewTxState {
            key: init_key,
            step: init_step,
            dir: 1.,
            clock: PhaseClock::default(),
        });
        let (dx, dy, alpha) = st.update(cx, |s, cx| {
            let next_step = self.step.unwrap_or(0);
            if s.key != self.key || (self.step.is_some() && s.step != next_step) {
                s.dir = if self.step.is_some() {
                    if next_step >= s.step { 1. } else { -1. }
                } else {
                    1.
                };
                s.key = self.key.clone();
                s.step = next_step;
                s.clock.restart(cx);
            }
            let age = s.clock.time(0.20, w, cx);
            let t = pro_ease((age / 0.20).clamp(0., 1.));
            if t >= 0.999 {
                (0., 0., 1.)
            } else if self.step.is_some() {
                (8. * s.dir * (1. - t), 0., 0.36 + 0.64 * t)
            } else {
                (0., 5. * (1. - t), 0.42 + 0.58 * t)
            }
        });

        let wrap = div()
            .relative()
            .left(px(dx))
            .top(px(dy))
            .opacity(alpha)
            .child(self.content);
        if self.fill_parent {
            wrap.size_full().min_h_0().flex().flex_col()
        } else {
            wrap.w_full()
        }
    }
}

/// Crisp `200ms` vertical micro-glide and fade whenever `state_key` changes (zero motion on initial mount).
pub fn pro_view_transition(
    id: impl Into<ElementId>,
    state_key: impl Into<SharedString>,
    content: impl IntoElement,
) -> AnyElement {
    ProViewTransition {
        id: id.into(),
        key: state_key.into(),
        step: None,
        fill_parent: false,
        content: content.into_any_element(),
    }
    .into_any_element()
}

/// Same as [`pro_view_transition`] for views that fill a `flex_1().min_h_0()` column.
pub fn pro_view_transition_fill(
    id: impl Into<ElementId>,
    state_key: impl Into<SharedString>,
    content: impl IntoElement,
) -> AnyElement {
    ProViewTransition {
        id: id.into(),
        key: state_key.into(),
        step: None,
        fill_parent: true,
        content: content.into_any_element(),
    }
    .into_any_element()
}

/// Directional horizontal step transition (`±8px` + fade) whenever `step_index` advances or retreats.
pub fn pro_step_transition(
    id: impl Into<ElementId>,
    step_index: i32,
    content: impl IntoElement,
) -> AnyElement {
    ProViewTransition {
        id: id.into(),
        key: SharedString::from(format!("step-{step_index}")),
        step: Some(step_index),
        fill_parent: false,
        content: content.into_any_element(),
    }
    .into_any_element()
}

struct ProFoldState {
    open: bool,
    phase: Slide,
    height: Rc<Cell<Pixels>>,
}

#[derive(IntoElement)]
struct ProFold {
    id: ElementId,
    open: bool,
    estimate: f32,
    parent_gap: f32,
    render: Box<dyn FnOnce() -> AnyElement>,
}

impl RenderOnce for ProFold {
    fn render(self, w: &mut Window, cx: &mut App) -> impl IntoElement {
        let init_open = self.open;
        let est = self.estimate;
        let st = w.use_keyed_state(pro_eid(&self.id, "fold"), cx, |_, _| ProFoldState {
            open: init_open,
            phase: Slide::new(f32::from(init_open)),
            height: Rc::new(Cell::new(px(est))),
        });
        let (amount, h_cell) = st.update(cx, |s, cx| {
            if s.open != self.open {
                s.open = self.open;
                s.phase.duration(f32::from(self.open), 0.24, cx);
            }
            (s.phase.get(w, cx).clamp(0., 1.), s.height.clone())
        });

        if !self.open && amount <= 0.002 {
            return div().into_any_element();
        }
        let h = h_cell.get();
        let content = (self.render)();
        div()
            .mt(px(-self.parent_gap * (1. - amount)))
            .w_full()
            .flex_none()
            .when(amount < 0.998, |v| v.h(h * amount).overflow_hidden())
            .opacity(amount)
            .child(
                div()
                    .w_full()
                    .flex_none()
                    .on_children_prepainted(move |bounds, _, _| {
                        if let Some(b) = bounds.first()
                            && b.size.height > px(0.)
                        {
                            h_cell.set(b.size.height);
                        }
                    })
                    .child(content),
            )
            .into_any_element()
    }
}

/// Measured-height spring disclosure (`0.24s`) that retains natural height during exit.
pub fn pro_fold(
    id: impl Into<ElementId>,
    open: bool,
    estimate: f32,
    parent_gap: f32,
    render: impl FnOnce() -> AnyElement + 'static,
) -> AnyElement {
    ProFold {
        id: id.into(),
        open,
        estimate,
        parent_gap,
        render: Box::new(render),
    }
    .into_any_element()
}

struct ProPresenceState {
    open: bool,
    phase: Slide,
}

#[derive(IntoElement)]
struct ProPresenceLayer {
    id: ElementId,
    open: bool,
    open_secs: f32,
    close_secs: f32,
    render: Box<dyn FnOnce(f32) -> AnyElement>,
}

impl RenderOnce for ProPresenceLayer {
    fn render(self, w: &mut Window, cx: &mut App) -> impl IntoElement {
        let init_open = self.open;
        let st = w.use_keyed_state(pro_eid(&self.id, "pres"), cx, |_, _| ProPresenceState {
            open: init_open,
            phase: Slide::new(f32::from(init_open)),
        });
        let amount = st.update(cx, |s, cx| {
            if s.open != self.open {
                s.open = self.open;
                let secs = if self.open {
                    self.open_secs
                } else {
                    self.close_secs
                };
                s.phase.duration(f32::from(self.open), secs, cx);
            }
            s.phase.get(w, cx).clamp(0., 1.)
        });
        if !self.open && amount <= 0.003 {
            return div().into_any_element();
        }
        (self.render)(amount)
    }
}

/// Animated presence (`0.0..=1.0`) for boolean-gated modals, popovers, drawers, and dirty-state bars.
///
/// Calls `render(amount)` while opening, open, or closing so both entrance and exit animate smoothly.
pub fn pro_presence(
    id: impl Into<ElementId>,
    open: bool,
    open_secs: f32,
    close_secs: f32,
    render: impl FnOnce(f32) -> AnyElement + 'static,
) -> AnyElement {
    ProPresenceLayer {
        id: id.into(),
        open,
        open_secs,
        close_secs,
        render: Box::new(render),
    }
    .into_any_element()
}

#[derive(IntoElement)]
struct ProModalAppear {
    id: ElementId,
    key: SharedString,
    scrim_alpha: f32,
    dialog: AnyElement,
}

impl RenderOnce for ProModalAppear {
    fn render(self, w: &mut Window, cx: &mut App) -> impl IntoElement {
        let init_key = self.key.clone();
        let st = w.use_keyed_state(pro_eid(&self.id, "mappear"), cx, |_, inner_cx| {
            let mut clock = PhaseClock::default();
            clock.restart(inner_cx);
            (init_key, clock)
        });
        let t = st.update(cx, |s, cx| {
            if s.0 != self.key {
                s.0 = self.key.clone();
                s.1.restart(cx);
            }
            let age = s.1.time(0.22, w, cx);
            pro_ease((age / 0.22).clamp(0., 1.))
        });
        div()
            .size_full()
            .bg(hsla(0., 0., 0., self.scrim_alpha * t))
            .flex()
            .items_center()
            .justify_center()
            .p(px(16.))
            .child(
                div()
                    .relative()
                    .top(px(8. * (1. - t)))
                    .opacity(t)
                    .child(self.dialog),
            )
    }
}

/// Smooth entrance & step-morph wrapper for `Option`-gated modal dialogs (`220ms` scrim fade + `8px` dialog settle).
pub fn pro_modal_appear(
    id: impl Into<ElementId>,
    step_key: impl Into<SharedString>,
    scrim_alpha: f32,
    dialog: impl IntoElement,
) -> AnyElement {
    ProModalAppear {
        id: id.into(),
        key: step_key.into(),
        scrim_alpha,
        dialog: dialog.into_any_element(),
    }
    .into_any_element()
}

#[derive(IntoElement)]
struct ProToastBanner {
    toast: Option<(SharedString, Instant)>,
}

impl RenderOnce for ProToastBanner {
    fn render(self, w: &mut Window, cx: &mut App) -> impl IntoElement {
        let Some((msg, started)) = self.toast else {
            return div().into_any_element();
        };
        let elapsed = started.elapsed().as_secs_f32();
        let lifetime = 2.40;
        if elapsed >= lifetime {
            return div().into_any_element();
        }
        let reduced = disabled_motion(cx);
        if !reduced {
            w.request_animation_frame();
        }
        let t = if reduced {
            1.0
        } else if elapsed < 0.20 {
            pro_ease(elapsed / 0.20)
        } else if elapsed > lifetime - 0.26 {
            1. - pro_ease((elapsed - (lifetime - 0.26)) / 0.26)
        } else {
            1.0
        };
        let p = ProPalette::get(cx);
        div()
            .absolute()
            .bottom(px(16. - 8. * (1. - t)))
            .right(px(16.))
            .opacity(t)
            .h(px(34.))
            .px(px(12.))
            .rounded(px(8.))
            .bg(if p.dark { rgb(0x232327) } else { rgb(0x18181b) })
            .border_1()
            .border_color(if p.dark {
                p.line_strong
            } else {
                hsla(0., 0., 1., 0.12)
            })
            .shadow_md()
            .flex()
            .items_center()
            .gap(px(7.))
            .text_size(px(12.5))
            .font_weight(FontWeight::MEDIUM)
            .text_color(rgb(0xffffff))
            .child(pro_icon(ProIcon::Check, 13., p.green))
            .child(msg)
            .into_any_element()
    }
}

/// Animated floating toast banner (`2.4s` window with smooth spring slide-in and fade-out).
pub fn pro_toast_banner(toast: Option<(SharedString, Instant)>) -> AnyElement {
    ProToastBanner { toast }.into_any_element()
}

#[cfg(test)]
mod tests {
    use super::{pro_ease, pro_mix_color};
    use gpui::{Hsla, Rgba, rgb, transparent_black};

    #[test]
    fn pro_ease_is_monotonic_and_clamped() {
        assert_eq!(pro_ease(-0.5), 0.0);
        assert_eq!(pro_ease(0.0), 0.0);
        assert_eq!(pro_ease(1.0), 1.0);
        assert_eq!(pro_ease(1.5), 1.0);
        let mut prev = 0.0;
        for step in 1..=20 {
            let y = pro_ease(step as f32 / 20.0);
            assert!(y >= prev, "expected monotonic increase at step {step}");
            prev = y;
        }
    }

    #[test]
    fn pro_mix_color_preserves_hue_when_fading_from_transparent() {
        let blue: Hsla = rgb(0x0190fe).into();
        let mid = Rgba::from(pro_mix_color(transparent_black(), blue, 0.5));
        let full = Rgba::from(blue);
        assert!((mid.a - 0.5).abs() < 0.01);
        assert!((mid.r - full.r).abs() < 0.02);
        assert!((mid.g - full.g).abs() < 0.02);
        assert!((mid.b - full.b).abs() < 0.02);
    }
}
