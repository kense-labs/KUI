//! Onboarding wizard step three for assistant name, voice tone, reply length, and handover.
//!
//! Renders a compact left-aligned form with radio-group presets and a capsule handover
//! switch while the decorative dashboard on the right glides down to a live test chat
//! whose assistant name, reply phrasing, and escalation target update immediately.

use crate::{
    components::{
        app_shell::AppShellLayout,
        pro_chrome::{
            ProEmptyCta, ProPalette, pro_ease, pro_eid, pro_empty_state, pro_fold,
            pro_mark_selected, pro_mix_color, pro_pill_indicator, pro_selection_sensor,
            pro_skeleton_bar, pro_spring_f32, pro_view_transition, tr,
        },
    },
    foundation::{Theme, ThemeMode},
    primitives::{
        control::activate,
        editor::{EditEvent, Editor, InputMode},
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

/// Conversational tone preset for [`OnboardingToneScreen`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OnboardingToneVoice {
    Friendly,
    #[default]
    Neutral,
    Formal,
}

impl OnboardingToneVoice {
    /// Ordered variants shown in the `Tone` radio group.
    pub const ALL: [Self; 3] = [Self::Friendly, Self::Neutral, Self::Formal];

    /// Canonical lowercase key (`"friendly"`, `"neutral"`, `"formal"`).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Friendly => "friendly",
            Self::Neutral => "neutral",
            Self::Formal => "formal",
        }
    }

    /// Localized display label for the radio option.
    pub fn label(self, cx: &App) -> &'static str {
        match self {
            Self::Friendly => tr(cx, "Friendly", "友好"),
            Self::Neutral => tr(cx, "Neutral", "中性"),
            Self::Formal => tr(cx, "Formal", "正式"),
        }
    }

    fn step_by(self, delta: isize) -> Self {
        let idx = match self {
            Self::Friendly => 0isize,
            Self::Neutral => 1,
            Self::Formal => 2,
        };
        Self::ALL[(idx + delta).rem_euclid(3) as usize]
    }
}

/// Reply verbosity preset for [`OnboardingToneScreen`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OnboardingReplyLength {
    Short,
    #[default]
    Balanced,
    Detailed,
}

impl OnboardingReplyLength {
    /// Ordered variants shown in the `Reply length` radio group.
    pub const ALL: [Self; 3] = [Self::Short, Self::Balanced, Self::Detailed];

    /// Canonical lowercase key (`"short"`, `"balanced"`, `"detailed"`).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Short => "short",
            Self::Balanced => "balanced",
            Self::Detailed => "detailed",
        }
    }

    /// Localized display label for the radio option.
    pub fn label(self, cx: &App) -> &'static str {
        match self {
            Self::Short => tr(cx, "Short", "简短"),
            Self::Balanced => tr(cx, "Balanced", "适中"),
            Self::Detailed => tr(cx, "Detailed", "详尽"),
        }
    }

    fn step_by(self, delta: isize) -> Self {
        let idx = match self {
            Self::Short => 0isize,
            Self::Balanced => 1,
            Self::Detailed => 2,
        };
        Self::ALL[(idx + delta).rem_euclid(3) as usize]
    }
}

/// Submitted assistant behavior configuration from [`OnboardingToneScreen`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OnboardingToneValues {
    pub name: SharedString,
    pub tone: OnboardingToneVoice,
    pub length: OnboardingReplyLength,
    pub handover: bool,
    pub email: SharedString,
}

/// Events emitted by [`OnboardingToneScreen`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OnboardingToneEvent {
    /// User pressed Continue with valid assistant settings.
    Continue(OnboardingToneValues),
    /// Any assistant setting changed interactively.
    Change(OnboardingToneValues),
    /// User pressed Back in the top header.
    Back,
    /// User pressed Log out in the top header.
    Logout,
    /// User activated a footer policy link (`"terms"` or `"privacy"`).
    OpenLegal(SharedString),
}

/// Validate a handover email address (`local@domain.tld`).
pub fn is_valid_handover_email(raw: &str) -> bool {
    let s = raw.trim();
    if s.is_empty() || s.chars().any(char::is_whitespace) {
        return false;
    }
    let Some((local, rest)) = s.split_once('@') else {
        return false;
    };
    if local.is_empty() || rest.contains('@') {
        return false;
    }
    let parts: Vec<&str> = rest.split('.').collect();
    parts.len() >= 2 && parts.iter().all(|seg| !seg.is_empty())
}

/// Sample test-chat reply matching the chosen [`OnboardingToneVoice`] and [`OnboardingReplyLength`].
pub fn sample_test_reply(tone: OnboardingToneVoice, length: OnboardingReplyLength) -> &'static str {
    match (tone, length) {
        (OnboardingToneVoice::Friendly, OnboardingReplyLength::Short) => {
            "Absolutely! Switch anytime and we'll prorate the rest of the month."
        }
        (OnboardingToneVoice::Friendly, OnboardingReplyLength::Balanced) => {
            "Of course! You can switch plans whenever you like, and we'll prorate the difference for the rest of the month."
        }
        (OnboardingToneVoice::Friendly, OnboardingReplyLength::Detailed) => {
            "Happy to help! You can change your plan anytime from Billing – upgrades start right away and we prorate your remaining days automatically."
        }
        (OnboardingToneVoice::Neutral, OnboardingReplyLength::Short) => {
            "Yes – switch anytime and we prorate the difference."
        }
        (OnboardingToneVoice::Neutral, OnboardingReplyLength::Balanced) => {
            "Yes. You can change your plan at any time; the difference is prorated."
        }
        (OnboardingToneVoice::Neutral, OnboardingReplyLength::Detailed) => {
            "Yes. You can upgrade or downgrade your plan at any time from Billing. Changes take effect immediately and the remaining cycle is prorated."
        }
        (OnboardingToneVoice::Formal, OnboardingReplyLength::Short) => {
            "Yes. Plan modifications take effect immediately on a prorated basis."
        }
        (OnboardingToneVoice::Formal, OnboardingReplyLength::Balanced) => {
            "Certainly. Plans may be modified at any point during the billing cycle, with charges adjusted on a prorated basis."
        }
        (OnboardingToneVoice::Formal, OnboardingReplyLength::Detailed) => {
            "Certainly. You may modify your subscription tier at any time under Billing Settings; any balance adjustment is calculated on a prorated basis."
        }
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

fn clean_host(domain: &str) -> String {
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
    let host = host.strip_prefix("www.").unwrap_or(host);
    if host.is_empty() {
        "northwind.com".to_string()
    } else {
        host.to_string()
    }
}

#[derive(Clone, Copy)]
struct ObwTokens {
    dark: bool,
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
    red: Hsla,
    mark: Hsla,
    focus: Hsla,
}

impl ObwTokens {
    fn resolve(cx: &App) -> Self {
        let dark = Theme::global(cx).mode == ThemeMode::Dark;
        let pro = ProPalette::get(cx);
        if dark {
            Self {
                dark: true,
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
                red: rgb(0xff8a7a).into(),
                mark: rgba(0x4d8dffda).into(),
                focus: pro.focus,
            }
        } else {
            Self {
                dark: false,
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
                red: rgb(0xd93a2b).into(),
                mark: rgba(0x0060e2c7).into(),
                focus: pro.focus,
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum ToneSampleScenario {
    #[default]
    Plan,
    Refund,
    Security,
}

impl ToneSampleScenario {
    const ALL: [Self; 3] = [Self::Plan, Self::Refund, Self::Security];

    fn as_str(self) -> &'static str {
        match self {
            Self::Plan => "plan",
            Self::Refund => "refund",
            Self::Security => "security",
        }
    }

    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::Plan => tr(cx, "Plan", "套餐"),
            Self::Refund => tr(cx, "Refund", "退款"),
            Self::Security => tr(cx, "Security", "安全"),
        }
    }

    fn question(self) -> &'static str {
        match self {
            Self::Plan => "Can I change my plan mid-month?",
            Self::Refund => "How do refunds work if we cancel?",
            Self::Security => "Where is our workspace data stored?",
        }
    }

    fn reply(self, tone: OnboardingToneVoice, length: OnboardingReplyLength) -> &'static str {
        match self {
            Self::Plan => sample_test_reply(tone, length),
            Self::Refund => match (tone, length) {
                (OnboardingToneVoice::Friendly, OnboardingReplyLength::Short) => {
                    "Cancel anytime! Unused days on annual plans are refunded within 5 business days."
                }
                (OnboardingToneVoice::Friendly, OnboardingReplyLength::Balanced) => {
                    "No lock-in at all! You can cancel from Billing anytime, and unused annual credit is refunded automatically."
                }
                (OnboardingToneVoice::Friendly, OnboardingReplyLength::Detailed) => {
                    "Happy to clarify! Cancel with one click under Billing — monthly plans stop at renewal, and annual plans refund unused days within 5 days."
                }
                (OnboardingToneVoice::Neutral, OnboardingReplyLength::Short) => {
                    "Cancel anytime; unused annual balance is refunded."
                }
                (OnboardingToneVoice::Neutral, OnboardingReplyLength::Balanced) => {
                    "You can cancel from Billing at any time. Unused annual time is refunded within 5 business days."
                }
                (OnboardingToneVoice::Neutral, OnboardingReplyLength::Detailed) => {
                    "Cancel anytime under Billing. Monthly subscriptions end at cycle close; annual plans receive a prorated refund within 5 business days."
                }
                (OnboardingToneVoice::Formal, OnboardingReplyLength::Short) => {
                    "Cancellations take effect per cycle; annual balances are refunded pro rata."
                }
                (OnboardingToneVoice::Formal, OnboardingReplyLength::Balanced) => {
                    "Subscriptions may be cancelled under Billing Settings; unused annual commitments are refunded on a prorated basis."
                }
                (OnboardingToneVoice::Formal, OnboardingReplyLength::Detailed) => {
                    "Subscriptions may be terminated at any time via Billing Settings. Eligible prorated annual refunds are issued within five business days."
                }
            },
            Self::Security => match (tone, length) {
                (OnboardingToneVoice::Friendly, OnboardingReplyLength::Short) => {
                    "Encrypted in US or EU regions with SOC 2 Type II — never used for training!"
                }
                (OnboardingToneVoice::Friendly, OnboardingReplyLength::Balanced) => {
                    "Your data stays encrypted at rest and in transit in your chosen US or EU region, and we never train models on it."
                }
                (OnboardingToneVoice::Friendly, OnboardingReplyLength::Detailed) => {
                    "Great question! Workspace data is AES-256 encrypted in US-East or EU-Central, SOC 2 Type II audited, with zero model training retention."
                }
                (OnboardingToneVoice::Neutral, OnboardingReplyLength::Short) => {
                    "Encrypted in US or EU regions; SOC 2 Type II certified."
                }
                (OnboardingToneVoice::Neutral, OnboardingReplyLength::Balanced) => {
                    "Data is encrypted in your selected US or EU region under SOC 2 Type II controls, with zero training retention."
                }
                (OnboardingToneVoice::Neutral, OnboardingReplyLength::Detailed) => {
                    "Workspace data is stored in US-East or EU-Central with AES-256 encryption, TLS 1.3 transport, SOC 2 Type II compliance, and zero training use."
                }
                (OnboardingToneVoice::Formal, OnboardingReplyLength::Short) => {
                    "Data resides in SOC 2 Type II audited US or EU regions with AES-256 encryption."
                }
                (OnboardingToneVoice::Formal, OnboardingReplyLength::Balanced) => {
                    "All customer records reside in isolated US or EU regions protected by AES-256 encryption and SOC 2 Type II controls."
                }
                (OnboardingToneVoice::Formal, OnboardingReplyLength::Detailed) => {
                    "Customer data is hosted in dedicated US or EU regions with AES-256 encryption at rest, TLS 1.3 in transit, and zero-retention model policies."
                }
            },
        }
    }
}

/// Step 3 of 4 in the AI assistant onboarding wizard: persona name, tone, length, and handover.
pub struct OnboardingToneScreen {
    id: ElementId,
    domain: SharedString,
    brand: Option<SharedString>,
    aliases: Vec<SharedString>,
    assistant_name: SharedString,
    explicit_name: bool,
    tone: OnboardingToneVoice,
    length: OnboardingReplyLength,
    scenario: ToneSampleScenario,
    handover: bool,
    handover_email: SharedString,
    explicit_handover_email: bool,
    account_email: SharedString,
    product: SharedString,
    layout: AppShellLayout,
    step: usize,
    steps: usize,
    show_back: bool,
    auto_focus: bool,
    did_auto_focus: bool,
    loading: bool,
    empty: bool,
    pending_name_sync: Option<String>,
    pending_email_sync: Option<String>,
    name_editor: Option<Entity<Editor>>,
    email_editor: Option<Entity<Editor>>,
    focus: FocusHandle,
    measured_width: Rc<Cell<f32>>,
    opened_at: Option<Instant>,
}

impl EventEmitter<OnboardingToneEvent> for OnboardingToneScreen {}

impl OnboardingToneScreen {
    /// Construct the Tone onboarding step with `Northwind Assistant` defaults.
    pub fn new(id: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let domain = "northwind.com";
        let brand = brand_from_domain(domain);
        Self {
            id: ElementId::Name(id.into()),
            domain: domain.into(),
            brand: None,
            aliases: vec!["Northwind Co.".into()],
            assistant_name: format!("{brand} Assistant").into(),
            explicit_name: false,
            tone: OnboardingToneVoice::Neutral,
            length: OnboardingReplyLength::Balanced,
            scenario: ToneSampleScenario::Plan,
            handover: true,
            handover_email: format!("support@{domain}").into(),
            explicit_handover_email: false,
            account_email: "alex@northwind.com".into(),
            product: "Lumen".into(),
            layout: AppShellLayout::Inset,
            step: 3,
            steps: 4,
            show_back: true,
            auto_focus: false,
            did_auto_focus: false,
            loading: false,
            empty: false,
            pending_name_sync: None,
            pending_email_sync: None,
            name_editor: None,
            email_editor: None,
            focus: cx.focus_handle(),
            measured_width: Rc::new(Cell::new(1280.)),
            opened_at: None,
        }
    }

    /// Set the primary website domain from Step 1; updates the suggested name and
    /// `support@<domain>` address unless explicitly overridden.
    pub fn with_domain(mut self, domain: impl Into<SharedString>) -> Self {
        self.domain = domain.into();
        let host = clean_host(&self.domain);
        if !self.explicit_name && self.brand.is_none() {
            let b = brand_from_domain(&host);
            self.assistant_name = format!("{b} Assistant").into();
        }
        if !self.explicit_handover_email {
            self.handover_email = format!("support@{host}").into();
        }
        self
    }

    /// Set the company brand name from Step 2; updates the suggested assistant name
    /// unless [`Self::with_default_name`] was already supplied.
    pub fn with_brand(mut self, brand: impl Into<SharedString>) -> Self {
        let b: SharedString = brand.into();
        if !self.explicit_name && !b.trim().is_empty() {
            self.assistant_name = format!("{} Assistant", b.trim()).into();
        }
        self.brand = Some(b);
        self
    }

    /// Set the short forms from Step 2 displayed on the decorative assistant card.
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

    /// Set the initial assistant display name.
    pub fn with_default_name(mut self, name: impl Into<SharedString>) -> Self {
        self.explicit_name = true;
        self.assistant_name = name.into();
        self
    }

    /// Set the initially selected conversational tone.
    pub fn with_default_tone(mut self, tone: OnboardingToneVoice) -> Self {
        self.tone = tone;
        self
    }

    /// Set the initially selected reply length.
    pub fn with_default_length(mut self, length: OnboardingReplyLength) -> Self {
        self.length = length;
        self
    }

    /// Set whether human handover is initially enabled when the assistant is unsure.
    pub fn with_default_handover(mut self, handover: bool) -> Self {
        self.handover = handover;
        self
    }

    /// Set the initial escalation email address.
    pub fn with_default_email(mut self, email: impl Into<SharedString>) -> Self {
        self.explicit_handover_email = true;
        self.handover_email = email.into();
        self
    }

    /// Set the signed-in account email displayed in the top-right header.
    pub fn with_email(mut self, email: impl Into<SharedString>) -> Self {
        self.account_email = email.into();
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

    /// Show or hide the `Back` button in the top bar.
    pub fn with_show_back(mut self, show: bool) -> Self {
        self.show_back = show;
        self
    }

    /// Focus the assistant name field automatically when the screen opens.
    pub fn with_auto_focus(mut self, auto_focus: bool) -> Self {
        self.auto_focus = auto_focus;
        self
    }

    /// Render skeleton placeholders in place of the step form.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Render an empty state in place of the step form.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Snapshot the current form values.
    pub fn values(&self) -> OnboardingToneValues {
        OnboardingToneValues {
            name: SharedString::from(self.assistant_name.trim().to_owned()),
            tone: self.tone,
            length: self.length,
            handover: self.handover,
            email: SharedString::from(self.handover_email.trim().to_owned()),
        }
    }

    /// Select a [`OnboardingToneVoice`] preset and emit [`OnboardingToneEvent::Change`].
    pub fn set_tone(&mut self, tone: OnboardingToneVoice, cx: &mut Context<Self>) {
        if self.tone != tone {
            self.tone = tone;
            let vals = self.values();
            cx.emit(OnboardingToneEvent::Change(vals));
            cx.notify();
        }
    }

    /// Select an [`OnboardingReplyLength`] preset and emit [`OnboardingToneEvent::Change`].
    pub fn set_length(&mut self, length: OnboardingReplyLength, cx: &mut Context<Self>) {
        if self.length != length {
            self.length = length;
            let vals = self.values();
            cx.emit(OnboardingToneEvent::Change(vals));
            cx.notify();
        }
    }

    /// Toggle human handover and emit [`OnboardingToneEvent::Change`].
    pub fn set_handover(&mut self, enabled: bool, cx: &mut Context<Self>) {
        if self.handover != enabled {
            self.handover = enabled;
            if let Some(ed) = &self.email_editor {
                ed.update(cx, |e, _| e.disabled = !enabled);
            }
            let vals = self.values();
            cx.emit(OnboardingToneEvent::Change(vals));
            cx.notify();
        }
    }

    /// Update the assistant name programmatically (useful in headless tests and flows).
    pub fn set_assistant_name(&mut self, name: impl Into<SharedString>, cx: &mut Context<Self>) {
        let next: SharedString = name.into();
        self.pending_name_sync = Some(next.to_string());
        self.assistant_name = next;
        let vals = self.values();
        cx.emit(OnboardingToneEvent::Change(vals));
        cx.notify();
    }

    /// Update the handover email programmatically (useful in headless tests and flows).
    pub fn set_handover_email(&mut self, email: impl Into<SharedString>, cx: &mut Context<Self>) {
        let next: SharedString = email.into();
        self.pending_email_sync = Some(next.to_string());
        self.handover_email = next;
        let vals = self.values();
        cx.emit(OnboardingToneEvent::Change(vals));
        cx.notify();
    }

    /// Whether the form is currently valid for submission.
    pub fn can_submit(&self) -> bool {
        let vals = self.values();
        !vals.name.is_empty() && (!vals.handover || is_valid_handover_email(&vals.email))
    }

    /// Validate and emit [`OnboardingToneEvent::Continue`] when valid.
    pub fn submit(&mut self, cx: &mut Context<Self>) -> bool {
        if !self.can_submit() {
            cx.notify();
            return false;
        }
        let vals = self.values();
        cx.emit(OnboardingToneEvent::Continue(vals));
        cx.notify();
        true
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

    fn resolved_brand(&self) -> String {
        self.brand
            .as_ref()
            .map(|b| b.trim().to_owned())
            .filter(|b| !b.is_empty())
            .unwrap_or_else(|| brand_from_domain(&self.domain))
    }

    fn ensure_editors(
        &mut self,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> (Entity<Editor>, Entity<Editor>) {
        let name_ed = if let Some(ed) = &self.name_editor {
            let ed = ed.clone();
            if let Some(sync_val) = self.pending_name_sync.take() {
                ed.update(cx, |e, cx| e.set_value(sync_val, w, cx));
            }
            ed
        } else {
            self.pending_name_sync = None;
            let initial = self.assistant_name.to_string();
            let ph = format!("{} Assistant", self.resolved_brand());
            let label: SharedString = tr(cx, "Name", "名称").into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(label, InputMode::Single, w, cx);
                e.set_placeholder(ph);
                e.font_size = 13.;
                e.set_value(initial, w, cx);
                e
            });
            cx.subscribe(&ed, |this, ed, ev: &EditEvent, cx| match ev {
                EditEvent::Changed => {
                    this.assistant_name = ed.read(cx).value().to_owned().into();
                    let vals = this.values();
                    cx.emit(OnboardingToneEvent::Change(vals));
                    cx.notify();
                }
                EditEvent::Submitted => {
                    let _ = this.submit(cx);
                }
                _ => {
                    cx.notify();
                }
            })
            .detach();
            if self.auto_focus && !self.did_auto_focus {
                self.did_auto_focus = true;
                ed.update(cx, |e, cx| e.focus(w, cx));
            }
            self.name_editor = Some(ed.clone());
            ed
        };

        let handover_on = self.handover;
        let email_ed = if let Some(ed) = &self.email_editor {
            let ed = ed.clone();
            let sync_val = self.pending_email_sync.take();
            ed.update(cx, |e, cx| {
                e.disabled = !handover_on;
                if let Some(val) = sync_val {
                    e.set_value(val, w, cx);
                }
            });
            ed
        } else {
            self.pending_email_sync = None;
            let initial = self.handover_email.to_string();
            let ph = format!("support@{}", clean_host(&self.domain));
            let label: SharedString = tr(cx, "Hand over to", "转人工邮箱").into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(label, InputMode::Single, w, cx);
                e.set_placeholder(ph);
                e.font_size = 13.;
                e.disabled = !handover_on;
                e.set_value(initial, w, cx);
                e
            });
            cx.subscribe(&ed, |this, ed, ev: &EditEvent, cx| match ev {
                EditEvent::Changed => {
                    this.handover_email = ed.read(cx).value().to_owned().into();
                    let vals = this.values();
                    cx.emit(OnboardingToneEvent::Change(vals));
                    cx.notify();
                }
                EditEvent::Submitted => {
                    let _ = this.submit(cx);
                }
                _ => {
                    cx.notify();
                }
            })
            .detach();
            self.email_editor = Some(ed.clone());
            ed
        };

        (name_ed, email_ed)
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
                |_, _, cx| cx.emit(OnboardingToneEvent::Back),
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
                    .child(
                        div()
                            .text_color(pal.muted)
                            .child(self.account_email.clone()),
                    ),
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
            |_, _, cx| cx.emit(OnboardingToneEvent::Logout),
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
                |_, _, cx| cx.emit(OnboardingToneEvent::OpenLegal("terms".into())),
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
                |_, _, cx| cx.emit(OnboardingToneEvent::OpenLegal("privacy".into())),
            ))
    }

    #[allow(clippy::too_many_arguments)]
    fn render_dashboard_preview(
        &self,
        preview_w: f32,
        compact_side: bool,
        offset_y: f32,
        live_name: &str,
        live_email: &str,
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

        let display_name = if live_name.trim().is_empty() {
            format!("{brand} Assistant")
        } else {
            live_name.trim().to_owned()
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
                    .child(initial.clone()),
            )
            .when(!compact_side, |d| {
                d.child(
                    div()
                        .min_w_0()
                        .truncate()
                        .text_size(px(13.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(pal.ink)
                        .child(brand.clone()),
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

        // Agent summary card (above fold)
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
                    .child(obw_agent_mark(&display_name, pal.mark, false))
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(pal.ink)
                            .child(display_name.clone()),
                    )
                    .child(
                        div()
                            .ml(px(2.))
                            .h(px(18.))
                            .px(px(6.))
                            .rounded(px(5.))
                            .bg(pal.line)
                            .text_color(pal.muted)
                            .text_size(px(11.))
                            .font_weight(FontWeight::MEDIUM)
                            .flex()
                            .items_center()
                            .child(tr(cx, "Draft", "草稿")),
                    ),
            )
            .child(
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
                            .child(format!(
                                "Hi, I'm {display_name} – ask me anything about our products, plans or your account."
                            )),
                    ),
            )
            .child(aka_row);

        let host = clean_host(&self.domain);
        let knowledge_card = render_knowledge_table(&host, &initial, pal, cx);

        // Live Test chat card (.ui-obw-chat) — front and center at offset_y = -946px!
        let question_text = self.scenario.question();
        let reply_text = self.scenario.reply(self.tone, self.length);
        let reply_tx_key: SharedString = format!(
            "{}-{}-{}",
            self.tone.as_str(),
            self.length.as_str(),
            self.scenario.as_str()
        )
        .into();
        let trimmed_email = live_email.trim();
        let shown_email = if trimmed_email.is_empty() {
            format!("support@{host}")
        } else {
            trimmed_email.to_owned()
        };

        let handover_line = if self.handover {
            div()
                .h(px(20.))
                .pl(px(26.))
                .flex()
                .items_center()
                .gap(px(6.))
                .text_size(px(12.))
                .text_color(pal.ink)
                .child(tr(cx, "Not sure? It hands over to", "不确定时转交给"))
                .child(
                    div()
                        .max_w(px(220.))
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
                        .child(shown_email),
                )
        } else {
            div()
                .h(px(20.))
                .pl(px(26.))
                .flex()
                .items_center()
                .text_size(px(12.))
                .text_color(pal.ink)
                .child(tr(
                    cx,
                    "It answers everything itself",
                    "所有问题均由助手独立解答",
                ))
        };

        let pv_scen_sensor = pro_selection_sensor();
        let pv_scen_strip = div()
            .relative()
            .p(px(2.))
            .rounded(px(7.))
            .bg(pal.line)
            .flex()
            .items_center()
            .gap(px(2.))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "pv-scen-pill"),
                self.scenario.as_str(),
                pv_scen_sensor.clone(),
                5.,
                pal.card,
                Some(pal.line),
                false,
            ))
            .children(ToneSampleScenario::ALL.into_iter().map(|sc| {
                let on = self.scenario == sc;
                let lbl: SharedString = sc.label(cx).into();
                activate(
                    div()
                        .id(pro_eid(&self.id, format!("pv-scen-{}", sc.as_str())))
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
                            d.text_color(pal.ink)
                                .child(pro_mark_selected(&pv_scen_sensor))
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
                        this.scenario = sc;
                        cx.notify();
                    },
                )
            }));

        let chat_card = div()
            .min_h(px(252.))
            .p(px(16.))
            .rounded(px(10.))
            .border_1()
            .border_color(pal.line)
            .flex()
            .flex_col()
            .gap(px(12.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .child(div().text_size(px(11.5)).text_color(pal.faint).child(tr(
                        cx,
                        "Sample scenario",
                        "示例场景",
                    )))
                    .child(pv_scen_strip),
            )
            .child(pro_view_transition(
                pro_eid(&self.id, "sample-q-tx"),
                self.scenario.as_str(),
                div()
                    .max_w(px(400.))
                    .flex()
                    .items_start()
                    .gap(px(10.))
                    .child(
                        div()
                            .mt(px(10.))
                            .child(obw_agent_mark("customer", pal.ink, true)),
                    )
                    .child(
                        div()
                            .px(px(14.))
                            .py(px(10.))
                            .rounded_tl(px(4.))
                            .rounded_tr(px(12.))
                            .rounded_br(px(12.))
                            .rounded_bl(px(12.))
                            .bg(pal.card)
                            .border_1()
                            .border_color(pal.line)
                            .text_size(px(13.))
                            .line_height(px(20.))
                            .text_color(pal.ink)
                            .child(question_text),
                    ),
            ))
            .child(
                div()
                    .max_w(px(420.))
                    .flex()
                    .items_start()
                    .gap(px(10.))
                    .child(
                        div()
                            .mt(px(30.))
                            .child(obw_agent_mark(&display_name, pal.mark, false)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap(px(4.))
                            .child(
                                div()
                                    .pl(px(2.))
                                    .truncate()
                                    .text_size(px(12.))
                                    .line_height(px(16.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(pal.muted)
                                    .child(display_name),
                            )
                            .child(pro_view_transition(
                                pro_eid(&self.id, "sample-reply-tx"),
                                reply_tx_key,
                                div()
                                    .max_w(px(394.))
                                    .px(px(14.))
                                    .py(px(10.))
                                    .rounded_tl(px(4.))
                                    .rounded_tr(px(12.))
                                    .rounded_br(px(12.))
                                    .rounded_bl(px(12.))
                                    .bg(pal.side)
                                    .border_1()
                                    .border_color(pal.line)
                                    .text_size(px(13.))
                                    .line_height(px(20.))
                                    .text_color(pal.ink)
                                    .child(reply_text),
                            )),
                    ),
            )
            .child(pro_view_transition(
                pro_eid(&self.id, "handover-line-tx"),
                format!("ho-{}", self.handover),
                handover_line,
            ));

        let pair_cards = div().flex().gap(px(12.)).children((0..2).map(|_| {
            div()
                .flex_1()
                .min_w_0()
                .h(px(178.))
                .p(px(16.))
                .rounded(px(10.))
                .border_1()
                .border_color(pal.line)
                .flex()
                .flex_col()
                .gap(px(14.))
                .child(sk_bar(88., 8., pal.sk))
                .child(sk_bar(158., 8., pal.sk))
        }));

        let trailing_table = render_trailing_skeleton_table(pal);

        let content = div()
            .pt(px(20.))
            .pb(px(40.))
            .px(px(16.))
            .flex()
            .flex_col()
            .gap(px(28.))
            .child(render_skeleton_kpis(pal))
            .child(render_section(
                Some(tr(cx, "Your assistant", "你的助手")),
                agent_card,
                pal,
            ))
            .child(render_section(
                Some(tr(cx, "Conversations", "对话量")),
                render_conversations_chart(pal),
                pal,
            ))
            .child(render_section(
                Some(tr(cx, "Knowledge", "知识库")),
                knowledge_card,
                pal,
            ))
            .child(render_section(
                Some(tr(cx, "Test chat", "测试对话")),
                chat_card,
                pal,
            ))
            .child(pair_cards)
            .child(render_section(None, trailing_table, pal));

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
            .child(render_skeleton_tabs(pal))
            .child(content);

        let win = div()
            .w_full()
            .min_h(px(1760.))
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
                    .top_0()
                    .left_0()
                    .right_0()
                    .h(px(22.))
                    .bg(linear_gradient(
                        180.,
                        linear_color_stop(pal.bg, 0.),
                        linear_color_stop(pal.bg.opacity(0.), 1.),
                    )),
            )
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

    fn render_step_body(
        &mut self,
        name_ed: Entity<Editor>,
        email_ed: Entity<Editor>,
        pal: ObwTokens,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Div {
        if self.loading {
            return div()
                .w_full()
                .max_w(px(440.))
                .flex()
                .flex_col()
                .items_center()
                .gap(px(14.))
                .child(pro_skeleton_bar(68., 4., 2., 0.1, cx))
                .child(pro_skeleton_bar(290., 26., 6., 0.2, cx))
                .child(pro_skeleton_bar(240., 14., 4., 0.3, cx))
                .child(
                    div()
                        .w_full()
                        .mt(px(14.))
                        .flex()
                        .flex_col()
                        .gap(px(16.))
                        .child(pro_skeleton_bar(440., 36., 8., 0.4, cx))
                        .child(pro_skeleton_bar(440., 34., 8., 0.5, cx))
                        .child(pro_skeleton_bar(440., 34., 8., 0.6, cx))
                        .child(pro_skeleton_bar(440., 36., 8., 0.7, cx))
                        .child(pro_skeleton_bar(440., 36., 8., 0.8, cx)),
                );
        }

        if self.empty {
            let cta: ProEmptyCta<Self> = (
                pro_eid(&self.id, "empty-defaults"),
                tr(cx, "Use recommended voice", "使用推荐语气配置").into(),
                Box::new(|this, _, cx| {
                    this.empty = false;
                    this.tone = OnboardingToneVoice::Neutral;
                    this.length = OnboardingReplyLength::Balanced;
                    this.handover = true;
                    cx.notify();
                }),
            );
            return div().w_full().max_w(px(440.)).child(pro_empty_state(
                ProIcon::Sparkles,
                tr(cx, "Voice settings unavailable", "尚未配置助手语气"),
                tr(
                    cx,
                    "Choose a name, conversational tone, reply length, and human escalation rule.",
                    "为助手设置名称、回复语气、长度与人工转交规则。",
                ),
                Some(cta),
                cx,
            ));
        }

        let name_focused = name_ed.read(cx).focus.is_focused(w);
        let name_empty = name_ed.read(cx).value().trim().is_empty();
        let email_val = email_ed.read(cx).value().to_owned();
        let email_focused = email_ed.read(cx).focus.is_focused(w);
        let handover_on = self.handover;
        let email_invalid =
            handover_on && !email_val.trim().is_empty() && !is_valid_handover_email(&email_val);

        let tone_target = match self.tone {
            OnboardingToneVoice::Friendly => 0.16,
            OnboardingToneVoice::Neutral => 0.50,
            OnboardingToneVoice::Formal => 0.84,
        };
        let tone_pos = pro_spring_f32(pro_eid(&self.id, "tone-axis-sp"), tone_target, 0.20, w, cx)
            .clamp(0., 1.);

        let len_target = match self.length {
            OnboardingReplyLength::Short => 0.16,
            OnboardingReplyLength::Balanced => 0.50,
            OnboardingReplyLength::Detailed => 0.84,
        };
        let len_pos =
            pro_spring_f32(pro_eid(&self.id, "len-axis-sp"), len_target, 0.20, w, cx).clamp(0., 1.);

        let sw_t = pro_spring_f32(
            pro_eid(&self.id, "handover-sw-sp"),
            f32::from(handover_on),
            0.20,
            w,
            cx,
        )
        .clamp(0., 1.);
        let name_focus_t = pro_spring_f32(
            pro_eid(&self.id, "tone-name-fsp"),
            f32::from(name_focused),
            0.18,
            w,
            cx,
        )
        .clamp(0., 1.);
        let email_focus_t = pro_spring_f32(
            pro_eid(&self.id, "tone-email-fsp"),
            f32::from(email_focused && handover_on),
            0.18,
            w,
            cx,
        )
        .clamp(0., 1.);

        // Progress dashes (.ui-obw-steps)
        let mut steps_bar = div().mb(px(20.)).flex().items_center().gap(px(4.));
        for idx in 1..=self.steps {
            let on = idx <= self.step;
            steps_bar = steps_bar.child(div().w(px(16.)).h(px(3.)).rounded(px(2.)).bg(if on {
                pal.dark_fill
            } else {
                pal.line
            }));
        }

        let name_field = div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(
                div()
                    .text_size(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(pal.ink)
                    .child(tr(cx, "Name", "名称")),
            )
            .child(
                div()
                    .h(px(36.))
                    .px(px(12.))
                    .rounded(px(8.))
                    .bg(pal.card)
                    .border_1()
                    .border_color(if name_empty {
                        pal.red.opacity(0.55)
                    } else {
                        pro_mix_color(pal.line, pal.line_firm, name_focus_t)
                    })
                    .flex()
                    .items_center()
                    .child(div().flex_1().min_w_0().h(px(20.)).child(name_ed)),
            );

        // Tone segmented radio group + spring axis slider (.ui-obt-seg)
        let active_tone = self.tone;
        let tone_sensor = pro_selection_sensor();
        let tone_seg = div()
            .id(pro_eid(&self.id, "tone-group"))
            .role(Role::Region)
            .aria_label(tr(cx, "Tone", "语气"))
            .relative()
            .p(px(2.))
            .rounded(px(8.))
            .bg(pal.line)
            .flex()
            .gap(px(2.))
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _, cx| {
                match ev.keystroke.key.as_str() {
                    "left" | "up" => {
                        this.set_tone(this.tone.step_by(-1), cx);
                        cx.stop_propagation();
                    }
                    "right" | "down" => {
                        this.set_tone(this.tone.step_by(1), cx);
                        cx.stop_propagation();
                    }
                    _ => {}
                }
            }))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "tone-seg-pill"),
                active_tone.as_str(),
                tone_sensor.clone(),
                6.,
                pal.card,
                Some(pal.line),
                !pal.dark,
            ))
            .children(OnboardingToneVoice::ALL.into_iter().map(|opt| {
                let checked = opt == active_tone;
                let lbl: SharedString = opt.label(cx).into();
                activate(
                    div()
                        .id(pro_eid(&self.id, format!("tone-{}", opt.as_str())))
                        .role(Role::RadioButton)
                        .aria_label(lbl.clone())
                        .tab_index(if checked { 0 } else { -1 })
                        .relative()
                        .flex_1()
                        .min_w_0()
                        .h(px(30.))
                        .px(px(10.))
                        .rounded(px(6.))
                        .when(checked, |d| {
                            d.text_color(pal.ink).child(pro_mark_selected(&tone_sensor))
                        })
                        .when(!checked, |d| {
                            d.text_color(pal.muted)
                                .hover(move |s| s.text_color(pal.ink))
                        })
                        .focus_visible(move |s| s.border_1().border_color(pal.focus))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(13.))
                        .font_weight(FontWeight::MEDIUM)
                        .cursor_pointer()
                        .child(lbl),
                    cx,
                    move |this, _, cx| this.set_tone(opt, cx),
                )
            }));

        let tone_slider = div()
            .relative()
            .w_full()
            .h(px(12.))
            .flex()
            .items_center()
            .child(
                div()
                    .w_full()
                    .h(px(4.))
                    .rounded_full()
                    .bg(pal.line)
                    .overflow_hidden()
                    .child(
                        div()
                            .h_full()
                            .w(relative(tone_pos))
                            .rounded_full()
                            .bg(pal.blue),
                    ),
            )
            .child(
                div()
                    .absolute()
                    .left(relative(tone_pos))
                    .top(px(1.))
                    .ml(px(-5.))
                    .size(px(10.))
                    .rounded_full()
                    .bg(pal.card)
                    .border_2()
                    .border_color(pal.blue),
            )
            .child(div().absolute().inset_0().flex().children(
                OnboardingToneVoice::ALL.into_iter().map(|opt| {
                    activate(
                        div()
                            .id(pro_eid(&self.id, format!("tone-rail-{}", opt.as_str())))
                            .flex_1()
                            .h_full()
                            .cursor_pointer(),
                        cx,
                        move |this, _, cx| this.set_tone(opt, cx),
                    )
                }),
            ));

        let tone_field = div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(6.))
            .child(
                div()
                    .text_size(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(pal.ink)
                    .child(tr(cx, "Tone", "语气")),
            )
            .child(tone_seg)
            .child(tone_slider);

        // Reply length segmented radio group + spring axis slider (.ui-obt-seg)
        let active_len = self.length;
        let len_sensor = pro_selection_sensor();
        let len_seg = div()
            .id(pro_eid(&self.id, "length-group"))
            .role(Role::Region)
            .aria_label(tr(cx, "Reply length", "回复长度"))
            .relative()
            .p(px(2.))
            .rounded(px(8.))
            .bg(pal.line)
            .flex()
            .gap(px(2.))
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _, cx| {
                match ev.keystroke.key.as_str() {
                    "left" | "up" => {
                        this.set_length(this.length.step_by(-1), cx);
                        cx.stop_propagation();
                    }
                    "right" | "down" => {
                        this.set_length(this.length.step_by(1), cx);
                        cx.stop_propagation();
                    }
                    _ => {}
                }
            }))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "len-seg-pill"),
                active_len.as_str(),
                len_sensor.clone(),
                6.,
                pal.card,
                Some(pal.line),
                !pal.dark,
            ))
            .children(OnboardingReplyLength::ALL.into_iter().map(|opt| {
                let checked = opt == active_len;
                let lbl: SharedString = opt.label(cx).into();
                activate(
                    div()
                        .id(pro_eid(&self.id, format!("len-{}", opt.as_str())))
                        .role(Role::RadioButton)
                        .aria_label(lbl.clone())
                        .tab_index(if checked { 0 } else { -1 })
                        .relative()
                        .flex_1()
                        .min_w_0()
                        .h(px(30.))
                        .px(px(10.))
                        .rounded(px(6.))
                        .when(checked, |d| {
                            d.text_color(pal.ink).child(pro_mark_selected(&len_sensor))
                        })
                        .when(!checked, |d| {
                            d.text_color(pal.muted)
                                .hover(move |s| s.text_color(pal.ink))
                        })
                        .focus_visible(move |s| s.border_1().border_color(pal.focus))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(13.))
                        .font_weight(FontWeight::MEDIUM)
                        .cursor_pointer()
                        .child(lbl),
                    cx,
                    move |this, _, cx| this.set_length(opt, cx),
                )
            }));

        let len_slider = div()
            .relative()
            .w_full()
            .h(px(12.))
            .flex()
            .items_center()
            .child(
                div()
                    .w_full()
                    .h(px(4.))
                    .rounded_full()
                    .bg(pal.line)
                    .overflow_hidden()
                    .child(
                        div()
                            .h_full()
                            .w(relative(len_pos))
                            .rounded_full()
                            .bg(pal.blue),
                    ),
            )
            .child(
                div()
                    .absolute()
                    .left(relative(len_pos))
                    .top(px(1.))
                    .ml(px(-5.))
                    .size(px(10.))
                    .rounded_full()
                    .bg(pal.card)
                    .border_2()
                    .border_color(pal.blue),
            )
            .child(div().absolute().inset_0().flex().children(
                OnboardingReplyLength::ALL.into_iter().map(|opt| {
                    activate(
                        div()
                            .id(pro_eid(&self.id, format!("len-rail-{}", opt.as_str())))
                            .flex_1()
                            .h_full()
                            .cursor_pointer(),
                        cx,
                        move |this, _, cx| this.set_length(opt, cx),
                    )
                }),
            ));

        let length_field = div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(6.))
            .child(
                div()
                    .text_size(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(pal.ink)
                    .child(tr(cx, "Reply length", "回复长度")),
            )
            .child(len_seg)
            .child(len_slider);

        // Handover switch + email field (.ui-obt-switch-row + .ui-obt-input)
        // Capsule rule (DESIGN.md): switch track and thumb keep rounded_full().
        let thumb_on_bg: Hsla = if pal.dark {
            rgb(0x111111).into()
        } else {
            rgb(0xffffff).into()
        };
        let thumb_bg = pro_mix_color(rgb(0xffffff).into(), thumb_on_bg, sw_t);
        let track_bg = pro_mix_color(pal.line, pal.dark_fill, sw_t);
        let switch_el = activate(
            div()
                .id(pro_eid(&self.id, "handover-switch"))
                .role(Role::Switch)
                .aria_label(tr(cx, "Hand over to a person", "转交给人工"))
                .tab_index(0)
                .w(px(32.))
                .h(px(20.))
                .relative()
                .flex_none()
                .rounded_full()
                .bg(track_bg)
                .cursor_pointer()
                .focus_visible(move |s| s.border_1().border_color(pal.focus))
                .child(
                    div()
                        .absolute()
                        .top(px(2.))
                        .left(px(2. + 12. * sw_t))
                        .size(px(16.))
                        .rounded_full()
                        .bg(thumb_bg),
                ),
            cx,
            |this, _, cx| {
                let next = !this.handover;
                this.set_handover(next, cx);
            },
        );

        let email_box_border = if email_invalid {
            pal.red
        } else {
            pro_mix_color(pal.line, pal.line_firm, email_focus_t)
        };
        let email_box_el = div()
            .h(px(36.))
            .px(px(12.))
            .rounded(px(8.))
            .bg(pal.card)
            .border_1()
            .border_color(email_box_border)
            .flex()
            .items_center()
            .child(div().flex_1().min_w_0().h(px(20.)).child(email_ed))
            .into_any_element();

        let handover_field = div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(16.))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(pal.ink)
                                    .child(tr(cx, "Hand over to a person", "转交给人工")),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .line_height(px(16.))
                                    .text_color(pal.faint)
                                    .child(tr(
                                        cx,
                                        "When it can't answer for sure",
                                        "当助手无法确定答案时",
                                    )),
                            ),
                    )
                    .child(switch_el),
            )
            .child(pro_fold(
                pro_eid(&self.id, "handover-email-fold"),
                handover_on,
                36.,
                8.,
                move || email_box_el,
            ));

        // Continue button (.ui-obt-go)
        let valid = self.can_submit();
        let go_lbl: SharedString = tr(cx, "Continue", "继续").into();
        let go_base = div()
            .id(pro_eid(&self.id, "continue"))
            .role(Role::Button)
            .aria_label(go_lbl.clone())
            .tab_index(if valid { 0 } else { -1 })
            .mt(px(4.))
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
            .when(!valid, |d| d.opacity(0.45))
            .when(valid, |d| {
                d.cursor_pointer()
                    .hover(move |s| s.opacity(0.88))
                    .focus_visible(move |s| s.border_1().border_color(pal.focus))
            })
            .child(go_lbl)
            .child(pro_icon(ProIcon::ArrowRight, 14., pal.on_dark));

        let go_btn = if valid {
            activate(go_base, cx, |this, _, cx| {
                let _ = this.submit(cx);
            })
            .into_any_element()
        } else {
            go_base.into_any_element()
        };

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
                    .child(tr(
                        cx,
                        "How should your assistant sound?",
                        "你的助手应该以什么语气交流？",
                    )),
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
                        "Give it a name, a voice and someone to ask.",
                        "为它起个名字、选好语气，并指定转交对象。",
                    )),
            )
            .child(
                div()
                    .w_full()
                    .mt(px(28.))
                    .flex()
                    .flex_col()
                    .gap(px(18.))
                    .text_left()
                    .child(name_field)
                    .child(tone_field)
                    .child(length_field)
                    .child(handover_field)
                    .child(go_btn),
            )
    }
}

impl Render for OnboardingToneScreen {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let pal = ObwTokens::resolve(cx);
        let reduced_motion = Theme::global(cx).reduced_motion;
        let now = Instant::now();
        let opened = *self.opened_at.get_or_insert(now);
        let elapsed_ms = now.duration_since(opened).as_secs_f32() * 1000.;

        let (name_ed, email_ed) = self.ensure_editors(w, cx);

        // Smooth glide down to -946px (Test chat section front and center)
        let target_offset_y = -946.0_f32;
        let offset_y = if reduced_motion || elapsed_ms >= 420. {
            target_offset_y
        } else {
            w.request_animation_frame();
            let t = pro_ease(elapsed_ms / 420.);
            -900.0 + (target_offset_y - -900.0) * t
        };

        let container_w = self.measured_width.get();
        let narrow = container_w <= 900.;
        let compact_side = container_w <= 1040.;
        let measure_cell = self.measured_width.clone();
        let live_name = name_ed.read(cx).value().to_owned();
        let live_email = email_ed.read(cx).value().to_owned();

        let preview = (!narrow).then(|| {
            self.render_dashboard_preview(
                container_w * 0.5,
                compact_side,
                offset_y,
                &live_name,
                &live_email,
                pal,
                cx,
            )
        });
        let top_bar = self.render_top_bar(narrow, pal, cx);
        let step_body = self.render_step_body(name_ed, email_ed, pal, w, cx);
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

fn obw_agent_mark(seed: &str, color: Hsla, full: bool) -> Div {
    let cells = if full {
        [true; 9]
    } else {
        agent_pixel_cells(seed)
    };
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

fn render_skeleton_tabs(pal: ObwTokens) -> Div {
    div()
        .px(px(16.))
        .py(px(10.))
        .border_b_1()
        .border_color(pal.line)
        .flex()
        .items_center()
        .gap(px(8.))
        .children([96.0_f32, 72., 112., 88.].into_iter().map(|w_lbl| {
            div()
                .h(px(28.))
                .pl(px(10.))
                .pr(px(12.))
                .rounded(px(7.))
                .border_1()
                .border_color(pal.line)
                .flex()
                .items_center()
                .gap(px(8.))
                .child(sk_sq(10., 3., pal.sk))
                .child(sk_bar(w_lbl, 8., pal.sk))
        }))
}

fn render_skeleton_kpis(pal: ObwTokens) -> Div {
    div()
        .flex()
        .gap(px(12.))
        .children([64.0_f32, 52., 72.].into_iter().map(|w_lbl| {
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
                .child(sk_bar(w_lbl, 8., pal.sk))
                .child(sk_sq_rect(44., 16., 5., pal.sk))
        }))
}

fn render_section(title: Option<&str>, card: Div, pal: ObwTokens) -> Div {
    let head = div()
        .min_h(px(28.))
        .flex()
        .flex_col()
        .gap(px(8.))
        .child(if let Some(t) = title {
            div()
                .text_size(px(14.))
                .line_height(px(14.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(pal.ink)
                .child(t.to_owned())
        } else {
            sk_bar(68., 10., pal.sk)
        })
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

fn render_knowledge_table(domain: &str, initial: &str, pal: ObwTokens, cx: &App) -> Div {
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

    let first_row = div()
        .h(px(34.))
        .border_b_1()
        .border_color(pal.line)
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
                        .child(initial.to_owned()),
                )
                .child(
                    div()
                        .min_w_0()
                        .truncate()
                        .text_size(px(13.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(pal.ink)
                        .child(domain.to_owned()),
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
        ));

    let mut tbl = div()
        .rounded(px(10.))
        .border_1()
        .border_color(pal.line)
        .overflow_hidden()
        .flex()
        .flex_col()
        .child(header)
        .child(first_row);

    for idx in 0..5usize {
        tbl = tbl.child(
            div()
                .h(px(34.))
                .when(idx < 4, |d| d.border_b_1().border_color(pal.line))
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
                )),
        );
    }
    tbl
}

fn render_trailing_skeleton_table(pal: ObwTokens) -> Div {
    let mut tbl = div()
        .rounded(px(10.))
        .border_1()
        .border_color(pal.line)
        .overflow_hidden()
        .flex()
        .flex_col();
    for idx in 0..6usize {
        tbl = tbl.child(
            div()
                .h(px(34.))
                .when(idx < 5, |d| d.border_b_1().border_color(pal.line))
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
                        .child(sk_bar(118., 8., pal.sk)),
                ))
                .child(table_cell(Some(120.), false, pal, sk_bar(48., 8., pal.sk)))
                .child(table_cell(None, false, pal, sk_bar(88., 8., pal.sk))),
        );
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
        OnboardingReplyLength, OnboardingToneEvent, OnboardingToneScreen, OnboardingToneVoice,
        is_valid_handover_email, sample_test_reply,
    };
    use gpui::TestAppContext;
    use std::{cell::RefCell, rc::Rc};

    #[test]
    fn handover_email_validation_and_tone_replies_cover_matrix() {
        assert!(is_valid_handover_email("support@northwind.com"));
        assert!(is_valid_handover_email("  help.desk@fieldwork.co.uk "));
        assert!(!is_valid_handover_email(""));
        assert!(!is_valid_handover_email("support@northwind"));
        assert!(!is_valid_handover_email("@northwind.com"));
        assert!(!is_valid_handover_email("support @northwind.com"));

        assert_eq!(
            sample_test_reply(
                OnboardingToneVoice::Neutral,
                OnboardingReplyLength::Balanced
            ),
            "Yes. You can change your plan at any time; the difference is prorated."
        );
        assert_ne!(
            sample_test_reply(OnboardingToneVoice::Friendly, OnboardingReplyLength::Short),
            sample_test_reply(OnboardingToneVoice::Formal, OnboardingReplyLength::Detailed)
        );
    }

    #[test]
    fn tone_form_blocks_invalid_handover_email_until_disabled_or_fixed() {
        let mut app = TestAppContext::single();
        app.update(crate::init);

        let events: Rc<RefCell<Vec<OnboardingToneEvent>>> = Rc::new(RefCell::new(Vec::new()));
        let captured = events.clone();

        let (view, w) = app.add_window_view(|_w, cx| {
            OnboardingToneScreen::new("test-obw-tone", cx)
                .with_domain("fieldwork.io")
                .with_brand("Fieldwork")
        });

        w.update(|_w, cx| {
            cx.subscribe(&view, move |_, ev: &OnboardingToneEvent, _| {
                captured.borrow_mut().push(ev.clone());
            })
            .detach();
            view.update(cx, |screen, cx| {
                let initial = screen.values();
                assert_eq!(initial.name.as_ref(), "Fieldwork Assistant");
                assert_eq!(initial.email.as_ref(), "support@fieldwork.io");
                assert!(screen.can_submit());

                // Invalid email blocks submission while handover is enabled
                screen.set_handover_email("invalid-address", cx);
                assert!(!screen.can_submit());
                assert!(!screen.submit(cx));

                // Turning off handover permits submission even with invalid email
                screen.set_tone(OnboardingToneVoice::Friendly, cx);
                screen.set_length(OnboardingReplyLength::Short, cx);
                screen.set_handover(false, cx);
                assert!(screen.can_submit());
                assert!(screen.submit(cx));
            });
        });

        let log = events.borrow();
        assert!(log.iter().any(|ev| matches!(
            ev,
            OnboardingToneEvent::Continue(vals)
                if vals.tone == OnboardingToneVoice::Friendly
                    && vals.length == OnboardingReplyLength::Short
                    && !vals.handover
        )));
    }
}
