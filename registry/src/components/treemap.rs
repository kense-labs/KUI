//! A responsive, hierarchical treemap with stable-id layout transitions.
use crate::components::animated_counter::AnimatedCounter;
use crate::primitives::{
    family::{Paint, Slide, Tone, face},
    presentation::{column, hint, row, title, tr},
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
#[derive(Clone, Debug)]
pub struct TreeTile {
    pub id: SharedString,
    pub label: SharedString,
    pub value: f32,
    pub color: Hsla,
    pub metrics: Vec<f32>,
    pub shades: Vec<Hsla>,
    pub shade_values: Vec<f32>,
    pub children: Vec<TreeTile>,
}
impl TreeTile {
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        value: f32,
        color: Hsla,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            value,
            color,
            metrics: vec![],
            shades: vec![],
            shade_values: vec![],
            children: vec![],
        }
    }
    pub fn with_children(mut self, children: Vec<TreeTile>) -> Self {
        self.children = children;
        self
    }
    /// Alternative size metrics in selector order; absent metrics use `value`.
    pub fn with_metrics(mut self, metrics: Vec<f32>) -> Self {
        self.metrics = metrics;
        self
    }
    /// Alternative shading colors in selector order; absent shades use `color`.
    pub fn with_shades(mut self, shades: Vec<Hsla>) -> Self {
        self.shades = shades;
        self
    }
    /// Values to map through the matching chart color scales.
    pub fn with_shade_values(mut self, values: Vec<f32>) -> Self {
        self.shade_values = values;
        self
    }
    fn weight(&self, metric: usize) -> f64 {
        let raw = self.metrics.get(metric).copied().unwrap_or(self.value);
        let value = if raw.is_finite() { raw.max(0.) } else { 0. };
        if value > 0. {
            f64::from(value)
        } else {
            self.children.iter().map(|c| c.weight(metric)).sum()
        }
    }
    fn shade(&self, index: usize, scales: &[TreemapColorScale]) -> Hsla {
        if let (Some(value), Some(scale)) = (self.shade_values.get(index), scales.get(index))
            && value.is_finite()
        {
            return scale.color_for(*value, self.color);
        }
        self.shades.get(index).copied().unwrap_or(self.color)
    }
}
/// An explicit quantitative color scale; labels and fills share these bounds.
#[derive(Clone, Debug)]
pub struct TreemapColorScale {
    pub min: f32,
    pub max: f32,
    pub low: Hsla,
    pub high: Hsla,
    pub suffix: SharedString,
    pub branch_hues: bool,
}
impl TreemapColorScale {
    pub fn new(min: f32, max: f32, low: Hsla, high: Hsla) -> Self {
        Self {
            min,
            max,
            low,
            high,
            suffix: "".into(),
            branch_hues: false,
        }
    }
    pub fn with_suffix(mut self, suffix: impl Into<SharedString>) -> Self {
        self.suffix = suffix.into();
        self
    }
    /// Preserve each branch's hue and encode magnitude in its lightness.
    pub fn with_branch_hues(mut self, enabled: bool) -> Self {
        self.branch_hues = enabled;
        self
    }
    fn fraction(&self, value: f32) -> f32 {
        let width = self.max - self.min;
        if width.is_finite() && width > 0. {
            ((value - self.min) / width).clamp(0., 1.)
        } else {
            0.
        }
    }
    fn color_for(&self, value: f32, mut branch: Hsla) -> Hsla {
        let t = self.fraction(value);
        if self.branch_hues {
            branch.s = (branch.s * (0.55 + 0.45 * t)).clamp(0., 1.);
            branch.l = (branch.l * (0.55 + 0.45 * t)).clamp(0., 1.);
            branch
        } else {
            self.low.blend(self.high.opacity(t))
        }
    }
}
#[derive(Clone, Debug)]
pub struct TreemapInspect {
    pub path: Vec<SharedString>,
    pub id: SharedString,
    pub value: f32,
    pub share: f32,
}
#[derive(Clone, Copy, Debug, Default)]
struct Rect {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}
fn nearest_tile(centers: &[(f32, f32)], current: usize, direction: &str) -> Option<usize> {
    let &(x, y) = centers.get(current)?;
    centers
        .iter()
        .copied()
        .enumerate()
        .filter(|(j, _)| *j != current)
        .filter_map(|(j, (xx, yy))| {
            let dx = xx - x;
            let dy = yy - y;
            let forward = match direction {
                "right" => dx,
                "left" => -dx,
                "down" => dy,
                "up" => -dy,
                _ => return None,
            };
            (forward > 0.01).then_some((j, dx * dx + dy * dy))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(j, _)| j)
}
struct AnimatedTile {
    id: SharedString,
    x: Slide,
    y: Slide,
    w: Slide,
    h: Slide,
    old_shade: Hsla,
    shade: Hsla,
    shade_progress: Slide,
}
struct Zoom {
    origin: Rect,
    outgoing: Vec<(TreeTile, Rect)>,
    entering: bool,
}
pub struct Treemap {
    title: SharedString,
    root: Vec<TreeTile>,
    path: Vec<usize>,
    tiles: Vec<AnimatedTile>,
    active: Option<usize>,
    bounds: Rc<Cell<Bounds<Pixels>>>,
    last_size: (f32, f32),
    dirty: bool,
    zoom: Option<Zoom>,
    zoom_progress: Slide,
    tip_x: Slide,
    tip_y: Slide,
    visible: bool,
    metric: usize,
    shade: usize,
    metric_labels: Vec<SharedString>,
    shade_labels: Vec<SharedString>,
    color_scales: Vec<TreemapColorScale>,
    total_counter: Option<Entity<AnimatedCounter>>,
}
impl EventEmitter<TreemapInspect> for Treemap {}
impl Treemap {
    pub fn new(title: impl Into<SharedString>, root: Vec<TreeTile>) -> Self {
        Self {
            title: title.into(),
            root,
            path: vec![],
            tiles: vec![],
            active: None,
            bounds: Default::default(),
            last_size: (0., 0.),
            dirty: true,
            zoom: None,
            zoom_progress: Slide::new(1.),
            tip_x: Slide::new(0.),
            tip_y: Slide::new(0.),
            visible: true,
            metric: 0,
            shade: 0,
            metric_labels: vec![],
            shade_labels: vec![],
            color_scales: vec![],
            total_counter: None,
        }
    }
    pub fn set_tiles(&mut self, root: Vec<TreeTile>, cx: &mut Context<Self>) {
        let mut ids = Vec::new();
        let mut old = self.root.as_slice();
        for &index in &self.path {
            let Some(node) = old.get(index) else { break };
            ids.push(node.id.clone());
            old = &node.children;
        }
        let mut path = Vec::new();
        let mut next = root.as_slice();
        for id in ids {
            let Some(index) = next.iter().position(|node| node.id == id) else {
                break;
            };
            path.push(index);
            next = &next[index].children;
        }
        self.root = root;
        self.path = path;
        self.active = None;
        self.zoom = None;
        self.dirty = true;
        cx.notify();
    }
    /// Switch the size metric; matching IDs animate to their new rectangles.
    pub fn set_metric(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.metric != index {
            self.metric = index;
            self.dirty = true;
            cx.notify();
        }
    }
    /// Switch the shading measure without changing tile geometry.
    pub fn set_shade(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.shade != index {
            self.shade = index;
            self.dirty = true;
            cx.notify();
        }
    }
    pub fn with_metric_labels(mut self, labels: Vec<SharedString>) -> Self {
        self.metric_labels = labels;
        self
    }
    pub fn with_shade_labels(mut self, labels: Vec<SharedString>) -> Self {
        self.shade_labels = labels;
        self
    }
    pub fn with_color_scales(mut self, scales: Vec<TreemapColorScale>) -> Self {
        self.color_scales = scales;
        self
    }
    fn children(&self) -> &[TreeTile] {
        let mut nodes = self.root.as_slice();
        for &i in &self.path {
            let Some(node) = nodes.get(i) else { return &[] };
            nodes = &node.children;
        }
        nodes
    }
    fn breadcrumbs(&self) -> Vec<SharedString> {
        let mut nodes = self.root.as_slice();
        let mut result = vec![];
        for &i in &self.path {
            let Some(node) = nodes.get(i) else { break };
            result.push(node.label.clone());
            nodes = &node.children;
        }
        result
    }
    fn reconcile(&mut self, cx: &App, width: f32, height: f32) {
        let nodes = self.children().to_vec();
        let weights: Vec<f64> = nodes.iter().map(|node| node.weight(self.metric)).collect();
        let rects = squarify(&weights, width, height);
        let mut old: Vec<Option<AnimatedTile>> = std::mem::take(&mut self.tiles)
            .into_iter()
            .map(Some)
            .collect();
        self.tiles = nodes
            .iter()
            .enumerate()
            .map(|(i, node)| {
                let r = rects[i];
                let initial = self.zoom.as_ref().map_or(
                    Rect {
                        x: 0.5,
                        y: 0.5,
                        w: 0.,
                        h: 0.,
                    },
                    |zoom| {
                        let o = zoom.origin;
                        if zoom.entering {
                            Rect {
                                x: o.x + r.x * o.w,
                                y: o.y + r.y * o.h,
                                w: r.w * o.w,
                                h: r.h * o.h,
                            }
                        } else {
                            let ow = o.w.max(0.001);
                            let oh = o.h.max(0.001);
                            Rect {
                                x: (r.x - o.x) / ow,
                                y: (r.y - o.y) / oh,
                                w: r.w / ow,
                                h: r.h / oh,
                            }
                        }
                    },
                );
                let mut tile = old
                    .iter()
                    .position(|v| v.as_ref().is_some_and(|v| v.id == node.id))
                    .and_then(|i| old[i].take())
                    .unwrap_or_else(|| AnimatedTile {
                        id: node.id.clone(),
                        x: Slide::new(initial.x),
                        y: Slide::new(initial.y),
                        w: Slide::new(initial.w),
                        h: Slide::new(initial.h),
                        old_shade: node.shade(self.shade, &self.color_scales),
                        shade: node.shade(self.shade, &self.color_scales),
                        shade_progress: Slide::new(1.),
                    });
                let target_shade = node.shade(self.shade, &self.color_scales);
                if tile.shade != target_shade {
                    tile.old_shade = tile.old_shade.blend(
                        tile.shade
                            .opacity(tile.shade_progress.value(cx).clamp(0., 1.)),
                    );
                    tile.shade = target_shade;
                    tile.shade_progress.reset(0.);
                    tile.shade_progress.duration(1., 0.48, cx);
                }
                for (s, v) in [
                    (&mut tile.x, r.x),
                    (&mut tile.y, r.y),
                    (&mut tile.w, r.w),
                    (&mut tile.h, r.h),
                ] {
                    s.duration(v, 0.62, cx);
                }
                tile
            })
            .collect();
        self.last_size = (width, height);
        self.dirty = false;
    }
    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        if self.visible != visible {
            self.visible = visible;
            cx.notify();
        }
    }
    fn inspect(&mut self, index: usize, cx: &mut Context<Self>) {
        let nodes = self.children();
        let Some(node) = nodes.get(index) else { return };
        let total: f64 = nodes.iter().map(|node| node.weight(self.metric)).sum();
        let event = TreemapInspect {
            path: self.breadcrumbs(),
            id: node.id.clone(),
            value: node.weight(self.metric).min(f64::from(f32::MAX)) as f32,
            share: if total > 0. {
                (node.weight(self.metric) / total) as f32
            } else {
                0.
            },
        };
        self.active = Some(index);
        cx.emit(event);
        cx.notify();
    }
    fn enter(&mut self, index: usize, cx: &mut Context<Self>) {
        if self
            .children()
            .get(index)
            .is_some_and(|node| !node.children.is_empty())
        {
            let Some(origin) = self.tile_rect(index, cx) else {
                return;
            };
            self.begin_zoom(origin, true, cx);
            self.path.push(index);
            self.active = None;
            self.dirty = true;
            cx.notify();
        } else {
            self.inspect(index, cx)
        }
    }
    fn back(&mut self, cx: &mut Context<Self>) {
        if let Some(index) = self.path.pop() {
            let nodes = self.children();
            let weights: Vec<f64> = nodes.iter().map(|node| node.weight(self.metric)).collect();
            let rects = squarify(&weights, self.last_size.0, self.last_size.1);
            let origin = rects.get(index).copied().unwrap_or_default();
            self.path.push(index);
            self.begin_zoom(origin, false, cx);
            self.path.pop();
            self.active = None;
            self.dirty = true;
            cx.notify();
        }
    }
    fn tile_rect(&self, index: usize, cx: &App) -> Option<Rect> {
        self.tiles.get(index).map(|t| Rect {
            x: t.x.value(cx),
            y: t.y.value(cx),
            w: t.w.value(cx),
            h: t.h.value(cx),
        })
    }
    fn begin_zoom(&mut self, origin: Rect, entering: bool, cx: &App) {
        let nodes = self.children().to_vec();
        let outgoing = nodes
            .into_iter()
            .enumerate()
            .filter_map(|(i, node)| self.tile_rect(i, cx).map(|r| (node, r)))
            .collect();
        self.zoom = Some(Zoom {
            origin,
            outgoing,
            entering,
        });
        self.tiles.clear();
        self.zoom_progress.reset(0.);
        self.zoom_progress.duration(1., 0.62, cx);
    }
}
/// Greedy squarification: tiles preserve area while seeking low aspect ratios.
fn squarify(weights: &[f64], width: f32, height: f32) -> Vec<Rect> {
    let mut result = vec![Rect::default(); weights.len()];
    let total: f64 = weights
        .iter()
        .copied()
        .filter(|v| v.is_finite() && *v > 0.)
        .sum();
    if total <= 0. || !total.is_finite() || width <= 0. || height <= 0. {
        return result;
    }
    let mut remaining: Vec<(usize, f32)> = weights
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, v)| v.is_finite() && *v > 0.)
        .map(|(i, v)| {
            (
                i,
                ((v / total) * f64::from(width) * f64::from(height)) as f32,
            )
        })
        .collect();
    remaining.sort_by(|a, b| b.1.total_cmp(&a.1));
    let (mut x, mut y, mut w, mut h) = (0., 0., width, height);
    while !remaining.is_empty() {
        let side = w.min(h).max(0.001);
        let score = |row: &[(usize, f32)]| {
            let sum: f32 = row.iter().map(|v| v.1).sum();
            let min = row.iter().map(|v| v.1).fold(f32::INFINITY, f32::min);
            let max = row.iter().map(|v| v.1).fold(0f32, f32::max);
            (side * side * max / (sum * sum)).max(sum * sum / (side * side * min))
        };
        let mut count = 1;
        while count < remaining.len() && score(&remaining[..=count]) <= score(&remaining[..count]) {
            count += 1;
        }
        let row = &remaining[..count];
        let sum: f32 = row.iter().map(|v| v.1).sum();
        if w >= h {
            let rw = (sum / h.max(0.001)).min(w);
            let mut yy = y;
            for &(i, a) in row {
                let rh = a / rw.max(0.001);
                result[i] = Rect {
                    x: x / width,
                    y: yy / height,
                    w: rw / width,
                    h: rh / height,
                };
                yy += rh;
            }
            x += rw;
            w = (w - rw).max(0.);
        } else {
            let rh = (sum / w.max(0.001)).min(h);
            let mut xx = x;
            for &(i, a) in row {
                let rw = a / rh.max(0.001);
                result[i] = Rect {
                    x: xx / width,
                    y: y / height,
                    w: rw / width,
                    h: rh / height,
                };
                xx += rw;
            }
            y += rh;
            h = (h - rh).max(0.);
        }
        remaining.drain(..count);
    }
    result
}
fn gap_rect(r: &Rect, width: f32, height: f32) -> Rect {
    let gx = if width > 1. { 2. / width } else { 0. };
    let gy = if height > 1. { 2. / height } else { 0. };
    Rect {
        x: r.x + gx * 0.5,
        y: r.y + gy * 0.5,
        w: (r.w - gx).max(0.),
        h: (r.h - gy).max(0.),
    }
}
struct TileStyle {
    color: Hsla,
    metric: usize,
    shade: usize,
    width: f32,
    height: f32,
    selected: bool,
    p: Paint,
    scales: Vec<TreemapColorScale>,
}
fn tile_content(node: &TreeTile, style: TileStyle, _cx: &App) -> Div {
    let TileStyle {
        color,
        metric,
        shade,
        width,
        height,
        selected,
        p,
        scales,
    } = style;
    let show_label = width > 70. && height > 30.;
    let show_children = !node.children.is_empty() && width > 90. && height > 58.;
    let header = width > 64. && height > 48.;
    let child_width = (width - 6.).max(1.);
    let child_height = (height - if header { 25. } else { 6. }).max(1.);
    let weights: Vec<f64> = node.children.iter().map(|c| c.weight(metric)).collect();
    let rects = squarify(&weights, child_width, child_height);
    div()
        .size_full()
        .flex()
        .flex_col()
        .px(px(8.))
        .py(px(if node.children.is_empty() { 6. } else { 3. }))
        .overflow_hidden()
        .rounded(px(if node.children.is_empty() { 6. } else { 8. }))
        .bg(p
            .panel
            .blend(color.opacity(if selected { 0.4 } else { 0.26 })))
        .relative()
        .text_color(p.ink)
        .text_size(px(12.))
        .line_height(px(16.))
        .when(show_label, |tile| {
            tile.child(
                row()
                    .justify_between()
                    .items_baseline()
                    .gap(px(if node.children.is_empty() { 1. } else { 8. }))
                    .child(
                        div()
                            .font_weight(FontWeight::MEDIUM)
                            .child(node.label.clone()),
                    )
                    .when(width > 120., |r| {
                        r.child(
                            div()
                                .flex_none()
                                .opacity(if node.children.is_empty() { 0.72 } else { 0.6 })
                                .child(format!("{:.0}", node.weight(metric))),
                        )
                    }),
            )
        })
        .when(selected, |tile| {
            tile.child(
                div()
                    .absolute()
                    .inset_0()
                    .rounded(px(if node.children.is_empty() { 6. } else { 8. }))
                    .border(px(2.))
                    .border_color(p.ink),
            )
        })
        .when(show_children, |tile| {
            tile.child(
                div()
                    .relative()
                    .w_full()
                    .flex_1()
                    .overflow_hidden()
                    .children(
                        node.children
                            .iter()
                            .zip(rects)
                            .filter(|(_, r)| r.w > 0.001 && r.h > 0.001)
                            .map(|(child, r)| {
                                let r = gap_rect(&r, child_width, child_height);
                                let label = if r.w * child_width > 64. && r.h * child_height > 24. {
                                    child.label.clone()
                                } else {
                                    "".into()
                                };
                                div()
                                    .absolute()
                                    .left(relative(r.x))
                                    .top(relative(r.y))
                                    .w(relative(r.w))
                                    .h(relative(r.h))
                                    .p(px(1.))
                                    .child(
                                        div()
                                            .size_full()
                                            .flex()
                                            .flex_col()
                                            .p_1()
                                            .overflow_hidden()
                                            .rounded(px(6.))
                                            .bg(p
                                                .panel
                                                .blend(child.shade(shade, &scales).opacity(0.48)))
                                            .text_color(p.ink)
                                            .child(label)
                                            .when(
                                                r.w * child_width > 64. && r.h * child_height > 45.,
                                                |tile| {
                                                    tile.child(hint(
                                                        format!("{:.0}", child.weight(metric)),
                                                        p,
                                                    ))
                                                },
                                            ),
                                    )
                            }),
                    ),
            )
        })
}
fn intersects_viewport(bounds: Bounds<Pixels>, viewport: Size<Pixels>) -> bool {
    bounds.size.width > px(0.)
        && bounds.size.height > px(0.)
        && bounds.origin.x < viewport.width
        && bounds.origin.y < viewport.height
        && bounds.origin.x + bounds.size.width > px(0.)
        && bounds.origin.y + bounds.size.height > px(0.)
}
impl Render for Treemap {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let measured = self.bounds.get();
        let on_screen =
            self.visible && w.is_visible() && intersects_viewport(measured, w.viewport_size());
        let width = f32::from(measured.size.width);
        let height = f32::from(measured.size.height);
        if width > 0. && height > 0. && (self.dirty || self.last_size != (width, height)) {
            self.reconcile(cx, width, height);
        }
        let p = Paint::get(cx);
        let data = self.children().to_vec();
        let rects: Vec<(Rect, Hsla)> = self
            .tiles
            .iter_mut()
            .map(|t| {
                (
                    Rect {
                        x: if on_screen {
                            t.x.get(w, cx)
                        } else {
                            t.x.value(cx)
                        },
                        y: if on_screen {
                            t.y.get(w, cx)
                        } else {
                            t.y.value(cx)
                        },
                        w: if on_screen {
                            t.w.get(w, cx)
                        } else {
                            t.w.value(cx)
                        },
                        h: if on_screen {
                            t.h.get(w, cx)
                        } else {
                            t.h.value(cx)
                        },
                    },
                    t.old_shade.blend(
                        t.shade.opacity(
                            if on_screen {
                                t.shade_progress.get(w, cx)
                            } else {
                                t.shade_progress.value(cx)
                            }
                            .clamp(0., 1.),
                        ),
                    ),
                )
            })
            .collect();
        let bounds = self.bounds.clone();
        let labels = self.breadcrumbs();
        let active = self.active;
        let progress = if on_screen {
            self.zoom_progress.get(w, cx)
        } else {
            self.zoom_progress.value(cx)
        }
        .clamp(0., 1.);
        let outgoing = self
            .zoom
            .as_ref()
            .map(|z| {
                z.outgoing
                    .iter()
                    .map(|(node, r)| {
                        let o = z.origin;
                        let target = if z.entering {
                            let ow = o.w.max(0.001);
                            let oh = o.h.max(0.001);
                            Rect {
                                x: (r.x - o.x) / ow,
                                y: (r.y - o.y) / oh,
                                w: r.w / ow,
                                h: r.h / oh,
                            }
                        } else {
                            Rect {
                                x: o.x + r.x * o.w,
                                y: o.y + r.y * o.h,
                                w: r.w * o.w,
                                h: r.h * o.h,
                            }
                        };
                        let lerp = |a: f32, b: f32| a + (b - a) * progress;
                        (
                            node.clone(),
                            Rect {
                                x: lerp(r.x, target.x),
                                y: lerp(r.y, target.y),
                                w: lerp(r.w, target.w),
                                h: lerp(r.h, target.h),
                            },
                        )
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if progress >= 0.9999 {
            self.zoom = None;
        }
        if let Some((r, _)) = self.active.and_then(|i| rects.get(i)) {
            let ax = (r.x + r.w / 2.) * width;
            let ay = r.y * height + (r.h * height / 2.).min(28.);
            let mut left = ax + 14.;
            let mut top = ay + 14.;
            if left + 160. > width {
                left = (ax - 14. - 160.).max(0.);
            }
            if top + 72. > height {
                top = (ay - 14. - 72.).max(0.);
            }
            self.tip_x.duration(left.max(0.), 0.26, cx);
            self.tip_y.duration(top.max(0.), 0.26, cx);
        }
        let tip_x = self.tip_x.get(w, cx);
        let tip_y = self.tip_y.get(w, cx);
        let graph = div()
            .id("treemap")
            .role(Role::Figure)
            .tab_index(0)
            .relative()
            .w_full()
            .h(px(420.))
            .rounded(px(8.))
            .overflow_hidden()
            .aria_label({
                let instructions = tr(
                    cx,
                    "Use arrow keys to inspect, enter to open, escape to go back",
                    "使用方向键查看，回车进入，Esc 返回",
                );
                let selected = active
                    .or((!data.is_empty()).then_some(0))
                    .and_then(|i| data.get(i));
                if let Some(node) = selected {
                    format!(
                        "{}. {} {:.1}. {}",
                        self.title,
                        node.label,
                        node.weight(self.metric),
                        instructions
                    )
                } else {
                    format!("{}. {}", self.title, instructions)
                }
            })
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                let n = s.children().len();
                if n == 0 {
                    return;
                }
                let i = s.active.unwrap_or(0);
                match e.keystroke.key.as_str() {
                    direction @ ("right" | "left" | "up" | "down") => {
                        if s.active.is_none() {
                            s.inspect(0, cx);
                            w.prevent_default();
                            cx.stop_propagation();
                            return;
                        }
                        let size = s.bounds.get().size;
                        let centers: Vec<_> = s
                            .tiles
                            .iter()
                            .take(n)
                            .map(|tile| {
                                (
                                    (tile.x.value(cx) + tile.w.value(cx) / 2.)
                                        * f32::from(size.width),
                                    (tile.y.value(cx) + tile.h.value(cx) / 2.)
                                        * f32::from(size.height),
                                )
                            })
                            .collect();
                        let next = nearest_tile(&centers, i, direction);
                        s.inspect(next.unwrap_or(i), cx);
                    }
                    "home" => s.inspect(0, cx),
                    "end" => s.inspect(n - 1, cx),
                    "enter" | "space" => s.enter(i, cx),
                    "escape" | "backspace" => s.back(cx),
                    _ => return,
                };
                w.prevent_default();
                cx.stop_propagation();
            }))
            .child(
                canvas(
                    move |b, w, _| {
                        let previous = bounds.get();
                        let viewport = w.viewport_size();
                        if previous.size != b.size
                            || intersects_viewport(previous, viewport)
                                != intersects_viewport(b, viewport)
                        {
                            w.request_animation_frame();
                        }
                        bounds.set(b)
                    },
                    |_, _, _, _| (),
                )
                .absolute()
                .inset_0(),
            )
            .children(outgoing.into_iter().map(|(node, r)| {
                div()
                    .absolute()
                    .left(relative(r.x))
                    .top(relative(r.y))
                    .w(relative(r.w.max(0.)))
                    .h(relative(r.h.max(0.)))
                    .p(px(3.))
                    .opacity(1. - progress)
                    .child(
                        div()
                            .size_full()
                            .rounded(crate::foundation::surface_radius(cx))
                            .bg(node.shade(self.shade, &self.color_scales).opacity(0.8)),
                    )
            }))
            .children(data.iter().enumerate().filter_map(|(i, node)| {
                let (r, color) = rects.get(i)?;
                let r = gap_rect(r, width, height);
                if r.w <= 0.001 || r.h <= 0.001 {
                    return None;
                }
                Some(
                    div()
                        .id(("treemap-tile", i))
                        .absolute()
                        .left(relative(r.x))
                        .top(relative(r.y))
                        .w(relative(r.w))
                        .h(relative(r.h))
                        .p(px(3.))
                        .child(tile_content(
                            node,
                            TileStyle {
                                color: *color,
                                metric: self.metric,
                                shade: self.shade,
                                width: r.w * width,
                                height: r.h * height,
                                selected: active == Some(i),
                                p,
                                scales: self.color_scales.clone(),
                            },
                            cx,
                        ))
                        .on_hover(cx.listener(move |s, h: &bool, _, cx| {
                            if *h {
                                s.inspect(i, cx)
                            }
                        }))
                        .on_click(cx.listener(move |s, _, _, cx| s.enter(i, cx))),
                )
            }))
            .when_some(
                self.active.and_then(|i| data.get(i).cloned()),
                |stage, node| {
                    stage.child(
                        div()
                            .absolute()
                            .left(px(tip_x))
                            .top(px(tip_y))
                            .min_w(px(160.))
                            .max_w(px(240.))
                            .px(px(12.))
                            .py(px(10.))
                            .flex()
                            .flex_col()
                            .gap(px(2.))
                            .rounded(crate::foundation::surface_radius(cx))
                            .border_1()
                            .border_color(p.line)
                            .bg(p.panel)
                            .shadow_lg()
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .line_height(px(20.))
                                    .text_color(p.faint)
                                    .child(node.label.clone()),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_baseline()
                                    .justify_between()
                                    .gap(px(12.))
                                    .text_size(px(14.))
                                    .line_height(px(18.9))
                                    .child(
                                        div()
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.ink)
                                            .child(format!("{:.1}", node.weight(self.metric))),
                                    ),
                            ),
                    )
                },
            );
        let detail = self
            .active
            .and_then(|i| data.get(i))
            .map(|v| format!("{}  {:.1}", v.label, v.weight(self.metric)))
            .unwrap_or_else(|| tr(cx, "Select a tile to inspect", "选择区域查看数据").to_string());
        let back_label = if labels.is_empty() {
            tr(cx, "Overview", "总览").to_string()
        } else {
            format!(
                "{} / {}",
                tr(cx, "Overview", "总览"),
                labels
                    .iter()
                    .map(|s| s.as_ref())
                    .collect::<Vec<_>>()
                    .join(" / ")
            )
        };
        let total: f64 = data.iter().map(|node| node.weight(self.metric)).sum();
        let counter = self
            .total_counter
            .get_or_insert_with(|| {
                cx.new(|_| AnimatedCounter::new(total.min(1_000_000_000_000.)).with_size(20.))
            })
            .clone();
        counter.update(cx, |counter, cx| {
            counter.set_value(total.min(1_000_000_000_000.), cx)
        });
        let controls = row()
            .flex_wrap()
            .gap_2()
            .when(!self.metric_labels.is_empty(), |r| {
                r.child(
                    row()
                        .gap_1()
                        .p(px(3.))
                        .rounded(crate::foundation::surface_radius(cx))
                        .bg(p.stage)
                        .children(self.metric_labels.iter().enumerate().map(|(i, label)| {
                            face(
                                ("treemap-metric", i),
                                label.clone(),
                                28.,
                                crate::foundation::surface_radius(cx).into(),
                                Tone::Clear,
                                cx,
                            )
                            .px(px(10.))
                            .aria_toggled((self.metric == i).into())
                            .when(self.metric == i, |e| e.bg(p.hover))
                            .child(label.clone())
                            .on_click(cx.listener(move |s, _, _, cx| s.set_metric(i, cx)))
                            .on_key_down(cx.listener(
                                move |s, e: &KeyDownEvent, w, cx| {
                                    if matches!(e.keystroke.key.as_str(), "enter" | "space")
                                        && !e.is_held
                                    {
                                        s.set_metric(i, cx);
                                        w.prevent_default();
                                        cx.stop_propagation();
                                    }
                                },
                            ))
                        })),
                )
            })
            .when(!self.shade_labels.is_empty(), |r| {
                r.child(
                    row()
                        .gap_1()
                        .p(px(3.))
                        .rounded(crate::foundation::surface_radius(cx))
                        .bg(p.stage)
                        .children(self.shade_labels.iter().enumerate().map(|(i, label)| {
                            face(
                                ("treemap-shade", i),
                                label.clone(),
                                28.,
                                crate::foundation::surface_radius(cx).into(),
                                Tone::Clear,
                                cx,
                            )
                            .px(px(10.))
                            .aria_toggled((self.shade == i).into())
                            .when(self.shade == i, |e| e.bg(p.hover))
                            .child(label.clone())
                            .on_click(cx.listener(move |s, _, _, cx| s.set_shade(i, cx)))
                            .on_key_down(cx.listener(
                                move |s, e: &KeyDownEvent, w, cx| {
                                    if matches!(e.keystroke.key.as_str(), "enter" | "space")
                                        && !e.is_held
                                    {
                                        s.set_shade(i, cx);
                                        w.prevent_default();
                                        cx.stop_propagation();
                                    }
                                },
                            ))
                        })),
                )
            });
        column()
            .w(px(560.))
            .max_w_full()
            .gap_3()
            .p_4()
            .rounded(crate::foundation::surface_radius(cx))
            .bg(p.panel)
            .border_1()
            .border_color(p.line)
            .text_color(p.ink)
            .child(
                row()
                    .flex_wrap()
                    .gap_2()
                    .justify_between()
                    .items_center()
                    .child(title(self.title.clone(), p))
                    .child(controls),
            )
            .child(
                row()
                    .justify_between()
                    .items_center()
                    .child(
                        face(
                            "treemap-back",
                            back_label.clone(),
                            32.,
                            crate::foundation::surface_radius(cx).into(),
                            Tone::Clear,
                            cx,
                        )
                        .px(px(6.))
                        .hover(|s| s.bg(p.ink.opacity(0.05)).text_color(p.ink))
                        .child(back_label)
                        .on_click(cx.listener(|s, _, _, cx| s.back(cx)))
                        .on_key_down(cx.listener(
                            |s, e: &KeyDownEvent, w, cx| {
                                if matches!(e.keystroke.key.as_str(), "enter" | "space")
                                    && !e.is_held
                                {
                                    s.back(cx);
                                    w.prevent_default();
                                    cx.stop_propagation();
                                }
                            },
                        )),
                    )
                    .when(total <= 1_000_000_000_000., |row| row.child(counter))
                    .when(total > 1_000_000_000_000., |row| {
                        row.child(format!("{total:.2e}"))
                    }),
            )
            .child(graph)
            .when(!self.shade_labels.is_empty(), |container| {
                container.child(
                    row()
                        .items_center()
                        .gap_2()
                        .child(hint(
                            self.shade_labels[self.shade.min(self.shade_labels.len() - 1)].clone(),
                            p,
                        ))
                        .when_some(self.color_scales.get(self.shade), |r, scale| {
                            r.child(hint(format!("{}{}", scale.min, scale.suffix), p))
                                .children((0..5).map(|i| {
                                    let value = scale.min + (scale.max - scale.min) * i as f32 / 4.;
                                    div().w(px(14.)).h(px(8.)).rounded(px(3.)).bg(
                                        if scale.branch_hues {
                                            p.panel
                                                .blend(p.secondary.opacity(0.25 + i as f32 * 0.16))
                                        } else {
                                            p.panel.blend(
                                                scale.color_for(value, p.secondary).opacity(0.48),
                                            )
                                        },
                                    )
                                }))
                                .child(hint(format!("{}{}", scale.max, scale.suffix), p))
                        }),
                )
            })
            .child(hint(detail, p))
    }
}
#[cfg(test)]
mod tests {
    use super::{TreemapColorScale, nearest_tile, squarify};
    use gpui::rgb;
    #[test]
    fn tiles_fill_space_and_preserve_shares() {
        let r = squarify(&[6., 3., 1.], 400., 200.);
        for (tile, weight) in r.iter().zip([0.6, 0.3, 0.1]) {
            assert!((tile.w * tile.h - weight).abs() < 0.001);
        }
        assert!(
            r.iter()
                .all(|t| t.x >= 0. && t.y >= 0. && t.x + t.w <= 1.001 && t.y + t.h <= 1.001)
        );
    }
    #[test]
    fn very_large_values_keep_finite_balanced_tiles() {
        let large = f64::from(f32::MAX);
        let tiles = squarify(&[large, large], 400., 200.);
        assert!(
            tiles
                .iter()
                .all(|tile| tile.w.is_finite() && tile.h.is_finite())
        );
        assert!((tiles[0].w * tiles[0].h - 0.5).abs() < 0.001);
    }
    #[test]
    fn zero_values_have_empty_tiles() {
        let r = squarify(&[0., 2., 0.], 100., 100.);
        assert_eq!(r[0].w, 0.);
        assert!((r[1].w * r[1].h - 1.).abs() < 0.001);
        assert_eq!(r[2].w, 0.);
    }
    #[test]
    fn magnitude_keeps_branch_hue_and_clamps_domain() {
        let base = rgb(0x62b5d0).into();
        let scale = TreemapColorScale::new(0., 20., rgb(0x111111).into(), rgb(0xffffff).into())
            .with_branch_hues(true);
        let low = scale.color_for(-1., base);
        let high = scale.color_for(21., base);
        assert_eq!(low.h, high.h);
        assert!(low.l < high.l);
        assert_eq!(low, scale.color_for(0., base));
        assert_eq!(high, scale.color_for(20., base));
    }
    #[test]
    fn arrow_keys_select_nearest_in_the_requested_direction() {
        let centers = [(20., 20.), (100., 20.), (20., 100.), (100., 100.)];
        assert_eq!(nearest_tile(&centers, 0, "right"), Some(1));
        assert_eq!(nearest_tile(&centers, 0, "down"), Some(2));
        assert_eq!(nearest_tile(&centers, 3, "left"), Some(2));
        assert_eq!(nearest_tile(&centers, 0, "up"), None);
    }
}
