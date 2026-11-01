# KUI

**KUI** is a **Kense** product — a Rust UI kit for [GPUI](https://gpui.rs) that runs on **native desktop** and **WASM web** from one control source. Install the [`kense-kui`](https://crates.io/crates/kense-kui) component library, or copy selected controls into a local `ui/` tree with the `kui` CLI.

This repository ships foundation tokens and 108 installable components, covering input, navigation, overlays, data visualization, collaboration and AI workflows. Browse the component families on the bilingual component site.

## Why KUI

- **Dual runtime** — macOS / Windows / Linux desktop and `wasm32-unknown-unknown` web
- **GPUI** — GPU UI, pinned `gpui-pre =0.3.10`
- **Two installation paths** — depend on the component library or own selected component source
- **Behavior vs presentation** — primitives stay unstyled; components ship finished defaults
- **Preview = install source** — story hosts depend on the same `registry/` tree

## Quick start

```bash
cargo add kense-kui --rename kui --no-default-features
```

The crates.io package is `kense-kui`; the Rust library and dependency name remain `kui`:

```rust
use kui::components::button::Button;

let button = Button::new("save").primary().label("Save");
```

Call `kui::init(cx)` once before opening windows and use `button.into_themed(cx)`
in your GPUI render method. Keep `gpui-pre` pinned to `=0.3.10`.
Default `test-support` is for host smoke tests; disable it for application builds
and WASM. The optional `native` feature provides GPUI platform hooks, and `web`
exposes semantic HTML document controls. Your app supplies its native or WASM host.
See the [crate guide](registry/README.md), [Rust API](https://docs.rs/kense-kui),
and [platform guide](docs/PLATFORMS.md).

## Develop this repository

```bash
# Check the KUI registry library (host CI; no Xcode Metal required)
cargo check -p kense-kui

# Same library for WASM
cargo check -p kense-kui --target wasm32-unknown-unknown --no-default-features

# Story smoke (headless)
cargo run -p kui-story

# Native desktop window (needs GPUI platform / Xcode Metal on macOS)
cargo run -p kui-story --features native

# WASM web gallery
./scripts/build-web.sh
python3 -m http.server -d apps/web/www 8080

# CLI
cargo install --path tools/cli
kui init
kui add button
kui doctor
```

Platform details: [`docs/PLATFORMS.md`](./docs/PLATFORMS.md).

## Website

[Kense](https://kense-labs.github.io/) is the organization home; [KUI](https://kense-labs.github.io/kui/) is its component product. Both languages, all page templates, and the website client live in this repository. The Rust generator renders indexable HTML, KUI supplies semantic document controls, and live stories use the same GPUI code as installed components. The small Rust/WASM website client handles navigation separately from the GPU runtime.

Build and serve locally:

```sh
./scripts/build-site.sh
python3 scripts/check-site.py website/dist
python3 -m http.server 4321 --bind 127.0.0.1 --directory website/dist
```

Open [the local site](http://127.0.0.1:4321/kui/). Building needs the Rust toolchain and wasm32 target, Python 3 and rsync; the build script installs the matching wasm-bindgen CLI if needed. No Node or TypeScript build is involved. On macOS, run `./scripts/check-webkit-navigation.sh` against the local server to check first navigation, canvas reuse, browser history, loading completion, Shell highlighting, and installed source file switching in the system WebKit.

## GitHub Pages

A push to `main` builds and checks the site once. The existing KUI project Pages deployment keeps old links working and hosts the release archive, its source commit and SHA-256 checksum at `/KUI/release.json`. The [organization publishing workflow](https://github.com/kense-labs/kense-labs.github.io/actions/workflows/publish.yml) checks for new releases every five minutes and deploys the verified archive to the organization root. GitHub may delay scheduled runs; **Run workflow** publishes on demand. Neither repository has credentials to write to the other. The canonical workflow for the hosting repository is in [ops/pages/publish.yml](ops/pages/publish.yml); select **GitHub Actions** as the Pages source in both repositories.

## Install a button into your app

To own the source instead of depending on the component library, install the CLI
from GitHub. The CLI is not published to crates.io.

```bash
cargo install --git https://github.com/kense-labs/KUI --locked kui-cli
kui init
kui add button
```

```rust
use crate::ui::components::button::Button;

Button::new("save").primary().label("Save")
```

The installed `ui/` source is host-agnostic. Use `crate::ui::` imports for copied
source and `kui::` imports for the published library. Your app Cargo.toml selects
desktop `gpui_platform` or a WASM cdylib host.

## Layout

```text
registry/           # authoritative source (package: kense-kui, library: kui)
apps/story/         # shared gallery + desktop/smoke host
apps/web/     # WASM web host + www/
tools/cli/          # kui init | add | doctor | sync-mod
tools/registry-build/
website/public/r/   # registry JSON
skills/kui/         # agent skills
docs/               # contracts + per-item markdown
```

## Brand

**Kense** is the company (more products will follow). **KUI** is this product. Its
crates.io package is **`kense-kui`**; the Rust library, CLI, modules, and paths use
**`kui`**. See [`website/public/assets/BRAND.md`](./website/public/assets/BRAND.md).

## Engineering

- [`AGENTS.md`](./AGENTS.md) — agent and contributor rules (Microsoft Pragmatic Rust Guidelines)
- [`DESIGN.md`](./DESIGN.md) — chrome tokens, Button metrics, interaction
- [`docs/PLATFORMS.md`](./docs/PLATFORMS.md) — native + WASM matrix
- [`docs/PUBLISHING.md`](./docs/PUBLISHING.md) — crates.io release, organization ownership, and documentation checks

## License

MIT.
