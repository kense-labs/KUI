# Permission request

Ask for one specific agent scope—who asks, target app, reason, and concrete impact—with `Allow once`, `Allow for 1 hour`, `Always allow`, or `Deny`, folding into a draining status line with `Revoke`.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add permission-request`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::permission_request::{
    PermissionRequest, PermissionRequestEvent, PermissionRisk, PermissionScope,
};

let request = cx.new(|cx| {
    PermissionRequest::new("coder-push-request", cx)
        .with_action("push to main")
        .with_target("acme/web", "github")
        .with_risk(PermissionRisk::Risky)
        .with_scope(PermissionScope::Timed)
});
cx.subscribe(&request, |_, event: &PermissionRequestEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Switch between `Once`, `1 hour`, and `Always` using clicks or `Left` / `Right` arrow keys—the primary button updates immediately (`Allow once`, `Allow for 1 hour`, `Always allow`). Press `Enter` to grant or `Escape` to deny; granting folds the card smoothly into a compact status row with a live remaining timer, a `2px` bottom draining bar, and a `Revoke` button that unfolds the request again. `PermissionRisk::Destructive` automatically removes `Always` and styles `Allow` in danger red.

## API

Primary GPUI type: `PermissionRequest` (`kui::components::permission_request::PermissionRequest`).
Event type: `PermissionRequestEvent`.

Supports `PermissionScope` (`Once`, `Timed`, `Always`), `PermissionRisk` (`Safe`, `Risky`, `Destructive`), `pro_agent_avatar` + `pro_app_tile`, light/dark `Theme` modes, and bilingual `UiLanguage`.

## Motion

`pro_pill_indicator` glides across the scope options, `pro_fold` transitions smoothly between the full scope card and the `58px` status line, and the timed grant progress bar drains continuously while respecting `Theme::reduced_motion`.

Desktop and web use the same GPUI source.
