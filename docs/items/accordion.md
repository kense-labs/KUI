# Accordion

Expand sections in a compact stack of disclosures.

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
kui add accordion
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::accordion::{Accordion, AccordionItem};

let control = cx.new(|cx| Accordion::new(vec![AccordionItem::new("Included", "All workspace members can review the draft.")], Some(0), cx));
```

## Interaction

Click a heading to toggle. Arrow keys, Home and End navigate headers.

## API

`multiple` · `(self, bool) → Self`

The selected initial index is optional. Event data contains index and expanded.

Event: `AccordionChanged`.

## Motion

The measured panel height, content opacity, 6px content offset and chevron follow the disclosure. A reversal keeps the current height.

Desktop and web use the same GPUI view.
