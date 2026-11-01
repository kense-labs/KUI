# Context menu

Right-click a project to open its commands at the pointer, or tap the project to open below it.

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
kui add context-menu
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::context_menu::ContextMenu;
use kui::primitives::menu::MenuItem;

let control = cx.new(|cx| ContextMenu::new("project", "Open actions",
    vec![MenuItem::new("edit", "Edit project")],
    |_, _| gpui::div().child("Quarterly launch").into_any_element(), cx));
```

Provide a target renderer and MenuItem list. Pointer placement and keyboard navigation share the same selection event.

## Interaction

Right-click, tap, or press Shift+F10 on the target. Use arrows, Home and End to navigate; Enter selects, Escape dismisses. The panel opens at the pointer and stays inside the viewport. Destructive commands retain a distinct highlight.

- Event: `MenuSelection`.
- Preview: [English](/en/kui/components/context-menu/) · [中文](/kui/components/context-menu/).
