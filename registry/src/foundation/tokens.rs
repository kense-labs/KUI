use gpui::{px, rgb, Hsla, Pixels};

/// Semantic color tokens. KUI components must read these instead of hard-coded RGB
/// when an `App` theme is available.
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
            background: rgb(0xfafafa).into(),
            surface: rgb(0xffffff).into(),
            surface_muted: rgb(0xf4f4f5).into(),
            foreground: rgb(0x18181b).into(),
            text_secondary: rgb(0x52525b).into(),
            text_muted: rgb(0x71717a).into(),
            border: rgb(0xe4e4e7).into(),
            border_strong: rgb(0xa1a1aa).into(),
            accent: rgb(0x18181b).into(),
            accent_foreground: rgb(0xfafafa).into(),
            accent_subtle: rgb(0xe4e4e7).into(),
            danger: rgb(0xdc2626).into(),
            danger_foreground: rgb(0xffffff).into(),
            success: rgb(0x16a34a).into(),
            warning: rgb(0xd97706).into(),
            focus_outline: rgb(0x2563eb).into(),
        }
    }

    pub fn dark() -> Self {
        Self {
            background: rgb(0x09090b).into(),
            surface: rgb(0x18181b).into(),
            surface_muted: rgb(0x27272a).into(),
            foreground: rgb(0xfafafa).into(),
            text_secondary: rgb(0xa1a1aa).into(),
            text_muted: rgb(0x71717a).into(),
            border: rgb(0x27272a).into(),
            border_strong: rgb(0x52525b).into(),
            accent: rgb(0xfafafa).into(),
            accent_foreground: rgb(0x18181b).into(),
            accent_subtle: rgb(0x27272a).into(),
            danger: rgb(0xf87171).into(),
            danger_foreground: rgb(0x18181b).into(),
            success: rgb(0x4ade80).into(),
            warning: rgb(0xfbbf24).into(),
            focus_outline: rgb(0x60a5fa).into(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct RadiusTokens {
    pub sm: Pixels,
    pub md: Pixels,
    pub lg: Pixels,
    pub full: Pixels,
}

impl Default for RadiusTokens {
    fn default() -> Self {
        Self {
            sm: px(4.),
            md: px(8.),
            lg: px(12.),
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
