# SlotText

Animate changes to a short numeric value with rolling digit cells.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add slot-text`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::slot_text::SlotText;

let control = cx.new(|_| SlotText::new("12,480"));
```

## Interaction

Change text with set_value; digits align from the right.

## API

`set_value` · `(impl Into<SharedString>, &mut Context<Self>)`

Change text with set_value; digits align from the right.

Event: `—`.

## Motion

Digit reels roll and settle with stagger; reduced motion jumps to the target.

Desktop and web use the same GPUI view.
