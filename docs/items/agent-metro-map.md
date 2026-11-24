# Agent metro map

Draw a multi-agent workflow as a transit metro map—with straight rails and crisp 45° bends, model-logo legend chips, twinkling agent train chips, interchange hubs, and amber approval stations.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add agent-metro-map`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::agent_metro_map::{AgentMetroMap, AgentMetroMapEvent};

let metro = cx.new(|cx| AgentMetroMap::new("agent-metro-map", cx));
cx.subscribe(&metro, |_, event: &AgentMetroMapEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Hover any line chip in the top legend or any station ring on the map to bring its route forward and dim unrelated lines. Click `Step trains` to glide active agent trains along the 45° rails to their next stops, click any legend chip to inspect its ordered stop list, or click the amber `Sign-off` station to `Approve` or `Reject` final rollout.

## API

Primary GPUI type: `AgentMetroMap` (`kui::components::agent_metro_map::AgentMetroMap`).
Event type: `AgentMetroMapEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_title(...)`, `with_stations(...)`, `with_lines(...)`, `advance_trains(...)`, `resolve_station(...)`, `with_loading(true)`, and `with_empty(true)`.

## Motion

Active agents ride their rails as elevated train chips with diagonal twinkling 3×3 pixel marks, filling the rail solid behind them while sending a short traveling light pulse ahead. Trains glide between stops via `Slide` springs and `Sign-off` breathes softly in amber; `Theme::reduced_motion` jumps trains cleanly without continuous animation.

Desktop and web use the same GPUI source.
