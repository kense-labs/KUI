# LogStream

Explore multi-source logs with live search, severity chips, tail follow, and structured details.

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
kui add log-stream
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::log_stream::{LogEntry, LogLevel, LogStream};

let control = cx.new(|cx| {
    LogStream::new(
        "logs",
        "Live logs",
        vec![LogEntry::new("1", "10:00:01", LogLevel::Info, "gateway", "ready")],
        window,
        cx,
    )
});
```

## Interaction

Filter entries by source from the left directory, toggle severity levels (`All`, `ERROR`, `WARN`, `INFO`, `DEBUG`), search live keywords or tags, toggle tail follow with the capsule Switch, and click any log row to inspect its structured fields.

## API

`new` · `(id, title, Vec<LogEntry>, &mut Window, &mut Context<Self>) → Self`

`with_sources`, `set_entries`, `push_entry`, `set_sources`, `set_level_filter`, and `set_source_filter` control the stream.

Event: `LogEntrySelected`.

Desktop and web use the same GPUI view.
