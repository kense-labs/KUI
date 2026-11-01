# ReadingOutline

Navigate article sections and track reading progress.

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
kui add toc-progress
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::toc_progress::{ReadingOutline, ReadingSection};

let control = cx.new(|_| ReadingOutline::new("Release notes",vec![ReadingSection::new("Overview","Workspace changes",0)], 4));
```

## Interaction

Scroll the article or jump to a heading. The active section and remaining minutes follow the viewport.

## API

`new` · `(title, Vec<ReadingSection>, minutes: u32) → Self`

Sections carry a heading, text and nesting depth. Events report a visible section index.

Event: `SectionChanged`.

## Motion

The reading marker tracks the fraction through the active section on a spring; jumping links scroll the article smoothly.

Desktop and web use the same GPUI view.
