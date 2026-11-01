//! Follow the steps of an agent run and inspect individual results.
use crate::primitives::family::Paint;
use crate::primitives::ops_surface::*;
use gpui::{prelude::*, *};
use web_time::Instant;
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
    cursor: f32,
    last: Option<Instant>,
    initialized: bool,
    speed: f32,
    selected: Option<usize>,
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
            cursor: 0.,
            last: None,
            initialized: false,
            speed: 1.,
            selected: None,
        }
    }
}
impl Render for RunLanes {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let end = self.spans.iter().map(|s| s.end).fold(1., f32::max);
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
            self.cursor = (self.cursor
                + now.saturating_duration_since(prev).as_secs_f32() * self.speed)
                .min(end);
            self.last = (self.cursor < end).then_some(now);
            if self.last.is_some() {
                w.request_animation_frame()
            }
        }
        let p = Paint::get(cx);
        let radius = crate::foundation::Theme::global(cx).tokens.radius.md;
        let colors = [0x7d8bb0, 0x779e93, 0xb29a65, 0x9c83b4, 0xc67a79];
        let mut lanes = column();
        for (l, name) in self.lanes.iter().enumerate() {
            let mut rail = div()
                .relative()
                .flex_1()
                .h(px(23.))
                .rounded(radius)
                .bg(p.stage);
            for (i, s) in self.spans.iter().enumerate().filter(|(_, s)| s.lane == l) {
                let c = rgb(colors[s.kind % 5]);
                let selected = self.selected == Some(i)
                    || self
                        .selected
                        .and_then(|j| self.spans.get(j))
                        .and_then(|t| t.parent)
                        == Some(i);
                rail = rail.child(
                    item(("span", i), s.detail.clone(), cx, move |v, _, cx| {
                        v.selected = Some(i);
                        cx.emit(SpanSelected(i));
                        cx.notify()
                    })
                    .absolute()
                    .left(relative((s.start / end).clamp(0., 1.)))
                    .w(relative(((s.end - s.start) / end).clamp(0., 1.)))
                    .h_full()
                    .overflow_hidden()
                    .p_0()
                    .bg(c.opacity(0.13))
                    .child(
                        div()
                            .h_full()
                            .w(relative(
                                ((self.cursor - s.start) / (s.end - s.start).max(0.001))
                                    .clamp(0., 1.),
                            ))
                            .bg(c.opacity(if selected { 1. } else { 0.76 })),
                    ),
                )
            }
            lanes = lanes.child(
                row()
                    .child(note(name.clone(), cx).w(px(82.)).truncate())
                    .child(
                        rail.child(
                            div()
                                .absolute()
                                .left(relative(self.cursor / end))
                                .w(px(1.))
                                .h_full()
                                .bg(p.ink),
                        ),
                    ),
            )
        }
        // Join dependent spans on the same coordinates as the rails. As the
        // destination starts, draw the elbow from the source to the destination.
        let links: Vec<_> = self
            .spans
            .iter()
            .enumerate()
            .filter_map(|(i, span)| {
                let parent = span.parent.and_then(|n| self.spans.get(n))?;
                let t = ((self.cursor - span.start) / 0.36).clamp(0., 1.);
                (t > 0.).then_some((
                    parent.end / end,
                    parent.lane,
                    span.start / end,
                    span.lane,
                    t,
                    self.selected == Some(i),
                ))
            })
            .collect();
        lanes = lanes.relative().child(
            canvas(
                |_, _, _| (),
                move |b, _, w, _| {
                    let left = b.left() + px(90.);
                    let width = (b.size.width - px(90.)).max(px(0.));
                    for &(start, a, finish, z, t, sel) in &links {
                        let from = point(left + width * start, b.top() + px(11.5 + 31. * a as f32));
                        let to = point(left + width * finish, b.top() + px(11.5 + 31. * z as f32));
                        let middle = point(to.x, from.y);
                        let mut path = PathBuilder::stroke(px(if sel { 2. } else { 1. }));
                        path.move_to(from);
                        path.line_to(from + (middle - from) * (2. * t).min(1.));
                        if t > 0.5 {
                            path.line_to(middle + (to - middle) * (2. * t - 1.));
                        }
                        if let Ok(path) = path.build() {
                            w.paint_path(path, p.secondary.opacity(if sel { 0.82 } else { 0.38 }))
                        }
                    }
                },
            )
            .absolute()
            .inset_0()
            .size_full(),
        );
        panel(&self.id, 522., cx)
            .child(title(self.title.clone()))
            .child(note(
                format!(
                    "{} · {:.1}s / {:.1}s",
                    if self.last.is_some() {
                        tr(cx, "Running", "运行中")
                    } else {
                        tr(cx, "Paused", "已暂停")
                    },
                    self.cursor,
                    end
                ),
                cx,
            ))
            .child(lanes)
            .child(
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
                                    s.cursor = 0.
                                }
                                s.last = Some(cx.background_executor().now())
                            }
                            cx.notify()
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
                            cx.notify()
                        },
                    ))
                    .child(btn(
                        "speed",
                        format!("{}×", self.speed),
                        false,
                        cx,
                        |s, _, cx| {
                            s.speed = if s.speed == 1. { 2. } else { 1. };
                            cx.notify()
                        },
                    )),
            )
            .when_some(self.selected.and_then(|i| self.spans.get(i)), |v, s| {
                v.child(note(
                    format!("{:.1}–{:.1}s · {}", s.start, s.end, s.detail),
                    cx,
                ))
            })
    }
}
