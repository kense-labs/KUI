# AvatarGroup

Show a compact group of people and reveal the remaining members.

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
kui add avatar-group
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::avatar_group::AvatarGroup;
use kui::components::avatar::Person;

let control = cx.new(|_| AvatarGroup::new(vec![Person::new("Ari Singh"), Person::new("Emma Collins")], 1));
```

## Interaction

Set the visible member limit. Activate the overflow count to inspect the roster.

## API

`set_people` · `Vec<Person>`

Set the visible member limit. Activate the overflow count to inspect the roster.

## Motion

Hover spreads nearby portraits and raises the active one. Membership changes resize keyed slots and roll the overflow count in the direction of change.

Desktop and web use the same GPUI view.
