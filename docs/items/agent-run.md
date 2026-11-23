# Agent run

Follow an autonomous multi-app run with a 5-step plan, numbered activity stream, risk-gated email approval, and 9-hunk inline diff review.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add agent-run`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::agent_run::{AgentRunScreen, AgentRunEvent};

let screen = cx.new(|cx| AgentRunScreen::new("agent-run", cx));
cx.subscribe(&screen, |_, event: &AgentRunEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Cmd+Enter approves pending input, Y/N accepts or rejects individual diff hunks, Left/Right switches inspector tabs, and Stop/Run again controls state.

## API

Primary GPUI type: `AgentRunScreen` (`kui::components::agent_run::AgentRunScreen`).
Event type: `AgentRunEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile adaptation.

## Motion

Interactive transitions, live status indicators, and skeleton sweeps follow KUI motion curves and respect Theme::reduced_motion. Responsive breakpoints adapt columns, toolbars, and side inspectors cleanly from full desktop down to 390px mobile viewports.

Desktop and web use the same GPUI source.
