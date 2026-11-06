//! Responsive navigation with a shared, animated menu surface, gliding caret and safe triangle hover intent.
use crate::primitives::{
    family::{Paint, PhaseClock, Slide, Tone, ease, face, measure},
    form_chrome::tr,
    glyph::{Glyph, icon},
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc, time::Duration};

#[derive(Clone, Debug)]
pub struct MegaMenuLink {
    pub id: SharedString,
    pub label: SharedString,
    pub description: SharedString,
}
impl MegaMenuLink {
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            description: "".into(),
        }
    }
    pub fn with_description(mut self, text: impl Into<SharedString>) -> Self {
        self.description = text.into();
        self
    }
}
#[derive(Clone, Debug)]
pub struct MegaMenuGroup {
    pub title: SharedString,
    pub links: Vec<MegaMenuLink>,
}
impl MegaMenuGroup {
    pub fn new(title: impl Into<SharedString>, links: Vec<MegaMenuLink>) -> Self {
        Self {
            title: title.into(),
            links,
        }
    }
}
#[derive(Clone, Debug)]
pub struct MegaMenuFeature {
    pub id: SharedString,
    pub title: SharedString,
    pub description: SharedString,
    pub action: SharedString,
}
impl MegaMenuFeature {
    pub fn new(
        id: impl Into<SharedString>,
        title: impl Into<SharedString>,
        action: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            description: "".into(),
            action: action.into(),
        }
    }
    pub fn with_description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = description.into();
        self
    }
}
#[derive(Clone, Debug)]
pub struct MegaMenuSection {
    pub id: SharedString,
    pub label: SharedString,
    pub groups: Vec<MegaMenuGroup>,
    pub feature: Option<MegaMenuFeature>,
}
impl MegaMenuSection {
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            groups: Vec::new(),
            feature: None,
        }
    }
    pub fn with_groups(mut self, groups: Vec<MegaMenuGroup>) -> Self {
        self.groups = groups;
        self
    }
    pub fn with_feature(mut self, feature: MegaMenuFeature) -> Self {
        self.feature = Some(feature);
        self
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MegaMenuNavigate(pub SharedString);
/// A navigation surface with hover intent, keyboard operation and a compact sheet.
pub struct MegaMenuMorph {
    brand: SharedString,
    brand_id: SharedString,
    primary: Option<(SharedString, SharedString)>,
    sections: Vec<MegaMenuSection>,
    open: Option<usize>,
    previous: Option<usize>,
    compact_open: bool,
    focus: Vec<FocusHandle>,
    menu_focus: FocusHandle,
    panel_focus: Vec<Vec<FocusHandle>>,
    presence: Slide,
    width: Slide,
    height: Slide,
    position: Slide,
    caret: Slide,
    switch: PhaseClock,
    direction: f32,
    viewport_width: Pixels,
    hover_epoch: u64,
    panel_hovered: bool,
    pointer: Point<Pixels>,
    previous_pointer: Point<Pixels>,
    panel_bounds: Rc<Cell<Bounds<Pixels>>>,
    root_bounds: Rc<Cell<Bounds<Pixels>>>,
    trigger_bounds: Vec<Rc<Cell<Bounds<Pixels>>>>,
    root_focus: FocusHandle,
    focused: bool,
    focus_subscriptions: Vec<Subscription>,
}
impl EventEmitter<MegaMenuNavigate> for MegaMenuMorph {}
impl MegaMenuMorph {
    pub fn new(
        brand: impl Into<SharedString>,
        sections: Vec<MegaMenuSection>,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus = (0..sections.len())
            .map(|_| cx.focus_handle().tab_stop(true))
            .collect();
        let trigger_bounds = (0..sections.len())
            .map(|_| Rc::new(Cell::new(Bounds::default())))
            .collect();
        let panel_focus = sections
            .iter()
            .map(|section| {
                let mut links: Vec<_> = section
                    .groups
                    .iter()
                    .flat_map(|group| group.links.iter())
                    .map(|_| cx.focus_handle().tab_stop(true))
                    .collect();
                if section.feature.is_some() {
                    links.push(cx.focus_handle().tab_stop(true));
                }
                links
            })
            .collect();
        Self {
            brand: brand.into(),
            brand_id: "brand".into(),
            primary: None,
            sections,
            open: None,
            previous: None,
            compact_open: false,
            focus,
            menu_focus: cx.focus_handle().tab_stop(true),
            panel_focus,
            presence: Slide::snappy(0.),
            width: Slide::snappy(340.),
            height: Slide::snappy(0.),
            position: Slide::snappy(0.),
            caret: Slide::snappy(0.),
            switch: Default::default(),
            direction: 1.,
            viewport_width: px(0.),
            hover_epoch: 0,
            panel_hovered: false,
            pointer: Point::default(),
            previous_pointer: Point::default(),
            panel_bounds: Default::default(),
            root_bounds: Default::default(),
            trigger_bounds,
            root_focus: cx.focus_handle(),
            focused: false,
            focus_subscriptions: Vec::new(),
        }
    }
    pub fn with_brand_id(mut self, id: impl Into<SharedString>) -> Self {
        self.brand_id = id.into();
        self
    }
    pub fn with_primary(
        mut self,
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
    ) -> Self {
        self.primary = Some((id.into(), label.into()));
        self
    }
    fn open_section(&mut self, i: usize, cx: &mut Context<Self>) {
        if i >= self.sections.len() {
            return;
        }
        if self.sections[i].groups.is_empty() && self.sections[i].feature.is_none() {
            if self.open.is_some() {
                self.close(cx);
            }
            return;
        }
        if self.open != Some(i) {
            self.previous = self.open;
            self.direction = if self.open.is_some_and(|old| i < old) {
                -1.
            } else {
                1.
            };
            self.open = Some(i);
            self.switch.restart(cx);
            self.presence.duration(1., 0.24, cx);
        }
        cx.notify();
    }
    fn close(&mut self, cx: &mut Context<Self>) {
        self.hover_epoch = self.hover_epoch.wrapping_add(1);
        self.previous = self.open.or(self.previous);
        self.open = None;
        self.presence.duration(0., 0.18, cx);
        cx.notify();
    }
    fn hover_section(&mut self, i: usize, cx: &mut Context<Self>) {
        self.hover_epoch = self.hover_epoch.wrapping_add(1);
        let epoch = self.hover_epoch;
        if self.open.is_some() && !self.heading_to_panel() {
            self.open_section(i, cx);
            return;
        }
        let delay = if self.open.is_some() { 180 } else { 90 };
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(delay))
                .await;
            let _ = this.update(cx, |s, cx| {
                if s.hover_epoch == epoch {
                    s.open_section(i, cx)
                }
            });
        })
        .detach();
    }
    fn heading_to_panel(&self) -> bool {
        let b = self.panel_bounds.get();
        let start = self.previous_pointer;
        let now = self.pointer;
        if now.y <= start.y || b.top() <= start.y || b.size.width <= px(0.) {
            return false;
        }
        let t = ((now.y - start.y) / (b.top() - start.y)).clamp(0., 1.);
        let left = start.x + (b.left() - start.x) * t - px(8.);
        let right = start.x + (b.right() - start.x) * t + px(8.);
        now.x >= left && now.x <= right
    }
    fn schedule_close(&mut self, cx: &mut Context<Self>) {
        self.hover_epoch = self.hover_epoch.wrapping_add(1);
        let epoch = self.hover_epoch;
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(240))
                .await;
            let _ = this.update(cx, |s, cx| {
                if s.hover_epoch == epoch && !s.panel_hovered && !s.focused {
                    s.close(cx)
                }
            });
        })
        .detach();
    }
    fn section_click(&mut self, i: usize, w: &mut Window, cx: &mut Context<Self>) {
        if self.sections[i].groups.is_empty() && self.sections[i].feature.is_none() {
            cx.emit(MegaMenuNavigate(self.sections[i].id.clone()));
            self.compact_open = false;
            self.close(cx);
        } else if self.open == Some(i) {
            self.close(cx)
        } else {
            self.open_section(i, cx);
        }
        self.focus[i].focus(w, cx);
    }
    fn trigger_key(
        &mut self,
        i: usize,
        event: &KeyDownEvent,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.sections.is_empty() {
            return;
        }
        let next = match event.keystroke.key.as_str() {
            "enter" | "space" => {
                self.section_click(i, w, cx);
                None
            }
            "left" => Some((i + self.sections.len() - 1) % self.sections.len()),
            "right" => Some((i + 1) % self.sections.len()),
            "home" => Some(0),
            "end" => Some(self.sections.len() - 1),
            "down" => {
                self.open_section(i, cx);
                if let Some(focus) = self.panel_focus[i].first() {
                    focus.focus(w, cx)
                }
                None
            }
            "escape" => {
                self.close(cx);
                if self.compact_open {
                    self.compact_open = false;
                    self.menu_focus.focus(w, cx);
                } else {
                    self.focus[i].focus(w, cx);
                }
                None
            }
            _ => return,
        };
        if let Some(j) = next {
            self.focus[j].focus(w, cx);
            if self.open.is_some() {
                self.open_section(j, cx);
            }
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
        let list = &self.panel_focus[section];
        if list.is_empty() {
            return;
        }
        match event.keystroke.key.as_str() {
            "enter" | "space" => {
                let id = self.sections[section]
                    .groups
                    .iter()
                    .flat_map(|group| group.links.iter())
                    .nth(index)
                    .map(|link| link.id.clone())
                    .or_else(|| {
                        let count = self.sections[section]
                            .groups
                            .iter()
                            .map(|group| group.links.len())
                            .sum::<usize>();
                        (index == count)
                            .then(|| {
                                self.sections[section]
                                    .feature
                                    .as_ref()
                                    .map(|feature| feature.id.clone())
                            })
                            .flatten()
                    });
                if let Some(id) = id {
                    cx.emit(MegaMenuNavigate(id));
                    self.close(cx);
                }
            }
            "down" => list[(index + 1).min(list.len() - 1)].focus(w, cx),
            "up" => {
                if index == 0 {
                    self.focus[section].focus(w, cx)
                } else {
                    list[index - 1].focus(w, cx)
                }
            }
            "escape" => {
                self.close(cx);
                self.focus[section].focus(w, cx)
            }
            "left" | "right" => {
                let next = if event.keystroke.key == "left" {
                    (section + self.sections.len() - 1) % self.sections.len()
                } else {
                    (section + 1) % self.sections.len()
                };
                self.open_section(next, cx);
                if let Some(f) = self.panel_focus[next].first() {
                    f.focus(w, cx)
                } else {
                    self.focus[next].focus(w, cx)
                }
            }
            _ => return,
        };
        cx.stop_propagation();
        w.prevent_default();
    }
}
impl Render for MegaMenuMorph {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.focus_subscriptions.is_empty() {
            self.focus_subscriptions
                .push(cx.on_focus_in(&self.root_focus, w, |s, _, _| s.focused = true));
            self.focus_subscriptions
                .push(cx.on_focus_out(&self.root_focus, w, |s, _, _, cx| {
                    s.focused = false;
                    if s.open.is_some() {
                        s.schedule_close(cx);
                    }
                }));
        }
        let p = Paint::get(cx);
        let dark = crate::foundation::Theme::global(cx).mode == crate::foundation::ThemeMode::Dark;
        let radius = crate::foundation::surface_radius(cx);
        let viewport = if self.viewport_width > px(0.) {
            f32::from(self.viewport_width)
        } else {
            f32::from(w.viewport_size().width)
        };
        let natural = measure(&self.brand, 15., w)
            + 40.
            + self
                .sections
                .iter()
                .map(|section| measure(&section.label, 14., w) + 34.)
                .sum::<f32>()
            + self
                .primary
                .as_ref()
                .map_or(0., |(_, label)| measure(label, 14., w) + 30.);
        let compact = viewport < natural + 24.;
        let visible = self.presence.get(w, cx).clamp(0., 1.);
        let index = self
            .open
            .or(self.previous)
            .filter(|&i| i < self.sections.len());
        let brand_id = self.brand_id.clone();
        let key_brand_id = brand_id.clone();
        let mut bar = div()
            .flex()
            .items_center()
            .justify_between()
            .gap(px(12.))
            .h(px(60.))
            .w_full()
            .pl(px(16.))
            .pr(px(12.))
            .border_b_1()
            .border_color(p.line)
            .bg(p.panel)
            .rounded_t(radius)
            .child(
                face(
                    "mega-brand",
                    self.brand.clone(),
                    36.,
                    radius.into(),
                    Tone::Clear,
                    cx,
                )
                .py(px(6.))
                .px(px(4.))
                .mr(px(8.))
                .text_size(px(16.))
                .font_weight(FontWeight::MEDIUM)
                .child(self.brand.clone())
                .on_click(cx.listener(move |s, _, _, cx| {
                    cx.emit(MegaMenuNavigate(brand_id.clone()));
                    s.close(cx);
                }))
                .on_key_down(cx.listener(move |s, e: &KeyDownEvent, w, cx| {
                    if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                        cx.emit(MegaMenuNavigate(key_brand_id.clone()));
                        s.close(cx);
                        cx.stop_propagation();
                        w.prevent_default();
                    }
                })),
            );
        if compact {
            let mut actions = div().flex().items_center().gap(px(4.));
            if let Some((id, label)) = &self.primary {
                let id = id.clone();
                let key_id = id.clone();
                actions = actions.child(
                    face(
                        "mega-primary",
                        label.clone(),
                        36.,
                        radius.into(),
                        Tone::Primary,
                        cx,
                    )
                    .px(px(14.))
                    .child(label.clone())
                    .on_click(cx.listener(move |s, _, _, cx| {
                        cx.emit(MegaMenuNavigate(id.clone()));
                        s.compact_open = false;
                        s.close(cx);
                    }))
                    .on_key_down(cx.listener(
                        move |s, e: &KeyDownEvent, w, cx| {
                            if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                                cx.emit(MegaMenuNavigate(key_id.clone()));
                                s.compact_open = false;
                                s.close(cx);
                                cx.stop_propagation();
                                w.prevent_default();
                            }
                        },
                    )),
                );
            }
            let label = if self.compact_open {
                tr(cx, "Close menu", "关闭菜单")
            } else {
                tr(cx, "Open menu", "打开菜单")
            };
            actions = actions.child(
                face("mega-toggle", label, 36., radius.into(), Tone::Clear, cx)
                    .size(px(36.))
                    .track_focus(&self.menu_focus)
                    .aria_expanded(self.compact_open)
                    .child(icon(
                        if self.compact_open {
                            Glyph::Close
                        } else {
                            Glyph::Menu
                        },
                        16.,
                        p.ink,
                    ))
                    .on_click(cx.listener(|s, _, _, cx| {
                        s.compact_open = !s.compact_open;
                        if !s.compact_open {
                            s.close(cx)
                        }
                        cx.notify()
                    }))
                    .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                        if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                            s.compact_open = !s.compact_open;
                            if !s.compact_open {
                                s.close(cx);
                            }
                            cx.notify();
                            cx.stop_propagation();
                            w.prevent_default();
                        } else if e.keystroke.key == "escape" && s.compact_open {
                            s.compact_open = false;
                            s.close(cx);
                            s.menu_focus.focus(w, cx);
                            cx.stop_propagation();
                            w.prevent_default();
                        }
                    })),
            );
            bar = bar.child(actions);
        } else {
            let mut triggers = div().flex().items_center().gap(px(2.));
            for (i, section) in self.sections.iter().enumerate() {
                let has_menu = !section.groups.is_empty() || section.feature.is_some();
                let bounds = self.trigger_bounds[i].clone();
                let is_open = self.open == Some(i);
                triggers = triggers.child(
                    face(
                        ("mega-section", i),
                        section.label.clone(),
                        36.,
                        999.,
                        Tone::Clear,
                        cx,
                    )
                    .relative()
                    .px(px(12.))
                    .gap(px(4.))
                    .when(has_menu, |d| d.pr(px(9.)))
                    .track_focus(&self.focus[i])
                    .aria_expanded(is_open)
                    .text_color(if is_open { p.ink } else { p.secondary })
                    .when(is_open, |d| d.bg(p.ink.opacity(0.05)))
                    .hover(move |s| s.bg(p.ink.opacity(0.05)).text_color(p.ink))
                    .child(section.label.clone())
                    .child(
                        canvas(move |b, _, _| bounds.set(b), |_, _, _, _| ())
                            .absolute()
                            .inset_0(),
                    )
                    .when(has_menu, |d| {
                        d.child(icon(
                            if is_open { Glyph::Up } else { Glyph::Down },
                            13.,
                            if is_open { p.ink } else { p.secondary },
                        ))
                    })
                    .on_hover(cx.listener(move |s, hover, _, cx| {
                        if *hover {
                            s.hover_section(i, cx)
                        } else {
                            s.schedule_close(cx)
                        }
                    }))
                    .on_click(cx.listener(move |s, _, w, cx| s.section_click(i, w, cx)))
                    .on_key_down(
                        cx.listener(move |s, e: &KeyDownEvent, w, cx| s.trigger_key(i, e, w, cx)),
                    ),
                );
            }
            bar = bar.child(triggers);
            if let Some((id, label)) = &self.primary {
                let id = id.clone();
                let key_id = id.clone();
                bar = bar.child(
                    face(
                        "mega-primary",
                        label.clone(),
                        34.,
                        radius.into(),
                        Tone::Primary,
                        cx,
                    )
                    .px(px(12.))
                    .child(label.clone())
                    .on_click(cx.listener(move |s, _, _, cx| {
                        cx.emit(MegaMenuNavigate(id.clone()));
                        s.close(cx);
                    }))
                    .on_key_down(cx.listener(
                        move |s, e: &KeyDownEvent, w, cx| {
                            if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                                cx.emit(MegaMenuNavigate(key_id.clone()));
                                s.close(cx);
                                cx.stop_propagation();
                                w.prevent_default();
                            }
                        },
                    )),
                );
            }
        }
        let weak = cx.weak_entity();
        let previous_width = self.viewport_width;
        let root_bounds = self.root_bounds.clone();
        let mut root = div()
            .id("mega-menu")
            .relative()
            .role(Role::Navigation)
            .aria_label(tr(cx, "Main navigation", "主导航"))
            .track_focus(&self.root_focus)
            .w(px(720.))
            .max_w_full()
            .text_color(p.ink)
            .on_mouse_move(cx.listener(|s, event: &MouseMoveEvent, _, _| {
                s.previous_pointer = s.pointer;
                s.pointer = event.position;
            }))
            .on_mouse_down_out(cx.listener(|s, _, _, cx| {
                if s.open.is_some() {
                    s.close(cx)
                }
                if s.compact_open {
                    s.compact_open = false;
                    cx.notify()
                }
            }))
            .child(
                canvas(
                    move |b, _, cx| {
                        root_bounds.set(b);
                        if (b.size.width - previous_width).abs() > px(0.1) {
                            let _ = weak.update(cx, |s, cx| {
                                s.viewport_width = b.size.width;
                                cx.notify()
                            });
                        }
                    },
                    |_, _, _, _| (),
                )
                .absolute()
                .inset_0(),
            )
            .child(bar);
        if compact && self.compact_open {
            let mut sheet = div()
                .id("mega-mobile-sheet")
                .flex()
                .flex_col()
                .w_full()
                .max_h(px(360.))
                .overflow_y_scroll()
                .pt(px(6.))
                .px(px(8.))
                .pb(px(12.))
                .rounded_b(radius)
                .bg(p.panel)
                .border_1()
                .border_color(p.line);
            for (i, section) in self.sections.iter().enumerate() {
                let has_children = !section.groups.is_empty() || section.feature.is_some();
                sheet = sheet.child(
                    face(
                        ("mega-mobile", i),
                        section.label.clone(),
                        48.,
                        radius.into(),
                        Tone::Clear,
                        cx,
                    )
                    .w_full()
                    .px(px(8.))
                    .text_size(px(16.))
                    .justify_between()
                    .track_focus(&self.focus[i])
                    .aria_expanded(self.open == Some(i))
                    .child(section.label.clone())
                    .when(has_children, |d| {
                        d.child(icon(
                            if self.open == Some(i) {
                                Glyph::Up
                            } else {
                                Glyph::Down
                            },
                            14.,
                            p.secondary,
                        ))
                    })
                    .on_click(cx.listener(move |s, _, w, cx| s.section_click(i, w, cx)))
                    .on_key_down(
                        cx.listener(move |s, e: &KeyDownEvent, w, cx| s.trigger_key(i, e, w, cx)),
                    ),
                );
                if self.open == Some(i) {
                    let mut link_index = 0;
                    for group in &section.groups {
                        sheet = sheet.child(
                            div()
                                .px(px(12.))
                                .py(px(6.))
                                .text_size(px(12.))
                                .text_color(p.faint)
                                .child(group.title.clone()),
                        );
                        for link in &group.links {
                            let id = link.id.clone();
                            let key_id = id.clone();
                            let focus = self.panel_focus[i][link_index].clone();
                            let position = link_index;
                            link_index += 1;
                            sheet =
                                sheet.child(
                                    face(
                                        (ElementId::Name("mobile-link".into()), id.clone()),
                                        link.label.clone(),
                                        38.,
                                        radius.into(),
                                        Tone::Clear,
                                        cx,
                                    )
                                    .w_full()
                                    .px(px(16.))
                                    .justify_start()
                                    .track_focus(&focus)
                                    .child(link.label.clone())
                                    .on_click(cx.listener(move |s, _, _, cx| {
                                        cx.emit(MegaMenuNavigate(id.clone()));
                                        s.compact_open = false;
                                        s.close(cx)
                                    }))
                                    .on_key_down(cx.listener(move |s, e: &KeyDownEvent, w, cx| {
                                        if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                                            cx.emit(MegaMenuNavigate(key_id.clone()));
                                            s.compact_open = false;
                                            s.close(cx);
                                            cx.stop_propagation();
                                            w.prevent_default();
                                        } else if e.keystroke.key == "escape" {
                                            s.compact_open = false;
                                            s.close(cx);
                                            s.menu_focus.focus(w, cx);
                                            cx.stop_propagation();
                                            w.prevent_default();
                                        } else {
                                            s.link_key(i, position, e, w, cx);
                                        }
                                    })),
                                );
                        }
                    }
                    if let Some(feature) = &section.feature {
                        let id = feature.id.clone();
                        let key_id = id.clone();
                        let focus = self.panel_focus[i][link_index].clone();
                        sheet = sheet.child(
                            face(
                                (ElementId::Name("mobile-feature".into()), id.clone()),
                                feature.title.clone(),
                                72.,
                                radius.into(),
                                Tone::Surface,
                                cx,
                            )
                            .w_full()
                            .h_auto()
                            .min_h(px(72.))
                            .p(px(10.))
                            .justify_start()
                            .border_1()
                            .border_color(p.line)
                            .bg(p.tray)
                            .track_focus(&focus)
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(4.))
                                    .child(feature.title.clone())
                                    .child(
                                        div()
                                            .text_size(px(11.))
                                            .text_color(p.secondary)
                                            .child(feature.description.clone()),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(4.))
                                            .text_size(px(12.))
                                            .child(feature.action.clone())
                                            .child(icon(Glyph::Arrow, 12., p.ink)),
                                    ),
                            )
                            .on_click(cx.listener(move |s, _, _, cx| {
                                cx.emit(MegaMenuNavigate(id.clone()));
                                s.compact_open = false;
                                s.close(cx);
                            }))
                            .on_key_down(cx.listener(
                                move |s, e: &KeyDownEvent, w, cx| {
                                    if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                                        cx.emit(MegaMenuNavigate(key_id.clone()));
                                        s.compact_open = false;
                                        s.close(cx);
                                        cx.stop_propagation();
                                        w.prevent_default();
                                    } else if e.keystroke.key == "escape" {
                                        s.compact_open = false;
                                        s.close(cx);
                                        s.menu_focus.focus(w, cx);
                                        cx.stop_propagation();
                                        w.prevent_default();
                                    }
                                },
                            )),
                        );
                    }
                }
            }
            root = root.child(sheet);
        }
        if !compact
            && visible > 0.
            && let Some(i) = index
        {
            let section = &self.sections[i];
            let count = section.groups.iter().map(|g| g.links.len()).sum::<usize>();
            let panel_h = (count as f32 * 50.
                + section.groups.len() as f32 * 26.
                + if section.feature.is_some() { 94. } else { 0. }
                + 18.)
                .clamp(64., 320.);
            self.height
                .duration(if self.open.is_some() { panel_h } else { 0. }, 0.25, cx);
            let desired_w: f32 = if section.feature.is_some() {
                360.
            } else {
                320.
            };
            let target_width = desired_w.min((viewport - 16.).max(0.));
            self.width.duration(target_width, 0.25, cx);
            let trigger = self.trigger_bounds[i].get();
            let root_bounds = self.root_bounds.get();
            let anchor = if trigger.size.width > px(0.) {
                f32::from(trigger.left() - root_bounds.left() + trigger.size.width * 0.5)
            } else {
                i as f32 * 92. + 100.
            };
            self.caret.duration(anchor, 0.25, cx);
            self.position.duration(
                (anchor - target_width * 0.5).clamp(8., (viewport - target_width - 8.).max(8.)),
                0.25,
                cx,
            );
            let h = self.height.get(w, cx);
            let width = self.width.get(w, cx);
            let left = self.position.get(w, cx);
            let caret_x = self.caret.get(w, cx);
            let phase = ease(self.switch.time(0.24, w, cx) / 0.24);
            let mut content = div()
                .flex()
                .flex_col()
                .gap(px(8.))
                .p(px(12.))
                .opacity(phase)
                .relative()
                .left(px((1. - phase) * self.direction * 18.));
            let mut n = 0;
            for group in &section.groups {
                content = content.child(
                    div()
                        .pt(px(6.))
                        .pb(px(4.))
                        .px(px(12.))
                        .flex()
                        .items_center()
                        .text_size(px(12.))
                        .text_color(p.faint)
                        .child(group.title.clone()),
                );
                for link in &group.links {
                    let id = link.id.clone();
                    let focus = self.panel_focus[i][n].clone();
                    let at = n;
                    n += 1;
                    content = content.child(
                        face(
                            (ElementId::Name("mega-link".into()), id.clone()),
                            link.label.clone(),
                            48.,
                            radius.into(),
                            Tone::Clear,
                            cx,
                        )
                        .w_full()
                        .h_auto()
                        .min_h(px(0.))
                        .px(px(12.))
                        .py(px(9.))
                        .items_start()
                        .justify_start()
                        .track_focus(&focus)
                        .hover(move |s| s.bg(p.ink.opacity(0.045)))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(1.))
                                .child(
                                    div()
                                        .font_weight(FontWeight::MEDIUM)
                                        .child(link.label.clone()),
                                )
                                .when(!link.description.is_empty(), |d| {
                                    d.child(
                                        div()
                                            .text_size(px(12.))
                                            .line_height(px(20.))
                                            .text_color(p.faint)
                                            .child(link.description.clone()),
                                    )
                                }),
                        )
                        .on_click(cx.listener(move |s, _, _, cx| {
                            cx.emit(MegaMenuNavigate(id.clone()));
                            s.close(cx)
                        }))
                        .on_key_down(cx.listener(
                            move |s, e: &KeyDownEvent, w, cx| s.link_key(i, at, e, w, cx),
                        )),
                    );
                }
            }
            if let Some(feature) = &section.feature {
                let id = feature.id.clone();
                let focus = self.panel_focus[i][n].clone();
                let at = n;
                content = content.child(
                    face(
                        (ElementId::Name("mega-feature".into()), id.clone()),
                        feature.title.clone(),
                        84.,
                        radius.into(),
                        Tone::Surface,
                        cx,
                    )
                    .w_full()
                    .h_auto()
                    .w(px(216.))
                    .min_h(px(0.))
                    .mt(px(4.))
                    .p(px(14.))
                    .justify_start()
                    .bg(p.tray)
                    .hover(move |s| s.bg(p.tray.blend(p.faint.opacity(0.3))))
                    .track_focus(&focus)
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(4.))
                            .child(
                                div()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(feature.title.clone()),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(p.secondary)
                                    .child(feature.description.clone()),
                            )
                            .child(
                                div()
                                    .pt(px(8.))
                                    .flex()
                                    .items_center()
                                    .gap(px(4.))
                                    .text_size(px(12.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(feature.action.clone())
                                    .child(icon(Glyph::Arrow, 12., p.ink)),
                            ),
                    )
                    .on_click(cx.listener(move |s, _, _, cx| {
                        cx.emit(MegaMenuNavigate(id.clone()));
                        s.close(cx)
                    }))
                    .on_key_down(
                        cx.listener(move |s, e: &KeyDownEvent, w, cx| s.link_key(i, at, e, w, cx)),
                    ),
                );
            }
            let mut surface = div().relative().child(content);
            if phase < 1.
                && self.open.is_some()
                && let Some(old) = self.previous.filter(|old| *old != i)
                && let Some(previous) = self.sections.get(old)
            {
                let mut leaving = div()
                    .absolute()
                    .top_0()
                    .left(px(-self.direction * 18. * phase))
                    .right_0()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .p(px(12.))
                    .opacity(1. - phase);
                for group in &previous.groups {
                    leaving = leaving.child(
                        div()
                            .pt(px(6.))
                            .pb(px(4.))
                            .px(px(12.))
                            .text_size(px(12.))
                            .text_color(p.faint)
                            .child(group.title.clone()),
                    );
                    for link in &group.links {
                        leaving = leaving.child(
                            div()
                                .px(px(12.))
                                .py(px(9.))
                                .flex()
                                .flex_col()
                                .gap(px(1.))
                                .child(link.label.clone())
                                .when(!link.description.is_empty(), |d| {
                                    d.child(
                                        div()
                                            .text_size(px(12.))
                                            .line_height(px(20.))
                                            .text_color(p.faint)
                                            .child(link.description.clone()),
                                    )
                                }),
                        );
                    }
                }
                surface = surface.child(leaving);
            }
            let bounds = self.panel_bounds.clone();
            root = root
                .child(
                    div()
                        .absolute()
                        .top(px(54.))
                        .left(px(caret_x - 6.))
                        .w(px(12.))
                        .h(px(12.))
                        .rounded_full()
                        .bg(p.bright)
                        .opacity(visible),
                )
                .child(
                    div()
                        .id("mega-panel")
                        .absolute()
                        .top(px(60.))
                        .left(px(left))
                        .w(px(width))
                        .h(px(h))
                        .overflow_hidden()
                        .rounded(radius)
                        .border_1()
                        .border_color(p.line)
                        .bg(p.panel)
                        .shadow(vec![
                            gpui::BoxShadow {
                                color: black().opacity(if dark { 0.3 } else { 0.04 }),
                                offset: gpui::point(px(0.), px(1.)),
                                blur_radius: px(2.),
                                spread_radius: px(0.),
                                inset: false,
                            },
                            gpui::BoxShadow {
                                color: black().opacity(if dark { 0.42 } else { 0.1 }),
                                offset: gpui::point(px(0.), px(if dark { 22. } else { 18. })),
                                blur_radius: px(if dark { 55. } else { 44. }),
                                spread_radius: px(0.),
                                inset: false,
                            },
                            gpui::BoxShadow {
                                color: black().opacity(if dark { 0. } else { 0.04 }),
                                offset: gpui::point(px(0.), px(4.)),
                                blur_radius: px(12.),
                                spread_radius: px(0.),
                                inset: false,
                            },
                        ])
                        .opacity(visible)
                        .child(
                            canvas(move |b, _, _| bounds.set(b), |_, _, _, _| ())
                                .absolute()
                                .inset_0(),
                        )
                        .on_hover(cx.listener(|s, hover, _, cx| {
                            s.panel_hovered = *hover;
                            if *hover {
                                s.hover_epoch = s.hover_epoch.wrapping_add(1)
                            } else {
                                s.schedule_close(cx)
                            }
                        }))
                        .child(
                            div()
                                .id("mega-scroll")
                                .h_full()
                                .overflow_y_scroll()
                                .child(surface),
                        ),
                );
        }
        root
    }
}
