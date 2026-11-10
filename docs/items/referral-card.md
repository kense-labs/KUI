# ReferralCard

A GPUI referral program with email chips, a conversion funnel and a paid milestone.

```sh
kui add referral-card
```

```rust
use gpui::{Task, prelude::*};
use kui::components::referral_card::{
    ReferralCard, ReferralCardOptions, ReferralMilestone,
};

let control = cx.new(|cx| {
    ReferralCard::new(
        ReferralCardOptions {
            link: "https://example.com/r/kense20".into(),
            referrals: Some(Vec::new()),
            milestone: ReferralMilestone::new(5, "3 months free"),
            ..Default::default()
        },
        window,
        cx,
    )
    .on_invite(|invite, _, _| {
        // Hand invite.emails and invite.signal to the host's delivery service.
        let _ = invite;
        Task::ready(Ok(()))
    })
});
```

With copied source, import from `crate::ui::components::referral_card`. Use unique stable IDs and newest-first referral snapshots. `Referral::new` takes an ID, email, `ReferralStage` and ISO invitation date. Names and avatar images are optional. Update the list with `set_referrals` as confirmations and paid conversions arrive.

| Option | Rust type | Default |
| --- | --- | --- |
| `link` | `SharedString` | Sample invite URL |
| `reward` | `u32` | 20 whole units |
| `currency` | `SharedString` | USD |
| `title`, `description` | `Option<SharedString>` | Generated from reward and currency |
| `referrals` | `Option<Vec<Referral>>` | Sample program when unset |
| `milestone` | `ReferralMilestone` | 5 paid referrals, 3 months free, applied to the next invoice |
| `share_message` | `Option<SharedString>` | Card title |
| `max_invites` | `usize` | 10 per request |
| `simulate` | `Option<bool>` | Enabled when referrals are unset |
| `on_invite` | `Fn(ReferralInvite, &mut Window, &mut App) -> Task<Result<(), SharedString>>` | Simulated delivery |
| `on_share` | `Fn(ReferralShareChannel, SharedString, &mut Window, &mut App)` | Simulated share |
| `on_copy` | `Fn(SharedString, &mut Window, &mut App)` | No callback |
| `on_milestone` | `Fn(&ReferralMilestone, &mut App)` | No callback |
| `with_style` | `Fn(Div) -> Div` | No override |

Enter, Tab, comma and space commit the draft. Paste addresses separated by spaces, commas or newlines. In the empty field, Enter sends the chips and Backspace removes the last one. The editor keeps its focus while a request is pending. Errors preserve the draft. Cancelling or dropping a pending card marks the request signal as cancelled. The host may observe that signal and abort its work.

`referral_share_url(channel, link, message)` returns a URL for X, LinkedIn or email. Copy writes the complete URL through the active platform. Host callbacks are responsible for delivery and opening external destinations. The sample program uses local fictitious addresses.
