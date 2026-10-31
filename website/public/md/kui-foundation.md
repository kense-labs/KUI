# KUI foundation

Theme tokens, motion presets, and `init` for every KUI app.

## Install

```bash
kui init
# or
kui add kui-foundation
```

## Usage

```rust
kui::foundation::init(cx);
let theme = kui::foundation::Theme::global(cx);
```

## Notes for AI

Install once per app. Components do not reinstall foundation by default.
