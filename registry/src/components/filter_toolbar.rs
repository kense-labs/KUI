//! Add, replace or remove one filter per facet.
use crate::primitives::{
    family::{Paint, Slide},
    flyout::TriggerBounds,
    glyph::{Glyph, icon},
    nav::*,
};
use gpui::{prelude::*, *};
use std::collections::BTreeMap;
pub struct Facet {
    pub name: SharedString,
    pub values: Vec<SharedString>,
}
impl Facet {
    pub fn new(name: impl Into<SharedString>, values: Vec<SharedString>) -> Self {
        Self {
            name: name.into(),
            values,
        }
    }
}
pub struct FiltersChanged(pub BTreeMap<usize, usize>);
pub struct FilterToolbar {
    facets: Vec<Facet>,
    active: BTreeMap<usize, usize>,
    menu: bool,
    facet: Option<usize>,
    popup: Fold,
    anchor: TriggerBounds,
    launch: Option<FocusHandle>,
    menu_focus: Option<FocusHandle>,
    pane: Slide,
    chips: Vec<(Option<usize>, Slide)>,
}
impl EventEmitter<FiltersChanged> for FilterToolbar {}
impl FilterToolbar {
    pub fn new(facets: Vec<Facet>, initial: Vec<(usize, usize)>) -> Self {
        let active: BTreeMap<_, _> = initial
            .into_iter()
            .filter(|&(f, v)| facets.get(f).is_some_and(|x| v < x.values.len()))
            .collect();
        let chips = (0..facets.len())
            .map(|i| {
                (
                    active.get(&i).copied(),
                    Slide::new(if active.contains_key(&i) { 1. } else { 0. }),
                )
            })
            .collect();
        Self {
            chips,
            anchor: Default::default(),
            launch: None,
            menu_focus: None,
            facets,
            active,
            menu: false,
            facet: None,
            popup: Fold::new(false, 46.),
            pane: Slide::new(1.),
        }
    }
    pub fn set(&mut self, f: usize, v: Option<usize>, cx: &mut Context<Self>) {
        if let Some(v) = v {
            if self.facets.get(f).is_none_or(|x| v >= x.values.len()) {
                return;
            }
            self.active.insert(f, v);
        } else {
            self.active.remove(&f);
        }
        self.menu = false;
        cx.emit(FiltersChanged(self.active.clone()));
        cx.notify()
    }
}
impl Render for FilterToolbar {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let launch = self
            .launch
            .get_or_insert_with(|| cx.focus_handle().tab_stop(true))
            .clone();
        let menu_focus = self
            .menu_focus
            .get_or_insert_with(|| cx.focus_handle())
            .clone();
        let mut line = row().flex_wrap();
        for (f, (last, appear)) in self.chips.iter_mut().enumerate() {
            let live = self.active.get(&f).copied();
            if let Some(v) = live {
                *last = Some(v);
            }
            appear.duration(if live.is_some() { 1. } else { 0. }, 0.24, cx);
            let fraction = appear.get(w, cx).clamp(0., 1.);
            let Some(v) = *last else {
                continue;
            };
            if live.is_none() && fraction < 0.001 {
                continue;
            }
            let facet = &self.facets[f];
            let name = format!("{} · {}", facet.name, facet.values[v]);
            let text: SharedString = name.clone().into();
            let width = crate::primitives::family::measure(&text, 13., w) + 47.;
            line = line.child(
                div()
                    .w(px((width + 8.) * fraction))
                    .overflow_hidden()
                    .opacity(fraction)
                    .child(
                        row()
                            .w(px(width))
                            .flex_none()
                            .bg(p.stage)
                            .rounded(crate::foundation::surface_radius(cx))
                            .child(button(("edit", f), name.clone(), cx).child(name).on_click(
                                cx.listener(move |s, _, _, cx| {
                                    s.facet = Some(f);
                                    s.menu = true;
                                    cx.notify()
                                }),
                            ))
                            .child(
                                button(("remove", f), tr(cx, "Remove filter", "移除筛选"), cx)
                                    .size(px(25.))
                                    .p_0()
                                    .child(icon(Glyph::Close, 12., p.secondary))
                                    .on_click(cx.listener(move |s, _, _, cx| s.set(f, None, cx))),
                            ),
                    ),
            )
        }
        let add = tr(cx, "Add filter", "添加筛选");
        let clear = tr(cx, "Clear all", "清除全部");
        line = line.child(
            button("add", add, cx)
                .relative()
                .track_focus(&launch)
                .child(self.anchor.measure())
                .border_1()
                .border_color(p.line)
                .aria_expanded(self.menu)
                .child(icon(Glyph::Plus, 14., p.secondary))
                .child(add)
                .on_click(cx.listener(|s, _, w, cx| {
                    s.menu = !s.menu;
                    if s.menu {
                        s.menu_focus.as_ref().unwrap().focus(w, cx);
                    }
                    s.facet = None;
                    cx.notify()
                })),
        );
        if !self.active.is_empty() {
            line = line.child(
                button("clear", clear, cx)
                    .ml_2()
                    .child(clear)
                    .on_click(cx.listener(|s, _, _, cx| {
                        s.active.clear();
                        s.menu = false;
                        cx.emit(FiltersChanged(s.active.clone()));
                        cx.notify()
                    })),
            )
        }
        let mut root = panel(cx).w(fit(w, 520.)).p_3().child(line);
        self.popup.set(self.menu, cx);
        let a = self.popup.fraction(w, cx);
        if self.menu || a > 0.001 {
            let b = self.anchor.get();
            let viewport = w.viewport_size();
            let width = fit(w, 238.);
            let total = self
                .facet
                .map_or(self.facets.len(), |f| self.facets[f].values.len());
            let max_h = (viewport.height - px(20.)).max(px(40.));
            let height = px(46. + 34. * total as f32).min(max_h);
            let right = b.center().x > viewport.width / 2.;
            let up = b.top() + height > viewport.height - px(8.) && b.bottom() > height;
            let pos = point(
                if right { b.right() } else { b.left() },
                if up { b.bottom() } else { b.top() },
            );
            let anchor = match (right, up) {
                (true, true) => Anchor::BottomRight,
                (true, false) => Anchor::TopRight,
                (false, true) => Anchor::BottomLeft,
                (false, false) => Anchor::TopLeft,
            };
            let mut menu = column()
                .id("filter-options")
                .w(width)
                .flex_none()
                .max_h(max_h)
                .overflow_y_scroll()
                .p(px(5.))
                .relative()
                .left(px(12. * (1. - self.pane.get(w, cx))))
                .opacity(((a - 0.2) / 0.8).clamp(0., 1.))
                .child(
                    button("back-filter", add, cx)
                        .justify_start()
                        .child(if let Some(f) = self.facet {
                            self.facets[f].name.clone()
                        } else {
                            add.into()
                        })
                        .on_click(cx.listener(|s, _, w, cx| {
                            if s.facet.take().is_some() {
                                s.pane.reset(0.);
                                s.pane.duration(1., 0.16, cx);
                            } else {
                                s.menu = false;
                                s.launch.as_ref().unwrap().focus(w, cx);
                            }
                            cx.notify()
                        })),
                );
            if let Some(f) = self.facet {
                for (i, v) in self.facets[f].values.iter().enumerate() {
                    menu = menu.child(
                        button(("val", i), v.clone(), cx)
                            .justify_start()
                            .child(v.clone())
                            .on_click(cx.listener(move |s, _, w, cx| {
                                s.set(f, Some(i), cx);
                                s.launch.as_ref().unwrap().focus(w, cx);
                            })),
                    )
                }
            } else {
                for (i, facet) in self.facets.iter().enumerate() {
                    menu = menu.child(
                        button(("facet", i), facet.name.clone(), cx)
                            .justify_between()
                            .child(facet.name.clone())
                            .child(icon(Glyph::Arrow, 12., p.faint))
                            .on_click(cx.listener(move |s, _, _, cx| {
                                s.pane.reset(0.);
                                s.pane.duration(1., 0.18, cx);
                                s.facet = Some(i);
                                cx.notify()
                            })),
                    )
                }
            }
            let shell = panel(cx)
                .id("filter-menu")
                .track_focus(&menu_focus)
                .tab_stop(false)
                .role(Role::Group)
                .occlude()
                .w(b.size.width + (width - b.size.width) * a)
                .h(b.size.height + (height - b.size.height) * a)
                .shadow_lg()
                .on_mouse_down_out(cx.listener(|s, e: &MouseDownEvent, _, cx| {
                    if !s.anchor.get().contains(&e.position) {
                        s.menu = false;
                        cx.notify();
                    }
                }))
                .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                    if e.keystroke.key == "escape" {
                        s.menu = false;
                        s.launch.as_ref().unwrap().focus(w, cx);
                        cx.stop_propagation();
                        cx.notify();
                    }
                }))
                .child(menu);
            root = root.child(overlay(shell, pos, anchor));
        }
        root.on_key_down(cx.listener(|s, e: &KeyDownEvent, _, cx| {
            if e.keystroke.key == "escape" {
                s.menu = false;
                cx.notify()
            }
        }))
    }
}
