# StretchRefresh

A scrollable GPUI feed that refreshes after a pull from the top or an explicit refresh action.

```sh
kui add stretch-refresh
```

```rust
use gpui::{div, prelude::*, px, SharedString, Task};
use kui::components::stretch_refresh::StretchRefresh;

let feed = cx.new(|cx| StretchRefresh::new(
    "Activity",
    vec![SharedString::from("Build passed")],
    |item| item.clone(),
    |item, _, _| div().p(px(12.)).child(item.clone()).into_any_element(),
    |_, _| Task::ready(Ok(None)),
    cx,
).with_subtitle("Updated just now"));
```

Place the view in a sized parent. For copied source, use `crate::ui::components::stretch_refresh`.

| Input | Rust type | Default |
| --- | --- | --- |
| Title | `Into<SharedString>` | Required |
| Items | `Vec<T>` | Required |
| Key | `Fn(&T) -> SharedString` | Required, stable for each row |
| Row renderer | `Fn(&T, &mut Window, &mut App) -> AnyElement` | Required |
| Refresh | `Fn(&mut Window, &mut Context<Self>) -> Task<StretchRefreshResult>` | Required |
| `with_subtitle` | `Into<SharedString>` | Empty |
| `with_threshold` | `Pixels` | 72px after resistance |
| `with_done_label` | `Into<SharedString>` | Up to date |
| `with_error_label` | `Into<SharedString>` | Couldn’t refresh |
| `with_refresh_label` | `Into<SharedString>` | Refresh |
| `with_style` | `Fn(Div) -> Div` | Theme defaults |

Use `set_items` and `set_subtitle` before resolving the refresh task to update the view. Return `Ok(Some(label))` for a custom result, `Ok(None)` for the default or `Err(StretchRefreshError)` for the configured failure text. Calls to `refresh` while a task is active are coalesced. Existing keys retain identity. Duplicate keys keep their first item.

Mouse and touch pulls arm only at the top edge. A below-threshold or cancelled pull returns to rest. Use a second upward scroll gesture when the feed is already at its top. Enter and Space activate the refresh button. The focused list scrolls with arrows, Page Up, Page Down, Home and End. Reduce Motion keeps the list stationary and shows the same busy and result states.

Use `with_id` to distinguish multiple mounted feeds. Use `with_visibility` to suppress paint requests while a host hides a mounted feed.
