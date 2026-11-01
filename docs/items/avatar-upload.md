# AvatarUpload

Choose a profile image and edit its crop before applying it.

## Install

```bash
kui add avatar-upload
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::avatar_upload::AvatarUpload;

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
