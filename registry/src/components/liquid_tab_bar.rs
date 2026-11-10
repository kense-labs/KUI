//! Section tabs with a draggable lens that inverts the content beneath it.
use crate::{
    foundation::{Theme, ThemeMode},
    primitives::{
        control::UiDirection,
        direct_touch::Contact,
        family::{PhaseClock, disabled_motion, ease},
        glyph::Glyph,
        presentation::tr,
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};

const SLOT: f32 = 44.;
const HEIGHT: f32 = 48.;
const INSET: f32 = 4.;
const GAP: f32 = 2.;
const TITLE_X: f32 = 40.;
const TITLE_TRACKING: f32 = -0.14;
const LABEL_GAP: f32 = 10.;
const MORPH_SECONDS: f32 = 0.42;
const BOUNCE: f32 = 0.16;
const FLICK_SPEED: f32 = 360.;

/// Determines which captions open beside the icons.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LiquidTabLabels {
    #[default]
    Active,
    Always,
    None,
}

/// A built-in, KUI, or asset-backed monochrome icon.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LiquidTabIcon {
    Home,
    Bookmark,
    Inbox,
    Profile,
    Glyph(Glyph),
    Svg(SharedString),
}

/// A section with a stable value and an optional rolling badge.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiquidTabItem {
    pub value: SharedString,
    pub label: SharedString,
    pub icon: LiquidTabIcon,
    pub active_icon: Option<LiquidTabIcon>,
    pub badge: u32,
    pub badge_label: Option<SharedString>,
}

impl LiquidTabItem {
    pub fn new(
        value: impl Into<SharedString>,
        label: impl Into<SharedString>,
        icon: LiquidTabIcon,
    ) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            icon,
            active_icon: None,
            badge: 0,
            badge_label: None,
        }
    }

    pub fn with_active_icon(mut self, icon: LiquidTabIcon) -> Self {
        self.active_icon = Some(icon);
        self
    }

    pub fn with_badge(mut self, count: u32) -> Self {
        self.badge = count;
        self
    }

    pub fn with_badge_label(mut self, label: impl Into<SharedString>) -> Self {
        self.badge_label = Some(label.into());
        self
    }
}

/// A selection request. The host supplies the accepted value with `set_value`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiquidTabChanged(pub SharedString);

/// Stable identifier for a tab within the given bar.
pub fn liquid_tab_id(bar: &str, value: &str) -> SharedString {
    format!("{bar}-tab-{value}").into()
}

/// Stable identifier for the host panel belonging to a tab.
pub fn liquid_panel_id(bar: &str, value: &str) -> SharedString {
    format!("{bar}-panel-{value}").into()
}

// One damped trajectory per axis keeps the lens a capsule throughout a retarget.
#[derive(Clone, Copy)]
struct Morph {
    from: f32,
    to: f32,
    speed: f32,
    clock: PhaseClock,
}

impl Morph {
    fn at(value: f32) -> Self {
        Self {
            from: value,
            to: value,
            speed: 0.,
            clock: PhaseClock::default(),
        }
    }

    fn sample(self, cx: &App) -> (f32, f32, bool) {
        let t = self.clock.age(cx);
        if t >= MORPH_SECONDS || disabled_motion(cx) {
            return (self.to, 0., false);
        }
        let ratio = 1. - BOUNCE;
        let sine = (1. - ratio * ratio).sqrt();
        let decay = (ratio / (0.001 * sine)).ln() / MORPH_SECONDS;
        let frequency = decay * sine / ratio;
        let offset = self.from - self.to;
        let b = (self.speed + decay * offset) / frequency;
        let (sin, cos) = (frequency * t).sin_cos();
        let wave = offset * cos + b * sin;
        let envelope = (-decay * t).exp();
        (
            self.to + envelope * wave,
            envelope * (frequency * (b * cos - offset * sin) - decay * wave),
            true,
        )
    }

    fn retarget(&mut self, value: f32, cx: &App) {
        if disabled_motion(cx) {
            *self = Self::at(value);
        } else if (value - self.to).abs() > 0.001 {
            let (from, speed, _) = self.sample(cx);
            *self = Self {
                from,
                to: value,
                speed,
                clock: PhaseClock::default(),
            };
            self.clock.restart(cx);
        }
    }

    fn value(self, w: &Window, cx: &App) -> f32 {
        let (value, _, moving) = self.sample(cx);
        if moving {
            w.request_animation_frame();
        }
        value
    }
}

struct Slot {
    item: LiquidTabItem,
    focus: FocusHandle,
    caption_width: Option<Morph>,
    badge_digits: Vec<Morph>,
    badge_visibility: Morph,
}

impl Slot {
    fn new(item: LiquidTabItem, cx: &mut Context<LiquidTabBar>) -> Self {
        let badge_digits = digits(item.badge).map(|d| Morph::at(d as f32)).collect();
        let badge_visibility = Morph::at(if item.badge > 0 { 1. } else { 0. });
        Self {
            item,
            focus: cx.focus_handle().tab_stop(true),
            caption_width: None,
            badge_digits,
            badge_visibility,
        }
    }

    fn update_item(&mut self, item: LiquidTabItem, cx: &App) {
        if self.item.badge != item.badge {
            self.badge_visibility
                .retarget(if item.badge == 0 { 0. } else { 1. }, cx);
            if item.badge > 0 {
                let next: Vec<_> = digits(item.badge).collect();
                if self.badge_digits.len() != next.len() || self.item.badge == 0 {
                    self.badge_digits = next.into_iter().map(|d| Morph::at(d as f32)).collect();
                } else {
                    for (wheel, next) in self.badge_digits.iter_mut().zip(next) {
                        let last = wheel.to as i32;
                        let change = next - last.rem_euclid(10);
                        let step = if item.badge > self.item.badge {
                            change.rem_euclid(10)
                        } else {
                            -(-change).rem_euclid(10)
                        };
                        wheel.retarget((last + step) as f32, cx);
                    }
                }
            }
        }
        self.item = item;
    }

    fn accessible_name(&self, cx: &App) -> SharedString {
        if self.item.badge == 0 {
            return self.item.label.clone();
        }
        let count = self.item.badge;
        let suffix = self.item.badge_label.clone().unwrap_or_else(|| {
            if tr(cx, "en", "zh") == "zh" {
                format!("{count} 条通知").into()
            } else {
                format!("{count} notifications").into()
            }
        });
        format!("{}, {suffix}", self.item.label).into()
    }
}

fn digits(value: u32) -> impl Iterator<Item = i32> {
    value
        .to_string()
        .into_bytes()
        .into_iter()
        .map(|b| i32::from(b - b'0'))
}

fn badge_width(digits: usize) -> f32 {
    10. + 7. * digits as f32
}

fn caption_clearance(digits: usize) -> f32 {
    if digits == 0 {
        0.
    } else {
        25. + badge_width(digits) + 4. - TITLE_X
    }
}

#[derive(Clone, Copy, Default)]
struct Span {
    left: f32,
    width: f32,
}

impl Span {
    fn center(self) -> f32 {
        self.left + self.width / 2.
    }
    fn contains(self, x: f32) -> bool {
        x >= self.left && x <= self.left + self.width
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum GestureAxis {
    Pending,
    Horizontal,
    Vertical,
}

struct Drag {
    tab: usize,
    start: Point<Pixels>,
    last_x: f32,
    last_time: f32,
    speed: f32,
    center: f32,
    width: f32,
    clock: PhaseClock,
    axis: GestureAxis,
    touch: bool,
}

/// Controlled tab list with one moving, shape-preserving selection.
pub struct LiquidTabBar {
    id: SharedString,
    label: Option<SharedString>,
    slots: Vec<Slot>,
    value: SharedString,
    labels: LiquidTabLabels,
    roving: usize,
    spans: Vec<Span>,
    content_bounds: Rc<Cell<Bounds<Pixels>>>,
    viewport_bounds: Rc<Cell<Bounds<Pixels>>>,
    scroll: ScrollHandle,
    lens: Option<[Morph; 2]>,
    shown: Option<usize>,
    leaving: Option<usize>,
    selection_clock: PhaseClock,
    label_reveal: Morph,
    drag_lift: Morph,
    hover: Option<usize>,
    touch_clock: PhaseClock,
    contact: Contact,
    drag: Option<Drag>,
    suppress_click: bool,
}

impl EventEmitter<LiquidTabChanged> for LiquidTabBar {}

impl LiquidTabBar {
    pub fn new(
        tabs: Vec<LiquidTabItem>,
        value: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> Self {
        let value = value.into();
        let roving = tabs.iter().position(|tab| tab.value == value).unwrap_or(0);
        Self {
            id: format!("liquid-tabs-{}", cx.entity_id()).into(),
            label: None,
            slots: tabs.into_iter().map(|tab| Slot::new(tab, cx)).collect(),
            value,
            labels: LiquidTabLabels::Active,
            roving,
            spans: vec![],
            content_bounds: Default::default(),
            viewport_bounds: Default::default(),
            scroll: ScrollHandle::new(),
            lens: None,
            shown: None,
            leaving: None,
            selection_clock: PhaseClock::default(),
            label_reveal: Morph::at(1.),
            drag_lift: Morph::at(0.),
            hover: None,
            touch_clock: PhaseClock::default(),
            contact: Contact::default(),
            drag: None,
            suppress_click: false,
        }
    }

    pub fn with_id(mut self, id: impl Into<SharedString>) -> Self {
        self.id = id.into();
        self
    }
    pub fn with_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }
    pub fn with_labels(mut self, labels: LiquidTabLabels) -> Self {
        self.labels = labels;
        self
    }
    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn set_value(&mut self, value: impl Into<SharedString>, cx: &mut Context<Self>) {
        let value = value.into();
        if self.value != value {
            self.value = value;
            if let Some(i) = self.selected() {
                self.roving = i;
            }
            self.drag = None;
            cx.notify();
        }
    }

    pub fn set_labels(&mut self, labels: LiquidTabLabels, cx: &mut Context<Self>) {
        if self.labels != labels {
            self.labels = labels;
            cx.notify();
        }
    }

    /// Replace the options while retaining badge motion and focus by value.
    pub fn set_tabs(&mut self, tabs: Vec<LiquidTabItem>, cx: &mut Context<Self>) {
        let roving = self.slots.get(self.roving).map(|s| s.item.value.clone());
        let mut old = std::mem::take(&mut self.slots);
        self.slots = tabs
            .into_iter()
            .map(|tab| {
                if let Some(i) = old.iter().position(|s| s.item.value == tab.value) {
                    let mut slot = old.remove(i);
                    slot.update_item(tab, cx);
                    slot
                } else {
                    Slot::new(tab, cx)
                }
            })
            .collect();
        self.roving = roving
            .and_then(|id| self.slots.iter().position(|s| s.item.value == id))
            .or_else(|| self.selected())
            .unwrap_or(0);
        self.shown = self.selected();
        self.leaving = None;
        self.drag = None;
        self.hover = None;
        cx.notify();
    }

    fn selected(&self) -> Option<usize> {
        self.slots.iter().position(|s| s.item.value == self.value)
    }

    fn choose(&mut self, i: usize, w: &mut Window, cx: &mut Context<Self>) {
        self.roving = i;
        self.slots[i].focus.focus(w, cx);
        if let Some(span) = self.spans.get(i) {
            let room = f32::from(self.viewport_bounds.get().size.width);
            let offset = -f32::from(self.scroll.offset().x);
            let target = if span.left < offset {
                (span.left - INSET).max(0.)
            } else if span.left + span.width > offset + room {
                (span.left + span.width + INSET - room).max(0.)
            } else {
                offset
            };
            if room > 0. && (target - offset).abs() > 0.1 {
                self.scroll.set_offset(point(px(-target), px(0.)));
            }
        }
        if self.selected() != Some(i) {
            cx.emit(LiquidTabChanged(self.slots[i].item.value.clone()));
        }
        cx.notify();
    }

    fn key(&mut self, from: usize, e: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        let n = self.slots.len();
        let to = match e.keystroke.key.as_str() {
            "left" | "right" => {
                let step = UiDirection::current(cx).inline_step(if e.keystroke.key == "left" {
                    -1
                } else {
                    1
                });
                (from as isize + step).rem_euclid(n as isize) as usize
            }
            "home" => 0,
            "end" => n - 1,
            "enter" | "space" if !e.is_held => from,
            "escape" if self.drag.is_some() => {
                self.end(true, w, cx);
                cx.stop_propagation();
                return;
            }
            _ => return,
        };
        self.choose(to, w, cx);
        w.prevent_default();
        cx.stop_propagation();
    }

    fn begin(&mut self, at: Point<Pixels>, touch: bool, cx: &mut Context<Self>) {
        self.suppress_click = false;
        if touch {
            self.touch_clock.restart(cx);
            self.hover = None;
        }
        let x = f32::from(at.x - self.content_bounds.get().left());
        let Some(i) = self.spans.iter().position(|span| span.contains(x)) else {
            return;
        };
        if !touch && self.selected() != Some(i) {
            return;
        }
        let (center, width) = self
            .lens
            .map_or((self.spans[i].center(), self.spans[i].width), |m| {
                (m[0].sample(cx).0, m[1].sample(cx).0)
            });
        let mut clock = PhaseClock::default();
        clock.restart(cx);
        self.drag = Some(Drag {
            tab: i,
            start: at,
            last_x: f32::from(at.x),
            last_time: 0.,
            speed: 0.,
            center,
            width,
            clock,
            axis: GestureAxis::Pending,
            touch,
        });
        cx.notify();
    }

    fn moved(&mut self, at: Point<Pixels>, touch: bool, cx: &mut Context<Self>) {
        let selected = self.selected();
        let Some(drag) = self.drag.as_mut().filter(|d| d.touch == touch) else {
            return;
        };
        let dx = f32::from(at.x - drag.start.x);
        let dy = f32::from(at.y - drag.start.y);
        if drag.axis == GestureAxis::Pending && dx.abs().max(dy.abs()) >= 4. {
            drag.axis = if (touch && dy.abs() > dx.abs()) || selected != Some(drag.tab) {
                GestureAxis::Vertical
            } else {
                GestureAxis::Horizontal
            };
            self.suppress_click = true;
        }
        if drag.axis != GestureAxis::Horizontal {
            return;
        }
        let now = drag.clock.age(cx);
        let dt = now - drag.last_time;
        let x = f32::from(at.x);
        if dt > 0.004 && x != drag.last_x {
            drag.speed = ((x - drag.last_x) / dt).clamp(-2200., 2200.);
            drag.last_time = now;
        }
        drag.last_x = x;
        cx.notify();
    }

    fn drag_center(&self, drag: &Drag) -> f32 {
        let x = drag.center + drag.last_x - f32::from(drag.start.x);
        let (min, max) = self
            .spans
            .iter()
            .map(|s| s.center())
            .fold((f32::INFINITY, f32::NEG_INFINITY), |(a, b), x| {
                (a.min(x), b.max(x))
            });
        let limit = x.clamp(min, max);
        limit + (x - limit) / (3. + (x - limit).abs() / 15.)
    }

    fn end(&mut self, cancelled: bool, w: &mut Window, cx: &mut Context<Self>) {
        let Some(drag) = self.drag.take() else {
            return;
        };
        if drag.axis == GestureAxis::Horizontal {
            let x = self.drag_center(&drag);
            let velocity = if drag.clock.age(cx) - drag.last_time < 0.09 {
                drag.speed
            } else {
                0.
            };
            self.lens = Some([Morph::at(x), Morph::at(drag.width)]);
            if !cancelled {
                let projected = x + velocity * 0.14;
                let closest = self
                    .spans
                    .iter()
                    .enumerate()
                    .min_by(|(_, a), (_, b)| {
                        (a.center() - projected)
                            .abs()
                            .total_cmp(&(b.center() - projected).abs())
                    })
                    .map_or(drag.tab, |(i, _)| i);
                let direction =
                    UiDirection::current(cx).inline_step(if velocity >= 0. { 1 } else { -1 });
                let target = if closest == drag.tab && velocity.abs() >= FLICK_SPEED {
                    drag.tab
                        .saturating_add_signed(direction)
                        .min(self.slots.len() - 1)
                } else {
                    closest
                };
                self.choose(target, w, cx);
            }
        } else if drag.touch && !cancelled && drag.axis == GestureAxis::Pending {
            self.suppress_click = true;
            self.choose(drag.tab, w, cx);
        }
        cx.notify();
    }
}

#[derive(Clone, Copy)]
struct Ink {
    track: Hsla,
    rim: Hsla,
    idle: Hsla,
    lens: Hsla,
    active: Hsla,
    badge: Hsla,
    focus: Hsla,
}

impl Ink {
    fn new(cx: &App) -> Self {
        let dark = Theme::global(cx).mode == ThemeMode::Dark;
        let colors = if dark {
            [0x242424, 0x222222, 0xb5b5b5, 0xf2f2f2, 0x141414, 0xf2f2f2]
        } else {
            [0xf8f8f8, 0xf3f3f3, 0x575757, 0x0b0b0b, 0xffffff, 0x0b0b0b]
        }
        .map(|v| Hsla::from(rgb(v)));
        let [track, rim, idle, lens, active, badge] = colors;
        Self {
            track,
            rim,
            idle,
            lens,
            active,
            badge,
            focus: rgb(if dark { 0xe1e1e1 } else { 0x1f1f1f })
                .opacity(0.72)
                .into(),
        }
    }
}

struct PaintTab {
    item: LiquidTabItem,
    span: Span,
    title: f32,
    base_title: bool,
    badge: (f32, Vec<f32>),
}

fn label_line(text: SharedString, em: f32, ink: Hsla, w: &Window) -> ShapedLine {
    let mut run = w.text_style().to_run(text.len());
    run.font.weight = FontWeight::MEDIUM;
    run.color = ink;
    w.text_system().shape_line(text, px(em), &[run], None)
}

fn caption_width(line: &ShapedLine) -> f32 {
    let tracking = if line.text.is_ascii() {
        TITLE_TRACKING * line.text.len() as f32
    } else {
        0.
    };
    f32::from(line.width) + tracking
}

fn paint_caption(
    line: &ShapedLine,
    origin: Point<Pixels>,
    ink: Hsla,
    w: &mut Window,
    cx: &mut App,
) {
    if !line.text.is_ascii() {
        let _ = line.paint(origin, px(14.), TextAlign::Left, None, w, cx);
        return;
    }
    let baseline = px(7.) + (line.ascent - line.descent) / 2.;
    let mut glyph_index = 0;
    for run in &line.runs {
        for glyph in &run.glyphs {
            let at =
                origin + glyph.position + point(px(TITLE_TRACKING * glyph_index as f32), baseline);
            let _ = w.paint_glyph(at, run.font_id, glyph.id, line.font_size, ink);
            glyph_index += 1;
        }
    }
}

// Most marks fit inside the capsule in one draw. Only marks crossing its rounded edge
// need short horizontal scissors to invert just the pixels below the lens.
fn through_lens(
    lens: Bounds<Pixels>,
    area: Bounds<Pixels>,
    w: &mut Window,
    mut paint: impl FnMut(&mut Window),
) {
    let clip = lens.intersect(&area);
    if clip.size.width <= px(0.) || clip.size.height <= px(0.) {
        return;
    }
    let radius = f32::from(lens.size.height / 2.);
    let inset = |y: Pixels| {
        let dy = f32::from(y - lens.center().y).abs().min(radius);
        px(radius - (radius * radius - dy * dy).sqrt())
    };
    let worst = inset(clip.top()).max(inset(clip.bottom()));
    if area.left() >= lens.left() + worst
        && area.right() <= lens.right() - worst
        && area.top() >= lens.top()
        && area.bottom() <= lens.bottom()
    {
        paint(w);
        return;
    }
    let mut top = clip.top();
    while top < clip.bottom() {
        let bottom = (top + px(1.)).min(clip.bottom());
        let dx = inset((top + bottom) / 2.);
        let left = (lens.left() + dx).max(clip.left());
        let right = (lens.right() - dx).min(clip.right());
        if right > left {
            let bounds = Bounds::from_corners(point(left, top), point(right, bottom));
            w.with_content_mask(Some(ContentMask { bounds }), |w| paint(w));
        }
        top = bottom;
    }
}

fn paint_mark(
    icon: &LiquidTabIcon,
    area: Bounds<Pixels>,
    ink: Hsla,
    filled: bool,
    w: &mut Window,
    cx: &App,
) {
    let at = |x, y| area.origin + point(px(x * 20. / 24.), px(y * 20. / 24.));
    let stroke = px(1.75 * 20. / 24.);
    let path = |geometry: fn(&mut PathBuilder, &dyn Fn(f32, f32) -> Point<Pixels>),
                closed: bool,
                w: &mut Window| {
        if filled && closed {
            let mut p = PathBuilder::fill();
            geometry(&mut p, &at);
            p.close();
            if let Ok(p) = p.build() {
                w.paint_path(p, ink);
            }
        }
        let mut p = PathBuilder::stroke(stroke);
        geometry(&mut p, &at);
        if closed {
            p.close();
        }
        if let Ok(p) = p.build() {
            w.paint_path(p, ink);
        }
    };
    match icon {
        LiquidTabIcon::Svg(asset) => {
            let _ = w.paint_svg(
                area,
                asset.clone(),
                None,
                TransformationMatrix::unit(),
                ink,
                cx,
            );
        }
        LiquidTabIcon::Home => {
            path(
                |p, at| {
                    p.move_to(at(3., 10.));
                    p.line_to(at(10.7, 2.9));
                    p.curve_to(at(13.3, 2.9), at(12., 1.7));
                    p.line_to(at(21., 10.));
                    p.line_to(at(21., 19.));
                    p.curve_to(at(19., 21.), at(21., 21.));
                    p.line_to(at(5., 21.));
                    p.curve_to(at(3., 19.), at(3., 21.));
                },
                true,
                w,
            );
            if !filled {
                path(
                    |p, at| {
                        p.move_to(at(9., 21.));
                        p.line_to(at(9., 13.));
                        p.curve_to(at(10., 12.), at(9., 12.));
                        p.line_to(at(14., 12.));
                        p.curve_to(at(15., 13.), at(15., 12.));
                        p.line_to(at(15., 21.));
                    },
                    false,
                    w,
                );
            }
        }
        LiquidTabIcon::Bookmark => path(
            |p, at| {
                p.move_to(at(5., 5.));
                p.curve_to(at(7., 3.), at(5., 3.));
                p.line_to(at(17., 3.));
                p.curve_to(at(19., 5.), at(19., 3.));
                p.line_to(at(19., 19.8));
                p.curve_to(at(17.5, 20.7), at(19., 21.4));
                p.line_to(at(12., 17.6));
                p.line_to(at(6.5, 20.7));
                p.curve_to(at(5., 19.8), at(5., 21.4));
            },
            true,
            w,
        ),
        LiquidTabIcon::Inbox => path(
            |p, at| {
                p.move_to(at(4.1, 17.));
                p.cubic_bezier_to(at(4., 6.), at(1.8, 13.8), at(1.6, 9.2));
                p.cubic_bezier_to(at(21.9, 11.9), at(9.9, -2.5), at(22.2, 2.7));
                p.cubic_bezier_to(at(7.4, 21.), at(22.1, 19.8), at(14.8, 24.4));
                p.line_to(at(3., 22.));
                p.curve_to(at(2.2, 20.8), at(1.8, 22.2));
            },
            true,
            w,
        ),
        LiquidTabIcon::Profile => {
            path(
                |p, at| {
                    let radius = point(px(20. / 6.), px(20. / 6.));
                    p.move_to(at(16., 7.));
                    p.arc_to(radius, px(0.), false, true, at(8., 7.));
                    p.arc_to(radius, px(0.), false, true, at(16., 7.));
                },
                true,
                w,
            );
            path(
                |p, at| {
                    p.move_to(at(5., 21.));
                    p.line_to(at(5., 19.));
                    p.curve_to(at(9., 15.), at(5., 15.));
                    p.line_to(at(15., 15.));
                    p.curve_to(at(19., 19.), at(19., 15.));
                    p.line_to(at(19., 21.));
                },
                filled,
                w,
            );
        }
        LiquidTabIcon::Glyph(glyph) => {
            for (i, line) in glyph.strokes().iter().enumerate() {
                let mut p = if filled && i == 0 {
                    PathBuilder::fill()
                } else {
                    PathBuilder::stroke(stroke)
                };
                for (j, &(x, y)) in line.iter().enumerate() {
                    if j == 0 {
                        p.move_to(at(x, y));
                    } else {
                        p.line_to(at(x, y));
                    }
                }
                if filled && i == 0 {
                    p.close();
                }
                if let Ok(p) = p.build() {
                    w.paint_path(p, ink);
                }
            }
        }
    }
}

impl PaintTab {
    fn paint(
        &self,
        origin: Point<Pixels>,
        lens: Option<Bounds<Pixels>>,
        colors: Ink,
        rtl: bool,
        w: &mut Window,
        cx: &mut App,
    ) {
        let inverse = lens.is_some();
        let fg = if inverse { colors.active } else { colors.idle };
        let x = self.span.left + if rtl { self.span.width - 32. } else { 12. };
        let icon_area = Bounds::new(origin + point(px(x), px(14.)), size(px(20.), px(20.)));
        let icon = if inverse {
            self.item.active_icon.as_ref().unwrap_or(&self.item.icon)
        } else {
            &self.item.icon
        };
        let mark = |w: &mut Window| {
            paint_mark(
                icon,
                icon_area,
                fg,
                inverse && self.item.active_icon.is_none(),
                w,
                cx,
            )
        };
        if let Some(lens) = lens {
            through_lens(lens, icon_area, w, mark);
        } else {
            mark(w);
        }

        let (alpha, digits) = &self.badge;
        if *alpha > 0.001 && !digits.is_empty() {
            let width = badge_width(digits.len());
            let bx = if rtl { x + 7. - width } else { x + 13. };
            let badge = Bounds::new(origin + point(px(bx), px(9.)), size(px(width), px(16.)));
            let ring = Bounds::new(
                badge.origin - point(px(2.), px(2.)),
                size(px(width + 4.), px(20.)),
            );
            let bg = if inverse { colors.active } else { colors.badge };
            let text = if inverse { colors.lens } else { colors.active };
            let mut badge_paint = |w: &mut Window| {
                w.paint_quad(
                    fill(
                        ring,
                        if inverse { colors.lens } else { colors.track }.opacity(*alpha),
                    )
                    .corner_radii(px(10.)),
                );
                w.paint_quad(fill(badge, bg.opacity(*alpha)).corner_radii(px(8.)));
                w.with_content_mask(Some(ContentMask { bounds: badge }), |w| {
                    for (i, &digit) in digits.iter().enumerate() {
                        let first = digit.floor();
                        for (d, offset) in
                            [(first, first - digit), (first + 1., first + 1. - digit)]
                        {
                            if offset.abs() >= 1. {
                                continue;
                            }
                            let line = label_line(
                                (d as i32).rem_euclid(10).to_string().into(),
                                11.,
                                text.opacity(*alpha),
                                w,
                            );
                            let at = badge.origin
                                + point(
                                    px(5. + i as f32 * 7.) + (px(7.) - line.width) / 2.,
                                    px(offset * 16.),
                                );
                            let _ = line.paint(at, px(16.), TextAlign::Left, None, w, cx);
                        }
                    }
                });
            };
            if let Some(lens) = lens {
                through_lens(lens, ring, w, badge_paint);
            } else {
                badge_paint(w);
            }
        }

        let clearance = caption_clearance(digits.len()) * *alpha;
        let offset = TITLE_X + clearance;
        let room = self.span.width - offset - 4.;
        if self.title < 0.001 || room < 0.5 || (!inverse && !self.base_title) {
            return;
        }
        let ink = fg.opacity(self.title);
        let line = label_line(self.item.label.clone(), 14., ink, w);
        let width = caption_width(&line);
        let start = self.span.left
            + if rtl {
                self.span.width - offset - width
            } else {
                offset
            };
        let slide = -4. * (1. - self.title) * if rtl { -1. } else { 1. };
        let origin = origin + point(px(start + slide), px(17.));
        let content = Bounds::new(origin, size(px(width + 1.), px(14.)));
        let room_left = if rtl {
            self.span.left + 4.
        } else {
            self.span.left + offset
        };
        let available = Bounds::new(
            point(origin.x + px(room_left - start - slide), origin.y),
            size(px(room), px(14.)),
        );
        let area = content.intersect(&available);
        let mut caption = |w: &mut Window| {
            w.with_content_mask(Some(ContentMask { bounds: area }), |w| {
                paint_caption(&line, origin, ink, w, cx);
            });
        };
        if let Some(lens) = lens {
            through_lens(lens, area, w, caption);
        } else {
            caption(w);
        }
    }
}

fn layout(widths: impl IntoIterator<Item = f32>, rtl: bool) -> (Vec<Span>, f32) {
    let mut end = INSET;
    let mut spans: Vec<_> = widths
        .into_iter()
        .map(|width| {
            let span = Span { left: end, width };
            end += width + GAP;
            span
        })
        .collect();
    let full = (end - GAP + INSET).max(INSET * 2.);
    if rtl {
        for span in &mut spans {
            span.left = full - span.left - span.width;
        }
    }
    (spans, full)
}

impl Render for LiquidTabBar {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let selected = self.selected();
        let rtl = UiDirection::current(cx).is_rtl();
        let reduced = disabled_motion(cx);
        let ink = Ink::new(cx);
        let mut target_widths = Vec::with_capacity(self.slots.len());
        let mut widths = Vec::with_capacity(self.slots.len());
        for (i, slot) in self.slots.iter_mut().enumerate() {
            let open = self.labels == LiquidTabLabels::Always
                || (self.labels == LiquidTabLabels::Active && selected == Some(i));
            let extra = if open {
                let line = label_line(slot.item.label.clone(), 14., ink.idle, w);
                let badge = if slot.item.badge == 0 {
                    0
                } else {
                    slot.badge_digits.len()
                };
                (caption_width(&line) + caption_clearance(badge)).round() + LABEL_GAP
            } else {
                0.
            };
            target_widths.push(SLOT + extra);
            let spring = slot.caption_width.get_or_insert(Morph::at(extra));
            spring.retarget(extra, cx);
            widths.push(SLOT + spring.value(w, cx).max(0.));
        }
        let (spans, full) = layout(widths, rtl);
        let (targets, _) = layout(target_widths, rtl);
        self.spans = spans;
        if self.shown != selected {
            self.leaving = self.shown;
            self.shown = selected;
            self.selection_clock.restart(cx);
            self.label_reveal = Morph::at(if self.lens.is_none() || reduced {
                1.
            } else {
                0.
            });
        }
        let dragging = self
            .drag
            .as_ref()
            .filter(|d| d.axis == GestureAxis::Horizontal)
            .map(|drag| (self.drag_center(drag), drag.width));
        self.drag_lift
            .retarget(f32::from(dragging.is_some() && !reduced), cx);
        let height = 40. - 2.5 * self.drag_lift.value(w, cx).clamp(0., 1.);
        let lens = selected.map(|i| {
            let target = targets[i];
            let hover = self
                .hover
                .filter(|&j| j != i && !reduced && dragging.is_none());
            let lean = hover.map_or(0., |j| {
                (targets[j].center() - target.center()).signum() * 3.
            });
            let m = self
                .lens
                .get_or_insert([Morph::at(target.center()), Morph::at(target.width)]);
            m[0].retarget(target.center() + lean, cx);
            m[1].retarget(target.width, cx);
            let mut center = m[0].value(w, cx);
            let mut width = m[1].value(w, cx).max(SLOT - 2.);
            if let Some((drag_center, drag_width)) = dragging {
                center = drag_center;
                width = drag_width;
            }
            if self.label_reveal.to == 0.
                && (center - target.center()).abs() <= 5.
                && dragging.is_none()
            {
                self.label_reveal.retarget(1., cx);
            }
            (center, width, height)
        });
        if selected.is_none() {
            self.lens = None;
        }
        let leaving =
            if self.labels == LiquidTabLabels::Active && self.leaving.is_some() && !reduced {
                1. - ease(self.selection_clock.time(0.10, w, cx) / 0.10)
            } else {
                0.
            };
        let appearing =
            self.label_reveal.value(w, cx).clamp(0., 1.) * (1. - f32::from(leaving > 0.02));
        let tabs: Vec<_> = self
            .slots
            .iter()
            .enumerate()
            .map(|(i, slot)| {
                let title = match self.labels {
                    LiquidTabLabels::Always => 1.,
                    LiquidTabLabels::Active if selected == Some(i) => appearing,
                    LiquidTabLabels::Active if self.leaving == Some(i) => leaving,
                    _ => 0.,
                };
                let opacity = slot.badge_visibility.value(w, cx).clamp(0., 1.);
                let digits = if opacity > 0.001 {
                    slot.badge_digits.iter().map(|d| d.value(w, cx)).collect()
                } else {
                    vec![]
                };
                PaintTab {
                    item: slot.item.clone(),
                    span: self.spans[i],
                    title,
                    base_title: self.labels == LiquidTabLabels::Always,
                    badge: (opacity, digits),
                }
            })
            .collect();

        let content_bounds = self.content_bounds.clone();
        let viewport_bounds = self.viewport_bounds.clone();
        let focus = if w.last_input_was_keyboard() {
            self.slots
                .iter()
                .position(|slot| slot.focus.is_focused(w))
                .map(|i| self.spans[i])
        } else {
            None
        };
        let drawing = canvas(
            move |bounds, _, _| content_bounds.set(bounds),
            move |b, _, w, cx| {
                for tab in &tabs {
                    tab.paint(b.origin, None, ink, rtl, w, cx);
                }
                if let Some((center, width, height)) = lens {
                    let rect = Bounds::new(
                        b.origin + point(px(center - width / 2.), px((HEIGHT - height) / 2.)),
                        size(px(width), px(height)),
                    );
                    w.paint_quad(fill(rect, ink.lens).corner_radii(rect.size.height / 2.));
                    for tab in &tabs {
                        tab.paint(b.origin, Some(rect), ink, rtl, w, cx);
                    }
                }
                if let Some(span) = focus {
                    let bounds = Bounds::new(
                        b.origin + point(px(span.left - 4.), px(0.)),
                        size(px(span.width + 8.), px(48.)),
                    );
                    w.paint_quad(quad(
                        bounds,
                        px(24.),
                        transparent_black(),
                        px(2.),
                        ink.focus,
                        BorderStyle::Solid,
                    ));
                }
            },
        )
        .absolute()
        .inset_0();
        let moved = cx.listener(|s, e: &MouseMoveEvent, _, cx| {
            if e.pressed_button == Some(MouseButton::Left) {
                s.moved(e.position, false, cx);
            }
        });
        let released = cx.listener(|s, e: &MouseUpEvent, w, cx| {
            if e.button == MouseButton::Left && s.drag.as_ref().is_some_and(|d| !d.touch) {
                s.end(false, w, cx);
            }
        });
        let mut track = div()
            .relative()
            .w(px(full))
            .h(px(HEIGHT))
            .flex_none()
            .child(drawing)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|s, e: &MouseDownEvent, _, cx| s.begin(e.position, false, cx)),
            )
            .child(
                canvas(
                    |_, _, _| (),
                    move |_, _, w, _| {
                        w.on_mouse_event(move |e: &MouseMoveEvent, phase, w, cx| {
                            if phase.capture() {
                                moved(e, w, cx);
                            }
                        });
                        w.on_mouse_event(move |e: &MouseUpEvent, phase, w, cx| {
                            if phase.capture() {
                                released(e, w, cx);
                            }
                        });
                    },
                )
                .absolute()
                .size(px(0.)),
            )
            .child(self.contact.listen(cx, |s, e, w, cx| match e.phase {
                TouchPhase::Started => s.begin(e.start_position, true, cx),
                TouchPhase::Moved => s.moved(e.position, true, cx),
                TouchPhase::Ended => {
                    s.moved(e.position, true, cx);
                    s.end(false, w, cx);
                }
                TouchPhase::Cancelled => s.end(true, w, cx),
            }));
        for (i, slot) in self.slots.iter().enumerate() {
            let tab_id = liquid_tab_id(&self.id, &slot.item.value);
            let name = slot.accessible_name(cx);
            let span = self.spans[i];
            track = track.child(
                div()
                    .id(tab_id.clone())
                    .accessibility_id(tab_id)
                    .debug_selector({
                        let name = name.clone();
                        move || name.to_string()
                    })
                    .aria_label(name)
                    .role(Role::Tab)
                    .aria_selected(selected == Some(i))
                    .track_focus(&slot.focus)
                    .tab_stop(self.roving == i)
                    .absolute()
                    .left(px(span.left))
                    .top(px(INSET))
                    .w(px(span.width))
                    .h(px(40.))
                    .rounded_full()
                    .cursor_pointer()
                    .on_hover(cx.listener(move |s, over: &bool, _, cx| {
                        if s.touch_clock.age(cx) < 0.55 {
                            return;
                        }
                        let before = s.hover;
                        if *over {
                            s.hover = Some(i);
                        } else if before == Some(i) {
                            s.hover = None;
                        }
                        if before != s.hover {
                            cx.notify();
                        }
                    }))
                    .on_click(cx.listener(move |s, _, w, cx| {
                        if !std::mem::take(&mut s.suppress_click) {
                            s.choose(i, w, cx);
                        }
                    }))
                    .on_key_down(cx.listener(move |s, e: &KeyDownEvent, w, cx| s.key(i, e, w, cx))),
            );
        }
        div()
            .id(self.id.clone())
            .debug_selector(|| "liquid-tab-bar".into())
            .accessibility_id(self.id.clone())
            .role(Role::TabList)
            .aria_label(
                self.label
                    .clone()
                    .unwrap_or_else(|| tr(cx, "Sections", "栏目").into()),
            )
            .block_mouse_except_scroll()
            .relative()
            .w(px(full))
            .max_w_full()
            .h(px(HEIGHT))
            .flex_none()
            .rounded_full()
            .bg(ink.track)
            .overflow_hidden()
            .shadow(vec![BoxShadow {
                color: ink.rim,
                offset: point(px(0.), px(0.)),
                blur_radius: px(0.),
                spread_radius: px(1.),
                inset: true,
            }])
            .child(
                canvas(move |b, _, _| viewport_bounds.set(b), |_, _, _, _| {})
                    .absolute()
                    .inset_0(),
            )
            .child(
                div()
                    .id("liquid-tab-scroll")
                    .w_full()
                    .h_full()
                    .overflow_x_scroll()
                    .track_scroll(&self.scroll)
                    .child(track),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::{LiquidTabBar, LiquidTabChanged, LiquidTabIcon, LiquidTabItem, LiquidTabLabels};
    use crate::foundation::Theme;
    use gpui::{
        Entity, Modifiers, MouseButton, TestAppContext, TouchEvent, TouchId, TouchPhase,
        VisualTestContext, point, px,
    };
    use std::{cell::RefCell, rc::Rc, time::Duration};

    fn sections() -> Vec<LiquidTabItem> {
        vec![
            LiquidTabItem::new("home", "Home", LiquidTabIcon::Home),
            LiquidTabItem::new("saved", "Saved", LiquidTabIcon::Bookmark),
            LiquidTabItem::new("inbox", "Inbox", LiquidTabIcon::Inbox)
                .with_badge(2)
                .with_badge_label("2 unread"),
            LiquidTabItem::new("profile", "Profile", LiquidTabIcon::Profile),
        ]
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

    fn accept(view: &Entity<LiquidTabBar>, w: &mut VisualTestContext) -> gpui::Subscription {
        w.update(|_, cx| {
            cx.subscribe(view, |view, event: &LiquidTabChanged, cx| {
                view.update(cx, |bar, cx| bar.set_value(event.0.clone(), cx));
            })
        })
    }

    #[test]
    fn keyboard_wraps_and_the_three_modes_change_measured_layout() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) = app.add_window_view(|_, cx| LiquidTabBar::new(sections(), "home", cx));
        let _subscription = accept(&view, w);
        let outer = w.debug_bounds("liquid-tab-bar").unwrap();
        let home = w.debug_bounds("Home").unwrap();
        let saved = w.debug_bounds("Saved").unwrap();
        assert_eq!(outer.size.height, px(48.));
        assert_eq!(home.left() - outer.left(), px(4.));
        assert!(home.size.width > px(44.));
        assert_eq!(saved.size.width, px(44.));
        assert_eq!(saved.left() - home.right(), px(2.));
        w.simulate_click(home.center(), Modifiers::default());
        w.simulate_keystrokes("left");
        w.update(|w, cx| {
            assert_eq!(view.read(cx).value(), "profile");
            assert!(view.read(cx).slots[3].focus.is_focused(w));
        });
        w.simulate_keystrokes("home right end right");
        w.update(|w, cx| {
            assert_eq!(view.read(cx).value(), "home");
            assert!(view.read(cx).slots[0].focus.is_focused(w));
            view.update(cx, |bar, cx| bar.set_labels(LiquidTabLabels::Always, cx));
        });
        tick(w, 440);
        assert!(w.debug_bounds("Saved").unwrap().size.width > px(44.));
        assert!(w.debug_bounds("Inbox, 2 unread").unwrap().size.width > px(44.));
        w.update(|_, cx| view.update(cx, |bar, cx| bar.set_labels(LiquidTabLabels::None, cx)));
        tick(w, 440);
        assert_eq!(
            w.debug_bounds("liquid-tab-bar").unwrap().size.width,
            px(190.)
        );
        assert_eq!(w.debug_bounds("Home").unwrap().size.width, px(44.));
    }

    #[test]
    fn flick_commits_one_host_request_and_the_lens_retargets_without_jumping() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) = app.add_window_view(|_, cx| {
            LiquidTabBar::new(sections(), "home", cx).with_labels(LiquidTabLabels::None)
        });
        let events = Rc::new(RefCell::new(Vec::new()));
        let _subscription = w.update(|_, cx| {
            let events = events.clone();
            cx.subscribe(&view, move |_, event: &LiquidTabChanged, _| {
                events.borrow_mut().push(event.0.clone())
            })
        });
        let start = w.debug_bounds("Home").unwrap().center();
        let end = start + point(px(10.), px(0.));
        w.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
        tick(w, 24);
        w.simulate_mouse_move(end, Some(MouseButton::Left), Modifiers::default());
        tick(w, 16);
        w.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
        assert_eq!(events.borrow().as_slice(), &["saved"]);
        w.update(|_, cx| {
            assert_eq!(view.read(cx).value(), "home");
            view.update(cx, |bar, cx| bar.set_value("saved", cx));
        });
        tick(w, 65);
        let before = w.update(|_, cx| view.read(cx).lens.unwrap()[0].sample(cx).0);
        assert!(before > 26. && before < 72.);
        w.update(|w, cx| {
            view.update(cx, |bar, cx| bar.set_value("home", cx));
            w.simulate_next_frame(cx);
        });
        let after = w.update(|_, cx| view.read(cx).lens.unwrap()[0].sample(cx).0);
        assert!((before - after).abs() < 0.25);
        tick(w, 450);
        assert_eq!(
            w.update(|_, cx| view.read(cx).lens.unwrap()[0].sample(cx).0),
            26.
        );
    }

    #[test]
    fn vertical_and_cancelled_touch_leave_selection_intact_but_tap_selects() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) = app.add_window_view(|_, cx| {
            LiquidTabBar::new(sections(), "home", cx).with_labels(LiquidTabLabels::None)
        });
        let _subscription = accept(&view, w);
        let home = w.debug_bounds("Home").unwrap().center();
        let saved = w.debug_bounds("Saved").unwrap().center();
        let vertical = home + point(px(7.), px(38.));
        for (id, phase, position) in [
            (1, TouchPhase::Started, home),
            (1, TouchPhase::Moved, vertical),
            (1, TouchPhase::Ended, vertical),
            (2, TouchPhase::Started, home),
            (2, TouchPhase::Moved, saved),
            (2, TouchPhase::Cancelled, saved),
        ] {
            w.simulate_event(TouchEvent {
                id: TouchId(id),
                phase,
                position,
                ..Default::default()
            });
            tick(w, 16);
        }
        assert_eq!(w.update(|_, cx| view.read(cx).value().to_owned()), "home");
        for phase in [TouchPhase::Started, TouchPhase::Ended] {
            w.simulate_event(TouchEvent {
                id: TouchId(3),
                phase,
                position: saved,
                ..Default::default()
            });
            tick(w, 16);
        }
        assert_eq!(w.update(|_, cx| view.read(cx).value().to_owned()), "saved");
    }

    #[test]
    fn reduced_motion_moves_directly_and_updates_badge_without_stale_digits() {
        let mut app = TestAppContext::single();
        app.update(|cx| {
            crate::init(cx);
            Theme::global_mut(cx).reduced_motion = true;
        });
        let (view, w) = app.add_window_view(|_, cx| LiquidTabBar::new(sections(), "home", cx));
        w.update(|_, cx| {
            view.update(cx, |bar, cx| {
                let mut items = sections();
                items[2].badge = 3;
                items[2].badge_label = Some("3 unread".into());
                bar.set_tabs(items, cx);
                bar.set_value("inbox", cx);
            })
        });
        tick(w, 0);
        assert!(w.debug_bounds("Inbox, 3 unread").is_some());
        assert!(w.debug_bounds("Inbox, 2 unread").is_none());
        w.update(|_, cx| {
            let bar = view.read(cx);
            assert_eq!(bar.slots[2].badge_digits[0].sample(cx).0, 3.);
            assert_eq!(
                bar.lens.unwrap()[0].sample(cx),
                (bar.spans[2].center(), 0., false)
            );
        });
    }
}
