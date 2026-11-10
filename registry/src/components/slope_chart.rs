//! Compare paired measurements with animated two-period slope lines.
use crate::primitives::{
    family::{Paint, PhaseClock, Slide, disabled_motion},
    glyph::{Glyph, icon, transformed},
    presentation::{column, in_out, title, tr},
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};

/// The story item is drawn in the shared first-series hue.
const HIGHLIGHT: u32 = 0x55adff;
/// Heading row above the plot.
const HEAD: f32 = 28.;
/// Vertical padding kept so dots never touch the plot edges.
const PAD: f32 = 14.;
/// Minimum centre distance between two spread labels.
const LABEL_GAP: f32 = 20.;
/// Pointer distance at which a line or label is still read.
const NEAR: f32 = 22.;
/// Visual settle time when a dataset switch morphs lines and labels.
const MORPH_SECONDS: f32 = 0.46;
/// Tooltip glide time between lines in focus.
const TIP_SECONDS: f32 = 0.26;
/// Tooltip fade-out time once nothing is in focus.
const TIP_FADE: f32 = 0.16;
/// Estimated tooltip box used to keep it inside the chart.
const TIP_W: f32 = 180.;
const TIP_H: f32 = 104.;

/// A stable series measured across exactly two periods.
///
/// The key keeps a line stable across datasets, so switching data morphs it.
#[derive(Clone, Debug)]
pub struct SlopeSeries {
    pub id: SharedString,
    pub label: SharedString,
    pub before: f32,
    pub after: f32,
}
impl SlopeSeries {
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        before: f32,
        after: f32,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            before: if before.is_finite() { before } else { 0. },
            after: if after.is_finite() { after } else { 0. },
        }
    }
}
/// A series entered focus, by pointer, touch or keyboard.
#[derive(Clone, Debug)]
pub struct SlopeInspect {
    pub id: SharedString,
    pub before: f32,
    pub after: f32,
}
struct SlopeVisual {
    id: SharedString,
    before: Slide,
    after: Slide,
    /// Rank move beside the end value: "=" or the moved places.
    rank: SharedString,
    /// The text rolling out while the new one rolls in.
    prev_rank: SharedString,
    /// Restarted when the rank text changes.
    roll: PhaseClock,
    /// A rise or level move rolls in from below; a fall rolls in from above.
    roll_from_below: bool,
}
impl SlopeVisual {
    fn roll_p(&self, cx: &App) -> f32 {
        (self.roll.age(cx) / 0.26).clamp(0., 1.)
    }
}
/// A before-and-after slope chart with a focus item, rank moves and a gliding tooltip.
pub struct SlopeChart {
    label: SharedString,
    meta: Option<SharedString>,
    start_label: SharedString,
    end_label: SharedString,
    series: Vec<SlopeSeries>,
    visual: Vec<SlopeVisual>,
    active: Option<usize>,
    highlight: Option<SharedString>,
    ranks: bool,
    value_prefix: SharedString,
    value_suffix: SharedString,
    row_height: f32,
    empty_label: SharedString,
    bounds: Rc<Cell<Bounds<Pixels>>>,
    initialized: bool,
    draw: PhaseClock,
    revealed: bool,
    visible: bool,
    touching: Rc<Cell<bool>>,
    tip_x: Slide,
    tip_y: Slide,
    /// The row whose content the tooltip keeps while it fades out.
    tip_shown: Option<usize>,
    /// Restarted when the fade-out begins.
    tip_fade: PhaseClock,
    tip_fade_on: bool,
}
impl EventEmitter<SlopeInspect> for SlopeChart {}
impl SlopeChart {
    pub fn new(label: impl Into<SharedString>, series: Vec<SlopeSeries>) -> Self {
        Self {
            label: label.into(),
            meta: None,
            start_label: "Before".into(),
            end_label: "After".into(),
            series,
            visual: vec![],
            active: None,
            highlight: None,
            ranks: true,
            value_prefix: "".into(),
            value_suffix: "".into(),
            row_height: 44.,
            empty_label: "No data".into(),
            bounds: Default::default(),
            initialized: false,
            draw: PhaseClock::default(),
            revealed: false,
            visible: true,
            touching: Default::default(),
            tip_x: Slide::new(0.),
            tip_y: Slide::new(0.),
            tip_shown: None,
            tip_fade: PhaseClock::default(),
            tip_fade_on: false,
        }
    }
    /// Secondary line under the heading.
    pub fn with_meta(mut self, meta: impl Into<SharedString>) -> Self {
        self.meta = Some(meta.into());
        self
    }
    /// Heading of the first column, such as "Q1".
    pub fn with_start_label(mut self, label: impl Into<SharedString>) -> Self {
        self.start_label = label.into();
        self
    }
    /// Heading of the second column, such as "Q2".
    pub fn with_end_label(mut self, label: impl Into<SharedString>) -> Self {
        self.end_label = label.into();
        self
    }
    /// The item drawn in the shared first-series hue.
    pub fn with_highlight(mut self, id: impl Into<SharedString>) -> Self {
        self.highlight = Some(id.into());
        self
    }
    /// Rank movement beside each end value. Defaults to shown.
    pub fn with_ranks(mut self, ranks: bool) -> Self {
        self.ranks = ranks;
        self
    }
    /// Text placed before every formatted value, such as "$".
    pub fn with_value_prefix(mut self, prefix: impl Into<SharedString>) -> Self {
        self.value_prefix = prefix.into();
        self
    }
    /// Text placed after every formatted value, such as "%".
    pub fn with_value_suffix(mut self, suffix: impl Into<SharedString>) -> Self {
        self.value_suffix = suffix.into();
        self
    }
    /// Plot height in pixels per item. Defaults to 44.
    pub fn with_row_height(mut self, row_height: f32) -> Self {
        self.row_height = row_height.max(20.);
        self
    }
    /// Message shown when there are no items.
    pub fn with_empty_label(mut self, label: impl Into<SharedString>) -> Self {
        self.empty_label = label.into();
        self
    }
    /// The host sets this to false when the chart or its tab is hidden.
    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        if self.visible != visible {
            self.visible = visible;
            cx.notify();
        }
    }
    /// Switch datasets; matched keys morph their lines, dots and labels.
    pub fn set_series(&mut self, series: Vec<SlopeSeries>, cx: &mut Context<Self>) {
        let kept = self
            .active
            .and_then(|i| self.visual.get(i))
            .map(|v| v.id.clone());
        self.series = series;
        self.active = kept.and_then(|id| {
            self.series
                .iter()
                .position(|s| s.id == id)
                .filter(|&i| self.series.get(i).is_some())
        });
        let old = std::mem::take(&mut self.visual);
        let mut old: Vec<Option<SlopeVisual>> = old.into_iter().map(Some).collect();
        self.visual = self
            .series
            .iter()
            .map(|s| {
                let index = old
                    .iter()
                    .position(|x| x.as_ref().is_some_and(|x| x.id == s.id));
                let mut state = index
                    .and_then(|i| old[i].take())
                    .unwrap_or_else(|| SlopeVisual {
                        id: s.id.clone(),
                        before: Slide::new(s.before),
                        after: Slide::new(s.before),
                        rank: rank_move_text(&self.series, s),
                        prev_rank: "".into(),
                        roll: PhaseClock::default(),
                        roll_from_below: true,
                    });
                state.before.duration(s.before, MORPH_SECONDS, cx);
                state.after.duration(s.after, MORPH_SECONDS, cx);
                let text = rank_move_text(&self.series, s);
                if state.rank.is_empty() {
                    state.rank = text;
                } else if state.rank != text {
                    state.prev_rank = state.rank.clone();
                    state.rank = text;
                    state.roll.restart(cx);
                    state.roll_from_below = rank_move(&self.series, &s.id) >= 0;
                }
                state
            })
            .collect();
        self.initialized = true;
        cx.notify();
    }
    fn inspect(&mut self, next: Option<usize>, cx: &mut Context<Self>) {
        if self.active == next {
            return;
        }
        self.active = next;
        if let Some(item) = next.and_then(|i| self.series.get(i)) {
            cx.emit(SlopeInspect {
                id: item.id.clone(),
                before: item.before,
                after: item.after,
            });
        }
        cx.notify();
    }
}
/// Rank in a column: 1 for the largest value; ties share a rank.
fn rank_of(series: &[SlopeSeries], field: fn(&SlopeSeries) -> f32, id: &SharedString) -> usize {
    let own = series.iter().find(|s| &s.id == id).map(field).unwrap_or(0.);
    series.iter().filter(|s| field(s) > own).count() + 1
}
/// Positive when the item moved up between the two periods.
fn rank_move(series: &[SlopeSeries], id: &SharedString) -> isize {
    let before = rank_of(series, |s| s.before, id);
    let after = rank_of(series, |s| s.after, id);
    before as isize - after as isize
}
fn rank_move_text(series: &[SlopeSeries], item: &SlopeSeries) -> SharedString {
    let move_places = rank_move(series, &item.id);
    if move_places == 0 {
        "=".into()
    } else {
        format!("{}", move_places.abs()).into()
    }
}
/// Items in end-value order, largest first.
fn end_order(series: &[SlopeSeries]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..series.len()).collect();
    order.sort_by(|&a, &b| series[b].after.total_cmp(&series[a].after));
    order
}
/// One decimal at most, trailing zero dropped: 4.0 → "4", 3.1 → "3.1".
fn grouped1(value: f32) -> String {
    let text = format!("{:.1}", value);
    text.strip_suffix(".0").unwrap_or(&text).to_string()
}
/// Reference change format: "+" or a true minus sign, then the difference.
fn change_text(diff: f32, prefix: &str, suffix: &str) -> String {
    let sign = if diff >= 0. { "+" } else { "\u{2212}" };
    format!("{sign}{prefix}{}{suffix}", grouped1(diff.abs()))
}
fn value_text(value: f32, prefix: &str, suffix: &str) -> String {
    format!("{prefix}{}{suffix}", grouped1(value))
}
fn rank_line(r0: usize, r1: usize) -> String {
    if r0 == r1 {
        "same rank".to_string()
    } else if r1 < r0 {
        format!("up {} to rank {}", r0 - r1, r1)
    } else {
        format!("down {} to rank {}", r1 - r0, r1)
    }
}
/// Moves labels apart just enough: touching labels form one group centred on
/// where its members want to be, clamped to the plot.
fn spread_labels(wanted: &[f32], height: f32) -> Vec<f32> {
    let gap = LABEL_GAP;
    let low = 8.;
    let high = (height - 8.).max(low);
    let mut order: Vec<usize> = (0..wanted.len()).collect();
    order.sort_by(|&a, &b| wanted[a].total_cmp(&wanted[b]));
    let mut groups: Vec<(Vec<usize>, f32)> = order.iter().map(|&i| (vec![i], wanted[i])).collect();
    let mut changed = true;
    while changed {
        changed = false;
        let mut next: Vec<(Vec<usize>, f32)> = Vec::new();
        for group in groups {
            if let Some(prev) = next
                .last_mut()
                .filter(|prev| prev.1 + prev.0.len() as f32 * gap > group.1)
            {
                let members = prev
                    .0
                    .iter()
                    .chain(group.0.iter())
                    .copied()
                    .collect::<Vec<_>>();
                let top = members
                    .iter()
                    .enumerate()
                    .map(|(k, &i)| wanted[i] - k as f32 * gap)
                    .sum::<f32>()
                    / members.len() as f32;
                *prev = (members, top);
                changed = true;
                continue;
            }
            next.push(group);
        }
        for group in next.iter_mut() {
            group.1 = group
                .1
                .clamp(low, (high - (group.0.len() - 1) as f32 * gap).max(low));
        }
        groups = next;
    }
    let mut out = vec![0.; wanted.len()];
    for group in &groups {
        for (k, &i) in group.0.iter().enumerate() {
            out[i] = group.1 + k as f32 * gap;
        }
    }
    out
}
/// The tooltip anchor: the line's middle, kept above when it fits, inside otherwise.
fn tip_anchor(mid: f32, plot_height: f32, width: f32, x0: f32, x1: f32) -> (f32, f32) {
    let above = mid - TIP_H - 16. >= 0.;
    let left = ((x0 + x1) * 0.5 - TIP_W * 0.5).clamp(0., (width - TIP_W).max(0.));
    let top = (HEAD + if above { mid - TIP_H - 16. } else { mid + 16. })
        .clamp(0., (HEAD + plot_height - TIP_H).max(0.));
    (left, top)
}
/// The row nearest the pointer: along the line inside the plot, or beside its label.
fn nearest_series(
    b: Bounds<Pixels>,
    pos: Point<Pixels>,
    rows: &[(f32, f32)],
    l0: &[f32],
    l1: &[f32],
    x0: f32,
    x1: f32,
) -> Option<usize> {
    let width = f32::from(b.size.width).max(1.);
    let height = f32::from(b.size.height).max(1.);
    let x = f32::from(pos.x - b.origin.x);
    let y = f32::from(pos.y - b.origin.y);
    if x < 0. || x > width || y < -4. || y > height + 4. {
        return None;
    }
    let t = ((x - x0) / (x1 - x0).max(1.)).clamp(0., 1.);
    rows.iter()
        .enumerate()
        .map(|(i, (a, z))| {
            let on_line = (a + (z - a) * t - y).abs();
            let on_label = if x < x0 {
                (l0.get(i).copied().unwrap_or(0.) - y).abs()
            } else if x > x1 {
                (l1.get(i).copied().unwrap_or(0.) - y).abs()
            } else {
                f32::INFINITY
            };
            (i, on_line.min(on_label))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .filter(|(_, d)| *d <= NEAR)
        .map(|(i, _)| i)
}
fn intersects_viewport(b: Bounds<Pixels>, viewport: Size<Pixels>) -> bool {
    b.size.width > px(0.)
        && b.size.height > px(0.)
        && b.origin.x < viewport.width
        && b.origin.y < viewport.height
        && b.origin.x + b.size.width > px(0.)
        && b.origin.y + b.size.height > px(0.)
}
/// Left and right label room; the rank column releases width when hidden.
fn gutters(width: f32, ranks: bool) -> (f32, f32) {
    let narrow = width < 440.;
    let left = if !narrow {
        168.
    } else if width < 300. {
        116.
    } else {
        132.
    };
    let right = if ranks {
        if narrow { 76. } else { 96. }
    } else if narrow {
        52.
    } else {
        64.
    };
    (left, right)
}
/// Axis coordinates from the label rooms: x0 sits at the left room, x1 that
/// far from the right edge, never closer than 40px to x0.
fn axes(width: f32, ranks: bool) -> (f32, f32) {
    let (left, right) = gutters(width, ranks);
    (left, (width - right).max(left + 40.))
}
impl SlopeChart {
    fn value_span(&self) -> (f32, f32) {
        let mut lo = f32::INFINITY;
        let mut hi = f32::NEG_INFINITY;
        for s in &self.series {
            lo = lo.min(s.before).min(s.after);
            hi = hi.max(s.before).max(s.after);
        }
        if !lo.is_finite() || !hi.is_finite() {
            return (0., 1.);
        }
        if hi - lo < 1e-9 {
            return (lo - 1., hi + 1.);
        }
        (lo, hi)
    }
}
fn highlight_color() -> Hsla {
    rgb(HIGHLIGHT).into()
}
impl Render for SlopeChart {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let empty = self.series.is_empty();
        let plot_height = (self.series.len() as f32 * self.row_height).max(200.);
        if !self.initialized {
            self.initialized = true;
            self.visual = self
                .series
                .iter()
                .map(|s| SlopeVisual {
                    id: s.id.clone(),
                    before: Slide::new(s.before),
                    after: Slide::new(s.after),
                    rank: rank_move_text(&self.series, s),
                    prev_rank: "".into(),
                    roll: PhaseClock::default(),
                    roll_from_below: true,
                })
                .collect();
        }
        let measured = self.bounds.get();
        let on_screen =
            self.visible && w.is_visible() && intersects_viewport(measured, w.viewport_size());
        if on_screen && !self.revealed {
            self.revealed = true;
            self.draw.restart(cx);
        }
        let now = cx.background_executor().now();
        let reduced = disabled_motion(cx);
        let age = if reduced {
            100.
        } else if self.revealed {
            self.draw.age(cx)
        } else {
            0.
        };
        let line_done = 0.1 + self.series.len().saturating_sub(1) as f32 * 0.05 + 0.7;
        let mut moving = false;
        let p = Paint::get(cx);
        let mut values: Vec<(f32, f32)> = Vec::with_capacity(self.visual.len());
        for v in &self.visual {
            let (before, a) = v.before.sample_batch(now, reduced);
            let (after, b) = v.after.sample_batch(now, reduced);
            moving |= a || b;
            values.push((before, after));
        }
        for v in &self.visual {
            moving |= v.roll.age(cx) < 0.3;
        }
        let width = if measured.size.width > px(0.) {
            f32::from(measured.size.width)
        } else {
            720.
        };
        let (x0, x1) = axes(width, self.ranks);
        let (lo, hi) = self.value_span();
        let span = (hi - lo).max(1e-9);
        let y_of = |v: f32| PAD + (1. - (v - lo) / span) * (plot_height - PAD * 2.);
        let l0 = spread_labels(
            &values.iter().map(|(v, _)| y_of(*v)).collect::<Vec<_>>(),
            plot_height,
        );
        let l1 = spread_labels(
            &values.iter().map(|(_, v)| y_of(*v)).collect::<Vec<_>>(),
            plot_height,
        );
        let rows: Vec<(f32, f32)> = values.iter().map(|(a, z)| (y_of(*a), y_of(*z))).collect();
        let by_end = end_order(&self.series);
        let active = self.active;
        let highlight = self.highlight.clone();
        let prefix = self.value_prefix.to_string();
        let suffix = self.value_suffix.to_string();
        let accent_flags: Vec<bool> = self
            .series
            .iter()
            .map(|s| highlight.as_ref().is_some_and(|h| h == &s.id))
            .collect();
        // Tooltip: glide to the line in focus, keep content while fading out.
        let tip_fading = self.tip_shown.is_some() && active.is_none();
        let tip_opacity = if tip_fading {
            if !self.tip_fade_on {
                self.tip_fade_on = true;
                self.tip_fade.restart(cx);
            }
            (1. - self.tip_fade.age(cx) / TIP_FADE).clamp(0., 1.)
        } else {
            1.
        };
        if tip_fading {
            let fade_age = self.tip_fade.age(cx);
            moving |= fade_age < TIP_FADE + 0.05;
            if fade_age > TIP_FADE {
                self.tip_shown = None;
                self.tip_fade_on = false;
            }
        }
        if let Some(i) = active {
            let mid = (rows[i].0 + rows[i].1) * 0.5;
            let (left, top) = tip_anchor(mid, plot_height, width, x0, x1);
            if self.tip_shown != Some(i) && self.tip_shown.is_none() {
                self.tip_x.reset(left);
                self.tip_y.reset(top);
            } else {
                self.tip_x.duration(left, TIP_SECONDS, cx);
                self.tip_y.duration(top, TIP_SECONDS, cx);
            }
            self.tip_shown = Some(i);
            self.tip_fade_on = false;
        }
        let (tip_left, tip_moving_x) = self.tip_x.sample_batch(now, reduced);
        let (tip_top, tip_moving_y) = self.tip_y.sample_batch(now, reduced);
        moving |= tip_moving_x || tip_moving_y;
        if on_screen && (moving || (!reduced && self.revealed && age < line_done + 0.9)) {
            w.request_animation_frame();
        }
        // Readings for assistive technology.
        let announce = active
            .and_then(|i| self.series.get(i))
            .map(|s| {
                let r0 = rank_of(&self.series, |x| x.before, &s.id);
                let r1 = rank_of(&self.series, |x| x.after, &s.id);
                format!(
                    "{}, {} {}, {} {}, {}, {}",
                    s.label,
                    self.start_label,
                    value_text(s.before, &prefix, &suffix),
                    self.end_label,
                    value_text(s.after, &prefix, &suffix),
                    change_text(s.after - s.before, &prefix, &suffix),
                    rank_line(r0, r1),
                )
            })
            .unwrap_or_default();
        let summary = if empty {
            format!("{}. {}.", self.label, self.empty_label)
        } else {
            let items = by_end
                .iter()
                .filter_map(|&i| self.series.get(i))
                .map(|s| {
                    format!(
                        "{} {} to {}",
                        s.label,
                        value_text(s.before, &prefix, &suffix),
                        value_text(s.after, &prefix, &suffix)
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "{}. {} to {}. {items}.",
                self.label, self.start_label, self.end_label
            )
        };
        let table_rows: Vec<String> = by_end
            .iter()
            .filter_map(|&i| self.series.get(i))
            .map(|s| {
                let r0 = rank_of(&self.series, |x| x.before, &s.id);
                let r1 = rank_of(&self.series, |x| x.after, &s.id);
                format!(
                    "{}: {} to {}, {}, rank {} to {}",
                    s.label,
                    value_text(s.before, &prefix, &suffix),
                    value_text(s.after, &prefix, &suffix),
                    change_text(s.after - s.before, &prefix, &suffix),
                    r0,
                    r1,
                )
            })
            .collect();
        // Per-row paint colours resolved once for the canvas and the labels.
        let row_inks: Vec<Hsla> = (0..self.series.len())
            .map(|i| {
                let focused = active == Some(i);
                let accent = accent_flags.get(i).copied().unwrap_or(false);
                if accent {
                    highlight_color()
                } else if focused {
                    p.ink
                } else {
                    p.ink.opacity(0.36)
                }
            })
            .collect();
        let p_bounds = self.bounds.clone();
        let hover_bounds = self.bounds.clone();
        let touch_bounds = self.bounds.clone();
        let touch_rows = rows.clone();
        let touch_l0 = l0.clone();
        let touch_l1 = l1.clone();
        let ranks_shown = self.ranks;
        let plot_left_labels = l0.clone();
        let plot_right_labels = l1.clone();
        let paint_values = values.clone();
        let paint_inks = row_inks.clone();
        let nearest_at = Rc::new(move |b: Bounds<Pixels>, pos: Point<Pixels>| {
            nearest_series(b, pos, &touch_rows, &touch_l0, &touch_l1, x0, x1)
        });
        let hover_nearest = nearest_at.clone();
        let touch_nearest = nearest_at.clone();
        let touching = self.touching.clone();
        let touch = cx.listener(move |s, e: &TouchDragEvent, _, cx| {
            let index = match e.phase {
                TouchPhase::Ended | TouchPhase::Cancelled => None,
                _ => touch_nearest(touch_bounds.get(), e.position),
            };
            s.inspect(index, cx);
        });
        let plot = div()
            .id("slope-plot")
            .relative()
            .w_full()
            .h(px(plot_height))
            .on_hover(cx.listener(|s, over: &bool, w, cx| {
                if !*over && !w.last_input_was_keyboard() {
                    s.inspect(None, cx);
                }
            }))
            .on_mouse_move(cx.listener(move |s, e: &MouseMoveEvent, _, cx| {
                s.inspect(hover_nearest(hover_bounds.get(), e.position), cx);
            }))
            .child(
                canvas(
                    move |b, w, _| {
                        let previous = p_bounds.get();
                        if previous.size != b.size
                            || intersects_viewport(previous, w.viewport_size())
                                != intersects_viewport(b, w.viewport_size())
                        {
                            w.request_animation_frame();
                        }
                        p_bounds.set(b);
                    },
                    move |b, _, w, _| {
                        let width = f32::from(b.size.width);
                        let height = f32::from(b.size.height);
                        let (x0, x1) = axes(width, ranks_shown);
                        let y_of = |v: f32| PAD + (1. - (v - lo) / span) * (height - PAD * 2.);
                        // Axes sit on whole pixels like the reference's
                        // crispEdges so a 1px rule stays a solid column.
                        for x in [x0, x1] {
                            let col = (b.origin.x + px(x)).round() - px(0.5);
                            let top = b.origin.y.round();
                            let bottom = (b.origin.y + b.size.height).round();
                            w.paint_quad(fill(
                                Bounds::new(point(col, top), size(px(1.), bottom - top)),
                                p.line,
                            ));
                        }
                        for (i, (a, z)) in paint_values.iter().enumerate() {
                            let focused = active == Some(i);
                            let dim = active.is_some_and(|on| on != i);
                            let opacity = if dim { 0.14 } else { 1. };
                            let ink = paint_inks[i];
                            let _ = focused;
                            let at = |x: f32, value: f32| {
                                point(b.origin.x + px(x), b.origin.y + px(y_of(value)))
                            };
                            let left = at(x0, *a);
                            let end = at(x1, *z);
                            // Leader hairlines from each spread label to its dot.
                            let leaders = [
                                (
                                    point(
                                        left.x - px(6.),
                                        b.origin.y
                                            + px(plot_left_labels
                                                .get(i)
                                                .copied()
                                                .unwrap_or(0.)
                                                .clamp(0., height)),
                                    ),
                                    point(left.x - px(1.), left.y),
                                ),
                                (
                                    point(end.x + px(1.), end.y),
                                    point(
                                        end.x + px(6.),
                                        b.origin.y
                                            + px(plot_right_labels
                                                .get(i)
                                                .copied()
                                                .unwrap_or(0.)
                                                .clamp(0., height)),
                                    ),
                                ),
                            ];
                            for (from, to) in leaders {
                                if (f32::from(from.y - to.y)).abs() < 0.75 {
                                    continue;
                                }
                                let mut leader = PathBuilder::stroke(px(1.));
                                leader.move_to(from);
                                leader.line_to(to);
                                if let Ok(path) = leader.build() {
                                    w.paint_path(path, p.line.opacity(opacity));
                                }
                            }
                            // The line draws in from start to end with a small stagger.
                            let delay = 0.1 + i as f32 * 0.05;
                            let progress = in_out(((age - delay) / 0.7).clamp(0., 1.));
                            let tip = point(
                                left.x + (end.x - left.x) * progress,
                                left.y + (end.y - left.y) * progress,
                            );
                            let mut line = PathBuilder::stroke(px(2.));
                            line.move_to(left);
                            line.line_to(tip);
                            if let Ok(path) = line.build() {
                                w.paint_path(path, ink.opacity(opacity));
                            }
                            // Dots arrive with the line: start immediately, end as it lands.
                            for (pt, dot_delay) in
                                [(left, i as f32 * 0.05), (tip, 0.7 + i as f32 * 0.05)]
                            {
                                let scale = if reduced {
                                    1.
                                } else {
                                    ((age - dot_delay) / 0.2).clamp(0., 1.)
                                };
                                if scale <= 0.01 {
                                    continue;
                                }
                                let r = 4. * scale;
                                let ring = r + 2.;
                                w.paint_quad(
                                    fill(
                                        Bounds::new(
                                            point(pt.x - px(ring), pt.y - px(ring)),
                                            size(px(ring * 2.), px(ring * 2.)),
                                        ),
                                        p.panel,
                                    )
                                    .corner_radii(px(ring)),
                                );
                                w.paint_quad(
                                    fill(
                                        Bounds::new(
                                            point(pt.x - px(r), pt.y - px(r)),
                                            size(px(r * 2.), px(r * 2.)),
                                        ),
                                        ink.opacity(opacity),
                                    )
                                    .corner_radii(px(r)),
                                );
                            }
                        }
                    },
                )
                .size_full(),
            )
            .children({
                let mut labels = Vec::new();
                for (i, series) in self.series.iter().enumerate() {
                    let focused = active == Some(i);
                    let accent = accent_flags.get(i).copied().unwrap_or(false);
                    let dim = active.is_some_and(|on| on != i);
                    let opacity = if dim { 0.3 } else { 1. };
                    let narrow = width < 440.;
                    let label_size = if narrow { 12. } else { 14. };
                    let label_gap = if narrow { 6. } else { 8. };
                    let name_color = if focused || accent {
                        p.ink
                    } else {
                        p.secondary
                    };
                    let visual = &self.visual[i];
                    let enter = if reduced {
                        1.
                    } else {
                        ((age - i as f32 * 0.05) / 0.3).clamp(0., 1.)
                    };
                    let step = if reduced {
                        1.
                    } else {
                        ((age - 0.7 - i as f32 * 0.05) / 0.3).clamp(0., 1.)
                    };
                    labels.push(
                        div()
                            .absolute()
                            .left(px(0.))
                            .top(px((l0[i] - 10.).max(0.)))
                            .w(px((x0 - 10.).max(1.)))
                            .h(px(20.))
                            .flex()
                            .items_center()
                            .justify_end()
                            .gap(px(label_gap))
                            .overflow_hidden()
                            .text_size(px(label_size))
                            .line_height(px(20.))
                            .opacity(opacity * enter)
                            .child(
                                div()
                                    .min_w_0()
                                    .overflow_hidden()
                                    .text_ellipsis()
                                    .text_color(name_color)
                                    .child(series.label.clone()),
                            )
                            .child(
                                div()
                                    .flex_none()
                                    .text_color(p.ink)
                                    .font_weight(if accent {
                                        FontWeight::MEDIUM
                                    } else {
                                        FontWeight::NORMAL
                                    })
                                    .child(value_text(series.before, &prefix, &suffix)),
                            ),
                    );
                    // Rank move: an arrow glyph and a number that rolls the way it moved.
                    let move_places = rank_move(&self.series, &series.id);
                    let roll_p = visual.roll_p(cx);
                    let dir: f32 = if visual.roll_from_below { 1. } else { -1. };
                    let new_off = dir * 6. * (1. - roll_p);
                    let old_off = -dir * 6. * roll_p;
                    let rank_color = if move_places > 0 {
                        p.secondary
                    } else {
                        p.faint
                    };
                    let arrow = if move_places > 0 {
                        Some(icon(Glyph::ArrowUp, 8., rank_color).into_any_element())
                    } else if move_places < 0 {
                        Some(
                            transformed(
                                Glyph::ArrowUp,
                                8.,
                                rank_color,
                                std::f32::consts::PI,
                                1.,
                                (0., 0.),
                                1.,
                            )
                            .into_any_element(),
                        )
                    } else {
                        None
                    };
                    let rank_number = div()
                        .relative()
                        .h(px(16.))
                        .min_w(px(12.))
                        .when_some(
                            (!visual.prev_rank.is_empty() && roll_p < 1.).then(|| {
                                div()
                                    .absolute()
                                    .left_0()
                                    .top(px(old_off))
                                    .text_color(rank_color)
                                    .opacity(1. - roll_p)
                                    .child(visual.prev_rank.clone())
                            }),
                            |row, old| row.child(old),
                        )
                        .child(
                            div()
                                .absolute()
                                .left_0()
                                .top(px(new_off))
                                .text_color(rank_color)
                                .opacity(roll_p)
                                .child(visual.rank.clone()),
                        );
                    labels.push(
                        div()
                            .absolute()
                            .left(px(x1 + 10.))
                            .top(px((l1[i] - 10.).max(0.)))
                            .h(px(20.))
                            .flex()
                            .items_center()
                            .gap(px(label_gap))
                            .text_size(px(label_size))
                            .line_height(px(20.))
                            .opacity(opacity * step)
                            .child(
                                div()
                                    .flex_none()
                                    .text_color(p.ink)
                                    .font_weight(if accent {
                                        FontWeight::MEDIUM
                                    } else {
                                        FontWeight::NORMAL
                                    })
                                    .child(value_text(series.after, &prefix, &suffix)),
                            )
                            .when(self.ranks, |row| {
                                row.child(
                                    div()
                                        .min_w(px(30.))
                                        .flex()
                                        .items_center()
                                        .gap(px(2.))
                                        .text_size(px(12.))
                                        .line_height(px(16.))
                                        .when_some(arrow, |rank, glyph| rank.child(glyph))
                                        .child(rank_number),
                                )
                            }),
                    );
                }
                labels
            })
            .child(
                canvas(
                    |b, w, _| w.insert_hitbox(b, HitboxBehavior::Normal),
                    move |_, hit, w, _| {
                        w.on_mouse_event(move |e: &TouchDragEvent, phase, w, cx| {
                            if !phase.bubble() {
                                return;
                            }
                            if e.phase == TouchPhase::Started {
                                touching.set(hit.is_hovered(w));
                            }
                            if !touching.get() {
                                return;
                            }
                            if matches!(e.phase, TouchPhase::Started | TouchPhase::Moved) {
                                touch(e, w, cx);
                            }
                            if matches!(e.phase, TouchPhase::Ended | TouchPhase::Cancelled) {
                                touching.set(false);
                            }
                        });
                    },
                )
                .absolute()
                .inset_0(),
            );
        // The stage carries the column headings, the plot and the tooltip.
        let stage = div()
            .id("slope-stage")
            .relative()
            .w_full()
            .role(Role::Group)
            .tab_index(if empty { -1 } else { 0 })
            .aria_label(format!(
                "{}. {}",
                self.label,
                tr(
                    cx,
                    "Use up and down arrows to move between items in end-value order",
                    "使用上下方向键按期末数值顺序在各项目间移动"
                )
            ))
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                let n = s.series.len();
                if n == 0 {
                    return;
                }
                let order = end_order(&s.series);
                let at = s
                    .active
                    .and_then(|i| order.iter().position(|&x| x == i))
                    .map(|o| o as isize)
                    .unwrap_or(-1);
                let target = match e.keystroke.key.as_str() {
                    "escape" if s.active.is_some() => None,
                    "down" | "right" => Some((at + 1).clamp(0, n as isize - 1) as usize),
                    "up" | "left" => {
                        Some((if at < 0 { 0 } else { at - 1 }).clamp(0, n as isize - 1) as usize)
                    }
                    "home" => Some(0),
                    "end" => Some(n - 1),
                    _ => return,
                };
                w.prevent_default();
                cx.stop_propagation();
                s.inspect(target.map(|o| order[o.min(n - 1)]), cx);
            }))
            .child(div().relative().h(px(HEAD)).when(!empty, |columns| {
                columns
                    .child(
                        div()
                            .absolute()
                            .left(px((x0 - 40.).max(0.)))
                            .top(px(4.))
                            .w(px(80.))
                            .text_center()
                            .text_size(px(14.))
                            .line_height(px(20.))
                            .text_color(p.faint)
                            .child(self.start_label.clone()),
                    )
                    .child(
                        div()
                            .absolute()
                            .left(px((x1 - 40.).max(0.)))
                            .top(px(4.))
                            .w(px(80.))
                            .text_center()
                            .text_size(px(14.))
                            .line_height(px(20.))
                            .text_color(p.faint)
                            .child(self.end_label.clone()),
                    )
            }))
            .child(if empty {
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .min_h(px(160.))
                    .text_size(px(14.))
                    .line_height(px(20.))
                    .text_color(p.faint)
                    .child(self.empty_label.clone())
                    .into_any_element()
            } else {
                plot.into_any_element()
            })
            .when_some(
                self.tip_shown
                    .filter(|_| !empty)
                    .and_then(|i| self.series.get(i).cloned())
                    .map(|series| {
                        let r0 = rank_of(&self.series, |x| x.before, &series.id);
                        let r1 = rank_of(&self.series, |x| x.after, &series.id);
                        (series, r0, r1)
                    })
                    .filter(|_| tip_opacity > 0.01),
                |stage, (series, r0, r1)| {
                    stage.child(
                        div()
                            .absolute()
                            .left(px(tip_left))
                            .top(px(tip_top))
                            .min_w(px(168.))
                            .max_w(px(240.))
                            .px(px(12.))
                            .py(px(10.))
                            .rounded(px(14.))
                            .bg(p.tray)
                            .border_1()
                            .border_color(p.line)
                            .shadow_lg()
                            .flex()
                            .flex_col()
                            .gap(px(3.))
                            .opacity(tip_opacity)
                            .child(
                                div()
                                    .mb(px(2.))
                                    .text_size(px(14.))
                                    .line_height(px(20.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .child(series.label.clone()),
                            )
                            .child(
                                div()
                                    .flex()
                                    .justify_between()
                                    .gap(px(12.))
                                    .text_size(px(14.))
                                    .line_height(px(18.))
                                    .child(
                                        div()
                                            .text_color(p.secondary)
                                            .child(self.start_label.clone()),
                                    )
                                    .child(
                                        div()
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.ink)
                                            .child(value_text(series.before, &prefix, &suffix)),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .justify_between()
                                    .gap(px(12.))
                                    .text_size(px(14.))
                                    .line_height(px(18.))
                                    .child(
                                        div().text_color(p.secondary).child(self.end_label.clone()),
                                    )
                                    .child(
                                        div()
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.ink)
                                            .child(value_text(series.after, &prefix, &suffix)),
                                    ),
                            )
                            .child(
                                div()
                                    .mt(px(2.))
                                    .text_size(px(12.))
                                    .line_height(px(20.))
                                    .text_color(p.faint)
                                    .child(format!(
                                        "{}, {}",
                                        change_text(series.after - series.before, &prefix, &suffix),
                                        rank_line(r0, r1),
                                    )),
                            ),
                    )
                },
            );
        let readings = div()
            .absolute()
            .size_1()
            .overflow_hidden()
            .child(div().id("slope-live").role(Role::Row).aria_label(announce))
            .child(
                div()
                    .id("slope-summary")
                    .role(Role::Row)
                    .aria_label(summary),
            )
            .child(div().id("slope-table").role(Role::Table).children(
                table_rows.into_iter().enumerate().map(|(i, row)| {
                    div()
                        .id(("slope-reading", i))
                        .role(Role::Row)
                        .aria_label(row)
                }),
            ));
        let narrow_card = width > 0. && width < 420.;
        column()
            .w(px(720.))
            .max_w_full()
            .gap(px(20.))
            .p(px(if narrow_card { 16. } else { 24. }))
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
            .child(
                div()
                    .id("slope-figure")
                    .role(Role::Figure)
                    .w_full()
                    .aria_label(self.label.clone())
                    .child(stage),
            )
            .child(readings)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        SlopeChart, SlopeSeries, axes, change_text, end_order, grouped1, nearest_series, rank_move,
        spread_labels, tip_anchor, value_text,
    };
    use gpui::{Bounds, TestAppContext, point, px};

    #[test]
    fn spread_forms_one_group_centred_on_its_members() {
        // Touching labels merge into one group: (50 + 30 + 11) / 3 = 30.3.
        let out = spread_labels(&[50., 50., 51.], 200.);
        assert!((out[0] - 30.33).abs() < 0.5, "{out:?}");
        assert!((out[1] - 50.33).abs() < 0.5, "{out:?}");
        assert!((out[2] - 70.33).abs() < 0.5, "{out:?}");
        // Far-apart labels stay where they want to be.
        assert_eq!(
            spread_labels(&[10., 100., 190.], 200.),
            vec![10., 100., 190.]
        );
        assert!(spread_labels(&[], 200.).is_empty());
    }
    #[test]
    fn grouped_values_drop_the_trailing_zero() {
        assert_eq!(grouped1(4.0), "4");
        assert_eq!(grouped1(3.1), "3.1");
        assert_eq!(grouped1(0.05), "0.1");
        assert_eq!(value_text(3.1, "", "%"), "3.1%");
        assert_eq!(change_text(1.5, "", "%"), "+1.5%");
        assert_eq!(change_text(-0.6, "", "%"), "\u{2212}0.6%");
    }
    #[test]
    fn end_order_and_rank_moves_read_largest_first() {
        let series = vec![
            SlopeSeries::new("email", "Email", 3.1, 4.6),
            SlopeSeries::new("search", "Search", 3.4, 3.6),
            SlopeSeries::new("social", "Social", 1.9, 1.3),
        ];
        assert_eq!(end_order(&series), vec![0, 1, 2]);
        assert_eq!(rank_move(&series, &"email".into()), 1);
        assert_eq!(rank_move(&series, &"social".into()), 0);
        // A dataset where social overtakes search by end value.
        let flipped = vec![
            SlopeSeries::new("email", "Email", 3.1, 3.2),
            SlopeSeries::new("search", "Search", 3.4, 1.0),
            SlopeSeries::new("social", "Social", 1.9, 4.0),
        ];
        assert_eq!(end_order(&flipped), vec![2, 0, 1]);
        assert_eq!(rank_move(&flipped, &"social".into()), 2);
    }
    #[test]
    fn nearest_reads_labels_beside_their_column() {
        let bounds = Bounds {
            origin: point(px(0.), px(0.)),
            size: gpui::size(px(720.), px(200.)),
        };
        let rows = vec![(50., 40.), (150., 160.)];
        let l0 = vec![50., 150.];
        let l1 = vec![40., 160.];
        let (x0, x1) = (168., 624.);
        // Inside the plot the line nearest the pointer wins.
        assert_eq!(
            nearest_series(bounds, point(px(396.), px(45.)), &rows, &l0, &l1, x0, x1),
            Some(0)
        );
        // Left of the first axis reads the spread start label.
        assert_eq!(
            nearest_series(bounds, point(px(20.), px(58.)), &rows, &l0, &l1, x0, x1),
            Some(0)
        );
        // Right of the last axis reads the spread end label.
        assert_eq!(
            nearest_series(bounds, point(px(700.), px(155.)), &rows, &l0, &l1, x0, x1),
            Some(1)
        );
        // Far away reads nothing.
        assert_eq!(
            nearest_series(bounds, point(px(396.), px(200.)), &rows, &l0, &l1, x0, x1),
            None
        );
    }
    #[test]
    fn tooltip_stays_above_when_it_fits_and_below_when_it_does_not() {
        let (left, top) = tip_anchor(150., 200., 720., 168., 624.);
        assert!((left - (396. - 90.)).abs() < 0.5, "{left}");
        assert!((top - (28. + 150. - 104. - 16.)).abs() < 0.5, "{top}");
        let (_, low) = tip_anchor(10., 200., 720., 168., 624.);
        assert!((low - (28. + 10. + 16.)).abs() < 0.5, "{low}");
    }
    #[test]
    fn axes_keep_the_plot_between_the_label_rooms() {
        // Wide card: 168 in from the left, 96 in from the right of 670.
        assert_eq!(axes(670., true), (168., 574.));
        assert_eq!(axes(670., false), (168., 606.));
        // Narrow stages shrink the rooms instead of crossing the axes.
        assert_eq!(axes(380., true), (132., 304.));
        assert_eq!(axes(150., true), (116., 156.));
    }
    #[test]
    fn plot_canvas_spans_the_full_card_width() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let series = vec![
            SlopeSeries::new("email", "Email", 3.1, 4.6),
            SlopeSeries::new("search", "Search", 3.4, 3.6),
            SlopeSeries::new("social", "Social", 1.9, 1.3),
            SlopeSeries::new("referral", "Referral", 2.2, 3.9),
        ];
        let (view, w) = app.add_window_view(|_, _| {
            SlopeChart::new("Conversion", series)
                .with_start_label("Q1")
                .with_end_label("Q2")
                .with_highlight("email")
        });
        w.run_until_parked();
        w.update(|_, cx| {
            let chart = view.read(cx);
            let b = chart.bounds.get();
            assert!(
                (b.size.width - px(670.)).abs() < px(2.),
                "plot canvas is {:?} wide, want 670",
                b.size.width
            );
        });
    }
    #[test]
    fn switching_datasets_slides_values_without_jumping() {
        use std::time::Duration;
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) = app.add_window_view(|_, _| {
            SlopeChart::new(
                "Conversion",
                vec![SlopeSeries::new("email", "Email", 3.1, 4.6)],
            )
        });
        w.run_until_parked();
        let samples = w.update(|_, cx| {
            let t0 = cx.background_executor().now();
            view.update(cx, |chart, cx| {
                chart.set_series(vec![SlopeSeries::new("email", "Email", 3.6, 5.2)], cx);
                (0..=5)
                    .map(|step| {
                        let at = t0 + Duration::from_millis(step * 120);
                        let (before, _) = chart.visual[0].before.sample_batch(at, false);
                        let (after, _) = chart.visual[0].after.sample_batch(at, false);
                        (before, after)
                    })
                    .collect::<Vec<_>>()
            })
        });
        // The first sample after the switch reads the old endpoints: motion
        // starts where the line was drawn, never from a reset position.
        assert!(
            (samples[0].0 - 3.1).abs() < 0.05 && (samples[0].1 - 4.6).abs() < 0.05,
            "first sample {samples:?}"
        );
        for (a, b) in samples.iter().zip(samples.iter().skip(1)) {
            assert!(
                b.0 >= a.0 - 0.02 && b.1 >= a.1 - 0.02,
                "monotone {samples:?}"
            );
        }
        assert!(
            (samples[5].0 - 3.6).abs() < 0.05 && (samples[5].1 - 5.2).abs() < 0.05,
            "settles at the new values {samples:?}"
        );
        let mid = samples[2].0;
        assert!(
            (3.15..3.55).contains(&mid),
            "mid-frame inside the slide: {mid}"
        );
    }
    #[test]
    fn switching_datasets_keeps_focus_when_the_key_survives() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) = app.add_window_view(|_, _| {
            SlopeChart::new(
                "Conversion",
                vec![SlopeSeries::new("email", "Email", 3.1, 4.6)],
            )
        });
        w.run_until_parked();
        w.update(|_, cx| {
            view.update(cx, |chart, cx| {
                chart.inspect(Some(0), cx);
                chart.set_series(vec![SlopeSeries::new("email", "Email", 3.6, 5.2)], cx);
            });
        });
        w.run_until_parked();
        w.update(|_, cx| {
            let chart = view.read(cx);
            assert_eq!(chart.active, Some(0), "a surviving key keeps its focus");
            assert_eq!(chart.visual[0].id, "email");
        });
    }
}
