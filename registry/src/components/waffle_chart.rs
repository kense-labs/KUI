use gpui::{prelude::*, *};
use std::cell::Cell;
use std::rc::Rc;
use web_time::Instant;

use crate::components::animated_counter::AnimatedCounter;
use crate::components::color_math::oklab_mix;
use crate::primitives::{
    family::{Paint, Slide, Tone, disabled_motion, face, measure},
    presentation::{row, tr},
};

/// One category of the whole. Categories without an explicit colour take the
/// reference series order: accent hue first, the three supporting series
/// next, then progressively fainter neutral steps.
pub struct WaffleCategory {
    pub id: SharedString,
    pub label: SharedString,
    pub value: f64,
    pub color: Option<Hsla>,
}
impl WaffleCategory {
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>, value: f64) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            value: if value.is_finite() { value.max(0.) } else { 0. },
            color: None,
        }
    }
    /// Paint this category with an explicit colour instead of the series order.
    pub fn with_color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }
}
/// Emitted when a pointer or keyboard cursor selects a category.
pub struct WaffleInspect {
    pub id: SharedString,
    pub value: f64,
    pub share: f64,
}
struct Square {
    id: SharedString,
    x: Slide,
    y: Slide,
    appear: Slide,
    flight: Slide,
    color_progress: Slide,
    old_color: Hsla,
    color: Hsla,
}
/// The host owns the data; legend visibility and inspection are local state.
pub struct WaffleChart {
    label: SharedString,
    data: Vec<WaffleCategory>,
    hidden: Vec<SharedString>,
    cells: Vec<Square>,
    assignment: Vec<usize>,
    active: Option<usize>,
    pointer: Option<usize>,
    /// The pointer was placed by the keyboard; a stray hover-out must not
    /// clear the reference cursor until the mouse moves again.
    key_cursor: bool,
    pinned: Option<usize>,
    preview: Option<usize>,
    unit: SharedString,
    bounds: Rc<Cell<Bounds<Pixels>>>,
    counters: Vec<Option<Entity<AnimatedCounter>>>,
    initialized: bool,
    visible: bool,
}
impl EventEmitter<WaffleInspect> for WaffleChart {}
impl WaffleChart {
    pub fn new(label: impl Into<SharedString>, categories: Vec<WaffleCategory>) -> Self {
        Self {
            label: label.into(),
            data: categories,
            hidden: vec![],
            cells: vec![],
            assignment: vec![],
            active: None,
            pointer: None,
            key_cursor: false,
            pinned: None,
            preview: None,
            unit: SharedString::default(),
            bounds: Default::default(),
            counters: vec![],
            initialized: false,
            visible: true,
        }
    }
    /// Word shown after raw values in the tooltip, such as "TWh".
    pub fn with_unit(mut self, unit: impl Into<SharedString>) -> Self {
        self.unit = unit.into();
        self
    }
    /// Retitle the chart (the accessible label names the whole).
    pub fn set_label(&mut self, label: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.label = label.into();
        cx.notify();
    }
    /// Set false when the host hides this chart or its tab.
    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        if self.visible != visible {
            self.visible = visible;
            cx.notify();
        }
    }
    pub fn set_categories(&mut self, categories: Vec<WaffleCategory>, cx: &mut Context<Self>) {
        self.data = categories;
        self.hidden
            .retain(|id| self.data.iter().any(|d| d.id == *id));
        self.active = None;
        self.pointer = None;
        self.pinned = None;
        self.preview = None;
        self.reconcile(cx);
        cx.notify();
    }
    /// Cells per category. The reference walks the cumulative share and
    /// rounds each boundary, so the blocks always tile the grid exactly:
    /// Germany reads 43/4/1/15/26/11 for the same inputs.
    fn counts(&self) -> Vec<usize> {
        let values: Vec<f64> = self
            .data
            .iter()
            .map(|v| {
                if !self.hidden.contains(&v.id) && v.value.is_finite() {
                    v.value.max(0.)
                } else {
                    0.
                }
            })
            .collect();
        let total: f64 = values.iter().sum();
        if !total.is_finite() || total <= 0. {
            return vec![0; values.len()];
        }
        let mut counts = vec![0usize; values.len()];
        let mut boundary = 0usize;
        let mut cum = 0.;
        for (slot, &value) in values.iter().enumerate() {
            cum += value / total * 100.;
            let next = (cum.round() as i64).clamp(0, 100) as usize;
            counts[slot] = next.saturating_sub(boundary);
            boundary = next;
        }
        counts
    }
    /// The reference paints the first category with the accent series, the
    /// next three with the supporting hues, and later ones with neutral
    /// steps mixed from the foreground over the surface.
    fn category_color(index: usize, item: &WaffleCategory, p: &Paint) -> Hsla {
        if let Some(color) = item.color {
            return color;
        }
        let mix = |t: f32| oklab_mix(p.panel, p.ink, t);
        match index {
            0 => rgb(0x55adff).into(),
            1 => rgb(0x9c37be).into(),
            2 => rgb(0xe66e00).into(),
            3 => rgb(0x00a861).into(),
            4 => mix(0.40),
            5 => mix(0.20),
            _ => mix(0.10),
        }
    }
    fn reconcile(&mut self, cx: &App) {
        let p = Paint::get(cx);
        let counts = self.counts();
        let targets: Vec<usize> = counts
            .iter()
            .enumerate()
            .flat_map(|(i, &n)| std::iter::repeat_n(i, n))
            .collect();
        let first = !self.initialized;
        self.initialized = true;
        let mut available: Vec<Option<Square>> = std::mem::take(&mut self.cells)
            .into_iter()
            .map(Some)
            .collect();
        let mut next = Vec::with_capacity(targets.len());
        // First reserve cells with the same category. This keeps their identities
        // when a series changes size, instead of shuffling every color.
        let mut reserved = Vec::with_capacity(targets.len());
        for &category in &targets {
            let id = &self.data[category].id;
            let match_at = available
                .iter()
                .position(|cell| cell.as_ref().is_some_and(|c| c.id == *id));
            reserved.push(match_at.and_then(|i| available[i].take()));
        }
        for (slot, (&category, existing)) in targets.iter().zip(reserved).enumerate() {
            let item = &self.data[category];
            let color = Self::category_color(category, item, &p);
            let reuse = existing.or_else(|| available.iter_mut().find_map(Option::take));
            let mut square = reuse.unwrap_or_else(|| Square {
                id: item.id.clone(),
                x: Slide::new(0.),
                y: Slide::new(9.),
                appear: Slide::new(0.),
                flight: Slide::new(1.),
                color_progress: Slide::new(1.),
                old_color: color,
                color,
            });
            // Reference staggering: each cell trails the one before it by
            // 3.2ms, so a refill reads as a wave from the bottom-left.
            let delay = slot as f32 / 100. * 0.32;
            let x = (slot / 10) as f32;
            let y = 9. - (slot % 10) as f32;
            let moving = square.id != item.id || square.x.value(cx) != x || square.y.value(cx) != y;
            if square.id != item.id {
                square.id = item.id.clone();
                square.old_color = square.color;
                square.color = color;
                square.color_progress.reset(0.);
                square.color_progress.delayed(1., 0.48, delay, cx);
            } else {
                square.color = color;
            }
            if first {
                square.x.reset(x);
                square.y.reset(y);
                square.appear.delayed(1., 0.24, delay, cx);
            } else {
                square.x.duration(x, 0.48, cx);
                square.y.duration(y, 0.48, cx);
                square.appear.duration(1., 0.24, cx);
                // A travelling cell lifts out of its block and lands in the
                // next: it dips in scale for the length of the flight.
                if moving {
                    square.flight.reset(0.);
                    square.flight.duration(1., 0.5, cx);
                }
            }
            next.push(square);
        }
        self.assignment = targets;
        self.cells = next;
    }
    fn inspect(&mut self, index: Option<usize>, cx: &mut Context<Self>) {
        if self.active == index {
            return;
        }
        self.active = index;
        if let Some(category) = index
            .and_then(|i| self.assignment.get(i))
            .and_then(|&i| self.data.get(i))
        {
            let total: f64 = self
                .data
                .iter()
                .filter(|v| !self.hidden.contains(&v.id) && v.value.is_finite())
                .map(|v| v.value.max(0.))
                .sum();
            cx.emit(WaffleInspect {
                id: category.id.clone(),
                value: category.value,
                share: if total > 0. {
                    category.value / total
                } else {
                    0.
                },
            });
        }
        cx.notify();
    }
    fn sync_focus(&mut self, cx: &mut Context<Self>) {
        let slot = self.pointer.or_else(|| {
            let category = self.preview.or(self.pinned)?;
            self.assignment.iter().position(|&item| item == category)
        });
        self.inspect(slot, cx);
    }
    fn pin(&mut self, index: usize, cx: &mut Context<Self>) {
        self.pointer = None;
        self.preview = None;
        self.pinned = (self.pinned != Some(index)).then_some(index);
        self.sync_focus(cx);
    }
}
fn intersects_viewport(bounds: Bounds<Pixels>, viewport: Size<Pixels>) -> bool {
    bounds.size.width > px(0.)
        && bounds.size.height > px(0.)
        && bounds.origin.x < viewport.width
        && bounds.origin.y < viewport.height
        && bounds.origin.x + bounds.size.width > px(0.)
        && bounds.origin.y + bounds.size.height > px(0.)
}
fn request_if_active(visible: bool, active: bool, request: impl FnOnce()) {
    if visible && active {
        request();
    }
}
type CellFrame = (f32, f32, f32, f32, Hsla, SharedString);
fn sample_cells(cells: &[Square], now: Instant, reduced: bool) -> (Vec<CellFrame>, bool) {
    let mut active = false;
    let values = cells
        .iter()
        .map(|c| {
            let (x, ax) = c.x.sample_batch(now, reduced);
            let (y, ay) = c.y.sample_batch(now, reduced);
            let (appearance, aa) = c.appear.sample_batch(now, reduced);
            let (flight, af) = c.flight.sample_batch(now, reduced);
            let (color, ac) = c.color_progress.sample_batch(now, reduced);
            active |= ax || ay || aa || af || ac;
            // Scale dips to 0.62 mid-flight, like the reference spring.
            let dip = 1. - 0.38 * (std::f32::consts::PI * flight.clamp(0., 1.)).sin();
            (
                x,
                y,
                appearance.clamp(0., 1.),
                dip,
                c.old_color.blend(c.color.opacity(color.clamp(0., 1.))),
                c.id.clone(),
            )
        })
        .collect();
    (values, active)
}
fn thousands(value: f64) -> SharedString {
    let rounded = value.round() as i64;
    let text = rounded.abs().to_string();
    let mut grouped = String::new();
    for (i, ch) in text.chars().enumerate() {
        if i > 0 && (text.len() - i).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    if rounded < 0 {
        grouped.insert(0, '-');
    }
    grouped.into()
}
impl Render for WaffleChart {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.initialized {
            self.reconcile(cx);
        }
        let p = Paint::get(cx);
        let instructions = tr(
            cx,
            "Use the arrow keys to move between cells, Page Up and Page Down to jump between categories",
            "使用方向键在单元格间移动，Page Up 和 Page Down 在类别间跳转",
        );
        let counts = self.counts();
        let total_cells: usize = counts.iter().sum();
        let selected = self
            .active
            .and_then(|i| self.assignment.get(i))
            .and_then(|&i| self.data.get(i))
            .map(|x| x.id.clone());
        let now = cx.background_executor().now();
        let (cells, active) = sample_cells(&self.cells, now, disabled_motion(cx));
        let on_screen = self.visible
            && w.is_visible()
            && intersects_viewport(self.bounds.get(), w.viewport_size());
        request_if_active(on_screen, active, || w.request_animation_frame());
        let saved = self.bounds.clone();
        let cursor = self.active;
        let ink = p.ink;
        let surface = p.panel;
        let graph = div()
            .id("waffle-grid")
            .role(Role::Group)
            .aria_label(format!(
                "{}. {} {}. {}",
                self.label,
                total_cells,
                tr(cx, "cells", "格"),
                instructions
            ))
            .tab_index(0)
            .relative()
            .flex_none()
            .w(px(232.))
            .max_w_full()
            .h(px(232.))
            .on_mouse_move(cx.listener(|s, event: &MouseMoveEvent, _, cx| {
                let b = s.bounds.get();
                let x = f32::from(event.position.x - b.origin.x);
                let y = f32::from(event.position.y - b.origin.y);
                let pitch = 23.5_f32;
                let col = (x / pitch) as usize;
                let row = (y / pitch) as usize;
                let index = if x >= 0.
                    && y >= 0.
                    && col < 10
                    && row < 10
                    && x < col as f32 * pitch + 20.5
                    && y < row as f32 * pitch + 20.5
                {
                    Some(col * 10 + (9 - row)).filter(|&slot| slot < s.assignment.len())
                } else {
                    None
                };
                if s.pointer != index {
                    s.pointer = index;
                    s.key_cursor = false;
                    s.sync_focus(cx);
                }
            }))
            .on_hover(cx.listener(|s, hovered: &bool, _, cx| {
                if !hovered && !s.key_cursor {
                    s.pointer = None;
                    s.sync_focus(cx);
                }
            }))
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                let n = s.assignment.len();
                if n == 0 {
                    return;
                }
                let at = s.pointer.or(s.active).unwrap_or(0).min(n - 1);
                let col = at / 10;
                let within = at % 10;
                let category = s.assignment[at];
                let category_first = |category: usize, s: &Self| {
                    s.assignment.iter().position(|&item| item == category)
                };
                let next = match e.keystroke.key.as_str() {
                    "right" => Some(((col + 1).min(9)) * 10 + within).filter(|&slot| slot < n),
                    "left" => Some(col.saturating_sub(1) * 10 + within),
                    "up" => Some(col * 10 + (within + 1).min(9)).filter(|&slot| slot < n),
                    "down" => Some(col * 10 + within.saturating_sub(1)),
                    "home" => Some(0),
                    "end" => Some(n - 1),
                    // Reference: Page Up/Down jumps to the first cell of the
                    // previous or next category.
                    "pageup" => (0..category).rev().find_map(|c| category_first(c, s)),
                    "pagedown" => (category + 1..s.data.len()).find_map(|c| category_first(c, s)),
                    "escape" => None,
                    _ => return,
                };
                w.prevent_default();
                cx.stop_propagation();
                s.pointer = next;
                s.key_cursor = next.is_some();
                s.sync_focus(cx);
            }))
            .child({
                let paint_selected = selected.clone();
                canvas(
                    move |b, w, _| {
                        let previous = saved.get();
                        if previous.size != b.size
                            || intersects_viewport(previous, w.viewport_size())
                                != intersects_viewport(b, w.viewport_size())
                        {
                            w.request_animation_frame();
                        }
                        saved.set(b);
                    },
                    move |b, _, w, _| {
                        for (index, (x, y, alpha, dip, color, id)) in cells.iter().enumerate() {
                            let dim = if paint_selected.as_ref().is_none_or(|active| *active == *id)
                            {
                                1.
                            } else {
                                0.26
                            };
                            let pop = 0.4 + 0.6 * alpha;
                            let scale = pop * dip;
                            let square_size = 20.5 * scale;
                            let pitch = 23.5;
                            let origin = point(
                                b.origin.x + px(x * pitch + (20.5 - square_size) / 2.),
                                b.origin.y + px(y * pitch + (20.5 - square_size) / 2.),
                            );
                            let radius = square_size * 0.22;
                            if cursor == Some(index) {
                                // Reference cursor ring: a 2px surface gap and
                                // a 3.5px foreground-at-55% outline.
                                w.paint_quad(
                                    fill(
                                        Bounds {
                                            origin: point(origin.x - px(3.5), origin.y - px(3.5)),
                                            size: size(px(square_size + 7.), px(square_size + 7.)),
                                        },
                                        ink.opacity(0.55),
                                    )
                                    .corner_radii(px(radius + 3.5)),
                                );
                                w.paint_quad(
                                    fill(
                                        Bounds {
                                            origin: point(origin.x - px(2.), origin.y - px(2.)),
                                            size: size(px(square_size + 4.), px(square_size + 4.)),
                                        },
                                        surface,
                                    )
                                    .corner_radii(px(radius + 2.)),
                                );
                            }
                            w.paint_quad(
                                fill(
                                    Bounds {
                                        origin,
                                        size: size(px(square_size), px(square_size)),
                                    },
                                    color.opacity(alpha * dim),
                                )
                                .corner_radii(px(radius)),
                            );
                        }
                    },
                )
                .size_full()
            });
        let tip = self.active.and_then(|slot| {
            let category = *self.assignment.get(slot)?;
            let item = self.data.get(category)?;
            let total: f64 = self
                .data
                .iter()
                .filter(|v| !self.hidden.contains(&v.id))
                .map(|v| v.value)
                .sum();
            let share = if total > 0. {
                (item.value / total * 100.).round() as i64
            } else {
                0
            };
            Some((
                slot,
                item.label.clone(),
                share,
                item.value,
                Self::category_color(category, item, &p),
            ))
        });
        let board = 232.;
        let cell = 20.5;
        let pitch = 23.5;
        let graph = if let Some((slot, label, share, value, color)) = tip {
            let col = slot / 10;
            let cell_row = 9 - (slot % 10);
            let center = col as f32 * pitch + cell * 0.5;
            let cell_top = cell_row as f32 * pitch;
            let share_text: SharedString = format!("{share}%").into();
            let meta_text: SharedString = if self.unit.is_empty() {
                thousands(value)
            } else {
                format!("{} {}", thousands(value), self.unit).into()
            };
            let name_w = measure(&label, 14., w);
            let meta_w = measure(&meta_text, 12., w);
            let row_w = 12. + 10. + 8. + name_w + 8. + measure(&share_text, 14., w) + 12.;
            let tip_w = row_w.max(18. + meta_w + 12.).clamp(150., 240.);
            let tip_h = 56.;
            let left = (center - tip_w * 0.5).clamp(0., (board - tip_w).max(0.));
            let above = cell_top - tip_h - 10.;
            let top = if above >= 0. {
                above
            } else {
                cell_top + cell + 10.
            };
            graph.child(
                div()
                    .absolute()
                    .left(px(left))
                    .top(px(top))
                    .w(px(tip_w))
                    .px(px(12.))
                    .py(px(8.))
                    .rounded(px(14.))
                    .bg(p.panel)
                    .border_1()
                    .border_color(p.line)
                    .shadow_lg()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .text_size(px(14.))
                            .line_height(px(18.))
                            .child(div().size(px(10.)).flex_none().rounded(px(3.)).bg(color))
                            .child(
                                div()
                                    .flex_1()
                                    .overflow_hidden()
                                    .text_color(p.secondary)
                                    .child(label),
                            )
                            .child(
                                div()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .child(share_text),
                            ),
                    )
                    .child(
                        div()
                            .ml(px(18.))
                            .text_size(px(12.))
                            .line_height(px(20.))
                            .text_color(p.faint)
                            .child(meta_text),
                    ),
            )
        } else {
            graph
        };
        let focus = self
            .active
            .and_then(|slot| self.assignment.get(slot))
            .copied();
        self.counters.resize(self.data.len(), None);
        let legend: Vec<AnyElement> = self
            .data
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let on = focus == Some(i);
                let dim = focus.is_some() && !on;
                let counter = self.counters[i].get_or_insert_with(|| {
                    cx.new(|_| {
                        AnimatedCounter::new(counts[i] as f64)
                            .with_suffix("%")
                            .with_size(16.)
                            .with_wheel_style(1.2, 0.18, FontWeight::MEDIUM)
                            .with_accessibility(false)
                    })
                });
                counter.update(cx, |counter, cx| {
                    counter.set_visible(self.visible, cx);
                    counter.set_value(counts[i] as f64, cx);
                });
                let color = Self::category_color(i, item, &p);
                let aria_label = format!("{}: {}%", item.label, counts[i]);
                face(("waffle-legend", i), aria_label, 36., 12., Tone::Clear, cx)
                    .w_full()
                    .pl(px(10.))
                    .pr(px(4.))
                    .gap(px(10.))
                    .justify_start()
                    .hover(|s| s.text_color(p.ink))
                    .font_weight(FontWeight::NORMAL)
                    .text_color(if on { p.ink } else { p.secondary })
                    .when(on, |e| e.bg(p.ink.opacity(0.05)))
                    .opacity(if dim { 0.5 } else { 1. })
                    .aria_toggled((self.pinned == Some(i)).into())
                    .child(div().size(px(10.)).flex_none().rounded(px(3.)).bg(color))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .overflow_hidden()
                            .text_ellipsis()
                            .child(item.label.clone()),
                    )
                    .child(
                        div()
                            .min_w(px(51.))
                            .flex_none()
                            .text_align(TextAlign::Right)
                            .child(counter.clone()),
                    )
                    .on_hover(cx.listener(move |s, hovered: &bool, _, cx| {
                        if *hovered {
                            s.preview = Some(i);
                            s.sync_focus(cx);
                        }
                    }))
                    .on_click(cx.listener(move |s, _, _, cx| s.pin(i, cx)))
                    .on_key_down(cx.listener(move |s, e: &KeyDownEvent, w, cx| {
                        if matches!(e.keystroke.key.as_str(), "enter" | "space") && !e.is_held {
                            s.pin(i, cx);
                            w.prevent_default();
                            cx.stop_propagation();
                        }
                    }))
                    .into_any_element()
            })
            .collect();
        row()
            .w_full()
            .max_w_full()
            .items_center()
            .gap(px(24.))
            .text_color(p.ink)
            .child(graph)
            .child(
                div()
                    .id("waffle-legend")
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .on_hover(cx.listener(|s, hovered: &bool, _, cx| {
                        if !hovered && s.preview.is_some() {
                            s.preview = None;
                            s.sync_focus(cx);
                        }
                    }))
                    .children(legend),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::{Square, request_if_active, sample_cells, thousands};
    use crate::primitives::family::Slide;
    use gpui::{TestAppContext, rgb};
    use std::time::Duration;
    #[test]
    fn batch_schedules_once_and_retarget_is_continuous() {
        let app = TestAppContext::single();
        let mut cells = app.update(|cx| {
            (0..100)
                .map(|_| {
                    let mut x = Slide::new(0.);
                    x.duration(1., 0.5, cx);
                    Square {
                        id: "a".into(),
                        x,
                        y: Slide::new(0.),
                        appear: Slide::new(1.),
                        flight: Slide::new(1.),
                        color_progress: Slide::new(1.),
                        old_color: rgb(0xffffff).into(),
                        color: rgb(0xffffff).into(),
                    }
                })
                .collect::<Vec<_>>()
        });
        app.executor().advance_clock(Duration::from_millis(80));
        app.update(|cx| {
            let now = cx.background_executor().now();
            let (before, moving) = sample_cells(&cells, now, false);
            let mut frames = 0;
            request_if_active(true, moving, || frames += 1);
            request_if_active(false, moving, || frames += 1);
            assert_eq!(frames, 1);
            assert!(before.iter().all(|(.., id)| id == "a"));
            assert!(!sample_cells(&cells, now, true).1);
            cells[0].x.duration(0., 0.5, cx);
            let (after, _) = sample_cells(&cells, now, false);
            assert_eq!(before[0].0, after[0].0);
        });
    }
    #[test]
    fn idle_chart_requests_no_frames() {
        let mut frames = 0;
        request_if_active(true, false, || frames += 1);
        request_if_active(false, true, || frames += 1);
        request_if_active(true, true, || frames += 1);
        assert_eq!(frames, 1);
    }
    #[test]
    fn thousands_groups_like_the_reference_readout() {
        assert_eq!(thousands(218.), "218");
        assert_eq!(thousands(4178.), "4,178");
        assert_eq!(thousands(507.), "507");
    }
    #[test]
    fn cumulative_counts_match_the_reference_countries() {
        let chart = |values: &[f64]| {
            let cats: Vec<super::WaffleCategory> = values
                .iter()
                .enumerate()
                .map(|(i, &v)| super::WaffleCategory::new(i.to_string(), "", v))
                .collect();
            super::WaffleChart::new("t", cats).counts()
        };
        assert_eq!(
            chart(&[218., 20., 5., 76., 132., 56.]),
            vec![43, 4, 1, 15, 26, 11]
        );
        assert_eq!(
            chart(&[69., 59., 321., 30., 5., 10.]),
            vec![14, 12, 65, 6, 1, 2]
        );
        assert_eq!(
            chart(&[97., 6., 40., 100., 3., 39.]),
            vec![34, 2, 14, 35, 1, 14]
        );
        assert_eq!(
            chart(&[585., 251., 794., 1796., 668., 84.]),
            vec![14, 6, 19, 43, 16, 2]
        );
    }
}
