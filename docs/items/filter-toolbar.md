# FilterToolbar

Combine and remove facet filters above a result set.

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
kui add filter-toolbar
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::filter_toolbar::{FilterToolbar, Facet};

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
