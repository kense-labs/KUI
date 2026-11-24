# Approval gates

Review pending approval requests across an active agent run as a physical card stack, with diffs, inline edits, held downstream steps, and an expandable decided log.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add approval-gates`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::approval_gates::{ApprovalGates, ApprovalGatesEvent};

let gates = cx.new(|cx| {
    ApprovalGates::new("run-approval-gates", cx)
        .with_task(Some("Launch the new pricing page".into()))
        .with_live(true)
});
cx.subscribe(&gates, |_, event: &ApprovalGatesEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

The top card in the deck shows the requesting agent (`pro_agent_avatar`), exact file diff or command arguments, reason, and expandable `Full context`. Click `Edit` to modify arguments or attach change instructions (`⌘↵` approves with your changes), `Approve` to advance to the next stacked gate, or `Reject` to halt dependent steps. Use `←` and `→` (or click a peeking back card) to cycle open requests, and click any row in `Decided` to inspect its audit record.

## API

Primary GPUI type: `ApprovalGates` (`kui::components::approval_gates::ApprovalGates`).
Event type: `ApprovalGatesEvent`.

Supports `GateKind` (`Tool`, `File`, `Agent`, `Message`), `GateFileDiff` (`Add`, `Del`, `Context`), `holds` step tracking, light/dark `Theme` modes, and bilingual `UiLanguage`.

## Motion

Deciding a card drops it into the `Decided` list while the next queued card rises smoothly via `Slide` springs (`deck_rise`), rolling counters update `Waiting on you`, and `pro_fold` animates context and decided-row disclosures while respecting `Theme::reduced_motion`.

Desktop and web use the same GPUI source.
