# Time dial

Choose one instant and read the corresponding local time in several offices. The dial uses host-supplied UTC offsets for the selected date.

## Installation

```sh
kui add time-dial
```

## Usage

```rust
use gpui::prelude::*;
use kui::components::time_dial::{TimeDial, TimeDialCity, TimeDialValueChanged};

let cities = vec![
    TimeDialCity::new("nyc", "New York", "NYC", -240),
    TimeDialCity::new("lon", "London", "LON", 60),
    TimeDialCity::new("tyo", "Tokyo", "TYO", 540),
];
let control = cx.new(|cx| TimeDial::new(cities, cx)
    .with_default_value(900)
    .with_working_hours(540, 1080)
    .with_label("Call time"));
window.subscribe(&control, cx, |_, _: &TimeDialValueChanged, _, _| {}).detach();
```

## API

| Builder or method | Default | Behavior |
| --- | --- | --- |
| `new(Vec<TimeDialCity>, &mut Context<Self>)` | Required cities | Show up to five unique stable IDs. Supply offsets in minutes and short codes for the dial. |
| `with_default_value(i32)`, `with_value(i32)`, `set_value(i32, &mut Context<Self>)` | Uncontrolled, UTC minute 0 | UTC minutes can cross the reference day. A controlled host accepts proposals with `set_value`. |
| `with_default_home(&str)`, `with_home(&str)`, `set_home(&str, &mut Context<Self>)` | First city | Change the local readout while preserving the UTC instant. Unknown IDs are ignored. |
| `with_bounds(Option<i32>, Option<i32>)` | Unbounded at both ends | Limits the committed UTC minute. An attempted overshoot stretches and returns. |
| `with_weekday(u8)` | 1, Monday | UTC reference weekday. Sunday is 0. |
| `with_hour_cycle(TimeDialCycle)` | `Twelve` | Select 12-hour time or zero-padded 24-hour time. |
| `with_working_hours(u16, u16)` | 540 to 1080 | Half-open interval in local minutes. When the end precedes the start, the interval spans midnight. |
| `with_label`, `with_id`, `with_style`, `with_visibility` | Localized “Time”, `time-dial`, no override, window visibility | Supply accessible text, stable identity, host layout refinement and optional preview visibility. |
| `TimeDialValueChanged(pub i32)` | — | Emits the destination on release or keyboard step. |
| `TimeDialHomeChanged(pub SharedString)` | — | Emits the selected city's stable ID. |

## Interaction and motion

Drag or flick along the dial, or tap a position on the face. Arrow keys step by 15 minutes and Page Up or Page Down by one hour. Home and End target the first and last quarter-hour of the current home day. Tap a city or use Enter or Space on its focused row to change the home city.

The dial follows the pointer with soft detents, carries release velocity into its spring, and resists pulling past its bounds. Changing the home city trades its track for the outer knob. The clock rolls changing digit positions and the sun and moon turn over between day and night. Reduced motion removes coasting and continuous travel.

The layout stacks below 600px, with a dial up to 256px. Rows tighten below 360px of available list width.
