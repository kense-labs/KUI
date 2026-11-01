# Dropdown menu

Keep a short list of project commands beside a labelled trigger.

`kui add dropdown-menu`

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::dropdown_menu::DropdownMenu;
use crate::ui::primitives::menu::MenuItem;

let control = cx.new(|cx| DropdownMenu::new("actions", "Project actions",
    vec![MenuItem::new("edit", "Edit project")], cx));
```

Construct with a label and MenuItem list; subscribe to MenuSelection for the stable command ID.

## Interaction

Enter, Space or Down opens. Arrows wrap through enabled commands; Home, End and initial letters navigate. Escape returns focus. A short stagger reveals the rows. One highlight glides between pointer targets and lands directly on keyboard targets.

- Event: `MenuSelection`.
- Preview: [English](/en/kui/components/dropdown-menu/) · [中文](/kui/components/dropdown-menu/).
