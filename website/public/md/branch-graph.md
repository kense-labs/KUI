# Branch graph

Inspect a short commit history and see how merge, rebase, or squash would change it. The preview is computed from the commits supplied by the host.

## Installation

Initialize KUI with `kui::init(cx)`. Use `cargo add kense-kui --rename kui --no-default-features` or run `kui init` and `kui add branch-graph`. With the CLI, change the import prefix to `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::branch_graph::{BranchGraph, BranchGraphOptions, BranchStrategy};

let options = BranchGraphOptions {
    default_strategy: BranchStrategy::Open,
    autoplay: false,
    ..Default::default()
};
let control = cx.new(|cx| BranchGraph::new(options, cx));
```

## API

| Option | Type | Default | Effect |
| --- | --- | --- | --- |
| `commits` | `Vec<BranchCommit>` | Eight checkout example commits | Newest first. Each has stable `id`, `hash`, `message`, optional `body`, `branch`, `parents`, `author`, a display `date`, and optional `additions`, `deletions`, `files`. A parent outside this slice is omitted from drawing. |
| `branches` | `Vec<BranchLane>` | `main`, `feat/checkout`, `fix/rounding` | Draws lanes in order. The first is the merge destination. Each lane may override its color. |
| `pull_request` | `BranchPullRequest` | #482, Checkout redesign, feat/checkout | Identifies the branch and names the new commit. |
| `strategy` | `Option<BranchStrategy>` | `None` | When present, selection is host-controlled. Subscribe to `BranchStrategyChange` and apply it through `set_strategy`. |
| `default_strategy` | `BranchStrategy` | `Open` | Initial uncontrolled view. |
| `strategies` | `Vec<BranchStrategy>` | Open, Merge, Rebase, Squash | Sets the order of buttons. One or zero hides the selector. |
| `title` | `Option<SharedString>` | Pull request title | An empty string hides the heading. |
| `autoplay` | `bool` | `true` | Cycles Open, Rebase, Merge, Squash when uncontrolled. The first deliberate interaction stops autoplay. |
| `speed` | `f32` | `1.0` | Multiplies playback speed. Zero selects states instantly. Finite values are clamped to 0–32. |
| `paused` | `bool` | `false` | Pauses autoplay while manual picks still animate. |
| `accent` | `Option<Hsla>` | First series color, with KUI accent if set | Base lane and hash rewrite accent. A lane-level color takes precedence. |
| `label` | `Option<SharedString>` | Built from the heading and lanes | Accessible name of the graph. |

`BranchAuthor` has a display `name` and an optional GPUI image URI. Use `with_id` and `with_style` for root identity and styles and `with_visibility` to integrate the host’s viewport visibility. `set_paused` changes autoplay at runtime. `BranchStrategyChange` is emitted for a user request and does not perform Git operations. Preview hashes are stable, synthetic seven-character values.

## Interaction and layout

Use the strategy buttons, arrows or Home and End. Tab into the commit list and use Up and Down, Home or End. Escape closes details. Hover a commit to inspect it, or tap to pin and tap away to dismiss. When the graph is narrower than 720px, 560px or 400px, the date, avatar and hash respectively move out of the row but remain available in details. Rows are 44px high. The reduced-motion setting disables entrance, flights and autoplay. Desktop and Web share the GPUI implementation.
