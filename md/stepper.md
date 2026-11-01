# Stepper

Guide a user through the stages of a sequential workflow.

## Install

```bash
kui add stepper
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::stepper::{Stepper, Step};

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
