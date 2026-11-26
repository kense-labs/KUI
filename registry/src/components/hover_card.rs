//! Hoverable and touch-accessible preview of an identity or link.
use crate::primitives::nav::{Content, Hint};
use gpui::*;
/// `PreviewCard` component.
///
/// # Examples
///
/// ```no_run
/// # #[allow(unused_variables, unused_imports)]
/// # fn example(window: &mut gpui::Window, cx: &mut gpui::App) {
/// # use gpui::prelude::*;
/// use gpui::prelude::*;
/// use kui::components::hover_card::PreviewCard;
///
/// let control = cx.new(|cx| PreviewCard::new("Jasmine Brooks","JB",Box::new(|_, _| gpui::div().child("Product designer").into_any_element()),cx));
/// # }
/// ```
pub struct PreviewCard(Entity<Hint>);
impl PreviewCard {
    pub fn new(
        name: impl Into<SharedString>,
        badge: impl Into<SharedString>,
        body: Content,
        cx: &mut App,
    ) -> Self {
        Self(cx.new(|cx| {
            let mut hint = Hint::new(name, "", cx);
            hint.badge = Some(badge.into());
            hint.body = Some(body);
            hint
        }))
    }
}
impl Render for PreviewCard {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.0.clone()
    }
}
