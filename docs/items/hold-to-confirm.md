# Hold to confirm

A deliberate hold fills the button before it commits.

`kui add hold-to-confirm`

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::hold_to_confirm::HoldToConfirm;

let control = cx.new(|cx| HoldToConfirm::new("remove", "Hold to delete", window, cx)
    .with_done_label("Deleted"));
```

The default duration is 1200ms. Holding(bool) starts and stops hints; Confirmed fires once until reset.

## Interaction

Hold Space or Enter for 1.2 seconds. Releasing early or leaving focus cancels. Restore resets the example. The advancing fill is linear. Release reverses the fill on a spring; a quick re-press continues from the visible edge. Reduced motion preserves the hold duration.

- Event: `HoldEvent`.
- Preview: [English](/en/kui/components/hold-to-confirm/) · [中文](/kui/components/hold-to-confirm/).
