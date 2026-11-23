# Onboarding website

Collect company website, docs, and help-center domains as chips on the left while each domain lands live in the assistant Knowledge table on the right.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add onboarding-website`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::onboarding_website::{OnboardingWebsiteScreen, OnboardingWebsiteEvent};

let screen = cx.new(|cx| OnboardingWebsiteScreen::new("onboarding-website", cx));
cx.subscribe(&screen, |_, event: &OnboardingWebsiteEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Enter, comma, or space normalizes and adds a domain chip, Backspace on an empty field removes the last chip, and Enter on an empty field continues.

## API

Primary GPUI type: `OnboardingWebsiteScreen` (`kui::components::onboarding_website::OnboardingWebsiteScreen`).
Event type: `OnboardingWebsiteEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile adaptation.

## Motion

Interactive transitions, live status indicators, and skeleton sweeps follow KUI motion curves and respect Theme::reduced_motion. Responsive breakpoints adapt columns, toolbars, and side inspectors cleanly from full desktop down to 390px mobile viewports.

Desktop and web use the same GPUI source.
