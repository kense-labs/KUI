# Control center

Put focus sessions, notification channels, presenting mode, alert volume and text size in one compact panel.

## Installation

```sh
kui add control-center
```

## Usage

```rust
use gpui::prelude::*;
use kui::components::control_center::{ControlCenter, ControlCenterChanged, FocusMode, NotificationChannel};
use kui::primitives::glyph::Glyph;

let settings = cx.new(|cx| {
    ControlCenter::new(
        vec![
            FocusMode::new("deep", "Deep work", Glyph::Lock, "Only mentions get through", 50),
            FocusMode::new("meeting", "Meetings", Glyph::Clock, "Keep calendar alerts", 30),
        ],
        vec![
            NotificationChannel::new("mentions", "Mentions").with_default_on(true),
            NotificationChannel::new("deploys", "Deploy alerts"),
        ],
        cx,
    )
});
cx.subscribe(&settings, |_, event: &ControlCenterChanged, _| {
    // Persist the settings and apply notification, presenting and text preferences in the host.
    let settings = &event.0;
    println!("Alert volume: {}", settings.volume);
}).detach();
```

## API

| Method or type | Default | Behavior |
| --- | --- | --- |
| `new(Vec<FocusMode>, Vec<NotificationChannel>, &mut Context<Self>)` | Required | Mode presets and channel definitions, with stable IDs. |
| `with_id(impl Into<ElementId>)` | Entity-derived | Sets root identity. |
| `with_people(Vec<FocusPerson>)` | Empty | Shows a footer with people allowed to reach the focused user. Supply name and host-supported image source. |
| `with_default_volume(u8)` | `60` | Clamps to 0–100 in steps of one. |
| `with_default_text_size(u8)` | `100` | Clamps and rounds to the 85–130 range in steps of five. |
| `with_format_time(impl Fn(DateTime<Local>) -> SharedString + 'static)` | Local 12-hour clock | Formats the prospective or committed end time. |
| `with_label(impl Into<SharedString>)` | Quick settings / 快捷设置 | Accessible name of the panel. |
| `with_style(impl Fn(Div) -> Div + 'static)` | None | GPUI refinement of the outer layout. |
| `state() -> &ControlCenterState` | Focus off, notifications on, presenting off | Reports focus, master notification state, channel preferences, presenting, volume and text size. |
| `ControlCenterChanged(ControlCenterState)` | No initial event | Emits the full snapshot on each distinct change, including meter steps and session expiry. |

`FocusMode::new` takes ID, label, glyph, description and default minutes. The duration is normalized to 5–120 minutes in steps of five. `NotificationChannel::new` takes an ID and label. A channel starts off unless `with_default_on` enables it. `with_description` adds secondary text. The master notification switch leaves individual preferences intact.

## Interaction

- Enter or Space activates the focused tile, chevron or switch. The Focus tile starts the first preset or ends the active session. The detail keeps mode and duration edits as a draft until Start or Update. End finishes the current session.
- The detail is modal inside the panel. Tab stays in the detail, and Esc, Close, or the scrim returns focus to its chevron.
- Meter drags start at the current level. A tap jumps to the touched position. Arrow keys change one step, Page Up and Page Down move four steps, and Home and End set the limits.
- The dial tracks rotation through twelve o’clock and snaps to five minutes when released. Arrow keys change five minutes, page keys change fifteen minutes, and Home and End set 5 or 120 minutes.
- The panel is 360×396px at full width. Narrow layouts tighten tile padding and use a 150px dial in place of 164px. Rectangular surfaces use the KUI surface radius. Switches, mode selection and dial remain circular or capsule-shaped. Reduced motion uses opacity changes for detail and status transitions. It snaps the geometry and removes meter stretch.
