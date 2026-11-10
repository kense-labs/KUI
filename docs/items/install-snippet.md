# Install snippet

Guide users through a setup with framework tabs, copyable snippets and a host-owned key request.

## Install

```bash
kui add install-snippet
```

Use `kui::` with the `kense-kui` crate and `crate::ui::` with copied source. Call `kui::init(cx)` before opening the application window.

## Usage

```rust
use gpui::prelude::*;
use kui::components::install_snippet::{InstallCommand, InstallFramework, InstallSnippet, InstallStep};

let steps = vec![InstallStep::new("install", "Install", vec![
    InstallCommand::new("Cargo", "cargo add kense-kui --rename kui"),
])];
let frameworks = vec![InstallFramework::new("rust", "Rust", steps)];
let control = cx.new(|cx| InstallSnippet::new(frameworks, cx));
```

## API

`InstallStep` accepts an optional description, filename and a `generate_key` flag. `with_title`, `with_description` and `with_version` set the card header. The first framework and package manager are selected by default. `select_framework` and `select_manager` change the selected indices.

Place `{{API_KEY}}` in snippets. `with_key_placeholder` changes the initial placeholder. `with_api_key` supplies a pre-existing key and hides the generation control. Subscribe to `InstallSnippetEvent::KeyRequested` and `KeyCancelled` to start or cancel host work. Pass the request ID to `finish_key` or `fail_key`. Late results for cancelled requests are discarded. The preview generates a synthetic key in memory.

`CopyFinished` includes the framework, step and whether the clipboard write succeeded. It supplies the copied text on success. A successful copy or key generation completes the step. Use `with_step_confirmation(true)` and `confirm_step` for an explicit host confirmation flow.

## Interaction

The preview uses KUI Desktop, Web, Library and CLI tabs. Code rows have reserved height and scroll independently. The multiline implementation step includes line numbers.

Use Left or Right to move within a tab list. Home and End select its first and last entries. Tab moves to other controls. Long code lines scroll horizontally within the snippet.
