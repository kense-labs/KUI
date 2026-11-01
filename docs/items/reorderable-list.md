# ReorderableList

Change list order with a pointer or keyboard.

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
kui add reorderable-list
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::reorderable_list::{ReorderableList, OrderedItem};

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
