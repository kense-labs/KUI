# Live visitors

Show an active visitor count, a map of recent arrivals, the busiest pages and a conversion ticker. Use it for a realtime view. Use a time-series chart for historical trends.

## Installation

```sh
kui add live-visitors
```

## Usage

```rust
use gpui::prelude::*;
use kui::components::live_visitors::{
    LivePage, LiveVisitors, LiveVisitorsPaused, LiveVisitorsSnapshot,
};

let control = cx.new(|cx| {
    LiveVisitors::new(cx)
        .with_snapshot(LiveVisitorsSnapshot {
            count: 128,
            pages: vec![LivePage::new("/pricing", 31)],
            ..Default::default()
        })
        .with_paused(false)
});
cx.subscribe(&control, |control, event: &LiveVisitorsPaused, cx| {
    control.update(cx, |control, cx| control.set_paused(event.0, cx));
}).detach();
```

Desktop and Web use the same control. The app owns the analytics subscription. Pass a complete snapshot to `set_snapshot` when new data arrives. In the subscription above, the app can also pause or resume its own source after confirming the new state. The preview uses sample data and makes no network requests.

## API

| Input or method | Default | Behavior |
| --- | --- | --- |
| `LiveVisitors::new(&App)` | Local simulation | Starts at 128 visitors. Use `with_snapshot` to supply host data from the first frame. |
| `with_snapshot(LiveVisitorsSnapshot)` | Simulation when omitted | Sets the initial host snapshot and disables the sample feed. The snapshot contains `count: u64`, `pages: Vec<LivePage>`, `visitors: Vec<LiveVisitorPing>` and `events: Vec<LiveVisitorEvent>`. Its own default is zero and three empty lists. |
| `set_snapshot(LiveVisitorsSnapshot, &mut Context<Self>)` | No initial event | Switches to host data and replaces the snapshot. While paused or disconnected, retains the last display and keeps the most recent pending snapshot. |
| `snapshot() -> &LiveVisitorsSnapshot` | Current displayed data | Returns the normalized snapshot, including rows outside the display limits. |
| `LivePage::new(path, visitors: u64)` | Required fields | Sorts by visitor count descending. For duplicate paths the highest count wins. Bars scale against the top page. |
| `LiveVisitorPing::new(id, LiveLocation)` | Arrival | Stable ID and finite latitude and longitude in degrees. The same ID moves without replaying its ring. Removed IDs fade out. |
| `LiveLocation::new(lat: f32, lon: f32)` | Required coordinates | Invalid or nonfinite map points are ignored. Valid bounds are ±90° latitude and ±180° longitude. |
| `LiveVisitorPing::with_kind(LivePingKind)` | `Arrival` | Use `Event` for a larger dot and ring. |
| `LiveVisitorEvent::new(id, label, city, country, at: web_time::SystemTime)` | Required fields | Use an ISO alpha-2 country and a stable ID. Events are sorted newest first. Duplicate IDs keep the newest. Keep personal data out of the ticker. |
| `LiveVisitorEvent::with_location(LiveLocation)` | None | The newest conversion may show a city label on the map for 2 seconds. |
| `LiveVisitorEvent::with_flag(Arc<Image>)` | None | Overrides the flag. US, GB, DE, FR, NL, SE, CH, JP, IN and BR have built-in artwork. Other country codes use a neutral swatch when no image is provided. |
| `with_connection(LiveConnection)` and `set_connection(LiveConnection, &mut Context<Self>)` | `Live` | `Connecting` shows placeholders within the same layout. `Reconnecting` preserves the last display. Resuming `Live` applies the pending snapshot. |
| `with_paused(bool)` | Uncontrolled | Switches to controlled mode. Confirm a button request with `set_paused`. |
| `with_default_paused(bool)` | `false` | Sets the initial pause state in uncontrolled mode. |
| `set_paused(bool, &mut Context<Self>)` and `is_paused() -> bool` | `false` | Applies or reads pause state. Applying state directly emits no extra button event. |
| `LiveVisitorsPaused(bool)` | No initial event | Emitted after an uncontrolled toggle, or as a controlled request. |
| `with_hide_pause(bool)` | `false` | Removes the pause button and its tab stop. |
| `with_label(Fn(u64) -> SharedString)` | Localized “people on the site now” | Formats the words beside the count. |
| `with_limits(max_pages: usize, max_events: usize)` | `4, 4` | Sets the maximum visible row count for each list. A zero limit hides that list. |
| `with_title(impl Into<SharedString>)` | Localized “Live visitors” | Names the accessible region. |
| `with_id(impl Into<ElementId>)` and `with_style(Fn(Div) -> Div)` | `live-visitors` and no override | Sets GPUI identity and refines outer layout. The card fills available width up to 640px. |
| `with_visibility(Fn(&App) -> bool)` | Local GPUI viewport | An embedding host may additionally suspend timers and animation when the viewport is hidden. |
| `LiveVisitorsSimulation::new(SystemTime)` | Active, 128 visitors | Optional deterministic feed for prototypes. `set_active` gates updates. `advance()` advances one 600ms step. `snapshot(UiLanguage)` reads it and `now()` gives its sample time. |

## Interaction and motion

- Tab reaches the pause button. Enter or Space toggles it. The map and lists are read-only.
- Pause freezes the last displayed snapshot, including the event age clock. Incoming snapshots are held until playback and the connection are both active. Count changes use a static accessible name, with no live announcements per digit.
- Decimal columns hold a fixed advance. A new leading column grows in. Digits, ranks and bars retarget over 280ms, with 160ms entry and exit opacity for rows. Status text exits in 100ms and enters in 160ms.
- A new arrival draws one 900ms ripple. Removed dots fade out in 400ms. The live dot pulses every 1.6 seconds while the card is visible and running.
- Reduced motion updates the same data with settled numbers, bars and rows, and no rings or pulse.
- At 520px of inner card width or less the lists stack, with 16px header and map padding. Below 360px of outer width the count label moves below the number. List paths and places truncate while the right-aligned values stay visible.
- The static dotted map is derived from public-domain Natural Earth 1:110m land, version 5.1.2. Its geometry is cached by bounds. No network requests are required to draw the card.
