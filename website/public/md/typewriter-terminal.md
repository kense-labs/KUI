# Typewriter terminal

Play a scripted terminal session with syntax-colored input, timed output, and a timeline that can rewind or jump between commands. The transcript is illustrative. The component does not execute processes.

## Installation

```sh
kui add typewriter-terminal
```

## Usage

```rust
use gpui::prelude::*;
use kui::components::typewriter_terminal::{
    TerminalOutput, TerminalStep, TerminalTone, TypewriterTerminal,
};

let control = cx.new(|cx| {
    TypewriterTerminal::new(cx)
        .with_title("Quick start")
        .with_steps(vec![
            TerminalStep::new("kui add button").with_output([
                TerminalOutput::progress("Preparing components"),
                TerminalOutput::text("Button installed", TerminalTone::Success),
            ]),
            TerminalStep::new("cargo check")
                .with_cwd("~/acme")
                .with_output([TerminalOutput::task("Checking the workspace")]),
        ])
        .with_rows(8)
        .with_loop(false)
});
```

## API

| Input or method | Default | Behavior |
| --- | --- | --- |
| `TypewriterTerminal::new(&App)` | Local three-step Cargo and KUI sample | Builds a terminal session. Use `with_steps(Vec<TerminalStep>)` for your own script. An empty script shows a prompt. |
| `set_steps(Vec<TerminalStep>, &mut Context<Self>)` | — | Rebuilds the transcript and timing, clears pending scrubbing, and resets to the beginning. With reduced motion it shows the completed script. |
| `TerminalStep::new(command)`, `with_output(impl IntoIterator<Item = TerminalOutput>)` | Empty output | Provides a command and its ordered output. |
| `TerminalStep::with_cwd(impl Into<SharedString>)` | Inherit | Changes the working directory for the current and following prompts. |
| `TerminalStep::with_pause(Duration)` | 900ms | Adds a rest after this step. |
| `TerminalOutput::Text { text, tone, delay }` | `Default` tone, 180ms delay | Prints a line. Empty text produces a blank line. The convenience constructor is `text(text, tone)` and `&str` converts to plain output. Tones are `Default`, `Muted`, `Success`, `Warning`, `Danger` and `Accent`. The three status tones include a symbol. HTTP and HTTPS links are underlined and open on activation. |
| `TerminalOutput::Progress { label, duration }` | 1.2s | Fills a bar and shows its percentage. `progress(label)` uses the default duration. |
| `TerminalOutput::Task { label, done, duration }` | 550ms, no trailing text | Shows a braille spinner, then a check and optional completion text. `task(label)` uses these defaults. |
| `with_title(impl Into<SharedString>)` | `zsh` | Sets the centered title. |
| `with_cwd(impl Into<SharedString>)`, `with_symbol(impl Into<SharedString>)` | `~`, `$` | Set the starting directory and prompt symbol. |
| `with_speed(f32)`, `set_speed(f32, &mut Context<Self>)` | `1` | Set the playback multiplier. Nonfinite and nonpositive values leave the speed as it was. |
| `with_paused(bool)`, `set_paused(bool, &mut Context<Self>)`, `is_paused() -> bool` | `false` | Choose the initial playback state or change it at runtime. A pause eases the playback rate to rest. |
| `with_loop(bool)` | `true` | The completed screen rests before clearing upward and rewinding. With looping off, it stops on the final prompt. |
| `with_rows(usize)` | `11` | Sets the visible row count, with a minimum of one. The terminal has a fixed body height at each width. |
| `with_copyable(bool)` | `true` | Shows an icon button that copies commands joined by newlines when the script contains commands. |
| `with_font_family(&'static str)` | `monospace` | Sets the transcript font. Web hosts should supply a registered monospaced family when their text system has no generic monospace. The gallery uses its bundled Fira Code. |
| `with_accent(Hsla)` | Theme accent | Colors the caret, progress, spinner, links and timeline fill. |
| `with_label(impl Into<SharedString>)` | Localized “Terminal session” | Sets the accessible name. The figure exposes the complete transcript separately from the painted screen. |
| `with_caption(Fn(&mut Window, &mut App) -> AnyElement)` | None | Mounts optional content below the controls. |
| `with_style(Fn(Div) -> Div)`, `with_id(impl Into<ElementId>)` | No style override, `typewriter-terminal` | Refine the outer layout or set a stable GPUI identity. The width is capped at 760px. |
| `with_visibility(Fn(&App) -> bool)` | Native viewport visibility | Adds a host condition for suspending the clock offscreen. |
| `position() -> f32`, `duration() -> f32`, `seek(f32, &mut Context<Self>)` | Seconds in the script | Read the playhead or seek to a finite, clamped time. |
| `TerminalAction` | No initial event | User interactions emit `Seek(f32)`, `Play`, `Pause` or `Replay`. Programmatic state changes emit no playback event. |

## Interaction and motion

- Drag the thumb or click the track to seek. The visible transcript follows the pointer. Releasing within 7px of a command marker settles on that start. Cancellation returns to the position at contact.
- When the timeline has focus, Left, Down and Page Up jump to the previous command. Right, Up and Page Down jump to the next. Home and End go to the first and last positions. Space or K toggles play. A separate replay button rewinds and plays again.
- Each script has deterministic Unicode-grapheme timings. Output rises 4px as it fades in over 160ms. Progress advances quickly, slows, then finishes. The play rate and visible row position ease on springs.
- Reduced motion starts at the complete transcript and seeks immediately. Offscreen playback suspends the animation clock.
- Below 560px, the font size is 12px with a 21px row and 16px padding. Otherwise it uses 14px with a 24.5px row and 20px padding. The time readout hides below 400px. Long lines wrap by grapheme. The timeline, play and replay controls remain 44px tall.
