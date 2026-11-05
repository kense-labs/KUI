# Smart form fill

Extract structured suggestions into editable fields with review and undo.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add smart-form-fill`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::smart_form_fill::{SmartField, SmartFormFill};

let fields = vec![SmartField::new("name", "Name"), SmartField::new("email", "Email")];
let control = cx.new(|cx| SmartFormFill::new(fields, window, cx));
```

## Interaction

Paste source text, request suggestions, review low-confidence values and save. A value being edited is queued for explicit confirmation. The demonstration uses a local sample parser.

## API

`apply` · `(u64, SmartSuggestion, &Window, &mut Context<Self>) -> bool`

Handle SmartFillEvent::Requested and Cancelled in the host; pass the request ID to apply, finish or fail. Submitted contains reviewed values; the host owns extraction and persistence.

Event: `SmartFillEvent`.

## Motion

Accepted fields receive a brief tint; the matching source text is highlighted while filling or hovering. Reduced motion applies the final state immediately.

Desktop and web use the same GPUI view.
