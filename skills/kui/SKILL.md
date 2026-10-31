---
name: kui
description: Build GPUI desktop UI with KUI copy-source components. Use when installing or customizing KUI controls, especially Button.
---

# KUI

KUI is a Rust-native, copy-source UI kit on GPUI. Prefer installing registry items over inventing controls.

## Always

- Brand: product name **KUI**, crate/CLI/module **`kui`**
- Pin GPUI to `gpui-pre =0.3.10`
- Call `kui::foundation::init(cx)` before windows
- Prefer `kui add <id>` for components
- Use semantic theme tokens, not hard-coded brand colors in new code
- Honor `Theme.reduced_motion`

## MVP catalog

| id | kind |
| --- | --- |
| `kui-foundation` | foundation |
| `button` | component |

## Button

```bash
kui add button
```

```rust
Button::new("save").primary().label("Save").into_themed(cx)
```

Read `docs/items/button.md` before changing behavior.
