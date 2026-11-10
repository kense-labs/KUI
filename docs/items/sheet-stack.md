# Sheet stack

Navigate a short stack of related sheets.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add sheet-stack`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::sheet_stack::{SheetStack, SheetPage, SheetLink};

let pages = vec![SheetPage::new("settings", "Settings", "Manage preferences").with_links(vec![SheetLink::new("privacy", "Privacy")]), SheetPage::new("privacy", "Privacy", "Review privacy settings")];
let control = cx.new(|cx| SheetStack::new("Open settings", pages, cx));
```

## Interaction

Open pages with push; pop returns one level and Escape closes the top layer.

## API

`push` · `(&str, &mut Window, &mut Context<Self>) -> bool`

SheetStackChanged carries the currently open page IDs. Existing IDs return to that page.

`with_launcher_context` places a heading or profile above the launch button. `set_presentation` accepts `SheetStackPresentation::{Auto, Sheet, Dialog}`. Auto uses the measured container width for a contained stack, or the window width otherwise, and opens a dialog at 640px and wider. Presentation changes preserve the open page IDs.

Event: `SheetStackChanged`.

## Motion

The top panel follows dragging; background layers move with depth. Reduced motion removes depth movement.

Desktop and web use the same GPUI view.
