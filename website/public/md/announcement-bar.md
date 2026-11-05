# Announcement bar

Show dismissible announcements and actions.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add announcement-bar`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::announcement_bar::{Announcement, AnnouncementBar};

let messages = vec![Announcement::new("launch", "New components are available").with_action("Learn more")];
let control = cx.new(|cx| AnnouncementBar::new(messages, cx));
```

## Interaction

Messages rotate every seven seconds and pause while hovered or focused. Close the bar or activate its action.

## API

`reopen` · `(&mut Context<Self>)`

Provide message IDs, optional action labels and countdown durations. AnnouncementEvent reports actions and dismissal. The host persists dismissal and handles navigation.

Event: `AnnouncementEvent`.

## Motion

Messages crossfade; dismissal collapses the bar. Reduced motion applies changes immediately.

Desktop and web use the same GPUI view.
