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

Start or pause processing; retry failures. In-flight and failed tickets show their original message. Once classified, a ticket displays the available intent, sentiment and confidence. Review uncertain suggestions and choose a destination. Assignment is shown only after host confirmation. Queue cards use two columns when the content area is at least 350px wide, then stack on narrower surfaces.

## API

Use `with_model(name)` to identify the classification model. Set `Ticket::with_sender` and `Ticket::with_channel` to supply the email context displayed in the queue.

`classified` · `(u64, TicketDecision, &mut Context<Self>) -> bool`

Handle ClassifyRequested, RouteRequested and Cancelled events. Return matching request IDs through classified, routed or failed. set_ticket cancels work based on older content.

Event: `TicketRouterEvent`.

## Motion

Ticket state changes fade in; reduced motion removes the transition.

Desktop and web use the same GPUI view.
