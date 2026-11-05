# Dictation button

Request dictation and insert a recognized draft.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add dictation-button`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::dictation_button::DictationButton;

let control = cx.new(|_| DictationButton::new());
```

## Interaction

The host owns microphone permission, recognition and editor insertion. A changed insertion point requires confirmation.

## API

`set_transcript` · `(impl Into<SharedString>, impl Into<SharedString>, &mut Context<Self>)`

Capture session_id on Start or Retry and use set_phase_for, set_transcript_for, set_level_for and finish_recognition_for. Confirm Insert then set Idle with the same ID. Old unscoped methods are compatible but require host-side stale-callback filtering.

Event: `DictationEvent`.

## Motion

The control grows into a compact listening row; reduced motion updates immediately.

Desktop and web use the same GPUI view.
