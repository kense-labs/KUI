# Agent ledger

Audit every external system change an agent made—with target app tiles, `before -> after` diffs, irreversible locks, and dependency-aware `Undo` that previews downstream consequences before rolling back.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add agent-ledger`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::agent_ledger::{AgentLedger, AgentLedgerEvent};

let ledger = cx.new(|cx| {
    AgentLedger::new("launch-ledger", cx)
        .with_agent("Launch agent")
});
cx.subscribe(&ledger, |_, ev: &AgentLedgerEvent, _| {
    let _ = ev;
}).detach();
```

## Interaction

Hover `Undo` on any row (`Changed price`, `Merged PR #482`, `Deployed kense.dev`) to preview its transitive dependency chain: reversible downstream mutations highlight in blue (`Also undone`), while irreversible actions such as sent emails highlight in amber (`Can't undo`). Click `Undo` to roll back the chain and strike through reverted rows; click `Redo` to restore a reverted row along with its undone prerequisites.

## API

Primary GPUI type: `AgentLedger` (`kui::components::agent_ledger::AgentLedger`).
Event type: `AgentLedgerEvent`.

Supports `LedgerEntry` (`with_change`, `with_detail`, `with_reversible`, `with_depends_on`), `with_point_at` walkthrough highlights, light/dark `Theme`, bilingual `UiLanguage`, `with_loading(true)`, and `with_empty(true)`.

## Motion

Undoing a mutation chain sweeps a spring-animated strikethrough line (`pro_spring_f32`) across reverted rows while dimming their app tiles; hovering `Undo` transitions dependent rows into `Also undone` and `Can't undo` preview states without shifting row heights. Respects `Theme::reduced_motion`.

Desktop and web use the same GPUI source.
