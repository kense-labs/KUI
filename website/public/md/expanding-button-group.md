# Expanding button group

Compact icon bar that reveals labels on hover or focus.

## When to use

Use when space is tight and the actions are familiar: row actions, card toolbars, dense editors.

## When not to use

Do not use when icons are not self-evident and users must read labels at all times; use `button-group`. Do not use for touch-only layouts where hover never occurs.

## Install

```bash
kui add expanding-button-group
```

Only `button` is in the MVP registry today. This item documents the Buttons family contract and lands next.

## Usage

```rust
use crate::ui::components::expanding_button_group::{ExpandItem, ExpandingButtonGroup};

ExpandingButtonGroup::new("row-actions")
    .item(ExpandItem::new("edit", IconName::Pencil, "Edit"))
    .item(ExpandItem::new("share", IconName::Share, "Share"))
    .item(ExpandItem::new("delete", IconName::Trash, "Delete"))
    .on_activate(|id, _window, _cx| run_action(id))
    .into_themed(cx)
```

## API

| Builder | Default | Notes |
| --- | --- | --- |
| `new(id)` | required | Stable `ElementId` |
| `item(ExpandItem)` | none | `ExpandItem::new(id, icon, label)` |
| `expand_on(ExpandTrigger)` | `HoverOrFocus` | `HoverOrFocus` / `Focus` / `Always` |
| `with_size` | `Md` | `Sm` / `Md` / `Lg` |
| `disabled` | `false` | Blocks the whole bar |
| `on_activate` | none | Receives the activated item id |
| `into_themed(cx)` | — | Applies KUI theme tokens |

## Keyboard

Focus expands the focused item. Arrow keys move between items. Enter and Space activate. Leaving the group collapses it.

## Accessibility

The label is always the accessible name, even while collapsed. Never rely on hover alone: keyboard focus must expand the same label. The collapsed hit target stays at least 32px.

## Motion

Labels expand with `MOTION.morph` (width and opacity together); neighbors shift without overlap. Use `MOTION.snappy` for press. When `Theme.reduced_motion` is true, labels appear and disappear without width animation.

## Platforms

Works on native desktop and WASM web. On touch hosts, use `expand_on(Always)` or a tap-to-expand host policy. See `docs/PLATFORMS.md`.

## Notes for AI

This is a **KUI** registry item. Prefer `kui add expanding-button-group` over hand-rolled hover reveals. Always provide a label per item. Reserve the expanded width so layout does not jump.
