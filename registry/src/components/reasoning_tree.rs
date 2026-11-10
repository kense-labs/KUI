//! Illustrative beam search over host-supplied thoughts; scores are not model output.
use crate::primitives::{
    family::{Paint, Slide, Tone, disabled_motion, face},
    nav::tr,
};
use gpui::{prelude::*, *};
use std::{cell::Cell, collections::HashSet, rc::Rc};
use web_time::Instant;

#[derive(Clone)]
pub struct Thought {
    pub id: SharedString,
    pub label: SharedString,
    pub detail: SharedString,
    pub score: Option<f32>,
    pub children: Vec<Thought>,
}
impl Thought {
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            detail: "".into(),
            score: None,
            children: Vec::new(),
        }
    }
    pub fn score(mut self, score: f32) -> Self {
        self.score = score.is_finite().then_some(score.clamp(0., 0.99));
        self
    }
    pub fn detail(mut self, detail: impl Into<SharedString>) -> Self {
        self.detail = detail.into();
        self
    }
    pub fn children(mut self, children: Vec<Thought>) -> Self {
        self.children = children;
        self
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum ReasoningTreeEvent {
    Answer {
        id: SharedString,
        path: Vec<SharedString>,
        beam_width: usize,
    },
    BeamWidth(usize),
    Retry,
}
#[derive(Clone)]
struct Node {
    thought: Thought,
    parent: Option<usize>,
    depth: usize,
    score: f32,
    kept: bool,
}
fn score_for(thought: &Thought, seed: u64) -> f32 {
    thought
        .score
        .filter(|v| v.is_finite())
        .map(|v| v.clamp(0., 0.99))
        .unwrap_or_else(|| {
            let mut hash = 14695981039346656037_u64 ^ seed;
            for byte in thought.id.as_bytes() {
                hash = (hash ^ u64::from(*byte)).wrapping_mul(1099511628211);
            }
            (30 + hash % 70) as f32 / 100.
        })
}
fn search(
    thoughts: &[Thought],
    width: usize,
    seed: u64,
) -> (Vec<Node>, Vec<Vec<usize>>, Option<usize>) {
    let mut nodes: Vec<Node> = Vec::new();
    let mut stages = Vec::new();
    let mut candidates: Vec<(Thought, Option<usize>)> =
        thoughts.iter().cloned().map(|t| (t, None)).collect();
    let mut best = None;
    for depth in 0..3 {
        if candidates.is_empty() {
            break;
        }
        let mut indices = Vec::new();
        for (thought, parent) in candidates.drain(..) {
            let score = score_for(&thought, seed);
            indices.push(nodes.len());
            nodes.push(Node {
                thought,
                parent,
                depth,
                score,
                kept: false,
            });
        }
        indices.sort_by(|a, b| nodes[*b].score.total_cmp(&nodes[*a].score).then(a.cmp(b)));
        let survivors: Vec<_> = indices.iter().copied().take(width.clamp(1, 3)).collect();
        for index in &survivors {
            nodes[*index].kept = true;
        }
        best = survivors.first().copied();
        stages.push(indices);
        for index in survivors {
            for child in nodes[index].thought.children.iter().cloned() {
                candidates.push((child, Some(index)));
            }
        }
    }
    if let Some(last) = stages.last() {
        for &index in last {
            nodes[index].kept = Some(index) == best;
        }
    }
    (nodes, stages, best)
}
/// Stable tree positions based on the full data, so a new stage does not move old nodes.
fn tree_rows(nodes: &[Node]) -> (Vec<f32>, f32) {
    fn visit(index: usize, nodes: &[Node], rows: &mut [f32], cursor: &mut f32) -> f32 {
        let children: Vec<usize> = nodes
            .iter()
            .enumerate()
            .filter_map(|(i, n)| (n.parent == Some(index)).then_some(i))
            .collect();
        let y = if children.is_empty() {
            let y = *cursor;
            *cursor += 1.;
            y
        } else {
            let first = visit(children[0], nodes, rows, cursor);
            let mut last = first;
            for child in children.iter().skip(1) {
                last = visit(*child, nodes, rows, cursor);
            }
            (first + last) / 2.
        };
        rows[index] = y;
        y
    }
    let mut rows = vec![0.; nodes.len()];
    let mut cursor = 0.;
    for (i, node) in nodes.iter().enumerate() {
        if node.parent.is_none() {
            visit(i, nodes, &mut rows, &mut cursor);
        }
    }
    (rows, cursor.max(1.))
}
pub struct ReasoningTree {
    question: SharedString,
    thoughts: Vec<Thought>,
    nodes: Vec<Node>,
    stages: Vec<Vec<usize>>,
    answer: Option<usize>,
    stage: usize,
    beam: usize,
    seed: u64,
    playing: bool,
    progress: f32,
    last_frame: Option<Instant>,
    in_view: bool,
    collapsed: HashSet<SharedString>,
    selected: usize,
    focus: FocusHandle,
    error: Option<SharedString>,
    width: Rc<Cell<Pixels>>,
    stage_motion: [Slide; 3],
}
impl EventEmitter<ReasoningTreeEvent> for ReasoningTree {}
impl ReasoningTree {
    pub fn new(
        question: impl Into<SharedString>,
        thoughts: Vec<Thought>,
        cx: &mut Context<Self>,
    ) -> Self {
        let (nodes, stages, answer) = search(&thoughts, 2, 7);
        Self {
            question: question.into(),
            thoughts,
            nodes,
            stages,
            answer,
            stage: 0,
            beam: 2,
            seed: 7,
            playing: false,
            progress: 0.,
            last_frame: None,
            in_view: true,
            collapsed: HashSet::new(),
            selected: 0,
            focus: cx.focus_handle().tab_stop(true),
            error: None,
            width: Rc::new(Cell::new(px(0.))),
            stage_motion: std::array::from_fn(|_| Slide::new(0.)),
        }
    }
    pub fn set_thoughts(&mut self, thoughts: Vec<Thought>, cx: &mut Context<Self>) {
        self.thoughts = thoughts;
        self.error = None;
        self.rebuild(cx);
    }
    pub fn set_error(&mut self, message: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.error = Some(message.into());
        self.playing = false;
        self.last_frame = None;
        cx.notify();
    }
    pub fn set_beam_width(&mut self, width: usize, cx: &mut Context<Self>) {
        let width = width.clamp(1, 3);
        if self.beam != width {
            self.beam = width;
            self.rebuild(cx);
            cx.emit(ReasoningTreeEvent::BeamWidth(width));
        }
    }
    fn rebuild(&mut self, cx: &mut Context<Self>) {
        (self.nodes, self.stages, self.answer) = search(&self.thoughts, self.beam, self.seed);
        self.restart(cx);
    }
    pub fn restart(&mut self, cx: &mut Context<Self>) {
        self.stage = 0;
        self.playing = false;
        self.progress = 0.;
        self.last_frame = None;
        self.selected = 0;
        self.collapsed.clear();
        for animation in &mut self.stage_motion {
            animation.reset(0.);
        }
        cx.notify();
    }
    pub fn set_playing(&mut self, playing: bool, cx: &mut Context<Self>) {
        if self.error.is_some() || self.stages.is_empty() || self.stage >= self.stages.len() {
            return;
        }
        self.playing = playing;
        self.last_frame = None;
        cx.notify();
    }
    pub fn step(&mut self, cx: &mut Context<Self>) {
        if self.error.is_some() || self.stage >= self.stages.len() {
            return;
        }
        self.stage_motion[self.stage].duration(1., 0.48, cx);
        self.stage += 1;
        self.progress = 0.;
        if self.stage == self.stages.len() {
            self.playing = false;
            if let Some(id) = self.answer {
                let mut path = Vec::new();
                let mut node = Some(id);
                while let Some(index) = node {
                    path.push(self.nodes[index].thought.id.clone());
                    node = self.nodes[index].parent;
                }
                path.reverse();
                cx.emit(ReasoningTreeEvent::Answer {
                    id: self.nodes[id].thought.id.clone(),
                    path,
                    beam_width: self.beam,
                });
            }
        }
        cx.notify();
    }
    fn visible(&self) -> Vec<usize> {
        fn add(tree: &ReasoningTree, parent: Option<usize>, out: &mut Vec<usize>) {
            for (i, node) in tree.nodes.iter().enumerate() {
                if node.parent != parent || node.depth >= tree.stage {
                    continue;
                }
                out.push(i);
                if !tree.collapsed.contains(&node.thought.id) {
                    add(tree, Some(i), out);
                }
            }
        }
        let mut result = Vec::new();
        add(self, None, &mut result);
        result
    }
}
impl Render for ReasoningTree {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let radius = crate::foundation::surface_radius(cx);
        let reduced = disabled_motion(cx);
        if self.playing && self.in_view && w.is_visible() {
            let now = Instant::now();
            if let Some(last) = self.last_frame {
                self.progress += now.saturating_duration_since(last).as_secs_f32().min(0.1) / 1.4;
            }
            self.last_frame = Some(now);
            if self.progress >= 1. {
                self.step(cx);
            }
            if self.playing {
                w.request_animation_frame();
            }
        } else {
            self.last_frame = None;
        }
        let visible = self.visible();
        if !visible.is_empty() {
            self.selected = self.selected.min(visible.len() - 1);
        } else {
            self.selected = 0;
        }
        let completed = !self.stages.is_empty() && self.stage == self.stages.len();
        let viewport = w.viewport_size();
        let weak = cx.weak_entity();
        let measured = self.width.get();
        let graph_width = if measured > px(0.) {
            (measured - px(42.)).max(px(1.))
        } else {
            (viewport.width - px(64.)).min(px(728.)).max(px(1.))
        };
        let wide = graph_width + px(42.) >= px(670.);
        let capture = self.width.clone();
        let animation: Vec<f32> = self
            .stage_motion
            .iter()
            .map(|slide| {
                if self.in_view && w.is_visible() {
                    slide.get(w, cx)
                } else {
                    slide.value(cx)
                }
            })
            .collect();
        let (positions, rows) = tree_rows(&self.nodes);
        let mut root =
            div()
                .on_children_prepainted(move |bounds, _, cx| {
                    if let Some(b) = bounds.first() {
                        let visible = b.left() < viewport.width
                            && b.top() < viewport.height
                            && b.right() > px(0.)
                            && b.bottom() > px(0.);
                        let _ = weak.update(cx, |s, cx| {
                            if s.in_view != visible {
                                s.in_view = visible;
                                s.last_frame = None;
                                cx.notify();
                            }
                        });
                    }
                })
                .id("reasoning-tree")
                .role(Role::Group)
                .aria_label(tr(cx, "Reasoning tree", "推理树"))
                .w_full()
                .relative()
                .child(
                    canvas(
                        move |bounds, w, _| {
                            if (capture.get() - bounds.size.width).abs() > px(0.5) {
                                capture.set(bounds.size.width);
                                w.request_animation_frame();
                            }
                        },
                        |_, _, _, _| (),
                    )
                    .absolute()
                    .inset_0(),
                )
                .max_w(px(760.))
                .p(px(20.))
                .rounded(radius)
                .border_1()
                .border_color(p.line)
                .bg(p.panel)
                .text_color(p.ink)
                .flex()
                .flex_col()
                .gap(px(20.))
                .child(
                    div()
                        .flex()
                        .justify_between()
                        .gap_2()
                        .child(tr(cx, "Reasoning tree", "推理树"))
                        .child(div().text_size(px(12.)).text_color(p.secondary).child(
                            if completed {
                                tr(cx, "Answer ready", "结果已就绪")
                            } else if self.playing {
                                tr(cx, "Searching", "搜索中")
                            } else {
                                tr(cx, "Paused", "已暂停")
                            },
                        )),
                )
                .child(
                    div()
                        .w_full()
                        .px(px(12.))
                        .py(px(10.))
                        .rounded(radius)
                        .border_1()
                        .border_color(p.line)
                        .bg(p.tray)
                        .text_size(px(13.))
                        .text_color(p.ink)
                        .child(self.question.clone()),
                );
        if let Some(error) = &self.error {
            root = root
                .child(
                    div()
                        .id("tree-error")
                        .role(Role::Alert)
                        .text_color(p.danger)
                        .child(error.clone()),
                )
                .child(
                    face(
                        "tree-retry",
                        tr(cx, "Retry", "重试"),
                        32.,
                        8.,
                        Tone::Surface,
                        cx,
                    )
                    .child(tr(cx, "Retry", "重试"))
                    .on_click(cx.listener(|_, _, _, cx| cx.emit(ReasoningTreeEvent::Retry))),
                );
        }
        let nav = visible.clone();
        let mut tree = div()
            .id("thought-tree")
            .track_focus(&self.focus)
            .tab_stop(true)
            .role(Role::Tree)
            .aria_label(tr(cx, "Thought candidates", "候选思路"))
            .flex()
            .flex_col()
            .gap_1()
            .on_key_down(cx.listener(move |s, e: &KeyDownEvent, _, cx| {
                if nav.is_empty() {
                    return;
                }
                let selected = s.selected.min(nav.len() - 1);
                let current = nav[selected];
                match e.keystroke.key.as_str() {
                    "up" => {
                        s.selected = if wide {
                            (0..selected)
                                .rev()
                                .find(|i| s.nodes[nav[*i]].depth == s.nodes[current].depth)
                                .unwrap_or(selected)
                        } else {
                            selected.saturating_sub(1)
                        };
                    }
                    "down" => {
                        s.selected = if wide {
                            (selected + 1..nav.len())
                                .find(|i| s.nodes[nav[*i]].depth == s.nodes[current].depth)
                                .unwrap_or(selected)
                        } else {
                            (selected + 1).min(nav.len() - 1)
                        };
                    }
                    "home" => s.selected = 0,
                    "end" => s.selected = nav.len() - 1,
                    "left" => {
                        if !s.collapsed.contains(&s.nodes[current].thought.id)
                            && s.nodes.iter().any(|n| n.parent == Some(current))
                        {
                            s.collapsed.insert(s.nodes[current].thought.id.clone());
                        } else if let Some(parent) = s.nodes[current].parent
                            && let Some(position) = nav.iter().position(|i| *i == parent)
                        {
                            s.selected = position;
                        }
                    }
                    "right" => {
                        if s.collapsed.remove(&s.nodes[current].thought.id) {
                        } else if let Some(position) =
                            nav.iter().position(|i| s.nodes[*i].parent == Some(current))
                        {
                            s.selected = position;
                        }
                    }
                    "enter" | "space" => {
                        let id = s.nodes[current].thought.id.clone();
                        if !s.collapsed.remove(&id) {
                            s.collapsed.insert(id);
                        }
                    }
                    _ => return,
                }
                cx.stop_propagation();
                cx.notify();
            }));
        let column = f32::from(graph_width) / 3.;
        let node_width = (column - 12.).clamp(120., 210.);
        let mut answer_path = Vec::new();
        if completed {
            let mut current = self.answer;
            while let Some(index) = current {
                answer_path.push(index);
                current = self.nodes[index].parent;
            }
        }
        if wide {
            let graph_height = px(rows * 56. + 12.);
            let edges: Vec<(f32, f32, f32, f32, f32)> = visible
                .iter()
                .filter_map(|i| {
                    let node = &self.nodes[*i];
                    let parent = node.parent?;
                    if !visible.contains(&parent) {
                        return None;
                    }
                    Some((
                        self.nodes[parent].depth as f32 * column + node_width,
                        positions[parent] * 56. + 22.,
                        node.depth as f32 * column,
                        positions[*i] * 56. + 22.,
                        animation[node.depth].clamp(0., 1.),
                    ))
                })
                .collect();
            tree = tree.relative().h(graph_height).child(
                canvas(
                    |_, _, _| (),
                    move |bounds, _, w, _| {
                        for (x0, y0, x1, y1, opacity) in &edges {
                            let mut path = PathBuilder::stroke(px(1.25));
                            let mid = (x0 + x1) / 2.;
                            path.move_to(bounds.origin + point(px(*x0), px(*y0)));
                            path.line_to(bounds.origin + point(px(mid), px(*y0)));
                            path.line_to(bounds.origin + point(px(mid), px(*y1)));
                            path.line_to(bounds.origin + point(px(*x1), px(*y1)));
                            if let Ok(path) = path.build() {
                                w.paint_path(path, p.secondary.opacity(*opacity * 0.55));
                            }
                        }
                    },
                )
                .absolute()
                .inset_0(),
            );
        } else {
            let edges = visible
                .iter()
                .enumerate()
                .map(|(row, &index)| {
                    let node = &self.nodes[index];
                    let parent = node.parent.and_then(|parent| {
                        visible.iter().position(|&candidate| parent == candidate)
                    });
                    (
                        node.depth as f32 * 16. + 9.,
                        parent.map_or(-12., |row| row as f32 * 50. + 44.),
                        row as f32 * 50. + 22.,
                        animation[node.depth].clamp(0., 1.),
                        answer_path.contains(&index),
                    )
                })
                .collect::<Vec<_>>();
            tree = tree
                .relative()
                .gap(px(6.))
                .min_h(px(self.nodes.len() as f32 * 50.))
                .child(
                    canvas(
                        |_, _, _| (),
                        move |bounds, _, w, _| {
                            for (x, start, end, opacity, answer) in &edges {
                                let mut path = PathBuilder::stroke(px(1.25));
                                path.move_to(bounds.origin + point(px(*x), px(*start)));
                                path.line_to(bounds.origin + point(px(*x), px(*end - 4.)));
                                path.line_to(bounds.origin + point(px(*x + 4.), px(*end)));
                                path.line_to(bounds.origin + point(px(*x + 7.), px(*end)));
                                if let Ok(path) = path.build() {
                                    w.paint_path(
                                        path,
                                        if *answer { p.bright } else { p.secondary }
                                            .opacity(*opacity * 0.55),
                                    );
                                }
                            }
                        },
                    )
                    .absolute()
                    .inset_0(),
                );
        }
        for (position, index) in visible.iter().copied().enumerate() {
            let node = &self.nodes[index];
            let has_children = self.nodes.iter().any(|n| n.parent == Some(index));
            let folded = self.collapsed.contains(&node.thought.id);
            let state = if completed && self.answer == Some(index) {
                tr(cx, "Answer", "结果")
            } else if node.kept {
                tr(cx, "Kept", "保留")
            } else {
                tr(cx, "Pruned", "已剪枝")
            };
            let title = format!(
                "{} · {} · {:.0}%",
                node.thought.label,
                state,
                node.score * 100.
            );
            let id = node.thought.id.clone();
            let node_animation = animation[node.depth].clamp(0., 1.);
            let mut entry = div()
                .id(("thought", index))
                .role(Role::TreeItem)
                .aria_label(format!("{} · {}", title, node.thought.detail))
                .aria_selected(position == self.selected)
                .when(position == self.selected, |row| {
                    row.aria_active_descendant()
                })
                .when(has_children, |row| row.aria_expanded(!folded))
                .ml(px(if wide {
                    0.
                } else {
                    ((node.depth + 1) * 16) as f32
                }))
                .h(px(44.))
                .flex_none()
                .rounded(radius)
                .border_1()
                .border_color(if answer_path.contains(&index) {
                    p.bright.opacity(0.55)
                } else {
                    p.line
                })
                .bg(if position == self.selected {
                    p.hover
                } else {
                    p.panel
                })
                .opacity(node_animation * if node.kept { 1. } else { 0.55 })
                .p_2()
                .flex()
                .items_center()
                .gap_2()
                .cursor_pointer()
                .child(if completed && self.answer == Some(index) {
                    "✓"
                } else if has_children {
                    if folded { "›" } else { "⌄" }
                } else {
                    "·"
                })
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .text_size(px(12.))
                        .truncate()
                        .child(node.thought.label.clone()),
                )
                .child(
                    div()
                        .w(px(30.))
                        .flex_none()
                        .text_right()
                        .text_size(px(11.))
                        .text_color(p.secondary)
                        .child(format!("{:.0}%", node.score * node_animation * 100.)),
                )
                .child(
                    div()
                        .w(px(if wide { 24. } else { 46. }))
                        .h(px(3.))
                        .flex_none()
                        .rounded_full()
                        .bg(p.line)
                        .child(
                            div()
                                .h_full()
                                .w(relative(node.score * node_animation))
                                .rounded_full()
                                .bg(if node.kept { p.bright } else { p.secondary }),
                        ),
                )
                .on_click(cx.listener(move |s, _, w, cx| {
                    s.selected = position;
                    if has_children && !s.collapsed.remove(&id) {
                        s.collapsed.insert(id.clone());
                    }
                    s.focus.focus(w, cx);
                    cx.notify();
                }));
            if wide {
                entry = entry
                    .absolute()
                    .left(px(node.depth as f32 * column))
                    .top(px(positions[index] * 56.))
                    .w(px(node_width))
                    .h(px(44.));
            }
            tree = tree.child(entry);
        }
        root = root.child(tree);
        if self.stages.is_empty() {
            return root.child(div().text_size(px(12.)).text_color(p.secondary).child(tr(
                cx,
                "No candidates",
                "没有候选思路",
            )));
        }
        let play = if self.playing {
            tr(cx, "Pause", "暂停")
        } else {
            tr(cx, "Play", "播放")
        };
        let mut controls = div().flex().flex_wrap().items_center().gap_2();
        if !completed && self.error.is_none() {
            controls = controls
                .child(
                    face("tree-play", play, 32., 8., Tone::Surface, cx)
                        .px_2()
                        .child(play)
                        .on_click(cx.listener(|s, _, _, cx| s.set_playing(!s.playing, cx))),
                )
                .child(
                    face(
                        "tree-step",
                        tr(cx, "Next step", "下一步"),
                        32.,
                        8.,
                        Tone::Surface,
                        cx,
                    )
                    .px_2()
                    .child(tr(cx, "Next step", "下一步"))
                    .on_click(cx.listener(|s, _, _, cx| {
                        s.playing = false;
                        s.step(cx);
                    })),
                );
        }
        controls = controls.child(
            face(
                "tree-restart",
                tr(cx, "Restart", "重新开始"),
                32.,
                8.,
                Tone::Surface,
                cx,
            )
            .px_2()
            .child(tr(cx, "Restart", "重新开始"))
            .on_click(cx.listener(|s, _, _, cx| s.restart(cx))),
        );
        for width in 1..=3 {
            controls = controls.child(
                face(
                    ("tree-beam", width),
                    format!("{width}"),
                    32.,
                    8.,
                    if self.beam == width {
                        Tone::Primary
                    } else {
                        Tone::Surface
                    },
                    cx,
                )
                .px_2()
                .child(format!("{width}"))
                .on_click(cx.listener(move |s, _, _, cx| s.set_beam_width(width, cx))),
            );
        }
        if reduced {
            controls = controls.child(div().text_size(px(12.)).text_color(p.secondary).child(tr(
                cx,
                "Reduced motion",
                "减少动效",
            )));
        }
        root.child(controls)
    }
}
#[cfg(test)]
mod tests {
    use super::{Thought, search};
    #[test]
    fn beam_width_changes_best_path_and_depth_is_bounded() {
        let a = Thought::new("a", "A")
            .score(0.9)
            .children(vec![Thought::new("a1", "A1").score(0.3)]);
        let b = Thought::new("b", "B")
            .score(0.8)
            .children(vec![Thought::new("b1", "B1").score(0.95)]);
        let (narrow, _, narrow_answer) = search(&[a.clone(), b.clone()], 1, 7);
        let (wide, _, wide_answer) = search(&[a, b], 2, 7);
        assert_eq!(narrow[narrow_answer.unwrap()].thought.id.as_ref(), "a1");
        assert_eq!(wide[wide_answer.unwrap()].thought.id.as_ref(), "b1");
        assert!(wide.iter().all(|n| n.depth < 3));
    }
}
