# Timeline

Read chronological updates grouped by date.

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
kui add timeline
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::timeline::{Timeline, TimelineEntry};

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
