# Agent sequence

Render a multi-agent run as an interactive UML sequence diagram with participant lifelines (`You`, agents, and tools), activation bars, directional solid call and dashed reply arrows, labelled `par` / `retry` frames, animated tip lights, and click-to-inspect popover cards.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add agent-sequence`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::agent_sequence::{
    AgentSequence, AgentSequenceEvent, SeqMessage, SeqParticipant,
};

let diagram = cx.new(|cx| {
    AgentSequence::new("checkout-sequence", cx)
        .with_title("Fix the checkout crash")
        .with_participants(vec![
            SeqParticipant::person("you", "You"),
            SeqParticipant::agent("lead", "Lead").with_model("Claude Opus"),
            SeqParticipant::agent("research", "Researcher").with_model("Claude Sonnet"),
            SeqParticipant::tool("github", "GitHub", "github"),
        ])
        .with_messages(vec![
            SeqMessage::call("m1", "you", "lead", "Fix the checkout crash").with_at_ms(0),
            SeqMessage::call("m2", "lead", "research", "Find the cause").with_at_ms(3_000),
            SeqMessage::call("m3", "research", "github", "search_code(\"emptyCart\")").with_at_ms(5_000),
            SeqMessage::reply("m4", "github", "research", "m3", "3 matches").with_at_ms(6_200),
            SeqMessage::reply("m5", "research", "lead", "m2", "Null total at cart.ts:142").with_at_ms(9_600),
        ])
});
cx.subscribe(&diagram, |_, ev: &AgentSequenceEvent, _| {
    let _ = ev;
}).detach();
```

## Interaction

Click (or focus via `Tab` and press `Enter` / `Space`) any participant chip on top to open its role, model, and sent/received count card. Click any message arrow—or focus the diagram and step with `ArrowUp` / `ArrowDown` / `Home` / `End` (`Escape` to close)—to inspect the call/reply route, frame badge (`PAR`, `RETRY`), monospace payload/error trace, and round-trip latency. Click the refresh icon in the header to replay all messages sequentially.

## API

Primary GPUI type: `AgentSequence` (`kui::components::agent_sequence::AgentSequence`).
Supporting types: `SeqParticipant`, `SeqParticipantKind`, `SeqMessage`, `SeqMessageKind`, `SeqMessageStatus`, `SequenceSelection`.
Event type: `AgentSequenceEvent` (`SelectionChanged(Option<SequenceSelection>)`, `ReplayStarted`).

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeletons, and `with_empty(true)` empty states.

## Motion

New messages draw across lifelines with a glowing tip streak and bloom a landing ripple at the target lifeline, open activation bars sweep a soft vertical glint while working agent 3x3 marks twinkle, and inspection cards settle via `pro_presence`; `Theme::reduced_motion` disables continuous motion cleanly.

Desktop and web use the same GPUI source.
