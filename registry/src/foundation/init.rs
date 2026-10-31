use gpui::{App, UpdateGlobal};

use super::theme::Theme;

/// Install KUI foundation globals. Call once before opening windows.
pub fn init(cx: &mut App) {
    if cx.has_global::<Theme>() {
        return;
    }
    Theme::set_global(cx, Theme::default());
}
