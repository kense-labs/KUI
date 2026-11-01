# CodeBlock

Read a labeled code excerpt and copy its source.

## Install

```bash
kui add code-block
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::code_block::CodeBlock;

let control = cx.new(|cx| CodeBlock::new("main.rs","Rust","fn main() {}",cx));
```

## Interaction

Copy the complete source with the header action.

## API

`new` · `(filename, language, source, &mut App) → Self`

The file name and language label are independent of the source text.

Event: `—`.

## Motion

Copy uses the host clipboard. Optional max_lines springs the source height and turns its chevron; short sources stay open.

Desktop and web use the same GPUI view.
