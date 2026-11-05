//! A compact selection surface that morphs into a searchable option list.
use crate::primitives::{
    control::UiLanguage,
    editor::{EditEvent, Editor, InputMode},
    family::{Paint, PhaseClock, Slide, ease, measure},
    glyph::{Glyph, icon},
};
use gpui::{prelude::*, *};

#[derive(Clone, Debug)]
pub struct MorphOption {
    pub value: SharedString,
    pub label: SharedString,
    pub detail: SharedString,
    pub group: SharedString,
    pub disabled: bool,
    pub keywords: Vec<SharedString>,
}
impl MorphOption {
    pub fn new(value: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            detail: "".into(),
            group: "".into(),
            disabled: false,
            keywords: Vec::new(),
        }
    }
    pub fn with_detail(mut self, detail: impl Into<SharedString>) -> Self {
        self.detail = detail.into();
        self
    }
    pub fn with_group(mut self, group: impl Into<SharedString>) -> Self {
        self.group = group.into();
        self
    }
    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
    pub fn with_keywords(mut self, keywords: Vec<SharedString>) -> Self {
        self.keywords = keywords;
        self
    }
    fn matches(&self, query: &str) -> bool {
        self.label.to_lowercase().contains(query)
            || self.detail.to_lowercase().contains(query)
            || self.group.to_lowercase().contains(query)
            || self
                .keywords
                .iter()
                .any(|word| word.to_lowercase().contains(query))
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MorphSelectionChanged(pub SharedString);
/// One choice from a local, optionally grouped and searchable list.
pub struct MorphSelect {
    label: SharedString,
    placeholder: SharedString,
    options: Vec<MorphOption>,
    visible: Vec<usize>,
    selected: Option<usize>,
    active: Option<usize>,
    open: bool,
    disabled: bool,
    searchable: bool,
    panel_width: f32,
    focus: FocusHandle,
    editor: Entity<Editor>,
    _subscription: Subscription,
    query: String,
    scroll: ScrollHandle,
    size: Slide,
    width: Slide,
    highlight: Slide,
    old: SharedString,
    direction: f32,
    swap: PhaseClock,
    typeahead: String,
    type_clock: PhaseClock,
}
impl EventEmitter<MorphSelectionChanged> for MorphSelect {}
impl MorphSelect {
    pub fn new(
        label: impl Into<SharedString>,
        options: Vec<MorphOption>,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let label = label.into();
        let lang = UiLanguage::current(cx);
        let editor = cx.new(|cx| {
            let mut editor = Editor::new(label.clone(), InputMode::Combobox, w, cx);
            editor.set_placeholder(lang.text("Search options", "搜索选项"));
            editor
        });
        let subscription = cx.subscribe_in(
            &editor,
            w,
            |s: &mut Self, editor, event: &EditEvent, w, cx| match event {
                EditEvent::Changed if s.open => {
                    s.query = editor.read(cx).value().trim().to_lowercase();
                    s.filter(cx);
                }
                EditEvent::Navigate(down) if s.open => s.step(if *down { 1 } else { -1 }, cx),
                EditEvent::Submitted if s.open => {
                    if let Some(index) = s.active {
                        s.choose(index, w, cx)
                    }
                }
                EditEvent::Escape if s.open => {
                    s.close(cx);
                    s.focus.focus(w, cx);
                }
                _ => {}
            },
        );
        let searchable = options.len() > 8;
        let visible = (0..options.len()).collect();
        Self {
            label,
            placeholder: lang.text("Select", "选择一项").into(),
            options,
            visible,
            selected: None,
            active: None,
            open: false,
            disabled: false,
            searchable,
            panel_width: 264.,
            focus: cx.focus_handle().tab_stop(true),
            editor,
            _subscription: subscription,
            query: String::new(),
            scroll: Default::default(),
            size: Slide::snappy(0.),
            width: Slide::snappy(140.),
            highlight: Slide::snappy(0.),
            old: "".into(),
            direction: 1.,
            swap: Default::default(),
            typeahead: String::new(),
            type_clock: Default::default(),
        }
    }
    pub fn with_placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }
    pub fn with_panel_width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.panel_width = width.clamp(180., 360.);
        }
        self
    }
    pub fn with_value(mut self, value: &str) -> Self {
        self.selected = self.options.iter().position(|o| o.value.as_ref() == value);
        self
    }
    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
    pub fn with_searchable(mut self, searchable: bool) -> Self {
        self.searchable = searchable;
        self
    }
    pub fn value(&self) -> Option<&str> {
        self.selected.map(|i| self.options[i].value.as_ref())
    }
    pub fn set_value(&mut self, value: Option<&str>, cx: &mut Context<Self>) {
        self.selected = value.and_then(|v| self.options.iter().position(|o| o.value.as_ref() == v));
        cx.notify();
    }
    fn caption(&self) -> SharedString {
        self.selected
            .map(|i| self.options[i].label.clone())
            .unwrap_or_else(|| self.placeholder.clone())
    }
    fn offsets(&self) -> (Vec<f32>, f32) {
        let mut result = Vec::with_capacity(self.visible.len());
        let mut y = 0.;
        let mut group: Option<&str> = None;
        for &index in &self.visible {
            let next = self.options[index].group.as_ref();
            if !next.is_empty() && group != Some(next) {
                if group.is_some() {
                    y += 4.;
                }
                y += 28.;
            }
            group = Some(next);
            result.push(y);
            y += 36.;
            if result.len() < self.visible.len() {
                y += 2.;
            }
        }
        (result, y.max(36.))
    }
    fn set_active(&mut self, index: Option<usize>, cx: &mut Context<Self>) {
        self.active = index;
        if let Some(i) = index
            && let Some(position) = self.visible.iter().position(|&v| v == i)
        {
            let (offsets, _) = self.offsets();
            self.highlight.duration(offsets[position], 0.28, cx);
            let visible_start = -f32::from(self.scroll.offset().y);
            let visible_height = self.offsets().1.min(240.);
            let target = offsets[position];
            let next = if target < visible_start {
                target
            } else if target + 36. > visible_start + visible_height {
                target + 36. - visible_height
            } else {
                visible_start
            };
            self.scroll.set_offset(point(px(0.), px(-next.max(0.))));
        }
        cx.notify();
    }
    fn filter(&mut self, cx: &mut Context<Self>) {
        self.visible = self
            .options
            .iter()
            .enumerate()
            .filter_map(|(i, o)| o.matches(&self.query).then_some(i))
            .collect();
        let active = self
            .selected
            .filter(|i| self.visible.contains(i) && !self.options[*i].disabled)
            .or_else(|| {
                self.visible
                    .iter()
                    .copied()
                    .find(|&i| !self.options[i].disabled)
            });
        self.scroll.set_offset(point(px(0.), px(0.)));
        self.set_active(active, cx);
    }
    fn show(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        self.open = true;
        self.query.clear();
        self.filter(cx);
        let initial = self
            .active
            .and_then(|i| self.visible.iter().position(|&v| v == i))
            .map(|position| self.offsets().0[position])
            .unwrap_or(0.);
        self.highlight.reset(initial);
        self.size.duration(1., 0.4, cx);
        if self.searchable {
            self.editor.update(cx, |e, cx| {
                e.expanded = Some(true);
                e.set_value("", w, cx);
            });
            let focus = self.editor.read(cx).focus.clone();
            focus.focus(w, cx)
        } else {
            self.focus.focus(w, cx)
        }
        cx.notify();
    }
    fn close(&mut self, cx: &mut Context<Self>) {
        self.open = false;
        if self.searchable {
            self.editor.update(cx, |e, _| e.expanded = Some(false));
        }
        self.size.duration(0., 0.34, cx);
        self.typeahead.clear();
        cx.notify();
    }
    fn choose(&mut self, i: usize, w: &mut Window, cx: &mut Context<Self>) {
        if self.disabled || self.options.get(i).is_none_or(|o| o.disabled) {
            return;
        }
        if self.selected != Some(i) {
            self.old = self.caption();
            self.direction = if self.selected.is_some_and(|old| i < old) {
                -1.
            } else {
                1.
            };
            self.swap.restart(cx);
            self.selected = Some(i);
            cx.emit(MorphSelectionChanged(self.options[i].value.clone()));
        }
        self.close(cx);
        self.focus.focus(w, cx);
    }
    fn step(&mut self, delta: isize, cx: &mut Context<Self>) {
        let enabled: Vec<_> = self
            .visible
            .iter()
            .copied()
            .filter(|&i| !self.options[i].disabled)
            .collect();
        if enabled.is_empty() {
            return;
        }
        let pos = self
            .active
            .and_then(|i| enabled.iter().position(|&j| j == i))
            .unwrap_or(0);
        let next = (pos as isize + delta).rem_euclid(enabled.len() as isize) as usize;
        self.set_active(Some(enabled[next]), cx);
    }
    fn key(&mut self, e: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        let k = e.keystroke.key.as_str();
        if self.open && self.searchable && self.editor.read(cx).focus.is_focused(w) {
            if self.editor.read(cx).is_composing() {
                return;
            }
            if k == "tab" {
                self.close(cx);
            } else if matches!(k, "pageup" | "pagedown") {
                self.step(if k == "pageup" { -8 } else { 8 }, cx);
                cx.stop_propagation();
                w.prevent_default();
            }
            return;
        }
        match k {
            "enter" | "space" => {
                if self.open {
                    if let Some(i) = self.active {
                        self.choose(i, w, cx)
                    }
                } else {
                    self.show(w, cx)
                }
            }
            "down" | "right" => {
                if self.open {
                    self.step(1, cx)
                } else {
                    self.show(w, cx)
                }
            }
            "up" | "left" => {
                if self.open {
                    self.step(-1, cx)
                } else {
                    self.show(w, cx)
                }
            }
            "pageup" | "pagedown" if self.open => {
                self.step(if k == "pageup" { -8 } else { 8 }, cx);
            }
            "home" => {
                if !self.open {
                    self.show(w, cx)
                }
                let first = self
                    .visible
                    .iter()
                    .copied()
                    .find(|&i| !self.options[i].disabled);
                self.set_active(first, cx);
            }
            "end" => {
                if !self.open {
                    self.show(w, cx)
                }
                let last = self
                    .visible
                    .iter()
                    .rev()
                    .copied()
                    .find(|&i| !self.options[i].disabled);
                self.set_active(last, cx);
            }
            "escape" if self.open => {
                self.close(cx);
                self.focus.focus(w, cx)
            }
            "tab" if self.open => {
                self.close(cx);
                return;
            }
            _ if k.chars().count() == 1
                && !e.keystroke.modifiers.platform
                && !e.keystroke.modifiers.control =>
            {
                if !self.open {
                    self.show(w, cx)
                }
                if self.searchable {
                    return;
                }
                if self.type_clock.age(cx) > 0.7 {
                    self.typeahead.clear();
                }
                self.type_clock.restart(cx);
                self.typeahead.push_str(&k.to_lowercase());
                let found = self.visible.iter().copied().find(|&i| {
                    !self.options[i].disabled
                        && (self.options[i]
                            .label
                            .to_lowercase()
                            .starts_with(&self.typeahead)
                            || self.options[i]
                                .keywords
                                .iter()
                                .any(|word| word.to_lowercase().starts_with(&self.typeahead)))
                });
                if found.is_some() {
                    self.set_active(found, cx)
                }
            }
            _ => return,
        }
        cx.notify();
        cx.stop_propagation();
        w.prevent_default();
    }
}
impl Render for MorphSelect {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let radius = crate::foundation::surface_radius(cx);
        let caption = self.caption();
        let closed = (measure(&caption, 14., w) + 74.).clamp(124., 240.);
        self.width
            .duration(if self.open { self.panel_width } else { closed }, 0.42, cx);
        let width = self.width.get(w, cx);
        let progress = self.size.get(w, cx).clamp(0., 1.);
        let (offsets, contents_h) = self.offsets();
        let list_h = contents_h.min(240.);
        let h = 44. + progress * (list_h + 12.);
        let highlight = self.highlight.get(w, cx);
        let mut list = div()
            .id("morph-options")
            .relative()
            .role(Role::ListBox)
            .aria_label(self.label.clone())
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .h(px(list_h))
            .w_full()
            .px(px(6.))
            .pt(px(6.))
            .pb(px(6.));
        let mut content = div().relative().flex().flex_col().w_full();
        if self.active.is_some() {
            content = content.child(
                div()
                    .absolute()
                    .left(px(0.))
                    .right(px(0.))
                    .top(px(highlight))
                    .h(px(36.))
                    .rounded(radius)
                    .bg(p.ink.opacity(0.065)),
            );
        }
        if self.visible.is_empty() {
            content = content.child(
                div()
                    .pt(px(12.))
                    .px(px(10.))
                    .pb(px(14.))
                    .flex()
                    .items_center()
                    .text_color(p.faint)
                    .child(UiLanguage::current(cx).text("No matches", "没有匹配项")),
            );
        }
        let mut last_group: Option<SharedString> = None;
        for (row, &i) in self.visible.iter().enumerate() {
            let option = &self.options[i];
            if !option.group.is_empty() && last_group.as_ref() != Some(&option.group) {
                content = content.child(
                    div()
                        .h(px(28.))
                        .pt(px(8.))
                        .pb(px(4.))
                        .px(px(10.))
                        .flex()
                        .items_center()
                        .text_size(px(12.))
                        .line_height(px(16.))
                        .when(last_group.is_some(), |g| g.mt(px(4.)))
                        .text_color(p.faint)
                        .child(option.group.clone()),
                );
            }
            last_group = Some(option.group.clone());
            let is_selected = self.selected == Some(i);
            let is_active = self.active == Some(i);
            let mut item = div()
                .id(("morph-option", i))
                .relative()
                .role(Role::ListBoxOption)
                .aria_label(option.label.clone())
                .aria_selected(is_selected)
                .when(is_active && !option.disabled, |item| {
                    item.aria_active_descendant()
                })
                .when(option.disabled, |item| {
                    item.aria_description(UiLanguage::current(cx).text("Disabled", "不可用"))
                })
                .h(px(36.))
                .pl(px(8.))
                .pr(px(10.))
                .flex()
                .items_center()
                .justify_between()
                .gap(px(8.))
                .text_size(px(14.))
                .text_color(if is_active || is_selected {
                    p.ink
                } else {
                    p.secondary
                })
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .font_weight(FontWeight::MEDIUM)
                        .child(option.label.clone()),
                )
                .child(
                    div()
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .when(!option.detail.is_empty(), |r| {
                            r.child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(p.faint)
                                    .child(option.detail.clone()),
                            )
                        })
                        .when(is_selected, |r| r.child(icon(Glyph::Check, 16., p.ink))),
                );
            if option.disabled {
                item = item.opacity(0.6)
            } else if self.open {
                let top = offsets[row];
                item = item
                    .cursor_pointer()
                    .on_mouse_down(MouseButton::Left, |_, w, cx| {
                        w.prevent_default();
                        cx.stop_propagation();
                    })
                    .on_mouse_move(cx.listener(move |s, _, _, cx| {
                        if s.active != Some(i) {
                            s.active = Some(i);
                            s.highlight.duration(top, 0.28, cx);
                            cx.notify()
                        }
                    }))
                    .on_click(cx.listener(move |s, _, w, cx| s.choose(i, w, cx)));
            }
            content = content.child(item);
        }
        list = list.child(content);
        let mut root = div()
            .id("morph-select")
            .relative()
            .w(px(width))
            .max_w_full()
            .h(px(h))
            .rounded(radius)
            .border_1()
            .border_color(if self.open { p.faint } else { p.line })
            .bg(p.panel)
            .when(self.open, |d| d.shadow_md())
            .overflow_hidden()
            .text_color(p.ink)
            .track_focus(&self.focus)
            .tab_index(if self.disabled { -1 } else { 0 })
            .role(if self.open && self.searchable {
                Role::Group
            } else {
                Role::ComboBox
            })
            .aria_label(self.label.clone())
            .aria_value(caption.clone())
            .aria_expanded(self.open)
            .when(self.disabled, |root| {
                root.aria_description(UiLanguage::current(cx).text("Disabled", "不可用"))
            })
            .focus_visible(move |s| s.border_color(p.focus))
            .on_key_down(cx.listener(Self::key))
            .on_mouse_down_out(cx.listener(|s, _, _, cx| {
                if s.open {
                    s.close(cx)
                }
            }));
        let t = ease(self.swap.time(0.28, w, cx) / 0.28);
        let mut label = div()
            .relative()
            .min_w_0()
            .flex_1()
            .h(px(20.))
            .overflow_hidden()
            .whitespace_nowrap()
            .font_weight(FontWeight::MEDIUM)
            .child(
                div()
                    .relative()
                    .top(px((1. - t) * 6. * self.direction))
                    .opacity(t)
                    .child(caption),
            );
        if t < 1. {
            label = label.child(
                div()
                    .absolute()
                    .left_0()
                    .top(px(-6. * t * self.direction))
                    .opacity(1. - t)
                    .child(self.old.clone()),
            );
        }
        let header = div()
            .id("morph-trigger")
            .h(px(44.))
            .w_full()
            .pl(px(14.))
            .pr(px(36.))
            .flex()
            .items_center()
            .justify_between()
            .gap(px(8.))
            .cursor_pointer()
            .child(if self.open && self.searchable {
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(icon(Glyph::Search, 14., p.faint))
                    .child(div().flex_1().min_w_0().h_full().child(self.editor.clone()))
                    .into_any_element()
            } else {
                label.into_any_element()
            })
            .child(
                div()
                    .id("morph-caret")
                    .flex_none()
                    .child(icon(
                        if self.open { Glyph::Up } else { Glyph::Down },
                        14.,
                        p.faint,
                    ))
                    .when(self.open && self.searchable, |caret| {
                        caret.on_click(cx.listener(|s, _, w, cx| {
                            s.close(cx);
                            s.focus.focus(w, cx);
                        }))
                    }),
            )
            .when(!self.open || !self.searchable, |header| {
                header.on_click(
                    cx.listener(|s, _, w, cx| if s.open { s.close(cx) } else { s.show(w, cx) }),
                )
            });
        root = root.child(header);
        if progress > 0. {
            root = root.child(
                div()
                    .opacity(progress)
                    .child(div().mx(px(12.)).h(px(1.)).bg(p.line))
                    .child(list),
            );
        }
        root.when(self.disabled, |d| d.opacity(0.5))
    }
}

#[cfg(test)]
mod tests {
    use super::{MorphOption, MorphSelect};
    use gpui::{EntityInputHandler, TestAppContext};
    #[test]
    fn matches_label_group_details_and_keywords() {
        let option = MorphOption::new("id", "Alpha")
            .with_group("Design")
            .with_detail("UTC+8")
            .with_keywords(vec!["ui".into()]);
        for query in ["alpha", "design", "utc", "ui"] {
            assert!(option.matches(query));
        }
        assert!(!option.matches("sales"));
    }
    #[test]
    fn page_keys_do_not_move_selection_during_ime_composition() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let options = (0..12)
            .map(|i| MorphOption::new(format!("id{i}"), format!("ni{i}")))
            .collect();
        let (control, w) = app.add_window_view(|w, cx| {
            MorphSelect::new("Options", options, w, cx).with_searchable(true)
        });
        w.update(|w, cx| control.update(cx, |s, cx| s.show(w, cx)));
        w.update(|w, cx| {
            let editor = control.read(cx).editor.clone();
            editor.update(cx, |e, cx| {
                e.replace_and_mark_text_in_range(None, "ni", Some(2..2), w, cx);
            })
        });
        w.update(|w, cx| {
            w.simulate_next_frame(cx);
            w.refresh();
        });
        let before = w.update(|_, cx| control.read(cx).active);
        w.simulate_keystrokes("pagedown");
        assert_eq!(w.update(|_, cx| control.read(cx).active), before);
    }
}
