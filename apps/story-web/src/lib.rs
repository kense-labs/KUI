//! WASM entry for the KUI Button gallery.
//!
//! ```bash
//! ./scripts/build-story-web.sh
//! python3 -m http.server -d apps/story-web/www 8080
//! ```

#[cfg(target_family = "wasm")]
mod web {
    use gpui::prelude::*;
    use gpui::{px, size, App, Bounds, WindowBounds, WindowOptions};
    use kui::foundation::init;
    use kui_story::ButtonGallery;

    fn run_gallery() {
        gpui_platform::application().run(|cx: &mut App| {
            init(cx);
            let bounds = Bounds::centered(None, size(px(960.), px(540.)), cx);
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                |_, cx| cx.new(|_| ButtonGallery::new()),
            )
            .expect("open KUI web gallery");
            cx.activate(true);
        });
    }

    #[wasm_bindgen::prelude::wasm_bindgen(start)]
    pub fn start() {
        gpui_platform::web_init();
        run_gallery();
    }
}

#[cfg(not(target_family = "wasm"))]
/// Host-target placeholder. Build with `--target wasm32-unknown-unknown`.
pub fn wasm_only() {
    let _ = kui_story::ButtonGallery::new();
}
