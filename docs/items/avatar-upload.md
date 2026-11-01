# AvatarUpload

Choose a profile image and edit its crop before applying it.

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
kui add avatar-upload
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::avatar_upload::AvatarUpload;

let control = cx.new(|cx| AvatarUpload::new("Jordan Reyes", cx));
```

## Interaction

Choose or drop a valid photo, zoom and pan the crop, then handle Upload with bytes and normalized crop coordinates.

## API

`finish` · `(ticket: u64, result: Result<(), String>, cx)`

Choose or drop a valid photo, zoom and pan the crop, then handle Upload with bytes and normalized crop coordinates.

Event: `AvatarEvent`.

## Motion

Crop diameter springs in both directions, upload ring consumes real progress, and successful completion emits the expanding ring.

Desktop and web use the same GPUI view.
