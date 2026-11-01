# TimePicker

Select a time of day from a scrollable list.

## Install

```bash
kui add time-picker
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::time_picker::{TimePicker, WallTime};

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
