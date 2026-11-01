# SymbolPicker

Find a symbol by category or search and insert it into a host view.

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
kui add emoji-picker
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::emoji_picker::SymbolPicker;

let control = cx.new(|cx| SymbolPicker::standard(window, cx));
```

## Interaction

Search in English or Chinese, or move between categories. Arrow keys move the current emoji. Enter inserts it. Hold Shift while choosing to keep the picker open.

## API

`standard` · `(window, cx)`

standard loads 1,870 emoji. new accepts a custom Vec<Symbol>. SymbolPicked contains the selected Unicode sequence including its tone. Save SymbolPreferences and pass it to with_preferences to restore the tone and up to 27 recent selections.

## Motion

Indicator uses measured tab positions, preview pops on active change, tone menu reverses through the same progress.

Desktop and web use the same GPUI view.
