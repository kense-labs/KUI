//! Ranking bars that move continuously when the host updates their values.
use crate::primitives::{
    direct_touch::Contact,
    family::{Paint, PhaseClock, Slide, Tone, disabled_motion, face},
    glyph::{Glyph, icon},
    presentation::{column, hint, row, title, tr},
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};

/// A named participant, identified across updates by its stable id.
#[derive(Clone, Debug)]
pub struct RaceEntry {
    pub id: SharedString,
    pub label: SharedString,
    pub value: f32,
    pub color: Hsla,
}
impl RaceEntry {
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        value: f32,
        color: Hsla,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            value,
            color,
        }
    }
    fn safe_value(&self) -> f32 {
        if self.value.is_finite() {
            self.value.max(0.)
        } else {
            0.
        }
    }
}
/// Emitted when the selected participant changes.
#[derive(Clone, Debug)]
pub struct RaceInspect {
    pub id: SharedString,
    pub value: f32,
    pub rank: usize,
}
/// One period of values keyed by the stable entry id. Missing ids are zero.
#[derive(Clone, Debug)]
pub struct RaceFrame {
    pub label: SharedString,
    pub values: Vec<(SharedString, f32)>,
}
impl RaceFrame {
    pub fn new(label: impl Into<SharedString>, values: Vec<(SharedString, f32)>) -> Self {
        Self {
            label: label.into(),
            values,
        }
    }
    fn value(&self, id: &SharedString) -> f32 {
        self.values
            .iter()
            .find(|(key, _)| key == id)
            .map(|(_, value)| *value)
            .filter(|value| value.is_finite())
            .unwrap_or(0.)
            .max(0.)
    }
}
#[derive(Clone, Debug)]
pub struct RaceFrameChanged {
    pub index: usize,
}
#[derive(Clone, Debug)]
pub struct RacePlaybackChanged {
    pub playing: bool,
}
fn race_layout(width: f32) -> (f32, f32, f32) {
    let narrow = width > 0. && width < 400.;
    let label = if narrow {
        0.
    } else if width < 520. {
        104.
    } else {
        128.
    };
    let bar_left = if label == 0. { 0. } else { label + 12. };
    let plot = (width - bar_left - 64.).max(40.);
    (label, bar_left, plot)
}
fn compact_value(value: f32) -> String {
    if value >= 1e12 {
        format!("{value:.1e}")
    } else if value >= 1e9 {
        format!("{:.1}B", value / 1e9)
    } else if value >= 1e6 {
        format!("{:.1}M", value / 1e6)
    } else if value >= 1e4 {
        format!("{:.1}K", value / 1e3)
    } else {
        format!("{value:.0}")
    }
}
struct MovingEntry {
    id: SharedString,
    rank: Slide,
    value: Slide,
}
/// An interactive ranking chart; animation advances when new data is supplied.
pub struct RaceBarChart {
    title: SharedString,
    data: Vec<RaceEntry>,
    moving: Vec<MovingEntry>,
    selected: Option<SharedString>,
    initialized: bool,
    frames: Vec<RaceFrame>,
    position: f32,
    playing: bool,
    visible: bool,
    auto_started: bool,
    frame_duration: f32,
    /// Playhead the bars follow. The source springs this toward `position` over 0.42s.
    shown: Slide,
    /// Time already spent on the last frame. Playback stops after 0.9 × frame duration.
    hold: f32,
    looping: bool,
    clock: PhaseClock,
    last_age: f32,
    dragging: bool,
    track: Rc<Cell<Bounds<Pixels>>>,
    chart_bounds: Rc<Cell<Bounds<Pixels>>>,
    contact: Contact,
}
impl EventEmitter<RaceInspect> for RaceBarChart {}
impl EventEmitter<RaceFrameChanged> for RaceBarChart {}
impl EventEmitter<RacePlaybackChanged> for RaceBarChart {}
impl RaceBarChart {
    pub fn new(title: impl Into<SharedString>, data: Vec<RaceEntry>) -> Self {
        Self {
            title: title.into(),
            data,
            moving: vec![],
            selected: None,
            initialized: false,
            frames: Vec::new(),
            position: 0.,
            playing: false,
            visible: true,
            auto_started: false,
            frame_duration: 1.4,
            shown: Slide::new(0.),
            hold: 0.,
            looping: false,
            clock: PhaseClock::default(),
            last_age: 0.,
            dragging: false,
            track: Default::default(),
            chart_bounds: Default::default(),
            contact: Default::default(),
        }
    }
    /// Enable a host-supplied timeline. Each period lasts 1400 ms by default.
    pub fn with_frames(mut self, frames: Vec<RaceFrame>) -> Self {
        self.frames = frames;
        self.position = 0.;
        self.update_values();
        self
    }
    pub fn with_frame_duration(mut self, seconds: f32) -> Self {
        if seconds.is_finite() && seconds > 0. {
            self.frame_duration = seconds.clamp(0.1, 60.);
        }
        self
    }
    pub fn with_autoplay(mut self, autoplay: bool) -> Self {
        self.auto_started = !autoplay;
        self
    }
    pub fn with_looping(mut self, looping: bool) -> Self {
        self.looping = looping;
        self
    }
    /// Replace frames without inventing values; current position is clamped to the new range.
    pub fn set_frames(&mut self, frames: Vec<RaceFrame>, cx: &mut Context<Self>) {
        self.frames = frames;
        self.position = self
            .position
            .min(self.frames.len().saturating_sub(1) as f32);
        if self.frames.len() < 2 && self.playing {
            self.set_playing(false, cx);
        }
        self.update_values();
        self.reconcile(cx);
        cx.notify();
    }
    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        if self.visible != visible {
            self.visible = visible;
            self.clock.restart(cx);
            self.last_age = 0.;
            cx.notify();
        }
    }
    pub fn set_playing(&mut self, playing: bool, cx: &mut Context<Self>) {
        self.auto_started = true;
        let playing = playing && self.frames.len() > 1 && !disabled_motion(cx);
        if self.playing != playing {
            self.playing = playing;
            self.clock.restart(cx);
            self.last_age = 0.;
            cx.emit(RacePlaybackChanged { playing });
            cx.notify();
        }
    }
    pub fn set_frame(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.frames.is_empty() {
            return;
        }
        self.set_playing(false, cx);
        let end = self.frames.len().saturating_sub(1);
        self.position = index.min(end) as f32;
        self.update_values();
        self.reconcile(cx);
        cx.emit(RaceFrameChanged {
            index: index.min(end),
        });
        cx.notify();
    }
    fn toggle_play(&mut self, cx: &mut Context<Self>) {
        if self.frames.len() < 2 {
            return;
        }
        let end = self.frames.len() - 1;
        if disabled_motion(cx) {
            let current = self.position.round() as usize;
            self.set_frame(if current >= end { 0 } else { current + 1 }, cx);
            return;
        }
        if !self.playing && self.position >= end as f32 {
            self.position = 0.;
            self.update_values();
            self.reconcile(cx);
            cx.emit(RaceFrameChanged { index: 0 });
        }
        self.set_playing(!self.playing, cx);
    }
    fn update_values_at(&mut self, at: f32) {
        if self.frames.is_empty() {
            return;
        }
        let index = (at.floor() as usize).min(self.frames.len() - 1);
        let next = (index + 1).min(self.frames.len() - 1);
        let t = f64::from((at - index as f32).clamp(0., 1.));
        for entry in &mut self.data {
            let a = f64::from(self.frames[index].value(&entry.id));
            let b = f64::from(self.frames[next].value(&entry.id));
            entry.value = (a + (b - a) * t).clamp(0., f64::from(f32::MAX)) as f32;
        }
    }
    fn update_values(&mut self) {
        self.update_values_at(self.position);
    }
    fn scrub(&mut self, x: Pixels, cx: &mut Context<Self>) {
        if self.frames.is_empty() {
            return;
        }
        self.set_playing(false, cx);
        let b = self.track.get();
        let fraction = ((x - b.origin.x) / b.size.width.max(px(1.))).clamp(0., 1.);
        let old = self.position.round() as usize;
        self.position = fraction * self.frames.len().saturating_sub(1) as f32;
        if disabled_motion(cx) {
            self.position = self.position.round();
        }
        self.update_values();
        self.reconcile(cx);
        let index = self.position.round() as usize;
        if old != index {
            cx.emit(RaceFrameChanged { index });
        }
        cx.notify();
    }
    fn release_scrub(&mut self, cx: &mut Context<Self>) {
        self.dragging = false;
        if self.frames.is_empty() {
            return;
        }
        let rounded = self.position.round();
        if (self.position - rounded).abs() > 0.001 {
            self.set_frame(rounded as usize, cx);
        } else {
            cx.notify();
        }
    }
    /// Reorder ranks and animate values from their currently displayed positions.
    pub fn set_entries(&mut self, data: Vec<RaceEntry>, cx: &mut Context<Self>) {
        self.data = data;
        self.update_values();
        self.reconcile(cx);
        cx.notify();
    }
    fn order(&self) -> Vec<usize> {
        let mut order: Vec<_> = (0..self.data.len()).collect();
        order.sort_by(|&a, &b| {
            self.data[b]
                .safe_value()
                .total_cmp(&self.data[a].safe_value())
                .then(a.cmp(&b))
        });
        order
    }
    fn reconcile(&mut self, cx: &App) {
        let order = self.order();
        let mut old: Vec<Option<MovingEntry>> = std::mem::take(&mut self.moving)
            .into_iter()
            .map(Some)
            .collect();
        self.moving = self
            .data
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let rank = order.iter().position(|&o| o == i).unwrap_or(0) as f32;
                let mut entry = old
                    .iter()
                    .position(|e| e.as_ref().is_some_and(|e| e.id == item.id))
                    .and_then(|i| old[i].take())
                    .unwrap_or_else(|| MovingEntry {
                        id: item.id.clone(),
                        rank: Slide::new(rank),
                        value: Slide::new(if self.initialized {
                            0.
                        } else {
                            item.safe_value()
                        }),
                    });
                // Rank uses the source spring: visualDuration 0.5, bounce 0.
                entry.rank.duration(rank, 0.5, cx);
                // Bar length is the interpolated value itself, not a second spring.
                entry.value.reset(item.safe_value());
                entry
            })
            .collect();
        self.selected = self
            .selected
            .take()
            .filter(|id| self.data.iter().any(|v| v.id == *id));
        self.initialized = true;
    }
    fn select(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(item) = self.data.get(index) else {
            return;
        };
        if self.selected.as_ref() == Some(&item.id) {
            return;
        }
        let rank = self.order().iter().position(|v| *v == index).unwrap_or(0) + 1;
        self.selected = Some(item.id.clone());
        cx.emit(RaceInspect {
            id: item.id.clone(),
            value: item.safe_value(),
            rank,
        });
        cx.notify();
    }
}
impl Render for RaceBarChart {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.initialized {
            self.reconcile(cx);
        }
        let p = Paint::get(cx);
        let bounds = self.chart_bounds.get();
        let viewport = w.viewport_size();
        let in_view = bounds.size.width > px(0.)
            && bounds.right() > px(0.)
            && bounds.left() < viewport.width
            && bounds.origin.y < viewport.height
            && bounds.origin.y + bounds.size.height > px(0.);
        let visible = self.visible && w.is_visible() && in_view;
        let reduced = disabled_motion(cx);
        let mut needs_frame = false;
        if reduced && self.playing {
            self.set_playing(false, cx);
        }
        if !self.auto_started && visible && self.frames.len() > 1 {
            self.auto_started = true;
            if !reduced {
                self.playing = true;
                self.clock.restart(cx);
                self.last_age = 0.;
                cx.emit(RacePlaybackChanged { playing: true });
            }
        }
        if self.playing && visible && !reduced && self.frames.len() > 1 {
            if !self.clock.age(cx).is_finite() {
                self.clock.restart(cx);
                self.last_age = 0.;
            }
            let age = self.clock.age(cx);
            let delta = (age - self.last_age).clamp(0., 0.05);
            self.last_age = age;
            let before = self.position.round() as usize;
            let end = self.frames.len() - 1;
            let end_at = end as f32;
            if self.position >= end_at - 1e-6 {
                self.position = end_at;
                self.hold += delta;
                if self.hold >= 0.9 * self.frame_duration {
                    self.hold = 0.;
                    if self.looping {
                        self.position = 0.;
                    } else {
                        self.playing = false;
                        cx.emit(RacePlaybackChanged { playing: false });
                    }
                }
            } else {
                self.hold = 0.;
                self.position = (self.position + delta / self.frame_duration).min(end_at);
            }
            let current = self.position.round() as usize;
            if current != before {
                cx.emit(RaceFrameChanged { index: current });
            }
            needs_frame |= self.playing;
        } else {
            self.clock.restart(cx);
            self.last_age = 0.;
        }
        let count = self.data.len();
        let now = cx.background_executor().now();
        if reduced {
            self.shown.reset(self.position);
        } else {
            self.shown.duration(self.position, 0.42, cx);
        }
        let (shown, shown_moving) = self.shown.sample_batch(now, reduced);
        needs_frame |= shown_moving;
        if !self.frames.is_empty() {
            self.update_values_at(shown);
        }
        self.reconcile(cx);
        let current: Vec<(f32, f32)> = self
            .moving
            .iter_mut()
            .map(|v| {
                let (value, value_moving) = v.value.sample_batch(now, reduced);
                let (rank, rank_moving) = v.rank.sample_batch(now, reduced);
                needs_frame |= value_moving || rank_moving;
                (rank, value)
            })
            .collect();
        if visible && needs_frame {
            w.request_animation_frame();
        }
        let max = current.iter().fold(1f32, |m, (_, v)| m.max(*v));
        let values = current.clone();
        let (label_w, bar_left, plot_w) = race_layout(f32::from(bounds.size.width));
        let chart_bounds = self.chart_bounds.clone();
        let mut chart = div()
            .id("race-chart")
            .role(Role::Figure)
            .tab_index(0)
            .relative()
            .w_full()
            .h(px(26. + (count.max(1) * 36) as f32 + 4.))
            .on_hover(cx.listener(|s, over: &bool, _, cx| {
                if !*over && s.selected.is_some() {
                    s.selected = None;
                    cx.notify();
                }
            }))
            .aria_label({
                let order = self.order();
                let current = self
                    .selected
                    .as_ref()
                    .and_then(|id| order.iter().position(|i| self.data[*i].id == *id))
                    .unwrap_or(0);
                let instructions = tr(
                    cx,
                    "Use up and down to inspect rankings",
                    "使用上下方向键查看排名",
                );
                if let Some(item) = order.get(current).and_then(|i| self.data.get(*i)) {
                    format!(
                        "{}. {} {}: {} {:.1}. {}",
                        self.title,
                        tr(cx, "Rank", "排名"),
                        current + 1,
                        item.label,
                        item.safe_value(),
                        instructions
                    )
                } else {
                    format!("{}. {}", self.title, instructions)
                }
            })
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                if !s.frames.is_empty() {
                    match e.keystroke.key.as_str() {
                        "left" => {
                            let i = (s.position.round() as usize).saturating_sub(1);
                            s.set_frame(i, cx);
                            w.prevent_default();
                            cx.stop_propagation();
                            return;
                        }
                        "right" => {
                            let i = (s.position.round() as usize + 1).min(s.frames.len() - 1);
                            s.set_frame(i, cx);
                            w.prevent_default();
                            cx.stop_propagation();
                            return;
                        }
                        "home" => {
                            s.set_frame(0, cx);
                            w.prevent_default();
                            cx.stop_propagation();
                            return;
                        }
                        "end" => {
                            s.set_frame(s.frames.len() - 1, cx);
                            w.prevent_default();
                            cx.stop_propagation();
                            return;
                        }
                        "space" | "enter" if !e.is_held => {
                            s.toggle_play(cx);
                            w.prevent_default();
                            cx.stop_propagation();
                            return;
                        }
                        _ => {}
                    }
                }
                let order = s.order();
                if order.is_empty() {
                    return;
                }
                let active = s
                    .selected
                    .as_ref()
                    .and_then(|id| order.iter().position(|&i| s.data[i].id == *id))
                    .unwrap_or(0);
                let next = match e.keystroke.key.as_str() {
                    "down" | "right" => {
                        if s.selected.is_none() {
                            0
                        } else {
                            (active + 1).min(order.len() - 1)
                        }
                    }
                    "up" | "left" => active.saturating_sub(1),
                    "home" => 0,
                    "end" => order.len() - 1,
                    "escape" => {
                        s.selected = None;
                        cx.notify();
                        w.prevent_default();
                        cx.stop_propagation();
                        return;
                    }
                    _ => return,
                };
                w.prevent_default();
                cx.stop_propagation();
                s.select(order[next], cx);
            }))
            .child(
                canvas(
                    move |b, w, _| {
                        let previous = chart_bounds.get();
                        let viewport = w.viewport_size();
                        let intersects = |bounds: Bounds<Pixels>| {
                            bounds.size.width > px(0.)
                                && bounds.origin.y < viewport.height
                                && bounds.origin.y + bounds.size.height > px(0.)
                        };
                        if previous.size != b.size || intersects(previous) != intersects(b) {
                            w.request_animation_frame();
                        }
                        chart_bounds.set(b);
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .inset_0(),
            )
            .child(
                div()
                    .absolute()
                    .left(px(bar_left))
                    .top(px(26.))
                    .w(px(1.))
                    .h(px((count.max(1) * 36) as f32))
                    .bg(p.line),
            )
            .children(self.data.iter().enumerate().map(|(i, item)| {
                let (rank, value) = values[i];
                let is_selected = self.selected.as_ref() == Some(&item.id);
                let dim = self.selected.is_some() && !is_selected;
                let fraction = (value / max).clamp(0., 1.);
                let bar_px = (fraction * plot_w).max(2.);
                let value_span = if label_w == 0. { bar_px.max(16.) } else { bar_px };
                let wash = p.panel.blend(item.color.opacity(if is_selected { 0.6 } else { 0.4 }));
                div()
                    .id(("race-item", i))
                    .absolute()
                    .left_0()
                    .right_0()
                    .top(px(26. + rank * 36.))
                    .h(px(36.))
                    .on_hover(cx.listener(move |s, hover: &bool, _, cx| {
                        if *hover {
                            s.select(i, cx)
                        }
                    }))
                    .on_click(cx.listener(move |s, _, _, cx| s.select(i, cx)))
                    .child(
                        div()
                            .absolute()
                            .top_0()
                            .w(px(if label_w == 0. { plot_w - 96. } else { label_w }))
                            .left(px(if label_w == 0. { 8. } else { 0. }))
                            .h(px(36.))
                            .pr(px(0.))
                            .flex()
                            .items_center()
                            .justify_end()
                            .overflow_hidden()
                            .text_size(px(14.))
                            .line_height(px(36.))
                            .text_color(if is_selected { p.ink } else { p.secondary })
                            .font_weight(if is_selected {
                                FontWeight::MEDIUM
                            } else {
                                FontWeight::NORMAL
                            })
                            .child(item.label.clone()),
                    )
                    .child(
                        div()
                            .absolute()
                            .left(px(bar_left))
                            .top(px(6.))
                            .h(px(24.))
                            .w(px(bar_px))
                            .rounded_tr(px(4.))
                            .rounded_br(px(4.))
                            .bg(wash)
                            .opacity(if dim { 0.55 } else { 1. })
                            .child(
                                div()
                                    .absolute()
                                    .right_0()
                                    .top_0()
                                    .bottom_0()
                                    .w(px(3.))
                                    .bg(item.color),
                            ),
                    )
                    .child(
                        div()
                            .absolute()
                            .top_0()
                            .left(px(bar_left + value_span + 8.))
                            .h(px(36.))
                            .flex()
                            .items_center()
                            .text_size(px(14.))
                            .line_height(px(36.))
                            .text_color(p.ink)
                            .font_weight(if is_selected {
                                FontWeight::MEDIUM
                            } else {
                                FontWeight::NORMAL
                            })
                            .child(compact_value(value)),
                    )
            }));
        if let Some(id) = self.selected.clone()
            && let Some((index, item)) = self
                .data
                .iter()
                .enumerate()
                .find(|(_, item)| item.id == id)
        {
            let (rank, value) = values[index];
            let place = self.order().iter().position(|v| *v == index).unwrap_or(0) + 1;
            chart = chart.child(
                div()
                    .absolute()
                    .left(px((bar_left + 14.).min(plot_w.max(14.))))
                    .top(px((26. + rank * 36. - 4.).max(4.)))
                    .min_w(px(150.))
                    .max_w(px(240.))
                    .px(px(12.))
                    .py(px(10.))
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .rounded(crate::foundation::surface_radius(cx))
                    .border_1()
                    .border_color(p.line)
                    .bg(p.panel)
                    .shadow_lg()
                    .child(
                        div()
                            .text_size(px(12.))
                            .line_height(px(20.))
                            .text_color(p.secondary)
                            .child(item.label.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(16.))
                            .line_height(px(21.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(compact_value(value)),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .line_height(px(20.))
                            .text_color(p.faint)
                            .child(format!("{} {}", tr(cx, "Rank", "排名"), place)),
                    ),
            );
        }
        let timeline =
            if self.frames.is_empty() {
                None
            } else {
                let end = self.frames.len() - 1;
                let index = (self.position.round() as usize).min(end);
                let period = self.frames[index].label.clone();
                let first = self.frames[0].label.clone();
                let last = self.frames[end].label.clone();
                let fraction = if end > 0 {
                    (self.position / end as f32).clamp(0., 1.)
                } else {
                    0.
                };
                let track_bounds = self.track.clone();
                let touch = self.contact.listen(cx, |s, e: &TouchDragEvent, _, cx| {
                    if matches!(e.phase, TouchPhase::Started | TouchPhase::Moved) {
                        s.scrub(e.position.x, cx);
                    }
                });
                let track = div()
                    .id("race-timeline")
                    .role(Role::Slider)
                    .tab_index(0)
                    .aria_label(tr(cx, "Race timeline", "排名时间轴"))
                    .aria_value(period.clone())
                    .relative()
                    .w_full()
                    .h(px(36.))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|s, e: &MouseDownEvent, _, cx| {
                            s.dragging = true;
                            s.scrub(e.position.x, cx);
                        }),
                    )
                    .on_mouse_move(cx.listener(|s, e: &MouseMoveEvent, _, cx| {
                        if s.dragging {
                            s.scrub(e.position.x, cx);
                        }
                    }))
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(|s, _, _, cx| s.release_scrub(cx)),
                    )
                    .on_mouse_up_out(
                        MouseButton::Left,
                        cx.listener(|s, _, _, cx| s.release_scrub(cx)),
                    )
                    .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                        let current = s.position.round() as usize;
                        let last = s.frames.len().saturating_sub(1);
                        let target = match e.keystroke.key.as_str() {
                            "left" | "down" => Some(current.saturating_sub(1)),
                            "right" | "up" => Some((current + 1).min(last)),
                            "home" => Some(0),
                            "end" => Some(last),
                            "space" | "enter" if !e.is_held => {
                                s.toggle_play(cx);
                                w.prevent_default();
                                cx.stop_propagation();
                                return;
                            }
                            _ => None,
                        };
                        if let Some(target) = target {
                            s.set_frame(target, cx);
                            w.prevent_default();
                            cx.stop_propagation();
                        }
                    }))
                    .child(
                        canvas(move |b, _, _| track_bounds.set(b), |_, _, _, _| {})
                            .absolute()
                            .inset_0(),
                    )
                    .child(
                        div()
                            .absolute()
                            .left_0()
                            .right_0()
                            .top(px(16.))
                            .h(px(4.))
                            .rounded(px(2.))
                            .bg(p.tray)
                            .child(
                                div()
                                    .h_full()
                                    .w(relative(fraction))
                                    .rounded(px(2.))
                                    .bg(p.ink.opacity(0.55)),
                            ),
                    )
                    .child(
                        div()
                            .absolute()
                            .left(relative(fraction))
                            .top(px(10.))
                            .ml(px(-8.))
                            .size(px(16.))
                            .rounded_full()
                            .bg(p.panel)
                            .border_1()
                            .border_color(p.faint)
                            .shadow_sm(),
                    )
                    .child(touch);
                let label = if reduced {
                    tr(cx, "Next period", "下一时间段")
                } else if self.playing {
                    tr(cx, "Pause", "暂停")
                } else if index == end {
                    tr(cx, "Replay", "重新播放")
                } else {
                    tr(cx, "Play", "播放")
                };
                Some(
                    column()
                        .w_full()
                        .gap(px(2.))
                        .child(
                            div()
                                .text_size(px(30.))
                                .line_height(px(33.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.ink)
                                .child(period.clone()),
                        )
                        .child(
                            row()
                                .items_center()
                                .gap(px(12.))
                                .child(
                                    face(
                                        "race-play",
                                        label,
                                        36.,
                                        18.,
                                        Tone::Surface,
                                        cx,
                                    )
                                    .size(px(36.))
                                    .min_w(px(36.))
                                    .p_0()
                                    .rounded_full()
                                    .border_1()
                                    .border_color(p.line)
                                    .bg(p.panel)
                                    .hover(|s| s.bg(p.tray))
                                    .child(icon(
                                        if reduced {
                                            Glyph::Arrow
                                        } else if self.playing {
                                            Glyph::Pause
                                        } else if index == end {
                                            Glyph::Undo
                                        } else {
                                            Glyph::Play
                                        },
                                        16.,
                                        p.ink,
                                    ))
                                    .on_click(cx.listener(|s, _, _, cx| s.toggle_play(cx)))
                                    .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                                        if matches!(e.keystroke.key.as_str(), "enter" | "space")
                                            && !e.is_held
                                        {
                                            s.toggle_play(cx);
                                            w.prevent_default();
                                            cx.stop_propagation();
                                        }
                                    })),
                                )
                                .child(
                                    column()
                                        .flex_1()
                                        .min_w_0()
                                        .gap(px(2.))
                                        .child(track)
                                        .child(
                                            row()
                                                .justify_between()
                                                .child(hint(first, p))
                                                .child(hint(last, p)),
                                        ),
                                ),
                        ),
                )
            };
        column()
            .w(px(496.))
            .max_w_full()
            .gap_3()
            .p_4()
            .rounded(crate::foundation::surface_radius(cx))
            .bg(p.panel)
            .border_1()
            .border_color(p.line)
            .text_color(p.ink)
            .child(title(self.title.clone(), p))
            .child(chart)
            .when_some(timeline, |panel, timeline| panel.child(timeline))
            .child(hint(
                tr(cx, "Select a bar to inspect its rank", "选择条形查看排名"),
                p,
            ))
            .child(
                row()
                    .flex_wrap()
                    .gap_2()
                    .children(self.order().into_iter().map(|i| {
                        let item = &self.data[i];
                        face(
                            ("race-legend", i),
                            item.label.clone(),
                            32.,
                            crate::foundation::surface_radius(cx).into(),
                            Tone::Clear,
                            cx,
                        )
                        .px(px(8.))
                        .gap(px(8.))
                        .text_size(px(12.))
                        .line_height(px(17.))
                        .text_color(p.secondary)
                        .child(div().size(px(10.)).rounded(px(3.)).bg(item.color))
                        .child(item.label.clone())
                        .on_click(cx.listener(move |s, _, _, cx| s.select(i, cx)))
                        .on_key_down(cx.listener(
                            move |s, e: &KeyDownEvent, w, cx| {
                                if matches!(e.keystroke.key.as_str(), "enter" | "space")
                                    && !e.is_held
                                {
                                    s.select(i, cx);
                                    w.prevent_default();
                                    cx.stop_propagation();
                                }
                            },
                        ))
                    })),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::{RaceBarChart, RaceEntry, RaceFrame};
    use gpui::rgb;
    #[test]
    fn timeline_interpolates_host_values_and_treats_missing_as_zero() {
        let entries = vec![
            RaceEntry::new("a", "A", 0., rgb(0x123456).into()),
            RaceEntry::new("b", "B", 0., rgb(0x123456).into()),
        ];
        let frames = vec![
            RaceFrame::new("one", vec![("a".into(), 10.), ("b".into(), f32::NAN)]),
            RaceFrame::new("two", vec![("a".into(), 30.)]),
        ];
        let mut chart = RaceBarChart::new("Race", entries).with_frames(frames);
        chart.position = 0.5;
        chart.update_values();
        assert_eq!(chart.data[0].value, 20.);
        assert_eq!(chart.data[1].value, 0.);
    }
}
