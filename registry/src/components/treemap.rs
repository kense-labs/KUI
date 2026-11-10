//! A squarified, zoomable treemap: branches drill in through one affine
//! carry, tiles morph between measures, and shading deepens with a measure.
use crate::primitives::{
    family::{Paint, Slide, Tone, disabled_motion, face, measure},
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
}
/// An explicit quantitative color scale; labels share these bounds.
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
}
#[derive(Clone, Debug)]
pub struct TreemapInspect {
    pub path: Vec<SharedString>,
    pub id: SharedString,
    pub value: f32,
    pub share: f32,
}

const GAP: f32 = 2.;
const PAD: f32 = 3.;
const HEADER: f32 = 22.;
const TIP: f32 = 14.;
const STEPS: usize = 5;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Rect {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}
impl Rect {
    fn inset(self, top: f32, side: f32) -> Rect {
        Rect {
            x: self.x + side,
            y: self.y + top,
            w: (self.w - side * 2.).max(0.),
            h: (self.h - top - side).max(0.),
        }
    }
    fn gapped(self) -> Rect {
        self.inset(GAP / 2., GAP / 2.)
    }
    fn center(self) -> (f32, f32) {
        (self.x + self.w / 2., self.y + self.h / 2.)
    }
}
/// The affine map that carries rect `from` onto rect `to`, applied to `r`.
fn carry(r: Rect, from: Rect, to: Rect) -> Rect {
    let sx = to.w / from.w.max(1.);
    let sy = to.h / from.h.max(1.);
    Rect {
        x: to.x + (r.x - from.x) * sx,
        y: to.y + (r.y - from.y) * sy,
        w: r.w * sx,
        h: r.h * sy,
    }
}
fn mix_rect(a: Rect, b: Rect, t: f32) -> Rect {
    Rect {
        x: a.x + (b.x - a.x) * t,
        y: a.y + (b.y - a.y) * t,
        w: a.w + (b.w - a.w) * t,
        h: a.h + (b.h - a.h) * t,
    }
}

/// One row of the flattened tree, in stable traversal order (root is index 0).
#[derive(Clone, Debug)]
struct FlatNode {
    id: SharedString,
    label: SharedString,
    value: f64,
    color: Option<f32>,
    depth: usize,
    parent: Option<usize>,
    children: Vec<usize>,
    /// Index among the top level branches, inherited by descendants.
    branch: Option<usize>,
    /// Labels from the top branch down to this node; the root holds its own.
    path: Vec<SharedString>,
}
fn flatten(title: &SharedString, root: &[TreeTile], metric: usize, shade: usize) -> Vec<FlatNode> {
    let mut flat = vec![FlatNode {
        id: "treemap-root".into(),
        label: title.clone(),
        value: 0.,
        color: None,
        depth: 0,
        parent: None,
        children: vec![],
        branch: None,
        path: vec![title.clone()],
    }];
    fn visit(
        node: &TreeTile,
        branch: Option<usize>,
        prefix: &[SharedString],
        parent: usize,
        metric: usize,
        shade: usize,
        flat: &mut Vec<FlatNode>,
    ) -> (f64, f64, f64) {
        let idx = flat.len();
        let mut path = prefix.to_vec();
        path.push(node.label.clone());
        flat.push(FlatNode {
            id: node.id.clone(),
            label: node.label.clone(),
            value: 0.,
            color: None,
            depth: flat[parent].depth + 1,
            parent: Some(parent),
            children: vec![],
            branch,
            path,
        });
        let mut value = 0.;
        let mut weighted = 0.;
        let mut colored = 0.;
        if node.children.is_empty() {
            value = node.weight(metric);
            if let Some(color) = node
                .shade_values
                .get(shade)
                .copied()
                .filter(|v| v.is_finite())
                && value > 0.
            {
                weighted = f64::from(color) * value;
                colored = value;
            }
        } else {
            // Root's children are the top branches; everything below keeps it.
            let child_prefix: Vec<SharedString> = if flat[idx].depth == 0 {
                vec![]
            } else {
                flat[idx].path.clone()
            };
            let mut kid_indices = vec![];
            for kid in &node.children {
                kid_indices.push(flat.len());
                let r = visit(kid, branch, &child_prefix, idx, metric, shade, flat);
                value += r.0;
                weighted += r.1;
                colored += r.2;
            }
            flat[idx].children = kid_indices;
        }
        let node = &mut flat[idx];
        node.value = value;
        node.color = if colored > 0. {
            Some((weighted / colored) as f32)
        } else {
            None
        };
        (value, weighted, colored)
    }
    let mut top = vec![];
    let mut top_indices = vec![];
    for (i, node) in root.iter().enumerate() {
        top_indices.push(flat.len());
        top.push(visit(node, Some(i), &[], 0, metric, shade, &mut flat));
    }
    let total: f64 = top.iter().map(|r| r.0).sum();
    let weighted: f64 = top.iter().map(|r| r.1).sum();
    let colored: f64 = top.iter().map(|r| r.2).sum();
    let root_node = &mut flat[0];
    root_node.value = total;
    root_node.color = if colored > 0. {
        Some((weighted / colored) as f32)
    } else {
        None
    };
    root_node.children = top_indices;
    flat
}

/// Greedy squarification into `rect` (pixels): rows of tiles along the short
/// side, each row closed once adding a tile would worsen the worst aspect
/// ratio. Zero-weight tiles keep empty rects at the origin.
fn squarify(values: &[f64], rect: Rect, out: &mut [Rect]) {
    let mut list: Vec<(usize, f64)> = values
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, v)| v.is_finite() && *v > 0.)
        .collect();
    let total: f64 = list.iter().map(|(_, v)| *v).sum();
    if total <= 0. || !total.is_finite() || rect.w <= 0. || rect.h <= 0. {
        return;
    }
    list.sort_by(|a, b| b.1.total_cmp(&a.1));
    let scale = rect.w as f64 * rect.h as f64 / total;
    let (mut x, mut y, mut w, mut h) = (rect.x, rect.y, rect.w, rect.h);
    let worst = |row: &[f64], side: f32| {
        let sum: f64 = row.iter().sum();
        let max = row.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let min = row.iter().copied().fold(f64::INFINITY, f64::min);
        let side = f64::from(side);
        ((side * side * max) / (sum * sum)).max((sum * sum) / (side * side * min))
    };
    let mut i = 0;
    while i < list.len() {
        let side = w.min(h);
        let mut row: Vec<f64> = vec![list[i].1 * scale];
        let mut j = i + 1;
        while j < list.len() {
            let mut next = row.clone();
            next.push(list[j].1 * scale);
            if worst(&next, side) > worst(&row, side) {
                break;
            }
            row = next;
            j += 1;
        }
        let sum: f64 = row.iter().sum();
        if w >= h {
            let cw = (sum / h.max(0.001) as f64) as f32;
            let mut cy = y;
            for k in i..j {
                let th = row[k - i] as f32 / cw.max(0.001);
                out[list[k].0] = Rect {
                    x,
                    y: cy,
                    w: cw,
                    h: th,
                };
                cy += th;
            }
            x += cw;
            w -= cw;
        } else {
            let rh = (sum / w.max(0.001) as f64) as f32;
            let mut cx = x;
            for k in i..j {
                let tw = row[k - i] as f32 / rh.max(0.001);
                out[list[k].0] = Rect {
                    x: cx,
                    y,
                    w: tw,
                    h: rh,
                };
                cx += tw;
            }
            y += rh;
            h -= rh;
        }
        i = j;
    }
}

/// Rects of every descendant while `center` fills `view`: its children fill
/// the view, and each branch holds its own children under a header.
fn layout_from(flat: &[FlatNode], center: usize, view: Rect) -> Vec<Option<Rect>> {
    let mut out = vec![None; flat.len()];
    fn nest(flat: &[FlatNode], id: usize, rect: Rect, top: bool, out: &mut Vec<Option<Rect>>) {
        let children = flat[id].children.clone();
        if children.is_empty() {
            return;
        }
        let room = if top {
            rect
        } else {
            rect.inset(
                if rect.h > 48. && rect.w > 64. {
                    HEADER
                } else {
                    PAD
                },
                PAD,
            )
        };
        let mut rects = vec![Rect::default(); children.len()];
        let values: Vec<f64> = children.iter().map(|&c| flat[c].value).collect();
        squarify(&values, room, &mut rects);
        for (child, r) in children.iter().zip(rects) {
            let gapped = r.gapped();
            out[*child] = Some(gapped);
            nest(flat, *child, gapped, false, out);
        }
    }
    nest(flat, center, view, true, &mut out);
    out
}

/// Arrow-key hop: nearest top tile in the direction, scoring distance along
/// the axis plus 1.5× the drift across it; tiles behind never win.
fn hop(centers: &[(f32, f32)], current: usize, dx: f32, dy: f32) -> Option<usize> {
    let (ox, oy) = *centers.get(current)?;
    let mut best = None;
    let mut best_score = f32::INFINITY;
    for (j, &(x, y)) in centers.iter().enumerate() {
        if j == current {
            continue;
        }
        let (vx, vy) = (x - ox, y - oy);
        let along = vx * dx + vy * dy;
        if along <= 1. {
            continue;
        }
        let across = (vx * dy - vy * dx).abs();
        let score = along + across * 1.5;
        if score < best_score {
            best_score = score;
            best = Some(j);
        }
    }
    best
}

/// Top level branches wear the four series hues in order; a fifth and beyond
/// stay neutral rather than reuse a hue.
fn series_hue(index: usize) -> Hsla {
    match index % 4 {
        0 => rgb(0x55adff).into(), // oklch(.73 .15 250)
        1 => rgb(0x9c37be).into(), // oklch(.54 .21 316)
        2 => rgb(0xe66e00).into(), // oklch(.665 .21 65)
        _ => rgb(0x00a861).into(), // oklch(.63 .18 160)
    }
}
fn hue_of(index: i32, ink: Hsla) -> Hsla {
    if index < 0 {
        series_hue(0)
    } else if index < 4 {
        series_hue(index as usize)
    } else {
        ink
    }
}
/// A few neutral steps from light to dark, so shade reads as more without a
/// second hue; quantised to `STEPS` steps between 14% and 58%.
fn shade_percent(colored: bool, value: Option<f32>, lo: f32, hi: f32) -> f32 {
    let Some(value) = value.filter(|_| colored) else {
        return 34.;
    };
    let width = if hi - lo == 0. { 1. } else { hi - lo };
    let t = ((value - lo) / width).clamp(0., 1.);
    14. + (t * (STEPS - 1) as f32).round() / (STEPS - 1) as f32 * 44.
}
/// The first reveal grows tiles in a quick wave from the largest.
fn reveal_delay(rect: Rect, view: Rect) -> f32 {
    let share = rect.w * rect.h / (view.w * view.h).max(1.);
    ((1. - share).clamp(0., 1.) * 0.3).min(0.3)
}
/// Grouped thousands with at most one fraction digit (`18,400`, `345.6`).
fn grouped(v: f64) -> SharedString {
    if !v.is_finite() {
        return "0".into();
    }
    let tenths = (v.abs() * 10.).round();
    let mut int = (tenths / 10.) as u64;
    let frac = (tenths % 10.) as u64;
    let mut parts = vec![];
    while int >= 1000 {
        parts.push(format!("{:03}", int % 1000));
        int /= 1000;
    }
    parts.push(format!("{int}"));
    parts.reverse();
    let mut text = parts.join(",");
    if frac > 0 {
        text.push_str(&format!(".{frac}"));
    }
    if v < 0. {
        text.insert(0, '-');
    }
    text.into()
}
/// Percent with at most one fraction digit, trailing zero dropped (`50%`).
fn pct(v: f32) -> SharedString {
    let mut text = format!("{:.1}", (v * 1000.).round() / 10.);
    if text.ends_with(".0") {
        text.truncate(text.len() - 2);
    }
    text.push('%');
    text.into()
}

struct Transition {
    t: Slide,
    from: Vec<Option<(Rect, f32)>>,
    to: Vec<Option<(Rect, f32)>>,
    delays: Vec<f32>,
    reveal: bool,
}
pub struct Treemap {
    title: SharedString,
    root: Vec<TreeTile>,
    flat: Vec<FlatNode>,
    center: usize,
    active: Option<usize>,
    pointer: Option<(f32, f32)>,
    layout: Vec<Option<Rect>>,
    drawn: Vec<Option<(Rect, f32)>>,
    anim: Option<Transition>,
    prev_focus: Option<(usize, Vec<Option<Rect>>)>,
    pending_retarget: bool,
    revealed: bool,
    dirty: bool,
    fills_target: Vec<Hsla>,
    fills_ready: bool,
    fills_prev: Option<Vec<Hsla>>,
    fill_swap: Slide,
    room_slides: Vec<Option<(Slide, Slide)>>,
    metric: usize,
    shade: usize,
    metric_labels: Vec<SharedString>,
    shade_labels: Vec<SharedString>,
    color_scales: Vec<TreemapColorScale>,
    total_shown: Option<SharedString>,
    total_exit: Option<SharedString>,
    total_swap: Slide,
    tip_x: Slide,
    tip_y: Slide,
    tip_show: Slide,
    tip_on: bool,
    focus_handle: Option<FocusHandle>,
    focus_subscriptions: Vec<Subscription>,
    bounds: Rc<Cell<Bounds<Pixels>>>,
    last_size: (f32, f32),
    visible: bool,
}
impl EventEmitter<TreemapInspect> for Treemap {}
impl Treemap {
    pub fn new(title: impl Into<SharedString>, root: Vec<TreeTile>) -> Self {
        let title = title.into();
        let flat = flatten(&title, &root, 0, 0);
        let fills_target = vec![Hsla::default(); flat.len()];
        let room_slides = vec![None; flat.len()];
        Self {
            title,
            root,
            flat,
            center: 0,
            active: None,
            pointer: None,
            layout: vec![],
            drawn: vec![],
            anim: None,
            prev_focus: None,
            pending_retarget: true,
            revealed: false,
            dirty: true,
            fills_target,
            fills_ready: false,
            fills_prev: None,
            fill_swap: Slide::new(1.),
            room_slides,
            metric: 0,
            shade: 0,
            metric_labels: vec![],
            shade_labels: vec![],
            color_scales: vec![],
            total_shown: None,
            total_exit: None,
            total_swap: Slide::new(1.),
            tip_x: Slide::new(0.),
            tip_y: Slide::new(0.),
            tip_show: Slide::new(0.),
            tip_on: false,
            focus_handle: None,
            focus_subscriptions: vec![],
            bounds: Default::default(),
            last_size: (0., 0.),
            visible: true,
        }
    }
    pub fn set_tiles(&mut self, root: Vec<TreeTile>, cx: &mut Context<Self>) {
        let center_id = self.flat.get(self.center).map(|n| n.id.clone());
        let active_id = self
            .active
            .and_then(|i| self.flat.get(i))
            .map(|n| n.id.clone());
        self.root = root;
        self.rebuild(cx);
        self.center = center_id
            .and_then(|id| self.flat.iter().position(|n| n.id == id))
            .filter(|&i| !self.flat[i].children.is_empty())
            .unwrap_or(0);
        self.active = active_id.and_then(|id| self.flat.iter().position(|n| n.id == id));
        self.prev_focus = None;
        self.pending_retarget = true;
        cx.notify();
    }
    /// Switch the size metric; matching tiles morph to their new rectangles.
    pub fn set_metric(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.metric != index {
            self.metric = index;
            let center_id = self.flat.get(self.center).map(|n| n.id.clone());
            let active_id = self
                .active
                .and_then(|i| self.flat.get(i))
                .map(|n| n.id.clone());
            self.rebuild(cx);
            self.center = center_id
                .and_then(|id| self.flat.iter().position(|n| n.id == id))
                .filter(|&i| !self.flat[i].children.is_empty())
                .unwrap_or(0);
            self.active = active_id.and_then(|id| self.flat.iter().position(|n| n.id == id));
            self.prev_focus = None;
            self.pending_retarget = true;
            cx.notify();
        }
    }
    /// Switch the shading measure without changing tile geometry.
    pub fn set_shade(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.shade != index {
            self.shade = index;
            let blend = self.fill_swap.value(cx).clamp(0., 1.);
            let shown: Vec<Hsla> = self
                .fills_target
                .iter()
                .enumerate()
                .map(|(i, target)| {
                    self.fills_prev
                        .as_ref()
                        .and_then(|prev| prev.get(i))
                        .map_or(*target, |old| old.blend(target.opacity(blend)))
                })
                .collect();
            self.fills_prev = Some(shown);
            self.flat = flatten(&self.title, &self.root, self.metric, self.shade);
            let p = Paint::get(cx);
            self.fills_target = self.compute_fills(&p);
            self.fills_ready = true;
            self.fill_swap.reset(0.);
            self.fill_swap.duration(1., 0.48, cx);
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
    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        if self.visible != visible {
            self.visible = visible;
            cx.notify();
        }
    }
    fn rebuild(&mut self, cx: &App) {
        // Keep what is on screen, keyed by tile id, so a data or metric switch
        // morphs from the current rectangles instead of replaying the reveal.
        let old_ids: Vec<SharedString> = self.flat.iter().map(|n| n.id.clone()).collect();
        let old_drawn = std::mem::take(&mut self.drawn);
        let old_rooms = std::mem::take(&mut self.room_slides);
        let swap_t = self.fill_swap.value(cx).clamp(0., 1.);
        let shown: Vec<Hsla> = self
            .fills_target
            .iter()
            .enumerate()
            .map(|(i, target)| {
                self.fills_prev
                    .as_ref()
                    .and_then(|prev| prev.get(i))
                    .map_or(*target, |old| old.blend(target.opacity(swap_t)))
            })
            .collect();
        self.flat = flatten(&self.title, &self.root, self.metric, self.shade);
        let p = Paint::get(cx);
        self.fills_target = self.compute_fills(&p);
        self.fills_prev = Some(
            self.flat
                .iter()
                .map(|n| {
                    old_ids
                        .iter()
                        .position(|id| *id == n.id)
                        .and_then(|old| shown.get(old).copied())
                        .unwrap_or_default()
                })
                .collect(),
        );
        self.fill_swap.reset(0.);
        self.fill_swap.duration(1., 0.48, cx);
        self.fills_ready = true;
        self.layout = Vec::new();
        self.drawn = self
            .flat
            .iter()
            .map(|n| {
                old_ids
                    .iter()
                    .position(|id| *id == n.id)
                    .and_then(|old| old_drawn.get(old).copied().flatten())
            })
            .collect();
        self.room_slides = self
            .flat
            .iter()
            .map(|n| {
                old_ids
                    .iter()
                    .position(|id| *id == n.id)
                    .and_then(|old| old_rooms.get(old).copied().flatten())
            })
            .collect();
        self.anim = None;
    }
    fn compute_fills(&self, p: &Paint) -> Vec<Hsla> {
        let (lo, hi) = self.color_bounds();
        let colored = self.colored();
        self.flat
            .iter()
            .map(|node| {
                let hue = hue_of(node.branch.map(|b| b as i32).unwrap_or(-1), p.ink);
                p.panel
                    .blend(hue.opacity(shade_percent(colored, node.color, lo, hi) / 100.))
            })
            .collect()
    }
    fn color_bounds(&self) -> (f32, f32) {
        if let Some(scale) = self.color_scales.get(self.shade) {
            return (scale.min, scale.max);
        }
        let leaves: Vec<f32> = self
            .flat
            .iter()
            .filter(|n| n.children.is_empty())
            .filter_map(|n| n.color)
            .collect();
        if leaves.is_empty() {
            (0., 1.)
        } else {
            (
                leaves.iter().copied().fold(f32::INFINITY, f32::min),
                leaves.iter().copied().fold(f32::NEG_INFINITY, f32::max),
            )
        }
    }
    fn colored(&self) -> bool {
        !self.shade_labels.is_empty()
            && self
                .flat
                .iter()
                .any(|n| n.children.is_empty() && n.color.is_some())
    }
    fn is_under(&self, id: usize, ancestor: usize) -> bool {
        let mut at = Some(id);
        while let Some(i) = at {
            if i == ancestor {
                return true;
            }
            at = self.flat.get(i).and_then(|n| n.parent);
        }
        false
    }
    /// The top level tile that contains `id` (itself when already on top).
    fn top_of(&self, id: usize) -> Option<usize> {
        let mut at = id;
        loop {
            let node = self.flat.get(at)?;
            match node.parent {
                None => return None,
                Some(parent) if parent == self.center => return Some(at),
                Some(parent) => at = parent,
            }
        }
    }
    fn top_tiles(&self) -> Vec<usize> {
        self.flat[self.center]
            .children
            .iter()
            .copied()
            .filter(|&c| self.flat[c].value > 0.)
            .collect()
    }
    fn inspect(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(node) = self.flat.get(index) else {
            return;
        };
        let parent = node.parent.unwrap_or(0);
        let parent_value = self.flat[parent].value;
        let event = TreemapInspect {
            path: node.path.clone(),
            id: node.id.clone(),
            value: node.value.min(f64::from(f32::MAX)) as f32,
            share: if parent_value > 0. {
                (node.value / parent_value) as f32
            } else {
                0.
            },
        };
        self.active = Some(index);
        cx.emit(event);
        cx.notify();
    }
    fn zoom_to(&mut self, id: usize, active: Option<usize>, cx: &mut Context<Self>) {
        let id = if self.flat.get(id).is_some_and(|n| !n.children.is_empty()) {
            id
        } else {
            0
        };
        if id != self.center {
            self.prev_focus = Some((self.center, self.layout.clone()));
            // The layout is keyed to the centered node, so re-derive it.
            self.dirty = true;
        }
        self.center = id;
        self.active = active;
        self.pointer = None;
        self.pending_retarget = true;
        cx.notify();
    }
    fn start_transition(&mut self, cx: &App) {
        let view = Rect {
            x: 0.,
            y: 0.,
            w: self.last_size.0,
            h: self.last_size.1,
        };
        let focus_changed = self
            .prev_focus
            .as_ref()
            .is_some_and(|(prev, _)| *prev != self.center);
        // The pivot: the tile that grows to fill the view, or the view that
        // shrinks back into its tile.
        let carry_maps = if focus_changed {
            self.prev_focus
                .as_ref()
                .and_then(|&(prev, ref prev_layout)| {
                    if self.is_under(self.center, prev) {
                        prev_layout
                            .get(self.center)
                            .copied()
                            .flatten()
                            .filter(|r| r.w > 0.)
                            .map(|in_old| ((view, in_old), (in_old, view)))
                    } else if self.is_under(prev, self.center) {
                        self.layout
                            .get(prev)
                            .copied()
                            .flatten()
                            .filter(|r| r.w > 0.)
                            .map(|in_new| ((in_new, view), (view, in_new)))
                    } else {
                        None
                    }
                })
        } else {
            None
        };
        let before = std::mem::take(&mut self.drawn);
        let first = before.iter().all(|v| v.is_none_or(|(_, o)| o < 0.5));
        let len = self.flat.len();
        let mut from = vec![None; len];
        let mut to = vec![None; len];
        let mut delays = vec![0.; len];
        for i in 0..len {
            match self.layout.get(i).copied().flatten() {
                Some(goal) => {
                    to[i] = Some((goal, 1.));
                    delays[i] = reveal_delay(goal, view);
                    from[i] = match before.get(i).copied().flatten() {
                        Some(prev) if prev.1 > 0.5 => Some(prev),
                        _ if first => Some((
                            Rect {
                                x: goal.x + goal.w / 2.,
                                y: goal.y + goal.h / 2.,
                                w: 0.,
                                h: 0.,
                            },
                            0.,
                        )),
                        _ => Some((
                            carry_maps
                                .as_ref()
                                .map(|(to_old, _)| carry(goal, to_old.0, to_old.1))
                                .unwrap_or(goal),
                            0.,
                        )),
                    };
                }
                None => {
                    if let Some(prev) = before.get(i).copied().flatten() {
                        from[i] = Some(prev);
                        let target = carry_maps
                            .as_ref()
                            .map(|(_, to_new)| carry(prev.0, to_new.0, to_new.1))
                            .unwrap_or(prev.0);
                        to[i] = Some((target, 0.));
                    }
                }
            }
        }
        let len = self.flat.len();
        self.drawn = vec![None; len];
        if disabled_motion(cx) {
            self.drawn = to;
            self.prev_focus = None;
            self.anim = None;
            return;
        }
        let mut t = Slide::new(0.);
        t.duration(1., if first { 0.9 } else { 0.62 }, cx);
        self.anim = Some(Transition {
            t,
            from,
            to,
            delays,
            reveal: first,
        });
        self.prev_focus = None;
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
impl Render for Treemap {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focus = self
            .focus_handle
            .get_or_insert_with(|| cx.focus_handle())
            .clone();
        if self.focus_subscriptions.is_empty() {
            // The reference clears the inspected tile when the chart loses focus.
            self.focus_subscriptions
                .push(cx.on_focus_out(&focus, w, |s, _, _, cx| {
                    if s.active.take().is_some() {
                        cx.notify();
                    }
                }));
        }
        let measured = self.bounds.get();
        let on_screen =
            self.visible && w.is_visible() && intersects_viewport(measured, w.viewport_size());
        let width = f32::from(measured.size.width);
        let height = f32::from(measured.size.height);
        let size_changed = width > 0. && height > 0. && self.last_size != (width, height);
        if size_changed {
            self.last_size = (width, height);
            self.pending_retarget = true;
        }
        if self.dirty || size_changed || self.layout.len() != self.flat.len() {
            let view = Rect {
                x: 0.,
                y: 0.,
                w: self.last_size.0,
                h: self.last_size.1,
            };
            self.layout = layout_from(&self.flat, self.center, view);
            self.dirty = false;
            self.pending_retarget = true;
        }
        if !self.revealed && on_screen && self.last_size.0 > 0. {
            self.revealed = true;
        }
        if self.pending_retarget && self.revealed {
            self.pending_retarget = false;
            self.start_transition(cx);
        }
        // Advance the running transition, writing display values back so a
        // retarget mid-flight continues from what is on screen.
        if let Some(anim) = &mut self.anim {
            let t = if on_screen {
                anim.t.get(w, cx)
            } else {
                anim.t.value(cx)
            }
            .clamp(0., 1.);
            let from = anim.from.clone();
            let to = anim.to.clone();
            let delays = anim.delays.clone();
            let reveal = anim.reveal;
            let mut display = vec![None; self.flat.len()];
            for i in 0..display.len() {
                if let (Some(a), Some(b)) = (from[i], to[i]) {
                    let local = if reveal {
                        ((t - delays[i]) / 0.7).clamp(0., 1.)
                    } else {
                        t
                    };
                    display[i] = Some((mix_rect(a.0, b.0, local), a.1 + (b.1 - a.1) * local));
                }
            }
            self.drawn = display;
            if t >= 0.9999 {
                self.drawn = to;
                self.anim = None;
            }
        }
        let p = Paint::get(cx);
        if !self.fills_ready || self.fills_target.len() != self.flat.len() {
            self.fills_target = self.compute_fills(&p);
            self.fills_prev = None;
            self.fill_swap.reset(1.);
            self.fills_ready = true;
        }
        if self.room_slides.len() != self.flat.len() {
            self.room_slides = vec![None; self.flat.len()];
        }
        let flat = &self.flat;
        let center_depth = flat[self.center].depth;
        let empty = flat[0].value <= 0.;
        let fill_t = if on_screen {
            self.fill_swap.get(w, cx)
        } else {
            self.fill_swap.value(cx)
        }
        .clamp(0., 1.);
        let fills: Vec<Hsla> = self
            .fills_target
            .iter()
            .enumerate()
            .map(|(i, target)| {
                self.fills_prev
                    .as_ref()
                    .and_then(|prev| prev.get(i))
                    .map_or(*target, |old| old.blend(target.opacity(fill_t)))
            })
            .collect();
        // Visible tiles, deeper levels painted above their parents.
        let mut visible: Vec<usize> = (0..flat.len())
            .filter(|&i| {
                let level = flat[i].depth as i32 - center_depth as i32;
                (1..=2).contains(&level)
                    && self
                        .drawn
                        .get(i)
                        .copied()
                        .flatten()
                        .is_some_and(|(r, o)| r.w >= 0.5 && r.h >= 0.5 && o > 0.01)
            })
            .collect();
        visible.sort_by_key(|&i| flat[i].depth);
        // Label and value visibility fade in step with the tile's room.
        for &i in &visible {
            let (r, _) = self.drawn[i].unwrap();
            let room = if r.w > 64. && r.h > 38. {
                if r.h > 54. { 2 } else { 1 }
            } else {
                0
            };
            let branch = !flat[i].children.is_empty() && flat[i].depth == center_depth + 1;
            let (label_bit, value_bit) = (room >= 1, room == 2 || branch && room >= 1);
            let entry = self.room_slides[i].get_or_insert_with(|| {
                (
                    Slide::new(label_bit as u8 as f32),
                    Slide::new(value_bit as u8 as f32),
                )
            });
            entry.0.duration(label_bit as u8 as f32, 0.16, cx);
            entry.1.duration(value_bit as u8 as f32, 0.16, cx);
        }
        let focus_top = self.active.and_then(|a| self.top_of(a));
        let trail: Vec<usize> = {
            let mut path = vec![self.center];
            while let Some(parent) = flat[path[0]].parent {
                path.insert(0, parent);
            }
            path
        };
        let total_text = grouped(flat[self.center].value);
        if self.total_shown.as_ref() != Some(&total_text) {
            self.total_exit = self.total_shown.clone();
            self.total_shown = Some(total_text.clone());
            if self.total_exit.is_some() {
                self.total_swap.reset(0.);
                self.total_swap.duration(1., 0.24, cx);
            }
        }
        let total_t = if on_screen {
            self.total_swap.get(w, cx)
        } else {
            self.total_swap.value(cx)
        }
        .clamp(0., 1.);
        // Tooltip: trails the pointer, or sits by the tile in focus.
        if let Some(active) = self.active.filter(|&a| {
            self.drawn
                .get(a)
                .copied()
                .flatten()
                .is_some_and(|(_, o)| o > 0.01)
        }) {
            let (ar, _) = self.drawn[active].unwrap();
            let ax = self.pointer.map_or(ar.x + ar.w / 2., |pp| pp.0);
            let ay = self.pointer.map_or(ar.y + (ar.h / 2.).min(28.), |pp| pp.1);
            let parent = flat[active].parent.unwrap_or(0);
            let title_text: SharedString = if flat[active].path.len() > 1 {
                flat[active].path[..flat[active].path.len() - 1]
                    .iter()
                    .map(|s| s.as_ref())
                    .collect::<Vec<_>>()
                    .join(" / ")
                    .into()
            } else {
                flat[0].label.clone()
            };
            let amount = grouped(flat[active].value);
            let note: SharedString = format!(
                "{} {} {}",
                pct((flat[active].value / flat[parent].value.max(1e-9)) as f32),
                tr(cx, "of", "占"),
                flat[parent].label
            )
            .into();
            let color_note = self
                .color_scales
                .get(self.shade)
                .filter(|_| self.colored())
                .and_then(|scale| {
                    flat[active].color.map(|v| {
                        format!(
                            "{} {}{}",
                            self.shade_labels
                                .get(self.shade)
                                .cloned()
                                .unwrap_or_default(),
                            grouped(f64::from(v)),
                            scale.suffix
                        )
                        .into()
                    })
                });
            let name = flat[active].label.clone();
            let tw = [
                measure(&title_text, 12., w),
                measure(&name, 14., w) + measure(&amount, 14., w) + 12.,
                measure(&note, 12., w),
                color_note.as_ref().map_or(0., |t| measure(t, 12., w)),
            ]
            .into_iter()
            .fold(0., f32::max)
            .clamp(160., 240.);
            let rows = 3. + color_note.is_some() as u8 as f32;
            let th = 10. * 2. + 12. * 1.4 * rows + 14. * 1.35 + (rows - 1.) * 2.;
            let mut left = ax + TIP;
            if left + tw > width {
                left = ax - TIP - tw;
            }
            let mut top = ay + TIP;
            if top + th > height {
                top = ay - TIP - th;
            }
            left = left.clamp(0., (width - tw).max(0.));
            top = top.clamp(0., (height - th).max(0.));
            if self.tip_on {
                self.tip_x.duration(left, 0.26, cx);
                self.tip_y.duration(top, 0.26, cx);
            } else {
                self.tip_x.set(left, cx);
                self.tip_y.set(top, cx);
                self.tip_on = true;
            }
            self.tip_show.duration(1., 0.16, cx);
        } else {
            self.tip_on = false;
            self.tip_show.duration(0., 0.16, cx);
        }
        let tip_x = self.tip_x.get(w, cx);
        let tip_y = self.tip_y.get(w, cx);
        let tip_t = self.tip_show.get(w, cx).clamp(0., 1.);
        let bounds = self.bounds.clone();
        let active = self.active;
        let mut stage_label = format!(
            "{}. {}, {}",
            self.title,
            flat[self.center].label,
            tr(
                cx,
                "Arrow keys move between tiles, Enter zooms in, Escape zooms out",
                "使用方向键在区块间移动，回车进入，Esc 退出",
            )
        );
        if let Some(a) = active
            && let Some(node) = flat.get(a)
        {
            let parent = node.parent.unwrap_or(0);
            let share = node.value / flat[parent].value.max(1e-9);
            let mut text = format!(
                "{}. {}, {} {} {}",
                node.path
                    .iter()
                    .map(|s| s.as_ref())
                    .collect::<Vec<_>>()
                    .join(", "),
                grouped(node.value),
                pct(share as f32),
                tr(cx, "of", "占"),
                flat[parent].label
            );
            if let (Some(color), Some(scale)) = (node.color, self.color_scales.get(self.shade))
                && self.colored()
            {
                text.push_str(&format!(
                    ", {} {}{}",
                    self.shade_labels
                        .get(self.shade)
                        .cloned()
                        .unwrap_or_default(),
                    grouped(f64::from(color)),
                    scale.suffix
                ));
            }
            stage_label = format!("{stage_label}. {text}");
        }
        let graph = div()
            .id("treemap-stage")
            .role(Role::Group)
            .when(!empty, |s| s.tab_index(0))
            .track_focus(&focus)
            .relative()
            .w_full()
            .h(px(420.))
            .rounded(px(12.))
            .overflow_hidden()
            .aria_label(stage_label)
            .on_key_down(cx.listener(move |s, e: &KeyDownEvent, w, cx| {
                if s.flat[s.center].children.is_empty() {
                    return;
                }
                let top_tiles = s.top_tiles();
                if top_tiles.is_empty() {
                    return;
                }
                let dirs = [
                    ("right", 1., 0.),
                    ("left", -1., 0.),
                    ("down", 0., 1.),
                    ("up", 0., -1.),
                ];
                let key = e.keystroke.key.as_str();
                let mut handled = true;
                if let Some((_, dx, dy)) = dirs.iter().find(|(k, _, _)| *k == key) {
                    s.pointer = None;
                    let current = s.active.and_then(|a| s.top_of(a));
                    let Some(current) = current.filter(|&c| top_tiles.contains(&c)) else {
                        s.inspect(top_tiles[0], cx);
                        w.prevent_default();
                        cx.stop_propagation();
                        return;
                    };
                    let centers: Vec<(f32, f32)> = top_tiles
                        .iter()
                        .map(|&t| {
                            s.layout
                                .get(t)
                                .copied()
                                .flatten()
                                .map_or((0., 0.), |r| r.center())
                        })
                        .collect();
                    let current_pos = top_tiles.iter().position(|&t| t == current);
                    let next = current_pos
                        .and_then(|pos| hop(&centers, pos, *dx, *dy))
                        .map(|pos| top_tiles[pos]);
                    if let Some(next) = next {
                        s.inspect(next, cx);
                    }
                } else if key == "home" {
                    s.pointer = None;
                    s.inspect(top_tiles[0], cx);
                } else if key == "end" {
                    s.pointer = None;
                    s.inspect(*top_tiles.last().unwrap(), cx);
                } else if key == "enter" || key == "space" {
                    if let Some(current) = s.active.and_then(|a| s.top_of(a))
                        && !s.flat[current].children.is_empty()
                    {
                        let first = s.flat[current].children[0];
                        s.zoom_to(current, Some(first), cx);
                    }
                } else if key == "escape" || key == "backspace" {
                    if let Some(parent) = s.flat[s.center].parent {
                        let from = s.center;
                        s.zoom_to(parent, Some(from), cx);
                    } else if s.active.is_some() {
                        s.active = None;
                        cx.notify();
                    }
                } else {
                    handled = false;
                }
                if handled {
                    w.prevent_default();
                    cx.stop_propagation();
                }
            }))
            .on_mouse_move(cx.listener(|s, e: &MouseMoveEvent, _, cx| {
                let b = s.bounds.get();
                let local = (
                    f32::from(e.position.x - b.origin.x),
                    f32::from(e.position.y - b.origin.y),
                );
                if s.pointer != Some(local) {
                    s.pointer = Some(local);
                    cx.notify();
                }
            }))
            .on_hover(cx.listener(|s, h: &bool, w, cx| {
                // Keyboard focus suppresses gpui hover entirely, so a keypress
                // would otherwise clear the inspected tile that the reference
                // keeps until the pointer really leaves the chart.
                if !*h && !w.last_input_was_keyboard() {
                    let had_active = s.active.take().is_some();
                    let had_pointer = s.pointer.take().is_some();
                    if had_active || had_pointer {
                        cx.notify();
                    }
                }
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
            .children(visible.iter().map(|&i| {
                let (r, o) = self.drawn[i].unwrap();
                let node = &flat[i];
                let level = node.depth - center_depth;
                let branch = !node.children.is_empty() && level == 1;
                let hue = hue_of(node.branch.map(|b| b as i32).unwrap_or(-1), p.ink);
                let fill = fills[i];
                let interactive = o > 0.6;
                let dim = focus_top.is_some() && self.top_of(i) != focus_top;
                let lineage = focus_top == Some(i) && active != Some(i);
                let (label_t, value_t) = match self.room_slides.get(i).and_then(|o| o.as_ref()) {
                    Some(pair) => (
                        if on_screen {
                            pair.0.get(w, cx)
                        } else {
                            pair.0.value(cx)
                        },
                        if on_screen {
                            pair.1.get(w, cx)
                        } else {
                            pair.1.value(cx)
                        },
                    ),
                    None => (0., 0.),
                };
                let tile_opacity = o * if dim { 0.5 } else { 1. };
                let radius = if branch { 8. } else { 6. };
                let value_text = grouped(node.value);
                let label = node.label.clone();
                let mut tile = div()
                    .id(("treemap-tile", i))
                    .absolute()
                    .left(px(r.x))
                    .top(px(r.y))
                    .w(px(r.w))
                    .h(px(r.h))
                    .overflow_hidden()
                    .rounded(px(radius))
                    .opacity(tile_opacity)
                    .text_color(p.ink);
                tile = if branch {
                    tile.bg(p.panel.blend(hue.opacity(0.09)))
                        .flex()
                        .flex_row()
                        .items_baseline()
                        .justify_between()
                        .gap(px(8.))
                        .px(px(8.))
                        .py(px(3.))
                        .child(
                            div()
                                .id(("treemap-tile-label", i))
                                .min_w_0()
                                .flex_1()
                                .text_size(px(12.))
                                .line_height(px(16.))
                                .font_weight(FontWeight::MEDIUM)
                                .truncate()
                                .opacity(label_t.clamp(0., 1.))
                                .child(label.clone()),
                        )
                        .child(
                            div()
                                .id(("treemap-tile-value", i))
                                .flex_none()
                                .text_size(px(12.))
                                .line_height(px(16.))
                                .opacity(0.6 * value_t.clamp(0., 1.))
                                .child(value_text.clone()),
                        )
                } else {
                    let (pad_x, pad_y) = if level == 2 { (7., 5.) } else { (8., 6.) };
                    tile.bg(fill)
                        .flex()
                        .flex_col()
                        .gap(px(1.))
                        .px(px(pad_x))
                        .py(px(pad_y))
                        .child(
                            div()
                                .id(("treemap-tile-label", i))
                                .min_w_0()
                                .text_size(px(12.))
                                .line_height(px(16.))
                                .font_weight(FontWeight::MEDIUM)
                                .truncate()
                                .opacity(label_t.clamp(0., 1.))
                                .child(label.clone()),
                        )
                        .child(
                            div()
                                .id(("treemap-tile-value", i))
                                .min_w_0()
                                .text_size(px(12.))
                                .line_height(px(16.))
                                .opacity(0.72 * value_t.clamp(0., 1.))
                                .truncate()
                                .child(value_text.clone()),
                        )
                };
                if active == Some(i) {
                    tile = tile.child(
                        div()
                            .absolute()
                            .inset_0()
                            .rounded(px(radius))
                            .border(px(2.))
                            .border_color(p.ink),
                    );
                }
                if lineage {
                    tile = tile.child(
                        div()
                            .absolute()
                            .inset_0()
                            .rounded(px(radius))
                            .border_1()
                            .border_color(hue.opacity(0.7)),
                    );
                }
                if interactive {
                    tile = tile
                        .on_hover(cx.listener(move |s, h: &bool, _, cx| {
                            if *h {
                                s.inspect(i, cx);
                            }
                        }))
                        .on_click(cx.listener(move |s, _, _, cx| {
                            let top = s.top_of(i);
                            if let Some(top) = top.filter(|&t| !s.flat[t].children.is_empty()) {
                                s.zoom_to(top, None, cx);
                            } else if s.active != Some(i) {
                                s.inspect(i, cx);
                            }
                        }));
                }
                tile
            }))
            .when(!empty, |mut stage| {
                // GPUI clips overflow to a rectangle, so the reference's rounded
                // stage mask is rebuilt from four panel-colored corner patches.
                // Each patch is a 24x24 rounded square whose r12 disc center sits
                // on the stage corner: it covers exactly the pixels outside the
                // r12 corner curve, in the card's own background color.
                let w = self.last_size.0;
                let h = 420f32;
                if w > 0. {
                    for (x, y) in [
                        (-12f32, -12f32),
                        (w - 12f32, -12f32),
                        (-12f32, h - 12f32),
                        (w - 12f32, h - 12f32),
                    ] {
                        stage = stage.child(
                            div()
                                .absolute()
                                .left(px(x))
                                .top(px(y))
                                .size(px(24.))
                                .rounded(px(12.))
                                .bg(p.panel),
                        );
                    }
                }
                stage
            })
            .when(empty, |stage| {
                stage.child(
                    div()
                        .absolute()
                        .inset_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(14.))
                        .text_color(p.faint)
                        .child(tr(cx, "No data yet", "暂无数据")),
                )
            })
            .when(tip_t > 0.01, |stage| {
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
                        .rounded(px(14.))
                        .border_1()
                        .border_color(p.line)
                        .bg(p.tray)
                        .shadow_lg()
                        .opacity(tip_t)
                        .when_some(active.and_then(|a| flat.get(a)), |tip, node| {
                            let parent = node.parent.unwrap_or(0);
                            let title: SharedString = if node.path.len() > 1 {
                                node.path[..node.path.len() - 1]
                                    .iter()
                                    .map(|s| s.as_ref())
                                    .collect::<Vec<_>>()
                                    .join(" / ")
                                    .into()
                            } else {
                                flat[0].label.clone()
                            };
                            let mut tip = tip
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .line_height(px(16.8))
                                        .text_color(p.faint)
                                        .truncate()
                                        .child(title),
                                )
                                .child(
                                    row()
                                        .items_baseline()
                                        .justify_between()
                                        .gap(px(12.))
                                        .text_size(px(14.))
                                        .line_height(px(18.9))
                                        .child(
                                            div()
                                                .min_w_0()
                                                .text_color(p.ink)
                                                .truncate()
                                                .child(node.label.clone()),
                                        )
                                        .child(
                                            div()
                                                .flex_none()
                                                .text_color(p.ink)
                                                .font_weight(FontWeight::MEDIUM)
                                                .child(grouped(node.value)),
                                        ),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .line_height(px(16.8))
                                        .text_color(p.faint)
                                        .child(format!(
                                            "{} {} {}",
                                            pct((node.value / flat[parent].value.max(1e-9)) as f32),
                                            tr(cx, "of", "占"),
                                            flat[parent].label
                                        )),
                                );
                            if let (Some(color), Some(scale)) =
                                (node.color, self.color_scales.get(self.shade))
                                && self.colored()
                            {
                                tip = tip.child(
                                    div()
                                        .text_size(px(12.))
                                        .line_height(px(16.8))
                                        .text_color(p.faint)
                                        .child(format!(
                                            "{} {}{}",
                                            self.shade_labels
                                                .get(self.shade)
                                                .cloned()
                                                .unwrap_or_default(),
                                            grouped(f64::from(color)),
                                            scale.suffix
                                        )),
                                );
                            }
                            tip
                        }),
                )
            });
        // Breadcrumb on the left, the total of what fills the view on the right.
        let bar = row()
            .flex_wrap()
            .items_center()
            .justify_between()
            .gap(px(12.))
            .child(
                div()
                    .min_w_0()
                    .ml(px(-4.))
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .children(trail.iter().enumerate().flat_map(|(idx, &node)| {
                        let last = idx + 1 == trail.len();
                        let label = flat[node].label.clone();
                        let mut crumb = if last {
                            div()
                                .id(("treemap-crumb-current", idx))
                                .px(px(4.))
                                .text_size(px(14.))
                                .line_height(px(20.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.ink)
                                .truncate()
                        } else {
                            div()
                                .id(("treemap-crumb", idx))
                                .flex()
                                .items_center()
                                .min_h(px(32.))
                                .px(px(6.))
                                .mx(px(-2.))
                                .rounded(px(10.))
                                .text_size(px(14.))
                                .line_height(px(20.))
                                .text_color(p.secondary)
                                .hover(|s| s.bg(p.ink.opacity(0.05)).text_color(p.ink))
                                .truncate()
                                .on_click(cx.listener(move |s, _, _, cx| s.zoom_to(node, None, cx)))
                                .on_key_down(cx.listener(move |s, e: &KeyDownEvent, w, cx| {
                                    if matches!(e.keystroke.key.as_str(), "enter" | "space")
                                        && !e.is_held
                                    {
                                        s.zoom_to(node, None, cx);
                                        w.prevent_default();
                                        cx.stop_propagation();
                                    }
                                }))
                        };
                        if idx > 0 {
                            crumb = crumb.child(
                                div()
                                    .px(px(4.))
                                    .text_size(px(14.))
                                    .text_color(p.panel.blend(p.ink.opacity(0.32)))
                                    .child("/"),
                            );
                        }
                        [crumb.child(label)]
                    })),
            )
            .child(
                div()
                    .relative()
                    .flex_none()
                    .h(px(20.))
                    .w(px(90.))
                    .overflow_hidden()
                    .when_some(self.total_exit.clone(), |c, old| {
                        c.child(
                            div()
                                .absolute()
                                .right_0()
                                .top(px(-6. * total_t))
                                .text_size(px(14.))
                                .line_height(px(20.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.ink)
                                .opacity(1. - total_t)
                                .child(old),
                        )
                    })
                    .when_some(self.total_shown.clone(), |c, text| {
                        c.child(
                            div()
                                .absolute()
                                .right_0()
                                .top(px(6. * (1. - total_t)))
                                .text_size(px(14.))
                                .line_height(px(20.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.ink)
                                .opacity(total_t)
                                .child(text),
                        )
                    }),
            );
        let colored = self.colored();
        let (lo, hi) = self.color_bounds();
        let scale = self.color_scales.get(self.shade).cloned();
        let suffix = scale
            .as_ref()
            .map(|s| s.suffix.clone())
            .unwrap_or_else(|| "".into());
        let step_hue = if flat[self.center].depth == 0 {
            p.ink
        } else {
            hue_of(
                flat[self.center].branch.map(|b| b as i32).unwrap_or(-1),
                p.ink,
            )
        };
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
            .relative()
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
            .child(bar)
            .child(graph)
            .when(colored, |c| {
                c.child(
                    row()
                        .flex_wrap()
                        .items_center()
                        .gap(px(8.))
                        .child(
                            div()
                                .mr(px(4.))
                                .text_size(px(12.))
                                .line_height(px(16.8))
                                .text_color(p.secondary)
                                .child(
                                    self.shade_labels
                                        .get(self.shade)
                                        .cloned()
                                        .unwrap_or_default(),
                                ),
                        )
                        .child(hint(format!("{}{}", grouped(f64::from(lo)), suffix), p))
                        .child(row().gap(px(2.)).children((0..STEPS).map(|i| {
                            div()
                                .w(px(18.))
                                .h(px(10.))
                                .rounded(px(3.))
                                .bg(p.panel.blend(
                                    step_hue.opacity(
                                        (14. + i as f32 / (STEPS - 1) as f32 * 44.) / 100.,
                                    ),
                                ))
                        })))
                        .child(hint(format!("{}{}", grouped(f64::from(hi)), suffix), p)),
                )
            })
            .when(!empty, |card| {
                // A visually hidden table lists every node's share.
                card.child(
                    div()
                        .id("treemap-readings")
                        .absolute()
                        .size(px(1.))
                        .overflow_hidden()
                        .role(Role::Table)
                        .aria_label(tr(cx, "Treemap readings", "矩形树图读数"))
                        .children(self.flat.iter().enumerate().skip(1).map(|(i, node)| {
                            let parent = node.parent.unwrap_or(0);
                            let mut text = format!(
                                "{}: {}, {} {}",
                                node.path
                                    .iter()
                                    .map(|s| s.as_ref())
                                    .collect::<Vec<_>>()
                                    .join(" / "),
                                grouped(node.value),
                                pct((node.value / self.flat[parent].value.max(1e-9)) as f32),
                                tr(cx, "of parent", "占父级")
                            );
                            if let (Some(color), Some(scale)) =
                                (node.color, self.color_scales.get(self.shade))
                                && colored
                            {
                                text.push_str(&format!(
                                    ", {} {}{}",
                                    self.shade_labels
                                        .get(self.shade)
                                        .cloned()
                                        .unwrap_or_default(),
                                    grouped(f64::from(color)),
                                    scale.suffix
                                ));
                            }
                            div()
                                .id(("treemap-reading", i))
                                .role(Role::Row)
                                .aria_label(text)
                        })),
                )
            })
    }
}
#[cfg(test)]
mod tests {
    use super::FlatNode;
    use gpui::rgb;

    fn rect(x: f32, y: f32, w: f32, h: f32) -> super::Rect {
        super::Rect { x, y, w, h }
    }
    fn squarify(values: &[f64], w: f32, h: f32) -> Vec<super::Rect> {
        let mut out = vec![rect(0., 0., 0., 0.); values.len()];
        super::squarify(values, rect(0., 0., w, h), &mut out);
        out
    }
    #[test]
    fn tiles_fill_space_and_preserve_shares() {
        let r = squarify(&[6., 3., 1.], 400., 200.);
        for (tile, weight) in r.iter().zip([0.6, 0.3, 0.1]) {
            assert!((tile.w * tile.h - weight * 80_000.).abs() < 1.);
        }
        assert!(
            r.iter()
                .all(|t| t.x >= 0. && t.y >= 0. && t.x + t.w <= 401. && t.y + t.h <= 201.)
        );
    }
    #[test]
    fn very_large_values_keep_finite_tiles() {
        let large = f64::from(f32::MAX) * 2.;
        let tiles = squarify(&[large, large], 400., 200.);
        assert!(tiles.iter().all(|t| t.w.is_finite() && t.h.is_finite()));
        assert!((tiles[0].w * tiles[0].h - 40_000.).abs() < 1.);
    }
    #[test]
    fn zero_values_have_empty_tiles() {
        let r = squarify(&[0., 2., 0.], 100., 100.);
        assert_eq!(r[0].w, 0.);
        assert!((r[1].w * r[1].h - 10_000.).abs() < 1.);
        assert_eq!(r[2].w, 0.);
    }
    #[test]
    fn equal_values_seek_square_tiles() {
        let r = squarify(&[1., 1., 1., 1.], 400., 200.);
        let ratio = |t: super::Rect| t.w.max(t.h) / t.w.min(t.h).max(0.001);
        assert!(r.iter().all(|t| ratio(*t) < 2.6), "{r:?}");
    }
    #[test]
    fn carry_maps_source_rect_onto_target() {
        let from = rect(10., 20., 100., 50.);
        let to = rect(0., 0., 200., 100.);
        let corner = super::carry(rect(10., 20., 100., 50.), from, to);
        assert_eq!(corner, rect(0., 0., 200., 100.));
        let inner = super::carry(rect(60., 45., 50., 25.), from, to);
        assert_eq!(inner, rect(100., 50., 100., 50.));
    }
    #[test]
    fn layout_from_insets_branch_header_and_gaps() {
        let flat: Vec<FlatNode> = vec![
            FlatNode {
                id: "root".into(),
                label: "Root".into(),
                value: 10.,
                color: None,
                depth: 0,
                parent: None,
                children: vec![1],
                branch: None,
                path: vec!["Root".into()],
            },
            FlatNode {
                id: "b".into(),
                label: "Branch".into(),
                value: 10.,
                color: None,
                depth: 1,
                parent: Some(0),
                children: vec![2, 3],
                branch: Some(0),
                path: vec!["Branch".into()],
            },
            FlatNode {
                id: "l1".into(),
                label: "One".into(),
                value: 5.,
                color: None,
                depth: 2,
                parent: Some(1),
                children: vec![],
                branch: Some(0),
                path: vec!["Branch".into(), "One".into()],
            },
            FlatNode {
                id: "l2".into(),
                label: "Two".into(),
                value: 5.,
                color: None,
                depth: 2,
                parent: Some(1),
                children: vec![],
                branch: Some(0),
                path: vec!["Branch".into(), "Two".into()],
            },
        ];
        let out = super::layout_from(&flat, 0, rect(0., 0., 400., 200.));
        let branch = out[1].unwrap();
        assert_eq!(branch, rect(1., 1., 398., 198.));
        let one = out[2].unwrap();
        let two = out[3].unwrap();
        // Children sit inside the header inset (top 22, sides 3) and keep
        // their area shares of the remaining room.
        let room = branch.inset(22., 3.);
        for tile in [one, two] {
            assert!((tile.y - (room.y + 1.)).abs() < 0.01, "{tile:?}");
            assert!((tile.h - (room.h - 2.)).abs() < 0.5, "{tile:?}");
            assert!(tile.w > 100. && tile.w < 200., "{tile:?}");
        }
        assert!(
            ((one.x + one.w + 2. - two.x).abs() < 0.5)
                || ((two.x + two.w + 2. - one.x).abs() < 0.5),
            "{one:?} {two:?}"
        );
    }
    #[test]
    fn hop_scores_along_plus_drift_and_excludes_backward() {
        // From (0, 0) heading right: (30, 30) scores 30 + 30*1.5 = 75 while
        // (60, 2) scores 60 + 2*1.5 = 63, so the aligned tile wins.
        let centers = [(0., 0.), (30., 30.), (60., 2.)];
        assert_eq!(super::hop(&centers, 0, 1., 0.), Some(2));
        assert_eq!(super::hop(&centers, 0, 0., 1.), Some(1));
        assert_eq!(super::hop(&centers, 0, -1., 0.), None);
        let row = [(10., 10.), (50., 10.), (10., 50.), (50., 50.)];
        assert_eq!(super::hop(&row, 0, 1., 0.), Some(1));
        assert_eq!(super::hop(&row, 0, 0., 1.), Some(2));
        assert_eq!(super::hop(&row, 3, -1., 0.), Some(2));
        assert_eq!(super::hop(&row, 0, 0., -1.), None);
    }
    #[test]
    fn shade_quantizes_to_five_neutral_steps() {
        let cases = [
            (Some(0.), 14.),
            (Some(2.5), 25.),
            (Some(5.), 36.),
            (Some(7.5), 47.),
            (Some(10.), 58.),
            (Some(-5.), 14.),
            (None, 34.),
        ];
        for (value, want) in cases {
            assert_eq!(super::shade_percent(true, value, 0., 10.), want);
        }
        assert_eq!(super::shade_percent(false, Some(9.), 0., 10.), 34.);
    }
    #[test]
    fn reveal_delay_shrinks_with_tile_share() {
        let view = rect(0., 0., 100., 100.);
        let big = super::reveal_delay(rect(0., 0., 50., 50.), view);
        let small = super::reveal_delay(rect(0., 0., 5., 5.), view);
        assert!(big < small);
        assert!(small <= 0.3 + 1e-6);
    }
    #[test]
    fn flatten_sums_values_and_blends_color_by_weight() {
        use super::{TreeTile, flatten};
        let leaves = |vals: [(f32, f32); 2]| -> Vec<TreeTile> {
            vals.iter()
                .map(|&(v, c)| TreeTile::new("x", "X", v, rgb(0).into()).with_shade_values(vec![c]))
                .collect()
        };
        let root = vec![
            TreeTile::new("a", "A", 0., rgb(0).into())
                .with_children(leaves([(2., 10.), (2., 30.)])),
            TreeTile::new("b", "B", 3., rgb(0).into()).with_shade_values(vec![99.]),
        ];
        let flat = flatten(&"Whole".into(), &root, 0, 0);
        assert_eq!(flat[0].children, vec![1, 4]);
        assert_eq!(flat[1].value, 4.);
        assert_eq!(flat[1].color, Some(20.)); // weighted mean of leaf colors
        assert_eq!(flat[4].value, 3.);
        assert_eq!(flat[4].color, Some(99.)); // a leaf keeps its own shade value
        assert_eq!(flat[0].value, 7.);
        let labels =
            |i: usize| -> Vec<String> { flat[i].path.iter().map(|s| s.to_string()).collect() };
        assert_eq!(labels(2), vec!["A", "X"]);
        assert_eq!(labels(1), vec!["A"]);
        assert_eq!(labels(0), vec!["Whole"]);
        assert_eq!(flat[4].branch, Some(1));
        assert_eq!(flat[3].branch, Some(0));
    }
    #[test]
    fn grouped_formats_thousands_with_one_fraction() {
        assert_eq!(super::grouped(18_400.), "18,400");
        assert_eq!(super::grouped(345.64), "345.6");
        assert_eq!(super::grouped(400.), "400");
        assert_eq!(super::grouped(0.), "0");
        assert_eq!(super::grouped(-1_234.5), "-1,234.5");
    }
}
