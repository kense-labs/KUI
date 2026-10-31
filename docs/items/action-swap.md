# Action swap

Button that cycles through related states, such as Follow and Following.

## When to use

Use for a reversible two- or three-state action where the label reflects current state: follow, subscribe, save, mute.

## When not to use

Do not use for destructive actions; use `confirm-morph`. Do not use for async commits; use `action-button`. Do not use for a pure on/off icon; use `icon-morph`.

## Install

```bash
kui add action-swap
```

Only `button` is in the MVP registry today. This item documents the Buttons family contract and lands next.

## Usage

```rust
use crate::ui::components::action_swap::{ActionSwap, SwapState};

ActionSwap::new("follow")
    .state(SwapState::new("follow", "Follow").primary())
    .state(SwapState::new("following", "Following").secondary())
    .hold_for(Duration::from_millis(800))
    .current("follow")
    .on_swap(|next, _window, _cx| set_follow(next))
    .into_themed(cx)
```

## API

| Builder | Default | Notes |
| --- | --- | --- |
| `new(id)` | required | Stable `ElementId` |
| `state(SwapState)` | none | `SwapState::new(id, label)`; variant per state |
| `current` | first state | Controlled state id |
| `hold_for` | none | Delay before a swap back or forward commits, so a hover or press can be undone |
| `with_size` | `Md` | `Sm` / `Md` / `Lg` |
| `disabled` | `false` | Blocks swaps |
| `on_swap` | none | Receives the next state id |
| `into_themed(cx)` | — | Applies KUI theme tokens |

## Keyboard

Enter and Space advance to the next state. If `hold_for` is set, activation again before it elapses cancels the swap.

## Accessibility

The current state label is the accessible name. Expose the toggled state when exactly two states exist. Announce the new label after each swap.

## Motion

Labels swap with `MOTION.morph` and the width interpolates between states. Variant fill changes with `MOTION.snappy`. When `Theme.reduced_motion` is true, label and fill change instantly.

## Platforms

Works on native desktop and WASM web. See `docs/PLATFORMS.md`.

## Notes for AI

This is a **KUI** registry item. Prefer `kui add action-swap` over a `Button` whose label you flip by hand. Use state ids, not magic strings in labels. Keep two to three states.
