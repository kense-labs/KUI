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

The preview offers Live, Error and No transcript. Use `MeetingStatus::NoTranscript` when the host has no source transcript.

The host provides notes, runs regeneration and sends actions; the example simulates sends. Copy writes to the real clipboard and reports success or failure.

## API

`finish_send` · `(u64, Result<SharedString, SharedString>, &mut Context<Self>)`

Handle Send, then finish_send with its ticket. Pending sends are deduplicated and cancelled when data changes. Timestamp buttons expose source quotes; Escape closes a quote.

Use with_transcript(credit, words) to display attribution and transcript size. Timestamp buttons open quotes. Escape closes the quote. Send and finish_send use the same ticket.

Event: `MeetingSummaryEvent`.

## Motion

When data arrives after Summarizing, summary points, decisions and actions reveal in order. Each row starts 170 ms after the previous one, and its text appears in five groups. The body follows its measured height with a critically damped 240 ms visual transition. Reduced motion displays all content immediately.

Desktop and web use the same GPUI view.
