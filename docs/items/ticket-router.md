# Ticket router

Classify tickets, review uncertain decisions and assign them through the host.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add ticket-router`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::ticket_router::{Ticket, TicketLane, TicketRouter};

let lanes = vec![TicketLane::new("support", "Support"), TicketLane::new("billing", "Billing")];
let tickets = vec![Ticket::new("one", "Invoice question", "Please check the invoice.")];
let control = cx.new(|_| TicketRouter::new("Inbox", lanes, tickets));
```

## Interaction

Start or pause processing with a click, Enter or Space. The current message shows its source, sender, subject and preview. Once classified, it shows intent, sentiment, confidence and the suggested destination. Retry failed requests or send them to manual review. Activate a review row to accept its suggested destination, or open the other destinations beside it. Assignment is shown only after host confirmation.

With up to three destination lanes, the view uses one row and a connector at 600px of content width. Below 600px it uses two columns, switching to one only below 256px of content width. Destination and review pages show up to three tickets at 600px and above, and up to two at smaller widths.

## API

Use `with_model(name)` to identify the classification model. Set `Ticket::with_sender` and `Ticket::with_channel` to supply the email context displayed in the queue.

- `with_history(assigned, review)` restores existing records oldest first. Both arguments are `Vec<(Ticket, TicketDecision)>`. Unknown destination IDs are skipped and incoming tickets take precedence over duplicate historical IDs.
- `with_portraits(portraits)` takes pairs of assignee `SharedString` and GPUI `ImageSource`.
- `set_visible(visible, cx)` suspends requests and emits cancellation while hidden. Work resumes on visibility unless paused by the user.
- `send_to_review(ticket, cx)` moves a failed ticket to manual review and returns whether the move was applied.

`classified` · `(u64, TicketDecision, &mut Context<Self>) -> bool`

Handle ClassifyRequested, RouteRequested and Cancelled events. Return matching request IDs through classified, routed or failed. set_ticket cancels work based on older content.

Event: `TicketRouterEvent`.

## Motion

The first message fades in over 220ms. Subsequent messages use a 100ms exit and 160ms entrance. Classification labels appear with a 50ms stagger. A 240ms pulse follows the wide connector during assignment. Reduced motion shows each state directly.

Desktop and web use the same GPUI view.
