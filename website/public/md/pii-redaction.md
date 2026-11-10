# PII redaction

Review and edit host-provided sensitive text ranges before copying a masked preview.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add pii-redaction`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::pii_redaction::PiiRedaction;

let control = cx.new(|cx| PiiRedaction::new(window, cx));
```

## Interaction

Select text to add ranges, remove or undo them, then explicitly review before copying. The preview only matches fixed fictional values and does not send text or detect real PII.

## API

Use `with_model(name)` to name the host-selected scan model. The range cards show the same replacement tokens used in the masked preview.

`finish_scan` · `(u64, Vec<PiiSpan>, &mut Context<Self>) -> bool`

Handle ScanRequested and Cancelled in the host. Pass the matching ID and non-overlapping UTF-8 byte ranges aligned to Unicode graphemes to finish_scan, or call fail_scan. Edits and cancellation invalidate old results.

Event: `PiiEvent`.

## Motion

The demo keeps feedback immediate and does not require motion.

Desktop and web use the same GPUI view.
