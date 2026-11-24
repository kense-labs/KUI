# Voice capture

Render a dictation capture panel with a recessed waveform well, live speech rhythm bars, ticking timer, action controls, and living under-glow.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add voice-capture`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::voice_capture::{VoiceCapture, VoiceCaptureEvent};

let capture = cx.new(|cx| {
    VoiceCapture::new("hero-voice-capture", cx)
        .with_start_seconds(14)
        .with_loudness(1.0)
});
cx.subscribe(&capture, |_, ev: &VoiceCaptureEvent, _| {
    let _ = ev;
})
.detach();
```

## Interaction

Click the recording indicator or press `Space` to pause or resume the timer and waveform, click `Done` or press `Enter` to confirm capture, or click `Cancel` or press `Escape` to reset the timer to `start_seconds`.

## API

Primary GPUI type: `VoiceCapture` (`kui::components::voice_capture::VoiceCapture`).
Event type: `VoiceCaptureEvent` (`kui::components::voice_capture::VoiceCaptureEvent`).

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_start_seconds(...)`, `with_loudness(...)`, `with_bars(...)`, `with_playing(...)`, `set_loudness(...)` for live host audio metering, `with_loading(true)` skeleton placeholders, and `with_empty(true)` empty states.

## Motion

Vertical speech bars inside the recessed quiet well rise and fall with natural phrase cadence while silent slots rest as `2×2px` pause dots, the tabular `0:14` clock ticks each second alongside a breathing red recording dot, and a tri-color glow shifts gently beneath the right-fading card; `Theme::reduced_motion` holds a calm static frame.

Desktop and web use the same GPUI source.
