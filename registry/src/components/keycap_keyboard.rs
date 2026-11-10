//! A mechanical keyboard preview with interactive keycaps and a typed text readout.
use crate::{
    foundation::{Theme, ThemeMode, surface_radius},
    primitives::{
        control::UiLanguage,
        direct_touch::Contact,
        family::{Paint, disabled_motion, ease},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc, time::Duration};
use unicode_segmentation::UnicodeSegmentation;
use web_time::Instant;

const KEY: u8 = 1;
const POINTER: u8 = 2;
const ACTIVATE: u8 = 4;
const PULSE: u8 = 8;
const MODIFIER: u8 = 16;
const DOWN: f32 = 0.035;
const RETURN: f32 = 0.2;
const GLOW: f32 = 0.7;
const FADE: f32 = 0.14;
const IDLE: f32 = 0.6;
const BLINK: f32 = 0.55;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum KeycapLayout {
    #[default]
    Compact,
    Letters,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum KeycapListen {
    #[default]
    Focus,
    Window,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Keycap {
    key: SharedString,
    span: f32,
}

impl Keycap {
    pub fn new(key: impl Into<SharedString>) -> Self {
        let key = key.into();
        let key = match key.as_ref() {
            " " => "space".into(),
            "return" => "enter".into(),
            "delete" => "backspace".into(),
            "capslock" | "caps lock" => "caps".into(),
            "cmd" | "command" | "meta" => "platform".into(),
            "ctrl" => "control".into(),
            "opt" | "option" => "alt".into(),
            _ => key,
        };
        Self { key, span: 1. }
    }

    pub fn with_span(mut self, span: f32) -> Self {
        if span.is_finite() {
            self.span = span.clamp(1., 24.);
        }
        self
    }

    fn legend(&self) -> &str {
        match self.key.as_ref() {
            "backspace" => "⌫",
            "enter" => "↵",
            "shift" => "⇧",
            "space" => "",
            "control" => "ctrl",
            "alt" => "opt",
            "platform" => "cmd",
            key => key,
        }
    }

    fn label(&self, lang: UiLanguage) -> SharedString {
        let label = match self.key.as_ref() {
            "backspace" => lang.text("Delete", "删除"),
            "enter" => lang.text("Return", "回车"),
            "space" => lang.text("Space", "空格"),
            "tab" => lang.text("Tab", "制表"),
            "caps" => lang.text("Caps lock", "大写锁定"),
            "shift" => lang.text("Shift", "Shift"),
            "control" => lang.text("Control", "Control"),
            "alt" => lang.text("Option", "Option"),
            "platform" => lang.text("Command", "Command"),
            _ => return self.key.clone(),
        };
        label.into()
    }
}

impl From<&str> for Keycap {
    fn from(key: &str) -> Self {
        Self::new(key)
    }
}

fn layout_rows(layout: KeycapLayout) -> Vec<Vec<Keycap>> {
    let row = |keys: &str| {
        keys.split_whitespace()
            .map(Keycap::from)
            .collect::<Vec<_>>()
    };
    let mut rows = match layout {
        KeycapLayout::Compact => [
            "` 1 2 3 4 5 6 7 8 9 0 - = backspace",
            "tab q w e r t y u i o p [ ] \\",
            "caps a s d f g h j k l ; ' enter",
            "shift z x c v b n m , . / shift",
            "control alt platform space platform alt control",
        ]
        .map(row)
        .to_vec(),
        KeycapLayout::Letters => [
            "q w e r t y u i o p",
            "a s d f g h j k l",
            "shift z x c v b n m backspace",
            "space",
        ]
        .map(row)
        .to_vec(),
    };
    let spans: &[&[(usize, f32)]] = match layout {
        KeycapLayout::Compact => &[
            &[(13, 2.)],
            &[(0, 1.5), (13, 1.5)],
            &[(0, 1.75), (12, 2.25)],
            &[(0, 2.25), (11, 2.75)],
            &[
                (0, 1.25),
                (1, 1.25),
                (2, 1.5),
                (3, 7.),
                (4, 1.5),
                (5, 1.25),
                (6, 1.25),
            ],
        ],
        KeycapLayout::Letters => &[&[], &[], &[(0, 1.5), (8, 1.5)], &[(0, 6.)]],
    };
    for (row, spans) in rows.iter_mut().zip(spans) {
        for &(index, span) in *spans {
            row[index].span = span;
        }
    }
    rows
}

/// New text from a physical key or a cap activation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeycapValueChanged(pub SharedString);

#[derive(Clone, Copy)]
struct Motion {
    held: u8,
    changed: Option<Instant>,
    start: f32,
    pulse_until: Option<Instant>,
}

impl Motion {
    fn new() -> Self {
        Self {
            held: 0,
            changed: None,
            start: 0.,
            pulse_until: None,
        }
    }

    fn elapsed(self, now: Instant) -> f32 {
        self.changed.map_or(GLOW + 1., |then| {
            now.saturating_duration_since(then).as_secs_f32()
        })
    }

    fn pose(self, now: Instant) -> f32 {
        let dt = self.elapsed(now);
        if self.held != 0 {
            return 1. + (self.start - 1.) * (-dt * 72.).exp();
        }
        if dt >= RETURN {
            return 0.;
        }
        let t = dt / RETURN;
        let rest = if t < 0.62 {
            -0.085 * ease(t / 0.62)
        } else {
            -0.085 * (1. - ease((t - 0.62) / 0.38))
        };
        self.start * (1. - ease(t)) + rest
    }

    fn glow(self, now: Instant) -> f32 {
        let dt = self.elapsed(now);
        if self.held != 0 {
            (dt / DOWN).clamp(0., 1.)
        } else {
            (1. - dt / GLOW).clamp(0., 1.).powf(2.5)
        }
    }

    fn change(&mut self, source: u8, down: bool, now: Instant) {
        let old = self.held;
        self.held = if down { old | source } else { old & !source };
        if (old == 0) != (self.held == 0) {
            let old = Self { held: old, ..*self };
            self.start = old.pose(now);
            self.changed = Some(now);
        }
    }

    fn pulse(&mut self, now: Instant) {
        self.change(PULSE, true, now);
        self.pulse_until = Some(now + Duration::from_millis(72));
    }

    fn tick(&mut self, now: Instant) -> bool {
        if let Some(until) = self.pulse_until.filter(|until| *until <= now) {
            self.pulse_until = None;
            self.change(PULSE, false, until);
        }
        let limit = if self.held == 0 { GLOW } else { 0.09 };
        self.pulse_until.is_some() || self.elapsed(now) < limit
    }
}

struct Cap {
    key: Keycap,
    motion: Motion,
    focus: FocusHandle,
    contact: Contact,
}

struct Metrics {
    unit: Pixels,
    gap: Pixels,
    travel: Pixels,
    padding: Pixels,
    radius: Pixels,
}

impl Metrics {
    fn new(width: Pixels, span: f32) -> Self {
        let unit = ((width - px(2.)) / (span * 1.14 - 0.14 + 0.784)).clamp(px(2.), px(54.));
        Self {
            unit,
            gap: unit * 0.14,
            travel: unit * 0.085,
            padding: unit * 0.392,
            radius: unit * 0.2,
        }
    }

    fn width(&self, span: f32) -> Pixels {
        self.unit * span + self.gap * (span - 1.)
    }
}

type WindowFilter = Box<dyn Fn(&Window, &App) -> bool>;
type BoardStyle = Box<dyn Fn(Div) -> Div>;
type Visibility = Box<dyn Fn(&App) -> bool>;

/// A focus-scoped keyboard by default, with optional window-wide observation.
pub struct KeycapKeyboard {
    id: ElementId,
    layout: KeycapLayout,
    custom: bool,
    rows: Vec<std::ops::Range<usize>>,
    caps: Vec<Cap>,
    max_span: f32,
    listen: KeycapListen,
    window_filter: Option<WindowFilter>,
    visibility: Option<Visibility>,
    value: SharedString,
    controlled: bool,
    max_length: usize,
    placeholder: Option<SharedString>,
    label: Option<SharedString>,
    show_output: bool,
    board_style: Option<BoardStyle>,
    root_focus: FocusHandle,
    output_focus: FocusHandle,
    last_width: Rc<Cell<Pixels>>,
    visible: Rc<Cell<bool>>,
    subscriptions: Vec<Subscription>,
    pointer: Option<usize>,
    virtual_shift: bool,
    last_input: Instant,
    changed_range: Option<std::ops::Range<usize>>,
    caret_bright: bool,
    caret_task: Option<Task<()>>,
}

impl EventEmitter<KeycapValueChanged> for KeycapKeyboard {}

impl KeycapKeyboard {
    pub fn new(cx: &App) -> Self {
        let mut keyboard = Self {
            id: "keycap-keyboard".into(),
            layout: Default::default(),
            custom: false,
            rows: Vec::new(),
            caps: Vec::new(),
            max_span: 1.,
            listen: Default::default(),
            window_filter: None,
            visibility: None,
            value: "".into(),
            controlled: false,
            max_length: 280,
            placeholder: None,
            label: None,
            show_output: true,
            board_style: None,
            root_focus: cx.focus_handle(),
            output_focus: cx.focus_handle().tab_stop(true),
            last_width: Default::default(),
            visible: Rc::new(Cell::new(true)),
            subscriptions: Vec::new(),
            pointer: None,
            virtual_shift: false,
            last_input: Instant::now(),
            changed_range: None,
            caret_bright: true,
            caret_task: None,
        };
        keyboard.rebuild(layout_rows(keyboard.layout), cx);
        keyboard
    }

    pub fn with_id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = id.into();
        self
    }

    pub fn with_layout(mut self, layout: KeycapLayout, cx: &App) -> Self {
        if !self.custom {
            self.layout = layout;
            self.rebuild(layout_rows(layout), cx);
        }
        self
    }

    pub fn with_rows(mut self, rows: Vec<Vec<Keycap>>, cx: &App) -> Self {
        self.custom = true;
        self.rebuild(rows, cx);
        self
    }

    pub fn set_layout(&mut self, layout: KeycapLayout, cx: &mut Context<Self>) {
        if self.custom || layout == self.layout {
            return;
        }
        self.layout = layout;
        self.rebuild(layout_rows(layout), cx);
        cx.notify();
    }

    pub fn with_default_value(mut self, value: impl Into<SharedString>) -> Self {
        self.controlled = false;
        self.value = keep_tail(value.into(), self.max_length);
        self
    }

    pub fn with_value(mut self, value: impl Into<SharedString>) -> Self {
        self.controlled = true;
        self.value = keep_tail(value.into(), self.max_length);
        self
    }

    pub fn with_max_length(mut self, max_length: usize) -> Self {
        self.max_length = max_length;
        self.value = keep_tail(self.value, max_length);
        self
    }

    pub fn with_listen(mut self, listen: KeycapListen) -> Self {
        self.listen = listen;
        self
    }

    /// Allows window observation while an application-owned, non-text control is focused.
    pub fn with_window_filter(mut self, filter: impl Fn(&Window, &App) -> bool + 'static) -> Self {
        self.window_filter = Some(Box::new(filter));
        self
    }

    pub fn with_visibility(mut self, visible: impl Fn(&App) -> bool + 'static) -> Self {
        self.visibility = Some(Box::new(visible));
        self
    }

    pub fn with_placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    pub fn with_show_output(mut self, show: bool) -> Self {
        self.show_output = show;
        self
    }

    pub fn with_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn with_style(mut self, style: impl Fn(Div) -> Div + 'static) -> Self {
        self.board_style = Some(Box::new(style));
        self
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn set_value(&mut self, value: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.value = keep_tail(value.into(), self.max_length);
        self.changed_range = None;
        cx.notify();
    }

    fn rebuild(&mut self, rows: Vec<Vec<Keycap>>, cx: &App) {
        self.rows.clear();
        self.caps.clear();
        self.pointer = None;
        self.virtual_shift = false;
        self.max_span = 1.;
        for row in rows.into_iter().filter(|row| !row.is_empty()) {
            let start = self.caps.len();
            self.max_span = self.max_span.max(row.iter().map(|key| key.span).sum());
            self.caps.extend(row.into_iter().map(|key| Cap {
                key,
                motion: Motion::new(),
                focus: cx.focus_handle().tab_stop(true),
                contact: Default::default(),
            }));
            self.rows.push(start..self.caps.len());
        }
    }

    fn cap_index(&self, key: &str) -> Option<usize> {
        let key = base_key(key);
        self.caps
            .iter()
            .position(|cap| cap.key.key.eq_ignore_ascii_case(key))
    }

    fn record(&mut self, input: Input<'_>, cx: &mut Context<Self>) -> bool {
        let mut value = self.value.to_string();
        let old_len = value.len();
        match input {
            Input::Backspace => {
                if let Some((i, _)) = value.grapheme_indices(true).next_back() {
                    value.truncate(i);
                }
            }
            Input::Insert(text) => value.push_str(text),
        }
        let added = value.len().saturating_sub(old_len);
        let next = keep_tail(value.into(), self.max_length);
        self.last_input = Instant::now();
        self.caret_bright = true;
        self.caret_task = None;
        if next == self.value {
            return false;
        }
        if !self.controlled {
            self.changed_range = (added > 0).then(|| next.len().saturating_sub(added)..next.len());
            self.value = next.clone();
        }
        cx.emit(KeycapValueChanged(next));
        cx.notify();
        true
    }

    fn physical(&mut self, stroke: &Keystroke, source: u8, cx: &mut Context<Self>) -> bool {
        let m = stroke.modifiers;
        if m.platform || m.control || m.alt {
            return false;
        }
        let key = stroke.key.as_str();
        let position = self.cap_index(key);
        if let Some(i) = position {
            let now = Instant::now();
            let motion = &mut self.caps[i].motion;
            if source == PULSE {
                motion.pulse(now);
            } else {
                motion.change(source, true, now);
            }
            cx.notify();
        }
        if let Some(input) = physical_input(stroke) {
            self.record(input, cx);
            return true;
        }
        // Tab keeps the platform's focus-navigation behavior.
        false
    }

    fn virtual_key(&mut self, index: usize, cx: &mut Context<Self>) {
        let key = self.caps[index].key.key.clone();
        match key.as_ref() {
            "shift" => self.virtual_shift = !self.virtual_shift,
            "backspace" => {
                self.record(Input::Backspace, cx);
            }
            "enter" => {
                self.record(Input::Insert("\n"), cx);
            }
            "space" => {
                self.record(Input::Insert(" "), cx);
            }
            key if printable(key) => {
                if self.virtual_shift {
                    let upper = shift_key(key);
                    self.record(Input::Insert(&upper), cx);
                    self.virtual_shift = false;
                } else {
                    self.record(Input::Insert(key), cx);
                }
            }
            _ => {}
        }
        cx.notify();
    }

    fn pointer_down(&mut self, index: usize, w: &mut Window, cx: &mut Context<Self>) {
        if self.pointer == Some(index) {
            return;
        }
        self.pointer_up(cx);
        self.pointer = Some(index);
        self.caps[index].focus.focus(w, cx);
        self.caps[index]
            .motion
            .change(POINTER, true, Instant::now());
        self.virtual_key(index, cx);
    }

    fn pointer_up(&mut self, cx: &mut Context<Self>) {
        if let Some(index) = self.pointer.take() {
            self.caps[index]
                .motion
                .change(POINTER, false, Instant::now());
            cx.notify();
        }
    }

    fn release_physical(&mut self, cx: &mut Context<Self>) {
        let now = Instant::now();
        for cap in &mut self.caps {
            cap.motion.change(KEY | MODIFIER | ACTIVATE, false, now);
        }
        self.caret_task = None;
        cx.notify();
    }

    fn watch(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if !self.subscriptions.is_empty() {
            return;
        }
        self.subscriptions
            .push(cx.observe_window_activation(w, |s, w, cx| {
                if !w.is_window_active() {
                    s.pointer_up(cx);
                    s.release_physical(cx);
                }
            }));
        self.subscriptions
            .push(cx.on_focus_out(&self.root_focus, w, |s, _, _, cx| {
                s.release_physical(cx);
            }));
        if self.listen != KeycapListen::Window {
            return;
        }
        let id = w.window_handle();
        self.subscriptions
            .push(cx.observe_keystrokes(move |s, event, w, cx| {
                if w.window_handle() == id
                    && event.action.is_none()
                    && event.input_preference != InputPreference::CharacterInput
                    && s.visible.get()
                    && s.visibility.as_ref().is_none_or(|f| f(cx))
                    && !s.root_focus.contains_focused(w, cx)
                    && (w.focused(cx).is_none()
                        || s.window_filter.as_ref().is_some_and(|f| f(w, cx)))
                {
                    s.physical(&event.keystroke, PULSE, cx);
                }
            }));
    }

    fn caret(&mut self, show: bool, reduce: bool, now: Instant, cx: &mut Context<Self>) {
        if !show || reduce {
            self.caret_task = None;
            self.caret_bright = true;
            return;
        }
        if self.caret_task.is_none() {
            let delay =
                (IDLE - now.saturating_duration_since(self.last_input).as_secs_f32()).max(0.);
            self.caret_task = Some(cx.spawn(async move |weak, cx| {
                cx.background_executor()
                    .timer(Duration::from_secs_f32(delay + BLINK))
                    .await;
                loop {
                    if weak
                        .update(cx, |s, cx| {
                            s.caret_bright = !s.caret_bright;
                            cx.notify();
                        })
                        .is_err()
                    {
                        return;
                    }
                    cx.background_executor()
                        .timer(Duration::from_secs_f32(BLINK))
                        .await;
                }
            }));
        }
    }
}

fn keep_tail(value: SharedString, max: usize) -> SharedString {
    if let Some((start, _)) = value.grapheme_indices(true).rev().nth(max) {
        let len = value[start..].graphemes(true).next().map_or(0, str::len);
        value[start + len..].to_owned().into()
    } else {
        value
    }
}

enum Input<'a> {
    Insert(&'a str),
    Backspace,
}

fn printable(key: &str) -> bool {
    !key.is_empty() && key.graphemes(true).count() == 1 && !key.chars().any(char::is_control)
}

fn physical_input(key: &Keystroke) -> Option<Input<'_>> {
    match key.key.as_str() {
        "backspace" => Some(Input::Backspace),
        "enter" => Some(Input::Insert("\n")),
        "space" => Some(Input::Insert(" ")),
        _ => key
            .key_char
            .as_deref()
            .or(Some(key.key.as_str()))
            .filter(|key| printable(key))
            .map(Input::Insert),
    }
}

const SHIFT: [(&str, &str); 21] = [
    ("`", "~"),
    ("1", "!"),
    ("2", "@"),
    ("3", "#"),
    ("4", "$"),
    ("5", "%"),
    ("6", "^"),
    ("7", "&"),
    ("8", "*"),
    ("9", "("),
    ("0", ")"),
    ("-", "_"),
    ("=", "+"),
    ("[", "{"),
    ("]", "}"),
    ("\\", "|"),
    (";", ":"),
    ("'", "\""),
    (",", "<"),
    (".", ">"),
    ("/", "?"),
];

fn shift_key(key: &str) -> String {
    SHIFT
        .iter()
        .find(|(normal, _)| *normal == key)
        .map_or_else(|| key.to_uppercase(), |(_, shifted)| (*shifted).to_owned())
}

fn base_key(key: &str) -> &str {
    SHIFT
        .iter()
        .find(|(_, shifted)| *shifted == key)
        .map_or(key, |(normal, _)| *normal)
}

#[derive(Clone, Copy)]
struct Palette {
    shell: Hsla,
    rim: Hsla,
    well: Hsla,
    top: Hsla,
    base: Hsla,
    edge: Hsla,
    lip: Hsla,
    legend: Hsla,
    focus: Hsla,
    accent: Hsla,
    pressed: Hsla,
}

impl Palette {
    fn new(cx: &App) -> Self {
        let p = Paint::get(cx);
        let dark = Theme::global(cx).mode == ThemeMode::Dark;
        let colors = if dark {
            [
                0x0e0e0e, 0x1b1b1b, 0x040404, 0x2b2b2b, 0x222222, 0x363636, 0x191919, 0xbfbfbf,
            ]
        } else {
            [
                0xe4e4e4, 0xd7d7d7, 0xd0d0d0, 0xffffff, 0xf2f2f2, 0xffffff, 0xd8d8d8, 0x4f4f4f,
            ]
        };
        let [shell, rim, well, top, base, edge, lip, legend] = colors.map(|c| rgb(c).into());
        Self {
            shell,
            rim,
            well,
            top,
            base,
            edge,
            lip,
            legend,
            focus: p.focus,
            accent: p.bright,
            pressed: p.inverse,
        }
    }
}

#[derive(Clone, Copy)]
struct Frame {
    now: Instant,
    reduce: bool,
    lang: UiLanguage,
    palette: Palette,
}

impl KeycapKeyboard {
    fn output(
        &self,
        metrics: &Metrics,
        frame: Frame,
        focused: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let p = Paint::get(cx);
        let height = (metrics.unit * 2.0384 + px(4.)).max(px(48.));
        let em = (metrics.unit * 0.784).clamp(px(15.), px(38.));
        let placeholder = self
            .placeholder
            .clone()
            .unwrap_or_else(|| frame.lang.text("Start typing", "开始输入").into());
        let empty = self.value.is_empty();
        let value = if empty {
            placeholder
        } else {
            self.value.clone()
        };
        let mut text = value.to_string();
        let caret = focused && (self.caret_bright || frame.reduce);
        let caret_start = text.len();
        if focused && !empty {
            text.push('▏');
        }
        let elapsed = frame
            .now
            .saturating_duration_since(self.last_input)
            .as_secs_f32();
        let mut ink = Vec::with_capacity(2);
        if !frame.reduce
            && !empty
            && elapsed < FADE
            && let Some(range) = &self.changed_range
        {
            ink.push((
                range.clone(),
                HighlightStyle {
                    color: Some(p.ink.opacity(ease(elapsed / FADE))),
                    ..Default::default()
                },
            ));
        }
        if text.len() != caret_start {
            ink.push((
                caret_start..text.len(),
                HighlightStyle {
                    color: Some(if caret { p.ink } else { transparent_black() }),
                    ..Default::default()
                },
            ));
        }
        div()
            .id("keycap-output")
            .role(Role::MultilineTextInput)
            .aria_label(frame.lang.text("Typed text", "已输入文字"))
            .aria_value(self.value.clone())
            .track_focus(&self.output_focus)
            .w_full()
            .h(height)
            .flex()
            .flex_col()
            .justify_end()
            .items_center()
            .pb(px(2.))
            .overflow_y_hidden()
            .rounded(surface_radius(cx))
            .border_1()
            .border_color(transparent_black())
            .focus_visible(move |s| s.border_color(p.focus))
            .cursor_text()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|s, _, w, cx| {
                    s.output_focus.focus(w, cx);
                    s.last_input = Instant::now();
                    s.caret_bright = true;
                    s.caret_task = None;
                    cx.notify();
                }),
            )
            .child(
                div()
                    .w_full()
                    .flex_none()
                    .text_center()
                    .whitespace_normal()
                    .text_size(em)
                    .line_height(em * 1.3)
                    .text_color(if empty { p.faint } else { p.ink })
                    .child(StyledText::new(SharedString::from(text)).with_highlights(ink)),
            )
    }

    fn cap(
        &self,
        index: usize,
        metrics: &Metrics,
        frame: Frame,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let cap = &self.caps[index];
        let motion = cap.motion;
        let p = frame.palette;
        let level = motion.glow(frame.now);
        let pressed = motion.held != 0 || (cap.key.key.as_ref() == "shift" && self.virtual_shift);
        let pose = if frame.reduce {
            0.
        } else {
            motion.pose(frame.now)
        };
        let light = if pressed { 0.86 } else { level * 0.22 };
        let top = p.top.blend(p.accent.opacity(light));
        let base = p.base.blend(p.accent.opacity(light));
        let legend = p.legend.blend(p.pressed.opacity(light));
        let width = metrics.width(cap.key.span);
        let unit = metrics.unit;
        let radius = metrics.radius.min(surface_radius(cx));
        let left = matches!(
            cap.key.key.as_ref(),
            "shift" | "enter" | "backspace" | "caps" | "tab" | "control" | "alt" | "platform"
        );
        let control_label = cap.key.label(frame.lang);
        let text: SharedString = cap.key.legend().to_owned().into();
        let toggled = cap.key.key.as_ref() == "shift";
        let visual = div()
            .absolute()
            .top(metrics.travel * pose)
            .left_0()
            .right_0()
            .h(unit)
            .flex()
            .items_center()
            .justify_center()
            .when(left, |d| d.justify_start().pl(unit * 0.20))
            .rounded(radius)
            .bg(linear_gradient(
                180.,
                linear_color_stop(top, 0.),
                linear_color_stop(base, 1.),
            ))
            .border_1()
            .border_color(p.edge.blend(p.accent.opacity(light * 0.34)))
            .text_color(legend)
            .text_size((unit * 0.3074).max(px(6.)))
            .child(
                div()
                    .absolute()
                    .left(px(1.))
                    .right(px(1.))
                    .bottom(px(1.))
                    .h((unit * 0.058).max(px(0.7)))
                    .rounded_b(radius)
                    .bg(p.lip.opacity(1. - light * 0.55)),
            )
            .child(text);
        div()
            .id(("keycap", index))
            .role(Role::Button)
            .aria_label(control_label)
            .when(toggled, |d| d.aria_toggled(self.virtual_shift.into()))
            .track_focus(&cap.focus)
            .relative()
            .w(width)
            .h(unit + metrics.travel)
            .flex_none()
            .cursor_pointer()
            .rounded(radius)
            .focus_visible(move |s| s.border_1().border_color(p.focus))
            .child(
                div()
                    .absolute()
                    .top(metrics.travel)
                    .left_0()
                    .right_0()
                    .h(unit)
                    .rounded(radius)
                    .bg(p.well)
                    .shadow(vec![
                        BoxShadow::new(px(0.), px(1.), black().opacity(0.22))
                            .blur_radius(unit * 0.17),
                    ]),
            )
            .when(level > 0.006, |d| {
                d.child(
                    div()
                        .absolute()
                        .top(metrics.travel)
                        .left_0()
                        .right_0()
                        .h(unit)
                        .rounded(radius)
                        .shadow(vec![
                            BoxShadow::new(px(0.), px(0.), p.accent.opacity(level * 0.33))
                                .blur_radius(unit * 0.40),
                        ]),
                )
            })
            .child(visual)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |s, _, w, cx| {
                    if !w.last_input_was_touch() {
                        s.pointer_down(index, w, cx);
                    }
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|s, _, _, cx| s.pointer_up(cx)),
            )
            .on_key_down(cx.listener(move |s, e: &KeyDownEvent, w, cx| {
                let m = e.keystroke.modifiers;
                if !m.platform
                    && !m.control
                    && !m.alt
                    && matches!(e.keystroke.key.as_str(), "enter" | "space")
                {
                    s.caps[index].motion.change(ACTIVATE, true, Instant::now());
                    if !e.is_held {
                        s.virtual_key(index, cx);
                    }
                    cx.stop_propagation();
                    w.prevent_default();
                }
            }))
            .on_key_up(cx.listener(move |s, e: &KeyUpEvent, _, cx| {
                if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                    s.caps[index].motion.change(ACTIVATE, false, Instant::now());
                    cx.notify();
                }
            }))
            .child(
                cap.contact
                    .listen(cx, move |s, event, w, cx| match event.phase {
                        TouchPhase::Started => s.pointer_down(index, w, cx),
                        TouchPhase::Ended | TouchPhase::Cancelled if s.pointer == Some(index) => {
                            s.pointer_up(cx)
                        }
                        _ => {}
                    }),
            )
            .into_any_element()
    }
}

impl Render for KeycapKeyboard {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.watch(w, cx);
        let now = Instant::now();
        let reduce = disabled_motion(cx);
        let lang = UiLanguage::current(cx);
        let p = Palette::new(cx);
        let frame = Frame {
            now,
            reduce,
            lang,
            palette: p,
        };
        let focused = self.root_focus.contains_focused(w, cx);
        let visible =
            self.visible.get() && w.is_visible() && self.visibility.as_ref().is_none_or(|f| f(cx));
        let display_caret = (focused || self.listen == KeycapListen::Window)
            && self.show_output
            && !self.value.is_empty();
        self.caret(visible && display_caret, reduce, now, cx);
        let mut motion = false;
        for cap in &mut self.caps {
            motion |= cap.motion.tick(now);
        }
        let text_fading = !reduce
            && self.changed_range.is_some()
            && now.saturating_duration_since(self.last_input).as_secs_f32() < FADE;
        if visible && (motion || text_fading) {
            w.request_animation_frame();
        }

        let window = w.viewport_size().width;
        let width = if self.last_width.get() > px(0.) {
            self.last_width.get().min(window.max(px(1.)))
        } else {
            window.min(px(960.))
        };
        let metrics = Metrics::new(width.max(px(60.)), self.max_span);
        let label = self
            .label
            .clone()
            .unwrap_or_else(|| lang.text("Keyboard", "键盘").into());
        let mut board = div()
            .w_full()
            .flex()
            .flex_col()
            .gap(metrics.gap)
            .p(metrics.padding)
            .bg(p.shell)
            .rounded(surface_radius(cx))
            .border_1()
            .border_color(p.rim)
            .shadow(vec![
                BoxShadow::new(px(0.), px(1.), black().opacity(0.08)).blur_radius(px(2.)),
            ])
            .children(self.rows.iter().cloned().map(|range| {
                div()
                    .flex()
                    .justify_center()
                    .gap(metrics.gap)
                    .children(range.map(|index| self.cap(index, &metrics, frame, cx)))
            }));
        if let Some(style) = &self.board_style {
            board = style(board);
        }
        let measured = self.last_width.clone();
        let on_screen = self.visible.clone();
        let viewport = Bounds::new(Point::default(), w.viewport_size());
        let mut root = div()
            .on_children_prepainted(move |children, w, _| {
                if let Some(&b) = children.first() {
                    let visible = b.intersects(&viewport);
                    if measured.replace(b.size.width) != b.size.width
                        || on_screen.replace(visible) != visible
                    {
                        w.request_animation_frame();
                    }
                }
            })
            .w_full()
            .max_w(px(960.))
            .flex()
            .flex_col()
            .gap(px(20.))
            .id(self.id.clone())
            .role(Role::Group)
            .aria_label(label)
            .track_focus(&self.root_focus)
            .tab_stop(false)
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                if s.physical(&e.keystroke, KEY, cx) {
                    cx.stop_propagation();
                    w.prevent_default();
                }
            }))
            .on_key_up(cx.listener(|s, e: &KeyUpEvent, _, cx| {
                if let Some(i) = s.cap_index(&e.keystroke.key) {
                    s.caps[i].motion.change(KEY, false, Instant::now());
                    cx.notify();
                }
            }))
            .on_modifiers_changed(cx.listener(|s, e: &ModifiersChangedEvent, _, cx| {
                let now = Instant::now();
                for (key, held) in [
                    ("shift", e.shift),
                    ("control", e.control),
                    ("alt", e.alt),
                    ("platform", e.platform),
                    ("caps", e.capslock.on),
                ] {
                    if let Some(i) = s.cap_index(key) {
                        s.caps[i].motion.change(MODIFIER, held, now);
                    }
                }
                cx.notify();
            }))
            .when(self.show_output, |d| {
                d.child(self.output(&metrics, frame, display_caret, cx))
            })
            .child(board);
        if self.pointer.is_some() {
            let release = cx.listener(|s, e: &MouseUpEvent, _, cx| {
                if e.button == MouseButton::Left {
                    s.pointer_up(cx);
                }
            });
            let move_event = cx.listener(|s, e: &MouseMoveEvent, w, cx| {
                if e.pressed_button != Some(MouseButton::Left) && !w.last_input_was_touch() {
                    s.pointer_up(cx);
                }
            });
            root = root.child(
                canvas(
                    |_, _, _| (),
                    move |_, _, w, _| {
                        w.on_mouse_event(move |e: &MouseUpEvent, phase, w, cx| {
                            if phase.capture() {
                                release(e, w, cx)
                            }
                        });
                        w.on_mouse_event(move |e: &MouseMoveEvent, phase, w, cx| {
                            if phase.capture() {
                                move_event(e, w, cx)
                            }
                        });
                    },
                )
                .absolute()
                .size(px(0.)),
            );
        }
        root
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ACTIVATE, Input, KEY, KeycapKeyboard, KeycapLayout, Metrics, Motion, POINTER, base_key,
        keep_tail, layout_rows, shift_key,
    };
    use gpui::{Keystroke, Modifiers, TestAppContext, px};
    use std::time::Duration;
    use web_time::Instant;

    #[test]
    fn the_compact_rows_share_a_fifteen_unit_grid() {
        let rows = layout_rows(KeycapLayout::Compact);
        assert_eq!(
            rows.iter().map(Vec::len).collect::<Vec<_>>(),
            [14, 14, 13, 12, 7]
        );
        assert!(
            rows.iter()
                .all(|row| (row.iter().map(|k| k.span).sum::<f32>() - 15.).abs() < 0.001)
        );
        let metrics = Metrics::new(px(521.52), 15.);
        assert!((f32::from(metrics.unit) - 29.28).abs() < 0.05);
        assert!((f32::from(metrics.width(7.)) - 229.6).abs() < 0.1);
        assert!(Metrics::new(px(320.), 15.).unit < metrics.unit);
    }

    #[test]
    fn a_second_source_keeps_the_cap_down_and_release_can_be_interrupted() {
        let t = Instant::now();
        let mut motion = Motion::new();
        assert_eq!(motion.glow(t), 0.);
        motion.change(KEY, true, t);
        assert!((motion.pose(t + Duration::from_millis(35)) - 0.92).abs() < 0.005);
        motion.change(POINTER, true, t + Duration::from_millis(45));
        motion.change(KEY, false, t + Duration::from_millis(60));
        assert_eq!(motion.held, POINTER);
        let release = t + Duration::from_millis(80);
        motion.change(POINTER, false, release);
        let reverse = release + Duration::from_millis(50);
        let before = motion.pose(reverse);
        motion.change(KEY, true, reverse);
        assert!((motion.pose(reverse) - before).abs() < 0.001);
        motion.change(KEY, false, reverse + Duration::from_millis(40));
        let end = reverse + Duration::from_millis(740);
        assert!(!motion.tick(end));
        assert_eq!(motion.pose(end), 0.);
        assert_eq!(motion.glow(end), 0.);
    }

    #[test]
    fn shifted_punctuation_uses_the_unshifted_physical_cap() {
        assert_eq!(base_key("!"), "1");
        assert_eq!(base_key("?"), "/");
        assert_eq!(shift_key("["), "{");
        assert_eq!(shift_key("a"), "A");
    }

    #[test]
    fn the_text_limit_and_delete_preserve_combined_graphemes() {
        let tail = keep_tail("Hi👩‍🚀e\u{301}".into(), 2);
        assert_eq!(tail.as_ref(), "👩‍🚀e\u{301}");
        assert!(keep_tail("abc".into(), 0).is_empty());
        let mut app = TestAppContext::single();
        app.update(|cx| crate::init(cx));
        let (control, window) =
            app.add_window_view(|_, cx| KeycapKeyboard::new(cx).with_default_value(tail));
        window.update(|_, cx| {
            control.update(cx, |control, cx| {
                assert!(control.record(Input::Backspace, cx));
                assert_eq!(control.value(), "👩‍🚀");
                assert!(control.record(Input::Insert("\n"), cx));
                assert!(control.record(Input::Insert("x"), cx));
                control.set_layout(KeycapLayout::Letters, cx);
                assert_eq!(control.value(), "👩‍🚀\nx");
            });
        });
    }

    #[test]
    fn virtual_shift_is_single_use_and_shortcuts_pass_through() {
        let mut app = TestAppContext::single();
        app.update(|cx| crate::init(cx));
        let (control, window) = app.add_window_view(|_, cx| KeycapKeyboard::new(cx));
        window.update(|_, cx| {
            control.update(cx, |control, cx| {
                let shift = control.cap_index("shift").expect("preset Shift");
                let a = control.cap_index("a").expect("preset A");
                control.virtual_key(shift, cx);
                control.virtual_key(a, cx);
                control.virtual_key(a, cx);
                assert_eq!(control.value(), "Aa");
                let shortcut = Keystroke {
                    modifiers: Modifiers {
                        platform: true,
                        ..Default::default()
                    },
                    key: "a".into(),
                    key_char: None,
                };
                assert!(!control.physical(&shortcut, ACTIVATE, cx));
                assert_eq!(control.value(), "Aa");
            });
        });
    }
}
