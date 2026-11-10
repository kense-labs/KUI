//! Floating section navigation with a compact scroll state and a draggable glass lens.
use crate::{
    foundation::{Theme, ThemeMode},
    primitives::{
        control::UiDirection,
        direct_touch::Contact,
        family::{Slide, disabled_motion, ease},
        glyph::Glyph,
        presentation::tr,
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc, sync::Arc};

const SMOOTH: f32 = 0.40;
const INSET: f32 = 4.;
const ACTION_GAP: f32 = 10.;
const DRAG_THRESHOLD: f32 = 4.;
const SCROLL_THRESHOLD: f32 = 8.;

/// Built-in navigation marks, any KUI glyph, or a monochrome SVG asset.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GlassTabIcon {
    Home,
    Explore,
    Saved,
    Profile,
    Search,
    Glyph(Glyph),
    Svg(SharedString),
}

/// A destination in the main bar or its optional action bubble.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GlassTabBarItem {
    pub value: SharedString,
    pub label: SharedString,
    pub icon: GlassTabIcon,
    pub badge: Option<SharedString>,
    pub badge_label: Option<SharedString>,
}

impl GlassTabBarItem {
    pub fn new(
        value: impl Into<SharedString>,
        label: impl Into<SharedString>,
        icon: GlassTabIcon,
    ) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            icon,
            badge: None,
            badge_label: None,
        }
    }

    pub fn with_badge(mut self, value: impl ToString) -> Self {
        let value = value.to_string();
        let value = value.trim();
        self.badge = (!value.is_empty() && value != "0").then(|| value.to_owned().into());
        self
    }

    pub fn with_badge_label(mut self, label: impl Into<SharedString>) -> Self {
        self.badge_label = Some(label.into());
        self
    }

    fn name(&self, cx: &App) -> SharedString {
        let Some(badge) = &self.badge else {
            return self.label.clone();
        };
        let suffix = self.badge_label.clone().unwrap_or_else(|| {
            if tr(cx, "en", "zh") == "zh" {
                format!("{badge} 条新消息").into()
            } else {
                format!("{badge} new").into()
            }
        });
        format!("{}, {suffix}", self.label).into()
    }
}

/// Selection accepted automatically unless `with_value` was used.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GlassTabBarValueChanged(pub SharedString);

/// Size accepted automatically unless `with_compact` was used.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GlassTabBarCompactChanged(pub bool);

/// A texture of the content behind the glass, in window coordinates.
/// `soft` should cover the same content as `image` with blur already applied.
#[derive(Clone)]
pub struct GlassTabBackdrop {
    pub image: Arc<RenderImage>,
    pub soft: Option<Arc<RenderImage>>,
    pub bounds: Bounds<Pixels>,
}

#[derive(Clone, Copy, Default, PartialEq)]
struct Layout {
    slot: f32,
    height: f32,
    track: f32,
    action: f32,
    count: usize,
    rtl: bool,
}

impl Layout {
    fn new(width: f32, count: usize, action: bool, compact: f32, rtl: bool) -> Self {
        let height = 62. - 14. * compact;
        let bubble = if action { height.min(width) } else { 0. };
        let room = |action: f32| {
            (width - action - if action > 0. { ACTION_GAP } else { 0. } - 2. * INSET).max(0.)
                / count.max(1) as f32
        };
        let normal = room(if action { 62. } else { 0. }).min(78.);
        let small = room(if action { 48. } else { 0. }).min(48.);
        let slot = normal + (small - normal) * compact;
        Self {
            slot,
            height,
            track: if count == 0 {
                0.
            } else {
                slot * count as f32 + 2. * INSET
            },
            action: bubble,
            count,
            rtl,
        }
    }

    fn gap(self) -> f32 {
        if self.track > 0. && self.action > 0. {
            ACTION_GAP
        } else {
            0.
        }
    }

    fn width(self) -> f32 {
        self.track + self.gap() + self.action
    }

    fn track_left(self) -> f32 {
        if self.rtl {
            self.action + self.gap()
        } else {
            0.
        }
    }

    fn action_left(self) -> f32 {
        if self.rtl {
            0.
        } else {
            self.track + self.gap()
        }
    }

    fn slot_center(self, index: f32) -> f32 {
        let index = if self.rtl {
            self.count as f32 - 1. - index
        } else {
            index
        };
        self.track_left() + INSET + (index + 0.5) * self.slot
    }

    fn index_at(self, x: f32) -> f32 {
        let i = (x - self.track_left() - INSET) / self.slot.max(0.01) - 0.5;
        if self.rtl {
            self.count as f32 - 1. - i
        } else {
            i
        }
    }

    fn hit(self, x: f32) -> Option<usize> {
        if self.action > 0. && (self.action_left()..self.action_left() + self.action).contains(&x) {
            return Some(self.count);
        }
        let index = self.index_at(x).round();
        (index >= 0. && index < self.count as f32).then_some(index as usize)
    }
}

struct Slot {
    item: GlassTabBarItem,
    focus: FocusHandle,
}

impl Slot {
    fn new(item: GlassTabBarItem, cx: &App) -> Self {
        Self {
            item,
            focus: cx.focus_handle().tab_stop(true),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Axis {
    Pending,
    Horizontal,
    Vertical,
}

struct Drag {
    from: usize,
    start: Point<Pixels>,
    start_index: f32,
    index: f32,
    start_offset: Pixels,
    axis: Axis,
    touch: bool,
}

/// A bottom bar with one tab stop, an optional action and a shared geometry spring.
pub struct GlassTabBar {
    id: SharedString,
    label: SharedString,
    tabs: Vec<Slot>,
    action: Option<Slot>,
    value: Option<SharedString>,
    roving: usize,
    value_controlled: bool,
    compact: bool,
    compact_controlled: bool,
    collapse_on_scroll: bool,
    scroll: Option<ScrollHandle>,
    scroll_anchor: Option<Pixels>,
    scroll_request: Option<bool>,
    style: Option<Box<dyn Fn(Div) -> Div>>,
    backdrop: Option<Box<dyn Fn() -> Option<GlassTabBackdrop>>>,
    solid: bool,
    increased_contrast: bool,
    room: Rc<Cell<Pixels>>,
    bounds: Rc<Cell<Bounds<Pixels>>>,
    layout: Layout,
    lens: Slide,
    opacity: Slide,
    action_fill: Slide,
    size: Slide,
    press: Slide,
    drag: Option<Drag>,
    suppress_click: bool,
    touch: Contact,
}

impl EventEmitter<GlassTabBarValueChanged> for GlassTabBar {}
impl EventEmitter<GlassTabBarCompactChanged> for GlassTabBar {}

impl GlassTabBar {
    pub fn new(items: Vec<GlassTabBarItem>, cx: &mut Context<Self>) -> Self {
        let value = items.first().map(|item| item.value.clone());
        Self {
            id: format!("glass-tabs-{}", cx.entity_id()).into(),
            label: "Tabs".into(),
            tabs: items.into_iter().map(|i| Slot::new(i, cx)).collect(),
            action: None,
            value_controlled: false,
            compact: false,
            compact_controlled: false,
            collapse_on_scroll: true,
            scroll: None,
            scroll_anchor: None,
            scroll_request: None,
            style: None,
            backdrop: None,
            solid: false,
            increased_contrast: false,
            room: Rc::default(),
            bounds: Rc::default(),
            layout: Layout::default(),
            lens: Slide::new(0.),
            opacity: Slide::snappy(f32::from(value.is_some())),
            action_fill: Slide::snappy(0.),
            size: Slide::new(0.),
            press: Slide::snappy(0.),
            value,
            roving: 0,
            drag: None,
            suppress_click: false,
            touch: Contact::default(),
        }
    }

    pub fn with_id(mut self, id: impl Into<SharedString>) -> Self {
        self.id = id.into();
        self
    }

    pub fn with_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = label.into();
        self
    }

    pub fn with_action(mut self, action: GlassTabBarItem, cx: &App) -> Self {
        self.action = Some(Slot::new(action, cx));
        self.initial_selection();
        self
    }

    /// Controlled selection. An unknown value hides both indicators.
    pub fn with_value(mut self, value: impl Into<SharedString>) -> Self {
        self.value_controlled = true;
        self.value = Some(value.into());
        self.initial_selection();
        self
    }

    pub fn with_default_value(mut self, value: impl Into<SharedString>) -> Self {
        if !self.value_controlled {
            self.value = Some(value.into());
            self.initial_selection();
        }
        self
    }

    pub fn with_compact(mut self, compact: bool) -> Self {
        self.compact_controlled = true;
        self.compact = compact;
        self.size.reset(f32::from(compact));
        self
    }

    pub fn with_default_compact(mut self, compact: bool) -> Self {
        if !self.compact_controlled {
            self.compact = compact;
            self.size.reset(f32::from(compact));
        }
        self
    }

    pub fn with_collapse_on_scroll(mut self, collapse: bool) -> Self {
        self.collapse_on_scroll = collapse;
        self
    }

    /// Watch a GPUI scroll viewport. Scroll hosts can also forward a positive offset explicitly.
    pub fn with_scroll_handle(mut self, handle: ScrollHandle) -> Self {
        self.scroll = Some(handle);
        self
    }

    pub fn with_style(mut self, style: impl Fn(Div) -> Div + 'static) -> Self {
        self.style = Some(Box::new(style));
        self
    }

    pub fn with_backdrop(
        mut self,
        backdrop: impl Fn() -> Option<GlassTabBackdrop> + 'static,
    ) -> Self {
        self.backdrop = Some(Box::new(backdrop));
        self
    }

    pub fn with_solid_surface(mut self, solid: bool) -> Self {
        self.solid = solid;
        self
    }

    pub fn with_increased_contrast(mut self, contrast: bool) -> Self {
        self.increased_contrast = contrast;
        self
    }

    pub fn value(&self) -> Option<&str> {
        self.value.as_deref()
    }

    pub fn is_compact(&self) -> bool {
        self.compact
    }

    pub fn set_value(&mut self, value: impl Into<SharedString>, cx: &mut Context<Self>) {
        let value = value.into();
        if self.value.as_ref() != Some(&value) {
            if let Some(drag) = self.drag.take() {
                if drag.axis == Axis::Horizontal {
                    self.lens.reset(drag.index);
                }
                self.press.duration(0., 0.18, cx);
            }
            self.value = Some(value);
            if let Some(i) = self.selected() {
                self.roving = i;
            }
            self.retarget(cx);
            cx.notify();
        }
    }

    pub fn set_compact(&mut self, compact: bool, cx: &mut Context<Self>) {
        if self.compact != compact {
            self.compact = compact;
            self.size.duration(f32::from(compact), SMOOTH, cx);
            cx.notify();
        }
    }

    /// Observe a positive scroll distance, with a short dead band for trackpad jitter.
    pub fn observe_scroll_offset(&mut self, offset: Pixels, cx: &mut Context<Self>) {
        let offset = offset.max(px(0.));
        let Some(anchor) = self.scroll_anchor else {
            self.scroll_anchor = Some(offset);
            return;
        };
        let delta = offset - anchor;
        if !self.collapse_on_scroll || delta.abs() < px(SCROLL_THRESHOLD) && offset > px(0.) {
            return;
        }
        let compact = offset > px(0.) && delta > px(0.);
        self.scroll_anchor = Some(offset);
        if self.scroll_request != Some(compact) {
            self.scroll_request = Some(compact);
            self.request_compact(compact, cx);
        }
    }

    fn len(&self) -> usize {
        self.tabs.len() + usize::from(self.action.is_some())
    }

    fn slot(&self, i: usize) -> Option<&Slot> {
        if i == self.tabs.len() {
            self.action.as_ref()
        } else {
            self.tabs.get(i)
        }
    }

    fn selected(&self) -> Option<usize> {
        let value = self.value.as_ref()?;
        self.tabs
            .iter()
            .chain(self.action.iter())
            .position(|s| &s.item.value == value)
    }

    fn initial_selection(&mut self) {
        let selected = self.selected();
        let main = selected.filter(|&i| i < self.tabs.len());
        self.roving = selected.unwrap_or(0);
        self.lens.reset(main.unwrap_or(0) as f32);
        self.opacity.reset(f32::from(main.is_some()));
        self.action_fill
            .reset(f32::from(selected.is_some_and(|i| i == self.tabs.len())));
    }

    fn retarget(&mut self, cx: &App) {
        let selected = self.selected();
        let main = selected.filter(|&i| i < self.tabs.len());
        if let Some(i) = main {
            self.lens.duration(i as f32, SMOOTH, cx);
        }
        self.opacity.duration(f32::from(main.is_some()), 0.18, cx);
        self.action_fill.duration(
            f32::from(selected.is_some_and(|i| i == self.tabs.len())),
            0.18,
            cx,
        );
    }

    fn request_compact(&mut self, compact: bool, cx: &mut Context<Self>) {
        if self.compact != compact {
            if !self.compact_controlled {
                self.set_compact(compact, cx);
            }
            cx.emit(GlassTabBarCompactChanged(compact));
        }
    }

    fn choose(&mut self, i: usize, grow: bool, w: &mut Window, cx: &mut Context<Self>) {
        let Some(slot) = self.slot(i) else { return };
        let value = slot.item.value.clone();
        slot.focus.focus(w, cx);
        self.roving = i;
        if self.value.as_ref() != Some(&value) {
            if !self.value_controlled {
                self.set_value(value.clone(), cx);
            }
            cx.emit(GlassTabBarValueChanged(value));
        }
        if grow {
            self.request_compact(false, cx);
            self.scroll_request = None;
        }
        cx.notify();
    }

    fn key(&mut self, i: usize, e: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        let key = e.keystroke.key.as_str();
        let next = match key {
            "left" | "right" => {
                let dir = UiDirection::current(cx).inline_step(if key == "left" { -1 } else { 1 });
                (i as isize + dir).rem_euclid(self.len() as isize) as usize
            }
            "home" => 0,
            "end" => self.len() - 1,
            "enter" | "space" if !e.is_held => i,
            "escape" if self.drag.is_some() => {
                self.finish(true, w, cx);
                w.prevent_default();
                cx.stop_propagation();
                return;
            }
            _ => return,
        };
        self.choose(next, matches!(key, "enter" | "space"), w, cx);
        w.prevent_default();
        cx.stop_propagation();
    }

    fn begin(&mut self, i: usize, start: Point<Pixels>, touch: bool, cx: &mut Context<Self>) {
        self.suppress_click = false;
        let active = self.selected() == Some(i) && i < self.tabs.len();
        if !active && !touch {
            return;
        }
        let index = self.lens.value(cx);
        self.drag = Some(Drag {
            from: i,
            start,
            start_index: index,
            index,
            start_offset: self.scroll.as_ref().map_or(px(0.), |h| h.offset().y),
            axis: Axis::Pending,
            touch,
        });
        if active {
            self.press.duration(1., 0.16, cx);
        }
        cx.notify();
    }

    fn move_drag(
        &mut self,
        at: Point<Pixels>,
        touch: bool,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let selected = self.selected();
        let Some(drag) = self.drag.as_mut().filter(|d| d.touch == touch) else {
            return;
        };
        let dx = f32::from(at.x - drag.start.x);
        let dy = f32::from(at.y - drag.start.y);
        if drag.axis == Axis::Pending && dx.abs().max(dy.abs()) >= DRAG_THRESHOLD {
            drag.axis = if dx.abs() >= dy.abs()
                && selected == Some(drag.from)
                && drag.from < self.tabs.len()
            {
                Axis::Horizontal
            } else {
                Axis::Vertical
            };
            self.suppress_click = true;
            if drag.axis == Axis::Vertical {
                self.press.duration(0., 0.16, cx);
            }
        }
        if drag.axis == Axis::Horizontal {
            let direction = if self.layout.rtl { -1. } else { 1. };
            drag.index = (drag.start_index + direction * dx / self.layout.slot.max(1.))
                .clamp(0., self.tabs.len().saturating_sub(1) as f32);
            cx.notify();
        } else if drag.axis == Axis::Vertical
            && touch
            && let Some(handle) = &self.scroll
        {
            let offset = (drag.start_offset + px(dy)).clamp(-handle.max_offset().y, px(0.));
            handle.set_offset(point(handle.offset().x, offset));
            self.observe_scroll_offset(-offset, cx);
            w.refresh();
            cx.notify();
        }
    }

    fn finish(&mut self, cancel: bool, w: &mut Window, cx: &mut Context<Self>) {
        let Some(drag) = self.drag.take() else { return };
        self.press.duration(0., 0.18, cx);
        if drag.axis == Axis::Horizontal {
            self.lens.reset(drag.index);
            if !cancel {
                self.choose(drag.index.round() as usize, false, w, cx);
            }
            self.retarget(cx);
        } else if drag.axis == Axis::Pending && drag.touch && !cancel {
            self.suppress_click = true;
            self.choose(drag.from, false, w, cx);
        }
        if self.suppress_click {
            let bar = cx.entity().downgrade();
            cx.defer(move |cx| {
                let _ = bar.update(cx, |bar, _| bar.suppress_click = false);
            });
        }
        cx.notify();
    }
}

#[derive(Clone, Copy)]
struct Palette {
    dark: bool,
    solid: bool,
    contrast: bool,
    ink: Hsla,
    selected: Hsla,
    focus: Hsla,
}

impl Palette {
    fn new(solid: bool, contrast: bool, cx: &App) -> Self {
        let theme = Theme::global(cx);
        let dark = theme.mode == ThemeMode::Dark;
        let selected: Hsla = rgb(if dark { 0xf2f2f2 } else { 0x0b0b0b }).into();
        let idle: Hsla = rgb(if dark { 0xf5f5f5 } else { 0x161616 }).into();
        let accent = theme.accent.for_mode(theme.mode);
        Self {
            dark,
            solid,
            contrast,
            ink: idle.opacity(if contrast {
                1.
            } else if dark {
                0.86
            } else {
                0.88
            }),
            selected: if accent.is_transparent() {
                selected
            } else {
                accent
            },
            focus: theme.tokens.colors.focus_outline,
        }
    }
}

fn capsule(b: Bounds<Pixels>, backdrop: Option<&GlassTabBackdrop>, p: Palette, w: &mut Window) {
    let r = b.size.height.min(b.size.width) / 2.;
    let (falloff_y, blur, falloff, contact) = if p.dark {
        (12., 32., 0.65, 0.40)
    } else {
        (10., 30., 0.30, 0.10)
    };
    w.paint_drop_shadows(
        b,
        r.into(),
        &[
            BoxShadow::new(px(0.), px(falloff_y), black().opacity(falloff))
                .blur_radius(px(blur))
                .spread_radius(px(-10.)),
            BoxShadow::new(px(0.), px(2.), black().opacity(contact))
                .blur_radius(px(8.))
                .spread_radius(px(-2.)),
        ],
    );
    if !p.solid
        && let Some(backdrop) = backdrop
    {
        let image = backdrop.soft.as_ref().unwrap_or(&backdrop.image);
        if image.as_bytes(0).is_some() {
            let _ = w.paint_image(b, backdrop.bounds, r.into(), image.clone(), 0, false);
        }
    }
    let tint: Hsla = rgb(if p.dark { 0x191919 } else { 0xfcfcfc }).into();
    let edge = if p.dark { white() } else { black() };
    let alpha = if p.solid {
        1.
    } else if p.contrast {
        0.8
    } else if p.dark {
        0.58
    } else {
        0.56
    };
    w.paint_quad(quad(
        b,
        r,
        tint.opacity(alpha),
        px(0.5),
        edge.opacity(if p.contrast { 0.30 } else { 0.07 }),
        BorderStyle::Solid,
    ));
    w.paint_inset_shadows(
        b,
        r.into(),
        &[
            BoxShadow::new(
                px(0.),
                px(1.),
                white().opacity(if p.dark { 0.36 } else { 0.95 }),
            )
            .blur_radius(px(0.5))
            .spread_radius(px(-0.5))
            .inset(),
            BoxShadow::new(
                px(0.),
                px(-1.),
                white().opacity(if p.dark { 0.14 } else { 0.50 }),
            )
            .blur_radius(px(0.5))
            .spread_radius(px(-0.5))
            .inset(),
            BoxShadow::new(
                px(0.),
                px(0.),
                white().opacity(if p.dark { 0.05 } else { 0.32 }),
            )
            .blur_radius(px(16.))
            .inset(),
        ],
    );
}

fn lens(
    b: Bounds<Pixels>,
    backdrop: Option<&GlassTabBackdrop>,
    p: Palette,
    alpha: f32,
    lift: f32,
    w: &mut Window,
) {
    if alpha <= 0.002 || b.size.width <= px(0.) {
        return;
    }
    let r = b.size.height.min(b.size.width) / 2.;
    if !p.solid
        && lift > 0.02
        && let Some(backdrop) = backdrop
        && backdrop.image.as_bytes(0).is_some()
    {
        let scale = 1. + 0.12 * lift;
        let anchor = b.center();
        let region = Bounds::new(
            anchor + (backdrop.bounds.origin - anchor) * scale,
            backdrop.bounds.size.map(|s| s * scale),
        );
        // Reproject the supplied content through the same rounded aperture as the moving lens.
        let _ = w.paint_image(b, region, r.into(), backdrop.image.clone(), 0, false);
    }
    let shade = if p.dark { white() } else { black() };
    let amount = if p.contrast {
        0.24
    } else if p.dark {
        0.13
    } else {
        0.07
    };
    let color = shade.opacity(alpha * amount * (1. - 0.53 * lift));
    let rim = white().opacity(alpha * (if p.dark { 0.10 } else { 0.45 } + 0.32 * lift));
    w.paint_quad(quad(b, r, color, px(0.5), rim, BorderStyle::Solid));
}

fn paint_icon(mark: &GlassTabIcon, b: Bounds<Pixels>, color: Hsla, w: &mut Window, cx: &App) {
    let scale = f32::from(b.size.width) / 24.;
    let at = |x, y| b.origin + point(px(x * scale), px(y * scale));
    let mut path = PathBuilder::stroke(px(1.8 * scale));
    let circle = |cx: f32, cy: f32, radius: f32, path: &mut PathBuilder| {
        let r = point(px(radius * scale), px(radius * scale));
        path.move_to(at(cx + radius, cy));
        path.arc_to(r, px(0.), false, true, at(cx - radius, cy));
        path.arc_to(r, px(0.), false, true, at(cx + radius, cy));
        path.close();
    };
    match mark {
        GlassTabIcon::Svg(asset) => {
            let _ = w.paint_svg(
                b,
                asset.clone(),
                None,
                TransformationMatrix::unit(),
                color,
                cx,
            );
            return;
        }
        GlassTabIcon::Glyph(glyph) => {
            for line in glyph.strokes() {
                if let Some((&(x, y), rest)) = line.split_first() {
                    path.move_to(at(x, y));
                    for &(x, y) in rest {
                        path.line_to(at(x, y));
                    }
                }
            }
        }
        GlassTabIcon::Home => {
            path.move_to(at(3., 9.6));
            path.line_to(at(11., 2.8));
            path.curve_to(at(13., 2.8), at(12., 2.));
            path.line_to(at(21., 9.6));
            path.line_to(at(21., 20.));
            path.curve_to(at(19.5, 21.5), at(21., 21.5));
            path.line_to(at(4.5, 21.5));
            path.curve_to(at(3., 20.), at(3., 21.5));
            path.close();
            path.move_to(at(9., 21.5));
            path.line_to(at(9., 12.));
            path.line_to(at(15., 12.));
            path.line_to(at(15., 21.5));
        }
        GlassTabIcon::Explore => {
            circle(12., 12., 10., &mut path);
            path.move_to(at(15.7, 8.3));
            path.line_to(at(13.5, 13.5));
            path.line_to(at(8.3, 15.7));
            path.line_to(at(10.5, 10.5));
            path.close();
        }
        GlassTabIcon::Saved => {
            path.move_to(at(5., 5.));
            path.curve_to(at(7., 3.), at(5., 3.));
            path.line_to(at(17., 3.));
            path.curve_to(at(19., 5.), at(19., 3.));
            path.line_to(at(19., 21.));
            path.line_to(at(12., 17.));
            path.line_to(at(5., 21.));
            path.close();
        }
        GlassTabIcon::Profile => {
            circle(12., 7., 4., &mut path);
            path.move_to(at(4., 22.));
            path.cubic_bezier_to(at(20., 22.), at(4., 11.5), at(20., 11.5));
        }
        GlassTabIcon::Search => {
            circle(10.5, 10.5, 7.5, &mut path);
            path.move_to(at(16.2, 16.2));
            path.line_to(at(21., 21.));
        }
    }
    if let Ok(path) = path.build() {
        w.paint_path(path, color);
    }
}

#[derive(Clone, Copy)]
struct MotionFrame {
    compact: f32,
    lens_index: f32,
    lens_alpha: f32,
    action: f32,
    lift: f32,
    stretch: f32,
}

impl MotionFrame {
    fn selection(self, i: usize) -> f32 {
        self.lens_alpha * (1. - (self.lens_index - i as f32).abs()).clamp(0., 1.)
    }
}

impl GlassTabBar {
    fn motion_frame(&self, w: &Window, cx: &App) -> MotionFrame {
        let reduced = disabled_motion(cx);
        let now = cx.background_executor().now();
        let (position, speed, running) = self.lens.sample(now);
        let drag = self.drag.as_ref().filter(|d| d.axis == Axis::Horizontal);
        if running && !reduced && drag.is_none() {
            w.request_animation_frame();
        }
        let (lift, stretch) = if reduced {
            (0., 0.)
        } else {
            let travel = (speed.abs() / 13.).clamp(0., 1.);
            let press = self.press.get(w, cx);
            (
                travel.max(press),
                travel.max(if drag.is_some() { 0.56 } else { 0. }),
            )
        };
        MotionFrame {
            compact: self.size.get(w, cx).clamp(0., 1.),
            lens_index: drag.map_or(
                if reduced {
                    self.lens.value(cx)
                } else {
                    position
                },
                |d| d.index,
            ),
            lens_alpha: self.opacity.get(w, cx).clamp(0., 1.),
            action: self.action_fill.get(w, cx).clamp(0., 1.),
            lift,
            stretch,
        }
    }

    fn render_slot(
        &self,
        i: usize,
        p: Palette,
        motion: MotionFrame,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let slot = self.slot(i).expect("a rendered tab index refers to a slot");
        let l = self.layout;
        let action = i == l.count;
        let width = if action { l.action } else { l.slot };
        let left = if action {
            l.action_left()
        } else {
            l.slot_center(i as f32) - width / 2.
        };
        let strength = if action {
            motion.action
        } else {
            motion.selection(i)
        };
        let ink = p.ink.blend(p.selected.opacity(strength));
        let icon_size = 22. * (1. + 0.10 * strength * motion.lift);
        let y = if action {
            l.height / 2.
        } else {
            23. + motion.compact
        };
        let name = slot.item.name(cx);
        let debug = name.clone();
        let id: SharedString = format!("{}-tab-{}", self.id, slot.item.value).into();
        let icon = slot.item.icon.clone();
        div()
            .id(id.clone())
            .debug_selector(move || debug.to_string())
            .accessibility_id(id)
            .role(Role::Tab)
            .aria_label(name)
            .aria_selected(self.selected() == Some(i))
            .track_focus(&slot.focus)
            .tab_stop(self.roving == i)
            .cursor_pointer()
            .absolute()
            .left(px(left))
            .top_0()
            .w(px(width))
            .h(px(l.height))
            .rounded_full()
            .border_2()
            .border_color(transparent_black())
            .focus_visible(move |s| s.border_color(p.focus))
            .child(
                canvas(
                    |_, _, _| (),
                    move |b, _, w, cx| paint_icon(&icon, b, ink, w, cx),
                )
                .absolute()
                .left(px((width - icon_size) / 2. - 2.))
                .top(px(y - icon_size / 2. - 2.))
                .size(px(icon_size)),
            )
            .when_some(slot.item.badge.clone(), |b, badge| {
                let dot: Hsla = rgb(if p.dark { 0xbebebe } else { 0x373737 }).into();
                let rim: Hsla = rgb(if p.dark { 0x191919 } else { 0xfcfcfc }).into();
                b.child(
                    div()
                        .absolute()
                        .left(px(width / 2. + 3.))
                        .top(px(y - 18.))
                        .min_w(px(16.))
                        .h(px(16.))
                        .px(px(4.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded_full()
                        .bg(dot)
                        .shadow(vec![
                            BoxShadow::new(
                                px(0.),
                                px(0.),
                                rim.opacity(if p.dark { 0.58 } else { 0.56 }),
                            )
                            .spread_radius(px(1.5)),
                        ])
                        .text_size(px(10.))
                        .line_height(px(13.))
                        .text_color(if p.dark { black() } else { white() })
                        .child(badge),
                )
            })
            .when(!action && motion.compact < 0.995, |b| {
                b.child(
                    div()
                        .absolute()
                        .left_0()
                        .right_0()
                        .top(px(37. + 5. * motion.compact))
                        .h(px(16.))
                        .text_size(px(12.))
                        .line_height(px(16.))
                        .text_color(ink)
                        .text_center()
                        .truncate()
                        .opacity(1. - ease(motion.compact))
                        .child(slot.item.label.clone()),
                )
            })
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |s, e: &MouseDownEvent, _, cx| {
                    s.begin(i, e.position, false, cx);
                }),
            )
            .on_click(cx.listener(move |s, _, w, cx| {
                if !std::mem::take(&mut s.suppress_click) {
                    s.choose(i, false, w, cx);
                }
            }))
            .on_key_down(cx.listener(move |s, e: &KeyDownEvent, w, cx| s.key(i, e, w, cx)))
    }
}

impl Render for GlassTabBar {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(handle) = &self.scroll {
            self.observe_scroll_offset(-handle.offset().y, cx);
        }
        let motion = self.motion_frame(w, cx);
        let p = Palette::new(self.solid, self.increased_contrast, cx);
        let room = self.room.get();
        let width = if room > px(0.) {
            room
        } else {
            w.viewport_size().width
        };
        let l = Layout::new(
            width.into(),
            self.tabs.len(),
            self.action.is_some(),
            motion.compact,
            UiDirection::current(cx) == UiDirection::Rtl,
        );
        self.layout = l;
        let backdrop = self.backdrop.as_ref().and_then(|sample| sample());
        let bounds = self.bounds.clone();
        let move_pointer =
            cx.listener(|s, e: &MouseMoveEvent, w, cx| s.move_drag(e.position, false, w, cx));
        let release_pointer = cx.listener(|s, e: &MouseUpEvent, w, cx| {
            if e.button == MouseButton::Left {
                s.move_drag(e.position, false, w, cx);
                s.finish(false, w, cx);
            }
        });
        let scroll_handle = self.scroll.clone();
        let scroll_changed = cx.listener(|_, _: &ScrollEvent, _, cx| cx.notify());
        let art = canvas(
            move |b, _, _| bounds.set(b),
            move |b, _, w, _| {
                let section = |left, width| {
                    Bounds::new(
                        b.origin + point(px(left), px(0.)),
                        size(px(width), px(l.height)),
                    )
                };
                if l.track > 0. {
                    capsule(section(l.track_left(), l.track), backdrop.as_ref(), p, w);
                    let width = l.slot * (1. + 0.15 * motion.stretch);
                    let height = l.height - 2. * INSET + 6. * motion.lift;
                    let x = l.slot_center(motion.lens_index);
                    let y = (l.height - height) / 2. - 3. * motion.lift;
                    let indicator = Bounds::new(
                        b.origin + point(px(x - width / 2.), px(y)),
                        size(px(width), px(height)),
                    );
                    lens(
                        indicator,
                        backdrop.as_ref(),
                        p,
                        motion.lens_alpha,
                        motion.lift,
                        w,
                    );
                }
                if l.action > 0. {
                    let action = section(l.action_left(), l.action);
                    capsule(action, backdrop.as_ref(), p, w);
                    lens(action.inset(px(INSET)), None, p, motion.action, 0., w);
                }
                w.on_mouse_event(move |e: &MouseMoveEvent, phase, w, cx| {
                    if phase.capture() {
                        move_pointer(e, w, cx);
                    }
                });
                w.on_mouse_event(move |e: &MouseUpEvent, phase, w, cx| {
                    if phase.capture() {
                        release_pointer(e, w, cx);
                    }
                });
                if let Some(handle) = scroll_handle {
                    w.on_mouse_event(move |e: &ScrollEvent, phase, w, cx| {
                        if phase.capture() && handle.bounds().contains(&e.position) {
                            scroll_changed(e, w, cx);
                        }
                    });
                }
            },
        )
        .absolute()
        .inset_0();

        let tabs = div()
            .id(self.id.clone())
            .debug_selector(|| "glass-tabbar-list".into())
            .role(Role::TabList)
            .aria_label(self.label.clone())
            .relative()
            .w(px(l.width()))
            .h(px(l.height))
            .flex_none()
            .child(art)
            .child(self.touch.listen(cx, |s, e, w, cx| match e.phase {
                TouchPhase::Started => {
                    let x = f32::from(e.start_position.x - s.bounds.get().left());
                    if let Some(i) = s.layout.hit(x) {
                        s.begin(i, e.start_position, true, cx);
                    }
                }
                TouchPhase::Moved => s.move_drag(e.position, true, w, cx),
                TouchPhase::Ended => {
                    s.move_drag(e.position, true, w, cx);
                    s.finish(false, w, cx);
                }
                TouchPhase::Cancelled => s.finish(true, w, cx),
            }))
            .children((0..self.len()).map(|i| self.render_slot(i, p, motion, cx)));
        let room = self.room.clone();
        let shell = div()
            .relative()
            .w_full()
            .h(px(l.height))
            .on_children_prepainted(move |children, w, _| {
                if let Some(b) = children.first()
                    && room.replace(b.size.width) != b.size.width
                {
                    w.request_animation_frame();
                }
            })
            .child(
                div()
                    .id("glass-tabbar-nav")
                    .role(Role::Navigation)
                    .aria_label(self.label.clone())
                    .w_full()
                    .h_full()
                    .flex()
                    .justify_center()
                    .child(tabs),
            );
        match &self.style {
            Some(style) => style(shell),
            None => shell,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        GlassTabBar, GlassTabBarCompactChanged, GlassTabBarItem, GlassTabBarValueChanged,
        GlassTabIcon,
    };
    use crate::foundation::Theme;
    use gpui::{
        Context, Entity, IntoElement, Modifiers, MouseButton, Render, ScrollHandle, TestAppContext,
        TouchEvent, TouchId, TouchPhase, VisualTestContext, Window, div, point, prelude::*, px,
    };
    use std::{cell::RefCell, rc::Rc, time::Duration};

    fn bar(cx: &mut Context<GlassTabBar>) -> GlassTabBar {
        GlassTabBar::new(
            vec![
                GlassTabBarItem::new("home", "Home", GlassTabIcon::Home),
                GlassTabBarItem::new("explore", "Explore", GlassTabIcon::Explore),
                GlassTabBarItem::new("saved", "Saved", GlassTabIcon::Saved).with_badge(3),
                GlassTabBarItem::new("profile", "Profile", GlassTabIcon::Profile),
            ],
            cx,
        )
        .with_action(
            GlassTabBarItem::new("search", "Search", GlassTabIcon::Search),
            cx,
        )
    }

    fn tick(w: &mut VisualTestContext, ms: u64) {
        w.executor().advance_clock(Duration::from_millis(ms));
        w.run_until_parked();
        w.update(|w, cx| {
            w.simulate_next_frame(cx);
            w.refresh();
        });
        w.run_until_parked();
    }

    #[test]
    fn keyboard_wraps_through_action_and_enter_reexpands_the_compact_bar() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) = app.add_window_view(|_, cx| bar(cx));
        tick(w, 0);
        let main = w.debug_bounds("Home").unwrap();
        let search = w.debug_bounds("Search").unwrap();
        assert_eq!(main.size.width, px(78.));
        assert_eq!(main.size.height, px(62.));
        assert_eq!(search.size.width, px(62.));
        assert!(w.debug_bounds("Saved, 3 new").is_some());
        w.update(|w, cx| {
            w.focus_next(cx);
            assert!(view.read(cx).tabs[0].focus.is_focused(w));
        });
        tick(w, 0);
        w.simulate_keystrokes("left");
        tick(w, 800);
        w.update(|w, cx| {
            let bar = view.read(cx);
            assert_eq!(bar.value(), Some("search"));
            assert!(bar.action.as_ref().unwrap().focus.is_focused(w));
            assert_eq!(bar.opacity.value(cx), 0.);
        });
        w.simulate_keystrokes("right");
        w.update(|_, cx| {
            view.update(cx, |b, cx| {
                b.observe_scroll_offset(px(0.), cx);
                b.observe_scroll_offset(px(50.), cx);
            })
        });
        tick(w, 800);
        assert_eq!(w.debug_bounds("Home").unwrap().size.width, px(48.));
        assert_eq!(w.debug_bounds("Home").unwrap().size.height, px(48.));
        w.simulate_keystrokes("enter");
        tick(w, 800);
        assert_eq!(w.debug_bounds("Home").unwrap().size.height, px(62.));
        assert_eq!(
            w.update(|_, cx| view.read(cx).value().unwrap().to_owned()),
            "home"
        );
    }

    #[test]
    fn controlled_drag_commits_once_and_interrupted_lens_preserves_position() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) = app.add_window_view(|_, cx| bar(cx).with_value("home").with_compact(false));
        let values = Rc::new(RefCell::new(Vec::new()));
        let compacts = Rc::new(RefCell::new(Vec::new()));
        let _values = w.update(|_, cx| {
            let values = values.clone();
            cx.subscribe(&view, move |_, event: &GlassTabBarValueChanged, _| {
                values.borrow_mut().push(event.0.clone())
            })
        });
        let _compacts = w.update(|_, cx| {
            let compacts = compacts.clone();
            cx.subscribe(&view, move |_, event: &GlassTabBarCompactChanged, _| {
                compacts.borrow_mut().push(event.0)
            })
        });
        let home = w.debug_bounds("Home").unwrap().center();
        let saved = w.debug_bounds("Saved, 3 new").unwrap().center();
        w.simulate_mouse_down(home, MouseButton::Left, Modifiers::default());
        w.simulate_mouse_move(saved, Some(MouseButton::Left), Modifiers::default());
        w.simulate_mouse_up(saved, MouseButton::Left, Modifiers::default());
        assert_eq!(values.borrow().as_slice(), &["saved"]);
        w.update(|_, cx| {
            view.update(cx, |bar, cx| {
                assert_eq!(bar.value(), Some("home"));
                bar.set_value("saved", cx);
                bar.observe_scroll_offset(px(0.), cx);
                bar.observe_scroll_offset(px(4.), cx);
                bar.observe_scroll_offset(px(30.), cx);
                bar.observe_scroll_offset(px(70.), cx);
                assert!(!bar.is_compact());
            })
        });
        assert_eq!(compacts.borrow().as_slice(), &[true]);
        tick(w, 65);
        w.update(|_, cx| {
            view.update(cx, |bar, cx| {
                let before = bar.lens.value(cx);
                bar.set_value("home", cx);
                assert!((before - bar.lens.value(cx)).abs() < 0.001);
                bar.set_compact(true, cx);
                bar.observe_scroll_offset(px(40.), cx);
            })
        });
        assert_eq!(compacts.borrow().as_slice(), &[true, false]);
    }

    struct Feed {
        bar: Entity<GlassTabBar>,
        scroll: ScrollHandle,
    }

    impl Render for Feed {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
                .relative()
                .w(px(288.))
                .h(px(400.))
                .child(
                    div()
                        .id("feed")
                        .size_full()
                        .track_scroll(&self.scroll)
                        .overflow_y_scroll()
                        .child(div().h(px(900.))),
                )
                .child(self.bar.clone())
        }
    }

    #[test]
    fn vertical_touch_pans_and_reduced_motion_snaps_the_narrow_bar() {
        let mut app = TestAppContext::single();
        app.update(|cx| {
            crate::init(cx);
            Theme::global_mut(cx).reduced_motion = true;
        });
        let (view, w) = app.add_window_view(|_, cx| {
            let scroll = ScrollHandle::new();
            let bar = cx.new(|cx| {
                bar(cx)
                    .with_scroll_handle(scroll.clone())
                    .with_style(|d| d.absolute().bottom(px(12.)))
            });
            Feed { bar, scroll }
        });
        tick(w, 0);
        let home = w.debug_bounds("Home").unwrap().center();
        let vertical = home - point(px(2.), px(56.));
        for (phase, position) in [
            (TouchPhase::Started, home),
            (TouchPhase::Moved, vertical),
            (TouchPhase::Ended, vertical),
        ] {
            w.simulate_event(TouchEvent {
                id: TouchId(1),
                phase,
                position,
                ..Default::default()
            });
            tick(w, 16);
        }
        w.update(|_, cx| {
            let feed = view.read(cx);
            let bar = feed.bar.read(cx);
            assert!(feed.scroll.offset().y < px(0.));
            assert!(bar.is_compact());
            assert_eq!(bar.value(), Some("home"));
            assert_eq!(bar.size.value(cx), 1.);
            assert_eq!(bar.lens.value(cx), 0.);
        });
        assert!(w.debug_bounds("glass-tabbar-list").unwrap().size.width <= px(288.));
        assert_eq!(w.debug_bounds("Search").unwrap().size.width, px(48.));
    }
}
