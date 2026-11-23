//! Workspace subscription billing, capsule period toggle, plan tiers, usage meters, and invoices.
//!
//! Renders inside [`AppShellPreset::Settings`] with current plan overview, upcoming invoice
//! breakdown, four quota meters, a capsule `Monthly` / `Annual` (`-20%`) selector, plan and
//! workflow add-on cards, editable billing address, `Visa •••• 4242` card details, and invoice history.

use crate::{
    components::{
        app_shell::{AppShell, AppShellEvent, AppShellPreset},
        pro_chrome::{
            ProBtnStyle, ProPalette, ProTone, pro_badge, pro_btn, pro_eid, pro_empty_state,
            pro_fold, pro_icon_btn, pro_mark_selected, pro_pill_indicator, pro_selection_sensor,
            pro_skeleton_bar, pro_spring_bar, pro_toast_banner, pro_view_transition, tr,
        },
    },
    foundation::Theme,
    primitives::{
        control::activate,
        editor::{EditEvent, Editor, InputMode},
        pro_icons::{ProIcon, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

const SVG_INFO_CIRCLE: &str = r#"<circle cx="12" cy="12" r="10" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></circle><path d="M12 16V12" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M12.125 8.25H12M12.25 8.25C12.25 8.11193 12.1381 8 12 8C11.8619 8 11.75 8.11193 11.75 8.25C11.75 8.38807 11.8619 8.5 12 8.5C12.1381 8.5 12.25 8.38807 12.25 8.25Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const SVG_STORAGE_RACK: &str = r#"<path d="M3 12C3 7.75736 3 5.63604 4.31802 4.31802C5.63604 3 7.75736 3 12 3C16.2426 3 18.364 3 19.682 4.31802C21 5.63604 21 7.75736 21 12C21 16.2426 21 18.364 19.682 19.682C18.364 21 16.2426 21 12 21C7.75736 21 5.63604 21 4.31802 19.682C3 18.364 3 16.2426 3 12Z" stroke="currentColor" stroke-width="1.9"></path><path d="M3 12H21" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M11 7.5L17 7.5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M7.125 7.5H7" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2.2"></path><path d="M11 16.5L17 16.5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M7.125 16.5H7" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2.2"></path>"#;

const SVG_AGENT_BRAIN: &str = r#"<path d="M7 4.5C5.34315 4.5 4 5.84315 4 7.5C4 8.06866 4.15822 8.60037 4.43304 9.0535C3.04727 9.31855 2 10.537 2 12C2 13.463 3.04727 14.6814 4.43304 14.9465M7 4.5C7 3.11929 8.11929 2 9.5 2C10.8807 2 12 3.11929 12 4.5V19.5C12 20.8807 10.8807 22 9.5 22C8.11929 22 7 20.8807 7 19.5C5.34315 19.5 4 18.1569 4 16.5C4 15.9313 4.15822 15.3996 4.43304 14.9465" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M17 19.4999C18.6569 19.4999 20 18.1567 20 16.4999C20 15.9312 19.8418 15.3995 19.567 14.9464C20.9527 14.6813 22 13.4629 22 11.9999C22 10.5369 20.9527 9.31843 19.567 9.05338M17 19.4999C17 20.8806 15.8807 21.9999 14.5 21.9999C13.1193 21.9999 12 20.8806 12 19.4999L12 4.49988C12 3.11917 13.1193 1.99988 14.5 1.99988C15.8807 1.99988 17 3.11917 17 4.49988C18.6569 4.49988 20 5.84302 20 7.49988C20 8.06854 19.8418 8.60024 19.567 9.05338" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const SVG_WORKFLOW_NODES: &str = r#"<path d="M21 6.5C21 8.433 19.433 10 17.5 10C15.567 10 14 8.433 14 6.5C14 4.567 15.567 3 17.5 3C19.433 3 21 4.567 21 6.5Z" stroke="currentColor" stroke-width="1.9"></path><path d="M3 6.5C3 4.85008 3 4.02513 3.51256 3.51256C4.02513 3 4.85008 3 6.5 3C8.14992 3 8.97487 3 9.48744 3.51256C10 4.02513 10 4.85008 10 6.5C10 8.14992 10 8.97487 9.48744 9.48744C8.97487 10 8.14992 10 6.5 10C4.85008 10 4.02513 10 3.51256 9.48744C3 8.97487 3 8.14992 3 6.5Z" stroke="currentColor" stroke-width="1.9"></path><path d="M3 17.5C3 15.8501 3 15.0251 3.51256 14.5126C4.02513 14 4.85008 14 6.5 14C8.14992 14 8.97487 14 9.48744 14.5126C10 15.0251 10 15.8501 10 17.5C10 19.1499 10 19.9749 9.48744 20.4874C8.97487 21 8.14992 21 6.5 21C4.85008 21 4.02513 21 3.51256 20.4874C3 19.9749 3 19.1499 3 17.5Z" stroke="currentColor" stroke-width="1.9"></path><path d="M14 17.5C14 15.8501 14 15.0251 14.5126 14.5126C15.0251 14 15.8501 14 17.5 14C19.1499 14 19.9749 14 20.4874 14.5126C21 15.0251 21 15.8501 21 17.5C21 19.1499 21 19.9749 20.4874 20.4874C19.9749 21 19.1499 21 17.5 21C15.8501 21 15.0251 21 14.5126 20.4874C14 19.9749 14 19.1499 14 17.5Z" stroke="currentColor" stroke-width="1.9"></path><path d="M17.5 10V14M14 17.5H10M10 6.5H14" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

/// Billing interval selected on the capsule period toggle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BillingInterval {
    /// Month-to-month billing.
    #[default]
    Monthly,
    /// Yearly billing with a 20% discount per seat and add-on.
    Annual,
}

/// Workspace subscription plan tier.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SubscriptionPlan {
    /// Entry-level free/starter workspace tier.
    Starter,
    /// Individual and small-team AI workspace (`$49/mo`).
    #[default]
    Pro,
    /// Shared agents, SSO/SCIM, and team workflows (`$69/mo`).
    Team,
    /// Unlimited meetings, app sharing, and governance (`$89/mo`).
    Business,
    /// Custom volume and dedicated deployment tier.
    Enterprise,
}

impl SubscriptionPlan {
    /// Display name of the plan tier.
    pub fn name(self, cx: &App) -> &'static str {
        match self {
            Self::Starter => tr(cx, "Starter", "入门版"),
            Self::Pro => tr(cx, "Pro", "专业版"),
            Self::Team => tr(cx, "Team", "团队版"),
            Self::Business => tr(cx, "Business", "商业版"),
            Self::Enterprise => tr(cx, "Enterprise", "企业版"),
        }
    }

    /// Per-member monthly rate in USD for the given [`BillingInterval`].
    pub fn price_per_member(self, interval: BillingInterval) -> u32 {
        let monthly = match self {
            Self::Starter => 0,
            Self::Pro => 49,
            Self::Team => 69,
            Self::Business => 89,
            Self::Enterprise => 119,
        };
        match interval {
            BillingInterval::Monthly => monthly,
            BillingInterval::Annual => ((monthly as f32) * 0.8).round() as u32,
        }
    }
}

/// Workflows capacity add-on tier.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WorkflowAddonTier {
    /// Included 2,500 workflow runs / month.
    #[default]
    StarterIncluded,
    /// 40,000 workflow runs / month (`$539/mo` monthly).
    Runs40k,
    /// 100,000 workflow runs / month (`$1,190/mo` monthly).
    Runs100k,
    /// 250,000 workflow runs / month (`$2,490/mo` monthly).
    Runs250k,
}

impl WorkflowAddonTier {
    /// Selectable paid Business workflow run tiers.
    pub const PAID_TIERS: [Self; 3] = [Self::Runs40k, Self::Runs100k, Self::Runs250k];

    fn picker_label(self) -> &'static str {
        match self {
            Self::StarterIncluded => "2.5k workflow runs / month",
            Self::Runs40k => "40k workflow runs / month",
            Self::Runs100k => "100k workflow runs / month",
            Self::Runs250k => "250k workflow runs / month",
        }
    }

    fn bullet_runs(self) -> &'static str {
        match self {
            Self::StarterIncluded => "2,500 runs / month",
            Self::Runs40k => "40,000 runs / month",
            Self::Runs100k => "100,000 runs / month",
            Self::Runs250k => "250,000 runs / month",
        }
    }

    /// Monthly equivalent workspace price in USD for the given [`BillingInterval`].
    pub fn monthly_price(self, interval: BillingInterval) -> u32 {
        let base = match self {
            Self::StarterIncluded => 0,
            Self::Runs40k => 539,
            Self::Runs100k => 1190,
            Self::Runs250k => 2490,
        };
        match interval {
            BillingInterval::Monthly => base,
            BillingInterval::Annual => ((base as f32) * 0.8).round() as u32,
        }
    }
}

/// Workspace quota metrics displayed in the four-column usage meter strip.
#[derive(Clone, Debug, PartialEq)]
pub struct SubscriptionUsage {
    pub seats_used: u32,
    pub seats_limit: u32,
    pub credits_used: u32,
    pub credits_limit: u32,
    pub storage_used_gb: f32,
    pub storage_limit_gb: f32,
    pub runs_used: u32,
    pub runs_limit: u32,
}

impl Default for SubscriptionUsage {
    fn default() -> Self {
        Self {
            seats_used: 6,
            seats_limit: 10,
            credits_used: 34_000,
            credits_limit: 50_000,
            storage_used_gb: 42.6,
            storage_limit_gb: 100.0,
            runs_used: 2_310,
            runs_limit: 2_500,
        }
    }
}

/// Billing address and tax details printed on workspace invoices.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BillingAddress {
    pub email: SharedString,
    pub company: SharedString,
    pub address: SharedString,
    pub vat_number: Option<SharedString>,
}

impl Default for BillingAddress {
    fn default() -> Self {
        Self {
            email: "billing@acme.com".into(),
            company: "Acme Inc.".into(),
            address: "Mariahilfer Straße 1, 1060 Vienna, Austria".into(),
            vat_number: None,
        }
    }
}

/// Stored payment card shown in `Billing details`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaymentCard {
    pub brand: SharedString,
    pub last4: SharedString,
    pub expires: SharedString,
    pub is_default: bool,
}

impl Default for PaymentCard {
    fn default() -> Self {
        Self {
            brand: "Visa".into(),
            last4: "4242".into(),
            expires: "08/2028".into(),
            is_default: true,
        }
    }
}

/// Status of a billing history row.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InvoiceStatus {
    /// Scheduled renewal invoice.
    Upcoming,
    /// Settled past invoice.
    #[default]
    Paid,
}

/// One row in the `History` invoices table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BillingInvoice {
    pub reference: SharedString,
    pub total_usd: u32,
    pub date: SharedString,
    pub status: InvoiceStatus,
}

impl BillingInvoice {
    /// Construct an invoice record.
    pub fn new(
        reference: impl Into<SharedString>,
        total_usd: u32,
        date: impl Into<SharedString>,
        status: InvoiceStatus,
    ) -> Self {
        Self {
            reference: reference.into(),
            total_usd,
            date: date.into(),
            status,
        }
    }
}

/// Sample invoice history matching the reference `Settings / Billing` DOM.
pub fn default_billing_invoices() -> Vec<BillingInvoice> {
    vec![
        BillingInvoice::new("INV-2026-0011", 294, "Nov 6, 2026", InvoiceStatus::Paid),
        BillingInvoice::new("INV-2026-0010", 294, "Oct 6, 2026", InvoiceStatus::Paid),
        BillingInvoice::new("INV-2026-0009", 245, "Sep 6, 2026", InvoiceStatus::Paid),
        BillingInvoice::new("INV-2026-0008", 245, "Aug 6, 2026", InvoiceStatus::Paid),
    ]
}

/// Compute upcoming renewal bill total in USD from seats, active plan, interval, and workflow add-on.
pub fn compute_upcoming_bill(
    seats: u32,
    plan: SubscriptionPlan,
    interval: BillingInterval,
    addon: WorkflowAddonTier,
) -> u32 {
    let seat_monthly = seats * plan.price_per_member(interval);
    let addon_monthly = addon.monthly_price(interval);
    let monthly_sum = seat_monthly + addon_monthly;
    match interval {
        BillingInterval::Monthly => monthly_sum,
        BillingInterval::Annual => monthly_sum * 12,
    }
}

fn format_thousands(n: u32) -> String {
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + 3);
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out.chars().rev().collect()
}

/// Events emitted by [`SettingsSubscriptionScreen`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SettingsSubscriptionEvent {
    /// Sidebar or cross-link navigation (`"Members"`, `"Models"`, `"Back to app"`, etc.).
    Navigate(SharedString),
    /// Billing interval switched between `Monthly` and `Annual`.
    IntervalChanged(BillingInterval),
    /// Workspace subscription plan changed.
    PlanChanged {
        plan: SubscriptionPlan,
        interval: BillingInterval,
    },
    /// Workflows add-on tier updated.
    AddonChanged(WorkflowAddonTier),
    /// Billing address updated.
    AddressUpdated(BillingAddress),
    /// Payment card updated.
    PaymentMethodUpdated(PaymentCard),
    /// Invoice downloaded or previewed.
    InvoiceAction(SharedString),
    /// Subscription canceled (`true`) or resumed (`false`).
    SubscriptionCanceled(bool),
}

impl EventEmitter<SettingsSubscriptionEvent> for SettingsSubscriptionScreen {}

/// KUI Pro `Settings / Billing` screen (`settings-subscription`).
pub struct SettingsSubscriptionScreen {
    id: ElementId,
    shell: Entity<AppShell>,
    focus: FocusHandle,
    standalone: bool,
    loading: bool,
    empty: bool,
    plan: SubscriptionPlan,
    interval: BillingInterval,
    addon_active: bool,
    addon_choice: WorkflowAddonTier,
    addon_menu_open: bool,
    usage: SubscriptionUsage,
    address: BillingAddress,
    card: PaymentCard,
    invoices: Vec<BillingInvoice>,
    bill_details_open: bool,
    compare_open: bool,
    editing_address: bool,
    adding_card: bool,
    cancel_confirm_open: bool,
    canceled: bool,
    toast: Option<(SharedString, Instant)>,
    addr_email_ed: Option<Entity<Editor>>,
    addr_company_ed: Option<Entity<Editor>>,
    addr_line_ed: Option<Entity<Editor>>,
    addr_vat_ed: Option<Entity<Editor>>,
    card_last4_ed: Option<Entity<Editor>>,
    card_exp_ed: Option<Entity<Editor>>,
    measured_width: Rc<Cell<f32>>,
    mounted_at: Instant,
}

impl SettingsSubscriptionScreen {
    /// Construct a `Settings / Billing` screen hosted inside [`AppShellPreset::Settings`].
    pub fn new(id: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let id_str: SharedString = id.into();
        let this = cx.entity();
        let shell = cx.new(|shell_cx| {
            let mut s = AppShell::new(format!("{id_str}-shell"), shell_cx)
                .with_preset(AppShellPreset::Settings, shell_cx)
                .with_active("Billing")
                .with_work_area(false, false);
            s.bind_page(&this, shell_cx, |screen, w, cx| {
                screen.render_page(w, cx).into_any_element()
            });
            s
        });
        cx.subscribe(&shell, |_, _, ev: &AppShellEvent, cx| match ev {
            AppShellEvent::Navigate(label) => {
                cx.emit(SettingsSubscriptionEvent::Navigate(label.clone()));
            }
            AppShellEvent::Back => {
                cx.emit(SettingsSubscriptionEvent::Navigate("Back to app".into()));
            }
            _ => {}
        })
        .detach();

        Self {
            id: ElementId::Name(id_str),
            shell,
            focus: cx.focus_handle(),
            standalone: false,
            loading: false,
            empty: false,
            plan: SubscriptionPlan::Pro,
            interval: BillingInterval::Monthly,
            addon_active: false,
            addon_choice: WorkflowAddonTier::Runs40k,
            addon_menu_open: false,
            usage: SubscriptionUsage::default(),
            address: BillingAddress::default(),
            card: PaymentCard::default(),
            invoices: default_billing_invoices(),
            bill_details_open: false,
            compare_open: false,
            editing_address: false,
            adding_card: false,
            cancel_confirm_open: false,
            canceled: false,
            toast: None,
            addr_email_ed: None,
            addr_company_ed: None,
            addr_line_ed: None,
            addr_vat_ed: None,
            card_last4_ed: None,
            card_exp_ed: None,
            measured_width: Rc::new(Cell::new(960.)),
            mounted_at: Instant::now(),
        }
    }

    /// Render only the inner billing page without the outer [`AppShell`] frame.
    pub fn standalone(mut self) -> Self {
        self.standalone = true;
        self
    }

    /// Override the workspace name displayed in the settings sidebar.
    pub fn with_workspace(
        self,
        workspace: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> Self {
        let ws = workspace.into();
        self.shell.update(cx, |s, cx| {
            *s = std::mem::replace(s, AppShell::new("tmp", cx)).with_workspace(ws);
        });
        self
    }

    /// Override the signed-in account footer in the settings sidebar.
    pub fn with_user(
        self,
        name: impl Into<SharedString>,
        email: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> Self {
        let (n, e) = (name.into(), email.into());
        self.shell.update(cx, |s, cx| {
            *s = std::mem::replace(s, AppShell::new("tmp", cx)).with_user(n, e);
        });
        self
    }

    /// Set the active workspace plan tier.
    pub fn with_plan(mut self, plan: SubscriptionPlan) -> Self {
        self.plan = plan;
        self
    }

    /// Set the initial billing interval (`Monthly` or `Annual`).
    pub fn with_interval(mut self, interval: BillingInterval) -> Self {
        self.interval = interval;
        self
    }

    /// Override quota utilization metrics.
    pub fn with_usage(mut self, usage: SubscriptionUsage) -> Self {
        self.usage = usage;
        self
    }

    /// Override invoice history rows.
    pub fn with_invoices(mut self, invoices: Vec<BillingInvoice>) -> Self {
        self.invoices = invoices;
        self
    }

    /// Toggle skeleton loading state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Mutate skeleton loading state at runtime.
    pub fn set_loading(&mut self, loading: bool, cx: &mut Context<Self>) {
        self.loading = loading;
        cx.notify();
    }

    /// Toggle empty invoice history state (`No invoices yet` + `Preview first bill`).
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Mutate empty invoice history state at runtime.
    pub fn set_empty(&mut self, empty: bool, cx: &mut Context<Self>) {
        self.empty = empty;
        cx.notify();
    }

    fn show_toast(&mut self, msg: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.toast = Some((msg.into(), Instant::now()));
        cx.notify();
    }

    fn active_addon(&self) -> WorkflowAddonTier {
        if self.addon_active {
            self.addon_choice
        } else {
            WorkflowAddonTier::StarterIncluded
        }
    }

    fn ensure_editors(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.addr_email_ed.is_none() {
            let init = self.address.email.to_string();
            let ed = cx.new(|cx| {
                let mut e = Editor::new("Billing email", InputMode::Single, w, cx);
                e.set_placeholder("billing@acme.com");
                e.set_value(init, w, cx);
                e.font_size = 13.;
                e
            });
            cx.subscribe(&ed, |_, _, _: &EditEvent, cx| cx.notify())
                .detach();
            self.addr_email_ed = Some(ed);
        }

        if self.addr_company_ed.is_none() {
            let init = self.address.company.to_string();
            let ed = cx.new(|cx| {
                let mut e = Editor::new("Company", InputMode::Single, w, cx);
                e.set_placeholder("Acme Inc.");
                e.set_value(init, w, cx);
                e.font_size = 13.;
                e
            });
            cx.subscribe(&ed, |_, _, _: &EditEvent, cx| cx.notify())
                .detach();
            self.addr_company_ed = Some(ed);
        }

        if self.addr_line_ed.is_none() {
            let init = self.address.address.to_string();
            let ed = cx.new(|cx| {
                let mut e = Editor::new("Street address", InputMode::Single, w, cx);
                e.set_placeholder("Street, postal code, city, country");
                e.set_value(init, w, cx);
                e.font_size = 13.;
                e
            });
            cx.subscribe(&ed, |_, _, _: &EditEvent, cx| cx.notify())
                .detach();
            self.addr_line_ed = Some(ed);
        }

        if self.addr_vat_ed.is_none() {
            let init = self
                .address
                .vat_number
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default();
            let ed = cx.new(|cx| {
                let mut e = Editor::new("VAT number", InputMode::Single, w, cx);
                e.set_placeholder("ATU12345678 (optional)");
                e.set_value(init, w, cx);
                e.font_size = 13.;
                e
            });
            cx.subscribe(&ed, |_, _, _: &EditEvent, cx| cx.notify())
                .detach();
            self.addr_vat_ed = Some(ed);
        }

        if self.card_last4_ed.is_none() {
            let ed = cx.new(|cx| {
                let mut e = Editor::new("Card last 4 digits", InputMode::Single, w, cx);
                e.set_placeholder("Last 4 digits (e.g. 8819)");
                e.font_size = 13.;
                e
            });
            cx.subscribe(&ed, |_, _, _: &EditEvent, cx| cx.notify())
                .detach();
            self.card_last4_ed = Some(ed);
        }

        if self.card_exp_ed.is_none() {
            let ed = cx.new(|cx| {
                let mut e = Editor::new("Expiry MM/YYYY", InputMode::Single, w, cx);
                e.set_placeholder("MM/YYYY (e.g. 11/2029)");
                e.font_size = 13.;
                e
            });
            cx.subscribe(&ed, |_, _, _: &EditEvent, cx| cx.notify())
                .detach();
            self.card_exp_ed = Some(ed);
        }
    }

    fn handle_key(&mut self, ev: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        if ev.keystroke.key == "escape"
            && (self.addon_menu_open
                || self.compare_open
                || self.editing_address
                || self.adding_card
                || self.cancel_confirm_open)
        {
            self.addon_menu_open = false;
            self.compare_open = false;
            self.editing_address = false;
            self.adding_card = false;
            self.cancel_confirm_open = false;
            cx.notify();
        }
    }

    fn render_overview_section(&self, narrow: bool, cx: &Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let frame_tint: Hsla = if p.dark {
            hsla(0., 0., 1., 0.03)
        } else {
            hsla(0., 0., 0., 0.022)
        };
        let unit_price = self.plan.price_per_member(self.interval);
        let upcoming_total = compute_upcoming_bill(
            self.usage.seats_used,
            self.plan,
            self.interval,
            self.active_addon(),
        );
        let billed_cadence = match self.interval {
            BillingInterval::Monthly => tr(cx, "billed monthly", "按月计费"),
            BillingInterval::Annual => tr(cx, "billed annually (−20%)", "按年计费 (−20%)"),
        };

        let left_card = div()
            .flex_1()
            .p(px(20.))
            .bg(p.card)
            .when(narrow, |d| d.rounded_t(px(11.)))
            .when(!narrow, |d| d.rounded_l(px(11.)))
            .flex()
            .flex_col()
            .justify_between()
            .gap(px(20.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .child(
                        div()
                            .size(px(38.))
                            .flex_none()
                            .rounded(px(10.))
                            .bg(p.accent_soft)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(pro_icon(ProIcon::Sparkles, 18., p.blue)),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(2.))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.))
                                    .child(
                                        div()
                                            .text_size(px(15.))
                                            .line_height(px(22.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.strong)
                                            .child(format!(
                                                "{} {}",
                                                self.plan.name(cx),
                                                tr(cx, "plan", "方案")
                                            )),
                                    )
                                    .when(self.canceled, |d| {
                                        d.child(pro_badge(
                                            tr(cx, "Cancels Dec 6", "12月6日到期"),
                                            ProTone::Amber,
                                            false,
                                            cx,
                                        ))
                                    }),
                            )
                            .child(div().text_size(px(13.)).text_color(p.muted).child(format!(
                                "${unit_price} {}, {billed_cadence}",
                                tr(cx, "per member / month", "每位成员 / 月")
                            ))),
                    ),
            )
            .child(div().flex().items_center().child(pro_btn(
                pro_eid(&self.id, "btn-explore-plans"),
                tr(cx, "Explore plans", "查看所有方案"),
                None,
                ProBtnStyle::Primary,
                true,
                cx,
                |this, _, cx| {
                    this.compare_open = !this.compare_open;
                    cx.notify();
                },
            )));

        let mut right_card = div()
            .flex_1()
            .p(px(20.))
            .bg(p.card)
            .when(narrow, |d| {
                d.rounded_b(px(11.)).border_t_1().border_color(p.line)
            })
            .when(!narrow, |d| {
                d.rounded_r(px(11.)).border_l_1().border_color(p.line)
            })
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(
                        div()
                            .text_size(px(14.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .child(tr(cx, "Upcoming bill", "下期账单")),
                    )
                    .child(pro_svg("0 0 24 24", SVG_INFO_CIRCLE, 14., p.faint)),
            )
            .child(
                div()
                    .mt(px(2.))
                    .text_size(px(13.))
                    .text_color(p.muted)
                    .child(if self.canceled {
                        tr(
                            cx,
                            "No renewal — active through Dec 6, 2026",
                            "已取消自动续费，有效期至 2026 年 12 月 6 日",
                        )
                    } else {
                        tr(cx, "Renews on Dec 6, 2026", "将于 2026 年 12 月 6 日续费")
                    }),
            )
            .child(
                div()
                    .mt(px(18.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(activate(
                        div()
                            .id(pro_eid(&self.id, "btn-bill-details"))
                            .role(Role::Button)
                            .tab_index(0)
                            .h(px(26.))
                            .flex()
                            .items_center()
                            .gap(px(4.))
                            .text_size(px(13.))
                            .text_color(p.text)
                            .hover(move |s| s.text_color(p.ink))
                            .cursor_pointer()
                            .child(tr(cx, "Show details", "账单明细"))
                            .child(pro_icon(
                                if self.bill_details_open {
                                    ProIcon::ChevronUp
                                } else {
                                    ProIcon::ChevronDown
                                },
                                13.,
                                p.faint,
                            )),
                        cx,
                        |this, _, cx| {
                            this.bill_details_open = !this.bill_details_open;
                            cx.notify();
                        },
                    ))
                    .child(
                        div()
                            .text_size(px(22.))
                            .line_height(px(28.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .child(format!(
                                "${}",
                                format_thousands(if self.canceled { 0 } else { upcoming_total })
                            )),
                    ),
            );

        let seat_subtotal = self.usage.seats_used * unit_price;
        let addon_cost = self.active_addon().monthly_price(self.interval);
        let bill_details_el = div()
            .pt(px(12.))
            .child(
                div()
                    .pt(px(12.))
                    .border_t_1()
                    .border_color(p.line)
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .text_size(px(12.5))
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .child(div().text_color(p.muted).child(format!(
                                "{} × {} (${unit_price}/mo)",
                                self.usage.seats_used,
                                self.plan.name(cx)
                            )))
                            .child(
                                div()
                                    .text_color(p.ink)
                                    .child(format!("${}", format_thousands(seat_subtotal))),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .child(div().text_color(p.muted).child(tr(
                                cx,
                                "Workflows add-on",
                                "工作流附加包",
                            )))
                            .child(
                                div()
                                    .text_color(p.ink)
                                    .child(format!("${}", format_thousands(addon_cost))),
                            ),
                    ),
            )
            .into_any_element();
        right_card = right_card.child(pro_fold(
            pro_eid(&self.id, "bill-details-fold"),
            self.bill_details_open,
            66.,
            0.,
            move || bill_details_el,
        ));

        // 4 Usage meters
        let u = &self.usage;
        let seats_ratio = (u.seats_used as f32 / u.seats_limit.max(1) as f32).clamp(0., 1.);
        let credits_ratio = (u.credits_used as f32 / u.credits_limit.max(1) as f32).clamp(0., 1.);
        let storage_ratio = (u.storage_used_gb / u.storage_limit_gb.max(1.)).clamp(0., 1.);
        let runs_ratio = (u.runs_used as f32 / u.runs_limit.max(1) as f32).clamp(0., 1.);

        let meter_card = |meter_key: &'static str,
                          lead: AnyElement,
                          title: SharedString,
                          trailing: Option<AnyElement>,
                          used: SharedString,
                          cap: SharedString,
                          ratio: f32| {
            let high = ratio >= 0.85;
            let fill: Hsla = if high { rgb(0xe8740e).into() } else { p.blue };
            div()
                .flex_1()
                .min_w(px(140.))
                .flex()
                .flex_col()
                .gap(px(7.))
                .child(
                    div()
                        .h(px(24.))
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .child(lead)
                        .child(
                            div()
                                .text_size(px(13.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.ink)
                                .child(title),
                        )
                        .when_some(trailing, |d, el| d.child(el)),
                )
                .child(
                    div()
                        .flex()
                        .items_baseline()
                        .gap(px(4.))
                        .text_size(px(12.5))
                        .child(
                            div()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.ink)
                                .child(used),
                        )
                        .child(div().text_color(p.muted).child(format!("/ {cap}"))),
                )
                .child(pro_spring_bar(
                    pro_eid(&self.id, meter_key),
                    ratio,
                    4.,
                    2.,
                    p.chip,
                    fill,
                ))
        };

        let add_seat_btn = div()
            .ml_auto()
            .child(pro_icon_btn(
                pro_eid(&self.id, "btn-add-seats"),
                tr(cx, "Add seats", "增加席位"),
                ProIcon::Plus,
                24.,
                false,
                cx,
                |_, _, cx| {
                    cx.emit(SettingsSubscriptionEvent::Navigate("Members".into()));
                },
            ))
            .into_any_element();

        let credits_info = pro_svg("0 0 24 24", SVG_INFO_CIRCLE, 13., p.faint);

        let m_seats = meter_card(
            "meter-seats",
            pro_icon(ProIcon::Users, 15., p.muted),
            tr(cx, "Seats", "席位").into(),
            Some(add_seat_btn),
            u.seats_used.to_string().into(),
            u.seats_limit.to_string().into(),
            seats_ratio,
        );
        let m_credits = meter_card(
            "meter-credits",
            pro_icon(ProIcon::Sparkles, 15., p.muted),
            tr(cx, "Credits", "模型点数").into(),
            Some(credits_info),
            format_thousands(u.credits_used).into(),
            format_thousands(u.credits_limit).into(),
            credits_ratio,
        );
        let m_storage = meter_card(
            "meter-storage",
            pro_svg("0 0 24 24", SVG_STORAGE_RACK, 15., p.muted),
            tr(cx, "Storage", "存储空间").into(),
            None,
            format!("{:.1} GB", u.storage_used_gb).into(),
            format!("{:.0} GB", u.storage_limit_gb).into(),
            storage_ratio,
        );
        let m_runs = meter_card(
            "meter-runs",
            pro_svg("0 0 24 24", SVG_AGENT_BRAIN, 15., p.muted),
            tr(cx, "Agent runs", "智能体运行").into(),
            None,
            format_thousands(u.runs_used).into(),
            format_thousands(u.runs_limit).into(),
            runs_ratio,
        );

        div()
            .mt(px(28.))
            .p(px(3.))
            .rounded(px(14.))
            .bg(frame_tint)
            .border_1()
            .border_color(p.line)
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .when(narrow, |d| d.flex_col())
                    .child(left_card)
                    .child(right_card),
            )
            .child(
                div()
                    .px(px(18.))
                    .pt(px(16.))
                    .pb(px(14.))
                    .flex()
                    .flex_wrap()
                    .gap(px(20.))
                    .child(m_seats)
                    .child(m_credits)
                    .child(m_storage)
                    .child(m_runs),
            )
    }

    /// Capsule (`rounded_full()`) billing interval toggle per `DESIGN.md` capsule rule.
    fn render_billing_capsule_toggle(&self, cx: &Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let monthly_active = self.interval == BillingInterval::Monthly;
        let annual_active = self.interval == BillingInterval::Annual;
        let sensor = pro_selection_sensor();
        let interval_key = match self.interval {
            BillingInterval::Monthly => "monthly",
            BillingInterval::Annual => "annual",
        };

        div()
            .relative()
            .p(px(3.))
            .rounded_full()
            .bg(p.chip)
            .border_1()
            .border_color(p.line)
            .flex()
            .items_center()
            .gap(px(2.))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "billing-interval-pill"),
                interval_key,
                sensor.clone(),
                999.,
                p.card,
                Some(p.line),
                true,
            ))
            .child(activate(
                div()
                    .id(pro_eid(&self.id, "period-monthly"))
                    .role(Role::RadioButton)
                    .tab_index(0)
                    .relative()
                    .h(px(26.))
                    .px(px(12.))
                    .rounded_full()
                    .when(monthly_active, |d| d.child(pro_mark_selected(&sensor)))
                    .flex()
                    .items_center()
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(if monthly_active { p.ink } else { p.text })
                    .cursor_pointer()
                    .child(tr(cx, "Monthly", "月付")),
                cx,
                |this, _, cx| {
                    this.interval = BillingInterval::Monthly;
                    cx.emit(SettingsSubscriptionEvent::IntervalChanged(
                        BillingInterval::Monthly,
                    ));
                    cx.notify();
                },
            ))
            .child(activate(
                div()
                    .id(pro_eid(&self.id, "period-annual"))
                    .role(Role::RadioButton)
                    .tab_index(0)
                    .relative()
                    .h(px(26.))
                    .pl(px(12.))
                    .pr(px(8.))
                    .rounded_full()
                    .when(annual_active, |d| d.child(pro_mark_selected(&sensor)))
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(if annual_active { p.ink } else { p.text })
                    .cursor_pointer()
                    .child(tr(cx, "Yearly", "年付"))
                    .child(
                        div()
                            .h(px(18.))
                            .px(px(6.))
                            .rounded_full()
                            .bg(p.accent_soft)
                            .flex()
                            .items_center()
                            .text_size(px(10.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.blue)
                            .child("−20%"),
                    ),
                cx,
                |this, _, cx| {
                    this.interval = BillingInterval::Annual;
                    cx.emit(SettingsSubscriptionEvent::IntervalChanged(
                        BillingInterval::Annual,
                    ));
                    cx.notify();
                },
            ))
    }

    fn render_plan_bullet(limited: bool, bold_prefix: Option<&str>, body: &str, cx: &App) -> Div {
        let p = ProPalette::get(cx);
        div()
            .flex()
            .items_start()
            .gap(px(9.))
            .text_size(px(13.))
            .line_height(px(19.))
            .child(
                div()
                    .mt(px(1.5))
                    .size(px(16.))
                    .flex_none()
                    .rounded_full()
                    .bg(p.chip)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(pro_icon(
                        if limited {
                            ProIcon::Minus
                        } else {
                            ProIcon::Check
                        },
                        10.5,
                        if limited { p.muted } else { p.ink },
                    )),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_wrap()
                    .gap(px(4.))
                    .when_some(bold_prefix, |d, bp| {
                        d.child(
                            div()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.ink)
                                .child(bp.to_string()),
                        )
                    })
                    .child(div().text_color(p.text).child(body.to_string())),
            )
    }

    fn render_plans_tray(&self, narrow: bool, cx: &Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let frame_tint: Hsla = if p.dark {
            hsla(0., 0., 1., 0.03)
        } else {
            hsla(0., 0., 0., 0.022)
        };
        let current_rate = self.plan.price_per_member(self.interval);
        let team_rate = SubscriptionPlan::Team.price_per_member(self.interval);
        let biz_rate = SubscriptionPlan::Business.price_per_member(self.interval);
        let is_team_current = self.plan == SubscriptionPlan::Team;
        let is_biz_current = self.plan == SubscriptionPlan::Business;

        let team_card = div()
            .flex_1()
            .p(px(20.))
            .rounded(px(12.))
            .bg(p.card)
            .border_1()
            .border_color(if is_team_current { p.blue } else { p.line })
            .flex()
            .flex_col()
            .child(
                div()
                    .text_size(px(15.))
                    .line_height(px(22.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .child(tr(cx, "Team", "团队版")),
            )
            .child(
                div()
                    .mt(px(2.))
                    .text_size(px(13.))
                    .line_height(px(19.))
                    .text_color(p.muted)
                    .child(tr(
                        cx,
                        "Chat and agents for your whole team",
                        "面向全团队的统一模型对话与协作智能体",
                    )),
            )
            .child(pro_view_transition(
                pro_eid(&self.id, "team-rate-vtx"),
                format!("team-{team_rate}"),
                div()
                    .mt(px(16.))
                    .text_size(px(26.))
                    .line_height(px(32.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .child(format!("${team_rate}")),
            ))
            .child(div().text_size(px(12.5)).text_color(p.muted).child(tr(
                cx,
                "per member / month · models included",
                "每位成员 / 月 · 含主流模型额度",
            )))
            .child(
                div()
                    .mt(px(16.))
                    .mb(px(20.))
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(Self::render_plan_bullet(
                        false,
                        None,
                        tr(
                            cx,
                            "Chat with the leading models in one place",
                            "一站式调用前沿大模型",
                        ),
                        cx,
                    ))
                    .child(Self::render_plan_bullet(
                        false,
                        None,
                        tr(
                            cx,
                            "Your own agents, skills and integrations",
                            "自定义智能体、技能包与第三方集成",
                        ),
                        cx,
                    ))
                    .child(Self::render_plan_bullet(
                        false,
                        Some(tr(cx, "Workflows:", "工作流：")),
                        tr(cx, "2,500 runs per month", "每月 2,500 次运行"),
                        cx,
                    ))
                    .child(Self::render_plan_bullet(
                        false,
                        None,
                        "SSO, SCIM and SAML",
                        cx,
                    ))
                    .child(Self::render_plan_bullet(
                        true,
                        Some(tr(cx, "Meetings:", "会议纪要：")),
                        tr(
                            cx,
                            "10 recordings per member a month",
                            "每位成员每月 10 场录音",
                        ),
                        cx,
                    ))
                    .child(Self::render_plan_bullet(
                        true,
                        Some(tr(cx, "Apps:", "应用分享：")),
                        tr(
                            cx,
                            "share each app with up to 5 people",
                            "每个应用最多共享给 5 人",
                        ),
                        cx,
                    )),
            )
            .child(
                div().mt_auto().w_full().child(
                    pro_btn(
                        pro_eid(&self.id, "btn-upgrade-team"),
                        if is_team_current {
                            tr(cx, "Current plan", "当前方案")
                        } else {
                            tr(cx, "Upgrade to Team", "升级至团队版")
                        },
                        None,
                        ProBtnStyle::Secondary,
                        false,
                        cx,
                        |this, _, cx| {
                            this.plan = SubscriptionPlan::Team;
                            let interval = this.interval;
                            cx.emit(SettingsSubscriptionEvent::PlanChanged {
                                plan: SubscriptionPlan::Team,
                                interval,
                            });
                            this.show_toast("Switched workspace to Team plan", cx);
                        },
                    )
                    .w_full(),
                ),
            );

        let biz_card = div()
            .flex_1()
            .p(px(20.))
            .rounded(px(12.))
            .bg(p.card)
            .border_1()
            .border_color(p.blue.opacity(0.32))
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(15.))
                            .line_height(px(22.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .child(tr(cx, "Business", "商业版")),
                    )
                    .child(pro_badge(
                        tr(cx, "Recommended", "推荐"),
                        ProTone::Blue,
                        false,
                        cx,
                    )),
            )
            .child(
                div()
                    .mt(px(2.))
                    .text_size(px(13.))
                    .line_height(px(19.))
                    .text_color(p.muted)
                    .child(tr(
                        cx,
                        "Unlimited meetings and app sharing, for the whole company",
                        "不限次数的会议转录与全公司应用共享及合规治理",
                    )),
            )
            .child(pro_view_transition(
                pro_eid(&self.id, "biz-rate-vtx"),
                format!("biz-{biz_rate}"),
                div()
                    .mt(px(16.))
                    .text_size(px(26.))
                    .line_height(px(32.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .child(format!("${biz_rate}")),
            ))
            .child(div().text_size(px(12.5)).text_color(p.muted).child(tr(
                cx,
                "per member / month · models included",
                "每位成员 / 月 · 含主流模型额度",
            )))
            .child(
                div()
                    .mt(px(16.))
                    .text_size(px(13.))
                    .text_color(p.muted)
                    .child(tr(
                        cx,
                        "Everything in Team, plus:",
                        "包含团队版全部能力，另有：",
                    )),
            )
            .child(
                div()
                    .mt(px(12.))
                    .mb(px(20.))
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(Self::render_plan_bullet(
                        false,
                        Some(tr(cx, "Unlimited meetings:", "无限会议：")),
                        tr(
                            cx,
                            "record, transcribe and sum up every one",
                            "自动录制、转写并总结每场会议",
                        ),
                        cx,
                    ))
                    .child(Self::render_plan_bullet(
                        false,
                        Some(tr(cx, "Unlimited app sharing:", "无限应用共享：")),
                        tr(
                            cx,
                            "with anyone, any group or the whole workspace",
                            "可向任意成员、团队或全工作区发布",
                        ),
                        cx,
                    ))
                    .child(Self::render_plan_bullet(
                        false,
                        Some(tr(cx, "4× the workflow runs:", "4 倍工作流配额：")),
                        tr(cx, "10,000 per month", "每月 10,000 次运行"),
                        cx,
                    ))
                    .child(Self::render_plan_bullet(
                        false,
                        Some(tr(cx, "Governance:", "合规审计：")),
                        tr(
                            cx,
                            "for compliance and access control",
                            "精细数据留存与访问权限控制",
                        ),
                        cx,
                    )),
            )
            .child(
                div().mt_auto().w_full().child(
                    pro_btn(
                        pro_eid(&self.id, "btn-upgrade-business"),
                        if is_biz_current {
                            tr(cx, "Current plan", "当前方案")
                        } else {
                            tr(cx, "Upgrade to Business", "升级至商业版")
                        },
                        None,
                        ProBtnStyle::Primary,
                        false,
                        cx,
                        |this, _, cx| {
                            this.plan = SubscriptionPlan::Business;
                            let interval = this.interval;
                            cx.emit(SettingsSubscriptionEvent::PlanChanged {
                                plan: SubscriptionPlan::Business,
                                interval,
                            });
                            this.show_toast("Upgraded workspace to Business plan", cx);
                        },
                    )
                    .w_full(),
                ),
            );

        // Workflows Add-on cards
        let addon_price = self.addon_choice.monthly_price(self.interval);
        let starter_addon_card = div()
            .flex_1()
            .p(px(20.))
            .rounded(px(12.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .flex()
            .flex_col()
            .child(
                div()
                    .text_size(px(15.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .child(tr(cx, "Starter", "基础包")),
            )
            .child(
                div()
                    .mt(px(6.))
                    .text_size(px(26.))
                    .line_height(px(32.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.muted)
                    .child(tr(cx, "Included", "已包含")),
            )
            .child(
                div()
                    .mt(px(14.))
                    .mb(px(20.))
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(Self::render_plan_bullet(
                        false,
                        None,
                        "2,500 runs / month",
                        cx,
                    ))
                    .child(Self::render_plan_bullet(
                        false,
                        None,
                        tr(
                            cx,
                            "Model use in workflows is billed at API prices",
                            "工作流内模型调用按标准 API 单价计费",
                        ),
                        cx,
                    )),
            )
            .child(
                div().mt_auto().w_full().child(
                    pro_btn(
                        pro_eid(&self.id, "btn-addon-starter"),
                        if !self.addon_active {
                            tr(cx, "Current plan", "当前方案")
                        } else {
                            tr(cx, "Switch to Included", "切换至包含版")
                        },
                        None,
                        ProBtnStyle::Secondary,
                        false,
                        cx,
                        |this, _, cx| {
                            this.addon_active = false;
                            cx.emit(SettingsSubscriptionEvent::AddonChanged(
                                WorkflowAddonTier::StarterIncluded,
                            ));
                            this.show_toast("Switched Workflows add-on to Starter", cx);
                        },
                    )
                    .w_full(),
                ),
            );

        let mut biz_addon_card = div()
            .flex_1()
            .p(px(20.))
            .rounded(px(12.))
            .bg(p.card)
            .border_1()
            .border_color(if self.addon_active { p.blue } else { p.line })
            .flex()
            .flex_col()
            .child(
                div()
                    .text_size(px(15.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .child(tr(cx, "Business", "商业扩容包")),
            )
            .child(pro_view_transition(
                pro_eid(&self.id, "addon-rate-vtx"),
                format!("addon-{addon_price}"),
                div()
                    .mt(px(6.))
                    .text_size(px(26.))
                    .line_height(px(32.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .child(format!("${}", format_thousands(addon_price))),
            ))
            .child(div().text_size(px(12.5)).text_color(p.muted).child(tr(
                cx,
                "per workspace / month",
                "每工作区 / 月",
            )))
            .child(activate(
                div()
                    .id(pro_eid(&self.id, "btn-workflow-runs-tier"))
                    .role(Role::Button)
                    .aria_label(tr(cx, "Workflow runs", "工作流运行配额"))
                    .tab_index(0)
                    .mt(px(12.))
                    .w_full()
                    .h(px(32.))
                    .px(px(10.))
                    .rounded(px(8.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line_strong)
                    .hover(move |s| s.bg(p.hover))
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_size(px(13.))
                    .text_color(p.ink)
                    .cursor_pointer()
                    .child(self.addon_choice.picker_label())
                    .child(pro_icon(ProIcon::ChevronDown, 13., p.muted)),
                cx,
                |this, _, cx| {
                    this.addon_menu_open = !this.addon_menu_open;
                    cx.notify();
                },
            ));

        let mut tier_opts = div()
            .p(px(4.))
            .rounded(px(8.))
            .bg(p.subtle)
            .border_1()
            .border_color(p.line_strong)
            .flex()
            .flex_col()
            .gap(px(2.));
        for (ti, tier) in WorkflowAddonTier::PAID_TIERS.into_iter().enumerate() {
            let picked = self.addon_choice == tier;
            tier_opts = tier_opts.child(activate(
                div()
                    .id(pro_eid(&self.id, format!("wf-tier-opt-{ti}")))
                    .role(Role::MenuItem)
                    .tab_index(0)
                    .h(px(28.))
                    .px(px(8.))
                    .rounded(px(6.))
                    .when(picked, |d| d.bg(p.select))
                    .hover(move |s| s.bg(p.hover))
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_size(px(12.5))
                    .text_color(p.ink)
                    .cursor_pointer()
                    .child(tier.picker_label())
                    .child(format!(
                        "${}/mo",
                        format_thousands(tier.monthly_price(self.interval))
                    )),
                cx,
                move |this, _, cx| {
                    this.addon_choice = tier;
                    this.addon_menu_open = false;
                    if this.addon_active {
                        cx.emit(SettingsSubscriptionEvent::AddonChanged(tier));
                    }
                    cx.notify();
                },
            ));
        }
        let tier_opts_el = div().pt(px(6.)).child(tier_opts).into_any_element();
        biz_addon_card = biz_addon_card.child(pro_fold(
            pro_eid(&self.id, "addon-tier-fold"),
            self.addon_menu_open,
            102.,
            0.,
            move || tier_opts_el,
        ));

        biz_addon_card = biz_addon_card
            .child(
                div()
                    .mt(px(14.))
                    .mb(px(20.))
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(Self::render_plan_bullet(
                        false,
                        None,
                        self.addon_choice.bullet_runs(),
                        cx,
                    ))
                    .child(Self::render_plan_bullet(
                        false,
                        None,
                        tr(
                            cx,
                            "Model use in workflows is billed at API prices",
                            "工作流内模型调用按标准 API 单价计费",
                        ),
                        cx,
                    )),
            )
            .child(
                div().mt_auto().w_full().child(
                    pro_btn(
                        pro_eid(&self.id, "btn-addon-upgrade"),
                        if self.addon_active {
                            tr(cx, "Active add-on", "已启用")
                        } else {
                            tr(cx, "Upgrade", "升级")
                        },
                        None,
                        if self.addon_active {
                            ProBtnStyle::Primary
                        } else {
                            ProBtnStyle::Secondary
                        },
                        false,
                        cx,
                        |this, _, cx| {
                            this.addon_active = true;
                            let chosen = this.addon_choice;
                            cx.emit(SettingsSubscriptionEvent::AddonChanged(chosen));
                            this.show_toast("Enabled Workflows Business add-on", cx);
                        },
                    )
                    .w_full(),
                ),
            );

        let mut tray = div()
            .mt(px(14.))
            .p(px(if narrow { 12. } else { 16. }))
            .rounded(px(16.))
            .bg(frame_tint)
            .border_1()
            .border_color(p.line)
            .flex()
            .flex_col()
            .gap(px(12.))
            .child(
                div()
                    .px(px(2.))
                    .pb(px(4.))
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(12.))
                    .child(
                        div()
                            .size(px(34.))
                            .flex_none()
                            .rounded(px(9.))
                            .bg(p.card)
                            .border_1()
                            .border_color(p.line)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(pro_icon(ProIcon::Sparkles, 16., p.blue)),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(p.muted)
                                    .child(tr(cx, "Plan", "核心方案")),
                            )
                            .child(
                                div()
                                    .text_size(px(15.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.strong)
                                    .child(tr(
                                        cx,
                                        "Chat, agents, meetings & apps",
                                        "对话、智能体、会议与应用",
                                    )),
                            ),
                    )
                    .child(
                        div()
                            .ml_auto()
                            .flex()
                            .items_center()
                            .gap(px(12.))
                            .when(!narrow, |d| {
                                d.child(
                                    div()
                                        .text_size(px(13.))
                                        .text_color(p.muted)
                                        .child(format!(
                                            "{} {}",
                                            self.usage.seats_used,
                                            tr(cx, "members", "位成员")
                                        )),
                                )
                            })
                            .child(pro_btn(
                                pro_eid(&self.id, "btn-manage-members"),
                                tr(cx, "Manage members", "管理成员"),
                                None,
                                ProBtnStyle::Secondary,
                                true,
                                cx,
                                |_, _, cx| {
                                    cx.emit(SettingsSubscriptionEvent::Navigate("Members".into()));
                                },
                            )),
                    ),
            )
            .child(
                div()
                    .px(px(18.))
                    .py(px(14.))
                    .rounded(px(12.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(10.))
                    .child(
                        div()
                            .text_size(px(15.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .child(self.plan.name(cx)),
                    )
                    .child(pro_badge(
                        tr(cx, "Current plan", "当前方案"),
                        ProTone::Neutral,
                        false,
                        cx,
                    ))
                    .child(
                        div()
                            .ml_auto()
                            .flex()
                            .items_baseline()
                            .gap(px(5.))
                            .child(
                                div()
                                    .text_size(px(15.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.strong)
                                    .child(format!("${current_rate}")),
                            )
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .text_color(p.muted)
                                    .child(tr(cx, "per member / month", "每位成员 / 月")),
                            ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .when(narrow, |d| d.flex_col())
                    .gap(px(12.))
                    .child(team_card)
                    .child(biz_card),
            )
            .child(
                div()
                    .px(px(2.))
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(6.))
                    .text_size(px(13.))
                    .text_color(p.muted)
                    .child(tr(
                        cx,
                        "Using your own model keys?",
                        "正在使用自有模型密钥？",
                    ))
                    .child(activate(
                        div()
                            .id(pro_eid(&self.id, "link-byok-pricing"))
                            .role(Role::Button)
                            .tab_index(0)
                            .underline()
                            .text_color(p.ink)
                            .cursor_pointer()
                            .child(tr(
                                cx,
                                "See pricing for your own keys",
                                "查看自有密钥计费说明",
                            )),
                        cx,
                        |_, _, cx| {
                            cx.emit(SettingsSubscriptionEvent::Navigate("Models".into()));
                        },
                    )),
            )
            .child(
                div()
                    .px(px(18.))
                    .py(px(14.))
                    .rounded(px(12.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .when(narrow, |d| d.flex_col().items_start())
                    .when(!narrow, |d| d.items_center().justify_between())
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
                                    .text_size(px(15.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.strong)
                                    .child(tr(cx, "Enterprise", "企业版")),
                            )
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .text_color(p.muted)
                                    .child(tr(
                                        cx,
                                        "For large rollouts: volume pricing, a dedicated deployment and personal support",
                                        "面向大规模部署：阶梯折扣、专属私有化集群与客户成功支持",
                                    )),
                            ),
                    )
                    .child(pro_btn(
                        pro_eid(&self.id, "btn-contact-sales"),
                        tr(cx, "Contact sales", "联系商务"),
                        None,
                        ProBtnStyle::Secondary,
                        true,
                        cx,
                        |this, _, cx| {
                            this.show_toast("Requested Enterprise consultation", cx);
                        },
                    )),
            )
            .child(activate(
                div()
                    .id(pro_eid(&self.id, "btn-compare-plans"))
                    .role(Role::Button)
                    .tab_index(0)
                    .h(px(28.))
                    .px(px(2.))
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .text_size(px(13.))
                    .text_color(p.text)
                    .hover(move |s| s.text_color(p.ink))
                    .cursor_pointer()
                    .child(tr(cx, "Compare plans", "对比方案详情"))
                    .child(pro_icon(
                        if self.compare_open {
                            ProIcon::ChevronUp
                        } else {
                            ProIcon::ChevronDown
                        },
                        13.,
                        p.faint,
                    )),
                cx,
                |this, _, cx| {
                    this.compare_open = !this.compare_open;
                    cx.notify();
                },
            ));

        let rows = [
            ("Price / seat", "Free", "$49/mo", "$69/mo", "$89/mo"),
            ("Monthly credits", "2,000", "50,000", "100,000", "250,000"),
            ("Workflow runs", "250", "2,500", "2,500", "10,000"),
            ("Meeting recordings", "—", "5 / mo", "10 / mo", "Unlimited"),
            ("SSO, SCIM & SAML", "—", "—", "Included", "Included + Audit"),
        ];
        let mut cmp = div()
            .rounded(px(12.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .overflow_hidden()
            .flex()
            .flex_col()
            .child(
                div()
                    .min_h(px(40.))
                    .px(px(16.))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .text_size(px(12.5))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .child(div().flex_1().child(tr(cx, "Feature", "能力项")))
                    .child(div().w(px(76.)).child("Starter"))
                    .child(div().w(px(76.)).child("Pro"))
                    .child(div().w(px(76.)).child("Team"))
                    .child(div().w(px(92.)).child("Business")),
            );
        for (feat, c1, c2, c3, c4) in rows {
            cmp = cmp.child(
                div()
                    .min_h(px(38.))
                    .px(px(16.))
                    .border_t_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .text_size(px(12.5))
                    .child(div().flex_1().text_color(p.muted).child(feat))
                    .child(div().w(px(76.)).text_color(p.text).child(c1))
                    .child(div().w(px(76.)).text_color(p.text).child(c2))
                    .child(div().w(px(76.)).text_color(p.text).child(c3))
                    .child(div().w(px(92.)).text_color(p.ink).child(c4)),
            );
        }
        let cmp_el = cmp.into_any_element();
        tray = tray.child(pro_fold(
            pro_eid(&self.id, "compare-plans-fold"),
            self.compare_open,
            232.,
            12.,
            move || cmp_el,
        ));

        // Workflows Add-on header + cards
        tray.child(
            div()
                .mt(px(10.))
                .pt(px(18.))
                .border_t_1()
                .border_color(p.line)
                .flex()
                .items_center()
                .gap(px(12.))
                .child(
                    div()
                        .size(px(34.))
                        .flex_none()
                        .rounded(px(9.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(pro_svg("0 0 24 24", SVG_WORKFLOW_NODES, 16., p.blue)),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .child(div().text_size(px(12.)).text_color(p.muted).child(tr(
                            cx,
                            "Add-on",
                            "附加包",
                        )))
                        .child(
                            div()
                                .text_size(px(15.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.strong)
                                .child(tr(cx, "Workflows", "自动化工作流")),
                        ),
                )
                .child(div().ml_auto().child(pro_btn(
                    pro_eid(&self.id, "btn-manage-workflows"),
                    tr(cx, "Manage workflows", "管理工作流"),
                    None,
                    ProBtnStyle::Secondary,
                    true,
                    cx,
                    |_, _, cx| {
                        cx.emit(SettingsSubscriptionEvent::Navigate("Workflows".into()));
                    },
                ))),
        )
        .child(
            div()
                .flex()
                .when(narrow, |d| d.flex_col())
                .gap(px(12.))
                .child(starter_addon_card)
                .child(biz_addon_card),
        )
    }

    fn render_details_section(&self, narrow: bool, cx: &Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let frame_tint: Hsla = if p.dark {
            hsla(0., 0., 1., 0.03)
        } else {
            hsla(0., 0., 0., 0.022)
        };

        let mut address_box = div()
            .flex_1()
            .p(px(18.))
            .rounded(px(12.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .items_start()
                    .justify_between()
                    .gap(px(12.))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(1.))
                            .child(
                                div()
                                    .text_size(px(14.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.strong)
                                    .child(tr(cx, "Address", "账单地址")),
                            )
                            .child(div().text_size(px(12.5)).text_color(p.muted).child(tr(
                                cx,
                                "Shown on every invoice",
                                "显示在每一张正式发票上",
                            ))),
                    )
                    .child(pro_icon_btn(
                        pro_eid(&self.id, "btn-edit-address"),
                        tr(cx, "Edit address", "编辑账单地址"),
                        ProIcon::NewChat,
                        28.,
                        self.editing_address,
                        cx,
                        |this, _, cx| {
                            this.editing_address = !this.editing_address;
                            cx.notify();
                        },
                    )),
            );

        let addr_body: AnyElement = if self.editing_address {
            let field_wrap = |ed: Option<Entity<Editor>>| {
                div()
                    .h(px(30.))
                    .px(px(9.))
                    .rounded(px(6.))
                    .bg(p.subtle)
                    .border_1()
                    .border_color(p.line_strong)
                    .flex()
                    .items_center()
                    .when_some(ed, |d, e| d.child(div().w_full().child(e)))
            };
            div()
                .mt(px(14.))
                .flex()
                .flex_col()
                .gap(px(8.))
                .child(field_wrap(self.addr_email_ed.clone()))
                .child(field_wrap(self.addr_company_ed.clone()))
                .child(field_wrap(self.addr_line_ed.clone()))
                .child(field_wrap(self.addr_vat_ed.clone()))
                .child(
                    div()
                        .mt(px(4.))
                        .flex()
                        .justify_end()
                        .gap(px(6.))
                        .child(pro_btn(
                            pro_eid(&self.id, "addr-cancel"),
                            tr(cx, "Cancel", "取消"),
                            None,
                            ProBtnStyle::Ghost,
                            true,
                            cx,
                            |this, _, cx| {
                                this.editing_address = false;
                                cx.notify();
                            },
                        ))
                        .child(pro_btn(
                            pro_eid(&self.id, "addr-save"),
                            tr(cx, "Save", "保存"),
                            None,
                            ProBtnStyle::Primary,
                            true,
                            cx,
                            |this, _, cx| {
                                if let Some(ed) = &this.addr_email_ed {
                                    let v = ed.read(cx).value().trim().to_string();
                                    if !v.is_empty() {
                                        this.address.email = v.into();
                                    }
                                }
                                if let Some(ed) = &this.addr_company_ed {
                                    let v = ed.read(cx).value().trim().to_string();
                                    if !v.is_empty() {
                                        this.address.company = v.into();
                                    }
                                }
                                if let Some(ed) = &this.addr_line_ed {
                                    let v = ed.read(cx).value().trim().to_string();
                                    if !v.is_empty() {
                                        this.address.address = v.into();
                                    }
                                }
                                if let Some(ed) = &this.addr_vat_ed {
                                    let v = ed.read(cx).value().trim().to_string();
                                    this.address.vat_number = (!v.is_empty()).then(|| v.into());
                                }
                                this.editing_address = false;
                                cx.emit(SettingsSubscriptionEvent::AddressUpdated(
                                    this.address.clone(),
                                ));
                                this.show_toast("Saved billing address", cx);
                            },
                        )),
                )
                .into_any_element()
        } else {
            let fact_row = |k: &str, v: SharedString, is_empty: bool| {
                div()
                    .flex()
                    .items_start()
                    .gap(px(12.))
                    .text_size(px(13.))
                    .child(
                        div()
                            .w(px(96.))
                            .flex_none()
                            .text_color(p.muted)
                            .child(k.to_string()),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_color(if is_empty { p.faint } else { p.ink })
                            .child(v),
                    )
            };
            div()
                .mt(px(16.))
                .flex()
                .flex_col()
                .gap(px(10.))
                .child(fact_row(
                    tr(cx, "Email", "邮箱"),
                    self.address.email.clone(),
                    false,
                ))
                .child(fact_row(
                    tr(cx, "Company", "公司"),
                    self.address.company.clone(),
                    false,
                ))
                .child(fact_row(
                    tr(cx, "Address", "地址"),
                    self.address.address.clone(),
                    false,
                ))
                .child(fact_row(
                    tr(cx, "VAT number", "税号"),
                    self.address
                        .vat_number
                        .clone()
                        .unwrap_or_else(|| tr(cx, "Not provided", "未填写").into()),
                    self.address.vat_number.is_none(),
                ))
                .into_any_element()
        };
        address_box = address_box.child(pro_view_transition(
            pro_eid(&self.id, "address-mode-vtx"),
            if self.editing_address { "edit" } else { "view" },
            addr_body,
        ));

        let mut payment_box = div()
            .flex_1()
            .p(px(18.))
            .rounded(px(12.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .items_start()
                    .justify_between()
                    .gap(px(12.))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(1.))
                            .child(
                                div()
                                    .text_size(px(14.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.strong)
                                    .child(tr(cx, "Payment", "支付方式")),
                            )
                            .child(div().text_size(px(12.5)).text_color(p.muted).child(tr(
                                cx,
                                "Charged on every renewal",
                                "每次到期自动扣款",
                            ))),
                    )
                    .child(pro_icon_btn(
                        pro_eid(&self.id, "btn-add-payment"),
                        tr(cx, "Add payment method", "更换或添加银行卡"),
                        ProIcon::Plus,
                        28.,
                        self.adding_card,
                        cx,
                        |this, _, cx| {
                            this.adding_card = !this.adding_card;
                            cx.notify();
                        },
                    )),
            )
            .child(
                div()
                    .mt(px(16.))
                    .px(px(12.))
                    .py(px(10.))
                    .rounded(px(10.))
                    .bg(frame_tint)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .child(
                        div()
                            .w(px(40.))
                            .h(px(26.))
                            .flex_none()
                            .rounded(px(5.))
                            .bg(rgb(0x1a1f71))
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(px(10.))
                            .font_weight(FontWeight::BOLD)
                            .text_color(rgb(0xffffff))
                            .child("VISA"),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.strong)
                                    .child(format!("{} •••• {}", self.card.brand, self.card.last4)),
                            )
                            .child(div().text_size(px(12.)).text_color(p.muted).child(format!(
                                "{} {}",
                                tr(cx, "Expires", "有效期至"),
                                self.card.expires
                            ))),
                    )
                    .when(self.card.is_default, |d| {
                        d.child(pro_badge(
                            tr(cx, "Default", "默认"),
                            ProTone::Neutral,
                            false,
                            cx,
                        ))
                    }),
            );

        let card_form_el = div()
            .pt(px(10.))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(
                        div()
                            .flex()
                            .gap(px(8.))
                            .child(
                                div()
                                    .flex_1()
                                    .h(px(30.))
                                    .px(px(9.))
                                    .rounded(px(6.))
                                    .bg(p.subtle)
                                    .border_1()
                                    .border_color(p.line_strong)
                                    .flex()
                                    .items_center()
                                    .when_some(self.card_last4_ed.clone(), |d, e| {
                                        d.child(div().w_full().child(e))
                                    }),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .h(px(30.))
                                    .px(px(9.))
                                    .rounded(px(6.))
                                    .bg(p.subtle)
                                    .border_1()
                                    .border_color(p.line_strong)
                                    .flex()
                                    .items_center()
                                    .when_some(self.card_exp_ed.clone(), |d, e| {
                                        d.child(div().w_full().child(e))
                                    }),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap(px(6.))
                            .child(pro_btn(
                                pro_eid(&self.id, "card-cancel"),
                                tr(cx, "Cancel", "取消"),
                                None,
                                ProBtnStyle::Ghost,
                                true,
                                cx,
                                |this, _, cx| {
                                    this.adding_card = false;
                                    cx.notify();
                                },
                            ))
                            .child(pro_btn(
                                pro_eid(&self.id, "card-save"),
                                tr(cx, "Update card", "更新银行卡"),
                                None,
                                ProBtnStyle::Primary,
                                true,
                                cx,
                                |this, w, cx| {
                                    let raw4 = this
                                        .card_last4_ed
                                        .as_ref()
                                        .map(|e| e.read(cx).value().trim().to_string())
                                        .unwrap_or_default();
                                    let digits: String = raw4
                                        .chars()
                                        .filter(|c| c.is_ascii_digit())
                                        .take(4)
                                        .collect();
                                    if digits.len() == 4 {
                                        this.card.last4 = digits.into();
                                    }
                                    if let Some(ed) = &this.card_exp_ed {
                                        let exp = ed.read(cx).value().trim().to_string();
                                        if !exp.is_empty() {
                                            this.card.expires = exp.into();
                                        }
                                        ed.update(cx, |e, cx| e.set_value("", w, cx));
                                    }
                                    if let Some(ed) = &this.card_last4_ed {
                                        ed.update(cx, |e, cx| e.set_value("", w, cx));
                                    }
                                    this.adding_card = false;
                                    cx.emit(SettingsSubscriptionEvent::PaymentMethodUpdated(
                                        this.card.clone(),
                                    ));
                                    this.show_toast("Updated default payment card", cx);
                                },
                            )),
                    ),
            )
            .into_any_element();
        payment_box = payment_box.child(pro_fold(
            pro_eid(&self.id, "adding-card-fold"),
            self.adding_card,
            82.,
            0.,
            move || card_form_el,
        ));

        payment_box = payment_box.child(
            div()
                .mt_auto()
                .pt(px(16.))
                .flex()
                .items_center()
                .gap(px(6.))
                .text_size(px(12.))
                .text_color(p.muted)
                .child(pro_icon(ProIcon::CreditCard, 14., p.faint))
                .child(tr(
                    cx,
                    "Cards are stored by our payment provider – never on our servers.",
                    "卡片信息由认证支付服务商加密托管，从不落盘我方服务器。",
                )),
        );

        div()
            .mt(px(14.))
            .flex()
            .when(narrow, |d| d.flex_col())
            .gap(px(12.))
            .child(address_box)
            .child(payment_box)
    }

    fn render_history_section(&self, narrow: bool, cx: &Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        if self.empty {
            return div()
                .mt(px(14.))
                .rounded(px(12.))
                .border_1()
                .border_color(p.line)
                .child(pro_empty_state(
                    ProIcon::PageDoc,
                    tr(cx, "No invoices yet", "尚无历史账单"),
                    tr(
                        cx,
                        "Your first invoice will appear here after your next renewal.",
                        "首次续费结算完成后，所有电子发票与收据将在此归档。",
                    ),
                    Some((
                        pro_eid(&self.id, "empty-preview-bill"),
                        tr(cx, "Preview first bill", "预览首期账单").into(),
                        Box::new(|this: &mut Self, _, cx| {
                            this.bill_details_open = true;
                            cx.emit(SettingsSubscriptionEvent::InvoiceAction("Upcoming".into()));
                            this.show_toast("Opened upcoming bill breakdown", cx);
                        }),
                    )),
                    cx,
                ));
        }

        let upcoming_total = compute_upcoming_bill(
            self.usage.seats_used,
            self.plan,
            self.interval,
            self.active_addon(),
        );

        let mut table = div()
            .mt(px(14.))
            .rounded(px(12.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .overflow_hidden()
            .flex()
            .flex_col()
            .child(
                div()
                    .min_h(px(40.))
                    .pl(px(18.))
                    .pr(px(10.))
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .text_size(px(12.))
                    .text_color(p.muted)
                    .child(
                        div()
                            .w(px(if narrow { 92. } else { 140. }))
                            .flex_none()
                            .child(tr(cx, "Total incl. tax", "含税总额")),
                    )
                    .when(!narrow, |d| {
                        d.child(div().w(px(140.)).flex_none().child(tr(cx, "Date", "日期")))
                    })
                    .child(div().w(px(116.)).flex_none()),
            );

        // Upcoming row
        if !self.canceled {
            table = table.child(
                div()
                    .min_h(px(48.))
                    .pl(px(18.))
                    .pr(px(10.))
                    .border_t_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .text_size(px(13.))
                    .text_color(p.muted)
                    .child(
                        div()
                            .w(px(if narrow { 92. } else { 140. }))
                            .flex_none()
                            .child(format!("${}", format_thousands(upcoming_total))),
                    )
                    .when(!narrow, |d| {
                        d.child(div().w(px(140.)).flex_none().child("Dec 6, 2026"))
                    })
                    .child(
                        div()
                            .w(px(116.))
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_end()
                            .gap(px(6.))
                            .when(!narrow, |d| {
                                d.child(pro_badge(
                                    tr(cx, "Upcoming", "待出账"),
                                    ProTone::Neutral,
                                    false,
                                    cx,
                                ))
                            })
                            .child(pro_icon_btn(
                                pro_eid(&self.id, "inv-preview-upcoming"),
                                tr(cx, "Preview upcoming invoice", "预览下期账单"),
                                ProIcon::Eye,
                                28.,
                                false,
                                cx,
                                |this, _, cx| {
                                    this.bill_details_open = true;
                                    cx.emit(SettingsSubscriptionEvent::InvoiceAction(
                                        "Upcoming".into(),
                                    ));
                                    this.show_toast("Previewing Dec 6, 2026 bill", cx);
                                },
                            )),
                    ),
            );
        }

        // Paid rows
        for (ii, inv) in self.invoices.iter().enumerate() {
            let ref_id = inv.reference.clone();
            let dl_ref = ref_id.clone();
            table = table.child(
                div()
                    .min_h(px(48.))
                    .pl(px(18.))
                    .pr(px(10.))
                    .border_t_1()
                    .border_color(p.line)
                    .hover(move |s| s.bg(p.hover))
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .text_size(px(13.))
                    .text_color(p.ink)
                    .child(div().flex_1().truncate().child(ref_id.clone()))
                    .child(
                        div()
                            .w(px(if narrow { 92. } else { 140. }))
                            .flex_none()
                            .child(format!("${}", format_thousands(inv.total_usd))),
                    )
                    .when(!narrow, |d| {
                        d.child(div().w(px(140.)).flex_none().child(inv.date.clone()))
                    })
                    .child(
                        div()
                            .w(px(116.))
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_end()
                            .gap(px(6.))
                            .child(pro_badge(
                                tr(cx, "Paid", "已支付"),
                                ProTone::Green,
                                false,
                                cx,
                            ))
                            .child(pro_icon_btn(
                                pro_eid(&self.id, format!("inv-dl-{ii}")),
                                format!("Download {ref_id}"),
                                ProIcon::Download,
                                28.,
                                false,
                                cx,
                                move |this, _, cx| {
                                    cx.emit(SettingsSubscriptionEvent::InvoiceAction(
                                        dl_ref.clone(),
                                    ));
                                    this.show_toast(format!("Downloaded {dl_ref}.pdf"), cx);
                                },
                            )),
                    ),
            );
        }
        table
    }

    fn render_loading_skeleton(&self, phase: f32, cx: &Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        div()
            .mt(px(28.))
            .flex()
            .flex_col()
            .gap(px(24.))
            .child(
                div()
                    .p(px(20.))
                    .rounded(px(14.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .flex_col()
                    .gap(px(18.))
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .child(pro_skeleton_bar(220., 36., 8., phase, cx))
                            .child(pro_skeleton_bar(120., 32., 8., phase, cx)),
                    )
                    .child(div().flex().gap(px(16.)).children((0..4).map(|_| {
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap(px(8.))
                            .child(pro_skeleton_bar(80., 13., 4., phase, cx))
                            .child(pro_skeleton_bar(96., 11., 4., phase, cx))
                            .child(pro_skeleton_bar(110., 4., 2., phase, cx))
                    }))),
            )
            .child(div().flex().gap(px(12.)).children((0..2).map(|_| {
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
                    .child(pro_skeleton_bar(110., 18., 4., phase, cx))
                    .child(pro_skeleton_bar(72., 28., 5., phase, cx))
                    .child(pro_skeleton_bar(190., 12., 4., phase, cx))
                    .child(pro_skeleton_bar(170., 12., 4., phase, cx))
                    .child(pro_skeleton_bar(180., 30., 7., phase, cx))
            })))
            .child(
                div()
                    .rounded(px(12.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .flex_col()
                    .children((0..4).map(|i| {
                        div()
                            .h(px(48.))
                            .px(px(18.))
                            .when(i > 0, |d| d.border_t_1().border_color(p.line))
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(pro_skeleton_bar(130., 13., 4., phase, cx))
                            .child(pro_skeleton_bar(64., 18., 5., phase, cx))
                    })),
            )
    }

    fn render_page(&mut self, w: &mut Window, cx: &mut Context<Self>) -> Stateful<Div> {
        self.ensure_editors(w, cx);
        let p = ProPalette::get(cx);
        let reduced = Theme::global(cx).reduced_motion;

        if let Some((_, ts)) = &self.toast
            && ts.elapsed().as_secs_f32() > 2.45
        {
            self.toast = None;
        } else if self.toast.is_some() && !reduced {
            w.request_animation_frame();
        }
        let skel_phase = if self.loading && !reduced {
            w.request_animation_frame();
            (self.mounted_at.elapsed().as_secs_f32() * 0.9).fract()
        } else {
            0.25
        };

        let width = self.measured_width.get();
        let narrow = width < 760.;

        let mut content = div()
            .w_full()
            .max_w(px(880.))
            .mx_auto()
            .pt(px(if narrow { 32. } else { 46. }))
            .pb(px(56.))
            .px(px(if narrow { 20. } else { 32. }))
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .items_start()
                    .justify_between()
                    .gap(px(16.))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_size(px(24.))
                                    .line_height(px(32.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.strong)
                                    .child(tr(cx, "Billing", "账单与订阅")),
                            )
                            .child(
                                div()
                                    .mt(px(4.))
                                    .text_size(px(13.))
                                    .line_height(px(20.))
                                    .text_color(p.text)
                                    .child(tr(
                                        cx,
                                        "See your plan, what you pay and what you've used.",
                                        "查看当前套餐、续费账单与团队资源用量。",
                                    )),
                            ),
                    )
                    .child(activate(
                        div()
                            .id(pro_eid(&self.id, "btn-open-docs"))
                            .role(Role::Button)
                            .tab_index(0)
                            .h(px(28.))
                            .mt(px(2.))
                            .flex()
                            .items_center()
                            .gap(px(4.))
                            .text_size(px(13.))
                            .text_color(p.text)
                            .hover(move |s| s.text_color(p.ink))
                            .cursor_pointer()
                            .child(tr(cx, "Open docs", "计费文档"))
                            .child(pro_icon(ProIcon::ExternalArrow, 14., p.muted)),
                        cx,
                        |this, _, cx| {
                            this.show_toast("Opened billing & seat proration guide", cx);
                        },
                    )),
            );

        if self.loading {
            content = content.child(self.render_loading_skeleton(skel_phase, cx));
        } else {
            content = content
                .child(self.render_overview_section(narrow, cx))
                .child(
                    div()
                        .mt(px(44.))
                        .flex()
                        .when(narrow, |d| d.flex_col().items_start().gap(px(12.)))
                        .when(!narrow, |d| d.items_end().justify_between().gap(px(16.)))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(2.))
                                .child(
                                    div()
                                        .text_size(px(14.))
                                        .line_height(px(20.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.strong)
                                        .child(tr(cx, "Plans & add-ons", "方案与附加包")),
                                )
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .line_height(px(20.))
                                        .text_color(p.muted)
                                        .child(tr(
                                            cx,
                                            "Change your plan any time – we prorate the difference.",
                                            "随时升级或调整方案，系统将自动按天折算差价。",
                                        )),
                                ),
                        )
                        .child(self.render_billing_capsule_toggle(cx)),
                )
                .child(self.render_plans_tray(narrow, cx))
                .child(
                    div()
                        .mt(px(44.))
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .child(
                            div()
                                .text_size(px(14.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.strong)
                                .child(tr(cx, "Billing details", "开票与支付信息")),
                        )
                        .child(
                            div()
                                .text_size(px(13.))
                                .text_color(p.muted)
                                .child(tr(
                                    cx,
                                    "Where the invoices go and how they're paid.",
                                    "管理账单接收信息及默认续费银行卡。",
                                )),
                        ),
                )
                .child(self.render_details_section(narrow, cx))
                .child(
                    div()
                        .mt(px(44.))
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .child(
                            div()
                                .text_size(px(14.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.strong)
                                .child(tr(cx, "History", "历史账单")),
                        )
                        .child(
                            div()
                                .text_size(px(13.))
                                .text_color(p.muted)
                                .child(tr(
                                    cx,
                                    "Your past invoices and payments.",
                                    "过往全部账单、税票及付款记录。",
                                )),
                        ),
                )
                .child(pro_view_transition(
                    pro_eid(&self.id, "history-vtx"),
                    if self.empty { "empty" } else { "table" },
                    self.render_history_section(narrow, cx),
                ));

            // Cancel / Resume subscription section
            let cancel_state_key = if self.cancel_confirm_open {
                "confirm"
            } else if self.canceled {
                "resume"
            } else {
                "cancel"
            };
            let cancel_el: AnyElement = if self.cancel_confirm_open {
                div()
                    .mt(px(28.))
                    .max_w(px(380.))
                    .p(px(16.))
                    .rounded(px(12.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line_strong)
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(
                        div()
                            .text_size(px(13.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .child(tr(cx, "Cancel subscription?", "确认取消订阅？")),
                    )
                    .child(
                        div()
                            .text_size(px(12.5))
                            .line_height(px(18.))
                            .text_color(p.muted)
                            .child(tr(
                                cx,
                                "Your workspace keeps full access until Dec 6, 2026. No further renewals will be charged.",
                                "工作区全部权益将保留至 2026 年 12 月 6 日，到期后不再扣费。",
                            )),
                    )
                    .child(
                        div()
                            .mt(px(4.))
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .child(pro_btn(
                                pro_eid(&self.id, "cancel-keep"),
                                tr(cx, "Keep plan", "保留当前方案"),
                                None,
                                ProBtnStyle::Secondary,
                                true,
                                cx,
                                |this, _, cx| {
                                    this.cancel_confirm_open = false;
                                    cx.notify();
                                },
                            ))
                            .child(pro_btn(
                                pro_eid(&self.id, "cancel-confirm"),
                                tr(cx, "Confirm cancel", "确认取消"),
                                None,
                                ProBtnStyle::Danger,
                                true,
                                cx,
                                |this, _, cx| {
                                    this.canceled = true;
                                    this.cancel_confirm_open = false;
                                    cx.emit(SettingsSubscriptionEvent::SubscriptionCanceled(true));
                                    this.show_toast("Subscription will end on Dec 6, 2026", cx);
                                },
                            )),
                    )
                    .into_any_element()
            } else {
                div()
                    .mt(px(32.))
                    .flex()
                    .child(if self.canceled {
                        pro_btn(
                            pro_eid(&self.id, "btn-resume-sub"),
                            tr(cx, "Resume subscription", "恢复自动续费"),
                            Some(ProIcon::Refresh),
                            ProBtnStyle::Primary,
                            true,
                            cx,
                            |this, _, cx| {
                                this.canceled = false;
                                cx.emit(SettingsSubscriptionEvent::SubscriptionCanceled(false));
                                this.show_toast("Resumed automatic renewal", cx);
                            },
                        )
                    } else {
                        pro_btn(
                            pro_eid(&self.id, "btn-cancel-sub"),
                            tr(cx, "Cancel subscription", "取消订阅"),
                            Some(ProIcon::Close),
                            ProBtnStyle::Secondary,
                            true,
                            cx,
                            |this, _, cx| {
                                this.cancel_confirm_open = true;
                                cx.notify();
                            },
                        )
                    })
                    .into_any_element()
            };
            content = content.child(pro_view_transition(
                pro_eid(&self.id, "cancel-sub-vtx"),
                cancel_state_key,
                cancel_el,
            ));
        }

        let measure_cell = self.measured_width.clone();
        div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .relative()
            .size_full()
            .min_h(px(520.))
            .bg(p.panel)
            .overflow_hidden()
            .flex()
            .flex_col()
            .on_key_down(cx.listener(Self::handle_key))
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
                    .id(pro_eid(&self.id, "scroll"))
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(content),
            )
            .child(pro_toast_banner(self.toast.clone()))
    }
}

impl Render for SettingsSubscriptionScreen {
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
        BillingInterval, InvoiceStatus, SubscriptionPlan, WorkflowAddonTier, compute_upcoming_bill,
        default_billing_invoices,
    };

    #[test]
    fn monthly_and_annual_plan_pricing_and_upcoming_bill_reconcile() {
        // Default 6 members on Pro ($49/mo) with Starter Included workflows = $294
        assert_eq!(
            SubscriptionPlan::Pro.price_per_member(BillingInterval::Monthly),
            49
        );
        assert_eq!(
            SubscriptionPlan::Team.price_per_member(BillingInterval::Monthly),
            69
        );
        assert_eq!(
            SubscriptionPlan::Business.price_per_member(BillingInterval::Monthly),
            89
        );
        assert_eq!(
            compute_upcoming_bill(
                6,
                SubscriptionPlan::Pro,
                BillingInterval::Monthly,
                WorkflowAddonTier::StarterIncluded
            ),
            294
        );

        // Annual applies -20% ($39/seat/mo * 6 seats * 12 months = $2,808)
        assert_eq!(
            SubscriptionPlan::Pro.price_per_member(BillingInterval::Annual),
            39
        );
        assert_eq!(
            compute_upcoming_bill(
                6,
                SubscriptionPlan::Pro,
                BillingInterval::Annual,
                WorkflowAddonTier::StarterIncluded
            ),
            2808
        );

        // Adding 40k workflow runs ($539/mo) to 6 Business seats ($89 * 6 = $534) = $1,073/mo
        assert_eq!(
            compute_upcoming_bill(
                6,
                SubscriptionPlan::Business,
                BillingInterval::Monthly,
                WorkflowAddonTier::Runs40k
            ),
            1073
        );
    }

    #[test]
    fn default_invoices_match_reference_history() {
        let history = default_billing_invoices();
        assert_eq!(history.len(), 4);
        assert_eq!(history[0].reference.as_ref(), "INV-2026-0011");
        assert_eq!(history[0].total_usd, 294);
        assert_eq!(history[0].status, InvoiceStatus::Paid);
        assert_eq!(history[3].reference.as_ref(), "INV-2026-0008");
        assert_eq!(history[3].total_usd, 245);
    }
}
