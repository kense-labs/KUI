# Flow builder

Build and test top-down agent automations in place — `+` buttons on every connector insert steps, `If / else` branches into named side-by-side lanes, edits revert cleanly with `Undo`, and `Test run` sends a glowing light pulse down each active connector.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add flow-builder`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::flow_builder::{
    FlowBranchPath, FlowBuilder, FlowBuilderEvent, FlowStep, FlowStepKind,
};

let builder = cx.new(|cx| {
    FlowBuilder::new("signup-onboarding", cx)
        .with_name("Onboard new signups")
        .with_steps(vec![
            FlowStep::new("signup", FlowStepKind::Trigger, "New signup").with_icon("form").with_detail("Signup form").with_note("Someone signs up on the website"),
            FlowStep::new("research", FlowStepKind::Agent, "Research the company").with_detail("Claude · web search").with_note("Reads their site, finds size and industry"),
            FlowStep::new("size", FlowStepKind::Branch, "Company size").with_detail("employees > 200").with_paths(vec![
                FlowBranchPath::new("enterprise", "Enterprise").with_steps(vec![
                    FlowStep::new("crm", FlowStepKind::Action, "Create CRM deal").with_icon("hubspot"),
                ]),
                FlowBranchPath::new("self", "Self-serve").with_steps(vec![
                    FlowStep::new("welcome", FlowStepKind::Action, "Send welcome email").with_icon("gmail"),
                ]),
            ]),
        ])
});
cx.subscribe(&builder, |_, ev: &FlowBuilderEvent, _| {
    let _ = ev;
}).detach();
```

## Interaction

Clicking `+` on any vertical connector opens an insertion menu (`Agent`, `Slack`, `Email`, `Wait`, or `If / else` conditional fork). Hovering or selecting a step reveals quick reorder (`↑` / `↓`, or `⌥↑` / `⌥↓`) and delete (`×`, or `Backspace`) controls; removing an `If / else` lifts its primary branch steps back into the parent lane, and `Undo` (`⌘Z`) restores prior snapshots. Clicking `Test run` streams a live pulse down active connectors into each step while dimming untaken branch paths.

## API

Primary GPUI type: `FlowBuilder` (`kui::components::flow_builder::FlowBuilder`).
Supporting types: `FlowStep`, `FlowStepKind`, `FlowBranchPath`, `FlowTemplate`, `FlowInsertTarget`, `FlowRunStatus`, `count_flow_steps`.
Event type: `FlowBuilderEvent` (`StepsChanged`, `Select`, `TestRunStarted`, `TestRunStopped`, `TestRunCompleted`, `Undone`).

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, and `with_empty(true)` empty states.

## Motion

During `Test run`, `pro_connector_pulse` sends a traveling light pulse down vertical and rounded fork connectors, active step cards receive a top-down blue illumination wash, `Agent` pixel avatars twinkle, and step notes shimmer; `Theme::reduced_motion` completes test runs and edits without continuous animation.

Desktop and web use the same GPUI source.
