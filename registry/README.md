# KUI

KUI is a Rust UI kit for GPUI on native desktop and WASM web, from Kense.
It provides foundation tokens, themes, motion, and 108 components from one source tree.

## Install

```sh
cargo add kense-kui --rename kui --no-default-features
```

The crates.io package is `kense-kui`; the Rust library and dependency name are `kui`.

```toml
[dependencies]
kui = { package = "kense-kui", version = "0.1.0", default-features = false }
gpui = { package = "gpui-pre", version = "=0.3.10", default-features = false }
```

Pin GPUI to `=0.3.10`. Call `kui::init(cx)` once after your GPUI application starts,
before opening windows. Your application supplies its native or WASM host.

```rust
use kui::components::button::Button;

let button = Button::new("save").primary().label("Save");
// In a GPUI render method, apply the current theme with button.into_themed(cx).
```

## Features

| Feature | Purpose |
| --- | --- |
| `test-support` (default) | Host smoke tests using GPUI test support. Disable for application builds and WASM. |
| `native` | Optional GPUI platform hooks for native desktop hosts. |
| `web` | Semantic HTML document controls; the shared GPUI controls also compile without it. |

For WASM, keep `default-features = false`. Never enable `test-support` on wasm32.
See the [platform guide](https://github.com/kense-labs/KUI/blob/main/docs/PLATFORMS.md)
for native and WASM host setup.

## Own the component source

The CLI can instead copy selected components into your application's `ui/` tree:

```sh
cargo install --git https://github.com/kense-labs/KUI --locked kui-cli
kui init
kui add button
```

CLI-installed controls use `crate::ui::components::button::Button` rather than
`kui::components::button::Button`. The CLI and story gallery use the same component source
as this crate. Only the component library is published to crates.io.

Browse the [component documentation](https://kense-labs.github.io/kui/docs/),
[Rust API](https://docs.rs/kense-kui), and [repository](https://github.com/kense-labs/KUI).

## License

MIT. Copyright (c) 2026 Kense.
