# Agent handoff

A host-controlled assistant-to-person support conversation.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add agent-handoff`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::agent_handoff::{AgentHandoff, HandoffSummary};

let summary = HandoffSummary::new("Customer needs help with an order");
let control = cx.new(|cx| AgentHandoff::new("Support", "Order 1042", summary, window, cx));
```

## Interaction

The host owns the conversation and transfer service. The preview simulates all transfers and sends; nothing is sent.

## API

`connect` · `(u64, impl Into<SharedString>, &mut Context<Self>) -> bool`

Handle Request with connect or fail_transfer using its ticket; Cancel invalidates that request. Handle Send with finish_send using its ticket; failure preserves the draft, CancelSend invalidates it, and successful messages are appended by the host with unique IDs. Mark messages after_handoff to display them after the summary card.

Event: `AgentHandoffEvent`.

## Motion

The shared summary expands on a 240 ms spring, respecting reduced motion. Streaming, traveling timeline dot and complete visual comparison remain unimplemented or unverified.

Desktop and web use the same GPUI view.
