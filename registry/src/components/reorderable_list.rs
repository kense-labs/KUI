//! Draggable ordered list with an equivalent keyboard pathway.
use crate::primitives::{
    family::{Paint, Slide},
    glyph::{Glyph, icon},
    nav::*,
};
use gpui::{prelude::*, *};
pub struct OrderedItem {
    pub id: SharedString,
    pub title: SharedString,
    pub detail: SharedString,
}
impl OrderedItem {
    pub fn new(
        id: impl Into<SharedString>,
        title: impl Into<SharedString>,
        detail: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            detail: detail.into(),
        }
    }
}
#[derive(Clone)]
struct DragRow {
    owner: EntityId,
    id: SharedString,
    label: SharedString,
}
struct Ghost(SharedString);
impl Render for Ghost {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        panel(cx).p_2().child(self.0.clone())
    }
}
pub struct OrderChanged(pub Vec<SharedString>);
pub struct ReorderableList {
    items: Vec<(OrderedItem, FocusHandle, Slide)>,
    preview: Option<(usize, usize)>,
}
impl EventEmitter<OrderChanged> for ReorderableList {}
impl ReorderableList {
    pub fn new(items: Vec<OrderedItem>, cx: &App) -> Self {
        Self {
            preview: None,
            items: items
                .into_iter()
                .enumerate()
                .map(|(at, i)| (i, cx.focus_handle().tab_stop(true), Slide::new(at as f32)))
                .collect(),
        }
    }
    pub fn move_to(&mut self, from: usize, to: usize, cx: &mut Context<Self>) {
        if from >= self.items.len() || to >= self.items.len() || from == to {
            return;
        }
        let v = self.items.remove(from);
        self.items.insert(to, v);
        for (at, (_, _, slot)) in self.items.iter_mut().enumerate() {
            slot.duration(at as f32, 0.26, cx)
        }
        cx.emit(OrderChanged(
            self.items.iter().map(|v| v.0.id.clone()).collect(),
        ));
        cx.notify()
    }
}
impl Render for ReorderableList {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        if self.preview.is_some() && !cx.has_active_drag() {
            self.preview = None;
            for (i, (_, _, pos)) in self.items.iter_mut().enumerate() {
                pos.duration(i as f32, 0.24, cx);
            }
        }
        let mut root = column()
            .id("rows")
            .gap_2()
            .w(fit(w, 460.))
            .on_drag_move::<DragRow>(cx.listener(|s, e: &DragMoveEvent<DragRow>, _, cx| {
                let data = e.drag(cx);
                if data.owner != cx.entity_id() {
                    return;
                }
                let Some(from) = s.items.iter().position(|v| v.0.id == data.id) else {
                    return;
                };
                let to = if e.bounds.contains(&e.event.position) {
                    ((f32::from(e.event.position.y - e.bounds.top()) / 66.).floor() as usize)
                        .min(s.items.len() - 1)
                } else {
                    from
                };
                if s.preview == Some((from, to)) {
                    return;
                }
                s.preview = Some((from, to));
                for (i, (_, _, pos)) in s.items.iter_mut().enumerate() {
                    let dest = if i == from {
                        to
                    } else if from < i && i <= to {
                        i - 1
                    } else if to <= i && i < from {
                        i + 1
                    } else {
                        i
                    };
                    pos.duration(dest as f32, 0.22, cx);
                }
                cx.notify();
            }));
        for (i, (item, focus, slot)) in self.items.iter().enumerate() {
            let owner = cx.entity_id();
            let drag = DragRow {
                owner,
                id: item.id.clone(),
                label: item.title.clone(),
            };
            root = root.child(
                row()
                    .id(("row", i))
                    .relative()
                    .top(px((slot.get(w, cx) - i as f32) * 66.))
                    .h(px(58.))
                    .px_3()
                    .gap_3()
                    .bg(p.panel)
                    .rounded(crate::foundation::surface_radius(cx))
                    .border_1()
                    .border_color(p.line)
                    .drag_over::<DragRow>(move |b, _, _, _| b.border_color(p.secondary))
                    .on_drop(cx.listener(move |s, d: &DragRow, _, cx| {
                        if d.owner == owner
                            && let Some(from) = s.items.iter().position(|v| v.0.id == d.id)
                        {
                            let to = s.preview.take().map_or(i, |(_, to)| to);
                            s.move_to(from, to, cx)
                        }
                    }))
                    .child(
                        button(item.id.clone(), item.title.clone(), cx)
                            .track_focus(focus)
                            .size(px(38.))
                            .p_0()
                            .bg(p.stage)
                            .child(icon(Glyph::Menu, 16., p.secondary))
                            .on_drag(drag, |d, _, _, cx| cx.new(|_| Ghost(d.label.clone())))
                            .on_key_down(cx.listener(move |s, e: &KeyDownEvent, w, cx| {
                                let to = match e.keystroke.key.as_str() {
                                    "up" => i.saturating_sub(1),
                                    "down" => (i + 1).min(s.items.len() - 1),
                                    _ => return,
                                };
                                s.move_to(i, to, cx);
                                s.items[to].1.focus(w, cx);
                                cx.stop_propagation()
                            })),
                    )
                    .child(
                        column()
                            .flex_1()
                            .child(label(item.title.clone(), 14., p.ink))
                            .child(label(item.detail.clone(), 12., p.secondary)),
                    )
                    .child(label(format!("{:02}", i + 1), 13., p.faint)),
            )
        }
        root
    }
}
