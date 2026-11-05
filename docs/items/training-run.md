# Training run

Monitor a host-controlled training job and inspect train and validation loss.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add training-run`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::training_run::{TrainingRun, TrainingRunSnapshot};

let job = TrainingRunSnapshot::new("Support classifier", "Example model", 1200, 3);
let control = cx.new(|_| TrainingRun::new(job));
```

## Interaction

Status, metrics, events, errors and endpoint come from the host; the component does not train or deploy. Actions are opt-in and must be resolved by request ID. Cancel requires two presses within three seconds.

## API

`resolve` · `(u64, Result<TrainingRunSnapshot, SharedString>, &mut Context<Self>)`

Provide a TrainingRunSnapshot, enable actions with with_actions, subscribe to TrainingRunRequest and call resolve. Supply a fixed positive loss ceiling when needed; the default is 2. Chart focus accepts arrows, Shift+arrows, Home, End and Escape.

Event: `TrainingRunRequest`.

## Motion

The demo is simulated and pauses its updates when hidden. Curves currently update directly without smoothing; pointer scrubbing, result-card transitions and real screen-reader, responsive, desktop and Web acceptance remain pending.

Desktop and web use the same GPUI view.
