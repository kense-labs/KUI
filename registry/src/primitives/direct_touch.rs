//! Direct contact input for controls that cannot wait for a synthesized tap.
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
#[derive(Default, Clone)]
pub(crate) struct Contact(Rc<Cell<bool>>);
impl Contact {
    /// Mount over the gesture target. Only the topmost, hovered hitbox may claim a new contact.
    pub(crate) fn listen<T: 'static>(
        &self,
        cx: &Context<T>,
        handle: impl Fn(&mut T, &TouchDragEvent, &mut Window, &mut Context<T>) + 'static,
    ) -> impl IntoElement {
        let listener = cx.listener(handle);
        let active = self.0.clone();
        canvas(
            |bounds, w, _| w.insert_hitbox(bounds, HitboxBehavior::Normal),
            move |_, hit, w, _| {
                w.on_mouse_event(move |e: &TouchDragEvent, phase, w, cx| {
                    if phase.bubble() {
                        if e.phase == TouchPhase::Started {
                            active.set(hit.is_hovered(w));
                        }
                        if !active.get() {
                            return;
                        }
                        if matches!(e.phase, TouchPhase::Ended | TouchPhase::Cancelled) {
                            active.set(false)
                        }
                        if e.phase == TouchPhase::Started {
                            w.prevent_default();
                        }
                        listener(e, w, cx);
                        cx.stop_propagation();
                    }
                })
            },
        )
        .absolute()
        .inset_0()
    }
}
