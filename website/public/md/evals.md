# Evals

Benchmark prompt versions across 25 customer test cases with regression cards, 12-version pass-rate trend, 7-hunk prompt diff review, and rubric inspector.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add evals`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::evals::{EvalsScreen, EvalsEvent};

let screen = cx.new(|cx| EvalsScreen::new("evals", cx));
cx.subscribe(&screen, |_, event: &EvalsEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Press R to run the eval suite live, [ / ] and Y / N to step and accept/reject prompt diff hunks, and J/K + Enter to inspect candidate vs baseline outputs.

## API

Primary GPUI type: `EvalsScreen` (`kui::components::evals::EvalsScreen`).
Event type: `EvalsEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile adaptation.

## Motion

Interactive transitions, live status indicators, and skeleton sweeps follow KUI motion curves and respect Theme::reduced_motion. Responsive breakpoints adapt columns, toolbars, and side inspectors cleanly from full desktop down to 390px mobile viewports.

Desktop and web use the same GPUI source.
