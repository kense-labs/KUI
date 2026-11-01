# DatePicker

Choose a single calendar date from a compact field.

## Install

```bash
kui add date-picker
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::date_picker::{DatePicker, CivilDate};

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
