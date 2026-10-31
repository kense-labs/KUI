# Floating button group

Soft, separate buttons in a quiet tray with a morphing highlight.

## When to use

Use for secondary toolbars that sit over content: editor tools, canvas controls, media actions. Buttons stay distinct and the tray stays calm.

## When not to use

Do not use for form actions or primary calls to action. Use `button-group` when items should read as one joined control.

## Install

```bash
kui add floating-button-group
```

Only `button` is in the MVP registry today. This item documents the Buttons family contract and lands next.

## Usage

```rust
use crate::ui::components::floating_button_group::{FloatingButtonGroup, FloatItem};

FloatingButtonGroup::new("tools")
    .item(FloatItem::new("select", "Select"))
    .item(FloatItem::new("draw", "Draw"))
    .item(FloatItem::new("erase", "Erase"))
    .selected("select")
    .on_change(|id, _window, _cx| set_tool(id))
    .into_themed(cx)
```

## API

| Builder | Default | Notes |
| --- | --- | --- |
| `new(id)` | required | Stable `ElementId` |
| `item(FloatItem)` | none | `FloatItem::new(id, label)`; `.icon(IconName)` optional |
| `selected` | none | Controlled item id |
| `with_size` | `Md` | `Sm` / `Md` / `Lg` |
| `gap` | `4px` | Space between buttons |
| `disabled` | `false` | Blocks the whole tray |
| `on_change` | none | Receives the activated item id |
| `into_themed(cx)` | — | Applies KUI theme tokens |

## Keyboard

Tab enters the tray once. Arrow keys move between buttons. Enter and Space activate. Escape does nothing; the tray never traps focus.

## Accessibility

Expose the tray as a toolbar. Icon-only items must set an accessible label. Keep contrast of the highlight against the tray at WCAG AA.

## Motion

The highlight morphs between buttons with `MOTION.morph`, changing position and width together. Hover lifts with `MOTION.snappy`. When `Theme.reduced_motion` is true, the highlight swaps in place.

## Platforms

Works on native desktop and WASM web. The tray uses a 1px hairline border, not a host blur effect, so it renders the same everywhere. See `docs/PLATFORMS.md`.

## Notes for AI

This is a **KUI** registry item. Prefer `kui add floating-button-group` over absolute-positioned `Button` rows. Use black/white tokens; do not tint the tray with the Kense brand color.
