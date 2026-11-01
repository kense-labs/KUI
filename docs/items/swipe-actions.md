# Swipe actions

Triage the inbox by swiping a message or choosing the same command from its menu.

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
kui add swipe-actions
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::swipe_actions::{SwipeAction, SwipeActions, SwipeRow};
use kui::primitives::glyph::Glyph;

let row = SwipeRow::new("draft", "Draft", |_, _| gpui::div().child("Draft").into_any_element())
    .leading(vec![SwipeAction::new("archive", "Archive", Glyph::Archive)]);
let control = cx.new(|cx| SwipeActions::new("inbox", "Inbox", vec![row], cx));
```

Supply stable row and action IDs. Removing actions collapse the row; keep_row actions leave it visible. Restore by row ID.

## Interaction

Drag horizontally to reveal. Swipe farther to commit. Tab to More actions; use arrows and Enter. Undo restores the message in place. The front follows the drag. Release projects velocity into a resting stop or full commit; dismissal slides out and closes the gap.

- Event: `SwipeSelection`.
- Preview: [English](/en/kui/components/swipe-actions/) · [中文](/kui/components/swipe-actions/).
