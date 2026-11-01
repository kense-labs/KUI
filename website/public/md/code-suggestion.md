# PatchReview

Review a proposed edit and accept or reject its changes.

## Install

```bash
kui add code-suggestion
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::code_suggestion::{PatchReview, Hunk, Decision};

let control = cx.new(|cx| PatchReview::new("patch","release.rs",vec![Hunk {line:4,before:vec!["let n = 1;".into()],after:vec!["let n = 2;".into()],decision:Decision::Pending}],cx));
```

## Interaction

Accept or reject each pending hunk once. Cmd/Ctrl+Enter accepts remaining hunks.

## API

`new` · `(id, file, Vec<Hunk>, cx)`

Each Hunk declares a starting line, before/after lines and a Decision. PatchEvent lists only newly decided indices.

Event: `PatchEvent`.

## Motion

Removed and new lines have separate clipping groups. Accept or reject folds the superseded lines over 260ms and new lines arrive in 38ms steps.

Desktop and web use the same GPUI view.
