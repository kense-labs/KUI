//! Building blocks for collection navigation and contextual disclosures.
use crate::primitives::{
    control::UiLanguage,
    family::{Paint, Slide},
    flyout::TriggerBounds,
    glyph::{Glyph, icon},
};
use gpui::{prelude::*, *};
use std::time::Duration;
pub fn tr(cx: &App, en: &'static str, zh: &'static str) -> &'static str {
    UiLanguage::current(cx).text(en, zh)
}
pub fn column() -> Div {
    div().flex().flex_col().min_w_0()
}
pub fn row() -> Div {
    div().flex().items_center().min_w_0()
}
pub fn panel(cx: &App) -> Div {
    let p = Paint::get(cx);
    column()
        .bg(p.panel)
        .text_color(p.ink)
        .border_1()
        .border_color(p.line)
        .rounded(crate::foundation::surface_radius(cx))
        .overflow_hidden()
}
pub fn label(text: impl Into<SharedString>, font: f32, color: Hsla) -> Div {
    div()
        .text_size(px(font))
        .line_height(px(font * 1.42))
        .text_color(color)
        .child(text.into())
}
pub fn button(id: impl Into<ElementId>, name: impl Into<SharedString>, cx: &App) -> Stateful<Div> {
    let p = Paint::get(cx);
    let name = name.into();
    let debug = name.clone();
    row()
        .id(id)
        .debug_selector(move || debug.to_string())
        .role(Role::Button)
        .aria_label(name)
        .tab_index(0)
        .justify_center()
        .gap(px(6.))
        .h(px(34.))
        .px(px(10.))
        .rounded(crate::foundation::surface_radius(cx))
        .text_size(px(13.))
        .text_color(p.ink)
        .cursor_pointer()
        .hover(move |s| s.bg(p.hover))
        .active(move |s| s.bg(p.line))
        .focus_visible(move |s| s.border_1().border_color(p.secondary))
}
pub fn fit(w: &Window, ideal: f32) -> Pixels {
    (w.viewport_size().width - px(24.))
        .min(px(ideal))
        .max(px(1.))
}
pub type Content = Box<dyn Fn(&mut Window, &mut App) -> AnyElement>;
pub fn overlay(
    body: impl IntoElement,
    position: Point<Pixels>,
    anchor: Anchor,
) -> impl IntoElement {
    deferred(
        anchored()
            .position(position)
            .anchor(anchor)
            .snap_to_window_with_margin(px(8.))
            .child(body),
    )
    .with_priority(2)
}
#[derive(Default)]
struct HintWarmth([Option<web_time::Instant>; 2]);
impl Global for HintWarmth {}
pub struct Hint {
    pub name: SharedString,
    pub text: SharedString,
    pub body: Option<Content>,
    pub marker: Option<Glyph>,
    pub badge: Option<SharedString>,
    focus: FocusHandle,
    bounds: TriggerBounds,
    presence: Slide,
    open: bool,
    epoch: u64,
    over: bool,
    over_panel: bool,
    fade: Fade,
    focused: bool,
    instant: bool,
}
impl Hint {
    pub fn new(name: impl Into<SharedString>, text: impl Into<SharedString>, cx: &App) -> Self {
        Self {
            name: name.into(),
            text: text.into(),
            body: None,
            marker: None,
            badge: None,
            focus: cx.focus_handle().tab_stop(true),
            bounds: Default::default(),
            presence: Slide::new(0.),
            open: false,
            epoch: 0,
            over: false,
            over_panel: false,
            fade: Default::default(),
            focused: false,
            instant: false,
        }
    }
    fn schedule(&mut self, open: bool, delay: u64, cx: &mut Context<Self>) {
        self.epoch += 1;
        let epoch = self.epoch;
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(delay))
                .await;
            let _ = this.update(cx, |v, cx| {
                if v.epoch == epoch {
                    v.show(open, cx)
                }
            });
        })
        .detach();
    }
    fn show(&mut self, open: bool, cx: &mut Context<Self>) {
        self.epoch += 1;
        if self.open != open {
            self.open = open;
            let now = cx.background_executor().now();
            if !cx.has_global::<HintWarmth>() {
                cx.set_global(HintWarmth::default());
            }
            cx.global_mut::<HintWarmth>().0[usize::from(self.body.is_some())] = Some(now);
            let seconds = match (self.body.is_some(), open) {
                (true, true) => 0.16,
                (true, false) => 0.12,
                (false, true) => 0.15,
                (false, false) => 0.09,
            };
            self.fade.set(
                open,
                if crate::primitives::family::disabled_motion(cx) {
                    0.09
                } else {
                    seconds
                },
                cx,
            );
            self.presence.duration(
                if open { 1. } else { 0. },
                if open { 0.15 } else { 0.10 },
                cx,
            );
            cx.notify()
        }
    }
}
impl Render for Hint {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let card = self.body.is_some();
        let focus = self.focus.clone();
        let mut trigger = button("hint", self.name.clone(), cx)
            .relative()
            .track_focus(&self.focus)
            .bg(p.panel)
            .border_1()
            .border_color(p.line)
            .when(!card, |b| b.size(px(32.)).p_0())
            .when(card, |b| b.h(px(36.)))
            .on_hover(cx.listener(|v, on: &bool, _, cx| {
                v.over = *on;
                if *on {
                    let card = v.body.is_some();
                    let now = cx.background_executor().now();
                    v.instant = cx
                        .try_global::<HintWarmth>()
                        .and_then(|state| state.0[usize::from(card)])
                        .is_some_and(|at| now.saturating_duration_since(at).as_millis() < 300);
                    let delay = match (card, v.instant) {
                        (true, true) => 80,
                        (true, false) => 500,
                        (false, true) => 0,
                        (false, false) => 250,
                    };
                    v.schedule(true, delay, cx)
                } else if !v.over_panel {
                    v.schedule(false, if v.body.is_some() { 140 } else { 0 }, cx)
                }
            }))
            .on_click(cx.listener(|v, _, _, cx| v.show(!v.open, cx)))
            .on_key_down(cx.listener(|v, e: &KeyDownEvent, _, cx| {
                if e.keystroke.key == "escape" {
                    v.show(false, cx)
                }
            }))
            .child(self.bounds.measure());
        if let Some(mark) = self.marker {
            trigger = trigger.child(icon(mark, 16., p.secondary))
        }
        if let Some(initials) = &self.badge {
            trigger = trigger.child(initials.clone())
        }
        if card {
            trigger = trigger.child(self.name.clone())
        }
        // Register once per window through the focus handle's state; keyboard focus opens immediately.
        let keyboard = focus.is_focused(w) && w.last_input_was_keyboard();
        if keyboard != self.focused {
            self.focused = keyboard;
            if keyboard || (!self.over && !self.over_panel) {
                self.show(keyboard, cx)
            }
        }
        let mut root = div().child(trigger);
        let a = if self.instant || crate::primitives::family::disabled_motion(cx) {
            1.
        } else {
            self.presence.get(w, cx).clamp(0., 1.)
        };
        let opacity = self.fade.get(w, cx);
        if self.open || opacity > 0.001 {
            let b = self.bounds.get();
            let mut box_el = panel(cx)
                .id("hint-panel")
                .role(if card { Role::Group } else { Role::Tooltip })
                .p(px(if card { 16. } else { 9. }))
                .shadow_md()
                .opacity(opacity)
                .max_w(fit(w, if card { 280. } else { 245. }))
                .occlude()
                .on_hover(cx.listener(|v, on: &bool, _, cx| {
                    v.over_panel = *on;
                    if *on {
                        v.schedule(true, 0, cx)
                    } else if !v.over {
                        v.schedule(false, 120, cx)
                    }
                }));
            box_el = if let Some(body) = &self.body {
                box_el.child(body(w, cx))
            } else {
                box_el.child(label(self.text.clone(), 12., p.ink))
            };
            let (pos, anchor) = if card {
                self.bounds
                    .place(w.viewport_size(), px(115.), px(6. + (1. - a) * 3.))
            } else {
                (
                    point(b.center().x, b.top() - px(6. + (1. - a) * 3.)),
                    Anchor::BottomCenter,
                )
            };
            root = root.child(overlay(box_el, pos, anchor));
        }
        root
    }
}
/// Animate to the last measured natural height. The inner column remains at its
/// intrinsic height while the outer viewport clips it during a transition.
pub struct Fold {
    slide: Slide,
    height: std::rc::Rc<std::cell::Cell<Pixels>>,
    pub open: bool,
}
impl Fold {
    pub fn new(open: bool, estimate: f32) -> Self {
        Self {
            open,
            slide: Slide::new(if open { 1. } else { 0. }),
            height: std::rc::Rc::new(std::cell::Cell::new(px(estimate))),
        }
    }
    pub fn fraction(&self, w: &Window, cx: &App) -> f32 {
        self.slide.get(w, cx).clamp(0., 1.)
    }
    pub fn set(&mut self, open: bool, cx: &App) {
        self.set_duration(open, 0.22, cx)
    }
    pub(crate) fn set_duration(&mut self, open: bool, seconds: f32, cx: &App) {
        self.open = open;
        self.slide.duration(if open { 1. } else { 0. }, seconds, cx)
    }
    pub fn render(&mut self, body: impl IntoElement, w: &mut Window, cx: &mut App) -> Div {
        let a = self.slide.get(w, cx).clamp(0., 1.);
        let saved = self.height.clone();
        column()
            .flex_none()
            .when(a < 0.999, |d| d.h(self.height.get() * a).overflow_hidden())
            .child(
                column()
                    .relative()
                    .top(px(-6. * (1. - a)))
                    .opacity(a)
                    .flex_none()
                    .child(body),
            )
            .on_children_prepainted(move |bounds, _, _| {
                if let Some(b) = bounds.first() {
                    saved.set(b.size.height)
                }
            })
    }
}

/// Optional embed hook for trackpad arbitration at horizontal scroll boundaries.
pub struct HorizontalScrollRegion(pub fn(Bounds<Pixels>, bool, bool));
impl Global for HorizontalScrollRegion {}
pub fn horizontal_viewport(
    id: impl Into<ElementId>,
    handle: &ScrollHandle,
    content: impl IntoElement,
) -> Div {
    let scroll = handle.clone();
    div()
        .relative()
        .child(
            div()
                .id(id)
                .overflow_x_scroll()
                .track_scroll(handle)
                .child(content),
        )
        .child(
            canvas(
                |_, _, _| (),
                move |bounds, _, _, cx| {
                    if let Some(host) = cx.try_global::<HorizontalScrollRegion>() {
                        (host.0)(
                            bounds,
                            -scroll.offset().x > px(0.1),
                            -scroll.offset().x + px(0.1) < scroll.max_offset().x,
                        )
                    }
                },
            )
            .absolute()
            .inset_0(),
        )
}

/// Disclosures retain departing descendants until the clipped row has closed.
/// Both the JSON inspector and file tree keep document order between updates.
#[derive(Clone, Copy)]
pub(crate) struct RowFrame {
    pub height: f32,
    pub opacity: f32,
}
pub(crate) struct BranchRows<T> {
    slots: Vec<(T, Slide, Fade)>,
    smooth_height: bool,
    mounted: bool,
}
impl<T> Default for BranchRows<T> {
    fn default() -> Self {
        Self {
            slots: Vec::new(),
            mounted: false,
            smooth_height: false,
        }
    }
}
impl<T: Clone> BranchRows<T> {
    /// Use a 400ms visual spring for geometry and a separate 160ms text fade.
    pub(crate) fn smooth_height() -> Self {
        Self {
            smooth_height: true,
            ..Self::default()
        }
    }
    pub fn sync<K: Ord>(
        &mut self,
        next: Vec<T>,
        key: impl Fn(&T) -> K,
        w: &Window,
        cx: &App,
    ) -> Vec<(T, RowFrame, bool)> {
        use std::collections::BTreeMap;
        if !self.mounted
            || next.len().max(self.slots.len()) > 400
            || crate::primitives::family::disabled_motion(cx)
        {
            self.mounted = true;
            self.slots = next
                .iter()
                .cloned()
                .map(|v| (v, Slide::new(1.), Fade::visible()))
                .collect();
            return next
                .into_iter()
                .map(|v| {
                    (
                        v,
                        RowFrame {
                            height: 1.,
                            opacity: 1.,
                        },
                        true,
                    )
                })
                .collect();
        }
        let index: BTreeMap<_, _> = next.iter().enumerate().map(|(i, v)| (key(v), i)).collect();
        let smooth = self.smooth_height;
        let frame = |slide: Slide, fade: &mut Fade, present: bool| {
            let height = slide.get(w, cx).clamp(0., 1.);
            let opacity = if smooth {
                fade.eased = true;
                fade.set(present, 0.16, cx);
                fade.get(w, cx)
            } else {
                height
            };
            RowFrame { height, opacity }
        };
        let mut live = BTreeMap::new();
        let mut departing: Vec<Vec<_>> = (0..=next.len()).map(|_| Vec::new()).collect();
        let mut anchor = next.len();
        for (old, mut slide, fade) in std::mem::take(&mut self.slots).into_iter().rev() {
            let k = key(&old);
            if let Some(&i) = index.get(&k) {
                anchor = i;
                live.insert(k, (slide, fade));
            } else {
                slide.duration(0., if smooth { 0.4 } else { 0.16 }, cx);
                if slide.get(w, cx) > 0.001 {
                    departing[anchor].push((old, slide, fade));
                }
            }
        }
        let mut rendered = Vec::new();
        let mut next = next.into_iter();
        for group in departing {
            for (slot, slide, mut fade) in group.into_iter().rev() {
                rendered.push((slot.clone(), frame(slide, &mut fade, false), false));
                self.slots.push((slot, slide, fade));
            }
            if let Some(slot) = next.next() {
                let (mut slide, mut fade) = live
                    .remove(&key(&slot))
                    .unwrap_or_else(|| (Slide::new(0.), Fade::default()));
                slide.duration(1., if smooth { 0.4 } else { 0.24 }, cx);
                rendered.push((slot.clone(), frame(slide, &mut fade, true), true));
                self.slots.push((slot, slide, fade));
            }
        }
        rendered
    }
}

/// Opacity-only feedback also runs when the host requests reduced motion.
#[derive(Clone, Copy, Default)]
pub(crate) struct Fade {
    eased: bool,
    from: f32,
    to: f32,
    start: Option<web_time::Instant>,
    seconds: f32,
}
impl Fade {
    fn visible() -> Self {
        Self {
            from: 1.,
            to: 1.,
            ..Self::default()
        }
    }
    fn at(&self, cx: &App) -> f32 {
        self.start.map_or(1., |time| {
            (cx.background_executor()
                .now()
                .saturating_duration_since(time)
                .as_secs_f32()
                / self.seconds)
                .min(1.)
        })
    }
    fn sample(&self, t: f32) -> f32 {
        let progress = if self.eased {
            crate::primitives::family::standard_ease(t)
        } else {
            t
        };
        self.from + (self.to - self.from) * progress
    }
    pub fn set(&mut self, visible: bool, seconds: f32, cx: &App) {
        let to = if visible { 1. } else { 0. };
        if to == self.to {
            return;
        }
        self.from = self.sample(self.at(cx));
        self.to = to;
        self.start = Some(cx.background_executor().now());
        self.seconds = seconds.max(0.001);
    }
    pub fn get(&self, w: &Window, cx: &App) -> f32 {
        let t = self.at(cx);
        if t < 1. {
            w.request_animation_frame();
        }
        self.sample(t)
    }
}
