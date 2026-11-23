# Onboarding brand

Capture company name and short aliases on Step 2 while updating the live assistant monogram, workspace title, and conversation chart on the right.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add onboarding-brand`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::onboarding_brand::{OnboardingBrandScreen, OnboardingBrandEvent};

let screen = cx.new(|cx| OnboardingBrandScreen::new("onboarding-brand", cx));
cx.subscribe(&screen, |_, event: &OnboardingBrandEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Type a company name or comma-separated aliases and press Enter to add, Backspace to pop the last alias, and Continue when at least one name is set.

## API

Primary GPUI type: `OnboardingBrandScreen` (`kui::components::onboarding_brand::OnboardingBrandScreen`).
Event type: `OnboardingBrandEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile adaptation.

## Motion

Interactive transitions, live status indicators, and skeleton sweeps follow KUI motion curves and respect Theme::reduced_motion. Responsive breakpoints adapt columns, toolbars, and side inspectors cleanly from full desktop down to 390px mobile viewports.

Desktop and web use the same GPUI source.
