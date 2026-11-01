# Card

Present related media, descriptive text and independent content actions.

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
kui add card
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::card::Card;

let control = cx.new(|_| Card::new("Lobby lounge", "A quiet space by the window.").with_byline("Updated today"));
```

## Interaction

The title opens content. Save toggles the bookmark independently.

## API

`with_cover` · `impl Into<ImageSource>`

The title opens content. Save toggles the bookmark independently.

Event: `CardEvent`.

## Motion

Hover lifts the surface and eases the cover zoom. Opening expands from the measured card bounds. Escape or the close action retraces the expansion.

Desktop and web use the same GPUI view.
