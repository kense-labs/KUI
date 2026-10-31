use gpui::{Hsla, Pixels, px, rgb};

/// Semantic color tokens. KUI components must read these instead of hard-coded RGB
/// when an `App` theme is available. Default chrome is pure black/white (DESIGN.md).
#[derive(Clone, Debug)]
pub struct ColorTokens {
    pub background: Hsla,
    pub surface: Hsla,
    pub surface_muted: Hsla,
    pub foreground: Hsla,
    pub text_secondary: Hsla,
    pub text_muted: Hsla,
    pub border: Hsla,
    pub border_strong: Hsla,
    pub accent: Hsla,
    pub accent_foreground: Hsla,
    pub accent_subtle: Hsla,
    pub danger: Hsla,
    pub danger_foreground: Hsla,
    pub success: Hsla,
    pub warning: Hsla,
    pub focus_outline: Hsla,
}

impl ColorTokens {
    pub fn light() -> Self {
        Self {
            background: rgb(0xffffff).into(),
            surface: rgb(0xffffff).into(),
            surface_muted: rgb(0xf5f5f5).into(),
            foreground: rgb(0x000000).into(),
            text_secondary: rgb(0x525252).into(),
            text_muted: rgb(0x737373).into(),
            border: rgb(0xe5e5e5).into(),
            border_strong: rgb(0xa3a3a3).into(),
            accent: rgb(0x000000).into(),
            accent_foreground: rgb(0xffffff).into(),
            accent_subtle: rgb(0xf5f5f5).into(),
            danger: rgb(0xb42318).into(),
            danger_foreground: rgb(0xffffff).into(),
            success: rgb(0x16a34a).into(),
            warning: rgb(0xd97706).into(),
            focus_outline: rgb(0x000000).into(),
        }
    }

    pub fn dark() -> Self {
        Self {
            background: rgb(0x000000).into(),
            surface: rgb(0x000000).into(),
            surface_muted: rgb(0x1a1a1a).into(),
            foreground: rgb(0xffffff).into(),
            text_secondary: rgb(0xa3a3a3).into(),
            text_muted: rgb(0x737373).into(),
            border: rgb(0x2a2a2a).into(),
            border_strong: rgb(0x525252).into(),
            accent: rgb(0xffffff).into(),
            accent_foreground: rgb(0x000000).into(),
            accent_subtle: rgb(0x1a1a1a).into(),
            danger: rgb(0xf97066).into(),
            danger_foreground: rgb(0x000000).into(),
            success: rgb(0x4ade80).into(),
            warning: rgb(0xfbbf24).into(),
            focus_outline: rgb(0xffffff).into(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct RadiusTokens {
    pub sm: Pixels,
    pub md: Pixels,
    pub lg: Pixels,
    pub panel: Pixels,
    pub full: Pixels,
}

impl Default for RadiusTokens {
    fn default() -> Self {
        Self {
            sm: px(4.),
            // Control radius — modest 8px (docs canvas/code use lg/panel).
            md: px(8.),
            lg: px(18.),
            panel: px(26.),
            full: px(9999.),
        }
    }
}

#[derive(Clone, Debug)]
pub struct SpaceTokens {
    pub one: Pixels,
    pub two: Pixels,
    pub three: Pixels,
    pub four: Pixels,
    pub six: Pixels,
    pub eight: Pixels,
}

impl Default for SpaceTokens {
    fn default() -> Self {
        Self {
            one: px(4.),
            two: px(8.),
            three: px(12.),
            four: px(16.),
            six: px(24.),
            eight: px(32.),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Tokens {
    pub colors: ColorTokens,
    pub radius: RadiusTokens,
    pub space: SpaceTokens,
}

impl Tokens {
    pub fn for_mode(mode: super::ThemeMode) -> Self {
        Self {
            colors: match mode {
                super::ThemeMode::Light => ColorTokens::light(),
                super::ThemeMode::Dark => ColorTokens::dark(),
            },
            radius: RadiusTokens::default(),
            space: SpaceTokens::default(),
        }
    }
}
