# Context map

Render an agent's context window as a living 2D squarified treemap where `System prompt`, `Tools`, `Memory`, `Files`, `Chat`, `Tool results`, and dashed `Free space` tiles glide smoothly via `Slide` springs as turns arrive or when `Compact` condenses older turns into a summary tile.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add context-map`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::context_map::{
    ContextBlock, ContextBlockKind, ContextMap, ContextMapEvent,
};

let map = cx.new(|cx| {
    ContextMap::new("session-treemap", cx)
        .with_blocks(
            vec![
                ContextBlock::new("sys", "System prompt", ContextBlockKind::System, 6_400),
                ContextBlock::new("tools", "Tool definitions", ContextBlockKind::Tools, 14_800),
                ContextBlock::new("cart", "src/cart/cart.ts", ContextBlockKind::File, 22_400),
                ContextBlock::new("chat", "Chat", ContextBlockKind::Chat, 41_000),
            ],
            200_000,
        )
});
cx.subscribe(&map, |_, ev: &ContextMapEvent, _| {
    let _ = ev;
}).detach();
```

## Interaction

- Hover any treemap tile to inspect its exact token count, window percentage, provenance note, and child items; click a tile to pin its inspector card (`Escape` or a second click unpins).
- Click `+ Turn` to append new file/test/chat blocks and watch all squarified tiles and the hatched/dashed `Free space` tile glide smoothly to their new bounds.
- Click `Compact` to condense `Chat` and `Tool result` blocks into an emerald `Compacted summary` tile, unlocking the `Before | After` comparison toggle in the header.
- Use the agent selector dropdown (`Checkout fixer · 200k`) to switch between agent windows.

## API

Primary GPUI type: `ContextMap` (`kui::components::context_map::ContextMap`).
Supporting types: `ContextBlock`, `ContextBlockKind`, `ContextAgentProfile`, `CompactionView`, `BlockFate`, `TreemapRect`, `compute_squarified_treemap`.
Event type: `ContextMapEvent` (`TileSelected`, `AgentChanged`, `ViewChanged`, `TurnAdded`, `Compacted`, `Reset`).

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, and `with_empty(true)` empty states.

## Motion

`Slide` springs interpolate every tile's `(x, y, w, h)` bounds across squarified treemap updates, `PathBuilder` draws the diagonal hatch and dashed inner outline on `Free space`, and `pro_rolling_number` rolls used token totals; `Theme::reduced_motion` snaps tile bounds immediately.

Desktop and web use the same GPUI source.
