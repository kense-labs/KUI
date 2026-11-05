# Meeting summary

Host-provided notes, decisions, citations and action items.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add meeting-summary`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::meeting_summary::{MeetingData, MeetingSummary};

let control = cx.new(|_| MeetingSummary::new(MeetingData::new("Release review")));
```

## Interaction

The host provides notes, runs regeneration and sends actions; the example simulates sends. Copy writes to the real clipboard and reports success or failure.

## API

`finish_send` · `(u64, Result<SharedString, SharedString>, &mut Context<Self>)`

Handle Send, then finish_send with its ticket. Pending sends are deduplicated and cancelled when data changes. Timestamp buttons expose source quotes; Escape closes a quote.

Event: `MeetingSummaryEvent`.

## Motion

States update immediately. Streaming, quote hover intent and detailed transitions are not yet implemented or visually verified.

Desktop and web use the same GPUI view.
