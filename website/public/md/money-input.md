# Money input

Enter an amount with a fixed currency symbol and quick-add amounts.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add money-input`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::money_input::MoneyInput;

let control = cx.new(|cx| MoneyInput::new("Amount", 12500, 0..=1000000, window, cx).with_symbol("$").with_quick_add(vec![1000, 5000]));
```

## Interaction

Type an amount with up to two fractional digits and press Enter or blur to commit. Invalid or out-of-range drafts revert to the previous value. Arrow keys and quick-add chips step the amount; a step that would cross a limit holds the value and answers with a refusal strain and a warning flash on the allowed-range note instead of silently clamping.

## API

`minor_units` · `() -> i64`

Values, limits and quick-add increments are integer minor units. set_disabled disables editing and quick-add controls. The allowed range is shown as money beside the quick-add chips; a chip at the ceiling dims but stays clickable so its refusal still answers.

Event: `MoneyChanged`.

## Motion

Typing stays direct so the input caret remains stable. A refused step strains the amount toward the pressed direction on a critically damped spring and returns to rest, and the range note flashes to the warning token. Reduced motion applies the refusal with no strain and no flash.

Desktop and web use the same GPUI view.
