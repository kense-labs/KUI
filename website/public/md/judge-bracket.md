# Judge bracket

LLM-as-a-judge single-elimination tournament bracket (`Quarterfinal -> Semifinal -> Final`) pairing two candidate completions at a time, fading eliminated contenders onto dashed bracket arms, lighting the champion's full path through the bracket, and opening any match's two completions side by side with rubric scores and the judge's reasoning below.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add judge-bracket`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::judge_bracket::{JudgeBracket, JudgeBracketEvent};

let bracket = cx.new(|cx| {
    JudgeBracket::new("judge-bracket", cx)
        .with_judge("Claude Opus 4.6")
        .with_live(true)
});
cx.subscribe(&bracket, |_, event: &JudgeBracketEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Click any match in `Quarterfinal`, `Semifinal`, or `Final` (or navigate across rounds and matches with the arrow keys) to open both contenders' completions side by side below with rubric scores (`/ 10`) and the judge model's verdict sentence. Use `Next match` or `Auto`/`Pause` to step through the bracket, and click `Use champion` (or `Use <winner>`) to accept the preferred completion. Press `Escape` to follow live judging again.

## API

Primary GPUI type: `JudgeBracket` (`kui::components::judge_bracket::JudgeBracket`).
Event type: `JudgeBracketEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_title(...)`, `with_task(...)`, `with_judge(...)`, `with_contenders(...)`, `with_verdicts(...)`, `with_decided_count(...)`, `with_live(...)`, `with_loading(true)`, and `with_empty(true)`.

## Motion

Both contenders in the match currently under evaluation shimmer via `pro_shimmer_text`; eliminated slots dim flat onto dashed arms; and once the Final concludes, a glowing blue `pro_connector_pulse` highlights the champion's full bracket path from Round 1 to the trophy-badged Final slot. Respects `Theme::reduced_motion` and stacks cleanly below `600px`.

Desktop and web use the same GPUI source.
