//! Reveal or commit list actions by dragging, or use the equivalent row menu.
use crate::{
    components::dropdown_menu::DropdownMenu,
    foundation::Theme,
    primitives::{
        control::UiLanguage,
        direct_touch::Contact,
        family::{Paint, Slide},
        glyph::{Glyph, icon},
        menu::{MenuItem, MenuSelection},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, collections::VecDeque, rc::Rc};
use web_time::Instant;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwipeAction {
    pub id: SharedString,
    pub label: SharedString,
    pub icon: Glyph,
    pub destructive: bool,
    pub keep_row: bool,
}
impl SwipeAction {
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>, icon: Glyph) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon,
            destructive: false,
            keep_row: false,
        }
    }
    pub fn destructive(mut self) -> Self {
        self.destructive = true;
        self
    }
    pub fn keep_row(mut self) -> Self {
        self.keep_row = true;
        self
    }
}
type RowBody = Box<dyn Fn(&mut Window, &mut App) -> AnyElement>;
pub struct SwipeRow {
    id: SharedString,
    label: SharedString,
    leading: Vec<SwipeAction>,
    trailing: Vec<SwipeAction>,
    body: RowBody,
}
impl SwipeRow {
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        body: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            leading: vec![],
            trailing: vec![],
            body: Box::new(body),
        }
    }
    pub fn leading(mut self, actions: Vec<SwipeAction>) -> Self {
        self.leading = actions;
        self
    }
    pub fn trailing(mut self, actions: Vec<SwipeAction>) -> Self {
        self.trailing = actions;
        self
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwipeSelection {
    pub row: SharedString,
    pub action: SharedString,
}
struct Entry {
    contact: Contact,
    row: SwipeRow,
    menu: Entity<DropdownMenu>,
    _sub: Subscription,
    offset: Slide,
    height: Slide,
    cover: Slide,
    removed: bool,
}
struct Drag {
    row: usize,
    start: Point<Pixels>,
    base: f32,
    locked: bool,
    armed: bool,
    samples: VecDeque<(Instant, f32)>,
}
pub struct SwipeActions {
    id: ElementId,
    label: SharedString,
    entries: Vec<Entry>,
    drag: Option<Drag>,
    row_height: f32,
    full: bool,
    open: Option<usize>,
    focus_next: Option<usize>,
    width: Rc<Cell<f32>>,
}
impl EventEmitter<SwipeSelection> for SwipeActions {}
impl SwipeActions {
    pub fn new(
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        rows: Vec<SwipeRow>,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut entries = vec![];
        let l = UiLanguage::current(cx);
        for (i, row) in rows.into_iter().enumerate() {
            let items = row
                .leading
                .iter()
                .chain(&row.trailing)
                .map(|a| {
                    let mut it = MenuItem::new(a.id.clone(), a.label.clone()).with_icon(a.icon);
                    it.destructive = a.destructive;
                    it
                })
                .collect();
            let button_name = format!("{}: {}", l.text("More actions", "更多操作"), row.label);
            let menu = cx.new(|cx| {
                DropdownMenu::new(("more", i), button_name, items, cx)
                    .with_icon(Glyph::More)
                    .icon_only()
            });
            let sub = cx.subscribe(&menu, move |v, _, sel: &MenuSelection, cx| {
                let row = &v.entries[i].row;
                let choice = row
                    .leading
                    .iter()
                    .chain(&row.trailing)
                    .find(|a| a.id == sel.0)
                    .cloned();
                if let Some(a) = choice {
                    let left = row.leading.iter().any(|a| a.id == sel.0);
                    v.commit(i, &a, left, cx);
                    if !a.keep_row {
                        v.focus_next = (i + 1..v.entries.len())
                            .chain((0..i).rev())
                            .find(|&j| !v.entries[j].removed);
                    }
                }
            });
            entries.push(Entry {
                contact: Default::default(),
                row,
                menu,
                _sub: sub,
                offset: Slide::snappy(0.),
                height: Slide::new(1.),
                cover: Slide::snappy(0.),
                removed: false,
            })
        }
        Self {
            id: id.into(),
            label: label.into(),
            entries,
            drag: None,
            row_height: 82.,
            full: true,
            open: None,
            focus_next: None,
            width: Rc::new(Cell::new(320.)),
        }
    }
    pub fn with_full_swipe(mut self, enabled: bool) -> Self {
        self.full = enabled;
        self
    }
    pub fn with_row_height(mut self, h: f32) -> Self {
        self.row_height = h.max(36.);
        self
    }
    pub fn remaining(&self) -> usize {
        self.entries.iter().filter(|s| !s.removed).count()
    }
    pub fn restore(&mut self, id: &str, cx: &mut Context<Self>) {
        if let Some(s) = self.entries.iter_mut().find(|e| e.row.id.as_ref() == id) {
            s.removed = false;
            s.offset.reset(0.);
            s.height.set(1., cx);
            cx.notify()
        }
    }
    fn close(&mut self, cx: &mut Context<Self>) {
        if let Some(i) = self.open.take() {
            self.entries[i].offset.set(0., cx)
        }
        self.drag = None;
        cx.notify()
    }
    fn commit(&mut self, i: usize, act: &SwipeAction, leading: bool, cx: &mut Context<Self>) {
        let s = &mut self.entries[i];
        if s.removed {
            return;
        }
        self.open = None;
        self.drag = None;
        if act.keep_row {
            s.offset.set(0., cx)
        } else {
            s.removed = true;
            s.offset.duration(
                (self.width.get() + 2.) * if leading { 1. } else { -1. },
                0.21,
                cx,
            );
            s.height.duration(0., 0.32, cx)
        }
        cx.emit(SwipeSelection {
            row: s.row.id.clone(),
            action: act.id.clone(),
        });
        cx.notify()
    }
    fn begin(&mut self, i: usize, pos: Point<Pixels>, _: &Window, cx: &mut Context<Self>) {
        if self.entries[i].removed {
            return;
        }
        if self.open.is_some() && self.open != Some(i) {
            self.close(cx)
        }
        let base = self.entries[i].offset.value(cx);
        self.drag = Some(Drag {
            row: i,
            start: pos,
            base,
            locked: false,
            armed: false,
            samples: VecDeque::from([(cx.background_executor().now(), f32::from(pos.x))]),
        });
        cx.notify()
    }
    fn motion(&mut self, pos: Point<Pixels>, cx: &mut Context<Self>) {
        let Some(d) = &mut self.drag else { return };
        let dx = f32::from(pos.x - d.start.x);
        let dy = f32::from(pos.y - d.start.y);
        if !d.locked {
            if dx.abs().max(dy.abs()) < 8. {
                return;
            }
            if dy.abs() > dx.abs() {
                self.drag = None;
                return;
            }
            d.locked = true
        }
        let raw = d.base + dx;
        let row = &mut self.entries[d.row];
        let count = if raw >= 0. {
            row.row.leading.len()
        } else {
            row.row.trailing.len()
        };
        let width = self.width.get().max(1.);
        let limit = if count == 0 {
            0.
        } else if self.full {
            width
        } else {
            76. * count as f32
        };
        let over = (raw.abs() - limit).max(0.);
        let current =
            raw.signum() * (raw.abs().min(limit) + width * (1. - 1. / (1. + over * 0.55 / width)));
        row.offset.reset(current);
        let threshold = (width * 0.56).max(count as f32 * 76. + 48.);
        d.armed =
            self.full && count > 0 && current.abs() > threshold - if d.armed { 20. } else { 0. };
        let now = cx.background_executor().now();
        d.samples.push_back((now, pos.x.into()));
        while d.samples.len() > 1
            && d.samples
                .front()
                .is_some_and(|s| now.saturating_duration_since(s.0).as_millis() > 80)
        {
            d.samples.pop_front();
        }
        cx.notify()
    }
    fn release(&mut self, _: &Window, cx: &mut Context<Self>) {
        let Some(d) = self.drag.take() else { return };
        let i = d.row;
        if !d.locked {
            self.close(cx);
            return;
        }
        let now = cx.background_executor().now();
        let velocity = match (d.samples.front(), d.samples.back()) {
            (Some(a), Some(b))
                if now.saturating_duration_since(b.0).as_millis() < 90 && b.0 > a.0 =>
            {
                (b.1 - a.1) / b.0.duration_since(a.0).as_secs_f32()
            }
            _ => 0.,
        };
        let s = &self.entries[i];
        let x = s.offset.value(cx);
        let acts = if x > 0. {
            &s.row.leading
        } else {
            &s.row.trailing
        };
        let open = acts.len() as f32 * 76.;
        let dist = x.abs();
        let out = velocity * x.signum();
        let projected = dist + out * 0.1;
        if !acts.is_empty()
            && self.full
            && ((d.armed && out > -900.)
                || (dist > open
                    && out > 900.
                    && projected > (open + 48.).max(self.width.get() * 0.56)))
        {
            let a = acts[if x > 0. { 0 } else { acts.len() - 1 }].clone();
            self.commit(i, &a, x > 0., cx)
        } else {
            let dest = if !acts.is_empty() && projected > open / 2. {
                x.signum() * open
            } else {
                0.
            };
            self.open = (dest != 0.).then_some(i);
            self.entries[i].offset.set(dest, cx);
            cx.notify()
        }
    }
}
impl Render for SwipeActions {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let r = Theme::global(cx).tokens.radius.md;
        if let Some(i) = self.focus_next.take() {
            self.entries[i]
                .menu
                .update(cx, |menu, cx| menu.focus(w, cx))
        }
        let measured = self.width.clone();
        let mut list = div()
            .on_children_prepainted(move |b, _, _| {
                if let Some(b) = b.first() {
                    measured.set(b.size.width.into())
                }
            })
            .id(self.id.clone())
            .role(Role::List)
            .aria_label(self.label.clone())
            .relative()
            .w_full()
            .flex()
            .flex_col()
            .rounded(r)
            .overflow_hidden()
            .bg(p.panel)
            .on_mouse_down_out(cx.listener(|v, _, _, cx| {
                if v.open.is_some() {
                    v.close(cx)
                }
            }))
            .on_key_down(cx.listener(|v, e: &KeyDownEvent, _, cx| {
                if e.keystroke.key == "escape" {
                    v.close(cx)
                }
            }));
        let visible = |e: &Entry| !e.removed || e.height.value(cx) * self.row_height >= 0.5;
        let first = self.entries.iter().position(visible);
        let last = self.entries.iter().rposition(visible);
        for (i, s) in self.entries.iter_mut().enumerate() {
            let height = s.height.get(w, cx) * self.row_height;
            if s.removed && height < 0.5 {
                continue;
            }
            let x = s.offset.get(w, cx);
            let leading = x >= 0.;
            let acts = if leading {
                &s.row.leading
            } else {
                &s.row.trailing
            };
            let mut row = div()
                .id(("row", i))
                .role(Role::ListItem)
                .aria_label(s.row.label.clone())
                .relative()
                .w_full()
                .flex_none()
                .h(px(height))
                .overflow_hidden();
            if x.abs() > 0.1 && !acts.is_empty() {
                let outer = if leading { 0 } else { acts.len() - 1 };
                let armed = self.drag.as_ref().is_some_and(|d| d.row == i && d.armed) || s.removed;
                s.cover.set(if armed { 1. } else { 0. }, cx);
                let mix = s.cover.get(w, cx);
                let mut rail = div()
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .when(leading, |e| e.left_0())
                    .when(!leading, |e| e.right_0())
                    .w(px(x.abs()))
                    .flex()
                    .overflow_hidden();
                for (j, a) in acts.iter().enumerate() {
                    let base = x.abs() / acts.len() as f32;
                    let part = base * (1. - mix) + if j == outer { x.abs() * mix } else { 0. };
                    let act = a.clone();
                    let bg = if a.destructive {
                        p.danger
                    } else if leading {
                        p.bright
                    } else {
                        p.secondary
                    };
                    let fg = if a.destructive {
                        white()
                    } else if leading {
                        p.inverse
                    } else {
                        p.neutral_inverse
                    };
                    rail = rail.child(
                        div()
                            .id(("rail", j))
                            .aria_label(a.label.clone())
                            .cursor_pointer()
                            .flex_none()
                            .w(px(part))
                            .when(first == Some(i) && j == 0 && leading, |e| e.rounded_tl(r))
                            .when(last == Some(i) && j == 0 && leading, |e| e.rounded_bl(r))
                            .when(first == Some(i) && j + 1 == acts.len() && !leading, |e| {
                                e.rounded_tr(r)
                            })
                            .when(last == Some(i) && j + 1 == acts.len() && !leading, |e| {
                                e.rounded_br(r)
                            })
                            .min_w_0()
                            .overflow_hidden()
                            .flex()
                            .flex_col()
                            .items_center()
                            .justify_center()
                            .gap(px(3.))
                            .bg(bg)
                            .text_color(fg)
                            .text_size(px(11.))
                            .child(icon(a.icon, 18., fg))
                            .child(a.label.clone())
                            .on_click(
                                cx.listener(move |v, _, _, cx| v.commit(i, &act, leading, cx)),
                            ),
                    )
                }
                row = row.child(rail)
            }
            let body = div()
                .relative()
                .h_full()
                .flex_1()
                .min_w_0()
                .child((s.row.body)(w, cx))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |v, e: &MouseDownEvent, w, cx| v.begin(i, e.position, w, cx)),
                )
                .child(s.contact.listen(cx, move |v, e, w, cx| match e.phase {
                    TouchPhase::Started => v.begin(i, e.position, w, cx),
                    TouchPhase::Moved => v.motion(e.position, cx),
                    TouchPhase::Ended => v.release(w, cx),
                    TouchPhase::Cancelled => {
                        v.entries[i].offset.set(0., cx);
                        v.close(cx)
                    }
                }));
            let front = div()
                .absolute()
                .left(px(x))
                .top_0()
                .w_full()
                .h(px(self.row_height))
                .bg(p.panel)
                .when(first == Some(i), |e| e.rounded_t(r))
                .when(last == Some(i), |e| e.rounded_b(r))
                .when(last != Some(i), |e| e.border_b_1().border_color(p.line))
                .flex()
                .items_center()
                .pr(px(7.))
                .child(body)
                .child(s.menu.clone());
            row = row.child(front);
            list = list.child(row);
        }
        if self.drag.is_some() {
            let moved = cx.listener(|v, e: &MouseMoveEvent, _, cx| {
                if e.pressed_button == Some(MouseButton::Left) {
                    v.motion(e.position, cx)
                }
            });
            let up = cx.listener(|v, _: &MouseUpEvent, w, cx| v.release(w, cx));
            list = list.child(
                canvas(
                    |_, _, _| (),
                    move |_, _, w, _| {
                        w.on_mouse_event(move |e: &MouseMoveEvent, phase, w, cx| {
                            if phase.capture() {
                                moved(e, w, cx)
                            }
                        });
                        w.on_mouse_event(move |e: &MouseUpEvent, phase, w, cx| {
                            if phase.capture() && e.button == MouseButton::Left {
                                up(e, w, cx)
                            }
                        })
                    },
                )
                .absolute()
                .inset_0(),
            );
        }

        list
    }
}
