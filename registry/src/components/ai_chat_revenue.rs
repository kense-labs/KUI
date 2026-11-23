//! Conversational revenue analyst screen with KPI cards, interactive charts, and source sheets.
//!
//! Renders [`AiChatRevenueScreen`] inside [`AppShell`] (`AppShellPreset::Chat`), pairing a
//! centered analyst thread (KPI cards, weekly YoY area/line charts, regional stacked-bar
//! splits, Q4 forecast curves, brush-to-ask chart focus popovers, source pills, and follow-up
//! suggestions) with a multi-mode composer and an optional right-hand revenue sheet inspector.

use crate::{
    components::{
        app_shell::{AppShell, AppShellEvent, AppShellPreset, ShellNavGroup, ShellNavItem},
        pro_chrome::{
            ProBtnStyle, ProPalette, pro_app_tile, pro_btn, pro_ease, pro_eid, pro_empty_state,
            pro_fold, pro_icon_btn, pro_mark_selected, pro_mix_color, pro_pill_indicator,
            pro_presence, pro_selection_sensor, pro_skeleton_bar, pro_spring_bar, pro_spring_f32,
            pro_toast_banner, pro_view_transition, tr,
        },
    },
    foundation::Theme,
    primitives::{
        control::activate,
        editor::{EditEvent, Editor, InputMode},
        pro_icons::{
            AGENT_WAVE_MS, ProIcon, agent_pixel_cells, pro_app_artwork, pro_icon, pro_svg,
        },
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

const HAND_APPROVAL_SVG: &str = r#"<path d="M52.5305 32.6557V24.8463C52.5305 22.1016 54.7555 19.8766 57.5001 19.8766C60.2448 19.8766 62.4698 22.1016 62.4698 24.8463V42.027C62.4698 53.3194 53.3154 62.4735 42.0233 62.4735H34.2388M22.7127 39.7552L15.94 31.8537C14.8314 30.5602 13.2129 29.8159 11.5094 29.8159H11.1235C8.11379 29.8159 5.67395 32.2558 5.67395 35.2655C5.67395 36.3412 5.99241 37.393 6.58919 38.2881L16.4118 53.0221C20.3482 58.9269 26.9753 62.4735 34.0719 62.4735" stroke="currentColor" stroke-width="4.7" stroke-linecap="round" stroke-linejoin="round"></path><path d="M52.534 24.1378V16.3284C52.534 13.5837 50.3091 11.3588 47.5644 11.3588C44.8197 11.3588 42.5948 13.5837 42.5948 16.3284V29.8174" stroke="currentColor" stroke-width="4.7" stroke-linecap="round" stroke-linejoin="round"></path><path d="M42.5958 29.8198V10.6513C42.5958 7.90661 40.3708 5.68163 37.6262 5.68163C34.8815 5.68163 32.6565 7.90661 32.6565 10.6513V28.4" stroke="currentColor" stroke-width="4.7" stroke-linecap="round" stroke-linejoin="round"></path><path d="M32.6576 28.3975V16.3284C32.6576 13.5837 30.4326 11.3588 27.6879 11.3588C24.9433 11.3588 22.7183 13.5837 22.7183 16.3284V39.7567" stroke="currentColor" stroke-width="4.7" stroke-linecap="round" stroke-linejoin="round"></path>"#;
const COPY_OVERLAP_SVG: &str = r#"<path d="M7.5 14.5C7.5 11.2002 7.5 9.55025 8.52513 8.52513C9.55025 7.5 11.2002 7.5 14.5 7.5C17.7998 7.5 19.4497 7.5 20.4749 8.52513C21.5 9.55025 21.5 11.2002 21.5 14.5C21.5 17.7998 21.5 19.4497 20.4749 20.4749C19.4497 21.5 17.7998 21.5 14.5 21.5C11.2002 21.5 9.55025 21.5 8.52513 20.4749C7.5 19.4497 7.5 17.7998 7.5 14.5Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M7.5 16.5C6.10355 16.5 5.40533 16.5 4.84402 16.3036C3.83866 15.9518 3.0482 15.1613 2.69641 14.156C2.5 13.5947 2.5 12.8964 2.5 11.5V9.5C2.5 6.20017 2.5 4.55025 3.52513 3.52513C4.55025 2.5 6.20017 2.5 9.5 2.5H11.5C12.8964 2.5 13.5947 2.5 14.156 2.69641C15.1613 3.0482 15.9518 3.83866 16.3036 4.84402C16.5 5.40533 16.5 6.10355 16.5 7.5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;
const DOWNLOAD_ARROW_SVG: &str = r#"<path d="M16.9504 12.1817C17.1981 12.814 16.5076 13.5726 15.1267 15.0899C13.6702 16.6902 12.9201 17.4904 12 17.5C11.0799 17.4904 10.3298 16.6902 8.87331 15.0899C7.49239 13.5726 6.80193 12.814 7.04964 12.1817C7.05868 12.1586 7.06851 12.1359 7.0791 12.1135C7.34928 11.542 8.24477 11.5029 10 11.5002V4.99998C10 4.53501 10 4.30253 10.0511 4.11179C10.1898 3.59414 10.5941 3.1898 11.1118 3.05111C11.3025 3 11.535 3 12 3C12.4649 3 12.6974 3 12.8882 3.05111C13.4058 3.1898 13.8102 3.59414 13.9489 4.11179C14 4.30253 14 4.53501 14 4.99998V11.5002C15.7552 11.5029 16.6507 11.542 16.9209 12.1135C16.9315 12.1359 16.9413 12.1586 16.9504 12.1817Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M5.00006 21H19.0001" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;
const THUMBS_UP_SVG: &str = r#"<path d="M2 12.5C2 11.3954 2.89543 10.5 4 10.5C5.65685 10.5 7 11.8431 7 13.5V17.5C7 19.1569 5.65685 20.5 4 20.5C2.89543 20.5 2 19.6046 2 18.5V12.5Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M15.4787 7.80626L15.2124 8.66634C14.9942 9.37111 14.8851 9.72349 14.969 10.0018C15.0369 10.2269 15.1859 10.421 15.389 10.5487C15.64 10.7065 16.0197 10.7065 16.7791 10.7065H17.1831C19.7532 10.7065 21.0382 10.7065 21.6452 11.4673C21.7145 11.5542 21.7762 11.6467 21.8296 11.7437C22.2965 12.5921 21.7657 13.7351 20.704 16.0211C19.7297 18.1189 19.2425 19.1678 18.338 19.7852C18.2505 19.8449 18.1605 19.9013 18.0683 19.9541C17.116 20.5 15.9362 20.5 13.5764 20.5H13.0646C10.2057 20.5 8.77628 20.5 7.88814 19.6395C7 18.7789 7 17.3939 7 14.6239V13.6503C7 12.1946 7 11.4668 7.25834 10.8006C7.51668 10.1344 8.01135 9.58664 9.00069 8.49112L13.0921 3.96056C13.1947 3.84694 13.246 3.79012 13.2913 3.75075C13.7135 3.38328 14.3652 3.42464 14.7344 3.84235C14.774 3.8871 14.8172 3.94991 14.9036 4.07554C15.0388 4.27205 15.1064 4.37031 15.1654 4.46765C15.6928 5.33913 15.8524 6.37436 15.6108 7.35715C15.5838 7.46692 15.5488 7.5801 15.4787 7.80626Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;
const CHECK_LINE_SVG: &str = r#"<path d="M5 14L8.5 17.5L19 6.5" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"></path>"#;

/// Action approval policy selectable from the bottom composer toolbar.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RevenueApprovalMode {
    /// Prompt before external writes or exports.
    #[default]
    RequestApproval,
    /// Run read-only queries automatically.
    AutoReads,
    /// Execute without confirmation prompts.
    FullAutonomy,
}

impl RevenueApprovalMode {
    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::RequestApproval => tr(cx, "Request approval", "需审批"),
            Self::AutoReads => tr(cx, "Auto-approve reads", "只读自动批准"),
            Self::FullAutonomy => tr(cx, "Full autonomy", "全自动执行"),
        }
    }
}

/// Time granularity toggle for revenue trend charts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RevenueChartPeriod {
    /// Weekly buckets (`W27`–`W39`).
    #[default]
    Weekly,
    /// Aggregated monthly buckets (`Jul`, `Aug`, `Sep`).
    Monthly,
}

/// Grouping dimension for stacked-bar revenue split charts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RevenueSplitGrouping {
    /// Stacked revenue columns grouped by geographic region.
    #[default]
    ByRegion,
    /// Stacked revenue columns grouped by subscription tier.
    ByPlan,
}

/// Artifact tab shown in the optional right-hand revenue sheet inspector.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RevenueArtifactKind {
    /// Live Stripe net revenue and MRR movements.
    #[default]
    StripeLedger,
    /// Closed-won HubSpot annual contracts.
    HubSpotDeals,
    /// Exportable weekly Q3 CSV table.
    WeeklyCsv,
}

/// Summary KPI card rendered above an analyst chart.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RevenueKpi {
    pub label: SharedString,
    pub value: SharedString,
    pub change: Option<SharedString>,
    pub good: bool,
}

impl RevenueKpi {
    /// Create a KPI card with label, primary value, change caption, and positive/negative tone.
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

/// One segment inside a stacked revenue column (`value` in dollars).
#[derive(Clone, Debug, PartialEq)]
pub struct RevenueSplitSegment {
    pub name: SharedString,
    pub value: f64,
}

impl RevenueSplitSegment {
    /// Create a named dollar segment inside a stacked bar.
    pub fn new(name: impl Into<SharedString>, value: f64) -> Self {
        Self {
            name: name.into(),
            value,
        }
    }
}

/// One stacked column inside [`RevenueChart::Split`].
#[derive(Clone, Debug, PartialEq)]
pub struct RevenueSplitGroup {
    pub name: SharedString,
    pub color_idx: usize,
    pub parts: Vec<RevenueSplitSegment>,
}

impl RevenueSplitGroup {
    /// Total dollar sum across all segments in this column.
    pub fn total(&self) -> f64 {
        self.parts.iter().map(|p| p.value).sum()
    }
}

/// Inline chart payload attached to an assistant turn.
#[derive(Clone, Debug, PartialEq)]
pub enum RevenueChart {
    /// Weekly or monthly comparison curve (`current` vs `previous`, values in `$K`).
    Trend {
        labels: Vec<SharedString>,
        current: Vec<f64>,
        previous: Vec<f64>,
        mark: Option<usize>,
    },
    /// Stacked column chart with region/plan view switching (`values` in `$`).
    Split {
        title: SharedString,
        previous_total: Option<f64>,
        since_label: Option<SharedString>,
        groups: Vec<RevenueSplitGroup>,
        alt_groups: Vec<RevenueSplitGroup>,
    },
    /// Historical actuals followed by dashed forecast curve and optional target (`values` in `$K`).
    Forecast {
        labels: Vec<SharedString>,
        actual: Vec<Option<f64>>,
        forecast: Vec<Option<f64>>,
        target: Option<f64>,
    },
}

/// Focused chart stretch or bar picked by the user to ask a targeted follow-up question.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RevenueChartFocus {
    pub turn_id: SharedString,
    pub from: usize,
    pub to: usize,
    pub label: SharedString,
    pub detail: SharedString,
}

/// One turn in the [`AiChatRevenueScreen`] conversation thread.
#[derive(Clone, Debug, PartialEq)]
pub enum RevenueTurn {
    /// Right-aligned user question with optional chart-focus quote banner above it.
    User {
        id: SharedString,
        quote: Option<SharedString>,
        text: SharedString,
    },
    /// Assistant analyst reply with KPI cards, chart, data sources, and follow-up chips.
    Assistant {
        id: SharedString,
        text: SharedString,
        kpis: Vec<RevenueKpi>,
        chart: Option<RevenueChart>,
        caption: Option<SharedString>,
        sources: Vec<SharedString>,
        next: Vec<SharedString>,
        liked: bool,
        streaming: bool,
    },
}

/// Events emitted by [`AiChatRevenueScreen`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AiChatRevenueEvent {
    /// Sidebar item selected.
    Navigate(SharedString),
    /// User asked a question from the composer, starter chip, or chart selection popover.
    Ask {
        question: SharedString,
        focus: Option<RevenueChartFocus>,
    },
    /// User picked a stretch of weeks or a regional bar on a chart.
    SelectRange(RevenueChartFocus),
    /// Answer summary copied to clipboard.
    CopyAnswer(SharedString),
    /// Source badge clicked (`"Stripe"`, `"HubSpot"`).
    OpenSource(SharedString),
    /// Sheet export / inspector opened for an answer.
    ExportSheet(SharedString),
    /// Helpful thumbs-up toggled on an assistant turn.
    Feedback { turn_id: SharedString, liked: bool },
    /// Assistant answer regenerated.
    Regenerate(SharedString),
}

impl EventEmitter<AiChatRevenueEvent> for AiChatRevenueScreen {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ComposerMenu {
    Add,
    Approval,
    Model,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ActivePick {
    turn_id: SharedString,
    anchor: usize,
    from: usize,
    to: usize,
    is_bar: bool,
    ask_open: bool,
}

/// Interactive AI Chat / Revenue Analyst Pro screen (`ai-chat-revenue`).
pub struct AiChatRevenueScreen {
    id: ElementId,
    shell: Entity<AppShell>,
    standalone: bool,
    turns: Vec<RevenueTurn>,
    period: RevenueChartPeriod,
    split_grouping: RevenueSplitGrouping,
    hovered_point: Option<(SharedString, usize)>,
    last_hovered_point: Option<(SharedString, usize)>,
    keyboard_point: usize,
    active_pick: Option<ActivePick>,
    last_pick_focus: Option<(SharedString, bool, RevenueChartFocus)>,
    dragging_pick: bool,
    active_artifact: Option<RevenueArtifactKind>,
    copied_turn_id: Option<(SharedString, Instant)>,
    toast: Option<(SharedString, Instant)>,
    expanded_sql_turns: Vec<SharedString>,
    approval_mode: RevenueApprovalMode,
    selected_model: usize,
    open_menu: Option<ComposerMenu>,
    last_menu: ComposerMenu,
    attachments: Vec<SharedString>,
    last_attachments: Vec<SharedString>,
    recording: bool,
    recording_started: Option<Instant>,
    composer_editor: Option<Entity<Editor>>,
    pick_editor: Option<Entity<Editor>>,
    editor_focused: bool,
    stream_state: Option<(SharedString, SharedString, Instant)>,
    loading: bool,
    empty: bool,
    focus: FocusHandle,
    scroll: ScrollHandle,
    boot_at: Instant,
    measured_width: Rc<Cell<f32>>,
}

impl Focusable for AiChatRevenueScreen {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl AiChatRevenueScreen {
    /// Construct an [`AiChatRevenueScreen`] preloaded with the Q3 revenue review conversation.
    pub fn new(id: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let id_str: SharedString = id.into();
        let this = cx.entity();
        let nav_groups = sample_revenue_nav_groups(cx);
        let shell = cx.new(|shell_cx| {
            let mut s = AppShell::new(format!("{id_str}-shell"), shell_cx)
                .with_preset(AppShellPreset::Chat, shell_cx)
                .with_groups(nav_groups)
                .with_active("q3-revenue-review")
                .with_side_cards(false)
                .with_sidebar_collapsed(true)
                .with_work_area(true, false)
                .with_panel_width(360.);
            s.bind_page(&this, shell_cx, |screen, w, cx| {
                screen.render_page(w, cx).into_any_element()
            });
            s.bind_panel(&this, |screen, w, cx| screen.render_work_panel(w, cx));
            s
        });
        cx.subscribe(&shell, |this, _, ev: &AppShellEvent, cx| match ev {
            AppShellEvent::Navigate(label) => {
                if label.as_ref() == "New chat" || label.as_ref() == "新建对话" {
                    this.empty = true;
                    cx.notify();
                }
                cx.emit(AiChatRevenueEvent::Navigate(label.clone()));
            }
            AppShellEvent::PanelOpen(false) => {
                cx.notify();
            }
            AppShellEvent::PanelOpen(true) => {
                if this.active_artifact.is_none() {
                    this.active_artifact = Some(RevenueArtifactKind::StripeLedger);
                }
                cx.notify();
            }
            _ => {}
        })
        .detach();

        Self {
            id: ElementId::Name(id_str),
            shell,
            standalone: false,
            turns: sample_revenue_turns(),
            period: RevenueChartPeriod::Weekly,
            split_grouping: RevenueSplitGrouping::ByRegion,
            hovered_point: None,
            last_hovered_point: None,
            keyboard_point: 8,
            active_pick: None,
            last_pick_focus: None,
            dragging_pick: false,
            active_artifact: None,
            copied_turn_id: None,
            toast: None,
            expanded_sql_turns: Vec::new(),
            approval_mode: RevenueApprovalMode::RequestApproval,
            selected_model: 0,
            open_menu: None,
            last_menu: ComposerMenu::Add,
            attachments: Vec::new(),
            last_attachments: Vec::new(),
            recording: false,
            recording_started: None,
            composer_editor: None,
            pick_editor: None,
            editor_focused: false,
            stream_state: None,
            loading: false,
            empty: false,
            focus: cx.focus_handle(),
            scroll: ScrollHandle::new(),
            boot_at: Instant::now(),
            measured_width: Rc::new(Cell::new(860.)),
        }
    }

    /// Render only the inner revenue chat view without the surrounding [`AppShell`].
    pub fn with_standalone(mut self, standalone: bool) -> Self {
        self.standalone = standalone;
        self
    }

    /// Replace the initial conversation turns.
    pub fn with_turns(mut self, turns: Vec<RevenueTurn>) -> Self {
        self.turns = turns;
        self
    }

    /// Override the sidebar chat groups inside [`AppShell`].
    pub fn with_chats(self, groups: Vec<ShellNavGroup>, cx: &mut Context<Self>) -> Self {
        self.shell.update(cx, |s, cx| {
            *s = std::mem::replace(s, AppShell::new("tmp", cx)).with_groups(groups);
            cx.notify();
        });
        self
    }

    /// Open or close the right-hand revenue sheet inspector on mount.
    pub fn with_panel_open(mut self, open: bool, cx: &mut Context<Self>) -> Self {
        if open && self.active_artifact.is_none() {
            self.active_artifact = Some(RevenueArtifactKind::StripeLedger);
        }
        self.shell.update(cx, |s, cx| s.set_panel_open(open, cx));
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

    /// Toggle skeleton loading mode at runtime.
    pub fn set_loading(&mut self, loading: bool, cx: &mut Context<Self>) {
        self.loading = loading;
        cx.notify();
    }

    /// Toggle empty starter mode at runtime.
    pub fn set_empty(&mut self, empty: bool, cx: &mut Context<Self>) {
        self.empty = empty;
        cx.notify();
    }

    /// Open a revenue source sheet inside the right-hand [`AppShell`] work area.
    pub fn open_artifact(&mut self, kind: RevenueArtifactKind, cx: &mut Context<Self>) {
        self.active_artifact = Some(kind);
        self.shell.update(cx, |s, cx| s.set_panel_open(true, cx));
        cx.notify();
    }

    /// Submit a question with optional chart-stretch focus and append a synthesized analyst turn.
    pub fn ask(
        &mut self,
        question: impl Into<String>,
        focus: Option<RevenueChartFocus>,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.loading {
            return false;
        }
        let trimmed = question.into().trim().to_string();
        if trimmed.is_empty() {
            return false;
        }
        self.empty = false;
        self.active_pick = None;
        self.dragging_pick = false;
        self.attachments.clear();

        let quote = focus
            .as_ref()
            .map(|f| SharedString::from(format!("{} · {}", f.label, f.detail)));
        let user_id: SharedString = format!("u-{}", self.turns.len() + 1).into();
        let q_shared: SharedString = trimmed.clone().into();
        self.turns.push(RevenueTurn::User {
            id: user_id,
            quote,
            text: q_shared.clone(),
        });

        let ans_id: SharedString = format!("a-{}", self.turns.len() + 1).into();
        let mut built = build_revenue_answer(&trimmed, focus.as_ref(), &self.turns);
        let full_text = match &mut built {
            RevenueTurn::Assistant {
                id,
                text,
                streaming,
                ..
            } => {
                *id = ans_id.clone();
                let target = text.clone();
                if !Theme::global(cx).reduced_motion {
                    *text = SharedString::default();
                    *streaming = true;
                }
                target
            }
            RevenueTurn::User { text, .. } => text.clone(),
        };
        self.turns.push(built);
        if !Theme::global(cx).reduced_motion {
            self.stream_state = Some((ans_id, full_text, Instant::now()));
        } else {
            self.stream_state = None;
        }

        self.scroll.scroll_to_bottom();
        cx.emit(AiChatRevenueEvent::Ask {
            question: q_shared,
            focus,
        });
        cx.notify();
        true
    }

    fn current_focus_for_pick(&self, pick: &ActivePick) -> Option<RevenueChartFocus> {
        let turn = self.turns.iter().find(|t| match t {
            RevenueTurn::Assistant { id, .. } => id == &pick.turn_id,
            _ => false,
        })?;
        let RevenueTurn::Assistant {
            chart: Some(chart), ..
        } = turn
        else {
            return None;
        };
        match chart {
            RevenueChart::Trend {
                labels,
                current,
                previous,
                ..
            } => {
                let (lbl, det) = compute_trend_focus(labels, current, previous, pick.from, pick.to);
                Some(RevenueChartFocus {
                    turn_id: pick.turn_id.clone(),
                    from: pick.from,
                    to: pick.to,
                    label: lbl.into(),
                    detail: det.into(),
                })
            }
            RevenueChart::Forecast {
                labels,
                actual,
                forecast,
                ..
            } => {
                let vals: Vec<f64> = (0..labels.len())
                    .map(|i| {
                        actual
                            .get(i)
                            .and_then(|v| *v)
                            .or_else(|| forecast.get(i).and_then(|v| *v))
                            .unwrap_or(0.0)
                    })
                    .collect();
                let lo = pick.from.min(pick.to).min(labels.len().saturating_sub(1));
                let hi = pick.from.max(pick.to).min(labels.len().saturating_sub(1));
                let sum_k: f64 = vals[lo..=hi].iter().sum();
                let label = if lo == hi {
                    labels[lo].to_string()
                } else {
                    format!("{} – {}", labels[lo], labels[hi])
                };
                let detail = format!("{} projected", format_k_dollars(sum_k));
                Some(RevenueChartFocus {
                    turn_id: pick.turn_id.clone(),
                    from: lo,
                    to: hi,
                    label: label.into(),
                    detail: detail.into(),
                })
            }
            RevenueChart::Split {
                groups, alt_groups, ..
            } => {
                let active_groups = if self.split_grouping == RevenueSplitGrouping::ByRegion {
                    groups
                } else {
                    alt_groups
                };
                let idx = pick.from.min(active_groups.len().saturating_sub(1));
                let g = active_groups.get(idx)?;
                let grand: f64 = active_groups.iter().map(RevenueSplitGroup::total).sum();
                let tot = g.total();
                let share = if grand > 0.0 {
                    (tot / grand * 100.0).round() as i32
                } else {
                    0
                };
                Some(RevenueChartFocus {
                    turn_id: pick.turn_id.clone(),
                    from: idx,
                    to: idx,
                    label: g.name.clone(),
                    detail: format!("{} · {share}% of total", format_dollars_compact(tot)).into(),
                })
            }
        }
    }

    fn ensure_editors(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.composer_editor.is_none() {
            let ph: SharedString = tr(
                cx,
                "Ask about revenue, customers or plans…",
                "询问营收、客户或订阅方案…",
            )
            .into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Single, w, cx);
                e.set_placeholder(ph);
                e.font_size = 13.;
                e
            });
            cx.subscribe_in(&ed, w, |this, ed_ent, ev: &EditEvent, w, cx| match ev {
                EditEvent::Focused => {
                    this.editor_focused = true;
                    this.open_menu = None;
                    cx.notify();
                }
                EditEvent::Blurred => {
                    this.editor_focused = false;
                    cx.notify();
                }
                EditEvent::Changed => {
                    cx.notify();
                }
                EditEvent::Submitted => {
                    let val = ed_ent.read(cx).value().to_string();
                    if this.ask(val, None, cx) {
                        ed_ent.update(cx, |e, cx| e.set_value("", w, cx));
                    }
                }
                EditEvent::Escape if this.recording => {
                    this.recording = false;
                    this.recording_started = None;
                    cx.notify();
                }
                _ => {}
            })
            .detach();
            self.composer_editor = Some(ed);
        }

        if self.pick_editor.is_none() {
            let ph: SharedString = tr(cx, "Ask about this selection…", "针对所选区间提问…").into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Single, w, cx);
                e.set_placeholder(ph);
                e.font_size = 13.;
                e
            });
            cx.subscribe_in(&ed, w, |this, ed_ent, ev: &EditEvent, w, cx| match ev {
                EditEvent::Submitted => {
                    let val = ed_ent.read(cx).value().to_string();
                    let focus = this
                        .active_pick
                        .as_ref()
                        .and_then(|pk| this.current_focus_for_pick(pk));
                    if this.ask(val, focus, cx) {
                        ed_ent.update(cx, |e, cx| e.set_value("", w, cx));
                        this.active_pick = None;
                    }
                }
                EditEvent::Escape => {
                    this.active_pick = None;
                    cx.notify();
                }
                EditEvent::Changed => {
                    cx.notify();
                }
                _ => {}
            })
            .detach();
            self.pick_editor = Some(ed);
        }
    }

    fn advance_streaming(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let reduced = Theme::global(cx).reduced_motion;
        if let Some((turn_id, full_text, started)) = self.stream_state.clone() {
            let elapsed_ms = started.elapsed().as_secs_f32() * 1000.;
            if reduced || elapsed_ms >= 1150. {
                for t in &mut self.turns {
                    if let RevenueTurn::Assistant {
                        id,
                        text,
                        streaming,
                        ..
                    } = t
                        && id == &turn_id
                    {
                        *text = full_text.clone();
                        *streaming = false;
                    }
                }
                self.stream_state = None;
            } else {
                let think_ms = 260.;
                if elapsed_ms > think_ms {
                    let frac = ((elapsed_ms - think_ms) / 890.).clamp(0., 1.);
                    let chars_total = full_text.chars().count();
                    let vis_chars = ((chars_total as f32 * frac).ceil() as usize).min(chars_total);
                    let partial: String = full_text.chars().take(vis_chars).collect();
                    for t in &mut self.turns {
                        if let RevenueTurn::Assistant { id, text, .. } = t
                            && id == &turn_id
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
            .copied_turn_id
            .as_ref()
            .is_some_and(|(_, t)| t.elapsed().as_secs_f32() >= 2.45)
        {
            self.copied_turn_id = None;
        } else if self.copied_turn_id.is_some() && !reduced {
            w.request_animation_frame();
        }
        if (self.loading || self.recording) && !reduced {
            w.request_animation_frame();
        }
    }

    fn handle_key_down(&mut self, ev: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        let key = ev.keystroke.key.as_str();
        if key == "escape" {
            if self.active_pick.is_some() || self.open_menu.is_some() {
                self.active_pick = None;
                self.dragging_pick = false;
                self.open_menu = None;
                cx.notify();
            }
            return;
        }
        if self.editor_focused {
            return;
        }
        if key == "/" {
            if let Some(ed) = &self.composer_editor {
                {
                    let fh = ed.read(cx).focus.clone();
                    fh.focus(w, cx);
                }
                w.prevent_default();
            }
            return;
        }

        // Keyboard stepping & picking across the latest assistant chart
        let Some((last_id, point_count, is_split)) =
            self.turns.iter().rev().find_map(|t| match t {
                RevenueTurn::Assistant {
                    id, chart: Some(c), ..
                } => match c {
                    RevenueChart::Trend { labels, .. } | RevenueChart::Forecast { labels, .. } => {
                        Some((id.clone(), labels.len(), false))
                    }
                    RevenueChart::Split { groups, .. } => Some((id.clone(), groups.len(), true)),
                },
                _ => None,
            })
        else {
            return;
        };
        if point_count == 0 {
            return;
        }

        match key {
            "left" | "h" => {
                self.keyboard_point = self.keyboard_point.min(point_count - 1).saturating_sub(1);
                self.hovered_point = Some((last_id.clone(), self.keyboard_point));
                if let Some(pk) = &mut self.active_pick
                    && pk.turn_id == last_id
                    && !pk.is_bar
                    && !pk.ask_open
                {
                    pk.from = pk.anchor.min(self.keyboard_point);
                    pk.to = pk.anchor.max(self.keyboard_point);
                }
                cx.notify();
            }
            "right" | "l" => {
                self.keyboard_point = (self.keyboard_point + 1).min(point_count - 1);
                self.hovered_point = Some((last_id.clone(), self.keyboard_point));
                if let Some(pk) = &mut self.active_pick
                    && pk.turn_id == last_id
                    && !pk.is_bar
                    && !pk.ask_open
                {
                    pk.from = pk.anchor.min(self.keyboard_point);
                    pk.to = pk.anchor.max(self.keyboard_point);
                }
                cx.notify();
            }
            "home" => {
                self.keyboard_point = 0;
                self.hovered_point = Some((last_id, 0));
                cx.notify();
            }
            "end" => {
                self.keyboard_point = point_count - 1;
                self.hovered_point = Some((last_id, self.keyboard_point));
                cx.notify();
            }
            "enter" | " " => {
                let pt = self.keyboard_point.min(point_count - 1);
                if is_split {
                    let pk = ActivePick {
                        turn_id: last_id,
                        anchor: pt,
                        from: pt,
                        to: pt,
                        is_bar: true,
                        ask_open: true,
                    };
                    if let Some(f) = self.current_focus_for_pick(&pk) {
                        cx.emit(AiChatRevenueEvent::SelectRange(f));
                    }
                    self.active_pick = Some(pk);
                    if let Some(ed) = &self.pick_editor {
                        ed.update(cx, |e, cx| e.set_value("", w, cx));
                        {
                            let fh = ed.read(cx).focus.clone();
                            fh.focus(w, cx);
                        }
                    }
                } else if let Some(mut pk) = self.active_pick.clone()
                    && pk.turn_id == last_id
                    && !pk.ask_open
                {
                    pk.ask_open = true;
                    if let Some(f) = self.current_focus_for_pick(&pk) {
                        cx.emit(AiChatRevenueEvent::SelectRange(f));
                    }
                    self.active_pick = Some(pk);
                    if let Some(ed) = &self.pick_editor {
                        ed.update(cx, |e, cx| e.set_value("", w, cx));
                        {
                            let fh = ed.read(cx).focus.clone();
                            fh.focus(w, cx);
                        }
                    }
                } else {
                    self.active_pick = Some(ActivePick {
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

    fn render_page(&mut self, w: &mut Window, cx: &mut Context<Self>) -> Stateful<Div> {
        self.ensure_editors(w, cx);
        self.advance_streaming(w, cx);

        if let Some(m) = self.open_menu {
            self.last_menu = m;
        }
        if !self.attachments.is_empty() {
            self.last_attachments = self.attachments.clone();
        }
        if let Some((tid, idx)) = &self.hovered_point {
            self.last_hovered_point = Some((tid.clone(), *idx));
        }
        if let Some(pk) = self.active_pick.clone()
            && pk.ask_open
            && let Some(foc) = self.current_focus_for_pick(&pk)
        {
            self.last_pick_focus = Some((pk.turn_id, pk.is_bar, foc));
        }

        let p = ProPalette::get(cx);
        let reduced = Theme::global(cx).reduced_motion;
        let elapsed_ms = self.boot_at.elapsed().as_secs_f32() * 1000.;
        let width = self.measured_width.get();
        let narrow = width < 560.;
        let col_pad = if narrow { 12. } else { 24. };
        let measure_cell = self.measured_width.clone();

        let thread_state_key: SharedString = if self.loading {
            "loading".into()
        } else if self.empty || self.turns.is_empty() {
            "empty".into()
        } else {
            "thread".into()
        };

        let thread_inner = if self.loading {
            let phase = if reduced {
                0.
            } else {
                (elapsed_ms / 1000.) * 0.72
            };
            Self::render_loading_skeleton(narrow, phase, cx).into_any_element()
        } else if self.empty || self.turns.is_empty() {
            self.render_empty_starter(cx).into_any_element()
        } else {
            let turns_snapshot = self.turns.clone();
            let mut turn_els = Vec::with_capacity(turns_snapshot.len());
            for turn in turns_snapshot {
                turn_els.push(self.render_turn(turn, narrow, elapsed_ms, reduced, w, cx));
            }
            div()
                .w_full()
                .max_w(px(760.))
                .mx_auto()
                .pt(px(if narrow { 44. } else { 32. }))
                .pb(px(24.))
                .flex()
                .flex_col()
                .gap(px(28.))
                .children(turn_els)
                .into_any_element()
        };

        let thread_body = pro_view_transition(
            pro_eid(&self.id, "thread-mode-vtx"),
            thread_state_key,
            thread_inner,
        );

        let composer = self.render_composer(narrow, elapsed_ms, reduced, w, cx);

        div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::handle_key_down))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, w, cx| {
                    if this.dragging_pick {
                        this.dragging_pick = false;
                        if let Some(pk) = &mut this.active_pick {
                            pk.ask_open = true;
                        }
                        if let Some(pk) = this.active_pick.clone()
                            && let Some(f) = this.current_focus_for_pick(&pk)
                        {
                            cx.emit(AiChatRevenueEvent::SelectRange(f));
                        }
                        if let Some(ed) = &this.pick_editor {
                            ed.update(cx, |e, cx| e.set_value("", w, cx));
                            {
                                let fh = ed.read(cx).focus.clone();
                                fh.focus(w, cx);
                            }
                        }
                        cx.notify();
                    }
                }),
            )
            .relative()
            .size_full()
            .bg(p.panel)
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
                .size_0(),
            )
            .child(
                div()
                    .id(pro_eid(&self.id, "thread-scroll"))
                    .track_scroll(&self.scroll)
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .px(px(col_pad))
                    .child(thread_body),
            )
            .child(composer)
            .child(pro_toast_banner(self.toast.clone()))
    }

    fn render_turn(
        &self,
        turn: RevenueTurn,
        narrow: bool,
        elapsed_ms: f32,
        reduced: bool,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let p = ProPalette::get(cx);
        let rva_blue: Hsla = if p.dark {
            rgb(0x4fb0fe).into()
        } else {
            rgb(0x0190fe).into()
        };
        let rva_good: Hsla = if p.dark {
            rgb(0x7dcf9f).into()
        } else {
            rgb(0x1e7a47).into()
        };
        let rva_bad: Hsla = if p.dark {
            rgb(0xff8a7a).into()
        } else {
            rgb(0xd72400).into()
        };
        let rva_edge: Hsla = if p.dark {
            hsla(0., 0., 1., 0.09)
        } else {
            hsla(0., 0., 0., 0.09)
        };

        match turn {
            RevenueTurn::User { quote, text, .. } => div()
                .w_full()
                .flex()
                .flex_col()
                .items_end()
                .gap(px(4.))
                .when_some(quote, |d, q| {
                    d.child(
                        div()
                            .max_w(relative(0.82))
                            .pl(px(8.))
                            .pr(px(10.))
                            .py(px(3.))
                            .rounded(px(8.))
                            .bg(rva_blue.opacity(0.08))
                            .border_l_2()
                            .border_color(rva_blue)
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .text_size(px(12.5))
                            .line_height(px(18.))
                            .text_color(p.text)
                            .child(pro_icon(ProIcon::UsageChart, 13., rva_blue))
                            .child(q),
                    )
                })
                .child(
                    div()
                        .max_w(relative(0.80))
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

            RevenueTurn::Assistant {
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
                let mark_color: Hsla = if p.dark {
                    rgba(0x4d8dffd9).into()
                } else {
                    rgba(0x0060e2c7).into()
                };
                let thinking_only = streaming && text.is_empty();

                if thinking_only {
                    return div()
                        .w_full()
                        .flex()
                        .items_center()
                        .gap(px(12.))
                        .when(!narrow, |d| {
                            d.child(analyst_pixel_mark(mark_color, true, elapsed_ms, reduced))
                        })
                        .child(div().text_size(px(13.)).text_color(p.muted).child(tr(
                            cx,
                            "Reading Stripe and HubSpot…",
                            "正在读取 Stripe 与 HubSpot 数据…",
                        )))
                        .into_any_element();
                }

                // Output widgets sit flush-left with the 16px pixel mark + 12px gap (-28px) on desktop
                let out_offset = if narrow { 0. } else { -28. };
                let mut out_col = div().flex().flex_col().gap(px(16.)).min_w_0();

                if !kpis.is_empty() && !streaming {
                    let kpi_el = if narrow {
                        let mut rows = div().flex().flex_col().gap(px(8.));
                        if kpis.len() == 3 {
                            rows = rows
                                .child(Self::render_kpi_card(
                                    &kpis[0], rva_edge, rva_good, rva_bad, &p,
                                ))
                                .child(
                                    div()
                                        .flex()
                                        .gap(px(8.))
                                        .child(div().flex_1().min_w_0().child(
                                            Self::render_kpi_card(
                                                &kpis[1], rva_edge, rva_good, rva_bad, &p,
                                            ),
                                        ))
                                        .child(div().flex_1().min_w_0().child(
                                            Self::render_kpi_card(
                                                &kpis[2], rva_edge, rva_good, rva_bad, &p,
                                            ),
                                        )),
                                );
                        } else {
                            for k in &kpis {
                                rows = rows.child(Self::render_kpi_card(
                                    k, rva_edge, rva_good, rva_bad, &p,
                                ));
                            }
                        }
                        rows
                    } else {
                        div()
                            .ml(px(out_offset))
                            .flex()
                            .gap(px(10.))
                            .children(kpis.iter().map(|k| {
                                div().flex_1().min_w_0().child(Self::render_kpi_card(
                                    k, rva_edge, rva_good, rva_bad, &p,
                                ))
                            }))
                    };
                    out_col = out_col.child(kpi_el);
                }

                if let Some(ch) = chart
                    && !streaming
                {
                    let chart_el = match ch {
                        RevenueChart::Trend {
                            labels,
                            current,
                            previous,
                            mark,
                        } => self.render_trend_figure(
                            &id, caption, labels, current, previous, mark, out_offset, narrow,
                            rva_blue, rva_edge, reduced, w, cx,
                        ),
                        RevenueChart::Forecast {
                            labels,
                            actual,
                            forecast,
                            target,
                        } => self.render_forecast_figure(
                            &id, caption, labels, actual, forecast, target, out_offset, narrow,
                            rva_blue, rva_edge, w, cx,
                        ),
                        RevenueChart::Split {
                            title,
                            previous_total,
                            since_label,
                            groups,
                            alt_groups,
                        } => self.render_split_figure(
                            &id,
                            title,
                            previous_total,
                            since_label,
                            groups,
                            alt_groups,
                            out_offset,
                            narrow,
                            rva_blue,
                            rva_edge,
                            w,
                            cx,
                        ),
                    };
                    out_col = out_col.child(chart_el);
                }

                if !streaming {
                    let is_copied = self
                        .copied_turn_id
                        .as_ref()
                        .is_some_and(|(cid, t)| cid == &id && t.elapsed().as_secs_f32() < 1.8);
                    let sql_open = self.expanded_sql_turns.iter().any(|tid| tid == &id);
                    let copy_text = text.clone();
                    let tid_copy = id.clone();
                    let tid_dl = id.clone();
                    let tid_like = id.clone();
                    let tid_regen = id.clone();
                    let tid_sql = id.clone();
                    let regen_text = text.clone();

                    let sql_snippet: &'static str = if id.as_ref().contains("split") {
                        "SELECT region, plan_tier, SUM(net_amount_cents) / 100.0 AS rev_usd\nFROM stripe.settled_invoices JOIN hubspot.accounts USING (org_id)\nWHERE quarter = '2026-Q3' GROUP BY 1, 2 ORDER BY rev_usd DESC;"
                    } else if id.as_ref().contains("forecast") {
                        "SELECT week_bucket, actual_mrr_k, p50_forecast_k, p80_band_k\nFROM finance.revenue_forecast_v4 WHERE horizon_quarter = '2026-Q4';"
                    } else {
                        "SELECT date_trunc('week', paid_at) AS wk, SUM(net_cents) / 100000.0 AS rev_k\nFROM stripe.invoices WHERE status = 'paid' AND paid_at >= '2025-07-01'\nGROUP BY 1 ORDER BY 1;"
                    };

                    let foot = div()
                        .ml(px(out_offset))
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
                                        sources.into_iter().enumerate().map(|(si, src)| {
                                            let src_click = src.clone();
                                            activate(
                                                div()
                                                    .id(pro_eid(&self.id, format!("src-{id}-{si}")))
                                                    .role(Role::Button)
                                                    .tab_index(0)
                                                    .h(px(20.))
                                                    .px(px(7.))
                                                    .rounded(px(6.))
                                                    .bg(p.chip)
                                                    .hover(move |s| {
                                                        s.bg(p.chip_hover).text_color(p.ink)
                                                    })
                                                    .flex()
                                                    .items_center()
                                                    .text_size(px(12.))
                                                    .text_color(p.text)
                                                    .cursor_pointer()
                                                    .child(src),
                                                cx,
                                                move |this, _, cx| {
                                                    let kind = if src_click
                                                        .to_ascii_lowercase()
                                                        .contains("hubspot")
                                                    {
                                                        RevenueArtifactKind::HubSpotDeals
                                                    } else {
                                                        RevenueArtifactKind::StripeLedger
                                                    };
                                                    this.open_artifact(kind, cx);
                                                    cx.emit(AiChatRevenueEvent::OpenSource(
                                                        src_click.clone(),
                                                    ));
                                                },
                                            )
                                        }),
                                    )
                                })
                                .child(activate(
                                    div()
                                        .id(pro_eid(&self.id, format!("sql-btn-{id}")))
                                        .role(Role::Button)
                                        .tab_index(0)
                                        .h(px(20.))
                                        .px(px(7.))
                                        .rounded(px(6.))
                                        .bg(if sql_open { p.accent_soft } else { p.chip })
                                        .hover(move |s| s.bg(p.chip_hover).text_color(p.ink))
                                        .flex()
                                        .items_center()
                                        .gap(px(4.))
                                        .text_size(px(11.5))
                                        .text_color(if sql_open { rva_blue } else { p.muted })
                                        .cursor_pointer()
                                        .child(tr(cx, "SQL & methodology", "查询与口径"))
                                        .child(pro_icon(
                                            if sql_open {
                                                ProIcon::ChevronUp
                                            } else {
                                                ProIcon::ChevronDown
                                            },
                                            11.,
                                            if sql_open { rva_blue } else { p.muted },
                                        )),
                                    cx,
                                    move |this, _, cx| {
                                        if let Some(pos) = this
                                            .expanded_sql_turns
                                            .iter()
                                            .position(|x| x == &tid_sql)
                                        {
                                            this.expanded_sql_turns.remove(pos);
                                        } else {
                                            this.expanded_sql_turns.push(tid_sql.clone());
                                        }
                                        cx.notify();
                                    },
                                )),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(2.))
                                .child(Self::action_icon_btn(
                                    pro_eid(&self.id, format!("act-copy-{id}")),
                                    if is_copied {
                                        tr(cx, "Copied", "已复制")
                                    } else {
                                        tr(cx, "Copy answer", "复制回答")
                                    },
                                    if is_copied {
                                        CHECK_LINE_SVG
                                    } else {
                                        COPY_OVERLAP_SVG
                                    },
                                    is_copied,
                                    &p,
                                    w,
                                    cx,
                                    move |this, _, cx| {
                                        cx.write_to_clipboard(ClipboardItem::new_string(
                                            copy_text.to_string(),
                                        ));
                                        let now = Instant::now();
                                        this.copied_turn_id = Some((tid_copy.clone(), now));
                                        this.toast = Some((
                                            tr(cx, "Copied summary to clipboard", "已复制分析摘要")
                                                .into(),
                                            now,
                                        ));
                                        cx.emit(AiChatRevenueEvent::CopyAnswer(tid_copy.clone()));
                                        cx.notify();
                                    },
                                ))
                                .child(Self::action_icon_btn(
                                    pro_eid(&self.id, format!("act-dl-{id}")),
                                    tr(cx, "Open revenue sheet", "打开营收数据表"),
                                    DOWNLOAD_ARROW_SVG,
                                    false,
                                    &p,
                                    w,
                                    cx,
                                    move |this, _, cx| {
                                        this.open_artifact(RevenueArtifactKind::WeeklyCsv, cx);
                                        this.toast = Some((
                                            tr(
                                                cx,
                                                "Opened Q3 weekly revenue sheet",
                                                "已打开 Q3 周度营收表",
                                            )
                                            .into(),
                                            Instant::now(),
                                        ));
                                        cx.emit(AiChatRevenueEvent::ExportSheet(tid_dl.clone()));
                                    },
                                ))
                                .child(Self::action_icon_btn(
                                    pro_eid(&self.id, format!("act-like-{id}")),
                                    tr(cx, "Good answer", "回答有帮助"),
                                    THUMBS_UP_SVG,
                                    liked,
                                    &p,
                                    w,
                                    cx,
                                    move |this, _, cx| {
                                        let mut new_liked = false;
                                        for t in &mut this.turns {
                                            if let RevenueTurn::Assistant {
                                                id: tid, liked: lk, ..
                                            } = t
                                                && tid == &tid_like
                                            {
                                                *lk = !*lk;
                                                new_liked = *lk;
                                            }
                                        }
                                        cx.emit(AiChatRevenueEvent::Feedback {
                                            turn_id: tid_like.clone(),
                                            liked: new_liked,
                                        });
                                        cx.notify();
                                    },
                                ))
                                .child(pro_icon_btn(
                                    pro_eid(&self.id, format!("act-regen-{id}")),
                                    tr(cx, "Regenerate", "重新生成"),
                                    ProIcon::Refresh,
                                    28.,
                                    false,
                                    cx,
                                    move |this, _, cx| {
                                        if !Theme::global(cx).reduced_motion {
                                            for t in &mut this.turns {
                                                if let RevenueTurn::Assistant {
                                                    id: tid,
                                                    text: tx,
                                                    streaming: st,
                                                    ..
                                                } = t
                                                    && tid == &tid_regen
                                                {
                                                    *tx = SharedString::default();
                                                    *st = true;
                                                }
                                            }
                                            this.stream_state = Some((
                                                tid_regen.clone(),
                                                regen_text.clone(),
                                                Instant::now(),
                                            ));
                                        }
                                        cx.emit(AiChatRevenueEvent::Regenerate(tid_regen.clone()));
                                        cx.notify();
                                    },
                                )),
                        );
                    out_col = out_col.child(foot);

                    let sql_card = div()
                        .ml(px(out_offset))
                        .p(px(12.))
                        .rounded(px(11.))
                        .bg(p.subtle)
                        .border_1()
                        .border_color(rva_edge)
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap(px(8.))
                                .child(
                                    div()
                                        .text_size(px(11.5))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.muted)
                                        .child(tr(
                                            cx,
                                            "Read-only warehouse query · accrual net of refunds",
                                            "只读数仓查询 · 已扣除退款净额口径",
                                        )),
                                )
                                .child(pro_btn(
                                    pro_eid(&self.id, format!("copy-sql-{id}")),
                                    tr(cx, "Copy SQL", "复制 SQL"),
                                    Some(ProIcon::Copy),
                                    ProBtnStyle::Secondary,
                                    true,
                                    cx,
                                    move |this, _, cx| {
                                        cx.write_to_clipboard(ClipboardItem::new_string(
                                            sql_snippet.to_string(),
                                        ));
                                        this.toast = Some((
                                            tr(cx, "Copied SQL query", "已复制 SQL 查询").into(),
                                            Instant::now(),
                                        ));
                                        cx.notify();
                                    },
                                )),
                        )
                        .child(
                            div()
                                .px(px(10.))
                                .py(px(8.))
                                .rounded(px(8.))
                                .bg(p.card)
                                .border_1()
                                .border_color(p.line_soft)
                                .text_size(px(11.5))
                                .line_height(px(17.))
                                .text_color(p.text)
                                .child(sql_snippet),
                        )
                        .into_any_element();
                    out_col = out_col.child(pro_fold(
                        pro_eid(&self.id, format!("sql-fold-{id}")),
                        sql_open,
                        104.,
                        16.,
                        move || sql_card,
                    ));

                    if !next.is_empty() {
                        let chips = div()
                            .ml(px(out_offset))
                            .flex()
                            .flex_wrap()
                            .gap(px(6.))
                            .children(next.into_iter().enumerate().map(|(ni, prompt)| {
                                let p_ask = prompt.to_string();
                                activate(
                                    div()
                                        .id(pro_eid(&self.id, format!("next-{id}-{ni}")))
                                        .role(Role::Button)
                                        .tab_index(0)
                                        .min_h(px(30.))
                                        .pl(px(12.))
                                        .pr(px(10.))
                                        .py(px(5.))
                                        .rounded(px(9.))
                                        .bg(p.card)
                                        .border_1()
                                        .border_color(rva_edge)
                                        .hover(move |s| {
                                            s.border_color(p.line_strong).text_color(p.ink)
                                        })
                                        .focus_visible(move |s| s.border_1().border_color(p.focus))
                                        .flex()
                                        .items_center()
                                        .gap(px(6.))
                                        .text_size(px(13.))
                                        .line_height(px(20.))
                                        .text_color(p.text)
                                        .cursor_pointer()
                                        .child(prompt)
                                        .child(pro_icon(ProIcon::ExternalArrow, 13., p.faint)),
                                    cx,
                                    move |this, _, cx| {
                                        this.ask(p_ask.clone(), None, cx);
                                    },
                                )
                            }));
                        out_col = out_col.child(chips);
                    }
                }

                let out_vtx = pro_view_transition(
                    pro_eid(&self.id, format!("turn-out-vtx-{id}")),
                    if streaming { "streaming" } else { "ready" },
                    out_col,
                );

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
                            .gap(px(16.))
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
                            .when(!streaming, |d| d.child(out_vtx)),
                    )
                    .into_any_element()
            }
        }
    }

    fn render_kpi_card(
        kpi: &RevenueKpi,
        edge: Hsla,
        good_c: Hsla,
        bad_c: Hsla,
        p: &ProPalette,
    ) -> Div {
        div()
            .px(px(14.))
            .py(px(12.))
            .rounded(px(12.))
            .bg(p.card)
            .border_1()
            .border_color(edge)
            .flex()
            .flex_col()
            .gap(px(2.))
            .min_w_0()
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
                        .text_color(if kpi.good { good_c } else { bad_c })
                        .truncate()
                        .child(chg),
                )
            })
    }

    #[allow(clippy::too_many_arguments)]
    fn render_trend_figure(
        &self,
        turn_id: &SharedString,
        caption: Option<SharedString>,
        labels: Vec<SharedString>,
        current: Vec<f64>,
        previous: Vec<f64>,
        mark: Option<usize>,
        out_offset: f32,
        narrow: bool,
        rva_blue: Hsla,
        rva_edge: Hsla,
        reduced: bool,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let p = ProPalette::get(cx);
        let is_monthly = self.period == RevenueChartPeriod::Monthly && current.len() >= 9;

        let (disp_labels, disp_cur, disp_prev, disp_mark, y_top, y_mid, y_bot) = if is_monthly {
            let chunk = (current.len() / 3).max(1);
            let m0_c: f64 = current[..chunk].iter().sum();
            let m1_c: f64 = current[chunk..(chunk * 2)].iter().sum();
            let m2_c: f64 = current[(chunk * 2)..].iter().sum();
            let m0_p: f64 = previous[..chunk.min(previous.len())].iter().sum();
            let m1_p: f64 = previous[chunk.min(previous.len())..(chunk * 2).min(previous.len())]
                .iter()
                .sum();
            let m2_p: f64 = previous[(chunk * 2).min(previous.len())..].iter().sum();
            (
                vec![
                    SharedString::from("Jul"),
                    SharedString::from("Aug"),
                    SharedString::from("Sep"),
                ],
                vec![m0_c, m1_c, m2_c],
                vec![m0_p, m1_p, m2_p],
                Some(2usize),
                600.0_f64,
                400.0_f64,
                200.0_f64,
            )
        } else {
            (
                labels,
                current.clone(),
                previous,
                mark,
                150.0_f64,
                100.0_f64,
                50.0_f64,
            )
        };

        let n = disp_cur.len();
        let active_pick = self
            .active_pick
            .as_ref()
            .filter(|pk| &pk.turn_id == turn_id && !pk.is_bar && !is_monthly)
            .cloned();

        let active_idx = if active_pick.is_some() {
            None
        } else {
            self.hovered_point
                .as_ref()
                .filter(|(tid, idx)| tid == turn_id && *idx < n)
                .map(|(_, idx)| *idx)
                .or_else(|| disp_mark.filter(|&m| m < n))
        };

        let prev_stroke = if p.dark {
            hsla(0., 0., 1., 0.36)
        } else {
            hsla(0., 0., 0., 0.30)
        };
        let grid_stroke = if p.dark {
            hsla(0., 0., 1., 0.08)
        } else {
            hsla(0., 0., 0., 0.06)
        };
        let guide_stroke = if p.dark {
            hsla(0., 0., 1., 0.20)
        } else {
            hsla(0., 0., 0., 0.13)
        };
        let card_bg = p.card;

        let min_y = y_bot as f32;
        let max_y = y_top as f32;
        let span_y = (max_y - min_y).max(1.);

        let mut cur_norm = Vec::with_capacity(n);
        for (i, &v) in disp_cur.iter().enumerate() {
            let target_ny = ((v as f32 - min_y) / span_y).clamp(0., 1.);
            cur_norm.push(
                pro_spring_f32(
                    pro_eid(&self.id, format!("tr-cy-{turn_id}-{i}")),
                    target_ny,
                    0.28,
                    w,
                    cx,
                )
                .clamp(0., 1.),
            );
        }
        let mut prev_norm = Vec::with_capacity(disp_prev.len());
        for (i, &v) in disp_prev.iter().enumerate() {
            let target_ny = ((v as f32 - min_y) / span_y).clamp(0., 1.);
            prev_norm.push(
                pro_spring_f32(
                    pro_eid(&self.id, format!("tr-py-{turn_id}-{i}")),
                    target_ny,
                    0.28,
                    w,
                    cx,
                )
                .clamp(0., 1.),
            );
        }

        let denom = (n.saturating_sub(1)).max(1) as f32;
        let fallback_idx = active_idx
            .or_else(|| {
                self.last_hovered_point
                    .as_ref()
                    .filter(|(tid, idx)| tid == turn_id && *idx < n)
                    .map(|(_, idx)| *idx)
            })
            .or_else(|| disp_mark.filter(|&m| m < n))
            .unwrap_or(0)
            .min(n.saturating_sub(1));
        let marker_tx = pro_spring_f32(
            pro_eid(&self.id, format!("tr-mtx-{turn_id}")),
            fallback_idx as f32 / denom,
            0.18,
            w,
            cx,
        )
        .clamp(0., 1.);
        let marker_cy = pro_spring_f32(
            pro_eid(&self.id, format!("tr-mcy-{turn_id}")),
            cur_norm.get(fallback_idx).copied().unwrap_or(0.5),
            0.18,
            w,
            cx,
        )
        .clamp(0., 1.);
        let marker_py = pro_spring_f32(
            pro_eid(&self.id, format!("tr-mpy-{turn_id}")),
            prev_norm.get(fallback_idx).copied().unwrap_or(0.4),
            0.18,
            w,
            cx,
        )
        .clamp(0., 1.);

        let pick_range_spring = active_pick.as_ref().map(|pk| {
            let lo_t = pk.from.min(pk.to) as f32 / denom;
            let hi_t = pk.from.max(pk.to) as f32 / denom;
            let s_lo = pro_spring_f32(
                pro_eid(&self.id, format!("tr-blo-{turn_id}")),
                lo_t,
                0.16,
                w,
                cx,
            )
            .clamp(0., 1.);
            let s_hi = pro_spring_f32(
                pro_eid(&self.id, format!("tr-bhi-{turn_id}")),
                hi_t,
                0.16,
                w,
                cx,
            )
            .clamp(0., 1.);
            (s_lo, s_hi, pk.from == pk.to)
        });

        let reveal = if reduced {
            1.0
        } else {
            pro_ease((self.boot_at.elapsed().as_secs_f32() / 0.85).clamp(0., 1.))
        };
        let show_marker = active_idx.is_some();

        let plot_canvas = canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let bw = f32::from(bounds.size.width);
                let bh = f32::from(bounds.size.height);
                if bw <= 8. || bh <= 8. || cur_norm.len() < 2 {
                    return;
                }
                let pt_at = |i: usize, ny: f32| -> Point<Pixels> {
                    let tx = i as f32 / (cur_norm.len() - 1) as f32;
                    let ty = 1. - ny.clamp(0., 1.);
                    point(bounds.left() + px(tx * bw), bounds.top() + px(ty * bh))
                };

                // 3 horizontal grid hairlines (top, middle, bottom)
                for frac in [0.0_f32, 0.5, 1.0] {
                    let gy = bounds.top() + px(frac * bh);
                    let mut g_pb = PathBuilder::stroke(px(1.));
                    g_pb.move_to(point(bounds.left(), gy));
                    g_pb.line_to(point(bounds.right(), gy));
                    if let Ok(path) = g_pb.build() {
                        window.paint_path(path, grid_stroke);
                    }
                }

                // Spring-animated range-drag selection band
                if let Some((lo_f, hi_f, single)) = pick_range_spring {
                    let x0 = if single {
                        (lo_f * bw - 12.).max(0.)
                    } else {
                        lo_f * bw
                    };
                    let x1 = if single {
                        (hi_f * bw + 12.).min(bw)
                    } else {
                        hi_f * bw
                    };
                    let band = Bounds::new(
                        point(bounds.left() + px(x0), bounds.top()),
                        size(px((x1 - x0).max(2.)), px(bh)),
                    );
                    window.paint_quad(fill(band, rva_blue.opacity(0.11)));
                    let mut edges = PathBuilder::stroke(px(1.));
                    edges.move_to(point(bounds.left() + px(x0), bounds.top()));
                    edges.line_to(point(bounds.left() + px(x0), bounds.bottom()));
                    edges.move_to(point(bounds.left() + px(x1), bounds.top()));
                    edges.line_to(point(bounds.left() + px(x1), bounds.bottom()));
                    if let Ok(path) = edges.build() {
                        window.paint_path(path, rva_blue.opacity(0.55));
                    }
                }

                // Dashed previous-year series
                paint_dashed_polyline(window, &prev_norm, pt_at, prev_stroke, 1.25, 3.0, 3.0);

                // Soft gradient area fill under current series
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
                            180.,
                            linear_color_stop(rva_blue.opacity(0.20), 0.),
                            linear_color_stop(rva_blue.opacity(0.0), 1.),
                        ),
                    );
                }

                // Solid current-year curve
                let mut line = PathBuilder::stroke(px(1.65));
                for (i, &ny) in cur_norm.iter().enumerate().take(vis_n) {
                    let pt = pt_at(i, ny);
                    if i == 0 {
                        line.move_to(pt);
                    } else {
                        line.line_to(pt);
                    }
                }
                if let Ok(path) = line.build() {
                    window.paint_path(path, rva_blue);
                }

                // Spring-gliding active vertical hairline + point markers
                if show_marker && fallback_idx < vis_n {
                    let cur_pt = point(
                        bounds.left() + px(marker_tx * bw),
                        bounds.top() + px((1. - marker_cy) * bh),
                    );
                    let prev_pt = point(
                        bounds.left() + px(marker_tx * bw),
                        bounds.top() + px((1. - marker_py) * bh),
                    );
                    let mut g_pb = PathBuilder::stroke(px(1.));
                    g_pb.move_to(point(cur_pt.x, bounds.top()));
                    g_pb.line_to(point(cur_pt.x, bounds.bottom()));
                    if let Ok(path) = g_pb.build() {
                        window.paint_path(path, guide_stroke);
                    }
                    if !prev_norm.is_empty() {
                        paint_ringed_dot(window, prev_pt, 3.0, 2.0, prev_stroke, card_bg);
                    }
                    paint_ringed_dot(window, cur_pt, 4.0, 2.0, rva_blue, card_bg);
                }
            },
        )
        .size_full();

        // Presence-animated tooltip overlay on active point with spring x-glide
        let tooltip_el = (n > 0).then(|| {
            let ai = fallback_idx;
            let on_left = marker_tx > 0.52;
            let cur_lbl = format_k_dollars(disp_cur[ai]);
            let prev_lbl = disp_prev
                .get(ai)
                .copied()
                .map(format_k_dollars)
                .unwrap_or_default();
            let tip_box = div()
                .min_w(px(148.))
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
                        .child(disp_labels[ai].clone()),
                )
                .child(div().h(px(1.)).bg(p.line))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(14.))
                        .text_size(px(12.))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .text_color(p.text)
                                .child(div().size(px(7.)).rounded(px(2.)).bg(rva_blue))
                                .child(tr(cx, "This year", "今年")),
                        )
                        .child(
                            div()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.ink)
                                .child(cur_lbl),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(14.))
                        .text_size(px(12.))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .text_color(p.text)
                                .child(div().w(px(7.)).h(px(2.)).rounded(px(1.)).bg(prev_stroke))
                                .child(tr(cx, "Last year", "去年")),
                        )
                        .child(
                            div()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.ink)
                                .child(prev_lbl),
                        ),
                )
                .into_any_element();
            pro_presence(
                pro_eid(&self.id, format!("tr-tip-pres-{turn_id}")),
                active_idx.is_some(),
                0.16,
                0.12,
                move |amt| {
                    let e = pro_ease(amt);
                    div()
                        .absolute()
                        .top(px(-2. + 4. * (1. - e)))
                        .when(on_left, |d| d.right(relative(1.0 - marker_tx)).mr(px(12.)))
                        .when(!on_left, |d| d.left(relative(marker_tx)).ml(px(12.)))
                        .opacity(e)
                        .child(tip_box)
                        .into_any_element()
                },
            )
        });

        // Presence-animated inline ask field when a week range is picked
        let is_ask_open = active_pick.as_ref().is_some_and(|pk| pk.ask_open);
        let ask_focus = if is_ask_open {
            active_pick
                .as_ref()
                .and_then(|pk| self.current_focus_for_pick(pk))
        } else {
            self.last_pick_focus
                .as_ref()
                .filter(|(tid, bar, _)| tid == turn_id && !*bar)
                .map(|(_, _, f)| f.clone())
        };
        let ask_popover = if let (Some(focus), Some(ed)) = (ask_focus, self.pick_editor.clone()) {
            let box_el = self
                .render_pick_ask_box(focus, ed, rva_blue, rva_edge, cx)
                .into_any_element();
            Some(pro_presence(
                pro_eid(&self.id, format!("tr-ask-pres-{turn_id}")),
                is_ask_open,
                0.18,
                0.13,
                move |amt| {
                    let e = pro_ease(amt);
                    div()
                        .w_full()
                        .max_h(px(64. * e))
                        .overflow_hidden()
                        .opacity(e)
                        .relative()
                        .top(px(-4. * (1. - e)))
                        .child(box_el)
                        .into_any_element()
                },
            ))
        } else {
            None
        };

        // Invisible interactive columns for hover & click-drag stretching
        let hit_strip = div().absolute().inset_0().flex().children((0..n).map(|pi| {
            let tid_h = turn_id.clone();
            let tid_d = turn_id.clone();
            div()
                .id(pro_eid(&self.id, format!("tpt-{turn_id}-{pi}")))
                .flex_1()
                .h_full()
                .cursor_crosshair()
                .on_hover(cx.listener(move |this, over: &bool, _, cx| {
                    if *over {
                        this.hovered_point = Some((tid_h.clone(), pi));
                        this.keyboard_point = pi;
                        if this.dragging_pick
                            && let Some(pk) = &mut this.active_pick
                            && pk.turn_id == tid_h
                        {
                            pk.from = pk.anchor.min(pi);
                            pk.to = pk.anchor.max(pi);
                        }
                        cx.notify();
                    } else if !this.dragging_pick
                        && this
                            .hovered_point
                            .as_ref()
                            .is_some_and(|(t, idx)| t == &tid_h && *idx == pi)
                    {
                        this.hovered_point = None;
                        cx.notify();
                    }
                }))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _, _, cx| {
                        if this.period == RevenueChartPeriod::Monthly {
                            return;
                        }
                        this.dragging_pick = true;
                        this.active_pick = Some(ActivePick {
                            turn_id: tid_d.clone(),
                            anchor: pi,
                            from: pi,
                            to: pi,
                            is_bar: false,
                            ask_open: false,
                        });
                        cx.notify();
                    }),
                )
        }));

        let first_lbl = disp_labels.first().cloned().unwrap_or_default();
        let mid_lbl = disp_labels.get(n / 2).cloned().unwrap_or_default();
        let last_lbl = disp_labels.last().cloned().unwrap_or_default();

        let period_key: SharedString = if self.period == RevenueChartPeriod::Monthly {
            "monthly".into()
        } else {
            "weekly".into()
        };

        let period_pill = (!narrow && current.len() >= 9).then(|| {
            let per_sensor = pro_selection_sensor();
            div()
                .relative()
                .h(px(22.))
                .p(px(2.))
                .rounded_full()
                .bg(p.chip)
                .flex()
                .items_center()
                .gap(px(2.))
                .child(pro_pill_indicator(
                    pro_eid(&self.id, format!("per-pill-{turn_id}")),
                    period_key.clone(),
                    per_sensor.clone(),
                    999.,
                    p.card,
                    Some(p.line),
                    !p.dark,
                ))
                .children(
                    [
                        (RevenueChartPeriod::Weekly, tr(cx, "Weekly", "周")),
                        (RevenueChartPeriod::Monthly, tr(cx, "Monthly", "月")),
                    ]
                    .into_iter()
                    .map(|(per, lbl)| {
                        let on = self.period == per;
                        let sensor = per_sensor.clone();
                        activate(
                            div()
                                .id(pro_eid(&self.id, format!("per-{turn_id}-{lbl}")))
                                .role(Role::Tab)
                                .tab_index(0)
                                .relative()
                                .h(px(18.))
                                .px(px(7.))
                                .rounded_full()
                                .when(!on, |d| d.hover(move |s| s.text_color(p.ink)))
                                .flex()
                                .items_center()
                                .text_size(px(11.))
                                .font_weight(if on {
                                    FontWeight::MEDIUM
                                } else {
                                    FontWeight::NORMAL
                                })
                                .text_color(if on { p.ink } else { p.muted })
                                .cursor_pointer()
                                .when(on, |d| d.child(pro_mark_selected(&sensor)))
                                .child(lbl),
                            cx,
                            move |this, _, cx| {
                                this.period = per;
                                this.active_pick = None;
                                cx.notify();
                            },
                        )
                    }),
                )
        });

        let plot_row = div()
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
                    .child(format_k_dollars(y_top))
                    .child(format_k_dollars(y_mid))
                    .child(format_k_dollars(y_bot)),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(
                        div()
                            .relative()
                            .w_full()
                            .h(px(152.))
                            .child(plot_canvas)
                            .when_some(tooltip_el, |d, tip| d.child(tip))
                            .child(hit_strip),
                    )
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
            );

        div()
            .ml(px(out_offset))
            .px(px(if narrow { 10. } else { 14. }))
            .pt(px(if narrow { 12. } else { 14. }))
            .pb(px(10.))
            .rounded(px(14.))
            .bg(p.card)
            .border_1()
            .border_color(rva_edge)
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
                                    .child(div().w(px(10.)).h(px(3.)).rounded(px(1.5)).bg(rva_blue))
                                    .child(tr(cx, "This year", "今年")),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(2.))
                                            .child(
                                                div()
                                                    .w(px(4.))
                                                    .h(px(2.5))
                                                    .rounded(px(1.))
                                                    .bg(prev_stroke),
                                            )
                                            .child(
                                                div()
                                                    .w(px(4.))
                                                    .h(px(2.5))
                                                    .rounded(px(1.))
                                                    .bg(prev_stroke),
                                            ),
                                    )
                                    .child(tr(cx, "Last year", "去年")),
                            )
                            .when_some(period_pill, |d, pill| d.child(pill)),
                    ),
            )
            .when_some(ask_popover, |d, pop| d.child(pop))
            .child(pro_view_transition(
                pro_eid(&self.id, format!("tr-mode-vtx-{turn_id}")),
                if is_monthly { "monthly" } else { "weekly" },
                plot_row,
            ))
            .child(div().text_size(px(12.)).text_color(p.faint).child(tr(
                cx,
                "Drag across the chart to ask about those weeks",
                "在图表上横向拖动即可针对所选周提问",
            )))
    }

    #[allow(clippy::too_many_arguments)]
    fn render_forecast_figure(
        &self,
        turn_id: &SharedString,
        caption: Option<SharedString>,
        labels: Vec<SharedString>,
        actual: Vec<Option<f64>>,
        forecast: Vec<Option<f64>>,
        target: Option<f64>,
        out_offset: f32,
        narrow: bool,
        rva_blue: Hsla,
        rva_edge: Hsla,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let p = ProPalette::get(cx);
        let n = labels.len();
        let active_pick = self
            .active_pick
            .as_ref()
            .filter(|pk| &pk.turn_id == turn_id && !pk.is_bar)
            .cloned();

        let active_idx = if active_pick.is_some() {
            None
        } else {
            self.hovered_point
                .as_ref()
                .filter(|(tid, idx)| tid == turn_id && *idx < n)
                .map(|(_, idx)| *idx)
                .or_else(|| n.checked_sub(1))
        };

        let violet_c = p.purple;
        let target_c = p.amber.opacity(0.65);
        let grid_stroke = if p.dark {
            hsla(0., 0., 1., 0.08)
        } else {
            hsla(0., 0., 0., 0.06)
        };
        let card_bg = p.card;
        let min_v = 60.0_f32;
        let max_v = 160.0_f32;
        let span = max_v - min_v;
        let denom = (n.saturating_sub(1)).max(1) as f32;

        let mut act_pts: Vec<(usize, f32)> = Vec::new();
        for (i, v) in actual.iter().enumerate() {
            if let Some(val) = v {
                let s_val = pro_spring_f32(
                    pro_eid(&self.id, format!("fc-av-{turn_id}-{i}")),
                    *val as f32,
                    0.28,
                    w,
                    cx,
                );
                act_pts.push((i, s_val));
            }
        }
        let mut fc_pts: Vec<(usize, f32)> = Vec::new();
        for (i, v) in forecast.iter().enumerate() {
            if let Some(val) = v {
                let s_val = pro_spring_f32(
                    pro_eid(&self.id, format!("fc-fv-{turn_id}-{i}")),
                    *val as f32,
                    0.28,
                    w,
                    cx,
                );
                fc_pts.push((i, s_val));
            }
        }

        let marker_ai = active_idx.unwrap_or_else(|| n.saturating_sub(1));
        let marker_raw_val = actual
            .get(marker_ai)
            .and_then(|v| *v)
            .or_else(|| forecast.get(marker_ai).and_then(|v| *v))
            .unwrap_or(110.0) as f32;
        let marker_is_actual = actual.get(marker_ai).and_then(|v| *v).is_some();
        let marker_tx = pro_spring_f32(
            pro_eid(&self.id, format!("fc-mtx-{turn_id}")),
            marker_ai as f32 / denom,
            0.18,
            w,
            cx,
        )
        .clamp(0., 1.);
        let marker_val = pro_spring_f32(
            pro_eid(&self.id, format!("fc-mval-{turn_id}")),
            marker_raw_val,
            0.18,
            w,
            cx,
        );

        let pick_range_spring = active_pick.as_ref().map(|pk| {
            let lo_t = pk.from.min(pk.to) as f32 / denom;
            let hi_t = pk.from.max(pk.to) as f32 / denom;
            let s_lo = pro_spring_f32(
                pro_eid(&self.id, format!("fc-blo-{turn_id}")),
                lo_t,
                0.16,
                w,
                cx,
            )
            .clamp(0., 1.);
            let s_hi = pro_spring_f32(
                pro_eid(&self.id, format!("fc-bhi-{turn_id}")),
                hi_t,
                0.16,
                w,
                cx,
            )
            .clamp(0., 1.);
            (s_lo, s_hi, pk.from == pk.to)
        });
        let show_marker = active_idx.is_some();

        let plot_canvas = canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let bw = f32::from(bounds.size.width);
                let bh = f32::from(bounds.size.height);
                if bw <= 8. || bh <= 8. || n < 2 {
                    return;
                }
                let pt_at = |i: usize, v: f32| -> Point<Pixels> {
                    let tx = i as f32 / (n - 1) as f32;
                    let ty = 1. - ((v - min_v) / span).clamp(0., 1.);
                    point(bounds.left() + px(tx * bw), bounds.top() + px(ty * bh))
                };

                for frac in [0.0_f32, 0.5, 1.0] {
                    let gy = bounds.top() + px(frac * bh);
                    let mut g_pb = PathBuilder::stroke(px(1.));
                    g_pb.move_to(point(bounds.left(), gy));
                    g_pb.line_to(point(bounds.right(), gy));
                    if let Ok(path) = g_pb.build() {
                        window.paint_path(path, grid_stroke);
                    }
                }

                if let Some((lo_f, hi_f, single)) = pick_range_spring {
                    let x0 = if single {
                        (lo_f * bw - 12.).max(0.)
                    } else {
                        lo_f * bw
                    };
                    let x1 = if single {
                        (hi_f * bw + 12.).min(bw)
                    } else {
                        hi_f * bw
                    };
                    let band = Bounds::new(
                        point(bounds.left() + px(x0), bounds.top()),
                        size(px((x1 - x0).max(2.)), px(bh)),
                    );
                    window.paint_quad(fill(band, violet_c.opacity(0.11)));
                }

                if let Some(tv) = target {
                    let ty = pt_at(0, tv as f32).y;
                    let mut x = 0.0_f32;
                    let mut t_pb = PathBuilder::stroke(px(1.1));
                    while x < bw {
                        let x2 = (x + 4.).min(bw);
                        t_pb.move_to(point(bounds.left() + px(x), ty));
                        t_pb.line_to(point(bounds.left() + px(x2), ty));
                        x += 8.;
                    }
                    if let Ok(path) = t_pb.build() {
                        window.paint_path(path, target_c);
                    }
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
                        window.paint_path(path, rva_blue);
                    }
                }

                if fc_pts.len() >= 2 {
                    let mut band = PathBuilder::fill();
                    for (k, &(idx, val)) in fc_pts.iter().enumerate() {
                        let spread = 4.0 + k as f32 * 1.6;
                        let pt = pt_at(idx, val + spread);
                        if k == 0 {
                            band.move_to(pt);
                        } else {
                            band.line_to(pt);
                        }
                    }
                    for (k, &(idx, val)) in fc_pts.iter().enumerate().rev() {
                        let spread = 4.0 + k as f32 * 1.6;
                        band.line_to(pt_at(idx, val - spread));
                    }
                    band.close();
                    if let Ok(path) = band.build() {
                        window.paint_path(path, violet_c.opacity(0.12));
                    }

                    for win in fc_pts.windows(2) {
                        let p0 = pt_at(win[0].0, win[0].1);
                        let p1 = pt_at(win[1].0, win[1].1);
                        paint_dashed_segment(window, p0, p1, violet_c, 1.6, 3.5, 3.0);
                    }
                }

                if show_marker {
                    let m_ty = 1. - ((marker_val - min_v) / span).clamp(0., 1.);
                    let m_pt = point(
                        bounds.left() + px(marker_tx * bw),
                        bounds.top() + px(m_ty * bh),
                    );
                    paint_ringed_dot(
                        window,
                        m_pt,
                        4.0,
                        2.0,
                        if marker_is_actual { rva_blue } else { violet_c },
                        card_bg,
                    );
                }
            },
        )
        .size_full();

        let is_ask_open = active_pick.as_ref().is_some_and(|pk| pk.ask_open);
        let ask_focus = if is_ask_open {
            active_pick
                .as_ref()
                .and_then(|pk| self.current_focus_for_pick(pk))
        } else {
            self.last_pick_focus
                .as_ref()
                .filter(|(tid, bar, _)| tid == turn_id && !*bar)
                .map(|(_, _, f)| f.clone())
        };
        let ask_popover = if let (Some(focus), Some(ed)) = (ask_focus, self.pick_editor.clone()) {
            let box_el = self
                .render_pick_ask_box(focus, ed, rva_blue, rva_edge, cx)
                .into_any_element();
            Some(pro_presence(
                pro_eid(&self.id, format!("fc-ask-pres-{turn_id}")),
                is_ask_open,
                0.18,
                0.13,
                move |amt| {
                    let e = pro_ease(amt);
                    div()
                        .w_full()
                        .max_h(px(64. * e))
                        .overflow_hidden()
                        .opacity(e)
                        .relative()
                        .top(px(-4. * (1. - e)))
                        .child(box_el)
                        .into_any_element()
                },
            ))
        } else {
            None
        };

        let hit_strip = div().absolute().inset_0().flex().children((0..n).map(|pi| {
            let tid_h = turn_id.clone();
            let tid_d = turn_id.clone();
            div()
                .id(pro_eid(&self.id, format!("fcpt-{turn_id}-{pi}")))
                .flex_1()
                .h_full()
                .cursor_crosshair()
                .on_hover(cx.listener(move |this, over: &bool, _, cx| {
                    if *over {
                        this.hovered_point = Some((tid_h.clone(), pi));
                        this.keyboard_point = pi;
                        if this.dragging_pick
                            && let Some(pk) = &mut this.active_pick
                            && pk.turn_id == tid_h
                        {
                            pk.from = pk.anchor.min(pi);
                            pk.to = pk.anchor.max(pi);
                        }
                        cx.notify();
                    }
                }))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _, _, cx| {
                        this.dragging_pick = true;
                        this.active_pick = Some(ActivePick {
                            turn_id: tid_d.clone(),
                            anchor: pi,
                            from: pi,
                            to: pi,
                            is_bar: false,
                            ask_open: false,
                        });
                        cx.notify();
                    }),
                )
        }));

        let readout = active_idx.and_then(|ai| {
            let lbl = labels.get(ai)?.clone();
            let val = actual
                .get(ai)
                .and_then(|v| *v)
                .or_else(|| forecast.get(ai).and_then(|v| *v))?;
            let kind_str = if actual.get(ai).and_then(|v| *v).is_some() {
                tr(cx, "Actual", "实际")
            } else {
                tr(cx, "Forecast", "预测")
            };
            Some((format!("{lbl} · {kind_str} {}", format_k_dollars(val)), val))
        });

        let pace_ratio = if let Some(tv) = target
            && tv > 0.0
        {
            let cur_v = readout.as_ref().map(|(_, v)| *v).unwrap_or(121.0);
            (cur_v / (tv * 1.2)).clamp(0.08, 1.0) as f32
        } else {
            0.88
        };

        div()
            .ml(px(out_offset))
            .px(px(if narrow { 10. } else { 14. }))
            .pt(px(14.))
            .pb(px(10.))
            .rounded(px(14.))
            .bg(p.card)
            .border_1()
            .border_color(rva_edge)
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
                            .when_some(readout.map(|(r, _)| r), |d, r| {
                                d.child(
                                    div()
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.ink)
                                        .child(r),
                                )
                            })
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(5.))
                                    .child(div().w(px(10.)).h(px(3.)).rounded(px(1.5)).bg(rva_blue))
                                    .child(tr(cx, "Actual", "实际")),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(5.))
                                    .child(div().w(px(10.)).h(px(3.)).rounded(px(1.5)).bg(violet_c))
                                    .child(tr(cx, "Forecast", "预测")),
                            ),
                    ),
            )
            .when_some(ask_popover, |d, pop| d.child(pop))
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
                            .child(
                                div()
                                    .relative()
                                    .w_full()
                                    .h(px(152.))
                                    .child(plot_canvas)
                                    .child(hit_strip),
                            )
                            .child(
                                div()
                                    .w_full()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .text_size(px(11.))
                                    .text_color(p.muted)
                                    .child(labels.first().cloned().unwrap_or_default())
                                    .child(labels.get(n / 2).cloned().unwrap_or_default())
                                    .child(labels.last().cloned().unwrap_or_default()),
                            ),
                    ),
            )
            .child(
                div()
                    .pt(px(4.))
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .text_size(px(11.5))
                    .text_color(p.muted)
                    .child(tr(cx, "Plan target pace", "目标达成进度"))
                    .child(div().w(px(88.)).flex_none().child(pro_spring_bar(
                        pro_eid(&self.id, format!("fc-pace-bar-{turn_id}")),
                        pace_ratio,
                        4.,
                        2.,
                        p.chip,
                        violet_c,
                    )))
                    .child(div().text_color(p.faint).child(tr(
                        cx,
                        "· Drag weeks to ask",
                        "· 拖动周区间可追问",
                    ))),
            )
    }

    #[allow(clippy::too_many_arguments)]
    fn render_split_figure(
        &self,
        turn_id: &SharedString,
        title: SharedString,
        previous_total: Option<f64>,
        since_label: Option<SharedString>,
        groups: Vec<RevenueSplitGroup>,
        alt_groups: Vec<RevenueSplitGroup>,
        out_offset: f32,
        narrow: bool,
        rva_blue: Hsla,
        rva_edge: Hsla,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let p = ProPalette::get(cx);
        let has_alt = !alt_groups.is_empty();
        let using_plans = self.split_grouping == RevenueSplitGrouping::ByPlan && has_alt;
        let active_groups = if using_plans { alt_groups } else { groups };

        let grand_total: f64 = active_groups.iter().map(RevenueSplitGroup::total).sum();
        let max_col = active_groups
            .iter()
            .map(RevenueSplitGroup::total)
            .fold(1.0_f64, f64::max);
        let y_max = ((max_col / 100_000.0).ceil() * 100_000.0).max(100_000.0);

        let change_badge = previous_total.and_then(|prev| {
            if prev <= 0.0 {
                return None;
            }
            let pct = ((grand_total - prev) / prev * 100.0).round() as i32;
            let up = pct >= 0;
            let text = match &since_label {
                Some(s) => format!("{}{pct}% since {s}", if up { "+" } else { "" }),
                None => format!("{}{pct}%", if up { "+" } else { "" }),
            };
            let (bg, fg) = if up {
                (p.green_soft, p.green_ink)
            } else {
                (p.red_soft, p.red_ink)
            };
            Some(
                div()
                    .h(px(22.))
                    .px(px(6.))
                    .rounded(px(6.))
                    .bg(bg)
                    .flex()
                    .items_center()
                    .text_size(px(12.))
                    .text_color(fg)
                    .child(text),
            )
        });

        let bar_pick = self
            .active_pick
            .as_ref()
            .filter(|pk| &pk.turn_id == turn_id && pk.is_bar)
            .cloned();
        let hovered_bar = self
            .hovered_point
            .as_ref()
            .filter(|(tid, idx)| tid == turn_id && *idx < active_groups.len())
            .map(|(_, idx)| *idx);

        let is_ask_open = bar_pick.as_ref().is_some_and(|pk| pk.ask_open);
        let ask_focus = if is_ask_open {
            bar_pick
                .as_ref()
                .and_then(|pk| self.current_focus_for_pick(pk))
        } else {
            self.last_pick_focus
                .as_ref()
                .filter(|(tid, bar, _)| tid == turn_id && *bar)
                .map(|(_, _, f)| f.clone())
        };
        let ask_popover = if let (Some(focus), Some(ed)) = (ask_focus, self.pick_editor.clone()) {
            let box_el = self
                .render_pick_ask_box(focus, ed, rva_blue, rva_edge, cx)
                .into_any_element();
            Some(pro_presence(
                pro_eid(&self.id, format!("sbar-ask-pres-{turn_id}")),
                is_ask_open,
                0.18,
                0.13,
                move |amt| {
                    let e = pro_ease(amt);
                    div()
                        .w_full()
                        .mb(px(10. * e))
                        .max_h(px(64. * e))
                        .overflow_hidden()
                        .opacity(e)
                        .relative()
                        .top(px(-4. * (1. - e)))
                        .child(box_el)
                        .into_any_element()
                },
            ))
        } else {
            None
        };

        let palette_tones = [rva_blue, p.teal, p.purple, p.amber];

        // Aggregate segment breakdown rows below bars
        let mut part_totals: Vec<(SharedString, f64)> = Vec::new();
        for g in &active_groups {
            for seg in &g.parts {
                if let Some(entry) = part_totals.iter_mut().find(|(n, _)| n == &seg.name) {
                    entry.1 += seg.value;
                } else {
                    part_totals.push((seg.name.clone(), seg.value));
                }
            }
        }

        let mut bar_els = Vec::with_capacity(active_groups.len());
        for (gi, grp) in active_groups.iter().enumerate() {
            let tot = grp.total();
            let target_bar_h = ((tot / y_max) * 152.0).clamp(6.0, 156.0) as f32;
            let bar_h = pro_spring_f32(
                pro_eid(&self.id, format!("sbar-h-{turn_id}-{gi}")),
                target_bar_h,
                0.28,
                w,
                cx,
            )
            .clamp(4.0, 160.0);

            let base_c = palette_tones[grp.color_idx % palette_tones.len()];
            let picked = bar_pick.as_ref().is_some_and(|pk| pk.from == gi);
            let dimmed = hovered_bar.is_some_and(|hi| hi != gi) && !picked;
            let anim_op = pro_spring_f32(
                pro_eid(&self.id, format!("sbar-op-{turn_id}-{gi}")),
                if dimmed { 0.38 } else { 1.0 },
                0.18,
                w,
                cx,
            )
            .clamp(0.2, 1.0);
            let anim_lift = pro_spring_f32(
                pro_eid(&self.id, format!("sbar-lift-{turn_id}-{gi}")),
                if picked || hovered_bar == Some(gi) {
                    1.0
                } else {
                    0.0
                },
                0.18,
                w,
                cx,
            )
            .clamp(0.0, 1.2);

            let mut seg_els = Vec::with_capacity(grp.parts.len());
            for (si, seg) in grp.parts.iter().enumerate().rev() {
                let target_sh = if tot > 0.0 {
                    (seg.value / tot * bar_h as f64).max(3.0) as f32
                } else {
                    3.0
                };
                let sh = pro_spring_f32(
                    pro_eid(&self.id, format!("sbar-seg-{turn_id}-{gi}-{si}")),
                    target_sh,
                    0.28,
                    w,
                    cx,
                )
                .max(2.0);
                let shade = match si {
                    0 => 1.0,
                    1 => 0.62,
                    _ => 0.36,
                };
                seg_els.push(div().w_full().h(px(sh)).bg(base_c.opacity(shade)));
            }

            let tid_c = turn_id.clone();
            let tid_h = turn_id.clone();
            bar_els.push(activate(
                div()
                    .id(pro_eid(&self.id, format!("sbar-{turn_id}-{gi}")))
                    .role(Role::Button)
                    .tab_index(0)
                    .relative()
                    .top(px(-2.0 * anim_lift))
                    .flex_1()
                    .h_full()
                    .rounded(px(8.))
                    .when(picked, |d| d.border_1().border_color(rva_blue))
                    .opacity(anim_op)
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_end()
                    .gap(px(6.))
                    .cursor_pointer()
                    .on_hover(cx.listener(move |this, over: &bool, _, cx| {
                        if *over {
                            this.hovered_point = Some((tid_h.clone(), gi));
                        } else if this
                            .hovered_point
                            .as_ref()
                            .is_some_and(|(t, i)| t == &tid_h && *i == gi)
                        {
                            this.hovered_point = None;
                        }
                        cx.notify();
                    }))
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(if picked || hovered_bar == Some(gi) {
                                p.strong
                            } else {
                                p.muted
                            })
                            .child(format_dollars_axis(tot)),
                    )
                    .child(
                        div()
                            .w(px(if narrow { 40. } else { 56. }))
                            .h(px(bar_h))
                            .rounded_t(px(5.))
                            .overflow_hidden()
                            .flex()
                            .flex_col()
                            .gap(px(2.))
                            .children(seg_els),
                    ),
                cx,
                move |this, w, cx| {
                    let pk = ActivePick {
                        turn_id: tid_c.clone(),
                        anchor: gi,
                        from: gi,
                        to: gi,
                        is_bar: true,
                        ask_open: true,
                    };
                    if let Some(f) = this.current_focus_for_pick(&pk) {
                        cx.emit(AiChatRevenueEvent::SelectRange(f));
                    }
                    this.active_pick = Some(pk);
                    if let Some(ed) = &this.pick_editor {
                        ed.update(cx, |e, cx| e.set_value("", w, cx));
                        {
                            let fh = ed.read(cx).focus.clone();
                            fh.focus(w, cx);
                        }
                    }
                    cx.notify();
                },
            ));
        }

        let grp_key: SharedString = if using_plans {
            "plans".into()
        } else {
            "regions".into()
        };

        let split_body = div()
            .w_full()
            .flex()
            .flex_col()
            .child(
                div()
                    .w_full()
                    .flex()
                    .gap(px(10.))
                    .child(
                        div()
                            .h(px(184.))
                            .w(px(38.))
                            .flex_none()
                            .flex()
                            .flex_col()
                            .justify_between()
                            .items_end()
                            .text_size(px(11.5))
                            .text_color(p.muted)
                            .child(format_dollars_axis(y_max))
                            .child(format_dollars_axis(y_max * 0.5))
                            .child("$0"),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .relative()
                                    .h(px(184.))
                                    .w_full()
                                    .border_t_1()
                                    .border_b_1()
                                    .border_color(p.line_soft)
                                    .flex()
                                    .items_end()
                                    .gap(px(12.))
                                    .px(px(8.))
                                    .children(bar_els),
                            )
                            .child(div().mt(px(10.)).px(px(8.)).flex().gap(px(12.)).children(
                                active_groups.iter().map(|grp| {
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .text_center()
                                        .text_size(px(12.))
                                        .text_color(p.text)
                                        .truncate()
                                        .child(grp.name.clone())
                                }),
                            )),
                    ),
            )
            .when(!part_totals.is_empty(), |d| {
                d.child(
                    div()
                        .mt(px(14.))
                        .pt(px(12.))
                        .border_t_1()
                        .border_color(p.line)
                        .flex()
                        .flex_wrap()
                        .gap(px(12.))
                        .children(part_totals.into_iter().enumerate().map(
                            |(pi, (pname, pval))| {
                                let ratio = if grand_total > 0.0 {
                                    (pval / grand_total).clamp(0.04, 1.0) as f32
                                } else {
                                    0.04
                                };
                                let pct = if grand_total > 0.0 {
                                    (pval / grand_total * 100.0).round() as i32
                                } else {
                                    0
                                };
                                let c = palette_tones[pi % palette_tones.len()];
                                div()
                                    .min_w(px(140.))
                                    .flex_1()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.))
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
                                    .child(div().w(px(48.)).flex_none().child(pro_spring_bar(
                                        pro_eid(&self.id, format!("spart-bar-{turn_id}-{pi}")),
                                        ratio,
                                        4.,
                                        2.,
                                        p.chip,
                                        c,
                                    )))
                                    .child(
                                        div()
                                            .w(px(32.))
                                            .text_right()
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.ink)
                                            .child(format!("{pct}%")),
                                    )
                            },
                        )),
                )
            });

        div()
            .ml(px(out_offset))
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(
                div()
                    .px(px(if narrow { 14. } else { 20. }))
                    .pt(px(18.))
                    .pb(px(14.))
                    .rounded(px(14.))
                    .bg(p.card)
                    .border_1()
                    .border_color(rva_edge)
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .mb(px(26.))
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .justify_between()
                            .gap(px(10.))
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
                                            .child(format_dollars_comma(grand_total)),
                                    )
                                    .when_some(change_badge, |d, b| d.child(b)),
                            )
                            .when(has_alt, |d| {
                                let grp_sensor = pro_selection_sensor();
                                d.child(
                                    div()
                                        .relative()
                                        .p(px(2.))
                                        .rounded(px(8.))
                                        .bg(p.chip)
                                        .flex()
                                        .items_center()
                                        .gap(px(2.))
                                        .child(pro_pill_indicator(
                                            pro_eid(&self.id, format!("sgrp-pill-{turn_id}")),
                                            grp_key.clone(),
                                            grp_sensor.clone(),
                                            6.,
                                            p.card,
                                            Some(p.line),
                                            !p.dark,
                                        ))
                                        .children(
                                            [
                                                (
                                                    RevenueSplitGrouping::ByRegion,
                                                    tr(cx, "Regions", "按地区"),
                                                ),
                                                (
                                                    RevenueSplitGrouping::ByPlan,
                                                    tr(cx, "Plans", "按方案"),
                                                ),
                                            ]
                                            .into_iter()
                                            .map(
                                                |(grp, lbl)| {
                                                    let on = self.split_grouping == grp;
                                                    let sensor = grp_sensor.clone();
                                                    activate(
                                                        div()
                                                            .id(pro_eid(
                                                                &self.id,
                                                                format!("sgrp-{turn_id}-{lbl}"),
                                                            ))
                                                            .role(Role::Tab)
                                                            .tab_index(0)
                                                            .relative()
                                                            .h(px(24.))
                                                            .px(px(10.))
                                                            .rounded(px(6.))
                                                            .when(!on, |d| {
                                                                d.hover(move |s| {
                                                                    s.text_color(p.ink)
                                                                })
                                                            })
                                                            .flex()
                                                            .items_center()
                                                            .text_size(px(12.5))
                                                            .font_weight(if on {
                                                                FontWeight::MEDIUM
                                                            } else {
                                                                FontWeight::NORMAL
                                                            })
                                                            .text_color(if on {
                                                                p.strong
                                                            } else {
                                                                p.text
                                                            })
                                                            .cursor_pointer()
                                                            .when(on, |d| {
                                                                d.child(pro_mark_selected(&sensor))
                                                            })
                                                            .child(lbl),
                                                        cx,
                                                        move |this, _, cx| {
                                                            this.split_grouping = grp;
                                                            this.active_pick = None;
                                                            cx.notify();
                                                        },
                                                    )
                                                },
                                            ),
                                        ),
                                )
                            }),
                    )
                    .when_some(ask_popover, |d, pop| d.child(pop))
                    .child(pro_view_transition(
                        pro_eid(&self.id, format!("sgrp-vtx-{turn_id}")),
                        grp_key,
                        split_body,
                    )),
            )
            .child(
                div()
                    .pl(px(2.))
                    .text_size(px(12.))
                    .text_color(p.faint)
                    .child(tr(
                        cx,
                        "Click a bar to ask about it",
                        "点击任一柱条即可追问",
                    )),
            )
    }

    fn render_pick_ask_box(
        &self,
        focus: RevenueChartFocus,
        editor: Entity<Editor>,
        rva_blue: Hsla,
        rva_edge: Hsla,
        cx: &Context<Self>,
    ) -> Div {
        let p = ProPalette::get(cx);
        let focus_submit = focus.clone();
        div()
            .px(px(12.))
            .py(px(8.))
            .rounded(px(11.))
            .bg(if p.dark {
                rgb(0x232327).into()
            } else {
                p.subtle
            })
            .border_1()
            .border_color(rva_edge)
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(10.))
            .child(
                div()
                    .px(px(8.))
                    .py(px(2.))
                    .rounded(px(6.))
                    .bg(rva_blue.opacity(0.12))
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(rva_blue)
                    .child(format!("{} · {}", focus.label, focus.detail)),
            )
            .child(
                div()
                    .flex_1()
                    .min_w(px(160.))
                    .h(px(22.))
                    .child(editor.clone()),
            )
            .child(pro_btn(
                pro_eid(&self.id, "pick-ask-submit"),
                tr(cx, "Ask", "提问"),
                None,
                ProBtnStyle::Primary,
                true,
                cx,
                move |this, w, cx| {
                    let val = editor.read(cx).value().to_string();
                    let q = if val.trim().is_empty() {
                        format!("What drove {}?", focus_submit.label)
                    } else {
                        val
                    };
                    if this.ask(q, Some(focus_submit.clone()), cx) {
                        editor.update(cx, |e, cx| e.set_value("", w, cx));
                    }
                },
            ))
            .child(pro_icon_btn(
                pro_eid(&self.id, "pick-ask-close"),
                tr(cx, "Cancel selection", "取消选择"),
                ProIcon::Close,
                24.,
                false,
                cx,
                |this, _, cx| {
                    this.active_pick = None;
                    cx.notify();
                },
            ))
    }

    #[allow(clippy::too_many_arguments)]
    fn action_icon_btn(
        id: ElementId,
        aria: &'static str,
        svg_markup: &'static str,
        active: bool,
        p: &ProPalette,
        w: &mut Window,
        cx: &mut Context<Self>,
        on_press: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + Clone + 'static,
    ) -> Stateful<Div> {
        let act_t =
            pro_spring_f32(pro_eid(&id, "act-t"), f32::from(active), 0.18, w, cx).clamp(0., 1.);
        let fg = pro_mix_color(p.muted, p.blue, act_t);
        let bg = pro_mix_color(transparent_black(), p.accent_soft, act_t);
        let hover_bg = p.hover;
        let hover_fg = p.ink;
        let focus_c = p.focus;
        let el = div()
            .id(id)
            .role(Role::Button)
            .aria_label(aria)
            .tab_index(0)
            .size(px(28.))
            .rounded(px(7.))
            .bg(bg)
            .hover(move |s| s.bg(hover_bg).text_color(hover_fg))
            .focus_visible(move |s| s.border_1().border_color(focus_c))
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .child(pro_svg("0 0 24 24", svg_markup, 14., fg));
        activate(el, cx, on_press)
    }

    fn render_composer(
        &self,
        narrow: bool,
        elapsed_ms: f32,
        reduced: bool,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let p = ProPalette::get(cx);
        let col_pad = if narrow { 12. } else { 24. };
        let draft_text = self
            .composer_editor
            .as_ref()
            .map(|e| e.read(cx).value().to_string())
            .unwrap_or_default();
        let can_send = !draft_text.trim().is_empty() && !self.loading;

        let models = ["Auto", "Claude Sonnet 4.6", "Claude Opus 4.7", "GPT-5"];
        let cur_model = models[self.selected_model.min(models.len() - 1)];

        let active_menu = self.open_menu.unwrap_or(self.last_menu);
        let menu_key: SharedString = match active_menu {
            ComposerMenu::Add => "add".into(),
            ComposerMenu::Approval => "approval".into(),
            ComposerMenu::Model => "model".into(),
        };

        let menu_card = match active_menu {
            ComposerMenu::Add => div()
                .id(pro_eid(&self.id, "rva-add-menu"))
                .occlude()
                .role(Role::Menu)
                .w(px(228.))
                .p(px(5.))
                .rounded(px(12.))
                .bg(p.card)
                .border_1()
                .border_color(p.line_strong)
                .shadow_md()
                .flex()
                .flex_col()
                .gap(px(2.))
                .child(
                    div()
                        .px(px(8.))
                        .py(px(4.))
                        .text_size(px(11.5))
                        .text_color(p.muted)
                        .child(tr(cx, "Add to the chat", "添加到对话")),
                )
                .children(
                    ["Stripe", "HubSpot", "Google Drive", "Slack"]
                        .into_iter()
                        .enumerate()
                        .map(|(ai, app)| {
                            activate(
                                div()
                                    .id(pro_eid(&self.id, format!("rva-add-{ai}")))
                                    .role(Role::MenuItem)
                                    .tab_index(0)
                                    .h(px(32.))
                                    .px(px(8.))
                                    .rounded(px(7.))
                                    .hover(move |s| s.bg(p.hover))
                                    .flex()
                                    .items_center()
                                    .gap(px(8.))
                                    .text_size(px(13.))
                                    .text_color(p.ink)
                                    .cursor_pointer()
                                    .child(pro_app_tile(app, 18., cx))
                                    .child(app),
                                cx,
                                move |this, _, cx| {
                                    if !this.attachments.iter().any(|x| x.as_ref() == app) {
                                        this.attachments.push(app.into());
                                    }
                                    this.open_menu = None;
                                    cx.notify();
                                },
                            )
                        }),
                ),
            ComposerMenu::Approval => {
                let pol_sensor = pro_selection_sensor();
                let pol_key: SharedString = match self.approval_mode {
                    RevenueApprovalMode::RequestApproval => "req".into(),
                    RevenueApprovalMode::AutoReads => "reads".into(),
                    RevenueApprovalMode::FullAutonomy => "auto".into(),
                };
                div()
                    .id(pro_eid(&self.id, "rva-approval-menu"))
                    .occlude()
                    .role(Role::ListBox)
                    .w(px(248.))
                    .p(px(5.))
                    .rounded(px(12.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line_strong)
                    .shadow_md()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .child(
                        div()
                            .px(px(8.))
                            .py(px(4.))
                            .text_size(px(11.5))
                            .text_color(p.muted)
                            .child(tr(
                                cx,
                                "How should actions be approved?",
                                "如何审批外部操作？",
                            )),
                    )
                    .child(
                        div()
                            .relative()
                            .flex()
                            .flex_col()
                            .gap(px(2.))
                            .child(pro_pill_indicator(
                                pro_eid(&self.id, "rva-pol-pill"),
                                pol_key,
                                pol_sensor.clone(),
                                7.,
                                p.select,
                                None,
                                false,
                            ))
                            .children(
                                [
                                    RevenueApprovalMode::RequestApproval,
                                    RevenueApprovalMode::AutoReads,
                                    RevenueApprovalMode::FullAutonomy,
                                ]
                                .into_iter()
                                .enumerate()
                                .map(|(mi, mode)| {
                                    let chosen = self.approval_mode == mode;
                                    let is_alert = mode == RevenueApprovalMode::FullAutonomy;
                                    let sensor = pol_sensor.clone();
                                    activate(
                                        div()
                                            .id(pro_eid(&self.id, format!("rva-pol-{mi}")))
                                            .role(Role::ListBoxOption)
                                            .tab_index(0)
                                            .relative()
                                            .h(px(32.))
                                            .px(px(8.))
                                            .rounded(px(7.))
                                            .when(!chosen, |d| d.hover(move |s| s.bg(p.hover)))
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .text_size(px(13.))
                                            .text_color(if is_alert { p.red_ink } else { p.ink })
                                            .cursor_pointer()
                                            .when(chosen, |d| d.child(pro_mark_selected(&sensor)))
                                            .child(mode.label(cx))
                                            .when(chosen, |d| {
                                                d.child(pro_svg(
                                                    "0 0 24 24",
                                                    CHECK_LINE_SVG,
                                                    13.,
                                                    p.ink,
                                                ))
                                            }),
                                        cx,
                                        move |this, _, cx| {
                                            this.approval_mode = mode;
                                            this.open_menu = None;
                                            cx.notify();
                                        },
                                    )
                                }),
                            ),
                    )
            }
            ComposerMenu::Model => {
                let mdl_sensor = pro_selection_sensor();
                let mdl_key: SharedString = format!("mdl-{}", self.selected_model).into();
                div()
                    .id(pro_eid(&self.id, "rva-model-menu"))
                    .occlude()
                    .role(Role::ListBox)
                    .w(px(210.))
                    .p(px(5.))
                    .ml_auto()
                    .rounded(px(12.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line_strong)
                    .shadow_md()
                    .relative()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .child(pro_pill_indicator(
                        pro_eid(&self.id, "rva-mdl-pill"),
                        mdl_key,
                        mdl_sensor.clone(),
                        7.,
                        p.select,
                        None,
                        false,
                    ))
                    .children(models.into_iter().enumerate().map(|(mi, name)| {
                        let chosen = self.selected_model == mi;
                        let sensor = mdl_sensor.clone();
                        activate(
                            div()
                                .id(pro_eid(&self.id, format!("rva-mdl-{mi}")))
                                .role(Role::ListBoxOption)
                                .tab_index(0)
                                .relative()
                                .h(px(30.))
                                .px(px(8.))
                                .rounded(px(7.))
                                .when(!chosen, |d| d.hover(move |s| s.bg(p.hover)))
                                .flex()
                                .items_center()
                                .justify_between()
                                .text_size(px(13.))
                                .text_color(p.ink)
                                .cursor_pointer()
                                .when(chosen, |d| d.child(pro_mark_selected(&sensor)))
                                .child(name)
                                .when(chosen, |d| {
                                    d.child(pro_svg("0 0 24 24", CHECK_LINE_SVG, 13., p.ink))
                                }),
                            cx,
                            move |this, _, cx| {
                                this.selected_model = mi;
                                this.open_menu = None;
                                cx.notify();
                            },
                        )
                    }))
            }
        };

        let menu_vtx = pro_view_transition(
            pro_eid(&self.id, "rva-menu-vtx"),
            menu_key,
            div().w_full().flex().child(menu_card),
        );
        let popover_pres = pro_presence(
            pro_eid(&self.id, "rva-menu-pres"),
            self.open_menu.is_some(),
            0.18,
            0.13,
            move |amt| {
                let e = pro_ease(amt);
                div()
                    .absolute()
                    .left_0()
                    .right_0()
                    .bottom_full()
                    .pb(px(8.))
                    .child(
                        div()
                            .w_full()
                            .relative()
                            .top(px(6. * (1. - e)))
                            .opacity(e)
                            .child(menu_vtx),
                    )
                    .into_any_element()
            },
        );

        let pol_fg = if self.approval_mode == RevenueApprovalMode::FullAutonomy {
            p.red_ink
        } else {
            p.muted
        };

        let send_t = pro_spring_f32(
            pro_eid(&self.id, "rva-send-t"),
            f32::from(can_send),
            0.18,
            w,
            cx,
        )
        .clamp(0., 1.);
        let focus_t = pro_spring_f32(
            pro_eid(&self.id, "rva-focus-t"),
            f32::from(self.editor_focused),
            0.18,
            w,
            cx,
        )
        .clamp(0., 1.);
        let send_bg = pro_mix_color(p.chip, p.primary, send_t);
        let send_fg = pro_mix_color(p.faint, p.on_primary, send_t);
        let box_border = pro_mix_color(p.line, p.line_strong, focus_t);

        let right_tools = if self.recording {
            let secs = self
                .recording_started
                .map(|t| t.elapsed().as_secs())
                .unwrap_or(0);
            div()
                .flex()
                .items_center()
                .gap(px(6.))
                .child(
                    div()
                        .h(px(20.))
                        .flex()
                        .items_center()
                        .gap(px(2.5))
                        .children((0..10usize).map(|bi| {
                            let bh = if reduced {
                                8.
                            } else {
                                4. + 11. * ((elapsed_ms / 170. + bi as f32 * 0.65).sin().abs())
                            };
                            div().w(px(2.)).h(px(bh)).rounded(px(1.)).bg(p.blue)
                        })),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(p.muted)
                        .child(format!("0:{secs:02}")),
                )
                .child(pro_icon_btn(
                    pro_eid(&self.id, "rva-rec-cancel"),
                    tr(cx, "Cancel voice", "取消录音"),
                    ProIcon::Close,
                    28.,
                    false,
                    cx,
                    |this, _, cx| {
                        this.recording = false;
                        this.recording_started = None;
                        cx.notify();
                    },
                ))
                .child(activate(
                    div()
                        .id(pro_eid(&self.id, "rva-rec-done"))
                        .role(Role::Button)
                        .tab_index(0)
                        .size(px(28.))
                        .rounded(px(8.))
                        .bg(p.primary)
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .child(pro_svg("0 0 24 24", CHECK_LINE_SVG, 14., p.on_primary)),
                    cx,
                    |this, w, cx| {
                        this.recording = false;
                        this.recording_started = None;
                        if let Some(ed) = &this.composer_editor {
                            ed.update(cx, |e, cx| {
                                e.set_value("Split Q3 revenue by region", w, cx);
                            });
                        }
                        cx.notify();
                    },
                ))
        } else {
            div()
                .flex()
                .items_center()
                .gap(px(4.))
                .child(activate(
                    div()
                        .id(pro_eid(&self.id, "rva-model-btn"))
                        .role(Role::Button)
                        .tab_index(0)
                        .h(px(28.))
                        .pl(px(8.))
                        .pr(px(6.))
                        .rounded(px(8.))
                        .when(self.open_menu == Some(ComposerMenu::Model), |d| {
                            d.bg(p.chip)
                        })
                        .hover(move |s| s.bg(p.hover))
                        .flex()
                        .items_center()
                        .gap(px(4.))
                        .text_size(px(13.))
                        .text_color(p.ink)
                        .cursor_pointer()
                        .child(cur_model)
                        .child(pro_icon(ProIcon::ChevronDown, 13., p.muted)),
                    cx,
                    |this, _, cx| {
                        this.open_menu = if this.open_menu == Some(ComposerMenu::Model) {
                            None
                        } else {
                            Some(ComposerMenu::Model)
                        };
                        cx.notify();
                    },
                ))
                .child(pro_icon_btn(
                    pro_eid(&self.id, "rva-mic-btn"),
                    tr(cx, "Dictate", "语音输入"),
                    ProIcon::Mic,
                    28.,
                    false,
                    cx,
                    |this, _, cx| {
                        this.recording = true;
                        this.recording_started = Some(Instant::now());
                        cx.notify();
                    },
                ))
                .child(activate(
                    div()
                        .id(pro_eid(&self.id, "rva-send-btn"))
                        .role(Role::Button)
                        .aria_label(tr(cx, "Send message", "发送"))
                        .tab_index(0)
                        .size(px(28.))
                        .ml(px(2.))
                        .rounded(px(8.))
                        .bg(send_bg)
                        .flex()
                        .items_center()
                        .justify_center()
                        .when(can_send, |d| d.cursor_pointer())
                        .child(pro_icon(ProIcon::ArrowUp, 15., send_fg)),
                    cx,
                    |this, w, cx| {
                        if let Some(ed) = this.composer_editor.clone() {
                            let val = ed.read(cx).value().to_string();
                            if this.ask(val, None, cx) {
                                ed.update(cx, |e, cx| e.set_value("", w, cx));
                            }
                        }
                    },
                ))
        };

        let att_items = if self.attachments.is_empty() {
            self.last_attachments.clone()
        } else {
            self.attachments.clone()
        };
        let att_row = div()
            .px(px(4.))
            .flex()
            .flex_wrap()
            .gap(px(6.))
            .children(att_items.into_iter().enumerate().map(|(ai, att)| {
                div()
                    .h(px(22.))
                    .px(px(8.))
                    .rounded(px(6.))
                    .bg(p.chip)
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .text_size(px(12.))
                    .text_color(p.ink)
                    .child(att)
                    .child(activate(
                        div()
                            .id(pro_eid(&self.id, format!("rva-att-rm-{ai}")))
                            .role(Role::Button)
                            .tab_index(0)
                            .cursor_pointer()
                            .child(pro_icon(ProIcon::Close, 10., p.muted)),
                        cx,
                        move |this, _, cx| {
                            if ai < this.attachments.len() {
                                this.attachments.remove(ai);
                                cx.notify();
                            }
                        },
                    ))
            }))
            .into_any_element();

        div()
            .flex_none()
            .w_full()
            .px(px(col_pad))
            .pb(px(if narrow { 12. } else { 20. }))
            .child(
                div()
                    .relative()
                    .w_full()
                    .max_w(px(760.))
                    .mx_auto()
                    .flex()
                    .flex_col()
                    .child(popover_pres)
                    .child(
                        div()
                            .w_full()
                            .pt(px(14.))
                            .px(px(10.))
                            .pb(px(10.))
                            .rounded(px(16.))
                            .bg(if p.dark { rgb(0x1c1c1e).into() } else { p.card })
                            .border_1()
                            .border_color(box_border)
                            .flex()
                            .flex_col()
                            .gap(px(12.))
                            .child(pro_fold(
                                pro_eid(&self.id, "rva-att-fold"),
                                !self.attachments.is_empty(),
                                24.,
                                12.,
                                move || att_row,
                            ))
                            .child(
                                div()
                                    .px(px(4.))
                                    .h(px(28.))
                                    .when_some(self.composer_editor.clone(), |d, ed| d.child(ed)),
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
                                                pro_eid(&self.id, "rva-plus-btn"),
                                                tr(cx, "Add", "添加"),
                                                ProIcon::Plus,
                                                28.,
                                                self.open_menu == Some(ComposerMenu::Add),
                                                cx,
                                                |this, _, cx| {
                                                    this.open_menu = if this.open_menu
                                                        == Some(ComposerMenu::Add)
                                                    {
                                                        None
                                                    } else {
                                                        Some(ComposerMenu::Add)
                                                    };
                                                    cx.notify();
                                                },
                                            ))
                                            .child(activate(
                                                div()
                                                    .id(pro_eid(&self.id, "rva-approval-btn"))
                                                    .role(Role::Button)
                                                    .tab_index(0)
                                                    .h(px(28.))
                                                    .pl(px(8.))
                                                    .pr(px(10.))
                                                    .rounded(px(8.))
                                                    .when(
                                                        self.open_menu
                                                            == Some(ComposerMenu::Approval),
                                                        |d| d.bg(p.chip),
                                                    )
                                                    .hover(move |s| s.bg(p.hover).text_color(p.ink))
                                                    .flex()
                                                    .items_center()
                                                    .gap(px(6.))
                                                    .text_size(px(13.))
                                                    .text_color(pol_fg)
                                                    .cursor_pointer()
                                                    .child(pro_svg(
                                                        "0 0 68.155 68.155",
                                                        HAND_APPROVAL_SVG,
                                                        15.,
                                                        pol_fg,
                                                    ))
                                                    .when(!narrow, |d| {
                                                        d.child(self.approval_mode.label(cx))
                                                    }),
                                                cx,
                                                |this, _, cx| {
                                                    this.open_menu = if this.open_menu
                                                        == Some(ComposerMenu::Approval)
                                                    {
                                                        None
                                                    } else {
                                                        Some(ComposerMenu::Approval)
                                                    };
                                                    cx.notify();
                                                },
                                            )),
                                    )
                                    .child(right_tools),
                            ),
                    ),
            )
    }

    fn render_loading_skeleton(narrow: bool, phase: f32, cx: &App) -> Div {
        div()
            .w_full()
            .max_w(px(760.))
            .mx_auto()
            .pt(px(32.))
            .pb(px(24.))
            .flex()
            .flex_col()
            .gap(px(28.))
            .child(
                div()
                    .w_full()
                    .flex()
                    .justify_end()
                    .child(pro_skeleton_bar(246., 38., 16., phase, cx)),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(16.))
                    .child(
                        div()
                            .flex()
                            .items_start()
                            .gap(px(12.))
                            .child(pro_skeleton_bar(16., 16., 4., phase, cx))
                            .child(
                                div()
                                    .flex_1()
                                    .flex()
                                    .flex_col()
                                    .gap(px(7.))
                                    .child(pro_skeleton_bar(
                                        if narrow { 260. } else { 480. },
                                        14.,
                                        5.,
                                        phase + 0.08,
                                        cx,
                                    ))
                                    .child(pro_skeleton_bar(
                                        if narrow { 190. } else { 340. },
                                        14.,
                                        5.,
                                        phase + 0.14,
                                        cx,
                                    )),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .gap(px(10.))
                            .child(div().flex_1().child(pro_skeleton_bar(
                                200.,
                                76.,
                                12.,
                                phase + 0.2,
                                cx,
                            )))
                            .child(div().flex_1().child(pro_skeleton_bar(
                                200.,
                                76.,
                                12.,
                                phase + 0.25,
                                cx,
                            )))
                            .when(!narrow, |d| {
                                d.child(div().flex_1().child(pro_skeleton_bar(
                                    200.,
                                    76.,
                                    12.,
                                    phase + 0.3,
                                    cx,
                                )))
                            }),
                    )
                    .child(pro_skeleton_bar(
                        if narrow { 300. } else { 680. },
                        210.,
                        14.,
                        phase + 0.35,
                        cx,
                    )),
            )
    }

    fn render_empty_starter(&self, cx: &Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let starters = [
            "How did revenue develop last quarter?",
            "Split it by region",
            "Forecast Q4",
            "Why did week 35 jump?",
        ];
        div()
            .w_full()
            .max_w(px(760.))
            .mx_auto()
            .min_h(px(360.))
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .child(pro_empty_state(
                ProIcon::UsageChart,
                tr(cx, "Ask about revenue, customers or plans", "探索营收、客户与增长趋势"),
                tr(
                    cx,
                    "Answers with KPI cards, weekly YoY curves, regional splits, and forecasts from Stripe and HubSpot.",
                    "基于 Stripe 与 HubSpot 实时数据生成指标卡、同比曲线、区域拆分与季度预测。",
                ),
                None,
                cx,
            ))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .justify_center()
                    .gap(px(8.))
                    .children(starters.into_iter().enumerate().map(|(si, q)| {
                        activate(
                            div()
                                .id(pro_eid(&self.id, format!("starter-{si}")))
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
                                .hover(move |s| s.border_color(p.line_strong).text_color(p.ink))
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .text_size(px(13.))
                                .text_color(p.text)
                                .cursor_pointer()
                                .child(q)
                                .child(pro_icon(ProIcon::ExternalArrow, 13., p.faint)),
                            cx,
                            move |this, _, cx| {
                                this.ask(q, None, cx);
                            },
                        )
                    })),
            )
    }

    fn render_work_panel(
        &mut self,
        _w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<(SharedString, ProIcon, AnyElement)> {
        let kind = self.active_artifact?;
        let p = ProPalette::get(cx);

        let title: SharedString = match kind {
            RevenueArtifactKind::StripeLedger => {
                tr(cx, "Stripe · Q3 revenue", "Stripe · Q3 营收账本").into()
            }
            RevenueArtifactKind::HubSpotDeals => {
                tr(cx, "HubSpot · Closed deals", "HubSpot · 签约商机").into()
            }
            RevenueArtifactKind::WeeklyCsv => {
                tr(cx, "q3-weekly-revenue.csv", "q3-weekly-revenue.csv").into()
            }
        };

        let kind_key: SharedString = match kind {
            RevenueArtifactKind::StripeLedger => "stripe".into(),
            RevenueArtifactKind::HubSpotDeals => "hubspot".into(),
            RevenueArtifactKind::WeeklyCsv => "csv".into(),
        };
        let art_sensor = pro_selection_sensor();

        let tabs = div()
            .px(px(12.))
            .py(px(8.))
            .border_b_1()
            .border_color(p.line_soft)
            .child(
                div()
                    .relative()
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .child(pro_pill_indicator(
                        pro_eid(&self.id, "rva-art-pill"),
                        kind_key.clone(),
                        art_sensor.clone(),
                        6.,
                        p.chip,
                        Some(p.line),
                        false,
                    ))
                    .children(
                        [
                            (RevenueArtifactKind::StripeLedger, "Stripe"),
                            (RevenueArtifactKind::HubSpotDeals, "HubSpot"),
                            (
                                RevenueArtifactKind::WeeklyCsv,
                                tr(cx, "Weekly CSV", "周度表"),
                            ),
                        ]
                        .into_iter()
                        .map(|(k, lbl)| {
                            let on = kind == k;
                            let sensor = art_sensor.clone();
                            activate(
                                div()
                                    .id(pro_eid(&self.id, format!("art-tab-{lbl}")))
                                    .role(Role::Tab)
                                    .tab_index(0)
                                    .relative()
                                    .h(px(26.))
                                    .px(px(9.))
                                    .rounded(px(6.))
                                    .when(!on, |d| d.hover(move |s| s.text_color(p.ink)))
                                    .flex()
                                    .items_center()
                                    .text_size(px(12.))
                                    .font_weight(if on {
                                        FontWeight::MEDIUM
                                    } else {
                                        FontWeight::NORMAL
                                    })
                                    .text_color(if on { p.ink } else { p.muted })
                                    .cursor_pointer()
                                    .when(on, |d| d.child(pro_mark_selected(&sensor)))
                                    .child(lbl),
                                cx,
                                move |this, _, cx| {
                                    this.active_artifact = Some(k);
                                    cx.notify();
                                },
                            )
                        }),
                    ),
            );

        let rows: Vec<(&str, &str, &str, f32)> = match kind {
            RevenueArtifactKind::StripeLedger => vec![
                ("Gross subscriptions", "$1,154,000", "+19.1%", 0.89),
                ("Expansion & seats", "$134,200", "+24.6%", 0.24),
                ("Refunds & credits", "-$48,200", "-2.1%", 0.11),
                ("Net Q3 revenue", "$1,240,000", "+18.2%", 0.96),
                ("Ending ARR run-rate", "$5,180,000", "+16.8%", 1.00),
            ],
            RevenueArtifactKind::HubSpotDeals => vec![
                ("University Systems EDU", "Enterprise", "$28,400", 1.00),
                ("Nordisk EdTech Group", "Enterprise", "$19,200", 0.68),
                ("Northwind Logistics", "Pro Annual", "$14,800", 0.52),
                ("Lumen Labs", "Enterprise", "$12,600", 0.44),
                ("Fieldwork Studio", "Pro Annual", "$11,000", 0.39),
            ],
            RevenueArtifactKind::WeeklyCsv => vec![
                ("W33 (Aug 17)", "$93K", "$81K LY", 0.69),
                ("W34 (Aug 24)", "$97K", "$80K LY", 0.72),
                ("W35 (Aug 31 · Peak)", "$134K", "$84K LY", 1.00),
                ("W36 (Sep 7)", "$104K", "$86K LY", 0.78),
                ("W39 (Sep 28)", "$109K", "$90K LY", 0.81),
            ],
        };

        let app_badge = match kind {
            RevenueArtifactKind::StripeLedger => pro_app_artwork("stripe", 16., cx),
            RevenueArtifactKind::HubSpotDeals => pro_app_artwork("hubspot", 16., cx),
            RevenueArtifactKind::WeeklyCsv => None,
        };

        let bar_tint = match kind {
            RevenueArtifactKind::StripeLedger => p.blue,
            RevenueArtifactKind::HubSpotDeals => p.teal,
            RevenueArtifactKind::WeeklyCsv => p.purple,
        };

        let panel_inner = div()
            .p(px(14.))
            .flex()
            .flex_col()
            .gap(px(12.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(7.))
                            .when_some(app_badge, |d, m| d.child(m))
                            .child(
                                div()
                                    .text_size(px(12.5))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .child(tr(
                                        cx,
                                        "Verified source snapshot",
                                        "已校验数据源快照",
                                    )),
                            ),
                    )
                    .child(pro_btn(
                        pro_eid(&self.id, "art-copy-csv"),
                        tr(cx, "Copy CSV", "复制 CSV"),
                        Some(ProIcon::Copy),
                        ProBtnStyle::Secondary,
                        true,
                        cx,
                        |this, _, cx| {
                            cx.write_to_clipboard(ClipboardItem::new_string(
                                "week,this_year_k,last_year_k\nW33,93,81\nW34,97,80\nW35,134,84\nW36,104,86\nW39,109,90".to_string(),
                            ));
                            this.toast = Some((
                                tr(cx, "Copied CSV to clipboard", "已复制 CSV 到剪贴板").into(),
                                Instant::now(),
                            ));
                            cx.notify();
                        },
                    )),
            )
            .child(
                div()
                    .rounded(px(10.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .children(
                        rows.into_iter()
                            .enumerate()
                            .map(|(ri, (c1, c2, c3, ratio))| {
                                div()
                                    .px(px(11.))
                                    .py(px(9.))
                                    .when(ri > 0, |d| d.border_t_1().border_color(p.line_soft))
                                    .flex()
                                    .flex_col()
                                    .gap(px(6.))
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .gap(px(8.))
                                            .text_size(px(12.5))
                                            .child(
                                                div()
                                                    .flex_1()
                                                    .min_w_0()
                                                    .truncate()
                                                    .text_color(p.ink)
                                                    .child(c1),
                                            )
                                            .child(
                                                div()
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .text_color(p.strong)
                                                    .child(c2),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(11.5))
                                                    .text_color(p.muted)
                                                    .child(c3),
                                            ),
                                    )
                                    .child(pro_spring_bar(
                                        pro_eid(&self.id, format!("rva-art-bar-{ri}")),
                                        ratio,
                                        3.5,
                                        2.,
                                        p.chip,
                                        bar_tint,
                                    ))
                            }),
                    ),
            );

        let body = div()
            .flex()
            .flex_col()
            .child(tabs)
            .child(pro_view_transition(
                pro_eid(&self.id, "rva-art-vtx"),
                kind_key,
                panel_inner,
            ))
            .into_any_element();

        Some((title, ProIcon::ClipboardDoc, body))
    }
}

impl Render for AiChatRevenueScreen {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.standalone {
            return self.render_page(w, cx).into_any_element();
        }
        self.shell.clone().into_any_element()
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
                let phase = ((elapsed_ms + AGENT_WAVE_MS[i]) / 1200.) * std::f32::consts::TAU;
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
            Bounds::new(center - point(outer, outer), size(outer * 2., outer * 2.)),
            ring_c,
        )
        .corner_radii(outer),
    );
    window.paint_quad(
        fill(
            Bounds::new(center - point(inner, inner), size(inner * 2., inner * 2.)),
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
        let p0 = pt_of(i, pair[0]);
        let p1 = pt_of(i + 1, pair[1]);
        paint_dashed_segment(window, p0, p1, color, stroke_w, dash, gap);
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

fn format_dollars_axis(dollars: f64) -> String {
    if dollars >= 1_000_000.0 {
        format!("${:.1}m", dollars / 1_000_000.0)
    } else if dollars >= 10_000.0 {
        format!("${:.0}k", dollars / 1_000.0)
    } else if dollars >= 1_000.0 {
        format!("${:.1}k", dollars / 1_000.0)
    } else {
        format!("${:.0}", dollars)
    }
}

fn format_dollars_comma(dollars: f64) -> String {
    let whole = dollars.round().max(0.0) as u64;
    let s = whole.to_string();
    let mut out = String::with_capacity(s.len() + 4);
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    let formatted: String = out.chars().rev().collect();
    format!("${formatted}")
}

fn compute_trend_focus(
    labels: &[SharedString],
    current: &[f64],
    previous: &[f64],
    from: usize,
    to: usize,
) -> (String, String) {
    if labels.is_empty() || current.is_empty() {
        return ("Selection".to_string(), "$0".to_string());
    }
    let lo = from.min(to).min(current.len() - 1);
    let hi = from.max(to).min(current.len() - 1);
    let sum_cur: f64 = current[lo..=hi].iter().sum();
    let sum_prev: f64 = previous
        .get(lo..=hi.min(previous.len().saturating_sub(1)))
        .map(|s| s.iter().sum())
        .unwrap_or(0.0);
    let label = if lo == hi {
        labels[lo].to_string()
    } else {
        format!("{} – {}", labels[lo], labels[hi])
    };
    let detail = if sum_prev > 0.0 {
        let pct = (sum_cur - sum_prev) / sum_prev * 100.0;
        format!("{} · {:+.1}% on last year", format_k_dollars(sum_cur), pct)
    } else {
        format_k_dollars(sum_cur)
    };
    (label, detail)
}

fn sample_revenue_nav_groups(cx: &App) -> Vec<ShellNavGroup> {
    vec![
        ShellNavGroup::primary(vec![
            ShellNavItem::new("new-chat", tr(cx, "New chat", "新建对话"), ProIcon::NewChat),
            ShellNavItem::new("folders", tr(cx, "Folders", "文件夹"), ProIcon::Folder),
            ShellNavItem::new(
                "integrations",
                tr(cx, "Integrations", "集成"),
                ProIcon::Grid,
            ),
        ]),
        ShellNavGroup::named(
            tr(cx, "Pinned", "已置顶"),
            vec![ShellNavItem::new(
                "q3-launch-msg",
                "Q3 launch messaging",
                ProIcon::Pin,
            )],
        ),
        ShellNavGroup::named(
            tr(cx, "Projects", "项目"),
            vec![
                ShellNavItem::new("q3-campaign", "Q3 Campaign", ProIcon::ProjectFolder),
                ShellNavItem::new(
                    "q3-revenue-review",
                    "Q3 revenue review",
                    ProIcon::UsageChart,
                ),
                ShellNavItem::new(
                    "linen-hooks",
                    "Ad hooks for the linen overshirt",
                    ProIcon::Thread,
                )
                .with_unread(true),
                ShellNavItem::new("brand-assets", "Brand assets", ProIcon::Folder),
                ShellNavItem::new("hiring", "Hiring", ProIcon::Folder),
                ShellNavItem::new("research", "Research", ProIcon::Folder),
            ],
        ),
        ShellNavGroup::named(
            tr(cx, "Chats", "最近对话"),
            vec![
                ShellNavItem::new(
                    "onboarding-draft",
                    "Onboarding email draft",
                    ProIcon::Thread,
                ),
                ShellNavItem::new("aug-calendar", "August content calendar", ProIcon::Thread),
            ],
        ),
    ]
}

fn sample_revenue_turns() -> Vec<RevenueTurn> {
    vec![
        RevenueTurn::User {
            id: "u-1".into(),
            quote: None,
            text: "How did revenue develop last quarter?".into(),
        },
        RevenueTurn::Assistant {
            id: "a-1".into(),
            text: "Q3 closed at $1.24M – 18% more than a year ago. Growth was steady through August and jumped in the back-to-school week, then settled about 12% higher than before.".into(),
            kpis: vec![
                RevenueKpi::new("Revenue", "$1.24M", "+18.2% vs. last year", true),
                RevenueKpi::new("New MRR", "$86K", "+9.4% vs. last year", true),
                RevenueKpi::new("Churn", "2.1%", "-0.6% vs. last year", true),
            ],
            chart: Some(RevenueChart::Trend {
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
            caption: Some("Weekly revenue · Q3 2026 against Q3 2025 · back to school in W35".into()),
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

fn build_revenue_answer(
    question: &str,
    focus: Option<&RevenueChartFocus>,
    _turns: &[RevenueTurn],
) -> RevenueTurn {
    let lower = question.to_ascii_lowercase();

    if let Some(foc) = focus {
        return RevenueTurn::Assistant {
            id: "a-focus".into(),
            text: format!(
                "Across {} ({}), enterprise renewals and annual plan upgrades accounted for 64% of volume, while involuntary churn stayed below 0.4%.",
                foc.label, foc.detail
            )
            .into(),
            kpis: vec![
                RevenueKpi::new("Selected slice", foc.label.clone(), foc.detail.clone(), true),
                RevenueKpi::new("Annual share", "64%", "+7.1% vs. Q2", true),
                RevenueKpi::new("Net retention", "114%", "+2.4 pts YoY", true),
            ],
            chart: None,
            caption: None,
            sources: vec!["Stripe".into(), "HubSpot".into()],
            next: vec![
                "Split it by region".into(),
                "Forecast Q4".into(),
            ],
            liked: false,
            streaming: false,
        };
    }

    if lower.contains("region") || lower.contains("split") {
        return RevenueTurn::Assistant {
            id: "a-split".into(),
            text: "North America brought in 55% of Q3 revenue ($682K), led by annual Enterprise seats. Europe grew fastest at +24% YoY after localized EUR billing launched.".into(),
            kpis: vec![
                RevenueKpi::new("North America", "$682K", "+16.4% vs. last year", true),
                RevenueKpi::new("Europe", "$348K", "+24.1% vs. last year", true),
                RevenueKpi::new("APAC & LATAM", "$210K", "+15.0% vs. last year", true),
            ],
            chart: Some(RevenueChart::Split {
                title: "Q3 revenue by region".into(),
                previous_total: Some(1_049_000.0),
                since_label: Some("Q3 2025".into()),
                groups: vec![
                    RevenueSplitGroup {
                        name: "North America".into(),
                        color_idx: 0,
                        parts: vec![
                            RevenueSplitSegment::new("Enterprise", 390_000.0),
                            RevenueSplitSegment::new("Pro", 212_000.0),
                            RevenueSplitSegment::new("Starter", 80_000.0),
                        ],
                    },
                    RevenueSplitGroup {
                        name: "Europe".into(),
                        color_idx: 1,
                        parts: vec![
                            RevenueSplitSegment::new("Enterprise", 184_000.0),
                            RevenueSplitSegment::new("Pro", 116_000.0),
                            RevenueSplitSegment::new("Starter", 48_000.0),
                        ],
                    },
                    RevenueSplitGroup {
                        name: "APAC".into(),
                        color_idx: 2,
                        parts: vec![
                            RevenueSplitSegment::new("Enterprise", 72_000.0),
                            RevenueSplitSegment::new("Pro", 48_000.0),
                            RevenueSplitSegment::new("Starter", 22_000.0),
                        ],
                    },
                    RevenueSplitGroup {
                        name: "LATAM".into(),
                        color_idx: 3,
                        parts: vec![
                            RevenueSplitSegment::new("Enterprise", 24_000.0),
                            RevenueSplitSegment::new("Pro", 28_000.0),
                            RevenueSplitSegment::new("Starter", 16_000.0),
                        ],
                    },
                ],
                alt_groups: vec![
                    RevenueSplitGroup {
                        name: "Enterprise".into(),
                        color_idx: 0,
                        parts: vec![
                            RevenueSplitSegment::new("North America", 390_000.0),
                            RevenueSplitSegment::new("Europe", 184_000.0),
                            RevenueSplitSegment::new("APAC & LATAM", 96_000.0),
                        ],
                    },
                    RevenueSplitGroup {
                        name: "Pro".into(),
                        color_idx: 1,
                        parts: vec![
                            RevenueSplitSegment::new("North America", 212_000.0),
                            RevenueSplitSegment::new("Europe", 116_000.0),
                            RevenueSplitSegment::new("APAC & LATAM", 76_000.0),
                        ],
                    },
                    RevenueSplitGroup {
                        name: "Starter".into(),
                        color_idx: 2,
                        parts: vec![
                            RevenueSplitSegment::new("North America", 80_000.0),
                            RevenueSplitSegment::new("Europe", 48_000.0),
                            RevenueSplitSegment::new("APAC & LATAM", 38_000.0),
                        ],
                    },
                ],
            }),
            caption: None,
            sources: vec!["Stripe".into(), "HubSpot".into()],
            next: vec![
                "Forecast Q4".into(),
                "Why did week 35 jump?".into(),
            ],
            liked: false,
            streaming: false,
        };
    }

    if lower.contains("forecast") || lower.contains("q4") {
        return RevenueTurn::Assistant {
            id: "a-forecast".into(),
            text: "Carrying September's 12% baseline lift into Q4 puts revenue on pace for $1.48M ($1.41M – $1.55M likely range), clearing the $1.40M Q4 plan by week 50.".into(),
            kpis: vec![
                RevenueKpi::new("Q4 forecast", "$1.48M", "+19.4% vs. Q3", true),
                RevenueKpi::new("Likely range", "$1.41M–$1.55M", "80% confidence", true),
                RevenueKpi::new("Plan coverage", "105.7%", "+$80K above target", true),
            ],
            chart: Some(RevenueChart::Forecast {
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
            next: vec![
                "Split it by region".into(),
                "Why did week 35 jump?".into(),
            ],
            liked: false,
            streaming: false,
        };
    }

    RevenueTurn::Assistant {
        id: "a-w35".into(),
        text: "Week 35 jumped +38% ($134K vs. $97K in W34) when 14 annual education workspaces renewed together and the back-to-school Pro campaign converted 410 trial teams.".into(),
        kpis: vec![
            RevenueKpi::new("W35 total", "$134K", "+59.5% vs. W35 last year", true),
            RevenueKpi::new("EDU renewals", "$52K", "14 annual contracts", true),
            RevenueKpi::new("Trial conversions", "410", "+2.1x weekly average", true),
        ],
        chart: Some(RevenueChart::Split {
            title: "Week 35 revenue drivers".into(),
            previous_total: Some(84_000.0),
            since_label: Some("W35 2025".into()),
            groups: vec![
                RevenueSplitGroup {
                    name: "EDU Annual".into(),
                    color_idx: 0,
                    parts: vec![
                        RevenueSplitSegment::new("Renewals", 38_000.0),
                        RevenueSplitSegment::new("Seat expansion", 14_000.0),
                    ],
                },
                RevenueSplitGroup {
                    name: "Self-serve Pro".into(),
                    color_idx: 1,
                    parts: vec![
                        RevenueSplitSegment::new("New annual", 29_000.0),
                        RevenueSplitSegment::new("Monthly", 15_000.0),
                    ],
                },
                RevenueSplitGroup {
                    name: "Enterprise".into(),
                    color_idx: 2,
                    parts: vec![
                        RevenueSplitSegment::new("New logos", 24_000.0),
                        RevenueSplitSegment::new("Add-ons", 14_000.0),
                    ],
                },
            ],
            alt_groups: Vec::new(),
        }),
        caption: None,
        sources: vec!["Stripe".into(), "HubSpot".into()],
        next: vec![
            "Split it by region".into(),
            "Forecast Q4".into(),
        ],
        liked: false,
        streaming: false,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        RevenueChart, RevenueChartFocus, RevenueTurn, build_revenue_answer, compute_trend_focus,
        sample_revenue_turns,
    };

    #[test]
    fn initial_q3_revenue_series_matches_w35_callout_and_range_focus() {
        let turns = sample_revenue_turns();
        assert_eq!(turns.len(), 2);
        let RevenueTurn::Assistant {
            kpis,
            chart:
                Some(RevenueChart::Trend {
                    labels,
                    current,
                    previous,
                    mark,
                }),
            sources,
            ..
        } = &turns[1]
        else {
            panic!("expected initial assistant trend turn");
        };

        assert_eq!(kpis.len(), 3);
        assert_eq!(kpis[0].value.as_ref(), "$1.24M");
        assert_eq!(labels.len(), 13);
        assert_eq!(mark, &Some(8));
        assert_eq!(labels[8].as_ref(), "W35");
        assert!((current[8] - 134.0).abs() < 1e-6);
        assert!((previous[8] - 84.0).abs() < 1e-6);
        assert_eq!(sources.len(), 2);

        let (lbl, detail) = compute_trend_focus(labels, current, previous, 5, 7);
        assert_eq!(lbl, "W32 – W34");
        assert_eq!(detail, "$284K · +18.3% on last year");
    }

    #[test]
    fn domain_responder_handles_region_split_forecast_and_focused_queries() {
        let turns = sample_revenue_turns();
        let split_turn = build_revenue_answer("Split it by region", None, &turns);
        assert!(matches!(
            split_turn,
            RevenueTurn::Assistant {
                chart: Some(RevenueChart::Split { .. }),
                ..
            }
        ));

        let fc_turn = build_revenue_answer("Forecast Q4", None, &turns);
        assert!(matches!(
            fc_turn,
            RevenueTurn::Assistant {
                chart: Some(RevenueChart::Forecast { .. }),
                ..
            }
        ));

        let focus = RevenueChartFocus {
            turn_id: "a-1".into(),
            from: 5,
            to: 7,
            label: "W32 – W34".into(),
            detail: "$284K · +18.3% on last year".into(),
        };
        let focused = build_revenue_answer("What drove these weeks?", Some(&focus), &turns);
        if let RevenueTurn::Assistant { text, kpis, .. } = focused {
            assert!(text.contains("W32 – W34"));
            assert_eq!(kpis[0].value.as_ref(), "W32 – W34");
        } else {
            panic!("expected assistant turn");
        }
    }
}
