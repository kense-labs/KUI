# Root cause tree

Fault-tree diagnostic explorer placing the failure symptom on top and branching downward through `OR` and `AND` logic gates—stepping through hypotheses one by one, fading ruled-out branches onto dashed lines, lighting the confirmed causal chain back up to the top failure, and displaying code diffs, logs, and `Fix it` / `Retry from here` actions below.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add root-cause-tree`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::root_cause_tree::{RootCauseTree, RootCauseTreeEvent};

let tree = cx.new(|cx| {
    RootCauseTree::new("root-cause-tree", cx)
        .with_live(true)
});
cx.subscribe(&tree, |_, event: &RootCauseTreeEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Watch the agent verify candidate hypotheses automatically, or click `Step check` to step manually. Click any node in the tree to inspect its verdict sentence, touched file paths, and monospace `Diff`, `Log`, or `Tool call` proof in the evidence panel below. Confirmed leaf root causes reveal `Retry from here` and `Fix it` buttons. Press `Escape` to return focus to the active investigation node.

## API

Primary GPUI type: `RootCauseTree` (`kui::components::root_cause_tree::RootCauseTree`).
Event type: `RootCauseTreeEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_title(...)`, `with_failure(...)`, `with_demo_step(...)`, `with_live(...)`, `with_loading(true)`, and `with_empty(true)`.

## Motion

Hypotheses under active verification shimmer via `pro_shimmer_text`; ruled-out branches dim in place with dashed orthogonal connectors so investigated dead ends remain visible; and once an `OR` or `AND` gate is satisfied, a blue `pro_connector_pulse` light runs upward from the confirmed leaf root cause(s) to the top failure symptom. Respects `Theme::reduced_motion` and switches to an indented list below `580px`.

Desktop and web use the same GPUI source.
