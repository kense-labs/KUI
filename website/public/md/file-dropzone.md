# FileDropzone

Select or drop local files and track each upload attempt.

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
kui add file-dropzone
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::file_dropzone::FileDropzone;

let control = cx.new(|cx| FileDropzone::new("Add files", 10 * 1024 * 1024, 6, cx).with_extensions(&["pdf", "png"]));
```

## Interaction

Consume the bytes from Upload and report progress, failure or success against its ticket. Retry gets a fresh ticket.

## API

`report` · `(ticket: u64, state: UploadState, cx)`

Consume the bytes from Upload and report progress, failure or success against its ticket. Retry gets a fresh ticket.

Event: `FileEvent`.

## Motion

Retains outgoing row until the spring ends and retargets file hover on the native and browser paths.

Desktop and web use the same GPUI view.
