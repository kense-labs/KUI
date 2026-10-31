# Button group

Related actions on one surface with a shared highlight.

## When to use

Use for a small set of peer actions or modes that read as one control: view toggles, alignment, range pickers.

## When not to use

Do not use for unrelated actions; use separate `button` values. Do not use for page navigation. Use `floating-button-group` when each button should stay visually separate.

## Install

```bash
kui add button-group
```

Only `button` is in the MVP registry today. This item documents the Buttons family contract and lands next.

## Usage

```rust
use crate::ui::components::button_group::{ButtonGroup, GroupItem};

ButtonGroup::new("view")
    .item(GroupItem::new("list", "List"))
    .item(GroupItem::new("grid", "Grid"))
    .selected("list")
    .on_change(|id, _window, _cx| set_view(id))
    .into_themed(cx)
```

## API

| Builder | Default | Notes |
| --- | --- | --- |
| `new(id)` | required | Stable `ElementId` |
| `item(GroupItem)` | none | `GroupItem::new(id, label)` |
| `selected` | none | Controlled selected item id |
| `selection(GroupSelection)` | `Single` | `Single` / `None` (plain actions) |
| `with_size` | `Md` | `Sm` / `Md` / `Lg` |
| `disabled` | `false` | Blocks the whole group |
| `on_change` | none | Receives the activated item id |
| `into_themed(cx)` | — | Applies KUI theme tokens |

## Keyboard

Tab enters the group once. Left and Right Arrow move focus between items. Enter and Space activate. Home and End jump to the first and last item.

## Accessibility

Expose the group as a single toolbar or radio group depending on `selection`. Each item needs a visible label or an explicit accessible name. Disabled items stay in order and are skipped by arrows.

## Motion

The shared highlight slides between items with `MOTION.morph`. Outer corners keep the 8px radius; inner edges are square with 1px dividers. When `Theme.reduced_motion` is true, the highlight jumps without sliding.

## Platforms

Works on native desktop and WASM web. See `docs/PLATFORMS.md`.

## Notes for AI

This is a **KUI** registry item. Prefer `kui add button-group` over rows of independent buttons with hand-rolled borders. Keep item ids stable; the highlight tracks them. Use pure black/white tokens for the highlight.
