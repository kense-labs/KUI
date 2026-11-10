//! Present a usage limit, a billing choice, and the result of an upgrade.
use crate::{
    components::{
        animated_counter::AnimatedCounter,
        billing_toggle::{BillingChanged, BillingPeriod, BillingToggle},
    },
    foundation::{Theme, ThemeMode, surface_radius},
    primitives::{
        brand_mark::brand_mark,
        family::{Paint, PhaseClock, Slide, Tone, disabled_motion, ease, face},
        glyph::{Glyph, icon, spinner},
        presentation::{column, row, tr},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc, time::Duration};

/// One upgrade offer. Prices use hundredths of the display currency.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpgradePlan {
    pub name: SharedString,
    pub currency: SharedString,
    pub monthly: u64,
    pub yearly_per_month: u64,
    pub limit: u64,
    pub features: Vec<UpgradeFeature>,
    pub limits: Vec<UpgradeLimit>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpgradeFeature {
    pub id: SharedString,
    pub label: SharedString,
}

impl UpgradeFeature {
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpgradeLimit {
    pub label: SharedString,
    pub value: SharedString,
}

impl UpgradeLimit {
    pub fn new(label: impl Into<SharedString>, value: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
        }
    }
}

impl UpgradePlan {
    pub fn new(
        name: impl Into<SharedString>,
        monthly: u64,
        yearly_per_month: u64,
        limit: u64,
    ) -> Self {
        Self {
            name: name.into(),
            currency: "USD".into(),
            monthly,
            yearly_per_month,
            limit,
            features: Vec::new(),
            limits: Vec::new(),
        }
    }

    fn currency_prefix(&self) -> SharedString {
        match self.currency.as_ref() {
            "USD" => "$".into(),
            "EUR" => "€".into(),
            "GBP" => "£".into(),
            "JPY" | "CNY" => "¥".into(),
            code => format!("{code}\u{a0}").into(),
        }
    }

    fn price(&self, period: BillingPeriod) -> u64 {
        match period {
            BillingPeriod::Monthly => self.monthly,
            BillingPeriod::Yearly => self.yearly_per_month,
        }
    }

    fn savings(&self) -> SharedString {
        if self.monthly == 0 || self.yearly_per_month >= self.monthly {
            return "".into();
        }
        let monthly = u128::from(self.monthly);
        let saving = u128::from(self.monthly - self.yearly_per_month);
        format!("−{}%", (100 * saving + monthly / 2) / monthly).into()
    }
}

impl Default for UpgradePlan {
    fn default() -> Self {
        let mut plan = Self::new("Pro", 2000, 1600, 2000);
        plan.features = vec![
            UpgradeFeature::new("credits", "2,000 AI credits a month"),
            UpgradeFeature::new("models", "Access to advanced models"),
            UpgradeFeature::new("queue", "Priority queue with no daily caps"),
        ];
        plan.limits = vec![
            UpgradeLimit::new("AI credits", "2,000 a month"),
            UpgradeLimit::new("Models", "Advanced models"),
            UpgradeLimit::new("Queue", "Priority"),
        ];
        plan
    }
}

/// Usage and copy supplied by the host.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpgradePromptOptions {
    pub used: u64,
    pub limit: u64,
    pub unit: SharedString,
    pub resets_on: SharedString,
    pub plan: UpgradePlan,
    pub default_interval: BillingPeriod,
    pub controlled_interval: Option<BillingPeriod>,
    pub title: Option<SharedString>,
    pub description: Option<SharedString>,
}

impl Default for UpgradePromptOptions {
    fn default() -> Self {
        Self {
            used: 50,
            limit: 50,
            unit: "AI credits".into(),
            resets_on: "Nov 1".into(),
            plan: UpgradePlan::default(),
            default_interval: BillingPeriod::Yearly,
            controlled_interval: None,
            title: None,
            description: None,
        }
    }
}

/// Display snapshot of the selection. The host authorizes and calculates real charges.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpgradeSelection {
    pub plan: SharedString,
    pub interval: BillingPeriod,
    pub amount: u64,
    pub currency: SharedString,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UpgradeEvent {
    IntervalChanged(BillingPeriod),
    Dismissed,
    Upgraded(UpgradeSelection),
    Continued,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Panel {
    Offer,
    Success,
    Reminder,
}

type Payment = Box<dyn Fn(UpgradeSelection, &mut App) -> Task<Result<(), SharedString>>>;

pub struct UpgradePrompt {
    options: UpgradePromptOptions,
    interval: BillingPeriod,
    billing: Entity<BillingToggle>,
    price: Entity<AnimatedCounter>,
    _billing_subscription: Subscription,
    payment: Option<Payment>,
    pending: Option<Task<()>>,
    receipt: Option<UpgradeSelection>,
    error: Option<SharedString>,
    panel: Panel,
    previous_panel: Panel,
    panel_clock: PhaseClock,
    spinner_clock: PhaseClock,
    meter: Slide,
    bounds: Rc<Cell<Bounds<Pixels>>>,
    revealed: bool,
    primary_focus: FocusHandle,
    later_focus: FocusHandle,
}

impl EventEmitter<UpgradeEvent> for UpgradePrompt {}

impl UpgradePrompt {
    pub fn new(options: UpgradePromptOptions, cx: &mut Context<Self>) -> Self {
        let interval = options
            .controlled_interval
            .unwrap_or(options.default_interval);
        let label = tr(cx, "Billing interval", "计费周期");
        let month = tr(cx, "Monthly", "月付");
        let year = tr(cx, "Yearly", "年付");
        let savings = options.plan.savings();
        let billing = cx.new(|cx| {
            BillingToggle::new(label, cx)
                .with_hide_label(true)
                .with_labels(month, year)
                .with_savings(savings)
                .with_inline_badge()
                .with_period(interval)
        });
        let _billing_subscription = cx.subscribe(&billing, |s, toggle, e: &BillingChanged, cx| {
            if s.interval == e.0 {
                return;
            }
            if s.options.controlled_interval.is_some() {
                toggle.update(cx, |toggle, cx| toggle.set_period(s.interval, cx));
            } else {
                s.set_interval(e.0, cx);
            }
            cx.emit(UpgradeEvent::IntervalChanged(e.0));
        });
        let decimal_places = usize::from(
            !options.plan.monthly.is_multiple_of(100)
                || !options.plan.yearly_per_month.is_multiple_of(100),
        ) * 2;
        let price = cx.new(|_| {
            AnimatedCounter::new(options.plan.price(interval) as f64 / 100.)
                .with_prefix(options.plan.currency_prefix())
                .with_size(18.)
                .with_decimals(decimal_places)
                .with_wheel_style(24. / 18., 0., FontWeight::MEDIUM)
                .with_wheel_duration(0.28)
                .with_accessibility(false)
        });
        Self {
            options,
            interval,
            billing,
            price,
            _billing_subscription,
            payment: None,
            pending: None,
            receipt: None,
            error: None,
            panel: Panel::Offer,
            previous_panel: Panel::Offer,
            panel_clock: Default::default(),
            spinner_clock: Default::default(),
            meter: Slide::new(0.),
            bounds: Default::default(),
            revealed: false,
            primary_focus: cx.focus_handle().tab_stop(true),
            later_focus: cx.focus_handle().tab_stop(true),
        }
    }

    /// Use a host-owned, cancellable task to activate the selected plan.
    pub fn with_upgrade(
        mut self,
        upgrade: impl Fn(UpgradeSelection, &mut App) -> Task<Result<(), SharedString>> + 'static,
    ) -> Self {
        self.payment = Some(Box::new(upgrade));
        self
    }

    /// Update the selection without emitting a user intent.
    pub fn set_interval(&mut self, interval: BillingPeriod, cx: &mut Context<Self>) {
        if self.interval == interval {
            return;
        }
        self.interval = interval;
        if self.options.controlled_interval.is_some() {
            self.options.controlled_interval = Some(interval);
        }
        self.billing
            .update(cx, |billing, cx| billing.set_period(interval, cx));
        self.price.update(cx, |price, cx| {
            price.set_value(self.options.plan.price(interval) as f64 / 100., cx);
        });
        cx.notify();
    }

    /// Update the usage shown in both the offer and success views.
    pub fn set_usage(&mut self, used: u64, limit: u64, cx: &mut Context<Self>) {
        self.options.used = used;
        self.options.limit = limit;
        cx.notify();
    }

    fn selection(&self) -> UpgradeSelection {
        let plan = &self.options.plan;
        UpgradeSelection {
            plan: plan.name.clone(),
            interval: self.interval,
            amount: plan
                .price(self.interval)
                .saturating_mul(match self.interval {
                    BillingPeriod::Monthly => 1,
                    BillingPeriod::Yearly => 12,
                }),
            currency: plan.currency.clone(),
        }
    }

    fn show(&mut self, panel: Panel, w: &mut Window, cx: &mut Context<Self>) {
        if self.panel != panel {
            self.previous_panel = self.panel;
            self.panel = panel;
            self.panel_clock.restart(cx);
            self.billing.update(cx, |toggle, cx| {
                toggle.set_disabled(panel != Panel::Offer, cx)
            });
            self.primary_focus.focus(w, cx);
            cx.notify();
        }
    }

    fn start(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.panel != Panel::Offer || self.pending.is_some() {
            return;
        }
        self.error = None;
        let selection = self.selection();
        let charge = self.payment.as_ref().map(|pay| pay(selection.clone(), cx));
        self.billing
            .update(cx, |toggle, cx| toggle.set_disabled(true, cx));
        self.spinner_clock.restart(cx);
        self.pending = Some(cx.spawn_in(w, async move |this, cx| {
            let result = if let Some(charge) = charge {
                charge.await
            } else {
                cx.background_executor()
                    .timer(Duration::from_millis(1100))
                    .await;
                Ok(())
            };
            let _ = cx.update(|w, cx| {
                this.update(cx, |s, cx| {
                    s.pending = None;
                    match result {
                        Ok(()) => {
                            s.receipt = Some(selection.clone());
                            s.show(Panel::Success, w, cx);
                            cx.emit(UpgradeEvent::Upgraded(selection));
                        }
                        Err(message) => {
                            s.error = Some(if message.is_empty() {
                                tr(
                                    cx,
                                    "The upgrade failed. Please try again.",
                                    "升级失败，请重试。",
                                )
                                .into()
                            } else {
                                message
                            });
                            s.billing
                                .update(cx, |toggle, cx| toggle.set_disabled(false, cx));
                            s.primary_focus.focus(w, cx);
                        }
                    }
                    cx.notify();
                })
            });
        }));
        cx.notify();
    }

    fn offer_copy(&self, cx: &App) -> (SharedString, SharedString) {
        let used = self.options.used;
        let limit = self.options.limit;
        let unit = &self.options.unit;
        let reset = &self.options.resets_on;
        let title = self.options.title.clone().unwrap_or_else(|| {
            if used >= limit {
                localized(
                    cx,
                    format!("You've used all {} {unit}", digits(limit)),
                    format!("已用完全部 {} {unit}", digits(limit)),
                )
            } else {
                localized(
                    cx,
                    format!("You've used {} {unit}", digits(used)),
                    format!("已使用 {} {unit}", digits(used)),
                )
            }
        });
        let detail = self.options.description.clone().unwrap_or_else(|| {
            localized(
                cx,
                format!("Your free {unit} reset {reset}. Upgrade to keep going without the wait."),
                format!("免费{unit}将于{reset}恢复。升级后即可继续使用。"),
            )
        });
        (title, detail)
    }

    fn header_copy(&self, panel: Panel, cx: &App) -> (SharedString, SharedString) {
        if panel == Panel::Success {
            (
                localized(
                    cx,
                    format!("You're on {}", self.options.plan.name),
                    format!("已升级至 {}", self.options.plan.name),
                ),
                tr(
                    cx,
                    "Your new limits are live. Pick up right where you left off.",
                    "新额度已生效，可继续操作。",
                )
                .into(),
            )
        } else {
            self.offer_copy(cx)
        }
    }

    fn header(&self, panel: Panel, p: Paint, cx: &App) -> Div {
        let (title, detail) = self.header_copy(panel, cx);
        column()
            .gap(px(4.))
            .child(
                div()
                    .id("upgrade-heading")
                    .role(Role::Heading)
                    .text_size(px(18.))
                    .line_height(px(20.))
                    .font_weight(FontWeight::MEDIUM)
                    .child(title),
            )
            .child(div().min_h(px(40.)).text_color(p.secondary).child(detail))
    }

    fn meter_labels(&self, panel: Panel, p: Paint, cx: &App) -> Div {
        let limit = self.limit(panel);
        let used = self.options.used;
        let exhausted = used >= limit;
        let amber: Hsla = rgb(if Theme::global(cx).mode == ThemeMode::Dark {
            0xe9b65f
        } else {
            0x995e1a
        })
        .into();
        let label = if exhausted {
            tr(cx, "Limit reached", "已达到上限").into()
        } else {
            let left = digits(limit - used);
            localized(
                cx,
                format!("{left} {} left", self.options.unit),
                format!("剩余 {left} {}", self.options.unit),
            )
        };
        let ink = if exhausted { amber } else { p.success };
        let state_icon = div()
            .size(px(14.))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .size(px(12.))
                    .rounded_full()
                    .border_1()
                    .border_color(ink)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(if exhausted {
                        column()
                            .items_center()
                            .gap(px(1.))
                            .child(div().w(px(1.)).h(px(3.)).bg(ink))
                            .child(div().size(px(1.)).bg(ink))
                            .into_any_element()
                    } else {
                        icon(Glyph::Check, 8., ink).into_any_element()
                    }),
            );
        row()
            .justify_between()
            .h(px(22.))
            .gap(px(8.))
            .child(
                row()
                    .flex_1()
                    .min_w_0()
                    .gap(px(6.))
                    .child(state_icon)
                    .child(div().truncate().child(label)),
            )
            .child(
                div()
                    .flex_none()
                    .text_color(p.faint)
                    .font_features(FontFeatures(vec![("tnum".into(), 1)].into()))
                    .child(localized(
                        cx,
                        format!("{} of {}", digits(used), digits(limit)),
                        format!("{} / {}", digits(used), digits(limit)),
                    )),
            )
    }

    fn limit(&self, panel: Panel) -> u64 {
        if panel == Panel::Success {
            self.options.plan.limit
        } else {
            self.options.limit
        }
    }

    fn body(&self, panel: Panel, stacked: bool, p: Paint, cx: &App) -> Div {
        let plan = &self.options.plan;
        match panel {
            Panel::Offer => {
                let charge = self.selection();
                let recurring = match charge.interval {
                    BillingPeriod::Yearly => localized(
                        cx,
                        format!(
                            "{} billed yearly. Cancel anytime.",
                            money(plan, charge.amount)
                        ),
                        format!("每年支付 {}，可随时取消。", money(plan, charge.amount)),
                    ),
                    BillingPeriod::Monthly => tr(
                        cx,
                        "Billed monthly. Cancel anytime.",
                        "按月付费，可随时取消。",
                    )
                    .into(),
                };
                column()
                    .gap(px(16.))
                    .child(
                        div()
                            .flex()
                            .gap(px(12.))
                            .items_center()
                            .justify_between()
                            .when(stacked, |d| d.flex_col().items_start().gap(px(8.)))
                            .child(
                                column()
                                    .when(!stacked, |d| d.flex_1())
                                    .min_w_0()
                                    .gap(px(2.))
                                    .child(
                                        row()
                                            .h(px(24.))
                                            .gap(px(6.))
                                            .child(
                                                div()
                                                    .id("upgrade-price")
                                                    .role(Role::Group)
                                                    .aria_label(money(
                                                        plan,
                                                        plan.price(self.interval),
                                                    ))
                                                    .child(self.price.clone()),
                                            )
                                            .child(
                                                div()
                                                    .text_color(p.faint)
                                                    .child(tr(cx, "/ month", "/ 月")),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .line_height(px(18.))
                                            .text_color(p.faint)
                                            .max_w_full()
                                            .truncate()
                                            .child(recurring),
                                    ),
                            )
                            .child(self.billing.clone()),
                    )
                    .children(
                        plan.features
                            .iter()
                            .map(|feature| {
                                row()
                                    .gap(px(8.))
                                    .h(px(20.))
                                    .child(icon(Glyph::Check, 14., p.secondary))
                                    .children(
                                        feature
                                            .label
                                            .split(" and ")
                                            .filter_map(|model| brand_mark(model, 14., cx)),
                                    )
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_w_0()
                                            .truncate()
                                            .child(feature.label.clone()),
                                    )
                            })
                            .enumerate()
                            .map(|(i, item)| if i == 0 { item } else { item.mt(px(-8.)) }),
                    )
            }
            Panel::Success => {
                let amount = self.receipt.as_ref().map_or(0, |r| r.amount);
                let detail = if self.payment.is_none() {
                    localized(
                        cx,
                        format!("{} today. Simulation only, no charge.", money(plan, amount)),
                        format!("本次仅为模拟，未扣款。账单金额 {}。", money(plan, amount)),
                    )
                } else {
                    localized(
                        cx,
                        format!("{} billed today.", money(plan, amount)),
                        format!("今日扣费 {}。", money(plan, amount)),
                    )
                };
                column()
                    .gap(px(12.))
                    .child(
                        column().children(plan.limits.iter().enumerate().map(|(i, item)| {
                            row()
                                .min_h(px(if i == 0 { 28. } else { 36. }))
                                .justify_between()
                                .gap(px(10.))
                                .when(i > 0, |d| d.border_t_1().border_color(p.line.opacity(0.65)))
                                .child(div().text_color(p.secondary).child(item.label.clone()))
                                .child(div().min_w_0().truncate().child(item.value.clone()))
                        })),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .line_height(px(18.))
                            .text_color(p.faint)
                            .child(detail),
                    )
            }
            Panel::Reminder => {
                let reset = &self.options.resets_on;
                row().items_start().gap(px(10.)).pt(px(24.))
                    .child(icon(Glyph::Bell, 18., p.secondary))
                    .child(column().flex_1().min_w_0().gap(px(4.))
                        .child(div().font_weight(FontWeight::MEDIUM).child(localized(cx,
                            format!("We'll remind you {reset}"), format!("将在{reset}提醒你"))))
                        .child(div().text_color(p.secondary).child(tr(cx,
                            "Your credits reset then, and your work stays here. You can upgrade whenever you need more.",
                            "额度恢复后即可继续使用，已有内容会保留。需要更多额度时，可以随时回来。"))))
            }
        }
    }
}

fn localized(cx: &App, en: String, zh: String) -> SharedString {
    if tr(cx, "en", "zh") == "en" {
        en.into()
    } else {
        zh.into()
    }
}

fn digits(number: u64) -> String {
    let raw = number.to_string();
    let first = raw.len() % 3;
    let mut grouped = String::with_capacity(raw.len() + raw.len() / 3);
    for (i, c) in raw.chars().enumerate() {
        if i > 0 && (i + 3 - first).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(c);
    }
    grouped
}

fn money(plan: &UpgradePlan, amount: u64) -> String {
    let whole = digits(amount / 100);
    if amount.is_multiple_of(100) {
        format!("{}{whole}", plan.currency_prefix())
    } else {
        format!("{}{whole}.{:02}", plan.currency_prefix(), amount % 100)
    }
}

fn wrapped_height(
    text: SharedString,
    width: Pixels,
    size: Pixels,
    line: Pixels,
    w: &Window,
) -> Pixels {
    let run = w.text_style().to_run(text.len());
    w.text_system()
        .shape_text(text, size, &[run], Some(width), None)
        .map(|rows| rows.iter().map(|row| row.size(line).height).sum())
        .unwrap_or(line)
}

impl Render for UpgradePrompt {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let radius = surface_radius(cx);
        let bounds = self.bounds.get();
        let viewport = w.viewport_size();
        let measured = f32::from(bounds.size.width);
        let width = if measured > 0. {
            measured.min(f32::from(viewport.width))
        } else {
            f32::from(viewport.width).min(440.)
        };
        let padding = if width < 420. { 20. } else { 24. };
        let inner_width = (width - 2. * padding - 2.).max(1.);
        let stacked = inner_width < 340.;
        let header_height = [Panel::Offer, Panel::Success]
            .into_iter()
            .map(|panel| {
                let (title, description) = self.header_copy(panel, cx);
                let inner_width = px(inner_width);
                wrapped_height(title, inner_width, px(18.), px(20.), w).max(px(20.))
                    + px(4.)
                    + wrapped_height(description, inner_width, px(14.), px(20.), w).max(px(40.))
            })
            .fold(px(64.), |height, item| height.max(item));
        let visible = w.is_visible() && bounds.bottom() > px(0.) && bounds.top() < viewport.height;
        self.revealed |= visible;
        let limit = self.limit(self.panel);
        let ratio = if limit == 0 {
            1.
        } else {
            (self.options.used as f64 / limit as f64).min(1.) as f32
        };
        if self.revealed {
            self.meter.duration(ratio, 0.5, cx);
        }
        let meter = if visible {
            self.meter.get(w, cx)
        } else {
            self.meter.value(cx)
        };
        let elapsed = self.panel_clock.age(cx);
        if elapsed < 0.16 && visible {
            w.request_animation_frame();
        }
        let entering = ease(elapsed / 0.16);
        let leaving = 1. - ease(elapsed / 0.10);
        let travel = if disabled_motion(cx) { 0. } else { 1. };
        let header = self.header(self.panel, p, cx);
        let labels = self.meter_labels(self.panel, p, cx);
        let body = self.body(self.panel, stacked, p, cx);
        let old = self.previous_panel;
        let changed = old != self.panel;
        let header_changed = changed && (old == Panel::Success || self.panel == Panel::Success);
        let content_height = (if stacked { 188. } else { 136. }
            + (self.options.plan.features.len() as f32 - 3.) * 28.)
            .max((self.options.plan.limits.len() as f32 * 36. - 8.).max(0.) + 30.)
            .max(136.);
        let pending = self.pending.is_some();
        let action = match self.panel {
            Panel::Offer if pending => tr(cx, "Upgrading", "正在升级"),
            Panel::Offer if self.error.is_some() => tr(cx, "Try again", "重试"),
            Panel::Offer | Panel::Reminder => "",
            Panel::Success => tr(cx, "Continue", "继续"),
        };
        let action: SharedString = if self.panel == Panel::Reminder {
            localized(
                cx,
                format!("See {} again", self.options.plan.name),
                format!("再次查看 {}", self.options.plan.name),
            )
        } else if action.is_empty() {
            localized(
                cx,
                format!("Upgrade to {}", self.options.plan.name),
                format!("升级至 {}", self.options.plan.name),
            )
        } else {
            action.into()
        };
        let primary = self.panel != Panel::Reminder;
        let mut main = face(
            "upgrade-primary",
            action.clone(),
            44.,
            radius.into(),
            if primary {
                Tone::Primary
            } else {
                Tone::Surface
            },
            cx,
        )
        .track_focus(&self.primary_focus)
        .flex_1()
        .min_w_0()
        .px(px(8.))
        .border_1()
        .border_color(if primary { transparent_black() } else { p.line })
        .hover(move |d| {
            d.bg(if primary {
                p.primary_hover(0.1)
            } else {
                p.hover
            })
        })
        .when(pending, |b| {
            let rotation = if travel == 0. {
                0.
            } else {
                self.spinner_clock.age(cx)
            };
            if travel > 0. && visible {
                w.request_animation_frame();
            }
            b.cursor_default()
                .aria_description(tr(cx, "Upgrade in progress", "升级进行中"))
                .child(spinner(rotation, 14., p.inverse))
        })
        .child(div().min_w_0().truncate().child(action));
        if !pending {
            main = main.on_click(cx.listener(|s, _, w, cx| match s.panel {
                Panel::Offer => s.start(w, cx),
                Panel::Success => {
                    cx.emit(UpgradeEvent::Continued);
                    if s.payment.is_none() {
                        s.show(Panel::Offer, w, cx);
                    }
                }
                Panel::Reminder => s.show(Panel::Offer, w, cx),
            }));
        }
        let notice = if pending {
            if self.payment.is_none() {
                tr(
                    cx,
                    "Simulating an upgrade. No charge.",
                    "正在模拟升级，未扣款。",
                )
                .into()
            } else {
                tr(cx, "Upgrade in progress", "升级进行中").into()
            }
        } else if let Some(error) = &self.error {
            error.clone()
        } else {
            match self.panel {
                Panel::Offer => "".into(),
                Panel::Success if self.payment.is_none() => tr(
                    cx,
                    "Simulated upgrade complete. No charge.",
                    "模拟升级完成，未扣款。",
                )
                .into(),
                Panel::Success => tr(cx, "The new plan is active.", "新方案已生效。").into(),
                Panel::Reminder => tr(cx, "Upgrade reminder saved.", "已保存升级提醒。").into(),
            }
        };
        let later = tr(cx, "Maybe later", "稍后再说");
        let card_bounds = self.bounds.clone();
        let color = if self.options.used >= limit {
            rgb(if Theme::global(cx).mode == ThemeMode::Dark {
                0xe9b65f
            } else {
                0xb47728
            })
            .into()
        } else {
            p.secondary
        };
        column()
            .id("upgrade-prompt")
            .debug_selector(|| "upgrade-prompt".into())
            .role(Role::Region)
            .aria_label(localized(
                cx,
                format!("Upgrade to {}", self.options.plan.name),
                format!("升级至 {}", self.options.plan.name),
            ))
            .relative()
            .w(px(440.))
            .max_w_full()
            .min_w_0()
            .p(px(padding))
            .border_1()
            .border_color(p.line)
            .rounded(radius)
            .bg(p.panel)
            .text_color(p.ink)
            .text_size(px(14.))
            .line_height(px(20.))
            .child(
                canvas(
                    move |next, w, _| {
                        if card_bounds.replace(next) != next {
                            w.request_animation_frame();
                        }
                    },
                    |_, _, _, _| (),
                )
                .absolute()
                .inset_0(),
            )
            .child(
                div()
                    .relative()
                    .min_h(header_height)
                    .child(
                        header
                            .relative()
                            .top(px(if header_changed {
                                2. * (1. - entering) * travel
                            } else {
                                0.
                            }))
                            .opacity(if header_changed { entering } else { 1. }),
                    )
                    .when(header_changed && leaving > 0., |d| {
                        d.child(self.header(old, p, cx).absolute().w_full().opacity(leaving))
                    }),
            )
            .child(
                column()
                    .mt(px(20.))
                    .gap(px(8.))
                    .child(
                        div()
                            .relative()
                            .child(labels.opacity(if header_changed { entering } else { 1. }))
                            .when(header_changed && leaving > 0., |d| {
                                d.child(
                                    self.meter_labels(old, p, cx)
                                        .absolute()
                                        .inset_0()
                                        .opacity(leaving),
                                )
                            }),
                    )
                    .child(
                        div()
                            .id("upgrade-meter")
                            .role(Role::Meter)
                            .aria_numeric_value(self.options.used.min(limit) as f64)
                            .aria_min_numeric_value(0.)
                            .aria_max_numeric_value(limit as f64)
                            .aria_label(format!(
                                "{} / {} {}",
                                digits(self.options.used),
                                digits(limit),
                                self.options.unit
                            ))
                            .h(px(6.))
                            .w_full()
                            .rounded_full()
                            .overflow_hidden()
                            .bg(p.ink.opacity(0.08))
                            .child(
                                div()
                                    .h_full()
                                    .w(relative(meter.clamp(0., 1.)))
                                    .rounded_full()
                                    .bg(color),
                            ),
                    ),
            )
            .child(
                column()
                    .mt(px(20.))
                    .pt(px(20.))
                    .border_t_1()
                    .border_color(p.line.opacity(0.65))
                    .child(
                        div()
                            .relative()
                            .min_h(px(content_height))
                            .child(
                                body.relative()
                                    .top(px(4. * (1. - entering) * travel))
                                    .opacity(entering),
                            )
                            .when(changed && leaving > 0., |d| {
                                d.child(
                                    self.body(old, stacked, p, cx)
                                        .absolute()
                                        .w_full()
                                        .opacity(leaving),
                                )
                            }),
                    )
                    .child(
                        div()
                            .id("upgrade-status")
                            .role(Role::Status)
                            .aria_label(notice.clone())
                            .h(px(24.))
                            .flex()
                            .items_center()
                            .text_size(px(12.))
                            .children(self.error.as_ref().map(|e| {
                                div()
                                    .w_full()
                                    .truncate()
                                    .text_color(p.danger)
                                    .child(e.clone())
                            })),
                    )
                    .child(
                        row()
                            .gap(px(8.))
                            .when(self.panel == Panel::Offer, |d| {
                                d.child(
                                    face(
                                        "upgrade-later",
                                        later,
                                        44.,
                                        radius.into(),
                                        Tone::Surface,
                                        cx,
                                    )
                                    .track_focus(&self.later_focus)
                                    .tab_index(if pending { -1 } else { 0 })
                                    .px(px(16.))
                                    .border_1()
                                    .border_color(p.line)
                                    .hover(|d| d.bg(p.hover))
                                    .child(later)
                                    .when(!pending, |d| {
                                        d.on_click(cx.listener(|s, _, w, cx| {
                                            s.error = None;
                                            s.show(Panel::Reminder, w, cx);
                                            cx.emit(UpgradeEvent::Dismissed);
                                        }))
                                    }),
                                )
                            })
                            .child(main),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::{BillingPeriod, Panel, UpgradePrompt, UpgradePromptOptions};
    use gpui::prelude::{ParentElement, Styled};
    use gpui::{
        AppContext, Context, Entity, IntoElement, Render, Task, TestAppContext, Window, div, px,
        size,
    };
    use std::{cell::Cell, rc::Rc, time::Duration};

    struct Page(Entity<UpgradePrompt>);

    impl Render for Page {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .items_start()
                .justify_center()
                .child(self.0.clone())
        }
    }

    #[test]
    fn charges_the_selected_period_and_keeps_the_footer_in_place() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (page, window) = app.add_window_view(|_, cx| {
            Page(cx.new(|cx| UpgradePrompt::new(UpgradePromptOptions::default(), cx)))
        });
        let control = window.update(|_, cx| page.read(cx).0.clone());
        window.simulate_resize(size(px(440.), px(650.)));
        window.run_until_parked();
        let first = window.debug_bounds("upgrade-prompt").unwrap();
        assert_eq!(first.size, size(px(440.), px(415.)));
        window.update(|w, cx| {
            control.update(cx, |control, cx| {
                assert_eq!(control.selection().amount, 19_200);
                control.set_interval(BillingPeriod::Monthly, cx);
                assert_eq!(control.selection().amount, 2000);
                control.show(Panel::Reminder, w, cx);
            })
        });
        window.run_until_parked();
        assert_eq!(window.debug_bounds("upgrade-prompt").unwrap(), first);
        window.update(|w, cx| {
            control.update(cx, |control, cx| {
                control.show(Panel::Offer, w, cx);
                control.start(w, cx);
                control.start(w, cx);
            })
        });
        window.run_until_parked();
        window.executor().advance_clock(Duration::from_millis(1099));
        window.run_until_parked();
        assert!(window.update(|_, cx| control.read(cx).pending.is_some()));
        window.executor().advance_clock(Duration::from_millis(1));
        window.run_until_parked();
        assert!(window.update(|w, cx| {
            let prompt = control.read(cx);
            prompt.panel == Panel::Success
                && prompt.receipt.as_ref().is_some_and(|r| r.amount == 2000)
                && prompt.primary_focus.is_focused(w)
        }));
        assert_eq!(window.debug_bounds("upgrade-prompt").unwrap(), first);
    }

    #[test]
    fn failure_retains_the_offer_and_retry_snapshots_the_same_selection() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let calls = Rc::new(Cell::new(0));
        let started = calls.clone();
        let (page, window) = app.add_window_view(|_, cx| {
            Page(cx.new(|cx| {
                UpgradePrompt::new(UpgradePromptOptions::default(), cx).with_upgrade(
                    move |selection, _| {
                        assert_eq!(selection.interval, BillingPeriod::Yearly);
                        assert_eq!(selection.amount, 19_200);
                        let n = started.get();
                        started.set(n + 1);
                        Task::ready(if n == 0 {
                            Err("Try another card.".into())
                        } else {
                            Ok(())
                        })
                    },
                )
            }))
        });
        let control = window.update(|_, cx| page.read(cx).0.clone());
        window.simulate_resize(size(px(320.), px(650.)));
        window.run_until_parked();
        let narrow = window.debug_bounds("upgrade-prompt").unwrap();
        window.update(|w, cx| {
            control.update(cx, |prompt, cx| {
                prompt.start(w, cx);
                prompt.start(w, cx);
            })
        });
        window.run_until_parked();
        assert_eq!(calls.get(), 1);
        assert!(window.update(|_, cx| {
            let prompt = control.read(cx);
            prompt.panel == Panel::Offer && prompt.error.as_deref() == Some("Try another card.")
        }));
        assert_eq!(window.debug_bounds("upgrade-prompt").unwrap(), narrow);
        window.update(|w, cx| control.update(cx, |prompt, cx| prompt.start(w, cx)));
        window.run_until_parked();
        assert_eq!(calls.get(), 2);
        assert!(window.update(|_, cx| control.read(cx).panel == Panel::Success));
        assert_eq!(window.debug_bounds("upgrade-prompt").unwrap(), narrow);
    }
}
