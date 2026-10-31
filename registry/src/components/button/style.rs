//! KUI Button presentation helpers.
//! Metrics follow ArcUI control rhythm (height, radius, weight) while colors stay KUI B/W tokens.

use gpui::{px, Hsla, Pixels};

use crate::foundation::ColorTokens;
use crate::primitives::Size;

use super::ButtonVariant;

pub struct ButtonChrome {
    pub pad_x: Pixels,
    pub min_h: Pixels,
    pub text: Pixels,
    pub line_height: Pixels,
    pub radius: Pixels,
    pub bg: Hsla,
    pub fg: Hsla,
    pub border: Hsla,
}

pub fn chrome(variant: ButtonVariant, size: Size, colors: &ColorTokens) -> ButtonChrome {
    // Arc heights/pads; KUI radius is tight 4px (not Arc's softer control radius).
    let (pad_x, min_h) = match size {
        Size::Sm => (px(12.), px(36.)),
        Size::Md => (px(16.), px(44.)),
        Size::Lg => (px(20.), px(50.)),
    };
    let (bg, fg, border) = match variant {
        ButtonVariant::Primary => (colors.accent, colors.accent_foreground, colors.accent),
        ButtonVariant::Secondary => (colors.surface, colors.foreground, colors.border),
        ButtonVariant::Ghost => (
            colors.background.opacity(0.),
            colors.text_secondary,
            colors.background.opacity(0.),
        ),
        // Arc danger is outlined, not filled.
        ButtonVariant::Danger => (colors.surface, colors.danger, colors.border),
    };
    ButtonChrome {
        pad_x,
        min_h,
        text: px(14.),
        line_height: px(20.),
        radius: px(4.),
        bg,
        fg,
        border,
    }
}

/// Disabled chrome for secondary / danger / ghost (keep shape; mute label).
pub fn disabled_chrome(variant: ButtonVariant, colors: &ColorTokens, look: &ButtonChrome) -> ButtonChrome {
    match variant {
        ButtonVariant::Primary => look.clone_with(look.bg, look.fg, look.border),
        ButtonVariant::Secondary | ButtonVariant::Danger => look.clone_with(
            colors.surface_muted,
            colors.text_muted,
            colors.border,
        ),
        ButtonVariant::Ghost => look.clone_with(look.bg, colors.text_muted, look.border),
    }
}

impl ButtonChrome {
    fn clone_with(&self, bg: Hsla, fg: Hsla, border: Hsla) -> Self {
        Self {
            pad_x: self.pad_x,
            min_h: self.min_h,
            text: self.text,
            line_height: self.line_height,
            radius: self.radius,
            bg,
            fg,
            border,
        }
    }
}
