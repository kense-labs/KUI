# Agent loop

Follow an agent through context gathering, tool calls, and verification.

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

Use Play/Pause, Next step or Restart to control the sample run. Tab to the controls and use the arrow keys to choose a playback speed. The host supplies the timeline; the component does not call an agent or tools.

## API

`set_playing` · `(bool, &mut Context<Self>)`

Use set_playing, step, restart and set_speed. Autoplay is off by default; autoplay(true) starts the first visible run. Set visibility in an embedded host to pause when the preview leaves the viewport. Events report station changes and completion.

Event: `AgentLoopEvent`. The Rust `restart` method returns to Ready; the preview’s Restart button immediately starts a new run. Next step pauses after the next context or result arrives. At narrow widths the last result occupies one shared slot; wider layouts retain the latest tool and check separately.

## Motion

The active point travels clockwise and pauses in place. Tools and checks resolve in fixed slots. Off-screen playback pauses. Reduced motion removes the moving trail and reveals each step without travel.

Desktop and web use the same GPUI view.
