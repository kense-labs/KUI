# Icon morph

Two-state icon button whose glyph morphs between on and off.

## When to use

Use for compact toggles with a well-known glyph pair: bookmark, mute, play and pause, menu and close.

## When not to use

Do not use when the state needs a text label; use `action-swap`. Do not use for one-way actions; use `button`.

## Install

```bash
kui add icon-morph
```

Only `button` is in the MVP registry today. This item documents the Buttons family contract and lands next.

## Usage

```rust
use crate::ui::components::icon_morph::IconMorph;

IconMorph::new("bookmark")
    .off(IconName::Bookmark)
    .on(IconName::BookmarkFilled)
    .label("Bookmark")
    .active(is_saved)
    .on_toggle(|next, _window, _cx| set_saved(next))
    .into_themed(cx)
```

## API

| Builder | Default | Notes |
| --- | --- | --- |
| `new(id)` | required | Stable `ElementId` |
| `off` / `on` | required | `IconName` for each state |
| `label` | required | Accessible name; shown as tooltip |
| `active` | `false` | Controlled on state |
| `ghost` / `secondary` | `ghost` | Variant |
| `with_size` | `Md` | `Sm` / `Md` / `Lg`; square hit target |
| `disabled` | `false` | Blocks toggles |
| `on_toggle` | none | Receives the next `bool` |
| `into_themed(cx)` | — | Applies KUI theme tokens |

## Keyboard

Enter and Space toggle. Focus stays on the button.

## Accessibility

`label` is the accessible name and does not change with state. Expose the pressed state so assistive tech reads on or off. The hit target stays at least 32px.

## Motion

Glyphs cross-morph with `MOTION.morph`: stroke fades and scale shift over a short interval. Press uses `MOTION.snappy`. When `Theme.reduced_motion` is true, the icon swaps with no morph.

## Platforms

Works on native desktop and WASM web. Icons render as vector paths on both. See `docs/PLATFORMS.md`.

## Notes for AI

This is a **KUI** registry item. Prefer `kui add icon-morph` over swapping two `Button` icons by hand. Pick icon pairs that share a bounding box so the morph stays stable. Never ship an icon-only control without `label`.
