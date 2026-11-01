# KUI foundation

Theme tokens, motion presets, and `init` for every KUI app.

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
# or
kui add kui-foundation
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
kui::foundation::init(cx);
let theme = kui::foundation::Theme::global(cx);
```

## Notes for AI

Install once per app. Components do not reinstall foundation by default.
