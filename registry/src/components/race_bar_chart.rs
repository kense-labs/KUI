//! A ranking over time: bars overtake each other on springs as values count up.
use crate::primitives::{
    direct_touch::Contact,
    family::{Paint, PhaseClock, Slide, Tone, disabled_motion, face},
    glyph::{Glyph, icon},
    presentation::{column, hint, row, title, tr},
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};

/// A named participant, identified across frames by its stable id.
#[derive(Clone, Debug)]
pub struct RaceEntry {
    pub id: SharedString,
    pub label: SharedString,
    /// Group id; bars take their hue from the group's series slot.
    pub group: Option<SharedString>,
    /// Interpolated display value, kept by the chart between frames.
    pub value: f32,
}
impl RaceEntry {
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            group: None,
            value: 0.,
        }
    }
    /// Place the entry in a group that maps onto the series palette.
    pub fn with_group(mut self, group: impl Into<SharedString>) -> Self {
        self.group = Some(group.into());
        self
    }
}
/// Emitted when the inspected bar changes.
#[derive(Clone, Debug)]
pub struct RaceInspect {
    pub id: SharedString,
    pub value: f32,
    pub rank: usize,
}
/// One period of values keyed by the stable entry id. Missing ids count as zero.
#[derive(Clone, Debug)]
pub struct RaceFrame {
    pub period: SharedString,
    pub values: Vec<(SharedString, f32)>,
}
impl RaceFrame {
    pub fn new(period: impl Into<SharedString>, values: Vec<(SharedString, f32)>) -> Self {
        Self {
            period: period.into(),
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
/// Series hues for the first four groups, matching the shared chart palette.
const SERIES: [u32; 4] = [0x55adff, 0x9c37be, 0xe66e00, 0x00a861];
const ROW_H: f32 = 36.;
const BAR_H: f32 = 24.;
const ROWS_TOP: f32 = 26.;
const LABEL_W: f32 = 104.;
const VALUE_GAP: f32 = 8.;

/// Compact money in the reference's shape: `$14K`, `$660K`, `$3.87M`.
fn compact_value(value: f32, prefix: &str) -> String {
    let v = if value.is_finite() && value >= 0. {
        value
    } else {
        0.
    };
    let units = [(1e9, "B"), (1e6, "M")];
    for (scale, suffix) in units {
        if v >= scale {
            let text = format!("{:.2}", v / scale)
                .trim_end_matches('0')
                .trim_end_matches('.')
                .to_string();
            return format!("{prefix}{text}{suffix}");
        }
    }
    if v >= 1e3 {
        return format!("{prefix}{:.0}K", v / 1e3);
    }
    format!("{prefix}{v:.0}")
}
/// The largest 1-2-5 multiple of a power of ten not above `raw`.
fn nice_step(raw: f32) -> f32 {
    if !raw.is_finite() || raw <= 0. {
        return 1.;
    }
    let exponent = raw.log10().floor();
    let base = 10f32.powf(exponent);
    let scaled = raw / base;
    let multiple = if scaled >= 5. {
        5.
    } else if scaled >= 2. {
        2.
    } else {
        1.
    };
    multiple * base
}

struct MovingEntry {
    id: SharedString,
    rank: Slide,
    /// 0 while the row is outside the shown cut, 1 once it has risen in.
    shown: Slide,
    value: f32,
}
/// An interactive bar chart race with a playable, scrubbable timeline.
pub struct RaceBarChart {
    label: SharedString,
    meta: Option<SharedString>,
    data: Vec<RaceEntry>,
    groups: Vec<(SharedString, SharedString)>,
    moving: Vec<MovingEntry>,
    selected: Option<SharedString>,
    highlight: Option<SharedString>,
    initialized: bool,
    frames: Vec<RaceFrame>,
    position: f32,
    playing: bool,
    visible: bool,
    auto_started: bool,
    frame_duration: f32,
    looping: bool,
    bars: usize,
    prefix: SharedString,
    /// Playhead the bars follow. The reference springs this toward `position`.
    shown: Slide,
    /// Time already spent holding the last frame before playback stops.
    hold: f32,
    /// Crossfade between the outgoing and incoming period word.
    word: Slide,
    word_prev: SharedString,
    word_now: SharedString,
    word_index: usize,
    word_direction: f32,
    tick_step: f32,
    previous_tick_step: f32,
    tick_fade: Slide,
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
    pub fn new(label: impl Into<SharedString>, data: Vec<RaceEntry>) -> Self {
        Self {
            label: label.into(),
            meta: None,
            data,
            groups: vec![],
            moving: vec![],
            selected: None,
            highlight: None,
            initialized: false,
            frames: Vec::new(),
            position: 0.,
            playing: false,
            visible: true,
            auto_started: false,
            frame_duration: 1.4,
            looping: false,
            bars: 8,
            prefix: SharedString::default(),
            shown: Slide::new(0.),
            hold: 0.,
            word: Slide::new(1.),
            word_prev: SharedString::default(),
            word_now: SharedString::default(),
            word_index: 0,
            word_direction: 1.,
            tick_step: 0.,
            previous_tick_step: 0.,
            tick_fade: Slide::new(1.),
            clock: PhaseClock::default(),
            last_age: 0.,
            dragging: false,
            track: Default::default(),
            chart_bounds: Default::default(),
            contact: Default::default(),
        }
    }
    /// Subtitle under the heading, such as "Year to date, 2025".
    pub fn with_meta(mut self, meta: impl Into<SharedString>) -> Self {
        self.meta = Some(meta.into());
        self
    }
    /// Named groups; the first four take the series hues and a legend appears.
    pub fn with_groups(mut self, groups: Vec<(SharedString, SharedString)>) -> Self {
        self.groups = groups;
        self
    }
    /// Value prefix such as "$" applied on bars, ticks, and the total.
    pub fn with_value_prefix(mut self, prefix: impl Into<SharedString>) -> Self {
        self.prefix = prefix.into();
        self
    }
    /// Bars shown at once; items below the cut slide out of view.
    pub fn with_bars(mut self, bars: usize) -> Self {
        self.bars = bars.clamp(1, 24);
        self
    }
    /// Item id whose name and value are emphasized.
    pub fn with_highlight(mut self, id: impl Into<SharedString>) -> Self {
        self.highlight = Some(id.into());
        self
    }
    /// Enable the timeline. Each period lasts 1400 ms by default.
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
    /// Replace frames without inventing values; position is clamped to the range.
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
    /// Pause playback and paint scheduling while the host is hidden.
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
    /// Glide the race to one period; playback pauses.
    pub fn set_frame(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.frames.is_empty() {
            return;
        }
        self.set_playing(false, cx);
        let end = self.frames.len().saturating_sub(1);
        self.position = index.min(end) as f32;
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
    fn safe_value(entry: &RaceEntry) -> f32 {
        if entry.value.is_finite() {
            entry.value.max(0.)
        } else {
            0.
        }
    }
    fn order(&self) -> Vec<usize> {
        let mut order: Vec<_> = (0..self.data.len()).collect();
        order.sort_by(|&a, &b| {
            Self::safe_value(&self.data[b])
                .total_cmp(&Self::safe_value(&self.data[a]))
                .then(a.cmp(&b))
        });
        order
    }
    /// Reorder ranks and re-target the row springs from displayed positions.
    fn reconcile(&mut self, cx: &App) {
        let order = self.order();
        let cut = self.bars as f32;
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
                let in_cut = rank < cut;
                let mut entry = old
                    .iter()
                    .position(|e| e.as_ref().is_some_and(|e| e.id == item.id))
                    .and_then(|i| old[i].take())
                    .unwrap_or_else(|| MovingEntry {
                        id: item.id.clone(),
                        rank: Slide::new(if in_cut { rank } else { cut }),
                        shown: Slide::new(if self.initialized { 0. } else { 1. }),
                        value: Self::safe_value(item),
                    });
                // Rank rides the source spring; overtakes read as one slide.
                entry.rank.duration(rank.min(cut), 0.5, cx);
                entry.shown.duration(if in_cut { 1. } else { 0. }, 0.26, cx);
                entry.value = Self::safe_value(item);
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
            value: Self::safe_value(item),
            rank,
        });
        cx.notify();
    }
    fn group_tone(&self, entry: &RaceEntry) -> Hsla {
        let index = entry
            .group
            .as_ref()
            .and_then(|g| self.groups.iter().position(|(id, _)| id == g));
        match index {
            Some(k) => rgb(SERIES[k % SERIES.len()]).into(),
            None => rgb(SERIES[0]).into(),
        }
    }
}
impl Render for RaceBarChart {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
            // Wall-clock pacing: clamp only against tab-switch stalls so the
            // race never slows down on a throttled frame rate.
            let delta = (age - self.last_age).clamp(0., 0.25);
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
        // The drawn clock springs toward the requested time; steps glide and a
        // replay sweeps back through every period.
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
        // Both words stay in the same reserved slot when the rounded period changes.
        let period_index = if self.frames.is_empty() {
            0
        } else {
            (shown.round() as usize).min(self.frames.len() - 1)
        };
        let period = self
            .frames
            .get(period_index)
            .map(|f| f.period.clone())
            .unwrap_or_default();
        if self.word_now != period {
            self.word_direction = if period_index < self.word_index {
                -1.
            } else {
                1.
            };
            self.word_index = period_index;
            self.word_prev = self.word_now.clone();
            self.word_now = period.clone();
            self.word
                .reset(f32::from(self.word_prev.is_empty() || reduced));
            self.word.duration(1., 0.16, cx);
        }
        let (word_t, word_moving) = self.word.sample_batch(now, reduced);
        needs_frame |= word_moving;
        let cut = self.bars.min(self.data.len().max(1));
        let mut rows: Vec<(f32, f32, f32)> = self
            .moving
            .iter_mut()
            .map(|v| {
                let (rank, rank_moving) = v.rank.sample_batch(now, reduced);
                let (shown, row_moving) = v.shown.sample_batch(now, reduced);
                needs_frame |= rank_moving || row_moving;
                (rank, shown, v.value)
            })
            .collect();
        let order = self.order();
        // Paint top-ranked rows last so overtake overlap matches the source.
        let paint_order: Vec<usize> = order.iter().rev().copied().collect();
        let axis_max = rows.iter().fold(1f32, |m, (_, _, v)| m.max(*v));
        let total: f32 = rows.iter().map(|(_, _, v)| *v).sum();
        let width = f32::from(bounds.size.width);
        let narrow = width > 0. && width < 400.;
        let bar_left = if narrow { 0. } else { LABEL_W + 12. };
        let tail_room = if narrow { 56. } else { 64. };
        let plot_w = (width - bar_left - tail_room).max(40.);
        // Nice steps choose labels only. Rounding the domain to a multiple of
        // the step caused every bar to jump whenever that multiple changed.
        let tick_count = (plot_w / 84.).floor().clamp(2., 4.);
        let tick_step = nice_step(axis_max / tick_count);
        if self.tick_step != tick_step {
            self.previous_tick_step = self.tick_step;
            self.tick_step = tick_step;
            self.tick_fade
                .reset(f32::from(self.previous_tick_step == 0. || reduced));
            self.tick_fade.duration(1., 0.16, cx);
        }
        let (tick_fade, ticks_moving) = self.tick_fade.sample_batch(now, reduced);
        if visible && (needs_frame || ticks_moving) {
            w.request_animation_frame();
        }
        let chart_bounds = self.chart_bounds.clone();
        let instructions = tr(
            cx,
            "Space plays or pauses, left and right arrows change the period, up and down arrows read the bars",
            "空格播放或暂停，左右方向键切换时间段，上下方向键读取条形",
        );
        let plot_label = format!(
            "{}, {}{}. {}",
            self.label,
            self.meta
                .clone()
                .map(|m| format!("{m}, "))
                .unwrap_or_default(),
            period,
            instructions
        );
        let prefix = self.prefix.to_string();
        let mut chart = div()
            .id("race-chart")
            .role(Role::Group)
            .tab_index(0)
            .relative()
            .w_full()
            .overflow_hidden()
            .h(px(ROWS_TOP + cut as f32 * ROW_H + 4.))
            .aria_label(plot_label)
            .on_hover(cx.listener(|s, over: &bool, w, cx| {
                // Keyboard focus suppresses pointer hover, so a keypress must
                // not clear the reading the reference keeps on screen.
                if !*over && !w.last_input_was_keyboard() {
                    s.selected = None;
                    cx.notify();
                }
            }))
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
                    .and_then(|id| order.iter().position(|&i| s.data[i].id == *id));
                let next = match e.keystroke.key.as_str() {
                    "down" => Some(match active {
                        None => 0,
                        Some(a) => (a + 1).min(order.len() - 1),
                    }),
                    "up" => Some(match active {
                        None => 0,
                        Some(a) => a.saturating_sub(1),
                    }),
                    "escape" => {
                        s.selected = None;
                        cx.notify();
                        w.prevent_default();
                        cx.stop_propagation();
                        return;
                    }
                    _ => return,
                };
                if let Some(rank) = next {
                    w.prevent_default();
                    cx.stop_propagation();
                    s.select(order[rank], cx);
                }
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
                    .top(px(ROWS_TOP))
                    .w(px(1.))
                    .h(px(cut as f32 * ROW_H))
                    .bg(p.faint),
            );
        // Draw outgoing and incoming ticks against the same continuous scale.
        for (outgoing, step, opacity) in [
            (true, self.previous_tick_step, 1. - tick_fade),
            (false, self.tick_step, tick_fade),
        ] {
            if step <= 0. || (outgoing && opacity <= 0.) {
                continue;
            }
            for index in 1..=12 {
                let tick = step * index as f32;
                if tick >= axis_max {
                    break;
                }
                let alternate = if outgoing {
                    self.tick_step
                } else {
                    self.previous_tick_step
                };
                let ratio = tick / alternate;
                let shared = alternate > 0. && (ratio - ratio.round()).abs() < 0.01;
                if outgoing && shared {
                    continue;
                }
                if !shared && opacity <= 0. {
                    continue;
                }
                let x = bar_left + tick / axis_max * plot_w;
                if x < bar_left + plot_w - 2. {
                    let alpha = if shared { 1. } else { opacity };
                    let alpha = alpha * ((bar_left + plot_w - x) / 14.).clamp(0., 1.);
                    chart = chart
                        .child(
                            div()
                                .absolute()
                                .left(px(x))
                                .top(px(ROWS_TOP))
                                .w(px(1.))
                                .h(px(cut as f32 * ROW_H))
                                .bg(p.line.opacity(alpha)),
                        )
                        .child(
                            div()
                                .absolute()
                                .left(px(x))
                                .top(px(4.))
                                .ml(px(-24.))
                                .w(px(48.))
                                .text_center()
                                .text_size(px(12.))
                                .line_height(px(12.))
                                .text_color(p.faint.opacity(alpha))
                                .child(compact_value(tick, &prefix)),
                        );
                }
            }
        }
        for i in paint_order {
            let item = &self.data[i];
            let (rank, row_shown, value) = rows[i];
            let is_selected = self.selected.as_ref() == Some(&item.id);
            let dim = self.selected.is_some() && !is_selected;
            let highlighted = self.highlight.as_ref() == Some(&item.id);
            let tone = self.group_tone(item);
            let fraction = (value / axis_max).clamp(0., 1.);
            let bar_px = (fraction * plot_w).max(2.);
            let wash = p
                .panel
                .blend(tone.opacity(if is_selected { 0.6 } else { 0.4 }));
            let value_x = bar_left + bar_px + VALUE_GAP;
            let label_inside = narrow;
            let place = order.iter().position(|&o| o == i).unwrap_or(0) + 1;
            chart = chart.child(
                div()
                    .id(("race-item", i))
                    .absolute()
                    .left_0()
                    .right_0()
                    .top(px(ROWS_TOP + rank * ROW_H))
                    .h(px(ROW_H))
                    .opacity(row_shown)
                    .on_hover(cx.listener(move |s, hover: &bool, _, cx| {
                        if *hover {
                            s.select(i, cx);
                        }
                    }))
                    .on_click(cx.listener(move |s, _, _, cx| s.select(i, cx)))
                    .when(!label_inside, |row| {
                        row.child(
                            div()
                                .absolute()
                                .top_0()
                                .left_0()
                                .w(px(LABEL_W))
                                .h(px(ROW_H))
                                .flex()
                                .items_center()
                                .justify_end()
                                .overflow_hidden()
                                .text_size(px(13.))
                                .line_height(px(ROW_H))
                                .text_color(if is_selected || highlighted {
                                    p.ink
                                } else {
                                    p.secondary
                                })
                                .font_weight(if highlighted {
                                    FontWeight::MEDIUM
                                } else {
                                    FontWeight::NORMAL
                                })
                                .child(item.label.clone()),
                        )
                    })
                    .child(
                        div()
                            .absolute()
                            .left(px(bar_left))
                            .top(px(6.))
                            .h(px(BAR_H))
                            .w(px(bar_px))
                            .overflow_hidden()
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
                                    .bg(tone),
                            )
                            .when(label_inside, |bar| {
                                bar.child(
                                    div()
                                        .absolute()
                                        .left(px(8.))
                                        .top_0()
                                        .bottom_0()
                                        .right(px(6.))
                                        .flex()
                                        .items_center()
                                        .overflow_hidden()
                                        .text_size(px(13.))
                                        .line_height(px(BAR_H))
                                        .text_color(p.ink)
                                        .child(item.label.clone()),
                                )
                            }),
                    )
                    .child(
                        div()
                            .absolute()
                            .top_0()
                            .left(px(if label_inside {
                                value_x.max(bar_left + 124.)
                            } else {
                                value_x
                            }))
                            .h(px(ROW_H))
                            .flex()
                            .items_center()
                            .text_size(px(13.))
                            .line_height(px(ROW_H))
                            .text_color(if highlighted {
                                p.ink
                            } else {
                                p.ink.opacity(0.9)
                            })
                            .font_weight(if highlighted {
                                FontWeight::MEDIUM
                            } else {
                                FontWeight::NORMAL
                            })
                            .child(compact_value(value, &prefix)),
                    )
                    .when(is_selected, |row| {
                        let tip_left = (bar_left + 14.).min((width - 252.).max(bar_left + 14.));
                        let tip_top =
                            (rank * ROW_H - 6.).clamp(2., (cut as f32 * ROW_H - 80.).max(2.));
                        row.child(
                            div()
                                .absolute()
                                .left(px(tip_left))
                                .top(px(tip_top))
                                .min_w(px(150.))
                                .max_w(px(240.))
                                .px(px(12.))
                                .py(px(10.))
                                .flex()
                                .flex_col()
                                .gap(px(2.))
                                .rounded(px(14.))
                                .border_1()
                                .border_color(p.line)
                                .bg(p.tray)
                                .shadow_lg()
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .line_height(px(18.))
                                        .text_color(p.secondary)
                                        .child(item.label.clone()),
                                )
                                .child(
                                    div()
                                        .text_size(px(16.))
                                        .line_height(px(20.8))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.ink)
                                        .child(compact_value(value, &prefix)),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .line_height(px(18.))
                                        .text_color(p.faint)
                                        .child(format!(
                                            "{} {} / {}",
                                            tr(cx, "Rank", "排名"),
                                            place,
                                            self.data.len()
                                        )),
                                ),
                        )
                    }),
            );
        }
        rows.clear();
        // Legend appears once groups are named.
        let legend = (!self.groups.is_empty()).then(|| {
            row().flex_wrap().gap_x(px(16.)).gap_y(px(6.)).children(
                self.groups.iter().enumerate().map(|(k, (_, label))| {
                    let tone: Hsla = rgb(SERIES[k % SERIES.len()]).into();
                    row()
                        .id(("race-legend", k))
                        .items_center()
                        .gap(px(8.))
                        .text_size(px(12.))
                        .line_height(px(18.))
                        .text_color(p.secondary)
                        .child(
                            div()
                                .relative()
                                .size(px(10.))
                                .overflow_hidden()
                                .rounded_tr(px(3.))
                                .rounded_br(px(3.))
                                .bg(p.panel.blend(tone.opacity(0.4)))
                                .child(
                                    div()
                                        .absolute()
                                        .right_0()
                                        .top_0()
                                        .bottom_0()
                                        .w(px(2.))
                                        .bg(tone),
                                ),
                        )
                        .child(label.clone())
                }),
            )
        });
        // Head: the period rolls in large display type with the running total.
        let head = (!period.is_empty()).then(|| {
            row()
                .h(px(42.))
                .items_center()
                .justify_between()
                .gap(px(12.))
                .child(
                    div()
                        .relative()
                        .flex_1()
                        .min_w_0()
                        .h_full()
                        .overflow_hidden()
                        .text_size(px(if width > 0. && width < 480. { 24. } else { 30. }))
                        .line_height(px(36.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.ink)
                        .child(
                            div()
                                .absolute()
                                .left_0()
                                .right_0()
                                .top(px(3. - self.word_direction * word_t * 5.))
                                .truncate()
                                .opacity(1. - word_t)
                                .child(self.word_prev.clone()),
                        )
                        .child(
                            div()
                                .absolute()
                                .left_0()
                                .right_0()
                                .top(px(3. + self.word_direction * (1. - word_t) * 5.))
                                .truncate()
                                .opacity(word_t)
                                .child(self.word_now.clone()),
                        ),
                )
                .child(
                    div()
                        .flex_none()
                        .text_right()
                        .text_size(px(13.))
                        .line_height(px(18.))
                        .text_color(p.faint)
                        .child(format!(
                            "{} {}",
                            tr(cx, "Total", "总计"),
                            compact_value(total, &prefix)
                        )),
                )
        });
        // Controls: round play button beside the scrubbing timeline.
        let controls = (!self.frames.is_empty()).then(|| {
            let end = self.frames.len() - 1;
            let index = (self.position.round() as usize).min(end);
            let first = self.frames[0].period.clone();
            let last = self.frames[end].period.clone();
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
                .aria_label(format!("{}, {}", self.label, tr(cx, "period", "时间段")))
                .aria_value(period.clone())
                .relative()
                .w_full()
                .h(px(36.))
                .cursor_pointer()
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
                        .bg(p.hover)
                        .child(
                            div()
                                .h_full()
                                .w(relative(fraction))
                                .rounded(px(2.))
                                .bg(p.panel.blend(p.ink.opacity(0.55))),
                        ),
                )
                .children((0..self.frames.len()).map(|i| {
                    let at = if end > 0 { i as f32 / end as f32 } else { 0. };
                    div()
                        .absolute()
                        .left(relative(at))
                        .top(px(17.))
                        .ml(px(-1.))
                        .size(px(2.))
                        .rounded_full()
                        .bg(if (i as f32) <= self.position + 1e-3 {
                            p.panel.opacity(0.9)
                        } else {
                            p.faint.opacity(0.6)
                        })
                }))
                .child(
                    div()
                        .absolute()
                        .left(relative(fraction))
                        .top(px(10.))
                        .ml(px(-8.))
                        .size(px(16.))
                        .rounded_full()
                        .bg(p.tray)
                        .border_1()
                        .border_color(p.line)
                        .shadow_sm(),
                )
                .child(touch);
            let play_label = if self.playing {
                tr(cx, "Pause", "暂停")
            } else if index == end {
                tr(cx, "Replay", "重新播放")
            } else {
                tr(cx, "Play", "播放")
            };
            row()
                .items_start()
                .gap(px(12.))
                .child(
                    face("race-play", play_label, 36., 18., Tone::Surface, cx)
                        .flex_none()
                        .size(px(36.))
                        .min_w(px(36.))
                        .p_0()
                        .rounded_full()
                        .border_1()
                        .border_color(p.line)
                        .bg(p.panel)
                        .hover(|s| s.bg(p.tray))
                        .child(icon(
                            if self.playing {
                                Glyph::Pause
                            } else if index == end {
                                Glyph::Undo
                            } else {
                                Glyph::Play
                            },
                            16.,
                            p.ink,
                        ))
                        // Press-started toggle: while playback animates, a
                        // web press/release pair can straddle a rebuild, so
                        // the release never matches the press to form a click.
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|s, _, _, cx| s.toggle_play(cx)),
                        )
                        .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                            if matches!(e.keystroke.key.as_str(), "enter" | "space") && !e.is_held {
                                s.toggle_play(cx);
                                w.prevent_default();
                                cx.stop_propagation();
                            }
                        })),
                )
                .child(
                    column().flex_1().min_w_0().gap(px(2.)).child(track).child(
                        row()
                            .justify_between()
                            .child(hint(first, p))
                            .child(hint(last, p)),
                    ),
                )
        });
        // Screen-reader table: every item's value for every period.
        let readings = div()
            .id("race-readings")
            .absolute()
            .size(px(1.))
            .overflow_hidden()
            .role(Role::Table)
            .aria_label(self.label.clone())
            .child(div().id("race-live").role(Role::Row).aria_label(format!(
                        "{}, {}",
                        period,
                        order
                            .first()
                            .and_then(|&i| self.data.get(i))
                            .map(|item| format!("{} {}", item.label, tr(cx, "leads", "领先")))
                            .unwrap_or_default()
                    )))
            .children(self.data.iter().enumerate().map(|(i, item)| {
                let cells = self
                    .frames
                    .iter()
                    .map(|f| compact_value(f.value(&item.id), &prefix))
                    .collect::<Vec<_>>()
                    .join(", ");
                div()
                    .id(("race-reading", i))
                    .role(Role::Row)
                    .aria_label(format!("{}: {}", item.label, cells))
            }));
        column()
            .w(px(720.))
            .max_w_full()
            .gap(px(20.))
            .p(px(if width > 0. && width < 420. { 16. } else { 24. }))
            .rounded(crate::foundation::surface_radius(cx))
            .bg(p.panel)
            .border_1()
            .border_color(p.line)
            .text_color(p.ink)
            .child(
                column()
                    .gap(px(2.))
                    .child(title(self.label.clone(), p))
                    .when_some(self.meta.clone(), |heading, meta| {
                        heading.child(
                            div()
                                .text_size(px(13.))
                                .line_height(px(18.))
                                .text_color(p.faint)
                                .child(meta),
                        )
                    }),
            )
            .when_some(head, |card, head| card.child(head))
            .child(chart)
            .when_some(legend, |card, legend| card.child(legend))
            .when_some(controls, |card, controls| card.child(controls))
            .child(readings)
    }
}

#[cfg(test)]
mod tests {
    use super::{BAR_H, RaceBarChart, RaceEntry, RaceFrame, compact_value, nice_step};
    use gpui::{TestAppContext, px, rgb, size};
    #[test]
    fn play_button_click_toggles_playback() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) = app.add_window_view(|_, _| {
            RaceBarChart::new(
                "Race",
                vec![RaceEntry::new("a", "A"), RaceEntry::new("b", "B")],
            )
            .with_autoplay(false)
            .with_frames(vec![
                RaceFrame::new("one", vec![("a".into(), 10.), ("b".into(), 30.)]),
                RaceFrame::new("two", vec![("a".into(), 40.), ("b".into(), 20.)]),
            ])
        });
        w.run_until_parked();
        let target = w.debug_bounds("Play").unwrap().center();
        w.simulate_click(target, gpui::Modifiers::default());
        w.run_until_parked();
        assert!(
            w.update(|_, cx| view.read(cx).playing),
            "clicking the play face should start playback"
        );
        let playing = w.update(|_, cx| view.read(cx).playing);
        let target = w
            .debug_bounds(if playing { "Pause" } else { "Play" })
            .unwrap()
            .center();
        w.simulate_click(target, gpui::Modifiers::default());
        w.run_until_parked();
        assert!(
            !w.update(|_, cx| view.read(cx).playing),
            "clicking again should pause playback"
        );
    }
    #[test]
    fn timeline_interpolates_host_values_and_treats_missing_as_zero() {
        let entries = vec![RaceEntry::new("a", "A"), RaceEntry::new("b", "B")];
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
    #[test]
    fn compact_values_match_the_reference_format() {
        assert_eq!(compact_value(14_000., "$"), "$14K");
        assert_eq!(compact_value(660_000., "$"), "$660K");
        assert_eq!(compact_value(3_872_000., "$"), "$3.87M");
        assert_eq!(compact_value(999., ""), "999");
    }
    #[test]
    fn axis_steps_read_as_1_2_5_times_powers_of_ten() {
        assert_eq!(nice_step(0.4), 0.2);
        assert_eq!(nice_step(3.2), 2.);
        assert_eq!(nice_step(7.5), 5.);
        assert_eq!(nice_step(220_000.), 200_000.);
    }
    #[test]
    fn crossing_a_tick_step_keeps_the_leader_at_the_same_painted_bounds() {
        let mut app = TestAppContext::single();
        app.update(|cx| {
            crate::init(cx);
            crate::Theme::global_mut(cx).reduced_motion = true;
        });
        let (view, w) = app.add_window_view(|_, _| {
            RaceBarChart::new("Race", vec![RaceEntry::new("a", "Leader")])
                .with_autoplay(false)
                .with_frames(vec![
                    RaceFrame::new("May", vec![("a".into(), 199_990.)]),
                    RaceFrame::new("September", vec![("a".into(), 200_010.)]),
                ])
        });
        w.simulate_resize(size(px(630.), px(600.)));
        w.run_until_parked();
        w.update(|win, cx| {
            win.simulate_next_frame(cx);
            win.refresh();
        });
        w.run_until_parked();
        let leader = |w: &mut gpui::VisualTestContext| {
            w.update(|w, _| {
                let scale = w.scale_factor();
                w.painted_quads()
                    .into_iter()
                    .filter(|quad| (quad.bounds.size.height.0 / scale - BAR_H).abs() < 0.5)
                    .max_by(|a, b| a.bounds.size.width.0.total_cmp(&b.bounds.size.width.0))
                    .expect("leader bar should be painted")
                    .bounds
            })
        };
        let before = leader(w);
        w.update(|_, cx| view.update(cx, |chart, cx| chart.set_frame(1, cx)));
        w.run_until_parked();
        assert_eq!(leader(w), before);
    }
    #[test]
    fn group_tones_follow_the_series_palette() {
        let entries = vec![
            RaceEntry::new("a", "A").with_group("furniture"),
            RaceEntry::new("b", "B").with_group("lighting"),
        ];
        let chart = RaceBarChart::new("Race", entries).with_groups(vec![
            ("furniture".into(), "Furniture".into()),
            ("lighting".into(), "Lighting".into()),
        ]);
        let tone = chart.group_tone(&chart.data[1]);
        let expected: gpui::Hsla = rgb(0x9c37be).into();
        assert_eq!(tone, expected);
    }
}
