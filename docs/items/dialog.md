# Dialog

Review and confirm an interrupting decision in a modal surface.

`kui add dialog`

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::dialog::Dialog;

let control = cx.new(|cx| Dialog::new("review", "Review changes", "Publish workspace", cx)
        .with_description("Your team will see these changes immediately.")
        .with_cancel("Keep editing").with_confirm("Publish now"));
```

Subscribe to Opened, Closed and Confirmed. with_body accepts a custom view factory.

## Interaction

Open with Enter. Tab cycles through the dialog. Escape dismisses and restores trigger focus. The backdrop fades while the panel rises 8px. Closing reverses the current spring.

- Event: `SurfaceEvent`.
- Preview: [English](/en/kui/components/dialog/) · [中文](/kui/components/dialog/).
