//! Split-screen website onboarding step with domain chips and live knowledge dashboard preview.
//!
//! Renders the first-step AI assistant setup wizard: a full-bleed two-column layout
//! with account chrome and a glowing multi-radial domain chip field on the left, paired
//! with a vertically glided decorative assistant dashboard on the right where each
//! added domain lands live in the Knowledge table.

use crate::{
    components::pro_chrome::{
        pro_ease, pro_eid, pro_empty_state, pro_fold, pro_mark_selected, pro_mix_color,
        pro_pill_indicator, pro_presence, pro_selection_sensor, pro_skeleton_bar, pro_spring_bar,
        pro_spring_f32, pro_view_transition, tr,
    },
    foundation::{Theme, ThemeMode},
    primitives::{
        control::activate,
        editor::{EditEvent, Editor, InputMode},
        pro_icons::{AGENT_WAVE_MS, ProIcon, agent_pixel_cells, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

/// Clean geometric product aperture mark (`20x20` default, no external brand marks).
const LUMEN_MARK_SVG: &str = r#"<path d="M10.5 2.5H19C20.3807 2.5 21.5 3.61929 21.5 5V13.5C21.5 14.8807 20.3807 16 19 16H16V19C16 20.3807 14.8807 21.5 13.5 21.5H5C3.61929 21.5 2.5 20.3807 2.5 19V10.5C2.5 9.11929 3.61929 8 5 8H8V5C8 3.61929 9.11929 2.5 10.5 2.5ZM9.25 9.25V13.5C9.25 14.1904 9.80964 14.75 10.5 14.75H14.75V10.5C14.75 9.80964 14.1904 9.25 13.5 9.25H9.25Z" fill="currentColor"></path>"#;

const LOGOUT_SVG: &str = r#"<path d="M18 18C18 18.4644 18 18.6965 17.978 18.8918C17.7952 20.5145 16.5145 21.7952 14.8919 21.978C14.6965 22 14.4644 22 14 22H11C7.70017 22 6.05025 22 5.02513 20.9749C4 19.9498 4 18.2998 4 15L4.00001 8.99998C4.00001 5.70016 4.00003 4.05024 5.02514 3.02512C6.05027 2 7.70018 2 11 2H14C14.4644 2 14.6965 2 14.8919 2.02201C16.5145 2.20483 17.7952 3.48547 17.978 5.10816C18 5.30347 18 5.53565 18 6" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.6"></path><path d="M8.07612 11.1183C8 11.3021 8 11.535 8 12.001C8 12.4669 8 12.6999 8.07612 12.8837C8.17761 13.1287 8.37229 13.3234 8.61732 13.4249C8.80109 13.501 9.03406 13.501 9.5 13.501H14.5C14.5002 15.2503 14.511 16.1299 15.0623 16.3858C15.0829 16.3954 15.1037 16.4042 15.1249 16.4124C15.7045 16.6353 16.3999 16.0139 17.7907 14.7711C19.2576 13.4602 19.9912 12.7851 20 11.957C19.9912 11.1289 19.2576 10.4538 17.7907 9.14301C16.3999 7.90018 15.7045 7.27876 15.1249 7.50171C15.1037 7.50985 15.0829 7.51869 15.0623 7.52822C14.5018 7.78844 14.5 8.69318 14.5 10.501H9.5C9.03406 10.501 8.80109 10.501 8.61732 10.5771C8.37229 10.6786 8.17761 10.8733 8.07612 11.1183Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.6"></path>"#;

const HELP_CIRCLE_SVG: &str = r#"<circle cx="12" cy="12" r="10" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.6"></circle><path d="M9.5 9.5C9.5 8.11929 10.6193 7 12 7C13.3807 7 14.5 8.11929 14.5 9.5C14.5 10.3569 14.0689 11.1131 13.4117 11.5636C12.7283 12.0319 12 12.6716 12 13.5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.6"></path><path d="M12.125 16.75H12M12.25 16.75C12.25 16.8881 12.1381 17 12 17C11.8619 17 11.75 16.8881 11.75 16.75C11.75 16.6119 11.8619 16.5 12 16.5C12.1381 16.5 12.25 16.6119 12.25 16.75Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.6"></path>"#;

/// Bar chart heights (28 buckets, 0.0..=1.0) matching the decorative dashboard plot.
const CHART_BARS: [f32; 28] = [
    0.34, 0.43, 0.38, 0.51, 0.46, 0.60, 0.53, 0.45, 0.64, 0.70, 0.62, 0.74, 0.68, 0.55, 0.64, 0.76,
    0.83, 0.74, 0.66, 0.79, 0.87, 0.81, 0.72, 0.85, 0.91, 0.83, 0.95, 0.89,
];

/// Framing style for the decorative right-hand dashboard window.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OnboardingWebsiteLayout {
    /// Rounded main canvas inset on a tinted window surface.
    #[default]
    Inset,
    /// Edge-to-edge sidebar and main body split by single hairlines.
    Flat,
}

/// Events emitted by [`OnboardingWebsiteScreen`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OnboardingWebsiteEvent {
    DomainAdded(SharedString),
    DomainRemoved(SharedString),
    DomainsChanged(Vec<SharedString>),
    Continue(Vec<SharedString>),
    Back,
    Logout,
    OpenLegal(SharedString),
}

impl EventEmitter<OnboardingWebsiteEvent> for OnboardingWebsiteScreen {}

/// Normalize user-entered URL or hostname input to a bare lowercase domain.
///
/// Strips `http://` / `https://` schemes, userinfo when preceded by a scheme,
/// leading `www.`, numeric ports, trailing paths/query strings/fragments, and
/// surrounding whitespace or delimiter punctuation.
pub fn normalize_domain(raw: &str) -> String {
    let mut s = raw.trim().trim_matches(|c| c == ',' || c == ';').trim();
    if s.is_empty() {
        return String::new();
    }
    let lower = s.to_ascii_lowercase();
    s = lower.as_str();

    let mut had_scheme = false;
    let host_and_rest = if let Some((_, rest)) = s.split_once("://") {
        had_scheme = true;
        rest
    } else {
        s
    };

    let host_part = host_and_rest.split(['/', '?', '#']).next().unwrap_or("");

    let without_auth = if had_scheme {
        host_part
            .rsplit_once('@')
            .map_or(host_part, |(_, host)| host)
    } else {
        host_part
    };

    let without_port = if let Some((host, port)) = without_auth.rsplit_once(':')
        && !port.is_empty()
        && port.chars().all(|c| c.is_ascii_digit())
    {
        host
    } else {
        without_auth
    };

    let without_www = without_port.strip_prefix("www.").unwrap_or(without_port);

    without_www.trim_end_matches('.').to_string()
}

/// Validate whether `domain` (after [`normalize_domain`]) is a well-formed DNS hostname.
pub fn is_valid_domain(domain: &str) -> bool {
    if domain.is_empty() || domain.len() > 253 {
        return false;
    }
    if domain.contains(char::is_whitespace)
        || domain.contains('@')
        || domain.contains('/')
        || domain.contains(':')
    {
        return false;
    }
    let labels: Vec<&str> = domain.split('.').collect();
    if labels.len() < 2 {
        return false;
    }
    for label in &labels {
        if label.is_empty() || label.len() > 63 {
            return false;
        }
        if label.starts_with('-') || label.ends_with('-') {
            return false;
        }
        if !label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return false;
        }
    }
    let Some(&tld) = labels.last() else {
        return false;
    };
    if let Some(puny) = tld.strip_prefix("xn--") {
        puny.len() >= 2 && puny.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
    } else {
        tld.len() >= 2 && tld.chars().all(|c| c.is_ascii_alphabetic())
    }
}

fn infer_source_kind<'a>(domain: &str, cx: &App) -> &'a str {
    if domain.starts_with("docs.") || domain.starts_with("dev.") || domain.starts_with("developer.")
    {
        tr(cx, "Docs", "文档站")
    } else if domain.starts_with("help.")
        || domain.starts_with("support.")
        || domain.starts_with("kb.")
    {
        tr(cx, "Help center", "帮助中心")
    } else {
        tr(cx, "Website", "官网")
    }
}

fn derive_brand_title(domain: &str) -> String {
    let labels: Vec<&str> = domain.split('.').collect();
    let core = if labels.len() >= 3
        && matches!(
            labels[0],
            "docs" | "help" | "support" | "dev" | "developer" | "kb" | "blog" | "app"
        ) {
        labels[1]
    } else {
        labels.first().copied().unwrap_or("Workspace")
    };
    let mut chars = core.chars();
    match chars.next() {
        Some(first) => {
            let mut title: String = first.to_uppercase().collect();
            title.push_str(chars.as_str());
            title
        }
        None => "Workspace".to_string(),
    }
}

fn domain_page_estimate(domain: &str) -> u32 {
    let mut h: u32 = 19;
    for b in domain.bytes() {
        h = h.wrapping_mul(31).wrapping_add(u32::from(b));
    }
    24 + (h % 140)
}

#[derive(Clone, Copy)]
struct ObwTokens {
    dark: bool,
    bg: Hsla,
    page: Hsla,
    card: Hsla,
    side: Hsla,
    sk: Hsla,
    line: Hsla,
    line_firm: Hsla,
    ink: Hsla,
    muted: Hsla,
    faint: Hsla,
    dark_fill: Hsla,
    on_dark: Hsla,
    blue: Hsla,
    purple: Hsla,
    pink: Hsla,
    red: Hsla,
    green: Hsla,
    green_ink: Hsla,
    green_soft: Hsla,
    focus: Hsla,
}

impl ObwTokens {
    fn resolve(cx: &App) -> Self {
        let dark = Theme::global(cx).mode == ThemeMode::Dark;
        if dark {
            Self {
                dark: true,
                bg: rgb(0x0f0f10).into(),
                page: rgb(0x0e0e10).into(),
                card: rgb(0x18181a).into(),
                side: hsla(0., 0., 1., 0.018),
                sk: hsla(0., 0., 1., 0.06),
                line: hsla(0., 0., 1., 0.07),
                line_firm: hsla(0., 0., 1., 0.18),
                ink: hsla(0., 0., 1., 0.92),
                muted: hsla(0., 0., 1., 0.60),
                faint: hsla(0., 0., 1., 0.42),
                dark_fill: hsla(0., 0., 1., 0.92),
                on_dark: rgb(0x111111).into(),
                blue: rgb(0x4fb0fe).into(),
                purple: rgb(0xb98af0).into(),
                pink: rgb(0xf06db5).into(),
                red: rgb(0xff8a7a).into(),
                green: rgb(0x5fc88a).into(),
                green_ink: rgb(0x7dcf9f).into(),
                green_soft: hsla(145. / 360., 0.46, 0.65, 0.14),
                focus: rgba(0x4d8dffcc).into(),
            }
        } else {
            Self {
                dark: false,
                bg: rgb(0xffffff).into(),
                page: rgb(0xf8f8f8).into(),
                card: rgb(0xffffff).into(),
                side: hsla(0., 0., 0., 0.018),
                sk: hsla(0., 0., 0., 0.05),
                line: hsla(0., 0., 0., 0.06),
                line_firm: hsla(0., 0., 0., 0.14),
                ink: hsla(0., 0., 0., 0.88),
                muted: hsla(0., 0., 0., 0.61),
                faint: hsla(0., 0., 0., 0.45),
                dark_fill: hsla(0., 0., 0., 0.88),
                on_dark: rgb(0xfcfcfc).into(),
                blue: rgb(0x0190fe).into(),
                purple: rgb(0xa651f0).into(),
                pink: rgb(0xee3e9e).into(),
                red: rgb(0xd93a2b).into(),
                green: rgb(0x5fc88a).into(),
                green_ink: rgb(0x1e8a4f).into(),
                green_soft: hsla(145. / 360., 0.48, 0.58, 0.14),
                focus: rgba(0x0060e2b3).into(),
            }
        }
    }
}

/// First-step website and knowledge-domain wizard screen with live dashboard preview.
pub struct OnboardingWebsiteScreen {
    id: ElementId,
    focus: FocusHandle,
    email: SharedString,
    product: SharedString,
    layout: OnboardingWebsiteLayout,
    step: u8,
    steps: u8,
    show_back: bool,
    auto_focus: bool,
    did_auto_focus: bool,
    auto_type: Option<SharedString>,
    auto_type_stopped: bool,
    loading: bool,
    empty: bool,
    domains: Vec<SharedString>,
    invalid_error: Option<SharedString>,
    last_invalid_error: SharedString,
    crawl_details_open: bool,
    invalid_ink: Rc<Cell<Option<Hsla>>>,
    domain_editor: Option<Entity<Editor>>,
    editor_focused: bool,
    measured_width: Rc<Cell<f32>>,
    started_at: Instant,
    last_domain_anim: Option<Instant>,
}

impl OnboardingWebsiteScreen {
    /// Construct a new [`OnboardingWebsiteScreen`] for `"alex@northwind.com"` on `"Lumen"`.
    pub fn new(id: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        Self {
            id: ElementId::Name(id.into()),
            focus: cx.focus_handle(),
            email: "alex@northwind.com".into(),
            product: "Lumen".into(),
            layout: OnboardingWebsiteLayout::Inset,
            step: 1,
            steps: 4,
            show_back: false,
            auto_focus: false,
            did_auto_focus: false,
            auto_type: None,
            auto_type_stopped: false,
            loading: false,
            empty: false,
            domains: Vec::new(),
            invalid_error: None,
            last_invalid_error: "".into(),
            crawl_details_open: true,
            invalid_ink: Rc::new(Cell::new(None)),
            domain_editor: None,
            editor_focused: false,
            measured_width: Rc::new(Cell::new(1280.)),
            started_at: Instant::now(),
            last_domain_anim: None,
        }
    }

    /// Set the signed-in account email displayed in the top-right bar.
    pub fn with_email(mut self, email: impl Into<SharedString>) -> Self {
        self.email = email.into();
        self
    }

    /// Set the product brand label used for the top-left mark and copyright footer.
    pub fn with_product(mut self, product: impl Into<SharedString>) -> Self {
        self.product = product.into();
        self
    }

    /// Choose `Inset` or `Flat` framing for the decorative dashboard preview.
    pub fn with_layout(mut self, layout: OnboardingWebsiteLayout) -> Self {
        self.layout = layout;
        self
    }

    /// Configure the 1-based step indicator (`step` out of `steps` dashes).
    pub fn with_step(mut self, step: u8, steps: u8) -> Self {
        let total = steps.max(1);
        self.steps = total;
        self.step = step.clamp(1, total);
        self
    }

    /// Show or hide the `Back` button in the top bar.
    pub fn with_show_back(mut self, show_back: bool) -> Self {
        self.show_back = show_back;
        self
    }

    /// Pre-populate known company or documentation domains (normalized and deduplicated).
    pub fn with_domains<I, S>(mut self, domains: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        self.domains.clear();
        for item in domains {
            let shared: SharedString = item.into();
            let norm = normalize_domain(&shared);
            if is_valid_domain(&norm) && !self.domains.iter().any(|d| d.eq_ignore_ascii_case(&norm))
            {
                self.domains.push(norm.into());
            }
        }
        self
    }

    /// Automatically focus the domain input field when first rendered.
    pub fn with_auto_focus(mut self, auto_focus: bool) -> Self {
        self.auto_focus = auto_focus;
        self
    }

    /// Type `sample` into the domain field once on load for demos; stops on user input.
    pub fn with_auto_type(mut self, sample: impl Into<SharedString>) -> Self {
        self.auto_type = Some(sample.into());
        self
    }

    /// Configure initial loading skeleton state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Configure initial empty state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Toggle loading state at runtime.
    pub fn set_loading(&mut self, loading: bool, cx: &mut Context<Self>) {
        if self.loading != loading {
            self.loading = loading;
            cx.notify();
        }
    }

    /// Return whether loading skeletons are active.
    pub fn is_loading(&self) -> bool {
        self.loading
    }

    /// Toggle empty state at runtime.
    pub fn set_empty(&mut self, empty: bool, cx: &mut Context<Self>) {
        if self.empty != empty {
            self.empty = empty;
            cx.notify();
        }
    }

    /// Return whether the empty state is active.
    pub fn is_empty(&self) -> bool {
        self.empty
    }

    /// Return the normalized domains currently added as chips.
    pub fn domains(&self) -> &[SharedString] {
        &self.domains
    }

    /// Return the current validation error message, if the last entry was invalid.
    pub fn invalid_message(&self) -> Option<&SharedString> {
        self.invalid_error.as_ref()
    }

    /// Normalize, validate, and add `raw` to the domain chip list.
    ///
    /// # Errors
    ///
    /// Returns a human-readable validation message and sets the polite error hint
    /// when `raw` does not normalize to a valid hostname.
    pub fn try_add_domain(
        &mut self,
        raw: &str,
        cx: &mut Context<Self>,
    ) -> Result<SharedString, SharedString> {
        let norm = normalize_domain(raw);
        if norm.is_empty() || !is_valid_domain(&norm) {
            let err: SharedString = tr(
                cx,
                "Enter a valid domain like northwind.com or docs.northwind.com",
                "请输入有效域名，例如 northwind.com 或 docs.northwind.com",
            )
            .into();
            self.last_invalid_error = err.clone();
            self.invalid_error = Some(err.clone());
            cx.notify();
            return Err(err);
        }
        self.invalid_error = None;
        if let Some(existing) = self
            .domains
            .iter()
            .find(|d| d.eq_ignore_ascii_case(&norm))
            .cloned()
        {
            cx.notify();
            return Ok(existing);
        }
        let added: SharedString = norm.into();
        self.domains.push(added.clone());
        self.last_domain_anim = Some(Instant::now());
        cx.emit(OnboardingWebsiteEvent::DomainAdded(added.clone()));
        cx.emit(OnboardingWebsiteEvent::DomainsChanged(self.domains.clone()));
        cx.notify();
        Ok(added)
    }

    /// Remove a domain chip by index.
    pub fn remove_domain(&mut self, index: usize, cx: &mut Context<Self>) -> Option<SharedString> {
        if index < self.domains.len() {
            let removed = self.domains.remove(index);
            self.invalid_error = None;
            self.last_domain_anim = Some(Instant::now());
            cx.emit(OnboardingWebsiteEvent::DomainRemoved(removed.clone()));
            cx.emit(OnboardingWebsiteEvent::DomainsChanged(self.domains.clone()));
            cx.notify();
            Some(removed)
        } else {
            None
        }
    }

    /// Remove the most recently added domain chip (used by `Backspace` on an empty input).
    pub fn pop_last_domain(&mut self, cx: &mut Context<Self>) -> Option<SharedString> {
        if self.domains.is_empty() {
            None
        } else {
            self.remove_domain(self.domains.len() - 1, cx)
        }
    }

    /// Submit and continue with the normalized domain list if at least one valid domain is present.
    pub fn submit_continue(&mut self, w: Option<&Window>, cx: &mut Context<Self>) -> bool {
        let draft_raw = self
            .domain_editor
            .as_ref()
            .map(|e| e.read(cx).value().trim().to_string())
            .unwrap_or_default();

        if !draft_raw.is_empty() {
            if self.try_add_domain(&draft_raw, cx).is_err() {
                return false;
            }
            if let (Some(win), Some(ed)) = (w, &self.domain_editor) {
                ed.update(cx, |editor, cx| editor.set_value("", win, cx));
            }
        }

        if self.domains.is_empty() {
            let err: SharedString = tr(
                cx,
                "Add your company website domain to continue.",
                "请先添加至少一个公司网站域名后再继续。",
            )
            .into();
            self.last_invalid_error = err.clone();
            self.invalid_error = Some(err);
            cx.notify();
            return false;
        }

        self.invalid_error = None;
        cx.emit(OnboardingWebsiteEvent::Continue(self.domains.clone()));
        cx.notify();
        true
    }

    fn placeholder_domain(&self) -> SharedString {
        if let Some((_, host)) = self.email.split_once('@') {
            let clean = normalize_domain(host);
            if is_valid_domain(&clean) {
                return clean.into();
            }
        }
        "northwind.com".into()
    }

    fn ensure_editor(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.domain_editor.is_none() {
            let ph = self.placeholder_domain();
            let invalid_cell = self.invalid_ink.clone();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(
                    tr(cx, "Website or docs domain", "网站或文档域名"),
                    InputMode::Single,
                    w,
                    cx,
                );
                e.set_placeholder(ph);
                e.font_size = 13.;
                e.set_run_styler(Rc::new(move |_text, base_run| {
                    let mut run = base_run.clone();
                    if let Some(err_color) = invalid_cell.get() {
                        run.color = err_color;
                    }
                    vec![run]
                }));
                e
            });

            cx.subscribe_in(&ed, w, |this, e, ev: &EditEvent, w, cx| match ev {
                EditEvent::Focused => {
                    this.editor_focused = true;
                    cx.notify();
                }
                EditEvent::Blurred => {
                    this.editor_focused = false;
                    cx.notify();
                }
                EditEvent::Changed => {
                    this.auto_type_stopped = true;
                    let val = e.read(cx).value().to_string();
                    if val.ends_with(',') || val.ends_with(' ') {
                        let token = val.trim_end_matches([',', ' ']).trim().to_string();
                        if token.is_empty() {
                            e.update(cx, |editor, cx| editor.set_value("", w, cx));
                        } else if this.try_add_domain(&token, cx).is_ok() {
                            e.update(cx, |editor, cx| editor.set_value("", w, cx));
                        } else {
                            e.update(cx, |editor, cx| editor.set_value(token, w, cx));
                        }
                    } else if this.invalid_error.is_some() {
                        this.invalid_error = None;
                        cx.notify();
                    } else {
                        cx.notify();
                    }
                }
                EditEvent::Submitted => {
                    this.auto_type_stopped = true;
                    let val = e.read(cx).value().trim().to_string();
                    if val.is_empty() {
                        this.submit_continue(Some(w), cx);
                    } else if this.try_add_domain(&val, cx).is_ok() {
                        e.update(cx, |editor, cx| editor.set_value("", w, cx));
                    }
                }
                _ => {}
            })
            .detach();
            self.domain_editor = Some(ed);
        }

        if self.auto_focus
            && !self.did_auto_focus
            && let Some(ed) = &self.domain_editor
        {
            self.did_auto_focus = true;
            {
                let fh = ed.read(cx).focus.clone();
                fh.focus(w, cx);
            }
        }

        if let Some(sample) = self.auto_type.clone()
            && !self.auto_type_stopped
            && let Some(ed) = self.domain_editor.clone()
        {
            let elapsed_ms = self.started_at.elapsed().as_secs_f32() * 1000.0;
            let chars: Vec<char> = sample.chars().collect();
            let step_chars = ((elapsed_ms / 70.0).floor() as usize).min(chars.len());
            let partial: String = chars.iter().take(step_chars).collect();
            if step_chars < chars.len() {
                if ed.read(cx).value() != partial {
                    ed.update(cx, |editor, cx| {
                        editor.set_value(partial, w, cx);
                    });
                    // Keep auto-type active across programmatic updates.
                    self.auto_type_stopped = false;
                }
                if !Theme::global(cx).reduced_motion {
                    w.request_animation_frame();
                }
            } else {
                self.auto_type_stopped = true;
                if self.try_add_domain(&sample, cx).is_ok() {
                    ed.update(cx, |editor, cx| editor.set_value("", w, cx));
                }
            }
        }
    }

    fn render_sk_bar(t: ObwTokens, width: f32, height: f32, radius: f32) -> Div {
        div()
            .w(px(width))
            .h(px(height))
            .flex_none()
            .rounded(px(radius))
            .bg(t.sk)
    }

    fn render_agent_mark(
        seed: &str,
        color: Hsla,
        full: bool,
        busy: bool,
        elapsed_ms: f32,
        reduced: bool,
    ) -> Div {
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
                let idx = r * 3 + c;
                let on = cells[idx];
                let alpha = if busy && !reduced {
                    let phase = ((elapsed_ms + AGENT_WAVE_MS[idx]) / 1200.) * std::f32::consts::TAU;
                    let wave = 0.60 + 0.40 * phase.cos();
                    if on { wave } else { 0.16 * wave }
                } else if on {
                    1.0
                } else {
                    0.0
                };
                row_el = row_el.child(
                    div()
                        .size(px(3.))
                        .rounded(px(0.75))
                        .bg(color.opacity(alpha)),
                );
            }
            grid = grid.child(row_el);
        }
        grid
    }

    fn render_monogram_tile(t: ObwTokens, text: &str, side: f32, font_px: f32) -> Div {
        let ch: String = text
            .chars()
            .find(|c| c.is_ascii_alphanumeric())
            .unwrap_or('L')
            .to_uppercase()
            .collect();
        div()
            .size(px(side))
            .flex_none()
            .rounded(px(4.))
            .bg(t.dark_fill)
            .text_color(t.on_dark)
            .text_size(px(font_px))
            .font_weight(FontWeight::MEDIUM)
            .flex()
            .items_center()
            .justify_center()
            .child(ch)
    }

    fn render_dashboard_preview(
        &self,
        t: ObwTokens,
        container_w: f32,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let reduced = Theme::global(cx).reduced_motion;
        let elapsed_ms = self.started_at.elapsed().as_secs_f32() * 1000.0;
        let has_domains = !self.domains.is_empty();
        let compact_side = container_w <= 1180.;
        let is_flat = self.layout == OnboardingWebsiteLayout::Flat;

        let domains_key: SharedString = self
            .domains
            .iter()
            .map(SharedString::as_ref)
            .collect::<Vec<_>>()
            .join(",")
            .into();
        let primary_key: SharedString = self
            .domains
            .first()
            .cloned()
            .unwrap_or_else(|| "none".into());

        let pulse_alpha = if reduced {
            1.0
        } else {
            w.request_animation_frame();
            0.675 + 0.325 * ((elapsed_ms / 1600.0) * std::f32::consts::TAU).cos()
        };

        // Smooth spring vertical glide so the Knowledge table sits squarely in the viewport.
        let base_target_y = -620.0_f32 - (self.domains.len().min(4) as f32 * 6.0);
        let spring_y = pro_spring_f32(
            pro_eid(&self.id, "preview-y-sp"),
            base_target_y,
            0.34,
            w,
            cx,
        );
        let glide_y = if reduced {
            base_target_y
        } else {
            let intro_t = (self.started_at.elapsed().as_secs_f32() / 0.95).clamp(0., 1.);
            if intro_t < 1.0 {
                w.request_animation_frame();
            }
            let intro = pro_ease(intro_t);
            -548.0 + (spring_y - (-548.0)) * intro
        };

        let kpi_sources = pro_spring_f32(
            pro_eid(&self.id, "kpi-sources-sp"),
            self.domains.len() as f32,
            0.24,
            w,
            cx,
        )
        .max(0.);

        let scan_ratio = if !has_domains {
            0.0
        } else if let Some(anim_at) = self.last_domain_anim {
            let dt = (anim_at.elapsed().as_secs_f32() / 0.70).clamp(0., 1.);
            if !reduced && dt < 1.0 {
                w.request_animation_frame();
                0.45 + 0.55 * pro_ease(dt)
            } else {
                1.0
            }
        } else {
            1.0
        };

        // Decorative sidebar
        let ws_header = if let Some(primary) = self.domains.first() {
            let brand = derive_brand_title(primary);
            div()
                .h(px(32.))
                .px(px(if compact_side { 0. } else { 8. }))
                .rounded(px(8.))
                .flex()
                .items_center()
                .when(compact_side, |d| d.justify_center())
                .gap(px(8.))
                .child(Self::render_monogram_tile(t, &brand, 18., 10.5))
                .when(!compact_side, |d| {
                    d.child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(t.ink)
                            .truncate()
                            .child(brand),
                    )
                    .child(div().size(px(4.)).rounded_full().bg(t.faint.opacity(0.6)))
                })
        } else {
            div()
                .h(px(32.))
                .px(px(if compact_side { 0. } else { 8. }))
                .rounded(px(8.))
                .flex()
                .items_center()
                .when(compact_side, |d| d.justify_center())
                .gap(px(8.))
                .child(Self::render_sk_bar(t, 16., 16., 4.))
                .when(!compact_side, |d| {
                    d.child(Self::render_sk_bar(t, 64., 8., 4.))
                        .child(div().size(px(4.)).rounded_full().bg(t.faint.opacity(0.6)))
                })
        };

        let nav_groups_widths: [&[f32]; 7] = [
            &[76., 68.],
            &[60., 76.],
            &[80., 72.],
            &[56., 64.],
            &[84., 70., 60.],
            &[72., 64.],
            &[80., 56., 68.],
        ];

        let mut side_col = div()
            .w(px(if compact_side { 52. } else { 232. }))
            .flex_none()
            .px(px(if compact_side { 8. } else { 12. }))
            .py(px(10.))
            .when(is_flat, |d| d.bg(t.side).border_r_1().border_color(t.line))
            .flex()
            .flex_col()
            .gap(px(22.))
            .child(pro_view_transition(
                pro_eid(&self.id, "ws-head-tx"),
                primary_key.clone(),
                ws_header,
            ));

        for widths in nav_groups_widths {
            let mut grp = div()
                .px(px(if compact_side { 0. } else { 4. }))
                .flex()
                .flex_col()
                .when(compact_side, |d| d.items_center())
                .gap(px(12.));
            if !compact_side {
                grp = grp.child(Self::render_sk_bar(t, 30., 6., 3.));
            }
            for &w_bar in widths {
                grp = grp.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(10.))
                        .child(Self::render_sk_bar(t, 10., 10., 3.))
                        .when(!compact_side, |d| {
                            d.child(Self::render_sk_bar(t, w_bar, 8., 4.))
                        }),
                );
            }
            side_col = side_col.child(grp);
        }

        // Decorative body: top bar + tabs + KPIs + agent + chart + Knowledge table + chat + pair
        let body_bar = div()
            .h(px(48.))
            .flex_none()
            .px(px(16.))
            .border_b_1()
            .border_color(t.line)
            .flex()
            .items_center()
            .gap(px(12.))
            .child(pro_icon(ProIcon::SidebarLeft, 15., t.faint))
            .child(if has_domains {
                div()
                    .text_size(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(t.muted)
                    .child(tr(cx, "Knowledge", "知识库"))
            } else {
                Self::render_sk_bar(t, 88., 8., 4.)
            })
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
                            .bg(t.side)
                            .border_1()
                            .border_color(t.line)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(pro_icon(ProIcon::Thread, 14., t.faint)),
                    )
                    .child(
                        div()
                            .size(px(28.))
                            .rounded(px(6.))
                            .bg(t.side)
                            .border_1()
                            .border_color(t.line)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(pro_svg("0 0 24 24", HELP_CIRCLE_SVG, 14., t.faint)),
                    ),
            );

        let tab_specs: [(f32, Option<&str>, bool); 4] = [
            (96., Some(tr(cx, "Overview", "概览")), false),
            (72., Some(tr(cx, "Threads", "会话")), false),
            (110., Some(tr(cx, "Knowledge", "知识库")), true),
            (88., Some(tr(cx, "Analytics", "分析")), false),
        ];
        let tab_sensor = pro_selection_sensor();
        let tabs_row = div()
            .relative()
            .px(px(16.))
            .py(px(10.))
            .border_b_1()
            .border_color(t.line)
            .flex()
            .items_center()
            .gap(px(8.))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "preview-tabs-pill"),
                if has_domains { "know" } else { "idle" },
                tab_sensor.clone(),
                7.,
                t.side,
                Some(t.line_firm),
                false,
            ))
            .children(tab_specs.into_iter().map(|(sk_w, lbl, is_know)| {
                let on = has_domains && is_know;
                div()
                    .relative()
                    .h(px(28.))
                    .pl(px(10.))
                    .pr(px(12.))
                    .rounded(px(7.))
                    .when(!on, |d| d.border_1().border_color(t.line))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .when(on, |d| d.child(pro_mark_selected(&tab_sensor)))
                    .child(Self::render_sk_bar(t, 10., 10., 3.))
                    .child(if let Some(txt) = lbl.filter(|_| has_domains) {
                        div()
                            .text_size(px(12.))
                            .font_weight(if on {
                                FontWeight::MEDIUM
                            } else {
                                FontWeight::NORMAL
                            })
                            .text_color(if on { t.ink } else { t.muted })
                            .child(txt)
                    } else {
                        Self::render_sk_bar(t, sk_w, 8., 4.)
                    })
            }));

        // Section 1: 3 KPI cards (offscreen above glide, preserved for authentic layout height)
        let shown_sources = kpi_sources.round().max(1.) as usize;
        let kpis_row =
            div()
                .flex()
                .gap(px(12.))
                .children(
                    [64.0_f32, 52.0, 72.0]
                        .into_iter()
                        .enumerate()
                        .map(|(i, w_lbl)| {
                            div()
                                .flex_1()
                                .h(px(82.))
                                .p(px(16.))
                                .rounded(px(10.))
                                .border_1()
                                .border_color(t.line)
                                .flex()
                                .flex_col()
                                .gap(px(14.))
                                .child(if i == 0 && has_domains {
                                    div().text_size(px(12.)).text_color(t.muted).child(tr(
                                        cx,
                                        "Sources",
                                        "知识源",
                                    ))
                                } else {
                                    Self::render_sk_bar(t, w_lbl, 8., 4.)
                                })
                                .child(if i == 0 && has_domains {
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(6.))
                                        .text_size(px(18.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(t.ink)
                                        .child(
                                            div()
                                                .size(px(7.))
                                                .rounded_full()
                                                .bg(t.green.opacity(pulse_alpha)),
                                        )
                                        .child(format!("{shown_sources}"))
                                } else {
                                    Self::render_sk_bar(t, 44., 16., 5.)
                                })
                        }),
                );

        // Section 2: Agent card
        let agent_section = div()
            .flex()
            .flex_col()
            .gap(px(14.))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(Self::render_sk_bar(t, 68., 11., 4.))
                    .child(Self::render_sk_bar(t, 246., 8., 4.)),
            )
            .child(
                div()
                    .h(px(168.))
                    .p(px(16.))
                    .rounded(px(10.))
                    .border_1()
                    .border_color(t.line)
                    .flex()
                    .flex_col()
                    .gap(px(14.))
                    .child(
                        div()
                            .h(px(28.))
                            .flex()
                            .items_center()
                            .gap(px(10.))
                            .child(Self::render_agent_mark(
                                "lumen-assistant",
                                t.blue,
                                false,
                                has_domains,
                                elapsed_ms,
                                reduced,
                            ))
                            .child(Self::render_sk_bar(t, 120., 8., 4.))
                            .child(
                                div()
                                    .h(px(18.))
                                    .px(px(6.))
                                    .rounded(px(5.))
                                    .bg(if has_domains { t.green_soft } else { t.line })
                                    .text_color(if has_domains { t.green_ink } else { t.muted })
                                    .text_size(px(11.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .flex()
                                    .items_center()
                                    .child(if has_domains {
                                        tr(cx, "Indexing", "索引中")
                                    } else {
                                        tr(cx, "Draft", "草稿")
                                    }),
                            ),
                    )
                    .child(
                        div()
                            .min_h(px(52.))
                            .px(px(14.))
                            .py(px(10.))
                            .rounded_tl(px(4.))
                            .rounded_tr(px(12.))
                            .rounded_bl(px(12.))
                            .rounded_br(px(12.))
                            .bg(t.side)
                            .border_1()
                            .border_color(t.line)
                            .flex()
                            .flex_col()
                            .justify_center()
                            .gap(px(8.))
                            .child(Self::render_sk_bar(t, 280., 8., 4.))
                            .child(if has_domains {
                                div().w(px(210.)).child(pro_spring_bar(
                                    pro_eid(&self.id, "agent-idx-bar"),
                                    scan_ratio,
                                    4.,
                                    999.,
                                    t.line,
                                    t.green,
                                ))
                            } else {
                                Self::render_sk_bar(t, 190., 8., 4.)
                            }),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .child(Self::render_sk_bar(t, 88., 8., 4.))
                            .child(Self::render_sk_bar(t, 56., 16., 5.)),
                    ),
            );

        // Section 3: 28-bar chart (bottom portion peeks in at top of viewport)
        let chart_section = div()
            .flex()
            .flex_col()
            .gap(px(14.))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(Self::render_sk_bar(t, 68., 11., 4.))
                    .child(Self::render_sk_bar(t, 246., 8., 4.)),
            )
            .child(
                div()
                    .rounded(px(10.))
                    .border_1()
                    .border_color(t.line)
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .h(px(36.))
                            .px(px(14.))
                            .border_b_1()
                            .border_color(t.line)
                            .flex()
                            .items_center()
                            .gap(px(10.))
                            .child(Self::render_sk_bar(t, 10., 10., 3.))
                            .child(Self::render_sk_bar(t, 88., 8., 4.)),
                    )
                    .child(
                        div()
                            .h(px(210.))
                            .px(px(14.))
                            .pt(px(14.))
                            .pb(px(12.))
                            .flex()
                            .gap(px(14.))
                            .child(
                                div()
                                    .pb(px(4.))
                                    .flex()
                                    .flex_col()
                                    .justify_between()
                                    .children((0..5).map(|_| Self::render_sk_bar(t, 24., 8., 4.))),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .relative()
                                    .pb(px(4.))
                                    .flex()
                                    .items_end()
                                    .gap(px(6.))
                                    // Subtle horizontal reference hairlines
                                    .children([0.0_f32, 0.25, 0.50, 0.75].into_iter().map(|frac| {
                                        div()
                                            .absolute()
                                            .left_0()
                                            .right_0()
                                            .top(relative(frac))
                                            .h(px(1.))
                                            .bg(t.line)
                                    }))
                                    .children(CHART_BARS.into_iter().map(|ratio| {
                                        div()
                                            .flex_1()
                                            .min_w_0()
                                            .h(relative(ratio))
                                            .rounded_t(px(3.))
                                            .rounded_b(px(1.))
                                            .bg(t.sk)
                                    })),
                            ),
                    ),
            );

        // Section 4: Knowledge table — CENTER STAGE where added domains land live!
        let know_head = if has_domains {
            div()
                .min_h(px(28.))
                .flex()
                .flex_col()
                .gap(px(6.))
                .child(
                    div()
                        .text_size(px(14.))
                        .line_height(px(16.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(t.ink)
                        .child(tr(cx, "Knowledge", "知识库")),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .line_height(px(16.))
                        .text_color(t.muted)
                        .child(tr(
                            cx,
                            "Pages and help centers your assistant learns from",
                            "助手回答问题时实时检索的网站与文档来源",
                        )),
                )
        } else {
            div()
                .min_h(px(28.))
                .flex()
                .flex_col()
                .gap(px(8.))
                .child(Self::render_sk_bar(t, 68., 11., 4.))
                .child(Self::render_sk_bar(t, 246., 8., 4.))
        };

        let row_slots = self.domains.len().max(6);
        let mut know_table = div()
            .rounded(px(10.))
            .border_1()
            .border_color(t.line)
            .overflow_hidden()
            .flex()
            .flex_col()
            // Header row (`36px`)
            .child(
                div()
                    .h(px(36.))
                    .bg(t.side)
                    .border_b_1()
                    .border_color(t.line)
                    .flex()
                    .child(
                        div()
                            .w(px(44.))
                            .flex_none()
                            .border_r_1()
                            .border_color(t.line)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(Self::render_sk_bar(t, 11., 8., 4.)),
                    )
                    .child(
                        div()
                            .w(px(210.))
                            .flex_none()
                            .px(px(12.))
                            .border_r_1()
                            .border_color(t.line)
                            .flex()
                            .items_center()
                            .child(if has_domains {
                                div()
                                    .text_size(px(11.5))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(t.muted)
                                    .child(tr(cx, "Source", "域名来源"))
                            } else {
                                Self::render_sk_bar(t, 56., 8., 4.)
                            }),
                    )
                    .child(
                        div()
                            .w(px(120.))
                            .flex_none()
                            .px(px(12.))
                            .border_r_1()
                            .border_color(t.line)
                            .flex()
                            .items_center()
                            .child(if has_domains {
                                div()
                                    .text_size(px(11.5))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(t.muted)
                                    .child(tr(cx, "Type", "类型"))
                            } else {
                                Self::render_sk_bar(t, 40., 8., 4.)
                            }),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .px(px(12.))
                            .flex()
                            .items_center()
                            .child(if has_domains {
                                div()
                                    .text_size(px(11.5))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(t.muted)
                                    .child(tr(cx, "Status", "状态"))
                            } else {
                                Self::render_sk_bar(t, 72., 8., 4.)
                            }),
                    ),
            );

        for row_idx in 0..row_slots {
            let is_last = row_idx + 1 == row_slots;
            if let Some(dom) = self.domains.get(row_idx) {
                let kind_lbl = infer_source_kind(dom, cx);
                let pages = domain_page_estimate(dom);
                let status_txt = if tr(cx, "en", "zh") == "zh" {
                    format!("实时同步 · {pages} 页")
                } else {
                    format!("Live · {pages} pages")
                };
                let lead_icon = if row_idx == 0 {
                    Self::render_monogram_tile(t, dom, 16., 9.5).into_any_element()
                } else {
                    pro_icon(ProIcon::Globe, 13., t.muted)
                };

                know_table = know_table.child(
                    div()
                        .h(px(34.))
                        .when(!is_last, |d| d.border_b_1().border_color(t.line))
                        .flex()
                        .child(
                            div()
                                .w(px(44.))
                                .flex_none()
                                .border_r_1()
                                .border_color(t.line)
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(
                                    div()
                                        .size(px(7.))
                                        .rounded_full()
                                        .bg(t.green.opacity(pulse_alpha)),
                                ),
                        )
                        .child(
                            div()
                                .w(px(210.))
                                .flex_none()
                                .px(px(12.))
                                .border_r_1()
                                .border_color(t.line)
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .child(lead_icon)
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .text_size(px(12.5))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(t.ink)
                                        .truncate()
                                        .child(dom.clone()),
                                ),
                        )
                        .child(
                            div()
                                .w(px(120.))
                                .flex_none()
                                .px(px(12.))
                                .border_r_1()
                                .border_color(t.line)
                                .flex()
                                .items_center()
                                .child(
                                    div()
                                        .h(px(18.))
                                        .px(px(6.))
                                        .rounded(px(5.))
                                        .bg(t.line)
                                        .text_color(t.muted)
                                        .text_size(px(11.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .flex()
                                        .items_center()
                                        .child(kind_lbl),
                                ),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .px(px(12.))
                                .flex()
                                .items_center()
                                .child(
                                    div()
                                        .h(px(18.))
                                        .px(px(6.))
                                        .rounded(px(5.))
                                        .bg(t.green_soft)
                                        .text_color(t.green_ink)
                                        .text_size(px(11.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .flex()
                                        .items_center()
                                        .gap(px(5.))
                                        .child(
                                            div()
                                                .size(px(5.))
                                                .rounded_full()
                                                .bg(t.green_ink.opacity(pulse_alpha)),
                                        )
                                        .child(status_txt),
                                ),
                        ),
                );
            } else {
                know_table = know_table.child(
                    div()
                        .h(px(34.))
                        .when(!is_last, |d| d.border_b_1().border_color(t.line))
                        .when(has_domains, |d| d.opacity(0.45))
                        .flex()
                        .child(
                            div()
                                .w(px(44.))
                                .flex_none()
                                .border_r_1()
                                .border_color(t.line)
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(Self::render_sk_bar(t, 11., 8., 4.)),
                        )
                        .child(
                            div()
                                .w(px(210.))
                                .flex_none()
                                .px(px(12.))
                                .border_r_1()
                                .border_color(t.line)
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .child(Self::render_sk_bar(t, 10., 10., 3.))
                                .child(Self::render_sk_bar(t, 104., 8., 4.)),
                        )
                        .child(
                            div()
                                .w(px(120.))
                                .flex_none()
                                .px(px(12.))
                                .border_r_1()
                                .border_color(t.line)
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .child(Self::render_sk_bar(t, 10., 10., 3.))
                                .child(Self::render_sk_bar(t, 52., 8., 4.)),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .px(px(12.))
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .child(Self::render_sk_bar(t, 10., 10., 3.))
                                .child(Self::render_sk_bar(t, 104., 8., 4.)),
                        ),
                );
            }
        }

        let know_section = div()
            .flex()
            .flex_col()
            .gap(px(14.))
            .child(know_head)
            .child(know_table);

        // Section 5: Live Q&A preview card right below the Knowledge table
        let assistant_name: SharedString = if let Some(first) = self.domains.first() {
            format!("{} Assistant", derive_brand_title(first)).into()
        } else {
            tr(cx, "Assistant", "智能助手").into()
        };
        let cite_domain = self.domains.first().cloned();

        let chat_section = div()
            .flex()
            .flex_col()
            .gap(px(14.))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(Self::render_sk_bar(t, 68., 11., 4.))
                    .child(Self::render_sk_bar(t, 246., 8., 4.)),
            )
            .child(
                div()
                    .p(px(16.))
                    .rounded(px(10.))
                    .border_1()
                    .border_color(t.line)
                    .flex()
                    .flex_col()
                    .gap(px(12.))
                    // Customer question
                    .child(
                        div()
                            .flex()
                            .items_start()
                            .gap(px(10.))
                            .child(
                                div().mt(px(10.)).child(Self::render_agent_mark(
                                    "customer",
                                    t.ink,
                                    true,
                                    false,
                                    elapsed_ms,
                                    reduced,
                                )),
                            )
                            .child(
                                div()
                                    .px(px(14.))
                                    .py(px(9.))
                                    .rounded_tl(px(4.))
                                    .rounded_tr(px(12.))
                                    .rounded_bl(px(12.))
                                    .rounded_br(px(12.))
                                    .bg(t.card)
                                    .border_1()
                                    .border_color(t.line)
                                    .text_size(px(13.))
                                    .text_color(t.ink)
                                    .child("Can I change my plan mid-month?"),
                            ),
                    )
                    // Assistant answer
                    .child(
                        div()
                            .max_w(px(400.))
                            .flex()
                            .items_start()
                            .gap(px(10.))
                            .child(
                                div().mt(px(28.)).child(Self::render_agent_mark(
                                    "lumen-assistant",
                                    t.blue,
                                    false,
                                    false,
                                    elapsed_ms,
                                    reduced,
                                )),
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
                                            .text_size(px(12.))
                                            .line_height(px(16.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(t.muted)
                                            .child(assistant_name),
                                    )
                                    .child(
                                        div()
                                            .px(px(14.))
                                            .py(px(10.))
                                            .rounded_tl(px(4.))
                                            .rounded_tr(px(12.))
                                            .rounded_bl(px(12.))
                                            .rounded_br(px(12.))
                                            .bg(t.side)
                                            .border_1()
                                            .border_color(t.line)
                                            .text_size(px(13.))
                                            .line_height(px(19.))
                                            .text_color(t.ink)
                                            .child(
                                                "Yes. You can change your plan at any time; the difference is prorated.",
                                            ),
                                    ),
                            ),
                    )
                    // Handoff note + live domain citation tag
                    .child(
                        div()
                            .h(px(20.))
                            .pl(px(26.))
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .text_size(px(12.))
                            .text_color(t.faint)
                            .child(
                                div()
                                    .text_color(t.muted)
                                    .child("It answers everything itself"),
                            )
                            .when_some(cite_domain, |d, dom| {
                                d.child(
                                    div()
                                        .h(px(18.))
                                        .px(px(6.))
                                        .rounded(px(5.))
                                        .bg(t.blue.opacity(0.12))
                                        .text_color(t.blue)
                                        .text_size(px(11.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .flex()
                                        .items_center()
                                        .child(format!("{dom}/pricing")),
                                )
                            }),
                    ),
            );

        // Section 6: Bottom pair of cards
        let pair_section = div().flex().gap(px(12.)).children((0..2).map(|_| {
            div()
                .flex_1()
                .h(px(112.))
                .p(px(16.))
                .rounded(px(10.))
                .border_1()
                .border_color(t.line)
                .flex()
                .flex_col()
                .gap(px(14.))
                .child(Self::render_sk_bar(t, 88., 8., 4.))
                .child(Self::render_sk_bar(t, 156., 8., 4.))
        }));

        let body_col = div()
            .flex_1()
            .min_w_0()
            .when(!is_flat, |d| {
                d.my(px(8.))
                    .rounded_tl(px(10.))
                    .rounded_bl(px(10.))
                    .bg(t.card)
                    .border_1()
                    .border_color(t.line)
            })
            .flex()
            .flex_col()
            .child(body_bar)
            .child(tabs_row)
            .child(
                div()
                    .px(px(16.))
                    .pt(px(20.))
                    .pb(px(40.))
                    .flex()
                    .flex_col()
                    .gap(px(28.))
                    .child(kpis_row)
                    .child(pro_view_transition(
                        pro_eid(&self.id, "agent-sec-tx"),
                        if has_domains { "live" } else { "draft" },
                        agent_section,
                    ))
                    .child(chart_section)
                    .child(pro_view_transition(
                        pro_eid(&self.id, "know-sec-tx"),
                        domains_key,
                        know_section,
                    ))
                    .child(pro_view_transition(
                        pro_eid(&self.id, "qa-sec-tx"),
                        primary_key,
                        chat_section,
                    ))
                    .child(pair_section),
            );

        let win = div()
            .mt(px(glide_y))
            .w_full()
            .rounded_tl(px(12.))
            .bg(if is_flat { t.card } else { t.page })
            .border_t_1()
            .border_l_1()
            .border_color(t.line)
            .overflow_hidden()
            .flex()
            .child(side_col)
            .child(body_col);

        div()
            .absolute()
            .top(px(56.))
            .bottom_0()
            .left_1_2()
            .w(px(1088.))
            .pt(px(24.))
            .pl(px(8.))
            .overflow_hidden()
            .child(win)
            // Top & bottom vertical fade masks matching `mask-image: linear-gradient(...)`
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .right_0()
                    .h(px(36.))
                    .bg(linear_gradient(
                        180.,
                        linear_color_stop(t.bg, 0.),
                        linear_color_stop(t.bg.opacity(0.), 1.),
                    )),
            )
            .child(
                div()
                    .absolute()
                    .bottom_0()
                    .left_0()
                    .right_0()
                    .h(px(96.))
                    .bg(linear_gradient(
                        0.,
                        linear_color_stop(t.bg, 0.08),
                        linear_color_stop(t.bg.opacity(0.), 1.),
                    )),
            )
    }

    fn render_loading_skeleton(&self, narrow: bool, w: &mut Window, cx: &App) -> Div {
        let phase = if Theme::global(cx).reduced_motion {
            0.25
        } else {
            w.request_animation_frame();
            (self.started_at.elapsed().as_secs_f32() / 1.4).fract()
        };

        div()
            .flex_1()
            .min_h_0()
            .when(narrow, |d| d.w_full().px(px(20.)))
            .when(!narrow, |d| d.w_1_2().px(px(40.)))
            .py(px(24.))
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .w_full()
                    .max_w(px(440.))
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(12.))
                    .child(pro_skeleton_bar(76., 4., 2., phase, cx))
                    .child(div().mt(px(6.)).child(pro_skeleton_bar(
                        296.,
                        26.,
                        6.,
                        phase + 0.06,
                        cx,
                    )))
                    .child(pro_skeleton_bar(336., 14., 4., phase + 0.12, cx))
                    .child(div().w_full().mt(px(16.)).child(pro_skeleton_bar(
                        420.,
                        44.,
                        12.,
                        phase + 0.18,
                        cx,
                    ))),
            )
    }
}

impl Render for OnboardingWebsiteScreen {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = ObwTokens::resolve(cx);
        let reduced = Theme::global(cx).reduced_motion;

        // Sync invalid-domain ink color for `Editor` run styler before painting.
        self.invalid_ink
            .set(self.invalid_error.as_ref().map(|_| t.red));
        self.ensure_editor(w, cx);

        let measure_cell = self.measured_width.clone();
        let container_w = measure_cell.get();
        let narrow = container_w <= 900.;

        // Top 56px bar: clean geometric product mark + product name, optional Back, account + Log out
        let top_bar = div()
            .h(px(56.))
            .flex_none()
            .px(px(if narrow { 16. } else { 24. }))
            .flex()
            .items_center()
            .gap(px(12.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(pro_svg("0 0 24 24", LUMEN_MARK_SVG, 20., t.dark_fill))
                    .child(
                        div()
                            .text_size(px(14.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(t.ink)
                            .child(self.product.clone()),
                    ),
            )
            .when(self.show_back, |d| {
                d.child(activate(
                    div()
                        .id(pro_eid(&self.id, "top-back"))
                        .role(Role::Button)
                        .tab_index(0)
                        .h(px(28.))
                        .pl(px(8.))
                        .pr(px(10.))
                        .rounded(px(6.))
                        .bg(t.card)
                        .border_1()
                        .border_color(t.line)
                        .hover(move |s| s.border_color(t.line_firm))
                        .focus_visible(move |s| s.border_1().border_color(t.focus))
                        .cursor_pointer()
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .text_size(px(13.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(t.ink)
                        .child(pro_icon(ProIcon::ChevronLeft, 14., t.faint))
                        .child(tr(cx, "Back", "返回")),
                    cx,
                    |_, _, cx| {
                        cx.emit(OnboardingWebsiteEvent::Back);
                    },
                ))
            })
            .child(
                div()
                    .ml_auto()
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .when(!narrow, |d| {
                        d.child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(4.))
                                .text_size(px(12.))
                                .line_height(px(16.))
                                .text_color(t.faint)
                                .child(tr(cx, "Logged in as", "当前登录"))
                                .child(div().text_color(t.muted).child(self.email.clone())),
                        )
                    })
                    .child(activate(
                        div()
                            .id(pro_eid(&self.id, "logout-btn"))
                            .role(Role::Button)
                            .tab_index(0)
                            .h(px(28.))
                            .pl(px(8.))
                            .pr(px(10.))
                            .rounded(px(6.))
                            .bg(t.card)
                            .border_1()
                            .border_color(t.line)
                            .shadow(vec![BoxShadow {
                                color: hsla(0., 0., 0., if t.dark { 0.30 } else { 0.05 }),
                                offset: point(px(0.), px(1.)),
                                blur_radius: px(2.),
                                spread_radius: px(0.),
                                inset: false,
                            }])
                            .hover(move |s| s.border_color(t.line_firm))
                            .focus_visible(move |s| s.border_1().border_color(t.focus))
                            .cursor_pointer()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(t.ink)
                            .child(pro_svg("0 0 24 24", LOGOUT_SVG, 14., t.faint))
                            .child(tr(cx, "Log out", "退出登录")),
                        cx,
                        |_, _, cx| {
                            cx.emit(OnboardingWebsiteEvent::Logout);
                        },
                    )),
            );

        // Bottom 48px footer (`© 2026 Lumen · Terms · Privacy`)
        let footer = div()
            .h(px(48.))
            .flex_none()
            .px(px(if narrow { 20. } else { 24. }))
            .flex()
            .items_center()
            .gap(px(16.))
            .text_size(px(12.))
            .line_height(px(16.))
            .text_color(t.faint)
            .child(format!("© 2026 {}", self.product))
            .child(activate(
                div()
                    .id(pro_eid(&self.id, "foot-terms"))
                    .role(Role::Button)
                    .tab_index(0)
                    .rounded(px(4.))
                    .hover(move |s| s.text_color(t.muted))
                    .cursor_pointer()
                    .child(tr(cx, "Terms", "服务条款")),
                cx,
                |_, _, cx| {
                    cx.emit(OnboardingWebsiteEvent::OpenLegal("Terms".into()));
                },
            ))
            .child(activate(
                div()
                    .id(pro_eid(&self.id, "foot-privacy"))
                    .role(Role::Button)
                    .tab_index(0)
                    .rounded(px(4.))
                    .hover(move |s| s.text_color(t.muted))
                    .cursor_pointer()
                    .child(tr(cx, "Privacy", "隐私政策")),
                cx,
                |_, _, cx| {
                    cx.emit(OnboardingWebsiteEvent::OpenLegal("Privacy".into()));
                },
            ));

        // Left-column step body
        let left_main = if self.loading {
            self.render_loading_skeleton(narrow, w, cx)
        } else if self.empty {
            div()
                .flex_1()
                .min_h_0()
                .when(narrow, |d| d.w_full().px(px(20.)))
                .when(!narrow, |d| d.w_1_2().px(px(40.)))
                .py(px(24.))
                .flex()
                .items_center()
                .justify_center()
                .child(pro_empty_state(
                    ProIcon::Globe,
                    tr(cx, "No knowledge domains yet", "尚未添加网站域名"),
                    tr(
                        cx,
                        "Add your company website, docs, or help center so your assistant can ground every answer.",
                        "添加公司官网、文档站或帮助中心域名，以便智能助手据此回答问题。",
                    ),
                    Some((
                        pro_eid(&self.id, "empty-start"),
                        tr(cx, "Add website", "添加网站").into(),
                        Box::new(|this: &mut Self, w, cx| {
                            this.empty = false;
                            if let Some(ed) = &this.domain_editor {
                                { let fh = ed.read(cx).focus.clone(); fh.focus(w, cx); }
                            }
                            cx.notify();
                        }),
                    )),
                    cx,
                ))
        } else {
            let draft_text = self
                .domain_editor
                .as_ref()
                .map(|e| e.read(cx).value().trim().to_string())
                .unwrap_or_default();
            let draft_norm = normalize_domain(&draft_text);
            let can_add = !draft_text.is_empty();
            let can_go = !self.domains.is_empty() || is_valid_domain(&draft_norm);

            let focus_t = pro_spring_f32(
                pro_eid(&self.id, "url-focus-sp"),
                f32::from(self.editor_focused),
                0.18,
                w,
                cx,
            )
            .clamp(0., 1.);
            let add_alpha = pro_spring_f32(
                pro_eid(&self.id, "add-btn-sp"),
                if can_add { 1. } else { 0.45 },
                0.18,
                w,
                cx,
            )
            .clamp(0.45, 1.);
            let go_alpha = pro_spring_f32(
                pro_eid(&self.id, "go-btn-sp"),
                if can_go { 1. } else { 0.45 },
                0.18,
                w,
                cx,
            )
            .clamp(0.45, 1.);

            let total_pages: u32 = self.domains.iter().map(|d| domain_page_estimate(d)).sum();
            let anim_pages = pro_spring_f32(
                pro_eid(&self.id, "left-pages-sp"),
                total_pages as f32,
                0.28,
                w,
                cx,
            )
            .max(0.);

            let left_scan_ratio = if self.domains.is_empty() {
                0.0
            } else if let Some(anim_at) = self.last_domain_anim {
                let dt = (anim_at.elapsed().as_secs_f32() / 0.70).clamp(0., 1.);
                if !reduced && dt < 1.0 {
                    w.request_animation_frame();
                    0.42 + 0.58 * pro_ease(dt)
                } else {
                    1.0
                }
            } else {
                1.0
            };

            // Drift phase for the ambient 3-lobe radial aura
            let drift_dx = if reduced {
                0.0
            } else {
                w.request_animation_frame();
                let phase = (self.started_at.elapsed().as_secs_f32() / 9.0) * std::f32::consts::PI;
                phase.sin() * 8.0
            };
            let glow_alpha = (if t.dark { 0.22 } else { 0.28 }) * (1.0 + 0.30 * focus_t);

            let step_dashes = div()
                .mb(px(20.))
                .flex()
                .items_center()
                .gap(px(4.))
                .children((1..=self.steps).map(|idx| {
                    let active = idx <= self.step;
                    div().w(px(16.)).h(px(3.)).rounded(px(2.)).bg(if active {
                        t.dark_fill
                    } else {
                        t.line
                    })
                }));

            let editor_el = self
                .domain_editor
                .clone()
                .map(|e| e.into_any_element())
                .unwrap_or_else(|| div().into_any_element());

            let add_btn = {
                let btn = div()
                    .id(pro_eid(&self.id, "add-domain"))
                    .role(Role::Button)
                    .aria_label(tr(cx, "Add domain", "添加域名"))
                    .tab_index(0)
                    .size(px(28.))
                    .flex_none()
                    .rounded(px(8.))
                    .bg(t.card)
                    .border_1()
                    .border_color(t.line)
                    .shadow(vec![BoxShadow {
                        color: hsla(0., 0., 0., if t.dark { 0.30 } else { 0.05 }),
                        offset: point(px(0.), px(1.)),
                        blur_radius: px(2.),
                        spread_radius: px(0.),
                        inset: false,
                    }])
                    .opacity(add_alpha)
                    .when(can_add, |d| {
                        d.cursor_pointer()
                            .hover(move |s| s.border_color(t.line_firm))
                    })
                    .focus_visible(move |s| s.border_1().border_color(t.focus))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(pro_icon(ProIcon::Plus, 14., t.ink));
                activate(btn, cx, |this, w, cx| {
                    let val = this
                        .domain_editor
                        .as_ref()
                        .map(|e| e.read(cx).value().trim().to_string())
                        .unwrap_or_default();
                    if !val.is_empty()
                        && this.try_add_domain(&val, cx).is_ok()
                        && let Some(ed) = &this.domain_editor
                    {
                        ed.update(cx, |editor, cx| editor.set_value("", w, cx));
                    }
                })
            };

            let go_btn = {
                let btn = div()
                    .id(pro_eid(&self.id, "continue-btn"))
                    .role(Role::Button)
                    .aria_label(tr(cx, "Continue", "继续"))
                    .tab_index(0)
                    .size(px(28.))
                    .flex_none()
                    .rounded(px(8.))
                    .bg(t.dark_fill)
                    .opacity(go_alpha)
                    .when(can_go, |d| d.cursor_pointer().hover(|s| s.opacity(0.88)))
                    .focus_visible(move |s| s.border_1().border_color(t.focus))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(pro_icon(ProIcon::ArrowRight, 14., t.on_dark));
                activate(btn, cx, |this, w, cx| {
                    this.submit_continue(Some(w), cx);
                })
            };

            let mut chips_row = div()
                .mt(px(10.))
                .min_h(px(26.))
                .flex()
                .flex_wrap()
                .justify_center()
                .gap(px(6.));

            for (idx, dom) in self.domains.iter().enumerate() {
                let dom_label = dom.clone();
                let is_primary = idx == 0;
                chips_row = chips_row.child(
                    div()
                        .h(px(26.))
                        .pl(px(if is_primary { 6. } else { 9. }))
                        .pr(px(4.))
                        .rounded(px(7.))
                        .bg(t.card)
                        .border_1()
                        .border_color(t.line)
                        .shadow(vec![BoxShadow {
                            color: hsla(0., 0., 0., if t.dark { 0.30 } else { 0.05 }),
                            offset: point(px(0.), px(1.)),
                            blur_radius: px(2.),
                            spread_radius: px(0.),
                            inset: false,
                        }])
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .when(is_primary, |d| {
                            d.child(Self::render_monogram_tile(t, &dom_label, 16., 9.5))
                        })
                        .child(
                            div()
                                .text_size(px(13.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(t.ink)
                                .child(dom_label.clone()),
                        )
                        .child(activate(
                            div()
                                .id(pro_eid(&self.id, format!("rm-dom-{idx}")))
                                .role(Role::Button)
                                .aria_label(format!("{} {dom_label}", tr(cx, "Remove", "移除")))
                                .tab_index(0)
                                .size(px(18.))
                                .ml(px(-2.))
                                .rounded(px(4.))
                                .hover(move |s| s.bg(t.line).text_color(t.ink))
                                .focus_visible(move |s| s.border_1().border_color(t.focus))
                                .cursor_pointer()
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(pro_icon(ProIcon::Close, 11., t.faint)),
                            cx,
                            move |this, _, cx| {
                                this.remove_domain(idx, cx);
                            },
                        )),
                );
            }

            let err_txt = self
                .invalid_error
                .clone()
                .unwrap_or_else(|| self.last_invalid_error.clone());
            let err_red = t.red;
            let err_fold = pro_fold(
                pro_eid(&self.id, "url-err-fold"),
                self.invalid_error.is_some(),
                18.,
                0.,
                move || {
                    div()
                        .mt(px(8.))
                        .text_size(px(12.))
                        .line_height(px(16.))
                        .text_color(err_red)
                        .text_center()
                        .child(err_txt)
                        .into_any_element()
                },
            );

            let has_doms = !self.domains.is_empty();
            let dom_count = self.domains.len();
            let pages_rounded = anim_pages.round() as u32;
            let crawl_summary_lbl: SharedString = if tr(cx, "en", "zh") == "zh" {
                format!("已扫描 {dom_count} 个来源 · 发现约 {pages_rounded} 个页面").into()
            } else {
                format!(
                    "Scanned {dom_count} {} · ~{pages_rounded} pages discovered",
                    if dom_count == 1 { "source" } else { "sources" }
                )
                .into()
            };
            let facts_key: SharedString = self
                .domains
                .iter()
                .map(SharedString::as_ref)
                .collect::<Vec<_>>()
                .join(",")
                .into();

            let mut facts_rows = div()
                .pt(px(8.))
                .mt(px(8.))
                .border_t_1()
                .border_color(t.line)
                .flex()
                .flex_col()
                .gap(px(5.));
            for dom in self.domains.iter().take(3) {
                let kind = infer_source_kind(dom, cx);
                let p_est = domain_page_estimate(dom);
                let p_lbl: SharedString = if tr(cx, "en", "zh") == "zh" {
                    format!("{p_est} 页")
                } else {
                    format!("{p_est} pages")
                }
                .into();
                facts_rows = facts_rows.child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(8.))
                        .text_size(px(12.))
                        .child(
                            div()
                                .min_w_0()
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .child(pro_icon(ProIcon::Globe, 12., t.muted))
                                .child(
                                    div()
                                        .min_w_0()
                                        .truncate()
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(t.ink)
                                        .child(dom.clone()),
                                ),
                        )
                        .child(
                            div()
                                .flex_none()
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .child(
                                    div()
                                        .px(px(5.))
                                        .py(px(1.))
                                        .rounded(px(4.))
                                        .bg(t.line)
                                        .text_size(px(11.))
                                        .text_color(t.muted)
                                        .child(kind),
                                )
                                .child(div().text_color(t.faint).child(p_lbl)),
                        ),
                );
            }
            let facts_body_el =
                pro_view_transition(pro_eid(&self.id, "facts-rows-tx"), facts_key, facts_rows);

            let crawl_card_div = div()
                .mt(px(12.))
                .w_full()
                .p(px(10.))
                .rounded(px(10.))
                .bg(t.card)
                .border_1()
                .border_color(t.line)
                .flex()
                .flex_col()
                .child(activate(
                    div()
                        .id(pro_eid(&self.id, "crawl-fold-btn"))
                        .role(Role::Button)
                        .tab_index(0)
                        .cursor_pointer()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(8.))
                        .child(
                            div()
                                .min_w_0()
                                .flex()
                                .items_center()
                                .gap(px(7.))
                                .child(div().size(px(6.)).rounded_full().bg(t.green))
                                .child(
                                    div()
                                        .min_w_0()
                                        .truncate()
                                        .text_size(px(12.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(t.ink)
                                        .child(crawl_summary_lbl),
                                ),
                        )
                        .child(pro_icon(
                            if self.crawl_details_open {
                                ProIcon::ChevronUp
                            } else {
                                ProIcon::ChevronDown
                            },
                            12.,
                            t.faint,
                        )),
                    cx,
                    |this, _, cx| {
                        this.crawl_details_open = !this.crawl_details_open;
                        cx.notify();
                    },
                ))
                .child(div().mt(px(7.)).w_full().child(pro_spring_bar(
                    pro_eid(&self.id, "left-scan-bar"),
                    left_scan_ratio,
                    3.,
                    999.,
                    t.line,
                    t.blue,
                )))
                .child(pro_fold(
                    pro_eid(&self.id, "extracted-facts-fold"),
                    has_doms && self.crawl_details_open,
                    68.,
                    0.,
                    move || facts_body_el,
                ));

            let crawl_presence = pro_presence(
                pro_eid(&self.id, "crawl-card-pres"),
                has_doms,
                0.22,
                0.16,
                move |amt| {
                    crawl_card_div
                        .relative()
                        .top(px(4. * (1. - amt)))
                        .opacity(amt)
                        .into_any_element()
                },
            );

            let hint_el = if self.invalid_error.is_none() && has_doms {
                div()
                    .mt(px(8.))
                    .min_h(px(16.))
                    .text_size(px(12.))
                    .line_height(px(16.))
                    .text_color(t.faint)
                    .text_center()
                    .child(tr(
                        cx,
                        "Add another domain or press Enter to continue",
                        "可继续添加更多域名，或直接按回车进入下一步",
                    ))
            } else {
                div().mt(px(8.)).min_h(px(16.))
            };

            let box_border_c = if self.invalid_error.is_some() {
                t.red.opacity(0.65)
            } else {
                pro_mix_color(t.line, t.line_firm, focus_t)
            };

            div()
                .flex_1()
                .min_h_0()
                .when(narrow, |d| d.w_full().px(px(20.)))
                .when(!narrow, |d| d.w_1_2().px(px(40.)))
                .py(px(24.))
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .w_full()
                        .max_w(px(440.))
                        .flex()
                        .flex_col()
                        .items_center()
                        .child(step_dashes)
                        .child(
                            div()
                                .text_size(px(24.))
                                .line_height(px(32.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(t.ink)
                                .text_center()
                                .child(tr(
                                    cx,
                                    "What's your company's website?",
                                    "您的公司网站是什么？",
                                )),
                        )
                        .child(
                            div()
                                .mt(px(6.))
                                .text_size(px(13.))
                                .line_height(px(20.))
                                .text_color(t.muted)
                                .text_center()
                                .child(tr(
                                    cx,
                                    "Your assistant learns from it. Add your docs or help center, too.",
                                    "助手将从中学习知识，也可同时添加文档站或帮助中心。",
                                )),
                        )
                        .child(
                            div()
                                .w_full()
                                .mt(px(28.))
                                .flex()
                                .flex_col()
                                .child(
                                    div()
                                        .relative()
                                        .w_full()
                                        // 3-lobe blurred ambient aura (blue -> purple -> pink)
                                        .child(
                                            div()
                                                .absolute()
                                                .left(px(8. + drift_dx))
                                                .top(px(6.))
                                                .w(relative(0.36))
                                                .h(px(32.))
                                                .rounded_full()
                                                .shadow(vec![BoxShadow {
                                                    color: t.blue.opacity(glow_alpha),
                                                    offset: point(px(0.), px(4.)),
                                                    blur_radius: px(28.),
                                                    spread_radius: px(6.),
                                                    inset: false,
                                                }]),
                                        )
                                        .child(
                                            div()
                                                .absolute()
                                                .left(relative(0.32))
                                                .top(px(4.))
                                                .w(relative(0.36))
                                                .h(px(36.))
                                                .rounded_full()
                                                .shadow(vec![BoxShadow {
                                                    color: t.purple.opacity(glow_alpha),
                                                    offset: point(px(drift_dx * 0.5), px(2.)),
                                                    blur_radius: px(28.),
                                                    spread_radius: px(6.),
                                                    inset: false,
                                                }]),
                                        )
                                        .child(
                                            div()
                                                .absolute()
                                                .right(px(8. - drift_dx))
                                                .top(px(6.))
                                                .w(relative(0.34))
                                                .h(px(32.))
                                                .rounded_full()
                                                .shadow(vec![BoxShadow {
                                                    color: t.pink.opacity(glow_alpha),
                                                    offset: point(px(0.), px(2.)),
                                                    blur_radius: px(28.),
                                                    spread_radius: px(6.),
                                                    inset: false,
                                                }]),
                                        )
                                        // Foreground 44px rounded-12 domain box
                                        .child(
                                            div()
                                                .relative()
                                                .h(px(44.))
                                                .pl(px(14.))
                                                .pr(px(8.))
                                                .rounded(px(12.))
                                                .bg(t.card)
                                                .border_1()
                                                .border_color(box_border_c)
                                                .shadow(vec![BoxShadow {
                                                    color: hsla(
                                                        0.,
                                                        0.,
                                                        0.,
                                                        if t.dark {
                                                            0.30 + 0.12 * focus_t
                                                        } else {
                                                            0.05 + 0.03 * focus_t
                                                        },
                                                    ),
                                                    offset: point(
                                                        px(0.),
                                                        px(1. + focus_t),
                                                    ),
                                                    blur_radius: px(2. + 4. * focus_t),
                                                    spread_radius: px(0.),
                                                    inset: false,
                                                }])
                                                .flex()
                                                .items_center()
                                                .gap(px(6.))
                                                .child(
                                                    div()
                                                        .flex_1()
                                                        .min_w_0()
                                                        .h_full()
                                                        .child(editor_el),
                                                )
                                                .child(add_btn)
                                                .child(go_btn),
                                        ),
                                )
                                .child(chips_row)
                                .child(err_fold)
                                .child(crawl_presence)
                                .child(hint_el),
                        ),
                )
        };

        let preview_layer = (!narrow).then(|| self.render_dashboard_preview(t, container_w, w, cx));

        div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .relative()
            .size_full()
            .min_h(px(540.))
            .bg(t.bg)
            .text_color(t.ink)
            .text_size(px(13.))
            .overflow_hidden()
            .flex()
            .flex_col()
            .capture_key_down(cx.listener(|this, ev: &KeyDownEvent, w, cx| {
                if ev.keystroke.key == "backspace"
                    && let Some(ed) = &this.domain_editor
                    && ed.read(cx).focus.is_focused(w)
                    && !ed.read(cx).is_composing()
                    && ed.read(cx).value().is_empty()
                    && !this.domains.is_empty()
                {
                    this.pop_last_domain(cx);
                    w.prevent_default();
                    cx.stop_propagation();
                }
            }))
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, w, cx| {
                if ev.keystroke.key == "/"
                    && let Some(ed) = &this.domain_editor
                    && !ed.read(cx).focus.is_focused(w)
                {
                    {
                        let fh = ed.read(cx).focus.clone();
                        fh.focus(w, cx);
                    }
                    w.prevent_default();
                } else if ev.keystroke.key == "escape" && this.invalid_error.is_some() {
                    this.invalid_error = None;
                    cx.notify();
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
                .left_0()
                .right_0()
                .top_0()
                .h(px(0.)),
            )
            .when_some(preview_layer, |d, p| d.child(p))
            .child(top_bar)
            .child(left_main)
            .child(footer)
    }
}

#[cfg(test)]
mod tests {
    use super::{OnboardingWebsiteScreen, is_valid_domain, normalize_domain};
    use gpui::TestAppContext;

    #[test]
    fn domain_normalization_strips_scheme_www_port_and_paths() {
        assert_eq!(
            normalize_domain("  HTTPS://www.Northwind.com:8443/docs/getting-started?tab=1#top "),
            "northwind.com"
        );
        assert_eq!(
            normalize_domain("http://docs.northwind.com/api/v2,"),
            "docs.northwind.com"
        );
        assert_eq!(normalize_domain("www.help.acme.co.uk."), "help.acme.co.uk");
    }

    #[test]
    fn domain_validation_rejects_partial_or_malformed_hosts() {
        assert!(is_valid_domain("northwind.com"));
        assert!(is_valid_domain("docs.northwind.co.uk"));
        assert!(!is_valid_domain("northwin"));
        assert!(!is_valid_domain("northwind.c"));
        assert!(!is_valid_domain("-bad.com"));
        assert!(!is_valid_domain("alex@northwind.com"));
        assert!(!is_valid_domain("not a domain"));
    }

    #[test]
    fn chip_addition_deduplication_validation_and_removal_work() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (screen, w) =
            app.add_window_view(|_w, cx| OnboardingWebsiteScreen::new("test-obw", cx));

        w.update(|win, cx| {
            screen.update(cx, |s, cx| {
                // Invalid domain sets error message and leaves chips empty.
                assert!(s.try_add_domain("northwind.c", cx).is_err());
                assert!(s.invalid_message().is_some());
                assert!(s.domains().is_empty());

                // Valid URL normalizes, clears error, and adds chip.
                let first = s
                    .try_add_domain("https://www.Northwind.com/pricing", cx)
                    .unwrap_or_default();
                assert_eq!(first.as_ref(), "northwind.com");
                assert!(s.invalid_message().is_none());

                // Duplicate domain is idempotent; subdomain adds a second chip.
                let _ = s.try_add_domain("NORTHWIND.COM", cx);
                let _ = s.try_add_domain("https://docs.northwind.com/reference", cx);
                assert_eq!(s.domains(), &["northwind.com", "docs.northwind.com"]);

                // Backspace-equivalent pop removes the last chip first.
                let popped = s.pop_last_domain(cx);
                assert_eq!(popped.as_deref(), Some("docs.northwind.com"));
                assert_eq!(s.domains(), &["northwind.com"]);

                // Submitting with a valid chip succeeds.
                assert!(s.submit_continue(Some(win), cx));

                // Removing index 0 empties the list.
                assert_eq!(s.remove_domain(0, cx).as_deref(), Some("northwind.com"));
                assert!(s.domains().is_empty());
            });
        });
    }
}
