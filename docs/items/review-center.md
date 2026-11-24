# Review center

Review what your agents want to do in one focused workspace—a left queue sorted by risk, one proposal inspected at a time with uncertain spans highlighted in amber, single-key `Approve` (`A`), inline `Edit` (`E`), `Reject` (`R`), and `Skip` (`S`), and an animated resolution stamp as each item yields to the next.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add review-center`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::review_center::{ReviewCenter, ReviewCenterEvent, ReviewFilter};

let center = cx.new(|cx| {
    ReviewCenter::new("agent-review-center", cx)
        .with_reviewed(24)
        .with_default_filter(ReviewFilter::All)
});
cx.subscribe(&center, |_, event: &ReviewCenterEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Press `A` to approve, `E` to edit the proposal inline via `Editor` (`⌘↵` saves and approves, `Esc` cancels), `R` followed by `1`–`4` to reject with a quick reason, `S` to skip, `J` / `K` (or arrow keys) to move through the queue, `F` to cycle uncertain amber spans and inspect the `Agent's note` card, and `Z` (or `⌘Z`) to undo the last decision.

## API

Primary GPUI type: `ReviewCenter` (`kui::components::review_center::ReviewCenter`).
Event type: `ReviewCenterEvent`.

Supports code diffs (`ReviewFile`), structured fields (`ReviewField`), numbered plans (`ReviewPlanStep`), and prose bodies, with full light/dark `Theme` parity, bilingual `UiLanguage`, loading skeletons, empty states, and responsive layout below `640px`.

## Motion

Filter switches glide with `pro_pill_indicator`, reviewed counts roll via `pro_rolling_number`, and deciding an item stamps `APPROVED` or `REJECTED` onto the departing card as the next item rises smoothly into place; `Theme::reduced_motion` switches immediately.

Desktop and web use the same GPUI source.
