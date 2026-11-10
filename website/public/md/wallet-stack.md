# Wallet stack

Fan a handful of payment cards, passes, or memberships. Lift a card to see its balance and three recent transactions.

## Installation

```sh
kui add wallet-stack
```

## Usage

```rust
use gpui::prelude::*;
use kui::components::wallet_stack::{
    WalletStack, WalletStackCard, WalletStackSelected, WalletTransaction, WalletVariant,
};

let wallet = cx.new(|cx| {
    WalletStack::new(
        vec![
            WalletStackCard::new(
                "debit", "Fieldnote", "Everyday debit", "4821", "Jordan Hayes",
                3248.60, "Available",
            )
            .with_expiration("09/29")
            .with_transactions(vec![WalletTransaction::new(
                "split", "Ryan Mitchell", "Dinner split · Today", 36.50,
            )]),
            WalletStackCard::new(
                "credit", "Meridian", "Reserve credit", "0917", "Jordan Hayes",
                1286.44, "Spent in September",
            )
            .with_variant(WalletVariant::Black),
        ],
        cx,
    )
    .with_label("Jordan’s cards")
});
cx.subscribe(&wallet, |_, event: &WalletStackSelected, _| {
    let selected_id = event.0.as_ref().map(|card| card.id.as_ref());
    println!("Opened card: {selected_id:?}");
}).detach();
```

## API

| Input | Default | Behavior |
| --- | --- | --- |
| `new(Vec<WalletStackCard>, &mut Context<Self>)` | Required | Cards in source order. Starts closed with the first card in front. |
| `with_id(impl Into<ElementId>)` | Entity-derived ID | Stable root element identity. |
| `with_label(impl Into<SharedString>)` | Wallet / 钱包 | Visible heading and accessible group name. |
| `with_currency(impl Into<SharedString>)` | `USD` | ISO currency code used for balance and activity amounts. |
| `with_locale(impl Into<SharedString>)` | `en-US` | Selects number separators and currency formatting for supported locales. |
| `with_amount_formatter(impl Fn(f64, &str, &str) -> String + 'static)` | Built-in formatter | Host override. Arguments are amount, currency, and locale. |
| `with_style(impl Fn(Div) -> Div + 'static)` | None | Refines the root layout. |
| `front_card()`, `selected_card()` | First card, no selection | Return the visible front card or the currently open card. |
| `set_device_light(Option<[f32; 2]>, &mut Context<Self>)` | Pointer light | Host can supply orientation axes normalized to −1 to 1. Passing `None` restores pointer tracking. |
| `WalletStackSelected(Option<WalletStackCard>)` | No event on initialization | Emits the opened card, or `None` when it returns to the stack. Closed-stack browsing emits no selection event. |

`WalletStackCard::new` takes ID, issuer, product, display suffix, holder, balance, and balance label. Supply only the display suffix. The component keeps at most its last four ASCII digits. `with_variant` selects `Metal` (default), `Black`, `Glass`, or `Color`. `with_finish` maps `Paper` to `Metal`, `Graphite` to `Black`, and `Clay` or `Photo` to `Color`. `with_tint` sets the color material’s base. `with_expiration`, `with_image`, and `with_transactions` add optional content. `with_network` accepts `Fictional` (default), `Hidden`, `Label(SharedString)`, or `Svg(SharedString)` with a registered SVG path.

`WalletTransaction::new` takes ID, merchant, detail, and amount. Negative amounts are expenses. Positive amounts are credits. `with_icon` supplies a KUI glyph and `with_avatar` accepts an image that takes precedence over the glyph.

## Interaction and motion

- Hover to fan and illuminate the front card. Drag down for the next card or up for the previous card. Dragging at either end encounters resistance, and a flick settles at most one card away.
- Click or tap a card to open its activity. Pull the open card down, press Esc, or use All cards to close and return focus to it. Down and Right advance, Up and Left go back, and Home and End jump to the endpoints. Enter and Space toggle the focused card. Only the front card is a tab stop in the card stack.
- Cards keep the ID-1 ratio and scale down from 296px to the available width. Rectangular card corners use the shared surface radius at full scale. Activity lists display the first three entries in source order.
- Card position and opening settle on critically damped springs. Direct manipulation shares the same geometry. Balance count-up and staggered activity fade stop requesting frames when settled. Reduced motion snaps state changes and omits pointer light, tilt, and staged reveals.
