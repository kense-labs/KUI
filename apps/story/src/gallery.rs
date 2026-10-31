//! Shared KUI Button gallery view used by native, web, and smoke hosts.

use gpui::{div, prelude::*, px, Context, Window};
use kui::components::button::Button;
use kui::foundation::Theme;
use kui::primitives::Size;

pub struct ButtonGallery {
    clicks: u32,
}

impl ButtonGallery {
    pub fn new() -> Self {
        Self { clicks: 0 }
    }
}

impl Default for ButtonGallery {
    fn default() -> Self {
        Self::new()
    }
}

impl gpui::Render for ButtonGallery {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
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
                    .child("Button MVP — native desktop and WASM web share this source"),
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
