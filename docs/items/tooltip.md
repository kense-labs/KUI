# Tooltip

Reveal a short explanation on hover or keyboard focus.

## Install

```bash
kui add tooltip
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::tooltip::Tooltip;

let control = cx.new(|cx| Tooltip::new("Project visibility", "Workspace members only", cx));
```

## Interaction

Hover or focus the icon. Escape dismisses the hint.

## API

`new` · `(name, text, &mut App) → Tooltip`

Supply both an accessible name and a short explanation.

Event: `—`.

## Motion

Waits 250ms on pointer intent. A 3px rise and separate opacity channel preserve reversal; reduced motion retains a 90ms fade.

Desktop and web use the same GPUI view.
