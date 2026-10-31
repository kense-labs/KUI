//! Window-relative placement for transient surfaces.
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};

#[derive(Default, Clone)]
pub(crate) struct TriggerBounds(Rc<Cell<Bounds<Pixels>>>);
impl TriggerBounds {
    pub fn measure(&self) -> impl IntoElement {
        let saved = self.0.clone();
        canvas(move |b, _, _| saved.set(b), |_, _, _, _| ())
            .absolute()
            .inset_0()
    }
    pub fn get(&self) -> Bounds<Pixels> {
        self.0.get()
    }
    /// Prefer below the trigger. Flip above when it leaves more room.
    pub fn place(
        &self,
        view: Size<Pixels>,
        height: Pixels,
        gap: Pixels,
    ) -> (Point<Pixels>, Anchor) {
        let b = self.get();
        if b.bottom() + gap + height > view.height - px(10.) && b.top() > view.height - b.bottom() {
            (point(b.left(), b.top() - gap), Anchor::BottomLeft)
        } else {
            (point(b.left(), b.bottom() + gap), Anchor::TopLeft)
        }
    }
}
