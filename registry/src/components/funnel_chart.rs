//! Conversion funnel with segment filtering and keyboard inspection.
use crate::foundation::ThemeMode;
use crate::primitives::{
    family::{Paint, Slide, Tone, disabled_motion, face},
    presentation::{column, row, title, tr},
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

#[derive(Clone, Debug)]
pub struct FunnelSegment {
    pub id: SharedString,
    pub label: SharedString,
}
impl FunnelSegment {
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
        }
    }
}
/// Values correspond to segment order; invalid and negative numbers count as zero.
#[derive(Clone, Debug)]
pub struct FunnelStage {
    pub id: SharedString,
    pub label: SharedString,
    pub values: Vec<f32>,
}
impl FunnelStage {
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        values: Vec<f32>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            values,
        }
    }
    fn value(&self, segment: Option<usize>) -> f32 {
        self.value_f64(segment).min(f64::from(f32::MAX)) as f32
    }
    fn value_f64(&self, segment: Option<usize>) -> f64 {
        match segment {
            Some(i) => self
                .values
                .get(i)
                .copied()
                .filter(|v| v.is_finite())
                .unwrap_or(0.)
                .max(0.) as f64,
            None => self
                .values
                .iter()
                .copied()
                .filter(|v| v.is_finite())
                .map(|v| f64::from(v.max(0.)))
                .sum::<f64>(),
        }
    }
}
#[derive(Clone, Debug)]
pub struct FunnelInspect {
    pub id: SharedString,
    pub value: f32,
    pub from_start: f32,
    pub from_previous: f32,
}
/// Which slice of the data the band draws: one stacked band, every segment
/// with a 2px gap, or one segment rescaled to fill the height.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FunnelView {
    Total,
    Split,
    Segment(usize),
}
struct MovingStage {
    id: SharedString,
    /// Height share of the tallest step, 0..1.
    height: Slide,
    /// Cumulative segment share after each inner boundary, 0..1.
    cums: Vec<Slide>,
    /// Heading count that rolls to its new value.
    roll: Slide,
}
/// Series tones for stacked segments, matching the wider chart palette.
const SERIES: [u32; 4] = [0x55adff, 0x9c37be, 0xe66e00, 0x00a861];

pub struct FunnelChart {
    title: SharedString,
    period: Option<SharedString>,
    unit: Option<SharedString>,
    stages: Vec<FunnelStage>,
    segments: Vec<FunnelSegment>,
    view: FunnelView,
    flow: bool,
    gap: Slide,
    reveal: Slide,
    revealed: bool,
    glow: Slide,
    column: Slide,
    tip_x: Slide,
    tip_y: Slide,
    tip_show: Slide,
    split_thumb: Slide,
    flow_thumb: Slide,
    segment_thumb: Slide,
    thumb_synced: bool,
    phase: f32,
    last_tick: Option<Instant>,
    visible: bool,
    active: Option<usize>,
    visual: Vec<MovingStage>,
    bounds: Rc<Cell<Bounds<Pixels>>>,
    initialized: bool,
    focus: FocusHandle,
    focus_subscriptions: Vec<Subscription>,
}
impl EventEmitter<FunnelInspect> for FunnelChart {}
impl FunnelChart {
    pub fn new(
        title: impl Into<SharedString>,
        stages: Vec<FunnelStage>,
        segments: Vec<FunnelSegment>,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut chart = Self {
            title: title.into(),
            period: None,
            unit: None,
            stages,
            segments,
            view: FunnelView::Total,
            flow: true,
            gap: Slide::new(0.),
            reveal: Slide::new(0.),
            revealed: false,
            glow: Slide::new(0.),
            column: Slide::new(0.),
            tip_x: Slide::new(0.),
            tip_y: Slide::new(0.),
            tip_show: Slide::new(0.),
            split_thumb: Slide::new(0.),
            flow_thumb: Slide::new(1.),
            segment_thumb: Slide::new(3.),
            thumb_synced: false,
            phase: 0.,
            last_tick: None,
            visible: true,
            active: None,
            visual: vec![],
            bounds: Default::default(),
            initialized: false,
            focus: cx.focus_handle().tab_stop(true),
            focus_subscriptions: vec![],
        };
        chart.reconcile(cx);
        chart
    }
    /// Time range named under the heading, such as "Last 30 days".
    pub fn with_period(mut self, period: impl Into<SharedString>) -> Self {
        self.period = Some(period.into());
        self
    }
    /// Noun after counts in the tooltip and the hidden table.
    pub fn with_unit(mut self, unit: impl Into<SharedString>) -> Self {
        self.unit = Some(unit.into());
        self
    }
    /// Configure whether flow particles are drawn.
    pub fn with_flow(mut self, flow: bool) -> Self {
        self.flow = flow;
        self
    }
    /// Choose the drawn view before the first paint.
    pub fn with_view(mut self, view: FunnelView) -> Self {
        self.view = view;
        self
    }
    /// Pause or resume particles without resetting their phase.
    pub fn set_flow(&mut self, flow: bool, cx: &mut Context<Self>) {
        if self.flow != flow {
            self.flow = flow;
            self.flow_thumb
                .duration(if flow { 1. } else { 0. }, 0.24, cx);
            self.last_tick = None;
            cx.notify();
        }
    }
    /// Pause flow particles and paint scheduling while the host is hidden.
    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        if self.visible != visible {
            self.visible = visible;
            self.last_tick = None;
            cx.notify();
        }
    }
    pub fn set_stages(&mut self, stages: Vec<FunnelStage>, cx: &mut Context<Self>) {
        self.stages = stages;
        self.active = None;
        self.reconcile(cx);
        cx.notify();
    }
    /// Focus inspection on one step, or clear it.
    pub fn set_active(&mut self, index: Option<usize>, cx: &mut Context<Self>) {
        self.select(index, cx);
    }
    /// Swap the drawn view; every change morphs the band with a spring.
    pub fn set_view(&mut self, view: FunnelView, cx: &mut Context<Self>) {
        if self.view == view {
            return;
        }
        self.view = view;
        self.split_thumb
            .duration(if view == FunnelView::Split { 1. } else { 0. }, 0.24, cx);
        self.reconcile(cx);
        cx.notify();
    }
    fn segment(&mut self, segment: Option<usize>, cx: &mut Context<Self>) {
        let next = match segment {
            Some(i) => FunnelView::Segment(i),
            None => FunnelView::Total,
        };
        self.set_view(next, cx);
    }
    fn split(&mut self, cx: &mut Context<Self>) {
        if self.segments.len() < 2 {
            return;
        }
        let on = self.view != FunnelView::Split;
        self.set_view(
            if on {
                FunnelView::Split
            } else {
                FunnelView::Total
            },
            cx,
        );
    }
    /// Recompute per-step animation targets for the current view.
    fn reconcile(&mut self, cx: &App) {
        let selected = match self.view {
            FunnelView::Segment(i) => Some(i),
            _ => None,
        };
        let scale = self
            .stages
            .iter()
            .map(|stage| stage.value_f64(selected))
            .fold(f64::EPSILON, f64::max);
        let count = self.segments.len().max(1);
        let mut old: Vec<Option<MovingStage>> = std::mem::take(&mut self.visual)
            .into_iter()
            .map(Some)
            .collect();
        self.visual = self
            .stages
            .iter()
            .map(|stage| {
                let total = stage.value_f64(None);
                let mut cum = 0.;
                let targets: Vec<f64> = (0..count)
                    .map(|k| {
                        let share = if self.segments.is_empty() || selected.is_some() {
                            if selected == Some(k) { 1. } else { 0. }
                        } else if total > 0. {
                            stage.value_f64(Some(k)) / total
                        } else {
                            0.
                        };
                        cum += share;
                        cum.min(1.)
                    })
                    .collect();
                let height = stage.value_f64(selected) / scale;
                let inner: Vec<f64> = targets[..count.saturating_sub(1)].to_vec();
                let mut v = old
                    .iter()
                    .position(|v| v.as_ref().is_some_and(|v| v.id == stage.id))
                    .and_then(|i| old[i].take())
                    .unwrap_or_else(|| MovingStage {
                        id: stage.id.clone(),
                        height: Slide::new(height as f32),
                        // Only inner boundaries animate; the outer edge is
                        // always the full thickness.
                        cums: inner.iter().map(|t| Slide::new(*t as f32)).collect(),
                        roll: Slide::new(total as f32),
                    });
                v.height.duration(height as f32, 0.42, cx);
                for (slide, target) in v.cums.iter_mut().zip(inner.iter()) {
                    slide.duration(*target as f32, 0.42, cx);
                }
                v.roll.duration(total as f32, 0.4, cx);
                v
            })
            .collect();
        self.initialized = true;
        self.gap.duration(
            if self.view == FunnelView::Split {
                1.
            } else {
                0.
            },
            0.42,
            cx,
        );
    }
    fn select(&mut self, next: Option<usize>, cx: &mut Context<Self>) {
        if self.active == next {
            return;
        }
        self.active = next;
        self.glow
            .duration(if next.is_some() { 1. } else { 0. }, 0.16, cx);
        self.tip_show
            .duration(if next.is_some() { 1. } else { 0. }, 0.16, cx);
        if let Some((index, stage)) = next.and_then(|i| self.stages.get(i).map(|s| (i, s))) {
            let value = stage.value(self.selected());
            let first = self.stages.first().map_or(0., |s| s.value(self.selected()));
            let previous = if index == 0 {
                first
            } else {
                self.stages[index - 1].value(self.selected())
            };
            cx.emit(FunnelInspect {
                id: stage.id.clone(),
                value,
                from_start: if first > 0. { value / first } else { 0. },
                from_previous: if previous > 0. { value / previous } else { 0. },
            });
        }
        cx.notify();
    }
    fn selected(&self) -> Option<usize> {
        match self.view {
            FunnelView::Segment(i) => Some(i),
            _ => None,
        }
    }
    fn view_value(&self, stage: &FunnelStage) -> f64 {
        match self.view {
            FunnelView::Segment(i) => stage.value_f64(Some(i)),
            _ => stage.value_f64(None),
        }
    }
}

/// Compact heading format: 100K, 61.2K, 3.6K, 2.5M.
fn compact(v: f64) -> String {
    if !v.is_finite() || v < 0. {
        return "0".into();
    }
    let units = [(1e9, "B"), (1e6, "M"), (1e3, "K")];
    for (scale, suffix) in units {
        if v >= scale {
            let scaled = v / scale;
            return if scaled >= 100. {
                format!("{scaled:.0}{suffix}")
            } else {
                format!("{scaled:.1}{suffix}")
            };
        }
    }
    format!("{v:.0}")
}
/// Percent with one decimal under 10%: 61%, 8.4%, 2.3%.
fn rate_text(v: f64) -> String {
    let p = (v * 100.).clamp(0., 1000.);
    if p >= 9.95 {
        format!("{p:.0}%")
    } else {
        format!("{p:.1}%")
    }
}
/// Full tooltip and table format with thousands separators.
fn thousands(v: f64) -> String {
    let n = v.max(0.).round() as u64;
    let s = n.to_string();
    let len = s.len();
    let mut out = String::with_capacity(len + len / 3);
    for (i, c) in s.chars().enumerate() {
        out.push(c);
        let remaining = len - 1 - i;
        if remaining > 0 && remaining.is_multiple_of(3) {
            out.push(',');
        }
    }
    out
}
/// Stage values hold flat through the first and last part of each slot and
/// ease between steps; the join runs from 0.275 to 0.725 of the slot.
fn funnel_sample(values: &[f32], x: f32, length: f32) -> f32 {
    let count = values.len();
    if count == 0 || length <= 0. {
        return 0.;
    }
    let n = x / (length / count as f32) - 0.5;
    if n <= 0. {
        return values[0];
    }
    let last = count - 1;
    if n >= last as f32 {
        return values[last];
    }
    let index = n.floor() as usize;
    let fraction = n - index as f32;
    let t = if fraction <= 0.275 {
        0.
    } else if fraction >= 0.725 {
        1.
    } else {
        let r = (fraction - 0.275) / 0.45;
        r * r * r * (r * (r * 6. - 15.) + 10.)
    };
    values[index] + (values[index + 1] - values[index]) * t
}
fn intersects_viewport(bounds: Bounds<Pixels>, viewport: Size<Pixels>) -> bool {
    bounds.size.width > px(0.)
        && bounds.size.height > px(0.)
        && bounds.left() < viewport.width
        && bounds.top() < viewport.height
        && bounds.right() > px(0.)
        && bounds.bottom() > px(0.)
}
fn series_tone(p: &Paint, index: usize) -> Hsla {
    match SERIES.get(index) {
        Some(v) => rgb(*v).into(),
        None => p.panel.blend(p.ink.opacity(0.45)),
    }
}
/// One paint pass over every segment layer, optionally clipped to a column.
#[allow(clippy::too_many_arguments)]
fn paint_band(
    w: &mut Window,
    p: &Paint,
    series: &[Hsla],
    layers: usize,
    n: usize,
    gap: f32,
    reveal: f32,
    heights: &[f32],
    cums: &[Vec<f32>],
    half: f32,
    main: f32,
    steps: usize,
    at: &dyn Fn(f32, f32) -> Point<Pixels>,
    pure: bool,
    alpha: f32,
    mix: f32,
    clip: Option<(f32, f32)>,
) {
    let edges = |i: usize| -> Vec<f32> {
        let thickness = half * heights[i];
        let mut out = Vec::with_capacity(layers + 1);
        out.push(-thickness);
        for c in &cums[i] {
            out.push(-thickness + 2. * thickness * c);
        }
        out.push(thickness);
        out
    };
    for (k, tone) in series.iter().enumerate() {
        let color = if pure {
            *tone
        } else {
            p.panel.blend(tone.opacity(mix))
        };
        let shift = if layers > 1 {
            (k as f32 - (layers - 1) as f32 / 2.) * 2. * gap
        } else {
            0.
        };
        let top: Vec<f32> = (0..n).map(|i| edges(i)[k] + shift).collect();
        let bottom: Vec<f32> = (0..n).map(|i| edges(i)[k + 1] + shift).collect();
        let mut band = PathBuilder::fill();
        let mut open: Option<usize> = None;
        for s in 0..=steps {
            let u = s as f32 / steps as f32;
            let x = u * main;
            let inside = clip.is_none_or(|(a, z)| x >= a && x <= z);
            if inside {
                let y = funnel_sample(&top, x, main) * reveal;
                if open.is_some() {
                    band.line_to(at(u, y));
                } else {
                    band.move_to(at(u, y));
                    open = Some(s);
                }
            } else if let Some(start) = open.take() {
                for r in (start..s).rev() {
                    let ru = r as f32 / steps as f32;
                    band.line_to(at(ru, funnel_sample(&bottom, ru * main, main) * reveal));
                }
                band.close();
            }
        }
        if let Some(start) = open.take() {
            for r in (start..=steps).rev() {
                let ru = r as f32 / steps as f32;
                band.line_to(at(ru, funnel_sample(&bottom, ru * main, main) * reveal));
            }
            band.close();
        }
        if let Ok(path) = band.build() {
            w.paint_path(path, color.opacity(alpha));
        }
    }
}
/// Drop-off arrow glyph and percent under (or beside) each boundary.
fn drops_paint(
    b: Bounds<Pixels>,
    w: &mut Window,
    cx: &mut App,
    p: &Paint,
    dropped: &[f32],
    vertical: bool,
) {
    let width = f32::from(b.size.width);
    let height = f32::from(b.size.height);
    let count = dropped.len();
    for (k, d) in dropped.iter().enumerate() {
        let arrow = |w: &mut Window, ox: Pixels, oy: Pixels, x: f32, y: f32| {
            let at = |dx: f32, dy: f32| point(ox + px(x + dx), oy + px(y + dy));
            let mut line = PathBuilder::stroke(px(1.3));
            line.move_to(at(4., 1.));
            line.line_to(at(4., 7.));
            line.move_to(at(1.5, 4.5));
            line.line_to(at(4., 7.));
            line.line_to(at(6.5, 4.5));
            if let Ok(path) = line.build() {
                w.paint_path(path, p.faint);
            }
        };
        let text = format!("{}%", (d * 100.).round() as u32);
        let mut run = w.text_style().to_run(text.len());
        run.color = p.faint;
        let shaped =
            w.text_system()
                .shape_line(text.into(), px(12.), std::slice::from_ref(&run), None);
        if vertical {
            let y = height * (k + 1) as f32 / (count + 1) as f32;
            arrow(w, b.origin.x, b.origin.y, 0., y - 4.);
            let _ = shaped.paint(
                point(b.origin.x + px(12.), b.origin.y + px(y + 1.)),
                px(12.),
                TextAlign::Left,
                None,
                w,
                cx,
            );
        } else {
            let x = width * (k + 1) as f32 / (count + 1) as f32;
            let total = 11. + f32::from(shaped.width()) + 2.;
            arrow(w, b.origin.x, b.origin.y, x - total / 2., 9.);
            let _ = shaped.paint(
                point(b.origin.x + px(x - total / 2. + 11.), b.origin.y + px(15.)),
                px(12.),
                TextAlign::Left,
                None,
                w,
                cx,
            );
        }
    }
}

/// All animation samples for one paint, plus follower spring retargets.
struct FunnelFrame {
    heights: Vec<f32>,
    cums: Vec<Vec<f32>>,
    rolls: Vec<f32>,
    series: Vec<Hsla>,
    gap: f32,
    reveal: f32,
    glow: f32,
    column: f32,
    tip_x: f32,
    tip_y: f32,
    tip_show: f32,
    split_thumb: f32,
    flow_thumb: f32,
    segment_thumb: f32,
    moving: bool,
    flowing: bool,
    phase: f32,
    active: Option<usize>,
    narrow: bool,
    measured_width: f32,
}
impl FunnelChart {
    /// Sample every spring in one pass and retarget the follower springs
    /// (focus column, tooltip, selection block) toward the inspected step.
    fn sample_frame(&mut self, w: &mut Window, cx: &mut Context<Self>) -> FunnelFrame {
        if !self.thumb_synced {
            self.thumb_synced = true;
            self.flow_thumb.set(if self.flow { 1. } else { 0. }, cx);
            self.split_thumb.set(
                if self.view == FunnelView::Split {
                    1.
                } else {
                    0.
                },
                cx,
            );
        }
        if self.focus_subscriptions.is_empty() {
            let focus = self.focus.clone();
            self.focus_subscriptions
                .push(cx.on_focus(&focus, w, |s, _, cx| {
                    if s.active.is_none() && !s.stages.is_empty() {
                        s.select(Some(0), cx);
                    }
                }));
        }
        let p = Paint::get(cx);
        let current_bounds = self.bounds.get();
        let measured_width = f32::from(current_bounds.size.width);
        let narrow = measured_width > 0. && measured_width < 420.;
        let visible = self.visible
            && w.is_visible()
            && intersects_viewport(current_bounds, w.viewport_size());
        if !self.revealed && visible {
            self.revealed = true;
            self.reveal.duration(1., 0.9, cx);
        }
        let now = cx.background_executor().now();
        let reduced = disabled_motion(cx);
        let moving = Cell::new(false);
        let sample = |slide: &Slide| {
            let (value, active) = slide.sample_batch(now, reduced);
            moving.set(moving.get() | active);
            value
        };
        let heights: Vec<f32> = self
            .visual
            .iter()
            .map(|stage| sample(&stage.height).clamp(0., 1.2))
            .collect();
        let cums: Vec<Vec<f32>> = self
            .visual
            .iter()
            .map(|stage| {
                stage
                    .cums
                    .iter()
                    .map(|slide| sample(slide).clamp(0., 1.))
                    .collect()
            })
            .collect();
        let rolls: Vec<f32> = self
            .visual
            .iter()
            .map(|s| sample(&s.roll).max(0.))
            .collect();
        let layers = self.segments.len().max(1);
        // The reference paints every layer with series-1 in the total view;
        // only split and isolated views tint layer 2+ with their own series.
        let series: Vec<Hsla> = (0..layers)
            .map(|k| {
                if self.view == FunnelView::Total {
                    series_tone(&p, 0)
                } else {
                    series_tone(&p, k)
                }
            })
            .collect();
        let gap = sample(&self.gap).clamp(0., 1.);
        let reveal = sample(&self.reveal).clamp(0., 1.);
        let glow = sample(&self.glow).clamp(0., 1.);
        let column = sample(&self.column);
        let tip_x = sample(&self.tip_x);
        let tip_y = sample(&self.tip_y);
        let tip_show = sample(&self.tip_show).clamp(0., 1.);
        let split_thumb = sample(&self.split_thumb);
        let flow_thumb = sample(&self.flow_thumb);
        let segment_thumb = sample(&self.segment_thumb);
        let flowing = self.flow && !reduced && visible && reveal > 0.75;
        if flowing {
            if let Some(last) = self.last_tick {
                self.phase = (self.phase
                    + now.saturating_duration_since(last).as_secs_f32().min(0.1) / 4.)
                    .fract();
            }
            self.last_tick = Some(now);
        } else {
            self.last_tick = None;
        }
        let phase = self.phase;
        if visible && (moving.get() || flowing) {
            w.request_animation_frame();
        }
        let n = self.stages.len();
        let active = self.active.filter(|i| *i < n);
        if let Some(i) = active {
            let plot_w = if narrow {
                measured_width * 0.7
            } else {
                measured_width
            };
            let col_w = plot_w / n.max(1) as f32;
            self.column.duration(i as f32 * col_w, 0.26, cx);
            if narrow {
                self.tip_x.duration(8., 0.26, cx);
                self.tip_y
                    .duration((i as f32 * col_w + 16.).min(84. * n as f32 - 96.), 0.26, cx);
            } else {
                self.tip_x
                    .duration((i as f32 * col_w + 8.).min(plot_w - 184.), 0.26, cx);
                let band_bottom = 100. + 92. * heights.get(i).copied().unwrap_or(0.);
                self.tip_y
                    .duration(band_bottom.clamp(96., 108.) + 8., 0.26, cx);
            }
        }
        FunnelFrame {
            heights,
            cums,
            rolls,
            series,
            gap,
            reveal,
            glow,
            column,
            tip_x,
            tip_y,
            tip_show,
            split_thumb,
            flow_thumb,
            segment_thumb,
            moving: moving.get(),
            flowing,
            phase,
            active,
            narrow,
            measured_width,
        }
    }
    fn build_graph(&mut self, f: &FunnelFrame, cx: &mut Context<Self>) -> Stateful<Div> {
        let p = Paint::get(cx);
        let n = self.stages.len();
        let layers = self.segments.len().max(1);
        let narrow = f.narrow;
        let aria = format!(
            "{}{}",
            self.title,
            self.period
                .clone()
                .map(|s| format!(", {s}"))
                .unwrap_or_default()
        );
        let instructions = tr(
            cx,
            "Use arrow keys to move between steps",
            "使用方向键在各步之间移动",
        );
        let aria = format!("{aria}. {instructions}.");
        let bounds = self.bounds.clone();
        let frame = FunnelFrame {
            heights: f.heights.clone(),
            cums: f.cums.clone(),
            rolls: Vec::new(),
            series: f.series.clone(),
            gap: f.gap,
            reveal: f.reveal,
            glow: f.glow,
            column: f.column,
            tip_x: f.tip_x,
            tip_y: f.tip_y,
            tip_show: f.tip_show,
            split_thumb: f.split_thumb,
            flow_thumb: f.flow_thumb,
            segment_thumb: f.segment_thumb,
            moving: f.moving,
            flowing: f.flowing,
            phase: f.phase,
            active: f.active,
            narrow: f.narrow,
            measured_width: f.measured_width,
        };
        let active_store = f.active;
        let view_values: Vec<f64> = self.stages.iter().map(|s| self.view_value(s)).collect();
        let first_total = self.stages.first().map_or(0., |s| s.value_f64(None));
        let empty = n == 0 || first_total <= 0.;
        let canvas_el = canvas(
            move |b, window, _| {
                let before = bounds.replace(b);
                if before.size != b.size
                    || intersects_viewport(before, window.viewport_size())
                        != intersects_viewport(b, window.viewport_size())
                {
                    window.request_animation_frame();
                }
            },
            move |b, _, w, _| {
                let f = &frame;
                let p = &p;
                if n < 2 {
                    return;
                }
                let width = f32::from(b.size.width);
                let height = f32::from(b.size.height);
                let vertical = width < 420.;
                let main = if vertical { height } else { width };
                let cross = if vertical {
                    (width * 0.44).clamp(120., 220.)
                } else {
                    height
                };
                let shift = if vertical { (width - cross) * 0.5 } else { 0. };
                let center = cross / 2.;
                let half = (cross - 16.) * 0.5;
                let at = move |t: f32, offset: f32| {
                    if vertical {
                        point(
                            b.origin.x + px(shift + center + offset),
                            b.origin.y + px(main * t),
                        )
                    } else {
                        point(b.origin.x + px(main * t), b.origin.y + px(center + offset))
                    }
                };
                let steps = ((main / 3.).ceil() as usize).max(12);
                let slot = main / n as f32;
                let mix = 0.24 + 0.18 * f.gap;
                paint_band(
                    w, p, &f.series, layers, n, f.gap, f.reveal, &f.heights, &f.cums, half, main,
                    steps, &at, false, 1., mix, None,
                );
                if f.glow > 0.01
                    && let Some(i) = active_store
                {
                    paint_band(
                        w,
                        p,
                        &f.series,
                        layers,
                        n,
                        f.gap,
                        f.reveal,
                        &f.heights,
                        &f.cums,
                        half,
                        main,
                        steps,
                        &at,
                        true,
                        f.glow,
                        mix,
                        Some((i as f32 * slot, (i + 1) as f32 * slot)),
                    );
                }
                if f.flowing {
                    let outer = |bound: usize| -> Vec<f32> {
                        (0..n)
                            .map(|i| {
                                let thickness = half * f.heights[i];
                                let edge = if bound == 0 { -thickness } else { thickness };
                                edge + if bound == 0 { -f.gap } else { f.gap }
                            })
                            .collect()
                    };
                    let top_all = outer(0);
                    let bottom_all = outer(1);
                    for dot in 0..40u32 {
                        let u = (f.phase + dot as f32 / 40.).fract();
                        let lane = 0.08 + ((dot.wrapping_mul(37) % 89) as f32) / 110.;
                        let kept = |stage: usize| -> f64 {
                            let before = view_values[stage - 1];
                            let now = view_values[stage];
                            if before > 1e-6 {
                                (now / before).clamp(0., 1.)
                            } else {
                                0.
                            }
                        };
                        let cutoff = (1..n).find_map(|stage| {
                            let draw =
                                ((dot.wrapping_mul(37) + stage as u32 * 71) % 97) as f32 / 97.;
                            (draw > kept(stage) as f32)
                                .then_some((stage as f32 - 0.5) / (n - 1) as f32)
                        });
                        let exit = cutoff.map_or(0., |cut| ((u - cut) / 0.12).clamp(0., 1.));
                        if exit >= 1. {
                            continue;
                        }
                        let travel = cutoff.map_or(u, |cut| u.min(cut));
                        let x = travel * main;
                        let top_v = funnel_sample(&top_all, x, main);
                        let bottom_v = funnel_sample(&bottom_all, x, main);
                        let depth = if exit > 0. {
                            bottom_v + exit * 20.
                        } else {
                            top_v + (bottom_v - top_v) * lane
                        };
                        let center_pt = at(travel, depth * f.reveal);
                        w.paint_quad(
                            fill(
                                Bounds {
                                    origin: point(center_pt.x - px(1.5), center_pt.y - px(1.5)),
                                    size: size(px(3.), px(3.)),
                                },
                                p.ink.opacity(0.55 * (1. - exit)),
                            )
                            .corner_radii(px(1.5)),
                        );
                    }
                }
            },
        )
        .size_full();
        let side_rows: Vec<Div> = (0..n)
            .map(|i| {
                let stage = &self.stages[i];
                div()
                    .absolute()
                    .top(relative(i as f32 / n.max(1) as f32))
                    .left(relative(0.72))
                    .right_0()
                    .h(px(84.))
                    .flex()
                    .flex_col()
                    .justify_center()
                    .gap(px(1.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .line_height(px(15.6))
                            .text_color(p.secondary)
                            .child(stage.label.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(18.))
                            .line_height(px(23.4))
                            .text_color(p.ink)
                            .child(compact(f64::from(f.rolls.get(i).copied().unwrap_or(0.)))),
                    )
            })
            .collect();
        let soft_column = div()
            .absolute()
            .rounded(px(14.))
            .bg(p.ink)
            .opacity(0.035 * f.glow)
            .when(narrow, |c| {
                c.left(relative(0.72))
                    .right_0()
                    .top(px(f.column))
                    .h(px(84.))
            })
            .when(!narrow, |c| {
                c.top_0()
                    .bottom_0()
                    .left(px(f.column))
                    .w(px(f.measured_width / n.max(1) as f32))
            });
        let tooltip = div()
            .absolute()
            .top(px(f.tip_y))
            .left(px(f.tip_x.max(8.)))
            .min_w(px(168.))
            .max_w(px(240.))
            .px(px(12.))
            .py(px(10.))
            .flex()
            .flex_col()
            .gap(px(4.))
            .rounded(px(14.))
            .border_1()
            .border_color(p.line)
            .bg(p.tray)
            .shadow_lg()
            .opacity(f.tip_show)
            .when_some(
                f.active
                    .and_then(|i| self.stages.get(i))
                    .map(|stage| self.tooltip_rows(stage, &p, cx)),
                |c, rows| c.children(rows),
            );
        div()
            .id("funnel-chart")
            .role(Role::Group)
            .tab_index(0)
            .track_focus(&self.focus)
            .w_full()
            .relative()
            .h(px(if narrow { 84. * n.max(1) as f32 } else { 200. }))
            .aria_label(aria)
            .on_hover(cx.listener(|s, h: &bool, w, cx| {
                // Keyboard focus suppresses pointer hover, so a keypress must
                // not clear the reading the reference keeps on screen.
                if !*h && !w.last_input_was_keyboard() {
                    s.select(None, cx);
                }
            }))
            .on_mouse_move(cx.listener(|s, e: &MouseMoveEvent, _, cx| {
                let b = s.bounds.get();
                let width = f32::from(b.size.width).max(1.);
                if s.stages.is_empty() {
                    return;
                }
                let vertical = width < 420.;
                let along = if vertical {
                    f32::from(e.position.y - b.origin.y) / f32::from(b.size.height).max(1.)
                } else {
                    f32::from(e.position.x - b.origin.x) / width
                };
                let index = ((along.clamp(0., 1.) * s.stages.len() as f32) as usize)
                    .min(s.stages.len() - 1);
                s.select(Some(index), cx);
            }))
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                let count = s.stages.len();
                if count == 0 {
                    return;
                }
                let next = match e.keystroke.key.as_str() {
                    "right" | "down" => Some(s.active.map_or(0, |i| (i + 1).min(count - 1))),
                    "left" | "up" => Some(s.active.map_or(0, |i| i.saturating_sub(1))),
                    "home" => Some(0),
                    "end" => Some(count - 1),
                    "escape" => None,
                    _ => return,
                };
                w.prevent_default();
                cx.stop_propagation();
                s.select(next, cx);
            }))
            .child(canvas_el)
            .when(narrow, |g| g.children(side_rows))
            .child(soft_column)
            .child(tooltip)
            .when(empty, |g| {
                g.child(
                    div()
                        .absolute()
                        .inset_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(13.))
                        .line_height(px(18.))
                        .text_color(p.faint)
                        .child(tr(cx, "No data", "暂无数据")),
                )
            })
    }
    fn tooltip_rows(&self, stage: &FunnelStage, p: &Paint, cx: &App) -> Vec<AnyElement> {
        let index = self.active.unwrap_or(0);
        let value = self.view_value(stage);
        let first = self.stages.first().map_or(0., |s| self.view_value(s));
        let previous = if index == 0 {
            first
        } else {
            self.view_value(&self.stages[index - 1])
        };
        let kept = if previous > 0. { value / previous } else { 0. };
        let unit = self.unit.as_deref().unwrap_or_default();
        let with_unit = |v: f64| {
            if unit.is_empty() {
                thousands(v)
            } else {
                format!("{} {}", thousands(v), unit)
            }
        };
        let mut rows: Vec<AnyElement> = vec![
            div()
                .mb(px(2.))
                .text_size(px(12.))
                .line_height(px(16.8))
                .text_color(p.secondary)
                .child(stage.label.clone())
                .into_any_element(),
        ];
        if self.view == FunnelView::Split {
            for (k, seg) in self.segments.iter().enumerate() {
                rows.push(
                    div()
                        .id(("funnel-tip-row", k))
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .text_size(px(14.))
                        .line_height(px(18.2))
                        .child(
                            div()
                                .w(px(10.))
                                .h(px(2.5))
                                .rounded(px(2.))
                                .bg(series_tone(p, k)),
                        )
                        .child(
                            div()
                                .flex_1()
                                .text_color(p.secondary)
                                .child(seg.label.clone()),
                        )
                        .child(
                            div()
                                .text_color(p.ink)
                                .font_weight(FontWeight::MEDIUM)
                                .child(thousands(f64::from(stage.value(Some(k))))),
                        )
                        .into_any_element(),
                );
            }
        } else {
            rows.push(
                div()
                    .id("funnel-tip-total")
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .text_size(px(14.))
                    .line_height(px(18.2))
                    .child(
                        div()
                            .flex_1()
                            .text_color(p.secondary)
                            .child(tr(cx, "Total", "全部")),
                    )
                    .child(
                        div()
                            .text_color(p.ink)
                            .font_weight(FontWeight::MEDIUM)
                            .child(with_unit(value)),
                    )
                    .into_any_element(),
            );
        }
        if index > 0 {
            rows.push(
                div()
                    .mt(px(2.))
                    .text_size(px(12.))
                    .line_height(px(16.8))
                    .text_color(p.faint)
                    .child(format!(
                        "{} {} {}",
                        rate_text(kept),
                        tr(cx, "kept from", "留存自"),
                        self.stages[index - 1].label.clone()
                    ))
                    .into_any_element(),
            );
        }
        rows
    }
    fn header_grid(&self, f: &FunnelFrame, p: &Paint, cx: &App) -> Option<Div> {
        let n = self.stages.len();
        (!f.narrow).then(|| {
            div()
                .grid()
                .grid_cols(n.max(1) as u16)
                .pb(px(12.))
                .children(self.stages.iter().enumerate().map(|(i, stage)| {
                    let on = f.active.is_none_or(|a| a == i);
                    let value = self.view_value(stage);
                    let first = self.stages.first().map_or(0., |s| self.view_value(s));
                    let rate = if i == 0 {
                        tr(cx, "Start", "起点").to_string()
                    } else if first > 0. {
                        format!(
                            "{} {}",
                            rate_text(value / first),
                            tr(cx, "of start", "占起始")
                        )
                    } else {
                        "0%".into()
                    };
                    column()
                        .gap(px(1.))
                        .pr(px(8.))
                        .justify_end()
                        .min_w_0()
                        .opacity(if on { 1. } else { 0.5 })
                        .child(
                            div()
                                .text_size(px(12.))
                                .line_height(px(15.6))
                                .text_color(p.secondary)
                                .max_h(px(31.2))
                                .overflow_hidden()
                                .child(stage.label.clone()),
                        )
                        .child(
                            div()
                                .text_size(px(18.))
                                .line_height(px(23.4))
                                .text_color(p.ink)
                                .font_weight(FontWeight::MEDIUM)
                                .whitespace_nowrap()
                                .child(compact(f64::from(f.rolls.get(i).copied().unwrap_or(0.)))),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .line_height(px(15.6))
                                .text_color(p.faint)
                                .whitespace_nowrap()
                                .child(rate),
                        )
                }))
        })
    }
    /// Drop-off share at every boundary for the current view.
    fn drops_values(&self) -> Vec<f32> {
        let selected = self.selected();
        (1..self.stages.len())
            .map(|k| {
                let prev = self.stages[k - 1].value_f64(selected);
                let now = self.stages[k].value_f64(selected);
                if prev > 0. {
                    (1. - now / prev) as f32
                } else {
                    0.
                }
            })
            .collect()
    }
    fn readings_element(&self, p: &Paint, cx: &App) -> Stateful<Div> {
        let unit = self.unit.as_deref().unwrap_or_default();
        let with_unit = |v: f64| {
            if unit.is_empty() {
                thousands(v)
            } else {
                format!("{} {}", thousands(v), unit)
            }
        };
        let first_total = self.stages.first().map_or(0., |s| s.value_f64(None));
        let view_first = self.stages.first().map_or(0., |s| self.view_value(s));
        let view_last = self.stages.last().map_or(0., |s| self.view_value(s));
        let converts = if view_first > 0. {
            view_last / view_first
        } else {
            0.
        };
        let empty = self.stages.is_empty() || first_total <= 0.;
        let summary = if empty {
            String::new()
        } else {
            let steps = self
                .stages
                .iter()
                .map(|s| format!("{} {}", s.label, with_unit(self.view_value(s))))
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "{}{}. {} {} {}",
                self.title,
                self.period
                    .clone()
                    .map(|s| format!(", {s}"))
                    .unwrap_or_default(),
                steps,
                rate_text(converts),
                tr(cx, "reach the last step", "到达最后一步")
            )
        };
        let live_text = self
            .active
            .and_then(|i| self.stages.get(i))
            .map(|stage| {
                let index = self.active.unwrap_or(0);
                let value = self.view_value(stage);
                let first = self.stages.first().map_or(0., |s| self.view_value(s));
                let previous = if index == 0 {
                    first
                } else {
                    self.view_value(&self.stages[index - 1])
                };
                format!(
                    "{}, {}{}, {} {}, {} {}",
                    stage.label,
                    thousands(value),
                    if unit.is_empty() {
                        String::new()
                    } else {
                        format!(" {unit}")
                    },
                    rate_text(if previous > 0. { value / previous } else { 0. }),
                    tr(cx, "of the previous step", "占上一步"),
                    rate_text(if first > 0. { value / first } else { 0. }),
                    tr(cx, "of the first", "占第一步"),
                )
            })
            .unwrap_or_default();
        let table_rows: Vec<String> = self
            .stages
            .iter()
            .enumerate()
            .map(|(i, stage)| {
                let total = stage.value_f64(None);
                let previous = if i == 0 {
                    total
                } else {
                    self.stages[i - 1].value_f64(None)
                };
                let segs = self
                    .segments
                    .iter()
                    .enumerate()
                    .map(|(k, seg)| {
                        format!("{} {}", seg.label, with_unit(stage.value_f64(Some(k))))
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(
                    "{}{}. {} {}. {} {} {}",
                    stage.label,
                    if segs.is_empty() {
                        String::new()
                    } else {
                        format!(", {segs}")
                    },
                    tr(cx, "shown", "显示"),
                    with_unit(total),
                    if previous > 0. {
                        rate_text(total / previous)
                    } else {
                        "0%".into()
                    },
                    if first_total > 0. {
                        rate_text(total / first_total)
                    } else {
                        "0%".into()
                    },
                    tr(cx, "of start", "占起始"),
                )
            })
            .collect();
        let _ = p;
        div()
            .id("funnel-readings")
            .absolute()
            .size(px(1.))
            .overflow_hidden()
            .role(Role::Table)
            .aria_label(self.title.clone())
            .child(
                div()
                    .id("funnel-live")
                    .role(Role::Row)
                    .aria_label(live_text),
            )
            .child(
                div()
                    .id("funnel-summary")
                    .role(Role::Row)
                    .aria_label(summary),
            )
            .children(table_rows.into_iter().enumerate().map(|(i, text)| {
                div()
                    .id(("funnel-reading", i))
                    .role(Role::Row)
                    .aria_label(text)
            }))
    }
    /// Device segmented control with a sliding selection block.
    fn device_control(&mut self, w: &mut Window, p: &Paint, cx: &mut Context<Self>) -> Div {
        let mut labels: Vec<SharedString> = vec![tr(cx, "All", "全部").into()];
        for seg in &self.segments {
            labels.push(seg.label.clone());
        }
        let text_run = w.text_style().to_run(8);
        let widths: Vec<f32> = labels
            .iter()
            .map(|label| {
                f32::from(
                    w.text_system()
                        .shape_line(
                            label.clone(),
                            px(13.),
                            std::slice::from_ref(&text_run),
                            None,
                        )
                        .width(),
                )
            })
            .collect();
        let selected_index = match self.view {
            FunnelView::Total | FunnelView::Split => 0,
            FunnelView::Segment(k) => k + 1,
        };
        let mut sel_left = 3.;
        for (i, width) in widths.iter().enumerate() {
            if i < selected_index {
                sel_left += width + 20. + 2.;
            }
        }
        self.segment_thumb.duration(sel_left, 0.26, cx);
        let sel_thumb = self.segment_thumb.value(cx);
        let mut track = row()
            .items_center()
            .relative()
            .p(px(3.))
            .gap(px(2.))
            .rounded(px(8.))
            .border_1()
            .border_color(p.line)
            .bg(p.hover)
            .child(
                div()
                    .absolute()
                    .top(px(3.))
                    .bottom(px(3.))
                    .left(px(sel_thumb))
                    .w(px(widths.get(selected_index).copied().unwrap_or(40.) + 20.))
                    .rounded(px(6.))
                    .border_1()
                    .border_color(p.line)
                    .bg(p.panel)
                    .shadow_sm(),
            );
        for (i, label) in labels.iter().enumerate() {
            let on = i == selected_index;
            track = track.child(
                face(("funnel-view", i), label.clone(), 28., 6., Tone::Clear, cx)
                    .px(px(10.))
                    .relative()
                    .text_size(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .map(|b| {
                        if on {
                            b.text_color(p.ink)
                        } else {
                            b.text_color(p.secondary)
                        }
                    })
                    .child(div().relative().child(label.clone()))
                    .on_click(cx.listener(move |s, _, _, cx| {
                        s.segment(if i == 0 { None } else { Some(i - 1) }, cx);
                    })),
            );
        }
        track
    }
    /// A capsule switch with the shared 42x24 track metrics.
    fn switch_control(
        &mut self,
        p: &Paint,
        cx: &mut Context<Self>,
        id: &'static str,
        label: SharedString,
        on: bool,
        thumb: f32,
    ) -> Stateful<Div> {
        let track = if on {
            p.hover.blend(p.bright.opacity(0.55))
        } else {
            p.hover
        };
        let is_split = id == "funnel-split";
        face(id, label.clone(), 32., 8., Tone::Clear, cx)
            .role(Role::Switch)
            .aria_toggled(on.into())
            .items_center()
            .gap(px(12.))
            .text_size(px(13.))
            .text_color(p.ink)
            .child(
                div()
                    .relative()
                    .flex_none()
                    .w(px(42.))
                    .h(px(24.))
                    .rounded_full()
                    .bg(track)
                    .child(
                        div()
                            .absolute()
                            .top(px(3.))
                            .left(px(3. + thumb * 18.))
                            .w(px(18.))
                            .h(px(18.))
                            .rounded_full()
                            .bg(Paint::for_mode(ThemeMode::Light).panel)
                            .shadow_sm(),
                    ),
            )
            .child(label)
            .on_click(cx.listener(move |s, _, _, cx| {
                if is_split {
                    s.split(cx);
                } else {
                    s.set_flow(!s.flow, cx);
                }
            }))
    }
}
impl Render for FunnelChart {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let f = self.sample_frame(w, cx);
        let n = self.stages.len();
        let empty = n == 0 || self.stages.first().is_none_or(|s| s.value_f64(None) <= 0.);
        let view_first = self.stages.first().map_or(0., |s| self.view_value(s));
        let view_last = self.stages.last().map_or(0., |s| self.view_value(s));
        let converts = if view_first > 0. {
            view_last / view_first
        } else {
            0.
        };
        let convert_meta = format!(
            "{}{} {}",
            self.period
                .clone()
                .map(|s| format!("{s}, "))
                .unwrap_or_default(),
            rate_text(converts),
            tr(cx, "convert", "转化")
        );
        let meta_text: SharedString = if empty {
            self.period.clone().unwrap_or_default()
        } else {
            convert_meta.into()
        };
        let header = self.header_grid(&f, &p, cx);
        let dropped = self.drops_values();
        let make_drops = |vertical: bool| {
            let dropped = dropped.clone();
            let paint_p = p;
            canvas(
                move |_, _, _| {},
                move |b, _, w, cx| drops_paint(b, w, cx, &paint_p, &dropped, vertical),
            )
        };
        let graph = self.build_graph(&f, cx);
        let readings = self.readings_element(&p, cx);
        let device_track = self.device_control(w, &p, cx);
        let split_switch = self.switch_control(
            &p,
            cx,
            "funnel-split",
            tr(cx, "Split", "拆分").into(),
            self.view == FunnelView::Split,
            f.split_thumb,
        );
        let flow_switch = self.switch_control(
            &p,
            cx,
            "funnel-flow",
            tr(cx, "Flow", "流动").into(),
            self.flow,
            f.flow_thumb,
        );
        let narrow = f.narrow;
        let switches_row = row()
            .items_center()
            .gap(px(16.))
            .when(self.segments.len() > 1, |r| r.child(split_switch))
            .child(flow_switch);
        let heading = column()
            .gap(px(2.))
            .child(title(self.title.clone(), p))
            .when(!meta_text.is_empty(), |c| {
                c.child(
                    div()
                        .text_size(px(13.))
                        .line_height(px(18.))
                        .text_color(p.faint)
                        .child(meta_text.clone()),
                )
            });
        let figure = column().when_some(header, |c, h| c.child(h)).child(graph);
        let figure = if narrow && !empty {
            figure.child(
                div()
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .right_0()
                    .w(relative(0.28))
                    .child(make_drops(true).w_full().h_full()),
            )
        } else if !empty {
            figure.child(make_drops(false).w_full().h(px(26.)))
        } else {
            figure
        };
        column()
            .w(px(720.))
            .max_w_full()
            .gap(px(24.))
            .p(px(if narrow { 16. } else { 24. }))
            .rounded(crate::foundation::surface_radius(cx))
            .bg(p.panel)
            .border_1()
            .border_color(p.line)
            .text_color(p.ink)
            .child(
                row()
                    .flex_wrap()
                    .justify_between()
                    .items_start()
                    .gap(px(12.))
                    .child(heading)
                    .child(
                        row()
                            .flex_wrap()
                            .items_center()
                            .gap(px(12.))
                            .gap_x(px(16.))
                            .child(device_track)
                            .child(switches_row),
                    ),
            )
            .child(figure)
            .child(readings)
    }
}

#[cfg(test)]
mod tests {
    use super::{FunnelStage, compact, funnel_sample, rate_text, thousands};
    #[test]
    fn finite_segments_are_summed_before_visual_normalization() {
        let stage = FunnelStage::new("large", "Large", vec![f32::MAX, f32::MAX]);
        let scale = stage.value_f64(None);
        assert!(scale.is_finite());
        assert!((stage.value_f64(Some(0)) / scale - 0.5).abs() < 1e-12);
        assert!((stage.value_f64(Some(1)) / scale - 0.5).abs() < 1e-12);
    }
    #[test]
    fn compact_counts_match_the_reference_format() {
        assert_eq!(compact(100_000.), "100K");
        assert_eq!(compact(61_170.), "61.2K");
        assert_eq!(compact(16_420.), "16.4K");
        assert_eq!(compact(3_590.), "3.6K");
        assert_eq!(compact(999.), "999");
        assert_eq!(compact(2_500_000.), "2.5M");
    }
    #[test]
    fn rates_use_one_decimal_under_ten_percent() {
        assert_eq!(rate_text(0.61), "61%");
        assert_eq!(rate_text(0.16), "16%");
        assert_eq!(rate_text(0.084), "8.4%");
        assert_eq!(rate_text(0.036), "3.6%");
        assert_eq!(rate_text(0.023), "2.3%");
    }
    #[test]
    fn thousands_separator_groups_every_three_digits() {
        assert_eq!(thousands(100_000.), "100,000");
        assert_eq!(thousands(61_170.), "61,170");
        assert_eq!(thousands(999.), "999");
        assert_eq!(thousands(1_540.), "1,540");
    }
    #[test]
    fn funnel_sample_holds_flat_then_eases_between_steps() {
        let values = [0., 1.];
        assert_eq!(funnel_sample(&values, 0., 2.), 0.);
        assert_eq!(funnel_sample(&values, 0.2, 2.), 0.);
        // Slot centers sit half a slot before the sample index, so x = 0.5
        // is still the flat lead-in and the join midpoint falls at x = 1.
        assert_eq!(funnel_sample(&values, 0.5, 2.), 0.);
        assert!((funnel_sample(&values, 1., 2.) - 0.5).abs() < 1e-3);
        assert_eq!(funnel_sample(&values, 1.4, 2.), 1.);
        assert_eq!(funnel_sample(&values, 2., 2.), 1.);
    }
    #[test]
    fn split_drop_off_matches_kept_share() {
        let stages = vec![
            FunnelStage::new("a", "A", vec![100., 100.]),
            FunnelStage::new("b", "B", vec![61., 61.]),
        ];
        let prev = stages[0].value_f64(None);
        let now = stages[1].value_f64(None);
        assert_eq!(((1. - now / prev) * 100.).round() as u32, 39);
    }
}
