# Agent chat progress

Run tool-using chat turns beside a dedicated Progress sidebar showing collapsible step checklists, generated documents, and session connectors.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add agent-chat-progress`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::agent_chat_progress::{AgentChatProgressScreen, AgentChatProgressEvent};

let screen = cx.new(|cx| AgentChatProgressScreen::new("agent-chat-progress", cx));
cx.subscribe(&screen, |_, event: &AgentChatProgressEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

J/K steps through tool calls, Enter/Space expands JSON arguments and results, P or [ toggles the progress panel, R replays, and Cmd+Enter sends.

## API

Primary GPUI type: `AgentChatProgressScreen` (`kui::components::agent_chat_progress::AgentChatProgressScreen`).
Event type: `AgentChatProgressEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile adaptation.

## Motion

Interactive transitions, live status indicators, and skeleton sweeps follow KUI motion curves and respect Theme::reduced_motion. Responsive breakpoints adapt columns, toolbars, and side inspectors cleanly from full desktop down to 390px mobile viewports.

Desktop and web use the same GPUI source.
