//! Keyboard-contained side sheet with a fixed viewport edge and focus return.
use crate::primitives::{
    family::{Paint, Slide},
    flyout::TriggerBounds,
    glyph::{Glyph, icon},
    nav::*,
};
use gpui::{prelude::*, *};
pub enum DrawerEvent {
    Opened,
    Closed,
}
pub struct Drawer {
    trigger: SharedString,
    title: SharedString,
    body: Content,
    open: bool,
    slide: Slide,
    fade: Fade,
    drag: Option<(Pixels, Pixels, web_time::Instant, f32)>,
    launch: FocusHandle,
    close: FocusHandle,
    scope: FocusHandle,
    container: Option<TriggerBounds>,
}
impl EventEmitter<DrawerEvent> for Drawer {}
impl Drawer {
    pub fn new(
        trigger: impl Into<SharedString>,
        title: impl Into<SharedString>,
        body: Content,
        cx: &App,
    ) -> Self {
        Self {
            trigger: trigger.into(),
            title: title.into(),
            body,
            open: false,
            slide: Slide::new(0.),
            fade: Default::default(),
            drag: None,
            launch: cx.focus_handle().tab_stop(true),
            close: cx.focus_handle().tab_stop(true),
            scope: cx.focus_handle(),
            container: None,
        }
    }
    /// Confine the sheet and scrim to the bounds supplied by its parent.
    pub fn contained(mut self) -> Self {
        self.container = Some(TriggerBounds::default());
        self
    }
    pub fn set_open(&mut self, open: bool, w: &mut Window, cx: &mut Context<Self>) {
        if self.open == open {
            return;
        }
        self.open = open;
        self.drag = None;
        self.fade.set(open, if open { 0.16 } else { 0.10 }, cx);
        self.slide.duration(
            if open { 1. } else { 0. },
            if open { 0.32 } else { 0.22 },
            cx,
        );
        if open {
            self.close.focus(w, cx)
        } else {
            self.launch.focus(w, cx)
        }
        cx.emit(if open {
            DrawerEvent::Opened
        } else {
            DrawerEvent::Closed
        });
        cx.notify()
    }
}
impl Render for Drawer {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let radius = crate::foundation::surface_radius(cx);
        let mut root = div()
            .relative()
            .when_some(self.container.as_ref(), |d, b| {
                d.size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .overflow_hidden()
                    .rounded(radius)
                    .child(b.measure())
            })
            .child(
                button("launch", self.trigger.clone(), cx)
                    .bg(p.panel)
                    .border_1()
                    .border_color(p.line)
                    .track_focus(&self.launch)
                    .aria_expanded(self.open)
                    .child(self.trigger.clone())
                    .on_click(cx.listener(|s, _, w, cx| s.set_open(true, w, cx))),
            );
        let a = self.slide.get(w, cx).clamp(0., 1.08);
        let opacity = self.fade.get(w, cx);
        let still = crate::primitives::family::disabled_motion(cx);
        if !self.open && a < 0.001 && opacity < 0.001 {
            return root;
        }
        let size = self
            .container
            .as_ref()
            .map_or(w.viewport_size(), |b| b.get().size);
        let width = if self.container.is_some() {
            (size.width - px(12.)).max(px(1.)).min(px(352.))
        } else {
            fit(w, 384.)
        };
        let offset = if still { px(0.) } else { -width * (1. - a) };
        let sheet = panel(cx)
            .id("sheet")
            .track_focus(&self.scope)
            .tab_stop(false)
            .role(Role::Dialog)
            .aria_label(self.title.clone())
            .absolute()
            .right(offset)
            .opacity(if still { opacity } else { 1. })
            .top_0()
            .bottom_0()
            .w(width)
            .p_5()
            .rounded_none()
            .rounded_l(radius)
            .when(self.container.is_some(), |e| {
                e.bg(transparent_black()).border_color(transparent_black())
            })
            .gap_4()
            .occlude()
            .overflow_y_scroll()
            .on_click(|_, _, cx| cx.stop_propagation())
            .on_key_down(cx.listener(
                |s, e: &KeyDownEvent, w, cx| match e.keystroke.key.as_str() {
                    "escape" => {
                        s.set_open(false, w, cx);
                        cx.stop_propagation()
                    }
                    "tab" => {
                        if e.keystroke.modifiers.shift {
                            w.focus_prev(cx)
                        } else {
                            w.focus_next(cx)
                        }
                        if !s.scope.contains_focused(w, cx) {
                            s.close.focus(w, cx)
                        }
                        cx.stop_propagation()
                    }
                    _ => {}
                },
            ))
            .child(
                row()
                    .id("drag-handle")
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|s, e: &MouseDownEvent, _, cx| {
                            if !crate::primitives::family::disabled_motion(cx) {
                                s.drag = Some((
                                    e.position.x,
                                    e.position.x,
                                    cx.background_executor().now(),
                                    0.,
                                ));
                            }
                        }),
                    )
                    .justify_between()
                    .child(label(self.title.clone(), 18., p.ink))
                    .child(
                        button("close", tr(cx, "Close", "关闭"), cx)
                            .size(px(32.))
                            .p_0()
                            .track_focus(&self.close)
                            .child(icon(Glyph::Close, 16., p.secondary))
                            .on_click(cx.listener(|s, _, w, cx| s.set_open(false, w, cx))),
                    ),
            )
            .child((self.body)(w, cx));
        let shade = div()
            .id("backdrop")
            .debug_selector(|| "drawer-backdrop".into())
            .relative()
            .w(size.width)
            .h(size.height)
            .bg(black().opacity(0.16 * if still { opacity } else { a }))
            .when(self.container.is_some(), |s| {
                // The GPUI overflow mask is rectangular. Paint the visible portion
                // independently so its trailing corners stay on the stage edge
                // throughout the slide while the full-width content moves intact.
                s.rounded(radius).child(
                    div()
                        .debug_selector(|| "drawer-sheet-face".into())
                        .absolute()
                        .top_0()
                        .bottom_0()
                        .right(offset.max(px(0.)))
                        .w((width + offset.min(px(0.))).max(px(0.)))
                        .rounded(radius)
                        .bg(p.panel)
                        .border_1()
                        .border_color(p.line)
                        .opacity(if still { opacity } else { 1. }),
                )
            })
            .on_mouse_move(cx.listener(move |s, e: &MouseMoveEvent, _, cx| {
                if let Some((start, last, instant, _)) = s.drag {
                    if e.pressed_button != Some(MouseButton::Left) {
                        return;
                    }
                    let now = cx.background_executor().now();
                    let dt = now
                        .saturating_duration_since(instant)
                        .as_secs_f32()
                        .max(0.001);
                    let delta = f32::from(e.position.x - start);
                    let shift = if delta < 0. { delta * 0.12 } else { delta };
                    s.slide
                        .reset((1. - shift / f32::from(width)).clamp(0., 1.08));
                    s.drag = Some((
                        start,
                        e.position.x,
                        now,
                        f32::from(e.position.x - last) / dt,
                    ));
                    cx.notify()
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(move |s, e: &MouseUpEvent, w, cx| {
                    if let Some((start, _, last, velocity)) = s.drag.take() {
                        let distance = e.position.x - start;
                        let flick = cx
                            .background_executor()
                            .now()
                            .saturating_duration_since(last)
                            .as_millis()
                            < 100
                            && velocity > 800.;
                        if distance > width / 3. || (distance > px(18.) && flick) {
                            s.set_open(false, w, cx)
                        } else {
                            s.slide.duration(1., 0.25, cx);
                            cx.notify()
                        }
                    }
                }),
            )
            .overflow_hidden()
            .occlude()
            .on_click(cx.listener(|s, _, w, cx| s.set_open(false, w, cx)))
            .child(sheet);
        root = if self.container.is_some() {
            root.child(shade.absolute().inset_0())
        } else {
            root.child(
                deferred(anchored().position(point(px(0.), px(0.))).child(shade)).with_priority(4),
            )
        };
        root
    }
}
