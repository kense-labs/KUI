# DetailCard

Open a summary card to inspect its additional detail.

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
kui add expandable-card
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::expandable_card::DetailCard;

let control = cx.new(|_| DetailCard::new("Launch checklist","4/6 complete", Box::new(|_, _| gpui::div().child("Invite reviewers").into_any_element())));
```

## Interaction

Activate the summary to reveal or collapse details in place.

## API

`set_expanded` · `(&mut self, bool, &mut Context<Self>)`

Pass the title, summary and a body builder. Subscribe to the expanded state.

Event: `DetailsChanged`.

## Motion

Width and measured height change on a shared spring. Details keep the open width and fade in after expansion starts; the arrow rotates.

Desktop and web use the same GPUI view.
