# Avatar

Identify one person with a portrait or an initials fallback.

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
kui add avatar
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::avatar::{Avatar, Person};

let control = cx.new(|_| Avatar::new(Person::new("Ari Singh"), 36.0));
```

## Interaction

Use a profile name for the accessible image label and fallback initials.

## API

`new` · `(Person, f32)`

Use a profile name for the accessible image label and fallback initials.

## Motion

An already decoded image appears directly. A newly decoded image fades in from soft focus over its initials. Presence appears with a short scale change.

Desktop and web use the same GPUI view.
