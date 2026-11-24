//! Focused human-in-the-loop review workspace for agent actions, diffs, plans, and drafts.
//!
//! Sorts pending agent proposals by risk (`High`, `Medium`, `Low`) in a left queue while
//! presenting one item at a time on the right with uncertain amber spans, `Agent's note`
//! inspection, inline [`Editor`] editing, single-key shortcuts (`A`, `E`, `R`, `S`, `F`, `Z`),
//! and an animated resolution stamp as decided cards yield to the next item.

use crate::{
    components::pro_chrome::{
        ProBtnStyle, ProPalette, ProSelectionSensor, pro_agent_avatar, pro_app_tile, pro_btn,
        pro_ease, pro_eid, pro_empty_state, pro_mark_selected, pro_pill_indicator, pro_presence,
        pro_rolling_number, pro_selection_sensor, pro_skeleton_bar, pro_spring_bar,
        pro_toast_banner, tr,
    },
    foundation::Theme,
    primitives::{
        control::UiLanguage,
        editor::{EditEvent, Editor, InputMode},
        family::{PhaseClock, Slide, disabled_motion},
        pro_icons::{ProIcon, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, cmp::Ordering, rc::Rc};
use web_time::Instant;

const SEAL_BADGE_SVG: &str = r#"<path d="M18.99 19H19M18.99 19c-.62.62-1.75.46-2.54.46-.97 0-1.44.19-2.13.88-.59.59-1.38 1.66-2.32 1.66-.93 0-1.72-1.07-2.31-1.66-.69-.69-1.16-.88-2.13-.88-.79 0-1.92.16-2.54-.46-.63-.62-.47-1.75-.47-2.55 0-1.01-.22-1.47-.94-2.19C2.53 13.19 2 12.66 2 12s.53-1.19 1.61-2.26c.72-.72.94-1.18.94-2.19 0-.8-.16-1.93.47-2.55.62-.62 1.75-.46 2.54-.46.97 0 1.44-.19 2.13-.88C10.28 3.07 11.07 2 12 2s1.72 1.07 2.31 1.66c.69.69 1.16.88 2.13.88.79 0 1.92-.16 2.54.46.63.62.47 1.75.47 2.55 0 1.01.22 1.47.94 2.19C21.47 10.81 22 11.34 22 12s-.53 1.19-1.61 2.26c-.72.72-.94 1.18-.94 2.19 0 .8.16 1.93-.46 2.55Z" stroke="currentColor" stroke-width="1.85" stroke-linecap="round" stroke-linejoin="round"></path><path d="m9 12.3 2 2 4.2-4.4" stroke="currentColor" stroke-width="1.85" stroke-linecap="round" stroke-linejoin="round"></path>"#;

const PENCIL_EDIT_SVG: &str = r#"<path d="m16.42 4.6 1-.99a2.1 2.1 0 0 1 2.97 2.97l-.99 1m-2.98-2.98-6.66 6.66c-.51.51-.87 1.15-1.04 1.84L8 16l2.9-.72c.69-.18 1.33-.54 1.84-1.04l6.66-6.66m-2.98-2.98 2.98 2.98" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"></path>"#;

/// Risk tier assigned to a [`ReviewItem`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ReviewRisk {
    /// Highest-impact actions sorted to the top of the queue.
    High,
    /// Moderate-impact changes requiring human sign-off.
    #[default]
    Medium,
    /// Routine low-risk actions.
    Low,
}

impl ReviewRisk {
    fn rank(self) -> u8 {
        match self {
            Self::High => 0,
            Self::Medium => 1,
            Self::Low => 2,
        }
    }

    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::High => tr(cx, "High risk", "高风险"),
            Self::Medium => tr(cx, "Medium risk", "中风险"),
            Self::Low => tr(cx, "Low risk", "低风险"),
        }
    }

    fn short_label(self, cx: &App) -> &'static str {
        match self {
            Self::High => tr(cx, "High", "高"),
            Self::Medium => tr(cx, "Med", "中"),
            Self::Low => tr(cx, "Low", "低"),
        }
    }
}

/// Queue filter tab in [`ReviewCenter`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ReviewFilter {
    /// Show every undecided item in the queue.
    #[default]
    All,
    /// Show only [`ReviewRisk::High`] items.
    HighRisk,
    /// Show items whose confidence is below `low_confidence` (`0.70` default) or have flagged spans.
    Unsure,
}

impl ReviewFilter {
    fn key(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::HighRisk => "high",
            Self::Unsure => "unsure",
        }
    }
}

/// Queue sort strategy for [`ReviewCenter`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ReviewOrder {
    /// Sort `High` -> `Medium` -> `Low`, breaking ties by longest wait first.
    #[default]
    Risk,
    /// Sort oldest waiting items first regardless of risk.
    Oldest,
    /// Preserve caller slice order.
    Given,
}

/// Final reviewer verdict on a [`ReviewItem`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReviewDecisionKind {
    /// Proposal approved (optionally with inline edits).
    Approved,
    /// Proposal rejected with an optional reason.
    Rejected,
}

/// Body rendering format for a [`ReviewItem`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ReviewFormat {
    /// Rich prose with left quotation rule and inline uncertain flags.
    #[default]
    Text,
    /// Monospace unified diff / code snippet.
    Code,
}

/// Inline text run or amber-highlighted uncertain span inside a [`ReviewItem`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewSpan {
    /// Display text of this span.
    pub text: SharedString,
    /// When `Some`, marks this span in amber and shows the agent's note on hover or `F`.
    pub note: Option<SharedString>,
    /// Optional policy or file citation shown inside the `Agent's note` card.
    pub source: Option<SharedString>,
}

impl ReviewSpan {
    /// Construct a plain unflagged text span.
    pub fn plain(text: impl Into<SharedString>) -> Self {
        Self {
            text: text.into(),
            note: None,
            source: None,
        }
    }

    /// Construct an uncertain/shaky span highlighted in amber with an explanatory `note`.
    pub fn flagged(
        text: impl Into<SharedString>,
        note: impl Into<SharedString>,
        source: Option<SharedString>,
    ) -> Self {
        Self {
            text: text.into(),
            note: Some(note.into()),
            source,
        }
    }

    /// Return `true` when this span carries an agent uncertainty note.
    pub fn is_flagged(&self) -> bool {
        self.note.is_some()
    }
}

/// Structured key-value metadata field inside a [`ReviewItem`] proposal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewField {
    /// Field heading (for example `"Amount"` or `"Order"`).
    pub label: SharedString,
    /// Inline plain and/or flagged spans composing the field value.
    pub spans: Vec<ReviewSpan>,
}

impl ReviewField {
    /// Construct a plain key-value field.
    pub fn plain(label: impl Into<SharedString>, value: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            spans: vec![ReviewSpan::plain(value)],
        }
    }

    /// Construct a field containing inline [`ReviewSpan`] runs.
    pub fn with_spans(label: impl Into<SharedString>, spans: Vec<ReviewSpan>) -> Self {
        Self {
            label: label.into(),
            spans,
        }
    }

    /// Plain concatenated string of all spans in this field.
    pub fn plain_text(&self) -> String {
        self.spans.iter().map(|s| s.text.as_ref()).collect()
    }
}

/// Line change kind in a [`ReviewFile`] unified diff.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ReviewDiffKind {
    /// Unchanged context line.
    #[default]
    Context,
    /// Added line (`+`).
    Add,
    /// Removed line (`-`).
    Del,
}

/// Single numbered line in a [`ReviewFile`] diff.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewDiffLine {
    /// Context, addition, or deletion marker.
    pub kind: ReviewDiffKind,
    /// 1-based line number displayed in the left gutter.
    pub line_no: usize,
    /// Code runs on this line (may include amber flagged expressions).
    pub spans: Vec<ReviewSpan>,
}

impl ReviewDiffLine {
    /// Create a plain diff line.
    pub fn new(kind: ReviewDiffKind, line_no: usize, code: impl Into<SharedString>) -> Self {
        Self {
            kind,
            line_no,
            spans: vec![ReviewSpan::plain(code)],
        }
    }

    /// Create a diff line containing flagged expressions.
    pub fn with_spans(kind: ReviewDiffKind, line_no: usize, spans: Vec<ReviewSpan>) -> Self {
        Self {
            kind,
            line_no,
            spans,
        }
    }
}

/// Code file change inspected inside [`ReviewCenter`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewFile {
    /// Repository-relative file path (for example `"src/queue/worker.ts"`).
    pub path: SharedString,
    /// Added line count (`+N`).
    pub added: usize,
    /// Removed line count (`-N`).
    pub removed: usize,
    /// Unified diff lines.
    pub lines: Vec<ReviewDiffLine>,
}

impl ReviewFile {
    /// Create a [`ReviewFile`] with `path`, `added`, `removed`, and `lines`.
    pub fn new(
        path: impl Into<SharedString>,
        added: usize,
        removed: usize,
        lines: Vec<ReviewDiffLine>,
    ) -> Self {
        Self {
            path: path.into(),
            added,
            removed,
            lines,
        }
    }
}

/// Numbered execution step in a plan-style [`ReviewItem`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewPlanStep {
    /// Step title.
    pub title: SharedString,
    /// Optional tool badge (such as `"stripe.keys.roll"`).
    pub tool: Option<SharedString>,
    /// Optional secondary detail line.
    pub detail: Option<SharedString>,
    /// Optional uncertainty note; when present, numbers the step badge in amber.
    pub flag_note: Option<SharedString>,
}

impl ReviewPlanStep {
    /// Construct a numbered plan step.
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            tool: None,
            detail: None,
            flag_note: None,
        }
    }

    /// Set the tool label displayed next to the step title.
    pub fn with_tool(mut self, tool: impl Into<SharedString>) -> Self {
        self.tool = Some(tool.into());
        self
    }

    /// Set secondary detail copy underneath the step title.
    pub fn with_detail(mut self, detail: impl Into<SharedString>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Flag this step as uncertain with an explanatory `note`.
    pub fn with_flag(mut self, note: impl Into<SharedString>) -> Self {
        self.flag_note = Some(note.into());
        self
    }
}

/// Grounding reference pill displayed in the footer of a [`ReviewItem`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewSource {
    /// Display label (for example `"PR #511"` or `"Refund policy §4"`).
    pub label: SharedString,
    /// Optional brand key (such as `"github"`, `"stripe"`, or `"slack"`).
    pub app: Option<SharedString>,
}

impl ReviewSource {
    /// Construct a footer source pill.
    pub fn new(label: impl Into<SharedString>, app: Option<SharedString>) -> Self {
        Self {
            label: label.into(),
            app,
        }
    }
}

/// Proposal awaiting human sign-off in [`ReviewCenter`].
#[derive(Clone, Debug, PartialEq)]
pub struct ReviewItem {
    /// Stable item identifier.
    pub id: SharedString,
    /// Action headline.
    pub title: SharedString,
    /// Proposing agent name.
    pub agent: SharedString,
    /// Model identifier shown beside the agent.
    pub model: SharedString,
    /// Optional connected app mark (`"github"`, `"stripe"`, `"slack"`, etc.).
    pub app: Option<SharedString>,
    /// Short category label (`"Code change"`, `"Refund"`, `"Migration"`, `"Plan"`, etc.).
    pub kind_label: SharedString,
    /// Risk tier (`High`, `Medium`, `Low`).
    pub risk: ReviewRisk,
    /// Agent self-reported confidence in `0.0..=1.0`.
    pub confidence: f32,
    /// Minutes this item has been waiting in the queue.
    pub waiting_mins: u32,
    /// Agent's concise explanation of *why* it proposes this action.
    pub reasoning: Option<SharedString>,
    /// Key-value fields (may contain flagged spans).
    pub fields: Vec<ReviewField>,
    /// Optional heading above the prose/code body (for example `"Reply to Maya"`).
    pub body_label: Option<SharedString>,
    /// Body spans (editable in place via `E`).
    pub body: Vec<ReviewSpan>,
    /// Rendering style for `body`.
    pub format: ReviewFormat,
    /// Unified diff files for code-review items.
    pub files: Vec<ReviewFile>,
    /// Ordered plan steps for plan-review items.
    pub steps: Vec<ReviewPlanStep>,
    /// Footer reference chips.
    pub sources: Vec<ReviewSource>,
}

impl ReviewItem {
    /// Construct a new [`ReviewItem`] with sensible defaults.
    pub fn new(id: impl Into<SharedString>, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            agent: "Support agent".into(),
            model: "Sonnet 5.5".into(),
            app: None,
            kind_label: "Action".into(),
            risk: ReviewRisk::Medium,
            confidence: 0.80,
            waiting_mins: 5,
            reasoning: None,
            fields: Vec::new(),
            body_label: None,
            body: Vec::new(),
            format: ReviewFormat::Text,
            files: Vec::new(),
            steps: Vec::new(),
            sources: Vec::new(),
        }
    }

    /// Set the proposing agent name and model label.
    pub fn with_agent(
        mut self,
        agent: impl Into<SharedString>,
        model: impl Into<SharedString>,
    ) -> Self {
        self.agent = agent.into();
        self.model = model.into();
        self
    }

    /// Set an optional connected app icon (`"github"`, `"stripe"`, `"slack"`, etc.).
    pub fn with_app(mut self, app: impl Into<SharedString>) -> Self {
        self.app = Some(app.into());
        self
    }

    /// Set the short proposal category label (`"Code change"`, `"Refund"`, etc.).
    pub fn with_kind_label(mut self, label: impl Into<SharedString>) -> Self {
        self.kind_label = label.into();
        self
    }

    /// Set the risk level (`High`, `Medium`, `Low`).
    pub fn with_risk(mut self, risk: ReviewRisk) -> Self {
        self.risk = risk;
        self
    }

    /// Set agent confidence (`0.0..=1.0`).
    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    /// Set how many minutes this item has been waiting in the queue.
    pub fn with_waiting_mins(mut self, mins: u32) -> Self {
        self.waiting_mins = mins;
        self
    }

    /// Set the `"Why"` reasoning sentence.
    pub fn with_reasoning(mut self, reasoning: impl Into<SharedString>) -> Self {
        self.reasoning = Some(reasoning.into());
        self
    }

    /// Attach key-value proposal fields.
    pub fn with_fields(mut self, fields: Vec<ReviewField>) -> Self {
        self.fields = fields;
        self
    }

    /// Attach prose or code body spans with an optional section label.
    pub fn with_body(mut self, label: Option<SharedString>, spans: Vec<ReviewSpan>) -> Self {
        self.body_label = label;
        self.body = spans;
        self
    }

    /// Set the body format (`Text` vs `Code`).
    pub fn with_format(mut self, format: ReviewFormat) -> Self {
        self.format = format;
        self
    }

    /// Attach unified diff files.
    pub fn with_files(mut self, files: Vec<ReviewFile>) -> Self {
        self.files = files;
        self
    }

    /// Attach numbered plan steps.
    pub fn with_steps(mut self, steps: Vec<ReviewPlanStep>) -> Self {
        self.steps = steps;
        self
    }

    /// Attach footer sources.
    pub fn with_sources(mut self, sources: Vec<ReviewSource>) -> Self {
        self.sources = sources;
        self
    }

    /// Collect all flagged `(text, note, source)` notes on this item in reading order.
    pub fn collect_flags(&self) -> Vec<(SharedString, SharedString, Option<SharedString>)> {
        let mut out = Vec::new();
        for f in &self.fields {
            for s in &f.spans {
                if let Some(note) = &s.note {
                    out.push((s.text.clone(), note.clone(), s.source.clone()));
                }
            }
        }
        for s in &self.body {
            if let Some(note) = &s.note {
                out.push((s.text.clone(), note.clone(), s.source.clone()));
            }
        }
        for file in &self.files {
            for line in &file.lines {
                for s in &line.spans {
                    if let Some(note) = &s.note {
                        let src = s.source.clone().or_else(|| Some(file.path.clone()));
                        out.push((s.text.clone(), note.clone(), src));
                    }
                }
            }
        }
        for step in &self.steps {
            if let Some(note) = &step.flag_note {
                out.push((step.title.clone(), note.clone(), step.tool.clone()));
            }
        }
        out
    }

    /// Plain text concatenation of `body`.
    pub fn body_plain_text(&self) -> String {
        self.body.iter().map(|s| s.text.as_ref()).collect()
    }

    /// Editable draft representation combining `body` (or first field / diff summary when body is empty).
    pub fn editable_seed(&self) -> String {
        if !self.body.is_empty() {
            return self.body_plain_text();
        }
        if !self.fields.is_empty() {
            return self
                .fields
                .iter()
                .map(|f| format!("{}: {}", f.label, f.plain_text()))
                .collect::<Vec<_>>()
                .join("\n");
        }
        if let Some(first_file) = self.files.first() {
            return first_file
                .lines
                .iter()
                .filter(|l| l.kind != ReviewDiffKind::Del)
                .map(|l| l.spans.iter().map(|s| s.text.as_ref()).collect::<String>())
                .collect::<Vec<_>>()
                .join("\n");
        }
        self.reasoning
            .as_ref()
            .map(|s| s.to_string())
            .unwrap_or_else(|| self.title.to_string())
    }
}

/// Decision payload emitted when a reviewer approves or rejects a [`ReviewItem`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewDecisionResult {
    /// Approved or rejected verdict.
    pub decision: ReviewDecisionKind,
    /// Rejection reason (when `decision == Rejected`).
    pub reason: Option<SharedString>,
    /// Edited proposal text when approved after inline editing (`E`).
    pub edited_text: Option<SharedString>,
}

/// Typed events emitted by [`ReviewCenter`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReviewCenterEvent {
    /// Emitted when an item is approved or rejected.
    Decided {
        /// Item id.
        id: SharedString,
        /// Verdict, optional reason, and optional inline edits.
        result: ReviewDecisionResult,
    },
    /// Emitted when the reviewer skips an item (`S`).
    Skipped(SharedString),
    /// Emitted when the last decision is undone (`Z` or `Cmd+Z`).
    Undone {
        /// Restored item id.
        id: SharedString,
        /// Reverted verdict.
        decision: ReviewDecisionKind,
    },
    /// Emitted when queue selection moves to another item.
    Selected(SharedString),
    /// Emitted when the queue filter tab changes.
    FilterChanged(ReviewFilter),
}

impl EventEmitter<ReviewCenterEvent> for ReviewCenter {}

#[derive(Clone)]
struct DecidedRecord {
    id: SharedString,
    title: SharedString,
    decision: ReviewDecisionKind,
    reason: Option<SharedString>,
    previous_edit: Option<SharedString>,
    decided_at: Instant,
}

#[derive(Clone)]
struct StampGhost {
    item: ReviewItem,
    decision: ReviewDecisionKind,
    reason: Option<SharedString>,
    started_at: Instant,
}

/// Focused human-in-the-loop review workspace for agent proposals.
pub struct ReviewCenter {
    id: ElementId,
    focus: FocusHandle,
    title: Option<SharedString>,
    items: Vec<ReviewItem>,
    base_reviewed: usize,
    sla_mins: u32,
    low_confidence: f32,
    order: ReviewOrder,
    filter: ReviewFilter,
    reasons: Vec<SharedString>,
    selected_id: Option<SharedString>,
    decided: Vec<DecidedRecord>,
    skipped_ids: Vec<SharedString>,
    edited_overrides: Vec<(SharedString, SharedString)>,
    active_flag_idx: Option<usize>,
    reject_picker_open: bool,
    editing: bool,
    editor: Option<Entity<Editor>>,
    editor_focused: bool,
    stamp_ghost: Option<StampGhost>,
    card_clock: PhaseClock,
    card_slide: Slide,
    card_dir: f32,
    filter_sensor: ProSelectionSensor,
    loading: bool,
    empty: bool,
    measured_width: Rc<Cell<f32>>,
    toast: Option<(SharedString, Instant)>,
}

impl ReviewCenter {
    /// Create a [`ReviewCenter`] preloaded with the 8 canonical agent review items (`24 of 32 reviewed`).
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<UiLanguage>(|_, cx| cx.notify())
            .detach();
        let items = default_review_items();
        let first_id = items.first().map(|i| i.id.clone());
        Self {
            id: id.into(),
            focus: cx.focus_handle(),
            title: None,
            items,
            base_reviewed: 24,
            sla_mins: 30,
            low_confidence: 0.70,
            order: ReviewOrder::Risk,
            filter: ReviewFilter::All,
            reasons: vec![
                "Wrong info".into(),
                "Tone".into(),
                "Not allowed".into(),
                "Other".into(),
            ],
            selected_id: first_id,
            decided: Vec::new(),
            skipped_ids: Vec::new(),
            edited_overrides: Vec::new(),
            active_flag_idx: Some(0),
            reject_picker_open: false,
            editing: false,
            editor: None,
            editor_focused: false,
            stamp_ghost: None,
            card_clock: PhaseClock::default(),
            card_slide: Slide::new(0.0),
            card_dir: 1.0,
            filter_sensor: pro_selection_sensor(),
            loading: false,
            empty: false,
            measured_width: Rc::new(Cell::new(780.0)),
            toast: None,
        }
    }

    /// Override the workspace header title (defaults to `"Review center"`).
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Replace the review queue items.
    pub fn with_items(mut self, items: Vec<ReviewItem>) -> Self {
        self.items = items;
        self.selected_id = self
            .visible_indices()
            .first()
            .map(|&idx| self.items[idx].id.clone());
        self.active_flag_idx = self
            .current_item()
            .and_then(|it| (!it.collect_flags().is_empty()).then_some(0));
        self
    }

    /// Set the count of items already reviewed earlier in the session (`24` default).
    pub fn with_reviewed(mut self, reviewed: usize) -> Self {
        self.base_reviewed = reviewed;
        self
    }

    /// Set the SLA threshold in minutes (`30` default; turns amber at 75%, red at 100%).
    pub fn with_sla_mins(mut self, sla_mins: u32) -> Self {
        self.sla_mins = sla_mins.max(1);
        self
    }

    /// Set the confidence cutoff below which an item counts under `Unsure` (`0.70` default).
    pub fn with_low_confidence(mut self, cutoff: f32) -> Self {
        self.low_confidence = cutoff.clamp(0.05, 0.99);
        self
    }

    /// Set the queue sort order (`Risk`, `Oldest`, or `Given`).
    pub fn with_order(mut self, order: ReviewOrder) -> Self {
        self.order = order;
        self.selected_id = self
            .visible_indices()
            .first()
            .map(|&idx| self.items[idx].id.clone());
        self
    }

    /// Set the initial queue filter (`All`, `HighRisk`, `Unsure`).
    pub fn with_default_filter(mut self, filter: ReviewFilter) -> Self {
        self.filter = filter;
        self.selected_id = self
            .visible_indices()
            .first()
            .map(|&idx| self.items[idx].id.clone());
        self
    }

    /// Override the quick rejection reason pills bound to keys `1..=4`.
    pub fn with_reasons(mut self, reasons: Vec<SharedString>) -> Self {
        if !reasons.is_empty() {
            self.reasons = reasons;
        }
        self
    }

    /// Toggle loading skeleton state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Toggle empty queue state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Return the active [`ReviewFilter`].
    pub fn filter(&self) -> ReviewFilter {
        self.filter
    }

    /// Return the total reviewed count (`base_reviewed + decided`).
    pub fn reviewed_count(&self) -> usize {
        self.base_reviewed + self.decided.len()
    }

    /// Return the number of undecided items remaining across all filters.
    pub fn waiting_count(&self) -> usize {
        self.items
            .iter()
            .filter(|it| !self.is_decided(&it.id))
            .count()
    }

    fn is_decided(&self, id: &SharedString) -> bool {
        self.decided.iter().any(|d| &d.id == id)
    }

    fn is_skipped(&self, id: &SharedString) -> bool {
        self.skipped_ids.iter().any(|s| s == id)
    }

    fn is_unsure(&self, item: &ReviewItem) -> bool {
        item.confidence < self.low_confidence || !item.collect_flags().is_empty()
    }

    fn sorted_undecided_indices(&self) -> Vec<usize> {
        let mut indices: Vec<usize> = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, it)| !self.is_decided(&it.id))
            .map(|(idx, _)| idx)
            .collect();
        match self.order {
            ReviewOrder::Risk => {
                indices.sort_by(|&a, &b| {
                    let ia = &self.items[a];
                    let ib = &self.items[b];
                    match ia.risk.rank().cmp(&ib.risk.rank()) {
                        Ordering::Equal => ib.waiting_mins.cmp(&ia.waiting_mins),
                        other => other,
                    }
                });
            }
            ReviewOrder::Oldest => {
                indices
                    .sort_by(|&a, &b| self.items[b].waiting_mins.cmp(&self.items[a].waiting_mins));
            }
            ReviewOrder::Given => {}
        }
        indices
    }

    fn visible_indices(&self) -> Vec<usize> {
        self.sorted_undecided_indices()
            .into_iter()
            .filter(|&idx| {
                let it = &self.items[idx];
                match self.filter {
                    ReviewFilter::All => true,
                    ReviewFilter::HighRisk => it.risk == ReviewRisk::High,
                    ReviewFilter::Unsure => self.is_unsure(it),
                }
            })
            .collect()
    }

    fn current_item_index(&self) -> Option<usize> {
        let vis = self.visible_indices();
        if vis.is_empty() {
            return None;
        }
        if let Some(sel) = &self.selected_id
            && let Some(&found) = vis.iter().find(|&&i| &self.items[i].id == sel)
        {
            return Some(found);
        }
        vis.first().copied()
    }

    fn current_item(&self) -> Option<&ReviewItem> {
        self.current_item_index().map(|i| &self.items[i])
    }

    fn edited_text_for(&self, id: &SharedString) -> Option<&SharedString> {
        self.edited_overrides
            .iter()
            .find(|(k, _)| k == id)
            .map(|(_, v)| v)
    }

    /// Set the active [`ReviewFilter`] and emit [`ReviewCenterEvent::FilterChanged`].
    pub fn set_filter(&mut self, next: ReviewFilter, cx: &mut Context<Self>) {
        if self.filter != next {
            self.filter = next;
            self.editing = false;
            self.reject_picker_open = false;
            let vis = self.visible_indices();
            if let Some(sel) = &self.selected_id {
                if !vis.iter().any(|&i| &self.items[i].id == sel) {
                    self.selected_id = vis.first().map(|&i| self.items[i].id.clone());
                }
            } else {
                self.selected_id = vis.first().map(|&i| self.items[i].id.clone());
            }
            self.sync_default_flag();
            self.card_dir = 1.0;
            self.card_slide.reset(12.0);
            self.card_slide.duration(0.0, 0.28, cx);
            self.card_clock.restart(cx);
            cx.emit(ReviewCenterEvent::FilterChanged(next));
            cx.notify();
        }
    }

    fn sync_default_flag(&mut self) {
        self.active_flag_idx = self.current_item().and_then(|it| {
            if it.collect_flags().is_empty() {
                None
            } else {
                Some(0)
            }
        });
    }

    /// Select a specific item by `id`.
    pub fn select_item(&mut self, id: SharedString, dir: f32, cx: &mut Context<Self>) {
        if self.selected_id.as_ref() != Some(&id) {
            self.selected_id = Some(id.clone());
            self.editing = false;
            self.reject_picker_open = false;
            self.sync_default_flag();
            self.card_dir = dir;
            self.card_slide.reset(12.0 * dir);
            self.card_slide.duration(0.0, 0.26, cx);
            self.card_clock.restart(cx);
            cx.emit(ReviewCenterEvent::Selected(id));
            cx.notify();
        }
    }

    fn step_selection(&mut self, delta: isize, cx: &mut Context<Self>) {
        let vis = self.visible_indices();
        if vis.is_empty() {
            return;
        }
        let cur_pos = self
            .current_item_index()
            .and_then(|ci| vis.iter().position(|&x| x == ci))
            .unwrap_or(0);
        let next_pos = if delta >= 0 {
            (cur_pos + 1).min(vis.len() - 1)
        } else {
            cur_pos.saturating_sub(1)
        };
        if next_pos != cur_pos {
            let next_id = self.items[vis[next_pos]].id.clone();
            self.select_item(next_id, if delta >= 0 { 1.0 } else { -1.0 }, cx);
        }
    }

    fn open_inline_editor(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let Some(item) = self.current_item().cloned() else {
            return;
        };
        let seed = self
            .edited_text_for(&item.id)
            .map(|s| s.to_string())
            .unwrap_or_else(|| item.editable_seed());
        let ph: SharedString = tr(
            cx,
            "Refine proposal before approving…",
            "在批准前微调提案内容…",
        )
        .into();
        let ed = cx.new(|cx| {
            let mut e = Editor::new(ph.clone(), InputMode::Multi, w, cx);
            e.set_placeholder(ph);
            e.font_size = 13.0;
            e.set_value(seed, w, cx);
            e
        });
        cx.subscribe_in(&ed, w, |this, ed_ent, ev: &EditEvent, w, cx| match ev {
            EditEvent::Focused => {
                this.editor_focused = true;
                cx.notify();
            }
            EditEvent::Blurred => {
                this.editor_focused = false;
                cx.notify();
            }
            EditEvent::Submitted => {
                this.save_edit_and_approve(w, cx);
            }
            EditEvent::Escape => {
                this.editing = false;
                this.editor_focused = false;
                this.focus.focus(w, cx);
                cx.notify();
            }
            EditEvent::Changed => {
                let _ = ed_ent;
                cx.notify();
            }
            _ => {}
        })
        .detach();
        ed.clone().update(cx, |e, cx| e.focus(w, cx));
        self.editor = Some(ed);
        self.editing = true;
        self.reject_picker_open = false;
        cx.notify();
    }

    fn save_edit_and_approve(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let draft = self
            .editor
            .as_ref()
            .map(|e| e.read(cx).value().trim().to_owned())
            .unwrap_or_default();
        self.editing = false;
        self.editor_focused = false;
        self.focus.focus(w, cx);
        let edited_opt = (!draft.is_empty()).then(|| SharedString::from(draft));
        self.commit_decision(ReviewDecisionKind::Approved, None, edited_opt, cx);
    }

    /// Approve the currently inspected item (`A`).
    pub fn approve_current(&mut self, cx: &mut Context<Self>) {
        let existing_edit = self
            .current_item()
            .and_then(|it| self.edited_text_for(&it.id).cloned());
        self.commit_decision(ReviewDecisionKind::Approved, None, existing_edit, cx);
    }

    /// Reject the currently inspected item with `reason` (`R` then `1..=4`).
    pub fn reject_current(&mut self, reason: Option<SharedString>, cx: &mut Context<Self>) {
        self.commit_decision(ReviewDecisionKind::Rejected, reason, None, cx);
    }

    fn commit_decision(
        &mut self,
        decision: ReviewDecisionKind,
        reason: Option<SharedString>,
        edited_text: Option<SharedString>,
        cx: &mut Context<Self>,
    ) {
        let vis = self.visible_indices();
        let Some(cur_idx) = self.current_item_index() else {
            return;
        };
        let cur_pos = vis.iter().position(|&x| x == cur_idx).unwrap_or(0);
        let item = self.items[cur_idx].clone();
        let prev_edit = self
            .edited_overrides
            .iter()
            .find(|(k, _)| k == &item.id)
            .map(|(_, v)| v.clone());
        if let Some(ed) = &edited_text {
            if let Some(slot) = self
                .edited_overrides
                .iter_mut()
                .find(|(k, _)| k == &item.id)
            {
                slot.1 = ed.clone();
            } else {
                self.edited_overrides.push((item.id.clone(), ed.clone()));
            }
        }

        // Compute the next item in the remaining visible list
        let next_id = vis
            .get(cur_pos + 1)
            .or_else(|| cur_pos.checked_sub(1).and_then(|p| vis.get(p)))
            .map(|&idx| self.items[idx].id.clone());

        self.stamp_ghost = Some(StampGhost {
            item: item.clone(),
            decision,
            reason: reason.clone(),
            started_at: Instant::now(),
        });
        self.decided.push(DecidedRecord {
            id: item.id.clone(),
            title: item.title.clone(),
            decision,
            reason: reason.clone(),
            previous_edit: prev_edit,
            decided_at: Instant::now(),
        });
        self.skipped_ids.retain(|s| s != &item.id);
        self.selected_id = next_id;
        self.editing = false;
        self.reject_picker_open = false;
        self.sync_default_flag();
        self.card_dir = 1.0;
        self.card_slide.reset(14.0);
        self.card_slide.duration(0.0, 0.30, cx);
        self.card_clock.restart(cx);

        cx.emit(ReviewCenterEvent::Decided {
            id: item.id,
            result: ReviewDecisionResult {
                decision,
                reason,
                edited_text,
            },
        });
        cx.notify();
    }

    /// Skip the current item without deciding (`S`) and advance to the next item.
    pub fn skip_current(&mut self, cx: &mut Context<Self>) {
        let vis = self.visible_indices();
        let Some(cur_idx) = self.current_item_index() else {
            return;
        };
        let id = self.items[cur_idx].id.clone();
        if !self.is_skipped(&id) {
            self.skipped_ids.push(id.clone());
        }
        if vis.len() > 1 {
            let cur_pos = vis.iter().position(|&x| x == cur_idx).unwrap_or(0);
            let next_pos = (cur_pos + 1) % vis.len();
            self.selected_id = Some(self.items[vis[next_pos]].id.clone());
        }
        self.editing = false;
        self.reject_picker_open = false;
        self.sync_default_flag();
        self.card_dir = 1.0;
        self.card_slide.reset(14.0);
        self.card_slide.duration(0.0, 0.30, cx);
        self.card_clock.restart(cx);
        self.toast = Some((
            tr(cx, "Skipped for later review", "已跳过，稍后复核").into(),
            Instant::now(),
        ));
        cx.emit(ReviewCenterEvent::Skipped(id));
        cx.notify();
    }

    /// Undo the most recent decision (`Z` or `Cmd+Z`).
    pub fn undo_last(&mut self, cx: &mut Context<Self>) {
        let Some(last) = self.decided.pop() else {
            return;
        };
        if let Some(prev) = last.previous_edit {
            if let Some(slot) = self
                .edited_overrides
                .iter_mut()
                .find(|(k, _)| k == &last.id)
            {
                slot.1 = prev;
            }
        } else {
            self.edited_overrides.retain(|(k, _)| k != &last.id);
        }
        self.stamp_ghost = None;
        self.selected_id = Some(last.id.clone());
        self.editing = false;
        self.reject_picker_open = false;
        self.sync_default_flag();
        self.card_dir = -1.0;
        self.card_slide.reset(-12.0);
        self.card_slide.duration(0.0, 0.28, cx);
        self.card_clock.restart(cx);
        cx.emit(ReviewCenterEvent::Undone {
            id: last.id,
            decision: last.decision,
        });
        cx.notify();
    }

    fn cycle_flag(&mut self, cx: &mut Context<Self>) {
        let Some(item) = self.current_item() else {
            return;
        };
        let count = item.collect_flags().len();
        if count == 0 {
            self.active_flag_idx = None;
            return;
        }
        self.active_flag_idx = match self.active_flag_idx {
            Some(i) if i + 1 < count => Some(i + 1),
            Some(_) => None,
            None => Some(0),
        };
        cx.notify();
    }

    fn handle_key_down(&mut self, ev: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        let key = ev.keystroke.key.as_str();
        let cmd = ev.keystroke.modifiers.platform || ev.keystroke.modifiers.control;

        if self.editing {
            if cmd && key == "enter" {
                self.save_edit_and_approve(w, cx);
            } else if key == "escape" {
                self.editing = false;
                self.editor_focused = false;
                self.focus.focus(w, cx);
                cx.notify();
            }
            return;
        }

        if cmd && key.eq_ignore_ascii_case("z") {
            self.undo_last(cx);
            return;
        }
        if cmd || ev.keystroke.modifiers.alt {
            return;
        }

        if self.reject_picker_open {
            match key {
                "1" | "2" | "3" | "4" => {
                    let idx = (key.as_bytes()[0] - b'1') as usize;
                    let reason = self.reasons.get(idx).cloned();
                    self.reject_current(reason, cx);
                    return;
                }
                "escape" => {
                    self.reject_picker_open = false;
                    cx.notify();
                    return;
                }
                _ => {}
            }
        }

        match key {
            "a" | "A" => self.approve_current(cx),
            "e" | "E" => self.open_inline_editor(w, cx),
            "r" | "R" => {
                if self.current_item().is_some() {
                    self.reject_picker_open = !self.reject_picker_open;
                    cx.notify();
                }
            }
            "s" | "S" => self.skip_current(cx),
            "j" | "J" | "down" => self.step_selection(1, cx),
            "k" | "K" | "up" => self.step_selection(-1, cx),
            "f" | "F" => self.cycle_flag(cx),
            "z" | "Z" => self.undo_last(cx),
            "escape" => {
                if self.reject_picker_open {
                    self.reject_picker_open = false;
                    cx.notify();
                } else if self.active_flag_idx.take().is_some() {
                    cx.notify();
                }
            }
            _ => {}
        }
    }

    fn render_kbd(&self, label: &str, inverted: bool, p: &ProPalette) -> Div {
        let (bg, fg) = if inverted {
            if p.dark {
                (hsla(0., 0., 0., 0.14), hsla(0., 0., 0., 0.62))
            } else {
                (hsla(0., 0., 1., 0.16), hsla(0., 0., 1., 0.75))
            }
        } else {
            (p.chip, p.muted)
        };
        div()
            .min_w(px(16.))
            .h(px(16.))
            .px(px(4.))
            .rounded(px(4.))
            .bg(bg)
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(10.))
            .font_weight(FontWeight::MEDIUM)
            .text_color(fg)
            .child(label.to_owned())
    }

    fn render_span_el(
        &self,
        span: &ReviewSpan,
        flag_cursor: &mut usize,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if span.is_flagged() {
            let my_idx = *flag_cursor;
            *flag_cursor += 1;
            let active = self.active_flag_idx == Some(my_idx);
            let flag_bg = if active {
                p.amber.opacity(if p.dark { 0.30 } else { 0.28 })
            } else {
                p.amber_soft
            };
            let flag_edge = p.orange.opacity(0.60);
            div()
                .id(pro_eid(&self.id, format!("flag-span-{my_idx}")))
                .role(Role::Button)
                .aria_label(SharedString::from(format!(
                    "{} — {}",
                    span.text,
                    span.note.as_deref().unwrap_or("")
                )))
                .tab_index(0)
                .px(px(4.))
                .py(px(1.))
                .rounded(px(4.))
                .bg(flag_bg)
                .border_b_1()
                .border_color(flag_edge)
                .text_color(p.amber_ink)
                .cursor_pointer()
                .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                    if *inside && this.active_flag_idx != Some(my_idx) {
                        this.active_flag_idx = Some(my_idx);
                        cx.notify();
                    }
                }))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.active_flag_idx = if this.active_flag_idx == Some(my_idx) {
                        None
                    } else {
                        Some(my_idx)
                    };
                    cx.notify();
                }))
                .child(span.text.clone())
                .into_any_element()
        } else {
            div().child(span.text.clone()).into_any_element()
        }
    }

    fn render_item_card(
        &self,
        item: &ReviewItem,
        interactive: bool,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> Div {
        let mut flag_cursor = 0usize;
        let avatar_el = if let Some(app_name) = &item.app {
            pro_app_tile(app_name, 32., cx)
        } else {
            pro_agent_avatar(&item.agent, p.blue, 32., false, 0., cx)
        };

        let (risk_bg, risk_fg) = match item.risk {
            ReviewRisk::High => (p.red_soft, p.red_ink),
            ReviewRisk::Medium => (p.amber_soft, p.amber_ink),
            ReviewRisk::Low => (p.green_soft, p.green_ink),
        };
        let conf_pct = (item.confidence * 100.0).round() as u32;
        let conf_unsure = item.confidence < self.low_confidence;

        let who_row = div()
            .flex()
            .items_center()
            .gap(px(10.))
            .min_w_0()
            .child(avatar_el)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .min_w_0()
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.strong)
                                    .truncate()
                                    .child(item.agent.clone()),
                            )
                            .child(
                                div()
                                    .h(px(20.))
                                    .px(px(7.))
                                    .rounded(px(5.))
                                    .bg(risk_bg)
                                    .flex()
                                    .items_center()
                                    .text_size(px(11.5))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(risk_fg)
                                    .child(item.risk.label(cx)),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .text_size(px(12.))
                            .text_color(p.muted)
                            .child(item.model.clone())
                            .child(div().text_color(p.faint).child("·"))
                            .child(item.kind_label.clone())
                            .child(div().text_color(p.faint).child("·"))
                            .child(
                                div()
                                    .text_color(if conf_unsure { p.amber_ink } else { p.muted })
                                    .font_weight(if conf_unsure {
                                        FontWeight::MEDIUM
                                    } else {
                                        FontWeight::NORMAL
                                    })
                                    .child(format!(
                                        "{conf_pct}% {}",
                                        tr(cx, "confidence", "置信度")
                                    )),
                            ),
                    ),
            );

        let mut proposal = div().w_full().min_w_0().flex().flex_col().gap(px(14.));

        // Fields block
        if !item.fields.is_empty() {
            let mut fields_wrap = div().flex().flex_wrap().gap_x(px(32.)).gap_y(px(10.));
            for fld in &item.fields {
                let mut val_row = div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(3.))
                    .text_size(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.ink);
                for sp in &fld.spans {
                    if interactive {
                        val_row = val_row.child(self.render_span_el(sp, &mut flag_cursor, p, cx));
                    } else {
                        val_row = val_row.child(sp.text.clone());
                    }
                }
                fields_wrap = fields_wrap.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(p.muted)
                                .child(fld.label.clone()),
                        )
                        .child(val_row),
                );
            }
            proposal = proposal.child(fields_wrap);
        }

        // Inline Editor when editing == true
        let edited_override = self.edited_text_for(&item.id);
        if interactive && self.editing {
            if let Some(ed) = &self.editor {
                proposal = proposal.child(
                    div()
                        .w_full()
                        .flex()
                        .flex_col()
                        .gap(px(6.))
                        .child(
                            div()
                                .text_size(px(12.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.blue)
                                .child(tr(
                                    cx,
                                    "Editing proposal inline (⌘↵ approves with edits, Esc cancels)",
                                    "行内编辑提案（⌘↵ 保存并批准，Esc 取消）",
                                )),
                        )
                        .child(
                            div()
                                .w_full()
                                .h(px(108.))
                                .p(px(10.))
                                .rounded(px(8.))
                                .bg(p.subtle)
                                .border_1()
                                .border_color(p.focus)
                                .child(ed.clone()),
                        ),
                );
            }
        } else {
            // Body prose / edited override
            if let Some(edited) = edited_override {
                proposal = proposal.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(6.))
                        .when_some(item.body_label.clone(), |d, lbl| {
                            d.child(div().text_size(px(12.)).text_color(p.muted).child(lbl))
                        })
                        .child(
                            div()
                                .pl(px(14.))
                                .border_l_2()
                                .border_color(p.green)
                                .py(px(2.))
                                .child(
                                    div()
                                        .px(px(5.))
                                        .py(px(2.))
                                        .rounded(px(4.))
                                        .bg(p.green_soft)
                                        .text_size(px(13.))
                                        .line_height(px(21.))
                                        .text_color(p.green_ink)
                                        .child(edited.clone()),
                                ),
                        ),
                );
            } else if !item.body.is_empty() {
                let mut text_flow = div()
                    .pl(px(14.))
                    .border_l_2()
                    .border_color(p.line_strong)
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_x(px(3.))
                    .gap_y(px(3.))
                    .text_size(px(13.))
                    .line_height(px(21.))
                    .text_color(p.ink);
                if item.format == ReviewFormat::Code {
                    text_flow = text_flow.font_family("Menlo");
                }
                for sp in &item.body {
                    if interactive {
                        text_flow =
                            text_flow.child(self.render_span_el(sp, &mut flag_cursor, p, cx));
                    } else {
                        text_flow = text_flow.child(sp.text.clone());
                    }
                }
                proposal = proposal.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(6.))
                        .when_some(item.body_label.clone(), |d, lbl| {
                            d.child(div().text_size(px(12.)).text_color(p.muted).child(lbl))
                        })
                        .child(text_flow),
                );
            }

            // Unified diff files
            if !item.files.is_empty() {
                let mut code_box = div()
                    .w_full()
                    .rounded(px(10.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .overflow_hidden()
                    .flex()
                    .flex_col();
                for (f_idx, file) in item.files.iter().enumerate() {
                    let (dir_part, base_part) = match file.path.rfind('/') {
                        Some(pos) => (
                            Some(file.path[..=pos].to_owned()),
                            file.path[pos + 1..].to_owned(),
                        ),
                        None => (None, file.path.to_string()),
                    };
                    let head = div()
                        .h(px(36.))
                        .px(px(14.))
                        .when(f_idx > 0, |d| d.border_t_1().border_color(p.line))
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .text_size(px(12.5))
                        .child(pro_icon(ProIcon::Code, 14., p.muted))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .items_center()
                                .truncate()
                                .when_some(dir_part, |d, dp| {
                                    d.child(div().text_color(p.muted).child(dp))
                                })
                                .child(
                                    div()
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.ink)
                                        .child(base_part),
                                ),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.green_ink)
                                .child(format!("+{}", file.added)),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.red_ink)
                                .child(format!("-{}", file.removed)),
                        );

                    let mut diff_rows = div()
                        .w_full()
                        .py(px(6.))
                        .border_t_1()
                        .border_color(p.line)
                        .font_family("Menlo")
                        .text_size(px(12.))
                        .line_height(px(20.))
                        .flex()
                        .flex_col();

                    for ln in &file.lines {
                        let (row_bg, sign_str, sign_col) = match ln.kind {
                            ReviewDiffKind::Context => (transparent_black(), " ", p.faint),
                            ReviewDiffKind::Add => (
                                p.green.opacity(if p.dark { 0.09 } else { 0.08 }),
                                "+",
                                p.green_ink,
                            ),
                            ReviewDiffKind::Del => (
                                p.red.opacity(if p.dark { 0.09 } else { 0.07 }),
                                "-",
                                p.red_ink,
                            ),
                        };
                        let mut code_spans = div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .text_color(p.ink);
                        for sp in &ln.spans {
                            if interactive {
                                code_spans = code_spans.child(self.render_span_el(
                                    sp,
                                    &mut flag_cursor,
                                    p,
                                    cx,
                                ));
                            } else {
                                code_spans = code_spans.child(sp.text.clone());
                            }
                        }
                        diff_rows = diff_rows.child(
                            div()
                                .w_full()
                                .px(px(8.))
                                .bg(row_bg)
                                .flex()
                                .items_start()
                                .gap(px(8.))
                                .child(
                                    div()
                                        .w(px(12.))
                                        .flex_none()
                                        .text_color(sign_col)
                                        .child(sign_str),
                                )
                                .child(
                                    div()
                                        .w(px(28.))
                                        .flex_none()
                                        .text_color(p.muted)
                                        .child(format!("{:>3}", ln.line_no)),
                                )
                                .child(code_spans),
                        );
                    }
                    code_box = code_box.child(head).child(diff_rows);
                }
                proposal = proposal.child(code_box);
            }

            // Numbered plan steps
            if !item.steps.is_empty() {
                let mut steps_col = div().w_full().flex().flex_col().gap(px(10.));
                for (s_idx, st) in item.steps.iter().enumerate() {
                    let is_flagged = st.flag_note.is_some();
                    let my_flag_idx = if is_flagged {
                        let cur = flag_cursor;
                        flag_cursor += 1;
                        Some(cur)
                    } else {
                        None
                    };
                    let badge_hot = my_flag_idx.is_some() && self.active_flag_idx == my_flag_idx;
                    let num_badge = div()
                        .size(px(22.))
                        .flex_none()
                        .rounded_full()
                        .bg(if is_flagged {
                            p.amber.opacity(if badge_hot { 0.30 } else { 0.16 })
                        } else {
                            transparent_black()
                        })
                        .border_1()
                        .border_color(if is_flagged {
                            p.orange.opacity(0.6)
                        } else {
                            p.line_strong
                        })
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(11.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(if is_flagged { p.amber_ink } else { p.muted })
                        .child(format!("{}", s_idx + 1));

                    let mut step_el = div()
                        .id(pro_eid(&self.id, format!("plan-step-{s_idx}")))
                        .flex()
                        .items_start()
                        .gap(px(12.))
                        .child(num_badge)
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .gap(px(2.))
                                .child(
                                    div()
                                        .flex()
                                        .flex_wrap()
                                        .items_center()
                                        .gap(px(8.))
                                        .child(
                                            div()
                                                .text_size(px(13.))
                                                .text_color(if is_flagged {
                                                    p.amber_ink
                                                } else {
                                                    p.ink
                                                })
                                                .font_weight(if is_flagged {
                                                    FontWeight::MEDIUM
                                                } else {
                                                    FontWeight::NORMAL
                                                })
                                                .child(st.title.clone()),
                                        )
                                        .when_some(st.tool.clone(), |d, tl| {
                                            d.child(
                                                div()
                                                    .px(px(6.))
                                                    .py(px(1.))
                                                    .rounded(px(4.))
                                                    .bg(p.chip)
                                                    .font_family("Menlo")
                                                    .text_size(px(11.))
                                                    .text_color(p.muted)
                                                    .child(tl),
                                            )
                                        }),
                                )
                                .when_some(st.detail.clone(), |d, det| {
                                    d.child(
                                        div()
                                            .text_size(px(12.5))
                                            .line_height(px(18.))
                                            .text_color(p.muted)
                                            .child(det),
                                    )
                                }),
                        );
                    if interactive && let Some(fi) = my_flag_idx {
                        step_el = step_el
                            .cursor_pointer()
                            .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                                if *inside && this.active_flag_idx != Some(fi) {
                                    this.active_flag_idx = Some(fi);
                                    cx.notify();
                                }
                            }))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.active_flag_idx = if this.active_flag_idx == Some(fi) {
                                    None
                                } else {
                                    Some(fi)
                                };
                                cx.notify();
                            }));
                    }
                    steps_col = steps_col.child(step_el);
                }
                proposal = proposal.child(steps_col);
            }
        }

        // Sources + Flag counter footer
        let flags = item.collect_flags();
        let foot = (!item.sources.is_empty() || !flags.is_empty()).then(|| {
            let mut srcs = div().flex().flex_wrap().items_center().gap(px(12.));
            for src in &item.sources {
                srcs = srcs.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .text_size(px(12.))
                        .text_color(p.muted)
                        .child(if let Some(app_key) = &src.app {
                            pro_app_tile(app_key, 15., cx)
                        } else {
                            pro_icon(ProIcon::PageDoc, 13., p.muted)
                        })
                        .child(src.label.clone()),
                );
            }
            let flag_btn = (!flags.is_empty() && interactive).then(|| {
                let shown_idx = self.active_flag_idx.map(|i| i + 1).unwrap_or(0);
                div()
                    .id(pro_eid(&self.id, "cycle-flag-btn"))
                    .role(Role::Button)
                    .tab_index(0)
                    .h(px(22.))
                    .px(px(7.))
                    .rounded(px(5.))
                    .bg(p.amber_soft)
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .cursor_pointer()
                    .text_size(px(11.5))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.amber_ink)
                    .on_click(cx.listener(|this, _, _, cx| this.cycle_flag(cx)))
                    .child(pro_icon(ProIcon::AlertTriangle, 12., p.amber_ink))
                    .child(if shown_idx > 0 {
                        format!(
                            "{shown_idx}/{} {}",
                            flags.len(),
                            tr(cx, "flagged", "处存疑")
                        )
                    } else {
                        format!("{} {}", flags.len(), tr(cx, "flagged", "处存疑"))
                    })
                    .child(self.render_kbd("F", false, p))
            });
            div()
                .w_full()
                .pt(px(4.))
                .flex()
                .flex_wrap()
                .items_center()
                .justify_between()
                .gap(px(10.))
                .child(srcs)
                .when_some(flag_btn, |d, fb| d.child(fb))
        });

        // Active Agent's note floating inspection card
        let note_card = if interactive {
            self.active_flag_idx
                .and_then(|idx| flags.get(idx))
                .map(|(expr, note, src)| {
                    div()
                        .w_full()
                        .p(px(12.))
                        .rounded(px(10.))
                        .bg(if p.dark {
                            rgb(0x202024).into()
                        } else {
                            p.subtle
                        })
                        .border_1()
                        .border_color(p.orange.opacity(0.42))
                        .shadow_sm()
                        .flex()
                        .flex_col()
                        .gap(px(5.))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap(px(8.))
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(6.))
                                        .text_size(px(10.5))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(p.amber_ink)
                                        .child(pro_icon(ProIcon::AlertTriangle, 12., p.amber_ink))
                                        .child(tr(cx, "AGENT'S NOTE", "智能体存疑说明")),
                                )
                                .child(
                                    div()
                                        .px(px(6.))
                                        .py(px(1.))
                                        .rounded(px(4.))
                                        .bg(p.amber_soft)
                                        .font_family("Menlo")
                                        .text_size(px(11.))
                                        .text_color(p.amber_ink)
                                        .truncate()
                                        .max_w(px(180.))
                                        .child(expr.clone()),
                                ),
                        )
                        .child(
                            div()
                                .text_size(px(13.))
                                .line_height(px(19.))
                                .text_color(p.ink)
                                .child(note.clone()),
                        )
                        .when_some(src.clone(), |d, source_lbl| {
                            d.child(
                                div().pt(px(2.)).flex().child(
                                    div()
                                        .px(px(6.))
                                        .py(px(2.))
                                        .rounded(px(5.))
                                        .bg(p.chip)
                                        .text_size(px(11.5))
                                        .text_color(p.text)
                                        .child(source_lbl),
                                ),
                            )
                        })
                })
        } else {
            None
        };

        div()
            .w_full()
            .p(px(20.))
            .flex()
            .flex_col()
            .gap(px(14.))
            .child(who_row)
            .child(
                div()
                    .text_size(px(15.))
                    .font_weight(FontWeight::MEDIUM)
                    .line_height(px(22.))
                    .text_color(p.strong)
                    .child(item.title.clone()),
            )
            .when_some(item.reasoning.clone(), |d, why| {
                d.child(
                    div()
                        .mt(px(-6.))
                        .flex()
                        .flex_wrap()
                        .items_baseline()
                        .gap(px(6.))
                        .text_size(px(13.))
                        .line_height(px(20.))
                        .text_color(p.muted)
                        .child(
                            div()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.text)
                                .child(tr(cx, "Why", "原因")),
                        )
                        .child(why),
                )
            })
            .child(proposal)
            .when_some(note_card, |d, nc| d.child(nc))
            .when_some(foot, |d, f| d.child(f))
    }
}

impl Render for ReviewCenter {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let reduced = disabled_motion(cx);
        let width = self.measured_width.get();
        let stacked = width < 640.0;
        let compact = width < 440.0;
        let measure_cell = self.measured_width.clone();

        if self.loading {
            if !reduced {
                w.request_animation_frame();
            }
            return div()
                .id(self.id.clone())
                .w_full()
                .rounded(px(12.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .p(px(18.))
                .flex()
                .flex_col()
                .gap(px(14.))
                .child(pro_skeleton_bar(240., 16., 6., 0.1, cx))
                .child(
                    div()
                        .flex()
                        .gap(px(16.))
                        .child(pro_skeleton_bar(220., 280., 8., 0.25, cx))
                        .child(
                            div()
                                .flex_1()
                                .flex()
                                .flex_col()
                                .gap(px(12.))
                                .child(pro_skeleton_bar(280., 20., 6., 0.4, cx))
                                .child(pro_skeleton_bar(360., 14., 5., 0.55, cx))
                                .child(pro_skeleton_bar(380., 160., 8., 0.7, cx)),
                        ),
                );
        }

        if self.empty {
            return div()
                .id(self.id.clone())
                .w_full()
                .rounded(px(12.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .child(pro_empty_state(
                    ProIcon::Check,
                    tr(cx, "No items awaiting review", "暂无待审批提案"),
                    tr(
                        cx,
                        "Agent actions requiring human sign-off will queue here by risk.",
                        "需要人工确认的智能体操作将按风险等级排列在此处。",
                    ),
                    None,
                    cx,
                ));
        }

        // Sample stamp ghost animation (320ms exit + stamp)
        let active_ghost = self.stamp_ghost.as_ref().and_then(|g| {
            let elapsed = g.started_at.elapsed().as_secs_f32();
            if reduced || elapsed >= 0.36 {
                None
            } else {
                w.request_animation_frame();
                Some((g.clone(), (elapsed / 0.36).clamp(0.0, 1.0)))
            }
        });
        if active_ghost.is_none() && self.stamp_ghost.is_some() {
            self.stamp_ghost = None;
        }

        // Sample card entrance glide via Slide + PhaseClock
        let card_age = self.card_clock.time(0.26, w, cx);
        let card_t = if reduced {
            1.0
        } else {
            pro_ease((card_age / 0.26).clamp(0.0, 1.0))
        };
        let card_dy = if reduced {
            0.0
        } else {
            self.card_slide.get(w, cx) * self.card_dir.abs().max(1.0)
        };
        let card_alpha = 0.35 + 0.65 * card_t;

        // Counts & SLA
        let undecided = self.sorted_undecided_indices();
        let all_count = undecided.len();
        let high_count = undecided
            .iter()
            .filter(|&&i| self.items[i].risk == ReviewRisk::High)
            .count();
        let unsure_count = undecided
            .iter()
            .filter(|&&i| self.is_unsure(&self.items[i]))
            .count();
        let reviewed_now = self.reviewed_count();
        let total_session = reviewed_now + all_count;
        let progress_ratio = if total_session > 0 {
            reviewed_now as f32 / total_session as f32
        } else {
            1.0
        };
        let oldest_mins = undecided
            .iter()
            .map(|&i| self.items[i].waiting_mins)
            .max()
            .unwrap_or(0);
        let sla_ratio = oldest_mins as f32 / self.sla_mins as f32;
        let oldest_col = if sla_ratio >= 1.0 {
            p.red_ink
        } else if sla_ratio >= 0.75 {
            p.amber_ink
        } else {
            p.text
        };

        let heading_text: SharedString = self
            .title
            .clone()
            .unwrap_or_else(|| tr(cx, "Review center", "审查工作台").into());

        // Top header bar
        let header = div()
            .w_full()
            .min_h(px(48.))
            .px(px(16.))
            .py(px(10.))
            .border_b_1()
            .border_color(p.line)
            .flex()
            .flex_wrap()
            .items_center()
            .gap_x(px(12.))
            .gap_y(px(8.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(pro_svg("0 0 24 24", SEAL_BADGE_SVG, 16., p.blue))
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .child(heading_text.clone()),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(div().w(px(80.)).child(pro_spring_bar(
                        pro_eid(&self.id, "hdr-prog"),
                        progress_ratio,
                        6.,
                        3.,
                        p.chip,
                        p.blue.opacity(if p.dark { 0.75 } else { 0.65 }),
                    )))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(4.))
                            .text_size(px(12.))
                            .text_color(p.muted)
                            .child(pro_rolling_number(
                                pro_eid(&self.id, "rev-num"),
                                reviewed_now as f64,
                                0,
                                "",
                                "",
                                12.,
                                FontWeight::MEDIUM,
                                p.ink,
                            ))
                            .child(format!(
                                "{} {total_session} {}",
                                tr(cx, "of", "/"),
                                tr(cx, "reviewed", "已审")
                            )),
                    ),
            )
            .when(oldest_mins > 0, |d| {
                d.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .text_size(px(12.))
                        .text_color(p.muted)
                        .child(div().text_color(p.faint).child("·"))
                        .child(tr(cx, "oldest", "最久等待"))
                        .child(
                            div()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(oldest_col)
                                .child(format!("{oldest_mins}m")),
                        ),
                )
            });

        // Left queue filter strip + rows
        let filter_tabs = [
            (ReviewFilter::All, tr(cx, "All", "全部"), all_count),
            (
                ReviewFilter::HighRisk,
                tr(cx, "High risk", "高风险"),
                high_count,
            ),
            (ReviewFilter::Unsure, tr(cx, "Unsure", "存疑"), unsure_count),
        ];
        let mut filter_bar = div()
            .relative()
            .w_full()
            .p(px(2.))
            .rounded(px(8.))
            .bg(p.subtle)
            .flex()
            .items_center()
            .gap(px(2.))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "flt-pill"),
                self.filter.key(),
                self.filter_sensor.clone(),
                6.,
                p.card,
                Some(p.line),
                true,
            ));

        for (flt, lbl, cnt) in filter_tabs {
            let selected = self.filter == flt;
            filter_bar = filter_bar.child(
                div()
                    .id(pro_eid(&self.id, format!("flt-{}", flt.key())))
                    .role(Role::Button)
                    .tab_index(0)
                    .relative()
                    .h(px(26.))
                    .px(px(9.))
                    .rounded(px(6.))
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .cursor_pointer()
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(if selected { p.ink } else { p.muted })
                    .on_click(cx.listener(move |this, _, _, cx| this.set_filter(flt, cx)))
                    .when(selected, |d| {
                        d.child(pro_mark_selected(&self.filter_sensor))
                    })
                    .child(lbl)
                    .child(
                        div()
                            .text_size(px(11.5))
                            .text_color(if selected { p.muted } else { p.faint })
                            .child(format!("{cnt}")),
                    ),
            );
        }

        let vis = self.visible_indices();
        let cur_idx = self.current_item_index();
        let mut queue_list = div()
            .id(pro_eid(&self.id, "queue-scroll"))
            .flex_1()
            .min_h_0()
            .when(stacked, |d| d.max_h(px(184.)))
            .p(px(8.))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(px(2.));

        if vis.is_empty() {
            queue_list = queue_list.child(
                div()
                    .w_full()
                    .h(px(110.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(12.))
                    .text_color(p.faint)
                    .child(tr(cx, "Queue clear", "队列已清空")),
            );
        } else {
            for &idx in &vis {
                let it = &self.items[idx];
                let is_cur = cur_idx == Some(idx);
                let item_id = it.id.clone();
                let skipped = self.is_skipped(&item_id);
                let row_sla = it.waiting_mins as f32 / self.sla_mins as f32;
                let wait_col = if row_sla >= 1.0 {
                    p.red_ink
                } else if row_sla >= 0.75 {
                    p.amber_ink
                } else {
                    p.faint
                };
                let tile = if let Some(app_name) = &it.app {
                    pro_app_tile(app_name, 24., cx)
                } else {
                    pro_agent_avatar(&it.agent, p.blue, 24., false, 0., cx)
                };
                let risk_col = match it.risk {
                    ReviewRisk::High => p.red_ink,
                    ReviewRisk::Medium => p.amber_ink,
                    ReviewRisk::Low => p.muted,
                };
                queue_list = queue_list.child(
                    div()
                        .id(pro_eid(&self.id, format!("qrow-{}", it.id)))
                        .role(Role::Button)
                        .aria_label(SharedString::from(format!(
                            "{}. {}, {}, {}m",
                            it.title,
                            it.kind_label,
                            it.risk.label(cx),
                            it.waiting_mins
                        )))
                        .tab_index(0)
                        .w_full()
                        .h(px(56.))
                        .px(px(10.))
                        .rounded(px(8.))
                        .bg(if is_cur {
                            if p.dark {
                                hsla(0., 0., 1., 0.06)
                            } else {
                                p.card
                            }
                        } else {
                            transparent_black()
                        })
                        .when(is_cur, |d| d.border_1().border_color(p.line).shadow_xs())
                        .hover(move |s| if is_cur { s } else { s.bg(p.hover) })
                        .flex()
                        .items_center()
                        .gap(px(10.))
                        .cursor_pointer()
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.select_item(item_id.clone(), 1.0, cx);
                        }))
                        .child(tile)
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .gap(px(2.))
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(8.))
                                        .min_w_0()
                                        .child(
                                            div()
                                                .flex_1()
                                                .min_w_0()
                                                .text_size(px(13.))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(p.ink)
                                                .truncate()
                                                .child(it.title.clone()),
                                        )
                                        .child(
                                            div()
                                                .flex_none()
                                                .text_size(px(12.))
                                                .text_color(wait_col)
                                                .child(format!("{}m", it.waiting_mins)),
                                        ),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(5.))
                                        .text_size(px(12.))
                                        .text_color(p.muted)
                                        .child(it.kind_label.clone())
                                        .when(it.risk != ReviewRisk::Low, |d| {
                                            d.child(div().text_color(p.faint).child("·")).child(
                                                div()
                                                    .text_color(risk_col)
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .child(it.risk.short_label(cx)),
                                            )
                                        })
                                        .when(skipped, |d| {
                                            d.child(div().text_color(p.faint).child("·")).child(
                                                div().text_color(p.faint).child(tr(
                                                    cx,
                                                    "Skipped",
                                                    "已跳过",
                                                )),
                                            )
                                        }),
                                ),
                        ),
                );
            }
        }

        let left_queue = div()
            .when(!stacked, |d| {
                d.w(px(276.)).h_full().border_r_1().border_color(p.line)
            })
            .when(stacked, |d| d.w_full().border_b_1().border_color(p.line))
            .bg(if p.dark {
                hsla(0., 0., 1., 0.015)
            } else {
                hsla(0., 0., 0., 0.015)
            })
            .flex()
            .flex_col()
            .min_w_0()
            .min_h_0()
            .child(
                div()
                    .p(px(8.))
                    .bg(p.card)
                    .border_b_1()
                    .border_color(p.line)
                    .child(filter_bar),
            )
            .child(queue_list);

        // Main detail inspection stage
        let cur_item_cloned = self.current_item().cloned();
        let detail_body = if let Some(item) = &cur_item_cloned {
            div()
                .relative()
                .top(px(card_dy))
                .opacity(card_alpha)
                .child(self.render_item_card(item, true, &p, cx))
                .into_any_element()
        } else {
            div()
                .w_full()
                .h_full()
                .py(px(56.))
                .px(px(24.))
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap(px(8.))
                .child(
                    div()
                        .size(px(40.))
                        .rounded(px(10.))
                        .bg(p.green_soft)
                        .border_1()
                        .border_color(p.green.opacity(0.32))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(pro_icon(ProIcon::Check, 18., p.green_ink)),
                )
                .child(
                    div()
                        .text_size(px(15.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.strong)
                        .child(tr(cx, "All caught up", "已全部审阅完毕")),
                )
                .child(
                    div()
                        .max_w(px(280.))
                        .text_size(px(13.))
                        .line_height(px(19.))
                        .text_color(p.muted)
                        .text_center()
                        .child(tr(
                            cx,
                            "Every queued proposal in this view has been resolved.",
                            "当前筛选视图下的所有智能体提案均已处理完成。",
                        )),
                )
                .into_any_element()
        };

        // Animated resolution stamp overlay on leaving card
        let ghost_overlay = active_ghost.map(|(g, progress)| {
            let fade_out = 1.0 - pro_ease(((progress - 0.35) / 0.65).clamp(0.0, 1.0));
            let stamp_pop = pro_ease((progress / 0.35).clamp(0.0, 1.0));
            let lift_y = -8.0 * pro_ease(progress);
            let (stamp_col, stamp_bg, stamp_icon, stamp_word) = match g.decision {
                ReviewDecisionKind::Approved => (
                    p.green_ink,
                    p.green_soft,
                    ProIcon::Check,
                    tr(cx, "APPROVED", "已批准"),
                ),
                ReviewDecisionKind::Rejected => (
                    p.red_ink,
                    p.red_soft,
                    ProIcon::Close,
                    tr(cx, "REJECTED", "已驳回"),
                ),
            };
            div()
                .absolute()
                .inset_0()
                .bg(p.card.opacity(fade_out))
                .top(px(lift_y))
                .opacity(fade_out)
                .overflow_hidden()
                .child(self.render_item_card(&g.item, false, &p, cx))
                .child(
                    div()
                        .absolute()
                        .top(px(20.0 + 4.0 * (1.0 - stamp_pop)))
                        .right(px(22.))
                        .opacity(stamp_pop)
                        .h(px(30.))
                        .px(px(10.))
                        .rounded(px(7.))
                        .bg(stamp_bg)
                        .border_2()
                        .border_color(stamp_col.opacity(0.65))
                        .shadow_sm()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .text_size(px(12.))
                        .font_weight(FontWeight::BOLD)
                        .text_color(stamp_col)
                        .child(pro_icon(stamp_icon, 14., stamp_col))
                        .child(stamp_word)
                        .when_some(g.reason, |d, r| {
                            d.child(
                                div()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(format!("· {r}")),
                            )
                        }),
                )
        });

        // Bottom action bar (`Approve A`, `Edit E`, `Reject R`, `Skip S` + `Undo Z`)
        let has_current = cur_item_cloned.is_some();
        let last_decided = self.decided.last().and_then(|rec| {
            (rec.decided_at.elapsed().as_secs_f32() < 8.0)
                .then_some((rec.title.clone(), rec.decision))
        });
        if last_decided.is_some() && !reduced {
            w.request_animation_frame();
        }

        let left_controls = if self.editing {
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap(px(8.))
                .child(pro_btn(
                    pro_eid(&self.id, "save-approve-btn"),
                    tr(cx, "Save & approve", "保存并批准"),
                    Some(ProIcon::Check),
                    ProBtnStyle::Primary,
                    true,
                    cx,
                    |this, w, cx| this.save_edit_and_approve(w, cx),
                ))
                .child(pro_btn(
                    pro_eid(&self.id, "cancel-edit-btn"),
                    tr(cx, "Cancel", "取消"),
                    None,
                    ProBtnStyle::Ghost,
                    true,
                    cx,
                    |this, w, cx| {
                        this.editing = false;
                        this.editor_focused = false;
                        this.focus.focus(w, cx);
                        cx.notify();
                    },
                ))
                .into_any_element()
        } else if self.reject_picker_open && has_current {
            let mut row = div().flex().flex_wrap().items_center().gap(px(6.)).child(
                div()
                    .text_size(px(12.5))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .mr(px(2.))
                    .child(tr(cx, "Reason:", "驳回原因：")),
            );
            for (r_idx, r_lbl) in self.reasons.iter().take(4).enumerate() {
                let reason_val = r_lbl.clone();
                let key_num = format!("{}", r_idx + 1);
                row = row.child(
                    div()
                        .id(pro_eid(&self.id, format!("reason-btn-{r_idx}")))
                        .role(Role::Button)
                        .tab_index(0)
                        .h(px(28.))
                        .px(px(8.))
                        .rounded(px(6.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line_strong)
                        .hover(|s| s.border_color(p.red.opacity(0.45)).bg(p.red_soft))
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .cursor_pointer()
                        .text_size(px(12.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.strong)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.reject_current(Some(reason_val.clone()), cx);
                        }))
                        .child(self.render_kbd(&key_num, false, &p))
                        .child(r_lbl.clone()),
                );
            }
            row = row.child(
                div()
                    .id(pro_eid(&self.id, "cancel-reject-btn"))
                    .role(Role::Button)
                    .tab_index(0)
                    .h(px(28.))
                    .px(px(8.))
                    .rounded(px(6.))
                    .flex()
                    .items_center()
                    .cursor_pointer()
                    .text_size(px(12.))
                    .text_color(p.muted)
                    .hover(|s| s.text_color(p.ink))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.reject_picker_open = false;
                        cx.notify();
                    }))
                    .child(tr(cx, "Cancel", "取消")),
            );
            row.into_any_element()
        } else {
            let show_kbd = !compact;
            let mk_action = |id_sfx: &str,
                             label: &str,
                             kbd: &str,
                             primary: bool,
                             icon_el: AnyElement,
                             handler: fn(&mut Self, &mut Window, &mut Context<Self>)|
             -> Stateful<Div> {
                let (bg, fg, bdr) = if primary {
                    (p.primary, p.on_primary, transparent_black())
                } else {
                    (
                        if p.dark { rgb(0x26262a).into() } else { p.card },
                        p.strong,
                        p.line_strong,
                    )
                };
                div()
                    .id(pro_eid(&self.id, id_sfx.to_owned()))
                    .role(Role::Button)
                    .tab_index(0)
                    .h(px(28.))
                    .px(px(8.))
                    .rounded(px(6.))
                    .bg(bg)
                    .when(!primary, |d| d.border_1().border_color(bdr))
                    .opacity(if has_current { 1.0 } else { 0.45 })
                    .when(has_current, |d| {
                        d.cursor_pointer()
                            .hover(move |s| {
                                if primary {
                                    s.opacity(0.92)
                                } else {
                                    s.bg(p.chip_hover)
                                }
                            })
                            .on_click(cx.listener(move |this, _, w, cx| handler(this, w, cx)))
                    })
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .text_size(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(fg)
                    .child(icon_el)
                    .child(label.to_owned())
                    .when(show_kbd, |d| d.child(self.render_kbd(kbd, primary, &p)))
            };

            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap(px(8.))
                .child(mk_action(
                    "act-approve",
                    tr(cx, "Approve", "批准"),
                    "A",
                    true,
                    pro_icon(ProIcon::Check, 14., p.on_primary),
                    |this, _, cx| this.approve_current(cx),
                ))
                .child(mk_action(
                    "act-edit",
                    tr(cx, "Edit", "编辑"),
                    "E",
                    false,
                    pro_svg("0 0 24 24", PENCIL_EDIT_SVG, 14., p.muted),
                    |this, w, cx| this.open_inline_editor(w, cx),
                ))
                .child(mk_action(
                    "act-reject",
                    tr(cx, "Reject", "驳回"),
                    "R",
                    false,
                    pro_icon(ProIcon::Close, 14., p.muted),
                    |this, _, cx| {
                        this.reject_picker_open = true;
                        cx.notify();
                    },
                ))
                .child(mk_action(
                    "act-skip",
                    tr(cx, "Skip", "跳过"),
                    "S",
                    false,
                    pro_icon(ProIcon::ChevronRight, 14., p.muted),
                    |this, _, cx| this.skip_current(cx),
                ))
                .into_any_element()
        };

        let undo_chip = pro_presence(
            pro_eid(&self.id, "undo-pres"),
            !self.decided.is_empty(),
            0.22,
            0.20,
            {
                let last_summary = self.decided.last().map(|d| {
                    let lbl = match &d.reason {
                        Some(r) => SharedString::from(format!("{} ({r})", d.title)),
                        None => d.title.clone(),
                    };
                    (d.decision, lbl)
                });
                let last_dec = last_summary.as_ref().map(|(dec, _)| *dec);
                let summary_txt = last_summary.map(|(_, lbl)| lbl).filter(|_| !compact);
                let undo_btn = div()
                    .id(pro_eid(&self.id, "undo-last-btn"))
                    .role(Role::Button)
                    .tab_index(0)
                    .h(px(26.))
                    .px(px(8.))
                    .rounded(px(6.))
                    .bg(p.subtle)
                    .border_1()
                    .border_color(p.line)
                    .hover(|s| s.bg(p.chip_hover))
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .cursor_pointer()
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .on_click(cx.listener(|this, _, _, cx| this.undo_last(cx)))
                    .child(if last_dec == Some(ReviewDecisionKind::Rejected) {
                        pro_icon(ProIcon::Close, 12., p.red_ink)
                    } else {
                        pro_icon(ProIcon::Check, 12., p.green_ink)
                    })
                    .child(tr(cx, "Undo", "撤销"))
                    .child(self.render_kbd("Z", false, &p));
                move |amt| {
                    div()
                        .opacity(amt)
                        .ml_auto()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .min_w_0()
                        .when_some(summary_txt, |d, st| {
                            d.child(
                                div()
                                    .max_w(px(140.))
                                    .text_size(px(11.5))
                                    .text_color(p.muted)
                                    .truncate()
                                    .child(st),
                            )
                        })
                        .child(undo_btn)
                        .into_any_element()
                }
            },
        );

        let right_main = div()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .bg(p.card)
            .flex()
            .flex_col()
            .child(
                div()
                    .id(pro_eid(&self.id, "stage-scroll"))
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(detail_body)
                    .when_some(ghost_overlay, |d, g| d.child(g)),
            )
            .child(
                div()
                    .w_full()
                    .min_h(px(52.))
                    .px(px(16.))
                    .py(px(10.))
                    .border_t_1()
                    .border_color(p.line)
                    .bg(p.card)
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(10.))
                    .child(left_controls)
                    .child(undo_chip),
            );

        div()
            .id(self.id.clone())
            .role(Role::Region)
            .aria_label(heading_text)
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::handle_key_down))
            .relative()
            .w_full()
            .min_w_0()
            .rounded(px(12.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .shadow_sm()
            .overflow_hidden()
            .flex()
            .flex_col()
            .child(
                canvas(
                    move |b, window, _| {
                        let nw = f32::from(b.size.width);
                        if nw > 10.0 && (measure_cell.get() - nw).abs() > 1.0 {
                            measure_cell.set(nw);
                            window.request_animation_frame();
                        }
                    },
                    |_, _, _, _| (),
                )
                .w_full()
                .h(px(0.)),
            )
            .child(header)
            .child(
                div()
                    .w_full()
                    .when(!stacked, |d| d.h(px(548.)).flex())
                    .when(stacked, |d| d.flex().flex_col())
                    .child(left_queue)
                    .child(right_main),
            )
            .child(pro_toast_banner(self.toast.clone()))
    }
}

fn default_review_items() -> Vec<ReviewItem> {
    vec![
        ReviewItem::new("merge-511", "Merge #511 – retries for webhook delivery")
            .with_agent("Code agent", "Opus 5.5")
            .with_app("github")
            .with_kind_label("Code change")
            .with_risk(ReviewRisk::High)
            .with_confidence(0.41)
            .with_waiting_mins(28)
            .with_reasoning("Deliveries time out under load – I added retries with backoff. Tests pass.")
            .with_files(vec![ReviewFile::new(
                "src/queue/worker.ts",
                14,
                3,
                vec![
                    ReviewDiffLine::new(
                        ReviewDiffKind::Context,
                        41,
                        "export async function deliverWebhook(job: WebhookJob) {",
                    ),
                    ReviewDiffLine::new(
                        ReviewDiffKind::Del,
                        42,
                        "  const maxAttempts = 3;",
                    ),
                    ReviewDiffLine::with_spans(
                        ReviewDiffKind::Add,
                        42,
                        vec![
                            ReviewSpan::plain("  const "),
                            ReviewSpan::flagged(
                                "maxAttempts = 10",
                                "Retries went from 3 to 10 with backoff – the last wait is over 2 minutes, longer than the worker's 60s timeout.",
                                Some("queue/worker.ts".into()),
                            ),
                            ReviewSpan::plain(";"),
                        ],
                    ),
                    ReviewDiffLine::with_spans(
                        ReviewDiffKind::Add,
                        43,
                        vec![
                            ReviewSpan::plain("  const waitMs = "),
                            ReviewSpan::flagged(
                                "2_000 * 2 ** attempt",
                                "Unbounded exponential wait reaches 512s on attempt 8 unless capped by maxBackoffMs.",
                                Some("config/limits.ts".into()),
                            ),
                            ReviewSpan::plain(";"),
                        ],
                    ),
                    ReviewDiffLine::new(
                        ReviewDiffKind::Context,
                        44,
                        "  return dispatchWithTimeout(job.url, job.payload, waitMs);",
                    ),
                ],
            )])
            .with_sources(vec![
                ReviewSource::new("PR #511", Some("github".into())),
                ReviewSource::new("queue/worker.ts", None),
            ]),
        ReviewItem::new("redline-72", "Redline §7.2 Limitation of liability")
            .with_agent("Legal agent", "Opus 5.5")
            .with_kind_label("Contract clause")
            .with_risk(ReviewRisk::High)
            .with_confidence(0.58)
            .with_waiting_mins(18)
            .with_reasoning("Counterparty asked to raise the super-cap for data processing claims before Friday signature.")
            .with_fields(vec![
                ReviewField::plain("Counterparty", "Northwind Health"),
                ReviewField::with_spans(
                    "Liability cap",
                    vec![ReviewSpan::flagged(
                        "24 months of fees paid",
                        "Standard MSA playbook caps aggregate liability at 12 months unless VP Legal signs off.",
                        Some("MSA Playbook §7.2".into()),
                    )],
                ),
            ])
            .with_body(
                Some("Proposed clause §7.2".into()),
                vec![
                    ReviewSpan::plain("Each party's aggregate liability under this Agreement shall not exceed "),
                    ReviewSpan::flagged(
                        "twenty-four (24) months of fees",
                        "Doubles our standard 12-month fee cap on a $140k ARR contract.",
                        Some("MSA Playbook §7.2".into()),
                    ),
                    ReviewSpan::plain(" paid in the preceding period, excluding indirect or consequential damages."),
                ],
            )
            .with_sources(vec![ReviewSource::new("MSA Playbook §7.2", None)]),
        ReviewItem::new(
            "split-customers",
            "Split customers.name into first and last name",
        )
        .with_agent("Data agent", "Sonnet 5.5")
        .with_kind_label("Migration")
        .with_risk(ReviewRisk::Medium)
        .with_confidence(0.64)
        .with_waiting_mins(14)
        .with_reasoning("CRM sync requires separate given_name and family_name columns on customers.")
        .with_files(vec![ReviewFile::new(
            "migrations/0089_split_customer_name.sql",
            6,
            0,
            vec![
                ReviewDiffLine::new(
                    ReviewDiffKind::Add,
                    1,
                    "ALTER TABLE customers ADD COLUMN first_name text, ADD COLUMN last_name text;",
                ),
                ReviewDiffLine::with_spans(
                    ReviewDiffKind::Add,
                    2,
                    vec![
                        ReviewSpan::plain("UPDATE customers SET last_name = "),
                        ReviewSpan::flagged(
                            "nullif(split_part(name, ' ', 2), '')",
                            "Compound surnames like 'de la Cruz' lose tokens past the second space on 3.4% of rows.",
                            Some("migrations/0089_split_customer_name.sql".into()),
                        ),
                        ReviewSpan::plain(";"),
                    ],
                ),
            ],
        )]),
        ReviewItem::new("rotate-stripe", "Rotate the Stripe API keys")
            .with_agent("Ops agent", "Sonnet 5.5")
            .with_app("stripe")
            .with_kind_label("Plan")
            .with_risk(ReviewRisk::Medium)
            .with_confidence(0.72)
            .with_waiting_mins(11)
            .with_reasoning("Quarterly credential rotation scheduled by security policy.")
            .with_steps(vec![
                ReviewPlanStep::new("Create restricted live key with Billing & Webhook scopes")
                    .with_tool("stripe.keys.create"),
                ReviewPlanStep::new("Roll production secrets across 4 checkout workers")
                    .with_tool("k8s.secret.patch"),
                ReviewPlanStep::new("Expire previous live key after 15m grace window")
                    .with_tool("stripe.keys.expire")
                    .with_detail("Legacy cron retry queue runs every 30m and could still reference the old secret.")
                    .with_flag("Grace window (15m) is shorter than the 30m billing reconciliation cron interval."),
            ])
            .with_sources(vec![ReviewSource::new("Stripe Security Runbook", Some("stripe".into()))]),
        ReviewItem::new("refund-4821", "Refund order #4821 for Maya Chen")
            .with_agent("Support agent", "Sonnet 5.5")
            .with_kind_label("Refund")
            .with_risk(ReviewRisk::Medium)
            .with_confidence(0.62)
            .with_waiting_mins(9)
            .with_reasoning("Her blender arrived cracked – damaged items are covered within 30 days.")
            .with_fields(vec![
                ReviewField::with_spans(
                    "Amount",
                    vec![ReviewSpan::flagged(
                        "$184.00",
                        "Includes $20 express shipping – policy §4 normally refunds item subtotal ($164.00) only.",
                        Some("Refund policy §4".into()),
                    )],
                ),
                ReviewField::plain("Order", "#4821"),
                ReviewField::plain("Customer", "Maya Chen"),
            ])
            .with_body(
                Some("Reply to Maya".into()),
                vec![
                    ReviewSpan::plain("Hi Maya, I'm so sorry your blender arrived cracked. I've issued a full "),
                    ReviewSpan::flagged(
                        "$184.00 refund including express shipping",
                        "Confirm whether express shipping should be refunded as a goodwill gesture.",
                        Some("Refund policy §4".into()),
                    ),
                    ReviewSpan::plain(" back to your original card—it should post within 3–5 business days."),
                ],
            )
            .with_sources(vec![ReviewSource::new("Refund policy §4", None)]),
        ReviewItem::new("tag-billing", "Tag 14 tickets as Billing")
            .with_agent("Triage agent", "Haiku 5.5")
            .with_kind_label("Triage")
            .with_risk(ReviewRisk::Low)
            .with_confidence(0.97)
            .with_waiting_mins(6)
            .with_reasoning("Matched invoice, VAT receipt, and proration keywords across 14 unassigned inbox threads.")
            .with_fields(vec![
                ReviewField::plain("Queue", "Support Inbox"),
                ReviewField::plain("Matched threads", "14 tickets"),
            ]),
        ReviewItem::new("bump-stripe", "Bump @stripe/stripe-js to 4.2.1")
            .with_agent("Deps agent", "Haiku 5.5")
            .with_app("github")
            .with_kind_label("Code change")
            .with_risk(ReviewRisk::Low)
            .with_confidence(0.95)
            .with_waiting_mins(4)
            .with_reasoning("Patch release fixing TypeScript Element mount typings; 418 checkout tests green.")
            .with_fields(vec![
                ReviewField::plain("Package", "@stripe/stripe-js"),
                ReviewField::plain("Version", "4.2.0 -> 4.2.1"),
            ]),
        ReviewItem::new("post-release", "Post release notes to #announcements")
            .with_agent("Comms agent", "Sonnet 5.5")
            .with_app("slack")
            .with_kind_label("Slack message")
            .with_risk(ReviewRisk::Low)
            .with_confidence(0.91)
            .with_waiting_mins(2)
            .with_reasoning("v2.18 deployment succeeded in all regions; summary drafted from merged PRs.")
            .with_body(
                Some("Message to #announcements".into()),
                vec![ReviewSpan::plain(
                    "KUI v2.18 is live: faster webhook retries, refined billing exports, and zero-downtime key rotation.",
                )],
            )
            .with_sources(vec![ReviewSource::new("#announcements", Some("slack".into()))]),
    ]
}

#[cfg(test)]
mod tests {
    use super::{ReviewFilter, ReviewOrder, ReviewRisk, default_review_items};

    #[test]
    fn default_items_sort_high_risk_and_oldest_first() {
        let items = default_review_items();
        assert_eq!(items.len(), 8);
        let high: Vec<&str> = items
            .iter()
            .filter(|i| i.risk == ReviewRisk::High)
            .map(|i| i.id.as_ref())
            .collect();
        assert_eq!(high, vec!["merge-511", "redline-72"]);

        let unsure_count = items
            .iter()
            .filter(|i| i.confidence < 0.70 || !i.collect_flags().is_empty())
            .count();
        assert_eq!(unsure_count, 5);
        assert_eq!(ReviewFilter::All.key(), "all");
        assert_eq!(ReviewOrder::default(), ReviewOrder::Risk);
    }

    #[test]
    fn collect_flags_extracts_diff_field_and_step_uncertainties() {
        let items = default_review_items();
        let merge_flags = items[0].collect_flags();
        assert_eq!(merge_flags.len(), 2);
        assert_eq!(merge_flags[0].0.as_ref(), "maxAttempts = 10");
        assert_eq!(merge_flags[0].2.as_deref(), Some("queue/worker.ts"));

        let plan_flags = items[3].collect_flags();
        assert_eq!(plan_flags.len(), 1);
        assert!(plan_flags[0].1.contains("15m"));
    }
}
