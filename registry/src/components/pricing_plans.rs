//! Pricing section block with a capsule billing toggle, rolling plan prices, and collapsible details.
//!
//! Renders [`PricingPlansBlock`] as a standalone multi-tier pricing section with a capsule
//! `Monthly` / `Yearly (-20%)` billing switcher (`radius.full`), rolling numeric prices,
//! quota rows, app sync badges, per-card detail disclosures, and a collapsible feature
//! comparison matrix below the plan grid.

use crate::{
    components::pro_chrome::{
        ProBtnStyle, ProPalette, pro_app_tile, pro_btn, pro_eid, pro_empty_state, pro_fold,
        pro_mark_selected, pro_pill_indicator, pro_rolling_number, pro_selection_sensor,
        pro_skeleton_bar, pro_toast_banner, pro_view_transition, tr,
    },
    foundation::Theme,
    primitives::{
        control::{UiLanguage, activate},
        pro_icons::{ProIcon, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

const SVG_EYE_LIMIT: &str = r#"<path d="M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7S2 12 2 12Z" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"></path><circle cx="12" cy="12" r="3" stroke="currentColor" stroke-width="1.8"></circle>"#;
const SVG_USERS_LIMIT: &str = r#"<circle cx="9" cy="8" r="3.5" stroke="currentColor" stroke-width="1.8"></circle><path d="M2.5 20c.8-3.5 3.4-5.5 6.5-5.5s5.7 2 6.5 5.5M16 4.8a3.5 3.5 0 0 1 0 6.4M18.5 14.8c1.6.9 2.6 2.6 3 5.2" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"></path>"#;
const SVG_FOLDER_LIMIT: &str = r#"<path d="M3 7.5A2.5 2.5 0 0 1 5.5 5H9l2 2.5h7.5A2.5 2.5 0 0 1 21 10v7.5a2.5 2.5 0 0 1-2.5 2.5h-13A2.5 2.5 0 0 1 3 17.5Z" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"></path>"#;
const SVG_CHECK_SM: &str = r#"<path d="m3.5 8.5 3 3 6-7" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"></path>"#;
const SVG_DASH_SM: &str =
    r#"<path d="M3.5 8h9" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"></path>"#;

/// Billing cycle selected on [`PricingPlansBlock`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PricingBillingPeriod {
    /// Month-to-month subscription billing.
    Monthly,
    /// Annual subscription billing with a 20% discount.
    #[default]
    Yearly,
}

impl PricingBillingPeriod {
    /// Machine-readable identifier (`"monthly"` or `"yearly"`).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Monthly => "monthly",
            Self::Yearly => "yearly",
        }
    }

    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::Monthly => tr(cx, "Monthly", "按月计费"),
            Self::Yearly => tr(cx, "Yearly", "按年计费"),
        }
    }

    fn billed_note(self, cx: &App) -> &'static str {
        match self {
            Self::Monthly => tr(cx, "Billed monthly", "按月扣费"),
            Self::Yearly => tr(cx, "Billed yearly", "按年扣费"),
        }
    }
}

/// Leading semantic icon for a plan quota row.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PricingLimitIcon {
    /// Tracked monthly events or runs (`Eye`).
    #[default]
    Events,
    /// Included team seats (`Users`).
    Members,
    /// Active workspaces or projects (`Folder`).
    Workspaces,
}

/// One quota limit row inside a [`PricingPlan`] card.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PricingLimit {
    /// Leading quota icon.
    pub icon: PricingLimitIcon,
    /// Quota name (`"Events / mo"`, `"Seats"`, `"Workspaces"`).
    pub label: SharedString,
    /// Formatted limit value (`"10k"`, `"100k"`, `"Unlimited"`).
    pub value: SharedString,
}

impl PricingLimit {
    /// Construct a plan quota row.
    pub fn new(
        icon: PricingLimitIcon,
        label: impl Into<SharedString>,
        value: impl Into<SharedString>,
    ) -> Self {
        Self {
            icon,
            label: label.into(),
            value: value.into(),
        }
    }
}

/// One feature checklist line inside a [`PricingPlan`] card.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PricingFeature {
    /// Feature description.
    pub label: SharedString,
    /// Whether this tier includes the feature.
    pub included: bool,
}

impl PricingFeature {
    /// Construct an included or excluded feature line.
    pub fn new(label: impl Into<SharedString>, included: bool) -> Self {
        Self {
            label: label.into(),
            included,
        }
    }
}

/// One subscription tier card rendered inside [`PricingPlansBlock`].
#[derive(Clone, Debug, PartialEq)]
pub struct PricingPlan {
    /// Stable plan identifier (`"starter"`, `"growth"`, `"scale"`).
    pub id: SharedString,
    /// Display name of the tier.
    pub name: SharedString,
    /// Optional highlight pill (`Some("Popular")`).
    pub badge: Option<SharedString>,
    /// Monthly price per month (`None` renders `"Custom"`).
    pub monthly_price: Option<f64>,
    /// Yearly equivalent monthly price (`None` renders `"Custom"`).
    pub yearly_price: Option<f64>,
    /// Call-to-action button label (`"Start free trial"`).
    pub cta_label: SharedString,
    /// Emphasize this card with a primary button and accent border.
    pub highlighted: bool,
    /// Key quota rows always shown above the details toggle.
    pub limits: Vec<PricingLimit>,
    /// Sync cadence chip shown in the expanded details section.
    pub sync_label: Option<SharedString>,
    /// Connected app identifiers rendered as crisp tiles in the expanded details section.
    pub sync_apps: Vec<SharedString>,
    /// Feature checklist shown when details are expanded.
    pub features: Vec<PricingFeature>,
}

impl PricingPlan {
    /// Return the active price for `billing`, or `None` when custom-quoted.
    pub fn price_for(&self, billing: PricingBillingPeriod) -> Option<f64> {
        match billing {
            PricingBillingPeriod::Monthly => self.monthly_price,
            PricingBillingPeriod::Yearly => self.yearly_price,
        }
        .filter(|v| v.is_finite() && *v >= 0.0)
    }
}

/// One row inside the collapsible cross-tier feature comparison table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PricingComparisonRow {
    /// Category header (`"Usage & quotas"`, `"Integrations"`, `"Security & governance"`).
    pub category: SharedString,
    /// Capability name.
    pub feature: SharedString,
    /// Per-plan cell text or `"yes"` / `"no"` marker aligned with `plans`.
    pub values: Vec<SharedString>,
}

/// Events emitted by [`PricingPlansBlock`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PricingPlansEvent {
    /// User switched between `Monthly` and `Yearly` billing.
    BillingChanged(PricingBillingPeriod),
    /// User activated a plan card's call-to-action button.
    PlanSelected {
        /// Identifier of the chosen plan.
        plan_id: SharedString,
        /// Active billing cycle at selection time.
        billing: PricingBillingPeriod,
    },
    /// User expanded or collapsed the per-card feature details.
    DetailsToggled {
        /// Whether per-card details are now visible.
        open: bool,
    },
    /// User expanded or collapsed the bottom feature comparison table.
    ComparisonToggled {
        /// Whether the comparison matrix is now visible.
        open: bool,
    },
}

/// Resolve a currency symbol prefix and decimal precision for a price.
pub fn resolve_currency_prefix(currency: &str) -> SharedString {
    let trimmed = currency.trim();
    match trimmed.to_ascii_uppercase().as_str() {
        "" | "USD" => "$".into(),
        "EUR" => "€".into(),
        "GBP" => "£".into(),
        "JPY" | "CNY" => "¥".into(),
        "CAD" => "CA$".into(),
        "AUD" => "A$".into(),
        _ => SharedString::from(trimmed.to_owned()),
    }
}

/// Return `2` decimal places when `price` has fractional cents, otherwise `0`.
pub fn price_decimals(price: f64) -> usize {
    if (price - price.round()).abs() > 0.005 {
        2
    } else {
        0
    }
}

/// Default three-tier plan catalog (`Starter`, `Growth`, `Scale`).
pub fn default_pricing_plans() -> Vec<PricingPlan> {
    let all_apps: Vec<SharedString> = [
        "hubspot", "stripe", "gmail", "slack", "github", "drive", "calendar",
    ]
    .into_iter()
    .map(SharedString::from)
    .collect();

    vec![
        PricingPlan {
            id: "starter".into(),
            name: "Starter".into(),
            badge: None,
            monthly_price: Some(30.0),
            yearly_price: Some(24.0),
            cta_label: "Start free trial".into(),
            highlighted: false,
            limits: vec![
                PricingLimit::new(PricingLimitIcon::Events, "Tracked events", "10k"),
                PricingLimit::new(PricingLimitIcon::Members, "Team seats", "2"),
                PricingLimit::new(PricingLimitIcon::Workspaces, "Workspaces", "1"),
            ],
            sync_label: Some("Hourly sync".into()),
            sync_apps: all_apps.iter().take(4).cloned().collect(),
            features: vec![
                PricingFeature::new("Unlimited live boards & funnels", true),
                PricingFeature::new("Weekly Slack & email digests", true),
                PricingFeature::new("CSV & webhook event ingestion", true),
                PricingFeature::new("Role-based access controls", false),
                PricingFeature::new("SAML SSO & SCIM directory sync", false),
                PricingFeature::new("Dedicated SLA & audit log export", false),
            ],
        },
        PricingPlan {
            id: "growth".into(),
            name: "Growth".into(),
            badge: Some("Popular".into()),
            monthly_price: Some(99.0),
            yearly_price: Some(79.0),
            cta_label: "Start free trial".into(),
            highlighted: true,
            limits: vec![
                PricingLimit::new(PricingLimitIcon::Events, "Tracked events", "100k"),
                PricingLimit::new(PricingLimitIcon::Members, "Team seats", "10"),
                PricingLimit::new(PricingLimitIcon::Workspaces, "Workspaces", "3"),
            ],
            sync_label: Some("Real-time".into()),
            sync_apps: all_apps.clone(),
            features: vec![
                PricingFeature::new("Unlimited live boards & funnels", true),
                PricingFeature::new("Weekly Slack & email digests", true),
                PricingFeature::new("CSV & webhook event ingestion", true),
                PricingFeature::new("Role-based access controls", true),
                PricingFeature::new("SAML SSO & SCIM directory sync", false),
                PricingFeature::new("Dedicated SLA & audit log export", false),
            ],
        },
        PricingPlan {
            id: "scale".into(),
            name: "Scale".into(),
            badge: None,
            monthly_price: Some(249.0),
            yearly_price: Some(199.0),
            cta_label: "Start free trial".into(),
            highlighted: false,
            limits: vec![
                PricingLimit::new(PricingLimitIcon::Events, "Tracked events", "1M"),
                PricingLimit::new(PricingLimitIcon::Members, "Team seats", "Unlimited"),
                PricingLimit::new(PricingLimitIcon::Workspaces, "Workspaces", "10"),
            ],
            sync_label: Some("Real-time + Warehouse".into()),
            sync_apps: all_apps,
            features: vec![
                PricingFeature::new("Unlimited live boards & funnels", true),
                PricingFeature::new("Weekly Slack & email digests", true),
                PricingFeature::new("CSV & webhook event ingestion", true),
                PricingFeature::new("Role-based access controls", true),
                PricingFeature::new("SAML SSO & SCIM directory sync", true),
                PricingFeature::new("Dedicated SLA & audit log export", true),
            ],
        },
    ]
}

fn default_comparison_rows() -> Vec<PricingComparisonRow> {
    vec![
        PricingComparisonRow {
            category: "Usage & quotas".into(),
            feature: "Monthly tracked events".into(),
            values: vec!["10,000".into(), "100,000".into(), "1,000,000".into()],
        },
        PricingComparisonRow {
            category: "Usage & quotas".into(),
            feature: "Included team seats".into(),
            values: vec!["2 seats".into(), "10 seats".into(), "Unlimited".into()],
        },
        PricingComparisonRow {
            category: "Usage & quotas".into(),
            feature: "Data retention window".into(),
            values: vec!["90 days".into(), "12 months".into(), "36 months".into()],
        },
        PricingComparisonRow {
            category: "Integrations".into(),
            feature: "Stripe, HubSpot & Slack sync".into(),
            values: vec!["Hourly".into(), "Real-time".into(), "Real-time".into()],
        },
        PricingComparisonRow {
            category: "Integrations".into(),
            feature: "Snowflake & BigQuery export".into(),
            values: vec!["no".into(), "yes".into(), "yes".into()],
        },
        PricingComparisonRow {
            category: "Security & governance".into(),
            feature: "Role-based permissions".into(),
            values: vec!["no".into(), "yes".into(), "yes".into()],
        },
        PricingComparisonRow {
            category: "Security & governance".into(),
            feature: "SAML 2.0 SSO & SCIM provisioning".into(),
            values: vec!["no".into(), "no".into(), "yes".into()],
        },
        PricingComparisonRow {
            category: "Security & governance".into(),
            feature: "99.95% uptime SLA & priority support".into(),
            values: vec!["no".into(), "no".into(), "yes".into()],
        },
    ]
}

/// Standalone three-tier pricing block with a capsule billing switcher, rolling numbers, and foldable comparison matrix.
///
/// # Examples
///
/// ```ignore
/// use kui::components::pricing_plans::{PricingBillingPeriod, PricingPlansBlock};
///
/// let block = cx.new(|cx| {
///     PricingPlansBlock::new("pricing-plans", cx)
///         .with_default_billing(PricingBillingPeriod::Yearly)
/// });
/// ```
pub struct PricingPlansBlock {
    id: ElementId,
    title: Option<SharedString>,
    subtitle: Option<SharedString>,
    plans: Vec<PricingPlan>,
    comparison_rows: Vec<PricingComparisonRow>,
    billing: PricingBillingPeriod,
    currency: SharedString,
    details_open: bool,
    comparison_open: bool,
    selected_plan_id: Option<SharedString>,
    loading: bool,
    empty: bool,
    toast: Option<(SharedString, Instant)>,
    boot_at: Instant,
    focus: FocusHandle,
    measured_width: Rc<Cell<f32>>,
}

impl EventEmitter<PricingPlansEvent> for PricingPlansBlock {}

impl PricingPlansBlock {
    /// Create a [`PricingPlansBlock`] populated with the default `Starter`, `Growth`, and `Scale` tiers.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<UiLanguage>(|_, cx| cx.notify())
            .detach();
        Self {
            id: id.into(),
            title: None,
            subtitle: None,
            plans: default_pricing_plans(),
            comparison_rows: default_comparison_rows(),
            billing: PricingBillingPeriod::Yearly,
            currency: "$".into(),
            details_open: true,
            comparison_open: false,
            selected_plan_id: None,
            loading: false,
            empty: false,
            toast: None,
            boot_at: Instant::now(),
            focus: cx.focus_handle(),
            measured_width: Rc::new(Cell::new(894.0)),
        }
    }

    /// Replace the displayed plan tiers (`1..=3` sit side-by-side; `4` arrange `2x2`).
    pub fn with_plans(mut self, plans: Vec<PricingPlan>) -> Self {
        self.plans = plans;
        self
    }

    /// Override the section heading (`"Choose your plan"` by default).
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Override the section subtitle under the heading.
    pub fn with_subtitle(mut self, subtitle: impl Into<SharedString>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    /// Set the initial [`PricingBillingPeriod`] (`Yearly` by default).
    pub fn with_default_billing(mut self, billing: PricingBillingPeriod) -> Self {
        self.billing = billing;
        self
    }

    /// Set a currency symbol (`"$"`) or ISO code (`"EUR"`, `"GBP"`).
    pub fn with_currency(mut self, currency: impl Into<SharedString>) -> Self {
        self.currency = currency.into();
        self
    }

    /// Set whether per-card feature details start expanded (`true` by default).
    pub fn with_details_open(mut self, open: bool) -> Self {
        self.details_open = open;
        self
    }

    /// Set whether the bottom feature comparison table starts expanded (`false` by default).
    pub fn with_comparison_open(mut self, open: bool) -> Self {
        self.comparison_open = open;
        self
    }

    /// Configure skeleton loading placeholders.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Configure the empty state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Return the currently selected [`PricingBillingPeriod`].
    pub fn billing(&self) -> PricingBillingPeriod {
        self.billing
    }

    /// Return whether per-card details are currently expanded.
    pub fn is_details_open(&self) -> bool {
        self.details_open
    }

    /// Return whether the bottom comparison matrix is currently expanded.
    pub fn is_comparison_open(&self) -> bool {
        self.comparison_open
    }

    /// Switch the active billing cycle and emit [`PricingPlansEvent::BillingChanged`].
    pub fn set_billing(&mut self, billing: PricingBillingPeriod, cx: &mut Context<Self>) {
        if self.billing != billing {
            self.billing = billing;
            cx.emit(PricingPlansEvent::BillingChanged(billing));
            cx.notify();
        }
    }

    /// Toggle the per-card feature details disclosure.
    pub fn toggle_details(&mut self, cx: &mut Context<Self>) {
        self.details_open = !self.details_open;
        cx.emit(PricingPlansEvent::DetailsToggled {
            open: self.details_open,
        });
        cx.notify();
    }

    /// Toggle the collapsible feature comparison matrix below the cards.
    pub fn toggle_comparison(&mut self, cx: &mut Context<Self>) {
        self.comparison_open = !self.comparison_open;
        cx.emit(PricingPlansEvent::ComparisonToggled {
            open: self.comparison_open,
        });
        cx.notify();
    }

    /// Select a plan tier and emit [`PricingPlansEvent::PlanSelected`].
    pub fn select_plan(&mut self, plan_id: impl Into<SharedString>, cx: &mut Context<Self>) {
        let pid = plan_id.into();
        let plan_name = self
            .plans
            .iter()
            .find(|pl| pl.id == pid)
            .map(|pl| pl.name.clone())
            .unwrap_or_else(|| pid.clone());
        self.selected_plan_id = Some(pid.clone());
        let cycle = self.billing.label(cx);
        let msg = format!(
            "{} · {} ({cycle})",
            tr(cx, "Started 14-day trial", "已开启 14 天试用"),
            plan_name
        );
        self.toast = Some((msg.into(), Instant::now()));
        cx.emit(PricingPlansEvent::PlanSelected {
            plan_id: pid,
            billing: self.billing,
        });
        cx.notify();
    }

    fn handle_key_down(&mut self, ev: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        match ev.keystroke.key.as_str() {
            "left" => self.set_billing(PricingBillingPeriod::Monthly, cx),
            "right" => self.set_billing(PricingBillingPeriod::Yearly, cx),
            "c" => self.toggle_comparison(cx),
            "d" => self.toggle_details(cx),
            _ => {}
        }
    }

    fn render_limit_icon(kind: PricingLimitIcon, color: Hsla) -> AnyElement {
        let svg = match kind {
            PricingLimitIcon::Events => SVG_EYE_LIMIT,
            PricingLimitIcon::Members => SVG_USERS_LIMIT,
            PricingLimitIcon::Workspaces => SVG_FOLDER_LIMIT,
        };
        pro_svg("0 0 24 24", svg, 13.5, color)
    }

    fn render_billing_capsule(&self, p: &ProPalette, cx: &mut Context<Self>) -> Stateful<Div> {
        let sensor = pro_selection_sensor();
        let bill_key: SharedString = self.billing.as_str().into();
        let pill_fill: Hsla = if p.dark { rgb(0x2c2c30).into() } else { p.card };

        div()
            .id(pro_eid(&self.id, "billing-group"))
            .role(Role::Group)
            .aria_label(tr(cx, "Billing period", "计费周期"))
            .relative()
            .mt(px(18.))
            .p(px(3.))
            .rounded_full()
            .bg(p.chip)
            .border_1()
            .border_color(p.line_soft)
            .flex()
            .items_center()
            .gap(px(2.))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "billing-capsule-pill"),
                bill_key,
                sensor.clone(),
                999.,
                pill_fill,
                Some(p.line),
                !p.dark,
            ))
            .children(
                [PricingBillingPeriod::Monthly, PricingBillingPeriod::Yearly]
                    .into_iter()
                    .map(|period| {
                        let on = self.billing == period;
                        let s_ref = sensor.clone();
                        let label = period.label(cx);
                        activate(
                            div()
                                .id(pro_eid(&self.id, format!("bill-opt-{}", period.as_str())))
                                .role(Role::RadioButton)
                                .aria_label(label)
                                .aria_toggled(on.into())
                                .tab_index(0)
                                .relative()
                                .h(px(28.))
                                .px(px(13.))
                                .rounded_full()
                                .when(!on, |d| d.hover(move |s| s.text_color(p.ink)))
                                .focus_visible(move |s| s.border_1().border_color(p.focus))
                                .flex()
                                .items_center()
                                .justify_center()
                                .gap(px(6.))
                                .text_size(px(13.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(if on { p.ink } else { p.text })
                                .cursor_pointer()
                                .when(on, |d| d.child(pro_mark_selected(&s_ref)))
                                .child(label)
                                .when(period == PricingBillingPeriod::Yearly, |d| {
                                    d.child(
                                        div()
                                            .h(px(18.))
                                            .px(px(6.))
                                            .rounded_full()
                                            .bg(p.accent_soft)
                                            .flex()
                                            .items_center()
                                            .text_size(px(11.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.blue)
                                            .child("-20%"),
                                    )
                                }),
                            cx,
                            move |this, _, cx| {
                                this.set_billing(period, cx);
                            },
                        )
                    }),
            )
    }

    fn render_plan_card(
        &self,
        plan: &PricingPlan,
        currency_prefix: &SharedString,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> Div {
        let is_featured = plan.highlighted;
        let is_selected = self.selected_plan_id.as_ref() == Some(&plan.id);
        let border_col = if is_selected {
            p.blue
        } else if is_featured {
            p.blue.opacity(if p.dark { 0.45 } else { 0.34 })
        } else {
            p.line
        };

        let price_opt = plan.price_for(self.billing);
        let price_row = if let Some(val) = price_opt {
            let dec = price_decimals(val);
            div()
                .mt(px(10.))
                .flex()
                .items_baseline()
                .gap(px(6.))
                .child(pro_rolling_number(
                    pro_eid(&self.id, format!("plan-price-{}", plan.id)),
                    val,
                    dec,
                    currency_prefix.clone(),
                    "",
                    26.,
                    FontWeight::MEDIUM,
                    p.strong,
                ))
                .child(div().text_size(px(12.)).text_color(p.muted).child(tr(
                    cx,
                    "per month",
                    "/ 月",
                )))
        } else {
            div().mt(px(10.)).flex().items_baseline().gap(px(6.)).child(
                div()
                    .text_size(px(26.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .child(tr(cx, "Custom", "定制报价")),
            )
        };

        let billed_caption = if price_opt.is_some() {
            self.billing.billed_note(cx)
        } else {
            tr(cx, "Annual volume contract", "按年度企业合同计费")
        };

        let pid_cta = plan.id.clone();
        let cta_lbl: SharedString = if is_selected {
            tr(cx, "Selected", "已选择").into()
        } else {
            plan.cta_label.clone()
        };
        let cta_style = if is_featured {
            ProBtnStyle::Primary
        } else {
            ProBtnStyle::Secondary
        };

        let limits_list = div()
            .mt(px(18.))
            .pt(px(14.))
            .border_t_1()
            .border_color(p.line)
            .flex()
            .flex_col()
            .gap(px(9.))
            .children(plan.limits.iter().map(|lim| {
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(10.))
                    .text_size(px(13.))
                    .child(
                        div()
                            .min_w_0()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .text_color(p.text)
                            .child(Self::render_limit_icon(lim.icon, p.muted))
                            .child(div().truncate().child(lim.label.clone())),
                    )
                    .child(
                        div()
                            .flex_none()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(lim.value.clone()),
                    )
            }));

        let details_open = self.details_open;
        let toggle_lbl = if details_open {
            tr(cx, "Hide details", "收起详情")
        } else {
            tr(cx, "Show details", "展开详情")
        };
        let toggle_btn = activate(
            div()
                .id(pro_eid(&self.id, format!("toggle-det-{}", plan.id)))
                .role(Role::Button)
                .aria_label(toggle_lbl)
                .aria_expanded(details_open)
                .tab_index(0)
                .mt(px(12.))
                .mx_auto()
                .h(px(24.))
                .px(px(8.))
                .rounded(px(6.))
                .hover(move |s| s.bg(p.hover).text_color(p.ink))
                .focus_visible(move |s| s.border_1().border_color(p.focus))
                .flex()
                .items_center()
                .justify_center()
                .gap(px(4.))
                .text_size(px(12.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.muted)
                .cursor_pointer()
                .child(toggle_lbl)
                .child(pro_icon(
                    if details_open {
                        ProIcon::ChevronUp
                    } else {
                        ProIcon::ChevronDown
                    },
                    12.,
                    p.muted,
                )),
            cx,
            |this, _, cx| {
                this.toggle_details(cx);
            },
        );

        let sync_label = plan.sync_label.clone();
        let sync_apps = plan.sync_apps.clone();
        let features = plan.features.clone();
        let p_copy = *p;
        let sync_tiles: Vec<AnyElement> = sync_apps
            .iter()
            .take(7)
            .map(|app| pro_app_tile(app, 20., cx))
            .collect();
        let eyebrow_txt: SharedString = tr(cx, "INTEGRATIONS", "应用连接").into();
        let syncs_txt: SharedString = tr(cx, "Syncs with", "支持同步").into();

        let fold_el = pro_fold(
            pro_eid(&self.id, format!("card-fold-{}", plan.id)),
            details_open,
            210.,
            0.,
            move || {
                div()
                    .mt(px(12.))
                    .pt(px(14.))
                    .border_t_1()
                    .border_color(p_copy.line)
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(8.))
                            .child(
                                div()
                                    .text_size(px(10.5))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p_copy.muted)
                                    .child(eyebrow_txt),
                            )
                            .when_some(sync_label, |d, chip| {
                                d.child(
                                    div()
                                        .px(px(6.))
                                        .h(px(18.))
                                        .rounded(px(5.))
                                        .bg(p_copy.accent_soft)
                                        .flex()
                                        .items_center()
                                        .text_size(px(11.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p_copy.blue)
                                        .child(chip),
                                )
                            }),
                    )
                    .when(!sync_tiles.is_empty(), |d| {
                        d.child(
                            div()
                                .mt(px(6.))
                                .text_size(px(12.))
                                .text_color(p_copy.muted)
                                .child(syncs_txt),
                        )
                        .child(
                            div()
                                .mt(px(8.))
                                .flex()
                                .flex_wrap()
                                .items_center()
                                .gap(px(6.))
                                .children(sync_tiles),
                        )
                    })
                    .child(div().mt(px(16.)).flex().flex_col().gap(px(8.)).children(
                        features.into_iter().map(|feat| {
                            let inc = feat.included;
                            div()
                                .flex()
                                .items_start()
                                .gap(px(8.))
                                .text_size(px(12.5))
                                .line_height(px(18.))
                                .text_color(if inc { p_copy.ink } else { p_copy.muted })
                                .child(div().mt(px(3.)).flex_none().child(pro_svg(
                                    "0 0 16 16",
                                    if inc { SVG_CHECK_SM } else { SVG_DASH_SM },
                                    12.,
                                    if inc { p_copy.text } else { p_copy.faint },
                                )))
                                .child(div().flex_1().min_w_0().child(feat.label))
                        }),
                    ))
                    .into_any_element()
            },
        );

        div()
            .p(px(20.))
            .rounded(px(12.))
            .bg(p.card)
            .border_1()
            .border_color(border_col)
            .when(!p.dark, |d| d.shadow_xs())
            .flex()
            .flex_col()
            .min_w_0()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .text_size(px(13.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.text)
                            .child(plan.name.clone()),
                    )
                    .when_some(plan.badge.clone(), |d, b| {
                        d.child(
                            div()
                                .flex_none()
                                .h(px(19.))
                                .px(px(6.5))
                                .rounded(px(5.))
                                .bg(p.accent_soft)
                                .flex()
                                .items_center()
                                .text_size(px(11.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.blue)
                                .child(b),
                        )
                    }),
            )
            .child(price_row)
            .child(
                div()
                    .mt(px(2.))
                    .text_size(px(12.))
                    .text_color(p.muted)
                    .child(billed_caption),
            )
            .child(
                div().mt(px(16.)).w_full().flex().child(
                    pro_btn(
                        pro_eid(&self.id, format!("cta-{}", plan.id)),
                        cta_lbl,
                        if is_selected {
                            Some(ProIcon::Check)
                        } else {
                            None
                        },
                        cta_style,
                        false,
                        cx,
                        move |this, _, cx| {
                            this.select_plan(pid_cta.clone(), cx);
                        },
                    )
                    .w_full(),
                ),
            )
            .child(limits_list)
            .child(toggle_btn)
            .child(fold_el)
    }

    fn render_comparison_section(
        &self,
        narrow: bool,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> Div {
        let open = self.comparison_open;
        let btn_lbl = if open {
            tr(cx, "Hide feature comparison", "收起功能对比表")
        } else {
            tr(cx, "Compare all features", "展开完整功能对比表")
        };

        let trigger = activate(
            div()
                .id(pro_eid(&self.id, "compare-matrix-toggle"))
                .role(Role::Button)
                .aria_label(btn_lbl)
                .aria_expanded(open)
                .tab_index(0)
                .mx_auto()
                .h(px(32.))
                .px(px(14.))
                .rounded_full()
                .bg(p.card)
                .border_1()
                .border_color(p.line_strong)
                .hover(move |s| s.bg(p.chip_hover).text_color(p.strong))
                .focus_visible(move |s| s.border_1().border_color(p.focus))
                .flex()
                .items_center()
                .gap(px(6.))
                .text_size(px(12.5))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.ink)
                .cursor_pointer()
                .child(btn_lbl)
                .child(pro_icon(
                    if open {
                        ProIcon::ChevronUp
                    } else {
                        ProIcon::ChevronDown
                    },
                    13.,
                    p.muted,
                )),
            cx,
            |this, _, cx| {
                this.toggle_comparison(cx);
            },
        );

        let plan_headers: Vec<(SharedString, bool)> = self
            .plans
            .iter()
            .take(4)
            .map(|pl| (pl.name.clone(), pl.highlighted))
            .collect();
        let rows = self.comparison_rows.clone();
        let p_copy = *p;
        let feat_col_lbl: SharedString = tr(cx, "Feature", "功能项").into();

        let base_id = self.id.clone();
        let table_fold = pro_fold(
            pro_eid(&self.id, "compare-matrix-fold"),
            open,
            340.,
            0.,
            move || {
                let mut table = div()
                    .id(pro_eid(&base_id, "compare-table"))
                    .mt(px(14.))
                    .role(Role::Table)
                    .rounded(px(12.))
                    .bg(p_copy.card)
                    .border_1()
                    .border_color(p_copy.line)
                    .overflow_hidden()
                    .flex()
                    .flex_col();

                // Header row
                let head = div()
                    .id(pro_eid(&base_id, "compare-head"))
                    .role(Role::Row)
                    .px(px(if narrow { 10. } else { 16. }))
                    .py(px(10.))
                    .bg(p_copy.subtle)
                    .border_b_1()
                    .border_color(p_copy.line)
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p_copy.muted)
                            .child(feat_col_lbl),
                    )
                    .children(plan_headers.iter().map(|(name, hi)| {
                        div()
                            .w(px(if narrow { 62. } else { 128. }))
                            .flex_none()
                            .text_center()
                            .truncate()
                            .text_size(px(12.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(if *hi { p_copy.blue } else { p_copy.strong })
                            .child(name.clone())
                    }));
                table = table.child(head);

                let mut last_cat = SharedString::default();
                for (ri, row) in rows.into_iter().enumerate() {
                    if row.category != last_cat {
                        last_cat = row.category.clone();
                        table = table.child(
                            div()
                                .px(px(if narrow { 10. } else { 16. }))
                                .py(px(7.))
                                .when(ri > 0, |d| d.border_t_1().border_color(p_copy.line))
                                .bg(p_copy.chip.opacity(0.55))
                                .text_size(px(11.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(p_copy.muted)
                                .child(row.category),
                        );
                    }
                    let line = div()
                        .id(pro_eid(&base_id, format!("compare-row-{ri}")))
                        .role(Role::Row)
                        .px(px(if narrow { 10. } else { 16. }))
                        .py(px(9.))
                        .border_t_1()
                        .border_color(p_copy.line_soft)
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .text_size(px(12.5))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .text_color(p_copy.ink)
                                .child(row.feature),
                        )
                        .children((0..plan_headers.len()).map(|ci| {
                            let cell = row.values.get(ci).map(SharedString::as_ref).unwrap_or("no");
                            let content = match cell {
                                "yes" => div()
                                    .flex()
                                    .justify_center()
                                    .child(pro_svg("0 0 16 16", SVG_CHECK_SM, 13., p_copy.blue))
                                    .into_any_element(),
                                "no" => div()
                                    .flex()
                                    .justify_center()
                                    .child(pro_svg("0 0 16 16", SVG_DASH_SM, 12., p_copy.faint))
                                    .into_any_element(),
                                txt => div()
                                    .text_center()
                                    .truncate()
                                    .text_size(px(if narrow { 11. } else { 12. }))
                                    .text_color(p_copy.text)
                                    .child(txt.to_owned())
                                    .into_any_element(),
                            };
                            div()
                                .w(px(if narrow { 62. } else { 128. }))
                                .flex_none()
                                .child(content)
                        }));
                    table = table.child(line);
                }

                table.into_any_element()
            },
        );

        div()
            .mt(px(24.))
            .w_full()
            .flex()
            .flex_col()
            .child(trigger)
            .child(table_fold)
    }

    fn render_skeleton(&self, narrow: bool, phase: f32, cx: &App) -> Div {
        let p = ProPalette::get(cx);
        let cards = (0..3).map(|i| {
            div()
                .flex_1()
                .p(px(20.))
                .rounded(px(12.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .flex()
                .flex_col()
                .gap(px(12.))
                .child(pro_skeleton_bar(72., 13., 4., phase + i as f32 * 0.12, cx))
                .child(pro_skeleton_bar(110., 26., 6., phase + i as f32 * 0.15, cx))
                .child(pro_skeleton_bar(180., 32., 8., phase + i as f32 * 0.18, cx))
                .child(div().h(px(1.)).bg(p.line))
                .child(pro_skeleton_bar(160., 12., 4., phase + i as f32 * 0.2, cx))
                .child(pro_skeleton_bar(140., 12., 4., phase + i as f32 * 0.22, cx))
        });

        div()
            .w_full()
            .flex()
            .flex_col()
            .items_center()
            .gap(px(16.))
            .child(pro_skeleton_bar(210., 24., 6., phase, cx))
            .child(pro_skeleton_bar(320., 13., 4., phase + 0.1, cx))
            .child(pro_skeleton_bar(168., 34., 999., phase + 0.15, cx))
            .child(
                div()
                    .mt(px(12.))
                    .w_full()
                    .flex()
                    .when(narrow, |d| d.flex_col())
                    .gap(px(12.))
                    .children(cards),
            )
    }
}

impl Render for PricingPlansBlock {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let reduced = Theme::global(cx).reduced_motion;
        if self.loading && !reduced {
            w.request_animation_frame();
        }
        if self
            .toast
            .as_ref()
            .is_some_and(|(_, t)| t.elapsed().as_secs_f32() >= 2.45)
        {
            self.toast = None;
        }

        let width = self.measured_width.get();
        let narrow = width < 760.0;
        let measure_cell = self.measured_width.clone();

        let body = if self.loading {
            let phase = (self.boot_at.elapsed().as_secs_f32() / 1.4).fract();
            self.render_skeleton(narrow, phase, cx).into_any_element()
        } else if self.empty || self.plans.is_empty() {
            pro_empty_state(
                ProIcon::CreditCard,
                tr(cx, "No pricing plans configured", "暂无可用订阅方案"),
                tr(
                    cx,
                    "Add up to three or four tiers to compare quotas, integrations, and annual savings.",
                    "添加订阅档位即可展示月付/年付切换、配额对比与功能明细。",
                ),
                Some((
                    pro_eid(&self.id, "restore-plans"),
                    tr(cx, "Load default plans", "载入默认方案").into(),
                    Box::new(|this: &mut Self, _, cx| {
                        this.empty = false;
                        if this.plans.is_empty() {
                            this.plans = default_pricing_plans();
                        }
                        cx.notify();
                    }),
                )),
                cx,
            )
            .into_any_element()
        } else {
            let heading = self
                .title
                .clone()
                .unwrap_or_else(|| tr(cx, "Choose your plan", "选择适合团队的方案").into());
            let sub = self.subtitle.clone().unwrap_or_else(|| {
                tr(
                    cx,
                    "Find the right plan for your team. Start with a 14-day free trial.",
                    "为团队挑选合适的配额与集成方案，立即开启 14 天免费试用。",
                )
                .into()
            });
            let prefix = resolve_currency_prefix(&self.currency);
            let plans_snapshot = self.plans.clone();
            let count = plans_snapshot.len();

            let grid = if narrow {
                div()
                    .mt(px(26.))
                    .w_full()
                    .max_w(px(420.))
                    .mx_auto()
                    .flex()
                    .flex_col()
                    .gap(px(12.))
                    .children(
                        plans_snapshot
                            .iter()
                            .map(|pl| self.render_plan_card(pl, &prefix, &p, cx)),
                    )
            } else if count == 4 {
                let first_pair: Vec<_> = plans_snapshot
                    .iter()
                    .take(2)
                    .map(|pl| {
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(self.render_plan_card(pl, &prefix, &p, cx))
                    })
                    .collect();
                let second_pair: Vec<_> = plans_snapshot
                    .iter()
                    .skip(2)
                    .take(2)
                    .map(|pl| {
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(self.render_plan_card(pl, &prefix, &p, cx))
                    })
                    .collect();
                div()
                    .mt(px(26.))
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap(px(12.))
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .items_start()
                            .gap(px(12.))
                            .children(first_pair),
                    )
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .items_start()
                            .gap(px(12.))
                            .children(second_pair),
                    )
            } else {
                div()
                    .mt(px(26.))
                    .w_full()
                    .flex()
                    .items_start()
                    .gap(px(12.))
                    .children(plans_snapshot.iter().map(|pl| {
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(self.render_plan_card(pl, &prefix, &p, cx))
                    }))
            };

            let cmp_section = self.render_comparison_section(narrow, &p, cx);

            div()
                .w_full()
                .max_w(px(820.))
                .mx_auto()
                .flex()
                .flex_col()
                .items_center()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .text_center()
                        .child(
                            div()
                                .text_size(px(24.))
                                .line_height(px(32.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.strong)
                                .child(heading),
                        )
                        .child(
                            div()
                                .mt(px(4.))
                                .text_size(px(13.))
                                .line_height(px(20.))
                                .text_color(p.text)
                                .child(sub),
                        ),
                )
                .child(self.render_billing_capsule(&p, cx))
                .child(grid)
                .child(cmp_section)
                .into_any_element()
        };

        let state_key: SharedString = if self.loading {
            "loading".into()
        } else if self.empty || self.plans.is_empty() {
            "empty".into()
        } else {
            "ready".into()
        };

        div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::handle_key_down))
            .relative()
            .w_full()
            .min_w_0()
            .bg(p.page)
            .px(px(if narrow { 16. } else { 36. }))
            .py(px(if narrow { 24. } else { 36. }))
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
            .child(pro_view_transition(
                pro_eid(&self.id, "plans-state-vtx"),
                state_key,
                body,
            ))
            .child(pro_toast_banner(self.toast.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        PricingBillingPeriod, PricingPlansBlock, default_pricing_plans, price_decimals,
        resolve_currency_prefix,
    };

    #[test]
    fn default_plans_carry_twenty_percent_annual_discount_and_popular_growth() {
        let plans = default_pricing_plans();
        assert_eq!(plans.len(), 3);
        assert_eq!(plans[0].name.as_ref(), "Starter");
        assert_eq!(
            plans[0].price_for(PricingBillingPeriod::Monthly),
            Some(30.0)
        );
        assert_eq!(plans[0].price_for(PricingBillingPeriod::Yearly), Some(24.0));

        assert_eq!(plans[1].name.as_ref(), "Growth");
        assert!(plans[1].highlighted);
        assert_eq!(plans[1].badge.as_deref(), Some("Popular"));
        assert_eq!(
            plans[1].price_for(PricingBillingPeriod::Monthly),
            Some(99.0)
        );
        assert_eq!(plans[1].price_for(PricingBillingPeriod::Yearly), Some(79.0));

        assert_eq!(plans[2].name.as_ref(), "Scale");
        assert_eq!(
            plans[2].price_for(PricingBillingPeriod::Monthly),
            Some(249.0)
        );
        assert_eq!(
            plans[2].price_for(PricingBillingPeriod::Yearly),
            Some(199.0)
        );
    }

    #[test]
    fn currency_codes_and_fractional_decimals_resolve_cleanly() {
        assert_eq!(resolve_currency_prefix("$").as_ref(), "$");
        assert_eq!(resolve_currency_prefix("EUR").as_ref(), "€");
        assert_eq!(resolve_currency_prefix("gbp").as_ref(), "£");
        assert_eq!(price_decimals(79.0), 0);
        assert_eq!(price_decimals(19.50), 2);
    }

    #[cfg(feature = "test-support")]
    #[test]
    fn pricing_plans_block_switches_billing_and_toggles_disclosures() {
        let mut app = gpui::TestAppContext::single();
        app.update(crate::init);
        let (block, w) = app.add_window_view(|_, cx| PricingPlansBlock::new("test-pricing", cx));
        w.update(|_, cx| {
            block.update(cx, |b, cx| {
                assert_eq!(b.billing(), PricingBillingPeriod::Yearly);
                assert!(b.is_details_open());
                assert!(!b.is_comparison_open());

                b.set_billing(PricingBillingPeriod::Monthly, cx);
                assert_eq!(b.billing(), PricingBillingPeriod::Monthly);

                b.toggle_details(cx);
                assert!(!b.is_details_open());

                b.toggle_comparison(cx);
                assert!(b.is_comparison_open());
            });
        });
    }
}
