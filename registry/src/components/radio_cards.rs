//! Descriptive radio cards with a shared, interruptible selection outline.
use crate::{
    foundation::Theme,
    primitives::{
        color_motion::{ColorFade, mix},
        control::{UiDirection, UiLanguage},
        family::{Slide, disabled_motion, standard_ease},
        glyph::{Glyph, icon},
    },
};
use gpui::{prelude::*, *};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use web_time::Instant;

struct RingHighlight {
    x: Slide,
    y: Slide,
    width: Slide,
    height: Slide,
    target: Option<(usize, Bounds<Pixels>)>,
    surface: Option<Size<Pixels>>,
    first_at: Option<Instant>,
}
impl Default for RingHighlight {
    fn default() -> Self {
        Self {
            x: Slide::new(0.),
            y: Slide::new(0.),
            width: Slide::new(0.),
            height: Slide::new(0.),
            target: None,
            surface: None,
            first_at: None,
        }
    }
}
impl RingHighlight {
    fn rect(
        &mut self,
        selected: usize,
        target: Bounds<Pixels>,
        surface: Bounds<Pixels>,
        w: &Window,
        cx: &App,
    ) -> (Bounds<Pixels>, f32) {
        let x = f32::from(target.left() - surface.left());
        let y = f32::from(target.top() - surface.top());
        let width = f32::from(target.size.width);
        let height = f32::from(target.size.height);
        let local = Bounds::new(point(px(x), px(y)), target.size);
        let resized = self.surface.is_some_and(|s| s != surface.size);
        let first = self.target.is_none();
        let moved = self
            .target
            .is_some_and(|(i, b)| i == selected && b != local);
        if first || resized || moved || disabled_motion(cx) {
            self.x.reset(x);
            self.y.reset(y);
            self.width.reset(width);
            self.height.reset(height);
            if first && !disabled_motion(cx) {
                self.first_at = Some(cx.background_executor().now());
            } else {
                self.first_at = None;
            }
        } else if self.target.is_some_and(|(i, _)| i != selected) {
            self.x.duration(x, 0.42, cx);
            self.y.duration(y, 0.42, cx);
            self.width.duration(width, 0.42, cx);
            self.height.duration(height, 0.42, cx);
        }
        self.target = Some((selected, local));
        self.surface = Some(surface.size);
        let alpha = if let Some(at) = self.first_at {
            let t = (cx
                .background_executor()
                .now()
                .saturating_duration_since(at)
                .as_secs_f32()
                / 0.16)
                .clamp(0., 1.);
            if t < 1. {
                w.request_animation_frame();
            }
            standard_ease(t)
        } else {
            1.
        };
        (
            Bounds::new(
                point(
                    surface.left() + px(self.x.get(w, cx)),
                    surface.top() + px(self.y.get(w, cx)),
                ),
                size(px(self.width.get(w, cx)), px(self.height.get(w, cx))),
            ),
            alpha,
        )
    }
    fn clear(&mut self) {
        self.target = None;
        self.surface = None;
        self.first_at = None;
    }
}

/// Arrangement of descriptive radio options.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RadioCardLayout {
    #[default]
    Grid,
    List,
}
/// A stable option with optional icon, detail, metadata and a disabled explanation.
#[derive(Clone)]
pub struct RadioCardOption {
    pub key: SharedString,
    pub label: SharedString,
    pub detail: SharedString,
    pub price: SharedString,
    pub icon: Option<Glyph>,
    pub disabled: bool,
    pub disabled_reason: Option<SharedString>,
}
impl RadioCardOption {
    pub fn new(key: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            detail: "".into(),
            price: "".into(),
            icon: None,
            disabled: false,
            disabled_reason: None,
        }
    }
    pub fn with_detail(mut self, detail: impl Into<SharedString>) -> Self {
        self.detail = detail.into();
        self
    }
    /// Trailing metadata in a list, or below the body in a grid.
    pub fn with_price(mut self, price: impl Into<SharedString>) -> Self {
        self.price = price.into();
        self
    }
    pub fn with_icon(mut self, glyph: Glyph) -> Self {
        self.icon = Some(glyph);
        self
    }
    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
    /// Preserve the existing builder: supplying a reason also disables the option.
    pub fn with_disabled_reason(mut self, reason: impl Into<SharedString>) -> Self {
        self.disabled_reason = Some(reason.into());
        self.disabled = true;
        self
    }
}
/// An interactive selection, identified by the host's stable option key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RadioCardChanged(pub SharedString);
#[derive(Clone, Copy)]
struct CardInk {
    background: ColorFade,
    border: ColorFade,
    indicator: ColorFade,
    indicator_fill: ColorFade,
}
/// One roving tab stop, responsive grid columns, and a shared moving outline.
pub struct RadioCards {
    label: SharedString,
    items: Vec<RadioCardOption>,
    layout: RadioCardLayout,
    selected: Option<usize>,
    active: usize,
    focus: Vec<FocusHandle>,
    indicators: Vec<Slide>,
    ring: Rc<RefCell<RingHighlight>>,
    card_bounds: Vec<Rc<Cell<Bounds<Pixels>>>>,
    disabled: bool,
    required: bool,
    controlled: bool,
    name: Option<SharedString>,
    min_column_width: f32,
    measured_width: f32,
    hovered: Option<usize>,
    pressed: Option<usize>,
    inks: Vec<Option<CardInk>>,
}
impl EventEmitter<RadioCardChanged> for RadioCards {}
impl RadioCards {
    pub fn new(
        label: impl Into<SharedString>,
        items: Vec<RadioCardOption>,
        cx: &mut Context<Self>,
    ) -> Self {
        let active = items.iter().position(|x| !x.disabled).unwrap_or(0);
        let n = items.len();
        Self {
            label: label.into(),
            items,
            layout: RadioCardLayout::Grid,
            selected: None,
            active,
            focus: (0..n).map(|_| cx.focus_handle()).collect(),
            indicators: vec![Slide::snappy(0.); n],
            ring: Default::default(),
            card_bounds: (0..n).map(|_| Rc::default()).collect(),
            disabled: false,
            required: false,
            controlled: false,
            name: None,
            min_column_width: 180.,
            measured_width: 0.,
            hovered: None,
            pressed: None,
            inks: vec![None; n],
        }
    }
    pub fn with_layout(mut self, layout: RadioCardLayout) -> Self {
        self.layout = layout;
        self
    }
    pub fn with_selected(mut self, key: &str) -> Self {
        if let Some(i) = self.items.iter().position(|x| x.key.as_ref() == key) {
            self.selected = Some(i);
            if !self.items[i].disabled {
                self.active = i;
            }
            self.indicators[i].reset(1.);
        }
        self
    }
    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
    pub fn with_required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }
    /// Leave selection under host control; emit intent and await `set_selected`.
    pub fn with_controlled(mut self, controlled: bool) -> Self {
        self.controlled = controlled;
        self
    }
    /// The host serializes this field when submitting a form.
    pub fn with_name(mut self, name: impl Into<SharedString>) -> Self {
        self.name = Some(name.into());
        self
    }
    pub fn form_value(&self) -> Option<(SharedString, SharedString)> {
        self.name
            .clone()
            .map(|name| (name, self.selected().unwrap_or_else(|| "".into())))
    }
    /// Minimum responsive column width, in px. Positive finite values are accepted.
    pub fn with_min_column_width(mut self, width: f32) -> Self {
        if width.is_finite() && width > 0. {
            self.min_column_width = width;
        }
        self
    }
    pub fn selected(&self) -> Option<SharedString> {
        self.selected.map(|i| self.items[i].key.clone())
    }
    /// Form serialization and submission remain the native/Web host's responsibility.
    pub fn is_valid(&self) -> bool {
        !self.required || self.selected.is_some()
    }
    pub fn set_selected(&mut self, key: &str, cx: &mut Context<Self>) -> bool {
        let Some(i) = self.items.iter().position(|x| x.key.as_ref() == key) else {
            return false;
        };
        self.selected = Some(i);
        if !self.items[i].disabled {
            self.active = i;
        }
        cx.notify();
        true
    }
    pub fn clear_selected(&mut self, cx: &mut Context<Self>) {
        if self.selected.take().is_some() {
            self.ring.borrow_mut().clear();
            cx.notify();
        }
    }
    fn select(&mut self, i: usize, w: &mut Window, cx: &mut Context<Self>) {
        if self.disabled || self.items.get(i).is_none_or(|x| x.disabled) {
            return;
        }
        self.focus[i].focus(w, cx);
        self.active = i;
        if self.selected != Some(i) {
            if !self.controlled {
                self.selected = Some(i);
            }
            cx.emit(RadioCardChanged(self.items[i].key.clone()));
        }
        cx.notify();
    }
    fn step(&mut self, d: isize, w: &mut Window, cx: &mut Context<Self>) {
        let usable: Vec<_> = self
            .items
            .iter()
            .enumerate()
            .filter_map(|(i, x)| (!x.disabled).then_some(i))
            .collect();
        if usable.is_empty() || self.disabled {
            return;
        }
        let old = usable.iter().position(|i| *i == self.active).unwrap_or(0);
        self.select(
            usable[(old as isize + d).rem_euclid(usable.len() as isize) as usize],
            w,
            cx,
        );
    }
    fn edge(&mut self, last: bool, w: &mut Window, cx: &mut Context<Self>) {
        let i = if last {
            self.items.iter().rposition(|x| !x.disabled)
        } else {
            self.items.iter().position(|x| !x.disabled)
        };
        if let Some(i) = i {
            self.select(i, w, cx);
        }
    }
    /// Apply one navigation key, returning whether it was handled. Only the
    /// inline axis (Left/Right) reverses under RTL; Up/Down keep the block
    /// order and Home/End still target the first/last usable option.
    fn key(&mut self, key: &str, w: &mut Window, cx: &mut Context<Self>) -> bool {
        let dir = UiDirection::current(cx);
        match key {
            "left" => self.step(dir.inline_step(-1), w, cx),
            "up" => self.step(-1, w, cx),
            "right" => self.step(dir.inline_step(1), w, cx),
            "down" => self.step(1, w, cx),
            "home" => self.edge(false, w, cx),
            "end" => self.edge(true, w, cx),
            _ => return false,
        }
        true
    }
}
impl Render for RadioCards {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = Theme::global(cx).tokens.colors.clone();
        let radius = crate::foundation::surface_radius(cx);
        let reduced = disabled_motion(cx);
        let seconds = if reduced { 0. } else { 0.16 };
        let now = cx.background_executor().now();
        // The host owns writing direction; GPUI has no direction property, so the
        // grid mirrors its columns explicitly the way CSS would under `dir="rtl"`.
        let rtl = UiDirection::current(cx).is_rtl();
        let width = if self.measured_width > 0. {
            self.measured_width
        } else {
            f32::from(w.viewport_size().width)
        };
        let columns = if self.layout == RadioCardLayout::List {
            1
        } else {
            ((width + 10.) / (self.min_column_width + 10.))
                .floor()
                .max(1.)
                .min(u16::MAX as f32) as u16
        };
        let cols = usize::from(columns);
        // The grid width is only known after the first layout pass, so a grid's
        // column count can be wrong on frame one. Keep that frame laid out but
        // invisible so the corrected layout is the first one the user sees;
        // a list has one column and needs no such pass.
        let pending = self.layout == RadioCardLayout::Grid && self.measured_width <= 0.;
        if pending {
            w.request_animation_frame();
        }
        // Mirror the grid when the host reads right-to-left, the way a CSS grid
        // flows under `dir="rtl"`: give each card its column explicitly so a
        // short last row still starts from the inline edge. Rows keep their
        // top-to-bottom order, and the keyboard reading order stays logical.
        let mirror = rtl && cols > 1;
        let tab_stop = self
            .selected
            .filter(|i| !self.items[*i].disabled)
            .or_else(|| self.items.iter().position(|item| !item.disabled));
        let mut cards = div()
            .relative()
            .w_full()
            .grid()
            .grid_cols(columns)
            .gap(px(if self.layout == RadioCardLayout::Grid {
                10.
            } else {
                8.
            }))
            .text_size(px(14.))
            .line_height(relative(1.4))
            .text_color(colors.foreground);
        for i in 0..self.items.len() {
            let item = &self.items[i];
            let selected = self.selected == Some(i);
            let off = self.disabled || item.disabled;
            let hovered = self.hovered == Some(i) && !w.last_input_was_touch();
            let focus_visible = self.focus[i].is_focused(w) && w.last_input_was_keyboard();
            let pressed = self.pressed == Some(i);
            self.indicators[i].duration(u8::from(selected) as f32, 0.26, cx);
            let dot = self.indicators[i].get(w, cx);
            let detail = if off {
                item.disabled_reason
                    .clone()
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| item.detail.clone())
            } else {
                item.detail.clone()
            };
            let bg = if off || !selected && (pressed || focus_visible) {
                colors.surface_muted
            } else if selected {
                mix(colors.surface, colors.accent, 0.04)
            } else {
                colors.surface
            };
            let border = if !off && !selected && (hovered || focus_visible) {
                colors.border_strong
            } else {
                colors.border
            };
            let indicator = if off {
                colors.border
            } else if selected {
                colors.accent
            } else {
                colors.border_strong
            };
            let indicator_fill = if selected {
                colors.accent
            } else {
                transparent_black()
            };
            let fade = |to| ColorFade {
                from: to,
                to,
                at: now,
            };
            let inks = self.inks[i].get_or_insert(CardInk {
                background: fade(bg),
                border: fade(border),
                indicator: fade(indicator),
                indicator_fill: fade(indicator_fill),
            });
            let (bg, m1) = inks.background.retarget(bg, now, seconds);
            let (border, m2) = inks.border.retarget(border, now, seconds);
            let (mark, m3) = inks.indicator.retarget(indicator, now, seconds);
            let (mark_bg, m4) = inks.indicator_fill.retarget(indicator_fill, now, seconds);
            if m1 || m2 || m3 || m4 {
                w.request_animation_frame();
            }
            let indicator = div()
                .size(px(18.))
                .flex_none()
                .rounded_full()
                .border(px(1.5))
                .border_color(mark)
                .bg(mark_bg)
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .size(px(6. * dot))
                        .rounded_full()
                        .bg(colors.accent_foreground),
                );
            let title = div()
                .flex()
                .min_w_0()
                .items_center()
                .gap(px(8.))
                .when(rtl, |d| d.flex_row_reverse())
                .font_weight(FontWeight::MEDIUM)
                .text_color(if off {
                    colors.text_muted
                } else {
                    colors.foreground
                })
                .when_some(item.icon, |d, g| {
                    d.child(icon(
                        g,
                        18.,
                        if selected {
                            colors.foreground
                        } else {
                            colors.text_secondary
                        },
                    ))
                })
                .child(div().min_w_0().truncate().child(item.label.clone()));
            let body = div()
                .flex()
                .flex_col()
                .flex_1()
                .min_w_0()
                .gap(px(2.))
                .when(rtl, |d| d.text_right())
                .child(title)
                .when(!detail.is_empty(), |d| {
                    d.child(
                        div()
                            .text_size(px(12.))
                            .line_height(relative(1.4))
                            .text_color(colors.text_secondary)
                            .child(detail.clone()),
                    )
                });
            let b = self.card_bounds[i].clone();
            let mut card = div()
                .relative()
                .min_w_0()
                .flex()
                .rounded(radius)
                .border_1()
                .border_color(border)
                .bg(bg)
                .text_size(px(14.))
                .line_height(relative(1.4))
                .when(mirror, |d| {
                    d.row_start((i / cols + 1) as i16)
                        .col_start((cols - i % cols) as i16)
                })
                .child(
                    canvas(move |bounds, _, _| b.set(bounds), |_, _, _, _| ())
                        .absolute()
                        .inset_0(),
                )
                .id(("radio-card", i))
                .debug_selector({
                    let label = item.label.clone();
                    move || label.to_string()
                })
                .role(Role::RadioButton)
                .aria_label(item.label.clone())
                .aria_description(if off && detail.is_empty() {
                    UiLanguage::current(cx).text("Disabled", "不可用").into()
                } else {
                    detail
                })
                .aria_toggled(selected.into())
                .track_focus(&self.focus[i])
                .tab_index(if !off && Some(i) == tab_stop { 0 } else { -1 });
            if self.layout == RadioCardLayout::Grid {
                card = card
                    .flex_col()
                    .items_stretch()
                    .px(px(16.))
                    .py(px(14.))
                    .gap(px(4.))
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .items_start()
                            .gap(px(12.))
                            .flex_1()
                            .when(rtl, |d| d.flex_row_reverse())
                            .child(body)
                            .child(indicator.mt(px(1.))),
                    )
                    .when(!item.price.is_empty(), |d| {
                        d.child(
                            div()
                                .mt(px(10.))
                                .flex_none()
                                .whitespace_nowrap()
                                .text_size(px(18.))
                                .line_height(relative(1.4))
                                .text_color(if off {
                                    colors.text_muted
                                } else {
                                    colors.foreground
                                })
                                .child(item.price.clone()),
                        )
                    });
            } else {
                card = card
                    .items_center()
                    .when(rtl, |d| d.flex_row_reverse())
                    .pl(px(14.))
                    .pr(px(16.))
                    .py(px(13.))
                    .gap(px(12.))
                    .child(indicator)
                    .child(body)
                    .when(!item.price.is_empty(), |d| {
                        d.child(
                            div()
                                .flex_none()
                                .whitespace_nowrap()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(if off {
                                    colors.text_muted
                                } else {
                                    colors.foreground
                                })
                                .child(item.price.clone()),
                        )
                    });
            }
            if !off {
                card = card
                    .cursor_pointer()
                    .on_hover(cx.listener(move |s, over: &bool, _, cx| {
                        s.hovered = if *over { Some(i) } else { None };
                        if !over {
                            s.pressed = None;
                        }
                        cx.notify();
                    }))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |s, _, _, cx| {
                            s.pressed = Some(i);
                            cx.notify();
                        }),
                    )
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(|s, _, _, cx| {
                            s.pressed = None;
                            cx.notify();
                        }),
                    )
                    .on_mouse_up_out(
                        MouseButton::Left,
                        cx.listener(|s, _, _, cx| {
                            s.pressed = None;
                            cx.notify();
                        }),
                    )
                    .on_click(cx.listener(move |s, _, w, cx| s.select(i, w, cx)))
                    .on_key_down(cx.listener(move |s, e: &KeyDownEvent, w, cx| {
                        s.active = i;
                        let handled = match e.keystroke.key.as_str() {
                            "enter" | "space" => {
                                s.select(i, w, cx);
                                true
                            }
                            key => s.key(key, w, cx),
                        };
                        if !handled {
                            return;
                        }
                        w.prevent_default();
                        cx.stop_propagation();
                    }));
            }
            cards = cards.child(card);
        }
        let weak = cx.weak_entity();
        let measured = self.measured_width;
        // The wrapper is always painted so the width measurement arrives on the
        // first pass. Only the grid itself is withheld while pending: a grid's
        // column count can be wrong on frame one, and the corrected layout must
        // be the first frame the user actually sees.
        let mut root = div().relative().w_full().child(
            div()
                .relative()
                .w_full()
                .when(pending, |d| d.opacity(0.))
                .child(cards),
        );
        root = root.child(
            canvas(
                move |b, _, cx| {
                    let width = f32::from(b.size.width);
                    if (measured - width).abs() > 0.5 {
                        let _ = weak.update(cx, |s, cx| {
                            s.measured_width = width;
                            cx.notify();
                        });
                    }
                },
                |_, _, _, _| (),
            )
            .absolute()
            .inset_0(),
        );
        // The ring is positioned from the card bounds reported by the grid's first
        // layout pass, so it must be withheld with the grid: on the pending frame
        // those bounds still describe the pre-measure column count and the ring
        // would be painted at the wrong place and size.
        if let Some(index) = self.selected
            && !pending
        {
            let b = self.card_bounds[index].clone();
            let ring = self.ring.clone();
            let ink = colors.accent;
            root = root.child(
                canvas(
                    |_, _, _| (),
                    move |surface, _, w, cx| {
                        let target = b.get();
                        if target.size.width <= px(0.) || target.size.height <= px(0.) {
                            return;
                        }
                        let (rect, alpha) = ring.borrow_mut().rect(index, target, surface, w, cx);
                        w.paint_quad(quad(
                            rect,
                            Corners::all(radius),
                            transparent_black(),
                            Edges::all(px(1.5)),
                            ink.opacity(alpha),
                            BorderStyle::Solid,
                        ));
                    },
                )
                .absolute()
                .inset_0(),
            );
        }
        root.id("radio-cards")
            .role(Role::RadioGroup)
            .aria_label(self.label.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::{RadioCardOption, RadioCards};
    use crate::primitives::control::UiDirection;
    use gpui::{TestAppContext, px, size};
    fn options() -> Vec<RadioCardOption> {
        vec![
            RadioCardOption::new("first", "First"),
            RadioCardOption::new("off", "Unavailable").with_disabled(true),
            RadioCardOption::new("last", "Last"),
        ]
    }
    #[test]
    fn keyboard_wraps_and_edges_skip_unavailable_options() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) = app.add_window_view(|_, cx| RadioCards::new("Plan", options(), cx));
        w.update(|w, cx| {
            view.update(cx, |s, cx| {
                s.select(0, w, cx);
                s.step(1, w, cx);
                assert_eq!(s.selected().as_deref(), Some("last"));
                s.step(1, w, cx);
                assert_eq!(s.selected().as_deref(), Some("first"));
                s.edge(true, w, cx);
                assert_eq!(s.selected().as_deref(), Some("last"));
                s.edge(false, w, cx);
                assert_eq!(s.selected().as_deref(), Some("first"));
            })
        });
    }
    #[test]
    fn host_selection_can_be_disabled_and_required_can_be_cleared() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) = app.add_window_view(|_, cx| {
            RadioCards::new("Plan", options(), cx)
                .with_selected("off")
                .with_required(true)
        });
        w.update(|_, cx| {
            view.update(cx, |s, cx| {
                assert_eq!(s.selected().as_deref(), Some("off"));
                assert!(s.is_valid());
                assert_eq!(s.active, 0);
                s.clear_selected(cx);
                assert!(!s.is_valid());
                assert!(!s.set_selected("unknown", cx));
                assert!(s.set_selected("last", cx));
                assert!(s.is_valid());
            })
        });
    }
    #[test]
    fn odd_grid_items_keep_their_column_and_narrow_layout_becomes_single_column() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) = app.add_window_view(|_, cx| RadioCards::new("Plan", options(), cx));
        w.simulate_resize(size(px(400.), px(500.)));
        w.run_until_parked();
        for _ in 0..3 {
            w.update(|w, cx| {
                w.simulate_next_frame(cx);
                w.refresh();
            });
            w.run_until_parked();
        }
        let first = w.debug_bounds("First").unwrap();
        let last = w.debug_bounds("Last").unwrap();
        assert!((f32::from(first.size.width - last.size.width)).abs() < 0.1);
        assert!(
            last.top() > first.top(),
            "first {first:?}, last {last:?}; width {}",
            w.update(|_, cx| view.read(cx).measured_width)
        );
        w.simulate_resize(size(px(320.), px(600.)));
        w.run_until_parked();
        for _ in 0..3 {
            w.update(|w, cx| {
                w.simulate_next_frame(cx);
                w.refresh();
            });
            w.run_until_parked();
        }
        w.update(|_, cx| {
            let s = view.read(cx);
            let first = s.card_bounds[0].get();
            let second = s.card_bounds[1].get();
            assert_eq!(first.left(), second.left());
            assert!(second.top() > first.top());
        });
    }
    #[test]
    fn controlled_selection_waits_for_host_and_real_keyboard_reaches_the_last_option() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) = app.add_window_view(|_, cx| {
            RadioCards::new("Plan", options(), cx)
                .with_selected("first")
                .with_controlled(true)
                .with_name("plan")
        });
        w.run_until_parked();
        let first = w.debug_bounds("First").unwrap().center();
        w.simulate_click(first, gpui::Modifiers::default());
        w.run_until_parked();
        w.simulate_keystrokes("end");
        w.run_until_parked();
        w.update(|w, cx| {
            let s = view.read(cx);
            assert_eq!(s.selected().as_deref(), Some("first"));
            assert!(s.focus[2].is_focused(w));
            assert_eq!(s.form_value().unwrap().1.as_ref(), "first");
        });
        w.update(|_, cx| {
            view.update(cx, |s, cx| {
                assert!(s.set_selected("last", cx));
            })
        });
        w.run_until_parked();
        assert_eq!(
            w.update(|_, cx| view.read(cx).selected()).as_deref(),
            Some("last")
        );
    }
    #[test]
    fn horizontal_arrows_reverse_only_under_rtl() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) = app
            .add_window_view(|_, cx| RadioCards::new("Plan", options(), cx).with_selected("first"));
        w.run_until_parked();
        let mut press = |view: &gpui::Entity<RadioCards>, key: &str, dir: UiDirection| {
            w.update(|w, cx| {
                cx.set_global(dir);
                view.update(cx, |s, cx| {
                    s.active = s.selected.unwrap_or(0);
                    s.key(key, w, cx);
                });
            });
            w.run_until_parked();
            w.update(|_, cx| view.read(cx).selected().map(|k| k.to_string()))
        };
        // LTR: Right advances to the next usable option, Left goes back.
        assert_eq!(
            press(&view, "right", UiDirection::Ltr).as_deref(),
            Some("last")
        );
        assert_eq!(
            press(&view, "left", UiDirection::Ltr).as_deref(),
            Some("first")
        );
        // RTL: the inline axis reverses, so Right moves back and Left advances.
        assert_eq!(
            press(&view, "right", UiDirection::Rtl).as_deref(),
            Some("last")
        );
        assert_eq!(
            press(&view, "right", UiDirection::Rtl).as_deref(),
            Some("first")
        );
        assert_eq!(
            press(&view, "left", UiDirection::Rtl).as_deref(),
            Some("last")
        );
        // The block axis and Home/End do not reverse under RTL.
        assert_eq!(
            press(&view, "down", UiDirection::Rtl).as_deref(),
            Some("first")
        );
        assert_eq!(
            press(&view, "up", UiDirection::Rtl).as_deref(),
            Some("last")
        );
        assert_eq!(
            press(&view, "home", UiDirection::Rtl).as_deref(),
            Some("first")
        );
        assert_eq!(
            press(&view, "end", UiDirection::Rtl).as_deref(),
            Some("last")
        );
    }
    #[test]
    fn rtl_grid_mirrors_columns_while_keeping_source_order() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (_view, mut w) = app.add_window_view(|_, cx| RadioCards::new("Plan", options(), cx));
        w.simulate_resize(size(px(400.), px(500.)));
        w.run_until_parked();
        let bounds = |w: &mut gpui::VisualTestContext| {
            for _ in 0..3 {
                w.update(|w, cx| {
                    w.simulate_next_frame(cx);
                    w.refresh();
                });
                w.run_until_parked();
            }
            (
                w.debug_bounds("First").unwrap(),
                w.debug_bounds("Unavailable").unwrap(),
            )
        };
        w.update(|_, cx| cx.set_global(UiDirection::Ltr));
        let (ltr_first, ltr_second) = bounds(&mut w);
        assert!(
            ltr_first.left() < ltr_second.left(),
            "LTR keeps the source order left-to-right: {ltr_first:?} {ltr_second:?}"
        );
        w.update(|_, cx| cx.set_global(UiDirection::Rtl));
        let (rtl_first, rtl_second) = bounds(&mut w);
        assert!(
            rtl_first.left() > rtl_second.left(),
            "RTL mirrors the first column to the inline start: {rtl_first:?} {rtl_second:?}"
        );
        assert_eq!(rtl_first.top(), ltr_first.top(), "rows keep their order");
    }
}
