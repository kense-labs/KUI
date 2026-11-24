# Tool access map

Hub map placing an active run agent in the center and every run tool around it—distinguishing `Running` (pulsing beam), `Used`, `Failed` (red), `Not used yet` (idle), and `No access` (dimmed dashed line), with an inline call-history inspector where individual calls can be edited and replayed.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add tool-access-map`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::tool_access_map::{ToolAccessMap, ToolAccessMapEvent};

let map = cx.new(|cx| {
    ToolAccessMap::new("tool-access-map", cx)
        .with_default_agent("coder")
});
cx.subscribe(&map, |_, event: &ToolAccessMapEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Switch agents using the top tab strip or Left/Right arrow keys to see each agent's granted vs blocked (`No access`) tools. Click any accessible tool card to open its call-history drawer: pick any call on the left to inspect its input and output/error, edit the input payload inline, and press `Replay` (or `⌘↵`) to re-run that invocation in isolation. Press `Escape` or the back arrow to return to the hub map.

## API

Primary GPUI type: `ToolAccessMap` (`kui::components::tool_access_map::ToolAccessMap`).
Event type: `ToolAccessMapEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_title(...)`, `with_tools(...)`, `with_agents(...)`, `with_default_agent(...)`, `with_loading(true)`, and `with_empty(true)`.

## Motion

Granted tools connect to the hub card via solid blue `pro_connector_pulse` wires with a continuous glint beam while `Running` (or red wire when `Failed`), while ungranted tools fade to `0.45` opacity over dashed connectors. Switching agent tabs transitions the center card via `pro_view_transition`, and opening a tool glides the call inspector in via `pro_presence`. Respects `Theme::reduced_motion` and stacks below `540px`.

Desktop and web use the same GPUI source.
