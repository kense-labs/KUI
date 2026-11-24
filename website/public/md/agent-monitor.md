# Agent monitor

Supervise one working agent in a compact card showing its live clock and rolling USD spend, proportional tool time strip, expandable finished steps with tool logos, exact command box when awaiting approval, and a foldable slim status bar.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add agent-monitor`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::agent_monitor::{
    AgentMonitor, AgentMonitorEvent, MonitorAsk, MonitoredAgent,
};

let monitor = cx.new(|cx| {
    AgentMonitor::new("release-monitor", cx)
        .with_agent(
            MonitoredAgent::new("Claude Opus 5.5")
                .with_role("Release agent · web-app v3.2", "发布智能体 · web-app v3.2")
                .with_provider("claude"),
        )
        .with_ask(Some(
            MonitorAsk::new("Ready to ship", "gh release create v3.2.0 --latest")
                .with_app("github"),
        ))
});
cx.subscribe(&monitor, |_, ev: &AgentMonitorEvent, _| {
    let _ = ev;
}).detach();
```

## Interaction

- Hover any segment in the time strip or any finished step row to cross-highlight its partner; click a step (or press `Enter` / `Space`) to unfold (`pro_fold`) its tool, timing, and input/output command summary.
- When `MonitorState::Waiting`, inspect the exact monospace command and click `Approve` (`Enter`) or `Deny` (`Escape`).
- Click the top-right collapse button (`Fold away`) to fold the card into a `44px` status bar showing overlapping icons of every tool used so far; hover a tool icon for its step tooltip or click the bar to reopen.

## API

Primary GPUI type: `AgentMonitor` (`kui::components::agent_monitor::AgentMonitor`).
Supporting types: `MonitoredAgent`, `MonitorState`, `MonitorStep`, `MonitorStepKind`, `MonitorStepState`, `MonitorAsk`, `MonitorOutcome`, `format_step_duration`, `format_live_clock`.
Event type: `AgentMonitorEvent` (`Approved`, `Denied`, `Paused`, `Resumed`, `Cancelled`, `Retried`, `OpenLog`, `FoldChanged`, `StepInspected`).

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, and `with_empty(true)` empty states.

## Motion

`pro_rolling_number` updates running USD spend, `pro_shimmer_text` sweeps across active step titles, and `pro_fold` drives both step input/output drawers and card collapse; `Theme::reduced_motion` disables continuous loops and transitions.

Desktop and web use the same GPUI source.
