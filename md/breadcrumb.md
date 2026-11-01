# Breadcrumb

Follow the hierarchy of locations back to an ancestor.

## Install

```bash
kui add breadcrumb
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::breadcrumb::Breadcrumb;

let control = cx.new(|_| Breadcrumb::new(vec!["Projects".into(), "Launch".into(), "Files".into()]).limit(3));
```

## Interaction

Select an ancestor. When the trail exceeds its limit, open the ellipsis.

## API

`navigate` · `(&mut self, usize, &mut Context<Self>)`

Pass ordered labels, configure the visible limit and subscribe to the new index.

Event: `Navigated`.

## Motion

An added crumb enters from 8px left. A leaving crumb stays until its opacity and offset settle. Initial navigation is immediate.

Desktop and web use the same GPUI view.
