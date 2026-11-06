//! Anchoring, dismissal and roving selection shared by command menus.
use crate::{
    foundation::{Theme, ThemeMode},
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
    pub detail: SharedString,
}
fn first_enabled(items: &[MenuItem]) -> Option<usize> {
    items.iter().position(|item| !item.disabled)
}
fn morph_geometry(
    start: Bounds<Pixels>,
    end: (f32, f32, f32, f32),
    progress: f32,
) -> (f32, f32, f32, f32) {
    let t = progress.clamp(0., 1.);
    (
        f32::from(start.left()) * (1. - t) + end.0 * t,
        f32::from(start.top()) * (1. - t) + end.1 * t,
        f32::from(start.size.width) * (1. - t) + end.2 * t,
        f32::from(start.size.height) * (1. - t) + end.3 * t,
    )
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
            detail: "".into(),
        }
    }
    pub fn with_detail(mut self, detail: impl Into<SharedString>) -> Self {
        self.detail = detail.into();
        self
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

/// Optional web host bridge for wheel events over an open scrollable menu.
pub struct MenuScrollRegion(pub fn(Bounds<Pixels>, bool, bool));
impl Global for MenuScrollRegion {}

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
    scroll: ScrollHandle,
    pending_reveal: Option<usize>,
}
impl MenuCore {
    /// Replace a panel's rows while keeping its focus and resetting its active row.
    /// This prevents a previous row index from addressing the new, shorter list.
    pub(crate) fn replace_items(&mut self, items: Vec<MenuItem>) {
        self.items = items;
        self.active = first_enabled(&self.items);
        self.pending_reveal = self.active;
        if let Some(i) = self.active {
            self.highlight.reset(self.top(i));
        }
    }
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
            scroll: ScrollHandle::new(),
            pending_reveal: None,
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
        self.scroll.set_offset(gpui::point(px(0.), px(0.)));
        self.point = point;
        self.open = true;
        self.appear.set(1., cx);
        self.clock.restart(cx);
        self.viewport = w.viewport_size();
        self.active = if last {
            self.items.iter().rposition(|r| !r.disabled)
        } else {
            first_enabled(&self.items)
        };
        self.visible_highlight(keyboard);
        self.pending_reveal = if keyboard { self.active } else { None };
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
    /// Optional trigger-to-panel geometry transition for media overlays.
    fn morph_panel(&self) -> bool {
        false
    }
    /// Roster rows use a full-row muted hover instead of the compact menu highlight.
    fn roster(&self) -> bool {
        false
    }
    fn leading(&self, _: usize, _: bool, _: &Context<Self>) -> Option<AnyElement> {
        None
    }
    /// In-panel content translation and opacity during a submenu transition.
    fn content_motion(&mut self, _: &Window, _: &App) -> (f32, f32) {
        (0., 1.)
    }
}
fn open_panel<T: MenuControl>(
    v: &mut T,
    point: Option<Point<Pixels>>,
    keyboard: bool,
    last: bool,
    w: &mut Window,
    cx: &mut Context<T>,
) {
    let morph = v.morph_panel();
    let m = v.core();
    let was_open = m.open;
    m.show(point, keyboard, last, w, cx);
    if morph && !was_open {
        m.appear.reset(0.);
        m.appear.duration(1., 0.36, cx);
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
            m.pending_reveal = Some(i);
        }
        _ => return,
    }
    w.prevent_default();
    cx.stop_propagation();
    cx.notify();
}
fn glass_panel_shadow(mode: ThemeMode) -> Vec<BoxShadow> {
    let ink = black();
    let dark = mode == ThemeMode::Dark;
    vec![
        BoxShadow {
            color: white().opacity(if dark { 0.12 } else { 0.5 }),
            offset: point(px(0.), px(1.)),
            blur_radius: px(0.),
            spread_radius: px(0.),
            inset: true,
        },
        BoxShadow {
            color: ink.opacity(if dark { 0.3 } else { 0.06 }),
            offset: point(px(0.), px(1.)),
            blur_radius: px(2.),
            spread_radius: px(0.),
            inset: false,
        },
        BoxShadow {
            color: ink.opacity(if dark { 0.5 } else { 0.2 }),
            offset: point(px(0.), px(if dark { 12. } else { 10. })),
            blur_radius: px(if dark { 32. } else { 28. }),
            spread_radius: px(0.),
            inset: false,
        },
        BoxShadow {
            color: ink.opacity(if dark { 0.55 } else { 0.24 }),
            offset: point(px(0.), px(28.)),
            blur_radius: px(if dark { 64. } else { 60. }),
            spread_radius: px(0.),
            inset: false,
        },
    ]
}
fn glass_hover_shadow(mode: ThemeMode) -> Vec<BoxShadow> {
    vec![
        BoxShadow {
            color: white().opacity(if mode == ThemeMode::Dark { 0.14 } else { 0.7 }),
            offset: point(px(0.), px(1.)),
            blur_radius: px(0.),
            spread_radius: px(0.),
            inset: true,
        },
        BoxShadow {
            color: black().opacity(0.06),
            offset: point(px(0.), px(1.)),
            blur_radius: px(2.),
            spread_radius: px(0.),
            inset: false,
        },
    ]
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
    let morph = v.morph_panel() && !sheet;
    let (content_x, content_alpha) = v.content_motion(w, cx);
    let roster = v.roster();
    let header = v.header(cx);
    let has_header = header.is_some();
    let header_h = if !has_header {
        0.
    } else if roster {
        34.
    } else {
        62.
    };
    let m = v.core();
    let name = m.name.clone();
    let anchor = m.anchor.clone();
    let bounds = anchor.get();
    if context && m.open && m.viewport != w.viewport_size() {
        m.close(w, cx)
    }
    let row_height = if sheet {
        46.
    } else if roster {
        48.
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
            if v.core().open {
                v.core().close(w, cx);
            } else {
                open_panel(v, None, false, false, w, cx);
            }
            cx.notify()
        }))
        .on_key_down(cx.listener(move |v, e: &KeyDownEvent, w, cx| {
            let k = e.keystroke.key.as_str();
            if matches!(k, "down" | "up" | "enter" | "space")
                || context && (k == "contextmenu" || k == "f10" && e.keystroke.modifiers.shift)
            {
                if !e.is_held {
                    open_panel(v, None, true, k == "up", w, cx);
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
                open_panel(v, Some(e.position), false, false, w, cx);
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
    // Rows follow an optional sheet handle, header, active highlight, and
    // separators in the tracked element's direct child list.
    if let Some(i) = m.pending_reveal
        && is_open
    {
        if m.scroll.bounds().size.height > px(0.) && (!morph || anim >= 0.99) {
            let prefix =
                usize::from(sheet) + usize::from(has_header) + usize::from(m.active.is_some());
            let separators = m
                .items
                .iter()
                .take(i + 1)
                .filter(|row| row.separator_before)
                .count();
            m.scroll.scroll_to_item(prefix + separators + i);
            m.pending_reveal = None;
        } else if w.is_visible() {
            w.request_animation_frame();
        }
    }
    let size = w.viewport_size();
    let separator_span = if morph { 11. } else { 9. };
    let pad = if morph || roster { 6. } else { 5. };
    let height = (pad * 2.
        + m.items.len() as f32 * row_height
        + grip_h
        + m.items.iter().filter(|r| r.separator_before).count() as f32 * separator_span
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
    let (x, y, display_width, display_height) = if morph {
        morph_geometry(bounds, (x, y, width, height), anim)
    } else {
        (x, y, width, height)
    };
    let trigger = m.anchor.clone();
    let active = m.active;
    let high = m.highlight.get(w, cx);
    let age = m.clock.time(0.35, w, cx);
    let rows = m.items.clone();
    let sheet_touch = m.sheet_touch.clone();
    let focus = m.focus.clone();
    let menu_scroll = m.scroll.clone();
    let mut panel = div()
        .id("menu-panel")
        .role(Role::Menu)
        .aria_label(name)
        .track_focus(&focus)
        .tab_stop(false)
        .occlude()
        .relative()
        .w(px(display_width))
        .max_h(size.height - px(16.))
        .p(px(pad))
        .rounded(rad.md)
        .when(sheet, |e| e.rounded_b(px(0.)))
        .border_1()
        .border_color(if morph { p.ink.opacity(0.1) } else { p.line })
        .bg(if morph {
            p.panel
                .opacity(if Theme::global(cx).mode == ThemeMode::Dark {
                    0.56
                } else {
                    0.62
                })
        } else {
            p.panel
        })
        .when(morph, |e| {
            e.shadow(glass_panel_shadow(Theme::global(cx).mode))
        })
        .when(!morph, |e| e.shadow_lg())
        .opacity(anim)
        .flex()
        .flex_col()
        .when(morph, |v| v.h(px(display_height)).overflow_hidden())
        .when(!morph || anim >= 0.99, |v| v.overflow_y_scroll())
        .track_scroll(&menu_scroll)
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
        let mode = Theme::global(cx).mode;
        let dark = mode == ThemeMode::Dark;
        panel = panel.child(
            div()
                .absolute()
                .top(px(pad + grip_h + header_h + high))
                .left(px(pad))
                .right(px(pad))
                .h(px(row_height))
                .rounded(if roster { px(2.) } else { rad.md })
                .bg(if roster && !rows[i].destructive {
                    p.tray
                } else if rows[i].destructive {
                    p.danger.opacity(if morph && dark {
                        0.24
                    } else if morph {
                        0.14
                    } else {
                        0.11
                    })
                } else if morph {
                    p.bright.opacity(if dark { 0.26 } else { 0.16 })
                } else {
                    p.hover
                })
                .when(morph, |e| e.shadow(glass_hover_shadow(mode))),
        );
    }
    for (i, item) in rows.into_iter().enumerate() {
        if item.separator_before {
            panel = panel.child(if morph {
                div()
                    .mx(px(10.))
                    .my(px(5.))
                    .h(px(1.))
                    .bg(p.ink.opacity(0.09))
                    .flex_none()
            } else {
                div().my(px(4.)).h(px(1.)).bg(p.line).flex_none()
            })
        }
        let ink = if item.destructive { p.danger } else { p.ink };
        let row_name = if roster && !item.detail.is_empty() {
            format!("{} · {}", item.label, item.detail)
        } else {
            item.label.to_string()
        };
        let mut row = div()
            .id(("menu-item", i))
            .debug_selector(move || row_name.to_string())
            .role(Role::MenuItem)
            .aria_label(item.label.clone())
            .relative()
            .left(px(content_x))
            .h(px(row_height))
            .flex_none()
            .flex()
            .items_center()
            .gap(px(if morph || roster { 10. } else { 9. }))
            .px(px(10.))
            .text_size(px(14.))
            .text_color(ink)
            .opacity(
                content_alpha
                    * if item.disabled {
                        0.4
                    } else {
                        ease((age - i.min(4) as f32 * 0.024) / 0.16)
                    },
            )
            .when(active == Some(i), |e| e.aria_active_descendant());
        if let Some(checked) = item.checked {
            row = row
                .role(Role::MenuItemCheckBox)
                .aria_toggled(checked.into());
        }
        if let Some(lead) = v.leading(i, active == Some(i), cx) {
            row = row.child(lead);
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
        row = row.child(if item.detail.is_empty() {
            div()
                .flex_1()
                .truncate()
                .child(item.label)
                .into_any_element()
        } else {
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(
                    div()
                        .truncate()
                        .text_size(px(14.))
                        .line_height(px(20.))
                        .font_weight(FontWeight::MEDIUM)
                        .child(item.label),
                )
                .child(
                    div()
                        .truncate()
                        .text_size(px(12.))
                        .line_height(px(20.))
                        .text_color(p.faint)
                        .child(item.detail),
                )
                .into_any_element()
        });
        if !sheet && !item.shortcut.is_empty() {
            row = row.child(
                div()
                    .text_size(px(if morph { 12. } else { 11. }))
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
    let report_scroll = is_open && (!morph || anim >= 0.99);
    let monitor = canvas(
        |_, _, _| (),
        move |bounds, _, _, cx| {
            if report_scroll && let Some(host) = cx.try_global::<MenuScrollRegion>() {
                let top = -menu_scroll.offset().y;
                (host.0)(
                    bounds,
                    top > px(0.1),
                    top + px(0.1) < menu_scroll.max_offset().y,
                );
            }
        },
    )
    .absolute()
    .inset_0();
    let content = div()
        .relative()
        .w(px(display_width))
        .h(px(display_height))
        .child(panel)
        .child(monitor);
    root.child(deferred(anchored().position(point(px(x), px(y))).child(content)).with_priority(3))
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::{MenuControl, MenuCore, MenuItem, MenuScrollRegion, morph_geometry, render_menu};
    use gpui::prelude::*;
    use gpui::{
        Bounds, Context, Modifiers, PlatformInput, Render, ScrollDelta, ScrollEvent,
        TestAppContext, VisualTestContext, Window, point, px, size,
    };
    use std::{cell::Cell, time::Duration};

    thread_local! {
        static REGION: Cell<(Bounds<gpui::Pixels>, bool, bool, usize)> = Cell::new((Bounds::default(), false, false, 0));
    }
    struct LongMenu {
        core: MenuCore,
    }
    impl MenuControl for LongMenu {
        fn core(&mut self) -> &mut MenuCore {
            &mut self.core
        }
        fn selected(&mut self, _: usize, w: &mut Window, cx: &mut Context<Self>) {
            self.core.close(w, cx);
            cx.notify();
        }
    }
    impl Render for LongMenu {
        fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let button = crate::primitives::family::face(
                "long-trigger",
                "Open actions",
                36.,
                8.,
                crate::primitives::family::Tone::Surface,
                cx,
            )
            .child("Open actions");
            render_menu(self, button, false, 192., w, cx)
        }
    }
    fn tick(w: &mut VisualTestContext, ms: u64) {
        w.executor().advance_clock(Duration::from_millis(ms));
        w.run_until_parked();
        w.update(|w, cx| {
            w.simulate_next_frame(cx);
            w.refresh();
        });
        w.run_until_parked();
    }

    #[test]
    fn long_menu_reports_scroll_boundaries_reveals_keyboard_selection_and_restores_focus() {
        let mut app = TestAppContext::single();
        app.update(|cx| {
            crate::init(cx);
            cx.set_global(MenuScrollRegion(|bounds, before, after| {
                REGION.with(|cell| {
                    let old = cell.get();
                    cell.set((bounds, before, after, old.3 + 1));
                });
            }));
        });
        let (control, mut w) = app.add_window_view(|_, cx| {
            let rows = (0..30)
                .map(|i| {
                    let row = MenuItem::new(format!("id-{i}"), format!("Action {i}"));
                    if i == 10 { row.with_separator() } else { row }
                })
                .collect();
            LongMenu {
                core: MenuCore::new("long-menu".into(), "Actions".into(), rows, cx),
            }
        });
        w.simulate_resize(size(px(320.), px(240.)));
        tick(&mut w, 100);
        let trigger = w.debug_bounds("Open actions").expect("trigger").center();
        w.simulate_click(trigger, Modifiers::default());
        tick(&mut w, 400);
        let (region, before, after, _) = REGION.with(Cell::get);
        assert!(!before && after, "the open menu initially scrolls down");
        w.update(|win, cx| {
            win.dispatch_event(
                PlatformInput::Scroll(ScrollEvent {
                    position: region.center(),
                    delta: ScrollDelta::Pixels(point(px(0.), px(-100.))),
                    ..Default::default()
                }),
                cx,
            )
        });
        tick(&mut w, 80);
        assert!(
            REGION.with(|cell| cell.get().1),
            "wheel scroll must reach the menu"
        );
        w.simulate_keystrokes("end");
        tick(&mut w, 80);
        tick(&mut w, 80);
        w.update(|_, cx| {
            let menu = control.read(cx);
            assert!(menu.core.scroll.offset().y < -px(1.));
            // The active highlight and one separator precede the final row.
            let final_row = menu.core.scroll.bounds_for_item(31).expect("last action");
            let viewport = menu.core.scroll.bounds();
            let offset = menu.core.scroll.offset().y;
            assert!(
                final_row.top() + offset >= viewport.top() - px(1.)
                    && final_row.bottom() + offset <= viewport.bottom() + px(1.),
                "End reveals the final action"
            );
        });
        w.simulate_keystrokes("escape");
        tick(&mut w, 400);
        let closed_count = REGION.with(|cell| cell.get().3);
        tick(&mut w, 100);
        assert_eq!(
            REGION.with(|cell| cell.get().3),
            closed_count,
            "closed menus no longer register a wheel region"
        );
        w.update(|window, cx| assert!(control.read(cx).core.trigger.is_focused(window)));
    }

    #[test]
    fn morph_geometry_starts_at_trigger_and_ends_at_panel() {
        let trigger = Bounds {
            origin: point(px(10.), px(20.)),
            size: size(px(90.), px(40.)),
        };
        let panel = (15., 65., 256., 200.);
        assert_eq!(morph_geometry(trigger, panel, 0.), (10., 20., 90., 40.));
        assert_eq!(morph_geometry(trigger, panel, 1.), panel);
        assert_eq!(
            morph_geometry(trigger, panel, 0.5),
            (12.5, 42.5, 173., 120.)
        );
    }

    #[test]
    fn replacing_rows_resets_out_of_range_highlight_and_skips_disabled() {
        let app = TestAppContext::single();
        app.update(|cx| {
            let mut menu = MenuCore::new(
                "actions".into(),
                "Actions".into(),
                (0..6)
                    .map(|i| MenuItem::new(format!("{i}"), "Old"))
                    .collect(),
                cx,
            );
            menu.active = Some(5);
            menu.highlight.reset(menu.top(5));
            menu.replace_items(vec![
                MenuItem::new("disabled", "Disabled").disabled(),
                MenuItem::new("next", "Next"),
            ]);
            assert_eq!(menu.active, Some(1));
            assert_eq!(menu.highlight.value(cx), 36.);
            menu.replace_items(vec![MenuItem::new("disabled", "Disabled").disabled()]);
            assert_eq!(menu.active, None);
            menu.replace_items(Vec::new());
            assert_eq!(menu.active, None);
        });
    }
}
