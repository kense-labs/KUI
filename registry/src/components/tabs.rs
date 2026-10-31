//! Peer views with a sliding selection and directional panel transitions.
use crate::{
    foundation::Theme,
    primitives::{
        control::UiLanguage,
        family::{Paint, PhaseClock, Slide, ease, measure},
        glyph::Glyph,
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};

type Content = Box<dyn Fn(&mut Window, &mut App) -> AnyElement>;
pub struct TabPage {
    value: SharedString,
    label: SharedString,
    disabled: bool,
    content: Content,
}
impl TabPage {
    /// Capture an Entity here to preserve a panel's edits and selection between visits.
    pub fn new<E: IntoElement + 'static>(
        value: impl Into<SharedString>,
        label: impl Into<SharedString>,
        render: impl Fn(&mut Window, &mut App) -> E + 'static,
    ) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            disabled: false,
            content: Box::new(move |w, cx| render(w, cx).into_any_element()),
        }
    }
    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabChanged(pub SharedString);
pub struct Tabs {
    id: ElementId,
    name: SharedString,
    pages: Vec<TabPage>,
    selected: Option<usize>,
    previous: Option<usize>,
    reveal_selection: bool,
    focus: FocusHandle,
    left: Slide,
    width: Slide,
    height: Slide,
    measured: Rc<Cell<f32>>,
    motion: PhaseClock,
    scroll: ScrollHandle,
    bar_width: Rc<Cell<f32>>,
}
impl EventEmitter<TabChanged> for Tabs {}
impl Tabs {
    pub fn new(
        id: impl Into<ElementId>,
        name: impl Into<SharedString>,
        pages: Vec<TabPage>,
        cx: &App,
    ) -> Self {
        let selected = pages.iter().position(|p| !p.disabled);
        Self {
            id: id.into(),
            name: name.into(),
            selected,
            pages,
            previous: None,
            reveal_selection: true,
            focus: cx.focus_handle().tab_stop(true),
            left: Slide::snappy(0.),
            width: Slide::snappy(88.),
            height: Slide::new(0.),
            measured: Default::default(),
            motion: Default::default(),
            scroll: Default::default(),
            bar_width: Default::default(),
        }
    }
    pub fn value(&self) -> Option<&str> {
        self.selected.map(|i| self.pages[i].value.as_ref())
    }
    pub fn select(&mut self, key: &str, cx: &mut Context<Self>) {
        if let Some(i) = self.pages.iter().position(|p| p.value.as_ref() == key) {
            self.choose(i, cx)
        }
    }
    fn choose(&mut self, i: usize, cx: &mut Context<Self>) {
        if self.selected == Some(i) || self.pages[i].disabled {
            return;
        }
        self.previous = self.selected.replace(i);
        self.reveal_selection = true;
        self.height.reset(self.measured.get());
        self.motion.restart(cx);
        cx.emit(TabChanged(self.pages[i].value.clone()));
        cx.notify()
    }
    fn navigate(&mut self, k: &str, cx: &mut Context<Self>) {
        let mut enabled = self
            .pages
            .iter()
            .enumerate()
            .filter(|(_, p)| !p.disabled)
            .map(|(i, _)| i);
        let current = self.selected.unwrap_or(0);
        let next = match k {
            "home" => enabled.next(),
            "end" => enabled.next_back(),
            "left" => enabled
                .clone()
                .rfind(|i| *i < current)
                .or_else(|| enabled.next_back()),
            _ => enabled
                .clone()
                .find(|i| *i > current)
                .or_else(|| enabled.next()),
        };
        if let Some(i) = next {
            self.choose(i, cx)
        }
    }
}
impl Render for Tabs {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let r = Theme::global(cx).tokens.radius.clone();
        let Some(idx) = self.selected else {
            return div().id(self.id.clone());
        };
        let widths = self
            .pages
            .iter()
            .map(|page| (24. + measure(&page.label, 14., w)).ceil().max(88.))
            .collect::<Vec<_>>();
        let offset = widths[..idx].iter().sum::<f32>() + 4. * idx as f32;
        let total =
            widths.iter().sum::<f32>() + 4. * self.pages.len().saturating_sub(1) as f32 + 8.;
        let age = self.motion.time(0.6, w, cx);
        let f = ease((age / 0.24).min(1.));
        if self.previous.is_none() {
            self.left.reset(offset);
            self.width.reset(widths[idx])
        } else {
            self.left.duration(offset, 0.28, cx);
            self.width.duration(widths[idx], 0.28, cx)
        }
        let mut items = div()
            .relative()
            .flex()
            .p(px(4.))
            .gap(px(4.))
            .w(px(total))
            .child(
                div()
                    .absolute()
                    .top(px(4.))
                    .left(px(4. + self.left.get(w, cx)))
                    .w(px(self.width.get(w, cx)))
                    .h(px(36.))
                    .rounded(r.sm)
                    .border_1()
                    .border_color(p.line)
                    .bg(p.panel)
                    .shadow_sm(),
            );
        for (i, page) in self.pages.iter().enumerate() {
            let selected = i == idx;
            let mut tab = div()
                .id(("tab", i))
                .debug_selector(|| page.label.to_string())
                .role(Role::Tab)
                .aria_label(page.label.clone())
                .aria_selected(selected)
                .relative()
                .flex_none()
                .w(px(widths[i]))
                .h(px(36.))
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(14.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(if selected { p.ink } else { p.secondary })
                .child(page.label.clone());
            if selected {
                tab = tab
                    .track_focus(&self.focus)
                    .tab_stop(true)
                    .rounded(r.sm)
                    .focus_visible(move |s| s.border_1().border_color(p.ink))
            }
            if page.disabled {
                tab = tab.opacity(0.5)
            } else {
                tab = tab
                    .cursor_pointer()
                    .hover(move |e| e.text_color(p.ink))
                    .on_click(cx.listener(move |v, _, w, cx| {
                        v.choose(i, cx);
                        v.focus.focus(w, cx)
                    }))
            }
            items = items.child(tab)
        }
        let saved = self.bar_width.clone();
        let viewport = self.scroll.clone();
        let active_left = offset + 4.;
        let right_edge = active_left + widths[idx];
        let changed = std::mem::take(&mut self.reveal_selection);
        let strip = div()
            .id("tabs-viewport")
            .max_w_full()
            .overflow_x_scroll()
            .track_scroll(&self.scroll)
            .child(items)
            .child(
                canvas(
                    move |b, w, _| {
                        let resized = (saved.replace(f32::from(b.size.width))
                            - f32::from(b.size.width))
                        .abs()
                            > 0.1;
                        if resized {
                            w.request_animation_frame()
                        }
                        if changed || resized {
                            let left = -f32::from(viewport.offset().x);
                            let width = f32::from(b.size.width);
                            let x = if active_left < left + 30. {
                                (active_left - 34.).max(0.)
                            } else if right_edge > left + width - 30. {
                                (right_edge + 34. - width).min((total - width).max(0.))
                            } else {
                                left
                            };
                            if (x - left).abs() > 0.1 {
                                viewport.set_offset(point(px(-x), px(0.)));
                                w.request_animation_frame()
                            }
                        }
                    },
                    |_, _, _, _| (),
                )
                .absolute()
                .inset_0(),
            );
        let mut shell = div()
            .id("tab-strip")
            .role(Role::TabList)
            .aria_label(self.name.clone())
            .relative()
            .max_w_full()
            .rounded(r.md)
            .overflow_hidden()
            .border_1()
            .border_color(p.line)
            .bg(p.stage)
            .child(strip)
            .on_key_down(cx.listener(|v, e: &KeyDownEvent, _, cx| {
                if matches!(e.keystroke.key.as_str(), "left" | "right" | "home" | "end") {
                    v.navigate(&e.keystroke.key, cx);
                    cx.stop_propagation()
                }
            }));
        let room = self.bar_width.get();
        if room > 0. && total > room + 1. {
            let x = -f32::from(self.scroll.offset().x);
            for (back, show) in [(true, x > 1.), (false, x + room + 1. < total)] {
                if !show {
                    continue;
                }
                let label = if back {
                    UiLanguage::current(cx).text("Scroll left", "向左滚动")
                } else {
                    UiLanguage::current(cx).text("Scroll right", "向右滚动")
                };
                let scroll = self.scroll.clone();
                shell = shell.child(
                    div()
                        .id(("edge", usize::from(back)))
                        .aria_label(label)
                        .role(Role::Button)
                        .absolute()
                        .top_0()
                        .bottom_0()
                        .when(back, |e| e.left_0())
                        .when(!back, |e| e.right_0())
                        .w(px(33.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(p.stage)
                        .cursor_pointer()
                        .on_click(cx.listener(move |_, _, _, cx| {
                            let dx = if back { room * 0.75 } else { -room * 0.75 };
                            scroll.set_offset(point(
                                px((f32::from(scroll.offset().x) + dx).clamp(-(total - room), 0.)),
                                px(0.),
                            ));
                            cx.notify()
                        }))
                        .child(crate::primitives::glyph::transformed(
                            Glyph::Down,
                            14.,
                            p.ink,
                            if back {
                                std::f32::consts::FRAC_PI_2
                            } else {
                                -std::f32::consts::FRAC_PI_2
                            },
                            1.,
                            (0., 0.),
                            1.,
                        )),
                )
            }
        }
        let direction = if self.previous.is_some_and(|old| old > idx) {
            -1.
        } else {
            1.
        };
        let mut panel = div().relative().w_full().mt_4().flex().flex_col();
        let observed = self.measured.clone();
        let content = (self.pages[idx].content)(w, cx);
        let saved = observed.get();
        if saved > 0. && age < 0.6 {
            self.height.duration(saved, 0.3, cx);
            panel = panel.h(px(self.height.get(w, cx))).overflow_hidden()
        }
        if age < 0.1
            && let Some(old) = self.previous
        {
            let leave = ease(age / 0.1);
            panel = panel.child(
                div()
                    .absolute()
                    .top_0()
                    .left(px(-6. * direction * leave))
                    .w_full()
                    .opacity(1. - leave)
                    .child((self.pages[old].content)(w, cx)),
            )
        }
        let live = div()
            .id(("panel", idx))
            .role(Role::TabPanel)
            .aria_label(self.pages[idx].label.clone())
            .relative()
            .left(px(direction * 8. * (1. - f)))
            .w_full()
            .flex_none()
            .opacity(f)
            .child(content);
        panel = panel.child(
            div()
                .w_full()
                .flex_none()
                .child(live)
                .on_children_prepainted(move |b, w, _| {
                    if let Some(b) = b.first()
                        && observed.replace(f32::from(b.size.height)) != f32::from(b.size.height)
                    {
                        w.request_animation_frame()
                    }
                }),
        );
        div()
            .id(self.id.clone())
            .w_full()
            .flex()
            .flex_col()
            .items_center()
            .child(shell)
            .child(panel)
    }
}
