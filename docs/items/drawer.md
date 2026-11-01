# Drawer

Open a side sheet for a focused task and return to the trigger.

## Install

```bash
kui add drawer
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::drawer::Drawer;

let control = cx.new(|cx| Drawer::new("Open details", "Project details", Box::new(|_, _| gpui::div().child("Owner: Maya").into_any_element()), cx));
```

## Interaction

Open the side sheet, Tab within its contents, Escape to dismiss.

## API

`set_open` · `(&mut self, bool, &mut Window, &mut Context<Self>)`

Provide a trigger, heading and body builder. Subscribe to Opened and Closed.

Event: `DrawerEvent`.

## Motion

The sheet slides from its edge; the scrim fades separately. A header drag past one third or a quick flick closes it; reduced motion uses opacity.

Desktop and web use the same GPUI view.
