# Breadcrumb

Follow the hierarchy of locations back to an ancestor.

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
kui add breadcrumb
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::breadcrumb::Breadcrumb;

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
