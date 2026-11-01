//! Manage masked provider keys, connection tests, and reversible credential removal.
use crate::foundation::{Theme, ThemeMode, surface_radius};
use crate::primitives::{
    control::activate,
    editor::{EditEvent, Editor, InputMode},
    family::{Paint, Tone, disabled_motion, face},
    glyph::{Glyph, countdown_ring, icon, spinner},
    ops_surface::*,
};
use gpui::{prelude::*, *};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};
use web_time::Instant;

pub struct Provider {
    pub id: String,
    pub name: String,
    pub masked: String,
    pub last_used: String,
    pub valid: bool,
    pub prefix: String,
}
pub enum KeyAction {
    Test {
        provider: String,
        secret: String,
        ticket: u64,
    },
    Reveal {
        provider: String,
        ticket: u64,
    },
    Copy(String),
    Remove(String),
    Restore(String),
}
/// Ask the host to validate an already saved key without exposing its value.
pub struct RetestKey {
    pub provider: String,
    pub ticket: u64,
}

struct Controls {
    edit: FocusHandle,
    remove: FocusHandle,
    undo: FocusHandle,
}
struct Removal {
    mask: String,
    valid: bool,
    ticket: u64,
}
struct Form {
    id: String,
    ticket: Option<u64>,
    error: Option<String>,
    tried: bool,
}

pub struct CredentialPanel {
    id: SharedString,
    pub providers: Vec<Provider>,
    editor: Entity<Editor>,
    _edits: Subscription,
    form: Option<Form>,
    last_form: Option<String>,
    header: FocusHandle,
    from_header: bool,
    handles: HashMap<String, Controls>,
    serial: u64,
    revealing: Option<(String, u64)>,
    revealed_key: Option<(String, String, Instant)>,
    retesting: HashMap<String, u64>,
    removed: HashMap<String, Removal>,
    copied: Option<(String, bool, u64)>,
    pub_notice: String,
    unavailable: HashSet<String>,
    demo: bool,
    max_width: f32,
    compact: Option<bool>,
    marks: HashMap<String, [ImageSource; 2]>,
    mono: SharedString,
}

impl EventEmitter<KeyAction> for CredentialPanel {}
impl EventEmitter<RetestKey> for CredentialPanel {}
impl CredentialPanel {
    pub fn new(
        id: impl Into<SharedString>,
        providers: Vec<Provider>,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let editor = editor(
            tr(cx, "Provider key", "提供商密钥").into(),
            InputMode::Password,
            w,
            cx,
        );
        let sub = cx.subscribe_in(
            &editor,
            w,
            |s: &mut Self, _, ev: &EditEvent, w, cx| match ev {
                EditEvent::Changed => {
                    if let Some(form) = &mut s.form {
                        form.ticket = None;
                        form.error = None;
                        form.tried = false;
                    }
                    cx.notify()
                }
                EditEvent::Submitted => s.test(cx),
                EditEvent::Escape => s.close(w, cx),
                _ => {}
            },
        );
        let fonts = cx.text_system().all_font_names();
        let mono = [
            "Fira Code",
            "Menlo",
            "Consolas",
            "Liberation Mono",
            "DejaVu Sans Mono",
        ]
        .into_iter()
        .find(|f| fonts.iter().any(|n| n.eq_ignore_ascii_case(f)))
        .unwrap_or("monospace")
        .into();
        let mut marks = HashMap::new();
        for p in &providers {
            if let Some(svg) = provider_art(&p.id) {
                let images = ["#191919", "#f2f2f2"].map(|ink| {
                    let data = svg.replace("currentColor", ink);
                    Arc::new(Image::from_bytes(ImageFormat::Svg, data.into_bytes())).into()
                });
                marks.insert(p.id.clone(), images);
            }
        }
        Self {
            id: id.into(),
            providers,
            editor,
            _edits: sub,
            form: None,
            last_form: None,
            header: cx.focus_handle(),
            from_header: false,
            handles: HashMap::new(),
            serial: 0,
            revealing: None,
            revealed_key: None,
            retesting: HashMap::new(),
            removed: HashMap::new(),
            copied: None,
            pub_notice: String::new(),
            unavailable: HashSet::new(),
            demo: false,
            max_width: 640.,
            compact: None,
            mono,
            marks,
        }
    }
    /// Hide reveal and copy controls when a host stores only masked metadata.
    pub fn with_mask_only(mut self, provider: impl Into<String>) -> Self {
        self.unavailable.insert(provider.into());
        self
    }
    /// Set the maximum width of this responsive card in logical pixels.
    pub fn with_max_width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.max_width = width.max(260.);
        }
        self
    }
    /// Enable locally generated example inputs and the in-memory preview notice.
    pub fn with_demo_inputs(mut self) -> Self {
        self.demo = true;
        self
    }
    fn ticket(&mut self) -> u64 {
        self.serial = self.serial.wrapping_add(1);
        self.serial
    }
    fn provider(&self, id: &str) -> Option<&Provider> {
        self.providers.iter().find(|p| p.id == id)
    }
    fn edit(&mut self, id: String, from_header: bool, w: &mut Window, cx: &mut Context<Self>) {
        let Some(p) = self.provider(&id) else { return };
        if self.removed.contains_key(&id) {
            return;
        }
        if self.form.as_ref().is_some_and(|f| f.id == id) {
            self.close(w, cx);
            return;
        }
        let placeholder = match p.id.as_str() {
            "anthropic" => "sk-ant-api03-…".to_owned(),
            "mistral" => tr(cx, "Paste a key", "粘贴密钥").to_owned(),
            _ => format!("{}…", p.prefix),
        };
        self.revealing = None;
        self.revealed_key = None;
        self.pub_notice.clear();
        self.from_header = from_header;
        self.last_form = Some(id.clone());
        self.form = Some(Form {
            id,
            ticket: None,
            error: None,
            tried: false,
        });
        let demo = self.demo;
        self.editor.update(cx, |e, cx| {
            e.revealed = demo;
            e.set_placeholder(placeholder);
            e.set_value("", w, cx);
            e.focus(w, cx)
        });
        cx.notify()
    }
    fn close(&mut self, w: &Window, cx: &mut Context<Self>) {
        if let Some(f) = self.form.take() {
            if self.from_header
                && self
                    .providers
                    .iter()
                    .any(|p| p.masked.is_empty() && !self.removed.contains_key(&p.id))
            {
                let focus = self.header.clone();
                cx.defer_in(w, move |_, w, cx| focus.focus(w, cx))
            } else if let Some(h) = self.handles.get(&f.id) {
                let focus = h.edit.clone();
                cx.defer_in(w, move |_, w, cx| focus.focus(w, cx))
            }
        }
        replace(&self.editor, "", w, cx);
        cx.notify()
    }
    fn format_error(p: &Provider, key: &str, cx: &App) -> Option<String> {
        if !p.prefix.is_empty() && !key.starts_with(&p.prefix) {
            return Some(format!(
                "{}{}{}",
                p.name,
                tr(cx, " keys start with ", " 密钥需以以下前缀开头 "),
                p.prefix
            ));
        }
        if key.chars().count() < 8 || key.contains(char::is_whitespace) {
            return Some(
                tr(
                    cx,
                    "Use at least 8 characters, without spaces.",
                    "请输入至少 8 个字符，不能包含空格。",
                )
                .to_owned(),
            );
        }
        None
    }
    fn test(&mut self, cx: &mut Context<Self>) {
        let Some(f) = &self.form else { return };
        if f.ticket.is_some() {
            return;
        }
        let id = f.id.clone();
        let Some(p) = self.provider(&id) else { return };
        let key = self.editor.read(cx).value().trim().to_owned();
        if Self::format_error(p, &key, cx).is_some() {
            self.form.as_mut().unwrap().tried = true;
            cx.notify();
            return;
        }
        let ticket = self.ticket();
        self.form.as_mut().unwrap().ticket = Some(ticket);
        cx.emit(KeyAction::Test {
            provider: id,
            secret: key,
            ticket,
        });
        cx.notify()
    }
    /// True only while this exact candidate is still awaiting a host response.
    pub fn awaiting(&self, ticket: u64) -> bool {
        self.form.as_ref().is_some_and(|f| f.ticket == Some(ticket))
    }
    pub fn tested(
        &mut self,
        ticket: u64,
        masked: Option<String>,
        w: &Window,
        cx: &mut Context<Self>,
    ) {
        if !self.awaiting(ticket) {
            return;
        }
        let f = self.form.as_mut().unwrap();
        f.ticket = None;
        let id = f.id.clone();
        if let Some(masked) = masked {
            let recent = tr(cx, "Tested just now", "刚刚验证");
            if let Some(p) = self.providers.iter_mut().find(|p| p.id == id) {
                p.masked = masked;
                p.valid = true;
                p.last_used = recent.into();
            }
            self.retesting.remove(&id);
            self.pub_notice = tr(cx, "Key connected and saved.", "密钥已验证并保存。").into();
            self.close(w, cx);
        } else {
            f.error = Some(
                tr(
                    cx,
                    "Connection rejected. Check the key and retry.",
                    "验证失败。请检查密钥后重试。",
                )
                .into(),
            );
            let focus = self.editor.read(cx).focus.clone();
            cx.defer_in(w, move |_, w, cx| focus.focus(w, cx));
            cx.notify()
        }
    }
    fn retest(&mut self, id: String, cx: &mut Context<Self>) {
        if self.retesting.contains_key(&id)
            || self.provider(&id).is_none_or(|p| p.masked.is_empty())
        {
            return;
        }
        let ticket = self.ticket();
        self.retesting.insert(id.clone(), ticket);
        cx.emit(RetestKey {
            provider: id,
            ticket,
        });
        cx.notify()
    }
    pub fn retested(&mut self, ticket: u64, ok: bool, cx: &mut Context<Self>) {
        let Some((id, _)) = self.retesting.iter().find(|(_, t)| **t == ticket) else {
            return;
        };
        let id = id.clone();
        self.retesting.remove(&id);
        let text = if ok {
            tr(cx, "Tested just now", "刚刚验证")
        } else {
            tr(cx, "Rejected just now", "刚刚被拒绝")
        };
        if let Some(p) = self
            .providers
            .iter_mut()
            .find(|p| p.id == id && !p.masked.is_empty())
        {
            p.valid = ok;
            p.last_used = text.into();
        }
        self.pub_notice = text.into();
        cx.notify()
    }
    fn toggle_reveal(&mut self, id: String, cx: &mut Context<Self>) {
        let again = self.revealed_key.as_ref().is_some_and(|r| r.0 == id)
            || self.revealing.as_ref().is_some_and(|r| r.0 == id);
        self.revealed_key = None;
        self.revealing = None;
        if !again {
            let ticket = self.ticket();
            self.revealing = Some((id.clone(), ticket));
            cx.emit(KeyAction::Reveal {
                provider: id,
                ticket,
            })
        }
        cx.notify()
    }
    pub fn revealed(
        &mut self,
        ticket: u64,
        provider: String,
        secret: String,
        cx: &mut Context<Self>,
    ) {
        if self.revealing.as_ref() != Some(&(provider.clone(), ticket)) {
            return;
        }
        self.revealing = None;
        let until = cx.background_executor().now() + Duration::from_secs(5);
        self.revealed_key = Some((provider.clone(), secret, until));
        cx.spawn(async move |v, cx| {
            cx.background_executor().timer(Duration::from_secs(5)).await;
            let _ = v.update(cx, |s, cx| {
                if s.revealed_key
                    .as_ref()
                    .is_some_and(|r| r.0 == provider && r.2 == until)
                {
                    s.revealed_key = None;
                    cx.notify()
                }
            });
        })
        .detach();
        cx.notify()
    }
    /// Report the actual clipboard write result, including permission failures.
    pub fn copy_finished(&mut self, id: String, ok: bool, cx: &mut Context<Self>) {
        let ticket = self.ticket();
        self.copied = Some((id, ok, ticket));
        self.pub_notice = if ok {
            tr(cx, "Key copied.", "密钥已复制。")
        } else {
            tr(cx, "Clipboard unavailable.", "剪贴板不可用。")
        }
        .into();
        cx.spawn(async move |v, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(1500))
                .await;
            let _ = v.update(cx, |s, cx| {
                if s.copied.as_ref().is_some_and(|p| p.2 == ticket) {
                    s.copied = None;
                    cx.notify()
                }
            });
        })
        .detach();
        cx.notify()
    }
    fn remove(&mut self, id: String, w: &mut Window, cx: &mut Context<Self>) {
        let ticket = self.ticket();
        let Some(p) = self
            .providers
            .iter_mut()
            .find(|p| p.id == id && !p.masked.is_empty())
        else {
            return;
        };
        self.removed.insert(
            id.clone(),
            Removal {
                mask: std::mem::take(&mut p.masked),
                valid: p.valid,
                ticket,
            },
        );
        if self.form.as_ref().is_some_and(|f| f.id == id) {
            self.close(w, cx)
        }
        self.retesting.remove(&id);
        self.revealing = None;
        self.revealed_key = None;
        self.pub_notice = tr(
            cx,
            "Key removed. Undo for five seconds.",
            "密钥已移除，可在五秒内撤销。",
        )
        .into();
        if let Some(h) = self.handles.get(&id) {
            let target = h.undo.clone();
            cx.defer_in(w, move |_, w, cx| target.focus(w, cx));
        }
        cx.spawn_in(w, async move |v, cx| {
            cx.background_executor().timer(Duration::from_secs(5)).await;
            let _ = cx.update(|w, cx| {
                let _ = v.update(cx, |s, cx| {
                    if s.removed.get(&id).is_some_and(|r| r.ticket == ticket) {
                        s.removed.remove(&id);
                        if let Some(h) = s.handles.get(&id)
                            && h.undo.is_focused(w)
                        {
                            h.edit.focus(w, cx)
                        }
                        cx.emit(KeyAction::Remove(id));
                        cx.notify()
                    }
                });
            });
        })
        .detach();
        cx.notify()
    }
    fn undo(&mut self, id: String, w: &mut Window, cx: &mut Context<Self>) {
        let Some(saved) = self.removed.remove(&id) else {
            return;
        };
        if let Some(p) = self.providers.iter_mut().find(|p| p.id == id) {
            p.masked = saved.mask;
            p.valid = saved.valid;
        }
        if let Some(h) = self.handles.get(&id) {
            h.remove.focus(w, cx)
        }
        self.pub_notice = tr(cx, "Key restored.", "密钥已恢复。").into();
        cx.emit(KeyAction::Restore(id));
        cx.notify()
    }
}
fn console_name(id: &str) -> Option<&'static str> {
    Some(match id {
        "anthropic" => "console.anthropic.com",
        "openai" => "platform.openai.com",
        "google" => "aistudio.google.com",
        "mistral" => "console.mistral.ai",
        "xai" => "console.x.ai",
        _ => return None,
    })
}
fn rotate_mark(ink: Hsla) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |b, _, w, _| {
            let p = |x: f32, y: f32| point(b.origin.x + px(x), b.origin.y + px(y));
            let mut path = PathBuilder::stroke(px(1.35));
            for start in [0.45f32, std::f32::consts::PI + 0.45] {
                for i in 0..=20 {
                    let a = start + 2.52 * i as f32 / 20.;
                    let q = p(8. + 5.7 * a.cos(), 8. + 5.7 * a.sin());
                    if i == 0 {
                        path.move_to(q)
                    } else {
                        path.line_to(q)
                    }
                }
            }
            path.move_to(p(2., 3.));
            path.line_to(p(2., 7.2));
            path.line_to(p(6., 7.2));
            path.move_to(p(14., 13.));
            path.line_to(p(14., 8.8));
            path.line_to(p(10., 8.8));
            if let Ok(path) = path.build() {
                w.paint_path(path, ink)
            }
        },
    )
    .size(px(16.))
    .flex_none()
}
fn unset_ring(ink: Hsla) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |b, _, w, _| {
            let mut path = PathBuilder::stroke(px(1.));
            for i in 0..9 {
                for step in 0..=3 {
                    let a = std::f32::consts::TAU * (i as f32 + 0.55 * step as f32 / 3.) / 9.;
                    let q = point(
                        b.origin.x + px(6.5 + 5.4 * a.cos()),
                        b.origin.y + px(6.5 + 5.4 * a.sin()),
                    );
                    if step == 0 {
                        path.move_to(q)
                    } else {
                        path.line_to(q)
                    }
                }
            }
            if let Ok(path) = path.build() {
                w.paint_path(path, ink)
            }
        },
    )
    .size(px(13.))
    .flex_none()
}
fn api_label(action: &str, p: &Provider) -> String {
    format!("{action} · {}", p.name)
}

impl CredentialPanel {
    fn status(&self, p: &Provider, i: usize, w: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let paint = Paint::get(cx);
        let pending = self.retesting.contains_key(&p.id);
        let (text, g, color) = if pending {
            (
                tr(cx, "Testing…", "验证中…"),
                Glyph::Spinner,
                paint.secondary,
            )
        } else if p.masked.is_empty() {
            (tr(cx, "Not set", "未设置"), Glyph::Minus, paint.faint)
        } else if p.valid {
            (tr(cx, "Connected", "已连接"), Glyph::Check, paint.success)
        } else {
            (tr(cx, "Invalid", "无效"), Glyph::Alert, paint.danger)
        };
        let status_icon = if pending {
            let fps = if disabled_motion(cx) { 1.6 } else { 0.85 };
            let t = cx.background_executor().now();
            let start = w.use_keyed_state(("test-phase", i), cx, |_, _| t);
            let turn = (t.saturating_duration_since(*start.read(cx)).as_secs_f32() / fps).fract();
            w.request_animation_frame();
            spinner(turn, 13., color).into_any_element()
        } else if g == Glyph::Minus {
            unset_ring(paint.faint).into_any_element()
        } else if g == Glyph::Check || g == Glyph::Alert {
            row()
                .size(px(13.))
                .rounded_full()
                .border_1()
                .border_color(color)
                .justify_center()
                .child(icon(g, 9., color))
                .into_any_element()
        } else {
            icon(g, 13., color).into_any_element()
        };
        let label = api_label(tr(cx, "Test connection", "测试连接"), p);
        let id = p.id.clone();
        let content = row().gap(px(5.)).child(status_icon).child(
            swap_label(("provider-status", i), text, w, cx)
                .text_color(paint.secondary)
                .text_size(px(13.))
                .line_height(px(18.)),
        );
        if p.masked.is_empty() {
            content.into_any_element()
        } else {
            activate(
                face(
                    ("status", i),
                    label,
                    24.,
                    surface_radius(cx).into(),
                    Tone::Clear,
                    cx,
                )
                .px(px(4.))
                .child(content),
                cx,
                move |s: &mut Self, _, cx| s.retest(id.clone(), cx),
            )
            .into_any_element()
        }
    }
    fn actions(&self, p: &Provider, i: usize, w: &Window, cx: &Context<Self>) -> Div {
        let paint = Paint::get(cx);
        let id = p.id.clone();
        let handles = &self.handles[&id];
        if self.removed.contains_key(&id) {
            return row().child(
                btn(
                    ("undo", i),
                    tr(cx, "Undo", "撤销"),
                    false,
                    cx,
                    move |s: &mut Self, w, cx| s.undo(id.clone(), w, cx),
                )
                .track_focus(&handles.undo)
                .aria_label(api_label(tr(cx, "Undo removal", "撤销移除"), p))
                .debug_selector(move || format!("Undo removal {i}")),
            );
        }
        if p.masked.is_empty() {
            let label = api_label(tr(cx, "Add key", "添加密钥"), p);
            return row().child(activate(
                face(
                    ("add", i),
                    label,
                    32.,
                    surface_radius(cx).into(),
                    Tone::Clear,
                    cx,
                )
                .px(px(9.))
                .border_1()
                .border_color(paint.line)
                .track_focus(&handles.edit)
                .aria_expanded(self.form.as_ref().is_some_and(|f| f.id == id))
                .child(icon(Glyph::Plus, 14., paint.ink))
                .child(tr(cx, "Add key", "添加密钥")),
                cx,
                move |s: &mut Self, w, cx| s.edit(id.clone(), false, w, cx),
            ));
        }
        let mut r = row().gap(px(2.));
        let visible = self.revealed_key.as_ref().filter(|v| v.0 == id);
        for (action, label, mut g) in [
            (
                0,
                if visible.is_some() {
                    tr(cx, "Hide key", "隐藏密钥")
                } else {
                    tr(cx, "Reveal key", "显示密钥")
                },
                Glyph::Eye,
            ),
            (1, tr(cx, "Copy key", "复制密钥"), Glyph::Copy),
            (2, tr(cx, "Rotate key", "更换密钥"), Glyph::Redo),
            (3, tr(cx, "Remove key", "移除密钥"), Glyph::Trash),
        ] {
            if action < 2 && self.unavailable.contains(&id) {
                continue;
            }
            let copied = self.copied.as_ref().filter(|c| action == 1 && c.0 == id);
            let color = if let Some(c) = copied {
                g = if c.1 { Glyph::Check } else { Glyph::Alert };
                if c.1 { paint.success } else { paint.danger }
            } else {
                paint.secondary
            };
            let id = id.clone();
            let mut button = face(
                (["reveal", "copy", "rotate", "remove"][action], i),
                api_label(label, p),
                32.,
                surface_radius(cx).into(),
                Tone::Clear,
                cx,
            )
            .w(px(32.))
            .relative()
            .hover(move |s| s.bg(paint.hover))
            .child(if action == 2 {
                rotate_mark(color).into_any_element()
            } else if action == 0 && visible.is_some() {
                div()
                    .relative()
                    .size(px(16.))
                    .child(icon(g, 16., color))
                    .child(
                        div()
                            .absolute()
                            .inset_0()
                            .child(icon(Glyph::Slash, 16., color)),
                    )
                    .into_any_element()
            } else {
                icon(g, 16., color).into_any_element()
            });
            if action == 0 && visible.is_none() {
                button = button.aria_toggled(false.into())
            }
            if action == 0
                && let Some((_, _, until)) = visible
            {
                button = button.aria_toggled(true.into());
                if !disabled_motion(cx) {
                    let frac = until
                        .saturating_duration_since(cx.background_executor().now())
                        .as_secs_f32()
                        / 5.;
                    w.request_animation_frame();
                    button = button.child(
                        div()
                            .absolute()
                            .top(px(1.))
                            .right(px(1.))
                            .child(countdown_ring(frac, 10., paint.secondary)),
                    );
                }
            }
            if action == 2 {
                button = button
                    .track_focus(&handles.edit)
                    .aria_expanded(self.form.as_ref().is_some_and(|f| f.id == id))
            }
            if action == 3 {
                button = button.track_focus(&handles.remove)
            }
            r = r.child(activate(
                button,
                cx,
                move |s: &mut Self, w, cx| match action {
                    0 => s.toggle_reveal(id.clone(), cx),
                    1 => cx.emit(KeyAction::Copy(id.clone())),
                    2 => s.edit(id.clone(), false, w, cx),
                    _ => s.remove(id.clone(), w, cx),
                },
            ));
        }
        r
    }
    fn form_ui(&self, p: &Provider, i: usize, small: bool, cx: &Context<Self>) -> Div {
        let paint = Paint::get(cx);
        let adding = p.masked.is_empty();
        let f = self.form.as_ref().filter(|f| f.id == p.id);
        let value = self.editor.read(cx).value().trim();
        let pending = f.is_some_and(|f| f.ticket.is_some());
        let format = if f.is_some() && (!value.is_empty() || f.is_some_and(|f| f.tried)) {
            Self::format_error(p, value, cx)
        } else {
            None
        };
        let error = f.and_then(|f| f.error.clone()).or(format);
        let helper = if pending {
            tr(cx, "Testing connection…", "正在测试连接…").to_owned()
        } else if let Some(e) = &error {
            e.clone()
        } else if adding {
            console_name(&p.id)
                .map(|h| format!("{}{h}", tr(cx, "Create a key at ", "创建密钥：")))
                .unwrap_or_else(|| tr(cx, "Paste a provider key.", "请粘贴提供商密钥。").into())
        } else {
            format!(
                "{}{}",
                tr(
                    cx,
                    "Keeps the current key until verified: ",
                    "验证通过前保留原密钥："
                ),
                p.masked
            )
        };
        let label = if adding {
            format!("{} API {}", p.name, tr(cx, "key", "密钥"))
        } else {
            format!(
                "{} {} {}",
                tr(cx, "New", "新"),
                p.name,
                tr(cx, "key", "密钥")
            )
        };
        let input = input(&self.editor, cx).when(error.is_some(), |b| b.border_color(paint.danger));
        let buttons = row()
            .gap(px(6.))
            .justify_end()
            .child(btn(
                ("cancel", i),
                tr(cx, "Cancel", "取消"),
                false,
                cx,
                |s: &mut Self, w, cx| s.close(w, cx),
            ))
            .child(
                btn(
                    ("test", i),
                    if pending {
                        tr(cx, "Testing…", "验证中…")
                    } else if adding {
                        tr(cx, "Save key", "保存密钥")
                    } else {
                        tr(cx, "Rotate", "更换")
                    },
                    true,
                    cx,
                    |s: &mut Self, _, cx| s.test(cx),
                )
                .debug_selector(move || format!("Save provider {i}")),
            );
        let editors = if small {
            column().gap(px(6.)).child(input).child(buttons)
        } else {
            row()
                .child(div().flex_1().min_w_0().child(input))
                .child(buttons)
        };
        let mut info = column()
            .gap(px(6.))
            .child(
                note(label, cx)
                    .text_color(paint.ink)
                    .font_weight(FontWeight::MEDIUM),
            )
            .child(editors)
            .child(note(helper, cx).when(error.is_some(), |e| e.text_color(paint.danger)));
        if self.demo {
            let prefix = p.prefix.clone();
            info = info.child(
                row().child(
                    btn(
                        ("sample", i),
                        tr(cx, "Use sample key", "使用示例密钥"),
                        false,
                        cx,
                        move |s: &mut Self, w, cx| {
                            replace(&s.editor, format!("{prefix}sample-workspace-7a2b"), w, cx);
                            s.editor.read(cx).focus.clone().focus(w, cx)
                        },
                    )
                    .h(px(24.))
                    .px(px(2.)),
                ),
            );
        }
        info
    }
}
impl Render for CredentialPanel {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let paint = Paint::get(cx);
        let small = self
            .compact
            .unwrap_or(self.max_width.min(w.viewport_size().width.into()) < 560.);
        let dark = usize::from(Theme::global(cx).mode == ThemeMode::Dark);
        let first = self
            .providers
            .iter()
            .find(|p| p.masked.is_empty() && !self.removed.contains_key(&p.id))
            .map(|p| p.id.clone());
        let head = row()
            .items_start()
            .gap(px(12.))
            .child(
                column()
                    .flex_1()
                    .min_w_0()
                    .gap(px(2.))
                    .child(title(tr(cx, "Provider keys", "提供商密钥")).text_size(px(15.)))
                    .child(
                        note(
                            if self.demo {
                                tr(
                                    cx,
                                    "Sample keys for this workspace. Connection tests run locally.",
                                    "此工作区使用示例密钥，连接测试仅在本地运行。",
                                )
                            } else {
                                tr(
                                    cx,
                                    "Manage the provider keys used by this workspace.",
                                    "管理此工作区使用的提供商密钥。",
                                )
                            },
                            cx,
                        )
                        .text_size(px(13.)),
                    ),
            )
            .child(
                face(
                    "header-add",
                    tr(cx, "Add a provider key", "添加提供商密钥"),
                    36.,
                    surface_radius(cx).into(),
                    Tone::Clear,
                    cx,
                )
                .track_focus(&self.header)
                .border_1()
                .border_color(paint.line)
                .px(px(11.))
                .child(icon(Glyph::Plus, 15., paint.ink))
                .child(tr(cx, "Add key", "添加密钥"))
                .when(first.is_none(), |button| {
                    button
                        .tab_stop(false)
                        .cursor_default()
                        .opacity(0.45)
                        .aria_description(tr(
                            cx,
                            "All providers have keys",
                            "所有提供商均已设置密钥",
                        ))
                })
                .when_some(first, |button, id| {
                    activate(button, cx, move |s: &mut Self, w, cx| {
                        s.edit(id.clone(), true, w, cx)
                    })
                }),
            );
        let mut list = column().gap_0().id("provider-list").role(Role::List);
        for (i, p) in self.providers.iter().enumerate() {
            self.handles
                .entry(p.id.clone())
                .or_insert_with(|| Controls {
                    edit: cx.focus_handle(),
                    remove: cx.focus_handle(),
                    undo: cx.focus_handle(),
                });
            let title = title(p.name.clone())
                .text_size(px(14.))
                .line_height(px(20.))
                .whitespace_nowrap();
            let mark = div()
                .size(px(20.))
                .flex_none()
                .when_some(self.marks.get(&p.id), |el, img_src| {
                    el.child(img(img_src[dark].clone()).size(px(20.)))
                })
                .when(!self.marks.contains_key(&p.id), |v| {
                    v.child(initials(&p.name))
                });
            let reveal = self
                .revealed_key
                .as_ref()
                .filter(|r| r.0 == p.id)
                .map(|r| r.1.as_str());
            let key = reveal.map(str::to_owned).unwrap_or_else(|| {
                if p.masked.is_empty() {
                    console_name(&p.id)
                        .map(|h| format!("{}{h}", tr(cx, "Get one at ", "获取密钥：")))
                        .unwrap_or_else(|| tr(cx, "No key yet", "尚无密钥").into())
                } else {
                    p.masked.clone()
                }
            });
            // A raw reveal is replaced immediately when hidden or expired.
            let key = note(key, cx)
                .when(!p.masked.is_empty(), |v| v.font_family(self.mono.clone()))
                .text_size(px(13.))
                .truncate()
                .whitespace_nowrap()
                .h(px(19.));
            let meta = if self.removed.contains_key(&p.id) {
                tr(cx, "Removed just now", "刚刚移除")
            } else if p.masked.is_empty() {
                ""
            } else {
                &p.last_used
            };
            let meta = swap_label(("provider-meta", i), meta.to_owned(), w, cx)
                .text_color(paint.secondary)
                .text_size(px(12.))
                .line_height(px(17.))
                .flex_1()
                .truncate()
                .whitespace_nowrap();
            let status = self.status(p, i, w, cx);
            let actions = self.actions(p, i, w, cx);
            let main = if small {
                column()
                    .gap(px(2.))
                    .child(
                        row()
                            .gap(px(12.))
                            .child(mark)
                            .child(div().flex_1().min_w_0().child(title))
                            .child(status),
                    )
                    .child(div().pl(px(32.)).child(key))
                    .child(
                        row()
                            .pl(px(32.))
                            .gap(px(2.))
                            .flex_wrap()
                            .child(meta.flex_none().max_w_full())
                            .child(actions.flex_none().ml_auto()),
                    )
            } else {
                row()
                    .gap(px(12.))
                    .child(mark)
                    .child(column().flex_1().min_w_0().gap_0().child(title).child(key))
                    .child(column().min_w(px(94.)).gap_0().child(status).child(meta))
                    .child(actions)
            };
            let mut r = column()
                .gap_0()
                .id(("provider-row", i))
                .role(Role::ListItem)
                .px(px(20.))
                .py(px(12.))
                .border_t_1()
                .border_color(paint.line)
                .child(main);
            let open = self.form.as_ref().is_some_and(|f| f.id == p.id);
            let fold = Fold::get(
                ("credential-form", i),
                open,
                if small { 160. } else { 104. },
                0.24,
                w,
                cx,
            );
            if fold.mounted() && self.last_form.as_ref() == Some(&p.id) {
                r = r.child(fold.wrap(0., div().pt(px(10.)).child(self.form_ui(p, i, small, cx))))
            }
            list = list.child(r)
        }
        let status = if self.pub_notice.is_empty() {
            if self.demo {
                tr(
                    cx,
                    "Demo keys stay in memory. Nothing is sent or logged.",
                    "示例密钥仅存于内存，不会发送或记录。",
                )
            } else {
                tr(
                    cx,
                    "Key storage and validation are managed by your host.",
                    "密钥由宿主保管并验证。",
                )
            }
        } else {
            &self.pub_notice
        };
        let card = panel(&self.id, self.max_width, cx)
            .p_0()
            .gap_0()
            .child(div().p(px(20.)).child(head))
            .child(list)
            .child(
                row()
                    .px(px(18.))
                    .py(px(10.))
                    .border_t_1()
                    .border_color(paint.line)
                    .child(icon(Glyph::Lock, 14., paint.faint))
                    .child(
                        note(status.to_owned(), cx)
                            .id("credential-notice")
                            .role(Role::Status),
                    ),
            );
        let owner = cx.weak_entity();
        div()
            .w_full()
            .max_w(px(self.max_width))
            .on_children_prepainted(move |bounds, _, cx| {
                if let Some(b) = bounds.first() {
                    let compact = b.size.width < px(560.);
                    let _ = owner.update(cx, |s, cx| {
                        if s.compact != Some(compact) {
                            s.compact = Some(compact);
                            cx.notify()
                        }
                    });
                }
            })
            .child(card)
    }
}

// Brand SVG artwork from LobeHub Icons (https://github.com/lobehub/lobe-icons).
// MIT License
//
// Copyright (c) 2023 LobeHub
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.
fn provider_art(id: &str) -> Option<&'static str> {
    Some(match id {
        "anthropic" => {
            r##"<svg fill="currentColor" fill-rule="evenodd" height="60" style="flex:none;line-height:1" viewBox="0 0 24 24" width="60" xmlns="http://www.w3.org/2000/svg"><title>Anthropic</title><path d="M13.827 3.52h3.603L24 20h-3.603l-6.57-16.48zm-7.258 0h3.767L16.906 20h-3.674l-1.343-3.461H5.017l-1.344 3.46H0L6.57 3.522zm4.132 9.959L8.453 7.687 6.205 13.48H10.7z"></path></svg>"##
        }
        "openai" => {
            r##"<svg fill="currentColor" fill-rule="evenodd" height="60" style="flex:none;line-height:1" viewBox="0 0 24 24" width="60" xmlns="http://www.w3.org/2000/svg"><title>OpenAI</title><path d="M9.205 8.658v-2.26c0-.19.072-.333.238-.428l4.543-2.616c.619-.357 1.356-.523 2.117-.523 2.854 0 4.662 2.212 4.662 4.566 0 .167 0 .357-.024.547l-4.71-2.759a.797.797 0 00-.856 0l-5.97 3.473zm10.609 8.8V12.06c0-.333-.143-.57-.429-.737l-5.97-3.473 1.95-1.118a.433.433 0 01.476 0l4.543 2.617c1.309.76 2.189 2.378 2.189 3.948 0 1.808-1.07 3.473-2.76 4.163zM7.802 12.703l-1.95-1.142c-.167-.095-.239-.238-.239-.428V5.899c0-2.545 1.95-4.472 4.591-4.472 1 0 1.927.333 2.712.928L8.23 5.067c-.285.166-.428.404-.428.737v6.898zM12 15.128l-2.795-1.57v-3.33L12 8.658l2.795 1.57v3.33L12 15.128zm1.796 7.23c-1 0-1.927-.332-2.712-.927l4.686-2.712c.285-.166.428-.404.428-.737v-6.898l1.974 1.142c.167.095.238.238.238.428v5.233c0 2.545-1.974 4.472-4.614 4.472zm-5.637-5.303l-4.544-2.617c-1.308-.761-2.188-2.378-2.188-3.948A4.482 4.482 0 014.21 6.327v5.423c0 .333.143.571.428.738l5.947 3.449-1.95 1.118a.432.432 0 01-.476 0zm-.262 3.9c-2.688 0-4.662-2.021-4.662-4.519 0-.19.024-.38.047-.57l4.686 2.71c.286.167.571.167.856 0l5.97-3.448v2.26c0 .19-.07.333-.237.428l-4.543 2.616c-.619.357-1.356.523-2.117.523zm5.899 2.83a5.947 5.947 0 005.827-4.756C22.287 18.339 24 15.84 24 13.296c0-1.665-.713-3.282-1.998-4.448.119-.5.19-.999.19-1.498 0-3.401-2.759-5.947-5.946-5.947-.642 0-1.26.095-1.88.31A5.962 5.962 0 0010.205 0a5.947 5.947 0 00-5.827 4.757C1.713 5.447 0 7.945 0 10.49c0 1.666.713 3.283 1.998 4.448-.119.5-.19 1-.19 1.499 0 3.401 2.759 5.946 5.946 5.946.642 0 1.26-.095 1.88-.309a5.96 5.96 0 004.162 1.713z"></path></svg>"##
        }
        "google" => {
            r##"<svg height="60" style="flex:none;line-height:1" viewBox="0 0 24 24" width="60" xmlns="http://www.w3.org/2000/svg"><title>Gemini</title><path d="M20.616 10.835a14.147 14.147 0 01-4.45-3.001 14.111 14.111 0 01-3.678-6.452.503.503 0 00-.975 0 14.134 14.134 0 01-3.679 6.452 14.155 14.155 0 01-4.45 3.001c-.65.28-1.318.505-2.002.678a.502.502 0 000 .975c.684.172 1.35.397 2.002.677a14.147 14.147 0 014.45 3.001 14.112 14.112 0 013.679 6.453.502.502 0 00.975 0c.172-.685.397-1.351.677-2.003a14.145 14.145 0 013.001-4.45 14.113 14.113 0 016.453-3.678.503.503 0 000-.975 13.245 13.245 0 01-2.003-.678z" fill="#3186FF"></path><path d="M20.616 10.835a14.147 14.147 0 01-4.45-3.001 14.111 14.111 0 01-3.678-6.452.503.503 0 00-.975 0 14.134 14.134 0 01-3.679 6.452 14.155 14.155 0 01-4.45 3.001c-.65.28-1.318.505-2.002.678a.502.502 0 000 .975c.684.172 1.35.397 2.002.677a14.147 14.147 0 014.45 3.001 14.112 14.112 0 013.679 6.453.502.502 0 00.975 0c.172-.685.397-1.351.677-2.003a14.145 14.145 0 013.001-4.45 14.113 14.113 0 016.453-3.678.503.503 0 000-.975 13.245 13.245 0 01-2.003-.678z" fill="url(#lobe-icons-gemini-0-_R_0_)"></path><path d="M20.616 10.835a14.147 14.147 0 01-4.45-3.001 14.111 14.111 0 01-3.678-6.452.503.503 0 00-.975 0 14.134 14.134 0 01-3.679 6.452 14.155 14.155 0 01-4.45 3.001c-.65.28-1.318.505-2.002.678a.502.502 0 000 .975c.684.172 1.35.397 2.002.677a14.147 14.147 0 014.45 3.001 14.112 14.112 0 013.679 6.453.502.502 0 00.975 0c.172-.685.397-1.351.677-2.003a14.145 14.145 0 013.001-4.45 14.113 14.113 0 016.453-3.678.503.503 0 000-.975 13.245 13.245 0 01-2.003-.678z" fill="url(#lobe-icons-gemini-1-_R_0_)"></path><path d="M20.616 10.835a14.147 14.147 0 01-4.45-3.001 14.111 14.111 0 01-3.678-6.452.503.503 0 00-.975 0 14.134 14.134 0 01-3.679 6.452 14.155 14.155 0 01-4.45 3.001c-.65.28-1.318.505-2.002.678a.502.502 0 000 .975c.684.172 1.35.397 2.002.677a14.147 14.147 0 014.45 3.001 14.112 14.112 0 013.679 6.453.502.502 0 00.975 0c.172-.685.397-1.351.677-2.003a14.145 14.145 0 013.001-4.45 14.113 14.113 0 016.453-3.678.503.503 0 000-.975 13.245 13.245 0 01-2.003-.678z" fill="url(#lobe-icons-gemini-2-_R_0_)"></path><defs><linearGradient gradientUnits="userSpaceOnUse" id="lobe-icons-gemini-0-_R_0_" x1="7" x2="11" y1="15.5" y2="12"><stop stop-color="#08B962"></stop><stop offset="1" stop-color="#08B962" stop-opacity="0"></stop></linearGradient><linearGradient gradientUnits="userSpaceOnUse" id="lobe-icons-gemini-1-_R_0_" x1="8" x2="11.5" y1="5.5" y2="11"><stop stop-color="#F94543"></stop><stop offset="1" stop-color="#F94543" stop-opacity="0"></stop></linearGradient><linearGradient gradientUnits="userSpaceOnUse" id="lobe-icons-gemini-2-_R_0_" x1="3.5" x2="17.5" y1="13.5" y2="12"><stop stop-color="#FABC12"></stop><stop offset=".46" stop-color="#FABC12" stop-opacity="0"></stop></linearGradient></defs></svg>"##
        }
        "mistral" => {
            r##"<svg height="60" style="flex:none;line-height:1" viewBox="0 0 24 24" width="60" xmlns="http://www.w3.org/2000/svg"><title>Mistral</title><path d="M3.428 3.4h3.429v3.428H3.428V3.4zm13.714 0h3.43v3.428h-3.43V3.4z" fill="gold"></path><path d="M3.428 6.828h6.857v3.429H3.429V6.828zm10.286 0h6.857v3.429h-6.857V6.828z" fill="#FFAF00"></path><path d="M3.428 10.258h17.144v3.428H3.428v-3.428z" fill="#FF8205"></path><path d="M3.428 13.686h3.429v3.428H3.428v-3.428zm6.858 0h3.429v3.428h-3.429v-3.428zm6.856 0h3.43v3.428h-3.43v-3.428z" fill="#FA500F"></path><path d="M0 17.114h10.286v3.429H0v-3.429zm13.714 0H24v3.429H13.714v-3.429z" fill="#E10500"></path></svg>"##
        }
        "xai" => {
            r##"<svg fill="currentColor" fill-rule="evenodd" height="60" style="flex:none;line-height:1" viewBox="0 0 24 24" width="60" xmlns="http://www.w3.org/2000/svg"><title>Grok</title><path d="M6.469 8.776L16.512 23h-4.464L2.005 8.776H6.47zm-.004 7.9l2.233 3.164L6.467 23H2l4.465-6.324zM22 2.582V23h-3.659V7.764L22 2.582zM22 1l-9.952 14.095-2.233-3.163L17.533 1H22z"></path></svg>"##
        }
        _ => return None,
    })
}
