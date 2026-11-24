# Autonomy dial

Configure how much an agent may do without asking via a three-stop dial while real workspace actions glide between `"Runs on its own"` and `"Asks you first"`.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add autonomy-dial`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::autonomy_dial::{AutonomyDial, AutonomyDialEvent, AutonomyLevel};

let dial = cx.new(|cx| {
    AutonomyDial::new("sales-agent-autonomy", cx)
        .with_title("Sales agent")
        .with_level(AutonomyLevel::IfRisky)
});
cx.subscribe(&dial, |_, event: &AutonomyDialEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Select `Always ask`, `If risky`, or `Full auto` (or press `Left` / `Right` arrow keys) to watch actions smoothly move between `Runs on its own` and `Asks you first`. Hover any action row and click its padlock to force it to always ask regardless of autonomy level; actions marked `.with_red_line(true)` stay permanently locked.

## API

Primary GPUI type: `AutonomyDial` (`kui::components::autonomy_dial::AutonomyDial`).
Event type: `AutonomyDialEvent`.
Helper function: `asks_first(&AutonomyAction, AutonomyLevel, &HashSet<SharedString>) -> bool`.

Supports `AutonomyActionRisk` (`Safe`, `Risky`, `Destructive`), multicolor app logos (`pro_app_artwork`), light/dark `Theme` modes, and bilingual `UiLanguage`.

## Motion

`pro_pill_indicator` glides the raised selection pill across the three levels, `pro_fold` and directional `PhaseClock` slides animate rows crossing between groups, and `pro_rolling_number` updates group counts while respecting `Theme::reduced_motion`.

Desktop and web use the same GPUI source.
