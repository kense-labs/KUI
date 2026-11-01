//! Website controls use the KUI document renderer; live stories use GPUI.
use kui::{
    components::{button::Button, input::InputStyle, search_field::SearchFieldStyle},
    html::{self, Markup},
};

pub fn button(attrs: &str, children: &str) -> String {
    html::button(
        &Button::new("document-action").secondary(),
        Markup(attrs),
        Markup(children),
    )
}
pub fn search(attrs: &str) -> String {
    html::field::<SearchFieldStyle>(Markup(attrs))
}
pub fn text(attrs: &str) -> String {
    html::field::<InputStyle>(Markup(attrs))
}
