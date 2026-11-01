# WorkspacePicker

Find and activate a team without leaving the current workspace.

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
kui add team-switcher
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::team_switcher::{WorkspacePicker, Workspace};

let control = cx.new(|cx| WorkspacePicker::new("team",vec![Workspace{id:"main".into(),name:"Product".into(),plan:"Team".into(),members:12,role:"Admin".into()}],window,cx));
```

## Interaction

The list starts with a search row whose underline gains contrast on focus. Use the arrows or Cmd/Ctrl+1–9 to choose a workspace. Create and Join have separate inputs. Escape returns to the list, clears a filter, then closes and restores trigger focus.

The list shows up to six 48px rows, with a shorter viewport in small windows. Longer lists scroll below the fixed search field and above the fixed actions. The highlighted row stays visible during keyboard navigation. With an empty query, Home and End jump to the ends of the menu. In the embedded preview, the next outward wheel event or new outward swipe at the boundary scrolls the page. A swipe that starts within the list stays there until release.

## API

`new` · `(id, Vec<Workspace>, window, cx)`

Workspace includes ID, plan, member count and role. Handle switch, create and join events separately.

Event: `WorkspaceEvent`.

## Motion

List, Create and Join share a panel up to 360px wide. The panel, form fields and row highlights share an 8px corner radius. On an internal route change the incoming content fades in over 160ms and travels 6px in the navigation direction. Reduced motion omits the travel. Closing retains outgoing rows, and switching workspaces crossfades the trigger name.

Desktop and web use the same GPUI view.
