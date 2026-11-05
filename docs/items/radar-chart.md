# Radar chart

Compare multiple profiles across shared dimensions.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add radar-chart`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::radar_chart::{RadarAxis, RadarChart, RadarProfile};

let axes = vec![RadarAxis::new("speed", "Speed"), RadarAxis::new("quality", "Quality"), RadarAxis::new("reach", "Reach")];
let profiles = vec![RadarProfile::new("current", "Current", vec![80.0, 65.0, 90.0], gpui::rgb(0x9d83e8).into())];
let control = cx.new(|_| RadarChart::new("Capabilities", axes, profiles));
```

## Interaction

Axes are capped at twelve (further axes are truncated); fewer are allowed. Missing and non-finite values render as zero; values are clamped to 0–100.

## API

`set_profiles` · `(Vec<RadarProfile>, &mut Context<Self>)`

Set axes and profiles at construction; replace profiles with set_profiles. Arrows or axis buttons inspect values.

Event: `RadarInspect`.

## Motion

Matching profile IDs retain their current vertex positions as values change; reduced motion updates immediately.

Desktop and web use the same GPUI view.
