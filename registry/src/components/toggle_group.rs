//! A compact single- or multiple-selection toggle group with gliding or joining fills.
use crate::foundation::{Theme, ThemeMode};
use crate::primitives::{
    family::{Paint, Slide, Tone, face},
    glyph::{Glyph, icon},
};
use gpui::{prelude::*, *};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeSet,
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
            self.x.duration(x, 0.42, cx);
            self.width.duration(width, 0.42, cx);
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

/// Selection behavior for a toggle group.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToggleMode {
    Single,
    Multiple,
}
/// A stable option key, accessible name and optional icon.
#[derive(Clone)]
pub struct ToggleOption {
    pub key: SharedString,
    pub label: SharedString,
    pub icon: Option<Glyph>,
    pub icon_only: bool,
    pub disabled: bool,
}
impl ToggleOption {
    pub fn new(key: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            icon: None,
            icon_only: false,
            disabled: false,
        }
    }
    pub fn with_icon(mut self, glyph: Glyph) -> Self {
        self.icon = Some(glyph);
        self
    }
    pub fn with_icon_only(mut self, icon_only: bool) -> Self {
        self.icon_only = icon_only;
        self
    }
    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}
/// Keys selected by the user, in visual order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToggleChanged(pub Vec<SharedString>);
/// Roving toolbar whose items support independent or exclusive selection.
pub struct ToggleGroup {
    label: SharedString,
    items: Vec<ToggleOption>,
    mode: ToggleMode,
    selected: BTreeSet<usize>,
    focus: Vec<FocusHandle>,
    active: usize,
    highlight: Rc<RefCell<Highlight>>,
    fills: Vec<Slide>,
    disabled: bool,
}
impl EventEmitter<ToggleChanged> for ToggleGroup {}
impl ToggleGroup {
    pub fn new(
        label: impl Into<SharedString>,
        items: Vec<ToggleOption>,
        mode: ToggleMode,
        cx: &mut Context<Self>,
    ) -> Self {
        let active = items.iter().position(|v| !v.disabled).unwrap_or(0);
        let len = items.len();
        Self {
            label: label.into(),
            items,
            mode,
            selected: BTreeSet::new(),
            focus: (0..len).map(|_| cx.focus_handle().tab_stop(true)).collect(),
            active,
            highlight: Default::default(),
            fills: vec![Slide::snappy(0.); len],
            disabled: false,
        }
    }
    pub fn with_selected(mut self, keys: &[&str]) -> Self {
        for (i, item) in self.items.iter().enumerate() {
            if !item.disabled && keys.contains(&item.key.as_ref()) {
                self.selected.insert(i);
                self.active = i;
                self.fills[i].reset(1.);
                if self.mode == ToggleMode::Single {
                    break;
                }
            }
        }
        self
    }
    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
    pub fn selected(&self) -> Vec<SharedString> {
        self.selected
            .iter()
            .map(|&i| self.items[i].key.clone())
            .collect()
    }
    fn toggle(&mut self, i: usize, w: &mut Window, cx: &mut Context<Self>) {
        if self.disabled || self.items.get(i).is_none_or(|x| x.disabled) {
            return;
        }
        self.active = i;
        self.focus[i].focus(w, cx);
        if self.mode == ToggleMode::Single {
            if self.selected.contains(&i) {
                return;
            }
            self.selected.clear();
            self.selected.insert(i);
        } else if !self.selected.remove(&i) {
            self.selected.insert(i);
        }
        cx.emit(ToggleChanged(self.selected()));
        cx.notify();
    }
    fn step(&mut self, delta: isize, w: &mut Window, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        let enabled: Vec<usize> = self
            .items
            .iter()
            .enumerate()
            .filter_map(|(i, v)| (!v.disabled).then_some(i))
            .collect();
        if enabled.is_empty() {
            return;
        }
        let old = enabled.iter().position(|x| *x == self.active).unwrap_or(0);
        let i = enabled[(old as isize + delta).rem_euclid(enabled.len() as isize) as usize];
        self.active = i;
        if self.mode == ToggleMode::Single {
            self.toggle(i, w, cx)
        } else {
            self.focus[i].focus(w, cx);
            cx.notify()
        }
    }
}
impl Render for ToggleGroup {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let radius = crate::foundation::surface_radius(cx);
        let item_radius = (radius - px(3.)).max(px(0.));
        let dark = Theme::global(cx).mode == ThemeMode::Dark;
        let fill_color = p.panel.blend(p.ink.opacity(if dark { 0.14 } else { 0.11 }));
        let mut bar = div()
            .id("toggle-group-bar")
            .relative()
            .flex()
            .items_center()
            .gap(px(2.))
            .p(px(3.))
            .rounded(radius)
            .border_1()
            .border_color(p.line)
            .bg(p.panel)
            .role(if self.mode == ToggleMode::Single {
                Role::RadioGroup
            } else {
                Role::Group
            })
            .aria_label(self.label.clone());
        let selected = if self.mode == ToggleMode::Single {
            self.selected.iter().next().copied()
        } else {
            None
        };
        let target = Rc::new(Cell::new(None::<Bounds<Pixels>>));
        if let Some(selected) = selected {
            let target_for_paint = target.clone();
            let highlight = self.highlight.clone();
            bar = bar.child(
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
                            let r = item_radius
                                .min(rect.size.width / 2.)
                                .min(rect.size.height / 2.);
                            w.paint_quad(fill(rect, fill_color).corner_radii(Corners::all(r)));
                        }
                    },
                )
                .absolute()
                .inset_0(),
            );
        }
        let mut row = div().flex().items_center().gap(px(2.));
        for (i, opt) in self.items.iter().enumerate() {
            let active = self.selected.contains(&i);
            let prev_active = i > 0 && self.selected.contains(&(i - 1));
            let next_active = self.selected.contains(&(i + 1));
            self.fills[i].duration(active as u8 as f32, 0.16, cx);
            let t = self.fills[i].get(w, cx).clamp(0., 1.);
            let show_text = !(opt.icon_only && opt.icon.is_some());
            let mut cell = face(
                ("toggle", i),
                opt.label.clone(),
                36.,
                item_radius.into(),
                Tone::Clear,
                cx,
            )
            .relative()
            .min_w(px(36.))
            .px(px(if show_text { 10. } else { 0. }))
            .gap(px(6.))
            .line_height(px(14.))
            .track_focus(&self.focus[i])
            .tab_index(if i == self.active && !opt.disabled && !self.disabled {
                0
            } else {
                -1
            })
            .role(if self.mode == ToggleMode::Single {
                Role::RadioButton
            } else {
                Role::Button
            })
            .aria_toggled(active.into());
            if self.mode == ToggleMode::Multiple {
                cell = cell.bg(fill_color.opacity(t));
                if active && prev_active {
                    cell =
                        cell.rounded_l(px(0.))
                            .ml(px(-2.))
                            .pl(px(if show_text { 12. } else { 2. }));
                }
                if active && next_active {
                    cell = cell.rounded_r(px(0.));
                }
            }
            let ink = if active { p.ink } else { p.secondary };
            cell = cell
                .text_color(ink)
                .when_some(opt.icon, |e, g| e.child(icon(g, 16., ink)))
                .when(show_text, |e| e.child(opt.label.clone()));
            if self.disabled || opt.disabled {
                cell = cell.opacity(0.45).cursor_default();
            } else {
                let hover_bg = p.tray;
                let active_ink = p.ink;
                cell = cell
                    .hover(move |s| s.text_color(active_ink))
                    .when(!active, move |e| e.hover(move |s| s.bg(hover_bg)))
                    .on_click(cx.listener(move |s, _, w, cx| s.toggle(i, w, cx)))
                    .on_key_down(cx.listener(move |s, e: &KeyDownEvent, w, cx| {
                        let delta = match e.keystroke.key.as_str() {
                            "left" | "up" => -1,
                            "right" | "down" => 1,
                            "home" => -(s.items.len() as isize),
                            "end" => s.items.len() as isize,
                            _ => return,
                        };
                        if matches!(e.keystroke.key.as_str(), "home" | "end") {
                            let enabled: Vec<usize> = s
                                .items
                                .iter()
                                .enumerate()
                                .filter_map(|(idx, v)| (!v.disabled).then_some(idx))
                                .collect();
                            if let Some(&target_idx) = if e.keystroke.key == "home" {
                                enabled.first()
                            } else {
                                enabled.last()
                            } {
                                s.active = target_idx;
                                if s.mode == ToggleMode::Single {
                                    s.toggle(target_idx, w, cx);
                                } else {
                                    s.focus[target_idx].focus(w, cx);
                                    cx.notify();
                                }
                            }
                        } else {
                            s.step(delta, w, cx);
                        }
                        w.prevent_default();
                        cx.stop_propagation()
                    }));
            }
            row = row.child(cell);
        }
        bar = bar.child(row.on_children_prepainted(move |bounds, _, _| {
            target.set(selected.and_then(|index| bounds.get(index).copied()));
        }));
        div()
            .flex()
            .flex_col()
            .items_start()
            .gap(px(6.))
            .max_w_full()
            .child(bar.when(self.disabled, |e| e.opacity(0.52)))
    }
}

#[cfg(test)]
mod geometry_tests {
    use super::{Highlight, ToggleGroup, ToggleMode};
    use gpui::{Bounds, TestAppContext, point, px, size};

    #[test]
    fn tracks_actual_option_width_and_snaps_after_resize() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (_view, w) =
            app.add_window_view(|_, cx| ToggleGroup::new("Group", vec![], ToggleMode::Single, cx));
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
