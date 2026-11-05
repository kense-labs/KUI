//! A host-routed navigation surface with expandable menus and local search.
use crate::primitives::{
    editor::{EditEvent, Editor, InputMode},
    family::{face, Paint, Slide, Tone},
    presentation::{column, hint, row, tr},
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc, time::Duration};
#[derive(Clone, Debug)]
pub struct MorphNavLink {
    pub id: SharedString,
    pub label: SharedString,
    pub description: SharedString,
}
impl MorphNavLink {
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            description: "".into(),
        }
    }
    pub fn with_description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = description.into();
        self
    }
}
#[derive(Clone, Debug)]
pub struct MorphNavItem {
    pub id: SharedString,
    pub label: SharedString,
    pub links: Vec<MorphNavLink>,
}
impl MorphNavItem {
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            links: vec![],
        }
    }
    pub fn with_links(mut self, links: Vec<MorphNavLink>) -> Self {
        self.links = links;
        self
    }
}
#[derive(Clone, Debug)]
pub struct MorphNavNavigate {
    pub id: SharedString,
    pub label: SharedString,
    pub section: Option<SharedString>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Panel {
    Menu(usize),
    Search,
    Compact,
}
pub struct MorphNav {
    brand: SharedString,
    brand_id: SharedString,
    items: Vec<MorphNavItem>,
    action: Option<MorphNavLink>,
    current: Option<SharedString>,
    panel: Option<Panel>,
    progress: Slide,
    compact: bool,
    visible: bool,
    bounds: Rc<Cell<Bounds<Pixels>>>,
    trigger_focus: Vec<FocusHandle>,
    link_focus: Vec<Vec<FocusHandle>>,
    search_focus: FocusHandle,
    menu_focus: FocusHandle,
    root_focus: FocusHandle,
    search: Entity<Editor>,
    query: SharedString,
    result_index: usize,
    hover_epoch: u64,
    hovered: bool,
    focus_subscriptions: Vec<Subscription>,
}
impl EventEmitter<MorphNavNavigate> for MorphNav {}
impl MorphNav {
    pub fn new(
        brand: impl Into<SharedString>,
        items: Vec<MorphNavItem>,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let search = cx.new(|cx| {
            let mut e = Editor::new(
                tr(cx, "Search navigation", "搜索导航"),
                InputMode::Combobox,
                w,
                cx,
            );
            e.set_placeholder(tr(cx, "Search destinations", "搜索目标"));
            e
        });
        cx.subscribe_in(
            &search,
            w,
            |s, editor, event: &EditEvent, w, cx| match event {
                EditEvent::Changed => {
                    s.query = editor.read(cx).value().to_owned().into();
                    s.result_index = 0;
                    cx.notify();
                }
                EditEvent::Navigate(down) => {
                    let n = s.results().len();
                    if n > 0 {
                        s.result_index = if *down {
                            (s.result_index + 1).min(n - 1)
                        } else {
                            s.result_index.saturating_sub(1)
                        };
                        cx.notify();
                    }
                }
                EditEvent::Submitted => {
                    if let Some((section, link)) = s.results().get(s.result_index).copied() {
                        s.navigate(section, link, cx);
                    }
                }
                EditEvent::Escape => {
                    s.close(cx);
                    s.search_focus.focus(w, cx);
                }
                _ => {}
            },
        )
        .detach();
        let trigger_focus = (0..items.len())
            .map(|_| cx.focus_handle().tab_stop(true))
            .collect();
        let link_focus = items
            .iter()
            .map(|item| {
                item.links
                    .iter()
                    .map(|_| cx.focus_handle().tab_stop(true))
                    .collect()
            })
            .collect();
        Self {
            brand: brand.into(),
            brand_id: "brand".into(),
            items,
            action: None,
            current: None,
            panel: None,
            progress: Slide::new(0.),
            compact: false,
            visible: true,
            bounds: Default::default(),
            trigger_focus,
            link_focus,
            search_focus: cx.focus_handle().tab_stop(true),
            menu_focus: cx.focus_handle().tab_stop(true),
            root_focus: cx.focus_handle(),
            search,
            query: "".into(),
            result_index: 0,
            hover_epoch: 0,
            hovered: false,
            focus_subscriptions: vec![],
        }
    }
    pub fn with_brand_id(mut self, id: impl Into<SharedString>) -> Self {
        self.brand_id = id.into();
        self
    }
    pub fn with_action(mut self, action: MorphNavLink) -> Self {
        self.action = Some(action);
        self
    }
    pub fn set_current(&mut self, id: Option<SharedString>, cx: &mut Context<Self>) {
        self.current = id;
        cx.notify();
    }
    pub fn set_compact(&mut self, compact: bool, cx: &mut Context<Self>) {
        self.compact = compact;
        cx.notify();
    }
    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        if self.visible != visible {
            self.visible = visible;
            cx.notify();
        }
    }
    fn results(&self) -> Vec<(usize, Option<usize>)> {
        let q = self.query.to_lowercase();
        let mut results = Vec::new();
        for (i, item) in self.items.iter().enumerate() {
            if item.links.is_empty() && (q.is_empty() || item.label.to_lowercase().contains(&q)) {
                results.push((i, None));
            }
            for (j, link) in item.links.iter().enumerate() {
                if q.is_empty()
                    || link.label.to_lowercase().contains(&q)
                    || link.description.to_lowercase().contains(&q)
                    || item.label.to_lowercase().contains(&q)
                {
                    results.push((i, Some(j)));
                }
            }
        }
        results.truncate(6);
        results
    }
    fn open(&mut self, panel: Panel, cx: &mut Context<Self>) {
        self.hover_epoch = self.hover_epoch.wrapping_add(1);
        self.panel = Some(panel);
        self.progress.duration(1., 0.24, cx);
        cx.notify();
    }
    fn close(&mut self, cx: &mut Context<Self>) {
        self.hover_epoch = self.hover_epoch.wrapping_add(1);
        self.panel = None;
        self.progress.duration(0., 0.18, cx);
        cx.notify();
    }
    fn delayed_open(&mut self, index: usize, cx: &mut Context<Self>) {
        if self
            .items
            .get(index)
            .is_none_or(|item| item.links.is_empty())
        {
            return;
        }
        self.hover_epoch = self.hover_epoch.wrapping_add(1);
        let epoch = self.hover_epoch;
        cx.spawn(async move |view, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(60))
                .await;
            let _ = view.update(cx, |s, cx| {
                if s.hover_epoch == epoch {
                    s.open(Panel::Menu(index), cx);
                }
            });
        })
        .detach();
    }
    fn delayed_close(&mut self, cx: &mut Context<Self>) {
        self.hover_epoch = self.hover_epoch.wrapping_add(1);
        let epoch = self.hover_epoch;
        cx.spawn(async move |view, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(180))
                .await;
            let _ = view.update(cx, |s, cx| {
                if s.hover_epoch == epoch && !s.hovered {
                    s.close(cx);
                }
            });
        })
        .detach();
    }
    fn navigate(&mut self, section: usize, link: Option<usize>, cx: &mut Context<Self>) {
        let Some(item) = self.items.get(section) else {
            return;
        };
        let target = if let Some(link) = link {
            let Some(link) = item.links.get(link) else {
                return;
            };
            MorphNavNavigate {
                id: link.id.clone(),
                label: link.label.clone(),
                section: Some(item.id.clone()),
            }
        } else {
            MorphNavNavigate {
                id: item.id.clone(),
                label: item.label.clone(),
                section: None,
            }
        };
        self.close(cx);
        cx.emit(target);
    }
    fn activate(&mut self, i: usize, w: &mut Window, cx: &mut Context<Self>, keyboard: bool) {
        if i >= self.items.len() {
            return;
        }
        if self.items[i].links.is_empty() {
            self.navigate(i, None, cx);
        } else if self.panel == Some(Panel::Menu(i)) && !keyboard {
            self.close(cx);
        } else {
            self.open(Panel::Menu(i), cx);
            if keyboard {
                if let Some(f) = self.link_focus[i].first().cloned() {
                    cx.defer_in(w, move |_, w, cx| f.focus(w, cx));
                }
            }
        }
    }
    fn trigger_key(&mut self, i: usize, e: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        let n = self.items.len();
        if n == 0 {
            return;
        }
        match e.keystroke.key.as_str() {
            "left" | "right" | "home" | "end" => {
                let next = match e.keystroke.key.as_str() {
                    "left" => (i + n - 1) % n,
                    "right" => (i + 1) % n,
                    "home" => 0,
                    _ => n - 1,
                };
                if self.panel.is_some() && !self.items[next].links.is_empty() {
                    self.open(Panel::Menu(next), cx);
                } else if self.panel.is_some() {
                    self.close(cx);
                }
                self.trigger_focus[next].focus(w, cx);
            }
            "down" => self.activate(i, w, cx, true),
            "enter" | "space" if !e.is_held => self.activate(i, w, cx, true),
            "escape" => self.close(cx),
            _ => return,
        }
        cx.stop_propagation();
        w.prevent_default();
    }
    fn link_key(
        &mut self,
        section: usize,
        index: usize,
        e: &KeyDownEvent,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let links = &self.link_focus[section];
        match e.keystroke.key.as_str() {
            "down" if index + 1 < links.len() => links[index + 1].focus(w, cx),
            "up" if index > 0 => links[index - 1].focus(w, cx),
            "up" => {
                if self.bounds.get().size.width > px(0.) && self.bounds.get().size.width < px(720.)
                {
                    self.menu_focus.focus(w, cx);
                } else {
                    self.trigger_focus[section].focus(w, cx);
                }
            }
            "escape" => {
                self.close(cx);
                if self.bounds.get().size.width > px(0.) && self.bounds.get().size.width < px(720.)
                {
                    self.menu_focus.focus(w, cx);
                } else {
                    self.trigger_focus[section].focus(w, cx);
                }
            }
            "enter" | "space" if !e.is_held => self.navigate(section, Some(index), cx),
            _ => return,
        }
        cx.stop_propagation();
        w.prevent_default();
    }
}
impl Render for MorphNav {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.focus_subscriptions.is_empty() {
            self.focus_subscriptions
                .push(cx.on_focus_out(&self.root_focus, w, |s, _, _, cx| {
                    if s.panel.is_some() {
                        s.close(cx);
                    }
                }));
        }
        let p = Paint::get(cx);
        let radius = crate::foundation::surface_radius(cx);
        let width = f32::from(self.bounds.get().size.width);
        let narrow = width > 0. && width < 720.;
        let progress = if self.visible && w.is_visible() {
            self.progress.get(w, cx)
        } else {
            self.progress.value(cx)
        }
        .clamp(0., 1.);
        let mut bar = row().w_full().items_center().gap_1().flex_wrap();
        let brand = self.brand_id.clone();
        let label = self.brand.clone();
        bar = bar.child(
            face(
                "morph-nav-brand",
                label.clone(),
                40.,
                radius.into(),
                Tone::Surface,
                cx,
            )
            .px(px(10.))
            .child(label.clone())
            .on_click(cx.listener(move |_, _, _, cx| {
                cx.emit(MorphNavNavigate {
                    id: brand.clone(),
                    label: label.clone(),
                    section: None,
                })
            }))
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, _, cx| {
                if matches!(e.keystroke.key.as_str(), "enter" | "space") && !e.is_held {
                    cx.emit(MorphNavNavigate {
                        id: s.brand_id.clone(),
                        label: s.brand.clone(),
                        section: None,
                    });
                    cx.stop_propagation();
                }
            })),
        );
        if narrow {
            bar = bar.child(
                face(
                    "morph-nav-menu",
                    tr(cx, "Menu", "菜单"),
                    40.,
                    radius.into(),
                    Tone::Surface,
                    cx,
                )
                .px(px(10.))
                .track_focus(&self.menu_focus)
                .aria_toggled((self.panel == Some(Panel::Compact)).into())
                .child(tr(cx, "Menu", "菜单"))
                .on_click(cx.listener(|s, _, _, cx| {
                    if s.panel == Some(Panel::Compact) {
                        s.close(cx)
                    } else {
                        s.open(Panel::Compact, cx)
                    }
                }))
                .on_key_down(cx.listener(|s, e: &KeyDownEvent, _, cx| {
                    match e.keystroke.key.as_str() {
                        "enter" | "space" | "down" if !e.is_held => {
                            if s.panel == Some(Panel::Compact) {
                                s.close(cx)
                            } else {
                                s.open(Panel::Compact, cx)
                            }
                        }
                        "escape" => s.close(cx),
                        _ => return,
                    }
                    cx.stop_propagation();
                })),
            );
        } else {
            for (i, item) in self.items.iter().enumerate() {
                let active =
                    self.panel == Some(Panel::Menu(i)) || self.current.as_ref() == Some(&item.id);
                bar = bar.child(
                    face(
                        ("morph-nav-trigger", i),
                        item.label.clone(),
                        40.,
                        radius.into(),
                        if active { Tone::Primary } else { Tone::Surface },
                        cx,
                    )
                    .px(px(if self.compact { 7. } else { 11. }))
                    .track_focus(&self.trigger_focus[i])
                    .aria_toggled((self.panel == Some(Panel::Menu(i))).into())
                    .child(item.label.clone())
                    .on_hover(cx.listener(move |s, over: &bool, _, cx| {
                        if *over {
                            s.delayed_open(i, cx)
                        } else {
                            s.hover_epoch = s.hover_epoch.wrapping_add(1);
                        }
                    }))
                    .on_click(cx.listener(move |s, _, w, cx| s.activate(i, w, cx, false)))
                    .on_key_down(
                        cx.listener(move |s, e: &KeyDownEvent, w, cx| s.trigger_key(i, e, w, cx)),
                    ),
                );
            }
        }
        bar = bar.child(
            face(
                "morph-nav-search",
                tr(cx, "Search", "搜索"),
                40.,
                radius.into(),
                Tone::Surface,
                cx,
            )
            .px(px(10.))
            .track_focus(&self.search_focus)
            .aria_toggled((self.panel == Some(Panel::Search)).into())
            .child("⌕")
            .on_click(cx.listener(|s, _, w, cx| {
                s.open(Panel::Search, cx);
                let search = s.search.clone();
                cx.defer_in(w, move |_, w, cx| search.read(cx).focus(w, cx));
            }))
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                if matches!(e.keystroke.key.as_str(), "enter" | "space" | "down") && !e.is_held {
                    s.open(Panel::Search, cx);
                    let search = s.search.clone();
                    cx.defer_in(w, move |_, w, cx| search.read(cx).focus(w, cx));
                    cx.stop_propagation();
                }
            })),
        );
        if let Some(action) = self.action.clone() {
            let copy = action.clone();
            bar = bar.child(
                face(
                    "morph-nav-action",
                    action.label.clone(),
                    40.,
                    radius.into(),
                    Tone::Primary,
                    cx,
                )
                .px(px(12.))
                .child(action.label.clone())
                .on_click(cx.listener(move |s, _, _, cx| {
                    s.close(cx);
                    cx.emit(MorphNavNavigate {
                        id: copy.id.clone(),
                        label: copy.label.clone(),
                        section: None,
                    });
                }))
                .on_key_down(cx.listener(move |s, e: &KeyDownEvent, _, cx| {
                    if matches!(e.keystroke.key.as_str(), "enter" | "space") && !e.is_held {
                        s.close(cx);
                        cx.emit(MorphNavNavigate {
                            id: action.id.clone(),
                            label: action.label.clone(),
                            section: None,
                        });
                        cx.stop_propagation();
                    }
                })),
            );
        }
        let mut body = column().id("morph-nav-content").w_full().gap_1();
        let panel_height = match self.panel {
            Some(Panel::Menu(section)) => {
                let links = self
                    .items
                    .get(section)
                    .map(|v| v.links.as_slice())
                    .unwrap_or(&[]);
                let columns = if width >= 420. && links.len() > 3 {
                    2
                } else {
                    1
                };
                let rows = links.len().div_ceil(columns);
                for start in (0..links.len()).step_by(columns) {
                    let mut line = row().w_full().gap_1();
                    for i in start..(start + columns).min(links.len()) {
                        let link = &links[i];
                        let label = link.label.clone();
                        let description = link.description.clone();
                        line = line.child(
                            face(
                                ("morph-nav-link", i),
                                label.clone(),
                                52.,
                                radius.into(),
                                Tone::Surface,
                                cx,
                            )
                            .flex_1()
                            .min_w(px(0.))
                            .px(px(10.))
                            .track_focus(&self.link_focus[section][i])
                            .child(column().w_full().child(label).child(hint(description, p)))
                            .on_click(
                                cx.listener(move |s, _, _, cx| s.navigate(section, Some(i), cx)),
                            )
                            .on_key_down(cx.listener(
                                move |s, e: &KeyDownEvent, w, cx| s.link_key(section, i, e, w, cx),
                            )),
                        );
                    }
                    body = body.child(line);
                }
                (rows as f32 * 56. + 8.).min(320.)
            }
            Some(Panel::Compact) => {
                for (i, item) in self.items.iter().enumerate() {
                    body = body.child(
                        face(
                            ("morph-nav-compact", i),
                            item.label.clone(),
                            42.,
                            radius.into(),
                            Tone::Surface,
                            cx,
                        )
                        .w_full()
                        .px(px(10.))
                        .child(item.label.clone())
                        .on_click(cx.listener(move |s, _, w, cx| s.activate(i, w, cx, false)))
                        .on_key_down(cx.listener(
                            move |s, e: &KeyDownEvent, w, cx| {
                                if matches!(e.keystroke.key.as_str(), "enter" | "space")
                                    && !e.is_held
                                {
                                    s.activate(i, w, cx, true);
                                    cx.stop_propagation();
                                } else if e.keystroke.key == "escape" {
                                    s.close(cx);
                                    s.menu_focus.focus(w, cx);
                                    cx.stop_propagation();
                                }
                            },
                        )),
                    );
                }
                (self.items.len() as f32 * 46. + 8.).min(320.)
            }
            Some(Panel::Search) => {
                body = body.child(
                    div()
                        .w_full()
                        .h(px(40.))
                        .px(px(8.))
                        .border_1()
                        .border_color(p.line)
                        .rounded(radius)
                        .child(self.search.clone()),
                );
                let results = self.results();
                for (idx, (i, j)) in results.iter().copied().enumerate() {
                    let label = j
                        .and_then(|j| self.items[i].links.get(j))
                        .map_or_else(|| self.items[i].label.clone(), |link| link.label.clone());
                    body = body.child(
                        face(
                            ("morph-nav-result", idx),
                            label.clone(),
                            42.,
                            radius.into(),
                            if idx == self.result_index {
                                Tone::Primary
                            } else {
                                Tone::Surface
                            },
                            cx,
                        )
                        .w_full()
                        .px(px(10.))
                        .child(label)
                        .on_click(cx.listener(move |s, _, _, cx| s.navigate(i, j, cx)))
                        .on_key_down(cx.listener(
                            move |s, e: &KeyDownEvent, w, cx| {
                                if matches!(e.keystroke.key.as_str(), "enter" | "space")
                                    && !e.is_held
                                {
                                    s.navigate(i, j, cx);
                                    cx.stop_propagation();
                                } else if e.keystroke.key == "escape" {
                                    s.close(cx);
                                    s.search_focus.focus(w, cx);
                                    cx.stop_propagation();
                                }
                            },
                        )),
                    );
                }
                48. + results.len() as f32 * 46.
            }
            None => 0.,
        };
        let bounds = self.bounds.clone();
        column()
            .id("morph-nav-root")
            .role(Role::Navigation)
            .aria_label(tr(cx, "Main navigation", "主导航"))
            .track_focus(&self.root_focus)
            .relative()
            .w_full()
            .max_w(px(760.))
            .rounded(radius)
            .border_1()
            .border_color(p.line)
            .bg(p.panel)
            .text_color(p.ink)
            .p(px(6.))
            .on_mouse_down_out(cx.listener(|s, _, _, cx| s.close(cx)))
            .on_hover(cx.listener(|s, over: &bool, _, cx| {
                s.hovered = *over;
                if *over {
                    s.hover_epoch = s.hover_epoch.wrapping_add(1);
                } else {
                    s.delayed_close(cx);
                }
            }))
            .child(
                canvas(
                    move |b, w, _| {
                        if bounds.get().size.width != b.size.width {
                            w.request_animation_frame();
                        }
                        bounds.set(b);
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .inset_0(),
            )
            .child(bar)
            .when(self.panel.is_some(), |v| {
                v.child(
                    div()
                        .id("morph-nav-panel")
                        .w_full()
                        .h(px(panel_height * progress))
                        .overflow_y_scroll()
                        .opacity(progress)
                        .child(body),
                )
            })
    }
}
