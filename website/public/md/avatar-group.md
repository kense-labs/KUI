# AvatarGroup

Show a compact group of people and reveal the remaining members.

## Install

```bash
kui add avatar-group
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::avatar_group::AvatarGroup;
use crate::ui::components::avatar::Person;

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
