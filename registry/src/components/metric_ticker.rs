//! Host-driven live metrics with rolling values and recent-history details.
use crate::primitives::{
    control::UiLanguage,
    family::{Paint, PhaseClock, Slide, disabled_motion, ease, measure},
    presentation::{column, hint, row, title, tr},
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};

#[derive(Default)]
struct ClippedRoll {
    shown: Option<(f64, SharedString)>,
    previous: SharedString,
    direction: f32,
    clock: PhaseClock,
}

fn grouped_core(metric: &TickerMetric) -> String {
    let number = format!("{:.*}", metric.decimals, metric.value.abs());
    let (whole, part) = number.split_once('.').unwrap_or((&number, ""));
    let mut grouped = String::new();
    for (i, c) in whole.chars().enumerate() {
        if i > 0 && (whole.len() - i).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(c);
    }
    if !part.is_empty() {
        grouped.push('.');
        grouped.push_str(part);
    }
    grouped
}

impl ClippedRoll {
    #[allow(clippy::too_many_arguments)]
    fn render(
        &mut self,
        metric: &TickerMetric,
        font_size: f32,
        line_h: f32,
        visible: bool,
        accessible: bool,
        p: Paint,
        w: &Window,
        cx: &App,
    ) -> Div {
        let core: SharedString = grouped_core(metric).into();
        let _ = accessible;
        if let Some((old_val, old_text)) = &self.shown {
            if *old_text != core {
                self.previous = old_text.clone();
                self.direction = if metric.value >= *old_val { 1. } else { -1. };
                self.shown = Some((metric.value, core.clone()));
                if visible && !disabled_motion(cx) {
                    self.clock.restart(cx);
                }
            }
        } else {
            self.previous = core.clone();
            self.direction = 1.;
            self.shown = Some((metric.value, core.clone()));
        }

        let duration = 0.24;
        let elapsed = if visible {
            self.clock.time(duration, w, cx)
        } else {
            duration
        };
        let animating = elapsed < duration && !disabled_motion(cx);
        let t = if animating {
            ease((elapsed / duration).clamp(0., 1.))
        } else {
            1.
        };

        let prev_chars: Vec<char> = self.previous.chars().collect();
        let next_chars: Vec<char> = core.chars().collect();
        let slots = if animating {
            prev_chars.len().max(next_chars.len())
        } else {
            next_chars.len()
        };
        let prev_pad = slots.saturating_sub(prev_chars.len());
        let next_pad = slots.saturating_sub(next_chars.len());

        let digit_w = measure(&"0".into(), font_size, w).ceil();
        let comma_w = measure(&",".into(), font_size, w).ceil();
        let dot_w = measure(&".".into(), font_size, w).ceil();
        let char_w = |ch: char| -> f32 {
            if ch == ' ' {
                0.
            } else if ch.is_ascii_digit() {
                digit_w
            } else if ch == ',' {
                comma_w
            } else if ch == '.' {
                dot_w
            } else {
                measure(&ch.to_string().into(), font_size, w).ceil()
            }
        };

        let mut container = div()
            .h(px(line_h))
            .overflow_hidden()
            .flex()
            .items_center()
            .whitespace_nowrap()
            .text_size(px(font_size))
            .line_height(px(line_h))
            .font_weight(FontWeight::SEMIBOLD)
            .font_features(FontFeatures(vec![("tnum".into(), 1)].into()))
            .text_color(p.ink);

        if !metric.prefix.is_empty() {
            container = container.child(
                div()
                    .flex_none()
                    .h(px(line_h))
                    .line_height(px(line_h))
                    .overflow_hidden()
                    .child(metric.prefix.clone()),
            );
        }
        if metric.value < 0. {
            container = container.child(
                div()
                    .flex_none()
                    .h(px(line_h))
                    .line_height(px(line_h))
                    .overflow_hidden()
                    .child("−"),
            );
        }

        for idx in 0..slots {
            let p_ch = if idx < prev_pad {
                ' '
            } else {
                prev_chars[idx - prev_pad]
            };
            let n_ch = if idx < next_pad {
                ' '
            } else {
                next_chars[idx - next_pad]
            };
            if !animating || p_ch == n_ch {
                if n_ch == ' ' {
                    continue;
                }
                let cw = char_w(n_ch);
                container = container.child(
                    div()
                        .relative()
                        .flex_none()
                        .w(px(cw))
                        .h(px(line_h))
                        .overflow_hidden()
                        .child(
                            div()
                                .w(px(cw))
                                .h(px(line_h))
                                .line_height(px(line_h))
                                .child(n_ch.to_string()),
                        ),
                );
            } else {
                let wp = char_w(p_ch);
                let wn = char_w(n_ch);
                let cw = (wp + (wn - wp) * t).max(0.5);
                container = container.child(
                    div()
                        .relative()
                        .flex_none()
                        .w(px(cw))
                        .h(px(line_h))
                        .overflow_hidden()
                        .when(p_ch != ' ', |col| {
                            col.child(
                                div()
                                    .absolute()
                                    .left_0()
                                    .top(px(-self.direction * line_h * t))
                                    .w(px(wp))
                                    .h(px(line_h))
                                    .line_height(px(line_h))
                                    .opacity((1. - t * 1.15).clamp(0., 1.))
                                    .child(p_ch.to_string()),
                            )
                        })
                        .when(n_ch != ' ', |col| {
                            col.child(
                                div()
                                    .absolute()
                                    .left_0()
                                    .top(px(self.direction * line_h * (1. - t)))
                                    .w(px(wn))
                                    .h(px(line_h))
                                    .line_height(px(line_h))
                                    .opacity(t.clamp(0., 1.))
                                    .child(n_ch.to_string()),
                            )
                        }),
                );
            }
        }

        if !metric.suffix.is_empty() {
            container = container.child(
                div()
                    .flex_none()
                    .h(px(line_h))
                    .line_height(px(line_h))
                    .overflow_hidden()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.secondary)
                    .child(metric.suffix.clone()),
            );
        }

        container
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TickerDelta {
    Percent,
    Absolute,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TickerGood {
    Up,
    Down,
}
#[derive(Clone, Debug)]
pub struct TickerMetric {
    pub id: SharedString,
    pub label: SharedString,
    pub value: f64,
    pub prefix: SharedString,
    pub suffix: SharedString,
    pub decimals: usize,
    pub delta: TickerDelta,
    pub good: TickerGood,
    pub history: Vec<f64>,
    pub caption: SharedString,
}
impl TickerMetric {
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>, value: f64) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            value,
            prefix: "".into(),
            suffix: "".into(),
            decimals: 0,
            delta: TickerDelta::Percent,
            good: TickerGood::Up,
            history: vec![],
            caption: "".into(),
        }
    }
    pub fn with_format(
        mut self,
        prefix: impl Into<SharedString>,
        suffix: impl Into<SharedString>,
        decimals: usize,
    ) -> Self {
        self.prefix = prefix.into();
        self.suffix = suffix.into();
        self.decimals = decimals.min(6);
        self
    }
    pub fn with_change(mut self, delta: TickerDelta, good: TickerGood) -> Self {
        self.delta = delta;
        self.good = good;
        self
    }
    pub fn with_history(mut self, history: Vec<f64>) -> Self {
        self.history = history;
        self
    }
    pub fn with_caption(mut self, caption: impl Into<SharedString>) -> Self {
        self.caption = caption.into();
        self
    }
}
struct Entry {
    metric: TickerMetric,
    roll: ClippedRoll,
    duplicate_roll: ClippedRoll,
    previous: Option<f64>,
    history: Vec<f64>,
    flash: PhaseClock,
}
#[derive(Clone, Debug)]
pub struct TickerInspect {
    pub id: SharedString,
    pub value: f64,
}
/// `MetricTicker` component.
///
/// # Examples
///
/// ```no_run
/// # #[allow(unused_variables, unused_imports)]
/// # fn example(window: &mut gpui::Window, cx: &mut gpui::App) {
/// # use gpui::prelude::*;
/// use gpui::prelude::*;
/// use kui::components::metric_ticker::{MetricTicker, TickerMetric};
///
/// let metrics = vec![TickerMetric::new("requests", "Requests", 1234.0)];
/// let control = cx.new(|cx| MetricTicker::new("Live metrics", metrics, cx));
/// # }
/// ```
pub struct MetricTicker {
    label: SharedString,
    entries: Vec<Entry>,
    selected: Option<usize>,
    last_selected: Option<usize>,
    pinned: bool,
    repeated: bool,
    hover: bool,
    visible: bool,
    scroll: ScrollHandle,
    focus: FocusHandle,
    was_focused: bool,
    panel_x: Slide,
    panel_reveal: Slide,
    speed: Slide,
    clock: PhaseClock,
    last_time: f32,
    measured: Rc<Cell<Bounds<Pixels>>>,
    marquee: bool,
    inline_deltas: bool,
}
impl EventEmitter<TickerInspect> for MetricTicker {}
impl MetricTicker {
    pub fn new(
        label: impl Into<SharedString>,
        metrics: Vec<TickerMetric>,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut result = Self {
            label: label.into(),
            entries: vec![],
            selected: None,
            last_selected: None,
            pinned: false,
            repeated: false,
            hover: false,
            visible: true,
            scroll: ScrollHandle::new(),
            focus: cx.focus_handle().tab_stop(true),
            was_focused: false,
            panel_x: Slide::new(0.),
            panel_reveal: Slide::new(0.),
            speed: Slide::new(0.),
            clock: PhaseClock::default(),
            last_time: 0.,
            measured: Default::default(),
            marquee: false,
            inline_deltas: true,
        };
        result.set_metrics(metrics, cx);
        result
    }
    pub fn with_marquee(mut self, on: bool) -> Self {
        self.marquee = on;
        self
    }
    /// Control whether the compact metric cells show their change badges.
    pub fn with_inline_deltas(mut self, enabled: bool) -> Self {
        self.inline_deltas = enabled;
        self
    }
    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        if self.visible != visible {
            self.visible = visible;
            self.clock.restart(cx);
            self.last_time = 0.;
            if visible {
                cx.notify();
            }
        }
    }
    pub fn set_metrics(&mut self, metrics: Vec<TickerMetric>, cx: &mut Context<Self>) {
        let selected_id = self
            .selected
            .and_then(|i| self.entries.get(i).map(|entry| entry.metric.id.clone()));
        let mut old: Vec<Option<Entry>> = std::mem::take(&mut self.entries)
            .into_iter()
            .map(Some)
            .collect();
        let mut entries = Vec::new();
        for mut metric in metrics {
            if !metric.value.is_finite() {
                continue;
            }
            metric.history.retain(|v| v.is_finite());
            let old_index = old.iter().position(|entry| {
                entry
                    .as_ref()
                    .is_some_and(|entry| entry.metric.id == metric.id)
            });
            if let Some(mut entry) = old_index.and_then(|i| old[i].take()) {
                if entry.metric.value != metric.value {
                    entry.previous = Some(entry.metric.value);
                    entry.flash.restart(cx);
                }
                if metric.history.is_empty() {
                    if entry.history.last() != Some(&metric.value) {
                        entry.history.push(metric.value);
                    }
                    if entry.history.len() > 24 {
                        entry.history.remove(0);
                    }
                } else {
                    entry.history = metric
                        .history
                        .iter()
                        .rev()
                        .take(24)
                        .rev()
                        .copied()
                        .collect();
                }
                if entry.metric.prefix != metric.prefix
                    || entry.metric.suffix != metric.suffix
                    || entry.metric.decimals != metric.decimals
                {
                    entry.roll = ClippedRoll::default();
                    entry.duplicate_roll = ClippedRoll::default();
                }
                entry.metric = metric;
                entries.push(entry);
            } else {
                let history = if metric.history.is_empty() {
                    vec![metric.value]
                } else {
                    metric
                        .history
                        .iter()
                        .rev()
                        .take(24)
                        .rev()
                        .copied()
                        .collect()
                };
                entries.push(Entry {
                    metric,
                    roll: ClippedRoll::default(),
                    duplicate_roll: ClippedRoll::default(),
                    previous: None,
                    history,
                    flash: PhaseClock::default(),
                });
            }
        }
        self.entries = entries;
        self.selected =
            selected_id.and_then(|id| self.entries.iter().position(|entry| entry.metric.id == id));
        if self.selected.is_none() {
            self.pinned = false;
        }
        cx.notify();
    }
    fn select(&mut self, index: Option<usize>, cx: &mut Context<Self>) {
        if self.selected == index {
            return;
        }
        // The panel grows out of the metric that opens it; gliding between
        // metrics keeps it open, so the reveal only replays after a close.
        let was_open = self.selected.is_some();
        self.selected = index;
        if let Some(index) = index {
            self.last_selected = Some(index);
        }
        if index.is_some() && !was_open {
            self.panel_reveal.reset(0.);
            self.panel_reveal.duration(1., 0.42, cx);
        }
        if let Some(index) = index
            && let Some(entry) = self.entries.get(index)
        {
            cx.emit(TickerInspect {
                id: entry.metric.id.clone(),
                value: entry.metric.value,
            });
        }
        cx.notify();
    }
}
fn delta_chip(arrow: &'static str, amount: &str, good: bool, age: f32, p: Paint) -> Div {
    let tone = if good { p.success } else { p.danger };
    let wash = if good { 0.14 } else { 0.13 };
    let flash = age < 1.6;
    // The arrow slides in from the direction of travel, then rests.
    let enter = ((0.3 - age.min(0.3)) / 0.3).max(0.);
    let offset = match arrow {
        "↑" => enter * 3.,
        "↓" => -enter * 3.,
        _ => 0.,
    };
    div()
        .relative()
        .flex_none()
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(2.))
                .pt(px(1.))
                .pb(px(1.))
                .pl(px(4.))
                .pr(px(6.))
                .rounded(px(8.))
                .text_size(px(11.))
                .line_height(px(16.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(if flash {
                    tone
                } else {
                    p.faint.blend(tone.opacity(0.7))
                })
                .when(flash, |chip| chip.bg(tone.opacity(wash)))
                .child(div().mt(px(offset)).child(arrow))
                .child(amount.to_owned()),
        )
        // A thin ring expands out of the chip once and fades with the flash.
        .when(flash && age < 0.45, |chip| {
            let t = age / 0.45;
            chip.child(
                canvas(
                    move |_, _, _| {},
                    move |b, _, w, _| {
                        let grow = px(1. + t * 5.);
                        let bounds = Bounds {
                            origin: point(b.origin.x - grow, b.origin.y - grow),
                            size: size(b.size.width + grow * 2., b.size.height + grow * 2.),
                        };
                        w.paint_quad(quad(
                            bounds,
                            px(8. + t * 5.),
                            Hsla::transparent_black(),
                            Edges::all(px(1.)),
                            tone.opacity((1. - t) * 0.9),
                            BorderStyle::Solid,
                        ));
                        w.request_animation_frame();
                    },
                )
                .absolute()
                .inset_0(),
            )
        })
}
fn change_text(previous: Option<f64>, metric: &TickerMetric) -> (&'static str, String, bool) {
    let Some(previous) = previous else {
        return ("→", "—".into(), true);
    };
    let diff = metric.value - previous;
    if previous == 0. && metric.delta == TickerDelta::Percent {
        return ("→", "—".into(), true);
    }
    let good = if metric.good == TickerGood::Up {
        diff >= 0.
    } else {
        diff <= 0.
    };
    let arrow = if diff > 0. {
        "↑"
    } else if diff < 0. {
        "↓"
    } else {
        "→"
    };
    let (value, suffix) = match metric.delta {
        TickerDelta::Percent if previous != 0. => ((diff / previous.abs() * 100.).abs(), "%"),
        TickerDelta::Percent => (0., "%"),
        TickerDelta::Absolute => (
            diff.abs(),
            if metric.suffix.as_ref() == "%" {
                " pts"
            } else {
                metric.suffix.as_ref()
            },
        ),
    };
    (arrow, format!("{value:.1}{suffix}"), good)
}
fn display_value(metric: &TickerMetric) -> String {
    format!(
        "{}{}{}{}",
        metric.prefix,
        if metric.value < 0. { "−" } else { "" },
        grouped_core(metric),
        metric.suffix
    )
}
impl Render for MetricTicker {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let focused = self.focus.is_focused(w);
        if focused && !self.was_focused && self.selected.is_none() && !self.entries.is_empty() {
            // Tab lands on the metric that last held focus, falling back to the first.
            let restore = self
                .last_selected
                .filter(|&i| i < self.entries.len())
                .or(Some(0));
            self.select(restore, cx);
        }
        if !focused && self.was_focused && !self.pinned && !self.hover {
            self.select(None, cx);
        }
        self.was_focused = focused;
        let measured = self.measured.get();
        let viewport_w = f32::from(w.viewport_size().width);
        let width = if measured.size.width > px(1.) {
            f32::from(measured.size.width)
        } else {
            (viewport_w - 40.).clamp(1., 520.)
        };
        let count = self.entries.len().max(1);
        let desktop = width >= 476. || (measured.size.width <= px(1.) && viewport_w >= 520.);
        let min_marquee_w = if self.inline_deltas { 180. } else { 156. };
        let cell_width = (width / count as f32).max(min_marquee_w);
        let content_width = cell_width * self.entries.len() as f32;
        let repeat = self.marquee && desktop && !disabled_motion(cx) && content_width > width + 1.;
        let moving = self.visible && repeat && w.is_visible();
        let max = f32::from(self.scroll.max_offset().x).abs();
        let target_speed = if moving && max > 0. && !self.hover && self.selected.is_none() {
            32.
        } else {
            0.
        };
        self.speed.duration(target_speed, 0.5, cx);
        let speed = if moving {
            self.speed.get(w, cx)
        } else {
            self.speed.reset(0.);
            0.
        };
        if moving && max > 0. && speed > 0.01 {
            if !self.clock.age(cx).is_finite() {
                self.clock.restart(cx);
                self.last_time = 0.;
            }
            let age = self.clock.age(cx);
            let dt = (age - self.last_time).clamp(0., 0.05);
            self.last_time = age;
            let current = -f32::from(self.scroll.offset().x);
            let mut next = current + dt * speed;
            if next >= content_width {
                next -= content_width;
            }
            self.scroll.set_offset(point(px(-next), px(0.)));
            w.request_animation_frame();
        } else {
            self.clock.restart(cx);
            self.last_time = 0.;
        }

        let col_est = if repeat {
            cell_width
        } else if desktop {
            width / count as f32
        } else {
            width * 0.5
        };
        let tight_col = col_est < 122.;
        let font_size = if tight_col { 15.5 } else { 18. };
        let line_h = if tight_col { 21. } else { 24. };
        let cell_px = if tight_col { 9. } else { 14. };
        let label_size = if tight_col { 11. } else { 12. };
        let label_lh = if tight_col { 14.5 } else { 16.5 };
        let visible_roll = self.visible && w.is_visible();
        let inline_deltas = self.inline_deltas;
        let selected_idx = self.selected;
        let repeated_sel = self.repeated;
        let radius = crate::foundation::surface_radius(cx);
        let total = self.entries.len();
        let last_row = total.saturating_sub(1) / 2;

        let mut cells = Vec::with_capacity(total);
        let mut duplicates = Vec::new();
        for (i, entry) in self.entries.iter_mut().enumerate() {
            let (arrow, amount, good) = change_text(entry.previous, &entry.metric);
            let change = if amount == "—" {
                amount.clone()
            } else {
                format!("{arrow} {amount}")
            };
            let age = if visible_roll && inline_deltas {
                entry.flash.time(1.6, w, cx)
            } else {
                1.6
            };
            let counter_el = entry.roll.render(
                &entry.metric,
                font_size,
                line_h,
                visible_roll,
                true,
                p,
                w,
                cx,
            );
            let label_text = entry.metric.label.clone();
            let aria = format!(
                "{} · {} · {}",
                label_text,
                display_value(&entry.metric),
                change
            );
            let odd_trailing = !desktop && total % 2 == 1 && i + 1 == total;

            let cell = div()
                .id(("ticker-metric", i))
                .role(Role::Button)
                .tab_index(-1)
                .aria_expanded(selected_idx == Some(i))
                .when(selected_idx == Some(i), |v| v.aria_active_descendant())
                .aria_label(aria)
                .relative()
                .when(repeat, |v| {
                    v.flex_none()
                        .w(px(cell_width - 8.))
                        .m(px(4.))
                        .rounded(radius)
                })
                .when(!repeat && desktop, |v| v.flex_1().min_w_0())
                .when(!repeat && !desktop && odd_trailing, |v| {
                    v.w_full().min_w_0()
                })
                .when(!repeat && !desktop && !odd_trailing, |v| {
                    v.w(relative(0.5)).min_w_0()
                })
                .when(!repeat && !desktop && (i / 2) < last_row, |v| {
                    v.border_b_1().border_color(p.line)
                })
                .when(!repeat && !desktop && i % 2 == 0 && i + 1 < total, |v| {
                    v.border_r_1().border_color(p.line)
                })
                .px(px(cell_px))
                .pt(px(10.))
                .pb(px(11.))
                .flex()
                .flex_col()
                .justify_between()
                .gap(px(6.))
                .overflow_hidden()
                .hover(|s| s.bg(p.tray))
                .when(selected_idx == Some(i) && !repeated_sel, |v| v.bg(p.tray))
                .child(
                    div()
                        .min_w_0()
                        .text_size(px(label_size))
                        .line_height(px(label_lh))
                        .text_color(p.faint)
                        .child(label_text.clone()),
                )
                .child(
                    row()
                        .min_w_0()
                        .flex_wrap()
                        .items_center()
                        .gap_x(px(6.))
                        .gap_y(px(2.))
                        .child(counter_el)
                        .when(inline_deltas, |r| {
                            r.child(delta_chip(arrow, &amount, good, age, p))
                        }),
                )
                .when((repeat || desktop) && i + 1 < total, |v| {
                    v.child(
                        div()
                            .absolute()
                            .right_0()
                            .top(px(12.))
                            .bottom(px(12.))
                            .w(px(1.))
                            .bg(p.line),
                    )
                })
                .on_hover(cx.listener(move |s, inside: &bool, _, cx| {
                    if *inside && !s.pinned {
                        if s.repeated {
                            s.repeated = false;
                            cx.notify();
                        }
                        s.select(Some(i), cx);
                    }
                }))
                .on_click(cx.listener(move |s, _, _, cx| {
                    s.repeated = false;
                    if s.pinned && s.selected == Some(i) {
                        s.pinned = false;
                        s.select(None, cx);
                    } else {
                        s.pinned = true;
                        s.select(Some(i), cx);
                    }
                }));
            cells.push(cell);

            if repeat {
                let dup_counter = entry.duplicate_roll.render(
                    &entry.metric,
                    font_size,
                    line_h,
                    visible_roll,
                    false,
                    p,
                    w,
                    cx,
                );
                duplicates.push(
                    div()
                        .id(("ticker-repeat", i))
                        .flex_none()
                        .relative()
                        .w(px(cell_width - 8.))
                        .m(px(4.))
                        .px(px(cell_px))
                        .pt(px(10.))
                        .pb(px(11.))
                        .rounded(radius)
                        .flex()
                        .flex_col()
                        .justify_between()
                        .gap(px(6.))
                        .overflow_hidden()
                        .hover(|s| s.bg(p.tray))
                        .when(selected_idx == Some(i) && repeated_sel, |v| v.bg(p.tray))
                        .child(
                            div()
                                .min_w_0()
                                .text_size(px(label_size))
                                .line_height(px(label_lh))
                                .text_color(p.faint)
                                .child(label_text),
                        )
                        .child(
                            row()
                                .min_w_0()
                                .flex_wrap()
                                .items_center()
                                .gap_x(px(6.))
                                .gap_y(px(2.))
                                .child(dup_counter)
                                .when(inline_deltas, |r| {
                                    r.child(delta_chip(arrow, &amount, good, 1.6, p))
                                }),
                        )
                        .child(
                            div()
                                .absolute()
                                .right_0()
                                .top(px(12.))
                                .bottom(px(12.))
                                .w(px(1.))
                                .bg(p.line),
                        )
                        .on_hover(cx.listener(move |s, inside: &bool, _, cx| {
                            if *inside && !s.pinned {
                                if !s.repeated {
                                    s.repeated = true;
                                    cx.notify();
                                }
                                s.select(Some(i), cx);
                            }
                        }))
                        .on_click(cx.listener(move |s, _, _, cx| {
                            s.repeated = true;
                            if s.pinned && s.selected == Some(i) {
                                s.pinned = false;
                                s.select(None, cx);
                            } else {
                                s.pinned = true;
                                s.select(Some(i), cx);
                            }
                        })),
                );
            }
        }

        let scroll = self.scroll.clone();
        let measured = self.measured.clone();
        let strip = div()
            .id("metric-ticker")
            .role(Role::Group)
            .tab_index(0)
            .track_focus(&self.focus)
            .aria_label(self.label.clone())
            .relative()
            .w_full()
            .rounded(radius)
            .border_1()
            .border_color(p.line)
            .bg(p.panel)
            .overflow_hidden()
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, _, cx| {
                let n = s.entries.len();
                if n == 0 {
                    return;
                }
                let i = s.selected.unwrap_or(0);
                let next = match e.keystroke.key.as_str() {
                    "right" => (i + 1) % n,
                    "left" => (i + n - 1) % n,
                    "home" => 0,
                    "end" => n - 1,
                    "enter" | "space" => {
                        s.pinned = !s.pinned;
                        if s.pinned {
                            s.select(Some(i), cx);
                        } else {
                            s.select(None, cx);
                        }
                        cx.stop_propagation();
                        return;
                    }
                    "escape" => {
                        s.pinned = false;
                        s.select(None, cx);
                        cx.stop_propagation();
                        return;
                    }
                    _ => return,
                };
                s.select(Some(next), cx);
                if s.marquee {
                    s.scroll.scroll_to_item(next);
                }
                cx.stop_propagation();
            }))
            .child(
                canvas(
                    move |b, w, _| {
                        if measured.get().size != b.size {
                            w.request_animation_frame();
                        }
                        measured.set(b);
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .inset_0(),
            )
            .child(
                div()
                    .id("ticker-scroll")
                    .w_full()
                    .flex()
                    .flex_row()
                    .when(!repeat && !desktop, |el| el.flex_wrap())
                    .when(repeat, |el| el.overflow_x_scroll().track_scroll(&scroll))
                    .children(cells)
                    .children(duplicates),
            )
            .when(repeat, |strip| {
                let panel = p.panel;
                strip.child(
                    canvas(
                        move |_, _, _| {},
                        move |b, _, w, _| {
                            let feather = px(24.);
                            let left = Bounds {
                                origin: b.origin,
                                size: size(feather, b.size.height),
                            };
                            let right = Bounds {
                                origin: point(b.origin.x + b.size.width - feather, b.origin.y),
                                size: size(feather, b.size.height),
                            };
                            w.paint_quad(quad(
                                left,
                                0.,
                                linear_gradient(
                                    90.,
                                    linear_color_stop(panel.opacity(0.95), 0.),
                                    linear_color_stop(panel.opacity(0.), 1.),
                                ),
                                Edges::all(px(0.)),
                                Hsla::transparent_black(),
                                BorderStyle::Solid,
                            ));
                            w.paint_quad(quad(
                                right,
                                0.,
                                linear_gradient(
                                    270.,
                                    linear_color_stop(panel.opacity(0.95), 0.),
                                    linear_color_stop(panel.opacity(0.), 1.),
                                ),
                                Edges::all(px(0.)),
                                Hsla::transparent_black(),
                                BorderStyle::Solid,
                            ));
                        },
                    )
                    .absolute()
                    .inset_0(),
                )
            });
        let reveal = if self.visible && w.is_visible() {
            self.panel_reveal.get(w, cx)
        } else {
            self.panel_reveal.value(cx)
        }
        .clamp(0., 1.);
        let panel = self.selected.and_then(|index| {
            self.entries.get(index).map(|entry| {
                let values = entry.history.clone();
                let min = values.iter().copied().fold(f64::INFINITY, f64::min);
                let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                let low = if min.is_finite() { min } else { 0. };
                let high = if max.is_finite() { max } else { 0. };
                let caption = if entry.metric.caption.is_empty() {
                    let count = values.len();
                    let english = format!("Last {count} updates");
                    let chinese = format!("近 {count} 次更新");
                    UiLanguage::current(cx).text(&english, &chinese).into()
                } else {
                    entry.metric.caption.clone()
                };
                let trend = if values.last() >= values.first() {
                    tr(cx, "Trending up", "趋势上升")
                } else {
                    tr(cx, "Trending down", "趋势下降")
                };
                let chart = canvas(
                    |_, _, _| {},
                    move |bounds, _, w, _| {
                        if values.is_empty() {
                            return;
                        }
                        let dx = f32::from(bounds.size.width)
                            / (values.len().saturating_sub(1).max(1)) as f32;
                        let dy = f32::from(bounds.size.height);
                        let spread = (high - low).max(0.00001);
                        let points: Vec<_> = values
                            .iter()
                            .enumerate()
                            .map(|(i, v)| {
                                point(
                                    bounds.origin.x + px(i as f32 * dx),
                                    bounds.origin.y
                                        + px(dy - (*v - low) as f32 / spread as f32 * dy),
                                )
                            })
                            .collect();
                        let progress = (points.len().saturating_sub(1)) as f32 * reveal;
                        let limit = progress.floor() as usize;
                        let mut visible =
                            points.iter().copied().take(limit + 1).collect::<Vec<_>>();
                        if limit + 1 < points.len() {
                            let a = points[limit];
                            let b = points[limit + 1];
                            let t = progress - limit as f32;
                            visible.push(point(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t));
                        }
                        let mut area = PathBuilder::fill();
                        if let Some(first) = visible.first() {
                            area.move_to(point(first.x, bounds.origin.y + px(dy)));
                            for q in &visible {
                                area.line_to(*q);
                            }
                            if let Some(last) = visible.last() {
                                area.line_to(point(last.x, bounds.origin.y + px(dy)));
                            }
                            area.close();
                            if let Ok(area) = area.build() {
                                w.paint_path(
                                    area,
                                    linear_gradient(
                                        180.,
                                        linear_color_stop(p.bright.opacity(0.14), 0.),
                                        linear_color_stop(p.bright.opacity(0.), 1.),
                                    ),
                                );
                            }
                        }
                        let mut path = PathBuilder::stroke(px(1.75));
                        for (i, q) in visible.iter().enumerate() {
                            if i == 0 {
                                path.move_to(*q);
                            } else {
                                path.line_to(*q);
                            }
                        }
                        if let Ok(path) = path.build() {
                            w.paint_path(path, p.bright);
                        }
                    },
                )
                .w_full()
                .h(px(56.));
                let open = ease(reveal);
                column()
                    .w(px(248.))
                    .max_w_full()
                    .gap(px(8.))
                    .px(px(14.))
                    .py(px(12.))
                    .shadow_lg()
                    .rounded(radius)
                    .bg(p.tray)
                    .border_1()
                    .border_color(p.line)
                    .opacity(0.01 + 0.99 * open)
                    .when(open < 1., |v| v.h(px(58. + 106. * open)).overflow_hidden())
                    .child(
                        row()
                            .justify_between()
                            .gap_2()
                            .child(hint(caption, p))
                            .child(hint(trend, p)),
                    )
                    .child(title(entry.metric.label.clone(), p))
                    .child(chart)
                    .child(
                        row()
                            .justify_between()
                            .child(hint(format!("{} {low:.1}", tr(cx, "Low", "最低")), p))
                            .child(hint(format!("{} {high:.1}", tr(cx, "High", "最高")), p)),
                    )
            })
        });
        let desired = self.selected.map_or(0., |i| {
            let slot_x = if repeat {
                i as f32 * cell_width
                    + if self.repeated { content_width } else { 0. }
                    + f32::from(self.scroll.offset().x)
            } else if desktop {
                i as f32 * (width / count as f32)
            } else {
                (i % 2) as f32 * (width * 0.5)
            };
            slot_x.clamp(0., (width - 248.).max(0.))
        });
        self.panel_x.duration(desired, 0.42, cx);
        let x = if self.visible && w.is_visible() {
            self.panel_x.get(w, cx)
        } else {
            self.panel_x.value(cx)
        };
        column()
            .id("ticker-shell")
            .relative()
            .w_full()
            .gap(px(8.))
            .text_color(p.ink)
            .on_mouse_down_out(cx.listener(|s, _, _, cx| {
                if s.pinned {
                    s.pinned = false;
                    s.select(None, cx);
                }
            }))
            .on_hover(cx.listener(|s, inside: &bool, _, cx| {
                s.hover = *inside;
                if !inside && !s.pinned {
                    s.select(None, cx);
                }
                cx.notify();
            }))
            .child(strip)
            .when_some(panel, |v, panel| {
                v.child(div().w_full().pl(px(x)).child(panel))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::{MetricTicker, TickerDelta, TickerGood, TickerMetric, change_text, display_value};
    use gpui::TestAppContext;

    #[test]
    fn delta_handles_direction_and_zero_baselines() {
        let mut metric = TickerMetric::new("latency", "Latency", 110.)
            .with_change(TickerDelta::Absolute, TickerGood::Down)
            .with_format("", " ms", 0);
        assert_eq!(
            change_text(Some(100.), &metric),
            ("↑", "10.0 ms".to_string(), false)
        );
        metric.value = 90.;
        assert_eq!(
            change_text(Some(100.), &metric),
            ("↓", "10.0 ms".to_string(), true)
        );
        metric.delta = TickerDelta::Percent;
        assert_eq!(change_text(Some(0.), &metric).1, "—");
        metric.delta = TickerDelta::Absolute;
        metric.suffix = "%".into();
        assert_eq!(change_text(Some(100.), &metric).1, "10.0 pts");
    }

    #[test]
    fn duplicate_matches_grouped_value() {
        let metric = TickerMetric::new("revenue", "Revenue", 12345.25).with_format("$", "", 2);
        assert_eq!(display_value(&metric), "$12,345.25");
    }

    #[test]
    fn desktop_and_mobile_layouts_mount_and_update_cleanly() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let metrics = vec![
            TickerMetric::new("mrr", "Monthly recurring revenue", 84_210.).with_format("$", "", 0),
            TickerMetric::new("active", "Active now", 1_284.),
            TickerMetric::new("conv", "Trial conversion", 18.4).with_format("", "%", 1),
            TickerMetric::new("lat", "API latency", 182.).with_format("", " ms", 0),
        ];
        let (view, w) =
            app.add_window_view(|_, cx| MetricTicker::new("Product health", metrics, cx));
        w.run_until_parked();
        w.update(|_, cx| {
            view.update(cx, |ticker, cx| {
                ticker.set_metrics(
                    vec![
                        TickerMetric::new("mrr", "Monthly recurring revenue", 85_000.)
                            .with_format("$", "", 0),
                        TickerMetric::new("active", "Active now", 1_401.),
                        TickerMetric::new("conv", "Trial conversion", 18.6).with_format("", "%", 1),
                        TickerMetric::new("lat", "API latency", 190.).with_format("", " ms", 0),
                    ],
                    cx,
                );
            });
        });
        w.run_until_parked();
    }
}
