//! A part-to-whole comparison whose boundaries animate when the data changes.
use crate::components::animated_counter::AnimatedCounter;
use crate::primitives::{
    family::{Paint, Slide, Tone, disabled_motion, face, measure},
    presentation::{column, hint, row, title, tr},
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};

#[derive(Clone, Debug)]
pub struct SplitSegment {
    pub id: SharedString,
    pub label: SharedString,
    pub value: f64,
    pub color: Hsla,
}
impl SplitSegment {
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        value: f64,
        color: Hsla,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            value,
            color,
        }
    }
    fn safe_value(&self) -> f64 {
        if self.value.is_finite() {
            self.value.max(0.)
        } else {
            0.
        }
    }
}
#[derive(Clone, Debug)]
pub struct SplitInspect {
    pub id: SharedString,
    pub value: f64,
    pub share: f64,
}
struct SegmentEdge {
    id: SharedString,
    left: Slide,
    right: Slide,
    outside: Slide,
    label_x: Slide,
    label_y: Slide,
    name: Slide,
    outside_name: Slide,
    inside_counter: Option<Entity<AnimatedCounter>>,
    outside_counter: Option<Entity<AnimatedCounter>>,
    legend_counter: Option<Entity<AnimatedCounter>>,
}
/// Data is host controlled; visibility and the inspected segment are local.
pub struct SplitBar {
    title: SharedString,
    description: SharedString,
    unit: SharedString,
    data: Vec<SplitSegment>,
    hidden: Vec<SharedString>,
    edges: Vec<SegmentEdge>,
    active: Option<usize>,
    bounds: Rc<Cell<Bounds<Pixels>>>,
    initialized: bool,
    layout_ready: bool,
    visible: bool,
    lane: Slide,
    hover_share: f32,
    readout: Option<Entity<AnimatedCounter>>,
}
impl EventEmitter<SplitInspect> for SplitBar {}
impl SplitBar {
    pub fn new(title: impl Into<SharedString>, data: Vec<SplitSegment>) -> Self {
        Self {
            title: title.into(),
            description: "".into(),
            unit: "".into(),
            data,
            hidden: vec![],
            edges: vec![],
            active: None,
            bounds: Default::default(),
            initialized: false,
            layout_ready: false,
            visible: true,
            lane: Slide::new(0.),
            hover_share: 0.,
            readout: None,
        }
    }
    pub fn with_description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = description.into();
        self
    }
    pub fn with_unit(mut self, unit: impl Into<SharedString>) -> Self {
        self.unit = unit.into();
        self
    }
    /// Stop animation scheduling while the embedding host hides the chart.
    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        if self.visible != visible {
            self.visible = visible;
            cx.notify();
        }
    }
    pub fn set_segments(&mut self, data: Vec<SplitSegment>, cx: &mut Context<Self>) {
        self.data = data;
        self.hidden
            .retain(|id| self.data.iter().any(|s| s.id == *id));
        self.active = None;
        self.reconcile(cx);
        cx.notify();
    }
    fn total(&self) -> f64 {
        self.data
            .iter()
            .filter(|s| !self.hidden.contains(&s.id))
            .map(SplitSegment::safe_value)
            .sum()
    }
    fn shares(&self) -> Vec<f64> {
        normalized_shares(
            &self
                .data
                .iter()
                .map(|segment| {
                    if self.hidden.contains(&segment.id) {
                        0.
                    } else {
                        segment.safe_value()
                    }
                })
                .collect::<Vec<_>>(),
        )
    }
    fn reconcile(&mut self, cx: &App) {
        let base = self.shares();
        let targets = expanded_shares(&base, self.active, self.hover_share);
        let mut left = 0f32;
        let mut previous: Vec<Option<SegmentEdge>> = std::mem::take(&mut self.edges)
            .into_iter()
            .map(Some)
            .collect();
        self.edges = self
            .data
            .iter()
            .enumerate()
            .map(|(index, segment)| {
                let right = (left + targets[index] as f32).min(1.);
                let mut edge = previous
                    .iter()
                    .position(|e| e.as_ref().is_some_and(|v| v.id == segment.id))
                    .and_then(|i| previous[i].take())
                    .unwrap_or_else(|| SegmentEdge {
                        id: segment.id.clone(),
                        left: Slide::new(left),
                        right: Slide::new(if self.initialized { left } else { right }),
                        outside: Slide::new(0.),
                        label_x: Slide::new(0.),
                        label_y: Slide::new(0.),
                        name: Slide::new(1.),
                        outside_name: Slide::new(1.),
                        inside_counter: None,
                        outside_counter: None,
                        legend_counter: None,
                    });
                edge.left.duration(left, 0.4, cx);
                edge.right.duration(right, 0.4, cx);
                left = right;
                edge
            })
            .collect();
        self.initialized = true;
    }
    fn inspect(&mut self, next: Option<usize>, w: &Window, cx: &mut Context<Self>) {
        if self.active == next {
            return;
        }
        self.active = next;
        self.hover_share = next
            .and_then(|index| self.data.get(index))
            .map_or(0., |segment| {
                (measure(&segment.label, 12., w) + 74.)
                    / f32::from(self.bounds.get().size.width).max(1.)
            });
        self.reconcile(cx);
        if let Some(s) = next.and_then(|i| self.data.get(i)) {
            let share = next.map_or(0., |index| self.shares()[index]);
            cx.emit(SplitInspect {
                id: s.id.clone(),
                value: s.safe_value(),
                share,
            });
        }
        cx.notify();
    }
    fn toggle(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(s) = self.data.get(index) {
            if let Some(pos) = self.hidden.iter().position(|id| *id == s.id) {
                self.hidden.remove(pos);
            } else if self.data.len().saturating_sub(self.hidden.len()) > 1 {
                self.hidden.push(s.id.clone());
            } else {
                return;
            }
            self.active = None;
            self.reconcile(cx);
            cx.notify();
        }
    }
}
// Normalize before summing to keep finite extreme values proportional.
fn normalized_shares(values: &[f64]) -> Vec<f64> {
    let scale = values.iter().copied().fold(0_f64, f64::max);
    if scale <= 0. {
        return vec![0.; values.len()];
    }
    let total: f64 = values.iter().map(|value| value / scale).sum();
    values.iter().map(|value| (value / scale) / total).collect()
}
fn expanded_shares(base: &[f64], active: Option<usize>, minimum: f32) -> Vec<f64> {
    let Some(index) =
        active.filter(|&i| base.get(i).is_some_and(|value| *value > 0. && *value < 1.))
    else {
        return base.to_vec();
    };
    let target = (base[index] + 0.045)
        .max(f64::from(minimum))
        .min(0.98)
        .max(base[index]);
    let scale = (1. - target) / (1. - base[index]);
    base.iter()
        .enumerate()
        .map(|(i, value)| if i == index { target } else { value * scale })
        .collect()
}

// Pack callouts as a group. A backwards pass reserves room for labels at the
// right edge instead of clamping every label onto the same final position.
fn pack_labels(labels: &[(f32, f32)], width: f32, gap: f32) -> Vec<(f32, f32)> {
    let width = width.max(1.);
    let mut result = vec![(0., 0.); labels.len()];
    let mut start = 0;
    let mut row = 0;
    while start < labels.len() {
        let mut end = start;
        let mut used = 0.;
        while end < labels.len() {
            let next = labels[end].1.min(width);
            if end > start && used + gap + next > width {
                break;
            }
            used += next + if end > start { gap } else { 0. };
            end += 1;
        }
        let mut previous_end = -gap;
        for i in start..end {
            let label_width = labels[i].1.min(width);
            let x = (labels[i].0 - label_width / 2.).max(previous_end + gap);
            result[i] = (x, row as f32 * 24.);
            previous_end = x + label_width;
        }
        let mut next_start = width + gap;
        for i in (start..end).rev() {
            result[i].0 = result[i]
                .0
                .min(next_start - gap - labels[i].1.min(width))
                .max(0.);
            next_start = result[i].0;
        }
        start = end;
        row += 1;
    }
    result
}
fn layout_target(slide: &mut Slide, target: f32, first_layout: bool, cx: &App) {
    if first_layout {
        slide.reset(target);
    } else {
        slide.duration(target, 0.4, cx);
    }
}
impl Render for SplitBar {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.initialized {
            self.reconcile(cx);
        }
        let p = Paint::get(cx);
        let b = self.bounds.get();
        let width = f32::from(b.size.width).max(1.);
        let first_layout = !self.layout_ready && b.size.width > px(1.);
        let viewport = w.viewport_size();
        let intersects = |b: Bounds<Pixels>| {
            b.size.width > px(0.)
                && b.right() > px(0.)
                && b.left() < viewport.width
                && b.bottom() > px(0.)
                && b.top() < viewport.height
        };
        let visible = self.visible && w.is_visible() && intersects(b);
        let reduced = disabled_motion(cx);
        let now = cx.background_executor().now();
        let mut moving = false;
        let mut sample = |slide: &Slide| {
            let (value, active) = slide.sample_batch(now, reduced);
            moving |= active;
            value
        };
        let base = self.shares();
        let mut lane_needed = false;
        let outside_labels: Vec<_> =
            self.edges
                .iter()
                .enumerate()
                .filter_map(|(i, edge)| {
                    let left = edge.left.sample_batch(now, reduced).0.clamp(0., 1.);
                    let right = edge.right.sample_batch(now, reduced).0.clamp(left, 1.);
                    let name_width = measure(&self.data[i].label, 12., w);
                    let percent_width = measure(&format!("{:.1}%", base[i] * 100.).into(), 12., w);
                    (((right - left) * width - 2. < percent_width + 24.) && base[i] > 0.)
                        .then_some((i, (left + right) * width * 0.5, name_width, percent_width))
                })
                .collect();
        let names_fit = outside_labels
            .iter()
            .map(|(_, _, name, percent)| name + percent + 4.)
            .sum::<f32>()
            + outside_labels.len().saturating_sub(1) as f32 * 12.
            <= width;
        let label_targets = pack_labels(
            &outside_labels
                .iter()
                .map(|(_, center, name, percent)| {
                    (
                        *center,
                        if names_fit {
                            name + 4. + percent
                        } else {
                            *percent
                        },
                    )
                })
                .collect::<Vec<_>>(),
            width,
            12.,
        );
        let mut pieces = Vec::new();
        let mut counter_views = Vec::new();
        for (i, edge) in self.edges.iter_mut().enumerate() {
            let left = sample(&edge.left).clamp(0., 1.);
            let right = sample(&edge.right).clamp(left, 1.);
            let segment = &self.data[i];
            let name_width = measure(&segment.label, 12., w);
            let percent_width = measure(&format!("{:.1}%", base[i] * 100.).into(), 12., w);
            let available = (right - left) * width - 2.;
            let outside = available < percent_width + 24. && base[i] > 0.;
            lane_needed |= outside;
            layout_target(&mut edge.outside, f32::from(outside), first_layout, cx);
            layout_target(
                &mut edge.name,
                f32::from(available >= name_width + percent_width + 30.),
                first_layout,
                cx,
            );
            layout_target(
                &mut edge.outside_name,
                f32::from(names_fit),
                first_layout,
                cx,
            );
            let outside_name = sample(&edge.outside_name).clamp(0., 1.);
            let label_width = (name_width + 4.) * outside_name + percent_width;
            let (label_x, label_y) = outside_labels
                .iter()
                .position(|label| label.0 == i)
                .map_or((0., 0.), |index| label_targets[index]);
            layout_target(&mut edge.label_x, label_x, first_layout, cx);
            layout_target(&mut edge.label_y, label_y, first_layout, cx);
            let outside_progress = sample(&edge.outside);
            let mut counter = |slot: &mut Option<Entity<AnimatedCounter>>,
                               shown: bool,
                               font_size: f32,
                               weight: FontWeight| {
                let view = slot
                    .get_or_insert_with(|| {
                        cx.new(|_| {
                            AnimatedCounter::new(base[i] * 100.)
                                .with_suffix("%")
                                .with_decimals(1)
                                .with_size(font_size)
                                .with_wheel_style(1.2, 0.18, weight)
                                .with_accessibility(false)
                        })
                    })
                    .clone();
                view.update(cx, |view, cx| {
                    view.set_visible(visible && shown, cx);
                    view.set_value(base[i] * 100., cx);
                });
                view
            };
            counter_views.push((
                counter(
                    &mut edge.inside_counter,
                    outside_progress < 0.999,
                    12.,
                    FontWeight::MEDIUM,
                ),
                counter(
                    &mut edge.outside_counter,
                    outside_progress > 0.001,
                    12.,
                    FontWeight::MEDIUM,
                ),
                counter(&mut edge.legend_counter, true, 14., FontWeight::NORMAL),
            ));
            pieces.push((
                left,
                right,
                outside_progress,
                sample(&edge.name),
                sample(&edge.label_x),
                outside_name,
                segment.label.clone(),
                segment.color,
                label_width,
                sample(&edge.label_y),
            ));
        }
        let label_rows = label_targets.iter().map(|(_, y)| *y).fold(0_f32, f32::max);
        layout_target(
            &mut self.lane,
            if lane_needed { 40. + label_rows } else { 0. },
            first_layout,
            cx,
        );
        self.layout_ready |= b.size.width > px(1.);
        let lane = sample(&self.lane).max(0.);
        if visible && moving {
            w.request_animation_frame();
        }
        let pointer_spans: Vec<_> = pieces.iter().map(|piece| (piece.0, piece.1)).collect();
        let active = self.active;
        let bounds = self.bounds.clone();
        let radius = crate::foundation::surface_radius(cx);
        let first_visible = pieces.iter().position(|piece| piece.1 > piece.0);
        let last_visible = pieces.iter().rposition(|piece| piece.1 > piece.0);
        let annotations: Vec<_> = pieces
            .iter()
            .map(|piece| {
                (
                    piece.0,
                    piece.1,
                    piece.2,
                    piece.3,
                    piece.4,
                    piece.5,
                    piece.6.clone(),
                    piece.9,
                )
            })
            .collect();
        let mut graph = div()
            .id("split-bar")
            .role(Role::Figure)
            .tab_index(0)
            .w_full()
            .h(px(44. + lane))
            .relative()
            .aria_label(format!(
                "{}. {}. {}",
                self.title,
                self.data
                    .iter()
                    .enumerate()
                    .map(|(i, segment)| format!("{} {:.1}%", segment.label, base[i] * 100.))
                    .collect::<Vec<_>>()
                    .join(", "),
                tr(
                    cx,
                    "Use arrow keys to inspect segments",
                    "使用方向键查看各部分"
                )
            ))
            .on_hover(cx.listener(|s, hovered: &bool, w, cx| {
                if !hovered {
                    s.inspect(None, w, cx);
                }
            }))
            .on_mouse_move(cx.listener(move |s, e: &MouseMoveEvent, w, cx| {
                let bounds = s.bounds.get();
                if e.position.y > bounds.top() + px(44.) {
                    return;
                }
                let x =
                    f32::from(e.position.x - bounds.left()) / f32::from(bounds.size.width).max(1.);
                let next = pointer_spans
                    .iter()
                    .position(|(left, right)| right > left && x >= *left && x <= *right);
                s.inspect(next, w, cx);
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|s, e: &MouseDownEvent, w, cx| {
                    let b = s.bounds.get();
                    let x = f32::from(e.position.x - b.left()) / f32::from(b.size.width).max(1.);
                    let next = s
                        .edges
                        .iter()
                        .position(|edge| x >= edge.left.value(cx) && x <= edge.right.value(cx));
                    s.inspect(next, w, cx);
                }),
            )
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                let available: Vec<_> = s
                    .data
                    .iter()
                    .enumerate()
                    .filter(|(_, item)| !s.hidden.contains(&item.id))
                    .map(|(i, _)| i)
                    .collect();
                if available.is_empty() {
                    return;
                }
                let at = s
                    .active
                    .and_then(|index| available.iter().position(|i| *i == index));
                let next = match e.keystroke.key.as_str() {
                    "right" | "down" => {
                        Some(available[at.map_or(0, |at| (at + 1) % available.len())])
                    }
                    "left" | "up" => Some(
                        available[at.map_or(available.len() - 1, |at| {
                            (at + available.len() - 1) % available.len()
                        })],
                    ),
                    "home" => available.first().copied(),
                    "end" => available.last().copied(),
                    "escape" => None,
                    _ => return,
                };
                w.prevent_default();
                cx.stop_propagation();
                s.inspect(next, w, cx);
            }))
            .child(
                canvas(
                    move |b, w, _| {
                        let before = bounds.replace(b);
                        let viewport = w.viewport_size();
                        let in_view = |b: Bounds<Pixels>| {
                            b.size.width > px(0.)
                                && b.right() > px(0.)
                                && b.left() < viewport.width
                                && b.bottom() > px(0.)
                                && b.top() < viewport.height
                        };
                        if before.size != b.size || in_view(before) != in_view(b) {
                            w.request_animation_frame();
                        }
                    },
                    move |b, _, w, cx| {
                        let width = f32::from(b.size.width);
                        let paint_text = |text: SharedString,
                                          at: Point<Pixels>,
                                          color: Hsla,
                                          size: f32,
                                          w: &mut Window,
                                          cx: &mut App| {
                            let mut run = w.text_style().to_run(text.len());
                            run.color = color;
                            let line = w.text_system().shape_line(text, px(size), &[run], None);
                            let _ = line.paint(at, px(18.), TextAlign::Left, None, w, cx);
                        };
                        for (
                            i,
                            (
                                left,
                                right,
                                outside,
                                name,
                                label_x,
                                outside_name,
                                label,
                                color,
                                label_width,
                                label_y,
                            ),
                        ) in pieces.iter().enumerate()
                        {
                            let piece_width = ((right - left) * width - 2.).max(0.);
                            if piece_width < 0.1 {
                                continue;
                            }
                            let dim = if active.is_none_or(|active| active == i) {
                                1.
                            } else {
                                0.5
                            };
                            let piece = Bounds::new(
                                point(b.left() + px(left * width), b.top()),
                                size(px(piece_width), px(44.)),
                            );
                            let corners = Corners {
                                top_left: if Some(i) == first_visible {
                                    radius
                                } else {
                                    px(0.)
                                },
                                bottom_left: if Some(i) == first_visible {
                                    radius
                                } else {
                                    px(0.)
                                },
                                top_right: if Some(i) == last_visible {
                                    radius
                                } else {
                                    px(0.)
                                },
                                bottom_right: if Some(i) == last_visible {
                                    radius
                                } else {
                                    px(0.)
                                },
                            };
                            let mix = if active == Some(i) { 0.62 } else { 0.42 };
                            w.paint_quad(
                                fill(piece, p.stage.blend(color.opacity(mix)).opacity(dim))
                                    .corner_radii(corners),
                            );
                            if Some(i) != last_visible {
                                w.paint_quad(fill(
                                    Bounds::new(
                                        point(piece.right() - px(3.), piece.top()),
                                        size(px(3.), px(44.)),
                                    ),
                                    color.opacity(dim),
                                ));
                            }
                            w.with_content_mask(Some(ContentMask { bounds: piece }), |w| {
                                let x = piece.left() + px(10.);
                                let name_width = measure(label, 12., w) * name;
                                if *name > 0.001 {
                                    let name_bounds = Bounds::new(
                                        point(x, piece.top()),
                                        size(px(name_width), piece.size.height),
                                    )
                                    .intersect(&piece);
                                    w.with_content_mask(
                                        Some(ContentMask {
                                            bounds: name_bounds,
                                        }),
                                        |w| {
                                            paint_text(
                                                label.clone(),
                                                point(x, piece.top() + px(13.)),
                                                p.ink.opacity(dim * (1. - outside) * name),
                                                12.,
                                                w,
                                                cx,
                                            );
                                        },
                                    );
                                }
                            });
                            if *outside > 0.001 {
                                let center = (left + right) * width * 0.5;
                                let end = label_x + label_width * 0.5;
                                let mut path = PathBuilder::stroke(px(1.));
                                path.move_to(point(b.left() + px(center), b.top() + px(44.)));
                                path.cubic_bezier_to(
                                    point(b.left() + px(end), b.top() + px(59. + label_y)),
                                    point(b.left() + px(center), b.top() + px(54.)),
                                    point(b.left() + px(end), b.top() + px(50. + label_y)),
                                );
                                if let Ok(path) = path.build() {
                                    w.paint_path(
                                        path,
                                        p.faint.opacity(outside * if dim < 1. { 0.4 } else { 1. }),
                                    );
                                }
                                if *outside_name > 0.001 {
                                    paint_text(
                                        label.clone(),
                                        point(b.left() + px(*label_x), b.top() + px(60. + label_y)),
                                        p.ink.opacity(
                                            outside
                                                * outside_name
                                                * if dim < 1. { 0.45 } else { 1. },
                                        ),
                                        12.,
                                        w,
                                        cx,
                                    );
                                }
                            }
                        }
                    },
                )
                .size_full(),
            );
        for (
            (left, right, outside, name, label_x, outside_name, label, label_y),
            (inside, outside_view, _),
        ) in annotations.into_iter().zip(&counter_views)
        {
            let piece_width = ((right - left) * width - 2.).max(0.);
            if piece_width < 0.1 {
                continue;
            }
            graph = graph
                .child(
                    div()
                        .absolute()
                        .left(px(left * width))
                        .top_0()
                        .w(px(piece_width))
                        .h(px(44.))
                        .overflow_hidden()
                        .child(
                            div()
                                .absolute()
                                .left(px(10. + (measure(&label, 12., w) + 4.) * name))
                                .top(px(15.5))
                                .opacity(1. - outside)
                                .child(inside.clone()),
                        ),
                )
                .child(
                    div()
                        .absolute()
                        .left(px(label_x + (measure(&label, 12., w) + 4.) * outside_name))
                        .top(px(62.5 + label_y))
                        .opacity(outside)
                        .child(outside_view.clone()),
                );
        }
        let total = self.total();
        let value = self
            .active
            .map_or(total, |index| self.data[index].safe_value());
        let readout = self
            .readout
            .get_or_insert_with(|| {
                cx.new(|_| {
                    AnimatedCounter::new(value)
                        .with_size(18.)
                        .with_wheel_style(1.2, 0.18, FontWeight::MEDIUM)
                        .with_accessibility(false)
                })
            })
            .clone();
        readout.update(cx, |counter, cx| {
            counter.set_visible(visible, cx);
            counter.set_value(value, cx);
        });
        let detail: SharedString = self.active.map_or_else(
            || tr(cx, "Total", "总计").into(),
            |index| self.data[index].label.clone(),
        );
        column()
            .w(px(520.))
            .max_w_full()
            .gap(px(12.))
            .text_color(p.ink)
            .child(
                row()
                    .flex_wrap()
                    .justify_between()
                    .gap(px(12.))
                    .child(
                        column()
                            .gap(px(2.))
                            .child(title(self.title.clone(), p))
                            .when(!self.description.is_empty(), |view| {
                                view.child(
                                    hint(self.description.clone(), p)
                                        .text_size(px(14.))
                                        .line_height(px(19.6)),
                                )
                            }),
                    )
                    .child(
                        row()
                            .items_baseline()
                            .gap(px(8.))
                            .child(hint(detail, p).text_size(px(14.)).line_height(px(20.)))
                            .when(value.is_finite(), |view| view.child(readout))
                            .when(!value.is_finite(), |view| view.child(">1.8e308"))
                            .when(!self.unit.is_empty(), |view| {
                                view.child(
                                    hint(self.unit.clone(), p)
                                        .text_size(px(14.))
                                        .line_height(px(20.)),
                                )
                            }),
                    ),
            )
            .child(graph)
            .child(
                row()
                    .flex_wrap()
                    .ml(px(-8.))
                    .gap_x(px(4.))
                    .gap_y(px(2.))
                    .children(self.data.iter().enumerate().map(|(i, segment)| {
                        let off = self.hidden.contains(&segment.id);
                        let label = format!(
                            "{}  {}",
                            segment.label,
                            if off {
                                tr(cx, "Off", "已隐藏").to_owned()
                            } else {
                                format!("{:.1}%", base[i] * 100.)
                            }
                        );
                        face(
                            ("split-segment", i),
                            label.clone(),
                            32.,
                            radius.into(),
                            Tone::Clear,
                            cx,
                        )
                        .pl(px(8.))
                        .pr(px(10.))
                        .aria_toggled((!off).into())
                        .gap(px(8.))
                        .text_size(px(14.))
                        .line_height(px(20.))
                        .font_weight(FontWeight::NORMAL)
                        .hover(|s| s.bg(p.tray))
                        .child(
                            div()
                                .relative()
                                .size(px(12.))
                                .flex_none()
                                .rounded(px(3.))
                                .when(off, |view| view.border(px(1.5)).border_color(p.faint))
                                .when(!off, |view| {
                                    view.bg(p.panel.blend(segment.color.opacity(0.42))).child(
                                        div()
                                            .absolute()
                                            .right_0()
                                            .top_0()
                                            .bottom_0()
                                            .w(px(3.))
                                            .bg(segment.color),
                                    )
                                }),
                        )
                        .child(segment.label.clone())
                        .when(off, |view| view.child(tr(cx, "Off", "已隐藏")))
                        .when(!off, |view| view.child(counter_views[i].2.clone()))
                        .on_click(cx.listener(move |s, _, _, cx| s.toggle(i, cx)))
                    })),
            )
    }
}
#[cfg(test)]
mod tests {
    use super::{expanded_shares, normalized_shares, pack_labels};
    #[test]
    fn large_finite_values_keep_their_proportions() {
        let normalized = normalized_shares(&[f64::MAX, f64::MAX, f64::MAX / 2.]);
        assert!((normalized[0] - 0.4).abs() < 1e-12);
        assert!((normalized[2] - 0.2).abs() < 1e-12);
    }
    #[test]
    fn inspection_gives_space_without_changing_the_whole() {
        let base = normalized_shares(&[40., 30., 29., 1.]);
        let expanded = expanded_shares(&base, Some(3), 0.2);
        assert!((expanded.iter().sum::<f64>() - 1.).abs() < 1e-12);
        assert!((expanded[3] - 0.2).abs() < 1e-7);
        assert!((expanded[0] / expanded[1] - base[0] / base[1]).abs() < 1e-12);
    }
    #[test]
    fn hidden_segments_do_not_reserve_empty_gaps() {
        let result = expanded_shares(&normalized_shares(&[0., 20., 80.]), Some(1), 0.3);
        assert_eq!(result[0], 0.);
        assert!((result.iter().sum::<f64>() - 1.).abs() < 1e-12);
    }

    #[test]
    fn right_edge_callouts_reserve_space_for_the_whole_cluster() {
        let labels = [(450., 82.), (490., 91.), (515., 70.)];
        let packed = pack_labels(&labels, 520., 12.);
        assert!(packed[0].0 >= 0.);
        for i in 1..packed.len() {
            assert!(packed[i].0 >= packed[i - 1].0 + labels[i - 1].1 + 12.);
        }
        assert!(packed[2].0 + labels[2].1 <= 520.);
    }

    #[test]
    fn narrow_callouts_keep_every_percentage_inside_its_lane() {
        let labels: Vec<_> = (0..8).map(|i| (i as f32 * 29. + 14.5, 33.)).collect();
        let packed = pack_labels(&labels, 232., 12.);
        for (i, &(x, y)) in packed.iter().enumerate() {
            assert!(x >= 0. && x + labels[i].1 <= 232.);
            if i > 0 && y == packed[i - 1].1 {
                assert!(x >= packed[i - 1].0 + labels[i - 1].1 + 12.);
            }
        }
    }
}
