//! An anchored, non-modal contextual panel.
use crate::primitives::disclosure::{Surface, SurfaceStyle};
pub struct PopoverStyle;
impl SurfaceStyle for PopoverStyle {
    const MODAL: bool = false;
}
pub type Popover = Surface<PopoverStyle>;
