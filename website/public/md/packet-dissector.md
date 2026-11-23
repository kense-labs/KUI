# PacketDissector

Inspect captured frames with protocol layer trees, redaction markers, and hex/ASCII dumps.

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
kui add packet-dissector
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::packet_dissector::{FrameDirection, PacketDissector, PacketFrame};

let control = cx.new(|_| {
    PacketDissector::new(
        "frames",
        "Protocol frames",
        vec![PacketFrame::new("1", 1, FrameDirection::Outbound, "HTTP", "GET /health")],
    )
});
```

## Interaction

Filter frames by protocol or error state, then select any row to inspect decoded protocol layers, redacted field warnings, and the 16-byte hex/ASCII payload dump.

## API

`new` · `(id, title, Vec<PacketFrame>) → Self`

`set_frames`, `push_frame`, `select_frame`, `set_protocol_filter`, and `set_errors_only` update captured data and active filters. `PacketDissector::format_hex_dump` renders canonical 16-byte hex/ASCII rows.

Event: `FrameSelected`.

Desktop and web use the same GPUI view.
