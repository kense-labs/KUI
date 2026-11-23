//! Onboarding wizard final step summarizing assistant knowledge, voice, and live readiness.
//!
//! Displays a four-row description list of configured domains, assistant identity,
//! voice style, and handover target with per-row `Edit` actions while the right-hand
//! dashboard glides to its top and transitions from `Draft` into `Live` status.

use crate::{
    components::{
        app_shell::AppShellLayout,
        pro_chrome::{
            ProEmptyCta, ProPalette, pro_ease, pro_eid, pro_empty_state, pro_fold,
            pro_mark_selected, pro_pill_indicator, pro_selection_sensor, pro_skeleton_bar,
            pro_spring_bar, pro_toast_banner, pro_view_transition, tr,
        },
    },
    foundation::{Theme, ThemeMode},
    primitives::{
        control::activate,
        pro_icons::{ProIcon, agent_pixel_cells, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

const LUMEN_MARK_SVG: &str = r#"<rect x="3" y="3" width="11" height="11" rx="2.6" fill="currentColor"></rect><path d="M16.5 9H18.4C19.8359 9 21 10.1641 21 11.6V18.4C21 19.8359 19.8359 21 18.4 21H11.6C10.1641 21 9 19.8359 9 18.4V16.5H14C15.3807 16.5 16.5 15.3807 16.5 14V9Z" fill="currentColor"></path>"#;

const LOGOUT_SVG: &str = r#"<path d="M18 18C18 18.4644 18 18.6965 17.978 18.8918C17.7952 20.5145 16.5145 21.7952 14.8919 21.978C14.6965 22 14.4644 22 14 22H11C7.70017 22 6.05025 22 5.02513 20.9749C4 19.9498 4 18.2998 4 15L4.00001 8.99998C4.00001 5.70016 4.00003 4.05024 5.02514 3.02512C6.05027 2 7.70018 2 11 2H14C14.4644 2 14.6965 2 14.8919 2.02201C16.5145 2.20483 17.7952 3.48547 17.978 5.10816C18 5.30347 18 5.53565 18 6" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.6"></path><path d="M8.07612 11.1183C8 11.3021 8 11.535 8 12.001C8 12.4669 8 12.6999 8.07612 12.8837C8.17761 13.1287 8.37229 13.3234 8.61732 13.4249C8.80109 13.501 9.03406 13.501 9.5 13.501H14.5C14.5002 15.2503 14.511 16.1299 15.0623 16.3858C15.0829 16.3954 15.1037 16.4042 15.1249 16.4124C15.7045 16.6353 16.3999 16.0139 17.7907 14.7711C19.2576 13.4602 19.9912 12.7851 20 11.957C19.9912 11.1289 19.2576 10.4538 17.7907 9.14301C16.3999 7.90018 15.7045 7.27876 15.1249 7.50171C15.1037 7.50985 15.0829 7.51869 15.0623 7.52822C14.5018 7.78844 14.5 8.69318 14.5 10.501H9.5C9.03406 10.501 8.80109 10.501 8.61732 10.5771C8.37229 10.6786 8.17761 10.8733 8.07612 11.1183Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.6"></path>"#;

const HELP_CIRCLE_SVG: &str = r#"<circle cx="12" cy="12" r="10" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.6"></circle><path d="M9.5 9.5C9.5 8.11929 10.6193 7 12 7C13.3807 7 14.5 8.11929 14.5 9.5C14.5 10.3569 14.0689 11.1131 13.4117 11.5636C12.7283 12.0319 12 12.6716 12 13.5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.6"></path><path d="M12.125 16.75H12M12.25 16.75C12.25 16.8881 12.1381 17 12 17C11.8619 17 11.75 16.8881 11.75 16.75C11.75 16.6119 11.8619 16.5 12 16.5C12.1381 16.5 12.25 16.6119 12.25 16.75Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.6"></path>"#;

const CHART_BAR_PCTS: [f32; 28] = [
    0.36, 0.45, 0.40, 0.54, 0.49, 0.63, 0.56, 0.47, 0.67, 0.74, 0.65, 0.78, 0.72, 0.58, 0.67, 0.80,
    0.87, 0.78, 0.69, 0.83, 0.92, 0.85, 0.76, 0.89, 0.96, 0.87, 1.00, 0.94,
];

const SIDE_NAV_GROUPS: [&[f32]; 7] = [
    &[75., 67.],
    &[59., 75.],
    &[80., 72.],
    &[56., 64.],
    &[83., 69., 59.],
    &[72., 64.],
    &[80., 56., 67.],
];

/// Conversational tone summarized on [`OnboardingReadyScreen`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OnboardingReadyTone {
    Friendly,
    #[default]
    Neutral,
    Formal,
}

impl OnboardingReadyTone {
    fn capitalized_en(self) -> &'static str {
        match self {
            Self::Friendly => "Friendly",
            Self::Neutral => "Neutral",
            Self::Formal => "Formal",
        }
    }

    fn label_zh(self) -> &'static str {
        match self {
            Self::Friendly => "友好",
            Self::Neutral => "中性",
            Self::Formal => "正式",
        }
    }
}

/// Reply length summarized on [`OnboardingReadyScreen`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OnboardingReadyLength {
    Short,
    #[default]
    Balanced,
    Detailed,
}

impl OnboardingReadyLength {
    fn lower_en(self) -> &'static str {
        match self {
            Self::Short => "short",
            Self::Balanced => "balanced",
            Self::Detailed => "detailed",
        }
    }

    fn label_zh(self) -> &'static str {
        match self {
            Self::Short => "简短",
            Self::Balanced => "适中",
            Self::Detailed => "详尽",
        }
    }
}

/// Earlier wizard step targeted by an `Edit` row button on [`OnboardingReadyScreen`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum OnboardingEditStep {
    /// Step 1: website and knowledge domains (`Learns from`).
    Website = 1,
    /// Step 2: company brand and short forms (`Name`).
    Brand = 2,
    /// Step 3: assistant voice, length, and escalation (`Voice` / `Hands over to`).
    Tone = 3,
}

impl OnboardingEditStep {
    /// Numeric 1-based step index (`1`, `2`, or `3`).
    pub fn step_number(self) -> u8 {
        self as u8
    }
}

/// Events emitted by [`OnboardingReadyScreen`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OnboardingReadyEvent {
    /// User pressed `Open dashboard` to finish setup and enter the product.
    Finish,
    /// User pressed `Edit` on a summary row to revisit an earlier step.
    Edit(OnboardingEditStep),
    /// User pressed `Back` in the top header.
    Back,
    /// User pressed `Log out` in the top header.
    Logout,
    /// User activated a footer policy link (`"terms"` or `"privacy"`).
    OpenLegal(SharedString),
}

/// Format the English voice summary phrase (`"Neutral, balanced replies"`).
pub fn format_voice_summary(tone: OnboardingReadyTone, length: OnboardingReadyLength) -> String {
    format!("{}, {} replies", tone.capitalized_en(), length.lower_en())
}

/// Format the handover summary value, falling back to `"Nobody – it answers itself"` when unset.
pub fn format_handover_summary(handover_to: Option<&str>) -> String {
    match handover_to.map(str::trim).filter(|s| !s.is_empty()) {
        Some(addr) => addr.to_owned(),
        None => "Nobody – it answers itself".to_owned(),
    }
}

fn brand_from_domain(domain: &str) -> String {
    let trimmed = domain.trim();
    let without_scheme = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))
        .unwrap_or(trimmed);
    let host = without_scheme
        .split(['/', '?', '#', ':'])
        .next()
        .unwrap_or("")
        .trim_matches('.');
    let labels: Vec<&str> = host.split('.').filter(|s| !s.is_empty()).collect();
    let chosen = match labels.as_slice() {
        [] => "",
        [only] => *only,
        [first, second, ..] => {
            let lower = first.to_ascii_lowercase();
            if labels.len() >= 3
                && matches!(
                    lower.as_str(),
                    "www" | "docs" | "help" | "support" | "app" | "api" | "blog"
                )
            {
                *second
            } else {
                *first
            }
        }
    };
    let words: Vec<String> = chosen
        .split(['-', '_'])
        .filter(|part| part.chars().any(char::is_alphanumeric))
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => {
                    let mut s: String = first.to_uppercase().collect();
                    s.push_str(&chars.as_str().to_lowercase());
                    s
                }
                None => String::new(),
            }
        })
        .filter(|w| !w.is_empty())
        .collect();
    if words.is_empty() {
        "Northwind".to_string()
    } else {
        words.join(" ")
    }
}

#[derive(Clone, Copy)]
struct ObwTokens {
    bg: Hsla,
    ink: Hsla,
    muted: Hsla,
    faint: Hsla,
    line: Hsla,
    line_firm: Hsla,
    card: Hsla,
    sk: Hsla,
    side: Hsla,
    page: Hsla,
    dark_fill: Hsla,
    on_dark: Hsla,
    blue: Hsla,
    green_dot: Hsla,
    green_soft: Hsla,
    green_ink: Hsla,
    mark: Hsla,
    focus: Hsla,
}

impl ObwTokens {
    fn resolve(cx: &App) -> Self {
        let dark = Theme::global(cx).mode == ThemeMode::Dark;
        let pro = ProPalette::get(cx);
        if dark {
            Self {
                bg: rgb(0x0f0f10).into(),
                ink: hsla(0., 0., 1., 0.92),
                muted: hsla(0., 0., 1., 0.60),
                faint: hsla(0., 0., 1., 0.42),
                line: hsla(0., 0., 1., 0.07),
                line_firm: hsla(0., 0., 1., 0.18),
                card: rgb(0x18181a).into(),
                sk: hsla(0., 0., 1., 0.06),
                side: hsla(0., 0., 1., 0.016),
                page: rgb(0x0e0e10).into(),
                dark_fill: hsla(0., 0., 1., 0.92),
                on_dark: rgb(0x111111).into(),
                blue: pro.blue,
                green_dot: rgb(0x5fc88a).into(),
                green_soft: hsla(145. / 360., 0.48, 0.58, 0.14),
                green_ink: rgb(0x7dcf9f).into(),
                mark: rgba(0x4d8dffda).into(),
                focus: pro.focus,
            }
        } else {
            Self {
                bg: rgb(0xffffff).into(),
                ink: hsla(0., 0., 0., 0.88),
                muted: hsla(0., 0., 0., 0.61),
                faint: hsla(0., 0., 0., 0.45),
                line: hsla(0., 0., 0., 0.06),
                line_firm: hsla(0., 0., 0., 0.14),
                card: rgb(0xffffff).into(),
                sk: hsla(0., 0., 0., 0.05),
                side: hsla(0., 0., 0., 0.018),
                page: rgb(0xf8f8f8).into(),
                dark_fill: hsla(0., 0., 0., 0.88),
                on_dark: rgb(0xfcfcfc).into(),
                blue: pro.blue,
                green_dot: rgb(0x5fc88a).into(),
                green_soft: hsla(145. / 360., 0.48, 0.58, 0.14),
                green_ink: rgb(0x1e8a4f).into(),
                mark: rgba(0x0060e2c7).into(),
                focus: pro.focus,
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum StarterPromptKind {
    #[default]
    Support,
    Pricing,
    Docs,
}

impl StarterPromptKind {
    const ALL: [Self; 3] = [Self::Support, Self::Pricing, Self::Docs];

    fn as_str(self) -> &'static str {
        match self {
            Self::Support => "support",
            Self::Pricing => "pricing",
            Self::Docs => "docs",
        }
    }

    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::Support => tr(cx, "Support", "客服"),
            Self::Pricing => tr(cx, "Pricing", "定价"),
            Self::Docs => tr(cx, "Docs", "文档"),
        }
    }

    fn description<'a>(self, cx: &App) -> &'a str {
        match self {
            Self::Support => tr(
                cx,
                "Greets visitors and resolves account & billing questions.",
                "主动问候访客并解答账户与账单咨询。",
            ),
            Self::Pricing => tr(
                cx,
                "Compares subscription tiers, seat bands, and proration.",
                "对比订阅套餐、席位档位及按比例计费规则。",
            ),
            Self::Docs => tr(
                cx,
                "Guides builders to quickstarts, SDKs, and API references.",
                "引导开发者快速定位入门指南、SDK 与 API 文档。",
            ),
        }
    }

    fn greeting(self, assistant_name: &str) -> String {
        match self {
            Self::Support => format!(
                "Hi, I'm {assistant_name} – ask me anything about our products, plans or your account."
            ),
            Self::Pricing => format!(
                "Welcome! I'm {assistant_name} – compare plans, seats, or ask for an enterprise quote."
            ),
            Self::Docs => format!(
                "Hey! I'm {assistant_name} – tell me what you're building and I'll link the exact docs."
            ),
        }
    }
}

/// Step 4 of 4 in the AI assistant onboarding wizard: completion summary and live launch.
pub struct OnboardingReadyScreen {
    id: ElementId,
    domains: Vec<SharedString>,
    brand: Option<SharedString>,
    aliases: Vec<SharedString>,
    name: Option<SharedString>,
    tone: OnboardingReadyTone,
    length: OnboardingReadyLength,
    handover_to: Option<SharedString>,
    show_edit: bool,
    show_back: bool,
    email: SharedString,
    product: SharedString,
    layout: AppShellLayout,
    step: usize,
    steps: usize,
    auto_focus: bool,
    did_auto_focus: bool,
    loading: bool,
    empty: bool,
    expanded_row: Option<usize>,
    verified_checks: [bool; 4],
    starter_prompt: StarterPromptKind,
    preview_tab: usize,
    toast: Option<(SharedString, Instant)>,
    focus: FocusHandle,
    cta_focus: FocusHandle,
    measured_width: Rc<Cell<f32>>,
    opened_at: Option<Instant>,
}

impl EventEmitter<OnboardingReadyEvent> for OnboardingReadyScreen {}

impl OnboardingReadyScreen {
    /// Construct the Ready onboarding step with `northwind.com` and `Northwind Assistant` defaults.
    pub fn new(id: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        Self {
            id: ElementId::Name(id.into()),
            domains: vec!["northwind.com".into()],
            brand: None,
            aliases: vec!["Northwind Co.".into()],
            name: None,
            tone: OnboardingReadyTone::Neutral,
            length: OnboardingReadyLength::Balanced,
            handover_to: Some("support@northwind.com".into()),
            show_edit: true,
            show_back: true,
            email: "alex@northwind.com".into(),
            product: "Lumen".into(),
            layout: AppShellLayout::Inset,
            step: 4,
            steps: 4,
            auto_focus: false,
            did_auto_focus: false,
            loading: false,
            empty: false,
            expanded_row: None,
            verified_checks: [true, true, true, true],
            starter_prompt: StarterPromptKind::Support,
            preview_tab: 0,
            toast: None,
            focus: cx.focus_handle(),
            cta_focus: cx.focus_handle().tab_stop(true),
            measured_width: Rc::new(Cell::new(1280.)),
            opened_at: None,
        }
    }

    /// Set the learned website and documentation domains from Step 1.
    pub fn with_domains<I, S>(mut self, domains: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        self.domains = domains
            .into_iter()
            .map(Into::into)
            .filter(|s| !s.trim().is_empty())
            .collect();
        self
    }

    /// Set the company brand name from Step 2.
    pub fn with_brand(mut self, brand: impl Into<SharedString>) -> Self {
        let b: SharedString = brand.into();
        self.brand = (!b.trim().is_empty()).then_some(b);
        self
    }

    /// Set the company short forms from Step 2 displayed on the live assistant card.
    pub fn with_aliases<I, S>(mut self, aliases: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        self.aliases = aliases
            .into_iter()
            .map(Into::into)
            .filter(|s| !s.trim().is_empty())
            .collect();
        self
    }

    /// Set the assistant's configured display name from Step 3.
    pub fn with_name(mut self, name: impl Into<SharedString>) -> Self {
        let n: SharedString = name.into();
        self.name = (!n.trim().is_empty()).then_some(n);
        self
    }

    /// Set the assistant's configured conversational tone from Step 3.
    pub fn with_tone(mut self, tone: OnboardingReadyTone) -> Self {
        self.tone = tone;
        self
    }

    /// Set the assistant's configured reply length from Step 3.
    pub fn with_length(mut self, length: OnboardingReadyLength) -> Self {
        self.length = length;
        self
    }

    /// Set the escalation target email; pass `None` or an empty string when the
    /// assistant answers everything itself.
    pub fn with_handover_to(mut self, handover_to: Option<SharedString>) -> Self {
        self.handover_to = handover_to.filter(|s| !s.trim().is_empty());
        self
    }

    /// Show or hide the `Edit` action button on every summary row.
    pub fn with_show_edit(mut self, show: bool) -> Self {
        self.show_edit = show;
        self
    }

    /// Show or hide the `Back` button in the top bar.
    pub fn with_show_back(mut self, show: bool) -> Self {
        self.show_back = show;
        self
    }

    /// Set the signed-in account email displayed in the top-right header.
    pub fn with_email(mut self, email: impl Into<SharedString>) -> Self {
        self.email = email.into();
        self
    }

    /// Set the product brand name used in the header mark label and copyright footer.
    pub fn with_product(mut self, product: impl Into<SharedString>) -> Self {
        self.product = product.into();
        self
    }

    /// Configure the framing style (`Inset` or `Flat`) of the decorative dashboard preview.
    pub fn with_layout(mut self, layout: AppShellLayout) -> Self {
        self.layout = layout;
        self
    }

    /// Set the 1-based step index and total step count shown in the progress dashes.
    pub fn with_step(mut self, step: usize, steps: usize) -> Self {
        self.step = step.max(1);
        self.steps = steps.max(self.step);
        self
    }

    /// Focus the `Open dashboard` button automatically when the screen opens.
    pub fn with_auto_focus(mut self, auto_focus: bool) -> Self {
        self.auto_focus = auto_focus;
        self
    }

    /// Render skeleton placeholders in place of the summary list.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Render an empty state when no onboarding sources have been configured.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Configured knowledge domains.
    pub fn domains(&self) -> &[SharedString] {
        &self.domains
    }

    /// Resolved company brand name (`brand` prop or derived from first domain).
    pub fn resolved_brand(&self) -> String {
        self.brand
            .as_ref()
            .map(|b| b.trim().to_owned())
            .filter(|b| !b.is_empty())
            .unwrap_or_else(|| {
                let first = self
                    .domains
                    .first()
                    .map(|d| d.as_ref())
                    .unwrap_or("northwind.com");
                brand_from_domain(first)
            })
    }

    /// Resolved assistant display name (`name` prop or `"<brand> Assistant"`).
    pub fn assistant_name(&self) -> String {
        self.name
            .as_ref()
            .map(|n| n.trim().to_owned())
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| format!("{} Assistant", self.resolved_brand()))
    }

    /// Localized summary text for the `Voice` row.
    pub fn voice_summary(&self, cx: &App) -> String {
        let en = format_voice_summary(self.tone, self.length);
        let zh = format!("{}，{}回复", self.tone.label_zh(), self.length.label_zh());
        if tr(cx, "en", "zh") == "zh" { zh } else { en }
    }

    /// Localized summary text for the `Hands over to` row.
    pub fn handover_summary(&self, cx: &App) -> String {
        match self
            .handover_to
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            Some(addr) => addr.to_owned(),
            None => tr(cx, "Nobody – it answers itself", "无——由助手独立回答").to_owned(),
        }
    }

    /// Emit [`OnboardingReadyEvent::Finish`] (`Open dashboard`).
    pub fn finish(&mut self, cx: &mut Context<Self>) {
        cx.emit(OnboardingReadyEvent::Finish);
        cx.notify();
    }

    /// Emit [`OnboardingReadyEvent::Edit`] for `step`.
    pub fn edit_step(&mut self, step: OnboardingEditStep, cx: &mut Context<Self>) {
        cx.emit(OnboardingReadyEvent::Edit(step));
        cx.notify();
    }

    /// Whether skeleton loading placeholders are active.
    pub fn is_loading(&self) -> bool {
        self.loading
    }

    /// Toggle skeleton loading placeholders at runtime.
    pub fn set_loading(&mut self, loading: bool, cx: &mut Context<Self>) {
        if self.loading != loading {
            self.loading = loading;
            cx.notify();
        }
    }

    /// Whether the empty state is active.
    pub fn is_empty(&self) -> bool {
        self.empty
    }

    /// Toggle the empty state at runtime.
    pub fn set_empty(&mut self, empty: bool, cx: &mut Context<Self>) {
        if self.empty != empty {
            self.empty = empty;
            cx.notify();
        }
    }

    fn render_top_bar(&self, narrow: bool, pal: ObwTokens, cx: &mut Context<Self>) -> Div {
        let mut bar = div()
            .h(px(56.))
            .flex_none()
            .pl(px(if narrow { 16. } else { 24. }))
            .pr(px(if narrow { 16. } else { 20. }))
            .flex()
            .items_center()
            .gap(px(12.))
            .child(
                div()
                    .mr(px(4.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(pro_svg("0 0 24 24", LUMEN_MARK_SVG, 20., pal.dark_fill)),
            );

        if self.show_back {
            let back_lbl: SharedString = tr(cx, "Back", "返回").into();
            bar = bar.child(activate(
                div()
                    .id(pro_eid(&self.id, "back"))
                    .role(Role::Button)
                    .aria_label(back_lbl.clone())
                    .tab_index(0)
                    .h(px(28.))
                    .pl(px(8.))
                    .pr(px(10.))
                    .rounded(px(6.))
                    .bg(pal.card)
                    .border_1()
                    .border_color(pal.line)
                    .hover(move |s| s.border_color(pal.line_firm))
                    .focus_visible(move |s| s.border_color(pal.focus))
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .text_size(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(pal.ink)
                    .cursor_pointer()
                    .child(pro_icon(ProIcon::ArrowLeft, 14., pal.faint))
                    .child(back_lbl),
                cx,
                |_, _, cx| cx.emit(OnboardingReadyEvent::Back),
            ));
        }

        if !narrow {
            bar = bar.child(
                div()
                    .ml_auto()
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .text_size(px(12.))
                    .line_height(px(16.))
                    .text_color(pal.faint)
                    .child(tr(cx, "Logged in as", "当前登录"))
                    .child(div().text_color(pal.muted).child(self.email.clone())),
            );
        }

        let logout_lbl: SharedString = tr(cx, "Log out", "退出登录").into();
        bar.child(activate(
            div()
                .id(pro_eid(&self.id, "logout"))
                .role(Role::Button)
                .aria_label(logout_lbl.clone())
                .tab_index(0)
                .when(narrow, |d| d.ml_auto())
                .h(px(28.))
                .pl(px(8.))
                .pr(px(10.))
                .rounded(px(6.))
                .bg(pal.card)
                .border_1()
                .border_color(pal.line)
                .hover(move |s| s.border_color(pal.line_firm))
                .focus_visible(move |s| s.border_color(pal.focus))
                .flex()
                .items_center()
                .gap(px(6.))
                .text_size(px(13.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(pal.ink)
                .cursor_pointer()
                .child(pro_svg("0 0 24 24", LOGOUT_SVG, 14., pal.faint))
                .child(logout_lbl),
            cx,
            |_, _, cx| cx.emit(OnboardingReadyEvent::Logout),
        ))
    }

    fn render_footer(&self, narrow: bool, pal: ObwTokens, cx: &mut Context<Self>) -> Div {
        let terms_lbl: SharedString = tr(cx, "Terms", "条款").into();
        let priv_lbl: SharedString = tr(cx, "Privacy", "隐私").into();
        div()
            .h(px(48.))
            .flex_none()
            .px(px(if narrow { 20. } else { 24. }))
            .flex()
            .items_center()
            .gap(px(16.))
            .text_size(px(12.))
            .line_height(px(16.))
            .text_color(pal.faint)
            .child(format!("© 2026 {}", self.product))
            .child(activate(
                div()
                    .id(pro_eid(&self.id, "terms"))
                    .role(Role::Button)
                    .tab_index(0)
                    .cursor_pointer()
                    .hover(move |s| s.text_color(pal.muted))
                    .focus_visible(move |s| s.text_color(pal.ink))
                    .child(terms_lbl),
                cx,
                |_, _, cx| cx.emit(OnboardingReadyEvent::OpenLegal("terms".into())),
            ))
            .child(activate(
                div()
                    .id(pro_eid(&self.id, "privacy"))
                    .role(Role::Button)
                    .tab_index(0)
                    .cursor_pointer()
                    .hover(move |s| s.text_color(pal.muted))
                    .focus_visible(move |s| s.text_color(pal.ink))
                    .child(priv_lbl),
                cx,
                |_, _, cx| cx.emit(OnboardingReadyEvent::OpenLegal("privacy".into())),
            ))
    }

    fn render_dashboard_preview(
        &self,
        preview_w: f32,
        compact_side: bool,
        offset_y: f32,
        pal: ObwTokens,
        cx: &mut Context<Self>,
    ) -> Div {
        let is_flat = self.layout == AppShellLayout::Flat;
        let brand = self.resolved_brand();
        let initial: String = brand
            .chars()
            .find(|c| c.is_alphanumeric())
            .unwrap_or('N')
            .to_uppercase()
            .collect();
        let assistant_name = self.assistant_name();
        let source_count = self.domains.len().max(1);
        let source_text = if tr(cx, "en", "zh") == "zh" {
            format!("{source_count} 个来源")
        } else if source_count == 1 {
            "1 source".to_owned()
        } else {
            format!("{source_count} sources")
        };

        // Sidebar (.ui-obw-side)
        let ws_row = div()
            .h(px(32.))
            .when(!compact_side, |d| d.mx(px(-4.)).px(px(8.)).gap(px(8.)))
            .when(compact_side, |d| d.justify_center())
            .rounded(px(8.))
            .flex()
            .items_center()
            .child(
                div()
                    .size(px(16.))
                    .flex_none()
                    .rounded(px(4.))
                    .bg(pal.dark_fill)
                    .text_color(pal.on_dark)
                    .text_size(px(10.))
                    .font_weight(FontWeight::MEDIUM)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(initial),
            )
            .when(!compact_side, |d| {
                d.child(
                    div()
                        .min_w_0()
                        .truncate()
                        .text_size(px(13.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(pal.ink)
                        .child(brand),
                )
                .child(
                    div()
                        .size(px(4.))
                        .flex_none()
                        .rounded_full()
                        .bg(pal.faint.opacity(0.6)),
                )
            });

        let mut side = div()
            .w(px(if compact_side { 52. } else { 232. }))
            .flex_none()
            .py(px(10.))
            .px(px(if compact_side { 8. } else { 12. }))
            .when(is_flat, |d| {
                d.bg(pal.side).border_r_1().border_color(pal.line)
            })
            .flex()
            .flex_col()
            .gap(px(22.))
            .child(ws_row);

        for group in SIDE_NAV_GROUPS {
            let mut nav = div()
                .flex()
                .flex_col()
                .gap(px(12.))
                .when(!compact_side, |d| d.px(px(4.)))
                .when(compact_side, |d| d.items_center());
            if !compact_side {
                nav = nav.child(sk_bar(30., 6., pal.sk));
            }
            for &w_bar in group {
                nav = nav.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(10.))
                        .child(sk_sq(10., 3., pal.sk))
                        .when(!compact_side, |d| d.child(sk_bar(w_bar, 8., pal.sk))),
                );
            }
            side = side.child(nav);
        }

        // Live populated tabs (.ui-obw-tabs)
        let tab_items = [
            tr(cx, "Overview", "概览"),
            tr(cx, "Conversations", "对话"),
            tr(cx, "Knowledge", "知识库"),
            tr(cx, "Settings", "设置"),
        ];
        let pv_tab_sensor = pro_selection_sensor();
        let tabs_bar = div()
            .relative()
            .px(px(16.))
            .py(px(10.))
            .border_b_1()
            .border_color(pal.line)
            .flex()
            .items_center()
            .gap(px(8.))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "ready-pv-tabs-pill"),
                format!("tab-{}", self.preview_tab),
                pv_tab_sensor.clone(),
                7.,
                pal.side,
                Some(pal.line_firm),
                false,
            ))
            .children(tab_items.into_iter().enumerate().map(|(idx, lbl)| {
                let active = self.preview_tab == idx;
                activate(
                    div()
                        .id(pro_eid(&self.id, format!("ready-pv-tab-{idx}")))
                        .role(Role::Tab)
                        .tab_index(0)
                        .relative()
                        .h(px(28.))
                        .pl(px(10.))
                        .pr(px(12.))
                        .rounded(px(7.))
                        .when(!active, |d| {
                            d.border_1()
                                .border_color(pal.line)
                                .hover(move |s| s.border_color(pal.line_firm))
                        })
                        .cursor_pointer()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .when(active, |d| d.child(pro_mark_selected(&pv_tab_sensor)))
                        .child(sk_sq(10., 3., pal.sk))
                        .child(
                            div()
                                .text_size(px(12.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(if active { pal.ink } else { pal.muted })
                                .child(lbl),
                        ),
                    cx,
                    move |this, _, cx| {
                        this.preview_tab = idx;
                        cx.notify();
                    },
                )
            }));

        // Live populated KPIs (.ui-obw-kpis)
        let kpis_row = div()
            .flex()
            .gap(px(12.))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .h(px(82.))
                    .p(px(16.))
                    .rounded(px(10.))
                    .border_1()
                    .border_color(pal.line)
                    .flex()
                    .flex_col()
                    .gap(px(14.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .line_height(px(12.))
                            .text_color(pal.muted)
                            .child(tr(cx, "Knowledge", "知识库")),
                    )
                    .child(
                        div()
                            .text_size(px(18.))
                            .line_height(px(18.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(pal.ink)
                            .child(source_text),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .h(px(82.))
                    .p(px(16.))
                    .rounded(px(10.))
                    .border_1()
                    .border_color(pal.line)
                    .flex()
                    .flex_col()
                    .gap(px(14.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .line_height(px(12.))
                            .text_color(pal.muted)
                            .child(tr(cx, "Conversations", "对话量")),
                    )
                    .child(
                        div()
                            .text_size(px(18.))
                            .line_height(px(18.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(pal.ink)
                            .child("0"),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .h(px(82.))
                    .p(px(16.))
                    .rounded(px(10.))
                    .border_1()
                    .border_color(pal.line)
                    .flex()
                    .flex_col()
                    .gap(px(14.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .line_height(px(12.))
                            .text_color(pal.muted)
                            .child(tr(cx, "Status", "运行状态")),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .text_size(px(18.))
                            .line_height(px(18.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(pal.ink)
                            .child(div().size(px(7.)).rounded_full().bg(pal.green_dot))
                            .child(tr(cx, "Live", "已上线")),
                    ),
            );

        // Live assistant card (.ui-obw-agent) with green "Live" badge
        let aka_row = if !self.aliases.is_empty() {
            let mut row = div().h(px(20.)).flex().items_center().gap(px(6.)).child(
                div()
                    .mr(px(2.))
                    .text_size(px(12.))
                    .text_color(pal.faint)
                    .child(tr(cx, "Also answers to", "同时响应简称")),
            );
            for alias in self.aliases.iter().take(3) {
                row = row.child(
                    div()
                        .max_w(px(140.))
                        .h(px(18.))
                        .px(px(6.))
                        .rounded(px(5.))
                        .bg(pal.blue.opacity(0.10))
                        .text_color(pal.blue)
                        .text_size(px(11.))
                        .font_weight(FontWeight::MEDIUM)
                        .flex()
                        .items_center()
                        .truncate()
                        .child(alias.clone()),
                );
            }
            row
        } else {
            div()
                .h(px(20.))
                .flex()
                .items_center()
                .gap(px(6.))
                .child(sk_bar(88., 8., pal.sk))
                .child(sk_sq_rect(56., 16., 5., pal.sk))
        };

        let agent_card = div()
            .h(px(168.))
            .p(px(16.))
            .rounded(px(10.))
            .border_1()
            .border_color(pal.line)
            .flex()
            .flex_col()
            .gap(px(14.))
            .child(
                div()
                    .h(px(28.))
                    .max_w(px(420.))
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .child(obw_agent_mark(&assistant_name, pal.mark))
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(pal.ink)
                            .child(assistant_name.clone()),
                    )
                    .child(
                        div()
                            .ml(px(2.))
                            .h(px(18.))
                            .px(px(6.))
                            .rounded(px(5.))
                            .bg(pal.green_soft)
                            .text_color(pal.green_ink)
                            .text_size(px(11.))
                            .font_weight(FontWeight::MEDIUM)
                            .flex()
                            .items_center()
                            .gap(px(5.))
                            .child(div().size(px(5.)).rounded_full().bg(pal.green_ink))
                            .child(tr(cx, "Live", "已上线")),
                    ),
            )
            .child(pro_view_transition(
                pro_eid(&self.id, "ready-greet-tx"),
                self.starter_prompt.as_str(),
                div()
                    .max_w(px(420.))
                    .min_h(px(52.))
                    .px(px(14.))
                    .py(px(10.))
                    .rounded_tl(px(4.))
                    .rounded_tr(px(12.))
                    .rounded_br(px(12.))
                    .rounded_bl(px(12.))
                    .bg(pal.side)
                    .border_1()
                    .border_color(pal.line)
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .text_size(px(13.))
                            .line_height(px(20.))
                            .text_color(pal.ink)
                            .child(self.starter_prompt.greeting(&assistant_name)),
                    ),
            ))
            .child(aka_row);

        let knowledge_card = render_knowledge_table(&self.domains, pal, cx);

        let content = pro_view_transition(
            pro_eid(&self.id, "ready-pv-body-tx"),
            format!("pv-{}", self.preview_tab),
            div()
                .pt(px(20.))
                .pb(px(40.))
                .px(px(16.))
                .flex()
                .flex_col()
                .gap(px(28.))
                .child(kpis_row)
                .child(render_section(
                    tr(cx, "Your assistant", "你的助手"),
                    agent_card,
                    pal,
                ))
                .child(render_section(
                    tr(cx, "Conversations", "对话量"),
                    render_conversations_chart(pal),
                    pal,
                ))
                .child(render_section(
                    tr(cx, "Knowledge", "知识库"),
                    knowledge_card,
                    pal,
                )),
        );

        let body = div()
            .flex_1()
            .min_w_0()
            .when(!is_flat, |d| {
                d.mt(px(8.))
                    .mb(px(8.))
                    .rounded_tl(px(10.))
                    .rounded_bl(px(10.))
                    .bg(pal.card)
                    .border_t_1()
                    .border_l_1()
                    .border_b_1()
                    .border_color(pal.line)
            })
            .flex()
            .flex_col()
            .child(render_body_top_bar(pal, cx))
            .child(tabs_bar)
            .child(content);

        let win = div()
            .w_full()
            .min_h(px(1193.))
            .rounded_tl(px(12.))
            .bg(if is_flat { pal.card } else { pal.page })
            .border_t_1()
            .border_l_1()
            .border_color(pal.line)
            .overflow_hidden()
            .flex()
            .child(side)
            .child(body);

        div()
            .absolute()
            .top(px(56.))
            .bottom_0()
            .left(px(preview_w))
            .w(px((preview_w * 1.4).max(1088.)))
            .pl(px(8.))
            .overflow_hidden()
            .child(div().mt(px(24. + offset_y)).w_full().child(win))
            .child(
                div()
                    .absolute()
                    .bottom_0()
                    .left_0()
                    .right_0()
                    .h(px(190.))
                    .bg(linear_gradient(
                        180.,
                        linear_color_stop(pal.bg.opacity(0.), 0.),
                        linear_color_stop(pal.bg, 0.90),
                    )),
            )
    }

    #[allow(clippy::too_many_arguments)]
    fn render_summary_row(
        &self,
        row_idx: usize,
        id_suffix: &'static str,
        dt_label: SharedString,
        dd_content: Div,
        detail_content: Div,
        edit_aria: SharedString,
        step: OnboardingEditStep,
        pal: ObwTokens,
        cx: &mut Context<Self>,
    ) -> Div {
        let edit_lbl: SharedString = tr(cx, "Edit", "编辑").into();
        let is_open = self.expanded_row == Some(row_idx);
        let checked = self.verified_checks[row_idx];
        let detail_el = div()
            .pt(px(6.))
            .pb(px(8.))
            .pl(px(24.))
            .pr(px(8.))
            .text_size(px(12.))
            .line_height(px(18.))
            .text_color(pal.muted)
            .child(detail_content)
            .into_any_element();

        div()
            .rounded(px(8.))
            .when(is_open, |d| d.bg(pal.side))
            .flex()
            .flex_col()
            .child(
                div()
                    .min_h(px(36.))
                    .px(px(4.))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(activate(
                        div()
                            .id(pro_eid(&self.id, format!("ready-chk-{row_idx}")))
                            .role(Role::CheckBox)
                            .aria_toggled(checked.into())
                            .tab_index(0)
                            .size(px(15.))
                            .flex_none()
                            .rounded(px(4.))
                            .border_1()
                            .border_color(if checked {
                                pal.green_dot
                            } else {
                                pal.line_firm
                            })
                            .bg(if checked {
                                pal.green_dot
                            } else {
                                transparent_black()
                            })
                            .cursor_pointer()
                            .flex()
                            .items_center()
                            .justify_center()
                            .when(checked, |d| {
                                d.child(pro_icon(ProIcon::Check, 10., rgb(0xffffff).into()))
                            }),
                        cx,
                        move |this, _, cx| {
                            this.verified_checks[row_idx] = !this.verified_checks[row_idx];
                            cx.notify();
                        },
                    ))
                    .child(activate(
                        div()
                            .id(pro_eid(&self.id, format!("ready-exp-{row_idx}")))
                            .role(Role::Button)
                            .tab_index(0)
                            .flex_1()
                            .min_w_0()
                            .py(px(4.))
                            .rounded(px(6.))
                            .hover(move |s| s.bg(pal.side))
                            .cursor_pointer()
                            .flex()
                            .items_center()
                            .gap(px(10.))
                            .child(
                                div()
                                    .w(px(96.))
                                    .flex_none()
                                    .text_size(px(13.))
                                    .text_color(pal.faint)
                                    .child(dt_label),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(pal.ink)
                                    .child(dd_content),
                            )
                            .child(pro_icon(
                                if is_open {
                                    ProIcon::ChevronUp
                                } else {
                                    ProIcon::ChevronDown
                                },
                                12.,
                                pal.faint,
                            )),
                        cx,
                        move |this, _, cx| {
                            this.expanded_row = if this.expanded_row == Some(row_idx) {
                                None
                            } else {
                                Some(row_idx)
                            };
                            cx.notify();
                        },
                    ))
                    .when(self.show_edit, |d| {
                        d.child(activate(
                            div()
                                .id(pro_eid(&self.id, id_suffix))
                                .role(Role::Button)
                                .aria_label(edit_aria)
                                .tab_index(0)
                                .h(px(26.))
                                .px(px(8.))
                                .flex_none()
                                .rounded(px(6.))
                                .text_size(px(13.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(pal.faint)
                                .flex()
                                .items_center()
                                .justify_center()
                                .cursor_pointer()
                                .hover(move |s| s.bg(pal.line).text_color(pal.ink))
                                .focus_visible(move |s| s.border_1().border_color(pal.focus))
                                .child(edit_lbl),
                            cx,
                            move |this, _, cx| this.edit_step(step, cx),
                        ))
                    }),
            )
            .child(pro_fold(
                pro_eid(&self.id, format!("ready-row-fold-{row_idx}")),
                is_open,
                42.,
                0.,
                move || detail_el,
            ))
    }

    fn render_step_body(&mut self, pal: ObwTokens, cx: &mut Context<Self>) -> Div {
        if self.loading {
            return div()
                .w_full()
                .max_w(px(440.))
                .flex()
                .flex_col()
                .items_center()
                .gap(px(14.))
                .child(pro_skeleton_bar(68., 4., 2., 0.1, cx))
                .child(pro_skeleton_bar(240., 26., 6., 0.2, cx))
                .child(pro_skeleton_bar(290., 14., 4., 0.3, cx))
                .child(
                    div()
                        .w_full()
                        .mt(px(14.))
                        .flex()
                        .flex_col()
                        .gap(px(10.))
                        .child(pro_skeleton_bar(440., 32., 6., 0.4, cx))
                        .child(pro_skeleton_bar(440., 32., 6., 0.5, cx))
                        .child(pro_skeleton_bar(440., 32., 6., 0.6, cx))
                        .child(pro_skeleton_bar(440., 32., 6., 0.7, cx))
                        .child(
                            div()
                                .mt(px(8.))
                                .child(pro_skeleton_bar(440., 36., 8., 0.8, cx)),
                        ),
                );
        }

        if self.empty {
            let cta: ProEmptyCta<Self> = (
                pro_eid(&self.id, "empty-edit-website"),
                tr(cx, "Add website source", "添加网站知识来源").into(),
                Box::new(|this, _, cx| {
                    this.empty = false;
                    this.edit_step(OnboardingEditStep::Website, cx);
                }),
            );
            return div().w_full().max_w(px(440.)).child(pro_empty_state(
                ProIcon::Check,
                tr(cx, "Assistant setup incomplete", "助手配置尚未完成"),
                tr(
                    cx,
                    "Add at least one website domain before publishing your assistant.",
                    "发布助手前请至少添加一个网站知识来源。",
                ),
                Some(cta),
                cx,
            ));
        }

        // Progress dashes (.ui-obw-steps) — all 4 lit on final step
        let mut steps_bar = div().mb(px(20.)).flex().items_center().gap(px(4.));
        for idx in 1..=self.steps {
            let on = idx <= self.step;
            steps_bar = steps_bar.child(div().w(px(16.)).h(px(3.)).rounded(px(2.)).bg(if on {
                pal.dark_fill
            } else {
                pal.line
            }));
        }

        // Launch readiness checklist progress bar (`pro_spring_bar`)
        let verified_n = self.verified_checks.iter().filter(|&&b| b).count();
        let ready_ratio = (verified_n as f32 / 4.0).clamp(0., 1.);
        let ready_count_lbl: SharedString = if tr(cx, "en", "zh") == "zh" {
            format!("{verified_n} / 4 项已确认").into()
        } else {
            format!("{verified_n} of 4 verified").into()
        };
        let readiness_meter = div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(6.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_size(px(12.))
                    .child(
                        div()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(pal.ink)
                            .child(tr(cx, "Launch checklist", "上线检查清单")),
                    )
                    .child(div().text_color(pal.muted).child(ready_count_lbl)),
            )
            .child(pro_spring_bar(
                pro_eid(&self.id, "ready-progress-bar"),
                ready_ratio,
                4.,
                999.,
                pal.line,
                pal.green_dot,
            ));

        // Row 1: Learns from
        let primary_domain = self
            .domains
            .first()
            .cloned()
            .unwrap_or_else(|| "northwind.com".into());
        let extra_domains = self.domains.len().saturating_sub(1);
        let learns_dd = div()
            .min_w_0()
            .flex()
            .items_center()
            .gap(px(8.))
            .child(div().min_w_0().truncate().child(primary_domain.clone()))
            .when(extra_domains > 0, |d| {
                d.child(
                    div()
                        .h(px(18.))
                        .px(px(6.))
                        .flex_none()
                        .rounded(px(5.))
                        .bg(pal.line)
                        .text_color(pal.muted)
                        .text_size(px(11.))
                        .line_height(px(18.))
                        .child(format!("+{extra_domains}")),
                )
            });
        let learns_detail = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(6.))
            .children(self.domains.iter().map(|dom| {
                div()
                    .h(px(20.))
                    .px(px(7.))
                    .rounded(px(5.))
                    .bg(pal.card)
                    .border_1()
                    .border_color(pal.line)
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .text_size(px(11.5))
                    .text_color(pal.ink)
                    .child(div().size(px(5.)).rounded_full().bg(pal.green_dot))
                    .child(dom.clone())
            }));

        // Row 2: Name (with 3x3 pixel mark)
        let assistant_name = self.assistant_name();
        let brand_name = self.resolved_brand();
        let name_dd = div()
            .min_w_0()
            .flex()
            .items_center()
            .gap(px(8.))
            .child(obw_agent_mark(&assistant_name, pal.mark))
            .child(div().min_w_0().truncate().child(assistant_name.clone()));
        let name_detail = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(6.))
            .child(format!("{}: {brand_name}", tr(cx, "Brand", "品牌主名")))
            .children(self.aliases.iter().take(3).map(|al| {
                div()
                    .px(px(6.))
                    .py(px(1.))
                    .rounded(px(4.))
                    .bg(pal.blue.opacity(0.10))
                    .text_size(px(11.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(pal.blue)
                    .child(al.clone())
            }));

        // Row 3: Voice
        let voice_str = self.voice_summary(cx);
        let voice_dd = div().min_w_0().truncate().child(voice_str);
        let voice_detail = div().child(tr(
            cx,
            "Grounded strictly in your indexed knowledge sources with citations.",
            "严格基于已索引的知识库内容生成带引用出处的回复。",
        ));

        // Row 4: Hands over to
        let handover_str = self.handover_summary(cx);
        let handover_dd = div().min_w_0().truncate().child(handover_str);
        let handover_detail = div().child(if self.handover_to.is_some() {
            tr(
                cx,
                "Includes full conversation transcript when confidence is low.",
                "当置信度较低时将自动附上完整对话上下文转交。",
            )
        } else {
            tr(
                cx,
                "Autonomous mode: asks clarifying follow-ups instead of escalating.",
                "独立应答模式：在问题不明确时主动追问确认。",
            )
        });

        let row_1 = self.render_summary_row(
            0,
            "edit-learns",
            tr(cx, "Learns from", "知识来源").into(),
            learns_dd,
            learns_detail,
            tr(cx, "Edit learns from", "编辑知识来源").into(),
            OnboardingEditStep::Website,
            pal,
            cx,
        );
        let row_2 = self.render_summary_row(
            1,
            "edit-name",
            tr(cx, "Name", "名称").into(),
            name_dd,
            name_detail,
            tr(cx, "Edit name", "编辑名称").into(),
            OnboardingEditStep::Brand,
            pal,
            cx,
        );
        let row_3 = self.render_summary_row(
            2,
            "edit-voice",
            tr(cx, "Voice", "语气与长度").into(),
            voice_dd,
            voice_detail,
            tr(cx, "Edit voice", "编辑语气与长度").into(),
            OnboardingEditStep::Tone,
            pal,
            cx,
        );
        let row_4 = self.render_summary_row(
            3,
            "edit-handover",
            tr(cx, "Hands over to", "转人工至").into(),
            handover_dd,
            handover_detail,
            tr(cx, "Edit hands over to", "编辑转人工设置").into(),
            OnboardingEditStep::Tone,
            pal,
            cx,
        );

        // Starter workflow / prompt switcher (`pro_pill_indicator` + `pro_view_transition`)
        let wf_sensor = pro_selection_sensor();
        let wf_strip = div()
            .relative()
            .p(px(2.))
            .rounded(px(7.))
            .bg(pal.line)
            .flex()
            .items_center()
            .gap(px(2.))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "starter-wf-pill"),
                self.starter_prompt.as_str(),
                wf_sensor.clone(),
                5.,
                pal.card,
                Some(pal.line),
                false,
            ))
            .children(StarterPromptKind::ALL.into_iter().map(|kind| {
                let on = self.starter_prompt == kind;
                let lbl: SharedString = kind.label(cx).into();
                activate(
                    div()
                        .id(pro_eid(&self.id, format!("starter-wf-{}", kind.as_str())))
                        .role(Role::Tab)
                        .tab_index(0)
                        .relative()
                        .h(px(22.))
                        .px(px(8.))
                        .rounded(px(5.))
                        .when(!on, |d| {
                            d.text_color(pal.muted)
                                .hover(move |s| s.text_color(pal.ink))
                        })
                        .when(on, |d| {
                            d.text_color(pal.ink).child(pro_mark_selected(&wf_sensor))
                        })
                        .cursor_pointer()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(11.5))
                        .font_weight(FontWeight::MEDIUM)
                        .child(lbl),
                    cx,
                    move |this, _, cx| {
                        this.starter_prompt = kind;
                        cx.notify();
                    },
                )
            }));

        let wf_desc: SharedString = self.starter_prompt.description(cx).into();
        let starter_box = div()
            .p(px(10.))
            .rounded(px(9.))
            .bg(pal.card)
            .border_1()
            .border_color(pal.line)
            .flex()
            .flex_col()
            .gap(px(6.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(pal.ink)
                            .child(tr(cx, "Starter prompt", "预设欢迎工作流")),
                    )
                    .child(wf_strip),
            )
            .child(pro_view_transition(
                pro_eid(&self.id, "starter-wf-desc-tx"),
                self.starter_prompt.as_str(),
                div()
                    .text_size(px(12.))
                    .line_height(px(17.))
                    .text_color(pal.muted)
                    .child(wf_desc),
            ));

        // Open dashboard primary CTA (.ui-obr-go) + Copy / Share toast actions
        let go_lbl: SharedString = tr(cx, "Open dashboard", "打开控制台").into();
        let go_btn = activate(
            div()
                .id(pro_eid(&self.id, "open-dashboard"))
                .track_focus(&self.cta_focus)
                .role(Role::Button)
                .aria_label(go_lbl.clone())
                .tab_index(0)
                .w_full()
                .h(px(36.))
                .px(px(14.))
                .rounded(px(8.))
                .bg(pal.dark_fill)
                .text_color(pal.on_dark)
                .text_size(px(13.))
                .font_weight(FontWeight::MEDIUM)
                .flex()
                .items_center()
                .justify_center()
                .gap(px(6.))
                .cursor_pointer()
                .hover(move |s| s.opacity(0.88))
                .focus_visible(move |s| s.border_1().border_color(pal.focus))
                .child(go_lbl)
                .child(pro_icon(ProIcon::ArrowRight, 14., pal.on_dark)),
            cx,
            |this, _, cx| this.finish(cx),
        );

        let embed_snippet = format!(
            "<script src=\"https://cdn.kui.dev/widget.js\" data-domain=\"{primary_domain}\" defer></script>"
        );
        let share_url = format!("https://app.kui.dev/preview/{primary_domain}");
        let copy_toast_lbl: SharedString = tr(
            cx,
            "Widget snippet copied to clipboard",
            "嵌入代码已复制到剪贴板",
        )
        .into();
        let share_toast_lbl: SharedString = tr(
            cx,
            "Preview link copied to clipboard",
            "预览链接已复制到剪贴板",
        )
        .into();

        let secondary_actions = div()
            .flex()
            .items_center()
            .gap(px(8.))
            .child(activate(
                div()
                    .id(pro_eid(&self.id, "copy-embed-btn"))
                    .role(Role::Button)
                    .tab_index(0)
                    .flex_1()
                    .h(px(30.))
                    .px(px(10.))
                    .rounded(px(7.))
                    .bg(pal.card)
                    .border_1()
                    .border_color(pal.line)
                    .hover(move |s| s.border_color(pal.line_firm))
                    .focus_visible(move |s| s.border_1().border_color(pal.focus))
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(px(6.))
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(pal.ink)
                    .child(pro_icon(ProIcon::Copy, 12.5, pal.faint))
                    .child(tr(cx, "Copy embed", "复制嵌入代码")),
                cx,
                move |this, _, cx| {
                    cx.write_to_clipboard(ClipboardItem::new_string(embed_snippet.clone()));
                    this.toast = Some((copy_toast_lbl.clone(), Instant::now()));
                    cx.notify();
                },
            ))
            .child(activate(
                div()
                    .id(pro_eid(&self.id, "share-preview-btn"))
                    .role(Role::Button)
                    .tab_index(0)
                    .flex_1()
                    .h(px(30.))
                    .px(px(10.))
                    .rounded(px(7.))
                    .bg(pal.card)
                    .border_1()
                    .border_color(pal.line)
                    .hover(move |s| s.border_color(pal.line_firm))
                    .focus_visible(move |s| s.border_1().border_color(pal.focus))
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(px(6.))
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(pal.ink)
                    .child(pro_icon(ProIcon::Link, 12.5, pal.faint))
                    .child(tr(cx, "Share test link", "分享测试链接")),
                cx,
                move |this, _, cx| {
                    cx.write_to_clipboard(ClipboardItem::new_string(share_url.clone()));
                    this.toast = Some((share_toast_lbl.clone(), Instant::now()));
                    cx.notify();
                },
            ));

        div()
            .w_full()
            .max_w(px(440.))
            .flex()
            .flex_col()
            .items_center()
            .child(steps_bar)
            .child(
                div()
                    .text_center()
                    .text_size(px(24.))
                    .line_height(px(32.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(pal.ink)
                    .child(tr(cx, "Your assistant is ready", "你的助手已准备就绪")),
            )
            .child(
                div()
                    .mt(px(6.))
                    .text_center()
                    .text_size(px(13.))
                    .line_height(px(20.))
                    .text_color(pal.muted)
                    .child(tr(
                        cx,
                        "Here's what it knows. You can change all of it later.",
                        "以下是它已掌握的配置，你随时可以在设置中修改。",
                    )),
            )
            .child(
                div()
                    .w_full()
                    .mt(px(24.))
                    .flex()
                    .flex_col()
                    .gap(px(16.))
                    .text_left()
                    .child(readiness_meter)
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(2.))
                            .child(row_1)
                            .child(row_2)
                            .child(row_3)
                            .child(row_4),
                    )
                    .child(starter_box)
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(8.))
                            .child(go_btn)
                            .child(secondary_actions),
                    ),
            )
    }
}

impl Render for OnboardingReadyScreen {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let pal = ObwTokens::resolve(cx);
        let reduced_motion = Theme::global(cx).reduced_motion;
        let now = Instant::now();
        let opened = *self.opened_at.get_or_insert(now);
        let elapsed_ms = now.duration_since(opened).as_secs_f32() * 1000.;

        if self.auto_focus && !self.did_auto_focus {
            self.did_auto_focus = true;
            self.cta_focus.focus(w, cx);
        }

        // Smooth glide back to 0px (top of live Overview dashboard)
        let offset_y = if reduced_motion || elapsed_ms >= 420. {
            0.0_f32
        } else {
            w.request_animation_frame();
            let t = pro_ease(elapsed_ms / 420.);
            -44.0 * (1.0 - t)
        };

        let container_w = self.measured_width.get();
        let narrow = container_w <= 900.;
        let compact_side = container_w <= 1040.;
        let measure_cell = self.measured_width.clone();

        let preview = (!narrow).then(|| {
            self.render_dashboard_preview(container_w * 0.5, compact_side, offset_y, pal, cx)
        });
        let top_bar = self.render_top_bar(narrow, pal, cx);
        let step_body = self.render_step_body(pal, cx);
        let footer = self.render_footer(narrow, pal, cx);

        div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .relative()
            .size_full()
            .min_h(px(560.))
            .bg(pal.bg)
            .text_color(pal.ink)
            .text_size(px(13.))
            .overflow_hidden()
            .flex()
            .flex_col()
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _, cx| {
                if ev.keystroke.key == "enter" && !this.loading && !this.empty {
                    this.finish(cx);
                    cx.stop_propagation();
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
            .when_some(preview, |d, p| d.child(p))
            .child(top_bar)
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .w(if narrow { relative(1.0) } else { relative(0.5) })
                    .px(px(if narrow { 20. } else { 40. }))
                    .py(px(24.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(step_body),
            )
            .child(footer)
            .child(pro_toast_banner(self.toast.clone()))
    }
}

fn sk_bar(width: f32, height: f32, bg: Hsla) -> Div {
    div()
        .w(px(width))
        .h(px(height))
        .flex_none()
        .rounded(px(4.))
        .bg(bg)
}

fn sk_sq(side: f32, radius: f32, bg: Hsla) -> Div {
    div().size(px(side)).flex_none().rounded(px(radius)).bg(bg)
}

fn sk_sq_rect(width: f32, height: f32, radius: f32, bg: Hsla) -> Div {
    div()
        .w(px(width))
        .h(px(height))
        .flex_none()
        .rounded(px(radius))
        .bg(bg)
}

fn obw_agent_mark(seed: &str, color: Hsla) -> Div {
    let cells = agent_pixel_cells(seed);
    let mut grid = div()
        .size(px(16.))
        .flex_none()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(1.5));
    for r in 0..3usize {
        let mut row_el = div().flex().items_center().gap(px(1.5));
        for c in 0..3usize {
            let on = cells[r * 3 + c];
            row_el = row_el.child(div().size(px(3.)).rounded(px(0.75)).bg(if on {
                color
            } else {
                transparent_black()
            }));
        }
        grid = grid.child(row_el);
    }
    grid
}

fn render_body_top_bar(pal: ObwTokens, cx: &App) -> Div {
    div()
        .h(px(48.))
        .flex_none()
        .px(px(16.))
        .border_b_1()
        .border_color(pal.line)
        .flex()
        .items_center()
        .gap(px(12.))
        .child(pro_icon(ProIcon::SidebarLeft, 14., pal.faint))
        .child(
            div()
                .text_size(px(13.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(pal.muted)
                .child(tr(cx, "Overview", "概览")),
        )
        .child(
            div()
                .ml_auto()
                .flex()
                .items_center()
                .gap(px(6.))
                .child(
                    div()
                        .size(px(28.))
                        .rounded(px(6.))
                        .bg(pal.side)
                        .border_1()
                        .border_color(pal.line)
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(pro_icon(ProIcon::Thread, 14., pal.faint)),
                )
                .child(
                    div()
                        .size(px(28.))
                        .rounded(px(6.))
                        .bg(pal.side)
                        .border_1()
                        .border_color(pal.line)
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(pro_svg("0 0 24 24", HELP_CIRCLE_SVG, 14., pal.faint)),
                ),
        )
}

fn render_section(title: &str, card: Div, pal: ObwTokens) -> Div {
    let head = div()
        .min_h(px(28.))
        .flex()
        .flex_col()
        .gap(px(8.))
        .child(
            div()
                .text_size(px(14.))
                .line_height(px(14.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(pal.ink)
                .child(title.to_owned()),
        )
        .child(sk_bar(246., 8., pal.sk));
    div().flex().flex_col().gap(px(14.)).child(head).child(card)
}

fn render_conversations_chart(pal: ObwTokens) -> Div {
    let max_bar_h = 176.0_f32;
    let mut plot_bars = div()
        .relative()
        .flex_1()
        .min_w_0()
        .h_full()
        .pb(px(4.))
        .flex()
        .items_end()
        .gap(px(6.));
    for frac in [0.0_f32, 0.25, 0.50, 0.75] {
        plot_bars = plot_bars.child(
            div()
                .absolute()
                .left_0()
                .right_0()
                .top(px(frac * max_bar_h))
                .h(px(1.))
                .bg(pal.line),
        );
    }
    for &pct in &CHART_BAR_PCTS {
        plot_bars = plot_bars.child(
            div()
                .flex_1()
                .min_w_0()
                .h(px((pct * max_bar_h).round()))
                .rounded_tl(px(3.))
                .rounded_tr(px(3.))
                .bg(pal.sk),
        );
    }

    div()
        .rounded(px(10.))
        .border_1()
        .border_color(pal.line)
        .overflow_hidden()
        .flex()
        .flex_col()
        .child(
            div()
                .h(px(36.))
                .px(px(14.))
                .border_b_1()
                .border_color(pal.line)
                .flex()
                .items_center()
                .gap(px(8.))
                .child(sk_sq(10., 3., pal.sk))
                .child(sk_bar(88., 8., pal.sk)),
        )
        .child(
            div()
                .h(px(210.))
                .pt(px(14.))
                .pb(px(12.))
                .px(px(14.))
                .flex()
                .gap(px(14.))
                .child(
                    div()
                        .h_full()
                        .pb(px(4.))
                        .flex()
                        .flex_col()
                        .justify_between()
                        .children((0..5).map(|_| sk_bar(26., 8., pal.sk))),
                )
                .child(plot_bars),
        )
}

fn render_knowledge_table(domains: &[SharedString], pal: ObwTokens, cx: &App) -> Div {
    let header = div()
        .h(px(36.))
        .bg(pal.side)
        .border_b_1()
        .border_color(pal.line)
        .flex()
        .child(table_cell(Some(44.), true, pal, sk_bar(10., 8., pal.sk)))
        .child(table_cell(Some(200.), false, pal, sk_bar(56., 8., pal.sk)))
        .child(table_cell(Some(120.), false, pal, sk_bar(40., 8., pal.sk)))
        .child(table_cell(None, false, pal, sk_bar(72., 8., pal.sk)));

    let mut tbl = div()
        .rounded(px(10.))
        .border_1()
        .border_color(pal.line)
        .overflow_hidden()
        .flex()
        .flex_col()
        .child(header);

    let total_rows = 6usize;
    for idx in 0..total_rows {
        let row = if let Some(dom) = domains.get(idx) {
            let init: String = brand_from_domain(dom)
                .chars()
                .next()
                .unwrap_or('N')
                .to_uppercase()
                .collect();
            div()
                .h(px(34.))
                .when(idx + 1 < total_rows, |d| {
                    d.border_b_1().border_color(pal.line)
                })
                .flex()
                .child(table_cell(Some(44.), true, pal, sk_bar(10., 8., pal.sk)))
                .child(table_cell(
                    Some(200.),
                    false,
                    pal,
                    div()
                        .min_w_0()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(
                            div()
                                .size(px(15.))
                                .flex_none()
                                .rounded(px(4.))
                                .bg(pal.dark_fill)
                                .text_color(pal.on_dark)
                                .text_size(px(9.))
                                .font_weight(FontWeight::MEDIUM)
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(init),
                        )
                        .child(
                            div()
                                .min_w_0()
                                .truncate()
                                .text_size(px(13.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(pal.ink)
                                .child(dom.clone()),
                        ),
                ))
                .child(table_cell(
                    Some(120.),
                    false,
                    pal,
                    div()
                        .h(px(18.))
                        .px(px(6.))
                        .rounded(px(5.))
                        .bg(pal.blue.opacity(0.10))
                        .text_color(pal.blue)
                        .text_size(px(11.))
                        .font_weight(FontWeight::MEDIUM)
                        .flex()
                        .items_center()
                        .child(tr(cx, "Learning", "学习中")),
                ))
                .child(table_cell(
                    None,
                    false,
                    pal,
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(sk_sq(10., 3., pal.sk))
                        .child(sk_bar(104., 8., pal.sk)),
                ))
        } else {
            div()
                .h(px(34.))
                .when(idx + 1 < total_rows, |d| {
                    d.border_b_1().border_color(pal.line)
                })
                .flex()
                .child(table_cell(Some(44.), true, pal, sk_bar(10., 8., pal.sk)))
                .child(table_cell(
                    Some(200.),
                    false,
                    pal,
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(sk_sq(10., 3., pal.sk))
                        .child(sk_bar(104., 8., pal.sk)),
                ))
                .child(table_cell(
                    Some(120.),
                    false,
                    pal,
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(sk_sq(10., 3., pal.sk))
                        .child(sk_bar(52., 8., pal.sk)),
                ))
                .child(table_cell(
                    None,
                    false,
                    pal,
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(sk_sq(10., 3., pal.sk))
                        .child(sk_bar(104., 8., pal.sk)),
                ))
        };
        tbl = tbl.child(row);
    }
    tbl
}

fn table_cell(width: Option<f32>, centered: bool, pal: ObwTokens, child: impl IntoElement) -> Div {
    let mut cell = div().h_full().flex().items_center().child(child);
    if let Some(w_px) = width {
        cell = cell
            .w(px(w_px))
            .flex_none()
            .border_r_1()
            .border_color(pal.line);
    } else {
        cell = cell.flex_1().min_w_0();
    }
    if centered {
        cell.justify_center()
    } else {
        cell.px(px(12.))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        OnboardingEditStep, OnboardingReadyEvent, OnboardingReadyLength, OnboardingReadyScreen,
        OnboardingReadyTone, format_handover_summary, format_voice_summary,
    };
    use gpui::TestAppContext;
    use std::{cell::RefCell, rc::Rc};

    #[test]
    fn summary_formatters_match_reference_copy_and_empty_handover() {
        assert_eq!(
            format_voice_summary(
                OnboardingReadyTone::Neutral,
                OnboardingReadyLength::Balanced
            ),
            "Neutral, balanced replies"
        );
        assert_eq!(
            format_voice_summary(OnboardingReadyTone::Formal, OnboardingReadyLength::Detailed),
            "Formal, detailed replies"
        );
        assert_eq!(
            format_handover_summary(Some("support@northwind.com")),
            "support@northwind.com"
        );
        assert_eq!(
            format_handover_summary(Some("   ")),
            "Nobody – it answers itself"
        );
        assert_eq!(format_handover_summary(None), "Nobody – it answers itself");
    }

    #[test]
    fn ready_screen_routes_step_edits_and_finish_events() {
        let mut app = TestAppContext::single();
        app.update(crate::init);

        let events: Rc<RefCell<Vec<OnboardingReadyEvent>>> = Rc::new(RefCell::new(Vec::new()));
        let captured = events.clone();

        let (view, w) = app.add_window_view(|_w, cx| {
            OnboardingReadyScreen::new("test-obw-ready", cx)
                .with_domains(["fieldwork.io", "docs.fieldwork.io"])
                .with_tone(OnboardingReadyTone::Friendly)
                .with_length(OnboardingReadyLength::Short)
                .with_handover_to(None)
        });

        w.update(|_w, cx| {
            cx.subscribe(&view, move |_, ev: &OnboardingReadyEvent, _| {
                captured.borrow_mut().push(ev.clone());
            })
            .detach();
            view.update(cx, |screen, cx| {
                assert_eq!(screen.domains().len(), 2);
                assert_eq!(screen.assistant_name(), "Fieldwork Assistant");
                assert_eq!(screen.voice_summary(cx), "Friendly, short replies");
                assert_eq!(screen.handover_summary(cx), "Nobody – it answers itself");

                screen.edit_step(OnboardingEditStep::Website, cx);
                screen.edit_step(OnboardingEditStep::Tone, cx);
                screen.finish(cx);
            });
        });

        let log = events.borrow();
        assert_eq!(
            log.as_slice(),
            &[
                OnboardingReadyEvent::Edit(OnboardingEditStep::Website),
                OnboardingReadyEvent::Edit(OnboardingEditStep::Tone),
                OnboardingReadyEvent::Finish,
            ]
        );
        assert_eq!(OnboardingEditStep::Website.step_number(), 1);
        assert_eq!(OnboardingEditStep::Brand.step_number(), 2);
        assert_eq!(OnboardingEditStep::Tone.step_number(), 3);
    }
}
