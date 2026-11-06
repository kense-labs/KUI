//! Host-supplied search results presented on an illustrative two-dimensional plane.
use crate::primitives::{
    family::{Paint, PhaseClock, Tone, disabled_motion, face},
    glyph::{Glyph, icon},
    presentation::{column, hint, row, title, tr},
};
use gpui::{prelude::*, *};
use std::f32::consts::TAU;
#[derive(Clone, Debug)]
pub struct VectorDocument {
    pub id: SharedString,
    pub title: SharedString,
    pub topic: SharedString,
    pub source: SharedString,
    pub x: f32,
    pub y: f32,
    pub color: Hsla,
}
impl VectorDocument {
    pub fn new(
        id: impl Into<SharedString>,
        title: impl Into<SharedString>,
        topic: impl Into<SharedString>,
        x: f32,
        y: f32,
        color: Hsla,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            topic: topic.into(),
            source: "".into(),
            x,
            y,
            color,
        }
    }
    pub fn with_source(mut self, source: impl Into<SharedString>) -> Self {
        self.source = source.into();
        self
    }
    fn position(&self) -> (f32, f32) {
        (coordinate(self.x), coordinate(self.y))
    }
}
#[derive(Clone, Debug)]
pub struct VectorMatch {
    pub document: SharedString,
    pub score: f32,
}
impl VectorMatch {
    pub fn new(document: impl Into<SharedString>, score: f32) -> Self {
        Self {
            document: document.into(),
            score,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VectorSearchStatus {
    Waiting,
    Searching,
    Ready,
    Failed,
}
#[derive(Clone, Debug)]
pub struct VectorSearchSnapshot {
    pub query: SharedString,
    pub query_x: f32,
    pub query_y: f32,
    pub dimensions: usize,
    pub documents: Vec<VectorDocument>,
    pub links: Vec<(SharedString, SharedString)>,
    pub matches: Vec<VectorMatch>,
    pub status: VectorSearchStatus,
    pub message: SharedString,
}
impl VectorSearchSnapshot {
    pub fn new(
        query: impl Into<SharedString>,
        documents: Vec<VectorDocument>,
        matches: Vec<VectorMatch>,
    ) -> Self {
        Self {
            query: query.into(),
            query_x: 0.,
            query_y: 0.,
            dimensions: 1536,
            documents,
            links: vec![],
            matches,
            status: VectorSearchStatus::Ready,
            message: "".into(),
        }
    }
}
#[derive(Clone, Debug)]
pub struct VectorStageChanged {
    pub stage: usize,
}
#[derive(Clone, Debug)]
pub struct VectorPlaybackChanged {
    pub paused: bool,
}
pub struct VectorSearch {
    snapshot: VectorSearchSnapshot,
    elapsed: f32,
    last_age: f32,
    clock: PhaseClock,
    paused: bool,
    visible: bool,
    selected: Option<usize>,
}
impl EventEmitter<VectorStageChanged> for VectorSearch {}
impl EventEmitter<VectorPlaybackChanged> for VectorSearch {}
const DURATION: f32 = 18.;
fn coordinate(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(-1., 1.)
    } else {
        0.
    }
}
impl VectorSearch {
    pub fn new(snapshot: VectorSearchSnapshot) -> Self {
        Self {
            snapshot,
            elapsed: 0.,
            last_age: 0.,
            clock: PhaseClock::default(),
            paused: false,
            visible: true,
            selected: None,
        }
    }
    pub fn set_snapshot(&mut self, snapshot: VectorSearchSnapshot, cx: &mut Context<Self>) {
        self.snapshot = snapshot;
        self.elapsed = 0.;
        self.selected = None;
        self.clock.restart(cx);
        self.last_age = 0.;
        cx.notify();
    }
    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        if self.visible != visible {
            self.visible = visible;
            self.clock.restart(cx);
            self.last_age = 0.;
            if visible {
                cx.notify();
            }
        }
    }
    pub fn set_paused(&mut self, paused: bool, cx: &mut Context<Self>) {
        if self.paused != paused {
            self.paused = paused;
            self.clock.restart(cx);
            self.last_age = 0.;
            cx.emit(VectorPlaybackChanged { paused });
            cx.notify();
        }
    }
    pub fn set_stage(&mut self, stage: usize, cx: &mut Context<Self>) {
        let stage = stage.min(3);
        self.elapsed = (stage as f32 + 0.75) * DURATION / 4.;
        self.selected = Some(stage);
        self.set_paused(true, cx);
        cx.emit(VectorStageChanged { stage });
        cx.notify();
    }
}
fn circle(center: Point<Pixels>, radius: f32, color: Hsla, w: &mut Window) {
    let mut path = PathBuilder::fill();
    for i in 0..=20 {
        let a = TAU * i as f32 / 20.;
        let q = point(
            center.x + px(a.cos() * radius),
            center.y + px(a.sin() * radius),
        );
        if i == 0 {
            path.move_to(q)
        } else {
            path.line_to(q)
        }
    }
    path.close();
    if let Ok(path) = path.build() {
        w.paint_path(path, color);
    }
}
fn connect(a: Point<Pixels>, b: Point<Pixels>, width: f32, color: Hsla, w: &mut Window) {
    let mut path = PathBuilder::stroke(px(width));
    path.move_to(a);
    path.line_to(b);
    if let Ok(path) = path.build() {
        w.paint_path(path, color);
    }
}
impl Render for VectorSearch {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let reduced = disabled_motion(cx);
        let playing = self.visible && w.is_visible() && !self.paused && !reduced;
        if playing {
            if !self.clock.age(cx).is_finite() {
                self.clock.restart(cx);
            }
            let age = self.clock.age(cx);
            self.elapsed = (self.elapsed + (age - self.last_age).clamp(0., 0.1)) % DURATION;
            self.last_age = age;
            self.selected = None;
            w.request_animation_frame();
        } else {
            self.clock.restart(cx);
            self.last_age = 0.;
        }
        let t = if reduced && self.selected.is_none() {
            1.
        } else {
            (self.elapsed / DURATION).clamp(0., 1.)
        };
        let stage = self.selected.unwrap_or_else(|| ((t * 4.) as usize).min(3));
        let stages = [
            tr(cx, "Embed", "嵌入"),
            tr(cx, "Index", "索引"),
            tr(cx, "Query", "查询"),
            tr(cx, "Rank", "排序"),
        ];
        let ready = self.snapshot.status == VectorSearchStatus::Ready;
        let docs = self.snapshot.documents.clone();
        let links = self.snapshot.links.clone();
        let matches = self.snapshot.matches.clone();
        let query = (
            coordinate(self.snapshot.query_x),
            coordinate(self.snapshot.query_y),
        );
        let description = format!(
            "{}. {}: {}. {}: {}. {}: {}.",
            tr(cx, "Illustrative vector search", "向量检索示意"),
            tr(cx, "Query", "查询"),
            self.snapshot.query,
            tr(cx, "Documents", "文档"),
            docs.len(),
            tr(cx, "Results", "结果"),
            matches.len()
        );
        let plane = div()
            .id("vector-search-plane")
            .role(Role::Image)
            .aria_label(description)
            .w_full()
            .min_w(px(240.))
            .h(px(230.))
            .child(
                canvas(
                    |_, _, _| {},
                    move |b, _, w, _| {
                        let xy = |x: f32, y: f32| {
                            point(
                                b.origin.x + b.size.width * (0.5 + coordinate(x) * 0.42),
                                b.origin.y + b.size.height * (0.5 + coordinate(y) * 0.38),
                            )
                        };
                        let query = xy(query.0, query.1);
                        if stage >= 1 {
                            for (a, b) in &links {
                                if let (Some(a), Some(b)) = (
                                    docs.iter().find(|d| d.id == *a),
                                    docs.iter().find(|d| d.id == *b),
                                ) {
                                    let (ax, ay) = a.position();
                                    let (bx, by) = b.position();
                                    connect(xy(ax, ay), xy(bx, by), 1., p.line, w);
                                }
                            }
                        }
                        let visible = if stage == 0 {
                            ((t * 4.) * docs.len() as f32).ceil() as usize
                        } else {
                            docs.len()
                        };
                        for doc in docs.iter().take(visible) {
                            let (x, y) = doc.position();
                            circle(xy(x, y), 4., doc.color, w);
                        }
                        if stage >= 2 {
                            circle(query, 6., p.bright, w);
                        }
                        if stage >= 3 && ready {
                            for result in &matches {
                                if let Some(doc) = docs.iter().find(|doc| doc.id == result.document)
                                {
                                    let (x, y) = doc.position();
                                    let pt = xy(x, y);
                                    connect(query, pt, 1.5, p.bright.opacity(0.5), w);
                                    circle(pt, 11., p.bright.opacity(0.16), w);
                                    circle(pt, 4., doc.color, w);
                                }
                            }
                        }
                    },
                )
                .size_full(),
            );
        let steps = div()
            .id("vector-search-stages")
            .role(Role::Group)
            .tab_index(0)
            .flex()
            .flex_wrap()
            .gap(px(6.))
            .aria_label(tr(cx, "Search stages", "检索阶段"))
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, _, cx| {
                let i = s
                    .selected
                    .unwrap_or(((s.elapsed / DURATION * 4.) as usize).min(3));
                let next = match e.keystroke.key.as_str() {
                    "left" => i.saturating_sub(1),
                    "right" => (i + 1).min(3),
                    "home" => 0,
                    "end" => 3,
                    _ => return,
                };
                s.set_stage(next, cx);
                cx.stop_propagation();
            }))
            .children(stages.iter().enumerate().map(|(i, name)| {
                let current = i == stage;
                face(
                    ("vector-stage", i),
                    format!("{} {name}", i + 1),
                    40.,
                    0.,
                    Tone::Clear,
                    cx,
                )
                .flex_1()
                .h(px(40.))
                .flex_col()
                .justify_center()
                .gap(px(6.))
                .text_size(px(12.))
                .line_height(px(15.6))
                .text_color(if current { p.ink } else { p.faint })
                .hover(|s| s.text_color(p.ink))
                .aria_toggled(current.into())
                .child(*name)
                .child(
                    div()
                        .w_full()
                        .h(px(2.))
                        .rounded_full()
                        .bg(if current { p.faint } else { p.line })
                        .child(
                            div()
                                .h_full()
                                .w(relative(if i <= stage { 1. } else { 0. }))
                                .rounded_full()
                                .bg(p.bright),
                        ),
                )
                .on_click(cx.listener(move |s, _, _, cx| s.set_stage(i, cx)))
            }));
        let state = match self.snapshot.status {
            VectorSearchStatus::Waiting => tr(cx, "Waiting", "等待"),
            VectorSearchStatus::Searching => tr(cx, "Searching", "检索中"),
            VectorSearchStatus::Ready => tr(cx, "Ready", "已完成"),
            VectorSearchStatus::Failed => tr(cx, "Failed", "失败"),
        };
        let results =
            column()
                .w(px(272.))
                .max_w_full()
                .gap(px(6.))
                .child(
                    div()
                        .text_size(px(12.))
                        .line_height(px(20.))
                        .text_color(p.faint)
                        .child(tr(cx, "Nearest documents", "相关文档")),
                )
                .when(stage >= 3 && ready, |view| {
                    view.children(self.snapshot.matches.iter().enumerate().filter_map(
                        |(i, result)| {
                            self.snapshot
                                .documents
                                .iter()
                                .find(|doc| doc.id == result.document)
                                .map(|doc| {
                                    let score = if result.score.is_finite() {
                                        format!("{:.2}", result.score)
                                    } else {
                                        "—".into()
                                    };
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap(px(6.))
                                        .child(
                                            row()
                                                .items_center()
                                                .gap(px(8.))
                                                .child(
                                                    div()
                                                        .w(px(12.))
                                                        .flex_none()
                                                        .text_size(px(12.))
                                                        .text_color(p.faint)
                                                        .child(format!("{}", i + 1)),
                                                )
                                                .child(
                                                    div()
                                                        .flex_1()
                                                        .min_w(px(0.))
                                                        .text_size(px(14.))
                                                        .line_height(px(20.))
                                                        .font_weight(FontWeight::MEDIUM)
                                                        .child(doc.title.clone()),
                                                )
                                                .child(
                                                    div()
                                                        .text_size(px(12.))
                                                        .font_weight(FontWeight::MEDIUM)
                                                        .text_color(
                                                            p.ink.blend(p.bright.opacity(0.7)),
                                                        )
                                                        .child(score),
                                                ),
                                        )
                                        .child(
                                            div()
                                                .w_full()
                                                .h(px(3.))
                                                .rounded_full()
                                                .bg(p.line)
                                                .child(
                                                    div()
                                                        .h_full()
                                                        .w(relative(result.score.clamp(0., 1.)))
                                                        .rounded_full()
                                                        .bg(p.bright),
                                                ),
                                        )
                                })
                        },
                    ))
                })
                .when(!(stage >= 3 && ready), |view| view.child(hint(state, p)));
        let mut topics: Vec<(SharedString, Hsla)> = vec![];
        for doc in &self.snapshot.documents {
            if !topics.iter().any(|(name, _)| name == &doc.topic) {
                topics.push((doc.topic.clone(), doc.color));
            }
        }
        column()
            .w_full()
            .max_w(px(960.))
            .gap(px(24.))
            .p(px(16.))
            .rounded(crate::foundation::surface_radius(cx))
            .bg(p.panel)
            .border_1()
            .border_color(p.line)
            .text_color(p.ink)
            .child(
                row()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(title(tr(cx, "Vector search", "向量检索"), p))
                    .child(
                        face(
                            "vector-play",
                            if self.paused {
                                tr(cx, "Play", "播放")
                            } else {
                                tr(cx, "Pause", "暂停")
                            },
                            32.,
                            16.,
                            Tone::Surface,
                            cx,
                        )
                        .size(px(32.))
                        .min_w(px(32.))
                        .p_0()
                        .rounded_full()
                        .border_1()
                        .border_color(p.line)
                        .text_color(p.secondary)
                        .hover(|s| s.border_color(p.faint).text_color(p.ink))
                        .child(icon(
                            if self.paused {
                                Glyph::Play
                            } else {
                                Glyph::Pause
                            },
                            16.,
                            p.secondary,
                        ))
                        .on_click(cx.listener(|s, _, _, cx| s.set_paused(!s.paused, cx))),
                    ),
            )
            .child(hint(
                tr(
                    cx,
                    "Illustration only: positions are not a semantic similarity calculation.",
                    "示意投影，不代表由二维距离计算真实语义相似度。",
                ),
                p,
            ))
            .child(
                row()
                    .flex_wrap()
                    .gap_3()
                    .children(topics.into_iter().map(|(name, color)| {
                        row()
                            .items_center()
                            .gap(px(6.))
                            .child(div().size(px(7.)).rounded_full().bg(color))
                            .child(hint(name, p))
                    })),
            )
            .child(steps)
            .child(hint(
                format!(
                    "{} {} / {} · {} {}",
                    tr(cx, "Embedded", "已嵌入"),
                    if stage == 0 {
                        ((t * 4.) * self.snapshot.documents.len() as f32).ceil() as usize
                    } else {
                        self.snapshot.documents.len()
                    },
                    self.snapshot.documents.len(),
                    tr(cx, "Links", "连接"),
                    if stage >= 1 {
                        self.snapshot.links.len()
                    } else {
                        0
                    }
                ),
                p,
            ))
            .child(hint(
                format!(
                    "{} · {} {}",
                    self.snapshot.query,
                    self.snapshot.dimensions,
                    tr(cx, "dimensions", "维")
                ),
                p,
            ))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap(px(40.))
                    .items_center()
                    .child(div().flex_1().min_w(px(240.)).child(plane))
                    .child(results),
            )
            .when(!self.snapshot.message.is_empty(), |view| {
                view.child(hint(self.snapshot.message.clone(), p))
            })
    }
}
#[cfg(test)]
mod tests {
    use super::coordinate;
    #[test]
    fn coordinates_are_bounded() {
        assert_eq!(coordinate(f32::NAN), 0.);
        assert_eq!(coordinate(2.), 1.);
        assert_eq!(coordinate(-2.), -1.);
    }
}
