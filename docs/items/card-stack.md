# Card stack

Review a sequence of cards and choose or skip each, with a throw, an undo, and a redeal.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add card-stack`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::card_stack::{CardStack, StackCard, StackFact};
use kui::primitives::glyph::Glyph;

let cards = vec![StackCard::new("kyo", "Kyoto", "Japan")
    .with_meta("KIX", "3–9 Nov")
    .with_pitch("Temple mornings, ryokan evenings.")
    .with_fact(StackFact::new(Glyph::Plane, "15h 40m with one stop"))];
let control = cx.new(|cx| CardStack::new("Offsite destinations", cards, cx));
```

## Interaction

Drag the top card; a flick over 500 px/s or a pull past the threshold throws it. `keep` and `pass` stamps fade in from 16px to 96px of travel. Left and right arrow keys decide, Backspace and Command/Ctrl+Z undo, and the empty state offers a redeal. Vertical travel resists past a 48px free zone and is never a decision.

## API

`decide` · `(Decision, &mut Context<Self>)` — throw the top card as chosen or skipped.

`undo` · `(&mut Context<Self>)` — fly the last card home from where it left.

`reset` · `(&mut Context<Self>)` — deal every card back, deepest first.

`index` / `message` — current position and the polite status line.

Labels: `with_labels(skip, choose, undo)`; empty state: `with_empty(title, none_note, action)`.

Events: `StackDecision`, `StackUndo`, `StackReset`.

## Motion

Four cards render: each card behind peeks 12px lower and 5% smaller (27.2px down per step) and darkens with depth; the fourth waits invisible and fades in as it rises. While the top card is dragged, the cards behind rise up to 40% of a step. Decisions retarget every depth spring over 0.35s while the thrown card flies out over 0.5s and fades in 0.22s; undo returns from the recorded exit point. Reduced motion updates immediately.

Desktop and web use the same GPUI view. The reference tilts cards as they drag; GPUI cannot rotate elements, so KUI tracks the same travel without the rotation.
