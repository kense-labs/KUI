//! Compare paired measurements with interactive, animated connecting lines.
use crate::primitives::{
    family::{Paint, PhaseClock, Slide, disabled_motion},
    presentation::{in_out, tr},
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};

/// A stable series and its measurements on the left and right axes.
#[derive(Clone, Debug)]
pub struct SlopeSeries {
    pub id: SharedString,
    pub label: SharedString,
    pub before: f32,
    pub after: f32,
    pub color: Hsla,
}
impl SlopeSeries {
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        before: f32,
        after: f32,
        color: Hsla,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            before: if before.is_finite() { before } else { 0. },
            after: if after.is_finite() { after } else { 0. },
            color,
        }
    }
}
/// Current series selected by pointer, legend or keyboard.
#[derive(Clone, Debug)]
pub struct SlopeInspect {
    pub id: SharedString,
    pub before: f32,
    pub after: f32,
}
struct Slope {
    id: SharedString,
    before: Slide,
    after: Slide,
}
/// A host-controlled paired-series comparison.
pub struct SlopeChart {
    title: SharedString,
    series: Vec<SlopeSeries>,
    visual: Vec<Slope>,
    active: Option<usize>,
    bounds: Rc<Cell<Bounds<Pixels>>>,
    initialized: bool,
    draw: PhaseClock,
    revealed: bool,
    visible: bool,
    touching: Rc<Cell<bool>>,
}
impl EventEmitter<SlopeInspect> for SlopeChart {}
impl SlopeChart {
    pub fn new(title: impl Into<SharedString>, series: Vec<SlopeSeries>) -> Self {
        Self {
            title: title.into(),
            series,
            visual: vec![],
            active: None,
            bounds: Default::default(),
            initialized: false,
            draw: PhaseClock::default(),
            revealed: false,
            visible: true,
            touching: Default::default(),
        }
    }
    /// The host sets this to false when the chart or its tab is hidden.
    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        if self.visible != visible {
            self.visible = visible;
            cx.notify();
        }
    }
    pub fn set_series(&mut self, series: Vec<SlopeSeries>, cx: &mut Context<Self>) {
        self.series = series;
        let old = std::mem::take(&mut self.visual);
        let mut old: Vec<Option<Slope>> = old.into_iter().map(Some).collect();
        self.visual = self
            .series
            .iter()
            .map(|s| {
                let index = old
                    .iter()
                    .position(|x| x.as_ref().is_some_and(|x| x.id == s.id));
                let mut state = index.and_then(|i| old[i].take()).unwrap_or_else(|| Slope {
                    id: s.id.clone(),
                    before: Slide::new(s.before),
                    after: Slide::new(s.before),
                });
                state.before.duration(s.before, 0.42, cx);
                state.after.duration(s.after, 0.42, cx);
                state
            })
            .collect();
        self.initialized = true;
        self.active = None;
        cx.notify();
    }
    fn inspect(&mut self, next: Option<usize>, cx: &mut Context<Self>) {
        if self.active == next {
            return;
        }
        self.active = next;
        if let Some(item) = next.and_then(|i| self.series.get(i)) {
            cx.emit(SlopeInspect {
                id: item.id.clone(),
                before: item.before,
                after: item.after,
            });
        }
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
fn gutters(width: f32) -> (f32, f32) {
    let narrow = width < 440.;
    let left = if !narrow {
        168.
    } else if width < 300. {
        116.
    } else {
        132.
    };
    let right = if narrow { 76. } else { 96. };
    (left, right)
}
fn spread_labels(values: &[f32], height: f32) -> Vec<f32> {
    let mut order: Vec<usize> = (0..values.len()).collect();
    order.sort_by(|&a, &b| values[a].total_cmp(&values[b]));
    let mut output = vec![0.; values.len()];
    let gap = 20.;
    let low = 8.;
    let high = (height - 8.).max(low);
    let mut previous = low - gap;
    for (rank, &i) in order.iter().enumerate() {
        let max = high - gap * (order.len() - 1 - rank) as f32;
        let y = values[i].max(previous + gap).min(max.max(low));
        output[i] = y;
        previous = y;
    }
    output
}
fn nearest_series(
    b: Bounds<Pixels>,
    pos: Point<Pixels>,
    points: &[(f32, f32, Hsla)],
    lower: f32,
    span: f32,
) -> Option<usize> {
    let width = f32::from(b.size.width).max(1.);
    let height = f32::from(b.size.height).max(1.);
    let x = f32::from(pos.x - b.origin.x);
    let y = f32::from(pos.y - b.origin.y);
    if x < 0. || y < 0. || x > width || y > height {
        return None;
    }
    let (left, right) = gutters(width);
    let t = ((x - left) / (width - left - right).max(1.)).clamp(0., 1.);
    points
        .iter()
        .enumerate()
        .map(|(i, (a, z, _))| {
            let v = a + (z - a) * t;
            let target = 14. + (1. - (v - lower) / span) * (height - 28.);
            (i, (target - y).abs())
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .filter(|(_, d)| *d < 24.)
        .map(|(i, _)| i)
}
impl Render for SlopeChart {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.initialized {
            self.initialized = true;
            self.visual = self
                .series
                .iter()
                .map(|s| Slope {
                    id: s.id.clone(),
                    before: Slide::new(s.before),
                    after: Slide::new(s.after),
                })
                .collect();
        }
        let measured = self.bounds.get();
        let on_screen =
            self.visible && w.is_visible() && intersects_viewport(measured, w.viewport_size());
        if on_screen && !self.revealed {
            self.revealed = true;
            self.draw.restart(cx);
        }
        let now = cx.background_executor().now();
        let reduced = disabled_motion(cx);
        let age = if reduced {
            100.
        } else if self.revealed {
            self.draw.age(cx)
        } else {
            0.
        };
        let line_done = 0.1 + self.series.len().saturating_sub(1) as f32 * 0.05 + 0.7;
        let mut moving = false;
        let p = Paint::get(cx);
        let values: Vec<(f32, f32, Hsla)> = self
            .visual
            .iter()
            .zip(&self.series)
            .map(|(v, s)| {
                let (before, a) = v.before.sample_batch(now, reduced);
                let (after, b) = v.after.sample_batch(now, reduced);
                moving |= a || b;
                (before, after, s.color)
            })
            .collect();
        if on_screen && (moving || (!reduced && self.revealed && age < line_done)) {
            w.request_animation_frame();
        }
        let lower = values.iter().fold(0f32, |a, (x, y, _)| a.min(*x).min(*y));
        let upper = values.iter().fold(1f32, |a, (x, y, _)| a.max(*x).max(*y));
        let span = (upper - lower).max(0.001);
        let active = self.active;
        let p_bounds = self.bounds.clone();
        let points = values.clone();
        let touch_points = values.clone();
        let touching = self.touching.clone();
        let touch = cx.listener(move |s, e: &TouchDragEvent, _, cx| {
            s.inspect(
                nearest_series(s.bounds.get(), e.position, &touch_points, lower, span),
                cx,
            );
        });
        let height = (self.series.len() as f32 * 44.).max(200.);
        let width = if measured.size.width > px(0.) {
            f32::from(measured.size.width)
        } else {
            300.
        };
        let (left, right) = gutters(width);
        let y_of = |v: f32| 14. + (1. - (v - lower) / span) * (height - 28.);
        let left_y = spread_labels(
            &values.iter().map(|(v, _, _)| y_of(*v)).collect::<Vec<_>>(),
            height,
        );
        let right_y = spread_labels(
            &values.iter().map(|(_, v, _)| y_of(*v)).collect::<Vec<_>>(),
            height,
        );
        let mut labels = Vec::new();
        for (i, series) in self.series.iter().enumerate() {
            let before_rank = self
                .series
                .iter()
                .filter(|other| other.before > series.before)
                .count();
            let after_rank = self
                .series
                .iter()
                .filter(|other| other.after > series.after)
                .count();
            let delta = before_rank as isize - after_rank as isize;
            let rank = if delta > 0 {
                format!("↑ {delta}")
            } else if delta < 0 {
                format!("↓ {}", -delta)
            } else {
                "=".into()
            };
            let focused = active == Some(i);
            let accent = i == 0;
            let narrow = width < 440.;
            let label_size = if narrow { 12. } else { 14. };
            let label_gap = if narrow { 6. } else { 8. };
            let opacity = if active.is_none_or(|v| v == i) {
                1.
            } else {
                0.3
            };
            let name_color = if focused || accent { p.ink } else { p.secondary };
            let enter = ((age - i as f32 * 0.05) / 0.3).clamp(0., 1.);
            labels.push(
                div()
                    .absolute()
                    .left(px(0.))
                    .top(px((left_y[i] - 10.).max(0.)))
                    .w(px((left - 10.).max(1.)))
                    .h(px(20.))
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap(px(label_gap))
                    .overflow_hidden()
                    .text_size(px(label_size))
                    .line_height(px(20.))
                    .opacity(opacity * enter)
                    .child(
                        div()
                            .min_w_0()
                            .overflow_hidden()
                            .text_ellipsis()
                            .text_color(name_color)
                            .child(series.label.clone()),
                    )
                    .child(
                        div()
                            .flex_none()
                            .text_color(p.ink)
                            .font_weight(if accent {
                                FontWeight::MEDIUM
                            } else {
                                FontWeight::NORMAL
                            })
                            .child(format!("{:.1}", values[i].0)),
                    ),
            );
            let delay = i as f32 * 0.05;
            let step = ((age - 0.7 - delay) / 0.3).clamp(0., 1.);
            labels.push(
                div()
                    .absolute()
                    .left(px(width - right + 10.))
                    .top(px((right_y[i] - 10.).max(0.)))
                    .h(px(20.))
                    .flex()
                    .items_center()
                    .gap(px(label_gap))
                    .text_size(px(label_size))
                    .line_height(px(20.))
                    .opacity(opacity * step)
                    .child(
                        div()
                            .text_color(p.ink)
                            .font_weight(if accent {
                                FontWeight::MEDIUM
                            } else {
                                FontWeight::NORMAL
                            })
                            .child(format!("{:.1}", values[i].1)),
                    )
                    .child(
                        div()
                            .min_w(px(30.))
                            .flex()
                            .items_center()
                            .gap(px(2.))
                            .text_size(px(12.))
                            .line_height(px(16.))
                            .text_color(p.faint)
                            .child(rank),
                    ),
            );
        }
        let axis_color = p.line;
        let muted_ink = p.ink.opacity(0.36);
        let accent_ink = p.bright;
        let full_ink = p.ink;
        let dot_ring = p.panel;
        let graph = div()
            .id("slope-chart")
            .relative()
            .role(Role::Figure)
            .aria_label({
                let detail = self
                    .active
                    .and_then(|i| self.series.get(i))
                    .map(|series| {
                        let before_rank = self
                            .series
                            .iter()
                            .filter(|other| other.before > series.before)
                            .count()
                            + 1;
                        let after_rank = self
                            .series
                            .iter()
                            .filter(|other| other.after > series.after)
                            .count()
                            + 1;
                        format!(
                            "{}: {:.1} → {:.1}, {} {} → {}",
                            series.label,
                            series.before,
                            series.after,
                            tr(cx, "rank", "排名"),
                            before_rank,
                            after_rank
                        )
                    })
                    .unwrap_or_default();
                format!(
                    "{}. {}. {}",
                    self.title,
                    detail,
                    tr(cx, "Use arrow keys to inspect series", "使用方向键查看数据")
                )
            })
            .tab_index(0)
            .relative()
            .w_full()
            .h(px(height))
            .on_hover(cx.listener(|s, h: &bool, _, cx| {
                if !h {
                    s.inspect(None, cx);
                }
            }))
            .on_mouse_move(cx.listener(move |s, e: &MouseMoveEvent, _, cx| {
                s.inspect(
                    nearest_series(s.bounds.get(), e.position, &points, lower, span),
                    cx,
                );
            }))
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                let n = s.series.len();
                if n == 0 {
                    return;
                }
                let i = s.active.unwrap_or(0);
                let next = match e.keystroke.key.as_str() {
                    "right" | "down" => Some((i + 1).min(n - 1)),
                    "left" | "up" => Some(i.saturating_sub(1)),
                    "home" => Some(0),
                    "end" => Some(n - 1),
                    "escape" => None,
                    _ => return,
                };
                w.prevent_default();
                cx.stop_propagation();
                s.inspect(next, cx);
            }))
            .child({
                let paint_left = left_y.clone();
                let paint_right = right_y.clone();
                canvas(
                    move |b, w, _| {
                        let previous = p_bounds.get();
                        if previous.size != b.size
                            || intersects_viewport(previous, w.viewport_size())
                                != intersects_viewport(b, w.viewport_size())
                        {
                            w.request_animation_frame();
                        }
                        p_bounds.set(b);
                    },
                    move |b, _, w, _| {
                        let width = f32::from(b.size.width);
                        let height = f32::from(b.size.height);
                        let (left_gutter, right_gutter) = gutters(width);
                        for x in [left_gutter, width - right_gutter] {
                            let mut axis = PathBuilder::stroke(px(1.));
                            axis.move_to(point(b.origin.x + px(x), b.origin.y));
                            axis.line_to(point(b.origin.x + px(x), b.origin.y + b.size.height));
                            if let Ok(path) = axis.build() {
                                w.paint_path(path, axis_color);
                            }
                        }
                        for (i, (a, z, _)) in values.iter().enumerate() {
                            let opacity = if active.is_none_or(|v| v == i) {
                                1.
                            } else {
                                0.14
                            };
                            let accent = i == 0;
                            let focused = active == Some(i);
                            let ink = if focused && !accent {
                                full_ink
                            } else if accent {
                                accent_ink
                            } else {
                                muted_ink
                            };
                            let at = |x: f32, value: f32| {
                                point(
                                    b.origin.x + px(x),
                                    b.origin.y
                                        + px(14. + (1. - (value - lower) / span) * (height - 28.)),
                                )
                            };
                            let left = at(left_gutter, *a);
                            let end = at(width - right_gutter, *z);
                            if (f32::from(left.y - b.origin.y) - paint_left[i]).abs() > 1. {
                                let mut leader = PathBuilder::stroke(px(1.));
                                leader.move_to(point(
                                    left.x - px(6.),
                                    b.origin.y + px(paint_left[i]),
                                ));
                                leader.line_to(point(left.x - px(1.), left.y));
                                if let Ok(path) = leader.build() {
                                    w.paint_path(path, axis_color.opacity(opacity));
                                }
                            }
                            if (f32::from(end.y - b.origin.y) - paint_right[i]).abs() > 1.
                                && ((age - (0.1 + i as f32 * 0.05)) / 0.7) >= 1.
                            {
                                let mut leader = PathBuilder::stroke(px(1.));
                                leader.move_to(point(end.x + px(1.), end.y));
                                leader.line_to(point(
                                    end.x + px(6.),
                                    b.origin.y + px(paint_right[i]),
                                ));
                                if let Ok(path) = leader.build() {
                                    w.paint_path(path, axis_color.opacity(opacity));
                                }
                            }
                            let delay = 0.1 + i as f32 * 0.05;
                            let progress = in_out(((age - delay) / 0.7).clamp(0., 1.));
                            let right = point(
                                left.x + (end.x - left.x) * progress,
                                left.y + (end.y - left.y) * progress,
                            );
                            let mut line = PathBuilder::stroke(px(2.));
                            line.move_to(left);
                            line.line_to(right);
                            if let Ok(path) = line.build() {
                                w.paint_path(path, ink.opacity(opacity));
                            }
                            for pt in [left, right] {
                                w.paint_quad(
                                    fill(
                                        Bounds {
                                            origin: point(pt.x - px(5.), pt.y - px(5.)),
                                            size: size(px(10.), px(10.)),
                                        },
                                        dot_ring,
                                    )
                                    .corner_radii(px(5.)),
                                );
                                w.paint_quad(
                                    fill(
                                        Bounds {
                                            origin: point(pt.x - px(3.), pt.y - px(3.)),
                                            size: size(px(6.), px(6.)),
                                        },
                                        ink.opacity(opacity),
                                    )
                                    .corner_radii(px(3.)),
                                );
                            }
                        }
                    },
                )
                .size_full()
            })
            .children(labels)
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
        let tip = self.active.and_then(|index| {
            let series = self.series.get(index)?;
            let before_rank = self
                .series
                .iter()
                .filter(|other| other.before > series.before)
                .count()
                + 1;
            let after_rank = self
                .series
                .iter()
                .filter(|other| other.after > series.after)
                .count()
                + 1;
            let change = series.after - series.before;
            Some((
                series.label.clone(),
                series.before,
                series.after,
                change,
                before_rank,
                after_rank,
                (left_y.get(index).copied().unwrap_or(0.)
                    + right_y.get(index).copied().unwrap_or(0.))
                    * 0.5,
            ))
        });
        let before = tr(cx, "Before", "之前");
        let after = tr(cx, "After", "之后");
        div()
            .relative()
            .w(px(560.))
            .max_w_full()
            .flex()
            .flex_col()
            .text_color(p.ink)
            .child(
                div()
                    .relative()
                    .h(px(28.))
                    .child(
                        div()
                            .absolute()
                            .left(px((left - 40.).max(0.)))
                            .top(px(4.))
                            .w(px(80.))
                            .text_center()
                            .text_size(px(14.))
                            .line_height(px(20.))
                            .text_color(p.faint)
                            .child(before),
                    )
                    .child(
                        div()
                            .absolute()
                            .left(px((width - right - 40.).max(0.)))
                            .top(px(4.))
                            .w(px(80.))
                            .text_center()
                            .text_size(px(14.))
                            .line_height(px(20.))
                            .text_color(p.faint)
                            .child(after),
                    ),
            )
            .child(graph)
            .when_some(tip, |view, (label, start, end, change, from, to, mid)| {
                let tip_top = (28. + mid - 78.).max(0.);
                view.child(
                    div()
                        .absolute()
                        .left(px(((left + (width - right)) * 0.5 - 84.).max(8.)))
                        .top(px(tip_top))
                        .min_w(px(168.))
                        .max_w(px(240.))
                        .px(px(12.))
                        .py(px(10.))
                        .rounded(px(8.))
                        .bg(p.panel)
                        .border_1()
                        .border_color(p.line)
                        .shadow_lg()
                        .flex()
                        .flex_col()
                        .gap(px(3.))
                        .child(
                            div()
                                .mb(px(2.))
                                .text_size(px(14.))
                                .line_height(px(20.))
                                .font_weight(FontWeight::MEDIUM)
                                .child(label),
                        )
                        .child(
                            div()
                                .flex()
                                .justify_between()
                                .gap(px(12.))
                                .text_size(px(14.))
                                .line_height(px(18.))
                                .child(div().text_color(p.secondary).child(before))
                                .child(
                                    div()
                                        .font_weight(FontWeight::MEDIUM)
                                        .child(format!("{start:.1}")),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .justify_between()
                                .gap(px(12.))
                                .text_size(px(14.))
                                .line_height(px(18.))
                                .child(div().text_color(p.secondary).child(after))
                                .child(
                                    div()
                                        .font_weight(FontWeight::MEDIUM)
                                        .child(format!("{end:.1}")),
                                ),
                        )
                        .child(
                            div()
                                .mt(px(2.))
                                .text_size(px(12.))
                                .line_height(px(20.))
                                .text_color(p.faint)
                                .child(format!(
                                    "{}{change:.1}, {} {from} → {to}",
                                    if change >= 0. { "+" } else { "" },
                                    tr(cx, "rank", "排名")
                                )),
                        ),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::spread_labels;
    #[test]
    fn overlapping_labels_are_spread_and_clamped() {
        let result = spread_labels(&[50., 50., 51.], 200.);
        assert!(result[0] >= 8. && result[2] <= 192.);
        assert!(result[1] - result[0] >= 20.);
        assert!(result[2] - result[1] >= 20.);
        assert!(spread_labels(&[], 200.).is_empty());
    }
}
