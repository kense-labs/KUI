//! Record a shortcut, preview held keys and resolve conflicting bindings before committing.
use crate::primitives::{
    family::{Paint, PhaseClock, disabled_motion, ease},
    form_chrome::*,
    glyph::Glyph,
};
use gpui::{prelude::*, *};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShortcutChanged(pub Option<String>);

/// Modifier labels and matching convention used for the currently displayed platform.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShortcutPlatform {
    Mac,
    Windows,
}
impl ShortcutPlatform {
    pub fn detected() -> Self {
        #[cfg(target_family = "wasm")]
        {
            let apple = js_sys::Reflect::get(&js_sys::global(), &"navigator".into())
                .ok()
                .and_then(|n| js_sys::Reflect::get(&n, &"platform".into()).ok())
                .and_then(|p| p.as_string())
                .is_some_and(|p| ["Mac", "iPhone", "iPad"].iter().any(|s| p.contains(s)));
            if apple { Self::Mac } else { Self::Windows }
        }
        #[cfg(not(target_family = "wasm"))]
        if cfg!(target_os = "macos") {
            Self::Mac
        } else {
            Self::Windows
        }
    }
    fn tokens(self, chord: &str) -> Vec<String> {
        let mut ctrl = false;
        let mut alt = false;
        let mut shift = false;
        let mut cmd = false;
        let mut name = None;
        for key in chord.split('+').filter(|s| !s.is_empty()) {
            match key.trim().to_ascii_lowercase().as_str() {
                "ctrl" | "control" | "⌃" => ctrl = true,
                "alt" | "option" | "⌥" => alt = true,
                "shift" | "⇧" => shift = true,
                "cmd" | "command" | "mod" | "⌘" => {
                    if self == Self::Mac {
                        cmd = true
                    } else {
                        ctrl = true
                    }
                }
                "meta" | "win" | "super" => cmd = true,
                k => {
                    name = Some(match k {
                        "enter" | "return" => if self == Self::Mac { "↩" } else { "Enter" }.into(),
                        "tab" => if self == Self::Mac { "⇥" } else { "Tab" }.into(),
                        "backspace" => if self == Self::Mac {
                            "⌫"
                        } else {
                            "Backspace"
                        }
                        .into(),
                        "delete" => if self == Self::Mac { "⌦" } else { "Del" }.into(),
                        "space" => "Space".into(),
                        "escape" => "Esc".into(),
                        "up" => "↑".into(),
                        "down" => "↓".into(),
                        "left" => "←".into(),
                        "right" => "→".into(),
                        "period" => ".".into(),
                        "comma" => ",".into(),
                        _ => k.to_uppercase(),
                    })
                }
            }
        }
        let mac = self == Self::Mac;
        let mut tokens = Vec::new();
        for (on, label) in [
            (ctrl, if mac { "⌃" } else { "Ctrl" }),
            (alt, if mac { "⌥" } else { "Alt" }),
            (shift, if mac { "⇧" } else { "Shift" }),
            (cmd, if mac { "⌘" } else { "Win" }),
        ] {
            if on {
                tokens.push(label.to_owned())
            }
        }
        tokens.extend(name);
        tokens
    }
    pub fn matches(self, chord: &str, other: &str) -> bool {
        !chord.is_empty() && self.tokens(chord) == self.tokens(other)
    }
    fn chord_for_key(self, modifiers: Modifiers, key: &str) -> String {
        let mut parts = Vec::new();
        if modifiers.control {
            parts.push(if self == Self::Windows { "Mod" } else { "Ctrl" })
        }
        if modifiers.platform {
            parts.push(if self == Self::Mac { "Cmd" } else { "Win" })
        }
        if modifiers.alt {
            parts.push("Alt")
        }
        if modifiers.shift {
            parts.push("Shift")
        }
        let key = key.to_uppercase();
        if !key.is_empty() {
            parts.push(&key)
        }
        parts.join("+")
    }
    pub fn contains_modifiers(self, chord: &str, held: Modifiers) -> bool {
        let tokens = self.tokens(chord);
        self.tokens(&self.chord_for_key(held, ""))
            .iter()
            .all(|k| tokens.contains(k))
    }
    pub fn matches_keystroke(self, chord: &str, key: &Keystroke) -> bool {
        self.matches(chord, &self.chord_for_key(key.modifiers, &key.key))
    }
    pub fn spoken(self, chord: &str) -> String {
        self.tokens(chord)
            .into_iter()
            .map(|k| match k.as_str() {
                "⌘" => "Command".into(),
                "⌃" | "Ctrl" => "Control".into(),
                "⌥" => "Option".into(),
                "⇧" => "Shift".into(),
                "⇥" => "Tab".into(),
                "↩" => "Return".into(),
                "⌫" => "Backspace".into(),
                "⌦" => "Delete".into(),
                "." => "Period".into(),
                "," => "Comma".into(),
                _ => k,
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
    pub fn keycaps(self, chord: &str, small: bool, held: Modifiers, cx: &App) -> Div {
        let pressed = self.tokens(&self.chord_for_key(held, ""));
        let p = Paint::get(cx);
        self.tokens(chord)
            .into_iter()
            .fold(row().flex_none().gap(px(4.)), |r, key| {
                let down = pressed.contains(&key);
                r.child(cap(key, small, down, false, 1., p, cx))
            })
    }
}

// Draw system key symbols on both renderers, including environments without a symbol fallback font.
fn key_symbol(key: &str, ink: Hsla) -> AnyElement {
    let symbol = key.to_owned();
    if !matches!(
        key,
        "⌘" | "⌥" | "⇧" | "⌃" | "↩" | "⇥" | "⌫" | "⌦" | "↑" | "↓" | "←" | "→"
    ) {
        return div().child(symbol).into_any_element();
    }
    // Keep the drawn modifier optically the same height as the adjacent 12px text.
    const SIDE: f32 = 11.;
    canvas(
        |_, _, _| (),
        move |bounds, _, w, _| {
            let pt = |x: f32, y: f32| {
                point(
                    bounds.left() + px(x * SIDE / 13.),
                    bounds.top() + px(y * SIDE / 13.),
                )
            };
            let mut path = PathBuilder::stroke(px(1.1));
            let line = |p: &mut PathBuilder, points: &[(f32, f32)]| {
                p.move_to(pt(points[0].0, points[0].1));
                for &(x, y) in &points[1..] {
                    p.line_to(pt(x, y))
                }
            };
            match symbol.as_str() {
                "⌘" => {
                    line(
                        &mut path,
                        &[(4., 4.), (9., 4.), (9., 9.), (4., 9.), (4., 4.)],
                    );
                    for (x, y, sx, sy) in [
                        (4., 4., -1., -1.),
                        (9., 4., 1., -1.),
                        (9., 9., 1., 1.),
                        (4., 9., -1., 1.),
                    ] {
                        path.move_to(pt(x, y));
                        path.line_to(pt(x + 1.5 * sx, y));
                        path.curve_to(pt(x + 3. * sx, y + 1.5 * sy), pt(x + 3. * sx, y));
                        path.curve_to(pt(x + 1.5 * sx, y + 3. * sy), pt(x + 3. * sx, y + 3. * sy));
                        path.curve_to(pt(x, y + 1.5 * sy), pt(x, y + 3. * sy));
                        path.line_to(pt(x, y));
                    }
                }
                "⌥" => {
                    line(&mut path, &[(0.5, 3.), (4., 3.), (9., 10.), (12.5, 10.)]);
                    line(&mut path, &[(8., 3.), (12.5, 3.)]);
                }
                "⇧" => line(
                    &mut path,
                    &[
                        (1., 6.),
                        (6.5, 1.),
                        (12., 6.),
                        (9., 6.),
                        (9., 11.5),
                        (4., 11.5),
                        (4., 6.),
                        (1., 6.),
                    ],
                ),
                "⌃" => line(&mut path, &[(1., 9.), (6.5, 4.), (12., 9.)]),
                "↩" => {
                    line(&mut path, &[(12., 2.), (12., 8.), (1., 8.)]);
                    line(&mut path, &[(4.5, 4.5), (1., 8.), (4.5, 11.5)])
                }
                "⇥" | "→" => {
                    line(&mut path, &[(0.5, 6.5), (10., 6.5)]);
                    line(&mut path, &[(6., 2.5), (10., 6.5), (6., 10.5)]);
                    if symbol == "⇥" {
                        line(&mut path, &[(12., 2.), (12., 11.)])
                    }
                }
                "←" => {
                    line(&mut path, &[(12., 6.5), (1., 6.5)]);
                    line(&mut path, &[(5., 2.5), (1., 6.5), (5., 10.5)])
                }
                "↑" => {
                    line(&mut path, &[(6.5, 12.), (6.5, 1.)]);
                    line(&mut path, &[(2.5, 5.), (6.5, 1.), (10.5, 5.)])
                }
                "↓" => {
                    line(&mut path, &[(6.5, 1.), (6.5, 12.)]);
                    line(&mut path, &[(2.5, 8.), (6.5, 12.), (10.5, 8.)])
                }
                _ => {
                    let flip = |x: f32| if symbol == "⌦" { 13. - x } else { x };
                    line(
                        &mut path,
                        &[
                            (flip(0.5), 6.5),
                            (flip(4.5), 2.),
                            (flip(12.), 2.),
                            (flip(12.), 11.),
                            (flip(4.5), 11.),
                            (flip(0.5), 6.5),
                        ],
                    );
                    line(&mut path, &[(flip(6.), 5.), (flip(9.), 8.)]);
                    line(&mut path, &[(flip(9.), 5.), (flip(6.), 8.)]);
                }
            }
            if let Ok(path) = path.build() {
                w.paint_path(path, ink)
            }
        },
    )
    .size(px(SIDE))
    .into_any_element()
}
fn cap(key: String, small: bool, pressed: bool, warn: bool, t: f32, p: Paint, cx: &App) -> Div {
    let side = if small { 22. } else { 26. };
    let warning = crate::foundation::Theme::global(cx).tokens.colors.warning;
    let border = if warn {
        p.faint.blend(warning.opacity(0.55))
    } else if pressed {
        p.secondary
    } else {
        p.faint.opacity(0.65)
    };
    let face = div()
        .absolute()
        .inset(px((1. - t) * side * 0.14))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .pb(px(1.))
        .rounded(crate::foundation::surface_radius(cx))
        .border_1()
        .border_b(px(if pressed { 1. } else { 2. }))
        .border_color(border)
        .bg(if warn {
            p.panel.blend(warning.opacity(0.12))
        } else if pressed {
            p.hover
        } else {
            p.panel.blend(p.ink.opacity(0.025))
        })
        .text_color(p.ink)
        .text_size(px(12.))
        .line_height(px(14.))
        .font_weight(FontWeight::MEDIUM)
        .opacity(t)
        .child(key_symbol(&key, p.ink));
    div()
        .relative()
        .flex_none()
        .min_w(px(side))
        .h(px(side))
        .px(px(if small { 6. } else { 8. }))
        .text_size(px(12.))
        .font_weight(FontWeight::MEDIUM)
        .line_height(px(14.))
        .top(px(4. * (1. - t)
            + if pressed && !disabled_motion(cx) {
                1.
            } else {
                0.
            }))
        .child(div().opacity(0.).child(key_symbol(&key, p.ink)))
        .child(face)
}
pub struct ShortcutRecorder {
    label: SharedString,
    value: Option<String>,
    initial: Option<String>,
    conflicts: Vec<(String, String)>,
    candidate: Option<String>,
    recording: bool,
    message: SharedString,
    focus: FocusHandle,
    platform: ShortcutPlatform,
    held: Modifiers,
    caps: PhaseClock,
    previous: Vec<String>,
    displayed: Option<Vec<String>>,
    transition: PhaseClock,
    status: FormReveal,
    last_message: SharedString,
    listen: PhaseClock,
    blur: Option<Subscription>,
}
impl EventEmitter<ShortcutChanged> for ShortcutRecorder {}
impl ShortcutRecorder {
    pub fn new(
        label: impl Into<SharedString>,
        initial: Option<String>,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            label: label.into(),
            value: initial.clone(),
            initial,
            conflicts: vec![],
            candidate: None,
            recording: false,
            message: "".into(),
            focus: cx.focus_handle().tab_stop(true),
            platform: ShortcutPlatform::detected(),
            held: Modifiers::default(),
            caps: Default::default(),
            previous: vec![],
            displayed: None,
            transition: Default::default(),
            status: FormReveal::new(false),
            last_message: "".into(),
            listen: Default::default(),
            blur: None,
        }
    }
    pub fn with_conflicts(mut self, keys: Vec<String>) -> Self {
        self.conflicts = keys.into_iter().map(|k| (k.clone(), k)).collect();
        self
    }
    pub fn set_bindings(&mut self, bindings: Vec<(String, String)>) {
        self.conflicts = bindings
    }
    pub fn set_platform(&mut self, platform: ShortcutPlatform, cx: &mut Context<Self>) {
        self.platform = platform;
        self.cancel(cx)
    }
    pub fn set_value(&mut self, value: Option<String>, cx: &mut Context<Self>) {
        self.value = value;
        self.cancel(cx);
        self.caps.restart(cx)
    }
    pub fn value(&self) -> Option<&str> {
        self.value.as_deref()
    }
    pub fn is_recording(&self) -> bool {
        self.recording
    }
    fn start(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        self.cancel(cx);
        self.recording = true;
        self.listen.restart(cx);
        self.focus.focus(w, cx);
        cx.notify()
    }
    fn cancel(&mut self, cx: &mut Context<Self>) {
        self.recording = false;
        self.candidate = None;
        self.held = Modifiers::default();
        self.message = "".into();
        cx.notify()
    }
    fn commit(&mut self, key: Option<String>, cx: &mut Context<Self>) {
        let changed = self.value != key;
        self.set_value(key.clone(), cx);
        if changed {
            cx.emit(ShortcutChanged(key))
        }
    }
    fn restore(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let key = self.initial.clone();
        if let Some(key) = key {
            self.propose(key, cx)
        } else {
            self.commit(None, cx)
        }
        self.focus.focus(w, cx)
    }
    fn propose(&mut self, key: String, cx: &mut Context<Self>) {
        if self
            .value
            .as_deref()
            .is_some_and(|old| self.platform.matches(old, &key))
        {
            self.cancel(cx);
            return;
        }
        let owned = self
            .conflicts
            .iter()
            .find(|(k, _)| self.platform.matches(k, &key))
            .map(|(_, name)| name.as_str());
        let reserved = [
            "W", "T", "N", "Q", "L", "R", "C", "V", "X", "Z", "A", "Shift+T",
        ]
        .iter()
        .any(|end| self.platform.matches(&format!("Mod+{end}"), &key));
        if owned.is_some() || reserved {
            self.message = if let Some(name) = owned {
                format!("{} {name}", tr(cx, "Already used by", "已被占用 ·"))
            } else {
                tr(cx, "Reserved by the system", "系统保留的快捷键").into()
            }
            .into();
            self.recording = false;
            self.candidate = Some(key);
            self.held = Modifiers::default();
            cx.notify()
        } else {
            self.commit(Some(key), cx)
        }
    }
    fn key(&mut self, e: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        let k = e.keystroke.key.to_ascii_lowercase();
        let m = e.keystroke.modifiers;
        if !self.recording {
            if m == Modifiers::default() && matches!(k.as_str(), "enter" | "space") && !e.is_held {
                self.start(w, cx)
            } else if k == "escape" && self.candidate.is_some() {
                self.cancel(cx)
            } else if m == Modifiers::default()
                && matches!(k.as_str(), "backspace" | "delete")
                && self.value.is_some()
            {
                self.commit(None, cx)
            } else {
                return;
            }
        } else if k == "tab" && !(m.control || m.platform || m.alt) {
            self.cancel(cx);
            return;
        } else if m == Modifiers::default() && k == "escape" {
            self.cancel(cx)
        } else if m == Modifiers::default() && matches!(k.as_str(), "backspace" | "delete") {
            self.commit(None, cx)
        } else if e.is_held
            || matches!(
                k.as_str(),
                "control"
                    | "shift"
                    | "alt"
                    | "option"
                    | "meta"
                    | "cmd"
                    | "super"
                    | "capslock"
                    | "fn"
                    | "process"
                    | "dead"
            )
        {
            self.held = m;
            cx.notify()
        } else if !(m.control
            || m.platform
            || m.alt
            || k.strip_prefix('f')
                .and_then(|n| n.parse::<u8>().ok())
                .is_some_and(|n| (1..=24).contains(&n)))
        {
            self.message = tr(cx, "Include Command, Control, or Alt", "请同时按住修饰键").into();
            cx.notify()
        } else {
            let chord = self.platform.chord_for_key(m, &k);
            self.propose(chord, cx)
        }
        cx.stop_propagation();
        w.prevent_default()
    }
}
impl Render for ShortcutRecorder {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.blur.is_none() {
            self.blur = Some(cx.on_blur(&self.focus, w, |s, _, cx| {
                if s.recording {
                    s.cancel(cx)
                }
            }));
        }
        let p = Paint::get(cx);
        let reduced = disabled_motion(cx);
        let pending = self.candidate.is_some();
        let sel = self
            .candidate
            .as_deref()
            .or(self.value.as_deref())
            .unwrap_or("");
        let live = self.platform.chord_for_key(self.held, "");
        let tokens = self
            .platform
            .tokens(if self.recording { &live } else { sel });
        if self.displayed.as_ref() != Some(&tokens)
            && let Some(prior) = self.displayed.replace(tokens.clone())
        {
            self.previous = prior;
            self.transition.restart(cx);
        }
        let elapsed = self.transition.time(0.43, w, cx);
        let mut keys = row().relative().gap(px(4.));
        if self.recording {
            let alpha = if reduced {
                1.
            } else {
                w.request_animation_frame();
                0.675 + 0.325 * (self.listen.age(cx) * std::f32::consts::TAU / 1.4).cos()
            };
            keys = keys.child(
                div()
                    .size(px(7.))
                    .ml(px(3.))
                    .mr(px(2.))
                    .rounded_full()
                    .bg(p.ink.opacity(alpha)),
            );
        }
        if tokens.is_empty() {
            keys = keys.child(hint(
                if self.recording {
                    tr(cx, "Press keys", "按下快捷键")
                } else {
                    tr(cx, "Record shortcut", "录制快捷键")
                },
                cx,
            ))
        }
        let age = self.caps.time(0.70, w, cx);
        if elapsed < 0.10 && !reduced {
            let mut outgoing = row().absolute().left_0().gap(px(4.));
            for k in &self.previous {
                let removed = !tokens.contains(k);
                outgoing = outgoing.child(
                    cap(
                        k.clone(),
                        false,
                        false,
                        false,
                        if removed { 1. - elapsed / 0.1 } else { 1. },
                        p,
                        cx,
                    )
                    .when(!removed, |e| e.opacity(0.)),
                );
            }
            keys = keys.child(outgoing);
        }
        for (i, key) in tokens.into_iter().enumerate() {
            let t = if reduced || self.previous.contains(&key) {
                1.
            } else {
                ease((elapsed - if self.recording { 0. } else { i as f32 * 0.035 }) / 0.26)
            };
            keys = keys.child(cap(
                key,
                false,
                self.recording || (!pending && age < 0.35),
                pending,
                t,
                p,
                cx,
            ));
        }
        let name = self.label.clone();
        let mut rec = div()
            .id("record")
            .debug_selector({
                let name = name.clone();
                move || name.to_string()
            })
            .role(Role::Button)
            .tab_index(0)
            .track_focus(&self.focus)
            .aria_toggled(self.recording.into())
            .aria_label(format!(
                "{name} · {}",
                if self.recording {
                    tr(cx, "Recording", "正在录制").into()
                } else if sel.is_empty() {
                    tr(cx, "Unassigned", "未设置").into()
                } else {
                    self.platform.spoken(sel)
                }
            ))
            .flex_1()
            .min_w_0()
            .h_full()
            .pl(px(9.))
            .pr(px(6.))
            .flex()
            .items_center()
            .rounded(crate::foundation::surface_radius(cx))
            .cursor_pointer()
            .on_modifiers_changed(cx.listener(|s, e: &ModifiersChangedEvent, _, cx| {
                if s.recording {
                    s.held = e.modifiers;
                    cx.notify()
                }
            }))
            .capture_key_down(cx.listener(Self::key))
            .on_click(cx.listener(|s, e: &ClickEvent, w, cx| {
                if s.recording {
                    if !e.is_keyboard() {
                        s.cancel(cx)
                    }
                } else {
                    s.start(w, cx)
                }
            }));
        rec = rec.child(keys);
        let mut control = row()
            .w_full()
            .h(px(44.))
            .gap(px(2.))
            .pr(px(4.))
            .rounded(crate::foundation::surface_radius(cx))
            .border_1()
            .border_color(if pending {
                crate::foundation::Theme::global(cx).tokens.colors.warning
            } else if self.recording || self.focus.is_focused(w) && w.last_input_was_keyboard() {
                p.ink
            } else {
                p.faint.opacity(0.68)
            })
            .when(!pending && !self.recording, |s| {
                s.hover(move |s| s.border_color(p.ink))
            })
            .bg(if self.recording {
                p.panel.blend(p.hover.opacity(0.3))
            } else {
                p.panel
            })
            .child(rec);
        let at_default = self.value == self.initial
            || self
                .value
                .as_deref()
                .zip(self.initial.as_deref())
                .is_some_and(|(a, b)| self.platform.matches(a, b));
        if (!at_default || pending) && !self.recording {
            control = control.child(
                glyph(
                    "reset",
                    tr(cx, "Reset shortcut", "恢复默认快捷键"),
                    Glyph::Undo,
                    cx,
                )
                .size(px(32.))
                .on_click(cx.listener(|s, _, w, cx| s.restore(w, cx))),
            );
        }
        if self.value.is_some() && !self.recording && !pending {
            control = control.child(
                glyph(
                    "clear",
                    tr(cx, "Clear shortcut", "清除快捷键"),
                    Glyph::Close,
                    cx,
                )
                .size(px(32.))
                .on_click(cx.listener(|s, _, w, cx| {
                    s.commit(None, cx);
                    s.focus.focus(w, cx)
                })),
            );
        }
        let copy = if !self.message.is_empty() {
            self.message.clone()
        } else if self.recording {
            tr(
                cx,
                "Esc to cancel, Backspace to clear",
                "Esc 取消，退格键清除",
            )
            .into()
        } else {
            "".into()
        };
        let show = !copy.is_empty();
        if show {
            self.last_message = copy
        }
        let a = self.status.sample(show, 0.24, 0.16, w, cx);
        let mut root = col()
            .w_full()
            .gap(px(8.))
            .id("recorder")
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                if s.candidate.is_some() && e.keystroke.key == "escape" {
                    s.cancel(cx);
                    s.focus.focus(w, cx);
                    w.prevent_default();
                    cx.stop_propagation()
                }
            }))
            .child(
                div()
                    .text_size(px(14.))
                    .line_height(px(20.))
                    .font_weight(FontWeight::MEDIUM)
                    .child(name),
            )
            .child(control);
        if show || a > 0. {
            let mut message = row().flex_wrap().justify_between().gap(px(6.)).child(
                row()
                    .gap(px(7.))
                    .when(pending, |r| {
                        r.child(
                            div()
                                .size(px(6.))
                                .rounded_full()
                                .bg(crate::foundation::Theme::global(cx).tokens.colors.warning),
                        )
                    })
                    .child(
                        hint(self.last_message.clone(), cx)
                            .id("message")
                            .role(Role::Status),
                    ),
            );
            if pending {
                message = message.child(
                    row()
                        .gap(px(5.))
                        .child(
                            button("replace", tr(cx, "Use anyway", "仍然使用"), cx)
                                .h(px(22.))
                                .px(px(2.))
                                .text_size(px(12.))
                                .underline()
                                .on_click(cx.listener(|s, _, w, cx| {
                                    if let Some(v) = s.candidate.take() {
                                        s.commit(Some(v), cx)
                                    }
                                    s.focus.focus(w, cx)
                                })),
                        )
                        .child(
                            button("cancel", tr(cx, "Cancel", "取消"), cx)
                                .h(px(22.))
                                .px(px(2.))
                                .text_size(px(12.))
                                .text_color(p.faint)
                                .on_click(cx.listener(|s, _, w, cx| {
                                    s.cancel(cx);
                                    s.focus.focus(w, cx)
                                })),
                        ),
                );
            }
            root =
                root.child(
                    self.status
                        .wrap(message, a, show, if pending { 22. } else { 17. }, 8.),
                );
        }
        root
    }
}
