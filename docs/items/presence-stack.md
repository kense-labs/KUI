# Presence stack

Show who is present and open a roster of everyone.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add presence-stack`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::presence_stack::{PresencePerson, PresenceStack, PresenceStatus};

let people = vec![PresencePerson::new("me", "Alex"), PresencePerson::new("emma", "Emma").with_status(PresenceStatus::Typing)];
let control = cx.new(|cx| PresenceStack::new("people", people, cx).with_self_id("me", cx));
```

## Interaction

Hover or focus to fan the faces out; use Left or Right to inspect names. Activate the stack to open the roster, then select a person or use Escape to close.

## API

`set_people` · `(Vec<PresencePerson>, &mut Context<Self>)`

Provide stable IDs, names, optional image sources, status and activity detail. Call set_people with each host snapshot; PresenceSelected reports the selected person ID.

Event: `PresenceSelected`.

## Motion

Keyed entries and positions use interruptible springs; the overflow count rolls and typing dots animate. Reduced motion applies changes immediately.

Desktop and web use the same GPUI view.
