//! Render KUI controls as native, accessible HTML in static documents.
//!
//! The GPUI renderer is used inside live component previews. Document controls
//! use the browser's keyboard activation and editing, including Safari IME.

use crate::components::button::{Button, ButtonVariant, chrome};
use crate::foundation::ColorTokens;
use crate::primitives::{editor::InputMode, field::FieldStyle};

/// Trusted, pre-escaped attributes and child markup from a document template.
pub struct Markup<'a>(pub &'a str);

/// Render a KUI Button using its configured size, variant and activation state.
pub fn button(control: &Button, attrs: Markup<'_>, children: Markup<'_>) -> String {
    let look = chrome(control.variant, control.size, &ColorTokens::light());
    let (height, radius) = (f32::from(look.min_h), f32::from(look.radius));
    let variant = match control.variant {
        ButtonVariant::Primary => "primary",
        ButtonVariant::Secondary => "secondary",
        ButtonVariant::Ghost => "ghost",
        ButtonVariant::Danger => "danger",
    };
    // aria-disabled preserves a loading button's tab stop. Activation is
    // suppressed by the document host while the button is busy.
    let state = if control.disabled {
        " disabled"
    } else if control.loading {
        " aria-busy=\"true\" aria-disabled=\"true\""
    } else {
        ""
    };
    format!(
        r#"<button type="button" data-kui-control="button" data-variant="{variant}" style="--kui-height:{height}px;--kui-radius:{radius}px" {}{state}>{}</button>"#,
        attrs.0, children.0
    )
}

/// Render a native editor with the same input mode as the GPUI field.
pub fn field<S: FieldStyle>(attrs: Markup<'_>) -> String {
    match S::MODE {
        InputMode::Multi | InputMode::InlineMulti => format!(
            r#"<textarea data-kui-control="field" {}></textarea>"#,
            attrs.0
        ),
        mode => {
            let kind = match mode {
                InputMode::Search => "search",
                InputMode::Password => "password",
                _ => "text",
            };
            format!(
                r#"<input data-kui-control="field" type="{kind}" {}>"#,
                attrs.0
            )
        }
    }
}
