//! A stack of navigable sheets, using a centered dialog on wide windows.
use crate::primitives::{
    family::{Paint, Slide},
    flyout::TriggerBounds,
    glyph::{Glyph, icon},
    nav::*,
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};

#[derive(Clone)]
pub struct SheetLink {
    pub id: SharedString,
    pub label: SharedString,
}
impl SheetLink {
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
        }
    }
}
type SheetContent = Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>;
#[derive(Clone)]
pub struct SheetPage {
    pub id: SharedString,
    pub title: SharedString,
    pub description: SharedString,
    pub links: Vec<SheetLink>,
    content: Option<SheetContent>,
}
impl SheetPage {
    pub fn new(
        id: impl Into<SharedString>,
        title: impl Into<SharedString>,
        description: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            description: description.into(),
            links: Vec::new(),
            content: None,
        }
    }
    pub fn with_content(
        mut self,
        content: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        self.content = Some(Rc::new(content));
        self
    }
    pub fn with_links(mut self, links: Vec<SheetLink>) -> Self {
        self.links = links;
        self
    }
}
#[derive(Clone, Debug)]
pub struct SheetStackChanged(pub Vec<SharedString>);
struct Drag {
    start: Pixels,
    origin: f32,
    last: Pixels,
    last_time: web_time::Instant,
    speed: f32,
}
pub struct SheetStack {
    trigger: SharedString,
    pages: Vec<SheetPage>,
    link_focus: Vec<Vec<FocusHandle>>,
    stack: Vec<usize>,
    transition: Slide,
    offset: Slide,
    drag: Option<Drag>,
    touch: Rc<Cell<bool>>,
    launch: FocusHandle,
    close: FocusHandle,
    scope: FocusHandle,
    container: Option<TriggerBounds>,
    back_label: SharedString,
    close_label: SharedString,
    presentation: SheetStackPresentation,
    launcher_context: Option<SheetContent>,
}
impl EventEmitter<SheetStackChanged> for SheetStack {}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SheetStackPresentation {
    #[default]
    Auto,
    Sheet,
    Dialog,
}

impl SheetStackPresentation {
    fn is_dialog(self, width: Pixels) -> bool {
        self == Self::Dialog || (self == Self::Auto && width >= px(640.))
    }
}

impl SheetStack {
    pub fn new(trigger: impl Into<SharedString>, pages: Vec<SheetPage>, cx: &App) -> Self {
        let link_focus = pages
            .iter()
            .map(|page| {
                page.links
                    .iter()
                    .map(|_| cx.focus_handle().tab_stop(true))
                    .collect()
            })
            .collect();
        Self {
            trigger: trigger.into(),
            pages,
            link_focus,
            stack: Vec::new(),
            transition: Slide::new(0.),
            offset: Slide::new(0.),
            drag: None,
            touch: Default::default(),
            launch: cx.focus_handle().tab_stop(true),
            close: cx.focus_handle().tab_stop(true),
            scope: cx.focus_handle(),
            container: None,
            back_label: "Back".into(),
            close_label: "Close all".into(),
            presentation: SheetStackPresentation::Auto,
            launcher_context: None,
        }
    }
    pub fn contained(mut self) -> Self {
        self.container = Some(TriggerBounds::default());
        self
    }
    /// Put a host-provided profile or heading above the launch button.
    pub fn with_launcher_context<E: IntoElement>(
        mut self,
        render: impl Fn(&mut Window, &mut App) -> E + 'static,
    ) -> Self {
        self.launcher_context = Some(Rc::new(move |w, cx| render(w, cx).into_any_element()));
        self
    }
    /// Change the layout used by open and future layers.
    pub fn set_presentation(
        &mut self,
        presentation: SheetStackPresentation,
        cx: &mut Context<Self>,
    ) {
        if self.presentation != presentation {
            self.presentation = presentation;
            self.drag = None;
            self.offset.reset(0.);
            cx.notify();
        }
    }
    fn viewport(&self, w: &Window) -> Size<Pixels> {
        self.container
            .as_ref()
            .map(|b| b.get().size)
            .filter(|b| b.width > px(0.) && b.height > px(0.))
            .unwrap_or_else(|| w.viewport_size())
    }
    pub fn with_action_labels(
        mut self,
        back: impl Into<SharedString>,
        close: impl Into<SharedString>,
    ) -> Self {
        self.back_label = back.into();
        self.close_label = close.into();
        self
    }
    fn emit(&self, cx: &mut Context<Self>) {
        cx.emit(SheetStackChanged(
            self.stack
                .iter()
                .map(|i| self.pages[*i].id.clone())
                .collect(),
        ));
    }
    pub fn push(&mut self, id: &str, w: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(index) = self.pages.iter().position(|p| p.id.as_ref() == id) else {
            return false;
        };
        if let Some(current) = self.stack.iter().position(|i| *i == index) {
            self.stack.truncate(current + 1);
        } else {
            self.stack.push(index);
        }
        let size = self.viewport(w);
        let dialog = self.presentation.is_dialog(size.width);
        self.offset.reset(if dialog {
            18.
        } else {
            f32::from(size.height) + 24.
        });
        self.offset.duration(0., 0.44, cx);
        self.transition
            .duration(1., if dialog { 0.2 } else { 0.3 }, cx);
        self.close.focus(w, cx);
        self.emit(cx);
        cx.notify();
        true
    }
    pub fn pop(&mut self, w: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(closed) = self.stack.pop() else {
            return false;
        };
        self.drag = None;
        if self.stack.is_empty() {
            self.transition.duration(0., 0.24, cx);
            self.launch.focus(w, cx);
        } else {
            let peek = if self.presentation.is_dialog(self.viewport(w).width) {
                14.
            } else {
                10.
            };
            self.offset.reset(-peek);
            self.offset.duration(0., 0.42, cx);
            let parent = *self.stack.last().unwrap();
            if let Some(link) = self.pages[parent]
                .links
                .iter()
                .position(|link| link.id == self.pages[closed].id)
            {
                self.link_focus[parent][link].focus(w, cx);
            } else {
                self.close.focus(w, cx);
            }
        }
        self.emit(cx);
        cx.notify();
        true
    }
    pub fn close(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if !self.stack.is_empty() {
            self.stack.clear();
            self.drag = None;
            self.transition.duration(0., 0.24, cx);
            self.launch.focus(w, cx);
            self.emit(cx);
            cx.notify();
        }
    }
    pub fn stack(&self) -> Vec<SharedString> {
        self.stack
            .iter()
            .map(|i| self.pages[*i].id.clone())
            .collect()
    }
    fn begin(&mut self, y: Pixels, cx: &mut Context<Self>) {
        self.drag = Some(Drag {
            start: y,
            origin: self.offset.value(cx),
            last: y,
            last_time: cx.background_executor().now(),
            speed: 0.,
        });
    }
    fn move_drag(&mut self, y: Pixels, cx: &mut Context<Self>) {
        if let Some(drag) = &mut self.drag {
            let now = cx.background_executor().now();
            drag.speed = f32::from(y - drag.last)
                / now
                    .saturating_duration_since(drag.last_time)
                    .as_secs_f32()
                    .max(0.001);
            drag.last = y;
            drag.last_time = now;
            let delta = drag.origin + f32::from(y - drag.start);
            self.offset
                .reset(if delta < 0. { delta * 0.12 } else { delta });
            cx.notify();
        }
    }
    fn end(&mut self, cancel: bool, w: &mut Window, cx: &mut Context<Self>) {
        let Some(drag) = self.drag.take() else {
            return;
        };
        let fresh = cx
            .background_executor()
            .now()
            .saturating_duration_since(drag.last_time)
            .as_millis()
            < 100;
        let size = self.viewport(w);
        let (_, _, _, height, _) = sheet_frame(
            f32::from(size.width),
            f32::from(size.height),
            0,
            self.presentation.is_dialog(size.width),
        );
        let offset = self.offset.value(cx);
        if !cancel && (offset > 0.35 * height || (fresh && drag.speed > 450. && offset > 16.)) {
            self.pop(w, cx);
        } else {
            self.offset.duration(0., 0.44, cx);
            cx.notify();
        }
    }
}
impl Render for SheetStack {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let r = crate::foundation::surface_radius(cx);
        let first = self.pages.first().map(|p| p.id.clone());
        let context = self.launcher_context.as_ref().map(|body| body(w, cx));
        let launch = button("stack-open", self.trigger.clone(), cx)
            .bg(if context.is_some() { p.bright } else { p.panel })
            .when(context.is_some(), |b| b.text_color(p.inverse))
            .border_1()
            .border_color(p.line)
            .track_focus(&self.launch)
            .child(self.trigger.clone())
            .on_click(cx.listener(move |s, _, w, cx| {
                if let Some(id) = first.as_ref() {
                    s.push(id, w, cx);
                }
            }));
        let root = div()
            .relative()
            .when_some(self.container.as_ref(), |d, b| {
                d.size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .overflow_hidden()
                    .rounded(r)
                    .child(b.measure())
            })
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(12.))
                    .children(context)
                    .child(launch),
            );
        let active = self.transition.get(w, cx).clamp(0., 1.);
        if self.stack.is_empty() && active < 0.001 {
            return root;
        }
        let size = self.viewport(w);
        let as_dialog = self.presentation.is_dialog(size.width);
        let offset = self.offset.get(w, cx);
        let veil = match crate::foundation::Theme::global(cx).mode {
            crate::foundation::ThemeMode::Dark => 0.56,
            crate::foundation::ThemeMode::Light => 0.32,
        };
        let dim = match crate::foundation::Theme::global(cx).mode {
            crate::foundation::ThemeMode::Dark => 0.34,
            crate::foundation::ThemeMode::Light => 0.16,
        };
        let mut layers = div()
            .id("sheet-stack-layer")
            .relative()
            .w(size.width)
            .h(size.height)
            .bg(black().opacity(active * veil))
            .overflow_hidden()
            .occlude()
            .on_click(cx.listener(|s, _, w, cx| s.close(w, cx)));
        for (depth, &page_index) in self.stack.iter().rev().enumerate().rev() {
            let page = &self.pages[page_index];
            let front = depth == 0;
            let (mut left, mut top, mut width, mut height, dialog) = sheet_frame(
                f32::from(size.width),
                f32::from(size.height),
                depth,
                as_dialog,
            );
            let scale = if crate::primitives::family::disabled_motion(cx) {
                1.
            } else {
                1. - 0.05 * depth as f32
            };
            let shrink_x = width * (1. - scale) * 0.5;
            let shrink_y = height * (1. - scale) * 0.5;
            left += shrink_x;
            top += shrink_y + if front { offset } else { 0. };
            width = (width * scale).max(1.);
            height = (height * scale).max(1.);
            let mut panel = panel(cx)
                .id(("stack-sheet", depth))
                .debug_selector(move || format!("stack-sheet-{depth}"))
                .absolute()
                .left(px(left))
                .top(px(top))
                .w(px(width))
                .h(px(height))
                .when(dialog, |d| d.rounded(r))
                .when(!dialog, |d| d.rounded_t(r))
                .shadow_lg()
                .p_0()
                .gap_0()
                .overflow_y_scroll()
                .on_click(|_, _, cx| cx.stop_propagation());
            if front {
                let active_touch = self.touch.clone();
                let touch = cx.listener(|s, e: &TouchDragEvent, w, cx| match e.phase {
                    TouchPhase::Started => s.begin(e.position.y, cx),
                    TouchPhase::Moved => s.move_drag(e.position.y, cx),
                    TouchPhase::Ended => s.end(false, w, cx),
                    TouchPhase::Cancelled => s.end(true, w, cx),
                });
                let drag = row()
                    .relative()
                    .id("stack-handle")
                    .pt(px(8.))
                    .justify_center()
                    .cursor_pointer()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|s, e: &MouseDownEvent, _, cx| s.begin(e.position.y, cx)),
                    )
                    .child(div().w(px(36.)).h(px(5.)).rounded_full().bg(p.faint))
                    .child(
                        canvas(
                            |b, w, _| w.insert_hitbox(b, HitboxBehavior::Normal),
                            move |_, hit, w, _| {
                                w.on_mouse_event(move |e: &TouchDragEvent, phase, w, cx| {
                                    if phase.bubble() {
                                        if e.phase == TouchPhase::Started {
                                            active_touch.set(hit.is_hovered(w));
                                        }
                                        if !active_touch.get() {
                                            return;
                                        }
                                        if matches!(
                                            e.phase,
                                            TouchPhase::Ended | TouchPhase::Cancelled
                                        ) {
                                            active_touch.set(false);
                                        }
                                        if e.phase == TouchPhase::Started {
                                            w.prevent_default();
                                        }
                                        touch(e, w, cx);
                                        cx.stop_propagation();
                                    }
                                });
                            },
                        )
                        .absolute()
                        .inset_0(),
                    );
                panel = panel
                    .track_focus(&self.scope)
                    .tab_stop(false)
                    .role(Role::Dialog)
                    .aria_label(page.title.clone())
                    .child(drag)
                    .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                        match e.keystroke.key.as_str() {
                            "escape" => {
                                s.pop(w, cx);
                                cx.stop_propagation();
                            }
                            "tab" => {
                                if e.keystroke.modifiers.shift {
                                    w.focus_prev(cx);
                                } else {
                                    w.focus_next(cx);
                                }
                                if !s.scope.contains_focused(w, cx) {
                                    s.close.focus(w, cx);
                                }
                                cx.stop_propagation();
                            }
                            _ => {}
                        }
                    }));
                let mut lead = div().flex().min_w_0();
                if self.stack.len() > 1 {
                    let parent_title = self.pages[self.stack[self.stack.len() - 2]].title.clone();
                    let back_name = format!("{} {}", self.back_label, parent_title);
                    lead = lead.child(
                        button("stack-back", back_name, cx)
                            .h(px(36.))
                            .pl(px(4.))
                            .pr(px(10.))
                            .gap(px(2.))
                            .rounded_full()
                            .text_size(px(14.))
                            .line_height(px(20.))
                            .text_color(p.secondary)
                            .child(parent_title)
                            .on_click(cx.listener(|s, _, w, cx| {
                                s.pop(w, cx);
                            })),
                    );
                }
                let header = div()
                    .relative()
                    .min_h(px(52.))
                    .px(px(10.))
                    .pt(px(6.))
                    .pb(px(2.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .child(lead)
                    .child(
                        div()
                            .absolute()
                            .left(px(72.))
                            .right(px(72.))
                            .text_center()
                            .text_size(px(16.))
                            .line_height(px(22.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(page.title.clone()),
                    )
                    .child(
                        button("stack-close", self.close_label.clone(), cx)
                            .size(px(36.))
                            .rounded_full()
                            .p_0()
                            .bg(p.tray)
                            .text_color(p.secondary)
                            .track_focus(&self.close)
                            .child(icon(Glyph::Close, 16., p.secondary))
                            .on_click(cx.listener(|s, _, w, cx| s.close(w, cx))),
                    );
                panel = panel.child(header).child(
                    div()
                        .px(px(24.))
                        .pb(px(4.))
                        .text_center()
                        .text_size(px(14.))
                        .line_height(px(20.))
                        .text_color(p.secondary)
                        .child(page.description.clone()),
                );
                if let Some(content) = &page.content {
                    panel = panel.child(content(w, cx));
                }
                let mut links = div().flex().flex_col().px(px(16.)).py(px(14.)).gap(px(12.));
                for (i, link) in page.links.iter().enumerate() {
                    let id = link.id.clone();
                    links = links.child(
                        button(("stack-link", i), link.label.clone(), cx)
                            .track_focus(&self.link_focus[page_index][i])
                            .w_full()
                            .h(px(44.))
                            .px(px(12.))
                            .gap(px(12.))
                            .border_1()
                            .border_color(p.line)
                            .rounded(r)
                            .text_size(px(14.))
                            .line_height(px(20.))
                            .font_weight(FontWeight::MEDIUM)
                            .justify_between()
                            .child(div().min_w_0().truncate().child(link.label.clone()))
                            .child(div().size(px(16.)).flex_none().child(icon(
                                Glyph::Arrow,
                                16.,
                                p.secondary,
                            )))
                            .on_click(cx.listener(move |s, _, w, cx| {
                                s.push(&id, w, cx);
                            })),
                    );
                }
                if !page.links.is_empty() {
                    panel = panel.child(links);
                }
            } else {
                panel = panel
                    .child(
                        div()
                            .absolute()
                            .inset_0()
                            .bg(black().opacity((depth as f32).min(2.) * 0.5 * dim)),
                    )
                    .child(label(page.title.clone(), 18., p.secondary));
            }
            layers = layers.child(panel);
        }
        layers = layers
            .on_mouse_move(cx.listener(|s, e: &MouseMoveEvent, _, cx| {
                if e.pressed_button == Some(MouseButton::Left) {
                    s.move_drag(e.position.y, cx)
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|s, _, w, cx| s.end(false, w, cx)),
            );
        if self.container.is_some() {
            root.child(layers.absolute().inset_0())
        } else {
            root.child(
                deferred(anchored().position(point(px(0.), px(0.))).child(layers)).with_priority(4),
            )
        }
    }
}

/// Sheet frames follow the source: 560px sheets leave 20px, dialogs are 440px and leave 80px.
fn sheet_frame(width: f32, height: f32, depth: usize, dialog: bool) -> (f32, f32, f32, f32, bool) {
    let depth = depth as f32;
    if dialog {
        let panel_w = (width - 32.).clamp(1., 440.);
        let panel_h = (height - 80. - 14. * depth).max(160.);
        let left = (width - panel_w) * 0.5;
        let top = (height - panel_h) * 0.5 - 14. * depth;
        (left, top, panel_w, panel_h, true)
    } else {
        let panel_w = width.clamp(1., 560.);
        let panel_h = (height - 20. - 10. * depth).max(160.);
        let left = (width - panel_w) * 0.5;
        let top = height - panel_h - 10. * depth;
        (left, top, panel_w, panel_h, false)
    }
}

#[cfg(test)]
mod tests {
    use super::{SheetLink, SheetPage, SheetStack, sheet_frame};
    use gpui::{TestAppContext, px, size};

    #[test]
    fn links_stay_inside_the_sheet_at_narrow_widths() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (stack, window) = app.add_window_view(|_, cx| {
            SheetStack::new(
                "Open",
                vec![
                    SheetPage::new("settings", "Settings", "Account settings")
                        .with_links(vec![SheetLink::new("privacy", "Privacy")]),
                    SheetPage::new("privacy", "Privacy", "Manage visibility"),
                ],
                cx,
            )
        });
        window.simulate_resize(size(px(320.), px(600.)));
        window.update(|w, cx| stack.update(cx, |stack, cx| stack.push("settings", w, cx)));
        window
            .executor()
            .advance_clock(std::time::Duration::from_secs(1));
        window.run_until_parked();
        let panel = window
            .debug_bounds("stack-sheet-0")
            .expect("front settings panel");
        let link = window.debug_bounds("Privacy").expect("privacy link");
        assert!(link.left() >= panel.left() + px(16.));
        assert!(link.right() <= panel.right() - px(16.));
    }

    #[test]
    fn sheet_and_dialog_frames_follow_the_source() {
        let (left, top, width, height, dialog) = sheet_frame(800., 700., 0, true);
        assert!(dialog);
        assert!((width - 440.).abs() < 0.001);
        assert!((height - 620.).abs() < 0.001);
        assert!((left - 180.).abs() < 0.001);
        assert!((top - 40.).abs() < 0.001);
        let (_, top, width, height, dialog) = sheet_frame(400., 700., 1, false);
        assert!(!dialog);
        assert!((width - 400.).abs() < 0.001);
        assert!((height - 670.).abs() < 0.001);
        assert!((top - 20.).abs() < 0.001);
    }
}
