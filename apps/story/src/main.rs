//! KUI button gallery host (desktop / smoke).
//!
//! - Default: headless smoke via `TestApp`
//! - `--features native`: desktop window through `gpui_platform`
//! - WASM web host lives in `apps/web` (same gallery source)

mod gallery;

use gallery::ButtonGallery;
use gpui::TestApp;
use kui::components::button::Button;
use kui::foundation::{init, Theme};

#[cfg(feature = "native")]
fn run_native() {
    use gpui::prelude::*;
    use gpui::{px, size, App, Bounds, WindowBounds, WindowOptions};

    gpui_platform::application().run(|cx: &mut App| {
        init(cx);
        let bounds = Bounds::centered(None, size(px(720.), px(420.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(|_| ButtonGallery::new()),
        )
        .expect("open KUI story window");
        cx.activate(true);
    });
}

fn run_smoke() {
    let mut cx = TestApp::new();
    cx.update(|cx| {
        init(cx);
        let _ = Theme::global(cx);
        let _ = Button::new("smoke").primary().label("KUI").into_themed(cx);
        let _gallery = ButtonGallery::new();
        let _ = &_gallery;
    });
    println!("kui-story smoke ok");
}

fn main() {
    #[cfg(feature = "native")]
    {
        run_native();
    }
    #[cfg(not(feature = "native"))]
    {
        eprintln!("kui-story: use --features native for a desktop window");
        eprintln!("kui-story: use apps/web for WASM (see docs/PLATFORMS.md)");
        run_smoke();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn button_variants_build_under_theme() {
        let mut cx = TestApp::new();
        cx.update(|cx| {
            init(cx);
            let _ = Button::new("p").primary().label("A").into_themed(cx);
            let _ = Button::new("s").secondary().label("B").into_themed(cx);
            let _ = Button::new("g").ghost().label("C").into_themed(cx);
            let _ = Button::new("d").danger().label("D").into_themed(cx);
        });
    }
}
