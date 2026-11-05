# Bottom sheet

Open a bottom panel with draggable snap heights.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add bottom-sheet`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::bottom_sheet::BottomSheet;

let body = Box::new(|_: &mut gpui::Window, _: &mut gpui::App| gpui::div().child("Flights, hotel, and bookings.").into_any_element());
let control = cx.new(|cx| BottomSheet::new("Trip details", "Lisbon, 3 nights", body, cx));
```

## Interaction

Drag the handle or use its arrow keys, Home, End, Enter and Space. Escape closes the sheet.

## API

`with_detents` · `(Vec<f32>) -> Self`

Detents are fractions of the available height. The sheet emits open, close and snap events and returns focus on close.

Event: `BottomSheetEvent`.

## Motion

The sheet follows touch, resists at the top and projects flick speed; reduced motion updates immediately.

Desktop and web use the same GPUI view.
