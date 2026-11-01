# Dialog

Review and confirm an interrupting decision in a modal surface.

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
kui add dialog
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::dialog::Dialog;

let control = cx.new(|cx| Dialog::new("review", "Review changes", "Publish workspace", cx)
        .with_description("Your team will see these changes immediately.")
        .with_cancel("Keep editing").with_confirm("Publish now"));
```

Subscribe to Opened, Closed and Confirmed. with_body accepts a custom view factory.

## Interaction

Open with Enter. Tab cycles through the dialog. Escape dismisses and restores trigger focus. The backdrop fades while the panel rises 8px. Closing reverses the current spring.

- Event: `SurfaceEvent`.
- Preview: [English](/en/kui/components/dialog/) · [中文](/kui/components/dialog/).
