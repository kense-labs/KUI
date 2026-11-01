# Dropdown menu

Keep a short list of project commands beside a labelled trigger.

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
kui add dropdown-menu
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::dropdown_menu::DropdownMenu;
use kui::primitives::menu::MenuItem;

let control = cx.new(|cx| DropdownMenu::new("actions", "Project actions",
    vec![MenuItem::new("edit", "Edit project")], cx));
```

Construct with a label and MenuItem list; subscribe to MenuSelection for the stable command ID.

## Interaction

Enter, Space or Down opens. Arrows wrap through enabled commands; Home, End and initial letters navigate. Escape returns focus. A short stagger reveals the rows. One highlight glides between pointer targets and lands directly on keyboard targets.

- Event: `MenuSelection`.
- Preview: [English](/en/kui/components/dropdown-menu/) · [中文](/kui/components/dropdown-menu/).
