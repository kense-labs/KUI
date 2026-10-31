//! Commands anchored at a pointer or beside a keyboard-focused target.
use crate::{
    foundation::Theme,
    primitives::{
        family::{Tone, face},
        menu::{MenuControl, MenuCore, MenuItem, MenuSelection, render_menu},
    },
};
use gpui::{prelude::*, *};
type Target = Box<dyn Fn(&mut Window, &mut App) -> AnyElement>;
pub struct ContextMenu {
    core: MenuCore,
    target: Target,
}
impl ContextMenu {
    pub fn new(
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        items: Vec<MenuItem>,
        target: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
        cx: &App,
    ) -> Self {
        Self {
            core: MenuCore::new(id.into(), label.into(), items, cx),
            target: Box::new(target),
        }
    }
    pub fn is_open(&self) -> bool {
        self.core.open
    }
}
impl EventEmitter<MenuSelection> for ContextMenu {}
impl MenuControl for ContextMenu {
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
impl Render for ContextMenu {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let target = (self.target)(w, cx);
        let btn = face(
            "target",
            self.core.name.clone(),
            44.,
            Theme::global(cx).tokens.radius.md.into(),
            Tone::Clear,
            cx,
        )
        .h_auto()
        .child(target);
        render_menu(self, btn, true, 208., w, cx)
    }
}
