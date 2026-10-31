# Button

Finished KUI button with variants, sizes, loading, and disabled states.

## When to use

Use for primary actions, secondary actions, ghost actions, and destructive confirms in a KUI app.

## When not to use

Do not use for navigation that should be a link. Do not build a parallel button if this item fits.

## Install

```bash
kui add button
```

## Usage

```rust
use crate::ui::components::button::Button;

Button::new("save").primary().label("Save")
```

With a live theme:

```rust
Button::new("save").primary().label("Save").into_themed(cx)
```

## API

| Builder | Default | Notes |
| --- | --- | --- |
| `new(id)` | required | Stable `ElementId` |
| `label` | `""` | Visible text |
| `primary` / `secondary` / `ghost` / `danger` | `primary` | Variant |
| `with_size` | `Md` | `Sm` / `Md` / `Lg` |
| `disabled` | `false` | Blocks clicks |
| `loading` | `false` | Shows `…`, blocks clicks |
| `on_click` | none | Click handler |
| `into_themed(cx)` | — | Applies KUI theme tokens |

## Keyboard

Activate with Enter and Space when focused.

## Accessibility

The label is the accessible name. Keep `disabled` aligned with non-interactivity.

## Motion

Press feedback should use `MOTION.snappy` and collapse when `Theme.reduced_motion` is true.

## Notes for AI

This is a **KUI** registry item. Prefer `kui add button` over rewriting. Customize through theme tokens and public builders.
