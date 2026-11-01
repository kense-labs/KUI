# Pagination

Move through a bounded sequence of result pages.

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
kui add pagination
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::pagination::Pagination;

let control = cx.new(|_| Pagination::new(12, 2));
```

## Interaction

Choose a number or the adjacent navigation arrows. Pages are one-based.

## API

`select` · `(&mut self, page: usize, &mut Context<Self>)`

Set the page count and initial page. The active index stays within bounds.

Event: `PageChanged`.

## Motion

Five consecutive page numbers roll on one coordinate; the 36px controls are 4px apart. Labels retain spacing and velocity through reversal; reduced motion swaps directly.

Desktop and web use the same GPUI view.
