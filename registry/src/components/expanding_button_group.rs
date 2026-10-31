//! An action strip that reveals the focused or hovered label.
use crate::primitives::strip::{Activated, Layout, Segment, Strip, StripControl, draw};
use gpui::{prelude::*, *};
pub struct ExpandingButtonGroup {
    strip: Strip,
}
impl StripControl for ExpandingButtonGroup {
    fn strip(&mut self) -> &mut Strip {
        &mut self.strip
    }
}
impl EventEmitter<Activated> for ExpandingButtonGroup {}
impl ExpandingButtonGroup {
    pub fn new(id: impl Into<ElementId>, items: Vec<Segment>, cx: &App) -> Self {
        Self {
            strip: Strip::new(id.into(), items, Layout::Expanding, cx),
        }
    }
    pub fn with_height(mut self, h: f32) -> Self {
        self.strip.height(h);
        self
    }
    pub fn primary(mut self) -> Self {
        self.strip.primary = true;
        self
    }

    pub fn update_label(
        &mut self,
        id: &str,
        label: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) {
        if let Some(item) = self.strip.items.iter_mut().find(|it| &*it.id == id) {
            item.label = label.into();
            cx.notify()
        }
    }
    pub fn set_disabled(&mut self, id: &str, disabled: bool, cx: &mut Context<Self>) {
        if let Some(item) = self.strip.items.iter_mut().find(|it| &*it.id == id) {
            item.disabled = disabled;
            cx.notify()
        }
    }
}
impl Render for ExpandingButtonGroup {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        draw(self, w, cx)
    }
}
