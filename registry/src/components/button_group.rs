//! A segmented group with shared highlighting.
use crate::primitives::strip::{Activated, Layout, Segment, Strip, StripControl, draw};
use gpui::{prelude::*, *};
pub struct ButtonGroup {
    strip: Strip,
}
impl StripControl for ButtonGroup {
    fn strip(&mut self) -> &mut Strip {
        &mut self.strip
    }
}
impl EventEmitter<Activated> for ButtonGroup {}
impl ButtonGroup {
    pub fn new(id: impl Into<ElementId>, items: Vec<Segment>, cx: &App) -> Self {
        Self {
            strip: Strip::new(id.into(), items, Layout::Attached, cx),
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
impl Render for ButtonGroup {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        draw(self, w, cx)
    }
}
