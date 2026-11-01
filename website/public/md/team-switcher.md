# WorkspacePicker

Find and activate a team without leaving the current workspace.

## Install

```bash
kui add team-switcher
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::team_switcher::{WorkspacePicker, Workspace};

let control = cx.new(|cx| WorkspacePicker::new("team",vec![Workspace{id:"main".into(),name:"Product".into(),plan:"Team".into(),members:12,role:"Admin".into()}],window,cx));
```

## Interaction

The list starts with a search row whose underline gains contrast on focus. Use the arrows or Cmd/Ctrl+1–9 to choose a workspace. Create and Join have separate inputs. Escape returns to the list, clears a filter, then closes and restores trigger focus.

The list holds six 48px rows. Longer lists scroll below the fixed search field and above the fixed actions. The highlighted row stays visible during keyboard navigation. With an empty query, Home and End jump to the ends of the menu. The embedded preview returns scrolling to the page at the list boundary.

## API

`new` · `(id, Vec<Workspace>, window, cx)`

Workspace includes ID, plan, member count and role. Handle switch, create and join events separately.

Event: `WorkspaceEvent`.

## Motion

List, Create and Join share a panel up to 360px wide. The panel, form fields and row highlights share an 8px corner radius. On an internal route change the incoming content fades in over 160ms and travels 6px in the navigation direction. Reduced motion omits the travel. Closing retains outgoing rows, and switching workspaces crossfades the trigger name.

Desktop and web use the same GPUI view.
