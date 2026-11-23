# StepConsole

Follow multi-stage execution steps beside a docked, filterable step log console.

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
kui add step-console
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::step_console::{StepConsole, StepStage, StepStatus};

let control = cx.new(|_| {
    StepConsole::new(
        "pipeline",
        "Pipeline execution",
        vec![StepStage::new("boot", "Runtime boot", StepStatus::Running)],
        vec![],
    )
});
```

## Interaction

Select any stage on the left to inspect its summary, diagnostic note, metrics, and scoped logs. Toggle Auto-follow to track failed or active stages automatically, switch between Current step and All logs, or isolate Error entries.

## API

`new` · `(id, title, Vec<StepStage>, Vec<StepLogLine>) → Self`

`set_stages`, `set_logs`, `push_log`, and `select_stage` update live execution state. `selected_stage_id` and `visible_logs` inspect the current selection.

Event: `StepSelected`.

Desktop and web use the same GPUI view.
