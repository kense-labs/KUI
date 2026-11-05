# Link unfurl

Compose a message and show a host-resolved link preview before sending.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add link-unfurl`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::link_unfurl::LinkUnfurl;

let control = cx.new(|cx| LinkUnfurl::new("Message", window, cx));
```

## Interaction

The component does not visit URLs. It emits Resolve, Cancel, Open and Send for the host to handle. Responses must match the request ID. A send failure preserves the draft; edits made while sending are not cleared by an older success.

## API

`resolve_preview` · `(u64, Result<Option<LinkPreview>, SharedString>, &mut Context<Self>)`

set_text updates the composer; resolve_preview returns a host-supplied preview or error; resolve_send confirms or fails sending. Images, when supplied, must be decoded RenderImage values. Removing a link or replacing it emits Cancel for its outstanding preview.

Event: `LinkUnfurlEvent`.

## Motion

The card expands and collapses with a reduced-motion-aware transition; hidden previews do not request animation frames. Inline link morph, multiple link previews, Alt+Enter, drag-to-collapse, live announcements and real visual/accessibility acceptance remain pending.

Desktop and web use the same GPUI view.
