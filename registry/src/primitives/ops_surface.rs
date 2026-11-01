//! Shared typography, keyboard actions and editing surfaces for workspace controls.
use crate::primitives::{
    control::{UiLanguage, activate},
    editor::{Editor, InputMode},
    family::{Paint, Tone, face},
    glyph::{Glyph, icon},
};
use gpui::{prelude::*, *};

pub fn tr(cx: &App, en: &'static str, zh: &'static str) -> &'static str {
    UiLanguage::current(cx).text(en, zh)
}
pub fn column() -> Div {
    div().flex().flex_col().min_w_0().gap(px(8.))
}
pub fn row() -> Div {
    div().flex().items_center().min_w_0().gap(px(8.))
}
pub fn panel(id: &SharedString, width: f32, cx: &App) -> Stateful<Div> {
    let p = Paint::get(cx);
    column()
        .id(id.clone())
        .w_full()
        .max_w(px(width))
        .p(px(16.))
        .gap(px(12.))
        .rounded(crate::foundation::Theme::global(cx).tokens.radius.md)
        .border_1()
        .border_color(p.line)
        .bg(p.panel)
        .text_color(p.ink)
        .text_size(px(14.))
        .line_height(px(20.))
}
pub fn title(s: impl Into<SharedString>) -> Div {
    div()
        .min_w_0()
        .font_weight(FontWeight::SEMIBOLD)
        .child(s.into())
}
pub fn note(s: impl Into<SharedString>, cx: &App) -> Div {
    div()
        .min_w_0()
        .text_size(px(12.))
        .line_height(px(17.))
        .text_color(Paint::get(cx).secondary)
        .child(s.into())
}
pub fn divider(cx: &App) -> Div {
    div().w_full().h(px(1.)).bg(Paint::get(cx).line)
}
pub fn badge(s: impl Into<SharedString>, cx: &App) -> Div {
    div()
        .flex_none()
        .text_size(px(11.))
        .px(px(6.))
        .rounded(crate::foundation::surface_radius(cx))
        .bg(Paint::get(cx).stage)
        .child(s.into())
}
pub fn btn<T: 'static>(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    selected: bool,
    cx: &Context<T>,
    action: impl Fn(&mut T, &mut Window, &mut Context<T>) + Clone + 'static,
) -> Stateful<Div> {
    let s = label.into();
    let p = Paint::get(cx);
    let el = face(
        id,
        s.clone(),
        32.,
        crate::foundation::surface_radius(cx).into(),
        if selected { Tone::Primary } else { Tone::Clear },
        cx,
    )
    .px(px(9.))
    .text_size(px(12.))
    .when(!selected, |e| e.hover(move |s| s.bg(p.hover)))
    .child(s);
    activate(el, cx, action)
}
pub fn icon_btn<T: 'static>(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    g: Glyph,
    cx: &Context<T>,
    action: impl Fn(&mut T, &mut Window, &mut Context<T>) + Clone + 'static,
) -> Stateful<Div> {
    let p = Paint::get(cx);
    activate(
        face(
            id,
            label,
            30.,
            crate::foundation::surface_radius(cx).into(),
            Tone::Clear,
            cx,
        )
        .w(px(30.))
        .hover(move |s| s.bg(p.hover))
        .child(icon(g, 16., p.secondary)),
        cx,
        action,
    )
}
pub fn item<T: 'static>(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    cx: &Context<T>,
    action: impl Fn(&mut T, &mut Window, &mut Context<T>) + Clone + 'static,
) -> Stateful<Div> {
    let p = Paint::get(cx);
    let label = label.into();
    activate(
        row()
            .id(id)
            .debug_selector({
                let label = label.clone();
                move || label.to_string()
            })
            .aria_label(label)
            .role(Role::Button)
            .tab_index(0)
            .px(px(8.))
            .py(px(6.))
            .rounded(crate::foundation::surface_radius(cx))
            .cursor_pointer()
            .hover(move |s| s.bg(p.hover))
            .focus_visible(move |s| s.bg(p.hover).border_1().border_color(p.faint)),
        cx,
        action,
    )
}
pub fn initials(s: &str) -> String {
    s.split_whitespace()
        .filter_map(|n| n.chars().next())
        .take(2)
        .collect()
}
pub fn avatar(name: &str, i: usize, cx: &App) -> Div {
    let hue = [0x8a65ac, 0x327982, 0xab6b49, 0x5f71a1, 0x997536][i % 5];
    row()
        .size(px(28.))
        .justify_center()
        .rounded_full()
        .bg(rgb(hue).opacity(0.18))
        .text_color(Paint::get(cx).ink)
        .text_size(px(10.))
        .child(initials(name))
}
pub fn editor<T: 'static>(
    label: SharedString,
    mode: InputMode,
    w: &mut Window,
    cx: &mut Context<T>,
) -> Entity<Editor> {
    cx.new(|cx| {
        let mut e = Editor::new(label.clone(), mode, w, cx);
        e.set_placeholder(label);
        e
    })
}
pub fn input(e: &Entity<Editor>, cx: &App) -> Div {
    let p = Paint::get(cx);
    div()
        .w_full()
        .min_w_0()
        .h(px(36.))
        .flex()
        .items_center()
        .px(px(8.))
        .rounded(crate::foundation::surface_radius(cx))
        .border_1()
        .border_color(p.line)
        .bg(p.panel)
        .child(e.clone())
}
pub fn replace(e: &Entity<Editor>, s: impl Into<String>, w: &Window, cx: &mut App) {
    e.update(cx, |e, cx| e.set_value(s, w, cx))
}
pub fn bar(fraction: f32, color: Hsla, cx: &App) -> Div {
    div()
        .w_full()
        .h(px(5.))
        .rounded_full()
        .overflow_hidden()
        .bg(Paint::get(cx).stage)
        .child(
            div()
                .h_full()
                .w(relative(fraction.clamp(0., 1.)))
                .rounded_full()
                .bg(color),
        )
}

/// Reveal a newly mounted element once. Stable IDs keep unrelated rerenders from replaying it.
pub fn arrive<E: IntoElement + Styled + 'static>(
    element: E,
    id: impl Into<ElementId>,
    delay_ms: u64,
    rise: f32,
    cx: &App,
) -> AnyElement {
    if crate::primitives::family::disabled_motion(cx) {
        return element.into_any_element();
    }
    let total = (160 + delay_ms) as f32;
    element
        .with_animation(
            id,
            Animation::new(std::time::Duration::from_millis(160 + delay_ms)),
            move |el, t| {
                let t = ((t * total - delay_ms as f32) / 160.).clamp(0., 1.);
                el.relative().top(px(rise * (1. - t))).opacity(t)
            },
        )
        .into_any_element()
}

/// Retains the natural height of a disclosure while it closes. The spring can be
/// retargeted before settling and reads reduced motion from the same scene.
pub struct Fold {
    amount: f32,
    height: std::rc::Rc<std::cell::Cell<Pixels>>,
    open: bool,
}
impl Fold {
    pub fn get(
        key: impl Into<ElementId>,
        open: bool,
        estimate: f32,
        seconds: f32,
        w: &mut Window,
        cx: &mut App,
    ) -> Self {
        use crate::primitives::family::Slide;
        struct State {
            open: bool,
            phase: Slide,
            height: std::rc::Rc<std::cell::Cell<Pixels>>,
        }
        let state = w.use_keyed_state(key, cx, |_, _| State {
            open,
            phase: Slide::new(f32::from(open)),
            height: std::rc::Rc::new(std::cell::Cell::new(px(estimate))),
        });
        state.update(cx, |s, cx| {
            if s.open != open {
                s.open = open;
                s.phase.duration(f32::from(open), seconds, cx)
            }
            Self {
                amount: s.phase.get(w, cx).clamp(0., 1.),
                height: s.height.clone(),
                open,
            }
        })
    }
    pub fn mounted(&self) -> bool {
        self.open || self.amount > 0.001
    }
    pub fn wrap(self, parent_gap: f32, content: impl IntoElement) -> Div {
        let h = self.height.get();
        // The scrolling parent owns clipping once a disclosure is open. Shrinking
        // individual rows would trim the last action in every wrapped toolbar.
        let shell = div()
            .mt(px(-parent_gap * (1. - self.amount)))
            .w_full()
            .flex_none()
            .when(self.amount < 0.999, |v| {
                v.h(h * self.amount).overflow_hidden()
            })
            .opacity(self.amount);
        shell.child(
            div()
                .w_full()
                .flex_none()
                .on_children_prepainted(move |bounds, _, _| {
                    if let Some(b) = bounds.first()
                        && b.size.height > px(0.)
                    {
                        self.height.set(b.size.height);
                    }
                })
                .child(content),
        )
    }
}

/// Paint both labels during a compact status change, preserving the interrupted mix.
pub fn swap_label(
    key: impl Into<ElementId>,
    text: impl Into<SharedString>,
    w: &mut Window,
    cx: &mut App,
) -> Div {
    swap_label_after(key, text, 0., w, cx)
}
/// Delay the crossfade for later cards without shifting neighboring cards.
pub fn swap_label_after(
    key: impl Into<ElementId>,
    text: impl Into<SharedString>,
    delay: f32,
    w: &mut Window,
    cx: &mut App,
) -> Div {
    use crate::primitives::family::PhaseClock;
    let next = text.into();
    struct Labels {
        previous: Vec<(SharedString, f32)>,
        current: SharedString,
        clock: PhaseClock,
    }
    let state = w.use_keyed_state(key, cx, |_, _| Labels {
        previous: vec![],
        current: next.clone(),
        clock: Default::default(),
    });
    state.update(cx, |s, cx| {
        let age = (s.clock.time(0.16 + delay, w, cx) - delay).max(0.);
        if s.current != next {
            let fade = (1. - age / 0.10).clamp(0., 1.);
            for (_, weight) in &mut s.previous {
                *weight *= fade
            }
            s.previous.retain(|(_, alpha)| *alpha > 0.01);
            let weight = (age / 0.16).clamp(0., 1.);
            if weight > 0.01 {
                s.previous.push((s.current.clone(), weight))
            }
            s.current = next;
            s.clock.restart(cx);
        }
        let age = (s.clock.time(0.16 + delay, w, cx) - delay).max(0.);
        let enter = (age / 0.16).clamp(0., 1.);
        let exit = (1. - age / 0.10).clamp(0., 1.);
        if exit == 0. {
            s.previous.clear()
        }
        div()
            .relative()
            .child(
                div()
                    .id("incoming-label")
                    .debug_selector({
                        let n = s.current.clone();
                        move || format!("Incoming {n}")
                    })
                    .relative()
                    .top(px(2. * (1. - enter)))
                    .opacity(enter)
                    .child(s.current.clone()),
            )
            .children(s.previous.iter().map(|(old, a)| {
                div()
                    .absolute()
                    .top(px(-2. * (1. - exit)))
                    .left_0()
                    .opacity(a * exit)
                    .child(old.clone())
            }))
    })
}

/// Retarget a finite numeric transition once per value change.
pub fn animated_value(
    key: impl Into<ElementId>,
    value: f32,
    duration: f32,
    w: &mut Window,
    cx: &mut App,
) -> f32 {
    use crate::primitives::family::Slide;
    let state = w.use_keyed_state(key, cx, |_, _| (value, Slide::new(value)));
    state.update(cx, |s, cx| {
        if s.0 != value {
            s.0 = value;
            s.1.duration(value, duration, cx)
        }
        s.1.get(w, cx)
    })
}

/// Opacity belongs to each appended byte range, so the preceding words keep
/// fading independently when another partial result arrives.
pub fn stream_highlights(
    key: impl Into<ElementId>,
    value: &str,
    w: &mut Window,
    cx: &mut App,
) -> Vec<(std::ops::Range<usize>, HighlightStyle)> {
    use crate::primitives::family::PhaseClock;
    type Chunk = (std::ops::Range<usize>, PhaseClock);
    let state = w.use_keyed_state(key, cx, |_, _| (String::new(), Vec::<Chunk>::new()));
    state.update(cx, |s, cx| {
        if value != s.0 {
            let offset = if value.starts_with(&s.0) {
                s.0.len()
            } else {
                s.1.clear();
                0
            };
            if offset < value.len() {
                let mut clock = PhaseClock::default();
                clock.restart(cx);
                s.1.push((offset..value.len(), clock))
            }
            s.0.clear();
            s.0.push_str(value);
        }
        let ink = Paint::get(cx).ink;
        let mut spans = vec![];
        s.1.retain(|(range, clock)| {
            let t = clock.time(0.14, w, cx) / 0.14;
            if t < 1. {
                spans.push((
                    range.clone(),
                    HighlightStyle {
                        color: Some(ink.opacity(t)),
                        ..Default::default()
                    },
                ));
                true
            } else {
                false
            }
        });
        spans
    })
}
