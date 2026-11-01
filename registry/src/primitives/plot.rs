//! Device-independent chart geometry. Keyboard and pointer use the same sample index.
use crate::primitives::{
    control::UiLanguage,
    family::{Paint, PhaseClock, Slide, disabled_motion, ease, standard_ease},
    presentation::*,
};
use gpui::{prelude::*, *};
use std::{
    cell::Cell,
    f32::consts::{PI, TAU},
    rc::Rc,
};
#[derive(Clone, Debug)]
pub struct Sample {
    pub label: SharedString,
    pub value: f32,
    pub comparison: Option<f32>,
}
impl Sample {
    pub fn new(label: impl Into<SharedString>, value: f32) -> Self {
        Self {
            label: label.into(),
            value: if value.is_finite() { value.max(0.) } else { 0. },
            comparison: None,
        }
    }
    pub fn compare(mut self, other: f32) -> Self {
        self.comparison = Some(other.max(0.));
        self
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PlotKind {
    Spark,
    Bar,
    Line,
    Calendar { first_weekday: usize },
    Donut,
}
#[derive(Clone, Copy, Debug)]
pub struct InspectSample(pub usize);
/// A bar keeps its identity and velocity as the timeline expands or contracts.
struct BarSlot {
    key: SharedString,
    center: Slide,
    width: Slide,
    value: Slide,
}
struct PlotFrame {
    kind: PlotKind,
    entrance: f32,
    cursor: f32,
    cursor_alpha: f32,
    comparison: f32,
    weights: Vec<f32>,
    bars: Vec<(f32, f32, f32)>,
}
fn spring_fraction(t: f32, duration: f32) -> f32 {
    if t <= 0. {
        return 0.;
    }
    if t >= duration * 3. {
        return 1.;
    }
    let u = 5.236 * t / duration;
    1. - (1. + u) * (-u).exp()
}
fn range_progress(kind: PlotKind, age: f32, index: usize, length: usize, forward: bool) -> f32 {
    match kind {
        PlotKind::Calendar { first_weekday } => {
            let slot = index + first_weekday;
            let last = (length + first_weekday).div_ceil(7) + 5;
            let rank = (slot / 7 + slot % 7).min(last);
            let phase = if forward { rank } else { last - rank };
            let delay = phase as f32 * (0.42 / last.max(1) as f32).min(0.012);
            standard_ease((age - delay) / 0.32)
        }
        PlotKind::Donut => spring_fraction(age, 0.5),
        _ => spring_fraction(age, 0.4),
    }
}
fn position_value(source: &[Sample], position: f32) -> (f32, Option<f32>) {
    if source.is_empty() {
        return (0., None);
    }
    let at = position.clamp(0., 1.) * (source.len() - 1) as f32;
    let a = &source[at.floor() as usize];
    let b = &source[at.ceil() as usize];
    (
        a.value + (b.value - a.value) * at.fract(),
        a.comparison
            .zip(b.comparison)
            .map(|(a, b)| a + (b - a) * at.fract()),
    )
}
/// Shape-preserving cubic interpolation on a common screen-space grid. Flat
/// tangents at turning points prevent a curve from overshooting its samples.
fn smooth_at(values: &[f32], t: f32) -> f32 {
    let last = values.len() - 1;
    if last == 0 {
        return values[0];
    }
    let tangent = |i: usize| {
        if i == 0 {
            return values[1] - values[0];
        }
        if i == last {
            return values[last] - values[last - 1];
        }
        let a = values[i] - values[i - 1];
        let b = values[i + 1] - values[i];
        if a * b <= 0. {
            0.
        } else {
            2. * a * b / (a + b)
        }
    };
    let pos = t.clamp(0., 1.) * last as f32;
    let i = (pos.floor() as usize).min(last - 1);
    let u = pos - i as f32;
    let u2 = u * u;
    let u3 = u2 * u;
    (2. * u3 - 3. * u2 + 1.) * values[i]
        + (u3 - 2. * u2 + u) * tangent(i)
        + (-2. * u3 + 3. * u2) * values[i + 1]
        + (u3 - u2) * tangent(i + 1)
}
pub struct Plot {
    kind: PlotKind,
    title: SharedString,
    data: Vec<Sample>,
    previous: Vec<Sample>,
    active: Option<usize>,
    clock: PhaseClock,
    entrance: Option<PhaseClock>,
    forward: bool,
    cursor: Slide,
    cursor_alpha: Slide,
    comparison_alpha: Slide,
    emphasis: Vec<Slide>,
    bars: Vec<BarSlot>,
    bounds: Rc<Cell<Bounds<Pixels>>>,
    pub header: Option<AnyView>,
    show_comparison: bool,
    segments: Vec<(SharedString, Slide)>,
    curve: Vec<(Slide, Option<Slide>)>,
    hidden: Vec<SharedString>,
    pinned: Option<usize>,
    readout: ChangingText,
    total: Slide,
    mean: Slide,
}
impl Plot {
    pub fn new(kind: PlotKind, title: impl Into<SharedString>, data: Vec<Sample>) -> Self {
        let total = data.iter().map(|p| p.value).sum::<f32>();
        Self {
            total: Slide::new(total),
            mean: Slide::new(total / data.len().max(1) as f32),
            kind,
            title: title.into(),
            data,
            previous: vec![],
            entrance: None,
            forward: true,
            cursor: Slide::snappy(0.),
            cursor_alpha: Slide::snappy(0.),
            comparison_alpha: Slide::new(1.),
            emphasis: vec![],
            bars: vec![],
            active: None,
            clock: Default::default(),
            bounds: Default::default(),
            header: None,
            show_comparison: true,
            segments: vec![],
            curve: vec![],
            hidden: vec![],
            pinned: None,
            readout: ChangingText::new(""),
        }
    }
    pub(crate) fn set_calendar_start(&mut self, weekday: usize) {
        self.kind = PlotKind::Calendar {
            first_weekday: weekday % 7,
        };
    }
    fn sync_bars(&mut self, first: bool, cx: &App) {
        if self.kind != PlotKind::Bar {
            return;
        }
        let n = self.data.len().max(1);
        self.bars
            .retain(|b| b.width.value(cx) > 0.0001 || self.data.iter().any(|s| s.label == b.key));
        for bar in &mut self.bars {
            if !self.data.iter().any(|s| s.label == bar.key) {
                bar.width.duration(0., 0.24, cx);
                bar.value.duration(0., 0.24, cx);
            }
        }
        for (i, s) in self.data.iter().enumerate() {
            let center = (i as f32 + 0.5) / n as f32;
            let width = 0.64 / n as f32;
            let idx = self.bars.iter().position(|b| b.key == s.label);
            let b = if let Some(i) = idx {
                &mut self.bars[i]
            } else {
                self.bars.push(BarSlot {
                    key: s.label.clone(),
                    center: Slide::new(center),
                    width: Slide::new(if first { width } else { 0. }),
                    value: Slide::new(0.),
                });
                self.bars.last_mut().expect("bar inserted")
            };
            b.center.duration(center, 0.4, cx);
            b.width.duration(width, 0.4, cx);
            b.value.delayed(
                s.value,
                0.42,
                if first {
                    i as f32 * (0.32 / n as f32).min(0.035)
                } else {
                    0.
                },
                cx,
            );
        }
    }
    fn sync_curve(&mut self, cx: &App) {
        if !matches!(self.kind, PlotKind::Line | PlotKind::Spark) {
            return;
        }
        if self.data.is_empty() {
            self.curve.clear();
            return;
        }
        let values: Vec<_> = self.data.iter().map(|s| s.value).collect();
        let comparison = self
            .data
            .iter()
            .map(|s| s.comparison)
            .collect::<Option<Vec<_>>>();
        // Keep the same abscissas across a 7-, 30- or 90-point range, so the
        // outgoing shape and its velocity survive even the first reversed frame.
        const POINTS: usize = 241;
        for i in 0..POINTS {
            let at = i as f32 / (POINTS - 1) as f32;
            let target = smooth_at(&values, at);
            let other = comparison.as_ref().map(|v| smooth_at(v, at));
            if let Some((main, prev)) = self.curve.get_mut(i) {
                main.duration(target, 0.4, cx);
                match (&mut *prev, other) {
                    (Some(s), Some(v)) => s.duration(v, 0.4, cx),
                    (_, v) => *prev = v.map(Slide::new),
                }
            } else {
                self.curve.push((Slide::new(target), other.map(Slide::new)));
            }
        }
    }
    fn sync_segments(&mut self, first: bool, cx: &App) {
        if self.kind != PlotKind::Donut {
            return;
        }
        let mut old = std::mem::take(&mut self.segments);
        self.segments = self
            .data
            .iter()
            .map(|s| {
                let amount = if self.hidden.contains(&s.label) {
                    0.
                } else {
                    s.value
                };
                let mut sweep = old.iter().position(|(key, _)| key == &s.label).map_or_else(
                    || Slide::new(if first { amount } else { 0. }),
                    |i| old.remove(i).1,
                );
                sweep.duration(amount, 0.5, cx);
                (s.label.clone(), sweep)
            })
            .collect();
    }
    fn toggle_segment(&mut self, i: usize, cx: &App) {
        let label = &self.data[i].label;
        if let Some(at) = self.hidden.iter().position(|l| l == label) {
            self.hidden.remove(at);
        } else if self
            .data
            .iter()
            .filter(|s| !self.hidden.contains(&s.label))
            .count()
            > 1
        {
            self.hidden.push(label.clone());
        } else {
            return;
        }
        if self.active == Some(i) {
            self.active = None;
            self.emphasis[i].duration(0., 0.26, cx);
        }
        if self.pinned == Some(i) {
            self.pinned = None;
        }
        self.sync_segments(false, cx);
    }
    fn shown_value(&self, i: usize, age: f32) -> (f32, Option<f32>) {
        let s = &self.data[i];
        if self.previous.is_empty() {
            return (s.value, s.comparison);
        }
        let f = range_progress(self.kind, age, i, self.data.len(), self.forward);
        let (old, comp) = if matches!(self.kind, PlotKind::Spark | PlotKind::Line) {
            position_value(
                &self.previous,
                i as f32 / self.data.len().saturating_sub(1).max(1) as f32,
            )
        } else {
            self.previous
                .get(i)
                .map_or((0., None), |s| (s.value, s.comparison))
        };
        (
            old + (s.value - old) * f,
            s.comparison
                .map(|v| comp.unwrap_or(v) + (v - comp.unwrap_or(v)) * f),
        )
    }
    pub fn set_data(&mut self, data: Vec<Sample>, cx: &App) {
        let age = self.clock.age(cx);
        self.previous = (0..self.data.len())
            .map(|i| {
                let (value, comparison) = self.shown_value(i, age);
                Sample {
                    label: self.data[i].label.clone(),
                    value,
                    comparison,
                }
            })
            .collect();
        self.forward = self
            .data
            .first()
            .zip(data.first())
            .is_none_or(|(a, b)| b.label >= a.label);
        let total = data.iter().map(|p| p.value).sum::<f32>();
        self.total.duration(total, 0.4, cx);
        self.mean
            .duration(total / data.len().max(1) as f32, 0.4, cx);
        self.data = data;
        self.sync_bars(false, cx);
        self.sync_curve(cx);
        self.sync_segments(false, cx);
        self.pinned = None;
        self.emphasis.resize(self.data.len(), Slide::new(0.));
        for s in &mut self.emphasis {
            s.duration(0., 0.26, cx);
        }
        self.cursor_alpha.duration(0., 0.16, cx);
        self.clock.restart(cx);
        self.active = None;
    }
    fn index_at(&self, position: Point<Pixels>) -> Option<usize> {
        let b = self.bounds.get();
        if self.data.is_empty() || !b.contains(&position) {
            return None;
        }
        let x = f32::from(position.x - b.left()) / f32::from(b.size.width).max(1.);
        let y = f32::from(position.y - b.top()) / f32::from(b.size.height).max(1.);
        let n = self.data.len();
        Some(match self.kind {
            PlotKind::Calendar { first_weekday } => {
                let column = (x * (n + first_weekday).div_ceil(7) as f32) as usize;
                let slot = column * 7 + (y * 7.) as usize;
                return slot.checked_sub(first_weekday).filter(|&i| i < n);
            }
            PlotKind::Donut => {
                let dx = (x - 0.5) * f32::from(b.size.width);
                let dy = (y - 0.5) * f32::from(b.size.height);
                let r = (dx * dx + dy * dy).sqrt()
                    / f32::from(b.size.height).min(f32::from(b.size.width));
                if !(0.31..=0.49).contains(&r) {
                    return None;
                }
                let fraction = (dy.atan2(dx) + PI / 2.).rem_euclid(TAU) / TAU;
                let total = self
                    .data
                    .iter()
                    .filter(|s| !self.hidden.contains(&s.label))
                    .map(|s| s.value)
                    .sum::<f32>();
                if total <= 0.0 {
                    return None;
                }
                let mut consumed = 0.;
                self.data
                    .iter()
                    .position(|s| {
                        if self.hidden.contains(&s.label) {
                            return false;
                        }
                        consumed += s.value;
                        consumed > fraction * total
                    })
                    .unwrap_or(n - 1)
            }
            PlotKind::Bar => (x * n as f32).floor().min((n - 1) as f32) as usize,
            _ => (x * (n - 1) as f32).round() as usize,
        })
    }
    pub fn render<T: EventEmitter<InspectSample> + 'static>(
        &mut self,
        w: &Window,
        cx: &Context<T>,
        access: fn(&mut T) -> &mut Self,
    ) -> Div {
        let p = Paint::get(cx);
        let kind = self.kind;
        if self.entrance.is_none() {
            let mut start = PhaseClock::default();
            start.restart(cx);
            self.entrance = Some(start);
            self.emphasis.resize(self.data.len(), Slide::new(0.));
            self.sync_bars(true, cx);
            self.sync_curve(cx);
            self.sync_segments(true, cx);
        }
        let entrance = self.entrance.unwrap_or_default().time(
            match kind {
                PlotKind::Donut => 2.45,
                PlotKind::Calendar { .. } => 1.15,
                PlotKind::Spark | PlotKind::Line => 0.84,
                PlotKind::Bar => 0.,
            },
            w,
            cx,
        );
        let age = self
            .clock
            .time(if kind == PlotKind::Donut { 1.5 } else { 1.2 }, w, cx);
        let values: Vec<_> = if kind == PlotKind::Donut {
            self.segments
                .iter()
                .map(|(_, s)| (s.get(w, cx).max(0.), None))
                .collect()
        } else {
            (0..self.data.len())
                .map(|i| self.shown_value(i, age))
                .collect()
        };
        let snapshot = PlotFrame {
            kind,
            entrance: if disabled_motion(cx) {
                f32::INFINITY
            } else {
                entrance
            },
            cursor: self.cursor.get(w, cx) / self.data.len().saturating_sub(1).max(1) as f32,
            cursor_alpha: self.cursor_alpha.get(w, cx),
            comparison: self.comparison_alpha.get(w, cx),
            weights: self.emphasis.iter().map(|s| s.get(w, cx)).collect(),
            bars: self
                .bars
                .iter()
                .map(|b| (b.center.get(w, cx), b.width.get(w, cx), b.value.get(w, cx)))
                .collect(),
        };
        let sum = if kind == PlotKind::Donut {
            values.iter().map(|s| s.0).sum::<f32>()
        } else {
            self.total.get(w, cx)
        };
        let bounds = self.bounds.clone();
        let active = self.active;
        let cmp = self.show_comparison;
        let height = match kind {
            PlotKind::Spark => 102.,
            PlotKind::Calendar { .. } => 84.,
            PlotKind::Donut => 176.,
            _ => 158.,
        };
        let inspect = move |v: &mut T, i: Option<usize>, cx: &mut Context<T>| {
            let plot = access(v);
            if plot.active != i {
                if let Some(old) = plot.active.and_then(|at| plot.emphasis.get_mut(at)) {
                    old.duration(0., 0.26, cx);
                }
                if let Some(at) = i {
                    if let Some(weight) = plot.emphasis.get_mut(at) {
                        weight.duration(1., 0.26, cx);
                    }
                    if plot.active.is_none() {
                        plot.cursor.reset(at as f32)
                    } else {
                        plot.cursor.duration(at as f32, 0.26, cx)
                    }
                } else if plot.kind == PlotKind::Spark {
                    plot.cursor
                        .duration(plot.data.len().saturating_sub(1) as f32, 0.26, cx)
                }
                plot.cursor_alpha.duration(f32::from(i.is_some()), 0.16, cx);
                plot.active = i;
                if let Some(i) = i {
                    cx.emit(InspectSample(i))
                }
                cx.notify()
            }
        };
        let values = Rc::new(values);
        let series = if self.curve.is_empty() || self.data.len() < 2 {
            values.clone()
        } else {
            Rc::new(
                self.curve
                    .iter()
                    .map(|(a, b)| (a.value(cx), b.as_ref().map(|x| x.value(cx))))
                    .collect(),
            )
        };
        let graph = div()
            .id("plot-area")
            .debug_selector(|| "plot-area".into())
            .role(Role::Figure)
            .aria_label(self.title.clone())
            .tab_index(0)
            .w_full()
            .h(px(height))
            .focus_visible(move |s| s.border_1().border_color(p.secondary))
            .on_hover(cx.listener(move |v, h: &bool, _, cx| {
                if !h {
                    let pinned = access(v).pinned;
                    inspect(v, pinned, cx)
                }
            }))
            .on_mouse_move(cx.listener(move |v, e: &MouseMoveEvent, _, cx| {
                let i = access(v).index_at(e.position);
                inspect(v, i, cx)
            }))
            .on_click(cx.listener(move |v, e: &ClickEvent, _, cx| {
                if !matches!(e, ClickEvent::Keyboard(_)) {
                    let plot = access(v);
                    let mut i = plot.index_at(e.position());
                    if plot.kind == PlotKind::Donut {
                        plot.pinned = i.filter(|at| Some(*at) != plot.pinned);
                        i = plot.pinned;
                    }
                    inspect(v, i, cx)
                }
            }))
            .on_key_down(cx.listener(move |v, e: &KeyDownEvent, _, cx| {
                let plot = access(v);
                let n = plot.data.len();
                if n == 0 {
                    return;
                }
                let i = plot.active.unwrap_or(0);
                let next = match e.keystroke.key.as_str() {
                    "right" => Some(
                        (i + if matches!(kind, PlotKind::Calendar { .. }) {
                            7
                        } else {
                            1
                        })
                        .min(n - 1),
                    ),
                    "left" => Some(i.saturating_sub(
                        if matches!(kind, PlotKind::Calendar { .. }) {
                            7
                        } else {
                            1
                        },
                    )),
                    "up" => Some(i.saturating_sub(1)),
                    "down" => Some((i + 1).min(n - 1)),
                    "home" => Some(0),
                    "end" => Some(n - 1),
                    "escape" => None,
                    _ => return,
                };
                if plot.kind == PlotKind::Donut && e.keystroke.key == "escape" {
                    plot.pinned = None;
                }
                cx.stop_propagation();
                inspect(v, next, cx)
            }))
            .child(
                canvas(
                    move |b, _, _| bounds.set(b),
                    move |b, _, w, _| draw(&snapshot, b, &series, active, p, w),
                )
                .size_full(),
            );
        let mean = self.mean.get(w, cx);
        let width = match kind {
            PlotKind::Spark => 360.,
            PlotKind::Calendar { .. } => 520.,
            _ => 496.,
        };
        let mut el = if kind == PlotKind::Spark {
            column().w(px(width)).max_w_full().gap_2()
        } else {
            frame(width, p)
        };
        let summary = match kind {
            PlotKind::Spark => self
                .data
                .last()
                .map(|v| format!("{:.0}", v.value))
                .unwrap_or_default(),
            PlotKind::Calendar { .. } => format!("{sum:.0}"),
            _ => String::new(),
        };
        el = el.child(
            row()
                .justify_between()
                .gap_2()
                .flex_wrap()
                .child(
                    column()
                        .child(title(self.title.clone(), p))
                        .when(self.header.is_some() && !summary.is_empty(), |c| {
                            c.child(hint(summary.clone(), p))
                        }),
                )
                .when_some(self.header.clone(), |c, h| c.child(h))
                .when(self.header.is_none(), |c| c.child(title(summary, p))),
        );
        match kind {
            PlotKind::Line | PlotKind::Bar => {
                let (label, number) = if kind == PlotKind::Line {
                    (tr(cx, "Total", "合计"), sum)
                } else {
                    (tr(cx, "Daily average", "日均"), mean)
                };
                el = el.child(
                    column()
                        .child(hint(label, p))
                        .child(div().text_size(px(27.)).child(format!("{number:.0}"))),
                )
            }
            PlotKind::Calendar { .. } if self.data.len() >= 365 => {
                el = el.child(row().justify_between().children(
                    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11].map(|m| {
                        hint(
                            if UiLanguage::current(cx) == UiLanguage::Chinese {
                                format!("{}月", m + 1)
                            } else {
                                [
                                    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep",
                                    "Oct", "Nov", "Dec",
                                ][m]
                                    .to_owned()
                            },
                            p,
                        )
                        .text_size(px(9.))
                    }),
                ))
            }
            _ => {}
        }
        if kind == PlotKind::Line && self.data.iter().any(|s| s.comparison.is_some()) {
            el = el.child(
                row()
                    .gap_2()
                    .child(hint(tr(cx, "This period", "本期"), p))
                    .child(
                        command(
                            "chart-comparison",
                            tr(cx, "Previous period", "上期"),
                            cx,
                            move |v, _, cx| {
                                let plot = access(v);
                                plot.show_comparison = !plot.show_comparison;
                                plot.comparison_alpha.duration(
                                    f32::from(plot.show_comparison),
                                    0.24,
                                    cx,
                                );
                                cx.notify()
                            },
                        )
                        .opacity(if cmp { 1. } else { 0.48 }),
                    ),
            )
        }
        if kind == PlotKind::Donut {
            let val = active.and_then(|i| self.data.get(i));
            let label = val.map_or_else(
                || tr(cx, "Total", "合计").to_owned(),
                |v| v.label.to_string(),
            );
            if self.readout.current.is_empty() {
                self.readout = ChangingText::new(label.clone());
            } else {
                self.readout.set(label, 1., cx);
            }
            el = el.child(
                div().relative().w_full().h(px(height)).child(graph).child(
                    column()
                        .absolute()
                        .top(px(height / 2. - 19.))
                        .left_0()
                        .right_0()
                        .items_center()
                        .child(
                            div()
                                .text_color(p.secondary)
                                .text_size(px(12.))
                                .child(self.readout.render(12., w, cx)),
                        )
                        .child(title(
                            format!(
                                "{:.0}",
                                active.and_then(|i| values.get(i)).map_or(sum, |s| s.0)
                            ),
                            p,
                        )),
                ),
            )
        } else if matches!(kind, PlotKind::Line | PlotKind::Bar) {
            let axis_max = self
                .data
                .iter()
                .map(|s| {
                    s.value.max(if cmp {
                        s.comparison.unwrap_or_default()
                    } else {
                        0.
                    })
                })
                .fold(1., f32::max);
            el =
                el.child(
                    row()
                        .gap_1()
                        .w_full()
                        .child(div().flex_1().min_w_0().child(graph))
                        .child(column().h(px(height)).justify_between().children(
                            (0..4).rev().map(|i| {
                                hint(format!("{:.0}", axis_max * i as f32 / 3.), p)
                                    .text_size(px(10.))
                            }),
                        )),
                );
        } else {
            el = el.child(graph)
        };
        if kind == PlotKind::Donut {
            el = el.child(
                column()
                    .gap_0()
                    .children(self.data.iter().enumerate().map(|(i, s)| {
                        let pct = 100. * values[i].0 / sum.max(f32::EPSILON);
                        let text = format!("{}  {:.0}  {:.0}%", s.label, s.value, pct);
                        crate::primitives::family::face(
                            ("slice", i),
                            text,
                            30.,
                            crate::foundation::surface_radius(cx).into(),
                            crate::primitives::family::Tone::Clear,
                            cx,
                        )
                        .on_click(cx.listener(move |v, _, _, cx| {
                            access(v).toggle_segment(i, cx);
                            cx.notify();
                        }))
                        .child(dot(6., rgb(SERIES[i % SERIES.len()]).into()))
                        .child(div().flex_1().child(s.label.clone()))
                        .child(hint(format!("{:.0}", s.value), p))
                        .child(hint(format!("{pct:.0}%"), p).w(px(33.)).text_right())
                        .opacity(if self.hidden.contains(&s.label) {
                            0.5
                        } else {
                            1.
                        })
                        .on_hover(cx.listener(move |v, on: &bool, _, cx| {
                            let plot = access(v);
                            let at = if *on && !plot.hidden.contains(&plot.data[i].label) {
                                Some(i)
                            } else {
                                plot.pinned
                            };
                            inspect(v, at, cx)
                        }))
                        .justify_end()
                        .gap_2()
                        .border_0()
                        .w_full()
                        .h(px(30.))
                        .text_size(px(12.))
                    })),
            )
        } else if kind == PlotKind::Bar && self.data.len() <= 8 {
            el = el.child(
                row().pr_4().justify_between().children(
                    self.data
                        .iter()
                        .map(|s| hint(s.label.clone(), p).text_size(px(10.))),
                ),
            );
        } else if let (Some(first), Some(last)) = (self.data.first(), self.data.last()) {
            el = el.child(
                row()
                    .justify_between()
                    .child(hint(first.label.clone(), p))
                    .child(hint(last.label.clone(), p)),
            )
        }
        if let Some(s) = active.and_then(|i| self.data.get(i)) {
            el = el.child(
                hint(
                    format!(
                        "{}   {:.0}{}",
                        s.label,
                        s.value,
                        s.comparison
                            .filter(|_| cmp)
                            .map_or(String::new(), |x| format!("  /  {x:.0}"))
                    ),
                    p,
                )
                .id("plot-status")
                .role(Role::Status),
            )
        } else if self.data.is_empty() {
            el = el.child(hint(tr(cx, "No data", "暂无数据"), p))
        } else if matches!(kind, PlotKind::Calendar { .. }) {
            el = el.child(
                row()
                    .justify_end()
                    .gap_1()
                    .child(hint(tr(cx, "Less", "少"), p))
                    .children([0.12, 0.32, 0.56, 0.79, 1.].map(|v| {
                        div()
                            .size(px(8.))
                            .rounded(px(2.))
                            .bg(p.stage.blend(p.ink.opacity(v)))
                    }))
                    .child(hint(tr(cx, "More", "多"), p)),
            )
        } else {
            el = el.child(hint(" ", p))
        }
        el
    }
}
fn draw(
    motion: &PlotFrame,
    b: Bounds<Pixels>,
    v: &[(f32, Option<f32>)],
    selected: Option<usize>,
    p: Paint,
    w: &mut Window,
) {
    let kind = motion.kind;
    let comparison = motion.comparison > 0.001;
    let n = v.len();
    if n == 0 {
        return;
    }
    let width = f32::from(b.size.width);
    let height = f32::from(b.size.height);
    if width <= 0. || height <= 0. {
        return;
    }
    let at = |x: f32, y: f32| point(b.left() + px(x), b.top() + px(y));
    let rect = |x: f32, y: f32, wd: f32, ht: f32| {
        Bounds::new(at(x, y), size(px(wd.max(0.)), px(ht.max(0.))))
    };
    let max = v
        .iter()
        .map(|(a, b)| {
            a.max(if comparison {
                b.unwrap_or_default()
            } else {
                0.
            })
        })
        .fold(1., f32::max);
    let top = 5.;
    let extent = (height - 2. * top).max(1.);
    if let PlotKind::Calendar { first_weekday } = kind {
        let cols = (n + first_weekday).div_ceil(7);
        let pitch = width / cols as f32;
        let h = height / 7.;
        let base = p.ink;
        for (i, (value, _)) in v.iter().enumerate() {
            let color = if *value == 0. {
                p.line
            } else {
                p.stage.blend(base.opacity(0.25 + 0.75 * value / max))
            };
            let slot = i + first_weekday;
            let delay = (slot / 7 + slot % 7) as f32 * (0.42 / (cols + 5) as f32).min(0.012);
            let appear = ease((motion.entrance - delay) / 0.36);
            let scale = 0.5 + 0.5 * appear;
            w.paint_quad(
                fill(
                    rect(
                        (pitch * 0.5) * (1. - scale) + ((i + first_weekday) / 7) as f32 * pitch,
                        (h * 0.5) * (1. - scale) + ((i + first_weekday) % 7) as f32 * h,
                        (pitch - 1.8).max(1.) * scale,
                        (h - 1.8).max(1.) * scale,
                    ),
                    (if selected == Some(i) { p.ink } else { color }).opacity(appear),
                )
                .corner_radii(px(2.)),
            )
        }
        return;
    }
    if kind == PlotKind::Donut {
        let total = v.iter().map(|e| e.0).sum::<f32>();
        if total <= 0. {
            return;
        }
        let r = width.min(height);
        let mut start = -PI / 2.;
        for (i, (val, _)) in v.iter().enumerate() {
            if *val <= 0. {
                continue;
            }
            let span = val / total * TAU;
            let gap = (span * 0.12).min(0.016);
            let a = -PI / 2. + (start + PI / 2.) * spring_fraction(motion.entrance, 0.72) + gap;
            let z = -PI / 2.
                + (start + span + PI / 2.)
                    * spring_fraction(motion.entrance - 0.04 - i as f32 * 0.035, 0.72)
                - gap;
            if z <= a {
                start += span;
                continue;
            }
            let weight = motion.weights.get(i).copied().unwrap_or_default();
            let mid = (a + z) * 0.5;
            let ox = mid.cos() * 3. * weight;
            let oy = mid.sin() * 3. * weight;
            let steps = ((z - a) * r / 4.).ceil().max(2.) as usize;
            let mut path = PathBuilder::fill();
            for k in 0..=steps {
                let t = a + (z - a) * k as f32 / steps as f32;
                let rr = 0.47 + 0.02 * weight;
                let pt = at(
                    width / 2. + ox + rr * r * t.cos(),
                    height / 2. + oy + rr * r * t.sin(),
                );
                if k == 0 {
                    path.move_to(pt)
                } else {
                    path.line_to(pt)
                }
            }
            for k in (0..=steps).rev() {
                let t = a + (z - a) * k as f32 / steps as f32;
                path.line_to(at(
                    width / 2. + ox + r * 0.31 * t.cos(),
                    height / 2. + oy + r * 0.31 * t.sin(),
                ))
            }
            path.close();
            if let Ok(path) = path.build() {
                w.paint_path(path, rgb(SERIES[i % 6]))
            }
            start += span;
        }
        return;
    }
    if kind != PlotKind::Spark {
        for i in 0..4 {
            w.paint_quad(fill(
                rect(0., top + i as f32 * extent / 3., width, 1.),
                p.line.opacity(0.65),
            ))
        }
    }
    if kind == PlotKind::Bar {
        for (center, bar_width, value) in &motion.bars {
            let ht = value / max * extent;
            w.paint_quad(
                fill(
                    rect(
                        (center - bar_width / 2.) * width,
                        height - top - ht,
                        bar_width * width,
                        ht,
                    ),
                    if selected.is_some_and(|i| {
                        ((i as f32 + 0.5) / n as f32 - center).abs() < bar_width / 2.
                    }) {
                        p.bright
                    } else {
                        p.secondary
                    },
                )
                .corner_radii(px(3.)),
            )
        }
        return;
    }
    if n == 1 {
        w.paint_quad(
            fill(rect(width / 2. - 2., height / 2. - 2., 4., 4.), p.ink).corner_radii(px(2.)),
        );
        return;
    }
    let low = if kind == PlotKind::Spark {
        v.iter().map(|p| p.0).fold(f32::MAX, f32::min)
    } else {
        0.
    };
    let y = |v: f32| {
        if (max - low).abs() < 0.001 {
            height / 2.
        } else {
            height - top - (v - low) / (max - low) * extent
        }
    };
    let x = |i: usize| 2. + (width - 4.) * i as f32 / n.saturating_sub(1).max(1) as f32;
    let t = (motion.entrance / 0.84).clamp(0., 1.);
    let reveal = crate::primitives::presentation::in_out(t);
    w.with_content_mask(
        Some(ContentMask {
            bounds: rect(0., 0., width * reveal, height),
        }),
        |w| {
            let mut area = PathBuilder::fill();
            area.move_to(at(x(0), height));
            for (i, (value, _)) in v.iter().enumerate() {
                area.line_to(at(x(i), y(*value)));
            }
            area.line_to(at(x(n - 1), height));
            area.close();
            if let Ok(path) = area.build() {
                w.paint_path(
                    path,
                    if kind == PlotKind::Spark {
                        p.success.opacity(0.08)
                    } else {
                        p.ink.opacity(0.09)
                    },
                );
            }
            for second in [true, false] {
                if second && (!comparison || v.iter().all(|e| e.1.is_none())) {
                    continue;
                }
                let mut path = PathBuilder::stroke(px(if second { 1.5 } else { 2. }));
                let mut begun = false;
                for (i, (a, c)) in v.iter().enumerate() {
                    let value = if second { *c } else { Some(*a) };
                    if let Some(value) = value {
                        let pt = at(x(i), y(value));
                        if begun {
                            path.line_to(pt)
                        } else {
                            path.move_to(pt);
                            begun = true
                        }
                    } else {
                        begun = false
                    }
                }
                if let Ok(path) = path.build() {
                    w.paint_path(
                        path,
                        if second {
                            p.faint.opacity(motion.comparison)
                        } else if kind == PlotKind::Spark {
                            p.success
                        } else {
                            p.ink
                        },
                    )
                }
            }
        },
    );
    if motion.cursor_alpha > 0.001 {
        let position = motion.cursor.clamp(0., 1.) * (n - 1) as f32;
        let i = position.floor() as usize;
        let yy = y(v[i].0 + (v[(i + 1).min(n - 1)].0 - v[i].0) * position.fract());
        let x = x(0) + (x(n - 1) - x(0)) * position / (n - 1) as f32;
        w.paint_quad(fill(
            rect(x - 0.5, 0., 1., height),
            p.secondary.opacity(0.45 * motion.cursor_alpha),
        ));
        w.paint_quad(
            fill(
                rect(x - 3., yy - 3., 6., 6.),
                p.ink.opacity(motion.cursor_alpha),
            )
            .corner_radii(px(3.)),
        );
    }
}
