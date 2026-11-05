# Mixture of experts

Illustrate token routing across sparse experts using host-supplied scores.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add mixture-of-experts`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::mixture_of_experts::{MixtureOfExperts, MoeExpert, MoeToken};

let experts = vec![MoeExpert::new("language", "Language", "Words"), MoeExpert::new("math", "Math", "Numbers")];
let tokens = vec![MoeToken::new("42", vec![0.2, 0.8])];
let control = cx.new(|_| MixtureOfExperts::new(experts, tokens));
```

## Interaction

The host supplies scores and optional parameter counts; no model is executed. Positive finite scores are ranked with stable tie order. Capacity controls skipped choices. Unspecified or invalid scores do not fabricate a route.

## API

`set_data` · `(Vec<MoeExpert>, Vec<MoeToken>, &mut Context<Self>)`

set_data replaces the illustration; set_top_k changes the number of choices, set_paused and set_visible control playback. MoeEvent reports routes and changes. Token and expert buttons support Enter, Space, arrows, Home, End and Escape.

Event: `MoeEvent`.

## Motion

The sample uses a moving packet along curved links. The stage switches to a vertical layout below 568 px. Hidden, paused and pinned views stop animation; reduced motion renders static links. Live announcements and desktop, Web, visual and accessibility verification remain pending.

Desktop and web use the same GPUI view.
