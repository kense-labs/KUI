# TextShimmer

Show a soft highlight sweeping over an active status label.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add text-shimmer`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::text_shimmer::TextShimmer;

let control = cx.new(|cx| TextShimmer::new("Thinking…", cx));
```

## Interaction

Pause with set_active or set_visible; defaults to a 1.8 second sweep.

## API

`set_active` · `(bool, &mut Context<Self>)`

Pause with set_active or set_visible; defaults to a 1.8 second sweep.

`with_font_size(f32)` 默认16px，接受正有限数，微光带仍按2.2em计算。

`with_active(bool)` 设置初始活动状态，默认true；false从静态文字开始，不播放停用过渡。

Event: `—`.

## Motion

The highlight passes across the label; reduced motion shows steady text.

Desktop and web use the same GPUI view.
