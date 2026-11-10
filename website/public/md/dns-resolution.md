# DNS resolution

An illustrative DNS lookup across a browser, resolver, root server, TLD server and authoritative nameserver. It replays the connection, cache hit and TTL expiry locally.

## Install

Initialize KUI with `kui::init(cx)`. Use `cargo add kense-kui --rename kui --no-default-features` or run `kui init` and `kui add dns-resolution`. With the CLI, change the import prefix to `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::dns_resolution::{DnsResolution, DnsResolutionOptions};

let options = DnsResolutionOptions {
    domain: "app.acme.dev".into(),
    address: "203.0.113.24".into(),
    ..Default::default()
};
let control = cx.new(|cx| DnsResolution::new(options, cx));
```

## API

| Option | Type | Default | Behavior |
| --- | --- | --- | --- |
| `domain` | `SharedString` | `app.acme.dev` | Supplies the query and derives the displayed suffix and default nameserver from the last two labels. |
| `address` | `SharedString` | `203.0.113.24` | Address shown on the answer and cached record. |
| `ttl` | `u32` | `300` | Simulated TTL in seconds. Its visible countdown is compressed into the scene timeline. |
| `lookup_ms`, `cached_ms` | `u32` | `84`, `1` | Displayed lookup latency and cache hit latency in milliseconds. |
| `nodes` | `[DnsNode; 5]` | Built-in names and metadata | Override a node with `with_node(DnsNodeKind, DnsNode)`. The authoritative node's metadata becomes the referred nameserver. |
| `speed` | `f32` | `1.0` | Cycle multiplier. One cycle at normal speed is 16 seconds. Zero holds the current frame. |
| `paused` | `bool` | `false` | Starting paused shows the cached state. Pausing during playback holds the current timeline. |
| `accent` | `Option<Hsla>` | KUI theme accent | Colors the active paths, record marker, ring and scrubber. |
| `controls`, `caption` | `bool` | `true`, `true` | Independently show playback controls and narration. |
| `label` | `Option<SharedString>` | Derived from the data | Stable accessible description of the figure. |

`DnsNode` supports optional `label`, `meta`, `detail`, KUI `icon`, `color` and a custom GPUI `mark` factory. Use `with_id` and `with_style` for root identity and GPUI styling. Use `set_speed`, `set_paused`, `seek` (zero-based), and `set_visible` to control playback from a host. Use `with_visibility` to integrate the host’s viewport or tab visibility.

## Interaction and motion

Click one of the six scrubber buttons to seek without changing the playback state. Pause before seeking to inspect a stage. Use Left, Right, Home or End from a focused stage. Play resumes the loop from the inspected position. Hovering a node exposes its detail and eases the clock to a slower pace. The width threshold for the landscape geometry is 640 logical pixels. With reduced motion, playback is stopped and the scrubber shows settled phases. Desktop and Web draw the same GPUI view.

The component models the lookup and makes no network requests.
