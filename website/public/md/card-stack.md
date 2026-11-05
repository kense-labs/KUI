# Card stack

Review a sequence of cards and choose or skip each.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add card-stack`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::card_stack::{CardStack, StackCard};

let cards = vec![StackCard::new("1", "Kyoto", "Morning walk")];
let control = cx.new(|cx| CardStack::new("Destinations", cards, cx));
```

## Interaction

Drag a card or use left and right arrow keys; Backspace undoes the last decision.

## API

`decide` · `(Decision, &mut Context<Self>)`

StackDecision includes the card ID and decision. Undo emits StackUndo so the host can reverse its own change.

Event: `StackDecision, StackUndo`.

## Motion

The decided card exits to the side; an uncommitted drag springs back. Reduced motion updates immediately.

Desktop and web use the same GPUI view.
