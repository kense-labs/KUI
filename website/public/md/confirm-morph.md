# Confirm morph

Destructive button that morphs into a confirm, then into Undo.

## When to use

Use for deletes and resets where one extra step is enough: remove item, clear history, revoke key.

## When not to use

Do not use for irreversible actions that need a typed confirmation; use a dialog. Do not use for non-destructive toggles; use `action-swap`.

## Install

```bash
kui add confirm-morph
```

Only `button` is in the MVP registry today. This item documents the Buttons family contract and lands next.

## Usage

```rust
use crate::ui::components::confirm_morph::ConfirmMorph;

ConfirmMorph::new("delete")
    .label("Delete")
    .confirm_label("Confirm")
    .undo_label("Undo")
    .undo_for(Duration::from_secs(5))
    .on_confirm(|_window, _cx| schedule_delete())
    .on_undo(|_window, _cx| cancel_delete())
    .into_themed(cx)
```

## API

| Builder | Default | Notes |
| --- | --- | --- |
| `new(id)` | required | Stable `ElementId`; owns `ConfirmPhase` |
| `label` | `""` | Idle text |
| `confirm_label` | `"Confirm"` | Armed text |
| `undo_label` | `"Undo"` | Shown during the undo window |
| `arm_timeout` | `3s` | Armed state reverts to idle |
| `undo_for` | `5s` | Undo window after confirm |
| `with_size` | `Md` | `Sm` / `Md` / `Lg` |
| `disabled` | `false` | Blocks all phases |
| `on_confirm` | required | Fires when the undo window ends, or immediately if `undo_for` is zero |
| `on_undo` | none | Fires when Undo is chosen |
| `into_themed(cx)` | — | Applies KUI theme tokens |

## Keyboard

Enter and Space arm, then confirm. Escape cancels the armed state. Focus stays on the button through every phase.

## Accessibility

Announce each phase change through a polite live region. The armed and undo labels are the accessible name. Never use color alone to signal danger; keep the outlined danger shape and the text.

## Motion

The button morphs between phases with `MOTION.morph`, holding height and animating width. The undo window shows a thin countdown line. When `Theme.reduced_motion` is true, labels swap instantly and the countdown line is static.

## Platforms

Works on native desktop and WASM web. Timers use GPUI executors. See `docs/PLATFORMS.md`.

## Notes for AI

This is a **KUI** registry item. Prefer `kui add confirm-morph` over a modal for simple deletes. Run the real destructive work only from `on_confirm`. Use pure black/white tokens with the outlined danger style.
