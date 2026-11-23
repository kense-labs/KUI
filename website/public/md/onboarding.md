# Onboarding

Guide new workspaces through a 4-step in-app setup checklist covering workspace profile, tool connections, team invites, and starter agent templates.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add onboarding`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::onboarding::{OnboardingScreen, OnboardingEvent};

let screen = cx.new(|cx| OnboardingScreen::new("onboarding", cx));
cx.subscribe(&screen, |_, event: &OnboardingEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Press 1-4 or [ / ] to move between steps, toggle tool switches in place, press Enter or Space to add invite chips, and Backspace to remove the last chip.

## API

Primary GPUI type: `OnboardingScreen` (`kui::components::onboarding::OnboardingScreen`).
Event type: `OnboardingEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile adaptation.

## Motion

Interactive transitions, live status indicators, and skeleton sweeps follow KUI motion curves and respect Theme::reduced_motion. Responsive breakpoints adapt columns, toolbars, and side inspectors cleanly from full desktop down to 390px mobile viewports.

Desktop and web use the same GPUI source.
