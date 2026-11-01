# ReadingOutline

Navigate article sections and track reading progress.

## Install

```bash
kui add toc-progress
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::toc_progress::{ReadingOutline, ReadingSection};

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
