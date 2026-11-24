//! Compact two-line A/B experiment card unfolding on hover with variant chart and winner selection.
//!
//! Cross-links a `PathBuilder` miniature head chart with tactile `pro_tactile` variant rows, a liquid
//! `pro_pill_indicator` metric switcher (`Conversion`, `Revenue`, `Retention`) with rolling numbers, and `pro_check_draw` winner confirmation.

use crate::{
    components::pro_chrome::{
        ProPalette, pro_check_draw, pro_eid, pro_empty_state, pro_fold, pro_mark_selected,
        pro_pill_indicator, pro_popover_presence, pro_rolling_number, pro_selection_sensor,
        pro_skeleton_bar, pro_spring_f32, pro_swap_label, pro_tactile, pro_toast_banner, tr,
    },
    primitives::{
        family::{Slide, disabled_motion},
        pro_icons::{ProIcon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::collections::BTreeMap;
use web_time::Instant;

const LIVE_DOT_SVG: &str = r#"<circle cx="5" cy="5" r="4.5" fill="currentColor" opacity="0.22"></circle><circle cx="5" cy="5" r="2.25" fill="currentColor"></circle>"#;
const TREND_LEAD_SVG: &str = r#"<path d="M1.5 8.5 4.5 5.5l2 2 4-4M8 3.5h2.5V6" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round"></path>"#;
const TREND_DROP_SVG: &str = r#"<path d="M1.5 3.5 4.5 6.5l2-2 4 4M8 8.5h2.5V6" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round"></path>"#;
const ARROW_SHIP_SVG: &str = r#"<path d="M2.5 6h7M6.5 3l3 3-3 3" stroke="currentColor" stroke-width="1.35" stroke-linecap="round" stroke-linejoin="round"></path>"#;

/// Semantic status tone for [`ExperimentStatus`] and lift badges.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ExperimentTone {
    Red,
    Amber,
    #[default]
    Green,
    Blue,
    Violet,
    Gray,
}

impl ExperimentTone {
    fn colors(self, dark: bool) -> (Hsla, Hsla) {
        match (self, dark) {
            (Self::Red, false) => (hsla(4. / 360., 0.86, 0.58, 0.09), rgb(0xf04438).into()),
            (Self::Red, true) => (hsla(4. / 360., 1.0, 0.69, 0.14), rgb(0xff6b5f).into()),
            (Self::Amber, false) => (hsla(45. / 360., 1.0, 0.45, 0.11), rgb(0x8f6c00).into()),
            (Self::Amber, true) => (hsla(46. / 360., 0.75, 0.60, 0.14), rgb(0xe6c24d).into()),
            (Self::Green, false) => (hsla(145. / 360., 0.49, 0.58, 0.10), rgb(0x1e8a4f).into()),
            (Self::Green, true) => (hsla(145. / 360., 0.46, 0.65, 0.14), rgb(0x5fc88a).into()),
            (Self::Blue, false) => (
                hsla(215. / 360., 1.0, 0.44, 0.07),
                hsla(215. / 360., 1.0, 0.44, 0.92),
            ),
            (Self::Blue, true) => (hsla(218. / 360., 1.0, 0.65, 0.14), rgb(0x90cdfe).into()),
            (Self::Violet, false) => (hsla(272. / 360., 0.85, 0.63, 0.09), rgb(0xa651f0).into()),
            (Self::Violet, true) => (hsla(268. / 360., 0.78, 0.74, 0.18), rgb(0xb98af0).into()),
            (Self::Gray, false) => (hsla(0., 0., 0., 0.05), hsla(0., 0., 0., 0.61)),
            (Self::Gray, true) => (hsla(0., 0., 1., 0.08), hsla(0., 0., 1., 0.62)),
        }
    }
}

/// Gesture mode that unfolds [`ExperimentCard`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ExperimentTrigger {
    /// Unfold quietly under pointer hover (clicking or pressing Enter/Space also pins/toggles).
    #[default]
    Hover,
    /// Unfold only when clicked or activated via keyboard.
    Click,
}

/// Status badge configuration under the title of [`ExperimentCard`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExperimentStatus {
    /// Short status text (such as `"Running"` or `"Completed"`).
    pub label: SharedString,
    /// Badge color tone.
    pub tone: ExperimentTone,
}

impl ExperimentStatus {
    /// Construct a status badge with `label` and `tone`.
    pub fn new(label: impl Into<SharedString>, tone: ExperimentTone) -> Self {
        Self {
            label: label.into(),
            tone,
        }
    }
}

/// Measured metric tab inside [`ExperimentCard`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExperimentMetric {
    /// Stable metric key referenced by [`ExperimentVariant::values`].
    pub id: SharedString,
    /// Segment button label.
    pub label: SharedString,
    /// Prefix shown before the rolling number (such as `"$"`).
    pub prefix: SharedString,
    /// Suffix shown after the rolling number (such as `"%"`).
    pub suffix: SharedString,
    /// Decimal precision for [`pro_rolling_number`].
    pub decimals: usize,
    /// Whether lower values beat the baseline (for example bounce rate or latency).
    pub lower_is_better: bool,
}

impl ExperimentMetric {
    /// Construct a metric definition with `id` and `label`.
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            prefix: "".into(),
            suffix: "".into(),
            decimals: 1,
            lower_is_better: false,
        }
    }

    /// Set numeric formatting (`prefix`, `decimals`, `suffix`).
    pub fn with_format(
        mut self,
        prefix: impl Into<SharedString>,
        decimals: usize,
        suffix: impl Into<SharedString>,
    ) -> Self {
        self.prefix = prefix.into();
        self.decimals = decimals;
        self.suffix = suffix.into();
        self
    }

    /// Mark negative deltas as improvements (green lead).
    pub fn with_lower_is_better(mut self, lower_is_better: bool) -> Self {
        self.lower_is_better = lower_is_better;
        self
    }
}

/// Individual A/B/n variant measured in [`ExperimentCard`].
#[derive(Clone, Debug, PartialEq)]
pub struct ExperimentVariant {
    /// Stable variant identifier.
    pub id: SharedString,
    /// Single-letter or short badge key (`"A"`, `"B"`, `"C"`).
    pub key: SharedString,
    /// Descriptive variant name.
    pub name: SharedString,
    /// Marks this variant as the baseline control (`true` on at most one variant).
    pub control: bool,
    /// Optional sample size / visitor count shown in the footer readout.
    pub visitors: Option<u64>,
    /// Measured metric value per [`ExperimentMetric::id`].
    pub values: BTreeMap<SharedString, f64>,
    /// Statistical confidence (`0..=100`) per [`ExperimentMetric::id`].
    pub confidence: BTreeMap<SharedString, u8>,
    /// Optional custom accent color override.
    pub color: Option<Hsla>,
}

impl ExperimentVariant {
    /// Construct a variant with `id`, short badge `key`, and `name`.
    pub fn new(
        id: impl Into<SharedString>,
        key: impl Into<SharedString>,
        name: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            key: key.into(),
            name: name.into(),
            control: false,
            visitors: None,
            values: BTreeMap::new(),
            confidence: BTreeMap::new(),
            color: None,
        }
    }

    /// Mark this variant as the baseline control.
    pub fn with_control(mut self, control: bool) -> Self {
        self.control = control;
        self
    }

    /// Set the sample size / visitor count.
    pub fn with_visitors(mut self, visitors: u64) -> Self {
        self.visitors = Some(visitors);
        self
    }

    /// Record a metric value for `metric_id`.
    pub fn with_value(mut self, metric_id: impl Into<SharedString>, value: f64) -> Self {
        self.values.insert(metric_id.into(), value);
        self
    }

    /// Record statistical confidence (`0..=100%`) for `metric_id`.
    pub fn with_confidence(mut self, metric_id: impl Into<SharedString>, pct: u8) -> Self {
        self.confidence.insert(metric_id.into(), pct.min(100));
        self
    }

    /// Override the variant's chart and badge color.
    pub fn with_color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }

    /// Read the finite metric value for `metric_id`, defaulting to `0.0`.
    pub fn metric_value(&self, metric_id: &str) -> f64 {
        self.values
            .get(metric_id)
            .copied()
            .filter(|v| v.is_finite())
            .unwrap_or(0.0)
    }
}

/// Typed events emitted by [`ExperimentCard`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExperimentCardEvent {
    /// Emitted when the card unfolds or folds.
    OpenChanged(bool),
    /// Emitted when the active comparison metric changes.
    MetricChanged(SharedString),
    /// Emitted when a variant is picked as winner (`Some(id)`) or unpicked (`None`).
    WinnerChanged(Option<SharedString>),
    /// Emitted when the footer winner confirmation button is clicked.
    WinnerConfirmed(SharedString),
}

impl EventEmitter<ExperimentCardEvent> for ExperimentCard {}

fn default_variant_color(index: usize, dark: bool) -> Hsla {
    match (index % 5, dark) {
        (0, false) => hsla(0., 0., 0., 0.24),
        (0, true) => hsla(0., 0., 1., 0.32),
        (1, false) => hsla(215. / 360., 1.0, 0.44, 0.80),
        (1, true) => rgb(0x4fb0fe).into(),
        (2, false) => rgb(0xa651f0).into(),
        (2, true) => rgb(0xb98af0).into(),
        (3, false) => rgb(0xe8b000).into(),
        (3, true) => rgb(0xe6c24d).into(),
        (_, false) => rgb(0x1e8a4f).into(),
        (_, true) => rgb(0x5fc88a).into(),
    }
}

fn format_visitors(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

/// Compact A/B test disclosure card (`experiment-card`).
pub struct ExperimentCard {
    id: ElementId,
    title: SharedString,
    status: ExperimentStatus,
    note: Option<SharedString>,
    metrics: Vec<ExperimentMetric>,
    active_metric: SharedString,
    variants: Vec<ExperimentVariant>,
    winner: Option<SharedString>,
    confirmed_winner: Option<SharedString>,
    action_prefix: SharedString,
    hint: Option<SharedString>,
    trigger: ExperimentTrigger,
    hovered_card: bool,
    pinned_open: bool,
    hovered_variant: Option<SharedString>,
    variant_spotlights: Vec<(Slide, Slide)>,
    loading: bool,
    empty: bool,
    focus: FocusHandle,
    toast: Option<(SharedString, Instant)>,
    started_at: Instant,
}

impl ExperimentCard {
    /// Create an [`ExperimentCard`] preloaded with the `"Signup button copy"` 3-variant experiment.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let metrics = vec![
            ExperimentMetric::new("conversion", "Conversion").with_format("", 1, "%"),
            ExperimentMetric::new("revenue", "Revenue").with_format("$", 2, ""),
            ExperimentMetric::new("retention", "Retention").with_format("", 1, "%"),
        ];
        let variants = vec![
            ExperimentVariant::new("a", "A", "Start free trial")
                .with_control(true)
                .with_visitors(4790)
                .with_value("conversion", 4.2)
                .with_value("revenue", 18.40)
                .with_value("retention", 61.4),
            ExperimentVariant::new("b", "B", "Try it free")
                .with_visitors(4812)
                .with_value("conversion", 4.7)
                .with_value("revenue", 19.10)
                .with_value("retention", 64.8)
                .with_confidence("conversion", 96)
                .with_confidence("revenue", 81)
                .with_confidence("retention", 89),
            ExperimentVariant::new("c", "C", "Get started \u{2013} $0")
                .with_visitors(4804)
                .with_value("conversion", 5.1)
                .with_value("revenue", 21.60)
                .with_value("retention", 68.2)
                .with_confidence("conversion", 98)
                .with_confidence("revenue", 94)
                .with_confidence("retention", 96),
        ];
        let variant_spotlights = (0..variants.len())
            .map(|_| (Slide::snappy(1.0), Slide::snappy(0.0)))
            .collect();
        Self {
            id: id.into(),
            title: "Signup button copy".into(),
            status: ExperimentStatus::new("Running", ExperimentTone::Green),
            note: Some("Day 9 of 14".into()),
            metrics,
            active_metric: "conversion".into(),
            variants,
            winner: None,
            confirmed_winner: None,
            action_prefix: "Pick winner".into(),
            hint: None,
            trigger: ExperimentTrigger::Hover,
            hovered_card: false,
            pinned_open: false,
            hovered_variant: None,
            variant_spotlights,
            loading: false,
            empty: false,
            focus: cx.focus_handle().tab_stop(true),
            toast: None,
            started_at: Instant::now(),
        }
    }

    /// Set the experiment title.
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = title.into();
        self
    }

    /// Configure the status badge shown below the title.
    pub fn with_status(mut self, label: impl Into<SharedString>, tone: ExperimentTone) -> Self {
        self.status = ExperimentStatus::new(label, tone);
        self
    }

    /// Set or clear the note shown beside the status pill (for example `"Day 9 of 14"`).
    pub fn with_note(mut self, note: impl Into<SharedString>) -> Self {
        let s: SharedString = note.into();
        self.note = (!s.is_empty()).then_some(s);
        self
    }

    /// Replace the selectable metrics list.
    pub fn with_metrics(mut self, metrics: Vec<ExperimentMetric>) -> Self {
        if let Some(first) = metrics.first()
            && !metrics.iter().any(|m| m.id == self.active_metric)
        {
            self.active_metric = first.id.clone();
        }
        self.metrics = metrics;
        self
    }

    /// Set the initial active metric id.
    pub fn with_default_metric(mut self, metric_id: impl Into<SharedString>) -> Self {
        self.active_metric = metric_id.into();
        self
    }

    /// Replace the experiment variants.
    pub fn with_variants(mut self, variants: Vec<ExperimentVariant>) -> Self {
        self.variant_spotlights = (0..variants.len())
            .map(|_| (Slide::snappy(1.0), Slide::snappy(0.0)))
            .collect();
        self.variants = variants;
        self
    }

    /// Pre-select a winning variant id on mount.
    pub fn with_default_winner(mut self, winner_id: Option<SharedString>) -> Self {
        self.winner = winner_id;
        self
    }

    /// Customize the action button prefix (defaults to `"Pick winner"`, e.g. `"Ship"` -> `"Ship C"`).
    pub fn with_action_prefix(mut self, prefix: impl Into<SharedString>) -> Self {
        self.action_prefix = prefix.into();
        self
    }

    /// Override the resting footer hint text (`"Pick a winner"` by default).
    pub fn with_hint(mut self, hint: impl Into<SharedString>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// Choose whether hovering or clicking unfolds the card.
    pub fn with_trigger(mut self, trigger: ExperimentTrigger) -> Self {
        self.trigger = trigger;
        self
    }

    /// Control whether the card starts unfolded on initial mount.
    pub fn with_default_open(mut self, open: bool) -> Self {
        self.pinned_open = open;
        self
    }

    /// Render skeleton placeholders while experiment results load.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Render an empty state when no variants are configured.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Whether the comparison body and footer tray are currently unfolded.
    pub fn is_open(&self) -> bool {
        self.pinned_open || (self.trigger == ExperimentTrigger::Hover && self.hovered_card)
    }

    /// Return the currently selected metric id.
    pub fn active_metric(&self) -> &str {
        self.active_metric.as_ref()
    }

    /// Return the currently picked winner variant id, if any.
    pub fn winner(&self) -> Option<&str> {
        self.winner.as_deref()
    }

    /// Return the confirmed/shipped winner variant id, if any.
    pub fn confirmed_winner(&self) -> Option<&str> {
        self.confirmed_winner.as_deref()
    }

    fn current_metric(&self) -> Option<&ExperimentMetric> {
        self.metrics
            .iter()
            .find(|m| m.id == self.active_metric)
            .or_else(|| self.metrics.first())
    }

    fn control_index(&self) -> usize {
        self.variants.iter().position(|v| v.control).unwrap_or(0)
    }

    /// Compute `(pct_delta, is_improvement)` for `variant_idx` against the baseline control on the active metric.
    pub fn variant_delta(&self, variant_idx: usize) -> Option<(f64, bool)> {
        let metric = self.current_metric()?;
        let ctrl_idx = self.control_index();
        if variant_idx == ctrl_idx {
            return None;
        }
        let base = self.variants.get(ctrl_idx)?.metric_value(&metric.id);
        if base.abs() <= 1e-9 {
            return None;
        }
        let val = self.variants.get(variant_idx)?.metric_value(&metric.id);
        let pct = ((val - base) / base.abs()) * 100.0;
        let good = if metric.lower_is_better {
            pct <= 0.0
        } else {
            pct >= 0.0
        };
        Some((pct, good))
    }

    /// Return the non-control variant with the strongest improvement on the active metric, along with its `%` delta.
    pub fn leading_variant(&self) -> Option<(&ExperimentVariant, f64, bool)> {
        let metric = self.current_metric()?;
        let ctrl_idx = self.control_index();
        let mut best: Option<(usize, f64, f64)> = None;
        for (idx, _) in self.variants.iter().enumerate() {
            if idx == ctrl_idx {
                continue;
            }
            if let Some((pct, _)) = self.variant_delta(idx) {
                let score = if metric.lower_is_better { -pct } else { pct };
                if best.is_none_or(|(_, best_score, _)| score > best_score) {
                    best = Some((idx, score, pct));
                }
            }
        }
        let (idx, score, pct) = best?;
        Some((&self.variants[idx], pct, score >= 0.0))
    }

    fn set_metric(&mut self, metric_id: SharedString, cx: &mut Context<Self>) {
        if self.active_metric != metric_id {
            self.active_metric = metric_id.clone();
            cx.emit(ExperimentCardEvent::MetricChanged(metric_id));
            cx.notify();
        }
    }

    fn step_metric(&mut self, delta: isize, cx: &mut Context<Self>) {
        if self.metrics.len() < 2 {
            return;
        }
        let cur = self
            .metrics
            .iter()
            .position(|m| m.id == self.active_metric)
            .unwrap_or(0) as isize;
        let len = self.metrics.len() as isize;
        let next = (cur + delta).rem_euclid(len) as usize;
        self.set_metric(self.metrics[next].id.clone(), cx);
    }

    fn toggle_pick_winner(&mut self, variant_id: SharedString, cx: &mut Context<Self>) {
        if self.winner.as_ref() == Some(&variant_id) {
            self.winner = None;
            self.confirmed_winner = None;
        } else {
            self.winner = Some(variant_id);
            self.confirmed_winner = None;
        }
        cx.emit(ExperimentCardEvent::WinnerChanged(self.winner.clone()));
        cx.notify();
    }

    fn confirm_winner(&mut self, cx: &mut Context<Self>) {
        let Some(wid) = self.winner.clone() else {
            return;
        };
        let key = self
            .variants
            .iter()
            .find(|v| v.id == wid)
            .map(|v| v.key.clone())
            .unwrap_or_else(|| wid.clone());
        self.confirmed_winner = Some(wid.clone());
        self.status = ExperimentStatus::new(
            format!("{} {key}", tr(cx, "Winner", "已选胜出")),
            ExperimentTone::Blue,
        );
        self.toast = Some((
            format!("{} {key}", tr(cx, "Picked winner", "已确认胜出方案")).into(),
            Instant::now(),
        ));
        cx.emit(ExperimentCardEvent::WinnerConfirmed(wid));
        cx.notify();
    }

    fn handle_key_down(&mut self, ev: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        match ev.keystroke.key.as_str() {
            "enter" | "space" => {
                let was_open = self.is_open();
                self.pinned_open = !was_open;
                if !self.pinned_open {
                    self.hovered_card = false;
                }
                cx.emit(ExperimentCardEvent::OpenChanged(self.is_open()));
                cx.notify();
            }
            "escape" if self.is_open() => {
                self.pinned_open = false;
                self.hovered_card = false;
                cx.emit(ExperimentCardEvent::OpenChanged(false));
                cx.notify();
            }
            "left" if self.is_open() => self.step_metric(-1, cx),
            "right" if self.is_open() => self.step_metric(1, cx),
            _ => {}
        }
    }
}

/// Paint a horizontal rounded-capsule bar using [`PathBuilder`] inside `bounds`.
fn paint_rounded_bar_path(
    bounds: Bounds<Pixels>,
    ratio: f32,
    track: Hsla,
    fill_color: Hsla,
    window: &mut Window,
) {
    let w = f32::from(bounds.size.width);
    let h = f32::from(bounds.size.height);
    if w <= 1.0 || h <= 0.5 {
        return;
    }
    let y = f32::from(bounds.center().y);
    let r = h * 0.5;
    let mut track_path = PathBuilder::stroke(px(h));
    track_path.move_to(point(bounds.left() + px(r), px(y)));
    track_path.line_to(point(bounds.right() - px(r), px(y)));
    if let Ok(p) = track_path.build() {
        window.paint_path(p, track);
    }
    let fw = (w * ratio.clamp(0., 1.)).max(h);
    if ratio > 0.005 && fw >= h {
        let mut bar_path = PathBuilder::stroke(px(h));
        bar_path.move_to(point(bounds.left() + px(r), px(y)));
        bar_path.line_to(point(bounds.left() + px((fw - r).max(r + 0.1)), px(y)));
        if let Ok(p) = bar_path.build() {
            window.paint_path(p, fill_color);
        }
    }
}

impl Render for ExperimentCard {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let dark = p.dark;
        let card_bg: Hsla = if dark {
            rgb(0x1c1c1e).into()
        } else {
            rgb(0xffffff).into()
        };
        let card_border: Hsla = if dark {
            hsla(0., 0., 1., 0.10)
        } else {
            hsla(0., 0., 0., 0.065)
        };
        let line: Hsla = if dark {
            hsla(0., 0., 1., 0.08)
        } else {
            hsla(0., 0., 0., 0.06)
        };
        let wash: Hsla = if dark {
            hsla(0., 0., 1., 0.05)
        } else {
            hsla(0., 0., 0., 0.035)
        };
        let track_c: Hsla = if dark {
            hsla(0., 0., 1., 0.08)
        } else {
            hsla(0., 0., 0., 0.06)
        };
        let pick_bg: Hsla = if dark {
            hsla(218. / 360., 1.0, 0.65, 0.14)
        } else {
            hsla(215. / 360., 1.0, 0.44, 0.065)
        };
        let pick_border: Hsla = if dark {
            hsla(218. / 360., 1.0, 0.65, 0.42)
        } else {
            hsla(215. / 360., 1.0, 0.44, 0.36)
        };

        if self.loading {
            let phase = if disabled_motion(cx) {
                0.25
            } else {
                w.request_animation_frame();
                (self.started_at.elapsed().as_secs_f32() / 1.4).fract()
            };
            return div()
                .id(pro_eid(&self.id, "loading"))
                .w_full()
                .max_w(px(340.))
                .rounded(px(10.))
                .bg(card_bg)
                .border_1()
                .border_color(card_border)
                .p(px(12.))
                .flex()
                .items_center()
                .gap(px(10.))
                .child(pro_skeleton_bar(36., 36., 8., phase, cx))
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .flex_col()
                        .gap(px(6.))
                        .child(pro_skeleton_bar(132., 14., 4., phase, cx))
                        .child(pro_skeleton_bar(94., 16., 5., phase, cx)),
                )
                .child(pro_skeleton_bar(68., 24., 6., phase, cx))
                .into_any_element();
        }

        if self.empty || self.variants.is_empty() {
            return div()
                .id(self.id.clone())
                .w_full()
                .max_w(px(340.))
                .rounded(px(10.))
                .bg(card_bg)
                .border_1()
                .border_color(card_border)
                .child(pro_empty_state::<Self>(
                    ProIcon::Activity,
                    tr(cx, "No active variants", "暂无实验分组"),
                    tr(
                        cx,
                        "Add at least two variants to compare conversion and confidence.",
                        "添加至少两个实验变体以对比转化率与置信度。",
                    ),
                    None,
                    cx,
                ))
                .into_any_element();
        }

        let open = self.is_open();
        let metric = self
            .current_metric()
            .cloned()
            .unwrap_or_else(|| ExperimentMetric::new("value", "Value"));
        let max_val = self
            .variants
            .iter()
            .map(|v| v.metric_value(&metric.id).abs())
            .fold(0.0f64, f64::max)
            .max(1e-6);

        let n_vars = self.variants.len();
        if self.variant_spotlights.len() < n_vars {
            self.variant_spotlights
                .resize_with(n_vars, || (Slide::snappy(1.0), Slide::snappy(0.0)));
        } else if self.variant_spotlights.len() > n_vars {
            self.variant_spotlights.truncate(n_vars);
        }

        let mut head_ratios = Vec::with_capacity(n_vars);
        let mut head_colors = Vec::with_capacity(n_vars);
        let mut head_hot_t = Vec::with_capacity(n_vars);
        let mut head_alphas = Vec::with_capacity(n_vars);

        for (i, v) in self.variants.iter().enumerate() {
            let raw_ratio = (v.metric_value(&metric.id).abs() / max_val) as f32;
            let visual_target = (0.28 + 0.72 * raw_ratio).clamp(0.22, 1.0);
            let spring_h = pro_spring_f32(
                pro_eid(&self.id, format!("hcol-{i}")),
                visual_target,
                0.38,
                w,
                cx,
            );
            let col_c = v.color.unwrap_or_else(|| default_variant_color(i, dark));
            let is_hot = self.hovered_variant.as_ref() == Some(&v.id);
            let any_hover = self.hovered_variant.is_some();
            let passed_winner = self
                .winner
                .as_ref()
                .is_some_and(|wid| wid != &v.id && !is_hot);
            let dim_target = if (any_hover && !is_hot) || passed_winner {
                0.32
            } else {
                1.0
            };
            let hot_target = if is_hot { 1.0 } else { 0.0 };
            self.variant_spotlights[i].0.set(dim_target, cx);
            self.variant_spotlights[i].1.set(hot_target, cx);

            head_ratios.push(spring_h);
            head_colors.push(col_c);
            head_alphas.push(self.variant_spotlights[i].0.get(w, cx).clamp(0.25, 1.0));
            head_hot_t.push(self.variant_spotlights[i].1.get(w, cx).clamp(0.0, 1.0));
        }

        let paint_alphas = head_alphas.clone();
        let mini_chart = div()
            .size(px(36.))
            .flex_none()
            .rounded(px(8.))
            .bg(wash)
            .border_1()
            .border_color(line)
            .p(px(6.))
            .child(
                canvas(
                    |_, _, _| (),
                    move |bounds, _, window, _| {
                        let n = head_ratios.len();
                        if n == 0 {
                            return;
                        }
                        let bw = f32::from(bounds.size.width);
                        let bh = f32::from(bounds.size.height);
                        let gap = if n > 3 { 2.0 } else { 3.0 };
                        let col_w =
                            ((bw - gap * (n.saturating_sub(1) as f32)) / n as f32).clamp(2.5, 6.0);
                        let total_w = col_w * n as f32 + gap * (n.saturating_sub(1) as f32);
                        let start_x = f32::from(bounds.left()) + (bw - total_w) * 0.5;
                        let bot_y = f32::from(bounds.bottom());

                        for i in 0..n {
                            let h_px = (bh * head_ratios[i]).max(3.0);
                            let w_eff = col_w * (1.0 + 0.24 * head_hot_t[i]);
                            let cx_x = start_x + i as f32 * (col_w + gap) + col_w * 0.5;
                            let x0 = cx_x - w_eff * 0.5;
                            let x1 = cx_x + w_eff * 0.5;
                            let y0 = bot_y - h_px;
                            let r = 1.5f32.min(w_eff * 0.5).min(h_px * 0.5);
                            let alpha = paint_alphas[i];

                            let mut bar = PathBuilder::fill();
                            bar.move_to(point(px(x0), px(bot_y)));
                            bar.line_to(point(px(x0), px(y0 + r)));
                            bar.curve_to(point(px(x0 + r), px(y0)), point(px(x0), px(y0)));
                            bar.line_to(point(px(x1 - r), px(y0)));
                            bar.curve_to(point(px(x1), px(y0 + r)), point(px(x1), px(y0)));
                            bar.line_to(point(px(x1), px(bot_y)));
                            bar.close();
                            if let Ok(path) = bar.build() {
                                window.paint_path(path, head_colors[i].opacity(alpha));
                            }
                        }
                    },
                )
                .size_full(),
            );

        let (st_bg, st_fg) = self.status.tone.colors(dark);
        let status_pill = div()
            .h(px(20.))
            .pl(px(5.))
            .pr(px(6.))
            .rounded(px(5.))
            .bg(st_bg)
            .text_color(st_fg)
            .text_size(px(12.))
            .font_weight(FontWeight::MEDIUM)
            .flex()
            .items_center()
            .gap(px(5.))
            .child(pro_svg("0 0 10 10", LIVE_DOT_SVG, 10., st_fg))
            .child(pro_swap_label(
                pro_eid(&self.id, "status-lbl"),
                self.status.label.clone(),
                12.,
                FontWeight::MEDIUM,
                st_fg,
            ));

        let lead_badge: Option<AnyElement> = if let Some(wid) = &self.winner
            && let Some(idx) = self.variants.iter().position(|v| &v.id == wid)
        {
            let v = &self.variants[idx];
            let (lbg, lfg) = ExperimentTone::Blue.colors(dark);
            let delta_txt: SharedString = self
                .variant_delta(idx)
                .map(|(pct, _)| {
                    let sign = if pct >= 0.0 { "+" } else { "\u{2212}" };
                    format!("{} {sign}{:.1}%", v.key, pct.abs()).into()
                })
                .unwrap_or_else(|| format!("{} {}", v.key, tr(cx, "Picked", "已选")).into());
            Some(
                div()
                    .h(px(24.))
                    .px(px(7.))
                    .flex_none()
                    .rounded(px(6.))
                    .bg(lbg)
                    .text_color(lfg)
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .child(pro_check_draw(
                        pro_eid(&self.id, "head-win-chk"),
                        true,
                        13.,
                        false,
                        lfg,
                    ))
                    .child(pro_swap_label(
                        pro_eid(&self.id, "head-win-lbl"),
                        delta_txt,
                        12.,
                        FontWeight::SEMIBOLD,
                        lfg,
                    ))
                    .into_any_element(),
            )
        } else if let Some((lead_v, pct, good)) = self.leading_variant() {
            let tone = if good {
                ExperimentTone::Green
            } else {
                ExperimentTone::Red
            };
            let (lbg, lfg) = tone.colors(dark);
            let arrow_svg = if pct >= 0.0 {
                TREND_LEAD_SVG
            } else {
                TREND_DROP_SVG
            };
            let sign = if pct >= 0.0 { "+" } else { "\u{2212}" };
            Some(
                div()
                    .h(px(24.))
                    .px(px(7.))
                    .flex_none()
                    .rounded(px(6.))
                    .bg(lbg)
                    .text_color(lfg)
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(lead_v.key.clone()),
                    )
                    .child(pro_svg("0 0 12 12", arrow_svg, 11., lfg))
                    .child(pro_rolling_number(
                        pro_eid(&self.id, "lead-num"),
                        pct.abs(),
                        1,
                        sign,
                        "%",
                        12.,
                        FontWeight::MEDIUM,
                        lfg,
                    ))
                    .into_any_element(),
            )
        } else {
            None
        };

        let head_row = div()
            .id(pro_eid(&self.id, "head-btn"))
            .role(Role::Button)
            .aria_label(self.title.clone())
            .w_full()
            .p(px(12.))
            .flex()
            .items_center()
            .gap(px(10.))
            .cursor_pointer()
            .on_click(cx.listener(|this, _, _, cx| {
                let next = !this.pinned_open;
                this.pinned_open = next;
                if !next {
                    this.hovered_card = false;
                }
                cx.emit(ExperimentCardEvent::OpenChanged(this.is_open()));
                cx.notify();
            }))
            .child(mini_chart)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_size(px(13.))
                            .line_height(px(20.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .truncate()
                            .child(self.title.clone()),
                    )
                    .child(
                        div()
                            .h(px(20.))
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .min_w_0()
                            .child(status_pill)
                            .when_some(self.note.clone(), |d, note| {
                                d.child(
                                    div()
                                        .text_size(px(12.))
                                        .line_height(px(16.))
                                        .text_color(p.muted)
                                        .truncate()
                                        .child(note),
                                )
                            }),
                    ),
            )
            .when_some(lead_badge, |d, lb| d.child(lb));

        // Pre-sample variant row bar springs (`pro_spring_f32`) so animations stay silky across metric switches.
        let mut row_bar_ratios = Vec::with_capacity(n_vars);
        for (i, v) in self.variants.iter().enumerate() {
            let ratio = ((v.metric_value(&metric.id).abs() / max_val) as f32).clamp(0.06, 1.0);
            let sp = pro_spring_f32(pro_eid(&self.id, format!("rbar-{i}")), ratio, 0.36, w, cx);
            row_bar_ratios.push(sp);
        }

        let ctrl_idx = self.control_index();
        let deltas: Vec<Option<(f64, bool)>> = (0..n_vars).map(|i| self.variant_delta(i)).collect();

        let caption_str: SharedString = if let Some(hvid) = &self.hovered_variant
            && let Some(hv) = self.variants.iter().find(|v| &v.id == hvid)
        {
            let vis_part = hv
                .visitors
                .map(|n| format!("{} {}", format_visitors(n), tr(cx, "visitors", "访客")))
                .unwrap_or_default();
            if hv.control {
                if vis_part.is_empty() {
                    tr(cx, "Control baseline", "对照组基准").into()
                } else {
                    format!("{vis_part} \u{00b7} {}", tr(cx, "Control", "对照组")).into()
                }
            } else if let Some(&conf) = hv.confidence.get(&metric.id) {
                if vis_part.is_empty() {
                    format!("{conf}% {}", tr(cx, "confidence", "置信度")).into()
                } else {
                    format!(
                        "{conf}% {} \u{00b7} {vis_part}",
                        tr(cx, "confidence", "置信度")
                    )
                    .into()
                }
            } else if !vis_part.is_empty() {
                vis_part.into()
            } else {
                tr(cx, "Click to pick winner", "点击选择胜出方案").into()
            }
        } else if let Some(wid) = &self.winner
            && let Some(wv) = self.variants.iter().find(|v| &v.id == wid)
        {
            if let Some(&conf) = wv.confidence.get(&metric.id) {
                format!(
                    "{} {} \u{00b7} {conf}% {}",
                    tr(cx, "Selected", "已选"),
                    wv.key,
                    tr(cx, "confidence", "置信度")
                )
                .into()
            } else {
                format!("{} {}", tr(cx, "Selected variant", "已选方案"), wv.key).into()
            }
        } else if let Some(custom_hint) = &self.hint {
            custom_hint.clone()
        } else {
            tr(cx, "Pick a winner", "点击方案选择胜出者").into()
        };

        // Build liquid `pro_pill_indicator` metric switcher (`Conversion / Revenue / Retention`).
        let pill_track_bg: Hsla = if dark {
            hsla(0., 0., 1., 0.045)
        } else {
            hsla(0., 0., 0., 0.035)
        };
        let pill_fill: Hsla = if dark {
            rgb(0x28282c).into()
        } else {
            rgb(0xffffff).into()
        };
        let pill_border: Hsla = if dark {
            hsla(0., 0., 1., 0.11)
        } else {
            hsla(0., 0., 0., 0.08)
        };

        let metric_switch: Option<AnyElement> = (self.metrics.len() > 1).then(|| {
            let sensor = pro_selection_sensor();
            let mut strip = div()
                .w_full()
                .p(px(2.))
                .rounded(px(7.))
                .bg(pill_track_bg)
                .border_1()
                .border_color(line)
                .relative()
                .flex()
                .items_center()
                .gap(px(2.))
                .child(pro_pill_indicator(
                    pro_eid(&self.id, "msw"),
                    metric.id.clone(),
                    sensor.clone(),
                    5.5,
                    pill_fill,
                    Some(pill_border),
                    !dark,
                ));
            for m in &self.metrics {
                let mid = m.id.clone();
                let selected = mid == metric.id;
                strip = strip.child(
                    div()
                        .id(pro_eid(&self.id, format!("m-{mid}")))
                        .role(Role::RadioButton)
                        .aria_label(m.label.clone())
                        .flex_1()
                        .h(px(26.))
                        .px(px(8.))
                        .relative()
                        .rounded(px(5.5))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(12.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(if selected { p.ink } else { p.text })
                        .cursor_pointer()
                        .when(!selected, |d| d.hover(move |s| s.text_color(p.ink)))
                        .when(selected, |d| d.child(pro_mark_selected(&sensor)))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.set_metric(mid.clone(), cx);
                        }))
                        .child(m.label.clone()),
                );
            }
            strip.into_any_element()
        });

        // Build tactile `pro_tactile` variant rows with `pro_rolling_number` & `pro_check_draw`.
        let mut rows_col = div()
            .w_full()
            .mt(px(if metric_switch.is_some() { 8. } else { 0. }))
            .flex()
            .flex_col()
            .gap(px(3.));

        for i in 0..n_vars {
            let v = self.variants[i].clone();
            let vid = v.id.clone();
            let vid_hov = vid.clone();
            let vid_click = vid.clone();
            let col_c = v.color.unwrap_or_else(|| default_variant_color(i, dark));
            let picked = self.winner.as_ref() == Some(&vid);
            let row_alpha = if picked {
                1.0
            } else {
                (0.45 + 0.55 * head_alphas[i]).clamp(0.45, 1.0)
            };
            let val = v.metric_value(&metric.id);
            let bar_ratio = row_bar_ratios[i];

            let badge_bg = col_c.opacity(if dark { 0.18 } else { 0.14 });
            let badge_fg: Hsla = if i == 0 { p.ink } else { col_c };

            let lift_pill = if i == ctrl_idx {
                let (gbg, gfg) = ExperimentTone::Gray.colors(dark);
                div()
                    .min_w(px(52.))
                    .h(px(20.))
                    .px(px(6.))
                    .rounded(px(5.))
                    .bg(gbg)
                    .text_color(gfg)
                    .text_size(px(11.))
                    .font_weight(FontWeight::MEDIUM)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child("Base")
                    .into_any_element()
            } else if let Some((pct, good)) = deltas[i] {
                let tone = if good {
                    ExperimentTone::Green
                } else {
                    ExperimentTone::Red
                };
                let (lbg, lfg) = tone.colors(dark);
                let sign = if pct >= 0.0 { "+" } else { "\u{2212}" };
                div()
                    .min_w(px(52.))
                    .h(px(20.))
                    .px(px(6.))
                    .rounded(px(5.))
                    .bg(lbg)
                    .text_color(lfg)
                    .text_size(px(11.))
                    .font_weight(FontWeight::MEDIUM)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(pro_rolling_number(
                        pro_eid(&self.id, format!("lift-{i}")),
                        pct.abs(),
                        1,
                        sign,
                        "%",
                        11.,
                        FontWeight::MEDIUM,
                        lfg,
                    ))
                    .into_any_element()
            } else {
                div().min_w(px(52.)).into_any_element()
            };

            let lead_mark: AnyElement = if picked {
                pro_check_draw(
                    pro_eid(&self.id, format!("vchk-{vid}")),
                    true,
                    20.,
                    true,
                    if i == 0 { p.blue } else { col_c },
                )
            } else {
                div()
                    .min_w(px(20.))
                    .h(px(20.))
                    .px(px(4.))
                    .flex_none()
                    .rounded(px(5.))
                    .bg(badge_bg)
                    .text_color(badge_fg)
                    .text_size(px(11.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(v.key.clone())
                    .into_any_element()
            };

            let row_content = div()
                .w_full()
                .px(px(6.))
                .py(px(7.))
                .flex()
                .flex_col()
                .gap(px(6.))
                .child(
                    div()
                        .w_full()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(lead_mark)
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .text_size(px(13.))
                                .font_weight(if picked {
                                    FontWeight::MEDIUM
                                } else {
                                    FontWeight::NORMAL
                                })
                                .text_color(if picked { p.strong } else { p.ink })
                                .truncate()
                                .child(v.name.clone()),
                        )
                        .child(pro_rolling_number(
                            pro_eid(&self.id, format!("vnum-{i}")),
                            val,
                            metric.decimals,
                            metric.prefix.clone(),
                            metric.suffix.clone(),
                            13.,
                            FontWeight::MEDIUM,
                            p.strong,
                        ))
                        .child(lift_pill),
                )
                .child(
                    div().w_full().pl(px(28.)).child(
                        canvas(
                            |_, _, _| (),
                            move |bounds, _, window, _| {
                                paint_rounded_bar_path(bounds, bar_ratio, track_c, col_c, window);
                            },
                        )
                        .w_full()
                        .flex_none()
                        .h(px(3.5)),
                    ),
                );

            let rest_bg = if picked { pick_bg } else { transparent_black() };
            let hover_bg = if picked { pick_bg } else { wash };
            let rest_border = if picked {
                pick_border
            } else {
                transparent_black()
            };
            let hover_border = if picked { pick_border } else { line };

            rows_col = rows_col.child(
                pro_tactile(
                    pro_eid(&self.id, format!("vrow-{vid}")),
                    v.name.clone(),
                    6.,
                    rest_bg,
                    hover_bg,
                    rest_border,
                    hover_border,
                    picked,
                    row_content,
                    cx,
                    move |this, _, cx| {
                        this.toggle_pick_winner(vid_click.clone(), cx);
                    },
                )
                .w_full()
                .opacity(row_alpha)
                .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                    if *inside {
                        this.hovered_variant = Some(vid_hov.clone());
                    } else if this.hovered_variant.as_ref() == Some(&vid_hov) {
                        this.hovered_variant = None;
                    }
                    cx.notify();
                })),
            );
        }

        // Winner confirmation button (`pro_popover_presence` + `pro_tactile` + `pro_swap_label` + `pro_check_draw`).
        let has_winner = self.winner.is_some();
        let winner_btn_inner: Option<AnyElement> = self.winner.as_ref().and_then(|wid| {
            let wv = self.variants.iter().find(|v| &v.id == wid)?;
            let is_confirmed = self.confirmed_winner.as_ref() == Some(wid);
            let label_txt: SharedString = if is_confirmed {
                format!("{} {}", tr(cx, "Confirmed", "已上线"), wv.key).into()
            } else {
                format!("{} {}", self.action_prefix, wv.key).into()
            };
            let btn = pro_tactile(
                pro_eid(&self.id, "confirm-winner"),
                label_txt.clone(),
                6.,
                p.primary,
                pro_chrome_hover_primary(&p),
                transparent_black(),
                transparent_black(),
                is_confirmed,
                div()
                    .h(px(28.))
                    .pl(px(10.))
                    .pr(px(8.))
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .whitespace_nowrap()
                    .child(pro_swap_label(
                        pro_eid(&self.id, "confirm-lbl"),
                        label_txt,
                        12.5,
                        FontWeight::MEDIUM,
                        p.on_primary,
                    ))
                    .child(if is_confirmed {
                        pro_check_draw(pro_eid(&self.id, "confirm-chk"), true, 14., false, p.green)
                    } else {
                        pro_svg("0 0 12 12", ARROW_SHIP_SVG, 11., p.on_primary)
                    }),
                cx,
                |this, _, cx| {
                    this.confirm_winner(cx);
                },
            )
            .flex_none()
            .into_any_element();
            Some(btn)
        });

        let action_presence =
            pro_popover_presence(pro_eid(&self.id, "confirm-pres"), has_winner, move || {
                winner_btn_inner.unwrap_or_else(|| div().into_any_element())
            });

        let unfolded_body = div()
            .w_full()
            .flex()
            .flex_col()
            .child(
                div()
                    .w_full()
                    .px(px(12.))
                    .pb(px(10.))
                    .flex()
                    .flex_col()
                    .when_some(metric_switch, |d, sw| d.child(sw))
                    .child(rows_col),
            )
            .child(
                div()
                    .w_full()
                    .h(px(44.))
                    .pl(px(12.))
                    .pr(px(8.))
                    .border_t_1()
                    .border_color(line)
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .overflow_hidden()
                            .child(pro_swap_label(
                                pro_eid(&self.id, "foot-cap"),
                                caption_str,
                                12.,
                                FontWeight::NORMAL,
                                p.muted,
                            )),
                    )
                    .child(action_presence),
            );

        let unfolded = pro_fold(pro_eid(&self.id, "fold"), open, 236., 0., move || {
            unfolded_body.into_any_element()
        });

        div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .w_full()
            .max_w(px(340.))
            .relative()
            .rounded(px(10.))
            .bg(card_bg)
            .border_1()
            .border_color(card_border)
            .when(!dark, |d| d.shadow_xs())
            .on_key_down(cx.listener(Self::handle_key_down))
            .on_hover(cx.listener(|this, inside: &bool, _, cx| {
                if this.trigger == ExperimentTrigger::Hover && this.hovered_card != *inside {
                    let prev = this.is_open();
                    this.hovered_card = *inside;
                    if !*inside {
                        this.hovered_variant = None;
                    }
                    let next = this.is_open();
                    if prev != next {
                        cx.emit(ExperimentCardEvent::OpenChanged(next));
                    }
                    cx.notify();
                }
            }))
            .child(head_row)
            .child(unfolded)
            .child(pro_toast_banner(self.toast.clone()))
            .into_any_element()
    }
}

fn pro_chrome_hover_primary(p: &ProPalette) -> Hsla {
    crate::components::pro_chrome::pro_mix_color(p.primary, p.on_primary, 0.12)
}

#[cfg(test)]
mod tests {
    use super::{ExperimentCard, ExperimentMetric, ExperimentVariant};
    use gpui::{AppContext, TestAppContext, px, size};
    use std::time::Duration;

    #[gpui::test]
    fn computes_deltas_and_confirms_winner(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let card = cx.new(|cx| ExperimentCard::new("xc-test", cx));
        card.update(cx, |c, cx| {
            let (lead, pct, good) = c.leading_variant().expect("has lead");
            assert_eq!(lead.id.as_ref(), "c");
            assert!(good);
            assert!((pct - 21.428).abs() < 0.1);

            c.set_metric("revenue".into(), cx);
            let (_, rev_pct, _) = c.leading_variant().expect("revenue lead");
            assert!((rev_pct - 17.39).abs() < 0.1);

            c.toggle_pick_winner("b".into(), cx);
            assert_eq!(c.winner(), Some("b"));
            c.confirm_winner(cx);
            assert_eq!(c.confirmed_winner(), Some("b"));
        });
    }

    #[gpui::test]
    fn honors_lower_is_better_metrics(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let card = cx.new(|cx| {
            ExperimentCard::new("xc-bounce", cx)
                .with_metrics(vec![
                    ExperimentMetric::new("bounce", "Bounce")
                        .with_format("", 1, "%")
                        .with_lower_is_better(true),
                ])
                .with_default_metric("bounce")
                .with_variants(vec![
                    ExperimentVariant::new("a", "A", "Control")
                        .with_control(true)
                        .with_value("bounce", 40.0),
                    ExperimentVariant::new("b", "B", "Variant B").with_value("bounce", 37.0),
                ])
        });
        card.update(cx, |c, _| {
            let (pct, good) = c.variant_delta(1).expect("delta");
            assert!((pct - -7.5).abs() < 0.01);
            assert!(good);
        });
    }

    #[gpui::test]
    fn unfolded_card_animates_winner_spotlight_and_metric_switch(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let exec = cx.executor();
        let (card, w) =
            cx.add_window_view(|_, cx| ExperimentCard::new("xc-open", cx).with_default_open(true));
        w.simulate_resize(size(px(360.), px(420.)));
        w.run_until_parked();

        w.update(|_, app| {
            card.update(app, |c, cx| {
                c.set_metric("retention".into(), cx);
                c.toggle_pick_winner("c".into(), cx);
            });
        });
        w.run_until_parked();

        exec.advance_clock(Duration::from_millis(60));
        w.update(|win, app| win.simulate_next_frame(app));
        w.run_until_parked();

        w.update(|_, app| {
            let c = card.read(app);
            assert_eq!(c.winner(), Some("c"));
            assert!(c.variant_spotlights[2].0.value(app) > c.variant_spotlights[0].0.value(app));
        });
    }
}
