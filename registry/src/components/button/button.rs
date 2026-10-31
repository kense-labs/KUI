use std::rc::Rc;

use gpui::{
    div, App, ClickEvent, Div, ElementId, FontWeight, InteractiveElement, IntoElement,
    ParentElement, SharedString, Stateful, StatefulInteractiveElement, Styled, Window,
};

use crate::foundation::{Theme, MOTION};
use crate::primitives::Size;

use super::style::{chrome, disabled_chrome, ButtonChrome};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ButtonVariant {
    #[default]
    Primary,
    Secondary,
    Ghost,
    Danger,
}

/// Finished KUI button with Arc-aligned control rhythm and theme-aware chrome.
pub struct Button {
    id: ElementId,
    label: SharedString,
    variant: ButtonVariant,
    size: Size,
    disabled: bool,
    loading: bool,
    on_click: Option<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>>,
}

impl Button {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            label: SharedString::from(""),
            variant: ButtonVariant::Primary,
            size: Size::Md,
            disabled: false,
            loading: false,
            on_click: None,
        }
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = label.into();
        self
    }

    pub fn primary(mut self) -> Self {
        self.variant = ButtonVariant::Primary;
        self
    }

    pub fn secondary(mut self) -> Self {
        self.variant = ButtonVariant::Secondary;
        self
    }

    pub fn ghost(mut self) -> Self {
        self.variant = ButtonVariant::Ghost;
        self
    }

    pub fn danger(mut self) -> Self {
        self.variant = ButtonVariant::Danger;
        self
    }

    pub fn with_size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    pub fn on_click(
        mut self,
        listener: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(listener));
        self
    }

    /// Build with live KUI theme tokens from `App`.
    pub fn into_themed(self, cx: &App) -> Stateful<Div> {
        let theme = Theme::global(cx);
        let look = chrome(self.variant, self.size, &theme.tokens.colors);
        let _press = MOTION.effective(MOTION.snappy, theme.reduced_motion);
        paint(self, look, &theme.tokens.colors)
    }
}

impl IntoElement for Button {
    type Element = Stateful<Div>;

    fn into_element(self) -> Self::Element {
        let colors = crate::foundation::ColorTokens::light();
        let look = chrome(self.variant, self.size, &colors);
        paint(self, look, &colors)
    }
}

fn paint(
    button: Button,
    look: ButtonChrome,
    colors: &crate::foundation::ColorTokens,
) -> Stateful<Div> {
    let inert = button.disabled || button.loading;
    let look = if inert && button.variant != ButtonVariant::Primary {
        disabled_chrome(button.variant, colors, &look)
    } else {
        look
    };

    let label = if button.loading {
        SharedString::from("…")
    } else {
        button.label.clone()
    };

    let mut el = div()
        .id(button.id)
        .flex()
        .items_center()
        .justify_center()
        .px(look.pad_x)
        .min_h(look.min_h)
        .rounded(look.radius)
        .border_1()
        .border_color(look.border)
        .text_size(look.text)
        .font_weight(FontWeight::MEDIUM)
        .line_height(look.line_height)
        .bg(look.bg)
        .text_color(look.fg)
        .child(label);

    if inert {
        // Arc primary disabled fades; outlined variants keep shape via disabled_chrome.
        if button.variant == ButtonVariant::Primary {
            el = el.opacity(0.52);
        }
    } else if let Some(on_click) = button.on_click.clone() {
        el = el.on_click(move |event, window, cx| on_click(event, window, cx));
    }

    el
}
