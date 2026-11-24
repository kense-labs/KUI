# Setup guide

Collapsible onboarding panel with a spring progress bar, `"3 of 7 completed"` step counter, animated check-mark drawing, smooth strikethrough titles, and per-step action buttons.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add setup-guide`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::setup_guide::{SetupGuide, SetupGuideEvent};

let guide = cx.new(|cx| {
    SetupGuide::new("workspace-onboarding", cx)
});
cx.subscribe(&guide, |_, event: &SetupGuideEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Press `Tab` to move across header buttons, group disclosures, task checkboxes, and per-step action buttons. Press `Enter` or `Space` to fold/unfold a group or check/uncheck a step; completing all steps in a group automatically folds it away and opens the next incomplete group.

## API

Primary GPUI type: `SetupGuide` (`kui::components::setup_guide::SetupGuide`).
Event type: `SetupGuideEvent`.

Customize sections and tasks with `with_groups(Vec<SetupGroup>)`, `SetupTask::new(id, label).with_action(cta).with_done(bool)`, `with_title`, `with_minimized`, `with_edit_button`, and `with_close_button`. Supports light and dark `Theme` modes, bilingual `UiLanguage`, `with_loading(true)`, and `with_empty(true)`.

## Motion

Checking off a task draws its vector check mark progressively inside an accent disc while sweeping a `1px` strikethrough across the title; `pro_spring_bar` advances overall progress and `pro_fold` smoothly expands or collapses task groups. `Theme::reduced_motion` completes transitions immediately.

Desktop and web use the same GPUI source.
