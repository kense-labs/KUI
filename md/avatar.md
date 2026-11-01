# Avatar

Identify one person with a portrait or an initials fallback.

## Install

```bash
kui add avatar
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::avatar::{Avatar, Person};

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
