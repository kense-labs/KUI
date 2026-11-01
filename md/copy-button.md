# Copy button

Copy a link or an invite code using the system clipboard.

`kui add copy-button`

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::copy_button::CopyButton;

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
