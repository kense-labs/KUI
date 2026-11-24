# Context compaction

Visualize an agent's context window filling up across `System`, `Tools`, `User`, `Assistant`, `Tool results`, and `Files` slices, with a dashed auto-compaction mark and a `"Compact now"` action that shimmers and folds older turns into a single `"Compacted summary"` row.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add context-compaction`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::context_compaction::{ContextCompaction, ContextCompactionEvent};

let compaction = cx.new(|cx| {
    ContextCompaction::new("cart-context", cx)
        .with_title("Fix the cart total")
        .with_model("Claude Sonnet")
        .with_limit(200_000)
        .with_threshold(0.80)
});
cx.subscribe(&compaction, |_, ev: &ContextCompactionEvent, _| {
    let _ = ev;
}).detach();
```

## Interaction

- Hover any slice in the top segmented bar or move through the ordered rows below (`ArrowUp` / `ArrowDown` / `Home` / `End`) to cross-highlight matching slices and entries.
- Click `"Compact now"` to shimmer (`pro_shimmer_text`) older conversational and tool-output entries, squeeze their bar slices via `Slide` springs into a green `"Compacted summary"` slice, and roll token totals down.
- Click the `"14 tools"` row or `"Compacted summary"` row (`Enter` / `Space`) to inspect connected app definitions or archived entries and their original token counts.

## API

Primary GPUI type: `ContextCompaction` (`kui::components::context_compaction::ContextCompaction`).
Supporting types: `ContextEntry`, `ContextEntryKind`, `format_tokens_compact`.
Event type: `ContextCompactionEvent` (`CompactStarted`, `Compacted { summary_id, before_tokens, after_tokens, count }`, `TurnAppended`, `Reset`).

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, and `with_empty(true)` empty states.

## Motion

`pro_shimmer_text` sweeps across entries undergoing compaction before `pro_spring_f32` (`Slide`) squeezes retired bar slices into the summary slice and `pro_rolling_number` rolls token counts and usage percentage down; `Theme::reduced_motion` applies compaction immediately without shimmer loops.

Desktop and web use the same GPUI source.
