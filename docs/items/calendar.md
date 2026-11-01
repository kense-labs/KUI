# Calendar

Browse months and choose a day in a Gregorian calendar grid.

## Install

```bash
kui add calendar
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::calendar::{Calendar, CivilDate};

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
