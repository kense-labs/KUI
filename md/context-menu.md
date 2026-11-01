# Context menu

Right-click a project to open its commands at the pointer, or tap the project to open below it.

`kui add context-menu`

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::context_menu::ContextMenu;
use crate::ui::primitives::menu::MenuItem;

let control = cx.new(|cx| ContextMenu::new("project", "Open actions",
    vec![MenuItem::new("edit", "Edit project")],
    |_, _| gpui::div().child("Quarterly launch").into_any_element(), cx));
```

Provide a target renderer and MenuItem list. Pointer placement and keyboard navigation share the same selection event.

## Interaction

Right-click, tap, or press Shift+F10 on the target. Use arrows, Home and End to navigate; Enter selects, Escape dismisses. The panel opens at the pointer and stays inside the viewport. Destructive commands retain a distinct highlight.

- Event: `MenuSelection`.
- Preview: [English](/en/kui/components/context-menu/) · [中文](/kui/components/context-menu/).
