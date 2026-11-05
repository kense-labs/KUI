//! Conversion funnel with segment filtering and keyboard inspection.
use crate::primitives::{
    family::{Paint, Slide, Tone, disabled_motion, face},
    presentation::{column, hint, row, title, tr},
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

#[derive(Clone, Debug)]
pub struct FunnelSegment {
    pub id: SharedString,
    pub label: SharedString,
}
impl FunnelSegment {
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
        }
    }
}
/// Values correspond to segment order; invalid and negative numbers count as zero.
#[derive(Clone, Debug)]
pub struct FunnelStage {
    pub id: SharedString,
    pub label: SharedString,
    pub values: Vec<f32>,
}
impl FunnelStage {
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        values: Vec<f32>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            values,
        }
    }
    fn value(&self, segment: Option<usize>) -> f32 {
        self.value_f64(segment).min(f64::from(f32::MAX)) as f32
    }
    fn value_f64(&self, segment: Option<usize>) -> f64 {
        match segment {
            Some(i) => self
                .values
                .get(i)
                .copied()
                .filter(|v| v.is_finite())
                .unwrap_or(0.)
                .max(0.) as f64,
            None => self
                .values
                .iter()
                .copied()
                .filter(|v| v.is_finite())
                .map(|v| f64::from(v.max(0.)))
                .sum::<f64>(),
        }
    }
}
#[derive(Clone, Debug)]
pub struct FunnelInspect {
    pub id: SharedString,
    pub value: f32,
    pub from_start: f32,
    pub from_previous: f32,
}
struct MovingStage {
    id: SharedString,
    first: Slide,
    second: Slide,
}
pub struct FunnelChart {
    title: SharedString,
    stages: Vec<FunnelStage>,
    segments: Vec<FunnelSegment>,
    active_segment: Option<usize>,
    split: bool,
    flow: bool,
    gap: Slide,
    reveal: Slide,
    revealed: bool,
    phase: f32,
    last_tick: Option<Instant>,
    visible: bool,
    active: Option<usize>,
    visual: Vec<MovingStage>,
    bounds: Rc<Cell<Bounds<Pixels>>>,
    initialized: bool,
}
impl EventEmitter<FunnelInspect> for FunnelChart {}
impl FunnelChart {
    pub fn new(
        title: impl Into<SharedString>,
        stages: Vec<FunnelStage>,
        segments: Vec<FunnelSegment>,
    ) -> Self {
        Self {
            title: title.into(),
            stages,
            segments,
            active_segment: None,
            split: false,
            flow: true,
            gap: Slide::new(0.),
            reveal: Slide::new(0.),
            revealed: false,
            phase: 0.,
            last_tick: None,
            visible: true,
            active: None,
            visual: vec![],
            bounds: Default::default(),
            initialized: false,
        }
    }
    /// Configure whether flow particles are drawn.
    pub fn with_flow(mut self, flow: bool) -> Self {
        self.flow = flow;
        self
    }
    /// Pause or resume particles without resetting their phase.
    pub fn set_flow(&mut self, flow: bool, cx: &mut Context<Self>) {
        if self.flow != flow {
            self.flow = flow;
            self.last_tick = None;
            cx.notify();
        }
    }
    /// Pause flow particles and paint scheduling while the embedding host is hidden.
    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        if self.visible != visible {
            self.visible = visible;
            self.last_tick = None;
            cx.notify();
        }
    }
    pub fn set_stages(&mut self, stages: Vec<FunnelStage>, cx: &mut Context<Self>) {
        self.stages = stages;
        self.active = None;
        self.reconcile(cx);
        cx.notify();
    }
    fn reconcile(&mut self, cx: &App) {
        let scale = self
            .stages
            .iter()
            .map(|stage| stage.value_f64(self.active_segment))
            .fold(1_f64, f64::max);
        let mut old: Vec<Option<MovingStage>> = std::mem::take(&mut self.visual)
            .into_iter()
            .map(Some)
            .collect();
        self.visual = self
            .stages
            .iter()
            .map(|stage| {
                let total = stage.value_f64(self.active_segment);
                let first = if self.split {
                    stage.value_f64(Some(0))
                } else {
                    total / 2.
                };
                let second = if self.split {
                    stage.value_f64(Some(1))
                } else {
                    total / 2.
                };
                let first = (first / scale) as f32;
                let second = (second / scale) as f32;
                let mut v = old
                    .iter()
                    .position(|v| v.as_ref().is_some_and(|v| v.id == stage.id))
                    .and_then(|i| old[i].take())
                    .unwrap_or_else(|| MovingStage {
                        id: stage.id.clone(),
                        first: Slide::new(if self.initialized { 0. } else { first }),
                        second: Slide::new(if self.initialized { 0. } else { second }),
                    });
                v.first.duration(first, 0.55, cx);
                v.second.duration(second, 0.55, cx);
                v
            })
            .collect();
        self.initialized = true;
        self.gap
            .duration(if self.split { 1. } else { 0. }, 0.55, cx);
    }
    fn select(&mut self, next: Option<usize>, cx: &mut Context<Self>) {
        if self.active == next {
            return;
        }
        self.active = next;
        if let Some((index, stage)) = next.and_then(|i| self.stages.get(i).map(|s| (i, s))) {
            let value = stage.value(self.active_segment);
            let first = self
                .stages
                .first()
                .map_or(0., |s| s.value(self.active_segment));
            let previous = if index == 0 {
                first
            } else {
                self.stages[index - 1].value(self.active_segment)
            };
            cx.emit(FunnelInspect {
                id: stage.id.clone(),
                value,
                from_start: if first > 0. { value / first } else { 0. },
                from_previous: if previous > 0. { value / previous } else { 0. },
            });
        }
        cx.notify();
    }
    fn segment(&mut self, segment: Option<usize>, cx: &mut Context<Self>) {
        if self.active_segment == segment && !self.split {
            return;
        }
        self.active_segment = segment;
        self.split = false;
        self.reconcile(cx);
        cx.notify();
    }
    fn split(&mut self, cx: &mut Context<Self>) {
        if self.split || self.segments.len() != 2 {
            return;
        }
        self.active_segment = None;
        self.split = true;
        self.reconcile(cx);
        cx.notify();
    }
}
impl Render for FunnelChart {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.initialized {
            self.reconcile(cx);
        }
        let p = Paint::get(cx);
        let current_bounds = self.bounds.get();
        let measured_width = f32::from(current_bounds.size.width);
        let narrow = measured_width > 0. && measured_width < 420.;
        let visible = self.visible
            && w.is_visible()
            && intersects_viewport(current_bounds, w.viewport_size());
        if !self.revealed && visible {
            self.revealed = true;
            self.reveal.duration(1., 0.8, cx);
        }
        let now = cx.background_executor().now();
        let reduced = disabled_motion(cx);
        let mut moving = false;
        let mut sample = |slide: &Slide| {
            let (value, active) = slide.sample_batch(now, reduced);
            moving |= active;
            value
        };
        let values: Vec<(f32, f32)> = self
            .visual
            .iter()
            .map(|stage| (sample(&stage.first), sample(&stage.second)))
            .collect();
        let n = values.len();
        let max = values.iter().fold(1_f32, |a, (x, y)| a.max(x + y));
        let gap = sample(&self.gap);
        let reveal = sample(&self.reveal).clamp(0., 1.);
        let flowing = self.flow && !reduced && visible && reveal > 0.75;
        if flowing {
            if let Some(last) = self.last_tick {
                self.phase = (self.phase
                    + now.saturating_duration_since(last).as_secs_f32().min(0.1) / 4.)
                    .fract();
            }
            self.last_tick = Some(now);
        } else {
            self.last_tick = None;
        }
        let phase = self.phase;
        if visible && (moving || flowing) {
            w.request_animation_frame();
        }
        let active = self.active;
        let inspected = self.active.or((!self.stages.is_empty()).then_some(0));
        let graph_label = inspected
            .and_then(|i| {
                self.stages.get(i).map(|stage| {
                    let value = stage.value(self.active_segment);
                    let first = self
                        .stages
                        .first()
                        .map_or(0., |stage| stage.value(self.active_segment));
                    let previous = if i == 0 {
                        first
                    } else {
                        self.stages[i - 1].value(self.active_segment)
                    };
                    format!(
                        "{}: {} {:.0}, {} {:.1}%, {} {:.1}%. {}",
                        self.title,
                        stage.label,
                        value,
                        tr(cx, "of start", "占起始"),
                        if first > 0. { value / first * 100. } else { 0. },
                        tr(cx, "of previous", "占上一阶段"),
                        if previous > 0. {
                            value / previous * 100.
                        } else {
                            0.
                        },
                        tr(
                            cx,
                            "Use arrow keys to inspect stages",
                            "使用方向键查看各阶段"
                        )
                    )
                })
            })
            .unwrap_or_else(|| format!("{}: {}", self.title, tr(cx, "No data", "暂无数据")));
        let bounds = self.bounds.clone();
        let mut graph = div()
            .id("funnel-chart")
            .role(Role::Figure)
            .tab_index(0)
            .w_full()
            .relative()
            .h(px(if narrow {
                84. * self.stages.len().max(1) as f32
            } else {
                200.
            }))
            .aria_label(graph_label)
            .on_hover(cx.listener(|s, h: &bool, _, cx| {
                if !h {
                    s.select(None, cx)
                }
            }))
            .on_mouse_move(cx.listener(|s, e: &MouseMoveEvent, _, cx| {
                let b = s.bounds.get();
                let width = f32::from(b.size.width).max(1.);
                if s.stages.is_empty() {
                    return;
                }
                let x = if width < 420. {
                    (f32::from(e.position.y - b.origin.y) / f32::from(b.size.height).max(1.))
                        .clamp(0., 1.)
                } else {
                    (f32::from(e.position.x - b.origin.x) / width).clamp(0., 1.)
                };
                let index = (x * (s.stages.len().saturating_sub(1)) as f32).round() as usize;
                s.select(Some(index), cx);
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|s, e: &MouseDownEvent, _, cx| {
                    let b = s.bounds.get();
                    if s.stages.is_empty() {
                        return;
                    }
                    let width = f32::from(b.size.width).max(1.);
                    let position = if width < 420. {
                        f32::from(e.position.y - b.origin.y) / f32::from(b.size.height).max(1.)
                    } else {
                        f32::from(e.position.x - b.origin.x) / width
                    };
                    let index = (position.clamp(0., 1.) * s.stages.len().saturating_sub(1) as f32)
                        .round() as usize;
                    s.select(Some(index), cx);
                }),
            )
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                let n = s.stages.len();
                if n == 0 {
                    return;
                }
                let next = match e.keystroke.key.as_str() {
                    "right" | "down" => Some(s.active.map_or(0, |i| (i + 1).min(n - 1))),
                    "left" | "up" => Some(s.active.map_or(0, |i| i.saturating_sub(1))),
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
                    move |b, window, _| {
                        let before = bounds.replace(b);
                        if before.size != b.size
                            || intersects_viewport(before, window.viewport_size())
                                != intersects_viewport(b, window.viewport_size())
                        {
                            window.request_animation_frame();
                        }
                    },
                    move |b, _, w, _| {
                        if n < 2 {
                            return;
                        }
                        let width = f32::from(b.size.width);
                        let height = f32::from(b.size.height);
                        let vertical = width < 420.;
                        let main = if vertical { height } else { width };
                        let cross = if vertical {
                            (width * 0.44).clamp(120., 220.)
                        } else {
                            height
                        };
                        let shift = if vertical {
                            (width - cross) * 0.5
                        } else {
                            0.
                        };
                        let at = |t: f32, offset: f32| {
                            if vertical {
                                point(
                                    b.origin.x + px(shift + cross / 2. + offset),
                                    b.origin.y + px(main * t),
                                )
                            } else {
                                point(
                                    b.origin.x + px(main * t),
                                    b.origin.y + px(cross / 2. + offset),
                                )
                            }
                        };
                        let radius = ((cross - 16.) * 0.5 - gap).max(1.) * reveal;
                        for side in 0..2 {
                            let sign = if side == 0 { -1. } else { 1. };
                            let val = |i: usize| if side == 0 { values[i].0 } else { values[i].1 };
                            let inner = sign * gap;
                            let edge = |i: usize| inner + sign * val(i) / max * radius;
                            let edges: Vec<f32> = (0..n).map(edge).collect();
                            let t = |i: usize| i as f32 / (n - 1) as f32;
                            let edge_at = |u: f32| funnel_sample(&edges, u * main, main);
                            let samples = (n - 1) * 12;
                            let mut band = PathBuilder::fill();
                            band.move_to(at(t(0), inner));
                            for i in 0..=samples {
                                let u = i as f32 / samples as f32;
                                band.line_to(at(u, edge_at(u)));
                            }
                            band.line_to(at(1., inner));
                            band.close();
                            let color = if side == 0 {
                                p.bright
                            } else {
                                p.bright.blend(p.success.opacity(gap * 0.7))
                            };
                            if let Ok(path) = band.build() {
                                w.paint_path(path, color.opacity(0.35));
                            }
                            let mut line = PathBuilder::stroke(px(2.));
                            line.move_to(at(0., edge(0)));
                            for i in 1..=samples {
                                let u = i as f32 / samples as f32;
                                line.line_to(at(u, edge_at(u)));
                            }
                            if let Ok(path) = line.build() {
                                w.paint_path(path, color);
                            }
                            if flowing {
                                for dot in 0..16 {
                                    let u = (phase + dot as f32 / 16.).fract();
                                    let lane = 0.18 + ((dot * 37 % 67) as f32) / 100.;
                                    let cutoff = (1..n).find_map(|stage| {
                                        let before = val(stage - 1);
                                        let kept = if before > 0. {
                                            (val(stage) / before).clamp(0., 1.)
                                        } else {
                                            0.
                                        };
                                        let draw = ((dot * 37 + stage * 71) % 97) as f32 / 97.;
                                        (draw > kept)
                                            .then_some((stage as f32 - 0.5) / (n - 1) as f32)
                                    });
                                    let exit =
                                        cutoff.map_or(0., |cut| ((u - cut) / 0.12).clamp(0., 1.));
                                    if exit >= 1. {
                                        continue;
                                    }
                                    let travel = cutoff.map_or(u, |cut| u.min(cut));
                                    let depth = if exit > 0. {
                                        edge_at(travel) + sign * exit * 20.
                                    } else {
                                        inner + (edge_at(travel) - inner) * lane
                                    };
                                    let center = at(u, depth);
                                    w.paint_quad(
                                        fill(
                                            Bounds {
                                                origin: point(
                                                    center.x - px(1.5),
                                                    center.y - px(1.5),
                                                ),
                                                size: size(px(3.), px(3.)),
                                            },
                                            color.opacity(0.7 * (1. - exit)),
                                        )
                                        .corner_radii(px(1.5)),
                                    );
                                }
                            }
                        }
                        if let Some(i) = active.filter(|i| *i < n) {
                            let t = i as f32 / (n - 1) as f32;
                            let mut line = PathBuilder::stroke(px(1.));
                            line.move_to(at(t, -gap - values[i].0 / max * radius));
                            line.line_to(at(t, gap + values[i].1 / max * radius));
                            if let Ok(path) = line.build() {
                                w.paint_path(path, p.ink.opacity(0.65));
                            }
                        }
                    },
                )
                .size_full(),
            )
            .children(if narrow && n > 1 {
                self.stages
                    .iter()
                    .enumerate()
                    .map(|(i, stage)| {
                        div()
                            .absolute()
                            .right_0()
                            .top(relative((i as f32 / (n - 1) as f32 * 0.88).min(0.88)))
                            .w(relative(0.28))
                            .text_size(px(12.))
                            .line_height(px(16.))
                            .text_color(p.secondary)
                            .child(stage.label.clone())
                    })
                    .collect::<Vec<_>>()
            } else {
                vec![]
            });
        if let Some(index) = self.active
            && let Some(stage) = self.stages.get(index)
        {
            let count = self.stages.len().max(1) as f32;
            let value = stage.value(self.active_segment);
            let first = self
                .stages
                .first()
                .map_or(0., |stage| stage.value(self.active_segment));
            let previous = if index == 0 {
                first
            } else {
                self.stages[index - 1].value(self.active_segment)
            };
            let along = (index as f32 + 0.5) / count;
            graph = graph.child(
                div()
                    .absolute()
                    .when(narrow, |tip| {
                        tip.left(px(8.)).top(relative(along.clamp(0.04, 0.72)))
                    })
                    .when(!narrow, |tip| {
                        tip.left(relative(along.clamp(0.02, 0.58)))
                            .top(px(8.))
                    })
                    .min_w(px(168.))
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
                            .child(stage.label.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(14.))
                            .line_height(px(18.))
                            .text_color(p.ink)
                            .child(format!("{value:.0}")),
                    )
                    .child(
                        div()
                            .mt(px(2.))
                            .text_size(px(12.))
                            .line_height(px(20.))
                            .text_color(p.faint)
                            .child(format!(
                                "{:.1}% / {:.1}%",
                                if first > 0. { value / first * 100. } else { 0. },
                                if previous > 0. {
                                    value / previous * 100.
                                } else {
                                    0.
                                }
                            )),
                    ),
            );
        }
        let detail = self
            .active
            .and_then(|i| {
                self.stages.get(i).map(|s| {
                    let value = s.value(self.active_segment);
                    if self.split && self.segments.len() == 2 {
                        return format!(
                            "{}  ·  {} {:.0}  ·  {} {:.0}",
                            s.label,
                            self.segments[0].label,
                            s.value(Some(0)),
                            self.segments[1].label,
                            s.value(Some(1))
                        );
                    }
                    let first = self
                        .stages
                        .first()
                        .map_or(0., |v| v.value(self.active_segment));
                    let previous = if i == 0 {
                        first
                    } else {
                        self.stages[i - 1].value(self.active_segment)
                    };
                    format!(
                        "{}  {:.0}  ·  {:.1}% / {:.1}%",
                        s.label,
                        value,
                        if first > 0. { value / first * 100. } else { 0. },
                        if previous > 0. {
                            value / previous * 100.
                        } else {
                            0.
                        }
                    )
                })
            })
            .unwrap_or_else(|| {
                tr(
                    cx,
                    "Select a stage to inspect conversion",
                    "选择阶段查看转化率",
                )
                .to_string()
            });
        let all = tr(cx, "All", "全部");
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
            .child(title(self.title.clone(), p))
            .child(
                row()
                    .flex_wrap()
                    .gap_2()
                    .child(
                        face(
                            "funnel-all",
                            all,
                            32.,
                            crate::foundation::surface_radius(cx).into(),
                            Tone::Clear,
                            cx,
                        )
                        .px(px(10.))
                        .aria_toggled((self.active_segment.is_none() && !self.split).into())
                        .when(self.active_segment.is_none() && !self.split, |e| {
                            e.bg(p.hover)
                        })
                        .child(all)
                        .on_click(cx.listener(|s, _, _, cx| s.segment(None, cx)))
                        .on_key_down(cx.listener(
                            |s, e: &KeyDownEvent, w, cx| {
                                if matches!(e.keystroke.key.as_str(), "enter" | "space")
                                    && !e.is_held
                                {
                                    s.segment(None, cx);
                                    w.prevent_default();
                                    cx.stop_propagation();
                                }
                            },
                        )),
                    )
                    .when(self.segments.len() == 2, |row| {
                        row.child(
                            face(
                                "funnel-split",
                                tr(cx, "Compare", "对比"),
                                32.,
                                crate::foundation::surface_radius(cx).into(),
                                Tone::Clear,
                                cx,
                            )
                            .px(px(10.))
                            .aria_toggled(self.split.into())
                            .when(self.split, |e| e.bg(p.hover))
                            .child(tr(cx, "Compare", "对比"))
                            .on_click(cx.listener(|s, _, _, cx| s.split(cx)))
                            .on_key_down(cx.listener(
                                |s, e: &KeyDownEvent, w, cx| {
                                    if matches!(e.keystroke.key.as_str(), "enter" | "space")
                                        && !e.is_held
                                    {
                                        s.split(cx);
                                        w.prevent_default();
                                        cx.stop_propagation();
                                    }
                                },
                            )),
                        )
                    })
                    .children(self.segments.iter().enumerate().map(|(i, s)| {
                        face(
                            ("funnel-segment", i),
                            s.label.clone(),
                            32.,
                            crate::foundation::surface_radius(cx).into(),
                            Tone::Clear,
                            cx,
                        )
                        .px(px(10.))
                        .aria_toggled((!self.split && self.active_segment == Some(i)).into())
                        .when(!self.split && self.active_segment == Some(i), |e| {
                            e.bg(p.hover)
                        })
                        .child(s.label.clone())
                        .on_click(cx.listener(move |s, _, _, cx| s.segment(Some(i), cx)))
                        .on_key_down(cx.listener(
                            move |s, e: &KeyDownEvent, w, cx| {
                                if matches!(e.keystroke.key.as_str(), "enter" | "space")
                                    && !e.is_held
                                {
                                    s.segment(Some(i), cx);
                                    w.prevent_default();
                                    cx.stop_propagation();
                                }
                            },
                        ))
                    }))
                    .child({
                        let label = if self.flow {
                            tr(cx, "Pause flow", "停止流动")
                        } else {
                            tr(cx, "Resume flow", "恢复流动")
                        };
                        face(
                            "funnel-flow",
                            label,
                            32.,
                            crate::foundation::surface_radius(cx).into(),
                            Tone::Clear,
                            cx,
                        )
                        .px(px(10.))
                        .child(label)
                        .on_click(cx.listener(|s, _, _, cx| {
                            s.flow = !s.flow;
                            cx.notify();
                        }))
                        .on_key_down(cx.listener(
                            |s, e: &KeyDownEvent, w, cx| {
                                if matches!(e.keystroke.key.as_str(), "enter" | "space")
                                    && !e.is_held
                                {
                                    s.flow = !s.flow;
                                    cx.notify();
                                    w.prevent_default();
                                    cx.stop_propagation();
                                }
                            },
                        ))
                    }),
            )
            .child(graph)
            .child(hint(detail, p))
            .child(row().justify_between().flex_wrap().gap_2().children(
                self.stages.iter().enumerate().map(|(i, s)| {
                    face(
                        ("funnel-stage", i),
                        s.label.clone(),
                        32.,
                        crate::foundation::surface_radius(cx).into(),
                        Tone::Clear,
                        cx,
                    )
                    .px(px(10.))
                    .child(s.label.clone())
                    .on_click(cx.listener(move |s, _, _, cx| s.select(Some(i), cx)))
                    .on_key_down(cx.listener(
                        move |s, e: &KeyDownEvent, w, cx| {
                            if matches!(e.keystroke.key.as_str(), "enter" | "space") && !e.is_held {
                                s.select(Some(i), cx);
                                w.prevent_default();
                                cx.stop_propagation();
                            }
                        },
                    ))
                }),
            ))
    }
}

/// Stage values hold through the first and last half-slot. The join eases from 0.26 to 0.74.
fn funnel_sample(values: &[f32], x: f32, length: f32) -> f32 {
    let count = values.len();
    if count == 0 || length <= 0. {
        return 0.;
    }
    let n = x / (length / count as f32) - 0.5;
    if n <= 0. {
        return values[0];
    }
    let last = count - 1;
    if n >= last as f32 {
        return values[last];
    }
    let index = n.floor() as usize;
    let fraction = n - index as f32;
    let t = if fraction <= 0.26 {
        0.
    } else if fraction >= 0.74 {
        1.
    } else {
        let r = (fraction - 0.26) / 0.48;
        r * r * r * (r * (r * 6. - 15.) + 10.)
    };
    values[index] + (values[index + 1] - values[index]) * t
}
fn intersects_viewport(bounds: Bounds<Pixels>, viewport: Size<Pixels>) -> bool {
    bounds.size.width > px(0.)
        && bounds.size.height > px(0.)
        && bounds.left() < viewport.width
        && bounds.top() < viewport.height
        && bounds.right() > px(0.)
        && bounds.bottom() > px(0.)
}

#[cfg(test)]
mod tests {
    use super::FunnelStage;
    #[test]
    fn finite_segments_are_summed_before_visual_normalization() {
        let stage = FunnelStage::new("large", "Large", vec![f32::MAX, f32::MAX]);
        let scale = stage.value_f64(None);
        assert!(scale.is_finite());
        assert!((stage.value_f64(Some(0)) / scale - 0.5).abs() < 1e-12);
        assert!((stage.value_f64(Some(1)) / scale - 0.5).abs() < 1e-12);
    }
}
