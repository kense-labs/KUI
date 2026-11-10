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

const PANEL_PADDING: f32 = 12.;
const COLUMN_GAP: f32 = 8.;
const GROUP_WIDTH: f32 = 250.;
const GROUP_MIN_WIDTH: f32 = 156.;
const FEATURE_WIDTH: f32 = 216.;

fn panel_layout(section: &MegaMenuSection, viewport: f32) -> (f32, bool) {
    let groups = section.groups.len() as f32;
    let feature = if section.feature.is_some() {
        FEATURE_WIDTH
    } else {
        0.
    };
    let columns = groups + f32::from(section.feature.is_some());
    let fixed = feature + (columns - 1.).max(0.) * COLUMN_GAP + PANEL_PADDING * 2. + 2.;
    let width = (groups * GROUP_WIDTH + fixed)
        .max(320.)
        .min((viewport - 16.).max(0.));
    (
        width,
        columns > 1. && width >= groups * GROUP_MIN_WIDTH + fixed,
    )
}

fn panel_left(anchor: f32, width: f32, viewport: f32) -> f32 {
    (anchor - width * 0.5).clamp(8., (viewport - width - 8.).max(8.))
}

fn feature_background(p: Paint, cx: &App) -> Hsla {
    if crate::foundation::Theme::global(cx).mode == crate::foundation::ThemeMode::Dark {
        p.tray
    } else {
        p.stage
    }
}

fn scroll_report(scroll: ScrollHandle) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |_, _, _, cx| {
            if let Some(host) = cx.try_global::<crate::primitives::editor::EditorScrollRegion>() {
                let top = -scroll.offset().y;
                (host.0)(
                    scroll.bounds(),
                    top > px(0.1),
                    top + px(0.1) < scroll.max_offset().y,
                );
            }
        },
    )
    .absolute()
    .size(px(0.))
}

/// A navigation surface with hover intent, keyboard operation and a compact sheet.
pub struct MegaMenuMorph {
    brand: SharedString,
    brand_id: SharedString,
    brand_images: Option<[ImageSource; 2]>,
    compact_below: Pixels,
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
    hover_epoch: u64,
    panel_hovered: bool,
    pointer: Point<Pixels>,
    previous_pointer: Point<Pixels>,
    panel_bounds: Rc<Cell<Bounds<Pixels>>>,
    panel_content_height: Vec<Rc<Cell<Pixels>>>,
    panel_scroll: ScrollHandle,
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
        let panel_content_height = vec![(); sections.len()]
            .into_iter()
            .map(|()| Rc::new(Cell::new(px(0.))))
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
            brand_images: None,
            compact_below: px(0.),
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
            hover_epoch: 0,
            panel_hovered: false,
            pointer: Point::default(),
            previous_pointer: Point::default(),
            panel_bounds: Default::default(),
            panel_content_height,
            panel_scroll: ScrollHandle::new(),
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
    /// Set the images shown before the brand label in each color theme.
    pub fn with_brand_images(mut self, light: ImageSource, dark: ImageSource) -> Self {
        self.brand_images = Some([light, dark]);
        self
    }
    /// Use the compact navigation below this width even if all labels fit.
    pub fn with_compact_below(mut self, width: Pixels) -> Self {
        self.compact_below = width.max(px(0.));
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
            let old = self.open.or(self.previous);
            let from_closed = self.presence.value(cx) <= 0.001;
            self.previous = if from_closed { None } else { old };
            self.panel_scroll.set_offset(Point::default());
            self.direction = if self.previous.is_some_and(|old| i < old) {
                -1.
            } else {
                1.
            };
            self.open = Some(i);
            if from_closed {
                let root = self.root_bounds.get();
                let trigger = self.trigger_bounds[i].get();
                let viewport = f32::from(root.size.width);
                let (width, _) = panel_layout(&self.sections[i], viewport);
                let anchor = f32::from(trigger.center().x - root.left());
                self.width.reset(width);
                self.position.reset(panel_left(anchor, width, viewport));
                self.caret.reset(anchor);
                self.switch = PhaseClock::default();
            } else if old != Some(i) {
                self.switch.restart(cx);
            }
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

    fn panel_body(&self, i: usize, viewport: f32, active: bool, cx: &mut Context<Self>) -> Div {
        let section = &self.sections[i];
        let (width, horizontal) = panel_layout(section, viewport);
        let p = Paint::get(cx);
        let feature_bg = feature_background(p, cx);
        let radius = crate::foundation::surface_radius(cx);
        let mut body = div()
            .w(px((width - 2.).max(0.)))
            .flex()
            .gap(px(COLUMN_GAP))
            .p(px(PANEL_PADDING))
            .when(!horizontal, |d| d.flex_col());
        let mut index = 0;
        for group in &section.groups {
            let mut column = div()
                .flex()
                .flex_col()
                .gap(px(4.))
                .when(horizontal, |d| d.flex_1().min_w_0())
                .child(
                    div()
                        .pt(px(6.))
                        .pb(px(4.))
                        .px(px(12.))
                        .text_size(px(12.))
                        .text_color(p.faint)
                        .child(group.title.clone()),
                );
            for link in &group.links {
                let content = div()
                    .flex()
                    .flex_col()
                    .gap(px(1.))
                    .w_full()
                    .px(px(12.))
                    .py(px(9.))
                    .text_size(px(14.))
                    .line_height(px(19.6))
                    .whitespace_normal()
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
                                .font_weight(FontWeight::NORMAL)
                                .child(link.description.clone()),
                        )
                    });
                let at = index;
                index += 1;
                column = if active {
                    let id = link.id.clone();
                    column.child(
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
                        .track_focus(&self.panel_focus[i][at])
                        .hover(move |s| s.bg(p.ink.opacity(0.045)))
                        .child(content)
                        .on_click(cx.listener(move |s, _, _, cx| {
                            cx.emit(MegaMenuNavigate(id.clone()));
                            s.close(cx);
                        }))
                        .on_key_down(cx.listener(
                            move |s, e: &KeyDownEvent, w, cx| s.link_key(i, at, e, w, cx),
                        )),
                    )
                } else {
                    column.child(content)
                };
            }
            body = body.child(column);
        }
        if let Some(feature) = &section.feature {
            let content = div()
                .flex()
                .flex_col()
                .gap(px(4.))
                .p(px(14.))
                .w_full()
                .text_size(px(14.))
                .line_height(px(19.6))
                .whitespace_normal()
                .child(
                    div()
                        .font_weight(FontWeight::MEDIUM)
                        .child(feature.title.clone()),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(p.secondary)
                        .font_weight(FontWeight::NORMAL)
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
                );
            let width = if horizontal {
                px(FEATURE_WIDTH)
            } else {
                px((width - PANEL_PADDING * 2. - 2.).max(0.))
            };
            body =
                if active {
                    let id = feature.id.clone();
                    body.child(
                        face(
                            (ElementId::Name("mega-feature".into()), id.clone()),
                            feature.title.clone(),
                            84.,
                            radius.into(),
                            Tone::Surface,
                            cx,
                        )
                        .w(width)
                        .h_auto()
                        .min_h(px(0.))
                        .items_start()
                        .justify_start()
                        .bg(feature_bg)
                        .hover(move |s| s.bg(feature_bg.blend(p.ink.opacity(0.05))))
                        .track_focus(&self.panel_focus[i][index])
                        .child(content)
                        .on_click(cx.listener(move |s, _, _, cx| {
                            cx.emit(MegaMenuNavigate(id.clone()));
                            s.close(cx);
                        }))
                        .on_key_down(cx.listener(
                            move |s, e: &KeyDownEvent, w, cx| s.link_key(i, index, e, w, cx),
                        )),
                    )
                } else {
                    body.child(div().w(width).rounded(radius).bg(feature_bg).child(content))
                };
        }
        body
    }
}
impl Render for MegaMenuMorph {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.focus_subscriptions.is_empty() {
            self.focus_subscriptions
                .push(cx.observe_window_bounds(w, |_, _, cx| cx.notify()));
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
        let measured = self.root_bounds.get().size.width;
        let window = w.viewport_size().width;
        let viewport = if measured > px(0.) && window > px(0.) {
            f32::from(measured.min(window))
        } else {
            f32::from(measured.max(window))
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
        let brand_width = if self.brand_images.is_some() { 24. } else { 0. };
        let compact = viewport < (natural + brand_width + 24.).max(self.compact_below.into());
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
                .when_some(self.brand_images.as_ref(), |b, images| {
                    b.child(
                        img(images[usize::from(dark)].clone())
                            .size(px(18.))
                            .flex_none(),
                    )
                })
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
        let root_bounds = self.root_bounds.clone();
        let mut root = div()
            .on_children_prepainted(move |children, w, _| {
                if let Some(&bar) = children.first()
                    && (root_bounds.replace(bar).size.width - bar.size.width).abs() > px(0.1)
                {
                    w.request_animation_frame();
                }
            })
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
            .child(bar);
        if compact && self.compact_open {
            let mut sheet = div()
                .id("mega-mobile-sheet")
                .flex()
                .flex_col()
                .w_full()
                .max_h(px(360.))
                .overflow_y_scroll()
                .track_scroll(&self.panel_scroll)
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
                            .bg(feature_background(p, cx))
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
            root = root.child(sheet.child(scroll_report(self.panel_scroll.clone())));
        }
        if !compact
            && visible > 0.
            && let Some(i) = index
        {
            let section = &self.sections[i];
            let (target_width, horizontal) = panel_layout(section, viewport);
            let groups = section
                .groups
                .iter()
                .map(|group| 26. + group.links.len() as f32 * 62.);
            let estimate = if horizontal {
                groups.fold(if section.feature.is_some() { 110. } else { 0. }, f32::max)
            } else {
                groups.sum::<f32>() + if section.feature.is_some() { 110. } else { 0. }
            } + PANEL_PADDING * 2.
                + 2.;
            let measured = self.panel_content_height[i].get();
            let panel_h = if measured > px(0.) {
                f32::from(measured) + 2.
            } else {
                estimate
            }
            .clamp(64., 320.);
            if self.open.is_some() {
                if self.previous.is_none() {
                    self.height.reset(panel_h);
                } else {
                    self.height.duration(panel_h, 0.25, cx);
                }
            }
            self.width.duration(target_width, 0.25, cx);
            let trigger = self.trigger_bounds[i].get();
            let root_bounds = self.root_bounds.get();
            let anchor = if trigger.size.width > px(0.) {
                f32::from(trigger.left() - root_bounds.left() + trigger.size.width * 0.5)
            } else {
                i as f32 * 92. + 100.
            };
            self.caret.duration(anchor, 0.25, cx);
            self.position
                .duration(panel_left(anchor, target_width, viewport), 0.25, cx);
            let h = self.height.get(w, cx);
            let width = self.width.get(w, cx).min((viewport - 16.).max(0.));
            let left = self
                .position
                .get(w, cx)
                .clamp(8., (viewport - width - 8.).max(8.));
            let caret_x = self.caret.get(w, cx);
            let phase = ease(self.switch.time(0.24, w, cx) / 0.24);
            let content = self
                .panel_body(i, viewport, self.open.is_some(), cx)
                .opacity(phase)
                .relative()
                .left(px((1. - phase) * self.direction * 18.));
            let measured = self.panel_content_height[i].clone();
            let mut surface = div()
                .relative()
                .on_children_prepainted(move |children, w, _| {
                    if let Some(content) = children.first()
                        && measured.replace(content.size.height) != content.size.height
                    {
                        w.request_animation_frame();
                    }
                })
                .child(content);
            if phase < 1.
                && self.open.is_some()
                && let Some(old) = self.previous.filter(|old| *old != i)
            {
                surface = surface.child(
                    self.panel_body(old, viewport, false, cx)
                        .absolute()
                        .top_0()
                        .left(px(-self.direction * 18. * phase))
                        .opacity(1. - phase),
                );
            }
            let bounds = self.panel_bounds.clone();
            root = root
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
                            BoxShadow::new(px(0.), px(2.), black().opacity(0.06))
                                .blur_radius(px(6.)),
                            BoxShadow::new(
                                px(0.),
                                px(10.),
                                black().opacity(if dark { 0.30 } else { 0.10 }),
                            )
                            .blur_radius(px(26.)),
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
                                .track_scroll(&self.panel_scroll)
                                .child(surface)
                                .child(scroll_report(self.panel_scroll.clone())),
                        ),
                )
                .child(
                    canvas(
                        |_, _, _| (),
                        move |bounds, _, w, _| {
                            let tip = point(bounds.center().x, bounds.top());
                            let left = point(bounds.left(), bounds.bottom());
                            let right = point(bounds.right(), bounds.bottom());
                            let mut fill = PathBuilder::fill();
                            fill.move_to(left);
                            fill.line_to(tip);
                            fill.line_to(right);
                            fill.close();
                            if let Ok(path) = fill.build() {
                                w.paint_path(path, p.panel.opacity(visible));
                            }
                            let mut rim = PathBuilder::stroke(px(1.));
                            rim.move_to(left);
                            rim.line_to(tip);
                            rim.line_to(right);
                            if let Ok(path) = rim.build() {
                                w.paint_path(path, p.line.opacity(visible));
                            }
                        },
                    )
                    .absolute()
                    .top(px(54.))
                    .left(px(caret_x
                        .clamp(left + 12., (left + width - 12.).max(left + 12.))
                        - 6.))
                    .w(px(12.))
                    .h(px(7.)),
                );
        }
        root
    }
}

#[cfg(test)]
mod tests {
    use super::{MegaMenuFeature, MegaMenuGroup, MegaMenuLink, MegaMenuSection, panel_layout};

    #[test]
    fn menu_stacks_when_two_groups_and_the_feature_no_longer_fit() {
        let section = MegaMenuSection::new("products", "Products")
            .with_groups(vec![
                MegaMenuGroup::new("Build", vec![MegaMenuLink::new("one", "One")]),
                MegaMenuGroup::new("Targets", vec![MegaMenuLink::new("two", "Two")]),
            ])
            .with_feature(MegaMenuFeature::new("news", "News", "Read more"));
        let (wide, horizontal) = panel_layout(&section, 600.);
        assert!(horizontal);
        assert!(wide <= 584.);
        let (narrow, horizontal) = panel_layout(&section, 520.);
        assert!(!horizontal);
        assert!(narrow <= 504.);
    }
}
