# Conflict radar

Inspect parallel coding-agent branches as a file-by-agent matrix with `PathBuilder` connector bars linking overlapping line edits, one-sentence resolution radio cards, and a `Merge branches` gate.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add conflict-radar`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::conflict_radar::{
    ConflictRadar, ConflictRadarEvent, RadarAgent, RadarEdit, RadarFile,
};

let radar = cx.new(|cx| {
    ConflictRadar::new("checkout-branches", cx)
        .with_agents(vec![
            RadarAgent::new("checkout", "Checkout"),
            RadarAgent::new("stripe", "Stripe v2"),
        ])
        .with_files(vec![
            RadarFile::new(
                "src/checkout/payment.ts",
                vec![
                    RadarEdit::new("checkout", 42, 78)
                        .with_summary("Validates card intent client-side.", "客户端预校验支付意图。"),
                    RadarEdit::new("stripe", 54, 92)
                        .with_summary("Uses Stripe PaymentElement webhook.", "使用 Stripe 回调确认。"),
                ],
            ),
        ])
});
cx.subscribe(&radar, |_, ev: &ConflictRadarEvent, _| {
    let _ = ev;
}).detach();
```

## Interaction

- Hover any agent column header to see its branch/model, touched file count, open conflicts, and kept resolutions; hover any cell pill to see exact modified line numbers.
- Click a conflicting file row (marked with an amber dot and horizontal `PathBuilder` connector bar) to inspect each agent's approach below as side-by-side radio cards (`Role::RadioGroup` / `Role::RadioButton`).
- Pick one agent's card per conflict (`Enter` / `Space` or click); once every overlap is settled and all agents are `Done`, `Merge branches` unlocks.

## API

Primary GPUI type: `ConflictRadar` (`kui::components::conflict_radar::ConflictRadar`).
Supporting types: `RadarAgent`, `RadarAgentStatus`, `RadarFile`, `RadarEdit`, `overlapping_agent_columns`.
Event type: `ConflictRadarEvent` (`Resolved { path, agent_id }`, `SelectedFile`, `MergeBranches`, `Reset`).

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton rows, and `with_empty(true)` empty states.

## Motion

`PathBuilder` draws horizontal overlap connector bars with a sweeping amber highlight pulse until settled, and `pro_view_transition` glides between conflict comparison panels; `Theme::reduced_motion` disables continuous pulses.

Desktop and web use the same GPUI source.
