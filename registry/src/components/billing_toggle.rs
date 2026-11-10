//! A two-period pricing control with stable, keyboard-accessible selection and a morphing savings badge.
use crate::foundation::{Theme, ThemeMode};
use crate::primitives::family::{
    Paint, PhaseClock, Slide, Tone, ease, face, measure, standard_ease,
};
use gpui::{prelude::*, *};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

struct Highlight {
    x: Slide,
    width: Slide,
    target: Option<(usize, f32, f32)>,
}
impl Default for Highlight {
    fn default() -> Self {
        Self {
            x: Slide::new(0.),
            width: Slide::new(0.),
            target: None,
        }
    }
}
impl Highlight {
    fn rect(
        &mut self,
        selected: usize,
        target: Bounds<Pixels>,
        surface: Bounds<Pixels>,
        w: &Window,
        cx: &App,
    ) -> Bounds<Pixels> {
        let (bounds, moving) = self.geometry(selected, target, surface, cx);
        if moving {
            w.request_animation_frame();
        }
        bounds
    }
    fn geometry(
        &mut self,
        selected: usize,
        target: Bounds<Pixels>,
        surface: Bounds<Pixels>,
        cx: &App,
    ) -> (Bounds<Pixels>, bool) {
        let x = f32::from(target.left() - surface.left());
        let width = f32::from(target.size.width);
        let old = self.target;
        if old.is_none()
            || old.is_some_and(|(index, ox, ow)| index == selected && (ox != x || ow != width))
        {
            self.x.reset(x);
            self.width.reset(width);
        } else if old.is_some_and(|(index, _, _)| index != selected) {
            self.x.duration(x, 0.34, cx);
            self.width.duration(width, 0.34, cx);
        }
        self.target = Some((selected, x, width));
        let now = cx.background_executor().now();
        let reduced = crate::primitives::family::disabled_motion(cx);
        let (x, moving_x) = self.x.sample_batch(now, reduced);
        let (width, moving_width) = self.width.sample_batch(now, reduced);
        (
            Bounds::new(
                point(surface.left() + px(x), target.top()),
                size(px(width), target.size.height),
            )
            .intersect(&surface),
            moving_x || moving_width,
        )
    }
}

/// The period chosen for billing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BillingPeriod {
    #[default]
    Monthly,
    Yearly,
}
/// Emitted only when the selected period changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BillingChanged(pub BillingPeriod);

/// A monthly/yearly radio control. The host owns price and checkout behavior.
pub struct BillingToggle {
    label: SharedString,
    hide_label: bool,
    monthly: SharedString,
    yearly: SharedString,
    badge: SharedString,
    active_badge: Option<SharedString>,
    period: BillingPeriod,
    highlight: Rc<RefCell<Highlight>>,
    badge_clock: PhaseClock,
    focus: [FocusHandle; 2],
    disabled: bool,
    inline_badge: bool,
}
impl EventEmitter<BillingChanged> for BillingToggle {}
impl BillingToggle {
    pub fn new(label: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        Self {
            label: label.into(),
            hide_label: false,
            monthly: "Monthly".into(),
            yearly: "Yearly".into(),
            badge: "Save 20%".into(),
            active_badge: None,
            period: BillingPeriod::Monthly,
            highlight: Default::default(),
            badge_clock: Default::default(),
            focus: [
                cx.focus_handle().tab_stop(true),
                cx.focus_handle().tab_stop(true),
            ],
            disabled: false,
            inline_badge: false,
        }
    }
    pub fn with_hide_label(mut self, hide_label: bool) -> Self {
        self.hide_label = hide_label;
        self
    }
    pub fn with_labels(
        mut self,
        monthly: impl Into<SharedString>,
        yearly: impl Into<SharedString>,
    ) -> Self {
        self.monthly = monthly.into();
        self.yearly = yearly.into();
        self
    }
    pub fn with_savings(mut self, badge: impl Into<SharedString>) -> Self {
        self.badge = badge.into();
        self
    }
    pub fn with_active_savings(mut self, active_badge: impl Into<SharedString>) -> Self {
        self.active_badge = Some(active_badge.into());
        self
    }
    pub fn with_period(mut self, period: BillingPeriod) -> Self {
        self.period = period;
        self
    }
    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
    #[allow(dead_code)]
    pub(crate) fn with_inline_badge(mut self) -> Self {
        self.inline_badge = true;
        self
    }
    #[allow(dead_code)]
    pub(crate) fn set_disabled(&mut self, disabled: bool, cx: &mut Context<Self>) {
        if self.disabled != disabled {
            self.disabled = disabled;
            cx.notify();
        }
    }
    pub fn period(&self) -> BillingPeriod {
        self.period
    }
    pub fn set_period(&mut self, period: BillingPeriod, cx: &mut Context<Self>) {
        if self.period != period {
            self.period = period;
            self.badge_clock.restart(cx);
            cx.notify();
        }
    }
    fn choose(&mut self, period: BillingPeriod, w: &mut Window, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        self.focus[usize::from(period == BillingPeriod::Yearly)].focus(w, cx);
        if self.period != period {
            self.set_period(period, cx);
            cx.emit(BillingChanged(period));
        }
    }
    fn active_badge_text(&self) -> SharedString {
        if let Some(active) = &self.active_badge {
            return active.clone();
        }
        self.badge.clone()
    }
}
impl Render for BillingToggle {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let radius = Theme::global(cx).tokens.radius.full;
        let selected = usize::from(self.period == BillingPeriod::Yearly);
        let target = Rc::new(Cell::new(None::<Bounds<Pixels>>));
        let target_for_paint = target.clone();
        let highlight = self.highlight.clone();
        let mut options = div()
            .relative()
            .flex()
            .items_center()
            .p(px(3.))
            .rounded(radius)
            .border_1()
            .border_color(p.line)
            .bg(if self.inline_badge {
                p.panel.blend(p.ink.opacity(0.05))
            } else {
                p.tray
            })
            .child(
                canvas(
                    |_, _, _| (),
                    move |surface, _, w, cx| {
                        let Some(target) = target_for_paint.get() else {
                            return;
                        };
                        let rect = highlight
                            .borrow_mut()
                            .rect(selected, target, surface, w, cx);
                        if rect.size.width > px(0.) && rect.size.height > px(0.) {
                            let r = Theme::global(cx)
                                .tokens
                                .radius
                                .full
                                .min(rect.size.width / 2.)
                                .min(rect.size.height / 2.);
                            let dark = Theme::global(cx).mode == ThemeMode::Dark;
                            let mut wash = |shift: f32, grow: f32, color: Hsla| {
                                w.paint_quad(
                                    fill(
                                        Bounds::new(
                                            point(
                                                rect.origin.x - px(grow),
                                                rect.origin.y + px(shift - grow),
                                            ),
                                            size(
                                                rect.size.width + px(grow * 2.),
                                                rect.size.height + px(grow * 2.),
                                            ),
                                        ),
                                        color,
                                    )
                                    .corner_radii(r + px(grow)),
                                );
                            };
                            if dark {
                                wash(1., 1., black().opacity(0.3));
                            } else {
                                wash(2., 3., black().opacity(0.04));
                                wash(1., 1., black().opacity(0.06));
                            }
                            w.paint_quad(quad(
                                rect,
                                Corners::all(r),
                                p.panel,
                                Edges::all(px(1.)),
                                if dark { p.line } else { p.line.opacity(0.72) },
                                BorderStyle::Solid,
                            ));
                            if dark {
                                w.paint_quad(fill(
                                    Bounds::new(
                                        point(rect.origin.x + r, rect.origin.y + px(1.)),
                                        size((rect.size.width - r * 2.).max(px(0.)), px(1.)),
                                    ),
                                    hsla(0., 0., 1., 0.05),
                                ));
                            }
                        }
                    },
                )
                .absolute()
                .inset_0(),
            );
        let age = self.badge_clock.time(0.24, w, cx);
        let enter = ease(age / 0.24);
        let exit = standard_ease(age / 0.16);
        let tint = standard_ease(age / 0.24);
        let dark = Theme::global(cx).mode == ThemeMode::Dark;
        let mut row = div().flex().items_center();
        for (i, (period, name)) in [
            (BillingPeriod::Monthly, self.monthly.clone()),
            (BillingPeriod::Yearly, self.yearly.clone()),
        ]
        .into_iter()
        .enumerate()
        {
            let active = self.period == period;
            let has_badge = period == BillingPeriod::Yearly && !self.badge.is_empty();
            let inactive_badge = self.badge.clone();
            let active_badge = self.active_badge_text();
            let shown_badge = if active {
                active_badge.clone()
            } else {
                inactive_badge.clone()
            };
            let prev_badge = if active { inactive_badge } else { active_badge };
            let aria_text = if has_badge {
                format!("{} · {}", name, shown_badge)
            } else {
                name.to_string()
            };
            let mut item = face(
                ("billing-period", i),
                aria_text,
                if self.inline_badge { 36. } else { 34. },
                radius.into(),
                Tone::Clear,
                cx,
            )
            .pl(px(if self.inline_badge { 12. } else { 14. }))
            .pr(px(if self.inline_badge && has_badge {
                16.
            } else if self.inline_badge {
                12.
            } else if has_badge {
                5.
            } else {
                14.
            }))
            .gap(px(8.))
            .track_focus(&self.focus[i])
            .tab_index(if active && !self.disabled { 0 } else { -1 })
            .role(Role::RadioButton)
            .aria_toggled(active.into())
            .line_height(px(14.))
            .font_weight(FontWeight::MEDIUM)
            .text_color(if active { p.ink } else { p.faint })
            .when(active, |v| v.cursor_default())
            .when(!active, |v| v.hover(|s| s.text_color(p.secondary)))
            .child(name);
            if has_badge {
                let badge_width = measure(&prev_badge, 12., w).max(measure(&shown_badge, 12., w));
                let shown_tint = if active { tint } else { 1. - tint };
                let rest_bg = p.ink.opacity(0.06);
                let on_bg = p.success.opacity(if dark { 0.18 } else { 0.13 });
                let on_fg = if dark {
                    p.success
                } else {
                    p.ink.blend(p.success.opacity(0.88))
                };
                let mut badge_pill = div()
                    .h(px(24.))
                    .w(px(badge_width + 16.))
                    .px(px(8.))
                    .rounded(radius)
                    .bg(rest_bg.blend(on_bg.opacity(shown_tint)))
                    .text_color(p.secondary.blend(on_fg.opacity(shown_tint)))
                    .text_size(px(12.))
                    .line_height(px(16.))
                    .font_weight(FontWeight::MEDIUM)
                    .flex()
                    .items_center()
                    .justify_center()
                    .overflow_hidden()
                    .relative()
                    .child(
                        div()
                            .relative()
                            .top(px((1. - enter) * 5.4))
                            .opacity(enter)
                            .child(shown_badge.clone()),
                    );
                if exit < 1. && prev_badge != shown_badge {
                    badge_pill = badge_pill.child(
                        div()
                            .absolute()
                            .top(px(-exit * 5.4))
                            .opacity(1. - exit)
                            .child(prev_badge),
                    );
                }
                item = item.child(if self.inline_badge {
                    div()
                        .flex_none()
                        .text_size(px(12.))
                        .text_color(p.secondary)
                        .child(shown_badge)
                } else {
                    badge_pill
                });
            }
            if !self.disabled {
                item = item
                    .on_click(cx.listener(move |s, _, w, cx| s.choose(period, w, cx)))
                    .on_key_down(cx.listener(move |s, e: &KeyDownEvent, w, cx| {
                        let next = match e.keystroke.key.as_str() {
                            "left" | "up" | "home" => BillingPeriod::Monthly,
                            "right" | "down" | "end" => BillingPeriod::Yearly,
                            _ => return,
                        };
                        s.choose(next, w, cx);
                        w.prevent_default();
                        cx.stop_propagation();
                    }));
            }
            row = row.child(item);
        }
        options = options.child(row.on_children_prepainted(move |bounds, _, _| {
            target.set(bounds.get(selected).copied());
        }));
        div()
            .id("billing-toggle")
            .flex()
            .flex_col()
            .items_start()
            .gap(px(6.))
            .role(Role::RadioGroup)
            .aria_label(self.label.clone())
            .max_w_full()
            .when(!self.hide_label && !self.label.is_empty(), |d| {
                d.child(
                    div()
                        .text_size(px(14.))
                        .line_height(px(20.))
                        .text_color(p.faint)
                        .child(self.label.clone()),
                )
            })
            .child(options.when(self.disabled, |e| e.opacity(0.45)))
    }
}

#[cfg(test)]
mod geometry_tests {
    use super::{BillingToggle, Highlight};
    use gpui::{Bounds, TestAppContext, point, px, size};

    #[test]
    fn tracks_actual_option_width_and_snaps_after_resize() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (_view, w) = app.add_window_view(|_, cx| BillingToggle::new("Period", cx));
        w.update(|_, cx| {
            let mut h = Highlight::default();
            let surface = Bounds::new(point(px(0.), px(0.)), size(px(400.), px(44.)));
            let first = Bounds::new(point(px(4.), px(4.)), size(px(80.), px(36.)));
            let r = h.geometry(0, first, surface, cx).0;
            assert_eq!(r, first);
            let resized = Bounds::new(point(px(4.), px(4.)), size(px(120.), px(36.)));
            let r = h.geometry(0, resized, surface, cx).0;
            assert_eq!(r, resized);
            let second = Bounds::new(point(px(124.), px(4.)), size(px(176.), px(36.)));
            h.geometry(1, second, surface, cx);
            assert_eq!(h.target, Some((1, 124., 176.)));
        });
    }
}
