//! Gallery library shared with `kui-web`.

mod base_buttons;
pub mod gallery;

pub use gallery::ButtonGallery;

mod host;
/// Focus and size integration for an embedded GPUI story.
pub struct StoryBridge {
    pub frame: fn(f32, kui::foundation::ThemeMode),
    pub leave: fn(bool),
    pub theme_target: fn(gpui::Bounds<gpui::Pixels>, gpui::Pixels),
}
impl gpui::Global for StoryBridge {}
pub use host::StoryHost;

mod family_story;
pub use family_story::{FAMILY_IDS, FamilyStory, ThemeBridge};

pub use family_story::collections::{AccountHost, IDS as COLLECTION_IDS};
