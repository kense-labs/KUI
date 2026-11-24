# Agent trail

Render the steps an autonomous agent works through as a quiet vertical progress trail with `"3/5"` completion count, branching sub-steps, a traveling rail light pulse, and dashed amber approval gates.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add agent-trail`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::agent_trail::{AgentTrail, AgentTrailEvent};

let trail = cx.new(|cx| {
    AgentTrail::new("checkout-trail", cx)
        .with_title("Fix the checkout crash")
});
cx.subscribe(&trail, |_, ev: &AgentTrailEvent, _| {
    let _ = ev;
}).detach();
```

## Interaction

Clicking or pressing `Enter` / `Space` on any step with sub-steps folds its nested sub-task spine open or closed via `pro_fold`. Long sub-step lists (`> 6`) fold cleanly to five items around the active sub-step with `+N earlier` and `+N more` toggles. Clicking a gated (`Approval` / `Needs you`) step opens the inline approval prompt with `Decline` and `Approve` actions.

## API

Primary GPUI type: `AgentTrail` (`kui::components::agent_trail::AgentTrail`).
Supporting types: `TrailStep`, `TrailStepState`, `TrailSubstep`, `TrailSubstepState`, `TrailAgent`, `format_trail_secs`.
Event type: `AgentTrailEvent` (`ToggleExpanded { step_id, expanded }`, `Approve(SharedString)`, `Decline(SharedString)`).

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `without_header()`, `with_loading(true)` skeleton rows, and `with_empty(true)` empty states.

## Motion

A spinning progress arc, shimmering status verb, and live ticking clock animate the `Running` step while a fading blue light pulse travels down the S-curved branch wire into the active sub-step and blooms a subtle arrival ring; `Theme::reduced_motion` disables the pulse, spin, and shimmer cleanly.

Desktop and web use the same GPUI source.
