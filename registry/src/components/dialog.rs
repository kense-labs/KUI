//! A modal decision surface with focus restoration.
use crate::primitives::disclosure::{Surface, SurfaceStyle};
/// `Dialog` component.
///
/// # Examples
///
/// ```no_run
/// # #[allow(unused_variables, unused_imports)]
/// # fn example(window: &mut gpui::Window, cx: &mut gpui::App) {
/// # use gpui::prelude::*;
/// use gpui::prelude::*;
/// use kui::components::dialog::Dialog;
///
/// let control = cx.new(|cx| Dialog::new("review", "Review changes", "Publish workspace", cx)
///         .with_description("Your team will see these changes immediately.")
///         .with_cancel("Keep editing").with_confirm("Publish now"));
/// # }
/// ```
pub struct DialogStyle;
impl SurfaceStyle for DialogStyle {
    const MODAL: bool = true;
}
pub type Dialog = Surface<DialogStyle>;
