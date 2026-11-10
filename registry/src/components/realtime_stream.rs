//! A host-driven live series which freezes while inspected.
use crate::components::animated_counter::AnimatedCounter;
use crate::primitives::{
    family::{Paint, PhaseClock, Slide, Tone, face},
    presentation::{column, row, tr},
};
use gpui::{prelude::*, *};
use std::{cell::Cell, collections::VecDeque, rc::Rc};
#[derive(Clone, Debug)]
pub struct StreamReading {
    pub label: SharedString,
    pub value: f32,
    pub anomaly: Option<SharedString>,
}
impl StreamReading {
    pub fn new(label: impl Into<SharedString>, value: f32) -> Self {
        Self {
            label: label.into(),
            value,
            anomaly: None,
        }
    }
    pub fn with_anomaly(mut self, label: impl Into<SharedString>) -> Self {
        self.anomaly = Some(label.into());
        self
    }
}
#[derive(Clone, Debug)]
pub struct LiveInspect {
    pub label: SharedString,
    pub value: f32,
    pub anomaly: Option<SharedString>,
}
const EDGE_HISTORY: usize = 8;

struct FrozenSeries {
    readings: Vec<StreamReading>,
    leading: VecDeque<f32>,
    offset: f32,
    axis: Option<(f32, f32)>,
    marker_age: f32,
    evictions: usize,
}

pub struct RealtimeStream {
    title: SharedString,
    unit: SharedString,
    counter: Option<Entity<AnimatedCounter>>,
    readings: Vec<StreamReading>,
    leading: VecDeque<f32>,
    evictions: usize,
    frozen: Option<FrozenSeries>,
    hovering: bool,
    keyboard_hold: bool,
    pinned: bool,
    selected: Option<usize>,
    capacity: usize,
    visible: bool,
    shift: Slide,
    axis_min: Slide,
    axis_max: Slide,
    axis_initialized: bool,
    axis_scale: f64,
    marker: PhaseClock,
    bounds: Rc<Cell<Bounds<Pixels>>>,
    touching: Rc<Cell<bool>>,
    graph_focus: Option<FocusHandle>,
    _focus_subscriptions: Option<[Subscription; 2]>,
}
impl EventEmitter<LiveInspect> for RealtimeStream {}
impl RealtimeStream {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            unit: "".into(),
            counter: None,
            readings: vec![],
            leading: VecDeque::with_capacity(EDGE_HISTORY),
            evictions: 0,
            frozen: None,
            hovering: false,
            keyboard_hold: false,
            pinned: false,
            selected: None,
            capacity: 60,
            visible: true,
            shift: Slide::new(0.),
            axis_min: Slide::new(0.),
            axis_max: Slide::new(1.),
            axis_initialized: false,
            axis_scale: 1.,
            marker: Default::default(),
            bounds: Default::default(),
            touching: Default::default(),
            graph_focus: None,
            _focus_subscriptions: None,
        }
    }
    pub fn with_capacity(mut self, capacity: usize) -> Self {
        self.capacity = capacity.clamp(2, 2000);
        self
    }
    pub fn with_unit(mut self, unit: impl Into<SharedString>) -> Self {
        self.unit = unit.into();
        self
    }
    /// Hosts should update this when the chart leaves or re-enters their viewport.
    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        if self.visible == visible {
            return;
        }
        self.visible = visible;
        self.shift.reset(0.);
        if visible {
            cx.notify();
        }
    }
    /// Add a finite observation. The retained buffer stays bounded while the display is frozen.
    pub fn push(&mut self, reading: StreamReading, cx: &mut Context<Self>) {
        if !reading.value.is_finite() {
            return;
        }
        if self.visible && self.frozen.is_none() && reading.anomaly.is_some() {
            self.marker.restart(cx);
        }
        self.readings.push(reading);
        if self.readings.len() > self.capacity {
            if self.leading.len() == EDGE_HISTORY {
                self.leading.pop_front();
            }
            self.leading.push_back(self.readings.remove(0).value);
            self.evictions += 1;
            if self.visible && self.frozen.is_none() {
                self.shift.translate(1.);
                self.shift.duration(0., 0.35, cx);
            }
        }
        if self.visible && self.frozen.is_none() {
            cx.notify();
        }
    }
    fn data(&self) -> &[StreamReading] {
        self.frozen
            .as_ref()
            .map_or(&self.readings, |frozen| &frozen.readings)
    }
    fn current_offset(&self, cx: &App) -> f32 {
        self.frozen.as_ref().map_or_else(
            || self.shift.value(cx).clamp(0., self.leading.len() as f32),
            |frozen| frozen.offset,
        )
    }
    /// Hold or resume the display; the host may continue supplying readings.
    pub fn set_paused(&mut self, paused: bool, cx: &mut Context<Self>) {
        if self.pinned == paused {
            return;
        }
        self.pinned = paused;
        self.freeze(self.pinned || self.hovering || self.keyboard_hold, cx);
        cx.notify();
    }
    fn toggle_pause(&mut self, cx: &mut Context<Self>) {
        self.keyboard_hold = false;
        self.set_paused(!self.pinned, cx);
    }
    fn freeze(&mut self, freeze: bool, cx: &mut Context<Self>) {
        if freeze && self.frozen.is_none() {
            self.frozen = Some(FrozenSeries {
                readings: self.readings.clone(),
                leading: self.leading.clone(),
                offset: self.current_offset(cx),
                axis: self
                    .axis_initialized
                    .then(|| (self.axis_min.value(cx), self.axis_max.value(cx))),
                marker_age: self.marker.age(cx).min(0.4),
                evictions: self.evictions,
            });
        } else if !freeze && let Some(frozen) = self.frozen.take() {
            self.selected = None;
            let delta = self.evictions.saturating_sub(frozen.evictions) as f32;
            self.shift
                .reset((frozen.offset + delta).min(self.leading.len() as f32));
            self.shift.duration(0., 0.45, cx);
            if let Some((min, max)) = frozen.axis {
                self.axis_min.reset(min);
                self.axis_max.reset(max);
            }
        } else {
            return;
        }
        cx.notify();
    }
    fn inspect(&mut self, index: Option<usize>, cx: &mut Context<Self>) {
        if self.selected == index {
            return;
        }
        self.selected = index;
        if let Some(reading) = index.and_then(|i| self.data().get(i)) {
            cx.emit(LiveInspect {
                label: reading.label.clone(),
                value: reading.value,
                anomaly: reading.anomaly.clone(),
            });
        }
        cx.notify();
    }
}
fn axis_scale(data: &[StreamReading]) -> f64 {
    // Stay in data units at ordinary magnitudes. Renormalizing the spring
    // endpoints on each sample makes the entire curve jump.
    let extent = data
        .iter()
        .map(|v| f64::from(v.value).abs())
        .fold(1., f64::max);
    if extent < 1e30 {
        1.
    } else {
        f64::from(f32::MAX)
    }
}
const GRAPH_HEIGHT: f32 = 200.;
const PLOT_TOP: f32 = 14.;
const PLOT_BOTTOM: f32 = 24.;
const AXIS_GUTTER: f32 = 48.;

fn plot_width(width: Pixels) -> f32 {
    (f32::from(width) - AXIS_GUTTER).max(1.)
}

fn last_visible_index(len: usize, offset: f32) -> usize {
    (len.saturating_sub(1) as f32 - offset).floor().max(0.) as usize
}

fn reading_index(bounds: Bounds<Pixels>, x: Pixels, len: usize, offset: f32) -> usize {
    let fraction = (f32::from(x - bounds.left()) / plot_width(bounds.size.width)).clamp(0., 1.);
    (fraction * len.saturating_sub(1) as f32 - offset)
        .round()
        .clamp(0., last_visible_index(len, offset) as f32) as usize
}

fn normalized(value: f32, scale: f64) -> f32 {
    (f64::from(value) / scale) as f32
}
impl Render for RealtimeStream {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.graph_focus.is_none() {
            let focus = cx.focus_handle().tab_stop(true);
            let enter = cx.on_focus_in(&focus, w, |s, _, cx| {
                let n = s.data().len();
                if n > 0 {
                    s.keyboard_hold = true;
                    s.freeze(true, cx);
                    s.inspect(Some(last_visible_index(n, s.current_offset(cx))), cx);
                }
            });
            let leave = cx.on_focus_out(&focus, w, |s, _, _, cx| {
                s.keyboard_hold = false;
                s.freeze(s.pinned || s.hovering, cx);
            });
            self.graph_focus = Some(focus);
            self._focus_subscriptions = Some([enter, leave]);
        }
        let graph_focus = self
            .graph_focus
            .as_ref()
            .expect("focus initialized")
            .clone();
        let p = Paint::get(cx);
        let data = self.data().to_vec();
        let n = data.len();
        let active = self.selected;
        // Normalize before subtraction: finite f32 endpoints can have an infinite f32 span.
        let scale = axis_scale(&data);
        let to_axis = move |value: f32| normalized(value, scale);
        let (target_min, target_max) = if n == 0 {
            (0., 1.)
        } else {
            (
                data.iter().fold(0f32, |a, v| a.min(to_axis(v.value))),
                data.iter()
                    .fold(f32::NEG_INFINITY, |a, v| a.max(to_axis(v.value))),
            )
        };
        let span = (target_max - target_min)
            .max(0.1 * target_max.abs())
            .max(1e-6);
        let step = 10_f32.powf((span / 3.).log10().floor());
        let target_min = ((target_min - 0.08 * span) / step).floor() * step;
        let target_max = ((target_max + 0.08 * span) / step).ceil() * step;
        let target_min = if data.iter().all(|r| r.value >= 0.) {
            target_min.max(0.)
        } else {
            target_min
        };
        if !self.axis_initialized || scale != self.axis_scale {
            self.axis_min.reset(target_min);
            self.axis_max.reset(target_max);
            self.axis_initialized = true;
            self.axis_scale = scale;
        } else {
            self.axis_min.duration(target_min, 0.45, cx);
            self.axis_max.duration(target_max, 0.45, cx);
        }
        let visible = self.visible && w.is_visible();
        let (min, max) = if let Some(range) = self.frozen.as_ref().and_then(|frozen| frozen.axis) {
            range
        } else if visible {
            (self.axis_min.get(w, cx), self.axis_max.get(w, cx))
        } else {
            (target_min, target_max)
        };
        let leading = self
            .frozen
            .as_ref()
            .map_or(&self.leading, |frozen| &frozen.leading)
            .clone();
        let phase = if let Some(frozen) = &self.frozen {
            frozen.offset
        } else if visible {
            self.shift.get(w, cx).clamp(0., leading.len() as f32)
        } else {
            0.
        };
        let marker_age = if let Some(frozen) = &self.frozen {
            frozen.marker_age
        } else if visible {
            self.marker.time(0.4, w, cx)
        } else {
            0.4
        };
        let bounds = self.bounds.clone();
        let touching = self.touching.clone();
        let graph_data = data.clone();
        let touch = cx.listener(|s, e: &TouchDragEvent, _, cx| match e.phase {
            TouchPhase::Started => s.freeze(true, cx),
            TouchPhase::Moved | TouchPhase::Ended => {
                let n = s.data().len();
                if n > 0 {
                    s.inspect(
                        Some(reading_index(
                            s.bounds.get(),
                            e.position.x,
                            n,
                            s.current_offset(cx),
                        )),
                        cx,
                    );
                }
                if e.phase == TouchPhase::Ended {
                    s.freeze(s.pinned || s.hovering || s.keyboard_hold, cx);
                }
            }
            TouchPhase::Cancelled => s.freeze(s.pinned || s.hovering || s.keyboard_hold, cx),
        });
        let tip_reading = active.filter(|&i| i < n).map(|i| graph_data[i].clone());
        let mut graph = div()
            .id("realtime-stream")
            .role(Role::Figure)
            .tab_index(0)
            .track_focus(&graph_focus)
            .relative()
            .w_full()
            .h(px(GRAPH_HEIGHT))
            .overflow_hidden()
            .aria_label(format!(
                "{}. {}",
                self.title,
                tr(
                    cx,
                    "Hover to pause. Use left and right to inspect readings",
                    "悬停暂停，使用左右方向键查看数据"
                )
            ))
            .on_hover(cx.listener(|s, h: &bool, _, cx| {
                s.hovering = *h;
                s.freeze(s.hovering || s.pinned || s.keyboard_hold, cx)
            }))
            .on_mouse_move(cx.listener(|s, e: &MouseMoveEvent, _, cx| {
                let n = s.data().len();
                if n == 0 {
                    return;
                }
                s.inspect(
                    Some(reading_index(
                        s.bounds.get(),
                        e.position.x,
                        n,
                        s.current_offset(cx),
                    )),
                    cx,
                );
            }))
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                let key = e.keystroke.key.as_str();
                if matches!(key, "space" | "enter") {
                    if !e.is_held {
                        s.toggle_pause(cx);
                    }
                    w.prevent_default();
                    cx.stop_propagation();
                    return;
                }
                if !matches!(
                    key,
                    "left" | "right" | "home" | "end" | "pageup" | "pagedown" | "escape"
                ) {
                    return;
                }
                let n = s.data().len();
                if n == 0 {
                    return;
                }
                if key != "escape" {
                    s.keyboard_hold = true;
                    if s.frozen.is_none() {
                        s.freeze(true, cx);
                    }
                }
                let last = last_visible_index(n, s.current_offset(cx));
                let i = s.selected.unwrap_or(last);
                let next = match key {
                    "left" => Some(i.saturating_sub(1)),
                    "right" => Some((i + 1).min(last)),
                    "pageup" => Some(i.saturating_sub(10)),
                    "pagedown" => Some((i + 10).min(last)),
                    "home" => Some(0),
                    "end" => Some(last),
                    "escape" => {
                        s.pinned = false;
                        s.keyboard_hold = false;
                        s.freeze(s.hovering, cx);
                        None
                    }
                    _ => return,
                };
                w.prevent_default();
                cx.stop_propagation();
                s.inspect(next, cx);
            }))
            .child(
                canvas(
                    move |b, w, _| {
                        // The header and tooltip share the measured chart width.
                        if bounds.replace(b).size.width != b.size.width {
                            w.request_animation_frame();
                        }
                    },
                    move |b, _, w, cx| {
                        if n == 0 {
                            return;
                        }
                        let plot_width = plot_width(b.size.width);
                        let height = f32::from(b.size.height) - PLOT_BOTTOM;
                        let span = (max - min).max((1. / scale) as f32);
                        let plot_height = (height - PLOT_TOP).max(1.);
                        let at_reading = |index: f32, value: f32| {
                            let x = if n > 1 {
                                plot_width * (index + phase) / (n - 1) as f32
                            } else {
                                plot_width
                            };
                            point(
                                b.origin.x + px(x),
                                b.origin.y
                                    + px(PLOT_TOP
                                        + (1. - (to_axis(value) - min) / span) * plot_height),
                            )
                        };
                        let at = |i: usize| at_reading(i as f32, graph_data[i].value);
                        for tick in 0..=2 {
                            let y = PLOT_TOP + plot_height * (1. - tick as f32 / 2.);
                            let mut grid = PathBuilder::stroke(px(1.));
                            grid.move_to(point(b.origin.x, b.origin.y + px(y)));
                            grid.line_to(point(b.origin.x + px(plot_width), b.origin.y + px(y)));
                            if let Ok(path) = grid.build() {
                                w.paint_path(path, p.line);
                            }
                            let tick_value = f64::from(min + span * tick as f32 / 2.) * scale;
                            let label: SharedString = if tick_value.abs() >= 1_000_000. {
                                format!("{tick_value:.1e}").into()
                            } else {
                                format!("{tick_value:.0}").into()
                            };
                            let mut run = w.text_style().to_run(label.len());
                            run.color = p.secondary;
                            let line = w.text_system().shape_line(label, px(10.), &[run], None);
                            let _ = line.paint(
                                point(b.origin.x + px(plot_width + 8.), b.origin.y + px(y - 6.)),
                                px(14.),
                                TextAlign::Left,
                                None,
                                w,
                                cx,
                            );
                        }
                        let clip = Bounds::new(b.origin, size(px(plot_width), px(height + 1.)));
                        w.with_content_mask(Some(ContentMask { bounds: clip }), |w| {
                            let overscan = phase.ceil() as usize;
                            let points = leading
                                .iter()
                                .skip(leading.len().saturating_sub(overscan))
                                .enumerate()
                                .map(|(i, &value)| at_reading(i as f32 - overscan as f32, value))
                                .chain((0..n).map(at));
                            let mut area = PathBuilder::fill();
                            let mut path = PathBuilder::stroke(px(2.));
                            let mut points = points;
                            if let Some(first) = points.next() {
                                area.move_to(point(first.x, b.origin.y + px(height)));
                                area.line_to(first);
                                path.move_to(first);
                            }
                            for sample in points {
                                area.line_to(sample);
                                path.line_to(sample);
                            }
                            area.line_to(point(at(n - 1).x, b.origin.y + px(height)));
                            area.close();
                            if let Ok(path) = area.build() {
                                w.paint_path(path, p.bright.opacity(0.1));
                            }
                            if let Ok(path) = path.build() {
                                w.paint_path(path, p.bright);
                            }
                        });
                        let mut last_tag_right = b.origin.x - px(8.);
                        for (i, r) in graph_data.iter().enumerate() {
                            if let Some(label) = r.anomaly.as_ref() {
                                let pt = at(i);
                                if pt.x >= b.origin.x && pt.x <= b.origin.x + px(plot_width) {
                                    let t = (marker_age / 0.4).clamp(0., 1.);
                                    let radius = if i + 1 == n {
                                        3. * (t + 0.3 * (std::f32::consts::PI * t).sin())
                                    } else {
                                        3.
                                    };
                                    w.paint_quad(
                                        fill(
                                            Bounds {
                                                origin: point(pt.x - px(radius), pt.y - px(radius)),
                                                size: size(px(radius * 2.), px(radius * 2.)),
                                            },
                                            p.danger,
                                        )
                                        .corner_radii(px(radius)),
                                    );
                                    let label = label.clone();
                                    let mut run = w.text_style().to_run(label.len());
                                    run.color = p.danger;
                                    let text =
                                        w.text_system().shape_line(label, px(10.), &[run], None);
                                    let x = px(f32::from(pt.x - text.width / 2.)
                                        .max(f32::from(b.origin.x))
                                        .min(f32::from(b.origin.x + px(plot_width) - text.width)));
                                    if x >= last_tag_right + px(8.) && text.width <= px(plot_width)
                                    {
                                        let y =
                                            px(f32::from(pt.y - px(18.))
                                                .max(f32::from(b.origin.y)));
                                        let _ = text.paint(
                                            point(x, y),
                                            px(14.),
                                            TextAlign::Left,
                                            None,
                                            w,
                                            cx,
                                        );
                                        last_tag_right = x + text.width;
                                    }
                                }
                            }
                        }
                        if n > 1 {
                            let last = last_visible_index(n, phase);
                            let middle =
                                reading_index(b, b.left() + px(plot_width * 0.5), n, phase);
                            for (i, anchor) in [(0, 0.), (middle, 0.5), (last, 1.)] {
                                if (anchor == 0.5 && (middle == 0 || middle == last))
                                    || (anchor == 1. && last == 0)
                                {
                                    continue;
                                }
                                let label = graph_data[i].label.clone();
                                let mut run = w.text_style().to_run(label.len());
                                run.color = p.secondary;
                                let line = w.text_system().shape_line(label, px(10.), &[run], None);
                                let x = anchor * (plot_width - f32::from(line.width)).max(0.);
                                let _ = line.paint(
                                    point(b.origin.x + px(x), b.origin.y + px(height + 2.)),
                                    px(14.),
                                    TextAlign::Left,
                                    None,
                                    w,
                                    cx,
                                );
                            }
                        }
                        if let Some(i) = active.filter(|&i| i < n) {
                            let pt = at(i);
                            let mut line = PathBuilder::stroke(px(1.));
                            line.move_to(point(pt.x, b.origin.y));
                            line.line_to(point(pt.x, b.origin.y + px(height)));
                            if let Ok(path) = line.build() {
                                w.paint_path(path, p.ink.opacity(0.5));
                            }
                            w.paint_quad(
                                fill(
                                    Bounds {
                                        origin: point(pt.x - px(6.), pt.y - px(6.)),
                                        size: size(px(12.), px(12.)),
                                    },
                                    p.panel,
                                )
                                .corner_radii(px(6.)),
                            );
                            w.paint_quad(
                                fill(
                                    Bounds {
                                        origin: point(pt.x - px(4.), pt.y - px(4.)),
                                        size: size(px(8.), px(8.)),
                                    },
                                    p.bright,
                                )
                                .corner_radii(px(4.)),
                            );
                            let _ = cx;
                        }
                    },
                )
                .size_full(),
            )
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
                            if e.phase == TouchPhase::Started
                                || e.phase == TouchPhase::Ended
                                || e.phase == TouchPhase::Cancelled
                            {
                                touch(e, w, cx);
                            } else if e.phase == TouchPhase::Moved && dx > 4. && dx > dy {
                                w.prevent_default();
                                touch(e, w, cx);
                                cx.stop_propagation();
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
        if let Some(point) = tip_reading {
            let plot_width = plot_width(self.bounds.get().size.width);
            let x = if n > 1 {
                (plot_width * (active.unwrap_or(0) as f32 + phase) / (n - 1) as f32).min(plot_width)
            } else {
                plot_width
            };
            let span = (max - min).max((1. / scale) as f32);
            let plot_height = GRAPH_HEIGHT - PLOT_TOP - PLOT_BOTTOM;
            let y = PLOT_TOP + (1. - (to_axis(point.value) - min) / span) * plot_height;
            let left = if x + 14. + 120. > plot_width {
                (x - 14. - 120.).max(0.)
            } else {
                x + 14.
            };
            let top = if y < 12. + 44. {
                y + 14.
            } else {
                y - 12. - 44.
            };
            graph = graph.child(
                div()
                    .absolute()
                    .left(px(left.max(0.)))
                    .top(px(top.max(0.)))
                    .px(px(12.))
                    .py(px(8.))
                    .flex()
                    .flex_col()
                    .gap(px(1.))
                    .rounded(crate::foundation::surface_radius(cx))
                    .border_1()
                    .border_color(p.line)
                    .bg(p.panel)
                    .shadow_lg()
                    .child(
                        div()
                            .text_size(px(14.))
                            .line_height(px(18.2))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(format!("{:.1}{}", point.value, self.unit)),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .line_height(px(15.6))
                            .text_color(p.faint)
                            .child(point.label.clone()),
                    ),
            );
        }
        let current = self
            .selected
            .and_then(|i| data.get(i))
            .or_else(|| data.last());
        let value = current.map_or(0., |v| v.value);
        let counter = self
            .counter
            .get_or_insert_with(|| {
                cx.new(|_| {
                    AnimatedCounter::new(f64::from(
                        value.clamp(-1_000_000_000_000., 1_000_000_000_000.),
                    ))
                    .with_size(44.)
                })
            })
            .clone();
        counter.update(cx, |v, cx| {
            v.set_value(
                f64::from(value.clamp(-1_000_000_000_000., 1_000_000_000_000.)),
                cx,
            )
        });
        let graph_width = self.bounds.get().size.width;
        let compact = if graph_width > px(0.) {
            graph_width < px(250.)
        } else {
            w.viewport_size().width < px(330.)
        };
        let label = if self.frozen.is_some() {
            tr(cx, "Paused", "已暂停")
        } else {
            tr(cx, "Live", "实时")
        };
        let status = face(
            "live-status",
            label,
            32.,
            crate::foundation::surface_radius(cx).into(),
            Tone::Clear,
            cx,
        )
        .pl(px(10.))
        .pr(px(12.))
        .gap(px(8.))
        .border_1()
        .border_color(p.line)
        .text_color(p.secondary)
        .hover(|s| s.bg(p.tray).text_color(p.ink))
        .aria_toggled(self.pinned.into())
        .child(if self.frozen.is_some() {
            div()
                .w(px(8.))
                .h(px(8.))
                .flex()
                .justify_between()
                .child(div().w(px(3.)).h(px(8.)).bg(p.secondary))
                .child(div().w(px(3.)).h(px(8.)).bg(p.secondary))
        } else {
            div().size(px(8.)).rounded_full().bg(p.bright)
        })
        .child(label)
        .on_click(cx.listener(|s, _, _, cx| s.toggle_pause(cx)))
        .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
            if matches!(e.keystroke.key.as_str(), "enter" | "space") && !e.is_held {
                s.toggle_pause(cx);
                w.prevent_default();
                cx.stop_propagation();
            }
        }));
        let (title_status, value_status) = if compact {
            (Some(status), None)
        } else {
            (None, Some(status))
        };
        column()
            .w(px(560.))
            .max_w_full()
            .gap(px(12.))
            .p(px(24.))
            .rounded(crate::foundation::surface_radius(cx))
            .bg(p.panel)
            .border_1()
            .border_color(p.line)
            .text_color(p.ink)
            .child(
                row()
                    .justify_between()
                    .items_center()
                    .min_h(px(22.))
                    .flex_wrap()
                    .gap(px(8.))
                    .child(
                        div()
                            .min_w_0()
                            .text_size(px(16.))
                            .line_height(px(22.))
                            .font_weight(FontWeight::MEDIUM)
                            .child(self.title.clone()),
                    )
                    .children(title_status),
            )
            .child(
                row()
                    .items_center()
                    .justify_between()
                    .child(
                        row()
                            .items_end()
                            .gap(px(6.))
                            .child(
                                row()
                                    .h(px(52.))
                                    .items_center()
                                    .font_features(FontFeatures(vec![("tnum".into(), 1)].into()))
                                    .when(value.abs() <= 1_000_000_000_000., |row| {
                                        row.child(counter)
                                    })
                                    .when(value.abs() > 1_000_000_000_000., |row| {
                                        row.child(format!("{value:.2e}"))
                                    }),
                            )
                            .child(
                                div()
                                    .pb(px(6.))
                                    .text_color(p.secondary)
                                    .child(self.unit.clone()),
                            ),
                    )
                    .children(value_status),
            )
            .child(graph)
            // A visually hidden table of the latest twenty readings, as on the reference.
            .child(
                div()
                    .id("live-readings")
                    .absolute()
                    .size(px(1.))
                    .overflow_hidden()
                    .role(Role::Table)
                    .aria_label(tr(cx, "Latest readings", "最新读数"))
                    .children(data.iter().rev().take(20).enumerate().map(|(i, r)| {
                        div()
                            .id(("live-reading", i))
                            .role(Role::Row)
                            .aria_label(format!(
                                "{} {:.1}{}{}",
                                r.label,
                                r.value,
                                self.unit,
                                r.anomaly
                                    .as_ref()
                                    .map_or(String::new(), |a| format!(" · {a}"))
                            ))
                    })),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::{RealtimeStream, StreamReading, axis_scale, normalized, reading_index};
    use gpui::{Bounds, TestAppContext, point, px, size};

    #[test]
    fn pointer_at_plot_end_selects_the_latest_reading() {
        let bounds = Bounds::new(point(px(20.), px(0.)), size(px(500.), px(200.)));
        assert_eq!(reading_index(bounds, px(20.), 61, 0.), 0);
        assert_eq!(reading_index(bounds, px(246.), 61, 0.), 30);
        assert_eq!(reading_index(bounds, px(472.), 61, 0.), 60);
        assert_eq!(reading_index(bounds, px(472.), 61, 1.), 59);
        assert_eq!(reading_index(bounds, px(472.), 61, 0.25), 59);
    }

    #[test]
    fn pause_keeps_the_curve_offset_and_its_left_edge_history() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (chart, window) =
            app.add_window_view(|_, _| RealtimeStream::new("Live").with_capacity(3));
        window.update(|_, cx| {
            chart.update(cx, |chart, cx| {
                for i in 0..4 {
                    chart.push(StreamReading::new(i.to_string(), i as f32), cx);
                }
                let before = chart.current_offset(cx);
                assert!(before > 0.);
                chart.set_paused(true, cx);
                chart.push(StreamReading::new("4", 4.), cx);
                let frozen = chart.frozen.as_ref().expect("paused");
                assert_eq!(frozen.offset, before);
                assert_eq!(frozen.leading.front(), Some(&0.));
                assert_eq!(frozen.readings.last().map(|r| r.value), Some(3.));
            });
        });
    }
    #[test]
    fn extreme_finite_values_produce_a_finite_axis() {
        let data = vec![
            StreamReading::new("low", -f32::MAX),
            StreamReading::new("high", f32::MAX),
        ];
        let scale = axis_scale(&data);
        let low = normalized(data[0].value, scale);
        let high = normalized(data[1].value, scale);
        assert!(low.is_finite() && high.is_finite());
        assert!((high - low).is_finite());
        assert_eq!(low, -1.);
        assert_eq!(high, 1.);
    }
    #[test]
    fn focusing_the_plot_holds_the_latest_visible_reading() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) = app.add_window_view(|_, _| RealtimeStream::new("Live"));
        w.update(|w, _| w.activate_window());
        w.run_until_parked();
        w.update(|w, cx| {
            view.update(cx, |s, cx| {
                for i in 0..12 {
                    s.push(StreamReading::new(i.to_string(), i as f32), cx);
                }
            });
            let focus = view.read(cx).graph_focus.clone().expect("focus");
            focus.focus(w, cx);
        });
        w.run_until_parked();
        assert!(w.update(|_, cx| view.read(cx).keyboard_hold));
        assert_eq!(w.update(|_, cx| view.read(cx).selected), Some(11));
        w.simulate_keystrokes("pageup");
        assert_eq!(w.update(|_, cx| view.read(cx).selected), Some(1));
        w.simulate_keystrokes("space");
        assert!(w.update(|_, cx| view.read(cx).pinned));
        assert!(w.update(|_, cx| view.read(cx).frozen.is_some()));
        w.simulate_keystrokes("enter");
        assert!(w.update(|_, cx| view.read(cx).frozen.is_none()));
        w.simulate_keystrokes("a");
        assert!(w.update(|_, cx| view.read(cx).frozen.is_none()));
        w.simulate_keystrokes("left");
        assert!(w.update(|_, cx| view.read(cx).frozen.is_some()));
        w.simulate_keystrokes("escape");
        w.run_until_parked();
        assert!(w.update(|_, cx| view.read(cx).frozen.is_none()));
    }
}
