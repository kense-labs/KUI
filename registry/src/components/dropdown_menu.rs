//! A labelled trigger for a short list of related commands.
use crate::{
    foundation::Theme,
    primitives::{
        family::{Paint, PhaseClock, Slide, Tone, face, lettering},
        glyph::{Glyph, icon, transformed},
        menu::{MenuAlign, MenuControl, MenuCore, MenuItem, MenuSelection, render_menu},
    },
};
use gpui::{prelude::*, *};
pub struct DropdownMenu {
    core: MenuCore,
    label: SharedString,
    icon: Option<Glyph>,
    icon_only: bool,
    width: Option<Slide>,
    previous: SharedString,
    changed: PhaseClock,
    turn: Slide,
}
impl EventEmitter<MenuSelection> for DropdownMenu {}
impl DropdownMenu {
    pub fn new(
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        items: Vec<MenuItem>,
        cx: &App,
    ) -> Self {
        let label = label.into();
        Self {
            core: MenuCore::new(id.into(), label.clone(), items, cx),
            previous: label.clone(),
            changed: Default::default(),
            label,
            icon: None,
            icon_only: false,
            width: None,
            turn: Slide::snappy(0.),
        }
    }
    pub fn with_align(mut self, align: MenuAlign) -> Self {
        self.core.align = align;
        self
    }
    pub fn with_icon(mut self, g: Glyph) -> Self {
        self.icon = Some(g);
        self
    }
    pub fn icon_only(mut self) -> Self {
        self.icon_only = true;
        self
    }
    pub fn set_label(&mut self, label: impl Into<SharedString>, cx: &mut Context<Self>) {
        let next = label.into();
        if self.label == next {
            return;
        }
        self.previous = std::mem::replace(&mut self.label, next);
        self.changed.restart(cx);
        self.core.name = self.label.clone();
        cx.notify()
    }
    pub fn is_open(&self) -> bool {
        self.core.open
    }
    pub fn focus(&self, w: &mut Window, cx: &mut App) {
        self.core.trigger.focus(w, cx)
    }
}
impl MenuControl for DropdownMenu {
    fn core(&mut self) -> &mut MenuCore {
        &mut self.core
    }
    fn selected(&mut self, i: usize, w: &mut Window, cx: &mut Context<Self>) {
        let id = self.core.items[i].id.clone();
        self.core.close(w, cx);
        cx.emit(MenuSelection(id));
        cx.notify()
    }
}
impl Render for DropdownMenu {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let r = Theme::global(cx).tokens.radius.md;
        let target = if self.icon_only {
            32.
        } else {
            let mut style = w.text_style();
            style.font_weight = FontWeight::MEDIUM;
            let advance: f32 = w
                .text_system()
                .shape_line(
                    self.label.clone(),
                    px(14.),
                    &[style.to_run(self.label.len())],
                    None,
                )
                .width
                .into();
            advance.ceil() + 52. + if self.icon.is_some() { 22. } else { 0. }
        }
        .min(f32::from(w.viewport_size().width) - 16.);
        let sw = self.width.get_or_insert_with(|| Slide::new(target));
        sw.set(target, cx);
        let width = sw.get(w, cx);
        self.turn.set(if self.core.open { 1. } else { 0. }, cx);
        let btn = face(
            "trigger",
            self.label.clone(),
            if self.icon_only { 32. } else { 36. },
            r.into(),
            Tone::Surface,
            cx,
        )
        .w(px(width))
        .gap(px(6.))
        .when(!self.icon_only, |e| e.border_1().border_color(p.line))
        .hover(move |s| s.bg(p.hover))
        .when_some(self.icon, |e, g| e.child(icon(g, 15., p.secondary)))
        .when(!self.icon_only, |e| {
            let text_width = (width - 51. - if self.icon.is_some() { 22. } else { 0. }).max(1.);
            let age = self.changed.time(0.48, w, cx);
            e.child(
                div()
                    .w(px(text_width))
                    .overflow_hidden()
                    .child(if age < 0.48 {
                        lettering(
                            self.previous.clone(),
                            self.label.clone(),
                            age,
                            false,
                            14.,
                            p.ink,
                            text_width,
                        )
                        .into_any_element()
                    } else {
                        div()
                            .truncate()
                            .child(self.label.clone())
                            .into_any_element()
                    }),
            )
            .child(transformed(
                Glyph::Down,
                13.,
                p.secondary,
                self.turn.get(w, cx) * std::f32::consts::PI,
                1.,
                (0., 0.),
                1.,
            ))
        });
        render_menu(self, btn, false, 192., w, cx)
    }
}
