//! Compose updates with keyboard-selectable people and channel mentions.
use crate::primitives::{
    editor::{EditEvent, Editor, InputMode},
    family::{Paint, Slide},
    form_chrome::*,
};
use gpui::{prelude::*, *};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    ops::Range,
    rc::Rc,
};

#[derive(Clone)]
pub struct Mention {
    pub id: SharedString,
    pub label: SharedString,
    pub trigger: char,
}
#[derive(Clone, Debug)]
pub enum MentionEvent {
    Picked(SharedString),
    Submitted(String),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MentionToken {
    pub id: SharedString,
    pub label: SharedString,
    pub trigger: char,
    pub bytes: Range<usize>,
}
#[derive(Clone, Debug)]
pub struct MentionValue {
    pub text: String,
    pub mentions: Vec<MentionToken>,
}
pub struct MentionInput {
    editor: Entity<Editor>,
    items: Vec<Mention>,
    about: HashMap<SharedString, SharedString>,
    tokens: Rc<RefCell<Vec<MentionToken>>>,
    previous: String,
    caret: Option<Range<usize>>,
    span: Option<Range<usize>>,
    matches: Vec<usize>,
    active: usize,
    dismissed: Option<(usize, String)>,
    reveal: FormReveal,
    painted: Vec<usize>,
    highlight: Slide,
    height: Slide,
    width: Rc<Cell<f32>>,
    field_bounds: Rc<Cell<Bounds<Pixels>>>,
    _sub: Subscription,
    _selection: Subscription,
}
impl EventEmitter<MentionEvent> for MentionInput {}
impl EventEmitter<MentionValue> for MentionInput {}
fn units(text: &str, r: Range<usize>) -> Range<usize> {
    text[..r.start].encode_utf16().count()..text[..r.end].encode_utf16().count()
}
fn reconcile(old: &str, new: &str, tokens: &mut Vec<MentionToken>) {
    if old == new {
        return;
    }
    let start = old
        .chars()
        .zip(new.chars())
        .take_while(|(a, b)| a == b)
        .map(|(a, _)| a.len_utf8())
        .sum::<usize>();
    let suffix = old[start..]
        .chars()
        .rev()
        .zip(new[start..].chars().rev())
        .take_while(|(a, b)| a == b)
        .map(|(a, _)| a.len_utf8())
        .sum::<usize>();
    let end = old.len() - suffix;
    let delta = new.len() as isize - old.len() as isize;
    tokens.retain_mut(|t| {
        if t.bytes.end <= start {
            true
        } else if t.bytes.start >= end {
            t.bytes = t.bytes.start.saturating_add_signed(delta)
                ..t.bytes.end.saturating_add_signed(delta);
            true
        } else {
            false
        }
    });
}
fn text_runs(text: &str, base: &TextRun, tokens: &[MentionToken]) -> Vec<TextRun> {
    let mut out = vec![];
    let mut at = 0;
    for t in tokens {
        let r = &t.bytes;
        if r.start < at
            || r.end > text.len()
            || !text.is_char_boundary(r.start)
            || !text.is_char_boundary(r.end)
        {
            continue;
        }
        if at < r.start {
            let mut plain = base.clone();
            plain.len = r.start - at;
            out.push(plain)
        }
        let mut token = base.clone();
        token.len = r.len();
        token.background_color =
            Some(
                base.color
                    .opacity(if t.trigger == '@' { 0.14 } else { 0.08 }),
            );
        token.font.weight = FontWeight::MEDIUM;
        out.push(token);
        at = r.end;
    }
    if at < text.len() || out.is_empty() {
        let mut plain = base.clone();
        plain.len = text.len() - at;
        out.push(plain)
    }
    out
}
impl MentionInput {
    pub fn new(
        label: impl Into<SharedString>,
        items: Vec<Mention>,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let tokens = Rc::new(RefCell::new(Vec::new()));
        let painted = tokens.clone();
        let editor = cx.new(|cx| {
            let mut e = Editor::new(label, InputMode::InlineMulti, w, cx);
            e.font_size = 16.;
            e.set_placeholder(tr(
                cx,
                "Share an update, @ to mention",
                "分享动态，输入 @ 提及成员",
            ));
            e.set_run_styler(Rc::new(move |text, base| {
                text_runs(text, base, &painted.borrow())
            }));
            e
        });
        let sub = cx.subscribe_in(
            &editor,
            w,
            |s: &mut Self, _, ev: &EditEvent, w, cx| match ev {
                EditEvent::Changed => {
                    s.sync(w, cx);
                    cx.notify()
                }
                EditEvent::Submitted => {
                    if s.span.is_some() {
                        if let Some(i) = s.matches.get(s.active).copied() {
                            s.pick(i, w, cx)
                        }
                    } else {
                        s.post(w, cx)
                    }
                }
                EditEvent::Escape => s.dismiss(cx),
                _ => {}
            },
        );
        let selection = cx.observe_in(&editor, w, |s: &mut Self, _, w, cx| s.sync(w, cx));
        Self {
            editor,
            items,
            about: HashMap::new(),
            tokens,
            previous: String::new(),
            caret: None,
            span: None,
            matches: vec![],
            active: 0,
            dismissed: None,
            reveal: FormReveal::new(false),
            painted: vec![],
            highlight: Slide::snappy(0.),
            height: Slide::new(46.),
            width: Rc::new(Cell::new(400.)),
            field_bounds: Default::default(),
            _sub: sub,
            _selection: selection,
        }
    }
    pub fn with_details(
        mut self,
        details: impl IntoIterator<Item = (SharedString, SharedString)>,
    ) -> Self {
        self.about.extend(details);
        self
    }
    pub fn value(&self, cx: &App) -> MentionValue {
        MentionValue {
            text: self.editor.read(cx).value().into(),
            mentions: self.tokens.borrow().clone(),
        }
    }
    fn dismiss(&mut self, cx: &mut Context<Self>) {
        if let Some(r) = self.span.take() {
            self.dismissed = Some((r.start, self.previous[r].to_owned()))
        }
        self.matches.clear();
        cx.notify()
    }
    fn sync(&mut self, _w: &mut Window, cx: &mut Context<Self>) {
        let e = self.editor.read(cx);
        let text = e.value();
        let caret = e.selection();
        if self.previous == text && self.caret.as_ref() == Some(&caret) {
            return;
        }
        let moved = self.caret.as_ref() != Some(&caret);
        let changed = self.previous != text;
        if changed {
            reconcile(&self.previous, text, &mut self.tokens.borrow_mut());
            self.previous = text.to_owned()
        }
        self.caret = Some(caret.clone());
        self.span = None;
        self.matches.clear();
        self.active = 0;
        if caret.is_empty() {
            let pos = caret.end;
            let trigger = text[..pos]
                .char_indices()
                .rev()
                .take_while(|(_, c)| !c.is_whitespace())
                .find(|(_, c)| *c == '@' || *c == '#');
            if let Some((start, kind)) = trigger
                && (start == 0
                    || text[..start]
                        .chars()
                        .next_back()
                        .is_some_and(|c| c.is_whitespace() || "([{（【“‘'\"".contains(c)))
                && !self
                    .tokens
                    .borrow()
                    .iter()
                    .any(|t| t.bytes.start <= start && t.bytes.end >= pos)
            {
                let q = text[start + 1..pos].to_lowercase();
                if self.dismissed.as_ref() != Some(&(start, text[start..pos].to_owned())) {
                    let mut found = self
                        .items
                        .iter()
                        .enumerate()
                        .filter(|(_, p)| p.trigger == kind)
                        .filter_map(|(i, p)| {
                            let name = p.label.to_lowercase();
                            let id = p.id.to_lowercase();
                            let rank = if name.starts_with(&q) {
                                0
                            } else if name.split_whitespace().any(|s| s.starts_with(&q))
                                || id.starts_with(&q)
                            {
                                1
                            } else if name.contains(&q) || id.contains(&q) {
                                2
                            } else if self
                                .about
                                .get(&p.id)
                                .is_some_and(|about| about.to_lowercase().contains(&q))
                            {
                                3
                            } else {
                                return None;
                            };
                            Some((rank, i))
                        })
                        .collect::<Vec<_>>();
                    found.sort_by_key(|&(rank, i)| (rank, i));
                    self.matches = found.into_iter().take(6).map(|(_, i)| i).collect();
                    self.span = Some(start..pos);
                    self.dismissed = None;
                }
            } else if moved || changed {
                self.dismissed = None
            }
        }
        cx.notify()
    }
    fn pick(&mut self, i: usize, w: &mut Window, cx: &mut Context<Self>) {
        let Some(r) = self.span.take() else { return };
        let item = &self.items[i];
        let text = format!("{}{}", item.trigger, item.label);
        // Reuse an existing separator and leave the caret beyond it.
        let replace_end = if self.previous[r.end..].starts_with(' ') {
            r.end + 1
        } else {
            r.end
        };
        let blank = if self.previous[replace_end..].starts_with('\n') {
            ""
        } else {
            " "
        };
        let range = units(&self.previous, r.start..replace_end);
        let tok = MentionToken {
            id: item.id.clone(),
            label: item.label.clone(),
            trigger: item.trigger,
            bytes: r.start..r.start + text.len(),
        };
        self.editor.update(cx, |e, cx| {
            e.replace_text_in_range(Some(range), &format!("{text}{blank}"), w, cx);
            e.focus(w, cx)
        });
        let new = self.editor.read(cx).value().to_owned();
        reconcile(&self.previous, &new, &mut self.tokens.borrow_mut());
        self.previous = new;
        self.tokens.borrow_mut().push(tok.clone());
        self.tokens.borrow_mut().sort_by_key(|t| t.bytes.start);
        self.caret = None;
        self.matches.clear();
        cx.emit(MentionEvent::Picked(tok.id));
        cx.notify()
    }
    fn post(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let value = self.value(cx);
        if !value.text.trim().is_empty() {
            cx.emit(MentionEvent::Submitted(value.text.clone()));
            cx.emit(value);
            self.tokens.borrow_mut().clear();
            self.editor.update(cx, |e, cx| {
                e.set_value("", w, cx);
                e.focus(w, cx)
            });
            self.span = None;
            self.matches.clear();
            cx.notify()
        }
    }
    fn trigger(&mut self, kind: char, w: &mut Window, cx: &mut Context<Self>) {
        self.dismissed = None;
        self.editor.update(cx, |e, cx| {
            let pre = &e.value()[..e.selection().start];
            let pad = pre
                .chars()
                .next_back()
                .is_some_and(|c| !c.is_whitespace() && !"([{（【“‘'\"".contains(c));
            e.replace_text_in_range(
                None,
                &format!("{}{kind}", if pad { " " } else { "" }),
                w,
                cx,
            );
            e.focus(w, cx)
        });
    }
    fn key(&mut self, event: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        if !self.editor.read(cx).focus.is_focused(w)
            || self
                .editor
                .update(cx, |e, cx| e.marked_text_range(w, cx).is_some())
        {
            return;
        }
        let key = event.keystroke.key.as_str();
        if self.span.is_some() && !self.matches.is_empty() && ["up", "down", "tab"].contains(&key) {
            let n = self.matches.len();
            match key {
                "up" => self.active = (self.active + n - 1) % n,
                "down" => self.active = (self.active + 1) % n,
                _ => self.pick(self.matches[self.active], w, cx),
            }
        } else if ["left", "right", "backspace", "delete"].contains(&key)
            && !event.keystroke.modifiers.modified()
        {
            let ed = self.editor.read(cx);
            let sel = ed.selection();
            if !sel.is_empty() {
                return;
            }
            let back = matches!(key, "left" | "backspace");
            let Some(t) = self
                .tokens
                .borrow()
                .iter()
                .find(|t| {
                    if back {
                        t.bytes.end == sel.start
                    } else {
                        t.bytes.start == sel.start
                    }
                })
                .cloned()
            else {
                return;
            };
            let r = units(ed.value(), t.bytes);
            self.editor.update(cx, |e, cx| {
                if matches!(key, "backspace" | "delete") {
                    e.replace_text_in_range(Some(r), "", w, cx)
                } else {
                    let x = if back { r.start } else { r.end };
                    e.set_selected_text_range(x..x, w, cx)
                }
            });
        } else {
            return;
        }
        w.prevent_default();
        cx.stop_propagation();
        cx.notify()
    }
}
fn avatar(i: usize) -> Div {
    let (colors, end) = [
        (0x7c57de, 0xecca56),
        (0x48d9a1, 0xf560a3),
        (0x5de67c, 0xdb69dc),
        (0x66bb60, 0xab79ca),
        (0x58b8d9, 0xe9c86e),
        (0xe575b3, 0x63c7cc),
    ][i % 6];
    div()
        .size(px(28.))
        .flex_none()
        .rounded_full()
        .bg(linear_gradient(
            35.,
            linear_color_stop(rgb(colors), 0.),
            linear_color_stop(rgb(end), 1.),
        ))
}
impl Render for MentionInput {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let text = self.editor.read(cx).value();
        let blank = text.trim().is_empty();
        let row_count = w
            .text_system()
            .shape_text(
                text.to_owned().into(),
                px(16.),
                &[w.text_style().to_run(text.len())],
                Some(px(self.width.get().max(1.))),
                None,
            )
            .map(|l| {
                l.iter()
                    .map(|l| 1 + l.wrap_boundaries().len())
                    .sum::<usize>()
            })
            .unwrap_or(1)
            .clamp(1, 8);
        self.height.duration(23. + 23. * row_count as f32, 0.24, cx);
        let height = self.height.get(w, cx);
        let bounds = self.width.clone();
        let field_bounds = self.field_bounds.clone();
        let field = div()
            .w_full()
            .h(px(height))
            .px(px(14.))
            .py(px(11.))
            .rounded(crate::foundation::surface_radius(cx))
            .border_1()
            .border_color(if self.editor.read(cx).focus.is_focused(w) {
                p.faint
            } else {
                p.line
            })
            .on_children_prepainted(move |bs, w, _| {
                if let Some(b) = bs.first() {
                    field_bounds.set(*b)
                }
                if let Some(b) = bs.first()
                    && bounds.replace(b.size.width.into()) != f32::from(b.size.width)
                {
                    w.request_animation_frame()
                }
            })
            .child(self.editor.clone());
        let mut bar = row().gap(px(2.));
        for (kind, action) in [
            ('@', tr(cx, "Mention someone", "提及成员")),
            ('#', tr(cx, "Add a channel", "添加频道")),
        ] {
            bar = bar.child(
                button(("trigger", kind as usize), "", cx)
                    .aria_label(action)
                    .debug_selector(move || action.to_owned())
                    .p_0()
                    .size(px(34.))
                    .text_size(px(18.))
                    .text_color(p.secondary)
                    .child(kind.to_string())
                    .on_click(cx.listener(move |s, _, w, cx| s.trigger(kind, w, cx))),
            )
        }
        bar = bar
            .child(
                hint(tr(cx, "Enter to post", "回车发布"), cx)
                    .ml(px(8.))
                    .flex_1(),
            )
            .child(
                button("post", tr(cx, "Post", "发布"), cx)
                    .h(px(34.))
                    .px(px(12.))
                    .gap(px(6.))
                    .bg(if blank { p.stage } else { p.bright })
                    .text_color(if blank { p.faint } else { p.inverse })
                    .when(blank, |b| b.tab_stop(false).cursor_default())
                    .child(div().text_size(px(16.)).child("↑"))
                    .on_click(cx.listener(|s, _, w, cx| s.post(w, cx))),
            );
        let mut out = section(522., cx)
            .id("composer")
            .capture_key_down(cx.listener(Self::key))
            .on_mouse_down_out(cx.listener(|s, e: &MouseDownEvent, _, cx| {
                if s.span.is_some() && !s.reveal.contains(e.position) {
                    s.dismiss(cx)
                }
            }))
            .child(
                panel(cx)
                    .w_full()
                    .p(px(12.))
                    .gap(px(8.))
                    .child(field)
                    .child(bar),
            );
        let open = self.span.is_some();
        if open {
            self.painted.clone_from(&self.matches);
            let r = self.span.as_ref().unwrap();
            let offset = self.previous[..r.start].encode_utf16().count();
            let field = self.field_bounds.get();
            let mut marker = self
                .editor
                .update(cx, |e, cx| {
                    e.bounds_for_range(offset..offset, Bounds::default(), w, cx)
                })
                .filter(|b| field.contains(&b.origin))
                .unwrap_or_else(|| Bounds::new(field.origin, size(px(1.), px(23.))));
            // Keep the popup inside the field, even when the trigger wraps to a new line.
            let left = field.left() - px(15.);
            let right = field.right() + px(15.);
            let width = (right - left).min(px(272.));
            marker.origin.x = (marker.left() - px(10.)).clamp(left, right - width);
            self.reveal.at(marker);
        }
        let progress = self.reveal.sample(open, 0.26, 0.126, w, cx);
        self.highlight.duration(self.active as f32, 0.28, cx);
        if open || progress > 0. {
            let n = self.painted.len();
            let mut list = panel(cx)
                .p(px(6.))
                .gap_0()
                .relative()
                .id("mention-results")
                .role(Role::ListBox)
                .aria_label(tr(cx, "Mention suggestions", "提及候选项"));
            if n > 0 {
                list = list.child(
                    div()
                        .absolute()
                        .left(px(6.))
                        .right(px(6.))
                        .top(px(6. + 50. * self.highlight.get(w, cx)))
                        .h(px(50.))
                        .rounded(crate::foundation::surface_radius(cx))
                        .bg(p.hover),
                )
            } else {
                list = list.child(hint(tr(cx, "No matches", "没有匹配结果"), cx).p(px(10.)))
            }
            for (pos, &i) in self.painted.iter().enumerate() {
                let item = &self.items[i];
                let meta = self.about.get(&item.id);
                let leading = if item.trigger == '@' {
                    avatar(i).into_any_element()
                } else {
                    div()
                        .size(px(28.))
                        .text_size(px(20.))
                        .text_color(p.secondary)
                        .child("#")
                        .into_any_element()
                };
                let name = item.label.clone();
                list = list.child(
                    row()
                        .id(("mention", i))
                        .debug_selector({
                            let n = name.clone();
                            move || n.to_string()
                        })
                        .h(px(50.))
                        .px(px(9.))
                        .gap(px(10.))
                        .aria_label(name.clone())
                        .role(Role::ListBoxOption)
                        .aria_selected(self.active == pos)
                        .cursor_pointer()
                        .child(leading)
                        .child(
                            col()
                                .flex_1()
                                .gap_0()
                                .child(div().truncate().font_weight(FontWeight::MEDIUM).child(name))
                                .when_some(meta, |v, m| v.child(hint(m.clone(), cx).truncate())),
                        )
                        .on_mouse_move(cx.listener(move |s, _, _, cx| {
                            if s.active != pos {
                                s.active = pos;
                                cx.notify()
                            }
                        }))
                        .on_click(cx.listener(move |s, _, w, cx| s.pick(i, w, cx))),
                )
            }
            out = out.child(self.reveal.popup(
                list,
                progress,
                open,
                (
                    (self.width.get() + 30.).min(272.),
                    14. + 50. * n.max(1) as f32,
                    4.,
                ),
                w,
            ))
        }
        out
    }
}
