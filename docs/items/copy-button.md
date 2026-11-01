# Copy button

Copy a link or an invite code using the system clipboard.

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
kui add copy-button
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::copy_button::CopyButton;

let control = cx.new(|_| CopyButton::new("invite", "KUI-4821", "Copy invite code").icon_only());
```

Mount the entity as a GPUI child. Copy feedback follows the platform write result. A denied write shows failure and permits a retry.

After a successful write, the old word fades out in 100ms, the new word enters in 160ms, and the check draws from 40–260ms. Feedback resets after 1.9s.

## Interface

The control uses the theme’s `radius.md` (8px by default).

- Event: `CopyResult(bool)`.
- API: `icon_only()`.
- Implementation: `registry/src/components/copy_button.rs`.
- Preview: [English](/en/kui/components/copy-button/) · [中文](/kui/components/copy-button/).
