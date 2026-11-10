# ChargeSession

A shared GPUI view for a simulated battery charging session. The host owns real charger state and network calls.

```sh
kui add charge-session
```

```rust
use gpui::prelude::*;
use kui::components::charge_session::{ChargeSession, ChargeSessionOptions};

let charger = cx.new(|cx| ChargeSession::new(
    ChargeSessionOptions {
        title: "Bay 12, depot charger".into(),
        location: "North yard, 350 kW".into(),
        capacity: 98.,
        max_power: 350.,
        start_charge: 22.,
        default_limit: 85.,
        rate: 0.38,
        currency: "EUR".into(),
        locale: "de-DE".into(),
        ..Default::default()
    },
    cx,
));
```

For copied source, import from `crate::ui::components::charge_session`.

| Option | Rust type | Default |
| --- | --- | --- |
| `title` | `SharedString` | `"Fast charger 4"` |
| `location` | `SharedString` | `"Harbor Street hub, 150 kW"`. Empty hides it |
| `capacity` | `f64` | 77 kWh |
| `max_power` | `f64` | 150 kW |
| `start_charge`, `default_limit`, `min_limit`, `taper_at` | `f64` | 34%, 80%, 50%, 50% |
| `rate` | `f64` | 0.42 per kWh |
| `currency`, `locale` | `SharedString` | `"USD"`, `"en-US"` |
| `speed` | `f64` | 1. One second represents 40 simulated seconds. Zero freezes energy while packets keep moving |
| `paused` | `bool` | false. Freezes energy and packets |
| `accent` | `Option<Hsla>` | Theme accent |
| `label` | `Option<SharedString>` | Title |

`ChargeSessionLimitChanged(f64)` emits once per settled limit change. `ChargeSessionStateChanged` provides the new `Charging`, `Stopped` or `Complete` state and a `ChargeSessionDetail` snapshot with charge, energy, cost and limit. A restarted session returns to the starting charge and keeps the chosen limit. Raising the limit after completion resumes the simulation.

The battery and graph both support pointer and touch scrubbing. Focus the limit slider and use arrows for 1%, Shift plus arrows or Page Up/Down for 10%, and Home/End for the endpoints. Vertical touch gestures preserve page scrolling in the Web preview.

Use `with_style` and `with_id` for root layout and identity. Use `with_visibility` and `set_paused` for host lifecycle, and `with_cost_formatter` for any currency/locale formatting beyond the built-in defaults. The graph and stats sit side by side from 640px. The card uses tighter padding and a full-width action below 420px.
