# Scrub input

Edit a numeric value or drag its label to adjust it.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add scrub-input`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::scrub_input::ScrubInput;

let control = cx.new(|cx| ScrubInput::new("Width", 0.0..=1000.0, 1.0, 176.0, "px", window, cx));
```

## Interaction

Drag the label; Shift and Alt use coarse and fine steps. Type an arithmetic expression with an accepted suffix; click the unit to cycle available units. Vertical touch gestures are left for scrolling.

## API

`with_wrap` · `(bool) -> Self`

Set bounds, step, initial value, accepted units and optional wrapping. with_units supplies unit labels; changing units does not convert numeric values. ScrubChanged, ScrubCommitted and ScrubUnitChanged report user changes. Invalid expressions restore the previous value.

Event: `ScrubChanged / ScrubCommitted / ScrubUnitChanged`.

## Motion

Dragging past a non-wrapping limit strains the field and springs back on release. Reduced motion skips the strain.

Desktop and web use the same GPUI view.
