# Pricing plans

Compare subscription tiers with a capsule monthly/yearly billing switcher, rolling plan prices, app sync badges, and collapsible feature matrices.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add pricing-plans`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::pricing_plans::{PricingBillingPeriod, PricingPlansBlock, PricingPlansEvent};

let block = cx.new(|cx| {
    PricingPlansBlock::new("pricing-plans", cx)
        .with_default_billing(PricingBillingPeriod::Yearly)
});
cx.subscribe(&block, |_, event: &PricingPlansEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Use Tab to move between the capsule billing switcher, plan CTAs, per-card detail toggles, and the feature comparison button. Press Left/Right or Enter/Space to switch between Monthly and Yearly (-20%), D to fold per-card details, and C to expand or hide the comparison table.

## API

Primary GPUI type: `PricingPlansBlock` (`kui::components::pricing_plans::PricingPlansBlock`).
Event type: `PricingPlansEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), custom currencies (`with_currency`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile adaptation.

## Motion

The capsule billing indicator glides smoothly across Monthly and Yearly, numeric prices roll smoothly on period changes, and per-card details plus the full comparison matrix fold open and closed while respecting Theme::reduced_motion.

Desktop and web use the same GPUI source.
