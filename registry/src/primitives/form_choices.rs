//! Shared roving selection and accessible option rendering. Selection uses stable keys.
use super::{
    editor::{EditEvent, Editor, InputMode},
    family::{Paint, Slide, Tone, face},
    form_chrome::*,
    glyph::Glyph,
};
use gpui::{prelude::*, *};
use std::{collections::BTreeSet, marker::PhantomData};
#[derive(Clone)]
pub struct FormOption {
    pub key: SharedString,
    pub label: SharedString,
    pub detail: SharedString,
    pub disabled: bool,
}
impl FormOption {
    pub fn new(key: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            detail: "".into(),
            disabled: false,
        }
    }
    pub fn detail(mut self, t: impl Into<SharedString>) -> Self {
        self.detail = t.into();
        self
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Arrangement {
    Radio,
    Segments,
    Chips,
    Multi,
}
pub trait ChoiceLayout: 'static {
    const STYLE: Arrangement;
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectionChanged(pub Vec<SharedString>);
pub struct FormChoices<L: ChoiceLayout> {
    label: SharedString,
    items: Vec<FormOption>,
    selected: BTreeSet<usize>,
    active: usize,
    focus: Vec<FocusHandle>,
    trigger: FocusHandle,
    search: Entity<Editor>,
    open: bool,
    offset: Slide,
    marks: Vec<Slide>,
    popup: FormReveal,
    one_chip: bool,
    expanded: bool,
    limit: usize,
    description: SharedString,
    _sub: Subscription,
    _layout: PhantomData<L>,
}
impl<L: ChoiceLayout> EventEmitter<SelectionChanged> for FormChoices<L> {}
impl<L: ChoiceLayout> FormChoices<L> {
    pub fn new(
        label: impl Into<SharedString>,
        items: Vec<FormOption>,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let search = cx.new(|cx| {
            let mut e = Editor::new(
                tr(cx, "Filter options", "筛选选项"),
                InputMode::Combobox,
                w,
                cx,
            );
            e.set_placeholder(tr(cx, "Search options…", "搜索选项…"));
            e
        });
        let sub = cx.subscribe_in(&search, w, |s: &mut Self, _, ev: &EditEvent, w, cx| {
            match ev {
                EditEvent::Changed => s.active = s.visible(cx).first().copied().unwrap_or(0),
                EditEvent::Escape => s.close(w, cx),
                EditEvent::Submitted => {
                    let i = s.active;
                    s.pick(i, cx)
                }
                EditEvent::Navigate(d) => s.step(if *d { 1 } else { -1 }, w, cx),
                _ => {}
            }
            cx.notify()
        });
        Self {
            label: label.into(),
            focus: (0..items.len())
                .map(|_| cx.focus_handle().tab_stop(true))
                .collect(),
            trigger: cx.focus_handle().tab_stop(true),
            marks: vec![Slide::snappy(0.); items.len()],
            items,
            search,
            selected: BTreeSet::new(),
            active: 0,
            open: false,
            offset: Slide::snappy(0.),
            popup: FormReveal::new(false),
            one_chip: false,
            expanded: false,
            limit: 6,
            description: "".into(),
            _sub: sub,
            _layout: PhantomData,
        }
    }
    pub fn with_selected(mut self, keys: &[&str]) -> Self {
        for (i, item) in self.items.iter().enumerate() {
            if !item.disabled && keys.contains(&item.key.as_ref()) {
                self.selected.insert(i);
                self.active = i;
                self.marks[i].reset(1.);
                self.offset.reset(i as f32);
                if self.single() {
                    break;
                }
            }
        }
        self
    }
    pub fn with_description(mut self, t: impl Into<SharedString>) -> Self {
        self.description = t.into();
        self
    }
    pub fn selected(&self) -> Vec<SharedString> {
        self.selected
            .iter()
            .map(|i| self.items[*i].key.clone())
            .collect()
    }
    pub fn clear(&mut self, cx: &mut Context<Self>) {
        if !self.selected.is_empty() {
            self.selected.clear();
            cx.emit(SelectionChanged(vec![]));
            cx.notify()
        }
    }
    pub fn set_multiple(&mut self, multiple: bool, cx: &mut Context<Self>) {
        self.one_chip = !multiple;
        if self.one_chip && self.selected.len() > 1 {
            let first = *self.selected.first().unwrap();
            self.selected.retain(|x| *x == first);
            cx.emit(SelectionChanged(self.selected()))
        }
        cx.notify()
    }
    fn single(&self) -> bool {
        (L::STYLE == Arrangement::Chips && self.one_chip)
            || matches!(L::STYLE, Arrangement::Radio | Arrangement::Segments)
    }
    fn visible(&self, cx: &App) -> Vec<usize> {
        let q = if L::STYLE == Arrangement::Multi {
            self.search.read(cx).value().to_lowercase()
        } else {
            String::new()
        };
        self.items
            .iter()
            .enumerate()
            .filter(|(_, o)| !o.disabled && o.label.to_lowercase().contains(&q))
            .map(|(i, _)| i)
            .collect()
    }
    fn pick(&mut self, i: usize, cx: &mut Context<Self>) {
        if self.items.get(i).is_none_or(|x| x.disabled) {
            return;
        }
        if self.single() && self.selected.contains(&i) {
            return;
        }
        if self.single() {
            self.selected.clear()
        }
        if !self.selected.remove(&i) {
            self.selected.insert(i);
        }
        self.active = i;
        if self.single() {
            self.offset.duration(i as f32, 0.42, cx);
        }
        cx.emit(SelectionChanged(self.selected()));
        cx.notify()
    }
    fn step(&mut self, delta: isize, w: &mut Window, cx: &mut Context<Self>) {
        let ids = self.visible(cx);
        if ids.is_empty() {
            return;
        }
        let pos = ids.iter().position(|v| *v == self.active).unwrap_or(0);
        let i = ids[(pos as isize + delta).rem_euclid(ids.len() as isize) as usize];
        self.active = i;
        if self.single() {
            self.pick(i, cx)
        }
        if L::STYLE != Arrangement::Multi {
            self.focus[i].focus(w, cx)
        }
        cx.notify()
    }
    fn close(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        self.open = false;
        self.trigger.focus(w, cx);
        cx.notify()
    }
    fn toggle(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.open {
            self.close(w, cx)
        } else {
            self.open = true;
            self.search.update(cx, |e, cx| {
                e.set_value("", w, cx);
                e.focus(w, cx)
            });
            cx.notify()
        }
    }
    fn keys(&mut self, e: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        match e.keystroke.key.as_str() {
            "down" | "right" => self.step(1, w, cx),
            "up" | "left" => self.step(-1, w, cx),
            "escape" if self.open => self.close(w, cx),
            _ => return,
        }
        w.prevent_default();
        cx.stop_propagation()
    }
}
impl<L: ChoiceLayout> Render for FormChoices<L> {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let style = L::STYLE;
        let pos = self.offset.get(w, cx);
        let popup = self.popup.sample(self.open, 0.26, 0.12, w, cx);
        for (i, spring) in self.marks.iter_mut().enumerate() {
            spring.duration(
                self.selected.contains(&i) as u8 as f32,
                if style == Arrangement::Multi {
                    0.40
                } else {
                    0.26
                },
                cx,
            );
        }
        let width = match style {
            Arrangement::Multi => 538.,
            Arrangement::Radio => 380.,
            Arrangement::Chips => 520.,
            Arrangement::Segments => 222.,
        };
        let mut root = section(width, cx)
            .id("choices")
            .on_key_down(cx.listener(Self::keys));
        if style == Arrangement::Multi {
            root = root.on_mouse_down_out(cx.listener(|s, e: &MouseDownEvent, _, cx| {
                if s.open && !s.popup.contains(e.position) {
                    s.open = false;
                    cx.notify()
                }
            }))
        }
        if style == Arrangement::Multi || style == Arrangement::Radio {
            root = root.child(self.label.clone())
        }
        if style == Arrangement::Multi {
            let mut chips = row()
                .relative()
                .min_h(px(24.))
                .flex_wrap()
                .gap(px(4.))
                .flex_1();
            let mut last_chip_opacity = 0_f32;
            for (i, mark) in self.marks.iter().enumerate() {
                let t = mark.get(w, cx).clamp(0., 1.);
                if t <= 0. {
                    continue;
                }
                last_chip_opacity = last_chip_opacity.max(t);
                let item = &self.items[i];
                let width = (super::family::measure(&item.label, 12., w).ceil() + 16.).min(144.);
                // Reveal the measured face without shrinking its corners or its text padding.
                let chip = div()
                    .id(("selected-value", i))
                    .debug_selector({
                        let key = item.key.clone();
                        move || format!("Selected value: {key}")
                    })
                    .flex_none()
                    .flex()
                    .items_center()
                    .w(px(width))
                    .h(px(24.))
                    .px(px(8.))
                    .rounded(crate::foundation::surface_radius(cx))
                    .bg(p.hover)
                    .text_size(px(12.))
                    .line_height(px(16.))
                    .child(div().min_w_0().truncate().child(item.label.clone()));
                chips = chips.child(
                    div()
                        .flex()
                        .w(px(width * t))
                        .overflow_hidden()
                        .opacity(t)
                        .child(chip),
                )
            }
            if self.selected.is_empty() {
                chips = chips.child(
                    hint(tr(cx, "Select…", "请选择…"), cx)
                        .absolute()
                        .left_0()
                        .opacity(1. - last_chip_opacity),
                )
            }
            let display = face(
                "open",
                self.label.clone(),
                42.,
                crate::foundation::surface_radius(cx).into(),
                Tone::Clear,
                cx,
            )
            .track_focus(&self.trigger)
            .aria_expanded(self.open)
            .justify_start()
            .flex_1()
            .min_w_0()
            .h_auto()
            .min_h(px(42.))
            .py(px(4.))
            .child(chips)
            .on_click(cx.listener(|s, _, w, cx| s.toggle(w, cx)));
            root = root.child(
                row()
                    .gap(px(2.))
                    .px(px(9.))
                    .min_h(px(44.))
                    .relative()
                    .child(self.popup.anchor())
                    .rounded(crate::foundation::surface_radius(cx))
                    .bg(p.panel)
                    .border_1()
                    .border_color(p.line)
                    .child(display)
                    .when(!self.selected.is_empty(), |d| {
                        d.child(
                            glyph(
                                "clear",
                                tr(cx, "Clear selections", "清除选择"),
                                Glyph::Close,
                                cx,
                            )
                            .size(px(22.))
                            .on_click(cx.listener(|s, _, _, cx| s.clear(cx))),
                        )
                    })
                    .child(
                        glyph(
                            "chevron",
                            tr(cx, "Show options", "展开选项"),
                            Glyph::Down,
                            cx,
                        )
                        .size(px(22.))
                        .tab_stop(false)
                        .on_click(cx.listener(|s, _, w, cx| s.toggle(w, cx))),
                    ),
            );
            if !self.open && popup <= 0. {
                return root.when(!self.description.is_empty(), |r| {
                    r.child(hint(self.description.clone(), cx))
                });
            }
        }
        let mut options = match style {
            Arrangement::Radio => col().relative().gap(px(4.)),
            Arrangement::Multi => panel(cx)
                .gap(px(2.))
                .child(div().h(px(36.)).p(px(6.)).child(self.search.clone())),
            Arrangement::Chips => row().flex_wrap().gap(px(6.)),
            Arrangement::Segments => row()
                .relative()
                .gap(px(2.))
                .p(px(4.))
                .rounded(crate::foundation::surface_radius(cx))
                .bg(p.tray),
        };
        if style == Arrangement::Segments && !self.items.is_empty() {
            let n = self.items.len() as f32;
            options = options.child(
                div().absolute().inset(px(4.)).child(
                    div()
                        .absolute()
                        .debug_selector(|| "Segment selection indicator".into())
                        .left(relative(pos / n))
                        .w(relative(1. / n))
                        .h_full()
                        .rounded(crate::foundation::surface_radius(cx))
                        .bg(p.panel)
                        .shadow_sm(),
                ),
            )
        }
        if style == Arrangement::Radio && !self.selected.is_empty() {
            options = options.child(
                div()
                    .absolute()
                    .left_0()
                    .right_0()
                    .top(px(64. * pos))
                    .h(px(60.))
                    .rounded(crate::foundation::surface_radius(cx))
                    .bg(p.hover)
                    .border_1()
                    .border_color(p.faint),
            );
        }
        let q = self.search.read(cx).value().to_lowercase();
        for (i, o) in self.items.iter().enumerate() {
            if (style == Arrangement::Multi && !o.label.to_lowercase().contains(&q))
                || (style == Arrangement::Chips && !self.expanded && i >= self.limit)
            {
                continue;
            }
            let on = self.selected.contains(&i);
            let val = self.marks[i].get(w, cx).clamp(0., 1.);
            let mut e = face(
                ("option", i),
                o.label.clone(),
                if style == Arrangement::Radio {
                    60.
                } else {
                    36.
                },
                crate::foundation::surface_radius(cx).into(),
                Tone::Clear,
                cx,
            )
            .track_focus(&self.focus[i])
            .tab_stop(!o.disabled && (!self.single() || i == self.active))
            .aria_toggled(on.into())
            .text_color(if on { p.ink } else { p.secondary });
            match style {
                Arrangement::Radio => {
                    e = e
                        .role(Role::RadioButton)
                        .border_1()
                        .border_color(p.line.opacity(1. - val))
                        .rounded(crate::foundation::surface_radius(cx))
                        .justify_start()
                        .px(px(12.))
                        .gap(px(10.))
                        .hover(move |s| s.bg(p.hover.opacity(0.4)))
                        .child(
                            div()
                                .size(px(18.))
                                .rounded_full()
                                .border_1()
                                .border_color(if on { p.bright } else { p.faint })
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(
                                    div()
                                        .size(px(10. * (0.4 + 0.6 * val)))
                                        .bg(p.bright.opacity(val))
                                        .rounded_full(),
                                ),
                        )
                        .child(
                            col()
                                .gap(px(1.))
                                .child(o.label.clone())
                                .child(hint(o.detail.clone(), cx)),
                        )
                }
                Arrangement::Segments => e = e.flex_1().min_w_0().child(o.label.clone()),
                Arrangement::Chips => {
                    e = e
                        .px(px(12.))
                        .border_1()
                        .border_color(if on { p.bright } else { p.line })
                        .bg(p.panel.blend(p.bright.opacity(val)))
                        .text_color(if on { p.inverse } else { p.secondary })
                        .gap_0()
                        .child(
                            div()
                                .w(px(17. * val))
                                .overflow_hidden()
                                .child(check_stroke(val, 0., 13., p.inverse)),
                        )
                        .child(o.label.clone())
                }
                Arrangement::Multi => {
                    e = e
                        .role(Role::ListBoxOption)
                        .justify_between()
                        .px(px(8.))
                        .bg(if i == self.active { p.hover } else { p.panel })
                        .hover(move |s| s.bg(p.hover.opacity(0.4)))
                        .child(o.label.clone())
                        .child(check_stroke(val, 0., 16., p.ink.opacity(val)))
                }
            };
            options = options.child(if o.disabled {
                e.opacity(0.45).cursor_default()
            } else {
                e.on_click(cx.listener(move |s, _, _, cx| s.pick(i, cx)))
            })
        }
        if style == Arrangement::Chips && self.items.len() > self.limit {
            let label = if self.expanded {
                tr(cx, "Show less", "收起").to_string()
            } else {
                format!(
                    "+{} {}",
                    self.items.len() - self.limit,
                    tr(cx, "more", "更多")
                )
            };
            options = options.child(button("more", label, cx).on_click(cx.listener(
                |s, _, _, cx| {
                    s.expanded = !s.expanded;
                    cx.notify()
                },
            )))
        }
        let options = if style == Arrangement::Multi {
            self.popup
                .popup(options, popup, self.open, (0., 200., 6.), w)
        } else {
            options
        };
        root.child(options).when(!self.description.is_empty(), |r| {
            r.child(hint(self.description.clone(), cx))
        })
    }
}
