//! An expandable search box with grouped suggestions and keyboard selection.
use crate::{
    foundation::Theme,
    primitives::{
        control::UiLanguage,
        editor::{EditEvent, Editor, InputMode},
        family::{Paint, PhaseClock, Slide, Tone, face, standard_ease},
        glyph::{Glyph, icon},
    },
};
use gpui::{prelude::*, *};
#[derive(Clone, Debug)]
pub struct SearchItem {
    pub id: SharedString,
    pub title: SharedString,
    pub detail: SharedString,
    pub group: SharedString,
    pub keywords: SharedString,
    pub icon: Glyph,
}
impl SearchItem {
    pub fn new(id: impl Into<SharedString>, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            detail: "".into(),
            group: "".into(),
            keywords: "".into(),
            icon: Glyph::FileText,
        }
    }
    pub fn description(
        mut self,
        group: impl Into<SharedString>,
        detail: impl Into<SharedString>,
        icon: Glyph,
    ) -> Self {
        self.group = group.into();
        self.detail = detail.into();
        self.icon = icon;
        self
    }
    pub fn keywords(mut self, v: impl Into<SharedString>) -> Self {
        self.keywords = v.into();
        self
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SearchEvent {
    Expanded(bool),
    Selected(SharedString),
}
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum SearchAnchor {
    Start,
    Center,
    #[default]
    End,
}
pub struct ExpandingSearch {
    id: ElementId,
    label: SharedString,
    items: Vec<SearchItem>,
    recent: Vec<SharedString>,
    results: Vec<usize>,
    active: Option<usize>,
    open: bool,
    width: f32,
    limit: usize,
    anchor: SearchAnchor,
    editor: Entity<Editor>,
    trigger: FocusHandle,
    panel_bounds: std::rc::Rc<std::cell::Cell<Option<Bounds<Pixels>>>>,
    slide: Slide,
    high: Slide,
    icon_fade: PhaseClock,
    icon_start: Vec<f32>,
    _sub: Subscription,
}
impl EventEmitter<SearchEvent> for ExpandingSearch {}
impl ExpandingSearch {
    pub fn new(
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        items: Vec<SearchItem>,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let label = label.into();
        let editor = cx.new(|cx| {
            let mut e = Editor::new(label.clone(), InputMode::Combobox, w, cx);
            e.set_placeholder(label.clone());
            e
        });
        let sub = cx.subscribe_in(&editor, w, |s, _, ev: &EditEvent, w, cx| {
            if !s.open {
                return;
            }
            match ev {
                EditEvent::Changed => s.filter(cx),
                EditEvent::Escape => s.collapse(true, w, cx),
                EditEvent::Submitted => {
                    if let Some(i) = s.active {
                        s.choose(i, w, cx)
                    }
                }
                EditEvent::Navigate(down) => {
                    let n = s.results.len();
                    if n > 0 {
                        let i = s.active.map_or(if *down { 0 } else { n - 1 }, |i| {
                            if *down { (i + 1) % n } else { (i + n - 1) % n }
                        });
                        s.activate(i, cx)
                    }
                }
                EditEvent::Blurred if s.editor.read(cx).value().trim().is_empty() => {
                    s.collapse(false, w, cx)
                }
                _ => {}
            }
        });
        Self {
            id: id.into(),
            label,
            items,
            recent: vec![],
            results: vec![],
            active: None,
            open: false,
            width: 360.,
            limit: 6,
            anchor: SearchAnchor::End,
            editor,
            trigger: cx.focus_handle().tab_stop(true),
            panel_bounds: Default::default(),
            slide: Slide::new(0.),
            high: Slide::snappy(0.),
            icon_fade: PhaseClock::default(),
            icon_start: Vec::new(),
            _sub: sub,
        }
    }
    pub fn with_suggestions(mut self, ids: Vec<SharedString>) -> Self {
        self.recent = ids;
        self
    }
    pub fn with_layout(mut self, anchor: SearchAnchor, width: f32, max_results: usize) -> Self {
        self.anchor = anchor;
        self.width = width.max(42.);
        self.limit = max_results.max(1);
        self
    }
    fn filter(&mut self, cx: &mut Context<Self>) {
        let query = self.editor.read(cx).value().trim().to_lowercase();
        self.results = if query.is_empty() {
            self.recent
                .iter()
                .filter_map(|id| self.items.iter().position(|v| v.id == *id))
                .take(self.limit)
                .collect()
        } else {
            let mut ranked: Vec<_> = self
                .items
                .iter()
                .enumerate()
                .filter_map(|(index, item)| {
                    let title = item.title.to_lowercase();
                    let rank = if title.starts_with(&query) {
                        0
                    } else if title.contains(&query) {
                        1
                    } else if item.keywords.to_lowercase().contains(&query) {
                        2
                    } else {
                        return None;
                    };
                    Some((rank, index))
                })
                .collect();
            ranked.sort();
            let mut results: Vec<_> = ranked
                .into_iter()
                .take(self.limit)
                .map(|(_, i)| i)
                .collect();
            // Preserve the catalog's group order even if a document prefix scores higher.
            results.sort_by_key(|i| {
                self.items
                    .iter()
                    .position(|item| item.group == self.items[*i].group)
            });
            results
        };
        self.active = (!query.is_empty() && !self.results.is_empty()).then_some(0);
        self.icon_fade = PhaseClock::default();
        self.icon_start.clear();
        self.high.reset(0.);
        cx.notify()
    }
    const ICON_FADE_SECONDS: f32 = 0.16;

    fn icon_emphasis(&self, i: usize, progress: f32) -> f32 {
        let target = f32::from(self.active == Some(i));
        let start = self.icon_start.get(i).copied().unwrap_or(target);
        start + (target - start) * progress
    }

    fn activate(&mut self, i: usize, cx: &mut Context<Self>) {
        if self.active == Some(i) {
            return;
        }
        // Snapshot the visible colors so rapid pointer and keyboard retargets stay continuous.
        let progress = standard_ease(self.icon_fade.age(cx) / Self::ICON_FADE_SECONDS);
        self.icon_start = (0..self.results.len())
            .map(|row| self.icon_emphasis(row, progress))
            .collect();
        self.icon_fade.restart(cx);
        self.active = Some(i);
        self.high.set(self.row_y(i, cx), cx);
        cx.notify()
    }
    fn expand(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.open {
            return;
        }
        self.open = true;
        self.slide.set(1., cx);
        self.editor.update(cx, |s, cx| {
            s.set_value("", w, cx);
            s.focus(w, cx)
        });
        w.request_virtual_keyboard();
        self.filter(cx);
        cx.emit(SearchEvent::Expanded(true));
        cx.notify()
    }
    fn collapse(&mut self, restore: bool, w: &mut Window, cx: &mut Context<Self>) {
        if !self.open {
            return;
        }
        self.open = false;
        self.slide.set(0., cx);
        if restore {
            self.trigger.focus(w, cx)
        }
        cx.emit(SearchEvent::Expanded(false));
        cx.notify()
    }
    fn choose(&mut self, i: usize, w: &mut Window, cx: &mut Context<Self>) {
        if self.open
            && let Some(&idx) = self.results.get(i)
        {
            cx.emit(SearchEvent::Selected(self.items[idx].id.clone()));
            self.collapse(true, w, cx)
        }
    }
    fn header(&self, i: usize, cx: &App) -> bool {
        i == 0
            || (!self.editor.read(cx).value().trim().is_empty()
                && self.items[self.results[i]].group != self.items[self.results[i - 1]].group)
    }
    fn row_y(&self, i: usize, cx: &App) -> f32 {
        8. + i as f32 * 48. + (0..=i).filter(|n| self.header(*n, cx)).count() as f32 * 30.
    }
}
impl Render for ExpandingSearch {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let r = Theme::global(cx).tokens.radius.clone();
        let l = UiLanguage::current(cx);
        let progress = self.slide.get(w, cx).clamp(0., 1.);
        let query = self.editor.read(cx).value().to_owned();
        let width = 42.
            + (self
                .width
                .min(f32::from(w.viewport_size().width) - 32.)
                .max(42.)
                - 42.)
                * progress;
        let shell = div()
            .relative()
            .w(px(width))
            .max_w_full()
            .h(px(42.))
            .flex()
            .items_center()
            .gap(px(6.))
            .px(px(10.))
            .rounded(r.md)
            .bg(p.panel)
            .border_1()
            .border_color(if self.open { p.faint } else { p.line })
            .overflow_hidden();
        let trigger = face(
            "open",
            self.label.clone(),
            40.,
            r.md.into(),
            Tone::Clear,
            cx,
        )
        .aria_expanded(self.open)
        .track_focus(&self.trigger)
        .w(px(40.))
        .absolute()
        .left_0()
        .top_0()
        .child(icon(Glyph::Search, 18., p.ink))
        .on_click(cx.listener(|s, _, w, cx| s.expand(w, cx)));
        let mut shell = if self.open {
            shell
                .child(icon(Glyph::Search, 17., p.secondary))
                .child(div().flex_1().min_w_0().h_full().child(self.editor.clone()))
        } else {
            shell.child(trigger)
        };
        if self.open && !query.is_empty() {
            shell = shell.child(
                face(
                    "clear",
                    l.text("Clear search", "清除搜索"),
                    26.,
                    r.md.into(),
                    Tone::Clear,
                    cx,
                )
                .tab_stop(false)
                .w(px(26.))
                .child(icon(Glyph::Close, 15., p.secondary))
                .on_mouse_down(MouseButton::Left, |_, w, _| w.prevent_default())
                .on_click(cx.listener(|s, _, w, cx| {
                    s.editor.update(cx, |ed, cx| {
                        ed.set_value("", w, cx);
                        ed.focus(w, cx)
                    })
                })),
            )
        }
        let mut root = div()
            .id(self.id.clone())
            .relative()
            .w_full()
            .max_w(px(self.width))
            .flex()
            .flex_col()
            .when(self.anchor == SearchAnchor::End, |s| s.items_end())
            .when(self.anchor == SearchAnchor::Center, |s| s.items_center())
            .child(shell);
        if self.open || progress > 0.03 {
            let mut panel = div()
                .id("results")
                .role(Role::ListBox)
                .aria_label(self.label.clone())
                .relative()
                .w(px(width))
                .max_w_full()
                .px(px(6.))
                .py(px(8.))
                .rounded(r.md)
                .bg(p.panel)
                .border_1()
                .border_color(p.line)
                .shadow_md()
                .overflow_hidden()
                .opacity(progress);
            if let Some(i) = self.active {
                let y = self.row_y(i, cx);
                self.high.set(y, cx);
                panel = panel.child(
                    div()
                        .absolute()
                        .left(px(6.))
                        .right(px(6.))
                        .top(px(self.high.get(w, cx)))
                        .h(px(48.))
                        .bg(p.hover)
                        .rounded(r.md),
                )
            }
            let empty = query.trim().is_empty();
            let icon_progress = standard_ease(
                self.icon_fade.time(Self::ICON_FADE_SECONDS, w, cx) / Self::ICON_FADE_SECONDS,
            );
            for (i, &idx) in self.results.iter().enumerate() {
                let item = &self.items[idx];
                if self.header(i, cx) {
                    panel = panel.child(
                        div()
                            .h(px(30.))
                            .flex()
                            .items_center()
                            .px(px(8.))
                            .text_size(px(11.))
                            .text_color(p.secondary)
                            .child(if empty {
                                l.text("Recent", "最近").into()
                            } else {
                                item.group.clone()
                            }),
                    )
                }
                let mut title = div().text_size(px(14.)).text_color(p.ink).truncate();
                let term = query.trim();
                if let Some((start, end)) = highlight_range(&item.title, term) {
                    let mut value = StyledText::new(item.title.clone());
                    value = value.with_highlights([(
                        start..end,
                        HighlightStyle {
                            font_weight: Some(FontWeight::BOLD),
                            ..Default::default()
                        },
                    )]);
                    title = title.child(value)
                } else {
                    title = title.child(item.title.clone())
                }
                panel = panel.child(
                    div()
                        .id(("result", i))
                        .debug_selector({
                            let id = item.id.clone();
                            move || id.to_string()
                        })
                        .role(Role::ListBoxOption)
                        .aria_selected(self.active == Some(i))
                        .when(self.active == Some(i), |row| row.aria_active_descendant())
                        .aria_label(item.title.clone())
                        .h(px(48.))
                        .w_full()
                        .flex()
                        .items_center()
                        .gap(px(12.))
                        .px(px(12.))
                        .cursor_pointer()
                        .child(
                            div()
                                .w(px(18.))
                                .flex_none()
                                .flex()
                                .justify_center()
                                .items_center()
                                .child(icon(
                                    item.icon,
                                    16.,
                                    p.faint
                                        .blend(p.ink.opacity(self.icon_emphasis(i, icon_progress))),
                                )),
                        )
                        .child(
                            div().min_w_0().flex_1().child(title).child(
                                div()
                                    .truncate()
                                    .text_size(px(12.))
                                    .text_color(p.secondary)
                                    .child(item.detail.clone()),
                            ),
                        )
                        .on_hover(cx.listener(move |s, on, _, cx| {
                            if *on && s.open && s.active != Some(i) {
                                s.activate(i, cx)
                            }
                        }))
                        .on_mouse_down(MouseButton::Left, |_, w, _| w.prevent_default())
                        .on_click(cx.listener(move |s, _, w, cx| s.choose(i, w, cx))),
                )
            }
            if self.results.is_empty() {
                panel = panel.child(
                    div()
                        .p(px(12.))
                        .text_size(px(13.))
                        .child(l.text("No results", "没有结果"))
                        .child(div().text_color(p.secondary).child(l.text(
                            "Try a shorter word or check the spelling.",
                            "试试更短的词，或检查拼写。",
                        ))),
                )
            }
            let n = self.results.len();
            panel = panel.child(
                div()
                    .id("count")
                    .role(Role::Status)
                    .px(px(8.))
                    .text_size(px(11.))
                    .text_color(p.secondary)
                    .child(if l == UiLanguage::Chinese {
                        format!("{n} 项{}", if empty { "最近使用" } else { "结果" })
                    } else {
                        format!(
                            "{n} {}",
                            if empty {
                                "recent items"
                            } else if n == 1 {
                                "result"
                            } else {
                                "results"
                            }
                        )
                    }),
            );
            root = root.child(
                div()
                    .absolute()
                    .top(px(51.))
                    .when(self.anchor == SearchAnchor::End, |s| s.right_0())
                    .when(self.anchor == SearchAnchor::Start, |s| s.left_0())
                    .on_children_prepainted({
                        let bounds = self.panel_bounds.clone();
                        move |b, _, _| bounds.set(b.first().copied())
                    })
                    .child(panel),
            )
        }
        root.on_mouse_down_out(cx.listener(|s, ev: &MouseDownEvent, w, cx| {
            if s.open
                && !s
                    .panel_bounds
                    .get()
                    .is_some_and(|b| b.contains(&ev.position))
            {
                s.collapse(false, w, cx);
            }
        }))
    }
}
fn highlight_range(text: &str, needle: &str) -> Option<(usize, usize)> {
    if needle.is_empty() {
        return None;
    }
    let needle = needle.to_lowercase();
    for (i, _) in text.char_indices() {
        let mut match_text = String::new();
        for (k, c) in text[i..].char_indices() {
            match_text.extend(c.to_lowercase());
            if match_text == needle {
                return Some((i, i + k + c.len_utf8()));
            }
            if !needle.starts_with(&match_text) {
                break;
            }
        }
    }
    None
}
