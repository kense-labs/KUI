# ScrollArea

Scroll an overflowing local region independently of the page.

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
kui add scroll-area
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::scroll_area::ScrollArea;

let control = cx.new(|_| ScrollArea::new(80., Box::new(|_, _| gpui::div().flex().flex_col().children((1..20).map(|n| gpui::div().child(n.to_string()))).into_any_element())));
```

## Interaction

Scroll with the wheel, Page Up/Down, Home or End.

## API

`handle` · `(&self) → ScrollHandle`

Supply a fixed viewport height and a content factory. horizontal() changes the overflow axis.

Event: `ScrollHandle`.

## Motion

The thumb fades in during scroll or hover, waits 900ms after leaving and fades out. Pressing the track pages 90%; dragging moves the thumb.

Desktop and web use the same GPUI view.
