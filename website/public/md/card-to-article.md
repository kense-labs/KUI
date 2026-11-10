# Card to article

Expand a responsive story card into a scrollable article and return it to its grid position.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Call `kui::init(cx)` before opening a window. For a local source copy, run `kui init` followed by `kui add card-to-article` and replace `kui::` with `crate::ui::` in the example.

## Usage

```rust
use gpui::prelude::*;
use kui::components::card_to_article::{ArticleItem, CardToArticle};

let item = ArticleItem::new("journal", "The journal", "story.jpg".to_owned(), "An open notebook", |_, _| {
    gpui::div().child("A first paragraph.").into_any_element()
})
.with_tag("Journal")
.with_meta("3 min read");
let control = cx.new(|cx| CardToArticle::new(vec![item], cx));
```

## API

| Item | Behavior |
| --- | --- |
| `ArticleItem` | Supply a stable ID, title, image, alt text, and a callback returning the GPUI body. Tag and metadata are optional. |
| `with_default_value` | Initially open the article matching the ID. Defaults to closed. |
| `with_value` and `set_value` | Accept host-controlled state. The event carries the requested article ID or `None` when closing. An unknown ID resolves to closed. |
| `with_aspect_ratio` | Use the same ratio on both heroes. Defaults to 4:3. |
| `with_close_label` | Override the accessible close label. It defaults to the localized equivalent of Close article. |
| `ArticlePreviewHost` | Optional bridge for a canvas inside a scrolling page. Supplies the visible vertical range, a modal scroll lock, and routing for inner scroll events. Native windows use their viewport directly. |

## Interaction and motion

Below 520px of container width the grid uses one column. Open with a click, Enter or Space. The article is capped at 680px wide and shows reading progress on its top edge. Escape or the close button returns focus to the same card. Tab stays in the article. Page Up, Page Down, Home and End scroll its content.

Drag the hero downward with a mouse, or pull down at the top of the article with continuous scroll or touch. A short pull springs back. Pull past 100px or flick downward to dismiss. The closing surface reads the current card bounds and animates back to its slot. Reduced motion switches immediately.
