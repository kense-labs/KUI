//! An interactive radial comparison with animated, host-supplied profiles.
use crate::primitives::{
    family::{Paint, Slide, disabled_motion},
    presentation::{column, row, tr},
};
use gpui::{prelude::*, *};
use std::{cell::Cell, f32::consts::TAU, rc::Rc};

/// A named dimension in a radial comparison.
#[derive(Clone, Debug)]
pub struct RadarAxis {
    pub id: SharedString,
    pub label: SharedString,
}
impl RadarAxis {
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
        }
    }
}
/// A named series. Values correspond to axes in order and are clamped to 0–100.
#[derive(Clone, Debug)]
pub struct RadarProfile {
    pub id: SharedString,
    pub label: SharedString,
    pub values: Vec<f32>,
    pub color: Hsla,
}
impl RadarProfile {
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        values: Vec<f32>,
        color: Hsla,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            values,
            color,
        }
    }
    fn value(&self, index: usize) -> f32 {
        self.values
            .get(index)
            .copied()
            .filter(|v| v.is_finite())
            .unwrap_or(0.)
            .clamp(0., 100.)
    }
}
/// Selected dimension and its values in profile order.
#[derive(Clone, Debug)]
pub struct RadarInspect {
    pub axis_id: SharedString,
    pub values: Vec<(SharedString, f32)>,
}
struct VisualProfile {
    id: SharedString,
    values: Vec<Slide>,
}

/// Side of the stage an axis label hangs from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LabelAnchor {
    Center,
    Start,
    End,
}

fn radar_metrics(stage_w: f32) -> (f32, f32) {
    let stage_w = if stage_w > 1. { stage_w } else { 320. };
    let size = stage_w.clamp(96., 448.);
    let max_r = (size / 2. - 64.).max(8.);
    (size, max_r)
}
fn axis_angle(i: usize, n: usize) -> f32 {
    i as f32 * TAU / n.max(1) as f32
}
/// Axis point at radius `r`, stage-local, starting at the top going clockwise.
fn stage_point(cx: f32, cy: f32, i: usize, n: usize, r: f32) -> (f32, f32) {
    let a = axis_angle(i, n);
    (cx + a.sin() * r, cy - a.cos() * r)
}
fn label_anchor(i: usize, n: usize) -> LabelAnchor {
    let sin = axis_angle(i, n).sin();
    if sin.abs() < 0.25 {
        LabelAnchor::Center
    } else if sin > 0. {
        LabelAnchor::Start
    } else {
        LabelAnchor::End
    }
}
/// Horizontal extent of an axis label box, matching the rendered label.
fn label_box(stage_w: f32, max_r: f32, i: usize, n: usize) -> (f32, f32) {
    let a = axis_angle(i, n);
    let x = stage_w / 2. + a.sin() * (max_r + 10.);
    match label_anchor(i, n) {
        LabelAnchor::Center => (x - 48., 96.),
        LabelAnchor::Start => (x, 72.),
        LabelAnchor::End => (x - 72., 72.),
    }
}
/// Tooltip top-left, stage-local: beside the axis label, centred on the
/// spoke end raised 12px, clamped inside the stage.
fn tooltip_anchor(
    stage_w: f32,
    size: f32,
    max_r: f32,
    i: usize,
    n: usize,
    tip_w: f32,
    tip_h: f32,
) -> (f32, f32) {
    let (_, sy) = stage_point(stage_w / 2., size / 2., i, n, max_r);
    let (lx, lw) = label_box(stage_w, max_r, i, n);
    let mut tx = lx + lw + 12.;
    if tx + tip_w > stage_w - 8. {
        tx = lx - 12. - tip_w;
    }
    let tx = tx.clamp(8., (stage_w - tip_w - 8.).max(8.));
    let ty = (sy - 12. - tip_h / 2.).clamp(8., (size - tip_h - 8.).max(8.));
    (tx, ty)
}
struct TipRow {
    label: SharedString,
    color: Hsla,
    value: f32,
    gap: Option<String>,
}
/// Tooltip rows for one axis, ordered by that axis's value, descending.
fn tip_rows(profiles: &[RadarProfile], axis: usize) -> Vec<TipRow> {
    let mut rows: Vec<(usize, f32)> = profiles
        .iter()
        .enumerate()
        .map(|(slot, p)| (slot, p.value(axis)))
        .collect();
    rows.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let first = rows.first().map(|(_, v)| *v).unwrap_or(0.);
    rows.into_iter()
        .enumerate()
        .map(|(rank, (slot, value))| {
            let profile = &profiles[slot];
            TipRow {
                label: profile.label.clone(),
                color: profile.color,
                value,
                gap: (rank > 0).then(|| minus_number(value - first)),
            }
        })
        .collect()
}
/// Signed difference with the true minus sign (U+2212).
fn minus_number(diff: f32) -> String {
    let v = diff.round();
    if v < 0. {
        format!("\u{2212}{}", -(v as i64))
    } else {
        format!("{v:.0}")
    }
}
fn tip_size(profiles: &[RadarProfile]) -> (f32, f32) {
    let rows = profiles.len().max(1);
    let longest = profiles.iter().map(|p| p.label.len()).max().unwrap_or(0) as f32;
    let w = (24. + 10. + 8. + longest * 7.2 + 8. + 34. + 31.).clamp(164., 240.);
    let h = 20. + 2. + 20. + 2. + rows as f32 * 20. + 2.;
    (w, h)
}

/// Radial comparison. Three to twelve dimensions are supported.
pub struct RadarChart {
    title: SharedString,
    meta: Option<SharedString>,
    header_extra: Option<AnyView>,
    axes: Vec<RadarAxis>,
    profiles: Vec<RadarProfile>,
    visual: Vec<VisualProfile>,
    markers: Vec<(Slide, Slide)>,
    marker_shown: Slide,
    tip_x: Slide,
    tip_y: Slide,
    tip_shown: Slide,
    axis_dim: Slide,
    focus_mix: Slide,
    focused: Option<usize>,
    active: Option<usize>,
    bounds: Rc<Cell<Bounds<Pixels>>>,
    last_center: Cell<(f32, f32)>,
    initialized: bool,
    reveal: Slide,
    revealed: bool,
    visible: bool,
    touching: Rc<Cell<bool>>,
}
impl EventEmitter<RadarInspect> for RadarChart {}
impl RadarChart {
    pub fn new(
        title: impl Into<SharedString>,
        mut axes: Vec<RadarAxis>,
        profiles: Vec<RadarProfile>,
    ) -> Self {
        axes.truncate(12);
        Self {
            title: title.into(),
            meta: None,
            header_extra: None,
            axes,
            profiles,
            visual: vec![],
            markers: vec![],
            marker_shown: Slide::new(0.),
            tip_x: Slide::new(0.),
            tip_y: Slide::new(0.),
            tip_shown: Slide::new(0.),
            axis_dim: Slide::new(0.),
            focus_mix: Slide::new(0.),
            focused: None,
            active: None,
            bounds: Default::default(),
            last_center: Cell::new((0., 0.)),
            initialized: false,
            reveal: Slide::new(0.),
            revealed: false,
            visible: true,
            touching: Default::default(),
        }
    }
    /// A one-line context note rendered under the title.
    pub fn with_meta(mut self, meta: impl Into<SharedString>) -> Self {
        self.meta = Some(meta.into());
        self
    }
    /// Sets the one-line context note rendered under the title.
    pub fn set_meta(&mut self, meta: Option<SharedString>, cx: &mut Context<Self>) {
        if self.meta != meta {
            self.meta = meta;
            cx.notify();
        }
    }
    /// Host content (e.g. a comparison selector) shown at the heading's side.
    pub fn with_header_extra(mut self, view: AnyView) -> Self {
        self.header_extra = Some(view);
        self
    }
    /// Sets host content shown at the heading's side.
    pub fn set_header_extra(&mut self, view: Option<AnyView>, cx: &mut Context<Self>) {
        if self.header_extra.is_some() != view.is_some() {
            cx.notify();
        }
        self.header_extra = view;
    }
    /// The host sets this to false when the chart or its tab is hidden.
    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        if self.visible != visible {
            self.visible = visible;
            cx.notify();
        }
    }
    /// Update profile values while retaining animated positions of matching IDs.
    pub fn set_profiles(&mut self, profiles: Vec<RadarProfile>, cx: &mut Context<Self>) {
        let mut old: Vec<Option<VisualProfile>> = std::mem::take(&mut self.visual)
            .into_iter()
            .map(Some)
            .collect();
        self.visual = profiles
            .iter()
            .map(|p| {
                let previous = old
                    .iter()
                    .position(|v| v.as_ref().is_some_and(|v| v.id == p.id))
                    .and_then(|i| old[i].take());
                let mut state = previous.unwrap_or_else(|| VisualProfile {
                    id: p.id.clone(),
                    values: vec![],
                });
                while state.values.len() < self.axes.len() {
                    state.values.push(Slide::new(0.));
                }
                state.values.truncate(self.axes.len());
                for (i, slide) in state.values.iter_mut().enumerate() {
                    slide.duration(p.value(i), 0.45, cx);
                }
                state
            })
            .collect();
        self.markers
            .resize(profiles.len(), (Slide::new(0.), Slide::new(0.)));
        self.focused = self.focused.filter(|i| *i < profiles.len());
        self.profiles = profiles;
        self.initialized = true;
        if self.active.is_some() {
            self.retarget(cx, false);
        }
        cx.notify();
    }
    fn select(&mut self, next: Option<usize>, cx: &mut Context<Self>) {
        if self.active == next {
            return;
        }
        let first = self.active.is_none() && next.is_some();
        self.active = next;
        if let Some((axis, i)) = next.and_then(|i| self.axes.get(i).map(|a| (a, i))) {
            cx.emit(RadarInspect {
                axis_id: axis.id.clone(),
                values: self
                    .profiles
                    .iter()
                    .map(|p| (p.id.clone(), p.value(i)))
                    .collect(),
            });
        }
        self.axis_dim
            .duration(if next.is_some() { 1. } else { 0. }, 0.16, cx);
        self.retarget(cx, first);
        cx.notify();
    }
    /// Re-aims the markers and the tooltip at the active axis.
    fn retarget(&mut self, cx: &mut Context<Self>, snap: bool) {
        let shown = if self.active.is_some() { 1. } else { 0. };
        if snap {
            // A zero-length spring would divide by zero; land immediately.
            self.marker_shown.reset(shown);
            self.tip_shown.reset(shown);
        } else {
            self.marker_shown.duration(shown, 0.16, cx);
            self.tip_shown.duration(shown, 0.16, cx);
        }
        let b = self.bounds.get();
        let width = f32::from(b.size.width);
        let (size, max_r) = radar_metrics(width);
        let cxp = width / 2.;
        let cyp = size / 2.;
        let n = self.axes.len();
        if let Some(i) = self.active.filter(|_| n >= 3) {
            for (j, marker) in self.markers.iter_mut().enumerate() {
                let r = max_r * self.profiles.get(j).map_or(0., |p| p.value(i)) / 100.;
                let (x, y) = stage_point(cxp, cyp, i, n, r);
                if snap {
                    marker.0.reset(x);
                    marker.1.reset(y);
                } else {
                    marker.0.duration(x, 0.16, cx);
                    marker.1.duration(y, 0.16, cx);
                }
            }
            let (tw, th) = tip_size(&self.profiles);
            let (tx, ty) = tooltip_anchor(width, size, max_r, i, n, tw, th);
            if snap {
                self.tip_x.reset(tx);
                self.tip_y.reset(ty);
            } else {
                self.tip_x.duration(tx, 0.16, cx);
                self.tip_y.duration(ty, 0.16, cx);
            }
        }
    }
    fn toggle_focus(&mut self, slot: usize, cx: &mut Context<Self>) {
        self.focused = if self.focused == Some(slot) {
            None
        } else {
            Some(slot)
        };
        self.focus_mix
            .duration(if self.focused.is_some() { 1. } else { 0. }, 0.16, cx);
        cx.notify();
    }
}
fn intersects_viewport(b: Bounds<Pixels>, viewport: Size<Pixels>) -> bool {
    b.size.width > px(0.)
        && b.size.height > px(0.)
        && b.origin.x < viewport.width
        && b.origin.y < viewport.height
        && b.origin.x + b.size.width > px(0.)
        && b.origin.y + b.size.height > px(0.)
}
fn axis_at(b: Bounds<Pixels>, pos: Point<Pixels>, count: usize) -> Option<usize> {
    if count < 3 {
        return None;
    }
    let width = f32::from(b.size.width);
    let (size, max_r) = radar_metrics(width);
    let dx = f32::from(pos.x - b.origin.x) - width / 2.;
    let dy = f32::from(pos.y - b.origin.y) - size / 2.;
    let d = dx.hypot(dy);
    if d > max_r + 25. || d < 8. {
        return None;
    }
    let angle = dx.atan2(-dy).rem_euclid(TAU);
    Some(((angle / TAU * count as f32).round() as usize) % count)
}
impl Render for RadarChart {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.initialized {
            self.initialized = true;
            self.visual = self
                .profiles
                .iter()
                .map(|p| VisualProfile {
                    id: p.id.clone(),
                    values: (0..self.axes.len())
                        .map(|i| Slide::new(p.value(i)))
                        .collect(),
                })
                .collect();
            self.markers = self
                .profiles
                .iter()
                .map(|_| (Slide::new(0.), Slide::new(0.)))
                .collect();
        }
        let measured = self.bounds.get();
        let on_screen =
            self.visible && w.is_visible() && intersects_viewport(measured, w.viewport_size());
        if on_screen && !self.revealed {
            self.revealed = true;
            self.reveal.duration(1., 0.9, cx);
        }
        let now = cx.background_executor().now();
        let reduced = disabled_motion(cx);
        let (reveal, mut moving) = self.reveal.sample_batch(now, reduced);
        let reveal = reveal.clamp(0., 1.);
        let p = Paint::get(cx);
        let n = self.axes.len();
        let empty = n < 3 || self.profiles.is_empty();
        let width = if measured.size.width > px(0.) {
            f32::from(measured.size.width)
        } else {
            320.
        };
        let (svg_size, max_r) = radar_metrics(width);
        let cxp = width / 2.;
        let cyp = svg_size / 2.;
        if self.last_center.get() != (cxp, cyp) {
            self.last_center.set((cxp, cyp));
            if !empty {
                self.retarget(cx, true);
            }
        }
        let (axis_dim, dim_moving) = self.axis_dim.sample_batch(now, reduced);
        let (focus_mix, focus_moving) = self.focus_mix.sample_batch(now, reduced);
        let (marker_shown, marker_moving) = self.marker_shown.sample_batch(now, reduced);
        let (tip_shown, tip_moving) = self.tip_shown.sample_batch(now, reduced);
        let (tip_x, tip_x_moving) = self.tip_x.sample_batch(now, reduced);
        let (tip_y, tip_y_moving) = self.tip_y.sample_batch(now, reduced);
        moving |= dim_moving
            || focus_moving
            || marker_moving
            || tip_moving
            || tip_x_moving
            || tip_y_moving;
        let visual: Vec<(Vec<f32>, Hsla)> = self
            .visual
            .iter()
            .zip(&self.profiles)
            .map(|(v, s)| {
                let values = v
                    .values
                    .iter()
                    .map(|slide| {
                        let (value, active) = slide.sample_batch(now, reduced);
                        moving |= active;
                        value
                    })
                    .collect();
                (values, s.color)
            })
            .collect();
        let markers: Vec<(f32, f32, Hsla)> = self
            .markers
            .iter()
            .zip(&self.profiles)
            .map(|((mx, my), s)| {
                let (x, x_active) = mx.sample_batch(now, reduced);
                let (y, y_active) = my.sample_batch(now, reduced);
                moving |= x_active || y_active;
                (x, y, s.color)
            })
            .collect();
        if on_screen && moving {
            w.request_animation_frame();
        }

        // Axis labels hang off each spoke end, anchored by their side.
        let labels: Vec<_> = (0..n)
            .map(|i| {
                let anchor = label_anchor(i, n);
                let a = axis_angle(i, n);
                let x = cxp + a.sin() * (max_r + 10.);
                let y = cyp - a.cos() * (max_r + 12.);
                let (lw, align) = match anchor {
                    LabelAnchor::Center => (96., TextAlign::Center),
                    LabelAnchor::Start => (72., TextAlign::Left),
                    LabelAnchor::End => (72., TextAlign::Right),
                };
                let left = match anchor {
                    LabelAnchor::Center => x - lw / 2.,
                    LabelAnchor::Start => x,
                    LabelAnchor::End => x - lw,
                };
                let on = self.active == Some(i);
                let opacity = if on || self.active.is_none() {
                    1.
                } else {
                    1. - 0.45 * axis_dim
                };
                div()
                    .absolute()
                    .left(px(left.clamp(0., (width - lw).max(0.))))
                    .top(px((y - 7.5).max(0.)))
                    .w(px(lw))
                    .text_align(align)
                    .text_size(px(12.))
                    .line_height(px(15.))
                    .text_color(if on { p.ink } else { p.secondary })
                    .opacity(opacity)
                    .overflow_hidden()
                    .child(self.axes[i].label.clone())
            })
            .collect();

        let touching = self.touching.clone();
        let touch = cx.listener(|s, e: &TouchDragEvent, _, cx| {
            s.select(axis_at(s.bounds.get(), e.position, s.axes.len()), cx);
        });
        let bounds = self.bounds.clone();
        let grid = p.line;
        let ink = p.ink;
        let panel = p.panel;
        let active = self.active;
        let focused = self.focused;
        let stage = div()
            .id("radar-chart")
            .role(Role::Group)
            .aria_label({
                let instructions = tr(
                    cx,
                    "Use arrow keys to move between measures",
                    "使用方向键在各指标之间移动",
                );
                let names = self
                    .profiles
                    .iter()
                    .map(|profile| profile.label.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                if names.is_empty() {
                    format!("{}. {}", self.title, instructions)
                } else {
                    format!("{}. {}. {}", self.title, names, instructions)
                }
            })
            .tab_index(0)
            .relative()
            .w_full()
            .h(px(if empty { 200. } else { svg_size }))
            .min_h(px(200.))
            .on_mouse_move(cx.listener(|s, e: &MouseMoveEvent, _, cx| {
                s.select(axis_at(s.bounds.get(), e.position, s.axes.len()), cx);
            }))
            .on_hover(cx.listener(|s, hovered: &bool, _, cx| {
                if !hovered {
                    s.select(None, cx);
                }
            }))
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                let n = s.axes.len();
                if n < 3 {
                    return;
                }
                let next = match e.keystroke.key.as_str() {
                    "right" | "down" => Some(s.active.map_or(0, |i| (i + 1) % n)),
                    "left" | "up" => Some(s.active.map_or(0, |i| (i + n - 1) % n)),
                    "home" => Some(0),
                    "end" => Some(n - 1),
                    "escape" => None,
                    _ => return,
                };
                w.prevent_default();
                cx.stop_propagation();
                s.select(next, cx);
            }))
            .child(
                canvas(
                    move |b, w, _| {
                        let previous = bounds.get();
                        if previous.size != b.size
                            || intersects_viewport(previous, w.viewport_size())
                                != intersects_viewport(b, w.viewport_size())
                        {
                            w.request_animation_frame();
                        }
                        bounds.set(b);
                    },
                    move |b, _, w, _| {
                        if n < 3 {
                            return;
                        }
                        let width = f32::from(b.size.width);
                        let (svg_size, chart_r) = radar_metrics(width);
                        let cx = b.origin.x + px(width / 2.);
                        let cy = b.origin.y + px(svg_size / 2.);
                        let radius = chart_r * reveal;
                        let at = |i: usize, r: f32| {
                            let a = axis_angle(i, n);
                            point(cx + px(a.sin() * r), cy - px(a.cos() * r))
                        };
                        for level in 1..=4 {
                            let mut path = PathBuilder::stroke(px(1.));
                            path.move_to(at(0, radius * level as f32 / 4.));
                            for i in 1..n {
                                path.line_to(at(i, radius * level as f32 / 4.));
                            }
                            path.close();
                            if let Ok(path) = path.build() {
                                w.paint_path(path, grid);
                            }
                        }
                        for i in 0..n {
                            let mut path = PathBuilder::stroke(px(1.));
                            path.move_to(at(i, 0.));
                            path.line_to(at(i, radius));
                            if let Ok(path) = path.build() {
                                w.paint_path(
                                    path,
                                    if active == Some(i) {
                                        ink.opacity(0.45)
                                    } else {
                                        grid
                                    },
                                );
                            }
                        }
                        for (slot, (values, color)) in visual.iter().enumerate() {
                            let pt = |i: usize| {
                                at(i, radius * values.get(i).copied().unwrap_or(0.) / 100.)
                            };
                            let base = if slot == 0 { 0.12 } else { 0.06 };
                            let on = focused == Some(slot);
                            let fill = if on {
                                base + (0.2 - base) * focus_mix
                            } else {
                                base
                            };
                            let dim = if focused.is_some() && !on {
                                1. - 0.78 * focus_mix
                            } else {
                                1.
                            };
                            let mut area = PathBuilder::fill();
                            area.move_to(pt(0));
                            for i in 1..n {
                                area.line_to(pt(i));
                            }
                            area.close();
                            if let Ok(path) = area.build() {
                                w.paint_path(path, color.opacity(fill * dim));
                            }
                            let mut line = PathBuilder::stroke(px(2.));
                            line.move_to(pt(0));
                            for i in 1..n {
                                line.line_to(pt(i));
                            }
                            line.close();
                            if let Ok(path) = line.build() {
                                w.paint_path(path, color.opacity(dim));
                            }
                        }
                        if marker_shown > 0.01 {
                            for (mx, my, color) in markers.iter() {
                                let center = point(b.origin.x + px(*mx), b.origin.y + px(*my));
                                w.paint_quad(
                                    fill(
                                        Bounds {
                                            origin: point(center.x - px(5.5), center.y - px(5.5)),
                                            size: size(px(11.), px(11.)),
                                        },
                                        panel,
                                    )
                                    .corner_radii(px(5.5)),
                                );
                                w.paint_quad(
                                    fill(
                                        Bounds {
                                            origin: point(center.x - px(4.5), center.y - px(4.5)),
                                            size: size(px(9.), px(9.)),
                                        },
                                        color.opacity(marker_shown),
                                    )
                                    .corner_radii(px(4.5)),
                                );
                            }
                        }
                    },
                )
                .size_full(),
            )
            .children(labels)
            .when(empty, |stage| {
                stage.child(
                    div()
                        .absolute()
                        .inset_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(14.))
                        .line_height(px(20.))
                        .text_color(p.faint)
                        .child(tr(cx, "No data to compare yet", "暂无可比较的数据")),
                )
            })
            .child(
                canvas(
                    |b, w, _| w.insert_hitbox(b, HitboxBehavior::Normal),
                    move |_, hit, w, _| {
                        w.on_mouse_event(move |e: &TouchDragEvent, phase, w, cx| {
                            if !phase.bubble() {
                                return;
                            }
                            if e.phase == TouchPhase::Started {
                                touching.set(hit.is_hovered(w));
                            }
                            if !touching.get() {
                                return;
                            }
                            let dx = f32::from(e.position.x - e.start_position.x).abs();
                            let dy = f32::from(e.position.y - e.start_position.y).abs();
                            if e.phase == TouchPhase::Ended && dx < 6. && dy < 6. {
                                touch(e, w, cx);
                            }
                            if matches!(e.phase, TouchPhase::Ended | TouchPhase::Cancelled) {
                                touching.set(false);
                            }
                        });
                    },
                )
                .absolute()
                .inset_0(),
            );
        // Reading tooltip: rows ordered by the axis value, gaps to the leader.
        let stage = if let Some(i) = self.active.filter(|_| !empty) {
            let rows = tip_rows(&self.profiles, i);
            let mut tip = div()
                .absolute()
                .left(px(tip_x))
                .top(px(tip_y))
                .min_w(px(164.))
                .max_w(px(240.))
                .px(px(12.))
                .py(px(10.))
                .flex()
                .flex_col()
                .gap(px(4.))
                .rounded(px(14.))
                .border_1()
                .border_color(p.line)
                .bg(p.tray)
                .shadow_lg()
                .opacity(tip_shown)
                .child(
                    div()
                        .mb(px(2.))
                        .text_size(px(12.))
                        .line_height(px(20.))
                        .text_color(p.secondary)
                        .child(self.axes[i].label.clone()),
                );
            for r in rows {
                tip = tip.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(div().w(px(10.)).h(px(2.5)).rounded(px(2.)).bg(r.color))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .text_size(px(14.))
                                .line_height(px(20.))
                                .text_color(p.secondary)
                                .child(r.label),
                        )
                        .child(
                            div()
                                .text_size(px(14.))
                                .line_height(px(20.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.ink)
                                .child(format!("{:.0}", r.value)),
                        )
                        .child(
                            div()
                                .min_w(px(31.))
                                .text_size(px(12.))
                                .line_height(px(16.))
                                .text_right()
                                .text_color(p.faint)
                                .when_some(r.gap, |v, gap| v.child(gap)),
                        ),
                );
            }
            stage.child(tip)
        } else {
            stage
        };
        // Legend keys toggle one profile forward and dim the rest.
        let legend = row()
            .flex_wrap()
            .justify_center()
            .gap_x(px(4.))
            .gap_y(px(2.))
            .children(self.profiles.iter().enumerate().map(|(slot, series)| {
                let pressed = focused == Some(slot);
                let dimmed = focused.is_some() && !pressed;
                div()
                    .id(("radar-key", slot))
                    .role(Role::Button)
                    .aria_label(series.label.clone())
                    .h(px(32.))
                    .px(px(10.))
                    .rounded_full()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .text_size(px(14.))
                    .line_height(px(20.))
                    .text_color(if pressed { p.ink } else { p.secondary })
                    .when(pressed, |v| v.bg(p.ink.opacity(0.05)))
                    .opacity(1. - 0.5 * focus_mix * dimmed as u8 as f32)
                    .hover(|s| s.text_color(p.ink))
                    .child(div().w(px(14.)).h(px(2.5)).rounded(px(2.)).bg(series.color))
                    .child(div().whitespace_nowrap().child(series.label.clone()))
                    .on_click(cx.listener(move |s, _, _, cx| s.toggle_focus(slot, cx)))
                    .on_key_down(cx.listener(move |s, e: &KeyDownEvent, w, cx| {
                        if matches!(e.keystroke.key.as_str(), "enter" | "space") && !e.is_held {
                            s.toggle_focus(slot, cx);
                            w.prevent_default();
                            cx.stop_propagation();
                        }
                    }))
            }));
        let heading = div()
            .flex()
            .flex_col()
            .gap(px(2.))
            .child(
                div()
                    .text_size(px(16.))
                    .line_height(px(20.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.ink)
                    .child(self.title.clone()),
            )
            .when_some(self.meta.clone(), |v, meta| {
                v.child(
                    div()
                        .text_size(px(14.))
                        .line_height(px(20.))
                        .text_color(p.faint)
                        .child(meta),
                )
            });
        let top = row()
            .flex_wrap()
            .items_start()
            .justify_between()
            .gap(px(12.))
            .child(heading)
            .when_some(self.header_extra.clone(), |v, extra| v.child(extra));
        let narrow = w.viewport_size().width < px(640.);
        column()
            .w(px(600.))
            .max_w_full()
            .gap(px(24.))
            .map(|card| {
                if narrow {
                    card.py(px(20.)).px(px(16.))
                } else {
                    card.p(px(24.))
                }
            })
            .rounded(crate::foundation::surface_radius(cx))
            .bg(p.panel)
            .border_1()
            .border_color(p.line)
            .text_color(p.ink)
            .child(top)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(16.))
                    .min_w_0()
                    .child(stage)
                    .child(legend),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::{
        LabelAnchor, RadarAxis, RadarProfile, axis_at, label_anchor, minus_number, radar_metrics,
        stage_point, tip_rows, tip_size, tooltip_anchor,
    };
    use gpui::{Bounds, point, px, rgb, size};

    fn profiles() -> Vec<RadarProfile> {
        vec![
            RadarProfile::new("a", "Lisbon", vec![64.], rgb(0x55adff).into()),
            RadarProfile::new("b", "Berlin", vec![86.], rgb(0x9c37be).into()),
        ]
    }
    #[test]
    fn svg_is_capped_at_448_with_a_64_gutter() {
        assert_eq!(radar_metrics(550.), (448., 160.));
        assert_eq!(radar_metrics(300.), (300., 86.));
        assert_eq!(radar_metrics(740.), (448., 160.));
    }
    #[test]
    fn axes_start_at_the_top_and_run_clockwise() {
        assert_eq!(stage_point(275., 224., 0, 6, 160.), (275., 64.));
        let (x, y) = stage_point(275., 224., 1, 6, 160.);
        assert!((x - 413.6).abs() < 0.1 && (y - 144.).abs() < 0.1);
    }
    #[test]
    fn labels_anchor_center_start_and_end_by_side() {
        assert_eq!(label_anchor(0, 6), LabelAnchor::Center);
        assert_eq!(label_anchor(1, 6), LabelAnchor::Start);
        assert_eq!(label_anchor(2, 6), LabelAnchor::Start);
        assert_eq!(label_anchor(3, 6), LabelAnchor::Center);
        assert_eq!(label_anchor(4, 6), LabelAnchor::End);
        assert_eq!(label_anchor(5, 6), LabelAnchor::End);
    }
    #[test]
    fn tooltip_sits_beside_the_top_label_matching_the_reference() {
        let (w, h) = tip_size(&profiles());
        let (x, y) = tooltip_anchor(550., 448., 160., 0, 6, w, h);
        assert!((x - 335.).abs() < 1., "x {x}");
        assert!((y - 9.5).abs() < 1.5, "y {y}");
    }
    #[test]
    fn tooltip_rows_lead_with_the_high_value_and_gap_to_it() {
        let rows = tip_rows(&profiles(), 0);
        assert_eq!(rows[0].label.as_ref(), "Berlin");
        assert_eq!(rows[0].gap, None);
        assert_eq!(rows[1].label.as_ref(), "Lisbon");
        assert_eq!(rows[1].gap.as_deref(), Some("\u{2212}22"));
        assert_eq!(minus_number(-22.), "\u{2212}22");
    }
    #[test]
    fn selects_axes_and_rejects_centre_or_empty_data() {
        let b = Bounds {
            origin: point(px(0.), px(0.)),
            size: size(px(280.), px(280.)),
        };
        assert_eq!(axis_at(b, point(px(140.), px(53.)), 4), Some(0));
        assert_eq!(axis_at(b, point(px(227.), px(140.)), 4), Some(1));
        assert_eq!(axis_at(b, point(px(140.), px(140.)), 4), None);
        assert_eq!(axis_at(b, point(px(140.), px(53.)), 0), None);
    }
    #[test]
    fn radar_axis_keeps_id_and_label() {
        let axis = RadarAxis::new("tax", "Tax climate");
        assert_eq!(axis.id.as_ref(), "tax");
        assert_eq!(axis.label.as_ref(), "Tax climate");
    }
    #[test]
    fn markers_and_tooltip_slide_between_axes() {
        use std::time::Duration;
        let mut app = gpui::TestAppContext::single();
        app.update(crate::init);
        let (view, w) = app.add_window_view(|_, _| {
            super::RadarChart::new(
                "Lisbon compared with",
                vec![
                    RadarAxis::new("talent", "Talent pool"),
                    RadarAxis::new("cost", "Affordability"),
                    RadarAxis::new("transit", "Transit"),
                    RadarAxis::new("time", "Time zone fit"),
                    RadarAxis::new("life", "Quality of life"),
                    RadarAxis::new("tax", "Tax climate"),
                ],
                vec![
                    RadarProfile::new(
                        "a",
                        "Lisbon",
                        vec![64., 78., 71., 88., 86., 66.],
                        rgb(0x55adff).into(),
                    ),
                    RadarProfile::new(
                        "b",
                        "Berlin",
                        vec![86., 58., 92., 80., 78., 46.],
                        rgb(0x9c37be).into(),
                    ),
                ],
            )
        });
        w.run_until_parked();
        let mut samples = w.update(|_, cx| {
            view.update(cx, |chart, cx| {
                chart.select(Some(0), cx);
            });
            // The second inspection starts its own glide from the settled
            // axis 0 vertex; sample from that instant onward.
            let t1 = cx.background_executor().now();
            view.update(cx, |chart, cx| {
                chart.select(Some(2), cx);
                [0, 2, 4, 8, 16, 32, 64, 128, 240]
                    .into_iter()
                    .map(|ms| {
                        let at = t1 + Duration::from_millis(ms);
                        let (mx, _) = chart.markers[0].0.sample_batch(at, false);
                        let (tx, _) = chart.tip_x.sample_batch(at, false);
                        (mx, tx)
                    })
                    .collect::<Vec<_>>()
            })
        });
        assert!(
            samples.iter().all(|(m, t)| m.is_finite() && t.is_finite()),
            "marker and tooltip stay finite across the glide: {samples:?}"
        );
        let xs: Vec<f32> = samples.iter().map(|s| s.0).collect();
        assert!((xs[0] - 275.).abs() < 0.5, "glide starts at axis 0: {xs:?}");
        assert!(
            xs.windows(2).all(|w| (w[1] - w[0]).abs() < 45.),
            "no frame teleports: {xs:?}"
        );
        assert!(
            xs[xs.len() - 1] - 373.4 < 2.,
            "glide ends at axis 2: {xs:?}"
        );
        let tip_end = samples[samples.len() - 1].1;
        assert!(
            (tip_end - 246.4).abs() < 2.,
            "tooltip ends beside the axis 2 label: {tip_end}"
        );
    }
}
