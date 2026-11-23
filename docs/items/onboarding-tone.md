# Onboarding tone

Configure assistant display name, voice tone, reply length, and human escalation email while previewing live test-chat replies on the right.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add onboarding-tone`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::onboarding_tone::{OnboardingToneScreen, OnboardingToneEvent};

let screen = cx.new(|cx| OnboardingToneScreen::new("onboarding-tone", cx));
cx.subscribe(&screen, |_, event: &OnboardingToneEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Arrow keys cycle Friendly/Neutral/Formal tone and Short/Balanced/Detailed length radios, toggle human handover with the capsule switch, and validate email.

## API

Primary GPUI type: `OnboardingToneScreen` (`kui::components::onboarding_tone::OnboardingToneScreen`).
Event type: `OnboardingToneEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile adaptation.

## Motion

Interactive transitions, live status indicators, and skeleton sweeps follow KUI motion curves and respect Theme::reduced_motion. Responsive breakpoints adapt columns, toolbars, and side inspectors cleanly from full desktop down to 390px mobile viewports.

Desktop and web use the same GPUI source.
