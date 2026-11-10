//! Follow the steps of an agent run and inspect individual results.
use crate::primitives::{family::Paint, ops_surface::*};
use gpui::{prelude::*, *};
use web_time::Instant;

const COLORS: [u32; 5] = [0x7594cb, 0x61ad94, 0xcd9b54, 0x9c83b4, 0xc67a79];
const ROW: f32 = 44.;

#[derive(Clone)]
pub struct Span {
    pub lane: usize,
    pub start: f32,
    pub end: f32,
    pub detail: String,
    pub kind: usize,
    pub parent: Option<usize>,
}

pub struct RunLanes {
    id: SharedString,
    pub title: String,
    pub lanes: Vec<String>,
    pub spans: Vec<Span>,
    models: Vec<SharedString>,
    totals: Option<SharedString>,
    cursor: f32,
    last: Option<Instant>,
    initialized: bool,
    speed: f32,
    selected: Option<usize>,
    seek_bounds: Bounds<Pixels>,
    seek_focus: Option<FocusHandle>,
    seeking: bool,
}

pub struct SpanSelected(pub usize);
impl EventEmitter<SpanSelected> for RunLanes {}

impl RunLanes {
    pub fn new(
        id: impl Into<SharedString>,
        title: impl Into<String>,
        lanes: Vec<String>,
        spans: Vec<Span>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            lanes,
            spans,
            models: Vec::new(),
            totals: None,
            cursor: 0.,
            last: None,
            initialized: false,
            speed: 1.,
            selected: None,
            seek_bounds: Bounds::default(),
            seek_focus: None,
            seeking: false,
        }
    }

    /// Optional labels paired by lane index, for example model names.
    pub fn with_models(mut self, models: Vec<SharedString>) -> Self {
        self.models = models;
        self
    }

    /// Optional host-formatted run totals, such as tokens and cost.
    pub fn with_totals(mut self, totals: impl Into<SharedString>) -> Self {
        self.totals = Some(totals.into());
        self
    }

    fn seek(&mut self, position: f32, end: f32, cx: &mut Context<Self>) {
        self.last = None;
        self.cursor = position.clamp(0., end);
        cx.notify();
    }
}

impl Render for RunLanes {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let end = self
            .spans
            .iter()
            .map(|s| s.end)
            .filter(|s| s.is_finite())
            .fold(1., f32::max);
        if !self.initialized {
            self.initialized = true;
            if crate::primitives::family::disabled_motion(cx) {
                self.cursor = end;
            } else {
                self.last = Some(cx.background_executor().now());
            }
        }
        if let Some(prev) = self.last {
            let now = cx.background_executor().now();
            if w.is_visible() {
                let elapsed = now.saturating_duration_since(prev).as_secs_f32().min(0.1);
                self.cursor = (self.cursor + elapsed * self.speed).min(end);
                w.request_animation_frame();
            }
            self.last = (self.cursor < end).then_some(now);
        }
        let p = Paint::get(cx);
        let radius = crate::foundation::surface_radius(cx);
        let compact = w.viewport_size().width < px(500.);
        let label_width = if compact { 94. } else { 176. };
        let cursor = self.cursor / end;
        let mut labels = column().w(px(label_width)).flex_none().gap_0();
        let mut plot = column().relative().flex_1().gap_0().min_w_0();
        for (lane, name) in self.lanes.iter().enumerate() {
            let model = self.models.get(lane);
            labels = labels.child(
                row()
                    .h(px(ROW))
                    .pr_2()
                    .child(
                        div()
                            .size(px(7.))
                            .flex_none()
                            .rounded_full()
                            .bg(rgb(COLORS[lane % COLORS.len()])),
                    )
                    .child(
                        column()
                            .gap_0()
                            .flex_1()
                            .child(div().truncate().text_size(px(12.)).child(name.clone()))
                            .when_some(model, |v, model| {
                                v.child(note(model.clone(), cx).truncate())
                            }),
                    ),
            );
            let mut rail = div()
                .relative()
                .w_full()
                .h(px(ROW))
                .border_b_1()
                .border_color(p.line);
            for (index, span) in self.spans.iter().enumerate().filter(|(_, s)| {
                s.lane == lane
                    && s.start.is_finite()
                    && s.end.is_finite()
                    && s.start < s.end
                    && s.end > 0.
            }) {
                let c = rgb(COLORS[span.kind % COLORS.len()]);
                let selected = self.selected == Some(index)
                    || self
                        .selected
                        .and_then(|i| self.spans.get(i))
                        .and_then(|v| v.parent)
                        == Some(index);
                let start = span.start.clamp(0., end);
                let finish = span.end.clamp(start, end);
                rail = rail.child(
                    item(("span", index), span.detail.clone(), cx, move |v, _, cx| {
                        v.selected = Some(index);
                        cx.emit(SpanSelected(index));
                        cx.notify();
                    })
                    .aria_selected(selected)
                    .absolute()
                    .top(px(12.))
                    .left(relative(start / end))
                    .w(relative((finish - start) / end))
                    .h(px(20.))
                    .p_0()
                    .rounded(radius)
                    .overflow_hidden()
                    .bg(c.opacity(if selected { 0.36 } else { 0.16 }))
                    .child(
                        div()
                            .h_full()
                            .w(relative(
                                ((self.cursor - start) / (finish - start).max(0.001)).clamp(0., 1.),
                            ))
                            .bg(c.opacity(if selected { 1. } else { 0.76 })),
                    ),
                );
            }
            plot = plot.child(rail);
        }
        let links: Vec<_> = self
            .spans
            .iter()
            .enumerate()
            .filter_map(|(index, span)| {
                let parent = span.parent.and_then(|index| self.spans.get(index))?;
                let progress = ((self.cursor - span.start) / 0.36).clamp(0., 1.);
                (progress > 0.
                    && parent.end.is_finite()
                    && span.lane < self.lanes.len()
                    && parent.lane < self.lanes.len())
                .then_some((
                    parent.end.clamp(0., end) / end,
                    parent.lane,
                    span.start.clamp(0., end) / end,
                    span.lane,
                    progress,
                    self.selected == Some(index),
                ))
            })
            .collect();
        plot = plot.child(
            canvas(
                |_, _, _| (),
                move |b, _, w, _| {
                    let pos = |fraction, lane| {
                        point(
                            b.left() + b.size.width * fraction,
                            b.top() + px(ROW * (lane as f32 + 0.5)),
                        )
                    };
                    for &(start, a, finish, z, t, selected) in &links {
                        let from = pos(start, a);
                        let to = pos(finish, z);
                        let middle = point(to.x, from.y);
                        let mut path = PathBuilder::stroke(px(if selected { 2. } else { 1. }));
                        path.move_to(from);
                        path.line_to(from + (middle - from) * (2. * t).min(1.));
                        if t > 0.5 {
                            path.line_to(middle + (to - middle) * (2. * t - 1.));
                        }
                        if let Ok(path) = path.build() {
                            w.paint_path(
                                path,
                                p.secondary.opacity(if selected { 0.88 } else { 0.52 }),
                            );
                        }
                    }
                    let x = b.left() + (b.size.width - px(1.)).max(px(0.)) * cursor;
                    w.paint_quad(fill(
                        Bounds::new(point(x, b.top()), size(px(1.), b.size.height)),
                        p.ink.opacity(0.70),
                    ));
                },
            )
            .absolute()
            .inset_0()
            .size_full(),
        );

        let ticks = if compact { 2 } else { 4 };
        let mut axis = row().w_full().justify_between();
        for index in 0..=ticks {
            axis = axis.child(note(
                format!("{:.0}s", end * index as f32 / ticks as f32),
                cx,
            ));
        }
        let focus = self
            .seek_focus
            .get_or_insert_with(|| cx.focus_handle().tab_stop(true))
            .clone();
        let seek_label = tr(cx, "Run position", "运行进度");
        let view = cx.weak_entity();
        let seek = div()
            .id("run-position")
            .track_focus(&focus)
            .tab_index(0)
            .role(Role::Slider)
            .aria_label(seek_label)
            .aria_value(format!("{:.1} / {:.1}s", self.cursor, end))
            .relative()
            .w_full()
            .h(px(24.))
            .cursor_pointer()
            .focus_visible(|v| v.bg(p.hover).rounded(radius))
            .child(
                div()
                    .absolute()
                    .left_0()
                    .right_0()
                    .top(px(11.))
                    .h(px(2.))
                    .bg(p.line)
                    .child(div().w(relative(cursor)).h_full().bg(p.secondary)),
            )
            .child(
                div()
                    .absolute()
                    .left(relative(cursor))
                    .ml(px(-4.))
                    .top(px(8.))
                    .size(px(8.))
                    .rounded_full()
                    .bg(p.ink),
            )
            .child(
                canvas(
                    move |b, _, cx| {
                        let _ = view.update(cx, |v, _| v.seek_bounds = b);
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .inset_0()
                .size_full(),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |v, e: &MouseDownEvent, w, cx| {
                    focus.focus(w, cx);
                    v.seeking = true;
                    let b = v.seek_bounds;
                    if b.size.width > px(0.) {
                        v.seek(end * ((e.position.x - b.left()) / b.size.width), end, cx);
                    }
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|v, _, _, _| v.seeking = false),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|v, _, _, _| v.seeking = false),
            )
            .on_mouse_move(cx.listener(move |v, e: &MouseMoveEvent, _, cx| {
                let b = v.seek_bounds;
                if v.seeking && e.pressed_button == Some(MouseButton::Left) && b.size.width > px(0.)
                {
                    v.seek(end * ((e.position.x - b.left()) / b.size.width), end, cx);
                }
            }))
            .on_key_down(cx.listener(move |v, e: &KeyDownEvent, w, cx| {
                let position = match e.keystroke.key.as_str() {
                    "left" | "down" => v.cursor - 1.,
                    "right" | "up" => v.cursor + 1.,
                    "home" => 0.,
                    "end" => end,
                    _ => return,
                };
                v.seek(position, end, cx);
                cx.stop_propagation();
                w.prevent_default();
            }));
        let mut legend = row().flex_wrap().gap(px(12.));
        for (index, label) in [
            tr(cx, "Reasoning", "推理"),
            tr(cx, "Tool", "工具"),
            tr(cx, "Output", "输出"),
            tr(cx, "Handoff", "转接"),
            tr(cx, "Error", "错误"),
        ]
        .into_iter()
        .enumerate()
        {
            if self.spans.iter().any(|s| s.kind % COLORS.len() == index) {
                legend = legend.child(
                    row()
                        .gap_1()
                        .child(div().size(px(7.)).rounded_full().bg(rgb(COLORS[index])))
                        .child(note(label, cx)),
                );
            }
        }
        panel(&self.id, 669., cx)
            .p(px(if compact { 12. } else { 24. }))
            .child(
                row()
                    .justify_between()
                    .flex_wrap()
                    .child(title(self.title.clone()))
                    .child(
                        row()
                            .flex_wrap()
                            .gap_2()
                            .child(
                                note(format!("{:.1}s / {:.1}s", self.cursor, end), cx)
                                    .w(px(96.))
                                    .text_right(),
                            )
                            .when_some(self.totals.clone(), |v, totals| v.child(note(totals, cx))),
                    ),
            )
            .child(row().items_start().gap_0().child(labels).child(plot))
            .child(
                row()
                    .gap_0()
                    .child(div().w(px(label_width)).flex_none())
                    .child(column().flex_1().gap_0().child(axis).child(seek)),
            )
            .child(
                row().flex_wrap().justify_between().child(legend).child(
                    row()
                        .child(btn(
                            "play",
                            if self.last.is_some() {
                                tr(cx, "Pause", "暂停")
                            } else {
                                tr(cx, "Play", "播放")
                            },
                            false,
                            cx,
                            move |s, _, cx| {
                                if s.last.take().is_none() {
                                    if s.cursor >= end {
                                        s.cursor = 0.;
                                    }
                                    s.last = Some(cx.background_executor().now());
                                }
                                cx.notify();
                            },
                        ))
                        .child(btn(
                            "rewind",
                            tr(cx, "Replay", "重播"),
                            false,
                            cx,
                            |s, _, cx| {
                                s.cursor = 0.;
                                s.last = Some(cx.background_executor().now());
                                cx.notify();
                            },
                        ))
                        .child(btn(
                            "speed",
                            format!("{}×", self.speed),
                            false,
                            cx,
                            |s, _, cx| {
                                s.speed = match s.speed {
                                    0.5 => 1.,
                                    1. => 2.,
                                    _ => 0.5,
                                };
                                cx.notify();
                            },
                        )),
                ),
            )
            .when_some(self.selected.and_then(|i| self.spans.get(i)), |v, s| {
                v.child(note(
                    format!("{:.1}–{:.1}s · {}", s.start, s.end, s.detail),
                    cx,
                ))
            })
    }
}
