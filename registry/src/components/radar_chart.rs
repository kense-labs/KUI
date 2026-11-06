//! An interactive radial comparison with animated, host-supplied profiles.
use crate::primitives::{
    family::{Paint, Slide, Tone, disabled_motion, face},
    presentation::{column, hint, row, title, tr},
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
/// Radial comparison. Three to twelve dimensions are supported.
pub struct RadarChart {
    title: SharedString,
    axes: Vec<RadarAxis>,
    profiles: Vec<RadarProfile>,
    visual: Vec<VisualProfile>,
    active: Option<usize>,
    bounds: Rc<Cell<Bounds<Pixels>>>,
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
            axes,
            profiles,
            visual: vec![],
            active: None,
            bounds: Default::default(),
            initialized: false,
            reveal: Slide::new(0.),
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
                    slide.duration(p.value(i), 0.6, cx);
                }
                state
            })
            .collect();
        self.profiles = profiles;
        self.initialized = true;
        cx.notify();
    }
    fn select(&mut self, next: Option<usize>, cx: &mut Context<Self>) {
        if self.active == next {
            return;
        }
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
fn radar_metrics(width: f32) -> (f32, f32, f32) {
    let width = if width > 1. { width } else { 320. };
    let gutter = if width < 360. { 46. } else { 64. };
    let radius = (320_f32.min((width - gutter * 2.).max(80.)) / 2.).max(40.);
    let stage = radius * 2. + gutter * 2.;
    (gutter, radius, stage)
}
fn axis_at(b: Bounds<Pixels>, pos: Point<Pixels>, count: usize) -> Option<usize> {
    if count < 3 {
        return None;
    }
    let dx = f32::from(pos.x - b.origin.x) - f32::from(b.size.width) / 2.;
    let dy = f32::from(pos.y - b.origin.y) - f32::from(b.size.height) / 2.;
    let (_, radius, _) = radar_metrics(f32::from(b.size.width));
    if dx.hypot(dy) > radius + 25. || dx.hypot(dy) < 8. {
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
        if on_screen && moving {
            w.request_animation_frame();
        }
        let width = if measured.size.width > px(0.) {
            f32::from(measured.size.width)
        } else {
            320.
        };
        let (gutter, radius, stage) = radar_metrics(width);
        let labels: Vec<_> = self
            .axes
            .iter()
            .enumerate()
            .map(|(i, axis)| {
                let angle = i as f32 * TAU / n.max(1) as f32;
                let sin = angle.sin();
                let centered = sin.abs() < 0.2;
                let label_width = if centered { 120. } else { gutter + 8. };
                let x = width / 2. + sin * (radius + 10.);
                let y = stage / 2. - angle.cos() * (radius + 12.);
                let left = if centered {
                    x - label_width / 2.
                } else if sin > 0. {
                    x
                } else {
                    x - label_width
                };
                div()
                    .absolute()
                    .left(px(left.clamp(0., (width - label_width).max(0.))))
                    .top(px((y - 8.).max(0.)))
                    .w(px(label_width))
                    .text_center()
                    .text_size(px(12.))
                    .line_height(px(15.))
                    .text_color(if self.active == Some(i) {
                        p.ink
                    } else {
                        p.secondary
                    })
                    .opacity(if self.active.is_some() && self.active != Some(i) {
                        0.55
                    } else {
                        1.
                    })
                    .overflow_hidden()
                    .child(axis.label.clone())
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
        let mut graph = div()
            .id("radar-chart")
            .role(Role::Figure)
            .aria_label({
                let instructions = tr(
                    cx,
                    "Use arrow keys to inspect dimensions",
                    "使用方向键查看各维度",
                );
                let current = self.active.or((!self.axes.is_empty()).then_some(0));
                if let Some((i, axis)) =
                    current.and_then(|i| self.axes.get(i).map(|axis| (i, axis)))
                {
                    let values = self
                        .profiles
                        .iter()
                        .map(|profile| format!("{} {:.1}", profile.label, profile.value(i)))
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!(
                        "{}. {}: {}. {}",
                        self.title, axis.label, values, instructions
                    )
                } else {
                    format!("{}. {}", self.title, instructions)
                }
            })
            .tab_index(0)
            .relative()
            .w_full()
            .h(px(stage))
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
                        let height = f32::from(b.size.height);
                        let (_, chart_radius, _) = radar_metrics(width);
                        let radius = chart_radius * reveal;
                        let at = |i: usize, r: f32| {
                            let a = i as f32 * TAU / n as f32;
                            point(
                                b.origin.x + px(width / 2. + a.sin() * r),
                                b.origin.y + px(height / 2. - a.cos() * r),
                            )
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
                                w.paint_path(path, grid);
                            }
                        }
                        for (series_index, (values, color)) in visual.iter().enumerate() {
                            let pt = |i: usize| {
                                at(i, radius * values.get(i).copied().unwrap_or(0.) / 100.)
                            };
                            let mut area = PathBuilder::fill();
                            area.move_to(pt(0));
                            for i in 1..n {
                                area.line_to(pt(i));
                            }
                            area.close();
                            if let Ok(path) = area.build() {
                                w.paint_path(
                                    path,
                                    color.opacity(if series_index == 0 { 0.12 } else { 0.06 }),
                                );
                            }
                            let mut line = PathBuilder::stroke(px(2.));
                            line.move_to(pt(0));
                            for i in 1..n {
                                line.line_to(pt(i));
                            }
                            line.close();
                            if let Ok(path) = line.build() {
                                w.paint_path(path, *color);
                            }
                            if active.is_some() {
                                for i in 0..n {
                                    let center = pt(i);
                                    w.paint_quad(
                                        fill(
                                            Bounds {
                                                origin: point(
                                                    center.x - px(5.5),
                                                    center.y - px(5.5),
                                                ),
                                                size: size(px(11.), px(11.)),
                                            },
                                            panel,
                                        )
                                        .corner_radii(px(5.5)),
                                    );
                                    w.paint_quad(
                                        fill(
                                            Bounds {
                                                origin: point(
                                                    center.x - px(3.5),
                                                    center.y - px(3.5),
                                                ),
                                                size: size(px(7.), px(7.)),
                                            },
                                            *color,
                                        )
                                        .corner_radii(px(3.5)),
                                    );
                                }
                            }
                        }
                        if let Some(i) = active {
                            let mut path = PathBuilder::stroke(px(1.));
                            path.move_to(at(i, 0.));
                            path.line_to(at(i, radius));
                            if let Ok(path) = path.build() {
                                w.paint_path(path, ink.opacity(0.45));
                            }
                        }
                    },
                )
                .size_full(),
            )
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
        if let Some(index) = self.active
            && let Some(axis) = self.axes.get(index)
        {
            let angle = index as f32 * TAU / n.max(1) as f32;
            let x = width / 2. + angle.sin() * radius * 0.45;
            let y = stage / 2. - angle.cos() * radius * 0.45;
            let mut tip = div()
                .absolute()
                .left(px(x.clamp(8., (width - 176.).max(8.))))
                .top(px((y - 28.).max(8.)))
                .min_w(px(164.))
                .max_w(px(240.))
                .px(px(12.))
                .py(px(10.))
                .flex()
                .flex_col()
                .gap(px(4.))
                .rounded(crate::foundation::surface_radius(cx))
                .border_1()
                .border_color(p.line)
                .bg(p.panel)
                .shadow_lg()
                .child(
                    div()
                        .mb(px(2.))
                        .text_size(px(12.))
                        .line_height(px(20.))
                        .text_color(p.secondary)
                        .child(axis.label.clone()),
                );
            for series in &self.profiles {
                tip = tip.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(div().size(px(8.)).rounded_full().bg(series.color))
                        .child(
                            div()
                                .flex_1()
                                .text_size(px(14.))
                                .line_height(px(18.))
                                .child(series.label.clone()),
                        )
                        .child(
                            div()
                                .min_w(px(31.))
                                .text_size(px(12.))
                                .line_height(px(16.))
                                .text_color(p.faint)
                                .child(format!("{:.0}", series.value(index))),
                        ),
                );
            }
            graph = graph.child(tip);
        }
        let detail = self
            .active
            .and_then(|i| {
                self.axes.get(i).map(|a| {
                    let values = self
                        .profiles
                        .iter()
                        .map(|profile| format!("{} {:.0}", profile.label, profile.value(i)))
                        .collect::<Vec<_>>()
                        .join("  ·  ");
                    format!("{}: {}", a.label, values)
                })
            })
            .unwrap_or_else(|| tr(cx, "Select a dimension", "选择一个维度").to_string());
        column()
            .w(px(496.))
            .max_w_full()
            .gap_3()
            .p_4()
            .rounded(crate::foundation::surface_radius(cx))
            .bg(p.panel)
            .border_1()
            .border_color(p.line)
            .text_color(p.ink)
            .child(title(self.title.clone(), p))
            .child(graph)
            .child(hint(detail, p))
            .child(
                row()
                    .flex_wrap()
                    .gap_2()
                    .children(self.axes.iter().enumerate().map(|(i, a)| {
                        face(
                            ("radar-axis", i),
                            a.label.clone(),
                            32.,
                            crate::foundation::surface_radius(cx).into(),
                            Tone::Clear,
                            cx,
                        )
                        .h(px(32.))
                        .px(px(10.))
                        .text_size(px(14.))
                        .line_height(px(20.))
                        .text_color(if self.active == Some(i) {
                            p.ink
                        } else {
                            p.secondary
                        })
                        .opacity(if self.active.is_some() && self.active != Some(i) {
                            0.55
                        } else {
                            1.
                        })
                        .hover(|s| s.text_color(p.ink))
                        .child(a.label.clone())
                        .on_click(cx.listener(move |s, _, _, cx| s.select(Some(i), cx)))
                        .on_key_down(cx.listener(
                            move |s, e: &KeyDownEvent, w, cx| {
                                if matches!(e.keystroke.key.as_str(), "enter" | "space")
                                    && !e.is_held
                                {
                                    s.select(Some(i), cx);
                                    w.prevent_default();
                                    cx.stop_propagation();
                                }
                            },
                        ))
                    })),
            )
            .child(row().flex_wrap().gap_x(px(16.)).gap_y(px(6.)).children(
                self.profiles.iter().enumerate().map(|(i, series)| {
                    div()
                        .id(("radar-series", i))
                        .h(px(32.))
                        .px(px(10.))
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .text_size(px(14.))
                        .line_height(px(20.))
                        .text_color(p.secondary)
                        .hover(|s| s.text_color(p.ink))
                        .child(div().size(px(8.)).rounded_full().bg(series.color))
                        .child(series.label.clone())
                }),
            ))
    }
}

#[cfg(test)]
mod tests {
    use super::axis_at;
    use gpui::{Bounds, point, px, size};
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
}
