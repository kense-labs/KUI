# Agent overlap map

Spot where teammates' AI coding agents collide—connecting left and right agent chips to severity-coded conflict badges (`Needs a decision`, `Heads-up`, `Minor`) with smooth Bezier curves while listing non-colliding teammates as in sync.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add agent-overlap-map`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::agent_overlap_map::{AgentOverlapMap, AgentOverlapMapEvent};

let map = cx.new(|cx| AgentOverlapMap::new("agent-overlap-map", cx));
cx.subscribe(&map, |_, event: &AgentOverlapMapEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Hover any conflict badge (`Same files`, `Contradicts`, `Out of sync`, `Same work`) or any teammate's agent chip to bring its Bezier curves to full saturation while dimming unrelated items. Click any conflict badge to inspect the involved teammates and click `Resolve conflict` to clear it and watch newly conflict-free teammates join `Jonas is in sync`.

## API

Primary GPUI type: `AgentOverlapMap` (`kui::components::agent_overlap_map::AgentOverlapMap`).
Event type: `AgentOverlapMapEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_title(...)`, `with_people(...)`, `with_conflicts(...)`, `resolve_conflict(...)`, `with_loading(true)`, and `with_empty(true)`.

## Motion

Agent 3×3 pixel marks twinkle diagonally while active, `High` and `Medium` collisions connect via smooth `PathBuilder` cubic Bezier curves, `Low` collisions render as crisp dashed Bezier curves, and `Theme::reduced_motion` disables continuous pixel twinkle cleanly.

Desktop and web use the same GPUI source.
