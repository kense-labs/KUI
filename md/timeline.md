# Timeline

Read chronological updates grouped by date.

## Install

```bash
kui add timeline
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::timeline::{Timeline, TimelineEntry};

let control = cx.new(|_| Timeline::new(vec![TimelineEntry::new("Today", "Deployment passed", "Build 1284", "11m")]));
```

## Interaction

Group consecutive entries by day. Activate an entry for its detail.

## API

`prepend` · `TimelineEntry`

Group consecutive entries by day. Activate an entry for its detail.

## Motion

New entries open measured space above their neighbors. The marker grows before its stem extends. Detail disclosures stay mounted during exit and reverse from their current height.

Desktop and web use the same GPUI view.
