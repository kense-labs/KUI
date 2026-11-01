# KUI platforms

KUI ships **one component source tree** for two runtimes:

| Runtime | Target | Host crate | Entry |
| --- | --- | --- | --- |
| Native desktop | macOS, Windows, Linux | `kui-story --features native` | `gpui_platform::application()` |
| WASM web | `wasm32-unknown-unknown` | `kui-web` | `gpui_platform::web_init()` + `application()` |
| Headless CI | host | `kui-story` (default) | `gpui::TestApp` |

GPUI pins stay exact: `gpui-pre` / `gpui-pre-platform` / `gpui-pre-web` at `=0.3.10`.

## Same source rule

- Controls live under `registry/` (package `kense-kui`, Rust library `kui`).
- Story UI lives in `apps/story/src/`; the gesture and menu examples share `family_story/collections.rs`.
- Desktop and web hosts choose the platform bootstrap and provide system services; controls draw the same chrome on both.

## Native desktop

```bash
cargo run -p kui-story --features native
```

Pass a registry ID after `--` to open a single example. On macOS, the default build needs the Xcode Metal compiler. If only Command Line Tools are installed, GPUI can compile the shaders at runtime:

```bash
cargo run -p kui-story --no-default-features --features native,gpui_platform/runtime_shaders -- swipe-actions
```

## WASM web

```bash
./scripts/build-web.sh
python3 -m http.server -d apps/web/www 8080
```

Open `http://127.0.0.1:8080/`. The page loads `pkg/kui_web.js` produced by `wasm-bindgen`.

Library check without a full bindgen pass:

```bash
cargo check -p kense-kui --target wasm32-unknown-unknown --no-default-features
cargo check -p kui-web --target wasm32-unknown-unknown
```

## Feature matrix

| Crate | `test-support` | `native` | `web` |
| --- | --- | --- | --- |
| `kense-kui` (lib `kui`) | default host smoke/CI; **off** for applications and wasm32 | optional platform hooks | semantic HTML controls |
| `kui-story` | default smoke/CI | desktop window | — |
| `kui-web` | never | — | wasm host |

## Platform constraints for controls

- Prefer GPUI APIs available on both native and web.
- Gate host-only APIs with `cfg(not(target_family = "wasm"))` (filesystem pickers, OS speech, etc.).
- Do not rely on `std::time::Instant` alone when web-time matters; follow GPUI’s timing helpers.
- Keep AccessKit roles; web maps what the GPUI web backend supports.
- Motion and `Theme.reduced_motion` apply on both hosts.

## App authors

Install the published component library with a stable `kui` dependency name:

```bash
cargo add kense-kui --rename kui --no-default-features
```

```toml
kui = { package = "kense-kui", version = "0.1.0", default-features = false }
```

This dependency works for native and WASM applications. `native` enables optional
GPUI platform hooks; `web` exposes KUI's semantic HTML backend. The shared GPUI
controls compile without either feature. `test-support` is enabled by default
for host smoke tests, so use `--no-default-features` in application dependencies
and never enable it on wasm32.

Alternatively, install the CLI with
`cargo install --git https://github.com/kense-labs/KUI --locked kui-cli`, then run
`kui init` and `kui add <id>` to own selected source. Only the component library is
published to crates.io. Copied source uses `crate::ui::` imports in place of `kui::`.

Both installation paths are host-agnostic. The application Cargo.toml chooses the host:

```toml
# Desktop
gpui = { package = "gpui-pre", version = "=0.3.10", default-features = false }
gpui_platform = { package = "gpui-pre-platform", version = "=0.3.10", default-features = false, features = ["font-kit"] }

# WASM cdylib also depends on gpui_platform (wasm cfg) + wasm-bindgen
```

Call `kui::init(cx)` once after the host `application()` starts, before opening
windows. With copied source, call `crate::ui::foundation::init(cx)` instead.

Embedded action previews take their language from the documentation route. Hosts may set
`kui::primitives::control::UiLanguage` as a GPUI global; English remains the default.
The web host bundles a small OFL-licensed Noto Sans SC subset named KUI Story Sans
for Chinese and Latin labels. Regenerate it with `scripts/subset-story-font.py` and
the upstream variable TTF after changing the preview vocabulary. The web build checks the shipped font against the Chinese characters used in the Rust stories and controls.
