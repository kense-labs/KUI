# Stepper form

Guide users through a multi-step onboarding and setup wizard with filling progress bars, self-drawing check marks, directional step transitions, height morphing, inline validation, and a celebratory completion view.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add stepper-form`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::stepper_form::{StepperForm, StepperFormEvent};

let wizard = cx.new(|cx| {
    StepperForm::new("workspace-onboarding", cx)
        .with_submit_label("Create workspace")
});
cx.subscribe(&wizard, |_, event: &StepperFormEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Enter a workspace name (auto-slugifying `kense.app/<slug>`) on Step 1, add optional teammate email chips with `Enter` or `,` on Step 2, pick a plan card (with Left/Right arrow keys) on Step 3, click `Edit` on any summary row on Step 4 to jump back without losing data, and click `Create workspace` to reach the celebratory completion view with `Copy URL` and `Reset`.

## API

Primary GPUI type: `StepperForm` (`kui::components::stepper_form::StepperForm`).
Event type: `StepperFormEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_values(...)`, `with_initial_step(...)`, `with_submit_label(...)`, `with_plans(...)`, `with_loading(true)`, and `with_empty(true)`.

## Motion

Completed step indicators draw their check marks progressively while filling top progress bars via `pro_spring_f32`; advancing or retreating slides panes directionally via `pro_step_transition` while morphing card height via `Slide`, invalid fields shake and disclose messages via `pro_fold`, plan selection glides via `pro_pill_indicator`, and `Theme::reduced_motion` switches states immediately. Below `420px`, the header condenses to `STEP X OF Y` and plan cards stack.

Desktop and web use the same GPUI source.
