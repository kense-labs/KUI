//! Onboarding wizard step two for naming a company and its customer-facing short forms.
//!
//! Pairs a centered chip-entry field on the left with a live decorative preview of
//! the assistant's dashboard on the right, glided to the assistant summary card so
//! workspace monograms, greeting copy, and alias tags update as names are entered.

use crate::{
    components::{
        app_shell::AppShellLayout,
        pro_chrome::{
            ProEmptyCta, ProPalette, pro_ease, pro_eid, pro_empty_state, pro_fold,
            pro_mark_selected, pro_mix_color, pro_pill_indicator, pro_presence,
            pro_selection_sensor, pro_skeleton_bar, pro_spring_f32, pro_view_transition, tr,
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

const BRAND_SWATCH_HEXES: [(u32, &str); 6] = [
    (0x0190fe, "Sky"),
    (0x635bff, "Indigo"),
    (0xa651f0, "Violet"),
    (0x1e8a4f, "Emerald"),
    (0xe88b00, "Amber"),
    (0xee3e9e, "Rose"),
];

/// Derive a clean Title-Case company name from a domain or URL string.
///
/// # Examples
///
/// ```
/// use kui::components::onboarding_brand::extract_brand_name;
///
/// assert_eq!(extract_brand_name("northwind.com"), "Northwind");
/// assert_eq!(extract_brand_name("https://docs.fieldwork.io/api"), "Fieldwork");
/// ```
pub fn extract_brand_name(domain: &str) -> String {
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
        "Workspace".to_string()
    } else {
        words.join(" ")
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
    purple: Hsla,
    pink: Hsla,
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
                purple: pro.purple,
                pink: pro.pink,
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
                purple: pro.purple,
                pink: pro.pink,
                red: rgb(0xd93a2b).into(),
                mark: rgba(0x0060e2c7).into(),
                focus: pro.focus,
            }
        }
    }
}

/// Events emitted by [`OnboardingBrandScreen`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OnboardingBrandEvent {
    /// User pressed Continue with the ordered company names (primary name first).
    Continue(Vec<SharedString>),
    /// Company names list changed when a chip was added or removed.
    Change(Vec<SharedString>),
    /// Brand accent swatch changed (`None` when reset to monochrome default).
    BrandColorChanged(Option<u32>),
    /// User pressed Back in the top header.
    Back,
    /// User pressed Log out in the top header.
    Logout,
    /// User activated a footer policy link (`"terms"` or `"privacy"`).
    OpenLegal(SharedString),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum BrandSpecimenMode {
    #[default]
    Card,
    Chat,
    Email,
}

impl BrandSpecimenMode {
    const ALL: [Self; 3] = [Self::Card, Self::Chat, Self::Email];

    fn as_str(self) -> &'static str {
        match self {
            Self::Card => "card",
            Self::Chat => "chat",
            Self::Email => "email",
        }
    }

    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::Card => tr(cx, "Card", "名片"),
            Self::Chat => tr(cx, "Chat", "对话"),
            Self::Email => tr(cx, "Email", "邮件"),
        }
    }
}

/// Step 2 of 4 in the AI assistant onboarding wizard: company name and short forms.
pub struct OnboardingBrandScreen {
    id: ElementId,
    domain: SharedString,
    names: Vec<SharedString>,
    explicit_names: bool,
    email: SharedString,
    product: SharedString,
    layout: AppShellLayout,
    step: usize,
    steps: usize,
    show_back: bool,
    auto_focus: bool,
    did_auto_focus: bool,
    auto_type: Option<SharedString>,
    auto_type_halted: bool,
    auto_type_index: usize,
    brand_color: Option<Hsla>,
    brand_swatch_hex: Option<u32>,
    show_color_swatches: bool,
    palette_open: bool,
    custom_hex_open: bool,
    specimen_mode: BrandSpecimenMode,
    error_hint: Option<SharedString>,
    last_error_hint: SharedString,
    loading: bool,
    empty: bool,
    name_editor: Option<Entity<Editor>>,
    focus: FocusHandle,
    measured_width: Rc<Cell<f32>>,
    opened_at: Option<Instant>,
}

impl EventEmitter<OnboardingBrandEvent> for OnboardingBrandScreen {}

impl OnboardingBrandScreen {
    /// Construct the Brand onboarding step with `northwind.com` defaults.
    pub fn new(id: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        let domain: SharedString = "northwind.com".into();
        let initial = SharedString::from(extract_brand_name(&domain));
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        Self {
            id: ElementId::Name(id.into()),
            domain,
            names: vec![initial],
            explicit_names: false,
            email: "alex@northwind.com".into(),
            product: "Lumen".into(),
            layout: AppShellLayout::Inset,
            step: 2,
            steps: 4,
            show_back: true,
            auto_focus: false,
            did_auto_focus: false,
            auto_type: None,
            auto_type_halted: false,
            auto_type_index: 0,
            brand_color: None,
            brand_swatch_hex: None,
            show_color_swatches: false,
            palette_open: false,
            custom_hex_open: false,
            specimen_mode: BrandSpecimenMode::Card,
            error_hint: None,
            last_error_hint: "".into(),
            loading: false,
            empty: false,
            name_editor: None,
            focus: cx.focus_handle(),
            measured_width: Rc::new(Cell::new(1280.)),
            opened_at: None,
        }
    }

    /// Set the website domain from Step 1; updates the suggested primary name unless
    /// [`Self::with_default_names`] was already supplied.
    pub fn with_domain(mut self, domain: impl Into<SharedString>) -> Self {
        self.domain = domain.into();
        if !self.explicit_names {
            self.names = vec![SharedString::from(extract_brand_name(&self.domain))];
        }
        self
    }

    /// Seed known company names, replacing the automatic domain suggestion.
    /// The first entry becomes the primary name the assistant introduces itself with.
    pub fn with_default_names<I, S>(mut self, names: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        self.explicit_names = true;
        self.names = names
            .into_iter()
            .map(Into::into)
            .filter(|s| !s.trim().is_empty())
            .collect();
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

    /// Show or hide the `Back` button in the top bar.
    pub fn with_show_back(mut self, show: bool) -> Self {
        self.show_back = show;
        self
    }

    /// Focus the name input automatically when the screen first opens.
    pub fn with_auto_focus(mut self, auto_focus: bool) -> Self {
        self.auto_focus = auto_focus;
        self
    }

    /// Type a demonstration string into the input once on load until the user edits.
    pub fn with_auto_type(mut self, text: impl Into<SharedString>) -> Self {
        let s = text.into();
        self.auto_type = (!s.is_empty()).then_some(s);
        self
    }

    /// Override the brand accent color used on the preview monogram, agent mark, and chart bars.
    pub fn with_brand_color(mut self, color: Hsla) -> Self {
        self.brand_color = Some(color);
        self
    }

    /// Enable an interactive brand accent swatch picker below the company name chips.
    pub fn with_color_swatches(mut self, enabled: bool) -> Self {
        self.show_color_swatches = enabled;
        self.palette_open = enabled;
        self
    }

    /// Render skeleton placeholders in place of the step form.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Render an empty state when no domain context is available.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Current ordered list of company names (primary first, short forms after).
    pub fn names(&self) -> &[SharedString] {
        &self.names
    }

    /// Primary company name used by the assistant, if at least one name is present.
    pub fn primary_name(&self) -> Option<&SharedString> {
        self.names.first()
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

    /// Add one or more comma-separated company names or short forms.
    /// Returns `true` if at least one new chip was appended.
    pub fn add_name(&mut self, raw: &str, cx: &mut Context<Self>) -> bool {
        let mut added = false;
        self.error_hint = None;
        for part in raw
            .split([',', '，'])
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            if self
                .names
                .iter()
                .any(|existing| existing.eq_ignore_ascii_case(part))
            {
                let msg: SharedString = tr(cx, "Already added", "已添加该名称").into();
                self.last_error_hint = msg.clone();
                self.error_hint = Some(msg);
            } else {
                self.names.push(SharedString::from(part.to_owned()));
                added = true;
            }
        }
        if added {
            self.error_hint = None;
            cx.emit(OnboardingBrandEvent::Change(self.names.clone()));
            cx.notify();
        } else if self.error_hint.is_some() {
            cx.notify();
        }
        added
    }

    /// Remove the company name chip at `index`.
    pub fn remove_name(&mut self, index: usize, cx: &mut Context<Self>) -> bool {
        if index < self.names.len() {
            self.names.remove(index);
            self.error_hint = None;
            cx.emit(OnboardingBrandEvent::Change(self.names.clone()));
            cx.notify();
            true
        } else {
            false
        }
    }

    /// Validate and emit [`OnboardingBrandEvent::Continue`] when at least one name is present.
    pub fn submit(&mut self, cx: &mut Context<Self>) -> bool {
        if self.names.is_empty() {
            let msg: SharedString = tr(
                cx,
                "Enter your company name to continue.",
                "请输入公司名称以继续。",
            )
            .into();
            self.last_error_hint = msg.clone();
            self.error_hint = Some(msg);
            cx.notify();
            return false;
        }
        self.error_hint = None;
        cx.emit(OnboardingBrandEvent::Continue(self.names.clone()));
        cx.notify();
        true
    }

    fn commit_editor(&mut self, w: &Window, cx: &mut Context<Self>) -> bool {
        let Some(ed) = self.name_editor.clone() else {
            return false;
        };
        let raw = ed.read(cx).value().to_owned();
        if raw.trim().is_empty() {
            return false;
        }
        let added = self.add_name(&raw, cx);
        if added {
            let ph = self.placeholder_text(cx);
            ed.update(cx, |e, cx| {
                e.set_value("", w, cx);
                e.set_placeholder(ph);
            });
        }
        added
    }

    fn continue_from_ui(&mut self, w: &Window, cx: &mut Context<Self>) {
        if let Some(ed) = self.name_editor.clone() {
            let pending = ed.read(cx).value().trim().to_owned();
            if !pending.is_empty() {
                let _ = self.add_name(&pending, cx);
                let ph = self.placeholder_text(cx);
                ed.update(cx, |e, cx| {
                    e.set_value("", w, cx);
                    e.set_placeholder(ph);
                });
            }
        }
        let _ = self.submit(cx);
    }

    fn placeholder_text(&self, cx: &App) -> SharedString {
        if self.names.is_empty() {
            extract_brand_name(&self.domain).into()
        } else {
            tr(cx, "Add a short form", "添加简称").into()
        }
    }

    fn ensure_editor(&mut self, w: &mut Window, cx: &mut Context<Self>) -> Entity<Editor> {
        if let Some(ed) = &self.name_editor {
            let ph = self.placeholder_text(cx);
            ed.update(cx, |e, _| e.set_placeholder(ph));
            return ed.clone();
        }
        let ph = self.placeholder_text(cx);
        let label: SharedString = tr(cx, "Company name", "公司名称").into();
        let ed = cx.new(|cx| {
            let mut e = Editor::new(label, InputMode::Single, w, cx);
            e.set_placeholder(ph);
            e.font_size = 13.;
            e
        });
        cx.subscribe_in(&ed, w, |this, ed, ev: &EditEvent, w, cx| match ev {
            EditEvent::Changed => {
                let val = ed.read(cx).value().to_owned();
                if let Some(at) = &this.auto_type
                    && !this.auto_type_halted
                    && !at.starts_with(&val)
                {
                    this.auto_type_halted = true;
                }
                if val.contains(',') || val.contains('，') {
                    let parts: Vec<&str> = val.split([',', '，']).collect();
                    if parts.len() > 1 {
                        let complete = parts[..parts.len() - 1].join(",");
                        let tail = parts[parts.len() - 1].trim_start().to_owned();
                        let _ = this.add_name(&complete, cx);
                        let ph = this.placeholder_text(cx);
                        ed.update(cx, |e, cx| {
                            e.set_value(tail, w, cx);
                            e.set_placeholder(ph);
                        });
                    }
                } else if this.error_hint.is_some() {
                    this.error_hint = None;
                }
                cx.notify();
            }
            EditEvent::Submitted => {
                this.auto_type_halted = true;
                let val = ed.read(cx).value().trim().to_owned();
                if !val.is_empty() {
                    let _ = this.commit_editor(w, cx);
                } else {
                    let _ = this.submit(cx);
                }
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
                |_, _, cx| cx.emit(OnboardingBrandEvent::Back),
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
            |_, _, cx| cx.emit(OnboardingBrandEvent::Logout),
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
                |_, _, cx| cx.emit(OnboardingBrandEvent::OpenLegal("terms".into())),
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
                |_, _, cx| cx.emit(OnboardingBrandEvent::OpenLegal("privacy".into())),
            ))
    }

    fn render_dashboard_preview(
        &self,
        preview_w: f32,
        compact_side: bool,
        offset_y: f32,
        live_input: &str,
        pal: ObwTokens,
        cx: &mut Context<Self>,
    ) -> Div {
        let is_flat = self.layout == AppShellLayout::Flat;
        let trimmed_input = live_input.trim();
        let effective_primary: Option<String> = self
            .names
            .first()
            .map(|s| s.to_string())
            .or_else(|| (!trimmed_input.is_empty()).then(|| trimmed_input.to_owned()));

        let mut aliases: Vec<String> = self.names.iter().skip(1).map(|s| s.to_string()).collect();
        if !self.names.is_empty()
            && !trimmed_input.is_empty()
            && !self
                .names
                .iter()
                .any(|existing| existing.eq_ignore_ascii_case(trimmed_input))
        {
            aliases.push(trimmed_input.to_owned());
        }

        let mono_bg = self.brand_color.unwrap_or(pal.dark_fill);
        let mono_fg: Hsla = if self.brand_color.is_some() {
            rgb(0xffffff).into()
        } else {
            pal.on_dark
        };
        let mark_color = self.brand_color.unwrap_or(pal.mark);
        let bar_fill = self
            .brand_color
            .map(|c| c.opacity(if pal.dark { 0.30 } else { 0.22 }))
            .unwrap_or(pal.sk);

        let committed_names_key = self
            .names
            .iter()
            .map(SharedString::as_ref)
            .collect::<Vec<_>>()
            .join(",");
        let swatch_key = self
            .brand_swatch_hex
            .map(|h| format!("{h:06x}"))
            .unwrap_or_else(|| "mono".to_owned());
        let ws_tx_key: SharedString = format!(
            "{}-{swatch_key}",
            self.names
                .first()
                .map(SharedString::as_ref)
                .unwrap_or("none")
        )
        .into();
        let specimen_tx_key: SharedString = format!(
            "{}-{swatch_key}-{committed_names_key}",
            self.specimen_mode.as_str()
        )
        .into();

        // Sidebar (.ui-obw-side)
        let ws_row = if let Some(ref brand) = effective_primary {
            let initial: String = brand
                .chars()
                .find(|c| c.is_alphanumeric())
                .unwrap_or('N')
                .to_uppercase()
                .collect();
            div()
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
                        .bg(mono_bg)
                        .text_color(mono_fg)
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
                            .child(brand.clone()),
                    )
                    .child(
                        div()
                            .size(px(4.))
                            .flex_none()
                            .rounded_full()
                            .bg(pal.faint.opacity(0.6)),
                    )
                })
        } else {
            div()
                .h(px(32.))
                .when(!compact_side, |d| d.mx(px(-4.)).px(px(8.)).gap(px(8.)))
                .when(compact_side, |d| d.justify_center())
                .flex()
                .items_center()
                .child(sk_sq(16., 4., pal.sk))
                .when(!compact_side, |d| {
                    d.child(sk_bar(64., 8., pal.sk)).child(
                        div()
                            .size(px(4.))
                            .flex_none()
                            .rounded_full()
                            .bg(pal.faint.opacity(0.6)),
                    )
                })
        };

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
            .child(pro_view_transition(
                pro_eid(&self.id, "brand-ws-tx"),
                ws_tx_key,
                ws_row,
            ));

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

        // Specimen card (.ui-obw-agent): Card / Chat / Email modes
        let brand_display = effective_primary
            .clone()
            .unwrap_or_else(|| extract_brand_name(&self.domain));
        let assistant_name = effective_primary
            .as_ref()
            .map(|b| format!("{b} {}", tr(cx, "Assistant", "助手")));
        let greeting = effective_primary.as_ref().map(|b| {
            format!(
                "Hi, I'm the {b} assistant – ask me anything about our products, plans or your account."
            )
        });
        let agent_seed = assistant_name.as_deref().unwrap_or("Northwind Assistant");

        let aka_row = if !aliases.is_empty() {
            let shown = aliases.iter().take(3);
            let extra = aliases.len().saturating_sub(3);
            let tag_bg = self
                .brand_color
                .map(|c| c.opacity(0.12))
                .unwrap_or_else(|| pal.blue.opacity(0.10));
            let tag_fg = self.brand_color.unwrap_or(pal.blue);
            let mut row = div().h(px(20.)).flex().items_center().gap(px(6.)).child(
                div()
                    .mr(px(2.))
                    .text_size(px(12.))
                    .text_color(pal.faint)
                    .child(tr(cx, "Also answers to", "同时响应简称")),
            );
            for alias in shown {
                row = row.child(
                    div()
                        .max_w(px(140.))
                        .h(px(18.))
                        .px(px(6.))
                        .rounded(px(5.))
                        .bg(tag_bg)
                        .text_color(tag_fg)
                        .text_size(px(11.))
                        .font_weight(FontWeight::MEDIUM)
                        .flex()
                        .items_center()
                        .truncate()
                        .child(alias.clone()),
                );
            }
            if extra > 0 {
                row = row.child(
                    div()
                        .h(px(18.))
                        .px(px(6.))
                        .rounded(px(5.))
                        .bg(pal.line)
                        .text_color(pal.muted)
                        .text_size(px(11.))
                        .font_weight(FontWeight::MEDIUM)
                        .flex()
                        .items_center()
                        .child(format!("+{extra}")),
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

        let specimen_card = match self.specimen_mode {
            BrandSpecimenMode::Card => div()
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
                        .child(obw_agent_mark(agent_seed, mark_color, false))
                        .child(if let Some(ref name) = assistant_name {
                            div()
                                .min_w_0()
                                .truncate()
                                .text_size(px(13.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(pal.ink)
                                .child(name.clone())
                        } else {
                            sk_bar(120., 8., pal.sk)
                        })
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
                        .flex_col()
                        .justify_center()
                        .gap(px(8.))
                        .child(if let Some(say) = greeting {
                            div()
                                .text_size(px(13.))
                                .line_height(px(20.))
                                .text_color(pal.ink)
                                .child(say)
                        } else {
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(8.))
                                .child(sk_bar(280., 8., pal.sk))
                                .child(sk_bar(196., 8., pal.sk))
                        }),
                )
                .child(aka_row),
            BrandSpecimenMode::Chat => {
                let user_bubble_bg = self.brand_color.unwrap_or(pal.dark_fill);
                let user_bubble_fg: Hsla = if self.brand_color.is_some() {
                    rgb(0xffffff).into()
                } else {
                    pal.on_dark
                };
                let alias_ref = aliases
                    .first()
                    .cloned()
                    .unwrap_or_else(|| brand_display.clone());
                div()
                    .h(px(168.))
                    .p(px(16.))
                    .rounded(px(10.))
                    .border_1()
                    .border_color(pal.line)
                    .flex()
                    .flex_col()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .child(
                                div()
                                    .max_w(px(300.))
                                    .px(px(12.))
                                    .py(px(8.))
                                    .rounded_tl(px(12.))
                                    .rounded_tr(px(4.))
                                    .rounded_bl(px(12.))
                                    .rounded_br(px(12.))
                                    .bg(user_bubble_bg)
                                    .text_color(user_bubble_fg)
                                    .text_size(px(12.5))
                                    .child(format!("Does {alias_ref} include team SSO?")),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_start()
                            .gap(px(10.))
                            .child(
                                div()
                                    .mt(px(4.))
                                    .child(obw_agent_mark(agent_seed, mark_color, false)),
                            )
                            .child(
                                div()
                                    .max_w(px(360.))
                                    .px(px(13.))
                                    .py(px(9.))
                                    .rounded_tl(px(4.))
                                    .rounded_tr(px(12.))
                                    .rounded_br(px(12.))
                                    .rounded_bl(px(12.))
                                    .bg(pal.side)
                                    .border_1()
                                    .border_color(pal.line)
                                    .text_size(px(12.5))
                                    .line_height(px(18.))
                                    .text_color(pal.ink)
                                    .child(format!(
                                        "Yes — {brand_display} Business includes SAML SSO and SCIM out of the box."
                                    )),
                            ),
                    )
                    .child(aka_row)
            }
            BrandSpecimenMode::Email => {
                let init_ch: String = brand_display
                    .chars()
                    .find(|c| c.is_alphanumeric())
                    .unwrap_or('N')
                    .to_uppercase()
                    .collect();
                div()
                    .h(px(168.))
                    .p(px(16.))
                    .rounded(px(10.))
                    .border_1()
                    .border_color(pal.line)
                    .flex()
                    .flex_col()
                    .justify_between()
                    .child(
                        div()
                            .pb(px(10.))
                            .border_b_1()
                            .border_color(pal.line)
                            .flex()
                            .items_center()
                            .gap(px(9.))
                            .child(
                                div()
                                    .size(px(22.))
                                    .rounded(px(6.))
                                    .bg(mono_bg)
                                    .text_color(mono_fg)
                                    .text_size(px(11.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(init_ch),
                            )
                            .child(
                                div()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .text_size(px(12.5))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(pal.ink)
                                            .truncate()
                                            .child(format!("{brand_display} Support")),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.))
                                            .text_color(pal.faint)
                                            .truncate()
                                            .child(format!("support@{}", self.domain)),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(12.5))
                            .line_height(px(18.))
                            .text_color(pal.ink)
                            .child(format!(
                                "Here is a summary from the {brand_display} assistant, plus links from {}.",
                                self.domain
                            )),
                    )
                    .child(aka_row)
            }
        };

        let specimen_tx = pro_view_transition(
            pro_eid(&self.id, "brand-specimen-tx"),
            specimen_tx_key,
            specimen_card,
        );

        // Conversations chart (.ui-obw-chart)
        let chart_card = render_conversations_chart(bar_fill, pal);

        // Knowledge table (.ui-obw-table)
        let domain_initial: String = extract_brand_name(&self.domain)
            .chars()
            .next()
            .unwrap_or('N')
            .to_uppercase()
            .collect();
        let knowledge_card =
            render_knowledge_table(&self.domain, &domain_initial, mono_bg, mono_fg, pal, cx);

        // Interactive specimen tabs in preview top strip
        let preview_tab_sensor = pro_selection_sensor();
        let preview_tabs = div()
            .relative()
            .px(px(16.))
            .py(px(10.))
            .border_b_1()
            .border_color(pal.line)
            .flex()
            .items_center()
            .gap(px(8.))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "preview-specimen-pill"),
                self.specimen_mode.as_str(),
                preview_tab_sensor.clone(),
                7.,
                pal.side,
                Some(pal.line_firm),
                false,
            ))
            .children(BrandSpecimenMode::ALL.into_iter().map(|mode| {
                let on = self.specimen_mode == mode;
                let lbl: SharedString = mode.label(cx).into();
                activate(
                    div()
                        .id(pro_eid(&self.id, format!("pv-tab-{}", mode.as_str())))
                        .role(Role::Tab)
                        .tab_index(0)
                        .relative()
                        .h(px(28.))
                        .pl(px(10.))
                        .pr(px(12.))
                        .rounded(px(7.))
                        .when(!on, |d| {
                            d.border_1()
                                .border_color(pal.line)
                                .hover(move |s| s.border_color(pal.line_firm))
                        })
                        .cursor_pointer()
                        .flex()
                        .items_center()
                        .gap(px(7.))
                        .when(on, |d| d.child(pro_mark_selected(&preview_tab_sensor)))
                        .child(div().size(px(7.)).rounded_full().bg(if on {
                            mono_bg
                        } else {
                            pal.sk
                        }))
                        .child(
                            div()
                                .text_size(px(12.))
                                .font_weight(if on {
                                    FontWeight::MEDIUM
                                } else {
                                    FontWeight::NORMAL
                                })
                                .text_color(if on { pal.ink } else { pal.muted })
                                .child(lbl),
                        ),
                    cx,
                    move |this, _, cx| {
                        this.specimen_mode = mode;
                        cx.notify();
                    },
                )
            }))
            .child(
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
                    .child(sk_bar(84., 8., pal.sk)),
            );

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
                specimen_tx,
                pal,
            ))
            .child(render_section(
                Some(tr(cx, "Conversations", "对话量")),
                chart_card,
                pal,
            ))
            .child(render_section(
                Some(tr(cx, "Knowledge", "知识库")),
                knowledge_card,
                pal,
            ));

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
            .child(preview_tabs)
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
        ed: Entity<Editor>,
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
                .child(pro_skeleton_bar(260., 26., 6., 0.2, cx))
                .child(pro_skeleton_bar(310., 14., 4., 0.3, cx))
                .child(
                    div()
                        .w_full()
                        .mt(px(14.))
                        .child(pro_skeleton_bar(440., 44., 12., 0.4, cx)),
                )
                .child(
                    div()
                        .flex()
                        .gap(px(8.))
                        .child(pro_skeleton_bar(86., 26., 7., 0.5, cx))
                        .child(pro_skeleton_bar(72., 26., 7., 0.6, cx)),
                );
        }

        if self.empty {
            let cta: ProEmptyCta<Self> = (
                pro_eid(&self.id, "empty-reset"),
                tr(cx, "Use domain suggestion", "使用域名推荐名称").into(),
                Box::new(|this, _, cx| {
                    this.empty = false;
                    if this.names.is_empty() {
                        this.names
                            .push(SharedString::from(extract_brand_name(&this.domain)));
                    }
                    cx.notify();
                }),
            );
            return div().w_full().max_w(px(440.)).child(pro_empty_state(
                ProIcon::Building,
                tr(cx, "No company name set", "尚未设置公司名称"),
                tr(
                    cx,
                    "Add the primary name and short forms customers use when referring to your workspace.",
                    "添加客户常用来称呼你工作区的主名称与简称。",
                ),
                Some(cta),
                cx,
            ));
        }

        let input_val = ed.read(cx).value().to_owned();
        let ed_focused = ed.read(cx).focus.is_focused(w);
        let add_disabled = input_val.trim().is_empty();
        let go_disabled = self.names.is_empty() && add_disabled;

        let focus_t = pro_spring_f32(
            pro_eid(&self.id, "name-focus-sp"),
            f32::from(ed_focused),
            0.18,
            w,
            cx,
        )
        .clamp(0., 1.);
        let add_alpha = pro_spring_f32(
            pro_eid(&self.id, "brand-add-sp"),
            if add_disabled { 0.45 } else { 1. },
            0.18,
            w,
            cx,
        )
        .clamp(0.45, 1.);
        let go_alpha = pro_spring_f32(
            pro_eid(&self.id, "brand-go-sp"),
            if go_disabled { 0.45 } else { 1. },
            0.18,
            w,
            cx,
        )
        .clamp(0.45, 1.);

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

        // Aurora glow behind .ui-obw-box
        let glow_alpha = (if pal.dark { 0.032 } else { 0.044 }) * (1.0 + 0.32 * focus_t);
        let mut glow_wrap = div().relative().w_full();
        for &(inset_y, inset_x, rad) in &[
            (-16., -24., 32.),
            (-11., -18., 26.),
            (-7., -12., 22.),
            (-3., -6., 16.),
        ] {
            glow_wrap = glow_wrap
                .child(
                    div()
                        .absolute()
                        .top(px(inset_y))
                        .bottom(px(inset_y))
                        .left(px(inset_x))
                        .right(px(inset_x * 0.2))
                        .rounded(px(rad))
                        .bg(linear_gradient(
                            95.,
                            linear_color_stop(pal.blue.opacity(glow_alpha), 0.08),
                            linear_color_stop(pal.purple.opacity(glow_alpha * 0.85), 0.78),
                        )),
                )
                .child(
                    div()
                        .absolute()
                        .top(px(inset_y))
                        .bottom(px(inset_y))
                        .left(px(inset_x * 0.2))
                        .right(px(inset_x))
                        .rounded(px(rad))
                        .bg(linear_gradient(
                            95.,
                            linear_color_stop(pal.purple.opacity(glow_alpha * 0.75), 0.22),
                            linear_color_stop(pal.pink.opacity(glow_alpha), 0.92),
                        )),
                );
        }

        let add_btn_base = div()
            .id(pro_eid(&self.id, "add-name"))
            .role(Role::Button)
            .aria_label(tr(cx, "Add name", "添加名称"))
            .tab_index(if add_disabled { -1 } else { 0 })
            .size(px(28.))
            .flex_none()
            .rounded(px(8.))
            .bg(pal.card)
            .border_1()
            .border_color(pal.line)
            .flex()
            .items_center()
            .justify_center()
            .opacity(add_alpha)
            .when(!add_disabled, |d| {
                d.cursor_pointer()
                    .hover(move |s| s.border_color(pal.line_firm))
                    .focus_visible(move |s| s.border_color(pal.focus))
            })
            .child(pro_icon(ProIcon::Plus, 14., pal.ink));
        let add_btn = if add_disabled {
            add_btn_base.into_any_element()
        } else {
            activate(add_btn_base, cx, |this, w, cx| {
                this.auto_type_halted = true;
                let _ = this.commit_editor(w, cx);
            })
            .into_any_element()
        };

        let go_fill = self.brand_color.unwrap_or(pal.dark_fill);
        let go_fg: Hsla = if self.brand_color.is_some() {
            rgb(0xffffff).into()
        } else {
            pal.on_dark
        };
        let go_btn_base = div()
            .id(pro_eid(&self.id, "continue"))
            .role(Role::Button)
            .aria_label(tr(cx, "Continue", "继续"))
            .tab_index(if go_disabled { -1 } else { 0 })
            .size(px(28.))
            .flex_none()
            .rounded(px(8.))
            .bg(go_fill)
            .flex()
            .items_center()
            .justify_center()
            .opacity(go_alpha)
            .when(!go_disabled, |d| {
                d.cursor_pointer()
                    .hover(move |s| s.opacity(0.88))
                    .focus_visible(move |s| s.border_1().border_color(pal.focus))
            })
            .child(pro_icon(ProIcon::ArrowRight, 14., go_fg));
        let go_btn = if go_disabled {
            go_btn_base.into_any_element()
        } else {
            activate(go_btn_base, cx, |this, w, cx| {
                this.auto_type_halted = true;
                this.continue_from_ui(w, cx);
            })
            .into_any_element()
        };

        let box_border = if self.error_hint.is_some() {
            pal.red.opacity(0.55)
        } else {
            pro_mix_color(pal.line, pal.line_firm, focus_t)
        };

        let input_box = div()
            .id(pro_eid(&self.id, "box"))
            .relative()
            .h(px(44.))
            .pl(px(14.))
            .pr(px(8.))
            .rounded(px(12.))
            .bg(pal.card)
            .border_1()
            .border_color(box_border)
            .flex()
            .items_center()
            .gap(px(6.))
            .capture_key_down(cx.listener(|this, ev: &KeyDownEvent, w, cx| {
                this.auto_type_halted = true;
                if ev.keystroke.key == "backspace"
                    && !ev.keystroke.modifiers.platform
                    && this
                        .name_editor
                        .as_ref()
                        .is_some_and(|e| e.read(cx).value().is_empty())
                    && !this.names.is_empty()
                {
                    let last = this.names.len() - 1;
                    let _ = this.remove_name(last, cx);
                    if let Some(ed) = this.name_editor.clone() {
                        let ph = this.placeholder_text(cx);
                        ed.update(cx, |e, _| e.set_placeholder(ph));
                    }
                    w.prevent_default();
                    cx.stop_propagation();
                }
            }))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .h(px(22.))
                    .flex()
                    .items_center()
                    .child(ed),
            )
            .child(add_btn)
            .child(go_btn);

        glow_wrap = glow_wrap.child(input_box);

        // Chips list (.ui-obw-chips)
        let chips_row = div()
            .mt(px(10.))
            .min_h(px(26.))
            .w_full()
            .flex()
            .flex_wrap()
            .justify_center()
            .items_center()
            .gap(px(6.))
            .children(self.names.iter().enumerate().map(|(idx, chip)| {
                let chip_txt = chip.clone();
                let rm_aria = format!("{} {chip_txt}", tr(cx, "Remove", "移除"));
                div()
                    .h(px(26.))
                    .pl(px(9.))
                    .pr(px(4.))
                    .rounded(px(7.))
                    .bg(pal.card)
                    .border_1()
                    .border_color(pal.line)
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .text_size(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(pal.ink)
                    .child(div().max_w(px(220.)).truncate().child(chip_txt))
                    .child(activate(
                        div()
                            .id(pro_eid(&self.id, format!("rm-chip-{idx}")))
                            .role(Role::Button)
                            .aria_label(rm_aria)
                            .tab_index(0)
                            .ml(px(-2.))
                            .size(px(18.))
                            .rounded(px(4.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .hover(move |s| s.bg(pal.line).text_color(pal.ink))
                            .focus_visible(move |s| s.border_1().border_color(pal.focus))
                            .child(pro_icon(ProIcon::Close, 12., pal.faint)),
                        cx,
                        move |this, _, cx| {
                            let _ = this.remove_name(idx, cx);
                        },
                    ))
            }));

        // Preview mode segmented pill strip (`Card / Chat / Email`) + Brand palette disclosure
        let mode_sensor = pro_selection_sensor();
        let mode_strip = div()
            .relative()
            .p(px(2.))
            .rounded(px(8.))
            .bg(pal.line)
            .flex()
            .items_center()
            .gap(px(2.))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "brand-mode-pill"),
                self.specimen_mode.as_str(),
                mode_sensor.clone(),
                6.,
                pal.card,
                Some(pal.line),
                !pal.dark,
            ))
            .children(BrandSpecimenMode::ALL.into_iter().map(|mode| {
                let checked = self.specimen_mode == mode;
                let lbl: SharedString = mode.label(cx).into();
                activate(
                    div()
                        .id(pro_eid(&self.id, format!("mode-{}", mode.as_str())))
                        .role(Role::Tab)
                        .tab_index(0)
                        .relative()
                        .h(px(24.))
                        .px(px(9.))
                        .rounded(px(6.))
                        .when(!checked, |d| {
                            d.text_color(pal.muted)
                                .hover(move |s| s.text_color(pal.ink))
                        })
                        .when(checked, |d| {
                            d.text_color(pal.ink).child(pro_mark_selected(&mode_sensor))
                        })
                        .focus_visible(move |s| s.border_1().border_color(pal.focus))
                        .cursor_pointer()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(12.))
                        .font_weight(FontWeight::MEDIUM)
                        .child(lbl),
                    cx,
                    move |this, _, cx| {
                        this.specimen_mode = mode;
                        cx.notify();
                    },
                )
            }));

        let swatches_open = self.palette_open || self.show_color_swatches;
        let kit_toggle = activate(
            div()
                .id(pro_eid(&self.id, "brand-kit-toggle"))
                .role(Role::Button)
                .tab_index(0)
                .h(px(28.))
                .px(px(9.))
                .rounded(px(8.))
                .bg(pal.card)
                .border_1()
                .border_color(if swatches_open {
                    pal.line_firm
                } else {
                    pal.line
                })
                .hover(move |s| s.border_color(pal.line_firm))
                .focus_visible(move |s| s.border_1().border_color(pal.focus))
                .cursor_pointer()
                .flex()
                .items_center()
                .gap(px(6.))
                .text_size(px(12.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(pal.ink)
                .child(div().size(px(10.)).rounded_full().bg(go_fill))
                .child(tr(cx, "Accent", "品牌色"))
                .child(pro_icon(
                    if swatches_open {
                        ProIcon::ChevronUp
                    } else {
                        ProIcon::ChevronDown
                    },
                    11.,
                    pal.faint,
                )),
            cx,
            |this, _, cx| {
                let next = !(this.palette_open || this.show_color_swatches);
                this.palette_open = next;
                this.show_color_swatches = next;
                if !next {
                    this.custom_hex_open = false;
                }
                cx.notify();
            },
        );

        let kit_bar = div()
            .mt(px(12.))
            .flex()
            .items_center()
            .justify_center()
            .gap(px(8.))
            .child(mode_strip)
            .child(kit_toggle);

        // Brand accent swatch selector + custom hex fold (`pro_fold`)
        let default_active = self.brand_swatch_hex.is_none();
        let active_hex_str: SharedString = match self.brand_swatch_hex {
            Some(hex) => format!("#{hex:06X}").into(),
            None => "#171717 · Monochrome".into(),
        };
        let hex_panel_div = div()
            .mt(px(8.))
            .px(px(10.))
            .py(px(7.))
            .rounded(px(8.))
            .bg(pal.card)
            .border_1()
            .border_color(pal.line)
            .flex()
            .items_center()
            .justify_between()
            .gap(px(8.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .text_size(px(11.5))
                    .text_color(pal.muted)
                    .child(tr(cx, "Token", "色值"))
                    .child(pro_view_transition(
                        pro_eid(&self.id, "active-hex-tx"),
                        active_hex_str.clone(),
                        div()
                            .px(px(6.))
                            .py(px(1.))
                            .rounded(px(4.))
                            .bg(pal.line)
                            .font_family("Fira Code")
                            .text_color(pal.ink)
                            .child(active_hex_str),
                    )),
            )
            .child(
                div().flex().items_center().gap(px(4.)).children(
                    [(0x0f766e_u32, "#0F766E"), (0xc2410c_u32, "#C2410C")]
                        .into_iter()
                        .map(|(hex, code)| {
                            activate(
                                div()
                                    .id(pro_eid(&self.id, format!("preset-hex-{hex:06x}")))
                                    .role(Role::Button)
                                    .tab_index(0)
                                    .h(px(20.))
                                    .px(px(6.))
                                    .rounded(px(5.))
                                    .border_1()
                                    .border_color(if self.brand_swatch_hex == Some(hex) {
                                        pal.focus
                                    } else {
                                        pal.line
                                    })
                                    .hover(move |s| s.border_color(pal.line_firm))
                                    .cursor_pointer()
                                    .flex()
                                    .items_center()
                                    .gap(px(4.))
                                    .text_size(px(11.))
                                    .text_color(pal.muted)
                                    .child(div().size(px(8.)).rounded_full().bg(rgb(hex)))
                                    .child(code),
                                cx,
                                move |this, _, cx| {
                                    this.brand_color = Some(rgb(hex).into());
                                    this.brand_swatch_hex = Some(hex);
                                    cx.emit(OnboardingBrandEvent::BrandColorChanged(Some(hex)));
                                    cx.notify();
                                },
                            )
                        }),
                ),
            )
            .into_any_element();

        let mut swatch_row = div()
            .mt(px(10.))
            .flex()
            .items_center()
            .justify_center()
            .gap(px(6.))
            .child(activate(
                div()
                    .id(pro_eid(&self.id, "swatch-default"))
                    .role(Role::RadioButton)
                    .aria_label(tr(cx, "Monochrome brand accent", "黑白默认品牌色"))
                    .tab_index(0)
                    .size(px(18.))
                    .rounded_full()
                    .bg(pal.dark_fill)
                    .border_2()
                    .border_color(if default_active {
                        pal.focus
                    } else {
                        transparent_black()
                    })
                    .cursor_pointer(),
                cx,
                |this, _, cx| {
                    this.brand_color = None;
                    this.brand_swatch_hex = None;
                    cx.emit(OnboardingBrandEvent::BrandColorChanged(None));
                    cx.notify();
                },
            ));
        for (hex, name) in BRAND_SWATCH_HEXES {
            let active = self.brand_swatch_hex == Some(hex);
            let swatch_c: Hsla = rgb(hex).into();
            swatch_row = swatch_row.child(activate(
                div()
                    .id(pro_eid(&self.id, format!("swatch-{hex:06x}")))
                    .role(Role::RadioButton)
                    .aria_label(name)
                    .tab_index(0)
                    .size(px(18.))
                    .rounded_full()
                    .bg(swatch_c)
                    .border_2()
                    .border_color(if active {
                        pal.focus
                    } else {
                        transparent_black()
                    })
                    .cursor_pointer(),
                cx,
                move |this, _, cx| {
                    this.brand_color = Some(rgb(hex).into());
                    this.brand_swatch_hex = Some(hex);
                    cx.emit(OnboardingBrandEvent::BrandColorChanged(Some(hex)));
                    cx.notify();
                },
            ));
        }
        swatch_row = swatch_row.child(activate(
            div()
                .id(pro_eid(&self.id, "swatch-hex-btn"))
                .role(Role::Button)
                .tab_index(0)
                .ml(px(2.))
                .h(px(20.))
                .px(px(6.))
                .rounded(px(5.))
                .bg(if self.custom_hex_open {
                    pal.line
                } else {
                    pal.card
                })
                .border_1()
                .border_color(pal.line)
                .hover(move |s| s.border_color(pal.line_firm))
                .cursor_pointer()
                .flex()
                .items_center()
                .text_size(px(11.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(pal.muted)
                .child("#Hex"),
            cx,
            |this, _, cx| {
                this.custom_hex_open = !this.custom_hex_open;
                cx.notify();
            },
        ));

        let custom_hex_open = self.custom_hex_open;
        let custom_hex_fold = pro_fold(
            pro_eid(&self.id, "custom-hex-fold"),
            swatches_open && custom_hex_open,
            36.,
            0.,
            move || hex_panel_div,
        );
        let swatches_wrap_el = div()
            .w_full()
            .flex()
            .flex_col()
            .items_center()
            .child(swatch_row)
            .child(custom_hex_fold)
            .into_any_element();
        let swatches_fold = pro_fold(
            pro_eid(&self.id, "swatches-fold"),
            swatches_open,
            40.,
            0.,
            move || swatches_wrap_el,
        );

        // Polite live status / error hint (.ui-obw-hint)
        let has_hint = self.error_hint.is_some() || self.names.len() >= 2;
        let (hint_text, is_error): (SharedString, bool) = if let Some(ref err) = self.error_hint {
            (err.clone(), true)
        } else if self.names.len() >= 2 {
            let count = self.names.len();
            (
                format!("{count} {} · {}", tr(cx, "names", "个名称"), self.names[0]).into(),
                false,
            )
        } else {
            (self.last_error_hint.clone(), true)
        };
        let hint_color = if is_error { pal.red } else { pal.faint };
        let hint_pres = pro_presence(
            pro_eid(&self.id, "brand-hint-pres"),
            has_hint,
            0.18,
            0.14,
            move |amt| {
                div()
                    .relative()
                    .top(px(3. * (1. - amt)))
                    .opacity(amt)
                    .text_size(px(12.))
                    .line_height(px(16.))
                    .text_color(hint_color)
                    .text_center()
                    .child(hint_text)
                    .into_any_element()
            },
        );

        div()
            .w_full()
            .max_w(px(440.))
            .flex()
            .flex_col()
            .items_center()
            .text_center()
            .child(steps_bar)
            .child(
                div()
                    .text_size(px(24.))
                    .line_height(px(32.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(pal.ink)
                    .child(tr(
                        cx,
                        "What's your company called?",
                        "你的公司叫什么名字？",
                    )),
            )
            .child(
                div()
                    .mt(px(6.))
                    .text_size(px(13.))
                    .line_height(px(20.))
                    .text_color(pal.muted)
                    .child(tr(
                        cx,
                        "Your assistant introduces itself with it. Add short forms, too.",
                        "助手会用这个名字展示名称，也可以添加常用简称。",
                    )),
            )
            .child(
                div()
                    .w_full()
                    .mt(px(28.))
                    .flex()
                    .flex_col()
                    .child(glow_wrap)
                    .child(chips_row)
                    .child(kit_bar)
                    .child(swatches_fold)
                    .child(div().mt(px(12.)).min_h(px(16.)).child(hint_pres)),
            )
    }
}

impl Render for OnboardingBrandScreen {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let pal = ObwTokens::resolve(cx);
        let reduced_motion = Theme::global(cx).reduced_motion;
        let now = Instant::now();
        let opened = *self.opened_at.get_or_insert(now);
        let elapsed_ms = now.duration_since(opened).as_secs_f32() * 1000.;

        let ed = self.ensure_editor(w, cx);

        // Optional demo auto-typing on initial open
        if let Some(target_str) = self.auto_type.clone()
            && !self.auto_type_halted
        {
            let chars: Vec<char> = target_str.chars().collect();
            if reduced_motion {
                self.auto_type_halted = true;
                ed.update(cx, |e, cx| e.set_value(target_str.to_string(), w, cx));
            } else {
                let desired = ((elapsed_ms / 65.).floor() as usize).min(chars.len());
                if desired > self.auto_type_index {
                    self.auto_type_index = desired;
                    let prefix: String = chars[..desired].iter().collect();
                    ed.update(cx, |e, cx| e.set_value(prefix, w, cx));
                }
                if desired < chars.len() {
                    w.request_animation_frame();
                } else {
                    self.auto_type_halted = true;
                }
            }
        }

        // Smooth glide of dashboard preview to -128px (Your assistant card)
        let target_offset_y = -128.0_f32;
        let offset_y = if reduced_motion || elapsed_ms >= 420. {
            target_offset_y
        } else {
            w.request_animation_frame();
            let t = pro_ease(elapsed_ms / 420.);
            -92.0 + (target_offset_y - -92.0) * t
        };

        let container_w = self.measured_width.get();
        let narrow = container_w <= 900.;
        let compact_side = container_w <= 1040.;
        let measure_cell = self.measured_width.clone();
        let live_input = ed.read(cx).value().to_owned();

        let preview = (!narrow).then(|| {
            self.render_dashboard_preview(
                container_w * 0.5,
                compact_side,
                offset_y,
                &live_input,
                pal,
                cx,
            )
        });
        let top_bar = self.render_top_bar(narrow, pal, cx);
        let step_body = self.render_step_body(ed, pal, w, cx);
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

fn render_section(title: Option<&str>, card: impl IntoElement, pal: ObwTokens) -> Div {
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

fn render_conversations_chart(bar_fill: Hsla, pal: ObwTokens) -> Div {
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
                .bg(bar_fill),
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

fn render_knowledge_table(
    domain: &str,
    initial: &str,
    mono_bg: Hsla,
    mono_fg: Hsla,
    pal: ObwTokens,
    cx: &App,
) -> Div {
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
                        .bg(mono_bg)
                        .text_color(mono_fg)
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
    use super::{OnboardingBrandEvent, OnboardingBrandScreen, extract_brand_name};
    use gpui::TestAppContext;
    use std::{cell::RefCell, rc::Rc};

    #[test]
    fn brand_extraction_normalizes_domains_and_subdomains() {
        assert_eq!(extract_brand_name("northwind.com"), "Northwind");
        assert_eq!(
            extract_brand_name("https://docs.fieldwork.io/getting-started"),
            "Fieldwork"
        );
        assert_eq!(extract_brand_name("www.acme-cloud.co.uk"), "Acme Cloud");
        assert_eq!(extract_brand_name("   "), "Workspace");
    }

    #[test]
    fn chip_addition_deduplicates_removes_and_submits_in_order() {
        let mut app = TestAppContext::single();
        app.update(crate::init);

        let events: Rc<RefCell<Vec<OnboardingBrandEvent>>> = Rc::new(RefCell::new(Vec::new()));
        let captured = events.clone();

        let (view, w) = app.add_window_view(|_w, cx| {
            OnboardingBrandScreen::new("test-obw-brand", cx)
                .with_domain("https://help.northwind.com")
        });

        w.update(|_w, cx| {
            cx.subscribe(&view, move |_, ev: &OnboardingBrandEvent, _| {
                captured.borrow_mut().push(ev.clone());
            })
            .detach();
            view.update(cx, |screen, cx| {
                assert_eq!(screen.names().len(), 1);
                assert_eq!(screen.primary_name().map(|s| s.as_ref()), Some("Northwind"));

                // Duplicate (case-insensitive) is rejected
                assert!(!screen.add_name("northwind", cx));
                assert_eq!(screen.names().len(), 1);

                // Comma-separated short forms append in order
                assert!(screen.add_name("Northwind Co., NW", cx));
                assert_eq!(screen.names().len(), 3);

                // Removing middle chip preserves primary and trailing alias
                assert!(screen.remove_name(1, cx));
                assert_eq!(screen.names().len(), 2);

                assert!(screen.submit(cx));
            });
        });

        let log = events.borrow();
        assert!(log.iter().any(|ev| matches!(
            ev,
            OnboardingBrandEvent::Continue(names)
                if names.len() == 2 && names[0].as_ref() == "Northwind" && names[1].as_ref() == "NW"
        )));
    }
}
