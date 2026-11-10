# Realtime stream

Monitor a live series and pause it to inspect individual readings.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add realtime-stream`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::realtime_stream::RealtimeStream;

let control = cx.new(|_| RealtimeStream::new("Requests").with_capacity(60).with_unit(" req/s"));
```

## Interaction

The host owns the data source and should set visibility when the chart leaves its viewport. Non-finite values are ignored. Focusing the plot holds the newest visible point. Arrow keys inspect readings, Page Up and Page Down move ten readings, and Home and End go to the visible endpoints. Space or Enter toggles a pinned pause. Escape clears inspection and returns to live when the pointer leaves the plot.

On a narrow card, the pause button sits beside the title so the reading and unit have their own row.

## API

`with_subtitle` adds supporting text below the title. `with_series_color` sets the stroke and area tint. `with_timing(window_ms, interval_ms)` sets the window and interval for evenly spaced samples, adds relative time labels to the axis and limits the retained buffer to 2–2000 samples. `with_capacity` remains available for a point-count window.

`push` · `(StreamReading, &mut Context<Self>)`

Use set_visible and set_paused to control rendering; an optional anomaly label identifies unusual readings.

Event: `LiveInspect`.

## Motion

New observations slide into a rescaling chart. Hover freezes the display for inspection. The unit follows the animated width of the rolling digits. Reduced motion updates directly.

Desktop and web use the same GPUI view.
