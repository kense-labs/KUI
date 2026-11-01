//! Single-select tree with visible-order keyboard navigation.
use crate::primitives::{
    family::{Paint, Slide, disabled_motion},
    glyph::{Glyph, icon, transformed},
    nav::*,
};
use gpui::{prelude::*, *};
use std::collections::BTreeSet;
pub struct TreeNode {
    pub id: SharedString,
    pub name: SharedString,
    pub children: Vec<TreeNode>,
}
impl TreeNode {
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        children: Vec<Self>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            children,
        }
    }
}
pub struct TreeSelected(pub SharedString);
struct RowEntrance {
    alpha: Slide,
    offset: Slide,
    starts: Option<web_time::Instant>,
}
pub struct TreeView {
    nodes: Vec<TreeNode>,
    expanded: BTreeSet<SharedString>,
    selected: Option<SharedString>,
    focus: FocusHandle,
    marker: Option<Slide>,
    appearance: BranchRows<Entry>,
    opened: Option<SharedString>,
    settled_once: bool,
    entrances: std::collections::BTreeMap<SharedString, RowEntrance>,
    chevrons: std::collections::BTreeMap<SharedString, Slide>,
}
#[derive(Clone)]
struct Entry {
    id: SharedString,
    name: SharedString,
    depth: usize,
    branch: bool,
    parent: Option<SharedString>,
}
impl EventEmitter<TreeSelected> for TreeView {}
impl TreeView {
    pub fn new(nodes: Vec<TreeNode>, cx: &App) -> Self {
        Self {
            nodes,
            expanded: Default::default(),
            selected: None,
            marker: None,
            appearance: Default::default(),
            opened: None,
            settled_once: false,
            entrances: Default::default(),
            chevrons: Default::default(),
            focus: cx.focus_handle().tab_stop(true),
        }
    }
    pub fn expanded(mut self, ids: Vec<SharedString>) -> Self {
        self.expanded.extend(ids);
        self
    }
    fn visible(&self) -> Vec<Entry> {
        fn walk(
            nodes: &[TreeNode],
            depth: usize,
            parent: Option<&SharedString>,
            expanded: &BTreeSet<SharedString>,
            out: &mut Vec<Entry>,
        ) {
            for node in nodes {
                out.push(Entry {
                    id: node.id.clone(),
                    name: node.name.clone(),
                    depth,
                    branch: !node.children.is_empty(),
                    parent: parent.cloned(),
                });
                if expanded.contains(&node.id) {
                    walk(&node.children, depth + 1, Some(&node.id), expanded, out)
                }
            }
        }
        let mut out = vec![];
        walk(&self.nodes, 0, None, &self.expanded, &mut out);
        out
    }
    fn toggle(&mut self, id: &SharedString) {
        if !self.expanded.remove(id) {
            self.expanded.insert(id.clone());
            self.opened = Some(id.clone());
        }
    }
    fn select(&mut self, id: SharedString, cx: &mut Context<Self>) {
        if self.selected.as_ref() != Some(&id) {
            self.selected = Some(id.clone());
            cx.emit(TreeSelected(id));
        }
        cx.notify()
    }
    fn key(&mut self, e: &KeyDownEvent, cx: &mut Context<Self>) {
        let visible = self.visible();
        if visible.is_empty() {
            return;
        }
        let i = visible
            .iter()
            .position(|v| Some(&v.id) == self.selected.as_ref())
            .unwrap_or(0);
        let it = &visible[i];
        let dir = e.keystroke.key.as_str();
        let j = match dir {
            "up" => i.saturating_sub(1),
            "down" => (i + 1).min(visible.len() - 1),
            "home" => 0,
            "end" => visible.len() - 1,
            "left" if self.expanded.remove(&it.id) => i,
            "left" => visible
                .iter()
                .position(|e| Some(&e.id) == it.parent.as_ref())
                .unwrap_or(i),
            "right" if it.branch && !self.expanded.contains(&it.id) => {
                self.toggle(&it.id);
                i
            }
            "right" if it.branch => i + 1,
            "enter" | "space" => {
                if it.branch {
                    self.toggle(&it.id)
                }
                i
            }
            _ => return,
        };
        self.select(visible[j].id.clone(), cx);
        cx.stop_propagation()
    }
}
impl Render for TreeView {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let mut root = panel(cx)
            .id("tree")
            .role(Role::Tree)
            .relative()
            .track_focus(&self.focus)
            .tab_stop(true)
            .w(fit(w, 390.))
            .p_2()
            .on_key_down(cx.listener(|s, e, _, cx| s.key(e, cx)));
        let visible = self.visible();
        let opened = visible
            .iter()
            .position(|e| Some(&e.id) == self.opened.as_ref());
        let rows = self.appearance.sync(visible, |v| v.id.clone(), w, cx);
        let mut previous = std::mem::take(&mut self.entrances);
        let now = cx.background_executor().now();
        let still = disabled_motion(cx);
        let initial = !self.settled_once;
        self.settled_once = true;
        if let Some(index) = rows
            .iter()
            .position(|(v, _, live)| *live && Some(&v.id) == self.selected.as_ref())
        {
            let y = rows[..index]
                .iter()
                .map(|(_, fraction, _)| 36. * fraction.height)
                .sum();
            let marker = self.marker.get_or_insert_with(|| Slide::new(y));
            marker.duration(y, 0.22, cx);
            root = root.child(
                div()
                    .absolute()
                    .left(px(8.))
                    .right(px(8.))
                    .top(px(8. + marker.get(w, cx)))
                    .h(px(36.))
                    .rounded(crate::foundation::surface_radius(cx))
                    .bg(p.hover),
            );
        }
        for (index, (entry, frame, live)) in rows.into_iter().enumerate() {
            let a = frame.height;
            let id = entry.id;
            let mut entrance = previous.remove(&id).unwrap_or_else(|| {
                let rank = opened.map_or(0, |parent| index.saturating_sub(parent + 1).min(7));
                let animate = !initial && !still;
                RowEntrance {
                    alpha: Slide::new(if animate { 0. } else { 1. }),
                    offset: Slide::new(if animate { -6. } else { 0. }),
                    starts: (animate && rank > 0)
                        .then(|| now + std::time::Duration::from_millis(35 * rank as u64)),
                }
            });
            if !live || still {
                entrance.starts = None;
            }
            if entrance.starts.is_some_and(|t| now < t) {
                w.request_animation_frame();
            } else {
                entrance.starts = None;
                entrance.alpha.duration(
                    if live { 1. } else { 0. },
                    if live { 0.15 } else { 0.09 },
                    cx,
                );
                entrance.offset.duration(
                    if live { 0. } else { -4. },
                    if live { 0.4 } else { 0.09 },
                    cx,
                );
            }
            let opacity = entrance.alpha.get(w, cx).clamp(0., 1.);
            let offset = entrance.offset.get(w, cx);
            self.entrances.insert(id.clone(), entrance);
            let open = self.expanded.contains(&id);
            let branch = entry.branch;
            let target = if open { 1. } else { 0. };
            let chevron = self
                .chevrons
                .entry(id.clone())
                .or_insert_with(|| Slide::new(target));
            chevron.duration(target, 0.2, cx);
            let angle = (chevron.get(w, cx) - 1.) * std::f32::consts::FRAC_PI_2;
            root = root.child(
                div()
                    .h(px(36. * a))
                    .opacity(opacity)
                    .overflow_hidden()
                    .child(
                        button(id.clone(), entry.name.clone(), cx)
                            .relative()
                            .left(px(offset))
                            .tab_index(-1)
                            .justify_start()
                            .pl(px(8. + 16. * entry.depth as f32))
                            .h(px(36.))
                            .role(Role::TreeItem)
                            .when(branch, |b| {
                                b.aria_expanded(open).child(transformed(
                                    Glyph::Down,
                                    12.,
                                    p.faint,
                                    angle,
                                    1.,
                                    (0., 0.),
                                    1.,
                                ))
                            })
                            .when(!branch, |b| b.child(div().w(px(12.))))
                            .child(icon(
                                if branch {
                                    Glyph::Folder
                                } else {
                                    Glyph::FileText
                                },
                                16.,
                                p.secondary,
                            ))
                            .child(entry.name)
                            .on_click(cx.listener(move |s, _, w, cx| {
                                if !live {
                                    return;
                                }
                                s.focus.focus(w, cx);
                                if branch {
                                    s.toggle(&id)
                                }
                                s.select(id.clone(), cx)
                            })),
                    ),
            )
        }
        root
    }
}
