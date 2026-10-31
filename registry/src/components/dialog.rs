//! A modal decision surface with focus restoration.
use crate::primitives::disclosure::{Surface, SurfaceStyle};
pub struct DialogStyle;
impl SurfaceStyle for DialogStyle {
    const MODAL: bool = true;
}
pub type Dialog = Surface<DialogStyle>;
