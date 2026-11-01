# DatePicker

Choose a single calendar date from a compact field.

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
kui add date-picker
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::date_picker::{DatePicker, CivilDate};

let control = cx.new(|cx| DatePicker::new("Review date", CivilDate::today_utc(), cx));
```

## Interaction

Arrows move by day or week. Page Up and Page Down change months. Enter selects the focused day. Clear removes the current date.

## API

`value` · `() -> Option<DateSpan>`

with_bounds optionally limits selectable dates. Choosing a new day emits DateSpan. Clearing an existing date emits DateCleared.

Event: `DateSpan`, `DateCleared`.

## Motion

The weekday header stays fixed while the month grid slides. A chosen day stays visible for 300ms before the popup closes. Reduced motion closes it immediately.

Desktop and web use the same GPUI view.
