//! Select a story in the desktop gallery or embed one named example on the web.
use crate::{ButtonGallery, FAMILY_IDS, FamilyStory};
use gpui::{Context, Entity, Window, div, prelude::*};
use kui::{foundation::Theme, primitives::control::action};

pub struct StoryHost {
    gallery: Entity<ButtonGallery>,
    selected: Option<gpui::AnyView>,
    embedded: bool,
}
impl StoryHost {
    fn select(id: &str, w: &mut Window, cx: &mut Context<Self>) -> Option<gpui::AnyView> {
        if FAMILY_IDS.contains(&id.split(':').next().unwrap_or(id))
            || crate::COLLECTION_IDS.contains(&id)
            || crate::FIELD_IDS.contains(&id)
            || crate::WORKSPACE_IDS.contains(&id)
        {
            Some(cx.new(|cx| FamilyStory::new(id, w, cx)).into())
        } else {
            None
        }
    }
    /// Replace the mounted example while keeping the window and font system alive.
    pub fn show_story(&mut self, id: &str, w: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(view) = Self::select(id, w, cx) else {
            return false;
        };
        self.selected = Some(view);
        cx.notify();
        true
    }

    pub fn new(id: Option<&str>, w: &mut Window, cx: &mut Context<Self>) -> Self {
        let selected = id.and_then(|id| Self::select(id, w, cx));
        Self {
            gallery: cx.new(|_| ButtonGallery::new()),
            embedded: selected.is_some(),
            selected,
        }
    }
}
impl Render for StoryHost {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let c = Theme::global(cx).tokens.colors.clone();
        if let Some(view) = self.selected.clone() {
            return div()
                .size_full()
                .flex()
                .flex_col()
                .bg(c.surface)
                .text_color(c.foreground)
                .when(!self.embedded, |e| {
                    e.child(action("back", "All components", cx, |v, _, cx| {
                        v.selected = None;
                        cx.notify();
                    }))
                })
                .child(div().flex_1().min_h_0().child(view))
                .into_any_element();
        }
        let mut row = div().flex().flex_wrap().gap_2().p_4();
        for id in FAMILY_IDS
            .into_iter()
            .chain(crate::COLLECTION_IDS)
            .chain(crate::FIELD_IDS)
            .chain(crate::WORKSPACE_IDS)
            .chain([
                "theme-switch:eclipse",
                "theme-switch:split",
                "theme-switch:rise",
            ])
        {
            row = row.child(action(id, id, cx, move |v, w, cx| {
                v.selected = Self::select(id, w, cx);
                cx.notify();
            }));
        }
        div()
            .id("gallery")
            .size_full()
            .overflow_y_scroll()
            .bg(c.surface)
            .text_color(c.foreground)
            .child(row)
            .child(self.gallery.clone())
            .into_any_element()
    }
}
