# Approval card

Ask a person before an agent executes a consequential action, with a risk badge, recipient preview, pausing countdown ring, and resolved-row morph.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add approval-card`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::approval_card::{ApprovalCard, ApprovalCardEvent, ApprovalDetail, ApprovalRisk};

let card = cx.new(|cx| {
    ApprovalCard::new("send-campaign-approval", cx)
        .with_title("Send campaign email")
        .with_description(Some("October product update · Newsletter".into()))
        .with_risk(Some(ApprovalRisk::Medium))
        .with_details(vec![
            ApprovalDetail::new("Subject", "What's new in October"),
            ApprovalDetail::new("Send", "Today, 9:00 AM"),
        ])
        .with_expires_in(Some(45))
});
cx.subscribe(&card, |_, event: &ApprovalCardEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Hovering or focusing anywhere on the card pauses the live expiration countdown and circular progress ring on `Approve` so the user is never rushed mid-read. Press `⌘↵` (`Ctrl+Enter` on Windows/Linux) or click `Approve` to confirm, press `Escape` or `Deny` to reject, check `Always allow this action` to persist trust, or click `Undo` on the resolved row to return to the pending prompt.

## API

Primary GPUI type: `ApprovalCard` (`kui::components::approval_card::ApprovalCard`).
Event type: `ApprovalCardEvent`.

Supports `ApprovalRisk` (`Low`, `Medium`, `High`), `ApprovalActionsPlacement` (`Tray`, `Card`), optional `with_editable(true)` inline field editing, light/dark `Theme` modes, bilingual `UiLanguage`, and responsive layouts down to `320px`.

## Motion

On `Approve`, `Deny`, or countdown expiration, `pro_fold` smoothly collapses the pending surface into a compact `48px` resolved status row while `Theme::reduced_motion` switches states cleanly without sweep overhead.

Desktop and web use the same GPUI source.
