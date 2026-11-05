# PhoneInput

Edit a telephone number with a separate country dialing code.

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
kui add phone-input
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::phone_input::PhoneInput;

let control = cx.new(|cx| PhoneInput::new("Mobile number", "US", window, cx));
```

## Interaction

Search calling codes or country names. Use international digits and possibility separately from verification.

## API

`value` · `(&App) -> PhoneValue`

Search calling codes or country names. Use international digits and possibility separately from verification.

Event: `PhoneValue`.

## Motion

Measured height and animated width drive reveal. Roving selection glides down the list, dialing-code digits roll.

Desktop and web use the same GPUI view.

## Country picker

- The selected-country trigger and each of the 49 supported rows show a color flag. Flags occupy a 20px slot; the native SVG artwork is drawn at 20 × 15px, centered vertically. Country names keep their accessible labels and calling codes, so flags are decorative rather than the only identifier.
- The list viewport grows to its content with a maximum height of 280px. Rows are 36px high, separated by 2px, with 10px horizontal padding and gaps. Names ellipsize, calling codes remain visible at 12px, and the selected row has a 16px check. Rectangular chrome retains KUI's 8px radius.
- Opening reveals the committed country. Mouse wheel and touch pan scroll the country list rather than the document while the list can consume that direction; a new outward gesture at a limit returns to normal document scrolling. The search row stays fixed.
- Up/Down move within the filtered rows without wrapping; PageUp/PageDown move eight rows. Home/End select the first/last row when the query is empty. Keyboard movement scrolls its row into view. Enter chooses; Escape closes and returns focus to the trigger; Tab closes without trapping focus.
- Search accepts an English or Chinese country name, ISO code, or calling code. Changing the query resets scroll to the first result. Empty results cannot be committed. Choosing a filtered country and reopening returns to that country's position in the complete list.
- Highlight motion uses a 0.28s zero-bounce spring. The existing expanding panel uses 0.4s open / 0.32s close; reduced motion applies the target directly. Hover moves the shared highlight without adding a second background.

The embedded Web host uses the existing `EditorScrollRegion` bridge for current list bounds and available scroll directions. Standalone desktop apps use GPUI's normal scroll input. No second HTML country selector is introduced.

### Flag artwork

Flags are bundled inside `primitives/phone_flags.rs`, including the MIT license from [flag-icons](https://github.com/lipis/flag-icons) at commit `ac7e0eb3259eb62aa6100c435607e2fe03896a97`. They do not depend on platform emoji fonts, a CDN or a network request at runtime. Image handles are cached; the same artwork ships through the library, WASM preview and `kui add phone-input`.

Country changes also update the empty input’s example using the public country masks and sample numbers; for example, US `(201) 555-0123` changes to China `131 2345 6789`. This does not imply full international number validation or verification.
