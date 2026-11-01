# MentionInput

Search people while composing and insert the chosen mention at the caret.

## Install

```bash
kui add mention-input
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::mention_input::{MentionInput, Mention};

let control = cx.new(|cx| MentionInput::new("Update", vec![Mention { id: "ava".into(), label: "Ava".into(), trigger: '@' }], window, cx));
```

## Interaction

Type @ for people or # for channels. Up and Down move the highlight, Tab or Enter inserts a match, and Escape dismisses suggestions. Shift+Enter inserts a new line. Enter posts when the list is closed.

## API

`new` · `(label, Vec<Mention>, window, cx)`

new receives the host roster. with_details adds searchable descriptions keyed by ID. value returns the draft and tagged UTF-8 byte ranges. On post, the control emits MentionValue with those ranges and MentionEvent::Submitted with the plain text.

## Motion

Preserves outgoing candidates and slides the active option plate on arrow navigation.

Desktop and web use the same GPUI view.
