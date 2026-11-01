//! A compact description exposed by hover, keyboard focus or touch.
use crate::primitives::{glyph::Glyph, nav::Hint};
use gpui::*;
pub struct Tooltip(Entity<Hint>);
impl Tooltip {
    pub fn new(name: impl Into<SharedString>, text: impl Into<SharedString>, cx: &mut App) -> Self {
        Self(cx.new(|cx| {
            let mut hint = Hint::new(name, text, cx);
            hint.marker = Some(Glyph::HelpCircle);
            hint
        }))
    }
}
impl Render for Tooltip {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.0.clone()
    }
}
