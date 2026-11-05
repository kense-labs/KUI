# Switch

Toggle a setting that takes effect immediately.

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
kui add switch
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::switch::Switch;

let control = cx.new(|cx| Switch::new("digest", "Weekly digest", window, cx).with_checked(true));
```

with_checked sets the initial value. set_checked synchronizes host updates without echoing a change event.

## Interaction

Click the label or activate with Enter or Space. A disabled switch ignores input. An 18px thumb stretches by 5px on press and glides across its 42 × 24px track.

## Shape

The 42 × 24px track uses `Theme.tokens.radius.full`, giving semicircular 12px ends; the 18px thumb remains circular and becomes a capsule while stretched. This is an explicit exception to KUI's 8px rectangular surface rule (user instruction, 2026-11-05). Embedded examples, including simulated failures, network interruptions and Cookie settings, keep this default. `with_track_radius(Pixels)` remains an explicit host override; the examples do not force 8px.

- Event: `bool`.
- Preview: [English](/en/kui/components/switch/) · [中文](/kui/components/switch/).
