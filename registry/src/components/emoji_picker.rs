//! Search a virtualized emoji grid with categories, skin tones and keyboard selection.
use crate::primitives::{
    editor::{EditEvent, Editor, EditorScrollRegion, InputMode},
    emoji_catalog::CATALOG,
    family::{Paint, Slide, disabled_motion},
    form_chrome::*,
    glyph::{Glyph, icon},
};
use gpui::{prelude::*, *};
use std::{cell::Cell, collections::HashMap, ops::Range, rc::Rc};

const GROUPS: [(&str, &str, Glyph); 9] = [
    ("Smileys and emotion", "表情与情感", Glyph::SmilePlus),
    ("People and body", "人物与身体", Glyph::User),
    ("Animals and nature", "动物与自然", Glyph::Sun),
    ("Food and drink", "食物与饮品", Glyph::Heart),
    ("Travel and places", "旅行与地点", Glyph::Rocket),
    ("Activities", "活动", Glyph::Celebrate),
    ("Objects", "物品", Glyph::Bulb),
    ("Symbols", "符号", Glyph::HelpCircle),
    ("Flags", "旗帜", Glyph::Globe),
];
const POPULAR: [&str; 18] = [
    "👍", "❤️", "😂", "🎉", "🙏", "🔥", "😊", "👀", "✅", "🙌", "💯", "😍", "🤔", "👏", "🚀", "😅",
    "🥳", "👋",
];
const TONES: [(&str, &str, u32); 6] = [
    ("Default tone", "默认肤色", 0xedba4c),
    ("Light", "浅色", 0xf2d7bf),
    ("Medium-light", "较浅", 0xdec3a6),
    ("Medium", "中等", 0xbd9275),
    ("Medium-dark", "较深", 0x936848),
    ("Dark", "深色", 0x654631),
];
#[derive(Clone)]
pub struct Symbol {
    pub text: SharedString,
    pub name: SharedString,
    pub category: SharedString,
    pub glyph: Option<Glyph>,
}
#[derive(Clone, Debug)]
pub struct SymbolPicked(pub SharedString);
#[derive(Clone, Default)]
pub struct SymbolPreferences {
    pub recent: Vec<SharedString>,
    pub tone: usize,
}
enum Line {
    Title(SharedString),
    Cells { first: usize, len: usize },
}
#[derive(Default)]
struct Grid {
    rows: Vec<Line>,
    order: Vec<usize>,
    positions: Vec<usize>,
    starts: Vec<(SharedString, usize)>,
}
impl Grid {
    fn group(&mut self, title: SharedString, ids: &[usize], cols: usize) {
        if ids.is_empty() {
            return;
        }
        self.starts.push((title.clone(), self.rows.len()));
        self.rows.push(Line::Title(title));
        for chunk in ids.chunks(cols) {
            let row = self.rows.len();
            self.rows.push(Line::Cells {
                first: self.order.len(),
                len: chunk.len(),
            });
            self.order.extend(chunk);
            self.positions.extend(std::iter::repeat_n(row, chunk.len()));
        }
    }
}
pub struct SymbolPicker {
    items: Vec<Symbol>,
    keywords: Vec<String>,
    groups: Vec<(SharedString, Glyph)>,
    recent: Vec<usize>,
    popular: Vec<usize>,
    tones: HashMap<SharedString, [SharedString; 6]>,
    tone: usize,
    tone_open: bool,
    pub open: bool,
    query: Entity<Editor>,
    trigger: FocusHandle,
    grid_focus: FocusHandle,
    scroll: UniformListScrollHandle,
    reveal: FormReveal,
    anchor: Rc<Cell<Bounds<Pixels>>>,
    grid: Rc<Grid>,
    dirty: bool,
    columns: usize,
    active: usize,
    hover: Option<usize>,
    grid_navigation: bool,
    category_marker: Slide,
    indicator_at: Option<f32>,
    pressed: Option<usize>,
    press: Slide,
    preview: Option<usize>,
    preview_scale: Slide,
    tone_slide: Slide,
    _sub: Subscription,
}
impl EventEmitter<SymbolPicked> for SymbolPicker {}
impl EventEmitter<SymbolPreferences> for SymbolPicker {}
impl SymbolPicker {
    pub fn new(items: Vec<Symbol>, w: &mut Window, cx: &mut Context<Self>) -> Self {
        let query = cx.new(|cx| {
            let mut e = Editor::new(
                tr(cx, "Search emoji", "搜索表情"),
                InputMode::Combobox,
                w,
                cx,
            );
            e.set_placeholder(tr(cx, "Search emoji", "搜索表情"));
            e.font_size = 14.;
            e
        });
        let sub = cx.subscribe_in(
            &query,
            w,
            |s: &mut Self, _, ev: &EditEvent, w, cx| match ev {
                EditEvent::Changed => {
                    s.dirty = true;
                    s.active = 0;
                    s.hover = None;
                    s.grid_navigation = false;
                    s.scroll.scroll_to_item(0, ScrollStrategy::Top);
                    cx.notify()
                }
                EditEvent::Escape => s.close(w, cx),
                EditEvent::Submitted => {
                    if let Some(i) = s.grid.order.get(s.active).copied() {
                        s.pick(i, w.modifiers().shift, w, cx)
                    }
                }
                _ => {}
            },
        );
        let mut groups = Vec::new();
        for i in &items {
            if !groups.iter().any(|(n, _)| *n == i.category) {
                groups.push((i.category.clone(), Glyph::SmilePlus))
            }
        }
        let keywords = items.iter().map(|a| a.name.to_lowercase()).collect();
        Self {
            items,
            keywords,
            groups,
            recent: vec![],
            popular: vec![],
            tones: HashMap::new(),
            tone: 0,
            tone_open: false,
            open: false,
            query,
            trigger: cx.focus_handle().tab_stop(true),
            grid_focus: cx.focus_handle(),
            scroll: UniformListScrollHandle::new(),
            reveal: FormReveal::new(false).above(),
            anchor: Default::default(),
            grid: Default::default(),
            dirty: true,
            columns: 0,
            active: 0,
            hover: None,
            grid_navigation: false,
            category_marker: Slide::snappy(0.),
            indicator_at: None,
            pressed: None,
            press: Slide::snappy(0.),
            preview: None,
            preview_scale: Slide::snappy(1.),
            tone_slide: Slide::snappy(0.),
            _sub: sub,
        }
    }
    /// Full Emoji 15.0 catalog with English aliases, Chinese keywords and Unicode tone sequences.
    pub fn standard(w: &mut Window, cx: &mut Context<Self>) -> Self {
        let chinese = tr(cx, "en", "zh") == "zh";
        let mut items = Vec::new();
        let mut aliases = Vec::new();
        let mut variants = HashMap::new();
        let groups: Vec<(_, _)> = GROUPS
            .iter()
            .map(|&(en, zh, g)| (SharedString::from(if chinese { zh } else { en }), g))
            .collect();
        for line in CATALOG.lines().filter(|l| !l.is_empty()) {
            let mut f = line.split('\t');
            let group = f.next().and_then(|s| s.parse::<usize>().ok()).unwrap_or(0);
            let (Some(text), Some(en), Some(zh), Some(keys), Some(tones)) =
                (f.next(), f.next(), f.next(), f.next(), f.next())
            else {
                continue;
            };
            items.push(Symbol {
                text: text.into(),
                name: if chinese { zh } else { en }.into(),
                category: groups[group].0.clone(),
                glyph: None,
            });
            aliases.push(format!("{en} {zh} {keys}").to_lowercase());
            if tones != "-" {
                let mut t = tones.split(',');
                variants.insert(
                    text.into(),
                    std::array::from_fn(|i| {
                        if i == 0 {
                            text.into()
                        } else {
                            t.next().unwrap_or(text).into()
                        }
                    }),
                );
            }
        }
        let popular = POPULAR
            .iter()
            .filter_map(|&e| items.iter().position(|i| i.text.as_ref() == e))
            .collect();
        let mut s = Self::new(items, w, cx);
        s.keywords = aliases;
        s.groups = groups;
        s.tones = variants;
        s.popular = popular;
        s
    }
    pub fn with_tones(
        mut self,
        variants: impl IntoIterator<Item = (SharedString, [SharedString; 6])>,
    ) -> Self {
        self.tones.extend(variants);
        self
    }
    pub fn with_preferences(mut self, prefs: SymbolPreferences) -> Self {
        self.tone = prefs.tone.min(5);
        for r in prefs.recent {
            if let Some(i) = self.items.iter().position(|e| e.text == r)
                && !self.recent.contains(&i)
            {
                self.recent.push(i)
            }
            if self.recent.len() == 27 {
                break;
            }
        }
        self
    }
    fn preferences(&self) -> SymbolPreferences {
        SymbolPreferences {
            recent: self
                .recent
                .iter()
                .map(|i| self.items[*i].text.clone())
                .collect(),
            tone: self.tone,
        }
    }
    fn token(&self, i: usize) -> SharedString {
        self.tones
            .get(&self.items[i].text)
            .map_or_else(|| self.items[i].text.clone(), |a| a[self.tone].clone())
    }
    fn rebuild(&mut self, cols: usize, cx: &App) {
        if self.columns == cols && !self.dirty {
            return;
        }
        self.columns = cols;
        self.dirty = false;
        let q = self.query.read(cx).value().trim().to_lowercase();
        let mut grid = Grid::default();
        if !q.is_empty() {
            let mut results = self
                .items
                .iter()
                .enumerate()
                .filter_map(|(i, item)| {
                    let name = item.name.to_lowercase();
                    let score = if item.text.as_ref() == q || name == q {
                        0
                    } else if name.starts_with(&q) {
                        1
                    } else if name.contains(&q) {
                        2
                    } else if self.keywords[i].contains(&q) {
                        3
                    } else {
                        return None;
                    };
                    Some((score, i))
                })
                .collect::<Vec<_>>();
            results.sort_unstable();
            let ids: Vec<_> = results.into_iter().map(|(_, i)| i).collect();
            grid.group(tr(cx, "Search results", "搜索结果").into(), &ids, cols);
        } else {
            if self.recent.is_empty() {
                grid.group(tr(cx, "Popular", "热门").into(), &self.popular, cols)
            } else {
                grid.group(
                    tr(cx, "Recently used", "最近使用").into(),
                    &self.recent,
                    cols,
                )
            }
            for (name, _) in &self.groups {
                let ids = self
                    .items
                    .iter()
                    .enumerate()
                    .filter_map(|(i, v)| (&v.category == name).then_some(i))
                    .collect::<Vec<_>>();
                grid.group(name.clone(), &ids, cols)
            }
        }
        self.active = self.active.min(grid.order.len().saturating_sub(1));
        self.grid = Rc::new(grid)
    }
    fn close(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        self.pressed = None;
        self.press.reset(0.);
        self.open = false;
        self.tone_open = false;
        self.trigger.focus(w, cx);
        cx.notify()
    }
    fn pick(&mut self, i: usize, many: bool, w: &mut Window, cx: &mut Context<Self>) {
        self.recent.retain(|&k| k != i);
        self.recent.insert(0, i);
        self.recent.truncate(27);
        cx.emit(SymbolPicked(self.token(i)));
        cx.emit(self.preferences());
        if many {
            self.grid_focus.focus(w, cx);
            self.grid_navigation = true
        } else {
            self.close(w, cx)
        }
        cx.notify()
    }
    fn move_to(&mut self, pos: usize) {
        self.active = pos;
        self.hover = None;
        self.grid_navigation = true;
        if let Some(row) = self.grid.positions.get(pos) {
            self.scroll.scroll_to_item(*row, ScrollStrategy::Nearest)
        }
    }
    fn key(&mut self, e: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        if self
            .query
            .update(cx, |q, cx| q.marked_text_range(w, cx).is_some())
        {
            return;
        }
        let key = e.keystroke.key.as_str();
        if self.tone_open {
            match key {
                "left" | "right" => {
                    self.tone = (self.tone + if key == "left" { 5 } else { 1 }) % 6;
                    cx.emit(self.preferences())
                }
                "enter" | "space" | "escape" => self.tone_open = false,
                _ => return,
            }
        } else {
            let n = self.grid.order.len();
            match key {
                "escape" => {
                    if self.query.read(cx).value().is_empty() {
                        self.close(w, cx)
                    } else {
                        self.query.update(cx, |e, cx| {
                            e.set_value("", w, cx);
                            e.focus(w, cx)
                        })
                    }
                }
                "enter" if n > 0 => self.pick(
                    self.grid.order[self.active],
                    e.keystroke.modifiers.shift,
                    w,
                    cx,
                ),
                "space" if self.grid_focus.is_focused(w) && n > 0 => self.pick(
                    self.grid.order[self.active],
                    e.keystroke.modifiers.shift,
                    w,
                    cx,
                ),
                "up" | "down" | "left" | "right" | "home" | "end" if n > 0 => {
                    let search = self.query.read(cx).focus.is_focused(w);
                    if search && ["left", "right"].contains(&key) {
                        return;
                    }
                    let pos = match key {
                        "down" if search => self.active,
                        "up" => self.active.saturating_sub(self.columns),
                        "down" => (self.active + self.columns).min(n - 1),
                        "left" => self.active.saturating_sub(1),
                        "right" => (self.active + 1).min(n - 1),
                        "home" => 0,
                        _ => n - 1,
                    };
                    self.move_to(pos);
                    self.grid_focus.focus(w, cx)
                }
                _ if self.grid_focus.is_focused(w)
                    && key.chars().count() == 1
                    && !e.keystroke.modifiers.control
                    && !e.keystroke.modifiers.platform =>
                {
                    let ch = key.to_owned();
                    self.query.update(cx, |q, cx| {
                        q.focus(w, cx);
                        q.replace_text_in_range(None, &ch, w, cx)
                    })
                }
                _ => return,
            }
        }
        w.prevent_default();
        cx.stop_propagation();
        cx.notify()
    }
    fn symbol(&self, i: usize, size: f32, cx: &App) -> AnyElement {
        self.items[i].glyph.map_or_else(
            || {
                div()
                    .text_size(px(size))
                    .line_height(px(size + 5.))
                    .child(self.token(i))
                    .into_any_element()
            },
            |g| icon(g, size, Paint::get(cx).ink).into_any_element(),
        )
    }
}

impl Render for SymbolPicker {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let anchor = self.anchor.clone();
        let mut out = col()
            .items_start()
            .id("symbols")
            .on_mouse_down_out(cx.listener(|s, e: &MouseDownEvent, w, cx| {
                if s.open && !s.reveal.contains(e.position) {
                    s.close(w, cx)
                }
            }))
            .child(
                glyph(
                    "open",
                    tr(cx, "Choose emoji", "选择表情"),
                    Glyph::SmilePlus,
                    cx,
                )
                .p_0()
                .size(px(36.))
                .bg(gpui::transparent_black())
                .relative()
                .track_focus(&self.trigger)
                .aria_expanded(self.open)
                .child(
                    canvas(move |b, _, _| anchor.set(b), |_, _, _, _| ())
                        .absolute()
                        .inset_0(),
                )
                .on_click(cx.listener(|s, _, w, cx| {
                    if s.open {
                        s.close(w, cx)
                    } else {
                        s.open = true;
                        s.dirty = true;
                        s.active = 0;
                        s.hover = None;
                        s.grid_navigation = false;
                        s.tone_open = false;
                        s.indicator_at = None;
                        s.scroll.scroll_to_item(0, ScrollStrategy::Top);
                        s.query.update(cx, |q, cx| {
                            q.set_value("", w, cx);
                            q.focus(w, cx)
                        });
                        cx.notify()
                    }
                })),
            );
        let progress = self.reveal.sample(self.open, 0.26, 0.14, w, cx);
        if !self.open && progress <= 0. {
            return out;
        }
        let anchor = self.anchor.get();
        self.reveal.at(anchor);
        let width = (f32::from(w.viewport_size().width) - 20.).clamp(140., 352.);
        let cols = ((width - 18.) / 36.).floor().max(3.) as usize;
        self.rebuild(cols, cx);
        let room = (f32::from(anchor.top())
            .max(f32::from(w.viewport_size().height - anchor.bottom()))
            - 16.)
            .clamp(165., 408.);
        let area = (room - 128.).max(40.);
        let cell = (width - 18.) / cols as f32;
        let top =
            ((-f32::from(self.scroll.0.borrow().base_handle.offset().y)).max(0.) / 36.) as usize;
        let selected = self
            .grid
            .starts
            .iter()
            .rposition(|(_, row)| *row <= top)
            .unwrap_or(0);
        let querying = !self.query.read(cx).value().is_empty();
        self.tone_slide
            .duration(f32::from(self.tone_open), 0.24, cx);
        let tone_width = 28. + 128. * self.tone_slide.get(w, cx).clamp(0., 1.);
        let mut tones = row()
            .flex_none()
            .justify_end()
            .overflow_hidden()
            .gap_0()
            .w(px(tone_width));
        if self.tone_open {
            for (i, (en, zh, color)) in TONES.iter().copied().enumerate() {
                tones = tones.child(
                    button(("tone", i), "", cx)
                        .aria_label(tr(cx, en, zh))
                        .debug_selector(move || en.to_string())
                        .p_0()
                        .size(px(26.))
                        .child(
                            div()
                                .size(px(17.))
                                .rounded_full()
                                .bg(rgb(color))
                                .when(i == self.tone, |s| s.border_2().border_color(p.ink)),
                        )
                        .on_click(cx.listener(move |s, _, w, cx| {
                            s.tone = i;
                            s.tone_open = false;
                            cx.emit(s.preferences());
                            s.query.read(cx).focus.clone().focus(w, cx);
                            cx.notify()
                        })),
                )
            }
        } else {
            tones = tones.child(
                button("tones", "", cx)
                    .aria_label(tr(cx, "Skin tone", "肤色"))
                    .debug_selector(|| "Skin tone".into())
                    .p_0()
                    .size(px(28.))
                    .child(
                        div()
                            .text_size(px(17.))
                            .child(["✋", "✋🏻", "✋🏼", "✋🏽", "✋🏾", "✋🏿"][self.tone]),
                    )
                    .on_click(cx.listener(|s, _, _, cx| {
                        s.tone_open = true;
                        cx.notify()
                    })),
            )
        }
        let search = row()
            .gap(px(5.))
            .h(px(36.))
            .px(px(7.))
            .child(icon(Glyph::Search, 16., p.faint))
            .child(div().flex_1().min_w_0().h_full().child(self.query.clone()))
            .child(tones);
        let mut tabs = row().gap_0().relative().w_full().h(px(36.));
        let mut sections = self.groups.clone();
        if !self.recent.is_empty() || !self.popular.is_empty() {
            sections.insert(
                0,
                (
                    if self.recent.is_empty() {
                        tr(cx, "Popular", "热门")
                    } else {
                        tr(cx, "Recently used", "最近使用")
                    }
                    .into(),
                    Glyph::Clock,
                ),
            )
        }
        let tab_width = (width - 18.) / sections.len().max(1) as f32;
        for (i, (name, g)) in sections.iter().enumerate() {
            let n = name.clone();
            let label = n.clone();
            tabs = tabs.child(
                button(("tab", i), "", cx)
                    .p_0()
                    .h_full()
                    .w(px(tab_width))
                    .aria_label(n.clone())
                    .debug_selector(move || label.to_string())
                    .aria_selected(!querying && i == selected)
                    .child(
                        div()
                            .debug_selector(move || format!("emoji-category-icon-{i}"))
                            .size(px(16.))
                            .child(icon(
                                *g,
                                16.,
                                if !querying && i == selected {
                                    p.ink
                                } else {
                                    p.faint
                                },
                            )),
                    )
                    .on_click(cx.listener(move |s, _, w, cx| {
                        s.query.update(cx, |e, cx| e.set_value("", w, cx));
                        s.dirty = true;
                        s.rebuild(s.columns, cx);
                        if let Some((_, idx)) = s.grid.starts.iter().find(|(tag, _)| *tag == n) {
                            let row = *idx;
                            s.scroll.scroll_to_item_strict(row, ScrollStrategy::Top);
                            s.active = s.grid.positions.iter().position(|&r| r > row).unwrap_or(0);
                            s.grid_focus.focus(w, cx);
                            s.grid_navigation = true;
                            s.hover = None;
                        }
                        cx.notify()
                    })),
            )
        }
        if !querying {
            let marker_width = (tab_width - 4.).clamp(2., 18.);
            let x = tab_width * selected as f32 + (tab_width - marker_width) / 2.;
            if self.indicator_at.is_none() {
                self.category_marker.reset(x)
            }
            self.indicator_at = Some(x);
            self.category_marker.duration(x, 0.30, cx);
            tabs = tabs.child(
                div()
                    .absolute()
                    .debug_selector(|| "emoji-category-current".into())
                    .left(px(self.category_marker.get(w, cx)))
                    .bottom_0()
                    .h(px(2.))
                    .w(px(marker_width))
                    .bg(p.ink),
            )
        }
        let data = self.grid.clone();
        let count = data.rows.len();
        let scroll = self.scroll.clone();
        let monitor = canvas(
            |_, _, _| (),
            move |b, _, _, cx| {
                let inner = scroll.0.borrow();
                let h = &inner.base_handle;
                if let Some(host) = cx.try_global::<EditorScrollRegion>() {
                    (host.0)(
                        b,
                        -h.offset().y > px(0.1),
                        -h.offset().y + px(0.1) < h.max_offset().y,
                    )
                }
            },
        )
        .absolute()
        .inset_0();
        let mut grid = div()
            .w_full()
            .h(px(area))
            .relative()
            .id("emoji-grid-focus")
            .role(Role::Grid)
            .aria_label(tr(cx, "Emoji grid", "表情网格"))
            .track_focus(&self.grid_focus);
        if count == 0 {
            grid = grid.child(
                col()
                    .size_full()
                    .items_center()
                    .justify_center()
                    .child(hint(tr(cx, "No matching emoji", "没有匹配表情"), cx)),
            )
        } else {
            grid = grid
                .child(
                    uniform_list(
                        "symbol-grid",
                        count,
                        cx.processor(move |s, rows: Range<usize>, w, cx| {
                            let p = Paint::get(cx);
                            rows.map(|r| match &data.rows[r] {
                                Line::Title(name) => div()
                                    .w_full()
                                    .h(px(36.))
                                    .flex()
                                    .items_center()
                                    .child(hint(name.clone(), cx))
                                    .into_any_element(),
                                Line::Cells { first, len } => {
                                    let mut line = row().h(px(36.)).gap_0();
                                    for pos in *first..(*first + *len) {
                                        let i = data.order[pos];
                                        let n = s.items[i].name.clone();
                                        let zoom = if s.pressed == Some(pos) {
                                            s.press.get(w, cx)
                                        } else {
                                            0.
                                        };
                                        let active = s.hover == Some(pos)
                                            || (s.hover.is_none()
                                                && s.grid_navigation
                                                && s.active == pos);
                                        line = line.child(
                                            div()
                                                .id(("emoji-cell", pos))
                                                .role(Role::GridCell)
                                                .aria_label(n.clone())
                                                .aria_selected(active)
                                                .debug_selector(move || n.to_string())
                                                .h(px(36.))
                                                .w(px(cell))
                                                .flex_none()
                                                .flex()
                                                .items_center()
                                                .justify_center()
                                                .rounded(crate::foundation::surface_radius(cx))
                                                .cursor_pointer()
                                                .when(active, |e| e.bg(p.hover))
                                                .active(move |e| e.bg(p.hover))
                                                .child(s.symbol(i, 22. - 2.5 * zoom, cx))
                                                .on_mouse_down(
                                                    MouseButton::Left,
                                                    cx.listener(move |s, _, _, cx| {
                                                        s.pressed = Some(pos);
                                                        s.press.reset(0.);
                                                        s.press.duration(1., 0.12, cx);
                                                        cx.notify()
                                                    }),
                                                )
                                                .on_mouse_up(
                                                    MouseButton::Left,
                                                    cx.listener(|s, _, _, cx| {
                                                        s.press.duration(0., 0.22, cx);
                                                        cx.notify()
                                                    }),
                                                )
                                                .on_hover(cx.listener(move |s, hover, _, cx| {
                                                    if *hover {
                                                        s.hover = Some(pos)
                                                    } else if s.hover == Some(pos) {
                                                        s.hover = None;
                                                        if s.pressed == Some(pos) {
                                                            s.press.duration(0., 0.22, cx);
                                                        }
                                                    }
                                                    cx.notify()
                                                }))
                                                .on_click(cx.listener(
                                                    move |s, e: &ClickEvent, w, cx| {
                                                        s.active = pos;
                                                        s.pick(i, e.modifiers().shift, w, cx)
                                                    },
                                                )),
                                        );
                                    }
                                    line.into_any_element()
                                }
                            })
                            .collect()
                        }),
                    )
                    .h_full()
                    .w_full()
                    .track_scroll(&self.scroll)
                    .on_scroll(cx.listener(|_, _, _, cx| cx.notify())),
                )
                .child(monitor)
        }
        let picked = self
            .hover
            .or_else(|| self.grid_navigation.then_some(self.active))
            .and_then(|pos| self.grid.order.get(pos).copied());
        if picked != self.preview {
            self.preview = picked;
            self.preview_scale
                .reset(if disabled_motion(cx) { 1. } else { 0.80 });
            self.preview_scale.duration(1., 0.24, cx)
        }
        let preview_size = 22. * self.preview_scale.get(w, cx);
        let footer = row()
            .w_full()
            .gap(px(10.))
            .h(px(36.))
            .px(px(3.))
            .border_t_1()
            .border_color(p.line)
            .when_some(picked, |v, i| {
                v.child(self.symbol(i, preview_size, cx))
                    .child(hint(self.items[i].name.clone(), cx).truncate())
            })
            .when(picked.is_none(), |v| {
                v.child(div().opacity(0.45).child("👋"))
                    .child(hint(tr(cx, "Pick an emoji", "选择一个表情"), cx))
            });
        let face = panel(cx)
            .id("palette")
            .role(Role::Dialog)
            .aria_label(tr(cx, "Emoji picker", "表情选择器"))
            .w(px(width))
            .p(px(8.))
            .gap_0()
            .capture_key_down(cx.listener(Self::key))
            .child(search)
            .child(tabs)
            .child(grid)
            .child(footer);
        out = out.child(self.reveal.popup(
            face,
            progress,
            self.open,
            (width, room, if disabled_motion(cx) { 0. } else { 4. }),
            w,
        ));
        out
    }
}
