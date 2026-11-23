# Onboarding ready

Summarize learned domains, assistant identity, voice, and handover rules on Step 4 beside the fully populated live assistant dashboard.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add onboarding-ready`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::onboarding_ready::{OnboardingReadyScreen, OnboardingReadyEvent};

let screen = cx.new(|cx| OnboardingReadyScreen::new("onboarding-ready", cx));
cx.subscribe(&screen, |_, event: &OnboardingReadyEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Activate Edit on any summary row to jump back to Website, Brand, or Tone, or press Open dashboard to launch the workspace.

## API

Primary GPUI type: `OnboardingReadyScreen` (`kui::components::onboarding_ready::OnboardingReadyScreen`).
Event type: `OnboardingReadyEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile adaptation.

## Motion

Interactive transitions, live status indicators, and skeleton sweeps follow KUI motion curves and respect Theme::reduced_motion. Responsive breakpoints adapt columns, toolbars, and side inspectors cleanly from full desktop down to 390px mobile viewports.

Desktop and web use the same GPUI source.
