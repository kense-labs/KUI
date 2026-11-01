# DateRangePicker

Set the start and end of a bounded calendar interval.

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
kui add date-range-picker
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::date_range_picker::{DateRangePicker, CivilDate};

let control = cx.new(|cx| DateRangePicker::new("Review date", CivilDate::today_utc(), cx));
```

## Interaction

Choose two endpoints or a preset, then Apply; Escape discards the draft.

## API

`value` · `() -> Option<DateSpan>`

with_bounds accepts optional inclusive start and end limits. Only Apply commits the draft and emits DateSpan when it changes.

Event: `DateSpan`.

## Motion

The date label moves between the field and the panel summary as the surface changes size. Reversing the transition keeps the label in motion. Reduced motion immediately places it in the active slot.

Desktop and web use the same GPUI view.
