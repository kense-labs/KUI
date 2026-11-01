# User menu

An account entry point with destinations, a theme preference and asynchronous sign-out.

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
kui add user-menu
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::user_menu::{Account, UserMenu};
use kui::primitives::menu::MenuItem;

let control = cx.new(|cx| UserMenu::new("account",
    Account::new("Emma Collins", "emma@example.com"),
    vec![MenuItem::new("profile", "Profile")], cx).show_name());
```

Handle Navigate, Appearance, Presence and SignOut in the host. After sign-out resolves call finish_sign_out; false leaves the menu ready to retry.

## Interaction

Enter opens the account menu. Up and Down move between rows; Left and Right change the active preference without closing. Escape dismisses. The account panel opens from the avatar. On small screens the same commands rise in a bottom sheet.

- Event: `AccountEvent`.
- Preview: [English](/en/kui/components/user-menu/) · [中文](/kui/components/user-menu/).
