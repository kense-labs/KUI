//! An anchored, non-modal contextual panel.
use crate::primitives::disclosure::{Surface, SurfaceStyle};
/// `Popover` component.
///
/// # Examples
///
/// ```no_run
/// # #[allow(unused_variables, unused_imports)]
/// # fn example(window: &mut gpui::Window, cx: &mut gpui::App) {
/// # use gpui::prelude::*;
/// use gpui::prelude::*;
/// use kui::components::popover::Popover;
///
/// let control = cx.new(|cx| Popover::new("checks", "Why this status?", "Ready for review", cx)
///         .with_description("All required checks passed.").with_cancel("Got it"));
/// # }
/// ```
pub struct PopoverStyle;
impl SurfaceStyle for PopoverStyle {
    const MODAL: bool = false;
}
pub type Popover = Surface<PopoverStyle>;
