# Accordion

Expand sections in a compact stack of disclosures.

## Install

```bash
kui add accordion
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::accordion::{Accordion, AccordionItem};

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
