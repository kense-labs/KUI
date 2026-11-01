# CodeBlock

Read a labeled code excerpt and copy its source.

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
kui add code-block
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::code_block::CodeBlock;

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
