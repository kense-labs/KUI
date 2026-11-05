//! A translucent, single-selection control with a fluid moving indicator.
use crate::primitives::{
    direct_touch::Contact,
    family::{Paint, PhaseClock, Slide, disabled_motion},
    glyph::{Glyph, icon},
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};

#[derive(Clone)]
pub struct GlassSegmentOption {
    pub value: SharedString,
    pub label: SharedString,
    pub icon: Option<Glyph>,
    pub badge: Option<SharedString>,
    pub disabled: bool,
}
impl GlassSegmentOption {
    pub fn new(value: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            icon: None,
            badge: None,
            disabled: false,
        }
    }
    pub fn with_icon(mut self, value: Glyph) -> Self {
        self.icon = Some(value);
        self
    }
    pub fn with_badge(mut self, value: impl Into<SharedString>) -> Self {
        self.badge = Some(value.into());
        self
    }
    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GlassSegmentChanged(pub SharedString);
/// Equal-width segments that can be selected by pointer, keyboard, or a drag of the selected item.
pub struct GlassSegmented {
    label: SharedString,
    options: Vec<GlassSegmentOption>,
    selected: Option<usize>,
    focus: Vec<FocusHandle>,
    left: Slide,
    right: Slide,
    bounds: Rc<Cell<Bounds<Pixels>>>,
    contact: Contact,
    drag: Option<(f32, f32)>,
    suppress_click: bool,
    velocity: f32,
    last: (f32, f32),
    clock: PhaseClock,
    small: bool,
    disabled: bool,
    hovered: Option<usize>,
}
impl EventEmitter<GlassSegmentChanged> for GlassSegmented {}
impl GlassSegmented {
    pub fn new(
        label: impl Into<SharedString>,
        options: Vec<GlassSegmentOption>,
        cx: &mut Context<Self>,
    ) -> Self {
        let selected = options.iter().position(|o| !o.disabled);
        let index = selected.unwrap_or(0) as f32;
        Self {
            label: label.into(),
            focus: options
                .iter()
                .map(|_| cx.focus_handle().tab_stop(true))
                .collect(),
            options,
            selected,
            left: Slide::snappy(index),
            right: Slide::snappy(index + 1.),
            bounds: Default::default(),
            contact: Default::default(),
            drag: None,
            suppress_click: false,
            velocity: 0.,
            last: (0., 0.),
            clock: Default::default(),
            small: false,
            disabled: false,
            hovered: None,
        }
    }
    pub fn with_value(mut self, value: &str) -> Self {
        if let Some(i) = self
            .options
            .iter()
            .position(|o| !o.disabled && o.value.as_ref() == value)
        {
            self.selected = Some(i);
            self.left.reset(i as f32);
            self.right.reset(i as f32 + 1.);
        }
        self
    }
    pub fn with_small(mut self, small: bool) -> Self {
        self.small = small;
        self
    }
    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
    pub fn value(&self) -> Option<&str> {
        self.selected.map(|i| self.options[i].value.as_ref())
    }
    pub fn set_value(&mut self, value: &str, cx: &mut Context<Self>) {
        if let Some(i) = self
            .options
            .iter()
            .position(|o| !o.disabled && o.value.as_ref() == value)
        {
            self.select(i, cx);
        }
    }
    fn select(&mut self, i: usize, cx: &mut Context<Self>) {
        if self.disabled || self.options.get(i).is_none_or(|o| o.disabled) {
            return;
        }
        let before = self.selected;
        self.selected = Some(i);
        let forward = self.left.value(cx) < i as f32;
        self.left
            .duration(i as f32, if forward { 0.5 } else { 0.26 }, cx);
        self.right
            .duration(i as f32 + 1., if forward { 0.26 } else { 0.5 }, cx);
        if before != Some(i) {
            cx.emit(GlassSegmentChanged(self.options[i].value.clone()));
        }
        cx.notify();
    }
    fn nearest(&self, x: f32) -> Option<usize> {
        self.options
            .iter()
            .enumerate()
            .filter(|(_, o)| !o.disabled)
            .min_by(|(a, _), (b, _)| ((*a as f32 - x).abs()).total_cmp(&((*b as f32 - x).abs())))
            .map(|(i, _)| i)
    }
    fn begin(&mut self, x: Pixels, cx: &mut Context<Self>) {
        self.suppress_click = false;
        let Some(i) = self.selected else {
            return;
        };
        let b = self.bounds.get();
        let width: f32 = b.size.width.into();
        if self.disabled || width <= 0. {
            return;
        }
        let x: f32 = x.into();
        let left: f32 = b.left().into();
        let at = ((x - left) / width * self.options.len() as f32).floor() as usize;
        if at != i {
            return;
        }
        let start = self.left.value(cx);
        self.drag = Some((x, start));
        self.velocity = 0.;
        self.clock.restart(cx);
        self.last = (0., start);
        cx.notify();
    }
    fn move_drag(&mut self, x: Pixels, cx: &mut Context<Self>) {
        let Some((start_x, start)) = self.drag else {
            return;
        };
        let width: f32 = self.bounds.get().size.width.into();
        if width <= 0. {
            return;
        }
        let x: f32 = x.into();
        if (x - start_x).abs() > 4. {
            self.suppress_click = true;
        }
        let target = (start + (x - start_x) / width * self.options.len() as f32)
            .clamp(0., self.options.len().saturating_sub(1) as f32);
        let now = self.clock.age(cx);
        let dt = now - self.last.0;
        if dt > 0.005 {
            self.velocity = ((target - self.last.1) / dt).clamp(-12., 12.);
            self.last = (now, target);
        }
        if disabled_motion(cx) {
            self.left.reset(target);
            self.right.reset(target + 1.);
        } else {
            self.left
                .duration(target, if self.velocity >= 0. { 0.22 } else { 0.12 }, cx);
            self.right.duration(
                target + 1.,
                if self.velocity >= 0. { 0.12 } else { 0.22 },
                cx,
            );
        }
        cx.notify();
    }
    fn end(&mut self, cancelled: bool, cx: &mut Context<Self>) {
        if self.drag.take().is_none() {
            return;
        }
        let projected = self.last.1
            + if cancelled || disabled_motion(cx) {
                0.
            } else {
                self.velocity * 0.12
            };
        let target = if cancelled {
            self.selected
        } else {
            self.nearest(projected)
        };
        if let Some(i) = target {
            self.select(i, cx);
        }
    }
}
impl Render for GlassSegmented {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let dark = crate::foundation::Theme::global(cx).mode == crate::foundation::ThemeMode::Dark;
        let radius = crate::foundation::Theme::global(cx).tokens.radius.full;
        let glass_shadow = vec![
            gpui::BoxShadow {
                color: white().opacity(if dark { 0.12 } else { 0.5 }),
                offset: gpui::point(px(0.), px(1.)),
                blur_radius: px(0.),
                spread_radius: px(0.),
                inset: true,
            },
            gpui::BoxShadow {
                color: black().opacity(if dark { 0.3 } else { 0.06 }),
                offset: gpui::point(px(0.), px(1.)),
                blur_radius: px(2.),
                spread_radius: px(0.),
                inset: false,
            },
            gpui::BoxShadow {
                color: black().opacity(if dark { 0.55 } else { 0.24 }),
                offset: gpui::point(px(0.), px(if dark { 12. } else { 10. })),
                blur_radius: px(if dark { 32. } else { 28. }),
                spread_radius: px(0.),
                inset: false,
            },
        ];
        let count = self.options.len().max(1) as f32;
        let l = self.left.get(w, cx).clamp(0., count);
        let r = self.right.get(w, cx).clamp(l, count);
        let stretch = (r - l - 1.).abs().min(1.);
        let bounds = self.bounds.clone();
        let mut root = div()
            .id("glass-segmented")
            .relative()
            .flex()
            .w_full()
            .max_w_full()
            .p(px(4.))
            .h(px(if self.small { 36. } else { 44. }))
            .rounded(radius)
            .border_1()
            .border_color(if dark {
                white().opacity(0.1)
            } else {
                p.ink.opacity(0.1)
            })
            .bg(p.panel.opacity(if dark { 0.52 } else { 0.58 }))
            .shadow(glass_shadow)
            .overflow_hidden()
            .role(Role::RadioGroup)
            .aria_label(self.label.clone())
            .child(
                canvas(move |b, _, _| bounds.set(b), |_, _, _, _| {})
                    .absolute()
                    .inset(px(4.)),
            )
            .when(self.selected.is_some(), |v| {
                v.child(
                    div().absolute().inset(px(4.)).child(
                        div()
                            .absolute()
                            .left(relative(l / count))
                            .w(relative((r - l).max(0.1) / count))
                            .top(relative(stretch * 0.09))
                            .h(relative(1. - stretch * 0.18))
                            .rounded(radius)
                            .bg(if dark {
                                white().opacity(0.1).blend(p.bright.opacity(0.3))
                            } else {
                                white().opacity(0.82).blend(p.bright.opacity(0.14))
                            })
                            .border_1()
                            .border_color(white().opacity(if dark { 0.16 } else { 0.9 })),
                    ),
                )
            });
        for (i, option) in self.options.iter().enumerate() {
            let active = self.selected == Some(i);
            let disabled = self.disabled || option.disabled;
            let ink = if active {
                p.ink.blend(p.bright.opacity(0.7))
            } else if self.hovered == Some(i) {
                p.ink
            } else {
                p.secondary
            };
            let cell = div()
                .id(("glass-segment", i))
                .debug_selector({
                    let label = option.label.clone();
                    move || label.to_string()
                })
                .relative()
                .flex_1()
                .min_w(px(0.))
                .h_full()
                .flex()
                .items_center()
                .justify_center()
                .gap(px(6.))
                .px(px(if self.small { 12. } else { 16. }))
                .rounded(radius)
                .text_size(px(if self.small { 12. } else { 14. }))
                .font_weight(FontWeight::MEDIUM)
                .text_color(ink)
                .when(!active && !disabled, |v| v.hover(|s| s.text_color(p.ink)))
                .on_hover(cx.listener(move |s, over: &bool, _, cx| {
                    if disabled {
                        return;
                    }
                    s.hovered = if *over { Some(i) } else { None };
                    cx.notify();
                }))
                .track_focus(&self.focus[i])
                .tab_index(if active && !disabled { 0 } else { -1 })
                .role(Role::RadioButton)
                .aria_label(option.label.clone())
                .aria_toggled(active.into())
                .focus_visible(move |s| s.border_1().border_color(p.focus))
                .when_some(option.icon, |v, glyph| v.child(icon(glyph, 16., ink)))
                .child(option.label.clone())
                .when_some(option.badge.clone(), |v, badge| {
                    v.child(
                        div()
                            .h(px(18.))
                            .min_w(px(18.))
                            .px(px(5.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_full()
                            .text_size(px(12.))
                            .child(badge),
                    )
                })
                .when(disabled, |v| v.opacity(0.4))
                .when(!disabled, |v| {
                    v.cursor_pointer()
                        .on_click(cx.listener(move |s, _, w, cx| {
                            if s.suppress_click {
                                s.suppress_click = false;
                                return;
                            }
                            s.select(i, cx);
                            s.focus[i].focus(w, cx);
                        }))
                        .on_key_down(cx.listener(move |s, e: &KeyDownEvent, w, cx| {
                            let enabled: Vec<usize> = s
                                .options
                                .iter()
                                .enumerate()
                                .filter_map(|(j, o)| (!o.disabled).then_some(j))
                                .collect();
                            if enabled.is_empty() {
                                return;
                            }
                            let current = enabled.iter().position(|j| *j == i).unwrap_or(0);
                            let j = match e.keystroke.key.as_str() {
                                "left" | "up" => {
                                    enabled[(current + enabled.len() - 1) % enabled.len()]
                                }
                                "right" | "down" => enabled[(current + 1) % enabled.len()],
                                "home" => enabled[0],
                                "end" => enabled[enabled.len() - 1],
                                _ => return,
                            };
                            s.select(j, cx);
                            s.focus[j].focus(w, cx);
                            w.prevent_default();
                            cx.stop_propagation();
                        }))
                });
            root = root.child(cell);
        }
        root = root
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|s, e: &MouseDownEvent, _, cx| s.begin(e.position.x, cx)),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|s, _: &MouseUpEvent, _, cx| s.end(false, cx)),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|s, _: &MouseUpEvent, _, cx| s.end(false, cx)),
            )
            .when(self.drag.is_some(), |v| {
                let moved = cx.listener(|s, e: &MouseMoveEvent, _, cx| {
                    if e.pressed_button == Some(MouseButton::Left) {
                        s.move_drag(e.position.x, cx);
                    }
                });
                let released = cx.listener(|s, _: &MouseUpEvent, _, cx| s.end(false, cx));
                v.child(
                    canvas(
                        |_, _, _| (),
                        move |_, _, w, _| {
                            w.on_mouse_event(move |e: &MouseMoveEvent, phase, w, cx| {
                                if phase.capture() {
                                    moved(e, w, cx);
                                }
                            });
                            w.on_mouse_event(move |e: &MouseUpEvent, phase, w, cx| {
                                if phase.capture() && e.button == MouseButton::Left {
                                    released(e, w, cx);
                                }
                            });
                        },
                    )
                    .absolute()
                    .size(px(0.)),
                )
            })
            .child(self.contact.listen(cx, |s, e, _, cx| match e.phase {
                TouchPhase::Started => s.begin(e.position.x, cx),
                TouchPhase::Moved => s.move_drag(e.position.x, cx),
                TouchPhase::Ended => s.end(false, cx),
                TouchPhase::Cancelled => s.end(true, cx),
            }));
        root
    }
}

#[cfg(test)]
mod tests {
    use super::{GlassSegmentOption, GlassSegmented};
    use gpui::{
        Modifiers, MouseButton, TestAppContext, TouchEvent, TouchId, TouchPhase, VisualTestContext,
    };
    use std::time::Duration;
    fn tick(w: &mut VisualTestContext, ms: u64) {
        w.executor().advance_clock(Duration::from_millis(ms));
        w.run_until_parked();
        w.update(|w, cx| {
            w.simulate_next_frame(cx);
            w.refresh();
        });
        w.run_until_parked();
    }
    #[test]
    fn keyboard_skips_disabled_and_drag_commits_an_enabled_option() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (control, mut w) = app.add_window_view(|_, cx| {
            GlassSegmented::new(
                "View",
                vec![
                    GlassSegmentOption::new("first", "First"),
                    GlassSegmentOption::new("blocked", "Blocked").with_disabled(true),
                    GlassSegmentOption::new("last", "Last"),
                ],
                cx,
            )
        });
        let first = w.debug_bounds("First").expect("first option").center();
        let last = w.debug_bounds("Last").expect("last option").center();
        w.simulate_click(first, Modifiers::default());
        w.simulate_keystrokes("right");
        tick(&mut w, 400);
        assert_eq!(
            w.update(|_, cx| control.read(cx).value().map(str::to_owned))
                .as_deref(),
            Some("last")
        );
        w.simulate_keystrokes("home");
        tick(&mut w, 400);
        w.simulate_mouse_down(first, MouseButton::Left, Modifiers::default());
        tick(&mut w, 16);
        w.simulate_mouse_move(last, Some(MouseButton::Left), Modifiers::default());
        tick(&mut w, 80);
        w.simulate_mouse_up(last, MouseButton::Left, Modifiers::default());
        tick(&mut w, 400);
        assert_eq!(
            w.update(|_, cx| control.read(cx).value().map(str::to_owned))
                .as_deref(),
            Some("last")
        );
    }
    #[test]
    fn cancelled_touch_does_not_change_selection() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (control, mut w) = app.add_window_view(|_, cx| {
            GlassSegmented::new(
                "View",
                vec![
                    GlassSegmentOption::new("first", "First"),
                    GlassSegmentOption::new("last", "Last"),
                ],
                cx,
            )
        });
        let first = w.debug_bounds("First").expect("first option").center();
        let last = w.debug_bounds("Last").expect("last option").center();
        for (phase, position) in [
            (TouchPhase::Started, first),
            (TouchPhase::Moved, last),
            (TouchPhase::Cancelled, last),
        ] {
            w.simulate_event(TouchEvent {
                id: TouchId(1),
                phase,
                position,
                ..Default::default()
            });
            tick(&mut w, 16);
        }
        tick(&mut w, 400);
        assert_eq!(
            w.update(|_, cx| control.read(cx).value().map(str::to_owned))
                .as_deref(),
            Some("first")
        );
    }
}
