# Agent inbox

Triage all your background agent tasks in one calm list or kanban board—with rolling filter counts, live ticking clocks, unfolding step histories, inline `Approve` / `Reject` / `Retry` / `Open` actions, and FLIP glides between groups.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add agent-inbox`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::agent_inbox::{
    AgentInbox, AgentInboxEvent, AgentInboxFilter, AgentInboxView,
};

let inbox = cx.new(|cx| {
    AgentInbox::new("background-agent-inbox", cx)
        .with_default_filter(AgentInboxFilter::All)
        .with_default_view(AgentInboxView::List)
});
cx.subscribe(&inbox, |_, ev: &AgentInboxEvent, _| {
    let _ = ev;
}).detach();
```

## Interaction

Switch filter tabs (`All`, `Needs you`, `Running`, `Done`) or toggle `List` and `Board` layouts on wide screens. Click any task card (or press Enter/Space) to unfold its recent step timeline and trigger `Approve`, `Reject`, `Retry`, or `Open` right in place; pressing `Escape` closes the open card.

## API

Primary GPUI type: `AgentInbox` (`kui::components::agent_inbox::AgentInbox`).
Event type: `AgentInboxEvent`.

Supports `AgentTaskStatus` (`Running`, `Waiting`, `Failed`, `Done`), `AgentInboxFilter`, `AgentInboxView`, `with_header_action`, light/dark `Theme`, bilingual `UiLanguage`, `with_loading(true)`, and `with_empty(true)`.

## Motion

Filter and layout pills slide via `pro_pill_indicator`, tab and group badges roll via `pro_rolling_number`, task details unfold via `pro_fold`, and approving or retrying a task glides its card smoothly into its new group (FLIP). Respects `Theme::reduced_motion`.

Desktop and web use the same GPUI source.
