//! A time-series overview with a draggable selection window.
use crate::primitives::{
    family::{Paint, Slide, Tone, face},
    presentation::{column, hint, row, title, tr},
};
use gpui::{prelude::*, *};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
#[derive(Clone, Debug)]
pub struct BrushPoint {
    pub label: SharedString,
    pub value: f32,
    pub event: Option<SharedString>,
}
impl BrushPoint {
    pub fn new(label: impl Into<SharedString>, value: f32) -> Self {
        Self {
            label: label.into(),
            value,
            event: None,
        }
    }
    pub fn with_event(mut self, event: impl Into<SharedString>) -> Self {
        self.event = Some(event.into());
        self
    }
    fn safe(&self) -> f32 {
        if self.value.is_finite() {
            self.value
        } else {
            0.
        }
    }
}
#[derive(Clone, Debug)]
pub struct BrushChange {
    pub start: usize,
    pub end: usize,
}
#[derive(Clone, Debug)]
pub struct BrushInspect {
    pub index: usize,
    pub label: SharedString,
    pub value: f32,
    pub event: Option<SharedString>,
}
/// A shortcut selecting the latest `points` samples; `None` selects all.
#[derive(Clone, Debug)]
pub struct BrushPreset {
    pub label: SharedString,
    pub points: Option<usize>,
}
impl BrushPreset {
    pub fn new(label: impl Into<SharedString>, points: Option<usize>) -> Self {
        Self {
            label: label.into(),
            points,
        }
    }
}
#[derive(Clone, Copy, PartialEq)]
enum Drag {
    Start,
    End,
    Window,
    New,
}
struct CachedOverview {
    bounds: Bounds<Pixels>,
    revision: u64,
    path: Option<Path<Pixels>>,
}
pub struct BrushChart {
    title: SharedString,
    points: Vec<BrushPoint>,
    values: Rc<Vec<f32>>,
    events: Rc<Vec<Option<SharedString>>>,
    date_labels: Rc<Vec<SharedString>>,
    revision: u64,
    overview_cache: Rc<RefCell<Option<CachedOverview>>>,
    start: usize,
    end: usize,
    left: Slide,
    right: Slide,
    axis_low: Slide,
    axis_high: Slide,
    axis_initialized: bool,
    reveal: Slide,
    revealed: bool,
    selected: Option<usize>,
    grip_hover: Option<usize>,
    drag: Option<Drag>,
    grab: f32,
    origin: (usize, usize),
    bounds: Rc<Cell<Bounds<Pixels>>>,
    overview: Rc<Cell<Bounds<Pixels>>>,
    touching: Rc<Cell<bool>>,
    presets: Vec<BrushPreset>,
}
impl EventEmitter<BrushChange> for BrushChart {}
impl EventEmitter<BrushInspect> for BrushChart {}
impl BrushChart {
    pub fn new(title: impl Into<SharedString>, points: Vec<BrushPoint>) -> Self {
        let end = points.len().saturating_sub(1);
        let start = end.saturating_sub(14);
        let values = Rc::new(points.iter().map(BrushPoint::safe).collect());
        let events = Rc::new(points.iter().map(|p| p.event.clone()).collect());
        let date_labels = Rc::new(points.iter().map(|p| p.label.clone()).collect());
        Self {
            title: title.into(),
            points,
            values,
            events,
            date_labels,
            revision: 0,
            overview_cache: Default::default(),
            start,
            end,
            left: Slide::new(start as f32),
            right: Slide::new(end as f32),
            axis_low: Slide::new(0.),
            axis_high: Slide::new(1.),
            axis_initialized: false,
            reveal: Slide::new(0.),
            revealed: false,
            selected: None,
            grip_hover: None,
            drag: None,
            grab: 0.,
            origin: (start, end),
            bounds: Default::default(),
            overview: Default::default(),
            touching: Default::default(),
            presets: vec![],
        }
    }
    pub fn set_window(&mut self, start: usize, end: usize, cx: &mut Context<Self>) {
        self.update(start, end, true, cx)
    }
    pub fn with_presets(mut self, presets: Vec<BrushPreset>) -> Self {
        self.presets = presets;
        self
    }
    pub fn set_points(&mut self, points: Vec<BrushPoint>, cx: &mut Context<Self>) {
        self.values = Rc::new(points.iter().map(BrushPoint::safe).collect());
        self.events = Rc::new(points.iter().map(|p| p.event.clone()).collect());
        self.date_labels = Rc::new(points.iter().map(|p| p.label.clone()).collect());
        self.revision = self.revision.wrapping_add(1);
        self.points = points;
        let last = self.points.len().saturating_sub(1);
        self.update(self.start.min(last), self.end.min(last), true, cx);
    }
    fn update(&mut self, start: usize, end: usize, animate: bool, cx: &mut Context<Self>) {
        let last = self.points.len().saturating_sub(1);
        let end = end.min(last);
        let start = start.min(end);
        let changed = (self.start, self.end) != (start, end);
        self.start = start;
        self.end = end;
        if animate {
            self.left.duration(start as f32, 0.42, cx);
            self.right.duration(end as f32, 0.42, cx);
        } else {
            self.left.reset(start as f32);
            self.right.reset(end as f32);
        }
        self.selected = self.selected.filter(|&i| i >= start && i <= end);
        if changed {
            cx.emit(BrushChange { start, end });
        }
        cx.notify();
    }
    fn select(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(p) = self.points.get(index) else {
            return;
        };
        if self.selected == Some(index) {
            return;
        }
        self.selected = Some(index);
        cx.emit(BrushInspect {
            index,
            label: p.label.clone(),
            value: p.safe(),
            event: p.event.clone(),
        });
        cx.notify();
    }
    fn fraction(&self, x: Pixels) -> f32 {
        let b = self.overview.get();
        (f32::from(x - b.origin.x) / f32::from(b.size.width).max(1.)).clamp(0., 1.)
    }
    fn begin(&mut self, x: Pixels, cx: &mut Context<Self>) {
        if self.points.len() < 2 {
            return;
        }
        let t = self.fraction(x);
        let count = self.points.len().saturating_sub(1) as f32;
        let left = self.start as f32 / count;
        let right = self.end as f32 / count;
        let hit = 14. / f32::from(self.overview.get().size.width).max(1.);
        self.drag = Some(if (t - left).abs() <= hit {
            Drag::Start
        } else if (t - right).abs() <= hit {
            Drag::End
        } else if t >= left && t <= right {
            Drag::Window
        } else {
            Drag::New
        });
        self.grab = t;
        self.origin = (self.start, self.end);
        if self.drag == Some(Drag::New) {
            let index = (t * count).round() as usize;
            self.update(index, index, false, cx);
        }
        cx.notify();
    }
    fn drag_to(&mut self, x: Pixels, cx: &mut Context<Self>) {
        let Some(drag) = self.drag else { return };
        let last = self.points.len().saturating_sub(1);
        let t = self.fraction(x);
        let index = (t * last as f32).round() as usize;
        match drag {
            Drag::Start => self.update(index.min(self.end), self.end, false, cx),
            Drag::End => self.update(self.start, index.max(self.start), false, cx),
            Drag::Window => {
                let width = self.origin.1 - self.origin.0;
                let delta = ((t - self.grab) * last as f32).round() as isize;
                let start = (self.origin.0 as isize + delta)
                    .clamp(0, last.saturating_sub(width) as isize)
                    as usize;
                self.update(start, start + width, false, cx)
            }
            Drag::New => {
                let anchor = (self.grab * last as f32).round() as usize;
                self.update(anchor.min(index), anchor.max(index), false, cx)
            }
        }
    }
}
/// Keep each pixel column's extreme samples in chronological order.
fn sample_indices(values: &[f32], start: usize, end: usize, columns: usize) -> Vec<usize> {
    let count = end - start + 1;
    let bucket = count.div_ceil(columns.max(1));
    if bucket <= 1 {
        return (start..=end).collect();
    }
    let mut out = vec![start];
    let mut from = start;
    while from <= end {
        let to = (from + bucket).min(end + 1);
        let mut min = from;
        let mut max = from;
        for i in from..to {
            if values[i] < values[min] {
                min = i;
            }
            if values[i] > values[max] {
                max = i;
            }
        }
        for i in [min.min(max), min.max(max)] {
            if out.last().copied() != Some(i) {
                out.push(i);
            }
        }
        from = to;
    }
    if out.last().copied() != Some(end) {
        out.push(end);
    }
    out
}
fn moving_average(values: &[f32], i: usize, start: usize, end: usize) -> f32 {
    let first = i.saturating_sub(3).max(start);
    let last = (i + 3).min(end);
    values[first..=last].iter().sum::<f32>() / (last - first + 1) as f32
}
impl Render for BrushChart {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let measured = self.bounds.get();
        if !self.revealed
            && measured.size.width > px(0.)
            && measured.origin.y < w.viewport_size().height
            && measured.origin.y + measured.size.height > px(0.)
            && w.is_visible()
        {
            self.revealed = true;
            self.reveal.duration(1., 0.9, cx);
        }
        let reveal = self.reveal.get(w, cx).clamp(0., 1.);
        let data = self.values.clone();
        let n = data.len();
        let start = self.start.min(n.saturating_sub(1));
        let end = self.end.min(n.saturating_sub(1)).max(start);
        let (target_low, target_high) = if data.is_empty() {
            (0., 1.)
        } else {
            let visible = &data[start..=end];
            let low = visible.iter().fold(0f32, |a, &b| a.min(b));
            let high = visible.iter().fold(1f32, |a, &b| a.max(b));
            (low, high)
        };
        if !self.axis_initialized {
            self.axis_initialized = true;
            self.axis_low.reset(target_low);
            self.axis_high.reset(target_high);
        } else {
            self.axis_low.duration(target_low, 0.45, cx);
            self.axis_high.duration(target_high, 0.45, cx);
        }
        let low = self.axis_low.get(w, cx);
        let high = self.axis_high.get(w, cx);
        let left = self.left.get(w, cx);
        let right = self.right.get(w, cx);
        let active = self.selected;
        let plot_bounds = self.bounds.clone();
        let values = data.clone();
        let events = self.events.clone();
        let date_labels = self.date_labels.clone();
        let plot = div()
            .id("brush-plot")
            .role(Role::Figure)
            .tab_index(0)
            .w_full()
            .h(px(240.))
            .aria_label(format!(
                "{}. {}",
                self.title,
                tr(cx, "Use arrow keys to inspect values", "使用方向键查看数据")
            ))
            .on_mouse_move(cx.listener(|s, e: &MouseMoveEvent, _, cx| {
                if s.points.is_empty() {
                    return;
                }
                let b = s.bounds.get();
                let t = (f32::from(e.position.x - b.origin.x)
                    / (f32::from(b.size.width) - 42.).max(1.))
                .clamp(0., 1.);
                let i = s.start + (t * (s.end - s.start) as f32).round() as usize;
                s.select(i, cx)
            }))
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, _, cx| {
                if s.points.is_empty() {
                    return;
                }
                let i = s.selected.unwrap_or(s.start);
                let next = match e.keystroke.key.as_str() {
                    "right" => (i + 1).min(s.end),
                    "left" => i.saturating_sub(1).max(s.start),
                    "home" => s.start,
                    "end" => s.end,
                    _ => return,
                };
                cx.stop_propagation();
                s.select(next, cx)
            }))
            .child(
                canvas(
                    move |b, w, _| {
                        if plot_bounds.get().size.width != b.size.width {
                            w.request_animation_frame();
                        }
                        plot_bounds.set(b);
                    },
                    move |b, _, w, cx| {
                        if values.is_empty() {
                            return;
                        }
                        let span = (high - low).max(1.);
                        let width = f32::from(b.size.width);
                        let height = f32::from(b.size.height);
                        let plot_width = width.max(1.);
                        let y_of = |v: f32| 28. + (1. - (v - low) / span) * (height - 28.);
                        let at = |i: usize, v: f32| {
                            point(
                                b.origin.x
                                    + px(if end > start {
                                        plot_width * (i - start) as f32 / (end - start) as f32
                                    } else {
                                        0.
                                    }),
                                b.origin.y + px(y_of(v)),
                            )
                        };
                        for tick in 0..=3 {
                            let value = low + span * tick as f32 / 3.;
                            let y = y_of(value);
                            let mut grid = PathBuilder::stroke(px(1.));
                            grid.move_to(point(b.origin.x, b.origin.y + px(y)));
                            grid.line_to(point(b.origin.x + px(plot_width), b.origin.y + px(y)));
                            if let Ok(path) = grid.build() {
                                w.paint_path(path, p.line);
                            }
                            let label: SharedString = format!("{value:.0}").into();
                            let mut run = w.text_style().to_run(label.len());
                            run.color = p.secondary;
                            let text = w.text_system().shape_line(label, px(10.), &[run], None);
                            let _ = text.paint(
                                point(b.origin.x + px(10.), b.origin.y + px(y - 7.)),
                                px(14.),
                                TextAlign::Left,
                                None,
                                w,
                                cx,
                            );
                        }
                        let reached = start as f32 + (end - start) as f32 * reveal;
                        let indices = sample_indices(&values, start, end, plot_width as usize);
                        let mix = (((end - start + 1) as f32 / plot_width) - 1.).clamp(0., 1.);
                        let display_value = |i| {
                            values[i] * (1. - mix) + moving_average(&values, i, start, end) * mix
                        };
                        let mut area = PathBuilder::fill();
                        area.move_to(at(start, low));
                        area.line_to(at(start, display_value(start)));
                        let mut previous = start;
                        let mut last_x = at(start, low).x;
                        for &i in indices.iter().skip(1) {
                            if i as f32 <= reached {
                                let pt = at(i, display_value(i));
                                area.line_to(pt);
                                last_x = pt.x;
                                previous = i;
                            } else {
                                let f = ((reached - previous as f32)
                                    / (i - previous).max(1) as f32)
                                    .clamp(0., 1.);
                                let a = at(previous, display_value(previous));
                                let z = at(i, display_value(i));
                                let pt = point(a.x + (z.x - a.x) * f, a.y + (z.y - a.y) * f);
                                area.line_to(pt);
                                last_x = pt.x;
                                break;
                            }
                        }
                        area.line_to(point(last_x, at(start, low).y));
                        area.close();
                        if let Ok(path) = area.build() {
                            w.paint_path(path, p.bright.opacity(0.10));
                        }
                        let make_path = |smooth: bool| {
                            let value = |i| {
                                if smooth {
                                    moving_average(&values, i, start, end)
                                } else {
                                    values[i]
                                }
                            };
                            let mut path = PathBuilder::stroke(px(if smooth { 2. } else { 1.5 }));
                            let mut previous = indices[0];
                            path.move_to(at(previous, value(previous)));
                            for &i in indices.iter().skip(1) {
                                if i as f32 <= reached {
                                    path.line_to(at(i, value(i)));
                                    previous = i;
                                } else {
                                    let f = ((reached - previous as f32)
                                        / (i - previous).max(1) as f32)
                                        .clamp(0., 1.);
                                    let a = at(previous, value(previous));
                                    let z = at(i, value(i));
                                    path.line_to(point(
                                        a.x + (z.x - a.x) * f,
                                        a.y + (z.y - a.y) * f,
                                    ));
                                    break;
                                }
                            }
                            path.build()
                        };
                        if let Ok(path) = make_path(false) {
                            w.paint_path(path, p.bright.opacity(1. - mix * 0.7));
                        }
                        if mix > 0.
                            && let Ok(path) = make_path(true)
                        {
                            w.paint_path(path, p.bright.opacity(mix));
                        }
                        let mut last_label_right = px(-8.);
                        for i in start..=end {
                            let Some(label) = events.get(i).and_then(|v| v.clone()) else {
                                continue;
                            };
                            if i as f32 > reached {
                                continue;
                            }
                            let pt = at(i, values[i]);
                            w.paint_quad(
                                fill(
                                    Bounds {
                                        origin: point(pt.x - px(3.), pt.y - px(3.)),
                                        size: size(px(6.), px(6.)),
                                    },
                                    p.success,
                                )
                                .corner_radii(px(3.)),
                            );
                            let mut style = w.text_style();
                            style.font_weight = FontWeight::MEDIUM;
                            let mut run = style.to_run(label.len());
                            run.color = p.secondary;
                            let text = w.text_system().shape_line(label, px(11.), &[run], None);
                            let x = px(f32::from(pt.x - text.width / 2.)
                                .max(f32::from(b.origin.x))
                                .min(f32::from(b.origin.x + px(plot_width) - text.width)));
                            if x >= last_label_right + px(8.) && text.width <= b.size.width {
                                let _ = text.paint(
                                    point(x, b.origin.y),
                                    px(14.),
                                    TextAlign::Left,
                                    None,
                                    w,
                                    cx,
                                );
                                last_label_right = x + text.width;
                            }
                        }
                        let ticks = if width < 360. { 2 } else { 4 };
                        for tick in 0..=ticks {
                            let index = start + (end - start) * tick / ticks;
                            let label = date_labels[index].clone();
                            let mut run = w.text_style().to_run(label.len());
                            run.color = p.secondary;
                            let text = w.text_system().shape_line(label, px(10.), &[run], None);
                            let x = plot_width * tick as f32 / ticks as f32;
                            let left = (x - f32::from(text.width) / 2.)
                                .clamp(0., (plot_width - f32::from(text.width)).max(0.));
                            let _ = text.paint(
                                point(b.origin.x + px(left), b.origin.y + px(height - 16.)),
                                px(14.),
                                TextAlign::Left,
                                None,
                                w,
                                cx,
                            );
                        }
                        if let Some(i) = active.filter(|&i| i >= start && i <= end) {
                            let pt = at(i, values[i]);
                            let mut line = PathBuilder::stroke(px(1.));
                            line.move_to(point(pt.x, b.origin.y));
                            line.line_to(point(pt.x, b.origin.y + px(height)));
                            if let Ok(path) = line.build() {
                                w.paint_path(path, p.ink.opacity(0.45));
                            }
                            w.paint_quad(quad(
                                Bounds {
                                    origin: point(pt.x - px(5.), pt.y - px(5.)),
                                    size: size(px(10.), px(10.)),
                                },
                                Corners::all(px(5.)),
                                p.bright,
                                Edges::all(px(2.)),
                                p.panel,
                                BorderStyle::Solid,
                            ));
                        }
                    },
                )
                .size_full(),
            );
        let plot = if let Some(i) = active {
            let point = self.points.get(i);
            let plot_w = f32::from(self.bounds.get().size.width).max(1.);
            let x = if end > start {
                plot_w * (i.saturating_sub(start)) as f32 / (end - start) as f32
            } else {
                0.
            };
            let span = (high - low).max(1.);
            let y = point.map_or(28., |point| {
                28. + (1. - (point.value - low) / span) * (240. - 28.)
            });
            let left = if x + 14. + 132. > plot_w {
                (x - 14. - 132.).max(0.)
            } else {
                x + 14.
            };
            let top = (y + 14.).clamp(0., 160.);
            plot.relative().when_some(point, |plot, point| {
                let mut tip = div()
                    .absolute()
                    .left(px(left))
                    .top(px(top))
                    .min_w(px(132.))
                    .max_w(px(220.))
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
                            .mb(px(2.))
                            .text_size(px(12.))
                            .line_height(px(20.))
                            .text_color(p.secondary)
                            .child(point.label.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(16.))
                            .line_height(px(22.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(format!("{:.0}", point.value)),
                    );
                if let Some(event) = &point.event {
                    tip = tip.child(
                        div()
                            .mt(px(4.))
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .text_size(px(12.))
                            .line_height(px(20.))
                            .text_color(p.ink)
                            .child(div().size(px(7.)).rounded_full().bg(p.secondary))
                            .child(event.clone()),
                    );
                }
                plot.child(tip)
            })
        } else {
            plot
        };
        let overview = self.overview.clone();
        let cache = self.overview_cache.clone();
        let revision = self.revision;
        let values = data.clone();
        let last = n.saturating_sub(1) as f32;
        let l = if last > 0. { left / last } else { 0. };
        let r = if last > 0. { right / last } else { 1. };
        let overview_width = f32::from(self.overview.get().size.width);
        let left_x = l * overview_width;
        let right_x = r * overview_width;
        let touching = self.touching.clone();
        let touch_begin = cx.listener(|s, e: &TouchDragEvent, _, cx| s.begin(e.position.x, cx));
        let touch_move = cx.listener(|s, e: &TouchDragEvent, _, cx| s.drag_to(e.position.x, cx));
        let touch_end = cx.listener(|s, _: &TouchDragEvent, _, cx| {
            s.drag = None;
            cx.notify();
        });
        let strip = div()
            .id("brush-window")
            .role(Role::Slider)
            .tab_index(0)
            .aria_label(tr(cx, "Selected range", "所选范围"))
            .aria_value(format!(
                "{} – {}",
                self.points.get(start).map_or("", |p| p.label.as_ref()),
                self.points.get(end).map_or("", |p| p.label.as_ref())
            ))
            .relative()
            .w_full()
            .mt(px(8.))
            .h(px(52.))
            .rounded(crate::foundation::surface_radius(cx))
            .overflow_hidden()
            .cursor_crosshair()
            .bg(p.ink.opacity(0.025))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|s, e: &MouseDownEvent, _, cx| {
                    if e.click_count >= 2 {
                        s.update(0, s.points.len().saturating_sub(1), true, cx);
                    } else {
                        s.begin(e.position.x, cx);
                    }
                }),
            )
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                let last = s.points.len().saturating_sub(1);
                let width = s.end.saturating_sub(s.start);
                let step = if e.keystroke.modifiers.shift {
                    (width / 2).max(1)
                } else {
                    (width / 10).max(1)
                };
                let (start, end) = match e.keystroke.key.as_str() {
                    "left" => {
                        let a = s.start.saturating_sub(step);
                        (a, a + width)
                    }
                    "right" => {
                        let a = (s.start + step).min(last.saturating_sub(width));
                        (a, a + width)
                    }
                    "home" => (0, width),
                    "end" => (last.saturating_sub(width), last),
                    "escape" | "0" => (0, last),
                    "+" | "=" => {
                        let inset = (width / 10).max(1).min(width / 2);
                        (s.start + inset, s.end - inset)
                    }
                    "-" => {
                        let grow = (width / 10).max(1);
                        (s.start.saturating_sub(grow), (s.end + grow).min(last))
                    }
                    _ => return,
                };
                s.update(start, end, true, cx);
                cx.stop_propagation();
                w.prevent_default();
            }))
            .on_mouse_move(cx.listener(|s, e: &MouseMoveEvent, _, cx| {
                if s.drag.is_some() {
                    s.drag_to(e.position.x, cx)
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|s, _, _, cx| {
                    s.drag = None;
                    cx.notify()
                }),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|s, _, _, cx| {
                    s.drag = None;
                    cx.notify()
                }),
            )
            .child(
                canvas(
                    move |b, w, _| {
                        if overview.get().size.width != b.size.width {
                            w.request_animation_frame();
                        }
                        overview.set(b);
                    },
                    move |b, _, w, _| {
                        let width = f32::from(b.size.width);
                        let height = f32::from(b.size.height);
                        let mut cached = cache.borrow_mut();
                        if cached
                            .as_ref()
                            .is_none_or(|old| old.bounds != b || old.revision != revision)
                        {
                            let path = if values.len() > 1 {
                                let low = values.iter().fold(f32::INFINITY, |a, &b| a.min(b));
                                let high = values.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
                                let span = (high - low).max(1.);
                                let at = |i: usize, v: f32| {
                                    point(
                                        b.origin.x
                                            + px(width * i as f32 / (values.len() - 1) as f32),
                                        b.origin.y
                                            + px(4. + (1. - (v - low) / span) * (height - 4.)),
                                    )
                                };
                                let mut path = PathBuilder::stroke(px(1.));
                                path.move_to(at(0, values[0]));
                                for i in sample_indices(
                                    &values,
                                    0,
                                    values.len() - 1,
                                    width.max(1.) as usize,
                                )
                                .into_iter()
                                .skip(1)
                                {
                                    path.line_to(at(i, values[i]));
                                }
                                path.build().ok()
                            } else {
                                None
                            };
                            *cached = Some(CachedOverview {
                                bounds: b,
                                revision,
                                path,
                            });
                        }
                        if let Some(path) = cached.as_ref().and_then(|v| v.path.as_ref()) {
                            w.paint_path(path.clone(), p.secondary);
                        }
                        drop(cached);
                        for (x, band_width) in [(0., l * width), (r * width, (1. - r) * width)] {
                            if band_width > 0. {
                                w.paint_quad(fill(
                                    Bounds {
                                        origin: point(b.origin.x + px(x), b.origin.y),
                                        size: size(px(band_width), b.size.height),
                                    },
                                    p.panel.opacity(0.65),
                                ));
                            }
                        }
                    },
                )
                .size_full(),
            )
            .child(
                div()
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .left(px(left_x))
                    .w(px((right_x - left_x).max(0.)))
                    .border_1()
                    .border_color(p.bright.opacity(0.45))
                    .rounded(crate::foundation::surface_radius(cx)),
            )
            .children(
                [left_x, right_x]
                    .into_iter()
                    .enumerate()
                    .map(|(i, position)| {
                        let label = if i == 0 {
                            tr(cx, "Start", "起点")
                        } else {
                            tr(cx, "End", "终点")
                        };
                        let value = self
                            .points
                            .get(if i == 0 { start } else { end })
                            .map_or_else(String::new, |v| v.label.to_string());
                        div()
                            .id(("brush-handle", i))
                            .role(Role::Slider)
                            .aria_label(label)
                            .aria_value(value)
                            .tab_index(0)
                            .absolute()
                            .top(px(6.))
                            .left(px(position))
                            .ml(px(-14.))
                            .w(px(28.))
                            .h(px(44.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_ew_resize()
                            .focus_visible(move |s| {
                                s.border_1().border_color(p.focus).rounded(px(8.))
                            })
                            .on_hover(cx.listener(move |s, over: &bool, _, cx| {
                                let next = if *over { Some(i) } else { None };
                                if s.grip_hover != next && (s.grip_hover == Some(i) || *over) {
                                    s.grip_hover = next;
                                    cx.notify();
                                }
                            }))
                            .child(
                                div()
                                    .w(px(6.))
                                    .h(px(if self.grip_hover == Some(i) {
                                        24.6
                                    } else {
                                        22.
                                    }))
                                    .rounded(px(3.))
                                    .bg(p.panel)
                                    .border_1()
                                    .border_color(p.ink.opacity(0.26))
                                    .shadow_sm(),
                            )
                            .on_key_down(cx.listener(move |s, e: &KeyDownEvent, w, cx| {
                                let current = if i == 0 { s.start } else { s.end };
                                let next = match e.keystroke.key.as_str() {
                                    "left" | "down" => current.saturating_sub(1),
                                    "right" | "up" => {
                                        (current + 1).min(s.points.len().saturating_sub(1))
                                    }
                                    "home" => 0,
                                    "end" => s.points.len().saturating_sub(1),
                                    _ => return,
                                };
                                if i == 0 {
                                    s.update(next.min(s.end), s.end, true, cx)
                                } else {
                                    s.update(s.start, next.max(s.start), true, cx)
                                }
                                cx.stop_propagation();
                                w.prevent_default();
                            }))
                    }),
            )
            .when(self.drag.is_some(), |strip| {
                let moved = cx.listener(|s, e: &MouseMoveEvent, _, cx| {
                    if e.pressed_button == Some(MouseButton::Left) {
                        s.drag_to(e.position.x, cx)
                    } else {
                        s.drag = None;
                        cx.notify();
                    }
                });
                let released = cx.listener(|s, _: &MouseUpEvent, _, cx| {
                    s.drag = None;
                    cx.notify();
                });
                strip.child(
                    canvas(
                        |_, _, _| (),
                        move |_, _, w, _| {
                            w.on_mouse_event(move |e: &MouseMoveEvent, phase, w, cx| {
                                if phase.capture() {
                                    moved(e, w, cx)
                                }
                            });
                            w.on_mouse_event(move |e: &MouseUpEvent, phase, w, cx| {
                                if phase.capture() && e.button == MouseButton::Left {
                                    released(e, w, cx)
                                }
                            });
                        },
                    )
                    .absolute()
                    .size(px(0.)),
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
                                if touching.get() {
                                    touch_begin(e, w, cx);
                                }
                            }
                            if !touching.get() {
                                return;
                            }
                            let dx = f32::from(e.position.x - e.start_position.x).abs();
                            let dy = f32::from(e.position.y - e.start_position.y).abs();
                            if e.phase == TouchPhase::Moved && dx > 4. && dx > dy {
                                w.prevent_default();
                                touch_move(e, w, cx);
                                cx.stop_propagation();
                            }
                            if matches!(e.phase, TouchPhase::Ended | TouchPhase::Cancelled) {
                                touch_end(e, w, cx);
                                touching.set(false);
                            }
                        });
                    },
                )
                .absolute()
                .inset_0(),
            );
        let read = self
            .selected
            .and_then(|i| self.points.get(i))
            .map(|v| {
                format!(
                    "{}  {:.1}{}",
                    v.label,
                    v.safe(),
                    v.event
                        .as_ref()
                        .map_or(String::new(), |e| format!(" · {e}"))
                )
            })
            .unwrap_or_else(|| tr(cx, "Select a point", "选择一个数据点").to_string());
        let labels = self
            .points
            .get(start)
            .zip(self.points.get(end))
            .map(|(a, b)| format!("{} – {}", a.label, b.label))
            .unwrap_or_default();
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
                    .items_center()
                    .justify_between()
                    .child(title(self.title.clone(), p))
                    .when(!self.presets.is_empty(), |r| {
                        r.child(
                            row()
                                .gap_1()
                                .p(px(3.))
                                .rounded(crate::foundation::surface_radius(cx))
                                .bg(p.stage)
                                .children(self.presets.iter().enumerate().map(|(i, preset)| {
                                    let last = self.points.len().saturating_sub(1);
                                    let preset_start = preset.points.map_or(0, |count| {
                                        last.saturating_sub(count.saturating_sub(1))
                                    });
                                    let selected = self.start == preset_start && self.end == last;
                                    face(
                                        ("brush-preset", i),
                                        preset.label.clone(),
                                        28.,
                                        crate::foundation::surface_radius(cx).into(),
                                        Tone::Clear,
                                        cx,
                                    )
                                    .px(px(10.))
                                    .aria_toggled(selected.into())
                                    .when(selected, |e| e.bg(p.hover))
                                    .child(preset.label.clone())
                                    .on_click(cx.listener(
                                        move |s, _, _, cx| s.update(preset_start, last, true, cx),
                                    ))
                                })),
                        )
                    }),
            )
            .child(hint(labels, p))
            .child(plot)
            .child(hint(read, p))
            .child(strip)
    }
}
#[cfg(test)]
mod tests {
    use super::{moving_average, sample_indices};
    #[test]
    fn downsampling_preserves_spikes_and_endpoints() {
        let mut values = vec![1.; 100];
        values[43] = 99.;
        values[48] = -4.;
        let indices = sample_indices(&values, 0, 99, 10);
        assert_eq!(indices.first(), Some(&0));
        assert_eq!(indices.last(), Some(&99));
        assert!(indices.contains(&43));
        assert!(indices.contains(&48));
        assert!(indices.windows(2).all(|pair| pair[0] < pair[1]));
    }
    #[test]
    fn moving_average_respects_the_selected_window() {
        assert_eq!(moving_average(&[100., 2., 4., 6., 100.], 2, 1, 3), 4.);
    }
}
