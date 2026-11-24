# Agent queue

Visualize queued agent jobs gliding into open worker pool slots with growing progress bars, landing in `Done` or `Failed` bins, and driving a live `PathBuilder` backlog depth area chart with simulated rate-limit spikes.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add agent-queue`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::agent_queue::{AgentQueue, AgentQueueEvent};

let queue = cx.new(|cx| {
    AgentQueue::new("ops-agent-queue", cx)
        .with_title("Queue", "agent-jobs")
        .with_backpressure(8)
});
cx.subscribe(&queue, |_, ev: &AgentQueueEvent, _| {
    let _ = ev;
}).detach();
```

## Interaction

- Click `Rate limit` in the header to simulate a provider `429` throttle: active worker bars hold in amber with a countdown while incoming jobs pile up in `Queue` and spike the backlog depth chart, then drain rapidly once cleared.
- Toggle `Pause` / `Resume` or step `Workers` (`- N +`) to scale worker concurrency live.
- Click `+ Urgent` to enqueue a high-priority job that sorts ahead of normal queued tasks, or click any queued/running job chip to open its inspector bar (`Move to front` / `Cancel job`).

## API

Primary GPUI type: `AgentQueue` (`kui::components::agent_queue::AgentQueue`).
Supporting types: `QueueJob`, `QueueJobKind`, `QueueJobStatus`, `QueuePriority`, `QueueWorker`.
Event type: `AgentQueueEvent` (`PausedChanged`, `RateLimitToggled`, `ConcurrencyChanged`, `JobPromoted`, `JobCancelled`, `JobEnqueued`, `JobSelected`).

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, and `with_empty(true)` empty states.

## Motion

`pro_spring_f32` advances worker execution bars smoothly, `pro_rolling_number` rolls KPI and `Done`/`Failed` counters, and `PathBuilder` paints the live backlog area chart with rate-limit bands and dashed backpressure threshold; `Theme::reduced_motion` halts continuous simulation loops.

Desktop and web use the same GPUI source.
