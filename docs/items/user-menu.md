# User menu

An account entry point with destinations, a theme preference and asynchronous sign-out.

`kui add user-menu`

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::user_menu::{Account, UserMenu};
use crate::ui::primitives::menu::MenuItem;

let control = cx.new(|cx| UserMenu::new("account",
    Account::new("Emma Collins", "emma@example.com"),
    vec![MenuItem::new("profile", "Profile")], cx).show_name());
```

Handle Navigate, Appearance, Presence and SignOut in the host. After sign-out resolves call finish_sign_out; false leaves the menu ready to retry.

## Interaction

Enter opens the account menu. Up and Down move between rows; Left and Right change the active preference without closing. Escape dismisses. The account panel opens from the avatar. On small screens the same commands rise in a bottom sheet.

- Event: `AccountEvent`.
- Preview: [English](/en/kui/components/user-menu/) · [中文](/kui/components/user-menu/).
