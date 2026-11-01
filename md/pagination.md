# Pagination

Move through a bounded sequence of result pages.

## Install

```bash
kui add pagination
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::pagination::Pagination;

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
