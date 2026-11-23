# Agent skills

Browse, toggle, install, and author reusable SKILL.md workflows with trigger phrases, 14-day run metrics, and a 4-way creation modal.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add agent-skills`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::agent_skills::{AgentSkillsScreen, AgentSkillsEvent};

let screen = cx.new(|cx| AgentSkillsScreen::new("agent-skills", cx));
cx.subscribe(&screen, |_, event: &AgentSkillsEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

J/K or arrows move across skill cards, Space toggles or installs, Enter inspects SKILL.md, N opens New skill, / focuses search, and Esc dismisses.

## API

Primary GPUI type: `AgentSkillsScreen` (`kui::components::agent_skills::AgentSkillsScreen`).
Event type: `AgentSkillsEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile adaptation.

## Motion

Interactive transitions, live status indicators, and skeleton sweeps follow KUI motion curves and respect Theme::reduced_motion. Responsive breakpoints adapt columns, toolbars, and side inspectors cleanly from full desktop down to 390px mobile viewports.

Desktop and web use the same GPUI source.
