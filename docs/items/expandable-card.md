# DetailCard

Open a summary card to inspect its additional detail.

## Install

```bash
kui add expandable-card
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::expandable_card::DetailCard;

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
