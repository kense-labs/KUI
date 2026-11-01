# TimePicker

Select a time of day from a scrollable list.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Library examples use `kui::` imports. Call `kui::init(cx)` before opening windows.
See the [platform guide](https://github.com/kense-labs/KUI/blob/main/docs/PLATFORMS.md)
for features and host setup. Never enable `test-support` on wasm32.

To own this component's source instead:

```bash
cargo install --git https://github.com/kense-labs/KUI --locked kui-cli
kui init
kui add time-picker
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::time_picker::{TimePicker, WallTime};

let control = cx.new(|cx| TimePicker::new("Review time", WallTime::new(9, 30).unwrap(), cx).with_step(15));
```

## Interaction

Use Up and Down to browse, including across midnight. Enter commits the highlighted time. Escape keeps the current value and returns focus to the field.

## API

`value` · `() -> WallTime`

WallTime stores the hour and minute. with_step sets the slot interval. with_24h changes only the display format.

Event: `WallTime`.

## Motion

The list opens centered on the current time. Its fixed-height viewport scrolls the active row into view during keyboard navigation.

Desktop and web use the same GPUI view.
