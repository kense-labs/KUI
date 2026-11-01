# CapacityBands

Inspect how different parts of a conversation consume its context budget.

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
kui add context-window
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::context_window::{CapacityBands, Band};

let control = cx.new(|_| CapacityBands::new("context", "Assistant", 128000, vec![Band { label: "Messages".into(), tokens: 24000, color: gpui::rgb(0x628995).into() }]));
```

## Interaction

Allocate 200 cells across ordered token categories, free space and reserved output.

## API

`used` · `(&self) -> u64`

Supply ordered Band totals, capacity, reserve and threshold. In response to CapacityAction, apply the host result. After a successful compaction call did_compact(previous_used, cx) with the pre-commit total.

Event: `CapacityAction`.

## Motion

After the host commits a smaller allocation, the first affected grid cell dims immediately. Subsequent cells start 6ms apart; over 720ms each reaches 0.35 opacity at 55%, then lands on the new allocation. A fresh compaction keeps the currently painted alpha and colour. Reduced motion settles on the first frame.

Desktop and web use the same GPUI view.
