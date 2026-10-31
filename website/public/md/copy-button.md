# Copy button

Clipboard copy with Copied or Failed shown in place.

## When to use

Use next to values users paste elsewhere: tokens, install commands, ids, code snippets.

## When not to use

Do not use for actions that are not clipboard writes; use `button` or `action-button`. Do not use to copy secrets without the user asking.

## Install

```bash
kui add copy-button
```

Only `button` is in the MVP registry today. This item documents the Buttons family contract and lands next.

## Usage

```rust
use crate::ui::components::copy_button::CopyButton;

CopyButton::new("copy-install")
    .text("kui add button")
    .label("Copy")
    .on_result(|result, _window, _cx| log_copy(result))
    .into_themed(cx)
```

## API

| Builder | Default | Notes |
| --- | --- | --- |
| `new(id)` | required | Stable `ElementId` |
| `text` | required | Value written to the clipboard |
| `label` | `"Copy"` | Idle text |
| `copied_label` / `failed_label` | `"Copied"` / `"Failed"` | In-place result text |
| `icon_only` | `false` | Shows icon; keeps label as accessible name |
| `primary` / `secondary` / `ghost` | `ghost` | Variant |
| `with_size` | `Md` | `Sm` / `Md` / `Lg` |
| `reset_after` | `1500ms` | Time before returning to idle |
| `on_result` | none | Receives `Result<(), CopyError>` |
| `into_themed(cx)` | — | Applies KUI theme tokens |

## Keyboard

Enter and Space copy. Focus stays on the button; the new label is announced.

## Accessibility

Announce "Copied" or "Failed" through a polite live region. Icon-only mode needs an accessible name from `label`. Do not move focus after copy.

## Motion

Icon and label swap with `MOTION.morph`. Press uses `MOTION.snappy`. When `Theme.reduced_motion` is true, the result state replaces idle instantly.

## Platforms

Uses the GPUI clipboard on native desktop. On WASM web, writes go through the browser clipboard and can fail without a user gesture or secure context; that path returns `CopyError`, shown as Failed. See `docs/PLATFORMS.md`.

## Notes for AI

This is a **KUI** registry item. Prefer `kui add copy-button` over custom clipboard wiring. Always handle `CopyError`; do not assume success. Keep the control width stable between states.
