# Agent memory

Inspect, edit, pin, review, and forget facts your agent learned across projects with full source provenance.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add agent-memory`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::agent_memory::{AgentMemoryScreen, AgentMemoryEvent};

let screen = cx.new(|cx| AgentMemoryScreen::new("agent-memory", cx));
cx.subscribe(&screen, |_, event: &AgentMemoryEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

J/K or arrows move through memories, Enter opens provenance, N adds a memory, P pins, R keeps a candidate, Backspace/Delete forgets with Undo, and / searches.

## API

Primary GPUI type: `AgentMemoryScreen` (`kui::components::agent_memory::AgentMemoryScreen`).
Event type: `AgentMemoryEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile adaptation.

## Motion

Interactive transitions, live status indicators, and skeleton sweeps follow KUI motion curves and respect Theme::reduced_motion. Responsive breakpoints adapt columns, toolbars, and side inspectors cleanly from full desktop down to 390px mobile viewports.

Desktop and web use the same GPUI source.
