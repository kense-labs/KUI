# Decision tree

Lay out an agent's operational or architectural dilemma as an interactive branching tree with `Agent's pick` highlights, consequence flags, nested follow-up choices, and an inline `"Other…"` path.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add decision-tree`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::decision_tree::{DecisionTree, DecisionTreeEvent};

let tree = cx.new(|cx| {
    DecisionTree::new("incident-decision-tree", cx)
});
cx.subscribe(&tree, |_, event: &DecisionTreeEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Hover any branch to light its connector path and dim competing branches; click a branch (or nested sub-choice) or press `1`–`3` to confirm it. Hover any amber-flagged consequence span or press `F` to read the agent's risk note and telemetry source, press `4` or click `Other…` to write a custom path instruction (`Enter` / `⌘↵`), and press `←` / `→` (`J` / `K`) to step through queued trees.

## API

Primary GPUI type: `DecisionTree` (`kui::components::decision_tree::DecisionTree`).
Event type: `DecisionTreeEvent`.

Supports `TreeDecision`, `DecisionBranch`, `DecisionSubOption`, `BranchConsequence` (plain or flagged with source notes), responsive top-to-bottom rail layout below `640px`, light/dark `Theme` modes, and bilingual `UiLanguage`.

## Motion

`pro_connector_pulse` (`PathBuilder`) paints rounded orthogonal S-bend curves with traveling light pulses along `Agent's pick` and hovered/confirmed paths, `pro_step_transition` glides between queued decisions, and `Theme::reduced_motion` disables continuous pulses.

Desktop and web use the same GPUI source.
