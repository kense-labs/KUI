# Upgrade prompt

Explain a usage limit and offer one next plan with monthly and yearly billing.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Call `kui::init(cx)` before opening a window. To copy the source, run `kui init` and `kui add upgrade-prompt`, then replace `kui::` with `crate::ui::` in the example.

## Usage

```rust
use gpui::prelude::*;
use kui::components::upgrade_prompt::{UpgradePrompt, UpgradePromptOptions};

let control = cx.new(|cx| UpgradePrompt::new(UpgradePromptOptions::default(), cx));
```

## API

| Item | Behavior |
| --- | --- |
| `UpgradePromptOptions` | Defaults to 50 of 50 AI credits, a Nov 1 reset, and the yearly offer. Supply usage, reset label, an `UpgradePlan`, or optional title and description. |
| `UpgradePlan` | Defaults to Pro with 2,000 credits, 2,000 hundredths of USD per month or 1,600 hundredths of USD per month on yearly billing. Configure the currency, feature list and new limits. |
| `controlled_interval` | Set this to let the host accept `IntervalChanged` via `set_interval`. Otherwise the prompt updates its own selection. |
| `with_upgrade` | The callback receives an `UpgradeSelection` and returns a GPUI `Task<Result<(), SharedString>>`. The prompt owns the task, ignores repeated submissions while it is pending, and cancels observation on drop. Only the host can confirm an actual upgrade. |
| `UpgradeSelection::amount` | Display total in hundredths of the currency. It is the monthly price or twelve times the yearly per-month price. The billing service validates the actual charge independently. |
| `UpgradeEvent` | Reports period changes, dismissal, successful upgrades and continuing. The host owns snooze persistence and retrying the blocked operation. |

## Interaction

Use Tab to enter the billing choice and footer actions. Arrow keys change the billing period. Maybe later shows a reminder with a return action. Upgrade submits once, displays the host error inline and offers retry. Continue emits the event. In the sample it returns to the offer after the 1.1s simulated upgrade.

## Motion

The card keeps the controls in place while the body crossfades. Price digits roll over 280ms. The meter animates into view and updates over a 500ms spring. Reduced motion uses direct price and meter updates and removes panel travel.

Desktop and Web use the same GPUI view. The bundled preview uses simulated billing.
