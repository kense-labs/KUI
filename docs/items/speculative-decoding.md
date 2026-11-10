# Speculative decoding

Show a draft model proposing tokens and a target model verifying them in one pass.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component, run `kui init` and `kui add speculative-decoding`. Replace `kui::` with `crate::ui::` in the example.

## Usage

```rust
use gpui::prelude::*;
use kui::components::speculative_decoding::{DecodeToken, SpeculativeDecoding};

let tokens = vec![
    DecodeToken::new("The"),
    DecodeToken::new(" build").with_ease(0.6).with_miss(" deploy"),
    DecodeToken::new(" passed"),
    DecodeToken::new("."),
];
let control = cx.new(|_| SpeculativeDecoding::from_tokens(tokens).with_default_compare(true));
```

Use `SpeculativeDecoding::new(rounds)` or `set_rounds` to replay supplied `DecodeRound { draft, accepted, target }` data. Recorded rounds omit the draft-length slider. Call `with_recorded(true)` to label the run as recorded.

## Interaction

Play, pause, or restart the run. The draft length runs from 2 through 8, and changing it replans the same answer from the start. Arrow keys move one step. Home and End select the limits. The compare switch adds a target-only line using the same clock and one token per pass. A guess after the first miss never joins the answer. The target supplies the replacement and writes the final token.

## API

| API | Behavior |
| --- | --- |
| `DecodeToken::new(text)` | Keeps leading spaces. Guess probability defaults to 0.85. Use `with_ease` and `with_miss` for a custom alternative. |
| `from_tokens`, `sample` | Build a deterministic simulation from tokens or the localized sample. |
| `with_seed(u64)`, `with_draft_cost(f32)`, `with_pass_duration_ms(u32)` | Defaults are 12, 0.1 per draft token, and 640ms per target pass. |
| `with_default_draft_length`, `with_draft_length`, `set_draft_length` | Defaults to 4. `with_draft_length` gives the host control and emits `DecodeDraftLengthChanged` on user input. |
| `with_default_compare`, `with_compare`, `set_compare` | Defaults to false. `with_compare` gives the host control and emits `DecodeCompareChanged` on user input. |
| `with_autoplay`, `set_visible`, `set_playing`, `restart` | Autoplay starts once when the card is in view. Reduced motion waits for explicit playback. |
| `set_status`, `with_retry` | Ready, Loading and Failed. Retry emits `DecodeRetry` when enabled. |
| `with_title`, `with_model_labels`, `with_id`, `with_style` | Customize the label, the two model names and details, and the GPUI root. |
| `DecodeCompleted` | Emits once when speculation finishes. Includes passes, drafted and accepted counts, tokens, acceptance, tokens per pass, and speedup with draft cost included. |

## Motion

Draft chips appear in order. Verification fills each target cell and exposes the verdict. Accepted tokens and the target token travel from their measured source cells to reserved answer positions. The two model origins use different underlines. Statistics roll after each completed pass. Reduced motion removes travel, shimmer and digit rolling. Desktop and web share the GPUI view.
