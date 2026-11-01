# Card

Present related media, descriptive text and independent content actions.

## Install

```bash
kui add card
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::card::Card;

let control = cx.new(|_| Card::new("Lobby lounge", "A quiet space by the window.").with_byline("Updated today"));
```

## Interaction

The title opens content. Save toggles the bookmark independently.

## API

`with_cover` · `impl Into<ImageSource>`

The title opens content. Save toggles the bookmark independently.

Event: `CardEvent`.

## Motion

Hover lifts the surface and eases the cover zoom. Opening expands from the measured card bounds. Escape or the close action retraces the expansion.

Desktop and web use the same GPUI view.
