# Reasoning tree

Illustrate a beam search over host-supplied thoughts.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add reasoning-tree`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::reasoning_tree::{ReasoningTree, Thought};

let thoughts = vec![Thought::new("first", "Reduce scope").score(0.8)];
let control = cx.new(|cx| ReasoningTree::new("Choose a plan", thoughts, cx));
```

## Interaction

The host supplies unique stable IDs, thoughts and illustrative scores; no model is queried. Invalid or missing scores use a deterministic fallback.

## API

`set_thoughts` · `(Vec<Thought>, &mut Context<Self>)`

Use set_beam_width (1–3), set_playing, step and restart. set_error reports a host failure; Retry asks the host to recover. Arrow keys navigate and expand the visible tree.

Event: `ReasoningTreeEvent`.

## Motion

Stages fade in with score growth; wide containers show connected tree nodes and narrow containers use an outline. Continuous-motion and input verification remain pending.

Desktop and web use the same GPUI view.
