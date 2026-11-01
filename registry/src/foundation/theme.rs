use gpui::{App, Global, Hsla, Rgba};

use super::tokens::{ColorTokens, Tokens};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ThemeMode {
    #[default]
    Light,
    Dark,
}

impl ThemeMode {
    pub fn toggle(self) -> Self {
        match self {
            Self::Light => Self::Dark,
            Self::Dark => Self::Light,
        }
    }
}

/// Accent tints over light and dark controls. Transparent tints preserve the authored palette.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AccentPalette {
    pub light: Hsla,
    pub dark: Hsla,
}

impl AccentPalette {
    pub fn for_mode(self, mode: ThemeMode) -> Hsla {
        match mode {
            ThemeMode::Light => self.light,
            ThemeMode::Dark => self.dark,
        }
    }

    /// A single user-selected color, as opposed to a pair authored for the two themes.
    pub(crate) fn is_shared_color(self) -> bool {
        self.light == self.dark && !self.light.is_transparent()
    }

    /// Preserve authored ink for a theme pair; pick black or white for a shared fill.
    pub(crate) fn on_fill(self, fill: Hsla, authored: Hsla) -> Hsla {
        if self.is_shared_color() || Self::contrast(fill, authored) < 4.5 {
            Self::most_legible_ink(fill)
        } else {
            authored
        }
    }

    fn luminance(color: Hsla) -> f32 {
        let rgb = Rgba::from(color);
        let linear = |c: f32| {
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        linear(rgb.r) * 0.2126 + linear(rgb.g) * 0.7152 + linear(rgb.b) * 0.0722
    }

    pub(crate) fn contrast(a: Hsla, b: Hsla) -> f32 {
        let (x, y) = (Self::luminance(a), Self::luminance(b));
        (x.max(y) + 0.05) / (x.min(y) + 0.05)
    }

    fn most_legible_ink(fill: Hsla) -> Hsla {
        let luminance = Self::luminance(fill);
        if (luminance + 0.05) / 0.05 >= 1.05 / (luminance + 0.05) {
            Hsla::black()
        } else {
            Hsla::white()
        }
    }

    /// Keep the hue of a custom focus ring or selected label while making it visible on its surface.
    pub(crate) fn visible_ink(self, preferred: Hsla, surface: Hsla, minimum: f32) -> Hsla {
        if !self.is_shared_color() || Self::contrast(preferred, surface) >= minimum {
            return preferred;
        }
        let toward = Self::most_legible_ink(surface);
        let (mut low, mut high) = (0., 1.);
        for _ in 0..9 {
            let mid = (low + high) * 0.5;
            if Self::contrast(preferred.blend(toward.opacity(mid)), surface) >= minimum {
                high = mid;
            } else {
                low = mid;
            }
        }
        preferred.blend(toward.opacity(high))
    }

    /// Tint the hover surface away from its text when tinting toward it would erase contrast.
    pub(crate) fn hover_tint(self, fill: Hsla, ink: Hsla, amount: f32) -> Hsla {
        if self.is_shared_color() && Self::contrast(fill.blend(ink.opacity(amount)), ink) < 4.5 {
            Self::most_legible_ink(ink)
        } else {
            ink
        }
    }
}

/// Application-wide KUI theme. Installed by [`super::init`].
#[derive(Clone, Debug)]
pub struct Theme {
    pub mode: ThemeMode,
    pub tokens: Tokens,
    pub reduced_motion: bool,
    pub accent: AccentPalette,
}

impl Default for Theme {
    fn default() -> Self {
        let mode = ThemeMode::Light;
        Self {
            mode,
            tokens: Tokens::for_mode(mode),
            reduced_motion: false,
            accent: AccentPalette::default(),
        }
    }
}

impl Global for Theme {}

impl Theme {
    pub fn global(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    pub fn global_mut(cx: &mut App) -> &mut Self {
        cx.global_mut::<Self>()
    }

    pub fn set_mode(&mut self, mode: ThemeMode) {
        self.mode = mode;
        self.tokens = Tokens::for_mode(mode);
        self.set_accent(self.accent);
    }

    /// Replace the accent while preserving the rest of the host's tokens.
    pub fn set_accent(&mut self, palette: AccentPalette) {
        self.accent = palette;
        let tint = palette.for_mode(self.mode);
        let base = match self.mode {
            ThemeMode::Light => ColorTokens::light(),
            ThemeMode::Dark => ColorTokens::dark(),
        };
        let colors = &mut self.tokens.colors;
        colors.accent = base.accent.blend(tint);
        colors.accent_foreground = palette.on_fill(colors.accent, base.accent_foreground);
        colors.focus_outline =
            palette.visible_ink(base.focus_outline.blend(tint), colors.surface_muted, 3.5);
        colors.accent_subtle = base.accent_subtle.blend(tint.opacity(0.16));
    }

    pub fn toggle_mode(&mut self) {
        self.set_mode(self.mode.toggle());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{px, rgb};

    #[test]
    fn accent_follows_mode_and_resets_without_altering_other_roles() {
        let palette = AccentPalette {
            light: rgb(0x245bc4).into(),
            dark: rgb(0x8bb4ff).into(),
        };
        let mut theme = Theme::default();
        theme.tokens.radius.md = px(12.);
        theme.set_accent(palette);
        assert_eq!(theme.tokens.colors.accent, palette.light);
        assert_eq!(theme.tokens.radius.md, px(12.));
        theme.toggle_mode();
        let dark = ColorTokens::dark();
        assert_eq!(theme.tokens.colors.accent, palette.dark);
        assert_eq!(theme.tokens.colors.focus_outline, palette.dark);
        assert_eq!(theme.tokens.colors.danger, dark.danger);
        theme.set_accent(AccentPalette::default());
        assert_eq!(theme.tokens.colors.accent, dark.accent);
        assert_eq!(theme.tokens.colors.accent_subtle, dark.accent_subtle);
        assert_eq!(theme.tokens.colors.focus_outline, dark.focus_outline);
    }
}
