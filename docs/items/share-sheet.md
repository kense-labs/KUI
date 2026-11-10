# ShareSheet

A GPUI share surface with copy, link access, channels and a recent-recipient picker.

```sh
kui add share-sheet
```

```rust
use gpui::{Task, prelude::*};
use kui::components::share_sheet::{ShareChannel, SharePerson, ShareSheet, ShareSheetAccess};

let control = cx.new(|cx| ShareSheet::new(
    "Release plan",
    "https://example.com/docs/release-plan",
    vec![SharePerson::new("riley", "Riley Morgan")],
    vec![
        ShareSheetAccess::new("team", "Workspace members").with_description("Can comment"),
        ShareSheetAccess::new("link", "Anyone with the link").with_description("Can view"),
    ],
    cx,
)
.with_channels(vec![ShareChannel::new("email", "Email").with_done_label("Sent")])
.on_send(|submission, _, _| {
    // Pass the selected people and access value to the application's share service.
    let _ = submission;
    Task::ready(Ok(()))
}));
```

Copied-source imports start with `crate::ui`. Supply stable recipient IDs, access values and channel IDs. Recipients can provide an `ImageSource` via `with_avatar`, a gradient via `with_accent` or use the initials fallback. Access and channel icons accept a KUI `Glyph` or a decorative GPUI `AnyView`.

| Option | Rust type | Default |
| --- | --- | --- |
| Title and link | `Into<SharedString>` | Required |
| Recent recipients | `Vec<SharePerson>` | Required |
| Access choices | `Vec<ShareSheetAccess>` | Required |
| `with_channels` | `Vec<ShareChannel>` | Empty |
| `with_default_access` | `&str` | First entry |
| `with_label` | `Into<SharedString>` | Share, localized |
| `with_align` | `ShareSheetAlign` | End |
| `with_sheet_on_phones` | `bool` | true at 520px and below |
| `with_width` | `f32` | 384px, clamped to available space |
| `with_style` | `Fn(Div) -> Div` | No override |
| `on_send` | `Fn(ShareSubmission, &mut Window, &mut App) -> Task<Result<(), SharedString>>` | No host handler |
| `on_channel` | `Fn(SharedString, &mut Window, &mut App) -> Task<Result<(), SharedString>>` | No host handler |
| `on_copy` | `Fn(SharedString, &mut Window, &mut App)` | No callback |

`on_send` receives a snapshot of selected people and access. Return a task that resolves when the server confirms the operation. Return an error message to retain the selections and allow retry. Channel callbacks receive the channel ID. The component copies the full supplied URL and calls `on_copy` only after a successful clipboard write. Supply callbacks to integrate a real delivery service. Undo returns to the editable panel with the selection intact.

Open with Enter or Space. Focus moves to Copy. Arrow keys wrap through access choices and Escape closes the list before closing the panel. Selecting a person toggles their chip. Escape and Done return focus to Share. A narrow window uses a dimmed, draggable bottom sheet. Reduced motion snaps geometry, skips avatar travel and the empty-selection shake, and retains a short fade.
