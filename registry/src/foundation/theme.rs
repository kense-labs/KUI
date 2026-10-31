use gpui::{App, Global};

use super::tokens::Tokens;

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

/// Application-wide KUI theme. Installed by [`super::init`].
#[derive(Clone, Debug)]
pub struct Theme {
    pub mode: ThemeMode,
    pub tokens: Tokens,
    pub reduced_motion: bool,
}

impl Default for Theme {
    fn default() -> Self {
        let mode = ThemeMode::Light;
        Self {
            mode,
            tokens: Tokens::for_mode(mode),
            reduced_motion: false,
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
    }

    pub fn toggle_mode(&mut self) {
        self.set_mode(self.mode.toggle());
    }
}
