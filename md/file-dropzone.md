# FileDropzone

Select or drop local files and track each upload attempt.

## Install

```bash
kui add file-dropzone
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::file_dropzone::FileDropzone;

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
