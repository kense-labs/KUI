# FilterToolbar

Combine and remove facet filters above a result set.

## Install

```bash
kui add filter-toolbar
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::filter_toolbar::{FilterToolbar, Facet};

let control = cx.new(|_| FilterToolbar::new(vec![Facet::new("Status", vec!["In review".into(), "Done".into()])], vec![(0,0)]));
```

## Interaction

Edit a chip, remove it, add a facet or clear all selections.

## API

`set` · `(&mut self, facet: usize, value: Option<usize>, &mut Context<Self>)`

Each facet accepts one option. Change events contain the complete selection map.

Event: `FiltersChanged`.

## Motion

Filter chips spring their widths. Add filter grows into an anchored chooser, and the contents slide when advancing to values.

Desktop and web use the same GPUI view.
