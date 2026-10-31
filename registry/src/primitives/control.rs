//! Focus and activation shared by installed action controls.
use crate::foundation::Theme;
use gpui::{
    App, Context, Div, ElementId, Role, SharedString, Stateful, Window, div, prelude::*, px,
};

pub fn surface(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    cx: &App,
) -> Stateful<Div> {
    let label = label.into();
    let c = &Theme::global(cx).tokens.colors;
    let outline = c.focus_outline;
    div()
        .id(id)
        .role(Role::Button)
        .debug_selector(|| label.to_string())
        .aria_label(label)
        .tab_index(0)
        .min_h(px(44.))
        .px_3()
        .flex()
        .items_center()
        .justify_center()
        .rounded(Theme::global(cx).tokens.radius.md)
        .border_1()
        .border_color(c.border)
        .bg(c.surface)
        .text_color(c.foreground)
        .cursor_pointer()
        .hover(|s| s.opacity(0.8))
        .focus_visible(move |s| {
            s.shadow(vec![gpui::BoxShadow {
                color: outline,
                offset: gpui::point(px(0.), px(0.)),
                blur_radius: px(0.),
                spread_radius: px(2.),
                inset: false,
            }])
        })
}

pub fn activate<T: 'static>(
    el: Stateful<Div>,
    cx: &Context<T>,
    f: impl Fn(&mut T, &mut Window, &mut Context<T>) + Clone + 'static,
) -> Stateful<Div> {
    // GPUI turns a completed Enter/Space press into one keyboard ClickEvent.
    // Registering a second key-down action also toggles on key-up in Web hosts.
    el.on_click(cx.listener(move |v, _, w, cx| f(v, w, cx)))
}

pub fn action<T: 'static>(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    cx: &Context<T>,
    f: impl Fn(&mut T, &mut Window, &mut Context<T>) + Clone + 'static,
) -> Stateful<Div> {
    let label = label.into();
    activate(surface(id, label.clone(), cx).child(label), cx, f)
}

/// Secondary action with a text label and a full keyboard/touch target.
pub fn quiet<T: 'static>(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    cx: &Context<T>,
    f: impl Fn(&mut T, &mut Window, &mut Context<T>) + Clone + 'static,
) -> Stateful<Div> {
    let c = Theme::global(cx).tokens.colors.clone();
    action(id, label, cx, f)
        .border_0()
        .bg(gpui::transparent_black())
        .text_size(px(12.))
        .text_color(c.text_secondary)
}

/// Monochrome control marks drawn with the same paths on both renderers.
#[derive(Clone, Copy)]
pub enum Mark {
    Hold,
    Check,
    Mic,
    Stop,
    Chevron,
    Up,
    Spark,
    Heart,
}
pub fn mark(kind: Mark, ink: gpui::Hsla) -> impl IntoElement {
    use gpui::{PathBuilder, canvas, point};
    canvas(
        |_, _, _| (),
        move |b, _, w, _| {
            let at = |x: f32, y: f32| point(b.origin.x + px(x), b.origin.y + px(y));
            let mut p = PathBuilder::stroke(px(1.55));
            match kind {
                Mark::Hold => {
                    p.move_to(at(3., 10.));
                    p.curve_to(at(8., 2.5), at(2.4, 2.7));
                    p.curve_to(at(13.3, 8.), at(13.2, 2.5));
                    p.move_to(at(4.6, 13.));
                    p.curve_to(at(5.5, 7.5), at(6., 10.5));
                    p.curve_to(at(10.5, 7.5), at(8., 3.5));
                    p.curve_to(at(9.4, 14.), at(11.1, 10.5));
                    p.move_to(at(8., 7.5));
                    p.curve_to(at(7.2, 12.1), at(8.6, 9.2));
                }
                Mark::Check => {
                    p.move_to(at(2.3, 8.1));
                    p.line_to(at(6.5, 12.));
                    p.line_to(at(13.8, 3.6));
                }
                Mark::Mic => {
                    p.move_to(at(5.4, 4.));
                    p.curve_to(at(10.6, 4.), at(8., -0.9));
                    p.line_to(at(10.6, 7.8));
                    p.curve_to(at(5.4, 7.8), at(8., 12.5));
                    p.line_to(at(5.4, 4.));
                    p.move_to(at(2.5, 7.5));
                    p.curve_to(at(13.5, 7.5), at(8., 16.5));
                    p.move_to(at(8., 12.));
                    p.line_to(at(8., 15.));
                    p.move_to(at(5.5, 15.));
                    p.line_to(at(10.5, 15.));
                }
                Mark::Stop => {
                    p = PathBuilder::fill();
                    p.move_to(at(3.5, 3.5));
                    p.line_to(at(12.5, 3.5));
                    p.line_to(at(12.5, 12.5));
                    p.line_to(at(3.5, 12.5));
                    p.close();
                }
                Mark::Up => {
                    p.move_to(at(8., 13.5));
                    p.line_to(at(8., 2.5));
                    p.move_to(at(3.2, 7.5));
                    p.line_to(at(8., 2.5));
                    p.line_to(at(12.8, 7.5));
                }
                Mark::Spark => {
                    p.move_to(at(8., 1.5));
                    p.curve_to(at(14.5, 8.), at(9.2, 6.7));
                    p.curve_to(at(8., 14.5), at(9.2, 9.3));
                    p.curve_to(at(1.5, 8.), at(6.8, 9.3));
                    p.curve_to(at(8., 1.5), at(6.8, 6.7));
                    p.close();
                }
                Mark::Heart => {
                    p.move_to(at(8., 13.8));
                    p.cubic_bezier_to(at(1.8, 5.6), at(5., 11.5), at(1.8, 8.6));
                    p.cubic_bezier_to(at(8., 4.4), at(1.8, 2.), at(5.8, 1.6));
                    p.cubic_bezier_to(at(14.2, 5.6), at(10.2, 1.6), at(14.2, 2.));
                    p.cubic_bezier_to(at(8., 13.8), at(14.2, 8.6), at(11., 11.5));
                    p.close();
                }
                Mark::Chevron => {
                    p.move_to(at(5., 6.5));
                    p.line_to(at(8., 9.5));
                    p.line_to(at(11., 6.5));
                }
            }
            if let Ok(path) = p.build() {
                w.paint_path(path, ink);
            }
        },
    )
    .size(px(16.))
    .flex_none()
}

/// Optional host language for labels owned by action controls; English is the default.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum UiLanguage {
    #[default]
    English,
    Chinese,
}
impl gpui::Global for UiLanguage {}
impl UiLanguage {
    pub fn current(cx: &App) -> Self {
        cx.try_global::<Self>().copied().unwrap_or_default()
    }
    pub fn text<'a>(self, english: &'a str, chinese: &'a str) -> &'a str {
        match self {
            Self::English => english,
            Self::Chinese => chinese,
        }
    }
}
