# SocialProofToast

A GPUI notification stack for recent customer activity. Supply real events for a production feed. The gallery uses fictitious records.

```sh
kui add social-proof-toast
```

```rust
use gpui::prelude::*;
use kui::components::social_proof_toast::{
    SocialProofEvent, SocialProofOptions, SocialProofPosition, SocialProofToast,
};

let control = cx.new(|cx| {
    SocialProofToast::new(
        SocialProofOptions {
            events: Some(vec![
                SocialProofEvent::new("example-1", "Taylor", "started a trial")
                    .with_meta("Lisbon"),
            ]),
            position: SocialProofPosition::BottomLeft,
            looping: false,
            limit: Some(6),
            ..Default::default()
        },
        window,
        cx,
    )
    .on_show(|event, _| eprintln!("Shown {}", event.id))
});
```

Copied-source imports start with `crate::ui`. Supply records oldest first with stable unique IDs. `with_avatar` takes a GPUI image source. `with_logo` takes a decorative GPUI view shown in an 18px slot when the event has no avatar. `SocialProofTime` accepts text or a millisecond timestamp. `SocialProofVerification` supports a default verified label or custom text.

| Option | Rust type | Default |
| --- | --- | --- |
| `events` | `Option<Vec<SocialProofEvent>>` | Fictitious sample feed when unset |
| `interval` | `Duration` | 4000ms, minimum 16ms |
| `initial_delay` | `Duration` | 800ms |
| `max_visible` | `usize` | 3, minimum 1 |
| `prefill` | `usize` | 0 |
| `position` | `SocialProofPosition` | Inline |
| `expand_on_hover` | `bool` | true |
| `looping` | `bool` | true |
| `limit` | `Option<usize>` | Unbounded, counts prefilled toasts |
| `paused` | `bool` | false |
| `label` | `Option<SharedString>` | Recent activity, localized |
| `on_show`, `on_dismiss`, `on_select` | `Fn(&SocialProofEvent, &mut App)` | No handler |
| `with_style` | `Fn(Div) -> Div` | No override |

Set `set_paused` while a host overlay is open. Use `set_visible` or `with_visibility` when embedding in a scroll or browser host that controls visibility outside the GPUI window. Pausing preserves the remaining delay. `set_events` updates the feed while preserving the cursor when the last shown ID is present. A looping feed may show the same event again later.

Hover and focus fan the stack out. Tab visits visible toast controls newest first. Escape closes the focused toast, or the front toast if the stack itself holds focus. The next toast receives focus. New arrivals are silent. A status node announces dismissal.

Cards are 64px tall. Corner placement uses 24px gutters and becomes full width with 16px gutters below 480px. Reduced motion removes travel and shrink while preserving the fades. Touch-first hosts show close controls immediately.
