# AccessEditor

Invite collaborators and manage access to a shared resource.

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
kui add share-access
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::share_access::{AccessEditor, Person, AccessRole};

let control = cx.new(|cx| AccessEditor::new("share","Release plan","https://workspace.example/release",vec![Person{name:"Maya".into(),email:"maya@example.com".into(),role:AccessRole::Owner}],window,cx));
```

## Interaction

Validate invitations and protect the owner row. Report copy success only after clipboard write.

## API

`new` · `(id, document, url, Vec<Person>, window, cx)`

Pass a document name, share URL and people with typed AccessRole values. Persist invitation, role and link changes via AccessEvent.

Event: `AccessEvent`.

## Motion

The panel and role menu have separate measured disclosures. Their leading gaps shrink with their height instead of jumping on mount/unmount.

Desktop and web use the same GPUI view.
