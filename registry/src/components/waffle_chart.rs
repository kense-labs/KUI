//! A hundred-cell chart with animated category reflow and keyboard inspection.
use crate::primitives::{
    family::{Paint, Slide, Tone, disabled_motion, face},
    presentation::tr,
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

/// A stable category identifier and the value it contributes to the whole.
#[derive(Clone, Debug)]
pub struct WaffleCategory {
    pub id: SharedString,
    pub label: SharedString,
    pub value: f64,
    pub color: Hsla,
}
impl WaffleCategory {
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        value: f64,
        color: Hsla,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            value: if value.is_finite() { value.max(0.) } else { 0. },
            color,
        }
    }
}
/// Emitted when a pointer or keyboard cursor selects a category.
#[derive(Clone, Debug)]
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
    color_progress: Slide,
    old_color: Hsla,
    color: Hsla,
}
/// The host owns the data; legend visibility and inspection are local state.
pub struct WaffleChart {
    title: SharedString,
    data: Vec<WaffleCategory>,
    hidden: Vec<SharedString>,
    cells: Vec<Square>,
    assignment: Vec<usize>,
    active: Option<usize>,
    pointer: Option<usize>,
    pinned: Option<usize>,
    preview: Option<usize>,
    bounds: Rc<Cell<Bounds<Pixels>>>,
    initialized: bool,
    visible: bool,
}
impl EventEmitter<WaffleInspect> for WaffleChart {}
impl WaffleChart {
    pub fn new(title: impl Into<SharedString>, categories: Vec<WaffleCategory>) -> Self {
        Self {
            title: title.into(),
            data: categories,
            hidden: vec![],
            cells: vec![],
            assignment: vec![],
            active: None,
            pointer: None,
            pinned: None,
            preview: None,
            bounds: Default::default(),
            initialized: false,
            visible: true,
        }
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
        let exact: Vec<f64> = values.iter().map(|v| v / total * 100.).collect();
        let mut counts: Vec<usize> = exact.iter().map(|v| v.floor() as usize).collect();
        let mut ranked: Vec<usize> = (0..values.len()).filter(|&i| values[i] > 0.).collect();
        ranked.sort_by(|&a, &b| {
            (exact[b] - counts[b] as f64)
                .total_cmp(&(exact[a] - counts[a] as f64))
                .then(a.cmp(&b))
        });
        let remaining = 100usize.saturating_sub(counts.iter().sum());
        for i in ranked.into_iter().take(remaining) {
            counts[i] += 1;
        }
        counts
    }
    fn reconcile(&mut self, cx: &App) {
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
            let reuse = existing.or_else(|| available.iter_mut().find_map(Option::take));
            let mut square = reuse.unwrap_or_else(|| Square {
                id: item.id.clone(),
                x: Slide::new(0.),
                y: Slide::new(9.),
                appear: Slide::new(0.),
                color_progress: Slide::new(1.),
                old_color: item.color,
                color: item.color,
            });
            if square.id != item.id {
                square.id = item.id.clone();
                square.old_color = square.color;
                square.color = item.color;
                square.color_progress.reset(0.);
                square.color_progress.duration(1., 0.42, cx);
            } else {
                square.color = item.color;
            }
            let x = (slot / 10) as f32;
            let y = 9. - (slot % 10) as f32;
            if first {
                square.x.reset(x);
                square.y.reset(y);
                square
                    .appear
                    .delayed(1., 0.24, slot as f32 / 100. * 0.32, cx);
            } else {
                square.x.duration(x, 0.42, cx);
                square.y.duration(y, 0.42, cx);
                square.appear.duration(1., 0.24, cx);
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
type CellFrame = (f32, f32, f32, Hsla, SharedString);
fn sample_cells(cells: &[Square], now: Instant, reduced: bool) -> (Vec<CellFrame>, bool) {
    let mut active = false;
    let values = cells
        .iter()
        .map(|c| {
            let (x, ax) = c.x.sample_batch(now, reduced);
            let (y, ay) = c.y.sample_batch(now, reduced);
            let (appearance, aa) = c.appear.sample_batch(now, reduced);
            let (color, ac) = c.color_progress.sample_batch(now, reduced);
            active |= ax || ay || aa || ac;
            (
                x,
                y,
                appearance.clamp(0., 1.),
                c.old_color.blend(c.color.opacity(color.clamp(0., 1.))),
                c.id.clone(),
            )
        })
        .collect();
    (values, active)
}
impl Render for WaffleChart {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.initialized {
            self.reconcile(cx);
        }
        let p = Paint::get(cx);
        let instructions = tr(cx, "Use arrow keys to inspect", "使用方向键查看");
        let counts = self.counts();
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
        let shapes: Vec<(SharedString, u8)> = self
            .data
            .iter()
            .enumerate()
            .map(|(index, item)| {
                (
                    item.id.clone(),
                    if index == 0 {
                        0
                    } else {
                        ((index - 1) % 3) as u8
                    },
                )
            })
            .collect();
        let cursor = self.active;
        let ink = p.ink;
        let surface = p.panel;
        let graph = div()
            .id("waffle-grid")
            .role(Role::Figure)
            .aria_label({
                let category = self
                    .active
                    .or((!self.assignment.is_empty()).then_some(0))
                    .and_then(|i| self.assignment.get(i))
                    .copied();
                if let Some((index, item)) =
                    category.and_then(|i| self.data.get(i).map(|item| (i, item)))
                {
                    format!(
                        "{}. {} {}%. {}",
                        self.title,
                        item.label,
                        counts.get(index).copied().unwrap_or(0),
                        instructions
                    )
                } else {
                    format!("{}. {}", self.title, instructions)
                }
            })
            .tab_index(0)
            .relative()
            .w(px(300.))
            .max_w_full()
            .h(px(300.))
            .on_mouse_move(cx.listener(|s, event: &MouseMoveEvent, _, cx| {
                let b = s.bounds.get();
                let x = f32::from(event.position.x - b.origin.x);
                let y = f32::from(event.position.y - b.origin.y);
                let side = f32::from(b.size.width).min(f32::from(b.size.height));
                let gap = 3.;
                let unit = ((side - gap * 9.) / 10.).max(1.);
                let pitch = unit + gap;
                let col = (x / pitch) as usize;
                let row = (y / pitch) as usize;
                let index = if side > 0.
                    && x >= 0.
                    && y >= 0.
                    && col < 10
                    && row < 10
                    && x < col as f32 * pitch + unit
                    && y < row as f32 * pitch + unit
                {
                    Some(col * 10 + (9 - row)).filter(|&slot| slot < s.assignment.len())
                } else {
                    None
                };
                if s.pointer != index {
                    s.pointer = index;
                    s.sync_focus(cx);
                }
            }))
            .on_hover(cx.listener(|s, hovered: &bool, _, cx| {
                if !hovered {
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
                let next = match e.keystroke.key.as_str() {
                    "right" => Some(((col + 1).min(9)) * 10 + within).filter(|&slot| slot < n),
                    "left" => Some(col.saturating_sub(1) * 10 + within),
                    "up" => Some(col * 10 + (within + 1).min(9)).filter(|&slot| slot < n),
                    "down" => Some(col * 10 + within.saturating_sub(1)),
                    "home" => Some(0),
                    "end" => Some(n - 1),
                    "escape" => None,
                    _ => return,
                };
                w.prevent_default();
                cx.stop_propagation();
                s.pointer = next;
                s.sync_focus(cx);
            }))
            .child({
                let paint_shapes = shapes.clone();
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
                        let side = f32::from(b.size.width).min(f32::from(b.size.height));
                        let gap = 3.;
                        let unit = ((side - gap * 9.) / 10.).max(0.);
                        let pitch = unit + gap;
                        for (index, (x, y, alpha, color, id)) in cells.iter().enumerate() {
                            let dim = if selected.as_ref().is_none_or(|active| *active == *id) {
                                1.
                            } else {
                                0.26
                            };
                            let square_size = unit * (0.4 + 0.6 * alpha);
                            let origin = point(
                                b.origin.x + px(x * pitch + (unit - square_size) / 2.),
                                b.origin.y + px(y * pitch + (unit - square_size) / 2.),
                            );
                            let shape = paint_shapes
                                .iter()
                                .find(|(key, _)| key == id)
                                .map(|(_, shape)| *shape)
                                .unwrap_or(0);
                            let radius = if shape == 1 {
                                square_size * 0.5
                            } else {
                                square_size * 0.22
                            };
                            if cursor == Some(index) {
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
                            if shape == 2 {
                                let inset = (square_size * 0.22).clamp(2.5, 4.);
                                w.paint_quad(
                                    fill(
                                        Bounds {
                                            origin: point(
                                                origin.x + px(inset),
                                                origin.y + px(inset),
                                            ),
                                            size: size(
                                                px((square_size - inset * 2.).max(0.)),
                                                px((square_size - inset * 2.).max(0.)),
                                            ),
                                        },
                                        surface,
                                    )
                                    .corner_radii(px((radius - inset).max(0.))),
                                );
                            }
                        }
                    },
                )
                .size_full()
            });
        let tip = self.active.and_then(|slot| {
            let category = *self.assignment.get(slot)?;
            let item = self.data.get(category)?;
            Some((
                slot / 10,
                9 - (slot % 10),
                item.label.clone(),
                counts[category],
                item.value,
                item.color,
                shapes
                    .iter()
                    .find(|(id, _)| id == &item.id)
                    .map(|(_, shape)| *shape)
                    .unwrap_or(0),
            ))
        });
        let measured = f32::from(self.bounds.get().size.width);
        let board = if measured > 1. { measured } else { 300. };
        let cell = ((board - 27.) / 10.).max(0.);
        let pitch = cell + 3.;
        let graph = if let Some((col, row, label, share, value, color, shape)) = tip {
            let center = col as f32 * pitch + cell * 0.5;
            let cell_top = row as f32 * pitch;
            let tip_w = 168.;
            let tip_h = 52.;
            let left = (center - tip_w * 0.5).clamp(0., (board - tip_w).max(0.));
            let above = cell_top - tip_h - 10.;
            let top = if above >= 0. {
                above
            } else {
                cell_top + cell + 10.
            };
            let radius = if shape == 1 { 5. } else { 2.2 };
            graph.child(
                div()
                    .absolute()
                    .left(px(left))
                    .top(px(top))
                    .w(px(tip_w))
                    .px(px(12.))
                    .py(px(8.))
                    .min_w(px(150.))
                    .max_w(px(240.))
                    .rounded(px(8.))
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
                            .child(
                                div()
                                    .size(px(10.))
                                    .flex_none()
                                    .rounded(px(radius))
                                    .bg(color),
                            )
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
                                    .child(format!("{share}%")),
                            ),
                    )
                    .child(
                        div()
                            .ml(px(18.))
                            .text_size(px(12.))
                            .line_height(px(20.))
                            .text_color(p.faint)
                            .child(format!("{value:.1}")),
                    ),
            )
        } else {
            graph
        };
        let focus = self
            .active
            .and_then(|slot| self.assignment.get(slot))
            .copied();
        div()
            .w(px(560.))
            .max_w_full()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(24.))
            .text_color(p.ink)
            .child(graph)
            .child(
                div()
                    .id("waffle-legend")
                    .flex_1()
                    .min_w(px(180.))
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .id("waffle-readout")
                    .on_hover(cx.listener(|s, hovered: &bool, _, cx| {
                        if !hovered && s.preview.is_some() {
                            s.preview = None;
                            s.sync_focus(cx);
                        }
                    }))
                    .children(self.data.iter().enumerate().map(|(i, item)| {
                        let on = focus == Some(i);
                        let dim = focus.is_some() && !on;
                        face(
                            ("waffle-legend", i),
                            format!("{}: {}%", item.label, counts[i]),
                            36.,
                            8.,
                            Tone::Clear,
                            cx,
                        )
                        .w_full()
                        .px(px(10.))
                        .gap(px(10.))
                        .justify_start()
                        .hover(|s| s.text_color(p.ink))
                        .font_weight(FontWeight::NORMAL)
                        .text_color(if on { p.ink } else { p.secondary })
                        .when(on, |e| e.bg(p.ink.opacity(0.05)))
                        .opacity(if dim { 0.5 } else { 1. })
                        .aria_toggled((self.pinned == Some(i)).into())
                        .child(
                            div()
                                .size(px(10.))
                                .flex_none()
                                .rounded(px(if i == 0 || (i - 1) % 3 != 1 { 2.2 } else { 5. }))
                                .bg(item.color),
                        )
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
                                .min_w(px(48.))
                                .text_align(TextAlign::Right)
                                .font_weight(FontWeight::MEDIUM)
                                .text_size(px(16.))
                                .text_color(p.ink)
                                .child(format!("{}%", counts[i])),
                        )
                        .on_hover(cx.listener(move |s, hovered: &bool, _, cx| {
                            if *hovered {
                                s.preview = Some(i);
                                s.sync_focus(cx);
                            }
                        }))
                        .on_click(cx.listener(move |s, _, _, cx| s.pin(i, cx)))
                        .on_key_down(cx.listener(
                            move |s, e: &KeyDownEvent, w, cx| {
                                if matches!(e.keystroke.key.as_str(), "enter" | "space")
                                    && !e.is_held
                                {
                                    s.pin(i, cx);
                                    w.prevent_default();
                                    cx.stop_propagation();
                                }
                            },
                        ))
                    })),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::{Square, request_if_active, sample_cells};
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
            request_if_active(true, moving, || frames += 1);
            assert_eq!(frames, 2);
            cells[0].x.duration(0., 0.5, cx);
            let (after, _) = sample_cells(&cells, now, false);
            assert_eq!(before[0].0, after[0].0);
            assert!(!sample_cells(&cells, now, true).1);
        });
    }
}
