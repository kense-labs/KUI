# Split button

Primary action joined to a menu of nearby alternatives.

## When to use

Use when one action is the common case and two to five related variants should stay one click away: merge strategies, export formats, send options.

## When not to use

Do not use when the alternatives are unrelated; use `button-group` or a menu. Do not use for a single action; use `button`.

## Install

```bash
kui add split-button
```

Only `button` is in the MVP registry today. This item documents the Buttons family contract and lands next.

## Usage

```rust
use crate::ui::components::split_button::{SplitButton, SplitItem};

SplitButton::new("merge")
    .label("Merge")
    .item(SplitItem::new("squash", "Squash and merge"))
    .item(SplitItem::new("rebase", "Rebase and merge"))
    .on_primary(|_window, _cx| merge_default())
    .on_select(|item, _window, _cx| merge_with(item))
    .into_themed(cx)
```

## API

| Builder | Default | Notes |
| --- | --- | --- |
| `new(id)` | required | Stable `ElementId`; menu id derives from it |
| `label` | `""` | Primary segment text |
| `item(SplitItem)` | none | Menu entry; `SplitItem::new(id, label)` |
| `primary` / `secondary` / `ghost` | `primary` | Variant; `danger` is not offered |
| `with_size` | `Md` | `Sm` / `Md` / `Lg` |
| `on_primary` | none | Fires from the main segment |
| `on_select` | none | Receives the chosen `SplitItem` id |
| `disabled` | `false` | Blocks both segments |
| `open` | uncontrolled | Controlled menu state |
| `into_themed(cx)` | — | Applies KUI theme tokens |

## Keyboard

Enter and Space activate the focused segment. Down Arrow on the chevron opens the menu. Arrow keys move, Enter selects, Escape closes and returns focus to the chevron.

## Accessibility

The primary segment and the chevron are separate focus stops. The chevron is named from the label, for example "More merge options". The menu exposes selectable items with the active one marked.

## Motion

Menu open uses `MOTION.snappy` (short fade and 4px rise). A 1px divider separates the segments and never animates. When `Theme.reduced_motion` is true, the menu appears without movement.

## Platforms

Works on native desktop and WASM web. Position the menu with GPUI anchored overlays, not host windows. See `docs/PLATFORMS.md`.

## Notes for AI

This is a **KUI** registry item. Prefer `kui add split-button` over placing two `Button` values side by side. Keep the primary action also reachable by its own label; the menu holds alternatives only.
