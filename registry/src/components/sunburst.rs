//! Hierarchical radial chart. Values belong to the host and branch totals are derived.
use crate::components::animated_counter::AnimatedCounter;
use crate::primitives::{
    family::{Paint, Slide, Tone, face},
    presentation::{column, hint, row, tr},
};
use gpui::{prelude::*, *};
use std::{
    cell::Cell,
    f32::consts::{PI, TAU},
    rc::Rc,
};

#[derive(Clone, Debug)]
pub struct SunburstNode {
    pub id: SharedString,
    pub label: SharedString,
    pub value: f32,
    pub color: Hsla,
    pub children: Vec<SunburstNode>,
}
impl SunburstNode {
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        value: f32,
        color: Hsla,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            value,
            color,
            children: vec![],
        }
    }
    pub fn with_children(mut self, children: Vec<Self>) -> Self {
        self.children = children;
        self
    }
    fn total(&self) -> f64 {
        if !self.children.is_empty() {
            self.children.iter().map(Self::total).sum()
        } else if self.value.is_finite() {
            f64::from(self.value.max(0.))
        } else {
            0.
        }
    }
}
#[derive(Clone, Debug)]
pub struct SunburstFocus {
    pub id: SharedString,
    pub path: Vec<SharedString>,
    pub value: f32,
}
#[derive(Clone, Copy, Default)]
struct Pose {
    start: f32,
    end: f32,
    inner: f32,
    outer: f32,
}
#[derive(Clone)]
struct Target {
    id: SharedString,
    label: SharedString,
    path: Vec<usize>,
    value: f32,
    share: f32,
    color: Hsla,
    pose: Pose,
}
struct ArcMotion {
    target: Target,
    from: Pose,
    to: Pose,
    current: Pose,
    progress: Slide,
    departing: bool,
}
fn node_at<'a>(root: &'a SunburstNode, path: &[usize]) -> Option<&'a SunburstNode> {
    path.iter().try_fold(root, |node, &i| node.children.get(i))
}
fn levels(node: &SunburstNode) -> usize {
    node.children.iter().map(levels).max().map_or(0, |v| 1 + v)
}
fn partition(
    node: &SunburstNode,
    path: &[usize],
    depth: usize,
    max_depth: usize,
    start: f32,
    end: f32,
    out: &mut Vec<Target>,
) {
    if depth >= max_depth {
        return;
    }
    let total = node.total();
    if total <= 0. {
        return;
    }
    let ring = (1. - 0.36) / max_depth as f32;
    let mut angle = start;
    for (i, child) in node.children.iter().enumerate() {
        let v = child.total();
        if v <= 0. {
            continue;
        }
        let next = angle + (end - start) * (v / total) as f32;
        let mut child_path = path.to_vec();
        child_path.push(i);
        out.push(Target {
            id: child.id.clone(),
            label: child.label.clone(),
            path: child_path.clone(),
            value: v.min(f64::from(f32::MAX)) as f32,
            share: (v / total) as f32,
            color: child.color,
            pose: Pose {
                start: angle,
                end: next,
                inner: 0.36 + depth as f32 * ring,
                outer: 0.36 + (depth + 1) as f32 * ring,
            },
        });
        partition(child, &child_path, depth + 1, max_depth, angle, next, out);
        angle = next;
    }
}
fn table_rows(node: &SunburstNode, prefix: &str, parent_total: f64, out: &mut Vec<String>) {
    let path = if prefix.is_empty() {
        node.label.to_string()
    } else {
        format!("{prefix} / {}", node.label)
    };
    let value = node.total();
    let share = if parent_total > 0. {
        value / parent_total * 100.
    } else {
        0.
    };
    out.push(format!("{path} · {value:.0} · {share:.1}%"));
    for child in &node.children {
        table_rows(child, &path, value, out);
    }
}
fn lerp_pose(a: Pose, b: Pose, t: f32) -> Pose {
    let v = |x: f32, y: f32| x + (y - x) * t;
    Pose {
        start: v(a.start, b.start),
        end: v(a.end, b.end),
        inner: v(a.inner, b.inner),
        outer: v(a.outer, b.outer),
    }
}
pub struct Sunburst {
    title: SharedString,
    root: SunburstNode,
    focus: Vec<usize>,
    active: Option<Vec<usize>>,
    pointer: Rc<Cell<Option<(f32, f32)>>>,
    tip_x: Slide,
    tip_y: Slide,
    arcs: Vec<ArcMotion>,
    dirty: bool,
    bounds: Rc<Cell<Bounds<Pixels>>>,
    counter: Option<Entity<AnimatedCounter>>,
    graph_focus: Option<FocusHandle>,
    focus_subscriptions: Vec<Subscription>,
    max_depth: usize,
    visible: bool,
}
impl EventEmitter<SunburstFocus> for Sunburst {}
impl Sunburst {
    pub fn new(title: impl Into<SharedString>, root: SunburstNode) -> Self {
        Self {
            title: title.into(),
            root,
            focus: vec![],
            active: None,
            pointer: Default::default(),
            tip_x: Slide::new(0.),
            tip_y: Slide::new(0.),
            arcs: vec![],
            dirty: true,
            bounds: Default::default(),
            counter: None,
            graph_focus: None,
            focus_subscriptions: vec![],
            max_depth: 3,
            visible: true,
        }
    }
    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        if self.visible != visible {
            self.visible = visible;
            cx.notify();
        }
    }
    pub fn with_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth.clamp(1, 8);
        self
    }
    pub fn set_root(&mut self, root: SunburstNode, cx: &mut Context<Self>) {
        let ids: Vec<_> = (1..=self.focus.len())
            .filter_map(|n| node_at(&self.root, &self.focus[..n]).map(|v| v.id.clone()))
            .collect();
        let mut focus = Vec::new();
        let mut current = &root;
        for id in ids {
            let Some(i) = current.children.iter().position(|child| child.id == id) else {
                break;
            };
            focus.push(i);
            current = &current.children[i];
        }
        self.root = root;
        self.focus = focus;
        self.active = None;
        self.dirty = true;
        cx.notify();
    }
    fn set_focus(&mut self, path: Vec<usize>, cx: &mut Context<Self>) {
        let Some(node) = node_at(&self.root, &path) else {
            return;
        };
        if self.focus == path {
            return;
        }
        let event = SunburstFocus {
            id: node.id.clone(),
            path: Self::labels(&self.root, &path),
            value: node.total().min(f64::from(f32::MAX)) as f32,
        };
        self.focus = path;
        self.active = None;
        self.dirty = true;
        cx.emit(event);
        cx.notify();
    }
    fn back(&mut self, cx: &mut Context<Self>) {
        if !self.focus.is_empty() {
            let mut path = self.focus.clone();
            path.pop();
            self.set_focus(path, cx);
        }
    }
    fn labels(root: &SunburstNode, path: &[usize]) -> Vec<SharedString> {
        let mut result = vec![root.label.clone()];
        let mut node = root;
        for &i in path {
            let Some(child) = node.children.get(i) else {
                break;
            };
            node = child;
            result.push(node.label.clone());
        }
        result
    }
    fn reconcile(&mut self, cx: &App) {
        let Some(root) = node_at(&self.root, &self.focus) else {
            return;
        };
        let mut targets = vec![];
        let depth = levels(root).min(self.max_depth).max(1);
        partition(
            root,
            &self.focus,
            0,
            depth,
            -PI / 2.,
            TAU - PI / 2.,
            &mut targets,
        );
        let mut previous = std::mem::take(&mut self.arcs);
        let mut next = Vec::with_capacity(targets.len() + previous.len());
        let first_layout = previous.is_empty();
        for target in targets {
            let old = previous
                .iter()
                .position(|arc| arc.target.id == target.id && !arc.departing)
                .map(|i| previous.remove(i));
            let from = old.map_or(
                Pose {
                    start: target.pose.start,
                    end: target.pose.start,
                    inner: target.pose.inner,
                    outer: target.pose.inner,
                },
                |arc| lerp_pose(arc.from, arc.to, arc.progress.value(cx).clamp(0., 1.)),
            );
            let to = target.pose;
            let mut progress = Slide::new(0.);
            progress.duration(1., if first_layout { 0.9 } else { 0.62 }, cx);
            next.push(ArcMotion {
                target,
                from,
                to,
                current: from,
                progress,
                departing: false,
            });
        }
        for arc in previous.into_iter().filter(|arc| !arc.departing) {
            let from = lerp_pose(arc.from, arc.to, arc.progress.value(cx).clamp(0., 1.));
            let middle = (from.start + from.end) / 2.;
            let to = Pose {
                start: middle,
                end: middle,
                inner: from.inner,
                outer: from.inner,
            };
            let mut progress = Slide::new(0.);
            progress.duration(1., 0.62, cx);
            next.push(ArcMotion {
                target: arc.target,
                from,
                to,
                current: from,
                progress,
                departing: true,
            });
        }
        self.arcs = next;
        self.dirty = false;
    }
    fn pick(&self, position: Point<Pixels>) -> Option<Vec<usize>> {
        let b = self.bounds.get();
        let w = f32::from(b.size.width);
        let h = f32::from(b.size.height);
        let radius = w.min(h) / 2.;
        if radius <= 0. {
            return None;
        }
        let dx = f32::from(position.x - b.origin.x) - w / 2.;
        let dy = f32::from(position.y - b.origin.y) - h / 2.;
        let r = (dx * dx + dy * dy).sqrt() / radius;
        let mut a = dy.atan2(dx);
        if a < -PI / 2. {
            a += TAU;
        }
        self.arcs
            .iter()
            .filter(|v| !v.departing)
            .find(|v| {
                let p = v.current;
                r >= p.inner && r <= p.outer && a >= p.start && a <= p.end
            })
            .map(|v| v.target.path.clone())
    }
}
fn intersects_viewport(bounds: Bounds<Pixels>, viewport: Size<Pixels>) -> bool {
    bounds.size.width > px(0.)
        && bounds.size.height > px(0.)
        && bounds.origin.x < viewport.width
        && bounds.origin.y < viewport.height
        && bounds.origin.x + bounds.size.width > px(0.)
        && bounds.origin.y + bounds.size.height > px(0.)
}
impl Render for Sunburst {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.dirty {
            self.reconcile(cx);
        }
        let p = Paint::get(cx);
        let on_screen = self.visible
            && w.is_visible()
            && intersects_viewport(self.bounds.get(), w.viewport_size());
        let root = node_at(&self.root, &self.focus)
            .unwrap_or(&self.root)
            .clone();
        let total = root.total();
        // The reference shows "No data yet" in the centre while the total is zero.
        let empty = total <= 0.;
        let counter = self
            .counter
            .get_or_insert_with(|| {
                cx.new(|_| AnimatedCounter::new(total.min(1_000_000_000_000.)).with_size(22.))
            })
            .clone();
        counter.update(cx, |v, cx| v.set_value(total.min(1_000_000_000_000.), cx));
        let active = self.active.clone();
        let data: Vec<(Target, Pose, bool)> = self
            .arcs
            .iter_mut()
            .map(|arc| {
                let t = if on_screen {
                    arc.progress.get(w, cx)
                } else {
                    arc.progress.value(cx)
                }
                .clamp(0., 1.);
                arc.current = lerp_pose(arc.from, arc.to, t);
                (arc.target.clone(), arc.current, arc.departing)
            })
            .collect();
        let bounds = self.bounds.clone();
        let selected = self.active.as_ref().and_then(|path| {
            self.arcs
                .iter()
                .find(|arc| !arc.departing && &arc.target.path == path)
                .map(|arc| (arc.target.label.clone(), arc.target.value, arc.target.share))
        });
        let info = selected.as_ref().map_or_else(
            || tr(cx, "Select a segment to inspect", "选择扇区查看数据").to_string(),
            |(label, value, share)| format!("{label} · {value:.0} · {:.1}%", share * 100.),
        );
        if self.active.is_some()
            && let Some((x, y)) = self.pointer.get()
        {
            let width = f32::from(self.bounds.get().size.width);
            let height = f32::from(self.bounds.get().size.height);
            let mut left = x + 14.;
            if left + 150. > width {
                left = (x - 14. - 150.).max(0.);
            }
            let mut top = y - 14. - 72.;
            if top < 0. {
                top = y + 14.;
            }
            self.tip_x.duration(left.max(0.), 0.26, cx);
            self.tip_y
                .duration(top.clamp(0., (height - 72.).max(0.)), 0.26, cx);
        }
        let tip_x = self.tip_x.get(w, cx);
        let tip_y = self.tip_y.get(w, cx);
        let graph_focus = self
            .graph_focus
            .get_or_insert_with(|| cx.focus_handle())
            .clone();
        if self.focus_subscriptions.is_empty() {
            // The reference highlights the first segment when the chart gains focus.
            self.focus_subscriptions
                .push(cx.on_focus(&graph_focus, w, |s, _, cx| {
                    if s.active.is_none()
                        && let Some(arc) = s.arcs.iter().find(|arc| !arc.departing)
                    {
                        s.active = Some(arc.target.path.clone());
                        cx.notify();
                    }
                }));
        }
        let mut graph = div()
            .id("sunburst")
            .role(Role::Figure)
            .tab_index(0)
            .track_focus(&graph_focus)
            .relative()
            .w(px(340.))
            .h(px(340.))
            .max_w_full()
            .aria_label(format!(
                "{}. {}. {}",
                self.title,
                tr(
                    cx,
                    "Use arrow keys to inspect, enter to zoom and escape to go back",
                    "使用方向键查看，回车进入，Esc 返回"
                ),
                info
            ))
            .on_mouse_move(cx.listener(|s, e: &MouseMoveEvent, _, cx| {
                let b = s.bounds.get();
                s.pointer.set(Some((
                    f32::from(e.position.x - b.origin.x),
                    f32::from(e.position.y - b.origin.y),
                )));
                let next = s.pick(e.position);
                if s.active != next {
                    s.active = next;
                    cx.notify();
                }
            }))
            .on_hover(cx.listener(|s, inside: &bool, _, cx| {
                if !inside {
                    s.pointer.set(None);
                    if s.active.take().is_some() {
                        cx.notify();
                    }
                }
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|s, e: &MouseDownEvent, _, cx| {
                    if let Some(path) = s.pick(e.position) {
                        s.set_focus(path, cx);
                    } else {
                        let b = s.bounds.get();
                        let x = f32::from(e.position.x - b.origin.x) - f32::from(b.size.width) / 2.;
                        let y =
                            f32::from(e.position.y - b.origin.y) - f32::from(b.size.height) / 2.;
                        if (x * x + y * y).sqrt()
                            < f32::from(b.size.width.min(b.size.height)) * 0.12
                        {
                            s.back(cx);
                        }
                    }
                }),
            )
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                if s.active.is_none()
                    && matches!(e.keystroke.key.as_str(), "left" | "right" | "up" | "down")
                {
                    if let Some(arc) = s.arcs.iter().find(|arc| !arc.departing) {
                        s.active = Some(arc.target.path.clone());
                        cx.notify();
                        w.prevent_default();
                        cx.stop_propagation();
                    }
                    return;
                }
                let mut path = s.active.clone().unwrap_or_else(|| {
                    let mut p = s.focus.clone();
                    p.push(0);
                    p
                });
                match e.keystroke.key.as_str() {
                    "right" | "left" => {
                        let Some(i) = path.pop() else {
                            return;
                        };
                        let n = node_at(&s.root, &path).map_or(0, |v| v.children.len());
                        if n == 0 {
                            return;
                        }
                        path.push(if e.keystroke.key == "right" {
                            (i + 1) % n
                        } else {
                            (i + n - 1) % n
                        });
                        s.active = Some(path);
                    }
                    "down" => {
                        if node_at(&s.root, &path).is_some_and(|v| !v.children.is_empty()) {
                            path.push(0);
                            s.active = Some(path);
                        }
                    }
                    "up" => {
                        if path.len() > s.focus.len() + 1 {
                            path.pop();
                            s.active = Some(path);
                        }
                    }
                    "enter" | "space" => s.set_focus(path, cx),
                    "escape" | "backspace" => s.back(cx),
                    _ => return,
                };
                w.prevent_default();
                cx.stop_propagation();
                cx.notify();
            }))
            .child(
                canvas(
                    move |b, w, _| {
                        let previous = bounds.get();
                        let viewport = w.viewport_size();
                        if previous.size != b.size
                            || intersects_viewport(previous, viewport)
                                != intersects_viewport(b, viewport)
                        {
                            w.request_animation_frame();
                        }
                        bounds.set(b);
                    },
                    move |b, _, w, cx| {
                        let width = f32::from(b.size.width);
                        let height = f32::from(b.size.height);
                        let radius = width.min(height) / 2. - 2.;
                        let center =
                            point(b.origin.x + px(width / 2.), b.origin.y + px(height / 2.));
                        for (item, pose, departing) in &data {
                            if pose.end - pose.start < 0.0001 || pose.outer - pose.inner < 0.001 {
                                continue;
                            }
                            let gap = (1. / radius.max(1.)).min((pose.end - pose.start) * 0.18);
                            let start = pose.start + gap;
                            let end = pose.end - gap;
                            let steps =
                                (((end - start).abs() * radius / 5.).ceil() as usize).clamp(2, 180);
                            let at = |angle: f32, r: f32| {
                                point(
                                    center.x + px(angle.cos() * radius * r),
                                    center.y + px(angle.sin() * radius * r),
                                )
                            };
                            let mut path = PathBuilder::fill();
                            path.move_to(at(start, pose.outer));
                            for i in 1..=steps {
                                path.line_to(at(
                                    start + (end - start) * i as f32 / steps as f32,
                                    pose.outer,
                                ));
                            }
                            for i in (0..=steps).rev() {
                                path.line_to(at(
                                    start + (end - start) * i as f32 / steps as f32,
                                    pose.inner,
                                ));
                            }
                            path.close();
                            if let Ok(path) = path.build() {
                                let is_active = active.as_ref().is_some_and(|p| p == &item.path);
                                let opacity = if *departing {
                                    0.45
                                } else if is_active {
                                    0.96
                                } else {
                                    0.70
                                };
                                let mut fill = p.panel.blend(item.color.opacity(opacity));
                                if active.is_some() && !is_active && !*departing {
                                    fill = fill.opacity(0.42);
                                }
                                w.paint_path(path, fill);
                            }
                            // Arrange upright glyphs along the arc; GPUI text does not expose
                            // glyph rotation, so this preserves legibility on both platforms.
                            let ring_radius = radius * (pose.inner + pose.outer) / 2.;
                            if !departing
                                && active.as_ref().map(|p| p == &item.path).unwrap_or(true)
                                && ring_radius > 0.
                                && (pose.outer - pose.inner) * radius >= 16.
                            {
                                let lines: Vec<_> = item
                                    .label
                                    .chars()
                                    .map(|ch| {
                                        let text: SharedString = ch.to_string().into();
                                        let mut run = w.text_style().to_run(text.len());
                                        run.color = p.ink;
                                        w.text_system().shape_line(text, px(10.), &[run], None)
                                    })
                                    .collect();
                                let length: f32 =
                                    lines.iter().map(|line| f32::from(line.width())).sum();
                                if length + 14. < (end - start) * ring_radius {
                                    let mid = (start + end) / 2.;
                                    let mut offset = -length / 2.;
                                    for line in lines {
                                        let width = f32::from(line.width());
                                        let angle = mid + (offset + width / 2.) / ring_radius;
                                        let pos = point(
                                            center.x + px(angle.cos() * ring_radius - width / 2.),
                                            center.y + px(angle.sin() * ring_radius - 6.),
                                        );
                                        let _ =
                                            line.paint(pos, px(12.), TextAlign::Left, None, w, cx);
                                        offset += width;
                                    }
                                }
                            }
                        }
                    },
                )
                .absolute()
                .inset_0(),
            )
            .child(
                div()
                    .absolute()
                    .left(relative(0.334))
                    .top(relative(0.334))
                    .w(relative(0.332))
                    .h(relative(0.332))
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap(px(2.))
                    .rounded_full()
                    .when(!self.focus.is_empty(), |view| {
                        view.hover(|s| s.bg(p.ink.opacity(0.035)))
                    })
                    .child(if empty {
                        div()
                            .text_size(px(12.))
                            .line_height(px(20.))
                            .text_color(p.secondary)
                            .child(tr(cx, "No data yet", "暂无数据").to_string())
                    } else {
                        div()
                            .text_size(px(12.))
                            .line_height(px(20.))
                            .text_color(p.secondary)
                            .child(root.label.clone())
                    })
                    .when(!empty && total <= 1_000_000_000_000., |view| {
                        view.child(counter)
                    })
                    .when(!empty && total > 1_000_000_000_000., |view| {
                        view.child(format!("{total:.2e}"))
                    }),
            );
        if let Some((label, value, share)) = &selected {
            graph = graph.child(
                div()
                    .absolute()
                    .left(px(tip_x))
                    .top(px(tip_y))
                    .min_w(px(150.))
                    .max_w(px(240.))
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
                            .text_size(px(12.))
                            .line_height(px(20.))
                            .text_color(p.faint)
                            .child(label.clone()),
                    )
                    .child(
                        div()
                            .flex()
                            .items_baseline()
                            .justify_between()
                            .gap(px(12.))
                            .text_size(px(14.))
                            .line_height(px(18.9))
                            .child(
                                div()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .child(format!("{value:.0}")),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(p.faint)
                                    .child(format!("{:.1}%", share * 100.)),
                            ),
                    ),
            );
        }
        let crumbs = Self::labels(&self.root, &self.focus);
        let crumb_count = crumbs.len();
        let focus = self.focus.clone();
        let depth = self.focus.len();
        let hot = self
            .active
            .as_ref()
            .and_then(|path| path.get(depth).copied());
        let legend = column().gap(px(2.)).min_w(px(150.)).flex_1().children(
            root.children.iter().enumerate().map(|(i, child)| {
                let mut path = focus.clone();
                path.push(i);
                let key_path = path.clone();
                let share = if total > 0. {
                    child.total() / total * 100.
                } else {
                    0.
                };
                face(
                    ("sunburst-legend", i),
                    child.label.clone(),
                    36.,
                    crate::foundation::surface_radius(cx).into(),
                    Tone::Clear,
                    cx,
                )
                .w_full()
                .px(px(10.))
                .gap(px(10.))
                .justify_start()
                .text_color(if hot == Some(i) { p.ink } else { p.secondary })
                .when(hot == Some(i), |row| row.bg(p.ink.opacity(0.05)))
                .when(hot.is_some() && hot != Some(i), |row| row.opacity(0.55))
                .child(
                    div()
                        .size(px(8.))
                        .flex_none()
                        .rounded_full()
                        .bg(child.color),
                )
                .child(div().flex_1().min_w(px(0.)).child(child.label.clone()))
                .child(
                    div()
                        .flex_none()
                        .text_color(p.ink)
                        .child(format!("{:.0}", child.total())),
                )
                .child(
                    div()
                        .min_w(px(44.8))
                        .flex_none()
                        .text_color(p.faint)
                        .child(format!("{share:.1}%")),
                )
                .on_click(cx.listener(move |s, _, _, cx| s.set_focus(path.clone(), cx)))
                .on_key_down(cx.listener(move |s, e: &KeyDownEvent, w, cx| {
                    if matches!(e.keystroke.key.as_str(), "enter" | "space") && !e.is_held {
                        s.set_focus(key_path.clone(), cx);
                        w.prevent_default();
                        cx.stop_propagation();
                    }
                }))
            }),
        );
        let mut rows = vec![];
        table_rows(&self.root, "", self.root.total(), &mut rows);
        column()
            .relative()
            .w_full()
            .max_w(px(700.))
            .gap(px(16.))
            .p(px(24.))
            .rounded(crate::foundation::surface_radius(cx))
            .bg(p.panel)
            .border_1()
            .border_color(p.line)
            .text_color(p.ink)
            .child(
                div()
                    .text_size(px(16.))
                    .line_height(px(22.))
                    .font_weight(FontWeight::MEDIUM)
                    .child(self.title.clone()),
            )
            .child(
                row()
                    .items_center()
                    .gap_1()
                    .children(crumbs.into_iter().enumerate().map(|(i, label)| {
                        let path = focus.iter().copied().take(i).collect::<Vec<_>>();
                        let key_path = path.clone();
                        let current = i + 1 == crumb_count;
                        face(
                            ("sunburst-crumb", i),
                            label.clone(),
                            32.,
                            crate::foundation::surface_radius(cx).into(),
                            Tone::Clear,
                            cx,
                        )
                        .px(px(if current { 4. } else { 6. }))
                        .text_color(if current { p.ink } else { p.secondary })
                        .when(current, |b| b.font_weight(FontWeight::MEDIUM))
                        .when(!current, |b| {
                            b.hover(|s| s.bg(p.ink.opacity(0.05)).text_color(p.ink))
                        })
                        .child(label)
                        .on_click(cx.listener(move |s, _, _, cx| s.set_focus(path.clone(), cx)))
                        .on_key_down(cx.listener(
                            move |s, e: &KeyDownEvent, w, cx| {
                                if matches!(e.keystroke.key.as_str(), "enter" | "space")
                                    && !e.is_held
                                {
                                    s.set_focus(key_path.clone(), cx);
                                    w.prevent_default();
                                    cx.stop_propagation();
                                }
                            },
                        ))
                    })),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_x(px(32.))
                    .gap_y(px(24.))
                    .items_center()
                    .justify_center()
                    .child(graph)
                    .child(legend),
            )
            .child(hint(info, p))
            .child(
                div()
                    .id("sunburst-data-table")
                    .absolute()
                    .size(px(1.))
                    .overflow_hidden()
                    .role(Role::Table)
                    .aria_label(tr(cx, "Hierarchy data", "层级数据"))
                    .children(rows.into_iter().enumerate().map(|(i, value)| {
                        div()
                            .id(("sunburst-data-row", i))
                            .role(Role::Row)
                            .aria_label(value)
                    })),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::{SunburstNode, partition};
    use gpui::rgb;
    #[test]
    fn very_large_values_keep_finite_balanced_angles() {
        let color = rgb(0x8899aa).into();
        let root = SunburstNode::new("root", "Root", 0., color).with_children(vec![
            SunburstNode::new("a", "A", f32::MAX, color),
            SunburstNode::new("b", "B", f32::MAX, color),
        ]);
        let mut arcs = Vec::new();
        partition(&root, &[], 0, 1, 0., std::f32::consts::TAU, &mut arcs);
        assert_eq!(arcs.len(), 2);
        assert!(
            arcs.iter()
                .all(|arc| arc.pose.start.is_finite() && arc.pose.end.is_finite())
        );
        assert!((arcs[0].share - 0.5).abs() < 0.001);
    }
}
