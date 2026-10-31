//! KUI button gallery — depends on the `kui` registry path (preview = install source).

use gpui::{div, prelude::*, px, Context, TestApp, Window};
use kui::components::button::Button;
use kui::foundation::{init, Theme};
use kui::primitives::Size;

struct ButtonGallery {
    clicks: u32,
}

impl ButtonGallery {
    fn new() -> Self {
        Self { clicks: 0 }
    }
}

impl Render for ButtonGallery {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::global(cx);
        let clicks = self.clicks;

        div()
            .size_full()
            .flex()
            .flex_col()
            .gap_4()
            .p_6()
            .bg(theme.tokens.colors.background)
            .text_color(theme.tokens.colors.foreground)
            .child(div().text_size(px(22.)).child("KUI"))
            .child(
                div()
                    .text_size(px(14.))
                    .text_color(theme.tokens.colors.text_secondary)
                    .child("Button MVP — same source the kui CLI installs"),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .flex_wrap()
                    .child(Button::new("primary").primary().label("Primary").into_themed(cx))
                    .child(
                        Button::new("secondary")
                            .secondary()
                            .label("Secondary")
                            .into_themed(cx),
                    )
                    .child(Button::new("ghost").ghost().label("Ghost").into_themed(cx))
                    .child(Button::new("danger").danger().label("Danger").into_themed(cx))
                    .child(
                        Button::new("sm")
                            .secondary()
                            .with_size(Size::Sm)
                            .label("Small")
                            .into_themed(cx),
                    )
                    .child(
                        Button::new("lg")
                            .primary()
                            .with_size(Size::Lg)
                            .label("Large")
                            .into_themed(cx),
                    )
                    .child(
                        Button::new("loading")
                            .primary()
                            .loading(true)
                            .label("Loading")
                            .into_themed(cx),
                    )
                    .child(
                        Button::new("disabled")
                            .secondary()
                            .disabled(true)
                            .label("Disabled")
                            .into_themed(cx),
                    )
                    .child(
                        Button::new("click")
                            .primary()
                            .label(format!("Clicked {clicks}"))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.clicks += 1;
                                cx.notify();
                            }))
                            .into_themed(cx),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        Button::new("theme")
                            .secondary()
                            .label("Toggle theme")
                            .on_click(|_, _, cx| {
                                Theme::global_mut(cx).toggle_mode();
                                cx.refresh_windows();
                            })
                            .into_themed(cx),
                    )
                    .child(
                        Button::new("motion")
                            .ghost()
                            .label(if theme.reduced_motion {
                                "Motion: off"
                            } else {
                                "Motion: on"
                            })
                            .on_click(|_, _, cx| {
                                let theme = Theme::global_mut(cx);
                                theme.reduced_motion = !theme.reduced_motion;
                                cx.refresh_windows();
                            })
                            .into_themed(cx),
                    ),
            )
    }
}

#[cfg(feature = "native")]
fn run_native() {
    use gpui::{size, App, Bounds, WindowBounds, WindowOptions};

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
        // Keep gallery type referenced for compile coverage under test-support.
        let _gallery = ButtonGallery::new();
        let _ = &_gallery.clicks;
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
        eprintln!("kui-story: use --features native for a window");
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
