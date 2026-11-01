//! An actionable empty view with animated copy and icon replacement.
use crate::{
    foundation::surface_radius,
    primitives::{
        control::activate,
        family::{Paint, PhaseClock, Slide, Tone, ease, face},
        glyph::{Glyph, icon},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmptyContent {
    pub title: SharedString,
    pub description: SharedString,
    pub glyph: Glyph,
    pub action: Option<SharedString>,
    pub secondary_action: bool,
}
impl EmptyContent {
    pub fn new(title: impl Into<SharedString>, description: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            description: description.into(),
            glyph: Glyph::ClosedFolder,
            secondary_action: false,
            action: None,
        }
    }
    pub fn with_icon(mut self, glyph: Glyph) -> Self {
        self.glyph = glyph;
        self
    }
    pub fn with_secondary_action(mut self, s: impl Into<SharedString>) -> Self {
        self.secondary_action = true;
        self.action = Some(s.into());
        self
    }
    pub fn with_action(mut self, s: impl Into<SharedString>) -> Self {
        self.secondary_action = false;
        self.action = Some(s.into());
        self
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EmptyAction;
pub struct EmptyState {
    id: ElementId,
    content: EmptyContent,
    old: Option<EmptyContent>,
    clock: PhaseClock,
    height: Slide,
    measured: Rc<Cell<f32>>,
}
impl EventEmitter<EmptyAction> for EmptyState {}
impl EmptyState {
    pub fn new(id: impl Into<ElementId>, content: EmptyContent) -> Self {
        Self {
            id: id.into(),
            content,
            old: None,
            clock: Default::default(),
            height: Slide::new(0.),
            measured: Default::default(),
        }
    }
    pub fn set_content(&mut self, content: EmptyContent, cx: &mut Context<Self>) {
        if self.content == content {
            return;
        }
        self.old = Some(std::mem::replace(&mut self.content, content));
        self.height.reset(self.measured.get());
        self.clock.restart(cx);
        cx.notify()
    }
    fn copy(c: &EmptyContent, p: Paint) -> Div {
        div()
            .flex()
            .flex_col()
            .items_center()
            .w_full()
            .text_center()
            .child(
                div()
                    .text_size(px(16.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.ink)
                    .child(c.title.clone()),
            )
            .child(
                div()
                    .mt_2()
                    .text_size(px(14.))
                    .line_height(px(20.))
                    .max_w(px(288.))
                    .text_color(p.secondary)
                    .child(c.description.clone()),
            )
    }
}
impl Render for EmptyState {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let radius = surface_radius(cx);
        let age = self.clock.time(0.6, w, cx);
        let f = ease((age / 0.24).min(1.));
        let out = 1. - ease((age / 0.16).min(1.));
        let glyph_changed = self
            .old
            .as_ref()
            .is_some_and(|c| c.glyph != self.content.glyph);
        let mut badge = div()
            .relative()
            .size(px(48.))
            .rounded(radius)
            .flex()
            .items_center()
            .justify_center()
            .border_1()
            .border_color(p.line)
            .bg(p.stage);
        if let Some(prev) = &self.old
            && glyph_changed
            && out > 0.
        {
            badge = badge.child(
                div()
                    .absolute()
                    .inset_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .opacity(out)
                    .child(icon(prev.glyph, 24. * (0.6 + 0.4 * out), p.secondary)),
            )
        }
        let g = if glyph_changed { f } else { 1. };
        badge = badge.child(div().opacity(g).child(icon(
            self.content.glyph,
            24. * (0.6 + 0.4 * g),
            p.secondary,
        )));
        let mut copy = div().relative().mt_5().w_full().flex().flex_col();
        let saved = self.measured.clone();
        if age < 0.6 && saved.get() > 0. {
            self.height.duration(saved.get(), 0.3, cx);
            copy = copy.h(px(self.height.get(w, cx))).overflow_hidden()
        }
        if out > 0.
            && let Some(prev) = &self.old
        {
            copy = copy.child(
                div()
                    .absolute()
                    .w_full()
                    .top(px(-4. * (1. - out)))
                    .opacity(out)
                    .child(Self::copy(prev, p)),
            )
        }
        copy = copy.child(
            div()
                .flex_none()
                .w_full()
                .relative()
                .top(px(4. * (1. - f)))
                .opacity(f)
                .child(Self::copy(&self.content, p))
                .on_children_prepainted(move |b, w, _| {
                    if let Some(b) = b.first()
                        && saved.replace(f32::from(b.size.height)) != f32::from(b.size.height)
                    {
                        w.request_animation_frame()
                    }
                }),
        );
        let mut root = div()
            .id(self.id.clone())
            .role(Role::Region)
            .aria_label(self.content.title.clone())
            .w_full()
            .px_5()
            .flex()
            .flex_col()
            .items_center()
            .child(badge)
            .child(copy);
        if let Some(label) = self.content.action.clone() {
            root = root.child(activate(
                face(
                    "empty-action",
                    label.clone(),
                    36.,
                    radius.into(),
                    if self.content.secondary_action {
                        Tone::Surface
                    } else {
                        Tone::Primary
                    },
                    cx,
                )
                .when(self.content.secondary_action, |e| {
                    e.border_1().border_color(p.line)
                })
                .mt_5()
                .px(px(14.))
                .hover(|s| s.opacity(0.86))
                .child(label),
                cx,
                |_, _, cx| cx.emit(EmptyAction),
            ))
        }
        root
    }
}
