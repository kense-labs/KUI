# Reactions

Add a reaction to a release note, change your vote, or pick another symbol.

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
kui add reactions
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::reactions::{Reaction, ReactionOption, Reactions};
use kui::primitives::glyph::Glyph;

let control = cx.new(|cx| Reactions::new("votes",
    vec![ReactionOption::new("like", "Like", Glyph::Like)],
    vec![Reaction::new("like", 3, false)], cx));
```

Supply picker options and initial counts. ReactionChanged supplies the changed ID, count and personal selection.

## Interaction

Enter or Space toggles a count. From Add reaction, use arrows, Home or End; Enter chooses and Escape returns to the add button. Digits roll up or down with the count. New pills expand, and the picker highlight follows its active option. A thin ring marks your own reactions. The moving highlight only indicates the current option; its caption has a check when you have already reacted.

- Event: `ReactionChanged`.
- Preview: [English](/en/kui/components/reactions/) · [中文](/kui/components/reactions/).
