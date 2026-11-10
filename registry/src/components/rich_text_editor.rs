//! Edit styled blocks directly; the editable surface, Markdown and escaped HTML share one document.
use crate::primitives::{
    clipboard,
    editor::{EditEvent, Editor, InputMode},
    family::{Paint, Slide},
    form_chrome::*,
    glyph::Glyph,
};
use gpui::{prelude::*, *};
use std::{
    cell::{Cell, RefCell},
    ops::Range,
    rc::Rc,
};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RichDocument {
    pub markdown: String,
    pub html: String,
    pub text: String,
    pub empty: bool,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mark {
    Bold,
    Italic,
    Code,
    Strike,
}
impl Mark {
    fn flag(self) -> u8 {
        match self {
            Self::Bold => 1,
            Self::Italic => 2,
            Self::Code => 4,
            Self::Strike => 8,
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Inline {
    flags: u8,
    link: String,
}
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
enum Kind {
    #[default]
    Paragraph,
    H1,
    H2,
    H3,
    List,
    Order,
    Quote,
    Code,
}
const BLOCKS: &[(Kind, &str, &str)] = &[
    (Kind::Paragraph, "Paragraph", "正文"),
    (Kind::H1, "Heading 1", "一级标题"),
    (Kind::H2, "Heading 2", "二级标题"),
    (Kind::H3, "Heading 3", "三级标题"),
    (Kind::List, "Bullet list", "无序列表"),
    (Kind::Order, "Numbered list", "有序列表"),
    (Kind::Quote, "Quote", "引用"),
    (Kind::Code, "Code block", "代码块"),
];
impl Kind {
    fn prefix(self) -> &'static str {
        match self {
            Self::H1 => "# ",
            Self::H2 => "## ",
            Self::H3 => "### ",
            Self::List => "- ",
            Self::Order => "1. ",
            Self::Quote => "> ",
            _ => "",
        }
    }
    fn tag(self) -> &'static str {
        match self {
            Self::H1 => "h1",
            Self::H2 => "h2",
            Self::H3 => "h3",
            Self::List | Self::Order => "li",
            Self::Quote => "blockquote",
            Self::Code => "pre",
            Self::Paragraph => "p",
        }
    }
    fn size(self) -> f32 {
        match self {
            Self::H1 => 22.,
            Self::H2 => 18.,
            Self::H3 => 16.,
            _ => 14.,
        }
    }
    fn list(self) -> bool {
        matches!(self, Self::List | Self::Order)
    }
    fn heading(self) -> bool {
        matches!(self, Self::H1 | Self::H2 | Self::H3)
    }
}
#[derive(Clone, Default, Debug, PartialEq, Eq)]
struct Block {
    kind: Kind,
    depth: u8,
    text: String,
    style: Vec<Inline>,
}
fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
fn safe_url(u: &str) -> Option<String> {
    let u = u.trim();
    if u.is_empty()
        || u.chars().any(|c| c.is_control() || c.is_whitespace())
        || u.starts_with("//")
        || u.contains('\\')
    {
        return None;
    }
    if ["https://", "http://", "mailto:", "#", "/", "./", "../"]
        .iter()
        .any(|p| u.starts_with(p))
    {
        Some(u.to_owned())
    } else if !u.contains(':') && u.contains('.') {
        Some(format!("https://{u}"))
    } else {
        None
    }
}
fn append(out: &mut Block, s: &str, style: &Inline) {
    out.text.push_str(s);
    out.style
        .extend(std::iter::repeat_n(style.clone(), s.chars().count()))
}
fn parse_inline(mut s: &str, style: Inline, out: &mut Block) {
    while !s.is_empty() {
        if let Some(rest) = s.strip_prefix('[')
            && let Some((name, tail)) = rest.split_once("](")
            && let Some((url, more)) = tail.split_once(')')
            && let Some(url) = safe_url(url)
        {
            let mut link = style.clone();
            link.link = url;
            parse_inline(name, link, out);
            s = more;
            continue;
        }
        let mut matched = false;
        for (m, kind) in [
            ("**", Mark::Bold),
            ("~~", Mark::Strike),
            ("_", Mark::Italic),
            ("`", Mark::Code),
        ] {
            if let Some(rest) = s.strip_prefix(m)
                && let Some((content, more)) = rest.split_once(m)
                && !content.is_empty()
            {
                let mut mark = style.clone();
                mark.flags |= kind.flag();
                if kind == Mark::Code {
                    append(out, content, &mark)
                } else {
                    parse_inline(content, mark, out)
                }
                s = more;
                matched = true;
                break;
            }
        }
        if matched {
            continue;
        }
        let ch = s.chars().next().unwrap();
        out.text.push(ch);
        out.style.push(style.clone());
        s = &s[ch.len_utf8()..]
    }
}
fn parse_doc(md: &str) -> Vec<Block> {
    let mut blocks = vec![];
    let mut code: Option<String> = None;
    for line in md.lines() {
        if line.trim_start().starts_with("```") {
            if let Some(txt) = code.take() {
                let mut b = Block {
                    kind: Kind::Code,
                    ..Default::default()
                };
                append(&mut b, txt.trim_end_matches('\n'), &Inline::default());
                blocks.push(b)
            } else {
                code = Some(String::new())
            }
            continue;
        }
        if let Some(txt) = &mut code {
            txt.push_str(line);
            txt.push('\n');
            continue;
        }
        if line.trim().is_empty() {
            continue;
        }
        let mut b = Block::default();
        let mut body = line;
        for k in [
            Kind::H3,
            Kind::H2,
            Kind::H1,
            Kind::List,
            Kind::Order,
            Kind::Quote,
        ] {
            if let Some(rem) = line.trim_start().strip_prefix(k.prefix()) {
                b.kind = k;
                b.depth = ((line.len() - line.trim_start().len()) / 2).min(4) as u8;
                body = rem;
                break;
            }
        }
        parse_inline(body, Inline::default(), &mut b);
        blocks.push(b)
    }
    if let Some(t) = code {
        let mut b = Block {
            kind: Kind::Code,
            ..Default::default()
        };
        append(&mut b, t.trim_end_matches('\n'), &Inline::default());
        blocks.push(b)
    }
    if blocks.is_empty() {
        blocks.push(Block::default())
    }
    blocks
}
fn runs_for(block: &Block, s: &str, base: &TextRun) -> Vec<TextRun> {
    let mut runs = vec![];
    for (i, c) in s.char_indices().enumerate() {
        let sty = block.style.get(i).cloned().unwrap_or_default();
        let mut r = base.clone();
        r.len = c.1.len_utf8();
        if sty.flags & 1 != 0 || block.kind.heading() {
            r.font.weight = FontWeight::SEMIBOLD
        }
        if sty.flags & 2 != 0 {
            r.font.style = FontStyle::Italic
        }
        if sty.flags & 4 != 0 || block.kind == Kind::Code {
            r.background_color = Some(base.color.opacity(0.09))
        }
        if sty.flags & 8 != 0 {
            r.strikethrough = Some(StrikethroughStyle {
                thickness: px(1.),
                ..Default::default()
            })
        }
        if !sty.link.is_empty() {
            r.underline = Some(UnderlineStyle {
                thickness: px(1.),
                ..Default::default()
            })
        }
        if let Some(last) = runs.last_mut() {
            let last: &mut TextRun = last;
            if last.font == r.font
                && last.background_color == r.background_color
                && last.underline == r.underline
                && last.strikethrough == r.strikethrough
            {
                last.len += r.len;
                continue;
            }
        }
        runs.push(r)
    }
    if runs.is_empty() {
        runs.push(base.clone())
    }
    runs
}
impl Block {
    fn indices(&self, r: Range<usize>) -> Range<usize> {
        self.text[..r.start].chars().count()..self.text[..r.end].chars().count()
    }
    fn splice(&mut self, text: String, typing: Option<Inline>) {
        let old = self.text.chars().collect::<Vec<_>>();
        let new = text.chars().collect::<Vec<_>>();
        let pre = old.iter().zip(&new).take_while(|(a, b)| a == b).count();
        let post = old[pre..]
            .iter()
            .rev()
            .zip(new[pre..].iter().rev())
            .take_while(|(a, b)| a == b)
            .count();
        let st = typing
            .or_else(|| pre.checked_sub(1).and_then(|a| self.style.get(a).cloned()))
            .unwrap_or_default();
        self.style.splice(
            pre..old.len() - post,
            std::iter::repeat_n(st, new.len() - pre - post),
        );
        self.text = text
    }
    fn body(&self) -> (String, String) {
        let mut md = String::new();
        let mut html = String::new();
        let chars = self.text.chars().collect::<Vec<_>>();
        let mut i = 0;
        while i < chars.len() {
            let attr = &self.style[i];
            let mut j = i + 1;
            while j < chars.len() && self.style[j] == *attr {
                j += 1
            }
            let mut m = chars[i..j].iter().collect::<String>();
            let mut h = esc(&m).replace('\n', "<br>");
            for (flag, sym, tag) in [
                (4, "`", "code"),
                (2, "_", "em"),
                (1, "**", "strong"),
                (8, "~~", "s"),
            ] {
                if attr.flags & flag != 0 {
                    m = format!("{sym}{m}{sym}");
                    h = format!("<{tag}>{h}</{tag}>")
                }
            }
            if !attr.link.is_empty() {
                m = format!("[{m}]({})", attr.link);
                h = format!("<a href=\"{}\">{h}</a>", esc(&attr.link))
            }
            md.push_str(&m);
            html.push_str(&h);
            i = j
        }
        (md, html)
    }
}
impl RichDocument {
    pub fn from_markdown(md: &str) -> Self {
        Self::from_blocks(&parse_doc(md))
    }
    fn from_blocks(blocks: &[Block]) -> Self {
        let mut md = String::new();
        let mut html = String::new();
        let mut text = String::new();
        let mut list = "";
        for (i, b) in blocks.iter().enumerate() {
            let (m, h) = b.body();
            let next = match b.kind {
                Kind::List => "ul",
                Kind::Order => "ol",
                _ => "",
            };
            if next != list {
                if !list.is_empty() {
                    html.push_str(&format!("</{list}>"))
                }
                if !next.is_empty() {
                    html.push_str(&format!("<{next}>"))
                }
                list = next
            }
            if i > 0 {
                md.push_str(if b.kind.list() && blocks[i - 1].kind == b.kind {
                    "\n"
                } else {
                    "\n\n"
                });
                text.push('\n')
            }
            text.push_str(&b.text);
            if b.kind == Kind::Code {
                md.push_str(&format!("```\n{}\n```", b.text));
                html.push_str(&format!("<pre><code>{}</code></pre>", esc(&b.text)))
            } else {
                if b.kind.list() {
                    md.push_str(&"  ".repeat(b.depth.into()))
                }
                md.push_str(b.kind.prefix());
                md.push_str(&m);
                let tag = b.kind.tag();
                html.push_str(&format!("<{tag}>{h}</{tag}>"))
            }
        }
        if !list.is_empty() {
            html.push_str(&format!("</{list}>"))
        }
        Self {
            markdown: md,
            html,
            empty: text.trim().is_empty(),
            text,
        }
    }
}
struct Line {
    id: u64,
    data: Rc<RefCell<Block>>,
    edit: Entity<Editor>,
    _subs: Vec<Subscription>,
}
#[derive(Clone)]
struct Snapshot {
    blocks: Vec<Block>,
    index: usize,
    selection: Range<usize>,
}
pub struct RichTextEditor {
    label: SharedString,
    lines: Vec<Line>,
    seq: u64,
    active: usize,
    selection: Range<usize>,
    typing: Option<Inline>,
    html: bool,
    doc: RichDocument,
    history: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    slash: Option<usize>,
    link_open: bool,
    link: Entity<Editor>,
    _link_sub: Subscription,
    toolbar: FormReveal,
    switch: Slide,
    slash_reveal: FormReveal,
    last_slash: Vec<(Kind, SharedString)>,
    notice: SharedString,
    document_width: Rc<Cell<Pixels>>,
    source_scroll: ScrollHandle,
    source_focus: FocusHandle,
}
impl EventEmitter<RichDocument> for RichTextEditor {}
impl RichTextEditor {
    pub fn new(
        label: impl Into<SharedString>,
        markdown: &str,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let link = cx.new(|cx| {
            let mut e = Editor::new(tr(cx, "Link URL", "链接地址"), InputMode::Single, w, cx);
            e.set_placeholder("https://");
            e
        });
        let sub = cx.subscribe_in(
            &link,
            w,
            |s: &mut Self, _, ev: &EditEvent, w, cx| match ev {
                EditEvent::Submitted => s.apply_link(w, cx),
                EditEvent::Escape => s.close_link(w, cx),
                _ => {}
            },
        );
        let mut s = Self {
            label: label.into(),
            seq: 0,
            lines: vec![],
            active: 0,
            selection: 0..0,
            typing: None,
            html: false,
            doc: RichDocument::from_markdown(markdown),
            history: vec![],
            redo: vec![],
            slash: None,
            link_open: false,
            link,
            _link_sub: sub,
            toolbar: FormReveal::new(false).above(),
            switch: Slide::snappy(0.),
            slash_reveal: FormReveal::new(false),
            last_slash: vec![],
            notice: "".into(),
            document_width: Default::default(),
            source_scroll: ScrollHandle::new(),
            source_focus: cx.focus_handle().tab_stop(true),
        };
        for b in parse_doc(markdown) {
            let line = s.create(b, w, cx);
            s.lines.push(line)
        }
        s
    }
    pub fn document(&self) -> &RichDocument {
        &self.doc
    }
    fn blocks(&self) -> Vec<Block> {
        self.lines.iter().map(|l| l.data.borrow().clone()).collect()
    }
    fn snapshot(&self) -> Snapshot {
        Snapshot {
            blocks: self.blocks(),
            index: self.active,
            selection: self.selection.clone(),
        }
    }
    fn checkpoint(&mut self) {
        let snap = self.snapshot();
        if self.history.last().is_none_or(|s| s.blocks != snap.blocks) {
            self.history.push(snap);
            if self.history.len() > 100 {
                self.history.remove(0);
            }
        }
        self.redo.clear()
    }
    fn publish(&mut self, cx: &mut Context<Self>) {
        let v = RichDocument::from_blocks(&self.blocks());
        if self.doc != v {
            self.doc = v.clone();
            cx.emit(v)
        }
        cx.notify()
    }
    fn create(&mut self, block: Block, w: &mut Window, cx: &mut Context<Self>) -> Line {
        let id = self.seq;
        self.seq += 1;
        let data = Rc::new(RefCell::new(block));
        let source = data.clone();
        let e = cx.new(|cx| {
            let b = data.borrow();
            let mut e = Editor::new(
                format!("{} · {}", self.label, id + 1),
                InputMode::InlineMulti,
                w,
                cx,
            );
            e.font_size = b.kind.size();
            e.set_value(b.text.clone(), w, cx);
            e.set_placeholder(tr(cx, "Type / for blocks", "输入 / 选择内容块"));
            e.set_run_styler(Rc::new(move |text, base| {
                runs_for(&source.borrow(), text, base)
            }));
            e
        });
        let sub = cx.subscribe_in(&e, w, move |s, e, v: &EditEvent, w, cx| {
            let Some(idx) = s.lines.iter().position(|l| l.id == id) else {
                return;
            };
            match v {
                EditEvent::Focused => {
                    if idx != s.active {
                        s.typing = None;
                        s.link_open = false
                    }
                    s.active = idx;
                    s.selection = e.read(cx).selection();
                    cx.notify()
                }
                EditEvent::Changed => {
                    let text = e.read(cx).value().to_owned();
                    if s.lines[idx].data.borrow().text == text {
                        return;
                    }
                    s.checkpoint();
                    s.active = idx;
                    s.lines[idx]
                        .data
                        .borrow_mut()
                        .splice(text, s.typing.clone());
                    s.selection = e.read(cx).selection();
                    let auto = {
                        let b = s.lines[idx].data.borrow();
                        BLOCKS.iter().map(|(k, _, _)| *k).find(|k| {
                            !k.prefix().is_empty()
                                && b.kind == Kind::Paragraph
                                && b.text == k.prefix()
                        })
                    };
                    if let Some(kind) = auto {
                        s.block_type(kind, true, w, cx);
                        return;
                    }
                    s.slash = s.slash_term().map(|_| 0);
                    s.publish(cx)
                }
                EditEvent::Submitted => s.split(w, cx),
                EditEvent::Escape => {
                    s.slash = None;
                    s.close_link(w, cx)
                }
                _ => {}
            }
        });
        let observe = cx.observe_in(&e, w, move |s, e, w, cx| {
            if !e.read(cx).focus.is_focused(w) {
                return;
            }
            if let Some(i) = s.lines.iter().position(|l| l.id == id) {
                let r = e.read(cx).selection();
                if (s.active != i || s.selection != r)
                    && r.end <= s.lines[i].data.borrow().text.len()
                {
                    s.active = i;
                    s.selection = r;
                    s.typing = None;
                    cx.notify()
                }
            }
        });
        Line {
            id,
            data,
            edit: e,
            _subs: vec![sub, observe],
        }
    }
    fn restore_focus(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let l = &self.lines[self.active];
        let text = l.data.borrow().text.clone();
        let r = self.selection.clone();
        if r.end > text.len() {
            return;
        }
        let units = text[..r.start].encode_utf16().count()..text[..r.end].encode_utf16().count();
        l.edit.update(cx, |e, cx| {
            e.focus(w, cx);
            e.set_selected_text_range(units, w, cx)
        });
    }
    fn sync_line(&self, index: usize, w: &mut Window, cx: &mut Context<Self>) {
        let l = &self.lines[index];
        let b = l.data.borrow();
        l.edit.update(cx, |e, cx| {
            e.font_size = b.kind.size();
            e.set_value(b.text.clone(), w, cx);
            cx.notify()
        })
    }
    fn undo(&mut self, redo: bool, w: &mut Window, cx: &mut Context<Self>) {
        let snap = if redo {
            self.redo.pop()
        } else {
            self.history.pop()
        };
        if let Some(v) = snap {
            let now = self.snapshot();
            if redo {
                self.history.push(now)
            } else {
                self.redo.push(now)
            }
            self.lines.clear();
            for b in v.blocks {
                let line = self.create(b, w, cx);
                self.lines.push(line)
            }
            self.active = v.index.min(self.lines.len() - 1);
            self.selection = v.selection;
            self.typing = None;
            self.slash = None;
            self.link_open = false;
            self.restore_focus(w, cx);
            self.publish(cx)
        }
    }
    fn apply_mark(&mut self, m: Mark, w: &mut Window, cx: &mut Context<Self>) {
        self.checkpoint();
        let l = &self.lines[self.active];
        {
            let mut b = l.data.borrow_mut();
            let r = b.indices(self.selection.clone());
            if r.is_empty() {
                let mut attr = self
                    .typing
                    .clone()
                    .or_else(|| r.start.checked_sub(1).and_then(|i| b.style.get(i).cloned()))
                    .unwrap_or_default();
                attr.flags ^= m.flag();
                self.typing = Some(attr)
            } else {
                let remove = b.style[r.clone()].iter().all(|s| s.flags & m.flag() != 0);
                for sty in &mut b.style[r] {
                    if remove {
                        sty.flags &= !m.flag()
                    } else {
                        sty.flags |= m.flag()
                    }
                }
            }
        }
        l.edit.update(cx, |_, cx| cx.notify());
        self.restore_focus(w, cx);
        self.publish(cx)
    }
    fn slash_term(&self) -> Option<String> {
        let b = self.lines.get(self.active)?.data.borrow();
        let before = b.text.get(..self.selection.end)?;
        let (_, term) = before.rsplit_once('/')?;
        let preceding = &before[..before.len() - term.len() - 1];
        (self.selection.is_empty()
            && (preceding.is_empty() || preceding.ends_with(char::is_whitespace))
            && !term.contains(char::is_whitespace))
        .then(|| term.to_lowercase())
    }
    fn slash_items(&self, cx: &App) -> Vec<(Kind, &'static str)> {
        let q = self.slash_term().unwrap_or_default();
        BLOCKS
            .iter()
            .filter(|(_, en, zh)| en.to_lowercase().contains(&q) || zh.contains(&q))
            .map(|(k, en, zh)| (*k, tr(cx, en, zh)))
            .collect()
    }
    fn block_type(&mut self, k: Kind, auto: bool, w: &mut Window, cx: &mut Context<Self>) {
        self.checkpoint();
        let term = self.slash_term();
        let i = self.active;
        {
            let mut b = self.lines[i].data.borrow_mut();
            if auto {
                b.text.clear();
                b.style.clear();
                self.selection = 0..0
            } else if let Some(q) = term {
                let start = self.selection.end.saturating_sub(q.len() + 1);
                let bytes = start..self.selection.end;
                let r = b.indices(bytes.clone());
                b.text.replace_range(bytes, "");
                b.style.drain(r);
                self.selection = start..start
            }
            b.kind = k
        }
        self.sync_line(i, w, cx);
        self.slash = None;
        self.restore_focus(w, cx);
        self.publish(cx)
    }
    fn split(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        self.checkpoint();
        let i = self.active;
        let mut new = Block::default();
        {
            let mut b = self.lines[i].data.borrow_mut();
            if b.text.is_empty() && b.kind != Kind::Paragraph {
                drop(b);
                self.block_type(Kind::Paragraph, false, w, cx);
                return;
            }
            let r = self.selection.clone();
            let indexes = b.indices(r.clone());
            new.kind = if b.kind.list() || b.kind == Kind::Quote {
                b.kind
            } else {
                Kind::Paragraph
            };
            new.depth = b.depth;
            new.text = b.text.split_off(r.end);
            b.text.truncate(r.start);
            new.style = b.style.split_off(indexes.end);
            b.style.truncate(indexes.start)
        }
        self.sync_line(i, w, cx);
        let n = self.create(new, w, cx);
        self.lines.insert(i + 1, n);
        self.active = i + 1;
        self.selection = 0..0;
        self.slash = None;
        self.restore_focus(w, cx);
        self.publish(cx)
    }
    fn join(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let i = self.active;
        if self.lines[i].data.borrow().kind != Kind::Paragraph {
            self.block_type(Kind::Paragraph, false, w, cx);
            return;
        }
        if i == 0 {
            return;
        }
        self.checkpoint();
        let line = self.lines.remove(i);
        let old = line.data.borrow();
        {
            let mut prev = self.lines[i - 1].data.borrow_mut();
            let at = prev.text.len();
            prev.text.push_str(&old.text);
            prev.style.extend_from_slice(&old.style);
            self.selection = at..at
        }
        self.active = i - 1;
        self.sync_line(self.active, w, cx);
        self.restore_focus(w, cx);
        self.publish(cx)
    }
    fn open_link(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let l = &self.lines[self.active];
        let b = l.data.borrow();
        let mut r = b.indices(self.selection.clone());
        if r.is_empty() && !b.style.is_empty() {
            let i = r.start.saturating_sub(1);
            let url = &b.style[i].link;
            if url.is_empty() {
                return;
            }
            r = i..i + 1;
            while r.start > 0 && b.style[r.start - 1].link == *url {
                r.start -= 1
            }
            while r.end < b.style.len() && b.style[r.end].link == *url {
                r.end += 1
            }
            let bytes = b
                .text
                .char_indices()
                .map(|(i, _)| i)
                .chain(std::iter::once(b.text.len()))
                .collect::<Vec<_>>();
            self.selection = bytes[r.start]..bytes[r.end]
        }
        if r.is_empty() {
            return;
        }
        let url = b.style[r.start].link.clone();
        drop(b);
        self.link_open = true;
        self.link.update(cx, |e, cx| {
            e.set_value(url, w, cx);
            e.focus(w, cx);
            e.select_all(w, cx)
        });
        cx.notify()
    }
    fn close_link(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.link_open {
            self.link_open = false;
            self.restore_focus(w, cx)
        }
        cx.notify()
    }
    fn apply_link(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let url = self.link.read(cx).value().trim();
        let val = if url.is_empty() {
            String::new()
        } else if let Some(u) = safe_url(url) {
            u
        } else {
            self.notice = tr(cx, "Enter a valid web address", "请输入有效网址").into();
            cx.notify();
            return;
        };
        self.checkpoint();
        {
            let mut b = self.lines[self.active].data.borrow_mut();
            let r = b.indices(self.selection.clone());
            for s in &mut b.style[r] {
                s.link = val.clone()
            }
        }
        self.notice = "".into();
        self.close_link(w, cx);
        self.publish(cx)
    }
    fn key(&mut self, e: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        let k = e.keystroke.key.as_str();
        let m = e.keystroke.modifiers;
        if self.link_open {
            return;
        }
        if let Some(i) = self.slash {
            let items = self.slash_items(cx);
            match k {
                "escape" => self.slash = None,
                "down" if !items.is_empty() => self.slash = Some((i + 1) % items.len()),
                "up" if !items.is_empty() => self.slash = Some((i + items.len() - 1) % items.len()),
                "enter" | "tab" if !items.is_empty() => {
                    self.block_type(items[i % items.len()].0, false, w, cx)
                }
                _ => return,
            }
        } else if m.control || m.platform {
            match k {
                "b" => self.apply_mark(Mark::Bold, w, cx),
                "i" => self.apply_mark(Mark::Italic, w, cx),
                "e" => self.apply_mark(Mark::Code, w, cx),
                "x" if m.shift => self.apply_mark(Mark::Strike, w, cx),
                "k" => self.open_link(w, cx),
                "z" => self.undo(m.shift, w, cx),
                "y" => self.undo(true, w, cx),
                "u" => {}
                _ => return,
            }
        } else if k == "backspace" && self.selection == (0..0) {
            self.join(w, cx)
        } else if k == "tab" && self.lines[self.active].data.borrow().kind.list() {
            self.checkpoint();
            {
                let mut b = self.lines[self.active].data.borrow_mut();
                b.depth = if m.shift {
                    b.depth.saturating_sub(1)
                } else {
                    (b.depth + 1).min(4)
                }
            }
            self.publish(cx)
        } else {
            return;
        }
        w.prevent_default();
        cx.stop_propagation();
        cx.notify()
    }
}
impl Render for RichTextEditor {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let width = (f32::from(w.viewport_size().width) - 32.).clamp(112., 500.);
        let measured_width = f32::from(self.document_width.get());
        let document_width = if measured_width > 0. {
            measured_width
        } else {
            width
        };
        let mut head = row().gap(px(3.));
        for (i, (g, en, zh, enabled)) in [
            (Glyph::Undo, "Undo", "撤销", !self.history.is_empty()),
            (Glyph::Redo, "Redo", "重做", !self.redo.is_empty()),
        ]
        .into_iter()
        .enumerate()
        {
            let b = glyph(("history", i), tr(cx, en, zh), g, cx).size(px(26.));
            head = head.child(if enabled {
                b.on_click(cx.listener(move |s, _, w, cx| s.undo(i == 1, w, cx)))
            } else {
                b.opacity(0.35).tab_stop(false).cursor_default()
            })
        }
        head = head.child(div().flex_1()).child(hint(
            format!(
                "{} {}",
                self.doc.text.split_whitespace().count(),
                tr(cx, "words", "词")
            ),
            cx,
        ));
        let mut shell = panel(cx).w_full().p(px(12.)).gap(px(9.)).child(head);
        let focused = self.lines[self.active].edit.read(cx).focus.is_focused(w);
        let show_toolbar = !self.selection.is_empty() && (focused || self.link_open);
        if focused && (show_toolbar || self.slash.is_some()) {
            let selection = self.lines[self.active].edit.update(cx, |e, cx| {
                let range = e.selected_text_range(true, w, cx)?.range;
                e.bounds_for_range(range, Bounds::default(), w, cx)
            });
            if let Some(b) = selection {
                self.toolbar.at(b);
                self.slash_reveal.at(b);
            }
        }
        let reveal = self.toolbar.sample(show_toolbar, 0.26, 0.144, w, cx);
        self.switch.duration(self.link_open as u8 as f32, 0.42, cx);
        let side = self.switch.get(w, cx).clamp(0., 1.);
        let slash = self
            .slash_reveal
            .sample(self.slash.is_some(), 0.26, 0.144, w, cx);
        if self.slash.is_some() {
            self.last_slash = self
                .slash_items(cx)
                .into_iter()
                .map(|(k, name)| (k, name.into()))
                .collect();
        }
        let mut number = 0;
        for (i, l) in self.lines.iter().enumerate() {
            let b = l.data.borrow();
            let text = b.text.clone();
            let sz = b.kind.size();
            let indent = f32::from(b.depth) * 12.;
            let chrome = if b.kind.list() {
                21. + indent
            } else if b.kind == Kind::Quote {
                2. + indent.max(7.)
            } else if b.kind == Kind::Code {
                10. + indent
            } else {
                indent
            };
            let runs = runs_for(&b, &text, &w.text_style().to_run(text.len()));
            let line = px(sz + 7.);
            let h = if text.is_empty() {
                line
            } else {
                w.text_system()
                    .shape_text(
                        text.into(),
                        px(sz),
                        &runs,
                        Some(px((document_width - 26. - chrome).max(1.))),
                        None,
                    )
                    .map(|rows| rows.iter().fold(px(0.), |n, r| n + r.size(line).height))
                    .unwrap_or(line)
                    .max(line)
            };
            let mut block = row()
                .w_full()
                .min_w_0()
                .gap(px(5.))
                .items_start()
                .when(b.kind == Kind::Quote, |d| {
                    d.border_l_2().border_color(p.line).pl(px(7.))
                })
                .when(b.kind == Kind::Code, |d| {
                    d.bg(p.hover)
                        .p(px(5.))
                        .rounded(crate::foundation::surface_radius(cx))
                })
                .when(b.depth > 0, |d| d.pl(px(f32::from(b.depth) * 12.)));
            if b.kind == Kind::Order {
                number += 1
            } else {
                number = 0
            }
            if b.kind.list() {
                block = block.child(
                    div()
                        .w(px(16.))
                        .flex_none()
                        .text_center()
                        .text_color(p.secondary)
                        .child(if b.kind == Kind::List {
                            "•".to_owned()
                        } else {
                            format!("{number}.")
                        }),
                )
            }
            block = block.child(div().flex_1().min_w_0().h(h).child(l.edit.clone()));
            shell = shell.child(block);
            if i == self.active && (show_toolbar || reveal > 0.) {
                let mut bar = div()
                    .relative()
                    .rounded(crate::foundation::surface_radius(cx))
                    .bg(p.tray)
                    .overflow_hidden();
                if self.link_open || side > 0. {
                    let link = row()
                        .p(px(3.))
                        .gap(px(2.))
                        .relative()
                        .left(px(18. * (1. - side)))
                        .opacity(side)
                        .child(
                            div()
                                .h(px(28.))
                                .min_w(px(100.))
                                .flex_1()
                                .child(self.link.clone()),
                        )
                        .child(
                            button("apply-link", tr(cx, "Apply", "应用"), cx)
                                .on_click(cx.listener(|s, _, w, cx| s.apply_link(w, cx))),
                        )
                        .child(
                            glyph("cancel-link", tr(cx, "Cancel", "取消"), Glyph::Close, cx)
                                .on_click(cx.listener(|s, _, w, cx| s.close_link(w, cx))),
                        );
                    bar = bar.child(if self.link_open {
                        link
                    } else {
                        link.absolute().top_0().w_full()
                    });
                }
                if !self.link_open || side < 1. {
                    let mut marks = row()
                        .p(px(3.))
                        .gap(px(2.))
                        .flex_wrap()
                        .relative()
                        .left(px(-18. * side))
                        .opacity(1. - side);
                    for (j, (label, name, zh, m)) in [
                        ("B", "Bold", "粗体", Mark::Bold),
                        ("I", "Italic", "斜体", Mark::Italic),
                        ("S", "Strikethrough", "删除线", Mark::Strike),
                        ("</>", "Inline code", "行内代码", Mark::Code),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        marks = marks.child(
                            button(("mark", j), label, cx)
                                .aria_label(tr(cx, name, zh))
                                .on_mouse_down(MouseButton::Left, |_, w, _| w.prevent_default())
                                .on_click(cx.listener(move |s, _, w, cx| s.apply_mark(m, w, cx))),
                        )
                    }
                    marks = marks.child(
                        glyph("link", tr(cx, "Edit link", "编辑链接"), Glyph::Link, cx)
                            .on_mouse_down(MouseButton::Left, |_, w, _| w.prevent_default())
                            .on_click(cx.listener(|s, _, w, cx| s.open_link(w, cx))),
                    );
                    bar = bar.child(if self.link_open {
                        marks.absolute().top_0().w_full()
                    } else {
                        marks
                    });
                }
                shell = shell.child(self.toolbar.popup(
                    bar,
                    reveal,
                    show_toolbar,
                    (164. + 90. * side, 38., 4.),
                    w,
                ))
            }
            if i == self.active && (self.slash.is_some() || slash > 0.) {
                let active = self.slash.unwrap_or(usize::MAX);
                let mut menu = panel(cx);
                let items = self.last_slash.clone();
                if items.is_empty() {
                    menu = menu.child(hint(tr(cx, "No matching blocks", "没有匹配的内容块"), cx))
                }
                for (j, (kind, label)) in items.into_iter().enumerate() {
                    menu = menu.child(
                        button(("block", j), label, cx)
                            .justify_start()
                            .when(j == active, |b| b.bg(p.hover))
                            .on_click(
                                cx.listener(move |s, _, w, cx| s.block_type(kind, false, w, cx)),
                            ),
                    )
                }
                shell = shell.child(self.slash_reveal.popup(
                    menu,
                    slash,
                    self.slash.is_some(),
                    (230., 200., 4.),
                    w,
                ))
            }
        }
        let mut bar = row().gap(px(3.));
        for (i, n) in ["Markdown", "HTML"].into_iter().enumerate() {
            bar = bar.child(
                button(("format", i), n, cx)
                    .when(self.html == (i == 1), |b| b.bg(p.hover))
                    .on_click(cx.listener(move |s, _, _, cx| {
                        s.html = i == 1;
                        s.source_scroll.set_offset(Point::default());
                        cx.notify()
                    })),
            )
        }
        bar = bar
            .child(div().flex_1())
            .child(
                button("copy", tr(cx, "Copy", "复制"), cx).on_click(cx.listener(|s, _, _, cx| {
                    let task =
                        clipboard::write(if s.html { &s.doc.html } else { &s.doc.markdown }, cx);
                    cx.spawn(async move |this, cx| {
                        let ok = task.await;
                        let _ = this.update(cx, |s, cx| {
                            s.notice = if ok {
                                tr(cx, "Copied", "已复制")
                            } else {
                                tr(cx, "Copy failed", "复制失败")
                            }
                            .into();
                            cx.notify()
                        });
                    })
                    .detach()
                })),
            );
        let source_scroll = self.source_scroll.clone();
        let document_width = self.document_width.clone();
        section(width, cx)
            .on_children_prepainted(move |children, w, _| {
                if let Some(child) = children.first()
                    && document_width.replace(child.size.width) != child.size.width
                {
                    w.request_animation_frame();
                }
            })
            .child(
                shell
                    .id("rich-document")
                    .capture_key_down(cx.listener(Self::key)),
            )
            .child(bar)
            .child(
                div()
                    .id("source")
                    .track_focus(&self.source_focus)
                    .tab_index(0)
                    .role(Role::Region)
                    .aria_label(if self.html { "HTML" } else { "Markdown" })
                    .relative()
                    .max_h(px(260.))
                    .overflow_y_scroll()
                    .track_scroll(&self.source_scroll)
                    .on_key_down(cx.listener(|s, event: &KeyDownEvent, w, cx| {
                        let scroll = &s.source_scroll;
                        let offset = scroll.offset();
                        let max = scroll.max_offset().y;
                        let page = (scroll.bounds().size.height - px(19.)).max(px(19.));
                        let y = match event.keystroke.key.as_str() {
                            "up" => offset.y + px(19.),
                            "down" => offset.y - px(19.),
                            "pageup" => offset.y + page,
                            "pagedown" => offset.y - page,
                            "home" => px(0.),
                            "end" => -max,
                            _ => return,
                        }
                        .clamp(-max, px(0.));
                        if y != offset.y {
                            scroll.set_offset(point(offset.x, y));
                            w.prevent_default();
                            cx.stop_propagation();
                            cx.notify();
                        }
                    }))
                    .rounded(crate::foundation::surface_radius(cx))
                    .border_1()
                    .border_color(transparent_black())
                    .focus_visible(move |style| style.border_color(p.focus))
                    .px(px(12.))
                    .py(px(10.))
                    .bg(p.tray)
                    .text_size(px(12.))
                    .line_height(px(19.))
                    .child(
                        canvas(
                            |_, _, _| (),
                            move |_, _, _, cx| {
                                if let Some(host) =
                                    cx.try_global::<crate::primitives::editor::EditorScrollRegion>()
                                {
                                    let top = -source_scroll.offset().y;
                                    (host.0)(
                                        source_scroll.bounds(),
                                        top > px(0.1),
                                        top + px(0.1) < source_scroll.max_offset().y,
                                    );
                                }
                            },
                        )
                        .absolute()
                        .size(px(0.)),
                    )
                    .child(if self.html {
                        self.doc.html.clone()
                    } else {
                        self.doc.markdown.clone()
                    }),
            )
            .when(!self.notice.is_empty(), |d| {
                d.child(hint(self.notice.clone(), cx))
            })
    }
}
#[cfg(test)]
mod tests {
    use super::{Block, Inline, Mark, RichDocument, RichTextEditor, parse_doc, safe_url};
    use gpui::{TestAppContext, px};

    #[test]
    fn keyboard_reaches_the_end_of_long_exports_and_returns_to_top() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let text = "Long paragraph with a line that wraps. ".repeat(55);
        let (editor, window) =
            app.add_window_view(|w, cx| RichTextEditor::new("Notes", &text, w, cx));
        window.update(|w, cx| editor.read(cx).source_focus.clone().focus(w, cx));
        window.run_until_parked();
        let max = window.update(|_, cx| editor.read(cx).source_scroll.max_offset().y);
        assert!(max > px(0.));
        window.simulate_keystrokes("end");
        assert_eq!(
            window.update(|_, cx| editor.read(cx).source_scroll.offset().y),
            -max
        );
        window.simulate_keystrokes("home");
        assert_eq!(
            window.update(|_, cx| editor.read(cx).source_scroll.offset().y),
            px(0.)
        );
    }
    #[test]
    fn preserves_markup_without_injecting_html() {
        let doc = RichDocument::from_markdown("# Title\n- <tag>\n- **last**\n> quote");
        assert_eq!(
            doc.html,
            "<h1>Title</h1><ul><li>&lt;tag&gt;</li><li><strong>last</strong></li></ul><blockquote>quote</blockquote>"
        );
        assert!(!doc.text.contains("**"));
        assert!(safe_url("javascript:alert(1)").is_none());
        assert_eq!(
            RichDocument::from_markdown("[open](https://example.org)").html,
            "<p><a href=\"https://example.org\">open</a></p>"
        )
    }
    #[test]
    fn unicode_insert_preserves_surrounding_marks() {
        let mut block = parse_doc("👋 **你好**!").remove(0);
        block.splice("👋 你们好!".into(), None);
        let (md, _) = block.body();
        assert_eq!(md, "👋 **你们好**!");
        let mut blank = Block::default();
        blank.splice(
            "中文".into(),
            Some(Inline {
                flags: Mark::Italic.flag(),
                ..Default::default()
            }),
        );
        assert_eq!(blank.body().0, "_中文_")
    }
}
