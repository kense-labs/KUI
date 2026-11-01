# Tooltip

Reveal a short explanation on hover or keyboard focus.

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
kui add tooltip
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::tooltip::Tooltip;

let control = cx.new(|cx| Tooltip::new("Project visibility", "Workspace members only", cx));
```

## Interaction

Hover or focus the icon. Escape dismisses the hint.

## API

`new` · `(name, text, &mut App) → Tooltip`

Supply both an accessible name and a short explanation.

Event: `—`.

## Motion

Waits 250ms on pointer intent. A 3px rise and separate opacity channel preserve reversal; reduced motion retains a 90ms fade.

Desktop and web use the same GPUI view.
