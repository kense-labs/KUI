//! Editable Unicode text, IME composition and selection shared by KUI fields.
use crate::primitives::family::Paint;
use gpui::{prelude::*, *};
use std::{ops::Range, time::Duration};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Single,
    Multi,
    Password,
    Search,
    Combobox,
    InlineMulti,
}
impl InputMode {
    fn multiline(self) -> bool {
        matches!(self, Self::Multi | Self::InlineMulti)
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditEvent {
    Changed,
    Submitted,
    Escape,
    Navigate(bool),
    Focused,
    Blurred,
}
struct Layout {
    bounds: Bounds<Pixels>,
    origin: Point<Pixels>,
    rows: Vec<WrappedLine>,
    line: Pixels,
    // Only populated when the painted text differs from the editable text.
    mask: Vec<usize>,
}
impl Layout {
    fn visible_index(&self, index: usize) -> usize {
        if self.mask.is_empty() {
            index
        } else {
            self.mask.partition_point(|p| *p < index) * '•'.len_utf8()
        }
    }
    fn index(&self, index: usize) -> usize {
        if self.mask.is_empty() {
            index
        } else {
            *self.mask.get(index / '•'.len_utf8()).unwrap_or_else(|| {
                self.mask
                    .last()
                    .expect("masked layout has a terminal offset")
            })
        }
    }
    fn at(&self, byte: usize) -> Point<Pixels> {
        let mut remain = self.visible_index(byte);
        let mut p = self.origin;
        for row in &self.rows {
            if remain <= row.len() {
                return p + row
                    .position_for_index(remain, self.line)
                    .unwrap_or_default();
            }
            remain -= row.len() + 1;
            p.y += row.size(self.line).height;
        }
        p
    }
    fn hit(&self, p: Point<Pixels>) -> usize {
        let mut local = p - self.origin;
        local.y = local.y.max(px(0.));
        let mut offset = 0;
        for row in &self.rows {
            let height = row.size(self.line).height;
            if local.y < height {
                return self.index(
                    offset
                        + row
                            .closest_index_for_position(local, self.line)
                            .unwrap_or_else(|v| v),
                );
            }
            local.y -= height;
            offset += row.len() + 1;
        }
        self.index(offset.saturating_sub(1))
    }
    fn rects(&self, range: Range<usize>) -> Vec<Bounds<Pixels>> {
        let a = self.at(range.start);
        let b = self.at(range.end);
        let mut y = a.y;
        let mut rects = Vec::new();
        while y <= b.y {
            let x = if y == a.y { a.x } else { self.bounds.left() };
            let end = if y == b.y { b.x } else { self.bounds.right() };
            rects.push(Bounds::new(
                point(x, y),
                size((end - x).max(px(1.)), self.line),
            ));
            y += self.line
        }
        rects
    }
}
#[derive(Clone)]
struct Snapshot {
    text: String,
    anchor: usize,
    caret: usize,
}
/// An embedding host may hand the wheel back to the document at the scroll limits.
pub struct EditorScrollRegion(pub fn(Bounds<Pixels>, bool, bool));
impl Global for EditorScrollRegion {}

// On the web the browser may send an ordinary, unflagged printable keydown
// before compositionstart. Let its textarea deliver text via input/composition
// instead of letting the GPUI keydown fallback insert the opening letter too.
#[cfg(target_family = "wasm")]
fn browser_text_input() -> Subscription {
    use wasm_bindgen::prelude::*;
    #[wasm_bindgen(inline_js = r#"
        export function listenForEditorText() {
            const controller = new AbortController();
            const options = { capture: true, signal: controller.signal };
            function attach(input) {
                let composing = false;
                input.addEventListener('compositionstart', () => { composing = true; }, options);
                input.addEventListener('compositionend', () => { composing = false; }, options);
                input.addEventListener('blur', () => { composing = false; }, options);
                input.addEventListener('keydown', event => {
                    if (input.readOnly) return;
                    const shortcut = event.metaKey || (event.ctrlKey && !event.getModifierState('AltGraph'));
                    const textKey = [...event.key].length === 1 || event.key === 'Dead';
                    if (composing || event.isComposing || event.keyCode === 229 || (!shortcut && textKey)) {
                        // Preserve the default action that drives the IME and regular typing.
                        event.stopImmediatePropagation();
                    }
                }, options);
            }
            if (document.activeElement instanceof HTMLTextAreaElement) {
                attach(document.activeElement);
            } else {
                // GPUI may assign editor focus just before it focuses the DOM mirror.
                const onFocus = event => {
                    if (!(event.target instanceof HTMLTextAreaElement)) return;
                    document.removeEventListener('focusin', onFocus, true);
                    attach(event.target);
                };
                document.addEventListener('focusin', onFocus, options);
            }
            return controller;
        }
        export function stopListeningForEditorText(controller) { controller.abort(); }
    "#)]
    extern "C" {
        #[wasm_bindgen(js_name = listenForEditorText)]
        fn listen() -> JsValue;
        #[wasm_bindgen(js_name = stopListeningForEditorText)]
        fn stop(controller: JsValue);
    }
    let controller = listen();
    Subscription::new(move || stop(controller))
}

/// Optional glyph runs for an attributed document. Text, IME and selection retain the normal input path.
pub type RunStyler = std::rc::Rc<dyn Fn(&str, &TextRun) -> Vec<TextRun>>;
pub struct Editor {
    run_styler: Option<RunStyler>,
    label: SharedString,
    placeholder: SharedString,
    pub(crate) expanded: Option<bool>,
    // A closed picker paints the committed option while keeping its query buffer empty.
    pub(crate) committed_label: Option<SharedString>,
    pub mode: InputMode,
    value: String,
    anchor: usize,
    caret: usize,
    marked: Option<Range<usize>>,
    pub focus: FocusHandle,
    pub revealed: bool,
    pub disabled: bool,
    pub font_size: f32,
    pub description: SharedString,
    layout: Option<Layout>,
    scroll: Point<Pixels>,
    follow_caret: bool,
    dragging: bool,
    history: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    cursor: bool,
    blink: Option<Task<()>>,
    #[cfg(target_family = "wasm")]
    browser_keys: Option<Subscription>,
    _listeners: Vec<Subscription>,
}
impl EventEmitter<EditEvent> for Editor {}
impl Focusable for Editor {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}
fn from_utf16(s: &str, offset: usize) -> usize {
    let mut units = 0;
    for (i, ch) in s.char_indices() {
        if units + ch.len_utf16() > offset {
            return i;
        }
        units += ch.len_utf16()
    }
    s.len()
}
impl Editor {
    pub fn new(
        label: impl Into<SharedString>,
        mode: InputMode,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus = cx.focus_handle().tab_stop(true);
        let entered = cx.on_focus(&focus, w, |s: &mut Self, _, cx| {
            #[cfg(target_family = "wasm")]
            {
                s.browser_keys = Some(browser_text_input());
            }
            s.cursor = true;
            s.blink = Some(cx.spawn(async move |this, cx| {
                loop {
                    cx.background_executor()
                        .timer(Duration::from_millis(530))
                        .await;
                    if this
                        .update(cx, |s, cx| {
                            s.cursor = !s.cursor;
                            cx.notify()
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            }));
            cx.emit(EditEvent::Focused);
            cx.notify()
        });
        let left = cx.on_blur(&focus, w, |s: &mut Self, _, cx| {
            #[cfg(target_family = "wasm")]
            {
                s.browser_keys = None;
            }
            s.blink = None;
            s.dragging = false;
            s.marked = None;
            cx.emit(EditEvent::Blurred);
            cx.notify()
        });
        Self {
            label: label.into(),
            run_styler: None,
            placeholder: "".into(),
            expanded: None,
            committed_label: None,
            mode,
            value: String::new(),
            anchor: 0,
            caret: 0,
            marked: None,
            focus,
            revealed: false,
            disabled: false,
            font_size: 14.,
            description: "".into(),
            layout: None,
            scroll: Default::default(),
            follow_caret: true,
            dragging: false,
            history: vec![],
            redo: vec![],
            cursor: true,
            blink: None,
            #[cfg(target_family = "wasm")]
            browser_keys: None,
            _listeners: vec![entered, left],
        }
    }
    pub fn set_run_styler(&mut self, styler: RunStyler) {
        self.run_styler = Some(styler)
    }
    /// The normalized byte range, including a collapsed caret.
    pub fn selection(&self) -> Range<usize> {
        self.selected()
    }
    pub fn value(&self) -> &str {
        &self.value
    }
    pub fn set_placeholder(&mut self, s: impl Into<SharedString>) {
        self.placeholder = s.into()
    }
    pub fn set_value(&mut self, s: impl Into<String>, w: &Window, cx: &mut Context<Self>) {
        let value = self.normalize(&s.into());
        if self.value == value {
            return;
        }
        self.value = value;
        self.caret = self.value.len();
        self.anchor = self.caret;
        self.marked = None;
        self.history.clear();
        self.redo.clear();
        self.changed(w, cx)
    }
    pub fn focus(&self, w: &mut Window, cx: &mut App) {
        if !self.disabled {
            self.focus.focus(w, cx)
        }
    }
    pub fn select_all(&mut self, w: &Window, cx: &mut Context<Self>) {
        self.anchor = 0;
        self.select(self.value.len(), true, w, cx)
    }
    fn normalize(&self, s: &str) -> String {
        let s = s.replace("\r\n", "\n").replace('\r', "\n");
        if self.mode.multiline() {
            s
        } else {
            s.replace('\n', " ")
        }
    }
    fn selected(&self) -> Range<usize> {
        self.anchor.min(self.caret)..self.anchor.max(self.caret)
    }
    fn bytes(&self, r: Range<usize>) -> Range<usize> {
        let a = from_utf16(&self.value, r.start);
        a..from_utf16(&self.value, r.end).max(a)
    }
    fn units(&self, r: Range<usize>) -> Range<usize> {
        self.value[..r.start].encode_utf16().count()..self.value[..r.end].encode_utf16().count()
    }
    fn select(&mut self, offset: usize, extend: bool, w: &Window, cx: &mut Context<Self>) {
        self.caret = offset.min(self.value.len());
        if !extend {
            self.anchor = self.caret
        }
        self.follow_caret = true;
        self.cursor = true;
        w.notify_text_selection_changed(&self.focus);
        cx.notify()
    }
    fn changed(&mut self, w: &Window, cx: &mut Context<Self>) {
        self.cursor = true;
        self.follow_caret = true;
        self.layout = None;
        w.notify_text_input_changed(&self.focus);
        cx.emit(EditEvent::Changed);
        cx.notify()
    }
    fn snapshot(&self) -> Snapshot {
        Snapshot {
            text: self.value.clone(),
            anchor: self.anchor,
            caret: self.caret,
        }
    }
    fn edit(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        composition: bool,
        sel: Option<Range<usize>>,
        w: &Window,
        cx: &mut Context<Self>,
    ) {
        if self.disabled {
            return;
        }
        let range = range
            .map(|r| self.bytes(r))
            .or_else(|| self.marked.clone())
            .unwrap_or_else(|| self.selected());
        let text = self.normalize(text);
        if self.marked.is_none() {
            self.history.push(self.snapshot());
            if self.history.len() > 100 {
                self.history.remove(0);
            }
            self.redo.clear()
        }
        self.value.replace_range(range.clone(), &text);
        let end = range.start + text.len();
        self.caret = end;
        self.anchor = end;
        self.marked = (composition && !text.is_empty()).then_some(range.start..end);
        if composition && let Some(sel) = sel {
            self.anchor = range.start + from_utf16(&text, sel.start);
            self.caret = range.start + from_utf16(&text, sel.end)
        }
        self.changed(w, cx)
    }
    fn undo(&mut self, forward: bool, w: &Window, cx: &mut Context<Self>) {
        let old = if forward {
            self.redo.pop()
        } else {
            self.history.pop()
        };
        if let Some(old) = old {
            let current = self.snapshot();
            if forward {
                self.history.push(current)
            } else {
                self.redo.push(current)
            }
            self.value = old.text;
            self.anchor = old.anchor;
            self.caret = old.caret;
            self.marked = None;
            self.changed(w, cx)
        }
    }
    fn boundary(&self, right: bool, word: bool) -> usize {
        if right {
            let tail = &self.value[self.caret..];
            self.caret
                + if word {
                    tail.split_word_bound_indices()
                        .next()
                        .map_or(0, |(_, s)| s.len())
                } else {
                    tail.graphemes(true).next().map_or(0, str::len)
                }
        } else {
            let head = &self.value[..self.caret];
            if word {
                head.split_word_bound_indices()
                    .next_back()
                    .map_or(0, |(i, _)| i)
            } else {
                head.grapheme_indices(true)
                    .next_back()
                    .map_or(0, |(i, _)| i)
            }
        }
    }
    fn key(&mut self, e: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        if self.disabled || self.marked.is_some() {
            return;
        }
        let m = e.keystroke.modifiers;
        let k = e.keystroke.key.as_str();
        let cmd = m.platform || m.control;
        let r = self.selected();
        match k {
            "a" if cmd => self.select_all(w, cx),
            "z" if cmd => self.undo(m.shift, w, cx),
            "y" if cmd => self.undo(true, w, cx),
            "c" | "x" if cmd => {
                if !r.is_empty() && self.mode != InputMode::Password {
                    cx.write_to_clipboard(ClipboardItem::new_string(self.value[r].to_owned()));
                    if k == "x" {
                        self.edit(None, "", false, None, w, cx)
                    }
                }
            }
            "v" if cmd && !cfg!(target_family = "wasm") => {
                if let Some(text) = cx.read_from_clipboard().and_then(|v| v.text()) {
                    self.edit(None, &text, false, None, w, cx)
                }
            }
            "left" | "right" => {
                let right = k == "right";
                let to = if !m.shift && !r.is_empty() {
                    if right { r.end } else { r.start }
                } else if m.platform {
                    self.line_edge(right)
                } else {
                    self.boundary(right, m.alt || m.control)
                };
                self.select(to, m.shift, w, cx)
            }
            "home" | "end" => {
                let end = k == "end";
                let to = if cmd {
                    if end { self.value.len() } else { 0 }
                } else {
                    self.line_edge(end)
                };
                self.select(to, m.shift, w, cx)
            }
            "backspace" | "delete" => {
                if r.is_empty() {
                    self.anchor = self.boundary(k == "delete", m.alt || m.control)
                }
                if !self.selected().is_empty() {
                    self.edit(None, "", false, None, w, cx)
                }
            }
            "up" | "down" if self.mode == InputMode::Combobox => {
                cx.emit(EditEvent::Navigate(k == "down"))
            }
            "up" | "down" if self.mode.multiline() => {
                if let Some(l) = &self.layout {
                    let mut p = l.at(self.caret);
                    p.y += if k == "down" {
                        l.line * 1.5
                    } else {
                        -l.line * 0.5
                    };
                    self.select(l.hit(p), m.shift, w, cx)
                }
            }
            "enter"
                if self.mode == InputMode::Multi
                    || (self.mode == InputMode::InlineMulti && m.shift) =>
            {
                self.edit(None, "\n", false, None, w, cx)
            }
            "enter" => cx.emit(EditEvent::Submitted),
            "escape" => cx.emit(EditEvent::Escape),
            _ => return,
        }
        w.prevent_default();
        cx.stop_propagation()
    }
    fn line_edge(&self, end: bool) -> usize {
        if end {
            self.value[self.caret..]
                .find('\n')
                .map_or(self.value.len(), |n| self.caret + n)
        } else {
            self.value[..self.caret].rfind('\n').map_or(0, |i| i + 1)
        }
    }
}
impl EntityInputHandler for Editor {
    fn text_for_range(
        &mut self,
        r: Range<usize>,
        actual: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        let r = self.bytes(r);
        *actual = Some(self.units(r.clone()));
        Some(self.value[r].to_owned())
    }
    fn selected_text_range(
        &mut self,
        _: bool,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.units(self.selected()),
            reversed: self.anchor > self.caret,
        })
    }
    fn text_length_utf16(&mut self, _: &mut Window, _: &mut Context<Self>) -> Option<usize> {
        Some(self.value.encode_utf16().count())
    }
    fn set_selected_text_range(&mut self, r: Range<usize>, w: &mut Window, cx: &mut Context<Self>) {
        let r = self.bytes(r);
        self.anchor = r.start;
        self.select(r.end, true, w, cx)
    }
    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        self.marked.clone().map(|r| self.units(r))
    }
    fn unmark_text(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        self.marked = None;
        w.notify_text_input_changed(&self.focus);
        cx.notify()
    }
    fn replace_text_in_range(
        &mut self,
        r: Option<Range<usize>>,
        s: &str,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.edit(r, s, false, None, w, cx)
    }
    fn replace_and_mark_text_in_range(
        &mut self,
        r: Option<Range<usize>>,
        s: &str,
        sel: Option<Range<usize>>,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.edit(r, s, true, sel, w, cx)
    }
    fn bounds_for_range(
        &mut self,
        r: Range<usize>,
        _: Bounds<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        self.layout
            .as_ref()?
            .rects(self.bytes(r))
            .into_iter()
            .next()
    }
    fn selection_bounds_for_range(
        &mut self,
        r: Range<usize>,
        _: Bounds<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Vec<Bounds<Pixels>> {
        self.layout
            .as_ref()
            .map_or_else(Vec::new, |l| l.rects(self.bytes(r)))
    }
    fn character_index_for_point(
        &mut self,
        p: Point<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<usize> {
        let n = self.layout.as_ref()?.hit(p).min(self.value.len());
        Some(self.units(n..n).start)
    }
    fn accepts_text_input(&self, _: &mut Window, _: &mut Context<Self>) -> bool {
        !self.disabled
    }
    fn text_input_configuration(
        &mut self,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> TextInputConfiguration {
        TextInputConfiguration {
            autocorrect: false,
            suggestions: self.mode != InputMode::Password,
            input_action: match self.mode {
                InputMode::Multi => TextInputAction::Enter,
                InputMode::Combobox | InputMode::Search => TextInputAction::Search,
                _ => TextInputAction::Done,
            },
            ..Default::default()
        }
    }
}
impl Editor {
    fn paint(&mut self, b: Bounds<Pixels>, w: &mut Window, cx: &mut Context<Self>) {
        let focused = self.focus.is_focused(w);
        let p = Paint::get(cx);
        let blank = self.value.is_empty();
        let masked = self.mode == InputMode::Password && !self.revealed && !blank;
        let mask = if masked {
            self.value
                .grapheme_indices(true)
                .map(|(i, _)| i)
                .chain(std::iter::once(self.value.len()))
                .collect::<Vec<_>>()
        } else {
            vec![]
        };
        let text: SharedString = if blank {
            self.committed_label
                .clone()
                .unwrap_or_else(|| self.placeholder.clone())
        } else if masked {
            "•".repeat(mask.len() - 1).into()
        } else {
            self.value.clone().into()
        };
        let mut run = w.text_style().to_run(text.len());
        run.color = if (blank && self.committed_label.is_none()) || self.disabled {
            p.secondary
        } else {
            p.ink
        };
        let line = px(self.font_size + 7.);
        let multi = self.mode.multiline();
        let styled = if !blank && !masked {
            self.run_styler.as_ref().map(|f| f(&text, &run))
        } else {
            None
        };
        let rows = w
            .text_system()
            .shape_text(
                text,
                px(self.font_size),
                styled.as_deref().unwrap_or(std::slice::from_ref(&run)),
                multi.then_some(b.size.width.max(px(1.))),
                None,
            )
            .unwrap_or_default()
            .into_vec();
        let height = rows
            .iter()
            .fold(px(0.), |sum, row| sum + row.size(line).height);
        let top = if multi {
            b.top()
        } else {
            b.top() + (b.size.height - line) * 0.5
        };
        let origin = point(b.left(), top);
        let mut layout = Layout {
            bounds: b,
            origin,
            rows,
            mask,
            line,
        };
        let pt = layout.at(self.caret) - origin;
        if self.follow_caret {
            let keep = |scroll: Pixels, pos: Pixels, room: Pixels| {
                scroll.min(pos).max(pos - room).max(px(0.))
            };
            self.scroll.x = if multi {
                px(0.)
            } else {
                keep(self.scroll.x, pt.x, b.size.width - px(2.))
            };
            self.scroll.y = keep(self.scroll.y, pt.y, (b.size.height - line).max(px(0.)));
            self.follow_caret = false
        }
        // A newly mounted flex child can first paint at width zero; never retain
        // that transient horizontal offset once the real width is known.
        let longest = layout
            .rows
            .iter()
            .map(|row| row.size(line).width)
            .max()
            .unwrap_or(px(0.));
        self.scroll.x = self
            .scroll
            .x
            .min((longest + px(2.) - b.size.width).max(px(0.)));
        self.scroll.y = self.scroll.y.min((height - b.size.height).max(px(0.)));
        if self.mode == InputMode::Multi
            && let Some(host) = cx.try_global::<EditorScrollRegion>()
        {
            (host.0)(
                b,
                self.scroll.y > px(0.),
                self.scroll.y + b.size.height < height,
            );
        }
        layout.origin -= self.scroll;
        w.with_content_mask(Some(ContentMask { bounds: b }), |w| {
            if focused && !self.selected().is_empty() {
                for rect in layout.rects(self.selected()) {
                    w.paint_quad(fill(rect, p.ink.opacity(0.19)))
                }
            }
            let mut pos = layout.origin;
            for row in &layout.rows {
                let _ = row.paint(pos, line, TextAlign::Left, None, w, cx);
                pos.y += row.size(line).height;
            }
            if let Some(mark) = &self.marked {
                for rect in layout.rects(mark.clone()) {
                    w.paint_quad(fill(
                        Bounds::new(
                            point(rect.left(), rect.bottom() - px(1.)),
                            size(rect.size.width, px(1.)),
                        ),
                        p.ink,
                    ))
                }
            }
            if focused
                && self.cursor
                && self.selected().is_empty()
                && self.committed_label.is_none()
            {
                let caret = layout.at(self.caret);
                w.paint_quad(fill(
                    Bounds::new(caret + point(px(0.), px(2.)), size(px(1.), line - px(4.))),
                    p.ink,
                ))
            }
        });
        self.layout = Some(layout);
        if !self.disabled {
            w.handle_input(&self.focus, ElementInputHandler::new(b, cx.entity()), cx)
        }
    }
}
impl Render for Editor {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity();
        let name = self.label.clone();
        let role = match self.mode {
            InputMode::Password => Role::PasswordInput,
            InputMode::Multi | InputMode::InlineMulti => Role::MultilineTextInput,
            InputMode::Search => Role::SearchInput,
            InputMode::Combobox => Role::ComboBox,
            _ => Role::TextInput,
        };
        div()
            .id("editor")
            .debug_selector(move || name.to_string())
            .track_focus(&self.focus)
            .tab_stop(!self.disabled)
            .role(role)
            .aria_label(self.label.clone())
            .aria_description(self.description.clone())
            .aria_placeholder(self.placeholder.clone())
            .when_some(self.expanded, |e, open| e.aria_expanded(open))
            .when(self.mode != InputMode::Password, |e| {
                e.aria_value(
                    self.committed_label
                        .clone()
                        .unwrap_or_else(|| self.value.clone().into()),
                )
            })
            .size_full()
            .min_w_0()
            .overflow_hidden()
            .cursor(CursorStyle::IBeam)
            .on_key_down(cx.listener(Self::key))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|s, e: &MouseDownEvent, w, cx| {
                    if s.disabled {
                        return;
                    }
                    s.focus(w, cx);
                    w.request_virtual_keyboard();
                    let pos = s
                        .layout
                        .as_ref()
                        .map_or(0, |l| l.hit(e.position))
                        .min(s.value.len());
                    if e.click_count >= 3 {
                        s.select_all(w, cx)
                    } else if e.click_count == 2 {
                        let r = s
                            .value
                            .split_word_bound_indices()
                            .find(|(i, t)| pos < *i + t.len())
                            .map(|(i, t)| i..i + t.len())
                            .unwrap_or(pos..pos);
                        s.anchor = r.start;
                        s.select(r.end, true, w, cx)
                    } else {
                        s.select(pos, e.modifiers.shift, w, cx)
                    }
                    s.dragging = true;
                }),
            )
            .on_mouse_move(cx.listener(|s, e: &MouseMoveEvent, w, cx| {
                if s.dragging
                    && e.pressed_button == Some(MouseButton::Left)
                    && let Some(l) = &s.layout
                {
                    s.select(l.hit(e.position), true, w, cx)
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|s, _, _, _| s.dragging = false),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|s, _, _, _| s.dragging = false),
            )
            .on_scroll(cx.listener(|s, e: &ScrollEvent, _, cx| {
                if s.mode.multiline()
                    && let Some(l) = &s.layout
                {
                    let max = l
                        .rows
                        .iter()
                        .map(|r| f32::from(r.size(l.line).height))
                        .sum::<f32>()
                        - f32::from(l.bounds.size.height);
                    let next =
                        (s.scroll.y - e.delta.pixel_delta(l.line).y).clamp(px(0.), px(max.max(0.)));
                    if next != s.scroll.y {
                        s.scroll.y = next;
                        s.follow_caret = false;
                        cx.stop_propagation();
                        cx.notify()
                    }
                }
            }))
            .child(
                canvas(
                    |_, _, _| (),
                    move |bounds, _, w, cx| entity.update(cx, |s, cx| s.paint(bounds, w, cx)),
                )
                .size_full(),
            )
    }
}
