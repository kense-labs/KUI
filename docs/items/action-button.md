# Action button

Button that runs an async commit and shows pending, success, or failure on the control itself.

## When to use

Use when a click starts work that takes time and the result belongs on the button: save, publish, sync, send.

## When not to use

Do not use for instant, synchronous actions; use `button`. Do not use for work that needs a progress bar or a log; show that elsewhere.

## Install

```bash
kui add action-button
```

Only `button` is in the MVP registry today. This item documents the Buttons family contract and lands next.

## Usage

```rust
use crate::ui::components::action_button::{ActionButton, CommitError};

ActionButton::new("publish")
    .label("Publish")
    .pending_label("Publishing")
    .success_label("Published")
    .on_commit(|_window, cx| {
        cx.background_spawn(async move { publish().await.map_err(CommitError::from) })
    })
    .into_themed(cx)
```

The button returns to idle after `settle_after` elapses.

## API

| Builder | Default | Notes |
| --- | --- | --- |
| `new(id)` | required | Stable `ElementId`; owns phase state |
| `label` | `""` | Idle text |
| `pending_label` / `success_label` / `failed_label` | idle label | In-place text per phase |
| `primary` / `secondary` / `ghost` / `danger` | `primary` | Reuses `ButtonVariant` |
| `with_size` | `Md` | `Sm` / `Md` / `Lg` |
| `on_commit` | required | Returns `Task<Result<(), CommitError>>` |
| `settle_after` | `1500ms` | Time in success or failed before idle |
| `disabled` | `false` | Blocks commits |
| `phase` | none | Controlled `ActionPhase` (`Idle` / `Pending` / `Success` / `Failed`) |
| `into_themed(cx)` | — | Applies KUI theme tokens |

## Keyboard

Enter and Space start a commit when idle. Input is ignored while `Pending`. Focus stays on the button across phases.

## Accessibility

The current phase label is the accessible name. Announce phase changes through a polite live region. Keep the control width stable so layout does not jump.

## Motion

Label swaps use `MOTION.morph` (short cross-fade with width held). Press feedback uses `MOTION.snappy`. When `Theme.reduced_motion` is true, swap labels instantly with no scale or fade.

## Platforms

Works on native desktop and WASM web. Spawn work through GPUI executors, not threads. See `docs/PLATFORMS.md`.

## Notes for AI

This is a **KUI** registry item. Prefer `kui add action-button` over wrapping `Button` with your own state. Never block inside `on_commit`; return a `Task`. Use `ActionPhase` instead of bool pairs.
