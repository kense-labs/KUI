# Agent loop

An illustrative gather, act and verify timeline.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add agent-loop`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::agent_loop::{AgentLoop, LoopIteration};

let iterations = vec![LoopIteration::new("Read logs", 1200, true, "Checks passed")];
let control = cx.new(|_| AgentLoop::new("Verify the build", iterations));
```

## Interaction

The host supplies the whole illustrative timeline; this component makes no agent or tool calls.

## API

`set_playing` · `(bool, &mut Context<Self>)`

Use set_playing, step, restart and set_speed. Events report station changes and completion.

Event: `AgentLoopEvent`.

## Motion

The head follows the ring during playback and stays at its current position when paused; off-screen playback pauses. Reduced motion skips travel. Detailed visual transitions remain to be verified.

Desktop and web use the same GPUI view.
