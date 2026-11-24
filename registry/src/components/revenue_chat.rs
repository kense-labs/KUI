//! Standalone conversational revenue analyst block with KPI cards, brushable charts, and composer.
//!
//! Renders [`RevenueChatBlock`] without an outer application shell: a centered analyst thread
//! with KPI cards, a pointer-brushable YoY weekly area/line chart, regional stacked-bar splits,
//! Q4 forecast cones, follow-up suggestion chips, and an [`Editor`]-backed prompt composer.

use crate::{
    components::pro_chrome::{
        ProBtnStyle, ProPalette, pro_btn, pro_ease, pro_eid, pro_empty_state, pro_icon_btn,
        pro_mix_color, pro_presence, pro_shimmer_text, pro_skeleton_bar, pro_spring_bar,
        pro_spring_f32, pro_toast_banner, pro_view_transition, tr,
    },
    foundation::Theme,
    primitives::{
        control::{UiLanguage, activate},
        editor::{EditEvent, Editor, InputMode},
        pro_icons::{AGENT_WAVE_MS, ProIcon, agent_pixel_cells, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, collections::HashMap, rc::Rc};
use web_time::Instant;

const SVG_HAND_APPROVAL: &str = r#"<path d="M52.5305 32.6557V24.8463C52.5305 22.1016 54.7555 19.8766 57.5001 19.8766C60.2448 19.8766 62.4698 22.1016 62.4698 24.8463V42.027C62.4698 53.3194 53.3154 62.4735 42.0233 62.4735H34.2388M22.7127 39.7552L15.94 31.8537C14.8314 30.5602 13.2129 29.8159 11.5094 29.8159H11.1235C8.11379 29.8159 5.67395 32.2558 5.67395 35.2655C5.67395 36.3412 5.99241 37.393 6.58919 38.2881L16.4118 53.0221C20.3482 58.9269 26.9753 62.4735 34.0719 62.4735" stroke="currentColor" stroke-width="4.7" stroke-linecap="round" stroke-linejoin="round"></path><path d="M52.534 24.1378V16.3284C52.534 13.5837 50.3091 11.3588 47.5644 11.3588C44.8197 11.3588 42.5948 13.5837 42.5948 16.3284V29.8174" stroke="currentColor" stroke-width="4.7" stroke-linecap="round" stroke-linejoin="round"></path><path d="M42.5958 29.8198V10.6513C42.5958 7.90661 40.3708 5.68163 37.6262 5.68163C34.8815 5.68163 32.6565 7.90661 32.6565 10.6513V28.4" stroke="currentColor" stroke-width="4.7" stroke-linecap="round" stroke-linejoin="round"></path><path d="M32.6576 28.3975V16.3284C32.6576 13.5837 30.4326 11.3588 27.6879 11.3588C24.9433 11.3588 22.7183 13.5837 22.7183 16.3284V39.7567" stroke="currentColor" stroke-width="4.7" stroke-linecap="round" stroke-linejoin="round"></path>"#;
const SVG_THUMBS_UP: &str = r#"<path d="M2 12.5C2 11.3954 2.89543 10.5 4 10.5C5.65685 10.5 7 11.8431 7 13.5V17.5C7 19.1569 5.65685 20.5 4 20.5C2.89543 20.5 2 19.6046 2 18.5V12.5Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M15.4787 7.80626L15.2124 8.66634C14.9942 9.37111 14.8851 9.72349 14.969 10.0018C15.0369 10.2269 15.1859 10.421 15.389 10.5487C15.64 10.7065 16.0197 10.7065 16.7791 10.7065H17.1831C19.7532 10.7065 21.0382 10.7065 21.6452 11.4673C21.7145 11.5542 21.7762 11.6467 21.8296 11.7437C22.2965 12.5921 21.7657 13.7351 20.704 16.0211C19.7297 18.1189 19.2425 19.1678 18.338 19.7852C18.2505 19.8449 18.1605 19.9013 18.0683 19.9541C17.116 20.5 15.9362 20.5 13.5764 20.5H13.0646C10.2057 20.5 8.77628 20.5 7.88814 19.6395C7 18.7789 7 17.3939 7 14.6239V13.6503C7 12.1946 7 11.4668 7.25834 10.8006C7.51668 10.1344 8.01135 9.58664 9.00069 8.49112L13.0921 3.96056C13.1947 3.84694 13.246 3.79012 13.2913 3.75075C13.7135 3.38328 14.3652 3.42464 14.7344 3.84235C14.774 3.8871 14.8172 3.94991 14.9036 4.07554C15.0388 4.27205 15.1064 4.37031 15.1654 4.46765C15.6928 5.33913 15.8524 6.37436 15.6108 7.35715C15.5838 7.46692 15.5488 7.5801 15.4787 7.80626Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const COMPOSER_MODELS: [&str; 4] = ["Auto", "Sonnet 4.6", "Opus 4.6", "GPT-5.4"];

/// Tool execution approval policy selectable in the composer footer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RevenueApprovalPolicy {
    /// Prompt before external writes or exports.
    #[default]
    RequestApproval,
    /// Run read-only queries automatically.
    AutoApproveReads,
    /// Execute without confirmation prompts.
    FullAutonomy,
}

impl RevenueApprovalPolicy {
    fn next(self) -> Self {
        match self {
            Self::RequestApproval => Self::AutoApproveReads,
            Self::AutoApproveReads => Self::FullAutonomy,
            Self::FullAutonomy => Self::RequestApproval,
        }
    }

    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::RequestApproval => tr(cx, "Request approval", "需人工确认"),
            Self::AutoApproveReads => tr(cx, "Auto-approve reads", "只读自动放行"),
            Self::FullAutonomy => tr(cx, "Full autonomy", "全自动执行"),
        }
    }
}

/// Summary KPI card rendered above an assistant chart.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RevenueChatKpi {
    /// Metric label (`"Revenue"`, `"New MRR"`, `"Churn"`).
    pub label: SharedString,
    /// Primary formatted metric (`"$1.24M"`).
    pub value: SharedString,
    /// Optional YoY or QoQ delta caption (`"+18.2% vs. last year"`).
    pub change: Option<SharedString>,
    /// Whether `change` represents a favorable movement.
    pub good: bool,
}

impl RevenueChatKpi {
    /// Construct a KPI metric card.
    pub fn new(
        label: impl Into<SharedString>,
        value: impl Into<SharedString>,
        change: impl Into<SharedString>,
        good: bool,
    ) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
            change: Some(change.into()),
            good,
        }
    }
}

/// One tier/segment slice inside a [`RevenueSplitBar`].
#[derive(Clone, Debug, PartialEq)]
pub struct RevenueSplitPart {
    /// Segment name (`"Enterprise"`, `"Growth"`, `"Starter"`).
    pub name: SharedString,
    /// Dollar contribution (`> 0.0`).
    pub value: f64,
}

impl RevenueSplitPart {
    /// Construct a named dollar slice.
    pub fn new(name: impl Into<SharedString>, value: f64) -> Self {
        Self {
            name: name.into(),
            value,
        }
    }
}

/// One stacked bar inside [`RevenueChartKind::Split`].
#[derive(Clone, Debug, PartialEq)]
pub struct RevenueSplitBar {
    /// Group label (`"Europe"`, `"North America"`).
    pub name: SharedString,
    /// Stacked dollar segments inside this bar.
    pub parts: Vec<RevenueSplitPart>,
}

impl RevenueSplitBar {
    /// Sum of all segments in dollars.
    pub fn total(&self) -> f64 {
        self.parts.iter().map(|p| p.value).sum()
    }
}

/// Interactive chart payload attached to an assistant turn.
#[derive(Clone, Debug, PartialEq)]
pub enum RevenueChartKind {
    /// Weekly YoY comparison area/line chart (values in `$K`).
    Trend {
        /// Point labels (`W27`–`W39`).
        labels: Vec<SharedString>,
        /// Current-period weekly values in `$K`.
        current: Vec<f64>,
        /// Prior-year weekly values in `$K`.
        previous: Vec<f64>,
        /// Default highlighted point index (`Some(8)` for `W35`).
        mark: Option<usize>,
    },
    /// Regional or driver stacked-bar breakdown (values in `$`).
    Split {
        /// Figure heading (`"Q3 revenue by region"`).
        title: SharedString,
        /// Optional prior-period total in `$` for delta calculation.
        previous: Option<f64>,
        /// Optional baseline label (`"Q2 2026"`).
        since: Option<SharedString>,
        /// Stacked columns.
        groups: Vec<RevenueSplitBar>,
    },
    /// Actual + dashed projection + confidence cone + target line (values in `$K`).
    Forecast {
        /// Point labels (`W36`–`W48`).
        labels: Vec<SharedString>,
        /// Realized weekly revenue in `$K`.
        actual: Vec<Option<f64>>,
        /// Projected weekly p50 revenue in `$K`.
        forecast: Vec<Option<f64>>,
        /// Optional horizontal weekly target in `$K`.
        target: Option<f64>,
    },
}

/// Brushed point range or selected bar passed when asking about part of a chart.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RevenueChartSelection {
    /// Assistant turn id owning the chart.
    pub turn_id: SharedString,
    /// Inclusive start index.
    pub from: usize,
    /// Inclusive end index.
    pub to: usize,
    /// Human-readable range label (`"W32 – W34"` or `"North America"`).
    pub label: SharedString,
    /// Computed metric summary (`"$284K · +18.3% on last year"`).
    pub detail: SharedString,
}

/// One user or assistant message inside [`RevenueChatBlock`].
#[derive(Clone, Debug, PartialEq)]
pub enum RevenueChatTurn {
    /// Right-aligned user question with optional chart-selection quote.
    User {
        /// Stable turn id.
        id: SharedString,
        /// Question text.
        text: SharedString,
        /// Optional brushed-range quote shown above the bubble.
        quote: Option<SharedString>,
    },
    /// Left-aligned analyst answer with KPI cards, chart, sources, and follow-up chips.
    Assistant {
        /// Stable turn id.
        id: SharedString,
        /// Plain-language narrative summary.
        text: SharedString,
        /// KPI summary strip.
        kpis: Vec<RevenueChatKpi>,
        /// Optional interactive chart.
        chart: Option<RevenueChartKind>,
        /// Optional figure caption.
        caption: Option<SharedString>,
        /// Data source badges (`"Stripe"`, `"HubSpot"`).
        sources: Vec<SharedString>,
        /// Clickable follow-up prompt chips.
        next: Vec<SharedString>,
        /// Thumbs-up state.
        liked: bool,
        /// Whether this turn is actively streaming.
        streaming: bool,
    },
}

/// Events emitted by [`RevenueChatBlock`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RevenueChatEvent {
    /// User submitted a question, optionally scoped to a brushed chart selection.
    Asked {
        /// Submitted question text.
        question: SharedString,
        /// Optional chart selection context.
        focus: Option<RevenueChartSelection>,
    },
    /// User brushed a week stretch or clicked a stacked bar on a chart.
    RangeSelected(RevenueChartSelection),
    /// User copied an assistant answer to the clipboard.
    CopiedAnswer(SharedString),
    /// User exported a chart's CSV snapshot to the clipboard.
    DownloadedChart(SharedString),
    /// User toggled thumbs-up feedback on an assistant turn.
    LikedTurn {
        /// Turn identifier.
        turn_id: SharedString,
        /// New liked state.
        liked: bool,
    },
    /// User cycled the composer approval policy.
    ApprovalPolicyChanged(RevenueApprovalPolicy),
}

#[derive(Clone, Debug)]
struct ActiveBrush {
    turn_id: SharedString,
    anchor: usize,
    from: usize,
    to: usize,
    is_bar: bool,
    ask_open: bool,
}

/// Compute human-readable range label and YoY summary for brushed indices `from..=to`.
pub fn compute_trend_selection(
    labels: &[SharedString],
    current: &[f64],
    previous: &[f64],
    from: usize,
    to: usize,
) -> (SharedString, SharedString) {
    if labels.is_empty() || current.is_empty() {
        return ("Selection".into(), "$0K".into());
    }
    let lo = from.min(to).min(labels.len() - 1);
    let hi = from.max(to).min(labels.len() - 1);
    let label: SharedString = if lo == hi {
        labels[lo].clone()
    } else {
        format!("{} – {}", labels[lo], labels[hi]).into()
    };
    let cur_sum: f64 = current[lo..=hi.min(current.len() - 1)].iter().sum();
    let prev_sum: f64 = if previous.is_empty() {
        0.0
    } else {
        previous[lo..=hi.min(previous.len() - 1)].iter().sum()
    };
    let sum_txt = format_k_dollars(cur_sum);
    let detail: SharedString = if prev_sum > 0.0 {
        let pct = (cur_sum - prev_sum) / prev_sum * 100.0;
        let sign = if pct >= 0.0 { "+" } else { "" };
        format!("{sum_txt} · {sign}{pct:.1}% on last year").into()
    } else {
        sum_txt.into()
    };
    (label, detail)
}

/// Default initial Q3 revenue analyst conversation.
pub fn default_revenue_turns() -> Vec<RevenueChatTurn> {
    vec![
        RevenueChatTurn::User {
            id: "u-1".into(),
            text: "How did revenue develop last quarter?".into(),
            quote: None,
        },
        RevenueChatTurn::Assistant {
            id: "a-1".into(),
            text: "Q3 closed at $1.24M – 18% more than a year ago. Growth was steady through August and jumped in the back-to-school week, then settled about 12% higher than before.".into(),
            kpis: vec![
                RevenueChatKpi::new("Revenue", "$1.24M", "+18.2% vs. last year", true),
                RevenueChatKpi::new("New MRR", "$86K", "+9.4% vs. last year", true),
                RevenueChatKpi::new("Churn", "2.1%", "-0.6% vs. last year", true),
            ],
            chart: Some(RevenueChartKind::Trend {
                labels: (27..=39).map(|w| SharedString::from(format!("W{w}"))).collect(),
                current: vec![
                    82.0, 86.0, 84.0, 89.0, 91.0, 94.0, 93.0, 97.0, 134.0, 104.0, 102.0, 106.0,
                    109.0,
                ],
                previous: vec![
                    74.0, 75.0, 73.0, 77.0, 78.0, 79.0, 81.0, 80.0, 84.0, 86.0, 85.0, 88.0, 90.0,
                ],
                mark: Some(8),
            }),
            caption: Some(
                "Weekly revenue · Q3 2026 against Q3 2025 · back to school in W35".into(),
            ),
            sources: vec!["Stripe".into(), "HubSpot".into()],
            next: vec![
                "Split it by region".into(),
                "Forecast Q4".into(),
                "Why did week 35 jump?".into(),
            ],
            liked: false,
            streaming: false,
        },
    ]
}

/// Synthesize a domain-accurate analyst turn for `question` and optional brushed `focus`.
pub fn synthesize_revenue_reply(
    turn_id: SharedString,
    question: &str,
    focus: Option<&RevenueChartSelection>,
) -> RevenueChatTurn {
    let lower = question.to_ascii_lowercase();

    if let Some(sel) = focus {
        return RevenueChatTurn::Assistant {
            id: turn_id,
            text: format!(
                "Across {} ({}), annual seat expansions and multi-workspace upgrades drove 64% of net volume, while involuntary churn stayed under 0.4%.",
                sel.label, sel.detail
            )
            .into(),
            kpis: vec![
                RevenueChatKpi::new("Selected window", sel.label.clone(), sel.detail.clone(), true),
                RevenueChatKpi::new("Annual share", "64%", "+7.1% vs. Q2", true),
                RevenueChatKpi::new("Net retention", "114%", "+2.4 pts YoY", true),
            ],
            chart: None,
            caption: None,
            sources: vec!["Stripe".into(), "HubSpot".into()],
            next: vec!["Split it by region".into(), "Forecast Q4".into()],
            liked: false,
            streaming: false,
        };
    }

    if lower.contains("region") || lower.contains("split") {
        return RevenueChatTurn::Assistant {
            id: turn_id,
            text: "Europe still brings in half, but North America grew fastest – up 31% on last quarter. Asia-Pacific is small and flat.".into(),
            kpis: vec![
                RevenueChatKpi::new("Europe", "$620K", "50.0% of Q3 total", true),
                RevenueChatKpi::new("North America", "$412K", "+31.0% vs. Q2", true),
                RevenueChatKpi::new("Asia-Pacific", "$136K", "+1.2% vs. Q2", false),
            ],
            chart: Some(RevenueChartKind::Split {
                title: "Q3 revenue by region".into(),
                previous: Some(1_049_000.0),
                since: Some("Q3 2025".into()),
                groups: vec![
                    RevenueSplitBar {
                        name: "Europe".into(),
                        parts: vec![
                            RevenueSplitPart::new("Scale", 340_000.0),
                            RevenueSplitPart::new("Growth", 196_000.0),
                            RevenueSplitPart::new("Starter", 84_000.0),
                        ],
                    },
                    RevenueSplitBar {
                        name: "North America".into(),
                        parts: vec![
                            RevenueSplitPart::new("Scale", 228_000.0),
                            RevenueSplitPart::new("Growth", 132_000.0),
                            RevenueSplitPart::new("Starter", 52_000.0),
                        ],
                    },
                    RevenueSplitBar {
                        name: "Asia-Pacific".into(),
                        parts: vec![
                            RevenueSplitPart::new("Scale", 64_000.0),
                            RevenueSplitPart::new("Growth", 48_000.0),
                            RevenueSplitPart::new("Starter", 24_000.0),
                        ],
                    },
                    RevenueSplitBar {
                        name: "Rest of world".into(),
                        parts: vec![
                            RevenueSplitPart::new("Scale", 28_000.0),
                            RevenueSplitPart::new("Growth", 28_000.0),
                            RevenueSplitPart::new("Starter", 16_000.0),
                        ],
                    },
                ],
            }),
            caption: None,
            sources: vec!["Stripe".into(), "HubSpot".into()],
            next: vec!["Forecast Q4".into(), "Why did week 35 jump?".into()],
            liked: false,
            streaming: false,
        };
    }

    if lower.contains("forecast") || lower.contains("q4") {
        return RevenueChatTurn::Assistant {
            id: turn_id,
            text: "Carrying September's 12% baseline lift into Q4 puts revenue on pace for $1.48M ($1.41M – $1.55M likely range), clearing the $1.40M Q4 plan by week 50.".into(),
            kpis: vec![
                RevenueChatKpi::new("Q4 forecast", "$1.48M", "+19.4% vs. Q3", true),
                RevenueChatKpi::new("Likely range", "$1.41M–$1.55M", "80% confidence", true),
                RevenueChatKpi::new("Plan coverage", "105.7%", "+$80K above target", true),
            ],
            chart: Some(RevenueChartKind::Forecast {
                labels: (36..=48).map(|w| SharedString::from(format!("W{w}"))).collect(),
                actual: vec![
                    Some(104.0),
                    Some(102.0),
                    Some(106.0),
                    Some(109.0),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                ],
                forecast: vec![
                    None,
                    None,
                    None,
                    Some(109.0),
                    Some(111.0),
                    Some(113.0),
                    Some(115.0),
                    Some(118.0),
                    Some(121.0),
                    Some(126.0),
                    Some(132.0),
                    Some(129.0),
                    Some(134.0),
                ],
                target: Some(115.0),
            }),
            caption: Some("Weekly Q4 projection ($K) · dashed target at $115K/wk".into()),
            sources: vec!["Stripe".into(), "HubSpot".into()],
            next: vec!["Split it by region".into(), "Why did week 35 jump?".into()],
            liked: false,
            streaming: false,
        };
    }

    RevenueChatTurn::Assistant {
        id: turn_id,
        text: "Week 35 jumped +38% ($134K vs. $97K in W34) when 14 annual education workspaces renewed together and the back-to-school Growth campaign converted 410 trial teams.".into(),
        kpis: vec![
            RevenueChatKpi::new("W35 total", "$134K", "+59.5% vs. W35 last year", true),
            RevenueChatKpi::new("EDU renewals", "$52K", "14 annual contracts", true),
            RevenueChatKpi::new("Trial conversions", "410", "+2.1x weekly average", true),
        ],
        chart: Some(RevenueChartKind::Split {
            title: "Week 35 revenue drivers".into(),
            previous: Some(84_000.0),
            since: Some("W35 2025".into()),
            groups: vec![
                RevenueSplitBar {
                    name: "EDU Annual".into(),
                    parts: vec![
                        RevenueSplitPart::new("Renewals", 38_000.0),
                        RevenueSplitPart::new("Seat expansion", 14_000.0),
                    ],
                },
                RevenueSplitBar {
                    name: "Self-serve Growth".into(),
                    parts: vec![
                        RevenueSplitPart::new("New annual", 29_000.0),
                        RevenueSplitPart::new("Monthly", 15_000.0),
                    ],
                },
                RevenueSplitBar {
                    name: "Scale".into(),
                    parts: vec![
                        RevenueSplitPart::new("New logos", 24_000.0),
                        RevenueSplitPart::new("Add-ons", 14_000.0),
                    ],
                },
            ],
        }),
        caption: None,
        sources: vec!["Stripe".into(), "HubSpot".into()],
        next: vec!["Split it by region".into(), "Forecast Q4".into()],
        liked: false,
        streaming: false,
    }
}

/// Standalone conversational revenue analyst block with brush-to-ask charts and prompt composer.
///
/// # Examples
///
/// ```ignore
/// use kui::components::revenue_chat::RevenueChatBlock;
///
/// let chat = cx.new(|cx| RevenueChatBlock::new("revenue-chat", cx));
/// ```
pub struct RevenueChatBlock {
    id: ElementId,
    turns: Vec<RevenueChatTurn>,
    approval_policy: RevenueApprovalPolicy,
    model_index: usize,
    attached_context: bool,
    recording: bool,
    hovered_point: Option<(SharedString, usize)>,
    keyboard_point: usize,
    active_brush: Option<ActiveBrush>,
    dragging_brush: bool,
    plot_bounds: HashMap<SharedString, Rc<Cell<Bounds<Pixels>>>>,
    composer_editor: Option<Entity<Editor>>,
    brush_editor: Option<Entity<Editor>>,
    editor_focused: bool,
    stream_state: Option<(SharedString, SharedString, Instant)>,
    copied_turn: Option<(SharedString, Instant)>,
    toast: Option<(SharedString, Instant)>,
    loading: bool,
    empty: bool,
    boot_at: Instant,
    focus: FocusHandle,
    scroll: ScrollHandle,
    measured_width: Rc<Cell<f32>>,
}

impl EventEmitter<RevenueChatEvent> for RevenueChatBlock {}

impl RevenueChatBlock {
    /// Create a standalone [`RevenueChatBlock`] preloaded with the Q3 revenue trend thread.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<UiLanguage>(|_, cx| cx.notify())
            .detach();
        Self {
            id: id.into(),
            turns: default_revenue_turns(),
            approval_policy: RevenueApprovalPolicy::RequestApproval,
            model_index: 0,
            attached_context: false,
            recording: false,
            hovered_point: None,
            keyboard_point: 8,
            active_brush: None,
            dragging_brush: false,
            plot_bounds: HashMap::new(),
            composer_editor: None,
            brush_editor: None,
            editor_focused: false,
            stream_state: None,
            copied_turn: None,
            toast: None,
            loading: false,
            empty: false,
            boot_at: Instant::now(),
            focus: cx.focus_handle(),
            scroll: ScrollHandle::new(),
            measured_width: Rc::new(Cell::new(894.0)),
        }
    }

    /// Replace the initial conversation turns.
    pub fn with_turns(mut self, turns: Vec<RevenueChatTurn>) -> Self {
        self.turns = turns;
        self
    }

    /// Set the initial composer [`RevenueApprovalPolicy`].
    pub fn with_approval_policy(mut self, policy: RevenueApprovalPolicy) -> Self {
        self.approval_policy = policy;
        self
    }

    /// Configure skeleton loading state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Configure empty new-chat starter state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Return the current conversation turns.
    pub fn turns(&self) -> &[RevenueChatTurn] {
        &self.turns
    }

    /// Return the active composer [`RevenueApprovalPolicy`].
    pub fn approval_policy(&self) -> RevenueApprovalPolicy {
        self.approval_policy
    }

    /// Submit a question (optionally scoped to a brushed chart selection) and append an analyst reply.
    pub fn ask(
        &mut self,
        question: impl Into<String>,
        focus: Option<RevenueChartSelection>,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.loading {
            return false;
        }
        let trimmed = question.into().trim().to_owned();
        if trimmed.is_empty() {
            return false;
        }
        self.empty = false;
        self.active_brush = None;
        self.dragging_brush = false;
        self.attached_context = false;

        let quote = focus
            .as_ref()
            .map(|f| SharedString::from(format!("{} · {}", f.label, f.detail)));
        let next_idx = self.turns.len() + 1;
        let q_shared: SharedString = trimmed.clone().into();
        self.turns.push(RevenueChatTurn::User {
            id: format!("u-{next_idx}").into(),
            text: q_shared.clone(),
            quote,
        });

        let ans_id: SharedString = format!("a-{}", next_idx + 1).into();
        let mut reply = synthesize_revenue_reply(ans_id.clone(), &trimmed, focus.as_ref());
        let reduced = Theme::global(cx).reduced_motion;
        if !reduced
            && let RevenueChatTurn::Assistant {
                text, streaming, ..
            } = &mut reply
        {
            let full = text.clone();
            *text = SharedString::default();
            *streaming = true;
            self.stream_state = Some((ans_id, full, Instant::now()));
        }
        self.turns.push(reply);
        self.scroll.scroll_to_bottom();

        cx.emit(RevenueChatEvent::Asked {
            question: q_shared,
            focus,
        });
        cx.notify();
        true
    }

    fn ensure_editors(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.composer_editor.is_none() {
            let ph: SharedString = tr(
                cx,
                "Ask about revenue, customers or plans...",
                "询问营收趋势、客户留存或订阅方案…",
            )
            .into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Single, w, cx);
                e.set_placeholder(ph);
                e.font_size = 13.5;
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
                EditEvent::Changed => cx.notify(),
                EditEvent::Submitted => {
                    let val = ed_ent.read(cx).value().to_string();
                    if this.ask(val, None, cx) {
                        ed_ent.update(cx, |e, cx| e.set_value("", w, cx));
                    }
                }
                _ => {}
            })
            .detach();
            self.composer_editor = Some(ed);
        }

        if self.brush_editor.is_none() {
            let ph: SharedString = tr(cx, "Ask about this selection…", "针对所选区间提问…").into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Single, w, cx);
                e.set_placeholder(ph);
                e.font_size = 13.0;
                e
            });
            cx.subscribe_in(&ed, w, |this, ed_ent, ev: &EditEvent, w, cx| match ev {
                EditEvent::Submitted => {
                    let val = ed_ent.read(cx).value().to_string();
                    let sel = this
                        .active_brush
                        .as_ref()
                        .and_then(|br| this.selection_for_brush(br));
                    let fallback = sel
                        .as_ref()
                        .map(|s| format!("What drove {}?", s.label))
                        .unwrap_or_default();
                    let q = if val.trim().is_empty() { fallback } else { val };
                    if this.ask(q, sel, cx) {
                        ed_ent.update(cx, |e, cx| e.set_value("", w, cx));
                    }
                }
                EditEvent::Escape => {
                    this.active_brush = None;
                    this.dragging_brush = false;
                    cx.notify();
                }
                EditEvent::Changed => cx.notify(),
                _ => {}
            })
            .detach();
            self.brush_editor = Some(ed);
        }
    }

    fn advance_animations(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let reduced = Theme::global(cx).reduced_motion;
        if let Some((tid, full_txt, started)) = self.stream_state.clone() {
            let ms = started.elapsed().as_secs_f32() * 1000.0;
            if reduced || ms >= 980.0 {
                for turn in &mut self.turns {
                    if let RevenueChatTurn::Assistant {
                        id,
                        text,
                        streaming,
                        ..
                    } = turn
                        && id == &tid
                    {
                        *text = full_txt.clone();
                        *streaming = false;
                    }
                }
                self.stream_state = None;
            } else {
                if ms > 260.0 {
                    let frac = ((ms - 260.0) / 720.0).clamp(0.0, 1.0);
                    let total = full_txt.chars().count();
                    let count = ((total as f32 * frac).ceil() as usize).min(total);
                    let partial: String = full_txt.chars().take(count).collect();
                    for turn in &mut self.turns {
                        if let RevenueChatTurn::Assistant { id, text, .. } = turn
                            && id == &tid
                        {
                            *text = partial.clone().into();
                        }
                    }
                }
                w.request_animation_frame();
            }
        }
        if self
            .toast
            .as_ref()
            .is_some_and(|(_, t)| t.elapsed().as_secs_f32() >= 2.45)
        {
            self.toast = None;
        }
        if self
            .copied_turn
            .as_ref()
            .is_some_and(|(_, t)| t.elapsed().as_secs_f32() >= 2.0)
        {
            self.copied_turn = None;
        } else if self.copied_turn.is_some() && !reduced {
            w.request_animation_frame();
        }
        if (self.loading || self.recording) && !reduced {
            w.request_animation_frame();
        }
    }

    fn selection_for_brush(&self, br: &ActiveBrush) -> Option<RevenueChartSelection> {
        let chart = self.turns.iter().find_map(|t| match t {
            RevenueChatTurn::Assistant { id, chart, .. } if id == &br.turn_id => chart.as_ref(),
            _ => None,
        })?;
        match chart {
            RevenueChartKind::Trend {
                labels,
                current,
                previous,
                ..
            } => {
                let (label, detail) =
                    compute_trend_selection(labels, current, previous, br.from, br.to);
                Some(RevenueChartSelection {
                    turn_id: br.turn_id.clone(),
                    from: br.from.min(br.to),
                    to: br.from.max(br.to),
                    label,
                    detail,
                })
            }
            RevenueChartKind::Forecast {
                labels,
                actual,
                forecast,
                ..
            } => {
                if labels.is_empty() {
                    return None;
                }
                let lo = br.from.min(br.to).min(labels.len() - 1);
                let hi = br.from.max(br.to).min(labels.len() - 1);
                let label: SharedString = if lo == hi {
                    labels[lo].clone()
                } else {
                    format!("{} – {}", labels[lo], labels[hi]).into()
                };
                let mut sum = 0.0;
                for idx in lo..=hi {
                    let v = actual
                        .get(idx)
                        .and_then(|x| *x)
                        .or_else(|| forecast.get(idx).and_then(|x| *x))
                        .unwrap_or(0.0);
                    sum += v;
                }
                Some(RevenueChartSelection {
                    turn_id: br.turn_id.clone(),
                    from: lo,
                    to: hi,
                    label,
                    detail: format!("{} projected", format_k_dollars(sum)).into(),
                })
            }
            RevenueChartKind::Split { groups, .. } => {
                let idx = br.from.min(groups.len().saturating_sub(1));
                let grp = groups.get(idx)?;
                let grand: f64 = groups.iter().map(RevenueSplitBar::total).sum();
                let tot = grp.total();
                let pct = if grand > 0.0 {
                    (tot / grand * 100.0).round() as i32
                } else {
                    0
                };
                Some(RevenueChartSelection {
                    turn_id: br.turn_id.clone(),
                    from: idx,
                    to: idx,
                    label: grp.name.clone(),
                    detail: format!("{} · {pct}% share", format_dollars_compact(tot)).into(),
                })
            }
        }
    }

    fn point_index_from_x(&self, turn_id: &SharedString, count: usize, x: Pixels) -> Option<usize> {
        if count == 0 {
            return None;
        }
        let b = self.plot_bounds.get(turn_id)?.get();
        let w = f32::from(b.size.width);
        if w <= 4.0 {
            return None;
        }
        let rel = ((f32::from(x - b.left())) / w).clamp(0.0, 1.0);
        let idx = (rel * (count.saturating_sub(1)) as f32).round() as usize;
        Some(idx.min(count - 1))
    }

    fn finish_brush(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if !self.dragging_brush {
            return;
        }
        self.dragging_brush = false;
        if let Some(br) = &mut self.active_brush {
            br.ask_open = true;
        }
        if let Some(br) = self.active_brush.clone()
            && let Some(sel) = self.selection_for_brush(&br)
        {
            cx.emit(RevenueChatEvent::RangeSelected(sel));
        }
        if let Some(ed) = &self.brush_editor {
            ed.update(cx, |e, cx| e.set_value("", w, cx));
            let fh = ed.read(cx).focus.clone();
            fh.focus(w, cx);
        }
        cx.notify();
    }

    fn handle_key_down(&mut self, ev: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        let key = ev.keystroke.key.as_str();
        if key == "escape" {
            if self.active_brush.is_some() || self.dragging_brush {
                self.active_brush = None;
                self.dragging_brush = false;
                cx.notify();
            }
            return;
        }
        if self.editor_focused {
            return;
        }
        if key == "/" {
            if let Some(ed) = &self.composer_editor {
                let fh = ed.read(cx).focus.clone();
                fh.focus(w, cx);
                w.prevent_default();
            }
            return;
        }

        let Some((last_id, count, is_split)) = self.turns.iter().rev().find_map(|t| match t {
            RevenueChatTurn::Assistant {
                id, chart: Some(c), ..
            } => match c {
                RevenueChartKind::Trend { labels, .. }
                | RevenueChartKind::Forecast { labels, .. } => {
                    Some((id.clone(), labels.len(), false))
                }
                RevenueChartKind::Split { groups, .. } => Some((id.clone(), groups.len(), true)),
            },
            _ => None,
        }) else {
            return;
        };
        if count == 0 {
            return;
        }

        match key {
            "left" => {
                self.keyboard_point = self.keyboard_point.min(count - 1).saturating_sub(1);
                self.hovered_point = Some((last_id.clone(), self.keyboard_point));
                if let Some(br) = &mut self.active_brush
                    && br.turn_id == last_id
                    && !br.is_bar
                    && !br.ask_open
                {
                    br.from = br.anchor.min(self.keyboard_point);
                    br.to = br.anchor.max(self.keyboard_point);
                }
                cx.notify();
            }
            "right" => {
                self.keyboard_point = (self.keyboard_point + 1).min(count - 1);
                self.hovered_point = Some((last_id.clone(), self.keyboard_point));
                if let Some(br) = &mut self.active_brush
                    && br.turn_id == last_id
                    && !br.is_bar
                    && !br.ask_open
                {
                    br.from = br.anchor.min(self.keyboard_point);
                    br.to = br.anchor.max(self.keyboard_point);
                }
                cx.notify();
            }
            "home" => {
                self.keyboard_point = 0;
                self.hovered_point = Some((last_id, 0));
                cx.notify();
            }
            "end" => {
                self.keyboard_point = count - 1;
                self.hovered_point = Some((last_id, self.keyboard_point));
                cx.notify();
            }
            "enter" => {
                let pt = self.keyboard_point.min(count - 1);
                if is_split {
                    let br = ActiveBrush {
                        turn_id: last_id,
                        anchor: pt,
                        from: pt,
                        to: pt,
                        is_bar: true,
                        ask_open: true,
                    };
                    if let Some(sel) = self.selection_for_brush(&br) {
                        cx.emit(RevenueChatEvent::RangeSelected(sel));
                    }
                    self.active_brush = Some(br);
                } else if let Some(mut br) = self.active_brush.clone()
                    && br.turn_id == last_id
                    && !br.ask_open
                {
                    br.ask_open = true;
                    if let Some(sel) = self.selection_for_brush(&br) {
                        cx.emit(RevenueChatEvent::RangeSelected(sel));
                    }
                    self.active_brush = Some(br);
                } else {
                    self.active_brush = Some(ActiveBrush {
                        turn_id: last_id,
                        anchor: pt,
                        from: pt,
                        to: pt,
                        is_bar: false,
                        ask_open: false,
                    });
                }
                cx.notify();
            }
            _ => {}
        }
    }

    fn render_kpi_strip(kpis: &[RevenueChatKpi], narrow: bool, p: &ProPalette) -> Div {
        let edge = p.line;
        let make_card = |kpi: &RevenueChatKpi| {
            div()
                .flex_1()
                .min_w(px(if narrow { 130. } else { 150. }))
                .px(px(14.))
                .py(px(12.))
                .rounded(px(12.))
                .bg(p.card)
                .border_1()
                .border_color(edge)
                .flex()
                .flex_col()
                .gap(px(2.))
                .child(
                    div()
                        .text_size(px(12.))
                        .line_height(px(16.))
                        .text_color(p.muted)
                        .truncate()
                        .child(kpi.label.clone()),
                )
                .child(
                    div()
                        .text_size(px(20.))
                        .line_height(px(28.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.ink)
                        .truncate()
                        .child(kpi.value.clone()),
                )
                .when_some(kpi.change.clone(), |d, chg| {
                    d.child(
                        div()
                            .text_size(px(12.))
                            .line_height(px(16.))
                            .text_color(if kpi.good { p.green_ink } else { p.red_ink })
                            .truncate()
                            .child(chg),
                    )
                })
        };

        if narrow && kpis.len() == 3 {
            div()
                .w_full()
                .flex()
                .flex_col()
                .gap(px(8.))
                .child(make_card(&kpis[0]))
                .child(
                    div()
                        .w_full()
                        .flex()
                        .gap(px(8.))
                        .child(make_card(&kpis[1]))
                        .child(make_card(&kpis[2])),
                )
        } else {
            div()
                .w_full()
                .flex()
                .flex_wrap()
                .gap(px(10.))
                .children(kpis.iter().map(make_card))
        }
    }

    fn render_brush_ask_bar(
        &self,
        sel: RevenueChartSelection,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let ed = self.brush_editor.clone()?;
        let sel_submit = sel.clone();
        let is_open = self.active_brush.as_ref().is_some_and(|b| b.ask_open);
        let turn_id = sel.turn_id.clone();

        let bar = div()
            .px(px(12.))
            .py(px(8.))
            .rounded(px(11.))
            .bg(if p.dark {
                rgb(0x232327).into()
            } else {
                p.subtle
            })
            .border_1()
            .border_color(p.blue.opacity(0.35))
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(8.))
            .child(
                div()
                    .px(px(8.))
                    .py(px(2.))
                    .rounded(px(6.))
                    .bg(p.accent_soft)
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.blue)
                    .child(format!("{} · {}", sel.label, sel.detail)),
            )
            .child(div().flex_1().min_w(px(140.)).h(px(22.)).child(ed.clone()))
            .child(pro_btn(
                pro_eid(&self.id, format!("brush-ask-go-{turn_id}")),
                tr(cx, "Ask", "提问"),
                None,
                ProBtnStyle::Primary,
                true,
                cx,
                move |this, w, cx| {
                    let raw = ed.read(cx).value().to_string();
                    let q = if raw.trim().is_empty() {
                        format!("What drove {}?", sel_submit.label)
                    } else {
                        raw
                    };
                    if this.ask(q, Some(sel_submit.clone()), cx) {
                        ed.update(cx, |e, cx| e.set_value("", w, cx));
                    }
                },
            ))
            .child(pro_icon_btn(
                pro_eid(&self.id, format!("brush-ask-close-{turn_id}")),
                tr(cx, "Clear selection", "清除所选区间"),
                ProIcon::Close,
                24.,
                false,
                cx,
                |this, _, cx| {
                    this.active_brush = None;
                    this.dragging_brush = false;
                    cx.notify();
                },
            ))
            .into_any_element();

        Some(pro_presence(
            pro_eid(&self.id, format!("brush-pres-{turn_id}")),
            is_open,
            0.18,
            0.13,
            move |amt| {
                let e = pro_ease(amt);
                div()
                    .w_full()
                    .opacity(e)
                    .relative()
                    .top(px(-4. * (1. - e)))
                    .child(bar)
                    .into_any_element()
            },
        ))
    }

    #[allow(clippy::too_many_arguments)]
    fn render_trend_chart(
        &mut self,
        turn_id: &SharedString,
        caption: Option<SharedString>,
        labels: Vec<SharedString>,
        current: Vec<f64>,
        previous: Vec<f64>,
        mark: Option<usize>,
        narrow: bool,
        reduced: bool,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let p = ProPalette::get(cx);
        let n = current.len();
        let bounds_cell = self
            .plot_bounds
            .entry(turn_id.clone())
            .or_insert_with(|| Rc::new(Cell::new(Bounds::default())))
            .clone();

        let active_br = self
            .active_brush
            .as_ref()
            .filter(|b| &b.turn_id == turn_id && !b.is_bar)
            .cloned();

        let active_idx = if active_br.is_some() {
            None
        } else {
            self.hovered_point
                .as_ref()
                .filter(|(tid, idx)| tid == turn_id && *idx < n)
                .map(|(_, idx)| *idx)
                .or_else(|| mark.filter(|&m| m < n))
        };

        let y_min = 50.0_f32;
        let y_max = 150.0_f32;
        let y_span = y_max - y_min;
        let denom = (n.saturating_sub(1)).max(1) as f32;

        let mut cur_norm = Vec::with_capacity(n);
        for (i, &v) in current.iter().enumerate() {
            let ny = ((v as f32 - y_min) / y_span).clamp(0.0, 1.0);
            cur_norm.push(
                pro_spring_f32(
                    pro_eid(&self.id, format!("rc-cy-{turn_id}-{i}")),
                    ny,
                    0.26,
                    w,
                    cx,
                )
                .clamp(0.0, 1.0),
            );
        }
        let mut prev_norm = Vec::with_capacity(previous.len());
        for (i, &v) in previous.iter().enumerate() {
            let ny = ((v as f32 - y_min) / y_span).clamp(0.0, 1.0);
            prev_norm.push(
                pro_spring_f32(
                    pro_eid(&self.id, format!("rc-py-{turn_id}-{i}")),
                    ny,
                    0.26,
                    w,
                    cx,
                )
                .clamp(0.0, 1.0),
            );
        }

        let fb_idx = active_idx.or(mark).unwrap_or(0).min(n.saturating_sub(1));
        let marker_tx = pro_spring_f32(
            pro_eid(&self.id, format!("rc-mtx-{turn_id}")),
            fb_idx as f32 / denom,
            0.16,
            w,
            cx,
        )
        .clamp(0.0, 1.0);
        let marker_cy = pro_spring_f32(
            pro_eid(&self.id, format!("rc-mcy-{turn_id}")),
            cur_norm.get(fb_idx).copied().unwrap_or(0.5),
            0.16,
            w,
            cx,
        );
        let marker_py = pro_spring_f32(
            pro_eid(&self.id, format!("rc-mpy-{turn_id}")),
            prev_norm.get(fb_idx).copied().unwrap_or(0.4),
            0.16,
            w,
            cx,
        );

        let brush_band = active_br.as_ref().map(|br| {
            let lo = br.from.min(br.to) as f32 / denom;
            let hi = br.from.max(br.to) as f32 / denom;
            (lo, hi, br.from == br.to)
        });

        let reveal = if reduced {
            1.0
        } else {
            pro_ease((self.boot_at.elapsed().as_secs_f32() / 0.80).clamp(0.0, 1.0))
        };

        let blue_c = p.blue;
        let prev_c = if p.dark {
            hsla(0., 0., 1., 0.34)
        } else {
            hsla(0., 0., 0., 0.28)
        };
        let grid_c = p.line;
        let guide_c = p.line_strong;
        let card_bg = p.card;
        let show_marker = active_idx.is_some();
        let bounds_writer = bounds_cell.clone();

        let chart_canvas = canvas(
            move |b, _, _| {
                bounds_writer.set(b);
            },
            move |bounds, _, window, _| {
                let bw = f32::from(bounds.size.width);
                let bh = f32::from(bounds.size.height);
                if bw <= 8.0 || bh <= 8.0 || cur_norm.len() < 2 {
                    return;
                }
                let pt_at = |i: usize, ny: f32| -> Point<Pixels> {
                    let tx = i as f32 / (cur_norm.len() - 1) as f32;
                    let ty = 1.0 - ny.clamp(0.0, 1.0);
                    point(bounds.left() + px(tx * bw), bounds.top() + px(ty * bh))
                };

                for frac in [0.0_f32, 0.5, 1.0] {
                    let gy = bounds.top() + px(frac * bh);
                    let mut gp = PathBuilder::stroke(px(1.0));
                    gp.move_to(point(bounds.left(), gy));
                    gp.line_to(point(bounds.right(), gy));
                    if let Ok(path) = gp.build() {
                        window.paint_path(path, grid_c);
                    }
                }

                if let Some((lo_f, hi_f, single)) = brush_band {
                    let x0 = if single {
                        (lo_f * bw - 12.0).max(0.0)
                    } else {
                        lo_f * bw
                    };
                    let x1 = if single {
                        (hi_f * bw + 12.0).min(bw)
                    } else {
                        hi_f * bw
                    };
                    let rect = Bounds::new(
                        point(bounds.left() + px(x0), bounds.top()),
                        size(px((x1 - x0).max(2.0)), px(bh)),
                    );
                    window.paint_quad(fill(rect, blue_c.opacity(0.11)));
                    let mut edge_pb = PathBuilder::stroke(px(1.0));
                    edge_pb.move_to(point(bounds.left() + px(x0), bounds.top()));
                    edge_pb.line_to(point(bounds.left() + px(x0), bounds.bottom()));
                    edge_pb.move_to(point(bounds.left() + px(x1), bounds.top()));
                    edge_pb.line_to(point(bounds.left() + px(x1), bounds.bottom()));
                    if let Ok(path) = edge_pb.build() {
                        window.paint_path(path, blue_c.opacity(0.58));
                    }
                }

                paint_dashed_polyline(window, &prev_norm, pt_at, prev_c, 1.25, 3.0, 3.0);

                let vis_n = (((cur_norm.len() - 1) as f32 * reveal).ceil() as usize + 1)
                    .clamp(2, cur_norm.len());
                let mut area = PathBuilder::fill();
                let first = pt_at(0, cur_norm[0]);
                let last = pt_at(vis_n - 1, cur_norm[vis_n - 1]);
                area.move_to(point(first.x, bounds.bottom()));
                area.line_to(first);
                for (i, &ny) in cur_norm.iter().enumerate().take(vis_n).skip(1) {
                    area.line_to(pt_at(i, ny));
                }
                area.line_to(point(last.x, bounds.bottom()));
                area.close();
                if let Ok(path) = area.build() {
                    window.paint_path(
                        path,
                        linear_gradient(
                            180.0,
                            linear_color_stop(blue_c.opacity(0.20), 0.0),
                            linear_color_stop(blue_c.opacity(0.0), 1.0),
                        ),
                    );
                }

                let mut line = PathBuilder::stroke(px(1.7));
                for (i, &ny) in cur_norm.iter().enumerate().take(vis_n) {
                    let pt = pt_at(i, ny);
                    if i == 0 {
                        line.move_to(pt);
                    } else {
                        line.line_to(pt);
                    }
                }
                if let Ok(path) = line.build() {
                    window.paint_path(path, blue_c);
                }

                if show_marker && fb_idx < vis_n {
                    let cur_pt = point(
                        bounds.left() + px(marker_tx * bw),
                        bounds.top() + px((1.0 - marker_cy) * bh),
                    );
                    let prev_pt = point(
                        bounds.left() + px(marker_tx * bw),
                        bounds.top() + px((1.0 - marker_py) * bh),
                    );
                    let mut gp = PathBuilder::stroke(px(1.0));
                    gp.move_to(point(cur_pt.x, bounds.top()));
                    gp.line_to(point(cur_pt.x, bounds.bottom()));
                    if let Ok(path) = gp.build() {
                        window.paint_path(path, guide_c);
                    }
                    if !prev_norm.is_empty() {
                        paint_ringed_dot(window, prev_pt, 3.0, 2.0, prev_c, card_bg);
                    }
                    paint_ringed_dot(window, cur_pt, 4.0, 2.0, blue_c, card_bg);
                }
            },
        )
        .size_full();

        let tooltip = (show_marker && n > 0).then(|| {
            let on_left = marker_tx > 0.54;
            let cur_txt = format_k_dollars(current[fb_idx]);
            let prev_txt = previous
                .get(fb_idx)
                .copied()
                .map(format_k_dollars)
                .unwrap_or_default();
            div()
                .absolute()
                .top(px(-4.))
                .when(on_left, |d| d.right(relative(1.0 - marker_tx)).mr(px(12.)))
                .when(!on_left, |d| d.left(relative(marker_tx)).ml(px(12.)))
                .min_w(px(144.))
                .px(px(10.))
                .py(px(7.))
                .rounded(px(10.))
                .bg(if p.dark { rgb(0x252528).into() } else { p.card })
                .border_1()
                .border_color(p.line_strong)
                .flex()
                .flex_col()
                .gap(px(4.))
                .child(
                    div()
                        .text_size(px(12.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.strong)
                        .child(labels[fb_idx].clone()),
                )
                .child(div().h(px(1.)).bg(p.line))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(12.))
                        .text_size(px(12.))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .text_color(p.text)
                                .child(div().size(px(7.)).rounded(px(2.)).bg(blue_c))
                                .child(tr(cx, "This year", "今年")),
                        )
                        .child(
                            div()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.ink)
                                .child(cur_txt),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(12.))
                        .text_size(px(12.))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .text_color(p.text)
                                .child(div().w(px(7.)).h(px(2.)).rounded(px(1.)).bg(prev_c))
                                .child(tr(cx, "Last year", "去年")),
                        )
                        .child(
                            div()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.ink)
                                .child(prev_txt),
                        ),
                )
        });

        let ask_bar = active_br
            .as_ref()
            .filter(|b| b.ask_open)
            .and_then(|br| self.selection_for_brush(br))
            .and_then(|sel| self.render_brush_ask_bar(sel, &p, cx));

        let tid_down = turn_id.clone();
        let tid_move = turn_id.clone();
        let tid_leave = turn_id.clone();
        let interactive_plot = div()
            .id(pro_eid(&self.id, format!("trend-plot-{turn_id}")))
            .role(Role::Figure)
            .aria_label(
                caption
                    .clone()
                    .unwrap_or_else(|| SharedString::from("Weekly revenue trend")),
            )
            .relative()
            .w_full()
            .h(px(152.))
            .cursor_crosshair()
            .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                if !*inside
                    && !this.dragging_brush
                    && this
                        .hovered_point
                        .as_ref()
                        .is_some_and(|(t, _)| t == &tid_leave)
                {
                    this.hovered_point = None;
                    cx.notify();
                }
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, ev: &MouseDownEvent, _, cx| {
                    if let Some(pi) = this.point_index_from_x(&tid_down, n, ev.position.x) {
                        this.dragging_brush = true;
                        this.keyboard_point = pi;
                        this.active_brush = Some(ActiveBrush {
                            turn_id: tid_down.clone(),
                            anchor: pi,
                            from: pi,
                            to: pi,
                            is_bar: false,
                            ask_open: false,
                        });
                        cx.notify();
                    }
                }),
            )
            .on_mouse_move(cx.listener(move |this, ev: &MouseMoveEvent, _, cx| {
                if let Some(pi) = this.point_index_from_x(&tid_move, n, ev.position.x) {
                    this.hovered_point = Some((tid_move.clone(), pi));
                    this.keyboard_point = pi;
                    if this.dragging_brush
                        && ev.pressed_button == Some(MouseButton::Left)
                        && let Some(br) = &mut this.active_brush
                        && br.turn_id == tid_move
                    {
                        br.from = br.anchor.min(pi);
                        br.to = br.anchor.max(pi);
                    }
                    cx.notify();
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _: &MouseUpEvent, w, cx| {
                    this.finish_brush(w, cx);
                }),
            )
            .child(chart_canvas)
            .when_some(tooltip, |d, tip| d.child(tip));

        let first_lbl = labels.first().cloned().unwrap_or_default();
        let mid_lbl = labels.get(n / 2).cloned().unwrap_or_default();
        let last_lbl = labels.last().cloned().unwrap_or_default();

        div()
            .px(px(if narrow { 10. } else { 14. }))
            .pt(px(14.))
            .pb(px(10.))
            .rounded(px(14.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .flex()
            .flex_col()
            .gap(px(10.))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .child(
                        div()
                            .text_size(px(12.5))
                            .text_color(p.muted)
                            .child(caption.unwrap_or_default()),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(12.))
                            .text_size(px(12.))
                            .text_color(p.muted)
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .child(div().w(px(10.)).h(px(3.)).rounded(px(1.5)).bg(blue_c))
                                    .child(tr(cx, "This year", "今年")),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .child(div().w(px(10.)).h(px(2.)).rounded(px(1.)).bg(prev_c))
                                    .child(tr(cx, "Last year", "去年")),
                            ),
                    ),
            )
            .when_some(ask_bar, |d, b| d.child(b))
            .child(
                div()
                    .w_full()
                    .flex()
                    .gap(px(10.))
                    .child(
                        div()
                            .h(px(152.))
                            .flex_none()
                            .flex()
                            .flex_col()
                            .justify_between()
                            .items_end()
                            .text_size(px(11.))
                            .text_color(p.muted)
                            .child("$150K")
                            .child("$100K")
                            .child("$50K"),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap(px(8.))
                            .child(interactive_plot)
                            .child(
                                div()
                                    .w_full()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .text_size(px(11.))
                                    .text_color(p.muted)
                                    .child(first_lbl)
                                    .child(mid_lbl)
                                    .child(last_lbl),
                            ),
                    ),
            )
            .child(div().text_size(px(12.)).text_color(p.faint).child(tr(
                cx,
                "Drag across the chart to ask about those weeks",
                "横向拖动图表区间即可针对所选周追问",
            )))
    }

    #[allow(clippy::too_many_arguments)]
    fn render_split_chart(
        &self,
        turn_id: &SharedString,
        title: SharedString,
        previous: Option<f64>,
        since: Option<SharedString>,
        groups: Vec<RevenueSplitBar>,
        narrow: bool,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let p = ProPalette::get(cx);
        let seg_colors = [p.blue, p.purple, p.teal, p.amber];
        let grand_total: f64 = groups.iter().map(RevenueSplitBar::total).sum();
        let max_bar = groups
            .iter()
            .map(RevenueSplitBar::total)
            .fold(1.0_f64, f64::max);

        let picked_bar = self
            .active_brush
            .as_ref()
            .filter(|b| &b.turn_id == turn_id && b.is_bar)
            .map(|b| b.from);

        let ask_bar = self
            .active_brush
            .as_ref()
            .filter(|b| &b.turn_id == turn_id && b.is_bar && b.ask_open)
            .and_then(|br| self.selection_for_brush(br))
            .and_then(|sel| self.render_brush_ask_bar(sel, &p, cx));

        let mut part_totals: Vec<(SharedString, f64)> = Vec::new();
        for g in &groups {
            for pt in &g.parts {
                if let Some(existing) = part_totals.iter_mut().find(|(n, _)| n == &pt.name) {
                    existing.1 += pt.value;
                } else {
                    part_totals.push((pt.name.clone(), pt.value));
                }
            }
        }

        let delta_chip = previous.filter(|&pv| pv > 0.0).map(|pv| {
            let pct = (grand_total - pv) / pv * 100.0;
            let sign = if pct >= 0.0 { "+" } else { "" };
            let txt = match &since {
                Some(s) => format!("{sign}{pct:.1}% vs. {s}"),
                None => format!("{sign}{pct:.1}%"),
            };
            div()
                .px(px(6.))
                .py(px(1.))
                .rounded(px(5.))
                .bg(if pct >= 0.0 { p.green_soft } else { p.red_soft })
                .text_size(px(11.5))
                .font_weight(FontWeight::MEDIUM)
                .text_color(if pct >= 0.0 { p.green_ink } else { p.red_ink })
                .child(txt)
        });

        let bars = groups.iter().enumerate().map(|(gi, grp)| {
            let tot = grp.total();
            let ratio = (tot / max_bar).clamp(0.06, 1.0) as f32;
            let anim_h = pro_spring_f32(
                pro_eid(&self.id, format!("rc-sbar-{turn_id}-{gi}")),
                ratio * 132.0,
                0.30,
                w,
                cx,
            );
            let is_picked = picked_bar == Some(gi);
            let tid_click = turn_id.clone();

            let stack = div()
                .w_full()
                .max_w(px(64.))
                .h(px(anim_h.max(8.0)))
                .rounded(px(7.))
                .overflow_hidden()
                .when(is_picked, |d| d.border_2().border_color(p.blue))
                .flex()
                .flex_col()
                .gap(px(1.5))
                .children(grp.parts.iter().enumerate().map(|(pi, pt)| {
                    let share = if tot > 0.0 {
                        (pt.value / tot).clamp(0.05, 1.0) as f32
                    } else {
                        0.33
                    };
                    div()
                        .w_full()
                        .h(relative(share))
                        .bg(seg_colors[pi % seg_colors.len()])
                }));

            activate(
                div()
                    .id(pro_eid(&self.id, format!("rc-split-col-{turn_id}-{gi}")))
                    .role(Role::Button)
                    .aria_label(format!("{} {}", grp.name, format_dollars_compact(tot)))
                    .tab_index(0)
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .px(px(4.))
                    .rounded(px(8.))
                    .hover(move |s| s.bg(p.hover))
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_end()
                    .gap(px(6.))
                    .cursor_pointer()
                    .child(
                        div()
                            .text_size(px(11.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(format_dollars_compact(tot)),
                    )
                    .child(stack),
                cx,
                move |this, w, cx| {
                    let br = ActiveBrush {
                        turn_id: tid_click.clone(),
                        anchor: gi,
                        from: gi,
                        to: gi,
                        is_bar: true,
                        ask_open: true,
                    };
                    if let Some(sel) = this.selection_for_brush(&br) {
                        cx.emit(RevenueChatEvent::RangeSelected(sel));
                    }
                    this.active_brush = Some(br);
                    if let Some(ed) = &this.brush_editor {
                        ed.update(cx, |e, cx| e.set_value("", w, cx));
                        let fh = ed.read(cx).focus.clone();
                        fh.focus(w, cx);
                    }
                    cx.notify();
                },
            )
        });

        div()
            .flex()
            .flex_col()
            .gap(px(6.))
            .child(
                div()
                    .px(px(if narrow { 12. } else { 18. }))
                    .pt(px(16.))
                    .pb(px(14.))
                    .rounded(px(14.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .flex_col()
                    .gap(px(12.))
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .justify_between()
                            .gap(px(8.))
                            .child(
                                div()
                                    .flex()
                                    .flex_wrap()
                                    .items_center()
                                    .gap(px(8.))
                                    .text_size(px(13.))
                                    .text_color(p.text)
                                    .child(title)
                                    .child(
                                        div()
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.strong)
                                            .child(format_dollars_compact(grand_total)),
                                    )
                                    .when_some(delta_chip, |d, c| d.child(c)),
                            ),
                    )
                    .when_some(ask_bar, |d, b| d.child(b))
                    .child(
                        div()
                            .h(px(164.))
                            .w_full()
                            .border_b_1()
                            .border_color(p.line)
                            .pb(px(6.))
                            .flex()
                            .items_end()
                            .gap(px(10.))
                            .children(bars),
                    )
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .gap(px(10.))
                            .children(groups.iter().map(|g| {
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .text_center()
                                    .truncate()
                                    .text_size(px(12.))
                                    .text_color(p.text)
                                    .child(g.name.clone())
                            })),
                    )
                    .when(!part_totals.is_empty(), |d| {
                        d.child(
                            div()
                                .pt(px(10.))
                                .border_t_1()
                                .border_color(p.line_soft)
                                .flex()
                                .flex_wrap()
                                .gap(px(12.))
                                .children(part_totals.into_iter().enumerate().map(
                                    |(pi, (pname, pval))| {
                                        let ratio = if grand_total > 0.0 {
                                            (pval / grand_total).clamp(0.04, 1.0) as f32
                                        } else {
                                            0.1
                                        };
                                        let pct = if grand_total > 0.0 {
                                            (pval / grand_total * 100.0).round() as i32
                                        } else {
                                            0
                                        };
                                        let c = seg_colors[pi % seg_colors.len()];
                                        div()
                                            .min_w(px(130.))
                                            .flex_1()
                                            .flex()
                                            .items_center()
                                            .gap(px(7.))
                                            .text_size(px(12.))
                                            .child(div().size(px(8.)).rounded(px(2.)).bg(c))
                                            .child(
                                                div()
                                                    .flex_1()
                                                    .min_w_0()
                                                    .truncate()
                                                    .text_color(p.text)
                                                    .child(pname),
                                            )
                                            .child(div().w(px(44.)).flex_none().child(
                                                pro_spring_bar(
                                                    pro_eid(
                                                        &self.id,
                                                        format!("rc-spart-{turn_id}-{pi}"),
                                                    ),
                                                    ratio,
                                                    4.,
                                                    2.,
                                                    p.chip,
                                                    c,
                                                ),
                                            ))
                                            .child(
                                                div()
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .text_color(p.ink)
                                                    .child(format!("{pct}%")),
                                            )
                                    },
                                )),
                        )
                    }),
            )
            .child(
                div()
                    .pl(px(2.))
                    .text_size(px(12.))
                    .text_color(p.faint)
                    .child(tr(
                        cx,
                        "Click a bar to ask about that segment",
                        "点击任一柱条即可针对该区域追问",
                    )),
            )
    }

    #[allow(clippy::too_many_arguments)]
    fn render_forecast_chart(
        &mut self,
        turn_id: &SharedString,
        caption: Option<SharedString>,
        labels: Vec<SharedString>,
        actual: Vec<Option<f64>>,
        forecast: Vec<Option<f64>>,
        target: Option<f64>,
        narrow: bool,
        cx: &mut Context<Self>,
    ) -> Div {
        let p = ProPalette::get(cx);
        let n = labels.len();
        let bounds_cell = self
            .plot_bounds
            .entry(turn_id.clone())
            .or_insert_with(|| Rc::new(Cell::new(Bounds::default())))
            .clone();

        let active_br = self
            .active_brush
            .as_ref()
            .filter(|b| &b.turn_id == turn_id && !b.is_bar)
            .cloned();

        let ask_bar = active_br
            .as_ref()
            .filter(|b| b.ask_open)
            .and_then(|br| self.selection_for_brush(br))
            .and_then(|sel| self.render_brush_ask_bar(sel, &p, cx));

        let min_v = 60.0_f32;
        let max_v = 160.0_f32;
        let span = max_v - min_v;
        let denom = (n.saturating_sub(1)).max(1) as f32;

        let act_pts: Vec<(usize, f32)> = actual
            .iter()
            .enumerate()
            .filter_map(|(i, v)| v.map(|val| (i, val as f32)))
            .collect();
        let fc_pts: Vec<(usize, f32)> = forecast
            .iter()
            .enumerate()
            .filter_map(|(i, v)| v.map(|val| (i, val as f32)))
            .collect();

        let brush_band = active_br.as_ref().map(|br| {
            (
                br.from.min(br.to) as f32 / denom,
                br.from.max(br.to) as f32 / denom,
                br.from == br.to,
            )
        });

        let blue_c = p.blue;
        let violet_c = p.purple;
        let target_c = p.amber.opacity(0.65);
        let grid_c = p.line;
        let bounds_writer = bounds_cell.clone();

        let chart_canvas = canvas(
            move |b, _, _| {
                bounds_writer.set(b);
            },
            move |bounds, _, window, _| {
                let bw = f32::from(bounds.size.width);
                let bh = f32::from(bounds.size.height);
                if bw <= 8.0 || bh <= 8.0 || n < 2 {
                    return;
                }
                let pt_at = |i: usize, v: f32| -> Point<Pixels> {
                    let tx = i as f32 / (n - 1) as f32;
                    let ty = 1.0 - ((v - min_v) / span).clamp(0.0, 1.0);
                    point(bounds.left() + px(tx * bw), bounds.top() + px(ty * bh))
                };

                for frac in [0.0_f32, 0.5, 1.0] {
                    let gy = bounds.top() + px(frac * bh);
                    let mut gp = PathBuilder::stroke(px(1.0));
                    gp.move_to(point(bounds.left(), gy));
                    gp.line_to(point(bounds.right(), gy));
                    if let Ok(path) = gp.build() {
                        window.paint_path(path, grid_c);
                    }
                }

                if let Some((lo_f, hi_f, single)) = brush_band {
                    let x0 = if single {
                        (lo_f * bw - 12.0).max(0.0)
                    } else {
                        lo_f * bw
                    };
                    let x1 = if single {
                        (hi_f * bw + 12.0).min(bw)
                    } else {
                        hi_f * bw
                    };
                    let rect = Bounds::new(
                        point(bounds.left() + px(x0), bounds.top()),
                        size(px((x1 - x0).max(2.0)), px(bh)),
                    );
                    window.paint_quad(fill(rect, violet_c.opacity(0.12)));
                }

                if let Some(tv) = target {
                    let ty = pt_at(0, tv as f32).y;
                    paint_dashed_segment(
                        window,
                        point(bounds.left(), ty),
                        point(bounds.right(), ty),
                        target_c,
                        1.1,
                        4.0,
                        4.0,
                    );
                }

                if act_pts.len() >= 2 {
                    let mut line = PathBuilder::stroke(px(1.7));
                    for (k, &(idx, val)) in act_pts.iter().enumerate() {
                        let pt = pt_at(idx, val);
                        if k == 0 {
                            line.move_to(pt);
                        } else {
                            line.line_to(pt);
                        }
                    }
                    if let Ok(path) = line.build() {
                        window.paint_path(path, blue_c);
                    }
                }

                if fc_pts.len() >= 2 {
                    let mut cone = PathBuilder::fill();
                    for (k, &(idx, val)) in fc_pts.iter().enumerate() {
                        let spread = 4.0 + k as f32 * 1.6;
                        let pt = pt_at(idx, val + spread);
                        if k == 0 {
                            cone.move_to(pt);
                        } else {
                            cone.line_to(pt);
                        }
                    }
                    for (k, &(idx, val)) in fc_pts.iter().enumerate().rev() {
                        let spread = 4.0 + k as f32 * 1.6;
                        cone.line_to(pt_at(idx, val - spread));
                    }
                    cone.close();
                    if let Ok(path) = cone.build() {
                        window.paint_path(path, violet_c.opacity(0.14));
                    }

                    for pair in fc_pts.windows(2) {
                        paint_dashed_segment(
                            window,
                            pt_at(pair[0].0, pair[0].1),
                            pt_at(pair[1].0, pair[1].1),
                            violet_c,
                            1.6,
                            3.5,
                            3.0,
                        );
                    }
                }
            },
        )
        .size_full();

        let tid_down = turn_id.clone();
        let tid_move = turn_id.clone();
        let plot_box = div()
            .id(pro_eid(&self.id, format!("fc-plot-{turn_id}")))
            .relative()
            .w_full()
            .h(px(152.))
            .cursor_crosshair()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, ev: &MouseDownEvent, _, cx| {
                    if let Some(pi) = this.point_index_from_x(&tid_down, n, ev.position.x) {
                        this.dragging_brush = true;
                        this.active_brush = Some(ActiveBrush {
                            turn_id: tid_down.clone(),
                            anchor: pi,
                            from: pi,
                            to: pi,
                            is_bar: false,
                            ask_open: false,
                        });
                        cx.notify();
                    }
                }),
            )
            .on_mouse_move(cx.listener(move |this, ev: &MouseMoveEvent, _, cx| {
                if this.dragging_brush
                    && ev.pressed_button == Some(MouseButton::Left)
                    && let Some(pi) = this.point_index_from_x(&tid_move, n, ev.position.x)
                    && let Some(br) = &mut this.active_brush
                    && br.turn_id == tid_move
                {
                    br.from = br.anchor.min(pi);
                    br.to = br.anchor.max(pi);
                    cx.notify();
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _: &MouseUpEvent, w, cx| {
                    this.finish_brush(w, cx);
                }),
            )
            .child(chart_canvas);

        let first_lbl = labels.first().cloned().unwrap_or_default();
        let mid_lbl = labels.get(n / 2).cloned().unwrap_or_default();
        let last_lbl = labels.last().cloned().unwrap_or_default();

        div()
            .px(px(if narrow { 10. } else { 14. }))
            .pt(px(14.))
            .pb(px(10.))
            .rounded(px(14.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .flex()
            .flex_col()
            .gap(px(10.))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .child(
                        div()
                            .text_size(px(12.5))
                            .text_color(p.muted)
                            .child(caption.unwrap_or_default()),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(12.))
                            .text_size(px(12.))
                            .text_color(p.muted)
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .child(div().w(px(10.)).h(px(3.)).rounded(px(1.5)).bg(blue_c))
                                    .child(tr(cx, "Actual", "实际")),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .child(div().w(px(10.)).h(px(3.)).rounded(px(1.5)).bg(violet_c))
                                    .child(tr(cx, "Forecast (p80)", "预测区间")),
                            ),
                    ),
            )
            .when_some(ask_bar, |d, b| d.child(b))
            .child(
                div()
                    .w_full()
                    .flex()
                    .gap(px(10.))
                    .child(
                        div()
                            .h(px(152.))
                            .flex_none()
                            .flex()
                            .flex_col()
                            .justify_between()
                            .items_end()
                            .text_size(px(11.))
                            .text_color(p.muted)
                            .child("$160K")
                            .child("$110K")
                            .child("$60K"),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap(px(8.))
                            .child(plot_box)
                            .child(
                                div()
                                    .w_full()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .text_size(px(11.))
                                    .text_color(p.muted)
                                    .child(first_lbl)
                                    .child(mid_lbl)
                                    .child(last_lbl),
                            ),
                    ),
            )
    }

    fn render_turn(
        &mut self,
        turn: RevenueChatTurn,
        narrow: bool,
        elapsed_ms: f32,
        reduced: bool,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let p = ProPalette::get(cx);
        match turn {
            RevenueChatTurn::User { id, text, quote } => div()
                .id(pro_eid(&self.id, format!("user-turn-{id}")))
                .w_full()
                .flex()
                .flex_col()
                .items_end()
                .gap(px(4.))
                .when_some(quote, |d, q| {
                    d.child(
                        div()
                            .max_w(relative(0.84))
                            .px(px(10.))
                            .py(px(3.))
                            .rounded(px(8.))
                            .bg(p.accent_soft)
                            .border_l_2()
                            .border_color(p.blue)
                            .text_size(px(12.))
                            .text_color(p.text)
                            .child(q),
                    )
                })
                .child(
                    div()
                        .max_w(relative(0.82))
                        .px(px(14.))
                        .py(px(9.))
                        .rounded(px(16.))
                        .bg(p.chip)
                        .text_size(px(14.))
                        .line_height(px(21.))
                        .text_color(p.ink)
                        .child(text),
                )
                .into_any_element(),

            RevenueChatTurn::Assistant {
                id,
                text,
                kpis,
                chart,
                caption,
                sources,
                next,
                liked,
                streaming,
            } => {
                let mark_color = if p.dark {
                    rgba(0x4d8dffda).into()
                } else {
                    rgba(0x0060e2c7).into()
                };

                if streaming && text.is_empty() {
                    return div()
                        .w_full()
                        .flex()
                        .items_center()
                        .gap(px(12.))
                        .child(analyst_pixel_mark(mark_color, true, elapsed_ms, reduced))
                        .child(pro_shimmer_text(
                            pro_eid(&self.id, format!("shim-{id}")),
                            tr(
                                cx,
                                "Reading Stripe and HubSpot…",
                                "正在读取 Stripe 与 HubSpot 数据…",
                            ),
                            p.muted,
                            p.ink,
                            13.,
                            FontWeight::NORMAL,
                            true,
                        ))
                        .into_any_element();
                }

                let mut body = div().w_full().flex().flex_col().gap(px(14.)).min_w_0();

                if !streaming {
                    if !kpis.is_empty() {
                        body = body.child(Self::render_kpi_strip(&kpis, narrow, &p));
                    }
                    if let Some(ch) = chart {
                        let fig = match ch {
                            RevenueChartKind::Trend {
                                labels,
                                current,
                                previous,
                                mark,
                            } => self.render_trend_chart(
                                &id, caption, labels, current, previous, mark, narrow, reduced, w,
                                cx,
                            ),
                            RevenueChartKind::Split {
                                title,
                                previous,
                                since,
                                groups,
                            } => self.render_split_chart(
                                &id, title, previous, since, groups, narrow, w, cx,
                            ),
                            RevenueChartKind::Forecast {
                                labels,
                                actual,
                                forecast,
                                target,
                            } => self.render_forecast_chart(
                                &id, caption, labels, actual, forecast, target, narrow, cx,
                            ),
                        };
                        body = body.child(fig);
                    }

                    let is_copied = self
                        .copied_turn
                        .as_ref()
                        .is_some_and(|(cid, t)| cid == &id && t.elapsed().as_secs_f32() < 1.8);
                    let copy_txt = text.clone();
                    let tid_copy = id.clone();
                    let tid_dl = id.clone();
                    let tid_like = id.clone();
                    let tid_retry = id.clone();
                    let retry_txt = text.clone();

                    let foot = div()
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .justify_between()
                        .gap(px(8.))
                        .child(
                            div()
                                .flex()
                                .flex_wrap()
                                .items_center()
                                .gap(px(6.))
                                .text_size(px(12.))
                                .text_color(p.muted)
                                .when(!sources.is_empty(), |d| {
                                    d.child(tr(cx, "From", "数据来自")).children(
                                        sources.into_iter().map(|src| {
                                            div()
                                                .h(px(20.))
                                                .px(px(7.))
                                                .rounded(px(6.))
                                                .bg(p.chip)
                                                .flex()
                                                .items_center()
                                                .text_size(px(12.))
                                                .text_color(p.text)
                                                .child(src)
                                        }),
                                    )
                                }),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(2.))
                                .child(pro_icon_btn(
                                    pro_eid(&self.id, format!("rc-copy-{id}")),
                                    tr(cx, "Copy answer", "复制回答"),
                                    if is_copied {
                                        ProIcon::Check
                                    } else {
                                        ProIcon::Copy
                                    },
                                    28.,
                                    is_copied,
                                    cx,
                                    move |this, _, cx| {
                                        cx.write_to_clipboard(ClipboardItem::new_string(
                                            copy_txt.to_string(),
                                        ));
                                        let now = Instant::now();
                                        this.copied_turn = Some((tid_copy.clone(), now));
                                        this.toast = Some((
                                            tr(cx, "Copied answer", "已复制分析回答").into(),
                                            now,
                                        ));
                                        cx.emit(RevenueChatEvent::CopiedAnswer(tid_copy.clone()));
                                        cx.notify();
                                    },
                                ))
                                .child(pro_icon_btn(
                                    pro_eid(&self.id, format!("rc-dl-{id}")),
                                    tr(cx, "Download chart", "导出图表数据"),
                                    ProIcon::Download,
                                    28.,
                                    false,
                                    cx,
                                    move |this, _, cx| {
                                        cx.write_to_clipboard(ClipboardItem::new_string(
                                            "week,this_year_k,last_year_k\nW33,93,81\nW34,97,80\nW35,134,84\nW36,104,86\nW39,109,90".to_owned(),
                                        ));
                                        this.toast = Some((
                                            tr(cx, "Copied chart CSV", "已复制图表 CSV").into(),
                                            Instant::now(),
                                        ));
                                        cx.emit(RevenueChatEvent::DownloadedChart(tid_dl.clone()));
                                        cx.notify();
                                    },
                                ))
                                .child(activate(
                                    div()
                                        .id(pro_eid(&self.id, format!("rc-like-{id}")))
                                        .role(Role::Button)
                                        .aria_label(tr(cx, "Good answer", "有帮助"))
                                        .aria_toggled(liked.into())
                                        .tab_index(0)
                                        .size(px(28.))
                                        .rounded(px(7.))
                                        .bg(if liked {
                                            p.accent_soft
                                        } else {
                                            transparent_black()
                                        })
                                        .hover(move |s| s.bg(p.hover))
                                        .focus_visible(move |s| s.border_1().border_color(p.focus))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .cursor_pointer()
                                        .child(pro_svg(
                                            "0 0 24 24",
                                            SVG_THUMBS_UP,
                                            14.,
                                            if liked { p.blue } else { p.muted },
                                        )),
                                    cx,
                                    move |this, _, cx| {
                                        for t in &mut this.turns {
                                            if let RevenueChatTurn::Assistant { id, liked, .. } = t
                                                && id == &tid_like
                                            {
                                                *liked = !*liked;
                                                let next_liked = *liked;
                                                cx.emit(RevenueChatEvent::LikedTurn {
                                                    turn_id: tid_like.clone(),
                                                    liked: next_liked,
                                                });
                                                break;
                                            }
                                        }
                                        cx.notify();
                                    },
                                ))
                                .child(pro_icon_btn(
                                    pro_eid(&self.id, format!("rc-retry-{id}")),
                                    tr(cx, "Try again", "重新生成"),
                                    ProIcon::Refresh,
                                    28.,
                                    false,
                                    cx,
                                    move |this, _, cx| {
                                        if !Theme::global(cx).reduced_motion {
                                            for t in &mut this.turns {
                                                if let RevenueChatTurn::Assistant {
                                                    id,
                                                    text,
                                                    streaming,
                                                    ..
                                                } = t
                                                    && id == &tid_retry
                                                {
                                                    *text = SharedString::default();
                                                    *streaming = true;
                                                }
                                            }
                                            this.stream_state = Some((
                                                tid_retry.clone(),
                                                retry_txt.clone(),
                                                Instant::now(),
                                            ));
                                            cx.notify();
                                        }
                                    },
                                )),
                        );
                    body = body.child(foot);

                    if !next.is_empty() {
                        let chips = div().flex().flex_wrap().gap(px(6.)).children(
                            next.into_iter().enumerate().map(|(ni, prompt)| {
                                let p_str = prompt.to_string();
                                activate(
                                    div()
                                        .id(pro_eid(&self.id, format!("rc-next-{id}-{ni}")))
                                        .role(Role::Button)
                                        .tab_index(0)
                                        .min_h(px(30.))
                                        .pl(px(12.))
                                        .pr(px(10.))
                                        .py(px(5.))
                                        .rounded(px(9.))
                                        .bg(p.card)
                                        .border_1()
                                        .border_color(p.line)
                                        .hover(move |s| {
                                            s.border_color(p.line_strong).text_color(p.ink)
                                        })
                                        .focus_visible(move |s| s.border_1().border_color(p.focus))
                                        .flex()
                                        .items_center()
                                        .gap(px(6.))
                                        .text_size(px(13.))
                                        .text_color(p.text)
                                        .cursor_pointer()
                                        .child(prompt)
                                        .child(pro_icon(ProIcon::ExternalArrow, 12.5, p.faint)),
                                    cx,
                                    move |this, _, cx| {
                                        this.ask(p_str.clone(), None, cx);
                                    },
                                )
                            }),
                        );
                        body = body.child(chips);
                    }
                }

                div()
                    .w_full()
                    .flex()
                    .items_start()
                    .gap(px(if narrow { 0. } else { 12. }))
                    .when(!narrow, |d| {
                        d.child(div().mt(px(3.)).child(analyst_pixel_mark(
                            mark_color, streaming, elapsed_ms, reduced,
                        )))
                    })
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap(px(14.))
                            .child(
                                div()
                                    .text_size(px(14.))
                                    .line_height(px(22.))
                                    .text_color(p.ink)
                                    .child(if streaming {
                                        format!("{text}▍")
                                    } else {
                                        text.to_string()
                                    }),
                            )
                            .when(!streaming, |d| d.child(body)),
                    )
                    .into_any_element()
            }
        }
    }

    fn render_composer(&self, narrow: bool, w: &mut Window, cx: &mut Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let has_text = self
            .composer_editor
            .as_ref()
            .is_some_and(|ed| !ed.read(cx).value().trim().is_empty());
        let send_t = pro_spring_f32(
            pro_eid(&self.id, "rc-send-t"),
            f32::from(has_text),
            0.18,
            w,
            cx,
        );
        let send_bg = pro_mix_color(p.chip, p.primary, send_t);
        let send_fg = pro_mix_color(p.faint, p.on_primary, send_t);
        let model_name = COMPOSER_MODELS[self.model_index % COMPOSER_MODELS.len()];
        let policy_lbl = self.approval_policy.label(cx);
        let ed_opt = self.composer_editor.clone();

        div()
            .w_full()
            .max_w(px(760.))
            .mx_auto()
            .px(px(if narrow { 12. } else { 24. }))
            .pb(px(18.))
            .child(
                div()
                    .px(px(12.))
                    .pt(px(12.))
                    .pb(px(10.))
                    .rounded(px(16.))
                    .bg(p.card)
                    .border_1()
                    .border_color(if self.editor_focused {
                        p.line_strong
                    } else {
                        p.line
                    })
                    .when(!p.dark, |d| d.shadow_xs())
                    .flex()
                    .flex_col()
                    .gap(px(12.))
                    .when(self.attached_context, |d| {
                        d.child(
                            div().flex().items_center().gap(px(6.)).child(
                                div()
                                    .h(px(22.))
                                    .px(px(8.))
                                    .rounded(px(6.))
                                    .bg(p.chip)
                                    .flex()
                                    .items_center()
                                    .gap(px(5.))
                                    .text_size(px(12.))
                                    .text_color(p.text)
                                    .child(pro_icon(ProIcon::Database, 12., p.blue))
                                    .child("stripe.q3_settled_mrr.csv"),
                            ),
                        )
                    })
                    .child(
                        div()
                            .px(px(2.))
                            .h(px(26.))
                            .when_some(ed_opt.clone(), |d, ed| d.child(ed)),
                    )
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
                                    .gap(px(4.))
                                    .child(pro_icon_btn(
                                        pro_eid(&self.id, "rc-attach"),
                                        tr(cx, "Add data context", "添加数据上下文"),
                                        ProIcon::Plus,
                                        28.,
                                        self.attached_context,
                                        cx,
                                        |this, _, cx| {
                                            this.attached_context = !this.attached_context;
                                            cx.notify();
                                        },
                                    ))
                                    .child(activate(
                                        div()
                                            .id(pro_eid(&self.id, "rc-approval-btn"))
                                            .role(Role::Button)
                                            .aria_label(policy_lbl)
                                            .tab_index(0)
                                            .h(px(28.))
                                            .px(px(8.))
                                            .rounded(px(8.))
                                            .hover(move |s| s.bg(p.hover).text_color(p.ink))
                                            .focus_visible(move |s| {
                                                s.border_1().border_color(p.focus)
                                            })
                                            .flex()
                                            .items_center()
                                            .gap(px(6.))
                                            .text_size(px(12.5))
                                            .text_color(p.muted)
                                            .cursor_pointer()
                                            .child(pro_svg(
                                                "0 0 68.155 68.155",
                                                SVG_HAND_APPROVAL,
                                                14.,
                                                p.muted,
                                            ))
                                            .when(!narrow, |d| d.child(policy_lbl)),
                                        cx,
                                        |this, _, cx| {
                                            this.approval_policy = this.approval_policy.next();
                                            cx.emit(RevenueChatEvent::ApprovalPolicyChanged(
                                                this.approval_policy,
                                            ));
                                            cx.notify();
                                        },
                                    )),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(4.))
                                    .child(activate(
                                        div()
                                            .id(pro_eid(&self.id, "rc-model-btn"))
                                            .role(Role::Button)
                                            .aria_label(model_name)
                                            .tab_index(0)
                                            .h(px(28.))
                                            .px(px(8.))
                                            .rounded(px(8.))
                                            .hover(move |s| s.bg(p.hover))
                                            .focus_visible(move |s| {
                                                s.border_1().border_color(p.focus)
                                            })
                                            .flex()
                                            .items_center()
                                            .gap(px(4.))
                                            .text_size(px(12.5))
                                            .text_color(p.ink)
                                            .cursor_pointer()
                                            .child(model_name)
                                            .child(pro_icon(ProIcon::ChevronDown, 12., p.muted)),
                                        cx,
                                        |this, _, cx| {
                                            this.model_index =
                                                (this.model_index + 1) % COMPOSER_MODELS.len();
                                            cx.notify();
                                        },
                                    ))
                                    .child(pro_icon_btn(
                                        pro_eid(&self.id, "rc-mic"),
                                        tr(cx, "Voice dictation", "语音输入"),
                                        ProIcon::Mic,
                                        28.,
                                        self.recording,
                                        cx,
                                        |this, _, cx| {
                                            this.recording = !this.recording;
                                            cx.notify();
                                        },
                                    ))
                                    .child(activate(
                                        div()
                                            .id(pro_eid(&self.id, "rc-send"))
                                            .role(Role::Button)
                                            .aria_label(tr(cx, "Send question", "发送问题"))
                                            .tab_index(0)
                                            .size(px(28.))
                                            .rounded(px(8.))
                                            .bg(send_bg)
                                            .focus_visible(move |s| {
                                                s.border_1().border_color(p.focus)
                                            })
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .cursor_pointer()
                                            .child(pro_icon(ProIcon::ArrowUp, 14., send_fg)),
                                        cx,
                                        move |this, w, cx| {
                                            if let Some(ed) = &ed_opt {
                                                let val = ed.read(cx).value().to_string();
                                                if this.ask(val, None, cx) {
                                                    ed.update(cx, |e, cx| e.set_value("", w, cx));
                                                }
                                            }
                                        },
                                    )),
                            ),
                    ),
            )
    }
}

impl Render for RevenueChatBlock {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.ensure_editors(w, cx);
        self.advance_animations(w, cx);

        let p = ProPalette::get(cx);
        let reduced = Theme::global(cx).reduced_motion;
        let elapsed_ms = self.boot_at.elapsed().as_secs_f32() * 1000.0;
        let width = self.measured_width.get();
        let narrow = width < 560.0;
        let measure_cell = self.measured_width.clone();

        let thread_content = if self.loading {
            let phase = (elapsed_ms / 1400.0).fract();
            div()
                .w_full()
                .max_w(px(760.))
                .mx_auto()
                .px(px(if narrow { 12. } else { 24. }))
                .py(px(28.))
                .flex()
                .flex_col()
                .gap(px(16.))
                .child(
                    div()
                        .self_end()
                        .child(pro_skeleton_bar(220., 36., 16., phase, cx)),
                )
                .child(pro_skeleton_bar(340., 16., 5., phase + 0.1, cx))
                .child(
                    div()
                        .flex()
                        .gap(px(10.))
                        .child(pro_skeleton_bar(160., 68., 12., phase + 0.15, cx))
                        .child(pro_skeleton_bar(160., 68., 12., phase + 0.2, cx))
                        .child(pro_skeleton_bar(160., 68., 12., phase + 0.25, cx)),
                )
                .child(pro_skeleton_bar(480., 180., 14., phase + 0.3, cx))
                .into_any_element()
        } else if self.empty || self.turns.is_empty() {
            pro_empty_state(
                ProIcon::UsageChart,
                tr(cx, "Ask about revenue, MRR, or cohorts", "开始探索营收、MRR 与留存趋势"),
                tr(
                    cx,
                    "Answers combine Stripe and HubSpot numbers into KPI cards and brushable charts.",
                    "回答将自动汇总 Stripe 与 HubSpot 指标并生成可框选追问的交互图表。",
                ),
                Some((
                    pro_eid(&self.id, "rc-starter-q"),
                    tr(cx, "How did revenue develop last quarter?", "上季度营收表现如何？").into(),
                    Box::new(|this: &mut Self, _, cx| {
                        this.turns = default_revenue_turns();
                        this.empty = false;
                        cx.notify();
                    }),
                )),
                cx,
            )
            .into_any_element()
        } else {
            let snapshot = self.turns.clone();
            let mut turn_els = Vec::with_capacity(snapshot.len());
            for turn in snapshot {
                turn_els.push(self.render_turn(turn, narrow, elapsed_ms, reduced, w, cx));
            }
            div()
                .w_full()
                .max_w(px(760.))
                .mx_auto()
                .px(px(if narrow { 12. } else { 24. }))
                .pt(px(if narrow { 20. } else { 28. }))
                .pb(px(20.))
                .flex()
                .flex_col()
                .gap(px(26.))
                .children(turn_els)
                .into_any_element()
        };

        let mode_key: SharedString = if self.loading {
            "loading".into()
        } else if self.empty || self.turns.is_empty() {
            "empty".into()
        } else {
            "ready".into()
        };

        let composer = self.render_composer(narrow, w, cx);

        div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::handle_key_down))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _: &MouseUpEvent, w, cx| {
                    this.finish_brush(w, cx);
                }),
            )
            .relative()
            .size_full()
            .min_h(px(560.))
            .bg(p.page)
            .flex()
            .flex_col()
            .child(
                canvas(
                    move |b, window, _| {
                        let nw = f32::from(b.size.width);
                        if nw > 10. && (measure_cell.get() - nw).abs() > 1.0 {
                            measure_cell.set(nw);
                            window.request_animation_frame();
                        }
                    },
                    |_, _, _, _| (),
                )
                .absolute()
                .inset_0(),
            )
            .child(
                div()
                    .id(pro_eid(&self.id, "rc-scroll"))
                    .track_scroll(&self.scroll)
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(pro_view_transition(
                        pro_eid(&self.id, "rc-mode-vtx"),
                        mode_key,
                        thread_content,
                    )),
            )
            .child(composer)
            .child(pro_toast_banner(self.toast.clone()))
    }
}

fn analyst_pixel_mark(color: Hsla, busy: bool, elapsed_ms: f32, reduced: bool) -> Div {
    let cells = agent_pixel_cells("RevenueAnalyst");
    let mut grid = div()
        .size(px(16.))
        .flex_none()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(1.5));
    for r in 0..3usize {
        let mut row = div().flex().items_center().gap(px(1.5));
        for c in 0..3usize {
            let i = r * 3 + c;
            let on = cells[i];
            let alpha = if busy && !reduced {
                let phase = ((elapsed_ms + AGENT_WAVE_MS[i]) / 1200.0) * std::f32::consts::TAU;
                let wave = 0.58 + 0.42 * phase.cos();
                if on { wave } else { 0.16 * wave }
            } else if on {
                1.0
            } else {
                0.0
            };
            row = row.child(
                div()
                    .size(px(3.))
                    .rounded(px(0.75))
                    .bg(color.opacity(alpha)),
            );
        }
        grid = grid.child(row);
    }
    grid
}

fn paint_ringed_dot(
    window: &mut Window,
    center: Point<Pixels>,
    inner_r: f32,
    ring_w: f32,
    fill_c: Hsla,
    ring_c: Hsla,
) {
    let outer = px(inner_r + ring_w);
    let inner = px(inner_r);
    window.paint_quad(
        fill(
            Bounds::new(center - point(outer, outer), size(outer * 2.0, outer * 2.0)),
            ring_c,
        )
        .corner_radii(outer),
    );
    window.paint_quad(
        fill(
            Bounds::new(center - point(inner, inner), size(inner * 2.0, inner * 2.0)),
            fill_c,
        )
        .corner_radii(inner),
    );
}

fn paint_dashed_segment(
    window: &mut Window,
    p0: Point<Pixels>,
    p1: Point<Pixels>,
    color: Hsla,
    stroke_w: f32,
    dash: f32,
    gap: f32,
) {
    let dx = f32::from(p1.x - p0.x);
    let dy = f32::from(p1.y - p0.y);
    let dist = dx.hypot(dy);
    if dist <= 0.5 {
        return;
    }
    let step = dash + gap;
    let mut d = 0.0_f32;
    let mut pb = PathBuilder::stroke(px(stroke_w));
    while d < dist {
        let t0 = d / dist;
        let t1 = ((d + dash) / dist).min(1.0);
        pb.move_to(point(p0.x + px(dx * t0), p0.y + px(dy * t0)));
        pb.line_to(point(p0.x + px(dx * t1), p0.y + px(dy * t1)));
        d += step;
    }
    if let Ok(path) = pb.build() {
        window.paint_path(path, color);
    }
}

fn paint_dashed_polyline(
    window: &mut Window,
    values: &[f32],
    pt_of: impl Fn(usize, f32) -> Point<Pixels>,
    color: Hsla,
    stroke_w: f32,
    dash: f32,
    gap: f32,
) {
    if values.len() < 2 {
        return;
    }
    for (i, pair) in values.windows(2).enumerate() {
        paint_dashed_segment(
            window,
            pt_of(i, pair[0]),
            pt_of(i + 1, pair[1]),
            color,
            stroke_w,
            dash,
            gap,
        );
    }
}

fn format_k_dollars(val_k: f64) -> String {
    if val_k >= 1000.0 {
        format!("${:.2}M", val_k / 1000.0)
    } else {
        format!("${:.0}K", val_k)
    }
}

fn format_dollars_compact(dollars: f64) -> String {
    if dollars >= 1_000_000.0 {
        format!("${:.2}M", dollars / 1_000_000.0)
    } else if dollars >= 10_000.0 {
        format!("${:.0}K", dollars / 1_000.0)
    } else if dollars >= 1_000.0 {
        format!("${:.1}K", dollars / 1_000.0)
    } else {
        format!("${:.0}", dollars)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        RevenueChartKind, RevenueChartSelection, RevenueChatBlock, RevenueChatTurn,
        compute_trend_selection, default_revenue_turns, synthesize_revenue_reply,
    };

    #[test]
    fn initial_q3_trend_and_brushed_range_summary_are_exact() {
        let turns = default_revenue_turns();
        assert_eq!(turns.len(), 2);
        let RevenueChatTurn::Assistant {
            chart:
                Some(RevenueChartKind::Trend {
                    labels,
                    current,
                    previous,
                    mark,
                }),
            ..
        } = &turns[1]
        else {
            panic!("expected initial trend chart");
        };

        assert_eq!(labels.len(), 13);
        assert_eq!(*mark, Some(8));
        assert_eq!(labels[8].as_ref(), "W35");

        let (lbl, detail) = compute_trend_selection(labels, current, previous, 5, 7);
        assert_eq!(lbl.as_ref(), "W32 – W34");
        assert_eq!(detail.as_ref(), "$284K · +18.3% on last year");
    }

    #[test]
    fn synthesizer_produces_region_split_forecast_and_brushed_answers() {
        let reg = synthesize_revenue_reply("a-2".into(), "Split it by region", None);
        assert!(matches!(
            reg,
            RevenueChatTurn::Assistant {
                chart: Some(RevenueChartKind::Split { .. }),
                ..
            }
        ));

        let fc = synthesize_revenue_reply("a-3".into(), "Forecast Q4", None);
        assert!(matches!(
            fc,
            RevenueChatTurn::Assistant {
                chart: Some(RevenueChartKind::Forecast { .. }),
                ..
            }
        ));

        let sel = RevenueChartSelection {
            turn_id: "a-1".into(),
            from: 5,
            to: 7,
            label: "W32 – W34".into(),
            detail: "$284K · +18.3% on last year".into(),
        };
        let focused = synthesize_revenue_reply("a-4".into(), "What drove these weeks?", Some(&sel));
        if let RevenueChatTurn::Assistant { text, kpis, .. } = focused {
            assert!(text.contains("W32 – W34"));
            assert_eq!(kpis[0].value.as_ref(), "W32 – W34");
        } else {
            panic!("expected assistant turn");
        }
    }

    #[cfg(feature = "test-support")]
    #[test]
    fn revenue_chat_block_appends_turns_on_ask() {
        let mut app = gpui::TestAppContext::single();
        app.update(crate::init);
        let (chat, w) = app.add_window_view(|_, cx| RevenueChatBlock::new("test-rev-chat", cx));
        w.update(|_, cx| {
            chat.update(cx, |c, cx| {
                assert_eq!(c.turns().len(), 2);
                assert!(c.ask("Split it by region", None, cx));
                assert_eq!(c.turns().len(), 4);
            });
        });
    }
}
