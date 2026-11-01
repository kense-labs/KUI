# ReorderableList

Change list order with a pointer or keyboard.

## Install

```bash
kui add reorderable-list
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::reorderable_list::{ReorderableList, OrderedItem};

let control = cx.new(|cx| ReorderableList::new(vec![OrderedItem::new("a","Review changes","Product brief"),OrderedItem::new("b","Ship","Release")],cx));
```

## Interaction

Drag a handle onto a row or use Up/Down while focused.

## API

`move_to` · `(&mut self, from: usize, to: usize, &mut Context<Self>)`

Use unique row IDs. Subscribe to the complete new order.

Event: `OrderChanged`.

## Motion

Crossing a slot shifts neighbors before release. Drop commits once; reversing or leaving restores the slot.

Desktop and web use the same GPUI view.
