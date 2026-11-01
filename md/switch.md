# Switch

Toggle a setting that takes effect immediately.

`kui add switch`

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::switch::Switch;

let control = cx.new(|cx| Switch::new("digest", "Weekly digest", window, cx).with_checked(true));
```

with_checked sets the initial value. set_checked synchronizes host updates without echoing a change event.

## Interaction

Click the label or activate with Enter or Space. A disabled switch ignores input. An 18px thumb stretches by 5px on press and glides across its 42 × 24px track.

- Event: `bool`.
- Preview: [English](/en/kui/components/switch/) · [中文](/kui/components/switch/).
