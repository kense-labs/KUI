//! Single-choice values, keyboard navigation and searchable listboxes.
use crate::{
    foundation::Theme,
    primitives::{
        control::UiLanguage,
        editor::{EditEvent, Editor, InputMode},
        family::{Paint, PhaseClock, Slide, ease},
        flyout::TriggerBounds,
        glyph::{Glyph, icon},
    },
};
use gpui::{prelude::*, *};
use std::marker::PhantomData;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickerOption {
    pub value: SharedString,
    pub label: SharedString,
    pub disabled: bool,
    pub keywords: Vec<SharedString>,
}
impl PickerOption {
    pub fn new(value: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            disabled: false,
            keywords: vec![],
        }
    }
    pub fn with_keywords(
        mut self,
        words: impl IntoIterator<Item = impl Into<SharedString>>,
    ) -> Self {
        self.keywords = words.into_iter().map(Into::into).collect();
        self
    }
    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }
    fn matches(&self, q: &str) -> bool {
        self.label.to_lowercase().contains(q)
            || self.value.to_lowercase().contains(q)
            || self.keywords.iter().any(|s| s.to_lowercase().contains(q))
    }
}
pub trait PickerStyle: 'static {
    const SEARCH: bool;
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionChanged(pub Option<SharedString>);
pub struct Picker<S: PickerStyle> {
    id: ElementId,
    label: SharedString,
    options: Vec<PickerOption>,
    selected: Option<usize>,
    active: Option<usize>,
    open: bool,
    disabled: bool,
    placeholder: SharedString,
    description: SharedString,
    focus: FocusHandle,
    editor: Option<Entity<Editor>>,
    _listener: Option<Subscription>,
    query: String,
    visible: Vec<usize>,
    anchor: TriggerBounds,
    scroll: ScrollHandle,
    presence: Slide,
    height: Slide,
    swap: PhaseClock,
    old: SharedString,
    direction: f32,
    marker: PhantomData<S>,
}
impl<S: PickerStyle> Focusable for Picker<S> {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}
impl<S: PickerStyle> EventEmitter<SelectionChanged> for Picker<S> {}
impl<S: PickerStyle> Picker<S> {
    pub fn new(
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        options: Vec<PickerOption>,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let label = label.into();
        let lang = UiLanguage::current(cx);
        let placeholder: SharedString = if S::SEARCH {
            lang.text("Search or select…", "搜索或选择…")
        } else {
            lang.text("Select an option", "选择一项")
        }
        .into();
        let editor = S::SEARCH.then(|| {
            cx.new(|cx| {
                let mut e = Editor::new(label.clone(), InputMode::Combobox, w, cx);
                e.set_placeholder(placeholder.clone());
                e
            })
        });
        let sub = editor.as_ref().map(|e| {
            cx.subscribe_in(
                e,
                w,
                |v: &mut Self, editor, ev: &EditEvent, w, cx| match ev {
                    EditEvent::Focused => v.show(w, cx),
                    EditEvent::Blurred => v.close(w, cx),
                    EditEvent::Changed if v.open => {
                        v.query = editor.read(cx).value().trim().to_lowercase();
                        v.filter();
                        cx.notify()
                    }
                    EditEvent::Changed
                        if !editor.read(cx).value().is_empty()
                            && editor.read(cx).focus.is_focused(w) =>
                    {
                        v.show(w, cx);
                        v.query = editor.read(cx).value().trim().to_lowercase();
                        v.filter();
                        cx.notify()
                    }
                    EditEvent::Navigate(down) => {
                        if v.open {
                            v.step(*down, cx)
                        } else {
                            v.show(w, cx)
                        }
                    }
                    EditEvent::Submitted if v.open => {
                        if let Some(i) = v.active {
                            v.choose(i, w, cx)
                        }
                    }
                    EditEvent::Escape => v.close(w, cx),
                    _ => {}
                },
            )
        });
        let focus = editor
            .as_ref()
            .map(|e| e.read(cx).focus.clone())
            .unwrap_or_else(|| cx.focus_handle().tab_stop(true));
        Self {
            visible: (0..options.len()).collect(),
            id: id.into(),
            label,
            options,
            disabled: false,
            selected: None,
            active: None,
            open: false,
            focus,
            editor,
            _listener: sub,
            query: String::new(),
            placeholder,
            description: "".into(),
            anchor: Default::default(),
            scroll: Default::default(),
            presence: Slide::snappy(0.),
            height: Slide::new(0.),
            swap: Default::default(),
            old: "".into(),
            direction: 1.,
            marker: PhantomData,
        }
    }
    pub fn with_description(mut self, s: impl Into<SharedString>) -> Self {
        self.description = s.into();
        self
    }
    pub fn with_value(mut self, key: &str) -> Self {
        self.selected = self.options.iter().position(|o| o.value.as_ref() == key);
        self
    }
    pub fn with_placeholder(mut self, s: impl Into<SharedString>) -> Self {
        self.placeholder = s.into();
        self
    }
    pub fn with_disabled(mut self, disabled: bool, cx: &mut App) -> Self {
        self.disabled = disabled;
        if let Some(e) = &self.editor {
            e.update(cx, |ed, _| ed.disabled = disabled)
        }
        self
    }
    pub fn value(&self) -> Option<&str> {
        self.selected.map(|i| self.options[i].value.as_ref())
    }
    pub fn is_open(&self) -> bool {
        self.open
    }
    /// Reflect a host selection without echoing an input event. None clears the field.
    pub fn set_value(&mut self, key: Option<&str>, cx: &mut Context<Self>) {
        let selected = match key {
            Some(key) => {
                let Some(i) = self.options.iter().position(|o| o.value.as_ref() == key) else {
                    return;
                };
                Some(i)
            }
            None => None,
        };
        if selected != self.selected {
            self.old = self.caption();
            self.direction = if selected < self.selected { -1. } else { 1. };
            self.selected = selected;
            self.swap.restart(cx);
            cx.notify()
        }
    }
    fn caption(&self) -> SharedString {
        self.selected
            .map(|i| self.options[i].label.clone())
            .unwrap_or_else(|| self.placeholder.clone())
    }
    fn filter(&mut self) {
        self.visible = self
            .options
            .iter()
            .enumerate()
            .filter(|(_, o)| o.matches(&self.query))
            .map(|(i, _)| i)
            .collect();
        self.active = None;
        self.scroll.set_offset(point(px(0.), px(0.)))
    }
    fn show(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.open || self.disabled {
            return;
        }
        self.open = true;
        self.query.clear();
        self.filter();
        if !S::SEARCH {
            self.active = self
                .selected
                .filter(|i| !self.options[*i].disabled)
                .or_else(|| {
                    self.visible
                        .iter()
                        .copied()
                        .find(|i| !self.options[*i].disabled)
                })
        }
        self.focus.focus(w, cx);
        self.presence.duration(1., 0.24, cx);
        self.height.reset(self.visible.len().max(1) as f32 * 36.);
        cx.notify()
    }
    fn close(&mut self, w: &Window, cx: &mut Context<Self>) {
        if !self.open {
            return;
        }
        self.open = false;
        self.presence.duration(0., 0.12, cx);
        self.query.clear();
        if let Some(e) = &self.editor {
            e.update(cx, |e, cx| e.set_value("", w, cx))
        }
        cx.notify()
    }
    fn choose(&mut self, i: usize, w: &mut Window, cx: &mut Context<Self>) {
        if self.disabled || self.options[i].disabled {
            return;
        }
        let prev = self.selected;
        self.old = self.caption();
        self.direction = if prev.is_some_and(|old| old > i) {
            -1.
        } else {
            1.
        };
        self.selected = Some(i);
        self.close(w, cx);
        if prev != Some(i) {
            self.swap.restart(cx);
            cx.emit(SelectionChanged(Some(self.options[i].value.clone())))
        }
        cx.notify()
    }
    fn highlight(&mut self, i: usize, cx: &mut Context<Self>) {
        self.active = Some(i);
        if let Some(row) = self.visible.iter().position(|j| *j == i) {
            self.scroll.scroll_to_item(row)
        }
        cx.notify()
    }
    fn step(&mut self, down: bool, cx: &mut Context<Self>) {
        let enabled: Vec<_> = self
            .visible
            .iter()
            .copied()
            .filter(|i| !self.options[*i].disabled)
            .collect();
        if enabled.is_empty() {
            return;
        }
        let n = enabled.len();
        let pos = self
            .active
            .and_then(|a| enabled.iter().position(|i| *i == a));
        let i = if down {
            pos.map_or(0, |i| (i + 1) % n)
        } else {
            pos.map_or(n - 1, |i| (i + n - 1) % n)
        };
        self.highlight(enabled[i], cx)
    }
    fn key(&mut self, e: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        let k = e.keystroke.key.as_str();
        match k {
            "down" | "up" if !self.open => self.show(w, cx),
            "down" | "up" => self.step(k == "down", cx),
            "escape" => self.close(w, cx),
            "tab" => {
                self.close(w, cx);
                return;
            }
            "home" | "end" if self.open => {
                let mut it = self
                    .visible
                    .iter()
                    .copied()
                    .filter(|i| !self.options[*i].disabled);
                if let Some(i) = if k == "home" {
                    it.next()
                } else {
                    it.next_back()
                } {
                    self.highlight(i, cx)
                }
            }
            s if s.chars().count() == 1 => {
                let n = self.options.len();
                let start = self.active.or(self.selected).map_or(0, |i| i + 1);
                if let Some(i) = (0..n).map(|j| (start + j) % n).find(|i| {
                    !self.options[*i].disabled
                        && self.options[*i]
                            .label
                            .to_lowercase()
                            .starts_with(&s.to_lowercase())
                }) {
                    if self.open {
                        self.highlight(i, cx)
                    } else {
                        self.choose(i, w, cx)
                    }
                }
            }
            _ => return,
        }
        w.prevent_default();
        cx.stop_propagation()
    }
}
impl<S: PickerStyle> Render for Picker<S> {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let radius = Theme::global(cx).tokens.radius.md;
        let caption = self.caption();
        let anchor = self.anchor.clone();
        let a = self.presence.get(w, cx).clamp(0., 1.);
        let mut field = div()
            .id(self.id.clone())
            .relative()
            .w_full()
            .h(px(44.))
            .px_3()
            .flex()
            .items_center()
            .gap_2()
            .bg(p.panel)
            .rounded(radius)
            .border_1()
            .border_color(if self.focus.is_focused(w) {
                p.focus
            } else {
                p.line
            })
            .when(!self.disabled, |e| {
                e.hover(move |s| s.border_color(p.faint))
            })
            .when(self.disabled, |e| e.opacity(0.5))
            .aria_label(self.label.clone())
            .aria_expanded(self.open);
        if let Some(editor) = &self.editor {
            editor.update(cx, |ed, _| {
                ed.set_placeholder(caption.clone());
                ed.expanded = Some(self.open);
                ed.committed_label =
                    (!self.open && self.selected.is_some()).then(|| caption.clone());
                ed.description = self.description.clone();
            });
            field = field
                .child(icon(Glyph::Search, 16., p.secondary))
                .child(div().flex_1().min_w_0().h_full().child(editor.clone()))
                .on_mouse_down(MouseButton::Left, cx.listener(|v, _, w, cx| v.show(w, cx)));
            if self.selected.is_some() && !self.disabled {
                let label = UiLanguage::current(cx).text("Clear selection", "清除选择");
                field = field.child(
                    div()
                        .id("clear")
                        .aria_label(label)
                        .role(Role::Button)
                        .tab_index(0)
                        .size(px(22.))
                        .rounded(radius)
                        .flex()
                        .justify_center()
                        .items_center()
                        .cursor_pointer()
                        .hover(move |e| e.bg(p.hover))
                        .on_mouse_down(MouseButton::Left, |_, w, cx| {
                            w.prevent_default();
                            cx.stop_propagation()
                        })
                        .on_click(cx.listener(|v, _, w, cx| {
                            v.selected = None;
                            v.show(w, cx);
                            if let Some(ed) = &v.editor {
                                ed.update(cx, |ed, cx| ed.set_value("", w, cx))
                            }
                            v.focus.focus(w, cx);
                            cx.emit(SelectionChanged(None));
                            cx.notify();
                            cx.stop_propagation()
                        }))
                        .child(icon(Glyph::Close, 13., p.secondary)),
                )
            }
        } else {
            let f = ease(self.swap.time(0.24, w, cx) / 0.24);
            let d = self.direction;
            let mut val = div()
                .relative()
                .flex_1()
                .min_w_0()
                .overflow_hidden()
                .h(px(22.))
                .text_size(px(14.))
                .line_height(px(22.))
                .text_color(if self.selected.is_some() {
                    p.ink
                } else {
                    p.secondary
                });
            if f < 1. {
                val = val.child(
                    div()
                        .absolute()
                        .top(px(-4. * f * d))
                        .w_full()
                        .truncate()
                        .opacity(1. - f)
                        .child(self.old.clone()),
                )
            }
            field = field
                .debug_selector(|| self.label.to_string())
                .role(Role::ComboBox)
                .aria_value(caption.clone())
                .aria_description(self.description.clone())
                .track_focus(&self.focus)
                .tab_stop(!self.disabled)
                .cursor_pointer()
                .on_click(cx.listener(|v, event, w, cx| {
                    if v.open {
                        if matches!(event, ClickEvent::Keyboard(_))
                            && let Some(i) = v.active
                        {
                            v.choose(i, w, cx)
                        } else {
                            v.close(w, cx)
                        }
                    } else {
                        v.show(w, cx)
                    }
                }))
                .on_key_down(cx.listener(Self::key))
                .child(
                    val.child(
                        div()
                            .relative()
                            .top(px(5. * (1. - f) * d))
                            .truncate()
                            .opacity(f)
                            .child(caption),
                    ),
                );
        }
        field = field
            .child(crate::primitives::glyph::transformed(
                Glyph::Down,
                16.,
                p.secondary,
                a * std::f32::consts::PI,
                1.,
                (0., 0.),
                1.,
            ))
            .child(anchor.measure());
        let mut root = div()
            .relative()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(7.))
            .text_size(px(14.))
            .text_color(p.ink)
            .child(
                div()
                    .id("picker-label")
                    .font_weight(FontWeight::MEDIUM)
                    .on_click(cx.listener(|v, _, w, cx| v.show(w, cx)))
                    .child(self.label.clone()),
            )
            .child(field);
        if !self.description.is_empty() {
            root = root.child(
                div()
                    .text_size(px(12.))
                    .text_color(p.secondary)
                    .child(self.description.clone()),
            )
        }
        if !self.open && a < 0.002 {
            if let Some(host) = cx.try_global::<crate::primitives::editor::EditorScrollRegion>() {
                (host.0)(Bounds::default(), false, false)
            }
            return root;
        }
        let size = w.viewport_size();
        let b = anchor.get();
        let limit = if S::SEARCH {
            (f32::from(size.height) * 0.4).min(300.)
        } else {
            320.
        };
        let room = f32::from((size.height - b.bottom()).max(b.top())) - 26.;
        let max_h = limit.min(room).max(36.);
        let desired = (self.visible.len().max(1) as f32 * 36.).min(max_h);
        self.height.duration(desired, 0.24, cx);
        let h = self.height.get(w, cx).min(max_h).max(1.);
        let mut list = div()
            .id("picker-list")
            .role(Role::ListBox)
            .aria_label(self.label.clone())
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .h(px(h));
        if self.visible.is_empty() {
            list = list.child(
                div()
                    .id("no-matches")
                    .px_3()
                    .py_2()
                    .role(Role::Status)
                    .text_color(p.secondary)
                    .child(UiLanguage::current(cx).text("No matches found", "没有匹配项")),
            )
        }
        for &i in &self.visible {
            let item = &self.options[i];
            let mut row = div()
                .id(("choice", i))
                .debug_selector(|| item.label.to_string())
                .role(Role::ListBoxOption)
                .aria_label(item.label.clone())
                .aria_selected(self.selected == Some(i))
                .h(px(36.))
                .px_3()
                .rounded(Theme::global(cx).tokens.radius.md)
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .text_size(px(14.))
                .when(self.active == Some(i), |e| {
                    e.bg(p.hover).aria_active_descendant()
                })
                .child(div().min_w_0().truncate().child(item.label.clone()))
                .when(self.selected == Some(i), |e| {
                    e.child(icon(Glyph::Check, 15., p.ink))
                });
            if item.disabled {
                row = row.opacity(0.45)
            } else if self.open {
                row = row
                    .on_mouse_down(MouseButton::Left, |_, w, cx| {
                        w.prevent_default();
                        cx.stop_propagation()
                    })
                    .on_hover(cx.listener(move |v, hover: &bool, _, cx| {
                        if *hover && v.active != Some(i) {
                            v.active = Some(i);
                            cx.notify()
                        }
                    }))
                    .on_click(cx.listener(move |v, _, w, cx| {
                        v.choose(i, w, cx);
                        cx.stop_propagation()
                    }))
            }
            list = list.child(row)
        }
        let (pos, side) = anchor.place(size, px(h + 12.), px(if S::SEARCH { 8. } else { 6. }));
        let sign = if side == Anchor::TopLeft { -1. } else { 1. };
        let scroll = self.scroll.clone();
        let panel = div()
            .on_children_prepainted(move |bounds, _, cx| {
                if let (Some(b), Some(host)) = (
                    bounds.first(),
                    cx.try_global::<crate::primitives::editor::EditorScrollRegion>(),
                ) {
                    let y = -scroll.offset().y;
                    (host.0)(*b, y > px(0.), y < scroll.max_offset().y)
                }
            })
            .id("picker-popup")
            .occlude()
            .w(b.size.width)
            .p(px(5.))
            .rounded(radius)
            .border_1()
            .border_color(p.line)
            .bg(p.panel)
            .text_color(p.ink)
            .shadow_lg()
            .opacity(a)
            .on_mouse_down_out(cx.listener(move |v, e: &MouseDownEvent, w, cx| {
                if v.open && !anchor.get().contains(&e.position) {
                    v.close(w, cx)
                }
            }))
            .child(list);
        root.child(
            deferred(
                anchored()
                    .anchor(side)
                    .position(pos + point(px(0.), px(sign * 6. * (1. - a))))
                    .snap_to_window_with_margin(px(10.))
                    .child(panel),
            )
            .with_priority(3),
        )
    }
}
