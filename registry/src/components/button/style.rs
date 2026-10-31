//! KUI Button presentation helpers.

use gpui::{px, Hsla, Pixels};

use crate::foundation::ColorTokens;
use crate::primitives::Size;

use super::ButtonVariant;

pub struct ButtonChrome {
    pub pad_x: Pixels,
    pub pad_y: Pixels,
    pub text: Pixels,
    pub radius: Pixels,
    pub bg: Hsla,
    pub fg: Hsla,
    pub border: Option<Hsla>,
}

pub fn chrome(variant: ButtonVariant, size: Size, colors: &ColorTokens) -> ButtonChrome {
    let (pad_x, pad_y, text) = match size {
        Size::Sm => (px(10.), px(4.), px(12.)),
        Size::Md => (px(14.), px(6.), px(14.)),
        Size::Lg => (px(18.), px(8.), px(16.)),
    };
    let (bg, fg, border) = match variant {
        ButtonVariant::Primary => (colors.accent, colors.accent_foreground, None),
        ButtonVariant::Secondary => (colors.surface_muted, colors.foreground, Some(colors.border)),
        ButtonVariant::Ghost => (colors.background.opacity(0.), colors.foreground, None),
        ButtonVariant::Danger => (colors.danger, colors.danger_foreground, None),
    };
    ButtonChrome {
        pad_x,
        pad_y,
        text,
        radius: px(8.),
        bg,
        fg,
        border,
    }
}
