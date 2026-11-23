# Agent thread

Converse with an orchestrator agent through streaming tool traces, task progress cards, inline approvals, docked decisions, and composer.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add agent-thread`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::agent_thread::{AgentThreadScreen, AgentThreadEvent};

let screen = cx.new(|cx| AgentThreadScreen::new("agent-thread", cx));
cx.subscribe(&screen, |_, event: &AgentThreadEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Enter sends messages, Shift+Enter adds a newline, collapsible tool groups toggle on click or Enter, and artifact pills open the workspace panel.

## API

Primary GPUI type: `AgentThreadScreen` (`kui::components::agent_thread::AgentThreadScreen`).
Event type: `AgentThreadEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile adaptation.

## Motion

Interactive transitions, live status indicators, and skeleton sweeps follow KUI motion curves and respect Theme::reduced_motion. Responsive breakpoints adapt columns, toolbars, and side inspectors cleanly from full desktop down to 390px mobile viewports.

Desktop and web use the same GPUI source.
