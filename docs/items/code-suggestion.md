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
use kui::components::code_suggestion::{Decision, Hunk, PatchReview};

let control = cx.new(|cx| {
    PatchReview::new("patch", "release.rs", vec![Hunk {
        line: 2,
        before: vec!["    let attempts = 1;".into()],
        after: vec!["    let attempts = 4;".into()],
        decision: Decision::Pending,
    }], cx).with_original("fn release() {\n    let attempts = 1;\n}\n")
});
```

## Interaction

Accept or reject each pending hunk once. Cmd/Ctrl+Enter accepts the remaining hunks, and Cmd/Ctrl+Backspace rejects them. Arrow Left and Arrow Right scroll long source lines while the code area has focus. After deciding a hunk, focus moves to the next undecided hunk or back to the source.

## API

Use `with_model(name)` to show the host-selected model. The source, diff markers and line numbers stay aligned while horizontally scrolling.

`new` · `(id, file, Vec<Hunk>, cx)`

Each `Hunk` declares a one-based source line, before and after lines, and a `Decision`. The number of `before` lines is the removal length. `PatchEvent` lists only newly decided indices.

- `with_original(source: &str)` includes unchanged lines and enables `resolved_code() -> Option<String>`. Only accepted hunks are applied to the returned text. Overlapping and out-of-range proposals are ignored in source mode.
- `with_generation_controls(max_code_height: Pixels)` exposes the Stop, Retry and Regenerate actions. Subscribe to `PatchControlEvent` to handle the corresponding host work.
- `set_phase(phase: PatchPhase, cx)` switches between `Thinking`, `Streaming`, `Ready` and `Error`. `Thinking` clears earlier proposals. Update `hunks` and notify while streaming. Review actions become available in `Ready`.

Event: `PatchEvent`.

## Motion

Streamed insertions open over 120ms per line. Review controls open with a 240ms disclosure. Accept or reject folds the superseded lines and controls over 260ms. The sample emits lines every 38ms while the preview is visible. Reduced motion keeps the source and decision states visible with immediate layout changes.

Desktop and web use the same GPUI view.
