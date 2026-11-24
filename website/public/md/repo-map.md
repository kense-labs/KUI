# Repo map

Render a codebase as a proportional folder-and-file treemap with live coding agents — active edits fill tiles from the bottom up in the agent's color with a sweeping sheen, read-only inspections draw a dashed perimeter, two agents editing one file blink orange, and clicking any agent isolates its touched files.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add repo-map`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::repo_map::{
    RepoAction, RepoAgent, RepoFile, RepoMap, RepoMapEvent,
};

let map = cx.new(|cx| {
    RepoMap::new("workspace-map", cx)
        .with_title("acme/web")
        .with_branch("feat/token-refresh")
        .with_files(vec![
            RepoFile::new("src/lib/auth.ts", 320).with_edit("auth", 64, 12),
            RepoFile::new("src/lib/session.ts", 240),
            RepoFile::new("tests/auth.test.ts", 230).with_edit("test", 58, 3),
        ])
        .with_agents(vec![
            RepoAgent::new("auth", "Auth agent").with_color_idx(0).with_target("src/lib/session.ts", RepoAction::Edit),
            RepoAgent::new("ui", "UI agent").with_color_idx(1).with_target("src/lib/session.ts", RepoAction::Edit),
            RepoAgent::new("docs", "Docs agent").with_color_idx(4).with_target("tests/auth.test.ts", RepoAction::Read),
        ])
});
cx.subscribe(&map, |_, ev: &RepoMapEvent, _| {
    let _ = ev;
}).detach();
```

## Interaction

Hovering any file tile (or walking tiles with `ArrowLeft`, `ArrowRight`, `ArrowUp`, `ArrowDown`, `Home`, or `End`) opens a floating file inspector showing full path, LOC, active agent tasks, diff stats, and concurrent write warnings. Clicking any agent pill in the top bar isolates its edited and occupied files while fading untouched regions (`Escape` clears). Clicking `Step agents` advances agents onto their next target files.

## API

Primary GPUI type: `RepoMap` (`kui::components::repo_map::RepoMap`).
Supporting types: `RepoFile`, `RepoEdit`, `RepoAgent`, `RepoAction`.
Event type: `RepoMapEvent` (`AgentIsolated`, `FileSelected`, `AgentsStepped`).

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, and `with_empty(true)` empty states.

## Motion

Agent pixel avatars glide smoothly across file tiles via `pro_spring_f32` when hopping, newly entered edit tiles fill from the bottom up with a sweeping sheen, and concurrent edit conflicts pulse orange; `Theme::reduced_motion` settles positions and fills immediately without blinking.

Desktop and web use the same GPUI source.
