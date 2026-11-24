# Agent steering

Steer an agent mid-run without restarting—queue a note for the next step and watch it glide smoothly into the execution plan when the current step finishes, or interrupt immediately.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add agent-steering`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::agent_steering::{AgentSteering, AgentSteeringEvent};

let steering = cx.new(|cx| {
    AgentSteering::new("migrate-steering", cx)
        .with_title("Migrate users table")
        .with_agent("Opus 5.5")
});
cx.subscribe(&steering, |_, event: &AgentSteeringEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Type an instruction in the bottom `Editor` composer and press `Enter` (or click the send button) to queue it for the next step. Click `Now` on any queued chip—or press `⌘↵` (`Ctrl+↵`) directly in the composer—to interrupt the active step immediately and attach the instruction in place. Use the header button to pause, resume, or replay the run.

## API

Primary GPUI type: `AgentSteering` (`kui::components::agent_steering::AgentSteering`).
Event type: `AgentSteeringEvent`.

Supports custom `SteerStep` plans, `SteerQueuedNote` trays, live or manual ticking (`with_auto_tick`), light/dark `Theme` modes, bilingual `UiLanguage`, loading skeletons, and empty states.

## Motion

Active steps sweep a highlight via `pro_shimmer_text` alongside a spinning arc, the queue tray opens and closes without layout jumps via `pro_fold`, and absorbed notes glide into their target step over `620ms` with a soft blue landing glow; `Theme::reduced_motion` disables continuous motion.

Desktop and web use the same GPUI source.
