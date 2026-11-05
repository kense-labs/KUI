# Speculative decoding

Replay host-supplied draft and verification rounds.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add speculative-decoding`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::speculative_decoding::{DecodeRound, SpeculativeDecoding};

let rounds = vec![DecodeRound::new(vec!["The".into(), " build".into(), " failed".into()], 2, Some(" passed".into()))];
let control = cx.new(|_| SpeculativeDecoding::new(rounds).with_recorded(true));
```

## Interaction

The host supplies the rounds; recorded and illustrative runs are explicitly labelled. Leading token spaces are preserved, acceptance is bounded, and speedup is an estimate based on configured draft cost.

## API

`set_rounds` · `(Vec<DecodeRound>, &mut Context<Self>)`

Play, pause, restart or enable the illustrative target-only comparison; set_visible stops off-screen playback. The host controls loading, failure and retry.

Event: `DecodeCompleted, DecodeRetry, DecodePlaybackChanged`.

## Motion

Drafts fade and slide in, verification sweeps across each chip, and accepted tokens appear in sequence; reduced motion steps directly. Full cross-position token flight and rolling statistics remain different from the reference; visual, touch and accessibility validation remain pending.

Desktop and web use the same GPUI view.
