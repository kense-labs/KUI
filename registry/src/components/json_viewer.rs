//! Browse a typed JSON tree, find matching rows and copy their values or paths.
use crate::foundation::{Theme, surface_radius};
use crate::primitives::{
    clipboard,
    editor::{EditEvent, Editor, EditorScrollRegion, InputMode},
    family::{Paint, Slide},
    glyph::{Glyph, icon, transformed},
    nav::{BranchRows, RowFrame, button, column, fit, label, panel, row, tr},
};
use gpui::{prelude::*, *};
pub use serde_json::Value as JsonValue;
use std::{
    collections::{HashMap, HashSet},
    ops::Range,
    time::Duration,
};

const ROW: f32 = 30.;
const PAGE: usize = 50;

/// Result of a clipboard request. The original RFC 6901 pointer identifies the copied node.
pub struct JsonCopied {
    pub pointer: String,
    pub success: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Object,
    Array,
    String,
    Number,
    Boolean,
    Null,
}
impl Kind {
    fn branch(self) -> bool {
        matches!(self, Self::Object | Self::Array)
    }
    fn name(self) -> &'static str {
        match self {
            Self::Object => "object",
            Self::Array => "array",
            Self::String => "string",
            Self::Number => "number",
            Self::Boolean => "boolean",
            Self::Null => "null",
        }
    }
}

struct Node {
    key: String,
    path: String,
    pointer: String,
    value: String,
    kind: Kind,
    parent: Option<usize>,
    position: usize,
    depth: usize,
    children: Vec<usize>,
}

fn child_path(parent: &str, key: &str, index: bool) -> String {
    if index {
        return format!("{parent}[{key}]");
    }
    let mut chars = key.chars();
    let ident = |c: char| c == '_' || c == '$' || c.is_ascii_alphabetic();
    if chars.next().is_some_and(ident) && chars.all(|c| ident(c) || c.is_ascii_digit()) {
        format!("{parent}.{key}")
    } else {
        format!("{parent}[{}]", JsonValue::String(key.to_owned()))
    }
}

fn index_value(
    nodes: &mut Vec<Node>,
    v: &JsonValue,
    parent: Option<usize>,
    key: String,
    position: usize,
) -> usize {
    let (path, pointer, depth) = match parent {
        None => (key.clone(), String::new(), 0),
        Some(i) => {
            let p = &nodes[i];
            (
                child_path(&p.path, &key, p.kind == Kind::Array),
                format!(
                    "{}/{}",
                    p.pointer,
                    key.replace('~', "~0").replace('/', "~1")
                ),
                p.depth + 1,
            )
        }
    };
    let (kind, text) = match v {
        JsonValue::Object(o) => {
            let mut keys = o.keys().take(3).cloned().collect::<Vec<_>>();
            if o.len() > 3 {
                keys.push("…".into())
            }
            (
                Kind::Object,
                if keys.is_empty() {
                    "{}".to_owned()
                } else {
                    format!("{{ {} }}", keys.join(", "))
                },
            )
        }
        JsonValue::Array(a) => (Kind::Array, if a.is_empty() { "[]" } else { "[…]" }.into()),
        JsonValue::String(_) => (Kind::String, v.to_string()),
        JsonValue::Number(_) => (Kind::Number, v.to_string()),
        JsonValue::Bool(_) => (Kind::Boolean, v.to_string()),
        JsonValue::Null => (Kind::Null, "null".into()),
    };
    let id = nodes.len();
    nodes.push(Node {
        key,
        path,
        pointer,
        value: text,
        kind,
        parent,
        position,
        depth,
        children: vec![],
    });
    let children = match v {
        JsonValue::Array(a) => a
            .iter()
            .enumerate()
            .map(|(i, v)| index_value(nodes, v, Some(id), i.to_string(), i))
            .collect(),
        JsonValue::Object(o) => o
            .iter()
            .enumerate()
            .map(|(i, (k, v))| index_value(nodes, v, Some(id), k.clone(), i))
            .collect(),
        _ => vec![],
    };
    nodes[id].children = children;
    id
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Row {
    Node(usize),
    More(usize),
}
impl Row {
    fn id(self) -> usize {
        match self {
            Self::Node(i) | Self::More(i) => i,
        }
    }
}

fn highlights(text: &str, needle: &str) -> Vec<Range<usize>> {
    if needle.is_empty() {
        return vec![];
    }
    // Case folding can change UTF-8 length. Map the folded bytes back to valid character boundaries.
    let mut lower = String::new();
    let mut offsets = Vec::new();
    for (i, c) in text.char_indices() {
        let begin = lower.len();
        lower.extend(c.to_lowercase());
        offsets.resize(offsets.len() + lower.len() - begin, (i, i + c.len_utf8()));
    }
    lower
        .match_indices(needle)
        .map(|(i, s)| offsets[i].0..offsets[i + s.len() - 1].1)
        .collect()
}

/// Searchable, paged JSON inspector. The data is indexed once, in its supplied iteration order.
pub struct JsonViewer {
    value: JsonValue,
    nodes: Vec<Node>,
    open: HashSet<usize>,
    pages: HashMap<usize, usize>,
    query: Entity<Editor>,
    needle: String,
    matches: Vec<usize>,
    match_index: usize,
    search_pages: HashMap<usize, usize>,
    search_closed: HashSet<usize>,
    _sub: Subscription,
    selected: Row,
    hovered: Option<usize>,
    visible: Vec<Row>,
    rows: BranchRows<Row>,
    focus: FocusHandle,
    scroll: ScrollHandle,
    flight: Option<(Slide, f32)>,
    marker: Option<Slide>,
    chevrons: HashMap<usize, Slide>,
    mono: SharedString,
    copied: Option<(usize, bool, bool)>,
    copy_generation: u64,
    max_height: f32,
}
impl EventEmitter<JsonCopied> for JsonViewer {}
impl JsonViewer {
    pub fn new(
        title: impl Into<SharedString>,
        value: JsonValue,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let title = title.into();
        let mut nodes = Vec::new();
        index_value(&mut nodes, &value, None, title.to_string(), 0);
        let query = cx.new(|cx| {
            let mut field =
                Editor::new(tr(cx, "Search JSON", "搜索 JSON"), InputMode::Search, w, cx);
            field.set_placeholder(tr(cx, "Search keys and values", "搜索键和值"));
            field
        });
        let sub = cx.subscribe_in(
            &query,
            w,
            |s: &mut Self, _, ev: &EditEvent, w, cx| match ev {
                EditEvent::Changed => s.search(cx),
                EditEvent::Submitted => s.next_match(if w.modifiers().shift { -1 } else { 1 }, cx),
                EditEvent::Escape => s.query.update(cx, |e, cx| e.set_value("", w, cx)),
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
        let mut viewer = Self {
            value,
            nodes,
            query,
            _sub: sub,
            open: HashSet::from([0]),
            pages: Default::default(),
            needle: String::new(),
            matches: vec![],
            match_index: 0,
            search_pages: Default::default(),
            search_closed: Default::default(),
            selected: Row::Node(0),
            hovered: None,
            visible: vec![],
            rows: BranchRows::smooth_height(),
            focus: cx.focus_handle(),
            scroll: ScrollHandle::new(),
            flight: None,
            marker: None,
            chevrons: Default::default(),
            mono,
            copied: None,
            copy_generation: 0,
            max_height: 420.,
        };
        viewer.rebuild();
        viewer
    }

    /// Limit the height of the independently scrollable tree, in logical pixels.
    pub fn with_max_height(mut self, height: f32) -> Self {
        if height.is_finite() {
            self.max_height = height.max(ROW)
        }
        self
    }
    /// Choose an already registered monospace font for keys, values and paths.
    pub fn with_font_family(mut self, family: impl Into<SharedString>) -> Self {
        self.mono = family.into();
        self
    }
    pub fn selected_pointer(&self) -> &str {
        &self.nodes[self.selected.id()].pointer
    }
    pub fn selected_path(&self) -> &str {
        &self.nodes[self.selected.id()].path
    }
    fn is_open(&self, i: usize) -> bool {
        !self.search_closed.contains(&i)
            && (self.open.contains(&i) || self.search_pages.contains_key(&i))
    }
    fn limit(&self, i: usize) -> usize {
        self.pages
            .get(&i)
            .copied()
            .unwrap_or(PAGE)
            .max(self.search_pages.get(&i).copied().unwrap_or(0))
    }
    fn rebuild(&mut self) {
        let mut todo = vec![Row::Node(0)];
        self.visible.clear();
        while let Some(r) = todo.pop() {
            self.visible.push(r);
            if let Row::Node(i) = r
                && self.is_open(i)
            {
                let cs = &self.nodes[i].children;
                let limit = self.limit(i).min(cs.len());
                if limit < cs.len() {
                    todo.push(Row::More(i));
                }
                todo.extend(cs[..limit].iter().rev().copied().map(Row::Node));
            }
        }
        while !self.visible.contains(&self.selected) {
            self.selected = match self.selected {
                Row::More(i) => Row::Node(i),
                Row::Node(i) => Row::Node(self.nodes[i].parent.unwrap_or(0)),
            };
            self.marker = None;
        }
    }
    fn search(&mut self, cx: &mut Context<Self>) {
        let needle = self.query.read(cx).value().trim().to_lowercase();
        if needle == self.needle {
            return;
        }
        self.needle = needle;
        self.search_pages.clear();
        self.search_closed.clear();
        self.matches.clear();
        self.match_index = 0;
        if !self.needle.is_empty() {
            for (i, n) in self.nodes.iter().enumerate() {
                let key_hit = n.parent.is_some_and(|p| self.nodes[p].kind != Kind::Array)
                    && n.key.to_lowercase().contains(&self.needle);
                let value_hit = !n.kind.branch()
                    && self.value.pointer(&n.pointer).is_some_and(|v| {
                        v.as_str()
                            .unwrap_or(&n.value)
                            .to_lowercase()
                            .contains(&self.needle)
                    });
                if !key_hit && !value_hit {
                    continue;
                }
                self.matches.push(i);
                let mut id = i;
                while let Some(p) = self.nodes[id].parent {
                    let required = self.nodes[id].position + 1;
                    let limit = self.search_pages.entry(p).or_default();
                    *limit = (*limit).max(required);
                    id = p
                }
                if self.matches.len() == 2000 {
                    break;
                }
            }
        }
        self.rebuild();
        if let Some(&id) = self.matches.first() {
            self.selected = Row::Node(id)
        }
        self.reveal(!self.matches.is_empty(), cx);
        cx.notify()
    }
    fn next_match(&mut self, step: isize, cx: &mut Context<Self>) {
        let n = self.matches.len();
        if n == 0 {
            return;
        }
        self.match_index = (self.match_index as isize + step).rem_euclid(n as isize) as usize;
        let id = self.matches[self.match_index];
        let mut p = self.nodes[id].parent;
        while let Some(i) = p {
            self.search_closed.remove(&i);
            p = self.nodes[i].parent;
        }
        self.rebuild();
        self.selected = Row::Node(id);
        self.reveal(true, cx);
        cx.notify()
    }
    fn reveal(&mut self, center: bool, cx: &App) {
        let Some(i) = self.visible.iter().position(|r| *r == self.selected) else {
            return;
        };
        let current = -f32::from(self.scroll.offset().y);
        let view = f32::from(self.scroll.bounds().size.height).max(ROW);
        let top = 6. + ROW * i as f32;
        let target = if center {
            top - (view - ROW) / 2.
        } else if top < current + 4. {
            top - 4.
        } else if top + ROW > current + view - 4. {
            top + ROW + 4. - view
        } else {
            current
        };
        let target = target.clamp(0., (12. + ROW * self.visible.len() as f32 - view).max(0.));
        let (slide, destination) = self
            .flight
            .get_or_insert_with(|| (Slide::new(current), current));
        slide.duration(target, 0.24, cx);
        *destination = target;
    }
    fn activate(&mut self, row: Row, cx: &mut Context<Self>) {
        self.selected = row;
        let i = row.id();
        match row {
            Row::More(_) => {
                let start = self.limit(i);
                let Some(&first) = self.nodes[i].children.get(start) else {
                    return;
                };
                self.pages.insert(i, start.saturating_add(PAGE));
                self.selected = Row::Node(first);
            }
            Row::Node(_) if self.nodes[i].kind.branch() => {
                if self.is_open(i) {
                    self.open.remove(&i);
                    if self.search_pages.contains_key(&i) {
                        self.search_closed.insert(i);
                    }
                } else {
                    self.search_closed.remove(&i);
                    self.open.insert(i);
                }
            }
            _ => {}
        }
        self.rebuild();
        self.reveal(false, cx);
        cx.notify()
    }
    fn expand_all(&mut self, open: bool, cx: &mut Context<Self>) {
        self.search_closed.clear();
        self.open.clear();
        if open {
            self.open.extend(
                self.nodes
                    .iter()
                    .enumerate()
                    .filter(|(_, n)| n.kind.branch())
                    .map(|(i, _)| i),
            );
        } else {
            self.open.insert(0);
            self.pages.clear();
            self.search_closed
                .extend(self.search_pages.keys().copied().filter(|i| *i != 0));
            self.selected = Row::Node(0);
        }
        self.rebuild();
        self.reveal(false, cx);
        cx.notify()
    }
    fn copy(&mut self, id: usize, path_only: bool, cx: &mut Context<Self>) {
        let n = &self.nodes[id];
        let Some(v) = self.value.pointer(&n.pointer) else {
            return;
        };
        let text = if path_only {
            n.path.clone()
        } else if let Some(s) = v.as_str() {
            s.to_owned()
        } else {
            serde_json::to_string_pretty(v).unwrap_or_default()
        };
        let pointer = n.pointer.clone();
        self.copy_generation = self.copy_generation.wrapping_add(1);
        let generation = self.copy_generation;
        let write = clipboard::write(&text, cx);
        cx.spawn(async move |weak, cx| {
            let ok = write.await;
            let _ = weak.update(cx, |s, cx| {
                cx.emit(JsonCopied {
                    pointer,
                    success: ok,
                });
                if s.copy_generation == generation {
                    s.copied = Some((id, path_only, ok));
                    cx.notify()
                }
            });
            cx.background_executor()
                .timer(Duration::from_millis(1400))
                .await;
            let _ = weak.update(cx, |s, cx| {
                if s.copy_generation == generation {
                    s.copied = None;
                    cx.notify()
                }
            });
        })
        .detach();
    }
    fn key(&mut self, e: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        let key = e.keystroke.key.as_str();
        if self.query.read(cx).focus.is_focused(w) {
            return;
        }
        let Some(pos) = self.visible.iter().position(|r| *r == self.selected) else {
            return;
        };
        let row = self.selected;
        let id = row.id();
        let next = match key {
            "/" => {
                self.query.update(cx, |v, cx| v.focus(w, cx));
                None
            }
            "c" if e.keystroke.modifiers.platform || e.keystroke.modifiers.control => {
                if matches!(row, Row::Node(_)) {
                    self.copy(id, e.keystroke.modifiers.shift, cx)
                }
                None
            }
            "up" => Some(self.visible[pos.saturating_sub(1)]),
            "down" => self.visible.get(pos + 1).copied(),
            "home" => self.visible.first().copied(),
            "end" => self.visible.last().copied(),
            "right" if matches!(row, Row::Node(_)) && self.nodes[id].kind.branch() => {
                if self.is_open(id) {
                    self.nodes[id].children.first().copied().map(Row::Node)
                } else {
                    self.activate(row, cx);
                    None
                }
            }
            "left"
                if matches!(row, Row::Node(_))
                    && self.nodes[id].kind.branch()
                    && self.is_open(id) =>
            {
                self.activate(row, cx);
                None
            }
            "left" => match row {
                Row::More(i) => Some(Row::Node(i)),
                Row::Node(i) => self.nodes[i].parent.map(Row::Node),
            },
            "enter" | "space" => {
                self.activate(row, cx);
                None
            }
            _ => return,
        };
        if let Some(next) = next {
            self.selected = next;
            self.reveal(false, cx);
            cx.notify()
        }
        w.prevent_default();
        cx.stop_propagation()
    }

    fn count_label(n: &Node, cx: &App) -> String {
        let c = n.children.len();
        let name = if n.kind == Kind::Array {
            tr(cx, if c == 1 { "item" } else { "items" }, "项")
        } else {
            tr(cx, if c == 1 { "key" } else { "keys" }, "个键")
        };
        format!("{c} {name}")
    }
    fn code(&self, text: &str, color: Hsla, mark: Option<Hsla>, px_size: f32) -> Div {
        let runs = mark.into_iter().flat_map(|bg| {
            highlights(text, &self.needle).into_iter().map(move |r| {
                (
                    r,
                    HighlightStyle {
                        background_color: Some(bg),
                        ..Default::default()
                    },
                )
            })
        });
        div()
            .min_w_0()
            .truncate()
            .font_family(self.mono.clone())
            .text_size(px(px_size))
            .line_height(px(19.))
            .text_color(color)
            .child(StyledText::new(text.to_owned()).with_highlights(runs))
    }
    fn copy_key(
        &self,
        id: usize,
        path: bool,
        footer: bool,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let status = self.copied.filter(|(i, p, _)| *i == id && *p == path);
        let name = match status {
            Some((_, _, true)) => tr(cx, "Copied", "已复制"),
            Some((_, _, false)) => tr(cx, "Copy failed", "复制失败"),
            None if path => tr(cx, "Copy path", "复制路径"),
            None => tr(cx, "Copy value", "复制值"),
        };
        let p = Paint::get(cx);
        let (g, color) = match status {
            Some((_, _, true)) => (Glyph::Check, p.success),
            Some((_, _, false)) => (Glyph::Alert, p.danger),
            _ if path && !footer => (Glyph::Link, p.secondary),
            _ => (Glyph::Copy, p.secondary),
        };
        button(
            (
                if footer {
                    "copy-current"
                } else if path {
                    "copy-path"
                } else {
                    "copy-value"
                },
                id,
            ),
            name,
            cx,
        )
        .tab_index(if footer { 0 } else { -1 })
        .size(px(if footer { 30. } else { 26. }))
        .p_0()
        .flex_none()
        .child(icon(g, 14., color))
        .on_mouse_down(MouseButton::Left, |_, w, cx| {
            w.prevent_default();
            cx.stop_propagation()
        })
        .on_click(cx.listener(move |s, _, _, cx| {
            cx.stop_propagation();
            s.copy(id, path, cx)
        }))
    }
    fn row_view(
        &mut self,
        r: Row,
        a: RowFrame,
        live: bool,
        narrow: bool,
        w: &Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let i = r.id();
        let n = &self.nodes[i];
        let more = matches!(r, Row::More(_));
        let selected = r == self.selected;
        let hovered = self.hovered == Some(i) && !more;
        let depth = n.depth + usize::from(more);
        let p = Paint::get(cx);
        let parent = n.parent;
        let indent = if narrow { 12. } else { 16. };
        let label_name = if more {
            tr(cx, "Show more", "显示更多").to_owned()
        } else {
            n.key.clone()
        };
        let key = r;
        let mut line = row()
            .id((if more { "more" } else { "node" }, i))
            .relative()
            .w_full()
            .h(px(ROW - 2.))
            .flex_none()
            .rounded(surface_radius(cx))
            .cursor_pointer()
            .role(Role::TreeItem)
            .tab_stop(false)
            .aria_level(depth + 1)
            .aria_selected(selected)
            .aria_position_in_set(if more {
                self.limit(i) + 1
            } else {
                n.position + 1
            })
            .aria_size_of_set(if more {
                self.limit(i) + 1
            } else {
                parent.map_or(1, |p| self.nodes[p].children.len())
            })
            .aria_label(if more {
                label_name.clone()
            } else {
                format!("{}: {}", n.path, n.value)
            })
            .debug_selector(move || label_name.clone())
            .pl(px(4. + indent * depth.min(18) as f32))
            .pr(px(6.))
            .gap(px(6.))
            .when(!selected, |e| e.hover(|s| s.bg(p.hover.opacity(0.65))))
            .when(selected, |e| e.aria_active_descendant())
            .when(!more && n.kind.branch(), |e| {
                e.aria_expanded(self.is_open(i))
            })
            .on_hover(cx.listener(move |s, inside: &bool, _, cx| {
                if !live {
                    return;
                }
                let next = if *inside {
                    Some(i)
                } else if s.hovered == Some(i) {
                    None
                } else {
                    s.hovered
                };
                if next != s.hovered {
                    s.hovered = next;
                    cx.notify()
                }
            }))
            .on_click(cx.listener(move |s, _, w, cx| {
                if live {
                    s.focus.focus(w, cx);
                    s.activate(key, cx)
                }
            }));
        if more {
            let hidden = n.children.len().saturating_sub(self.limit(i));
            let caption = if tr(cx, "en", "zh") == "en" {
                format!("Show {} more", PAGE.min(hidden))
            } else {
                format!("再显示 {} 项", PAGE.min(hidden))
            };
            line = line
                .child(div().w(px(14.)))
                .child(label(caption, 13., p.ink))
                .child(label(
                    format!("{hidden} {}", tr(cx, "hidden", "项未显示")),
                    12.,
                    p.faint,
                ));
        } else {
            let branch = n.kind.branch();
            let is_open = self.is_open(i);
            let target = if is_open { 1. } else { 0. };
            let mut chev = div().size(px(14.)).flex_none();
            if branch {
                let tween = self.chevrons.entry(i).or_insert_with(|| Slide::new(target));
                tween.duration(target, 0.22, cx);
                chev = chev.child(transformed(
                    Glyph::Down,
                    14.,
                    if selected { p.ink } else { p.faint },
                    (tween.get(w, cx) - 1.) * std::f32::consts::FRAC_PI_2,
                    1.,
                    (0., 0.),
                    1.,
                ));
            }
            let t = &Theme::global(cx).tokens.colors;
            let mark = (!self.needle.is_empty()).then(|| {
                t.warning
                    .opacity(if self.matches.get(self.match_index) == Some(&i) {
                        0.48
                    } else {
                        0.22
                    })
            });
            let key_color = if parent.is_some_and(|p| self.nodes[p].kind == Kind::Array) {
                p.faint
            } else {
                p.ink
            };
            let mut contents = row().flex_1().overflow_hidden().gap(px(5.)).child(
                self.code(&n.key, key_color, mark, 13.)
                    .max_w(relative(if narrow { 0.40 } else { 0.45 }))
                    .flex_shrink_0(),
            );
            if parent.is_some() || !branch {
                contents =
                    contents.child(self.code(":", p.faint, None, 13.).ml(px(-4.)).flex_none())
            }
            if !branch || !is_open {
                let ink = match n.kind {
                    Kind::String => p.success,
                    Kind::Number => t.accent,
                    Kind::Boolean => t.warning,
                    _ => p.faint,
                };
                contents =
                    contents.child(self.code(&n.value, ink, if branch { None } else { mark }, 13.));
            }
            if branch {
                contents = contents.child(
                    label(Self::count_label(n, cx), 12., p.faint)
                        .whitespace_nowrap()
                        .flex_none(),
                );
            }
            line = line.child(chev).child(contents);
            if live && (selected || hovered) {
                line = line.child(
                    row()
                        .flex_none()
                        .child(self.copy_key(i, false, false, cx))
                        .child(self.copy_key(i, true, false, cx)),
                );
            }
        }
        div()
            .h(px(ROW * a.height))
            .w_full()
            .flex_none()
            .overflow_hidden()
            .pt(px(1.))
            .opacity(a.opacity)
            .child(line)
    }
    fn toolbar(&self, w: &Window, cx: &mut Context<Self>) -> Div {
        let p = Paint::get(cx);
        let mut search = row()
            .min_w_0()
            .flex_1()
            .h(px(34.))
            .gap(px(4.))
            .pl(px(10.))
            .pr(px(4.))
            .rounded(surface_radius(cx))
            .bg(p.stage)
            .child(icon(Glyph::Search, 15., p.faint))
            .child(div().h_full().min_w_0().flex_1().child(self.query.clone()))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|s, _, w, cx| s.query.read(cx).focus.clone().focus(w, cx)),
            );
        if !self.needle.is_empty() {
            let total = self.matches.len();
            let count = if total == 0 {
                "0/0".into()
            } else {
                format!("{}/{}", self.match_index + 1, total)
            };
            search = search.child(label(count, 12., p.secondary).flex_none());
            for (dir, title, g) in [
                (-1, tr(cx, "Previous match", "上一个匹配"), Glyph::Up),
                (1, tr(cx, "Next match", "下一个匹配"), Glyph::Down),
            ] {
                search = search.child(
                    button(title, title, cx)
                        .size(px(26.))
                        .flex_none()
                        .p_0()
                        .when(total == 0, |e| {
                            e.opacity(0.4).cursor_default().tab_index(-1)
                        })
                        .child(icon(g, 14., p.secondary))
                        .on_click(cx.listener(move |s, _, w, cx| {
                            s.next_match(dir, cx);
                            s.query.read(cx).focus.clone().focus(w, cx)
                        })),
                );
            }
        }
        let mut bar = row()
            .w_full()
            .p(px(6.))
            .gap(px(2.))
            .border_b_1()
            .border_color(p.line)
            .child(search)
            .child(div().mx(px(4.)).w(px(1.)).h(px(18.)).bg(p.line));
        for (expand, title) in [
            (true, tr(cx, "Expand all", "全部展开")),
            (false, tr(cx, "Collapse all", "全部收起")),
        ] {
            let size = if w.viewport_size().width < px(360.) {
                28.
            } else {
                34.
            };
            let (a, b) = if expand {
                (Glyph::Up, Glyph::Down)
            } else {
                (Glyph::Down, Glyph::Up)
            };
            bar = bar.child(
                button(title, title, cx)
                    .size(px(size))
                    .p_0()
                    .flex_none()
                    .child(
                        column()
                            .items_center()
                            .child(icon(a, 11., p.secondary))
                            .child(icon(b, 11., p.secondary)),
                    )
                    .on_click(cx.listener(move |s, _, _, cx| s.expand_all(expand, cx))),
            );
        }
        bar
    }
}
impl Render for JsonViewer {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let narrow = w.viewport_size().width <= px(420.);
        if let Some((s, target)) = &self.flight {
            let y = s.get(w, cx);
            self.scroll.set_offset(point(px(0.), px(-y)));
            if (y - target).abs() < 0.05 {
                self.flight = None
            }
        }
        let rows = self.rows.sync(self.visible.clone(), |r| *r, w, cx);
        let mut list = column()
            .id("json-tree")
            .debug_selector(|| "JSON tree".into())
            .relative()
            .role(Role::Tree)
            .aria_label(tr(cx, "JSON tree", "JSON 树"))
            .track_focus(&self.focus)
            .tab_index(0)
            .max_h(px(self.max_height))
            .p(px(6.))
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .on_scroll(cx.listener(|s, _, _, cx| {
                s.flight = None;
                cx.notify()
            }))
            .on_key_down(cx.listener(Self::key));
        let mut y = 0.;
        for (r, a, live) in &rows {
            if *live && *r == self.selected {
                let marker = self.marker.get_or_insert_with(|| Slide::new(y));
                marker.duration(y, 0.24, cx);
                list = list.child(
                    div()
                        .absolute()
                        .left(px(6.))
                        .right(px(6.))
                        .top(px(7. + marker.get(w, cx)))
                        .h(px(ROW - 2.))
                        .rounded(surface_radius(cx))
                        .bg(p.stage),
                );
                break;
            }
            y += ROW * a.height;
        }
        // Keep full scroll geometry while shaping text only near the viewport.
        // Fractional row heights still participate while a branch is unfolding.
        // The layout engine clamps scrolling after prepaint. Clamp here too so
        // folding a large branch cannot leave an empty virtual window for a frame.
        let extent = 12. + rows.iter().map(|(_, a, _)| ROW * a.height).sum::<f32>();
        let offset =
            (-f32::from(self.scroll.offset().y)).clamp(0., (extent - self.max_height).max(0.));
        self.scroll.set_offset(point(px(0.), px(-offset)));
        let first = offset - 3. * ROW;
        let end = offset + self.max_height + 3. * ROW;
        let mut top = 0.;
        let mut before = 0.;
        let mut after = 0.;
        for (r, a, live) in rows {
            let height = ROW * a.height;
            if top + height < first {
                before += height;
            } else if top > end {
                after += height;
            } else {
                if before > 0. {
                    list = list.child(div().h(px(before)).flex_none());
                    before = 0.;
                }
                list = list.child(self.row_view(r, a, live, narrow, w, cx));
            }
            top += height;
        }
        if before + after > 0. {
            list = list.child(div().h(px(before + after)).flex_none());
        }
        let sc = self.scroll.clone();
        let monitor = canvas(
            |_, _, _| (),
            move |b, _, _, cx| {
                if let Some(h) = cx.try_global::<EditorScrollRegion>() {
                    (h.0)(
                        b,
                        -sc.offset().y > px(0.1),
                        -sc.offset().y + px(0.1) < sc.max_offset().y,
                    )
                }
            },
        )
        .absolute()
        .inset_0();
        let id = self.selected.id();
        let n = &self.nodes[id];
        let page = matches!(self.selected, Row::More(_));
        let detail = if page {
            tr(cx, "page", "分页").to_owned()
        } else if n.kind.branch() {
            format!("{}, {}", n.kind.name(), Self::count_label(n, cx))
        } else {
            n.kind.name().into()
        };
        let footer = row()
            .w_full()
            .gap(px(10.))
            .pl(px(14.))
            .pr(px(6.))
            .py(px(6.))
            .border_t_1()
            .border_color(p.line)
            .child(self.code(&n.path, p.secondary, None, 12.).flex_1())
            .when(!narrow, |r| {
                r.child(label(detail, 12., p.faint).flex_none())
            })
            .when(!page, |r| r.child(self.copy_key(id, true, true, cx)));
        panel(cx)
            .id("json-viewer")
            .w(fit(w, 522.))
            .child(self.toolbar(w, cx))
            .child(column().relative().child(list).child(monitor))
            .child(footer)
            .capture_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                if s.query.read(cx).focus.is_focused(w) && e.keystroke.key == "down" {
                    s.focus.focus(w, cx);
                    s.reveal(false, cx);
                    w.prevent_default();
                    cx.stop_propagation();
                } else if s.query.read(cx).focus.is_focused(w)
                    && e.keystroke.key == "enter"
                    && e.keystroke.modifiers.shift
                {
                    s.next_match(-1, cx);
                    w.prevent_default();
                    cx.stop_propagation();
                }
            }))
    }
}
