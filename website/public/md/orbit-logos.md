# Orbit logos

Browse integrations on two moving rings, filter by category, and open a card with connection actions.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To install source, run `kui init` and `kui add orbit-logos`, then replace `kui::` with `crate::ui::` in the example.

## Usage

```rust
use gpui::prelude::*;
use kui::components::orbit_logos::{OrbitLogo, OrbitLogos};

let logos = vec![
    OrbitLogo::new("issues", "Issues")
        .with_category("Development")
        .with_description("Sync issue updates with your workspace."),
    OrbitLogo::new("messages", "Messages")
        .with_category("Communication")
        .with_description("Send status updates to your team.")
        .with_connected(true),
];
let orbit = cx.new(|cx| OrbitLogos::new(logos, cx).with_host_connections());
```

## Interaction

- Select a category to bring its integrations toward the card and dim other logos. Select All to restore the full list.
- Click or tap an integration to open its details. Select it again or use Close to dismiss it. Previous and Next move within the current category.
- Tab passes through the filter chips, one logo and the card. Use the arrow keys, Home and End to move between eligible logos. Press Enter or Space to toggle a focused logo and Escape to close the card and return focus.
- Below 760px, the card moves under the orbit and the selected logo parks at the bottom. The category strip scrolls horizontally when needed.

## API

| Rust API | Behavior and defaults |
| --- | --- |
| `OrbitLogo::new(id, name)` | Supply unique, stable IDs. Selection also accepts a matching name when no ID matches. |
| `with_path(path, color)` | Paint 24×24 SVG path data. `None` follows light and dark ink. |
| `with_image(source)`, `with_mark(render)` | Provide an image source or a GPUI render function. The default shows the first letter. |
| `with_category(category)`, `with_description(text)` | Opt into a filter and supply card text. Both are optional. |
| `with_connected(bool)`, `with_docs(url)` | The default is disconnected. An HTTP or HTTPS URL adds the Docs link. |
| `OrbitLogos::new(logos, cx)` | Starts with All and no card. |
| `with_center(render)`, `with_id(id)`, `with_style(style)` | Render the center disc content or customize the root layout and identity. |
| `with_label(label)`, `with_all_label(label)` | Defaults to Integrations and All in English, with localized Chinese defaults. |
| `with_speed(f32)`, `with_paused(bool)`, `set_paused(bool, cx)` | Speed defaults to 1.0. Zero or a paused state stops ambient rotation. |
| `with_default_selected(id)` | Open an initial card. The default is none. |
| `with_selected(Option<&str>)`, `set_selected(Option<&str>, cx)` | Control selection from the host. User actions emit `OrbitSelectionChanged` with the ID and zero-based index. Closing uses `None` for both fields. |
| `with_host_connections()` | Keep requests pending until the host responds. The default simulates a short successful connection for examples. |
| `OrbitConnectRequested { id, index, ticket }` | One request per connection attempt. Use `finish_connect(ticket, success, cx)` when the operation settles. Failure exposes Try again, and stale tickets return false. |
| `set_visible(bool, cx)` | Let an embedding host suspend animation when it hides the view. |

## Motion

The rings turn in opposite directions with periods of 64s and 88s at default speed. Pointer entry and focus slow the ambient rotation to rest. Filter and detail actions park a ring on its shortest path. The card surface grows from the logo and the connector extends from the center. Reduced motion settles the layout immediately and stops ambient turns.
