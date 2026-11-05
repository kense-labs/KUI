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

The host owns the data source and should set visibility when the chart leaves its viewport; non-finite values are ignored.

## API

`push` · `(StreamReading, &mut Context<Self>)`

Use set_visible and set_paused to control rendering; an optional anomaly label identifies unusual readings.

Event: `LiveInspect`.

## Motion

New observations slide into a rescaling chart; hover freezes the display for inspection. Reduced motion updates directly.

Desktop and web use the same GPUI view.
