# MetricCard

Read one metric in the context of its change and comparison period.

## Install

```bash
kui add metric-card
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::metric_card::MetricCard;

let control = cx.new(|_| MetricCard::new("Active users", 2486.0, 18.4, "Previous 7 days"));
```

## Interaction

Pass the absolute value, percentage change and the comparison period.

## API

`update_metric` · `(f32, f32, comparison)`

Pass the absolute value, percentage change and the comparison period.

## Motion

Decimal places roll independently with a stagger on entry. Metric changes retarget the wheels, while the comparison and description crossfade.

Desktop and web use the same GPUI view.
