//! Interactive stacked breakdown bar chart with measure/slice/segment pickers and linked table.
//!
//! Aggregates flat records across user-chosen `Measure`, `Slice`, and `Segment` dimensions,
//! animates stacked bar heights via `Slide` springs, displays a pointer hover tooltip, and
//! links bar segments bidirectionally with the breakdown matrix and toggleable legend below.

use crate::{
    components::pro_chrome::{
        ProPalette, pro_card_shadow, pro_check_draw, pro_eid, pro_empty_state, pro_mark_selected,
        pro_pill_indicator, pro_popover_presence, pro_rolling_number, pro_selection_sensor,
        pro_skeleton_bar, pro_swap_label, pro_tactile, tr,
    },
    primitives::{
        control::activate,
        family::{Slide, disabled_motion},
        pro_icons::{ProIcon, pro_icon},
    },
};
use gpui::{prelude::*, *};
use std::{
    cell::Cell,
    collections::{HashMap, HashSet},
    rc::Rc,
};

/// One flat record aggregated by [`BreakdownChart`].
#[derive(Clone, Debug, PartialEq)]
pub struct BreakdownRecord {
    /// Categorical dimension values keyed by dimension key (e.g. `"status" -> "Done"`).
    pub fields: HashMap<SharedString, SharedString>,
    /// Numeric metric values keyed by measure field (e.g. `"points" -> 5.0`).
    pub metrics: HashMap<SharedString, f64>,
}

impl BreakdownRecord {
    /// Construct an empty [`BreakdownRecord`].
    pub fn new() -> Self {
        Self {
            fields: HashMap::new(),
            metrics: HashMap::new(),
        }
    }

    /// Set a categorical dimension value.
    pub fn with_field(
        mut self,
        key: impl Into<SharedString>,
        value: impl Into<SharedString>,
    ) -> Self {
        self.fields.insert(key.into(), value.into());
        self
    }

    /// Set a numeric metric value.
    pub fn with_metric(mut self, key: impl Into<SharedString>, value: f64) -> Self {
        self.metrics.insert(key.into(), value);
        self
    }
}

impl Default for BreakdownRecord {
    fn default() -> Self {
        Self::new()
    }
}

/// Categorical dimension definition (`Status`, `Priority`, `Team`, etc.).
#[derive(Clone, Debug, PartialEq)]
pub struct BreakdownDimension {
    /// Dimension key (e.g. `"status"`).
    pub key: SharedString,
    /// English display label.
    pub label: SharedString,
    /// Localized Chinese label.
    pub label_zh: SharedString,
    /// Ordered categorical values along this dimension.
    pub values: Vec<SharedString>,
    /// Optional explicit color overrides keyed by value name.
    pub colors: HashMap<SharedString, Hsla>,
}

impl BreakdownDimension {
    /// Create a [`BreakdownDimension`].
    pub fn new(
        key: impl Into<SharedString>,
        label: impl Into<SharedString>,
        values: impl IntoIterator<Item = impl Into<SharedString>>,
    ) -> Self {
        let label = label.into();
        Self {
            key: key.into(),
            label_zh: label.clone(),
            label,
            values: values.into_iter().map(Into::into).collect(),
            colors: HashMap::new(),
        }
    }

    /// Set localized Chinese label.
    pub fn with_label_zh(mut self, zh: impl Into<SharedString>) -> Self {
        self.label_zh = zh.into();
        self
    }

    /// Assign a custom [`Hsla`] color for one categorical value.
    pub fn with_color(mut self, value: impl Into<SharedString>, color: Hsla) -> Self {
        self.colors.insert(value.into(), color);
        self
    }
}

/// Numeric measure definition (`Task count`, `Story points`, etc.).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BreakdownMeasure {
    /// Measure key (e.g. `"count"`, `"points"`).
    pub key: SharedString,
    /// English display label.
    pub label: SharedString,
    /// Localized Chinese label.
    pub label_zh: SharedString,
    /// Unit suffix (`"tasks"`, `"pts"`, `"hrs"`).
    pub unit: SharedString,
    /// Localized Chinese unit suffix.
    pub unit_zh: SharedString,
    /// Record metric key to sum (`None` counts `1.0` per record).
    pub field: Option<SharedString>,
}

impl BreakdownMeasure {
    /// Create a record-counting [`BreakdownMeasure`].
    pub fn count(
        key: impl Into<SharedString>,
        label: impl Into<SharedString>,
        unit: impl Into<SharedString>,
    ) -> Self {
        let label = label.into();
        let unit = unit.into();
        Self {
            key: key.into(),
            label_zh: label.clone(),
            label,
            unit_zh: unit.clone(),
            unit,
            field: None,
        }
    }

    /// Create a field-summing [`BreakdownMeasure`].
    pub fn sum(
        key: impl Into<SharedString>,
        label: impl Into<SharedString>,
        unit: impl Into<SharedString>,
        field: impl Into<SharedString>,
    ) -> Self {
        let label = label.into();
        let unit = unit.into();
        Self {
            key: key.into(),
            label_zh: label.clone(),
            label,
            unit_zh: unit.clone(),
            unit,
            field: Some(field.into()),
        }
    }

    /// Set localized Chinese label and unit.
    pub fn with_zh(
        mut self,
        label_zh: impl Into<SharedString>,
        unit_zh: impl Into<SharedString>,
    ) -> Self {
        self.label_zh = label_zh.into();
        self.unit_zh = unit_zh.into();
        self
    }
}

/// Row orientation of the linked breakdown table below the chart.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BreakdownTableMode {
    /// Rows represent stacked segments (`Urgent`, `High`, ...); hovering highlights that segment across all bars.
    #[default]
    BySegment,
    /// Rows represent x-axis slices (`Backlog`, `Todo`, ...); hovering highlights that bar column.
    BySlice,
}

impl BreakdownTableMode {
    fn key(self) -> &'static str {
        match self {
            Self::BySegment => "by-segment",
            Self::BySlice => "by-slice",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ActivePicker {
    Measure,
    Slice,
    Segment,
}

/// Typed events emitted by [`BreakdownChart`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BreakdownChartEvent {
    /// Active measure changed.
    MeasureChanged(SharedString),
    /// Active x-axis slice dimension changed.
    SliceChanged(SharedString),
    /// Active stack segment dimension changed (`"none"` when unstacked).
    SegmentChanged(SharedString),
    /// A segment legend item was toggled on or off.
    SegmentVisibilityToggled {
        segment: SharedString,
        visible: bool,
    },
    /// User clicked a slice bar or table row.
    SliceFocused(SharedString),
}

/// Interactive stacked breakdown chart with linked cross-highlighting table and legend.
///
/// # Examples
///
/// ```no_run
/// # #[allow(unused_variables, unused_imports)]
/// # fn example(window: &mut gpui::Window, cx: &mut gpui::App) {
/// # use gpui::prelude::*;
/// use gpui::prelude::*;
/// use kui::components::breakdown_chart::{BreakdownChart, BreakdownChartEvent};
///
/// let chart = cx.new(|cx| {
///     BreakdownChart::new("sprint-breakdown", cx)
///         .with_title("Sprint 24")
///         .with_default_measure("count")
///         .with_default_slice("status")
///         .with_default_segment("priority")
/// });
/// cx.subscribe(&chart, |_, event: &BreakdownChartEvent, _| {
///     let _ = event;
/// }).detach();
/// # }
/// ```
pub struct BreakdownChart {
    id: ElementId,
    title: SharedString,
    title_zh: SharedString,
    records: Vec<BreakdownRecord>,
    dimensions: Vec<BreakdownDimension>,
    measures: Vec<BreakdownMeasure>,
    measure_key: SharedString,
    slice_key: SharedString,
    segment_key: SharedString,
    table_mode: BreakdownTableMode,
    hidden_segments: HashSet<SharedString>,
    hovered_slice: Option<SharedString>,
    hovered_segment: Option<SharedString>,
    bar_slides: Vec<Vec<Slide>>,
    bars_seeded: bool,
    y_max_slide: Slide,
    col_spotlights: Vec<Slide>,
    seg_spotlights: Vec<Slide>,
    picker: Option<ActivePicker>,
    loading: bool,
    empty: bool,
    width_cell: Rc<Cell<f32>>,
    focus: FocusHandle,
}

impl EventEmitter<BreakdownChartEvent> for BreakdownChart {}

impl BreakdownChart {
    /// Create a [`BreakdownChart`] preloaded with the 212-task `Sprint 24` dataset.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        Self {
            id: id.into(),
            title: "Sprint 24".into(),
            title_zh: "第 24 迭代工作量拆分".into(),
            records: default_sprint_records(),
            dimensions: default_dimensions(),
            measures: default_measures(),
            measure_key: "count".into(),
            slice_key: "status".into(),
            segment_key: "priority".into(),
            table_mode: BreakdownTableMode::BySegment,
            hidden_segments: HashSet::new(),
            hovered_slice: None,
            hovered_segment: None,
            bar_slides: Vec::new(),
            bars_seeded: false,
            y_max_slide: Slide::new(100.0),
            col_spotlights: Vec::new(),
            seg_spotlights: Vec::new(),
            picker: None,
            loading: false,
            empty: false,
            width_cell: Rc::new(Cell::new(560.)),
            focus: cx.focus_handle(),
        }
    }

    /// Customize chart heading.
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        let t = title.into();
        self.title_zh = t.clone();
        self.title = t;
        self
    }

    /// Provide custom flat [`BreakdownRecord`] entries.
    pub fn with_records(mut self, records: Vec<BreakdownRecord>) -> Self {
        self.records = records;
        self
    }

    /// Provide custom categorical [`BreakdownDimension`] definitions.
    pub fn with_dimensions(mut self, dimensions: Vec<BreakdownDimension>) -> Self {
        if let Some(first) = dimensions.first() {
            self.slice_key = first.key.clone();
        }
        if let Some(second) = dimensions.get(1) {
            self.segment_key = second.key.clone();
        }
        self.dimensions = dimensions;
        self
    }

    /// Provide custom numeric [`BreakdownMeasure`] definitions.
    pub fn with_measures(mut self, measures: Vec<BreakdownMeasure>) -> Self {
        if let Some(first) = measures.first() {
            self.measure_key = first.key.clone();
        }
        self.measures = measures;
        self
    }

    /// Set initial active measure key.
    pub fn with_default_measure(mut self, key: impl Into<SharedString>) -> Self {
        self.measure_key = key.into();
        self
    }

    /// Set initial active x-axis slice dimension key.
    pub fn with_default_slice(mut self, key: impl Into<SharedString>) -> Self {
        self.slice_key = key.into();
        self
    }

    /// Set initial active stack segment dimension key (or `"none"`).
    pub fn with_default_segment(mut self, key: impl Into<SharedString>) -> Self {
        self.segment_key = key.into();
        self
    }

    /// Choose initial table row mode ([`BreakdownTableMode::BySegment`] or [`BreakdownTableMode::BySlice`]).
    pub fn with_table_mode(mut self, mode: BreakdownTableMode) -> Self {
        self.table_mode = mode;
        self
    }

    /// Toggle skeleton loading state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Toggle forced empty state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Return current measure key.
    pub fn measure_key(&self) -> &str {
        &self.measure_key
    }

    /// Return current slice dimension key.
    pub fn slice_key(&self) -> &str {
        &self.slice_key
    }

    /// Return current segment dimension key (`"none"` if unstacked).
    pub fn segment_key(&self) -> &str {
        &self.segment_key
    }

    /// Switch active measure.
    pub fn set_measure(&mut self, key: &str, cx: &mut Context<Self>) -> bool {
        let Some(found) = self.measures.iter().find(|m| m.key.as_ref() == key) else {
            return false;
        };
        self.measure_key = found.key.clone();
        self.picker = None;
        cx.emit(BreakdownChartEvent::MeasureChanged(
            self.measure_key.clone(),
        ));
        cx.notify();
        true
    }

    /// Switch active x-axis slice dimension.
    pub fn set_slice(&mut self, key: &str, cx: &mut Context<Self>) -> bool {
        let Some(found) = self.dimensions.iter().find(|d| d.key.as_ref() == key) else {
            return false;
        };
        self.slice_key = found.key.clone();
        if self.segment_key == self.slice_key {
            self.segment_key = "none".into();
            self.hidden_segments.clear();
        }
        self.hovered_slice = None;
        self.picker = None;
        cx.emit(BreakdownChartEvent::SliceChanged(self.slice_key.clone()));
        cx.notify();
        true
    }

    /// Switch active stack segment dimension (pass `"none"` for single-series bars).
    pub fn set_segment(&mut self, key: &str, cx: &mut Context<Self>) -> bool {
        if key != "none" && !self.dimensions.iter().any(|d| d.key.as_ref() == key) {
            return false;
        }
        self.segment_key = SharedString::from(key.to_string());
        self.hidden_segments.clear();
        self.hovered_segment = None;
        self.picker = None;
        cx.emit(BreakdownChartEvent::SegmentChanged(
            self.segment_key.clone(),
        ));
        cx.notify();
        true
    }

    /// Toggle visibility of one stacked segment in the chart and totals.
    pub fn toggle_segment_visibility(&mut self, segment: &str, cx: &mut Context<Self>) {
        let key = SharedString::from(segment.to_string());
        let visible = if self.hidden_segments.contains(&key) {
            self.hidden_segments.remove(&key);
            true
        } else {
            self.hidden_segments.insert(key.clone());
            false
        };
        cx.emit(BreakdownChartEvent::SegmentVisibilityToggled {
            segment: key,
            visible,
        });
        cx.notify();
    }

    /// Show all hidden segments again.
    pub fn reset_hidden_segments(&mut self, cx: &mut Context<Self>) {
        self.hidden_segments.clear();
        cx.notify();
    }

    /// Compute `(slices, segments, matrix[slice_idx][seg_idx], grand_total)` for visible segments.
    pub fn compute_matrix(&self) -> (Vec<SharedString>, Vec<SharedString>, Vec<Vec<f64>>, f64) {
        let slices = self
            .dimensions
            .iter()
            .find(|d| d.key == self.slice_key)
            .map(|d| d.values.clone())
            .unwrap_or_default();

        let segments = if self.segment_key.as_ref() == "none" {
            vec![SharedString::from("Total")]
        } else {
            self.dimensions
                .iter()
                .find(|d| d.key == self.segment_key)
                .map(|d| d.values.clone())
                .unwrap_or_else(|| vec![SharedString::from("Total")])
        };

        let metric_field = self
            .measures
            .iter()
            .find(|m| m.key == self.measure_key)
            .and_then(|m| m.field.clone());

        let mut matrix = vec![vec![0.0_f64; segments.len()]; slices.len()];
        for rec in &self.records {
            let Some(sv) = rec.fields.get(&self.slice_key) else {
                continue;
            };
            let Some(s_idx) = slices.iter().position(|x| x == sv) else {
                continue;
            };
            let g_idx = if self.segment_key.as_ref() == "none" {
                0
            } else {
                let Some(gv) = rec.fields.get(&self.segment_key) else {
                    continue;
                };
                let Some(pos) = segments.iter().position(|x| x == gv) else {
                    continue;
                };
                pos
            };
            let val = match &metric_field {
                Some(f) => rec
                    .metrics
                    .get(f)
                    .copied()
                    .filter(|v| v.is_finite())
                    .unwrap_or(0.0)
                    .max(0.0),
                None => 1.0,
            };
            matrix[s_idx][g_idx] += val;
        }

        let mut grand_total = 0.0;
        for row in &matrix {
            for (g_idx, &v) in row.iter().enumerate() {
                if !self.hidden_segments.contains(&segments[g_idx]) {
                    grand_total += v;
                }
            }
        }
        (slices, segments, matrix, grand_total)
    }

    fn segment_color(&self, seg_name: &str, seg_idx: usize, p: &ProPalette) -> Hsla {
        if self.segment_key.as_ref() == "none" {
            return p.blue;
        }
        if let Some(dim) = self.dimensions.iter().find(|d| d.key == self.segment_key)
            && let Some(&custom) = dim.colors.get(seg_name)
        {
            return custom;
        }
        let fallback = [
            rgb(0xf04438).into(),
            rgb(0xe8b000).into(),
            p.blue,
            rgb(0x6cbcf8).into(),
            if p.dark {
                rgb(0x52525b).into()
            } else {
                rgb(0xd4d4d8).into()
            },
            p.purple,
            p.teal,
        ];
        fallback[seg_idx % fallback.len()]
    }
}

impl Render for BreakdownChart {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let width_cell = self.width_cell.clone();
        let narrow = width_cell.get() < 460.;

        let sensor = canvas(
            move |bounds, _, _| {
                let bw = f32::from(bounds.size.width);
                if bw > 10. {
                    width_cell.set(bw);
                }
            },
            |_, _, _, _| (),
        )
        .w_full()
        .h(px(0.));

        if self.loading {
            let phase = if disabled_motion(cx) { 0.3 } else { 0.5 };
            return div()
                .id(self.id.clone())
                .w_full()
                .max_w(px(580.))
                .mx_auto()
                .rounded(px(10.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .p(px(16.))
                .flex()
                .flex_col()
                .gap(px(14.))
                .child(pro_skeleton_bar(200., 18., 4., phase, cx))
                .child(pro_skeleton_bar(520., 32., 6., phase, cx))
                .child(pro_skeleton_bar(520., 190., 8., phase, cx))
                .child(pro_skeleton_bar(520., 120., 6., phase, cx))
                .into_any_element();
        }

        if self.empty || self.records.is_empty() {
            return div()
                .id(self.id.clone())
                .w_full()
                .max_w(px(580.))
                .mx_auto()
                .rounded(px(10.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .child(pro_empty_state(
                    ProIcon::UsageChart,
                    tr(cx, "No records to break down", "暂无可拆分的记录数据"),
                    tr(
                        cx,
                        "Supply task, ticket, or order records to inspect stacked slices.",
                        "传入任务、工单或订单记录以查看多维堆叠拆分图。",
                    ),
                    Some((
                        pro_eid(&self.id, "empty-seed"),
                        SharedString::from(tr(cx, "Load Sprint 24 data", "加载示例数据")),
                        Box::new(|this: &mut Self, _, cx| {
                            this.empty = false;
                            if this.records.is_empty() {
                                this.records = default_sprint_records();
                            }
                            cx.notify();
                        }),
                    )),
                    cx,
                ))
                .into_any_element();
        }

        let (slices, segments, matrix, grand_total) = self.compute_matrix();
        let slice_totals: Vec<f64> = matrix
            .iter()
            .map(|row| {
                row.iter()
                    .enumerate()
                    .filter(|(g_idx, _)| !self.hidden_segments.contains(&segments[*g_idx]))
                    .map(|(_, &v)| v)
                    .sum()
            })
            .collect();

        let active_measure = self
            .measures
            .iter()
            .find(|m| m.key == self.measure_key)
            .cloned()
            .unwrap_or_else(|| BreakdownMeasure::count("count", "Count", "items"));

        let heading = tr(cx, self.title.as_ref(), self.title_zh.as_ref()).to_string();
        let unit_text = tr(
            cx,
            active_measure.unit.as_ref(),
            active_measure.unit_zh.as_ref(),
        )
        .to_string();

        // Dynamic rolling header total when hovering a slice, segment, or matrix cell!
        let (shown_grand, shown_unit): (f64, SharedString) =
            match (self.hovered_slice.as_ref(), self.hovered_segment.as_ref()) {
                (Some(hs), Some(hg)) => {
                    let s_i = slices.iter().position(|x| x == hs);
                    let g_i = segments.iter().position(|x| x == hg);
                    if let (Some(si), Some(gi)) = (s_i, g_i) {
                        let cell_v = if self.hidden_segments.contains(hg) {
                            0.0
                        } else {
                            matrix[si][gi]
                        };
                        (cell_v, format!("{hs} \u{00b7} {hg} {unit_text}").into())
                    } else {
                        (grand_total, unit_text.clone().into())
                    }
                }
                (Some(hs), None) => {
                    if let Some(si) = slices.iter().position(|x| x == hs) {
                        (
                            slice_totals[si],
                            format!("{hs} \u{00b7} {unit_text}").into(),
                        )
                    } else {
                        (grand_total, unit_text.clone().into())
                    }
                }
                (None, Some(hg)) => {
                    if let Some(gi) = segments.iter().position(|x| x == hg) {
                        let seg_sum: f64 = if self.hidden_segments.contains(hg) {
                            0.0
                        } else {
                            (0..slices.len()).map(|si| matrix[si][gi]).sum()
                        };
                        (seg_sum, format!("{hg} \u{00b7} {unit_text}").into())
                    } else {
                        (grand_total, unit_text.clone().into())
                    }
                }
                (None, None) => (grand_total, unit_text.clone().into()),
            };

        let header = div()
            .w_full()
            .pt(px(14.))
            .px(px(16.))
            .flex()
            .items_baseline()
            .justify_between()
            .gap(px(12.))
            .child(
                div()
                    .min_w_0()
                    .truncate()
                    .text_size(px(15.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .child(heading),
            )
            .child(
                div()
                    .flex()
                    .items_baseline()
                    .gap(px(5.))
                    .child(pro_rolling_number(
                        pro_eid(&self.id, "grand-roll"),
                        shown_grand,
                        0,
                        "",
                        "",
                        15.,
                        FontWeight::MEDIUM,
                        p.ink,
                    ))
                    .child(pro_swap_label(
                        pro_eid(&self.id, "grand-unit"),
                        shown_unit,
                        12.,
                        FontWeight::NORMAL,
                        p.muted,
                    )),
            );

        // 3 Pickers: Measure | Slice | Segment (using `pro_popover_presence` + `pro_check_draw` + `pro_swap_label`)
        let cur_measure_lbl = tr(
            cx,
            active_measure.label.as_ref(),
            active_measure.label_zh.as_ref(),
        )
        .to_string();
        let cur_slice_lbl = self
            .dimensions
            .iter()
            .find(|d| d.key == self.slice_key)
            .map(|d| tr(cx, d.label.as_ref(), d.label_zh.as_ref()).to_string())
            .unwrap_or_else(|| self.slice_key.to_string());
        let cur_seg_lbl = if self.segment_key.as_ref() == "none" {
            tr(cx, "None", "不拆分").to_string()
        } else {
            self.dimensions
                .iter()
                .find(|d| d.key == self.segment_key)
                .map(|d| tr(cx, d.label.as_ref(), d.label_zh.as_ref()).to_string())
                .unwrap_or_else(|| self.segment_key.to_string())
        };

        let m_open = self.picker == Some(ActivePicker::Measure);
        let mut m_menu = div()
            .w(px(168.))
            .p(px(4.))
            .rounded(px(9.))
            .bg(p.card)
            .border_1()
            .border_color(p.line_strong)
            .shadow_lg()
            .flex()
            .flex_col()
            .gap(px(1.));
        for m in &self.measures {
            let sel = m.key == self.measure_key;
            let mk = m.key.to_string();
            let mlbl = tr(cx, m.label.as_ref(), m.label_zh.as_ref()).to_string();
            m_menu = m_menu.child(activate(
                div()
                    .id(pro_eid(&self.id, format!("m-opt-{mk}")))
                    .role(Role::Button)
                    .aria_label(mlbl.clone())
                    .tab_index(0)
                    .h(px(28.))
                    .px(px(8.))
                    .rounded(px(6.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_size(px(12.5))
                    .text_color(p.ink)
                    .cursor_pointer()
                    .hover(move |s| s.bg(p.hover))
                    .child(mlbl)
                    .when(sel, |d| {
                        d.child(pro_check_draw(
                            pro_eid(&self.id, format!("m-chk-{mk}")),
                            true,
                            14.,
                            false,
                            p.blue,
                        ))
                    }),
                cx,
                move |this, _, cx| {
                    this.set_measure(&mk, cx);
                },
            ));
        }

        let sl_open = self.picker == Some(ActivePicker::Slice);
        let mut sl_menu = div()
            .w(px(160.))
            .p(px(4.))
            .rounded(px(9.))
            .bg(p.card)
            .border_1()
            .border_color(p.line_strong)
            .shadow_lg()
            .flex()
            .flex_col()
            .gap(px(1.));
        for d in &self.dimensions {
            let sel = d.key == self.slice_key;
            let dk = d.key.to_string();
            let dlbl = tr(cx, d.label.as_ref(), d.label_zh.as_ref()).to_string();
            sl_menu = sl_menu.child(activate(
                div()
                    .id(pro_eid(&self.id, format!("sl-opt-{dk}")))
                    .role(Role::Button)
                    .aria_label(dlbl.clone())
                    .tab_index(0)
                    .h(px(28.))
                    .px(px(8.))
                    .rounded(px(6.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_size(px(12.5))
                    .text_color(p.ink)
                    .cursor_pointer()
                    .hover(move |s| s.bg(p.hover))
                    .child(dlbl)
                    .when(sel, |el| {
                        el.child(pro_check_draw(
                            pro_eid(&self.id, format!("sl-chk-{dk}")),
                            true,
                            14.,
                            false,
                            p.blue,
                        ))
                    }),
                cx,
                move |this, _, cx| {
                    this.set_slice(&dk, cx);
                },
            ));
        }

        let sg_open = self.picker == Some(ActivePicker::Segment);
        let mut sg_menu = div()
            .w(px(160.))
            .p(px(4.))
            .rounded(px(9.))
            .bg(p.card)
            .border_1()
            .border_color(p.line_strong)
            .shadow_lg()
            .flex()
            .flex_col()
            .gap(px(1.));
        for d in &self.dimensions {
            if d.key == self.slice_key {
                continue;
            }
            let sel = d.key == self.segment_key;
            let dk = d.key.to_string();
            let dlbl = tr(cx, d.label.as_ref(), d.label_zh.as_ref()).to_string();
            sg_menu = sg_menu.child(activate(
                div()
                    .id(pro_eid(&self.id, format!("sg-opt-{dk}")))
                    .role(Role::Button)
                    .aria_label(dlbl.clone())
                    .tab_index(0)
                    .h(px(28.))
                    .px(px(8.))
                    .rounded(px(6.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_size(px(12.5))
                    .text_color(p.ink)
                    .cursor_pointer()
                    .hover(move |s| s.bg(p.hover))
                    .child(dlbl)
                    .when(sel, |el| {
                        el.child(pro_check_draw(
                            pro_eid(&self.id, format!("sg-chk-{dk}")),
                            true,
                            14.,
                            false,
                            p.blue,
                        ))
                    }),
                cx,
                move |this, _, cx| {
                    this.set_segment(&dk, cx);
                },
            ));
        }
        let none_sel = self.segment_key.as_ref() == "none";
        sg_menu = sg_menu.child(activate(
            div()
                .id(pro_eid(&self.id, "sg-opt-none"))
                .role(Role::Button)
                .aria_label(tr(cx, "None", "不拆分"))
                .tab_index(0)
                .h(px(28.))
                .px(px(8.))
                .rounded(px(6.))
                .flex()
                .items_center()
                .justify_between()
                .text_size(px(12.5))
                .text_color(p.ink)
                .cursor_pointer()
                .hover(move |s| s.bg(p.hover))
                .child(tr(cx, "None", "不拆分"))
                .when(none_sel, |el| {
                    el.child(pro_check_draw(
                        pro_eid(&self.id, "sg-chk-none"),
                        true,
                        14.,
                        false,
                        p.blue,
                    ))
                }),
            cx,
            |this, _, cx| {
                this.set_segment("none", cx);
            },
        ));

        let build_picker_col = |id_sfx: &'static str,
                                label: &str,
                                val: String,
                                open: bool,
                                target: ActivePicker,
                                menu: Div,
                                align_right: bool,
                                cx: &mut Context<Self>| {
            let pop_layer = {
                let inner = pro_popover_presence(
                    pro_eid(&self.id, format!("{id_sfx}-pop")),
                    open,
                    move || menu.into_any_element(),
                );
                let wrap = div().occlude().absolute().top(px(50.)).child(inner);
                if align_right {
                    wrap.right_0().into_any_element()
                } else {
                    wrap.left_0().into_any_element()
                }
            };

            div()
                .flex_1()
                .min_w_0()
                .relative()
                .flex()
                .flex_col()
                .gap(px(4.))
                .child(
                    div()
                        .pl(px(2.))
                        .text_size(px(11.5))
                        .text_color(p.muted)
                        .child(label.to_string()),
                )
                .child(activate(
                    div()
                        .id(pro_eid(&self.id, id_sfx))
                        .role(Role::Button)
                        .aria_label(format!("{label}: {val}"))
                        .tab_index(0)
                        .w_full()
                        .h(px(28.))
                        .pl(px(9.))
                        .pr(px(6.))
                        .rounded(px(6.))
                        .bg(if p.dark { rgb(0x232326).into() } else { p.card })
                        .border_1()
                        .border_color(if open { p.line_strong } else { p.line })
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(4.))
                        .text_size(px(12.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.strong)
                        .cursor_pointer()
                        .hover(move |s| s.border_color(p.line_strong))
                        .focus_visible(move |s| s.border_color(p.focus))
                        .child(div().min_w_0().overflow_hidden().child(pro_swap_label(
                            pro_eid(&self.id, format!("{id_sfx}-lbl")),
                            val,
                            12.5,
                            FontWeight::MEDIUM,
                            p.strong,
                        )))
                        .child(pro_icon(
                            if open {
                                ProIcon::ChevronUp
                            } else {
                                ProIcon::ChevronDown
                            },
                            14.,
                            p.muted,
                        )),
                    cx,
                    move |this, _, cx| {
                        this.picker = if this.picker == Some(target) {
                            None
                        } else {
                            Some(target)
                        };
                        cx.notify();
                    },
                ))
                .child(pop_layer)
        };

        let m_col = build_picker_col(
            "pick-measure",
            tr(cx, "Measure", "统计指标"),
            cur_measure_lbl,
            m_open,
            ActivePicker::Measure,
            m_menu,
            false,
            cx,
        );
        let sl_col = build_picker_col(
            "pick-slice",
            tr(cx, "Slice", "横轴分组"),
            cur_slice_lbl.clone(),
            sl_open,
            ActivePicker::Slice,
            sl_menu,
            false,
            cx,
        );
        let sg_col = build_picker_col(
            "pick-segment",
            tr(cx, "Segment", "堆叠拆分"),
            cur_seg_lbl.clone(),
            sg_open,
            ActivePicker::Segment,
            sg_menu,
            true,
            cx,
        );

        let controls_row = div()
            .w_full()
            .pt(px(12.))
            .px(px(16.))
            .flex()
            .when(narrow, |d| d.flex_wrap())
            .gap(px(8.))
            .child(m_col)
            .child(sl_col)
            .child(sg_col);

        // Compute nice rounded Y scale max and spring-morph stacked bar segments via positional `Slide`s!
        let raw_max = slice_totals
            .iter()
            .copied()
            .fold(0.0_f64, f64::max)
            .max(1.0);
        let y_max = nice_axis_ceiling(raw_max);
        let plot_h = 188.0_f32;

        if !self.bars_seeded {
            self.y_max_slide.reset(y_max as f32);
        } else {
            self.y_max_slide.duration(y_max as f32, 0.34, cx);
        }
        let anim_y_max = self.y_max_slide.get(w, cx).max(1.0) as f64;

        // Sync positional `bar_slides[s_idx][g_idx]`, `col_spotlights`, and `seg_spotlights`.
        let n_slices = slices.len();
        let n_segs = segments.len();
        if self.bar_slides.len() < n_slices {
            self.bar_slides.resize_with(n_slices, Vec::new);
        }
        for row_slides in &mut self.bar_slides[..n_slices] {
            if row_slides.len() < n_segs {
                row_slides.resize_with(n_segs, || Slide::new(0.0));
            }
        }
        if self.col_spotlights.len() < n_slices {
            self.col_spotlights
                .resize_with(n_slices, || Slide::snappy(1.0));
        } else if self.col_spotlights.len() > n_slices {
            self.col_spotlights.truncate(n_slices);
        }
        if self.seg_spotlights.len() < n_segs {
            self.seg_spotlights
                .resize_with(n_segs, || Slide::snappy(1.0));
        } else if self.seg_spotlights.len() > n_segs {
            self.seg_spotlights.truncate(n_segs);
        }

        let mut col_alphas = Vec::with_capacity(n_slices);
        for (s_idx, s_name) in slices.iter().enumerate() {
            let dim_col = self.hovered_slice.as_ref().is_some_and(|hs| hs != s_name);
            self.col_spotlights[s_idx].set(if dim_col { 0.34 } else { 1.0 }, cx);
            col_alphas.push(self.col_spotlights[s_idx].get(w, cx).clamp(0.20, 1.0));
        }

        let mut seg_alphas = Vec::with_capacity(n_segs);
        for (g_idx, g_name) in segments.iter().enumerate() {
            let dim_seg = self.hovered_segment.as_ref().is_some_and(|hg| hg != g_name);
            self.seg_spotlights[g_idx].set(if dim_seg { 0.22 } else { 1.0 }, cx);
            seg_alphas.push(self.seg_spotlights[g_idx].get(w, cx).clamp(0.15, 1.0));
        }

        // Horizontal gridlines + right Y ticks
        let mut grid_layer = div().absolute().inset_0();
        for step in 0..=5usize {
            let frac = step as f32 / 5.0;
            let val = y_max * f64::from(1.0 - frac);
            let y_px = frac * plot_h;
            let is_zero = step == 5;
            grid_layer = grid_layer.child(
                div()
                    .absolute()
                    .left_0()
                    .right(px(34.))
                    .top(px(y_px))
                    .h(px(1.))
                    .bg(if is_zero { p.line_strong } else { p.line })
                    .child(
                        div()
                            .absolute()
                            .right(px(-34.))
                            .top(px(-8.))
                            .w(px(30.))
                            .flex()
                            .justify_end()
                            .text_size(px(10.5))
                            .text_color(p.muted)
                            .child(format!("{val:.0}")),
                    ),
            );
        }

        // Stacked bars + x-axis labels
        let mut bars_row = div()
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right(px(34.))
            .flex()
            .items_end()
            .gap(px(6.));
        let mut xaxis_row = div()
            .w_full()
            .flex_none()
            .mt(px(8.))
            .pr(px(34.))
            .flex()
            .items_center()
            .gap(px(6.));

        let first_seed = !self.bars_seeded;
        for (s_idx, s_name) in slices.iter().enumerate() {
            let s_total = slice_totals[s_idx];
            let slice_active = self.hovered_slice.as_ref() == Some(s_name);
            let col_alpha = col_alphas[s_idx];

            let mut stack_col = div()
                .w(if narrow { px(14.) } else { px(18.) })
                .rounded_t(px(3.5))
                .overflow_hidden()
                .flex()
                .flex_col_reverse()
                .gap(px(1.));

            for (g_idx, g_name) in segments.iter().enumerate() {
                let hidden = self.hidden_segments.contains(g_name);
                let cell_val = if hidden { 0.0 } else { matrix[s_idx][g_idx] };
                let target_h = ((cell_val / anim_y_max) as f32 * plot_h).clamp(0.0, plot_h);
                if first_seed {
                    self.bar_slides[s_idx][g_idx].reset(target_h);
                } else {
                    self.bar_slides[s_idx][g_idx].duration(target_h, 0.34, cx);
                }
                let anim_h = self.bar_slides[s_idx][g_idx].get(w, cx).max(0.0);

                if anim_h <= 0.4 {
                    continue;
                }
                let base_c = self.segment_color(g_name, g_idx, &p);
                let seg_c = base_c.opacity(base_c.a * seg_alphas[g_idx]);

                stack_col = stack_col.child(div().w_full().flex_none().h(px(anim_h)).bg(seg_c));
            }

            let s_hov = s_name.clone();
            let s_click = s_name.clone();
            let col_btn = activate(
                div()
                    .id(pro_eid(&self.id, format!("slice-col-{s_idx}")))
                    .role(Role::Button)
                    .aria_label(format!("{s_name}: {s_total:.0} {unit_text}"))
                    .tab_index(0)
                    .flex_1()
                    .h_full()
                    .relative()
                    .rounded(px(6.))
                    .opacity(col_alpha)
                    .cursor_pointer()
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                        if *inside {
                            this.hovered_slice = Some(s_hov.clone());
                        } else if this.hovered_slice.as_ref() == Some(&s_hov) {
                            this.hovered_slice = None;
                        }
                        cx.notify();
                    }))
                    .flex()
                    .items_end()
                    .justify_center()
                    .when(slice_active, |d| {
                        d.child(
                            div()
                                .absolute()
                                .top_0()
                                .bottom_0()
                                .w(if narrow { px(14.) } else { px(18.) })
                                .rounded_t(px(3.5))
                                .bg(p.hover),
                        )
                    })
                    .child(stack_col),
                cx,
                move |_, _, cx| {
                    cx.emit(BreakdownChartEvent::SliceFocused(s_click.clone()));
                },
            );
            bars_row = bars_row.child(col_btn);

            xaxis_row = xaxis_row.child(
                div()
                    .flex_1()
                    .min_w_0()
                    .h(px(22.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(if slice_active {
                        div()
                            .px(px(6.))
                            .h(px(20.))
                            .rounded(px(5.))
                            .bg(p.chip)
                            .flex()
                            .items_center()
                            .text_size(px(11.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .truncate()
                            .child(s_name.clone())
                            .into_any_element()
                    } else {
                        div()
                            .min_w_0()
                            .truncate()
                            .text_size(px(11.))
                            .text_color(p.muted)
                            .child(s_name.clone())
                            .into_any_element()
                    }),
            );
        }
        // Clear any unused trailing bar_slides if slice/segment count shrank so future growth starts clean.
        for row_slides in &mut self.bar_slides[n_slices..] {
            for sl in row_slides.iter_mut() {
                sl.duration(0.0, 0.24, cx);
                let _ = sl.get(w, cx);
            }
        }
        for row_slides in &mut self.bar_slides[..n_slices] {
            for sl in &mut row_slides[n_segs..] {
                sl.duration(0.0, 0.24, cx);
                let _ = sl.get(w, cx);
            }
        }
        self.bars_seeded = true;

        // Hover breakdown tooltip with `pro_popover_presence` + `pro_rolling_number`
        let tip_open = self
            .hovered_slice
            .as_ref()
            .is_some_and(|hs| slices.contains(hs));
        let tooltip_el = if let Some(hov_s) = &self.hovered_slice
            && let Some(s_idx) = slices.iter().position(|x| x == hov_s)
        {
            let tot = slice_totals[s_idx];
            let place_left = s_idx * 2 >= slices.len();
            let mut tip_rows = div()
                .mt(px(5.))
                .pt(px(5.))
                .border_t_1()
                .border_color(p.line)
                .flex()
                .flex_col()
                .gap(px(2.));

            for (g_idx, g_name) in segments.iter().enumerate() {
                if self.hidden_segments.contains(g_name) {
                    continue;
                }
                let val = matrix[s_idx][g_idx];
                let sw = self.segment_color(g_name, g_idx, &p);
                let seg_hi = self.hovered_segment.as_ref() == Some(g_name);
                tip_rows = tip_rows.child(
                    div()
                        .opacity(seg_alphas[g_idx].max(0.45))
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(12.))
                        .text_size(px(11.5))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .child(div().size(px(8.)).rounded(px(2.)).bg(sw))
                                .child(
                                    div()
                                        .font_weight(if seg_hi {
                                            FontWeight::MEDIUM
                                        } else {
                                            FontWeight::NORMAL
                                        })
                                        .text_color(if seg_hi { p.strong } else { p.text })
                                        .child(g_name.clone()),
                                ),
                        )
                        .child(
                            div()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.ink)
                                .child(format!("{val:.0}")),
                        ),
                );
            }

            let hov_title = hov_s.clone();
            let show_segs = self.segment_key.as_ref() != "none";
            let tip_id = pro_eid(&self.id, "tip-pres");
            let tot_id = pro_eid(&self.id, "tip-tot");
            let title_id = pro_eid(&self.id, "tip-title");

            let card_pop = pro_popover_presence(tip_id, tip_open, move || {
                div()
                    .min_w(px(156.))
                    .py(px(7.))
                    .px(px(10.))
                    .rounded(px(9.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line_strong)
                    .shadow_md()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(14.))
                            .child(pro_swap_label(
                                title_id,
                                hov_title,
                                12.5,
                                FontWeight::MEDIUM,
                                p.strong,
                            ))
                            .child(pro_rolling_number(
                                tot_id,
                                tot,
                                0,
                                "",
                                "",
                                12.5,
                                FontWeight::MEDIUM,
                                p.strong,
                            )),
                    )
                    .when(show_segs, |d| d.child(tip_rows))
                    .into_any_element()
            });

            let pos_wrap = div().absolute().top(px(4.)).child(card_pop);
            if place_left {
                pos_wrap.left(px(8.)).into_any_element()
            } else {
                pos_wrap.right(px(42.)).into_any_element()
            }
        } else {
            div().into_any_element()
        };

        let chart_section = div()
            .w_full()
            .pt(px(18.))
            .px(px(16.))
            .pb(px(12.))
            .flex()
            .flex_col()
            .child(
                div()
                    .w_full()
                    .flex_none()
                    .h(px(plot_h))
                    .relative()
                    .child(grid_layer)
                    .child(bars_row)
                    .child(tooltip_el),
            )
            .child(xaxis_row);

        // Table mode pill bar (`pro_pill_indicator` switching BySegment <-> BySlice)
        let mode_sensor = pro_selection_sensor();
        let mode_pill_strip = div()
            .p(px(2.))
            .rounded(px(7.))
            .bg(p.subtle)
            .border_1()
            .border_color(p.line)
            .relative()
            .flex()
            .items_center()
            .gap(px(2.))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "tmode-ind"),
                self.table_mode.key(),
                mode_sensor.clone(),
                5.,
                p.card,
                Some(p.line),
                !p.dark,
            ))
            .children(
                [
                    (
                        BreakdownTableMode::BySegment,
                        tr(cx, "By segment", "按拆分项"),
                    ),
                    (BreakdownTableMode::BySlice, tr(cx, "By slice", "按分组列")),
                ]
                .into_iter()
                .map(|(m, lbl)| {
                    let sel = self.table_mode == m;
                    activate(
                        div()
                            .id(pro_eid(&self.id, format!("tmode-{}", m.key())))
                            .role(Role::Button)
                            .aria_label(lbl)
                            .tab_index(0)
                            .h(px(22.))
                            .px(px(8.))
                            .relative()
                            .rounded(px(5.))
                            .flex()
                            .items_center()
                            .text_size(px(11.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(if sel { p.ink } else { p.muted })
                            .cursor_pointer()
                            .when(!sel, move |d| d.hover(move |s| s.text_color(p.ink)))
                            .when(sel, |d| d.child(pro_mark_selected(&mode_sensor)))
                            .child(lbl),
                        cx,
                        move |this, _, cx| {
                            this.table_mode = m;
                            cx.notify();
                        },
                    )
                }),
            );

        let table_toolbar = div()
            .w_full()
            .px(px(16.))
            .py(px(7.))
            .border_t_1()
            .border_color(p.line)
            .flex()
            .items_center()
            .justify_between()
            .gap(px(8.))
            .child(div().text_size(px(11.)).text_color(p.muted).child(tr(
                cx,
                "Hover any row or matrix cell to highlight across bars",
                "悬停任一表格行或单元格即可联动高亮柱状图",
            )))
            .child(mode_pill_strip);

        // Linked Breakdown Table (hovering individual matrix cells cross-highlights `(column, segment)`!)
        let linked_table = match self.table_mode {
            BreakdownTableMode::BySegment => {
                let mut head_row = div()
                    .w_full()
                    .h(px(30.))
                    .px(px(16.))
                    .border_t_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .text_size(px(10.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.muted)
                    .child(
                        div()
                            .w(px(96.))
                            .flex_none()
                            .child(cur_seg_lbl.to_uppercase()),
                    )
                    .child(
                        div()
                            .w(px(54.))
                            .flex_none()
                            .flex()
                            .justify_end()
                            .child(tr(cx, "TOTAL", "合计")),
                    );

                if !narrow {
                    for s_name in &slices {
                        let s_act = self.hovered_slice.as_ref() == Some(s_name);
                        head_row = head_row.child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .justify_end()
                                .text_color(if s_act { p.ink } else { p.muted })
                                .child(div().truncate().child(s_name.to_uppercase())),
                        );
                    }
                } else {
                    head_row = head_row.child(
                        div()
                            .flex_1()
                            .flex()
                            .justify_end()
                            .child(tr(cx, "SHARE", "占比")),
                    );
                }

                let mut body_rows = div().w_full().flex().flex_col();
                for (g_idx, g_name) in segments.iter().enumerate() {
                    let hidden = self.hidden_segments.contains(g_name);
                    let row_sum: f64 = (0..slices.len()).map(|si| matrix[si][g_idx]).sum();
                    let row_active = self.hovered_segment.as_ref() == Some(g_name);
                    let sw = self.segment_color(g_name, g_idx, &p);
                    let g_hov = g_name.clone();
                    let g_tog = g_name.to_string();
                    let row_alpha = if hidden {
                        0.40
                    } else {
                        seg_alphas[g_idx].max(0.55)
                    };

                    let mut tr_el = activate(
                        div()
                            .id(pro_eid(&self.id, format!("tseg-row-{g_idx}")))
                            .role(Role::Button)
                            .aria_label(format!("{g_name}: {row_sum:.0}"))
                            .tab_index(0)
                            .w_full()
                            .h(px(34.))
                            .px(px(16.))
                            .border_t_1()
                            .border_color(p.line)
                            .bg(if row_active {
                                p.hover
                            } else {
                                transparent_black()
                            })
                            .opacity(row_alpha)
                            .cursor_pointer()
                            .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                                if *inside {
                                    this.hovered_segment = Some(g_hov.clone());
                                } else if this.hovered_segment.as_ref() == Some(&g_hov) {
                                    this.hovered_segment = None;
                                }
                                cx.notify();
                            }))
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .child(
                                div()
                                    .w(px(96.))
                                    .flex_none()
                                    .flex()
                                    .items_center()
                                    .gap(px(7.))
                                    .child(div().size(px(8.)).rounded(px(2.)).bg(sw))
                                    .child(
                                        div()
                                            .min_w_0()
                                            .truncate()
                                            .text_size(px(12.5))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.ink)
                                            .child(g_name.clone()),
                                    ),
                            )
                            .child(div().w(px(54.)).flex_none().flex().justify_end().child(
                                pro_rolling_number(
                                    pro_eid(&self.id, format!("tseg-tot-{g_idx}")),
                                    row_sum,
                                    0,
                                    "",
                                    "",
                                    12.5,
                                    FontWeight::MEDIUM,
                                    p.ink,
                                ),
                            )),
                        cx,
                        move |this, _, cx| {
                            if this.segment_key.as_ref() != "none" {
                                this.toggle_segment_visibility(&g_tog, cx);
                            }
                        },
                    );

                    if !narrow {
                        for (s_idx, s_name) in slices.iter().enumerate() {
                            let v = matrix[s_idx][g_idx];
                            let s_act = self.hovered_slice.as_ref() == Some(s_name);
                            let cell_hot = s_act && row_active;
                            let cs_name = s_name.clone();
                            let cg_name = g_name.clone();
                            tr_el = tr_el.child(
                                div()
                                    .id(pro_eid(&self.id, format!("mcell-{g_idx}-{s_idx}")))
                                    .flex_1()
                                    .min_w_0()
                                    .h(px(24.))
                                    .px(px(4.))
                                    .rounded(px(4.))
                                    .bg(if cell_hot {
                                        p.select
                                    } else {
                                        transparent_black()
                                    })
                                    .flex()
                                    .items_center()
                                    .justify_end()
                                    .text_size(px(12.5))
                                    .font_weight(if s_act || row_active {
                                        FontWeight::MEDIUM
                                    } else {
                                        FontWeight::NORMAL
                                    })
                                    .text_color(if cell_hot {
                                        p.strong
                                    } else if s_act || row_active {
                                        p.ink
                                    } else {
                                        p.text
                                    })
                                    .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                                        if *inside {
                                            this.hovered_slice = Some(cs_name.clone());
                                            this.hovered_segment = Some(cg_name.clone());
                                        } else if this.hovered_slice.as_ref() == Some(&cs_name) {
                                            this.hovered_slice = None;
                                        }
                                        cx.notify();
                                    }))
                                    .child(format!("{v:.0}")),
                            );
                        }
                    } else {
                        let share = if grand_total > 0.0 && !hidden {
                            (row_sum / grand_total) * 100.0
                        } else {
                            0.0
                        };
                        tr_el = tr_el.child(
                            div()
                                .flex_1()
                                .flex()
                                .justify_end()
                                .text_size(px(12.))
                                .text_color(p.muted)
                                .child(format!("{share:.0}%")),
                        );
                    }
                    body_rows = body_rows.child(tr_el);
                }

                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .child(head_row)
                    .child(body_rows)
                    .into_any_element()
            }
            BreakdownTableMode::BySlice => {
                let mut head_row = div()
                    .w_full()
                    .h(px(30.))
                    .px(px(16.))
                    .border_t_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .text_size(px(10.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.muted)
                    .child(
                        div()
                            .w(px(108.))
                            .flex_none()
                            .child(cur_slice_lbl.to_uppercase()),
                    )
                    .child(
                        div()
                            .w(px(64.))
                            .flex_none()
                            .flex()
                            .justify_end()
                            .child(tr(cx, "TOTAL", "合计")),
                    );

                if !narrow {
                    for (g_idx, g_name) in segments.iter().enumerate() {
                        let sw = self.segment_color(g_name, g_idx, &p);
                        let g_act = self.hovered_segment.as_ref() == Some(g_name);
                        head_row = head_row.child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .items_center()
                                .justify_end()
                                .gap(px(4.))
                                .text_color(if g_act { p.ink } else { p.muted })
                                .child(div().size(px(6.)).rounded(px(1.5)).bg(sw))
                                .child(div().truncate().child(g_name.to_uppercase())),
                        );
                    }
                }

                let mut body_rows = div().w_full().flex().flex_col();
                for (s_idx, s_name) in slices.iter().enumerate() {
                    let s_tot = slice_totals[s_idx];
                    let row_act = self.hovered_slice.as_ref() == Some(s_name);
                    let s_hov = s_name.clone();
                    let s_click = s_name.clone();

                    let mut tr_el = activate(
                        div()
                            .id(pro_eid(&self.id, format!("tsl-row-{s_idx}")))
                            .role(Role::Button)
                            .aria_label(format!("{s_name}: {s_tot:.0}"))
                            .tab_index(0)
                            .w_full()
                            .h(px(34.))
                            .px(px(16.))
                            .border_t_1()
                            .border_color(p.line)
                            .bg(if row_act {
                                p.hover
                            } else {
                                transparent_black()
                            })
                            .opacity(col_alphas[s_idx].max(0.55))
                            .cursor_pointer()
                            .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                                if *inside {
                                    this.hovered_slice = Some(s_hov.clone());
                                } else if this.hovered_slice.as_ref() == Some(&s_hov) {
                                    this.hovered_slice = None;
                                }
                                cx.notify();
                            }))
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .child(
                                div()
                                    .w(px(108.))
                                    .flex_none()
                                    .truncate()
                                    .text_size(px(12.5))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .child(s_name.clone()),
                            )
                            .child(div().w(px(64.)).flex_none().flex().justify_end().child(
                                pro_rolling_number(
                                    pro_eid(&self.id, format!("tsl-tot-{s_idx}")),
                                    s_tot,
                                    0,
                                    "",
                                    "",
                                    12.5,
                                    FontWeight::MEDIUM,
                                    p.ink,
                                ),
                            )),
                        cx,
                        move |_, _, cx| {
                            cx.emit(BreakdownChartEvent::SliceFocused(s_click.clone()));
                        },
                    );

                    if !narrow {
                        for (g_idx, g_name) in segments.iter().enumerate() {
                            let hidden = self.hidden_segments.contains(g_name);
                            let v = matrix[s_idx][g_idx];
                            let g_act = self.hovered_segment.as_ref() == Some(g_name);
                            let cell_hot = row_act && g_act;
                            let cs_name = s_name.clone();
                            let cg_name = g_name.clone();
                            tr_el = tr_el.child(
                                div()
                                    .id(pro_eid(&self.id, format!("scell-{s_idx}-{g_idx}")))
                                    .flex_1()
                                    .min_w_0()
                                    .h(px(24.))
                                    .px(px(4.))
                                    .rounded(px(4.))
                                    .bg(if cell_hot {
                                        p.select
                                    } else {
                                        transparent_black()
                                    })
                                    .flex()
                                    .items_center()
                                    .justify_end()
                                    .opacity(if hidden { 0.35 } else { 1.0 })
                                    .text_size(px(12.5))
                                    .font_weight(if cell_hot {
                                        FontWeight::MEDIUM
                                    } else {
                                        FontWeight::NORMAL
                                    })
                                    .text_color(if cell_hot { p.strong } else { p.text })
                                    .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                                        if *inside {
                                            this.hovered_slice = Some(cs_name.clone());
                                            this.hovered_segment = Some(cg_name.clone());
                                        } else if this.hovered_segment.as_ref() == Some(&cg_name) {
                                            this.hovered_segment = None;
                                        }
                                        cx.notify();
                                    }))
                                    .child(format!("{v:.0}")),
                            );
                        }
                    }
                    body_rows = body_rows.child(tr_el);
                }

                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .child(head_row)
                    .child(body_rows)
                    .into_any_element()
            }
        };

        // Interactive segment legend footer (`pro_tactile` chips + `Slide::snappy` dim/highlight)
        let footer = if self.segment_key.as_ref() != "none" {
            let mut legend_strip = div().flex().flex_wrap().items_center().gap(px(4.));
            for (g_idx, g_name) in segments.iter().enumerate() {
                let hidden = self.hidden_segments.contains(g_name);
                let sw = self.segment_color(g_name, g_idx, &p);
                let is_hov = self.hovered_segment.as_ref() == Some(g_name);
                let g_hov = g_name.clone();
                let g_tog = g_name.to_string();
                let chip_alpha = if hidden {
                    0.42
                } else {
                    (0.45 + 0.55 * seg_alphas[g_idx]).clamp(0.45, 1.0)
                };

                legend_strip = legend_strip.child(
                    pro_tactile(
                        pro_eid(&self.id, format!("leg-chip-{g_idx}")),
                        format!("Toggle {g_name}"),
                        6.,
                        transparent_black(),
                        p.hover,
                        transparent_black(),
                        sw.opacity(0.32),
                        is_hov && !hidden,
                        div()
                            .h(px(26.))
                            .px(px(8.))
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .whitespace_nowrap()
                            .child(div().size(px(8.)).flex_none().rounded(px(2.)).bg(sw))
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(if is_hov { p.ink } else { p.text })
                                    .child(g_name.clone()),
                            ),
                        cx,
                        move |this, _, cx| {
                            this.toggle_segment_visibility(&g_tog, cx);
                        },
                    )
                    .flex_none()
                    .opacity(chip_alpha)
                    .on_hover(cx.listener(
                        move |this, inside: &bool, _, cx| {
                            if *inside {
                                this.hovered_segment = Some(g_hov.clone());
                            } else if this.hovered_segment.as_ref() == Some(&g_hov) {
                                this.hovered_segment = None;
                            }
                            cx.notify();
                        },
                    )),
                );
            }

            let has_hidden = !self.hidden_segments.is_empty();
            div()
                .w_full()
                .min_h(px(42.))
                .px(px(10.))
                .py(px(6.))
                .border_t_1()
                .border_color(p.line)
                .flex()
                .items_center()
                .justify_between()
                .gap(px(8.))
                .child(legend_strip)
                .when(has_hidden, |d| {
                    d.child(
                        pro_tactile(
                            pro_eid(&self.id, "leg-reset"),
                            tr(cx, "Show all segments", "恢复全部"),
                            6.,
                            transparent_black(),
                            p.hover,
                            transparent_black(),
                            p.blue.opacity(0.28),
                            false,
                            div()
                                .h(px(26.))
                                .px(px(9.))
                                .flex()
                                .items_center()
                                .text_size(px(12.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.blue)
                                .whitespace_nowrap()
                                .child(tr(cx, "Reset", "重置")),
                            cx,
                            |this, _, cx| {
                                this.reset_hidden_segments(cx);
                            },
                        )
                        .flex_none(),
                    )
                })
                .into_any_element()
        } else {
            div().into_any_element()
        };

        div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _, cx| {
                if ev.keystroke.key == "escape" && this.picker.is_some() {
                    this.picker = None;
                    cx.notify();
                }
            }))
            .w_full()
            .relative()
            .flex()
            .flex_col()
            .child(sensor)
            .child(
                div()
                    .w_full()
                    .max_w(px(580.))
                    .mx_auto()
                    .rounded(px(10.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .shadow(pro_card_shadow(p.dark))
                    .flex()
                    .flex_col()
                    .child(header)
                    .child(controls_row)
                    .child(chart_section)
                    .child(table_toolbar)
                    .child(linked_table)
                    .child(footer),
            )
            .into_any_element()
    }
}

fn nice_axis_ceiling(raw: f64) -> f64 {
    if raw <= 10.0 {
        return 10.0;
    }
    let exp = raw.log10().floor();
    let mag = 10.0_f64.powf(exp);
    let norm = raw / mag;
    let step = if norm <= 1.2 {
        1.2
    } else if norm <= 1.5 {
        1.5
    } else if norm <= 2.0 {
        2.0
    } else if norm <= 2.5 {
        2.5
    } else if norm <= 5.0 {
        5.0
    } else {
        10.0
    };
    (step * mag).ceil()
}

fn default_measures() -> Vec<BreakdownMeasure> {
    vec![
        BreakdownMeasure::count("count", "Task count", "tasks").with_zh("任务数量", "项"),
        BreakdownMeasure::sum("points", "Story points", "pts", "points").with_zh("故事点数", "点"),
        BreakdownMeasure::sum("hours", "Cycle hours", "hrs", "hours").with_zh("执行工时", "小时"),
    ]
}

fn default_dimensions() -> Vec<BreakdownDimension> {
    vec![
        BreakdownDimension::new(
            "status",
            "Status",
            [
                "Backlog",
                "Todo",
                "In progress",
                "In review",
                "Done",
                "Canceled",
            ],
        )
        .with_label_zh("状态"),
        BreakdownDimension::new(
            "priority",
            "Priority",
            ["Urgent", "High", "Medium", "Low", "None"],
        )
        .with_label_zh("优先级")
        .with_color("Urgent", rgb(0xf04438).into())
        .with_color("High", rgb(0xe8b000).into())
        .with_color("Medium", rgb(0x0190fe).into())
        .with_color("Low", rgb(0x6cbcf8).into()),
        BreakdownDimension::new("team", "Team", ["Core UI", "Runtime", "Platform", "Growth"])
            .with_label_zh("团队"),
    ]
}

fn default_sprint_records() -> Vec<BreakdownRecord> {
    // Exact 212-record Status × Priority distribution matching reference DOM
    let table: [(&str, [usize; 5]); 6] = [
        ("Backlog", [3, 6, 6, 8, 3]),
        ("Todo", [1, 7, 12, 11, 5]),
        ("In progress", [1, 9, 12, 7, 5]),
        ("In review", [0, 1, 6, 4, 2]),
        ("Done", [7, 14, 23, 25, 27]),
        ("Canceled", [0, 1, 3, 1, 2]),
    ];
    let priorities = ["Urgent", "High", "Medium", "Low", "None"];
    let teams = ["Core UI", "Runtime", "Platform", "Growth"];
    let pts_by_prio = [8.0, 5.0, 3.0, 2.0, 1.0];
    let hrs_by_prio = [14.0, 9.0, 6.0, 3.0, 2.0];

    let mut out = Vec::with_capacity(212);
    let mut seq = 0usize;
    for (status, counts) in table {
        for (p_idx, &cnt) in counts.iter().enumerate() {
            for _ in 0..cnt {
                let team = teams[seq % teams.len()];
                seq += 1;
                out.push(
                    BreakdownRecord::new()
                        .with_field("status", status)
                        .with_field("priority", priorities[p_idx])
                        .with_field("team", team)
                        .with_metric("points", pts_by_prio[p_idx])
                        .with_metric("hours", hrs_by_prio[p_idx]),
                );
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::BreakdownChart;
    use gpui::{AppContext, TestAppContext, px, size};
    use std::time::Duration;

    #[gpui::test]
    fn sprint_dataset_aggregates_212_tasks_and_switches_dimensions_cleanly(
        cx: &mut TestAppContext,
    ) {
        let chart = cx.new(|cx| BreakdownChart::new("test-breakdown-chart", cx));

        chart.update(cx, |bc, cx| {
            let (slices, segments, matrix, grand) = bc.compute_matrix();
            assert_eq!(slices.len(), 6);
            assert_eq!(segments.len(), 5);
            assert!((grand - 212.0).abs() < 1e-6);
            // Done slice has 96 tasks total
            let done_sum: f64 = matrix[4].iter().sum();
            assert!((done_sum - 96.0).abs() < 1e-6);

            // Hiding Urgent (3+1+1+0+7+0 = 12) reduces total to 200
            bc.toggle_segment_visibility("Urgent", cx);
            let (_, _, _, after_hide) = bc.compute_matrix();
            assert!((after_hide - 200.0).abs() < 1e-6);

            bc.reset_hidden_segments(cx);
            assert!(bc.set_measure("points", cx));
            assert!(bc.set_slice("team", cx));
            let (team_slices, _, _, pts_total) = bc.compute_matrix();
            assert_eq!(team_slices.len(), 4);
            assert!(pts_total > 212.0);
        });
    }

    #[gpui::test]
    fn switching_slice_springs_positional_bars_smoothly(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let exec = cx.executor();
        let (chart, w) = cx.add_window_view(|_, cx| BreakdownChart::new("bc-spring", cx));
        w.simulate_resize(size(px(600.), px(520.)));
        w.run_until_parked();

        let h0 = w.update(|_, app| chart.read(app).bar_slides[0][0].value(app));
        w.update(|_, app| {
            chart.update(app, |bc, cx| {
                bc.set_measure("points", cx);
                bc.set_slice("team", cx);
                bc.hovered_segment = Some("Urgent".into());
            });
        });
        w.run_until_parked();

        exec.advance_clock(Duration::from_millis(70));
        w.update(|win, app| win.simulate_next_frame(app));
        w.run_until_parked();

        w.update(|_, app| {
            let bc = chart.read(app);
            let h_mid = bc.bar_slides[0][0].value(app);
            assert!(
                (h_mid - h0).abs() > 0.1,
                "expected positional bar slide to morph smoothly, got {h0} -> {h_mid}"
            );
            let dim_seg = bc.seg_spotlights[1].value(app);
            assert!(dim_seg < 0.95 && dim_seg >= 0.22);
        });
    }
}
