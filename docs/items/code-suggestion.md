# PatchReview

Review a proposed edit and accept or reject its changes.

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
kui add code-suggestion
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::code_suggestion::{PatchReview, Hunk, Decision};

let control = cx.new(|cx| PatchReview::new("patch","release.rs",vec![Hunk {line:4,before:vec!["let n = 1;".into()],after:vec!["let n = 2;".into()],decision:Decision::Pending}],cx));
```

## Interaction

Accept or reject each pending hunk once. Cmd/Ctrl+Enter accepts remaining hunks.

## API

Use `with_model(name)` to show the host-selected model. The review aligns line numbers and change markers independently from code.

`new` · `(id, file, Vec<Hunk>, cx)`

Each Hunk declares a starting line, before/after lines and a Decision. PatchEvent lists only newly decided indices.

Event: `PatchEvent`.

## Motion

Removed and new lines have separate clipping groups. Accept or reject folds the superseded lines over 260ms and new lines arrive in 38ms steps.

Desktop and web use the same GPUI view.
