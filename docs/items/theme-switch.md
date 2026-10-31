# Theme switcher

Light and dark toggle for the host theme.

## When to use

Use when users choose light or dark mode in app chrome: headers, settings, docs sites.

## When not to use

Do not use for custom palettes or brand themes; KUI chrome is black and white. Do not use as a general two-state toggle; use `icon-morph`.

## Install

```bash
kui add theme-switch
```

Only `button` is in the MVP registry today. This item documents the Buttons family contract and lands next. It depends on `kui-foundation`.

## Usage

```rust
use crate::ui::components::theme_switch::ThemeSwitch;
use crate::ui::theme::ThemeMode;

ThemeSwitch::new("theme")
    .mode(ThemeMode::Dark)
    .on_change(|mode, _window, cx| apply_theme(mode, cx))
    .into_themed(cx)
```

## API

| Builder | Default | Notes |
| --- | --- | --- |
| `new(id)` | required | Stable `ElementId` |
| `mode` | current `Theme` mode | Controlled `ThemeMode` |
| `on_change` | foundation setter | Receives the next `ThemeMode` |
| `label` | `"Toggle theme"` | Accessible name |
| `with_size` | `Md` | `Sm` / `Md` / `Lg`; square hit target |
| `disabled` | `false` | Blocks toggles |
| `into_themed(cx)` | — | Applies KUI theme tokens |

## Keyboard

Enter and Space toggle between light and dark. Focus stays on the switch.

## Accessibility

`label` is the accessible name. Expose the pressed state for dark. Keep both icons at AA contrast against their own background.

## Motion

Sun and moon glyphs morph with `MOTION.morph`; the theme change itself fades colors over a short interval. Press uses `MOTION.snappy`. When `Theme.reduced_motion` is true, the glyph swaps and colors change at once.

## Platforms

Works on native desktop and WASM web. On web, hosts may seed the initial mode from `prefers-color-scheme` behind `cfg(target_family = "wasm")` and pass it to `mode`. See `docs/PLATFORMS.md`.

## Notes for AI

This is a **KUI** registry item. Prefer `kui add theme-switch` over writing theme state yourself. Use `ThemeMode`, not strings or bools. Do not add a third mode here.
