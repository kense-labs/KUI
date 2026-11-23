//! Conversational AI spend planner with provider breakdowns, trend curves, and savings levers.
//!
//! Renders [`AiChatSpendScreen`] inside [`AppShell`] (`AppShellPreset::Chat`), pairing a
//! centered AI cost thread (KPI cards, stacked provider/model columns, weekly trend curves,
//! interactive savings levers with live six-month forecast updates, source pills, and
//! follow-up chips) with a multi-mode composer and an optional right-hand usage ledger panel.

use crate::{
    components::{
        app_shell::{AppShell, AppShellEvent, AppShellPreset, ShellNavGroup, ShellNavItem},
        pro_chrome::{
            ProBtnStyle, ProPalette, pro_app_tile, pro_btn, pro_ease, pro_eid, pro_empty_state,
            pro_fold, pro_icon_btn, pro_mark_selected, pro_mix_color, pro_pill_indicator,
            pro_presence, pro_selection_sensor, pro_skeleton_bar, pro_spring_bar, pro_spring_f32,
            pro_switch, pro_toast_banner, pro_view_transition, tr,
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
pub enum SpendApprovalMode {
    /// Prompt before applying routing rules or budget caps.
    #[default]
    RequestApproval,
    /// Run read-only usage queries automatically.
    AutoReads,
    /// Apply cost routing changes without prompts.
    FullAutonomy,
}

impl SpendApprovalMode {
    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::RequestApproval => tr(cx, "Request approval", "需审批"),
            Self::AutoReads => tr(cx, "Auto-approve reads", "只读自动批准"),
            Self::FullAutonomy => tr(cx, "Full autonomy", "全自动执行"),
        }
    }
}

/// Palette swatch for provider or team stacked columns.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SpendBarTone {
    /// Warm terracotta/coral (`Anthropic`).
    #[default]
    Coral,
    /// Teal (`OpenAI`).
    Teal,
    /// Vivid blue (`Google`).
    Blue,
    /// Purple (`Platform` / custom).
    Purple,
    /// Amber (`Growth` / custom).
    Amber,
}

impl SpendBarTone {
    fn resolve(self, p: &ProPalette) -> Hsla {
        match self {
            Self::Coral => {
                if p.dark {
                    rgb(0xe07d5b).into()
                } else {
                    rgb(0xd97757).into()
                }
            }
            Self::Teal => {
                if p.dark {
                    rgb(0x25bfa9).into()
                } else {
                    rgb(0x13b3a0).into()
                }
            }
            Self::Blue => {
                if p.dark {
                    rgb(0x1e96ff).into()
                } else {
                    rgb(0x0190fe).into()
                }
            }
            Self::Purple => p.purple,
            Self::Amber => p.amber,
        }
    }
}

/// Right-hand artifact tab inside [`AppShell`]'s inspector panel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SpendArtifactKind {
    /// Token and model usage breakdown from the Usage API.
    #[default]
    UsageApi,
    /// Monthly invoice lines and committed credits.
    BillingInvoice,
    /// Exportable model-routing rules configuration.
    RoutingRules,
}

/// Summary KPI card rendered above a spend chart.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpendKpi {
    pub label: SharedString,
    pub value: SharedString,
    pub change: Option<SharedString>,
    pub good: bool,
}

impl SpendKpi {
    /// Construct a spend KPI card (`good = false` renders cost increases in red).
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

/// One model cost segment inside a provider or team bar (`value` in dollars).
#[derive(Clone, Debug, PartialEq)]
pub struct SpendModelPart {
    pub name: SharedString,
    pub value: f64,
}

impl SpendModelPart {
    /// Construct a named model segment in dollars.
    pub fn new(name: impl Into<SharedString>, value: f64) -> Self {
        Self {
            name: name.into(),
            value,
        }
    }
}

/// One provider or team column inside [`SpendChart::Split`].
#[derive(Clone, Debug, PartialEq)]
pub struct SpendSplitGroup {
    pub name: SharedString,
    /// Optional brand mark key (`"claude"`, `"openai"`, `"gemini"`).
    pub logo: Option<SharedString>,
    pub tone: SpendBarTone,
    pub parts: Vec<SpendModelPart>,
}

impl SpendSplitGroup {
    /// Total dollar spend across all model segments in this group.
    pub fn total(&self) -> f64 {
        self.parts.iter().map(|p| p.value).sum()
    }
}

/// Control type for an interactive optimization lever inside [`SpendChart::Plan`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpendLeverKind {
    /// Percentage slider from `0` to `max_pct` in `5%` increments.
    Range { max_pct: u32 },
    /// Binary on/off toggle (`1.0` on, `0.0` off).
    Switch,
}

/// Interactive cost-saving lever in a [`SpendChart::Plan`] card.
#[derive(Clone, Debug, PartialEq)]
pub struct SpendLever {
    pub id: SharedString,
    pub label: SharedString,
    pub detail: SharedString,
    pub kind: SpendLeverKind,
    /// Monthly dollar spend touched by this lever.
    pub base_dollars: f64,
    /// Fraction of `base_dollars` saved at `100%` (or when switched on).
    pub save_rate: f64,
    /// Current slider percentage (`0..=max_pct`) or switch state (`0.0` / `1.0`).
    pub value: f64,
}

impl SpendLever {
    /// Monthly dollar savings produced at the lever's current `value`.
    pub fn monthly_savings(&self) -> f64 {
        match self.kind {
            SpendLeverKind::Range { max_pct } => {
                let clamped = self.value.clamp(0.0, max_pct as f64);
                self.base_dollars * (clamped / 100.0) * self.save_rate
            }
            SpendLeverKind::Switch => {
                if self.value >= 0.5 {
                    self.base_dollars * self.save_rate
                } else {
                    0.0
                }
            }
        }
    }

    /// Return whether this lever is actively saving money.
    pub fn is_active(&self) -> bool {
        self.monthly_savings() > 1.0
    }
}

/// Inline chart or interactive planner attached to an assistant turn.
#[derive(Clone, Debug, PartialEq)]
pub enum SpendChart {
    /// Stacked bar breakdown by provider or team (`values` in `$`).
    Split {
        title: SharedString,
        previous_total: Option<f64>,
        since_label: Option<SharedString>,
        groups: Vec<SpendSplitGroup>,
        alt_tab_labels: Option<(SharedString, SharedString)>,
        alt_groups: Vec<SpendSplitGroup>,
    },
    /// Weekly spend curve against prior month (`values` in `$K`).
    Trend {
        labels: Vec<SharedString>,
        current: Vec<f64>,
        previous: Vec<f64>,
        mark: Option<usize>,
    },
    /// Interactive optimization planner with sliders, switch, and live 6-month forecast.
    Plan {
        now_dollars: f64,
        months: Vec<SharedString>,
        monthly_growth: f64,
        levers: Vec<SpendLever>,
    },
}

/// Focused provider bar or week stretch picked by the user to ask a targeted question.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpendChartFocus {
    pub turn_id: SharedString,
    pub from: usize,
    pub to: usize,
    pub label: SharedString,
    pub detail: SharedString,
}

/// One turn in the [`AiChatSpendScreen`] conversation thread.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, PartialEq)]
pub enum SpendTurn {
    /// Right-aligned user prompt with optional chart or plan quote banner above it.
    User {
        id: SharedString,
        quote: Option<SharedString>,
        text: SharedString,
    },
    /// Assistant spend reply with KPI cards, chart/planner, sources, and follow-up chips.
    Assistant {
        id: SharedString,
        text: SharedString,
        kpis: Vec<SpendKpi>,
        chart: Option<SpendChart>,
        caption: Option<SharedString>,
        sources: Vec<SharedString>,
        next: Vec<SharedString>,
        liked: bool,
        streaming: bool,
    },
}

/// Events emitted by [`AiChatSpendScreen`].
#[derive(Clone, Debug, PartialEq)]
pub enum AiChatSpendEvent {
    /// Sidebar item clicked.
    Navigate(SharedString),
    /// User asked a question from the composer, chip, or chart selection field.
    Ask {
        question: SharedString,
        focus: Option<SpendChartFocus>,
    },
    /// User picked a provider bar or week stretch on a chart.
    SelectChartPart(SpendChartFocus),
    /// User adjusted one of the interactive savings levers (`lever_id`, `new_value`).
    AdjustLever { id: SharedString, value: f64 },
    /// User clicked `Turn into rules` on a savings plan (`monthly_savings_dollars`).
    ApplyPlan { monthly_savings: f64 },
    /// Answer summary copied to clipboard.
    CopyAnswer(SharedString),
    /// Source badge clicked (`"Usage API"`, `"Billing"`).
    OpenSource(SharedString),
    /// Ledger sheet opened for an answer.
    ExportLedger(SharedString),
    /// Helpful thumbs-up toggled on an assistant turn.
    Feedback { turn_id: SharedString, liked: bool },
    /// Assistant answer regenerated.
    Regenerate(SharedString),
}

impl EventEmitter<AiChatSpendEvent> for AiChatSpendScreen {}

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

/// Interactive AI Chat / Spend Planner Pro screen (`ai-chat-spend`).
pub struct AiChatSpendScreen {
    id: ElementId,
    shell: Entity<AppShell>,
    standalone: bool,
    turns: Vec<SpendTurn>,
    use_alt_split: bool,
    trend_recent_only: bool,
    hovered_point: Option<(SharedString, usize)>,
    last_hovered_point: Option<(SharedString, usize)>,
    keyboard_point: usize,
    active_pick: Option<ActivePick>,
    last_pick_focus: Option<(SharedString, bool, SpendChartFocus)>,
    dragging_pick: bool,
    dragging_slider: Option<(SharedString, usize)>,
    slider_bounds: Vec<Rc<Cell<(f32, f32)>>>,
    active_artifact: Option<SpendArtifactKind>,
    copied_turn_id: Option<(SharedString, Instant)>,
    toast: Option<(SharedString, Instant)>,
    expanded_sql_turns: Vec<SharedString>,
    approval_mode: SpendApprovalMode,
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

impl Focusable for AiChatSpendScreen {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl AiChatSpendScreen {
    /// Construct an [`AiChatSpendScreen`] preloaded with the November AI spend conversation.
    pub fn new(id: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let id_str: SharedString = id.into();
        let this = cx.entity();
        let nav_groups = sample_spend_nav_groups(cx);
        let shell = cx.new(|shell_cx| {
            let mut s = AppShell::new(format!("{id_str}-shell"), shell_cx)
                .with_preset(AppShellPreset::Chat, shell_cx)
                .with_groups(nav_groups)
                .with_active("nov-ai-spend")
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
                cx.emit(AiChatSpendEvent::Navigate(label.clone()));
            }
            AppShellEvent::PanelOpen(false) => {
                cx.notify();
            }
            AppShellEvent::PanelOpen(true) => {
                if this.active_artifact.is_none() {
                    this.active_artifact = Some(SpendArtifactKind::UsageApi);
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
            turns: sample_spend_turns(),
            use_alt_split: false,
            trend_recent_only: false,
            hovered_point: None,
            last_hovered_point: None,
            keyboard_point: 0,
            active_pick: None,
            last_pick_focus: None,
            dragging_pick: false,
            dragging_slider: None,
            slider_bounds: (0..6).map(|_| Rc::new(Cell::new((0., 240.)))).collect(),
            active_artifact: None,
            copied_turn_id: None,
            toast: None,
            expanded_sql_turns: Vec::new(),
            approval_mode: SpendApprovalMode::RequestApproval,
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

    /// Render only the inner spend planner column without the surrounding [`AppShell`].
    pub fn with_standalone(mut self, standalone: bool) -> Self {
        self.standalone = standalone;
        self
    }

    /// Replace the initial conversation turns.
    pub fn with_turns(mut self, turns: Vec<SpendTurn>) -> Self {
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

    /// Open or close the right-hand usage/billing ledger inspector on mount.
    pub fn with_panel_open(mut self, open: bool, cx: &mut Context<Self>) -> Self {
        if open && self.active_artifact.is_none() {
            self.active_artifact = Some(SpendArtifactKind::UsageApi);
        }
        self.shell.update(cx, |s, cx| s.set_panel_open(open, cx));
        self
    }

    /// Configure skeleton loading mode.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Configure empty new-chat starter mode.
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

    /// Open a usage, billing, or routing-rules artifact in the right-hand [`AppShell`] panel.
    pub fn open_artifact(&mut self, kind: SpendArtifactKind, cx: &mut Context<Self>) {
        self.active_artifact = Some(kind);
        self.shell.update(cx, |s, cx| s.set_panel_open(true, cx));
        cx.notify();
    }

    /// Submit a spend question with optional chart focus (or pre-formatted quote) and append an answer.
    pub fn ask(
        &mut self,
        question: impl Into<String>,
        focus: Option<SpendChartFocus>,
        cx: &mut Context<Self>,
    ) -> bool {
        self.ask_with_quote(question, focus, None, cx)
    }

    fn ask_with_quote(
        &mut self,
        question: impl Into<String>,
        focus: Option<SpendChartFocus>,
        explicit_quote: Option<SharedString>,
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

        let quote = explicit_quote.or_else(|| {
            focus
                .as_ref()
                .map(|f| SharedString::from(format!("{} · {}", f.label, f.detail)))
        });
        let user_id: SharedString = format!("u-{}", self.turns.len() + 1).into();
        let q_shared: SharedString = trimmed.clone().into();
        self.turns.push(SpendTurn::User {
            id: user_id,
            quote,
            text: q_shared.clone(),
        });

        let ans_id: SharedString = format!("a-{}", self.turns.len() + 1).into();
        let mut built = build_spend_answer(&trimmed, focus.as_ref());
        let full_text = match &mut built {
            SpendTurn::Assistant {
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
            SpendTurn::User { text, .. } => text.clone(),
        };
        self.turns.push(built);
        if !Theme::global(cx).reduced_motion {
            self.stream_state = Some((ans_id, full_text, Instant::now()));
        } else {
            self.stream_state = None;
        }

        self.scroll.scroll_to_bottom();
        cx.emit(AiChatSpendEvent::Ask {
            question: q_shared,
            focus,
        });
        cx.notify();
        true
    }

    fn set_lever_value(
        &mut self,
        turn_id: &str,
        lever_idx: usize,
        raw_val: f64,
        cx: &mut Context<Self>,
    ) {
        let mut emitted: Option<(SharedString, f64)> = None;
        for t in &mut self.turns {
            if let SpendTurn::Assistant {
                id,
                chart: Some(SpendChart::Plan { levers, .. }),
                ..
            } = t
                && id.as_ref() == turn_id
                && let Some(lv) = levers.get_mut(lever_idx)
            {
                let next_v = match lv.kind {
                    SpendLeverKind::Range { max_pct } => {
                        let stepped = (raw_val / 5.0).round() * 5.0;
                        stepped.clamp(0.0, max_pct as f64)
                    }
                    SpendLeverKind::Switch => {
                        if raw_val >= 0.5 {
                            1.0
                        } else {
                            0.0
                        }
                    }
                };
                if (lv.value - next_v).abs() > 1e-4 {
                    lv.value = next_v;
                    emitted = Some((lv.id.clone(), next_v));
                }
            }
        }
        if let Some((id, value)) = emitted {
            cx.emit(AiChatSpendEvent::AdjustLever { id, value });
            cx.notify();
        }
    }

    fn current_focus_for_pick(&self, pick: &ActivePick) -> Option<SpendChartFocus> {
        let turn = self.turns.iter().find(|t| match t {
            SpendTurn::Assistant { id, .. } => id == &pick.turn_id,
            _ => false,
        })?;
        let SpendTurn::Assistant {
            chart: Some(chart), ..
        } = turn
        else {
            return None;
        };
        match chart {
            SpendChart::Split {
                groups, alt_groups, ..
            } => {
                let active_groups = if self.use_alt_split && !alt_groups.is_empty() {
                    alt_groups
                } else {
                    groups
                };
                let idx = pick.from.min(active_groups.len().saturating_sub(1));
                let (label, detail) = compute_split_bar_focus(active_groups, idx)?;
                Some(SpendChartFocus {
                    turn_id: pick.turn_id.clone(),
                    from: idx,
                    to: idx,
                    label: label.into(),
                    detail: detail.into(),
                })
            }
            SpendChart::Trend {
                labels,
                current,
                previous,
                ..
            } => {
                let lo = pick.from.min(pick.to).min(current.len().saturating_sub(1));
                let hi = pick.from.max(pick.to).min(current.len().saturating_sub(1));
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
                    format!("${:.1}K · {:+.1}% vs. prior", sum_cur, pct)
                } else {
                    format!("${:.1}K", sum_cur)
                };
                Some(SpendChartFocus {
                    turn_id: pick.turn_id.clone(),
                    from: lo,
                    to: hi,
                    label: label.into(),
                    detail: detail.into(),
                })
            }
            SpendChart::Plan { .. } => None,
        }
    }

    fn ensure_editors(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.composer_editor.is_none() {
            let ph: SharedString = tr(
                cx,
                "Ask about spend, models or teams…",
                "询问 AI 成本、模型占比或团队用量…",
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
            let ph: SharedString = tr(cx, "Ask about this selection…", "针对所选部分提问…").into();
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
                    if let SpendTurn::Assistant {
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
                        if let SpendTurn::Assistant { id, text, .. } = t
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

        let Some((last_id, chart_kind)) = self.turns.iter().rev().find_map(|t| match t {
            SpendTurn::Assistant {
                id, chart: Some(c), ..
            } => Some((id.clone(), c.clone())),
            _ => None,
        }) else {
            return;
        };

        match chart_kind {
            SpendChart::Plan { levers, .. } => {
                if levers.is_empty() {
                    return;
                }
                let idx = self.keyboard_point.min(levers.len() - 1);
                match key {
                    "up" | "k" => {
                        self.keyboard_point = idx.saturating_sub(1);
                        cx.notify();
                    }
                    "down" | "j" => {
                        self.keyboard_point = (idx + 1).min(levers.len() - 1);
                        cx.notify();
                    }
                    "left" => {
                        if matches!(levers[idx].kind, SpendLeverKind::Range { .. }) {
                            let next = levers[idx].value - 5.0;
                            self.set_lever_value(&last_id, idx, next, cx);
                        }
                    }
                    "right" => {
                        if matches!(levers[idx].kind, SpendLeverKind::Range { .. }) {
                            let next = levers[idx].value + 5.0;
                            self.set_lever_value(&last_id, idx, next, cx);
                        }
                    }
                    " " => {
                        if matches!(levers[idx].kind, SpendLeverKind::Switch) {
                            let next = if levers[idx].value >= 0.5 { 0.0 } else { 1.0 };
                            self.set_lever_value(&last_id, idx, next, cx);
                        }
                    }
                    _ => {}
                }
            }
            SpendChart::Split { groups, .. } => {
                let count = groups.len();
                if count == 0 {
                    return;
                }
                match key {
                    "left" | "h" => {
                        self.keyboard_point = self.keyboard_point.min(count - 1).saturating_sub(1);
                        self.hovered_point = Some((last_id, self.keyboard_point));
                        cx.notify();
                    }
                    "right" | "l" => {
                        self.keyboard_point = (self.keyboard_point + 1).min(count - 1);
                        self.hovered_point = Some((last_id, self.keyboard_point));
                        cx.notify();
                    }
                    "enter" | " " => {
                        let pt = self.keyboard_point.min(count - 1);
                        let pk = ActivePick {
                            turn_id: last_id,
                            anchor: pt,
                            from: pt,
                            to: pt,
                            is_bar: true,
                            ask_open: true,
                        };
                        if let Some(f) = self.current_focus_for_pick(&pk) {
                            cx.emit(AiChatSpendEvent::SelectChartPart(f));
                        }
                        self.active_pick = Some(pk);
                        if let Some(ed) = &self.pick_editor {
                            ed.update(cx, |e, cx| e.set_value("", w, cx));
                            {
                                let fh = ed.read(cx).focus.clone();
                                fh.focus(w, cx);
                            }
                        }
                        cx.notify();
                    }
                    _ => {}
                }
            }
            SpendChart::Trend { labels, .. } => {
                let count = labels.len();
                if count == 0 {
                    return;
                }
                match key {
                    "left" | "h" => {
                        self.keyboard_point = self.keyboard_point.min(count - 1).saturating_sub(1);
                        self.hovered_point = Some((last_id.clone(), self.keyboard_point));
                        if let Some(pk) = &mut self.active_pick
                            && pk.turn_id == last_id
                            && !pk.ask_open
                        {
                            pk.from = pk.anchor.min(self.keyboard_point);
                            pk.to = pk.anchor.max(self.keyboard_point);
                        }
                        cx.notify();
                    }
                    "right" | "l" => {
                        self.keyboard_point = (self.keyboard_point + 1).min(count - 1);
                        self.hovered_point = Some((last_id.clone(), self.keyboard_point));
                        if let Some(pk) = &mut self.active_pick
                            && pk.turn_id == last_id
                            && !pk.ask_open
                        {
                            pk.from = pk.anchor.min(self.keyboard_point);
                            pk.to = pk.anchor.max(self.keyboard_point);
                        }
                        cx.notify();
                    }
                    "enter" => {
                        let pt = self.keyboard_point.min(count - 1);
                        if let Some(mut pk) = self.active_pick.clone()
                            && pk.turn_id == last_id
                            && !pk.ask_open
                        {
                            pk.ask_open = true;
                            if let Some(f) = self.current_focus_for_pick(&pk) {
                                cx.emit(AiChatSpendEvent::SelectChartPart(f));
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
            .on_mouse_move(cx.listener(|this, ev: &MouseMoveEvent, _, cx| {
                if let Some((turn_id, li)) = this.dragging_slider.clone()
                    && let Some(bounds_cell) = this.slider_bounds.get(li)
                {
                    let (left, width) = bounds_cell.get();
                    if width > 8. {
                        let frac = ((f32::from(ev.position.x) - left) / width).clamp(0., 1.);
                        let mut max_p = 80.0_f64;
                        for t in &this.turns {
                            if let SpendTurn::Assistant {
                                id,
                                chart: Some(SpendChart::Plan { levers, .. }),
                                ..
                            } = t
                                && id == &turn_id
                                && let Some(lv) = levers.get(li)
                                && let SpendLeverKind::Range { max_pct } = lv.kind
                            {
                                max_p = max_pct as f64;
                            }
                        }
                        this.set_lever_value(&turn_id, li, frac as f64 * max_p, cx);
                    }
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, w, cx| {
                    if this.dragging_slider.take().is_some() {
                        cx.notify();
                    }
                    if this.dragging_pick {
                        this.dragging_pick = false;
                        if let Some(pk) = &mut this.active_pick {
                            pk.ask_open = true;
                        }
                        if let Some(pk) = this.active_pick.clone()
                            && let Some(f) = this.current_focus_for_pick(&pk)
                        {
                            cx.emit(AiChatSpendEvent::SelectChartPart(f));
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
        turn: SpendTurn,
        narrow: bool,
        elapsed_ms: f32,
        reduced: bool,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let p = ProPalette::get(cx);
        let spc_blue: Hsla = if p.dark {
            rgb(0x4fb0fe).into()
        } else {
            rgb(0x0190fe).into()
        };
        let spc_good: Hsla = if p.dark {
            rgb(0x7dcf9f).into()
        } else {
            rgb(0x1e7a47).into()
        };
        let spc_bad: Hsla = if p.dark {
            rgb(0xff8a7a).into()
        } else {
            rgb(0xd72400).into()
        };
        let spc_edge: Hsla = if p.dark {
            hsla(0., 0., 1., 0.09)
        } else {
            hsla(0., 0., 0., 0.09)
        };

        match turn {
            SpendTurn::User { quote, text, .. } => div()
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
                            .bg(spc_blue.opacity(0.08))
                            .border_l_2()
                            .border_color(spc_blue)
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .text_size(px(12.5))
                            .line_height(px(18.))
                            .text_color(p.text)
                            .child(pro_icon(ProIcon::Sliders, 13., spc_blue))
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

            SpendTurn::Assistant {
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
                if streaming && text.is_empty() {
                    return div()
                        .w_full()
                        .flex()
                        .items_center()
                        .gap(px(12.))
                        .when(!narrow, |d| {
                            d.child(planner_pixel_mark(mark_color, true, elapsed_ms, reduced))
                        })
                        .child(div().text_size(px(13.)).text_color(p.muted).child(tr(
                            cx,
                            "Reading your usage…",
                            "正在读取模型用量与账单…",
                        )))
                        .into_any_element();
                }

                let out_offset = if narrow { 0. } else { -28. };
                let mut out_col = div().flex().flex_col().gap(px(16.)).min_w_0();

                if !kpis.is_empty() && !streaming {
                    let kpi_el = if narrow {
                        let mut rows = div().flex().flex_col().gap(px(8.));
                        if kpis.len() == 3 {
                            rows = rows
                                .child(Self::render_kpi_card(
                                    &kpis[0], spc_edge, spc_good, spc_bad, &p,
                                ))
                                .child(
                                    div()
                                        .flex()
                                        .gap(px(8.))
                                        .child(div().flex_1().min_w_0().child(
                                            Self::render_kpi_card(
                                                &kpis[1], spc_edge, spc_good, spc_bad, &p,
                                            ),
                                        ))
                                        .child(div().flex_1().min_w_0().child(
                                            Self::render_kpi_card(
                                                &kpis[2], spc_edge, spc_good, spc_bad, &p,
                                            ),
                                        )),
                                );
                        } else {
                            for k in &kpis {
                                rows = rows.child(Self::render_kpi_card(
                                    k, spc_edge, spc_good, spc_bad, &p,
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
                                    k, spc_edge, spc_good, spc_bad, &p,
                                ))
                            }))
                    };
                    out_col = out_col.child(kpi_el);
                }

                if let Some(ch) = chart
                    && !streaming
                {
                    let chart_el = match ch {
                        SpendChart::Split {
                            title,
                            previous_total,
                            since_label,
                            groups,
                            alt_tab_labels,
                            alt_groups,
                        } => self.render_split_card(
                            &id,
                            title,
                            previous_total,
                            since_label,
                            groups,
                            alt_tab_labels,
                            alt_groups,
                            out_offset,
                            narrow,
                            spc_blue,
                            spc_edge,
                            w,
                            cx,
                        ),
                        SpendChart::Trend {
                            labels,
                            current,
                            previous,
                            mark,
                        } => self.render_trend_card(
                            &id, caption, labels, current, previous, mark, out_offset, narrow,
                            spc_blue, spc_edge, reduced, w, cx,
                        ),
                        SpendChart::Plan {
                            now_dollars,
                            months,
                            monthly_growth,
                            levers,
                        } => self.render_plan_card(
                            &id,
                            now_dollars,
                            months,
                            monthly_growth,
                            levers,
                            out_offset,
                            narrow,
                            spc_blue,
                            spc_good,
                            spc_edge,
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

                    let sql_snippet: &'static str = if id.as_ref().contains("plan") {
                        "SELECT step_kind, model, SUM(uncached_input_tokens) AS uncached_tok,\n       SUM(cost_usd) * 0.62 AS sonnet_route_savings_usd\nFROM telemetry.llm_runs WHERE month = '2026-11' GROUP BY 1, 2;"
                    } else if id.as_ref().contains("trend") {
                        "SELECT date_trunc('week', started_at) AS wk, SUM(cost_usd) / 1000.0 AS spend_k\nFROM telemetry.llm_runs WHERE started_at >= '2026-10-01' GROUP BY 1 ORDER BY 1;"
                    } else {
                        "SELECT provider, model, SUM(input_usd + cached_usd + output_usd) AS spend_usd\nFROM billing.model_usage_daily WHERE billing_month = '2026-11'\nGROUP BY 1, 2 ORDER BY spend_usd DESC;"
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
                                                        .contains("billing")
                                                    {
                                                        SpendArtifactKind::BillingInvoice
                                                    } else if src_click
                                                        .to_ascii_lowercase()
                                                        .contains("rule")
                                                    {
                                                        SpendArtifactKind::RoutingRules
                                                    } else {
                                                        SpendArtifactKind::UsageApi
                                                    };
                                                    this.open_artifact(kind, cx);
                                                    cx.emit(AiChatSpendEvent::OpenSource(
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
                                        .text_color(if sql_open { spc_blue } else { p.muted })
                                        .cursor_pointer()
                                        .child(tr(cx, "SQL & methodology", "查询与口径"))
                                        .child(pro_icon(
                                            if sql_open {
                                                ProIcon::ChevronUp
                                            } else {
                                                ProIcon::ChevronDown
                                            },
                                            11.,
                                            if sql_open { spc_blue } else { p.muted },
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
                                            tr(cx, "Copied summary to clipboard", "已复制成本分析")
                                                .into(),
                                            now,
                                        ));
                                        cx.emit(AiChatSpendEvent::CopyAnswer(tid_copy.clone()));
                                        cx.notify();
                                    },
                                ))
                                .child(Self::action_icon_btn(
                                    pro_eid(&self.id, format!("act-dl-{id}")),
                                    tr(cx, "Open spend ledger", "打开用量账单"),
                                    DOWNLOAD_ARROW_SVG,
                                    false,
                                    &p,
                                    w,
                                    cx,
                                    move |this, _, cx| {
                                        this.open_artifact(SpendArtifactKind::UsageApi, cx);
                                        this.toast = Some((
                                            tr(
                                                cx,
                                                "Opened November usage ledger",
                                                "已打开 11 月用量账本",
                                            )
                                            .into(),
                                            Instant::now(),
                                        ));
                                        cx.emit(AiChatSpendEvent::ExportLedger(tid_dl.clone()));
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
                                            if let SpendTurn::Assistant {
                                                id: tid, liked: lk, ..
                                            } = t
                                                && tid == &tid_like
                                            {
                                                *lk = !*lk;
                                                new_liked = *lk;
                                            }
                                        }
                                        cx.emit(AiChatSpendEvent::Feedback {
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
                                                if let SpendTurn::Assistant {
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
                                        cx.emit(AiChatSpendEvent::Regenerate(tid_regen.clone()));
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
                        .border_color(spc_edge)
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
                                            "Usage telemetry query · billed input/cache/output rates",
                                            "监控用量查询 · 按输入/缓存/输出实际单价折算",
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
                                        .border_color(spc_edge)
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
                        d.child(div().mt(px(3.)).child(planner_pixel_mark(
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
        kpi: &SpendKpi,
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
    fn render_split_card(
        &self,
        turn_id: &SharedString,
        title: SharedString,
        previous_total: Option<f64>,
        since_label: Option<SharedString>,
        groups: Vec<SpendSplitGroup>,
        alt_tab_labels: Option<(SharedString, SharedString)>,
        alt_groups: Vec<SpendSplitGroup>,
        out_offset: f32,
        narrow: bool,
        spc_blue: Hsla,
        spc_edge: Hsla,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let p = ProPalette::get(cx);
        let has_alt = !alt_groups.is_empty() && alt_tab_labels.is_some();
        let using_alt = self.use_alt_split && has_alt;
        let active_groups = if using_alt { alt_groups } else { groups };

        let grand_total: f64 = active_groups.iter().map(SpendSplitGroup::total).sum();
        let max_col = active_groups
            .iter()
            .map(SpendSplitGroup::total)
            .fold(1.0_f64, f64::max);
        let y_max = if max_col <= 40_000.0 {
            40_000.0
        } else {
            ((max_col / 10_000.0).ceil() * 10_000.0).max(20_000.0)
        };

        let change_pill = previous_total.and_then(|prev| {
            if prev <= 0.0 {
                return None;
            }
            let pct = ((grand_total - prev) / prev * 100.0).round() as i32;
            let sign = if pct >= 0 { "+" } else { "" };
            let label = match &since_label {
                Some(s) => format!("{sign}{pct}% since {s}"),
                None => format!("{sign}{pct}%"),
            };
            let up_bg: Hsla = if p.dark {
                hsla(145. / 360., 0.46, 0.58, 0.14)
            } else {
                hsla(143. / 360., 0.69, 0.39, 0.10)
            };
            let up_fg: Hsla = if p.dark {
                rgb(0x5fd08f).into()
            } else {
                rgb(0x1a7f45).into()
            };
            Some(
                div()
                    .h(px(22.))
                    .px(px(6.))
                    .rounded(px(6.))
                    .bg(up_bg)
                    .flex()
                    .items_center()
                    .text_size(px(12.5))
                    .text_color(up_fg)
                    .child(label),
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
                .render_pick_ask_box(focus, ed, spc_blue, spc_edge, cx)
                .into_any_element();
            Some(pro_presence(
                pro_eid(&self.id, format!("sp-bar-ask-pres-{turn_id}")),
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

        // Presence-animated floating tooltip when hovering a provider/team column
        let tip_open = bar_pick.is_none() && hovered_bar.is_some();
        let tip_idx = hovered_bar
            .or_else(|| {
                self.last_hovered_point
                    .as_ref()
                    .filter(|(tid, idx)| tid == turn_id && *idx < active_groups.len())
                    .map(|(_, idx)| *idx)
            })
            .unwrap_or(0);
        let hover_tip = active_groups.get(tip_idx).map(|grp| {
            let tot = grp.total();
            let base_c = grp.tone.resolve(&p);
            let logo_el = grp
                .logo
                .as_ref()
                .and_then(|lg| pro_app_artwork(lg, 13., cx));
            let on_right = tip_idx == 0;
            let tip_card = div()
                .min_w(px(214.))
                .px(px(12.))
                .py(px(10.))
                .rounded(px(10.))
                .bg(if p.dark { rgb(0x232326).into() } else { p.card })
                .border_1()
                .border_color(p.line_strong)
                .flex()
                .flex_col()
                .gap(px(5.))
                .child(
                    div()
                        .mb(px(2.))
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .text_size(px(12.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.ink)
                        .when_some(logo_el, |d, m| d.child(m))
                        .child(grp.name.clone()),
                )
                .children(grp.parts.iter().enumerate().map(|(si, part)| {
                    let ratio = if tot > 0.0 {
                        (part.value / tot).clamp(0.04, 1.0) as f32
                    } else {
                        0.04
                    };
                    let pct = if tot > 0.0 {
                        (part.value / tot * 100.0).round() as i32
                    } else {
                        0
                    };
                    let shade = match si {
                        0 => 1.0,
                        1 => 0.58,
                        _ => 0.34,
                    };
                    let fill_c = base_c.opacity(shade);
                    div()
                        .flex()
                        .items_center()
                        .gap(px(7.))
                        .text_size(px(12.))
                        .child(div().size(px(8.)).rounded(px(2.)).bg(fill_c))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .text_color(p.text)
                                .child(part.name.clone()),
                        )
                        .child(div().w(px(34.)).flex_none().child(pro_spring_bar(
                            pro_eid(&self.id, format!("sp-tip-bar-{turn_id}-{si}")),
                            ratio,
                            3.5,
                            2.,
                            p.chip,
                            fill_c,
                        )))
                        .child(div().text_color(p.muted).child(format!("{pct}%")))
                        .child(
                            div()
                                .min_w(px(44.))
                                .text_right()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.strong)
                                .child(format_spend_short(part.value)),
                        )
                }))
                .child(
                    div()
                        .mt(px(4.))
                        .pt(px(5.))
                        .border_t_1()
                        .border_color(p.line)
                        .flex()
                        .items_center()
                        .justify_between()
                        .text_size(px(12.))
                        .child(div().text_color(p.muted).child(tr(cx, "Total", "合计")))
                        .child(
                            div()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.strong)
                                .child(format_dollars_comma(tot)),
                        ),
                )
                .into_any_element();
            pro_presence(
                pro_eid(&self.id, format!("sp-tip-pres-{turn_id}")),
                tip_open,
                0.16,
                0.12,
                move |amt| {
                    let e = pro_ease(amt);
                    div()
                        .absolute()
                        .top(px(28. + 4. * (1. - e)))
                        .when(on_right, |d| d.right(px(16.)))
                        .when(!on_right, |d| d.left(px(16.)))
                        .opacity(e)
                        .child(tip_card)
                        .into_any_element()
                },
            )
        });

        let grid_line_c = if p.dark {
            hsla(0., 0., 1., 0.08)
        } else {
            hsla(0., 0., 0., 0.07)
        };

        let mut bar_els = Vec::with_capacity(active_groups.len());
        for (gi, grp) in active_groups.iter().enumerate() {
            let tot = grp.total();
            let target_bar_h = ((tot / y_max) * 200.0).clamp(5.0, 194.0) as f32;
            let bar_h = pro_spring_f32(
                pro_eid(&self.id, format!("pbar-h-{turn_id}-{gi}")),
                target_bar_h,
                0.28,
                w,
                cx,
            )
            .clamp(4.0, 196.0);

            let base_c = grp.tone.resolve(&p);
            let picked = bar_pick.as_ref().is_some_and(|pk| pk.from == gi);
            let dimmed = hovered_bar.is_some_and(|hi| hi != gi) && !picked;
            let anim_op = pro_spring_f32(
                pro_eid(&self.id, format!("pbar-op-{turn_id}-{gi}")),
                if dimmed { 0.35 } else { 1.0 },
                0.18,
                w,
                cx,
            )
            .clamp(0.2, 1.0);
            let anim_lift = pro_spring_f32(
                pro_eid(&self.id, format!("pbar-lift-{turn_id}-{gi}")),
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
                    pro_eid(&self.id, format!("pbar-seg-{turn_id}-{gi}-{si}")),
                    target_sh,
                    0.28,
                    w,
                    cx,
                )
                .max(2.0);
                let shade = match si {
                    0 => 1.0,
                    1 => 0.52,
                    _ => 0.30,
                };
                seg_els.push(div().w_full().h(px(sh)).bg(base_c.opacity(shade)));
            }

            let tid_h = turn_id.clone();
            let tid_c = turn_id.clone();
            bar_els.push(activate(
                div()
                    .id(pro_eid(&self.id, format!("pbar-{turn_id}-{gi}")))
                    .role(Role::Button)
                    .tab_index(0)
                    .relative()
                    .top(px(-2.0 * anim_lift))
                    .flex_1()
                    .h_full()
                    .rounded(px(8.))
                    .when(picked, |d| d.border_1().border_color(spc_blue))
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
                            this.keyboard_point = gi;
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
                            .line_height(px(16.))
                            .text_color(if picked || hovered_bar == Some(gi) {
                                p.strong
                            } else {
                                p.muted
                            })
                            .child(format_bar_top(tot)),
                    )
                    .child(
                        div()
                            .w(px(if narrow { 44. } else { 64. }))
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
                        cx.emit(AiChatSpendEvent::SelectChartPart(f));
                    }
                    this.active_pick = Some(pk);
                    if let Some(ed) = &this.pick_editor {
                        ed.update(cx, |e, cx| e.set_value("", w, cx));
                        let fh = ed.read(cx).focus.clone();
                        fh.focus(w, cx);
                    }
                    cx.notify();
                },
            ));
        }

        let tab_key: SharedString = if using_alt {
            "alt".into()
        } else {
            "primary".into()
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
                            .h(px(200.))
                            .w(px(36.))
                            .flex_none()
                            .flex()
                            .flex_col()
                            .justify_between()
                            .items_end()
                            .text_size(px(12.))
                            .text_color(p.muted)
                            .child(format_k_axis(y_max))
                            .child(format_k_axis(y_max * 0.5))
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
                                    .h(px(200.))
                                    .w_full()
                                    .child(
                                        div()
                                            .absolute()
                                            .top_0()
                                            .left_0()
                                            .right_0()
                                            .h(px(1.))
                                            .bg(grid_line_c),
                                    )
                                    .child(
                                        div()
                                            .absolute()
                                            .top(px(100.))
                                            .left_0()
                                            .right_0()
                                            .h(px(1.))
                                            .bg(grid_line_c),
                                    )
                                    .child(
                                        div()
                                            .absolute()
                                            .bottom_0()
                                            .left_0()
                                            .right_0()
                                            .h(px(1.))
                                            .bg(grid_line_c),
                                    )
                                    .child(
                                        div()
                                            .relative()
                                            .size_full()
                                            .px(px(8.))
                                            .flex()
                                            .items_end()
                                            .gap(px(12.))
                                            .children(bar_els),
                                    )
                                    .when_some(hover_tip, |d, tip| d.child(tip)),
                            )
                            .child(div().mt(px(10.)).px(px(8.)).flex().gap(px(12.)).children(
                                active_groups.iter().enumerate().map(|(gi, grp)| {
                                    let active = hovered_bar == Some(gi)
                                        || bar_pick.as_ref().is_some_and(|pk| pk.from == gi);
                                    let dimmed = hovered_bar.is_some_and(|hi| hi != gi) && !active;
                                    let logo = grp
                                        .logo
                                        .as_ref()
                                        .and_then(|lg| pro_app_artwork(lg, 13., cx));
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .when(dimmed, |d| d.opacity(0.5))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .gap(px(5.))
                                        .text_size(px(12.))
                                        .font_weight(if active {
                                            FontWeight::MEDIUM
                                        } else {
                                            FontWeight::NORMAL
                                        })
                                        .text_color(if active { p.strong } else { p.text })
                                        .when_some(logo, |d, m| d.child(m))
                                        .when(!narrow, |d| {
                                            d.child(div().truncate().child(grp.name.clone()))
                                        })
                                }),
                            )),
                    ),
            )
            .child(
                div()
                    .mt(px(14.))
                    .pt(px(12.))
                    .border_t_1()
                    .border_color(p.line)
                    .flex()
                    .flex_wrap()
                    .gap(px(12.))
                    .children(active_groups.iter().enumerate().map(|(gi, grp)| {
                        let gtot = grp.total();
                        let ratio = if grand_total > 0.0 {
                            (gtot / grand_total).clamp(0.04, 1.0) as f32
                        } else {
                            0.04
                        };
                        let pct = if grand_total > 0.0 {
                            (gtot / grand_total * 100.0).round() as i32
                        } else {
                            0
                        };
                        let c = grp.tone.resolve(&p);
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
                                    .child(grp.name.clone()),
                            )
                            .child(div().w(px(48.)).flex_none().child(pro_spring_bar(
                                pro_eid(&self.id, format!("sp-share-bar-{turn_id}-{gi}")),
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
                    })),
            );

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
                    .border_color(spc_edge)
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .mb(px(28.))
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
                                    .when_some(change_pill, |d, pill| d.child(pill)),
                            )
                            .when_some(alt_tab_labels, |d, (t0, t1)| {
                                let tab_sensor = pro_selection_sensor();
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
                                            pro_eid(&self.id, format!("sp-tab-pill-{turn_id}")),
                                            tab_key.clone(),
                                            tab_sensor.clone(),
                                            6.,
                                            p.card,
                                            Some(p.line),
                                            !p.dark,
                                        ))
                                        .children([(false, t0), (true, t1)].into_iter().map(
                                            |(alt_flag, lbl)| {
                                                let on = using_alt == alt_flag;
                                                let sensor = tab_sensor.clone();
                                                activate(
                                                    div()
                                                        .id(pro_eid(
                                                            &self.id,
                                                            format!("sp-tab-{turn_id}-{alt_flag}"),
                                                        ))
                                                        .role(Role::Tab)
                                                        .tab_index(0)
                                                        .relative()
                                                        .h(px(24.))
                                                        .px(px(10.))
                                                        .rounded(px(6.))
                                                        .when(!on, |d| {
                                                            d.hover(move |s| s.text_color(p.ink))
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
                                                        this.use_alt_split = alt_flag;
                                                        this.active_pick = None;
                                                        cx.notify();
                                                    },
                                                )
                                            },
                                        )),
                                )
                            }),
                    )
                    .when_some(ask_popover, |d, pop| d.child(pop))
                    .child(pro_view_transition(
                        pro_eid(&self.id, format!("sp-split-vtx-{turn_id}")),
                        tab_key,
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

    #[allow(clippy::too_many_arguments)]
    fn render_trend_card(
        &self,
        turn_id: &SharedString,
        caption: Option<SharedString>,
        labels: Vec<SharedString>,
        current: Vec<f64>,
        previous: Vec<f64>,
        mark: Option<usize>,
        out_offset: f32,
        narrow: bool,
        spc_blue: Hsla,
        spc_edge: Hsla,
        reduced: bool,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let p = ProPalette::get(cx);
        let can_zoom = current.len() > 4;
        let offset = if self.trend_recent_only && can_zoom {
            current.len() - 4
        } else {
            0
        };
        let disp_labels: Vec<SharedString> = labels[offset..].to_vec();
        let disp_cur: Vec<f64> = current[offset..].to_vec();
        let disp_prev: Vec<f64> = if offset < previous.len() {
            previous[offset..].to_vec()
        } else {
            Vec::new()
        };
        let disp_mark = mark.and_then(|m| m.checked_sub(offset));

        let n = disp_cur.len();
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
                .filter(|(tid, idx)| tid == turn_id && *idx >= offset && (*idx - offset) < n)
                .map(|(_, idx)| *idx - offset)
                .or_else(|| disp_mark.filter(|&m| m < n))
        };

        let prev_stroke = if p.dark {
            hsla(0., 0., 1., 0.34)
        } else {
            hsla(0., 0., 0., 0.28)
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

        let min_v = 4.0_f32;
        let max_v = 16.0_f32;
        let span = max_v - min_v;
        let denom = (n.saturating_sub(1)).max(1) as f32;

        let mut cur_norm = Vec::with_capacity(n);
        for (i, &v) in disp_cur.iter().enumerate() {
            let target_ny = ((v as f32 - min_v) / span).clamp(0., 1.);
            cur_norm.push(
                pro_spring_f32(
                    pro_eid(&self.id, format!("sp-tr-cy-{turn_id}-{i}")),
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
            let target_ny = ((v as f32 - min_v) / span).clamp(0., 1.);
            prev_norm.push(
                pro_spring_f32(
                    pro_eid(&self.id, format!("sp-tr-py-{turn_id}-{i}")),
                    target_ny,
                    0.28,
                    w,
                    cx,
                )
                .clamp(0., 1.),
            );
        }

        let marker_ai = active_idx.unwrap_or_else(|| n.saturating_sub(1));
        let marker_tx = pro_spring_f32(
            pro_eid(&self.id, format!("sp-tr-mtx-{turn_id}")),
            marker_ai as f32 / denom,
            0.18,
            w,
            cx,
        )
        .clamp(0., 1.);
        let marker_cy = pro_spring_f32(
            pro_eid(&self.id, format!("sp-tr-mcy-{turn_id}")),
            cur_norm.get(marker_ai).copied().unwrap_or(0.5),
            0.18,
            w,
            cx,
        )
        .clamp(0., 1.);

        let pick_range_spring = active_pick.as_ref().map(|pk| {
            let lo_rel = pk
                .from
                .min(pk.to)
                .saturating_sub(offset)
                .min(n.saturating_sub(1));
            let hi_rel = pk
                .from
                .max(pk.to)
                .saturating_sub(offset)
                .min(n.saturating_sub(1));
            let s_lo = pro_spring_f32(
                pro_eid(&self.id, format!("sp-tr-blo-{turn_id}")),
                lo_rel as f32 / denom,
                0.16,
                w,
                cx,
            )
            .clamp(0., 1.);
            let s_hi = pro_spring_f32(
                pro_eid(&self.id, format!("sp-tr-bhi-{turn_id}")),
                hi_rel as f32 / denom,
                0.16,
                w,
                cx,
            )
            .clamp(0., 1.);
            (s_lo, s_hi, lo_rel == hi_rel)
        });

        let reveal = if reduced {
            1.0
        } else {
            pro_ease((self.boot_at.elapsed().as_secs_f32() / 0.8).clamp(0., 1.))
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
                    window.paint_quad(fill(band, spc_blue.opacity(0.11)));
                }

                for (i, pair) in prev_norm.windows(2).enumerate() {
                    paint_dashed_segment(
                        window,
                        pt_at(i, pair[0]),
                        pt_at(i + 1, pair[1]),
                        prev_stroke,
                        1.25,
                        3.0,
                        3.0,
                    );
                }

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
                            linear_color_stop(spc_blue.opacity(0.20), 0.),
                            linear_color_stop(spc_blue.opacity(0.0), 1.),
                        ),
                    );
                }

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
                    window.paint_path(path, spc_blue);
                }

                if show_marker && marker_ai < vis_n {
                    let cur_pt = point(
                        bounds.left() + px(marker_tx * bw),
                        bounds.top() + px((1. - marker_cy) * bh),
                    );
                    let mut g_pb = PathBuilder::stroke(px(1.));
                    g_pb.move_to(point(cur_pt.x, bounds.top()));
                    g_pb.line_to(point(cur_pt.x, bounds.bottom()));
                    if let Ok(path) = g_pb.build() {
                        window.paint_path(path, guide_stroke);
                    }
                    paint_ringed_dot(window, cur_pt, 4.0, 2.0, spc_blue, card_bg);
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
                .render_pick_ask_box(focus, ed, spc_blue, spc_edge, cx)
                .into_any_element();
            Some(pro_presence(
                pro_eid(&self.id, format!("sp-tr-ask-pres-{turn_id}")),
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
            let abs_pi = offset + pi;
            let tid_h = turn_id.clone();
            let tid_d = turn_id.clone();
            div()
                .id(pro_eid(&self.id, format!("spt-{turn_id}-{pi}")))
                .flex_1()
                .h_full()
                .cursor_crosshair()
                .on_hover(cx.listener(move |this, over: &bool, _, cx| {
                    if *over {
                        this.hovered_point = Some((tid_h.clone(), abs_pi));
                        this.keyboard_point = abs_pi;
                        if this.dragging_pick
                            && let Some(pk) = &mut this.active_pick
                            && pk.turn_id == tid_h
                        {
                            pk.from = pk.anchor.min(abs_pi);
                            pk.to = pk.anchor.max(abs_pi);
                        }
                        cx.notify();
                    } else if !this.dragging_pick
                        && this
                            .hovered_point
                            .as_ref()
                            .is_some_and(|(t, idx)| t == &tid_h && *idx == abs_pi)
                    {
                        this.hovered_point = None;
                        cx.notify();
                    }
                }))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _, _, cx| {
                        this.dragging_pick = true;
                        this.active_pick = Some(ActivePick {
                            turn_id: tid_d.clone(),
                            anchor: abs_pi,
                            from: abs_pi,
                            to: abs_pi,
                            is_bar: false,
                            ask_open: false,
                        });
                        cx.notify();
                    }),
                )
        }));

        let readout = active_idx.and_then(|ai| {
            Some(format!(
                "{} · ${:.1}K",
                disp_labels.get(ai)?,
                disp_cur.get(ai)?
            ))
        });

        let range_key: SharedString = if self.trend_recent_only && can_zoom {
            "4w".into()
        } else {
            "8w".into()
        };

        let range_pill = (!narrow && can_zoom).then(|| {
            let rng_sensor = pro_selection_sensor();
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
                    pro_eid(&self.id, format!("sp-rng-pill-{turn_id}")),
                    range_key.clone(),
                    rng_sensor.clone(),
                    999.,
                    p.card,
                    Some(p.line),
                    !p.dark,
                ))
                .children(
                    [
                        (false, tr(cx, "All 8W", "近 8 周")),
                        (true, tr(cx, "Last 4W", "近 4 周")),
                    ]
                    .into_iter()
                    .map(|(recent, lbl)| {
                        let on = self.trend_recent_only == recent;
                        let sensor = rng_sensor.clone();
                        activate(
                            div()
                                .id(pro_eid(&self.id, format!("sp-rng-{turn_id}-{recent}")))
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
                                this.trend_recent_only = recent;
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
                    .child("$16K")
                    .child("$10K")
                    .child("$4K"),
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
                            .child(disp_labels.first().cloned().unwrap_or_default())
                            .child(disp_labels.get(n / 2).cloned().unwrap_or_default())
                            .child(disp_labels.last().cloned().unwrap_or_default()),
                    ),
            );

        div()
            .ml(px(out_offset))
            .px(px(if narrow { 10. } else { 14. }))
            .pt(px(14.))
            .pb(px(10.))
            .rounded(px(14.))
            .bg(p.card)
            .border_1()
            .border_color(spc_edge)
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
                            .when_some(readout, |d, r| {
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
                                    .child(div().w(px(10.)).h(px(3.)).rounded(px(1.5)).bg(spc_blue))
                                    .child(tr(cx, "November", "11 月")),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(5.))
                                    .child(
                                        div().w(px(10.)).h(px(2.5)).rounded(px(1.)).bg(prev_stroke),
                                    )
                                    .child(tr(cx, "October", "10 月")),
                            )
                            .when_some(range_pill, |d, pill| d.child(pill)),
                    ),
            )
            .when_some(ask_popover, |d, pop| d.child(pop))
            .child(pro_view_transition(
                pro_eid(&self.id, format!("sp-tr-vtx-{turn_id}")),
                range_key,
                plot_row,
            ))
            .child(div().text_size(px(12.)).text_color(p.faint).child(tr(
                cx,
                "Drag across weeks to ask about that stretch",
                "横向拖动即可针对所选周区间追问",
            )))
    }

    #[allow(clippy::too_many_arguments)]
    fn render_plan_card(
        &self,
        turn_id: &SharedString,
        now_dollars: f64,
        months: Vec<SharedString>,
        monthly_growth: f64,
        levers: Vec<SpendLever>,
        out_offset: f32,
        narrow: bool,
        spc_blue: Hsla,
        spc_good: Hsla,
        spc_edge: Hsla,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let p = ProPalette::get(cx);
        let (monthly_save, six_mo_save, baseline_k, planned_k) =
            compute_plan_projection(now_dollars, months.len(), monthly_growth, &levers);

        let base_stroke = if p.dark {
            hsla(0., 0., 1., 0.34)
        } else {
            hsla(0., 0., 0., 0.28)
        };
        let grid_stroke = if p.dark {
            hsla(0., 0., 1., 0.08)
        } else {
            hsla(0., 0., 0., 0.06)
        };
        let card_bg = p.card;
        let base_f32: Vec<f32> = baseline_k.iter().map(|&v| v as f32).collect();
        let mut plan_f32: Vec<f32> = Vec::with_capacity(planned_k.len());
        for (mi, &pv) in planned_k.iter().enumerate() {
            plan_f32.push(pro_spring_f32(
                pro_eid(&self.id, format!("plan-pt-{turn_id}-{mi}")),
                pv as f32,
                0.28,
                w,
                cx,
            ));
        }

        let forecast_canvas = canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let bw = f32::from(bounds.size.width);
                let bh = f32::from(bounds.size.height);
                if bw <= 8. || bh <= 8. || plan_f32.len() < 2 {
                    return;
                }
                let min_v = 20.0_f32;
                let max_v = 80.0_f32;
                let span = max_v - min_v;
                let pt_at = |i: usize, v: f32| -> Point<Pixels> {
                    let tx = i as f32 / (plan_f32.len() - 1) as f32;
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

                // Dashed baseline trajectory without changes
                for (i, pair) in base_f32.windows(2).enumerate() {
                    paint_dashed_segment(
                        window,
                        pt_at(i, pair[0]),
                        pt_at(i + 1, pair[1]),
                        base_stroke,
                        1.3,
                        3.5,
                        3.0,
                    );
                }

                // Soft fill + solid line for spring-morphed planned spend trajectory
                let mut area = PathBuilder::fill();
                let first = pt_at(0, plan_f32[0]);
                let last = pt_at(plan_f32.len() - 1, plan_f32[plan_f32.len() - 1]);
                area.move_to(point(first.x, bounds.bottom()));
                area.line_to(first);
                for (i, &v) in plan_f32.iter().enumerate().skip(1) {
                    area.line_to(pt_at(i, v));
                }
                area.line_to(point(last.x, bounds.bottom()));
                area.close();
                if let Ok(path) = area.build() {
                    window.paint_path(
                        path,
                        linear_gradient(
                            180.,
                            linear_color_stop(spc_blue.opacity(0.18), 0.),
                            linear_color_stop(spc_blue.opacity(0.0), 1.),
                        ),
                    );
                }

                let mut line = PathBuilder::stroke(px(1.75));
                for (i, &v) in plan_f32.iter().enumerate() {
                    let pt = pt_at(i, v);
                    if i == 0 {
                        line.move_to(pt);
                    } else {
                        line.line_to(pt);
                    }
                }
                if let Ok(path) = line.build() {
                    window.paint_path(path, spc_blue);
                }
                paint_ringed_dot(window, last, 4.0, 2.0, spc_blue, card_bg);
            },
        )
        .size_full();

        let summary_quote: SharedString = {
            let parts: Vec<String> = levers
                .iter()
                .filter(|l| l.is_active())
                .map(|l| match l.kind {
                    SpendLeverKind::Range { .. } => format!("{:.0}% {}", l.value, l.label),
                    SpendLeverKind::Switch => l.label.to_string(),
                })
                .collect();
            format!(
                "Plan · -${:.1}K/mo ({})",
                monthly_save / 1000.0,
                if parts.is_empty() {
                    "no changes".to_string()
                } else {
                    parts.join(", ")
                }
            )
            .into()
        };

        let mut lever_els = Vec::with_capacity(levers.len());
        for (li, lv) in levers.into_iter().enumerate() {
            let save = lv.monthly_savings();
            let active = lv.is_active();
            let save_lbl = if active {
                format!("-${:.1}K/mo", save / 1000.0)
            } else {
                "$0/mo".to_string()
            };
            let tid = turn_id.clone();

            match lv.kind {
                SpendLeverKind::Range { max_pct } => {
                    let pct_ratio = (lv.value / max_pct.max(1) as f64).clamp(0.0, 1.0) as f32;
                    let is_dragging = self
                        .dragging_slider
                        .as_ref()
                        .is_some_and(|(t, idx)| t == &tid && *idx == li);
                    let anim_ratio = pro_spring_f32(
                        pro_eid(&self.id, format!("lever-fill-{tid}-{li}")),
                        pct_ratio,
                        if is_dragging { 0.09 } else { 0.24 },
                        w,
                        cx,
                    )
                    .clamp(0., 1.);
                    let thumb_scale = pro_spring_f32(
                        pro_eid(&self.id, format!("lever-th-{tid}-{li}")),
                        if is_dragging { 1.12 } else { 1.0 },
                        0.15,
                        w,
                        cx,
                    )
                    .clamp(0.9, 1.25);
                    let thumb_sz = 16.0 * thumb_scale;

                    let bounds_cell = self
                        .slider_bounds
                        .get(li)
                        .cloned()
                        .unwrap_or_else(|| Rc::new(Cell::new((0., 240.))));
                    let bounds_read = bounds_cell.clone();
                    let tid_drag = tid.clone();

                    lever_els.push(
                        div()
                            .py(px(14.))
                            .when(li > 0, |d| d.border_t_1().border_color(spc_edge))
                            .flex()
                            .flex_col()
                            .gap(px(10.))
                            .child(
                                div()
                                    .flex()
                                    .items_start()
                                    .justify_between()
                                    .gap(px(16.))
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_w_0()
                                            .flex()
                                            .flex_col()
                                            .gap(px(2.))
                                            .child(
                                                div()
                                                    .text_size(px(13.5))
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .text_color(p.ink)
                                                    .child(lv.label.clone()),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(12.5))
                                                    .text_color(p.muted)
                                                    .child(lv.detail),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(13.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(if active { spc_good } else { p.faint })
                                            .child(save_lbl),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(12.))
                                    .child(
                                        div()
                                            .id(pro_eid(&self.id, format!("lever-rng-{tid}-{li}")))
                                            .role(Role::Slider)
                                            .aria_label(lv.label)
                                            .tab_index(0)
                                            .relative()
                                            .flex_1()
                                            .h(px(20.))
                                            .flex()
                                            .items_center()
                                            .cursor_pointer()
                                            .on_mouse_down(
                                                MouseButton::Left,
                                                cx.listener(
                                                    move |this, ev: &MouseDownEvent, _, cx| {
                                                        let (left, width) = bounds_read.get();
                                                        if width > 8. {
                                                            let frac = ((f32::from(ev.position.x)
                                                                - left)
                                                                / width)
                                                                .clamp(0., 1.);
                                                            this.set_lever_value(
                                                                &tid_drag,
                                                                li,
                                                                frac as f64 * max_pct as f64,
                                                                cx,
                                                            );
                                                        }
                                                        this.dragging_slider =
                                                            Some((tid_drag.clone(), li));
                                                        this.keyboard_point = li;
                                                        cx.notify();
                                                    },
                                                ),
                                            )
                                            .child(
                                                canvas(
                                                    move |b, _, _| {
                                                        bounds_cell.set((
                                                            f32::from(b.left()),
                                                            f32::from(b.size.width),
                                                        ));
                                                    },
                                                    |_, _, _, _| (),
                                                )
                                                .absolute()
                                                .inset_0(),
                                            )
                                            .child(
                                                div()
                                                    .w_full()
                                                    .h(px(4.))
                                                    .rounded(px(2.))
                                                    .bg(p.ink.opacity(0.10))
                                                    .child(
                                                        div()
                                                            .h_full()
                                                            .w(relative(anim_ratio))
                                                            .rounded(px(2.))
                                                            .bg(spc_blue),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .absolute()
                                                    .left(relative(anim_ratio))
                                                    .ml(px(-thumb_sz * 0.5))
                                                    .size(px(thumb_sz))
                                                    .rounded_full()
                                                    .bg(rgb(0xffffff))
                                                    .border_1()
                                                    .border_color(hsla(0., 0., 0., 0.20)),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .w(px(36.))
                                            .text_right()
                                            .text_size(px(12.5))
                                            .text_color(p.text)
                                            .child(format!("{:.0}%", lv.value)),
                                    ),
                            )
                            .into_any_element(),
                    );
                }
                SpendLeverKind::Switch => {
                    let checked = lv.value >= 0.5;
                    let tid_sw = tid.clone();
                    lever_els.push(
                        div()
                            .py(px(14.))
                            .when(li > 0, |d| d.border_t_1().border_color(spc_edge))
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(14.))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .gap(px(2.))
                                    .child(
                                        div()
                                            .text_size(px(13.5))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.ink)
                                            .child(lv.label.clone()),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(12.5))
                                            .text_color(p.muted)
                                            .child(lv.detail),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(if active { spc_good } else { p.faint })
                                    .child(save_lbl),
                            )
                            .child(pro_switch(
                                pro_eid(&self.id, format!("lever-sw-{tid}-{li}")),
                                lv.label,
                                checked,
                                cx,
                                move |this, _, cx| {
                                    this.keyboard_point = li;
                                    this.set_lever_value(
                                        &tid_sw,
                                        li,
                                        if checked { 0.0 } else { 1.0 },
                                        cx,
                                    );
                                },
                            ))
                            .into_any_element(),
                    );
                }
            }
        }

        let save_capture_ratio = if now_dollars > 0.0 {
            (monthly_save / (now_dollars * 0.50)).clamp(0.0, 1.0) as f32
        } else {
            0.0
        };

        div()
            .ml(px(out_offset))
            .flex()
            .flex_col()
            .gap(px(12.))
            .child(
                div()
                    .px(px(if narrow { 14. } else { 16. }))
                    .py(px(4.))
                    .rounded(px(14.))
                    .bg(p.card)
                    .border_1()
                    .border_color(spc_edge)
                    .flex()
                    .flex_col()
                    .children(lever_els),
            )
            // Live 6-month forecast card following the levers
            .child(
                div()
                    .px(px(14.))
                    .pt(px(14.))
                    .pb(px(10.))
                    .rounded(px(14.))
                    .bg(p.card)
                    .border_1()
                    .border_color(spc_edge)
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
                            .child(div().text_size(px(12.5)).text_color(p.muted).child(tr(
                                cx,
                                "Six-month spend forecast",
                                "未来六个月成本预测",
                            )))
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
                                            .gap(px(5.))
                                            .child(
                                                div()
                                                    .w(px(10.))
                                                    .h(px(3.))
                                                    .rounded(px(1.5))
                                                    .bg(spc_blue),
                                            )
                                            .child(tr(cx, "With plan", "应用方案后")),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(5.))
                                            .child(
                                                div()
                                                    .w(px(10.))
                                                    .h(px(2.5))
                                                    .rounded(px(1.))
                                                    .bg(base_stroke),
                                            )
                                            .child(tr(cx, "Current trajectory", "原增长趋势")),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .gap(px(10.))
                            .child(
                                div()
                                    .h(px(136.))
                                    .flex_none()
                                    .flex()
                                    .flex_col()
                                    .justify_between()
                                    .items_end()
                                    .text_size(px(11.))
                                    .text_color(p.muted)
                                    .child("$80K")
                                    .child("$50K")
                                    .child("$20K"),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .gap(px(8.))
                                    .child(div().w_full().h(px(136.)).child(forecast_canvas))
                                    .child(
                                        div()
                                            .w_full()
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .text_size(px(11.))
                                            .text_color(p.muted)
                                            .children(months.into_iter().map(|m| div().child(m))),
                                    ),
                            ),
                    ),
            )
            // Plan summary + spring savings meter + "Turn into rules" button
            .child(
                div()
                    .px(px(2.))
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .justify_between()
                    .gap(px(12.))
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .gap(px(8.))
                            .text_size(px(13.))
                            .text_color(p.muted)
                            .child(tr(cx, "Saves", "预计节省"))
                            .child(
                                div()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .child(format!("${:.1}K/mo", monthly_save / 1000.0)),
                            )
                            .child(format!("· ${:.0}K over 6 months", six_mo_save / 1000.0))
                            .child(div().w(px(64.)).flex_none().child(pro_spring_bar(
                                pro_eid(&self.id, format!("plan-save-bar-{turn_id}")),
                                save_capture_ratio,
                                4.5,
                                2.25,
                                p.chip,
                                spc_good,
                            ))),
                    )
                    .child(activate(
                        div()
                            .id(pro_eid(&self.id, format!("apply-plan-{turn_id}")))
                            .role(Role::Button)
                            .tab_index(0)
                            .h(px(30.))
                            .px(px(14.))
                            .rounded(px(9.))
                            .bg(p.primary)
                            .hover(move |s| s.opacity(0.88))
                            .flex()
                            .items_center()
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.on_primary)
                            .cursor_pointer()
                            .child(tr(cx, "Turn into rules", "生成路由规则")),
                        cx,
                        move |this, _, cx| {
                            cx.emit(AiChatSpendEvent::ApplyPlan {
                                monthly_savings: monthly_save,
                            });
                            this.toast = Some((
                                tr(
                                    cx,
                                    "Staged 3 routing rules in model-routing.toml",
                                    "已生成 3 条模型路由规则",
                                )
                                .into(),
                                Instant::now(),
                            ));
                            this.open_artifact(SpendArtifactKind::RoutingRules, cx);
                            this.ask_with_quote(
                                "Turn this plan into routing rules",
                                None,
                                Some(summary_quote.clone()),
                                cx,
                            );
                        },
                    )),
            )
    }

    fn render_pick_ask_box(
        &self,
        focus: SpendChartFocus,
        editor: Entity<Editor>,
        spc_blue: Hsla,
        spc_edge: Hsla,
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
            .border_color(spc_edge)
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(10.))
            .child(
                div()
                    .px(px(8.))
                    .py(px(2.))
                    .rounded(px(6.))
                    .bg(spc_blue.opacity(0.12))
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(spc_blue)
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
                        format!("What drives {} spend?", focus_submit.label)
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
                tr(cx, "Close", "关闭"),
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
                .id(pro_eid(&self.id, "spc-add-menu"))
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
                    ["Claude", "OpenAI", "Google Drive", "Slack"]
                        .into_iter()
                        .enumerate()
                        .map(|(ai, app)| {
                            activate(
                                div()
                                    .id(pro_eid(&self.id, format!("spc-add-{ai}")))
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
                    SpendApprovalMode::RequestApproval => "req".into(),
                    SpendApprovalMode::AutoReads => "reads".into(),
                    SpendApprovalMode::FullAutonomy => "auto".into(),
                };
                div()
                    .id(pro_eid(&self.id, "spc-approval-menu"))
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
                                "如何审批策略变更？",
                            )),
                    )
                    .child(
                        div()
                            .relative()
                            .flex()
                            .flex_col()
                            .gap(px(2.))
                            .child(pro_pill_indicator(
                                pro_eid(&self.id, "spc-pol-pill"),
                                pol_key,
                                pol_sensor.clone(),
                                7.,
                                p.select,
                                None,
                                false,
                            ))
                            .children(
                                [
                                    SpendApprovalMode::RequestApproval,
                                    SpendApprovalMode::AutoReads,
                                    SpendApprovalMode::FullAutonomy,
                                ]
                                .into_iter()
                                .enumerate()
                                .map(|(mi, mode)| {
                                    let chosen = self.approval_mode == mode;
                                    let is_alert = mode == SpendApprovalMode::FullAutonomy;
                                    let sensor = pol_sensor.clone();
                                    activate(
                                        div()
                                            .id(pro_eid(&self.id, format!("spc-pol-{mi}")))
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
                    .id(pro_eid(&self.id, "spc-model-menu"))
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
                        pro_eid(&self.id, "spc-mdl-pill"),
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
                                .id(pro_eid(&self.id, format!("spc-mdl-{mi}")))
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
            pro_eid(&self.id, "spc-menu-vtx"),
            menu_key,
            div().w_full().flex().child(menu_card),
        );
        let popover_pres = pro_presence(
            pro_eid(&self.id, "spc-menu-pres"),
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

        let pol_fg = if self.approval_mode == SpendApprovalMode::FullAutonomy {
            p.red_ink
        } else {
            p.muted
        };

        let send_t = pro_spring_f32(
            pro_eid(&self.id, "spc-send-t"),
            f32::from(can_send),
            0.18,
            w,
            cx,
        )
        .clamp(0., 1.);
        let focus_t = pro_spring_f32(
            pro_eid(&self.id, "spc-focus-t"),
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
                    pro_eid(&self.id, "spc-rec-cancel"),
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
                        .id(pro_eid(&self.id, "spc-rec-done"))
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
                                e.set_value("Where can we save on Opus runs?", w, cx);
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
                        .id(pro_eid(&self.id, "spc-model-btn"))
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
                    pro_eid(&self.id, "spc-mic-btn"),
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
                        .id(pro_eid(&self.id, "spc-send-btn"))
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
                            .id(pro_eid(&self.id, format!("spc-att-rm-{ai}")))
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
                                pro_eid(&self.id, "spc-att-fold"),
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
                                                pro_eid(&self.id, "spc-plus-btn"),
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
                                                    .id(pro_eid(&self.id, "spc-approval-btn"))
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
                    .child(pro_skeleton_bar(210., 38., 16., phase, cx)),
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
                                        if narrow { 250. } else { 490. },
                                        14.,
                                        5.,
                                        phase + 0.08,
                                        cx,
                                    ))
                                    .child(pro_skeleton_bar(
                                        if narrow { 180. } else { 320. },
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
                        246.,
                        14.,
                        phase + 0.35,
                        cx,
                    )),
            )
    }

    fn render_empty_starter(&self, cx: &Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let starters = [
            "Where does our AI spend go?",
            "Where can we save?",
            "Show the trend",
            "Which team spends most?",
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
                ProIcon::Coins,
                tr(cx, "Ask about spend, models or teams", "分析 AI 成本、模型占比与节省空间"),
                tr(
                    cx,
                    "Inspect provider spend, simulate model-routing and prompt-caching levers, and turn plans into live rules.",
                    "查看各模型提供商成本占比，交互调节模型路由与提示词缓存比例，并一键生成路由策略。",
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
            SpendArtifactKind::UsageApi => {
                tr(cx, "Usage API · November", "Usage API · 11 月明细").into()
            }
            SpendArtifactKind::BillingInvoice => {
                tr(cx, "Billing · November", "账单汇总 · 11 月").into()
            }
            SpendArtifactKind::RoutingRules => {
                tr(cx, "model-routing.toml", "model-routing.toml").into()
            }
        };

        let kind_key: SharedString = match kind {
            SpendArtifactKind::UsageApi => "usage".into(),
            SpendArtifactKind::BillingInvoice => "billing".into(),
            SpendArtifactKind::RoutingRules => "rules".into(),
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
                        pro_eid(&self.id, "spc-art-pill"),
                        kind_key.clone(),
                        art_sensor.clone(),
                        6.,
                        p.chip,
                        Some(p.line),
                        false,
                    ))
                    .children(
                        [
                            (SpendArtifactKind::UsageApi, "Usage API"),
                            (SpendArtifactKind::BillingInvoice, "Billing"),
                            (SpendArtifactKind::RoutingRules, tr(cx, "Rules", "规则")),
                        ]
                        .into_iter()
                        .map(|(k, lbl)| {
                            let on = kind == k;
                            let sensor = art_sensor.clone();
                            activate(
                                div()
                                    .id(pro_eid(&self.id, format!("sp-art-{lbl}")))
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
            SpendArtifactKind::UsageApi => vec![
                ("Claude Opus 5.5", "$26,400", "54.8%", 0.92),
                ("Claude Sonnet 5.5", "$8,800", "18.3%", 0.34),
                ("GPT-6", "$5,100", "10.6%", 0.21),
                ("Gemini 3 Pro", "$2,700", "5.6%", 0.12),
                ("Claude Haiku 5.5", "$2,100", "4.4%", 0.09),
                ("GPT-6 mini + Flash", "$3,100", "6.3%", 0.14),
            ],
            SpendArtifactKind::BillingInvoice => vec![
                ("Anthropic Enterprise", "$37,300", "+26% MoM", 0.84),
                ("OpenAI Platform", "$7,000", "+8% MoM", 0.19),
                ("Google Vertex AI", "$3,900", "+5% MoM", 0.11),
                ("Committed credits", "-$4,000", "Applied", 0.10),
                ("Net November total", "$44,200", "Due Dec 1", 1.00),
            ],
            SpendArtifactKind::RoutingRules => vec![
                ("classify / summarize", "Sonnet 5.5", "-62% cost", 0.62),
                ("repo context >= 16K", "Prompt cache", "5m TTL", 0.45),
                ("overnight evals", "Batch API", "-50% cost", 0.50),
                ("architecture / codegen", "Opus 5.5", "Unchanged", 1.00),
            ],
        };

        let bar_tint = match kind {
            SpendArtifactKind::UsageApi => SpendBarTone::Coral.resolve(&p),
            SpendArtifactKind::BillingInvoice => p.blue,
            SpendArtifactKind::RoutingRules => p.green,
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
                            .text_size(px(12.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(tr(cx, "Live workspace ledger", "工作区实时成本账本")),
                    )
                    .child(pro_btn(
                        pro_eid(&self.id, "sp-art-copy"),
                        tr(cx, "Copy", "复制"),
                        Some(ProIcon::Copy),
                        ProBtnStyle::Secondary,
                        true,
                        cx,
                        |this, _, cx| {
                            cx.write_to_clipboard(ClipboardItem::new_string(
                                "provider,spend_usd\nAnthropic,37300\nOpenAI,7000\nGoogle,3900"
                                    .to_string(),
                            ));
                            this.toast = Some((
                                tr(cx, "Copied ledger to clipboard", "已复制账本到剪贴板").into(),
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
                                        pro_eid(&self.id, format!("spc-art-bar-{ri}")),
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
                pro_eid(&self.id, "spc-art-vtx"),
                kind_key,
                panel_inner,
            ))
            .into_any_element();

        Some((title, ProIcon::Coins, body))
    }
}

impl Render for AiChatSpendScreen {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.standalone {
            return self.render_page(w, cx).into_any_element();
        }
        self.shell.clone().into_any_element()
    }
}

fn planner_pixel_mark(color: Hsla, busy: bool, elapsed_ms: f32, reduced: bool) -> Div {
    let cells = agent_pixel_cells("SpendPlanner");
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

fn format_k_axis(dollars: f64) -> String {
    format!("${:.0}k", dollars / 1_000.0)
}

/// Format bar value labels matching `$37k`, `$7k`, `$3.9k` in the reference DOM.
fn format_bar_top(dollars: f64) -> String {
    let k = dollars / 1_000.0;
    if k >= 10.0 || (k - k.round()).abs() < 0.05 {
        format!("${:.0}k", k)
    } else {
        format!("${:.1}k", k)
    }
}

fn format_spend_short(dollars: f64) -> String {
    let k = dollars / 1_000.0;
    if k >= 1.0 {
        format!("${:.1}K", k)
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

fn compute_split_bar_focus(groups: &[SpendSplitGroup], index: usize) -> Option<(String, String)> {
    let grp = groups.get(index)?;
    let grand: f64 = groups.iter().map(SpendSplitGroup::total).sum();
    let tot = grp.total();
    let pct = if grand > 0.0 {
        (tot / grand * 100.0).round() as i32
    } else {
        0
    };
    Some((
        grp.name.to_string(),
        format!("{} · {pct}% of spend", format_spend_short(tot)),
    ))
}

fn compute_plan_projection(
    now_dollars: f64,
    month_count: usize,
    monthly_growth: f64,
    levers: &[SpendLever],
) -> (f64, f64, Vec<f64>, Vec<f64>) {
    let monthly_save: f64 = levers.iter().map(SpendLever::monthly_savings).sum();
    let save_ratio = if now_dollars > 0.0 {
        (monthly_save / now_dollars).clamp(0.0, 0.85)
    } else {
        0.0
    };
    let n = month_count.max(2);
    let mut baseline_k = Vec::with_capacity(n);
    let mut planned_k = Vec::with_capacity(n);
    let mut six_mo_save = 0.0_f64;

    for m in 0..n {
        let base_m = now_dollars * (1.0 + monthly_growth).powi(m as i32);
        let plan_m = if m == 0 {
            base_m
        } else {
            base_m * (1.0 - save_ratio)
        };
        if m > 0 {
            six_mo_save += base_m - plan_m;
        }
        baseline_k.push(base_m / 1000.0);
        planned_k.push(plan_m / 1000.0);
    }
    (monthly_save, six_mo_save, baseline_k, planned_k)
}

fn sample_spend_nav_groups(cx: &App) -> Vec<ShellNavGroup> {
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
                ShellNavItem::new("brand-assets", "Brand assets", ProIcon::Folder),
                ShellNavItem::new("hiring", "Hiring", ProIcon::Folder),
                ShellNavItem::new("research", "Research", ProIcon::Folder),
            ],
        ),
        ShellNavGroup::named(
            tr(cx, "Chats", "最近对话"),
            vec![
                ShellNavItem::new("nov-ai-spend", "AI spend in November", ProIcon::Coins),
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

fn sample_plan_levers() -> Vec<SpendLever> {
    vec![
        SpendLever {
            id: "sonnet-routing".into(),
            label: "Route routine steps to Sonnet 5.5".into(),
            detail: "Classification, file lookups and short summaries currently on Opus 5.5".into(),
            kind: SpendLeverKind::Range { max_pct: 80 },
            base_dollars: 26_400.0,
            save_rate: 0.62,
            value: 50.0,
        },
        SpendLever {
            id: "prompt-cache".into(),
            label: "Cache repeated system & repo prompts".into(),
            detail: "Long agent runs resend ~42K context tokens on every tool step".into(),
            kind: SpendLeverKind::Range { max_pct: 90 },
            base_dollars: 18_200.0,
            save_rate: 0.45,
            value: 60.0,
        },
        SpendLever {
            id: "batch-evals".into(),
            label: "Batch overnight evaluations & digests".into(),
            detail: "50% off batch pricing for scheduled non-interactive runs".into(),
            kind: SpendLeverKind::Switch,
            base_dollars: 6_400.0,
            save_rate: 0.50,
            value: 1.0,
        },
    ]
}

fn sample_spend_turns() -> Vec<SpendTurn> {
    vec![
        SpendTurn::User {
            id: "u-1".into(),
            quote: None,
            text: "Where does our AI spend go?".into(),
        },
        SpendTurn::Assistant {
            id: "a-1".into(),
            text: "You spent $48.2K on AI this month – 22% more than in October. Opus 5.5 is more than half of it, mostly long agent runs; GPT-6 and Gemini are small and steady.".into(),
            kpis: vec![
                SpendKpi::new("Spend this month", "$48.2K", "+22.0% vs. October", false),
                SpendKpi::new("Per active user", "$3.10", "+4.2% vs. October", false),
                SpendKpi::new("Runs", "18.4K", "+31.5% vs. October", true),
            ],
            chart: Some(SpendChart::Split {
                title: "November".into(),
                previous_total: Some(39_500.0),
                since_label: Some("October".into()),
                groups: vec![
                    SpendSplitGroup {
                        name: "Anthropic".into(),
                        logo: Some("claude".into()),
                        tone: SpendBarTone::Coral,
                        parts: vec![
                            SpendModelPart::new("Claude Opus 5.5", 26_400.0),
                            SpendModelPart::new("Claude Sonnet 5.5", 8_800.0),
                            SpendModelPart::new("Claude Haiku 5.5", 2_100.0),
                        ],
                    },
                    SpendSplitGroup {
                        name: "OpenAI".into(),
                        logo: Some("openai".into()),
                        tone: SpendBarTone::Teal,
                        parts: vec![
                            SpendModelPart::new("GPT-6", 5_100.0),
                            SpendModelPart::new("GPT-6 mini", 1_900.0),
                        ],
                    },
                    SpendSplitGroup {
                        name: "Google".into(),
                        logo: Some("gemini".into()),
                        tone: SpendBarTone::Blue,
                        parts: vec![
                            SpendModelPart::new("Gemini 3 Pro", 2_700.0),
                            SpendModelPart::new("Gemini 3 Flash", 1_200.0),
                        ],
                    },
                ],
                alt_tab_labels: Some(("Providers".into(), "Teams".into())),
                alt_groups: vec![
                    SpendSplitGroup {
                        name: "Platform".into(),
                        logo: None,
                        tone: SpendBarTone::Coral,
                        parts: vec![
                            SpendModelPart::new("Opus 5.5", 13_800.0),
                            SpendModelPart::new("Sonnet 5.5", 4_100.0),
                            SpendModelPart::new("GPT-6", 1_500.0),
                        ],
                    },
                    SpendSplitGroup {
                        name: "Product".into(),
                        logo: None,
                        tone: SpendBarTone::Purple,
                        parts: vec![
                            SpendModelPart::new("Opus 5.5", 8_200.0),
                            SpendModelPart::new("Sonnet 5.5", 4_200.0),
                            SpendModelPart::new("Gemini 3", 2_400.0),
                        ],
                    },
                    SpendSplitGroup {
                        name: "Growth".into(),
                        logo: None,
                        tone: SpendBarTone::Teal,
                        parts: vec![
                            SpendModelPart::new("Opus 5.5", 3_100.0),
                            SpendModelPart::new("GPT-6", 3_600.0),
                            SpendModelPart::new("Gemini 3", 1_900.0),
                        ],
                    },
                    SpendSplitGroup {
                        name: "Support".into(),
                        logo: None,
                        tone: SpendBarTone::Blue,
                        parts: vec![
                            SpendModelPart::new("Haiku & mini", 3_400.0),
                            SpendModelPart::new("Sonnet 5.5", 2_000.0),
                        ],
                    },
                ],
            }),
            caption: None,
            sources: vec!["Usage API".into(), "Billing".into()],
            next: vec![
                "Where can we save?".into(),
                "Show the trend".into(),
                "Which team spends most?".into(),
            ],
            liked: false,
            streaming: false,
        },
    ]
}

fn build_spend_answer(question: &str, focus: Option<&SpendChartFocus>) -> SpendTurn {
    let lower = question.to_ascii_lowercase();

    if let Some(foc) = focus {
        return SpendTurn::Assistant {
            id: "a-focus".into(),
            text: format!(
                "{} ({}) is driven by multi-step code review and repository indexing agents; enabling prompt caching cuts repeated input tokens by 44%.",
                foc.label, foc.detail
            )
            .into(),
            kpis: vec![
                SpendKpi::new("Focused slice", foc.label.clone(), foc.detail.clone(), false),
                SpendKpi::new("Uncached input", "61%", "Eligible for 5m cache", true),
                SpendKpi::new("Est. savings", "-$6.4K/mo", "With routing + cache", true),
            ],
            chart: None,
            caption: None,
            sources: vec!["Usage API".into(), "Billing".into()],
            next: vec![
                "Where can we save?".into(),
                "Which team spends most?".into(),
            ],
            liked: false,
            streaming: false,
        };
    }

    if lower.contains("rule") {
        return SpendTurn::Assistant {
            id: "a-rules".into(),
            text: "Staged three routing rules in model-routing.toml: routine classification and file lookups move to Sonnet 5.5, repository contexts above 16K tokens enable 5-minute prompt caching, and nightly evaluations run on the Batch API.".into(),
            kpis: vec![
                SpendKpi::new("Active rules", "3", "Staged for review", true),
                SpendKpi::new("Projected Dec", "$31.9K", "-33.8% vs. November", true),
                SpendKpi::new("Latency impact", "-18%", "Faster Sonnet steps", true),
            ],
            chart: None,
            caption: None,
            sources: vec!["Usage API".into(), "Routing rules".into()],
            next: vec![
                "Show the trend".into(),
                "Which team spends most?".into(),
            ],
            liked: false,
            streaming: false,
        };
    }

    if lower.contains("save") || lower.contains("plan") || lower.contains("lever") {
        return SpendTurn::Assistant {
            id: "a-plan".into(),
            text: "Three changes trim about $16.3K a month without touching final code-generation quality. Adjust the levers below to see how the six-month forecast responds.".into(),
            kpis: vec![
                SpendKpi::new("Current pace", "$48.2K/mo", "+8% monthly growth", false),
                SpendKpi::new("With default plan", "$31.9K/mo", "-33.8% vs. current", true),
                SpendKpi::new("Quality impact", "None", "Keeps Opus on synthesis", true),
            ],
            chart: Some(SpendChart::Plan {
                now_dollars: 48_200.0,
                months: vec![
                    "Nov".into(),
                    "Dec".into(),
                    "Jan".into(),
                    "Feb".into(),
                    "Mar".into(),
                    "Apr".into(),
                ],
                monthly_growth: 0.08,
                levers: sample_plan_levers(),
            }),
            caption: None,
            sources: vec!["Usage API".into(), "Billing".into()],
            next: vec![
                "Show the trend".into(),
                "Which team spends most?".into(),
            ],
            liked: false,
            streaming: false,
        };
    }

    if lower.contains("trend") || lower.contains("week") {
        return SpendTurn::Assistant {
            id: "a-trend".into(),
            text: "Weekly AI spend climbed from $9.8K at the start of November to $13.6K in W46 when autonomous test-fixing rolled out across Platform.".into(),
            kpis: vec![
                SpendKpi::new("Peak week (W46)", "$13.6K", "+38.8% vs. W44", false),
                SpendKpi::new("Weekly average", "$12.1K", "+22.0% vs. October", false),
                SpendKpi::new("Cost per run", "$2.62", "-7.4% vs. October", true),
            ],
            chart: Some(SpendChart::Trend {
                labels: (40..=47).map(|w| SharedString::from(format!("W{w}"))).collect(),
                current: vec![9.4, 9.8, 10.2, 10.9, 11.4, 12.2, 13.6, 12.8],
                previous: vec![8.1, 8.4, 8.6, 8.9, 9.2, 9.6, 10.1, 9.9],
                mark: Some(6),
            }),
            caption: Some("Weekly AI spend ($K) · W46 Platform agent rollout".into()),
            sources: vec!["Usage API".into(), "Billing".into()],
            next: vec![
                "Where can we save?".into(),
                "Which team spends most?".into(),
            ],
            liked: false,
            streaming: false,
        };
    }

    SpendTurn::Assistant {
        id: "a-teams".into(),
        text: "Platform accounts for 40% of November spend ($19.4K), mostly background CI triage and migration agents on Opus 5.5. Product and Growth together make up $23.4K.".into(),
        kpis: vec![
            SpendKpi::new("Platform", "$19.4K", "40% of total · +29% MoM", false),
            SpendKpi::new("Product", "$14.8K", "31% of total · +18% MoM", false),
            SpendKpi::new("Support & Growth", "$14.0K", "29% of total · +14% MoM", true),
        ],
        chart: Some(SpendChart::Split {
            title: "November by team".into(),
            previous_total: Some(39_500.0),
            since_label: Some("October".into()),
            groups: vec![
                SpendSplitGroup {
                    name: "Platform".into(),
                    logo: None,
                    tone: SpendBarTone::Coral,
                    parts: vec![
                        SpendModelPart::new("Opus 5.5", 13_800.0),
                        SpendModelPart::new("Sonnet 5.5", 4_100.0),
                        SpendModelPart::new("GPT-6", 1_500.0),
                    ],
                },
                SpendSplitGroup {
                    name: "Product".into(),
                    logo: None,
                    tone: SpendBarTone::Purple,
                    parts: vec![
                        SpendModelPart::new("Opus 5.5", 8_200.0),
                        SpendModelPart::new("Sonnet 5.5", 4_200.0),
                        SpendModelPart::new("Gemini 3", 2_400.0),
                    ],
                },
                SpendSplitGroup {
                    name: "Growth".into(),
                    logo: None,
                    tone: SpendBarTone::Teal,
                    parts: vec![
                        SpendModelPart::new("Opus 5.5", 3_100.0),
                        SpendModelPart::new("GPT-6", 3_600.0),
                        SpendModelPart::new("Gemini 3", 1_900.0),
                    ],
                },
                SpendSplitGroup {
                    name: "Support".into(),
                    logo: None,
                    tone: SpendBarTone::Blue,
                    parts: vec![
                        SpendModelPart::new("Haiku & mini", 3_400.0),
                        SpendModelPart::new("Sonnet 5.5", 2_000.0),
                    ],
                },
            ],
            alt_tab_labels: Some(("Teams".into(), "Providers".into())),
            alt_groups: vec![
                SpendSplitGroup {
                    name: "Anthropic".into(),
                    logo: Some("claude".into()),
                    tone: SpendBarTone::Coral,
                    parts: vec![
                        SpendModelPart::new("Claude Opus 5.5", 26_400.0),
                        SpendModelPart::new("Claude Sonnet 5.5", 8_800.0),
                        SpendModelPart::new("Claude Haiku 5.5", 2_100.0),
                    ],
                },
                SpendSplitGroup {
                    name: "OpenAI".into(),
                    logo: Some("openai".into()),
                    tone: SpendBarTone::Teal,
                    parts: vec![
                        SpendModelPart::new("GPT-6", 5_100.0),
                        SpendModelPart::new("GPT-6 mini", 1_900.0),
                    ],
                },
                SpendSplitGroup {
                    name: "Google".into(),
                    logo: Some("gemini".into()),
                    tone: SpendBarTone::Blue,
                    parts: vec![
                        SpendModelPart::new("Gemini 3 Pro", 2_700.0),
                        SpendModelPart::new("Gemini 3 Flash", 1_200.0),
                    ],
                },
            ],
        }),
        caption: None,
        sources: vec!["Usage API".into(), "Billing".into()],
        next: vec![
            "Where can we save?".into(),
            "Show the trend".into(),
        ],
        liked: false,
        streaming: false,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        SpendChart, SpendLeverKind, SpendSplitGroup, SpendTurn, build_spend_answer,
        compute_plan_projection, compute_split_bar_focus, format_bar_top, sample_plan_levers,
        sample_spend_turns,
    };

    #[test]
    fn november_provider_breakdown_totals_match_48200_and_focus_shares() {
        let turns = sample_spend_turns();
        assert_eq!(turns.len(), 2);
        let SpendTurn::Assistant {
            kpis,
            chart: Some(SpendChart::Split { groups, .. }),
            ..
        } = &turns[1]
        else {
            panic!("expected initial assistant split turn");
        };

        assert_eq!(kpis.len(), 3);
        assert!(!kpis[0].good);
        assert!(!kpis[1].good);
        assert!(kpis[2].good);

        let grand: f64 = groups.iter().map(SpendSplitGroup::total).sum();
        assert!((grand - 48_200.0).abs() < 1e-6);
        assert_eq!(format_bar_top(groups[0].total()), "$37k");
        assert_eq!(format_bar_top(groups[1].total()), "$7k");
        assert_eq!(format_bar_top(groups[2].total()), "$3.9k");

        let (lbl, detail) = compute_split_bar_focus(groups, 0).expect("first provider exists");
        assert_eq!(lbl, "Anthropic");
        assert_eq!(detail, "$37.3K · 77% of spend");
    }

    #[test]
    fn savings_plan_levers_recompute_monthly_and_six_month_forecast() {
        let mut levers = sample_plan_levers();
        let (monthly_save, six_mo_save, base_k, plan_k) =
            compute_plan_projection(48_200.0, 6, 0.08, &levers);

        assert!(monthly_save > 16_000.0 && monthly_save < 16_600.0);
        assert!(six_mo_save > 90_000.0);
        assert_eq!(base_k.len(), 6);
        assert_eq!(plan_k.len(), 6);
        assert!((base_k[0] - plan_k[0]).abs() < 1e-6);
        assert!(plan_k[5] < base_k[5]);

        assert!(matches!(
            levers[0].kind,
            SpendLeverKind::Range { max_pct: 80 }
        ));
        assert!(matches!(levers[2].kind, SpendLeverKind::Switch));

        // Zeroing all levers collapses planned trajectory back to baseline
        for lv in &mut levers {
            lv.value = 0.0;
        }
        let (zero_save, _, base2, plan2) = compute_plan_projection(48_200.0, 6, 0.08, &levers);
        assert!(zero_save.abs() < 1e-6);
        assert!((base2[5] - plan2[5]).abs() < 1e-6);

        let plan_turn = build_spend_answer("Where can we save?", None);
        assert!(matches!(
            plan_turn,
            SpendTurn::Assistant {
                chart: Some(SpendChart::Plan { .. }),
                ..
            }
        ));
    }
}
