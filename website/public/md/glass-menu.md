# Glass menu

Show contextual actions on a translucent menu.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add glass-menu`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::glass_menu::{GlassMenu, GlassMenuGroup, GlassMenuItem};

let items = vec![GlassMenuGroup::new(vec![GlassMenuItem::new("copy", "Copy"), GlassMenuItem::new("move", "Move to").with_children(vec![GlassMenuItem::new("archive", "Archive")])])];
let control = cx.new(|cx| GlassMenu::new("photo-actions", "Photo actions", items, cx));
```

## Interaction

Use arrows, Home and End to move. Right or Enter opens one submenu; Left returns. Escape closes and restores trigger focus.

## API

`set_checked` · `(&str, bool, &mut Context<Self>)`

Provide groups of actions with stable IDs. GlassMenuSelected reports the ID and a proposed checked value; use set_checked to reflect host state. The surface is translucent; true background blur is not available.

Event: `GlassMenuSelected`.

## Motion

The panel springs from the trigger while submenu rows slide in. Reduced motion removes movement.

Desktop and web use the same GPUI view.
