# Collaboration stream

Watch what collaborating agents say to each other—every message shows `From -> To` avatars, `Handoff` and `Question for you` badges, clickable file preview chips, collapsible structured payload cards, and an `@agent` composer.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add collaboration-stream`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::collaboration_stream::{
    CollaborationStream, CollaborationStreamEvent,
};

let stream = cx.new(|cx| {
    CollaborationStream::new("pricing-collab-stream", cx)
        .with_task("Launch the new pricing page")
        .with_live(true)
});
cx.subscribe(&stream, |_, ev: &CollaborationStreamEvent, _| {
    let _ = ev;
}).detach();
```

## Interaction

Filter by participant (`Everyone`, `You`, `Orchestrator`, `Researcher`, `Writer`) at the top. Click any file chip (`pricing-table.csv`, `sources.json`, `copy-draft.md`) to open its inline preview modal, toggle `Details` on a message to expand or collapse its structured payload card (`pro_fold`), and click `Answer` / `Reply` or type `@` in the `Editor` composer to direct a message to specific agents.

## API

Primary GPUI type: `CollaborationStream` (`kui::components::collaboration_stream::CollaborationStream`).
Event type: `CollaborationStreamEvent`.

Supports `StreamMessageKind` (`Ask`, `Reply`, `Handoff`, `Share`, `Question`, `Spawn`), `StreamAttachment` (`table`, `code`, `document`), `StreamPayload`, `Editor` `@agent` mentions, light/dark `Theme`, bilingual `UiLanguage`, `with_loading(true)`, and `with_empty(true)`.

## Motion

Active writers twinkle their 3x3 `pro_agent_avatar` pixel marks while streaming text shimmers via `pro_shimmer_text`, filter tabs glide via `pro_pill_indicator`, payload cards unfold via `pro_fold`, and file preview dialogs settle smoothly via `pro_presence`. Respects `Theme::reduced_motion`.

Desktop and web use the same GPUI source.
