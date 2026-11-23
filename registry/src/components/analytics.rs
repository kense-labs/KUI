//! Revenue analytics workspace screen with channel series, breakdown sparklines, and product share.
//!
//! Renders an [`AppShell`] revenue dashboard featuring period selection, KPI stat cards
//! with inline mini-bar histograms, multi-channel trend inspection, revenue breakdown
//! sparklines, top-product share bars, session and AOV comparison charts, loading
//! skeletons, and an empty store-connection state.

use crate::{
    components::{
        app_shell::{AppShell, AppShellEvent, ShellNavGroup, ShellNavItem},
        pro_chrome::{
            ProBtnStyle, ProPalette, pro_btn, pro_eid, pro_empty_state, pro_mark_selected,
            pro_pill_indicator, pro_presence, pro_selection_sensor, pro_skeleton_bar,
            pro_spring_bar, pro_spring_f32, pro_toast_banner, pro_view_transition, tr,
        },
    },
    foundation::Theme,
    primitives::{
        control::activate,
        pro_icons::{ProIcon, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

const TRIANGLE_UP_SVG: &str = r#"<path d="M4 1.2 7.2 6.4H.8L4 1.2Z" fill="currentColor"/>"#;
const TRIANGLE_DOWN_SVG: &str = r#"<path d="M4 6.8 7.2 1.6H.8L4 6.8Z" fill="currentColor"/>"#;

/// Time range window for [`AnalyticsScreen`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AnalyticsPeriod {
    Last24Hours,
    #[default]
    Last7Days,
    Last30Days,
    Last90Days,
}

impl AnalyticsPeriod {
    /// Return all selectable time range windows in menu order.
    pub const fn all() -> [Self; 4] {
        [
            Self::Last24Hours,
            Self::Last7Days,
            Self::Last30Days,
            Self::Last90Days,
        ]
    }

    /// Localized display label for the period button and dropdown menu.
    pub fn label(self, cx: &App) -> &'static str {
        match self {
            Self::Last24Hours => tr(cx, "Last 24 hours", "过去 24 小时"),
            Self::Last7Days => tr(cx, "Last 7 days", "过去 7 天"),
            Self::Last30Days => tr(cx, "Last 30 days", "过去 30 天"),
            Self::Last90Days => tr(cx, "Last 90 days", "过去 90 天"),
        }
    }

    /// Short file-slug token used in export filenames.
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Last24Hours => "24h",
            Self::Last7Days => "7d",
            Self::Last30Days => "30d",
            Self::Last90Days => "90d",
        }
    }

    fn scale_factor(self) -> f32 {
        match self {
            Self::Last24Hours => 0.16,
            Self::Last7Days => 1.0,
            Self::Last30Days => 4.15,
            Self::Last90Days => 11.8,
        }
    }

    fn x_labels(self) -> [&'static str; 7] {
        match self {
            Self::Last24Hours => ["00:00", "04:00", "08:00", "12:00", "16:00", "20:00", "Now"],
            Self::Last7Days => [
                "Mar 3", "Mar 4", "Mar 5", "Mar 6", "Mar 7", "Mar 8", "Mar 9",
            ],
            Self::Last30Days => [
                "Feb 8", "Feb 13", "Feb 18", "Feb 23", "Feb 28", "Mar 4", "Mar 9",
            ],
            Self::Last90Days => [
                "Dec 10", "Dec 25", "Jan 9", "Jan 24", "Feb 8", "Feb 23", "Mar 9",
            ],
        }
    }
}

/// Direction of change considered positive news for an [`AnalyticsStat`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AnalyticsGoodDir {
    #[default]
    Up,
    Down,
}

/// Semantic color tone for a breakdown row sparkline.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AnalyticsSparkTone {
    #[default]
    Up,
    Down,
    Flat,
}

/// Summary KPI card model at the top of [`AnalyticsScreen`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AnalyticsStat {
    pub label: SharedString,
    pub value: SharedString,
    pub delta: SharedString,
    pub bars: Vec<f32>,
    pub good: AnalyticsGoodDir,
    pub up: bool,
}

impl AnalyticsStat {
    /// Return whether this stat's directional change represents positive performance.
    pub const fn is_good(&self) -> bool {
        matches!(
            (self.good, self.up),
            (AnalyticsGoodDir::Up, true) | (AnalyticsGoodDir::Down, false)
        )
    }
}

/// One row of the revenue breakdown list with a 7-point mini sparkline.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AnalyticsBreakdownRow {
    pub label: SharedString,
    pub value: SharedString,
    pub spark: Vec<f32>,
    pub tone: AnalyticsSparkTone,
}

/// One product entry in the top-products share card.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AnalyticsProduct {
    pub label: SharedString,
    pub value: SharedString,
    /// Relative share percentage (`0.0..=100.0`) against the leading product.
    pub share: f32,
}

/// Daily values for one revenue channel line on the main chart.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AnalyticsChannelSeries {
    pub id: SharedString,
    pub label: SharedString,
    pub values: Vec<f32>,
}

/// Events emitted by [`AnalyticsScreen`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnalyticsEvent {
    Navigate(SharedString),
    PeriodChanged(AnalyticsPeriod),
    SelectStat(Option<usize>),
    FilterChannel(Option<SharedString>),
    FilterProduct(Option<SharedString>),
    ConnectStore,
    Export,
    SaveReport,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum HoveredChart {
    Channels,
    Sessions,
    Aov,
}

struct MultiLineSpec<'a> {
    chart: HoveredChart,
    aria: SharedString,
    y_ticks: [&'a str; 3],
    x_labels: [&'a str; 7],
    max_y: f32,
    height: f32,
    series: Vec<(SharedString, Hsla, Vec<f32>, bool)>,
    hover_idx: Option<usize>,
    bounds_cell: Rc<Cell<Bounds<Pixels>>>,
    unit_prefix: &'a str,
    unit_suffix: &'a str,
}

fn push_grouped_digits(out: &mut String, n: u64) {
    let bytes = n.to_string().into_bytes();
    let mut chunks = bytes.rchunks(3).rev();
    if let Some(first) = chunks.next() {
        for &b in first {
            out.push(b as char);
        }
        for chunk in chunks {
            out.push(',');
            for &b in chunk {
                out.push(b as char);
            }
        }
    }
}

fn format_dollars(amount: f32) -> String {
    let neg = amount < 0.;
    let n = amount.abs().round() as u64;
    let mut out = String::with_capacity(16);
    if neg {
        out.push('−');
    }
    out.push('$');
    push_grouped_digits(&mut out, n);
    out
}

fn format_int(n: u64) -> String {
    let mut out = String::with_capacity(12);
    push_grouped_digits(&mut out, n);
    out
}

fn build_sidebar_groups(cx: &App) -> Vec<ShellNavGroup> {
    vec![
        ShellNavGroup::primary(vec![
            ShellNavItem::new("home", tr(cx, "Home", "首页"), ProIcon::Home),
            ShellNavItem::new("revenue", tr(cx, "Revenue", "营收"), ProIcon::UsageChart),
            ShellNavItem::new("invoices", tr(cx, "Invoices", "账单发票"), ProIcon::PageDoc)
                .with_count(12),
            ShellNavItem::new(
                "payouts",
                tr(cx, "Payouts", "结算打款"),
                ProIcon::CreditCard,
            ),
            ShellNavItem::new("customers", tr(cx, "Customers", "客户"), ProIcon::Users),
        ]),
        ShellNavGroup::named(
            tr(cx, "Channels", "渠道"),
            vec![
                ShellNavItem::new(
                    "online-store",
                    tr(cx, "Online store", "在线商店"),
                    ProIcon::Building,
                ),
                ShellNavItem::new(
                    "marketplace",
                    tr(cx, "Marketplace", "应用市场"),
                    ProIcon::Grid,
                ),
                ShellNavItem::new(
                    "wholesale",
                    tr(cx, "Wholesale", "企业批发"),
                    ProIcon::Folder,
                ),
                ShellNavItem::new(
                    "subscriptions",
                    tr(cx, "Subscriptions", "订阅续费"),
                    ProIcon::Refresh,
                ),
            ],
        ),
    ]
}

fn channel_filter_scale(channel: Option<&str>) -> f32 {
    match channel {
        Some("Online store") | Some("在线商店") => 0.48,
        Some("Marketplace") | Some("应用市场") => 0.29,
        Some("Wholesale") | Some("企业批发") => 0.14,
        Some("Subscriptions") | Some("订阅续费") => 0.09,
        _ => 1.0,
    }
}

fn product_filter_scale(product: Option<&str>) -> f32 {
    match product {
        Some("Annual plan") | Some("年度订阅") => 0.40,
        Some("Team seats") | Some("团队席位") => 0.25,
        Some("Monthly plan") | Some("月度订阅") => 0.20,
        Some("Add-ons") | Some("增值扩展") => 0.10,
        _ => 1.0,
    }
}

/// Revenue and channel telemetry screen hosted inside [`AppShell`].
pub struct AnalyticsScreen {
    id: ElementId,
    standalone: bool,
    shell: Entity<AppShell>,
    focus: FocusHandle,
    workspace: SharedString,
    user_name: SharedString,
    user_email: SharedString,
    period: AnalyticsPeriod,
    period_menu_open: bool,
    loading: bool,
    empty: bool,
    custom_stats: Option<Vec<AnalyticsStat>>,
    custom_breakdown: Option<Vec<AnalyticsBreakdownRow>>,
    custom_products: Option<Vec<AnalyticsProduct>>,
    custom_channels: Option<Vec<AnalyticsChannelSeries>>,
    selected_stat: Option<usize>,
    channel_filter: Option<SharedString>,
    product_filter: Option<SharedString>,
    hover_channel_day: Option<usize>,
    hover_sessions_day: Option<usize>,
    hover_aov_day: Option<usize>,
    channel_plot_bounds: Rc<Cell<Bounds<Pixels>>>,
    sessions_plot_bounds: Rc<Cell<Bounds<Pixels>>>,
    aov_plot_bounds: Rc<Cell<Bounds<Pixels>>>,
    measured_width: Rc<Cell<f32>>,
    toast: Option<(SharedString, Instant)>,
    saved_report: bool,
    anim_start: Instant,
}

impl EventEmitter<AnalyticsEvent> for AnalyticsScreen {}

impl Focusable for AnalyticsScreen {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl AnalyticsScreen {
    /// Construct a full [`AnalyticsScreen`] framed inside [`AppShell`].
    pub fn new(id: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        Self::build(id.into(), false, cx)
    }

    /// Construct the analytics canvas without the outer [`AppShell`] sidebar.
    pub fn standalone(id: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        Self::build(id.into(), true, cx)
    }

    fn build(id_str: SharedString, standalone: bool, cx: &mut Context<Self>) -> Self {
        let this = cx.entity();
        let active_label: SharedString = tr(cx, "Revenue", "营收").into();
        let shell = cx.new(|shell_cx| {
            let mut s = AppShell::new(format!("{id_str}-shell"), shell_cx)
                .with_workspace("Acme")
                .with_user("Alex Morgan", "alex@acme.ai")
                .with_groups(build_sidebar_groups(shell_cx))
                .with_active(active_label)
                .with_side_cards(true)
                .with_work_area(false, false);
            s.bind_page(&this, shell_cx, |screen, w, cx| {
                screen.render_page(w, cx).into_any_element()
            });
            s
        });

        cx.subscribe(&shell, |this, _, ev: &AppShellEvent, cx| {
            if let AppShellEvent::Navigate(label) = ev {
                let is_channel = matches!(
                    label.as_ref(),
                    "Online store"
                        | "Marketplace"
                        | "Wholesale"
                        | "Subscriptions"
                        | "在线商店"
                        | "应用市场"
                        | "企业批发"
                        | "订阅续费"
                );
                if is_channel {
                    if this.channel_filter.as_ref() == Some(label) {
                        this.channel_filter = None;
                        let rev: SharedString = tr(cx, "Revenue", "营收").into();
                        this.shell.update(cx, |s, _| s.set_active(rev));
                    } else {
                        this.channel_filter = Some(label.clone());
                    }
                    cx.emit(AnalyticsEvent::FilterChannel(this.channel_filter.clone()));
                } else if label.as_ref() == "Revenue" || label.as_ref() == "营收" {
                    this.channel_filter = None;
                    cx.emit(AnalyticsEvent::FilterChannel(None));
                }
                cx.emit(AnalyticsEvent::Navigate(label.clone()));
                cx.notify();
            }
        })
        .detach();

        Self {
            id: ElementId::Name(id_str),
            standalone,
            shell,
            focus: cx.focus_handle(),
            workspace: "Acme".into(),
            user_name: "Alex Morgan".into(),
            user_email: "alex@acme.ai".into(),
            period: AnalyticsPeriod::Last7Days,
            period_menu_open: false,
            loading: false,
            empty: false,
            custom_stats: None,
            custom_breakdown: None,
            custom_products: None,
            custom_channels: None,
            selected_stat: None,
            channel_filter: None,
            product_filter: None,
            hover_channel_day: None,
            hover_sessions_day: None,
            hover_aov_day: None,
            channel_plot_bounds: Rc::new(Cell::new(Bounds::default())),
            sessions_plot_bounds: Rc::new(Cell::new(Bounds::default())),
            aov_plot_bounds: Rc::new(Cell::new(Bounds::default())),
            measured_width: Rc::new(Cell::new(960.)),
            toast: None,
            saved_report: false,
            anim_start: Instant::now(),
        }
    }

    /// Override the sidebar workspace name.
    pub fn with_workspace(
        mut self,
        workspace: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> Self {
        let ws = workspace.into();
        self.workspace = ws.clone();
        self.refresh_shell_meta(cx);
        self
    }

    /// Override the sidebar user account footer.
    pub fn with_user(
        mut self,
        name: impl Into<SharedString>,
        email: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> Self {
        self.user_name = name.into();
        self.user_email = email.into();
        self.refresh_shell_meta(cx);
        self
    }

    fn refresh_shell_meta(&self, cx: &mut Context<Self>) {
        let ws = self.workspace.clone();
        let name = self.user_name.clone();
        let email = self.user_email.clone();
        let shell_id = format!("{}-shell", self.id);
        let this = cx.entity();
        self.shell.update(cx, |s, shell_cx| {
            let active: SharedString = tr(shell_cx, "Revenue", "营收").into();
            let mut next = AppShell::new(shell_id, shell_cx)
                .with_workspace(ws)
                .with_user(name, email)
                .with_groups(build_sidebar_groups(shell_cx))
                .with_active(active)
                .with_side_cards(true)
                .with_work_area(false, false);
            next.bind_page(&this, shell_cx, |screen, w, cx| {
                screen.render_page(w, cx).into_any_element()
            });
            *s = next;
        });
    }

    /// Configure the initial time period window.
    pub fn with_period(mut self, period: AnalyticsPeriod) -> Self {
        self.period = period;
        self
    }

    /// Configure whether skeleton placeholders are shown.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Configure whether the no-revenue empty state is shown.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Supply custom KPI summary cards.
    pub fn with_stats(mut self, stats: Vec<AnalyticsStat>) -> Self {
        self.custom_stats = Some(stats);
        self
    }

    /// Supply custom revenue breakdown rows.
    pub fn with_breakdown(mut self, rows: Vec<AnalyticsBreakdownRow>) -> Self {
        self.custom_breakdown = Some(rows);
        self
    }

    /// Supply custom top-product bars.
    pub fn with_products(mut self, products: Vec<AnalyticsProduct>) -> Self {
        self.custom_products = Some(products);
        self
    }

    /// Supply custom channel trend series.
    pub fn with_channels(mut self, channels: Vec<AnalyticsChannelSeries>) -> Self {
        self.custom_channels = Some(channels);
        self
    }

    /// Return the active time range window.
    pub const fn period(&self) -> AnalyticsPeriod {
        self.period
    }

    /// Return the currently active channel filter, if any.
    pub fn channel_filter(&self) -> Option<&str> {
        self.channel_filter.as_deref()
    }

    /// Return the currently active product filter, if any.
    pub fn product_filter(&self) -> Option<&str> {
        self.product_filter.as_deref()
    }

    /// Toggle loading state at runtime.
    pub fn set_loading(&mut self, loading: bool, cx: &mut Context<Self>) {
        self.loading = loading;
        self.anim_start = Instant::now();
        cx.notify();
    }

    /// Toggle empty state at runtime.
    pub fn set_empty(&mut self, empty: bool, cx: &mut Context<Self>) {
        self.empty = empty;
        cx.notify();
    }

    /// Select a time range window and notify listeners.
    pub fn set_period(&mut self, period: AnalyticsPeriod, cx: &mut Context<Self>) {
        self.period_menu_open = false;
        if self.period != period {
            self.period = period;
            cx.emit(AnalyticsEvent::PeriodChanged(period));
        }
        cx.notify();
    }

    /// Toggle a channel filter by display name (`None` clears).
    pub fn toggle_channel_filter(
        &mut self,
        label: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) {
        let label = label.into();
        if self.channel_filter.as_ref() == Some(&label) {
            self.channel_filter = None;
        } else {
            self.channel_filter = Some(label);
        }
        cx.emit(AnalyticsEvent::FilterChannel(self.channel_filter.clone()));
        cx.notify();
    }

    /// Toggle a top-product filter by display name (`None` clears).
    pub fn toggle_product_filter(
        &mut self,
        label: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) {
        let label = label.into();
        if self.product_filter.as_ref() == Some(&label) {
            self.product_filter = None;
        } else {
            self.product_filter = Some(label);
        }
        cx.emit(AnalyticsEvent::FilterProduct(self.product_filter.clone()));
        cx.notify();
    }

    /// Clear all active filters and stat highlights.
    pub fn clear_filters(&mut self, cx: &mut Context<Self>) {
        self.channel_filter = None;
        self.product_filter = None;
        self.selected_stat = None;
        let rev: SharedString = tr(cx, "Revenue", "营收").into();
        self.shell.update(cx, |s, _| s.set_active(rev));
        cx.emit(AnalyticsEvent::FilterChannel(None));
        cx.emit(AnalyticsEvent::FilterProduct(None));
        cx.notify();
    }

    fn combined_scale(&self) -> f32 {
        self.period.scale_factor()
            * channel_filter_scale(self.channel_filter.as_deref())
            * product_filter_scale(self.product_filter.as_deref())
    }

    fn effective_stats(&self, cx: &App) -> Vec<AnalyticsStat> {
        if let Some(custom) = &self.custom_stats {
            return custom.clone();
        }
        let s = self.combined_scale();
        let gross = if (s - 1.0).abs() < 1e-3 {
            "$96,412".to_string()
        } else {
            format_dollars(96_412. * s)
        };
        let customers = if (s - 1.0).abs() < 1e-3 {
            "2,847".to_string()
        } else {
            format_int((2_847. * s.powf(0.82)).round().max(12.) as u64)
        };
        let conv = match self.period {
            AnalyticsPeriod::Last24Hours => "3.41%",
            AnalyticsPeriod::Last7Days => "3.24%",
            AnalyticsPeriod::Last30Days => "3.18%",
            AnalyticsPeriod::Last90Days => "3.09%",
        };
        let refund = match self.period {
            AnalyticsPeriod::Last24Hours => "1.5%",
            AnalyticsPeriod::Last7Days => "1.8%",
            AnalyticsPeriod::Last30Days => "1.9%",
            AnalyticsPeriod::Last90Days => "2.1%",
        };
        vec![
            AnalyticsStat {
                label: tr(cx, "Gross revenue", "总营收").into(),
                value: gross.into(),
                delta: "12.5%".into(),
                bars: vec![0.42, 0.56, 0.48, 0.68, 0.62, 0.84, 1.0],
                good: AnalyticsGoodDir::Up,
                up: true,
            },
            AnalyticsStat {
                label: tr(cx, "Active customers", "活跃客户").into(),
                value: customers.into(),
                delta: "8.2%".into(),
                bars: vec![0.50, 0.58, 0.55, 0.66, 0.72, 0.80, 0.94],
                good: AnalyticsGoodDir::Up,
                up: true,
            },
            AnalyticsStat {
                label: tr(cx, "Conversion", "转化率").into(),
                value: conv.into(),
                delta: "0.4%".into(),
                bars: vec![0.65, 0.52, 0.70, 0.60, 0.78, 0.74, 0.90],
                good: AnalyticsGoodDir::Up,
                up: true,
            },
            AnalyticsStat {
                label: tr(cx, "Refund rate", "退款率").into(),
                value: refund.into(),
                delta: "0.3%".into(),
                bars: vec![0.88, 0.78, 0.82, 0.68, 0.64, 0.54, 0.44],
                good: AnalyticsGoodDir::Down,
                up: false,
            },
        ]
    }

    fn effective_breakdown(&self, cx: &App) -> Vec<AnalyticsBreakdownRow> {
        if let Some(custom) = &self.custom_breakdown {
            return custom.clone();
        }
        let s = self.combined_scale();
        let fmt = |exact_7d: &str, raw: f32| -> SharedString {
            if (s - 1.0).abs() < 1e-3 {
                exact_7d.to_owned().into()
            } else {
                format_dollars(raw * s).into()
            }
        };
        vec![
            AnalyticsBreakdownRow {
                label: tr(cx, "Gross sales", "商品总营收额").into(),
                value: fmt("$104,210", 104_210.),
                spark: vec![0.0, 0.4, 0.2, 0.6, 0.4, 0.8, 1.0],
                tone: AnalyticsSparkTone::Up,
            },
            AnalyticsBreakdownRow {
                label: tr(cx, "Discounts", "折扣减免").into(),
                value: fmt("−$4,180", -4_180.),
                spark: vec![0.5, 0.0, 1.0, 0.5, 0.0, 0.5, 0.0],
                tone: AnalyticsSparkTone::Down,
            },
            AnalyticsBreakdownRow {
                label: tr(cx, "Refunds", "退款冲销").into(),
                value: fmt("−$1,734", -1_734.),
                spark: vec![1.0, 0.67, 0.67, 0.33, 0.33, 0.0, 0.0],
                tone: AnalyticsSparkTone::Down,
            },
            AnalyticsBreakdownRow {
                label: tr(cx, "Shipping", "运费收入").into(),
                value: fmt("$2,940", 2_940.),
                spark: vec![0.0, 0.33, 0.33, 0.67, 0.33, 0.67, 1.0],
                tone: AnalyticsSparkTone::Up,
            },
            AnalyticsBreakdownRow {
                label: tr(cx, "Taxes", "税费代收").into(),
                value: fmt("$8,316", 8_316.),
                spark: vec![0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5],
                tone: AnalyticsSparkTone::Flat,
            },
            AnalyticsBreakdownRow {
                label: tr(cx, "Net revenue", "净营收").into(),
                value: fmt("$96,412", 96_412.),
                spark: vec![0.0, 0.2, 0.2, 0.4, 0.6, 0.6, 1.0],
                tone: AnalyticsSparkTone::Up,
            },
        ]
    }

    fn effective_products(&self, cx: &App) -> Vec<AnalyticsProduct> {
        if let Some(custom) = &self.custom_products {
            return custom.clone();
        }
        let s = self.period.scale_factor() * channel_filter_scale(self.channel_filter.as_deref());
        let fmt = |exact_7d: &str, raw: f32| -> SharedString {
            if (s - 1.0).abs() < 1e-3 {
                exact_7d.to_owned().into()
            } else {
                format_dollars(raw * s).into()
            }
        };
        vec![
            AnalyticsProduct {
                label: tr(cx, "Annual plan", "年度订阅").into(),
                value: fmt("$38,400", 38_400.),
                share: 100.0,
            },
            AnalyticsProduct {
                label: tr(cx, "Team seats", "团队席位").into(),
                value: fmt("$24,150", 24_150.),
                share: 62.9,
            },
            AnalyticsProduct {
                label: tr(cx, "Monthly plan", "月度订阅").into(),
                value: fmt("$18,920", 18_920.),
                share: 49.3,
            },
            AnalyticsProduct {
                label: tr(cx, "Add-ons", "增值扩展").into(),
                value: fmt("$9,640", 9_640.),
                share: 25.1,
            },
        ]
    }

    fn effective_channels(&self, cx: &App) -> Vec<AnalyticsChannelSeries> {
        if let Some(custom) = &self.custom_channels {
            return custom.clone();
        }
        let prod_scale = product_filter_scale(self.product_filter.as_deref());
        let scale_vec =
            |vals: [f32; 7]| -> Vec<f32> { vals.into_iter().map(|v| v * prod_scale).collect() };
        vec![
            AnalyticsChannelSeries {
                id: "online-store".into(),
                label: tr(cx, "Online store", "在线商店").into(),
                values: scale_vec([4.20, 4.70, 3.90, 4.60, 5.40, 5.10, 5.90]),
            },
            AnalyticsChannelSeries {
                id: "marketplace".into(),
                label: tr(cx, "Marketplace", "应用市场").into(),
                values: scale_vec([3.60, 3.30, 4.40, 4.80, 4.00, 4.50, 4.20]),
            },
            AnalyticsChannelSeries {
                id: "wholesale".into(),
                label: tr(cx, "Wholesale", "企业批发").into(),
                values: scale_vec([1.80, 2.30, 2.00, 1.50, 2.40, 2.20, 2.70]),
            },
            AnalyticsChannelSeries {
                id: "subscriptions".into(),
                label: tr(cx, "Subscriptions", "订阅续费").into(),
                values: scale_vec([1.20, 1.10, 1.70, 2.10, 1.60, 1.30, 1.50]),
            },
        ]
    }

    fn index_for_position(
        bounds: Bounds<Pixels>,
        pos: Point<Pixels>,
        count: usize,
    ) -> Option<usize> {
        let w = f32::from(bounds.size.width);
        if w <= 4. || count < 2 {
            return None;
        }
        let rel = ((f32::from(pos.x - bounds.left())) / w).clamp(0., 1.);
        Some((rel * (count - 1) as f32).round() as usize)
    }

    fn render_delta_pill(delta: &str, up: bool, is_good: bool, p: &ProPalette) -> Div {
        let (bg, fg) = if is_good {
            (p.green_soft, p.green_ink)
        } else {
            (p.red_soft, p.red_ink)
        };
        let tri = if up {
            TRIANGLE_UP_SVG
        } else {
            TRIANGLE_DOWN_SVG
        };
        div()
            .h(px(20.))
            .px(px(6.))
            .rounded(px(5.))
            .bg(bg)
            .text_color(fg)
            .text_size(px(12.))
            .font_weight(FontWeight::MEDIUM)
            .flex_none()
            .flex()
            .items_center()
            .gap(px(4.))
            .child(pro_svg("0 0 8 8", tri, 7., fg))
            .child(delta.to_owned())
    }

    fn parse_metric_magnitude(s: &str) -> f32 {
        let mut buf = String::with_capacity(s.len());
        for ch in s.chars() {
            if ch.is_ascii_digit() || ch == '.' {
                buf.push(ch);
            }
        }
        buf.parse::<f32>().unwrap_or(0.0)
    }

    fn render_mini_bars(
        card_id: ElementId,
        bars: &[f32],
        active: bool,
        p: &ProPalette,
        w: &mut Window,
        cx: &mut App,
    ) -> Div {
        let bar_color = if active {
            p.blue
        } else if p.dark {
            hsla(0., 0., 1., 0.26)
        } else {
            hsla(0., 0., 0., 0.22)
        };
        let mut row = div()
            .w(px(66.))
            .h(px(22.))
            .flex_none()
            .flex()
            .items_end()
            .justify_between()
            .gap(px(4.));
        for (bi, &v) in bars.iter().enumerate() {
            let target = if active {
                (v * 1.08).clamp(0.14, 1.0)
            } else {
                v.clamp(0.12, 1.0)
            };
            let anim_v =
                pro_spring_f32(pro_eid(&card_id, format!("bar-{bi}")), target, 0.28, w, cx);
            let h_px = (anim_v * 20.).round().max(3.);
            row = row.child(div().w(px(4.5)).h(px(h_px)).rounded(px(2.)).bg(bar_color));
        }
        row
    }

    fn render_sparkline(points: Vec<f32>, tone: AnalyticsSparkTone, p: &ProPalette) -> AnyElement {
        let stroke = match tone {
            AnalyticsSparkTone::Up => {
                if p.dark {
                    p.green
                } else {
                    p.green_ink
                }
            }
            AnalyticsSparkTone::Down => p.red,
            AnalyticsSparkTone::Flat => p.faint,
        };
        canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let w = f32::from(bounds.size.width);
                let h = f32::from(bounds.size.height);
                if w <= 2. || h <= 2. || points.len() < 2 {
                    return;
                }
                let pad = 1.5;
                let mut builder = PathBuilder::stroke(px(1.5));
                for (i, &v) in points.iter().enumerate() {
                    let tx = i as f32 / (points.len() - 1) as f32;
                    let ty = 1.0 - v.clamp(0., 1.);
                    let pt = point(
                        bounds.left() + px(tx * w),
                        bounds.top() + px(pad + ty * (h - 2. * pad)),
                    );
                    if i == 0 {
                        builder.move_to(pt);
                    } else {
                        builder.line_to(pt);
                    }
                }
                if let Ok(path) = builder.build() {
                    window.paint_path(path, stroke);
                }
            },
        )
        .w(px(40.))
        .h(px(14.))
        .flex_none()
        .into_any_element()
    }

    fn render_multiline_chart(
        &self,
        spec: MultiLineSpec<'_>,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> Div {
        let chart_kind = spec.chart;
        let bounds_for_canvas = spec.bounds_cell.clone();
        let bounds_for_move = spec.bounds_cell.clone();
        let count = spec.x_labels.len();
        let max_y = spec.max_y.max(0.001);
        let grid_color = p.line;
        let guide_color = p.line_strong;
        let card_bg = p.card;
        let hover_idx = spec.hover_idx;
        let series_for_canvas: Vec<(Hsla, Vec<f32>, bool)> = spec
            .series
            .iter()
            .map(|(_, c, vals, dim)| (*c, vals.clone(), *dim))
            .collect();

        let plot_canvas = canvas(
            move |b, _, _| {
                bounds_for_canvas.set(b);
            },
            move |bounds, _, window, _| {
                let w = f32::from(bounds.size.width);
                let h = f32::from(bounds.size.height);
                if w <= 4. || h <= 4. {
                    return;
                }
                // 3 horizontal grid hairlines (top, middle, bottom)
                for step in [0.0_f32, 0.5, 1.0] {
                    let y = bounds.top() + px(step * h);
                    let mut grid = PathBuilder::stroke(px(1.));
                    grid.move_to(point(bounds.left(), y));
                    grid.line_to(point(bounds.right(), y));
                    if let Ok(path) = grid.build() {
                        window.paint_path(path, grid_color);
                    }
                }

                // Series polylines
                for (color, vals, dimmed) in &series_for_canvas {
                    if vals.len() < 2 {
                        continue;
                    }
                    let mut line = PathBuilder::stroke(px(1.6));
                    for (i, &v) in vals.iter().enumerate() {
                        let tx = i as f32 / (vals.len() - 1) as f32;
                        let ty = 1.0 - (v / max_y).clamp(0.02, 0.985);
                        let pt = point(bounds.left() + px(tx * w), bounds.top() + px(ty * h));
                        if i == 0 {
                            line.move_to(pt);
                        } else {
                            line.line_to(pt);
                        }
                    }
                    if let Ok(path) = line.build() {
                        let c = if *dimmed { color.opacity(0.22) } else { *color };
                        window.paint_path(path, c);
                    }
                }

                // Hover guide & dots
                if let Some(idx) = hover_idx
                    && idx < count
                {
                    let tx = idx as f32 / (count - 1).max(1) as f32;
                    let gx = bounds.left() + px(tx * w);
                    let mut guide = PathBuilder::stroke(px(1.));
                    guide.move_to(point(gx, bounds.top()));
                    guide.line_to(point(gx, bounds.bottom()));
                    if let Ok(path) = guide.build() {
                        window.paint_path(path, guide_color);
                    }
                    for (color, vals, dimmed) in &series_for_canvas {
                        if *dimmed {
                            continue;
                        }
                        if let Some(&v) = vals.get(idx) {
                            let ty = 1.0 - (v / max_y).clamp(0.02, 0.985);
                            let gy = bounds.top() + px(ty * h);
                            window.paint_quad(
                                fill(
                                    Bounds::new(
                                        point(gx - px(5.), gy - px(5.)),
                                        size(px(10.), px(10.)),
                                    ),
                                    card_bg,
                                )
                                .corner_radii(px(5.)),
                            );
                            window.paint_quad(
                                fill(
                                    Bounds::new(
                                        point(gx - px(3.5), gy - px(3.5)),
                                        size(px(7.), px(7.)),
                                    ),
                                    *color,
                                )
                                .corner_radii(px(3.5)),
                            );
                        }
                    }
                }
            },
        )
        .size_full();

        let tooltip_overlay = hover_idx.and_then(|idx| {
            let date = *spec.x_labels.get(idx)?;
            let on_right = idx * 2 < count;
            let mut list = div()
                .mt(px(4.))
                .pt(px(4.))
                .border_t_1()
                .border_color(p.line)
                .flex()
                .flex_col()
                .gap(px(2.));
            for (name, color, vals, dimmed) in &spec.series {
                if *dimmed {
                    continue;
                }
                if let Some(&val) = vals.get(idx) {
                    let formatted = if spec.unit_suffix == "k" {
                        format!("{}{:.2}k", spec.unit_prefix, val)
                    } else {
                        format!("{}{:.0}{}", spec.unit_prefix, val, spec.unit_suffix)
                    };
                    list = list.child(
                        div()
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
                                    .child(div().size(px(8.)).rounded(px(2.)).bg(*color))
                                    .child(div().text_color(p.text).child(name.clone())),
                            )
                            .child(
                                div()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .child(formatted),
                            ),
                    );
                }
            }
            let pop_bg: Hsla = if p.dark {
                rgb(0x252527).into()
            } else {
                rgb(0xffffff).into()
            };
            let mut pop = div()
                .absolute()
                .top(px(2.))
                .min_w(px(154.))
                .px(px(10.))
                .py(px(6.))
                .rounded(px(10.))
                .bg(pop_bg)
                .border_1()
                .border_color(p.line_strong)
                .child(
                    div()
                        .text_size(px(12.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.strong)
                        .child(date.to_owned()),
                )
                .child(list);
            if on_right {
                pop = pop.right(px(8.));
            } else {
                pop = pop.left(px(8.));
            }
            Some(pop)
        });

        let id_suffix = match chart_kind {
            HoveredChart::Channels => "plot-channels",
            HoveredChart::Sessions => "plot-sessions",
            HoveredChart::Aov => "plot-aov",
        };

        let plot_box = div()
            .id(pro_eid(&self.id, id_suffix))
            .role(Role::Region)
            .aria_label(spec.aria)
            .tab_index(0)
            .relative()
            .flex_1()
            .min_w_0()
            .h(px(spec.height))
            .cursor_pointer()
            .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                if !hovered {
                    match chart_kind {
                        HoveredChart::Channels => this.hover_channel_day = None,
                        HoveredChart::Sessions => this.hover_sessions_day = None,
                        HoveredChart::Aov => this.hover_aov_day = None,
                    }
                    cx.notify();
                }
            }))
            .on_mouse_move(cx.listener(move |this, ev: &MouseMoveEvent, _, cx| {
                let next = Self::index_for_position(bounds_for_move.get(), ev.position, count);
                let slot = match chart_kind {
                    HoveredChart::Channels => &mut this.hover_channel_day,
                    HoveredChart::Sessions => &mut this.hover_sessions_day,
                    HoveredChart::Aov => &mut this.hover_aov_day,
                };
                if *slot != next {
                    *slot = next;
                    cx.notify();
                }
            }))
            .on_key_down(cx.listener(move |this, ev: &KeyDownEvent, _, cx| {
                let slot = match chart_kind {
                    HoveredChart::Channels => &mut this.hover_channel_day,
                    HoveredChart::Sessions => &mut this.hover_sessions_day,
                    HoveredChart::Aov => &mut this.hover_aov_day,
                };
                let cur = slot.unwrap_or(0);
                match ev.keystroke.key.as_str() {
                    "left" => {
                        *slot = Some(cur.saturating_sub(1));
                        cx.notify();
                    }
                    "right" => {
                        *slot = Some((cur + 1).min(count.saturating_sub(1)));
                        cx.notify();
                    }
                    "escape" => {
                        *slot = None;
                        cx.notify();
                    }
                    _ => {}
                }
            }))
            .child(plot_canvas)
            .when_some(tooltip_overlay, |d, pop| d.child(pop));

        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(6.))
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_start()
                    .gap(px(10.))
                    .child(
                        div()
                            .h(px(spec.height))
                            .flex_none()
                            .flex()
                            .flex_col()
                            .justify_between()
                            .items_end()
                            .text_size(px(11.))
                            .line_height(px(14.))
                            .text_color(p.muted)
                            .child(spec.y_ticks[0].to_owned())
                            .child(spec.y_ticks[1].to_owned())
                            .child(spec.y_ticks[2].to_owned()),
                    )
                    .child(plot_box),
            )
            .child(
                div()
                    .pl(px(30.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_size(px(11.))
                    .line_height(px(16.))
                    .text_color(p.muted)
                    .child(spec.x_labels[0].to_owned())
                    .child(spec.x_labels[3].to_owned())
                    .child(spec.x_labels[6].to_owned()),
            )
    }

    fn render_loading_content(
        &self,
        phase: f32,
        stats_cols_2: bool,
        stack_main: bool,
        stack_bottom: bool,
        p: &ProPalette,
        cx: &App,
    ) -> Div {
        let stats = self.effective_stats(cx);
        let stat_cards: Vec<Div> = stats
            .iter()
            .map(|st| {
                div()
                    .flex_1()
                    .min_w(px(160.))
                    .p(px(16.))
                    .pt(px(14.))
                    .rounded(px(12.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .items_end()
                    .justify_between()
                    .gap(px(12.))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(6.))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .text_color(p.text)
                                    .child(st.label.clone()),
                            )
                            .child(pro_skeleton_bar(92., 24., 5., phase, cx))
                            .child(pro_skeleton_bar(58., 20., 5., phase, cx)),
                    )
                    .child(pro_skeleton_bar(66., 22., 4., phase, cx))
            })
            .collect();

        let stats_grid = if stats_cols_2 {
            let mut col = div().flex().flex_col().gap(px(12.));
            let mut r1 = div().flex().gap(px(12.));
            let mut r2 = div().flex().gap(px(12.));
            for (idx, c) in stat_cards.into_iter().enumerate() {
                if idx < 2 {
                    r1 = r1.child(c);
                } else {
                    r2 = r2.child(c);
                }
            }
            col = col.child(r1).child(r2);
            col
        } else {
            div().flex().gap(px(12.)).children(stat_cards)
        };

        let main_chart_skel = div()
            .flex_1()
            .min_w_0()
            .rounded(px(12.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .flex()
            .flex_col()
            .child(
                div()
                    .p(px(16.))
                    .pt(px(14.))
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(div().text_size(px(13.)).text_color(p.text).child(tr(
                        cx,
                        "Revenue by channel",
                        "各渠道营收",
                    )))
                    .child(pro_skeleton_bar(96., 26., 5., phase, cx))
                    .child(pro_skeleton_bar(64., 20., 5., phase, cx))
                    .child(
                        div()
                            .mt(px(8.))
                            .w_full()
                            .h(px(184.))
                            .rounded(px(6.))
                            .bg(p.chip_hover.opacity(0.45)),
                    ),
            )
            .child(
                div()
                    .px(px(16.))
                    .py(px(10.))
                    .border_t_1()
                    .border_color(p.line)
                    .child(pro_skeleton_bar(96., 14., 4., phase, cx)),
            );

        let mut breakdown_rows = div().mt(px(12.)).flex().flex_col().gap(px(4.));
        for idx in 0..6usize {
            breakdown_rows = breakdown_rows.child(
                div()
                    .h(px(38.))
                    .px(px(8.))
                    .rounded(px(8.))
                    .when(idx % 2 == 1, |d| d.bg(p.hover))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(12.))
                    .child(pro_skeleton_bar(88., 13., 4., phase, cx))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(12.))
                            .child(pro_skeleton_bar(56., 13., 4., phase, cx))
                            .child(pro_skeleton_bar(40., 12., 4., phase, cx)),
                    ),
            );
        }

        let breakdown_skel = div()
            .when(!stack_main, |d| d.w(px(340.)).flex_none())
            .when(stack_main, |d| d.w_full())
            .p(px(16.))
            .pt(px(14.))
            .rounded(px(12.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .flex()
            .flex_col()
            .child(div().text_size(px(12.5)).text_color(p.muted).child(tr(
                cx,
                "Revenue breakdown",
                "营收构成",
            )))
            .child(breakdown_rows);

        let products_skel = div()
            .flex_1()
            .min_w_0()
            .p(px(16.))
            .pt(px(14.))
            .rounded(px(12.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .flex()
            .flex_col()
            .gap(px(14.))
            .child(div().text_size(px(12.5)).text_color(p.muted).child(tr(
                cx,
                "Top products",
                "热门产品",
            )))
            .children([180.0_f32, 130., 98., 56.].into_iter().map(|bw| {
                div()
                    .flex()
                    .flex_col()
                    .gap(px(7.))
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .child(pro_skeleton_bar(84., 13., 4., phase, cx))
                            .child(pro_skeleton_bar(52., 13., 4., phase, cx)),
                    )
                    .child(pro_skeleton_bar(bw, 6., 3., phase, cx))
            }));

        let mini_chart_skel = |title: &'static str| {
            div()
                .flex_1()
                .min_w_0()
                .rounded(px(12.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .flex()
                .flex_col()
                .child(
                    div()
                        .p(px(16.))
                        .pt(px(14.))
                        .flex()
                        .flex_col()
                        .gap(px(6.))
                        .child(div().text_size(px(13.)).text_color(p.text).child(title))
                        .child(pro_skeleton_bar(76., 24., 5., phase, cx))
                        .child(pro_skeleton_bar(52., 20., 5., phase, cx))
                        .child(
                            div()
                                .mt(px(8.))
                                .w_full()
                                .h(px(116.))
                                .rounded(px(6.))
                                .bg(p.chip_hover.opacity(0.45)),
                        ),
                )
                .child(
                    div()
                        .px(px(16.))
                        .py(px(10.))
                        .border_t_1()
                        .border_color(p.line)
                        .child(pro_skeleton_bar(124., 13., 4., phase, cx)),
                )
        };

        div()
            .px(px(16.))
            .pb(px(16.))
            .flex()
            .flex_col()
            .gap(px(12.))
            .child(stats_grid)
            .child(
                div()
                    .flex()
                    .when(stack_main, |d| d.flex_col())
                    .gap(px(12.))
                    .child(main_chart_skel)
                    .child(breakdown_skel),
            )
            .child(
                div()
                    .flex()
                    .when(stack_bottom, |d| d.flex_col())
                    .gap(px(12.))
                    .child(products_skel)
                    .child(mini_chart_skel(tr(cx, "Sessions over time", "会话趋势")))
                    .child(mini_chart_skel(tr(cx, "Average order value", "客单价趋势"))),
            )
    }

    fn render_page(&mut self, w: &mut Window, cx: &mut Context<Self>) -> Stateful<Div> {
        if let Some((_, started)) = &self.toast
            && started.elapsed().as_secs_f32() >= 2.45
        {
            self.toast = None;
        }
        let p = ProPalette::get(cx);
        let reduced = Theme::global(cx).reduced_motion;
        let phase = if self.loading && !reduced {
            w.request_animation_frame();
            (self.anim_start.elapsed().as_secs_f32() * 0.9).fract()
        } else {
            0.35
        };

        let measure_cell = self.measured_width.clone();
        let main_w = measure_cell.get();
        let compact_head = main_w < 580.;
        let stats_cols_2 = main_w < 820.;
        let stack_main = main_w < 860.;
        let stack_bottom = main_w < 780.;

        // Sticky top header (`48px`)
        let filter_pills = {
            let mut pills = div().flex().items_center().gap(px(6.));
            if let Some(ch) = self.channel_filter.clone() {
                pills = pills.child(activate(
                    div()
                        .id(pro_eid(&self.id, "clear-channel-pill"))
                        .role(Role::Button)
                        .tab_index(0)
                        .h(px(22.))
                        .px(px(8.))
                        .rounded_full()
                        .bg(p.accent_soft)
                        .text_color(p.blue)
                        .text_size(px(11.5))
                        .font_weight(FontWeight::MEDIUM)
                        .flex()
                        .items_center()
                        .gap(px(4.))
                        .cursor_pointer()
                        .child(ch)
                        .child(pro_icon(ProIcon::Close, 11., p.blue)),
                    cx,
                    |this, _, cx| {
                        this.channel_filter = None;
                        let rev: SharedString = tr(cx, "Revenue", "营收").into();
                        this.shell.update(cx, |s, _| s.set_active(rev));
                        cx.emit(AnalyticsEvent::FilterChannel(None));
                        cx.notify();
                    },
                ));
            }
            if let Some(prod) = self.product_filter.clone() {
                pills = pills.child(activate(
                    div()
                        .id(pro_eid(&self.id, "clear-product-pill"))
                        .role(Role::Button)
                        .tab_index(0)
                        .h(px(22.))
                        .px(px(8.))
                        .rounded_full()
                        .bg(p.accent_soft)
                        .text_color(p.blue)
                        .text_size(px(11.5))
                        .font_weight(FontWeight::MEDIUM)
                        .flex()
                        .items_center()
                        .gap(px(4.))
                        .cursor_pointer()
                        .child(prod)
                        .child(pro_icon(ProIcon::Close, 11., p.blue)),
                    cx,
                    |this, _, cx| {
                        this.product_filter = None;
                        cx.emit(AnalyticsEvent::FilterProduct(None));
                        cx.notify();
                    },
                ));
            }
            pills
        };

        let period_btn = activate(
            div()
                .id(pro_eid(&self.id, "period-btn"))
                .role(Role::Button)
                .aria_label(self.period.label(cx))
                .tab_index(0)
                .h(px(28.))
                .px(px(10.))
                .rounded(px(9.))
                .bg(p.card)
                .border_1()
                .border_color(p.line_strong)
                .flex()
                .items_center()
                .gap(px(6.))
                .text_size(px(12.5))
                .text_color(p.text)
                .cursor_pointer()
                .hover(move |s| s.bg(p.hover).text_color(p.ink))
                .child(pro_icon(ProIcon::Calendar, 14., p.muted))
                .child(self.period.label(cx))
                .child(pro_icon(ProIcon::ChevronDown, 12., p.muted)),
            cx,
            |this, _, cx| {
                this.period_menu_open = !this.period_menu_open;
                cx.notify();
            },
        );

        let save_label = if self.saved_report {
            tr(cx, "Saved report", "已保存报表")
        } else {
            tr(cx, "Save as report", "存为报表")
        };

        let header = div()
            .min_h(px(48.))
            .pl(px(if !self.standalone && compact_head {
                48.
            } else {
                20.
            }))
            .pr(px(12.))
            .py(px(8.))
            .flex_none()
            .bg(p.panel)
            .flex()
            .flex_wrap()
            .items_center()
            .justify_between()
            .gap(px(8.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .text_size(px(13.))
                    .child(
                        div()
                            .text_color(p.muted)
                            .child(tr(cx, "Analytics", "数据分析")),
                    )
                    .child(div().text_color(p.faint).child("/"))
                    .child(
                        div()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(tr(cx, "Revenue", "营收")),
                    )
                    .child(filter_pills),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(period_btn)
                    .child(pro_btn(
                        pro_eid(&self.id, "export-btn"),
                        tr(cx, "Export", "导出"),
                        Some(ProIcon::Download),
                        ProBtnStyle::Secondary,
                        true,
                        cx,
                        |this, _, cx| {
                            let slug = this.period.slug();
                            this.toast = Some((
                                format!("{} revenue-{slug}.csv", tr(cx, "Exported", "已导出"))
                                    .into(),
                                Instant::now(),
                            ));
                            cx.emit(AnalyticsEvent::Export);
                            cx.notify();
                        },
                    ))
                    .child(pro_btn(
                        pro_eid(&self.id, "save-report-btn"),
                        save_label,
                        if self.saved_report {
                            Some(ProIcon::Check)
                        } else {
                            None
                        },
                        ProBtnStyle::Primary,
                        true,
                        cx,
                        |this, _, cx| {
                            this.saved_report = true;
                            this.toast = Some((
                                tr(cx, "Saved to workspace reports", "已保存至工作区报表").into(),
                                Instant::now(),
                            ));
                            cx.emit(AnalyticsEvent::SaveReport);
                            cx.notify();
                        },
                    )),
            );

        // Main scrollable body
        let body = if self.empty {
            div()
                .id(pro_eid(&self.id, "empty-body"))
                .flex_1()
                .min_h_0()
                .flex()
                .items_center()
                .justify_center()
                .child(pro_empty_state(
                    ProIcon::UsageChart,
                    tr(cx, "No revenue yet", "暂无营收数据"),
                    tr(
                        cx,
                        "Once orders come in, key figures, channels, breakdown, and top products show up here.",
                        "接入订单后，关键营收指标、渠道趋势、构成明细与热门产品将在此展示。",
                    ),
                    Some((
                        pro_eid(&self.id, "connect-store-btn"),
                        tr(cx, "Connect store", "连接店铺").into(),
                        Box::new(|this: &mut Self, _, cx| {
                            this.empty = false;
                            cx.emit(AnalyticsEvent::ConnectStore);
                            cx.notify();
                        }),
                    )),
                    cx,
                ))
        } else if self.loading {
            div()
                .id(pro_eid(&self.id, "loading-scroll"))
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .child(self.render_loading_content(
                    phase,
                    stats_cols_2,
                    stack_main,
                    stack_bottom,
                    &p,
                    cx,
                ))
        } else {
            let stats = self.effective_stats(cx);
            let selected_stat = self.selected_stat;
            let mut stat_els: Vec<Stateful<Div>> = Vec::with_capacity(stats.len());
            for (idx, st) in stats.iter().enumerate() {
                let is_sel = selected_stat == Some(idx);
                let is_dim = selected_stat.is_some_and(|sel| sel != idx);
                let card_eid = pro_eid(&self.id, format!("stat-{idx}"));
                let mini_bars =
                    Self::render_mini_bars(card_eid.clone(), &st.bars, is_sel, &p, w, cx);
                stat_els.push(activate(
                    div()
                        .id(card_eid)
                        .role(Role::Button)
                        .aria_label(format!("{}: {}", st.label, st.value))
                        .tab_index(0)
                        .flex_1()
                        .min_w(px(150.))
                        .p(px(16.))
                        .pt(px(14.))
                        .rounded(px(12.))
                        .bg(p.card)
                        .border_1()
                        .border_color(if is_sel { p.blue } else { p.line })
                        .when(is_dim, |d| d.opacity(0.52))
                        .cursor_pointer()
                        .hover(move |s| {
                            s.border_color(if is_sel { p.blue } else { p.line_strong })
                                .opacity(1.0)
                        })
                        .flex()
                        .items_end()
                        .justify_between()
                        .gap(px(12.))
                        .child(
                            div()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .line_height(px(20.))
                                        .text_color(p.text)
                                        .truncate()
                                        .child(st.label.clone()),
                                )
                                .child(
                                    div()
                                        .mt(px(2.))
                                        .text_size(px(24.))
                                        .line_height(px(32.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.strong)
                                        .child(st.value.clone()),
                                )
                                .child(div().mt(px(4.)).flex().items_center().child(
                                    Self::render_delta_pill(&st.delta, st.up, st.is_good(), &p),
                                )),
                        )
                        .child(mini_bars),
                    cx,
                    move |this, _, cx| {
                        if this.selected_stat == Some(idx) {
                            this.selected_stat = None;
                        } else {
                            this.selected_stat = Some(idx);
                        }
                        cx.emit(AnalyticsEvent::SelectStat(this.selected_stat));
                        cx.notify();
                    },
                ));
            }

            let stats_grid = if stats_cols_2 {
                let mut col = div().flex().flex_col().gap(px(12.));
                let mut r1 = div().flex().gap(px(12.));
                let mut r2 = div().flex().gap(px(12.));
                for (i, card) in stat_els.into_iter().enumerate() {
                    if i < 2 {
                        r1 = r1.child(card);
                    } else {
                        r2 = r2.child(card);
                    }
                }
                col = col.child(r1).child(r2);
                col
            } else {
                div().flex().gap(px(12.)).children(stat_els)
            };

            // Main channel chart card
            let channels = self.effective_channels(cx);
            let channel_colors = [
                rgb(0x4fb0fe).into(),
                rgb(0xee3e9e).into(),
                rgb(0xa651f0).into(),
                rgb(0x13b3a0).into(),
            ];
            let x_labels = self.period.x_labels();
            let active_ch_filter = self.channel_filter.clone();
            let channel_series: Vec<(SharedString, Hsla, Vec<f32>, bool)> = channels
                .iter()
                .enumerate()
                .map(|(i, ch)| {
                    let c = channel_colors[i % channel_colors.len()];
                    let dimmed = active_ch_filter
                        .as_ref()
                        .is_some_and(|flt| flt != &ch.label);
                    (ch.label.clone(), c, ch.values.clone(), dimmed)
                })
                .collect();

            let scale = self.combined_scale();
            let channel_headline = if (scale - 1.0).abs() < 1e-3 {
                "$96.4k".to_string()
            } else {
                format!("${:.1}k", 96.4 * scale)
            };
            let channel_delta = if (scale - 1.0).abs() < 1e-3 {
                "$7.9k".to_string()
            } else {
                format!("${:.1}k", 7.9 * scale)
            };

            let channel_chart = self.render_multiline_chart(
                MultiLineSpec {
                    chart: HoveredChart::Channels,
                    aria: tr(cx, "Revenue by channel", "各渠道营收趋势").into(),
                    y_ticks: ["$6k", "$3k", "$0"],
                    x_labels,
                    max_y: 6.0,
                    height: 172.,
                    series: channel_series,
                    hover_idx: self.hover_channel_day,
                    bounds_cell: self.channel_plot_bounds.clone(),
                    unit_prefix: "$",
                    unit_suffix: "k",
                },
                &p,
                cx,
            );

            let legend_chips = {
                let ch_sensor = pro_selection_sensor();
                let mut row = div()
                    .relative()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(10.));
                if let Some(active_ch) = &self.channel_filter {
                    row = row.child(pro_pill_indicator(
                        pro_eid(&self.id, "ch-leg-pill"),
                        active_ch.clone(),
                        ch_sensor.clone(),
                        5.,
                        p.chip_hover,
                        None,
                        false,
                    ));
                }
                for (i, ch) in channels.iter().enumerate() {
                    let dot_c = channel_colors[i % channel_colors.len()];
                    let is_active = self.channel_filter.as_ref() == Some(&ch.label);
                    let lbl = ch.label.clone();
                    row = row.child(activate(
                        div()
                            .id(pro_eid(&self.id, format!("ch-leg-{}", ch.id)))
                            .role(Role::Button)
                            .tab_index(0)
                            .relative()
                            .px(px(5.))
                            .py(px(2.))
                            .rounded(px(5.))
                            .when(is_active, |d| {
                                d.text_color(p.ink).child(pro_mark_selected(&ch_sensor))
                            })
                            .flex()
                            .items_center()
                            .gap(px(5.))
                            .cursor_pointer()
                            .when(!is_active, |d| {
                                d.hover(move |s| s.bg(p.hover).text_color(p.ink))
                            })
                            .child(div().size(px(6.)).rounded_full().bg(dot_c))
                            .child(lbl.clone()),
                        cx,
                        move |this, _, cx| {
                            this.toggle_channel_filter(lbl.clone(), cx);
                        },
                    ));
                }
                row
            };

            let main_chart_card = div()
                .flex_1()
                .min_w_0()
                .rounded(px(12.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .flex()
                .flex_col()
                .child(
                    div()
                        .p(px(16.))
                        .pt(px(14.))
                        .flex_1()
                        .flex()
                        .flex_col()
                        .gap(px(12.))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .line_height(px(20.))
                                        .text_color(p.text)
                                        .child(tr(cx, "Revenue by channel", "各渠道营收")),
                                )
                                .child(
                                    div()
                                        .mt(px(2.))
                                        .text_size(px(24.))
                                        .line_height(px(32.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.strong)
                                        .child(channel_headline),
                                )
                                .child(div().mt(px(4.)).flex().items_center().child(
                                    Self::render_delta_pill(&channel_delta, true, true, &p),
                                )),
                        )
                        .child(channel_chart),
                )
                .child(
                    div()
                        .px(px(16.))
                        .py(px(9.))
                        .border_t_1()
                        .border_color(p.line)
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .justify_between()
                        .gap(px(8.))
                        .text_size(px(12.))
                        .text_color(p.muted)
                        .child(tr(cx, "Top 4 channels", "前 4 大渠道"))
                        .child(legend_chips),
                );

            // Right Revenue breakdown card (`340px`)
            let breakdown = self.effective_breakdown(cx);
            let max_bd = breakdown
                .iter()
                .map(|r| Self::parse_metric_magnitude(&r.value))
                .fold(1.0_f32, f32::max);
            let mut bd_list = div().mt(px(12.)).mx(px(-6.)).flex().flex_col();
            for (idx, item) in breakdown.iter().enumerate() {
                let bd_mag = Self::parse_metric_magnitude(&item.value);
                let bd_frac =
                    ((bd_mag / max_bd) * (0.72 + 0.28 * scale.min(1.35))).clamp(0.05, 1.0);
                let bd_bar_color = match item.tone {
                    AnalyticsSparkTone::Up => {
                        if p.dark {
                            p.green
                        } else {
                            p.green_ink
                        }
                    }
                    AnalyticsSparkTone::Down => p.red,
                    AnalyticsSparkTone::Flat => p.blue,
                };
                bd_list = bd_list.child(
                    div()
                        .h(px(38.))
                        .px(px(8.))
                        .rounded(px(8.))
                        .when(idx % 2 == 1, |d| d.bg(p.hover))
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(10.))
                        .text_size(px(13.))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .text_color(p.muted)
                                .child(item.label.clone()),
                        )
                        .child(div().w(px(40.)).flex_none().child(pro_spring_bar(
                            pro_eid(&self.id, format!("bd-bar-{idx}")),
                            bd_frac,
                            4.,
                            2.,
                            p.chip,
                            bd_bar_color,
                        )))
                        .child(
                            div()
                                .flex_none()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.ink)
                                .child(item.value.clone()),
                        )
                        .child(Self::render_sparkline(item.spark.clone(), item.tone, &p)),
                );
            }

            let breakdown_card = div()
                .when(!stack_main, |d| d.w(px(340.)).flex_none())
                .when(stack_main, |d| d.w_full())
                .p(px(16.))
                .pt(px(14.))
                .rounded(px(12.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_size(px(12.5))
                        .line_height(px(18.))
                        .text_color(p.muted)
                        .child(tr(cx, "Revenue breakdown", "营收构成")),
                )
                .child(bd_list);

            // Bottom Row Col 1: Top products
            let products = self.effective_products(cx);
            let bar_fill: Hsla = rgb(0x4fb0fe).into();
            let mut prod_list = div().flex().flex_col().gap(px(14.));
            for (idx, prod) in products.iter().enumerate() {
                let is_filtered = self.product_filter.as_ref() == Some(&prod.label);
                let is_dimmed = self
                    .product_filter
                    .as_ref()
                    .is_some_and(|flt| flt != &prod.label);
                let lbl = prod.label.clone();
                let frac = (prod.share / 100.0).clamp(0.04, 1.0);
                prod_list = prod_list.child(activate(
                    div()
                        .id(pro_eid(&self.id, format!("product-{idx}")))
                        .role(Role::Button)
                        .tab_index(0)
                        .rounded(px(6.))
                        .when(is_dimmed, |d| d.opacity(0.45))
                        .cursor_pointer()
                        .hover(|s| s.opacity(1.0))
                        .flex()
                        .flex_col()
                        .gap(px(7.))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap(px(12.))
                                .text_size(px(13.))
                                .child(
                                    div()
                                        .min_w_0()
                                        .truncate()
                                        .text_color(if is_filtered { p.ink } else { p.muted })
                                        .when(is_filtered, |d| d.font_weight(FontWeight::MEDIUM))
                                        .child(prod.label.clone()),
                                )
                                .child(
                                    div()
                                        .flex_none()
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.ink)
                                        .child(prod.value.clone()),
                                ),
                        )
                        .child(pro_spring_bar(
                            pro_eid(&self.id, format!("prod-bar-{idx}")),
                            if is_dimmed {
                                (frac * 0.65).clamp(0.04, 1.0)
                            } else {
                                frac
                            },
                            6.,
                            3.,
                            p.chip,
                            bar_fill,
                        )),
                    cx,
                    move |this, _, cx| {
                        this.toggle_product_filter(lbl.clone(), cx);
                    },
                ));
            }

            let products_card = div()
                .flex_1()
                .min_w_0()
                .p(px(16.))
                .pt(px(14.))
                .rounded(px(12.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .flex()
                .flex_col()
                .gap(px(14.))
                .child(
                    div()
                        .text_size(px(12.5))
                        .line_height(px(18.))
                        .text_color(p.muted)
                        .child(tr(cx, "Top products", "热门产品")),
                )
                .child(prod_list);

            // Bottom Row Col 2: Sessions over time
            let cmp_blue: Hsla = if p.dark {
                rgb(0x4fb0fe).into()
            } else {
                rgb(0xa6d7fe).into()
            };
            let cmp_color = if p.dark {
                cmp_blue.opacity(0.45)
            } else {
                cmp_blue
            };
            let sessions_chart = self.render_multiline_chart(
                MultiLineSpec {
                    chart: HoveredChart::Sessions,
                    aria: tr(cx, "Sessions over time", "会话趋势").into(),
                    y_ticks: ["3k", "1.5k", "0"],
                    x_labels,
                    max_y: 3.0,
                    height: 108.,
                    series: vec![
                        (
                            tr(cx, "This week", "本周").into(),
                            rgb(0x4fb0fe).into(),
                            vec![1.82, 2.14, 1.96, 2.48, 2.71, 2.39, 2.95],
                            false,
                        ),
                        (
                            tr(cx, "Last week", "上周").into(),
                            cmp_color,
                            vec![1.65, 1.72, 1.88, 1.79, 2.01, 1.94, 2.10],
                            false,
                        ),
                    ],
                    hover_idx: self.hover_sessions_day,
                    bounds_cell: self.sessions_plot_bounds.clone(),
                    unit_prefix: "",
                    unit_suffix: "k",
                },
                &p,
                cx,
            );

            let sessions_val = if (scale - 1.0).abs() < 1e-3 {
                "16.4k".to_string()
            } else {
                format!("{:.1}k", 16.4 * scale.powf(0.85))
            };

            let sessions_card = div()
                .flex_1()
                .min_w_0()
                .rounded(px(12.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .flex()
                .flex_col()
                .child(
                    div()
                        .p(px(16.))
                        .pt(px(14.))
                        .flex_1()
                        .flex()
                        .flex_col()
                        .gap(px(12.))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .line_height(px(20.))
                                        .text_color(p.text)
                                        .child(tr(cx, "Sessions over time", "会话趋势")),
                                )
                                .child(
                                    div()
                                        .mt(px(2.))
                                        .text_size(px(24.))
                                        .line_height(px(32.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.strong)
                                        .child(sessions_val),
                                )
                                .child(
                                    div()
                                        .mt(px(4.))
                                        .flex()
                                        .items_center()
                                        .child(Self::render_delta_pill("2.3k", true, true, &p)),
                                ),
                        )
                        .child(sessions_chart),
                )
                .child(
                    div()
                        .px(px(16.))
                        .py(px(10.))
                        .border_t_1()
                        .border_color(p.line)
                        .text_size(px(12.))
                        .text_color(p.muted)
                        .child(tr(cx, "This week vs. last week", "本周对比上周")),
                );

            // Bottom Row Col 3: Average order value
            let aov_chart = self.render_multiline_chart(
                MultiLineSpec {
                    chart: HoveredChart::Aov,
                    aria: tr(cx, "Average order value", "平均客单价").into(),
                    y_ticks: ["$160", "$80", "$0"],
                    x_labels,
                    max_y: 160.0,
                    height: 108.,
                    series: vec![
                        (
                            tr(cx, "This week", "本周").into(),
                            rgb(0x4fb0fe).into(),
                            vec![118.0, 124.0, 121.0, 132.0, 129.0, 137.0, 141.0],
                            false,
                        ),
                        (
                            tr(cx, "Last week", "上周").into(),
                            cmp_color,
                            vec![112.0, 115.0, 119.0, 117.0, 121.0, 120.0, 124.0],
                            false,
                        ),
                    ],
                    hover_idx: self.hover_aov_day,
                    bounds_cell: self.aov_plot_bounds.clone(),
                    unit_prefix: "$",
                    unit_suffix: "",
                },
                &p,
                cx,
            );

            let aov_card = div()
                .flex_1()
                .min_w_0()
                .rounded(px(12.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .flex()
                .flex_col()
                .child(
                    div()
                        .p(px(16.))
                        .pt(px(14.))
                        .flex_1()
                        .flex()
                        .flex_col()
                        .gap(px(12.))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .line_height(px(20.))
                                        .text_color(p.text)
                                        .child(tr(cx, "Average order value", "平均客单价")),
                                )
                                .child(
                                    div()
                                        .mt(px(2.))
                                        .text_size(px(24.))
                                        .line_height(px(32.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.strong)
                                        .child("$141"),
                                )
                                .child(
                                    div()
                                        .mt(px(4.))
                                        .flex()
                                        .items_center()
                                        .child(Self::render_delta_pill("$17", true, true, &p)),
                                ),
                        )
                        .child(aov_chart),
                )
                .child(
                    div()
                        .px(px(16.))
                        .py(px(10.))
                        .border_t_1()
                        .border_color(p.line)
                        .text_size(px(12.))
                        .text_color(p.muted)
                        .child(tr(cx, "This week vs. last week", "本周对比上周")),
                );

            let dash_key = format!(
                "{}-{:?}-{:?}-{:?}",
                self.period.slug(),
                self.selected_stat,
                self.channel_filter,
                self.product_filter
            );
            div()
                .id(pro_eid(&self.id, "body-scroll"))
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .child(pro_view_transition(
                    pro_eid(&self.id, "dashboard-vtx"),
                    dash_key,
                    div()
                        .px(px(16.))
                        .pb(px(16.))
                        .flex()
                        .flex_col()
                        .gap(px(12.))
                        .child(stats_grid)
                        .child(
                            div()
                                .flex()
                                .when(stack_main, |d| d.flex_col())
                                .gap(px(12.))
                                .child(main_chart_card)
                                .child(breakdown_card),
                        )
                        .child(
                            div()
                                .flex()
                                .when(stack_bottom, |d| d.flex_col())
                                .gap(px(12.))
                                .child(products_card)
                                .child(sessions_card)
                                .child(aov_card),
                        ),
                ))
        };

        let mut page = div()
            .id(pro_eid(&self.id, "page"))
            .track_focus(&self.focus)
            .relative()
            .size_full()
            .min_h_0()
            .bg(p.panel)
            .flex()
            .flex_col()
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _, cx| {
                match ev.keystroke.key.as_str() {
                    "1" => this.set_period(AnalyticsPeriod::Last24Hours, cx),
                    "2" => this.set_period(AnalyticsPeriod::Last7Days, cx),
                    "3" => this.set_period(AnalyticsPeriod::Last30Days, cx),
                    "4" => this.set_period(AnalyticsPeriod::Last90Days, cx),
                    "escape" => {
                        this.period_menu_open = false;
                        this.toast = None;
                        this.hover_channel_day = None;
                        this.hover_sessions_day = None;
                        this.hover_aov_day = None;
                        if this.channel_filter.is_some()
                            || this.product_filter.is_some()
                            || this.selected_stat.is_some()
                        {
                            this.clear_filters(cx);
                        } else {
                            cx.notify();
                        }
                    }
                    _ => {}
                }
            }))
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
            .child(header)
            .child(body);

        // Animated period dropdown menu overlay
        {
            let pop_bg: Hsla = if p.dark {
                rgb(0x1b1b1e).into()
            } else {
                rgb(0xffffff).into()
            };
            let period_sensor = pro_selection_sensor();
            let scrim_el = div()
                .id(pro_eid(&self.id, "period-scrim"))
                .absolute()
                .inset_0()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, _, cx| {
                        this.period_menu_open = false;
                        cx.notify();
                    }),
                );
            let menu_el = div()
                .id(pro_eid(&self.id, "period-menu"))
                .role(Role::Menu)
                .absolute()
                .right(px(148.))
                .min_w(px(172.))
                .p(px(4.))
                .rounded(px(12.))
                .bg(pop_bg)
                .border_1()
                .border_color(p.line_strong)
                .shadow_md()
                .flex()
                .flex_col()
                .gap(px(1.))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(pro_pill_indicator(
                    pro_eid(&self.id, "period-pill"),
                    self.period.slug(),
                    period_sensor.clone(),
                    8.,
                    p.select,
                    None,
                    false,
                ))
                .children(AnalyticsPeriod::all().into_iter().map(|opt| {
                    let active = self.period == opt;
                    activate(
                        div()
                            .id(pro_eid(&self.id, format!("period-{}", opt.slug())))
                            .role(Role::MenuItem)
                            .tab_index(0)
                            .relative()
                            .h(px(32.))
                            .px(px(10.))
                            .rounded(px(8.))
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(12.))
                            .text_size(px(13.))
                            .text_color(if active { p.ink } else { p.text })
                            .when(active, |d| {
                                d.font_weight(FontWeight::MEDIUM)
                                    .child(pro_mark_selected(&period_sensor))
                            })
                            .cursor_pointer()
                            .when(!active, |d| {
                                d.hover(move |s| s.bg(p.hover).text_color(p.ink))
                            })
                            .child(opt.label(cx))
                            .when(active, |d| d.child(pro_icon(ProIcon::Check, 14., p.ink))),
                        cx,
                        move |this, _, cx| {
                            this.set_period(opt, cx);
                        },
                    )
                }));
            let menu_open = self.period_menu_open;
            page = page.child(pro_presence(
                pro_eid(&self.id, "period-menu-pres"),
                menu_open,
                0.18,
                0.12,
                move |a| {
                    div()
                        .absolute()
                        .inset_0()
                        .when(menu_open, |d| d.child(scrim_el))
                        .child(menu_el.top(px(44. - 5. * (1. - a))).opacity(a))
                        .into_any_element()
                },
            ));
        }

        // Bottom animated toast feedback banner
        page = page.child(pro_toast_banner(self.toast.clone()));

        page
    }
}

impl Render for AnalyticsScreen {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.standalone {
            return self.render_page(w, cx).into_any_element();
        }
        self.shell.clone().into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AnalyticsGoodDir, AnalyticsPeriod, AnalyticsStat, channel_filter_scale, format_dollars,
        product_filter_scale,
    };

    #[test]
    fn stat_good_direction_evaluates_refund_and_revenue_deltas() {
        let revenue = AnalyticsStat {
            label: "Gross revenue".into(),
            value: "$96,412".into(),
            delta: "12.5%".into(),
            bars: vec![0.5, 1.0],
            good: AnalyticsGoodDir::Up,
            up: true,
        };
        let refund = AnalyticsStat {
            label: "Refund rate".into(),
            value: "1.8%".into(),
            delta: "0.3%".into(),
            bars: vec![0.8, 0.4],
            good: AnalyticsGoodDir::Down,
            up: false,
        };
        let refund_spike = AnalyticsStat {
            up: true,
            ..refund.clone()
        };
        assert!(revenue.is_good());
        assert!(refund.is_good());
        assert!(!refund_spike.is_good());
    }

    #[test]
    fn period_and_filter_scaling_formats_currency_cleanly() {
        assert_eq!(format_dollars(96_412.), "$96,412");
        assert_eq!(format_dollars(-4_180.), "−$4,180");
        let scaled = 96_412.
            * AnalyticsPeriod::Last30Days.scale_factor()
            * channel_filter_scale(Some("Online store"))
            * product_filter_scale(Some("Annual plan"));
        assert!(scaled > 70_000. && scaled < 85_000.);
        assert_eq!(format_dollars(scaled), "$76,821");
    }
}
