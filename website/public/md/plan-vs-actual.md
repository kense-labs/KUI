# Plan vs actual

Compare an autonomous agent's planned Gantt schedule against actual execution—dashed planned ghost bars, amber overruns, downstream steps sliding right via `Slide` springs, dependency connectors, and projected `ETA` vs `Plan` finish markers.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add plan-vs-actual`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::plan_vs_actual::{PlanStep, PlanVsActual, PlanVsActualEvent};

let gantt = cx.new(|cx| {
    PlanVsActual::new("pricing-gantt", cx)
        .with_task("Ship the pricing page")
        .with_steps(vec![
            PlanStep::new("brief", "Read the brief", 20_000).with_icon("slack").with_actual(Some(0), Some(16_000)),
            PlanStep::new("layout", "Build the layout", 60_000).with_deps(["brief"]).with_icon("figma").with_actual(Some(16_000), Some(84_000)),
            PlanStep::new("stripe", "Wire up Stripe", 45_000).with_deps(["layout"]).with_icon("stripe").with_actual(Some(84_000), None),
        ])
        .with_now(136_000)
});
cx.subscribe(&gantt, |_, ev: &PlanVsActualEvent, _| {
    let _ = ev;
}).detach();
```

## Interaction

Hovering, clicking, or focusing (`Tab`) any step dims unrelated rows and draws orthogonal dependency connectors (`PathBuilder`) to the upstream steps it waited on and the downstream steps waiting for it. Click `Replay` in the legend footer to watch the run play from `0:00` as overruns turn amber and push downstream steps and `ETA` to the right.

## API

Primary GPUI type: `PlanVsActual` (`kui::components::plan_vs_actual::PlanVsActual`).
Supporting types: `PlanStep`, `PlanStepStatus`, `format_plan_ms`.
Event type: `PlanVsActualEvent` (`FocusStep(Option<SharedString>)`, `ReplayStarted`).

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeletons, and `with_empty(true)` empty states.

## Motion

`Slide` springs smoothly glide waiting forecast bars and the horizontal time axis range when upstream steps overrun or finish early, while active steps spin a progress arc; `Theme::reduced_motion` disables continuous animations cleanly.

Desktop and web use the same GPUI source.
