//! A compact menu for contextual actions on a colorful surface.
use crate::primitives::{
    family::{Paint, PhaseClock, Tone, ease, face},
    glyph::{Glyph, icon},
    menu::{MenuAlign, MenuControl, MenuCore, MenuItem, render_menu},
};
use gpui::{prelude::*, *};

/// A menu action or an entry with a single level of children.
#[derive(Clone)]
pub struct GlassMenuItem {
    pub id: SharedString,
    pub label: SharedString,
    pub icon: Option<Glyph>,
    pub shortcut: SharedString,
    pub checked: Option<bool>,
    pub disabled: bool,
    pub danger: bool,
    pub keep_open: bool,
    pub children: Vec<GlassMenuItem>,
}
impl GlassMenuItem {
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: None,
            shortcut: "".into(),
            checked: None,
            disabled: false,
            danger: false,
            keep_open: false,
            children: Vec::new(),
        }
    }
    pub fn with_icon(mut self, icon: Glyph) -> Self {
        self.icon = Some(icon);
        self
    }
    pub fn with_shortcut(mut self, shortcut: impl Into<SharedString>) -> Self {
        self.shortcut = shortcut.into();
        self
    }
    pub fn with_checked(mut self, checked: bool) -> Self {
        self.checked = Some(checked);
        self
    }
    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
    pub fn with_danger(mut self, danger: bool) -> Self {
        self.danger = danger;
        self
    }
    pub fn with_keep_open(mut self, keep_open: bool) -> Self {
        self.keep_open = keep_open;
        self
    }
    pub fn with_children(mut self, children: Vec<GlassMenuItem>) -> Self {
        self.children = children;
        self
    }
}
#[derive(Clone)]
pub struct GlassMenuGroup {
    pub label: Option<SharedString>,
    pub items: Vec<GlassMenuItem>,
}
impl GlassMenuGroup {
    pub fn new(items: Vec<GlassMenuItem>) -> Self {
        Self { label: None, items }
    }
    pub fn with_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GlassMenuSelected {
    pub id: SharedString,
    pub checked: Option<bool>,
}
/// Single-level contextual action menu. Host state owns the result of each action.
pub struct GlassMenu {
    core: MenuCore,
    title: SharedString,
    root: Vec<GlassMenuItem>,
    separators: Vec<usize>,
    submenu: Option<usize>,
    width: f32,
    submenu_motion: PhaseClock,
    submenu_direction: f32,
}
impl EventEmitter<GlassMenuSelected> for GlassMenu {}
impl GlassMenu {
    pub fn new(
        id: impl Into<ElementId>,
        title: impl Into<SharedString>,
        groups: Vec<GlassMenuGroup>,
        cx: &App,
    ) -> Self {
        let title = title.into();
        let mut root = Vec::new();
        let mut separators = Vec::new();
        for group in groups {
            if !root.is_empty() && !group.items.is_empty() {
                separators.push(root.len());
            }
            if !group.items.is_empty()
                && let Some(label) = group.label
            {
                root.push(
                    GlassMenuItem::new(format!("__group_{}", root.len()), label)
                        .with_disabled(true),
                );
            }
            root.extend(group.items);
        }
        let rows = Self::root_rows(&root, &separators);
        Self {
            core: MenuCore::new(id.into(), title.clone(), rows, cx),
            title,
            root,
            separators,
            submenu: None,
            width: 256.,
            submenu_motion: PhaseClock::default(),
            submenu_direction: 1.,
        }
    }
    fn as_menu(item: &GlassMenuItem) -> MenuItem {
        MenuItem {
            id: item.id.clone(),
            label: item.label.clone(),
            glyph: item.icon,
            disabled: item.disabled,
            destructive: item.danger,
            separator_before: false,
            shortcut: item.shortcut.clone(),
            checked: item.checked,
            detail: "".into(),
        }
    }
    fn root_rows(root: &[GlassMenuItem], separators: &[usize]) -> Vec<MenuItem> {
        root.iter()
            .enumerate()
            .map(|(i, item)| {
                let mut row = Self::as_menu(item);
                row.separator_before = separators.contains(&i);
                row
            })
            .collect()
    }
    fn show_children(&mut self, i: usize, cx: &mut Context<Self>) {
        let Some(item) = self.root.get(i) else {
            return;
        };
        if item.disabled || item.children.is_empty() {
            return;
        }
        let mut rows = vec![MenuItem::new("__back", item.label.clone()).with_icon(Glyph::Undo)];
        rows.extend(item.children.iter().map(Self::as_menu));
        self.submenu = Some(i);
        self.core.replace_items(rows);
        self.submenu_direction = 1.;
        self.submenu_motion.restart(cx);
        cx.notify();
    }
    fn show_root(&mut self, cx: &mut Context<Self>) {
        self.submenu = None;
        self.core
            .replace_items(Self::root_rows(&self.root, &self.separators));
        self.submenu_direction = -1.;
        self.submenu_motion.restart(cx);
        cx.notify();
    }
    pub fn with_align(mut self, align: MenuAlign) -> Self {
        self.core.align = align;
        self
    }
    pub fn with_width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = width.clamp(160., 480.);
        }
        self
    }
    pub fn is_open(&self) -> bool {
        self.core.open
    }
    /// Reflect host-owned checkbox state without emitting an event.
    pub fn set_checked(&mut self, id: &str, checked: bool, cx: &mut Context<Self>) {
        for item in &mut self.root {
            if item.id.as_ref() == id && item.checked.is_some() {
                item.checked = Some(checked);
            }
            for child in &mut item.children {
                if child.id.as_ref() == id && child.checked.is_some() {
                    child.checked = Some(checked);
                }
            }
        }
        if let Some(parent) = self.submenu {
            for (row, item) in self
                .core
                .items
                .iter_mut()
                .skip(1)
                .zip(&self.root[parent].children)
            {
                row.checked = item.checked;
            }
        } else {
            for (row, item) in self.core.items.iter_mut().zip(&self.root) {
                row.checked = item.checked;
            }
        }
        cx.notify();
    }
}
impl MenuControl for GlassMenu {
    fn core(&mut self) -> &mut MenuCore {
        &mut self.core
    }
    fn morph_panel(&self) -> bool {
        true
    }
    fn content_motion(&mut self, w: &Window, cx: &App) -> (f32, f32) {
        let t = ease(self.submenu_motion.time(0.22, w, cx) / 0.22);
        (self.submenu_direction * 18. * (1. - t), t)
    }
    fn selected(&mut self, i: usize, w: &mut Window, cx: &mut Context<Self>) {
        if self.submenu.is_some() && i == 0 {
            self.show_root(cx);
            return;
        }
        if self.submenu.is_none()
            && self
                .root
                .get(i)
                .is_some_and(|item| !item.children.is_empty())
        {
            self.show_children(i, cx);
            return;
        }
        let item = if let Some(parent) = self.submenu {
            self.root[parent].children.get(i.saturating_sub(1))
        } else {
            self.root.get(i)
        };
        let Some(item) = item.cloned() else {
            return;
        };
        if item.disabled {
            return;
        }
        cx.emit(GlassMenuSelected {
            id: item.id.clone(),
            checked: item.checked.map(|v| !v),
        });
        if !item.keep_open {
            self.core.close(w, cx);
            self.submenu = None;
            self.core
                .replace_items(Self::root_rows(&self.root, &self.separators));
        }
        cx.notify();
    }
    fn horizontal(&mut self, i: usize, right: bool, cx: &mut Context<Self>) {
        if right && self.submenu.is_none() {
            self.show_children(i, cx);
        } else if !right && self.submenu.is_some() {
            self.show_root(cx);
        }
    }
    fn accessory(&self, i: usize, cx: &Context<Self>, _: &Window) -> Option<AnyElement> {
        if self.submenu.is_none()
            && self
                .root
                .get(i)
                .is_some_and(|item| !item.children.is_empty())
        {
            Some(icon(Glyph::Arrow, 14., Paint::get(cx).secondary).into_any_element())
        } else {
            None
        }
    }
}
impl Render for GlassMenu {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.core.open && self.submenu.take().is_some() {
            self.core
                .replace_items(Self::root_rows(&self.root, &self.separators));
        }
        let p = Paint::get(cx);
        let r = crate::foundation::surface_radius(cx);
        let button = face(
            "glass-menu-trigger",
            self.title.clone(),
            40.,
            r.into(),
            Tone::Surface,
            cx,
        )
        .pl(px(12.))
        .pr(px(14.))
        .gap(px(8.))
        .text_size(px(14.))
        .font_weight(FontWeight::MEDIUM)
        .border_1()
        .border_color(p.line)
        .bg(p.panel.opacity(0.85))
        .child(self.title.clone())
        .child(icon(Glyph::Down, 14., p.secondary));
        render_menu(self, button, false, self.width, w, cx)
    }
}
