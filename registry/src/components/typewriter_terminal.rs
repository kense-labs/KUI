//! A scriptable terminal replay. A session is displayed as text and never executed.
use crate::{
    components::copy_button::CopyButton,
    foundation::{Theme, surface_radius},
    primitives::{
        control::UiLanguage,
        direct_touch::Contact,
        family::{Paint, Slide, disabled_motion},
        glyph::{Glyph, icon},
    },
};
use gpui::{prelude::*, *};
use std::{
    cell::Cell,
    collections::{HashMap, HashSet},
    ops::Range,
    rc::Rc,
    time::Duration,
};
use unicode_segmentation::UnicodeSegmentation;
use web_time::Instant;

const MAX_WIDTH: f32 = 760.;
const STEP_PAUSE: f32 = 0.9;
const OUTPUT_DELAY: f32 = 0.18;
const LOOP_REST: f32 = 0.7;
const SNAP_PIXELS: f32 = 7.;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TerminalTone {
    #[default]
    Default,
    Muted,
    Success,
    Warning,
    Danger,
    Accent,
}

#[derive(Clone, Debug)]
pub enum TerminalOutput {
    Text {
        text: SharedString,
        tone: TerminalTone,
        delay: Option<Duration>,
    },
    Progress {
        label: SharedString,
        duration: Option<Duration>,
    },
    Task {
        label: SharedString,
        done: Option<SharedString>,
        duration: Option<Duration>,
    },
}

impl TerminalOutput {
    pub fn text(text: impl Into<SharedString>, tone: TerminalTone) -> Self {
        Self::Text {
            text: text.into(),
            tone,
            delay: None,
        }
    }

    pub fn progress(label: impl Into<SharedString>) -> Self {
        Self::Progress {
            label: label.into(),
            duration: None,
        }
    }

    pub fn task(label: impl Into<SharedString>) -> Self {
        Self::Task {
            label: label.into(),
            done: None,
            duration: None,
        }
    }
}

impl From<&str> for TerminalOutput {
    fn from(text: &str) -> Self {
        Self::text(text.to_owned(), TerminalTone::Default)
    }
}

#[derive(Clone, Debug)]
pub struct TerminalStep {
    pub command: SharedString,
    pub output: Vec<TerminalOutput>,
    pub cwd: Option<SharedString>,
    pub pause: Option<Duration>,
}

impl TerminalStep {
    pub fn new(command: impl Into<SharedString>) -> Self {
        Self {
            command: command.into(),
            output: Vec::new(),
            cwd: None,
            pause: None,
        }
    }

    pub fn with_output(mut self, output: impl IntoIterator<Item = TerminalOutput>) -> Self {
        self.output = output.into_iter().collect();
        self
    }

    pub fn with_cwd(mut self, cwd: impl Into<SharedString>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    pub fn with_pause(mut self, pause: Duration) -> Self {
        self.pause = Some(pause);
        self
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Syntax {
    Ink,
    Muted,
    Flag,
    Path,
    String,
    Package,
    Assignment,
    Operator,
    Success,
    Warning,
    Danger,
    Accent,
    Link,
}

#[derive(Clone)]
struct Span {
    range: Range<usize>,
    syntax: Syntax,
}

fn shell_spans(command: &str) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut words = command.char_indices().peekable();
    let mut first = true;
    while let Some((start, c)) = words.next() {
        if c.is_whitespace() {
            continue;
        }
        let mut quoted = matches!(c, '\'' | '"').then_some(c);
        let starts_quoted = quoted.is_some();
        let operator = "|&;<>".contains(c);
        let mut escaped = c == '\\';
        while let Some(&(_, next)) = words.peek() {
            if !escaped && quoted.is_none() {
                if next.is_whitespace() || "|&;<>".contains(next) != operator {
                    break;
                }
                if !operator && matches!(next, '\'' | '"') {
                    quoted = Some(next);
                }
            } else if !escaped && quoted == Some(next) {
                quoted = None;
            }
            words.next();
            escaped = !escaped && next == '\\';
        }
        let end = words.peek().map_or(command.len(), |(i, _)| *i);
        let word = &command[start..end];
        let syntax = if operator {
            first = word.contains(['|', '&', ';']);
            Syntax::Operator
        } else if starts_quoted {
            Syntax::String
        } else if let Some((name, _)) = word.split_once('=')
            && !name.is_empty()
            && name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
        {
            Syntax::Assignment
        } else if word.starts_with('-') {
            Syntax::Flag
        } else if word.starts_with(['/', '~', '.']) || word.contains('/') && !word.starts_with('@')
        {
            Syntax::Path
        } else if !first && word.contains('@') {
            Syntax::Package
        } else {
            first = false;
            Syntax::Ink
        };
        spans.push(Span {
            range: start..end,
            syntax,
        });
    }
    spans
}

#[derive(Clone)]
struct TextLink {
    range: Range<usize>,
    url: SharedString,
}

fn links(text: &str) -> Vec<TextLink> {
    let mut links = Vec::new();
    let mut offset = 0;
    while let Some(relative) = text[offset..].find("http") {
        let from = offset + relative;
        let tail = &text[from..];
        offset = from + "http".len();
        if !tail.starts_with("https://") && !tail.starts_with("http://") {
            continue;
        }
        let end = tail
            .find(|c: char| c.is_whitespace() || matches!(c, '"' | '\'' | '<' | '>'))
            .unwrap_or(tail.len());
        let url = tail[..end].trim_end_matches(['.', ',', ';', ')', ']', '}']);
        if url
            .split_once("://")
            .is_some_and(|(_, address)| !address.is_empty() && !address.starts_with('/'))
        {
            links.push(TextLink {
                range: from..from + url.len(),
                url: url.to_owned().into(),
            });
        }
        offset = from + end;
    }
    links
}

struct Prompt {
    prefix: SharedString,
    command: SharedString,
    keys: Vec<(f32, usize)>,
    spans: Vec<Span>,
}

struct TextOutput {
    text: SharedString,
    tone: TerminalTone,
    links: Vec<TextLink>,
}

enum LineKind {
    Prompt(Prompt),
    Text(TextOutput),
    Progress(SharedString),
    Task {
        label: SharedString,
        done: Option<SharedString>,
    },
    Gap,
}

struct TimedLine {
    at: f32,
    end: f32,
    kind: LineKind,
}

struct Mark {
    at: f32,
    command: SharedString,
}

struct Session {
    lines: Vec<TimedLine>,
    marks: Vec<Mark>,
    duration: f32,
    commands: SharedString,
    transcript: SharedString,
}

fn secs(duration: Option<Duration>, default: f32) -> f32 {
    duration.map_or(default, |time| time.as_secs_f32())
}

fn key_schedule(command: &str) -> Vec<(f32, usize)> {
    let mut hash = command
        .bytes()
        .fold(0x811c9dc5u32, |h, b| {
            (h ^ u32::from(b)).wrapping_mul(0x01000193)
        })
        .max(1);
    let mut time = 0.24;
    command
        .grapheme_indices(true)
        .map(|(i, cluster)| {
            hash ^= hash << 13;
            hash ^= hash >> 17;
            hash ^= hash << 5;
            let pause = if cluster.chars().all(char::is_whitespace) {
                0.048
            } else if cluster.starts_with(|c: char| c.is_uppercase() || "|&@-".contains(c)) {
                0.024
            } else {
                0.
            };
            time += 0.036 + (hash % 39) as f32 / 1000. + pause;
            (time, i + cluster.len())
        })
        .collect()
}

fn prompt(cwd: &str, symbol: &str, command: SharedString) -> Prompt {
    let prefix = format!("{cwd} {symbol} ").into();
    let keys = key_schedule(&command);
    let spans = shell_spans(&command);
    Prompt {
        prefix,
        command,
        keys,
        spans,
    }
}

impl Session {
    fn compile(steps: &[TerminalStep], initial: &str, symbol: &str) -> Self {
        let mut at = if steps.is_empty() { 0. } else { 0.3 };
        let mut cwd = initial;
        let mut lines = Vec::new();
        let mut marks = Vec::new();
        let mut transcript = Vec::new();
        let mut commands = Vec::new();
        for (index, step) in steps.iter().enumerate() {
            if let Some(path) = &step.cwd {
                cwd = path.as_ref();
            }
            marks.push(Mark {
                at,
                command: step.command.clone(),
            });
            commands.push(step.command.as_ref());
            transcript.push(format!("{cwd} {symbol} {}", step.command));
            let input = prompt(cwd, symbol, step.command.clone());
            let end = at + input.keys.last().map_or(0.22, |(t, _)| *t);
            lines.push(TimedLine {
                at,
                end,
                kind: LineKind::Prompt(input),
            });
            at = end + 0.26;
            for output in &step.output {
                let (duration, kind) = match output {
                    TerminalOutput::Text { text, tone, delay } => {
                        at += secs(*delay, OUTPUT_DELAY);
                        transcript.push(text.to_string());
                        (
                            0.,
                            LineKind::Text(TextOutput {
                                text: text.clone(),
                                tone: *tone,
                                links: links(text),
                            }),
                        )
                    }
                    TerminalOutput::Progress { label, duration } => {
                        at += OUTPUT_DELAY;
                        transcript.push(format!("{label} 100%"));
                        (secs(*duration, 1.2), LineKind::Progress(label.clone()))
                    }
                    TerminalOutput::Task {
                        label,
                        done,
                        duration,
                    } => {
                        at += OUTPUT_DELAY;
                        transcript.push(format!("{label} {}", done.as_deref().unwrap_or("✓")));
                        (
                            secs(*duration, 0.55),
                            LineKind::Task {
                                label: label.clone(),
                                done: done.clone(),
                            },
                        )
                    }
                };
                lines.push(TimedLine {
                    at,
                    end: at + duration,
                    kind,
                });
                at += duration;
            }
            at += secs(step.pause, STEP_PAUSE);
            if index + 1 < steps.len() {
                lines.push(TimedLine {
                    at: (at - 0.2).max(lines.last().map_or(0., |line| line.end)),
                    end: at,
                    kind: LineKind::Gap,
                });
            }
        }
        let last = prompt(cwd, symbol, "".into());
        lines.push(TimedLine {
            at,
            end: at,
            kind: LineKind::Prompt(last),
        });
        Self {
            lines,
            marks,
            duration: if steps.is_empty() { 0. } else { at + 0.32 },
            commands: commands.join("\n").into(),
            transcript: transcript.join("\n").into(),
        }
    }

    fn next(&self, position: f32) -> f32 {
        self.marks
            .iter()
            .find(|mark| mark.at > position + 0.025)
            .map_or(self.duration, |mark| mark.at)
    }

    fn previous(&self, position: f32) -> f32 {
        self.marks
            .iter()
            .rev()
            .find(|mark| mark.at < position - 0.025)
            .map_or(0., |mark| mark.at)
    }

    fn nearest_start(&self, position: f32, tolerance: f32) -> f32 {
        self.marks
            .iter()
            .map(|mark| mark.at)
            .chain([0., self.duration])
            .min_by(|a, b| (a - position).abs().total_cmp(&(b - position).abs()))
            .filter(|mark| (*mark - position).abs() <= tolerance)
            .unwrap_or(position)
    }
}

#[derive(Clone, Copy)]
struct Journey {
    from: f32,
    to: f32,
    duration: f32,
    elapsed: f32,
    clearing: bool,
}

impl Journey {
    fn progress(self) -> f32 {
        let x = (self.elapsed / self.duration).clamp(0., 1.);
        x * x * (3. - 2. * x)
    }
}

struct Playback {
    position: f32,
    playing: bool,
    velocity: f32,
    rate: f32,
    last: Option<Instant>,
    journey: Option<Journey>,
    rest: f32,
}

impl Playback {
    fn new() -> Self {
        Self {
            position: 0.,
            playing: true,
            velocity: 0.,
            rate: 0.,
            last: None,
            journey: None,
            rest: 0.,
        }
    }

    fn jump(&mut self, to: f32, time: f32, clearing: bool) {
        self.rest = 0.;
        self.rate = 0.;
        self.velocity = 0.;
        self.last = None;
        self.journey = None;
        if time > 0. && (self.position - to).abs() > 0.001 {
            self.journey = Some(Journey {
                from: self.position,
                to,
                duration: time,
                elapsed: 0.,
                clearing,
            });
        } else {
            self.position = to;
        }
    }

    fn step(&mut self, now: Instant, duration: f32, speed: f32, repeat: bool) -> bool {
        let dt = self.last.replace(now).map_or(0., |last| {
            now.saturating_duration_since(last).as_secs_f32().min(0.1)
        });
        if let Some(mut journey) = self.journey {
            journey.elapsed += dt;
            self.position = journey.from + (journey.to - journey.from) * journey.progress();
            if journey.elapsed >= journey.duration {
                self.position = journey.to;
                self.journey = None;
            } else {
                self.journey = Some(journey);
            }
            return true;
        }
        if self.position >= duration && self.playing {
            if repeat && duration > 0. {
                self.rest += dt;
                if self.rest >= LOOP_REST {
                    self.jump(0., 0.5, true);
                }
            } else {
                self.playing = false;
                self.rate = 0.;
                self.velocity = 0.;
            }
            return self.playing;
        }
        let target = if self.playing { 1. } else { 0. };
        let old = self.rate;
        let displacement = self.rate - target;
        let omega = 24.;
        let slope = self.velocity + omega * displacement;
        let decay = (-omega * dt).exp();
        self.rate = target + (displacement + slope * dt) * decay;
        self.velocity = (slope - omega * (displacement + slope * dt)) * decay;
        self.position = (self.position + (old + self.rate) * dt * 0.5 * speed).clamp(0., duration);
        let active = self.playing || self.rate > 0.005 || self.velocity.abs() > 0.005;
        if !active {
            self.rate = 0.;
            self.velocity = 0.;
            self.last = None;
        }
        active
    }
}

fn default_steps() -> Vec<TerminalStep> {
    let ms = Duration::from_millis;
    let tone = TerminalOutput::text;
    vec![
        TerminalStep::new("cargo new acme --bin --vcs none")
            .with_output([
                tone("Creating a binary package in ~/acme", TerminalTone::Muted),
                TerminalOutput::Progress {
                    label: "Preparing workspace".into(),
                    duration: Some(ms(1430)),
                },
                tone("Created acme", TerminalTone::Success),
            ])
            .with_pause(ms(1930)),
        TerminalStep::new("cd acme && kui init && kui add button")
            .with_output([
                TerminalOutput::Task {
                    label: "Resolving button".into(),
                    done: Some("0.4s".into()),
                    duration: Some(ms(440)),
                },
                TerminalOutput::Task {
                    label: "Writing ui/components/button".into(),
                    done: Some("0.2s".into()),
                    duration: Some(ms(260)),
                },
                tone("Added Button and shared tokens", TerminalTone::Success),
            ])
            .with_pause(ms(1580)),
        TerminalStep::new("cargo run")
            .with_cwd("~/acme")
            .with_output([
                tone("Compiling acme v0.1.0", TerminalTone::Muted),
                tone("Finished dev profile", TerminalTone::Success),
                "Running target/debug/acme".into(),
                "Hello, world!".into(),
                tone(
                    "Guide: https://kense-labs.github.io/kui/",
                    TerminalTone::Muted,
                ),
            ])
            .with_pause(ms(1600)),
    ]
}

#[derive(Clone, Copy, PartialEq)]
struct Palette {
    ink: Hsla,
    muted: Hsla,
    flag: Hsla,
    path: Hsla,
    string: Hsla,
    package: Hsla,
    assignment: Hsla,
    accent: Hsla,
    success: Hsla,
    warning: Hsla,
    danger: Hsla,
}

impl Palette {
    fn new(p: Paint, accent: Hsla, dark: bool) -> Self {
        let [flag, path, string, package, assignment, warning] = if dark {
            [0xe4bc78, 0xb4a0e3, 0x96c69f, 0x7ac9eb, 0xb3bbee, 0xe8ae5e]
        } else {
            [0x875516, 0x6538a6, 0x226b3b, 0x006d91, 0x4f508f, 0x875516]
        };
        Self {
            ink: p.ink,
            muted: p.faint,
            flag: rgb(flag).into(),
            path: rgb(path).into(),
            string: rgb(string).into(),
            package: rgb(package).into(),
            assignment: rgb(assignment).into(),
            accent,
            success: p.success,
            warning: rgb(warning).into(),
            danger: p.danger,
        }
    }

    fn color(self, syntax: Syntax) -> Hsla {
        match syntax {
            Syntax::Ink => self.ink,
            Syntax::Muted | Syntax::Operator => self.muted,
            Syntax::Flag => self.flag,
            Syntax::Path => self.path,
            Syntax::String => self.string,
            Syntax::Package => self.package,
            Syntax::Assignment => self.assignment,
            Syntax::Success => self.success,
            Syntax::Warning => self.warning,
            Syntax::Danger => self.danger,
            Syntax::Accent | Syntax::Link => self.accent,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
struct Geometry {
    width: Pixels,
    em: Pixels,
    line: Pixels,
    font_family: &'static str,
    palette: Palette,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Phase {
    Static,
    Key(usize),
    Spinner(u8),
    Done,
}

impl TimedLine {
    fn phase(&self, time: f32) -> Phase {
        match &self.kind {
            LineKind::Prompt(prompt) => {
                Phase::Key(prompt.keys.partition_point(|(at, _)| *at <= time - self.at))
            }
            LineKind::Task { .. } if time < self.end => {
                Phase::Spinner((((time - self.at) / 0.08) as u8) % 10)
            }
            LineKind::Task { .. } => Phase::Done,
            _ => Phase::Static,
        }
    }

    fn text(&self, phase: Phase) -> LineText {
        match &self.kind {
            LineKind::Prompt(prompt) => {
                let Phase::Key(count) = phase else {
                    unreachable!("prompt phase")
                };
                let end = count.checked_sub(1).map_or(0, |i| prompt.keys[i].1);
                let indent = prompt.prefix.len();
                let mut text = LineText::new(
                    format!("{}{}", prompt.prefix, &prompt.command[..end]),
                    Syntax::Ink,
                );
                text.spans.push(Span {
                    range: 0..indent,
                    syntax: Syntax::Muted,
                });
                text.spans
                    .extend(
                        prompt
                            .spans
                            .iter()
                            .take_while(|s| s.range.start < end)
                            .map(|s| Span {
                                range: indent + s.range.start..indent + s.range.end.min(end),
                                syntax: s.syntax,
                            }),
                    );
                text
            }
            LineKind::Text(output) => {
                let (symbol, mark, base) = match output.tone {
                    TerminalTone::Default => ("", Syntax::Ink, Syntax::Ink),
                    TerminalTone::Muted => ("", Syntax::Muted, Syntax::Muted),
                    TerminalTone::Accent => ("", Syntax::Accent, Syntax::Accent),
                    TerminalTone::Success => ("✓ ", Syntax::Success, Syntax::Ink),
                    TerminalTone::Warning => ("! ", Syntax::Warning, Syntax::Ink),
                    TerminalTone::Danger => ("× ", Syntax::Danger, Syntax::Ink),
                };
                let mut text = LineText::new(format!("{symbol}{}", output.text), base);
                if !symbol.is_empty() {
                    text.spans.push(Span {
                        range: 0..symbol.len(),
                        syntax: mark,
                    });
                }
                text.links = output
                    .links
                    .iter()
                    .map(|link| TextLink {
                        range: link.range.start + symbol.len()..link.range.end + symbol.len(),
                        url: link.url.clone(),
                    })
                    .collect();
                text.spans.extend(text.links.iter().map(|link| Span {
                    range: link.range.clone(),
                    syntax: Syntax::Link,
                }));
                text
            }
            LineKind::Progress(label) => LineText::new(label.to_string(), Syntax::Muted),
            LineKind::Task { label, done } => {
                const BRAILLE: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
                let (marker, syntax) = if let Phase::Spinner(frame) = phase {
                    (BRAILLE[usize::from(frame)], Syntax::Accent)
                } else {
                    ('✓', Syntax::Success)
                };
                let mut text = LineText::new(format!("{marker} {label}"), Syntax::Ink);
                text.spans.push(Span {
                    range: 0..marker.len_utf8(),
                    syntax,
                });
                if phase == Phase::Done
                    && let Some(done) = done
                {
                    let start = text.text.len();
                    text.text.push(' ');
                    text.text.push_str(done);
                    text.spans.push(Span {
                        range: start..text.text.len(),
                        syntax: Syntax::Muted,
                    });
                }
                text
            }
            LineKind::Gap => LineText::new(String::new(), Syntax::Ink),
        }
    }
}

struct LineText {
    text: String,
    base: Syntax,
    spans: Vec<Span>,
    links: Vec<TextLink>,
}

impl LineText {
    fn new(text: String, base: Syntax) -> Self {
        Self {
            text,
            base,
            spans: Vec::new(),
            links: Vec::new(),
        }
    }

    fn shape(&self, range: Range<usize>, g: Geometry, w: &Window) -> ShapedLine {
        let mut runs = Vec::new();
        let font = font(g.font_family);
        let run = |len, syntax: Syntax| TextRun {
            len,
            font: font.clone(),
            color: g.palette.color(syntax),
            background_color: None,
            underline: (syntax == Syntax::Link).then_some(UnderlineStyle {
                thickness: px(1.),
                color: Some(g.palette.accent.opacity(0.62)),
                wavy: false,
            }),
            strikethrough: None,
        };
        let mut from = range.start;
        for span in &self.spans {
            let start = span.range.start.max(range.start);
            let end = span.range.end.min(range.end);
            if end <= start {
                continue;
            }
            if from < start {
                runs.push(run(start - from, self.base));
            }
            runs.push(run(end - start, span.syntax));
            from = end;
        }
        if from < range.end {
            runs.push(run(range.end - from, self.base));
        }
        w.text_system()
            .shape_line(self.text[range].to_owned().into(), g.em, &runs, None)
    }
}

#[derive(Clone)]
struct LinkPart {
    key: usize,
    x: Pixels,
    width: Pixels,
    url: SharedString,
}

#[derive(Clone)]
struct TerminalRow {
    shaped: ShapedLine,
    links: Vec<LinkPart>,
    meter: Option<Pixels>,
}

fn render_line(line: &TimedLine, phase: Phase, g: Geometry, w: &Window) -> Vec<TerminalRow> {
    let text = line.text(phase);
    let mut offset = 0;
    let mut rows = Vec::new();
    for paragraph in text.text.split('\n') {
        let span = offset..offset + paragraph.len();
        let whole = text.shape(span.clone(), g, w);
        let mut ranges = Vec::new();
        if whole.width > g.width {
            let mut start = 0;
            for (i, grapheme) in paragraph.grapheme_indices(true) {
                let advance = whole.x_for_index(i + grapheme.len()) - whole.x_for_index(start);
                if advance.abs() > g.width && i > start {
                    ranges.push(offset + start..offset + i);
                    start = i;
                }
            }
            ranges.push(offset + start..span.end);
        } else {
            ranges.push(span.clone());
        }
        for range in ranges {
            let shaped = if range == span {
                whole.clone()
            } else {
                text.shape(range.clone(), g, w)
            };
            let links = text
                .links
                .iter()
                .enumerate()
                .filter_map(|(key, link)| {
                    let start = link.range.start.max(range.start);
                    let end = link.range.end.min(range.end);
                    if start >= end {
                        return None;
                    }
                    let a = shaped.x_for_index(start - range.start);
                    let b = shaped.x_for_index(end - range.start);
                    Some(LinkPart {
                        key,
                        x: a.min(b),
                        width: (b - a).abs(),
                        url: link.url.clone(),
                    })
                })
                .collect();
            rows.push(TerminalRow {
                shaped,
                links,
                meter: None,
            });
        }
        offset = span.end + 1;
    }
    if matches!(line.kind, LineKind::Progress(_))
        && let Some(last) = rows.last_mut()
    {
        let start = last.shaped.width + px(10.);
        if g.width - start < px(96.) {
            rows.push(TerminalRow {
                shaped: ShapedLine::default(),
                links: Vec::new(),
                meter: Some(px(0.)),
            });
        } else {
            last.meter = Some(start);
        }
    }
    rows
}

fn progress_at(t: f32) -> f32 {
    let t = t.clamp(0., 1.);
    if t < 0.38 {
        0.58 * (1. - (1. - t / 0.38).powi(2))
    } else if t < 0.68 {
        0.58 + (t - 0.38) * 0.32
    } else {
        0.676 + (t - 0.68) * (0.324 / 0.32)
    }
}

#[derive(Clone, Copy)]
struct LayoutSample {
    width: Pixels,
    visible: bool,
}

struct CachedLine {
    phase: Phase,
    rows: Rc<Vec<TerminalRow>>,
}

#[derive(Clone, Copy)]
struct Drag {
    start: f32,
    resume: bool,
}

type Caption = Box<dyn Fn(&mut Window, &mut App) -> AnyElement>;
type Visibility = Box<dyn Fn(&App) -> bool>;

/// Keyboard and pointer actions report the final seek position or playback intent.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TerminalAction {
    Seek(f32),
    Play,
    Pause,
    Replay,
}

pub struct TypewriterTerminal {
    id: ElementId,
    title: SharedString,
    cwd: SharedString,
    symbol: SharedString,
    label: Option<SharedString>,
    steps: Vec<TerminalStep>,
    session: Session,
    playback: Playback,
    reduced: bool,
    speed: f32,
    repeat: bool,
    rows: usize,
    copyable: bool,
    copy: Option<Entity<CopyButton>>,
    font_family: &'static str,
    accent: Option<Hsla>,
    caption: Option<Caption>,
    style: Option<Box<dyn Fn(Div) -> Div>>,
    visibility: Option<Visibility>,
    layout: Rc<Cell<LayoutSample>>,
    scroll: Slide,
    geometry: Option<Geometry>,
    cached: Vec<Option<CachedLine>>,
    scrub_focus: FocusHandle,
    play_focus: FocusHandle,
    replay_focus: FocusHandle,
    link_focus: HashMap<(usize, usize), FocusHandle>,
    contact: Contact,
    track: Rc<Cell<Bounds<Pixels>>>,
    drag: Option<Drag>,
    announcement: Option<SharedString>,
}

impl EventEmitter<TerminalAction> for TypewriterTerminal {}

impl TypewriterTerminal {
    pub fn new(cx: &App) -> Self {
        let steps = default_steps();
        let session = Session::compile(&steps, "~", "$");
        Self {
            id: "typewriter-terminal".into(),
            title: "zsh".into(),
            cwd: "~".into(),
            symbol: "$".into(),
            label: None,
            steps,
            session,
            playback: Playback::new(),
            reduced: false,
            speed: 1.,
            repeat: true,
            rows: 11,
            copyable: true,
            copy: None,
            font_family: "monospace",
            accent: None,
            caption: None,
            style: None,
            visibility: None,
            layout: Rc::new(Cell::new(LayoutSample {
                width: px(0.),
                visible: true,
            })),
            scroll: Slide::new(0.),
            geometry: None,
            cached: Vec::new(),
            scrub_focus: cx.focus_handle().tab_stop(true),
            play_focus: cx.focus_handle().tab_stop(true),
            replay_focus: cx.focus_handle().tab_stop(true),
            link_focus: HashMap::new(),
            contact: Default::default(),
            track: Default::default(),
            drag: None,
            announcement: None,
        }
    }

    pub fn with_id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = id.into();
        self
    }

    pub fn with_steps(mut self, steps: Vec<TerminalStep>) -> Self {
        self.steps = steps;
        self.recompile();
        self
    }

    pub fn set_steps(&mut self, steps: Vec<TerminalStep>, cx: &mut Context<Self>) {
        self.steps = steps;
        self.recompile();
        self.drag = None;
        self.announcement = None;
        self.playback.jump(
            if self.reduced {
                self.session.duration
            } else {
                0.
            },
            0.,
            false,
        );
        cx.notify();
    }

    fn recompile(&mut self) {
        self.session = Session::compile(&self.steps, &self.cwd, &self.symbol);
        self.cached.clear();
        self.link_focus.clear();
        self.copy = None;
        self.scroll.reset(0.);
    }

    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = title.into();
        self
    }

    pub fn with_cwd(mut self, cwd: impl Into<SharedString>) -> Self {
        self.cwd = cwd.into();
        self.recompile();
        self
    }

    pub fn with_symbol(mut self, symbol: impl Into<SharedString>) -> Self {
        self.symbol = symbol.into();
        self.recompile();
        self
    }

    pub fn with_speed(mut self, speed: f32) -> Self {
        if speed.is_finite() && speed > 0. {
            self.speed = speed;
        }
        self
    }

    pub fn set_speed(&mut self, speed: f32, cx: &mut Context<Self>) {
        if speed.is_finite() && speed > 0. && self.speed != speed {
            self.speed = speed;
            cx.notify();
        }
    }

    pub fn with_paused(mut self, paused: bool) -> Self {
        self.playback.playing = !paused;
        self
    }

    pub fn with_loop(mut self, repeat: bool) -> Self {
        self.repeat = repeat;
        self
    }

    pub fn with_rows(mut self, rows: usize) -> Self {
        self.rows = rows.max(1);
        self
    }

    pub fn with_copyable(mut self, copyable: bool) -> Self {
        self.copyable = copyable;
        self
    }

    /// Sets the registered monospaced font family used for the transcript.
    pub fn with_font_family(mut self, family: &'static str) -> Self {
        self.font_family = family;
        self
    }

    pub fn with_accent(mut self, accent: Hsla) -> Self {
        self.accent = Some(accent);
        self
    }

    pub fn with_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn with_caption(
        mut self,
        caption: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        self.caption = Some(Box::new(caption));
        self
    }

    pub fn with_style(mut self, style: impl Fn(Div) -> Div + 'static) -> Self {
        self.style = Some(Box::new(style));
        self
    }

    pub fn with_visibility(mut self, visible: impl Fn(&App) -> bool + 'static) -> Self {
        self.visibility = Some(Box::new(visible));
        self
    }

    pub fn position(&self) -> f32 {
        self.playback.position
    }

    pub fn duration(&self) -> f32 {
        self.session.duration
    }

    pub fn is_paused(&self) -> bool {
        !self.playback.playing
    }

    pub fn set_paused(&mut self, paused: bool, cx: &mut Context<Self>) {
        let duration = self.session.duration;
        self.playback.playing = !paused && duration > 0. && !disabled_motion(cx);
        self.playback.last = None;
        if !paused && self.position() >= duration {
            self.playback
                .jump(if disabled_motion(cx) { duration } else { 0. }, 0.38, false);
        }
        cx.notify();
    }

    pub fn seek(&mut self, seconds: f32, cx: &mut Context<Self>) {
        if seconds.is_finite() {
            self.playback
                .jump(seconds.clamp(0., self.session.duration), 0., false);
            cx.notify();
        }
    }

    fn value_text(&self, lang: UiLanguage, at: f32) -> SharedString {
        let index = self.session.marks.partition_point(|mark| mark.at <= at);
        match index.checked_sub(1).and_then(|i| self.session.marks.get(i)) {
            Some(mark) if lang == UiLanguage::Chinese => format!(
                "第 {index} 步，共 {} 步。{}",
                self.session.marks.len(),
                mark.command
            )
            .into(),
            Some(mark) => format!(
                "Step {index} of {}. {}",
                self.session.marks.len(),
                mark.command
            )
            .into(),
            None if self.session.marks.is_empty() => lang.text("Empty session", "空会话").into(),
            None => lang.text("Start", "开始").into(),
        }
    }

    fn user_seek(&mut self, to: f32, duration: f32, cx: &mut Context<Self>) {
        self.playback
            .jump(to, if disabled_motion(cx) { 0. } else { duration }, false);
        self.announcement = Some(self.value_text(UiLanguage::current(cx), to));
        cx.emit(TerminalAction::Seek(to));
        cx.notify();
    }

    fn toggle(&mut self, cx: &mut Context<Self>) {
        let pause = self.playback.playing;
        self.set_paused(pause, cx);
        let lang = UiLanguage::current(cx);
        self.announcement = Some(
            if pause {
                lang.text("Playback paused", "已暂停播放")
            } else {
                lang.text("Playback resumed", "已恢复播放")
            }
            .into(),
        );
        cx.emit(if pause {
            TerminalAction::Pause
        } else {
            TerminalAction::Play
        });
    }

    fn replay(&mut self, cx: &mut Context<Self>) {
        let reduced = disabled_motion(cx);
        let target = if reduced { self.session.duration } else { 0. };
        self.playback.playing = !reduced && self.session.duration > 0.;
        self.playback
            .jump(target, if reduced { 0. } else { 0.38 }, false);
        self.announcement = Some(
            UiLanguage::current(cx)
                .text("Replaying the session", "正在重播会话")
                .into(),
        );
        cx.emit(TerminalAction::Replay);
        cx.notify();
    }

    fn key(&mut self, e: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        let position = self
            .playback
            .journey
            .map_or(self.position(), |jump| jump.to);
        let to = match e.keystroke.key.as_str() {
            "left" | "down" | "pageup" => Some(self.session.previous(position)),
            "right" | "up" | "pagedown" => Some(self.session.next(position)),
            "home" => Some(0.),
            "end" => Some(self.session.duration),
            "space" | "k" => {
                self.toggle(cx);
                None
            }
            _ => return,
        };
        if let Some(to) = to {
            self.user_seek(to, 0.26, cx);
        }
        w.prevent_default();
        cx.stop_propagation();
    }

    fn time_at(&self, x: Pixels) -> f32 {
        let b = self.track.get();
        ((x - b.left()) / b.size.width.max(px(1.))).clamp(0., 1.) * self.session.duration
    }

    fn begin(&mut self, x: Pixels, w: &mut Window, cx: &mut Context<Self>) {
        if self.drag.is_some() || self.session.duration <= 0. {
            return;
        }
        self.drag = Some(Drag {
            start: self.position(),
            resume: self.playback.playing,
        });
        self.playback.playing = false;
        self.seek(self.time_at(x), cx);
        self.scrub_focus.focus(w, cx);
        w.prevent_default();
        cx.stop_propagation();
    }

    fn drag_to(&mut self, x: Pixels, cx: &mut Context<Self>) {
        if self.drag.is_some() {
            self.seek(self.time_at(x), cx);
        }
    }

    fn end(&mut self, x: Pixels, cancelled: bool, cx: &mut Context<Self>) {
        let Some(drag) = self.drag.take() else { return };
        self.playback.playing = drag.resume && !disabled_motion(cx);
        if cancelled {
            self.playback.jump(drag.start, 0., false);
            cx.notify();
            return;
        }
        let at = self.time_at(x);
        self.playback.jump(at, 0., false);
        let width = f32::from(self.track.get().size.width.max(px(1.)));
        let snap = self
            .session
            .nearest_start(at, self.session.duration * SNAP_PIXELS / width);
        self.user_seek(snap, 0.18, cx);
    }
}

#[derive(Clone, Copy)]
struct ScreenFrame {
    geometry: Geometry,
    inset: Pixels,
    height: Pixels,
    time: f32,
    direct: bool,
    visible: bool,
    clear: Option<f32>,
}

impl TypewriterTerminal {
    fn screen(&mut self, frame: ScreenFrame, w: &Window, cx: &mut Context<Self>) -> Div {
        let geometry = frame.geometry;
        if self.geometry != Some(geometry) {
            self.geometry = Some(geometry);
            self.cached.clear();
            self.scroll.reset(0.);
        }
        self.cached.resize_with(self.session.lines.len(), || None);
        let visible_count = self
            .session
            .lines
            .partition_point(|line| line.at <= frame.time);
        let mut rows = Vec::new();
        for (index, line) in self.session.lines[..visible_count].iter().enumerate() {
            let phase = line.phase(frame.time);
            let cache = &mut self.cached[index];
            if cache.as_ref().is_none_or(|entry| entry.phase != phase) {
                *cache = Some(CachedLine {
                    phase,
                    rows: Rc::new(render_line(line, phase, geometry, w)),
                });
            }
            if let Some(entry) = cache {
                rows.extend((0..entry.rows.len()).map(|i| (index, i, entry.rows.clone())));
            }
        }

        let visible_height = frame.height - frame.inset * 2.;
        let target = f32::from((geometry.line * rows.len() as f32 - visible_height).max(px(0.)));
        let offset = if frame.direct || !frame.visible {
            self.scroll.reset(target);
            target
        } else {
            self.scroll.duration(target, 0.26, cx);
            self.scroll.get(w, cx)
        };
        let offset = px(offset) + frame.height * frame.clear.unwrap_or_default();
        let p = Paint::get(cx);
        let mut viewport = div().relative().w_full().h(frame.height).overflow_hidden();
        let mut visible_links = HashSet::new();
        for (row_index, (index, local, rows)) in rows.iter().enumerate() {
            let mut y = frame.inset + geometry.line * row_index as f32 - offset;
            if y + geometry.line <= px(0.) || y >= frame.height {
                continue;
            }
            let data = &rows[*local];
            let source = &self.session.lines[*index];
            let final_row = *index + 1 == visible_count && *local + 1 == rows.len();
            let caret = final_row && matches!(source.kind, LineKind::Prompt(_));
            let output = !matches!(source.kind, LineKind::Prompt(_) | LineKind::Gap);
            let rise = if frame.direct || !output {
                0.
            } else {
                (1. - (frame.time - source.at) / 0.16).clamp(0., 1.)
            };
            y += px(4. * rise);
            let fade_top = if offset > px(1.) {
                ((y + geometry.line * 0.75) / px(16.)).clamp(0., 1.)
            } else {
                1.
            };
            let alpha = (1. - rise) * fade_top;
            let cursor_alpha = if frame.direct || !self.playback.playing {
                0.46
            } else if frame.time < source.end {
                1.
            } else {
                0.26 + 0.68 * (0.5 + (frame.time * 5.7).cos() * 0.5)
            };
            let row = data.shaped.clone();
            let meter = data.meter.map(|x| {
                let elapsed = frame.time - source.at;
                let progress = progress_at(if source.end <= source.at {
                    1.
                } else {
                    elapsed / (source.end - source.at)
                });
                (x, progress)
            });
            viewport = viewport.child(
                canvas(
                    |_, _, _| (),
                    move |b, _, w, cx| {
                        let _ = row.paint(b.origin, geometry.line, TextAlign::Left, None, w, cx);
                        if caret {
                            let width = px(8.).min(b.size.width);
                            let x = row.width.min((b.size.width - width).max(px(0.)));
                            let h = geometry.em * 1.26;
                            let at = b.origin + point(x, (geometry.line - h) / 2.);
                            w.paint_quad(
                                fill(
                                    Bounds::new(at, size(width, h)),
                                    geometry.palette.accent.opacity(cursor_alpha),
                                )
                                .corner_radii(px(1.5)),
                            );
                        }
                        if let Some((x, progress)) = meter {
                            let width = (b.size.width - x - px(40.)).clamp(px(8.), px(96.));
                            let meter_at = b.origin + point(x, (geometry.line - px(4.)) / 2.);
                            w.paint_quad(
                                fill(Bounds::new(meter_at, size(width, px(4.))), p.line)
                                    .corner_radii(px(2.)),
                            );
                            if progress > 0. {
                                w.paint_quad(
                                    fill(
                                        Bounds::new(meter_at, size(width * progress, px(4.))),
                                        geometry.palette.accent,
                                    )
                                    .corner_radii(px(2.)),
                                );
                            }
                            let percent = format!("{:>3}%", (progress * 100.).round() as u32);
                            let run = TextRun {
                                len: percent.len(),
                                font: font(geometry.font_family),
                                color: p.secondary,
                                background_color: None,
                                underline: None,
                                strikethrough: None,
                            };
                            let percent = w.text_system().shape_line(
                                percent.into(),
                                geometry.em * 0.88,
                                &[run],
                                None,
                            );
                            let _ = percent.paint(
                                b.origin + point(x + width + px(8.), px(0.)),
                                geometry.line,
                                TextAlign::Right,
                                Some(px(30.)),
                                w,
                                cx,
                            );
                        }
                    },
                )
                .absolute()
                .left(frame.inset)
                .top(y)
                .w(geometry.width)
                .h(geometry.line)
                .opacity(alpha),
            );
            for link in &data.links {
                let key = (*index, link.key);
                let first = visible_links.insert(key);
                let url = link.url.clone();
                let mut element = div()
                    .id(SharedString::from(format!(
                        "terminal-link-{row_index}-{}",
                        link.key
                    )))
                    .absolute()
                    .left(frame.inset + link.x)
                    .top(y)
                    .w(link.width)
                    .h(geometry.line)
                    .rounded(px(2.))
                    .role(Role::Link)
                    .aria_label(url.clone())
                    .cursor_pointer()
                    .hover(move |s| s.bg(p.ink.opacity(0.06)))
                    .on_click(move |_, _, cx| cx.open_url(&url));
                if first {
                    let focus = self
                        .link_focus
                        .entry(key)
                        .or_insert_with(|| cx.focus_handle().tab_stop(true));
                    element = element
                        .track_focus(focus)
                        .focus_visible(move |s| s.border_1().border_color(p.focus));
                }
                viewport = viewport.child(element);
            }
        }
        viewport
    }

    fn scrubber(&self, cx: &mut Context<Self>, p: Paint, accent: Hsla) -> impl IntoElement {
        let duration = self.session.duration;
        let current = self.position();
        let value = if duration > 0. {
            current / duration
        } else {
            0.
        };
        let marks: Vec<_> = self
            .session
            .marks
            .iter()
            .skip(1)
            .map(|mark| mark.at / duration)
            .collect();
        let measured = self.track.clone();
        let track = canvas(
            move |bounds, _, _| measured.set(bounds),
            move |b, _, w, _| {
                let center_y = b.center().y;
                let track = Bounds::new(
                    point(b.left(), center_y - px(1.5)),
                    size(b.size.width, px(3.)),
                );
                w.paint_quad(fill(track, p.line).corner_radii(px(2.)));
                if value > 0. {
                    w.paint_quad(
                        fill(
                            Bounds::new(track.origin, size(track.size.width * value, px(3.))),
                            accent.opacity(0.74),
                        )
                        .corner_radii(px(2.)),
                    );
                }
                for value in marks {
                    let x = b.left() + b.size.width * value;
                    let mark =
                        Bounds::new(point(x - px(0.5), center_y - px(3.)), size(px(1.), px(6.)));
                    w.paint_quad(fill(mark, p.faint.opacity(0.48)).corner_radii(px(1.)));
                }
                let thumb = point(b.left() + b.size.width * value, center_y);
                let edge = Bounds::new(thumb - point(px(7.), px(7.)), size(px(14.), px(14.)));
                let face = Bounds::new(thumb - point(px(6.), px(6.)), size(px(12.), px(12.)));
                w.paint_quad(fill(edge, p.line.blend(p.ink.opacity(0.16))).corner_radii(px(7.)));
                w.paint_quad(fill(face, p.panel).corner_radii(px(6.)));
            },
        )
        .absolute()
        .left_0()
        .right_0()
        .top_0()
        .bottom_0();
        let lang = UiLanguage::current(cx);
        let label = lang.text("Session position", "会话位置");
        let mut surface = div()
            .relative()
            .flex_1()
            .min_w(px(44.))
            .h(px(44.))
            .id("terminal-scrubber")
            .role(Role::Slider)
            .aria_label(label)
            .aria_min_numeric_value(0.)
            .aria_max_numeric_value(f64::from(duration))
            .aria_numeric_value(f64::from(current))
            .aria_value(self.value_text(lang, current))
            .track_focus(&self.scrub_focus)
            .rounded(surface_radius(cx))
            .border_1()
            .border_color(transparent_black())
            .focus_visible(move |s| s.border_color(p.focus))
            .cursor_pointer()
            .on_key_down(cx.listener(Self::key))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|s, e: &MouseDownEvent, w, cx| {
                    if !w.last_input_was_touch() {
                        s.begin(e.position.x, w, cx);
                    }
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|s, e: &MouseUpEvent, _, cx| s.end(e.position.x, false, cx)),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|s, e: &MouseUpEvent, _, cx| s.end(e.position.x, false, cx)),
            )
            .child(track)
            .child(
                self.contact
                    .listen(cx, |s, e: &TouchDragEvent, w, cx| match e.phase {
                        TouchPhase::Started => s.begin(e.position.x, w, cx),
                        TouchPhase::Moved => s.drag_to(e.position.x, cx),
                        TouchPhase::Ended => s.end(e.position.x, false, cx),
                        TouchPhase::Cancelled => s.end(e.position.x, true, cx),
                    }),
            );
        if self.drag.is_some() {
            let move_drag = cx.listener(|s, e: &MouseMoveEvent, _, cx| {
                if e.pressed_button == Some(MouseButton::Left) {
                    s.drag_to(e.position.x, cx);
                }
            });
            surface = surface.child(
                canvas(
                    |_, _, _| (),
                    move |_, _, w, _| {
                        w.on_mouse_event(move |e: &MouseMoveEvent, phase, w, cx| {
                            if phase.capture() {
                                move_drag(e, w, cx);
                            }
                        });
                    },
                )
                .absolute()
                .size(px(0.)),
            );
        }
        surface
    }
}

fn terminal_time(seconds: f32) -> String {
    let seconds = seconds.floor() as u64;
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

fn playback_button(
    id: &'static str,
    label: &'static str,
    focus: &FocusHandle,
    p: Paint,
) -> Stateful<Div> {
    div()
        .size(px(44.))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .id(id)
        .role(Role::Button)
        .aria_label(label)
        .track_focus(focus)
        .cursor_pointer()
        .border_1()
        .border_color(transparent_black())
        .focus_visible(move |s| s.border_color(p.focus))
        .hover(move |s| s.bg(p.hover))
}

fn replay_icon(color: Hsla) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |b, _, w, _| {
            let center = b.center();
            let mut path = PathBuilder::stroke(px(1.35));
            let radius = px(6.);
            let start = center + point(px(-3.8), px(-4.6));
            path.move_to(start);
            path.arc_to(
                point(radius, radius),
                px(0.),
                true,
                true,
                center + point(px(-5.8), px(1.4)),
            );
            path.move_to(center + point(px(-6.5), px(-6.)));
            path.line_to(center + point(px(-6.5), px(-2.7)));
            path.line_to(center + point(px(-3.), px(-2.7)));
            if let Ok(path) = path.build() {
                w.paint_path(path, color);
            }
        },
    )
    .size(px(18.))
}

impl Render for TypewriterTerminal {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let reduced = disabled_motion(cx);
        if reduced != self.reduced {
            self.reduced = reduced;
            if reduced {
                self.playback.playing = false;
                self.playback.jump(self.session.duration, 0., false);
            }
        }
        let layout = self.layout.get();
        let visible = layout.visible && self.visibility.as_ref().is_none_or(|visible| visible(cx));
        let width = if layout.width > px(0.) {
            layout.width
        } else if w.viewport_size().width > px(0.) {
            w.viewport_size().width.min(px(MAX_WIDTH))
        } else {
            px(MAX_WIDTH)
        };
        let compact = width < px(560.);
        let inset = px(if compact { 16. } else { 20. });
        let (em, line) = if compact { (12., 21.) } else { (14., 24.5) };
        let p = Paint::get(cx);
        let accent = self.accent.unwrap_or(p.bright);
        let dark = Theme::global(cx).mode == crate::foundation::ThemeMode::Dark;
        let palette = Palette::new(p, accent, dark);
        let geometry = Geometry {
            width: (width - inset * 2. - px(2.)).max(px(1.)),
            em: px(em),
            line: px(line),
            font_family: self.font_family,
            palette,
        };
        let height = geometry.line * self.rows as f32 + inset * 2.;
        if visible && !reduced && self.drag.is_none() && self.session.duration > 0. {
            if self.playback.step(
                cx.background_executor().now(),
                self.session.duration,
                self.speed,
                self.repeat,
            ) {
                w.request_animation_frame();
            }
        } else {
            self.playback.last = None;
        }
        let clear = self
            .playback
            .journey
            .filter(|jump| jump.clearing)
            .map(Journey::progress);
        let frame = ScreenFrame {
            geometry,
            inset,
            height,
            time: if clear.is_some() {
                self.session.duration
            } else {
                self.position()
            },
            direct: reduced || self.drag.is_some() || self.playback.journey.is_some(),
            visible,
            clear,
        };
        let radius = surface_radius(cx);
        let lang = UiLanguage::current(cx);
        let copy_label = lang.text("Copy commands", "复制命令");
        let mut title = div()
            .relative()
            .flex()
            .items_center()
            .justify_center()
            .h(px(44.))
            .bg(p.panel)
            .child(
                div()
                    .px(px(96.))
                    .min_w_0()
                    .text_ellipsis()
                    .text_size(px(13.))
                    .text_color(p.secondary)
                    .font_weight(FontWeight::MEDIUM)
                    .child(self.title.clone()),
            )
            .child(
                div()
                    .absolute()
                    .left(px(12.))
                    .top(px(16.))
                    .flex()
                    .gap(px(7.))
                    .children((0..3).map(|_| {
                        div()
                            .size(px(11.))
                            .rounded_full()
                            .bg(p.ink.opacity(0.04))
                            .border_1()
                            .border_color(p.ink.opacity(0.06))
                    })),
            );
        if self.copyable && !self.session.commands.is_empty() {
            let commands = self.session.commands.clone();
            let copy = self.copy.get_or_insert_with(|| {
                cx.new(|_| CopyButton::new("terminal-copy", commands, copy_label).icon_only())
            });
            title = title.child(
                div()
                    .absolute()
                    .right(px(8.))
                    .top(px(4.))
                    .child(copy.clone()),
            );
        }
        let terminal = div()
            .flex()
            .flex_col()
            .w_full()
            .bg(p.panel)
            .rounded(radius)
            .border_1()
            .border_color(p.line)
            .overflow_hidden()
            .shadow(vec![
                BoxShadow::new(
                    px(0.),
                    px(22.),
                    black().opacity(if dark { 0.34 } else { 0.12 }),
                )
                .blur_radius(px(55.)),
            ])
            .child(title)
            .child(self.screen(frame, w, cx));
        let playing = self.playback.playing;
        let action = if playing {
            lang.text("Pause typing", "暂停打字")
        } else if self.position() >= self.duration() && !self.repeat {
            lang.text("Replay typing", "重新播放")
        } else {
            lang.text("Play typing", "播放打字")
        };
        let controls = div()
            .flex()
            .items_center()
            .gap(px(4.))
            .w_full()
            .h(px(44.))
            .child(
                playback_button("terminal-play", action, &self.play_focus, p)
                    .mr(px(8.))
                    .aria_toggled(playing.into())
                    .on_click(cx.listener(|s, _, _, cx| s.toggle(cx)))
                    .child(icon(
                        if playing { Glyph::Pause } else { Glyph::Play },
                        15.,
                        p.secondary,
                    )),
            )
            .child(self.scrubber(cx, p, accent))
            .when(width >= px(400.), |row| {
                row.child(
                    div()
                        .w(px(84.))
                        .flex_none()
                        .text_center()
                        .text_size(px(12.))
                        .text_color(p.faint)
                        .child(format!(
                            "{} / {}",
                            terminal_time(self.position()),
                            terminal_time(self.duration())
                        )),
                )
            })
            .child(
                playback_button(
                    "terminal-replay",
                    lang.text("Replay from the start", "从头播放"),
                    &self.replay_focus,
                    p,
                )
                .on_click(cx.listener(|s, _, _, cx| s.replay(cx)))
                .child(replay_icon(p.secondary)),
            );
        let name = self
            .label
            .clone()
            .unwrap_or_else(|| lang.text("Terminal session", "终端会话").into());
        let figure = div()
            .relative()
            .w_full()
            .max_w(px(MAX_WIDTH))
            .flex()
            .flex_col()
            .gap(px(16.))
            .id(self.id.clone())
            .role(Role::Figure)
            .aria_label(name)
            .aria_description(self.session.transcript.clone())
            .child(terminal)
            .child(controls)
            .children(self.announcement.as_ref().map(|message| {
                div()
                    .id("terminal-status")
                    .absolute()
                    .size(px(0.))
                    .role(Role::Status)
                    .aria_label(message.clone())
            }))
            .children(self.caption.as_ref().map(|caption| caption(w, cx)));
        let layout = self.layout.clone();
        let shell = div()
            .w_full()
            .max_w(px(MAX_WIDTH))
            .child(figure)
            .on_children_prepainted(move |bounds, w, _| {
                if let Some(bounds) = bounds.first() {
                    let viewport = Bounds::new(Point::default(), w.viewport_size());
                    let next = LayoutSample {
                        width: bounds.size.width,
                        visible: w.is_visible() && bounds.intersects(&viewport),
                    };
                    let old = layout.replace(next);
                    if next.width != old.width || next.visible != old.visible {
                        w.request_animation_frame();
                    }
                }
            });
        match &self.style {
            Some(style) => style(shell),
            None => shell,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Geometry, LineKind, Palette, Phase, Playback, Session, Syntax, TerminalOutput,
        TerminalStep, TerminalTone, TypewriterTerminal, default_steps, key_schedule, links,
        render_line, shell_spans,
    };
    use crate::foundation::Theme;
    use crate::primitives::family::Paint;
    use gpui::{TestAppContext, px};
    use std::time::Duration;
    use web_time::Instant;

    #[test]
    fn typing_preserves_whole_graphemes_and_shell_boundaries() {
        let command = "printf 'é👩‍💻' && cargo --locked ./Cargo.toml MODE=dev @kui/button";
        let keys = key_schedule(command);
        assert_eq!(keys, key_schedule(command));
        assert!(keys.windows(2).all(|window| window[0].0 < window[1].0));
        for part in ["é", "👩‍💻"] {
            let start = command.find(part).expect("test text");
            assert!(keys.iter().any(|(_, end)| *end == start + part.len()));
            assert!(
                !keys
                    .iter()
                    .any(|(_, end)| *end > start && *end < start + part.len())
            );
        }
        let kinds = shell_spans(command)
            .into_iter()
            .map(|span| (span.syntax, &command[span.range]))
            .collect::<Vec<_>>();
        assert!(kinds.contains(&(Syntax::String, "'é👩‍💻'")));
        assert!(kinds.contains(&(Syntax::Operator, "&&")));
        assert!(kinds.contains(&(Syntax::Flag, "--locked")));
        assert!(kinds.contains(&(Syntax::Path, "./Cargo.toml")));
        assert!(kinds.contains(&(Syntax::Assignment, "MODE=dev")));
        assert!(kinds.contains(&(Syntax::Package, "@kui/button")));
        let urls = links("guide (https://example.test/docs), javascript:alert(1)");
        assert_eq!(urls.len(), 1);
        assert_eq!(urls[0].url.as_ref(), "https://example.test/docs");
    }

    #[test]
    fn compiled_steps_keep_directory_and_event_order_after_zero_pause() {
        let steps = [
            TerminalStep::new("cargo check")
                .with_cwd("~/project")
                .with_pause(Duration::ZERO)
                .with_output([TerminalOutput::Progress {
                    label: "Build".into(),
                    duration: Some(Duration::ZERO),
                }]),
            TerminalStep::new("cargo run").with_output([TerminalOutput::Text {
                text: "Ready".into(),
                tone: TerminalTone::Success,
                delay: Some(Duration::ZERO),
            }]),
        ];
        let script = Session::compile(&steps, "~", "%");
        assert!(
            script
                .lines
                .windows(2)
                .all(|window| window[0].at <= window[1].at)
        );
        assert_eq!(script.commands.as_ref(), "cargo check\ncargo run");
        let LineKind::Prompt(prompt) = &script.lines.last().expect("final prompt").kind else {
            panic!("expected final prompt");
        };
        assert_eq!(prompt.prefix.as_ref(), "~/project % ");
        let first = script.next(0.);
        let next = script.next(first);
        assert_eq!(first, script.marks[0].at);
        assert_eq!(next, script.marks[1].at);
        assert_eq!(script.previous(next), first);
        assert_eq!(script.previous(first), 0.);
        assert_eq!(script.nearest_start(next + 0.08, 0.12), next);
        assert!(script.nearest_start(next + 0.13, 0.12) > next);
        let demo = Session::compile(&default_steps(), "~", "$");
        assert!((16. ..17.).contains(&demo.duration));
    }

    #[test]
    fn pause_eases_to_rest_and_interrupted_replay_keeps_its_current_position() {
        let start = Instant::now();
        let mut playback = Playback::new();
        let mut now = start;
        for _ in 0..80 {
            playback.step(now, 12., 1., false);
            now += Duration::from_millis(16);
        }
        let moving = playback.position;
        playback.playing = false;
        for _ in 0..80 {
            playback.step(now, 12., 1., false);
            now += Duration::from_millis(16);
        }
        let stopped = playback.position;
        assert!(stopped > moving && stopped - moving < 0.2);
        assert!(!playback.step(now, 12., 1., false));
        playback.jump(8., 0.26, false);
        playback.step(now, 12., 1., false);
        playback.step(now + Duration::from_millis(65), 12., 1., false);
        let mid = playback.position;
        assert!(mid > stopped && mid < 8.);
        playback.jump(0., 0.38, false);
        assert_eq!(playback.position, mid);
        let mut clock = now + Duration::from_millis(65);
        for _ in 0..40 {
            playback.step(clock, 12., 1., false);
            clock += Duration::from_millis(16);
        }
        assert_eq!(playback.position, 0.);
    }

    #[test]
    fn rendered_lines_wrap_long_words_and_reduced_motion_seeks_immediately() {
        let mut app = TestAppContext::single();
        app.update(|cx| {
            crate::init(cx);
            Theme::global_mut(cx).reduced_motion = true;
        });
        let (control, window) = app.add_window_view(|_, cx| {
            TypewriterTerminal::new(cx)
                .with_steps(vec![TerminalStep::new("cargo run").with_output([
                    "https://example.test/abcdefghijklmnopqrstuvwxyz".into(),
                ])])
        });
        window.run_until_parked();
        window.update(|w, cx| {
            let terminal = control.read(cx);
            assert_eq!(terminal.position(), terminal.duration());
            let output = terminal
                .session
                .lines
                .iter()
                .find(|line| matches!(line.kind, LineKind::Text(_)))
                .expect("output");
            let p = Paint::get(cx);
            let geometry = Geometry {
                width: px(86.),
                em: px(12.),
                line: px(21.),
                font_family: terminal.font_family,
                palette: Palette::new(p, p.bright, true),
            };
            let rows = render_line(output, Phase::Static, geometry, w);
            assert!(rows.len() > 1);
            assert!(rows.iter().all(|row| row.shaped.width <= px(86.5)));
            assert!(rows.iter().all(|row| !row.links.is_empty()));
        });
        window.update(|_, cx| {
            control.update(cx, |terminal, cx| terminal.user_seek(0., 0.26, cx));
            assert_eq!(control.read(cx).position(), 0.);
        });
    }
}
