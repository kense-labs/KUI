# Stepper

Guide a user through the stages of a sequential workflow.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Library examples use `kui::` imports. Call `kui::init(cx)` before opening windows.
See the [platform guide](https://github.com/kense-labs/KUI/blob/main/docs/PLATFORMS.md)
for features and host setup. Never enable `test-support` on wasm32.

To own this component's source instead:

```bash
cargo install --git https://github.com/kense-labs/KUI --locked kui-cli
kui init
kui add stepper
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::stepper::{Stepper, Step};

let control = cx.new(|_| Stepper::new(vec![Step::new("Profile", "Add your name"), Step::new("Review", "Confirm the details")], 0));
```

## Interaction

Continue advances; earlier steps remain available. The last action emits Finished.

## API

`select` · `usize`

Use `with_compact(true)` with `set_vertical(true, cx)` to place the marker rail beside the form. The default vertical layout also labels each step. The 28px markers and 2px connectors share an axis in both orientations.

Event: `StepEvent`.

## Motion

The active ring scales, completed markers draw their check, and connectors fill in sequence. Validation can hold a step in place and change the message.

Desktop and web use the same GPUI view.
