# Commit trail

Render a Git branch commit feed on a vertical blue trail with tinted checks, colored change-kind tags, backtick file/branch chips, foldable merge curves, live `"Committing"` shimmer, and expandable commit details.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add commit-trail`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::commit_trail::{CommitTrail, CommitTrailEvent};

let history = cx.new(|cx| {
    CommitTrail::new("main-commits", cx)
        .with_branches(["main", "invoice-pdf", "tax-rates"])
        .with_has_more(true)
});
cx.subscribe(&history, |_, ev: &CommitTrailEvent, _| {
    let _ = ev;
}).detach();
```

## Interaction

Click the header branch name (`main`) to switch branches via an animated popover menu. Click any merge commit to fold or unfold the commits it brought in along a blue `PathBuilder` side S-curve. Click any regular or nested commit to open its `pro_fold` detail drawer showing human/agent avatars (`pro_person_avatar` / `pro_agent_avatar`), short hash with one-click clipboard copy, modified file count, and `+added` / `−removed` line totals. Hovering any backtick-wrapped file (`queue.ts`) or branch (`invoice-pdf`) chip displays its path or summary pill.

## API

Primary GPUI type: `CommitTrail` (`kui::components::commit_trail::CommitTrail`).
Supporting types: `TrailCommit`, `CommitAuthor`, `CommitKind`.
Event type: `CommitTrailEvent` (`BranchChanged(SharedString)`, `OpenCommitChanged(Option<SharedString>)`, `ToggleMerge { commit_id, expanded }`, `CopiedHash(SharedString)`).

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_max_height(...)`, `with_loading(true)` skeleton rows, and `with_empty(true)` empty states.

## Motion

While a commit is `pending`, its marker arc spins, `"Committing"` shimmers, and an upward fading blue light pulse runs from the newest check into the spinning ring; `pro_fold` smoothly expands merge curves and commit details while `Theme::reduced_motion` freezes continuous animations cleanly.

Desktop and web use the same GPUI source.
