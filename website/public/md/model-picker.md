# Model picker

Compare and select LLMs with spring-animated `Speed`, `Quality`, and `Cost` meters, unfolding `Extended thinking` effort controls, and rolling latency and token pricing.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add model-picker`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::model_picker::{ModelPicker, ModelPickerEvent, ThinkingEffort};

let picker = cx.new(|cx| {
    ModelPicker::new("composer-model-picker", cx)
        .with_value("claude-sonnet")
        .with_thinking(true)
        .with_effort(ThinkingEffort::Medium)
});
cx.subscribe(&picker, |_, event: &ModelPickerEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Hover or use `Up` and `Down` arrow keys across `Claude`, `GPT`, `Gemini`, `DeepSeek`, and `Auto` to watch each model's `Speed`, `Quality`, and `Cost` spring bars fill while the footer tray rolls first-token latency and USD per `1M` tokens. Flip `Extended thinking` to unfold `Low`, `Medium`, and `High` effort (`Left` / `Right` arrows) and toggle `Web search`.

## API

Primary GPUI type: `ModelPicker` (`kui::components::model_picker::ModelPicker`).
Event type: `ModelPickerEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive adaptation down to narrow viewports.

## Motion

Capability meters (`pro_spring_bar`), capsule switches (`pro_switch`), unfolding effort disclosure (`pro_fold`), sliding effort thumb (`pro_pill_indicator`), and rolling latency/price readouts (`pro_rolling_number`) respect `Theme::reduced_motion`.

Desktop and web use the same GPUI source.
