# PaymentFlow

Illustrates a simulated card payment in one GPUI figure. The same component renders in desktop and Web.

```sh
kui add payment-flow
```

```rust
use gpui::prelude::*;
use kui::components::payment_flow::{PaymentFlow, PaymentFlowOptions};

let options = PaymentFlowOptions {
    amount: 42.50,
    fee: 1.53,
    balance: 1284.10,
    ..Default::default()
};
let control = cx.new(|cx| PaymentFlow::new(options, cx));
```

For installed source, import from `crate::ui::components::payment_flow`.

| Option | Rust type | Default |
| --- | --- | --- |
| `nodes` | `[PaymentFlowNode; 5]` | Checkout, Stripe, Mastercard, Bank, Your balance |
| `amount`, `fee`, `balance` | `f64` each | `42.50`, `1.53`, `1284.10` in major units |
| `currency`, `locale` | `SharedString` each | `"USD"`, `"en-US"` |
| `authorization_ms` | `u32` | `1340`. Controls the step readout |
| `labels` | `PaymentFlowLabels` | Authorization, Settlement, Next business day, Approved |
| `hide_steps` | `bool` | `false` |
| `speed` | `f32` | `1.0`. Zero holds the current phase |
| `paused` | `bool` | `false` |
| `accent` | `Option<Hsla>` | `None`, uses the theme accent |
| `label` | `Option<SharedString>` | `None`, generated from amounts and stops |

Override a stop with `options.with_node(PaymentFlowStop, PaymentFlowNode)`. Each override accepts an optional `label`, `meta`, `detail`, `icon` (`Glyph`), and `color`. Merchant metadata always shows the balance. Use `with_mark` for an image or a custom vector element and `with_style` to style the root `Div`. `with_id` gives each figure a stable host identifier. `with_caption` adds optional visible text.

The built-in formatter covers common currency symbols, ISO fraction digits and English, Chinese, French, German, Spanish, Italian and Portuguese number conventions. Use `with_money_formatter` for product-specific or additional locale formatting. Values are illustrative. The component makes no payment calls.

The payment chip runs out along the authorization rail and the approval chip returns. Settlement takes a second rail. At the processor waypoint the gross and net values crossfade, and the merchant balance keeps accumulating between cycles. At speed 1 a loop lasts about 15.3 seconds. Node hover slows playback and shows supplemental detail. Use `set_speed`, `set_paused`, `set_visible` or `with_visibility` to control animation in the host.

Below 700px the authorization rail runs downward and settlement returns up to the merchant. The progress rows stack below 420px. Reduced motion renders the frame after the fee. The five stops are informational and the entire figure has one accessible description.
