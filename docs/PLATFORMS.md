# KUI platforms

KUI ships **one component source tree** for two runtimes:

| Runtime | Target | Host crate | Entry |
| --- | --- | --- | --- |
| Native desktop | macOS, Windows, Linux | `kui-story --features native` | `gpui_platform::application()` |
| WASM web | `wasm32-unknown-unknown` | `kui-web` | `gpui_platform::web_init()` + `application()` |
| Headless CI | host | `kui-story` (default) | `gpui::TestApp` |

GPUI pins stay exact: `gpui-pre` / `gpui-pre-platform` / `gpui-pre-web` at `=0.3.10`.

## Same source rule

- Controls live under `registry/` (crate `kui`).
- Story UI lives in `apps/story/src/gallery.rs`.
- Desktop and web hosts only choose a platform bootstrap. They must not fork Button chrome.

## Native desktop

```bash
cargo run -p kui-story --features native
```

Needs a working GPUI native stack (on macOS: Xcode Metal tooling).

## WASM web

```bash
./scripts/build-web.sh
python3 -m http.server -d apps/web/www 8080
```

Open `http://127.0.0.1:8080/`. The page loads `pkg/kui_web.js` produced by `wasm-bindgen`.

Library check without a full bindgen pass:

```bash
cargo check -p kui --target wasm32-unknown-unknown --no-default-features
cargo check -p kui-web --target wasm32-unknown-unknown
```

## Feature matrix

| Crate | `test-support` | `native` | `web` |
| --- | --- | --- | --- |
| `kui` | default; **off** on wasm32 | optional | marker only |
| `kui-story` | default smoke/CI | desktop window | — |
| `kui-web` | never | — | wasm host |

## Platform constraints for controls

- Prefer GPUI APIs available on both native and web.
- Gate host-only APIs with `cfg(not(target_family = "wasm"))` (filesystem pickers, OS speech, etc.).
- Do not rely on `std::time::Instant` alone when web-time matters; follow GPUI’s timing helpers.
- Keep AccessKit roles; web maps what the GPUI web backend supports.
- Motion and `Theme.reduced_motion` apply on both hosts.

## App authors

Installed `ui/` source is host-agnostic. The application Cargo.toml chooses the host:

```toml
# Desktop
gpui = { package = "gpui-pre", version = "=0.3.10", default-features = false }
gpui_platform = { package = "gpui-pre-platform", version = "=0.3.10", default-features = false, features = ["font-kit"] }

# WASM cdylib also depends on gpui_platform (wasm cfg) + wasm-bindgen
```

Call `kui::foundation::init(cx)` once after the host `application()` starts, before opening windows.

Embedded action previews take their language from the documentation route. Hosts may set
`kui::primitives::control::UiLanguage` as a GPUI global; English remains the default.
The web host bundles a small OFL-licensed Noto Sans SC subset named KUI Story Sans
for Chinese and Latin labels. Regenerate it with `scripts/subset-story-font.py` and
the upstream variable TTF after changing the preview vocabulary. The web build checks the shipped font against the Chinese characters used in the Rust stories and controls.
