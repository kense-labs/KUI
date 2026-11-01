# TreeView

Navigate nested items using their visible parent and child relationships.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Library examples use `kui::` imports. Call `kui::init(cx)` before opening windows.
See the [platform guide](https://github.com/kense-labs/KUI/blob/main/docs/PLATFORMS.md)
for features and host setup. Never enable `test-support` on wasm32.

To own this component's source instead:

```bash
cargo install --git https://github.com/kense-labs/KUI --locked kui-cli
kui init
kui add tree-view
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::tree_view::{TreeView, TreeNode};

let control = cx.new(|cx| TreeView::new(vec![TreeNode::new("src","src", vec![TreeNode::new("main","main.rs",vec![])])], cx).expanded(vec!["src".into()]));
```

## Interaction

Up/Down traverse visible rows. Left folds or selects the parent. Right unfolds or enters a branch.

## API

`expanded` · `(self, Vec<SharedString>) → Self`

Each node has a unique ID, a label and its children. Events carry the ID.

Event: `TreeSelected`.

## Motion

Branch height opens together. Labels and icons enter from 6px left at 35ms intervals (seven-step cap). New delay is cancelled on exit, and visible rows retain position on reversal.

Desktop and web use the same GPUI view.
