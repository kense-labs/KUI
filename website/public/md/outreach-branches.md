# Outreach branches

Render three rounded S-curve branches carrying pastel light pulses from a single source node into Gmail, Slack, and Notion tiles.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add outreach-branches`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::outreach_branches::{OutreachBranches, OutreachBranchesEvent};

let branches = cx.new(|cx| {
    OutreachBranches::new("hero-outreach", cx)
        .with_label("One workflow fanning out to Gmail, Slack, and Notion")
});
cx.subscribe(&branches, |_, ev: &OutreachBranchesEvent, _| {
    let _ = ev;
})
.detach();
```

## Interaction

Hover or click any destination tile to focus that branch and fire an immediate light pulse down its curve; click the top source dot to broadcast pulses down all three branches simultaneously.

## API

Primary GPUI type: `OutreachBranches` (`kui::components::outreach_branches::OutreachBranches`).
Event type: `OutreachBranchesEvent` (`kui::components::outreach_branches::OutreachBranchesEvent`).

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_channels(...)`, `with_playing(...)`, `with_focused_branch(...)`, `with_loading(true)` skeleton placeholders, and `with_empty(true)` empty states.

## Motion

Four nested gradient-tapered pulse layers travel down the left, center, and right Bezier branches on a staggered `3.4s` loop (`0s`, `0.95s`, `1.85s` offsets) and split cleanly around both halves of each destination tile perimeter as the card glows; `Theme::reduced_motion` freezes the resting pastel rim and ambient glow immediately.

Desktop and web use the same GPUI source.
