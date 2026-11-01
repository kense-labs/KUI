# Calendar

Browse months and choose a day in a Gregorian calendar grid.

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
kui add calendar
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::calendar::{Calendar, CivilDate};

let control = cx.new(|cx| Calendar::new("Review date", CivilDate::today_utc(), cx));
```

## Interaction

Navigate dates with arrows or Page Up/Down. Enter selects the focused day.

## API

`value` · `() -> Option<DateSpan>`

Navigate dates with arrows or Page Up/Down. Enter selects the focused day.

Event: `DateSpan`.

## Motion

Adjacent months coexist during the slide, jumps skip intermediate months, and the selected plate retargets.

Desktop and web use the same GPUI view.
