# Invoice & credit note

Present a billed invoice and its adjustable credit note as tactile stacked paper sheets with live rolling totals.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add invoice-credit`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::invoice_credit::{InvoiceCreditBlock, InvoiceCreditEvent};

let billing = cx.new(|cx| {
    InvoiceCreditBlock::new("invoice-credit", cx)
        .with_invoice_no("INV-2041")
        .with_credit_no("CN-0117")
});
cx.subscribe(&billing, |_, event: &InvoiceCreditEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Use the `Invoice` / `Credit note` segmented switch (Left/Right/Home/End keys), `Credit invoice`, or the peeking back-sheet edge to swap sheets. On the credit note, toggle line checkboxes with Space or click, edit or step per-line credit amounts (`Enter` confirms, `Escape` reverts), and click `Issue credit note` with full undo/reopen support.

## API

Primary GPUI type: `InvoiceCreditBlock` (`kui::components::invoice_credit::InvoiceCreditBlock`).
Event type: `InvoiceCreditEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_seller(...)`, `with_buyer(...)`, `with_invoice_no(...)`, `with_credit_no(...)`, `with_invoice_date(...)`, `with_due_date(...)`, `with_period(...)`, `with_currency(...)`, `with_lines(...)`, `with_loading(true)`, `with_empty(true)`, and responsive desktop-to-mobile adaptation.

## Motion

Switching documents drives a critically-damped `Slide` spring that brings the chosen sheet forward while tucking the other behind with its top edge peeking; checkmarks animate smoothly, `pro_rolling_number` rolls subtotal, tax, and adjustment totals live, and `Theme::reduced_motion` snaps transitions immediately. Below `560px`, party blocks stack and secondary columns collapse cleanly down to `390px`.

Desktop and web use the same GPUI source.
