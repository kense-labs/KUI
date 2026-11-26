//! A primary action and a keyboard-accessible alternatives menu.
use crate::primitives::strip::{Activated, Layout, Segment, Strip, StripControl, draw};
use gpui::{prelude::*, *};
/// `SplitButton` component.
///
/// # Examples
///
/// ```no_run
/// # #[allow(unused_variables, unused_imports)]
/// # fn example(window: &mut gpui::Window, cx: &mut gpui::App) {
/// # use gpui::prelude::*;
/// use gpui::prelude::*;
/// use kui::components::split_button::SplitButton;
/// use kui::primitives::glyph::Glyph;
/// use kui::primitives::strip::{Segment, Choice};
///
/// let control = cx.new(|cx| SplitButton::new("publish-control", vec![
///     Segment::new("publish", "Publish", Some(Glyph::Send), 103.),
///     Segment::new("options", "More actions", Some(Glyph::Down), 38.).icon_only()
///         .with_menu(vec![Choice::new("draft", "Save as draft")]),
/// ], cx));
/// # }
/// ```
pub struct SplitButton {
    strip: Strip,
}
impl StripControl for SplitButton {
    fn strip(&mut self) -> &mut Strip {
        &mut self.strip
    }
}
impl EventEmitter<Activated> for SplitButton {}
impl SplitButton {
    pub fn new(id: impl Into<ElementId>, items: Vec<Segment>, cx: &App) -> Self {
        Self {
            strip: Strip::new(id.into(), items, Layout::Split, cx),
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
impl Render for SplitButton {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        draw(self, w, cx)
    }
}
