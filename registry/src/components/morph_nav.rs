//! A host-routed navigation bar that morphs into menus or search.
use crate::{
    foundation::surface_radius,
    primitives::{
        editor::{EditEvent, Editor, InputMode},
        family::{Paint, PhaseClock, Slide, Tone, disabled_motion, ease, face, measure},
        flyout::TriggerBounds,
        glyph::{Glyph, icon},
        presentation::{column, row, tr},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc, time::Duration};

/// A destination rendered in a panel, on the bar, or as the primary action.
#[derive(Clone, Debug)]
pub struct MorphNavLink {
    pub id: SharedString,
    pub label: SharedString,
    pub description: SharedString,
    pub href: Option<SharedString>,
    pub icon: Option<Glyph>,
}

impl MorphNavLink {
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            description: "".into(),
            href: None,
            icon: None,
        }
    }

    pub fn with_description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = description.into();
        self
    }

    pub fn with_href(mut self, href: impl Into<SharedString>) -> Self {
        self.href = Some(href.into());
        self
    }

    pub fn with_icon(mut self, icon: Glyph) -> Self {
        self.icon = Some(icon);
        self
    }
}

/// Column policy for an expanded section.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MorphNavColumns {
    #[default]
    Automatic,
    One,
    Two,
}

/// A direct destination or a section with links and optional feature and footer destinations.
#[derive(Clone, Debug)]
pub struct MorphNavItem {
    pub destination: MorphNavLink,
    pub links: Vec<MorphNavLink>,
    pub columns: MorphNavColumns,
    pub feature: Option<MorphNavLink>,
    pub feature_art: Option<AnyView>,
    pub footer: Option<MorphNavLink>,
}

impl MorphNavItem {
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            destination: MorphNavLink::new(id, label),
            links: vec![],
            columns: MorphNavColumns::Automatic,
            feature: None,
            feature_art: None,
            footer: None,
        }
    }

    pub fn with_href(mut self, href: impl Into<SharedString>) -> Self {
        self.destination.href = Some(href.into());
        self
    }

    pub fn with_links(mut self, links: Vec<MorphNavLink>) -> Self {
        self.links = links;
        self
    }

    pub fn with_columns(mut self, columns: MorphNavColumns) -> Self {
        self.columns = columns;
        self
    }

    pub fn with_feature(mut self, feature: MorphNavLink) -> Self {
        self.feature = Some(feature);
        self
    }

    /// Places a decorative GPUI view above the feature destination's text.
    pub fn with_feature_art(mut self, art: AnyView) -> Self {
        self.feature_art = Some(art);
        self
    }

    pub fn with_footer(mut self, footer: MorphNavLink) -> Self {
        self.footer = Some(footer);
        self
    }

    fn destinations(&self) -> impl Iterator<Item = &MorphNavLink> {
        self.links
            .iter()
            .chain(self.feature.iter())
            .chain(self.footer.iter())
    }

    fn menu_len(&self) -> usize {
        self.links.len().max(1)
    }

    fn has_panel(&self) -> bool {
        self.destinations().next().is_some()
    }
}

/// An independently indexed search result.
#[derive(Clone, Debug)]
pub struct MorphNavSearchItem {
    pub destination: MorphNavLink,
    pub group: Option<SharedString>,
    pub section: Option<SharedString>,
    pub keywords: Vec<SharedString>,
}

impl MorphNavSearchItem {
    pub fn new(destination: MorphNavLink) -> Self {
        Self {
            destination,
            group: None,
            section: None,
            keywords: vec![],
        }
    }

    pub fn with_group(mut self, group: impl Into<SharedString>) -> Self {
        self.group = Some(group.into());
        self
    }

    pub fn with_section(mut self, section: impl Into<SharedString>) -> Self {
        self.section = Some(section.into());
        self
    }

    pub fn with_keywords(mut self, keywords: Vec<SharedString>) -> Self {
        self.keywords = keywords;
        self
    }
}

/// Routes a selected destination through the host.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MorphNavNavigate {
    pub id: SharedString,
    pub label: SharedString,
    pub href: Option<SharedString>,
    pub section: Option<SharedString>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Panel {
    Section(usize),
    Menu,
    Search,
}

struct NavSearch {
    editor: Entity<Editor>,
    items: Vec<MorphNavSearchItem>,
    query: String,
    results: Vec<usize>,
    active: usize,
    _edit: Subscription,
}

impl NavSearch {
    fn new(
        placeholder: SharedString,
        items: Vec<MorphNavSearchItem>,
        w: &mut Window,
        cx: &mut Context<MorphNav>,
    ) -> Self {
        let label = tr(cx, "Search", "搜索");
        let editor = cx.new(|cx| {
            let mut editor = Editor::new(label, InputMode::Combobox, w, cx);
            editor.set_placeholder(placeholder);
            editor
        });
        let edit = cx.subscribe_in(&editor, w, |nav, editor, event: &EditEvent, w, cx| {
            if nav.panel != Some(Panel::Search) {
                return;
            }
            let Some(search) = nav.search.as_mut() else {
                return;
            };
            match event {
                EditEvent::Changed => search.filter(editor.read(cx).value()),
                EditEvent::Navigate(down) => {
                    if *down {
                        search.active =
                            (search.active + 1).min(search.results.len().saturating_sub(1));
                    } else {
                        search.active = search.active.saturating_sub(1);
                    }
                }
                EditEvent::Submitted => {
                    let index = search.active;
                    nav.choose_result(index, w, cx);
                }
                EditEvent::Escape => nav.restore_focus(w, cx),
                _ => {}
            }
            cx.notify();
        });
        let mut search = Self {
            editor,
            items,
            query: String::new(),
            results: vec![],
            active: 0,
            _edit: edit,
        };
        search.filter("");
        search
    }

    fn filter(&mut self, input: &str) {
        self.query = input.trim().to_lowercase();
        self.active = 0;
        if self.query.is_empty() {
            self.results = (0..self.items.len().min(5)).collect();
            return;
        }
        let mut ranked = self
            .items
            .iter()
            .enumerate()
            .filter_map(|(i, item)| {
                let label = item.destination.label.to_lowercase();
                let score = if label == self.query {
                    0
                } else if label.starts_with(&self.query) {
                    1
                } else if label.contains(&self.query) {
                    2
                } else if item
                    .destination
                    .description
                    .to_lowercase()
                    .contains(&self.query)
                    || item
                        .group
                        .as_ref()
                        .is_some_and(|group| group.to_lowercase().contains(&self.query))
                    || item
                        .keywords
                        .iter()
                        .any(|keyword| keyword.to_lowercase().contains(&self.query))
                {
                    3
                } else {
                    return None;
                };
                Some((score, i))
            })
            .collect::<Vec<_>>();
        ranked.sort_unstable();
        self.results = ranked.into_iter().take(6).map(|(_, i)| i).collect();
    }
}

/// A responsive navigation landmark with a single expanding surface.
pub struct MorphNav {
    label: Option<SharedString>,
    brand: MorphNavLink,
    items: Vec<MorphNavItem>,
    action: Option<MorphNavLink>,
    search: Option<NavSearch>,
    current: Option<SharedString>,
    visible: bool,
    collapse_below: Pixels,
    panel: Option<Panel>,
    painted: Option<Panel>,
    panel_clock: PhaseClock,
    enter_side: f32,
    panel_height: Slide,
    shell_width: Option<Slide>,
    density: Slide,
    highlight: Option<(Slide, Slide)>,
    highlight_alpha: Slide,
    hovered: Option<usize>,
    hover_open: bool,
    hover_task: Option<Task<()>>,
    last_touch: PhaseClock,
    container: Rc<Cell<Bounds<Pixels>>>,
    pointer_inside: Rc<Cell<bool>>,
    surface: TriggerBounds,
    triggers: Vec<TriggerBounds>,
    measured: Rc<Cell<Option<(Panel, Pixels)>>>,
    root_focus: FocusHandle,
    search_focus: FocusHandle,
    menu_focus: FocusHandle,
    trigger_focus: Vec<FocusHandle>,
    link_focus: Vec<Vec<FocusHandle>>,
    _focus_exit: Subscription,
}

impl EventEmitter<MorphNavNavigate> for MorphNav {}

impl MorphNav {
    pub fn new(
        brand: impl Into<SharedString>,
        items: Vec<MorphNavItem>,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let root_focus = cx.focus_handle();
        let focus_exit = cx.on_focus_out(&root_focus, w, |s, _, _, cx| s.close(cx));
        Self {
            label: None,
            brand: MorphNavLink::new("brand", brand),
            action: None,
            search: None,
            current: None,
            visible: true,
            collapse_below: px(720.),
            panel: None,
            painted: None,
            panel_clock: PhaseClock::default(),
            enter_side: 0.,
            panel_height: Slide::new(0.),
            shell_width: None,
            density: Slide::new(0.),
            highlight: None,
            highlight_alpha: Slide::new(0.),
            hovered: None,
            hover_open: false,
            hover_task: None,
            last_touch: PhaseClock::default(),
            container: Rc::default(),
            pointer_inside: Rc::default(),
            surface: TriggerBounds::default(),
            triggers: (0..items.len()).map(|_| TriggerBounds::default()).collect(),
            measured: Rc::default(),
            root_focus,
            search_focus: cx.focus_handle().tab_stop(true),
            menu_focus: cx.focus_handle().tab_stop(true),
            trigger_focus: items
                .iter()
                .map(|_| cx.focus_handle().tab_stop(true))
                .collect(),
            link_focus: items
                .iter()
                .map(|item| {
                    (0..item.destinations().count().max(1))
                        .map(|_| cx.focus_handle().tab_stop(true))
                        .collect()
                })
                .collect(),
            items,
            _focus_exit: focus_exit,
        }
    }

    pub fn with_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn with_brand(mut self, brand: MorphNavLink) -> Self {
        self.brand = brand;
        self
    }

    pub fn with_brand_id(mut self, id: impl Into<SharedString>) -> Self {
        self.brand.id = id.into();
        self
    }

    pub fn with_action(mut self, action: MorphNavLink) -> Self {
        self.action = Some(action);
        self
    }

    pub fn with_search(
        mut self,
        placeholder: impl Into<SharedString>,
        items: Vec<MorphNavSearchItem>,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        self.search = Some(NavSearch::new(placeholder.into(), items, w, cx));
        self
    }

    pub fn with_collapse_below(mut self, width: Pixels) -> Self {
        self.collapse_below = width.max(px(0.));
        self
    }

    pub fn with_current(mut self, id: impl Into<SharedString>) -> Self {
        self.current = Some(id.into());
        self
    }

    pub fn with_compact(mut self, compact: bool) -> Self {
        self.density.reset(f32::from(compact));
        self
    }

    pub fn set_current(&mut self, id: Option<SharedString>, cx: &mut Context<Self>) {
        self.current = id;
        cx.notify();
    }

    pub fn set_compact(&mut self, compact: bool, cx: &mut Context<Self>) {
        self.density.duration(f32::from(compact), 0.22, cx);
        cx.notify();
    }

    /// Stops frame requests when the embedding host hides the control.
    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        if self.visible != visible {
            self.visible = visible;
            cx.notify();
        }
    }

    fn narrow(&self, w: &Window) -> bool {
        let width = self.container.get().size.width;
        (if width == px(0.) {
            w.viewport_size().width
        } else {
            width
        }) < self.collapse_below
    }

    fn open(&mut self, panel: Panel, hover: bool, cx: &mut Context<Self>) {
        self.hover_task = None;
        self.hover_open = hover;
        if self.panel == Some(panel) {
            return;
        }
        self.enter_side = match (self.painted, panel) {
            (Some(Panel::Section(previous)), Panel::Section(next)) => {
                (next as f32 - previous as f32).signum()
            }
            _ => 0.,
        };
        self.panel = Some(panel);
        self.painted = Some(panel);
        self.panel_clock.restart(cx);
        cx.notify();
    }

    fn close(&mut self, cx: &mut Context<Self>) {
        self.hover_task = None;
        self.hover_open = false;
        if self.panel.take().is_some() {
            self.panel_clock.restart(cx);
            self.panel_height.duration(0., 0.22, cx);
            cx.notify();
        }
    }

    fn hover(&mut self, i: usize, over: bool, cx: &mut Context<Self>) {
        if !over {
            if self.hovered == Some(i) {
                self.hovered = None;
                self.hover_task = None;
                cx.notify();
            }
            return;
        }
        if self.last_touch.age(cx) < 0.5 {
            return;
        }
        self.hovered = Some(i);
        self.hover_task = Some(cx.spawn(async move |view, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(60))
                .await;
            let _ = view.update(cx, |s, cx| {
                if s.items[i].has_panel() {
                    s.open(Panel::Section(i), true, cx);
                } else if s.hover_open {
                    s.close(cx);
                }
            });
        }));
        cx.notify();
    }

    fn leave(&mut self, cx: &mut Context<Self>) {
        self.hovered = None;
        self.hover_task = None;
        if self.hover_open {
            self.hover_task = Some(cx.spawn(async |view, cx| {
                cx.background_executor()
                    .timer(Duration::from_millis(180))
                    .await;
                let _ = view.update(cx, |s, cx| s.close(cx));
            }));
        }
        cx.notify();
    }

    fn restore_focus(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let focus = match self.panel {
            Some(Panel::Search) => self.search_focus.clone(),
            Some(Panel::Section(i)) if !self.narrow(w) => self.trigger_focus[i].clone(),
            Some(_) => self.menu_focus.clone(),
            None => return,
        };
        self.close(cx);
        focus.focus(w, cx);
    }

    fn emit(&mut self, link: MorphNavLink, section: Option<SharedString>, cx: &mut Context<Self>) {
        self.close(cx);
        cx.emit(MorphNavNavigate {
            id: link.id,
            label: link.label,
            href: link.href,
            section,
        });
    }

    fn choose_link(
        &mut self,
        section: usize,
        index: usize,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let item = &self.items[section];
        let link = item
            .destinations()
            .nth(index)
            .cloned()
            .unwrap_or_else(|| item.destination.clone());
        let parent = item.has_panel().then(|| item.destination.id.clone());
        self.restore_focus(w, cx);
        self.emit(link, parent, cx);
    }

    fn choose_result(&mut self, index: usize, w: &mut Window, cx: &mut Context<Self>) {
        let Some(search) = &self.search else { return };
        let Some(entry) = search
            .results
            .get(index)
            .and_then(|i| search.items.get(*i))
            .cloned()
        else {
            return;
        };
        self.restore_focus(w, cx);
        self.emit(entry.destination, entry.section, cx);
    }

    fn show_search(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let Some(search) = self.search.as_mut() else {
            return;
        };
        search.filter("");
        let editor = search.editor.clone();
        self.open(Panel::Search, false, cx);
        editor.update(cx, |editor, cx| editor.set_value("", w, cx));
        cx.defer_in(w, move |_, w, cx| {
            editor.update(cx, |editor, cx| editor.focus(w, cx));
            w.request_virtual_keyboard();
        });
    }

    fn activate(&mut self, i: usize, keyboard: bool, w: &mut Window, cx: &mut Context<Self>) {
        if !self.items[i].has_panel() {
            self.emit(self.items[i].destination.clone(), None, cx);
        } else if self.panel == Some(Panel::Section(i)) && !keyboard && !self.hover_open {
            self.close(cx);
        } else {
            self.open(Panel::Section(i), false, cx);
            if keyboard {
                let focus = self.link_focus[i][0].clone();
                cx.defer_in(w, move |_, w, cx| focus.focus(w, cx));
            }
        }
    }

    fn toggle_menu(&mut self, keyboard: bool, w: &mut Window, cx: &mut Context<Self>) {
        if self.panel == Some(Panel::Menu) {
            self.restore_focus(w, cx);
        } else {
            self.open(Panel::Menu, false, cx);
            if keyboard && let Some(first) = self.link_focus.first() {
                let focus = first[0].clone();
                cx.defer_in(w, move |_, w, cx| focus.focus(w, cx));
            }
        }
    }

    fn trigger_key(
        &mut self,
        i: usize,
        event: &KeyDownEvent,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let n = self.items.len();
        match event.keystroke.key.as_str() {
            "left" | "right" | "home" | "end" => {
                let rtl = crate::primitives::control::UiDirection::current(cx).is_rtl();
                let next = match event.keystroke.key.as_str() {
                    "home" => 0,
                    "end" => n - 1,
                    "right" if !rtl => (i + 1) % n,
                    "left" if rtl => (i + 1) % n,
                    _ => (i + n - 1) % n,
                };
                if self.panel.is_some() {
                    if self.items[next].has_panel() {
                        self.open(Panel::Section(next), false, cx);
                    } else {
                        self.close(cx);
                    }
                }
                self.trigger_focus[next].focus(w, cx);
            }
            "down" if self.items[i].has_panel() => self.activate(i, true, w, cx),
            "enter" | "space" if !event.is_held => self.activate(i, true, w, cx),
            _ => return,
        }
        cx.stop_propagation();
        w.prevent_default();
    }

    fn link_key(
        &mut self,
        section: usize,
        index: usize,
        event: &KeyDownEvent,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let len = if self.panel == Some(Panel::Menu) {
            self.items[section].menu_len()
        } else {
            self.link_focus[section].len()
        };
        let links = &self.link_focus[section][..len];
        let focus = match event.keystroke.key.as_str() {
            "down" if index + 1 < links.len() => Some(&links[index + 1]),
            "down" if self.panel == Some(Panel::Menu) => {
                self.link_focus.get(section + 1).and_then(|row| row.first())
            }
            "up" if index > 0 => Some(&links[index - 1]),
            "up" if self.panel == Some(Panel::Menu) && section > 0 => {
                self.link_focus[section - 1].get(self.items[section - 1].menu_len() - 1)
            }
            "up" if self.narrow(w) => Some(&self.menu_focus),
            "up" => Some(&self.trigger_focus[section]),
            "enter" | "space" if !event.is_held => {
                self.choose_link(section, index, w, cx);
                None
            }
            _ => return,
        };
        if let Some(focus) = focus {
            focus.focus(w, cx);
        }
        cx.stop_propagation();
        w.prevent_default();
    }

    fn density(&self, w: &Window, cx: &App) -> f32 {
        self.sample(&self.density, w, cx).clamp(0., 1.)
    }

    fn sample(&self, slide: &Slide, w: &Window, cx: &App) -> f32 {
        if self.visible && w.is_visible() {
            slide.get(w, cx)
        } else {
            slide.value(cx)
        }
    }

    fn top_width(&self, width: Pixels, narrow: bool, compact: f32, w: &Window, cx: &App) -> f32 {
        let text = |label: &SharedString| measure(label, 14., w).ceil();
        let height = 36. - 4. * compact;
        let pad = 28. - 6. * compact;
        let brand = if self.brand.label.is_empty() {
            0.
        } else {
            18. + if self.brand.icon.is_some() { 24. } else { 0. }
                + text(&self.brand.label)
                    * if self.brand.icon.is_some() {
                        if narrow { 0. } else { 1. - compact }
                    } else {
                        1.
                    }
        };
        let tabs = if narrow {
            text(&tr(cx, "Menu", "菜单").into()) + pad + 22.
        } else {
            self.items
                .iter()
                .map(|item| {
                    text(&item.destination.label) + pad + if item.has_panel() { 22. } else { 0. }
                })
                .sum()
        };
        let tools = self.search.as_ref().map_or(0., |_| height + 8.);
        let action = self
            .action
            .as_ref()
            .map_or(0., |link| text(&link.label) + 32. + 4.);
        (brand + tabs + tools + action + 24.).min(f32::from(width))
    }

    fn top_bar(&mut self, narrow: bool, density: f32, w: &Window, cx: &Context<Self>) -> Div {
        let height = 36. - 4. * density;
        let pad = 14. - 3. * density;
        let p = Paint::get(cx);
        let rtl = crate::primitives::control::UiDirection::current(cx).is_rtl();
        let mut bar = row()
            .w_full()
            .justify_center()
            .gap(px(4.))
            .when(rtl, |d| d.flex_row_reverse());
        if !self.brand.label.is_empty() {
            let link = &self.brand;
            bar = bar.child(
                nav_face(
                    "morph-nav-brand",
                    link.label.clone(),
                    height,
                    Tone::Clear,
                    cx,
                )
                .px(px(6.))
                .children(link.icon.map(|mark| icon(mark, 20., p.ink)))
                .when(!narrow || link.icon.is_none(), |e| {
                    let visibility = if link.icon.is_some() {
                        1. - density
                    } else {
                        1.
                    };
                    e.child(
                        div()
                            .w(px(measure(&link.label, 14., w).ceil() * visibility))
                            .overflow_hidden()
                            .opacity(visibility)
                            .child(link.label.clone()),
                    )
                })
                .on_click(cx.listener(|s, _, _, cx| s.emit(s.brand.clone(), None, cx)))
                .on_key_down(cx.listener(|s, event: &KeyDownEvent, w, cx| {
                    if activation(event, w, cx) {
                        s.emit(s.brand.clone(), None, cx);
                    }
                })),
            );
        }
        if narrow {
            bar = bar.child(
                nav_face(
                    "morph-nav-menu",
                    tr(cx, "Menu", "菜单"),
                    height,
                    Tone::Clear,
                    cx,
                )
                .px(px(pad))
                .track_focus(&self.menu_focus)
                .aria_expanded(self.panel == Some(Panel::Menu))
                .child(tr(cx, "Menu", "菜单"))
                .child(icon(Glyph::Down, 14., p.secondary))
                .on_click(cx.listener(|s, _, w, cx| s.toggle_menu(false, w, cx)))
                .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                    if activation(e, w, cx) || e.keystroke.key == "down" {
                        s.toggle_menu(true, w, cx);
                        cx.stop_propagation();
                    }
                })),
            );
        } else {
            let target = self.hovered.or_else(|| match self.panel {
                Some(Panel::Section(i)) => Some(i),
                _ => self
                    .items
                    .iter()
                    .position(|item| self.current.as_ref() == Some(&item.destination.id)),
            });
            let mut tabs = row()
                .relative()
                .flex_none()
                .when(rtl, |d| d.flex_row_reverse());
            let has_highlight = target.and_then(|i| {
                let b = self.triggers[i].get();
                (b.size.width > px(0.)).then_some(b)
            });
            self.highlight_alpha
                .duration(f32::from(has_highlight.is_some()), 0.12, cx);
            if let Some(bounds) = has_highlight {
                let origin = if rtl { self.triggers.len() - 1 } else { 0 };
                let x = f32::from(bounds.left() - self.triggers[origin].get().left());
                let width = f32::from(bounds.size.width);
                let (left, span) = self
                    .highlight
                    .get_or_insert((Slide::new(x), Slide::new(width)));
                left.duration(x, 0.24, cx);
                span.duration(width, 0.24, cx);
            }
            let alpha = self.sample(&self.highlight_alpha, w, cx);
            if let Some((left, span)) = &self.highlight
                && alpha > 0.001
            {
                tabs = tabs.child(
                    div()
                        .absolute()
                        .top_0()
                        .left(px(self.sample(left, w, cx)))
                        .w(px(self.sample(span, w, cx)))
                        .h(px(height))
                        .rounded(surface_radius(cx))
                        .bg(p.hover)
                        .opacity(alpha),
                );
            }
            for (i, item) in self.items.iter().enumerate() {
                let has_panel = item.has_panel();
                let link = &item.destination;
                let bounds = self.triggers[i].clone();
                tabs = tabs.child(
                    nav_surface(
                        ("morph-nav-trigger", i),
                        link.label.clone(),
                        height,
                        Tone::Clear,
                        cx,
                    )
                    .px(px(pad))
                    .relative()
                    .track_focus(&self.trigger_focus[i])
                    .when(has_panel, |e| {
                        e.aria_expanded(self.panel == Some(Panel::Section(i)))
                    })
                    .child(link.label.clone())
                    .when(has_panel, |e| e.child(icon(Glyph::Down, 14., p.secondary)))
                    .child(bounds.measure())
                    .on_hover(cx.listener(move |s, over: &bool, _, cx| s.hover(i, *over, cx)))
                    .on_click(cx.listener(move |s, _, w, cx| s.activate(i, false, w, cx)))
                    .on_key_down(cx.listener(
                        move |s, event: &KeyDownEvent, w, cx| s.trigger_key(i, event, w, cx),
                    )),
                );
            }
            bar = bar.child(tabs);
        }
        if self.search.is_some() {
            bar = bar.child(
                nav_face(
                    "morph-nav-search",
                    tr(cx, "Search", "搜索"),
                    height,
                    Tone::Clear,
                    cx,
                )
                .w(px(height))
                .track_focus(&self.search_focus)
                .child(icon(Glyph::Search, 18., p.ink))
                .on_click(cx.listener(|s, _, w, cx| s.show_search(w, cx)))
                .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                    if activation(e, w, cx) {
                        s.show_search(w, cx);
                    }
                })),
            );
        }
        if let Some(link) = &self.action {
            bar = bar.child(
                nav_face(
                    "morph-nav-action",
                    link.label.clone(),
                    height,
                    Tone::Primary,
                    cx,
                )
                .px(px(16.))
                .child(link.label.clone())
                .children(link.icon.map(|mark| icon(mark, 14., p.inverse)))
                .on_click(cx.listener(|s, _, _, cx| {
                    if let Some(link) = s.action.clone() {
                        s.emit(link, None, cx);
                    }
                }))
                .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                    if activation(e, w, cx)
                        && let Some(link) = s.action.clone()
                    {
                        s.emit(link, None, cx);
                    }
                })),
            );
        }
        bar
    }

    fn search_bar(&self, search: &NavSearch, height: f32, cx: &Context<Self>) -> Div {
        let p = Paint::get(cx);
        row()
            .w_full()
            .gap(px(10.))
            .px(px(12.))
            .child(icon(Glyph::Search, 20., p.secondary))
            .child(
                row()
                    .flex_1()
                    .h(px(height))
                    .text_size(px(16.))
                    .child(search.editor.clone()),
            )
            .child(
                nav_face(
                    "morph-nav-search-close",
                    tr(cx, "Close search", "关闭搜索"),
                    height,
                    Tone::Clear,
                    cx,
                )
                .w(px(height))
                .child(icon(Glyph::Close, 18., p.ink))
                .on_click(cx.listener(|s, _, w, cx| s.restore_focus(w, cx)))
                .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                    if activation(e, w, cx) {
                        s.restore_focus(w, cx);
                    }
                })),
            )
    }

    fn panel_link(
        &self,
        section: usize,
        i: usize,
        link: &MorphNavLink,
        style: LinkStyle,
        active: bool,
        cx: &Context<Self>,
    ) -> Stateful<Div> {
        let p = Paint::get(cx);
        let detail = matches!(style, LinkStyle::Detailed | LinkStyle::Feature);
        let height = match style {
            LinkStyle::Detailed => 74.,
            LinkStyle::Compact => 36.,
            LinkStyle::Feature => 128.,
            LinkStyle::Footer => 34.,
        };
        let label = if detail && !link.description.is_empty() {
            format!("{} {}", link.label, link.description).into()
        } else {
            link.label.clone()
        };
        let art = if style == LinkStyle::Feature {
            self.items[section].feature_art.clone()
        } else {
            None
        };
        let mut button = nav_face(
            ("morph-nav-link", section * 100_000 + i),
            label,
            height,
            Tone::Clear,
            cx,
        )
        .min_w_0()
        .px(px(12.))
        .justify_start()
        .when(style == LinkStyle::Feature, |e| {
            e.h_full().min_h(px(height)).bg(p.hover)
        })
        .when(style == LinkStyle::Footer, |e| {
            e.text_size(px(12.)).text_color(p.secondary)
        })
        .when(art.is_some(), |e| e.flex_col().items_start().py(px(8.)))
        .when(art.is_none(), |e| {
            e.children(link.icon.map(|mark| icon(mark, 16., p.secondary)))
        })
        .children(art)
        .child(
            column()
                .min_w_0()
                .flex_1()
                .overflow_hidden()
                .child(div().truncate().child(link.label.clone()))
                .when(detail && !link.description.is_empty(), |col| {
                    col.child(
                        div()
                            .whitespace_normal()
                            .text_size(px(12.))
                            .line_height(px(17.))
                            .text_color(p.secondary)
                            .child(link.description.clone()),
                    )
                }),
        );
        if active {
            button = button
                .track_focus(&self.link_focus[section][i])
                .on_click(cx.listener(move |s, _, w, cx| s.choose_link(section, i, w, cx)))
                .on_key_down(cx.listener(move |s, event: &KeyDownEvent, w, cx| {
                    s.link_key(section, i, event, w, cx)
                }));
        } else {
            button = button.tab_stop(false);
        }
        button
    }

    fn panel_body(
        &self,
        panel: Panel,
        width: Pixels,
        active: bool,
        w: &Window,
        cx: &Context<Self>,
    ) -> Div {
        let p = Paint::get(cx);
        let mut body = column()
            .w_full()
            .flex_none()
            .px(px(16.))
            .pt(px(8.))
            .pb(px(14.));
        match panel {
            Panel::Section(section) => {
                let item = &self.items[section];
                let two = w.viewport_size().width >= px(420.)
                    && width >= px(400.)
                    && match item.columns {
                        MorphNavColumns::Automatic => item.links.len() > 3,
                        MorphNavColumns::One => false,
                        MorphNavColumns::Two => true,
                    };
                let columns = usize::from(two) + 1;
                let mut links = column().flex_1().gap(px(2.));
                for (r, chunk) in item.links.chunks(columns).enumerate() {
                    links = links.child(
                        row()
                            .w_full()
                            .gap(px(2.))
                            .children(chunk.iter().enumerate().map(|(i, link)| {
                                self.panel_link(
                                    section,
                                    r * columns + i,
                                    link,
                                    LinkStyle::Detailed,
                                    active,
                                    cx,
                                )
                                .flex_1()
                            }))
                            .when(chunk.len() == 1 && two, |line| line.child(div().flex_1())),
                    );
                }
                let mut grid = row().w_full().items_stretch().gap(px(8.)).child(links);
                if let Some(feature) = &item.feature
                    && width >= px(540.)
                {
                    grid = grid.child(
                        self.panel_link(
                            section,
                            item.links.len(),
                            feature,
                            LinkStyle::Feature,
                            active,
                            cx,
                        )
                        .w(width * 0.30),
                    );
                }
                let label = item.destination.label.clone();
                let mut region = column()
                    .id("morph-nav-section")
                    .role(Role::Region)
                    .aria_label(label)
                    .child(grid);
                if let Some(feature) = &item.feature
                    && width < px(540.)
                {
                    region = region.child(
                        self.panel_link(
                            section,
                            item.links.len(),
                            feature,
                            LinkStyle::Feature,
                            active,
                            cx,
                        )
                        .w_full(),
                    );
                }
                if let Some(footer) = &item.footer {
                    let i = item.links.len() + usize::from(item.feature.is_some());
                    region =
                        region.child(div().mt(px(6.)).border_t_1().border_color(p.line).child(
                            self.panel_link(section, i, footer, LinkStyle::Footer, active, cx),
                        ));
                }
                body = body.child(region);
            }
            Panel::Menu => {
                let two = w.viewport_size().width >= px(420.) && width >= px(400.);
                let groups = self
                    .items
                    .iter()
                    .enumerate()
                    .filter(|(_, item)| item.has_panel())
                    .map(|(section, item)| {
                        column()
                            .gap(px(2.))
                            .min_w_0()
                            .child(
                                div()
                                    .px(px(12.))
                                    .py(px(4.))
                                    .text_size(px(12.))
                                    .text_color(p.secondary)
                                    .child(item.destination.label.clone()),
                            )
                            .children(item.links.iter().enumerate().map(|(i, link)| {
                                self.panel_link(section, i, link, LinkStyle::Compact, active, cx)
                            }))
                            .when(item.links.is_empty(), |group| {
                                group.children(item.feature.as_ref().or(item.footer.as_ref()).map(
                                    |feature| {
                                        self.panel_link(
                                            section,
                                            0,
                                            feature,
                                            LinkStyle::Compact,
                                            active,
                                            cx,
                                        )
                                    },
                                ))
                            })
                    })
                    .collect::<Vec<_>>();
                let plain = self
                    .items
                    .iter()
                    .enumerate()
                    .filter(|(_, item)| !item.has_panel())
                    .map(|(i, item)| {
                        self.panel_link(i, 0, &item.destination, LinkStyle::Compact, active, cx)
                    });
                if two {
                    let mut a = column().flex_1().gap(px(10.));
                    let mut b = column().flex_1().gap(px(10.));
                    for (i, group) in groups.into_iter().enumerate() {
                        if i % 2 == 0 {
                            a = a.child(group);
                        } else {
                            b = b.child(group);
                        }
                    }
                    body = body.child(row().items_start().gap(px(8.)).child(a).child(b));
                } else {
                    body = body.gap(px(8.)).children(groups);
                }
                body = body.children(plain);
            }
            Panel::Search => {
                let Some(search) = &self.search else {
                    return body;
                };
                let header = if search.query.is_empty() {
                    tr(cx, "Suggested", "建议")
                } else {
                    tr(cx, "Results", "搜索结果")
                };
                let count = search.results.len();
                body = body
                    .max_w(px(560.))
                    .mx_auto()
                    .gap(px(6.))
                    .child(
                        div()
                            .id("morph-nav-results-label")
                            .role(Role::Status)
                            .px(px(12.))
                            .text_size(px(12.))
                            .text_color(p.secondary)
                            .aria_label(if search.query.is_empty() {
                                header.to_owned()
                            } else {
                                format!("{header}: {count}")
                            })
                            .child(header),
                    )
                    .child(
                        column()
                            .id("morph-nav-results")
                            .role(Role::ListBox)
                            .aria_label(tr(cx, "Search results", "搜索结果"))
                            .gap(px(2.))
                            .when(count == 0, |list| {
                                list.child(div().p(px(12.)).text_color(p.secondary).child(tr(
                                    cx,
                                    "No matches",
                                    "没有匹配项",
                                )))
                            })
                            .children(search.results.iter().enumerate().map(|(i, idx)| {
                                let entry = &search.items[*idx];
                                let link = &entry.destination;
                                let selected = i == search.active;
                                let mut result = nav_face(
                                    ("morph-nav-result", i),
                                    link.label.clone(),
                                    54.,
                                    Tone::Clear,
                                    cx,
                                )
                                .role(Role::ListBoxOption)
                                .aria_selected(selected)
                                .tab_stop(false)
                                .px(px(12.))
                                .justify_start()
                                .when(selected, |e| e.bg(p.hover))
                                .children(link.icon.map(|mark| icon(mark, 16., p.secondary)))
                                .child(
                                    column()
                                        .flex_1()
                                        .min_w_0()
                                        .child(div().truncate().child(link.label.clone()))
                                        .when(!link.description.is_empty(), |col| {
                                            col.child(
                                                div()
                                                    .truncate()
                                                    .text_size(px(12.))
                                                    .text_color(p.secondary)
                                                    .child(link.description.clone()),
                                            )
                                        }),
                                )
                                .children(entry.group.clone().map(|group| {
                                    div()
                                        .flex_none()
                                        .text_size(px(12.))
                                        .text_color(p.secondary)
                                        .child(group)
                                }));
                                if active {
                                    result = result
                                        .on_mouse_down(MouseButton::Left, |_, w, _| {
                                            w.prevent_default()
                                        })
                                        .on_click(cx.listener(move |s, _, w, cx| {
                                            s.choose_result(i, w, cx)
                                        }));
                                }
                                result
                            })),
                    );
            }
        }
        body
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LinkStyle {
    Detailed,
    Compact,
    Feature,
    Footer,
}

impl Render for MorphNav {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let width = self.container.get().size.width;
        let width = if width == px(0.) {
            w.viewport_size().width
        } else {
            width
        };
        let narrow = self.narrow(w);
        let density = self.density(w, cx);
        let height = 36. - 4. * density;
        let bar_height = 56. - 10. * density;
        let search = self.panel == Some(Panel::Search);
        let reduced = disabled_motion(cx);
        let goal = if self.panel.is_some() {
            f32::from(width)
        } else {
            self.top_width(width, narrow, density, w, cx)
        };
        let width_slide = self.shell_width.get_or_insert_with(|| Slide::new(goal));
        width_slide.duration(goal, 0.27, cx);
        let width_slide = *width_slide;
        let shell_width = self.sample(&width_slide, w, cx).min(f32::from(width));
        let measured_height = self
            .measured
            .get()
            .filter(|(panel, _)| Some(*panel) == self.panel)
            .map(|(_, h)| f32::from(h));
        if let Some(target) = measured_height {
            self.panel_height.duration(
                target,
                if target > self.panel_height.value(cx) {
                    0.27
                } else {
                    0.22
                },
                cx,
            );
        }
        let panel_height = self.sample(&self.panel_height, w, cx).max(0.);
        let anim = if self.visible && w.is_visible() {
            self.panel_clock.time(0.18, w, cx)
        } else {
            self.panel_clock.age(cx)
        };
        let enter = ease(anim / 0.18);
        let alpha = if self.panel.is_some() {
            enter
        } else {
            1. - enter
        };
        if self.panel.is_none() && panel_height < 0.02 && alpha == 0. {
            self.painted = None;
        }
        let bar = if search && let Some(search) = &self.search {
            self.search_bar(search, height, cx).opacity(enter)
        } else {
            self.top_bar(narrow, density, w, cx)
                .opacity(if self.painted == Some(Panel::Search) {
                    enter
                } else {
                    1.
                })
        };
        let mut shell = column()
            .id("morph-nav-surface")
            .debug_selector(|| "morph-nav-surface".into())
            .relative()
            .w(px(shell_width.max(0.)))
            .max_w_full()
            .rounded(surface_radius(cx))
            .border_1()
            .border_color(p.line)
            .bg(p.panel.opacity(0.88))
            .shadow_md()
            .text_color(p.ink)
            .on_mouse_down_out(cx.listener(|s, _, _, cx| s.close(cx)))
            .child(self.surface.measure())
            .child(row().w_full().h(px(bar_height - 2.)).px(px(8.)).child(bar));
        if let Some(panel) = self.painted {
            let active = self.panel == Some(panel);
            let measured = self.measured.clone();
            let mut inner = column()
                .relative()
                .w(width.min(px(shell_width)).max(px(0.)))
                .max_w_full()
                .flex_none()
                .child(self.panel_body(panel, width, active, w, cx))
                .on_children_prepainted(move |children, w, _| {
                    if let Some(bounds) = children.first() {
                        let next = Some((panel, bounds.size.height));
                        if measured.replace(next) != next {
                            w.request_animation_frame();
                        }
                    }
                });
            if active && !reduced {
                inner = inner.left(px(self.enter_side * 16. * (1. - enter)));
            }
            shell = shell.child(
                div()
                    .id("morph-nav-panel")
                    .debug_selector(|| "morph-nav-panel".into())
                    .relative()
                    .w_full()
                    .h(px(panel_height))
                    .overflow_hidden()
                    .opacity(alpha)
                    .child(inner),
            );
        }
        let bounds = self.container.clone();
        let inside = self.pointer_inside.clone();
        let surface = self.surface.clone();
        let pointer = cx.listener(|s, entered: &bool, _, cx| {
            if *entered {
                if s.hovered.is_none() {
                    s.hover_task = None;
                }
            } else {
                s.leave(cx);
            }
        });
        let touch = cx.listener(|s, _: &TouchDragEvent, _, cx| {
            s.last_touch.restart(cx);
            s.hover_task = None;
            if s.hovered.take().is_some() {
                cx.notify();
            }
        });
        column()
            .id("morph-nav-root")
            .role(Role::Navigation)
            .aria_label(
                self.label
                    .clone()
                    .unwrap_or_else(|| tr(cx, "Main", "主导航").into()),
            )
            .track_focus(&self.root_focus)
            .relative()
            .w_full()
            .items_center()
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                if e.keystroke.key == "escape" && s.panel.is_some() {
                    s.restore_focus(w, cx);
                    cx.stop_propagation();
                }
            }))
            .child(
                canvas(
                    move |measured, w, _| {
                        if bounds.replace(measured).size.width != measured.size.width {
                            w.request_animation_frame();
                        }
                        measured
                    },
                    move |_, _, w, _| {
                        let touch_surface = surface.clone();
                        w.on_mouse_event(move |event: &MouseMoveEvent, phase, w, cx| {
                            if phase.bubble() {
                                let entered = surface.get().contains(&event.position);
                                if inside.replace(entered) != entered {
                                    pointer(&entered, w, cx);
                                }
                            }
                        });
                        w.on_mouse_event(move |event: &TouchDragEvent, phase, w, cx| {
                            if phase.bubble()
                                && event.phase == TouchPhase::Started
                                && touch_surface.get().contains(&event.start_position)
                            {
                                touch(event, w, cx);
                            }
                        });
                    },
                )
                .absolute()
                .inset_0(),
            )
            .child(shell)
    }
}

fn nav_face(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    height: f32,
    tone: Tone,
    cx: &App,
) -> Stateful<Div> {
    let p = Paint::get(cx);
    nav_surface(id, label, height, tone, cx).hover(move |s| {
        s.bg(if tone == Tone::Primary {
            p.primary_hover(0.12)
        } else {
            p.hover
        })
    })
}

fn nav_surface(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    height: f32,
    tone: Tone,
    cx: &App,
) -> Stateful<Div> {
    face(id, label, height, surface_radius(cx).into(), tone, cx)
        .relative()
        .border_1()
        .border_color(transparent_black())
        .active(|s| s.opacity(0.76))
}

fn activation(event: &KeyDownEvent, w: &mut Window, cx: &mut App) -> bool {
    if !event.is_held && matches!(event.keystroke.key.as_str(), "enter" | "space") {
        cx.stop_propagation();
        w.prevent_default();
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::{
        MorphNav, MorphNavItem, MorphNavLink, MorphNavNavigate, MorphNavSearchItem, Panel,
    };
    use gpui::{TestAppContext, point, px, size};
    use std::{cell::RefCell, rc::Rc, time::Duration};

    fn items() -> Vec<MorphNavItem> {
        vec![
            MorphNavItem::new("product", "Product").with_links(vec![
                MorphNavLink::new("planning", "Planning").with_href("/planning"),
                MorphNavLink::new("rules", "Rules"),
            ]),
            MorphNavItem::new("resources", "Resources")
                .with_links(vec![MorphNavLink::new("help", "Help")]),
            MorphNavItem::new("pricing", "Pricing"),
        ]
    }

    #[test]
    fn keyboard_moves_between_sections_and_returns_from_link_focus() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) = app.add_window_view(|w, cx| MorphNav::new("KUI", items(), w, cx));
        w.simulate_resize(size(px(840.), px(520.)));
        w.update(|w, cx| view.read(cx).trigger_focus[0].clone().focus(w, cx));
        w.simulate_keystrokes("down");
        w.update(|w, cx| {
            assert_eq!(view.read(cx).panel, Some(Panel::Section(0)));
            assert!(view.read(cx).link_focus[0][0].is_focused(w));
        });
        w.simulate_keystrokes("up right");
        w.update(|w, cx| {
            assert_eq!(view.read(cx).panel, Some(Panel::Section(1)));
            assert!(view.read(cx).trigger_focus[1].is_focused(w));
        });
        w.simulate_keystrokes("down escape");
        w.update(|w, cx| {
            assert_eq!(view.read(cx).panel, None);
            assert!(view.read(cx).trigger_focus[1].is_focused(w));
        });
    }

    #[test]
    fn compact_menu_exposes_grouped_links_and_routes_the_selected_href() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let events = Rc::new(RefCell::new(Vec::<MorphNavNavigate>::new()));
        let (view, w) = app.add_window_view(|w, cx| MorphNav::new("KUI", items(), w, cx));
        let _sub = w.update(|_, cx| {
            let events = events.clone();
            cx.subscribe(&view, move |_, event: &MorphNavNavigate, _| {
                events.borrow_mut().push(event.clone())
            })
        });
        w.simulate_resize(size(px(390.), px(600.)));
        w.update(|w, cx| view.read(cx).menu_focus.clone().focus(w, cx));
        w.simulate_keystrokes("enter");
        w.update(|w, cx| {
            assert_eq!(view.read(cx).panel, Some(Panel::Menu));
            assert!(view.read(cx).link_focus[0][0].is_focused(w));
        });
        w.simulate_keystrokes("enter");
        assert_eq!(events.borrow()[0].href.as_deref(), Some("/planning"));
        assert_eq!(events.borrow()[0].section.as_deref(), Some("product"));
        w.update(|w, cx| assert!(view.read(cx).menu_focus.is_focused(w)));
    }

    #[test]
    fn search_keeps_a_five_entry_suggested_list_and_uses_its_own_index() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let events = Rc::new(RefCell::new(Vec::<MorphNavNavigate>::new()));
        let (view, w) = app.add_window_view(|w, cx| {
            let mut entries = (0..8)
                .map(|i| {
                    MorphNavSearchItem::new(MorphNavLink::new(
                        format!("entry-{i}"),
                        format!("Guide {i}"),
                    ))
                })
                .collect::<Vec<_>>();
            entries.push(
                MorphNavSearchItem::new(
                    MorphNavLink::new("independent", "Handbook").with_href("/handbook"),
                )
                .with_keywords(vec!["Policy".into()]),
            );
            MorphNav::new("KUI", items(), w, cx).with_search("Find a page", entries, w, cx)
        });
        let _sub = w.update(|_, cx| {
            let events = events.clone();
            cx.subscribe(&view, move |_, event: &MorphNavNavigate, _| {
                events.borrow_mut().push(event.clone())
            })
        });
        w.update(|w, cx| view.read(cx).search_focus.clone().focus(w, cx));
        w.simulate_keystrokes("enter");
        let editor = w.update(|_, cx| {
            let nav = view.read(cx);
            assert_eq!(nav.panel, Some(Panel::Search));
            assert_eq!(nav.search.as_ref().unwrap().results.len(), 5);
            nav.search.as_ref().unwrap().editor.clone()
        });
        w.update(|w, cx| editor.update(cx, |e, cx| e.set_value("guide", w, cx)));
        w.update(|_, cx| assert_eq!(view.read(cx).search.as_ref().unwrap().results.len(), 6));
        w.update(|w, cx| editor.update(cx, |e, cx| e.set_value("POLICY", w, cx)));
        w.simulate_keystrokes("enter");
        assert_eq!(events.borrow()[0].href.as_deref(), Some("/handbook"));
        w.update(|w, cx| assert!(view.read(cx).search_focus.is_focused(w)));
    }

    #[test]
    fn mouse_intent_waits_and_the_surface_shrinks_after_exit() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) = app.add_window_view(|w, cx| MorphNav::new("KUI", items(), w, cx));
        w.simulate_resize(size(px(840.), px(600.)));
        w.run_until_parked();
        let product = w.debug_bounds("Product").unwrap().center();
        w.update(|w, cx| w.simulate_mouse_move(product, cx));
        w.run_until_parked();
        w.executor().advance_clock(Duration::from_millis(59));
        w.update(|_, cx| assert_eq!(view.read(cx).panel, None));
        w.executor().advance_clock(Duration::from_millis(1));
        w.run_until_parked();
        w.update(|_, cx| assert_eq!(view.read(cx).panel, Some(Panel::Section(0))));
        for _ in 0..22 {
            w.executor().advance_clock(Duration::from_millis(16));
            w.update(|w, cx| w.simulate_next_frame(cx));
            w.run_until_parked();
        }
        let open_height = w.debug_bounds("morph-nav-panel").unwrap().size.height;
        assert!(open_height > px(74.));
        w.update(|w, cx| w.simulate_mouse_move(point(px(0.), px(570.)), cx));
        w.run_until_parked();
        w.executor().advance_clock(Duration::from_millis(179));
        w.update(|_, cx| assert_eq!(view.read(cx).panel, Some(Panel::Section(0))));
        w.executor().advance_clock(Duration::from_millis(5));
        w.run_until_parked();
        w.update(|_, cx| assert_eq!(view.read(cx).panel, None));
        w.executor().advance_clock(Duration::from_millis(64));
        w.update(|w, cx| {
            w.simulate_next_frame(cx);
            w.refresh();
        });
        w.run_until_parked();
        let during_close = w.debug_bounds("morph-nav-panel").unwrap().size.height;
        let sampled = w.update(|_, cx| view.read(cx).panel_height.value(cx));
        assert!(
            during_close > px(0.) && during_close < open_height,
            "{open_height:?} -> {during_close:?}, sampled {sampled}"
        );
        w.update(|_, cx| assert_eq!(view.read(cx).panel, None));
    }
}
