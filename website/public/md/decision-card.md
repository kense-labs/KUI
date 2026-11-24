# Decision card

Ask a blocking agent question right above the chat composer—with numbered options (`Recommended`), visual wireframe tiles, `Approve` / `Decline` facts, an inline `"Other"` input, `1`–`5` shortcuts, and `"Later"` one-line folding.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add decision-card`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::decision_card::{DecisionCard, DecisionCardEvent};

let card = cx.new(|cx| {
    DecisionCard::new("composer-decision-card", cx)
});
cx.subscribe(&card, |_, event: &DecisionCardEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Press `1`–`4` or click any row or preview tile to answer immediately; press the next number key (e.g. `5`) or `Down` past the last option to type a custom instruction in `"Other — type your own…"`. Use the header pager (`‹ 1 / 3 ›` or `Left` / `Right`) to inspect queued questions across multiple tasks, or click `Later` (`Escape`) to fold the card to a compact summary line until you are ready.

## API

Primary GPUI type: `DecisionCard` (`kui::components::decision_card::DecisionCard`).
Event type: `DecisionCardEvent`.

Supports `DecisionKind` (`Choice`, `Pick` visual wireframe tiles, `Approval` structured facts), `with_composer_chrome(bool)` for standalone embedding, light/dark `Theme` modes, and bilingual `UiLanguage`.

## Motion

`pro_step_transition` glides horizontally between queued questions, `pro_fold` collapses the card smoothly into the `"Later"` summary bar (or folds away completely once the last decision is answered), and `Theme::reduced_motion` snaps transitions cleanly.

Desktop and web use the same GPUI source.
