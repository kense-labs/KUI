# Agent cursors

Render a multiplayer document and table stage where collaborating AI agents glide across prose, resize hero figures, patch inline code, and leave margin comments like teammates.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add agent-cursors`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::agent_cursors::{AgentCursors, AgentCursorsEvent};

let stage = cx.new(|cx| {
    AgentCursors::new("launch-brief-cursors", cx)
        .with_title("Product / Launches")
});
cx.subscribe(&stage, |_, ev: &AgentCursorsEvent, _| {
    let _ = ev;
}).detach();
```

## Interaction

- Click or focus any agent badge in the header (`Enter` / `Space`) to follow that agent and dim unfollowed pointers and selections.
- Click any underlined/struck-through suggestion in prose or code to inspect the agent's note and `Accept` or `Reject` it, or click `Accept all` in the header to splice every pending edit via `accept_suggestion`.
- Click a margin comment pin to expand or collapse its discussion card, send a quick reply, or mark the thread resolved.
- Switch between `Doc` and `Table` views at the top to collaborate on structured deliverables.

## API

Primary GPUI type: `AgentCursors` (`kui::components::agent_cursors::AgentCursors`).
Supporting types: `CursorAgent`, `DocBlock`, `DocBlockKind`, `ImageFloat`, `ImageAnchor`, `LiveCursor`, `DocSuggestion`, `DocComment`, `DocTab`, `TableCell`, `CellTone`, `accept_suggestion`.
Event type: `AgentCursorsEvent` (`AcceptSuggestion`, `RejectSuggestion`, `AcceptAll`, `ResolveComment`, `ReplyComment`, `FollowChanged`, `TabChanged`, `RowAdded`).

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, and `with_empty(true)` empty states.

## Motion

`Slide` springs glide each `PathBuilder` cursor arrow and pill nametag across prose, table cells, and resizing image handles while active header badges rotate a live progress arc; `Theme::reduced_motion` snaps pointers immediately without continuous loops.

Desktop and web use the same GPUI source.
