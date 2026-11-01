//! Anchoring, dismissal and roving selection shared by command menus.
use crate::{
    foundation::Theme,
    primitives::{
        direct_touch::Contact,
        family::{Paint, PhaseClock, Slide, ease},
        glyph::{Glyph, icon},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuItem {
    pub id: SharedString,
    pub label: SharedString,
    pub glyph: Option<Glyph>,
    pub disabled: bool,
    pub destructive: bool,
    pub separator_before: bool,
    pub shortcut: SharedString,
    pub checked: Option<bool>,
}
impl MenuItem {
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            glyph: None,
            disabled: false,
            destructive: false,
            separator_before: false,
            shortcut: "".into(),
            checked: None,
        }
    }
    pub fn with_icon(mut self, g: Glyph) -> Self {
        self.glyph = Some(g);
        self
    }
    pub fn with_separator(mut self) -> Self {
        self.separator_before = true;
        self
    }
    pub fn destructive(mut self) -> Self {
        self.destructive = true;
        self
    }
    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = Some(checked);
        self
    }
    pub fn with_shortcut(mut self, s: impl Into<SharedString>) -> Self {
        self.shortcut = s.into();
        self
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuSelection(pub SharedString);

/// Horizontal placement relative to an anchored trigger.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAlign {
    Start,
    Center,
    #[default]
    End,
}
pub(crate) struct MenuCore {
    pub align: MenuAlign,
    pub id: ElementId,
    pub items: Vec<MenuItem>,
    pub name: SharedString,
    pub trigger: FocusHandle,
    focus: FocusHandle,
    pub open: bool,
    pub anchor: Rc<Cell<Bounds<Pixels>>>,
    point: Option<Point<Pixels>>,
    active: Option<usize>,
    highlight: Slide,
    pub(crate) appear: Slide,
    clock: PhaseClock,
    viewport: Size<Pixels>,
    row_height: f32,
    sheet_drag: Slide,
    sheet_touch: Contact,
}
impl MenuCore {
    pub fn new(id: ElementId, name: SharedString, items: Vec<MenuItem>, cx: &App) -> Self {
        Self {
            id,
            align: MenuAlign::End,
            name,
            items,
            trigger: cx.focus_handle().tab_stop(true),
            focus: cx.focus_handle(),
            open: false,
            anchor: Default::default(),
            point: None,
            active: None,
            highlight: Slide::snappy(0.),
            appear: Slide::snappy(0.),
            clock: Default::default(),
            viewport: Default::default(),
            row_height: 36.,
            sheet_drag: Slide::snappy(0.),
            sheet_touch: Default::default(),
        }
    }
    fn top(&self, i: usize) -> f32 {
        self.items
            .iter()
            .take(i + 1)
            .filter(|r| r.separator_before)
            .count() as f32
            * 9.
            + i as f32 * self.row_height
    }
    pub fn show(
        &mut self,
        point: Option<Point<Pixels>>,
        keyboard: bool,
        last: bool,
        w: &mut Window,
        cx: &mut App,
    ) {
        self.sheet_drag.reset(0.);
        self.point = point;
        self.open = true;
        self.appear.set(1., cx);
        self.clock.restart(cx);
        self.viewport = w.viewport_size();
        self.active = if last {
            self.items.iter().rposition(|r| !r.disabled)
        } else {
            self.items.iter().position(|r| !r.disabled)
        };
        self.visible_highlight(keyboard);
        self.focus.focus(w, cx)
    }
    fn visible_highlight(&mut self, show: bool) {
        if !show {
            self.active = None
        }
        if let Some(i) = self.active {
            self.highlight.reset(self.top(i))
        }
    }
    pub fn close(&mut self, w: &mut Window, cx: &mut App) {
        if self.open {
            self.open = false;
            self.appear.duration(0., 0.13, cx);
            self.trigger.focus(w, cx)
        }
    }
}
pub(crate) trait MenuControl: Sized + 'static {
    fn core(&mut self) -> &mut MenuCore;
    fn selected(&mut self, i: usize, w: &mut Window, cx: &mut Context<Self>);
    fn header(&self, _: &App) -> Option<AnyElement> {
        None
    }
    fn accessory(&self, _: usize, _: &Context<Self>, _: &Window) -> Option<AnyElement> {
        None
    }
    fn horizontal(&mut self, _: usize, _: bool, _: &mut Context<Self>) {}
    fn compact(&self, _: &Window) -> bool {
        false
    }
}
fn key<T: MenuControl>(v: &mut T, e: &KeyDownEvent, w: &mut Window, cx: &mut Context<T>) {
    let m = v.core();
    let k = e.keystroke.key.as_str();
    if !m.open {
        return;
    }
    let enabled: Vec<_> = m
        .items
        .iter()
        .enumerate()
        .filter(|(_, r)| !r.disabled)
        .map(|(i, _)| i)
        .collect();
    let curr = m.active.and_then(|i| enabled.iter().position(|j| *j == i));
    let n = enabled.len();
    match k {
        "escape" | "tab" => {
            m.close(w, cx);
            if k == "tab" {
                if e.keystroke.modifiers.shift {
                    w.focus_prev(cx)
                } else {
                    w.focus_next(cx)
                }
            }
        }
        "enter" | "space" => {
            if let Some(i) = m.active.or_else(|| enabled.first().copied()) {
                v.selected(i, w, cx)
            }
        }
        "left" | "right" => {
            if let Some(i) = m.active {
                v.horizontal(i, k == "right", cx)
            }
        }
        _ if n > 0 => {
            let i = match k {
                "down" => enabled[curr.map_or(0, |j| (j + 1) % n)],
                "up" => enabled[curr.map_or(n - 1, |j| (j + n - 1) % n)],
                "home" => enabled[0],
                "end" => enabled[n - 1],
                s if s.chars().count() == 1 => {
                    let search = s.to_lowercase();
                    let Some(id) = (1..=n)
                        .map(|j| enabled[(curr.unwrap_or(n - 1) + j) % n])
                        .find(|i| m.items[*i].label.to_lowercase().starts_with(&search))
                    else {
                        return;
                    };
                    id
                }
                _ => return,
            };
            m.active = Some(i);
            m.highlight.reset(m.top(i));
        }
        _ => return,
    }
    w.prevent_default();
    cx.stop_propagation();
    cx.notify();
}
/// Add the trigger and an out-of-flow panel to the same stable element.
pub(crate) fn render_menu<T: MenuControl>(
    v: &mut T,
    button: Stateful<Div>,
    context: bool,
    width: f32,
    w: &mut Window,
    cx: &mut Context<T>,
) -> AnyElement {
    let p = Paint::get(cx);
    let rad = Theme::global(cx).tokens.radius.clone();
    let sheet = v.compact(w);
    let header = v.header(cx);
    let header_h = if header.is_some() { 62. } else { 0. };
    let m = v.core();
    let name = m.name.clone();
    let anchor = m.anchor.clone();
    let bounds = anchor.get();
    if context && m.open && m.viewport != w.viewport_size() {
        m.close(w, cx)
    }
    let row_height = if sheet {
        46.
    } else if header.is_some() {
        32.
    } else {
        36.
    };
    m.row_height = row_height;
    let grip_h = if sheet { 22. } else { 0. };
    let is_open = m.open;
    let mut btn = button
        .track_focus(&m.trigger)
        .aria_expanded(is_open)
        .on_click(cx.listener(|v, _, w, cx| {
            let m = v.core();
            if m.open {
                m.close(w, cx)
            } else {
                m.show(None, false, false, w, cx)
            }
            cx.notify()
        }))
        .on_key_down(cx.listener(move |v, e: &KeyDownEvent, w, cx| {
            let k = e.keystroke.key.as_str();
            if matches!(k, "down" | "up" | "enter" | "space")
                || context && (k == "contextmenu" || k == "f10" && e.keystroke.modifiers.shift)
            {
                if !e.is_held {
                    v.core().show(None, true, k == "up", w, cx);
                    cx.notify()
                }
                w.prevent_default();
                cx.stop_propagation()
            }
        }));
    if context {
        btn = btn.on_mouse_down(
            MouseButton::Right,
            cx.listener(|v, e: &MouseDownEvent, w, cx| {
                v.core().show(Some(e.position), false, false, w, cx);
                cx.notify();
                cx.stop_propagation()
            }),
        );
    }
    let mut root = div()
        .relative()
        .child(btn)
        .on_children_prepainted(move |b, _, _| {
            if let Some(b) = b.first() {
                anchor.set(*b)
            }
        })
        .id(m.id.clone());
    let anim = m.appear.get(w, cx).clamp(0., 1.);
    if (!is_open && anim < 0.015) || bounds.size.width <= px(0.) {
        if is_open && bounds.size.width <= px(0.) {
            w.request_animation_frame()
        }
        return root.into_any_element();
    }
    let size = w.viewport_size();
    let height = (12.
        + m.items.len() as f32 * row_height
        + grip_h
        + m.items.iter().filter(|r| r.separator_before).count() as f32 * 9.
        + header_h)
        .min(f32::from(size.height) - 16.);
    let width = if sheet {
        f32::from(size.width)
    } else {
        width.min(f32::from(size.width) - 16.)
    }
    .max(1.);
    let origin = m.point.unwrap_or_else(|| {
        point(
            if context {
                bounds.left() + px(12.)
            } else {
                match m.align {
                    MenuAlign::Start => bounds.left(),
                    MenuAlign::Center => bounds.center().x - px(width * 0.5),
                    MenuAlign::End => bounds.right() - px(width),
                }
            },
            bounds.bottom() + px(8.),
        )
    });
    let y = if sheet {
        f32::from(size.height) - height + (1. - anim) * height + m.sheet_drag.get(w, cx)
    } else if m.point.is_none()
        && origin.y + px(height) > size.height - px(8.)
        && bounds.top() > px(height)
    {
        f32::from(bounds.top()) - height - 8.
    } else {
        f32::from(origin.y)
    };
    let x = if sheet {
        (f32::from(size.width) - width) / 2.
    } else {
        f32::from(origin.x).clamp(8., (f32::from(size.width) - width - 8.).max(8.))
    };
    let y = if sheet {
        y
    } else {
        y.clamp(8., (f32::from(size.height) - height - 8.).max(8.)) - (1. - anim) * 5.
    };
    let trigger = m.anchor.clone();
    let active = m.active;
    let high = m.highlight.get(w, cx);
    let age = m.clock.time(0.35, w, cx);
    let rows = m.items.clone();
    let sheet_touch = m.sheet_touch.clone();
    let focus = m.focus.clone();
    let mut panel = div()
        .id("menu-panel")
        .role(Role::Menu)
        .aria_label(name)
        .track_focus(&focus)
        .tab_stop(false)
        .occlude()
        .relative()
        .w(px(width))
        .max_h(size.height - px(16.))
        .p(px(5.))
        .rounded(rad.md)
        .when(sheet, |e| e.rounded_b(px(0.)))
        .border_1()
        .border_color(p.line)
        .bg(p.panel)
        .shadow_lg()
        .opacity(anim)
        .flex()
        .flex_col()
        .overflow_y_scroll()
        .on_key_down(cx.listener(key::<T>))
        .on_mouse_down_out(cx.listener(move |v, e: &MouseDownEvent, w, cx| {
            if v.core().open && !trigger.get().contains(&e.position) {
                v.core().close(w, cx);
                cx.notify()
            }
        }));
    if sheet {
        let handle = div()
            .relative()
            .h(px(grip_h))
            .flex_none()
            .flex()
            .justify_center()
            .items_center()
            .child(div().w(px(34.)).h(px(4.)).rounded_full().bg(p.faint))
            .child(sheet_touch.listen(cx, |v, e, w, cx| {
                let m = v.core();
                let dy = f32::from(e.position.y - e.start_position.y).max(0.);
                match e.phase {
                    TouchPhase::Moved => m.sheet_drag.reset(dy),
                    TouchPhase::Ended if dy > 48. => m.close(w, cx),
                    TouchPhase::Ended | TouchPhase::Cancelled => m.sheet_drag.set(0., cx),
                    _ => {}
                }
                cx.notify()
            }));
        panel = panel.child(handle);
    }
    if let Some(header) = header {
        panel = panel.child(header)
    }
    if let Some(i) = active {
        panel = panel.child(
            div()
                .absolute()
                .top(px(5. + grip_h + header_h + high))
                .left(px(5.))
                .right(px(5.))
                .h(px(row_height))
                .rounded(rad.md)
                .bg(if rows[i].destructive {
                    p.danger.opacity(0.11)
                } else {
                    p.hover
                }),
        );
    }
    for (i, item) in rows.into_iter().enumerate() {
        if item.separator_before {
            panel = panel.child(div().my(px(4.)).h(px(1.)).bg(p.line).flex_none())
        }
        let ink = if item.destructive { p.danger } else { p.ink };
        let row_name = item.label.clone();
        let mut row = div()
            .id(("menu-item", i))
            .debug_selector(move || row_name.to_string())
            .role(Role::MenuItem)
            .aria_label(item.label.clone())
            .relative()
            .h(px(row_height))
            .flex_none()
            .flex()
            .items_center()
            .gap(px(9.))
            .px(px(10.))
            .text_size(px(14.))
            .text_color(ink)
            .opacity(if item.disabled {
                0.4
            } else {
                ease((age - i.min(4) as f32 * 0.024) / 0.16)
            })
            .when(active == Some(i), |e| e.aria_active_descendant());
        if let Some(checked) = item.checked {
            row = row
                .role(Role::MenuItemCheckBox)
                .aria_toggled(checked.into());
        }
        if let Some(g) = item
            .glyph
            .or(item.checked.filter(|v| *v).map(|_| Glyph::Check))
        {
            row = row.child(icon(
                g,
                16.,
                if item.destructive { ink } else { p.secondary },
            ))
        }
        row = row.child(div().flex_1().truncate().child(item.label));
        if !sheet && !item.shortcut.is_empty() {
            row = row.child(
                div()
                    .text_size(px(11.))
                    .text_color(p.faint)
                    .child(item.shortcut),
            )
        }
        if let Some(tail) = v.accessory(i, cx, w) {
            row = row.child(tail)
        }
        if !item.disabled && is_open {
            row = row
                .cursor_pointer()
                .on_mouse_move(cx.listener(move |v, _, w, cx| {
                    let m = v.core();
                    if m.active != Some(i) {
                        let t = m.top(i);
                        if m.active.is_none() {
                            m.highlight.reset(t)
                        } else {
                            m.highlight.set(t, cx)
                        }
                        m.active = Some(i);
                        m.focus.focus(w, cx);
                        cx.notify()
                    }
                }))
                .on_click(cx.listener(move |v, _, w, cx| {
                    if v.core().open {
                        v.selected(i, w, cx)
                    }
                    cx.stop_propagation()
                }));
        }
        panel = panel.child(row);
    }
    if sheet {
        let scrim = div()
            .id("sheet-scrim")
            .w(size.width)
            .h(size.height)
            .occlude()
            .bg(black().opacity(0.28 * anim))
            .on_click(cx.listener(|v, _, w, cx| {
                v.core().close(w, cx);
                cx.notify()
            }));
        root = root.child(
            deferred(anchored().position(point(px(0.), px(0.))).child(scrim)).with_priority(2),
        );
    }
    root.child(deferred(anchored().position(point(px(x), px(y))).child(panel)).with_priority(3))
        .into_any_element()
}
