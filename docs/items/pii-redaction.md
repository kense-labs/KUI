# PII redaction

Review masked messages and restore model replies using local values.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add pii-redaction`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::pii_redaction::PiiRedaction;

let control = cx.new(|cx| {
    PiiRedaction::new(window, cx)
        .with_model("Claude Sonnet 5.5")
        .with_reply()
});
```

## Interaction

Scan the message, inspect the four type mappings and use Show original to compare the masked text. Review the ranges before copying or requesting a model reply. The reply streams with placeholders, then restores known values locally when the matching request completes. Stop cancels an active scan or reply. Reset clears the current result and returns focus to the message.

Cmd/Ctrl+Enter scans from the message field. Escape stops the pending operation. The preview uses local detection and reply simulations with four fictional values.

## API

Use `with_model(name)` to name the host-selected scan model. The range cards show the same replacement tokens used in the masked preview.

- `with_reply(self) -> Self` enables the masked reply workflow. The default is scan and copy.
- `request_reply(&mut self, cx: &mut Context<Self>) -> bool` emits a request after the current message and ranges have been reviewed.
- `append_reply(&mut self, id: u64, chunk: &str, cx: &mut Context<Self>) -> bool` appends a response chunk containing placeholders.
- `finish_reply(&mut self, id: u64, cx: &mut Context<Self>) -> bool` restores this request's known placeholders and drops its value map.
- `fail_reply(&mut self, id: u64, error: impl Into<SharedString>, cx: &mut Context<Self>) -> bool` records a matching failure.
- `reply_text(&self) -> &str` returns the streaming placeholder text or the completed restored text.
- `reply_phase(&self) -> PiiReplyPhase` returns `Idle`, `Sending`, `Streaming`, `Complete`, `Failed` or `Stopped`.

`finish_scan` accepts `(u64, Vec<PiiSpan>, &mut Context<Self>) -> bool`. Handle `PiiEvent::ScanRequested` and `PiiEvent::Cancelled` in the detector host. Pass non-overlapping UTF-8 byte ranges aligned to Unicode graphemes to `finish_scan`, or call `fail_scan`.

For replies, subscribe to `PiiReplyEvent::Requested { id, prompt }` and `PiiReplyEvent::Cancelled(id)`. The model receives the masked prompt. Edits, range changes, removal of review approval and cancellation invalidate the pending reply. Unknown placeholders stay intact. Restoration performs one pass, including when a restored value itself contains placeholder text.

Events use the separate `PiiEvent` and `PiiReplyEvent` types.

## Motion

The Show original thumb settles over approximately 160ms. Each active connector has a 460ms dot movement. Completed reply text fades out for 100ms and restored text fades in for 160ms in the same reply well. The local sample streams four words per 34ms. Reduced motion shows complete sample chunks and the restored text immediately.

Desktop and web use the same GPUI view.
