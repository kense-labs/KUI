//! Shared control size tiers (`Sm`, `Md`, `Lg`) across KUI components.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Size {
    Sm,
    #[default]
    Md,
    Lg,
}
