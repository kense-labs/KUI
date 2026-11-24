# Command menu

Search and run workspace actions from one `⌘K` palette — SaaS tool actions with brand logos and `⌘1`–`⌘5` shortcuts alongside everyday navigation commands.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add command-menu`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::command_menu::{CommandMenu, CommandMenuEvent};

let menu = cx.new(|cx| {
    CommandMenu::new("workspace-cmd", cx)
        .with_auto_focus(true)
});
cx.subscribe(&menu, |_, event: &CommandMenuEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Use `↑` / `↓` to step row by row, `Home` / `End` to jump to the ends, `Tab` / `Shift+Tab` to jump between sections, `⌘1`–`⌘5` to run a suggestion immediately, `Enter` to execute the active row, and `Esc` to clear the query or close.

## API

Primary GPUI type: `CommandMenu` (`kui::components::command_menu::CommandMenu`).
Event type: `CommandMenuEvent`.

Customize sections and shortcuts with `with_groups(Vec<CommandGroup>)`, `CommandItem::app(id, label, app_name)`, `CommandItem::new(id, label, icon)`, `with_placeholder`, and `with_auto_focus(true)`. Supports inline substring match highlighting, AI assistant query fallback, light and dark `Theme` modes, and bilingual `UiLanguage`.

## Motion

A spring-driven selection pill glides smoothly from row to row across sections, and executed commands reveal a check confirmation in the footer tray. `Theme::reduced_motion` snaps selection changes instantaneously.

Desktop and web use the same GPUI source.
