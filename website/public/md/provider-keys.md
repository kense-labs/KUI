# CredentialPanel

Manage a provider credential with masked display and validation feedback.

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
kui add provider-keys
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::provider_keys::{CredentialPanel, Provider};

let control = cx.new(|cx| CredentialPanel::new("keys",vec![Provider{id:"custom".into(),name:"Provider".into(),masked:String::new(),last_used:String::new(),valid:false,prefix:"sk-".into()}],window,cx));
```

## Interaction

The host owns key storage. A replacement is applied only after its own test passes. Remove hides that row immediately and emits Remove after an independent five-second undo window. Escape and Undo restore keyboard focus. The preview uses an in-memory vault.

## API

Test carries a draft and ticket. Check awaiting(ticket) before persisting, then call tested(ticket, masked, window, cx). RetestKey carries only provider and ticket; finish with retested. Complete Reveal with revealed and report Copy through copy_finished. with_mask_only disables access to a full secret. The existing `tested(ticket, Option<String>, &Window, cx)` accepts only the active candidate. A failed test keeps the previous key and returns focus to the input. `KeyAction::Restore` reports an undo while the secret is still in the host. `with_demo_inputs` exposes local sample keys for use in previews.

## Motion

The inline form follows a measured 240ms spring. Status and last-used text fade out over 100ms and in over 160ms. Reveal remasks after five seconds. Reduced motion skips travel and the countdown ring.

Desktop and web use the same GPUI view.
