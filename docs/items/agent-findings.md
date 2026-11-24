# Agent findings

Display a parallel audit or verification swarm with calm per-agent progress bars on top and a live feed where newly uncovered `Critical`, `Warning`, and `Note` findings slide smoothly into view.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add agent-findings`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::agent_findings::{
    AgentFindings, AgentFindingsEvent, Finding, FindingAgent, FindingSeverity,
};

let audit = cx.new(|cx| {
    AgentFindings::new("security-verification", cx)
        .with_title("Verification")
        .with_agents(vec![
            FindingAgent::new("auditor", "Auditor", 8, 24)
                .with_step("Tracing calls in Vault.withdraw", "追踪 Vault.withdraw 调用链"),
            FindingAgent::new("fuzzer", "Fuzzer", 7, 18)
                .with_step("Fuzzing transfer with 10k inputs", "模糊测试 transfer"),
        ])
        .with_findings(vec![
            Finding::new("f1", "auditor", FindingSeverity::Critical, "Reentrancy in withdraw()")
                .with_location("Vault.sol:142"),
        ])
});
cx.subscribe(&audit, |_, ev: &AgentFindingsEvent, _| {
    let _ = ev;
}).detach();
```

## Interaction

- Click or focus any agent row (`Enter` / `Space`) to filter `LATEST FINDINGS` to that agent alone; click again or dismiss the filter chip to show all agents.
- Click any finding row in the bottom feed to inspect its remediation note (`pro_fold`) and emit `AgentFindingsEvent::OpenFinding`.
- Use `Next check` or the header replay button to step or replay live discovery progression.

## API

Primary GPUI type: `AgentFindings` (`kui::components::agent_findings::AgentFindings`).
Supporting types: `FindingAgent`, `FindingAgentStatus`, `Finding`, `FindingSeverity`, `format_relative_age`.
Event type: `AgentFindingsEvent` (`OpenFinding`, `FilterAgent`, `FindingDiscovered`, `ResetRun`).

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton rows, and `with_empty(true)` empty states.

## Motion

`pro_spring_bar` smoothly advances each agent's check meter, `pro_rolling_number` rolls header severity counts, and newly discovered findings glide into the feed via `PhaseClock` + `pro_spring_f32`; `Theme::reduced_motion` disables continuous progression and transitions.

Desktop and web use the same GPUI source.
